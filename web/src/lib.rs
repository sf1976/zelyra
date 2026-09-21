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
    CrudDeleteViewDef, CrudDetailViewDef, CrudDetailViewMode, CrudErrorViewDef, CrudFormViewDef,
    CrudFormViewMode, CrudListViewDef, CrudListViewMode, CrudLoadingViewDef, FormDef, TableDef,
    Type,
};
use zelyra_database::{QueryValue, Schema};
use zelyra_forms::{validate, FieldError};
mod i18n;
#[cfg(test)]
use i18n::framework_text;
use i18n::{
    field_text, framework_text_with_catalog, identifier as locale_identifier, LOCALE_REFERENCE_END,
    LOCALE_REFERENCE_PARAMETER, LOCALE_REFERENCE_START,
};

const ZELYRA_DESIGN_SYSTEM_CSS: &str = include_str!("../assets/zelyra.css");
pub const PROJECT_THEME_CSS_PATH: &str = "/__zelyra/theme.css";
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

const CRUD_LAYOUT_CONTENT_MARKER: &str = "\u{0}ZELYRA_CRUD_CONTENT\u{0}";

#[derive(Clone, Debug, PartialEq, Eq)]
struct DefaultNavigationLink {
    path: String,
    label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DefaultUiContext {
    current_path: String,
    current_label: String,
    navigation: Vec<DefaultNavigationLink>,
}

fn apply_generated_layout(mut response: Response, layout_html: Option<&str>) -> Response {
    let Some(layout_html) = layout_html else {
        return response;
    };
    if response.location.is_some() || !response.content_type.starts_with("text/html") {
        return response;
    }
    response.body = layout_html.replace(CRUD_LAYOUT_CONTENT_MARKER, &response.body);
    response
}

fn render_default_application_shell(
    content: &str,
    context: &DefaultUiContext,
    language: UiLanguage,
) -> String {
    let document_start = content.trim_start();
    if document_start
        .get(..14)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("<!doctype html"))
        || document_start
            .get(..5)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("<html"))
    {
        return content.to_owned();
    }

    let localized_current_label = localize_user_text(language, &context.current_label);
    let current_label = html_escape_preserving_locale_references(&localized_current_label);
    let page_title =
        html_escape_preserving_locale_references(&format!("{localized_current_label} | Zelyra"));
    let home_path = context
        .navigation
        .first()
        .map(|link| link.path.as_str())
        .unwrap_or(if context.current_path == "/login" {
            "/"
        } else {
            &context.current_path
        });
    let mut navigation = String::new();
    for (index, link) in context.navigation.iter().enumerate() {
        let active = navigation_link_is_active(&link.path, &context.current_path);
        let current = if active { " aria-current=\"page\"" } else { "" };
        let label =
            html_escape_preserving_locale_references(&localize_user_text(language, &link.label));
        navigation.push_str(&format!(
            "<a href=\"{}\" aria-label=\"{label}\"{current}><span class=\"zelyra-nav-icon\" aria-hidden=\"true\">{:02}</span><span class=\"zelyra-nav-label\">{label}</span></a>",
            html_escape(&link.path),
            index + 1,
        ));
    }

    let main_content = if content.trim_start().starts_with("<main") {
        content.to_owned()
    } else {
        format!("<main>{content}</main>")
    };
    format!(
        "<!doctype html><html data-zelyra-language><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{page_title}</title></head><body><a class=\"zelyra-skip-link\" href=\"#zelyra-content\">{}</a><div class=\"zelyra-app\"><aside class=\"zelyra-sidebar\"><a class=\"zelyra-brand\" aria-label=\"Zelyra\" href=\"{}\"><span class=\"zelyra-mark\" aria-hidden=\"true\">Z</span><span class=\"zelyra-brand-copy\"><span class=\"zelyra-brand-name\">Zelyra</span><span class=\"zelyra-brand-descriptor\">{}</span></span></a><p class=\"zelyra-sidebar-caption\">{}</p><nav class=\"zelyra-nav\" aria-label=\"{}\">{navigation}</nav><div class=\"zelyra-sidebar-footer\"><span class=\"zelyra-status-dot\" aria-hidden=\"true\"></span><span>{}</span></div></aside><div class=\"zelyra-workspace\"><header class=\"zelyra-topbar\"><div class=\"zelyra-breadcrumb\"><span>{}</span><span aria-hidden=\"true\">/</span><strong>{current_label}</strong></div><span class=\"zelyra-environment\">{}</span></header><div class=\"zelyra-page-content\" id=\"zelyra-content\">{main_content}</div></div></div></body></html>",
        tr(language, "shell.skip_to_content"),
        html_escape(home_path),
        tr(language, "shell.brand_descriptor"),
        tr(language, "shell.navigation_caption"),
        tr(language, "shell.navigation_label"),
        tr(language, "shell.powered_by"),
        tr(language, "shell.workspace_label"),
        tr(language, "shell.environment_label"),
    )
}

fn navigation_link_is_active(link_path: &str, current_path: &str) -> bool {
    if link_path == "/" {
        return current_path == "/";
    }
    current_path == link_path
        || current_path
            .strip_prefix(link_path.trim_end_matches('/'))
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn crud_route_matches_path(crud: &CrudRoute, path: &str) -> bool {
    let base = crud.path.trim_end_matches('/');
    let base = if base.is_empty() { "/" } else { base };
    let patterns = [
        crud.path.clone(),
        format!("{base}/new"),
        format!("{base}/{{id}}"),
        format!("{base}/{{id}}/edit"),
        format!("{base}/{{id}}/delete"),
        format!("{base}/{{id}}/restore"),
    ];
    patterns
        .iter()
        .any(|pattern| match_path(pattern, path).is_some())
        || crud
            .actions
            .iter()
            .any(|action| match_path(&action.form.path, path).is_some())
}

fn add_navigation_link(
    navigation: &mut Vec<DefaultNavigationLink>,
    path: impl Into<String>,
    label: impl Into<String>,
) {
    let path = path.into();
    if !navigation.iter().any(|link| link.path == path) {
        navigation.push(DefaultNavigationLink {
            path,
            label: label.into(),
        });
    }
}

fn tr(_language: UiLanguage, key: &str) -> String {
    i18n::reference(key)
}

#[cfg(test)]
fn localize_html(source: &str, language: UiLanguage) -> String {
    localize_html_with_catalog(source, language, &ProjectUiCatalogs::default())
}

fn localize_html_with_catalog(
    source: &str,
    language: UiLanguage,
    project_catalogs: &ProjectUiCatalogs,
) -> String {
    let language_code = match language {
        UiLanguage::English => "en",
        UiLanguage::German => "de",
    };
    let translated = localize_framework_markup_with_catalog(source, language, project_catalogs);
    let mut html = translated.replace("data-zelyra-language", &format!("lang=\"{language_code}\""));
    let marker = "data-zelyra-i18n=\"";
    let mut search_from = 0;
    while let Some(relative_start) = html[search_from..].find(marker) {
        let marker_start = search_from + relative_start;
        let key_start = marker_start + marker.len();
        let Some(key_end_relative) = html[key_start..].find('"') else {
            break;
        };
        let key_end = key_start + key_end_relative;
        let key = html[key_start..key_end].to_owned();
        let Some(open_tag_start) = html[..marker_start].rfind('<') else {
            break;
        };
        let Some(tag_name_end_relative) = html[open_tag_start + 1..]
            .find(|character: char| character.is_whitespace() || character == '>')
        else {
            break;
        };
        let tag_name_end = open_tag_start + 1 + tag_name_end_relative;
        let tag_name = html[open_tag_start + 1..tag_name_end].to_owned();
        let Some(open_tag_end_relative) = html[tag_name_end..].find('>') else {
            break;
        };
        let content_start = tag_name_end + open_tag_end_relative + 1;
        let closing_tag = format!("</{tag_name}>");
        let Some(content_end_relative) = html[content_start..].find(&closing_tag) else {
            break;
        };
        let content_end = content_start + content_end_relative;
        let translated = html_escape(&project_catalogs.text(language, &key));
        html.replace_range(content_start..content_end, &translated);
        search_from = content_start + translated.len() + closing_tag.len();
    }
    resolve_locale_references(&html, language, project_catalogs)
}

fn localize_framework_markup_with_catalog(
    source: &str,
    language: UiLanguage,
    project_catalogs: &ProjectUiCatalogs,
) -> String {
    let mut html = source.to_owned();
    for tag in [
        "h1", "h2", "h3", "p", "button", "label", "legend", "th", "option",
    ] {
        let opening = format!("<{tag}");
        let closing = format!("</{tag}>");
        let mut cursor = 0;
        while let Some(relative_start) = html[cursor..].find(&opening) {
            let start = cursor + relative_start;
            let after_name = start + opening.len();
            if html
                .as_bytes()
                .get(after_name)
                .is_some_and(|byte| !byte.is_ascii_whitespace() && *byte != b'>')
            {
                cursor = after_name;
                continue;
            }
            let Some(open_end_relative) = html[after_name..].find('>') else {
                break;
            };
            let content_start = after_name + open_end_relative + 1;
            let Some(content_end_relative) = html[content_start..].find(&closing) else {
                break;
            };
            let content_end = content_start + content_end_relative;
            if !html[content_start..content_end].contains('<') {
                if let Some(copy) = framework_text_with_catalog(
                    language,
                    &html[content_start..content_end],
                    project_catalogs,
                ) {
                    let translated = html_escape(&copy);
                    html.replace_range(content_start..content_end, &translated);
                    cursor = content_start + translated.len() + closing.len();
                    continue;
                }
            }
            cursor = content_end + closing.len();
        }
    }
    html
}

fn resolve_locale_references(
    source: &str,
    language: UiLanguage,
    project_catalogs: &ProjectUiCatalogs,
) -> String {
    let mut output = String::with_capacity(source.len());
    let mut remaining = source;
    while let Some(start) = remaining.find(LOCALE_REFERENCE_START) {
        output.push_str(&remaining[..start]);
        let reference_start = start + LOCALE_REFERENCE_START.len();
        let Some(end_relative) = remaining[reference_start..].find(LOCALE_REFERENCE_END) else {
            output.push_str(&remaining[start..]);
            return output;
        };
        let end = reference_start + end_relative;
        let token_end = end + LOCALE_REFERENCE_END.len_utf8();
        let reference = &remaining[reference_start..end];
        if let Some(translated) = locale_reference_text(reference, language, project_catalogs, 0) {
            output.push_str(&html_escape(&translated));
        } else {
            output.push_str(&remaining[start..token_end]);
        }
        remaining = &remaining[token_end..];
    }
    output.push_str(remaining);
    output
}

fn locale_reference_text(
    reference: &str,
    language: UiLanguage,
    project_catalogs: &ProjectUiCatalogs,
    depth: usize,
) -> Option<String> {
    if depth > 8 {
        return None;
    }
    let (key, encoded_field) = reference
        .split_once(LOCALE_REFERENCE_PARAMETER)
        .map_or((reference, None), |(key, value)| (key, Some(value)));
    if !i18n::valid_catalog_key(key) {
        return None;
    }
    let mut translated = project_catalogs.text(language, key).into_owned();
    if let Some(encoded_parameter) = encoded_field {
        let (parameter, encoded_value) = encoded_parameter.split_once('=')?;
        if !i18n::valid_catalog_key(parameter) {
            return None;
        }
        let mut value = decode_locale_parameter(encoded_value)?;
        if parameter == "field" {
            if let Some(nested) = value
                .strip_prefix(LOCALE_REFERENCE_START)
                .and_then(|value| value.strip_suffix(LOCALE_REFERENCE_END))
            {
                value = locale_reference_text(nested, language, project_catalogs, depth + 1)?;
            }
        }
        translated = translated.replace(&format!("{{{parameter}}}"), &value);
    }
    Some(translated)
}

fn decode_locale_parameter(value: &str) -> Option<String> {
    if value.len() & 1 != 0 {
        return None;
    }
    let bytes = value
        .as_bytes()
        .chunks(2)
        .map(|pair| {
            let pair: &[u8; 2] = pair.try_into().ok()?;
            let digits = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(digits, 16).ok()
        })
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok()
}

fn localize_user_text(_language: UiLanguage, source: &str) -> String {
    source.strip_prefix("@i18n:").map_or_else(
        || {
            source
                .replace(LOCALE_REFERENCE_START, "&#xe000;zelyra-locale:")
                .replace(LOCALE_REFERENCE_PARAMETER, "&#xe002;")
                .replace(LOCALE_REFERENCE_END, "&#xe001;")
        },
        i18n::reference,
    )
}

fn inject_design_system(source: &str, has_project_theme: bool) -> String {
    if !source.contains("class=\"zelyra-app\"") {
        return source.to_owned();
    }
    let default_theme_missing = !source.contains("data-zelyra-theme=\"default\"");
    let project_theme_missing =
        has_project_theme && !source.contains(&format!("href=\"{PROJECT_THEME_CSS_PATH}\""));
    if !default_theme_missing && !project_theme_missing {
        return source.to_owned();
    }
    let mut stylesheets = String::new();
    if default_theme_missing {
        stylesheets.push_str(&format!(
            "<style data-zelyra-theme=\"default\">{ZELYRA_DESIGN_SYSTEM_CSS}</style>"
        ));
    }
    if project_theme_missing {
        stylesheets.push_str(&format!(
            "<link rel=\"stylesheet\" href=\"{PROJECT_THEME_CSS_PATH}\" data-zelyra-theme=\"project\">"
        ));
    }
    if let Some(head_end) = source.rfind("</head>") {
        let mut html = String::with_capacity(source.len() + stylesheets.len());
        html.push_str(&source[..head_end]);
        html.push_str(&stylesheets);
        html.push_str(&source[head_end..]);
        html
    } else {
        format!("{stylesheets}{source}")
    }
}

fn append_learning_assistant(
    html: &str,
    path: &str,
    language: UiLanguage,
    generated_crud: bool,
) -> String {
    let machine_page = path.starts_with("/machines");
    let section = if machine_page {
        "machine"
    } else if generated_crud {
        "crud"
    } else {
        "view"
    };
    let title = tr(language, &format!("learning.{section}.title"));
    let introduction = tr(language, &format!("learning.{section}.intro"));
    let heading = tr(language, &format!("learning.{section}.heading"));
    let example = tr(language, &format!("learning.{section}.example"));
    let guidance = tr(language, &format!("learning.{section}.guidance"));
    let title = html_escape_preserving_locale_references(&title);
    let introduction = html_escape_preserving_locale_references(&introduction);
    let heading = html_escape_preserving_locale_references(&heading);
    let example = html_escape_preserving_locale_references(&example);
    let guidance = html_escape_preserving_locale_references(&guidance);
    let button_label = html_escape_preserving_locale_references(&tr(language, "learning.button"));
    let style = r#"<style>
.zelyra-learning-assistant{position:fixed;right:24px;bottom:24px;z-index:2147483000;font:500 14px/1.5 system-ui,-apple-system,"Segoe UI",sans-serif;color:#17243b}
.zelyra-learning-assistant summary{display:flex;align-items:center;gap:9px;list-style:none;cursor:pointer;padding:10px 16px 10px 10px;border:1px solid #dce3f2;border-radius:999px;background:linear-gradient(135deg,#fff 5%,#f2f5ff 100%);box-shadow:0 10px 34px #17243b2b;color:#27375a;font-weight:700}
.zelyra-learning-assistant summary::-webkit-details-marker{display:none}
.zelyra-learning-assistant summary:focus-visible{outline:3px solid #927cff;outline-offset:3px}
.zelyra-learning-assistant-icon{display:grid;place-items:center;width:28px;height:28px;border-radius:50%;background:linear-gradient(145deg,#8067ff,#32c7c1);color:white;font-weight:800}
.zelyra-learning-assistant[open] summary{border-color:#a89bff}
.zelyra-learning-panel{position:absolute;right:0;bottom:56px;width:min(390px,calc(100vw - 32px));padding:22px;border:1px solid #e1e6f0;border-radius:18px;background:#fff;box-shadow:0 22px 70px #17243b30}
.zelyra-learning-panel h2{margin:0 0 6px;font-size:19px;letter-spacing:-.02em}
.zelyra-learning-panel h3{margin:18px 0 8px;font-size:15px}
.zelyra-learning-panel p{margin:8px 0;color:#526078}
.zelyra-learning-panel pre{overflow:auto;margin:10px 0;padding:13px 15px;border-radius:10px;background:#111b31;color:#e4e9ff;font:13px/1.6 ui-monospace,SFMono-Regular,Consolas,monospace}
.zelyra-learning-panel code{color:#5140bb;font:600 12px ui-monospace,SFMono-Regular,Consolas,monospace}
@media(max-width:560px){.zelyra-learning-assistant{right:14px;bottom:14px}.zelyra-learning-panel{right:-2px;max-height:70vh;overflow:auto}}
</style>"#;
    let widget = format!(
        "{style}<details class=\"zelyra-learning-assistant\"><summary aria-label=\"{button_label}\"><span class=\"zelyra-learning-assistant-icon\" aria-hidden=\"true\">i</span><span>{button_label}</span></summary><section class=\"zelyra-learning-panel\" role=\"region\" aria-label=\"{title}\"><h2>{title}</h2><p>{introduction}</p><h3>{heading}</h3><pre><code>{example}</code></pre><p>{guidance}</p></section></details>"
    );
    if let Some(body_end) = html.rfind("</body>") {
        let mut rendered = String::with_capacity(html.len() + widget.len());
        rendered.push_str(&html[..body_end]);
        rendered.push_str(&widget);
        rendered.push_str(&html[body_end..]);
        rendered
    } else {
        format!("{html}{widget}")
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
    pub ui_language: UiLanguage,
    pub ui_level: UiLevel,
    pub project_theme_css: Option<String>,
    allowed_hosts: Vec<String>,
    project_ui_catalogs: ProjectUiCatalogs,
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
            ui_language: UiLanguage::default(),
            ui_level: UiLevel::default(),
            project_theme_css: None,
            allowed_hosts: default_allowed_hosts(),
            project_ui_catalogs: ProjectUiCatalogs::default(),
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
            ui_language: UiLanguage::default(),
            ui_level: UiLevel::default(),
            project_theme_css: None,
            allowed_hosts: default_allowed_hosts(),
            project_ui_catalogs: ProjectUiCatalogs::default(),
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
        if (request.headers.contains_key("host") || request_has_browser_origin_or_session(request))
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
            return Response::html(
                400,
                format!(
                    "<h1>{}</h1><p>{}</p>",
                    html_escape(&title),
                    html_escape(&message)
                ),
            );
        }
        let mut response = self.dispatch_inner(request);
        if response.content_type.starts_with("text/html") {
            if response.location.is_none() {
                if let Some(context) = self.default_ui_context(request) {
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
            && request.method == "GET"
            && response.status == 200
            && response.content_type.starts_with("text/html")
        {
            let generated_crud = self
                .cruds
                .iter()
                .any(|crud| crud_route_matches_path(crud, &request.path));
            response.body = append_learning_assistant(
                &response.body,
                &request.path,
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
    }

    fn dispatch_inner(&self, request: &Request) -> Response {
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
                    return self.apply_api_cors(
                        request,
                        Response::json(
                            403,
                            "{\"error\":{\"code\":\"Forbidden\",\"message\":\"request origin is not allowed\"}}",
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
                    return self.apply_api_cors(request, response);
                }
                return self.apply_api_cors(request, (api.handler)(request, &path_params));
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
                return apply_generated_layout(
                    dispatch_form_with_language(
                        form,
                        request,
                        &path_params,
                        self.database_url.as_deref(),
                        session_from_request(self, request, self.database_url.as_deref())
                            .and_then(|session| session.user_id),
                        self.ui_language,
                    ),
                    form.layout_html.as_deref(),
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
                return apply_generated_layout(
                    dispatch_crud(
                        crud,
                        request,
                        self.database_url.as_deref(),
                        crud_ui_actions(crud, request, self),
                        self.ui_language,
                    ),
                    crud.layout_html.as_deref(),
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
                    return apply_generated_layout(
                        dispatch_form_with_language(
                            &action.form,
                            request,
                            &path_params,
                            self.database_url.as_deref(),
                            session_from_request(self, request, self.database_url.as_deref())
                                .and_then(|session| session.user_id),
                            self.ui_language,
                        ),
                        action.form.layout_html.as_deref(),
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
                return apply_generated_layout(
                    dispatch_crud_restore(
                        crud,
                        request,
                        &path_params,
                        self.database_url.as_deref(),
                        self.auth_route
                            .as_ref()
                            .and_then(|auth| auth.audit_table.as_deref()),
                        self.auth_route
                            .as_ref()
                            .is_some_and(|auth| auth.audit_chain),
                        session_from_request(self, request, self.database_url.as_deref())
                            .and_then(|session| session.user_id),
                    ),
                    crud.layout_html.as_deref(),
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
                return apply_generated_layout(
                    dispatch_crud_delete(
                        crud,
                        request,
                        &path_params,
                        self.database_url.as_deref(),
                        self.auth_route
                            .as_ref()
                            .and_then(|auth| auth.audit_table.as_deref()),
                        self.auth_route
                            .as_ref()
                            .is_some_and(|auth| auth.audit_chain),
                        session_from_request(self, request, self.database_url.as_deref())
                            .and_then(|session| session.user_id),
                    ),
                    crud.layout_html.as_deref(),
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
                return apply_generated_layout(
                    dispatch_crud_detail(
                        crud,
                        request,
                        &path_params,
                        self.database_url.as_deref(),
                        crud_ui_actions(crud, request, self),
                        self.ui_language,
                    ),
                    crud.layout_html.as_deref(),
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
                format!(
                    "zelyra_session={session_id}; Path=/; HttpOnly; SameSite=Lax{}",
                    secure_cookie_attribute(request)
                ),
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

fn render_login(auth: &AuthRoute, language: UiLanguage) -> String {
    format!(
        "<main><h1>{}</h1><form method=\"post\" action=\"/login\">\
         <input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{}\">\
         <label for=\"email\">{}</label><input id=\"email\" name=\"email\" type=\"email\" required>\
         <label for=\"password\">{}</label><input id=\"password\" name=\"password\" type=\"password\" required>\
         <button type=\"submit\">{}</button></form></main>",
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
    html.push_str("</table><h2>");
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
        let name = name.trim().to_ascii_lowercase();
        if matches!(
            name.as_str(),
            "host" | "origin" | "referer" | "x-forwarded-proto" | "cookie" | "authorization"
        ) && headers.contains_key(&name)
        {
            return Err(HttpError {
                message: "request contains a duplicate security-sensitive header".into(),
            });
        }
        headers.insert(name, value.trim().into());
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
        UiLanguage::English,
    )
}

fn dispatch_form_with_language(
    form: &FormRoute,
    request: &Request,
    path_params: &HashMap<String, String>,
    database_url: Option<&str>,
    actor_user_id: Option<i64>,
    language: UiLanguage,
) -> Response {
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
            render_form_with_language(&rendered_form, &values, &[], None, &options, language),
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
    if !verify_csrf_request(
        request,
        &form.csrf,
        input.get("_zelyra_csrf").map(String::as_str),
    ) {
        return Response::html(403, "<h1>403 Forbidden</h1><p>Invalid CSRF token.</p>");
    }
    let mut values = input;
    values.remove("_zelyra_csrf");
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
        let before_values = if form.audit_table.is_some()
            && form
                .audit_event
                .as_deref()
                .is_some_and(|event| event == "crud.update" || event.starts_with("crud.action."))
        {
            load_existing_form_values(form, path_params, Some(database_url))
                .ok()
                .flatten()
        } else {
            None
        };
        if let Err(error) = execute_form_action(
            form,
            action,
            &values,
            path_params,
            database_url,
            actor_user_id,
            before_values.as_ref(),
        ) {
            eprintln!("zelyra web: form action failed: {error}");
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
    let query = format!(
        "SELECT {} FROM {} WHERE {} = :id{}",
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
        soft_delete_condition
    );
    let result = match zelyra_database::execute_mariadb_query(
        database_url,
        &query,
        vec![("id".into(), zelyra_database::QueryValue::Int(id))],
    ) {
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
    database_url: Option<&str>,
    audit_table: Option<&str>,
    audit_chain: bool,
    actor_user_id: Option<i64>,
) -> Response {
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
    let (query, success_message) = if let Some(soft_delete) = &crud.soft_delete {
        (
            format!(
                "UPDATE {} SET {} = CURRENT_TIMESTAMP WHERE {} = :id AND {} IS NULL",
                quote_identifier(&crud.table),
                quote_identifier(&soft_delete.column),
                quote_identifier("id"),
                quote_identifier(&soft_delete.column)
            ),
            "Record archived.",
        )
    } else {
        (
            format!(
                "DELETE FROM {} WHERE {} = :id",
                quote_identifier(&crud.table),
                quote_identifier("id")
            ),
            "Record deleted.",
        )
    };
    let event = if crud.soft_delete.is_some() {
        "crud.archive"
    } else {
        "crud.delete"
    };
    let details = format!(
        "table={};operation={};record_id={}",
        audit_component(&crud.table),
        event,
        id
    );
    let mut queries = vec![zelyra_database::Query {
        sql: query,
        params: vec![("id".into(), zelyra_database::QueryValue::Int(id))],
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
    if let Err(error) = zelyra_database::execute_mariadb_queries(database_url, &queries, true) {
        eprintln!("zelyra web: CRUD delete failed: {error}");
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
    database_url: Option<&str>,
    audit_table: Option<&str>,
    audit_chain: bool,
    actor_user_id: Option<i64>,
) -> Response {
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
    let query = format!(
        "UPDATE {} SET {} = NULL WHERE {} = :id AND {} IS NOT NULL",
        quote_identifier(&crud.table),
        quote_identifier(&soft_delete.column),
        quote_identifier("id"),
        quote_identifier(&soft_delete.column)
    );
    let details = format!(
        "table={};operation=crud.restore;record_id={}",
        audit_component(&crud.table),
        id
    );
    let mut queries = vec![zelyra_database::Query {
        sql: query,
        params: vec![("id".into(), zelyra_database::QueryValue::Int(id))],
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
    if let Err(error) = zelyra_database::execute_mariadb_queries(database_url, &queries, true) {
        eprintln!("zelyra web: CRUD restore failed: {error}");
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
    actor_user_id: Option<i64>,
    before_values: Option<&HashMap<String, String>>,
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
    if let (Some(audit_table), Some(audit_event)) =
        (form.audit_table.as_deref(), form.audit_event.as_deref())
    {
        let (record_id, details) =
            form_audit_details(form, audit_event, path_params, before_values, values);
        queries.extend(audit_insert_queries(
            audit_table,
            form.audit_chain,
            actor_user_id,
            audit_event,
            record_id,
            &details,
        ));
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
    render_form_with_language(
        route,
        values,
        errors,
        notice,
        relation_options,
        UiLanguage::English,
    )
}

fn render_form_with_language(
    route: &FormRoute,
    values: &HashMap<String, String>,
    errors: &[FieldError],
    notice: Option<&str>,
    relation_options: &HashMap<String, Vec<SelectOption>>,
    language: UiLanguage,
) -> String {
    let mut html = String::new();
    let form_kind = if route.form.name.ends_with("Create") {
        Some(("form.create_title", "form.create_submit"))
    } else if route.form.name.ends_with("Edit") {
        Some(("form.edit_title", "form.save_submit"))
    } else {
        None
    };
    let generated_resource = form_kind.map(|_| {
        route
            .form
            .name
            .strip_suffix("Create")
            .or_else(|| route.form.name.strip_suffix("Edit"))
            .unwrap_or_else(|| route.form.table.as_deref().unwrap_or("record"))
    });
    let generated_title = form_kind
        .zip(generated_resource)
        .map(|((title_key, _), resource)| {
            field_text(
                language,
                title_key,
                &localized_identifier_reference(language, resource),
            )
        });
    if let Some(title) = route
        .form_view
        .title
        .as_ref()
        .map(|title| localize_user_text(language, title))
        .or(generated_title)
    {
        html.push_str("<h1>");
        html.push_str(&html_escape_preserving_locale_references(&title));
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
        html.push_str(&html_escape_preserving_locale_references(
            &localize_user_text(language, notice),
        ));
        html.push_str("</p>");
    }
    for field in &route.form.fields {
        let label = field
            .label
            .as_deref()
            .map(|label| localize_user_text(language, label))
            .unwrap_or_else(|| localized_identifier_reference(language, &field.name));
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
        html.push_str(&html_escape_preserving_locale_references(&label));
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
                html.push_str(&format!(
                    "<option value=\"\">-- {} --</option>",
                    tr(language, "action.select")
                ));
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
                html.push_str(&html_escape_preserving_locale_references(
                    &localize_user_text(language, placeholder),
                ));
                html.push('"');
            }
            if field.readonly {
                html.push_str(" readonly");
            }
            html.push('>');
        }
        for error in field_errors {
            html.push_str("<p class=\"zelyra-error\">");
            html.push_str(&html_escape_preserving_locale_references(
                &localized_validation_message(language, &error.message),
            ));
            html.push_str("</p>");
        }
        html.push_str("</div>");
    }
    html.push_str("<button type=\"submit\">");
    let default_submit = form_kind
        .map(|(_, submit_key)| tr(language, submit_key))
        .unwrap_or(tr(language, "form.submit"));
    let submit = route
        .form_view
        .submit
        .as_deref()
        .map(|submit| localize_user_text(language, submit))
        .unwrap_or(default_submit);
    html.push_str(&html_escape_preserving_locale_references(&submit));
    html.push_str("</button></form>");
    if cards {
        html.push_str("</section>");
    }
    html
}

fn localized_validation_message(language: UiLanguage, message: &str) -> String {
    let key = match message {
        "unknown form field" => "validation.unknown_field",
        "read-only field cannot be submitted" => "validation.readonly",
        "value is required" => "validation.required",
        "must be an integer" => "validation.integer",
        "must be an unsigned integer" => "validation.unsigned_integer",
        "must be a number" => "validation.number",
        "must be true or false" => "validation.boolean",
        "must be a valid email address" => "validation.email",
        "must be a valid HTTP URL" => "validation.url",
        "must be an integer identifier" => "validation.id",
        "must be a UUID" => "validation.uuid",
        _ => {
            if let Some(maximum) = message.strip_prefix("value exceeds maximum length of ") {
                return i18n::parameterized_reference("validation.max_length", "max", maximum);
            }
            return message.to_owned();
        }
    };
    tr(language, key).to_owned()
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

pub fn localized_identifier(language: UiLanguage, name: &str) -> String {
    if let Some(label) = locale_identifier(language, name) {
        return label.to_owned();
    }
    if language == UiLanguage::English {
        return humanize(name);
    }
    let words = name
        .to_ascii_lowercase()
        .split('_')
        .map(|word| locale_identifier(language, word).unwrap_or(word))
        .collect::<Vec<_>>()
        .join(" ");
    let mut characters = words.chars();
    characters.next().map_or(words.clone(), |first| {
        first.to_uppercase().collect::<String>() + characters.as_str()
    })
}

fn localized_identifier_reference(language: UiLanguage, name: &str) -> String {
    let key = format!("identifier.{}", name.to_ascii_lowercase());
    if i18n::valid_catalog_key(&key) {
        i18n::reference(&key)
    } else {
        localized_identifier(language, name)
    }
}

fn localized_identifier_marker(language: UiLanguage, name: &str) -> String {
    let key = format!("identifier.{}", name.to_ascii_lowercase());
    if i18n::valid_catalog_key(&key) {
        format!("@i18n:{key}")
    } else {
        localized_identifier(language, name)
    }
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

    #[test]
    fn chained_audit_insert_uses_previous_hash_and_sha256() {
        let queries = audit_insert_queries(
            "audit_log",
            true,
            Some(7),
            "crud.update",
            Some(42),
            "table=customers;changed=name",
        );
        assert_eq!(queries.len(), 3);
        assert!(queries[1].sql.contains("INTO @zelyra_prev_hash"));
        assert!(queries[2].sql.contains("previous_hash"));
        assert!(queries[2].sql.contains("SHA2(CONCAT"));
        assert!(queries[2].sql.contains("DATE_FORMAT(CURRENT_TIMESTAMP"));
    }

    fn router() -> Router {
        Router::new(vec![Route {
            path: "/hello/{name}".into(),
            html: "<h1>Hello, {name}!</h1>".into(),
            query: Vec::new(),
            page_size: None,
            sort_columns: Vec::new(),
            search_columns: Vec::new(),
            filters: Vec::new(),
            data: Vec::new(),
            requires_auth: false,
            permissions: Vec::new(),
        }])
    }

    #[test]
    fn applies_crud_layout_only_to_html_responses() {
        let response = apply_generated_layout(
            Response::html(200, "<main>CRUD content</main>"),
            Some("<body>\u{0}ZELYRA_CRUD_CONTENT\u{0}</body>"),
        );
        assert_eq!(response.body, "<body><main>CRUD content</main></body>");

        let redirect = apply_generated_layout(
            Response::redirect("/customers"),
            Some("<body>\u{0}ZELYRA_CRUD_CONTENT\u{0}</body>"),
        );
        assert_eq!(redirect.location.as_deref(), Some("/customers"));
        assert!(redirect.body.is_empty());
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
            post_only: false,
            audit_table: None,
            audit_event: None,
            audit_chain: false,
            layout_html: None,
        }
    }

    fn default_shell_crud(layout_html: Option<String>) -> CrudRoute {
        CrudRoute {
            path: "/machines".into(),
            title: "Maschinen".into(),
            table: "machines".into(),
            list_columns: Vec::new(),
            search_columns: Vec::new(),
            filter_columns: Vec::new(),
            list_view: CrudListViewDef::default(),
            detail_view: CrudDetailViewDef::default(),
            delete_view: CrudDeleteViewDef::default(),
            loading_view: CrudLoadingViewDef::default(),
            error_view: CrudErrorViewDef::default(),
            layout_html,
            soft_delete: None,
            actions: Vec::new(),
            requires_auth: false,
            permissions: Vec::new(),
            create_permissions: Vec::new(),
            edit_permissions: Vec::new(),
            delete_permissions: Vec::new(),
            csrf: CsrfProtection::new("crud-csrf"),
            schema: Schema {
                database: None,
                tables: Vec::new(),
            },
        }
    }

    #[test]
    fn generated_crud_routes_use_the_default_localized_shell_and_crud_guide() {
        let mut create_form = form_route();
        create_form.path = "/machines/new".into();
        create_form.action = "/machines/new".into();
        create_form.form.name = "MachineCreate".into();
        let mut edit_form = create_form.clone();
        edit_form.path = "/machines/{id}/edit".into();
        edit_form.action = "/machines/{id}/edit".into();
        edit_form.form.name = "MachineEdit".into();
        let app = WebApp::new(Vec::new(), vec![create_form, edit_form])
            .with_cruds(vec![default_shell_crud(None)])
            .with_ui_settings(UiLanguage::German, UiLevel::Learn);

        for path in [
            "/machines",
            "/machines/new",
            "/machines/7",
            "/machines/7/edit",
            "/machines/7/delete",
            "/machines/7/restore",
        ] {
            let request =
                parse_request(&format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n")).unwrap();
            let response = app.dispatch(&request);
            assert!(
                response.body.contains("class=\"zelyra-app\""),
                "default app shell missing for {path}"
            );
            assert!(response.body.contains("lang=\"de\""));
            assert!(response.body.contains("data-zelyra-theme=\"default\""));
            assert!(response
                .body
                .contains("<a href=\"/machines\" aria-label=\"Maschinen\" aria-current=\"page\">"));
        }
    }

    #[test]
    fn generated_crud_learning_guide_uses_general_crud_guidance() {
        let html = append_learning_assistant(
            "<html><body><main>Inventory</main></body></html>",
            "/inventory",
            UiLanguage::German,
            true,
        );
        let html = localize_html(&html, UiLanguage::German);
        assert!(html.contains("Deine Zelyra-CRUD-Lernhilfe"));
        assert!(html.contains("Ein Feld in der Verwaltung ergänzen"));
        assert!(html.contains("list, search, filter oder form"));
    }

    #[test]
    fn default_application_shell_escapes_project_navigation_labels_and_paths() {
        let context = DefaultUiContext {
            current_path: "/machines".into(),
            current_label: "Machines <script>".into(),
            navigation: vec![DefaultNavigationLink {
                path: "/machines\" onmouseover=\"alert(1)".into(),
                label: "Machines <script>".into(),
            }],
        };
        let html =
            render_default_application_shell("<h1>Machines</h1>", &context, UiLanguage::English);

        assert!(html.contains("Machines &lt;script&gt;"));
        assert!(html.contains("href=\"/machines&quot; onmouseover=&quot;alert(1)\""));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn default_application_shell_does_not_rewrap_complete_html_documents() {
        let context = DefaultUiContext {
            current_path: "/machines".into(),
            current_label: "Machines".into(),
            navigation: Vec::new(),
        };
        for document in [
            "<!doctype html><html><body>Existing shell</body></html>",
            "  <HTML><body>Existing shell</body></HTML>",
        ] {
            assert_eq!(
                render_default_application_shell(document, &context, UiLanguage::English),
                document
            );
        }
    }

    #[test]
    fn explicitly_configured_crud_layout_takes_precedence_over_the_default_shell() {
        let app = WebApp::new(Vec::new(), Vec::new()).with_cruds(vec![default_shell_crud(Some(
            format!("<div class=\"custom-shell\">{CRUD_LAYOUT_CONTENT_MARKER}</div>"),
        ))]);
        let request = parse_request("GET /machines HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let response = app.dispatch(&request);

        assert!(response.body.starts_with("<div class=\"custom-shell\">"));
        assert!(!response.body.contains("class=\"zelyra-app\""));
        assert!(!response.body.contains("data-zelyra-theme=\"default\""));
    }

    #[test]
    fn default_shell_covers_standalone_forms_and_tableviews_but_not_custom_pages() {
        let form = form_route();
        let tableview = TableViewRoute {
            path: "/reports".into(),
            title: "Reports".into(),
            source: "report query".into(),
            columns: Vec::new(),
            filters: Vec::new(),
            searchable: false,
            sortable: false,
            page_size: None,
            requires_auth: false,
            permissions: Vec::new(),
        };
        let app = WebApp::new(Vec::new(), vec![form])
            .with_cruds(vec![default_shell_crud(None)])
            .with_tableviews(vec![tableview])
            .with_ui_settings(UiLanguage::English, UiLevel::Work);

        for path in ["/forms/CustomerCreate", "/reports"] {
            let request =
                parse_request(&format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n")).unwrap();
            let response = app.dispatch(&request);
            assert!(response.body.contains("class=\"zelyra-app\""));
            assert!(response.body.contains("href=\"/forms/CustomerCreate\""));
        }

        let page = Route {
            path: "/custom".into(),
            html: "<main>My custom page</main>".into(),
            query: Vec::new(),
            page_size: None,
            sort_columns: Vec::new(),
            search_columns: Vec::new(),
            filters: Vec::new(),
            data: Vec::new(),
            requires_auth: false,
            permissions: Vec::new(),
        };
        let custom_app = WebApp::new(vec![page], Vec::new())
            .with_project_theme_css(Some(":root { --zelyra-color-accent: #e04b67; }".into()));
        let request = parse_request("GET /custom HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let response = custom_app.dispatch(&request);
        assert_eq!(response.body, "<main>My custom page</main>");
        assert!(!response.body.contains("data-zelyra-theme"));
        assert!(!response.body.contains(PROJECT_THEME_CSS_PATH));
    }

    #[test]
    fn generated_login_page_uses_the_localized_default_shell() {
        let auth = AuthRoute {
            table: "users".into(),
            session_table: None,
            permissions_table: None,
            roles_table: None,
            role_permissions_table: None,
            audit_table: None,
            audit_chain: false,
            admin_path: Some("/admin".into()),
            admin_permission: None,
            admin_role: None,
            schema: Schema {
                database: None,
                tables: Vec::new(),
            },
            csrf: CsrfProtection::new("login-csrf"),
        };
        let app = WebApp::new(Vec::new(), Vec::new())
            .with_auth_route(auth)
            .with_ui_settings(UiLanguage::German, UiLevel::Work);
        let request = parse_request("GET /login HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let response = app.dispatch(&request);

        assert!(response.body.contains("class=\"zelyra-app\""));
        assert!(response.body.contains("lang=\"de\""));
        assert!(response.body.contains("<title>Anmelden | Zelyra</title>"));
        assert!(response.body.contains("Zum Inhalt springen"));
        assert!(response.body.contains("id=\"zelyra-content\""));
        assert!(response.body.contains("href=\"/\""));
    }

    #[test]
    fn project_theme_css_is_linked_after_the_default_design_and_served_as_css() {
        let route = Route {
            path: "/".into(),
            html: "<html><head></head><body><div class=\"zelyra-app\"><main>Home</main></div></body></html>".into(),
            query: Vec::new(),
            page_size: None,
            sort_columns: Vec::new(),
            search_columns: Vec::new(),
            filters: Vec::new(),
            data: Vec::new(),
            requires_auth: false,
            permissions: Vec::new(),
        };
        let theme_css = ":root { --zelyra-color-accent: #e04b67; }";
        let app =
            WebApp::new(vec![route], Vec::new()).with_project_theme_css(Some(theme_css.into()));
        let page_request = parse_request("GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let page = app.dispatch(&page_request);

        let default_style = page
            .body
            .find("data-zelyra-theme=\"default\"")
            .expect("default design system should be embedded");
        let project_style = page
            .body
            .find("href=\"/__zelyra/theme.css\"")
            .expect("project theme should be linked");
        assert!(default_style < project_style);

        let css_request =
            parse_request("GET /__zelyra/theme.css HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let css = app.dispatch(&css_request);
        assert_eq!(css.status, 200);
        assert_eq!(css.content_type, "text/css; charset=utf-8");
        assert_eq!(css.body, theme_css);
        assert!(css.to_http().contains("Cache-Control: no-cache\r\n"));

        let post_request =
            parse_request("POST /__zelyra/theme.css HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let rejected = app.dispatch(&post_request);
        assert_eq!(rejected.status, 405);
        assert!(rejected.to_http().contains("Allow: GET\r\n"));

        let no_theme = WebApp::new(Vec::new(), Vec::new()).dispatch(&css_request);
        assert_eq!(no_theme.status, 404);
    }

    #[test]
    fn default_design_system_exposes_the_complete_public_theme_token_set() {
        for token in [
            "--zelyra-color-accent",
            "--zelyra-color-accent-strong",
            "--zelyra-color-accent-text",
            "--zelyra-color-accent-soft",
            "--zelyra-color-ink",
            "--zelyra-color-muted",
            "--zelyra-color-border",
            "--zelyra-color-canvas",
            "--zelyra-color-surface",
            "--zelyra-color-surface-subtle",
            "--zelyra-color-sidebar-start",
            "--zelyra-color-sidebar-middle",
            "--zelyra-color-sidebar-end",
            "--zelyra-color-sidebar-foreground",
            "--zelyra-color-sidebar-muted",
            "--zelyra-color-hero-start",
            "--zelyra-color-hero-middle",
            "--zelyra-color-hero-end",
            "--zelyra-color-success-background",
            "--zelyra-color-success-border",
            "--zelyra-color-success-ink",
            "--zelyra-color-danger-background",
            "--zelyra-color-danger-border",
            "--zelyra-color-danger-ink",
            "--zelyra-color-focus",
            "--zelyra-font-body",
            "--zelyra-radius-card",
            "--zelyra-radius-control",
            "--zelyra-content-max-width",
        ] {
            assert!(
                ZELYRA_DESIGN_SYSTEM_CSS.contains(token),
                "default design system misses {token}"
            );
        }
    }

    #[test]
    fn default_design_system_covers_all_generated_state_families_responsively() {
        for selector in [
            ".zelyra-crud-cards",
            ".zelyra-crud-card",
            ".zelyra-crud-detail-card",
            ".zelyra-crud-form-card",
            ".zelyra-action-confirmation",
            ".zelyra-action-error",
            ".zelyra-crud-error",
            ".zelyra-delete-message",
            ".zelyra-query-controls",
            ".zelyra-pagination",
            "main[data-loading-message]",
        ] {
            assert!(
                ZELYRA_DESIGN_SYSTEM_CSS.contains(selector),
                "default design system misses generated state selector {selector}"
            );
        }
        assert!(ZELYRA_DESIGN_SYSTEM_CSS.contains("@media(max-width:700px)"));
        assert!(ZELYRA_DESIGN_SYSTEM_CSS.contains("@media(max-width:560px)"));
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
    fn formats_crud_audit_changes_without_exposing_sensitive_values() {
        let mut route = form_route();
        route.table = Some(zelyra_ast::TableDef {
            name: "customers".into(),
            columns: Vec::new(),
            indexes: Vec::new(),
            uniques: Vec::new(),
            span: zelyra_ast::Span::default(),
        });
        route.form.fields.push(zelyra_ast::FormField {
            name: "password".into(),
            ty: Some(Type::String),
            label: None,
            placeholder: None,
            required: false,
            max: None,
            widget: None,
            readonly: false,
            span: zelyra_ast::Span::default(),
        });
        let before = HashMap::from([
            ("name".into(), "Old customer".into()),
            ("password".into(), "old-secret".into()),
        ]);
        let after = HashMap::from([
            ("name".into(), "New customer".into()),
            ("password".into(), "new-secret".into()),
        ]);
        let path_params = HashMap::from([(String::from("id"), String::from("7"))]);
        let (record_id, details) =
            form_audit_details(&route, "crud.update", &path_params, Some(&before), &after);
        assert_eq!(record_id, Some(7));
        assert!(details.contains("name:Old customer->New customer"));
        assert!(details.contains("password=changed"));
        assert!(!details.contains("old-secret"));
        assert!(!details.contains("new-secret"));
    }

    #[test]
    fn dispatches_literal_and_parameter_routes() {
        let response = router().dispatch("GET", "/hello/Zelyra");
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "<h1>Hello, Zelyra!</h1>");
    }

    #[test]
    fn applies_locale_catalog_theme_and_learning_mode_to_template_views() {
        let route = Route {
            path: "/".into(),
            html: "<html data-zelyra-language><head><title data-zelyra-i18n=\"app.document_title\">Fallback</title></head><body><div class=\"zelyra-app\"><h1 data-zelyra-i18n=\"app.home_title\">Fallback</h1></div></body></html>".into(),
            query: Vec::new(),
            page_size: None,
            sort_columns: Vec::new(),
            search_columns: Vec::new(),
            filters: Vec::new(),
            data: Vec::new(),
            requires_auth: false,
            permissions: Vec::new(),
        };
        let request = parse_request("GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let german = WebApp::new(vec![route.clone()], Vec::new())
            .with_ui_settings(UiLanguage::German, UiLevel::Learn);
        let german_response = german.dispatch(&request);
        assert!(german_response.body.contains("lang=\"de\""));
        assert!(german_response
            .body
            .contains("Maschinenverwaltung | Zelyra"));
        assert!(german_response.body.contains("Alle Maschinen im Blick."));
        assert!(german_response
            .body
            .contains("data-zelyra-theme=\"default\""));
        assert!(
            german_response.body.contains("Zelyra-Lernhilfe")
                || german_response.body.contains("Lernhilfe")
        );

        let english = WebApp::new(vec![route], Vec::new())
            .with_ui_settings(UiLanguage::English, UiLevel::Work);
        let english_response = english.dispatch(&request);
        assert!(english_response.body.contains("lang=\"en\""));
        assert!(english_response.body.contains("Machine workspace | Zelyra"));
        assert!(english_response
            .body
            .contains("A clear view of every machine."));
        assert!(!english_response
            .body
            .contains("<details class=\"zelyra-learning-assistant\""));
    }

    #[test]
    fn project_catalogs_override_markers_escape_html_and_fall_back_to_english() {
        let action_label = localize_user_text(UiLanguage::German, "@i18n:custom.action");
        let route = Route {
            path: "/".into(),
            html: format!(
                r#"<main><h1 data-zelyra-i18n="custom.title"></h1><p data-zelyra-i18n="app.home_title"></p><strong>{action_label}</strong></main>"#
            ),
            query: Vec::new(),
            page_size: None,
            sort_columns: Vec::new(),
            search_columns: Vec::new(),
            filters: Vec::new(),
            data: Vec::new(),
            requires_auth: false,
            permissions: Vec::new(),
        };
        let request = parse_request("GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let mut catalogs = ProjectUiCatalogs::default();
        catalogs
            .set_json(
                UiLanguage::English,
                r#"{"custom.title":"Project title","custom.action":"<script>alert(1)</script>","app.home_title":"Project home"}"#,
            )
            .unwrap();
        catalogs
            .set_json(UiLanguage::German, r#"{"custom.title":"Projekttitel"}"#)
            .unwrap();

        let app = WebApp::new(vec![route], Vec::new())
            .with_ui_settings(UiLanguage::German, UiLevel::Work)
            .with_project_ui_catalogs(catalogs);
        let response = app.dispatch(&request);
        assert!(response.body.contains(">Projekttitel</h1>"));
        assert!(response.body.contains(">Project home</p>"));
        assert!(response
            .body
            .contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(!response.body.contains("<script>alert(1)</script>"));
    }

    #[test]
    fn project_catalogs_override_generated_ui_and_parameterized_labels() {
        let auth = AuthRoute {
            table: "users".into(),
            session_table: None,
            permissions_table: None,
            roles_table: None,
            role_permissions_table: None,
            audit_table: None,
            audit_chain: false,
            admin_path: None,
            admin_permission: None,
            admin_role: None,
            schema: Schema {
                database: None,
                tables: Vec::new(),
            },
            csrf: CsrfProtection::new("catalog-csrf"),
        };
        let mut catalogs = ProjectUiCatalogs::default();
        catalogs
            .set_json(
                UiLanguage::German,
                r#"{"shell.brand_descriptor":"Eigene Oberfläche","auth.login_title":"Projektanmeldung","auth.email":"E-Mail-Adresse","learning.button":"Projekt-Hilfe öffnen","query.filter_value":"Wert für {field}","identifier.department":"Kostenstelle"}"#,
            )
            .unwrap();
        let app = WebApp::new(Vec::new(), Vec::new())
            .with_auth_route(auth)
            .with_ui_settings(UiLanguage::German, UiLevel::Learn)
            .with_project_ui_catalogs(catalogs.clone());
        let request = parse_request("GET /login HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let response = app.dispatch(&request);

        assert!(response.body.contains("Eigene Oberfläche"));
        assert!(response.body.contains("<h1>Projektanmeldung</h1>"));
        assert!(response
            .body
            .contains("<label for=\"email\">E-Mail-Adresse</label>"));
        assert!(response.body.contains("Projekt-Hilfe öffnen"));
        assert!(!response.body.contains(LOCALE_REFERENCE_START));

        let filter_label = field_text(
            UiLanguage::German,
            "query.filter_value",
            &localized_identifier_reference(UiLanguage::German, "department"),
        );
        assert_eq!(
            resolve_locale_references(&filter_label, UiLanguage::German, &catalogs),
            "Wert für Kostenstelle"
        );
    }

    #[test]
    fn html_escaped_user_text_cannot_forge_a_locale_reference() {
        let forged_reference =
            format!("{LOCALE_REFERENCE_START}shell.brand_descriptor{LOCALE_REFERENCE_END}");
        let escaped = html_escape(&forged_reference);
        let resolved =
            resolve_locale_references(&escaped, UiLanguage::German, &ProjectUiCatalogs::default());

        assert_eq!(resolved, escaped);
        assert!(!resolved.contains("Zuverlässige Business-Anwendungen"));
    }

    #[test]
    fn framework_errors_auth_labels_and_validation_use_the_locale_catalog() {
        let error = localize_html(
            "<main><h1>403 Forbidden</h1><p>The Database capability is not granted.</p></main>",
            UiLanguage::German,
        );
        assert!(error.contains("403 Zugriff verweigert"));
        assert!(error.contains("Die Datenbank-Capability wurde nicht freigegeben."));
        assert_eq!(
            framework_text(
                UiLanguage::German,
                "Operator `contains` is not supported for filter `active`."
            ),
            Some("Der Operator `contains` wird für den Filter `active` nicht unterstützt.".into())
        );

        let auth = AuthRoute {
            table: "users".into(),
            session_table: None,
            permissions_table: None,
            roles_table: None,
            role_permissions_table: None,
            audit_table: None,
            audit_chain: false,
            admin_path: None,
            admin_permission: None,
            admin_role: None,
            schema: Schema {
                database: None,
                tables: Vec::new(),
            },
            csrf: CsrfProtection::new("csrf-token"),
        };
        let login = localize_html(&render_login(&auth, UiLanguage::German), UiLanguage::German);
        assert!(login.contains("<h1>Anmelden</h1>"));
        assert!(login.contains("<label for=\"email\">E-Mail</label>"));
        assert!(!login.contains(">Login<"));

        assert_eq!(
            localize_html(
                &localized_validation_message(UiLanguage::German, "value is required"),
                UiLanguage::German,
            ),
            "Ein Wert ist erforderlich."
        );
        assert_eq!(
            localize_html(
                &localized_validation_message(
                    UiLanguage::German,
                    "value exceeds maximum length of 80",
                ),
                UiLanguage::German,
            ),
            "Der Wert überschreitet die maximale Länge von 80 Zeichen."
        );
    }

    #[test]
    fn generated_form_titles_buttons_and_field_names_use_selected_language() {
        let mut route = form_route();
        route.form.fields[0].label = None;
        let html = render_form_with_language(
            &route,
            &HashMap::new(),
            &[],
            None,
            &HashMap::new(),
            UiLanguage::German,
        );
        let html = localize_html(&html, UiLanguage::German);
        assert!(html.contains("<h1>Kunde anlegen</h1>"));
        assert!(html.contains("<label for=\"name\">Name</label>"));
        assert!(html.contains(">Anlegen</button>"));
    }

    #[test]
    fn localized_filter_and_identifier_labels_are_readable_german() {
        assert_eq!(
            localized_identifier(UiLanguage::German, "machine_number"),
            "Maschinennummer"
        );
        assert_eq!(
            localized_identifier(UiLanguage::German, "department"),
            "Abteilung"
        );
        assert_eq!(
            localize_html(
                &FilterOperator::Contains.label(UiLanguage::German),
                UiLanguage::German,
            ),
            "enthält"
        );
        assert_eq!(
            localize_html(
                &field_text(UiLanguage::German, "query.filter_value", "Aktiv"),
                UiLanguage::German,
            ),
            "Wert für Aktiv"
        );
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
    fn dispatches_the_matching_method_when_api_routes_share_a_path() {
        let app = WebApp::new(Vec::new(), Vec::new()).with_apis(vec![
            ApiRoute::new("GET", "/setup", |_request, _| Response::html(200, "get")),
            ApiRoute::new("POST", "/setup", |_request, _| Response::html(200, "post")),
        ]);
        let get = parse_request("GET /setup HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let post = parse_request("POST /setup HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        assert_eq!(app.dispatch(&get).body, "get");
        assert_eq!(app.dispatch(&post).body, "post");
    }

    #[test]
    fn adds_cors_headers_for_an_allowed_api_origin() {
        let policy = CorsPolicy::new(vec!["http://localhost:5173".into()], false).unwrap();
        let app = WebApp::new(Vec::new(), Vec::new())
            .with_apis(vec![ApiRoute::new("GET", "/health", |_request, _| {
                Response::json(200, "{}")
            })])
            .with_cors(policy);
        let request = parse_request(
            "GET /health HTTP/1.1\r\nHost: localhost:3000\r\nOrigin: http://localhost:5173\r\n\r\n",
        )
        .unwrap();
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
            "OPTIONS /customers HTTP/1.1\r\nHost: localhost\r\nOrigin: https://app.example\r\nAccess-Control-Request-Method: POST\r\nAccess-Control-Request-Headers: content-type, authorization\r\n\r\n",
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
            "OPTIONS /health HTTP/1.1\r\nHost: localhost\r\nOrigin: https://evil.example\r\nAccess-Control-Request-Method: GET\r\n\r\n",
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
    fn csrf_requires_a_same_origin_browser_request() {
        let csrf = CsrfProtection::new("known-form-token");
        let same_origin = parse_request(
            "POST /save HTTP/1.1\r\nHost: example.test\r\nOrigin: http://example.test\r\n\r\n",
        )
        .unwrap();
        assert!(verify_csrf_request(
            &same_origin,
            &csrf,
            Some("known-form-token")
        ));

        let cross_origin = parse_request(
            "POST /save HTTP/1.1\r\nHost: example.test\r\nOrigin: https://attacker.test\r\n\r\n",
        )
        .unwrap();
        assert!(!verify_csrf_request(
            &cross_origin,
            &csrf,
            Some("known-form-token")
        ));

        let forwarded_https = parse_request(
            "POST /save HTTP/1.1\r\nHost: example.test\r\nX-Forwarded-Proto: https\r\nOrigin: https://example.test\r\n\r\n",
        )
        .unwrap();
        assert!(verify_csrf_request(
            &forwarded_https,
            &csrf,
            Some("known-form-token")
        ));
        assert!(secure_cookie_attribute(&forwarded_https).contains("Secure"));

        let missing_origin =
            parse_request("POST /save HTTP/1.1\r\nHost: example.test\r\n\r\n").unwrap();
        assert!(!verify_csrf_request(
            &missing_origin,
            &csrf,
            Some("known-form-token")
        ));
    }

    #[test]
    fn csrf_rejects_mismatched_scheme_and_malformed_origins() {
        let csrf = CsrfProtection::new("known-form-token");
        for origin in [
            "http://example.test",
            "https://example.test.evil.test",
            "null",
            "https://user@example.test",
            "https://example.test/path",
        ] {
            let request = parse_request(&format!(
                "POST /save HTTP/1.1\r\nHost: example.test\r\nX-Forwarded-Proto: https\r\nOrigin: {origin}\r\n\r\n"
            ))
            .unwrap();
            assert!(
                !verify_csrf_request(&request, &csrf, Some("known-form-token")),
                "accepted unexpected origin {origin}"
            );
        }

        let referer = parse_request(
            "POST /save HTTP/1.1\r\nHost: example.test:443\r\nX-Forwarded-Proto: https\r\nReferer: https://EXAMPLE.test/path?x=1\r\n\r\n",
        )
        .unwrap();
        assert!(verify_csrf_request(
            &referer,
            &csrf,
            Some("known-form-token")
        ));
    }

    #[test]
    fn host_allowlist_blocks_dns_rebinding_and_accepts_configured_hosts() {
        let health = Route {
            path: "/health".into(),
            html: "ok".into(),
            query: Vec::new(),
            page_size: None,
            sort_columns: Vec::new(),
            search_columns: Vec::new(),
            filters: Vec::new(),
            data: Vec::new(),
            requires_auth: false,
            permissions: Vec::new(),
        };
        let app = WebApp::new(vec![health], Vec::new());
        let rebinding = parse_request(
            "GET /health HTTP/1.1\r\nHost: attacker.example:3000\r\nOrigin: http://attacker.example:3000\r\n\r\n",
        )
        .unwrap();
        let response = app.dispatch(&rebinding);
        assert_eq!(response.status, 400);
        assert!(response.body.contains("This request host is not allowed."));

        let german_app = WebApp::new(
            vec![Route {
                path: "/health".into(),
                html: "ok".into(),
                query: Vec::new(),
                page_size: None,
                sort_columns: Vec::new(),
                search_columns: Vec::new(),
                filters: Vec::new(),
                data: Vec::new(),
                requires_auth: false,
                permissions: Vec::new(),
            }],
            Vec::new(),
        )
        .with_ui_settings(UiLanguage::German, UiLevel::Work);
        let german_response = german_app.dispatch(&rebinding);
        assert_eq!(german_response.status, 400);
        assert!(german_response
            .body
            .contains("Der Hostname dieser Anfrage ist nicht freigegeben."));

        let configured = app
            .with_allowed_hosts(vec!["APP.Example".to_owned()])
            .unwrap();
        let request = parse_request(
            "GET /health HTTP/1.1\r\nHost: app.example:8080\r\nOrigin: http://app.example:8080\r\n\r\n",
        )
        .unwrap();
        assert_eq!(configured.dispatch(&request).status, 200);
    }

    #[test]
    fn host_allowlist_normalizes_equivalent_ipv6_literals() {
        let app = WebApp::new(Vec::new(), Vec::new());
        let request = parse_request("GET / HTTP/1.1\r\nHost: [0:0:0:0:0:0:0:1]\r\n\r\n").unwrap();

        assert_eq!(app.dispatch(&request).status, 404);
    }

    #[test]
    fn host_allowlist_rejects_missing_host_for_browser_requests_and_invalid_config() {
        let health = Route {
            path: "/health".into(),
            html: "ok".into(),
            query: Vec::new(),
            page_size: None,
            sort_columns: Vec::new(),
            search_columns: Vec::new(),
            filters: Vec::new(),
            data: Vec::new(),
            requires_auth: false,
            permissions: Vec::new(),
        };
        let app = WebApp::new(vec![health], Vec::new());
        let missing_host =
            parse_request("GET /health HTTP/1.1\r\nOrigin: http://localhost\r\n\r\n").unwrap();
        assert_eq!(app.dispatch(&missing_host).status, 400);

        assert!(app
            .clone()
            .with_allowed_hosts(Vec::<String>::new())
            .is_err());
        assert!(app
            .clone()
            .with_allowed_hosts(vec!["https://app.example".to_owned()])
            .is_err());
        assert!(app
            .clone()
            .with_allowed_hosts(vec!["app.example:8080".to_owned()])
            .is_err());
        assert!(app
            .with_allowed_hosts(vec!["bad..example".to_owned()])
            .is_err());
    }

    #[test]
    fn secure_session_cookie_is_set_when_tls_terminates_at_a_proxy() {
        let request = parse_request(
            "POST /login HTTP/1.1\r\nHost: example.test\r\nX-Forwarded-Proto: https\r\n\r\n",
        )
        .unwrap();
        assert_eq!(secure_cookie_attribute(&request), "; Secure");

        let local_request =
            parse_request("POST /login HTTP/1.1\r\nHost: 127.0.0.1:3000\r\n\r\n").unwrap();
        assert_eq!(secure_cookie_attribute(&local_request), "");
    }

    #[test]
    fn logout_clears_session_cookie_with_the_matching_secure_attribute() {
        let auth = AuthRoute {
            table: "users".into(),
            session_table: None,
            permissions_table: None,
            roles_table: None,
            role_permissions_table: None,
            audit_table: None,
            audit_chain: false,
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
            .with_auth_route(auth)
            .with_allowed_hosts(vec!["example.test".into()])
            .unwrap();
        let request = parse_request(
            "POST /logout HTTP/1.1\r\nHost: example.test\r\nX-Forwarded-Proto: https\r\nOrigin: https://example.test\r\nCookie: zelyra_session=old-token\r\n\r\n_zelyra_csrf=csrf-token",
        )
        .unwrap();
        let response = app.dispatch(&request);
        assert_eq!(response.status, 303);
        let cookie = response
            .headers
            .iter()
            .find(|(name, _)| name == "Set-Cookie")
            .map(|(_, value)| value.as_str())
            .unwrap();
        assert!(cookie.contains("Max-Age=0"));
        assert!(cookie.contains("; Secure"));
    }

    #[test]
    fn escapes_route_parameters() {
        let response = router().dispatch("GET", "/hello/<script>");
        assert_eq!(response.body, "<h1>Hello, &lt;script&gt;!</h1>");
    }

    #[test]
    fn escapes_loaded_page_data_fields() {
        let params = HashMap::new();
        let data = HashMap::from([("customer.name".into(), "<script>".into())]);
        assert_eq!(
            render_template(
                "<h1>{customer.name}</h1>",
                &params,
                &LoadedRouteData {
                    values: data,
                    collections: HashMap::new(),
                },
            ),
            "<h1>&lt;script&gt;</h1>"
        );
    }

    #[test]
    fn validates_typed_page_query_values() {
        assert!(matches!(
            page_query_value(&Type::Option(Box::new(Type::UInt)), "25"),
            Ok(QueryValue::UInt(25))
        ));
        assert!(matches!(
            page_query_value(&Type::Bool, "true"),
            Ok(QueryValue::Bool(true))
        ));
        assert!(page_query_value(&Type::Int, "not-a-number").is_err());
        assert!(page_query_value(&Type::Bool, "yes").is_err());
    }

    #[test]
    fn validates_page_pagination_state_and_wraps_sql_safely() {
        assert_eq!(page_number(&HashMap::new()), Ok(1));
        assert_eq!(
            page_number(&HashMap::from([("page".into(), "3".into())])),
            Ok(3)
        );
        assert!(page_number(&HashMap::from([("page".into(), "0".into())])).is_err());
        assert!(page_number(&HashMap::from([("page".into(), "nope".into())])).is_err());
        let (query, parameters) = page_collection_query(
            " SELECT id FROM customers; ",
            PageCollectionQueryOptions {
                search: None,
                search_columns: &[],
                filters: &[],
                sort: None,
                order: "ASC",
                page_size: Some(25),
                page: 1,
            },
        )
        .unwrap();
        assert_eq!(
            query,
            "SELECT zelyra_page.* FROM (SELECT id FROM customers) AS zelyra_page LIMIT :zelyra_page_limit OFFSET :zelyra_page_offset"
        );
        assert_eq!(parameters.len(), 2);
        let (sorted_query, _) = page_collection_query(
            "SELECT id FROM customers",
            PageCollectionQueryOptions {
                search: None,
                search_columns: &[],
                filters: &[],
                sort: Some("name"),
                order: "DESC",
                page_size: None,
                page: 1,
            },
        )
        .unwrap();
        assert_eq!(
            sorted_query,
            "SELECT zelyra_page.* FROM (SELECT id FROM customers) AS zelyra_page ORDER BY zelyra_page.`name` DESC"
        );
        let (searched_query, search_parameters) = page_collection_query(
            "SELECT id, name, email FROM customers",
            PageCollectionQueryOptions {
                search: Some("Ada"),
                search_columns: &["name".into(), "email".into()],
                filters: &[],
                sort: Some("name"),
                order: "ASC",
                page_size: Some(25),
                page: 1,
            },
        )
        .unwrap();
        assert!(searched_query.contains(
            "WHERE (CAST(zelyra_page.`name` AS CHAR) LIKE CONCAT('%', :zelyra_page_search, '%') OR CAST(zelyra_page.`email` AS CHAR) LIKE CONCAT('%', :zelyra_page_search, '%'))"
        ));
        assert!(searched_query.contains("ORDER BY zelyra_page.`name` ASC"));
        assert_eq!(search_parameters.len(), 3);
        let route = Route {
            path: "/customers".into(),
            html: String::new(),
            query: Vec::new(),
            page_size: None,
            sort_columns: vec!["name".into(), "created_at".into()],
            search_columns: Vec::new(),
            filters: Vec::new(),
            data: Vec::new(),
            requires_auth: false,
            permissions: Vec::new(),
        };
        assert_eq!(
            page_sort_state(
                &route,
                &HashMap::from([
                    ("sort".into(), "name".into()),
                    ("order".into(), "desc".into())
                ])
            ),
            Ok((Some("name".into()), "DESC"))
        );
        assert!(page_sort_state(&route, &HashMap::from([("sort".into(), "id".into())])).is_err());
        assert!(page_sort_state(
            &route,
            &HashMap::from([("order".into(), "sideways".into())])
        )
        .is_err());
        let mut searchable_route = route.clone();
        searchable_route.search_columns = vec!["name".into(), "email".into()];
        assert_eq!(
            page_search_state(
                &searchable_route,
                &HashMap::from([("search".into(), "Ada".into())])
            ),
            Ok(Some("Ada".into()))
        );
        assert!(
            page_search_state(&route, &HashMap::from([("search".into(), "Ada".into())])).is_err()
        );
        let mut filter_route = route;
        filter_route.filters = vec![
            TableViewFilter {
                name: "name".into(),
                kind: TableViewFilterKind::Text,
            },
            TableViewFilter {
                name: "quantity".into(),
                kind: TableViewFilterKind::Numeric,
            },
        ];
        let filter_values = HashMap::from([
            ("filter_name".into(), "Ada".into()),
            ("filter_name__operator".into(), "contains".into()),
            ("filter_quantity".into(), "10".into()),
            ("filter_quantity__operator".into(), "gte".into()),
        ]);
        let page_filters = page_filter_state(&filter_route, &filter_values).unwrap();
        assert_eq!(page_filters.len(), 2);
        let (filtered_query, filter_parameters) = page_collection_query(
            "SELECT id, name, quantity FROM customers",
            PageCollectionQueryOptions {
                search: None,
                search_columns: &[],
                filters: &page_filters,
                sort: None,
                order: "ASC",
                page_size: None,
                page: 1,
            },
        )
        .unwrap();
        assert!(filtered_query.contains(
            "WHERE zelyra_page.`name` LIKE CONCAT('%', :zelyra_page_filter_name, '%') AND zelyra_page.`quantity` >= :zelyra_page_filter_quantity"
        ));
        assert_eq!(filter_parameters.len(), 2);
        assert!(page_filter_state(
            &filter_route,
            &HashMap::from([("filter_name__operator".into(), "gte".into())])
        )
        .is_err());
        assert!(page_filter_state(
            &filter_route,
            &HashMap::from([("filter_missing".into(), "value".into())])
        )
        .is_err());
    }

    #[test]
    fn renders_each_loaded_page_collection_row_with_escaping() {
        let params = HashMap::new();
        let data = LoadedRouteData {
            values: HashMap::new(),
            collections: HashMap::from([(
                "customers".into(),
                vec![
                    HashMap::from([("name".into(), "Ada".into())]),
                    HashMap::from([("name".into(), "<Grace>".into())]),
                ],
            )]),
        };
        assert_eq!(
            render_template(
                "<ul>\nfor customer in customers {<li>{customer.name}</li>}\n</ul>",
                &params,
                &data,
            ),
            "<ul>\n<li>Ada</li><li>&lt;Grace&gt;</li>\n</ul>"
        );
    }

    #[test]
    fn renders_page_query_controls_and_preserves_state() {
        let route = Route {
            path: "/customers".into(),
            html: "<html><body><h1>Customers</h1><p>{total}/{pages}</p></body></html>".into(),
            query: Vec::new(),
            page_size: Some(2),
            sort_columns: vec!["name".into()],
            search_columns: vec!["name".into()],
            filters: vec![TableViewFilter {
                name: "name".into(),
                kind: TableViewFilterKind::Text,
            }],
            data: vec![RouteData {
                name: "customers".into(),
                query: "SELECT id, name FROM customers".into(),
                fields: vec!["id".into(), "name".into()],
                collection: true,
                optional: false,
            }],
            requires_auth: false,
            permissions: Vec::new(),
        };
        let query_values = HashMap::from([
            ("filter_name".into(), "Ada".into()),
            ("filter_name__operator".into(), "contains".into()),
            ("order".into(), "desc".into()),
            ("page".into(), "2".into()),
            ("search".into(), "A".into()),
            ("sort".into(), "name".into()),
        ]);
        let data = LoadedRouteData {
            values: HashMap::from([
                ("page".into(), "2".into()),
                ("pages".into(), "3".into()),
                ("total".into(), "5".into()),
            ]),
            collections: HashMap::new(),
        };
        let html = localize_html(
            &render_page(&route, "/customers", &HashMap::new(), &query_values, &data),
            UiLanguage::English,
        );
        assert!(html.contains("<body><form method=\"get\">"));
        assert!(html.contains("name=\"filter_name\" value=\"Ada\""));
        assert!(html.contains("Page 2 of 3"));
        assert!(html.contains(
            "/customers?page=1&amp;filter_name=Ada&amp;filter_name__operator=contains&amp;order=desc&amp;search=A&amp;sort=name"
        ));
        assert!(html.contains("<p>5/3</p>"));
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
    fn rejects_duplicate_security_sensitive_request_headers() {
        for header in [
            "Host",
            "Origin",
            "Referer",
            "X-Forwarded-Proto",
            "Cookie",
            "Authorization",
        ] {
            let raw = format!("GET / HTTP/1.1\r\n{header}: first\r\n{header}: second\r\n\r\n");
            let error = parse_request(&raw).unwrap_err();
            assert!(
                error
                    .message
                    .contains("duplicate security-sensitive header"),
                "did not reject duplicate {header}"
            );
        }
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
        assert!(wire.contains("Referrer-Policy: same-origin\r\n"));
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
            audit_chain: false,
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
            audit_chain: false,
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

        let mut post_only_route = route.clone();
        post_only_route.post_only = true;
        let get_request = parse_request("GET /forms/action HTTP/1.1\r\n\r\n").unwrap();
        let response = dispatch_form(&post_only_route, &get_request, &HashMap::new(), None, None);
        assert_eq!(response.status, 405);
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
            UiLanguage::English,
        );
        let html = localize_html(&html, UiLanguage::English);
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
            UiLanguage::English,
        );
        let html = localize_html(&html, UiLanguage::English);
        assert!(html.contains(
            "<fieldset class=\"zelyra-query-controls\"><legend>Search and filters</legend>"
        ));
        assert!(html.contains("name=\"filter_orders__operator\""));
        assert!(html.contains("for=\"filter_orders__operator\">Filter Orders operator</label>"));
        assert!(html.contains("for=\"filter_orders\">Filter Orders value</label>"));
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
        let unordered_query_values = HashMap::from([
            ("filter_z".into(), "2".into()),
            ("filter_a".into(), "1".into()),
        ]);
        assert_eq!(
            tableview_page_url(&tableview, &unordered_query_values, "", "id", "ASC", 1,),
            "/views/customers?page=1&sort=id&order=asc&filter_a=1&filter_z=2"
        );
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
            loading_view: CrudLoadingViewDef::default(),
            error_view: CrudErrorViewDef::default(),
            layout_html: None,
            soft_delete: None,
            actions: Vec::new(),
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
                archived: false,
                success: Some("Saved <unsafe>"),
                success_title: Some("Completed"),
            },
        );
        let html = localize_html(&html, UiLanguage::English);
        assert!(html.contains("&lt;unsafe&gt;"));
        assert!(html.contains(
            "<fieldset class=\"zelyra-query-controls\"><legend>Search and filters</legend>"
        ));
        assert!(html.contains("for=\"filter_name__operator\">Filter Name operator</label>"));
        assert!(html.contains("for=\"filter_name\">Filter Name value</label>"));
        assert!(html.contains(
            "class=\"zelyra-success\" role=\"status\"><h2>Completed</h2><p>Saved &lt;unsafe&gt;</p>"
        ));
        assert!(html.contains("value=\"CNC machine\""));
        assert!(html
            .contains("page=1&amp;per_page=1&amp;sort=id&amp;order=asc&amp;search=CNC%20machine"));
        assert!(html
            .contains("page=3&amp;per_page=1&amp;sort=id&amp;order=asc&amp;search=CNC%20machine"));

        let name_column = ["name"];
        let name_only_html = render_crud_list(
            &route,
            CrudListView {
                query_columns: &columns,
                display_columns: &name_column,
                filter_columns: &[],
                sort_columns: &columns,
                rows: &rows,
                search: "",
                query_values: &query_values,
                sort: "id",
                order: "ASC",
                page: 1,
                per_page: 50,
                archived: false,
                success: None,
                success_title: None,
            },
        );
        assert!(name_only_html.contains("<td><a href=\"/machines/1\">&lt;unsafe&gt;</a></td>"));

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
                archived: false,
                success: None,
                success_title: None,
            },
            CrudUiActions {
                create: false,
                edit: false,
                delete: false,
                restore: false,
                custom: Vec::new(),
            },
            UiLanguage::English,
        );
        assert!(!restricted_html.contains("href=\"/machines/new\""));

        let mut cards_route = route.clone();
        cards_route.list_view.mode = CrudListViewMode::Cards;
        cards_route.list_view.empty = Some("Nothing <yet>.".into());
        cards_route.loading_view.message = Some("Loading machines...".into());
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
                archived: false,
                success: None,
                success_title: None,
            },
        );
        assert!(cards_html.contains("zelyra-crud-cards"));
        assert!(cards_html.contains("zelyra-crud-card"));
        assert!(cards_html.contains("data-loading-message=\"Loading machines...\""));
        assert!(!cards_html.contains("<table>"));

        let cards_name_only_html = render_crud_list(
            &cards_route,
            CrudListView {
                query_columns: &columns,
                display_columns: &name_column,
                filter_columns: &[],
                sort_columns: &columns,
                rows: &rows,
                search: "",
                query_values: &query_values,
                sort: "id",
                order: "ASC",
                page: 1,
                per_page: 50,
                archived: false,
                success: None,
                success_title: None,
            },
        );
        assert!(
            cards_name_only_html.contains("<dd><a href=\"/machines/1\">&lt;unsafe&gt;</a></dd>")
        );

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
                archived: false,
                success: None,
                success_title: None,
            },
        );
        assert!(empty_html.contains("Nothing &lt;yet&gt;."));

        let mut error_route = route.clone();
        error_route.error_view.title = Some("Customer error".into());
        error_route.error_view.message = Some("Try <again>.".into());
        error_route.loading_view.message = Some("Loading customers".into());
        let error_response =
            crud_error_response(&error_route, 500, "Internal Server Error", "Fallback");
        assert_eq!(error_response.status, 500);
        assert!(error_response.body.contains(">Customer error</h1>"));
        assert!(error_response.body.contains("Try &lt;again&gt;."));
        assert!(error_response
            .body
            .contains("data-loading-message=\"Loading customers\""));
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
                        name: None,
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
            loading_view: CrudLoadingViewDef::default(),
            error_view: CrudErrorViewDef::default(),
            layout_html: None,
            soft_delete: None,
            actions: Vec::new(),
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
                archived: false,
                success: None,
                success_title: None,
            },
        );
        let html = localize_html(&html, UiLanguage::English);
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
            loading_view: CrudLoadingViewDef::default(),
            error_view: CrudErrorViewDef::default(),
            layout_html: None,
            soft_delete: None,
            actions: Vec::new(),
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
            loading_view: CrudLoadingViewDef::default(),
            error_view: CrudErrorViewDef::default(),
            layout_html: None,
            soft_delete: None,
            actions: Vec::new(),
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
    fn database_capability_denies_page_data_before_database_access() {
        let route = Route {
            path: "/customers/{name}".into(),
            html: "<h1>{customer.name}</h1>".into(),
            query: Vec::new(),
            page_size: None,
            sort_columns: Vec::new(),
            search_columns: Vec::new(),
            filters: Vec::new(),
            data: vec![RouteData {
                name: "customer".into(),
                query: "SELECT name FROM customers WHERE name = :name".into(),
                fields: vec!["name".into()],
                collection: false,
                optional: false,
            }],
            requires_auth: false,
            permissions: Vec::new(),
        };
        let app = WebApp::new(vec![route], Vec::new()).with_database_capability(false);
        let request = parse_request("GET /customers/Ada HTTP/1.1\r\n\r\n").unwrap();
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
            audit_chain: false,
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
            query: Vec::new(),
            page_size: None,
            sort_columns: Vec::new(),
            search_columns: Vec::new(),
            filters: Vec::new(),
            data: Vec::new(),
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
                query: Vec::new(),
                page_size: None,
                sort_columns: Vec::new(),
                search_columns: Vec::new(),
                filters: Vec::new(),
                data: Vec::new(),
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
            loading_view: CrudLoadingViewDef::default(),
            error_view: CrudErrorViewDef::default(),
            layout_html: None,
            soft_delete: None,
            actions: Vec::new(),
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
    fn mutating_api_rejects_cross_origin_browser_requests_unless_cors_allows_them() {
        let route = ApiRoute::new("POST", "/mutate", |_request, _| {
            Response::json(200, "{\"ok\":true}")
        });
        let app = WebApp::new(Vec::new(), Vec::new())
            .with_apis(vec![route.clone()])
            .with_allowed_hosts(vec!["example.test".into()])
            .unwrap();
        let cross_origin = parse_request(
            "POST /mutate HTTP/1.1\r\nHost: example.test\r\nOrigin: https://attacker.test\r\n\r\n",
        )
        .unwrap();
        assert_eq!(app.dispatch(&cross_origin).status, 403);

        let same_origin = parse_request(
            "POST /mutate HTTP/1.1\r\nHost: example.test\r\nOrigin: http://example.test\r\n\r\n",
        )
        .unwrap();
        assert_eq!(app.dispatch(&same_origin).status, 200);

        let policy = CorsPolicy::new(vec!["https://attacker.test".into()], true).unwrap();
        let explicitly_allowed = WebApp::new(Vec::new(), Vec::new())
            .with_apis(vec![route])
            .with_cors(policy)
            .with_allowed_hosts(vec!["example.test".into()])
            .unwrap();
        assert_eq!(explicitly_allowed.dispatch(&cross_origin).status, 200);

        let cookie_only = parse_request(
            "POST /mutate HTTP/1.1\r\nHost: example.test\r\nCookie: zelyra_session=session-token\r\n\r\n",
        )
        .unwrap();
        assert_eq!(app.dispatch(&cookie_only).status, 403);

        let get_route = ApiRoute::new("GET", "/possibly-stateful", |_request, _| {
            Response::json(200, "{\"ok\":true}")
        });
        let app = WebApp::new(Vec::new(), Vec::new())
            .with_apis(vec![get_route])
            .with_allowed_hosts(vec!["example.test".into()])
            .unwrap();
        let cross_origin_get = parse_request(
            "GET /possibly-stateful HTTP/1.1\r\nHost: example.test\r\nOrigin: https://attacker.test\r\nCookie: zelyra_session=session-token\r\n\r\n",
        )
        .unwrap();
        assert_eq!(app.dispatch(&cross_origin_get).status, 403);

        let same_origin_get = parse_request(
            "GET /possibly-stateful HTTP/1.1\r\nHost: example.test\r\nReferer: http://example.test/customers\r\nCookie: zelyra_session=session-token\r\n\r\n",
        )
        .unwrap();
        assert_eq!(app.dispatch(&same_origin_get).status, 200);
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
            loading_view: CrudLoadingViewDef::default(),
            error_view: CrudErrorViewDef::default(),
            layout_html: None,
            soft_delete: None,
            actions: Vec::new(),
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
            loading_view: CrudLoadingViewDef::default(),
            error_view: CrudErrorViewDef::default(),
            layout_html: None,
            soft_delete: None,
            actions: Vec::new(),
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
        let html = localize_html(
            &render_crud_detail(&route, &["id", "name"], &["1".into(), "CNC".into()], "1"),
            UiLanguage::English,
        );
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
                restore: false,
                custom: Vec::new(),
            },
            UiLanguage::English,
        );
        assert!(!restricted_html.contains("/machines/1/edit"));
        assert!(!restricted_html.contains("/machines/new"));
        assert!(!restricted_html.contains(">Delete</button>"));

        let action_html = render_crud_detail_with_actions(
            &route,
            &["id", "name"],
            &["1".into(), "CNC".into()],
            "1",
            CrudUiActions {
                create: false,
                edit: false,
                delete: false,
                restore: false,
                custom: vec![CrudUiActionLink {
                    label: "Deactivate".into(),
                    icon: Some("pause".into()),
                    path: "/machines/{id}/deactivate".into(),
                    csrf: "crud-csrf".into(),
                    confirm: Some("Deactivate <unsafe> customer?".into()),
                    confirm_page: None,
                    fields: vec![
                        CrudUiActionField {
                            name: "active".into(),
                            label: "Active".into(),
                            input_type: "checkbox".into(),
                            required: true,
                            max: None,
                            relation: false,
                            options: Vec::new(),
                        },
                        CrudUiActionField {
                            name: "department".into(),
                            label: "Department".into(),
                            input_type: "text".into(),
                            required: true,
                            max: None,
                            relation: true,
                            options: vec![SelectOption {
                                value: "2".into(),
                                label: "Production <unsafe>".into(),
                            }],
                        },
                    ],
                }],
            },
            UiLanguage::English,
        );
        assert!(action_html.contains("action=\"/machines/1/deactivate\""));
        assert!(action_html.contains(">Deactivate</button>"));
        assert!(action_html.contains("name=\"_zelyra_csrf\" value=\"crud-csrf\""));
        assert!(action_html.contains("name=\"active\" type=\"checkbox\" value=\"true\""));
        assert!(action_html
            .contains("class=\"zelyra-action-icon zelyra-action-icon-pause\" data-icon=\"pause\""));
        assert!(action_html.contains("<select id=\"department\" name=\"department\" required>"));
        assert!(action_html.contains("<option value=\"2\">Production &lt;unsafe&gt;</option>"));
        assert!(action_html.contains(
            "data-confirm=\"Deactivate &lt;unsafe&gt; customer?\" onsubmit=\"return confirm(this.dataset.confirm)\""
        ));

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
    fn renders_action_error_without_database_details() {
        let action = zelyra_ast::FormAction {
            name: "save".into(),
            label: None,
            icon: None,
            confirm: None,
            confirm_page: None,
            success_page: None,
            error_page: Some(zelyra_ast::CrudActionNoticeDef {
                title: Some("Action failed <unsafe>".into()),
                message: Some("Please try again <later>.".into()),
            }),
            fields: Vec::new(),
            requires_auth: false,
            permissions: Vec::new(),
            statements: Vec::new(),
            success: None,
            redirect: None,
            span: zelyra_ast::Span::default(),
        };
        let response = action_error_response(
            &action,
            500,
            "Internal Server Error",
            "database password must not leak",
        );
        assert_eq!(response.status, 500);
        assert!(response.body.contains("Action failed &lt;unsafe&gt;"));
        assert!(response.body.contains("Please try again &lt;later&gt;."));
        assert!(!response.body.contains("database password"));
    }

    #[test]
    fn form_post_requires_csrf_and_reports_validation_errors() {
        let app = WebApp::new(Vec::new(), vec![form_route()]);
        let invalid_csrf = parse_request(
            "POST /forms/CustomerCreate HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\nContent-Length: 18\r\n\r\n_zelyra_csrf=wrong",
        )
        .unwrap();
        assert_eq!(app.dispatch(&invalid_csrf).status, 403);

        let missing_name = parse_request(
            "POST /forms/CustomerCreate HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\n\r\n_zelyra_csrf=csrf-token",
        )
        .unwrap();
        let response = app.dispatch(&missing_name);
        assert_eq!(response.status, 422);
        assert!(response.body.contains("value is required"));

        let cross_origin = parse_request(
            "POST /forms/CustomerCreate HTTP/1.1\r\nHost: localhost\r\nOrigin: https://attacker.test\r\n\r\n_zelyra_csrf=csrf-token&name=Anna",
        )
        .unwrap();
        assert_eq!(app.dispatch(&cross_origin).status, 403);
    }

    #[test]
    fn custom_form_layout_cannot_bypass_csrf_validation_or_authorization() {
        let mut route = form_route();
        route.layout_html = Some(format!(
            "<div class=\"custom-form-shell\">{CRUD_LAYOUT_CONTENT_MARKER}</div>"
        ));
        let app = WebApp::new(Vec::new(), vec![route.clone()]);

        let invalid_csrf = parse_request(
            "POST /forms/CustomerCreate HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\n\r\n_zelyra_csrf=wrong",
        )
        .unwrap();
        let csrf_response = app.dispatch(&invalid_csrf);
        assert_eq!(csrf_response.status, 403);
        assert!(csrf_response.body.contains("custom-form-shell"));

        let missing_name = parse_request(
            "POST /forms/CustomerCreate HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\n\r\n_zelyra_csrf=csrf-token",
        )
        .unwrap();
        let validation_response = app.dispatch(&missing_name);
        assert_eq!(validation_response.status, 422);
        assert!(validation_response.body.contains("custom-form-shell"));
        assert!(validation_response.body.contains("value is required"));

        let mut protected_route = route;
        protected_route.requires_auth = true;
        protected_route.permissions = vec!["customers.view".into()];
        let protected_app = WebApp::new(Vec::new(), vec![protected_route]);
        let get_request =
            parse_request("GET /forms/CustomerCreate HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let authorization_response = protected_app.dispatch(&get_request);
        assert_eq!(authorization_response.status, 401);
        assert!(!authorization_response.body.contains("custom-form-shell"));
    }

    #[test]
    fn form_post_returns_accepted_after_validating_input() {
        let app = WebApp::new(Vec::new(), vec![form_route()]);
        let request = parse_request(
            "POST /forms/CustomerCreate HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\n\r\n_zelyra_csrf=csrf-token&name=Anna",
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
            label: None,
            icon: None,
            confirm: None,
            confirm_page: None,
            success_page: None,
            error_page: None,
            fields: Vec::new(),
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
            label: None,
            icon: None,
            confirm: None,
            confirm_page: None,
            success_page: None,
            error_page: None,
            fields: Vec::new(),
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
            label: None,
            icon: None,
            confirm: None,
            confirm_page: None,
            success_page: None,
            error_page: None,
            fields: Vec::new(),
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
            "POST /forms/CustomerCreate HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\n\r\n_zelyra_csrf=csrf-token&name=Anna",
        )
        .unwrap();
        assert_eq!(app.dispatch(&request).status, 503);
    }
}
