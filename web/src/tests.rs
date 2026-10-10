use super::*;
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};

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
fn built_in_liveness_endpoint_is_database_free_and_method_bounded() {
    let app = WebApp::new(Vec::new(), Vec::new());
    let request = |method: &str| Request {
        method: method.into(),
        target: HEALTH_LIVENESS_PATH.into(),
        path: HEALTH_LIVENESS_PATH.into(),
        headers: HashMap::new(),
        body: String::new(),
        remote_addr: None,
    };

    let get = app.dispatch(&request("GET"));
    assert_eq!(get.status, 200);
    assert_eq!(get.content_type, "application/json; charset=utf-8");
    assert_eq!(get.body, r#"{"status":"ok"}"#);
    assert!(get
        .headers
        .iter()
        .any(|(name, value)| name == "Cache-Control" && value == "no-store"));

    let head = app.dispatch(&request("HEAD"));
    assert_eq!(head.status, 200);
    assert!(head.body.is_empty());

    let post = app.dispatch(&request("POST"));
    assert_eq!(post.status, 405);
    assert!(post
        .headers
        .iter()
        .any(|(name, value)| name == "Allow" && value == "GET, HEAD"));
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
        tenant_column: None,
        tenant_membership_table: None,
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

#[test]
fn edit_form_snapshots_are_signed_and_bound_to_csrf_key_and_field_set() {
    let csrf = CsrfProtection::new("session-specific-csrf-secret");
    let form = form_route().form;
    let values = HashMap::from([("name".to_owned(), "original".to_owned())]);
    let token = sign_form_snapshot(&csrf, &values).unwrap();
    let signature = verify_form_snapshot(&token).unwrap();

    assert!(form_snapshot_matches(&csrf, &signature, &values));
    assert!(!form_snapshot_matches(
        &CsrfProtection::new("other-session"),
        &signature,
        &values
    ));
    assert!(!form_snapshot_matches(
        &csrf,
        &signature,
        &HashMap::from([("name".to_owned(), "modified".to_owned())])
    ));
    assert!(verify_form_snapshot(&format!("{token}00")).is_none());

    let mut mismatched_form = form.clone();
    mismatched_form.fields[0].name = "email".into();
    let mismatched_values = HashMap::from([("email".to_owned(), "original".to_owned())]);
    assert!(!form_snapshot_matches(
        &csrf,
        &signature,
        &mismatched_values
    ));
    assert!(attach_form_snapshot("<form></form>", &token).contains("name=\"_zelyra_snapshot\""));
}

fn default_shell_crud(layout_html: Option<String>) -> CrudRoute {
    CrudRoute {
        path: "/machines".into(),
        title: "Maschinen".into(),
        table: "machines".into(),
        tenant_column: None,
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
    let html = render_default_application_shell("<h1>Machines</h1>", &context, UiLanguage::English);

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
        membership_table: None,
        session_table: None,
        permissions_table: None,
        roles_table: None,
        role_permissions_table: None,
        audit_table: None,
        audit_chain: false,
        admin_path: Some("/admin".into()),
        admin_permission: None,
        admin_role: None,
        login_rate_limit: DEFAULT_LOGIN_RATE_LIMIT,
        login_block_seconds: DEFAULT_LOGIN_BLOCK_SECONDS,
        reset_tokens_table: None,
        reset_rate_limit: DEFAULT_RESET_RATE_LIMIT,
        reset_block_seconds: DEFAULT_RESET_BLOCK_SECONDS,
        schema: Schema {
            database: None,
            tables: Vec::new(),
        },
        csrf: CsrfProtection::new("login-csrf"),
    };
    let app = WebApp::new(Vec::new(), Vec::new())
        .with_auth_route(auth.clone())
        .with_ui_settings(UiLanguage::German, UiLevel::Work);
    let request = parse_request("GET /login HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let response = app.dispatch(&request);

    assert!(response.body.contains("class=\"zelyra-app\""));
    assert!(response.body.contains("lang=\"de\""));
    assert!(response.body.contains("<title>Anmelden | Zelyra</title>"));
    assert!(response.body.contains("Zum Inhalt springen"));
    assert!(response.body.contains("id=\"zelyra-content\""));
    assert!(response.body.contains("href=\"/\""));
    assert!(!response.body.contains("/forgot-password"));

    let mut reset_auth = auth;
    reset_auth.reset_tokens_table = Some("password_resets".into());
    let reset_app = WebApp::new(Vec::new(), Vec::new())
        .with_auth_route(reset_auth)
        .with_ui_settings(UiLanguage::German, UiLevel::Work);
    let reset_response = reset_app.dispatch(&request);
    assert!(reset_response.body.contains("href=\"/forgot-password\""));
    assert!(reset_response.body.contains("Passwort vergessen?"));
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
    let app = WebApp::new(vec![route], Vec::new()).with_project_theme_css(Some(theme_css.into()));
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
        access: Default::default(),
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
        access: Default::default(),
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

    let english =
        WebApp::new(vec![route], Vec::new()).with_ui_settings(UiLanguage::English, UiLevel::Work);
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
            &localized_validation_message(UiLanguage::German, "value exceeds maximum length of 80",),
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
        |_request, parameters| Response::json(200, format!("{{\"id\":\"{}\"}}", parameters["id"])),
    )]);
    let request = parse_request("GET /customers/42 HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
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
fn request_correlation_id_is_validated_propagated_and_returned_once() {
    let app = WebApp::new(Vec::new(), Vec::new()).with_apis(vec![ApiRoute::new(
        "GET",
        "/correlated",
        |request, _| {
            Response::json(
                200,
                request
                    .headers
                    .get("x-request-id")
                    .cloned()
                    .unwrap_or_default(),
            )
            .with_header("x-request-id", "handler-controlled")
        },
    )]);

    let valid = parse_request(
        "GET /correlated HTTP/1.1\r\nHost: localhost\r\nX-Request-ID: trace_42.a\r\n\r\n",
    )
    .unwrap();
    let valid_response = app.dispatch(&valid);
    assert_eq!(valid_response.body, "trace_42.a");
    assert_eq!(
        valid_response
            .headers
            .iter()
            .filter(|(name, _)| name.eq_ignore_ascii_case("x-request-id"))
            .map(|(_, value)| value.as_str())
            .collect::<Vec<_>>(),
        ["trace_42.a"]
    );

    let invalid = parse_request(
        "GET /correlated HTTP/1.1\r\nHost: localhost\r\nX-Request-ID: injected%0d%0aheader\r\n\r\n",
    )
    .unwrap();
    let invalid_response = app.dispatch(&invalid);
    assert!(valid_request_id(&invalid_response.body));
    assert_ne!(invalid_response.body, "injected%0d%0aheader");
    assert_eq!(
        invalid_response
            .headers
            .iter()
            .filter(|(name, _)| name.eq_ignore_ascii_case("x-request-id"))
            .count(),
        1
    );

    let missing = parse_request("GET /correlated HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let missing_response = app.dispatch(&missing);
    assert!(valid_request_id(&missing_response.body));
    assert_ne!(missing_response.body, "");
    assert_eq!(
        missing_response
            .headers
            .iter()
            .filter(|(name, _)| name.eq_ignore_ascii_case("x-request-id"))
            .map(|(_, value)| value.as_str())
            .collect::<Vec<_>>(),
        [missing_response.body.as_str()]
    );
}

#[test]
fn api_metadata_and_per_client_rate_limits_are_applied() {
    let app = WebApp::new(Vec::new(), Vec::new()).with_apis(vec![ApiRoute::new(
        "GET",
        "/limited",
        |_request, _| Response::json(200, "{}"),
    )
    .with_metadata(
        Some("v1".into()),
        true,
        Some(zelyra_ast::ApiRateLimit {
            requests: 2,
            window_seconds: 60,
        }),
    )]);
    let mut request = parse_request("GET /limited HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    request.remote_addr = Some(SocketAddr::from(([127, 0, 0, 1], 3001)));

    let first = app.dispatch(&request);
    let second = app.dispatch(&request);
    let limited = app.dispatch(&request);
    assert_eq!(first.status, 200);
    assert_eq!(second.status, 200);
    assert_eq!(limited.status, 429);
    assert!(limited.headers.iter().any(|(name, value)| {
        name == "Retry-After" && value.parse::<u64>().is_ok_and(|seconds| seconds > 0)
    }));
    for response in [first, second, limited] {
        assert!(response
            .headers
            .iter()
            .any(|(name, value)| name == "X-Zelyra-API-Version" && value == "v1"));
        assert!(response
            .headers
            .iter()
            .any(|(name, value)| name == "X-Zelyra-API-Deprecated" && value == "true"));
    }

    request.remote_addr = Some(SocketAddr::from(([127, 0, 0, 2], 3001)));
    assert_eq!(app.dispatch(&request).status, 200);
}

#[test]
fn api_rate_limiter_caps_client_state_and_fails_closed_for_new_clients() {
    let app = WebApp::new(Vec::new(), Vec::new()).with_apis(vec![ApiRoute::new(
        "GET",
        "/limited",
        |_request, _| Response::json(200, "{}"),
    )
    .with_metadata(
        None,
        false,
        Some(zelyra_ast::ApiRateLimit {
            requests: 5,
            window_seconds: 60,
        }),
    )]);
    let now = Instant::now();
    {
        let mut clients = app.api_throttle.lock().unwrap();
        for index in 0..API_THROTTLE_MAX_CLIENTS {
            clients.insert(
                format!("seed-{index}"),
                ApiThrottle {
                    window_started: now,
                    requests: 1,
                    window_seconds: 60,
                },
            );
        }
    }

    let mut request = parse_request("GET /limited HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    request.remote_addr = Some(SocketAddr::from(([127, 0, 0, 9], 3001)));
    assert_eq!(app.dispatch(&request).status, 429);
    assert_eq!(
        app.api_throttle.lock().unwrap().len(),
        API_THROTTLE_MAX_CLIENTS
    );
}

#[test]
fn api_rate_limiter_discards_expired_client_windows() {
    let app = WebApp::new(Vec::new(), Vec::new()).with_apis(vec![ApiRoute::new(
        "GET",
        "/limited",
        |_request, _| Response::json(200, "{}"),
    )
    .with_metadata(
        None,
        false,
        Some(zelyra_ast::ApiRateLimit {
            requests: 5,
            window_seconds: 1,
        }),
    )]);
    app.api_throttle.lock().unwrap().insert(
        "expired".into(),
        ApiThrottle {
            window_started: Instant::now() - Duration::from_secs(2),
            requests: 5,
            window_seconds: 1,
        },
    );
    let mut request = parse_request("GET /limited HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    request.remote_addr = Some(SocketAddr::from(([127, 0, 0, 10], 3001)));
    assert_eq!(app.dispatch(&request).status, 200);
    assert_eq!(app.api_throttle.lock().unwrap().len(), 1);
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
    assert!(page_search_state(&route, &HashMap::from([("search".into(), "Ada".into())])).is_err());
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
fn incomplete_http_request_is_closed_at_absolute_deadline() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let app = WebApp::new(Vec::new(), Vec::new());
        handle_connection_until(
            &mut stream,
            &app,
            Instant::now() + Duration::from_millis(100),
        )
        .unwrap();
    });
    let mut client = TcpStream::connect(address).unwrap();
    client
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n")
        .unwrap();
    let started = Instant::now();
    server.join().unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
    let mut response = String::new();
    client.read_to_string(&mut response).unwrap();
    assert!(response.is_empty());
}

#[test]
fn built_in_liveness_endpoint_is_available_over_http() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        handle_connection(&mut stream, &WebApp::new(Vec::new(), Vec::new())).unwrap();
    });
    let mut client = TcpStream::connect(address).unwrap();
    client
        .write_all(b"GET /__zelyra/health/live HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut response = String::new();
    client.read_to_string(&mut response).unwrap();
    server.join().unwrap();

    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.contains("Cache-Control: no-store\r\n"));
    assert!(response.ends_with(r#"{"status":"ok"}"#));
}

#[test]
fn a_slow_connection_does_not_block_another_http_connection() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let app = Arc::new(WebApp::new(
        vec![Route {
            path: "/ready".into(),
            html: "ready".into(),
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
    ));
    let active_connections = Arc::new(AtomicUsize::new(0));

    let mut slow_client = TcpStream::connect(address).unwrap();
    slow_client
        .write_all(b"GET /ready HTTP/1.1\r\nHost: localhost\r\n")
        .unwrap();
    let (slow_stream, _) = listener.accept().unwrap();
    let slow_worker = spawn_connection(
        slow_stream,
        Arc::clone(&app),
        Arc::clone(&active_connections),
    )
    .unwrap()
    .unwrap();

    let mut fast_client = TcpStream::connect(address).unwrap();
    fast_client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let (fast_stream, _) = listener.accept().unwrap();
    let fast_worker = spawn_connection(fast_stream, app, Arc::clone(&active_connections))
        .unwrap()
        .unwrap();
    fast_client
        .write_all(b"GET /ready HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .unwrap();
    let mut response = String::new();
    fast_client.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response.ends_with("ready"), "{response}");

    drop(slow_client);
    slow_worker.join().unwrap();
    fast_worker.join().unwrap();
    assert_eq!(active_connections.load(Ordering::Acquire), 0);
}

#[test]
fn server_drops_connections_after_reaching_its_worker_limit() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let mut client = TcpStream::connect(address).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let (stream, _) = listener.accept().unwrap();
    let active_connections = Arc::new(AtomicUsize::new(MAX_CONCURRENT_CONNECTIONS));
    let result = spawn_connection(
        stream,
        Arc::new(WebApp::new(Vec::new(), Vec::new())),
        Arc::clone(&active_connections),
    )
    .unwrap();
    assert!(result.is_none());
    assert_eq!(
        active_connections.load(Ordering::Acquire),
        MAX_CONCURRENT_CONNECTIONS
    );
    let mut response = Vec::new();
    client.read_to_end(&mut response).unwrap();
    assert!(response.is_empty());
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
    assert_eq!(key, login_throttle_key("user@example.test"));
    assert!(!key.contains("example.test"));
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
fn login_throttle_uses_configured_attempt_window_and_block_time() {
    let app = WebApp::new(Vec::new(), Vec::new());
    let key = login_throttle_key("custom@example.test");
    let policy = zelyra_ast::ApiRateLimit {
        requests: 2,
        window_seconds: 30,
    };
    record_login_failure_with_policy(&app, key.clone(), policy, 7);
    assert!(!login_is_blocked_with_policy(&app, &key, policy));
    record_login_failure_with_policy(&app, key.clone(), policy, 7);
    assert!(login_is_blocked_with_policy(&app, &key, policy));
    let state = app
        .login_throttle
        .lock()
        .unwrap()
        .get(&key)
        .unwrap()
        .clone();
    assert!(state.blocked_until.unwrap() <= Instant::now() + Duration::from_secs(7));
    assert!(state.blocked_until.unwrap() > Instant::now());
}

#[test]
fn login_throttle_bounds_keys_and_cleans_expired_windows() {
    let app = WebApp::new(Vec::new(), Vec::new());
    let policy = zelyra_ast::ApiRateLimit {
        requests: 5,
        window_seconds: 30,
    };
    for index in 0..LOGIN_THROTTLE_MAX_KEYS {
        record_login_failure_with_policy(&app, format!("user-{index}@example.test"), policy, 10);
    }
    let fresh_key = "fresh@example.test";
    assert!(login_is_blocked_with_policy(&app, fresh_key, policy));
    assert_eq!(
        app.login_throttle.lock().unwrap().len(),
        LOGIN_THROTTLE_MAX_KEYS
    );

    for state in app.login_throttle.lock().unwrap().values_mut() {
        state.window_started = Instant::now() - Duration::from_secs(31);
    }
    assert!(!login_is_blocked_with_policy(&app, fresh_key, policy));
    assert_eq!(app.login_throttle.lock().unwrap().len(), 0);
}

#[test]
fn login_block_duration_is_not_truncated_by_failure_window() {
    let app = WebApp::new(Vec::new(), Vec::new());
    let key = "long-block@example.test";
    let policy = zelyra_ast::ApiRateLimit {
        requests: 1,
        window_seconds: 30,
    };
    record_login_failure_with_policy(&app, key.into(), policy, 60);
    {
        let mut throttle = app.login_throttle.lock().unwrap();
        let state = throttle.get_mut(key).unwrap();
        state.window_started = Instant::now() - Duration::from_secs(31);
        state.blocked_until = Some(Instant::now() + Duration::from_secs(2));
    }
    assert!(login_is_blocked_with_policy(&app, key, policy));

    app.login_throttle
        .lock()
        .unwrap()
        .get_mut(key)
        .unwrap()
        .blocked_until = Some(Instant::now() - Duration::from_secs(1));
    assert!(!login_is_blocked_with_policy(&app, key, policy));
    assert!(app.login_throttle.lock().unwrap().is_empty());
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
            expires_at: Instant::now() + SESSION_LIFETIME,
            device: None,
        },
    );
    let auth = AuthRoute {
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
fn expired_memory_session_is_rejected_and_removed() {
    let app = WebApp::new(Vec::new(), Vec::new());
    for (token, expires_at) in [
        ("expired", Instant::now() - Duration::from_secs(1)),
        ("valid", Instant::now() + SESSION_LIFETIME),
    ] {
        app.sessions.lock().unwrap().insert(
            token.into(),
            Session {
                user_id: Some(7),
                permissions: vec!["auth.manage".into()],
                expires_at,
                device: None,
            },
        );
    }
    assert!(memory_session(&app, "expired").is_none());
    assert!(!app.sessions.lock().unwrap().contains_key("expired"));
    assert_eq!(memory_session(&app, "valid").unwrap().user_id, Some(7));
}

#[test]
fn session_device_metadata_is_bounded_and_escaped_in_self_service_view() {
    let request =
        parse_request("GET / HTTP/1.1\r\nUser-Agent: Browser <script>\u{0001}\r\n\r\n").unwrap();
    assert_eq!(
        request_device_metadata(&request).as_deref(),
        Some("Browser <script>")
    );
    let no_user_agent = parse_request("GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    assert_eq!(request_device_metadata(&no_user_agent), None);

    let long_request = parse_request(&format!(
        "GET / HTTP/1.1\r\nUser-Agent: {}\r\n\r\n",
        "a".repeat(300)
    ))
    .unwrap();
    assert_eq!(request_device_metadata(&long_request).unwrap().len(), 255);

    let rows = [AccountSessionRow {
        key: "id:7".into(),
        number: Some("7".into()),
        expires: "tomorrow".into(),
        expires_relative: false,
        current: true,
        device: Some("<script>".into()),
    }];
    let html = localize_html(
        &render_account_sessions(&account_sessions_test_auth(), &rows, UiLanguage::English),
        UiLanguage::English,
    );
    assert!(html.contains("&lt;script&gt;"));
    assert!(!html.contains("<script>"));
    assert!(html.contains("Browser or device"));
}

#[test]
fn session_administration_preserves_legacy_tables_and_escapes_rows() {
    let mut auth = AuthRoute {
        table: "users".into(),
        membership_table: None,
        session_table: Some("sessions".into()),
        permissions_table: None,
        roles_table: None,
        role_permissions_table: None,
        audit_table: None,
        audit_chain: false,
        admin_path: Some("/admin/access".into()),
        admin_permission: Some("auth.manage".into()),
        admin_role: None,
        login_rate_limit: DEFAULT_LOGIN_RATE_LIMIT,
        login_block_seconds: DEFAULT_LOGIN_BLOCK_SECONDS,
        reset_tokens_table: None,
        reset_rate_limit: DEFAULT_RESET_RATE_LIMIT,
        reset_block_seconds: DEFAULT_RESET_BLOCK_SECONDS,
        schema: Schema {
            database: None,
            tables: vec![zelyra_database::Table {
                name: "sessions".into(),
                columns: vec![],
                foreign_keys: vec![],
                indexes: vec![],
                uniques: vec![],
            }],
        },
        csrf: CsrfProtection::new("csrf-token"),
    };
    assert!(administrable_session_table(&auth).is_none());
    assert!(
        !render_auth_admin(&auth, &[], &[], &[], &[], &[], UiLanguage::English)
            .contains("revoke_session")
    );
    auth.schema.tables[0].columns.push(zelyra_database::Column {
        name: "id".into(),
        sql_type: "BIGINT".into(),
        nullable: false,
        primary_key: true,
        auto: true,
        unique: false,
        default: None,
    });
    assert_eq!(administrable_session_table(&auth), Some("sessions"));
    let rows = vec![vec![
        "1".into(),
        "7".into(),
        "<script>@example.test".into(),
        "2026-10-05".into(),
    ]];
    let html = render_auth_sessions(&auth, &rows, UiLanguage::English);
    assert!(html.contains("&lt;script&gt;@example.test"));
    assert!(!html.contains("<script>"));
    assert!(html.contains("name=\"session_id\" value=\"1\""));
    assert!(localize_html(
        &render_auth_sessions(&auth, &rows, UiLanguage::German),
        UiLanguage::German
    )
    .contains("Sitzung sperren"));
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
fn resolves_module_qualified_relationship_types_to_schema_tables() {
    let mut route = relation_form_route();
    route.table.as_mut().unwrap().columns[0].ty = Type::Named("crm::Department".into());
    let field = &route.form.fields[1];

    assert_eq!(
        relation_target_for_field(&route, field).as_deref(),
        Some("departments")
    );
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
    assert!(html
        .contains("<fieldset class=\"zelyra-query-controls\"><legend>Search and filters</legend>"));
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
        tenant_column: None,
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
    assert!(html
        .contains("<fieldset class=\"zelyra-query-controls\"><legend>Search and filters</legend>"));
    assert!(html.contains("for=\"filter_name__operator\">Filter Name operator</label>"));
    assert!(html.contains("for=\"filter_name\">Filter Name value</label>"));
    assert!(html.contains(
        "class=\"zelyra-success\" role=\"status\"><h2>Completed</h2><p>Saved &lt;unsafe&gt;</p>"
    ));
    assert!(html.contains("value=\"CNC machine\""));
    assert!(
        html.contains("page=1&amp;per_page=1&amp;sort=id&amp;order=asc&amp;search=CNC%20machine")
    );
    assert!(
        html.contains("page=3&amp;per_page=1&amp;sort=id&amp;order=asc&amp;search=CNC%20machine")
    );

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
    assert!(cards_name_only_html.contains("<dd><a href=\"/machines/1\">&lt;unsafe&gt;</a></dd>"));

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
        tenant_column: None,
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
        tenant_column: None,
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
        tenant_column: None,
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
fn tenant_scoped_crud_fails_closed_without_a_database_session_identity() {
    let mut crud = default_shell_crud(None);
    crud.tenant_column = Some("tenant_id".into());
    let app = WebApp::new(Vec::new(), Vec::new()).with_cruds(vec![crud]);
    let request = parse_request("GET /machines HTTP/1.1\r\nX-Zelyra-Tenant: 41\r\n\r\n").unwrap();
    let response = app.dispatch(&request);
    assert_eq!(response.status, 403);
}

#[test]
fn static_bearer_does_not_supply_a_tenant_principal() {
    let mut crud = default_shell_crud(None);
    crud.tenant_column = Some("tenant_id".into());
    let app = WebApp::new(Vec::new(), Vec::new())
        .with_auth(Some("shared-token".into()), vec!["invoices.view".into()])
        .with_cruds(vec![crud]);
    let request =
        parse_request("GET /machines HTTP/1.1\r\nAuthorization: Bearer shared-token\r\n\r\n")
            .unwrap();
    let response = app.dispatch(&request);
    assert_eq!(response.status, 403);
}

#[test]
fn tenant_selector_accepts_header_or_url_and_rejects_conflicts() {
    let query = parse_request("GET /invoices?tenant_id=41 HTTP/1.1\r\n\r\n").unwrap();
    assert_eq!(tenant_selector(&query).unwrap().as_deref(), Some("41"));

    let header = parse_request("GET /invoices HTTP/1.1\r\nX-Zelyra-Tenant: 41\r\n\r\n").unwrap();
    assert_eq!(tenant_selector(&header).unwrap().as_deref(), Some("41"));

    let conflict =
        parse_request("GET /invoices?tenant_id=41 HTTP/1.1\r\nX-Zelyra-Tenant: 42\r\n\r\n")
            .unwrap();
    assert_eq!(tenant_selector(&conflict).unwrap_err(), 400);

    let duplicate =
        parse_request("GET /invoices?tenant_id=41&tenant_id=42 HTTP/1.1\r\n\r\n").unwrap();
    assert_eq!(tenant_selector(&duplicate).unwrap_err(), 400);

    let encoded_duplicate =
        parse_request("GET /invoices?tenant_id=41&tenant%5Fid=42 HTTP/1.1\r\n\r\n").unwrap();
    assert_eq!(tenant_selector(&encoded_duplicate).unwrap_err(), 400);
}

#[test]
fn tenant_urls_are_preserved_in_generated_links_forms_and_redirects() {
    assert_eq!(
        append_tenant_selector("/invoices", 41),
        "/invoices?tenant_id=41"
    );
    assert_eq!(
        append_tenant_selector("/invoices?page=2", 41),
        "/invoices?page=2&tenant_id=41"
    );
    assert_eq!(
        append_tenant_selector("/invoices?tenant_id=41", 41),
        "/invoices?tenant_id=41"
    );
    assert_eq!(
        append_tenant_selector("https://example.test/x", 41),
        "https://example.test/x"
    );

    let html = preserve_tenant_selector_in_html(
        "<a href=\"/invoices/3/edit\">Edit</a><form action=\"/invoices/new\"></form>",
        41,
    );
    assert!(html.contains("href=\"/invoices/3/edit?tenant_id=41\""));
    assert!(html.contains("action=\"/invoices/new?tenant_id=41\""));
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
        membership_table: None,
        session_table: Some("sessions".into()),
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
        parse_request("GET /admin HTTP/1.1\r\nAuthorization: Bearer test-token\r\n\r\n").unwrap();
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
        tenant_column: None,
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
        tenant_column: None,
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
        tenant_column: None,
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
fn edit_form_rejects_posts_without_a_signed_record_snapshot() {
    let mut route = form_route();
    route.form.name = "CustomerEdit".into();
    let app = WebApp::new(Vec::new(), vec![route]);
    let request = parse_request(
        "POST /forms/CustomerCreate HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\n\r\n_zelyra_csrf=csrf-token&name=Anna",
    )
    .unwrap();

    let response = app.dispatch(&request);
    assert_eq!(response.status, 409);
    assert!(response.body.contains("Reload the form"));
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
fn custom_crud_layout_preserves_escaping_for_loaded_record_values() {
    let route = default_shell_crud(Some(format!(
        "<div class=\"custom-record-shell\">{CRUD_LAYOUT_CONTENT_MARKER}</div>"
    )));
    let content = render_crud_detail(
        &route,
        &["id", "name"],
        &["7".into(), "<script>alert(1)</script>".into()],
        "7",
    );
    let response =
        apply_generated_layout(Response::html(200, content), route.layout_html.as_deref());

    assert_eq!(response.status, 200);
    assert!(response.body.contains("custom-record-shell"));
    assert!(response
        .body
        .contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(!response.body.contains("<script>alert(1)</script>"));
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
