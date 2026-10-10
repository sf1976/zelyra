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
mod mfa;
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
pub const ACCOUNT_SECURITY_PATH: &str = "/account/security";
pub const LOGIN_MFA_PATH: &str = "/login/mfa";
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
    /// Tenant key for generated CRUD forms; resolved from the authenticated request.
    pub tenant_column: Option<String>,
    pub tenant_membership_table: Option<String>,
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
    pub tenant_column: Option<String>,
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
    pub membership_table: Option<String>,
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
    pub mfa_table: Option<String>,
    pub mfa_recovery_table: Option<String>,
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
struct TenantContext {
    user_id: i64,
    tenant_id: i64,
    membership_table: String,
}

struct CrudMutationContext<'a> {
    database_url: Option<&'a str>,
    audit_table: Option<&'a str>,
    audit_chain: bool,
    actor_user_id: Option<i64>,
    tenant_context: Option<&'a TenantContext>,
}

struct FormExecutionContext<'a> {
    actor_user_id: Option<i64>,
    expected_snapshot: Option<&'a [u8]>,
    tenant_context: Option<&'a TenantContext>,
}

fn tenant_selector(request: &Request) -> Result<Option<String>, u16> {
    let header_tenant = request
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("x-zelyra-tenant"))
        .map(|(_, value)| value.clone());
    let query_string = request
        .target
        .split_once('?')
        .map_or("", |(_, query)| query);
    let query_tenant_count = query_string
        .split('&')
        .filter(|pair| {
            pair.split_once('=')
                .and_then(|(name, _)| percent_decode(name).ok())
                .is_some_and(|name| name == "tenant_id")
        })
        .count();
    if query_tenant_count > 1 {
        return Err(400);
    }
    let query_tenant = parse_urlencoded(query_string)
        .map_err(|_| 400u16)?
        .remove("tenant_id");
    if header_tenant.is_some()
        && query_tenant.is_some()
        && header_tenant.as_deref() != query_tenant.as_deref()
    {
        return Err(400);
    }
    Ok(header_tenant.or(query_tenant))
}

fn resolve_tenant_context(
    app: &WebApp,
    tenant_column: Option<&str>,
    request: &Request,
    database_url: Option<&str>,
) -> Result<Option<TenantContext>, u16> {
    if tenant_column.is_none() {
        return Ok(None);
    }
    let Some(auth) = app.auth_route.as_ref() else {
        return Err(403);
    };
    let Some(membership_table) = auth.membership_table.as_deref() else {
        return Err(403);
    };
    let Some(database_url) = database_url else {
        return Err(503);
    };
    let Some(user_id) =
        session_from_request(app, request, Some(database_url)).and_then(|session| session.user_id)
    else {
        return Err(403);
    };
    let selected_tenant = tenant_selector(request)?;
    let selected_tenant_id = match selected_tenant {
        Some(value) => match value.trim().parse::<i64>() {
            Ok(id) if id > 0 => Some(id),
            _ => return Err(400),
        },
        None => None,
    };
    let mut query = format!(
        "SELECT tenant_id FROM {} WHERE user_id = :zelyra_user_id AND active = true",
        quote_identifier(membership_table),
    );
    let mut params = vec![("zelyra_user_id".into(), QueryValue::Int(user_id))];
    if let Some(tenant_id) = selected_tenant_id {
        query.push_str(" AND tenant_id = :zelyra_selected_tenant_id");
        params.push((
            "zelyra_selected_tenant_id".into(),
            QueryValue::Int(tenant_id),
        ));
    }
    query.push_str(" ORDER BY tenant_id LIMIT 2");
    let result =
        zelyra_database::execute_mariadb_query(database_url, &query, params).map_err(|error| {
            eprintln!("zelyra web: tenant membership lookup failed: {error}");
            503u16
        })?;
    let tenant_ids = result
        .rows
        .iter()
        .filter_map(|row| row.first()?.parse::<i64>().ok())
        .filter(|tenant_id| *tenant_id > 0)
        .collect::<Vec<_>>();
    let tenant_id = match (selected_tenant_id, tenant_ids.as_slice()) {
        (Some(selected), [matched]) if selected == *matched => selected,
        (None, [only]) => *only,
        _ => return Err(403),
    };
    Ok(Some(TenantContext {
        user_id,
        tenant_id,
        membership_table: membership_table.to_owned(),
    }))
}

fn tenant_resolution_error(status: u16) -> Response {
    let title = match status {
        400 => "400 Bad Request",
        403 => "403 Forbidden",
        _ => "503 Service Unavailable",
    };
    Response::html(status, format!("<h1>{title}</h1>"))
}

fn tenant_membership_predicate(context: &TenantContext) -> String {
    format!(
        "EXISTS (SELECT 1 FROM {} AS zelyra_membership WHERE zelyra_membership.user_id = :zelyra_user_id AND zelyra_membership.tenant_id = :zelyra_tenant_id AND zelyra_membership.active = true)",
        quote_identifier(&context.membership_table)
    )
}

fn tenant_query_params(context: &TenantContext) -> Vec<(String, QueryValue)> {
    vec![
        ("zelyra_user_id".into(), QueryValue::Int(context.user_id)),
        (
            "zelyra_tenant_id".into(),
            QueryValue::Int(context.tenant_id),
        ),
    ]
}

fn append_tenant_selector(url: &str, tenant_id: i64) -> String {
    if !url.starts_with('/') || url.contains("tenant_id=") {
        return url.to_owned();
    }
    let separator = if url.contains('?') { "&" } else { "?" };
    format!("{url}{separator}tenant_id={tenant_id}")
}

fn preserve_tenant_selector_in_html(html: &str, tenant_id: i64) -> String {
    let mut output = html.to_owned();
    for attribute in ["href=\"", "action=\""] {
        let mut search_from = 0;
        while let Some(relative_start) = output[search_from..].find(attribute) {
            let value_start = search_from + relative_start + attribute.len();
            let Some(relative_end) = output[value_start..].find('"') else {
                break;
            };
            let value_end = value_start + relative_end;
            let current = &output[value_start..value_end];
            let updated = append_tenant_selector(current, tenant_id);
            output.replace_range(value_start..value_end, &updated);
            search_from = value_start + updated.len() + 1;
        }
    }
    output
}

fn tenant_cache_policy(mut response: Response, tenant_context: Option<&TenantContext>) -> Response {
    if let Some(tenant_context) = tenant_context {
        response.body = preserve_tenant_selector_in_html(&response.body, tenant_context.tenant_id);
        if let Some(location) = response.location.as_mut() {
            *location = append_tenant_selector(location, tenant_context.tenant_id);
        }
        response.with_header("Cache-Control", "private, no-store")
    } else {
        response
    }
}

fn execute_crud_mutation(
    database_url: &str,
    queries: &[zelyra_database::Query],
    tenant_context: Option<&TenantContext>,
    scoped_record_check: Option<&zelyra_database::Query>,
) -> Result<(), zelyra_database::DatabaseError> {
    let Some(tenant_context) = tenant_context else {
        zelyra_database::execute_mariadb_queries(database_url, queries, true)?;
        return Ok(());
    };
    let Some((mutation, audit)) = queries.split_first() else {
        return Err(zelyra_database::DatabaseError {
            message: "CRUD mutation transaction has no statement".into(),
        });
    };
    zelyra_database::with_mariadb_transaction(database_url, |transaction| {
        let membership_check = zelyra_database::Query {
            sql: format!(
                "SELECT tenant_id FROM {} WHERE user_id = :zelyra_user_id AND tenant_id = :zelyra_tenant_id AND active = true FOR UPDATE",
                quote_identifier(&tenant_context.membership_table)
            ),
            params: tenant_query_params(tenant_context),
        };
        if transaction.execute(&membership_check)?.rows.is_empty() {
            return Err(zelyra_database::DatabaseError {
                message: "ZELYRA_TENANT_DENIED: membership is no longer active".into(),
            });
        }
        let Some(scoped_record_check) = scoped_record_check else {
            return Err(zelyra_database::DatabaseError {
                message: "tenant-scoped CRUD mutation has no record check".into(),
            });
        };
        if transaction.execute(scoped_record_check)?.rows.is_empty() {
            return Err(zelyra_database::DatabaseError {
                message: "ZELYRA_TENANT_DENIED: record is not available in this tenant".into(),
            });
        }
        transaction.execute(mutation)?;
        for query in audit {
            transaction.execute(query)?;
        }
        Ok(())
    })
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
            if request.path == LOGIN_MFA_PATH && auth_route.mfa_table.is_some() {
                return dispatch_login_mfa(self, auth_route, request, self.database_url.as_deref());
            }
            if request.path == "/logout" {
                return dispatch_logout(self, request, self.database_url.as_deref());
            }
            if request.path == ACCOUNT_SESSIONS_PATH {
                return dispatch_account_sessions(self, request, self.database_url.as_deref());
            }
            if request.path == ACCOUNT_SECURITY_PATH && auth_route.mfa_table.is_some() {
                return dispatch_account_security(
                    self,
                    auth_route,
                    request,
                    self.database_url.as_deref(),
                );
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
                    return self.apply_api_metadata(api, self.apply_api_cors(request, response));
                }
                return self.apply_api_metadata(
                    api,
                    self.apply_api_cors(request, (api.handler)(request, &path_params)),
                );
            }
        }
        if api_path_matched {
            return self.apply_api_cors(
                request,
                Response::json(
                    405,
                    "{\"error\":{\"code\":\"MethodNotAllowed\",\"message\":\"method not allowed\"}}",
                )
                .with_header("Allow", self.api_allowed_methods(&request.path)),
            );
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
                let tenant_context = match resolve_tenant_context(
                    self,
                    form.tenant_column.as_deref(),
                    request,
                    self.database_url.as_deref(),
                ) {
                    Ok(context) => context,
                    Err(status) => return tenant_resolution_error(status),
                };
                return tenant_cache_policy(
                    apply_generated_layout(
                        dispatch_form_with_language(
                            form,
                            request,
                            &path_params,
                            self.database_url.as_deref(),
                            tenant_context
                                .as_ref()
                                .map(|context| context.user_id)
                                .or_else(|| {
                                    session_from_request(
                                        self,
                                        request,
                                        self.database_url.as_deref(),
                                    )
                                    .and_then(|session| session.user_id)
                                }),
                            tenant_context.as_ref(),
                            self.ui_language,
                        ),
                        form.layout_html.as_deref(),
                    ),
                    tenant_context.as_ref(),
                );
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
                let tenant_context = match resolve_tenant_context(
                    self,
                    crud.tenant_column.as_deref(),
                    request,
                    self.database_url.as_deref(),
                ) {
                    Ok(context) => context,
                    Err(status) => return tenant_resolution_error(status),
                };
                return tenant_cache_policy(
                    apply_generated_layout(
                        dispatch_crud(
                            crud,
                            request,
                            self.database_url.as_deref(),
                            crud_ui_actions(crud, request, self),
                            tenant_context.as_ref(),
                            self.ui_language,
                        ),
                        crud.layout_html.as_deref(),
                    ),
                    tenant_context.as_ref(),
                );
            }
            for action in &crud.actions {
                if let Some(path_params) = match_path(&action.form.path, &request.path) {
                    if self.database_capability_granted == Some(false) {
                        return database_capability_denied();
                    }
                    let (requires_auth, permissions) = form_authorization(&action.form);
                    if let Some(response) = authorize(
                        requires_auth,
                        &permissions,
                        request,
                        self,
                        self.database_url.as_deref(),
                    ) {
                        return response;
                    }
                    let tenant_context = match resolve_tenant_context(
                        self,
                        action.form.tenant_column.as_deref(),
                        request,
                        self.database_url.as_deref(),
                    ) {
                        Ok(context) => context,
                        Err(status) => return tenant_resolution_error(status),
                    };
                    return tenant_cache_policy(
                        apply_generated_layout(
                            dispatch_form_with_language(
                                &action.form,
                                request,
                                &path_params,
                                self.database_url.as_deref(),
                                tenant_context
                                    .as_ref()
                                    .map(|context| context.user_id)
                                    .or_else(|| {
                                        session_from_request(
                                            self,
                                            request,
                                            self.database_url.as_deref(),
                                        )
                                        .and_then(|session| session.user_id)
                                    }),
                                tenant_context.as_ref(),
                                self.ui_language,
                            ),
                            action.form.layout_html.as_deref(),
                        ),
                        tenant_context.as_ref(),
                    );
                }
            }
            let restore_path = format!("{}/{{id}}/restore", crud.path.trim_end_matches('/'));
            if let Some(path_params) = match_path(&restore_path, &request.path) {
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
                let tenant_context = match resolve_tenant_context(
                    self,
                    crud.tenant_column.as_deref(),
                    request,
                    self.database_url.as_deref(),
                ) {
                    Ok(context) => context,
                    Err(status) => return tenant_resolution_error(status),
                };
                return tenant_cache_policy(
                    apply_generated_layout(
                        dispatch_crud_restore(
                            crud,
                            request,
                            &path_params,
                            CrudMutationContext {
                                database_url: self.database_url.as_deref(),
                                audit_table: self
                                    .auth_route
                                    .as_ref()
                                    .and_then(|auth| auth.audit_table.as_deref()),
                                audit_chain: self
                                    .auth_route
                                    .as_ref()
                                    .is_some_and(|auth| auth.audit_chain),
                                actor_user_id: session_from_request(
                                    self,
                                    request,
                                    self.database_url.as_deref(),
                                )
                                .and_then(|session| session.user_id),
                                tenant_context: tenant_context.as_ref(),
                            },
                        ),
                        crud.layout_html.as_deref(),
                    ),
                    tenant_context.as_ref(),
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
                let tenant_context = match resolve_tenant_context(
                    self,
                    crud.tenant_column.as_deref(),
                    request,
                    self.database_url.as_deref(),
                ) {
                    Ok(context) => context,
                    Err(status) => return tenant_resolution_error(status),
                };
                return tenant_cache_policy(
                    apply_generated_layout(
                        dispatch_crud_delete(
                            crud,
                            request,
                            &path_params,
                            CrudMutationContext {
                                database_url: self.database_url.as_deref(),
                                audit_table: self
                                    .auth_route
                                    .as_ref()
                                    .and_then(|auth| auth.audit_table.as_deref()),
                                audit_chain: self
                                    .auth_route
                                    .as_ref()
                                    .is_some_and(|auth| auth.audit_chain),
                                actor_user_id: session_from_request(
                                    self,
                                    request,
                                    self.database_url.as_deref(),
                                )
                                .and_then(|session| session.user_id),
                                tenant_context: tenant_context.as_ref(),
                            },
                        ),
                        crud.layout_html.as_deref(),
                    ),
                    tenant_context.as_ref(),
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
                let tenant_context = match resolve_tenant_context(
                    self,
                    crud.tenant_column.as_deref(),
                    request,
                    self.database_url.as_deref(),
                ) {
                    Ok(context) => context,
                    Err(status) => return tenant_resolution_error(status),
                };
                return tenant_cache_policy(
                    apply_generated_layout(
                        dispatch_crud_detail(
                            crud,
                            request,
                            &path_params,
                            self.database_url.as_deref(),
                            crud_ui_actions(crud, request, self),
                            tenant_context.as_ref(),
                            self.ui_language,
                        ),
                        crud.layout_html.as_deref(),
                    ),
                    tenant_context.as_ref(),
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
                return dispatch_tableview(
                    tableview,
                    request,
                    self.database_url.as_deref(),
                    self.ui_language,
                );
            }
        }
        for route in &self.routes {
            if match_path(&route.path, &request.path).is_some() {
                if self.database_capability_granted == Some(false) && !route.data.is_empty() {
                    return database_capability_denied();
                }
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
        Router::new(self.routes.clone()).dispatch_with_database_language_and_catalogs(
            &request.method,
            &request.target,
            self.database_url.as_deref(),
            self.ui_language,
            &self.project_ui_catalogs,
        )
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

    fn apply_api_metadata(&self, api: &ApiRoute, mut response: Response) -> Response {
        if let Some(version) = &api.version {
            response = response.with_header("X-Zelyra-API-Version", version.clone());
        }
        if api.deprecated {
            response = response.with_header("X-Zelyra-API-Deprecated", "true");
        }
        response
    }

    fn api_rate_limit_retry_after(&self, api: &ApiRoute, request: &Request) -> Option<u64> {
        let limit = api.rate_limit?;
        let window = Duration::from_secs(u64::from(limit.window_seconds));
        let now = Instant::now();
        let client = request
            .remote_addr
            .map(|address| address.ip().to_string())
            .unwrap_or_else(|| "in-process".into());
        let key = format!("{} {} {client}", api.method, api.path);
        let Ok(mut clients) = self.api_throttle.lock() else {
            return Some(u64::from(limit.window_seconds));
        };

        clients.retain(|_, throttle| {
            now.duration_since(throttle.window_started)
                < Duration::from_secs(u64::from(throttle.window_seconds))
        });

        if let Some(throttle) = clients.get_mut(&key) {
            if now.duration_since(throttle.window_started) >= window {
                throttle.window_started = now;
                throttle.requests = 1;
                throttle.window_seconds = limit.window_seconds;
                return None;
            }
            if throttle.requests >= limit.requests {
                let remaining = window.saturating_sub(now.duration_since(throttle.window_started));
                return Some(remaining.as_secs().max(1));
            }
            throttle.requests += 1;
            return None;
        }

        if clients.len() >= API_THROTTLE_MAX_CLIENTS {
            return Some(u64::from(limit.window_seconds));
        }
        clients.insert(
            key,
            ApiThrottle {
                window_started: now,
                requests: 1,
                window_seconds: limit.window_seconds,
            },
        );
        None
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

    fn api_request_origin_is_allowed(&self, request: &Request) -> bool {
        if request_origin_matches_host(request) {
            return true;
        }
        request
            .headers
            .get("origin")
            .zip(self.cors_policy.as_ref())
            .is_some_and(|(origin, policy)| {
                policy.allows(origin)
                    && (cookie_value(request, "zelyra_session").is_none()
                        || policy.allow_credentials)
            })
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

fn default_allowed_hosts() -> Vec<String> {
    vec!["localhost".into(), "127.0.0.1".into(), "[::1]".into()]
}

fn request_has_browser_origin_or_session(request: &Request) -> bool {
    request.headers.contains_key("origin")
        || request.headers.contains_key("referer")
        || cookie_value(request, "zelyra_session").is_some()
}

fn request_host_is_allowed(request: &Request, allowed_hosts: &[String]) -> bool {
    request
        .headers
        .get("host")
        .and_then(|authority| host_name_from_authority(authority))
        .is_some_and(|host| allowed_hosts.iter().any(|allowed| allowed == &host))
}

fn normalize_configured_host(value: &str) -> Option<String> {
    if value.starts_with('[') && value.ends_with(']') {
        let address = value[1..value.len() - 1]
            .parse::<std::net::Ipv6Addr>()
            .ok()?;
        return Some(format!("[{address}]"));
    }
    let value = value.strip_suffix('.').unwrap_or(value);
    if value.is_empty() || value.len() > 253 {
        return None;
    }
    for label in value.split('.') {
        if label.is_empty()
            || label.len() > 63
            || !label.as_bytes()[0].is_ascii_alphanumeric()
            || !label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
            || !label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return None;
        }
    }
    Some(value.to_ascii_lowercase())
}

fn host_name_from_authority(authority: &str) -> Option<String> {
    let hostname = if authority.starts_with('[') {
        let bracket_end = authority.find(']')?;
        let hostname = &authority[..=bracket_end];
        let suffix = &authority[bracket_end + 1..];
        if !suffix.is_empty() {
            let port = suffix.strip_prefix(':')?.parse::<u16>().ok()?;
            if port == 0 {
                return None;
            }
        }
        hostname
    } else {
        match authority.matches(':').count() {
            0 => authority,
            1 => {
                let (hostname, port) = authority.rsplit_once(':')?;
                port.parse::<u16>().ok().filter(|port| *port != 0)?;
                hostname
            }
            _ => return None,
        }
    };
    normalize_configured_host(hostname)
}

fn api_request_requires_origin_check(request: &Request) -> bool {
    request.method != "OPTIONS"
        && (request.headers.contains_key("origin")
            || request.headers.contains_key("referer")
            || cookie_value(request, "zelyra_session").is_some())
}

fn verify_csrf_request(request: &Request, csrf: &CsrfProtection, candidate: Option<&str>) -> bool {
    csrf.verify(candidate) && request_origin_matches_host(request)
}

fn request_origin_matches_host(request: &Request) -> bool {
    let Some(host) = request.headers.get("host") else {
        return false;
    };
    let scheme = match request.headers.get("x-forwarded-proto") {
        Some(value) => value.split(',').next().unwrap_or_default().trim(),
        None => "http",
    };
    if !matches!(scheme, "http" | "https") {
        return false;
    }
    let Some(expected_authority) = normalize_authority(host, scheme) else {
        return false;
    };

    let mut saw_origin = false;
    for (header, allow_path) in [("origin", false), ("referer", true)] {
        let Some(value) = request.headers.get(header) else {
            continue;
        };
        saw_origin = true;
        let Some((actual_scheme, actual_authority)) = parse_request_origin(value, allow_path)
        else {
            return false;
        };
        if actual_scheme != scheme || actual_authority != expected_authority {
            return false;
        }
    }
    saw_origin
}

fn parse_request_origin(value: &str, allow_path: bool) -> Option<(&str, String)> {
    let (scheme, remainder) = value.split_once("://")?;
    if !matches!(scheme, "http" | "https") || value.trim() != value {
        return None;
    }
    let authority_end = remainder.find(['/', '?', '#']).unwrap_or(remainder.len());
    let authority = &remainder[..authority_end];
    if authority.is_empty() || (!allow_path && authority_end != remainder.len()) {
        return None;
    }
    Some((scheme, normalize_authority(authority, scheme)?))
}

fn normalize_authority(authority: &str, scheme: &str) -> Option<String> {
    if authority.is_empty() || authority.contains(['@', '/', '?', '#', '\\', ' ', '\t', '\r', '\n'])
    {
        return None;
    }
    let (host, port) = if let Some(bracket_end) = authority
        .strip_prefix('[')
        .and_then(|_| authority.find(']'))
    {
        let host = &authority[..=bracket_end];
        let suffix = &authority[bracket_end + 1..];
        let address = host[1..host.len() - 1].parse::<std::net::Ipv6Addr>().ok()?;
        let port = if suffix.is_empty() {
            None
        } else {
            let port = suffix.strip_prefix(':')?.parse::<u16>().ok()?;
            if port == 0 {
                return None;
            }
            Some(port)
        };
        (format!("[{address}]"), port)
    } else {
        if authority.matches(':').count() > 1 {
            return None;
        }
        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) => (host, Some(port.parse::<u16>().ok()?)),
            None => (authority, None),
        };
        if host.is_empty()
            || !host
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
        {
            return None;
        }
        (host.to_ascii_lowercase(), port)
    };
    let default_port = match scheme {
        "http" => Some(80),
        "https" => Some(443),
        _ => return None,
    };
    Some(match port {
        Some(port) if Some(port) != default_port => format!("{host}:{port}"),
        _ => host,
    })
}

fn secure_cookie_attribute(request: &Request) -> &'static str {
    if request
        .headers
        .get("x-forwarded-proto")
        .and_then(|value| value.split(',').next())
        .is_some_and(|scheme| scheme.trim() == "https")
    {
        "; Secure"
    } else {
        ""
    }
}

fn dispatch_login(
    app: &WebApp,
    auth: &AuthRoute,
    request: &Request,
    database_url: Option<&str>,
) -> Response {
    match request.method.as_str() {
        "GET" => Response::html(200, render_login(auth, app.ui_language)),
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
            if !verify_csrf_request(
                request,
                &auth.csrf,
                input.get("_zelyra_csrf").map(String::as_str),
            ) {
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
            if login_is_blocked_with_policy(app, &throttle_key, auth.login_rate_limit) {
                return Response::html(429, "<h1>429 Too Many Requests</h1><p>Too many failed login attempts. Try again later.</p>")
                    .with_header("Retry-After", auth.login_block_seconds.to_string());
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
                record_login_failure_with_policy(
                    app,
                    throttle_key,
                    auth.login_rate_limit,
                    auth.login_block_seconds,
                );
                return Response::html(401, "<h1>401 Unauthorized</h1><p>Invalid credentials.</p>");
            }
            clear_login_failures(app, &throttle_key);
            let Some(user_id) = login_user_id else {
                return Response::html(500, "<h1>500 Internal Server Error</h1>");
            };
            if let Err(error) = rotate_existing_session(app, auth, request, Some(database_url)) {
                eprintln!("zelyra web: session rotation failed: {error}");
                return Response::html(500, "<h1>500 Internal Server Error</h1>");
            }
            let Ok(session_id) = CsrfProtection::generate().map(|csrf| csrf.token().to_owned())
            else {
                return Response::html(500, "<h1>500 Internal Server Error</h1>");
            };
            let device = request_device_metadata(request);
            let mfa_pending = if let Some(session_table) = &auth.session_table {
                let has_device_column = auth_session_has_column(auth, "device_label");
                let has_mfa_column = auth.mfa_table.is_some();
                match zelyra_database::with_mariadb_transaction(database_url, |transaction| {
                    let mfa_pending = if let Some(mfa_table) = auth.mfa_table.as_deref() {
                        let factor = transaction.execute(&zelyra_database::Query {
                            sql: format!(
                                "SELECT enabled_at IS NOT NULL FROM {} WHERE user_id = :user_id LIMIT 1 FOR UPDATE",
                                quote_identifier(mfa_table)
                            ),
                            params: vec![("user_id".into(), QueryValue::Int(user_id))],
                        })?;
                        factor
                            .rows
                            .first()
                            .and_then(|row| row.first())
                            .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
                    } else {
                        false
                    };
                    let query = zelyra_database::Query {
                        sql: format!(
                            "INSERT INTO {} (user_id, token_hash, expires_at{}{} ) VALUES (:user_id, :token_hash, DATE_ADD(NOW(), INTERVAL 1 DAY){}{})",
                            quote_identifier(session_table),
                            if has_device_column { ", device_label" } else { "" },
                            if has_mfa_column { ", mfa_verified" } else { "" },
                            if has_device_column { ", :device_label" } else { "" },
                            if has_mfa_column { ", :mfa_verified" } else { "" },
                        ),
                        params: vec![
                            ("user_id".into(), QueryValue::Int(user_id)),
                            (
                                "token_hash".into(),
                                QueryValue::String(session_token_hash(&session_id)),
                            ),
                        ]
                        .into_iter()
                        .chain(has_device_column.then(|| (
                            "device_label".into(),
                            device
                                .clone()
                                .map(QueryValue::String)
                                .unwrap_or(QueryValue::Null),
                        )))
                        .chain(has_mfa_column.then_some((
                            "mfa_verified".into(),
                            QueryValue::Bool(!mfa_pending),
                        )))
                        .collect(),
                    };
                    transaction.execute(&query)?;
                    if let Some(audit_table) = auth.audit_table.as_deref() {
                        for query in audit_insert_queries(
                            audit_table,
                            auth.audit_chain,
                            Some(user_id),
                            if mfa_pending {
                                "auth.login_mfa_pending"
                            } else {
                                "auth.login"
                            },
                            Some(user_id),
                            "",
                        ) {
                            transaction.execute(&query)?;
                        }
                    }
                    Ok(mfa_pending)
                }) {
                    Ok(mfa_pending) => mfa_pending,
                    Err(error) => {
                        eprintln!("zelyra web: session creation failed: {error}");
                        return Response::html(500, "<h1>500 Internal Server Error</h1>");
                    }
                }
            } else {
                if auth.mfa_table.is_some() {
                    return Response::html(503, "<h1>503 Service Unavailable</h1>");
                }
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
                        expires_at: Instant::now() + SESSION_LIFETIME,
                        device,
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
                false
            };
            Response::redirect(if mfa_pending { "/login/mfa" } else { "/" }).with_header(
                "Set-Cookie",
                format!(
                    "zelyra_session={session_id}; Path=/; HttpOnly; SameSite=Lax{}",
                    secure_cookie_attribute(request)
                ),
            )
        }
        _ => Response::html(405, "<h1>405 Method Not Allowed</h1>"),
    }
}

fn dispatch_account_security(
    app: &WebApp,
    auth: &AuthRoute,
    request: &Request,
    database_url: Option<&str>,
) -> Response {
    if app.database_capability_granted == Some(false) {
        return database_capability_denied();
    }
    let (Some(database_url), Some(mfa_table), Some(recovery_table), Some(session_table)) = (
        database_url,
        auth.mfa_table.as_deref(),
        auth.mfa_recovery_table.as_deref(),
        auth.session_table.as_deref(),
    ) else {
        return Response::html(503, "<h1>503 Service Unavailable</h1>")
            .with_header("Cache-Control", "no-store");
    };
    let Some(session) = session_from_request(app, request, Some(database_url)) else {
        return Response::redirect("/login");
    };
    let Some(user_id) = session.user_id.filter(|id| *id > 0) else {
        return Response::redirect("/login");
    };
    match request.method.as_str() {
        "GET" => {
            let result = zelyra_database::execute_mariadb_query(
                database_url,
                &format!(
                    "SELECT enabled_at IS NOT NULL, enrollment_expires_at > CURRENT_TIMESTAMP FROM {} WHERE user_id = :user_id LIMIT 1",
                    quote_identifier(mfa_table)
                ),
                vec![("user_id".into(), QueryValue::Int(user_id))],
            );
            let row = match result {
                Ok(result) => result.rows.into_iter().next(),
                Err(error) => {
                    eprintln!("zelyra web: MFA settings lookup failed: {error}");
                    return Response::html(500, "<h1>500 Internal Server Error</h1>")
                        .with_header("Cache-Control", "no-store");
                }
            };
            let enabled = row
                .as_ref()
                .and_then(|row| row.first())
                .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));
            let pending = row
                .as_ref()
                .and_then(|row| row.get(1))
                .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));
            let status = if enabled {
                "Multi-factor authentication is enabled."
            } else if pending {
                "MFA setup is pending. Start again to replace the pending secret."
            } else {
                "Multi-factor authentication is not enabled."
            };
            let actions = if enabled {
                format!(
                    "<section><h2>Replace recovery codes</h2><form method=\"post\" action=\"{ACCOUNT_SECURITY_PATH}\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{}\"><input type=\"hidden\" name=\"action\" value=\"rotate_recovery\"><label for=\"rotate-password\">Confirm password</label><input id=\"rotate-password\" name=\"password\" type=\"password\" autocomplete=\"current-password\" required><label for=\"rotate-code\">Current authenticator code</label><input id=\"rotate-code\" name=\"code\" inputmode=\"numeric\" pattern=\"[0-9]{{6}}\" required><button type=\"submit\">Replace recovery codes</button></form></section><section><h2>Turn off MFA</h2><form method=\"post\" action=\"{ACCOUNT_SECURITY_PATH}\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{}\"><input type=\"hidden\" name=\"action\" value=\"disable\"><label for=\"disable-password\">Confirm password</label><input id=\"disable-password\" name=\"password\" type=\"password\" autocomplete=\"current-password\" required><label for=\"disable-code\">Current authenticator code</label><input id=\"disable-code\" name=\"code\" inputmode=\"numeric\" pattern=\"[0-9]{{6}}\" required><button type=\"submit\">Turn off MFA</button></form></section>",
                    html_escape(auth.csrf.token()),
                    html_escape(auth.csrf.token())
                )
            } else {
                format!(
                    "<form method=\"post\" action=\"{ACCOUNT_SECURITY_PATH}\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{}\"><input type=\"hidden\" name=\"action\" value=\"begin\"><label for=\"password\">Confirm password</label><input id=\"password\" name=\"password\" type=\"password\" autocomplete=\"current-password\" required><button type=\"submit\">Set up authenticator</button></form>",
                    html_escape(auth.csrf.token())
                )
            };
            let html = format!("<main><h1>Account security</h1><p>{status}</p>{actions}</main>");
            Response::html(200, html).with_header("Cache-Control", "no-store")
        }
        "POST" => {
            let input = match parse_urlencoded(&request.body) {
                Ok(input) => input,
                Err(_) => {
                    return Response::html(400, "<h1>400 Bad Request</h1>")
                        .with_header("Cache-Control", "no-store")
                }
            };
            if !verify_csrf_request(
                request,
                &auth.csrf,
                input.get("_zelyra_csrf").map(String::as_str),
            ) {
                return Response::html(403, "<h1>403 Forbidden</h1>")
                    .with_header("Cache-Control", "no-store");
            }
            let password = input
                .get("password")
                .map(String::as_str)
                .unwrap_or_default();
            if !verify_current_account_password(auth, database_url, user_id, password) {
                return Response::html(
                    401,
                    "<h1>401 Unauthorized</h1><p>Password confirmation failed.</p>",
                )
                .with_header("Cache-Control", "no-store");
            }
            match input.get("action").map(String::as_str) {
                Some("begin") => begin_mfa_enrollment(auth, database_url, mfa_table, user_id),
                Some("confirm") => {
                    let current_token_hash = cookie_value(request, "zelyra_session")
                        .map(|session_id| session_token_hash(&session_id))
                        .unwrap_or_default();
                    confirm_mfa_enrollment(
                        MfaAccountContext {
                            auth,
                            database_url,
                            mfa_table,
                            recovery_table,
                            session_table,
                            user_id,
                            current_token_hash: &current_token_hash,
                        },
                        input.get("code").map(String::as_str).unwrap_or_default(),
                    )
                }
                Some("rotate_recovery") | Some("disable") => {
                    let current_token_hash = cookie_value(request, "zelyra_session")
                        .map(|session_id| session_token_hash(&session_id))
                        .unwrap_or_default();
                    update_mfa_security(
                        MfaAccountContext {
                            auth,
                            database_url,
                            mfa_table,
                            recovery_table,
                            session_table,
                            user_id,
                            current_token_hash: &current_token_hash,
                        },
                        input.get("code").map(String::as_str).unwrap_or_default(),
                        input.get("action").map(String::as_str) == Some("disable"),
                    )
                }
                _ => Response::html(422, "<h1>422 Unprocessable Entity</h1>")
                    .with_header("Cache-Control", "no-store"),
            }
        }
        _ => Response::empty(405)
            .with_header("Allow", "GET, POST")
            .with_header("Cache-Control", "no-store"),
    }
}

fn verify_current_account_password(
    auth: &AuthRoute,
    database_url: &str,
    user_id: i64,
    password: &str,
) -> bool {
    if password.is_empty() {
        return false;
    }
    let Some(user_table) = auth
        .schema
        .tables
        .iter()
        .find(|table| table.name == auth.table)
    else {
        return false;
    };
    let active_clause = if user_table
        .columns
        .iter()
        .any(|column| column.name == "active")
    {
        " AND active = true"
    } else {
        ""
    };
    let Ok(result) = zelyra_database::execute_mariadb_query(
        database_url,
        &format!(
            "SELECT password_hash FROM {} WHERE id = :user_id{active_clause} LIMIT 1",
            quote_identifier(&auth.table)
        ),
        vec![("user_id".into(), QueryValue::Int(user_id))],
    ) else {
        return false;
    };
    result.rows.first().is_some_and(|row| {
        row.first()
            .and_then(|hash| PasswordHash::new(hash).ok())
            .is_some_and(|hash| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &hash)
                    .is_ok()
            })
    })
}

fn load_mfa_encryption_key() -> Result<zeroize::Zeroizing<[u8; 32]>, mfa::MfaError> {
    let value = zeroize::Zeroizing::new(
        std::env::var("ZELYRA_MFA_ENCRYPTION_KEY")
            .map_err(|_| mfa::MfaError::InvalidEncryptionKey)?,
    );
    mfa::encryption_key_from_hex(&value)
}

fn begin_mfa_enrollment(
    auth: &AuthRoute,
    database_url: &str,
    mfa_table: &str,
    user_id: i64,
) -> Response {
    let key = match load_mfa_encryption_key() {
        Ok(key) => key,
        Err(_) => {
            eprintln!("zelyra web: MFA encryption key is missing or invalid");
            return Response::html(503, "<h1>503 Service Unavailable</h1>")
                .with_header("Cache-Control", "no-store");
        }
    };
    let secret = match mfa::generate_totp_secret() {
        Ok(secret) => zeroize::Zeroizing::new(secret),
        Err(_) => return Response::html(503, "<h1>503 Service Unavailable</h1>"),
    };
    let encrypted = match mfa::encrypt_totp_secret(&key, &secret) {
        Ok(encrypted) => encrypted,
        Err(_) => {
            eprintln!("zelyra web: MFA secret encryption failed");
            return Response::html(503, "<h1>503 Service Unavailable</h1>")
                .with_header("Cache-Control", "no-store");
        }
    };
    let result = zelyra_database::with_mariadb_transaction(database_url, |transaction| {
        let existing = transaction.execute(&zelyra_database::Query {
            sql: format!(
                "SELECT enabled_at IS NOT NULL FROM {} WHERE user_id = :user_id LIMIT 1 FOR UPDATE",
                quote_identifier(mfa_table)
            ),
            params: vec![("user_id".into(), QueryValue::Int(user_id))],
        })?;
        if existing
            .rows
            .first()
            .and_then(|row| row.first())
            .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        {
            return Ok(false);
        }
        transaction.execute(&zelyra_database::Query {
            sql: format!(
                "INSERT INTO {} (user_id, secret_ciphertext, enabled_at, last_totp_step, failed_attempts, locked_until, enrollment_expires_at) VALUES (:user_id, :secret, NULL, NULL, 0, NULL, DATE_ADD(CURRENT_TIMESTAMP, INTERVAL 10 MINUTE)) ON DUPLICATE KEY UPDATE secret_ciphertext = VALUES(secret_ciphertext), enabled_at = NULL, last_totp_step = NULL, failed_attempts = 0, locked_until = NULL, enrollment_expires_at = VALUES(enrollment_expires_at)",
                quote_identifier(mfa_table)
            ),
            params: vec![
                ("user_id".into(), QueryValue::Int(user_id)),
                ("secret".into(), QueryValue::String(encrypted)),
            ],
        })?;
        Ok(true)
    });
    match result {
        Ok(true) => {
            let csrf = html_escape(auth.csrf.token());
            Response::html(
                200,
                format!(
                    "<main><h1>Set up authenticator</h1><p>Enter this key in your authenticator app. It is shown only during setup.</p><p><code>{}</code></p><form method=\"post\" action=\"{ACCOUNT_SECURITY_PATH}\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{csrf}\"><input type=\"hidden\" name=\"action\" value=\"confirm\"><label for=\"password\">Confirm password again</label><input id=\"password\" name=\"password\" type=\"password\" autocomplete=\"current-password\" required><label for=\"code\">Six-digit authenticator code</label><input id=\"code\" name=\"code\" inputmode=\"numeric\" pattern=\"[0-9]{{6}}\" required><button type=\"submit\">Enable MFA</button></form></main>",
                    html_escape(&secret)
                ),
            )
            .with_header("Cache-Control", "no-store")
        }
        Ok(false) => Response::html(409, "<h1>409 Conflict</h1><p>MFA is already enabled.</p>")
            .with_header("Cache-Control", "no-store"),
        Err(error) => {
            eprintln!("zelyra web: MFA enrollment could not be started: {error}");
            Response::html(500, "<h1>500 Internal Server Error</h1>")
                .with_header("Cache-Control", "no-store")
        }
    }
}

#[derive(Clone, Copy)]
struct MfaAccountContext<'a> {
    auth: &'a AuthRoute,
    database_url: &'a str,
    mfa_table: &'a str,
    recovery_table: &'a str,
    session_table: &'a str,
    user_id: i64,
    current_token_hash: &'a str,
}

fn confirm_mfa_enrollment(context: MfaAccountContext<'_>, code: &str) -> Response {
    let MfaAccountContext {
        auth,
        database_url,
        mfa_table,
        recovery_table,
        session_table,
        user_id,
        current_token_hash,
    } = context;
    let key = match load_mfa_encryption_key() {
        Ok(key) => key,
        Err(_) => {
            eprintln!("zelyra web: MFA encryption key is missing or invalid");
            return Response::html(503, "<h1>503 Service Unavailable</h1>")
                .with_header("Cache-Control", "no-store");
        }
    };
    let code = zeroize::Zeroizing::new(code.to_owned());
    let result = zelyra_database::with_mariadb_transaction(database_url, |transaction| {
        let factor = transaction.execute(&zelyra_database::Query {
            sql: format!(
                "SELECT secret_ciphertext, failed_attempts, locked_until > CURRENT_TIMESTAMP FROM {} WHERE user_id = :user_id AND enabled_at IS NULL AND enrollment_expires_at > CURRENT_TIMESTAMP LIMIT 1 FOR UPDATE",
                quote_identifier(mfa_table)
            ),
            params: vec![("user_id".into(), QueryValue::Int(user_id))],
        })?;
        let Some(row) = factor.rows.first() else {
            return Ok(EnrollmentResult::Expired);
        };
        if row
            .get(2)
            .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        {
            return Ok(EnrollmentResult::Locked);
        }
        let encrypted = row.first().map(String::as_str).unwrap_or_default();
        let secret = mfa::decrypt_totp_secret(&key, encrypted).map_err(|_| {
            zelyra_database::DatabaseError {
                message: "MFA enrollment secret could not be decrypted".into(),
            }
        })?;
        let now = mfa::unix_time_seconds().map_err(|_| zelyra_database::DatabaseError {
            message: "MFA clock is unavailable".into(),
        })?;
        let step = mfa::matching_totp_step(&secret, &code, now).ok().flatten();
        let Some(step) = step else {
            transaction.execute(&zelyra_database::Query {
                sql: format!(
                    "UPDATE {} SET failed_attempts = failed_attempts + 1, locked_until = IF(failed_attempts + 1 >= 5, DATE_ADD(CURRENT_TIMESTAMP, INTERVAL 15 MINUTE), locked_until) WHERE user_id = :user_id AND enabled_at IS NULL",
                    quote_identifier(mfa_table)
                ),
                params: vec![("user_id".into(), QueryValue::Int(user_id))],
            })?;
            return Ok(EnrollmentResult::Invalid);
        };
        let recovery_codes =
            mfa::generate_recovery_codes().map_err(|_| zelyra_database::DatabaseError {
                message: "recovery code generation failed".into(),
            })?;
        let mut recovery_hashes = Vec::with_capacity(recovery_codes.len());
        for recovery_code in &recovery_codes {
            let hash = mfa::hash_recovery_code(recovery_code).map_err(|_| {
                zelyra_database::DatabaseError {
                    message: "recovery code hashing failed".into(),
                }
            })?;
            recovery_hashes.push(hash);
        }
        transaction.execute(&zelyra_database::Query {
            sql: format!(
                "UPDATE {} SET enabled_at = CURRENT_TIMESTAMP, last_totp_step = :step, failed_attempts = 0, locked_until = NULL, enrollment_expires_at = NULL WHERE user_id = :user_id AND enabled_at IS NULL",
                quote_identifier(mfa_table)
            ),
            params: vec![
                ("step".into(), QueryValue::Int(step as i64)),
                ("user_id".into(), QueryValue::Int(user_id)),
            ],
        })?;
        transaction.execute(&zelyra_database::Query {
            sql: format!(
                "DELETE FROM {} WHERE user_id = :user_id",
                quote_identifier(recovery_table)
            ),
            params: vec![("user_id".into(), QueryValue::Int(user_id))],
        })?;
        for hash in recovery_hashes {
            transaction.execute(&zelyra_database::Query {
                sql: format!(
                    "INSERT INTO {} (user_id, code_hash, used_at) VALUES (:user_id, :code_hash, NULL)",
                    quote_identifier(recovery_table)
                ),
                params: vec![
                    ("user_id".into(), QueryValue::Int(user_id)),
                    ("code_hash".into(), QueryValue::String(hash)),
                ],
            })?;
        }
        transaction.execute(&zelyra_database::Query {
            sql: format!(
                "DELETE FROM {} WHERE user_id = :user_id AND token_hash <> :current_hash",
                quote_identifier(session_table)
            ),
            params: vec![
                ("user_id".into(), QueryValue::Int(user_id)),
                (
                    "current_hash".into(),
                    QueryValue::String(current_token_hash.to_owned()),
                ),
            ],
        })?;
        if let Some(audit_table) = auth.audit_table.as_deref() {
            for query in audit_insert_queries(
                audit_table,
                auth.audit_chain,
                Some(user_id),
                "auth.mfa_enabled",
                Some(user_id),
                "recovery_codes=10",
            ) {
                transaction.execute(&query)?;
            }
        }
        Ok(EnrollmentResult::Enabled(recovery_codes))
    });
    match result {
        Ok(EnrollmentResult::Enabled(codes)) => {
            let rendered_codes = codes
                .iter()
                .map(|code| format!("<li><code>{}</code></li>", html_escape(code)))
                .collect::<String>();
            Response::html(
                200,
                format!(
                    "<main><h1>MFA enabled</h1><p>Save these recovery codes now. They will not be shown again.</p><ol>{rendered_codes}</ol><a href=\"{ACCOUNT_SECURITY_PATH}\">Continue</a></main>"
                ),
            )
            .with_header("Cache-Control", "no-store")
        }
        Ok(EnrollmentResult::Invalid) => Response::html(
            401,
            "<h1>401 Unauthorized</h1><p>Invalid verification code.</p>",
        )
        .with_header("Cache-Control", "no-store"),
        Ok(EnrollmentResult::Locked) => {
            Response::html(429, "<h1>429 Too Many Requests</h1><p>Try again later.</p>")
                .with_header("Retry-After", "900")
                .with_header("Cache-Control", "no-store")
        }
        Ok(EnrollmentResult::Expired) => {
            Response::html(410, "<h1>410 Gone</h1><p>Start MFA setup again.</p>")
                .with_header("Cache-Control", "no-store")
        }
        Err(error) => {
            eprintln!("zelyra web: MFA enrollment could not be completed: {error}");
            Response::html(503, "<h1>503 Service Unavailable</h1>")
                .with_header("Cache-Control", "no-store")
        }
    }
}

enum EnrollmentResult {
    Enabled(Vec<zeroize::Zeroizing<String>>),
    Invalid,
    Locked,
    Expired,
}

fn update_mfa_security(
    context: MfaAccountContext<'_>,
    supplied_code: &str,
    disable: bool,
) -> Response {
    let MfaAccountContext {
        auth,
        database_url,
        mfa_table,
        recovery_table,
        session_table,
        user_id,
        current_token_hash,
    } = context;
    let key = match load_mfa_encryption_key() {
        Ok(key) => key,
        Err(_) => {
            eprintln!("zelyra web: MFA encryption key is missing or invalid");
            return Response::html(503, "<h1>503 Service Unavailable</h1>")
                .with_header("Cache-Control", "no-store");
        }
    };
    let supplied_code = zeroize::Zeroizing::new(supplied_code.to_owned());
    let (recovery_codes, recovery_hashes) = if disable {
        (Vec::new(), Vec::new())
    } else {
        let codes = match mfa::generate_recovery_codes() {
            Ok(codes) => codes,
            Err(_) => return Response::html(503, "<h1>503 Service Unavailable</h1>"),
        };
        let mut hashes = Vec::with_capacity(codes.len());
        for code in &codes {
            match mfa::hash_recovery_code(code) {
                Ok(hash) => hashes.push(hash),
                Err(_) => return Response::html(503, "<h1>503 Service Unavailable</h1>"),
            }
        }
        (codes, hashes)
    };
    let result = zelyra_database::with_mariadb_transaction(database_url, |transaction| {
        let factor = transaction.execute(&zelyra_database::Query {
            sql: format!(
                "SELECT secret_ciphertext, last_totp_step, locked_until > CURRENT_TIMESTAMP FROM {} WHERE user_id = :user_id AND enabled_at IS NOT NULL LIMIT 1 FOR UPDATE",
                quote_identifier(mfa_table)
            ),
            params: vec![("user_id".into(), QueryValue::Int(user_id))],
        })?;
        let Some(row) = factor.rows.first() else {
            return Ok(MfaSecurityResult::NotEnabled);
        };
        if row
            .get(2)
            .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        {
            return Ok(MfaSecurityResult::Locked);
        }
        let encrypted_secret = row.first().map(String::as_str).unwrap_or_default();
        let last_step = row.get(1).and_then(|value| value.parse::<u64>().ok());
        let secret = mfa::decrypt_totp_secret(&key, encrypted_secret).map_err(|_| {
            zelyra_database::DatabaseError {
                message: "MFA factor could not be decrypted".into(),
            }
        })?;
        let now = mfa::unix_time_seconds().map_err(|_| zelyra_database::DatabaseError {
            message: "MFA clock is unavailable".into(),
        })?;
        let matched_step = mfa::matching_totp_step(&secret, &supplied_code, now)
            .ok()
            .flatten()
            .filter(|step| last_step.is_some_and(|last| *step > last));
        let Some(step) = matched_step else {
            transaction.execute(&zelyra_database::Query {
                sql: format!(
                    "UPDATE {} SET failed_attempts = failed_attempts + 1, locked_until = IF(failed_attempts + 1 >= 5, DATE_ADD(CURRENT_TIMESTAMP, INTERVAL 15 MINUTE), locked_until) WHERE user_id = :user_id AND enabled_at IS NOT NULL",
                    quote_identifier(mfa_table)
                ),
                params: vec![("user_id".into(), QueryValue::Int(user_id))],
            })?;
            return Ok(MfaSecurityResult::Invalid);
        };

        if disable {
            transaction.execute(&zelyra_database::Query {
                sql: format!(
                    "DELETE FROM {} WHERE user_id = :user_id",
                    quote_identifier(recovery_table)
                ),
                params: vec![("user_id".into(), QueryValue::Int(user_id))],
            })?;
            transaction.execute(&zelyra_database::Query {
                sql: format!(
                    "DELETE FROM {} WHERE user_id = :user_id AND enabled_at IS NOT NULL",
                    quote_identifier(mfa_table)
                ),
                params: vec![("user_id".into(), QueryValue::Int(user_id))],
            })?;
            transaction.execute(&zelyra_database::Query {
                sql: format!(
                    "DELETE FROM {} WHERE user_id = :user_id AND token_hash <> :current_hash",
                    quote_identifier(session_table)
                ),
                params: vec![
                    ("user_id".into(), QueryValue::Int(user_id)),
                    (
                        "current_hash".into(),
                        QueryValue::String(current_token_hash.to_owned()),
                    ),
                ],
            })?;
            if let Some(audit_table) = auth.audit_table.as_deref() {
                for query in audit_insert_queries(
                    audit_table,
                    auth.audit_chain,
                    Some(user_id),
                    "auth.mfa_disabled",
                    Some(user_id),
                    "",
                ) {
                    transaction.execute(&query)?;
                }
            }
            return Ok(MfaSecurityResult::Disabled);
        }

        transaction.execute(&zelyra_database::Query {
            sql: format!(
                "UPDATE {} SET last_totp_step = :step, failed_attempts = 0, locked_until = NULL WHERE user_id = :user_id AND enabled_at IS NOT NULL",
                quote_identifier(mfa_table)
            ),
            params: vec![
                ("step".into(), QueryValue::Int(step as i64)),
                ("user_id".into(), QueryValue::Int(user_id)),
            ],
        })?;
        transaction.execute(&zelyra_database::Query {
            sql: format!(
                "DELETE FROM {} WHERE user_id = :user_id",
                quote_identifier(recovery_table)
            ),
            params: vec![("user_id".into(), QueryValue::Int(user_id))],
        })?;
        for hash in &recovery_hashes {
            transaction.execute(&zelyra_database::Query {
                sql: format!(
                    "INSERT INTO {} (user_id, code_hash, used_at) VALUES (:user_id, :code_hash, NULL)",
                    quote_identifier(recovery_table)
                ),
                params: vec![
                    ("user_id".into(), QueryValue::Int(user_id)),
                    ("code_hash".into(), QueryValue::String(hash.clone())),
                ],
            })?;
        }
        if let Some(audit_table) = auth.audit_table.as_deref() {
            for query in audit_insert_queries(
                audit_table,
                auth.audit_chain,
                Some(user_id),
                "auth.mfa_recovery_codes_replaced",
                Some(user_id),
                "recovery_codes=10",
            ) {
                transaction.execute(&query)?;
            }
        }
        Ok(MfaSecurityResult::RecoveryCodes(recovery_codes))
    });

    match result {
        Ok(MfaSecurityResult::Disabled) => {
            Response::redirect(ACCOUNT_SECURITY_PATH).with_header("Cache-Control", "no-store")
        }
        Ok(MfaSecurityResult::RecoveryCodes(codes)) => {
            let rendered_codes = codes
                .iter()
                .map(|code| format!("<li><code>{}</code></li>", html_escape(code)))
                .collect::<String>();
            Response::html(
                200,
                format!(
                    "<main><h1>Recovery codes replaced</h1><p>Save these codes now. Earlier recovery codes no longer work. They will not be shown again.</p><ol>{rendered_codes}</ol><a href=\"{ACCOUNT_SECURITY_PATH}\">Continue</a></main>"
                ),
            )
            .with_header("Cache-Control", "no-store")
        }
        Ok(MfaSecurityResult::Invalid) => Response::html(
            401,
            "<h1>401 Unauthorized</h1><p>Invalid authenticator code.</p>",
        )
        .with_header("Cache-Control", "no-store"),
        Ok(MfaSecurityResult::Locked) => {
            Response::html(429, "<h1>429 Too Many Requests</h1><p>Try again later.</p>")
                .with_header("Retry-After", "900")
                .with_header("Cache-Control", "no-store")
        }
        Ok(MfaSecurityResult::NotEnabled) => {
            Response::html(409, "<h1>409 Conflict</h1><p>MFA is not enabled.</p>")
                .with_header("Cache-Control", "no-store")
        }
        Err(error) => {
            eprintln!("zelyra web: MFA security update failed: {error}");
            Response::html(503, "<h1>503 Service Unavailable</h1>")
                .with_header("Cache-Control", "no-store")
        }
    }
}

enum MfaSecurityResult {
    Disabled,
    RecoveryCodes(Vec<zeroize::Zeroizing<String>>),
    Invalid,
    Locked,
    NotEnabled,
}

fn dispatch_login_mfa(
    app: &WebApp,
    auth: &AuthRoute,
    request: &Request,
    database_url: Option<&str>,
) -> Response {
    if app.database_capability_granted == Some(false) {
        return database_capability_denied();
    }
    let (Some(session_table), Some(mfa_table), Some(recovery_table), Some(database_url)) = (
        auth.session_table.as_deref(),
        auth.mfa_table.as_deref(),
        auth.mfa_recovery_table.as_deref(),
        database_url,
    ) else {
        return Response::html(503, "<h1>503 Service Unavailable</h1>")
            .with_header("Cache-Control", "no-store");
    };
    let Some(session_id) = cookie_value(request, "zelyra_session") else {
        return Response::redirect("/login");
    };
    let token_hash = session_token_hash(&session_id);
    let pending_user = match pending_mfa_user(auth, database_url, session_table, &token_hash) {
        Ok(Some(user_id)) => user_id,
        Ok(None) => return Response::redirect("/login"),
        Err(error) => {
            eprintln!("zelyra web: MFA session lookup failed: {error}");
            return Response::html(500, "<h1>500 Internal Server Error</h1>")
                .with_header("Cache-Control", "no-store");
        }
    };
    match request.method.as_str() {
        "GET" => Response::html(
            200,
            format!(
                "<main><h1>Additional verification</h1><p>Enter an authenticator code or an unused recovery code.</p><form method=\"post\" action=\"{LOGIN_MFA_PATH}\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{}\"><label for=\"code\">Authenticator or recovery code</label><input id=\"code\" name=\"code\" autocomplete=\"one-time-code\" required><button type=\"submit\">Verify</button></form></main>",
                html_escape(auth.csrf.token())
            ),
        )
        .with_header("Cache-Control", "no-store"),
        "POST" => {
            let input = match parse_urlencoded(&request.body) {
                Ok(input) => input,
                Err(_) => {
                    return Response::html(400, "<h1>400 Bad Request</h1>")
                        .with_header("Cache-Control", "no-store")
                }
            };
            if !verify_csrf_request(
                request,
                &auth.csrf,
                input.get("_zelyra_csrf").map(String::as_str),
            ) {
                return Response::html(403, "<h1>403 Forbidden</h1>")
                    .with_header("Cache-Control", "no-store");
            }
            let Some(code) = input.get("code").map(String::as_str) else {
                return Response::html(401, "<h1>401 Unauthorized</h1><p>Invalid verification code.</p>")
                    .with_header("Cache-Control", "no-store");
            };
            let key = match load_mfa_encryption_key() {
                Ok(key) => key,
                Err(_) => {
                    eprintln!("zelyra web: MFA encryption key is missing or invalid");
                    return Response::html(503, "<h1>503 Service Unavailable</h1>")
                        .with_header("Cache-Control", "no-store");
                }
            };
            let now = match mfa::unix_time_seconds() {
                Ok(now) => now,
                Err(_) => return Response::html(503, "<h1>503 Service Unavailable</h1>"),
            };
            let code = code.to_owned();
            let result = zelyra_database::with_mariadb_transaction(database_url, |transaction| {
                let session = transaction.execute(&zelyra_database::Query {
                    sql: format!(
                        "SELECT s.user_id FROM {} AS s INNER JOIN {} AS u ON u.id = s.user_id WHERE s.token_hash = :token_hash AND s.expires_at > CURRENT_TIMESTAMP AND s.mfa_verified = false{} LIMIT 1 FOR UPDATE",
                        quote_identifier(session_table),
                        quote_identifier(&auth.table),
                        if auth.schema.tables.iter().find(|table| table.name == auth.table).is_some_and(|table| table.columns.iter().any(|column| column.name == "active")) { " AND u.active = true" } else { "" },
                    ),
                    params: vec![("token_hash".into(), QueryValue::String(token_hash.clone()))],
                })?;
                let user_id = session
                    .rows
                    .first()
                    .and_then(|row| row.first())
                    .and_then(|value| value.parse::<i64>().ok());
                if user_id != Some(pending_user) {
                    return Ok(MfaChallengeResult::NoPendingSession);
                }
                let factor = transaction.execute(&zelyra_database::Query {
                    sql: format!(
                        "SELECT secret_ciphertext, last_totp_step, locked_until > CURRENT_TIMESTAMP FROM {} WHERE user_id = :user_id AND enabled_at IS NOT NULL LIMIT 1 FOR UPDATE",
                        quote_identifier(mfa_table)
                    ),
                    params: vec![("user_id".into(), QueryValue::Int(pending_user))],
                })?;
                let Some(row) = factor.rows.first() else {
                    return Ok(MfaChallengeResult::Invalid);
                };
                if row.get(2).is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true")) {
                    return Ok(MfaChallengeResult::Locked);
                }
                let encrypted_secret = row.first().map(String::as_str).unwrap_or_default();
                let last_step = row.get(1).and_then(|value| value.parse::<u64>().ok());
                let secret = mfa::decrypt_totp_secret(&key, encrypted_secret)
                    .map_err(|_| zelyra_database::DatabaseError { message: "MFA factor could not be decrypted".into() })?;
                let matched_step = mfa::matching_totp_step(&secret, &code, now)
                    .ok()
                    .flatten()
                    .filter(|step| last_step.is_some_and(|last| *step > last));
                let recovery_id = if matched_step.is_none() {
                    let recovery_rows = transaction.execute(&zelyra_database::Query {
                        sql: format!(
                            "SELECT id, code_hash FROM {} WHERE user_id = :user_id AND used_at IS NULL ORDER BY id LIMIT 20 FOR UPDATE",
                            quote_identifier(recovery_table)
                        ),
                        params: vec![("user_id".into(), QueryValue::Int(pending_user))],
                    })?;
                    recovery_rows.rows.into_iter().find_map(|recovery| {
                        let id = recovery.first()?.parse::<i64>().ok()?;
                        let hash = recovery.get(1)?;
                        mfa::verify_recovery_code(&code, hash).then_some(id)
                    })
                } else {
                    None
                };
                if matched_step.is_none() && recovery_id.is_none() {
                    transaction.execute(&zelyra_database::Query {
                        sql: format!(
                            "UPDATE {} SET failed_attempts = failed_attempts + 1, locked_until = IF(failed_attempts + 1 >= 5, DATE_ADD(CURRENT_TIMESTAMP, INTERVAL 15 MINUTE), locked_until) WHERE user_id = :user_id AND enabled_at IS NOT NULL",
                            quote_identifier(mfa_table)
                        ),
                        params: vec![("user_id".into(), QueryValue::Int(pending_user))],
                    })?;
                    return Ok(MfaChallengeResult::Invalid);
                }
                if let Some(step) = matched_step {
                    transaction.execute(&zelyra_database::Query {
                        sql: format!(
                            "UPDATE {} SET last_totp_step = :step, failed_attempts = 0, locked_until = NULL WHERE user_id = :user_id AND enabled_at IS NOT NULL",
                            quote_identifier(mfa_table)
                        ),
                        params: vec![
                            ("step".into(), QueryValue::Int(step as i64)),
                            ("user_id".into(), QueryValue::Int(pending_user)),
                        ],
                    })?;
                } else if let Some(recovery_id) = recovery_id {
                    transaction.execute(&zelyra_database::Query {
                        sql: format!(
                            "UPDATE {} SET used_at = CURRENT_TIMESTAMP WHERE id = :id AND user_id = :user_id AND used_at IS NULL",
                            quote_identifier(recovery_table)
                        ),
                        params: vec![
                            ("id".into(), QueryValue::Int(recovery_id)),
                            ("user_id".into(), QueryValue::Int(pending_user)),
                        ],
                    })?;
                    transaction.execute(&zelyra_database::Query {
                        sql: format!(
                            "UPDATE {} SET failed_attempts = 0, locked_until = NULL WHERE user_id = :user_id AND enabled_at IS NOT NULL",
                            quote_identifier(mfa_table)
                        ),
                        params: vec![("user_id".into(), QueryValue::Int(pending_user))],
                    })?;
                }
                transaction.execute(&zelyra_database::Query {
                    sql: format!(
                        "UPDATE {} SET mfa_verified = true WHERE token_hash = :token_hash AND user_id = :user_id AND mfa_verified = false AND expires_at > CURRENT_TIMESTAMP",
                        quote_identifier(session_table)
                    ),
                    params: vec![
                        ("token_hash".into(), QueryValue::String(token_hash.clone())),
                        ("user_id".into(), QueryValue::Int(pending_user)),
                    ],
                })?;
                if let Some(audit_table) = auth.audit_table.as_deref() {
                    for query in audit_insert_queries(
                        audit_table,
                        auth.audit_chain,
                        Some(pending_user),
                        "auth.login",
                        Some(pending_user),
                        "mfa=verified",
                    ) {
                        transaction.execute(&query)?;
                    }
                }
                Ok(MfaChallengeResult::Verified)
            });
            match result {
                Ok(MfaChallengeResult::Verified) => {
                    Response::redirect("/").with_header("Cache-Control", "no-store")
                }
                Ok(MfaChallengeResult::Locked) => Response::html(
                    429,
                    "<h1>429 Too Many Requests</h1><p>Try again later.</p>",
                )
                .with_header("Retry-After", "900")
                .with_header("Cache-Control", "no-store"),
                Ok(MfaChallengeResult::NoPendingSession) => Response::redirect("/login"),
                Ok(MfaChallengeResult::Invalid) => Response::html(
                    401,
                    "<h1>401 Unauthorized</h1><p>Invalid verification code.</p>",
                )
                .with_header("Cache-Control", "no-store"),
                Err(error) => {
                    eprintln!("zelyra web: MFA challenge failed: {error}");
                    Response::html(503, "<h1>503 Service Unavailable</h1>")
                        .with_header("Cache-Control", "no-store")
                }
            }
        }
        _ => Response::empty(405)
            .with_header("Allow", "GET, POST")
            .with_header("Cache-Control", "no-store"),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MfaChallengeResult {
    Verified,
    Invalid,
    Locked,
    NoPendingSession,
}

fn pending_mfa_user(
    auth: &AuthRoute,
    database_url: &str,
    session_table: &str,
    token_hash: &str,
) -> Result<Option<i64>, String> {
    let result = zelyra_database::execute_mariadb_query(
        database_url,
        &format!(
            "SELECT s.user_id FROM {} AS s INNER JOIN {} AS u ON u.id = s.user_id WHERE s.token_hash = :token_hash AND s.expires_at > CURRENT_TIMESTAMP AND s.mfa_verified = false{} LIMIT 1",
            quote_identifier(session_table),
            quote_identifier(&auth.table),
            if auth.schema.tables.iter().find(|table| table.name == auth.table).is_some_and(|table| table.columns.iter().any(|column| column.name == "active")) { " AND u.active = true" } else { "" },
        ),
        vec![("token_hash".into(), QueryValue::String(token_hash.into()))],
    )
    .map_err(|error| error.to_string())?;
    Ok(result
        .rows
        .first()
        .and_then(|row| row.first())
        .and_then(|value| value.parse::<i64>().ok()))
}

fn login_throttle_key(email: &str) -> String {
    let normalized = email.trim().to_ascii_lowercase();
    let digest = Blake2s256::digest(normalized.as_bytes());
    let encoded = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("email:{encoded}")
}

#[cfg(test)]
fn login_is_blocked(app: &WebApp, key: &str) -> bool {
    login_is_blocked_with_policy(app, key, DEFAULT_LOGIN_RATE_LIMIT)
}

fn login_is_blocked_with_policy(
    app: &WebApp,
    key: &str,
    rate_limit: zelyra_ast::ApiRateLimit,
) -> bool {
    let Ok(mut throttle) = app.login_throttle.lock() else {
        return true;
    };
    let now = Instant::now();
    throttle.retain(|_, state| {
        let window_expired = now.duration_since(state.window_started)
            >= Duration::from_secs(u64::from(state.window_seconds));
        let still_blocked = state.blocked_until.is_some_and(|until| now < until);
        !window_expired || still_blocked
    });
    let Some(state) = throttle.get_mut(key) else {
        return throttle.len() >= LOGIN_THROTTLE_MAX_KEYS;
    };
    if let Some(blocked_until) = state.blocked_until {
        if now < blocked_until {
            return true;
        }
        state.failures = 0;
        state.blocked_until = None;
        state.window_started = now;
        return false;
    }
    if now.duration_since(state.window_started)
        >= Duration::from_secs(u64::from(rate_limit.window_seconds))
    {
        throttle.remove(key);
        return false;
    }
    false
}

#[cfg(test)]
fn record_login_failure(app: &WebApp, key: String) {
    record_login_failure_with_policy(
        app,
        key,
        DEFAULT_LOGIN_RATE_LIMIT,
        DEFAULT_LOGIN_BLOCK_SECONDS,
    );
}

fn record_login_failure_with_policy(
    app: &WebApp,
    key: String,
    rate_limit: zelyra_ast::ApiRateLimit,
    block_seconds: u32,
) {
    let Ok(mut throttle) = app.login_throttle.lock() else {
        return;
    };
    let now = Instant::now();
    throttle.retain(|_, state| {
        let window_expired = now.duration_since(state.window_started)
            >= Duration::from_secs(u64::from(state.window_seconds));
        let still_blocked = state.blocked_until.is_some_and(|until| now < until);
        !window_expired || still_blocked
    });
    if !throttle.contains_key(&key) && throttle.len() >= LOGIN_THROTTLE_MAX_KEYS {
        return;
    }
    let state = throttle.entry(key).or_insert(LoginThrottle {
        window_started: now,
        window_seconds: rate_limit.window_seconds,
        failures: 0,
        blocked_until: None,
    });
    if now.duration_since(state.window_started)
        >= Duration::from_secs(u64::from(rate_limit.window_seconds))
    {
        state.window_started = now;
        state.window_seconds = rate_limit.window_seconds;
        state.failures = 0;
        state.blocked_until = None;
    }
    state.failures = state.failures.saturating_add(1);
    if state.failures >= rate_limit.requests {
        state.blocked_until = Some(now + Duration::from_secs(u64::from(block_seconds)));
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
    if !verify_csrf_request(
        request,
        &auth.csrf,
        input.get("_zelyra_csrf").map(String::as_str),
    ) {
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
        format!(
            "zelyra_session=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax{}",
            secure_cookie_attribute(request)
        ),
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AccountSessionRow {
    key: String,
    number: Option<String>,
    expires: String,
    expires_relative: bool,
    current: bool,
    device: Option<String>,
}

fn dispatch_account_sessions(
    app: &WebApp,
    request: &Request,
    database_url: Option<&str>,
) -> Response {
    let Some(auth) = app.auth_route.as_ref() else {
        return Response::empty(404);
    };
    if app.database_capability_granted == Some(false) && auth.session_table.is_some() {
        return database_capability_denied();
    }
    let Some(session) = session_from_request(app, request, database_url) else {
        return Response::redirect("/login");
    };
    let Some(user_id) = session.user_id.filter(|id| *id > 0) else {
        return Response::redirect("/login");
    };
    match request.method.as_str() {
        "GET" => {
            let rows = match account_sessions_for_user(
                app,
                auth,
                database_url,
                user_id,
                cookie_value(request, "zelyra_session").as_deref(),
            ) {
                Ok(rows) => rows,
                Err(error) => {
                    eprintln!("zelyra web: account session lookup failed: {error}");
                    return Response::html(500, "<h1>500 Internal Server Error</h1>");
                }
            };
            Response::html(200, render_account_sessions(auth, &rows, app.ui_language))
        }
        "POST" => {
            let input = match parse_urlencoded(&request.body) {
                Ok(input) => input,
                Err(error) => {
                    return Response::html(400, format!("<h1>400 Bad Request</h1><p>{error}</p>"))
                }
            };
            if !verify_csrf_request(
                request,
                &auth.csrf,
                input.get("_zelyra_csrf").map(String::as_str),
            ) {
                return Response::html(403, "<h1>403 Forbidden</h1><p>Invalid CSRF token.</p>");
            }
            let Some(key) = input.get("session_key").map(String::as_str) else {
                return Response::html(
                    422,
                    "<h1>422 Unprocessable Entity</h1><p>A valid session is required.</p>",
                );
            };
            let current_token_hash = cookie_value(request, "zelyra_session")
                .as_deref()
                .map(session_token_hash);
            let target_token_hash = if let (Some(table), Some(database_url)) =
                (auth.session_table.as_deref(), database_url)
            {
                let has_id = session_table_has_id(auth);
                let target_hash = match lookup_account_session_token_hash(
                    table,
                    database_url,
                    user_id,
                    key,
                    has_id,
                ) {
                    Ok(Some(token_hash)) => token_hash,
                    Ok(None) => return Response::redirect(ACCOUNT_SESSIONS_PATH),
                    Err(error) => {
                        eprintln!("zelyra web: account session lookup failed: {error}");
                        return Response::html(500, "<h1>500 Internal Server Error</h1>");
                    }
                };
                let (predicate, parameter, audit_details) = if has_id {
                    let Some(id) = key
                        .strip_prefix("id:")
                        .and_then(|id| id.parse::<i64>().ok())
                        .filter(|id| *id > 0)
                    else {
                        return Response::html(
                            422,
                            "<h1>422 Unprocessable Entity</h1><p>A valid session is required.</p>",
                        );
                    };
                    (
                        "id = :session_id",
                        ("session_id", zelyra_database::QueryValue::Int(id)),
                        format!("session_id={id}"),
                    )
                } else {
                    let Some(hash) = key
                        .strip_prefix("hash:")
                        .filter(|hash| is_session_hash(hash))
                    else {
                        return Response::html(
                            422,
                            "<h1>422 Unprocessable Entity</h1><p>A valid session is required.</p>",
                        );
                    };
                    (
                        "token_hash = :token_hash",
                        (
                            "token_hash",
                            zelyra_database::QueryValue::String(hash.into()),
                        ),
                        "session_id=redacted".into(),
                    )
                };
                let query = zelyra_database::Query {
                    sql: format!(
                        "DELETE FROM {} WHERE user_id = :user_id AND {predicate}",
                        quote_identifier(table)
                    ),
                    params: vec![
                        ("user_id".into(), zelyra_database::QueryValue::Int(user_id)),
                        (parameter.0.into(), parameter.1),
                    ],
                };
                if let Err(error) = execute_auth_admin_mutation(
                    auth,
                    database_url,
                    vec![query],
                    Some(user_id),
                    "auth.session_self_revoke",
                    Some(user_id),
                    &audit_details,
                ) {
                    eprintln!("zelyra web: account session revocation failed: {error}");
                    return Response::html(500, "<h1>500 Internal Server Error</h1>");
                }
                target_hash
            } else {
                let Some(hash) = key
                    .strip_prefix("hash:")
                    .filter(|hash| is_session_hash(hash))
                else {
                    return Response::html(
                        422,
                        "<h1>422 Unprocessable Entity</h1><p>A valid session is required.</p>",
                    );
                };
                let Ok(mut sessions) = app.sessions.lock() else {
                    return Response::html(500, "<h1>500 Internal Server Error</h1>");
                };
                let target = sessions
                    .iter()
                    .find(|(token, candidate)| {
                        candidate.user_id == Some(user_id) && session_token_hash(token) == hash
                    })
                    .map(|(token, _)| token.clone());
                if let Some(token) = target {
                    sessions.remove(&token);
                }
                hash.to_owned()
            };
            if current_token_hash.as_deref() == Some(target_token_hash.as_str()) {
                return Response::redirect("/login").with_header(
                    "Set-Cookie",
                    format!(
                        "zelyra_session=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax{}",
                        secure_cookie_attribute(request)
                    ),
                );
            }
            Response::redirect(ACCOUNT_SESSIONS_PATH)
        }
        _ => Response::empty(405).with_header("Allow", "GET, POST"),
    }
}

fn session_table_has_id(auth: &AuthRoute) -> bool {
    auth.session_table.as_deref().is_some_and(|name| {
        auth.schema
            .tables
            .iter()
            .find(|table| table.name == name)
            .is_some_and(|table| table.columns.iter().any(|column| column.name == "id"))
    })
}

fn is_session_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn lookup_account_session_token_hash(
    table: &str,
    database_url: &str,
    user_id: i64,
    key: &str,
    has_id: bool,
) -> Result<Option<String>, String> {
    let (selector, value) = if has_id {
        let Some(id) = key
            .strip_prefix("id:")
            .and_then(|id| id.parse::<i64>().ok())
            .filter(|id| *id > 0)
        else {
            return Ok(None);
        };
        (
            "id = :session_id",
            ("session_id", zelyra_database::QueryValue::Int(id)),
        )
    } else {
        let Some(hash) = key
            .strip_prefix("hash:")
            .filter(|hash| is_session_hash(hash))
        else {
            return Ok(None);
        };
        (
            "token_hash = :token_hash",
            (
                "token_hash",
                zelyra_database::QueryValue::String(hash.into()),
            ),
        )
    };
    let result = zelyra_database::execute_mariadb_query(
        database_url,
        &format!(
            "SELECT token_hash FROM {} WHERE user_id = :user_id AND {selector} AND expires_at > CURRENT_TIMESTAMP LIMIT 1",
            quote_identifier(table)
        ),
        vec![
            ("user_id".into(), zelyra_database::QueryValue::Int(user_id)),
            (value.0.into(), value.1),
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(result.rows.first().and_then(|row| row.first()).cloned())
}

fn account_sessions_for_user(
    app: &WebApp,
    auth: &AuthRoute,
    database_url: Option<&str>,
    user_id: i64,
    current_token: Option<&str>,
) -> Result<Vec<AccountSessionRow>, String> {
    let current_hash = current_token.map(session_token_hash);
    if let (Some(table), Some(database_url)) = (auth.session_table.as_deref(), database_url) {
        let has_id = session_table_has_id(auth);
        let has_device = auth_session_has_column(auth, "device_label");
        let first_column = if has_id { "id" } else { "token_hash" };
        let result = zelyra_database::execute_mariadb_query(
            database_url,
            &format!(
                "SELECT {first_column}, token_hash, expires_at, {} FROM {} WHERE user_id = :user_id AND expires_at > CURRENT_TIMESTAMP ORDER BY expires_at DESC LIMIT 100",
                if has_device { "device_label" } else { "''" },
                quote_identifier(table)
            ),
            vec![("user_id".into(), zelyra_database::QueryValue::Int(user_id))],
        )
        .map_err(|error| error.to_string())?;
        return Ok(result
            .rows
            .iter()
            .filter_map(|row| {
                let key = row.first()?;
                let token_hash = row.get(1)?;
                let expires = row.get(2)?;
                Some(AccountSessionRow {
                    key: if has_id {
                        format!("id:{key}")
                    } else {
                        format!("hash:{key}")
                    },
                    number: has_id.then(|| key.clone()),
                    expires: expires.clone(),
                    expires_relative: false,
                    current: current_hash.as_deref() == Some(token_hash.as_str()),
                    device: row
                        .get(3)
                        .filter(|value| !value.is_empty() && *value != "NULL")
                        .cloned(),
                })
            })
            .collect());
    }
    let mut sessions = app
        .sessions
        .lock()
        .map_err(|_| "session state is unavailable".to_owned())?;
    let now = Instant::now();
    sessions.retain(|_, session| session.expires_at > now);
    let mut rows = sessions
        .iter()
        .filter(|(_, session)| session.user_id == Some(user_id))
        .map(|(token, session)| {
            let hash = session_token_hash(token);
            AccountSessionRow {
                key: format!("hash:{hash}"),
                number: None,
                expires: session
                    .expires_at
                    .saturating_duration_since(now)
                    .as_secs()
                    .to_string(),
                expires_relative: true,
                current: current_hash.as_deref() == Some(hash.as_str()),
                device: session.device.clone(),
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| std::cmp::Reverse(row.current));
    rows.truncate(100);
    Ok(rows)
}

fn render_account_sessions(
    auth: &AuthRoute,
    sessions: &[AccountSessionRow],
    language: UiLanguage,
) -> String {
    let csrf = html_escape(auth.csrf.token());
    let mut html = format!(
        "<main><h1>{}</h1><p>{}</p><table><thead><tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr></thead><tbody>",
        tr(language, "auth.self_sessions_title"),
        tr(language, "auth.self_sessions_help"),
        tr(language, "auth.session_label"),
        tr(language, "auth.session_expiry"),
        tr(language, "auth.session_device"),
        tr(language, "auth.actions"),
    );
    for session in sessions {
        let label = session
            .number
            .as_ref()
            .map(|number| {
                format!(
                    "{} {}",
                    tr(language, "auth.session_label"),
                    html_escape(number)
                )
            })
            .unwrap_or_else(|| tr(language, "auth.session_label"));
        let label = if session.current {
            format!("{} ({})", label, tr(language, "auth.current_session"))
        } else {
            label
        };
        let expires = if session.expires_relative {
            format!(
                "{} {}",
                html_escape(&session.expires),
                tr(language, "auth.seconds_remaining")
            )
        } else {
            html_escape(&session.expires)
        };
        let device = session
            .device
            .as_deref()
            .map(html_escape)
            .unwrap_or_else(|| tr(language, "auth.session_device_unknown"));
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td><form method=\"post\" action=\"{}\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{}\"><input type=\"hidden\" name=\"session_key\" value=\"{}\"><button type=\"submit\">{}</button></form></td></tr>",
            label,
            expires,
            device,
            ACCOUNT_SESSIONS_PATH,
            csrf,
            html_escape(&session.key),
            tr(language, "auth.revoke_session"),
        ));
    }
    html.push_str("</tbody></table></main>");
    html
}

fn render_login(auth: &AuthRoute, language: UiLanguage) -> String {
    let reset_link = auth
        .reset_tokens_table
        .as_ref()
        .map(|_| {
            format!(
                "<p><a href=\"/forgot-password\">{}</a></p>",
                tr(language, "auth.reset_request_link")
            )
        })
        .unwrap_or_default();
    format!(
        "<main><h1>{}</h1><form method=\"post\" action=\"/login\">\
         <input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{}\">\
         <label for=\"email\">{}</label><input id=\"email\" name=\"email\" type=\"email\" required>\
         <label for=\"password\">{}</label><input id=\"password\" name=\"password\" type=\"password\" required>\
         <button type=\"submit\">{}</button></form>{reset_link}</main>",
        tr(language, "auth.login_title"),
        html_escape(auth.csrf.token()),
        tr(language, "auth.email"),
        tr(language, "auth.password"),
        tr(language, "auth.login_submit"),
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
                    &data.sessions,
                    app.ui_language,
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
    sessions: Vec<Vec<String>>,
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
        queries.extend(audit_insert_queries(
            audit_table,
            auth.audit_chain,
            actor_user_id,
            action,
            target_user_id,
            details,
        ));
    }
    zelyra_database::execute_mariadb_queries(database_url, &queries, true)
        .map(|_| zelyra_database::QueryResult::default())
}

pub fn audit_insert_queries(
    audit_table: &str,
    chain: bool,
    actor_user_id: Option<i64>,
    event: &str,
    target_record_id: Option<i64>,
    details: &str,
) -> Vec<zelyra_database::Query> {
    let details = details.chars().take(1000).collect::<String>();
    let params = vec![
        (
            "actor_user_id".into(),
            actor_user_id.map_or(
                zelyra_database::QueryValue::Null,
                zelyra_database::QueryValue::Int,
            ),
        ),
        (
            "event".into(),
            zelyra_database::QueryValue::String(event.into()),
        ),
        (
            "target_user_id".into(),
            target_record_id.map_or(
                zelyra_database::QueryValue::Null,
                zelyra_database::QueryValue::Int,
            ),
        ),
        (
            "details".into(),
            zelyra_database::QueryValue::String(details),
        ),
    ];
    if !chain {
        return vec![zelyra_database::Query {
            sql: format!(
                "INSERT INTO {} (actor_user_id, event, target_user_id, details) VALUES (:actor_user_id, :event, :target_user_id, :details)",
                quote_identifier(audit_table)
            ),
            params,
        }];
    }
    vec![
        zelyra_database::Query {
            sql: "SET @zelyra_prev_hash = '';".into(),
            params: Vec::new(),
        },
        zelyra_database::Query {
            sql: format!(
                "SELECT COALESCE(entry_hash, '') INTO @zelyra_prev_hash FROM {} ORDER BY id DESC LIMIT 1 FOR UPDATE",
                quote_identifier(audit_table)
            ),
            params: Vec::new(),
        },
        zelyra_database::Query {
            sql: format!(
                "INSERT INTO {} (actor_user_id, event, target_user_id, details, previous_hash, entry_hash, created_at) VALUES (:actor_user_id, :event, :target_user_id, :details, @zelyra_prev_hash, SHA2(CONCAT(@zelyra_prev_hash, '|', COALESCE(:actor_user_id, 'NULL'), '|', :event, '|', COALESCE(:target_user_id, 'NULL'), '|', :details, '|', DATE_FORMAT(CURRENT_TIMESTAMP, '%Y-%m-%d %H:%i:%s')), 256), CURRENT_TIMESTAMP)",
                quote_identifier(audit_table)
            ),
            params,
        },
    ]
}

fn audit_component(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            ';' | '\n' | '\r' => ' ',
            character => character,
        })
        .collect()
}

fn audit_sensitive_field(field: &str) -> bool {
    let field = field.to_ascii_lowercase();
    ["password", "token", "secret", "hash"]
        .iter()
        .any(|marker| field.contains(marker))
}

fn form_audit_details(
    form: &FormRoute,
    event: &str,
    path_params: &HashMap<String, String>,
    before_values: Option<&HashMap<String, String>>,
    values: &HashMap<String, String>,
) -> (Option<i64>, String) {
    let record_id = path_params
        .get("id")
        .and_then(|value| value.parse::<i64>().ok());
    let changes = form
        .form
        .fields
        .iter()
        .filter_map(|field| {
            let before = before_values.and_then(|values| values.get(&field.name));
            let after = values.get(&field.name);
            if event == "crud.create" {
                return after.map(|_| format!("{}=set", field.name));
            }
            if before == after {
                return None;
            }
            if audit_sensitive_field(&field.name) {
                return Some(format!("{}=changed", field.name));
            }
            Some(format!(
                "{}:{}->{}",
                field.name,
                before.map_or("<none>", String::as_str),
                after.map_or("<none>", String::as_str)
            ))
        })
        .map(|change| audit_component(&change))
        .collect::<Vec<_>>();
    let table = form
        .table
        .as_ref()
        .map_or("unknown", |table| table.name.as_str());
    let changes = if changes.is_empty() {
        "none".into()
    } else {
        changes.join(",")
    };
    (
        record_id,
        format!(
            "table={};operation={};record_id={};changes={}",
            audit_component(table),
            audit_component(event),
            record_id.map_or_else(|| "<unknown>".into(), |id| id.to_string()),
            changes
        ),
    )
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
    let sessions = if let Some(session_table) = administrable_session_table(auth) {
        zelyra_database::execute_mariadb_query(
            database_url,
            &format!(
                "SELECT s.id, s.user_id, u.email, s.expires_at, {} FROM {} AS s INNER JOIN {} AS u ON u.id = s.user_id WHERE s.expires_at > CURRENT_TIMESTAMP ORDER BY s.expires_at, s.id LIMIT 100",
                if auth_session_has_column(auth, "device_label") { "s.device_label" } else { "''" },
                quote_identifier(session_table),
                quote_identifier(&auth.table),
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
        sessions,
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
    if !verify_csrf_request(
        request,
        &auth.csrf,
        input.get("_zelyra_csrf").map(String::as_str),
    ) {
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
        "revoke_session" => {
            let Some(session_table) = administrable_session_table(auth) else {
                return Response::html(
                    409,
                    "<h1>409 Conflict</h1><p>Session administration requires a persistent session table with an id column.</p>",
                );
            };
            let Some(session_id) = input
                .get("session_id")
                .and_then(|value| value.parse::<i64>().ok())
                .filter(|id| *id > 0)
            else {
                return Response::html(
                    422,
                    "<h1>422 Unprocessable Entity</h1><p>A valid session ID is required.</p>",
                );
            };
            let Some(user_id) = user_id.filter(|id| *id > 0) else {
                return Response::html(
                    422,
                    "<h1>422 Unprocessable Entity</h1><p>A valid user ID is required.</p>",
                );
            };
            execute_auth_admin_mutation(
                auth,
                database_url,
                vec![zelyra_database::Query {
                    sql: format!(
                        "DELETE FROM {} WHERE id = :session_id AND user_id = :user_id",
                        quote_identifier(session_table)
                    ),
                    params: vec![
                        (
                            "session_id".into(),
                            zelyra_database::QueryValue::Int(session_id),
                        ),
                        ("user_id".into(), zelyra_database::QueryValue::Int(user_id)),
                    ],
                }],
                actor_user_id,
                "auth.session_revoke_requested",
                Some(user_id),
                &format!("session_id={session_id}"),
            )
        }
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
    sessions: &[Vec<String>],
    language: UiLanguage,
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
        "<main><h1>{}</h1><h2>{}</h2><form method=\"post\" action=\"{path}\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{csrf}\"><input type=\"hidden\" name=\"operation\" value=\"create_user\"><label>{}</label><input name=\"email\" type=\"email\" required><label>{}</label><input name=\"password\" type=\"password\" minlength=\"8\" required><button type=\"submit\">{}</button></form><table><tr><th>{}</th><th>{}</th><th>{}</th></tr>",
        tr(language, "auth.admin_title"),
        tr(language, "auth.users_title"),
        tr(language, "auth.email"),
        tr(language, "auth.initial_password"),
        tr(language, "auth.create_user"),
        tr(language, "auth.email"),
        tr(language, "auth.status"),
        tr(language, "auth.actions"),
    );
    for row in users {
        if let (Some(user_id), Some(email), Some(active)) = (row.first(), row.get(1), row.get(2)) {
            let is_active = matches!(active.as_str(), "1" | "true" | "TRUE");
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td><form method=\"post\" action=\"{}\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{}\"><input type=\"hidden\" name=\"operation\" value=\"reset_password\"><input type=\"hidden\" name=\"user_id\" value=\"{}\"><input name=\"password\" type=\"password\" minlength=\"8\" required placeholder=\"{}\"><button type=\"submit\">{}</button></form>{}</td></tr>",
                html_escape(email),
                tr(language, if is_active { "auth.active" } else { "auth.inactive" }),
                path,
                csrf,
                html_escape(user_id),
                tr(language, "auth.new_password_placeholder"),
                tr(language, "auth.reset_password"),
                if active_supported {
                    format!(
                        "<form method=\"post\" action=\"{}\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{}\"><input type=\"hidden\" name=\"operation\" value=\"{}\"><input type=\"hidden\" name=\"user_id\" value=\"{}\"><button type=\"submit\">{}</button></form>",
                        path,
                        csrf,
                        if is_active { "deactivate_user" } else { "activate_user" },
                        html_escape(user_id),
                        tr(language, if is_active { "auth.deactivate" } else { "auth.activate" }),
                    )
                } else {
                    String::new()
                },
            ));
        }
    }
    html.push_str("</table>");
    if administrable_session_table(auth).is_some() {
        html.push_str(&render_auth_sessions(auth, sessions, language));
    }
    html.push_str("<h2>");
    html.push_str(&tr(language, "auth.assign_role"));
    html.push_str("</h2><form method=\"post\" action=\"");
    html.push_str(&path);
    html.push_str("\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"");
    html.push_str(&csrf);
    html.push_str("\"><input type=\"hidden\" name=\"operation\" value=\"grant_role\"><label>");
    html.push_str(&tr(language, "auth.user_id"));
    html.push_str("</label><input name=\"user_id\" type=\"number\" required><label>");
    html.push_str(&tr(language, "auth.role"));
    html.push_str("</label><input name=\"role\" required><button type=\"submit\">");
    html.push_str(&tr(language, "auth.grant_role"));
    html.push_str("</button></form><h2>");
    html.push_str(&tr(language, "auth.role_assignments"));
    html.push_str("</h2><table><tr><th>");
    html.push_str(&tr(language, "auth.user"));
    html.push_str("</th><th>");
    html.push_str(&tr(language, "auth.role"));
    html.push_str("</th><th>");
    html.push_str(&tr(language, "auth.action"));
    html.push_str("</th></tr>");
    for row in assignments {
        if let (Some(user_id), Some(email), Some(role)) = (row.first(), row.get(1), row.get(2)) {
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td><form method=\"post\" action=\"{}\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{}\"><input type=\"hidden\" name=\"operation\" value=\"revoke_role\"><input type=\"hidden\" name=\"user_id\" value=\"{}\"><input type=\"hidden\" name=\"role\" value=\"{}\"><button type=\"submit\">{}</button></form></td></tr>",
                html_escape(email),
                html_escape(role),
                path,
                csrf,
                html_escape(user_id),
                html_escape(role),
                tr(language, "auth.revoke"),
            ));
        }
    }
    html.push_str("</table><h2>");
    html.push_str(&tr(language, "auth.role_permissions"));
    html.push_str("</h2><form method=\"post\" action=\"");
    html.push_str(&path);
    html.push_str("\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"");
    html.push_str(&csrf);
    html.push_str(
        "\"><input type=\"hidden\" name=\"operation\" value=\"grant_permission\"><label>",
    );
    html.push_str(&tr(language, "auth.role"));
    html.push_str("</label><input name=\"role\" required><label>");
    html.push_str(&tr(language, "auth.permission"));
    html.push_str("</label><input name=\"permission\" required><button type=\"submit\">");
    html.push_str(&tr(language, "auth.grant_permission"));
    html.push_str("</button></form><table><tr><th>");
    html.push_str(&tr(language, "auth.role"));
    html.push_str("</th><th>");
    html.push_str(&tr(language, "auth.permission"));
    html.push_str("</th><th>");
    html.push_str(&tr(language, "auth.action"));
    html.push_str("</th></tr>");
    for row in permissions {
        if let (Some(role), Some(permission)) = (row.first(), row.get(1)) {
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td><form method=\"post\" action=\"{}\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{}\"><input type=\"hidden\" name=\"operation\" value=\"revoke_permission\"><input type=\"hidden\" name=\"role\" value=\"{}\"><input type=\"hidden\" name=\"permission\" value=\"{}\"><button type=\"submit\">{}</button></form></td></tr>",
                html_escape(role),
                html_escape(permission),
                path,
                csrf,
                html_escape(role),
                html_escape(permission),
                tr(language, "auth.revoke"),
            ));
        }
    }
    if auth.audit_table.is_some() {
        html.push_str("</table><h2>");
        html.push_str(&tr(language, "auth.audit_log"));
        html.push_str("</h2><p>");
        html.push_str(&tr(language, "auth.audit_latest"));
        html.push_str("</p><table><tr><th>");
        html.push_str(&tr(language, "auth.actor"));
        html.push_str("</th><th>");
        html.push_str(&tr(language, "auth.action"));
        html.push_str("</th><th>");
        html.push_str(&tr(language, "auth.target"));
        html.push_str("</th><th>");
        html.push_str(&tr(language, "auth.details"));
        html.push_str("</th><th>");
        html.push_str(&tr(language, "auth.created"));
        html.push_str("</th></tr>");
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

fn administrable_session_table(auth: &AuthRoute) -> Option<&str> {
    let name = auth.session_table.as_deref()?;
    auth.schema
        .tables
        .iter()
        .find(|table| table.name == name && table.columns.iter().any(|column| column.name == "id"))
        .map(|_| name)
}

fn auth_session_has_column(auth: &AuthRoute, name: &str) -> bool {
    auth.session_table.as_deref().is_some_and(|table_name| {
        auth.schema
            .tables
            .iter()
            .find(|table| table.name == table_name)
            .is_some_and(|table| table.columns.iter().any(|column| column.name == name))
    })
}

fn request_device_metadata(request: &Request) -> Option<String> {
    let value = request.headers.get("user-agent")?.trim();
    if value.is_empty() {
        return None;
    }
    let mut result = String::with_capacity(value.len().min(255));
    for character in value.chars().filter(|character| !character.is_control()) {
        if result.len() + character.len_utf8() > 255 {
            break;
        }
        result.push(character);
    }
    (!result.is_empty()).then_some(result)
}

fn render_auth_sessions(
    auth: &AuthRoute,
    sessions: &[Vec<String>],
    language: UiLanguage,
) -> String {
    let path = html_escape(auth.admin_path.as_deref().unwrap_or("/"));
    let csrf = html_escape(auth.csrf.token());
    let mut html = format!(
        "<h2>{}</h2><p>{}</p><table><tr><th>ID</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr>",
        tr(language, "auth.sessions"),
        tr(language, "auth.sessions_help"),
        tr(language, "auth.email"),
        tr(language, "auth.session_expires"),
        tr(language, "auth.session_device"),
        tr(language, "auth.actions")
    );
    for row in sessions {
        if let [id, user_id, email, expires, ..] = row.as_slice() {
            let device = row
                .get(4)
                .map(String::as_str)
                .filter(|device| !device.is_empty() && *device != "NULL")
                .map(html_escape)
                .unwrap_or_else(|| tr(language, "auth.session_device_unknown"));
            html.push_str(&format!("<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><form method=\"post\" action=\"{path}\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{csrf}\"><input type=\"hidden\" name=\"operation\" value=\"revoke_session\"><input type=\"hidden\" name=\"session_id\" value=\"{}\"><input type=\"hidden\" name=\"user_id\" value=\"{}\"><button type=\"submit\">{}</button></form></td></tr>",
                html_escape(id), html_escape(email), html_escape(expires), device, html_escape(id), html_escape(user_id), tr(language, "auth.revoke_session")));
        }
    }
    html.push_str("</table>");
    html
}

#[cfg(test)]
fn account_sessions_test_auth() -> AuthRoute {
    AuthRoute {
        table: "users".into(),
        membership_table: None,
        session_table: None,
        permissions_table: None,
        roles_table: None,
        role_permissions_table: None,
        audit_table: None,
        audit_chain: false,
        admin_path: None,
        admin_permission: None,
        admin_role: None,
        login_rate_limit: DEFAULT_LOGIN_RATE_LIMIT,
        login_block_seconds: DEFAULT_LOGIN_BLOCK_SECONDS,
        reset_tokens_table: None,
        reset_rate_limit: DEFAULT_RESET_RATE_LIMIT,
        reset_block_seconds: DEFAULT_RESET_BLOCK_SECONDS,
        mfa_table: None,
        mfa_recovery_table: None,
        schema: Schema {
            database: None,
            tables: Vec::new(),
        },
        csrf: CsrfProtection::new("sessions-csrf"),
    }
}

#[test]
fn account_sessions_are_scoped_to_the_signed_in_user_and_hide_bearer_tokens() {
    let app = WebApp::new(Vec::new(), Vec::new()).with_auth_route(account_sessions_test_auth());
    {
        let mut sessions = app.sessions.lock().unwrap();
        for (token, user_id) in [
            ("current-bearer", 7),
            ("other-bearer", 7),
            ("foreign-bearer", 8),
        ] {
            sessions.insert(
                token.into(),
                Session {
                    user_id: Some(user_id),
                    permissions: Vec::new(),
                    expires_at: Instant::now() + Duration::from_secs(3600),
                    device: None,
                },
            );
        }
    }
    let request = parse_request(
        "GET /account/sessions HTTP/1.1\r\nHost: localhost\r\nCookie: zelyra_session=current-bearer\r\n\r\n",
    )
    .unwrap();
    let response = app.dispatch(&request);
    assert_eq!(response.status, 200);
    assert!(response.body.contains("Your sessions"));
    assert!(response.body.contains("Session (current)"));
    assert!(response
        .body
        .contains(&format!("hash:{}", session_token_hash("other-bearer"))));
    assert!(!response.body.contains("current-bearer"));
    assert!(!response.body.contains("other-bearer"));
    assert!(!response
        .body
        .contains(&session_token_hash("foreign-bearer")));
    let german_app = app.with_ui_settings(UiLanguage::German, UiLevel::Work);
    let german_response = german_app.dispatch(&request);
    assert!(german_response.body.contains("Deine Sitzungen"));
    assert!(german_response.body.contains("Sekunden verbleibend"));
    assert!(german_response.body.contains("(aktuell)"));
}

#[test]
fn account_sessions_revoke_only_owned_sessions_and_sign_out_if_current_is_revoked() {
    let app = WebApp::new(Vec::new(), Vec::new()).with_auth_route(account_sessions_test_auth());
    {
        let mut sessions = app.sessions.lock().unwrap();
        for (token, user_id) in [
            ("current-bearer", 7),
            ("other-bearer", 7),
            ("foreign-bearer", 8),
        ] {
            sessions.insert(
                token.into(),
                Session {
                    user_id: Some(user_id),
                    permissions: Vec::new(),
                    expires_at: Instant::now() + Duration::from_secs(3600),
                    device: None,
                },
            );
        }
    }
    let other_key = format!("hash:{}", session_token_hash("other-bearer"));
    let other_body = format!("_zelyra_csrf=sessions-csrf&session_key={other_key}");
    let other_request = parse_request(&format!(
        "POST /account/sessions HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\nCookie: zelyra_session=current-bearer\r\nContent-Length: {}\r\n\r\n{}",
        other_body.len(), other_body
    ))
    .unwrap();
    let response = app.dispatch(&other_request);
    assert_eq!(response.status, 303);
    assert_eq!(response.location.as_deref(), Some(ACCOUNT_SESSIONS_PATH));
    let sessions = app.sessions.lock().unwrap();
    assert!(sessions.contains_key("current-bearer"));
    assert!(!sessions.contains_key("other-bearer"));
    assert!(sessions.contains_key("foreign-bearer"));
    drop(sessions);

    let foreign_key = format!("hash:{}", session_token_hash("foreign-bearer"));
    let foreign_body = format!("_zelyra_csrf=sessions-csrf&session_key={foreign_key}");
    let foreign_request = parse_request(&format!(
        "POST /account/sessions HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\nCookie: zelyra_session=current-bearer\r\nContent-Length: {}\r\n\r\n{}",
        foreign_body.len(), foreign_body
    ))
    .unwrap();
    app.dispatch(&foreign_request);
    assert!(app.sessions.lock().unwrap().contains_key("foreign-bearer"));

    let current_key = format!("hash:{}", session_token_hash("current-bearer"));
    let current_body = format!("_zelyra_csrf=sessions-csrf&session_key={current_key}");
    let current_request = parse_request(&format!(
        "POST /account/sessions HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\nCookie: zelyra_session=current-bearer\r\nContent-Length: {}\r\n\r\n{}",
        current_body.len(), current_body
    ))
    .unwrap();
    let response = app.dispatch(&current_request);
    assert_eq!(response.location.as_deref(), Some("/login"));
    assert!(response.headers.iter().any(
        |(name, value)| name.eq_ignore_ascii_case("set-cookie") && value.contains("Max-Age=0")
    ));
    assert!(!app.sessions.lock().unwrap().contains_key("current-bearer"));
}

#[test]
fn account_session_revocation_requires_valid_csrf() {
    let app = WebApp::new(Vec::new(), Vec::new()).with_auth_route(account_sessions_test_auth());
    app.sessions.lock().unwrap().insert(
        "current-bearer".into(),
        Session {
            user_id: Some(7),
            permissions: Vec::new(),
            expires_at: Instant::now() + Duration::from_secs(3600),
            device: None,
        },
    );
    let key = format!("hash:{}", session_token_hash("current-bearer"));
    let body = format!("_zelyra_csrf=wrong&session_key={key}");
    let request = parse_request(&format!(
        "POST /account/sessions HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\nCookie: zelyra_session=current-bearer\r\nContent-Length: {}\r\n\r\n{}",
        body.len(), body
    ))
    .unwrap();
    assert_eq!(app.dispatch(&request).status, 403);
    assert!(app.sessions.lock().unwrap().contains_key("current-bearer"));

    let body = format!("_zelyra_csrf=sessions-csrf&session_key={key}");
    let cross_origin = parse_request(&format!(
        "POST /account/sessions HTTP/1.1\r\nHost: localhost\r\nOrigin: https://attacker.test\r\nCookie: zelyra_session=current-bearer\r\nContent-Length: {}\r\n\r\n{}",
        body.len(), body
    ))
    .unwrap();
    assert_eq!(app.dispatch(&cross_origin).status, 403);
    assert!(app.sessions.lock().unwrap().contains_key("current-bearer"));
}

fn memory_session(app: &WebApp, session_id: &str) -> Option<Session> {
    let mut sessions = app.sessions.lock().ok()?;
    let now = Instant::now();
    sessions.retain(|_, session| session.expires_at > now);
    sessions.get(session_id).cloned()
}

fn session_from_request(
    app: &WebApp,
    request: &Request,
    database_url: Option<&str>,
) -> Option<Session> {
    let session_id = cookie_value(request, "zelyra_session")?;
    let Some(auth) = &app.auth_route else {
        return memory_session(app, &session_id);
    };
    let (Some(session_table), Some(database_url)) = (&auth.session_table, database_url) else {
        return memory_session(app, &session_id);
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
    let mfa_clause = if auth.mfa_table.is_some() {
        " AND s.mfa_verified = true"
    } else {
        ""
    };
    let query = if active_clause {
        format!(
            "SELECT s.user_id FROM {} AS s INNER JOIN {} AS u ON u.id = s.user_id WHERE s.token_hash = :token_hash AND s.expires_at > CURRENT_TIMESTAMP AND u.active = true{mfa_clause} LIMIT 1",
            quote_identifier(session_table),
            quote_identifier(&auth.table),
        )
    } else if auth.mfa_table.is_some() {
        format!(
            "SELECT s.user_id FROM {} AS s WHERE s.token_hash = :token_hash AND s.expires_at > CURRENT_TIMESTAMP{mfa_clause} LIMIT 1",
            quote_identifier(session_table),
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
        expires_at: Instant::now() + SESSION_LIFETIME,
        device: None,
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
        self.dispatch_with_database(method, path, None)
    }

    pub fn dispatch_with_database(
        &self,
        method: &str,
        path: &str,
        database_url: Option<&str>,
    ) -> Response {
        self.dispatch_with_database_and_language(method, path, database_url, UiLanguage::English)
    }

    pub fn dispatch_with_database_and_language(
        &self,
        method: &str,
        path: &str,
        database_url: Option<&str>,
        language: UiLanguage,
    ) -> Response {
        self.dispatch_with_database_language_and_catalogs(
            method,
            path,
            database_url,
            language,
            &ProjectUiCatalogs::default(),
        )
    }

    fn dispatch_with_database_language_and_catalogs(
        &self,
        method: &str,
        path: &str,
        database_url: Option<&str>,
        language: UiLanguage,
        project_catalogs: &ProjectUiCatalogs,
    ) -> Response {
        if method != "GET" {
            return Response::html(405, "<h1>405 Method Not Allowed</h1>");
        }
        let target = path;
        let path = target.split_once('?').map_or(target, |(path, _)| path);
        for route in &self.routes {
            if let Some(params) = match_path(&route.path, path) {
                let query_string = target.split_once('?').map_or("", |(_, query)| query);
                let query_values = match parse_urlencoded(query_string) {
                    Ok(values) => values,
                    Err(error) => {
                        return Response::html(
                            400,
                            format!(
                                "<h1>400 Bad Request</h1><p>{}</p>",
                                html_escape(&error.message)
                            ),
                        )
                    }
                };
                let data = match load_route_data(route, &params, &query_values, database_url) {
                    Ok(data) => data,
                    Err(RouteDataError::NotFound) => {
                        return Response::html(404, "<h1>404 Not Found</h1>");
                    }
                    Err(RouteDataError::DatabaseUnavailable) => {
                        return Response::html(
                            503,
                            "<h1>503 Service Unavailable</h1><p>Database is unavailable.</p>",
                        );
                    }
                    Err(RouteDataError::Query) => {
                        return Response::html(
                            500,
                            "<h1>500 Internal Server Error</h1><p>Page data could not be loaded.</p>",
                        );
                    }
                    Err(RouteDataError::InvalidQuery(message)) => {
                        return Response::html(
                            400,
                            format!("<h1>400 Bad Request</h1><p>{}</p>", html_escape(&message)),
                        );
                    }
                };
                let html =
                    render_page_with_language(route, path, &params, &query_values, &data, language);
                return Response::html(
                    200,
                    localize_html_with_catalog(&html, language, project_catalogs),
                );
            }
        }
        Response::html(404, "<h1>404 Not Found</h1>")
    }
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
            '\u{e000}' => escaped.push_str("&#xe000;"),
            '\u{e001}' => escaped.push_str("&#xe001;"),
            '\u{e002}' => escaped.push_str("&#xe002;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn html_escape_preserving_locale_references(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    let mut remaining = value;
    while let Some(start) = remaining.find(LOCALE_REFERENCE_START) {
        escaped.push_str(&html_escape(&remaining[..start]));
        let reference_start = start + LOCALE_REFERENCE_START.len();
        let Some(end_relative) = remaining[reference_start..].find(LOCALE_REFERENCE_END) else {
            escaped.push_str(&html_escape(&remaining[start..]));
            return escaped;
        };
        let end = reference_start + end_relative;
        let token_end = end + LOCALE_REFERENCE_END.len_utf8();
        let reference = &remaining[reference_start..end];
        let (key, parameter) = reference
            .split_once(LOCALE_REFERENCE_PARAMETER)
            .map_or((reference, None), |(key, value)| (key, Some(value)));
        let valid_parameter = match parameter {
            None => true,
            Some(value) => value.split_once('=').is_some_and(|(name, encoded)| {
                i18n::valid_catalog_key(name) && decode_locale_parameter(encoded).is_some()
            }),
        };
        if i18n::valid_catalog_key(key) && valid_parameter {
            escaped.push_str(&remaining[start..token_end]);
        } else {
            escaped.push_str(&html_escape(&remaining[start..token_end]));
        }
        remaining = &remaining[token_end..];
    }
    escaped.push_str(&html_escape(remaining));
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

#[cfg(test)]
fn dispatch_form(
    form: &FormRoute,
    request: &Request,
    path_params: &HashMap<String, String>,
    database_url: Option<&str>,
    actor_user_id: Option<i64>,
) -> Response {
    dispatch_form_with_language(
        form,
        request,
        path_params,
        database_url,
        actor_user_id,
        None,
        UiLanguage::English,
    )
}

fn dispatch_form_with_language(
    form: &FormRoute,
    request: &Request,
    path_params: &HashMap<String, String>,
    database_url: Option<&str>,
    actor_user_id: Option<i64>,
    tenant_context: Option<&TenantContext>,
    language: UiLanguage,
) -> Response {
    if form.tenant_column.is_some() && tenant_context.is_none() {
        return Response::html(403, "<h1>403 Forbidden</h1>");
    }
    let confirmation_view = form
        .form
        .actions
        .first()
        .and_then(|action| action.confirm_page.as_ref());
    if request.method == "GET" {
        if let Some(confirmation_view) = confirmation_view {
            let rendered_form = form_with_path_params(form, path_params);
            let options = match load_relation_options(&rendered_form, database_url) {
                Ok(options) => options,
                Err(error) => return relation_options_error(database_url, error),
            };
            return Response::html(
                200,
                render_action_confirmation(&rendered_form, confirmation_view, &options, language),
            );
        }
    }
    if form.post_only && request.method != "POST" {
        return Response::html(405, "<h1>405 Method Not Allowed</h1>");
    }
    let rendered_form = form_with_path_params(form, path_params);
    if request.method == "GET" {
        let options = match load_relation_options(&rendered_form, database_url) {
            Ok(options) => options,
            Err(error) => return relation_options_error(database_url, error),
        };
        let values = if rendered_form.form.name.ends_with("Edit") {
            match load_existing_form_values(
                &rendered_form,
                path_params,
                database_url,
                tenant_context,
            ) {
                Ok(Some(values)) => values,
                Ok(None) => return Response::html(404, "<h1>404 Not Found</h1>"),
                Err(error) => return relation_options_error(database_url, error),
            }
        } else {
            HashMap::new()
        };
        let mut html =
            render_form_with_language(&rendered_form, &values, &[], None, &options, language);
        if is_edit_form(form) {
            let Some(snapshot) = sign_form_snapshot(&form.csrf, &values) else {
                return Response::html(500, "<h1>500 Internal Server Error</h1>");
            };
            html = attach_form_snapshot(&html, &snapshot);
        }
        return Response::html(200, html);
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
    if !verify_csrf_request(
        request,
        &form.csrf,
        input.get("_zelyra_csrf").map(String::as_str),
    ) {
        return Response::html(403, "<h1>403 Forbidden</h1><p>Invalid CSRF token.</p>");
    }
    let mut values = input;
    values.remove("_zelyra_csrf");
    let expected_snapshot = if is_edit_form(form) {
        let Some(snapshot) = values
            .remove("_zelyra_snapshot")
            .and_then(|token| verify_form_snapshot(&token))
        else {
            return Response::html(
                409,
                "<main><h1>409 Conflict</h1><p>This record changed while you were editing it. Reload the form and review the latest values.</p></main>",
            );
        };
        Some(snapshot)
    } else {
        None
    };
    for field in &rendered_form.form.fields {
        if input_type(&rendered_form, field) == "checkbox" {
            values
                .entry(field.name.clone())
                .or_insert_with(|| "false".into());
        }
    }
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
            render_form_with_language(
                &rendered_form,
                &values,
                &errors,
                Some("@i18n:form.correct_errors"),
                &relation_options,
                language,
            ),
        );
    }
    if let Some(action) = form.form.actions.first() {
        let Some(database_url) = database_url else {
            return action_error_response(
                action,
                503,
                "Service Unavailable",
                "DATABASE_URL is required for this form action.",
            );
        };
        if let Err(error) = execute_form_action(
            form,
            action,
            &values,
            path_params,
            database_url,
            FormExecutionContext {
                actor_user_id,
                expected_snapshot: expected_snapshot.as_deref(),
                tenant_context,
            },
        ) {
            eprintln!("zelyra web: form action failed: {error}");
            if error.starts_with("ZELYRA_TENANT_DENIED:") {
                return Response::html(404, "<h1>404 Not Found</h1>");
            }
            if error.starts_with("ZELYRA_CONFLICT:") {
                return action_error_response(
                    action,
                    409,
                    "409 Conflict",
                    "This record changed while you were editing it. Reload the form and review the latest values.",
                );
            }
            return action_error_response(
                action,
                500,
                "Internal Server Error",
                "The action could not be completed.",
            );
        }
        let mut redirect = action.redirect.as_deref().unwrap_or("/").to_owned();
        if let Some(success) = action_success_message(action) {
            // Keep catalog keys intact across the redirect. The destination
            // resolves them with the active project catalog when it renders.
            redirect = append_query_parameter(&redirect, "zelyra_success", success);
        }
        if let Some(title) = action
            .success_page
            .as_ref()
            .and_then(|page| page.title.as_deref())
        {
            redirect = append_query_parameter(&redirect, "zelyra_success_title", title);
        }
        return Response::redirect(redirect);
    }
    Response::html(
        202,
        render_form_with_language(
            &rendered_form,
            &values,
            &[],
            Some("@i18n:form.validated_not_enabled"),
            &relation_options,
            language,
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

fn render_action_confirmation(
    route: &FormRoute,
    view: &zelyra_ast::CrudConfirmViewDef,
    relation_options: &HashMap<String, Vec<SelectOption>>,
    language: UiLanguage,
) -> String {
    let title = view
        .title
        .as_deref()
        .map(|title| localize_user_text(language, title))
        .unwrap_or_else(|| tr(language, "confirm.title"));
    let message = view
        .message
        .as_deref()
        .map(|message| localize_user_text(language, message))
        .unwrap_or_else(|| tr(language, "confirm.message"));
    let submit = view
        .submit
        .clone()
        .unwrap_or_else(|| "@i18n:confirm.submit".to_owned());
    let mut confirmation_form = route.clone();
    confirmation_form.form_view.title = None;
    confirmation_form.form_view.submit = Some(submit);
    format!(
        "<main class=\"zelyra-action-confirmation\"><h1>{}</h1><p>{}</p>{}</main>",
        html_escape_preserving_locale_references(&title),
        html_escape_preserving_locale_references(&message),
        render_form_with_language(
            &confirmation_form,
            &HashMap::new(),
            &[],
            None,
            relation_options,
            language,
        )
    )
}

fn action_success_message(action: &zelyra_ast::FormAction) -> Option<&str> {
    action.success.as_deref().or_else(|| {
        action.success_page.as_ref().map(|page| {
            page.message
                .as_deref()
                .unwrap_or("Action completed successfully.")
        })
    })
}

fn action_error_response(
    action: &zelyra_ast::FormAction,
    status: u16,
    default_title: &str,
    default_message: &str,
) -> Response {
    let title = action
        .error_page
        .as_ref()
        .and_then(|page| page.title.as_deref())
        .unwrap_or(default_title);
    let message = action
        .error_page
        .as_ref()
        .and_then(|page| page.message.as_deref())
        .unwrap_or(default_message);
    Response::html(
        status,
        format!(
            "<main class=\"zelyra-action-error\"><h1>{}</h1><p>{}</p></main>",
            html_escape(title),
            html_escape(message)
        ),
    )
}

fn load_existing_form_values(
    form: &FormRoute,
    path_params: &HashMap<String, String>,
    database_url: Option<&str>,
    tenant_context: Option<&TenantContext>,
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
    let columns = form_value_columns(form, schema_table);
    let tenant_condition = match (form.tenant_column.as_deref(), tenant_context) {
        (Some(column), Some(context)) => format!(
            " AND {} = :zelyra_tenant_id AND {}",
            quote_identifier(column),
            tenant_membership_predicate(context)
        ),
        (Some(_), None) => return Err("tenant context is required for this form".into()),
        _ => String::new(),
    };
    let query = format!(
        "SELECT {} FROM {} WHERE {} = :id{}",
        columns
            .iter()
            .map(|column| quote_identifier(column))
            .collect::<Vec<_>>()
            .join(", "),
        quote_identifier(table_name),
        quote_identifier("id"),
        tenant_condition
    );
    let mut params = vec![("id".into(), zelyra_database::QueryValue::Int(id))];
    if let Some(tenant_context) = tenant_context {
        params.extend(tenant_query_params(tenant_context));
    }
    let result = zelyra_database::execute_mariadb_query(database_url, &query, params)
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

fn form_value_columns(form: &FormRoute, schema_table: &zelyra_database::Table) -> Vec<String> {
    form.form
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
        .collect()
}

fn is_edit_form(form: &FormRoute) -> bool {
    form.form.name.ends_with("Edit")
}

fn sign_form_snapshot(csrf: &CsrfProtection, values: &HashMap<String, String>) -> Option<String> {
    let signature = form_snapshot_signature(csrf, values)?;
    Some(hex_encode(&signature))
}

fn form_snapshot_signature(
    csrf: &CsrfProtection,
    values: &HashMap<String, String>,
) -> Option<Vec<u8>> {
    let snapshot = values.iter().collect::<BTreeMap<_, _>>();
    let payload = serde_json::to_vec(&snapshot).ok()?;
    let key = hmac::Key::new(hmac::HMAC_SHA256, csrf.token.as_bytes());
    Some(hmac::sign(&key, &payload).as_ref().to_vec())
}

fn verify_form_snapshot(token: &str) -> Option<Vec<u8>> {
    if token.len() != 64 {
        return None;
    }
    let signature = hex_decode(token)?;
    if signature.len() != 32 {
        return None;
    }
    Some(signature)
}

fn form_snapshot_matches(
    csrf: &CsrfProtection,
    expected_signature: &[u8],
    values: &HashMap<String, String>,
) -> bool {
    let Some(snapshot) = form_snapshot_signature(csrf, values) else {
        return false;
    };
    constant_time_equal(expected_signature, &snapshot)
}

fn attach_form_snapshot(html: &str, snapshot: &str) -> String {
    let hidden =
        format!("<input type=\"hidden\" name=\"_zelyra_snapshot\" value=\"{snapshot}\" />");
    if let Some(index) = html.rfind("</form>") {
        let mut output = String::with_capacity(html.len() + hidden.len());
        output.push_str(&html[..index]);
        output.push_str(&hidden);
        output.push_str(&html[index..]);
        output
    } else {
        html.to_owned()
    }
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

fn crud_column_label(
    language: UiLanguage,
    schema: &Schema,
    table_name: &str,
    column: &str,
) -> String {
    let Some(table) = schema.tables.iter().find(|table| table.name == table_name) else {
        return localized_identifier_reference(language, column);
    };
    if crud_foreign_key(table, column).is_some() {
        return localized_identifier_reference(
            language,
            column.strip_suffix("_id").unwrap_or(column),
        );
    }
    localized_identifier_reference(language, column)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CrudUiActions {
    create: bool,
    edit: bool,
    delete: bool,
    restore: bool,
    custom: Vec<CrudUiActionLink>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CrudUiActionLink {
    label: String,
    icon: Option<String>,
    path: String,
    csrf: String,
    confirm: Option<String>,
    confirm_page: Option<zelyra_ast::CrudConfirmViewDef>,
    fields: Vec<CrudUiActionField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CrudUiActionField {
    name: String,
    label: String,
    input_type: String,
    required: bool,
    max: Option<u32>,
    relation: bool,
    options: Vec<SelectOption>,
}

fn crud_ui_actions(crud: &CrudRoute, request: &Request, app: &WebApp) -> CrudUiActions {
    let detail_path = format!("{}/{{id}}", crud.path.trim_end_matches('/'));
    let include_relation_options = match_path(&detail_path, &request.path).is_some();
    let archived = request
        .target
        .split_once('?')
        .and_then(|(_, query)| parse_urlencoded(query).ok())
        .and_then(|values| values.get("archived").cloned())
        .is_some_and(|value| matches!(value.as_str(), "true" | "1"));
    let delete_authorized =
        can_authorize(crud.requires_auth, &crud.delete_permissions, request, app);
    CrudUiActions {
        create: can_authorize(crud.requires_auth, &crud.create_permissions, request, app),
        edit: !archived && can_authorize(crud.requires_auth, &crud.edit_permissions, request, app),
        delete: delete_authorized && !archived,
        restore: archived && crud.soft_delete.is_some() && delete_authorized,
        custom: crud
            .actions
            .iter()
            .filter_map(|action| {
                let (requires_auth, permissions) = form_authorization(&action.form);
                if !can_authorize(requires_auth, &permissions, request, app) {
                    return None;
                }
                let relation_options = if include_relation_options {
                    load_relation_options(&action.form, app.database_url.as_deref())
                        .unwrap_or_default()
                } else {
                    HashMap::new()
                };
                Some(CrudUiActionLink {
                    label: action.label.clone(),
                    icon: action.icon.clone(),
                    path: action.form.path.clone(),
                    csrf: action.form.csrf.token().into(),
                    confirm: action.confirm.clone(),
                    confirm_page: action.confirm_page.clone(),
                    fields: action
                        .form
                        .form
                        .fields
                        .iter()
                        .map(|field| CrudUiActionField {
                            name: field.name.clone(),
                            label: field.label.clone().unwrap_or_else(|| humanize(&field.name)),
                            input_type: input_type(&action.form, field).into(),
                            required: is_required(&action.form, field),
                            max: field_max(&action.form, field),
                            relation: relation_target_for_field(&action.form, field).is_some(),
                            options: relation_options
                                .get(&field.name)
                                .cloned()
                                .unwrap_or_default(),
                        })
                        .collect(),
                })
            })
            .collect(),
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

fn crud_loading_attribute(crud: &CrudRoute) -> String {
    crud.loading_view
        .message
        .as_deref()
        .map(|message| format!(" data-loading-message=\"{}\"", html_escape(message)))
        .unwrap_or_default()
}

fn crud_error_response(
    crud: &CrudRoute,
    status: u16,
    default_title: &str,
    default_message: &str,
) -> Response {
    let title = crud.error_view.title.as_deref().unwrap_or(default_title);
    let message = crud
        .error_view
        .message
        .as_deref()
        .unwrap_or(default_message);
    let body = format!(
        "<main class=\"zelyra-crud-error\"{}><h1>{}</h1><p>{}</p></main>",
        crud_loading_attribute(crud),
        html_escape(title),
        html_escape(message)
    );
    Response::html(status, body)
}

fn dispatch_crud(
    crud: &CrudRoute,
    request: &Request,
    database_url: Option<&str>,
    ui_actions: CrudUiActions,
    tenant_context: Option<&TenantContext>,
    language: UiLanguage,
) -> Response {
    if request.method != "GET" {
        return Response::html(405, "<h1>405 Method Not Allowed</h1>");
    }
    let Some(database_url) = database_url else {
        return crud_error_response(
            crud,
            503,
            "Service Unavailable",
            "DATABASE_URL is required for CRUD lists.",
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
    let archived = query_values
        .get("archived")
        .is_some_and(|value| matches!(value.as_str(), "true" | "1"));
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
        return crud_error_response(
            crud,
            500,
            "Internal Server Error",
            "CRUD table is unavailable.",
        );
    };
    let all_columns = table
        .columns
        .iter()
        .filter(|column| crud.tenant_column.as_deref() != Some(column.name.as_str()))
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    if all_columns.is_empty() {
        return crud_error_response(
            crud,
            500,
            "Internal Server Error",
            "CRUD table has no columns.",
        );
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
    for (name, value) in sorted_filter_query_values(&query_values) {
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
    if let Some(soft_delete) = &crud.soft_delete {
        conditions.push(format!(
            "{} IS {}",
            crud_storage_expression(table, &soft_delete.column),
            if archived { "NOT NULL" } else { "NULL" }
        ));
    }
    if let (Some(tenant_column), Some(tenant_context)) =
        (crud.tenant_column.as_deref(), tenant_context)
    {
        conditions.push(format!(
            "{}.{} = :zelyra_tenant_id AND {}",
            quote_identifier("base"),
            quote_identifier(tenant_column),
            tenant_membership_predicate(tenant_context)
        ));
    } else if crud.tenant_column.is_some() {
        return Response::html(403, "<h1>403 Forbidden</h1>");
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
    if let Some(tenant_context) = tenant_context {
        params.extend(tenant_query_params(tenant_context));
    }
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
            return crud_error_response(
                crud,
                500,
                "Internal Server Error",
                "The requested data could not be loaded.",
            );
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
                archived,
                success: query_values.get("zelyra_success").map(String::as_str),
                success_title: query_values.get("zelyra_success_title").map(String::as_str),
            },
            ui_actions,
            language,
        ),
    )
}

fn dispatch_tableview(
    tableview: &TableViewRoute,
    request: &Request,
    database_url: Option<&str>,
    language: UiLanguage,
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
    for (name, value) in sorted_filter_query_values(&query_values) {
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
            filters.push((column.to_string(), value.to_string(), operator));
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
            language,
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
    language: UiLanguage,
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
    html.push_str("><fieldset class=\"zelyra-query-controls\"><legend>");
    html.push_str(&tr(language, "query.legend"));
    html.push_str("</legend>");
    if tableview.searchable {
        html.push_str("<label for=\"search\">");
        html.push_str(&tr(language, "query.search"));
        html.push_str("</label><input id=\"search\" name=\"search\" value=\"");
        html.push_str(&html_escape(search));
        html.push_str("\">");
    }
    if tableview.sortable {
        html.push_str("<label for=\"sort\">");
        html.push_str(&tr(language, "query.sort"));
        html.push_str("</label><select id=\"sort\" name=\"sort\">");
        for column in &tableview.columns {
            html.push_str("<option value=\"");
            html.push_str(&html_escape_preserving_locale_references(
                &localized_identifier_reference(language, column),
            ));
            html.push('"');
            if column == sort {
                html.push_str(" selected");
            }
            html.push('>');
            html.push_str(&html_escape(column));
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
    for filter in &tableview.filters {
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
    if rows.is_empty() {
        html.push_str("<p>");
        html.push_str(&tr(language, "table.empty"));
        html.push_str("</p>");
    } else {
        html.push_str("<table><thead><tr>");
        for column in &tableview.columns {
            html.push_str("<th>");
            html.push_str(&html_escape_preserving_locale_references(
                &localized_identifier_reference(language, column),
            ));
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
        html.push_str(&format!("\">{}</a> ", tr(language, "pagination.previous")));
    }
    html.push_str("<span>");
    html.push_str(&tr(language, "pagination.page"));
    html.push(' ');
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
        html.push_str(&format!("\">{}</a>", tr(language, "pagination.next")));
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
    for (name, value) in sorted_query_values(query_values) {
        if matches!(name, "page" | "sort" | "order" | "search") || value.is_empty() {
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
    tenant_context: Option<&TenantContext>,
    language: UiLanguage,
) -> Response {
    if request.method != "GET" {
        return Response::html(405, "<h1>405 Method Not Allowed</h1>");
    }
    let Some(database_url) = database_url else {
        return crud_error_response(
            crud,
            503,
            "Service Unavailable",
            "DATABASE_URL is required for CRUD details.",
        );
    };
    let Some(id) = path_params.get("id") else {
        return Response::html(400, "<h1>400 Bad Request</h1>");
    };
    let id_text = id.clone();
    let Ok(id) = id.parse::<i64>() else {
        return Response::html(400, "<h1>400 Bad Request</h1><p>id must be an integer.</p>");
    };
    let archived = request
        .target
        .split_once('?')
        .and_then(|(_, query)| parse_urlencoded(query).ok())
        .and_then(|values| values.get("archived").cloned())
        .is_some_and(|value| matches!(value.as_str(), "true" | "1"));
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
        .filter(|column| crud.tenant_column.as_deref() != Some(column.name.as_str()))
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    let soft_delete_condition = crud
        .soft_delete
        .as_ref()
        .map(|soft_delete| {
            format!(
                " AND {} IS {}",
                crud_storage_expression(table, &soft_delete.column),
                if archived { "NOT NULL" } else { "NULL" }
            )
        })
        .unwrap_or_default();
    let tenant_condition = match (crud.tenant_column.as_deref(), tenant_context) {
        (Some(tenant_column), Some(context)) => format!(
            " AND {}.{} = :zelyra_tenant_id AND {}",
            quote_identifier("base"),
            quote_identifier(tenant_column),
            tenant_membership_predicate(context)
        ),
        (Some(_), None) => return Response::html(403, "<h1>403 Forbidden</h1>"),
        (None, _) => String::new(),
    };
    let query = format!(
        "SELECT {} FROM {} WHERE {} = :id{}{}",
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
        format_args!("{}.{}", quote_identifier("base"), quote_identifier("id")),
        soft_delete_condition,
        tenant_condition
    );
    let mut params = vec![("id".into(), zelyra_database::QueryValue::Int(id))];
    if let Some(tenant_context) = tenant_context {
        params.extend(tenant_query_params(tenant_context));
    }
    let result = match zelyra_database::execute_mariadb_query(database_url, &query, params) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("zelyra web: CRUD detail query failed: {error}");
            return crud_error_response(
                crud,
                500,
                "Internal Server Error",
                "The requested record could not be loaded.",
            );
        }
    };
    let Some(row) = result.rows.first() else {
        return Response::html(404, "<h1>404 Not Found</h1>");
    };
    Response::html(
        200,
        render_crud_detail_with_actions(crud, &columns, row, &id_text, ui_actions, language),
    )
}

fn dispatch_crud_delete(
    crud: &CrudRoute,
    request: &Request,
    path_params: &HashMap<String, String>,
    context: CrudMutationContext<'_>,
) -> Response {
    let CrudMutationContext {
        database_url,
        audit_table,
        audit_chain,
        actor_user_id,
        tenant_context,
    } = context;
    if request.method != "POST" {
        return Response::html(405, "<h1>405 Method Not Allowed</h1>");
    }
    let Some(database_url) = database_url else {
        return crud_error_response(
            crud,
            503,
            "Service Unavailable",
            "DATABASE_URL is required for CRUD actions.",
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
    if !verify_csrf_request(
        request,
        &crud.csrf,
        input.get("_zelyra_csrf").map(String::as_str),
    ) {
        return Response::html(403, "<h1>403 Forbidden</h1><p>Invalid CSRF token.</p>");
    }
    if crud.tenant_column.is_some() && tenant_context.is_none() {
        return Response::html(403, "<h1>403 Forbidden</h1>");
    }
    let tenant_condition = match (crud.tenant_column.as_deref(), tenant_context) {
        (Some(column), Some(context)) => format!(
            " AND {} = :zelyra_tenant_id AND {}",
            quote_identifier(column),
            tenant_membership_predicate(context)
        ),
        _ => String::new(),
    };
    let scoped_record_check = match (crud.tenant_column.as_deref(), tenant_context) {
        (Some(tenant_column), Some(context)) => {
            let mut params = vec![("id".into(), zelyra_database::QueryValue::Int(id))];
            params.extend(tenant_query_params(context));
            Some(zelyra_database::Query {
                sql: format!(
                    "SELECT {} FROM {} WHERE {} = :id AND {} = :zelyra_tenant_id AND {}{} FOR UPDATE",
                    quote_identifier("id"),
                    quote_identifier(&crud.table),
                    quote_identifier("id"),
                    quote_identifier(tenant_column),
                    tenant_membership_predicate(context),
                    crud.soft_delete.as_ref().map_or_else(String::new, |soft_delete| {
                        format!(" AND {} IS NULL", quote_identifier(&soft_delete.column))
                    })
                ),
                params,
            })
        }
        _ => None,
    };
    let (query, success_message) = if let Some(soft_delete) = &crud.soft_delete {
        (
            format!(
                "UPDATE {} SET {} = CURRENT_TIMESTAMP WHERE {} = :id AND {} IS NULL{}",
                quote_identifier(&crud.table),
                quote_identifier(&soft_delete.column),
                quote_identifier("id"),
                quote_identifier(&soft_delete.column),
                tenant_condition
            ),
            "Record archived.",
        )
    } else {
        (
            format!(
                "DELETE FROM {} WHERE {} = :id{}",
                quote_identifier(&crud.table),
                quote_identifier("id"),
                tenant_condition
            ),
            "Record deleted.",
        )
    };
    let event = if crud.soft_delete.is_some() {
        "crud.archive"
    } else {
        "crud.delete"
    };
    let mut details = format!(
        "table={};operation={};record_id={}",
        audit_component(&crud.table),
        event,
        id
    );
    if let Some(tenant_context) = tenant_context {
        details.push_str(&format!(";tenant_id={}", tenant_context.tenant_id));
    }
    let mut delete_params = vec![("id".into(), zelyra_database::QueryValue::Int(id))];
    if let Some(tenant_context) = tenant_context {
        delete_params.extend(tenant_query_params(tenant_context));
    }
    let mut queries = vec![zelyra_database::Query {
        sql: query,
        params: delete_params,
    }];
    if let Some(audit_table) = audit_table {
        queries.extend(audit_insert_queries(
            audit_table,
            audit_chain,
            actor_user_id,
            event,
            Some(id),
            &details,
        ));
    }
    if let Err(error) = execute_crud_mutation(
        database_url,
        &queries,
        tenant_context,
        scoped_record_check.as_ref(),
    ) {
        eprintln!("zelyra web: CRUD delete failed: {error}");
        if error.message.starts_with("ZELYRA_TENANT_DENIED:") {
            return Response::html(404, "<h1>404 Not Found</h1>");
        }
        return crud_error_response(
            crud,
            500,
            "Internal Server Error",
            "The record could not be deleted.",
        );
    }
    Response::redirect(append_query_parameter(
        &crud.path,
        "zelyra_success",
        success_message,
    ))
}

fn dispatch_crud_restore(
    crud: &CrudRoute,
    request: &Request,
    path_params: &HashMap<String, String>,
    context: CrudMutationContext<'_>,
) -> Response {
    let CrudMutationContext {
        database_url,
        audit_table,
        audit_chain,
        actor_user_id,
        tenant_context,
    } = context;
    if request.method != "POST" {
        return Response::html(405, "<h1>405 Method Not Allowed</h1>");
    }
    let Some(soft_delete) = &crud.soft_delete else {
        return Response::html(404, "<h1>404 Not Found</h1>");
    };
    let Some(database_url) = database_url else {
        return crud_error_response(
            crud,
            503,
            "Service Unavailable",
            "DATABASE_URL is required for CRUD actions.",
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
    if !verify_csrf_request(
        request,
        &crud.csrf,
        input.get("_zelyra_csrf").map(String::as_str),
    ) {
        return Response::html(403, "<h1>403 Forbidden</h1><p>Invalid CSRF token.</p>");
    }
    if crud.tenant_column.is_some() && tenant_context.is_none() {
        return Response::html(403, "<h1>403 Forbidden</h1>");
    }
    let tenant_condition = match (crud.tenant_column.as_deref(), tenant_context) {
        (Some(column), Some(context)) => format!(
            " AND {} = :zelyra_tenant_id AND {}",
            quote_identifier(column),
            tenant_membership_predicate(context)
        ),
        _ => String::new(),
    };
    let scoped_record_check = match (crud.tenant_column.as_deref(), tenant_context) {
        (Some(tenant_column), Some(context)) => {
            let mut params = vec![("id".into(), zelyra_database::QueryValue::Int(id))];
            params.extend(tenant_query_params(context));
            Some(zelyra_database::Query {
                sql: format!(
                    "SELECT {} FROM {} WHERE {} = :id AND {} = :zelyra_tenant_id AND {} AND {} IS NOT NULL FOR UPDATE",
                    quote_identifier("id"),
                    quote_identifier(&crud.table),
                    quote_identifier("id"),
                    quote_identifier(tenant_column),
                    tenant_membership_predicate(context),
                    quote_identifier(&soft_delete.column)
                ),
                params,
            })
        }
        _ => None,
    };
    let query = format!(
        "UPDATE {} SET {} = NULL WHERE {} = :id AND {} IS NOT NULL{}",
        quote_identifier(&crud.table),
        quote_identifier(&soft_delete.column),
        quote_identifier("id"),
        quote_identifier(&soft_delete.column),
        tenant_condition
    );
    let mut details = format!(
        "table={};operation=crud.restore;record_id={}",
        audit_component(&crud.table),
        id
    );
    if let Some(tenant_context) = tenant_context {
        details.push_str(&format!(";tenant_id={}", tenant_context.tenant_id));
    }
    let mut restore_params = vec![("id".into(), zelyra_database::QueryValue::Int(id))];
    if let Some(tenant_context) = tenant_context {
        restore_params.extend(tenant_query_params(tenant_context));
    }
    let mut queries = vec![zelyra_database::Query {
        sql: query,
        params: restore_params,
    }];
    if let Some(audit_table) = audit_table {
        queries.extend(audit_insert_queries(
            audit_table,
            audit_chain,
            actor_user_id,
            "crud.restore",
            Some(id),
            &details,
        ));
    }
    if let Err(error) = execute_crud_mutation(
        database_url,
        &queries,
        tenant_context,
        scoped_record_check.as_ref(),
    ) {
        eprintln!("zelyra web: CRUD restore failed: {error}");
        if error.message.starts_with("ZELYRA_TENANT_DENIED:") {
            return Response::html(404, "<h1>404 Not Found</h1>");
        }
        return crud_error_response(
            crud,
            500,
            "Internal Server Error",
            "The record could not be restored.",
        );
    }
    Response::redirect(append_query_parameter(
        &crud.path,
        "zelyra_success",
        "Record restored.",
    ))
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct PageFilterSelection {
    name: String,
    value: String,
    operator: FilterOperator,
    kind: TableViewFilterKind,
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

    fn label(self, language: UiLanguage) -> String {
        let key = match self {
            Self::Equal => "operator.equal",
            Self::Contains => "operator.contains",
            Self::StartsWith => "operator.starts_with",
            Self::EndsWith => "operator.ends_with",
            Self::GreaterThan => "operator.greater_than",
            Self::GreaterThanOrEqual => "operator.greater_equal",
            Self::LessThan => "operator.less_than",
            Self::LessThanOrEqual => "operator.less_equal",
            Self::IsNull => "operator.is_null",
            Self::IsNotNull => "operator.is_not_null",
        };
        tr(language, key)
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

fn selected_filter_value<'a>(
    query_values: &'a HashMap<String, String>,
    value_name: &str,
    column: &str,
) -> &'a str {
    if let Some(value) = query_values.get(value_name) {
        return value;
    }
    let prefix = format!("filter_{column}__");
    sorted_query_values(query_values)
        .into_iter()
        .find(|(name, _)| name.starts_with(&prefix) && !name.ends_with("__operator"))
        .map_or("", |(_, value)| value)
}

fn sorted_query_values(query_values: &HashMap<String, String>) -> Vec<(&str, &str)> {
    let mut values = query_values
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect::<Vec<_>>();
    values.sort_unstable();
    values
}

fn sorted_filter_query_values(query_values: &HashMap<String, String>) -> Vec<(&str, &str)> {
    sorted_query_values(query_values)
        .into_iter()
        .filter(|(name, _)| name.starts_with("filter_"))
        .collect()
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
    archived: bool,
    success: Option<&'a str>,
    success_title: Option<&'a str>,
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
            restore: false,
            custom: Vec::new(),
        },
        UiLanguage::English,
    )
}

fn render_crud_list_with_actions(
    crud: &CrudRoute,
    view: CrudListView<'_>,
    ui_actions: CrudUiActions,
    language: UiLanguage,
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
        archived,
        success,
        success_title,
    } = view;
    let mut html = String::from("<main");
    html.push_str(&crud_loading_attribute(crud));
    html.push_str("><h1>");
    html.push_str(&html_escape_preserving_locale_references(
        &localize_user_text(language, &crud.title),
    ));
    html.push_str("</h1>");
    if crud.soft_delete.is_some() {
        html.push_str("<p class=\"zelyra-archive-toggle\"><a href=\"");
        if archived {
            html.push_str(&html_escape(&crud.path));
            html.push_str(&format!(
                "\">{}</a></p>",
                tr(language, "crud.archive.active")
            ));
        } else {
            html.push_str(&html_escape(&append_query_parameter(
                &crud.path, "archived", "true",
            )));
            html.push_str(&format!(
                "\">{}</a></p>",
                tr(language, "crud.archive.archived")
            ));
        }
    }
    if let Some(success) = success {
        html.push_str("<section class=\"zelyra-success\" role=\"status\">");
        if let Some(title) = success_title {
            html.push_str("<h2>");
            html.push_str(&html_escape_preserving_locale_references(
                &localize_user_text(language, title),
            ));
            html.push_str("</h2>");
        }
        html.push_str("<p>");
        html.push_str(&html_escape_preserving_locale_references(
            &localize_user_text(language, success),
        ));
        html.push_str("</p></section>");
    }
    if ui_actions.create {
        html.push_str("<p><a href=\"");
        html.push_str(&html_escape(&format!("{}/new", crud.path)));
        html.push_str(&format!("\">{}</a></p>", tr(language, "crud.create")));
    }
    html.push_str("<form method=\"get\" action=\"");
    html.push_str(&html_escape(&crud.path));
    html.push_str("\"><fieldset class=\"zelyra-query-controls\"><legend>");
    html.push_str(&tr(language, "query.legend"));
    html.push_str("</legend><label for=\"search\">");
    html.push_str(&tr(language, "query.search"));
    html.push_str("</label><input id=\"search\" name=\"search\" value=\"");
    html.push_str(&html_escape(search));
    html.push_str("\"><label for=\"sort\">");
    html.push_str(&tr(language, "query.sort"));
    html.push_str("</label><select id=\"sort\" name=\"sort\">");
    for column in sort_columns {
        html.push_str("<option value=\"");
        html.push_str(&html_escape(column));
        html.push('"');
        if *column == sort {
            html.push_str(" selected");
        }
        html.push('>');
        html.push_str(&html_escape_preserving_locale_references(
            &crud_column_label(language, &crud.schema, &crud.table, column),
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
        html.push_str("__operator\">");
        html.push_str(&html_escape_preserving_locale_references(&field_text(
            language,
            "query.filter_operator",
            &crud_column_label(language, &crud.schema, &crud.table, column),
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
                html.push_str(&operator.label(language));
                html.push_str("</option>");
            }
        } else {
            html.push_str("<option value=\"eq\" selected>");
            html.push_str(&tr(language, "query.operator_equal"));
            html.push_str("</option>");
        }
        html.push_str("</select><label for=\"filter_");
        html.push_str(&html_escape(column));
        html.push_str("\">");
        html.push_str(&html_escape_preserving_locale_references(&field_text(
            language,
            "query.filter_value",
            &crud_column_label(language, &crud.schema, &crud.table, column),
        )));
        html.push_str("</label><input id=\"filter_");
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
    html.push_str("<button type=\"submit\">");
    html.push_str(&tr(language, "query.apply"));
    html.push_str("</button></fieldset></form>");
    if rows.is_empty() {
        html.push_str("<p>");
        let empty_label = crud
            .list_view
            .empty
            .as_deref()
            .map(str::to_owned)
            .unwrap_or_else(|| tr(language, "table.empty"));
        html.push_str(&html_escape_preserving_locale_references(&empty_label));
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
                html.push_str(&html_escape_preserving_locale_references(
                    &crud_column_label(language, &crud.schema, &crud.table, column),
                ));
                html.push_str("</dt><dd>");
                if let Some(href) =
                    crud_list_detail_href(crud, query_columns, display_columns, row, column, value)
                {
                    html.push_str("<a href=\"");
                    html.push_str(&html_escape(&href));
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
            html.push_str(&html_escape_preserving_locale_references(
                &crud_column_label(language, &crud.schema, &crud.table, column),
            ));
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
                if let Some(href) =
                    crud_list_detail_href(crud, query_columns, display_columns, row, column, value)
                {
                    html.push_str("<a href=\"");
                    html.push_str(&html_escape(&href));
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
        html.push_str(&format!("\">{}</a> ", tr(language, "pagination.previous")));
    }
    html.push_str("<span>");
    html.push_str(&tr(language, "pagination.page"));
    html.push(' ');
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
        html.push_str(&format!("\">{}</a>", tr(language, "pagination.next")));
    }
    html.push_str("</nav></main>");
    html
}

fn crud_list_detail_href(
    crud: &CrudRoute,
    query_columns: &[&str],
    display_columns: &[&str],
    row: &[String],
    column: &str,
    value: &str,
) -> Option<String> {
    let detail_id = if column == "id" {
        Some(value)
    } else if display_columns.first().copied() == Some(column) {
        query_columns
            .iter()
            .position(|query_column| *query_column == "id")
            .and_then(|index| row.get(index))
            .map(String::as_str)
    } else {
        None
    }?;

    Some(format!("{}/{}", crud.path, detail_id))
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
            restore: false,
            custom: Vec::new(),
        },
        UiLanguage::English,
    )
}

fn render_crud_detail_with_actions(
    crud: &CrudRoute,
    columns: &[&str],
    row: &[String],
    id: &str,
    ui_actions: CrudUiActions,
    language: UiLanguage,
) -> String {
    let title = crud
        .detail_view
        .title
        .as_deref()
        .map(|title| localize_user_text(language, title))
        .unwrap_or_else(|| format!("{} {}", crud.title, tr(language, "detail.suffix")));
    let cards = crud.detail_view.mode == CrudDetailViewMode::Cards;
    let mut html = String::from("<main");
    html.push_str(&crud_loading_attribute(crud));
    html.push_str("><p><a href=\"");
    html.push_str(&html_escape(&crud.path));
    html.push_str(&format!("\">{}</a></p><h1>", tr(language, "detail.back")));
    html.push_str(&html_escape_preserving_locale_references(&title));
    html.push_str("</h1>");
    if cards {
        html.push_str("<article class=\"zelyra-crud-detail-card\">");
    }
    html.push_str("<dl>");
    for (column, value) in columns.iter().zip(row) {
        html.push_str("<dt>");
        html.push_str(&html_escape_preserving_locale_references(
            &crud_column_label(language, &crud.schema, &crud.table, column),
        ));
        html.push_str("</dt><dd>");
        html.push_str(&html_escape(value));
        html.push_str("</dd>");
    }
    if ui_actions.edit || ui_actions.create {
        html.push_str("</dl><p>");
        if ui_actions.edit {
            html.push_str("<a href=\"");
            html.push_str(&html_escape(&format!("{}/{}/edit", crud.path, id)));
            html.push_str(&format!("\">{}</a>", tr(language, "action.edit")));
        }
        if ui_actions.create {
            if ui_actions.edit {
                html.push(' ');
            }
            html.push_str("<a href=\"");
            html.push_str(&html_escape(&format!("{}/new", crud.path)));
            html.push_str(&format!("\">{}</a>", tr(language, "crud.create")));
        }
        html.push_str("</p>");
    } else {
        html.push_str("</dl>");
    }
    for action in &ui_actions.custom {
        let action_path = action.path.replace("{id}", id);
        if action.confirm_page.is_some() {
            html.push_str("<p><a class=\"zelyra-action-confirm-link\" href=\"");
            html.push_str(&html_escape(&action_path));
            html.push_str("\">");
            if let Some(icon) = &action.icon {
                html.push_str("<span class=\"zelyra-action-icon zelyra-action-icon-");
                html.push_str(&html_escape(icon));
                html.push_str("\" data-icon=\"");
                html.push_str(&html_escape(icon));
                html.push_str("\" aria-hidden=\"true\"></span>");
            }
            html.push_str(&html_escape_preserving_locale_references(
                &localize_user_text(language, &action.label),
            ));
            html.push_str("</a></p>");
        } else {
            html.push_str("<form method=\"post\" action=\"");
            html.push_str(&html_escape(&action_path));
            html.push('"');
            if let Some(confirm) = &action.confirm {
                let confirm = localize_user_text(language, confirm);
                html.push_str(" data-confirm=\"");
                html.push_str(&html_escape_preserving_locale_references(&confirm));
                html.push_str("\" onsubmit=\"return confirm(this.dataset.confirm)\"");
            }
            html.push_str("><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"");
            html.push_str(&html_escape(&action.csrf));
            html.push_str("\">");
            for field in &action.fields {
                html.push_str("<label for=\"");
                html.push_str(&html_escape(&field.name));
                html.push_str("\">");
                html.push_str(&html_escape_preserving_locale_references(
                    &localize_user_text(language, &field.label),
                ));
                html.push_str("</label>");
                if field.relation {
                    html.push_str("<select id=\"");
                    html.push_str(&html_escape(&field.name));
                    html.push_str("\" name=\"");
                    html.push_str(&html_escape(&field.name));
                    html.push('"');
                    if field.required {
                        html.push_str(" required");
                    }
                    html.push('>');
                    if !field.required {
                        html.push_str(&format!(
                            "<option value=\"\">-- {} --</option>",
                            tr(language, "action.select")
                        ));
                    }
                    for option in &field.options {
                        html.push_str("<option value=\"");
                        html.push_str(&html_escape(&option.value));
                        html.push_str("\">");
                        html.push_str(&html_escape(&option.label));
                        html.push_str("</option>");
                    }
                    html.push_str("</select>");
                } else {
                    html.push_str("<input id=\"");
                    html.push_str(&html_escape(&field.name));
                    html.push_str("\" name=\"");
                    html.push_str(&html_escape(&field.name));
                    html.push_str("\" type=\"");
                    html.push_str(&html_escape(&field.input_type));
                    html.push('"');
                    if field.input_type == "checkbox" {
                        html.push_str(" value=\"true\"");
                    }
                    if field.required && field.input_type != "checkbox" {
                        html.push_str(" required");
                    }
                    if let Some(max) = field.max {
                        html.push_str(" maxlength=\"");
                        html.push_str(&max.to_string());
                        html.push('"');
                    }
                    html.push('>');
                }
            }
            html.push_str("<button type=\"submit\">");
            if let Some(icon) = &action.icon {
                html.push_str("<span class=\"zelyra-action-icon zelyra-action-icon-");
                html.push_str(&html_escape(icon));
                html.push_str("\" data-icon=\"");
                html.push_str(&html_escape(icon));
                html.push_str("\" aria-hidden=\"true\"></span>");
            }
            html.push_str(&html_escape_preserving_locale_references(
                &localize_user_text(language, &action.label),
            ));
            html.push_str("</button></form>");
        }
    }
    if ui_actions.restore {
        html.push_str("<form method=\"post\" action=\"");
        html.push_str(&html_escape(&format!("{}/{}/restore", crud.path, id)));
        html.push_str("\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"");
        html.push_str(&html_escape(crud.csrf.token()));
        html.push_str("\"><button type=\"submit\">");
        html.push_str(&tr(language, "action.restore"));
        html.push_str("</button></form>");
    } else if ui_actions.delete {
        if let Some(title) = &crud.delete_view.title {
            html.push_str("<h2>");
            html.push_str(&html_escape_preserving_locale_references(
                &localize_user_text(language, title),
            ));
            html.push_str("</h2>");
        }
        if let Some(message) = &crud.delete_view.message {
            html.push_str("<p class=\"zelyra-delete-message\">");
            html.push_str(&html_escape_preserving_locale_references(
                &localize_user_text(language, message),
            ));
            html.push_str("</p>");
        }
        html.push_str("<form method=\"post\" action=\"");
        html.push_str(&html_escape(&format!("{}/{}/delete", crud.path, id)));
        html.push_str("\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"");
        html.push_str(&html_escape(crud.csrf.token()));
        html.push_str("\"><button type=\"submit\">");
        let submit = crud
            .delete_view
            .submit
            .as_deref()
            .map(|submit| localize_user_text(language, submit))
            .unwrap_or_else(|| tr(language, "action.delete"));
        html.push_str(&html_escape_preserving_locale_references(&submit));
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

fn append_query_parameter(path: &str, name: &str, value: &str) -> String {
    let separator = if path.contains('?') { '&' } else { '?' };
    format!(
        "{path}{separator}{}={}",
        url_encode(name),
        url_encode(value)
    )
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
    let lower = name.rsplit("::").next()?.to_ascii_lowercase();
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
    context: FormExecutionContext<'_>,
) -> Result<(), String> {
    let FormExecutionContext {
        actor_user_id,
        expected_snapshot,
        tenant_context,
    } = context;
    let mut parameters = form_query_parameters(form, values)?;
    if let Some(tenant_context) = tenant_context {
        parameters.extend(tenant_query_params(tenant_context));
    }
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
    zelyra_database::with_mariadb_transaction(database_url, |transaction| {
        if form.tenant_column.is_some() && tenant_context.is_none() {
            return Err(zelyra_database::DatabaseError {
                message: "tenant context is required for this form".into(),
            });
        }
        if let Some(tenant_context) = tenant_context {
            let membership_check = zelyra_database::Query {
                sql: format!(
                    "SELECT tenant_id FROM {} WHERE user_id = :zelyra_user_id AND tenant_id = :zelyra_tenant_id AND active = true FOR UPDATE",
                    quote_identifier(&tenant_context.membership_table)
                ),
                params: tenant_query_params(tenant_context),
            };
            if transaction.execute(&membership_check)?.rows.is_empty() {
                return Err(zelyra_database::DatabaseError {
                    message: "ZELYRA_TENANT_DENIED: membership is no longer active".into(),
                });
            }
        }
        let before_values = if let Some(expected_snapshot) = expected_snapshot {
            let table_name =
                form.form
                    .table
                    .as_deref()
                    .ok_or_else(|| zelyra_database::DatabaseError {
                        message: "edit form has no source table".into(),
                    })?;
            let table = form
                .schema
                .as_ref()
                .and_then(|schema| schema.tables.iter().find(|table| table.name == table_name))
                .ok_or_else(|| zelyra_database::DatabaseError {
                    message: "edit form table is missing from schema".into(),
                })?;
            let id = path_params
                .get("id")
                .and_then(|id| id.parse::<i64>().ok())
                .ok_or_else(|| zelyra_database::DatabaseError {
                    message: "edit form path parameter `id` is not an integer".into(),
                })?;
            let columns = form_value_columns(form, table);
            let tenant_condition = match (form.tenant_column.as_deref(), tenant_context) {
                (Some(column), Some(context)) => format!(
                    " AND {} = :zelyra_tenant_id AND {}",
                    quote_identifier(column),
                    tenant_membership_predicate(context)
                ),
                (Some(_), None) => {
                    return Err(zelyra_database::DatabaseError {
                        message: "tenant context is required for this form".into(),
                    })
                }
                _ => String::new(),
            };
            let query = zelyra_database::Query {
                sql: format!(
                    "SELECT {} FROM {} WHERE {} = :id{} FOR UPDATE",
                    columns
                        .iter()
                        .map(|column| quote_identifier(column))
                        .collect::<Vec<_>>()
                        .join(", "),
                    quote_identifier(table_name),
                    quote_identifier("id"),
                    tenant_condition
                ),
                params: {
                    let mut params = vec![("id".into(), zelyra_database::QueryValue::Int(id))];
                    if let Some(tenant_context) = tenant_context {
                        params.extend(tenant_query_params(tenant_context));
                    }
                    params
                },
            };
            let result = transaction.execute(&query)?;
            let Some(row) = result.rows.first() else {
                return Err(zelyra_database::DatabaseError {
                    message: if tenant_context.is_some() {
                        "ZELYRA_TENANT_DENIED: record is not available in this tenant".into()
                    } else {
                        "ZELYRA_CONFLICT: edited record no longer exists".into()
                    },
                });
            };
            let current_values = form
                .form
                .fields
                .iter()
                .zip(row)
                .map(|(field, value)| (field.name.clone(), value.clone()))
                .collect::<HashMap<_, _>>();
            if !form_snapshot_matches(&form.csrf, expected_snapshot, &current_values) {
                return Err(zelyra_database::DatabaseError {
                    message: "ZELYRA_CONFLICT: edited record changed after the form was loaded"
                        .into(),
                });
            }
            Some(current_values)
        } else {
            None
        };

        for query in &queries {
            transaction.execute(query)?;
        }
        if let (Some(audit_table), Some(audit_event)) =
            (form.audit_table.as_deref(), form.audit_event.as_deref())
        {
            let (record_id, mut details) = form_audit_details(
                form,
                audit_event,
                path_params,
                before_values.as_ref(),
                values,
            );
            if let Some(tenant_context) = tenant_context {
                details.push_str(&format!(";tenant_id={}", tenant_context.tenant_id));
            }
            for query in audit_insert_queries(
                audit_table,
                form.audit_chain,
                actor_user_id,
                audit_event,
                record_id,
                &details,
            ) {
                transaction.execute(&query)?;
            }
        }
        Ok(())
    })
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

#[derive(Clone, Debug, PartialEq, Eq)]
enum RouteDataError {
    NotFound,
    DatabaseUnavailable,
    Query,
    InvalidQuery(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct LoadedRouteData {
    values: HashMap<String, String>,
    collections: HashMap<String, Vec<HashMap<String, String>>>,
}

fn load_route_data(
    route: &Route,
    params: &HashMap<String, String>,
    query_values: &HashMap<String, String>,
    database_url: Option<&str>,
) -> Result<LoadedRouteData, RouteDataError> {
    let mut bound_query_values = Vec::new();
    for input in &route.query {
        let value = match query_values.get(&input.name) {
            Some(value) => {
                page_query_value(&input.ty, value).map_err(RouteDataError::InvalidQuery)?
            }
            None if matches!(input.ty, Type::Option(_)) => QueryValue::Null,
            None => {
                return Err(RouteDataError::InvalidQuery(format!(
                    "missing required query parameter `{}`",
                    input.name
                )))
            }
        };
        bound_query_values.push((input.name.clone(), value));
    }
    let mut loaded = LoadedRouteData::default();
    for input in &route.query {
        loaded.values.insert(
            input.name.clone(),
            query_values.get(&input.name).cloned().unwrap_or_default(),
        );
    }
    let page = if route.page_size.is_some() {
        let page = page_number(query_values).map_err(RouteDataError::InvalidQuery)?;
        loaded.values.insert("page".into(), page.to_string());
        page
    } else {
        1
    };
    let (sort, order) =
        page_sort_state(route, query_values).map_err(RouteDataError::InvalidQuery)?;
    if let Some(sort) = &sort {
        loaded.values.insert("sort".into(), sort.clone());
        loaded
            .values
            .insert("order".into(), order.to_ascii_lowercase());
    }
    let search = page_search_state(route, query_values).map_err(RouteDataError::InvalidQuery)?;
    if let Some(search) = &search {
        loaded.values.insert("search".into(), search.clone());
    }
    let filters = page_filter_state(route, query_values).map_err(RouteDataError::InvalidQuery)?;
    for filter in &filters {
        loaded
            .values
            .insert(format!("filter_{}", filter.name), filter.value.clone());
        loaded.values.insert(
            format!("filter_{}__operator", filter.name),
            filter.operator.key().into(),
        );
    }
    if route.data.is_empty() {
        return Ok(loaded);
    }
    let Some(database_url) = database_url else {
        return Err(RouteDataError::DatabaseUnavailable);
    };
    for data in &route.data {
        let mut query = data.query.clone();
        let mut query_params = params
            .iter()
            .map(|(name, value)| (name.clone(), QueryValue::String(value.clone())))
            .collect::<Vec<_>>();
        query_params.extend(bound_query_values.iter().cloned());
        if data.collection && route.page_size.is_some() && !loaded.values.contains_key("total") {
            let (count_source, count_parameters) = page_collection_query(
                &query,
                PageCollectionQueryOptions {
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
