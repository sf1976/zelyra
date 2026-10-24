use super::*;

#[test]
fn generated_crud_maps_file_ref_attachment_metadata_to_upload_fields() {
    let program = parse(
        &lex(r#"
            table contracts {
                id: Id primary auto
                signed_pdf: FileRef? attachment {
                    max_size: "20MB"
                    allowed_types: ["application/pdf"]
                }
            }
            crud Contract -> contracts {}
            fn main() {}
            "#)
        .unwrap(),
    )
    .unwrap();
    let schema = zelyra_database::build_schema(&program).unwrap();
    let route = generated_crud_form(
        &program,
        &program.cruds[0],
        &program.tables[0],
        &schema,
        false,
        CrudGenerationContext {
            layout_html: None,
            before_form_html: None,
            after_form_html: None,
            field_content_html: HashMap::new(),
            csrf: CsrfProtection::generate().unwrap(),
            membership_table: None,
            audit_table: None,
            audit_chain: false,
        },
    );
    let field = route
        .form
        .fields
        .iter()
        .find(|field| field.name == "signed_pdf")
        .expect("attachment should appear in the generated form");
    assert_eq!(field.ty, Some(Type::Named("File".into())));
    assert!(!field.required);
    assert_eq!(
        field
            .file_upload
            .as_ref()
            .map(|options| options.max_size_bytes),
        Some(20_000_000)
    );
    assert_eq!(
        field.file_upload.as_ref().unwrap().allowed_types,
        ["application/pdf"]
    );
}

#[test]
fn database_web_gate_accepts_any_explicit_database_operation_grant() {
    assert!(database_runtime_capability_granted(None));
    assert!(!database_runtime_capability_granted(Some(&HashSet::new())));
    assert!(database_runtime_capability_granted(Some(&HashSet::from([
        "Database(read)".to_owned(),
    ]))));
    assert!(database_runtime_capability_granted(Some(&HashSet::from([
        "Database(write)".to_owned(),
    ]))));
}

#[test]
fn typed_crud_action_bodies_are_checked_in_row_and_session_scope() {
    let directory = env::temp_dir().join(format!(
        "zelyra-typed-crud-action-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let entry = directory.join("main.zyl");
    fs::write(&entry, "fn main() {}\n").unwrap();
    fs::write(
        directory.join("zelyra.toml"),
        "[capabilities]\ndatabase_write = true\n",
    )
    .unwrap();
    let entry = entry.to_string_lossy().into_owned();
    let source = r#"
        table invoices { id: Id primary auto is_locked: Bool required }
        crud Invoice -> invoices {
            action lock {
                uses Database(write)
                guard when row.is_locked == false { error: "Already locked." }
                execute {
                    previous_lock_state = row.is_locked
                    active_tenant = session.tenant_id
                    sql { UPDATE invoices SET is_locked = true WHERE id = :row.id }
                }
            }
        }
        fn main() {}
    "#;
    let program = zelyra_parser::parse(&zelyra_lexer::lex(source).unwrap()).unwrap();
    assert!(validate_typed_crud_actions(&entry, &program));

    let invalid = r#"
        table invoices { id: Id primary auto is_locked: Bool required }
        crud Invoice -> invoices {
            action lock {
                guard when row.missing_field == false { error: "Already locked." }
                execute { previous_lock_state = row.is_locked }
            }
        }
        fn main() {}
    "#;
    let program = zelyra_parser::parse(&zelyra_lexer::lex(invalid).unwrap()).unwrap();
    assert!(!validate_typed_crud_actions(&entry, &program));

    let standalone = zelyra_parser::parse(
        &zelyra_lexer::lex("form Example { action run { execute { value = 1 } } } fn main() {}")
            .unwrap(),
    )
    .unwrap();
    assert!(!validate_typed_crud_actions(&entry, &standalone));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn standalone_form_handlers_require_matching_typed_fields_and_effects() {
    let directory = env::temp_dir().join(format!(
        "zelyra-standalone-form-handler-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let entry = directory.join("main.zyl").to_string_lossy().into_owned();
    let valid = r#"
        fn generate_password(length: Int) -> String uses Random {
            return "generated"
        }
        form PasswordGenerator {
            field length: Int { required min_value: "8" max_value: "64" }
            action generate {
                handler: generate_password
                uses Random
            }
        }
        fn main() {}
    "#;
    let program = zelyra_parser::parse(&zelyra_lexer::lex(valid).unwrap()).unwrap();
    assert!(validate_typed_crud_actions(&entry, &program));

    let authorized = valid.replace(
        "                handler: generate_password",
        "                requires auth\n                permits \"password_generator.use\"\n                handler: generate_password",
    );
    let program = zelyra_parser::parse(&zelyra_lexer::lex(&authorized).unwrap()).unwrap();
    assert!(validate_typed_crud_actions(&entry, &program));

    let invalid_signature = valid.replace("field length: Int", "field size: Int");
    let program = zelyra_parser::parse(&zelyra_lexer::lex(&invalid_signature).unwrap()).unwrap();
    assert!(!validate_typed_crud_actions(&entry, &program));

    let missing_effect = valid.replace("                uses Random\n", "");
    let program = zelyra_parser::parse(&zelyra_lexer::lex(&missing_effect).unwrap()).unwrap();
    assert!(!validate_typed_crud_actions(&entry, &program));

    let table_backed = valid.replace(
        "form PasswordGenerator {",
        "table generated { id: Id primary auto length: Int required }\n        form PasswordGenerator -> generated {",
    );
    let program = zelyra_parser::parse(&zelyra_lexer::lex(&table_backed).unwrap()).unwrap();
    assert!(!validate_typed_crud_actions(&entry, &program));

    let unsupported_confirmation = valid.replace(
        "handler: generate_password",
        "handler: generate_password\n                confirm: \"Continue?\"",
    );
    let program =
        zelyra_parser::parse(&zelyra_lexer::lex(&unsupported_confirmation).unwrap()).unwrap();
    assert!(!validate_typed_crud_actions(&entry, &program));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn typed_crud_action_database_capability_requires_project_grant() {
    let directory = env::temp_dir().join(format!(
        "zelyra-typed-crud-action-capability-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let entry = directory.join("main.zyl");
    fs::write(&entry, "fn main() {}\n").unwrap();
    fs::write(
        directory.join("zelyra.toml"),
        "[capabilities]\ndatabase_write = false\n",
    )
    .unwrap();
    let entry = entry.to_string_lossy().into_owned();
    let source = r#"
        table invoices { id: Id primary auto }
        crud Invoice -> invoices {
            action mark {
                uses Database(write)
                execute { sql { UPDATE invoices SET id = :row.id } }
            }
        }
        fn main() {}
    "#;
    let program = zelyra_parser::parse(&zelyra_lexer::lex(source).unwrap()).unwrap();
    assert!(!validate_typed_crud_actions(&entry, &program));

    fs::write(
        directory.join("zelyra.toml"),
        "[capabilities]\ndatabase_read = true\ndatabase_write = false\n",
    )
    .unwrap();
    let read_only = r#"
        table invoices { id: Id primary auto }
        crud Invoice -> invoices {
            action mark {
                uses Database(read)
                execute { sql { UPDATE invoices SET id = :row.id } }
            }
        }
        fn main() {}
    "#;
    let program = zelyra_parser::parse(&zelyra_lexer::lex(read_only).unwrap()).unwrap();
    assert!(!validate_typed_crud_actions(&entry, &program));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn compiler_scope_gate_rejects_sql_touching_declared_tenant_tables() {
    let source = r#"
        type TenantId = Id
        table invoices scoped by tenant_id: TenantId {
            id: Id primary auto
            tenant_id: TenantId required
        }
        table customers { id: Id primary auto }
        fn read_invoice() uses Database(read) {
            rows = sql<Int[]> { SELECT id FROM invoices }
        }
        fn main() {}
    "#;
    let program = zelyra_parser::parse(&zelyra_lexer::lex(source).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(
        !validate_tenant_access_boundaries("main.zyl", &program, &schema),
        "raw SQL touching a scoped table must fail the compiler gate"
    );

    let unscoped_source = r#"
        type TenantId = Id
        table invoices scoped by tenant_id: TenantId {
            id: Id primary auto
            tenant_id: TenantId required
        }
        table customers { id: Id primary auto }
        fn read_customer() uses Database(read) {
            rows = sql<Int[]> { SELECT id FROM customers }
        }
        fn main() {}
    "#;
    let program = zelyra_parser::parse(&zelyra_lexer::lex(unscoped_source).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(validate_tenant_access_boundaries(
        "main.zyl", &program, &schema
    ));
}

#[test]
fn compiler_accepts_only_tightly_scoped_public_token_forms() {
    let valid = r#"
        type TenantId = Id
        table tenants { id: Id primary auto }
        table suppressions scoped by tenant_id: TenantId {
            id: Id primary auto
            tenant_id: TenantId required references tenants
            token_hash: String(64) required unique
            unsubscribed_at: Timestamp?
        }
        form PublicUnsubscribe -> suppressions {
            route: "/unsubscribe/{token}"
            public_token {
                hash_field: token_hash
                timestamp_field: unsubscribed_at
            }
            action unsubscribe {
                require_match
                label: "Unsubscribe"
                confirm_page {
                    title: "Confirm"
                    message: "Stop these messages?"
                    submit: "Unsubscribe"
                }
                success "Saved"
                redirect "/unsubscribe/done"
            }
        }
        fn main() {}
    "#;
    let program = zelyra_parser::parse(&zelyra_lexer::lex(valid).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(validate_tenant_access_boundaries(
        "main.zyl", &program, &schema
    ));

    for invalid in [
        valid.replace("token_hash: String(64) required unique", "token_hash: String(64) required"),
        valid.replace("Timestamp?", "String?"),
        valid.replace("route: \"/unsubscribe/{token}\"", "route: \"/unsubscribe/{token}/{other}\""),
        valid.replace(
            "success \"Saved\"",
            "sql { UPDATE suppressions SET tenant_id = :tenant_id }\n                success \"Saved\"",
        ),
    ] {
        let program = zelyra_parser::parse(&zelyra_lexer::lex(&invalid).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        assert!(
            !validate_tenant_access_boundaries("main.zyl", &program, &schema),
            "invalid public-token form passed compiler scope validation: {invalid}"
        );
    }
}

#[test]
fn compiler_scope_gate_rejects_tableviews_reading_tenant_tables() {
    let source = r#"
        type TenantId = Id
        table invoices scoped by tenant_id: TenantId {
            id: Id primary auto
            tenant_id: TenantId required
            amount: Decimal required
        }
        table customers { id: Id primary auto }
        struct InvoiceSummary {
            id: Id
            amount: Decimal
        }
        tableview InvoiceSummaryView {
            source sql<InvoiceSummary[]> {
                SELECT i.id, i.amount
                FROM invoices AS i
                JOIN customers AS c ON c.id = i.id
            }
            columns { id amount }
        }
        fn main() {}
    "#;
    let program = zelyra_parser::parse(&zelyra_lexer::lex(source).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();

    assert!(
        !validate_tenant_access_boundaries("main.zyl", &program, &schema),
        "tableview SQL must not bypass tenant-scoped CRUD boundaries"
    );

    let unscoped_tableview = r#"
        type TenantId = Id
        table invoices scoped by tenant_id: TenantId {
            id: Id primary auto
            tenant_id: TenantId required
        }
        table customers {
            id: Id primary auto
            name: String(100) required
        }
        struct CustomerSummary { id: Id name: String }
        tableview CustomerSummaryView {
            source sql<CustomerSummary[]> {
                SELECT id, name FROM customers
            }
            columns { id name }
        }
        fn main() {}
    "#;
    let program = zelyra_parser::parse(&zelyra_lexer::lex(unscoped_tableview).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(
        validate_tenant_access_boundaries("main.zyl", &program, &schema),
        "unscoped tableviews should remain available when they read unprotected tables"
    );
}

#[test]
fn project_mail_policy_reads_local_smtp_settings_and_redacts_secrets() {
    let directory = std::env::temp_dir().join(format!(
        "zelyra-mail-policy-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let ca_certificate = include_bytes!("../../tests/fixtures/smtp-test-ca.pem");
    fs::write(directory.join("smtp-ca.pem"), ca_certificate).unwrap();
    fs::write(
        directory.join(".env"),
        "ZELYRA_SMTP_HOST=mail.example.test\nZELYRA_SMTP_SECURITY=starttls\nZELYRA_SMTP_FROM=Zelyra <noreply@example.test>\nZELYRA_SMTP_USERNAME=mail-user\nZELYRA_SMTP_PASSWORD=mail-secret-value\nZELYRA_SMTP_CA_CERT=smtp-ca.pem\n",
    )
    .unwrap();
    let entry = directory.join("main.zyl");
    fs::write(&entry, "fn main() {}\n").unwrap();
    let policy = project_mail_policy(entry.to_str().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(policy.host.as_deref(), Some("mail.example.test"));
    assert_eq!(policy.port, Some(587));
    assert_eq!(policy.security.as_deref(), Some("starttls"));
    assert_eq!(
        policy.ca_certificate_pem.as_deref(),
        Some(ca_certificate.as_slice())
    );
    assert!(format!("{policy:?}").contains("<redacted>"));
    assert!(!format!("{policy:?}").contains("mail-secret-value"));
    fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn smtp_ca_configuration_rejects_symlink_files() {
    use std::os::unix::fs::symlink;

    let directory = std::env::temp_dir().join(format!(
        "zelyra-smtp-ca-symlink-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let entry = directory.join("main.zyl");
    fs::write(&entry, "fn main() {}\n").unwrap();
    fs::write(
        directory.join("ca.pem"),
        include_bytes!("../../tests/fixtures/smtp-test-ca.pem"),
    )
    .unwrap();
    symlink("ca.pem", directory.join("ca-link.pem")).unwrap();
    let error = project_smtp_ca_certificate(entry.to_str().unwrap(), "ca-link.pem").unwrap_err();
    assert!(error.contains("regular PEM file"));
    fs::remove_dir_all(directory).unwrap();
}

fn migration_fixture_schema(columns: Vec<zelyra_database::Column>) -> Schema {
    Schema {
        database: Some(zelyra_database::DatabaseConfig {
            name: "main".into(),
            engine: "mariadb".into(),
            database: Some("zelyra_test".into()),
        }),
        tables: vec![zelyra_database::Table {
            name: "customers".into(),
            tenant_scope_column: None,
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
                tenant_scope_column: None,
                columns: vec![migration_fixture_column("id", "BIGINT", false)],
                foreign_keys: vec![zelyra_database::ForeignKey {
                    name: None,
                    column: "group_id".into(),
                    columns: Vec::new(),
                    referenced_table: "customer_groups".into(),
                    referenced_column: "id".into(),
                    referenced_columns: Vec::new(),
                    on_delete: zelyra_ast::ReferentialAction::NoAction,
                }],
                indexes: Vec::new(),
                uniques: Vec::new(),
            },
            zelyra_database::Table {
                name: "invoices".into(),
                tenant_scope_column: None,
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
                tenant_scope_column: None,
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
            columns: Vec::new(),
            referenced_table: "accounts".into(),
            referenced_column: "id".into(),
            referenced_columns: Vec::new(),
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
    assert_eq!(first["preflights"][2]["columns"][0], "account_id");
    assert_eq!(first["preflights"][2]["referenced_columns"][0], "id");
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
fn account_commands_resolve_database_url_from_project_env_file() {
    let _environment = TEST_ENVIRONMENT_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let old_database_url = env::var_os("DATABASE_URL");
    let old_named_url = env::var_os("ZELYRA_DATABASE_MAIN_URL");
    env::remove_var("DATABASE_URL");
    env::remove_var("ZELYRA_DATABASE_MAIN_URL");

    let directory = std::env::temp_dir().join(format!(
        "zelyra-account-env-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("main.zyl"),
        "database main { engine: mariadb }\n",
    )
    .unwrap();
    fs::write(
        directory.join(".env"),
        "ZELYRA_DATABASE_MAIN_URL=mariadb://project:secret@127.0.0.1:3306/app\n",
    )
    .unwrap();
    let source = fs::read_to_string(directory.join("main.zyl")).unwrap();
    let program = parse(&lex(&source).unwrap()).unwrap();
    assert_eq!(
        account_cli::project_database_url(directory.join("main.zyl").to_str().unwrap(), &program)
            .unwrap(),
        Some("mariadb://project:secret@127.0.0.1:3306/app".into())
    );
    fs::remove_dir_all(directory).unwrap();

    if let Some(value) = old_database_url {
        env::set_var("DATABASE_URL", value);
    }
    if let Some(value) = old_named_url {
        env::set_var("ZELYRA_DATABASE_MAIN_URL", value);
    }
}

#[test]
fn project_database_url_expands_the_configured_local_host_port() {
    assert_eq!(
        expand_database_host_port(
            "mariadb://zelyra:secret@127.0.0.1:${ZELYRA_DB_HOST_PORT:-3306}/invoice",
            "43127",
        ),
        "mariadb://zelyra:secret@127.0.0.1:43127/invoice"
    );
    assert_eq!(
        expand_database_host_port(
            "mariadb://zelyra:secret@127.0.0.1:${ZELYRA_DB_HOST_PORT}/invoice",
            "43127",
        ),
        "mariadb://zelyra:secret@127.0.0.1:43127/invoice"
    );
}

#[test]
fn compose_port_conflicts_identify_database_or_application_ports() {
    assert_eq!(
        compose_conflict_port(
            "Error: listen tcp4 127.0.0.1:3307: bind: address already in use",
            3307,
            3000,
        ),
        Some(("ZELYRA_DB_HOST_PORT", 3307))
    );
    assert_eq!(
        compose_conflict_port(
            "Ports are not available: exposing TCP 127.0.0.1:3000: port is already allocated",
            3307,
            3000,
        ),
        Some(("ZELYRA_HOST_PORT", 3000))
    );
    assert_eq!(compose_conflict_port("image pull failed", 3307, 3000), None);
}

#[test]
fn database_port_fallback_keeps_local_urls_in_sync_and_preserves_external_urls() {
    let contents = "ZELYRA_DB_HOST_PORT=3307\nZELYRA_DATABASE_MAIN_URL=mariadb://user:secret@127.0.0.1:3307/invoice\nDATABASE_URL=mariadb://user:secret@127.0.0.1:3307/invoice\nOTHER_DATABASE_URL=mariadb://user:secret@db.example.test:3307/other\n";
    let (updated, found) = update_database_urls_for_host_port(contents, 3307, 43127);
    assert!(found);
    assert!(updated.contains("127.0.0.1:43127/invoice"));
    assert!(updated.contains("db.example.test:3307/other"));
    let (dynamic, found) = update_database_urls_for_host_port(
        "ZELYRA_DATABASE_MAIN_URL=mariadb://user:secret@127.0.0.1:${ZELYRA_DB_HOST_PORT:-3306}/invoice\n",
        3306,
        43127,
    );
    assert!(found);
    assert!(dynamic.contains("${ZELYRA_DB_HOST_PORT:-3306}"));
}

#[test]
fn compose_web_port_fallback_activates_the_selected_project_setting() {
    let updated = replace_template_env_assignment(
        "# ZELYRA_HOST_PORT=3000\nZELYRA_DB_HOST_PORT=3306\n",
        "ZELYRA_HOST_PORT",
        43127,
        true,
    );
    assert!(updated.contains("ZELYRA_HOST_PORT=43127"));
    assert!(!updated.contains("# ZELYRA_HOST_PORT=3000"));
}

#[test]
fn empty_database_host_ports_use_the_compose_fallback() {
    assert_eq!(
        resolve_database_host_port(Some(String::new()), Some("3308".into())),
        "3306"
    );
    assert_eq!(
        resolve_database_host_port(Some("  ".into()), Some("3308".into())),
        "  "
    );
    assert_eq!(
        resolve_database_host_port(Some("3307".into()), Some("3308".into())),
        "3307"
    );
    assert_eq!(
        resolve_database_host_port(None, Some("3308".into())),
        "3308"
    );
}

#[test]
fn resolved_database_urls_do_not_require_reading_the_project_port_file() {
    let directory = std::env::temp_dir().join(format!(
        "zelyra-resolved-db-url-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join(".env"), [0xff, 0xfe]).unwrap();
    let resolved_url = "mariadb://external:secret@db.example.test:3307/invoice";
    assert_eq!(
        expand_project_database_url(&directory, resolved_url).unwrap(),
        resolved_url
    );
    assert!(expand_project_database_url(
        &directory,
        "mariadb://local:secret@127.0.0.1:${ZELYRA_DB_HOST_PORT:-3306}/invoice"
    )
    .is_err());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn unreadable_project_database_file_skips_compose_when_process_url_is_available() {
    let external_url = "mariadb://external:secret@db.example.test:3307/invoice";
    let directory = std::env::temp_dir().join(format!(
        "zelyra-unreadable-compose-url-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join(".env"), [0xff, 0xfe]).unwrap();
    assert_eq!(
        project_file_database_url_for_compose(&directory, Some("main"), true).unwrap(),
        None
    );
    assert!(project_file_database_url_for_compose(&directory, Some("main"), false).is_err());
    fs::remove_dir_all(directory).unwrap();
    assert_eq!(
        project_compose_database_url(Some(external_url), Some(external_url), "3307", true),
        None,
        "process override provenance must win even when URL text matches .env"
    );
}

#[test]
fn studio_compose_target_requires_the_generated_local_mariadb_endpoint() {
    assert!(is_project_mariadb_compose_target(
        "mariadb://zelyra:secret@127.0.0.1:3308/invoice",
        "3308",
    ));
    assert!(is_project_mariadb_compose_target(
        "mysql://zelyra:secret@localhost/invoice",
        "3306",
    ));
    assert!(!is_project_mariadb_compose_target(
        "mariadb://zelyra:secret@db.example.test:3308/invoice",
        "3308",
    ));
    assert!(!is_project_mariadb_compose_target(
        "mariadb://zelyra:secret@127.0.0.1:3306/invoice",
        "3308",
    ));
    assert!(!is_project_mariadb_compose_target(
        "postgres://zelyra:secret@127.0.0.1:3308/invoice",
        "3308",
    ));
}

#[test]
fn studio_compose_bootstrap_rejects_ambient_database_overrides() {
    let project_url = "mariadb://project:secret@127.0.0.1:3308/invoice";
    assert_eq!(
        project_compose_database_url(
            Some("mariadb://external:secret@db.example.test:3308/invoice"),
            Some(project_url),
            "3308",
            true,
        ),
        None
    );
    assert_eq!(
        project_compose_database_url(Some(project_url), Some(project_url), "3308", true),
        None,
        "an environment override must not authorize automatic schema setup even when its URL matches the project file"
    );
    assert_eq!(
        project_compose_database_url(Some(project_url), Some(project_url), "3308", false),
        Some(project_url)
    );
    assert_eq!(
        project_compose_database_url(
            Some("mariadb://project:secret@127.0.0.1:3308/another_database"),
            Some(project_url),
            "3308",
            false,
        ),
        None
    );
}

#[test]
fn project_file_database_url_uses_only_project_environment_configuration() {
    let directory = std::env::temp_dir().join(format!(
        "zelyra-project-db-url-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join(".env"),
        "ZELYRA_DATABASE_MAIN_URL=mariadb://project:secret@127.0.0.1:3308/invoice\nDATABASE_URL=mariadb://fallback:secret@127.0.0.1:3308/fallback\n",
    )
    .unwrap();
    assert_eq!(
        database_url_from_project_file(&directory, Some("main")).unwrap(),
        Some("mariadb://project:secret@127.0.0.1:3308/invoice".into())
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
    let _environment = TEST_ENVIRONMENT_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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
    let _environment = TEST_ENVIRONMENT_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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

    let tabler_theme = "/* zelyra:tabler */\nbody { color: #223; }";
    fs::write(&theme_path, tabler_theme).unwrap();
    let resolved_theme = project_theme_css(source_path.to_str().unwrap())
        .unwrap()
        .expect("project override should be loaded");
    assert!(!resolved_theme.contains("Tabler v1.6.1"));
    assert!(!resolved_theme.contains("zelyra:tabler"));
    assert!(resolved_theme.contains("body { color: #223; }"));
    assert!(resolved_theme.len() < PROJECT_THEME_CSS_MAX_BYTES as usize);

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

#[test]
fn ui_stylesheet_selection_supports_default_tabler_and_safe_project_files() {
    let _environment = TEST_ENVIRONMENT_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let directory = env::temp_dir().join(format!(
        "zelyra-ui-stylesheet-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(directory.join("assets")).unwrap();
    let source_path = directory.join("main.zyl");
    fs::write(&source_path, "fn main() {}\n").unwrap();
    fs::write(
        directory.join("assets/custom.css"),
        "body { color: #123; }\n",
    )
    .unwrap();
    let previous = env::var_os("ZELYRA_UI_STYLESHEET");
    env::remove_var("ZELYRA_UI_STYLESHEET");

    assert_eq!(
        project_ui_stylesheet_css(source_path.to_str().unwrap()).unwrap(),
        None
    );
    fs::write(directory.join(".env"), "ZELYRA_UI_STYLESHEET=tabler\n").unwrap();
    let tabler = project_ui_stylesheet_css(source_path.to_str().unwrap())
        .unwrap()
        .expect("Tabler should be bundled");
    assert!(tabler.contains("Tabler v1.6.1"));
    fs::write(
        directory.join(".env"),
        "ZELYRA_UI_STYLESHEET=assets/custom.css\n",
    )
    .unwrap();
    assert_eq!(
        project_ui_stylesheet_css(source_path.to_str().unwrap()).unwrap(),
        Some("body { color: #123; }\n".into())
    );

    env::set_var("ZELYRA_UI_STYLESHEET", "zelyra");
    assert_eq!(
        project_ui_stylesheet_css(source_path.to_str().unwrap()).unwrap(),
        None
    );
    if let Some(previous) = previous {
        env::set_var("ZELYRA_UI_STYLESHEET", previous);
    } else {
        env::remove_var("ZELYRA_UI_STYLESHEET");
    }
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn project_ui_stylesheet_rejects_traversal_bad_extensions_invalid_utf8_and_large_files() {
    let directory = env::temp_dir().join(format!(
        "zelyra-ui-stylesheet-invalid-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let source_path = directory.join("main.zyl");
    fs::write(&source_path, "fn main() {}\n").unwrap();
    assert!(
        load_project_stylesheet(source_path.to_str().unwrap(), "../outside.css")
            .unwrap_err()
            .contains("`..`")
    );
    assert!(
        load_project_stylesheet(source_path.to_str().unwrap(), "https://example.test/a.css")
            .unwrap_err()
            .contains("project-relative")
    );
    assert!(
        load_project_stylesheet(source_path.to_str().unwrap(), "style.scss")
            .unwrap_err()
            .contains("end in `.css`")
    );
    fs::write(directory.join("bad.css"), [0xff, 0xfe]).unwrap();
    assert!(
        load_project_stylesheet(source_path.to_str().unwrap(), "bad.css")
            .unwrap_err()
            .contains("UTF-8")
    );
    fs::write(
        directory.join("large.css"),
        vec![b'x'; PROJECT_UI_CSS_MAX_BYTES as usize + 1],
    )
    .unwrap();
    assert!(
        load_project_stylesheet(source_path.to_str().unwrap(), "large.css")
            .unwrap_err()
            .contains("512 KiB")
    );
    fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn project_ui_stylesheet_rejects_symlinks_in_every_path_segment() {
    use std::os::unix::fs::symlink;

    let directory = env::temp_dir().join(format!(
        "zelyra-ui-stylesheet-link-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let source_path = directory.join("main.zyl");
    fs::write(&source_path, "fn main() {}\n").unwrap();
    fs::write(directory.join("outside.css"), "private").unwrap();
    symlink(directory.join("outside.css"), directory.join("style.css")).unwrap();
    let error = load_project_stylesheet(source_path.to_str().unwrap(), "style.css").unwrap_err();
    assert!(error.contains("symbolic links"));
    assert!(!error.contains("private"));
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
fn project_routes_cannot_shadow_the_builtin_module_settings_page() {
    let direct = parse(
        &lex(r#"page "/__zelyra/module-settings" { html { <main>Project page</main> } }"#).unwrap(),
    )
    .unwrap();
    let wildcard =
        parse(&lex(r#"api POST "/__zelyra/{resource}" { output String }"#).unwrap()).unwrap();
    let unrelated = parse(&lex("fn main() { }\n").unwrap()).unwrap();

    assert!(project_uses_reserved_module_settings_route(&direct));
    assert!(project_uses_reserved_module_settings_route(&wildcard));
    assert!(!project_uses_reserved_module_settings_route(&unrelated));
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
fn project_routes_cannot_shadow_the_private_schedule_probe() {
    let direct = parse(
        &lex(r#"page "/__zelyra/schedule/probe" { html { <main>Project page</main> } }"#).unwrap(),
    )
    .unwrap();
    let wildcard =
        parse(&lex(r#"api POST "/__zelyra/{resource}/{action}" { output String }"#).unwrap())
            .unwrap();
    let unrelated =
        parse(&lex(r#"page "/scheduled" { html { <main>Schedule page</main> } }"#).unwrap())
            .unwrap();

    assert!(project_uses_reserved_schedule_probe_route(&direct));
    assert!(project_uses_reserved_schedule_probe_route(&wildcard));
    assert!(!project_uses_reserved_schedule_probe_route(&unrelated));
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
fn doctor_uses_project_env_by_default_and_preserves_explicit_env_file() {
    let directory = env::temp_dir().join(format!(
        "zelyra-doctor-project-env-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let source = directory.join("main.zyl");
    let env_file = directory.join(".env");
    fs::write(&source, "").unwrap();
    fs::write(
        &env_file,
        "DATABASE_URL=mariadb://user:secret@127.0.0.1/db\n",
    )
    .unwrap();

    assert_eq!(
        doctor_environment_file(source.to_str().unwrap(), None),
        Some(env_file.to_string_lossy().into_owned())
    );
    assert_eq!(
        doctor_environment_file(source.to_str().unwrap(), Some("custom.env".into())),
        Some("custom.env".into())
    );

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn doctor_warns_when_the_default_port_is_busy_but_a_fallback_is_available() {
    let occupied = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = occupied.local_addr().unwrap().port();
    let check = doctor_web_port_check(port);
    assert_eq!(check.status, "warn");
    assert!(check.message.contains("already in use"));
    assert!(check
        .message
        .contains("select a free local port automatically"));
}

#[test]
fn web_server_uses_an_ephemeral_port_when_the_requested_port_is_busy() {
    let occupied = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let requested = occupied.local_addr().unwrap();
    let (listener, actual, used_fallback) = bind_web_application(&requested.to_string()).unwrap();
    assert!(used_fallback);
    assert_eq!(actual.ip(), requested.ip());
    assert_ne!(actual.port(), requested.port());
    assert_eq!(listener.local_addr().unwrap(), actual);
}

#[test]
fn outbox_cli_rejects_invalid_inspection_options_before_loading_a_project() {
    let arguments = ["list".to_owned(), "--limit".to_owned(), "0".to_owned()];
    assert_eq!(
        outbox_cli::outbox_command(arguments.into_iter()),
        ExitCode::from(2)
    );
    let arguments = ["list".to_owned(), "--limit".to_owned(), "201".to_owned()];
    assert_eq!(
        outbox_cli::outbox_command(arguments.into_iter()),
        ExitCode::from(2)
    );
    let arguments = ["list".to_owned(), "--format".to_owned(), "yaml".to_owned()];
    assert_eq!(
        outbox_cli::outbox_command(arguments.into_iter()),
        ExitCode::from(2)
    );
}

#[test]
fn outbox_list_explains_how_to_create_a_missing_internal_table() {
    let missing = outbox_cli::describe_outbox_inspection_error(
        "MySqlError { ERROR 1146 (42S02): Table 'zelyra_app._zelyra_outbox' doesn't exist }",
        "main.zyl",
    );
    assert!(missing.contains("error[E-OUTBOX-005]: the MariaDB outbox table is missing"));
    assert!(missing.contains("zelyra outbox setup main.zyl"));
    assert!(!missing.contains("MySqlError"));

    let spaced_path = outbox_cli::describe_outbox_inspection_error(
        "ERROR 1146 (42S02): Table '_zelyra_outbox' doesn't exist",
        "invoice tutorial/main.zyl",
    );
    assert!(spaced_path.contains("zelyra outbox setup 'invoice tutorial/main.zyl'"));

    let connection = outbox_cli::describe_outbox_inspection_error(
        "ERROR 1045 (28000): access denied",
        "main.zyl",
    );
    assert!(connection.contains("error[E-OUTBOX-005]: ERROR 1045"));
    assert!(!connection.contains("zelyra outbox setup"));
}

#[test]
fn outbox_cli_requires_an_explicit_handler_for_run() {
    let arguments = ["run".to_owned()];
    assert_eq!(
        outbox_cli::outbox_command(arguments.into_iter()),
        ExitCode::from(2)
    );
    let arguments = [
        "run".to_owned(),
        "--handler".to_owned(),
        "orders.created".to_owned(),
    ];
    assert_eq!(
        outbox_cli::outbox_command(arguments.into_iter()),
        ExitCode::from(2)
    );
    let arguments = [
        "run".to_owned(),
        "--handler".to_owned(),
        "orders.created=handle_order".to_owned(),
        "--lease-seconds".to_owned(),
        "3601".to_owned(),
    ];
    assert_eq!(
        outbox_cli::outbox_command(arguments.into_iter()),
        ExitCode::from(2)
    );
}

#[test]
fn outbox_cli_loads_database_url_from_the_project_env_file() {
    let directory = env::temp_dir().join(format!(
        "zelyra-outbox-env-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let source_path = directory.join("main.zyl");
    fs::write(
        &source_path,
        "database main { engine: mariadb } table events { id: Id primary auto } fn main() {}\n",
    )
    .unwrap();
    fs::write(
        directory.join(".env"),
        "ZELYRA_DATABASE_MAIN_URL=mariadb://user:pass@127.0.0.1:3312/app\n",
    )
    .unwrap();
    let schema = database_cli::load_schema(source_path.to_str().unwrap()).unwrap();
    assert_eq!(
        outbox_cli::project_outbox_database_url(source_path.to_str().unwrap(), &schema).unwrap(),
        Some("mariadb://user:pass@127.0.0.1:3312/app".into())
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn outbox_cli_run_invokes_zelyra_handler_and_skips_unmapped_event_types() {
    let Ok(database_url) = env::var("ZELYRA_DB_TIMEOUT_TEST_URL") else {
        eprintln!("skipping outbox CLI integration test: database URL is not configured");
        return;
    };
    let suffix = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let handled_id = format!("outbox-cli-handled-{suffix}");
    let unmapped_id = format!("outbox-cli-unmapped-{suffix}");
    zelyra_database::outbox::ensure_mariadb_outbox(&database_url).unwrap();
    let handled_event = zelyra_database::outbox::OutboxEvent::new(
        &handled_id,
        "cli.test",
        &json!({"source": "cli-test"}),
    )
    .unwrap();
    let unmapped_event = zelyra_database::outbox::OutboxEvent::new(
        &unmapped_id,
        "cli.unmapped",
        &json!({"source": "cli-test"}),
    )
    .unwrap();
    zelyra_database::with_mariadb_transaction(&database_url, |transaction| {
        zelyra_database::outbox::enqueue_mariadb_event(transaction, &unmapped_event)?;
        zelyra_database::outbox::enqueue_mariadb_event(transaction, &handled_event)
    })
    .unwrap();

    let directory = env::temp_dir().join(format!("zelyra-outbox-cli-{suffix}"));
    fs::create_dir_all(&directory).unwrap();
    let source_path = directory.join("main.zyl");
    fs::write(
        &source_path,
        "fn main() {}\npub fn handle_event(event_id: String, event_type: String, payload_json: String) -> Bool {\n    return true\n}\n",
    )
    .unwrap();
    let handler_specs = [("cli.test".to_owned(), "handle_event".to_owned())];
    assert_eq!(
        outbox_cli::run_outbox_worker(
            source_path.to_str().unwrap(),
            &database_url,
            &handler_specs,
            true,
            30
        ),
        ExitCode::SUCCESS
    );

    for (event_id, expected_delivered, expected_attempts) in
        [(&handled_id, true, "1"), (&unmapped_id, false, "0")]
    {
        let row = zelyra_database::execute_mariadb_query(
            &database_url,
            "SELECT `delivered_at`, `attempts` FROM `_zelyra_outbox` WHERE `event_id` = :event_id",
            vec![("event_id".into(), QueryValue::String(event_id.clone()))],
        )
        .unwrap()
        .rows
        .into_iter()
        .next()
        .expect("outbox event should remain present");
        assert_eq!(row[0] != "\\N", expected_delivered);
        assert_eq!(row[1], expected_attempts);
    }
    zelyra_database::execute_mariadb_query(
        &database_url,
        "DELETE FROM `_zelyra_outbox` WHERE `event_id` IN (:handled_id, :unmapped_id)",
        vec![
            ("handled_id".into(), QueryValue::String(handled_id)),
            ("unmapped_id".into(), QueryValue::String(unmapped_id)),
        ],
    )
    .unwrap();
    fs::remove_dir_all(directory).unwrap();
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
        ..QueryResult::default()
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
fn composes_typed_view_properties_into_page_metadata_and_escapes_values() {
    let source = r#"
        view SiteShell {
            props { title: String description: String? }
            html {
                <html><head><title>{title}</title><meta name="description" content="{description}" /></head>
                <body><slot /></body></html>
            }
        }
        page "/handbook" {
            view: SiteShell(title: "Zelyra & Business", description: "Types < security")
            html { <h1>Handbook</h1> }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_views("view-props.zyl", &program));
    let html = compose_page_view(&program, &program.pages[0]);
    assert!(html.contains("<title>Zelyra &amp; Business</title>"));
    assert!(html.contains("content=\"Types &lt; security\""));
    assert!(html.contains("<h1>Handbook</h1>"));
    let body_start = html.find("<body>").expect("view body opening tag");
    let page_content = html
        .find("<h1>Handbook</h1>")
        .expect("page content remains in the view body");
    let body_end = html.find("</body>").expect("view body closing tag");
    assert!(
        body_start < page_content && page_content < body_end,
        "composed layout lost its slot structure: {html}"
    );
    assert!(
        !html.contains("<slot"),
        "composed layout kept an unresolved slot: {html}"
    );
    assert!(!html.contains("{title}"));
    assert!(!html.contains("{description}"));
}

#[test]
fn rejects_missing_unknown_and_incompatible_page_view_properties() {
    for source in [
        r#"view Shell { props { title: String } html { <title>{title}</title><slot /> } }
            page "/" { view: Shell html { <p>Home</p> } }"#,
        r#"view Shell { props { title: String } html { <title>{title}</title><slot /> } }
            page "/" { view: Shell(title: "Home", extra: "x") html { <p>Home</p> } }"#,
        r#"view Shell { props { title: Int } html { <title>{title}</title><slot /> } }
            page "/" { view: Shell(title: "Home") html { <p>Home</p> } }"#,
    ] {
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(!validate_views("invalid-view-props.zyl", &program));
    }
}

#[test]
fn checks_dynamic_view_properties_against_page_inputs() {
    let source = r#"
        view SiteShell {
            props { title: String }
            html { <html><head><title>{title}</title></head><body><slot /></body></html> }
        }
        page "/handbook" {
            input { page_title: String }
            view: SiteShell(title: "{page_title}")
            html { <h1>Handbook</h1> }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_views("dynamic-view-props.zyl", &program));
    let html = compose_page_view(&program, &program.pages[0]);
    assert!(html.contains("<title>{page_title}</title>"));

    let incompatible = source.replace("page_title: String", "page_title: Int");
    let program = parse(&lex(&incompatible).unwrap()).unwrap();
    assert!(!validate_views("dynamic-view-props.zyl", &program));
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
fn imports_zelyra_ui_and_renders_checked_semantic_appearances() {
    let directory = std::env::temp_dir().join(format!(
        "zelyra-ui-component-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let entry = directory.join("main.zyl");
    fs::write(
        &entry,
        r#"
            import "zelyra/ui" as ui
            page "/ui" {
                html {
                    <PageHeader title="Invoices" description="Open work" />
                    <Grid columns="two">
                        <Card title="Ready"><Badge text="Paid" appearance="success" /></Card>
                        <Card title="Attention"><Badge text="Late" appearance="warning" /></Card>
                    </Grid>
                    <Button label="Open report" href="/reports" appearance="danger" />
                    <Button label="Local action" />
                    <SubmitButton label="Save" />
                    <Button label="&lt;script&gt;alert(1)&lt;/script&gt;" href="/reports?next=&lt;script&gt;" />
                    <TextField id="reference" name="reference" label="Reference" invalid="true" required="false" />
                }
            }
            form Search {
                field number: String { appearance: warning }
                field password: String { widget: password }
            }
        "#,
    )
    .unwrap();

    let loaded = project::load(entry.to_str().unwrap()).unwrap();
    assert!(loaded
        .program
        .components
        .iter()
        .any(|component| component.name == "PageHeader"));
    assert!(validate_components("main.zyl", &loaded.program));
    let html = compose_page_view(&loaded.program, &loaded.program.pages[0]);
    assert!(html.contains("class=\"z-badge z-badge--success\""));
    assert!(html.contains("class=\"z-badge z-badge--warning\""));
    assert!(html.contains("class=\"z-button z-button--danger\" href=\"/reports\""));
    assert!(html.contains(">Open report</a>"));
    assert!(html.contains("class=\"z-button z-button--default\" type=\"button\""));
    assert!(html.contains(">Local action</button>"));
    assert!(html.contains("class=\"z-button z-button--default\" type=\"submit\""));
    assert!(html.contains(">Save</button>"));
    assert!(html.contains("&amp;lt;script&amp;gt;alert(1)&amp;lt;/script&amp;gt;"));
    assert!(!html.contains("<script>alert(1)</script>"));
    assert!(html.contains("class=\"z-input z-input--default\" id=\"reference\""));
    assert!(html.contains("aria-invalid=\"true\""));
    assert!(!html.contains(
        "id=\"reference\" name=\"reference\" type=\"text\" value=\"\" placeholder=\"\" required"
    ));
    assert!(validate_form_properties("main.zyl", &loaded.program));

    fs::write(
        &entry,
        r#"import "zelyra/ui" as ui
           page "/bad" { html { <Badge text="No" appearance="neon" /> } }
           form Search { field number: String { appearance: "primary" } }"#,
    )
    .unwrap();
    let invalid = project::load(entry.to_str().unwrap()).unwrap();
    assert!(!validate_components("main.zyl", &invalid.program));
    assert!(!validate_form_properties("main.zyl", &invalid.program));
    fs::write(
        &entry,
        r#"form Search { field token: String { widget: hidden } }"#,
    )
    .unwrap();
    let invalid_widget = project::load(entry.to_str().unwrap()).unwrap();
    assert!(!validate_form_properties(
        "main.zyl",
        &invalid_widget.program
    ));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn dialog_defaults_generate_document_local_ids_and_keep_explicit_values() {
    let directory = std::env::temp_dir().join(format!(
        "zelyra-dialog-defaults-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let entry = directory.join("main.zyl");
    fs::write(
        &entry,
        r#"
            import "zelyra/ui" as ui
            page "/dialogs" {
                html {
                    <Dialog title="A &amp; B" open_label="Open &amp; view">
                        <p>Dialogue contents</p>
                    </Dialog>
                    <Dialog title="Second dialog" open_label="Open second">
                        <p>Second contents</p>
                    </Dialog>
                    <Dialog id="explicit-dialog" title="Explicit" open_label="Explicit opener" close_label="Dismiss">
                        <p>Explicit contents</p>
                    </Dialog>
                }
            }
        "#,
    )
    .unwrap();
    let loaded = project::load(entry.to_str().unwrap()).unwrap();
    assert!(validate_components("main.zyl", &loaded.program));
    let html = compose_page_view(&loaded.program, &loaded.program.pages[0]);
    assert_eq!(html.matches("data-z-dialog-auto=\"").count(), 2);
    assert!(html.contains(
        "id=\"z-dialog-auto-0000000000\" data-z-dialog data-z-dialog-auto=\"z-dialog-auto-0000000000\""
    ));
    assert!(html.contains(
        "id=\"z-dialog-auto-0000000001\" data-z-dialog data-z-dialog-auto=\"z-dialog-auto-0000000001\""
    ));
    assert!(html.contains("aria-controls=\"z-dialog-auto-0000000000\" aria-haspopup=\"dialog\""));
    assert!(html.contains("aria-labelledby=\"z-dialog-auto-0000000000-title\""));
    assert!(html.contains("data-z-dialog-close-default=\"true\" data-zelyra-i18n=\"ui.dialog.close\">Schließen</button>"));
    assert!(html.contains("Open &amp;amp; view"));
    assert!(html.contains("id=\"explicit-dialog\""));
    assert!(html.contains("data-z-dialog-close>Dismiss</button>"));
    assert!(html.contains("A &amp;amp; B"));
    assert!(!html.contains("A & B"));

    fs::write(
        &entry,
        r#"import "zelyra/ui" as ui
           page "/missing-opener" { html { <Dialog title="Required opener"><p>Content</p></Dialog> } }"#,
    )
    .unwrap();
    let missing_required = project::load(entry.to_str().unwrap()).unwrap();
    assert!(!validate_components("main.zyl", &missing_required.program));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn invoice_tutorial_component_gallery_renders_its_charts_and_accessible_tables() {
    let entry = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples/invoice_tutorial/main.zyl");
    let loaded = project::load(entry.to_str().unwrap()).unwrap();
    let page = loaded
        .program
        .pages
        .iter()
        .find(|page| page.path == "/ui/components")
        .expect("component gallery page");
    assert!(validate_views(
        "examples/invoice_tutorial/main.zyl",
        &loaded.program
    ));
    let html = compose_page_view(&loaded.program, page);

    assert!(html.contains("<figure class=\"z-chart z-chart--bar\">"));
    assert!(html.contains("<figure class=\"z-chart z-chart--line\">"));
    assert!(html.contains("<figure class=\"z-chart z-chart--donut\">"));
    assert!(html.contains(
        "<caption data-zelyra-i18n=\"invoice.ui.volume_caption\">Rechnungsvolumen nach Monat</caption>"
    ));
    assert!(html.contains("<dialog class=\"z-dialog\""));
}

#[test]
fn recursive_component_expansion_is_rejected_with_a_stable_diagnostic() {
    let source = r#"
        component First { html { <section><Second /></section> } }
        component Second { html { <section><First /></section> } }
        page "/" { html { <First /> } }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(!validate_components("recursive-components.zyl", &program));
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
fn validates_static_translation_references_in_view_text_and_attributes() {
    let source = r#"
        page "/" {
            html {
                <main title='{t("site.home.title")}'>
                    <h1>{t("site.home.title")}</h1>
                    <p>{i18n("site.home.intro")}</p>
                </main>
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_components("localized-page.zyl", &program));

    let invalid = r#"
        page "/" { html { <h1>{t("bad key")}</h1> } }
    "#;
    let program = parse(&lex(invalid).unwrap()).unwrap();
    assert!(!validate_components("invalid-locale-key.zyl", &program));

    let invalid_media_type = r#"
        component FeedTitle {
            html { <title>{t("site.feed.title")}</title> }
        }
        page "/feed.xml" {
            content_type: "application/xml; charset=utf-8"
            html { <feed><FeedTitle /></feed> }
        }
    "#;
    let program = parse(&lex(invalid_media_type).unwrap()).unwrap();
    assert!(!validate_page_data("localized-xml.zyl", &program));
}

#[test]
fn validates_optional_typed_page_data_for_missing_detail_rows() {
    let source = r#"
        table packages { id: Id primary auto slug: String(120) required name: String(200) required }
        page "/packages/{slug}" {
            load package = sql<Package?> {
                SELECT id, slug, name FROM packages WHERE slug = :slug
            }
            html { <main>{package}</main> }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_page_data("optional-page-data.zyl", &program));
    assert!(matches!(
        program.pages[0].data[0].result_type,
        Type::Option(_)
    ));
    assert_eq!(
        page_data_fields(&program, &program.pages[0].data[0].result_type),
        ["id", "slug", "name"]
    );

    let unsupported_source = r#"
        page "/invalid" {
            load value = sql<Int?> { SELECT 1 }
            html { <main>{value}</main> }
        }
    "#;
    let unsupported = parse(&lex(unsupported_source).unwrap()).unwrap();
    assert!(!validate_page_data(
        "unsupported-page-data.zyl",
        &unsupported
    ));
}

#[test]
fn validates_dynamic_page_media_types_and_rejects_wildcards() {
    let valid_source = r#"
        page "/feed.xml" {
            content_type: "application/xml; charset=utf-8"
            html { <feed><title>Zelyra</title></feed> }
        }
    "#;
    let valid = parse(&lex(valid_source).unwrap()).unwrap();
    assert!(validate_page_data("feed.zyl", &valid));

    let invalid_source = r#"
        page "/anything" {
            content_type: "*/*"
            html { <p>not a concrete media type</p> }
        }
    "#;
    let invalid = parse(&lex(invalid_source).unwrap()).unwrap();
    assert!(!validate_page_data("wildcard.zyl", &invalid));
}

#[test]
fn validates_create_edit_contexts_and_custom_tableview_row_actions() {
    let source = r#"
        table customers { id: Id primary auto name: String(100) email: Email password_hash: String(255) }
        crud Customer -> customers {
            view {
                fields { name email password_hash }
                create {
                    before_form { html { <p>{values.email}</p> } }
                    field_content { name { html { <a href="/help">Help</a><small>{values.email}</small> } } }
                }
                edit {
                    after_form { html { <p>{record.name}</p> } }
                }
            }
        }
        tableview Customers {
            source sql<Customer[]> { SELECT id, name FROM customers }
            columns { id name }
            view {
                row_actions {
                    item: customer
                    html { <a href="/customers/{customer.id}">Open</a><button type="button" data-name="{customer.name}">Inspect</button> }
                }
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_components("generated.zyl", &program));
}

#[test]
fn tableview_row_actions_can_use_imported_zelyra_dialog_components() {
    let directory = std::env::temp_dir().join(format!(
        "zelyra-tableview-dialog-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let entry = directory.join("main.zyl");
    fs::write(
        &entry,
        r#"
            import "zelyra/ui" as ui
            struct CustomerRow { id: Id name: String }
            tableview Customers {
                source sql<CustomerRow[]> { SELECT id, name FROM customers }
                columns { id name }
                view {
                    row_actions {
                        item: customer
                        html {
                            <Dialog title="Customer details" open_label="Details">
                                <p>{customer.name}</p>
                            </Dialog>
                        }
                    }
                }
            }
        "#,
    )
    .unwrap();

    let loaded = project::load(entry.to_str().unwrap()).unwrap();
    assert!(validate_components("main.zyl", &loaded.program));
    let tableview = &loaded.program.tableviews[0];
    let actions = tableview.view.row_actions.as_ref().unwrap();
    let expanded = expand_view_components(&loaded.program, actions.html.clone());
    assert!(expanded.contains("data-z-dialog"));
    assert!(expanded.contains("aria-controls="));
    assert!(expanded.contains("{customer.name}"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn validates_tableview_hidden_row_fields_and_column_content_context() {
    let source = r#"
        struct ItemRow { sku: String customer_number: String }
        tableview Items {
            source sql<ItemRow[]> { SELECT sku, customer_number FROM items }
            columns { sku }
            view {
                row_fields { customer_number }
                column_content {
                    item: item
                    sku { html { <small>Customer: {item.customer_number}</small> } }
                }
                row_actions {
                    item: item
                    html { <a href="/customers/{item.customer_number}">Customer</a> }
                }
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_components("tableview-content.zyl", &program));

    let invalid = source.replace("row_fields { customer_number }", "row_fields { absent }");
    let program = parse(&lex(&invalid).unwrap()).unwrap();
    assert!(!validate_components(
        "tableview-content-invalid.zyl",
        &program
    ));

    let invalid_column = source.replace("sku { html", "unknown { html");
    let program = parse(&lex(&invalid_column).unwrap()).unwrap();
    assert!(!validate_components(
        "tableview-column-invalid.zyl",
        &program
    ));
}

#[test]
fn validates_policy_scoped_tableview_actions_against_complete_crud_rows() {
    let source = r#"
        table invoices {
            id: Id primary auto
            status: String(20) required
            locked: Bool required
        }
        struct Invoice { id: Id status: String locked: Bool }
        crud Invoice -> invoices {
            guard edit when locked == true { error: "Locked" }
            guard delete when locked == true { error: "Locked" }
        }
        tableview InvoiceRows {
            source sql<Invoice[]> { SELECT id, status, locked FROM invoices }
            columns { status }
            view {
                row_fields { id locked }
                row_actions {
                    item: invoice
                    crud: Invoice
                    edit { html { <a href="/invoices/{invoice.id}/edit">Edit</a> } }
                    delete { html { <button type="button">Delete</button> } }
                }
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_components("policy-tableview.zyl", &program));

    let incomplete = source.replace("row_fields { id locked }", "row_fields { id }");
    let program = parse(&lex(&incomplete).unwrap()).unwrap();
    assert!(!validate_components(
        "policy-tableview-incomplete.zyl",
        &program
    ));
}

#[test]
fn rejects_saved_record_in_create_and_unavailable_or_sensitive_fields() {
    let source = r#"
        table customers { id: Id primary auto name: String(100) password_hash: String(255) }
        crud Customer -> customers {
            view {
                fields { name password_hash }
                create { before_form { html { <p>{record.name}</p> } } }
                edit {
                    after_form { html { <p>{record.id} {record.password_hash}</p> } }
                    field_content { missing { html { <p>Missing</p> } } }
                }
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(!validate_components("invalid-context.zyl", &program));
}

#[test]
fn rejects_generated_form_nested_in_crud_custom_content() {
    let source = r#"
        table customers { id: Id primary auto name: String(100) }
        crud Customer -> customers { view { create { before_form { html { <form><p>Wrong</p></form> } } } } }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(!validate_components("nested-form.zyl", &program));

    let explicit_button = r#"
        table customers { id: Id primary auto name: String(100) }
        crud Customer -> customers {
            view {
                fields { name }
                create {
                    field_content { name { html { <button type="button">Help</button> } } }
                }
            }
        }
    "#;
    let program = parse(&lex(explicit_button).unwrap()).unwrap();
    assert!(validate_components("explicit-button.zyl", &program));

    let implicit_submit = explicit_button.replace(" type=\"button\"", "");
    let program = parse(&lex(&implicit_submit).unwrap()).unwrap();
    assert!(!validate_components("implicit-submit.zyl", &program));
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
fn component_string_properties_are_html_escaped() {
    let source = r#"
        component ModuleNotice {
            props { module: String }
            html { <h1>{module}</h1> }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_components("notice.zyl", &program));

    let rendered = render_view_component(
        &program.components[0],
        "module=\"<script>alert(1)</script> & 'quoted'\"",
        None,
    );
    assert_eq!(
        rendered.trim(),
        "<h1>&lt;script&gt;alert(1)&lt;/script&gt; &amp; &#39;quoted&#39;</h1>"
    );
}

#[test]
fn component_properties_resolve_explicit_project_locale_references() {
    let source = r#"
        component LocalizedHeader {
            props { title: String }
            html { <h1>{title}</h1> }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_components("localized.zyl", &program));
    let rendered = render_view_component(
        &program.components[0],
        "title=\"@i18n:invoice.ui.title\"",
        None,
    );
    assert_eq!(
        rendered.trim(),
        "<h1>\u{e000}zelyra-locale:invoice.ui.title\u{e001}</h1>"
    );

    let invalid = render_view_component(&program.components[0], "title=\"@i18n:bad key\"", None);
    assert_eq!(invalid.trim(), "<h1>@i18n:bad key</h1>");
}

#[test]
fn url_encode_component_interpolation_is_typed_and_handles_static_and_dynamic_values() {
    let source = r#"
        component EmailLink {
            props { recipient: String subject: String body: String }
            html { <a href="mailto:{url_encode:recipient}?subject={url_encode:subject}&amp;body={url_encode:body}">Email</a> }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_components("email.zyl", &program));

    let dynamic = render_view_component(
        &program.components[0],
        "recipient=\"{invoice.customer_email}\" subject=\"{invoice.invoice_number}\" body=\"{invoice.invoice_number}\"",
        None,
    );
    assert!(dynamic.contains(
        "mailto:{url_encode:invoice.customer_email}?subject={url_encode:invoice.invoice_number}&amp;body={url_encode:invoice.invoice_number}"
    ));

    let static_value = render_view_component(
        &program.components[0],
        "recipient=\"rechnung@example.test\" subject=\"RE 2026&001\" body=\"Rechnung RE 2026&001\"",
        None,
    );
    assert!(static_value.contains(
        "mailto:rechnung%40example.test?subject=RE%202026%26001&amp;body=Rechnung%20RE%202026%26001"
    ));
}

#[test]
fn rejects_url_encode_component_interpolation_on_non_text_values() {
    let source = r#"
        component InvalidEmailLink {
            props { recipient: Int }
            html { <a href="mailto:{url_encode:recipient}">Email</a> }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(!validate_components("email.zyl", &program));
}

#[test]
fn markdown_template_interpolation_is_typed_and_content_only() {
    let valid = r#"
        component HelpArticle {
            props { markdown: String }
            html { <article>{render_markdown:markdown}</article> }
        }
    "#;
    let program = parse(&lex(valid).unwrap()).unwrap();
    assert!(validate_components("markdown.zyl", &program));
    let dynamic = render_view_component(&program.components[0], "markdown=\"{doc.body}\"", None);
    assert!(dynamic.contains("{render_markdown:doc.body}"));
    let static_html = render_view_component(
        &program.components[0],
        "markdown=\"<script>alert(1)</script>\"",
        None,
    );
    assert!(!static_html.contains("<script"));

    let invalid_type = valid.replace("markdown: String", "markdown: Int");
    let program = parse(&lex(&invalid_type).unwrap()).unwrap();
    assert!(!validate_components("markdown.zyl", &program));

    let invalid_attribute = valid.replace(
        "<article>{render_markdown:markdown}</article>",
        "<article title=\"{render_markdown:markdown}\">Help</article>",
    );
    let program = parse(&lex(&invalid_attribute).unwrap()).unwrap();
    assert!(!validate_components("markdown.zyl", &program));
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
fn nested_component_slots_remain_owned_by_their_component_inside_views() {
    let source = r#"
        component Frame {
            html {
                <div><header><slot name="toolbar" /></header><main><slot /></main></div>
            }
        }
        component Card {
            html { <article><header><slot name="header" /></header><section><slot /></section></article> }
        }
        view Shell {
            html {
                <Frame>
                    <slot name="toolbar"><span>Default toolbar</span></slot>
                    <slot />
                </Frame>
            }
        }
        page "/nested" {
            view: Shell
            html {
                <Card><slot name="header"><strong>Page heading</strong></slot><p>Page body</p></Card>
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    assert!(validate_views("nested.zyl", &program));
    assert!(validate_components("nested.zyl", &program));
    let html = compose_page_view(&program, &program.pages[0]);
    assert!(html.contains("<header><span>Default toolbar</span></header>"));
    assert!(html.contains("<header><strong>Page heading</strong></header>"));
    assert!(html.contains("<section><p>Page body</p></section>"));
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
fn composes_tableview_layout_around_generated_table_content() {
    let source = r#"
        view Shell {
            html { <html><body><header>Invoices</header><main><slot /></main></body></html> }
        }
        tableview InvoiceDashboard {
            layout: Shell
            source sql<Invoice[]> { SELECT id, invoice_number FROM invoices }
            columns { id invoice_number }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();

    assert!(validate_views("dashboard.zyl", &program));
    let layout = tableview_layout_html(&program, &program.tableviews[0]).unwrap();
    assert!(layout.contains("<header>Invoices</header>"));
    assert!(layout.contains(CRUD_LAYOUT_CONTENT_MARKER));
    assert!(!layout.contains("<slot"));
}

#[test]
fn rejects_unknown_tableview_layout_view() {
    let source = r#"
        tableview InvoiceDashboard {
            layout: MissingShell
            source sql<Invoice[]> { SELECT id, invoice_number FROM invoices }
            columns { id invoice_number }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();

    assert!(!validate_views("dashboard.zyl", &program));
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
fn dispatches_typed_api_responses_with_status_headers_and_structured_bodies() {
    let program = parse(
        &lex(r#"
            struct Customer { id: Int name: String }
            api POST "/customers" { handler create output ApiResponse<Customer> responses { 201 "application/json" } }
            fn create() -> ApiResponse<Customer> {
                return api_response(201, ["Cache-Control: no-store", "Content-Type: application/json"], Customer { id: 42 name: "Anna" })
            }
            fn main() { }
        "#)
        .unwrap(),
    )
    .unwrap();
    check(&program).unwrap();
    check_apis(&program).unwrap();
    let api = &program.apis[0];
    let request =
        zelyra_web::parse_request("POST /customers HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let response = dispatch_api(&program, api, "create", &request, &HashMap::new(), None);
    assert_eq!(response.status, 201);
    assert_eq!(response.content_type, "application/json");
    assert_eq!(
        response.headers,
        [("Cache-Control".into(), "no-store".into())]
    );
    let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(body, serde_json::json!({"id": 42, "name": "Anna"}));
    let openapi = docs::format_openapi(&program);
    assert!(openapi.contains("\"201\""));
    assert!(openapi.contains("application/json"));
    let client = docs::format_typescript_client(&program);
    assert!(client.contains("Promise<ApiResponse<Customer>>"));
}

#[test]
fn dispatches_typed_api_text_response_without_json_quoting() {
    let program = parse(
        &lex(r#"
            api GET "/feed.xml" { handler feed output ApiResponse<String> responses { 200 "application/xml; charset=utf-8" } }
            fn feed() -> ApiResponse<String> {
                return api_response(200, ["Content-Type: application/xml; charset=utf-8"], "<feed><item>1</item></feed>")
            }
            fn main() { }
        "#)
        .unwrap(),
    )
    .unwrap();
    check(&program).unwrap();
    check_apis(&program).unwrap();
    let request =
        zelyra_web::parse_request("GET /feed.xml HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let response = dispatch_api(
        &program,
        &program.apis[0],
        "feed",
        &request,
        &HashMap::new(),
        None,
    );
    assert_eq!(response.status, 200);
    assert_eq!(response.content_type, "application/xml; charset=utf-8");
    assert_eq!(response.body, "<feed><item>1</item></feed>");
}

#[test]
fn typed_api_response_contracts_reject_missing_mismatched_and_duplicate_declarations() {
    for (source, expected) in [
        (
            r#"api GET "/x" { handler get output ApiResponse<String> } fn get() -> ApiResponse<String> { return api_response(200, [], "ok") } fn main() {}"#,
            "requires a `responses` block",
        ),
        (
            r#"api GET "/x" { handler get output String responses { 200 "application/json" } } fn get() -> String { return "ok" } fn main() {}"#,
            "require output `ApiResponse<T>`",
        ),
        (
            r#"api GET "/x" { handler get output ApiResponse<String> responses { 200 "text/html" } } fn get() -> ApiResponse<String> { return api_response(200, [], "ok") } fn main() {}"#,
            "non-executable",
        ),
    ] {
        let program = parse(&lex(source).unwrap()).unwrap();
        let errors = check_apis(&program).expect_err("invalid response contract must fail");
        assert!(
            errors.iter().any(|error| error.message.contains(expected)),
            "expected diagnostic containing {expected:?}, got {errors:?}"
        );
    }

    let duplicate = parse(
        &lex(r#"api GET "/x" { handler get output ApiResponse<String> responses { 200 "application/json" 200 "text/plain" } } fn get() -> ApiResponse<String> { return api_response(200, [], "ok") } fn main() {}"#).unwrap(),
    )
    .unwrap();
    assert!(check_apis(&duplicate).unwrap_err().iter().any(|error| error
        .message
        .contains("duplicate API successful response status")));
}

#[test]
fn typed_api_response_cannot_return_an_undeclared_status() {
    let program = parse(
        &lex(r#"
            api GET "/x" { handler get output ApiResponse<String> responses { 200 "text/plain" } }
            fn get() -> ApiResponse<String> { return api_response(201, [], "created") }
            fn main() {}
        "#)
        .unwrap(),
    )
    .unwrap();
    check(&program).unwrap();
    check_apis(&program).unwrap();
    let request = zelyra_web::parse_request("GET /x HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let response = dispatch_api(
        &program,
        &program.apis[0],
        "get",
        &request,
        &HashMap::new(),
        None,
    );
    assert_eq!(response.status, 500);
    assert!(response.body.contains("ApiResponse status is not declared"));
}

#[test]
fn evaluates_declared_table_validation_before_api_handler() {
    let program = parse(
        &lex(r#"
            table invoices {
                id: Id primary auto
                amount: Int required
                validate { amount > 0: "Amount must be positive" }
            }
            api POST "/invoices" {
                handler create_invoice
                input { amount: Int }
                validates invoices
                output Result<Int, String>
                errors { 422 ValidationError: String }
            }
            fn create_invoice(amount: Int) -> Result<Int, String> { return Ok(amount) }
            fn main() { }
        "#)
        .unwrap(),
    )
    .unwrap();
    check_apis(&program).expect("API validation contract must type-check");
    let api = &program.apis[0];

    let invalid = zelyra_web::parse_request(
        "POST /invoices HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"amount\":0}",
    )
    .unwrap();
    let response = dispatch_api(
        &program,
        api,
        "create_invoice",
        &invalid,
        &HashMap::new(),
        None,
    );
    assert_eq!(response.status, 422);
    let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(body["error"]["code"], "ValidationError");
    assert_eq!(body["error"]["details"], "Amount must be positive");

    let valid = zelyra_web::parse_request(
        "POST /invoices HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"amount\":42}",
    )
    .unwrap();
    let response = dispatch_api(
        &program,
        api,
        "create_invoice",
        &valid,
        &HashMap::new(),
        None,
    );
    assert_eq!(response.status, 200);
    assert_eq!(response.body, "42");
}

#[test]
fn api_validation_passes_server_computed_fields_to_handlers_without_request_input() {
    let program = parse(
        &lex(r#"
            table invoice_items {
                id: Id primary auto
                quantity: Int required
                unit_price: Int required
                total: Int required compute(quantity * unit_price)
                validate { total >= 0: "Total must not be negative" }
            }
            api POST "/invoice-items" {
                handler create_item
                input { quantity: Int unit_price: Int }
                validates invoice_items
                output Result<Int, String>
                errors { 422 ValidationError: String }
            }
            api PATCH "/invoice-items/{id}" {
                handler patch_item
                input { id: Id quantity: Int unit_price: Int }
                validates invoice_items
                output Result<Int, String>
                errors { 422 ValidationError: String }
            }
            fn create_item(quantity: Int, unit_price: Int, total: Int) -> Result<Int, String> {
                return Ok(total)
            }
            fn patch_item(id: Id, quantity: Int, unit_price: Int, total: Int) -> Result<Int, String> {
                return Ok(total)
            }
            fn main() { }
        "#)
        .unwrap(),
    )
    .unwrap();
    check_apis(&program).expect("computed columns should be injected into the handler contract");
    zelyra_runtime::check(&program).expect("computed API handler types should be valid");

    let request = zelyra_web::parse_request(
        "POST /invoice-items HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"quantity\":3,\"unit_price\":8,\"total\":999}",
    )
    .unwrap();
    let response = dispatch_api(
        &program,
        &program.apis[0],
        "create_item",
        &request,
        &HashMap::new(),
        None,
    );
    assert_eq!(response.status, 200, "{}", response.body);
    assert_eq!(response.body, "24", "client-supplied total must be ignored");

    let patch = zelyra_web::parse_request(
        "PATCH /invoice-items/7 HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"quantity\":4,\"unit_price\":8,\"total\":999}",
    )
    .unwrap();
    let response = dispatch_api(
        &program,
        &program.apis[1],
        "patch_item",
        &patch,
        &HashMap::from([("id".to_owned(), "7".to_owned())]),
        None,
    );
    assert_eq!(response.status, 200, "{}", response.body);
    assert_eq!(
        response.body, "32",
        "PATCH recomputes the derived value and ignores the submitted value"
    );
}

#[test]
fn api_validation_rejects_computed_columns_as_request_inputs() {
    let program = parse(
        &lex(r#"
            table invoice_items {
                id: Id primary auto
                quantity: Int required
                total: Int required compute(quantity * 2)
                validate { total >= 0: "Total must not be negative" }
            }
            api POST "/invoice-items" {
                handler create_item
                input { quantity: Int total: Int }
                validates invoice_items
                output Int
                errors { 422 ValidationError: String }
            }
            fn create_item(quantity: Int, total: Int) -> Int { return total }
            fn main() { }
        "#)
        .unwrap(),
    )
    .unwrap();
    let errors = check_apis(&program).expect_err("computed fields cannot be request-controlled");
    assert!(errors.iter().any(|error| error
        .message
        .contains("must not accept computed field `total` from request input")));
}

#[test]
fn computed_api_handlers_cannot_be_called_outside_the_validated_api_boundary() {
    let source = r#"
        table invoice_items {
            quantity: Int required
            total: Int required compute(quantity * 2)
            validate { total >= 0: "Total must not be negative" }
        }
        api POST "/invoice-items" {
            handler create_item
            input { quantity: Int }
            validates invoice_items
            output Result<Int, String>
            errors { 422 ValidationError: String }
        }
        fn create_item(quantity: Int, total: Int) -> Result<Int, String> { return Ok(total) }
        fn main() { create_item(1, 999) }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    let errors = check_apis(&program)
        .expect_err("API-generated computed parameters cannot be forged by a source caller");
    assert!(errors.iter().any(|error| error
        .message
        .contains("cannot be called from Zelyra functions")));
}

#[test]
#[ignore = "requires ZELYRA_TEST_DATABASE_URL pointing to a disposable MariaDB or PostgreSQL database"]
fn computed_api_fields_are_recomputed_before_validation_and_database_write() {
    let source = r#"
        database main { engine: mariadb }
        table __TABLE__ {
            id: Id primary auto
            quantity: Decimal(10, 3) required
            unit_price: Decimal(10, 3) required
            total: Decimal(12, 2) required compute(quantity * unit_price)
            validate { total > decimal("0"): "Total must be positive" }
        }
        api POST "/items" {
            handler create_item
            input { quantity: Decimal unit_price: Decimal }
            validates __TABLE__
            output Result<Decimal, String>
            errors { 422 ValidationError: String }
        }
        api PUT "/items/{id}" {
            handler update_item
            input { id: Id quantity: Decimal unit_price: Decimal }
            validates __TABLE__
            output Result<Decimal, String>
            errors { 422 ValidationError: String }
        }
            api PATCH "/items/{id}" {
                handler update_item
                input { id: Id quantity: Decimal unit_price: Decimal }
                validates __TABLE__
                output Result<Decimal, String>
                errors { 422 ValidationError: String }
            }
        fn create_item(quantity: Decimal, unit_price: Decimal, total: Decimal) -> Result<Decimal, String> uses Database(write) {
            transaction {
                sql { INSERT INTO __TABLE__ (quantity, unit_price, total) VALUES (:quantity, :unit_price, :total) }
            }
            return Ok(total)
        }
        fn update_item(id: Id, quantity: Decimal, unit_price: Decimal, total: Decimal) -> Result<Decimal, String> uses Database(write) {
            transaction {
                sql { UPDATE __TABLE__ SET quantity = :quantity, unit_price = :unit_price, total = :total WHERE id = :id }
            }
            return Ok(total)
        }
        fn main() { }
    "#;
    let table_name = format!("zelyra_computed_api_{}", std::process::id());
    let database_url = env::var("ZELYRA_TEST_DATABASE_URL")
        .expect("ignored integration test needs a disposable database URL");
    let engine = if zelyra_database::is_postgres_url(&database_url) {
        "postgres"
    } else {
        "mariadb"
    };
    let source = source
        .replace("__TABLE__", &table_name)
        .replace("engine: mariadb", &format!("engine: {engine}"));
    let program = parse(&lex(&source).unwrap()).unwrap();
    check_apis(&program).expect("computed API contract should type-check");
    zelyra_runtime::check(&program).expect("computed API handler should type-check");
    let schema = zelyra_database::build_schema(&program).unwrap();
    let create_sql = schema.create_sql();
    zelyra_database::execute_query(&database_url, &create_sql, Vec::new()).unwrap_or_else(
        |error| panic!("could not create computed API test schema: {error}; SQL: {create_sql}"),
    );

    let request = zelyra_web::parse_request(
        "POST /items HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"quantity\":3,\"unit_price\":0.385,\"total\":999}",
    )
    .unwrap();
    let response = dispatch_api(
        &program,
        &program.apis[0],
        "create_item",
        &request,
        &HashMap::new(),
        Some(&database_url),
    );
    assert_eq!(response.status, 200, "{}", response.body);
    let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(body.as_str(), Some("1.16"));
    let rows = zelyra_database::execute_query(
        &database_url,
        &format!("SELECT id, total FROM {table_name}"),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(rows.rows.len(), 1);
    assert_eq!(rows.rows[0][1], "1.16");
    let id = rows.rows[0][0].clone();
    let update_request = zelyra_web::parse_request(
        "PUT /items/1 HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"quantity\":4,\"unit_price\":0.385,\"total\":999}",
    )
    .unwrap();
    let response = dispatch_api(
        &program,
        &program.apis[1],
        "update_item",
        &update_request,
        &HashMap::from([("id".to_owned(), id.clone())]),
        Some(&database_url),
    );
    assert_eq!(response.status, 200, "{}", response.body);
    let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(body.as_str(), Some("1.54"));
    let rows = zelyra_database::execute_query(
        &database_url,
        &format!("SELECT total FROM {table_name}"),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(rows.rows, vec![vec!["1.54".to_owned()]]);
    let invalid_patch_request = zelyra_web::parse_request(
        "PATCH /items/1 HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"quantity\":-5}",
    )
    .unwrap();
    let invalid_patch_response = dispatch_api(
        &program,
        &program.apis[2],
        "update_item",
        &invalid_patch_request,
        &HashMap::from([("id".to_owned(), id.clone())]),
        Some(&database_url),
    );
    assert_eq!(
        invalid_patch_response.status, 422,
        "{}",
        invalid_patch_response.body
    );
    let rows = zelyra_database::execute_query(
        &database_url,
        &format!("SELECT quantity, unit_price, total FROM {table_name}"),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(
        rows.rows,
        vec![vec![
            String::from("4.000"),
            String::from("0.385"),
            String::from("1.54"),
        ]]
    );

    let patch_request = zelyra_web::parse_request(
        "PATCH /items/1 HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"quantity\":5,\"total\":999}",
    )
    .unwrap();
    let response = dispatch_api(
        &program,
        &program.apis[2],
        "update_item",
        &patch_request,
        &HashMap::from([("id".to_owned(), id.clone())]),
        Some(&database_url),
    );
    assert_eq!(response.status, 200, "{}", response.body);
    let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(body.as_str(), Some("1.93"));
    let rows = zelyra_database::execute_query(
        &database_url,
        &format!("SELECT total FROM {table_name}"),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(rows.rows, vec![vec!["1.93".to_owned()]]);
    let rows = zelyra_database::execute_query(
        &database_url,
        &format!("SELECT quantity, unit_price, total FROM {table_name}"),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(
        rows.rows,
        vec![vec![
            String::from("5.000"),
            String::from("0.385"),
            String::from("1.93"),
        ]]
    );
    zelyra_database::execute_query(
        &database_url,
        &format!("DROP TABLE {table_name}"),
        Vec::new(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires ZELYRA_TEST_DATABASE_URL pointing to a disposable MariaDB or PostgreSQL database"]
fn api_patch_row_merge_is_limited_to_the_verified_tenant() {
    let database_url = env::var("ZELYRA_TEST_DATABASE_URL")
        .expect("ignored integration test needs a disposable database URL");
    let engine = if zelyra_database::is_postgres_url(&database_url) {
        "postgres"
    } else {
        "mariadb"
    };
    let source = format!(
        r#"
            database main {{ engine: {engine} }}
            type TenantId = Id
            table patch_tenant_scope_{pid} scoped by tenant_id: TenantId {{
                id: Id primary
                tenant_id: TenantId required
                label: String required
            }}
            fn main() {{}}
        "#,
        pid = std::process::id()
    );
    let program = parse(&lex(&source).unwrap()).unwrap();
    let table = &program.tables[0];
    let schema = zelyra_database::build_schema(&program).unwrap();
    zelyra_database::execute_query(&database_url, &schema.create_sql(), Vec::new()).unwrap();
    for (id, tenant_id, label) in [(1, 10, "Northwind"), (2, 11, "Contoso")] {
        zelyra_database::execute_query(
            &database_url,
            &format!(
                "INSERT INTO {} (id, tenant_id, label) VALUES (:id, :tenant, :label)",
                table.name
            ),
            vec![
                ("id".into(), QueryValue::Int(id)),
                ("tenant".into(), QueryValue::Int(tenant_id)),
                ("label".into(), QueryValue::String(label.into())),
            ],
        )
        .unwrap();
    }

    let mut foreign_tenant_values =
        HashMap::from([(String::from("id"), serde_json::Value::from(1))]);
    assert!(matches!(
        merge_api_patch_row_values(
            &program,
            table,
            &database_url,
            Some(11),
            &mut foreign_tenant_values
        ),
        Err(ApiPatchMergeError::NotFound)
    ));
    assert_eq!(foreign_tenant_values.len(), 1);

    assert!(matches!(
        merge_api_patch_row_values(
            &program,
            table,
            &database_url,
            None,
            &mut HashMap::from([(String::from("id"), serde_json::Value::from(1))])
        ),
        Err(ApiPatchMergeError::BadRequest(_))
    ));

    let mut matching_tenant_values =
        HashMap::from([(String::from("id"), serde_json::Value::from(1))]);
    merge_api_patch_row_values(
        &program,
        table,
        &database_url,
        Some(10),
        &mut matching_tenant_values,
    )
    .unwrap();
    assert_eq!(
        matching_tenant_values.get("label"),
        Some(&serde_json::Value::from("Northwind"))
    );
    assert_eq!(
        matching_tenant_values.get("tenant_id"),
        Some(&serde_json::Value::from("10"))
    );
    zelyra_database::execute_query(
        &database_url,
        &format!("DROP TABLE {}", table.name),
        Vec::new(),
    )
    .unwrap();
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
            table_validation_functions: &[],
            computed_column_functions: &[],
            verified_tenant_id: None,
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
            table_validation_functions: &[],
            computed_column_functions: &[],
            verified_tenant_id: None,
        },
    );
    assert_eq!(allowed.status, 500);
    assert!(!allowed.body.contains("not granted"));
}

#[test]
fn scoped_api_validation_uses_only_verified_tenant_context() {
    let program = parse(
        &lex(r#"
                type TenantId = Id
                auth users { table: users memberships: tenant_memberships }
                table invoices scoped by tenant_id: TenantId {
                    id: Id primary auto
                    tenant_id: TenantId required
                    amount: Int required
                    invoice_date: Date required
                    due_date: Date?
                    validate {
                        amount > 0: "Amount must be positive."
                        match due_date {
                            Some(due) => due >= invoice_date
                            None => true
                        }: "Due date must not precede invoice date."
                    }
                }
                api POST "/api/check" {
                    requires auth
                    handler check
                    validates invoices
                    input { amount: Int invoice_date: Date due_date: Date? }
                    output Result<TenantId, String>
                    errors { 422 ValidationError: String }
                }
                fn check(amount: Int, invoice_date: Date, due_date: Date?, tenant_id: TenantId) -> Result<TenantId, String> {
                    return Ok(tenant_id)
                }
                fn main() {}
            "#)
        .unwrap(),
    )
    .unwrap();
    check_apis(&program).expect("scoped API and handler contract should type-check");
    let (lowered_program, table_validation_functions) =
        zelyra_ast::lower_table_validation_functions(&program);
    let api = &program.apis[0];
    let request = |body: &str| {
        zelyra_web::parse_request(&format!(
            "POST /api/check HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(), body
        ))
        .unwrap()
    };
    let context = |verified_tenant_id| ApiRuntimeContext {
        database_url: None,
        capability_grants: None,
        runtime_policy: None,
        table_validation_functions: &table_validation_functions,
        computed_column_functions: &[],
        verified_tenant_id,
    };

    let valid = dispatch_api_with_capabilities(
        &lowered_program,
        api,
        "check",
        &request(r#"{"amount":100,"invoice_date":"2026-10-20","due_date":"2026-10-21"}"#),
        &HashMap::new(),
        context(Some(41)),
    );
    assert_eq!(valid.status, 200, "{}", valid.body);
    assert_eq!(valid.body, "41");

    let forged = dispatch_api_with_capabilities(
        &lowered_program,
        api,
        "check",
        &request(
            r#"{"amount":100,"invoice_date":"2026-10-20","due_date":"2026-10-21","tenant_id":42}"#,
        ),
        &HashMap::new(),
        context(Some(41)),
    );
    assert_eq!(forged.status, 400);

    let invalid = dispatch_api_with_capabilities(
        &lowered_program,
        api,
        "check",
        &request(r#"{"amount":100,"invoice_date":"2026-10-20","due_date":"2026-10-19"}"#),
        &HashMap::new(),
        context(Some(41)),
    );
    assert_eq!(invalid.status, 422);
    assert!(invalid
        .body
        .contains("Due date must not precede invoice date."));

    let missing_context = dispatch_api_with_capabilities(
        &lowered_program,
        api,
        "check",
        &request(r#"{"amount":100,"invoice_date":"2026-10-20","due_date":"2026-10-21"}"#),
        &HashMap::new(),
        context(None),
    );
    assert_eq!(missing_context.status, 403);
}

#[test]
fn project_step_budget_applies_to_executed_api_handlers() {
    let program = parse(
        &lex(
            "api GET \"/bounded\" { handler bounded output Int } fn bounded() -> Int { loop { } } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    assert!(check_apis(&program).is_ok());
    let request =
        zelyra_web::parse_request("GET /bounded HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let response = dispatch_api_with_capabilities(
        &program,
        &program.apis[0],
        "bounded",
        &request,
        &HashMap::new(),
        ApiRuntimeContext {
            database_url: None,
            capability_grants: None,
            runtime_policy: Some(&RuntimePolicy {
                step_budget: Some(30),
                ..RuntimePolicy::default()
            }),
            table_validation_functions: &[],
            computed_column_functions: &[],
            verified_tenant_id: None,
        },
    );
    assert_eq!(response.status, 500);
    assert!(response
        .body
        .contains("execution step budget exceeded (limit 30)"));
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
fn checks_crud_child_foreign_key_order_and_numeric_aggregates() {
    let source = r#"
        table invoices {
            id: Id primary auto
            net_amount: Decimal required default 0
            tax_amount: Decimal required default 0
        }
        table invoice_items {
            id: Id primary auto
            invoice: Invoice required on_delete cascade
            position: Int required
            total_net: Decimal required
            total_gross: Decimal required
        }
        table invoice_attachments {
            id: Id primary auto
            invoice: Invoice required on_delete cascade
            file_name: String(255) required
        }
        crud Invoice -> invoices {
            has_many items: InvoiceItem {
                foreign_key: invoice_id
                order_by: position asc
                cascade: delete
                aggregates {
                    net_amount: sum(total_net)
                    tax_amount: sum(total_gross - total_net)
                }
            }
            has_many attachments: InvoiceAttachment {
                foreign_key: invoice_id
                cascade: delete
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(validate_cruds("invoice.zyl", &program, &schema));
    let crud = &program.cruds[0];
    let parent = program
        .tables
        .iter()
        .find(|table| table.name == "invoices")
        .unwrap();
    let generated = generated_crud_form(
        &program,
        crud,
        parent,
        &schema,
        false,
        CrudGenerationContext {
            layout_html: None,
            before_form_html: None,
            after_form_html: None,
            field_content_html: HashMap::new(),
            csrf: CsrfProtection::new("master-detail-test-token"),
            membership_table: None,
            audit_table: None,
            audit_chain: false,
        },
    );
    assert_eq!(generated.child_collections.len(), 2);
    let child_form = generated.child_collections.first().unwrap();
    assert_eq!(child_form.definition.name, "items");
    assert_eq!(
        child_form
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["position", "total_net", "total_gross"]
    );
    assert!(generated
        .form
        .fields
        .iter()
        .all(|field| field.name != "net_amount" && field.name != "tax_amount"));
    assert_eq!(
        generated.child_collections[0].count_key,
        "_items_zelyra_child_count"
    );
    assert_eq!(
        generated.child_collections[1].definition.name,
        "attachments"
    );
    assert_eq!(
        generated.child_collections[1].count_key,
        "_attachments_zelyra_child_count"
    );

    let invalid_source = source.replace("foreign_key: invoice_id", "foreign_key: missing");
    let program = parse(&lex(&invalid_source).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(!validate_cruds("invoice.zyl", &program, &schema));

    let duplicate_child_table_source = source.replace(
        "has_many attachments: InvoiceAttachment",
        "has_many attachments: InvoiceItem",
    );
    let program = parse(&lex(&duplicate_child_table_source).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(!validate_cruds("invoice.zyl", &program, &schema));

    let duplicate_child_name_source = source.replace(
        "has_many attachments: InvoiceAttachment",
        "has_many items: InvoiceAttachment",
    );
    let tokens = lex(&duplicate_child_name_source).unwrap();
    assert!(parse(&tokens).is_err());

    let invalid_cascade = source.replace("cascade: delete", "cascade: restrict");
    let program = parse(&lex(&invalid_cascade).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(!validate_cruds("invoice.zyl", &program, &schema));

    let invalid_aggregate = source.replace("sum(total_net)", "sum(description)");
    let program = parse(&lex(&invalid_aggregate).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(!validate_cruds("invoice.zyl", &program, &schema));
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
        &program,
        crud,
        table,
        &schema,
        false,
        CrudGenerationContext {
            layout_html: None,
            before_form_html: None,
            after_form_html: None,
            field_content_html: HashMap::new(),
            csrf: CsrfProtection::new("test-csrf"),
            membership_table: None,
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
fn tenant_crud_forms_hide_and_server_bind_tenant_key() {
    let source = r#"
        table users {
            id: Id primary auto
            email: Email required
            password_hash: String(255) required
        }
        table auth_sessions {
            id: Id primary auto
            user_id: Id required
            token_hash: String(64) required
            expires_at: Timestamp required
        }
        table memberships {
            id: Id primary auto
            user_id: Id required
            tenant_id: Id required
            active: Bool required
        }
        table invoices {
            id: Id primary auto
            tenant_id: Id required
            title: String(100) required
        }
        auth users {
            table: users
            sessions: auth_sessions
            memberships: memberships
        }
        crud Invoice -> invoices {
            tenant: tenant_id
            requires auth
            view { fields { title } }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(validate_auth("tenant.zyl", &program, &schema));
    assert!(validate_cruds("tenant.zyl", &program, &schema));
    let crud = &program.cruds[0];
    let table = program
        .tables
        .iter()
        .find(|table| table.name == "invoices")
        .unwrap();
    let make_form = |edit| {
        generated_crud_form(
            &program,
            crud,
            table,
            &schema,
            edit,
            CrudGenerationContext {
                layout_html: None,
                before_form_html: None,
                after_form_html: None,
                field_content_html: HashMap::new(),
                csrf: CsrfProtection::new("tenant-test-csrf"),
                membership_table: Some("memberships".into()),
                audit_table: None,
                audit_chain: false,
            },
        )
    };
    let create = make_form(false);
    assert_eq!(
        create
            .form
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["title"]
    );
    let create_sql = match &create.form.actions[0].statements[0] {
        zelyra_ast::Stmt::Expr(expression) => match &expression.kind {
            zelyra_ast::ExprKind::Sql { query, .. } => query,
            _ => panic!("generated create form must use SQL"),
        },
        _ => panic!("generated create form must use SQL"),
    };
    assert!(create_sql.contains("`title`, `tenant_id`"), "{create_sql}");
    assert!(create_sql.contains(":zelyra_tenant_id"), "{create_sql}");
    assert!(!create_sql.contains("memberships"), "{create_sql}");
    assert!(!create_sql.contains("active = true"), "{create_sql}");
    assert!(!create_sql.contains(":zelyra_user_id"), "{create_sql}");
    // Tenant membership/ownership is revalidated in the same write
    // transaction, using the project's actual auth schema. Do not bake an
    // optional membership `active` column into generated mutation SQL.

    let edit = make_form(true);
    let edit_sql = match &edit.form.actions[0].statements[0] {
        zelyra_ast::Stmt::Expr(expression) => match &expression.kind {
            zelyra_ast::ExprKind::Sql { query, .. } => query,
            _ => panic!("generated edit form must use SQL"),
        },
        _ => panic!("generated edit form must use SQL"),
    };
    assert!(edit_sql.contains("WHERE `id` = :id AND `tenant_id` = :zelyra_tenant_id"));
    assert!(!edit_sql.contains("memberships"), "{edit_sql}");
    assert!(!edit_sql.contains("active = true"), "{edit_sql}");
}

#[test]
fn tenant_crud_rejects_unscoped_custom_actions_and_missing_memberships() {
    let source = r#"
        table users { id: Id primary auto email: Email required password_hash: String(255) required }
        table auth_sessions { id: Id primary auto user_id: Id required token_hash: String(64) required expires_at: Timestamp required }
        table invoices { id: Id primary auto tenant_id: Id required title: String(100) required }
        auth users { table: users sessions: auth_sessions }
        crud Invoice -> invoices {
            tenant: tenant_id
            requires auth
            action mark_paid {
                field title: String { required }
                sql {
                    UPDATE invoices SET title = :title WHERE id = :id
                }
            }
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(!validate_cruds("tenant.zyl", &program, &schema));
}

#[test]
fn notification_crud_requires_nullable_timestamp_for_unread_marker() {
    let source = r#"
        table notifications {
            id: Id primary auto
            title: String(160) required
            read_at: String(40) required
            expires_at: Timestamp?
        }
        crud Notification -> notifications {
            unread: read_at
            expires: expires_at
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(!validate_cruds("notifications.zyl", &program, &schema));
}

#[test]
fn notification_read_receipts_require_database_authentication() {
    let source = r#"
        table notifications {
            id: Id primary auto
            read_at: Timestamp?
        }
        crud Notification -> notifications {
            unread: read_at
        }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(!validate_cruds("notifications.zyl", &program, &schema));
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
fn auth_login_enabled_requires_a_required_boolean_user_column() {
    let valid = r#"
        table users {
            id: Id primary auto
            email: String(255) required unique
            password_hash: String(255) required
            is_approved: Bool required default true
        }
        auth users { table: users login_enabled: is_approved }
        fn main() {}
    "#;
    let program = parse(&lex(valid).unwrap()).unwrap();
    let schema = build_schema(&program).unwrap();
    assert!(validate_auth("auth.zyl", &program, &schema));

    for field in ["is_approved: Bool?", "is_approved: String(10) required"] {
        let source = format!(
            "table users {{ id: Id primary auto email: String(255) required unique password_hash: String(255) required {field} }} auth users {{ table: users login_enabled: is_approved }} fn main() {{}}"
        );
        let program = parse(&lex(&source).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        assert!(!validate_auth("auth.zyl", &program, &schema));
    }
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
fn decimal_api_inputs_use_decimal_values_without_float_conversion() {
    let program = parse(&lex("fn main() {}\n").unwrap()).unwrap();
    let value: Value = serde_json::from_str("9999999999999999.99").unwrap();
    let decoded = api_value_json(&value, &Type::Decimal, &program).unwrap();
    assert!(matches!(decoded, RuntimeValue::Decimal(_)));
    assert_eq!(decoded.output(), "9999999999999999.99");
    let value: Value = serde_json::from_str("0.000000001").unwrap();
    let decoded = api_value_json(&value, &Type::Decimal, &program).unwrap();
    assert_eq!(decoded.output(), "0.000000001");
}

#[test]
fn nominal_id_api_inputs_use_integer_runtime_values() {
    let program = parse(&lex("fn main() {}\n").unwrap()).unwrap();
    for value in [serde_json::json!(42), serde_json::json!("42")] {
        assert!(matches!(
            api_value_json(&value, &Type::Named("Id".into()), &program),
            Ok(RuntimeValue::Int(42))
        ));
    }
    for value in [serde_json::json!(true), serde_json::json!("not-an-id")] {
        assert!(api_value_json(&value, &Type::Named("Id".into()), &program).is_err());
    }
}

#[test]
fn accepts_password_reset_schema_with_audit_and_single_use_token_storage() {
    let source = r#"
        auth users {
            table: users
            reset_tokens: password_resets
            reset_rate_limits: password_reset_limits
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

        table password_reset_limits {
            bucket_key: String(64) primary
            window_started: Timestamp required
            request_count: Int required
            blocked_until: Timestamp?

            index {
                window_started
            }
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
    let mut missing_rate_table = program.clone();
    missing_rate_table.auth[0].reset_rate_limits_table = None;
    assert!(!validate_auth("test.zyl", &missing_rate_table, &schema));
    let mut orphan_rate_table = program.clone();
    orphan_rate_table.auth[0].reset_tokens_table = None;
    assert!(!validate_auth("test.zyl", &orphan_rate_table, &schema));
    let mut conflicting_rate_table = program.clone();
    conflicting_rate_table.auth[0].reset_rate_limits_table = Some("users".into());
    assert!(!validate_auth("test.zyl", &conflicting_rate_table, &schema));
    let mut malformed_rate_schema = schema.clone();
    malformed_rate_schema
        .tables
        .iter_mut()
        .find(|table| table.name == "password_reset_limits")
        .unwrap()
        .columns
        .iter_mut()
        .find(|column| column.name == "request_count")
        .unwrap()
        .sql_type = "varchar(20)".into();
    assert!(!validate_auth("test.zyl", &program, &malformed_rate_schema));
    let mut unindexed_rate_schema = schema.clone();
    unindexed_rate_schema
        .tables
        .iter_mut()
        .find(|table| table.name == "password_reset_limits")
        .unwrap()
        .indexes
        .clear();
    assert!(!validate_auth("test.zyl", &program, &unindexed_rate_schema));
    let mut auto_rate_schema = schema.clone();
    auto_rate_schema
        .tables
        .iter_mut()
        .find(|table| table.name == "password_reset_limits")
        .unwrap()
        .columns
        .iter_mut()
        .find(|column| column.name == "window_started")
        .unwrap()
        .auto = true;
    assert!(!validate_auth("test.zyl", &program, &auto_rate_schema));
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
fn project_runtime_policy_reads_and_bounds_max_stack_depth() {
    let directory = std::env::temp_dir().join(format!(
        "zelyra-runtime-policy-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let entry = directory.join("main.zyl");
    fs::write(&entry, "fn main() {}\n").unwrap();
    fs::write(
        directory.join("zelyra.toml"),
        "[project]\nname = \"runtime-policy\"\n\n[runtime]\nmax_stack_depth = 500\nmax_steps = 12000\nscheduler_job_timeout_seconds = 900\n",
    )
    .unwrap();
    let policy = project_runtime_policy(entry.to_str().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(policy.max_stack_depth, Some(500));
    assert_eq!(policy.step_budget, Some(12000));
    let limits = project_runtime_limits(entry.to_str().unwrap()).unwrap();
    assert_eq!(limits.scheduler_job_timeout_seconds, Some(900));

    fs::write(directory.join("zelyra.toml"), "[runtime]\nmax_steps = 0\n").unwrap();
    let error = project_runtime_policy(entry.to_str().unwrap()).unwrap_err();
    assert!(error.contains("max_steps must be a positive integer"));

    fs::write(
        directory.join("zelyra.toml"),
        "[runtime]\nmax_steps = 10\nmax_steps = 20\n",
    )
    .unwrap();
    let error = project_runtime_policy(entry.to_str().unwrap()).unwrap_err();
    assert!(error.contains("max_steps is configured more than once"));

    fs::write(
        directory.join("zelyra.toml"),
        "[runtime]\nmax_stack_depth = 501\n",
    )
    .unwrap();
    let error = project_runtime_policy(entry.to_str().unwrap()).unwrap_err();
    assert!(error.contains("from 1 through 500"));

    fs::write(
        directory.join("zelyra.toml"),
        "[runtime]\nscheduler_job_timeout_seconds = 86401\n",
    )
    .unwrap();
    let error = project_runtime_policy(entry.to_str().unwrap()).unwrap_err();
    assert!(error.contains("scheduler_job_timeout_seconds must be an integer from 1 through 86400"));
    fs::write(
        directory.join("zelyra.toml"),
        "[runtime]\nscheduler_job_timeout_seconds = 30\nscheduler_job_timeout_seconds = 60\n",
    )
    .unwrap();
    let error = project_runtime_policy(entry.to_str().unwrap()).unwrap_err();
    assert!(error.contains("scheduler_job_timeout_seconds is configured more than once"));
    fs::remove_dir_all(directory).unwrap();
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
            canonical_layout: false,
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
    assert!(dockerfile.contains("COPY . ./"));
    let compose = fs::read_to_string(path.join("docker-compose.mariadb.yml")).unwrap();
    assert!(compose.contains("ZELYRA_UI_STYLESHEET: ${ZELYRA_UI_STYLESHEET:-zelyra}"));
    assert!(path.join("public/.keep").is_file());
    assert!(path.join("src/.keep").is_file());
    assert!(path.join("locales/de.json").is_file());
    assert!(path.join("locales/en.json").is_file());
    for token in [
        "--z-color-primary",
        "--z-color-on-primary",
        "--z-color-primary-hover",
        "--z-color-primary-soft",
        "--z-color-success",
        "--z-color-warning",
        "--z-color-danger",
        "--z-color-canvas",
        "--z-color-surface",
        "--z-color-text",
        "--z-color-text-muted",
        "--z-color-border",
        "--z-color-focus",
        "--z-content-width",
        "--z-radius-md",
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
            canonical_layout: false,
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
fn canonical_layout_generates_checkable_database_templates() {
    for (template, crud_template, auth_template, business_template) in [
        ("mariadb", false, false, false),
        ("crud", true, false, false),
        ("auth", false, true, false),
        ("business", false, false, true),
    ] {
        let path = env::temp_dir().join(format!(
            "zelyra-cli-canonical-{template}-{}-{}",
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
                crud_template,
                auth_template,
                business_template,
                canonical_layout: true,
                web_port: DEFAULT_WEB_PORT,
                host_port: DEFAULT_WEB_PORT,
                database_host_port: DEFAULT_DATABASE_HOST_PORT,
                host_port_given: false,
                database_host_port_given: false,
            },
        );
        assert_eq!(status, ExitCode::SUCCESS, "failed to generate {template}");
        assert!(path.join("schema/tables.zyl").is_file());
        assert!(path.join("routes/web.zyl").is_file());
        assert!(path.join("domain/dashboard/actions.zyl").is_file());
        assert!(!path.join("src/database.zyl").exists());
        let schema = fs::read_to_string(path.join("schema/tables.zyl")).unwrap();
        assert!(
            schema.contains("database main"),
            "{template} schema misses database declaration"
        );
        if template != "mariadb" {
            assert!(
                schema.contains("table "),
                "{template} schema misses table declarations"
            );
        }
        validate(path.join("main.zyl").to_str().unwrap()).unwrap_or_else(|()| {
            panic!("generated canonical {template} project failed validation")
        });
        fs::remove_dir_all(path).unwrap();
    }
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
                canonical_layout: false,
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
    assert!(env_example.contains("ZELYRA_SMTP_HOST=127.0.0.1"));
    assert!(env_example.contains("ZELYRA_SMTP_SECURITY=local_plaintext"));
    assert!(env_example.contains("never point local tests at real recipients"));
    assert!(
        compose.contains("ZELYRA_DB_CONNECT_TIMEOUT_SECS: ${ZELYRA_DB_CONNECT_TIMEOUT_SECS:-10}")
    );
    assert!(compose.contains("ZELYRA_DB_QUERY_TIMEOUT_SECS: ${ZELYRA_DB_QUERY_TIMEOUT_SECS:-30}"));
    assert!(compose.contains("ZELYRA_DB_POOL_MAX_SIZE: ${ZELYRA_DB_POOL_MAX_SIZE:-8}"));
    assert!(compose
        .contains("ZELYRA_DB_POOL_WAIT_TIMEOUT_SECS: ${ZELYRA_DB_POOL_WAIT_TIMEOUT_SECS:-10}"));
    assert!(compose.contains("ZELYRA_DB_TLS_MODE: ${ZELYRA_DB_TLS_MODE:-disabled}"));
    assert!(compose.contains("ZELYRA_DB_TLS_CA_CERT_FILE: ${ZELYRA_DB_TLS_CA_CERT_FILE:-}"));
    assert!(compose.contains("ZELYRA_SMTP_SECURITY: ${ZELYRA_SMTP_SECURITY:-}"));
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
    assert!(gitignore.contains(".zelyra/schedules.cron"));

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
fn schema_setup_retries_only_transient_database_startup_errors() {
    assert!(schema_setup_error_is_transient(
        "DriverError { error: IoError(ConnectionRefused) }"
    ));
    assert!(schema_setup_error_is_transient(
        "Database connection timed out while MariaDB was starting"
    ));
    assert!(schema_setup_error_is_transient(
        "error[E-DB-005]: MySqlError { ERROR 1045 (28000): Access denied for user 'zelyra'@'172.20.0.1' (using password: YES) }"
    ));
    assert!(schema_setup_error_is_transient(
        "error[E-DB-005]: MySqlError { ERROR 1049 (42000): Unknown database 'invoice_dev' }"
    ));
    assert!(schema_setup_error_is_transient(
        "Access denied for user zelyra"
    ));
    assert!(!schema_setup_error_is_transient(
        "error[E-DB-004]: schema changes requiring review were refused"
    ));
    assert!(!schema_setup_error_is_transient(
        "error[E-DB-005]: cannot require `users.email` because existing rows contain NULL values"
    ));
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

    let first = setup_action(
        path.to_str().unwrap(),
        "prepare",
        &SetupOptions::default(),
        false,
    )
    .unwrap();
    let contents = fs::read_to_string(path.join(".env")).unwrap();
    let second = setup_action(
        path.to_str().unwrap(),
        "prepare",
        &SetupOptions::default(),
        false,
    )
    .unwrap();
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
    assert!(hint.contains("Docker Compose v2 or newer is required"));
    assert!(hint.contains("https://docs.docker.com/"));
    assert!(hint.contains("docker compose version"));
    assert!(hint.contains("asks whether to install"));
}

#[test]
fn compose_install_prompt_uses_only_supported_platform_package_managers() {
    let apt = compose_package_install_command_for("linux", &HashSet::from(["apt-get"]))
        .expect("apt-get should have a supported Linux Compose package");
    assert_eq!(apt.0, "apt-get");
    assert_eq!(apt.1, ["install", "-y", "docker-compose-plugin"]);
    assert!(apt.2);

    let brew = compose_package_install_command_for("macos", &HashSet::from(["brew"]))
        .expect("Homebrew should have a supported Compose package");
    assert_eq!(brew.0, "brew");
    assert_eq!(brew.1, ["install", "docker-compose"]);
    assert!(!brew.2);

    let winget = compose_package_install_command_for("windows", &HashSet::from(["winget"]))
        .expect("winget should offer Docker Desktop, including Compose v2");
    assert_eq!(winget.0, "winget");
    assert_eq!(
        winget.1,
        ["install", "--id", "Docker.DockerDesktop", "--exact"]
    );
    assert!(!winget.2);

    assert!(compose_package_install_command_for("linux", &HashSet::new()).is_none());
    assert!(compose_package_install_command_for("unknown", &HashSet::from(["apt-get"])).is_none());
}

#[test]
fn compose_start_errors_are_actionable_without_exposing_output() {
    let secret = "mariadb://zelyra:secret-value@127.0.0.1:3306/zelyra_app";
    let permission = compose_start_failure_message(
        DockerComposeCommand::Standalone,
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

#[test]
fn decimal_api_docs_describe_exact_string_outputs() {
    assert_eq!(docs::typescript_type(&Type::Decimal), "string");
    let schema = zelyra_ast::json::parse(&docs::openapi_schema(&Type::Decimal)).unwrap();
    assert_eq!(schema["type"], "string");
    assert_eq!(schema["format"], "decimal");
}

#[test]
fn precision_diagnostics_locate_the_mismatched_alias_column() {
    let program = parse(&lex("type Amount = Decimal table bills { id: Id primary amount: Amount required quantity: UInt required }").unwrap()).unwrap();
    let message = "fixed-point storage for `bills.amount` must match DECIMAL(39,9)";
    assert_eq!(
        decimal_storage_error_span(&program, message),
        program.tables[0].columns[1].span
    );
}

#[test]
fn schema_setup_runs_independently_of_web_server_readiness() {
    let args = container_schema_setup_args();
    assert_eq!(args[0], "run");
    assert!(args.contains(&"--rm"));
    assert!(args.contains(&"--no-deps"));
    assert_eq!(
        &args[args.len() - 4..],
        ["zelyra", "db", "apply", "main.zyl"]
    );
}

#[test]
fn temporal_api_inputs_produce_validated_runtime_types() {
    let program = parse(&lex("fn main() {}").unwrap()).unwrap();
    let date = api_value_json(&serde_json::json!("2000-02-29"), &Type::Date, &program).unwrap();
    assert!(matches!(date, RuntimeValue::Date(value) if value == "2000-02-29"));
    let time = api_value_json(&serde_json::json!("23:59:59"), &Type::Time, &program).unwrap();
    assert!(matches!(time, RuntimeValue::Time(value) if value == "23:59:59"));
    let timestamp = api_value_json(&serde_json::json!(-1), &Type::Timestamp, &program).unwrap();
    assert!(matches!(timestamp, RuntimeValue::Timestamp(-1)));
    for (ty, value) in [
        (Type::Date, serde_json::json!("1900-02-29")),
        (Type::Time, serde_json::json!("24:00:00")),
        (Type::Timestamp, serde_json::json!(253402300800000_i64)),
        (Type::Duration, serde_json::json!(true)),
    ] {
        assert!(api_value_json(&value, &ty, &program).is_err());
    }
}

#[test]
fn temporal_client_schemas_match_existing_wire_formats() {
    assert_eq!(docs::typescript_type(&Type::Timestamp), "number");
    assert_eq!(docs::typescript_type(&Type::Duration), "number");
    assert_eq!(docs::typescript_type(&Type::Date), "string");
    let date_schema = zelyra_ast::json::parse(&docs::openapi_schema(&Type::Date)).unwrap();
    assert_eq!(date_schema["format"], "date");
    let timestamp_schema =
        zelyra_ast::json::parse(&docs::openapi_schema(&Type::Timestamp)).unwrap();
    assert_eq!(timestamp_schema["type"], "integer");
}

#[test]
fn compose_versions_reject_python_v1_and_accept_current_compose() {
    for version in ["1.29.2", "v1.29.2", "", "unknown"] {
        assert!(!supported_compose_version(version), "{version}");
    }
    for version in [
        "2.0.0",
        "v2.29.1\n",
        "2.40",
        "Docker Compose version v2.0.0",
        "2.40.0-desktop.1",
        "2.40.3+ds1-0ubuntu1~24.04.1",
        "docker: 2.40.3+ds1-0ubuntu1~24.04.1",
        "v5.6.0",
    ] {
        assert!(supported_compose_version(version), "{version}");
    }
}

#[test]
fn stale_database_credentials_are_recovered_only_for_legacy_invoice_tutorials() {
    let directory = std::env::temp_dir().join(format!(
        "zelyra-invoice-recovery-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("zelyra.tutorial"), "invoice-v1\n").unwrap();
    fs::write(
        directory.join("docker-compose.mariadb.yml"),
        "services:\n  mariadb:\n",
    )
    .unwrap();
    let auth_error = "error[E-DB-002]: ERROR 1045 (28000): Access denied for user";

    assert!(invoice_tutorial_database_recovery_required(
        &directory, auth_error
    ));
    assert!(!invoice_tutorial_database_recovery_required(
        &directory,
        "schema mismatch"
    ));

    fs::write(
        directory.join("docker-compose.mariadb.yml"),
        "name: zelyra-invoice-existing\nservices:\n  mariadb:\n",
    )
    .unwrap();
    assert!(!invoice_tutorial_database_recovery_required(
        &directory, auth_error
    ));

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn compose_containerconfig_with_apport_failure_is_not_a_socket_permission_error() {
    let message = compose_start_failure_message(
        DockerComposeCommand::Standalone,
        "KeyError: 'ContainerConfig'\nPermissionError: [Errno 13] Permission denied: '/var/crash/_usr_bin_docker-compose.1000.crash'",
    );
    assert!(message.contains("ContainerConfig"));
    assert!(message.contains("v2 or newer"));
    assert!(!message.contains("usermod"));
    let crash_only = compose_start_failure_message(
        DockerComposeCommand::Standalone,
        "PermissionError: [Errno 13] Permission denied: '/var/crash/_usr_bin_docker-compose.1000.crash'",
    );
    assert!(!crash_only.contains("usermod"));
}
