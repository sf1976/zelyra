use super::*;
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn number_series_formats_render_literal_year_and_padded_counter() {
    assert_eq!(
        render_number_series_value("KD-{0001}", 7, "2026").as_deref(),
        Some("KD-0007")
    );
    assert_eq!(
        render_number_series_value("AG-{year}-{0001}", 42, "2026").as_deref(),
        Some("AG-2026-0042")
    );
    assert!(render_number_series_value("KD-{counter}", 1, "2026").is_none());
    assert!(render_number_series_value("KD-}", 1, "2026").is_none());
}

#[test]
#[ignore = "requires ZELYRA_TEST_DATABASE_URL pointing to a disposable MariaDB or PostgreSQL database"]
fn number_series_allocator_is_atomic_and_scoped_per_tenant() {
    let database_url = std::env::var("ZELYRA_TEST_DATABASE_URL")
        .expect("the ignored integration test requires a disposable database URL");
    let is_postgres = zelyra_database::is_postgres_url(&database_url);
    let suffix = std::process::id();
    let table = format!("zelyra_series_test_{suffix}");
    let series = format!("{table}.number");
    zelyra_database::execute_query(
        &database_url,
        if is_postgres {
            "CREATE TABLE IF NOT EXISTS \"_zelyra_number_sequences\" (\"series_key\" VARCHAR(255) NOT NULL, \"scope_key\" VARCHAR(255) NOT NULL, \"next_value\" BIGINT NOT NULL, CONSTRAINT \"uq_zelyra_number_series_scope\" UNIQUE (\"series_key\", \"scope_key\"))"
        } else {
            "CREATE TABLE IF NOT EXISTS `_zelyra_number_sequences` (`series_key` VARCHAR(191) NOT NULL, `scope_key` VARCHAR(191) NOT NULL, `next_value` BIGINT NOT NULL, CONSTRAINT `uq_zelyra_number_series_scope` UNIQUE (`series_key`, `scope_key`)) ENGINE=InnoDB"
        },
        Vec::new(),
    )
    .unwrap();
    zelyra_database::execute_query(
        &database_url,
        &if is_postgres {
            format!("CREATE TABLE {} ({} BIGSERIAL PRIMARY KEY, {} BIGINT NOT NULL, {} VARCHAR(32) NOT NULL, CONSTRAINT {} UNIQUE ({}, {}))", quote_ident(&table, true), quote_ident("id", true), quote_ident("tenant_id", true), quote_ident("number", true), quote_ident("uq_test_tenant_number", true), quote_ident("tenant_id", true), quote_ident("number", true))
        } else {
            format!("CREATE TABLE {} ({} BIGINT PRIMARY KEY AUTO_INCREMENT, {} BIGINT NOT NULL, {} VARCHAR(32) NOT NULL, CONSTRAINT {} UNIQUE ({}, {})) ENGINE=InnoDB", quote_ident(&table, false), quote_ident("id", false), quote_ident("tenant_id", false), quote_ident("number", false), quote_ident("uq_test_tenant_number", false), quote_ident("tenant_id", false), quote_ident("number", false))
        },
        Vec::new(),
    )
    .unwrap();

    let mut route = form_route();
    route.form.table = Some(table.clone());
    route.form.fields = vec![zelyra_ast::FormField {
        name: "number".into(),
        ty: Some(Type::String),
        label: None,
        placeholder: None,
        required: false,
        min_length: None,
        max: None,
        min_value: None,
        max_value: None,
        choices: Vec::new(),
        widget: None,
        appearance: None,
        file_upload: None,
        readonly: true,
        span: zelyra_ast::Span::default(),
    }];
    route.table = Some(zelyra_ast::TableDef {
        name: table.clone(),
        columns: vec![zelyra_ast::ColumnDef {
            name: "number".into(),
            ty: Type::String,
            length: Some(32),
            storage_name: None,
            decimal_precision: None,
            auto_sequence: Some("KD-{0001}".into()),
            computed: None,
            attachment: None,
            required: true,
            primary_key: false,
            auto: false,
            unique: false,
            unique_within_tenant: false,
            default: None,
            references: None,
            on_delete: None,
            span: zelyra_ast::Span::default(),
        }],
        validations: Vec::new(),
        indexes: Vec::new(),
        uniques: Vec::new(),
        access: zelyra_ast::TableAccessDef::default(),
        tenant_scope: None,
        span: zelyra_ast::Span::default(),
    });
    let action = zelyra_ast::FormAction {
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
        capabilities: Vec::new(),
        require_match: false,
        statements: vec![zelyra_ast::Stmt::Expr(zelyra_ast::Expr {
            kind: zelyra_ast::ExprKind::Sql {
                result_type: Type::Unit,
                query: format!(
                    "INSERT INTO {} ({}, {}) VALUES (1, :number)",
                    quote_ident(&table, is_postgres),
                    quote_ident("tenant_id", is_postgres),
                    quote_ident("number", is_postgres)
                ),
            },
            span: zelyra_ast::Span::default(),
        })],
        execute_body: None,
        guard: None,
        success: None,
        redirect: None,
        span: zelyra_ast::Span::default(),
    };
    execute_form_action_with_children(
        &route,
        &action,
        &HashMap::from([("number".into(), "FORGED".into())]),
        &HashMap::new(),
        &HashMap::new(),
        &database_url,
        FormExecutionContext {
            actor_user_id: None,
            expected_snapshot: None,
            tenant_context: None,
            validation_program: None,
            table_validation_functions: &[],
            crud_policy_functions: &[],
            crud_action_functions: &[],
            capability_grants: None,
            runtime_policy: None,
            granted_permissions: &[],
        },
    )
    .unwrap();
    let inserted = zelyra_database::execute_query(
        &database_url,
        &format!(
            "SELECT {} FROM {} WHERE {} = 1",
            quote_ident("number", is_postgres),
            quote_ident(&table, is_postgres),
            quote_ident("tenant_id", is_postgres)
        ),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(inserted.rows, [vec![String::from("KD-0001")]]);

    let allocate = |tenant_id: i64| {
        zelyra_database::with_decision_transaction(&database_url, |transaction| {
            allocate_number_series(
                transaction,
                is_postgres,
                &series,
                &format!("tenant:{tenant_id}"),
                "KD-{0001}",
                &NumberSeriesTarget {
                    table: &table,
                    column: "number",
                    tenant_column: Some("tenant_id"),
                    tenant_id: Some(tenant_id),
                },
                0,
            )
        })
        .unwrap()
    };

    assert_eq!(allocate(41), "KD-0001");
    assert_eq!(allocate(41), "KD-0002");
    assert_eq!(allocate(42), "KD-0001");
    let allocate_year = |tenant_id: i64| {
        zelyra_database::with_decision_transaction(&database_url, |transaction| {
            allocate_number_series(
                transaction,
                is_postgres,
                &format!("{series}.yearly"),
                &format!("tenant:{tenant_id}"),
                "AG-{year}-{0001}",
                &NumberSeriesTarget {
                    table: &table,
                    column: "number",
                    tenant_column: Some("tenant_id"),
                    tenant_id: Some(tenant_id),
                },
                0,
            )
        })
        .unwrap()
    };
    let yearly_first = allocate_year(41);
    let yearly_second = allocate_year(41);
    let yearly_prefix = yearly_first.rsplit_once('-').unwrap().0;
    assert!(yearly_prefix.starts_with("AG-"));
    assert_eq!(yearly_first, format!("{yearly_prefix}-0001"));
    assert_eq!(yearly_second, format!("{yearly_prefix}-0002"));
    let mut rolled_back_yearly_number = String::new();
    let rollback_yearly: std::result::Result<(), zelyra_database::DatabaseError> =
        zelyra_database::with_decision_transaction(&database_url, |transaction| {
            rolled_back_yearly_number = allocate_number_series(
                transaction,
                is_postgres,
                &format!("{series}.yearly"),
                "tenant:46",
                "AG-{year}-{0001}",
                &NumberSeriesTarget {
                    table: &table,
                    column: "number",
                    tenant_column: Some("tenant_id"),
                    tenant_id: Some(46),
                },
                0,
            )?;
            Err(zelyra_database::DatabaseError {
                message: "test rollback".into(),
            })
        });
    assert!(rollback_yearly.is_err());
    zelyra_database::execute_query(
        &database_url,
        &format!(
            "INSERT INTO {} ({}, {}) VALUES (:tenant_id, :number)",
            quote_ident(&table, is_postgres),
            quote_ident("tenant_id", is_postgres),
            quote_ident("number", is_postgres)
        ),
        vec![
            ("tenant_id".into(), QueryValue::Int(46)),
            (
                "number".into(),
                QueryValue::String(rolled_back_yearly_number.clone()),
            ),
        ],
    )
    .unwrap();
    let yearly_collision = allocate_year(46);
    assert_eq!(
        yearly_collision,
        format!(
            "{}-0002",
            rolled_back_yearly_number.rsplit_once('-').unwrap().0
        )
    );
    zelyra_database::execute_query(
        &database_url,
        &format!(
            "INSERT INTO {} ({}, {}) VALUES (45, 'KD-0001')",
            quote_ident(&table, is_postgres),
            quote_ident("tenant_id", is_postgres),
            quote_ident("number", is_postgres)
        ),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(allocate(45), "KD-0002");

    let rollback: std::result::Result<(), zelyra_database::DatabaseError> =
        zelyra_database::with_decision_transaction(&database_url, |transaction| {
            let value = allocate_number_series(
                transaction,
                is_postgres,
                &series,
                "tenant:43",
                "KD-{0001}",
                &NumberSeriesTarget {
                    table: &table,
                    column: "number",
                    tenant_column: Some("tenant_id"),
                    tenant_id: Some(43),
                },
                0,
            )?;
            assert_eq!(value, "KD-0001");
            Err(zelyra_database::DatabaseError {
                message: "test rollback".into(),
            })
        });
    assert!(rollback.is_err());
    assert_eq!(allocate(43), "KD-0001");

    let values = std::thread::scope(|scope_thread| {
        let handles = (0..8)
            .map(|_| scope_thread.spawn(|| allocate(44)))
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    let unique = values.iter().collect::<std::collections::HashSet<_>>();
    assert_eq!(unique.len(), 8);
    zelyra_database::execute_query(
        &database_url,
        "DELETE FROM `_zelyra_number_sequences` WHERE `series_key` = :series",
        vec![("series".into(), QueryValue::String(series))],
    )
    .unwrap();
    zelyra_database::execute_query(
        &database_url,
        &format!("DROP TABLE {}", quote_ident(&table, is_postgres)),
        Vec::new(),
    )
    .unwrap();
}

#[test]
fn database_timestamps_convert_to_zelyra_utc_milliseconds() {
    assert_eq!(
        timestamp_value("2026-10-20 04:25:38.161000"),
        zelyra_ast::temporal::parse_timestamp_input("2026-10-20T04:25:38.161Z")
    );
    assert_eq!(
        timestamp_value("2026-10-20 04:25:38"),
        zelyra_ast::temporal::parse_timestamp_input("2026-10-20T04:25:38Z")
    );
    assert_eq!(timestamp_value("not-a-timestamp"), None);
}

#[test]
fn form_route_parameters_bind_numeric_ids_and_opaque_tokens_safely() {
    assert!(matches!(
        path_parameter_query_value("42"),
        QueryValue::Int(42)
    ));
    assert!(matches!(
        path_parameter_query_value("tok_abc-123"),
        QueryValue::String(value) if value == "tok_abc-123"
    ));
}

#[test]
fn table_validation_predicate_runs_before_form_writes_and_returns_declared_message() {
    let source = r#"
        table invoices {
            invoice_date: Date required
            due_date: Date?
            validate {
                match due_date {
                    Some(due) => due >= invoice_date
                    None => true
                }: "Due date must not precede invoice date."
            }
        }
        fn main() {}
    "#;
    let program = zelyra_parser::parse(&zelyra_lexer::lex(source).unwrap()).unwrap();
    zelyra_runtime::check(&program).expect("the rule is statically checked");
    let (lowered, mappings) = zelyra_ast::lower_table_validation_functions(&program);

    let invalid = HashMap::from([
        ("invoice_date".to_owned(), "2026-10-20".to_owned()),
        ("due_date".to_owned(), "2026-10-19".to_owned()),
    ]);
    let errors = evaluate_table_validations(&lowered, &mappings, "invoices", &invalid).unwrap();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "Due date must not precede invoice date.");

    let valid = HashMap::from([
        ("invoice_date".to_owned(), "2026-10-20".to_owned()),
        ("due_date".to_owned(), "2026-10-21".to_owned()),
    ]);
    assert!(
        evaluate_table_validations(&lowered, &mappings, "invoices", &valid)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn computed_table_columns_are_recalculated_and_replace_submitted_values() {
    let source = r#"
        table invoice_items {
            quantity: Int required
            unit_price: Int required
            total_net: Int required compute(quantity * unit_price)
            total_gross: Int required compute(total_net + 2)
        }
        fn main() {}
    "#;
    let program = zelyra_parser::parse(&zelyra_lexer::lex(source).unwrap()).unwrap();
    zelyra_runtime::check(&program).expect("computed expressions are type checked");
    let mut lowered = program.clone();
    let (functions, mappings) = zelyra_ast::build_computed_column_functions(&program);
    lowered.functions.extend(functions);
    let mut values = HashMap::from([
        ("quantity".to_owned(), "3".to_owned()),
        ("unit_price".to_owned(), "125".to_owned()),
        ("total_net".to_owned(), "999999".to_owned()),
        ("total_gross".to_owned(), "999999".to_owned()),
    ]);

    evaluate_computed_columns(&lowered, &mappings, "invoice_items", &mut values)
        .expect("server-side computed fields should evaluate in dependency order");

    assert_eq!(values.get("total_net").map(String::as_str), Some("375"));
    assert_eq!(values.get("total_gross").map(String::as_str), Some("377"));

    let mut invalid = HashMap::from([
        ("quantity".to_owned(), "not-a-number".to_owned()),
        ("unit_price".to_owned(), "125".to_owned()),
    ]);
    let error = evaluate_computed_columns(&lowered, &mappings, "invoice_items", &mut invalid)
        .expect_err("invalid user input must be returned as a field error");
    assert!(error.is_user_input);
    assert_eq!(error.field, "quantity");
}

#[test]
fn computed_field_evaluation_preserves_decimal_input_for_storage_validation() {
    let source = r#"
        table invoice_items {
            amount: Decimal(12, 2) required
            squared: Decimal(12, 2) required compute(amount * amount)
        }
        fn main() {}
    "#;
    let program = zelyra_parser::parse(&zelyra_lexer::lex(source).unwrap()).unwrap();
    zelyra_runtime::check(&program).expect("decimal computed fields should type check");
    let schema = zelyra_database::build_schema(&program).unwrap();
    let (functions, mappings) = zelyra_ast::build_computed_column_functions(&program);
    let mut lowered = program.clone();
    lowered.functions.extend(functions);
    let mut values = HashMap::from([
        ("amount".to_owned(), "1.001".to_owned()),
        ("squared".to_owned(), "forged".to_owned()),
    ]);

    evaluate_computed_columns(&lowered, &mappings, "invoice_items", &mut values)
        .expect("computed output can be calculated before normal field validation");
    assert_eq!(values.get("amount").map(String::as_str), Some("1.001"));
    assert_eq!(values.get("squared").map(String::as_str), Some("1"));

    let mut input_form = zelyra_ast::FormDef {
        name: "InvoiceItemCreate".into(),
        table: Some("invoice_items".into()),
        path: None,
        post_only: false,
        verify: None,
        fields: vec![
            zelyra_ast::FormField {
                name: "amount".into(),
                ty: Some(Type::Decimal),
                label: None,
                placeholder: None,
                required: true,
                min_length: None,
                max: None,
                min_value: None,
                max_value: None,
                choices: Vec::new(),
                widget: None,
                appearance: None,
                file_upload: None,
                readonly: false,
                span: zelyra_ast::Span::default(),
            },
            zelyra_ast::FormField {
                name: "squared".into(),
                ty: Some(Type::Decimal),
                label: None,
                placeholder: None,
                required: true,
                min_length: None,
                max: None,
                min_value: None,
                max_value: None,
                choices: Vec::new(),
                widget: None,
                appearance: None,
                file_upload: None,
                readonly: false,
                span: zelyra_ast::Span::default(),
            },
        ],
        actions: Vec::new(),
        span: zelyra_ast::Span::default(),
    };
    input_form.fields[1].readonly = false;
    let validation =
        zelyra_forms::validate(&input_form, program.tables.first(), Some(&schema), &values);
    assert!(validation
        .errors
        .iter()
        .any(|error| error.field == "amount"));
}

#[test]
fn computed_field_invalid_input_returns_form_error_before_database_access() {
    let source = r#"
        table invoice_items {
            quantity: Int required
            unit_price: Int required
            total: Int required compute(quantity * unit_price)
        }
        fn main() {}
    "#;
    let program = zelyra_parser::parse(&zelyra_lexer::lex(source).unwrap()).unwrap();
    zelyra_runtime::check(&program).expect("computed expressions should type check");
    let (functions, mappings) = zelyra_ast::build_computed_column_functions(&program);
    let mut lowered = program.clone();
    lowered.functions.extend(functions);

    let mut route = form_route();
    route.form.table = Some("invoice_items".into());
    let field = |name: &str, ty: Type, required: bool| zelyra_ast::FormField {
        name: name.into(),
        ty: Some(ty),
        label: None,
        placeholder: None,
        required,
        min_length: None,
        max: None,
        min_value: None,
        max_value: None,
        choices: Vec::new(),
        widget: None,
        appearance: None,
        file_upload: None,
        readonly: false,
        span: zelyra_ast::Span::default(),
    };
    route.form.fields = vec![
        field("quantity", Type::Int, true),
        field("unit_price", Type::Int, true),
        field("total", Type::Int, false),
    ];
    route.form.fields[2].readonly = true;
    route.table = Some(program.tables[0].clone());

    let request = parse_request(
        "POST /forms/CustomerCreate HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\nContent-Type: application/x-www-form-urlencoded\r\n\r\n_zelyra_csrf=csrf-token&quantity=not-an-integer&unit_price=4&total=999",
    )
    .unwrap();
    let response = dispatch_form_with_language(
        &route,
        &request,
        &HashMap::new(),
        None,
        FormDispatchContext {
            actor_user_id: None,
            tenant_context: None,
            language: UiLanguage::English,
            validation_program: Some(&lowered),
            table_validation_functions: &[],
            computed_column_functions: &mappings,
            crud_policy_functions: &[],
            crud_action_functions: &[],
            capability_grants: None,
            runtime_policy: None,
            granted_permissions: Vec::new(),
            multipart_parts: &[],
            upload_storage_root: None,
        },
    );

    assert_eq!(response.status, 422);
    assert!(response.body.contains("quantity"));
    assert!(response.body.contains("not-an-integer"));
}

#[test]
fn computed_decimal_fields_round_half_away_from_zero_and_reject_precision_overflow() {
    let source = r#"
        table invoice_items {
            quantity: Decimal(10, 3) required
            unit_price: Decimal(10, 3) required
            total: Decimal(5, 2) required compute(quantity * unit_price)
        }
        fn main() {}
    "#;
    let program = zelyra_parser::parse(&zelyra_lexer::lex(source).unwrap()).unwrap();
    zelyra_runtime::check(&program).expect("decimal computed expression should type check");
    let (functions, mappings) = zelyra_ast::build_computed_column_functions(&program);
    let mut lowered = program.clone();
    lowered.functions.extend(functions);
    let mut values = HashMap::from([
        ("quantity".to_owned(), "3".to_owned()),
        ("unit_price".to_owned(), "0.385".to_owned()),
    ]);
    evaluate_computed_columns(&lowered, &mappings, "invoice_items", &mut values).unwrap();
    assert_eq!(values.get("total").map(String::as_str), Some("1.16"));

    let mut overflow = HashMap::from([
        ("quantity".to_owned(), "999".to_owned()),
        ("unit_price".to_owned(), "999".to_owned()),
    ]);
    assert!(
        evaluate_computed_columns(&lowered, &mappings, "invoice_items", &mut overflow)
            .unwrap_err()
            .message
            .contains("overflows Decimal(5, 2)")
    );
}

#[test]
fn computed_optional_fields_use_typed_option_matching_for_null_values() {
    let source = r#"
        table invoice_items {
            net_amount: Decimal?
            gross_amount: Decimal? compute(match net_amount {
                Some(net) => Some(net * decimal("2"))
                None => None
            })
        }
        fn main() {}
    "#;
    let program = zelyra_parser::parse(&zelyra_lexer::lex(source).unwrap()).unwrap();
    zelyra_runtime::check(&program).expect("optional computed fields are type checked");
    let (functions, mappings) = zelyra_ast::build_computed_column_functions(&program);
    let mut lowered = program.clone();
    lowered.functions.extend(functions);

    let mut values = HashMap::from([("net_amount".to_owned(), String::new())]);
    evaluate_computed_columns(&lowered, &mappings, "invoice_items", &mut values).unwrap();
    assert_eq!(values.get("net_amount").map(String::as_str), Some(""));
    assert_eq!(values.get("gross_amount").map(String::as_str), Some(""));

    let mut present = HashMap::from([("net_amount".to_owned(), "10.00".to_owned())]);
    evaluate_computed_columns(&lowered, &mappings, "invoice_items", &mut present).unwrap();
    assert_eq!(present.get("gross_amount").map(String::as_str), Some("20"));
}

#[test]
fn crud_file_download_fields_come_from_table_attachment_metadata() {
    let program = zelyra_parser::parse(
        &zelyra_lexer::lex(
            r#"table contracts { id: Id primary auto signed_pdf: FileRef? attachment {
                max_size: "20MB"
                allowed_types: ["application/pdf"]
            } } fn main() {}"#,
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        crud_file_fields(Some(&program), "contracts"),
        BTreeMap::from([("signed_pdf".to_owned(), 20_000_000)])
    );
    assert!(crud_file_fields(Some(&program), "missing").is_empty());
}

#[test]
#[ignore = "requires ZELYRA_TEST_DATABASE_URL pointing to a disposable MariaDB or PostgreSQL database"]
fn computed_decimal_crud_post_persists_the_server_recalculated_value() {
    let source = r#"
        database main { engine: mariadb }
        table __TABLE__ {
            id: Id primary auto
            quantity: Decimal(10, 3) required
            unit_price: Decimal(10, 3) required
            total: Decimal(12, 2) required compute(quantity * unit_price)
            validate { total > decimal("0"): "Total must be positive" }
        }
        form InvoiceCreate -> __TABLE__ {
            fields { quantity unit_price total }
            action save {
                sql { INSERT INTO __TABLE__ (quantity, unit_price, total) VALUES (:quantity, :unit_price, :total) }
            }
        }
        form InvoiceEdit -> __TABLE__ {
            fields { quantity unit_price total }
            action save {
                sql { UPDATE __TABLE__ SET quantity = :quantity, unit_price = :unit_price, total = :total WHERE id = :id }
            }
        }
        fn main() {}
    "#;
    let table_name = format!("zelyra_computed_test_{}", std::process::id());
    let source = source.replace("__TABLE__", &table_name);
    let program = zelyra_parser::parse(&zelyra_lexer::lex(&source).unwrap()).unwrap();
    zelyra_runtime::check(&program).expect("CRUD form fields should type check");
    let schema = zelyra_database::build_schema(&program).unwrap();
    let database_url = std::env::var("ZELYRA_TEST_DATABASE_URL")
        .expect("ignored integration test requires a disposable database URL");
    zelyra_database::execute_query(&database_url, &schema.create_sql(), Vec::new()).unwrap();

    let mut form = form_route();
    form.path = format!("/{table_name}/new");
    form.action = form.path.clone();
    form.form = program.forms[0].clone();
    form.form.fields = vec![
        field("quantity", Type::Decimal, true),
        field("unit_price", Type::Decimal, true),
        field("total", Type::Decimal, false),
    ];
    form.form.fields[2].readonly = true;
    form.form.table = Some(table_name.clone());
    form.table = Some(program.tables[0].clone());
    form.schema = Some(schema);
    let mut lowered = program.clone();
    let (functions, computed_column_functions) =
        zelyra_ast::build_computed_column_functions(&program);
    lowered.functions.extend(functions);
    let (functions, table_validation_functions) =
        zelyra_ast::build_table_validation_functions(&program);
    lowered.functions.extend(functions);
    let request = parse_request(
        &format!("POST /{table_name}/new HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\nContent-Type: application/x-www-form-urlencoded\r\n\r\n_zelyra_csrf=csrf-token&quantity=3&unit_price=0.385&total=999999"),
    )
    .unwrap();
    let response = dispatch_form_with_language(
        &form,
        &request,
        &HashMap::new(),
        Some(&database_url),
        FormDispatchContext {
            actor_user_id: None,
            tenant_context: None,
            language: UiLanguage::English,
            validation_program: Some(&lowered),
            table_validation_functions: &table_validation_functions,
            computed_column_functions: &computed_column_functions,
            crud_policy_functions: &[],
            crud_action_functions: &[],
            capability_grants: None,
            runtime_policy: None,
            granted_permissions: Vec::new(),
            multipart_parts: &[],
            upload_storage_root: None,
        },
    );
    assert_eq!(response.status, 303, "{}", response.body);
    let stored = zelyra_database::execute_query(
        &database_url,
        &format!("SELECT total FROM {table_name}"),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(stored.rows, vec![vec!["1.16".to_owned()]]);

    let id = zelyra_database::execute_query(
        &database_url,
        &format!("SELECT id FROM {table_name}"),
        Vec::new(),
    )
    .unwrap()
    .rows[0][0]
        .clone();
    let mut edit_route = form_route();
    edit_route.path = format!("/{table_name}/{{id}}/edit");
    edit_route.action = edit_route.path.clone();
    edit_route.form = program.forms[1].clone();
    edit_route.form.fields = vec![
        field("quantity", Type::Decimal, true),
        field("unit_price", Type::Decimal, true),
        field("total", Type::Decimal, false),
    ];
    edit_route.form.fields[2].readonly = true;
    edit_route.form.table = Some(table_name.clone());
    edit_route.table = Some(program.tables[0].clone());
    edit_route.schema = Some(zelyra_database::build_schema(&program).unwrap());
    let path_params = HashMap::from([("id".to_owned(), id)]);
    let edit_get = parse_request(&format!(
        "GET /{table_name}/{}/edit HTTP/1.1\r\nHost: localhost\r\n\r\n",
        path_params["id"]
    ))
    .unwrap();
    let edit_context = || FormDispatchContext {
        actor_user_id: None,
        tenant_context: None,
        language: UiLanguage::English,
        validation_program: Some(&lowered),
        table_validation_functions: &table_validation_functions,
        computed_column_functions: &computed_column_functions,
        crud_policy_functions: &[],
        crud_action_functions: &[],
        capability_grants: None,
        runtime_policy: None,
        granted_permissions: Vec::new(),
        multipart_parts: &[],
        upload_storage_root: None,
    };
    let edit_page = dispatch_form_with_language(
        &edit_route,
        &edit_get,
        &path_params,
        Some(&database_url),
        edit_context(),
    );
    assert_eq!(edit_page.status, 200, "{}", edit_page.body);
    let snapshot = edit_page
        .body
        .split("name=\"_zelyra_snapshot\" value=\"")
        .nth(1)
        .and_then(|value| value.split('"').next())
        .expect("edit form should include its signed record snapshot");
    let invalid_edit_post = parse_request(&format!(
        "POST /{table_name}/{}/edit HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\nContent-Type: application/x-www-form-urlencoded\r\n\r\n_zelyra_csrf=csrf-token&_zelyra_snapshot={snapshot}&quantity=-4&unit_price=0.385&total=999999",
        path_params["id"]
    ))
    .unwrap();
    let invalid_edit_response = dispatch_form_with_language(
        &edit_route,
        &invalid_edit_post,
        &path_params,
        Some(&database_url),
        edit_context(),
    );
    assert_eq!(
        invalid_edit_response.status, 422,
        "{}",
        invalid_edit_response.body
    );
    assert!(invalid_edit_response
        .body
        .contains("Total must be positive"));
    assert!(invalid_edit_response.body.contains("-1.54"));
    let unchanged = zelyra_database::execute_query(
        &database_url,
        &format!("SELECT total FROM {table_name}"),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(unchanged.rows, vec![vec!["1.16".to_owned()]]);

    let edit_post = parse_request(&format!(
        "POST /{table_name}/{}/edit HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\nContent-Type: application/x-www-form-urlencoded\r\n\r\n_zelyra_csrf=csrf-token&_zelyra_snapshot={snapshot}&quantity=4&unit_price=0.385&total=999999",
        path_params["id"]
    ))
    .unwrap();
    let edit_response = dispatch_form_with_language(
        &edit_route,
        &edit_post,
        &path_params,
        Some(&database_url),
        edit_context(),
    );
    assert_eq!(edit_response.status, 303, "{}", edit_response.body);
    let stored = zelyra_database::execute_query(
        &database_url,
        &format!("SELECT total FROM {table_name}"),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(stored.rows, vec![vec!["1.54".to_owned()]]);
    zelyra_database::execute_query(
        &database_url,
        &format!("DROP TABLE {table_name}"),
        Vec::new(),
    )
    .unwrap();

    fn field(name: &str, ty: Type, required: bool) -> zelyra_ast::FormField {
        zelyra_ast::FormField {
            name: name.into(),
            ty: Some(ty),
            label: None,
            placeholder: None,
            required,
            min_length: None,
            max: None,
            min_value: None,
            max_value: None,
            choices: Vec::new(),
            widget: None,
            appearance: None,
            file_upload: None,
            readonly: false,
            span: zelyra_ast::Span::default(),
        }
    }
}

#[test]
fn table_validation_converts_relation_fields_to_database_identity_values() {
    let source = r#"
        table departments { id: Int primary auto name: String required }
        table machines {
            department: Department required
            validate { department == department: "A department is required." }
        }
        fn main() {}
    "#;
    let program = zelyra_parser::parse(&zelyra_lexer::lex(source).unwrap()).unwrap();
    zelyra_runtime::check(&program).expect("relation field and validation are well typed");
    let (lowered, mappings) = zelyra_ast::lower_table_validation_functions(&program);
    let values = HashMap::from([("department".to_owned(), "1".to_owned())]);

    assert!(
        evaluate_table_validations(&lowered, &mappings, "machines", &values)
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        runtime_value_for_type(&lowered, &Type::Named("Department".into()), "1"),
        Ok(RuntimeValue::Int(1))
    ));
    assert!(runtime_value_for_type(&lowered, &Type::Named("UnknownRecord".into()), "1").is_err());
}

#[test]
fn tables_without_invariants_do_not_require_validation_form_values() {
    let source = "table machines { number: String required } fn main() {}";
    let program = zelyra_parser::parse(&zelyra_lexer::lex(source).unwrap()).unwrap();
    let (lowered, mappings) = zelyra_ast::lower_table_validation_functions(&program);
    assert!(
        evaluate_table_validations(&lowered, &mappings, "machines", &HashMap::new())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn auto_dialog_ids_are_repeatable_and_unique_inside_collection_loops() {
    let template = r#"
<ul>
for dvd in dvds {
<li><span id="{dvd.collision}"></span>{dvd.title}<button id="z-dialog-auto-0000000000-open" data-z-dialog-auto-open="z-dialog-auto-0000000000" data-z-dialog-open="z-dialog-auto-0000000000" aria-controls="z-dialog-auto-0000000000">Open</button><dialog id="z-dialog-auto-0000000000" data-z-dialog data-z-dialog-auto="z-dialog-auto-0000000000" aria-labelledby="z-dialog-auto-0000000000-title"><h2 id="z-dialog-auto-0000000000-title" data-z-dialog-auto-title="z-dialog-auto-0000000000">DVD</h2><button data-z-dialog-close>Schließen</button><p>{dvd.title}</p></dialog></li>
}
</ul>
"#;
    let values = HashMap::new();
    let collections = HashMap::from([(
        "dvds".to_owned(),
        vec![
            HashMap::from([
                ("title".to_owned(), "A & B".to_owned()),
                (
                    "collision".to_owned(),
                    "z-dialog-auto-0000000000-r0".to_owned(),
                ),
            ]),
            HashMap::from([
                ("title".to_owned(), "C".to_owned()),
                ("collision".to_owned(), "dvd-explicit-id".to_owned()),
            ]),
        ],
    )]);
    let first = render_static_template_preview(template, &values, &collections);
    let repeated = render_static_template_preview(template, &values, &collections);
    assert_eq!(
        first, repeated,
        "IDs are deterministic per rendered document"
    );
    assert_eq!(first.matches("data-z-dialog-auto=\"").count(), 2);
    let first_id = "z-dialog-auto-0000000000-r0-auto-2";
    let second_id = "z-dialog-auto-0000000000-r1";
    assert!(first.contains(&format!("id=\"{first_id}\" data-z-dialog")));
    assert!(first.contains(&format!(
        "data-z-dialog-open=\"{first_id}\" aria-controls=\"{first_id}\""
    )));
    assert!(first.contains(&format!("aria-labelledby=\"{first_id}-title\"")));
    assert!(first.contains(&format!("id=\"{second_id}\" data-z-dialog")));
    assert!(first.contains(&format!(
        "data-z-dialog-open=\"{second_id}\" aria-controls=\"{second_id}\""
    )));
    assert!(first.contains(&format!("aria-labelledby=\"{second_id}-title\"")));
    assert!(
        first.contains("id=\"z-dialog-auto-0000000000-r0\""),
        "explicit conflicting ID remains untouched"
    );
    assert!(first.contains("A &amp; B"));
    assert!(!first.contains("A & B"));
    let concurrent_template = template.to_owned();
    let concurrent_values = values.clone();
    let concurrent_collections = collections.clone();
    let concurrent = std::thread::spawn(move || {
        render_static_template_preview(
            &concurrent_template,
            &concurrent_values,
            &concurrent_collections,
        )
    });
    assert_eq!(first, concurrent.join().expect("parallel render completes"));

    let duplicate = r#"<button id="z-dialog-auto-0000000009-open" data-z-dialog-auto-open="z-dialog-auto-0000000009" data-z-dialog-open="z-dialog-auto-0000000009" aria-controls="z-dialog-auto-0000000009">Open</button><dialog id="z-dialog-auto-0000000009" data-z-dialog-auto="z-dialog-auto-0000000009" aria-labelledby="z-dialog-auto-0000000009-title"><h2 id="z-dialog-auto-0000000009-title" data-z-dialog-auto-title="z-dialog-auto-0000000009">Title</h2></dialog>"#;
    let duplicate = format!("{duplicate}{duplicate}");
    let duplicate_render = render_static_template_preview(&duplicate, &values, &HashMap::new());
    assert_eq!(duplicate_render.matches("data-z-dialog-auto=\"").count(), 2);
    assert!(duplicate_render.contains("id=\"z-dialog-auto-0000000009\" data-z-dialog-auto="));
    assert!(duplicate_render.contains("id=\"z-dialog-auto-0000000009-auto-2\" data-z-dialog-auto="));
    assert!(duplicate_render.contains("aria-controls=\"z-dialog-auto-0000000009-auto-2\""));
    assert!(localize_html(
        "<button data-zelyra-i18n=\"ui.dialog.close\">Schließen</button>",
        UiLanguage::English,
    )
    .contains(">Close</button>"));
    assert!(localize_html(
        "<button data-zelyra-i18n=\"ui.dialog.close\">Schließen</button>",
        UiLanguage::German,
    )
    .contains(">Schließen</button>"));
}

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
fn schedule_probe_is_private_to_the_configured_server_token() {
    let app = WebApp::new(Vec::new(), Vec::new())
        .with_schedule_probe_token(Some("random-per-server-secret".into()));
    let authorized = parse_request(
        "POST /__zelyra/schedule/probe HTTP/1.1\r\nHost: 127.0.0.1:3000\r\nx-zelyra-schedule-token: random-per-server-secret\r\nContent-Length: 0\r\n\r\n",
    )
    .unwrap();
    assert_eq!(app.dispatch(&authorized).status, 204);

    let missing = parse_request(
        "POST /__zelyra/schedule/probe HTTP/1.1\r\nHost: 127.0.0.1:3000\r\nContent-Length: 0\r\n\r\n",
    )
    .unwrap();
    assert_eq!(app.dispatch(&missing).status, 404);

    let wrong = parse_request(
        "POST /__zelyra/schedule/probe HTTP/1.1\r\nHost: 127.0.0.1:3000\r\nx-zelyra-schedule-token: wrong\r\nContent-Length: 0\r\n\r\n",
    )
    .unwrap();
    assert_eq!(app.dispatch(&wrong).status, 404);
}

#[test]
fn applies_crud_layout_only_to_html_responses() {
    let response = apply_generated_layout(
        Response::html(200, "<main>CRUD content</main>"),
        Some("<body>\u{0}ZELYRA_CRUD_CONTENT\u{0}</body>"),
    );
    assert_eq!(
        response.body,
        "<body><div class=\"z-generated-page\">CRUD content</div></body>"
    );

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
            path: None,
            post_only: false,
            verify: None,
            fields: vec![zelyra_ast::FormField {
                name: "name".into(),
                ty: Some(Type::String),
                label: Some("Name".into()),
                placeholder: None,
                required: true,
                min_length: None,
                max: Some(20),
                min_value: None,
                max_value: None,
                choices: Vec::new(),
                widget: None,
                appearance: None,
                file_upload: None,
                readonly: false,
                span: zelyra_ast::Span::default(),
            }],
            actions: Vec::new(),
            span: zelyra_ast::Span::default(),
        },
        table: None,
        schema: None,
        tenant_column: None,
        recipient_column: None,
        tenant_membership_table: None,
        requires_auth: false,
        permissions: Vec::new(),
        csrf: CsrfProtection::new("csrf-token"),
        form_view: CrudFormViewDef::default(),
        before_form_html: None,
        after_form_html: None,
        record_context_fields: Vec::new(),
        form_context_fields: Vec::new(),
        field_content_html: HashMap::new(),
        post_only: false,
        audit_table: None,
        audit_event: None,
        audit_chain: false,
        layout_html: None,
        child_collections: Vec::new(),
    }
}

fn child_collection_form_route() -> FormRoute {
    let mut route = form_route();
    let fields = vec![
        zelyra_ast::FormField {
            name: "sku".into(),
            ty: Some(Type::String),
            label: Some("SKU".into()),
            placeholder: None,
            required: true,
            min_length: None,
            max: None,
            min_value: None,
            max_value: None,
            choices: Vec::new(),
            widget: None,
            appearance: None,
            file_upload: None,
            readonly: false,
            span: zelyra_ast::Span::default(),
        },
        zelyra_ast::FormField {
            name: "quantity".into(),
            ty: Some(Type::Int),
            label: Some("Quantity".into()),
            placeholder: None,
            required: true,
            min_length: None,
            max: None,
            min_value: Some("1".into()),
            max_value: None,
            choices: Vec::new(),
            widget: None,
            appearance: None,
            file_upload: None,
            readonly: false,
            span: zelyra_ast::Span::default(),
        },
    ];
    route.child_collections = vec![CrudChildFormRoute {
        definition: zelyra_ast::CrudChildRelationDef {
            name: "items".into(),
            model: "InvoiceItem".into(),
            foreign_key: "invoice_id".into(),
            order_by: Some("position".into()),
            descending: false,
            on_delete: Some(zelyra_ast::ReferentialAction::Cascade),
            aggregates: Vec::new(),
            span: zelyra_ast::Span::default(),
        },
        table: zelyra_ast::TableDef {
            name: "invoice_items".into(),
            columns: Vec::new(),
            validations: Vec::new(),
            indexes: Vec::new(),
            uniques: Vec::new(),
            access: Default::default(),
            tenant_scope: None,
            span: zelyra_ast::Span::default(),
        },
        fields,
        count_key: "_zelyra_child_count".into(),
    }];
    route
}

#[test]
fn renders_accessible_child_rows_and_escapes_submitted_values() {
    let route = child_collection_form_route();
    let values = HashMap::from([
        ("_zelyra_child_count".into(), "2".into()),
        ("items.0.sku".into(), "<script>alert(1)</script>".into()),
        ("items.0.quantity".into(), "2".into()),
        ("items.1.sku".into(), "SKU-2".into()),
        ("items.1.quantity".into(), "3".into()),
    ]);
    let html = render_form(&route, &values, &[], None);
    assert!(html.contains("<fieldset class=\"z-fieldset z-crud-child-collection\">"));
    assert!(html.contains("name=\"items.0.sku\""));
    assert!(html.contains("name=\"items.1.quantity\""));
    assert!(html.contains("value=\"&lt;script&gt;alert(1)&lt;/script&gt;\""));
    assert!(!html.contains("value=\"<script>"));
    assert!(html.contains("value=\"add\""));
    assert!(html.contains("value=\"remove:1\""));
    assert_eq!(html.matches("<form ").count(), 1);
}

#[test]
fn child_collection_legends_localize_after_html_escaping() {
    let route = child_collection_form_route();
    let rendered = render_form(&route, &HashMap::new(), &[], None);
    let localized = localize_html(&rendered, UiLanguage::English);
    assert!(localized.contains("<legend>Items</legend>"));
    assert!(!localized.contains("&#xe000;zelyra-locale:"));
    assert!(!localized.contains("zelyra-locale:identifier.items"));
}

#[test]
fn child_collection_legend_uses_project_translation_and_language_fallback() {
    let route = child_collection_form_route();
    let rendered = render_form(&route, &HashMap::new(), &[], None);
    let mut catalogs = ProjectUiCatalogs::default();
    catalogs
        .set_json(
            UiLanguage::German,
            r#"{"identifier.items":"Rechnungspositionen"}"#,
        )
        .unwrap();

    let german = localize_html_with_catalog(&rendered, UiLanguage::German, &catalogs);
    assert!(german.contains("<legend>Rechnungspositionen</legend>"));
    assert!(german.contains("<legend>Rechnungspositionen 1</legend>"));

    // A missing project entry falls back through the built-in English identifier
    // humanizer instead of exposing the internal locale marker.
    let english = localize_html_with_catalog(&rendered, UiLanguage::English, &catalogs);
    assert!(english.contains("<legend>Items</legend>"));
    assert!(!german.contains("zelyra-locale:"));
    assert!(!english.contains("zelyra-locale:"));
}

#[test]
fn automatic_invoice_number_queries_are_tenant_scoped() {
    let mut route = form_route();
    route.form.table = Some("invoices".into());
    route.tenant_column = Some("tenant_id".into());
    let tenant = TenantContext {
        user_id: 7,
        tenant_id: 23,
        membership_table: None,
        membership_has_active: false,
        tenant_owner_table: None,
        tenant_owner_column: None,
    };

    let lookup =
        automatic_invoice_number_lookup_query(&route, false, "RE-000042", 42, Some(&tenant));
    assert!(lookup
        .sql
        .contains("AND `tenant_id` = :zelyra_invoice_tenant_id"));
    assert!(lookup.sql.ends_with("FOR UPDATE"));
    assert!(lookup.params.iter().any(|(name, value)| {
        name == "zelyra_invoice_tenant_id" && matches!(value, QueryValue::Int(23))
    }));

    let update =
        automatic_invoice_number_update_query(&route, false, "RE-000042".into(), 42, Some(&tenant));
    assert!(update
        .sql
        .contains("AND `tenant_id` = :zelyra_invoice_tenant_id"));
    assert!(update.params.iter().any(|(name, value)| {
        name == "zelyra_invoice_tenant_id" && matches!(value, QueryValue::Int(23))
    }));
}

#[test]
fn legacy_invoice_number_generation_remains_for_fields_without_a_sequence() {
    let mut route = form_route();
    route.form.name = "InvoiceCreate".into();
    route.form.table = Some("invoices".into());
    route.form.fields[0].name = "invoice_number".into();
    assert!(is_automatic_invoice_number_form(&route));

    route.table = Some(zelyra_ast::TableDef {
        name: "invoices".into(),
        columns: vec![zelyra_ast::ColumnDef {
            name: "invoice_number".into(),
            ty: Type::String,
            length: Some(32),
            storage_name: None,
            decimal_precision: None,
            auto_sequence: Some("RE-{0001}".into()),
            computed: None,
            attachment: None,
            required: true,
            primary_key: false,
            auto: false,
            unique: false,
            unique_within_tenant: false,
            default: None,
            references: None,
            on_delete: None,
            span: zelyra_ast::Span::default(),
        }],
        validations: Vec::new(),
        indexes: Vec::new(),
        uniques: Vec::new(),
        access: zelyra_ast::TableAccessDef::default(),
        tenant_scope: None,
        span: zelyra_ast::Span::default(),
    });
    assert!(!is_automatic_invoice_number_form(&route));
}

#[test]
fn renders_relation_options_from_the_matching_child_collection() {
    let mut route = child_collection_form_route();
    let child = route.child_collections.first_mut().unwrap();
    child.fields[0].name = "customer_id".into();
    child.fields[0].ty = Some(Type::Named("Customer".into()));
    let options = HashMap::from([(
        "items.customer_id".into(),
        vec![SelectOption {
            value: "42".into(),
            label: "Acme GmbH".into(),
        }],
    )]);
    let html = render_form_with_language(
        &route,
        &HashMap::from([("items.0.customer_id".into(), "42".into())]),
        &[],
        None,
        &options,
        UiLanguage::English,
    );
    assert!(html.contains(
        "<select class=\"z-select\" id=\"items.0.customer_id\" name=\"items.0.customer_id\""
    ));
    assert!(html.contains("<option value=\"42\" selected>Acme GmbH</option>"));
}

#[test]
fn child_row_add_remove_actions_are_bounded_and_preserve_values() {
    let route = child_collection_form_route();
    let child = route.child_collections.first().unwrap();
    let mut values = HashMap::from([
        ("_zelyra_child_count".into(), "2".into()),
        ("items.0.sku".into(), "A".into()),
        ("items.1.sku".into(), "B".into()),
    ]);
    assert!(mutate_child_form_rows(
        &mut values,
        std::slice::from_ref(child),
        "add"
    ));
    assert_eq!(
        values.get("_zelyra_child_count").map(String::as_str),
        Some("3")
    );
    values.insert("items.2.sku".into(), "C".into());
    assert!(mutate_child_form_rows(
        &mut values,
        std::slice::from_ref(child),
        "remove:1"
    ));
    assert_eq!(values.get("items.1.sku").map(String::as_str), Some("C"));
    assert_eq!(
        values.get("_zelyra_child_count").map(String::as_str),
        Some("2")
    );
    values.insert("_zelyra_child_count".into(), "50".into());
    assert!(!mutate_child_form_rows(
        &mut values,
        std::slice::from_ref(child),
        "add"
    ));
    assert!(!mutate_child_form_rows(
        &mut values,
        std::slice::from_ref(child),
        "remove:0"
    ));
}

#[test]
fn multiple_child_sections_have_independent_counts_and_row_actions() {
    let mut route = child_collection_form_route();
    route.child_collections[0].count_key = "_items_zelyra_child_count".into();
    let mut attachments = route.child_collections[0].clone();
    attachments.definition.name = "attachments".into();
    attachments.definition.model = "InvoiceAttachment".into();
    attachments.definition.foreign_key = "invoice_id".into();
    attachments.count_key = "_attachments_zelyra_child_count".into();
    attachments.fields[0].name = "filename".into();
    attachments.fields[0].label = Some("Filename".into());
    route.child_collections.push(attachments);
    let values = HashMap::from([
        ("_items_zelyra_child_count".into(), "2".into()),
        ("_attachments_zelyra_child_count".into(), "1".into()),
        ("items.0.sku".into(), "SKU-A".into()),
        ("items.1.sku".into(), "SKU-B".into()),
        ("attachments.0.filename".into(), "invoice.pdf".into()),
    ]);
    let html = render_form(&route, &values, &[], None);
    assert!(html.contains("name=\"items.1.sku\""));
    assert!(html.contains("name=\"attachments.0.filename\""));
    assert!(html.contains("name=\"_attachments_zelyra_child_count\" value=\"1\""));
    assert!(html.contains("value=\"items:add\""));
    assert!(html.contains("value=\"attachments:add\""));
    assert_eq!(html.matches("z-crud-child-collection").count(), 2);

    let mut values = values;
    assert!(mutate_child_form_rows(
        &mut values,
        &route.child_collections,
        "attachments:add"
    ));
    assert_eq!(values.get("_items_zelyra_child_count").unwrap(), "2");
    assert_eq!(values.get("_attachments_zelyra_child_count").unwrap(), "2");
    assert!(mutate_child_form_rows(
        &mut values,
        &route.child_collections,
        "items:remove:1"
    ));
    assert_eq!(values.get("_items_zelyra_child_count").unwrap(), "1");
    assert_eq!(values.get("_attachments_zelyra_child_count").unwrap(), "2");
}

#[test]
fn child_field_names_are_isolated_from_parent_form_validation() {
    let route = child_collection_form_route();
    let child = route.child_collections.first().unwrap();
    assert!(is_crud_child_input_name(child, "items.0.sku"));
    assert!(is_crud_child_input_name(child, "items.12.quantity"));
    assert!(!is_crud_child_input_name(child, "items.0.unknown"));
    assert!(!is_crud_child_input_name(child, "sku"));
}

#[test]
fn table_validation_execution_has_a_hard_step_limit() {
    let source = r#"
        fn expensive() -> Bool {
            mutable i = 0
            while i < 20000 { i = i + 1 }
            return true
        }
        table invoices {
            amount: Int required
            validate { expensive(): "Value rejected" }
        }
        fn main() {}
    "#;
    let program = zelyra_parser::parse(&zelyra_lexer::lex(source).unwrap()).unwrap();
    zelyra_runtime::check(&program).unwrap();
    zelyra_runtime::check_capabilities(&program).unwrap();
    let (lowered, mappings) = zelyra_ast::lower_table_validation_functions(&program);
    let values = HashMap::from([("amount".to_owned(), "5".to_owned())]);
    let error = evaluate_table_validations(&lowered, &mappings, "invoices", &values)
        .expect_err("form validation must stop after its bounded execution budget");
    assert!(error.contains("execution step budget exceeded"));
}

#[test]
fn table_validation_rejects_invalid_form_before_action_execution() {
    let source = r#"
        table invoices {
            invoice_date: Date required
            due_date: Date?
            validate {
                match due_date {
                    Some(due) => due >= invoice_date
                    None => true
                }: "Due date must not precede invoice date."
            }
        }
        fn main() {}
    "#;
    let program = zelyra_parser::parse(&zelyra_lexer::lex(source).unwrap()).unwrap();
    zelyra_runtime::check(&program).unwrap();
    let (lowered, mappings) = zelyra_ast::lower_table_validation_functions(&program);
    let table = program.tables[0].clone();

    let mut route = form_route();
    route.form.name = "InvoiceCreate".into();
    route.form.table = Some("invoices".into());
    route.form.fields = vec![
        zelyra_ast::FormField {
            name: "invoice_date".into(),
            ty: Some(Type::Date),
            label: Some("Invoice date".into()),
            placeholder: None,
            required: true,
            min_length: None,
            max: None,
            min_value: None,
            max_value: None,
            choices: Vec::new(),
            widget: None,
            appearance: None,
            file_upload: None,
            readonly: false,
            span: zelyra_ast::Span::default(),
        },
        zelyra_ast::FormField {
            name: "due_date".into(),
            ty: Some(Type::Option(Box::new(Type::Date))),
            label: Some("Due date".into()),
            placeholder: None,
            required: false,
            min_length: None,
            max: None,
            min_value: None,
            max_value: None,
            choices: Vec::new(),
            widget: None,
            appearance: None,
            file_upload: None,
            readonly: false,
            span: zelyra_ast::Span::default(),
        },
    ];
    route.form.actions = vec![zelyra_ast::FormAction {
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
        capabilities: Vec::new(),
        require_match: false,
        statements: vec![zelyra_ast::Stmt::Expr(zelyra_ast::Expr {
            kind: zelyra_ast::ExprKind::Sql {
                result_type: Type::Unit,
                query: "INSERT INTO invoices (invoice_date, due_date) VALUES (:invoice_date, :due_date)".into(),
            },
            span: zelyra_ast::Span::default(),
        })],
        execute_body: None,
        guard: None,
        success: None,
        redirect: None,
        span: zelyra_ast::Span::default(),
    }];
    route.table = Some(table);

    let request = parse_request("POST /forms/CustomerCreate HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\n\r\n_zelyra_csrf=csrf-token&invoice_date=2026-10-20&due_date=2026-10-19").unwrap();
    let response = dispatch_form_with_language(
        &route,
        &request,
        &HashMap::new(),
        None,
        FormDispatchContext {
            actor_user_id: None,
            tenant_context: None,
            language: UiLanguage::English,
            validation_program: Some(&lowered),
            table_validation_functions: &mappings,
            computed_column_functions: &[],
            crud_policy_functions: &[],
            crud_action_functions: &[],
            capability_grants: None,
            runtime_policy: None,
            granted_permissions: Vec::new(),
            multipart_parts: &[],
            upload_storage_root: None,
        },
    );
    assert_eq!(response.status, 422);
    assert!(
        response
            .body
            .contains("Due date must not precede invoice date."),
        "unexpected invalid-form response: {}",
        response.body
    );

    let valid_request = parse_request("POST /forms/CustomerCreate HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\n\r\n_zelyra_csrf=csrf-token&invoice_date=2026-10-20&due_date=2026-10-21").unwrap();
    let valid_response = dispatch_form_with_language(
        &route,
        &valid_request,
        &HashMap::new(),
        None,
        FormDispatchContext {
            actor_user_id: None,
            tenant_context: None,
            language: UiLanguage::English,
            validation_program: Some(&lowered),
            table_validation_functions: &mappings,
            computed_column_functions: &[],
            crud_policy_functions: &[],
            crud_action_functions: &[],
            capability_grants: None,
            runtime_policy: None,
            granted_permissions: Vec::new(),
            multipart_parts: &[],
            upload_storage_root: None,
        },
    );
    assert_eq!(
        valid_response.status, 503,
        "the valid request reaches the configured write action and requires a database"
    );
}

#[test]
fn monetary_form_and_query_parameters_preserve_every_digit() {
    for ty in [
        Type::Decimal,
        Type::Named("Money".into()),
        Type::Option(Box::new(Type::Decimal)),
    ] {
        let mut form = form_route();
        form.form.fields[0].ty = Some(ty.clone());
        let text = "999999999999999.9999";
        let parameters =
            form_query_parameters(&form, &HashMap::from([("name".into(), text.into())])).unwrap();
        assert!(
            matches!(&parameters[0].1, QueryValue::Decimal(value) if value == text),
            "money was converted to Float"
        );
        let value = page_query_value(&ty, text).unwrap();
        assert!(matches!(value, QueryValue::Decimal(value) if value == text));
    }

    let column = zelyra_database::Column {
        name: "price".into(),
        sql_type: "DECIMAL(18,2)".into(),
        nullable: false,
        primary_key: false,
        auto: false,
        unique: false,
        default: None,
    };
    let value = filter_query_value(&column, "999999999999999.9999").unwrap();
    assert!(matches!(value, QueryValue::Decimal(value) if value == "999999999999999.9999"));
}

#[test]
fn edit_form_snapshots_are_signed_and_bound_to_csrf_key_and_field_set() {
    let csrf = CsrfProtection::new("session-specific-csrf-secret");
    let form = form_route().form;
    let values = EditValues {
        values: HashMap::from([("name".to_owned(), "original".to_owned())]),
        nulls: HashSet::new(),
    };
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
        &EditValues {
            values: HashMap::from([("name".to_owned(), "modified".to_owned())]),
            nulls: HashSet::new()
        }
    ));
    assert!(verify_form_snapshot(&format!("{token}00")).is_none());

    let mut mismatched_form = form.clone();
    mismatched_form.fields[0].name = "email".into();
    let mismatched_values = EditValues {
        values: HashMap::from([("email".to_owned(), "original".to_owned())]),
        nulls: HashSet::new(),
    };
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
        file_columns: Vec::new(),
        tenant_column: None,
        recipient_column: None,
        unread_column: None,
        expires_column: None,
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
            response.body.contains("class=\"z-app-shell zelyra-app\""),
            "default app shell missing for {path}"
        );
        assert!(response.body.contains("lang=\"de\""));
        assert!(response.body.contains("data-z-ui-stylesheet"));
        assert!(response
            .body
            .contains("class=\"z-app-shell__main zelyra-generated-main\""));
        assert!(response
            .body
            .contains("aria-controls=\"zelyra-default-navigation\""));
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
fn url_encoded_template_interpolation_escapes_mailto_query_values() {
    let params = HashMap::new();
    let data = HashMap::from([
        ("recipient".into(), "rechnung@example.test".into()),
        ("subject".into(), "RE 2026&001".into()),
    ]);

    assert_eq!(
        render_interpolations(
            "mailto:{url_encode:recipient}?subject={url_encode:subject}",
            &params,
            &data,
        ),
        "mailto:rechnung%40example.test?subject=RE%202026%26001"
    );
}

#[test]
fn static_view_translation_interpolation_uses_project_locale_and_escapes_html() {
    let rendered = render_interpolations(
        "<h1>{t(\"site.title\")}</h1><meta content=\"{i18n(\"site.summary\")}\"><p>{t(\"site.fallback\")}</p><p>{t(\"site.missing\")}</p>",
        &HashMap::new(),
        &HashMap::new(),
    );
    assert!(rendered.contains("\u{e000}zelyra-locale:site.title\u{e001}"));

    let mut catalogs = ProjectUiCatalogs::default();
    catalogs
        .set_json(
            UiLanguage::German,
            r#"{"site.title":"Willkommen <script>","site.summary":"A & B"}"#,
        )
        .unwrap();
    catalogs
        .set_json(
            UiLanguage::English,
            r#"{"site.fallback":"English & fallback"}"#,
        )
        .unwrap();
    let localized = localize_html_with_catalog(&rendered, UiLanguage::German, &catalogs);
    assert_eq!(
        localized,
        "<h1>Willkommen &lt;script&gt;</h1><meta content=\"A &amp; B\"><p>English &amp; fallback</p><p>[missing translation]</p>"
    );
}

#[test]
fn locale_prefix_selects_catalog_for_shared_view_and_rejects_unknown_locale() {
    let route = Route {
        path: "/{locale}/home".into(),
        html: "<h1>{t(\"site.home.title\")}</h1>".into(),
        query: Vec::new(),
        page_size: None,
        sort_columns: Vec::new(),
        search_columns: Vec::new(),
        filters: Vec::new(),
        data: Vec::new(),
        requires_auth: false,
        permissions: Vec::new(),
    };
    let router = Router::new(vec![route.clone()]);
    let mut catalogs = ProjectUiCatalogs::default();
    catalogs
        .set_json(UiLanguage::German, r#"{"site.home.title":"Startseite"}"#)
        .unwrap();
    catalogs
        .set_json(UiLanguage::English, r#"{"site.home.title":"Home"}"#)
        .unwrap();

    let render = |path| {
        router.dispatch_with_database_language_and_catalogs(
            "GET",
            path,
            None,
            UiLanguage::English,
            &catalogs,
        )
    };
    assert_eq!(render("/de/home").body, "<h1>Startseite</h1>");
    assert_eq!(render("/en/home").body, "<h1>Home</h1>");
    assert_eq!(render("/fr/home").status, 404);

    let app = WebApp::new(vec![route], Vec::new()).with_project_ui_catalogs(catalogs);
    let response = app.dispatch(&Request {
        method: "GET".into(),
        target: "/de/home".into(),
        path: "/de/home".into(),
        headers: HashMap::new(),
        body: String::new(),
        remote_addr: None,
    });
    assert_eq!(response.status, 200);
    assert_eq!(response.body, "<h1>Startseite</h1>");
}

#[test]
fn default_application_shell_uses_the_shared_mobile_navigation_contract() {
    let context = DefaultUiContext {
        current_path: "/invoices".into(),
        current_label: "Invoices".into(),
        navigation: vec![DefaultNavigationLink {
            path: "/invoices".into(),
            label: "Invoices".into(),
        }],
    };
    let html = render_default_application_shell("<h1>Invoices</h1>", &context, UiLanguage::English);

    assert!(html.contains("class=\"z-app-shell__navigation zelyra-nav\""));
    assert!(html.contains("data-z-mobile-nav-toggle"));
    assert!(html.contains("aria-controls=\"zelyra-default-navigation\""));
    assert!(html.contains("aria-expanded=\"false\""));
    assert!(html.contains("id=\"zelyra-default-navigation\""));
    assert!(html.contains("<section class=\"z-app-shell__main zelyra-generated-main\""));
    assert_eq!(html.matches("<main").count(), 1);
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
    assert!(!response.body.contains("class=\"z-app-shell zelyra-app\""));
    assert!(!response.body.contains("data-z-ui-stylesheet"));
}

#[test]
fn generated_main_content_becomes_a_fragment_inside_a_custom_layout() {
    let layout = format!("<main class=\"app-shell-main\">{CRUD_LAYOUT_CONTENT_MARKER}</main>");
    let response = apply_generated_layout(
        Response::html(
            200,
            "<main data-loading-message=\"Loading\"><h1>Invoices</h1><table></table></main>",
        ),
        Some(&layout),
    );

    assert!(response.body.contains(
        "<main class=\"app-shell-main\"><div class=\"z-generated-page\" data-loading-message=\"Loading\">"
    ));
    assert!(response
        .body
        .contains("<h1>Invoices</h1><table></table></div></main>"));
    assert_eq!(response.body.matches("<main").count(), 1);
}

#[test]
fn default_shell_covers_standalone_forms_and_tableviews_but_not_custom_pages() {
    let form = form_route();
    let tableview = TableViewRoute {
        path: "/reports".into(),
        title: "Reports".into(),
        layout_html: None,
        source: "report query".into(),
        columns: Vec::new(),
        row_fields: Vec::new(),
        column_content: Vec::new(),
        before_content_html: None,
        after_content_html: None,
        row_actions: None,
        policy_row_actions: None,
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
        assert!(response.body.contains("class=\"z-app-shell zelyra-app\""));
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
fn dynamic_page_content_type_is_sent_and_rejects_header_injection() {
    let route = Route {
        path: "/sitemap.xml".into(),
        html: "<urlset><url><loc>https://example.test/</loc></url></urlset>".into(),
        query: Vec::new(),
        page_size: None,
        sort_columns: Vec::new(),
        search_columns: Vec::new(),
        filters: Vec::new(),
        data: Vec::new(),
        requires_auth: false,
        permissions: Vec::new(),
    };
    let app = WebApp::new(vec![route], Vec::new())
        .with_page_content_types(HashMap::from([(
            "/sitemap.xml".into(),
            "application/xml; charset=utf-8".into(),
        )]))
        .expect("valid XML media type is accepted");
    let request = parse_request("GET /sitemap.xml HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let response = app.dispatch(&request);
    assert_eq!(response.content_type, "application/xml; charset=utf-8");
    assert_eq!(
        response.body,
        "<urlset><url><loc>https://example.test/</loc></url></urlset>"
    );
    assert!(response
        .to_http()
        .contains("Content-Type: application/xml; charset=utf-8\r\n"));

    assert!(WebApp::new(Vec::new(), Vec::new())
        .with_page_content_types(HashMap::from([(
            "/bad".into(),
            "text/plain\r\nX-Injected: yes".into(),
        )]))
        .is_err());
}

#[test]
fn generated_login_page_uses_the_localized_default_shell() {
    let auth = AuthRoute {
        table: "users".into(),
        login_enabled_column: None,
        membership_table: None,
        tenant_owner_table: None,
        tenant_owner_column: None,
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
        reset_rate_limits_table: None,
        reset_rate_limit: DEFAULT_RESET_RATE_LIMIT,
        reset_block_seconds: DEFAULT_RESET_BLOCK_SECONDS,
        mfa_table: None,
        mfa_recovery_table: None,
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

    assert!(response.body.contains("<html data-z-ui"));
    assert!(response.body.contains("class=\"z-app-shell zelyra-app\""));
    assert!(response.body.contains("lang=\"de\""));
    assert!(response.body.contains("<title>Anmelden | Zelyra</title>"));
    assert!(response.body.contains("Zum Inhalt springen"));
    assert!(response.body.contains("id=\"zelyra-content\""));
    assert!(response.body.contains("href=\"/\""));
    assert!(!response.body.contains("/forgot-password"));

    let mut reset_auth = auth;
    reset_auth.reset_tokens_table = Some("password_resets".into());
    reset_auth.reset_rate_limits_table = Some("password_reset_limits".into());
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
        .find(&format!("href=\"{ZELYRA_UI_CSS_PATH}\""))
        .expect("default design system should be linked locally");
    let ui_script = page
        .body
        .find(&format!("src=\"{ZELYRA_UI_JS_PATH}\""))
        .expect("local UI behavior should be linked");
    let project_style = page
        .body
        .find("href=\"/__zelyra/theme.css\"")
        .expect("project theme should be linked");
    assert!(default_style < project_style);
    assert!(ui_script < project_style);
    assert!(page.body.contains("<html data-z-ui>"));

    let asset_request = parse_request(&format!(
        "GET {ZELYRA_UI_CSS_PATH} HTTP/1.1\r\nHost: localhost\r\n\r\n"
    ))
    .unwrap();
    let asset = app.dispatch(&asset_request);
    assert_eq!(asset.status, 200);
    assert_eq!(asset.content_type, "text/css; charset=utf-8");
    assert!(asset
        .body
        .contains("@layer reset, base, layout, components, utilities, theme, overrides;"));
    assert!(asset
        .to_http()
        .contains("Cache-Control: public, max-age=31536000, immutable\r\n"));

    let js_request = parse_request(&format!(
        "GET {ZELYRA_UI_JS_PATH} HTTP/1.1\r\nHost: localhost\r\n\r\n"
    ))
    .unwrap();
    let script = app.dispatch(&js_request);
    assert_eq!(script.content_type, "text/javascript; charset=utf-8");
    assert!(script.body.contains("window.ZelyraUI"));

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
fn zelyra_ui_assets_do_not_restyle_unmarked_html() {
    let html = "<!doctype html><html><head></head><body><main class=\"content\">Plain HTML</main></body></html>";
    assert_eq!(inject_design_system(html, false, false), html);

    let scoped = include_str!("../assets/zelyra-ui.v1.css");
    assert!(scoped.contains("@layer reset, base, layout, components, utilities, theme, overrides;"));
    assert!(scoped.contains(":where(html[data-z-ui])"));
    assert!(!scoped.lines().any(|line| {
        line.split_once('{')
            .is_some_and(|(selector, _)| selector.trim() == "body")
    }));
}

#[test]
fn replacement_stylesheet_replaces_the_default_and_keeps_project_overrides_last() {
    let route = Route {
        path: "/".into(),
        html: "<html><head></head><body><main class=\"z-card\">Home</main></body></html>".into(),
        query: Vec::new(),
        page_size: None,
        sort_columns: Vec::new(),
        search_columns: Vec::new(),
        filters: Vec::new(),
        data: Vec::new(),
        requires_auth: false,
        permissions: Vec::new(),
    };
    let replacement = "/* local replacement */ body { color: #123; }";
    let app = WebApp::new(vec![route], Vec::new())
        .with_ui_stylesheet_css(Some(replacement.into()))
        .with_project_theme_css(Some(":root { --project: yes; }".into()));
    let request = parse_request("GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let response = app.dispatch(&request);
    assert!(response
        .body
        .contains(&format!("href=\"{PROJECT_UI_CSS_PATH}\"")));
    assert!(!response
        .body
        .contains(&format!("href=\"{ZELYRA_UI_CSS_PATH}\"")));
    assert!(
        response.body.find(PROJECT_UI_CSS_PATH).unwrap()
            < response.body.find(PROJECT_THEME_CSS_PATH).unwrap()
    );

    let css_request = parse_request(&format!(
        "GET {PROJECT_UI_CSS_PATH} HTTP/1.1\r\nHost: localhost\r\n\r\n"
    ))
    .unwrap();
    let css = app.dispatch(&css_request);
    assert_eq!(css.status, 200);
    assert_eq!(css.body, replacement);
    assert!(css.to_http().contains("Cache-Control: no-cache\r\n"));
    assert_eq!(
        WebApp::new(Vec::new(), Vec::new())
            .dispatch(&css_request)
            .status,
        404
    );
}

#[test]
fn default_design_system_exposes_the_complete_public_theme_token_set() {
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
        "--z-font-sans",
        "--z-font-mono",
        "--z-content-width",
        "--z-control-height",
        "--z-radius-md",
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
        ".z-generated-page",
        ".zelyra-pagination",
        "main[data-loading-message]",
    ] {
        assert!(
            ZELYRA_DESIGN_SYSTEM_CSS.contains(selector),
            "default design system misses generated state selector {selector}"
        );
    }
    assert!(ZELYRA_DESIGN_SYSTEM_CSS.contains("@media (max-width: 900px)"));
    assert!(ZELYRA_DESIGN_SYSTEM_CSS.contains("@media (max-width: 600px)"));
    assert!(
        ZELYRA_DESIGN_SYSTEM_CSS.contains(":where(html[data-z-ui]) .zelyra-crud-form-card form")
    );
    assert!(ZELYRA_DESIGN_SYSTEM_CSS.contains(".z-query-controls__fields"));
}

fn relation_form_route() -> FormRoute {
    let mut route = form_route();
    route.form.fields.push(zelyra_ast::FormField {
        name: "department".into(),
        ty: None,
        label: Some("Department".into()),
        placeholder: None,
        required: true,
        min_length: None,
        max: None,
        min_value: None,
        max_value: None,
        choices: Vec::new(),
        widget: None,
        appearance: None,
        file_upload: None,
        readonly: false,
        span: zelyra_ast::Span::default(),
    });
    route.table = Some(zelyra_ast::TableDef {
        name: "customers".into(),
        validations: Vec::new(),
        tenant_scope: None,
        columns: vec![zelyra_ast::ColumnDef {
            name: "department".into(),
            ty: Type::Named("Department".into()),
            length: None,
            storage_name: None,
            decimal_precision: None,
            auto_sequence: None,
            computed: None,
            attachment: None,
            required: true,
            primary_key: false,
            auto: false,
            unique: false,
            unique_within_tenant: false,
            default: None,
            references: None,
            on_delete: None,
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
            tenant_scope_column: None,
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
        validations: Vec::new(),
        tenant_scope: None,
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
        min_length: None,
        max: None,
        min_value: None,
        max_value: None,
        choices: Vec::new(),
        widget: None,
        appearance: None,
        file_upload: None,
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
    assert!(german_response.body.contains("data-z-ui-stylesheet"));
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
fn project_catalog_references_localize_component_properties_in_both_languages() {
    let reference = localize_user_text(UiLanguage::German, "@i18n:invoice.ui.title");
    let route = Route {
        path: "/".into(),
        html: format!("<main><h1 title=\"{reference}\">{reference}</h1></main>"),
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
            r#"{"invoice.ui.title":"Zelyra UI – Components"}"#,
        )
        .unwrap();
    catalogs
        .set_json(
            UiLanguage::German,
            r#"{"invoice.ui.title":"Zelyra UI – Komponenten"}"#,
        )
        .unwrap();

    for (language, title) in [
        (UiLanguage::German, "Zelyra UI – Komponenten"),
        (UiLanguage::English, "Zelyra UI – Components"),
    ] {
        let app = WebApp::new(vec![route.clone()], Vec::new())
            .with_ui_settings(language, UiLevel::Work)
            .with_project_ui_catalogs(catalogs.clone());
        let response = app.dispatch(&request);
        assert!(response
            .body
            .contains(&format!("<h1 title=\"{title}\">{title}</h1>")));
        assert!(!response.body.contains(LOCALE_REFERENCE_START));
    }
}

#[test]
fn project_catalogs_override_generated_ui_and_parameterized_labels() {
    let auth = AuthRoute {
        table: "users".into(),
        login_enabled_column: None,
        membership_table: None,
        tenant_owner_table: None,
        tenant_owner_column: None,
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
        reset_rate_limits_table: None,
        reset_rate_limit: DEFAULT_RESET_RATE_LIMIT,
        reset_block_seconds: DEFAULT_RESET_BLOCK_SECONDS,
        mfa_table: None,
        mfa_recovery_table: None,
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
        login_enabled_column: None,
        membership_table: None,
        tenant_owner_table: None,
        tenant_owner_column: None,
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
        reset_rate_limits_table: None,
        reset_rate_limit: DEFAULT_RESET_RATE_LIMIT,
        reset_block_seconds: DEFAULT_RESET_BLOCK_SECONDS,
        mfa_table: None,
        mfa_recovery_table: None,
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
    assert!(html.contains("<h1 class=\"z-page-header__title\">Kunde anlegen</h1>"));
    assert!(html.contains("<label for=\"name\" class=\"z-field__label\">Name</label>"));
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
fn tenant_api_route_fails_closed_without_authentication_configuration() {
    let called = Arc::new(AtomicUsize::new(0));
    let called_by_handler = called.clone();
    let route = ApiRoute::new_with_tenant_context(
        "POST",
        "/scoped",
        "tenant_id",
        move |_request, _parameters, _tenant_id| {
            called_by_handler.fetch_add(1, Ordering::SeqCst);
            Response::json(200, "{}")
        },
    )
    .with_auth(false, Vec::new());
    assert!(
        route.requires_auth,
        "tenant context cannot disable authentication"
    );
    let app = WebApp::new(Vec::new(), Vec::new()).with_apis(vec![route]);
    let request =
        parse_request("POST /scoped HTTP/1.1\r\nHost: localhost\r\nContent-Length: 2\r\n\r\n{}")
            .unwrap();
    assert_eq!(app.dispatch(&request).status, 401);
    assert_eq!(called.load(Ordering::SeqCst), 0);
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
fn forwarded_host_never_overrides_the_allowlisted_request_host() {
    let app = WebApp::new(
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
    .with_allowed_hosts(vec!["app.example".into()])
    .unwrap();

    let forged = parse_request(
        "GET /health HTTP/1.1\r\nHost: attacker.example\r\nX-Forwarded-Host: app.example\r\n\r\n",
    )
    .unwrap();
    assert_eq!(app.dispatch(&forged).status, 400);

    let allowed_with_forged_forward = parse_request(
        "GET /health HTTP/1.1\r\nHost: app.example\r\nX-Forwarded-Host: attacker.example\r\n\r\n",
    )
    .unwrap();
    assert_eq!(app.dispatch(&allowed_with_forged_forward).status, 200);
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
        login_enabled_column: None,
        membership_table: None,
        tenant_owner_table: None,
        tenant_owner_column: None,
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
        reset_rate_limits_table: None,
        reset_rate_limit: DEFAULT_RESET_RATE_LIMIT,
        reset_block_seconds: DEFAULT_RESET_BLOCK_SECONDS,
        mfa_table: None,
        mfa_recovery_table: None,
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
fn decodes_path_parameters_without_accepting_encoded_path_structure() {
    assert_eq!(
        match_path("/module/{name}", "/module/Anfrage%20A%2B%20%F0%9F%93%84").unwrap()["name"],
        "Anfrage A+ 📄"
    );
    assert!(match_path("/module/{name}", "/module/%2e%2e").is_none());
    assert!(match_path("/module/{name}", "/module/%2Fetc").is_none());
    assert!(match_path("/module/{name}", "/module/%5Cwindows").is_none());
    assert!(match_path("/module/{name}", "/module/%00").is_none());
    assert!(match_path("/module/{name}", "/module/%ZZ").is_none());
    assert!(match_path("/module/{name}", "/module/%E0%A4").is_none());
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
fn static_preview_uses_the_normal_escaping_and_collection_renderer() {
    let values = HashMap::from([("title".into(), "<Invoice>".into())]);
    let collections = HashMap::from([(
        "items".into(),
        vec![HashMap::from([("name".into(), "A & B".into())])],
    )]);
    let html = render_static_template_preview(
        "<h1>{title}</h1><ul>\nfor item in items {<li>{item.name}</li>}\n</ul>",
        &values,
        &collections,
    );
    assert!(html.contains("&lt;Invoice&gt;"));
    assert!(html.contains("A &amp; B"));
    assert!(!html.contains("<Invoice>"));
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
fn public_files_are_served_as_bytes_with_safe_path_and_method_rules() {
    let root = std::env::temp_dir().join(format!(
        "zelyra-public-{}-{}",
        std::process::id(),
        AtomicUsize::new(0).fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(root.join("public/assets")).unwrap();
    fs::create_dir_all(root.join("public/.well-known/acme-challenge")).unwrap();
    fs::write(root.join("public/index.html"), "<h1>Static home</h1>").unwrap();
    fs::write(root.join("public/assets/pixel.png"), [0, 1, 2, 255]).unwrap();
    fs::write(root.join("public/.env"), "secret=value").unwrap();
    fs::write(
        root.join("public/.well-known/acme-challenge/token"),
        "challenge",
    )
    .unwrap();
    fs::write(root.join("public/declared.html"), "static fallback").unwrap();
    fs::File::create(root.join("public/large.bin"))
        .unwrap()
        .set_len(MAX_PUBLIC_FILE_BYTES + 1)
        .unwrap();
    let app = WebApp::new(
        vec![Route {
            path: "/declared.html".into(),
            html: "<h1>Declared route</h1>".into(),
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
    .with_public_dir(&root);
    let request = |method: &str, path: &str| {
        parse_request(&format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\n\r\n"
        ))
        .unwrap()
    };

    let home = app.dispatch(&request("GET", "/"));
    assert_eq!(home.status, 200);
    assert_eq!(home.content_type, "text/html; charset=utf-8");
    assert_eq!(
        home.body_bytes.as_deref(),
        Some(b"<h1>Static home</h1>".as_slice())
    );
    assert!(home
        .headers
        .iter()
        .any(|(name, value)| name == "Content-Security-Policy"
            && value.contains("default-src 'self'")));

    let image = app.dispatch(&request("GET", "/assets/pixel.png"));
    assert_eq!(image.content_type, "image/png");
    assert_eq!(image.body_bytes.as_deref(), Some([0, 1, 2, 255].as_slice()));
    assert!(image.to_http_bytes().ends_with(&[0, 1, 2, 255]));

    let head = app.dispatch(&request("HEAD", "/assets/pixel.png"));
    assert_eq!(head.status, 200);
    assert!(head.body_bytes.as_deref().unwrap().is_empty());
    assert!(head
        .to_http_bytes()
        .windows(b"Content-Length: 4\r\n".len())
        .any(|window| window == b"Content-Length: 4\r\n"));

    assert_eq!(
        app.dispatch(&request("GET", "/%2e%2e/private.txt")).status,
        404
    );
    assert_eq!(
        app.dispatch(&request("GET", "/assets%2fpixel.png")).status,
        404
    );
    assert_eq!(
        app.dispatch(&request("POST", "/assets/pixel.png")).status,
        405
    );
    assert_eq!(app.dispatch(&request("GET", "/.env")).status, 404);
    assert_eq!(
        app.dispatch(&request("GET", "/.well-known/acme-challenge/token"))
            .body_bytes
            .as_deref(),
        Some(b"challenge".as_slice())
    );
    assert_eq!(app.dispatch(&request("GET", "/large.bin")).status, 413);
    assert!(app
        .dispatch(&request("GET", "/declared.html"))
        .body
        .contains("Declared route"));

    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn public_files_reject_symlinks_outside_the_document_root() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!(
        "zelyra-public-symlink-{}-{}",
        std::process::id(),
        AtomicUsize::new(0).fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(root.join("public")).unwrap();
    fs::write(root.join("secret.txt"), "private").unwrap();
    symlink(root.join("secret.txt"), root.join("public/leak.txt")).unwrap();
    let app = WebApp::new(Vec::new(), Vec::new()).with_public_dir(&root);
    let request = parse_request("GET /leak.txt HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();

    assert_eq!(app.dispatch(&request).status, 404);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn public_binary_assets_survive_real_http_serialization() {
    let root = std::env::temp_dir().join(format!(
        "zelyra-public-http-{}-{}",
        std::process::id(),
        AtomicUsize::new(0).fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(root.join("public")).unwrap();
    let image = [0x89, b'P', b'N', b'G', 0, 0xff, 0x80];
    fs::write(root.join("public/image.png"), image).unwrap();
    let app = WebApp::new(Vec::new(), Vec::new()).with_public_dir(&root);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        handle_connection(&mut stream, &app).unwrap();
    });
    let mut client = TcpStream::connect(address).unwrap();
    client
        .write_all(b"GET /image.png HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut response = Vec::new();
    client.read_to_end(&mut response).unwrap();
    server.join().unwrap();

    let separator = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap();
    let headers = std::str::from_utf8(&response[..separator]).unwrap();
    assert!(headers.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(headers.contains("Content-Type: image/png\r\n"));
    assert!(headers.contains("Content-Length: 7\r\n"));
    assert_eq!(&response[separator + 4..], &image);
    fs::remove_dir_all(root).unwrap();
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
        "X-Zelyra-Tenant",
        "X-Zelyra-Editor-Token",
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
fn multipart_binary_request_is_rejected_when_route_declares_no_upload_field() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let app = Arc::new(WebApp::new(Vec::new(), Vec::new()));
    let active_connections = Arc::new(AtomicUsize::new(0));
    let mut client = TcpStream::connect(address).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let (stream, _) = listener.accept().unwrap();
    let worker = spawn_connection(stream, app, active_connections)
        .unwrap()
        .unwrap();

    let body = b"--upload-boundary\r\nContent-Disposition: form-data; name=\"document\"; filename=\"invoice.pdf\"\r\nContent-Type: application/pdf\r\n\r\n%PDF\0\xff\r\n--upload-boundary--\r\n";
    let head = format!(
        "POST /upload HTTP/1.1\r\nHost: localhost\r\nContent-Type: multipart/form-data; boundary=upload-boundary\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    client.write_all(head.as_bytes()).unwrap();
    client.write_all(body).unwrap();
    let mut response = Vec::new();
    client.read_to_end(&mut response).unwrap();
    worker.join().unwrap();
    let response = String::from_utf8(response).unwrap();
    assert!(
        response.starts_with("HTTP/1.1 415 Unsupported Media Type"),
        "{response}"
    );
    assert!(response.contains("this route does not accept multipart form data"));
}

#[test]
fn multipart_reader_uses_declared_form_limit_without_raising_other_routes() {
    let mut route = form_route();
    route.path = "/upload".into();
    route.form.fields[0].ty = Some(Type::Named("File".into()));
    route.form.fields[0].file_upload = Some(zelyra_ast::FileUploadOptions {
        max_size_bytes: 2 * 1024 * 1024,
        allowed_types: vec!["application/pdf".into()],
    });
    let app = Arc::new(WebApp::new(Vec::new(), vec![route]));
    let upload_head = "POST /upload HTTP/1.1\r\nHost: localhost\r\nContent-Type: multipart/form-data; boundary=b\r\n";
    let ordinary_head = "POST /other HTTP/1.1\r\nHost: localhost\r\nContent-Type: multipart/form-data; boundary=b\r\n";
    assert_eq!(
        app.request_body_limit_from_headers(upload_head),
        3 * 1024 * 1024
    );
    assert_eq!(
        app.request_body_limit_from_headers(ordinary_head),
        MAX_REQUEST_BODY_BYTES
    );
    assert_eq!(
        app.request_body_limit_from_headers(&upload_head.replace(
            "Content-Type:",
            "Content-Type: multipart/form-data; boundary=b\r\nContent-Type:",
        )),
        MAX_REQUEST_BODY_BYTES
    );

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let active_connections = Arc::new(AtomicUsize::new(0));
    let mut client = TcpStream::connect(address).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let (stream, _) = listener.accept().unwrap();
    let worker = spawn_connection(stream, app, active_connections)
        .unwrap()
        .unwrap();
    let payload = vec![b'x'; MAX_REQUEST_BODY_BYTES + 128];
    let mut body = b"--b\r\nContent-Disposition: form-data; name=\"document\"; filename=\"invoice.pdf\"\r\nContent-Type: application/pdf\r\n\r\n".to_vec();
    body.extend_from_slice(&payload);
    body.extend_from_slice(b"\r\n--b--\r\n");
    let head = format!(
        "POST /upload HTTP/1.1\r\nHost: localhost\r\nContent-Type: multipart/form-data; boundary=b\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    client.write_all(head.as_bytes()).unwrap();
    client.write_all(&body).unwrap();
    let mut response = Vec::new();
    client.read_to_end(&mut response).unwrap();
    worker.join().unwrap();
    let response = String::from_utf8(response).unwrap();
    assert!(response.starts_with("HTTP/1.1 403 Forbidden"), "{response}");
    assert!(response.contains("Invalid CSRF token."));
}

#[test]
fn multipart_upload_stages_only_after_csrf_and_cleans_up_when_database_is_unavailable() {
    let root = std::env::temp_dir().join(format!(
        "zelyra-multipart-form-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut route = form_route();
    route.path = "/upload".into();
    route.form.fields = vec![zelyra_ast::FormField {
        name: "document".into(),
        ty: Some(Type::Named("File".into())),
        label: Some("Document".into()),
        placeholder: None,
        required: true,
        min_length: None,
        max: None,
        min_value: None,
        max_value: None,
        choices: Vec::new(),
        widget: None,
        appearance: None,
        file_upload: Some(zelyra_ast::FileUploadOptions {
            max_size_bytes: 1024,
            allowed_types: vec!["application/pdf".into()],
        }),
        readonly: false,
        span: zelyra_ast::Span::default(),
    }];
    route.form.actions = vec![zelyra_ast::FormAction {
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
        capabilities: Vec::new(),
        require_match: false,
        statements: vec![zelyra_ast::Stmt::Expr(zelyra_ast::Expr {
            kind: zelyra_ast::ExprKind::Sql {
                result_type: Type::Unit,
                query: "INSERT INTO uploads (document) VALUES (:document)".into(),
            },
            span: zelyra_ast::Span::default(),
        })],
        execute_body: None,
        guard: None,
        success: None,
        redirect: None,
        span: zelyra_ast::Span::default(),
    }];
    let app = WebApp::new(Vec::new(), vec![route]).with_upload_storage(&root);
    let data = b"%PDF-1.7\nminimal test document";
    let parts = [multipart::MultipartPart {
        name: "document".into(),
        filename: Some("invoice.pdf".into()),
        content_type: Some("application/pdf".into()),
        data,
    }];
    let request = Request {
        method: "POST".into(),
        target: "/upload".into(),
        path: "/upload".into(),
        headers: HashMap::from([
            ("host".into(), "localhost".into()),
            ("origin".into(), "http://localhost".into()),
        ]),
        body: "_zelyra_csrf=csrf-token".into(),
        remote_addr: None,
    };

    let invalid_csrf = Request {
        body: "_zelyra_csrf=wrong".into(),
        ..request.clone()
    };
    assert_eq!(
        app.dispatch_with_multipart(&invalid_csrf, &parts).status,
        403
    );
    assert!(!root.exists(), "CSRF failure must precede storage creation");

    assert_eq!(app.dispatch_with_multipart(&request, &parts).status, 503);
    let tenant_dir = root.join("tenants/0");
    assert!(tenant_dir.is_dir());
    assert_eq!(
        std::fs::read_dir(&tenant_dir).unwrap().count(),
        0,
        "a failed database action must remove published bytes and manifests"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires a disposable MariaDB URL in ZELYRA_UPLOAD_TEST_URL"]
fn multipart_upload_commits_reference_and_removes_file_after_sql_rollback() {
    let database_url = std::env::var("ZELYRA_UPLOAD_TEST_URL")
        .expect("the upload E2E script provides a disposable MariaDB URL");
    zelyra_database::execute_query(
        &database_url,
        "DROP TABLE IF EXISTS zelyra_upload_shared_e2e",
        Vec::new(),
    )
    .expect("drop test-only shared upload reference table");
    zelyra_database::execute_query(
        &database_url,
        "DROP TABLE IF EXISTS zelyra_upload_e2e",
        Vec::new(),
    )
    .expect("drop test-only upload table");
    zelyra_database::execute_query(
        &database_url,
        "CREATE TABLE zelyra_upload_e2e (id BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY, document CHAR(36) NULL, title VARCHAR(80) NULL, owner_id BIGINT NOT NULL DEFAULT 7, tenant_id BIGINT NOT NULL DEFAULT 0)",
        Vec::new(),
    )
    .expect("create test-only upload table");
    zelyra_database::execute_query(
        &database_url,
        "CREATE TABLE zelyra_upload_shared_e2e (id BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY, document CHAR(36) NULL, owner_id BIGINT NOT NULL DEFAULT 7, tenant_id BIGINT NOT NULL DEFAULT 0)",
        Vec::new(),
    )
    .expect("create test-only shared upload reference table");
    zelyra_database::execute_query(
        &database_url,
        "DROP TABLE IF EXISTS zelyra_upload_membership_e2e",
        Vec::new(),
    )
    .expect("drop test-only upload membership table");
    zelyra_database::execute_query(
        &database_url,
        "CREATE TABLE zelyra_upload_membership_e2e (id BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY, user_id BIGINT NOT NULL, tenant_id BIGINT NOT NULL)",
        Vec::new(),
    )
    .expect("create test-only upload membership table");
    let storage_root = std::env::temp_dir().join(format!(
        "zelyra-upload-e2e-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut route = form_route();
    route.path = "/upload".into();
    route.form.fields = vec![zelyra_ast::FormField {
        name: "document".into(),
        ty: Some(Type::Named("File".into())),
        label: Some("Document".into()),
        placeholder: None,
        required: true,
        min_length: None,
        max: None,
        min_value: None,
        max_value: None,
        choices: Vec::new(),
        widget: None,
        appearance: None,
        file_upload: Some(zelyra_ast::FileUploadOptions {
            max_size_bytes: 1024,
            allowed_types: vec!["application/pdf".into()],
        }),
        readonly: false,
        span: zelyra_ast::Span::default(),
    }];
    let sql_action = |statements: Vec<&str>| zelyra_ast::FormAction {
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
        capabilities: Vec::new(),
        require_match: false,
        statements: statements
            .into_iter()
            .map(|query| {
                zelyra_ast::Stmt::Expr(zelyra_ast::Expr {
                    kind: zelyra_ast::ExprKind::Sql {
                        result_type: Type::Unit,
                        query: query.into(),
                    },
                    span: zelyra_ast::Span::default(),
                })
            })
            .collect(),
        execute_body: None,
        guard: None,
        success: None,
        redirect: None,
        span: zelyra_ast::Span::default(),
    };
    route.form.actions = vec![sql_action(vec![
        "INSERT INTO zelyra_upload_e2e (document, owner_id) VALUES (:document, 7)",
    ])];
    let app =
        WebApp::with_database_url(Vec::new(), vec![route.clone()], Some(database_url.clone()))
            .with_upload_storage(&storage_root);
    let bytes = b"%PDF-1.7\ntransactional upload";
    let boundary = "zelyra-upload-e2e-boundary";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"_zelyra_csrf\"\r\n\r\ncsrf-token\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"document\"; filename=\"document.pdf\"\r\nContent-Type: application/pdf\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind HTTP upload test listener");
    let address = listener
        .local_addr()
        .expect("read HTTP upload test address");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept HTTP upload test request");
        handle_connection(&mut stream, &app).expect("handle HTTP upload test request");
    });
    let mut client = TcpStream::connect(address).expect("connect to HTTP upload test server");
    client
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("set HTTP upload test timeout");
    write!(
        client,
        "POST /upload HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\nContent-Type: multipart/form-data; boundary={boundary}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .expect("write HTTP upload test headers");
    client
        .write_all(&body)
        .expect("write HTTP upload test body");
    let mut response = Vec::new();
    client
        .read_to_end(&mut response)
        .expect("read HTTP upload test response");
    server.join().expect("join HTTP upload test server");
    let response_head = String::from_utf8_lossy(&response);
    assert!(
        response_head.starts_with("HTTP/1.1 303 See Other\r\n"),
        "{response_head}"
    );
    let rows = zelyra_database::execute_read_only_query(
        &database_url,
        "SELECT id, document FROM zelyra_upload_e2e",
        Vec::new(),
    )
    .expect("read saved file reference");
    assert_eq!(rows.rows.len(), 1);
    let id = rows.rows[0][0].clone();
    let file_ref = &rows.rows[0][1];
    let storage = file_storage::FileStorage::new(&storage_root).expect("open managed storage");
    let (metadata, saved_bytes) = storage
        .read_verified(0, file_ref, 1024)
        .expect("verify stored upload referenced by committed row");
    assert_eq!(metadata.media_type, "application/pdf");
    assert_eq!(saved_bytes, bytes);

    let edit_program = zelyra_parser::parse(
        &zelyra_lexer::lex(
            r#"
            database main { engine: mariadb }
            table zelyra_upload_e2e {
                id: Id primary auto
                document: FileRef?
                title: String(80)?
                owner_id: Int required default 7
                tenant_id: Int required default 0
            }
            table zelyra_upload_shared_e2e {
                id: Id primary auto
                document: FileRef?
                owner_id: Int required default 7
                tenant_id: Int required default 0
            }
            fn main() {}
            "#,
        )
        .expect("lex upload edit metadata"),
    )
    .expect("parse upload edit metadata");
    let schema = zelyra_database::build_schema(&edit_program).expect("build edit schema");
    let mut edit_route = route.clone();
    edit_route.path = "/upload/{id}".into();
    edit_route.action = edit_route.path.clone();
    edit_route.form.name = "DocumentEdit".into();
    edit_route.form.table = Some("zelyra_upload_e2e".into());
    edit_route.table = edit_program.tables.first().cloned();
    edit_route.schema = Some(schema);
    edit_route.form.fields = vec![
        zelyra_ast::FormField {
            name: "document".into(),
            ty: Some(Type::Named("File".into())),
            label: Some("Document".into()),
            placeholder: None,
            required: true,
            min_length: None,
            max: None,
            min_value: None,
            max_value: None,
            choices: Vec::new(),
            widget: None,
            appearance: None,
            file_upload: Some(zelyra_ast::FileUploadOptions {
                max_size_bytes: 1024,
                allowed_types: vec!["application/pdf".into()],
            }),
            readonly: false,
            span: zelyra_ast::Span::default(),
        },
        zelyra_ast::FormField {
            name: "title".into(),
            ty: Some(Type::String),
            label: Some("Title".into()),
            placeholder: None,
            required: true,
            min_length: None,
            max: Some(80),
            min_value: None,
            max_value: None,
            choices: Vec::new(),
            widget: None,
            appearance: None,
            file_upload: None,
            readonly: false,
            span: zelyra_ast::Span::default(),
        },
    ];
    edit_route.form.actions = vec![sql_action(vec![
        "UPDATE zelyra_upload_e2e SET document = :document, title = :title WHERE id = :id",
    ])];
    let edit_app =
        WebApp::with_database_url(Vec::new(), vec![edit_route], Some(database_url.clone()))
            .with_upload_storage(&storage_root);
    let edit_get = edit_app.dispatch(&Request {
        method: "GET".into(),
        target: format!("/upload/{id}"),
        path: format!("/upload/{id}"),
        headers: HashMap::from([("host".into(), "localhost".into())]),
        body: String::new(),
        remote_addr: None,
    });
    assert_eq!(edit_get.status, 200, "{}", edit_get.body);
    assert!(edit_get.body.contains("type=\"file\""));
    assert!(!edit_get
        .body
        .contains("type=\"file\" accept=\"application/pdf\" required"));
    assert!(
        !edit_get.body.contains(file_ref),
        "the file input must not echo the stored FileRef"
    );
    let snapshot = edit_get
        .body
        .split("name=\"_zelyra_snapshot\" value=\"")
        .nth(1)
        .and_then(|value| value.split('\"').next())
        .expect("edit GET includes a signed row snapshot");
    let edit_post = edit_app.dispatch(&Request {
        method: "POST".into(),
        target: format!("/upload/{id}"),
        path: format!("/upload/{id}"),
        headers: HashMap::from([
            ("host".into(), "localhost".into()),
            ("origin".into(), "http://localhost".into()),
        ]),
        body: format!(
            "_zelyra_csrf=csrf-token&_zelyra_snapshot={snapshot}&document=123e4567-e89b-42d3-a456-426614174099&title=Updated"
        ),
        remote_addr: None,
    });
    assert_eq!(edit_post.status, 303, "{}", edit_post.body);
    let edited = zelyra_database::execute_read_only_query(
        &database_url,
        "SELECT document, title FROM zelyra_upload_e2e WHERE id = :id",
        vec![(
            "id".into(),
            zelyra_database::QueryValue::Int(id.parse().unwrap()),
        )],
    )
    .expect("read edit result");
    assert_eq!(
        edited.rows[0][0], *file_ref,
        "an edit without a new upload preserves the stored reference"
    );
    assert_eq!(edited.rows[0][1], "Updated");
    assert!(storage.read_verified(0, file_ref, 1024).is_ok());

    let replacement_get = edit_app.dispatch(&Request {
        method: "GET".into(),
        target: format!("/upload/{id}"),
        path: format!("/upload/{id}"),
        headers: HashMap::from([("host".into(), "localhost".into())]),
        body: String::new(),
        remote_addr: None,
    });
    assert_eq!(replacement_get.status, 200, "{}", replacement_get.body);
    let replacement_snapshot = replacement_get
        .body
        .split("name=\"_zelyra_snapshot\" value=\"")
        .nth(1)
        .and_then(|value| value.split('\"').next())
        .expect("replacement edit GET includes a signed row snapshot");
    let replacement_bytes = b"%PDF-1.7\nreplacement document";
    let replacement = edit_app.dispatch_with_multipart(
        &Request {
            method: "POST".into(),
            target: format!("/upload/{id}"),
            path: format!("/upload/{id}"),
            headers: HashMap::from([
                ("host".into(), "localhost".into()),
                ("origin".into(), "http://localhost".into()),
            ]),
            body: format!(
                "_zelyra_csrf=csrf-token&_zelyra_snapshot={replacement_snapshot}&title=Replaced"
            ),
            remote_addr: None,
        },
        &[multipart::MultipartPart {
            name: "document".into(),
            filename: Some("replacement.pdf".into()),
            content_type: Some("application/pdf".into()),
            data: replacement_bytes,
        }],
    );
    assert_eq!(replacement.status, 303, "{}", replacement.body);
    let replaced = zelyra_database::execute_read_only_query(
        &database_url,
        "SELECT document FROM zelyra_upload_e2e WHERE id = :id",
        vec![(
            "id".into(),
            zelyra_database::QueryValue::Int(id.parse().unwrap()),
        )],
    )
    .expect("read replacement file reference");
    let replacement_ref = &replaced.rows[0][0];
    assert_ne!(replacement_ref, file_ref);
    assert!(
        storage.read_verified(0, file_ref, 1024).is_err(),
        "replaced upload should be removed after the database action commits"
    );
    assert_eq!(
        storage
            .read_verified(0, replacement_ref, 1024)
            .expect("replacement upload remains readable")
            .1,
        replacement_bytes
    );

    let program = zelyra_parser::parse(
        &zelyra_lexer::lex(
            r#"
            database main { engine: mariadb }
            table zelyra_upload_e2e {
                id: Id primary auto
                document: FileRef? attachment {
                    max_size: "1KB"
                    allowed_types: ["application/pdf"]
                }
                owner_id: Int required
                tenant_id: Int required default 7
            }
            table zelyra_upload_shared_e2e {
                id: Id primary auto
                document: FileRef?
                owner_id: Int required default 7
                tenant_id: Int required default 7
            }
            table zelyra_upload_membership_e2e {
                id: Id primary auto
                user_id: Id required
                tenant_id: Int required
            }
            form Upload -> zelyra_upload_e2e {
                field document: File { max_size: "1KB" allowed_types: ["application/pdf"] }
            }
            fn main() {}
            "#,
        )
        .expect("lex upload download route metadata"),
    )
    .expect("parse upload download route metadata");
    let mut crud = default_shell_crud(None);
    crud.path = "/uploads".into();
    crud.table = "zelyra_upload_e2e".into();
    crud.recipient_column = Some("owner_id".into());
    crud.schema = zelyra_database::build_schema(&program).expect("build upload table schema");
    let path_params = HashMap::from([
        ("id".into(), id.clone()),
        ("field".into(), "document".into()),
    ]);
    let download = dispatch_crud_file_download(
        &crud,
        &Request {
            method: "GET".into(),
            target: format!("/uploads/{id}/files/document"),
            path: format!("/uploads/{id}/files/document"),
            headers: HashMap::new(),
            body: String::new(),
            remote_addr: None,
        },
        &path_params,
        CrudFileDownloadContext {
            database_url: Some(&database_url),
            storage_root: Some(&storage_root),
            validation_program: Some(&program),
            actor_user_id: Some(7),
            tenant_context: None,
        },
    );
    assert_eq!(download.status, 200);
    assert_eq!(download.content_type, "application/pdf");
    assert_eq!(
        download.body_bytes.as_deref(),
        Some(replacement_bytes.as_slice())
    );
    assert_eq!(
        download
            .headers
            .iter()
            .find(|(name, _)| name == "Content-Disposition")
            .map(|(_, value)| value.as_str()),
        Some("attachment; filename=\"download.pdf\"")
    );
    assert!(download
        .headers
        .iter()
        .any(|(name, value)| name == "Cache-Control" && value == "private, no-store"));
    zelyra_database::execute_query(
        &database_url,
        "INSERT INTO zelyra_upload_membership_e2e (user_id, tenant_id) VALUES (7, 0)",
        Vec::new(),
    )
    .expect("grant the test actor access to tenant 7");
    let mut tenant_crud = crud.clone();
    tenant_crud.tenant_column = Some("tenant_id".into());
    let tenant_context = TenantContext {
        user_id: 7,
        tenant_id: 0,
        membership_table: Some("zelyra_upload_membership_e2e".into()),
        membership_has_active: false,
        tenant_owner_table: None,
        tenant_owner_column: None,
    };
    let tenant_download = dispatch_crud_file_download(
        &tenant_crud,
        &Request {
            method: "GET".into(),
            target: format!("/uploads/{id}/files/document"),
            path: format!("/uploads/{id}/files/document"),
            headers: HashMap::new(),
            body: String::new(),
            remote_addr: None,
        },
        &path_params,
        CrudFileDownloadContext {
            database_url: Some(&database_url),
            storage_root: Some(&storage_root),
            validation_program: Some(&program),
            actor_user_id: Some(7),
            tenant_context: Some(&tenant_context),
        },
    );
    assert_eq!(tenant_download.status, 200, "tenant member can download");
    let other_tenant = TenantContext {
        tenant_id: 8,
        ..tenant_context.clone()
    };
    let cross_tenant_download = dispatch_crud_file_download(
        &tenant_crud,
        &Request {
            method: "GET".into(),
            target: format!("/uploads/{id}/files/document"),
            path: format!("/uploads/{id}/files/document"),
            headers: HashMap::new(),
            body: String::new(),
            remote_addr: None,
        },
        &path_params,
        CrudFileDownloadContext {
            database_url: Some(&database_url),
            storage_root: Some(&storage_root),
            validation_program: Some(&program),
            actor_user_id: Some(7),
            tenant_context: Some(&other_tenant),
        },
    );
    assert_eq!(
        cross_tenant_download.status, 404,
        "a different tenant cannot discover or download the attachment"
    );
    let unauthenticated_app =
        WebApp::with_database_url(Vec::new(), Vec::new(), Some(database_url.clone()))
            .with_cruds(vec![crud.clone()])
            .with_validation_program(program.clone())
            .with_upload_storage(&storage_root);
    assert_ne!(
        unauthenticated_app
            .dispatch(&Request {
                method: "GET".into(),
                target: format!("/uploads/{id}/files/document"),
                path: format!("/uploads/{id}/files/document"),
                headers: HashMap::new(),
                body: String::new(),
                remote_addr: None,
            })
            .status,
        200,
        "the generated file route requires an authenticated session"
    );
    assert_eq!(
        dispatch_crud_file_download(
            &crud,
            &Request {
                method: "GET".into(),
                target: format!("/uploads/{id}/files/document"),
                path: format!("/uploads/{id}/files/document"),
                headers: HashMap::new(),
                body: String::new(),
                remote_addr: None,
            },
            &path_params,
            CrudFileDownloadContext {
                database_url: Some(&database_url),
                storage_root: Some(&storage_root),
                validation_program: Some(&program),
                actor_user_id: Some(8),
                tenant_context: None,
            },
        )
        .status,
        404,
        "file download must not cross the CRUD recipient boundary"
    );
    let file_columns = crud_file_fields(Some(&program), &crud.table)
        .into_keys()
        .collect::<Vec<_>>();
    assert_eq!(file_columns, vec!["document"]);
    let detail_html = localize_html(
        &render_crud_detail_with_actions(
            &crud,
            &["document"],
            std::slice::from_ref(file_ref),
            &id,
            CrudUiActions {
                create: false,
                edit: false,
                delete: false,
                restore: false,
                custom: Vec::new(),
            },
            UiLanguage::English,
            &file_columns,
        ),
        UiLanguage::English,
    );
    assert!(detail_html.contains(&format!("/uploads/{id}/files/document")));
    assert!(detail_html.contains("Download file"));
    assert!(
        !detail_html.contains(file_ref),
        "opaque references are not shown"
    );

    let parts = [multipart::MultipartPart {
        name: "document".into(),
        filename: Some("document.pdf".into()),
        content_type: Some("application/pdf".into()),
        data: bytes,
    }];
    let request = Request {
        method: "POST".into(),
        target: "/upload".into(),
        path: "/upload".into(),
        headers: HashMap::from([
            ("host".into(), "localhost".into()),
            ("origin".into(), "http://localhost".into()),
        ]),
        body: "_zelyra_csrf=csrf-token".into(),
        remote_addr: None,
    };
    let mut failing_route = route;
    failing_route.form.actions = vec![sql_action(vec![
        "INSERT INTO zelyra_upload_e2e (document, owner_id) VALUES (:document, 7)",
        "INSERT INTO zelyra_upload_missing_table (document) VALUES (:document)",
    ])];
    let failing_app =
        WebApp::with_database_url(Vec::new(), vec![failing_route], Some(database_url.clone()))
            .with_upload_storage(&storage_root);
    let rolled_back = failing_app.dispatch_with_multipart(&request, &parts);
    assert_eq!(rolled_back.status, 500, "{}", rolled_back.body);
    let rows = zelyra_database::execute_read_only_query(
        &database_url,
        "SELECT id, document FROM zelyra_upload_e2e",
        Vec::new(),
    )
    .expect("read rows after transaction rollback");
    assert_eq!(rows.rows.len(), 1, "failed SQL action must roll back");
    assert!(storage.read_verified(0, replacement_ref, 1024).is_ok());
    let tenant_dir = storage_root.join("tenants/0");
    assert_eq!(
        std::fs::read_dir(&tenant_dir).unwrap().count(),
        2,
        "failed SQL action must remove only its staged/published file and manifest"
    );

    let mut delete_crud = crud.clone();
    delete_crud.csrf = CsrfProtection::new("csrf-token");
    delete_crud.schema = zelyra_database::build_schema(&edit_program).expect("build delete schema");
    zelyra_database::execute_query(
        &database_url,
        "INSERT INTO zelyra_upload_shared_e2e (document, owner_id) VALUES (:document, 7)",
        vec![(
            "document".into(),
            zelyra_database::QueryValue::String(replacement_ref.clone()),
        )],
    )
    .expect("add a second declared reference to the replacement file");
    let delete_response = dispatch_crud_delete(
        &delete_crud,
        &Request {
            method: "POST".into(),
            target: format!("/uploads/{id}/delete"),
            path: format!("/uploads/{id}/delete"),
            headers: HashMap::from([
                ("host".into(), "localhost".into()),
                ("origin".into(), "http://localhost".into()),
            ]),
            body: "_zelyra_csrf=csrf-token".into(),
            remote_addr: None,
        },
        &HashMap::from([("id".into(), id.clone())]),
        Some(&storage_root),
        CrudMutationContext {
            database_url: Some(&database_url),
            audit_table: None,
            audit_chain: false,
            actor_user_id: Some(7),
            tenant_context: None,
            validation_program: Some(&edit_program),
            crud_policy_functions: &[],
            granted_permissions: Vec::new(),
        },
    );
    assert_eq!(delete_response.status, 303, "{}", delete_response.body);
    assert!(
        storage.read_verified(0, replacement_ref, 1024).is_ok(),
        "deleting one row must retain an upload still referenced by another row"
    );
    let mut delete_shared_crud = delete_crud.clone();
    delete_shared_crud.table = "zelyra_upload_shared_e2e".into();
    let delete_shared_response = dispatch_crud_delete(
        &delete_shared_crud,
        &Request {
            method: "POST".into(),
            target: "/shared/1/delete".into(),
            path: "/shared/1/delete".into(),
            headers: HashMap::from([
                ("host".into(), "localhost".into()),
                ("origin".into(), "http://localhost".into()),
            ]),
            body: "_zelyra_csrf=csrf-token".into(),
            remote_addr: None,
        },
        &HashMap::from([("id".into(), "1".into())]),
        Some(&storage_root),
        CrudMutationContext {
            database_url: Some(&database_url),
            audit_table: None,
            audit_chain: false,
            actor_user_id: Some(7),
            tenant_context: None,
            validation_program: Some(&edit_program),
            crud_policy_functions: &[],
            granted_permissions: Vec::new(),
        },
    );
    assert_eq!(
        delete_shared_response.status, 303,
        "{}",
        delete_shared_response.body
    );
    assert!(
        storage.read_verified(0, replacement_ref, 1024).is_err(),
        "the final permanent CRUD deletion should remove its unreferenced upload after commit"
    );

    zelyra_database::execute_query(
        &database_url,
        "DROP TABLE zelyra_upload_shared_e2e",
        Vec::new(),
    )
    .expect("drop test-only shared upload reference table");
    zelyra_database::execute_query(
        &database_url,
        "DROP TABLE zelyra_upload_membership_e2e",
        Vec::new(),
    )
    .expect("drop test-only upload membership table");
    zelyra_database::execute_query(&database_url, "DROP TABLE zelyra_upload_e2e", Vec::new())
        .expect("drop test-only upload table");
    std::fs::remove_dir_all(storage_root).unwrap();
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
        login_enabled_column: None,
        membership_table: None,
        tenant_owner_table: None,
        tenant_owner_column: None,
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
        reset_rate_limits_table: None,
        reset_rate_limit: DEFAULT_RESET_RATE_LIMIT,
        reset_block_seconds: DEFAULT_RESET_BLOCK_SECONDS,
        mfa_table: None,
        mfa_recovery_table: None,
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
        login_enabled_column: None,
        membership_table: None,
        tenant_owner_table: None,
        tenant_owner_column: None,
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
        reset_rate_limits_table: None,
        reset_rate_limit: DEFAULT_RESET_RATE_LIMIT,
        reset_block_seconds: DEFAULT_RESET_BLOCK_SECONDS,
        mfa_table: None,
        mfa_recovery_table: None,
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
        login_enabled_column: None,
        membership_table: None,
        tenant_owner_table: None,
        tenant_owner_column: None,
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
        reset_rate_limits_table: None,
        reset_rate_limit: DEFAULT_RESET_RATE_LIMIT,
        reset_block_seconds: DEFAULT_RESET_BLOCK_SECONDS,
        mfa_table: None,
        mfa_recovery_table: None,
        schema: Schema {
            database: None,
            tables: vec![zelyra_database::Table {
                name: "sessions".into(),
                tenant_scope_column: None,
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
    let mut route = form_route();
    route.form.fields[0].min_length = Some(3);
    let mut amount = zelyra_ast::FormField {
        name: "amount".into(),
        ty: Some(Type::Decimal),
        label: Some("Amount".into()),
        placeholder: None,
        required: false,
        min_length: None,
        max: None,
        min_value: Some("0.10".into()),
        max_value: Some("12.50".into()),
        choices: Vec::new(),
        widget: None,
        appearance: None,
        file_upload: None,
        readonly: false,
        span: zelyra_ast::Span::default(),
    };
    amount.required = true;
    route.form.fields.push(amount);
    let mut priority = zelyra_ast::FormField {
        name: "priority".into(),
        ty: Some(Type::Int),
        label: Some("Priority".into()),
        placeholder: None,
        required: false,
        min_length: None,
        max: None,
        min_value: Some("1".into()),
        max_value: Some("9".into()),
        choices: Vec::new(),
        widget: None,
        appearance: None,
        file_upload: None,
        readonly: false,
        span: zelyra_ast::Span::default(),
    };
    priority.required = true;
    route.form.fields.push(priority);
    route.form.fields.push(zelyra_ast::FormField {
        name: "status".into(),
        ty: Some(Type::String),
        label: Some("Status".into()),
        placeholder: None,
        required: false,
        min_length: None,
        max: None,
        min_value: None,
        max_value: None,
        choices: vec!["draft".into(), "<script>".into()],
        widget: None,
        appearance: None,
        file_upload: None,
        readonly: false,
        span: zelyra_ast::Span::default(),
    });
    let html = render_form(&route, &HashMap::new(), &[], None);
    assert!(html.contains("name=\"_zelyra_csrf\" value=\"csrf-token\""));
    assert!(html.contains("name=\"name\" type=\"text\""));
    assert!(html.contains(" required"));
    assert!(html.contains("maxlength=\"20\""));
    assert!(html.contains("minlength=\"3\""));
    assert!(html.contains("name=\"amount\" type=\"text\" inputmode=\"decimal\""));
    assert!(html.contains("name=\"priority\" type=\"number\""));
    assert!(html.contains("min=\"1\""));
    assert!(html.contains("max=\"9\""));
    assert!(html.contains("<select class=\""));
    assert!(html.contains("id=\"status\" name=\"status\""));
    assert!(html.contains("&lt;script&gt;</option>"));
    assert!(!html.contains("<script>"));

    let mut cards_route = route.clone();
    cards_route.form_view.mode = CrudFormViewMode::Cards;
    cards_route.form_view.title = Some("Customer form".into());
    cards_route.form_view.submit = Some("Save customer <now>".into());
    let cards_html = render_form(&cards_route, &HashMap::new(), &[], None);
    assert!(cards_html.starts_with("<section class=\"z-generated-page\"><h1"));
    assert!(cards_html.ends_with("</section></section>"));
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
fn file_form_renders_multipart_without_echoing_submitted_file_names() {
    let mut route = form_route();
    route.form.fields[0].ty = Some(Type::Named("File".into()));
    route.form.fields[0].required = true;
    route.form.fields[0].file_upload = Some(zelyra_ast::FileUploadOptions {
        max_size_bytes: 10 * 1024 * 1024,
        allowed_types: vec!["application/pdf".into()],
    });
    let values = HashMap::from([("name".into(), "../../private.pdf".into())]);
    let html = render_form(&route, &values, &[], None);
    assert!(html.contains("<form method=\"post\" enctype=\"multipart/form-data\""));
    assert!(html.contains("id=\"name\" name=\"name\" type=\"file\""));
    assert!(!html.contains("value=\"../../private.pdf\""));
    assert!(!html.contains("type=\"file\" value="));
}

#[test]
fn form_appearance_is_separate_from_server_validation_state() {
    let mut route = form_route();
    route.form.fields[0].appearance = Some("danger".into());
    let appearance_only = render_form(&route, &HashMap::new(), &[], None);
    assert!(appearance_only.contains("class=\"z-input z-input--danger\""));
    assert!(!appearance_only.contains("aria-invalid=\"true\""));

    let invalid = vec![FieldError {
        field: "name".into(),
        message: "value is required".into(),
    }];
    let validation_error = render_form(&route, &HashMap::new(), &invalid, None);
    assert!(validation_error.contains("class=\"z-input z-input--danger\""));
    assert!(validation_error.contains("aria-invalid=\"true\""));

    let mut checkbox = route.form.fields[0].clone();
    checkbox.name = "active".into();
    checkbox.label = Some("Active".into());
    checkbox.ty = Some(Type::Bool);
    checkbox.appearance = Some("warning".into());
    route.form.fields.push(checkbox);
    let checkbox_html = render_form(&route, &HashMap::new(), &[], None);
    assert!(checkbox_html.contains("z-field--checkbox"));
    assert!(checkbox_html.contains("class=\"z-check z-check--warning\""));
    assert!(!checkbox_html.contains("class=\"z-check z-check--warning\" id=\"active\" name=\"active\" type=\"checkbox\" aria-invalid=\"true\""));

    let mut password = route.form.fields[0].clone();
    password.name = "password".into();
    password.label = Some("Password".into());
    password.widget = Some("password".into());
    route.form.fields.push(password);
    let mut token = route.form.fields[0].clone();
    token.name = "api_token".into();
    token.label = Some("API token".into());
    route.form.fields.push(token);
    let submitted = HashMap::from([
        ("name".into(), "Alice".into()),
        ("password".into(), "do-not-reflect-this".into()),
        ("api_token".into(), "also-do-not-reflect-this".into()),
    ]);
    let html = render_form(&route, &submitted, &invalid, None);
    assert!(html.contains("id=\"password\" name=\"password\" type=\"password\" value=\"\""));
    assert!(!html.contains("do-not-reflect-this"));
    assert!(!html.contains("also-do-not-reflect-this"));

    let mut relation_route = relation_form_route();
    relation_route.form.fields[1].appearance = Some("success".into());
    let options = HashMap::from([(
        "department".into(),
        vec![SelectOption {
            value: "7".into(),
            label: "Sales".into(),
        }],
    )]);
    let select_html =
        render_form_with_options(&relation_route, &HashMap::new(), &[], None, &options);
    assert!(select_html.contains("<select class=\"z-select z-select--success\""));
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
    let select_start = html.find("<select ").unwrap();
    let select_end = select_start + html[select_start..].find('>').unwrap();
    let select_tag = &html[select_start..=select_end];
    assert!(select_tag.contains("class=\"z-select"));
    assert!(select_tag.contains("id=\"department\""));
    assert!(select_tag.contains("name=\"department\""));
    assert!(select_tag.contains("required"));
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
        tenant_scope_column: None,
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
        filter_condition(&table, "name", FilterOperator::Contains, false),
        "`base`.`name` LIKE CONCAT('%', :filter_name, '%')"
    );
    assert_eq!(
        filter_condition(&table, "name", FilterOperator::IsNull, false),
        "`base`.`name` IS NULL"
    );
    assert_eq!(
        filter_condition(&table, "name", FilterOperator::Contains, true),
        "\"base\".\"name\" LIKE CONCAT('%', :filter_name, '%')"
    );
    assert_eq!(
        filter_condition(&table, "name", FilterOperator::IsNull, true),
        "\"base\".\"name\" IS NULL"
    );
}

#[test]
fn renders_tableview_with_escaped_values_and_pagination_state() {
    let tableview = TableViewRoute {
        path: "/views/customers".into(),
        title: "Customers".into(),
        layout_html: None,
        source: "SELECT id, name FROM customers".into(),
        columns: vec!["id".into(), "name".into()],
        row_fields: Vec::new(),
        column_content: Vec::new(),
        before_content_html: None,
        after_content_html: None,
        row_actions: None,
        policy_row_actions: None,
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
        &[vec![false, false]],
        TableViewRenderState {
            query_values: &query_values,
            search: "CNC machine",
            sort: "name",
            order: "DESC",
            page: 2,
            per_page: 1,
        },
        UiLanguage::English,
        TableViewPolicyContext {
            program: None,
            functions: &[],
            granted_permissions: Vec::new(),
            edit_permission_granted: true,
            delete_permission_granted: true,
        },
    );
    let html = localize_html(&html, UiLanguage::English);
    assert!(html.contains("&lt;unsafe&gt;"));
    assert!(html.contains("value=\"CNC machine\""));
    assert!(html.contains("page=1&amp;sort=name&amp;order=desc&amp;search=CNC%20machine"));
    assert!(html.contains("name=\"sort\""));
    assert!(html.contains("class=\"z-generated-page\""));
    assert!(html.contains("class=\"z-table-tools\""));
    assert!(html.contains("class=\"z-table\""));
    assert!(html.contains("<caption>Customers</caption>"));
}

#[test]
fn renders_crud_page_slots_and_field_content_with_escaped_values_outside_form() {
    let mut route = form_route();
    route.before_form_html = Some("<p class=\"before\">{values.name}</p>".into());
    route.after_form_html = Some("<p class=\"after\">Done</p>".into());
    route.form_context_fields = vec!["name".into()];
    route
        .field_content_html
        .insert("name".into(), "<small>{values.name}</small>".into());
    let invalid = vec![FieldError {
        field: "name".into(),
        message: "value is required".into(),
    }];
    let html = render_form(
        &route,
        &HashMap::from([(String::from("name"), "<North>".into())]),
        &invalid,
        None,
    );
    let before = html.find("class=\"before\"").unwrap();
    let form = html.find("<form").unwrap();
    let field_help = html.find("<small>&lt;North&gt;</small>").unwrap();
    let after = html.find("class=\"after\"").unwrap();
    let close_form = html.find("</form>").unwrap();
    assert!(before < form);
    assert!(field_help < close_form);
    assert!(after > close_form);
    assert!(html.contains("<p class=\"before\">&lt;North&gt;</p>"));
    assert!(html.contains("aria-invalid=\"true\""));
    assert!(html.contains("<small>&lt;North&gt;</small>"));
}

#[test]
fn renders_tableview_row_actions_for_each_row_and_escapes_dynamic_content() {
    let tableview = TableViewRoute {
        path: "/views/customers".into(),
        title: "Customers".into(),
        layout_html: None,
        source: "SELECT id, name FROM customers".into(),
        columns: vec!["id".into(), "name".into()],
        row_fields: Vec::new(),
        column_content: Vec::new(),
        before_content_html: Some("<p>Before</p>".into()),
        after_content_html: Some("<p>After</p>".into()),
        row_actions: Some(("customer".into(), "<button type=\"button\" data-name=\"{customer.name}\" data-z-dialog-auto-open=\"customer-dialog\">Open</button><dialog id=\"z-dialog-auto-0000000000\" data-z-dialog data-z-dialog-auto=\"z-dialog-auto-0000000000\" aria-labelledby=\"z-dialog-auto-0000000000-title\"><h2 id=\"z-dialog-auto-0000000000-title\">Details for {customer.name}</h2></dialog>".into())),
        policy_row_actions: None,
        filters: Vec::new(),
        searchable: false,
        sortable: false,
        page_size: None,
        requires_auth: false,
        permissions: Vec::new(),
    };
    let html = render_tableview(
        &tableview,
        &[
            vec!["1".into(), "<North>".into()],
            vec!["2".into(), "South".into()],
        ],
        &[vec![false, false], vec![false, false]],
        TableViewRenderState {
            query_values: &HashMap::new(),
            search: "",
            sort: "",
            order: "ASC",
            page: 1,
            per_page: 25,
        },
        UiLanguage::English,
        TableViewPolicyContext {
            program: None,
            functions: &[],
            granted_permissions: Vec::new(),
            edit_permission_granted: true,
            delete_permission_granted: true,
        },
    );
    assert!(html.contains("<p>Before</p>"));
    assert!(html.contains("<button type=\"button\" data-name=\"&lt;North&gt;\" data-z-dialog-auto-open=\"customer-dialog-r0\">Open</button>"));
    assert!(html.contains("data-name=\"South\""));
    assert!(html.contains("data-z-dialog-auto-open=\"customer-dialog-r0\""));
    assert!(html.contains("data-z-dialog-auto-open=\"customer-dialog-r1\""));
    assert!(html.contains("id=\"z-dialog-auto-0000000000-r0\" data-z-dialog data-z-dialog-auto=\"z-dialog-auto-0000000000-r0\""));
    assert!(html.contains("id=\"z-dialog-auto-0000000000-r1\" data-z-dialog data-z-dialog-auto=\"z-dialog-auto-0000000000-r1\""));
    assert!(html.contains("Details for &lt;North&gt;"));
    assert!(html.contains("<th scope=\"col\">Actions</th>"));
    assert!(html.contains("<p>After</p>"));
}

#[test]
fn renders_hidden_tableview_row_fields_as_escaped_column_supporting_content() {
    let tableview = TableViewRoute {
        path: "/views/items".into(),
        title: "Items".into(),
        layout_html: None,
        source: "SELECT sku, customer_number FROM items".into(),
        columns: vec!["sku".into()],
        row_fields: vec!["customer_number".into()],
        column_content: vec![(
            "sku".into(),
            "item".into(),
            "<small>Customer: {item.customer_number}</small><button type=\"button\" data-z-dialog-auto-open=\"z-dialog-auto-0000000000\" aria-controls=\"z-dialog-auto-0000000000\">Details</button><dialog id=\"z-dialog-auto-0000000000\" data-z-dialog data-z-dialog-auto=\"z-dialog-auto-0000000000\" aria-labelledby=\"z-dialog-auto-0000000000-title\"><h2 id=\"z-dialog-auto-0000000000-title\" data-z-dialog-auto-title=\"z-dialog-auto-0000000000\">{item.customer_number}</h2></dialog>".into(),
        )],
        before_content_html: None,
        after_content_html: None,
        row_actions: None,
        policy_row_actions: None,
        filters: Vec::new(),
        searchable: false,
        sortable: false,
        page_size: None,
        requires_auth: false,
        permissions: Vec::new(),
    };
    assert_eq!(
        tableview_selected_columns(&tableview),
        ["sku", "customer_number"]
    );
    let html = render_tableview(
        &tableview,
        &[
            vec!["SKU-7".into(), "<Customer 18>".into()],
            vec!["SKU-8".into(), "Customer 19".into()],
        ],
        &[vec![false, false], vec![false, false]],
        TableViewRenderState {
            query_values: &HashMap::new(),
            search: "",
            sort: "",
            order: "ASC",
            page: 1,
            per_page: 25,
        },
        UiLanguage::English,
        TableViewPolicyContext {
            program: None,
            functions: &[],
            granted_permissions: Vec::new(),
            edit_permission_granted: true,
            delete_permission_granted: true,
        },
    );
    assert!(html.contains("<td>SKU-7<small>Customer: &lt;Customer 18&gt;</small><button"));
    assert!(!html.contains("<td><Customer 18>"));
    assert!(html.contains("data-z-dialog-auto-open=\"z-dialog-auto-0000000000-r0-0\""));
    assert!(html.contains("data-z-dialog-auto-open=\"z-dialog-auto-0000000000-r1-0\""));
    assert!(html.contains("id=\"z-dialog-auto-0000000000-r0-0\""));
    assert!(html.contains("id=\"z-dialog-auto-0000000000-r1-0\""));
    assert!(html.contains("id=\"z-dialog-auto-0000000000-r0-0-title\""));
    assert!(html.contains("aria-labelledby=\"z-dialog-auto-0000000000-r0-0-title\""));
    assert!(html.contains("&gt;</h2>"));
}

#[test]
fn renders_typed_tableview_filters_and_preserves_state() {
    let tableview = TableViewRoute {
        path: "/views/customers".into(),
        title: "Customers".into(),
        layout_html: None,
        source: "SELECT id, orders FROM customer_overview".into(),
        columns: vec!["id".into(), "orders".into()],
        row_fields: Vec::new(),
        column_content: Vec::new(),
        before_content_html: None,
        after_content_html: None,
        row_actions: None,
        policy_row_actions: None,
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
        &[vec![false, false]],
        TableViewRenderState {
            query_values: &query_values,
            search: "",
            sort: "id",
            order: "ASC",
            page: 1,
            per_page: 25,
        },
        UiLanguage::English,
        TableViewPolicyContext {
            program: None,
            functions: &[],
            granted_permissions: Vec::new(),
            edit_permission_granted: true,
            delete_permission_granted: true,
        },
    );
    let html = localize_html(&html, UiLanguage::English);
    assert!(html.contains(
        "<form class=\"z-table-tools\" method=\"get\" action=\"/views/customers\"><fieldset"
    ));
    assert!(html.contains(
        "<fieldset class=\"z-fieldset zelyra-query-controls\"><legend>Search and filters</legend><div class=\"z-query-controls__fields\">"
    ));
    assert!(html.contains("<option value=\"id\" selected>ID</option>"));
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
        file_columns: Vec::new(),
        tenant_column: None,
        recipient_column: None,
        unread_column: Some("read_at".into()),
        expires_column: None,
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
        tenant_scope_column: None,
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
            unread_count: Some(4),
            success: Some("Saved <unsafe>"),
            success_title: Some("Completed"),
        },
    );
    assert!(html.contains(LOCALE_REFERENCE_START));
    let html = localize_html(&html, UiLanguage::English);
    assert!(html.contains("&lt;unsafe&gt;"));
    assert!(html.contains("class=\"zelyra-unread-count\""));
    assert!(!html.contains("&amp;#xe000;"));
    assert!(html.contains(": 4</strong>"));
    assert!(html.contains(
        "<fieldset class=\"z-fieldset zelyra-query-controls\"><legend>Search and filters</legend>"
    ));
    assert!(html.contains(
        "<div class=\"z-field\"><label for=\"search\">Search</label><input class=\"z-input\" id=\"search\""
    ));
    assert!(html.contains("<select class=\"z-select\" id=\"sort\""));
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
            unread_count: None,
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
            unread_count: None,
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
            unread_count: None,
            success: None,
            success_title: None,
        },
    );
    assert!(cards_html.contains("zelyra-crud-cards"));
    assert!(cards_html.contains("zelyra-crud-card"));
    assert!(cards_html.contains(
        "</legend><div class=\"z-query-controls__fields\"><div class=\"z-field\"><label for=\"search\">"
    ));
    assert!(cards_html.contains("</button></div></fieldset></form>"));
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
            unread_count: None,
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
            unread_count: None,
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

    let database_error = crud_error_response(
        &route,
        500,
        "Internal Server Error",
        "The requested data could not be loaded. Check that the database is running and the project schema has been applied.",
    );
    assert!(database_error.body.contains("database is running"));
    assert!(database_error
        .body
        .contains("project schema has been applied"));
    assert_eq!(
        framework_text(
            UiLanguage::German,
            "The requested data could not be loaded. Check that the database is running and the project schema has been applied.",
        )
        .as_deref(),
        Some("Die angeforderten Daten konnten nicht geladen werden. Prüfe, ob die Datenbank läuft und das Projektschema eingerichtet wurde.")
    );
}

#[test]
fn crud_lists_render_file_refs_as_download_links_instead_of_raw_values() {
    let mut route = default_shell_crud(None);
    route.file_columns = vec!["signed_pdf".into()];
    let columns = ["id", "signed_pdf"];
    let rows = [vec![
        "42".into(),
        "f4dcb4cc-87e1-4ed5-9d9f-4d0e16a5035f".into(),
    ]];
    let html = render_crud_list(
        &route,
        CrudListView {
            query_columns: &columns,
            display_columns: &columns,
            filter_columns: &[],
            sort_columns: &columns,
            rows: &rows,
            search: "",
            query_values: &HashMap::new(),
            sort: "id",
            order: "ASC",
            page: 1,
            per_page: 50,
            archived: false,
            unread_count: None,
            success: None,
            success_title: None,
        },
    );
    assert!(html.contains("/machines/42/files/signed_pdf"));
    assert!(html.contains("/files/signed_pdf\">"));
    assert!(!html.contains("f4dcb4cc-87e1-4ed5-9d9f-4d0e16a5035f"));
}

#[test]
fn renders_relationship_labels_in_crud_views() {
    let schema = Schema {
        database: None,
        tables: vec![
            zelyra_database::Table {
                name: "departments".into(),
                tenant_scope_column: None,
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
                tenant_scope_column: None,
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
                    columns: Vec::new(),
                    referenced_table: "departments".into(),
                    referenced_column: "id".into(),
                    referenced_columns: Vec::new(),
                    on_delete: zelyra_ast::ReferentialAction::NoAction,
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
        file_columns: Vec::new(),
        tenant_column: None,
        recipient_column: None,
        unread_column: None,
        expires_column: None,
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
            unread_count: None,
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
        file_columns: Vec::new(),
        tenant_column: None,
        recipient_column: None,
        unread_column: None,
        expires_column: None,
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
        file_columns: Vec::new(),
        tenant_column: None,
        recipient_column: None,
        unread_column: None,
        expires_column: None,
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
fn tenant_access_supports_legacy_membership_rows_and_direct_ownership() {
    let context = TenantContext {
        user_id: 7,
        tenant_id: 41,
        membership_table: Some("tenant_user".into()),
        membership_has_active: false,
        tenant_owner_table: Some("tenants".into()),
        tenant_owner_column: Some("user_id".into()),
    };
    let predicate = tenant_access_predicate(&context, false);
    assert!(predicate.contains("zelyra_membership.user_id = :zelyra_user_id"));
    assert!(!predicate.contains("zelyra_membership.active"));
    assert!(predicate.contains(" OR "));
    assert!(predicate.contains("zelyra_owner.`user_id` = :zelyra_user_id"));

    let queries = tenant_access_queries(&context, false, Some(41), true);
    assert_eq!(queries.len(), 2);
    assert!(queries[0].sql.contains("FROM `tenant_user`"));
    assert!(queries[0]
        .sql
        .contains("tenant_id = :zelyra_selected_tenant_id"));
    assert!(!queries[0].sql.contains("active = true"));
    assert!(queries[0].sql.ends_with("FOR UPDATE"));
    assert!(queries[1].sql.contains("FROM `tenants`"));
    assert!(queries[1].sql.contains("`user_id` = :zelyra_user_id"));
    assert!(queries[1].sql.ends_with("FOR UPDATE"));
}

#[test]
fn tenant_access_still_requires_active_for_membership_schemas_that_declare_it() {
    let context = TenantContext {
        user_id: 7,
        tenant_id: 41,
        membership_table: Some("tenant_memberships".into()),
        membership_has_active: true,
        tenant_owner_table: None,
        tenant_owner_column: None,
    };
    assert!(tenant_access_predicate(&context, false).contains("active = true"));
    assert!(tenant_access_queries(&context, false, None, false)[0]
        .sql
        .contains("active = true"));
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
        login_enabled_column: None,
        membership_table: None,
        tenant_owner_table: None,
        tenant_owner_column: None,
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
        reset_rate_limits_table: None,
        reset_rate_limit: DEFAULT_RESET_RATE_LIMIT,
        reset_block_seconds: DEFAULT_RESET_BLOCK_SECONDS,
        mfa_table: None,
        mfa_recovery_table: None,
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
        file_columns: Vec::new(),
        tenant_column: None,
        recipient_column: None,
        unread_column: None,
        expires_column: None,
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
        file_columns: Vec::new(),
        tenant_column: None,
        recipient_column: None,
        unread_column: None,
        expires_column: None,
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
    let mut route = CrudRoute {
        path: "/machines".into(),
        title: "Machines".into(),
        table: "machines".into(),
        file_columns: Vec::new(),
        tenant_column: None,
        recipient_column: None,
        unread_column: None,
        expires_column: None,
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

    route.unread_column = Some("read_at".into());
    let unread_html = localize_html(
        &render_crud_detail_with_actions(
            &route,
            &["id", "read_at"],
            &["1".into(), "\\N".into()],
            "1",
            CrudUiActions {
                create: false,
                edit: false,
                delete: false,
                restore: false,
                custom: Vec::new(),
            },
            UiLanguage::English,
            &[],
        ),
        UiLanguage::English,
    );
    assert!(unread_html.contains("method=\"post\" action=\"/machines/1/read\""));
    assert!(unread_html.contains(">Mark as read</button>"));
    assert!(unread_html.contains("name=\"_zelyra_csrf\" value=\"crud-csrf\""));
    let read_html = render_crud_detail_with_actions(
        &route,
        &["id", "read_at"],
        &["1".into(), "2026-10-12 09:00:00".into()],
        "1",
        CrudUiActions {
            create: false,
            edit: false,
            delete: false,
            restore: false,
            custom: Vec::new(),
        },
        UiLanguage::English,
        &[],
    );
    assert!(!read_html.contains("/machines/1/read"));

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
        &[],
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
        &[],
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
        capabilities: Vec::new(),
        require_match: false,
        statements: Vec::new(),
        execute_body: None,
        guard: None,
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
fn invalid_form_post_preserves_safe_input_and_redacts_secrets() {
    let mut route = form_route();
    route.form.fields.push(zelyra_ast::FormField {
        name: "email".into(),
        ty: Some(Type::String),
        label: Some("Email".into()),
        placeholder: None,
        required: true,
        min_length: None,
        max: None,
        min_value: None,
        max_value: None,
        choices: Vec::new(),
        widget: Some("email".into()),
        appearance: None,
        file_upload: None,
        readonly: false,
        span: zelyra_ast::Span::default(),
    });
    for name in ["password", "api_token"] {
        route.form.fields.push(zelyra_ast::FormField {
            name: name.into(),
            ty: Some(Type::String),
            label: Some(name.into()),
            placeholder: None,
            required: false,
            min_length: None,
            max: None,
            min_value: None,
            max_value: None,
            choices: Vec::new(),
            widget: (name == "password").then(|| "password".into()),
            appearance: None,
            file_upload: None,
            readonly: false,
            span: zelyra_ast::Span::default(),
        });
    }
    let request = parse_request(
        "POST /forms/CustomerCreate HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\n\r\n_zelyra_csrf=csrf-token&name=%3Cscript%3Ealert%281%29%3C%2Fscript%3E&email=not-an-email&password=never-render-this&api_token=never-render-this-token",
    )
    .unwrap();

    let response = dispatch_form(&route, &request, &HashMap::new(), None, None);

    assert_eq!(response.status, 422);
    assert!(response
        .body
        .contains("value=\"&lt;script&gt;alert(1)&lt;/script&gt;\""));
    assert!(response.body.contains("value=\"not-an-email\""));
    assert!(response.body.contains("id=\"email\""));
    assert!(response.body.contains("aria-invalid=\"true\""));
    assert!(response.body.contains("type=\"password\" value=\"\""));
    assert!(!response.body.contains("never-render-this"));
    assert!(!response.body.contains("never-render-this-token"));
    assert!(!response.body.contains("<script>alert(1)</script>"));
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
        capabilities: Vec::new(),
        require_match: false,
        statements: Vec::new(),
        execute_body: None,
        guard: None,
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
        capabilities: Vec::new(),
        require_match: false,
        statements: Vec::new(),
        execute_body: None,
        guard: None,
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
        capabilities: Vec::new(),
        require_match: false,
        statements: vec![zelyra_ast::Stmt::Expr(zelyra_ast::Expr {
            kind: zelyra_ast::ExprKind::Sql {
                result_type: Type::Unit,
                query: "INSERT INTO customers (name) VALUES (:name)".into(),
            },
            span: zelyra_ast::Span::default(),
        })],
        execute_body: None,
        guard: None,
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

#[test]
fn typed_numeric_filters_reject_invalid_values_and_preserve_integer_limits() {
    for kind in [
        TableViewFilterKind::Decimal,
        TableViewFilterKind::Integer,
        TableViewFilterKind::Unsigned,
        TableViewFilterKind::Numeric,
    ] {
        for invalid in ["NaN", "inf", "-inf", "1e999"] {
            assert!(numeric_filter_query_value(kind, invalid).is_err());
        }
        assert!(tableview_filter_operator_supported(
            kind,
            FilterOperator::LessThan
        ));
    }
    assert!(matches!(
        numeric_filter_query_value(TableViewFilterKind::Unsigned, "18446744073709551615").unwrap(),
        QueryValue::UInt(u64::MAX)
    ));
    assert!(matches!(
        numeric_filter_query_value(TableViewFilterKind::Integer, "-9223372036854775808").unwrap(),
        QueryValue::Int(i64::MIN)
    ));
    assert!(numeric_filter_query_value(TableViewFilterKind::Decimal, "1.0000000001").is_err());
    assert!(page_query_value(&Type::Float, "NaN").is_err());
    assert!(page_query_value(&Type::Decimal, "1e-9").is_err());
}

#[test]
fn decimal_fields_use_text_input_and_optional_blanks_bind_sql_null() {
    let mut form = form_route();
    form.form.fields[0].ty = Some(Type::Option(Box::new(Type::Decimal)));
    form.form.fields[0].required = false;
    let html = render_form(&form, &HashMap::new(), &[], None);
    assert!(html.contains("type=\"text\" inputmode=\"decimal\""));
    for values in [HashMap::new(), HashMap::from([("name".into(), "".into())])] {
        let params = form_query_parameters(&form, &values).unwrap();
        assert!(matches!(params[0].1, QueryValue::Null));
    }
}

#[test]
fn decimal_presentation_retains_digits_without_padding_or_float_conversion() {
    assert_eq!(display_exact_decimal_value("150.000000000"), "150");
    assert_eq!(
        display_exact_decimal_value("9007199254740993.123456789"),
        "9007199254740993.123456789"
    );
    assert_eq!(display_exact_decimal_value("-12.340000000"), "-12.34");
    assert_eq!(display_exact_decimal_value("0.000000001"), "0.000000001");
    assert_eq!(
        display_database_value("150.000000000"),
        "150.000000000",
        "ordinary text stays unchanged"
    );
}

#[test]
fn crud_cards_present_exact_decimals_and_localized_booleans() {
    let schema = Schema {
        database: None,
        tables: vec![zelyra_database::Table {
            name: "items".into(),
            tenant_scope_column: None,
            columns: vec![
                zelyra_database::Column {
                    name: "unit_price".into(),
                    sql_type: "DECIMAL(39,9)".into(),
                    nullable: false,
                    primary_key: false,
                    auto: false,
                    unique: false,
                    default: None,
                },
                zelyra_database::Column {
                    name: "active".into(),
                    sql_type: "BOOLEAN".into(),
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
    };
    assert_eq!(
        display_crud_database_value(
            "125.000000000",
            &schema,
            "items",
            "unit_price",
            UiLanguage::German
        ),
        "125"
    );
    let active_de =
        display_crud_database_value("1", &schema, "items", "active", UiLanguage::German);
    let active_en =
        display_crud_database_value("0", &schema, "items", "active", UiLanguage::English);
    assert!(active_de.contains("identifier.yes"));
    assert!(active_en.contains("identifier.no"));
    assert!(
        display_crud_database_value("t", &schema, "items", "active", UiLanguage::English)
            .contains("identifier.yes")
    );
    assert!(
        display_crud_database_value("f", &schema, "items", "active", UiLanguage::German)
            .contains("identifier.no")
    );
}

#[test]
fn edit_snapshots_distinguish_native_null_literal_marker_and_empty_text() {
    let csrf = CsrfProtection::new("snapshot-key");
    let literal = EditValues {
        values: HashMap::from([("note".into(), "\\N".into())]),
        nulls: HashSet::new(),
    };
    let null = EditValues {
        values: literal.values.clone(),
        nulls: HashSet::from(["note".into()]),
    };
    let empty = EditValues {
        values: HashMap::from([("note".into(), "".into())]),
        nulls: HashSet::new(),
    };
    let token = sign_form_snapshot(&csrf, &null).unwrap();
    let signature = verify_form_snapshot(&token).unwrap();
    assert!(form_snapshot_matches(&csrf, &signature, &null));
    assert!(!form_snapshot_matches(&csrf, &signature, &literal));
    assert!(!form_snapshot_matches(&csrf, &signature, &empty));
    assert_eq!(null.rendered_values()["note"], "");
    assert_eq!(literal.rendered_values()["note"], "\\N");
    let legacy = serde_json::to_vec(&literal.values.iter().collect::<BTreeMap<_, _>>()).unwrap();
    let key = hmac::Key::new(hmac::HMAC_SHA256, csrf.token.as_bytes());
    assert_eq!(
        sign_form_snapshot(&csrf, &literal).unwrap(),
        hex_encode(hmac::sign(&key, &legacy).as_ref())
    );
}

#[test]
fn edit_snapshots_fail_closed_on_missing_or_invalid_native_metadata() {
    let form = form_route();
    let mut result = zelyra_database::QueryResult {
        columns: vec!["name".into()],
        rows: vec![vec!["\\N".into()]],
        ..Default::default()
    };
    assert!(EditValues::from_result(&form, &result).is_err());
    result.nulls = vec![vec![false]];
    assert_eq!(
        EditValues::from_result(&form, &result)
            .unwrap()
            .unwrap()
            .rendered_values()["name"],
        "\\N"
    );
    result.nulls = vec![vec![true]];
    assert_eq!(
        EditValues::from_result(&form, &result)
            .unwrap()
            .unwrap()
            .rendered_values()["name"],
        ""
    );
    result.nulls[0].clear();
    assert!(EditValues::from_result(&form, &result).is_err());
    let invalid = EditValues {
        values: HashMap::new(),
        nulls: HashSet::from(["missing".into()]),
    };
    assert!(sign_form_snapshot(&CsrfProtection::new("key"), &invalid).is_none());
}

#[test]
fn nullable_storage_binds_omitted_text_as_null_without_weakening_required_fields() {
    let mut form = form_route();
    form.form.table = Some("notes".into());
    form.form.fields[0].required = false;
    form.schema = Some(Schema {
        database: None,
        tables: vec![zelyra_database::Table {
            name: "notes".into(),
            tenant_scope_column: None,
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
        }],
    });
    let empty = HashMap::new();
    assert!(validate(&form.form, None, form.schema.as_ref(), &empty)
        .errors
        .is_empty());
    let params = form_query_parameters(&form, &empty).unwrap();
    assert!(matches!(&params[0], (name, QueryValue::Null) if name == "name"));
    let literal = HashMap::from([("name".into(), "\\N".into())]);
    let params = form_query_parameters(&form, &literal).unwrap();
    assert!(
        matches!(&params[0], (name, QueryValue::String(value)) if name == "name" && value == "\\N")
    );
    form.form.fields[0].required = true;
    assert_eq!(
        validate(&form.form, None, form.schema.as_ref(), &empty)
            .errors
            .len(),
        1
    );
}

#[test]
fn postgres_audit_insert_rejects_chained_and_accepts_unchained() {
    let unchained = audit_insert_queries_for(
        "audit_log",
        false,
        Some(42),
        "user.created",
        Some(10),
        "details",
        true,
    )
    .unwrap();
    assert_eq!(unchained.len(), 1);
    assert!(unchained[0].sql.starts_with("INSERT INTO \"audit_log\""));

    let chained = audit_insert_queries_for(
        "audit_log",
        true,
        Some(42),
        "user.created",
        Some(10),
        "details",
        true,
    );
    assert!(chained.is_err());
    assert!(chained
        .unwrap_err()
        .contains("PostgreSQL does not support chained audit tables"));
}

#[test]
fn execute_form_action_rejects_postgres_chained_audit() {
    let mut form = form_route();
    form.audit_table = Some("audit_log".into());
    form.audit_chain = true;
    let action = zelyra_ast::FormAction {
        name: "save".into(),
        label: None,
        icon: None,
        confirm: None,
        confirm_page: None,
        success_page: None,
        error_page: None,
        statements: vec![zelyra_ast::Stmt::Expr(zelyra_ast::Expr {
            kind: zelyra_ast::ExprKind::Sql {
                result_type: Type::Unit,
                query: "UPDATE customers SET name = :name WHERE id = :id".into(),
            },
            span: zelyra_ast::Span::default(),
        })],
        execute_body: None,
        guard: None,
        fields: Vec::new(),
        requires_auth: false,
        permissions: Vec::new(),
        capabilities: Vec::new(),
        require_match: false,
        success: None,
        redirect: None,
        span: zelyra_ast::Span::default(),
    };
    let values = HashMap::from([("name".into(), "Alice".into())]);
    let path_params = HashMap::from([("id".into(), "1".into())]);
    let context = FormExecutionContext {
        actor_user_id: Some(1),
        expected_snapshot: None,
        tenant_context: None,
        validation_program: None,
        table_validation_functions: &[],
        crud_policy_functions: &[],
        crud_action_functions: &[],
        capability_grants: None,
        runtime_policy: None,
        granted_permissions: &[],
    };
    let err = execute_form_action(
        &form,
        &action,
        &values,
        &path_params,
        "postgres://user:pass@localhost:5432/db",
        context,
    );
    assert!(err.is_err());
    assert!(err
        .unwrap_err()
        .contains("PostgreSQL does not support chained audit tables"));
}

#[test]
fn crud_row_policy_evaluation_distinguishes_guard_and_conditional_permission() {
    let source = r#"
        table invoices {
            id: Int primary auto
            status: String(20) required
            locked: Bool required
        }
        crud Invoice -> invoices {
            permits edit "invoices.manage" when status == "draft"
            guard edit when locked == true { error: "Locked invoices cannot be edited." }
            guard delete when locked == true { error: "Locked invoices cannot be deleted." }
        }
    "#;
    let program = zelyra_parser::parse(&zelyra_lexer::lex(source).unwrap()).unwrap();
    let (functions, mappings) = zelyra_ast::build_crud_policy_functions(&program);
    let mut executable = program.clone();
    executable.functions.extend(functions);
    let query_result = |status: &str, locked: bool| zelyra_database::QueryResult {
        columns: vec!["id".into(), "status".into(), "locked".into()],
        rows: vec![vec!["7".into(), status.into(), locked.to_string()]],
        nulls: vec![vec![false; 3]],
        column_types: Vec::new(),
    };

    let allowed = evaluate_crud_operation_policies(
        &executable,
        &mappings,
        0,
        zelyra_ast::CrudRecordAction::Edit,
        &query_result("draft", false),
    )
    .unwrap();
    assert!(check_crud_policy_result(allowed, &["invoices.manage".into()]).is_ok());

    let denied_permission = evaluate_crud_operation_policies(
        &executable,
        &mappings,
        0,
        zelyra_ast::CrudRecordAction::Edit,
        &query_result("draft", false),
    )
    .unwrap();
    assert!(check_crud_policy_result(denied_permission, &[])
        .unwrap_err()
        .contains("invoices.manage"));

    let guarded = evaluate_crud_operation_policies(
        &executable,
        &mappings,
        0,
        zelyra_ast::CrudRecordAction::Edit,
        &query_result("draft", true),
    )
    .unwrap();
    assert!(
        check_crud_policy_result(guarded, &["invoices.manage".into()])
            .unwrap_err()
            .contains("Locked invoices cannot be edited.")
    );

    let tableview = TableViewRoute {
        path: "/views/invoice-rows".into(),
        title: "Invoice rows".into(),
        layout_html: None,
        source: "SELECT id, status, locked FROM invoices".into(),
        columns: vec!["status".into()],
        row_fields: vec!["id".into(), "locked".into()],
        column_content: Vec::new(),
        before_content_html: None,
        after_content_html: None,
        row_actions: None,
        policy_row_actions: Some(TableViewPolicyRowActions {
            crud_index: 0,
            item: "invoice".into(),
            edit_html: Some("<a href=\"/invoices/{invoice.id}/edit\">Edit</a>".into()),
            delete_html: Some("<button type=\"button\">Delete</button>".into()),
            requires_auth: false,
            edit_permissions: Vec::new(),
            delete_permissions: Vec::new(),
        }),
        filters: Vec::new(),
        searchable: false,
        sortable: false,
        page_size: Some(10),
        requires_auth: false,
        permissions: Vec::new(),
    };
    let render_policy_row = |status: &str, locked: bool| {
        render_tableview(
            &tableview,
            &[vec![status.into(), "7".into(), locked.to_string()]],
            &[vec![false; 3]],
            TableViewRenderState {
                query_values: &HashMap::new(),
                search: "",
                sort: "status",
                order: "ASC",
                page: 1,
                per_page: 10,
            },
            UiLanguage::English,
            TableViewPolicyContext {
                program: Some(&executable),
                functions: &mappings,
                granted_permissions: vec!["invoices.manage".into()],
                edit_permission_granted: true,
                delete_permission_granted: true,
            },
        )
    };
    let editable_html = render_policy_row("draft", false);
    assert!(editable_html.contains("/invoices/7/edit"));
    assert!(editable_html.contains(">Delete</button>"));
    let non_draft_html = render_policy_row("sent", false);
    assert!(!non_draft_html.contains("/invoices/7/edit"));
    assert!(non_draft_html.contains(">Delete</button>"));
    let locked_html = render_policy_row("draft", true);
    assert!(!locked_html.contains("/invoices/7/edit"));
    assert!(!locked_html.contains(">Delete</button>"));
}
