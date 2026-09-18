use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use blake2::{Blake2s256, Digest};
use rand_core::OsRng;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use zelyra_ast::{
    CrudDeleteViewDef, CrudDetailViewDef, CrudDetailViewMode, CrudFormViewDef, CrudFormViewMode,
    CrudListViewDef, CrudListViewMode, FormDef, TableDef, Type,
};
use zelyra_database::Schema;
use zelyra_forms::{validate, FieldError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub path: String,
    pub html: String,
    pub requires_auth: bool,
    pub permissions: Vec<String>,
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
pub struct CorsPolicy {
    pub allowed_origins: Vec<String>,
    pub allow_credentials: bool,
}

impl CorsPolicy {
    pub fn new(allowed_origins: Vec<String>, allow_credentials: bool) -> Result<Self, HttpError> {
        if let Some(origin) = allowed_origins.iter().find(|origin| !valid_origin(origin)) {
            return Err(HttpError {
                message: format!("invalid CORS origin `{origin}`"),
            });
        }
        Ok(Self {
            allowed_origins,
            allow_credentials,
        })
    }

    fn allows(&self, origin: &str) -> bool {
        self.allowed_origins.iter().any(|allowed| allowed == origin)
    }
}

fn valid_origin(origin: &str) -> bool {
    let Some(host) = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
    else {
        return false;
    };
    !host.is_empty()
        && !host.contains(['/', '?', '#', '*', ' ', '\t', '\r', '\n'])
        && !origin.ends_with('/')
}

type ApiHandler = dyn Fn(&Request, &HashMap<String, String>) -> Response + Send + Sync;

#[derive(Clone)]
pub struct ApiRoute {
    pub method: String,
    pub path: String,
    pub requires_auth: bool,
    pub permissions: Vec<String>,
    handler: Arc<ApiHandler>,
}

impl fmt::Debug for ApiRoute {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApiRoute")
            .field("method", &self.method)
            .field("path", &self.path)
            .field("requires_auth", &self.requires_auth)
            .field("permissions", &self.permissions)
            .finish_non_exhaustive()
    }
}

impl ApiRoute {
    pub fn new(
        method: impl Into<String>,
        path: impl Into<String>,
        handler: impl Fn(&Request, &HashMap<String, String>) -> Response + Send + Sync + 'static,
    ) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            requires_auth: false,
            permissions: Vec::new(),
            handler: Arc::new(handler),
        }
    }

    pub fn with_auth(mut self, requires_auth: bool, permissions: Vec<String>) -> Self {
        self.requires_auth = requires_auth;
        self.permissions = permissions;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub reason: String,
    pub content_type: String,
    pub body: String,
    pub location: Option<String>,
    pub headers: Vec<(String, String)>,
}

impl Response {
    pub fn html(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            reason: reason_phrase(status).into(),
            content_type: "text/html; charset=utf-8".into(),
            body: body.into(),
            location: None,
            headers: Vec::new(),
        }
    }

    pub fn json(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            reason: reason_phrase(status).into(),
            content_type: "application/json; charset=utf-8".into(),
            body: body.into(),
            location: None,
            headers: Vec::new(),
        }
    }

    pub fn empty(status: u16) -> Self {
        Self {
            status,
            reason: reason_phrase(status).into(),
            content_type: "text/plain; charset=utf-8".into(),
            body: String::new(),
            location: None,
            headers: Vec::new(),
        }
    }

    pub fn to_http(&self) -> String {
        format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\nReferrer-Policy: no-referrer\r\n{}{}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
            self.status,
            self.reason,
            self.content_type,
            self.location
                .as_deref()
                .map_or(String::new(), |location| format!("Location: {location}\r\n")),
            self.headers
                .iter()
                .map(|(name, value)| format!("{name}: {value}\r\n"))
                .collect::<String>(),
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
            headers: Vec::new(),
        }
    }

    fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
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

/// Creates an Argon2 password hash suitable for the `password_hash` column of
/// an authenticated user table.
pub fn hash_password(password: &str) -> Result<String, String> {
    if password.is_empty() {
        return Err("password must not be empty".into());
    }
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| format!("could not hash password: {error}"))
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
    pub requires_auth: bool,
    pub permissions: Vec<String>,
    pub csrf: CsrfProtection,
    pub form_view: CrudFormViewDef,
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
    pub list_columns: Vec<String>,
    pub search_columns: Vec<String>,
    pub filter_columns: Vec<String>,
    pub list_view: CrudListViewDef,
    pub detail_view: CrudDetailViewDef,
    pub delete_view: CrudDeleteViewDef,
    pub requires_auth: bool,
    pub permissions: Vec<String>,
    pub create_permissions: Vec<String>,
    pub edit_permissions: Vec<String>,
    pub delete_permissions: Vec<String>,
    pub schema: Schema,
    pub csrf: CsrfProtection,
}

#[derive(Clone, Debug)]
pub struct TableViewRoute {
    pub path: String,
    pub title: String,
    pub source: String,
    pub columns: Vec<String>,
    pub filters: Vec<TableViewFilter>,
    pub searchable: bool,
    pub sortable: bool,
    pub page_size: Option<u32>,
    pub requires_auth: bool,
    pub permissions: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableViewFilterKind {
    Text,
    Numeric,
    Bool,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableViewFilter {
    pub name: String,
    pub kind: TableViewFilterKind,
}

#[derive(Clone, Debug)]
pub struct AuthRoute {
    pub table: String,
    pub session_table: Option<String>,
    pub permissions_table: Option<String>,
    pub roles_table: Option<String>,
    pub role_permissions_table: Option<String>,
    pub audit_table: Option<String>,
    pub admin_path: Option<String>,
    pub admin_permission: Option<String>,
    pub admin_role: Option<String>,
    pub schema: Schema,
    pub csrf: CsrfProtection,
}

#[derive(Clone, Debug)]
struct Session {
    user_id: Option<i64>,
    permissions: Vec<String>,
}

#[derive(Clone, Debug)]
struct LoginThrottle {
    window_started: Instant,
    failures: u32,
    blocked_until: Option<Instant>,
}

const LOGIN_FAILURE_LIMIT: u32 = 5;
const LOGIN_FAILURE_WINDOW: Duration = Duration::from_secs(15 * 60);
const LOGIN_BLOCK_DURATION: Duration = Duration::from_secs(60);

#[derive(Clone, Debug)]
pub struct WebApp {
    pub routes: Vec<Route>,
    pub apis: Vec<ApiRoute>,
    pub forms: Vec<FormRoute>,
    pub cruds: Vec<CrudRoute>,
    pub tableviews: Vec<TableViewRoute>,
    pub database_url: Option<String>,
    pub database_capability_granted: Option<bool>,
    pub auth_token: Option<String>,
    pub auth_permissions: Vec<String>,
    pub auth_route: Option<AuthRoute>,
    pub cors_policy: Option<CorsPolicy>,
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    login_throttle: Arc<Mutex<HashMap<String, LoginThrottle>>>,
}

impl WebApp {
    pub fn new(routes: Vec<Route>, forms: Vec<FormRoute>) -> Self {
        Self {
            routes,
            apis: Vec::new(),
            forms,
            cruds: Vec::new(),
            tableviews: Vec::new(),
            database_url: None,
            database_capability_granted: None,
            auth_token: None,
            auth_permissions: Vec::new(),
            auth_route: None,
            cors_policy: None,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            login_throttle: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_database_url(
        routes: Vec<Route>,
        forms: Vec<FormRoute>,
        database_url: Option<String>,
    ) -> Self {
        Self {
            routes,
            apis: Vec::new(),
            forms,
            cruds: Vec::new(),
            tableviews: Vec::new(),
            database_url,
            database_capability_granted: None,
            auth_token: None,
            auth_permissions: Vec::new(),
            auth_route: None,
            cors_policy: None,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            login_throttle: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_cruds(mut self, cruds: Vec<CrudRoute>) -> Self {
        self.cruds = cruds;
        self
    }

    pub fn with_tableviews(mut self, tableviews: Vec<TableViewRoute>) -> Self {
        self.tableviews = tableviews;
        self
    }

    pub fn with_database_capability(mut self, granted: bool) -> Self {
        self.database_capability_granted = Some(granted);
        self
    }

    pub fn with_apis(mut self, apis: Vec<ApiRoute>) -> Self {
        self.apis = apis;
        self
    }

    pub fn with_cors(mut self, policy: CorsPolicy) -> Self {
        self.cors_policy = Some(policy);
        self
    }

    pub fn with_auth(mut self, token: Option<String>, permissions: Vec<String>) -> Self {
        self.auth_token = token;
        self.auth_permissions = permissions;
        self
    }

    pub fn with_auth_route(mut self, auth_route: AuthRoute) -> Self {
        self.auth_route = Some(auth_route);
        self
    }

    pub fn dispatch(&self, request: &Request) -> Response {
        if let Some(auth_route) = &self.auth_route {
            if request.path == "/login" {
                return dispatch_login(self, auth_route, request, self.database_url.as_deref());
            }
            if request.path == "/logout" {
                return dispatch_logout(self, request, self.database_url.as_deref());
            }
            if auth_route
                .admin_path
                .as_deref()
                .is_some_and(|path| path == request.path)
            {
                return dispatch_auth_admin(
                    self,
                    auth_route,
                    request,
                    self.database_url.as_deref(),
                );
            }
        }
        for api in &self.apis {
            if let Some(path_params) = match_path(&api.path, &request.path) {
                if request.method == "OPTIONS" {
                    return self.api_preflight(&request.path, request);
                }
                if api.method != request.method {
                    return self.apply_api_cors(
                        request,
                        Response::json(
                        405,
                        "{\"error\":{\"code\":\"MethodNotAllowed\",\"message\":\"method not allowed\"}}",
                        )
                        .with_header("Allow", self.api_allowed_methods(&request.path)),
                    );
                }
                if let Some(response) = authorize_api(
                    api.requires_auth,
                    &api.permissions,
                    request,
                    self,
                    self.database_url.as_deref(),
                ) {
                    return self.apply_api_cors(request, response);
                }
                return self.apply_api_cors(request, (api.handler)(request, &path_params));
            }
        }
        for form in &self.forms {
            if let Some(path_params) = match_path(&form.path, &request.path) {
                if self.database_capability_granted == Some(false) {
                    return database_capability_denied();
                }
                let (requires_auth, permissions) = form_authorization(form);
                if let Some(response) = authorize(
                    requires_auth,
                    &permissions,
                    request,
                    self,
                    self.database_url.as_deref(),
                ) {
                    return response;
                }
                return dispatch_form(form, request, &path_params, self.database_url.as_deref());
            }
        }
        for crud in &self.cruds {
            if match_path(&crud.path, &request.path).is_some() {
                if self.database_capability_granted == Some(false) {
                    return database_capability_denied();
                }
                if let Some(response) = authorize(
                    crud.requires_auth,
                    &crud.permissions,
                    request,
                    self,
                    self.database_url.as_deref(),
                ) {
                    return response;
                }
                return dispatch_crud(
                    crud,
                    request,
                    self.database_url.as_deref(),
                    crud_ui_actions(crud, request, self),
                );
            }
            let delete_path = format!("{}/{{id}}/delete", crud.path.trim_end_matches('/'));
            if let Some(path_params) = match_path(&delete_path, &request.path) {
                if self.database_capability_granted == Some(false) {
                    return database_capability_denied();
                }
                if let Some(response) = authorize(
                    crud.requires_auth,
                    &crud.delete_permissions,
                    request,
                    self,
                    self.database_url.as_deref(),
                ) {
                    return response;
                }
                return dispatch_crud_delete(
                    crud,
                    request,
                    &path_params,
                    self.database_url.as_deref(),
                );
            }
            let detail_path = format!("{}/{{id}}", crud.path.trim_end_matches('/'));
            if let Some(path_params) = match_path(&detail_path, &request.path) {
                if self.database_capability_granted == Some(false) {
                    return database_capability_denied();
                }
                if let Some(response) = authorize(
                    crud.requires_auth,
                    &crud.permissions,
                    request,
                    self,
                    self.database_url.as_deref(),
                ) {
                    return response;
                }
                return dispatch_crud_detail(
                    crud,
                    request,
                    &path_params,
                    self.database_url.as_deref(),
                    crud_ui_actions(crud, request, self),
                );
            }
        }
        for tableview in &self.tableviews {
            if match_path(&tableview.path, &request.path).is_some() {
                if self.database_capability_granted == Some(false) {
                    return database_capability_denied();
                }
                if let Some(response) = authorize(
                    tableview.requires_auth,
                    &tableview.permissions,
                    request,
                    self,
                    self.database_url.as_deref(),
                ) {
                    return response;
                }
                return dispatch_tableview(tableview, request, self.database_url.as_deref());
            }
        }
        for route in &self.routes {
            if match_path(&route.path, &request.path).is_some() {
                if let Some(response) = authorize(
                    route.requires_auth,
                    &route.permissions,
                    request,
                    self,
                    self.database_url.as_deref(),
                ) {
                    return response;
                }
                break;
            }
        }
        Router::new(self.routes.clone()).dispatch(&request.method, &request.target)
    }

    fn api_allowed_methods(&self, path: &str) -> String {
        let mut methods = self
            .apis
            .iter()
            .filter(|api| match_path(&api.path, path).is_some())
            .map(|api| api.method.clone())
            .collect::<Vec<_>>();
        methods.sort();
        methods.dedup();
        methods.join(", ")
    }

    fn api_preflight(&self, path: &str, request: &Request) -> Response {
        let methods = self.api_allowed_methods(path);
        let origin = request.headers.get("origin");
        if let Some(origin) = origin {
            let Some(policy) = &self.cors_policy else {
                return Response::json(
                    403,
                    "{\"error\":{\"code\":\"CorsDenied\",\"message\":\"CORS is not enabled\"}}",
                );
            };
            if !policy.allows(origin) {
                return Response::json(
                    403,
                    "{\"error\":{\"code\":\"CorsDenied\",\"message\":\"origin is not allowed\"}}",
                );
            }
        }
        let requested_method = request
            .headers
            .get("access-control-request-method")
            .map(String::as_str)
            .unwrap_or("GET");
        if !methods.split(", ").any(|method| method == requested_method) {
            return Response::json(
                405,
                "{\"error\":{\"code\":\"MethodNotAllowed\",\"message\":\"requested CORS method is not allowed\"}}",
            )
            .with_header("Allow", methods);
        }
        let mut response = Response::empty(204).with_header("Allow", methods.clone());
        if let Some(origin) = origin {
            response = response
                .with_header("Access-Control-Allow-Origin", origin)
                .with_header("Vary", "Origin")
                .with_header("Access-Control-Allow-Methods", methods)
                .with_header(
                    "Access-Control-Allow-Headers",
                    request
                        .headers
                        .get("access-control-request-headers")
                        .cloned()
                        .unwrap_or_default(),
                )
                .with_header("Access-Control-Max-Age", "600");
            if self
                .cors_policy
                .as_ref()
                .is_some_and(|policy| policy.allow_credentials)
            {
                response = response.with_header("Access-Control-Allow-Credentials", "true");
            }
        }
        response
    }

    fn apply_api_cors(&self, request: &Request, mut response: Response) -> Response {
        let Some(origin) = request.headers.get("origin") else {
            return response;
        };
        let Some(policy) = &self.cors_policy else {
            return response;
        };
        if policy.allows(origin) {
            response = response
                .with_header("Access-Control-Allow-Origin", origin)
                .with_header("Vary", "Origin");
            if policy.allow_credentials {
                response = response.with_header("Access-Control-Allow-Credentials", "true");
            }
        }
        response
    }
}

fn authenticated_permissions(
    app: &WebApp,
    request: &Request,
    database_url: Option<&str>,
) -> Option<Vec<String>> {
    if let Some(session) = session_from_request(app, request, database_url) {
        return Some(session.permissions);
    }
    let bearer_authenticated = app
        .auth_token
        .as_deref()
        .zip(request.headers.get("authorization").map(String::as_str))
        .is_some_and(|(expected, header)| {
            let Some(token) = header.strip_prefix("Bearer ") else {
                return false;
            };
            constant_time_equal(expected.as_bytes(), token.as_bytes())
        });
    bearer_authenticated.then(|| app.auth_permissions.clone())
}

fn authorize(
    requires_auth: bool,
    permissions: &[String],
    request: &Request,
    app: &WebApp,
    database_url: Option<&str>,
) -> Option<Response> {
    if !requires_auth && permissions.is_empty() {
        return None;
    }
    let Some(granted_permissions) = authenticated_permissions(app, request, database_url) else {
        return Some(Response::html(
            401,
            "<h1>401 Unauthorized</h1><p>Authentication is required.</p>",
        ));
    };
    if let Some(permission) = permissions.iter().find(|permission| {
        !granted_permissions
            .iter()
            .any(|granted| granted == *permission)
    }) {
        return Some(Response::html(
            403,
            format!("<h1>403 Forbidden</h1><p>Missing permission: {permission}</p>"),
        ));
    }
    None
}

fn authorize_api(
    requires_auth: bool,
    permissions: &[String],
    request: &Request,
    app: &WebApp,
    database_url: Option<&str>,
) -> Option<Response> {
    if !requires_auth && permissions.is_empty() {
        return None;
    }
    let Some(granted_permissions) = authenticated_permissions(app, request, database_url) else {
        return Some(Response::json(
            401,
            "{\"error\":{\"code\":\"Unauthorized\",\"message\":\"authentication is required\"}}",
        ));
    };
    if let Some(permission) = permissions.iter().find(|permission| {
        !granted_permissions
            .iter()
            .any(|granted| granted == *permission)
    }) {
        return Some(Response::json(
            403,
            format!(
                "{{\"error\":{{\"code\":\"Forbidden\",\"message\":\"missing permission: {}\"}}}}",
                json_escape(permission)
            ),
        ));
    }
    None
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn dispatch_login(
    app: &WebApp,
    auth: &AuthRoute,
    request: &Request,
    database_url: Option<&str>,
) -> Response {
    match request.method.as_str() {
        "GET" => Response::html(200, render_login(auth)),
        "POST" => {
            if app.database_capability_granted == Some(false) {
                return database_capability_denied();
            }
            let Some(database_url) = database_url else {
                return Response::html(
                    503,
                    "<h1>503 Service Unavailable</h1><p>DATABASE_URL is required for login.</p>",
                );
            };
            let input = match parse_urlencoded(&request.body) {
                Ok(input) => input,
                Err(error) => {
                    return Response::html(400, format!("<h1>400 Bad Request</h1><p>{error}</p>"))
                }
            };
            if !auth
                .csrf
                .verify(input.get("_zelyra_csrf").map(String::as_str))
            {
                return Response::html(403, "<h1>403 Forbidden</h1><p>Invalid CSRF token.</p>");
            }
            let email = input.get("email").cloned().unwrap_or_default();
            let password = input.get("password").cloned().unwrap_or_default();
            if email.is_empty() || password.is_empty() {
                return Response::html(
                    422,
                    "<h1>422 Unprocessable Entity</h1><p>Email and password are required.</p>",
                );
            }
            let throttle_key = login_throttle_key(&email);
            if login_is_blocked(app, &throttle_key) {
                return Response::html(429, "<h1>429 Too Many Requests</h1><p>Too many failed login attempts. Try again later.</p>")
                    .with_header("Retry-After", LOGIN_BLOCK_DURATION.as_secs().to_string());
            }
            let Some(table) = auth
                .schema
                .tables
                .iter()
                .find(|table| table.name == auth.table)
            else {
                return Response::html(500, "<h1>500 Internal Server Error</h1>");
            };
            let active_clause = if table.columns.iter().any(|column| column.name == "active") {
                " AND active = true"
            } else {
                ""
            };
            let query = format!(
                "SELECT id, email, password_hash FROM {} WHERE email = :email{} LIMIT 1",
                quote_identifier(&auth.table),
                active_clause
            );
            let result = match zelyra_database::execute_mariadb_query(
                database_url,
                &query,
                vec![("email".into(), zelyra_database::QueryValue::String(email))],
            ) {
                Ok(result) => result,
                Err(error) => {
                    eprintln!("zelyra web: login query failed: {error}");
                    return Response::html(500, "<h1>500 Internal Server Error</h1>");
                }
            };
            let valid_password = result.rows.first().is_some_and(|row| {
                row.get(2)
                    .and_then(|hash| PasswordHash::new(hash).ok())
                    .is_some_and(|hash| {
                        Argon2::default()
                            .verify_password(password.as_bytes(), &hash)
                            .is_ok()
                    })
            });
            let login_user_id = result
                .rows
                .first()
                .and_then(|row| row.first())
                .and_then(|value| value.parse::<i64>().ok());
            if !valid_password {
                if auth.audit_table.is_some() {
                    if let Err(error) = execute_auth_admin_mutation(
                        auth,
                        database_url,
                        Vec::new(),
                        None,
                        "auth.login_failed",
                        login_user_id,
                        "",
                    ) {
                        eprintln!("zelyra web: failed-login audit write failed: {error}");
                    }
                }
                record_login_failure(app, throttle_key);
                return Response::html(401, "<h1>401 Unauthorized</h1><p>Invalid credentials.</p>");
            }
            clear_login_failures(app, &throttle_key);
            if let Err(error) = rotate_existing_session(app, auth, request, Some(database_url)) {
                eprintln!("zelyra web: session rotation failed: {error}");
                return Response::html(500, "<h1>500 Internal Server Error</h1>");
            }
            let Ok(session_id) = CsrfProtection::generate().map(|csrf| csrf.token().to_owned())
            else {
                return Response::html(500, "<h1>500 Internal Server Error</h1>");
            };
            if let Some(session_table) = &auth.session_table {
                let Some(user_id) = login_user_id else {
                    return Response::html(500, "<h1>500 Internal Server Error</h1>");
                };
                let query = zelyra_database::Query {
                    sql: format!(
                        "INSERT INTO {} (user_id, token_hash, expires_at) VALUES (:user_id, :token_hash, DATE_ADD(NOW(), INTERVAL 1 DAY))",
                        quote_identifier(session_table)
                    ),
                    params: vec![
                        ("user_id".into(), zelyra_database::QueryValue::Int(user_id)),
                        (
                            "token_hash".into(),
                            zelyra_database::QueryValue::String(session_token_hash(&session_id)),
                        ),
                    ],
                };
                if let Err(error) = execute_auth_admin_mutation(
                    auth,
                    database_url,
                    vec![query],
                    Some(user_id),
                    "auth.login",
                    Some(user_id),
                    "",
                ) {
                    eprintln!("zelyra web: session creation failed: {error}");
                    return Response::html(500, "<h1>500 Internal Server Error</h1>");
                }
            } else {
                let Some(user_id) = login_user_id else {
                    return Response::html(500, "<h1>500 Internal Server Error</h1>");
                };
                let permissions =
                    match load_user_permissions(auth, database_url, user_id, &app.auth_permissions)
                    {
                        Ok(permissions) => permissions,
                        Err(error) => {
                            eprintln!("zelyra web: permission lookup failed: {error}");
                            return Response::html(500, "<h1>500 Internal Server Error</h1>");
                        }
                    };
                let Ok(mut sessions) = app.sessions.lock() else {
                    return Response::html(500, "<h1>500 Internal Server Error</h1>");
                };
                sessions.insert(
                    session_id.clone(),
                    Session {
                        user_id: Some(user_id),
                        permissions,
                    },
                );
                if auth.audit_table.is_some() {
                    if let Err(error) = execute_auth_admin_mutation(
                        auth,
                        database_url,
                        Vec::new(),
                        Some(user_id),
                        "auth.login",
                        Some(user_id),
                        "",
                    ) {
                        eprintln!("zelyra web: login audit write failed: {error}");
                    }
                }
            }
            Response::redirect("/").with_header(
                "Set-Cookie",
                format!("zelyra_session={session_id}; Path=/; HttpOnly; SameSite=Lax"),
            )
        }
        _ => Response::html(405, "<h1>405 Method Not Allowed</h1>"),
    }
}

fn login_throttle_key(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}

fn login_is_blocked(app: &WebApp, key: &str) -> bool {
    let Ok(mut throttle) = app.login_throttle.lock() else {
        return false;
    };
    let Some(state) = throttle.get_mut(key) else {
        return false;
    };
    let now = Instant::now();
    if now.duration_since(state.window_started) >= LOGIN_FAILURE_WINDOW {
        throttle.remove(key);
        return false;
    }
    if let Some(blocked_until) = state.blocked_until {
        if now < blocked_until {
            return true;
        }
        state.failures = 0;
        state.blocked_until = None;
        state.window_started = now;
    }
    false
}

fn record_login_failure(app: &WebApp, key: String) {
    let Ok(mut throttle) = app.login_throttle.lock() else {
        return;
    };
    let now = Instant::now();
    let state = throttle.entry(key).or_insert(LoginThrottle {
        window_started: now,
        failures: 0,
        blocked_until: None,
    });
    if now.duration_since(state.window_started) >= LOGIN_FAILURE_WINDOW {
        state.window_started = now;
        state.failures = 0;
        state.blocked_until = None;
    }
    state.failures = state.failures.saturating_add(1);
    if state.failures >= LOGIN_FAILURE_LIMIT {
        state.blocked_until = Some(now + LOGIN_BLOCK_DURATION);
    }
}

fn clear_login_failures(app: &WebApp, key: &str) {
    if let Ok(mut throttle) = app.login_throttle.lock() {
        throttle.remove(key);
    }
}

fn rotate_existing_session(
    app: &WebApp,
    auth: &AuthRoute,
    request: &Request,
    database_url: Option<&str>,
) -> Result<(), String> {
    let Some(session_id) = cookie_value(request, "zelyra_session") else {
        return Ok(());
    };
    if let (Some(session_table), Some(database_url)) = (&auth.session_table, database_url) {
        let query = format!(
            "DELETE FROM {} WHERE token_hash = :token_hash",
            quote_identifier(session_table)
        );
        if let Err(error) = zelyra_database::execute_mariadb_query(
            database_url,
            &query,
            vec![(
                "token_hash".into(),
                zelyra_database::QueryValue::String(session_token_hash(&session_id)),
            )],
        ) {
            return Err(error.to_string());
        }
    } else if let Ok(mut sessions) = app.sessions.lock() {
        sessions.remove(&session_id);
    }
    Ok(())
}

fn dispatch_logout(app: &WebApp, request: &Request, database_url: Option<&str>) -> Response {
    if request.method != "POST" {
        return Response::html(405, "<h1>405 Method Not Allowed</h1>");
    }
    let Some(auth) = app.auth_route.as_ref() else {
        return Response::html(404, "<h1>404 Not Found</h1>");
    };
    let input = match parse_urlencoded(&request.body) {
        Ok(input) => input,
        Err(error) => {
            return Response::html(400, format!("<h1>400 Bad Request</h1><p>{error}</p>"))
        }
    };
    if !auth
        .csrf
        .verify(input.get("_zelyra_csrf").map(String::as_str))
    {
        return Response::html(403, "<h1>403 Forbidden</h1><p>Invalid CSRF token.</p>");
    }
    if app.database_capability_granted == Some(false) && auth.session_table.is_some() {
        return database_capability_denied();
    }
    if let Some(session_id) = cookie_value(request, "zelyra_session") {
        if let Some(auth) = &app.auth_route {
            if let (Some(session_table), Some(database_url)) = (&auth.session_table, database_url) {
                let actor_user_id = session_from_request(app, request, Some(database_url))
                    .and_then(|session| session.user_id);
                let query = zelyra_database::Query {
                    sql: format!(
                        "DELETE FROM {} WHERE token_hash = :token_hash",
                        quote_identifier(session_table)
                    ),
                    params: vec![(
                        "token_hash".into(),
                        zelyra_database::QueryValue::String(session_token_hash(&session_id)),
                    )],
                };
                if let Err(error) = execute_auth_admin_mutation(
                    auth,
                    database_url,
                    vec![query],
                    actor_user_id,
                    "auth.logout",
                    actor_user_id,
                    "",
                ) {
                    eprintln!("zelyra web: session deletion failed: {error}");
                    return Response::html(500, "<h1>500 Internal Server Error</h1>");
                }
            } else if let Ok(mut sessions) = app.sessions.lock() {
                sessions.remove(&session_id);
            }
        }
    }
    Response::redirect("/login").with_header(
        "Set-Cookie",
        "zelyra_session=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax",
    )
}

fn render_login(auth: &AuthRoute) -> String {
    format!(
        "<main><h1>Login</h1><form method=\"post\" action=\"/login\">\
         <input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{}\">\
         <label for=\"email\">Email</label><input id=\"email\" name=\"email\" type=\"email\" required>\
         <label for=\"password\">Password</label><input id=\"password\" name=\"password\" type=\"password\" required>\
         <button type=\"submit\">Login</button></form></main>",
        html_escape(auth.csrf.token())
    )
}

fn dispatch_auth_admin(
    app: &WebApp,
    auth: &AuthRoute,
    request: &Request,
    database_url: Option<&str>,
) -> Response {
    if app.database_capability_granted == Some(false) {
        return database_capability_denied();
    }
    let Some(admin_permission) = auth.admin_permission.as_deref() else {
        return Response::html(404, "<h1>404 Not Found</h1>");
    };
    if let Some(response) = authorize(
        true,
        &[admin_permission.to_owned()],
        request,
        app,
        database_url,
    ) {
        return response;
    }
    let Some(database_url) = database_url else {
        return Response::html(
            503,
            "<h1>503 Service Unavailable</h1><p>DATABASE_URL is required for role administration.</p>",
        );
    };
    match request.method.as_str() {
        "GET" => match load_auth_admin_data(auth, database_url) {
            Ok(data) => Response::html(
                200,
                render_auth_admin(
                    auth,
                    &data.users,
                    &data.assignments,
                    &data.permissions,
                    &data.audit,
                ),
            ),
            Err(error) => {
                eprintln!("zelyra web: auth administration query failed: {error}");
                Response::html(500, "<h1>500 Internal Server Error</h1>")
            }
        },
        "POST" => dispatch_auth_admin_post(
            auth,
            request,
            database_url,
            session_from_request(app, request, Some(database_url))
                .and_then(|session| session.user_id),
        ),
        _ => Response::html(405, "<h1>405 Method Not Allowed</h1>"),
    }
}

struct AuthAdminData {
    users: Vec<Vec<String>>,
    assignments: Vec<Vec<String>>,
    permissions: Vec<Vec<String>>,
    audit: Vec<Vec<String>>,
}

fn execute_auth_admin_mutation(
    auth: &AuthRoute,
    database_url: &str,
    mut queries: Vec<zelyra_database::Query>,
    actor_user_id: Option<i64>,
    action: &str,
    target_user_id: Option<i64>,
    details: &str,
) -> Result<zelyra_database::QueryResult, zelyra_database::DatabaseError> {
    if let Some(audit_table) = auth.audit_table.as_deref() {
        let details = details.chars().take(1000).collect::<String>();
        queries.push(zelyra_database::Query {
            sql: format!(
                "INSERT INTO {} (actor_user_id, event, target_user_id, details) VALUES (:actor_user_id, :event, :target_user_id, :details)",
                quote_identifier(audit_table)
            ),
            params: vec![
                (
                    "actor_user_id".into(),
                    actor_user_id
                        .map_or(zelyra_database::QueryValue::Null, zelyra_database::QueryValue::Int),
                ),
                (
                    "event".into(),
                    zelyra_database::QueryValue::String(action.into()),
                ),
                (
                    "target_user_id".into(),
                    target_user_id
                        .map_or(zelyra_database::QueryValue::Null, zelyra_database::QueryValue::Int),
                ),
                ("details".into(), zelyra_database::QueryValue::String(details)),
            ],
        });
    }
    zelyra_database::execute_mariadb_queries(database_url, &queries, true)
        .map(|_| zelyra_database::QueryResult::default())
}

fn load_auth_admin_data(auth: &AuthRoute, database_url: &str) -> Result<AuthAdminData, String> {
    let Some(roles_table) = auth.roles_table.as_deref() else {
        return Err("role administration has no roles table".into());
    };
    let Some(role_permissions_table) = auth.role_permissions_table.as_deref() else {
        return Err("role administration has no role permissions table".into());
    };
    let Some(user_table) = auth
        .schema
        .tables
        .iter()
        .find(|table| table.name == auth.table)
    else {
        return Err("role administration has no user table".into());
    };
    let active_column = if user_table
        .columns
        .iter()
        .any(|column| column.name == "active")
    {
        "active"
    } else {
        "true AS active"
    };
    let users = zelyra_database::execute_mariadb_query(
        database_url,
        &format!(
            "SELECT id, email, {} FROM {} ORDER BY email",
            active_column,
            quote_identifier(&auth.table),
        ),
        Vec::new(),
    )
    .map_err(|error| error.to_string())?
    .rows;
    let assignments = zelyra_database::execute_mariadb_query(
        database_url,
        &format!(
            "SELECT ur.user_id, u.email, ur.role FROM {} AS ur INNER JOIN {} AS u ON u.id = ur.user_id ORDER BY u.email, ur.role",
            quote_identifier(roles_table),
            quote_identifier(&auth.table),
        ),
        Vec::new(),
    )
    .map_err(|error| error.to_string())?
    .rows;
    let permissions = zelyra_database::execute_mariadb_query(
        database_url,
        &format!(
            "SELECT role, permission FROM {} ORDER BY role, permission",
            quote_identifier(role_permissions_table)
        ),
        Vec::new(),
    )
    .map_err(|error| error.to_string())?
    .rows;
    let audit = if let Some(audit_table) = auth.audit_table.as_deref() {
        zelyra_database::execute_mariadb_query(
            database_url,
            &format!(
                "SELECT actor_user_id, event, target_user_id, details, created_at FROM {} ORDER BY created_at DESC, id DESC LIMIT 100",
                quote_identifier(audit_table)
            ),
            Vec::new(),
        )
        .map_err(|error| error.to_string())?
        .rows
    } else {
        Vec::new()
    };
    Ok(AuthAdminData {
        users,
        assignments,
        permissions,
        audit,
    })
}

fn dispatch_auth_admin_post(
    auth: &AuthRoute,
    request: &Request,
    database_url: &str,
    actor_user_id: Option<i64>,
) -> Response {
    let input = match parse_urlencoded(&request.body) {
        Ok(input) => input,
        Err(error) => {
            return Response::html(400, format!("<h1>400 Bad Request</h1><p>{error}</p>"))
        }
    };
    if !auth
        .csrf
        .verify(input.get("_zelyra_csrf").map(String::as_str))
    {
        return Response::html(403, "<h1>403 Forbidden</h1><p>Invalid CSRF token.</p>");
    }
    let operation = input
        .get("operation")
        .map(String::as_str)
        .unwrap_or_default();
    let role = input.get("role").cloned().unwrap_or_default();
    let permission = input.get("permission").cloned().unwrap_or_default();
    let user_id = input
        .get("user_id")
        .and_then(|value| value.parse::<i64>().ok());
    let Some(roles_table) = auth.roles_table.as_deref() else {
        return Response::html(500, "<h1>500 Internal Server Error</h1>");
    };
    let Some(role_permissions_table) = auth.role_permissions_table.as_deref() else {
        return Response::html(500, "<h1>500 Internal Server Error</h1>");
    };
    let active_supported = auth
        .schema
        .tables
        .iter()
        .find(|table| table.name == auth.table)
        .is_some_and(|table| table.columns.iter().any(|column| column.name == "active"));
    let result = match operation {
        "create_user" => {
            let email = input
                .get("email")
                .map(|value| value.trim())
                .unwrap_or_default();
            let password = input
                .get("password")
                .map(String::as_str)
                .unwrap_or_default();
            if !email.contains('@') || email.starts_with('@') || email.ends_with('@') {
                return Response::html(
                    422,
                    "<h1>422 Unprocessable Entity</h1><p>A valid email address is required.</p>",
                );
            }
            if password.chars().count() < 8 {
                return Response::html(
                    422,
                    "<h1>422 Unprocessable Entity</h1><p>Password must contain at least 8 characters.</p>",
                );
            }
            let password_hash = match hash_password(password) {
                Ok(hash) => hash,
                Err(error) => {
                    eprintln!("zelyra web: user password hashing failed: {error}");
                    return Response::html(500, "<h1>500 Internal Server Error</h1>");
                }
            };
            let (sql, params) = if active_supported {
                (
                    format!(
                        "INSERT INTO {} (email, password_hash, active) VALUES (:email, :password_hash, true)",
                        quote_identifier(&auth.table)
                    ),
                    vec![
                        ("email".into(), zelyra_database::QueryValue::String(email.into())),
                        (
                            "password_hash".into(),
                            zelyra_database::QueryValue::String(password_hash),
                        ),
                    ],
                )
            } else {
                (
                    format!(
                        "INSERT INTO {} (email, password_hash) VALUES (:email, :password_hash)",
                        quote_identifier(&auth.table)
                    ),
                    vec![
                        (
                            "email".into(),
                            zelyra_database::QueryValue::String(email.into()),
                        ),
                        (
                            "password_hash".into(),
                            zelyra_database::QueryValue::String(password_hash),
                        ),
                    ],
                )
            };
            execute_auth_admin_mutation(
                auth,
                database_url,
                vec![zelyra_database::Query { sql, params }],
                actor_user_id,
                "user.create",
                None,
                &format!("email={email}"),
            )
        }
        "reset_password" => {
            let Some(user_id) = user_id else {
                return Response::html(
                    422,
                    "<h1>422 Unprocessable Entity</h1><p>A valid user ID is required.</p>",
                );
            };
            let password = input
                .get("password")
                .map(String::as_str)
                .unwrap_or_default();
            if password.chars().count() < 8 {
                return Response::html(
                    422,
                    "<h1>422 Unprocessable Entity</h1><p>Password must contain at least 8 characters.</p>",
                );
            }
            let password_hash = match hash_password(password) {
                Ok(hash) => hash,
                Err(error) => {
                    eprintln!("zelyra web: user password hashing failed: {error}");
                    return Response::html(500, "<h1>500 Internal Server Error</h1>");
                }
            };
            let update = zelyra_database::Query {
                sql: format!(
                    "UPDATE {} SET password_hash = :password_hash WHERE id = :user_id",
                    quote_identifier(&auth.table)
                ),
                params: vec![
                    (
                        "password_hash".into(),
                        zelyra_database::QueryValue::String(password_hash),
                    ),
                    ("user_id".into(), zelyra_database::QueryValue::Int(user_id)),
                ],
            };
            let mut queries = vec![update];
            if let Some(session_table) = auth.session_table.as_deref() {
                queries.push(zelyra_database::Query {
                    sql: format!(
                        "DELETE FROM {} WHERE user_id = :user_id",
                        quote_identifier(session_table)
                    ),
                    params: vec![("user_id".into(), zelyra_database::QueryValue::Int(user_id))],
                });
            }
            execute_auth_admin_mutation(
                auth,
                database_url,
                queries,
                actor_user_id,
                "user.password_reset",
                Some(user_id),
                "",
            )
        }
        "activate_user" | "deactivate_user" => {
            let Some(user_id) = user_id else {
                return Response::html(
                    422,
                    "<h1>422 Unprocessable Entity</h1><p>A valid user ID is required.</p>",
                );
            };
            if !active_supported {
                return Response::html(
                    409,
                    "<h1>409 Conflict</h1><p>User activation requires an active column.</p>",
                );
            }
            let activating = operation == "activate_user";
            if !activating {
                if let (Some(admin_role), Some(roles_table)) =
                    (auth.admin_role.as_deref(), auth.roles_table.as_deref())
                {
                    let target_query = format!(
                        "SELECT COUNT(*) FROM {} WHERE user_id = :user_id AND role = :role",
                        quote_identifier(roles_table)
                    );
                    let target_is_admin = match zelyra_database::execute_mariadb_query(
                        database_url,
                        &target_query,
                        vec![
                            ("user_id".into(), zelyra_database::QueryValue::Int(user_id)),
                            (
                                "role".into(),
                                zelyra_database::QueryValue::String(admin_role.into()),
                            ),
                        ],
                    ) {
                        Ok(result) => result
                            .rows
                            .first()
                            .and_then(|row| row.first())
                            .and_then(|value| value.parse::<u64>().ok())
                            .is_some_and(|count| count > 0),
                        Err(error) => {
                            eprintln!("zelyra web: admin role lookup failed: {error}");
                            return Response::html(500, "<h1>500 Internal Server Error</h1>");
                        }
                    };
                    if target_is_admin {
                        let active_admin_query = format!(
                            "SELECT COUNT(*) FROM {} AS ur INNER JOIN {} AS u ON u.id = ur.user_id WHERE ur.role = :role AND u.active = true",
                            quote_identifier(roles_table),
                            quote_identifier(&auth.table),
                        );
                        let active_admins = match zelyra_database::execute_mariadb_query(
                            database_url,
                            &active_admin_query,
                            vec![(
                                "role".into(),
                                zelyra_database::QueryValue::String(admin_role.into()),
                            )],
                        ) {
                            Ok(result) => result
                                .rows
                                .first()
                                .and_then(|row| row.first())
                                .and_then(|value| value.parse::<u64>().ok())
                                .unwrap_or(0),
                            Err(error) => {
                                eprintln!("zelyra web: active admin count failed: {error}");
                                return Response::html(500, "<h1>500 Internal Server Error</h1>");
                            }
                        };
                        if active_admins <= 1 {
                            return Response::html(
                                409,
                                "<h1>409 Conflict</h1><p>The last active administrator cannot be deactivated.</p>",
                            );
                        }
                    }
                }
            }
            let value = if activating { "true" } else { "false" };
            let update = zelyra_database::Query {
                sql: format!(
                    "UPDATE {} SET active = {} WHERE id = :user_id",
                    quote_identifier(&auth.table),
                    value
                ),
                params: vec![("user_id".into(), zelyra_database::QueryValue::Int(user_id))],
            };
            let mut queries = vec![update];
            if !activating {
                if let Some(session_table) = auth.session_table.as_deref() {
                    queries.push(zelyra_database::Query {
                        sql: format!(
                            "DELETE FROM {} WHERE user_id = :user_id",
                            quote_identifier(session_table)
                        ),
                        params: vec![("user_id".into(), zelyra_database::QueryValue::Int(user_id))],
                    });
                }
            }
            execute_auth_admin_mutation(
                auth,
                database_url,
                queries,
                actor_user_id,
                if activating {
                    "user.activate"
                } else {
                    "user.deactivate"
                },
                Some(user_id),
                "",
            )
        }
        "grant_role" => {
            let Some(user_id) = user_id else {
                return Response::html(
                    422,
                    "<h1>422 Unprocessable Entity</h1><p>A valid user ID is required.</p>",
                );
            };
            if role.is_empty() {
                return Response::html(
                    422,
                    "<h1>422 Unprocessable Entity</h1><p>A role is required.</p>",
                );
            }
            let sql = format!(
                "INSERT INTO {} (user_id, role) SELECT :user_id, :role FROM DUAL WHERE NOT EXISTS (SELECT 1 FROM {} WHERE user_id = :user_id AND role = :role)",
                quote_identifier(roles_table),
                quote_identifier(roles_table),
            );
            execute_auth_admin_mutation(
                auth,
                database_url,
                vec![zelyra_database::Query {
                    sql,
                    params: vec![
                        ("user_id".into(), zelyra_database::QueryValue::Int(user_id)),
                        (
                            "role".into(),
                            zelyra_database::QueryValue::String(role.clone()),
                        ),
                    ],
                }],
                actor_user_id,
                "role.grant",
                Some(user_id),
                &format!("role={role}"),
            )
        }
        "revoke_role" => {
            let Some(user_id) = user_id else {
                return Response::html(
                    422,
                    "<h1>422 Unprocessable Entity</h1><p>A valid user ID is required.</p>",
                );
            };
            if role.is_empty() {
                return Response::html(
                    422,
                    "<h1>422 Unprocessable Entity</h1><p>A role is required.</p>",
                );
            }
            if auth.admin_role.as_deref() == Some(role.as_str()) {
                let count_query = format!(
                    "SELECT COUNT(*) FROM {} WHERE role = :role",
                    quote_identifier(roles_table)
                );
                let count = match zelyra_database::execute_mariadb_query(
                    database_url,
                    &count_query,
                    vec![(
                        "role".into(),
                        zelyra_database::QueryValue::String(role.clone()),
                    )],
                ) {
                    Ok(result) => result
                        .rows
                        .first()
                        .and_then(|row| row.first())
                        .and_then(|value| value.parse::<u64>().ok())
                        .unwrap_or(0),
                    Err(error) => {
                        eprintln!("zelyra web: admin role count failed: {error}");
                        return Response::html(500, "<h1>500 Internal Server Error</h1>");
                    }
                };
                if count <= 1 {
                    return Response::html(
                        409,
                        "<h1>409 Conflict</h1><p>The last administrator role assignment cannot be removed.</p>",
                    );
                }
            }
            let sql = format!(
                "DELETE FROM {} WHERE user_id = :user_id AND role = :role",
                quote_identifier(roles_table)
            );
            execute_auth_admin_mutation(
                auth,
                database_url,
                vec![zelyra_database::Query {
                    sql,
                    params: vec![
                        ("user_id".into(), zelyra_database::QueryValue::Int(user_id)),
                        (
                            "role".into(),
                            zelyra_database::QueryValue::String(role.clone()),
                        ),
                    ],
                }],
                actor_user_id,
                "role.revoke",
                Some(user_id),
                &format!("role={role}"),
            )
        }
        "grant_permission" | "revoke_permission" => {
            if role.is_empty() || permission.is_empty() {
                return Response::html(
                    422,
                    "<h1>422 Unprocessable Entity</h1><p>Role and permission are required.</p>",
                );
            }
            if operation == "grant_permission" {
                let sql = format!(
                    "INSERT INTO {} (role, permission) SELECT :role, :permission FROM DUAL WHERE NOT EXISTS (SELECT 1 FROM {} WHERE role = :role AND permission = :permission)",
                    quote_identifier(role_permissions_table),
                    quote_identifier(role_permissions_table),
                );
                execute_auth_admin_mutation(
                    auth,
                    database_url,
                    vec![zelyra_database::Query {
                        sql,
                        params: vec![
                            (
                                "role".into(),
                                zelyra_database::QueryValue::String(role.clone()),
                            ),
                            (
                                "permission".into(),
                                zelyra_database::QueryValue::String(permission.clone()),
                            ),
                        ],
                    }],
                    actor_user_id,
                    "role_permission.grant",
                    None,
                    &format!("role={role};permission={permission}"),
                )
            } else {
                let sql = format!(
                    "DELETE FROM {} WHERE role = :role AND permission = :permission",
                    quote_identifier(role_permissions_table)
                );
                execute_auth_admin_mutation(
                    auth,
                    database_url,
                    vec![zelyra_database::Query {
                        sql,
                        params: vec![
                            (
                                "role".into(),
                                zelyra_database::QueryValue::String(role.clone()),
                            ),
                            (
                                "permission".into(),
                                zelyra_database::QueryValue::String(permission.clone()),
                            ),
                        ],
                    }],
                    actor_user_id,
                    "role_permission.revoke",
                    None,
                    &format!("role={role};permission={permission}"),
                )
            }
        }
        _ => {
            return Response::html(
                400,
                "<h1>400 Bad Request</h1><p>Unknown administration operation.</p>",
            )
        }
    };
    if let Err(error) = result {
        eprintln!("zelyra web: auth administration write failed: {error}");
        return Response::html(500, "<h1>500 Internal Server Error</h1>");
    }
    Response::redirect(auth.admin_path.as_deref().unwrap_or("/"))
}

fn render_auth_admin(
    auth: &AuthRoute,
    users: &[Vec<String>],
    assignments: &[Vec<String>],
    permissions: &[Vec<String>],
    audit: &[Vec<String>],
) -> String {
    let path = html_escape(auth.admin_path.as_deref().unwrap_or("/"));
    let csrf = html_escape(auth.csrf.token());
    let active_supported = auth
        .schema
        .tables
        .iter()
        .find(|table| table.name == auth.table)
        .is_some_and(|table| table.columns.iter().any(|column| column.name == "active"));
    let mut html = format!(
        "<main><h1>Role administration</h1><h2>User administration</h2><form method=\"post\" action=\"{path}\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{csrf}\"><input type=\"hidden\" name=\"operation\" value=\"create_user\"><label>Email</label><input name=\"email\" type=\"email\" required><label>Initial password</label><input name=\"password\" type=\"password\" minlength=\"8\" required><button type=\"submit\">Create user</button></form><table><tr><th>Email</th><th>Status</th><th>Actions</th></tr>"
    );
    for row in users {
        if let (Some(user_id), Some(email), Some(active)) = (row.first(), row.get(1), row.get(2)) {
            let is_active = matches!(active.as_str(), "1" | "true" | "TRUE");
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td><form method=\"post\" action=\"{}\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{}\"><input type=\"hidden\" name=\"operation\" value=\"reset_password\"><input type=\"hidden\" name=\"user_id\" value=\"{}\"><input name=\"password\" type=\"password\" minlength=\"8\" required placeholder=\"New password\"><button type=\"submit\">Reset password</button></form>{}</td></tr>",
                html_escape(email),
                if is_active { "Active" } else { "Inactive" },
                path,
                csrf,
                html_escape(user_id),
                if active_supported {
                    format!(
                        "<form method=\"post\" action=\"{}\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{}\"><input type=\"hidden\" name=\"operation\" value=\"{}\"><input type=\"hidden\" name=\"user_id\" value=\"{}\"><button type=\"submit\">{}</button></form>",
                        path,
                        csrf,
                        if is_active { "deactivate_user" } else { "activate_user" },
                        html_escape(user_id),
                        if is_active { "Deactivate" } else { "Activate" },
                    )
                } else {
                    String::new()
                },
            ));
        }
    }
    html.push_str("</table><h2>Assign role</h2><form method=\"post\" action=\"");
    html.push_str(&path);
    html.push_str("\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"");
    html.push_str(&csrf);
    html.push_str("\"><input type=\"hidden\" name=\"operation\" value=\"grant_role\"><label>User ID</label><input name=\"user_id\" type=\"number\" required><label>Role</label><input name=\"role\" required><button type=\"submit\">Grant role</button></form>");
    html.push_str(
        "<h2>Role assignments</h2><table><tr><th>User</th><th>Role</th><th>Action</th></tr>",
    );
    for row in assignments {
        if let (Some(user_id), Some(email), Some(role)) = (row.first(), row.get(1), row.get(2)) {
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td><form method=\"post\" action=\"{}\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{}\"><input type=\"hidden\" name=\"operation\" value=\"revoke_role\"><input type=\"hidden\" name=\"user_id\" value=\"{}\"><input type=\"hidden\" name=\"role\" value=\"{}\"><button type=\"submit\">Revoke</button></form></td></tr>",
                html_escape(email),
                html_escape(role),
                path,
                csrf,
                html_escape(user_id),
                html_escape(role),
            ));
        }
    }
    html.push_str("</table><h2>Role permissions</h2><form method=\"post\" action=\"");
    html.push_str(&path);
    html.push_str("\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"");
    html.push_str(&csrf);
    html.push_str("\"><input type=\"hidden\" name=\"operation\" value=\"grant_permission\"><label>Role</label><input name=\"role\" required><label>Permission</label><input name=\"permission\" required><button type=\"submit\">Grant permission</button></form><table><tr><th>Role</th><th>Permission</th><th>Action</th></tr>");
    for row in permissions {
        if let (Some(role), Some(permission)) = (row.first(), row.get(1)) {
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td><form method=\"post\" action=\"{}\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{}\"><input type=\"hidden\" name=\"operation\" value=\"revoke_permission\"><input type=\"hidden\" name=\"role\" value=\"{}\"><input type=\"hidden\" name=\"permission\" value=\"{}\"><button type=\"submit\">Revoke</button></form></td></tr>",
                html_escape(role),
                html_escape(permission),
                path,
                csrf,
                html_escape(role),
                html_escape(permission),
            ));
        }
    }
    if auth.audit_table.is_some() {
        html.push_str(
            "</table><h2>Audit log</h2><p>Latest 100 administrative changes.</p><table><tr><th>Actor</th><th>Action</th><th>Target</th><th>Details</th><th>Created</th></tr>",
        );
        for row in audit {
            let cells = row
                .iter()
                .map(|value| format!("<td>{}</td>", html_escape(value)))
                .collect::<String>();
            html.push_str(&format!("<tr>{cells}</tr>"));
        }
        html.push_str("</table>");
    } else {
        html.push_str("</table>");
    }
    html.push_str("</main>");
    html
}

fn cookie_value(request: &Request, name: &str) -> Option<String> {
    request
        .headers
        .get("cookie")?
        .split(';')
        .map(str::trim)
        .find_map(|cookie| {
            let (cookie_name, value) = cookie.split_once('=')?;
            (cookie_name == name).then(|| value.to_owned())
        })
}

fn load_user_permissions(
    auth: &AuthRoute,
    database_url: &str,
    user_id: i64,
    fallback: &[String],
) -> Result<Vec<String>, String> {
    let has_database_permissions = auth.permissions_table.is_some() || auth.roles_table.is_some();
    if !has_database_permissions {
        return Ok(fallback.to_vec());
    }
    let mut permissions = Vec::new();
    if let Some(permissions_table) = &auth.permissions_table {
        let query = format!(
            "SELECT permission FROM {} WHERE user_id = :user_id",
            quote_identifier(permissions_table)
        );
        let result = zelyra_database::execute_mariadb_query(
            database_url,
            &query,
            vec![("user_id".into(), zelyra_database::QueryValue::Int(user_id))],
        )
        .map_err(|error| error.to_string())?;
        permissions.extend(
            result
                .rows
                .into_iter()
                .filter_map(|row| row.into_iter().next()),
        );
    }
    if let (Some(roles_table), Some(role_permissions_table)) =
        (&auth.roles_table, &auth.role_permissions_table)
    {
        let query = format!(
            "SELECT rp.permission FROM {} AS ur INNER JOIN {} AS rp ON rp.role = ur.role WHERE ur.user_id = :user_id",
            quote_identifier(roles_table),
            quote_identifier(role_permissions_table),
        );
        let result = zelyra_database::execute_mariadb_query(
            database_url,
            &query,
            vec![("user_id".into(), zelyra_database::QueryValue::Int(user_id))],
        )
        .map_err(|error| error.to_string())?;
        permissions.extend(
            result
                .rows
                .into_iter()
                .filter_map(|row| row.into_iter().next()),
        );
    }
    Ok(permissions)
}

fn session_from_request(
    app: &WebApp,
    request: &Request,
    database_url: Option<&str>,
) -> Option<Session> {
    let session_id = cookie_value(request, "zelyra_session")?;
    let Some(auth) = &app.auth_route else {
        return app.sessions.lock().ok()?.get(&session_id).cloned();
    };
    let (Some(session_table), Some(database_url)) = (&auth.session_table, database_url) else {
        return app.sessions.lock().ok()?.get(&session_id).cloned();
    };
    if app.database_capability_granted == Some(false) {
        return None;
    }
    let active_clause = auth
        .schema
        .tables
        .iter()
        .find(|table| table.name == auth.table)
        .is_some_and(|table| table.columns.iter().any(|column| column.name == "active"));
    let query = if active_clause {
        format!(
            "SELECT s.user_id FROM {} AS s INNER JOIN {} AS u ON u.id = s.user_id WHERE s.token_hash = :token_hash AND s.expires_at > CURRENT_TIMESTAMP AND u.active = true LIMIT 1",
            quote_identifier(session_table),
            quote_identifier(&auth.table),
        )
    } else {
        format!(
            "SELECT user_id FROM {} WHERE token_hash = :token_hash AND expires_at > CURRENT_TIMESTAMP LIMIT 1",
            quote_identifier(session_table)
        )
    };
    let result = match zelyra_database::execute_mariadb_query(
        database_url,
        &query,
        vec![(
            "token_hash".into(),
            zelyra_database::QueryValue::String(session_token_hash(&session_id)),
        )],
    ) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("zelyra web: session lookup failed: {error}");
            return None;
        }
    };
    let user_id = result
        .rows
        .first()
        .and_then(|row| row.first())
        .and_then(|value| value.parse::<i64>().ok())?;
    let permissions =
        match load_user_permissions(auth, database_url, user_id, &app.auth_permissions) {
            Ok(permissions) => permissions,
            Err(error) => {
                eprintln!("zelyra web: permission lookup failed: {error}");
                return None;
            }
        };
    Some(Session {
        user_id: Some(user_id),
        permissions,
    })
}

fn database_capability_denied() -> Response {
    Response::html(
        403,
        "<h1>403 Forbidden</h1><p>The Database capability is not granted.</p>",
    )
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
    let mut headers: HashMap<String, String> = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        let (name, value) = line.split_once(':').ok_or_else(|| HttpError {
            message: "malformed HTTP header".into(),
        })?;
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().into());
    }
    if let Some(content_length) = headers.get("content-length") {
        let content_length = content_length.parse::<usize>().map_err(|_| HttpError {
            message: "content-length must be a non-negative integer".into(),
        })?;
        if content_length > MAX_REQUEST_BODY_BYTES {
            return Err(HttpError {
                message: format!(
                    "request body exceeds the {} byte limit",
                    MAX_REQUEST_BODY_BYTES
                ),
            });
        }
        if content_length != body.len() {
            return Err(HttpError {
                message: format!(
                    "content-length declares {content_length} bytes, received {}",
                    body.len()
                ),
            });
        }
    }
    if body.len() > MAX_REQUEST_BODY_BYTES {
        return Err(HttpError {
            message: format!(
                "request body exceeds the {} byte limit",
                MAX_REQUEST_BODY_BYTES
            ),
        });
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

pub const MAX_REQUEST_BODY_BYTES: usize = 1_048_576;
const MAX_REQUEST_HEADER_BYTES: usize = 64 * 1024;

#[derive(Debug)]
enum RequestReadError {
    Io(io::Error),
    Http(HttpError),
    PayloadTooLarge,
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn declared_content_length(header_text: &str) -> Result<Option<usize>, HttpError> {
    let mut content_length = None;
    for line in header_text.split("\r\n").skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            if !line.is_empty() {
                return Err(HttpError {
                    message: "malformed HTTP header".into(),
                });
            }
            continue;
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            let parsed = value.trim().parse::<usize>().map_err(|_| HttpError {
                message: "content-length must be a non-negative integer".into(),
            })?;
            if let Some(previous) = content_length {
                if previous != parsed {
                    return Err(HttpError {
                        message: "conflicting content-length headers".into(),
                    });
                }
            }
            content_length = Some(parsed);
        }
    }
    Ok(content_length)
}

fn read_http_request_from<R: Read>(reader: &mut R) -> Result<String, RequestReadError> {
    let mut buffer = Vec::new();
    let total_length = loop {
        let mut chunk = [0_u8; 8192];
        let size = reader.read(&mut chunk).map_err(RequestReadError::Io)?;
        if size == 0 {
            break None;
        }
        buffer.extend_from_slice(&chunk[..size]);
        if buffer.len() > MAX_REQUEST_HEADER_BYTES && find_header_end(&buffer).is_none() {
            return Err(RequestReadError::Http(HttpError {
                message: "HTTP headers exceed the configured limit".into(),
            }));
        }
        let Some(header_end) = find_header_end(&buffer) else {
            continue;
        };
        if header_end > MAX_REQUEST_HEADER_BYTES {
            return Err(RequestReadError::Http(HttpError {
                message: "HTTP headers exceed the configured limit".into(),
            }));
        }
        let header_text = std::str::from_utf8(&buffer[..header_end]).map_err(|error| {
            RequestReadError::Http(HttpError {
                message: error.to_string(),
            })
        })?;
        let content_length = declared_content_length(header_text)
            .map_err(RequestReadError::Http)?
            .unwrap_or(0);
        if content_length > MAX_REQUEST_BODY_BYTES {
            return Err(RequestReadError::PayloadTooLarge);
        }
        let total_length = header_end + 4 + content_length;
        if buffer.len() >= total_length {
            break Some(total_length);
        }
        while buffer.len() < total_length {
            let size = reader.read(&mut chunk).map_err(RequestReadError::Io)?;
            if size == 0 {
                break;
            }
            buffer.extend_from_slice(&chunk[..size]);
        }
        break Some(total_length);
    };
    if let Some(total_length) = total_length {
        buffer.truncate(total_length);
    }
    String::from_utf8(buffer).map_err(|error| {
        RequestReadError::Http(HttpError {
            message: error.to_string(),
        })
    })
}

fn read_http_request(stream: &mut TcpStream) -> Result<String, RequestReadError> {
    read_http_request_from(stream)
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

fn form_authorization(form: &FormRoute) -> (bool, Vec<String>) {
    let mut permissions = form.permissions.clone();
    let (action_requires_auth, action_permissions) = form
        .form
        .actions
        .first()
        .map(|action| (action.requires_auth, action.permissions.as_slice()))
        .unwrap_or((false, &[]));
    permissions.extend(action_permissions.iter().cloned());
    (form.requires_auth || action_requires_auth, permissions)
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

fn crud_foreign_key<'a>(
    table: &'a zelyra_database::Table,
    column: &str,
) -> Option<&'a zelyra_database::ForeignKey> {
    table.foreign_keys.iter().find(|foreign_key| {
        foreign_key.column == column
            || foreign_key.column == format!("{column}_id")
            || column
                == foreign_key
                    .column
                    .strip_suffix("_id")
                    .unwrap_or(&foreign_key.column)
    })
}

fn crud_relation_alias(table: &zelyra_database::Table, column: &str) -> Option<String> {
    table
        .foreign_keys
        .iter()
        .position(|foreign_key| foreign_key.column == column)
        .map(|index| format!("zelyra_relation_{index}"))
}

fn crud_column_expression(schema: &Schema, table: &zelyra_database::Table, column: &str) -> String {
    let Some(foreign_key) = crud_foreign_key(table, column) else {
        return format!("{}.{}", quote_identifier("base"), quote_identifier(column));
    };
    let Some(alias) = crud_relation_alias(table, &foreign_key.column) else {
        return format!("{}.{}", quote_identifier("base"), quote_identifier(column));
    };
    let Some(relation_table) = schema
        .tables
        .iter()
        .find(|candidate| candidate.name == foreign_key.referenced_table)
    else {
        return format!("{}.{}", quote_identifier("base"), quote_identifier(column));
    };
    format!(
        "{}.{}",
        quote_identifier(&alias),
        quote_identifier(relation_display_column(relation_table))
    )
}

fn crud_storage_expression(table: &zelyra_database::Table, column: &str) -> String {
    let storage_column = crud_foreign_key(table, column)
        .map(|foreign_key| foreign_key.column.as_str())
        .unwrap_or(column);
    format!(
        "{}.{}",
        quote_identifier("base"),
        quote_identifier(storage_column)
    )
}

fn crud_relation_joins(schema: &Schema, table: &zelyra_database::Table) -> String {
    table
        .foreign_keys
        .iter()
        .enumerate()
        .filter_map(|(index, foreign_key)| {
            let relation_table = schema
                .tables
                .iter()
                .find(|candidate| candidate.name == foreign_key.referenced_table)?;
            let alias = format!("zelyra_relation_{index}");
            Some(format!(
                " LEFT JOIN {} AS {} ON {}.{} = {}.{}",
                quote_identifier(&relation_table.name),
                quote_identifier(&alias),
                quote_identifier("base"),
                quote_identifier(&foreign_key.column),
                quote_identifier(&alias),
                quote_identifier(&foreign_key.referenced_column),
            ))
        })
        .collect()
}

fn crud_column_label(schema: &Schema, table_name: &str, column: &str) -> String {
    let Some(table) = schema.tables.iter().find(|table| table.name == table_name) else {
        return humanize(column);
    };
    if crud_foreign_key(table, column).is_some() {
        return humanize(column.strip_suffix("_id").unwrap_or(column));
    }
    humanize(column)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CrudUiActions {
    create: bool,
    edit: bool,
    delete: bool,
}

fn crud_ui_actions(crud: &CrudRoute, request: &Request, app: &WebApp) -> CrudUiActions {
    CrudUiActions {
        create: can_authorize(crud.requires_auth, &crud.create_permissions, request, app),
        edit: can_authorize(crud.requires_auth, &crud.edit_permissions, request, app),
        delete: can_authorize(crud.requires_auth, &crud.delete_permissions, request, app),
    }
}

fn can_authorize(
    requires_auth: bool,
    permissions: &[String],
    request: &Request,
    app: &WebApp,
) -> bool {
    if !requires_auth && permissions.is_empty() {
        return true;
    }
    let Some(granted_permissions) =
        authenticated_permissions(app, request, app.database_url.as_deref())
    else {
        return false;
    };
    permissions.iter().all(|permission| {
        granted_permissions
            .iter()
            .any(|granted| granted == permission)
    })
}

fn dispatch_crud(
    crud: &CrudRoute,
    request: &Request,
    database_url: Option<&str>,
    ui_actions: CrudUiActions,
) -> Response {
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
    let all_columns = table
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    if all_columns.is_empty() {
        return Response::html(500, "<h1>500 Internal Server Error</h1>");
    }
    let display_columns = crud
        .list_columns
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let display_columns = if display_columns.is_empty() {
        all_columns.clone()
    } else {
        display_columns
    };
    let mut query_columns = Vec::new();
    if all_columns.contains(&"id") {
        query_columns.push("id");
    }
    for column in &display_columns {
        if !query_columns.contains(column) {
            query_columns.push(column);
        }
    }
    let sort_columns = all_columns.clone();
    let search_columns = crud
        .search_columns
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let search_columns = if crud.search_columns.is_empty() {
        table
            .columns
            .iter()
            .filter(|column| {
                let sql_type = column.sql_type.to_ascii_uppercase();
                sql_type.contains("CHAR") || sql_type.contains("TEXT")
            })
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>()
    } else {
        search_columns
    };
    let filter_columns = crud
        .filter_columns
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let filter_columns = if crud.filter_columns.is_empty() {
        all_columns
            .iter()
            .copied()
            .filter(|column| *column != "id")
            .collect::<Vec<_>>()
    } else {
        filter_columns
    };
    let sort_column = query_values
        .get("sort")
        .map(String::as_str)
        .unwrap_or_else(|| {
            sort_columns
                .iter()
                .copied()
                .find(|column| *column == "id")
                .unwrap_or(sort_columns[0])
        });
    if !sort_columns.contains(&sort_column) {
        return Response::html(400, "<h1>400 Bad Request</h1><p>Unknown sort column.</p>");
    }
    let order = match query_values
        .get("order")
        .map(String::as_str)
        .unwrap_or("asc")
    {
        "asc" => "ASC",
        "desc" => "DESC",
        _ => {
            return Response::html(
                400,
                "<h1>400 Bad Request</h1><p>order must be asc or desc.</p>",
            )
        }
    };
    let mut filters = Vec::new();
    let mut seen_filter_columns = HashSet::new();
    for (name, value) in &query_values {
        let Some(filter_name) = name.strip_prefix("filter_") else {
            continue;
        };
        if filter_name.ends_with("__operator") {
            let column = filter_name.trim_end_matches("__operator");
            if !filter_columns.contains(&column) {
                return Response::html(
                    400,
                    "<h1>400 Bad Request</h1><p>Unknown filter column.</p>",
                );
            }
            if FilterOperator::parse(value).is_none() {
                return Response::html(
                    400,
                    format!("<h1>400 Bad Request</h1><p>Unknown filter operator for {column}.</p>"),
                );
            }
            continue;
        }
        let (column, direct_operator) = filter_name
            .split_once("__")
            .map_or((filter_name, None), |(column, operator)| {
                (column, FilterOperator::parse(operator))
            });
        if !filter_columns.contains(&column) {
            return Response::html(400, "<h1>400 Bad Request</h1><p>Unknown filter column.</p>");
        }
        if filter_name.contains("__") && direct_operator.is_none() {
            return Response::html(
                400,
                format!("<h1>400 Bad Request</h1><p>Unknown filter operator for {column}.</p>"),
            );
        }
        let operator =
            direct_operator.unwrap_or_else(|| selected_filter_operator(&query_values, column));
        let Some(schema_column) = table.columns.iter().find(|candidate| {
            let storage_column = crud_foreign_key(table, column)
                .map(|foreign_key| foreign_key.column.as_str())
                .unwrap_or(column);
            candidate.name == storage_column
        }) else {
            return Response::html(500, "<h1>500 Internal Server Error</h1>");
        };
        if !filter_operator_supported(schema_column, operator) {
            return Response::html(
                400,
                format!(
                    "<h1>400 Bad Request</h1><p>Operator `{}` is not supported for filter `{column}`.</p>",
                    operator.key()
                ),
            );
        }
        if !operator.needs_value() || !value.is_empty() {
            if !seen_filter_columns.insert(column) {
                return Response::html(
                    400,
                    format!(
                        "<h1>400 Bad Request</h1><p>Filter `{column}` was specified more than once.</p>"
                    ),
                );
            }
            filters.push((column, value, operator));
        }
    }
    let mut query = format!(
        "SELECT {} FROM {}",
        query_columns
            .iter()
            .map(|column| {
                format!(
                    "{} AS {}",
                    crud_column_expression(&crud.schema, table, column),
                    quote_identifier(column)
                )
            })
            .collect::<Vec<_>>()
            .join(", "),
        format_args!(
            "{} AS {}{}",
            quote_identifier(&crud.table),
            quote_identifier("base"),
            crud_relation_joins(&crud.schema, table)
        )
    );
    let mut conditions = Vec::new();
    if !search.is_empty() && !search_columns.is_empty() {
        conditions.push(format!(
            "({})",
            search_columns
                .iter()
                .map(|column| {
                    format!(
                        "{} LIKE CONCAT('%', :search, '%')",
                        crud_column_expression(&crud.schema, table, column)
                    )
                })
                .collect::<Vec<_>>()
                .join(" OR ")
        ));
    }
    for (column, _, operator) in &filters {
        conditions.push(filter_condition(table, column, *operator));
    }
    if !conditions.is_empty() {
        query.push_str(" WHERE ");
        query.push_str(&conditions.join(" AND "));
    }
    query.push_str(&format!(
        " ORDER BY {} {} LIMIT :limit OFFSET :offset",
        crud_column_expression(&crud.schema, table, sort_column),
        order
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
    if !search.is_empty() && !search_columns.is_empty() {
        params.push((
            "search".into(),
            zelyra_database::QueryValue::String(search.clone()),
        ));
    }
    for (column, value, operator) in filters {
        if !operator.needs_value() {
            continue;
        }
        let storage_column = crud_foreign_key(table, column)
            .map(|foreign_key| foreign_key.column.as_str())
            .unwrap_or(column);
        let schema_column = table
            .columns
            .iter()
            .find(|candidate| candidate.name == storage_column)
            .expect("filter column was validated against the schema");
        let value = match filter_query_value(schema_column, value) {
            Ok(value) => value,
            Err(message) => {
                return Response::html(400, format!("<h1>400 Bad Request</h1><p>{message}</p>"))
            }
        };
        params.push((format!("filter_{column}"), value));
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
        render_crud_list_with_actions(
            crud,
            CrudListView {
                query_columns: &query_columns,
                display_columns: &display_columns,
                filter_columns: &filter_columns,
                sort_columns: &sort_columns,
                rows: &result.rows,
                search: &search,
                query_values: &query_values,
                sort: sort_column,
                order,
                page,
                per_page,
            },
            ui_actions,
        ),
    )
}

fn dispatch_tableview(
    tableview: &TableViewRoute,
    request: &Request,
    database_url: Option<&str>,
) -> Response {
    if request.method != "GET" {
        return Response::html(405, "<h1>405 Method Not Allowed</h1>");
    }
    let Some(database_url) = database_url else {
        return Response::html(
            503,
            "<h1>503 Service Unavailable</h1><p>DATABASE_URL is required for table views.</p>",
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
    let per_page = u64::from(tableview.page_size.unwrap_or(50));
    let offset = (page.saturating_sub(1)).saturating_mul(per_page);
    let Some(default_sort) = tableview.columns.first().map(String::as_str) else {
        return Response::html(500, "<h1>500 Internal Server Error</h1>");
    };
    let sort = query_values
        .get("sort")
        .map(String::as_str)
        .unwrap_or(default_sort);
    if !tableview.sortable && query_values.contains_key("sort") {
        return Response::html(
            400,
            "<h1>400 Bad Request</h1><p>This table view is not sortable.</p>",
        );
    }
    if !tableview.columns.iter().any(|column| column == sort) {
        return Response::html(400, "<h1>400 Bad Request</h1><p>Unknown sort column.</p>");
    }
    let order = match query_values
        .get("order")
        .map(String::as_str)
        .unwrap_or("asc")
    {
        "asc" => "ASC",
        "desc" => "DESC",
        _ => {
            return Response::html(
                400,
                "<h1>400 Bad Request</h1><p>order must be asc or desc.</p>",
            )
        }
    };
    if !tableview.sortable && query_values.contains_key("order") {
        return Response::html(
            400,
            "<h1>400 Bad Request</h1><p>This table view is not sortable.</p>",
        );
    }
    let mut filters = Vec::new();
    let mut seen_filter_columns = HashSet::new();
    for (name, value) in &query_values {
        let Some(filter_name) = name.strip_prefix("filter_") else {
            continue;
        };
        if filter_name.ends_with("__operator") {
            let column = filter_name.trim_end_matches("__operator");
            let Some(filter) = tableview
                .filters
                .iter()
                .find(|filter| filter.name == column)
            else {
                return Response::html(
                    400,
                    "<h1>400 Bad Request</h1><p>Unknown filter column.</p>",
                );
            };
            let Some(operator) = FilterOperator::parse(value) else {
                return Response::html(
                    400,
                    format!("<h1>400 Bad Request</h1><p>Unknown filter operator for {column}.</p>"),
                );
            };
            if !tableview_filter_operator_supported(filter.kind, operator) {
                return Response::html(
                    400,
                    format!(
                        "<h1>400 Bad Request</h1><p>Operator `{}` is not supported for filter `{column}`.</p>",
                        operator.key()
                    ),
                );
            }
            continue;
        }
        let (column, direct_operator) = filter_name
            .split_once("__")
            .map_or((filter_name, None), |(column, operator)| {
                (column, FilterOperator::parse(operator))
            });
        let Some(filter) = tableview
            .filters
            .iter()
            .find(|filter| filter.name == column)
        else {
            return Response::html(400, "<h1>400 Bad Request</h1><p>Unknown filter column.</p>");
        };
        if filter_name.contains("__") && direct_operator.is_none() {
            return Response::html(
                400,
                format!("<h1>400 Bad Request</h1><p>Unknown filter operator for {column}.</p>"),
            );
        }
        let operator =
            direct_operator.unwrap_or_else(|| selected_filter_operator(&query_values, column));
        if !tableview_filter_operator_supported(filter.kind, operator) {
            return Response::html(
                400,
                format!(
                    "<h1>400 Bad Request</h1><p>Operator `{}` is not supported for filter `{column}`.</p>",
                    operator.key()
                ),
            );
        }
        if !operator.needs_value() || !value.is_empty() {
            if !seen_filter_columns.insert(column) {
                return Response::html(
                    400,
                    format!("<h1>400 Bad Request</h1><p>Filter `{column}` was specified more than once.</p>"),
                );
            }
            filters.push((column.to_string(), value.clone(), operator));
        }
    }
    let source = tableview.source.trim().trim_end_matches(';').trim();
    if source.is_empty() {
        return Response::html(500, "<h1>500 Internal Server Error</h1>");
    }
    let select_columns = tableview
        .columns
        .iter()
        .map(|column| {
            format!(
                "{}.{} AS {}",
                quote_identifier("zelyra_view"),
                quote_identifier(column),
                quote_identifier(column)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let mut query = format!(
        "SELECT {select_columns} FROM ({source}) AS {}",
        quote_identifier("zelyra_view")
    );
    let mut has_where = false;
    if tableview.searchable && !search.is_empty() {
        let search_conditions = tableview
            .columns
            .iter()
            .map(|column| {
                format!(
                    "CAST({}.{} AS CHAR) LIKE CONCAT('%', :search, '%')",
                    quote_identifier("zelyra_view"),
                    quote_identifier(column)
                )
            })
            .collect::<Vec<_>>()
            .join(" OR ");
        query.push_str(" WHERE (");
        query.push_str(&search_conditions);
        query.push(')');
        has_where = true;
    } else if !search.is_empty() {
        return Response::html(
            400,
            "<h1>400 Bad Request</h1><p>This table view is not searchable.</p>",
        );
    }
    for (column, _, operator) in &filters {
        if has_where {
            query.push_str(" AND ");
        } else {
            query.push_str(" WHERE ");
            has_where = true;
        }
        query.push_str(&tableview_filter_condition(column, *operator));
    }
    if tableview.sortable {
        query.push_str(&format!(
            " ORDER BY {}.{} {order}",
            quote_identifier("zelyra_view"),
            quote_identifier(sort)
        ));
    }
    query.push_str(" LIMIT :limit OFFSET :offset");
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
    if tableview.searchable && !search.is_empty() {
        params.push((
            "search".into(),
            zelyra_database::QueryValue::String(search.clone()),
        ));
    }
    for (column, value, operator) in filters {
        if !operator.needs_value() {
            continue;
        }
        let filter = tableview
            .filters
            .iter()
            .find(|filter| filter.name == column)
            .expect("tableview filter was validated");
        let value = match tableview_filter_query_value(filter, &value) {
            Ok(value) => value,
            Err(message) => {
                return Response::html(400, format!("<h1>400 Bad Request</h1><p>{message}</p>"))
            }
        };
        params.push((format!("filter_{column}"), value));
    }
    let result = match zelyra_database::execute_mariadb_query(database_url, &query, params) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("zelyra web: tableview query failed: {error}");
            return Response::html(500, "<h1>500 Internal Server Error</h1>");
        }
    };
    Response::html(
        200,
        render_tableview(
            tableview,
            &result.rows,
            TableViewRenderState {
                query_values: &query_values,
                search: &search,
                sort,
                order,
                page,
                per_page,
            },
        ),
    )
}

struct TableViewRenderState<'a> {
    query_values: &'a HashMap<String, String>,
    search: &'a str,
    sort: &'a str,
    order: &'a str,
    page: u64,
    per_page: u64,
}

fn render_tableview(
    tableview: &TableViewRoute,
    rows: &[Vec<String>],
    state: TableViewRenderState<'_>,
) -> String {
    let TableViewRenderState {
        query_values,
        search,
        sort,
        order,
        page,
        per_page,
    } = state;
    let mut html = String::from("<main><h1>");
    html.push_str(&html_escape(&tableview.title));
    html.push_str("</h1><form method=\"get\" action=\"");
    html.push_str(&html_escape(&tableview.path));
    html.push_str("\">");
    if tableview.searchable {
        html.push_str(
            "<label for=\"search\">Search</label><input id=\"search\" name=\"search\" value=\"",
        );
        html.push_str(&html_escape(search));
        html.push_str("\">");
    }
    if tableview.sortable {
        html.push_str("<label for=\"sort\">Sort</label><select id=\"sort\" name=\"sort\">");
        for column in &tableview.columns {
            html.push_str("<option value=\"");
            html.push_str(&html_escape(column));
            html.push('"');
            if column == sort {
                html.push_str(" selected");
            }
            html.push('>');
            html.push_str(&html_escape(column));
            html.push_str("</option>");
        }
        html.push_str(
            "</select><label for=\"order\">Order</label><select id=\"order\" name=\"order\">",
        );
        for (value, label) in [("asc", "Ascending"), ("desc", "Descending")] {
            html.push_str("<option value=\"");
            html.push_str(value);
            html.push('"');
            if value.eq_ignore_ascii_case(order) {
                html.push_str(" selected");
            }
            html.push('>');
            html.push_str(label);
            html.push_str("</option>");
        }
        html.push_str("</select>");
    }
    for filter in &tableview.filters {
        let selected_operator = selected_filter_operator(query_values, &filter.name);
        html.push_str("<label for=\"filter_");
        html.push_str(&html_escape(&filter.name));
        html.push_str("\">");
        html.push_str(&html_escape(&filter.name));
        html.push_str("</label><select id=\"filter_");
        html.push_str(&html_escape(&filter.name));
        html.push_str("__operator\" name=\"filter_");
        html.push_str(&html_escape(&filter.name));
        html.push_str("__operator\">");
        for operator in tableview_filter_operator_options(filter.kind) {
            html.push_str("<option value=\"");
            html.push_str(operator.key());
            html.push('"');
            if *operator == selected_operator {
                html.push_str(" selected");
            }
            html.push('>');
            html.push_str(operator.label());
            html.push_str("</option>");
        }
        html.push_str("</select><input id=\"filter_");
        html.push_str(&html_escape(&filter.name));
        html.push_str("\" name=\"filter_");
        html.push_str(&html_escape(&filter.name));
        html.push_str("\" value=\"");
        let value_name = format!("filter_{}", filter.name);
        html.push_str(&html_escape(
            query_values
                .get(&value_name)
                .map(String::as_str)
                .unwrap_or(""),
        ));
        html.push_str("\">");
    }
    html.push_str("<button type=\"submit\">Apply</button></form>");
    if rows.is_empty() {
        html.push_str("<p>No records found.</p>");
    } else {
        html.push_str("<table><thead><tr>");
        for column in &tableview.columns {
            html.push_str("<th>");
            html.push_str(&html_escape(column));
            html.push_str("</th>");
        }
        html.push_str("</tr></thead><tbody>");
        for row in rows {
            html.push_str("<tr>");
            for index in 0..tableview.columns.len() {
                html.push_str("<td>");
                html.push_str(&html_escape(
                    row.get(index).map(String::as_str).unwrap_or(""),
                ));
                html.push_str("</td>");
            }
            html.push_str("</tr>");
        }
        html.push_str("</tbody></table>");
    }
    html.push_str("<nav class=\"zelyra-pagination\">");
    if page > 1 {
        html.push_str("<a href=\"");
        html.push_str(&html_escape(&tableview_page_url(
            tableview,
            query_values,
            search,
            sort,
            order,
            page - 1,
        )));
        html.push_str("\">Previous</a> ");
    }
    html.push_str("<span>Page ");
    html.push_str(&page.to_string());
    html.push_str("</span>");
    if rows.len() as u64 == per_page {
        html.push_str(" <a href=\"");
        html.push_str(&html_escape(&tableview_page_url(
            tableview,
            query_values,
            search,
            sort,
            order,
            page + 1,
        )));
        html.push_str("\">Next</a>");
    }
    html.push_str("</nav></main>");
    html
}

fn tableview_page_url(
    tableview: &TableViewRoute,
    query_values: &HashMap<String, String>,
    search: &str,
    sort: &str,
    order: &str,
    page: u64,
) -> String {
    let mut url = format!(
        "{}?page={page}&sort={sort}&order={}",
        tableview.path,
        order.to_ascii_lowercase()
    );
    if !search.is_empty() {
        url.push_str("&search=");
        url.push_str(&url_encode(search));
    }
    for (name, value) in query_values {
        if matches!(name.as_str(), "page" | "sort" | "order" | "search") || value.is_empty() {
            continue;
        }
        url.push('&');
        url.push_str(&url_encode(name));
        url.push('=');
        url.push_str(&url_encode(value));
    }
    url
}

fn dispatch_crud_detail(
    crud: &CrudRoute,
    request: &Request,
    path_params: &HashMap<String, String>,
    database_url: Option<&str>,
    ui_actions: CrudUiActions,
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
            .map(|column| {
                format!(
                    "{} AS {}",
                    crud_column_expression(&crud.schema, table, column),
                    quote_identifier(column)
                )
            })
            .collect::<Vec<_>>()
            .join(", "),
        format_args!(
            "{} AS {}{}",
            quote_identifier(&crud.table),
            quote_identifier("base"),
            crud_relation_joins(&crud.schema, table)
        ),
        format_args!("{}.{}", quote_identifier("base"), quote_identifier("id"))
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
    Response::html(
        200,
        render_crud_detail_with_actions(crud, &columns, row, &id_text, ui_actions),
    )
}

fn dispatch_crud_delete(
    crud: &CrudRoute,
    request: &Request,
    path_params: &HashMap<String, String>,
    database_url: Option<&str>,
) -> Response {
    if request.method != "POST" {
        return Response::html(405, "<h1>405 Method Not Allowed</h1>");
    }
    let Some(database_url) = database_url else {
        return Response::html(
            503,
            "<h1>503 Service Unavailable</h1><p>DATABASE_URL is required for CRUD actions.</p>",
        );
    };
    let Some(id) = path_params.get("id") else {
        return Response::html(400, "<h1>400 Bad Request</h1>");
    };
    let Ok(id) = id.parse::<i64>() else {
        return Response::html(400, "<h1>400 Bad Request</h1><p>id must be an integer.</p>");
    };
    let input = match parse_urlencoded(&request.body) {
        Ok(input) => input,
        Err(error) => {
            return Response::html(400, format!("<h1>400 Bad Request</h1><p>{error}</p>"))
        }
    };
    if !crud
        .csrf
        .verify(input.get("_zelyra_csrf").map(String::as_str))
    {
        return Response::html(403, "<h1>403 Forbidden</h1><p>Invalid CSRF token.</p>");
    }
    let query = format!(
        "DELETE FROM {} WHERE {} = :id",
        quote_identifier(&crud.table),
        quote_identifier("id")
    );
    if let Err(error) = zelyra_database::execute_mariadb_queries(
        database_url,
        &[zelyra_database::Query {
            sql: query,
            params: vec![("id".into(), zelyra_database::QueryValue::Int(id))],
        }],
        true,
    ) {
        eprintln!("zelyra web: CRUD delete failed: {error}");
        return Response::html(500, "<h1>500 Internal Server Error</h1>");
    }
    Response::redirect(&crud.path)
}

fn positive_query_value(values: &HashMap<String, String>, name: &str) -> Option<u64> {
    values
        .get(name)
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FilterOperator {
    Equal,
    Contains,
    StartsWith,
    EndsWith,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    IsNull,
    IsNotNull,
}

impl FilterOperator {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "eq" => Some(Self::Equal),
            "contains" => Some(Self::Contains),
            "starts_with" => Some(Self::StartsWith),
            "ends_with" => Some(Self::EndsWith),
            "gt" => Some(Self::GreaterThan),
            "gte" => Some(Self::GreaterThanOrEqual),
            "lt" => Some(Self::LessThan),
            "lte" => Some(Self::LessThanOrEqual),
            "is_null" => Some(Self::IsNull),
            "is_not_null" => Some(Self::IsNotNull),
            _ => None,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Equal => "eq",
            Self::Contains => "contains",
            Self::StartsWith => "starts_with",
            Self::EndsWith => "ends_with",
            Self::GreaterThan => "gt",
            Self::GreaterThanOrEqual => "gte",
            Self::LessThan => "lt",
            Self::LessThanOrEqual => "lte",
            Self::IsNull => "is_null",
            Self::IsNotNull => "is_not_null",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Equal => "is",
            Self::Contains => "contains",
            Self::StartsWith => "starts with",
            Self::EndsWith => "ends with",
            Self::GreaterThan => "greater than",
            Self::GreaterThanOrEqual => "at least",
            Self::LessThan => "less than",
            Self::LessThanOrEqual => "at most",
            Self::IsNull => "is empty",
            Self::IsNotNull => "is not empty",
        }
    }

    fn needs_value(self) -> bool {
        !matches!(self, Self::IsNull | Self::IsNotNull)
    }
}

fn tableview_filter_operator_supported(
    kind: TableViewFilterKind,
    operator: FilterOperator,
) -> bool {
    match kind {
        TableViewFilterKind::Text => matches!(
            operator,
            FilterOperator::Equal
                | FilterOperator::Contains
                | FilterOperator::StartsWith
                | FilterOperator::EndsWith
                | FilterOperator::IsNull
                | FilterOperator::IsNotNull
        ),
        TableViewFilterKind::Numeric => matches!(
            operator,
            FilterOperator::Equal
                | FilterOperator::GreaterThan
                | FilterOperator::GreaterThanOrEqual
                | FilterOperator::LessThan
                | FilterOperator::LessThanOrEqual
                | FilterOperator::IsNull
                | FilterOperator::IsNotNull
        ),
        TableViewFilterKind::Bool | TableViewFilterKind::Other => matches!(
            operator,
            FilterOperator::Equal | FilterOperator::IsNull | FilterOperator::IsNotNull
        ),
    }
}

fn tableview_filter_operator_options(kind: TableViewFilterKind) -> &'static [FilterOperator] {
    match kind {
        TableViewFilterKind::Text => &[
            FilterOperator::Equal,
            FilterOperator::Contains,
            FilterOperator::StartsWith,
            FilterOperator::EndsWith,
            FilterOperator::IsNull,
            FilterOperator::IsNotNull,
        ],
        TableViewFilterKind::Numeric => &[
            FilterOperator::Equal,
            FilterOperator::GreaterThan,
            FilterOperator::GreaterThanOrEqual,
            FilterOperator::LessThan,
            FilterOperator::LessThanOrEqual,
            FilterOperator::IsNull,
            FilterOperator::IsNotNull,
        ],
        TableViewFilterKind::Bool | TableViewFilterKind::Other => &[
            FilterOperator::Equal,
            FilterOperator::IsNull,
            FilterOperator::IsNotNull,
        ],
    }
}

fn tableview_filter_condition(column: &str, operator: FilterOperator) -> String {
    let expression = format!(
        "{}.{}",
        quote_identifier("zelyra_view"),
        quote_identifier(column)
    );
    let parameter = format!(":filter_{column}");
    match operator {
        FilterOperator::Equal => format!("{expression} = {parameter}"),
        FilterOperator::Contains => format!("{expression} LIKE CONCAT('%', {parameter}, '%')"),
        FilterOperator::StartsWith => format!("{expression} LIKE CONCAT({parameter}, '%')"),
        FilterOperator::EndsWith => format!("{expression} LIKE CONCAT('%', {parameter})"),
        FilterOperator::GreaterThan => format!("{expression} > {parameter}"),
        FilterOperator::GreaterThanOrEqual => format!("{expression} >= {parameter}"),
        FilterOperator::LessThan => format!("{expression} < {parameter}"),
        FilterOperator::LessThanOrEqual => format!("{expression} <= {parameter}"),
        FilterOperator::IsNull => format!("{expression} IS NULL"),
        FilterOperator::IsNotNull => format!("{expression} IS NOT NULL"),
    }
}

fn tableview_filter_query_value(
    filter: &TableViewFilter,
    value: &str,
) -> Result<zelyra_database::QueryValue, String> {
    match filter.kind {
        TableViewFilterKind::Bool => match value {
            "true" | "1" => Ok(zelyra_database::QueryValue::Bool(true)),
            "false" | "0" => Ok(zelyra_database::QueryValue::Bool(false)),
            _ => Err(format!("filter_{} must be true or false", filter.name)),
        },
        TableViewFilterKind::Numeric => value
            .parse::<f64>()
            .map(zelyra_database::QueryValue::Float)
            .map_err(|_| format!("filter_{} must be a number", filter.name)),
        TableViewFilterKind::Text | TableViewFilterKind::Other => {
            Ok(zelyra_database::QueryValue::String(value.into()))
        }
    }
}

fn column_is_text(column: &zelyra_database::Column) -> bool {
    let sql_type = column.sql_type.to_ascii_uppercase();
    sql_type.contains("CHAR") || sql_type.contains("TEXT")
}

fn column_is_numeric(column: &zelyra_database::Column) -> bool {
    let sql_type = column.sql_type.to_ascii_uppercase();
    sql_type.contains("INT")
        || sql_type.contains("DECIMAL")
        || sql_type.contains("NUMERIC")
        || sql_type.contains("DOUBLE")
        || sql_type.contains("FLOAT")
        || sql_type.contains("REAL")
}

fn filter_operator_supported(column: &zelyra_database::Column, operator: FilterOperator) -> bool {
    match operator {
        FilterOperator::Equal | FilterOperator::IsNull | FilterOperator::IsNotNull => true,
        FilterOperator::Contains | FilterOperator::StartsWith | FilterOperator::EndsWith => {
            column_is_text(column)
        }
        FilterOperator::GreaterThan
        | FilterOperator::GreaterThanOrEqual
        | FilterOperator::LessThan
        | FilterOperator::LessThanOrEqual => column_is_numeric(column) || column_is_text(column),
    }
}

fn filter_operator_options(column: &zelyra_database::Column) -> &'static [FilterOperator] {
    if column_is_text(column) {
        &[
            FilterOperator::Equal,
            FilterOperator::Contains,
            FilterOperator::StartsWith,
            FilterOperator::EndsWith,
            FilterOperator::IsNull,
            FilterOperator::IsNotNull,
        ]
    } else if column_is_numeric(column) {
        &[
            FilterOperator::Equal,
            FilterOperator::GreaterThan,
            FilterOperator::GreaterThanOrEqual,
            FilterOperator::LessThan,
            FilterOperator::LessThanOrEqual,
            FilterOperator::IsNull,
            FilterOperator::IsNotNull,
        ]
    } else {
        &[
            FilterOperator::Equal,
            FilterOperator::IsNull,
            FilterOperator::IsNotNull,
        ]
    }
}

fn selected_filter_operator(
    query_values: &HashMap<String, String>,
    column: &str,
) -> FilterOperator {
    let operator_name = format!("filter_{column}__operator");
    if let Some(operator) = query_values
        .get(&operator_name)
        .and_then(|value| FilterOperator::parse(value))
    {
        return operator;
    }
    let prefix = format!("filter_{column}__");
    query_values
        .keys()
        .filter_map(|name| name.strip_prefix(&prefix))
        .find_map(FilterOperator::parse)
        .unwrap_or(FilterOperator::Equal)
}

fn filter_condition(
    table: &zelyra_database::Table,
    column: &str,
    operator: FilterOperator,
) -> String {
    let expression = crud_storage_expression(table, column);
    let parameter = format!(":filter_{column}");
    match operator {
        FilterOperator::Equal => format!("{expression} = {parameter}"),
        FilterOperator::Contains => {
            format!("{expression} LIKE CONCAT('%', {parameter}, '%')")
        }
        FilterOperator::StartsWith => {
            format!("{expression} LIKE CONCAT({parameter}, '%')")
        }
        FilterOperator::EndsWith => format!("{expression} LIKE CONCAT('%', {parameter})"),
        FilterOperator::GreaterThan => format!("{expression} > {parameter}"),
        FilterOperator::GreaterThanOrEqual => format!("{expression} >= {parameter}"),
        FilterOperator::LessThan => format!("{expression} < {parameter}"),
        FilterOperator::LessThanOrEqual => format!("{expression} <= {parameter}"),
        FilterOperator::IsNull => format!("{expression} IS NULL"),
        FilterOperator::IsNotNull => format!("{expression} IS NOT NULL"),
    }
}

fn filter_query_value(
    column: &zelyra_database::Column,
    value: &str,
) -> Result<zelyra_database::QueryValue, String> {
    let sql_type = column.sql_type.to_ascii_uppercase();
    if sql_type.contains("BOOL") {
        return match value {
            "true" | "1" => Ok(zelyra_database::QueryValue::Bool(true)),
            "false" | "0" => Ok(zelyra_database::QueryValue::Bool(false)),
            _ => Err(format!("filter_{} must be true or false", column.name)),
        };
    }
    if sql_type.contains("INT") {
        return value
            .parse::<i64>()
            .map(zelyra_database::QueryValue::Int)
            .map_err(|_| format!("filter_{} must be an integer", column.name));
    }
    if sql_type.contains("DECIMAL") || sql_type.contains("NUMERIC") || sql_type.contains("DOUBLE") {
        return value
            .parse::<f64>()
            .map(zelyra_database::QueryValue::Float)
            .map_err(|_| format!("filter_{} must be a number", column.name));
    }
    Ok(zelyra_database::QueryValue::String(value.into()))
}

struct CrudListView<'a> {
    query_columns: &'a [&'a str],
    display_columns: &'a [&'a str],
    filter_columns: &'a [&'a str],
    sort_columns: &'a [&'a str],
    rows: &'a [Vec<String>],
    search: &'a str,
    query_values: &'a HashMap<String, String>,
    sort: &'a str,
    order: &'a str,
    page: u64,
    per_page: u64,
}

#[cfg(test)]
fn render_crud_list(crud: &CrudRoute, view: CrudListView<'_>) -> String {
    render_crud_list_with_actions(
        crud,
        view,
        CrudUiActions {
            create: true,
            edit: true,
            delete: true,
        },
    )
}

fn render_crud_list_with_actions(
    crud: &CrudRoute,
    view: CrudListView<'_>,
    ui_actions: CrudUiActions,
) -> String {
    let CrudListView {
        query_columns,
        display_columns,
        filter_columns,
        sort_columns,
        rows,
        search,
        query_values,
        sort,
        order,
        page,
        per_page,
    } = view;
    let mut html = String::from("<main><h1>");
    html.push_str(&html_escape(&crud.title));
    html.push_str("</h1>");
    if ui_actions.create {
        html.push_str("<p><a href=\"");
        html.push_str(&html_escape(&format!("{}/new", crud.path)));
        html.push_str("\">Create new</a></p>");
    }
    html.push_str("<form method=\"get\" action=\"");
    html.push_str(&html_escape(&crud.path));
    html.push_str(
        "\"><label for=\"search\">Search</label><input id=\"search\" name=\"search\" value=\"",
    );
    html.push_str(&html_escape(search));
    html.push_str("\"><label for=\"sort\">Sort</label><select id=\"sort\" name=\"sort\">");
    for column in sort_columns {
        html.push_str("<option value=\"");
        html.push_str(&html_escape(column));
        html.push('"');
        if *column == sort {
            html.push_str(" selected");
        }
        html.push('>');
        html.push_str(&html_escape(&crud_column_label(
            &crud.schema,
            &crud.table,
            column,
        )));
        html.push_str("</option>");
    }
    html.push_str(
        "</select><label for=\"order\">Order</label><select id=\"order\" name=\"order\">",
    );
    for (value, label) in [("asc", "Ascending"), ("desc", "Descending")] {
        html.push_str("<option value=\"");
        html.push_str(value);
        html.push('"');
        if value.eq_ignore_ascii_case(order) {
            html.push_str(" selected");
        }
        html.push('>');
        html.push_str(label);
        html.push_str("</option>");
    }
    html.push_str("</select>");
    for column in filter_columns {
        let schema_table = crud
            .schema
            .tables
            .iter()
            .find(|table| table.name == crud.table);
        let storage_column = schema_table
            .and_then(|table| crud_foreign_key(table, column))
            .map(|foreign_key| foreign_key.column.as_str())
            .unwrap_or(column);
        let schema_column = schema_table.and_then(|table| {
            table
                .columns
                .iter()
                .find(|candidate| candidate.name == storage_column)
        });
        let selected_operator = selected_filter_operator(query_values, column);
        html.push_str("<label for=\"filter_");
        html.push_str(&html_escape(column));
        html.push_str("\">");
        html.push_str(&html_escape(&format!(
            "Filter {}",
            crud_column_label(&crud.schema, &crud.table, column)
        )));
        html.push_str("</label><select id=\"filter_");
        html.push_str(&html_escape(column));
        html.push_str("__operator\" name=\"filter_");
        html.push_str(&html_escape(column));
        html.push_str("__operator\">");
        if let Some(schema_column) = schema_column {
            for operator in filter_operator_options(schema_column) {
                html.push_str("<option value=\"");
                html.push_str(operator.key());
                html.push('"');
                if *operator == selected_operator {
                    html.push_str(" selected");
                }
                html.push('>');
                html.push_str(operator.label());
                html.push_str("</option>");
            }
        } else {
            html.push_str("<option value=\"eq\" selected>is</option>");
        }
        html.push_str("</select><input id=\"filter_");
        html.push_str(&html_escape(column));
        html.push_str("\" name=\"filter_");
        html.push_str(&html_escape(column));
        html.push_str("\" value=\"");
        html.push_str(&html_escape(
            query_values
                .get(&format!("filter_{column}"))
                .map(String::as_str)
                .unwrap_or(""),
        ));
        html.push_str("\">");
    }
    html.push_str("<button type=\"submit\">Apply</button></form>");
    if rows.is_empty() {
        html.push_str("<p>");
        html.push_str(&html_escape(
            crud.list_view
                .empty
                .as_deref()
                .unwrap_or("No records found."),
        ));
        html.push_str("</p>");
    } else if crud.list_view.mode == CrudListViewMode::Cards {
        html.push_str("<section class=\"zelyra-crud-cards\">");
        for row in rows {
            html.push_str("<article class=\"zelyra-crud-card\">");
            for column in display_columns {
                let value = query_columns
                    .iter()
                    .position(|query_column| query_column == column)
                    .and_then(|index| row.get(index))
                    .map(String::as_str)
                    .unwrap_or("");
                html.push_str("<dl><dt>");
                html.push_str(&html_escape(&crud_column_label(
                    &crud.schema,
                    &crud.table,
                    column,
                )));
                html.push_str("</dt><dd>");
                if *column == "id" {
                    html.push_str("<a href=\"");
                    html.push_str(&html_escape(&format!("{}/{}", crud.path, value)));
                    html.push_str("\">");
                    html.push_str(&html_escape(value));
                    html.push_str("</a>");
                } else {
                    html.push_str(&html_escape(value));
                }
                html.push_str("</dd></dl>");
            }
            html.push_str("</article>");
        }
        html.push_str("</section>");
    } else {
        html.push_str("<table><thead><tr>");
        for column in display_columns {
            html.push_str("<th>");
            html.push_str(&html_escape(&crud_column_label(
                &crud.schema,
                &crud.table,
                column,
            )));
            html.push_str("</th>");
        }
        html.push_str("</tr></thead><tbody>");
        for row in rows {
            html.push_str("<tr>");
            for column in display_columns {
                html.push_str("<td>");
                let value = query_columns
                    .iter()
                    .position(|query_column| query_column == column)
                    .and_then(|index| row.get(index))
                    .map(String::as_str)
                    .unwrap_or("");
                if *column == "id" {
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
            query_values,
            sort,
            order,
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
            query_values,
            sort,
            order,
            page + 1,
            per_page,
        )));
        html.push_str("\">Next</a>");
    }
    html.push_str("</nav></main>");
    html
}

#[cfg(test)]
fn render_crud_detail(crud: &CrudRoute, columns: &[&str], row: &[String], id: &str) -> String {
    render_crud_detail_with_actions(
        crud,
        columns,
        row,
        id,
        CrudUiActions {
            create: true,
            edit: true,
            delete: true,
        },
    )
}

fn render_crud_detail_with_actions(
    crud: &CrudRoute,
    columns: &[&str],
    row: &[String],
    id: &str,
    ui_actions: CrudUiActions,
) -> String {
    let title = crud
        .detail_view
        .title
        .clone()
        .unwrap_or_else(|| format!("{} detail", crud.title));
    let cards = crud.detail_view.mode == CrudDetailViewMode::Cards;
    let mut html = String::from("<main><p><a href=\"");
    html.push_str(&html_escape(&crud.path));
    html.push_str("\">Back to list</a></p><h1>");
    html.push_str(&html_escape(&title));
    html.push_str("</h1>");
    if cards {
        html.push_str("<article class=\"zelyra-crud-detail-card\">");
    }
    html.push_str("<dl>");
    for (column, value) in columns.iter().zip(row) {
        html.push_str("<dt>");
        html.push_str(&html_escape(&crud_column_label(
            &crud.schema,
            &crud.table,
            column,
        )));
        html.push_str("</dt><dd>");
        html.push_str(&html_escape(value));
        html.push_str("</dd>");
    }
    if ui_actions.edit || ui_actions.create {
        html.push_str("</dl><p>");
        if ui_actions.edit {
            html.push_str("<a href=\"");
            html.push_str(&html_escape(&format!("{}/{}/edit", crud.path, id)));
            html.push_str("\">Edit</a>");
        }
        if ui_actions.create {
            if ui_actions.edit {
                html.push(' ');
            }
            html.push_str("<a href=\"");
            html.push_str(&html_escape(&format!("{}/new", crud.path)));
            html.push_str("\">Create new</a>");
        }
        html.push_str("</p>");
    } else {
        html.push_str("</dl>");
    }
    if ui_actions.delete {
        if let Some(title) = &crud.delete_view.title {
            html.push_str("<h2>");
            html.push_str(&html_escape(title));
            html.push_str("</h2>");
        }
        if let Some(message) = &crud.delete_view.message {
            html.push_str("<p class=\"zelyra-delete-message\">");
            html.push_str(&html_escape(message));
            html.push_str("</p>");
        }
        html.push_str("<form method=\"post\" action=\"");
        html.push_str(&html_escape(&format!("{}/{}/delete", crud.path, id)));
        html.push_str("\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"");
        html.push_str(&html_escape(crud.csrf.token()));
        html.push_str("\"><button type=\"submit\">");
        html.push_str(&html_escape(
            crud.delete_view.submit.as_deref().unwrap_or("Delete"),
        ));
        html.push_str("</button></form>");
    }
    if cards {
        html.push_str("</article>");
    }
    html.push_str("</main>");
    html
}

fn crud_page_url(
    path: &str,
    search: &str,
    query_values: &HashMap<String, String>,
    sort: &str,
    order: &str,
    page: u64,
    per_page: u64,
) -> String {
    let mut url = format!(
        "{path}?page={page}&per_page={per_page}&sort={sort}&order={}",
        order.to_ascii_lowercase()
    );
    if !search.is_empty() {
        url.push_str("&search=");
        url.push_str(&url_encode(search));
    }
    let mut filter_names = query_values
        .keys()
        .filter(|name| name.starts_with("filter_"))
        .collect::<Vec<_>>();
    filter_names.sort();
    for name in filter_names {
        if let Some(value) = query_values.get(name) {
            if !value.is_empty() {
                url.push('&');
                url.push_str(name);
                url.push('=');
                url.push_str(&url_encode(value));
            }
        }
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
    if let Some(title) = &route.form_view.title {
        html.push_str("<h1>");
        html.push_str(&html_escape(title));
        html.push_str("</h1>");
    }
    let cards = route.form_view.mode == CrudFormViewMode::Cards;
    if cards {
        html.push_str("<section class=\"zelyra-crud-form-card\">");
    }
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
    html.push_str("<button type=\"submit\">");
    html.push_str(&html_escape(
        route.form_view.submit.as_deref().unwrap_or("Submit"),
    ));
    html.push_str("</button></form>");
    if cards {
        html.push_str("</section>");
    }
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

fn session_token_hash(token: &str) -> String {
    let digest = Blake2s256::digest(token.as_bytes());
    hex_encode(&digest)
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
        204 => "No Content",
        200 => "OK",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        202 => "Accepted",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
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
    let response = match read_http_request(stream) {
        Ok(raw) => match parse_request(&raw) {
            Ok(request) => app.dispatch(&request),
            Err(error) if error.message.contains("request body exceeds") => Response::json(
                413,
                "{\"error\":{\"code\":\"PayloadTooLarge\",\"message\":\"request body is too large\"}}",
            ),
            Err(_) => Response::html(400, "<h1>400 Bad Request</h1>"),
        },
        Err(RequestReadError::PayloadTooLarge) => Response::json(
            413,
            "{\"error\":{\"code\":\"PayloadTooLarge\",\"message\":\"request body is too large\"}}",
        ),
        Err(RequestReadError::Http(error)) => {
            drop(error);
            Response::html(400, "<h1>400 Bad Request</h1>")
        }
        Err(RequestReadError::Io(error)) => return Err(error),
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
            requires_auth: false,
            permissions: Vec::new(),
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
            requires_auth: false,
            permissions: Vec::new(),
            csrf: CsrfProtection::new("csrf-token"),
            form_view: CrudFormViewDef::default(),
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
    fn dispatches_typed_api_routes_through_a_handler() {
        let app = WebApp::new(Vec::new(), Vec::new()).with_apis(vec![ApiRoute::new(
            "GET",
            "/customers/{id}",
            |_request, parameters| {
                Response::json(200, format!("{{\"id\":\"{}\"}}", parameters["id"]))
            },
        )]);
        let request =
            parse_request("GET /customers/42 HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let response = app.dispatch(&request);
        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "application/json; charset=utf-8");
        assert_eq!(response.body, "{\"id\":\"42\"}");
        assert_eq!(
            app.dispatch(&parse_request("POST /customers/42 HTTP/1.1\r\n\r\n").unwrap())
                .status,
            405
        );
    }

    #[test]
    fn adds_cors_headers_for_an_allowed_api_origin() {
        let policy = CorsPolicy::new(vec!["http://localhost:5173".into()], false).unwrap();
        let app = WebApp::new(Vec::new(), Vec::new())
            .with_apis(vec![ApiRoute::new("GET", "/health", |_request, _| {
                Response::json(200, "{}")
            })])
            .with_cors(policy);
        let request =
            parse_request("GET /health HTTP/1.1\r\nOrigin: http://localhost:5173\r\n\r\n").unwrap();
        let response = app.dispatch(&request);
        assert_eq!(response.status, 200);
        assert!(response.headers.contains(&(
            "Access-Control-Allow-Origin".into(),
            "http://localhost:5173".into()
        )));
        assert!(response.headers.contains(&("Vary".into(), "Origin".into())));
    }

    #[test]
    fn answers_cors_preflight_for_an_allowed_api_method() {
        let policy = CorsPolicy::new(vec!["https://app.example".into()], true).unwrap();
        let app = WebApp::new(Vec::new(), Vec::new())
            .with_apis(vec![ApiRoute::new("POST", "/customers", |_request, _| {
                Response::json(201, "{}")
            })])
            .with_cors(policy);
        let request = parse_request(
            "OPTIONS /customers HTTP/1.1\r\nOrigin: https://app.example\r\nAccess-Control-Request-Method: POST\r\nAccess-Control-Request-Headers: content-type, authorization\r\n\r\n",
        )
        .unwrap();
        let response = app.dispatch(&request);
        assert_eq!(response.status, 204);
        assert!(response
            .headers
            .contains(&("Access-Control-Allow-Methods".into(), "POST".into())));
        assert!(response.headers.contains(&(
            "Access-Control-Allow-Headers".into(),
            "content-type, authorization".into()
        )));
        assert!(response
            .headers
            .contains(&("Access-Control-Allow-Credentials".into(), "true".into())));
    }

    #[test]
    fn rejects_disallowed_cors_preflight_origin() {
        let policy = CorsPolicy::new(vec!["https://app.example".into()], false).unwrap();
        let app = WebApp::new(Vec::new(), Vec::new())
            .with_apis(vec![ApiRoute::new("GET", "/health", |_request, _| {
                Response::json(200, "{}")
            })])
            .with_cors(policy);
        let request = parse_request(
            "OPTIONS /health HTTP/1.1\r\nOrigin: https://evil.example\r\nAccess-Control-Request-Method: GET\r\n\r\n",
        )
        .unwrap();
        let response = app.dispatch(&request);
        assert_eq!(response.status, 403);
        assert!(response.body.contains("CorsDenied"));
    }

    #[test]
    fn rejects_invalid_cors_origins() {
        assert!(CorsPolicy::new(vec!["*".into()], false).is_err());
        assert!(CorsPolicy::new(vec!["https://app.example/path".into()], false).is_err());
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
    fn validates_declared_request_body_length() {
        assert!(parse_request("POST /echo HTTP/1.1\r\nContent-Length: 2\r\n\r\nok").is_ok());
        assert!(parse_request("POST /echo HTTP/1.1\r\nContent-Length: 3\r\n\r\nok").is_err());
        assert!(
            parse_request("POST /echo HTTP/1.1\r\nContent-Length: not-a-number\r\n\r\nok").is_err()
        );
    }

    #[test]
    fn rejects_request_bodies_over_the_limit() {
        let raw = format!(
            "POST /echo HTTP/1.1\r\nContent-Length: {}\r\n\r\n",
            MAX_REQUEST_BODY_BYTES + 1
        );
        let error = parse_request(&raw).unwrap_err();
        assert!(error.message.contains("exceeds"));
    }

    #[test]
    fn reads_complete_requests_across_multiple_network_reads() {
        let body = "x".repeat(12_000);
        let raw = format!(
            "POST /echo HTTP/1.1\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let mut reader = std::io::Cursor::new(raw.into_bytes());
        let request = parse_request(&read_http_request_from(&mut reader).unwrap()).unwrap();
        assert_eq!(request.body, body);
    }

    #[test]
    fn rejects_oversized_requests_before_reading_the_body() {
        let raw = format!(
            "POST /echo HTTP/1.1\r\nContent-Length: {}\r\n\r\n",
            MAX_REQUEST_BODY_BYTES + 1
        );
        let mut reader = std::io::Cursor::new(raw.into_bytes());
        assert!(matches!(
            read_http_request_from(&mut reader),
            Err(RequestReadError::PayloadTooLarge)
        ));
    }

    #[test]
    fn rejects_headers_over_the_limit() {
        let raw = format!(
            "GET /echo HTTP/1.1\r\nX-Large: {}\r\n\r\n",
            "x".repeat(MAX_REQUEST_HEADER_BYTES)
        );
        let mut reader = std::io::Cursor::new(raw.into_bytes());
        assert!(matches!(
            read_http_request_from(&mut reader),
            Err(RequestReadError::Http(_))
        ));
    }

    #[test]
    fn serializes_http_response() {
        let wire = Response::html(200, "ok").to_http();
        assert!(wire.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(wire.contains("Content-Length: 2\r\n"));
        assert!(wire.contains("X-Content-Type-Options: nosniff\r\n"));
        assert!(wire.contains("X-Frame-Options: DENY\r\n"));
        assert!(wire.contains("Referrer-Policy: no-referrer\r\n"));
        assert!(wire.ends_with("\r\n\r\nok"));
    }

    #[test]
    fn serializes_redirect_response() {
        let wire = Response::redirect("/customers").to_http();
        assert!(wire.starts_with("HTTP/1.1 303 See Other\r\n"));
        assert!(wire.contains("Location: /customers\r\n"));
    }

    #[test]
    fn generated_password_hash_is_argon2_and_verifiable() {
        let encoded = hash_password("correct horse battery staple").unwrap();
        let parsed = PasswordHash::new(&encoded).unwrap();
        assert!(Argon2::default()
            .verify_password(b"correct horse battery staple", &parsed)
            .is_ok());
        assert!(Argon2::default()
            .verify_password(b"wrong", &parsed)
            .is_err());
    }

    #[test]
    fn password_hash_rejects_empty_password() {
        assert!(hash_password("").is_err());
    }

    #[test]
    fn logout_requires_csrf() {
        let auth = AuthRoute {
            table: "users".into(),
            session_table: None,
            permissions_table: None,
            roles_table: None,
            role_permissions_table: None,
            audit_table: None,
            admin_path: None,
            admin_permission: None,
            admin_role: None,
            schema: Schema {
                database: None,
                tables: Vec::new(),
            },
            csrf: CsrfProtection::new("csrf-token"),
        };
        let app = WebApp::new(Vec::new(), Vec::new()).with_auth_route(auth);
        let request = parse_request("POST /logout HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(app.dispatch(&request).status, 403);
    }

    #[test]
    fn throttles_after_five_failed_login_attempts() {
        let app = WebApp::new(Vec::new(), Vec::new());
        let key = login_throttle_key(" User@Example.test ");
        assert_eq!(key, "user@example.test");
        for attempt in 1..=4 {
            record_login_failure(&app, key.clone());
            assert!(
                !login_is_blocked(&app, &key),
                "blocked on attempt {attempt}"
            );
        }
        record_login_failure(&app, key.clone());
        assert!(login_is_blocked(&app, &key));
        clear_login_failures(&app, &key);
        assert!(!login_is_blocked(&app, &key));
    }

    #[test]
    fn rotating_memory_session_invalidates_previous_token() {
        let old_token = "old-session-token";
        let app = WebApp::new(Vec::new(), Vec::new());
        app.sessions.lock().unwrap().insert(
            old_token.into(),
            Session {
                user_id: None,
                permissions: Vec::new(),
            },
        );
        let auth = AuthRoute {
            table: "users".into(),
            session_table: None,
            permissions_table: None,
            roles_table: None,
            role_permissions_table: None,
            audit_table: None,
            admin_path: None,
            admin_permission: None,
            admin_role: None,
            schema: Schema {
                database: None,
                tables: Vec::new(),
            },
            csrf: CsrfProtection::new("csrf-token"),
        };
        let request = parse_request(&format!(
            "POST /login HTTP/1.1\r\nCookie: zelyra_session={old_token}\r\n\r\n"
        ))
        .unwrap();
        rotate_existing_session(&app, &auth, &request, None).unwrap();
        assert!(!app.sessions.lock().unwrap().contains_key(old_token));
    }

    #[test]
    fn renders_form_with_csrf_and_field_attributes() {
        let route = form_route();
        let html = render_form(&route, &HashMap::new(), &[], None);
        assert!(html.contains("name=\"_zelyra_csrf\" value=\"csrf-token\""));
        assert!(html.contains("name=\"name\" type=\"text\""));
        assert!(html.contains(" required"));
        assert!(html.contains("maxlength=\"20\""));

        let mut cards_route = route.clone();
        cards_route.form_view.mode = CrudFormViewMode::Cards;
        cards_route.form_view.title = Some("Customer form".into());
        cards_route.form_view.submit = Some("Save customer <now>".into());
        let cards_html = render_form(&cards_route, &HashMap::new(), &[], None);
        assert!(cards_html.contains(">Customer form</h1>"));
        assert!(cards_html.contains("zelyra-crud-form-card"));
        assert!(cards_html.contains("Save customer &lt;now&gt;"));
        assert!(cards_html.contains("name=\"_zelyra_csrf\" value=\"csrf-token\""));
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
    fn supports_typed_filter_operators() {
        let text = zelyra_database::Column {
            name: "name".into(),
            sql_type: "VARCHAR(100)".into(),
            nullable: false,
            primary_key: false,
            auto: false,
            unique: false,
            default: None,
        };
        let number = zelyra_database::Column {
            name: "quantity".into(),
            sql_type: "BIGINT".into(),
            nullable: false,
            primary_key: false,
            auto: false,
            unique: false,
            default: None,
        };
        assert_eq!(
            FilterOperator::parse("contains"),
            Some(FilterOperator::Contains)
        );
        assert!(filter_operator_supported(&text, FilterOperator::Contains));
        assert!(!filter_operator_supported(
            &number,
            FilterOperator::Contains
        ));
        assert_eq!(FilterOperator::GreaterThanOrEqual.key(), "gte");
    }

    #[test]
    fn builds_safe_filter_conditions_for_text_and_null_checks() {
        let table = zelyra_database::Table {
            name: "machines".into(),
            columns: vec![zelyra_database::Column {
                name: "name".into(),
                sql_type: "VARCHAR(100)".into(),
                nullable: true,
                primary_key: false,
                auto: false,
                unique: false,
                default: None,
            }],
            foreign_keys: Vec::new(),
            indexes: Vec::new(),
            uniques: Vec::new(),
        };
        assert_eq!(
            filter_condition(&table, "name", FilterOperator::Contains),
            "`base`.`name` LIKE CONCAT('%', :filter_name, '%')"
        );
        assert_eq!(
            filter_condition(&table, "name", FilterOperator::IsNull),
            "`base`.`name` IS NULL"
        );
    }

    #[test]
    fn renders_tableview_with_escaped_values_and_pagination_state() {
        let tableview = TableViewRoute {
            path: "/views/customers".into(),
            title: "Customers".into(),
            source: "SELECT id, name FROM customers".into(),
            columns: vec!["id".into(), "name".into()],
            filters: Vec::new(),
            searchable: true,
            sortable: true,
            page_size: Some(1),
            requires_auth: false,
            permissions: Vec::new(),
        };
        let query_values = HashMap::from([
            ("search".into(), "CNC machine".into()),
            ("sort".into(), "name".into()),
            ("order".into(), "desc".into()),
        ]);
        let html = render_tableview(
            &tableview,
            &[vec!["1".into(), "<unsafe>".into()]],
            TableViewRenderState {
                query_values: &query_values,
                search: "CNC machine",
                sort: "name",
                order: "DESC",
                page: 2,
                per_page: 1,
            },
        );
        assert!(html.contains("&lt;unsafe&gt;"));
        assert!(html.contains("value=\"CNC machine\""));
        assert!(html.contains("page=1&amp;sort=name&amp;order=desc&amp;search=CNC%20machine"));
        assert!(html.contains("name=\"sort\""));
    }

    #[test]
    fn renders_typed_tableview_filters_and_preserves_state() {
        let tableview = TableViewRoute {
            path: "/views/customers".into(),
            title: "Customers".into(),
            source: "SELECT id, orders FROM customer_overview".into(),
            columns: vec!["id".into(), "orders".into()],
            filters: vec![TableViewFilter {
                name: "orders".into(),
                kind: TableViewFilterKind::Numeric,
            }],
            searchable: false,
            sortable: true,
            page_size: Some(25),
            requires_auth: false,
            permissions: Vec::new(),
        };
        let query_values = HashMap::from([
            ("filter_orders".into(), "3".into()),
            ("filter_orders__operator".into(), "gte".into()),
        ]);
        let html = render_tableview(
            &tableview,
            &[vec!["1".into(), "3".into()]],
            TableViewRenderState {
                query_values: &query_values,
                search: "",
                sort: "id",
                order: "ASC",
                page: 1,
                per_page: 25,
            },
        );
        assert!(html.contains("name=\"filter_orders__operator\""));
        assert!(html.contains("<option value=\"gte\" selected>at least</option>"));
        assert!(html.contains("name=\"filter_orders\" value=\"3\""));
        assert_eq!(
            tableview_filter_condition("orders", FilterOperator::GreaterThanOrEqual),
            "`zelyra_view`.`orders` >= :filter_orders"
        );
        assert!(tableview_filter_operator_supported(
            TableViewFilterKind::Numeric,
            FilterOperator::GreaterThanOrEqual
        ));
        assert!(!tableview_filter_operator_supported(
            TableViewFilterKind::Numeric,
            FilterOperator::Contains
        ));
    }

    #[test]
    fn renders_crud_list_with_escaped_rows_and_pagination() {
        let route = CrudRoute {
            path: "/machines".into(),
            title: "Machines".into(),
            table: "machines".into(),
            list_columns: Vec::new(),
            search_columns: Vec::new(),
            filter_columns: Vec::new(),
            list_view: CrudListViewDef::default(),
            detail_view: CrudDetailViewDef::default(),
            delete_view: CrudDeleteViewDef::default(),
            requires_auth: false,
            permissions: Vec::new(),
            create_permissions: Vec::new(),
            edit_permissions: Vec::new(),
            delete_permissions: Vec::new(),
            csrf: CsrfProtection::new("crud-csrf"),
            schema: zelyra_database::Schema {
                database: None,
                tables: Vec::new(),
            },
        };
        let _table = zelyra_database::Table {
            name: "machines".into(),
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
        };
        let columns = ["id", "name"];
        let rows = [vec!["1".into(), "<unsafe>".into()]];
        let query_values = HashMap::new();
        let html = render_crud_list(
            &route,
            CrudListView {
                query_columns: &columns,
                display_columns: &columns,
                filter_columns: &["name"],
                sort_columns: &columns,
                rows: &rows,
                search: "CNC machine",
                query_values: &query_values,
                sort: "id",
                order: "ASC",
                page: 2,
                per_page: 1,
            },
        );
        assert!(html.contains("&lt;unsafe&gt;"));
        assert!(html.contains("value=\"CNC machine\""));
        assert!(html
            .contains("page=1&amp;per_page=1&amp;sort=id&amp;order=asc&amp;search=CNC%20machine"));
        assert!(html
            .contains("page=3&amp;per_page=1&amp;sort=id&amp;order=asc&amp;search=CNC%20machine"));

        let restricted_html = render_crud_list_with_actions(
            &route,
            CrudListView {
                query_columns: &columns,
                display_columns: &columns,
                filter_columns: &["name"],
                sort_columns: &columns,
                rows: &rows,
                search: "",
                query_values: &query_values,
                sort: "id",
                order: "ASC",
                page: 1,
                per_page: 50,
            },
            CrudUiActions {
                create: false,
                edit: false,
                delete: false,
            },
        );
        assert!(!restricted_html.contains("href=\"/machines/new\""));

        let mut cards_route = route.clone();
        cards_route.list_view.mode = CrudListViewMode::Cards;
        cards_route.list_view.empty = Some("Nothing <yet>.".into());
        let cards_html = render_crud_list(
            &cards_route,
            CrudListView {
                query_columns: &columns,
                display_columns: &columns,
                filter_columns: &[],
                sort_columns: &columns,
                rows: &rows,
                search: "",
                query_values: &query_values,
                sort: "id",
                order: "ASC",
                page: 1,
                per_page: 50,
            },
        );
        assert!(cards_html.contains("zelyra-crud-cards"));
        assert!(cards_html.contains("zelyra-crud-card"));
        assert!(!cards_html.contains("<table>"));

        let empty_rows: Vec<Vec<String>> = Vec::new();
        let empty_html = render_crud_list(
            &cards_route,
            CrudListView {
                query_columns: &columns,
                display_columns: &columns,
                filter_columns: &[],
                sort_columns: &columns,
                rows: &empty_rows,
                search: "",
                query_values: &query_values,
                sort: "id",
                order: "ASC",
                page: 1,
                per_page: 50,
            },
        );
        assert!(empty_html.contains("Nothing &lt;yet&gt;."));
    }

    #[test]
    fn renders_relationship_labels_in_crud_views() {
        let schema = Schema {
            database: None,
            tables: vec![
                zelyra_database::Table {
                    name: "departments".into(),
                    columns: vec![zelyra_database::Column {
                        name: "name".into(),
                        sql_type: "VARCHAR(100)".into(),
                        nullable: false,
                        primary_key: false,
                        auto: false,
                        unique: false,
                        default: None,
                    }],
                    foreign_keys: Vec::new(),
                    indexes: Vec::new(),
                    uniques: Vec::new(),
                },
                zelyra_database::Table {
                    name: "machines".into(),
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
                            name: "department_id".into(),
                            sql_type: "BIGINT".into(),
                            nullable: false,
                            primary_key: false,
                            auto: false,
                            unique: false,
                            default: None,
                        },
                    ],
                    foreign_keys: vec![zelyra_database::ForeignKey {
                        column: "department_id".into(),
                        referenced_table: "departments".into(),
                        referenced_column: "id".into(),
                    }],
                    indexes: Vec::new(),
                    uniques: Vec::new(),
                },
            ],
        };
        let route = CrudRoute {
            path: "/machines".into(),
            title: "Machines".into(),
            table: "machines".into(),
            list_columns: Vec::new(),
            search_columns: Vec::new(),
            filter_columns: Vec::new(),
            list_view: CrudListViewDef::default(),
            detail_view: CrudDetailViewDef::default(),
            delete_view: CrudDeleteViewDef::default(),
            requires_auth: false,
            permissions: Vec::new(),
            create_permissions: Vec::new(),
            edit_permissions: Vec::new(),
            delete_permissions: Vec::new(),
            csrf: CsrfProtection::new("crud-csrf"),
            schema,
        };
        let columns = ["id", "department_id"];
        let rows = [vec!["1".into(), "Production".into()]];
        let html = render_crud_list(
            &route,
            CrudListView {
                query_columns: &columns,
                display_columns: &columns,
                filter_columns: &columns,
                sort_columns: &columns,
                rows: &rows,
                search: "",
                query_values: &HashMap::new(),
                sort: "id",
                order: "ASC",
                page: 1,
                per_page: 50,
            },
        );
        assert!(html.contains("<th>Department</th>"));
        assert!(html.contains("<td>Production</td>"));
        assert!(html.contains("Filter Department"));
    }

    #[test]
    fn crud_requires_database_url() {
        let app = WebApp::new(Vec::new(), Vec::new()).with_cruds(vec![CrudRoute {
            path: "/machines".into(),
            title: "Machines".into(),
            table: "machines".into(),
            list_columns: Vec::new(),
            search_columns: Vec::new(),
            filter_columns: Vec::new(),
            list_view: CrudListViewDef::default(),
            detail_view: CrudDetailViewDef::default(),
            delete_view: CrudDeleteViewDef::default(),
            requires_auth: false,
            permissions: Vec::new(),
            create_permissions: Vec::new(),
            edit_permissions: Vec::new(),
            delete_permissions: Vec::new(),
            csrf: CsrfProtection::new("crud-csrf"),
            schema: zelyra_database::Schema {
                database: None,
                tables: Vec::new(),
            },
        }]);
        let request = parse_request("GET /machines HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(app.dispatch(&request).status, 503);
    }

    #[test]
    fn database_capability_denies_crud_before_connecting() {
        let app = WebApp::with_database_url(
            Vec::new(),
            Vec::new(),
            Some("mariadb://root:invalid@127.0.0.1:1/test".into()),
        )
        .with_database_capability(false)
        .with_cruds(vec![CrudRoute {
            path: "/machines".into(),
            title: "Machines".into(),
            table: "machines".into(),
            list_columns: Vec::new(),
            search_columns: Vec::new(),
            filter_columns: Vec::new(),
            list_view: CrudListViewDef::default(),
            detail_view: CrudDetailViewDef::default(),
            delete_view: CrudDeleteViewDef::default(),
            requires_auth: false,
            permissions: Vec::new(),
            create_permissions: Vec::new(),
            edit_permissions: Vec::new(),
            delete_permissions: Vec::new(),
            csrf: CsrfProtection::new("crud-csrf"),
            schema: zelyra_database::Schema {
                database: None,
                tables: Vec::new(),
            },
        }]);
        let request = parse_request("GET /machines HTTP/1.1\r\n\r\n").unwrap();
        let response = app.dispatch(&request);
        assert_eq!(response.status, 403);
        assert!(response.body.contains("Database capability is not granted"));
    }

    #[test]
    fn database_capability_denies_forms_before_database_access() {
        let app = WebApp::new(Vec::new(), vec![form_route()]).with_database_capability(false);
        let request = parse_request("GET /forms/CustomerCreate HTTP/1.1\r\n\r\n").unwrap();
        let response = app.dispatch(&request);
        assert_eq!(response.status, 403);
        assert!(response.body.contains("Database capability is not granted"));
    }

    #[test]
    fn database_capability_denies_persistent_login() {
        let auth = AuthRoute {
            table: "users".into(),
            session_table: Some("sessions".into()),
            permissions_table: None,
            roles_table: None,
            role_permissions_table: None,
            audit_table: None,
            admin_path: None,
            admin_permission: None,
            admin_role: None,
            schema: Schema {
                database: None,
                tables: Vec::new(),
            },
            csrf: CsrfProtection::new("csrf-token"),
        };
        let app = WebApp::new(Vec::new(), Vec::new())
            .with_database_capability(false)
            .with_auth_route(auth);
        let request = parse_request("POST /login HTTP/1.1\r\n\r\n").unwrap();
        let response = app.dispatch(&request);
        assert_eq!(response.status, 403);
        assert!(response.body.contains("Database capability is not granted"));
    }

    #[test]
    fn protects_routes_with_authentication_and_permissions() {
        let route = Route {
            path: "/admin".into(),
            html: "<h1>Admin</h1>".into(),
            requires_auth: true,
            permissions: vec!["admin.view".into()],
        };
        let app = WebApp::new(vec![route], Vec::new())
            .with_auth(Some("test-token".into()), vec!["admin.view".into()]);
        let request = parse_request("GET /admin HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(app.dispatch(&request).status, 401);
        let request =
            parse_request("GET /admin HTTP/1.1\r\nAuthorization: Bearer wrong\r\n\r\n").unwrap();
        assert_eq!(app.dispatch(&request).status, 401);
        let request =
            parse_request("GET /admin HTTP/1.1\r\nAuthorization: Bearer test-token\r\n\r\n")
                .unwrap();
        assert_eq!(app.dispatch(&request).status, 200);
        let app = WebApp::new(
            vec![Route {
                path: "/admin".into(),
                html: "<h1>Admin</h1>".into(),
                requires_auth: true,
                permissions: vec!["admin.delete".into()],
            }],
            Vec::new(),
        )
        .with_auth(Some("test-token".into()), vec!["admin.view".into()]);
        assert_eq!(app.dispatch(&request).status, 403);
    }

    #[test]
    fn protects_generated_crud_forms_with_action_permissions() {
        let mut form = form_route();
        form.requires_auth = true;
        form.permissions = vec!["customers.create".into()];
        let app = WebApp::new(Vec::new(), vec![form])
            .with_auth(Some("test-token".into()), vec!["customers.create".into()]);

        let request = parse_request("GET /forms/CustomerCreate HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(app.dispatch(&request).status, 401);
        let request = parse_request(
            "GET /forms/CustomerCreate HTTP/1.1\r\nAuthorization: Bearer test-token\r\n\r\n",
        )
        .unwrap();
        assert_eq!(app.dispatch(&request).status, 200);

        let mut form = form_route();
        form.requires_auth = true;
        form.permissions = vec!["customers.edit".into()];
        let app = WebApp::new(Vec::new(), vec![form])
            .with_auth(Some("test-token".into()), vec!["customers.create".into()]);
        let response = app.dispatch(&request);
        assert_eq!(response.status, 403);
        assert!(response.body.contains("customers.edit"));
    }

    #[test]
    fn protects_crud_delete_with_delete_permission() {
        let app = WebApp::with_database_url(
            Vec::new(),
            Vec::new(),
            Some("mariadb://root:invalid@127.0.0.1:1/test".into()),
        )
        .with_auth(Some("test-token".into()), vec!["customers.view".into()])
        .with_cruds(vec![CrudRoute {
            path: "/customers".into(),
            title: "Customers".into(),
            table: "customers".into(),
            list_columns: Vec::new(),
            search_columns: Vec::new(),
            filter_columns: Vec::new(),
            list_view: CrudListViewDef::default(),
            detail_view: CrudDetailViewDef::default(),
            delete_view: CrudDeleteViewDef::default(),
            requires_auth: true,
            permissions: vec!["customers.view".into()],
            create_permissions: vec!["customers.create".into()],
            edit_permissions: vec!["customers.edit".into()],
            delete_permissions: vec!["customers.delete".into()],
            csrf: CsrfProtection::new("crud-csrf"),
            schema: Schema {
                database: None,
                tables: Vec::new(),
            },
        }]);
        let request = parse_request(
            "POST /customers/1/delete HTTP/1.1\r\nAuthorization: Bearer test-token\r\n\r\n",
        )
        .unwrap();
        let response = app.dispatch(&request);
        assert_eq!(response.status, 403);
        assert!(response.body.contains("customers.delete"));
    }

    #[test]
    fn protects_api_routes_with_json_authentication_errors() {
        let api = ApiRoute::new("GET", "/customers", |_request, _parameters| {
            Response::json(200, "[]")
        })
        .with_auth(true, vec!["customers.view".into()]);
        let app = WebApp::new(Vec::new(), Vec::new())
            .with_auth(Some("test-token".into()), vec!["customers.view".into()])
            .with_apis(vec![api]);

        let request = parse_request("GET /customers HTTP/1.1\r\n\r\n").unwrap();
        let response = app.dispatch(&request);
        assert_eq!(response.status, 401);
        assert!(response.body.contains("Unauthorized"));
        assert_eq!(response.content_type, "application/json; charset=utf-8");

        let request =
            parse_request("GET /customers HTTP/1.1\r\nAuthorization: Bearer test-token\r\n\r\n")
                .unwrap();
        assert_eq!(app.dispatch(&request).status, 200);

        let api = ApiRoute::new("GET", "/customers", |_request, _parameters| {
            Response::json(200, "[]")
        })
        .with_auth(true, vec!["customers.delete".into()]);
        let app = WebApp::new(Vec::new(), Vec::new())
            .with_auth(Some("test-token".into()), vec!["customers.view".into()])
            .with_apis(vec![api]);
        let response = app.dispatch(&request);
        assert_eq!(response.status, 403);
        assert!(response.body.contains("customers.delete"));
    }

    #[test]
    fn crud_delete_requires_csrf() {
        let app = WebApp::with_database_url(
            Vec::new(),
            Vec::new(),
            Some("mariadb://root:invalid@127.0.0.1:3306/test".into()),
        )
        .with_cruds(vec![CrudRoute {
            path: "/machines".into(),
            title: "Machines".into(),
            table: "machines".into(),
            list_columns: Vec::new(),
            search_columns: Vec::new(),
            filter_columns: Vec::new(),
            list_view: CrudListViewDef::default(),
            detail_view: CrudDetailViewDef::default(),
            delete_view: CrudDeleteViewDef::default(),
            requires_auth: false,
            permissions: Vec::new(),
            create_permissions: Vec::new(),
            edit_permissions: Vec::new(),
            delete_permissions: Vec::new(),
            csrf: CsrfProtection::new("crud-csrf"),
            schema: zelyra_database::Schema {
                database: None,
                tables: Vec::new(),
            },
        }]);
        let request = parse_request(
            "POST /machines/1/delete HTTP/1.1\r\nContent-Length: 18\r\n\r\n_zelyra_csrf=wrong",
        )
        .unwrap();
        assert_eq!(app.dispatch(&request).status, 403);
    }

    #[test]
    fn renders_crud_delete_confirmation_form() {
        let route = CrudRoute {
            path: "/machines".into(),
            title: "Machines".into(),
            table: "machines".into(),
            list_columns: Vec::new(),
            search_columns: Vec::new(),
            filter_columns: Vec::new(),
            list_view: CrudListViewDef::default(),
            detail_view: CrudDetailViewDef::default(),
            delete_view: CrudDeleteViewDef::default(),
            requires_auth: false,
            permissions: Vec::new(),
            create_permissions: Vec::new(),
            edit_permissions: Vec::new(),
            delete_permissions: Vec::new(),
            csrf: CsrfProtection::new("crud-csrf"),
            schema: zelyra_database::Schema {
                database: None,
                tables: Vec::new(),
            },
        };
        let html = render_crud_detail(&route, &["id", "name"], &["1".into(), "CNC".into()], "1");
        assert!(html.contains("method=\"post\" action=\"/machines/1/delete\""));
        assert!(html.contains("name=\"_zelyra_csrf\" value=\"crud-csrf\""));
        assert!(html.contains(">Delete</button>"));

        let restricted_html = render_crud_detail_with_actions(
            &route,
            &["id", "name"],
            &["1".into(), "CNC".into()],
            "1",
            CrudUiActions {
                create: false,
                edit: false,
                delete: false,
            },
        );
        assert!(!restricted_html.contains("/machines/1/edit"));
        assert!(!restricted_html.contains("/machines/new"));
        assert!(!restricted_html.contains(">Delete</button>"));

        let mut custom_route = route.clone();
        custom_route.delete_view.title = Some("Delete machine".into());
        custom_route.delete_view.message = Some("Delete <unsafe>?".into());
        custom_route.delete_view.submit = Some("Delete now".into());
        let custom_html = render_crud_detail(
            &custom_route,
            &["id", "name"],
            &["1".into(), "CNC".into()],
            "1",
        );
        assert!(custom_html.contains(">Delete machine</h2>"));
        assert!(custom_html.contains("Delete &lt;unsafe&gt;?"));
        assert!(custom_html.contains(">Delete now</button>"));
        assert!(custom_html.contains("name=\"_zelyra_csrf\" value=\"crud-csrf\""));

        let mut cards_route = route.clone();
        cards_route.detail_view.mode = CrudDetailViewMode::Cards;
        cards_route.detail_view.title = Some("Machine overview".into());
        let cards_html = render_crud_detail(
            &cards_route,
            &["id", "name"],
            &["1".into(), "CNC <unsafe>".into()],
            "1",
        );
        assert!(cards_html.contains(">Machine overview</h1>"));
        assert!(cards_html.contains("zelyra-crud-detail-card"));
        assert!(cards_html.contains("CNC &lt;unsafe&gt;"));
        assert!(cards_html.contains("name=\"_zelyra_csrf\""));
    }

    #[test]
    fn form_post_requires_csrf_and_reports_validation_errors() {
        let app = WebApp::new(Vec::new(), vec![form_route()]);
        let invalid_csrf = parse_request(
            "POST /forms/CustomerCreate HTTP/1.1\r\nContent-Length: 18\r\n\r\n_zelyra_csrf=wrong",
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
    fn protects_custom_form_actions_with_action_permissions() {
        let mut route = form_route();
        route.form.actions.push(zelyra_ast::FormAction {
            name: "save".into(),
            requires_auth: true,
            permissions: vec!["customers.save".into()],
            statements: Vec::new(),
            success: None,
            redirect: None,
            span: zelyra_ast::Span::default(),
        });
        let app = WebApp::new(Vec::new(), vec![route])
            .with_auth(Some("test-token".into()), vec!["customers.save".into()]);
        let request = parse_request("GET /forms/CustomerCreate HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(app.dispatch(&request).status, 401);
        let request = parse_request(
            "GET /forms/CustomerCreate HTTP/1.1\r\nAuthorization: Bearer test-token\r\n\r\n",
        )
        .unwrap();
        assert_eq!(app.dispatch(&request).status, 200);

        let mut route = form_route();
        route.form.actions.push(zelyra_ast::FormAction {
            name: "save".into(),
            requires_auth: true,
            permissions: vec!["customers.save".into()],
            statements: Vec::new(),
            success: None,
            redirect: None,
            span: zelyra_ast::Span::default(),
        });
        let app = WebApp::new(Vec::new(), vec![route])
            .with_auth(Some("test-token".into()), vec!["customers.view".into()]);
        let response = app.dispatch(&request);
        assert_eq!(response.status, 403);
        assert!(response.body.contains("customers.save"));
    }

    #[test]
    fn form_action_requires_database_url() {
        let mut route = form_route();
        route.form.actions.push(zelyra_ast::FormAction {
            name: "save".into(),
            requires_auth: false,
            permissions: Vec::new(),
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
