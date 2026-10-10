Warning: truncated output (original token count: 72533)
Total output lines: 8069

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use blake2::{Blake2s256, Digest};
use rand_core::{OsRng, RngCore};
use ring::hmac;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use zelyra_ast::{
    CrudDeleteViewDef, CrudDetailViewDef, CrudDetailViewMode, CrudErrorViewDef, CrudFormViewDef,
    CrudFormViewMode, CrudListViewDef, CrudListViewMode, CrudLoadingViewDef, FormDef, TableDef,
    Type,
};
use zelyra_database::{QueryValue, Schema};
use zelyra_forms::{validate, FieldError};
mod forms;
mod http;
mod i18n;
mod password_reset;
mod server;
mod ui;
use forms::*;
pub use forms::{localized_identifier, render_form};
use http::*;
pub use http::{parse_request, HttpError, MAX_REQUEST_BODY_BYTES};
#[cfg(test)]
use i18n::framework_text;
use i18n::{
    field_text, framework_text_with_catalog, identifier as locale_identifier, LOCALE_REFERENCE_END,
    LOCALE_REFERENCE_PARAMETER, LOCALE_REFERENCE_START,
};
pub use password_reset::{PasswordResetMailer, PasswordResetSmtpConfig};
pub use server::serve_app;
#[cfg(test)]
pub(crate) use server::{spawn_connection, MAX_CONCURRENT_CONNECTIONS};
use ui::*;

const ZELYRA_DESIGN_SYSTEM_CSS: &str = include_str!("../assets/zelyra.css");
pub const PROJECT_THEME_CSS_PATH: &str = "/__zelyra/theme.css";
pub const HEALTH_LIVENESS_PATH: &str = "/__zelyra/health/live";
pub const ACCOUNT_SESSIONS_PATH: &str = "/account/sessions";
pub use i18n::{ProjectUiCatalogs, UiLanguage, UiLevel};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub path: String,
    pub html: String,
    pub query: Vec<RouteQuery>,
    pub page_size: Option<u32>,
    pub sort_columns: Vec<String>,
    pub search_columns: Vec<String>,
    pub filters: Vec<TableViewFilter>,
    pub data: Vec<RouteData>,
    pub requires_auth: bool,
    pub permissions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteQuery {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteData {
    pub name: String,
    pub query: String,
    pub fields: Vec<String>,
    pub collection: bool,
    pub optional: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    pub target: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub remote_addr: Option<SocketAddr>,
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
    pub version: Option<String>,
    pub deprecated: bool,
    pub rate_limit: Option<zelyra_ast::ApiRateLimit>,
    handler: Arc<ApiHandler>,
}

#[derive(Clone, Debug)]
struct ApiThrottle {
    window_started: Instant,
    requests: u32,
    window_seconds: u32,
}

const API_THROTTLE_MAX_CLIENTS: usize = 4096;

impl fmt::Debug for ApiRoute {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApiRoute")
            .field("method", &self.method)
            .field("path", &self.path)
            .field("requires_auth", &self.requires_auth)
            .field("permissions", &self.permissions)
            .field("version", &self.version)
            .field("deprecated", &self.deprecated)
            .field("rate_limit", &self.rate_limit)
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
            version: None,
            deprecated: false,
            rate_limit: None,
            handler: Arc::new(handler),
        }
    }

    pub fn with_auth(mut self, requires_auth: bool, permissions: Vec<String>) -> Self {
        self.requires_auth = requires_auth;
        self.permissions = permissions;
        self
    }

    pub fn with_metadata(
        mut self,
        version: Option<String>,
        deprecated: bool,
        rate_limit: Option<zelyra_ast::ApiRateLimit>,
    ) -> Self {
        self.version = version;
        self.deprecated = deprecated;
        self.rate_limit = rate_limit;
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

    pub fn css(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            reason: reason_phrase(status).into(),
            content_type: "text/css; charset=utf-8".into(),
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
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\nReferrer-Policy: same-origin\r\n{}{}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
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
    pub post_only: bool,
    pub audit_table: Option<String>,
    pub audit_event: Option<String>,
    pub audit_chain: bool,
    /// Pre-composed named view layout for generated CRUD forms.
    pub layout_html: Option<String>,
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
    pub loading_view: CrudLoadingViewDef,
    pub error_view: CrudErrorViewDef,
    /// Pre-composed named view layout. The content marker is replaced at request time.
    pub layout_html: Option<String>,
    pub soft_delete: Option<zelyra_ast::CrudSoftDeleteDef>,
    pub actions: Vec<CrudActionRoute>,
    pub requires_auth: bool,
    pub permissions: Vec<String>,
    pub create_permissions: Vec<String>,
    pub edit_permissions: Vec<String>,
    pub delete_permissions: Vec<String>,
    pub schema: Schema,
    pub csrf: CsrfProtection,
}

#[derive(Clone, Debug)]
pub struct CrudActionRoute {
    pub name: String,
    pub label: String,
    pub icon: Option<String>,
    pub confirm: Option<String>,
    pub confirm_page: Option<zelyra_ast::CrudConfirmViewDef>,
    pub form: FormRoute,
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
    pub audit_chain: bool,
    pub admin_path: Option<String>,
    pub admin_permission: Option<String>,
    pub admin_role: Option<String>,
    pub login_rate_limit: zelyra_ast::ApiRateLimit,
    pub login_block_seconds: u32,
    pub reset_tokens_table: Option<String>,
    pub reset_rate_limit: zelyra_ast::ApiRateLimit,
    pub reset_block_seconds: u32,
    pub schema: Schema,
    pub csrf: CsrfProtection,
}

#[derive(Clone, Debug)]
struct Session {
    user_id: Option<i64>,
    permissions: Vec<String>,
    expires_at: Instant,
    device: Option<String>,
}

#[derive(Clone, Debug)]
struct LoginThrottle {
    window_started: Instant,
    window_seconds: u32,
    failures: u32,
    blocked_until: Option<Instant>,
}

#[cfg(test)]
const DEFAULT_LOGIN_RATE_LIMIT: zelyra_ast::ApiRateLimit = zelyra_ast::ApiRateLimit {
    requests: 5,
    window_seconds: 15 * 60,
};
#[cfg(test)]
const DEFAULT_LOGIN_BLOCK_SECONDS: u32 = 60;
const SESSION_LIFETIME: Duration = Duration::from_secs(24 * 60 * 60);
const LOGIN_THROTTLE_MAX_KEYS: usize = 4096;
pub const DEFAULT_RESET_RATE_LIMIT: zelyra_ast::ApiRateLimit = zelyra_ast::ApiRateLimit {
    requests: 3,
    window_seconds: 15 * 60,
};
pub const DEFAULT_RESET_BLOCK_SECONDS: u32 = 15 * 60;
static REQUEST_ID_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn valid_request_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn request_id(request: &Request) -> String {
    if let Some(value) = request.headers.get("x-request-id") {
        if valid_request_id(value) {
            return value.clone();
        }
    }

    new_request_id()
}

fn new_request_id() -> String {
    let mut random = [0_u8; 16];
    if OsRng.try_fill_bytes(&mut random).is_ok() {
        return hex_encode(&random);
    }

    let sequence = REQUEST_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("local-{sequence}")
}

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
    pub ui_language: UiLanguage,
    pub ui_level: UiLevel,
    pub project_theme_css: Option<String>,
    allowed_hosts: Vec<String>,
    project_ui_catalogs: ProjectUiCatalogs,
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    login_throttle: Arc<Mutex<HashMap<String, LoginThrottle>>>,
    reset_throttle: Arc<Mutex<HashMap<String, LoginThrottle>>>,
    password_reset_issue_lock: Arc<Mutex<()>>,
    password_reset_mailer: Option<Arc<crate::password_reset::PasswordResetMailer>>,
    api_throttle: Arc<Mutex<HashMap<String, ApiThrottle>>>,
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
            ui_language: UiLanguage::default(),
            ui_level: UiLevel::default(),
            project_theme_css: None,
            allowed_hosts: default_allowed_hosts(),
            project_ui_catalogs: ProjectUiCatalogs::default(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            login_throttle: Arc::new(Mutex::new(HashMap::new())),
            reset_throttle: Arc::new(Mutex::new(HashMap::new())),
            password_reset_issue_lock: Arc::new(Mutex::new(())),
            password_reset_mailer: None,
            api_throttle: Arc::new(Mutex::new(HashMap::new())),
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
            ui_language: UiLanguage::default(),
            ui_level: UiLevel::default(),
            project_theme_css: None,
            allowed_hosts: default_allowed_hosts(),
            project_ui_catalogs: ProjectUiCatalogs::default(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            login_throttle: Arc::new(Mutex::new(HashMap::new())),
            reset_throttle: Arc::new(Mutex::new(HashMap::new())),
            password_reset_issue_lock: Arc::new(Mutex::new(())),
            password_reset_mailer: None,
            api_throttle: Arc::new(Mutex::new(HashMap::new())),
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

    pub fn with_allowed_hosts(
        mut self,
        allowed_hosts: impl IntoIterator<Item = String>,
    ) -> Result<Self, HttpError> {
        let mut normalized = Vec::new();
        for host in allowed_hosts {
            let Some(host) = normalize_configured_host(&host) else {
                return Err(HttpError {
                    message: "ZELYRA_ALLOWED_HOSTS contains an invalid hostname or IP address"
                        .into(),
                });
            };
            if !normalized.contains(&host) {
                normalized.push(host);
            }
        }
        if normalized.is_empty() {
            return Err(HttpError {
                message: "at least one allowed host must be configured".into(),
            });
        }
        self.allowed_hosts = normalized;
        Ok(self)
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

    pub fn with_password_reset_mailer(
        mut self,
        mailer: crate::password_reset::PasswordResetMailer,
    ) -> Self {
        self.password_reset_mailer = Some(Arc::new(mailer));
        self
    }

    pub fn with_ui_settings(mut self, language: UiLanguage, level: UiLevel) -> Self {
        self.ui_language = language;
        self.ui_level = level;
        self
    }

    pub fn with_project_theme_css(mut self, css: Option<String>) -> Self {
        self.project_theme_css = css;
        self
    }

    pub fn with_project_ui_catalogs(mut self, catalogs: ProjectUiCatalogs) -> Self {
        self.project_ui_catalogs = catalogs;
        self
    }

    fn default_ui_context(&self, request: &Request) -> Option<DefaultUiContext> {
        let mut context = if let Some(crud) = self
            .cruds
            .iter()
            .find(|crud| crud_route_matches_path(crud, &request.path))
        {
            if crud.layout_html.is_some() {
                return None;
            }
            DefaultUiContext {
                current_path: crud.path.clone(),
                current_label: crud.title.clone(),
                navigation: Vec::new(),
            }
        } else if let Some(form) = self
            .forms
            .iter()
            .find(|form| match_path(&form.path, &request.path).is_some())
        {
            if form.layout_html.is_some() {
                return None;
            }
            let label = form
                .table
                .as_ref()
                .map(|table| localized_identifier_marker(self.ui_language, &table.name))
                .unwrap_or_else(|| localized_identifier_marker(self.ui_language, &form.form.name));
            DefaultUiContext {
                current_path: form.path.clone(),
                current_label: label,
                navigation: Vec::new(),
            }
        } else if let Some(tableview) = self
            .tableviews
            .iter()
            .find(|tableview| match_path(&tableview.path, &request.path).is_some())
        {
            DefaultUiContext {
                current_path: tableview.path.clone(),
                current_label: tableview.title.clone(),
                navigation: Vec::new(),
            }
        } else if request.path == "/login" && self.auth_route.is_some() {
            DefaultUiContext {
                current_path: request.path.clone(),
                current_label: "@i18n:auth.login_title".into(),
                navigation: Vec::new(),
            }
        } else if request.path == ACCOUNT_SESSIONS_PATH && self.auth_route.is_some() {
            DefaultUiContext {
                current_path: request.path.clone(),
                current_label: "@i18n:auth.self_sessions_title".into(),
                navigation: Vec::new(),
            }
        } else {
            let admin_path = self
                .auth_route
                .as_ref()
                .and_then(|auth| auth.admin_path.as_deref())
                .filter(|admin_path| *admin_path == request.path)?;
            DefaultUiContext {
                current_path: admin_path.to_owned(),
                current_label: "@i18n:auth.admin_title".into(),
                navigation: Vec::new(),
            }
        };

        if request.path != "/login" {
            if self.routes.iter().any(|route| route.path == "/") {
                add_navigation_link(&mut context.navigation, "/", "@i18n:shell.overview");
            }
            for crud in &self.cruds {
                add_navigation_link(
                    &mut context.navigation,
                    crud.path.clone(),
                    crud.title.clone(),
                );
            }
            for tableview in &self.tableviews {
                add_navigation_link(
                    &mut context.navigation,
                    tableview.path.clone(),
                    tableview.title.clone(),
                );
            }
            for form in &self.forms {
                if self
                    .cruds
                    .iter()
                    .any(|crud| crud_route_matches_path(crud, &form.path))
                {
                    continue;
                }
                let label = form
                    .table
                    .as_ref()
                    .map(|table| localized_identifier_marker(self.ui_language, &table.name))
                    .unwrap_or_else(|| {
                        localized_identifier_marker(self.ui_language, &form.form.name)
                    });
                add_navigation_link(&mut context.navigation, form.path.clone(), label);
            }
            if let Some(auth) = &self.auth_route {
                add_navigation_link(
                    &mut context.navigation,
                    ACCOUNT_SESSIONS_PATH,
                    "@i18n:auth.self_sessions_title",
                );
                if let Some(path) = &auth.admin_path {
                    add_navigation_link(
                        &mut context.navigation,
                        path.clone(),
                        "@i18n:auth.admin_title",
                    );
                }
            }
        }
        if context.navigation.is_empty() && request.path != "/login" {
            add_navigation_link(
                &mut context.navigation,
                context.current_path.clone(),
                context.current_label.clone(),
            );
        }
        Some(context)
    }

    pub fn dispatch(&self, request: &Request) -> Response {
        let request_id = request_id(request);
        let mut correlated_request = request.clone();
        correlated_request
            .headers
            .insert("x-request-id".into(), request_id.clone());

        let mut response = if (correlated_request.headers.contains_key("host")
            || request_has_browser_origin_or_session(&correlated_request))
            && !request_host_is_allowed(request, &self.allowed_hosts)
        {
            let title = framework_text_with_catalog(
                self.ui_language,
                "400 Bad Request",
                &self.project_ui_catalogs,
            )
            .unwrap_or_default();
            let message = framework_text_with_catalog(
                self.ui_language,
                "This request host is not allowed.",
                &self.project_ui_catalogs,
            )
            .unwrap_or_default();
            Response::html(
                400,
                format!(
                    "<h1>{}</h1><p>{}</p>",
                    html_escape(&title),
                    html_escape(&message)
                ),
            )
        } else {
            self.dispatch_inner(&correlated_request)
        };
        if response.content_type.starts_with("text/html") {
            if response.location.is_none() {
                if let Some(context) = self.default_ui_context(&correlated_request) {
                    response.body = render_default_application_shell(
                        &response.body,
                        &context,
                        self.ui_language,
                    );
                }
            }
            response.body = inject_design_system(&response.body, self.project_theme_css.is_some());
        }
        if self.ui_level == UiLevel::Learn
            && correlated_request.method == "GET"
            && response.status == 200
            && response.content_type.starts_with("text/html")
        {
            let generated_crud = self
                .cruds
                .iter()
                .any(|crud| crud_route_matches_path(crud, &request.path));
            response.body = append_learning_assistant(
                &response.body,
                &correlated_request.path,
                self.ui_language,
                generated_crud,
            );
        }
        if response.content_type.starts_with("text/html") {
            response.body = localize_html_with_catalog(
                &response.body,
                self.ui_language,
                &self.project_ui_catalogs,
            );
        }
        response
            .headers
            .retain(|(name, _)| !name.eq_ignore_ascii_case("x-request-id"));
        response.headers.push(("X-Request-ID".into(), request_id));
        response
    }

    fn dispatch_inner(&self, request: &Request) -> Response {
        if request.path == HEALTH_LIVENESS_PATH {
            return match request.method.as_str() {
                "GET" => Response::json(200, r#"{"status":"ok"}"#)
                    .with_header("Cache-Control", "no-store"),
                "HEAD" => Response::json(200, "").with_header("Cache-Control", "no-store"),
                _ => Response::empty(405)
                    .with_header("Allow", "GET, HEAD")
                    .with_header("Cache-Control", "no-store"),
            };
        }
        if request.path == PROJECT_THEME_CSS_PATH {
            return match (&self.project_theme_css, request.method.as_str()) {
                (Some(css), "GET") => {
                    Response::css(200, css.clone()).with_header("Cache-Control", "no-cache")
                }
                (Some(_), _) => Response::empty(405).with_header("Allow", "GET"),
                (None, _) => Response::empty(404),
            };
        }
        if let Some(auth_route) = &self.auth_route {
            if matches!(
                request.path.as_str(),
                "/forgot-password" | "/reset-password"
            ) {
                return password_reset::dispatch_password_reset(self, auth_route, request);
            }
            if request.path == "/login" {
                return dispatch_login(self, auth_route, request, self.database_url.as_deref());
            }
            if request.path == "/logout" {
                return dispatch_logout(self, request, self.database_url.as_deref());
            }
            if request.path == ACCOUNT_SESSIONS_PATH {
                return dispatch_account_sessions(self, request, self.database_url.as_deref());
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
        let mut api_path_matched = false;
        for api in &self.apis {
            if let Some(path_params) = match_path(&api.path, &request.path) {
                api_path_matched = true;
                if request.method == "OPTIONS" {
                    return self.api_preflight(&request.path, request);
                }
                if api.method != request.method {
                    continue;
                }
                if api_request_requires_origin_check(request)
                    && !self.api_request_origin_is_allowed(request)
                {
                    return self.apply_api_metadata(api, self.apply_api_cors(
                        request,
                        Response::json(
                            403,
                            "{\"error\":{\"code\":\"Forbidden\",\"message\":\"request origin is not allowed\"}}",
                        ),
                    ));
                }
                if let Some(retry_after) = self.api_rate_limit_retry_after(api, request) {
                    return self.apply_api_metadata(
                        api,
                        self.apply_api_cors(
                            request,
                            Response::json(
                                429,
                                "{\"error\":{\"code\":\"RateLimitExceeded\",\"message\":\"API request limit exceeded\"}}",
                            )
                            .with_header("Retry-After", retry_after.to_string()),
                        ),
                    );
                }
                if let Some(response) = authorize_api(
                    api.requires_auth,
                    &api.permissions,
                    request,
                    self,
                    self.database_url.as_deref(),
                ) {
                    return self.apply_api_metadata(api, self.apply_api_cors(…56533 tokens truncated…     PageCollectionQueryOptions {
                    search: search.as_deref(),
                    search_columns: &route.search_columns,
                    filters: &filters,
                    sort: None,
                    order: "ASC",
                    page_size: None,
                    page: 1,
                },
            )
            .map_err(RouteDataError::InvalidQuery)?;
            let count_query = format!("SELECT COUNT(*) FROM ({count_source}) AS zelyra_page_count");
            let mut count_query_params = query_params.clone();
            count_query_params.extend(count_parameters);
            let count_result = match zelyra_database::execute_mariadb_query(
                database_url,
                &count_query,
                count_query_params,
            ) {
                Ok(result) => result,
                Err(error) => {
                    eprintln!("zelyra web: page count query failed: {error}");
                    return Err(RouteDataError::Query);
                }
            };
            let Some(total) = count_result
                .rows
                .first()
                .and_then(|row| row.first())
                .and_then(|value| value.parse::<u64>().ok())
            else {
                eprintln!("zelyra web: page count query returned an invalid result");
                return Err(RouteDataError::Query);
            };
            loaded.values.insert("total".into(), total.to_string());
            let page_count = route
                .page_size
                .map(|size| total.div_ceil(u64::from(size)))
                .unwrap_or(0);
            loaded.values.insert("pages".into(), page_count.to_string());
        }
        if data.collection
            && (search.is_some()
                || sort.is_some()
                || !filters.is_empty()
                || route.page_size.is_some())
        {
            let (wrapped_query, generated_params) = page_collection_query(
                &query,
                PageCollectionQueryOptions {
                    search: search.as_deref(),
                    search_columns: &route.search_columns,
                    filters: &filters,
                    sort: sort.as_deref(),
                    order,
                    page_size: route.page_size,
                    page,
                },
            )
            .map_err(RouteDataError::InvalidQuery)?;
            query = wrapped_query;
            query_params.extend(generated_params);
        }
        let result =
            match zelyra_database::execute_mariadb_query(database_url, &query, query_params) {
                Ok(result) => result,
                Err(error) => {
                    eprintln!("zelyra web: page data query failed: {error}");
                    return Err(RouteDataError::Query);
                }
            };
        if data.collection {
            let rows = result
                .rows
                .iter()
                .map(|row| route_data_row(&result.columns, row, &data.fields))
                .collect();
            loaded.collections.insert(data.name.clone(), rows);
            continue;
        }
        let row = result.rows.first();
        if row.is_none() && !data.optional {
            return Err(RouteDataError::NotFound);
        }
        for (field, value) in route_data_row(
            &result.columns,
            row.map_or(&[][..], Vec::as_slice),
            &data.fields,
        ) {
            loaded
                .values
                .insert(format!("{}.{}", data.name, field), value);
        }
    }
    Ok(loaded)
}

fn page_number(query_values: &HashMap<String, String>) -> Result<u64, String> {
    match query_values.get("page") {
        None => Ok(1),
        Some(value) => value
            .parse::<u64>()
            .ok()
            .filter(|page| *page > 0)
            .ok_or_else(|| "page must be a positive integer".into()),
    }
}

fn page_sort_state(
    route: &Route,
    query_values: &HashMap<String, String>,
) -> Result<(Option<String>, &'static str), String> {
    let explicit_sort_input = route.query.iter().any(|input| input.name == "sort");
    let explicit_order_input = route.query.iter().any(|input| input.name == "order");
    if route.sort_columns.is_empty() {
        if (query_values.contains_key("sort") && !explicit_sort_input)
            || (query_values.contains_key("order") && !explicit_order_input)
        {
            return Err("this page does not declare sortable fields".into());
        }
        return Ok((None, "ASC"));
    }
    let sort = query_values
        .get("sort")
        .cloned()
        .unwrap_or_else(|| route.sort_columns[0].clone());
    if !route.sort_columns.iter().any(|column| column == &sort) {
        return Err(format!("unknown sort field `{sort}`"));
    }
    let order = match query_values
        .get("order")
        .map(String::as_str)
        .unwrap_or("asc")
    {
        "asc" => "ASC",
        "desc" => "DESC",
        _ => return Err("order must be `asc` or `desc`".into()),
    };
    Ok((Some(sort), order))
}

fn page_search_state(
    route: &Route,
    query_values: &HashMap<String, String>,
) -> Result<Option<String>, String> {
    let explicit_search_input = route.query.iter().any(|input| input.name == "search");
    if route.search_columns.is_empty() {
        if query_values.contains_key("search") && !explicit_search_input {
            return Err("this page does not declare searchable fields".into());
        }
        return Ok(None);
    }
    Ok(Some(
        query_values.get("search").cloned().unwrap_or_default(),
    ))
}

fn page_filter_state(
    route: &Route,
    query_values: &HashMap<String, String>,
) -> Result<Vec<PageFilterSelection>, String> {
    let mut filters = Vec::new();
    let mut seen_filter_columns = HashSet::new();
    for (name, value) in sorted_filter_query_values(query_values) {
        let Some(filter_name) = name.strip_prefix("filter_") else {
            continue;
        };
        if route.query.iter().any(|input| input.name == name) {
            continue;
        }
        if filter_name.ends_with("__operator") {
            let column = filter_name.trim_end_matches("__operator");
            let Some(filter) = route.filters.iter().find(|filter| filter.name == column) else {
                return Err(format!("unknown filter field `{column}`"));
            };
            let Some(operator) = FilterOperator::parse(value) else {
                return Err(format!("unknown filter operator for `{column}`"));
            };
            if !tableview_filter_operator_supported(filter.kind, operator) {
                return Err(format!(
                    "operator `{}` is not supported for filter `{column}`",
                    operator.key()
                ));
            }
            continue;
        }
        let (column, direct_operator) = filter_name
            .split_once("__")
            .map_or((filter_name, None), |(column, operator)| {
                (column, FilterOperator::parse(operator))
            });
        let Some(filter) = route.filters.iter().find(|filter| filter.name == column) else {
            return Err(format!("unknown filter field `{column}`"));
        };
        if filter_name.contains("__") && direct_operator.is_none() {
            return Err(format!("unknown filter operator for `{column}`"));
        }
        let operator =
            direct_operator.unwrap_or_else(|| selected_filter_operator(query_values, column));
        if !tableview_filter_operator_supported(filter.kind, operator) {
            return Err(format!(
                "operator `{}` is not supported for filter `{column}`",
                operator.key()
            ));
        }
        if !operator.needs_value() || !value.is_empty() {
            if !seen_filter_columns.insert(column) {
                return Err(format!("filter `{column}` was specified more than once"));
            }
            filters.push(PageFilterSelection {
                name: column.to_string(),
                value: value.to_string(),
                operator,
                kind: filter.kind,
            });
        }
    }
    if route.filters.is_empty()
        && sorted_filter_query_values(query_values)
            .iter()
            .any(|(name, _)| {
                name.starts_with("filter_") && !route.query.iter().any(|input| input.name == *name)
            })
    {
        return Err("this page does not declare filter fields".into());
    }
    Ok(filters)
}

struct PageCollectionQueryOptions<'a> {
    search: Option<&'a str>,
    search_columns: &'a [String],
    filters: &'a [PageFilterSelection],
    sort: Option<&'a str>,
    order: &'a str,
    page_size: Option<u32>,
    page: u64,
}

fn page_collection_query(
    source: &str,
    options: PageCollectionQueryOptions<'_>,
) -> Result<(String, Vec<(String, QueryValue)>), String> {
    let source = source.trim().trim_end_matches(';').trim();
    let mut query = format!("SELECT zelyra_page.* FROM ({source}) AS zelyra_page");
    let mut parameters = Vec::new();
    let mut conditions = Vec::new();
    if let Some(search) = options.search.filter(|search| !search.is_empty()) {
        if !options.search_columns.is_empty() {
            conditions.push(format!(
                "({})",
                options
                    .search_columns
                    .iter()
                    .map(|column| {
                        format!(
                            "CAST(zelyra_page.{} AS CHAR) LIKE CONCAT('%', :zelyra_page_search, '%')",
                            quote_identifier(column)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" OR ")
            ));
            parameters.push((
                "zelyra_page_search".into(),
                QueryValue::String(search.into()),
            ));
        }
    }
    for filter in options.filters {
        conditions.push(page_filter_condition(&filter.name, filter.operator));
        if filter.operator.needs_value() {
            let value = page_filter_query_value(filter)?;
            parameters.push((format!("zelyra_page_filter_{}", filter.name), value));
        }
    }
    if !conditions.is_empty() {
        query.push_str(" WHERE ");
        query.push_str(&conditions.join(" AND "));
    }
    if let Some(sort) = options.sort {
        query.push_str(" ORDER BY zelyra_page.");
        query.push_str(&quote_identifier(sort));
        query.push(' ');
        query.push_str(options.order);
    }
    if let Some(page_size) = options.page_size {
        query.push_str(" LIMIT :zelyra_page_limit OFFSET :zelyra_page_offset");
        parameters.push((
            "zelyra_page_limit".into(),
            QueryValue::Int(i64::from(page_size)),
        ));
        let offset = options
            .page
            .saturating_sub(1)
            .saturating_mul(u64::from(page_size));
        parameters.push((
            "zelyra_page_offset".into(),
            QueryValue::Int(offset.min(i64::MAX as u64) as i64),
        ));
    }
    Ok((query, parameters))
}

fn page_filter_condition(column: &str, operator: FilterOperator) -> String {
    let expression = format!("zelyra_page.{}", quote_identifier(column));
    let parameter = format!(":zelyra_page_filter_{column}");
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

fn page_filter_query_value(filter: &PageFilterSelection) -> Result<QueryValue, String> {
    match filter.kind {
        TableViewFilterKind::Bool => match filter.value.as_str() {
            "true" | "1" => Ok(QueryValue::Bool(true)),
            "false" | "0" => Ok(QueryValue::Bool(false)),
            _ => Err(format!("filter_{} must be true or false", filter.name)),
        },
        TableViewFilterKind::Numeric => filter
            .value
            .parse::<f64>()
            .map(QueryValue::Float)
            .map_err(|_| format!("filter_{} must be a number", filter.name)),
        TableViewFilterKind::Text | TableViewFilterKind::Other => {
            Ok(QueryValue::String(filter.value.clone()))
        }
    }
}

fn page_query_value(ty: &Type, value: &str) -> Result<QueryValue, String> {
    let ty = match ty {
        Type::Option(inner) => inner.as_ref(),
        other => other,
    };
    match ty {
        Type::Int => value
            .parse::<i64>()
            .map(QueryValue::Int)
            .map_err(|_| "query parameter must be an integer".into()),
        Type::UInt => value
            .parse::<u64>()
            .map(QueryValue::UInt)
            .map_err(|_| "query parameter must be an unsigned integer".into()),
        Type::Float | Type::Decimal => value
            .parse::<f64>()
            .map(QueryValue::Float)
            .map_err(|_| "query parameter must be a number".into()),
        Type::Bool => match value {
            "true" | "1" => Ok(QueryValue::Bool(true)),
            "false" | "0" => Ok(QueryValue::Bool(false)),
            _ => Err("query parameter must be true or false".into()),
        },
        _ => Ok(QueryValue::String(value.into())),
    }
}

fn route_data_row(
    columns: &[String],
    row: &[String],
    fields: &[String],
) -> HashMap<String, String> {
    fields
        .iter()
        .map(|field| {
            let value = columns
                .iter()
                .position(|column| column.eq_ignore_ascii_case(field))
                .and_then(|index| row.get(index))
                .cloned()
                .unwrap_or_default();
            (field.clone(), value)
        })
        .collect()
}

struct TemplateForBlock {
    start: usize,
    end: usize,
    item: String,
    collection: String,
    body: String,
}

fn next_template_for_block(template: &str) -> Option<Result<TemplateForBlock, String>> {
    let mut search_from = 0;
    while let Some(relative_start) = template[search_from..].find("for ") {
        let start = search_from + relative_start;
        let line_start = template[..start].rfind('\n').map_or(0, |index| index + 1);
        if !template[line_start..start].trim().is_empty() {
            search_from = start + 4;
            continue;
        }
        let Some(relative_open) = template[start..].find('{') else {
            return Some(Err("view `for` block is missing `{`".into()));
        };
        let open = start + relative_open;
        let header = template[start + 4..open].trim();
        let parts = header.split_whitespace().collect::<Vec<_>>();
        if parts.len() != 3
            || parts[1] != "in"
            || !view_identifier(parts[0])
            || !view_identifier(parts[2])
        {
            return Some(Err(
                "view `for` block must use `for item in collection { ... }`".into(),
            ));
        }
        let mut depth = 1;
        let mut position = open + 1;
        while position < template.len() {
            match template.as_bytes()[position] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(Ok(TemplateForBlock {
                            start,
                            end: position + 1,
                            item: parts[0].into(),
                            collection: parts[2].into(),
                            body: template[open + 1..position].into(),
                        }));
                    }
                }
                _ => {}
            }
            position += 1;
        }
        return Some(Err("view `for` block is unterminated".into()));
    }
    None
}

fn view_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.chars().enumerate().all(|(index, character)| {
            if index == 0 {
                character.is_ascii_alphabetic() || character == '_'
            } else {
                character.is_ascii_alphanumeric() || character == '_'
            }
        })
}

fn render_template(
    template: &str,
    params: &HashMap<String, String>,
    data: &LoadedRouteData,
) -> String {
    render_template_fragment(template, params, &data.values, &data.collections)
}

#[cfg(test)]
fn render_page(
    route: &Route,
    path: &str,
    params: &HashMap<String, String>,
    query_values: &HashMap<String, String>,
    data: &LoadedRouteData,
) -> String {
    render_page_with_language(route, path, params, query_values, data, UiLanguage::English)
}

fn render_page_with_language(
    route: &Route,
    path: &str,
    params: &HashMap<String, String>,
    query_values: &HashMap<String, String>,
    data: &LoadedRouteData,
    language: UiLanguage,
) -> String {
    let page = render_template(&route.html, params, data);
    let controls = render_page_query_controls(route, path, query_values, data, language);
    if controls.is_empty() {
        return page;
    }
    if let Some(body_start) = page.find("<body") {
        if let Some(relative_end) = page[body_start..].find('>') {
            let insert_at = body_start + relative_end + 1;
            let mut rendered = String::with_capacity(page.len() + controls.len());
            rendered.push_str(&page[..insert_at]);
            rendered.push_str(&controls);
            rendered.push_str(&page[insert_at..]);
            return rendered;
        }
    }
    format!("{controls}{page}")
}

fn render_page_query_controls(
    route: &Route,
    path: &str,
    query_values: &HashMap<String, String>,
    data: &LoadedRouteData,
    language: UiLanguage,
) -> String {
    let has_collection = route.data.iter().any(|data| data.collection);
    let has_controls = has_collection
        && (!route.search_columns.is_empty()
            || !route.sort_columns.is_empty()
            || !route.filters.is_empty()
            || route.page_size.is_some());
    if !has_controls {
        return String::new();
    }
    let search = query_values.get("search").map(String::as_str).unwrap_or("");
    let sort = query_values
        .get("sort")
        .map(String::as_str)
        .or_else(|| route.sort_columns.first().map(String::as_str))
        .unwrap_or("");
    let order = query_values
        .get("order")
        .map(String::as_str)
        .unwrap_or("asc");
    let page = data
        .values
        .get("page")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1);
    let mut html =
        String::from("<form method=\"get\"><fieldset class=\"zelyra-query-controls\"><legend>");
    html.push_str(&tr(language, "query.legend"));
    html.push_str("</legend>");
    if !route.search_columns.is_empty() {
        html.push_str("<label for=\"search\">");
        html.push_str(&tr(language, "query.search"));
        html.push_str("</label><input id=\"search\" name=\"search\" value=\"");
        html.push_str(&html_escape(search));
        html.push_str("\">");
    }
    if !route.sort_columns.is_empty() {
        html.push_str("<label for=\"sort\">");
        html.push_str(&tr(language, "query.sort"));
        html.push_str("</label><select id=\"sort\" name=\"sort\">");
        for column in &route.sort_columns {
            html.push_str("<option value=\"");
            html.push_str(&html_escape(column));
            html.push('"');
            if column == sort {
                html.push_str(" selected");
            }
            html.push('>');
            html.push_str(&html_escape_preserving_locale_references(
                &localized_identifier_reference(language, column),
            ));
            html.push_str("</option>");
        }
        html.push_str("</select><label for=\"order\">");
        html.push_str(&tr(language, "query.order"));
        html.push_str("</label><select id=\"order\" name=\"order\">");
        for (value, key) in [("asc", "query.ascending"), ("desc", "query.descending")] {
            html.push_str("<option value=\"");
            html.push_str(value);
            html.push('"');
            if value.eq_ignore_ascii_case(order) {
                html.push_str(" selected");
            }
            html.push('>');
            html.push_str(&tr(language, key));
            html.push_str("</option>");
        }
        html.push_str("</select>");
    }
    for filter in &route.filters {
        let selected_operator = selected_filter_operator(query_values, &filter.name);
        html.push_str("<label for=\"filter_");
        html.push_str(&html_escape(&filter.name));
        html.push_str("__operator\">");
        html.push_str(&html_escape_preserving_locale_references(&field_text(
            language,
            "query.filter_operator",
            &localized_identifier_reference(language, &filter.name),
        )));
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
            html.push_str(&operator.label(language));
            html.push_str("</option>");
        }
        html.push_str("</select><label for=\"filter_");
        html.push_str(&html_escape(&filter.name));
        html.push_str("\">");
        html.push_str(&html_escape_preserving_locale_references(&field_text(
            language,
            "query.filter_value",
            &localized_identifier_reference(language, &filter.name),
        )));
        html.push_str("</label><input id=\"filter_");
        html.push_str(&html_escape(&filter.name));
        html.push_str("\" name=\"filter_");
        html.push_str(&html_escape(&filter.name));
        html.push_str("\" value=\"");
        let value_name = format!("filter_{}", filter.name);
        html.push_str(&html_escape(selected_filter_value(
            query_values,
            &value_name,
            &filter.name,
        )));
        html.push_str("\">");
    }
    html.push_str("<button type=\"submit\">");
    html.push_str(&tr(language, "query.apply"));
    html.push_str("</button></fieldset></form>");
    if route.page_size.is_some() {
        let page_count = data
            .values
            .get("pages")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        html.push_str("<nav class=\"zelyra-pagination\">");
        if page > 1 {
            html.push_str("<a href=\"");
            html.push_str(&html_escape(&page_query_url(path, query_values, page - 1)));
            html.push_str(&format!("\">{}</a> ", tr(language, "pagination.previous")));
        }
        html.push_str("<span>");
        html.push_str(&tr(language, "pagination.page"));
        html.push(' ');
        html.push_str(&page.to_string());
        if page_count > 0 {
            html.push(' ');
            html.push_str(&tr(language, "pagination.of"));
            html.push(' ');
            html.push_str(&page_count.to_string());
        }
        html.push_str("</span>");
        if page_count > page {
            html.push_str(" <a href=\"");
            html.push_str(&html_escape(&page_query_url(path, query_values, page + 1)));
            html.push_str(&format!("\">{}</a>", tr(language, "pagination.next")));
        }
        html.push_str("</nav>");
    }
    html
}

fn page_query_url(path: &str, query_values: &HashMap<String, String>, page: u64) -> String {
    let mut url = format!("{path}?page={page}");
    for (name, value) in sorted_query_values(query_values) {
        if name == "page" || value.is_empty() {
            continue;
        }
        url.push('&');
        url.push_str(&url_encode(name));
        url.push('=');
        url.push_str(&url_encode(value));
    }
    url
}

fn render_template_fragment(
    template: &str,
    params: &HashMap<String, String>,
    values: &HashMap<String, String>,
    collections: &HashMap<String, Vec<HashMap<String, String>>>,
) -> String {
    if let Some(block) = next_template_for_block(template) {
        let Ok(block) = block else {
            return render_interpolations(template, params, values);
        };
        let mut rendered = render_interpolations(&template[..block.start], params, values);
        if let Some(rows) = collections.get(&block.collection) {
            for row in rows {
                let mut row_values = values.clone();
                for (field, value) in row {
                    row_values.insert(format!("{}.{}", block.item, field), value.clone());
                }
                rendered.push_str(&render_template_fragment(
                    &block.body,
                    params,
                    &row_values,
                    collections,
                ));
            }
        }
        rendered.push_str(&render_template_fragment(
            &template[block.end..],
            params,
            values,
            collections,
        ));
        return rendered;
    }
    render_interpolations(template, params, values)
}

fn render_interpolations(
    template: &str,
    params: &HashMap<String, String>,
    data: &HashMap<String, String>,
) -> String {
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
        if let Some(value) = params.get(name).or_else(|| data.get(name)) {
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

fn hex_decode(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Some((hex_value(pair[0])? << 4) | hex_value(pair[1])?))
        .collect()
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

pub fn route_pattern_matches_path(pattern: &str, path: &str) -> bool {
    match_path(pattern, path).is_some()
}

fn handle_connection(stream: &mut TcpStream, app: &WebApp) -> io::Result<()> {
    handle_connection_until(stream, app, Instant::now() + HTTP_EXCHANGE_TIMEOUT)
}

fn handle_connection_until(
    stream: &mut TcpStream,
    app: &WebApp,
    deadline: Instant,
) -> io::Result<()> {
    let response = match read_http_request(stream, deadline) {
        Ok(raw) => match parse_request(&raw) {
            Ok(mut request) => {
                request.remote_addr = stream.peer_addr().ok();
                let _query_deadline = zelyra_database::set_query_deadline(deadline);
                app.dispatch(&request)
            }
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
        Err(RequestReadError::Io(error))
            if matches!(error.kind(), io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock) =>
        {
            return Ok(())
        }
        Err(RequestReadError::Io(error)) => return Err(error),
    };
    let mut response = response;
    if !response
        .headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("x-request-id"))
    {
        response
            .headers
            .push(("X-Request-ID".into(), new_request_id()));
    }
    let response = response.to_http().into_bytes();
    let mut written_total = 0;
    while written_total < response.len() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(());
        }
        stream.set_write_timeout(Some(remaining))?;
        match stream.write(&response[written_total..]) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "failed to write HTTP response",
                ))
            }
            Ok(written) => {
                written_total += written;
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) =>
            {
                return Ok(())
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

#[cfg(test)]
mod fuzz_regressions;
#[cfg(test)]
mod tests;
