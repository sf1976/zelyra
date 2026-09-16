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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

#[derive(Clone, Debug)]
pub struct CrudRoute {
    pub path: String,
    pub title: String,
    pub table: String,
    pub schema: Schema,
}

#[derive(Clone, Debug)]
pub struct WebApp {
    pub routes: Vec<Route>,
    pub forms: Vec<FormRoute>,
    pub cruds: Vec<CrudRoute>,
    pub database_url: Option<String>,
}

impl WebApp {
    pub fn new(routes: Vec<Route>, forms: Vec<FormRoute>) -> Self {
        Self {
            routes,
            forms,
            cruds: Vec::new(),
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
            cruds: Vec::new(),
            database_url,
        }
    }

    pub fn with_cruds(mut self, cruds: Vec<CrudRoute>) -> Self {
        self.cruds = cruds;
        self
    }

    pub fn dispatch(&self, request: &Request) -> Response {
        for form in &self.forms {
            if let Some(path_params) = match_path(&form.path, &request.path) {
                return dispatch_form(form, request, &path_params, self.database_url.as_deref());
            }
        }
        for crud in &self.cruds {
            if match_path(&crud.path, &request.path).is_some() {
                return dispatch_crud(crud, request, self.database_url.as_deref());
            }
            let detail_path = format!("{}/{{id}}", crud.path.trim_end_matches('/'));
            if let Some(path_params) = match_path(&detail_path, &request.path) {
                return dispatch_crud_detail(
                    crud,
                    request,
                    &path_params,
                    self.database_url.as_deref(),
                );
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

fn dispatch_form(
    form: &FormRoute,
    request: &Request,
    path_params: &HashMap<String, String>,
    database_url: Option<&str>,
) -> Response {
    let rendered_form = form_with_path_params(form, path_params);
    if request.method == "GET" {
        let options = match load_relation_options(&rendered_form, database_url) {
            Ok(options) => options,
            Err(error) => return relation_options_error(database_url, error),
        };
        let values = if rendered_form.form.name.ends_with("Edit") {
            match load_existing_form_values(&rendered_form, path_params, database_url) {
                Ok(Some(values)) => values,
                Ok(None) => return Response::html(404, "<h1>404 Not Found</h1>"),
                Err(error) => return relation_options_error(database_url, error),
            }
        } else {
            HashMap::new()
        };
        return Response::html(
            200,
            render_form_with_options(&rendered_form, &values, &[], None, &options),
        );
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
    let relation_options = match load_relation_options(&rendered_form, database_url) {
        Ok(options) => options,
        Err(error) => return relation_options_error(database_url, error),
    };
    let validation = validate(
        &form.form,
        form.table.as_ref(),
        form.schema.as_ref(),
        &values,
    );
    let mut errors = validation.errors;
    validate_relation_values(form, &relation_options, &values, &mut errors);
    if !errors.is_empty() {
        return Response::html(
            422,
            render_form_with_options(
                &rendered_form,
                &values,
                &errors,
                Some("Please correct the errors."),
                &relation_options,
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
        if let Err(error) = execute_form_action(form, action, &values, path_params, database_url) {
            eprintln!("zelyra web: form action failed: {error}");
            return Response::html(500, "<h1>500 Internal Server Error</h1>");
        }
        return Response::redirect(action.redirect.as_deref().unwrap_or("/"));
    }
    Response::html(
        202,
        render_form_with_options(
            &rendered_form,
            &values,
            &[],
            Some("Input validated. Database action execution is not enabled yet."),
            &relation_options,
        ),
    )
}

fn form_with_path_params(form: &FormRoute, path_params: &HashMap<String, String>) -> FormRoute {
    let mut rendered_form = form.clone();
    for (name, value) in path_params {
        rendered_form.action = rendered_form.action.replace(&format!("{{{name}}}"), value);
    }
    rendered_form
}

fn load_existing_form_values(
    form: &FormRoute,
    path_params: &HashMap<String, String>,
    database_url: Option<&str>,
) -> Result<Option<HashMap<String, String>>, String> {
    let Some(database_url) = database_url else {
        return Err("DATABASE_URL is required for edit forms".into());
    };
    let Some(id) = path_params.get("id") else {
        return Err("edit form path parameter `id` is missing".into());
    };
    let id = id
        .parse::<i64>()
        .map_err(|_| "edit form path parameter `id` is not an integer".to_owned())?;
    let table_name = form
        .form
        .table
        .as_deref()
        .ok_or_else(|| "edit form has no source table".to_owned())?;
    let schema_table = form
        .schema
        .as_ref()
        .and_then(|schema| schema.tables.iter().find(|table| table.name == table_name))
        .ok_or_else(|| format!("edit form table `{table_name}` is missing from schema"))?;
    let columns = form
        .form
        .fields
        .iter()
        .map(|field| {
            schema_table
                .columns
                .iter()
                .find(|column| {
                    column.name == field.name || column.name == format!("{}_id", field.name)
                })
                .map(|column| column.name.clone())
                .unwrap_or_else(|| field.name.clone())
        })
        .collect::<Vec<_>>();
    let query = format!(
        "SELECT {} FROM {} WHERE {} = :id",
        columns
            .iter()
            .map(|column| quote_identifier(column))
            .collect::<Vec<_>>()
            .join(", "),
        quote_identifier(table_name),
        quote_identifier("id")
    );
    let result = zelyra_database::execute_mariadb_query(
        database_url,
        &query,
        vec![("id".into(), zelyra_database::QueryValue::Int(id))],
    )
    .map_err(|error| error.to_string())?;
    let Some(row) = result.rows.first() else {
        return Ok(None);
    };
    let values = form
        .form
        .fields
        .iter()
        .zip(row)
        .map(|(field, value)| (field.name.clone(), value.clone()))
        .collect();
    Ok(Some(values))
}

fn dispatch_crud(crud: &CrudRoute, request: &Request, database_url: Option<&str>) -> Response {
    if request.method != "GET" {
        return Response::html(405, "<h1>405 Method Not Allowed</h1>");
    }
    let Some(database_url) = database_url else {
        return Response::html(
            503,
            "<h1>503 Service Unavailable</h1><p>DATABASE_URL is required for CRUD lists.</p>",
        );
    };
    let query_string = request
        .target
        .split_once('?')
        .map_or("", |(_, query)| query);
    let query_values = match parse_urlencoded(query_string) {
        Ok(values) => values,
        Err(error) => {
            return Response::html(400, format!("<h1>400 Bad Request</h1><p>{error}</p>"))
        }
    };
    let search = query_values.get("search").cloned().unwrap_or_default();
    let page = positive_query_value(&query_values, "page").unwrap_or(1);
    let per_page = positive_query_value(&query_values, "per_page")
        .unwrap_or(50)
        .clamp(1, 100);
    let offset = (page.saturating_sub(1)).saturating_mul(per_page);
    let Some(table) = crud
        .schema
        .tables
        .iter()
        .find(|table| table.name == crud.table)
    else {
        return Response::html(500, "<h1>500 Internal Server Error</h1>");
    };
    let columns = table
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    if columns.is_empty() {
        return Response::html(500, "<h1>500 Internal Server Error</h1>");
    }
    let order_column = columns
        .iter()
        .copied()
        .find(|column| *column == "id")
        .unwrap_or(columns[0]);
    let text_columns = table
        .columns
        .iter()
        .filter(|column| {
            let sql_type = column.sql_type.to_ascii_uppercase();
            sql_type.contains("CHAR") || sql_type.contains("TEXT")
        })
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    let mut query = format!(
        "SELECT {} FROM {}",
        columns
            .iter()
            .map(|column| quote_identifier(column))
            .collect::<Vec<_>>()
            .join(", "),
        quote_identifier(&crud.table)
    );
    if !search.is_empty() && !text_columns.is_empty() {
        query.push_str(" WHERE ");
        query.push_str(
            &text_columns
                .iter()
                .map(|column| {
                    format!(
                        "{} LIKE CONCAT('%', :search, '%')",
                        quote_identifier(column)
                    )
                })
                .collect::<Vec<_>>()
                .join(" OR "),
        );
    }
    query.push_str(&format!(
        " ORDER BY {} LIMIT :limit OFFSET :offset",
        quote_identifier(order_column)
    ));
    let mut params = vec![
        (
            "limit".into(),
            zelyra_database::QueryValue::Int(per_page as i64),
        ),
        (
            "offset".into(),
            zelyra_database::QueryValue::Int(offset as i64),
        ),
    ];
    if !search.is_empty() && !text_columns.is_empty() {
        params.push((
            "search".into(),
            zelyra_database::QueryValue::String(search.clone()),
        ));
    }
    let result = match zelyra_database::execute_mariadb_query(database_url, &query, params) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("zelyra web: CRUD query failed: {error}");
            return Response::html(500, "<h1>500 Internal Server Error</h1>");
        }
    };
    Response::html(
        200,
        render_crud_list(crud, &columns, &result.rows, &search, page, per_page),
    )
}

fn dispatch_crud_detail(
    crud: &CrudRoute,
    request: &Request,
    path_params: &HashMap<String, String>,
    database_url: Option<&str>,
) -> Response {
    if request.method != "GET" {
        return Response::html(405, "<h1>405 Method Not Allowed</h1>");
    }
    let Some(database_url) = database_url else {
        return Response::html(
            503,
            "<h1>503 Service Unavailable</h1><p>DATABASE_URL is required for CRUD details.</p>",
        );
    };
    let Some(id) = path_params.get("id") else {
        return Response::html(400, "<h1>400 Bad Request</h1>");
    };
    let id_text = id.clone();
    let Ok(id) = id.parse::<i64>() else {
        return Response::html(400, "<h1>400 Bad Request</h1><p>id must be an integer.</p>");
    };
    let Some(table) = crud
        .schema
        .tables
        .iter()
        .find(|table| table.name == crud.table)
    else {
        return Response::html(500, "<h1>500 Internal Server Error</h1>");
    };
    let columns = table
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    let query = format!(
        "SELECT {} FROM {} WHERE {} = :id",
        columns
            .iter()
            .map(|column| quote_identifier(column))
            .collect::<Vec<_>>()
            .join(", "),
        quote_identifier(&crud.table),
        quote_identifier("id")
    );
    let result = match zelyra_database::execute_mariadb_query(
        database_url,
        &query,
        vec![("id".into(), zelyra_database::QueryValue::Int(id))],
    ) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("zelyra web: CRUD detail query failed: {error}");
            return Response::html(500, "<h1>500 Internal Server Error</h1>");
        }
    };
    let Some(row) = result.rows.first() else {
        return Response::html(404, "<h1>404 Not Found</h1>");
    };
    Response::html(200, render_crud_detail(crud, &columns, row, &id_text))
}

fn positive_query_value(values: &HashMap<String, String>, name: &str) -> Option<u64> {
    values
        .get(name)
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
}

fn render_crud_list(
    crud: &CrudRoute,
    columns: &[&str],
    rows: &[Vec<String>],
    search: &str,
    page: u64,
    per_page: u64,
) -> String {
    let mut html = String::from("<main><h1>");
    html.push_str(&html_escape(&crud.title));
    html.push_str("</h1><form method=\"get\" action=\"");
    html.push_str(&html_escape(&crud.path));
    html.push_str(
        "\"><label for=\"search\">Search</label><input id=\"search\" name=\"search\" value=\"",
    );
    html.push_str(&html_escape(search));
    html.push_str("\"><button type=\"submit\">Search</button></form>");
    if rows.is_empty() {
        html.push_str("<p>No records found.</p>");
    } else {
        html.push_str("<table><thead><tr>");
        for column in columns {
            html.push_str("<th>");
            html.push_str(&html_escape(&humanize(column)));
            html.push_str("</th>");
        }
        html.push_str("</tr></thead><tbody>");
        for row in rows {
            html.push_str("<tr>");
            for (index, value) in row.iter().enumerate() {
                html.push_str("<td>");
                if columns.get(index) == Some(&"id") {
                    html.push_str("<a href=\"");
                    html.push_str(&html_escape(&format!("{}/{}", crud.path, value)));
                    html.push_str("\">");
                    html.push_str(&html_escape(value));
                    html.push_str("</a>");
                } else {
                    html.push_str(&html_escape(value));
                }
                html.push_str("</td>");
            }
            html.push_str("</tr>");
        }
        html.push_str("</tbody></table>");
    }
    html.push_str("<nav class=\"zelyra-pagination\">");
    if page > 1 {
        html.push_str("<a href=\"");
        html.push_str(&html_escape(&crud_page_url(
            &crud.path,
            search,
            page - 1,
            per_page,
        )));
        html.push_str("\">Previous</a> ");
    }
    html.push_str("<span>Page ");
    html.push_str(&page.to_string());
    html.push_str("</span>");
    if rows.len() as u64 == per_page {
        html.push_str(" <a href=\"");
        html.push_str(&html_escape(&crud_page_url(
            &crud.path,
            search,
            page + 1,
            per_page,
        )));
        html.push_str("\">Next</a>");
    }
    html.push_str("</nav></main>");
    html
}

fn render_crud_detail(crud: &CrudRoute, columns: &[&str], row: &[String], id: &str) -> String {
    let mut html = String::from("<main><p><a href=\"");
    html.push_str(&html_escape(&crud.path));
    html.push_str("\">Back to list</a></p><h1>");
    html.push_str(&html_escape(&crud.title));
    html.push_str(" detail</h1><dl>");
    for (column, value) in columns.iter().zip(row) {
        html.push_str("<dt>");
        html.push_str(&html_escape(&humanize(column)));
        html.push_str("</dt><dd>");
        html.push_str(&html_escape(value));
        html.push_str("</dd>");
    }
    html.push_str("</dl><p><a href=\"");
    html.push_str(&html_escape(&format!("{}/{}/edit", crud.path, id)));
    html.push_str("\">Edit</a> <a href=\"");
    html.push_str(&html_escape(&format!("{}/new", crud.path)));
    html.push_str("\">Create new</a></p></main>");
    html
}

fn crud_page_url(path: &str, search: &str, page: u64, per_page: u64) -> String {
    let mut url = format!("{path}?page={page}&per_page={per_page}");
    if !search.is_empty() {
        url.push_str("&search=");
        url.push_str(&url_encode(search));
    }
    url
}

fn url_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn relation_options_error(database_url: Option<&str>, error: String) -> Response {
    eprintln!("zelyra web: could not load relation options: {error}");
    if database_url.is_none() {
        Response::html(
            503,
            "<h1>503 Service Unavailable</h1><p>DATABASE_URL is required for relationship fields.</p>",
        )
    } else {
        Response::html(500, "<h1>500 Internal Server Error</h1>")
    }
}

fn load_relation_options(
    form: &FormRoute,
    database_url: Option<&str>,
) -> Result<HashMap<String, Vec<SelectOption>>, String> {
    let mut options = HashMap::new();
    for field in &form.form.fields {
        let Some(target_table) = relation_target_for_field(form, field) else {
            continue;
        };
        let Some(database_url) = database_url else {
            return Err(format!(
                "relationship field `{}` requires DATABASE_URL",
                field.name
            ));
        };
        let schema_table = form
            .schema
            .as_ref()
            .and_then(|schema| {
                schema
                    .tables
                    .iter()
                    .find(|table| table.name == target_table)
            })
            .ok_or_else(|| format!("relationship table `{target_table}` is missing from schema"))?;
        let display_column = relation_display_column(schema_table);
        let query = format!(
            "SELECT {}, {} FROM {} ORDER BY {}",
            quote_identifier("id"),
            quote_identifier(display_column),
            quote_identifier(&target_table),
            quote_identifier(display_column)
        );
        let result = zelyra_database::execute_mariadb_query(database_url, &query, Vec::new())
            .map_err(|error| error.to_string())?;
        let field_options = result
            .rows
            .into_iter()
            .filter_map(|row| match row.as_slice() {
                [value, label, ..] => Some(SelectOption {
                    value: value.clone(),
                    label: label.clone(),
                }),
                _ => None,
            })
            .collect();
        options.insert(field.name.clone(), field_options);
    }
    Ok(options)
}

fn validate_relation_values(
    form: &FormRoute,
    options: &HashMap<String, Vec<SelectOption>>,
    values: &HashMap<String, String>,
    errors: &mut Vec<FieldError>,
) {
    for field in &form.form.fields {
        let Some(field_options) = options.get(&field.name) else {
            continue;
        };
        let Some(value) = values.get(&field.name) else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        if !field_options.iter().any(|option| option.value == *value) {
            errors.push(FieldError {
                field: field.name.clone(),
                message: "selected value does not exist".into(),
            });
        }
    }
}

fn relation_target_for_field(form: &FormRoute, field: &zelyra_ast::FormField) -> Option<String> {
    let ty = field.ty.as_ref().or_else(|| {
        form.table.as_ref().and_then(|table| {
            table
                .columns
                .iter()
                .find(|column| column.name == field.name)
                .map(|column| &column.ty)
        })
    });
    let Type::Named(name) = ty? else {
        return None;
    };
    if matches!(name.as_str(), "Id" | "Email" | "Url" | "Uuid" | "Money") {
        return None;
    }
    let schema = form.schema.as_ref()?;
    let lower = name.to_ascii_lowercase();
    if schema.tables.iter().any(|table| table.name == lower) {
        return Some(lower);
    }
    let plural = if lower.ends_with('y') {
        format!("{}ies", &lower[..lower.len() - 1])
    } else {
        format!("{lower}s")
    };
    schema
        .tables
        .iter()
        .any(|table| table.name == plural)
        .then_some(plural)
}

fn relation_display_column(table: &zelyra_database::Table) -> &str {
    ["name", "number", "title", "email"]
        .iter()
        .find(|candidate| {
            table
                .columns
                .iter()
                .any(|column| column.name == **candidate)
        })
        .copied()
        .unwrap_or("id")
}

fn quote_identifier(identifier: &str) -> String {
    format!("`{}`", identifier.replace('`', "``"))
}

fn execute_form_action(
    form: &FormRoute,
    action: &zelyra_ast::FormAction,
    values: &HashMap<String, String>,
    path_params: &HashMap<String, String>,
    database_url: &str,
) -> Result<(), String> {
    let mut parameters = form_query_parameters(form, values)?;
    for (name, value) in path_params {
        let parsed = value
            .parse::<i64>()
            .map(zelyra_database::QueryValue::Int)
            .map_err(|_| format!("path parameter `{name}` is not an integer"))?;
        parameters.push((name.clone(), parsed));
    }
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
            params: parameters.clone(),
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
            Some(Type::Named(_)) if relation_target_for_field(form, field).is_some() => value
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
    render_form_with_options(route, values, errors, notice, &HashMap::new())
}

fn render_form_with_options(
    route: &FormRoute,
    values: &HashMap<String, String>,
    errors: &[FieldError],
    notice: Option<&str>,
    relation_options: &HashMap<String, Vec<SelectOption>>,
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
        let required = is_required(route, field);
        let max = field_max(route, field);
        html.push_str("<div class=\"zelyra-field\">");
        html.push_str("<label for=\"");
        html.push_str(&html_escape(&field.name));
        html.push_str("\">");
        html.push_str(&html_escape(&label));
        html.push_str("</label>");
        if let Some(options) = relation_options.get(&field.name) {
            html.push_str("<select id=\"");
            html.push_str(&html_escape(&field.name));
            html.push_str("\" name=\"");
            html.push_str(&html_escape(&field.name));
            html.push('"');
            if required {
                html.push_str(" required");
            }
            html.push('>');
            if !required {
                html.push_str("<option value=\"\">-- Select --</option>");
            }
            for option in options {
                html.push_str("<option value=\"");
                html.push_str(&html_escape(&option.value));
                html.push('"');
                if option.value == value {
                    html.push_str(" selected");
                }
                html.push('>');
                html.push_str(&html_escape(&option.label));
                html.push_str("</option>");
            }
            html.push_str("</select>");
        } else {
            let input_type = input_type(route, field);
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
        }
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

    fn relation_form_route() -> FormRoute {
        let mut route = form_route();
        route.form.fields.push(zelyra_ast::FormField {
            name: "department".into(),
            ty: None,
            label: Some("Department".into()),
            placeholder: None,
            required: true,
            max: None,
            widget: None,
            readonly: false,
            span: zelyra_ast::Span::default(),
        });
        route.table = Some(zelyra_ast::TableDef {
            name: "customers".into(),
            columns: vec![zelyra_ast::ColumnDef {
                name: "department".into(),
                ty: Type::Named("Department".into()),
                length: None,
                required: true,
                primary_key: false,
                auto: false,
                unique: false,
                default: None,
                span: zelyra_ast::Span::default(),
            }],
            indexes: Vec::new(),
            uniques: Vec::new(),
            span: zelyra_ast::Span::default(),
        });
        route.schema = Some(zelyra_database::Schema {
            database: None,
            tables: vec![zelyra_database::Table {
                name: "departments".into(),
                columns: vec![
                    zelyra_database::Column {
                        name: "id".into(),
                        sql_type: "BIGINT".into(),
                        nullable: false,
                        primary_key: true,
                        auto: true,
                        unique: false,
                        default: None,
                    },
                    zelyra_database::Column {
                        name: "name".into(),
                        sql_type: "VARCHAR(100)".into(),
                        nullable: false,
                        primary_key: false,
                        auto: false,
                        unique: false,
                        default: None,
                    },
                ],
                foreign_keys: Vec::new(),
                indexes: Vec::new(),
                uniques: Vec::new(),
            }],
        });
        route
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
    fn renders_relationship_as_escaped_select_options() {
        let route = relation_form_route();
        let options = HashMap::from([(
            "department".into(),
            vec![SelectOption {
                value: "7".into(),
                label: "R&D <East>".into(),
            }],
        )]);
        let html = render_form_with_options(
            &route,
            &HashMap::from([(String::from("department"), String::from("7"))]),
            &[],
            None,
            &options,
        );
        assert!(html.contains("<select id=\"department\" name=\"department\" required>"));
        assert!(html.contains("value=\"7\" selected>R&amp;D &lt;East&gt;</option>"));
        assert!(!html.contains("<input id=\"department\""));
    }

    #[test]
    fn rejects_unknown_relationship_value() {
        let route = relation_form_route();
        let options = HashMap::from([(
            "department".into(),
            vec![SelectOption {
                value: "7".into(),
                label: "R&D".into(),
            }],
        )]);
        let values = HashMap::from([(String::from("department"), String::from("99"))]);
        let mut errors = Vec::new();
        validate_relation_values(&route, &options, &values, &mut errors);
        assert_eq!(errors[0].field, "department");
        assert_eq!(errors[0].message, "selected value does not exist");
    }

    #[test]
    fn renders_crud_list_with_escaped_rows_and_pagination() {
        let route = CrudRoute {
            path: "/machines".into(),
            title: "Machines".into(),
            table: "machines".into(),
            schema: zelyra_database::Schema {
                database: None,
                tables: Vec::new(),
            },
        };
        let html = render_crud_list(
            &route,
            &["id", "name"],
            &[vec!["1".into(), "<unsafe>".into()]],
            "CNC machine",
            2,
            1,
        );
        assert!(html.contains("&lt;unsafe&gt;"));
        assert!(html.contains("value=\"CNC machine\""));
        assert!(html.contains("page=1&amp;per_page=1&amp;search=CNC%20machine"));
        assert!(html.contains("page=3&amp;per_page=1&amp;search=CNC%20machine"));
    }

    #[test]
    fn crud_requires_database_url() {
        let app = WebApp::new(Vec::new(), Vec::new()).with_cruds(vec![CrudRoute {
            path: "/machines".into(),
            title: "Machines".into(),
            table: "machines".into(),
            schema: zelyra_database::Schema {
                database: None,
                tables: Vec::new(),
            },
        }]);
        let request = parse_request("GET /machines HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(app.dispatch(&request).status, 503);
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
