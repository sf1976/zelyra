use super::*;
use serde_json::{json, Value};

pub const PATH: &str = "/__zelyra/module-settings";

/// A schema-backed storage adapter for the built-in application settings page.
/// Implementations must redact secret values and enforce their schema again
/// when saving; browser validation is only a usability aid.
pub trait ModuleSettingsStore: fmt::Debug + Send + Sync {
    fn read_json(&self) -> Result<Value, ModuleSettingsError>;
    fn write_json(&self, update: &Value) -> Result<(), ModuleSettingsError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleSettingsError {
    pub status: u16,
    pub message: String,
}

impl ModuleSettingsError {
    pub fn new(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

pub(super) fn dispatch(app: &WebApp, request: &Request) -> Response {
    let Some(auth) = app.auth_route.as_ref() else {
        return Response::empty(404).with_header("Cache-Control", "no-store");
    };
    let Some(permission) = auth.admin_permission.as_deref() else {
        return Response::empty(404).with_header("Cache-Control", "no-store");
    };
    let Some(store) = app.module_settings_store.as_ref() else {
        return Response::empty(404).with_header("Cache-Control", "no-store");
    };
    if let Some(response) = authorize(
        false,
        &[permission.to_owned()],
        request,
        app,
        app.database_url.as_deref(),
    ) {
        return response.with_header("Cache-Control", "no-store");
    }

    match request.method.as_str() {
        "GET" => match store.read_json() {
            Ok(snapshot) => Response::html(200, render_page(&snapshot, auth.csrf.token(), None))
                .with_header("Cache-Control", "no-store"),
            Err(error) => store_error(error),
        },
        "POST" => {
            if request.body.len() > 64 * 1024 {
                return Response::html(413, "<h1>413 Content Too Large</h1>")
                    .with_header("Cache-Control", "no-store");
            }
            let input = match parse_form_strict(&request.body) {
                Ok(input) => input,
                Err(()) => {
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
            let snapshot = match store.read_json() {
                Ok(snapshot) => snapshot,
                Err(error) => return store_error(error),
            };
            let Some((package_path, setting)) = find_setting(
                &snapshot,
                input
                    .get("package_path")
                    .map(String::as_str)
                    .unwrap_or_default(),
                input.get("name").map(String::as_str).unwrap_or_default(),
            ) else {
                return Response::html(422, "<h1>422 Invalid module setting</h1>")
                    .with_header("Cache-Control", "no-store");
            };
            let Some(revision) = input.get("revision").filter(|value| !value.is_empty()) else {
                return Response::html(400, "<h1>400 Missing settings revision</h1>")
                    .with_header("Cache-Control", "no-store");
            };
            let update = if input.get("clear").is_some_and(|value| value == "true") {
                if setting["required"].as_bool().unwrap_or(false) {
                    return render_error_page(
                        &snapshot,
                        auth.csrf.token(),
                        422,
                        tr(
                            UiLanguage::default(),
                            "module_settings.cannot_clear_required",
                        ),
                    );
                }
                json!({"packagePath":package_path,"name":setting["name"],"clear":true})
            } else if setting["secret"].as_bool().unwrap_or(false) {
                let env = input.get("value").map(String::as_str).unwrap_or_default();
                if env.is_empty() {
                    if setting["configured"].as_bool().unwrap_or(false) {
                        return Response::redirect(PATH).with_header("Cache-Control", "no-store");
                    }
                    return render_error_page(
                        &snapshot,
                        auth.csrf.token(),
                        422,
                        tr(
                            UiLanguage::default(),
                            "module_settings.enter_environment_name",
                        ),
                    );
                }
                json!({"packagePath":package_path,"name":setting["name"],"env":env})
            } else {
                let raw = input.get("value").map(String::as_str).unwrap_or_default();
                let value =
                    match parse_setting_value(setting["type"].as_str().unwrap_or_default(), raw) {
                        Ok(value) => value,
                        Err(key) => {
                            return render_error_page(
                                &snapshot,
                                auth.csrf.token(),
                                422,
                                tr(UiLanguage::default(), key),
                            )
                        }
                    };
                json!({"packagePath":package_path,"name":setting["name"],"value":value})
            };
            match store.write_json(&json!({
                "revision": revision,
                "updates": [update],
            })) {
                Ok(()) => Response::redirect(PATH).with_header("Cache-Control", "no-store"),
                Err(error) => {
                    let latest = if error.status == 409 {
                        store.read_json().unwrap_or(snapshot)
                    } else {
                        snapshot
                    };
                    render_error_page(&latest, auth.csrf.token(), error.status, error.message)
                }
            }
        }
        _ => Response::empty(405)
            .with_header("Allow", "GET, POST")
            .with_header("Cache-Control", "no-store"),
    }
}

fn parse_form_strict(body: &str) -> Result<HashMap<String, String>, ()> {
    let mut input = HashMap::new();
    if body.is_empty() {
        return Ok(input);
    }
    for pair in body.split('&') {
        let (key, value) = pair.split_once('=').ok_or(())?;
        let key = super::percent_decode(key).map_err(|_| ())?;
        let value = super::percent_decode(value).map_err(|_| ())?;
        if key.is_empty() || input.insert(key, value).is_some() {
            return Err(());
        }
    }
    Ok(input)
}

fn find_setting<'a>(
    snapshot: &'a Value,
    package_path: &str,
    name: &str,
) -> Option<(&'a str, &'a Value)> {
    snapshot
        .get("packages")?
        .as_array()?
        .iter()
        .find(|package| package.get("path").and_then(Value::as_str) == Some(package_path))
        .and_then(|package| {
            let declared_path = package.get("path")?.as_str()?;
            let settings = package.get("settings")?.as_array()?;
            settings
                .iter()
                .find(|setting| setting.get("name").and_then(Value::as_str) == Some(name))
                .map(|setting| (declared_path, setting))
        })
}

fn parse_setting_value(type_name: &str, raw: &str) -> Result<Value, &'static str> {
    match type_name {
        "String" => Ok(json!(raw)),
        "Int" => raw
            .parse::<i64>()
            .map(|value| json!(value))
            .map_err(|_| "module_settings.enter_integer"),
        "Float" => raw
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(|value| json!(value))
            .ok_or("module_settings.enter_finite_number"),
        "Bool" => match raw {
            "true" => Ok(json!(true)),
            "false" => Ok(json!(false)),
            _ => Err("module_settings.choose_boolean"),
        },
        _ => Err("module_settings.unsupported_type"),
    }
}

fn render_page(snapshot: &Value, csrf: &str, error: Option<&str>) -> String {
    let mut html = String::from("<main class=\"z-container z-module-settings\"><header class=\"z-page-header\"><h1 data-zelyra-i18n=\"module_settings.title\">Module settings</h1><p data-zelyra-i18n=\"module_settings.description\">Values are checked against each module's declared schema. Secret values and environment-variable references are never displayed.</p></header>");
    if let Some(error) = error {
        html.push_str(&format!(
            "<div class=\"z-alert z-alert--danger\" role=\"alert\">{}</div>",
            html_escape(error)
        ));
    }
    let packages = snapshot
        .get("packages")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let revision = snapshot
        .get("revision")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if packages.is_empty() {
        html.push_str("<section class=\"z-card\"><h2 data-zelyra-i18n=\"module_settings.empty_title\">No module settings</h2><p data-zelyra-i18n=\"module_settings.empty_body\">This project has no local module settings schemas.</p></section>");
    }
    for package in packages {
        html.push_str("<section class=\"z-card z-module-settings__package\"><header class=\"z-card__header\"><h2>");
        html.push_str(&html_escape(package["name"].as_str().unwrap_or("Module")));
        html.push_str("</h2><p>");
        html.push_str(&html_escape(package["path"].as_str().unwrap_or_default()));
        html.push_str(" · version ");
        html.push_str(&html_escape(
            package["version"].as_str().unwrap_or_default(),
        ));
        html.push_str("</p></header><div class=\"z-card__body\">");
        let settings = package
            .get("settings")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();
        for (index, setting) in settings.iter().enumerate() {
            render_setting(&mut html, package, setting, index, csrf, revision);
        }
        html.push_str("</div></section>");
    }
    html.push_str("<p class=\"z-muted\" data-zelyra-i18n=\"module_settings.restart_note\">Saved values apply to new program runs. Restart a running zelyra serve process to load changed settings.</p></main>");
    html
}

fn render_setting(
    html: &mut String,
    package: &Value,
    setting: &Value,
    index: usize,
    csrf: &str,
    revision: &str,
) {
    let package_path = package["path"].as_str().unwrap_or_default();
    let name = setting["name"].as_str().unwrap_or_default();
    let type_name = setting["type"].as_str().unwrap_or_default();
    let package_id = package
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let id = format!("module-setting-{index}-{}-{name}", package_id);
    let required = setting["required"].as_bool().unwrap_or(false);
    let secret = setting["secret"].as_bool().unwrap_or(false);
    let configured = setting["configured"].as_bool().unwrap_or(false);
    let value = setting.get("value").filter(|value| !value.is_null());
    html.push_str("<form class=\"z-module-settings__field\" method=\"post\" action=\"");
    html.push_str(PATH);
    html.push_str("\"><fieldset><legend id=\"");
    html.push_str(&html_escape(&id));
    html.push_str("\">");
    html.push_str(&html_escape(name));
    html.push_str(" <span class=\"z-badge\">");
    html.push_str(&html_escape(type_name));
    html.push_str("</span></legend>");
    if let Some(description) = setting.get("description").and_then(Value::as_str) {
        if !description.is_empty() {
            html.push_str("<p class=\"z-muted\">");
            html.push_str(&html_escape(description));
            html.push_str("</p>");
        }
    }
    html.push_str("<input type=\"hidden\" name=\"_zelyra_csrf\" value=\"");
    html.push_str(&html_escape(csrf));
    html.push_str("\"><input type=\"hidden\" name=\"revision\" value=\"");
    html.push_str(&html_escape(revision));
    html.push_str("\"><input type=\"hidden\" name=\"package_path\" value=\"");
    html.push_str(&html_escape(package_path));
    html.push_str("\"><input type=\"hidden\" name=\"name\" value=\"");
    html.push_str(&html_escape(name));
    html.push_str("\"><label for=\"");
    html.push_str(&html_escape(&id));
    html.push_str("\">");
    if secret {
        html.push_str("<span data-zelyra-i18n=\"module_settings.environment_name\">Environment variable name</span>");
    } else {
        html.push_str("<span data-zelyra-i18n=\"module_settings.value\">Value</span>");
    }
    html.push_str("</label>");
    if secret {
        html.push_str(&format!("<input id=\"{}\" name=\"value\" type=\"text\" autocomplete=\"off\" maxlength=\"128\"{}>", html_escape(&id), if required && !configured { " required" } else { "" }));
        html.push_str("<p class=\"z-muted\" data-zelyra-i18n=\"module_settings.secret_help\">Enter the name of an environment variable. Its value is read by the server and is never shown here.</p>");
    } else if type_name == "Bool" {
        html.push_str(&format!("<select id=\"{}\" name=\"value\"{}><option value=\"\"{}>—</option><option value=\"true\"{}>true</option><option value=\"false\"{}>false</option></select>", html_escape(&id), if required { " required" } else { "" }, if value.and_then(Value::as_bool).is_none() { " selected" } else { "" }, if value.and_then(Value::as_bool) == Some(true) { " selected" } else { "" }, if value.and_then(Value::as_bool) == Some(false) { " selected" } else { "" }));
    } else {
        let input_type = match type_name {
            "Int" | "Float" => "number",
            _ => "text",
        };
        let step = if type_name == "Float" {
            " step=\"any\""
        } else {
            ""
        };
        let rendered_value = match value {
            Some(Value::String(value)) => value.clone(),
            Some(value) => value.to_string(),
            None => String::new(),
        };
        html.push_str(&format!(
            "<input id=\"{}\" name=\"value\" type=\"{}\" value=\"{}\"{}{}{}>",
            html_escape(&id),
            input_type,
            html_escape(&rendered_value),
            step,
            if required { " required" } else { "" },
            if type_name == "String" {
                " maxlength=\"4096\""
            } else {
                ""
            }
        ));
    }
    html.push_str("<div class=\"z-form-actions\"><button class=\"z-button z-button--primary\" type=\"submit\" data-zelyra-i18n=\"module_settings.save\">Save</button>");
    if configured && !required {
        html.push_str("<button class=\"z-button z-button--secondary\" type=\"submit\" name=\"clear\" value=\"true\" formnovalidate data-zelyra-i18n=\"module_settings.clear\">Clear override</button>");
    }
    html.push_str("</div></fieldset></form>");
}

fn render_error_page(
    snapshot: &Value,
    csrf: &str,
    status: u16,
    message: impl AsRef<str>,
) -> Response {
    Response::html(status, render_page(snapshot, csrf, Some(message.as_ref())))
        .with_header("Cache-Control", "no-store")
}

fn store_error(error: ModuleSettingsError) -> Response {
    Response::html(
        error.status,
        format!(
            "<main><h1>Module settings unavailable</h1><p>{}</p></main>",
            html_escape(&error.message)
        ),
    )
    .with_header("Cache-Control", "no-store")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug)]
    struct FakeStore {
        snapshot: Mutex<Value>,
        writes: AtomicUsize,
    }

    impl ModuleSettingsStore for FakeStore {
        fn read_json(&self) -> Result<Value, ModuleSettingsError> {
            Ok(self.snapshot.lock().unwrap().clone())
        }

        fn write_json(&self, _update: &Value) -> Result<(), ModuleSettingsError> {
            self.writes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn fixture() -> (WebApp, Arc<FakeStore>) {
        let store = Arc::new(FakeStore {
            snapshot: Mutex::new(json!({
                "revision":"revision-1",
                "packages":[{
                    "path":"packages/billing",
                    "name":"billing",
                    "version":"1.0.0",
                    "settings":[
                        {"name":"currency","type":"String","required":true,"secret":false,"description":"Invoice currency","value":"EUR","configured":true},
                        {"name":"api_token","type":"String","required":true,"secret":true,"description":"Private integration key","value":null,"configured":false,"env":"MUST_NOT_BE_RENDERED"}
                    ]
                }],
                "secretValuesIncluded":false
            })),
            writes: AtomicUsize::new(0),
        });
        let mut auth = super::super::account_sessions_test_auth();
        auth.admin_permission = Some("settings.manage".into());
        let app = WebApp::new(Vec::new(), Vec::new())
            .with_auth(Some("owner-token".into()), vec!["settings.manage".into()])
            .with_auth_route(auth)
            .with_module_settings_store(store.clone());
        (app, store)
    }

    fn request(method: &str, headers: &[(&str, &str)], body: &str) -> Request {
        let mut raw = format!("{method} {PATH} HTTP/1.1\r\nHost: 127.0.0.1:3000\r\n");
        for (name, value) in headers {
            raw.push_str(name);
            raw.push_str(": ");
            raw.push_str(value);
            raw.push_str("\r\n");
        }
        raw.push_str(&format!("Content-Length: {}\r\n\r\n{}", body.len(), body));
        parse_request(&raw).unwrap()
    }

    #[test]
    fn page_requires_auth_and_redacts_secret_reference_metadata() {
        let (app, _) = fixture();
        let anonymous = app.dispatch(&request("GET", &[], ""));
        assert_eq!(anonymous.status, 401);
        let page = app.dispatch(&request(
            "GET",
            &[("Authorization", "Bearer owner-token")],
            "",
        ));
        assert_eq!(page.status, 200);
        assert!(page.body.contains("Module settings"));
        assert!(page.body.contains("value=\"EUR\""));
        assert!(page.body.contains("Private integration key"));
        assert!(!page.body.contains("MUST_NOT_BE_RENDERED"));
        assert!(page
            .headers
            .iter()
            .any(|(name, value)| name == "Cache-Control" && value == "no-store"));
    }

    #[test]
    fn setting_control_ids_are_unique_across_packages() {
        let (app, store) = fixture();
        store.snapshot.lock().unwrap()["packages"].as_array_mut().unwrap().push(json!({
            "path":"packages/reporting",
            "name":"reporting",
            "version":"1.0.0",
            "settings":[{"name":"currency","type":"String","required":true,"secret":false,"description":"Currency","value":"EUR","configured":true}]
        }));
        let page = app.dispatch(&request(
            "GET",
            &[("Authorization", "Bearer owner-token")],
            "",
        ));
        assert_eq!(page.status, 200);
        assert!(page
            .body
            .contains("id=\"module-setting-0-packages/billing-currency\""));
        assert!(page
            .body
            .contains("id=\"module-setting-0-packages/reporting-currency\""));
        assert_eq!(page.body.matches("id=\"module-setting-").count(), 6);
    }

    #[test]
    fn post_requires_permission_csrf_origin_and_a_current_revision() {
        let (app, store) = fixture();
        let body = "_zelyra_csrf=sessions-csrf&revision=revision-1&package_path=packages%2Fbilling&name=currency&value=USD";
        let bad_csrf = request(
            "POST",
            &[
                ("Authorization", "Bearer owner-token"),
                ("Origin", "http://127.0.0.1:3000"),
            ],
            &body.replace("sessions-csrf", "wrong"),
        );
        assert_eq!(app.dispatch(&bad_csrf).status, 403);
        assert_eq!(store.writes.load(Ordering::SeqCst), 0);

        let valid = request(
            "POST",
            &[
                ("Authorization", "Bearer owner-token"),
                ("Origin", "http://127.0.0.1:3000"),
            ],
            body,
        );
        let response = app.dispatch(&valid);
        assert_eq!(response.status, 303);
        assert_eq!(response.location.as_deref(), Some(PATH));
        assert_eq!(store.writes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn page_escapes_package_and_setting_descriptions_and_does_not_grant_permissions() {
        let (mut app, store) = fixture();
        {
            let mut snapshot = store.snapshot.lock().unwrap();
            snapshot["packages"][0]["name"] = json!("<script>bad()</script>");
            snapshot["packages"][0]["settings"][0]["description"] =
                json!("\"><script>bad()</script>");
        }
        let page = app.dispatch(&request(
            "GET",
            &[("Authorization", "Bearer owner-token")],
            "",
        ));
        assert_eq!(page.status, 200);
        assert!(!page.body.contains("<script>bad()</script>"));
        assert!(page.body.contains("&lt;script&gt;bad()&lt;/script&gt;"));
        app.auth_permissions.clear();
        let forbidden = app.dispatch(&request(
            "GET",
            &[("Authorization", "Bearer owner-token")],
            "",
        ));
        assert_eq!(forbidden.status, 403);
        assert_eq!(store.writes.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn page_is_absent_without_an_admin_permission_and_localizes_to_german() {
        let (app, store) = fixture();
        let mut auth = super::super::account_sessions_test_auth();
        auth.admin_permission = None;
        let app_without_admin = WebApp::new(Vec::new(), Vec::new())
            .with_auth(Some("owner-token".into()), vec!["settings.manage".into()])
            .with_auth_route(auth)
            .with_module_settings_store(store.clone());
        let absent = app_without_admin.dispatch(&request(
            "GET",
            &[("Authorization", "Bearer owner-token")],
            "",
        ));
        assert_eq!(absent.status, 404);

        let app = app.with_ui_settings(UiLanguage::German, UiLevel::Work);
        let page = app.dispatch(&request(
            "GET",
            &[("Authorization", "Bearer owner-token")],
            "",
        ));
        assert!(page.body.contains("Moduleinstellungen"));
        assert!(!page.body.contains("No module settings"));
    }

    #[test]
    fn malformed_or_wrongly_typed_updates_never_reach_the_store() {
        let (app, store) = fixture();
        let headers = [
            ("Authorization", "Bearer owner-token"),
            ("Origin", "http://127.0.0.1:3000"),
        ];
        let wrong_type = request(
            "POST",
            &headers,
            "_zelyra_csrf=sessions-csrf&revision=revision-1&package_path=packages%2Fbilling&name=currency&value=USD&value=EUR",
        );
        assert_eq!(app.dispatch(&wrong_type).status, 400);
        assert_eq!(store.writes.load(Ordering::SeqCst), 0);

        let malformed_int = request(
            "POST",
            &headers,
            "_zelyra_csrf=sessions-csrf&revision=revision-1&package_path=packages%2Fbilling&name=currency&value=USD",
        );
        // A String field accepts arbitrary text; make the schema entry Int to
        // verify that the renderer rejects a malformed typed value.
        store.snapshot.lock().unwrap()["packages"][0]["settings"][0]["type"] = json!("Int");
        assert_eq!(app.dispatch(&malformed_int).status, 422);
        assert_eq!(store.writes.load(Ordering::SeqCst), 0);
    }
}
