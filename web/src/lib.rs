use std::collections::HashMap;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use zelyra_ast::{FormDef, TableDef, Type};
use zelyra_database::Schema;
use zelyra_forms::{validate, FieldError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub path: String,
    pub html: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    pub target: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub reason: String,
    pub content_type: String,
    pub body: String,
    pub location: Option<String>,
}

impl Response {
    pub fn html(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            reason: reason_phrase(status).into(),
            content_type: "text/html; charset=utf-8".into(),
            body: body.into(),
            location: None,
        }
    }

    pub fn to_http(&self) -> String {
        format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
            self.status,
            self.reason,
            self.content_type,
            self.location
                .as_deref()
                .map_or(String::new(), |location| format!("Location: {location}\r\n")),
            self.body.len(),
            self.body
        )
    }

    pub fn redirect(location: impl Into<String>) -> Self {
        let location = location.into();
        let location = if location.contains(['\r', '\n']) {
            "/".into()
        } else {
            location
        };
        Self {
            status: 303,
            reason: reason_phrase(303).into(),
            content_type: "text/plain; charset=utf-8".into(),
            body: String::new(),
            location: Some(location),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Router {
    routes: Vec<Route>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CsrfProtection {
    token: String,
}

impl CsrfProtection {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }

    pub fn generate() -> io::Result<Self> {
        let mut bytes = [0_u8; 32];
        let mut source = std::fs::File::open("/dev/urandom")?;
        source.read_exact(&mut bytes)?;
        Ok(Self::new(hex_encode(&bytes)))
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn verify(&self, candidate: Option<&str>) -> bool {
        let Some(candidate) = candidate else {
            return false;
        };
        constant_time_equal(self.token.as_bytes(), candidate.as_bytes())
    }
}

#[derive(Clone, Debug)]
pub struct FormRoute {
    pub path: String,
    pub action: String,
    pub form: FormDef,
    pub table: Option<TableDef>,
    pub schema: Option<Schema>,
    pub csrf: CsrfProtection,
}

#[derive(Clone, Debug)]
pub struct WebApp {
    pub routes: Vec<Route>,
    pub forms: Vec<FormRoute>,
    pub database_url: Option<String>,
}

impl WebApp {
    pub fn new(routes: Vec<Route>, forms: Vec<FormRoute>) -> Self {
        Self {
            routes,
            forms,
            database_url: None,
        }
    }

    pub fn with_database_url(
        routes: Vec<Route>,
        forms: Vec<FormRoute>,
        database_url: Option<String>,
    ) -> Self {
        Self {
            routes,
            forms,
            database_url,
        }
    }

    pub fn dispatch(&self, request: &Request) -> Response {
        for form in &self.forms {
            if match_path(&form.path, &request.path).is_some() {
                return dispatch_form(form, request, self.database_url.as_deref());
            }
        }
        Router::new(self.routes.clone()).dispatch(&request.method, &request.target)
    }
}

impl Router {
    pub fn new(routes: Vec<Route>) -> Self {
        Self { routes }
    }

    pub fn dispatch(&self, method: &str, path: &str) -> Response {
        if method != "GET" {
            return Response::html(405, "<h1>405 Method Not Allowed</h1>");
        }
        let path = path.split_once('?').map_or(path, |(path, _)| path);
        for route in &self.routes {
            if let Some(params) = match_path(&route.path, path) {
                return Response::html(200, render_template(&route.html, &params));
            }
        }
        Response::html(404, "<h1>404 Not Found</h1>")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpError {
    pub message: String,
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl std::error::Error for HttpError {}

pub fn parse_request(raw: &str) -> Result<Request, HttpError> {
    let (header_text, body) = raw.split_once("\r\n\r\n").unwrap_or((raw, ""));
    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().ok_or_else(|| HttpError {
        message: "request is empty".into(),
    })?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().ok_or_else(|| HttpError {
        message: "request method is missing".into(),
    })?;
    let target = request_parts.next().ok_or_else(|| HttpError {
        message: "request target is missing".into(),
    })?;
    let version = request_parts.next().ok_or_else(|| HttpError {
        message: "HTTP version is missing".into(),
    })?;
    if version != "HTTP/1.1" && version != "HTTP/1.0" {
        return Err(HttpError {
            message: format!("unsupported HTTP version `{version}`"),
        });
    }
    if request_parts.next().is_some() {
        return Err(HttpError {
            message: "request line contains too many fields".into(),
        });
    }
    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        let (name, value) = line.split_once(':').ok_or_else(|| HttpError {
            message: "malformed HTTP header".into(),
        })?;
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().into());
    }
    let path = target.split_once('?').map_or(target, |(path, _)| path);
    Ok(Request {
        method: method.into(),
        target: target.into(),
        path: path.into(),
        headers,
        body: body.into(),
    })
}

pub fn html_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn dispatch_form(form: &FormRoute, request: &Request, database_url: Option<&str>) -> Response {
    if request.method == "GET" {
        return Response::html(200, render_form(form, &HashMap::new(), &[], None));
    }
    if request.method != "POST" {
        return Response::html(405, "<h1>405 Method Not Allowed</h1>");
    }
    let input = match parse_urlencoded(&request.body) {
        Ok(input) => input,
        Err(error) => {
            return Response::html(400, format!("<h1>400 Bad Request</h1><p>{error}</p>"))
        }
    };
    if !form
        .csrf
        .verify(input.get("_zelyra_csrf").map(String::as_str))
    {
        return Response::html(403, "<h1>403 Forbidden</h1><p>Invalid CSRF token.</p>");
    }
    let mut values = input;
    values.remove("_zelyra_csrf");
    let validation = validate(
        &form.form,
        form.table.as_ref(),
        form.schema.as_ref(),
        &values,
    );
    if !validation.is_valid() {
        return Response::html(
            422,
            render_form(
                form,
                &values,
                &validation.errors,
                Some("Please correct the errors."),
            ),
        );
    }
    if let Some(action) = form.form.actions.first() {
        let Some(database_url) = database_url else {
            return Response::html(
                503,
                "<h1>503 Service Unavailable</h1><p>DATABASE_URL is required for this form action.</p>",
            );
        };
        if let Err(error) = execute_form_action(form, action, &values, database_url) {
            eprintln!("zelyra web: form action failed: {error}");
            return Response::html(500, "<h1>500 Internal Server Error</h1>");
        }
        return Response::redirect(action.redirect.as_deref().unwrap_or("/"));
    }
    Response::html(
        202,
        render_form(
            form,
            &values,
            &[],
            Some("Input validated. Database action execution is not enabled yet."),
        ),
    )
}

fn execute_form_action(
    form: &FormRoute,
    action: &zelyra_ast::FormAction,
    values: &HashMap<String, String>,
    database_url: &str,
) -> Result<(), String> {
    let mut queries = Vec::new();
    for statement in &action.statements {
        let expression = match statement {
            zelyra_ast::Stmt::Expr(expression) => expression,
            _ => return Err("form actions may contain only SQL statements".into()),
        };
        let zelyra_ast::ExprKind::Sql { query, .. } = &expression.kind else {
            return Err("form actions may contain only SQL statements".into());
        };
        queries.push(zelyra_database::Query {
            sql: query.clone(),
            params: form_query_parameters(form, values)?,
        });
    }
    if queries.is_empty() {
        return Err("form action must contain at least one SQL statement".into());
    }
    zelyra_database::execute_mariadb_queries(database_url, &queries, true)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn form_query_parameters(
    form: &FormRoute,
    values: &HashMap<String, String>,
) -> Result<Vec<(String, zelyra_database::QueryValue)>, String> {
    let mut parameters = Vec::new();
    for field in &form.form.fields {
        let Some(value) = values.get(&field.name) else {
            continue;
        };
        let ty = field.ty.as_ref().or_else(|| {
            form.table.as_ref().and_then(|table| {
                table
                    .columns
                    .iter()
                    .find(|column| column.name == field.name)
                    .map(|column| &column.ty)
            })
        });
        let query_value = match ty {
            Some(Type::Int) => value
                .parse::<i64>()
                .map(zelyra_database::QueryValue::Int)
                .map_err(|_| format!("form field `{}` is not an integer", field.name))?,
            Some(Type::Named(name)) if name == "Id" => value
                .parse::<i64>()
                .map(zelyra_database::QueryValue::Int)
                .map_err(|_| format!("form field `{}` is not an integer", field.name))?,
            Some(Type::UInt) => value
                .parse::<u64>()
                .map(zelyra_database::QueryValue::UInt)
                .map_err(|_| format!("form field `{}` is not an unsigned integer", field.name))?,
            Some(Type::Float) | Some(Type::Decimal) => value
                .parse::<f64>()
                .map(zelyra_database::QueryValue::Float)
                .map_err(|_| format!("form field `{}` is not a number", field.name))?,
            Some(Type::Named(name)) if name == "Money" => value
                .parse::<f64>()
                .map(zelyra_database::QueryValue::Float)
                .map_err(|_| format!("form field `{}` is not a number", field.name))?,
            Some(Type::Bool) => {
                zelyra_database::QueryValue::Bool(matches!(value.as_str(), "true" | "1"))
            }
            _ => zelyra_database::QueryValue::String(value.clone()),
        };
        parameters.push((field.name.clone(), query_value));
    }
    Ok(parameters)
}

pub fn render_form(
    route: &FormRoute,
    values: &HashMap<String, String>,
    errors: &[FieldError],
    notice: Option<&str>,
) -> String {
    let mut html = String::new();
    html.push_str("<form method=\"post\" action=\"");
    html.push_str(&html_escape(&route.action));
    html.push_str("\">");
    html.push_str("<input type=\"hidden\" name=\"_zelyra_csrf\" value=\"");
    html.push_str(&html_escape(route.csrf.token()));
    html.push_str("\">");
    if let Some(notice) = notice {
        html.push_str("<p class=\"zelyra-notice\">");
        html.push_str(&html_escape(notice));
        html.push_str("</p>");
    }
    for field in &route.form.fields {
        let label = field.label.clone().unwrap_or_else(|| humanize(&field.name));
        let value = values.get(&field.name).map(String::as_str).unwrap_or("");
        let field_errors = errors
            .iter()
            .filter(|error| error.field == field.name)
            .collect::<Vec<_>>();
        let input_type = input_type(route, field);
        let required = is_required(route, field);
        let max = field_max(route, field);
        html.push_str("<div class=\"zelyra-field\">");
        html.push_str("<label for=\"");
        html.push_str(&html_escape(&field.name));
        html.push_str("\">");
        html.push_str(&html_escape(&label));
        html.push_str("</label>");
        html.push_str("<input id=\"");
        html.push_str(&html_escape(&field.name));
        html.push_str("\" name=\"");
        html.push_str(&html_escape(&field.name));
        html.push_str("\" type=\"");
        html.push_str(input_type);
        html.push('"');
        if input_type == "checkbox" {
            html.push_str(" value=\"true\"");
            if matches!(value, "true" | "1") {
                html.push_str(" checked");
            }
        } else {
            html.push_str(" value=\"");
            html.push_str(&html_escape(value));
            html.push('"');
        }
        if required {
            html.push_str(" required");
        }
        if let Some(max) = max {
            html.push_str(" maxlength=\"");
            html.push_str(&max.to_string());
            html.push('"');
        }
        if let Some(placeholder) = &field.placeholder {
            html.push_str(" placeholder=\"");
            html.push_str(&html_escape(placeholder));
            html.push('"');
        }
        if field.readonly {
            html.push_str(" readonly");
        }
        html.push('>');
        for error in field_errors {
            html.push_str("<p class=\"zelyra-error\">");
            html.push_str(&html_escape(&error.message));
            html.push_str("</p>");
        }
        html.push_str("</div>");
    }
    html.push_str("<button type=\"submit\">Submit</button></form>");
    html
}

fn input_type(route: &FormRoute, field: &zelyra_ast::FormField) -> &'static str {
    if let Some(widget) = field.widget.as_deref() {
        return match widget {
            "email" => "email",
            "number" => "number",
            "url" => "url",
            "checkbox" => "checkbox",
            "date" => "date",
            "time" => "time",
            _ => "text",
        };
    }
    let ty = field.ty.as_ref().or_else(|| {
        route.table.as_ref().and_then(|table| {
            table
                .columns
                .iter()
                .find(|column| column.name == field.name)
                .map(|column| &column.ty)
        })
    });
    match ty {
        Some(Type::Bool) => "checkbox",
        Some(Type::Int | Type::UInt | Type::Float | Type::Decimal) => "number",
        Some(Type::Named(name)) if name == "Email" => "email",
        Some(Type::Named(name)) if name == "Url" => "url",
        _ => "text",
    }
}

fn is_required(route: &FormRoute, field: &zelyra_ast::FormField) -> bool {
    if field.required {
        return true;
    }
    let table_required = route.table.as_ref().and_then(|table| {
        table
            .columns
            .iter()
            .find(|column| column.name == field.name)
            .map(|column| column.required || column.primary_key)
    });
    let schema_required = route.schema.as_ref().and_then(|schema| {
        route.form.table.as_deref().and_then(|table_name| {
            schema
                .tables
                .iter()
                .find(|table| table.name == table_name)
                .and_then(|table| {
                    table
                        .columns
                        .iter()
                        .find(|column| column.name == field.name)
                })
                .map(|column| !column.nullable || column.primary_key)
        })
    });
    table_required.unwrap_or(false) || schema_required.unwrap_or(false)
}

fn field_max(route: &FormRoute, field: &zelyra_ast::FormField) -> Option<u32> {
    field.max.or_else(|| {
        route.table.as_ref().and_then(|table| {
            table
                .columns
                .iter()
                .find(|column| column.name == field.name)
                .and_then(|column| column.length)
        })
    })
}

fn humanize(name: &str) -> String {
    let mut result = name.replace('_', " ");
    if let Some(first) = result.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    result
}

pub fn parse_urlencoded(body: &str) -> Result<HashMap<String, String>, HttpError> {
    let mut values = HashMap::new();
    if body.is_empty() {
        return Ok(values);
    }
    for pair in body.split('&') {
        let (key, value) = pair.split_once('=').ok_or_else(|| HttpError {
            message: "malformed form field".into(),
        })?;
        if key.is_empty() {
            return Err(HttpError {
                message: "form field name is empty".into(),
            });
        }
        values.insert(percent_decode(key)?, percent_decode(value)?);
    }
    Ok(values)
}

fn percent_decode(value: &str) -> Result<String, HttpError> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => decoded.push(b' '),
            b'%' if index + 2 < bytes.len() => {
                let high = hex_value(bytes[index + 1]).ok_or_else(|| HttpError {
                    message: "invalid percent encoding".into(),
                })?;
                let low = hex_value(bytes[index + 2]).ok_or_else(|| HttpError {
                    message: "invalid percent encoding".into(),
                })?;
                decoded.push(high * 16 + low);
                index += 2;
            }
            b'%' => {
                return Err(HttpError {
                    message: "invalid percent encoding".into(),
                })
            }
            byte => decoded.push(byte),
        }
        index += 1;
    }
    String::from_utf8(decoded).map_err(|_| HttpError {
        message: "form field is not valid UTF-8".into(),
    })
}

fn match_path(pattern: &str, path: &str) -> Option<HashMap<String, String>> {
    let pattern_parts = path_parts(pattern);
    let path_parts = path_parts(path);
    if pattern_parts.len() != path_parts.len() {
        return None;
    }
    let mut params = HashMap::new();
    for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts) {
        if let Some(name) = pattern_part
            .strip_prefix('{')
            .and_then(|part| part.strip_suffix('}'))
        {
            if name.is_empty() {
                return None;
            }
            params.insert(name.into(), path_part.into());
        } else if *pattern_part != path_part {
            return None;
        }
    }
    Some(params)
}

fn path_parts(path: &str) -> Vec<&str> {
    path.trim_matches('/')
        .split('/')
        .filter(|part| !part.is_empty())
        .collect()
}

fn render_template(template: &str, params: &HashMap<String, String>) -> String {
    let mut rendered = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        rendered.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('}') else {
            rendered.push_str(&rest[open..]);
            return rendered;
        };
        let name = &after_open[..close];
        if let Some(value) = params.get(name) {
            rendered.push_str(&html_escape(value));
        } else {
            rendered.push('{');
            rendered.push_str(name);
            rendered.push('}');
        }
        rest = &after_open[close + 1..];
    }
    rendered.push_str(rest);
    rendered
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 0x0f) as usize] as char);
    }
    result
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    let mut difference = (left.len() ^ right.len()) as u8;
    for index in 0..left.len().max(right.len()) {
        difference |=
            left.get(index).copied().unwrap_or(0) ^ right.get(index).copied().unwrap_or(0);
    }
    difference == 0
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        422 => "Unprocessable Entity",
        303 => "See Other",
        _ => "Response",
    }
}

pub fn serve(routes: Vec<Route>, address: &str) -> io::Result<()> {
    serve_app(WebApp::new(routes, Vec::new()), address)
}

pub fn serve_app(app: WebApp, address: &str) -> io::Result<()> {
    let listener = TcpListener::bind(address)?;
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => handle_connection(&mut stream, &app)?,
            Err(error) => eprintln!("zelyra web: connection failed: {error}"),
        }
    }
    Ok(())
}

fn handle_connection(stream: &mut TcpStream, app: &WebApp) -> io::Result<()> {
    let mut buffer = [0_u8; 16 * 1024];
    let size = stream.read(&mut buffer)?;
    let response = match std::str::from_utf8(&buffer[..size])
        .map_err(|error| HttpError {
            message: error.to_string(),
        })
        .and_then(parse_request)
    {
        Ok(request) => app.dispatch(&request),
        Err(_) => Response::html(400, "<h1>400 Bad Request</h1>"),
    };
    stream.write_all(response.to_http().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn router() -> Router {
        Router::new(vec![Route {
            path: "/hello/{name}".into(),
            html: "<h1>Hello, {name}!</h1>".into(),
        }])
    }

    fn form_route() -> FormRoute {
        FormRoute {
            path: "/forms/CustomerCreate".into(),
            action: "/forms/CustomerCreate".into(),
            form: FormDef {
                name: "CustomerCreate".into(),
                table: None,
                fields: vec![zelyra_ast::FormField {
                    name: "name".into(),
                    ty: Some(Type::String),
                    label: Some("Name".into()),
                    placeholder: None,
                    required: true,
                    max: Some(20),
                    widget: None,
                    readonly: false,
                    span: zelyra_ast::Span::default(),
                }],
                actions: Vec::new(),
                span: zelyra_ast::Span::default(),
            },
            table: None,
            schema: None,
            csrf: CsrfProtection::new("csrf-token"),
        }
    }

    #[test]
    fn dispatches_literal_and_parameter_routes() {
        let response = router().dispatch("GET", "/hello/Zelyra");
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "<h1>Hello, Zelyra!</h1>");
    }

    #[test]
    fn escapes_route_parameters() {
        let response = router().dispatch("GET", "/hello/<script>");
        assert_eq!(response.body, "<h1>Hello, &lt;script&gt;!</h1>");
    }

    #[test]
    fn returns_not_found_and_method_errors() {
        assert_eq!(router().dispatch("GET", "/missing").status, 404);
        assert_eq!(router().dispatch("POST", "/hello/Ada").status, 405);
    }

    #[test]
    fn parses_request_and_query_string() {
        let request = parse_request(
            "GET /hello/Ada?active=true HTTP/1.1\r\nHost: localhost\r\nAccept: text/html\r\n\r\n",
        )
        .unwrap();
        assert_eq!(request.path, "/hello/Ada");
        assert_eq!(request.target, "/hello/Ada?active=true");
        assert_eq!(request.headers["host"], "localhost");
    }

    #[test]
    fn serializes_http_response() {
        let wire = Response::html(200, "ok").to_http();
        assert!(wire.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(wire.contains("Content-Length: 2\r\n"));
        assert!(wire.ends_with("\r\n\r\nok"));
    }

    #[test]
    fn serializes_redirect_response() {
        let wire = Response::redirect("/customers").to_http();
        assert!(wire.starts_with("HTTP/1.1 303 See Other\r\n"));
        assert!(wire.contains("Location: /customers\r\n"));
    }

    #[test]
    fn renders_form_with_csrf_and_field_attributes() {
        let html = render_form(&form_route(), &HashMap::new(), &[], None);
        assert!(html.contains("name=\"_zelyra_csrf\" value=\"csrf-token\""));
        assert!(html.contains("name=\"name\" type=\"text\""));
        assert!(html.contains(" required"));
        assert!(html.contains("maxlength=\"20\""));
    }

    #[test]
    fn form_post_requires_csrf_and_reports_validation_errors() {
        let app = WebApp::new(Vec::new(), vec![form_route()]);
        let invalid_csrf = parse_request(
            "POST /forms/CustomerCreate HTTP/1.1\r\nContent-Length: 24\r\n\r\n_zelyra_csrf=wrong",
        )
        .unwrap();
        assert_eq!(app.dispatch(&invalid_csrf).status, 403);

        let missing_name =
            parse_request("POST /forms/CustomerCreate HTTP/1.1\r\n\r\n_zelyra_csrf=csrf-token")
                .unwrap();
        let response = app.dispatch(&missing_name);
        assert_eq!(response.status, 422);
        assert!(response.body.contains("value is required"));
    }

    #[test]
    fn form_post_returns_accepted_after_validating_input() {
        let app = WebApp::new(Vec::new(), vec![form_route()]);
        let request = parse_request(
            "POST /forms/CustomerCreate HTTP/1.1\r\n\r\n_zelyra_csrf=csrf-token&name=Anna",
        )
        .unwrap();
        let response = app.dispatch(&request);
        assert_eq!(response.status, 202);
        assert!(response.body.contains("Input validated"));
    }

    #[test]
    fn form_action_requires_database_url() {
        let mut route = form_route();
        route.form.actions.push(zelyra_ast::FormAction {
            name: "save".into(),
            statements: vec![zelyra_ast::Stmt::Expr(zelyra_ast::Expr {
                kind: zelyra_ast::ExprKind::Sql {
                    result_type: Type::Unit,
                    query: "INSERT INTO customers (name) VALUES (:name)".into(),
                },
                span: zelyra_ast::Span::default(),
            })],
            success: Some("Saved".into()),
            redirect: Some("/customers".into()),
            span: zelyra_ast::Span::default(),
        });
        let app = WebApp::new(Vec::new(), vec![route]);
        let request = parse_request(
            "POST /forms/CustomerCreate HTTP/1.1\r\n\r\n_zelyra_csrf=csrf-token&name=Anna",
        )
        .unwrap();
        assert_eq!(app.dispatch(&request).status, 503);
    }
}
