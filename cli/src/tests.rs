use super::*;

fn migration_fixture_schema(columns: Vec<zelyra_database::Column>) -> Schema {
    Schema {
        database: Some(zelyra_database::DatabaseConfig {
            name: "main".into(),
            engine: "mariadb".into(),
            database: Some("zelyra_test".into()),
        }),
        tables: vec![zelyra_database::Table {
            name: "customers".into(),
            columns,
            foreign_keys: Vec::new(),
            indexes: Vec::new(),
            uniques: Vec::new(),
        }],
    }
}

fn migration_fixture_column(name: &str, sql_type: &str, nullable: bool) -> zelyra_database::Column {
    zelyra_database::Column {
        name: name.into(),
        sql_type: sql_type.into(),
        nullable,
        primary_key: name == "id",
        auto: name == "id",
        unique: false,
        default: None,
    }
}

#[test]
fn database_map_json_groups_source_tables_and_reports_unmapped_live_tables() {
    let declared = Schema {
        database: Some(zelyra_database::DatabaseConfig {
            name: "main".into(),
            engine: "mariadb".into(),
            database: Some("zelyra_test".into()),
        }),
        tables: vec![
            zelyra_database::Table {
                name: "customers".into(),
                columns: vec![migration_fixture_column("id", "BIGINT", false)],
                foreign_keys: vec![zelyra_database::ForeignKey {
                    name: None,
                    column: "group_id".into(),
                    referenced_table: "customer_groups".into(),
                    referenced_column: "id".into(),
                }],
                indexes: Vec::new(),
                uniques: Vec::new(),
            },
            zelyra_database::Table {
                name: "invoices".into(),
                columns: vec![migration_fixture_column("id", "BIGINT", false)],
                foreign_keys: Vec::new(),
                indexes: Vec::new(),
                uniques: Vec::new(),
            },
        ],
    };
    let mut live_customer = declared.tables[0].clone();
    live_customer.columns.push(migration_fixture_column(
        "display_name",
        "VARCHAR(120)",
        false,
    ));
    let live = Schema {
        database: declared.database.clone(),
        tables: vec![
            live_customer,
            zelyra_database::Table {
                name: "legacy_notes".into(),
                columns: vec![migration_fixture_column("body", "TEXT", false)],
                foreign_keys: Vec::new(),
                indexes: Vec::new(),
                uniques: Vec::new(),
            },
        ],
    };
    let owners = HashMap::from([
        ("table:customers".into(), "src/customers.zyl".into()),
        ("table:invoices".into(), "src/billing.zyl".into()),
    ]);

    let mapped = database_cli::database_map_json(&declared, &live, &owners);
    assert_eq!(mapped["read_only"], true);
    assert_eq!(mapped["ownership_enforced"], false);
    assert_eq!(mapped["modules"][0]["source_module"], "src/billing.zyl");
    assert_eq!(
        mapped["modules"][0]["tables"][0]["status"],
        "declared_missing_live"
    );
    assert_eq!(mapped["modules"][1]["source_module"], "src/customers.zyl");
    assert_eq!(mapped["modules"][1]["tables"][0]["status"], "matched");
    assert_eq!(
        mapped["modules"][1]["tables"][0]["declared_columns"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        mapped["modules"][1]["tables"][0]["live_columns"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        mapped["modules"][1]["tables"][0]["foreign_keys"][0]["references_table"],
        "customer_groups"
    );
    assert_eq!(mapped["unmapped_live_tables"][0]["name"], "legacy_notes");
    assert_eq!(
        serde_json::to_string(&mapped).unwrap(),
        serde_json::to_string(&database_cli::database_map_json(&declared, &live, &owners)).unwrap()
    );
}

#[test]
fn schema_plan_json_is_versioned_deterministic_and_emits_reviewed_reverse_plan() {
    let current = migration_fixture_schema(vec![migration_fixture_column("id", "BIGINT", false)]);
    let desired = migration_fixture_schema(vec![
        migration_fixture_column("id", "BIGINT", false),
        migration_fixture_column("name", "VARCHAR(30)", false),
    ]);
    let mut plan = diff(&desired, &current);
    plan.unique_index_preflights
        .push(zelyra_database::UniqueIndexPreflight {
            table: "customers".into(),
            columns: vec!["email".into()],
        });
    plan.foreign_key_preflights
        .push(zelyra_database::ForeignKeyPreflight {
            table: "customers".into(),
            column: "account_id".into(),
            referenced_table: "accounts".into(),
            referenced_column: "id".into(),
            referenced_table_exists: true,
        });
    let first = schema_plan_json(&desired, &current, &plan);
    let second = schema_plan_json(&desired, &current, &plan);
    assert_eq!(first, second);
    assert_eq!(first["format"], "zelyra.schema-plan/v1");
    assert_eq!(first["drift"], "present");
    assert!(first["requires_operator_approval"].as_bool().unwrap());
    assert_eq!(first["preflights"][0]["kind"], "table_must_be_empty");
    assert_eq!(first["preflights"][1]["kind"], "values_must_be_unique");
    assert_eq!(first["preflights"][1]["columns"][0], "email");
    assert_eq!(
        first["preflights"][2]["kind"],
        "foreign_key_values_must_exist"
    );
    assert_eq!(first["rollback"]["generated"], true);
    assert_eq!(
        first["rollback"]["changes"][0]["description"],
        "drop column customers.name"
    );
    assert_eq!(first["rollback"]["requires_operator_approval"], true);
    assert_eq!(first["rollback"]["requires_verified_backup"], true);
    assert_eq!(first["rollback"]["safe_to_apply_automatically"], false);
    let reverse = diff(&current, &desired);
    let reverse_json = schema_plan_json(&current, &desired, &reverse);
    assert_eq!(first["rollback"]["plan_id"], reverse_json["plan_id"]);
    assert_eq!(
        first["rollback"]["from_schema_sha256"],
        first["desired_schema_sha256"]
    );
    assert_eq!(
        first["rollback"]["to_schema_sha256"],
        first["current_schema_sha256"]
    );
    assert_eq!(first["automatic_retries"], false);
    assert!(first["plan_id"].as_str().unwrap().starts_with("sha256:"));
    assert_eq!(first["current_schema_sha256"].as_str().unwrap().len(), 64);
    assert_eq!(first["desired_schema_sha256"].as_str().unwrap().len(), 64);
}

#[test]
fn schema_plan_omits_reverse_plan_when_reverse_diff_is_unsupported() {
    let mut current =
        migration_fixture_schema(vec![migration_fixture_column("id", "BIGINT", true)]);
    current.database.as_mut().unwrap().engine = "sqlite".into();
    let mut desired = current.clone();
    desired.tables[0].columns[0].nullable = false;
    let plan = diff(&desired, &current);
    let json = schema_plan_json(&desired, &current, &plan);

    assert_eq!(json["rollback"]["generated"], false);
    assert_eq!(json["rollback"]["plan_id"], Value::Null);
    assert_eq!(json["rollback"]["changes"][0]["risk"], "unsupported");
    assert_eq!(json["rollback"]["requires_verified_backup"], true);
}

#[test]
fn schema_fingerprint_ignores_inspection_order() {
    let first = migration_fixture_schema(vec![
        migration_fixture_column("name", "VARCHAR(30)", true),
        migration_fixture_column("id", "BIGINT", false),
    ]);
    let second = migration_fixture_schema(vec![
        migration_fixture_column("id", "BIGINT", false),
        migration_fixture_column("name", "VARCHAR(30)", true),
    ]);
    assert_eq!(schema_fingerprint(&first), schema_fingerprint(&second));
}

#[test]
fn table_module_grants_apply_least_privilege_and_unknown_requires_combined_access() {
    let program = parse(
        &lex(
            "table customers { id: Id primary auto access { read: [\"src/report.zyl\"] write: [\"src/importer.zyl\"] read_write: [\"src/admin.zyl\"] } }",
        )
        .unwrap(),
    )
    .unwrap();
    let table = &program.tables[0];

    assert!(table_access_granted(table, "src/report.zyl", "read"));
    assert!(!table_access_granted(table, "src/report.zyl", "write"));
    assert!(!table_access_granted(table, "src/report.zyl", "read_write"));
    assert!(table_access_granted(table, "src/importer.zyl", "write"));
    assert!(!table_access_granted(table, "src/importer.zyl", "read"));
    assert!(!table_access_granted(table, "src/importer.zyl", "unknown"));
    assert!(table_access_granted(table, "src/admin.zyl", "unknown"));

    let separate = parse(
        &lex(
            "table customers { id: Id primary auto access { read: [\"src/service.zyl\"] write: [\"src/service.zyl\"] } }",
        )
        .unwrap(),
    )
    .unwrap();
    assert!(table_access_granted(
        &separate.tables[0],
        "src/service.zyl",
        "read_write"
    ));
    assert!(!table_access_granted(
        &separate.tables[0],
        "src/service.zyl",
        "unknown"
    ));
}

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios",
    windows
))]
#[test]
fn bundle_publish_never_replaces_an_existing_empty_directory() {
    let root = std::env::temp_dir().join(format!(
        "zelyra-bundle-no-replace-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let staging = root.join("staging");
    let output = root.join("output");
    fs::create_dir_all(&staging).unwrap();
    fs::create_dir(&output).unwrap();
    fs::write(staging.join("bundle.txt"), "staged bundle").unwrap();

    let error = publish_directory_no_replace(&staging, &output)
        .expect_err("existing destination must not be replaced");
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
    assert_eq!(
        fs::read_to_string(staging.join("bundle.txt")).unwrap(),
        "staged bundle"
    );

    fs::remove_dir(&output).unwrap();
    publish_directory_no_replace(&staging, &output)
        .expect("a missing destination should accept the complete staged directory");
    assert_eq!(
        fs::read_to_string(output.join("bundle.txt")).unwrap(),
        "staged bundle"
    );
    assert!(!staging.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn database_environment_names_are_derived_from_database_declarations() {
    assert_eq!(
        database_url_environment_name("main"),
        "ZELYRA_DATABASE_MAIN_URL"
    );
    assert_eq!(
        database_url_environment_name("sales_2"),
        "ZELYRA_DATABASE_SALES_2_URL"
    );
    assert_eq!(
        database_url_environment_name("sales-west"),
        "ZELYRA_DATABASE_SALES_WEST_URL"
    );
}

#[test]
fn named_database_url_in_env_file_precedes_legacy_url_and_falls_back() {
    let directory = std::env::temp_dir().join(format!(
        "zelyra-database-env-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let env_file = directory.join(".env");
    fs::write(
        &env_file,
        "DATABASE_URL=mariadb://legacy.invalid/app\nZELYRA_DATABASE_MAIN_URL=mariadb://named.invalid/app\n",
    )
    .unwrap();

    assert_eq!(
        database_url_from_env_file(env_file.to_str().unwrap(), Some("main")).unwrap(),
        Some((
            "mariadb://named.invalid/app".to_owned(),
            "ZELYRA_DATABASE_MAIN_URL".to_owned()
        ))
    );

    fs::write(&env_file, "DATABASE_URL=mariadb://legacy.invalid/app\n").unwrap();
    assert_eq!(
        database_url_from_env_file(env_file.to_str().unwrap(), Some("main")).unwrap(),
        Some((
            "mariadb://legacy.invalid/app".to_owned(),
            "DATABASE_URL".to_owned()
        ))
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn feature_defaults_are_simple_and_enabled() {
    let features = feature_defaults();
    assert_eq!(features.len(), PROJECT_FEATURES.len());
    assert!(features.values().all(|setting| setting.enabled));
    assert!(features.values().all(|setting| setting.source == "default"));
}

#[test]
fn feature_settings_accept_known_manifest_values() {
    let values = parse_feature_section(
        "[project]\nname = \"demo\"\n\n[features]\napi = false\ncrud = true\n",
    )
    .expect("feature settings should parse");
    assert_eq!(values.get("api"), Some(&false));
    assert_eq!(values.get("crud"), Some(&true));
    assert!(!values.contains_key("web"));
}

#[test]
fn project_environment_settings_use_process_then_dotenv_then_fallback() {
    let directory = std::env::temp_dir().join(format!(
        "zelyra-ui-settings-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let source_path = directory.join("main.zyl");
    fs::write(&source_path, "").unwrap();
    fs::write(directory.join(".env"), "ZELYRA_TEST_UI_LOCALE=de\n").unwrap();

    let key = "ZELYRA_TEST_UI_LOCALE";
    let previous = env::var_os(key);
    env::remove_var(key);
    assert_eq!(
        project_ui_setting(source_path.to_str().unwrap(), key, "en").unwrap(),
        "de"
    );
    env::set_var(key, "en");
    assert_eq!(
        project_ui_setting(source_path.to_str().unwrap(), key, "fallback").unwrap(),
        "en"
    );
    env::remove_var(key);
    assert_eq!(
        project_ui_setting(
            source_path.to_str().unwrap(),
            "ZELYRA_TEST_UI_MISSING",
            "work"
        )
        .unwrap(),
        "work"
    );
    if let Some(previous) = previous {
        env::set_var(key, previous);
    }
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn allowed_hosts_use_process_then_project_env_then_loopback_fallback() {
    let directory = std::env::temp_dir().join(format!(
        "zelyra-allowed-hosts-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let source_path = directory.join("main.zyl");
    fs::write(&source_path, "").unwrap();
    fs::write(
        directory.join(".env"),
        "ZELYRA_ALLOWED_HOSTS=localhost,app.example\n",
    )
    .unwrap();

    let previous = env::var_os("ZELYRA_ALLOWED_HOSTS");
    env::remove_var("ZELYRA_ALLOWED_HOSTS");
    assert_eq!(
        project_allowed_hosts(source_path.to_str().unwrap()).unwrap(),
        ["localhost", "app.example"]
    );
    fs::remove_file(directory.join(".env")).unwrap();
    assert_eq!(
        project_allowed_hosts(source_path.to_str().unwrap()).unwrap(),
        ["localhost", "127.0.0.1", "[::1]"]
    );
    fs::write(
        directory.join(".env"),
        "ZELYRA_ALLOWED_HOSTS=localhost,app.example\n",
    )
    .unwrap();
    env::set_var("ZELYRA_ALLOWED_HOSTS", "override.example, localhost");
    assert_eq!(
        project_allowed_hosts(source_path.to_str().unwrap()).unwrap(),
        ["override.example", "localhost"]
    );
    env::set_var("ZELYRA_ALLOWED_HOSTS", " , ");
    assert!(project_allowed_hosts(source_path.to_str().unwrap()).is_err());
    env::set_var("ZELYRA_ALLOWED_HOSTS", "localhost, ");
    assert!(project_allowed_hosts(source_path.to_str().unwrap()).is_err());
    if let Some(previous) = previous {
        env::set_var("ZELYRA_ALLOWED_HOSTS", previous);
    } else {
        env::remove_var("ZELYRA_ALLOWED_HOSTS");
    }
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn project_theme_css_is_optional_utf8_and_size_limited() {
    let directory = env::temp_dir().join(format!(
        "zelyra-project-theme-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let source_path = directory.join("main.zyl");
    fs::write(&source_path, "").unwrap();
    assert_eq!(
        project_theme_css(source_path.to_str().unwrap()).unwrap(),
        None
    );

    let theme_path = directory.join(PROJECT_THEME_CSS_FILE);
    let theme = ":root { --zelyra-color-accent: #e04b67; }";
    fs::write(&theme_path, theme).unwrap();
    assert_eq!(
        project_theme_css(source_path.to_str().unwrap()).unwrap(),
        Some(theme.into())
    );

    fs::write(
        &theme_path,
        vec![b'x'; PROJECT_THEME_CSS_MAX_BYTES as usize + 1],
    )
    .unwrap();
    assert!(project_theme_css(source_path.to_str().unwrap())
        .unwrap_err()
        .contains("128 KiB size limit"));

    fs::write(&theme_path, [0xff, 0xfe]).unwrap();
    assert!(project_theme_css(source_path.to_str().unwrap())
        .unwrap_err()
        .contains("UTF-8"));

    fs::remove_file(&theme_path).unwrap();
    fs::create_dir(&theme_path).unwrap();
    assert!(project_theme_css(source_path.to_str().unwrap())
        .unwrap_err()
        .contains("regular project file"));
    fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn project_theme_css_does_not_follow_symbolic_links() {
    use std::os::unix::fs::symlink;

    let directory = env::temp_dir().join(format!(
        "zelyra-project-theme-link-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let source_path = directory.join("main.zyl");
    let outside_file = directory.join("private.css");
    fs::write(&source_path, "").unwrap();
    fs::write(&outside_file, "private content").unwrap();
    symlink(&outside_file, directory.join(PROJECT_THEME_CSS_FILE)).unwrap();

    let error = project_theme_css(source_path.to_str().unwrap()).unwrap_err();
    assert!(error.contains("not a symbolic link"));
    assert!(!error.contains("private content"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn project_locale_catalogs_are_optional_validated_and_size_limited() {
    let directory = env::temp_dir().join(format!(
        "zelyra-project-locales-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let source_path = directory.join("main.zyl");
    fs::write(&source_path, "").unwrap();
    assert!(project_ui_catalogs(source_path.to_str().unwrap()).is_ok());

    let locale_directory = directory.join(PROJECT_LOCALE_DIRECTORY);
    fs::create_dir(&locale_directory).unwrap();
    fs::write(
        locale_directory.join("de.json"),
        r#"{"custom.title":"Titel"}"#,
    )
    .unwrap();
    fs::write(
        locale_directory.join("en.json"),
        r#"{"custom.title":"Title"}"#,
    )
    .unwrap();
    assert!(project_ui_catalogs(source_path.to_str().unwrap()).is_ok());

    fs::write(locale_directory.join("de.json"), r#"{"custom.title":true}"#).unwrap();
    let error = project_ui_catalogs(source_path.to_str().unwrap()).unwrap_err();
    assert!(error.contains("locales/de.json"));
    assert!(error.contains("string values"));
    assert!(!error.contains("true"));

    fs::write(
        locale_directory.join("de.json"),
        vec![b'x'; PROJECT_LOCALE_MAX_BYTES as usize + 1],
    )
    .unwrap();
    assert!(project_ui_catalogs(source_path.to_str().unwrap())
        .unwrap_err()
        .contains("256 KiB size limit"));
    fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn project_locale_catalog_loader_does_not_follow_symbolic_links() {
    use std::os::unix::fs::symlink;

    let directory = env::temp_dir().join(format!(
        "zelyra-project-locales-link-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let source_path = directory.join("main.zyl");
    fs::write(&source_path, "").unwrap();
    let outside_file = directory.join("outside.json");
    fs::write(&outside_file, r#"{"title":"private"}"#).unwrap();
    let locale_directory = directory.join(PROJECT_LOCALE_DIRECTORY);
    fs::create_dir(&locale_directory).unwrap();
    symlink(&outside_file, locale_directory.join("de.json")).unwrap();

    let error = project_ui_catalogs(source_path.to_str().unwrap()).unwrap_err();
    assert!(error.contains("locales/de.json"));
    assert!(error.contains("symbolic link"));
    assert!(!error.contains("private"));

    fs::remove_file(locale_directory.join("de.json")).unwrap();
    fs::remove_dir(&locale_directory).unwrap();
    let outside_directory = directory.join("outside-locales");
    fs::create_dir(&outside_directory).unwrap();
    symlink(&outside_directory, &locale_directory).unwrap();
    let error = project_ui_catalogs(source_path.to_str().unwrap()).unwrap_err();
    assert!(error.contains("locales"));
    assert!(error.contains("symbolic link"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn project_theme_stylesheet_route_is_reserved_only_by_theme_projects() {
    let themed_program =
        parse(&lex("page \"/__zelyra/theme.css\" { html { <main>Theme</main> } }").unwrap())
            .unwrap();
    let parameter_program =
        parse(&lex("page \"/{namespace}/{asset}\" { html { <main>Asset</main> } }").unwrap())
            .unwrap();
    let ordinary_program = parse(&lex("fn main() { }").unwrap()).unwrap();

    assert!(project_uses_reserved_theme_route(&themed_program));
    assert!(project_uses_reserved_theme_route(&parameter_program));
    assert!(!project_uses_reserved_theme_route(&ordinary_program));
}

#[test]
fn project_routes_cannot_shadow_the_builtin_liveness_endpoint() {
    let direct = parse(
        &lex(r#"page "/__zelyra/health/live" { html { <main>Project page</main> } }"#).unwrap(),
    )
    .unwrap();
    let wildcard =
        parse(&lex(r#"api GET "/__zelyra/{resource}/{action}" { output String }"#).unwrap())
            .unwrap();
    let unrelated =
        parse(&lex(r#"page "/health" { html { <main>Health</main> } }"#).unwrap()).unwrap();

    assert!(project_uses_reserved_health_route(&direct));
    assert!(project_uses_reserved_health_route(&wildcard));
    assert!(!project_uses_reserved_health_route(&unrelated));
}

#[test]
fn account_session_route_is_reserved_for_authenticated_projects() {
    let auth_page = parse(
        &lex(r#"auth users { table: users }
               page "/account/sessions" { html { <main>Project page</main> } }"#)
        .unwrap(),
    )
    .unwrap();
    let auth_api = parse(
        &lex(r#"auth users { table: users }
               api GET "/account/{area}" { output String }"#)
        .unwrap(),
    )
    .unwrap();
    let auth_crud = parse(
        &lex(r#"auth users { table: users }
               table account { id: Id primary auto }
               crud Account -> account { list { id } }"#)
        .unwrap(),
    )
    .unwrap();
    let ordinary_page =
        parse(&lex(r#"page "/account/sessions" { html { <main>Project page</main> } }"#).unwrap())
            .unwrap();

    assert!(project_uses_reserved_account_sessions_route(&auth_page));
    assert!(project_uses_reserved_account_sessions_route(&auth_api));
    assert!(project_uses_reserved_account_sessions_route(&auth_crud));
    assert!(!project_uses_reserved_account_sessions_route(
        &ordinary_page
    ));
}

#[test]
fn feature_settings_accept_only_known_env_overrides() {
    let values =
        parse_env_feature_overrides("# optional\nZELYRA_FEATURE_API=false\nZELYRA_WEB_PORT=3000\n")
            .expect("feature environment settings should parse");
    assert_eq!(values.get("api"), Some(&false));
    assert_eq!(values.len(), 1);
}

#[test]
fn feature_settings_reject_unknown_values() {
    let error = parse_feature_section("[features]\nmagic = true\n")
        .expect_err("unknown features must not be silently accepted");
    assert!(error.contains("unknown feature setting"));
    let error = parse_env_feature_overrides("ZELYRA_FEATURE_MAGIC=true\n")
        .expect_err("unknown environment features must not be silently accepted");
    assert!(error.contains("unknown ZELYRA_FEATURE_"));
}

#[test]
fn formats_verification_results_with_source_location() {
    let result = VerificationResult {
        function: "reduce".into(),
        kind: zelyra_runtime::ContractKind::LoopInvariant,
        index: 0,
        status: VerificationStatus::Proven,
        span: zelyra_ast::Span::new(6, 12, 2, 1),
        message: "The verifier proved this condition for all analyzed paths.".into(),
        counterexample: None,
    };
    assert_eq!(
        format_verification_result("src/reduce.zyl", "first\nsecond value\n", &result),
        "PROVEN [V-001]: reduce.invariant[0] (src/reduce.zyl:2:1-2:7)\n  = The verifier proved this condition for all analyzed paths.\n    |\n  2 | second value\n    | ^^^^^^"
    );
}

#[test]
fn doctor_rejects_invalid_port() {
    let arguments = ["--port".to_owned(), "not-a-port".to_owned()];
    assert_eq!(doctor_command(arguments.into_iter()), ExitCode::from(2));
    let arguments = ["--port".to_owned(), "0".to_owned()];
    assert_eq!(doctor_command(arguments.into_iter()), ExitCode::from(2));
}

#[test]
fn reads_database_url_from_an_env_file_without_normalizing_secrets() {
    let path = env::temp_dir().join(format!(
        "zelyra-doctor-env-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &path,
        "# local\nexport DATABASE_URL='mariadb://user:secret@127.0.0.1:3306/app'\n",
    )
    .unwrap();
    assert_eq!(
        read_env_value(path.to_str().unwrap(), "DATABASE_URL").unwrap(),
        Some("mariadb://user:secret@127.0.0.1:3306/app".into())
    );
    fs::remove_file(path).unwrap();
}

#[test]
fn formats_doctor_json_without_database_credentials() {
    let checks = vec![
        DoctorCheck {
            name: "project_file",
            category: "configuration",
            status: "pass",
            message: "app.zyl exists".into(),
        },
        DoctorCheck {
            name: "static_checks",
            category: "project",
            status: "pass",
            message: "source is valid".into(),
        },
        DoctorCheck {
            name: "database",
            category: "configuration",
            status: "warn",
            message: "mariadb: DATABASE_URL is not set".into(),
        },
    ];
    let document: serde_json::Value =
        serde_json::from_str(&format_doctor_json("app.zyl", &checks)).unwrap();
    assert_eq!(document["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(document["status"], "ready");
    assert_eq!(document["warnings"], 1);
    assert_eq!(document["checks"][1]["category"], "project");
    assert_eq!(document["checks"][2]["status"], "warn");
    assert_eq!(document["checks"][2]["category"], "configuration");
    assert!(!format_doctor_json("app.zyl", &checks).contains("password"));
}

#[test]
fn doctor_classifies_database_failures_without_returning_backend_details() {
    let cases = [
        ("Access denied for user `app`", "authentication"),
        ("connection timed out", "timeout"),
        (
            "query execution was interrupted (max_statement_time exceeded)",
            "timeout",
        ),
        ("Unknown database 'private_name'", "configuration"),
        ("could not start mariadb: executable missing", "tooling"),
        ("Can't connect to server on 'db.example'", "connectivity"),
        ("unexpected column metadata", "schema"),
    ];
    for (detail, expected) in cases {
        let category = doctor_database_category(detail);
        assert_eq!(category, expected, "wrong category for {detail}");
        let safe_message = doctor_database_failure_message(category);
        assert!(!safe_message.contains(detail));
        assert!(!safe_message.contains("private_name"));
    }
}

#[test]
fn formats_verification_results_as_json() {
    let result = VerificationResult {
        function: "say\"hello".into(),
        kind: zelyra_runtime::ContractKind::Ensures,
        index: 1,
        status: VerificationStatus::RuntimeCheck,
        span: zelyra_ast::Span::new(6, 12, 2, 1),
        message: "This postcondition needs a runtime check because not all return paths are symbolically modeled.".into(),
        counterexample: Some(vec![("value".into(), 0)]),
    };
    assert_eq!(
        format_verification_json("src/file.zyl", "first\nsecond value\n", &[result]),
        r#"[{"status":"RUNTIME_CHECK","code":"V-002","message":"This postcondition needs a runtime check because not all return paths are symbolically modeled.","function":"say\"hello","kind":"ensures","index":1,"counterexample":{"value":0},"location":{"file":"src/file.zyl","start":{"line":2,"column":1},"end":{"line":2,"column":7}}}]"#
    );
}

#[test]
fn formats_audit_rows_as_json_and_csv_without_losing_nulls() {
    let result = QueryResult {
        columns: vec![
            "actor_user_id".into(),
            "event".into(),
            "target_user_id".into(),
            "details".into(),
            "created_at".into(),
        ],
        rows: vec![vec![
            "NULL".into(),
            "auth.login_failed".into(),
            "NULL".into(),
            "email=anna@example.test;note=\"unknown\"".into(),
            "2026-09-17 12:00:00".into(),
        ]],
    };
    let json: serde_json::Value = serde_json::from_str(&audit_rows_json(&result)).unwrap();
    assert_eq!(json[0]["actor_user_id"], serde_json::Value::Null);
    assert_eq!(json[0]["target_user_id"], serde_json::Value::Null);
    assert_eq!(json[0]["event"], "auth.login_failed");
    assert!(audit_rows_csv(&result).contains("\"email=anna@example.test;note=\"\"unknown\"\"\""));
}

#[test]
fn composes_a_page_inside_its_named_view() {
    let source = r#"
        view Shell {
            html { <body><slot /></body> }
        }
        page "/hello/{name}" {
            view: Shell
            html { <h1>Hello, {name}!</h1> }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_views("views.zyl", &program));
    let html = compose_page_view(&program, &program.pages[0]);
    assert!(html.contains("<body>"));
    assert!(html.contains("<h1>Hello, {name}!</h1>"));
    assert!(!html.contains("<slot />"));
}

#[test]
fn composes_default_component_slots_and_nested_components() {
    let source = r#"
        component Panel {
            html { <section class="panel"><slot /></section> }
        }
        component Badge {
            props { text: String }
            html { <strong>{text}</strong> }
        }
        page "/status" {
            html {
                <Panel><Badge text="Ready" /></Panel>
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_components("components.zyl", &program));
    let html = compose_page_view(&program, &program.pages[0]);
    assert!(html.contains("<section class=\"panel\">"));
    assert!(html.contains("<strong>Ready</strong>"));
    assert!(!html.contains("<slot />"));
}

#[test]
fn validates_route_values_used_by_typed_view_components() {
    let source = r#"
        component Greeting {
            props { text: String }
            html { <strong>{text}</strong> }
        }
        page "/hello/{name}" {
            html { <Greeting text="{name}" /> }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_components("views.zyl", &program));
}

#[test]
fn validates_typed_page_data_field_bindings() {
    let source = r#"
        table customers { id: Id primary auto name: String(100) }
        page "/customers/{name}" {
            load customer = sql<Customer> {
                SELECT id, name FROM customers WHERE name = :name
            }
            html { <h1>{customer.name}</h1> }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_page_data("views.zyl", &program));
    assert!(validate_components("views.zyl", &program));
}

#[test]
fn rejects_unknown_typed_page_data_field() {
    let source = r#"
        table customers { id: Id primary auto name: String(100) }
        page "/customers/{name}" {
            load customer = sql<Customer> {
                SELECT id, name FROM customers WHERE name = :name
            }
            html { <h1>{customer.email}</h1> }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_page_data("views.zyl", &program));
    assert!(!validate_components("views.zyl", &program));
}

#[test]
fn validates_typed_page_collection_loops() {
    let source = r#"
        table customers { id: Id primary auto name: String(100) }
        page "/customers" {
            load customers = sql<Customer[]> {
                SELECT id, name FROM customers
            }
            html {
                <ul>
                    for customer in customers {
                        <li>{customer.name}</li>
                    }
                </ul>
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_page_data("views.zyl", &program));
    assert!(validate_components("views.zyl", &program));
}

#[test]
fn validates_typed_page_filters_and_rejects_unknown_fields() {
    let valid_source = r#"
        table customers {
            id: Id primary auto
            name: String(100) required
            active: Bool default true
        }
        page "/customers" {
            filter { name active }
            load customers = sql<Customer[]> {
                SELECT id, name, active FROM customers
            }
            html { <p>{filter_name}</p> }
        }
    "#;
    let valid_program = parse(&lex(valid_source).unwrap()).unwrap();
    assert!(validate_page_data("filters.zyl", &valid_program));
    assert!(validate_components("filters.zyl", &valid_program));

    let invalid_source = valid_source.replace("name active", "username active");
    let invalid_program = parse(&lex(&invalid_source).unwrap()).unwrap();
    assert!(!validate_page_data("filters.zyl", &invalid_program));
}

#[test]
fn rejects_non_array_page_collection_loops() {
    let source = r#"
        table customers { id: Id primary auto name: String(100) }
        page "/customers" {
            load customer = sql<Customer> {
                SELECT id, name FROM customers
            }
            html {
                for customer in customer {
                    <p>{customer.name}</p>
                }
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_page_data("views.zyl", &program));
    assert!(!validate_components("views.zyl", &program));
}

#[test]
fn rejects_unknown_view_values() {
    let source = r#"
        page "/hello" {
            html { <p>{missing}</p> }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(!validate_components("views.zyl", &program));
}

#[test]
fn rejects_route_values_with_an_incompatible_component_property_type() {
    let source = r#"
        component Counter {
            props { count: Int }
            html { <strong>{count}</strong> }
        }
        page "/hello/{name}" {
            html { <Counter count="{name}" /> }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(!validate_components("views.zyl", &program));
}

#[test]
fn rejects_component_content_without_a_default_slot() {
    let source = r#"
        component Panel {
            html { <section /> }
        }
        page "/status" {
            html { <Panel><p>Unexpected content</p></Panel> }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(!validate_components("components.zyl", &program));
}

#[test]
fn composes_named_component_slots() {
    let source = r#"
        component Layout {
            html {
                <header><slot name="header" /></header>
                <main><slot /></main>
            }
        }
        page "/dashboard" {
            html {
                <Layout>
                    <slot name="header"><h1>Dashboard</h1></slot>
                    <p>Content</p>
                </Layout>
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_components("components.zyl", &program));
    let html = compose_page_view(&program, &program.pages[0]);
    assert!(html.contains("<header><h1>Dashboard</h1></header>"));
    assert!(html.contains("<main>"));
    assert!(html.contains("<p>Content</p>"));
    assert!(!html.contains("<slot"));
}

#[test]
fn allows_component_slots_without_a_page_view_layout() {
    let source = r#"
        component DashboardPanel {
            html {
                <section>
                    <header><slot name="header"><h1>Dashboard</h1></slot></header>
                    <main><slot /></main>
                </section>
            }
        }
        page "/dashboard" {
            html {
                <DashboardPanel>
                    <slot name="header"><h1>Custom dashboard</h1></slot>
                    <p>Reusable content.</p>
                </DashboardPanel>
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_views("components.zyl", &program));
    assert!(validate_components("components.zyl", &program));
    let html = compose_page_view(&program, &program.pages[0]);
    assert!(html.contains("Custom dashboard"));
    assert!(html.contains("Reusable content."));
}

#[test]
fn uses_named_component_slot_fallbacks_when_not_overridden() {
    let source = r#"
        component Layout {
            html {
                <header><slot name="header"><h1>Default heading</h1></slot></header>
                <main><slot /></main>
            }
        }
        page "/dashboard" {
            html {
                <Layout><p>Content</p></Layout>
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_components("components.zyl", &program));
    let html = compose_page_view(&program, &program.pages[0]);
    assert!(html.contains("<header><h1>Default heading</h1></header>"));
    assert!(html.contains("<main><p>Content</p></main>"));
    assert!(!html.contains("<slot"));
}

#[test]
fn supplied_named_component_slot_replaces_its_fallback() {
    let source = r#"
        component Layout {
            html { <header><slot name="header"><h1>Default heading</h1></slot></header> }
        }
        page "/dashboard" {
            html {
                <Layout><slot name="header"><h1>Custom heading</h1></slot></Layout>
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_components("components.zyl", &program));
    let html = compose_page_view(&program, &program.pages[0]);
    assert!(html.contains("<header><h1>Custom heading</h1></header>"));
    assert!(!html.contains("Default heading"));
}

#[test]
fn rejects_unknown_named_component_slots() {
    let source = r#"
        component Layout {
            html { <main><slot name="content" /></main> }
        }
        page "/dashboard" {
            html {
                <Layout><slot name="footer"><p>Footer</p></slot></Layout>
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(!validate_components("components.zyl", &program));
}

#[test]
fn composes_named_view_slots_and_fallbacks() {
    let source = r#"
        view Shell {
            html {
                <html>
                    <body>
                        <header><slot name="header"><h1>Default heading</h1></slot></header>
                        <main><slot /></main>
                        <footer><slot name="footer">Default footer</slot></footer>
                    </body>
                </html>
            }
        }
        page "/dashboard" {
            view: Shell
            html {
                <slot name="header"><h1>Custom heading</h1></slot>
                <p>Dashboard content</p>
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_views("views.zyl", &program));
    assert!(validate_components("views.zyl", &program));
    let html = compose_page_view(&program, &program.pages[0]);
    assert!(html.contains("<header><h1>Custom heading</h1></header>"));
    assert!(html.contains("<main>"));
    assert!(html.contains("<p>Dashboard content</p>"));
    assert!(html.contains("<footer>Default footer</footer>"));
    assert!(!html.contains("<slot"));
}

#[test]
fn rejects_unknown_named_view_slots() {
    let source = r#"
        view Shell {
            html {
                <body>
                    <header><slot name="header" /></header>
                    <main><slot /></main>
                </body>
            }
        }
        page "/dashboard" {
            view: Shell
            html {
                <slot name="footer"><p>Footer</p></slot>
                <p>Dashboard content</p>
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(!validate_views("views.zyl", &program));
}

#[test]
fn composes_crud_layout_with_generated_content_marker() {
    let source = r#"
        component Badge {
            props { label: String }
            html { <strong>{label}</strong> }
        }
        view Shell {
            html {
                <html><body>
                    <header><slot name="heading"><h1>Customers</h1></slot></header>
                    <main><slot /></main>
                    <aside><slot name="help"><p>Default help</p></slot></aside>
                    <footer><slot name="footer"><p>Default footer</p></slot></footer>
                </body></html>
            }
        }
        table customers { id: Id primary auto name: String(100) }
        crud Customer -> customers {
            layout: Shell
            slots {
                heading { html { <Badge label="Machine register" /> } }
                help { html { <p>Choose a machine to see its details.</p> } }
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_views("views.zyl", &program));
    assert!(validate_components("views.zyl", &program));
    let layout = crud_layout_html(&program, &program.cruds[0]).unwrap();
    assert!(layout.contains("<strong>Machine register</strong>"));
    assert!(layout.contains(CRUD_LAYOUT_CONTENT_MARKER));
    assert!(layout.contains("<p>Choose a machine to see its details.</p>"));
    assert!(!layout.contains("Default help"));
    assert!(layout.contains("Default footer"));
    assert!(!layout.contains("<slot"));
}

#[test]
fn rejects_crud_layout_slots_without_a_matching_layout() {
    let source = r#"
        view Shell {
            html { <header><slot name="heading" /></header><main><slot /></main> }
        }
        table customers { id: Id primary auto }
        crud Customer -> customers {
            layout: Shell
            slots { missing { html { <p>Custom content</p> } } }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(!validate_views("views.zyl", &program));

    let source_without_layout = r#"
        table customers { id: Id primary auto }
        crud Customer -> customers {
            slots { heading { html { <h1>Customers</h1> } } }
        }
    "#;
    let program = parse(&lex(source_without_layout).unwrap()).unwrap();
    assert!(!validate_views("views.zyl", &program));

    let duplicate_slots = r#"
        view Shell {
            html {
                <header><slot name="heading" /></header><main><slot /></main>
            }
        }
        table customers { id: Id primary auto }
        crud Customer -> customers {
            layout: Shell
            slots {
                heading { html { <h1>First heading</h1> } }
                heading { html { <h1>Second heading</h1> } }
            }
        }
    "#;
    let program = parse(&lex(duplicate_slots).unwrap()).unwrap();
    assert!(!validate_views("views.zyl", &program));
}

#[test]
fn crud_layout_slot_content_cannot_read_crud_record_values() {
    let source = r#"
        view Shell {
            html { <header><slot name="heading" /></header><main><slot /></main> }
        }
        table customers { id: Id primary auto name: String(100) }
        crud Customer -> customers {
            layout: Shell
            slots { heading { html { <h1>{customer.name}</h1> } } }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_views("views.zyl", &program));
    assert!(!validate_components("views.zyl", &program));
}

#[test]
fn crud_layout_components_are_checked_even_without_a_page_using_the_view() {
    let source = r#"
        component BrandMark {
            props { label: String }
            html { <strong>{label}</strong> }
        }
        view Shell {
            html {
                <header><slot name="heading" /><BrandMark /></header><main><slot /></main>
            }
        }
        table customers { id: Id primary auto }
        crud Customer -> customers {
            layout: Shell
            slots { heading { html { <h1>Customers</h1> } } }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_views("views.zyl", &program));
    assert!(!validate_components("views.zyl", &program));
}

#[test]
fn rejects_unknown_crud_layout_view() {
    let source = r#"
        table customers { id: Id primary auto }
        crud Customer -> customers { layout: MissingShell }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(!validate_views("views.zyl", &program));
}

#[test]
fn rejects_duplicate_named_view_slots() {
    let source = r#"
        view Shell {
            html {
                <header><slot name="header" /></header>
                <main><slot /></main>
            }
        }
        page "/dashboard" {
            view: Shell
            html {
                <slot name="header"><h1>First</h1></slot>
                <slot name="header"><h1>Second</h1></slot>
                <p>Dashboard content</p>
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(!validate_views("views.zyl", &program));
}

#[test]
fn rejects_invalid_named_view_composition() {
    let source = r#"
        view Shell {
            html { <body>No content slot</body> }
        }
        page "/customers" {
            view: MissingShell
            html { <h1>Customers</h1> }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(!validate_views("views.zyl", &program));
}

#[test]
fn formats_api_declarations_as_openapi() {
    let program = parse(
        &lex(
            "type CustomerId = Id table customers { id: CustomerId primary auto name: String(100) required } api GET \"/customers/{id}\" { version \"v1\" deprecated rate_limit 30 per 60 input { id: CustomerId } output Customer errors { 404 NotFound } } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    let document = docs::format_openapi(&program);
    assert!(document.contains("\"openapi\":\"3.0.3\""));
    assert!(document.contains("\"/customers/{id}\""));
    assert!(document.contains("\"404\":{\"description\":\"NotFound\"}"));
    assert!(document.contains("#/components/schemas/Customer"));
    assert!(document.contains("\"x-zelyra-api-version\":\"v1\""));
    assert!(document.contains("\"deprecated\":true"));
    assert!(document.contains("\"x-zelyra-rate-limit\":{\"requests\":30,\"window_seconds\":60}"));
}

#[test]
fn formats_typed_typescript_client_from_api_and_records() {
    let program = parse(
        &lex(
            "struct Address { city: String } api GET \"/customers/{id}\" { input { id: Id, term: String? } output Address errors { 404 NotFound } } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    let client = docs::format_typescript_client(&program);
    assert!(client.contains("export interface Address"));
    assert!(client.contains("async get_customers_id"));
    assert!(client.contains("encodeURIComponent(String(params.id))"));
    assert!(client.contains("Promise<Address>"));
    assert!(client.contains("new URLSearchParams()"));
    assert!(client.contains("ZelyraApiError"));
    assert!(client.contains("ZelyraApiErrorCode = \"NotFound\" | (string & {})"));
    assert!(client.contains("fromResponse(response)"));
    assert!(client.contains("payload.error.message"));
}

#[test]
fn formats_typed_api_error_payloads_for_openapi_and_typescript() {
    let program = parse(
        &lex(
            "struct Problem { message: String } api GET \"/fail\" { output Result<String, Problem> errors { 422 Validation: Problem } } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    let openapi = docs::format_openapi(&program);
    assert!(openapi.contains("\"details\""));
    assert!(openapi.contains("#/components/schemas/Problem"));
    let client = docs::format_typescript_client(&program);
    assert!(client.contains("ZelyraApiErrorPayloads"));
    assert!(client.contains("\"Validation\": Problem"));
    assert!(client.contains("details: Details | undefined"));
}

#[test]
fn dispatches_json_api_input_to_a_typed_handler() {
    let program = parse(
        &lex(
            "api POST \"/echo\" { handler echo input { value: Int } output Int } fn echo(value: Int) -> Int { return value } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    assert!(check_apis(&program).is_ok());
    let api = &program.apis[0];
    let request = zelyra_web::parse_request(
        "POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\r\n{\"value\":42}",
    )
    .unwrap();
    let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
    assert_eq!(response.status, 200);
    assert_eq!(response.content_type, "application/json; charset=utf-8");
    assert_eq!(response.body, "42");
}

#[test]
fn enforces_scoped_database_effects_in_generated_api_handlers() {
    let program = parse(
        &lex(
            "fn readiness() -> Int uses Database(read) { return sql<Int> { SELECT 1 } } api GET \"/ready\" { handler readiness output Int } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    assert!(check_apis(&program).is_ok());
    let request =
        zelyra_web::parse_request("GET /ready HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let api = &program.apis[0];
    let database_url = "mariadb://invalid:invalid@127.0.0.1:1/invalid";

    let no_grants = HashSet::new();
    let denied = dispatch_api_with_capabilities(
        &program,
        api,
        "readiness",
        &request,
        &HashMap::new(),
        ApiRuntimeContext {
            database_url: Some(database_url),
            capability_grants: Some(&no_grants),
            runtime_policy: None,
        },
    );
    assert_eq!(denied.status, 500);
    assert!(denied.body.contains("Database(read)"));
    assert!(denied.body.contains("requires `Database(read)`"));

    let read_grant = HashSet::from([String::from("Database(read)")]);
    let allowed = dispatch_api_with_capabilities(
        &program,
        api,
        "readiness",
        &request,
        &HashMap::new(),
        ApiRuntimeContext {
            database_url: Some(database_url),
            capability_grants: Some(&read_grant),
            runtime_policy: None,
        },
    );
    assert_eq!(allowed.status, 500);
    assert!(!allowed.body.contains("not granted"));
}

#[test]
fn dispatches_string_keyed_map_api_input_and_output() {
    let program = parse(
        &lex(
            "api POST \"/settings\" { handler echo input { settings: Map<String, Int> } output Map<String, Int> } fn echo(settings: Map<String, Int>) -> Map<String, Int> { return settings } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    assert!(check_apis(&program).is_ok());
    let api = &program.apis[0];
    let request = zelyra_web::parse_request(
        "POST /settings HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"settings\":{\"standard\":10,\"premium\":20}}",
    )
    .unwrap();
    let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
    assert_eq!(response.status, 200);
    let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(body, serde_json::json!({"premium": 20, "standard": 10}));
}

#[test]
fn returns_structured_json_for_invalid_api_input() {
    let program = parse(
        &lex(
            "api POST \"/echo\" { handler echo input { value: Int } output Int } fn echo(value: Int) -> Int { return value } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    let api = &program.apis[0];
    let request = zelyra_web::parse_request(
        "POST /echo HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{}",
    )
    .unwrap();
    let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
    assert_eq!(response.status, 400);
    assert_eq!(response.content_type, "application/json; charset=utf-8");
    assert!(response.body.contains("\"code\":\"BadRequest\""));
    assert!(response.body.contains("missing API input"));
}

#[test]
fn rejects_unsupported_api_request_media_types() {
    let program = parse(
        &lex(
            "api POST \"/echo\" { handler echo input { value: String } output String } fn echo(value: String) -> String { return value } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    let api = &program.apis[0];
    let request =
        zelyra_web::parse_request("POST /echo HTTP/1.1\r\nContent-Type: text/plain\r\n\r\nhello")
            .unwrap();
    let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
    assert_eq!(response.status, 415);
    assert!(response.body.contains("UnsupportedMediaType"));
}

#[test]
fn decodes_json_strings_with_commas_colons_and_escapes() {
    let program = parse(
        &lex(
            "api POST \"/echo\" { handler echo input { value: String } output String } fn echo(value: String) -> String { return value } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    let api = &program.apis[0];
    let request = zelyra_web::parse_request(
        "POST /echo HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"value\":\"a,b: \\\"quoted\\\"\"}",
    )
    .unwrap();
    let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
    assert_eq!(response.status, 200);
    assert_eq!(response.body, "\"a,b: \\\"quoted\\\"\"");
}

#[test]
fn decodes_json_booleans_and_null_options() {
    let program = parse(
        &lex(
            "api POST \"/echo\" { handler echo input { active: Bool? } output Bool? } fn echo(active: Bool?) -> Bool? { return active } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    let api = &program.apis[0];
    let request = zelyra_web::parse_request(
        "POST /echo HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"active\":null}",
    )
    .unwrap();
    let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
    assert_eq!(response.status, 200);
    assert_eq!(response.body, "null");
}

#[test]
fn decodes_and_serializes_typed_json_arrays() {
    let program = parse(
        &lex(
            "api POST \"/echo\" { handler echo input { values: Int[] } output Int[] } fn echo(values: Int[]) -> Int[] { return values } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    assert!(check_apis(&program).is_ok());
    let api = &program.apis[0];
    let request = zelyra_web::parse_request(
        "POST /echo HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"values\":[1,2,3]}",
    )
    .unwrap();
    let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
    assert_eq!(response.status, 200);
    assert_eq!(response.body, "[1,2,3]");
}

#[test]
fn decodes_and_serializes_nested_structured_api_objects() {
    let program = parse(
        &lex(
            "struct Address { city: String } struct CustomerInput { name: String address: Address } api POST \"/customers\" { handler echo input { customer: CustomerInput } output CustomerInput } fn echo(customer: CustomerInput) -> CustomerInput { return customer } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    assert!(check_apis(&program).is_ok());
    let api = &program.apis[0];
    let request = zelyra_web::parse_request(
        "POST /customers HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"customer\":{\"name\":\"Anna\",\"address\":{\"city\":\"Berlin\"}}}",
    )
    .unwrap();
    let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
    assert_eq!(response.status, 200);
    let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(body["name"], "Anna");
    assert_eq!(body["address"]["city"], "Berlin");
}

#[test]
fn rejects_unknown_and_missing_record_fields() {
    let program = parse(
        &lex(
            "struct CustomerInput { name: String email: Email? } api POST \"/customers\" { handler echo input { customer: CustomerInput } output CustomerInput } fn echo(customer: CustomerInput) -> CustomerInput { return customer } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    let api = &program.apis[0];
    let unknown = zelyra_web::parse_request(
        "POST /customers HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"customer\":{\"name\":\"Anna\",\"unknown\":true}}",
    )
    .unwrap();
    let response = dispatch_api(&program, api, "echo", &unknown, &HashMap::new(), None);
    assert_eq!(response.status, 400);
    assert!(response.body.contains("unknown field"));

    let missing = zelyra_web::parse_request(
        "POST /customers HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"customer\":{}}",
    )
    .unwrap();
    let response = dispatch_api(&program, api, "echo", &missing, &HashMap::new(), None);
    assert_eq!(response.status, 400);
    assert!(response.body.contains("missing field"));
}

#[test]
fn maps_declared_result_errors_to_http_responses() {
    let program = parse(
        &lex(
            "api GET \"/customers\" { handler find input { } output Result<String, String> errors { 404 NotFound } } fn find() -> Result<String, String> { return Err(\"NotFound\") } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    let api = &program.apis[0];
    let request = zelyra_web::parse_request("GET /customers HTTP/1.1\r\n\r\n").unwrap();
    let response = dispatch_api(&program, api, "find", &request, &HashMap::new(), None);
    assert_eq!(response.status, 404);
    assert!(response.body.contains("\"code\":\"NotFound\""));
}

#[test]
fn returns_internal_error_for_undeclared_result_errors() {
    let program = parse(
        &lex(
            "api GET \"/customers\" { handler find input { } output Result<String, String> errors { 404 NotFound } } fn find() -> Result<String, String> { return Err(\"Other\") } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    let api = &program.apis[0];
    let request = zelyra_web::parse_request("GET /customers HTTP/1.1\r\n\r\n").unwrap();
    let response = dispatch_api(&program, api, "find", &request, &HashMap::new(), None);
    assert_eq!(response.status, 500);
    assert!(response.body.contains("InternalServerError"));
    assert!(response.body.contains("unmapped API error"));
}

#[test]
fn returns_typed_details_for_declared_api_errors() {
    let program = parse(
        &lex(
            "struct Problem { message: String } api GET \"/fail\" { handler fail input { } output Result<String, Problem> errors { 422 Validation: Problem } } fn fail() -> Result<String, Problem> { return Err(Problem { message: \"invalid customer\" }) } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    assert!(check_apis(&program).is_ok());
    let api = &program.apis[0];
    let request = zelyra_web::parse_request("GET /fail HTTP/1.1\r\n\r\n").unwrap();
    let response = dispatch_api(&program, api, "fail", &request, &HashMap::new(), None);
    assert_eq!(response.status, 422);
    let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(body["error"]["code"], "Validation");
    assert_eq!(body["error"]["details"]["message"], "invalid customer");
}

#[test]
fn rejects_api_error_payload_that_does_not_match_result_error_type() {
    let program = parse(
        &lex(
            "struct Problem { message: String } struct Other { code: Int } api GET \"/fail\" { handler fail input { } output Result<String, Problem> errors { 422 Validation: Other } } fn fail() -> Result<String, Problem> { return Err(Problem { message: \"invalid\" }) } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    let errors = check_apis(&program).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("does not match handler error type")));
}

#[test]
fn rejects_object_json_for_scalar_input() {
    let program = parse(
        &lex(
            "api POST \"/echo\" { handler echo input { value: String } output String } fn echo(value: String) -> String { return value } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    let api = &program.apis[0];
    let request = zelyra_web::parse_request(
        "POST /echo HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"value\":{\"nested\":true}}",
    )
    .unwrap();
    let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
    assert_eq!(response.status, 400);
    assert!(response.body.contains("scalar JSON value"));
}

#[test]
fn rejects_unknown_configured_crud_columns() {
    let source = r#"
        table machines {
            id: Id primary auto
            name: String(100) required
        }

        crud Machine -> machines {
            view { fields { missing } }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(!validate_cruds("test.zyl", &program, &schema));
}

#[test]
fn shared_crud_view_fields_drive_list_and_form_defaults() {
    let source = r#"
        table customers {
            id: Id primary auto
            name: String(100) required
            email: Email?
            active: Bool default true
        }

        crud Customer -> customers {
            view { fields { name email active } }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(validate_cruds("test.zyl", &program, &schema));
    let crud = &program.cruds[0];
    let table = program
        .tables
        .iter()
        .find(|table| table.name == "customers")
        .unwrap();
    let form = generated_crud_form(
        crud,
        table,
        &schema,
        false,
        CrudGenerationContext {
            layout_html: None,
            csrf: CsrfProtection::new("test-csrf"),
            audit_table: None,
            audit_chain: false,
        },
    );
    assert_eq!(
        form.form
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["name", "email", "active"]
    );
    let list = configured_crud_columns(&program, &schema, crud, &crud.view.fields, |table| {
        table
            .columns
            .iter()
            .map(|column| column.name.clone())
            .collect()
    });
    assert_eq!(list, ["name", "email", "active"]);
}

#[test]
fn rejects_invalid_soft_delete_columns() {
    let missing_source = r#"
        table machines {
            id: Id primary auto
            name: String(100) required
        }

        crud Machine -> machines {
            soft_delete { column: deleted_at }
        }
    "#;
    let missing_program = parse(&lex(missing_source).unwrap()).unwrap();
    let missing_schema = build_schema(&missing_program).unwrap();
    assert!(!validate_cruds(
        "test.zyl",
        &missing_program,
        &missing_schema
    ));

    let wrong_type_source = r#"
        table machines {
            id: Id primary auto
            name: String(100) required
            deleted_at: String?
        }

        crud Machine -> machines {
            soft_delete { column: deleted_at }
        }
    "#;
    let wrong_type_program = parse(&lex(wrong_type_source).unwrap()).unwrap();
    let wrong_type_schema = build_schema(&wrong_type_program).unwrap();
    assert!(!validate_cruds(
        "test.zyl",
        &wrong_type_program,
        &wrong_type_schema
    ));
}

#[test]
fn rejects_protected_routes_without_auth_definition() {
    let source = r#"
        page "/admin" {
            requires auth
            html {
                <h1>Admin</h1>
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(!validate_auth("test.zyl", &program, &schema));
}

#[test]
fn rejects_scoped_crud_permissions_without_auth_definition() {
    let source = r#"
        table customers {
            id: Id primary auto
            name: String(100) required
        }

        crud Customer -> customers {
            permits create "customers.create"
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(!validate_auth("test.zyl", &program, &schema));
}

#[test]
fn rejects_protected_form_action_without_auth_definition() {
    let source = r#"
        form CustomerForm {
            field name: String {
                required
            }
            action save {
                requires auth
                permits "customers.save"
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(!validate_auth("test.zyl", &program, &schema));
}

#[test]
fn rejects_protected_api_without_auth_definition() {
    let source = r#"
        api GET "/admin" {
            requires auth
            output String
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(!validate_auth("test.zyl", &program, &schema));
}

#[test]
fn accepts_persistent_auth_tables() {
    let source = r#"
        auth users {
            table: users
            sessions: auth_sessions
            permissions: user_permissions
            roles: user_roles
            role_permissions: role_permissions
        }

        table users {
            id: Id primary auto
            email: Email required
            password_hash: String(255) required
        }

        table auth_sessions {
            id: Id primary auto
            user: User required
            token_hash: String(64) required
            expires_at: Timestamp required
        }

        table user_permissions {
            id: Id primary auto
            user: User required
            permission: String(100) required
        }

        table user_roles {
            id: Id primary auto
            user: User required
            role: String(100) required
        }

        table role_permissions {
            id: Id primary auto
            role: String(100) required
            permission: String(100) required
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(validate_auth("test.zyl", &program, &schema));
}

#[test]
fn accepts_password_reset_schema_with_audit_and_single_use_token_storage() {
    let source = r#"
        auth users {
            table: users
            reset_tokens: password_resets
            audit: auth_audit_log
        }

        table users {
            id: Id primary auto
            email: Email required unique
            password_hash: String(255) required
        }

        table password_resets {
            id: Id primary auto
            user: User required
            token_hash: String(64) required unique
            expires_at: Timestamp required
            consumed_at: Timestamp?
            delivery_payload: String(2048)
            delivery_retry_at: Timestamp?
        }

        table auth_audit_log {
            id: Id primary auto
            actor_user_id: Int?
            event: String(100) required
            target_user_id: Int?
            details: String(1000) required
            created_at: Timestamp default now
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    let mut schema = build_schema(&program).unwrap();
    assert!(validate_auth("test.zyl", &program, &schema));
    {
        let column = schema
            .tables
            .iter_mut()
            .find(|table| table.name == "password_resets")
            .unwrap()
            .columns
            .iter_mut()
            .find(|column| column.name == "delivery_retry_at")
            .unwrap();
        column.nullable = false;
        column.sql_type = "bigint".into();
    }
    assert!(!validate_auth("test.zyl", &program, &schema));
    {
        let column = schema
            .tables
            .iter_mut()
            .find(|table| table.name == "password_resets")
            .unwrap()
            .columns
            .iter_mut()
            .find(|column| column.name == "delivery_retry_at")
            .unwrap();
        column.nullable = true;
        column.sql_type = "timestamp".into();
    }
    {
        let column = schema
            .tables
            .iter_mut()
            .find(|table| table.name == "password_resets")
            .unwrap()
            .columns
            .iter_mut()
            .find(|column| column.name == "delivery_payload")
            .unwrap();
        column.nullable = false;
        column.sql_type = "varchar(1024)".into();
    }
    assert!(!validate_auth("test.zyl", &program, &schema));
    schema
        .tables
        .iter_mut()
        .find(|table| table.name == "password_resets")
        .unwrap()
        .columns
        .retain(|column| column.name != "delivery_payload");
    assert!(!validate_auth("test.zyl", &program, &schema));
}

#[test]
fn rejects_partial_auth_role_configuration() {
    let source = r#"
        auth users {
            table: users
            roles: user_roles
        }

        table users {
            id: Id primary auto
            email: Email required
            password_hash: String(255) required
        }

        table user_roles {
            id: Id primary auto
            user: User required
            role: String(100) required
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(!validate_auth("test.zyl", &program, &schema));
}

#[test]
fn rejects_chained_audit_without_hash_columns() {
    let source = r#"
        auth users {
            table: users
            audit: audit_log
            audit_chain: true
        }

        table users {
            id: Id primary auto
            email: Email required
            password_hash: String(255) required
        }

        table audit_log {
            id: Id primary auto
            actor_user_id: Int?
            event: String(100) required
            target_user_id: Int?
            details: String(1000) required
            created_at: Timestamp default now
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(!validate_auth("test.zyl", &program, &schema));
}

#[test]
fn reads_project_capability_grants() {
    let grants = project_capability_grants("../examples/capabilities.zyl")
        .unwrap()
        .unwrap();
    assert!(grants.contains("Database"));
    assert!(grants.contains("Network"));
    assert!(grants.contains("Console"));
}

#[test]
fn reads_scoped_database_capability_grants() {
    let directory = env::temp_dir().join(format!(
        "zelyra-scoped-capability-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("zelyra.toml"),
        "[capabilities]\ndatabase_read = true\n",
    )
    .unwrap();
    fs::write(directory.join("main.zyl"), "fn main() { }").unwrap();

    let grants = project_capability_grants(directory.join("main.zyl").to_str().unwrap())
        .unwrap()
        .unwrap();
    assert!(grants.contains("Database(read)"));
    assert!(!grants.contains("Database(write)"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn defaults_project_file_system_policy_to_project_root() {
    let policy = project_filesystem_policy("../examples/filesystem_api.zyl")
        .unwrap()
        .unwrap();
    let project_root = fs::canonicalize("..").unwrap();
    assert!(policy.read_roots.iter().any(|root| root == &project_root));
    assert!(policy.write_roots.is_empty());
}

#[test]
fn defaults_project_network_policy_to_no_allowed_hosts() {
    let policy = project_network_policy("../examples/filesystem_api.zyl")
        .unwrap()
        .unwrap();
    assert!(policy.allowed_hosts.is_empty());
    assert_eq!(policy.timeout_ms, 5_000);
    assert_eq!(policy.max_response_bytes, 1_048_576);
}

#[test]
fn defaults_project_process_policy_to_no_allowed_commands() {
    let policy = project_process_policy("../examples/filesystem_api.zyl")
        .unwrap()
        .unwrap();
    assert!(policy.allowed_commands.is_empty());
    assert_eq!(policy.timeout_ms, 5_000);
    assert_eq!(policy.max_output_bytes, 1_048_576);
}

#[test]
fn generates_project_with_target_release_version() {
    let path = env::temp_dir().join(format!(
        "zelyra-cli-template-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let status = create_project(
        path.to_str().unwrap(),
        ProjectOptions {
            allow_current_directory: false,
            with_mariadb: true,
            crud_template: false,
            auth_template: false,
            business_template: false,
            web_port: DEFAULT_WEB_PORT,
            host_port: DEFAULT_WEB_PORT,
            database_host_port: DEFAULT_DATABASE_HOST_PORT,
            host_port_given: false,
            database_host_port_given: false,
        },
    );
    assert_eq!(status, ExitCode::SUCCESS);

    let dockerfile = fs::read_to_string(path.join("Dockerfile")).unwrap();
    let project_config = fs::read_to_string(path.join("zelyra.toml")).unwrap();
    let project_theme = fs::read_to_string(path.join(PROJECT_THEME_CSS_FILE)).unwrap();
    assert!(dockerfile.contains(&format!("ARG ZELYRA_REF=v{}", env!("CARGO_PKG_VERSION"))));
    assert!(dockerfile.contains("git -C /zelyra fetch --depth=1 origin \"$ZELYRA_REF\""));
    assert!(dockerfile.contains("git -C /zelyra checkout --detach FETCH_HEAD"));
    assert!(!dockerfile.contains("git clone --depth 1 --branch"));
    assert!(project_config.contains(&format!("version = \"{}\"", env!("CARGO_PKG_VERSION"))));
    assert!(project_config.contains("console = false"));
    assert!(dockerfile.contains("COPY main.zyl zelyra.toml zelyra.theme.css ./"));
    assert!(dockerfile.contains("COPY src ./src"));
    assert!(path.join("src/.keep").is_file());
    assert!(dockerfile.contains("COPY locales ./locales"));
    assert!(path.join("locales/de.json").is_file());
    assert!(path.join("locales/en.json").is_file());
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
            project_theme.contains(token),
            "generated theme misses {token}"
        );
    }

    fs::remove_dir_all(path).unwrap();
}

#[test]
fn mariadb_crud_scaffold_includes_the_fictional_demo_fixture() {
    let path = env::temp_dir().join(format!(
        "zelyra-cli-crud-demo-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let status = create_project(
        path.to_str().unwrap(),
        ProjectOptions {
            allow_current_directory: false,
            with_mariadb: true,
            crud_template: true,
            auth_template: false,
            business_template: false,
            web_port: DEFAULT_WEB_PORT,
            host_port: DEFAULT_WEB_PORT,
            database_host_port: DEFAULT_DATABASE_HOST_PORT,
            host_port_given: false,
            database_host_port_given: false,
        },
    );
    assert_eq!(status, ExitCode::SUCCESS);

    let fixture = fs::read_to_string(path.join("machine-management-demo.sql")).unwrap();
    assert!(fixture.contains("ZLY-DEMO-030"));
    assert!(fixture.contains("INSERT IGNORE INTO machines"));
    let source = fs::read_to_string(path.join("main.zyl")).unwrap();
    assert!(source.contains("machines.resources.title"));
    assert!(source.contains("mode: cards"));

    fs::remove_dir_all(path).unwrap();
}

#[test]
fn generated_mariadb_template_uses_selected_web_port() {
    let path = env::temp_dir().join(format!(
        "zelyra-cli-port-template-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut selected_ports = None;
    for _ in 0..8 {
        let database_host_port = find_free_port(34_000, &[]).unwrap();
        let host_port = find_free_port(35_000, &[database_host_port]).unwrap();
        let status = create_project(
            path.to_str().unwrap(),
            ProjectOptions {
                allow_current_directory: false,
                with_mariadb: true,
                crud_template: false,
                auth_template: false,
                business_template: false,
                web_port: 8080,
                host_port,
                database_host_port,
                host_port_given: true,
                database_host_port_given: true,
            },
        );
        if status == ExitCode::SUCCESS {
            selected_ports = Some((host_port, database_host_port));
            break;
        }
    }
    let (host_port, database_host_port) =
        selected_ports.expect("template test should acquire two free host ports");

    let env_example = fs::read_to_string(path.join(".env.example")).unwrap();
    let compose = fs::read_to_string(path.join("docker-compose.mariadb.yml")).unwrap();
    let dockerfile = fs::read_to_string(path.join("Dockerfile")).unwrap();
    let dockerignore = fs::read_to_string(path.join(".dockerignore")).unwrap();
    let gitignore = fs::read_to_string(path.join(".gitignore")).unwrap();
    assert!(env_example.contains("ZELYRA_WEB_PORT=8080"));
    assert!(env_example.contains(&format!("ZELYRA_HOST_PORT={host_port}")));
    assert!(env_example.contains(&format!("ZELYRA_DB_HOST_PORT={database_host_port}")));
    assert!(env_example.contains("ZELYRA_DB_CONNECT_TIMEOUT_SECS=10"));
    assert!(env_example.contains("ZELYRA_DB_QUERY_TIMEOUT_SECS=30"));
    assert!(env_example.contains("ZELYRA_DB_POOL_MAX_SIZE=8"));
    assert!(env_example.contains("ZELYRA_DB_POOL_WAIT_TIMEOUT_SECS=10"));
    assert!(env_example.contains("ZELYRA_DB_TLS_MODE=disabled"));
    assert!(
        compose.contains("ZELYRA_DB_CONNECT_TIMEOUT_SECS: ${ZELYRA_DB_CONNECT_TIMEOUT_SECS:-10}")
    );
    assert!(compose.contains("ZELYRA_DB_QUERY_TIMEOUT_SECS: ${ZELYRA_DB_QUERY_TIMEOUT_SECS:-30}"));
    assert!(compose.contains("ZELYRA_DB_POOL_MAX_SIZE: ${ZELYRA_DB_POOL_MAX_SIZE:-8}"));
    assert!(compose
        .contains("ZELYRA_DB_POOL_WAIT_TIMEOUT_SECS: ${ZELYRA_DB_POOL_WAIT_TIMEOUT_SECS:-10}"));
    assert!(compose.contains("ZELYRA_DB_TLS_MODE: ${ZELYRA_DB_TLS_MODE:-disabled}"));
    assert!(compose.contains("ZELYRA_DB_TLS_CA_CERT_FILE: ${ZELYRA_DB_TLS_CA_CERT_FILE:-}"));
    assert!(compose.contains("0.0.0.0:${ZELYRA_WEB_PORT:-8080}"));
    assert!(compose.contains(&format!(
        "127.0.0.1:${{ZELYRA_HOST_PORT:-{host_port}}}:${{ZELYRA_WEB_PORT:-8080}}"
    )));
    assert!(compose.contains(&format!(
        "127.0.0.1:${{ZELYRA_DB_HOST_PORT:-{database_host_port}}}:3306"
    )));
    assert!(compose.contains("0.0.0.0:${ZELYRA_WEB_PORT:-8080}"));
    assert!(dockerfile.contains("EXPOSE 8080"));
    assert!(dockerfile.contains("0.0.0.0:8080"));
    assert!(dockerignore.contains(".env"));
    assert!(dockerignore.contains(".env.*"));
    assert!(gitignore.contains(".env"));

    fs::remove_dir_all(path).unwrap();
}

#[test]
fn setup_accepts_mariadb_project_without_env_example() {
    let path = env::temp_dir().join(format!(
        "zelyra-cli-setup-config-only-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    fs::write(
        path.join("zelyra.toml"),
        "[database.main]\nengine = \"mariadb\"\n",
    )
    .unwrap();

    assert_eq!(
        setup_project(path.to_str().unwrap(), &SetupOptions::default()),
        ExitCode::SUCCESS
    );
    let env_file = fs::read_to_string(path.join(".env")).unwrap();
    assert!(env_file.contains("ZELYRA_DATABASE_MAIN_URL=mariadb://zelyra:"));
    assert!(env_file.contains("MARIADB_ROOT_PASSWORD="));

    fs::remove_dir_all(path).unwrap();
}

#[test]
fn setup_suggests_relative_path_when_absolute_path_is_probably_mistyped() {
    let leaf = format!(
        "zelyra-setup-path-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let current = env::temp_dir().join(format!("{leaf}-cwd"));
    let candidate = current.join(&leaf);
    fs::create_dir_all(&candidate).unwrap();
    let absolute = format!("/{leaf}");

    let error = setup_directory_from(&absolute, Some(&current)).unwrap_err();
    assert!(error.contains(&format!("`./{leaf}` exists in the current directory")));
    assert!(error.contains(&format!("zelyra setup ./{leaf}")));

    fs::remove_dir_all(current).unwrap();
}

#[test]
fn setup_does_not_create_mariadb_credentials_for_minimal_init_project() {
    let path = env::temp_dir().join(format!(
        "zelyra-cli-setup-minimal-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("zelyra.toml"), "[project]\nname = \"minimal\"\n").unwrap();

    let error = match ensure_local_env_file(&path, &SetupOptions::default()) {
        Ok(_) => panic!("minimal projects must not get MariaDB credentials"),
        Err(error) => error,
    };
    assert!(error.contains("A project created with `zelyra init` runs without setup"));
    assert!(error.contains("zelyra init <directory> --mariadb"));
    assert!(!path.join(".env").exists());

    fs::remove_dir_all(path).unwrap();
}

#[test]
fn setup_action_prepare_is_idempotent_and_does_not_replace_credentials() {
    let path = env::temp_dir().join(format!(
        "zelyra-cli-setup-action-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    fs::write(
        path.join("zelyra.toml"),
        "[database.main]\nengine = \"mariadb\"\n",
    )
    .unwrap();

    let first = setup_action(path.to_str().unwrap(), "prepare", &SetupOptions::default()).unwrap();
    let contents = fs::read_to_string(path.join(".env")).unwrap();
    let second = setup_action(path.to_str().unwrap(), "prepare", &SetupOptions::default()).unwrap();
    assert!(first.contains("created protected .env"));
    assert!(second.contains("kept existing .env"));
    assert_eq!(contents, fs::read_to_string(path.join(".env")).unwrap());

    fs::remove_dir_all(path).unwrap();
}

#[test]
fn setup_web_url_uses_the_effective_host_port() {
    let path = env::temp_dir().join(format!(
        "zelyra-cli-setup-web-url-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    fs::write(
        path.join("zelyra.toml"),
        "[database.main]\nengine = \"mariadb\"\n",
    )
    .unwrap();
    fs::write(
        path.join(".env.example"),
        "# ZELYRA_HOST_PORT=18080\nZELYRA_DB_HOST_PORT=3308\n",
    )
    .unwrap();
    fs::write(path.join(".env"), "ZELYRA_HOST_PORT=18443\n").unwrap();

    let env_path = path.join(".env");
    let env_path = env_path.to_string_lossy();
    assert_eq!(
        project_web_url_with_host_port(&path, None, &env_path).unwrap(),
        "http://127.0.0.1:18443"
    );

    fs::write(path.join(".env"), "ZELYRA_DB_HOST_PORT=3308\n").unwrap();
    assert_eq!(
        project_web_url_with_host_port(&path, None, &env_path).unwrap(),
        "http://127.0.0.1:18080"
    );

    assert_eq!(
        project_web_url_with_host_port(&path, Some("18444"), "unused.env").unwrap(),
        "http://127.0.0.1:18444"
    );

    fs::remove_dir_all(path).unwrap();
}

#[test]
fn setup_web_requires_the_token_and_renders_the_local_actions() {
    let path = env::temp_dir().join(format!(
        "zelyra-cli-setup-web-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    let state = Arc::new(Mutex::new(SetupWebState {
        directory: path.clone(),
        token: "test-token".into(),
        message: String::new(),
    }));
    let request = |target: &str| zelyra_web::Request {
        method: "GET".into(),
        target: target.into(),
        path: "/".into(),
        headers: HashMap::new(),
        body: String::new(),
        remote_addr: None,
    };

    let forbidden = setup_web_response(&state, &request("/?token=wrong"));
    assert_eq!(forbidden.status, 403);
    let page = setup_web_response(&state, &request("/?token=test-token"));
    assert_eq!(page.status, 200);
    assert!(page.body.contains("Konfiguration vorbereiten"));
    assert!(page.body.contains("MariaDB und Anwendung starten"));

    fs::remove_dir_all(path).unwrap();
}

#[test]
fn web_port_validation_rejects_zero_and_non_numeric_values() {
    assert_eq!(parse_web_port("1"), Ok(1));
    assert_eq!(parse_web_port("65535"), Ok(65535));
    assert!(parse_web_port("0").is_err());
    assert!(parse_web_port("65536").is_err());
    assert!(parse_web_port("web").is_err());
    assert_eq!(parse_database_host_port("3308"), Ok(3308));
    assert!(parse_database_host_port("0").is_err());
    assert!(parse_database_host_port("database").is_err());
}

#[test]
fn docker_compose_hint_is_actionable_and_platform_specific() {
    let hint = docker_compose_install_hint();
    assert!(hint.contains("Docker Compose is unavailable"));
    assert!(hint.contains("https://docs.docker.com/"));
    assert!(hint.contains("docker compose version"));
}

#[test]
fn compose_start_errors_are_actionable_without_exposing_output() {
    let secret = "mariadb://zelyra:secret-value@127.0.0.1:3306/zelyra_app";
    let permission = compose_start_failure_message(
        DockerComposeCommand::Legacy,
        &format!("permission denied while connecting to docker.sock: {secret}"),
    );
    assert!(permission.contains("usermod -aG docker"));
    assert!(permission.contains("\n  newgrp docker\n  id -nG\n  docker ps\n"));
    assert!(permission.contains("Opening another terminal window alone"));
    assert!(!permission.contains(secret));

    let conflict = compose_start_failure_message(
        DockerComposeCommand::Plugin,
        "failed to bind host port: address already in use",
    );
    assert!(conflict.contains("port is already in use"));
    assert!(!conflict.contains("failed to bind"));
}

#[test]
fn free_port_selection_skips_a_bound_port() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let occupied = listener.local_addr().unwrap().port();
    let selected = find_free_port(occupied, &[]).unwrap();
    assert_ne!(selected, occupied);
}

#[test]
fn explicit_port_conflicts_are_rejected() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let occupied = listener.local_addr().unwrap().port();
    let error = resolve_host_port(occupied, true, "web host", true, &[])
        .expect_err("an explicitly occupied port must be rejected");
    assert!(error.contains("already in use"));
    assert!(error.contains("choose a different port"));
}
