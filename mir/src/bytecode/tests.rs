//! Differential and verifier tests for the bytecode backend.

use super::*;
use crate::execution;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use zelyra_lexer::lex;
use zelyra_parser::parse;

struct FixedHost {
    timestamp: i64,
    environment: Option<String>,
    calls: AtomicUsize,
}

struct MailHost {
    delivery: Mutex<Option<(String, String, String)>>,
    calls: AtomicUsize,
}

struct SqlHost {
    query: Mutex<Option<zelyra_database::Query>>,
    validations: AtomicUsize,
    result: zelyra_database::QueryResult,
}

struct SqlBatchHost {
    batch: Mutex<Option<(Vec<zelyra_database::Query>, bool)>>,
    validations: AtomicUsize,
}

struct OutboxHost {
    batch: Mutex<Option<(Vec<zelyra_database::Query>, bool)>>,
    validations: AtomicUsize,
    random_calls: AtomicUsize,
}

impl HostServices for SqlBatchHost {
    fn unix_time_millis(&self) -> Result<i64, String> {
        Ok(0)
    }
    fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
        Ok(None)
    }
    fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
        output.fill(0);
        Ok(())
    }
    fn validate_database_storage(
        &self,
        _url: &str,
        _schema: &zelyra_database::Schema,
    ) -> Result<(), String> {
        self.validations.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn execute_database_batch(
        &self,
        _url: &str,
        queries: &[zelyra_database::Query],
        transaction: bool,
    ) -> Result<Vec<zelyra_database::QueryResult>, String> {
        *self.batch.lock().unwrap() = Some((queries.to_vec(), transaction));
        Ok(vec![zelyra_database::QueryResult::default(); queries.len()])
    }
}

impl HostServices for OutboxHost {
    fn unix_time_millis(&self) -> Result<i64, String> {
        Ok(0)
    }
    fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
        Ok(None)
    }
    fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
        self.random_calls.fetch_add(1, Ordering::SeqCst);
        output.fill(0xab);
        Ok(())
    }
    fn validate_database_storage(
        &self,
        _url: &str,
        _schema: &zelyra_database::Schema,
    ) -> Result<(), String> {
        self.validations.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn execute_database_batch(
        &self,
        _url: &str,
        queries: &[zelyra_database::Query],
        transaction: bool,
    ) -> Result<Vec<zelyra_database::QueryResult>, String> {
        *self.batch.lock().unwrap() = Some((queries.to_vec(), transaction));
        Ok(vec![zelyra_database::QueryResult::default(); queries.len()])
    }
}

impl HostServices for SqlHost {
    fn unix_time_millis(&self) -> Result<i64, String> {
        Ok(0)
    }
    fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
        Ok(None)
    }
    fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
        output.fill(0);
        Ok(())
    }

    fn validate_database_storage(
        &self,
        _database_url: &str,
        _schema: &zelyra_database::Schema,
    ) -> Result<(), String> {
        self.validations.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn execute_database_query(
        &self,
        _database_url: &str,
        query: &zelyra_database::Query,
    ) -> Result<zelyra_database::QueryResult, String> {
        *self.query.lock().unwrap() = Some(query.clone());
        Ok(self.result.clone())
    }
}

impl HostServices for MailHost {
    fn unix_time_millis(&self) -> Result<i64, String> {
        Ok(1_700_000_000_000)
    }

    fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
        Ok(None)
    }

    fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
        output.fill(0);
        Ok(())
    }

    fn deliver_outbox_message(
        &self,
        event_id: &str,
        payload_json: &str,
        mail: &MailPolicy,
        network: &NetworkPolicy,
    ) -> Result<(), String> {
        let host = mail
            .host
            .as_deref()
            .ok_or_else(|| "missing test SMTP host".to_owned())?;
        if !network.allowed_hosts.iter().any(|allowed| allowed == host) {
            return Err("test SMTP host is not allowed".into());
        }
        self.calls.fetch_add(1, Ordering::SeqCst);
        *self.delivery.lock().unwrap() = Some((
            event_id.to_owned(),
            payload_json.to_owned(),
            host.to_owned(),
        ));
        Ok(())
    }
}

impl HostServices for FixedHost {
    fn unix_time_millis(&self) -> Result<i64, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.timestamp)
    }

    fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.environment.clone())
    }

    fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        output.fill(0);
        Ok(())
    }
}

struct ConsoleHost {
    line: Option<String>,
    prompt: std::sync::Mutex<Option<String>>,
    calls: AtomicUsize,
}

impl HostServices for ConsoleHost {
    fn unix_time_millis(&self) -> Result<i64, String> {
        Ok(0)
    }

    fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
        Ok(None)
    }

    fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
        output.fill(0);
        Ok(())
    }

    fn read_console_line(&self, prompt: &str) -> Result<Option<String>, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        *self.prompt.lock().unwrap() = Some(prompt.to_owned());
        Ok(self.line.clone())
    }
}

fn compile_source(source: &str, entry: &str) -> (MirProgram, BytecodeProgram) {
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source type-checks and lowers");
    let bytecode = compile(&mir, entry).expect("entry lowers to bytecode");
    (mir, bytecode)
}

#[test]
fn tenant_scope_metadata_roundtrips_and_bytecode_verifier_blocks_raw_sql() {
    let source = parse(
        &lex("type TenantId = Id table invoices scoped by tenant_id: TenantId { id: Id primary auto tenant_id: TenantId required } fn main() {}").unwrap(),
    )
    .unwrap();
    let mir = crate::lower(&source).expect("scoped table metadata lowers");
    let bytecode = compile(&mir, "main").expect("program without SQL compiles");
    assert_eq!(bytecode.scoped_tables, ["invoices"]);
    assert_eq!(bytecode.version, BYTECODE_VERSION);
    let decoded = decode(&encode(&bytecode).expect("scoped metadata encodes"))
        .expect("scoped metadata decodes");
    assert_eq!(decoded.scoped_tables, ["invoices"]);

    let checked_schema_source = parse(
        &lex("type TenantId = Id table invoices scoped by tenant_id: TenantId { id: Id primary auto tenant_id: TenantId required } fn main() {}").unwrap(),
    )
    .unwrap();
    let checked_schema = zelyra_database::build_schema(&checked_schema_source).unwrap();

    let unscoped_source = parse(
        &lex("table invoices { id: Id primary auto } fn main() uses Database(read) { rows = sql<Int[]> { SELECT id FROM invoices } }").unwrap(),
    )
    .unwrap();
    let unscoped_mir = crate::lower(&unscoped_source).expect("unscoped SQL lowers");
    let mut forged = compile(&unscoped_mir, "main").expect("unscoped SQL compiles");
    forged.scoped_tables.push("invoices".into());
    let errors = verify_bytecode(&forged).expect_err("scoped raw SQL must be rejected");
    assert!(errors
        .iter()
        .any(|error| { error.message.contains("tenant-scoped table `invoices`") }));

    let mut scope_stripped = compile(&unscoped_mir, "main").unwrap();
    scope_stripped.scoped_tables.clear();
    let host = Arc::new(SqlHost {
        query: Mutex::new(None),
        validations: AtomicUsize::new(0),
        result: zelyra_database::QueryResult::default(),
    });
    let grants = HashSet::from(["Database(read)".to_owned()]);
    let result = execute_with_database_and_host_services_and_policy_captured(
        &scope_stripped,
        Vec::new(),
        ExecutionLimits::default(),
        Some(&grants),
        None,
        BytecodeDatabaseExecution {
            url: "memory://db",
            schema: &Ok(checked_schema),
        },
        host.clone(),
    );
    assert!(result.error.is_some_and(|error| error
        .message
        .contains("does not match the checked project schema")));
    assert_eq!(host.validations.load(Ordering::SeqCst), 0);
    assert!(host.query.lock().unwrap().is_none());
}

#[test]
fn bytecode_sql_query_uses_explicit_database_context_and_shared_host_checks() {
    let source = r#"
            database main { engine: mariadb database: "bytecode_test" }
            fn lookup(expected: Int) -> Int[] uses Database(read) {
                return sql<Int[]> { SELECT :expected AS value }
            }
            fn main() {}
        "#;
    let (mir, bytecode) = compile_source(source, "lookup");
    let encoded = encode(&bytecode).expect("SQL bytecode encodes");
    let bytecode = decode(&encoded).expect("SQL bytecode decodes");
    let host = Arc::new(SqlHost {
        query: Mutex::new(None),
        validations: AtomicUsize::new(0),
        result: zelyra_database::QueryResult {
            columns: vec!["value".into()],
            rows: vec![vec!["42".into()]],
            nulls: vec![vec![false]],
            column_types: vec![zelyra_database::QueryColumnType::Integer],
        },
    });
    let grants = HashSet::from(["Database(read)".to_owned()]);
    let database = BytecodeDatabaseExecution {
        url: "mariadb://user:secret@127.0.0.1:3306/bytecode_test",
        schema: &mir.database_schema,
    };
    let actual = execute_with_database_and_host_services_and_policy_captured(
        &bytecode,
        vec![Value::Int(42)],
        ExecutionLimits::default(),
        Some(&grants),
        None,
        database,
        host.clone(),
    );
    assert_eq!(actual.error, None);
    assert!(
        matches!(actual.value, Some(Value::Rows { ref rows, .. }) if rows == &vec![vec![String::from("42")]])
    );
    assert_eq!(host.validations.load(Ordering::SeqCst), 1);
    let query = host.query.lock().unwrap().clone().expect("host query");
    assert_eq!(query.sql, " SELECT :expected AS value ");
    assert_eq!(query.params.len(), 1);
    assert_eq!(query.params[0].0, "expected");
    assert!(matches!(
        query.params[0].1,
        zelyra_database::QueryValue::Int(42)
    ));

    let no_database_host = Arc::new(SqlHost {
        query: Mutex::new(None),
        validations: AtomicUsize::new(0),
        result: zelyra_database::QueryResult::default(),
    });
    let no_database = execute_with_host_services(
        &bytecode,
        vec![Value::Int(42)],
        ExecutionLimits::default(),
        Some(&grants),
        no_database_host.clone(),
    )
    .unwrap_err();
    assert!(no_database.message.contains("requires a database runtime"));
    assert_eq!(no_database_host.validations.load(Ordering::SeqCst), 0);

    let mut tampered = bytecode.clone();
    let sql = tampered.functions[0]
        .code
        .iter_mut()
        .find_map(|instruction| match &mut instruction.operation {
            BytecodeOp::Sql { query, .. } => Some(query),
            _ => None,
        })
        .expect("compiled SQL op");
    *sql = "SELECT * FROM missing_table".into();
    let tamper_host = Arc::new(SqlHost {
        query: Mutex::new(None),
        validations: AtomicUsize::new(0),
        result: zelyra_database::QueryResult::default(),
    });
    let tampered_result = execute_with_database_and_host_services_and_policy_captured(
        &tampered,
        vec![Value::Int(42)],
        ExecutionLimits::default(),
        Some(&grants),
        None,
        database,
        tamper_host.clone(),
    );
    let tampered_error = tampered_result.error.unwrap();
    assert!(
        tampered_error
            .message
            .contains("does not exist in the Zelyra schema"),
        "{tampered_error:?}"
    );
    assert_eq!(tamper_host.validations.load(Ordering::SeqCst), 0);
    assert!(tamper_host.query.lock().unwrap().is_none());

    let denied_host = Arc::new(SqlHost {
        query: Mutex::new(None),
        validations: AtomicUsize::new(0),
        result: zelyra_database::QueryResult::default(),
    });
    let denied = execute_with_database_and_host_services_and_policy_captured(
        &bytecode,
        vec![Value::Int(42)],
        ExecutionLimits::default(),
        None,
        None,
        database,
        denied_host.clone(),
    );
    assert!(denied.error.unwrap().message.contains("Database(read)"));
    assert_eq!(denied_host.validations.load(Ordering::SeqCst), 0);
    assert!(denied_host.query.lock().unwrap().is_none());
}

#[test]
fn bytecode_sql_transaction_is_encoded_and_runs_as_one_checked_batch() {
    let source = r#"
            database main { engine: mariadb database: "bytecode_test" }
            fn update(expected: Int) uses Database(read) {
                transaction {
                    sql<Int[]> { SELECT :expected AS value }
                    sql<Int[]> { SELECT :expected AS value }
                }
            }
            fn main() uses Database(read) { update(12) }
        "#;
    let (mir, bytecode) = compile_source(source, "update");
    let bytes = encode(&bytecode).expect("transaction encodes");
    let bytecode = decode(&bytes).expect("transaction decodes");
    let host = Arc::new(SqlBatchHost {
        batch: Mutex::new(None),
        validations: AtomicUsize::new(0),
    });
    let grants = HashSet::from(["Database(read)".to_owned()]);
    let database = BytecodeDatabaseExecution {
        url: "mariadb://user:secret@127.0.0.1:3306/bytecode_test",
        schema: &mir.database_schema,
    };
    let outcome = execute_with_database_and_host_services_and_policy_captured(
        &bytecode,
        vec![Value::Int(12)],
        ExecutionLimits::default(),
        Some(&grants),
        None,
        database,
        host.clone(),
    );
    assert_eq!(outcome.error, None);
    assert_eq!(host.validations.load(Ordering::SeqCst), 1);
    let (queries, transactional) = host.batch.lock().unwrap().clone().expect("batch call");
    assert!(transactional);
    assert_eq!(queries.len(), 2);
    assert!(queries.iter().all(|query| {
        query.params.len() == 1
            && query.params[0].0 == "expected"
            && matches!(query.params[0].1, zelyra_database::QueryValue::Int(12))
    }));

    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let ast_host = Arc::new(SqlBatchHost {
        batch: Mutex::new(None),
        validations: AtomicUsize::new(0),
    });
    let ast_outcome = zelyra_runtime::execute_captured_with_database_and_host_services(
        &ast,
        database.url,
        ast_host.clone(),
        Some(&grants),
        None,
    );
    assert_eq!(ast_outcome.error, None);
    let (bytecode_queries, bytecode_transaction) =
        host.batch.lock().unwrap().clone().expect("bytecode batch");
    let (ast_queries, ast_transaction) = ast_host.batch.lock().unwrap().clone().expect("AST batch");
    assert_eq!(bytecode_transaction, ast_transaction);
    assert_eq!(bytecode_queries.len(), ast_queries.len());
    for (bytecode_query, ast_query) in bytecode_queries.iter().zip(ast_queries.iter()) {
        assert_eq!(bytecode_query.sql, ast_query.sql);
        assert_eq!(bytecode_query.params.len(), ast_query.params.len());
        for ((bytecode_name, bytecode_value), (ast_name, ast_value)) in
            bytecode_query.params.iter().zip(ast_query.params.iter())
        {
            assert_eq!(bytecode_name, ast_name);
            assert!(matches!(
                (bytecode_value, ast_value),
                (
                    zelyra_database::QueryValue::Int(12),
                    zelyra_database::QueryValue::Int(12)
                )
            ));
        }
    }

    let budget_host = Arc::new(SqlBatchHost {
        batch: Mutex::new(None),
        validations: AtomicUsize::new(0),
    });
    let budget_error = execute_with_database_and_host_services_and_policy_captured(
        &bytecode,
        vec![Value::Int(12)],
        ExecutionLimits {
            max_steps: 2,
            max_stack_depth: 20,
        },
        Some(&grants),
        None,
        database,
        budget_host.clone(),
    )
    .error
    .expect("transaction statements consume the shared step budget");
    assert!(budget_error.message.contains("step budget exceeded"));
    assert!(budget_host.batch.lock().unwrap().is_none());
    assert_eq!(budget_host.validations.load(Ordering::SeqCst), 0);

    let denied_host = Arc::new(SqlBatchHost {
        batch: Mutex::new(None),
        validations: AtomicUsize::new(0),
    });
    let denied = execute_with_database_and_host_services_and_policy_captured(
        &bytecode,
        vec![Value::Int(12)],
        ExecutionLimits::default(),
        None,
        None,
        database,
        denied_host.clone(),
    );
    assert!(denied.error.unwrap().message.contains("Database(read)"));
    assert!(denied_host.batch.lock().unwrap().is_none());
    assert_eq!(denied_host.validations.load(Ordering::SeqCst), 0);
}

#[test]
fn bytecode_transactional_mail_queue_uses_outbox_and_preflights_before_host_calls() {
    let source = r#"
            database main { engine: mariadb database: "bytecode_test" }
            table logs { id: Id primary auto message: String(100) required }
            fn queue(message: String) uses Email, Database(write) {
                transaction {
                    sql { INSERT INTO logs (message) VALUES (:message) }
                    mail_queue("customer@example.test", "Invoice ready", message)
                }
            }
            fn main() uses Email, Database(write) { queue("Your invoice is ready.") }
        "#;
    let (mir, bytecode) = compile_source(source, "main");
    let bytecode = decode(&encode(&bytecode).expect("outbox bytecode encodes"))
        .expect("outbox bytecode decodes");
    let grants = HashSet::from(["Email".to_owned(), "Database(write)".to_owned()]);
    let database = BytecodeDatabaseExecution {
        url: "mariadb://user:secret@127.0.0.1:3306/bytecode_test",
        schema: &mir.database_schema,
    };
    let host = Arc::new(OutboxHost {
        batch: Mutex::new(None),
        validations: AtomicUsize::new(0),
        random_calls: AtomicUsize::new(0),
    });
    let outcome = execute_with_database_and_host_services_and_policy_captured(
        &bytecode,
        vec![],
        ExecutionLimits::default(),
        Some(&grants),
        None,
        database,
        host.clone(),
    );
    assert_eq!(outcome.error, None);
    assert_eq!(host.validations.load(Ordering::SeqCst), 1);
    assert_eq!(host.random_calls.load(Ordering::SeqCst), 1);
    let (queries, transaction) = host.batch.lock().unwrap().clone().expect("outbox batch");
    assert!(transaction);
    assert_eq!(queries.len(), 2);
    assert!(queries[0].sql.contains("INSERT INTO logs"));
    assert_eq!(queries[0].params.len(), 1);
    assert!(matches!(
        &queries[0].params[0].1,
        zelyra_database::QueryValue::String(message) if message == "Your invoice is ready."
    ));
    assert!(queries[1].sql.contains("INSERT INTO `_zelyra_outbox`"));
    assert_eq!(queries[1].params.len(), 3);
    assert_eq!(queries[1].params[0].0, "event_id");
    assert!(matches!(
        &queries[1].params[0].1,
        zelyra_database::QueryValue::String(id) if id == &"ab".repeat(16)
    ));
    assert_eq!(queries[1].params[1].0, "event_type");
    assert!(matches!(
        &queries[1].params[1].1,
        zelyra_database::QueryValue::String(kind) if kind == "zelyra.mail.send"
    ));
    assert!(matches!(
        &queries[1].params[2].1,
        zelyra_database::QueryValue::String(payload)
            if payload.contains("customer@example.test") && payload.contains("Your invoice is ready.")
    ));

    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let ast_host = Arc::new(OutboxHost {
        batch: Mutex::new(None),
        validations: AtomicUsize::new(0),
        random_calls: AtomicUsize::new(0),
    });
    let ast_outcome = zelyra_runtime::execute_captured_with_database_and_host_services(
        &ast,
        database.url,
        ast_host.clone(),
        Some(&grants),
        None,
    );
    assert_eq!(ast_outcome.error, None);
    let (ast_queries, ast_transaction) = ast_host
        .batch
        .lock()
        .unwrap()
        .clone()
        .expect("AST outbox batch");
    assert!(ast_transaction);
    assert_eq!(ast_queries.len(), queries.len());
    for (ast_query, bytecode_query) in ast_queries.iter().zip(queries.iter()) {
        assert_eq!(ast_query.sql, bytecode_query.sql);
        assert_eq!(ast_query.params.len(), bytecode_query.params.len());
        for (ast_parameter, bytecode_parameter) in
            ast_query.params.iter().zip(bytecode_query.params.iter())
        {
            assert_eq!(ast_parameter.0, bytecode_parameter.0);
            match (&ast_parameter.1, &bytecode_parameter.1) {
                (
                    zelyra_database::QueryValue::String(ast_value),
                    zelyra_database::QueryValue::String(bytecode_value),
                ) => assert_eq!(ast_value, bytecode_value),
                (ast_value, bytecode_value) => {
                    assert!(matches!(
                        (ast_value, bytecode_value),
                        (zelyra_database::QueryValue::Int(left), zelyra_database::QueryValue::Int(right)) if left == right
                    ));
                }
            }
        }
    }

    let denied_host = Arc::new(OutboxHost {
        batch: Mutex::new(None),
        validations: AtomicUsize::new(0),
        random_calls: AtomicUsize::new(0),
    });
    let denied = execute_with_database_and_host_services_and_policy_captured(
        &bytecode,
        vec![],
        ExecutionLimits::default(),
        Some(&HashSet::from(["Database(write)".to_owned()])),
        None,
        database,
        denied_host.clone(),
    );
    assert!(denied.error.unwrap().message.contains("Email"));
    assert!(denied_host.batch.lock().unwrap().is_none());
    assert_eq!(denied_host.random_calls.load(Ordering::SeqCst), 0);
    assert_eq!(denied_host.validations.load(Ordering::SeqCst), 0);
}

#[test]
fn bytecode_pdf_attachment_survives_encoding_and_uses_the_shared_renderer() {
    let source = r#"
            database main { engine: mariadb database: "bytecode_test" }
            struct Invoice { number: String }
            view InvoicePrint { html { <p>{number}</p> } }
            fn queue(invoice: Invoice) uses Email, Database(write) {
                transaction {
                    mail_queue("customer@example.test", "Invoice", "Attached", "invoice.pdf", render_pdf("InvoicePrint", invoice))
                }
            }
            fn main() uses Email, Database(write) {
                queue(Invoice { number: "INV-1" })
            }
        "#;
    let (mir, bytecode) = compile_source(source, "main");
    let bytecode =
        decode(&encode(&bytecode).expect("PDF bytecode encodes")).expect("PDF bytecode decodes");
    let mut duplicate_views = bytecode.clone();
    duplicate_views.views.push(duplicate_views.views[0].clone());
    assert!(verify_bytecode(&duplicate_views).is_err());
    let host = Arc::new(OutboxHost {
        batch: Mutex::new(None),
        validations: AtomicUsize::new(0),
        random_calls: AtomicUsize::new(0),
    });
    let database = BytecodeDatabaseExecution {
        url: "mariadb://user:secret@127.0.0.1:3306/bytecode_test",
        schema: &mir.database_schema,
    };
    let grants = HashSet::from(["Email".to_owned(), "Database(write)".to_owned()]);
    let outcome = execute_with_database_and_host_services_and_policy_captured(
        &bytecode,
        vec![],
        ExecutionLimits::default(),
        Some(&grants),
        None,
        database,
        host.clone(),
    );
    assert_eq!(outcome.error, None);
    let (queries, transactional) = host.batch.lock().unwrap().clone().expect("outbox batch");
    assert!(transactional);
    assert_eq!(queries.len(), 1);
    let payload = match &queries[0].params[2].1 {
        zelyra_database::QueryValue::String(payload) => payload,
        other => panic!("expected JSON outbox payload, got {other:?}"),
    };
    assert!(payload.contains("application/pdf"));
    assert!(payload.contains("invoice.pdf"));
    assert!(
        payload.contains("JVBER"),
        "the attachment contains encoded PDF bytes"
    );

    let denied_host = Arc::new(OutboxHost {
        batch: Mutex::new(None),
        validations: AtomicUsize::new(0),
        random_calls: AtomicUsize::new(0),
    });
    let denied = execute_with_database_and_host_services_and_policy_captured(
        &bytecode,
        vec![],
        ExecutionLimits::default(),
        Some(&HashSet::from(["Database(write)".to_owned()])),
        None,
        database,
        denied_host.clone(),
    );
    assert!(denied.error.unwrap().message.contains("Email"));
    assert!(denied_host.batch.lock().unwrap().is_none());
    assert_eq!(denied_host.random_calls.load(Ordering::SeqCst), 0);
    assert_eq!(denied_host.validations.load(Ordering::SeqCst), 0);
}

#[test]
fn bytecode_mail_delivery_uses_shared_mail_policy_and_email_preflight() {
    let source = "fn deliver(id: String, payload: String) -> Result<Unit, String> uses Email { return mail_deliver(id, payload) } fn main() {}";
    let (_, bytecode) = compile_source(source, "deliver");
    let grants = HashSet::from(["Email".to_owned()]);
    let mut mail = MailPolicy::default();
    mail.host = Some("smtp.example.test".into());
    mail.from = Some("billing@example.test".into());
    let policy = RuntimePolicy {
        network: Some(NetworkPolicy {
            allowed_hosts: vec!["smtp.example.test".into()],
            timeout_ms: 1000,
            max_response_bytes: 1024,
        }),
        mail: Some(mail),
        ..RuntimePolicy::default()
    };
    let host = Arc::new(MailHost {
        delivery: Mutex::new(None),
        calls: AtomicUsize::new(0),
    });
    let result = execute_with_host_services_and_policy_captured(
        &decode(&encode(&bytecode).unwrap()).unwrap(),
        vec![
            Value::String("event-01".into()),
            Value::String(r#"{"to":"a@example.test","subject":"Hi","text":"Body"}"#.into()),
        ],
        ExecutionLimits::default(),
        Some(&grants),
        Some(&policy),
        host.clone(),
    );
    let bytecode_value = result.value.expect("bytecode returns a Result value");
    assert_eq!(bytecode_value, Value::Result(Ok(Box::new(Value::Unit))));
    assert_eq!(host.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        *host.delivery.lock().unwrap(),
        Some((
            "event-01".into(),
            r#"{"to":"a@example.test","subject":"Hi","text":"Body"}"#.into(),
            "smtp.example.test".into(),
        ))
    );

    let ast = parse(&lex(source).unwrap()).unwrap();
    let ast_host = Arc::new(MailHost {
        delivery: Mutex::new(None),
        calls: AtomicUsize::new(0),
    });
    let ast_value =
        zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
            &ast,
            "deliver",
            vec![
                Value::String("event-01".into()),
                Value::String(r#"{"to":"a@example.test","subject":"Hi","text":"Body"}"#.into()),
            ],
            None,
            Some(&grants),
            Some(&policy),
            ast_host.clone(),
        )
        .expect("AST reference delivery");
    assert_eq!(bytecode_value, ast_value);
    assert_eq!(ast_host.calls.load(Ordering::SeqCst), 1);

    let missing_policy = RuntimePolicy::default();
    let no_policy_host = Arc::new(MailHost {
        delivery: Mutex::new(None),
        calls: AtomicUsize::new(0),
    });
    let no_policy = execute_with_host_services_and_policy_captured(
        &bytecode,
        vec![Value::String("event-02".into()), Value::String("{}".into())],
        ExecutionLimits::default(),
        Some(&grants),
        Some(&missing_policy),
        no_policy_host.clone(),
    );
    assert!(matches!(
        no_policy.value,
        Some(Value::Result(Err(message))) if matches!(&*message, Value::String(message) if message.contains("SMTP configuration is incomplete"))
    ));
    assert_eq!(no_policy_host.calls.load(Ordering::SeqCst), 0);

    let mut malformed = bytecode.clone();
    for instruction in &mut malformed.functions[0].code {
        if let BytecodeOp::Builtin {
            name, result_type, ..
        } = &mut instruction.operation
        {
            if name == "mail_deliver" {
                *result_type = Type::Int;
            }
        }
    }
    assert!(verify_bytecode(&malformed).unwrap_err()[0]
        .message
        .contains("mail_deliver"));

    let guarded_source = "fn effect() -> String { print(\"argument evaluated\"); return \"event\" } fn deliver() -> Result<Unit, String> uses Email { return mail_deliver(effect(), effect()) } fn main() {}";
    let (_, guarded_bytecode) = compile_source(guarded_source, "deliver");
    let denied = execute_with_host_services_and_policy_captured(
        &guarded_bytecode,
        vec![],
        ExecutionLimits::default(),
        None,
        Some(&policy),
        no_policy_host.clone(),
    );
    assert!(denied.output.is_empty());
    assert!(denied
        .error
        .is_some_and(|error| error.message.contains("Email")));
    assert_eq!(no_policy_host.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn bytecode_executes_scalar_calls_and_matches_mir() {
    let source = r#"
fn add(left: Int, right: Int) -> Int { return left + right }
fn compute(value: Int) -> Int {
    if value > 0 { return add(value, 2) } else { return add(value, -2) }
}
fn main() { print(compute(3)) }
"#;
    let (mir, bytecode) = compile_source(source, "compute");
    let arguments = vec![Value::Int(40)];
    let reference = execution::execute_function(
        &mir,
        "compute",
        arguments.clone(),
        ExecutionLimits::default(),
    )
    .expect("MIR reference execution");
    let output =
        execute(&bytecode, arguments, ExecutionLimits::default()).expect("bytecode execution");
    assert_eq!(output.value, reference.value);
    assert_eq!(output.value, Value::Int(42));
}

#[test]
fn generated_bounded_integer_expressions_match_independent_oracle_and_backends() {
    let mut state = 0x5eed_cafe_u64;
    let mut source = String::from("fn main() {}\n");
    let mut cases = Vec::new();
    for index in 0..96 {
        let mut next_operand = || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((state >> 32) % 41) as i64 - 20
        };
        let left = next_operand();
        let middle = next_operand();
        let right = next_operand();
        let first_operator = match state.rotate_left(7) % 3 {
            0 => '+',
            1 => '-',
            _ => '*',
        };
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let second_operator = match state.rotate_left(13) % 3 {
            0 => '+',
            1 => '-',
            _ => '*',
        };
        let first = match first_operator {
            '+' => left + middle,
            '-' => left - middle,
            '*' => left * middle,
            _ => unreachable!(),
        };
        let expected = match second_operator {
            '+' => first + right,
            '-' => first - right,
            '*' => first * right,
            _ => unreachable!(),
        };
        let name = format!("generated_{index}");
        source.push_str(&format!(
            "fn {name}() -> Int {{ return (({left} {first_operator} {middle}) {second_operator} {right}) }}\n"
        ));
        cases.push((name, expected));
    }

    let ast =
        parse(&lex(&source).expect("generated source lexes")).expect("generated source parses");
    let mir = crate::lower(&ast).expect("generated source type-checks and lowers");
    for (name, expected) in cases {
        let ast_value = zelyra_runtime::execute_function(&ast, &name, Vec::new(), None)
            .unwrap_or_else(|error| panic!("AST {name}: {}", error.message));
        let mir_value =
            execution::execute_function(&mir, &name, Vec::new(), ExecutionLimits::default())
                .unwrap_or_else(|error| panic!("MIR {name}: {}", error.message));
        let bytecode =
            compile(&mir, &name).unwrap_or_else(|error| panic!("compile {name}: {error:?}"));
        let decoded = decode(&encode(&bytecode).expect("bytecode encodes"))
            .expect("encoded bytecode verifies on load");
        let bytecode_value = execute(&bytecode, Vec::new(), ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("bytecode {name}: {}", error.message));
        let decoded_value = execute(&decoded, Vec::new(), ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("decoded bytecode {name}: {}", error.message));
        let oracle = Value::Int(expected);
        assert_eq!(ast_value, oracle, "independent integer oracle for {name}");
        assert_eq!(mir_value.value, oracle, "MIR for {name}");
        assert_eq!(bytecode_value.value, oracle, "bytecode for {name}");
        assert_eq!(decoded_value.value, oracle, "decoded bytecode for {name}");
    }
}

#[test]
fn integer_division_and_remainder_failures_match_ast_mir_and_bytecode_spans() {
    let source = r#"
fn divide(left: Int, right: Int) -> Int { return left / right }
fn remainder(left: Int, right: Int) -> Int { return left % right }
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source type-checks and lowers");
    for (name, arguments) in [
        ("divide", vec![Value::Int(1), Value::Int(0)]),
        ("remainder", vec![Value::Int(1), Value::Int(0)]),
        ("divide", vec![Value::Int(i64::MIN), Value::Int(-1)]),
        ("remainder", vec![Value::Int(i64::MIN), Value::Int(-1)]),
    ] {
        let expected = zelyra_runtime::execute_function(&ast, name, arguments.clone(), None)
            .expect_err("AST must reject zero division and signed overflow");
        let mir_error =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .expect_err("MIR must reject zero division and signed overflow");
        let bytecode = compile(&mir, name).expect("error-path function compiles");
        let decoded = decode(&encode(&bytecode).expect("error-path bytecode encodes"))
            .expect("error-path bytecode verifies after decoding");
        let actual = execute(&decoded, arguments, ExecutionLimits::default())
            .expect_err("bytecode must reject zero division and signed overflow");
        assert_eq!(mir_error.message, expected.message, "MIR {name}");
        assert_eq!(mir_error.span, expected.span, "MIR source span {name}");
        assert_eq!(actual.message, expected.message, "bytecode {name}");
        assert_eq!(actual.span, expected.span, "bytecode source span {name}");
    }
}

#[test]
fn configured_recursion_depth_matches_ast_mir_and_bytecode() {
    let source = r#"
fn descend(depth: Int) -> Int {
    if depth == 0 { return 0 }
    return descend(depth - 1) + 1
}
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source type-checks and lowers");
    let bytecode = compile(&mir, "descend").expect("recursive function compiles");
    let bytecode = decode(&encode(&bytecode).expect("recursive bytecode encodes"))
        .expect("recursive bytecode verifies after decoding");
    let max_stack_depth = 40;
    let policy = RuntimePolicy {
        max_stack_depth: Some(max_stack_depth),
        ..RuntimePolicy::default()
    };
    let arguments = vec![Value::Int(24)];
    let ast_result = zelyra_runtime::execute_function_with_capabilities_and_policies(
        &ast,
        "descend",
        arguments.clone(),
        None,
        None,
        Some(&policy),
    )
    .expect("AST honors the configured depth above the default");
    let limits = ExecutionLimits {
        max_steps: 10_000,
        max_stack_depth,
    };
    let mir_result = execution::execute_function(&mir, "descend", arguments.clone(), limits)
        .expect("MIR honors the same configured depth");
    let bytecode_result =
        execute(&bytecode, arguments, limits).expect("bytecode honors the same configured depth");
    assert_eq!(ast_result, Value::Int(24));
    assert_eq!(mir_result.value, ast_result);
    assert_eq!(bytecode_result.value, ast_result);

    let default_policy = RuntimePolicy {
        max_stack_depth: Some(20),
        ..RuntimePolicy::default()
    };
    let ast_error = zelyra_runtime::execute_function_with_capabilities_and_policies(
        &ast,
        "descend",
        vec![Value::Int(24)],
        None,
        None,
        Some(&default_policy),
    )
    .expect_err("AST enforces the configured limit");
    let limited = ExecutionLimits {
        max_steps: 10_000,
        max_stack_depth: 20,
    };
    let mir_error = execution::execute_function(&mir, "descend", vec![Value::Int(24)], limited)
        .expect_err("MIR enforces the same configured limit");
    let bytecode_error = execute(&bytecode, vec![Value::Int(24)], limited)
        .expect_err("bytecode enforces the same configured limit");
    assert!(ast_error.message.contains("depth"), "{}", ast_error.message);
    assert!(mir_error.message.contains("depth"), "{}", mir_error.message);
    assert!(
        bytecode_error.message.contains("depth"),
        "{}",
        bytecode_error.message
    );
    assert_eq!(mir_error.span, ast_error.span);
    assert_eq!(bytecode_error.span, ast_error.span);
}

#[test]
fn zero_step_budget_failure_keeps_source_location_across_backends() {
    let source = "fn run() -> Int {\n    answer = 40 + 2\n    return answer\n}\nfn main() {}\n";
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source type-checks and lowers");
    let bytecode = compile(&mir, "run").expect("run function compiles");
    let bytecode = decode(&encode(&bytecode).expect("bytecode encodes"))
        .expect("bytecode verifies after decoding");
    let policy = RuntimePolicy {
        step_budget: Some(0),
        ..RuntimePolicy::default()
    };
    let ast_error = zelyra_runtime::execute_function_with_capabilities_and_policies(
        &ast,
        "run",
        Vec::new(),
        None,
        None,
        Some(&policy),
    )
    .expect_err("AST rejects execution before the first step");
    let limits = ExecutionLimits {
        max_steps: 0,
        ..ExecutionLimits::default()
    };
    let mir_error = execution::execute_function(&mir, "run", Vec::new(), limits)
        .expect_err("MIR rejects execution before the first step");
    let bytecode_error = execute(&bytecode, Vec::new(), limits)
        .expect_err("bytecode rejects execution before the first step");
    assert!(ast_error.message.contains("step budget exceeded"));
    assert!(mir_error.message.contains("step budget exceeded"));
    assert!(bytecode_error.message.contains("step budget exceeded"));
    assert_eq!(mir_error.span, ast_error.span);
    assert_eq!(bytecode_error.span.line, ast_error.span.line);
}

#[test]
fn bytecode_to_float_conversion_matches_ast_for_the_laravel_projection_shape() {
    let source = r#"
fn useful_life(months: UInt) -> Float {
    return round(to_float(months) / 12.0, 2)
}
fn from_int(value: Int) -> Float { return to_float(value) }
fn from_uint(value: UInt) -> Float { return to_float(value) }
fn from_float(value: Float) -> Float { return to_float(value) }
fn from_string(value: String) -> Float { return to_float(value) }
fn invalid_string(value: String) -> Float { return to_float(value) }
fn invalid_float(value: Float) -> Float { return to_float(value) }
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source lowers to MIR");

    for (name, arguments, expected_value) in [
        ("useful_life", vec![Value::UInt(60)], Value::Float(5.0)),
        ("from_int", vec![Value::Int(-12)], Value::Float(-12.0)),
        ("from_uint", vec![Value::UInt(18)], Value::Float(18.0)),
        ("from_float", vec![Value::Float(2.75)], Value::Float(2.75)),
        (
            "from_string",
            vec![Value::String(" 3.125 ".into())],
            Value::Float(3.125),
        ),
    ] {
        let expected = zelyra_runtime::execute_function(&ast, name, arguments.clone(), None)
            .unwrap_or_else(|error| panic!("AST {name}: {}", error.message));
        let mir_result =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .unwrap_or_else(|error| panic!("MIR {name}: {}", error.message));
        let bytecode = decode(
            &encode(
                &compile(&mir, name)
                    .unwrap_or_else(|error| panic!("bytecode compile {name}: {error:?}")),
            )
            .expect("encode bytecode"),
        )
        .expect("decode bytecode");
        let bytecode_result = execute(&bytecode, arguments, ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("bytecode {name}: {}", error.message));
        assert_eq!(expected, expected_value, "AST {name}");
        assert_eq!(mir_result.value, expected, "MIR {name}");
        assert_eq!(bytecode_result.value, expected, "bytecode {name}");
    }

    for (name, input, message_fragment) in [
        (
            "invalid_string",
            Value::String("not-a-number".into()),
            "cannot convert",
        ),
        (
            "invalid_string",
            Value::String("inf".into()),
            "to a finite Float",
        ),
        (
            "invalid_float",
            Value::Float(f64::INFINITY),
            "requires a finite value",
        ),
    ] {
        let arguments = vec![input];
        let expected = zelyra_runtime::execute_function(&ast, name, arguments.clone(), None)
            .expect_err("AST must reject invalid or non-finite float conversions");
        let mir_error =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .expect_err("MIR must reject invalid or non-finite float conversions");
        let bytecode = decode(
            &encode(&compile(&mir, name).expect("compile error case")).expect("encode error case"),
        )
        .expect("decode error case");
        let actual = execute(&bytecode, arguments, ExecutionLimits::default())
            .expect_err("bytecode must reject invalid or non-finite float conversions");
        assert_eq!(mir_error.message, expected.message, "MIR error parity");
        assert_eq!(actual.message, expected.message, "bytecode error parity");
        assert!(
            actual.message.contains(message_fragment),
            "{}",
            actual.message
        );
    }
}

#[test]
fn integer_conversion_matches_ast_mir_and_serialized_bytecode() {
    let source = r#"
fn from_int(value: Int) -> Int { return to_int(value) }
fn from_uint(value: UInt) -> Int { return to_int(value) }
fn from_float(value: Float) -> Int { return to_int(value) }
fn from_bool(value: Bool) -> Int { return to_int(value) }
fn from_string(value: String) -> Int { return to_int(value) }
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source lowers to MIR");

    for (name, argument, expected_value) in [
        ("from_int", Value::Int(-12), Value::Int(-12)),
        ("from_uint", Value::UInt(42), Value::Int(42)),
        ("from_float", Value::Float(3.85), Value::Int(3)),
        ("from_float", Value::Float(-3.85), Value::Int(-3)),
        ("from_bool", Value::Bool(true), Value::Int(1)),
        ("from_bool", Value::Bool(false), Value::Int(0)),
        (
            "from_string",
            Value::String("  -9223372036854775808  ".into()),
            Value::Int(i64::MIN),
        ),
    ] {
        let arguments = vec![argument];
        let expected = zelyra_runtime::execute_function(&ast, name, arguments.clone(), None)
            .unwrap_or_else(|error| panic!("AST {name}: {}", error.message));
        let mir_result =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .unwrap_or_else(|error| panic!("MIR {name}: {}", error.message));
        let bytecode = decode(
            &encode(
                &compile(&mir, name)
                    .unwrap_or_else(|error| panic!("bytecode compile {name}: {error:?}")),
            )
            .expect("encode bytecode"),
        )
        .expect("decode bytecode");
        let bytecode_result = execute(&bytecode, arguments, ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("bytecode {name}: {}", error.message));
        assert_eq!(expected, expected_value, "AST {name}");
        assert_eq!(mir_result.value, expected, "MIR {name}");
        assert_eq!(bytecode_result.value, expected, "bytecode {name}");
    }

    for (name, input, message_fragment) in [
        ("from_uint", Value::UInt(u64::MAX), "to_int cannot convert"),
        (
            "from_float",
            Value::Float(2_f64.powi(63)),
            "to_int Float must be finite and within the Int range",
        ),
        (
            "from_float",
            Value::Float(f64::INFINITY),
            "to_int Float must be finite and within the Int range",
        ),
        (
            "from_string",
            Value::String("9223372036854775808".into()),
            "to_int cannot convert",
        ),
    ] {
        let arguments = vec![input];
        let expected = zelyra_runtime::execute_function(&ast, name, arguments.clone(), None)
            .expect_err("AST must reject out-of-range integer conversion");
        let mir_error =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .expect_err("MIR must reject out-of-range integer conversion");
        let bytecode = decode(
            &encode(&compile(&mir, name).expect("compile error case")).expect("encode error case"),
        )
        .expect("decode error case");
        let actual = execute(&bytecode, arguments, ExecutionLimits::default())
            .expect_err("bytecode must reject out-of-range integer conversion");
        assert_eq!(mir_error.message, expected.message, "MIR error parity");
        assert_eq!(actual.message, expected.message, "bytecode error parity");
        assert!(
            actual.message.contains(message_fragment),
            "{}",
            actual.message
        );
    }
}

#[test]
fn result_parsers_match_ast_mir_and_serialized_bytecode() {
    let source = r#"
fn parse_integer(value: String) -> Result<Int, String> { return parse_int(value) }
fn parse_decimal(value: String) -> Result<Float, String> { return parse_float(value) }
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source lowers to MIR");

    for (name, input, expected_value) in [
        (
            "parse_integer",
            "  -42  ",
            Value::Result(Ok(Box::new(Value::Int(-42)))),
        ),
        (
            "parse_integer",
            "9223372036854775808",
            Value::Result(Err(Box::new(Value::String(
                "invalid integer: 9223372036854775808".into(),
            )))),
        ),
        (
            "parse_decimal",
            " 3.125 ",
            Value::Result(Ok(Box::new(Value::Float(3.125)))),
        ),
        (
            "parse_decimal",
            "Infinity",
            Value::Result(Err(Box::new(Value::String(
                "invalid float: Infinity".into(),
            )))),
        ),
        (
            "parse_decimal",
            "not-a-number",
            Value::Result(Err(Box::new(Value::String(
                "invalid float: not-a-number".into(),
            )))),
        ),
    ] {
        let arguments = vec![Value::String(input.into())];
        let expected = zelyra_runtime::execute_function(&ast, name, arguments.clone(), None)
            .unwrap_or_else(|error| panic!("AST {name}: {}", error.message));
        let mir_result =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .unwrap_or_else(|error| panic!("MIR {name}: {}", error.message));
        let bytecode = decode(
            &encode(
                &compile(&mir, name)
                    .unwrap_or_else(|error| panic!("bytecode compile {name}: {error:?}")),
            )
            .expect("encode bytecode"),
        )
        .expect("decode bytecode");
        let bytecode_result = execute(&bytecode, arguments, ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("bytecode {name}: {}", error.message));
        assert_eq!(expected, expected_value, "AST {name}");
        assert_eq!(mir_result.value, expected, "MIR {name}");
        assert_eq!(bytecode_result.value, expected, "bytecode {name}");
    }
}

#[test]
fn array_collection_helpers_match_ast_mir_and_serialized_bytecode() {
    let source = r#"
fn unique_values(values: Int[]) -> Int[] { return unique(values) }
fn chunks(values: Int[], size: Int) -> Int[][] { return chunk(values, size) }
fn flatten_values(values: Int[][]) -> Int[] { return flatten(values) }
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source lowers to MIR");

    for (name, arguments, expected_value) in [
        (
            "unique_values",
            vec![Value::Array(vec![
                Value::Int(2),
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
                Value::Int(1),
            ])],
            Value::Array(vec![Value::Int(2), Value::Int(1), Value::Int(3)]),
        ),
        (
            "chunks",
            vec![
                Value::Array((1..=5).map(Value::Int).collect()),
                Value::Int(2),
            ],
            Value::Array(vec![
                Value::Array(vec![Value::Int(1), Value::Int(2)]),
                Value::Array(vec![Value::Int(3), Value::Int(4)]),
                Value::Array(vec![Value::Int(5)]),
            ]),
        ),
        (
            "chunks",
            vec![Value::Array(Vec::new()), Value::Int(4)],
            Value::Array(Vec::new()),
        ),
        (
            "flatten_values",
            vec![Value::Array(vec![
                Value::Array(vec![Value::Int(1), Value::Int(2)]),
                Value::Array(Vec::new()),
                Value::Array(vec![Value::Int(3)]),
            ])],
            Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
        ),
    ] {
        let expected = zelyra_runtime::execute_function(&ast, name, arguments.clone(), None)
            .unwrap_or_else(|error| panic!("AST {name}: {}", error.message));
        let mir_result =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .unwrap_or_else(|error| panic!("MIR {name}: {}", error.message));
        let bytecode = decode(
            &encode(
                &compile(&mir, name)
                    .unwrap_or_else(|error| panic!("bytecode compile {name}: {error:?}")),
            )
            .expect("encode bytecode"),
        )
        .expect("decode bytecode");
        let bytecode_result = execute(&bytecode, arguments, ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("bytecode {name}: {}", error.message));
        assert_eq!(expected, expected_value, "AST {name}");
        assert_eq!(mir_result.value, expected, "MIR {name}");
        assert_eq!(bytecode_result.value, expected, "bytecode {name}");
    }

    let invalid_arguments = vec![Value::Array(vec![Value::Int(1)]), Value::Int(0)];
    let expected =
        zelyra_runtime::execute_function(&ast, "chunks", invalid_arguments.clone(), None)
            .expect_err("AST rejects non-positive chunk sizes");
    let mir_error = execution::execute_function(
        &mir,
        "chunks",
        invalid_arguments.clone(),
        ExecutionLimits::default(),
    )
    .expect_err("MIR rejects non-positive chunk sizes");
    let bytecode = decode(
        &encode(&compile(&mir, "chunks").expect("compile error case")).expect("encode error case"),
    )
    .expect("decode error case");
    let actual = execute(&bytecode, invalid_arguments, ExecutionLimits::default())
        .expect_err("bytecode rejects non-positive chunk sizes");
    assert_eq!(mir_error.message, expected.message, "MIR error parity");
    assert_eq!(actual.message, expected.message, "bytecode error parity");
    assert!(actual
        .message
        .contains("chunk size must be greater than zero"));
}

#[test]
fn set_operations_match_ast_mir_and_serialized_bytecode() {
    let source = r#"
fn union(left: Set<Int>, right: Set<Int>) -> Set<Int> {
    return set_union(left, right)
}
fn intersection(left: Set<Int>, right: Set<Int>) -> Set<Int> {
    return set_intersection(left, right)
}
fn difference(left: Set<Int>, right: Set<Int>) -> Set<Int> {
    return set_difference(left, right)
}
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source lowers to MIR");
    let left = Value::Set(vec![Value::Int(1), Value::Int(3)]);
    let right = Value::Set(vec![Value::Int(2), Value::Int(3)]);

    for (name, expected_value) in [
        (
            "union",
            Value::Set(vec![Value::Int(1), Value::Int(3), Value::Int(2)]),
        ),
        ("intersection", Value::Set(vec![Value::Int(3)])),
        ("difference", Value::Set(vec![Value::Int(1)])),
    ] {
        let arguments = vec![left.clone(), right.clone()];
        let expected = zelyra_runtime::execute_function(&ast, name, arguments.clone(), None)
            .unwrap_or_else(|error| panic!("AST {name}: {}", error.message));
        let mir_result =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .unwrap_or_else(|error| panic!("MIR {name}: {}", error.message));
        let bytecode = decode(
            &encode(
                &compile(&mir, name)
                    .unwrap_or_else(|error| panic!("bytecode compile {name}: {error:?}")),
            )
            .expect("encode bytecode"),
        )
        .expect("decode bytecode");
        let bytecode_result = execute(&bytecode, arguments, ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("bytecode {name}: {}", error.message));
        assert_eq!(expected, expected_value, "AST {name}");
        assert_eq!(mir_result.value, expected, "MIR {name}");
        assert_eq!(bytecode_result.value, expected, "bytecode {name}");
    }
}

#[test]
fn map_update_helpers_match_ast_mir_and_serialized_bytecode() {
    let source = r#"
fn remove(map: Map<String, Int>, key: String) -> Map<String, Int> {
    return map_remove(map, key)
}
fn merge(left: Map<String, Int>, right: Map<String, Int>) -> Map<String, Int> {
    return map_merge(left, right)
}
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source lowers to MIR");
    let left = Value::Map(vec![
        (Value::String("a".into()), Value::Int(1)),
        (Value::String("b".into()), Value::Int(2)),
    ]);
    let right = Value::Map(vec![
        (Value::String("b".into()), Value::Int(20)),
        (Value::String("c".into()), Value::Int(3)),
    ]);

    for (name, arguments, expected_value) in [
        (
            "remove",
            vec![left.clone(), Value::String("a".into())],
            Value::Map(vec![(Value::String("b".into()), Value::Int(2))]),
        ),
        (
            "remove",
            vec![left.clone(), Value::String("missing".into())],
            left.clone(),
        ),
        (
            "merge",
            vec![left, right],
            Value::Map(vec![
                (Value::String("a".into()), Value::Int(1)),
                (Value::String("b".into()), Value::Int(20)),
                (Value::String("c".into()), Value::Int(3)),
            ]),
        ),
    ] {
        let expected = zelyra_runtime::execute_function(&ast, name, arguments.clone(), None)
            .unwrap_or_else(|error| panic!("AST {name}: {}", error.message));
        let mir_result =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .unwrap_or_else(|error| panic!("MIR {name}: {}", error.message));
        let bytecode = decode(
            &encode(
                &compile(&mir, name)
                    .unwrap_or_else(|error| panic!("bytecode compile {name}: {error:?}")),
            )
            .expect("encode bytecode"),
        )
        .expect("decode bytecode");
        let bytecode_result = execute(&bytecode, arguments, ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("bytecode {name}: {}", error.message));
        assert_eq!(expected, expected_value, "AST {name}");
        assert_eq!(mir_result.value, expected, "MIR {name}");
        assert_eq!(bytecode_result.value, expected, "bytecode {name}");
    }
}

#[test]
fn map_lookup_helpers_match_ast_mir_and_serialized_bytecode() {
    let source = r#"
fn fallback(map: Map<String, Int>, key: String, fallback_value: Int) -> Int {
    return get_or(map, key, fallback_value)
}
fn fallback_alias(map: Map<String, Int>, key: String, fallback_value: Int) -> Int {
    return get_or_default(map, key, fallback_value)
}
fn has(map: Map<String, Int>, key: String) -> Bool { return has_key(map, key) }
fn map_keys(map: Map<String, Int>) -> String[] { return keys(map) }
fn map_values(map: Map<String, Int>) -> Int[] { return values(map) }
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source lowers to MIR");
    let map = Value::Map(vec![
        (Value::String("b".into()), Value::Int(2)),
        (Value::String("a".into()), Value::Int(1)),
    ]);

    for (name, arguments, expected_value) in [
        (
            "fallback",
            vec![map.clone(), Value::String("b".into()), Value::Int(-1)],
            Value::Int(2),
        ),
        (
            "fallback",
            vec![map.clone(), Value::String("missing".into()), Value::Int(-1)],
            Value::Int(-1),
        ),
        (
            "fallback_alias",
            vec![map.clone(), Value::String("missing".into()), Value::Int(7)],
            Value::Int(7),
        ),
        (
            "has",
            vec![map.clone(), Value::String("a".into())],
            Value::Bool(true),
        ),
        (
            "has",
            vec![map.clone(), Value::String("z".into())],
            Value::Bool(false),
        ),
        (
            "map_keys",
            vec![map.clone()],
            Value::Array(vec![Value::String("b".into()), Value::String("a".into())]),
        ),
        (
            "map_values",
            vec![map],
            Value::Array(vec![Value::Int(2), Value::Int(1)]),
        ),
    ] {
        let expected = zelyra_runtime::execute_function(&ast, name, arguments.clone(), None)
            .unwrap_or_else(|error| panic!("AST {name}: {}", error.message));
        let mir_result =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .unwrap_or_else(|error| panic!("MIR {name}: {}", error.message));
        let bytecode = decode(
            &encode(
                &compile(&mir, name)
                    .unwrap_or_else(|error| panic!("bytecode compile {name}: {error:?}")),
            )
            .expect("encode bytecode"),
        )
        .expect("decode bytecode");
        let bytecode_result = execute(&bytecode, arguments, ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("bytecode {name}: {}", error.message));
        assert_eq!(expected, expected_value, "AST {name}");
        assert_eq!(mir_result.value, expected, "MIR {name}");
        assert_eq!(bytecode_result.value, expected, "bytecode {name}");
    }
}

#[test]
fn map_increment_matches_ast_mir_and_serialized_bytecode() {
    let source = r#"
fn increment(map: Map<String, Int>, key: String) -> Result<Map<String, Int>, String> {
    return map_increment(map, key)
}
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source lowers to MIR");

    for (map, key, expected_value) in [
        (
            Value::Map(vec![(Value::String("count".into()), Value::Int(4))]),
            "count",
            Value::Result(Ok(Box::new(Value::Map(vec![(
                Value::String("count".into()),
                Value::Int(5),
            )])))),
        ),
        (
            Value::Map(Vec::new()),
            "new",
            Value::Result(Ok(Box::new(Value::Map(vec![(
                Value::String("new".into()),
                Value::Int(1),
            )])))),
        ),
        (
            Value::Map(vec![(Value::String("count".into()), Value::Int(i64::MAX))]),
            "count",
            Value::Result(Err(Box::new(Value::String("map counter overflow".into())))),
        ),
    ] {
        let arguments = vec![map, Value::String(key.into())];
        let expected = zelyra_runtime::execute_function(&ast, "increment", arguments.clone(), None)
            .unwrap_or_else(|error| panic!("AST increment: {}", error.message));
        let mir_result = execution::execute_function(
            &mir,
            "increment",
            arguments.clone(),
            ExecutionLimits::default(),
        )
        .unwrap_or_else(|error| panic!("MIR increment: {}", error.message));
        let bytecode = decode(
            &encode(&compile(&mir, "increment").expect("compile increment"))
                .expect("encode bytecode"),
        )
        .expect("decode bytecode");
        let bytecode_result = execute(&bytecode, arguments, ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("bytecode increment: {}", error.message));
        assert_eq!(expected, expected_value);
        assert_eq!(mir_result.value, expected);
        assert_eq!(bytecode_result.value, expected);
    }
}

#[test]
fn numeric_array_aggregates_match_ast_mir_and_serialized_bytecode() {
    let source = r#"
fn sum_int(values: Int[]) -> Int { return array_sum(values) }
fn sum_float(values: Float[]) -> Float { return array_sum(values) }
fn sum_decimal(values: Decimal[]) -> Decimal { return array_sum(values) }
fn sum_empty(values: Int[]) -> Int { return array_sum(values) }
fn avg_int(values: Int[]) -> Float { return avg(values) }
fn avg_float(values: Float[]) -> Float { return avg(values) }
fn avg_decimal(values: Decimal[]) -> Decimal { return avg(values) }
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source lowers to MIR");

    let cases = [
        (
            "sum_int",
            vec![Value::Array(vec![
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
            ])],
            Value::Int(6),
        ),
        (
            "sum_float",
            vec![Value::Array(vec![Value::Float(1.25), Value::Float(2.75)])],
            Value::Float(4.0),
        ),
        (
            "sum_decimal",
            vec![Value::Array(vec![Value::Decimal(125), Value::Decimal(200)])],
            Value::Decimal(325),
        ),
        ("sum_empty", vec![Value::Array(Vec::new())], Value::Int(0)),
        (
            "avg_int",
            vec![Value::Array(vec![Value::Int(1), Value::Int(2)])],
            Value::Float(1.5),
        ),
        (
            "avg_float",
            vec![Value::Array(vec![Value::Float(-1.0), Value::Float(2.0)])],
            Value::Float(0.5),
        ),
        (
            "avg_decimal",
            vec![Value::Array(vec![Value::Decimal(101), Value::Decimal(204)])],
            Value::Decimal(152),
        ),
    ];

    for (name, arguments, expected_value) in cases {
        let expected = zelyra_runtime::execute_function(&ast, name, arguments.clone(), None)
            .unwrap_or_else(|error| panic!("AST {name}: {}", error.message));
        let mir_result =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .unwrap_or_else(|error| panic!("MIR {name}: {}", error.message));
        let bytecode = decode(
            &encode(
                &compile(&mir, name)
                    .unwrap_or_else(|error| panic!("bytecode compile {name}: {error:?}")),
            )
            .expect("encode bytecode"),
        )
        .expect("decode bytecode");
        let bytecode_result = execute(&bytecode, arguments, ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("bytecode {name}: {}", error.message));
        assert_eq!(expected, expected_value, "AST {name}");
        assert_eq!(mir_result.value, expected, "MIR {name}");
        assert_eq!(bytecode_result.value, expected, "bytecode {name}");
    }

    for (name, values, message_fragment) in [
        (
            "sum_int",
            vec![Value::Int(i64::MAX), Value::Int(1)],
            "sum integer overflow",
        ),
        (
            "sum_float",
            vec![Value::Float(f64::MAX), Value::Float(f64::MAX)],
            "array_sum Float overflow",
        ),
        (
            "sum_decimal",
            vec![Value::Decimal(i128::MAX), Value::Decimal(1)],
            "sum decimal overflow",
        ),
        (
            "avg_float",
            vec![Value::Float(f64::INFINITY)],
            "avg requires finite Float elements",
        ),
        ("avg_int", Vec::new(), "avg of empty array is undefined"),
    ] {
        let arguments = vec![Value::Array(values)];
        let expected = zelyra_runtime::execute_function(&ast, name, arguments.clone(), None)
            .expect_err("AST aggregate must reject the invalid operation");
        let mir_error =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .expect_err("MIR aggregate must reject the invalid operation");
        let bytecode = decode(
            &encode(&compile(&mir, name).expect("compile aggregate error case"))
                .expect("encode aggregate error case"),
        )
        .expect("decode aggregate error case");
        let actual = execute(&bytecode, arguments, ExecutionLimits::default())
            .expect_err("bytecode aggregate must reject the invalid operation");
        assert_eq!(mir_error.message, expected.message, "MIR error parity");
        assert_eq!(actual.message, expected.message, "bytecode error parity");
        assert!(
            actual.message.contains(message_fragment),
            "{}",
            actual.message
        );
    }
}

#[test]
fn array_extrema_match_ast_mir_and_serialized_bytecode() {
    let source = r#"
fn min_int(values: Int[]) -> Option<Int> { return min_of(values) }
fn max_uint(values: UInt[]) -> Option<UInt> { return max_of(values) }
fn max_float(values: Float[]) -> Option<Float> { return max_of(values) }
fn min_decimal(values: Decimal[]) -> Option<Decimal> { return min_of(values) }
fn max_string(values: String[]) -> Option<String> { return max_of(values) }
fn min_empty(values: Int[]) -> Option<Int> { return min_of(values) }
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source lowers to MIR");

    let cases = [
        (
            "min_int",
            vec![Value::Array(vec![
                Value::Int(8),
                Value::Int(-2),
                Value::Int(4),
            ])],
            Value::Option(Some(Box::new(Value::Int(-2)))),
        ),
        (
            "max_uint",
            vec![Value::Array(vec![
                Value::UInt(8),
                Value::UInt(12),
                Value::UInt(4),
            ])],
            Value::Option(Some(Box::new(Value::UInt(12)))),
        ),
        (
            "max_float",
            vec![Value::Array(vec![Value::Float(8.5), Value::Float(-2.0)])],
            Value::Option(Some(Box::new(Value::Float(8.5)))),
        ),
        (
            "min_decimal",
            vec![Value::Array(vec![Value::Decimal(8), Value::Decimal(-2)])],
            Value::Option(Some(Box::new(Value::Decimal(-2)))),
        ),
        (
            "max_string",
            vec![Value::Array(vec![
                Value::String("birch".into()),
                Value::String("cedar".into()),
                Value::String("ash".into()),
            ])],
            Value::Option(Some(Box::new(Value::String("cedar".into())))),
        ),
        (
            "min_empty",
            vec![Value::Array(Vec::new())],
            Value::Option(None),
        ),
    ];

    for (name, arguments, expected_value) in cases {
        let expected = zelyra_runtime::execute_function(&ast, name, arguments.clone(), None)
            .unwrap_or_else(|error| panic!("AST {name}: {}", error.message));
        let mir_result =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .unwrap_or_else(|error| panic!("MIR {name}: {}", error.message));
        let bytecode = decode(
            &encode(
                &compile(&mir, name)
                    .unwrap_or_else(|error| panic!("bytecode compile {name}: {error:?}")),
            )
            .expect("encode bytecode"),
        )
        .expect("decode bytecode");
        let bytecode_result = execute(&bytecode, arguments, ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("bytecode {name}: {}", error.message));
        assert_eq!(expected, expected_value, "AST {name}");
        assert_eq!(mir_result.value, expected, "MIR {name}");
        assert_eq!(bytecode_result.value, expected, "bytecode {name}");
    }

    {
        let (name, values, message) = (
            "max_float",
            vec![Value::Float(1.0), Value::Float(f64::NAN)],
            "max_of requires finite Float elements",
        );
        let arguments = vec![Value::Array(values)];
        let expected = zelyra_runtime::execute_function(&ast, name, arguments.clone(), None)
            .expect_err("AST extrema must reject non-finite Float values");
        let mir_error =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .expect_err("MIR extrema must reject non-finite Float values");
        let bytecode = decode(
            &encode(&compile(&mir, name).expect("compile non-finite case"))
                .expect("encode non-finite case"),
        )
        .expect("decode non-finite case");
        let actual = execute(&bytecode, arguments, ExecutionLimits::default())
            .expect_err("bytecode extrema must reject non-finite Float values");
        assert_eq!(mir_error.message, expected.message);
        assert_eq!(actual.message, expected.message);
        assert!(actual.message.contains(message));
    }
}

#[test]
fn date_and_variant_predicates_match_ast_mir_and_serialized_bytecode() {
    let source = r#"
fn add_days(date_value: Date, days: Int) -> Date { return date_add_days(date_value, days) }
fn difference(start: Date, end: Date) -> Int { return date_diff_days(start, end) }
fn before(left: Date, right: Date) -> Bool { return is_before(left, right) }
fn after(left: Date, right: Date) -> Bool { return is_after(left, right) }
fn option_is_some(value: Option<Int>) -> Bool { return is_some(value) }
fn option_is_none(value: Option<Int>) -> Bool { return is_none(value) }
fn result_is_ok(value: Result<Int, String>) -> Bool { return is_ok(value) }
fn result_is_err(value: Result<Int, String>) -> Bool { return is_err(value) }
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source lowers to MIR");

    let date_feb_28 = Value::Date("2024-02-28".into());
    let date_mar_01 = Value::Date("2024-03-01".into());
    let cases = [
        (
            "add_days",
            vec![date_feb_28.clone(), Value::Int(1)],
            Value::Date("2024-02-29".into()),
        ),
        (
            "difference",
            vec![date_mar_01.clone(), date_feb_28.clone()],
            Value::Int(2),
        ),
        (
            "before",
            vec![date_feb_28.clone(), date_mar_01.clone()],
            Value::Bool(true),
        ),
        ("after", vec![date_mar_01, date_feb_28], Value::Bool(true)),
        (
            "option_is_some",
            vec![Value::Option(Some(Box::new(Value::Int(1))))],
            Value::Bool(true),
        ),
        (
            "option_is_none",
            vec![Value::Option(None)],
            Value::Bool(true),
        ),
        (
            "result_is_ok",
            vec![Value::Result(Ok(Box::new(Value::Int(1))))],
            Value::Bool(true),
        ),
        (
            "result_is_err",
            vec![Value::Result(Err(Box::new(Value::String("bad".into()))))],
            Value::Bool(true),
        ),
    ];

    for (name, arguments, expected_value) in cases {
        let expected = zelyra_runtime::execute_function(&ast, name, arguments.clone(), None)
            .unwrap_or_else(|error| panic!("AST {name}: {}", error.message));
        let mir_result =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .unwrap_or_else(|error| panic!("MIR {name}: {}", error.message));
        let bytecode = decode(
            &encode(
                &compile(&mir, name)
                    .unwrap_or_else(|error| panic!("bytecode compile {name}: {error:?}")),
            )
            .expect("encode bytecode"),
        )
        .expect("decode bytecode");
        let bytecode_result = execute(&bytecode, arguments, ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("bytecode {name}: {}", error.message));
        assert_eq!(expected, expected_value, "AST {name}");
        assert_eq!(mir_result.value, expected, "MIR {name}");
        assert_eq!(bytecode_result.value, expected, "bytecode {name}");
    }

    let invalid_date_args = vec![Value::Date("2024-02-30".into()), Value::Int(1)];
    let expected =
        zelyra_runtime::execute_function(&ast, "add_days", invalid_date_args.clone(), None)
            .expect_err("AST must reject invalid Date values");
    let mir_error = execution::execute_function(
        &mir,
        "add_days",
        invalid_date_args.clone(),
        ExecutionLimits::default(),
    )
    .expect_err("MIR must reject invalid Date values");
    let bytecode = decode(
        &encode(&compile(&mir, "add_days").expect("compile invalid Date case"))
            .expect("encode invalid Date case"),
    )
    .expect("decode invalid Date case");
    let actual = execute(&bytecode, invalid_date_args, ExecutionLimits::default())
        .expect_err("bytecode must reject invalid Date values");
    assert_eq!(mir_error.message, expected.message);
    assert_eq!(actual.message, expected.message);
    assert!(actual.message.contains("date arithmetic is outside years"));
}

#[test]
fn hash_base64_and_url_helpers_match_ast_mir_and_serialized_bytecode() {
    let source = r#"
fn hash(value: String) -> String { return hash_sha256(value) }
fn base64_encode_value(value: String) -> String { return base64_encode(value) }
fn base64_decode_value(value: String) -> Result<String, String> { return base64_decode(value) }
fn url_encode_value(value: String) -> String { return url_encode(value) }
fn url_decode_value(value: String) -> Result<String, String> { return url_decode(value) }
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source lowers to MIR");

    for (name, input, expected_value) in [
        (
            "hash",
            "abc",
            Value::String(
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
            ),
        ),
        (
            "base64_encode_value",
            "hello",
            Value::String("aGVsbG8=".into()),
        ),
        (
            "base64_encode_value",
            "Grüße",
            Value::String("R3LDvMOfZQ==".into()),
        ),
        (
            "base64_decode_value",
            "aGVsbG8",
            Value::Result(Ok(Box::new(Value::String("hello".into())))),
        ),
        (
            "base64_decode_value",
            "%%%",
            Value::Result(Err(Box::new(Value::String(
                "invalid base64 character: `%`".into(),
            )))),
        ),
        (
            "url_encode_value",
            "Grüße a+b",
            Value::String("Gr%C3%BC%C3%9Fe%20a%2Bb".into()),
        ),
        (
            "url_decode_value",
            "Gr%C3%BC%C3%9Fe+a%2Bb",
            Value::Result(Ok(Box::new(Value::String("Grüße a+b".into())))),
        ),
        (
            "url_decode_value",
            "%2",
            Value::Result(Err(Box::new(Value::String(
                "incomplete percent encoding".into(),
            )))),
        ),
    ] {
        let arguments = vec![Value::String(input.into())];
        let expected = zelyra_runtime::execute_function(&ast, name, arguments.clone(), None)
            .unwrap_or_else(|error| panic!("AST {name}: {}", error.message));
        let mir_result =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .unwrap_or_else(|error| panic!("MIR {name}: {}", error.message));
        let bytecode = decode(
            &encode(
                &compile(&mir, name)
                    .unwrap_or_else(|error| panic!("bytecode compile {name}: {error:?}")),
            )
            .expect("encode bytecode"),
        )
        .expect("decode bytecode");
        let bytecode_result = execute(&bytecode, arguments, ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("bytecode {name}: {}", error.message));
        assert_eq!(expected, expected_value, "AST {name}");
        assert_eq!(mir_result.value, expected, "MIR {name}");
        assert_eq!(bytecode_result.value, expected, "bytecode {name}");
    }
}

#[test]
fn text_padding_matches_ast_mir_and_serialized_bytecode() {
    let source = r#"
fn left_unicode() -> String { return text_pad_left("é", 4, "🙂a") }
fn right_cycle() -> String { return text_pad_right("x", 4, "ab") }
fn empty_pad() -> String { return text_pad_left("x", 3, "") }
fn no_truncation() -> String { return text_pad_right("long", 2, ".") }
fn negative_length() -> String { return text_pad_left("x", -1, "0") }
fn excessive_length() -> String { return text_pad_right("x", 16777217, "0") }
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source lowers to MIR");

    for (name, expected_value) in [
        ("left_unicode", Value::String("🙂a🙂é".into())),
        ("right_cycle", Value::String("xaba".into())),
        ("empty_pad", Value::String("  x".into())),
        ("no_truncation", Value::String("long".into())),
    ] {
        let ast_result = zelyra_runtime::execute_function(&ast, name, vec![], None)
            .unwrap_or_else(|error| panic!("AST {name}: {}", error.message));
        let mir_result =
            execution::execute_function(&mir, name, vec![], ExecutionLimits::default())
                .unwrap_or_else(|error| panic!("MIR {name}: {}", error.message));
        let bytecode = decode(
            &encode(&compile(&mir, name).expect("function compiles")).expect("encode bytecode"),
        )
        .expect("decode bytecode");
        let bytecode_result = execute(&bytecode, vec![], ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("bytecode {name}: {}", error.message));
        assert_eq!(ast_result, expected_value, "AST {name}");
        assert_eq!(mir_result.value, expected_value, "MIR {name}");
        assert_eq!(bytecode_result.value, expected_value, "bytecode {name}");
    }

    for (name, message_fragment) in [
        ("negative_length", "must be non-negative"),
        ("excessive_length", "16 MiB text limit"),
    ] {
        let ast_error = zelyra_runtime::execute_function(&ast, name, vec![], None)
            .expect_err("AST rejects invalid padding length");
        let mir_error = execution::execute_function(&mir, name, vec![], ExecutionLimits::default())
            .expect_err("MIR rejects invalid padding length");
        let bytecode = decode(
            &encode(&compile(&mir, name).expect("function compiles")).expect("encode bytecode"),
        )
        .expect("decode bytecode");
        let bytecode_error = execute(&bytecode, vec![], ExecutionLimits::default())
            .expect_err("bytecode rejects invalid padding length");
        assert!(ast_error.message.contains(message_fragment), "AST {name}");
        assert_eq!(mir_error.message, ast_error.message, "MIR {name}");
        assert_eq!(bytecode_error.message, ast_error.message, "bytecode {name}");
    }
}

#[test]
fn array_replace_result_matches_ast_mir_and_serialized_bytecode() {
    let source = r#"
fn replace_valid() -> Result<Int[], String> {
    return array_replace_result([3, 4, 5], 1, 9)
}
fn replace_out_of_bounds() -> Result<Int[], String> {
    return array_replace_result([3, 4, 5], 8, 9)
}
fn replace_negative_index() -> Result<Int[], String> {
    return array_replace_result([3, 4, 5], -1, 9)
}
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source lowers to MIR");
    let cases = [
        (
            "replace_valid",
            Value::Result(Ok(Box::new(Value::Array(vec![
                Value::Int(3),
                Value::Int(9),
                Value::Int(5),
            ])))),
        ),
        (
            "replace_out_of_bounds",
            Value::Result(Err(Box::new(Value::String(
                "array_replace_result index is out of bounds".into(),
            )))),
        ),
        (
            "replace_negative_index",
            Value::Result(Err(Box::new(Value::String(
                "array_replace_result index is out of bounds".into(),
            )))),
        ),
    ];

    for (name, expected_value) in cases {
        let ast_result = zelyra_runtime::execute_function(&ast, name, vec![], None)
            .unwrap_or_else(|error| panic!("AST {name}: {}", error.message));
        let mir_result =
            execution::execute_function(&mir, name, vec![], ExecutionLimits::default())
                .unwrap_or_else(|error| panic!("MIR {name}: {}", error.message));
        let bytecode = decode(
            &encode(&compile(&mir, name).expect("function compiles")).expect("encode bytecode"),
        )
        .expect("decode bytecode");
        let bytecode_result = execute(&bytecode, vec![], ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("bytecode {name}: {}", error.message));
        assert_eq!(ast_result, expected_value, "AST {name}");
        assert_eq!(mir_result.value, expected_value, "MIR {name}");
        assert_eq!(bytecode_result.value, expected_value, "bytecode {name}");
    }
}

#[test]
fn collection_shape_builtins_match_ast_mir_and_serialized_bytecode() {
    let source = r#"
fn map_entries() -> Tuple<String, Int>[] {
    return entries(Map { "a": 1, "b": 2 })
}
fn zip_to_shorter_input() -> Tuple<Int, String>[] {
    return zip([1, 2, 3], ["x"])
}
fn remove_duplicates_in_first_seen_order() -> Int[] {
    return dedup([2, 1, 2, 1, 3])
}
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source lowers to MIR");
    let cases = [
        (
            "map_entries",
            Value::Array(vec![
                Value::Tuple(vec![Value::String("a".into()), Value::Int(1)]),
                Value::Tuple(vec![Value::String("b".into()), Value::Int(2)]),
            ]),
        ),
        (
            "zip_to_shorter_input",
            Value::Array(vec![Value::Tuple(vec![
                Value::Int(1),
                Value::String("x".into()),
            ])]),
        ),
        (
            "remove_duplicates_in_first_seen_order",
            Value::Array(vec![Value::Int(2), Value::Int(1), Value::Int(3)]),
        ),
    ];

    for (name, expected_value) in cases {
        let ast_result = zelyra_runtime::execute_function(&ast, name, vec![], None)
            .unwrap_or_else(|error| panic!("AST {name}: {}", error.message));
        let mir_result =
            execution::execute_function(&mir, name, vec![], ExecutionLimits::default())
                .unwrap_or_else(|error| panic!("MIR {name}: {}", error.message));
        let bytecode = decode(
            &encode(&compile(&mir, name).expect("function compiles")).expect("encode bytecode"),
        )
        .expect("decode bytecode");
        let bytecode_result = execute(&bytecode, vec![], ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("bytecode {name}: {}", error.message));
        assert_eq!(ast_result, expected_value, "AST {name}");
        assert_eq!(mir_result.value, expected_value, "MIR {name}");
        assert_eq!(bytecode_result.value, expected_value, "bytecode {name}");
    }
}

#[test]
fn result_text_builtins_match_ast_mir_and_serialized_bytecode() {
    let source = r#"
fn parse_bool_true() -> Result<Bool, String> { return parse_bool(" TrUe ") }
fn parse_bool_false() -> Result<Bool, String> { return parse_bool("0") }
fn parse_bool_invalid() -> Result<Bool, String> { return parse_bool("yes") }
fn replace_text() -> Result<String, String> {
    return text_replace_result("red blue red", "red", "green")
}
fn replace_empty_search() -> Result<String, String> {
    return text_replace_result("abc", "", "x")
}
fn split_text() -> Result<String[], String> {
    return text_split_result("a::b::c", "::")
}
fn split_empty_separator() -> Result<String[], String> {
    return text_split_result("abc", "")
}
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source lowers to MIR");
    let cases = [
        (
            "parse_bool_true",
            Value::Result(Ok(Box::new(Value::Bool(true)))),
        ),
        (
            "parse_bool_false",
            Value::Result(Ok(Box::new(Value::Bool(false)))),
        ),
        (
            "parse_bool_invalid",
            Value::Result(Err(Box::new(Value::String(
                "invalid boolean string: yes".into(),
            )))),
        ),
        (
            "replace_text",
            Value::Result(Ok(Box::new(Value::String("green blue green".into())))),
        ),
        (
            "replace_empty_search",
            Value::Result(Err(Box::new(Value::String(
                "text_replace_result requires a non-empty search text".into(),
            )))),
        ),
        (
            "split_text",
            Value::Result(Ok(Box::new(Value::Array(vec![
                Value::String("a".into()),
                Value::String("b".into()),
                Value::String("c".into()),
            ])))),
        ),
        (
            "split_empty_separator",
            Value::Result(Err(Box::new(Value::String(
                "text_split_result requires a non-empty separator".into(),
            )))),
        ),
    ];

    for (name, expected_value) in cases {
        let ast_result = zelyra_runtime::execute_function(&ast, name, vec![], None)
            .unwrap_or_else(|error| panic!("AST {name}: {}", error.message));
        let mir_result =
            execution::execute_function(&mir, name, vec![], ExecutionLimits::default())
                .unwrap_or_else(|error| panic!("MIR {name}: {}", error.message));
        let bytecode = decode(
            &encode(&compile(&mir, name).expect("function compiles")).expect("encode bytecode"),
        )
        .expect("decode bytecode");
        let bytecode_result = execute(&bytecode, vec![], ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("bytecode {name}: {}", error.message));
        assert_eq!(ast_result, expected_value, "AST {name}");
        assert_eq!(mir_result.value, expected_value, "MIR {name}");
        assert_eq!(bytecode_result.value, expected_value, "bytecode {name}");
    }
}

#[test]
fn unicode_text_slice_and_trim_builtins_match_ast_mir_and_bytecode() {
    let source = r#"
fn trim_left() -> String { return text_trim_start("  \tZelyra  ") }
fn trim_right() -> String { return text_trim_end("  \tZelyra  ") }
fn blank_unicode_space() -> Bool { return text_is_blank(" \t\n ") }
fn substring_unicode() -> String { return text_substring("A🙂BéC", 1, 3) }
fn substring_negative_start() -> String { return text_substring("A🙂B", -4, 2) }
fn substring_huge_length() -> String {
    return text_substring("A🙂B", 1, 9223372036854775807)
}
fn substring_after_end() -> String { return text_substring("abc", 99, 3) }
fn substring_negative_length() -> String { return text_substring("abc", 1, -9) }
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source lowers to MIR");
    let cases = [
        ("trim_left", Value::String("Zelyra  ".into())),
        ("trim_right", Value::String("  \tZelyra".into())),
        ("blank_unicode_space", Value::Bool(true)),
        ("substring_unicode", Value::String("🙂Bé".into())),
        ("substring_negative_start", Value::String("A🙂".into())),
        ("substring_huge_length", Value::String("🙂B".into())),
        ("substring_after_end", Value::String(String::new())),
        ("substring_negative_length", Value::String(String::new())),
    ];

    for (name, expected_value) in cases {
        let ast_result = zelyra_runtime::execute_function(&ast, name, vec![], None)
            .unwrap_or_else(|error| panic!("AST {name}: {}", error.message));
        let mir_result =
            execution::execute_function(&mir, name, vec![], ExecutionLimits::default())
                .unwrap_or_else(|error| panic!("MIR {name}: {}", error.message));
        let bytecode = decode(
            &encode(&compile(&mir, name).expect("function compiles")).expect("encode bytecode"),
        )
        .expect("decode bytecode");
        let bytecode_result = execute(&bytecode, vec![], ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("bytecode {name}: {}", error.message));
        assert_eq!(ast_result, expected_value, "AST {name}");
        assert_eq!(mir_result.value, expected_value, "MIR {name}");
        assert_eq!(bytecode_result.value, expected_value, "bytecode {name}");
    }
}

#[test]
fn unicode_text_index_repeat_and_case_builtins_match_all_backends() {
    let source = r#"
fn index_after_multibyte() -> Option<Int> { return text_index_of("é🙂z", "z") }
fn index_missing() -> Option<Int> { return text_index_of("é🙂z", "x") }
fn index_empty_needle() -> Option<Int> { return text_index_of("é🙂z", "") }
fn repeat_unicode() -> String { return text_repeat("é🙂", 2) }
fn repeat_zero() -> String { return text_repeat("abc", 0) }
fn repeat_empty_large_count() -> String { return text_repeat("", 9223372036854775807) }
fn empty_string() -> Bool { return text_is_empty("") }
fn nonempty_string() -> Bool { return text_is_empty(" ") }
fn capitalize_expanding_unicode() -> String { return text_capitalize("ßeta") }
fn capitalize_empty() -> String { return text_capitalize("") }
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source lowers to MIR");
    let cases = [
        (
            "index_after_multibyte",
            Value::Option(Some(Box::new(Value::Int(2)))),
        ),
        ("index_missing", Value::Option(None)),
        (
            "index_empty_needle",
            Value::Option(Some(Box::new(Value::Int(0)))),
        ),
        ("repeat_unicode", Value::String("é🙂é🙂".into())),
        ("repeat_zero", Value::String(String::new())),
        ("repeat_empty_large_count", Value::String(String::new())),
        ("empty_string", Value::Bool(true)),
        ("nonempty_string", Value::Bool(false)),
        (
            "capitalize_expanding_unicode",
            Value::String("SSeta".into()),
        ),
        ("capitalize_empty", Value::String(String::new())),
    ];

    for (name, expected_value) in cases {
        let ast_result = zelyra_runtime::execute_function(&ast, name, vec![], None)
            .unwrap_or_else(|error| panic!("AST {name}: {}", error.message));
        let mir_result =
            execution::execute_function(&mir, name, vec![], ExecutionLimits::default())
                .unwrap_or_else(|error| panic!("MIR {name}: {}", error.message));
        let bytecode = decode(
            &encode(&compile(&mir, name).expect("function compiles")).expect("encode bytecode"),
        )
        .expect("decode bytecode");
        let bytecode_result = execute(&bytecode, vec![], ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("bytecode {name}: {}", error.message));
        assert_eq!(ast_result, expected_value, "AST {name}");
        assert_eq!(mir_result.value, expected_value, "MIR {name}");
        assert_eq!(bytecode_result.value, expected_value, "bytecode {name}");
    }

    for (name, source, expected_message) in [
        (
            "repeat_negative",
            "fn repeat_negative() -> String { return text_repeat(\"x\", -1) } fn main() {}",
            "text_repeat count must be non-negative",
        ),
        (
            "repeat_over_limit",
            "fn repeat_over_limit() -> String { return text_repeat(\"ab\", 8388609) } fn main() {}",
            "text_repeat output exceeds the 16 MiB text limit",
        ),
    ] {
        let ast = parse(&lex(source).expect("error source lexes")).expect("error source parses");
        let mir = crate::lower(&ast).expect("error source lowers");
        let bytecode = compile(&mir, name).expect("error case compiles");
        let decoded = decode(&encode(&bytecode).expect("error bytecode encodes"))
            .expect("error bytecode decodes");
        let ast_error = zelyra_runtime::execute_function(&ast, name, vec![], None)
            .expect_err("AST rejects invalid repeat");
        let mir_error = execution::execute_function(&mir, name, vec![], ExecutionLimits::default())
            .expect_err("MIR rejects invalid repeat");
        let bytecode_error = execute(&bytecode, vec![], ExecutionLimits::default())
            .expect_err("bytecode rejects invalid repeat");
        let decoded_error = execute(&decoded, vec![], ExecutionLimits::default())
            .expect_err("decoded bytecode rejects invalid repeat");
        let expected_error_message = ast_error.message.clone();
        let expected_error_span = ast_error.span;
        for (backend, error) in [
            ("AST", ast_error),
            ("MIR", mir_error),
            ("bytecode", bytecode_error),
            ("decoded bytecode", decoded_error),
        ] {
            assert!(
                error.message.contains(expected_message),
                "{backend}: {}",
                error.message
            );
            assert_eq!(
                error.message, expected_error_message,
                "{backend} error message"
            );
            assert_eq!(error.span, expected_error_span, "{backend} source span");
        }
    }

    let generic_call = parse(
        &lex("fn main() { value = text_repeat<String>(\"x\", 2) }").expect("generic call lexes"),
    )
    .expect("generic call parses");
    let errors = crate::lower(&generic_call).expect_err("text_repeat is not generic");
    assert!(errors.iter().any(|error| {
        error
            .message
            .contains("text_repeat expects two arguments and no type arguments")
    }));
}

#[test]
fn core_text_builtins_match_ast_mir_and_serialized_bytecode() {
    let source = r#"
fn contains_unicode() -> Bool { return text_contains("Käse🙂", "🙂") }
fn trims_unicode_space() -> String { return text_trim("  \tZelyra  \n") }
fn splits_unicode_scalars() -> String[] { return text_chars("Aä🧭") }
fn lowercases_unicode() -> String { return text_to_lower("İ") }
fn uppercases_expanding_character() -> String { return text_to_upper("straße") }
fn ucfirst_preserves_non_ascii_leading_character() -> String { return text_ucfirst("ßeta") }
fn matches_prefix() -> Bool { return text_starts_with("Zelyra Studio", "Zelyra") }
fn matches_suffix() -> Bool { return text_ends_with("invoice.zyl", ".zyl") }
fn replaces_all_occurrences() -> String { return text_replace("é-é-é", "é", "🙂") }
fn replaces_empty_search() -> String { return text_replace("é🙂", "", "-") }
fn splits_by_unicode_separator() -> String[] { return text_split("é🙂é", "é") }
fn splits_by_empty_separator() -> String[] { return split("é🙂", "") }
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source lowers to MIR");
    let cases = [
        ("contains_unicode", Value::Bool(true)),
        ("trims_unicode_space", Value::String("Zelyra".into())),
        (
            "splits_unicode_scalars",
            Value::Array(
                ["A", "ä", "🧭"]
                    .into_iter()
                    .map(|character| Value::String(character.into()))
                    .collect(),
            ),
        ),
        ("lowercases_unicode", Value::String("i\u{307}".into())),
        (
            "uppercases_expanding_character",
            Value::String("STRASSE".into()),
        ),
        (
            "ucfirst_preserves_non_ascii_leading_character",
            Value::String("ßeta".into()),
        ),
        ("matches_prefix", Value::Bool(true)),
        ("matches_suffix", Value::Bool(true)),
        ("replaces_all_occurrences", Value::String("🙂-🙂-🙂".into())),
        ("replaces_empty_search", Value::String("-é-🙂-".into())),
        (
            "splits_by_unicode_separator",
            Value::Array(vec![
                Value::String(String::new()),
                Value::String("🙂".into()),
                Value::String(String::new()),
            ]),
        ),
        (
            "splits_by_empty_separator",
            Value::Array(vec![
                Value::String(String::new()),
                Value::String("é".into()),
                Value::String("🙂".into()),
                Value::String(String::new()),
            ]),
        ),
    ];

    for (name, expected_value) in cases {
        let ast_result = zelyra_runtime::execute_function(&ast, name, vec![], None)
            .unwrap_or_else(|error| panic!("AST {name}: {}", error.message));
        let mir_result =
            execution::execute_function(&mir, name, vec![], ExecutionLimits::default())
                .unwrap_or_else(|error| panic!("MIR {name}: {}", error.message));
        let bytecode = decode(
            &encode(&compile(&mir, name).expect("function compiles")).expect("encode bytecode"),
        )
        .expect("decode bytecode");
        let bytecode_result = execute(&bytecode, vec![], ExecutionLimits::default())
            .unwrap_or_else(|error| panic!("bytecode {name}: {}", error.message));
        assert_eq!(ast_result, expected_value, "AST {name}");
        assert_eq!(mir_result.value, expected_value, "MIR {name}");
        assert_eq!(bytecode_result.value, expected_value, "bytecode {name}");
    }

    let generic_call = parse(
        &lex("fn main() { value = text_contains<Bool>(\"x\", \"x\") }")
            .expect("generic text call lexes"),
    )
    .expect("generic text call parses");
    let errors = crate::lower(&generic_call).expect_err("text_contains is not generic");
    assert!(errors.iter().any(|error| {
        error
            .message
            .contains("text_contains expects two arguments and no type arguments")
    }));

    for (source, expected_message) in [
        (
            "fn main() { value = text_trim_start<String>(\"x\") }",
            "text_trim_start expects one argument and no type arguments",
        ),
        (
            "fn main() { value = text_substring<String>(\"x\", 0, 1) }",
            "text_substring expects three arguments and no type arguments",
        ),
        (
            "fn main() { value = parse_bool<Bool>(\"true\") }",
            "parse_bool expects one argument and no type arguments",
        ),
        (
            "fn main() { value = text_replace_result<String>(\"x\", \"x\", \"y\") }",
            "text_replace_result expects three arguments and no type arguments",
        ),
        (
            "fn main() { value = text_split_result<String>(\"x\", \"x\") }",
            "text_split_result expects two arguments and no type arguments",
        ),
        (
            "fn main() { value = text_pad_left<String>(\"x\", 2, \"0\") }",
            "text_pad_left expects three arguments and no type arguments",
        ),
    ] {
        let program = parse(&lex(source).expect("generic builtin call lexes"))
            .expect("generic builtin call parses");
        let errors = crate::lower(&program).expect_err("text builtin is not generic");
        assert!(errors
            .iter()
            .any(|error| error.message.contains(expected_message)));
    }
}

#[test]
fn bytecode_clock_and_environment_use_declared_capabilities_and_host_grants() {
    let source = r#"
            fn current() -> Timestamp uses Clock { return now() }
            fn closure_clock() -> Timestamp uses Clock {
                callback: Function<(Int) -> Timestamp> uses {Clock} = |unused| now()
                return callback(0)
            }
            fn setting() -> String? uses Environment { return env("APP_MODE") }
            fn main() {}
        "#;
    let ast = parse(&lex(source).unwrap()).unwrap();
    let mir = crate::lower(&ast).unwrap();
    let clock = compile(&mir, "current").unwrap();
    let closure_clock = compile(&mir, "closure_clock").unwrap();
    let environment = compile(&mir, "setting").unwrap();
    let encoded = encode(&clock).unwrap();
    let clock = decode(&encoded).unwrap();
    assert!(clock.functions[0]
        .declared_capabilities
        .contains(&"Clock".into()));

    let make_host = || {
        Arc::new(FixedHost {
            timestamp: 1_700_000_000_000,
            environment: Some("test".into()),
            calls: AtomicUsize::new(0),
        })
    };
    let clock_grants = HashSet::from(["Clock".to_owned()]);
    let clock_host = make_host();
    let clock_value = execute_with_host_services(
        &clock,
        vec![],
        ExecutionLimits::default(),
        Some(&clock_grants),
        clock_host.clone(),
    )
    .unwrap()
    .value;
    let expected_clock =
        zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
            &ast,
            "current",
            vec![],
            None,
            Some(&clock_grants),
            None,
            make_host(),
        )
        .unwrap();
    assert_eq!(clock_value, expected_clock);
    assert_eq!(clock_host.calls.load(Ordering::SeqCst), 1);

    let closure_clock = decode(&encode(&closure_clock).unwrap()).unwrap();
    let closure_host = make_host();
    let closure_value = execute_with_host_services(
        &closure_clock,
        vec![],
        ExecutionLimits::default(),
        Some(&clock_grants),
        closure_host.clone(),
    )
    .unwrap()
    .value;
    let expected_closure =
        zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
            &ast,
            "closure_clock",
            vec![],
            None,
            Some(&clock_grants),
            None,
            make_host(),
        )
        .unwrap();
    assert_eq!(closure_value, expected_closure);
    assert_eq!(closure_host.calls.load(Ordering::SeqCst), 1);

    let denied_closure_host = make_host();
    let denied_closure = execute_with_host_services(
        &closure_clock,
        vec![],
        ExecutionLimits::default(),
        Some(&HashSet::new()),
        denied_closure_host.clone(),
    )
    .expect_err("stored Clock closure still requires the project grant");
    assert!(denied_closure.message.contains("Clock"));
    assert_eq!(denied_closure_host.calls.load(Ordering::SeqCst), 0);

    let environment_grants = HashSet::from(["Environment".to_owned()]);
    let environment_host = make_host();
    let value = execute_with_host_services(
        &environment,
        vec![],
        ExecutionLimits::default(),
        Some(&environment_grants),
        environment_host.clone(),
    )
    .unwrap()
    .value;
    let expected =
        zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
            &ast,
            "setting",
            vec![],
            None,
            Some(&environment_grants),
            None,
            make_host(),
        )
        .unwrap();
    assert_eq!(value, expected);
    assert_eq!(
        value,
        Value::Option(Some(Box::new(Value::String("test".into()))))
    );
    assert_eq!(environment_host.calls.load(Ordering::SeqCst), 1);

    let denied_host = make_host();
    let denied = execute_with_host_services(
        &clock,
        vec![],
        ExecutionLimits::default(),
        None,
        denied_host.clone(),
    )
    .unwrap_err();
    assert!(denied.message.contains("`Clock` is not granted"));
    assert_eq!(denied_host.calls.load(Ordering::SeqCst), 0);

    let mut erased_capability = clock.clone();
    erased_capability.functions[0].declared_capabilities.clear();
    let errors = verify_bytecode(&erased_capability)
        .expect_err("bytecode cannot erase a host builtin's source capability");
    assert!(errors
        .iter()
        .any(|error| error.message.contains("capability")));
}

#[test]
fn bytecode_console_input_uses_declared_capability_and_host_grant() {
    let source = r#"
            fn prompt() -> String? uses Console { return read_console("Name? ") }
            fn main() {}
        "#;
    let ast = parse(&lex(source).unwrap()).unwrap();
    let mir = crate::lower(&ast).unwrap();
    let bytecode = compile(&mir, "prompt").unwrap();
    let serialized = encode(&bytecode).unwrap();
    let bytecode = decode(&serialized).unwrap();
    let grants = HashSet::from(["Console".to_owned()]);
    let host = Arc::new(ConsoleHost {
        line: Some("Ada".into()),
        prompt: std::sync::Mutex::new(None),
        calls: AtomicUsize::new(0),
    });
    let actual = execute_with_host_services(
        &bytecode,
        vec![],
        ExecutionLimits::default(),
        Some(&grants),
        host.clone(),
    )
    .unwrap()
    .value;
    let expected_host = Arc::new(ConsoleHost {
        line: Some("Ada".into()),
        prompt: std::sync::Mutex::new(None),
        calls: AtomicUsize::new(0),
    });
    let expected =
        zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
            &ast,
            "prompt",
            vec![],
            None,
            Some(&grants),
            None,
            expected_host.clone(),
        )
        .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(
        actual,
        Value::Option(Some(Box::new(Value::String("Ada".into()))))
    );
    assert_eq!(*host.prompt.lock().unwrap(), Some("Name? ".into()));
    assert_eq!(host.calls.load(Ordering::SeqCst), 1);

    let denied_host = Arc::new(ConsoleHost {
        line: Some("should not be read".into()),
        prompt: std::sync::Mutex::new(None),
        calls: AtomicUsize::new(0),
    });
    let denied = execute_with_host_services(
        &bytecode,
        vec![],
        ExecutionLimits::default(),
        None,
        denied_host.clone(),
    )
    .unwrap_err();
    assert!(denied.message.contains("`Console` is not granted"));
    assert_eq!(denied_host.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn bytecode_random_builtins_use_shared_host_semantics_and_grants() {
    let source = r#"
            fn draw() -> Int uses Random { return random_int(-10, 10) }
            fn make_id() -> String uses Random { return uuid_v4() }
            fn argument() -> Int uses Environment {
                configured = env("APP_LIMIT")
                return 10
            }
            fn guarded_draw() -> Int uses Environment, Random {
                return random_int(argument(), 20)
            }
            fn main() {}
        "#;
    let ast = parse(&lex(source).unwrap()).unwrap();
    let mir = crate::lower(&ast).unwrap();
    let draw = decode(&encode(&compile(&mir, "draw").unwrap()).unwrap()).unwrap();
    let make_id = decode(&encode(&compile(&mir, "make_id").unwrap()).unwrap()).unwrap();
    let grants = HashSet::from(["Random".to_owned()]);
    let make_host = || {
        Arc::new(FixedHost {
            timestamp: 1_700_000_000_000,
            environment: None,
            calls: AtomicUsize::new(0),
        })
    };

    let random_host = make_host();
    let random = execute_with_host_services(
        &draw,
        vec![],
        ExecutionLimits::default(),
        Some(&grants),
        random_host.clone(),
    )
    .unwrap()
    .value;
    let expected_random =
        zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
            &ast,
            "draw",
            vec![],
            None,
            Some(&grants),
            None,
            make_host(),
        )
        .unwrap();
    assert_eq!(random, expected_random);
    assert_eq!(random, Value::Int(-10));
    assert_eq!(random_host.calls.load(Ordering::SeqCst), 1);

    let uuid_host = make_host();
    let uuid = execute_with_host_services(
        &make_id,
        vec![],
        ExecutionLimits::default(),
        Some(&grants),
        uuid_host.clone(),
    )
    .unwrap()
    .value;
    let expected_uuid =
        zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
            &ast,
            "make_id",
            vec![],
            None,
            Some(&grants),
            None,
            make_host(),
        )
        .unwrap();
    assert_eq!(uuid, expected_uuid);
    assert_eq!(
        uuid,
        Value::String("00000000-0000-4000-8000-000000000000".into())
    );
    assert_eq!(uuid_host.calls.load(Ordering::SeqCst), 1);

    let denied_host = make_host();
    let denied = execute_with_host_services(
        &draw,
        vec![],
        ExecutionLimits::default(),
        None,
        denied_host.clone(),
    )
    .unwrap_err();
    assert!(denied.message.contains("`Random` is not granted"));
    assert_eq!(denied_host.calls.load(Ordering::SeqCst), 0);

    let guarded_draw = decode(&encode(&compile(&mir, "guarded_draw").unwrap()).unwrap()).unwrap();
    let environment_only = HashSet::from(["Environment".to_owned()]);
    let preflight_host = make_host();
    let denied_before_arguments = execute_with_host_services(
        &guarded_draw,
        vec![],
        ExecutionLimits::default(),
        Some(&environment_only),
        preflight_host.clone(),
    )
    .unwrap_err();
    assert!(denied_before_arguments
        .message
        .contains("`Random` is not granted"));
    assert_eq!(preflight_host.calls.load(Ordering::SeqCst), 0);
    let ast_preflight_host = make_host();
    let ast_denied_before_arguments =
        zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
            &ast,
            "guarded_draw",
            vec![],
            None,
            Some(&environment_only),
            None,
            ast_preflight_host.clone(),
        )
        .unwrap_err();
    assert!(ast_denied_before_arguments.message.contains("Random"));
    assert_eq!(ast_preflight_host.calls.load(Ordering::SeqCst), 0);

    let mut missing_preflight = guarded_draw.clone();
    for function in &mut missing_preflight.functions {
        function.code.retain(|instruction| {
            !matches!(
                &instruction.operation,
                BytecodeOp::HostCallPreflight { name, .. } if name == "random_int"
            )
        });
    }
    assert!(verify_bytecode(&missing_preflight)
        .unwrap_err()
        .iter()
        .any(|error| error.message.contains("has no preflight")));

    let mut erased_capability = draw;
    erased_capability.functions[0].declared_capabilities.clear();
    assert!(verify_bytecode(&erased_capability)
        .unwrap_err()
        .iter()
        .any(|error| error.message.contains("capability")));
}

#[test]
fn bytecode_process_calls_share_ast_allowlist_limits_and_capability_order() {
    type ProcessRequest = (String, Vec<String>, u64, usize);

    struct ProcessHost {
        calls: AtomicUsize,
        request: std::sync::Mutex<Option<ProcessRequest>>,
    }

    impl HostServices for ProcessHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            Ok(0)
        }

        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }

        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }

        fn run_process(
            &self,
            command: &str,
            arguments: &[String],
            timeout_ms: u64,
            max_output_bytes: usize,
        ) -> Result<String, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            *self.request.lock().unwrap() = Some((
                command.to_owned(),
                arguments.to_vec(),
                timeout_ms,
                max_output_bytes,
            ));
            Ok("mock output".to_owned())
        }
    }

    let source = r#"
fn run() -> String uses Process { return run_process("approved-tool", ["--format", "json"]) }
fn denied() -> String uses Process { return run_process("other-tool", ["unused"]) }
fn side_effect_command() -> String { print("argument evaluated"); return "approved-tool" }
fn guarded() -> String uses Process { return run_process(side_effect_command(), ["unused"]) }
fn main() {}
"#;
    let ast = parse(&lex(source).unwrap()).unwrap();
    let mir = crate::lower(&ast).unwrap();
    let grants = HashSet::from(["Process".to_owned()]);
    let empty_grants = HashSet::new();
    let policy = RuntimePolicy {
        process: Some(ProcessPolicy {
            allowed_commands: vec!["approved-tool".to_owned()],
            timeout_ms: 250,
            max_output_bytes: 80,
        }),
        ..RuntimePolicy::default()
    };
    let process_host = Arc::new(ProcessHost {
        calls: AtomicUsize::new(0),
        request: std::sync::Mutex::new(None),
    });
    let host_services: Arc<dyn HostServices> = process_host.clone();
    let program = decode(&encode(&compile(&mir, "run").unwrap()).unwrap()).unwrap();
    let vm_result = execute_with_host_services_and_policy_captured(
        &program,
        Vec::new(),
        ExecutionLimits::default(),
        Some(&grants),
        Some(&policy),
        host_services.clone(),
    );
    let ast_result =
        zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
            &ast,
            "run",
            Vec::new(),
            None,
            Some(&grants),
            Some(&policy),
            host_services.clone(),
        );
    assert_eq!(vm_result.error, None);
    assert_eq!(vm_result.value, ast_result.value);
    assert_eq!(vm_result.value, Some(Value::String("mock output".into())));
    assert_eq!(process_host.calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        *process_host.request.lock().unwrap(),
        Some((
            "approved-tool".into(),
            vec!["--format".into(), "json".into()],
            250,
            80,
        ))
    );

    let denied_program = decode(&encode(&compile(&mir, "denied").unwrap()).unwrap()).unwrap();
    let denied_vm = execute_with_host_services_and_policy_captured(
        &denied_program,
        Vec::new(),
        ExecutionLimits::default(),
        Some(&grants),
        Some(&policy),
        host_services.clone(),
    );
    let denied_ast =
        zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
            &ast,
            "denied",
            Vec::new(),
            None,
            Some(&grants),
            Some(&policy),
            host_services.clone(),
        );
    assert_eq!(denied_vm.error, denied_ast.error);
    assert!(denied_vm
        .error
        .is_some_and(|error| error.message.contains("process execution denied")));
    assert_eq!(process_host.calls.load(Ordering::SeqCst), 2);

    let guarded_program = decode(&encode(&compile(&mir, "guarded").unwrap()).unwrap()).unwrap();
    let guarded_vm = execute_with_host_services_and_policy_captured(
        &guarded_program,
        Vec::new(),
        ExecutionLimits::default(),
        Some(&empty_grants),
        Some(&policy),
        host_services.clone(),
    );
    let guarded_ast =
        zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
            &ast,
            "guarded",
            Vec::new(),
            None,
            Some(&empty_grants),
            Some(&policy),
            host_services,
        );
    assert!(guarded_vm.output.is_empty());
    assert!(guarded_ast.output.is_empty());
    assert!(guarded_vm
        .error
        .is_some_and(|error| error.message.contains("Process")));
    assert!(guarded_ast
        .error
        .is_some_and(|error| error.message.contains("Process")));
    assert_eq!(process_host.calls.load(Ordering::SeqCst), 2);
}

#[test]
fn bytecode_http_get_shares_ast_network_policy_and_capability_order() {
    type HttpRequest = (String, String, u64, usize);

    struct HttpHost {
        calls: AtomicUsize,
        request: std::sync::Mutex<Option<HttpRequest>>,
    }

    impl HostServices for HttpHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            Ok(0)
        }

        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }

        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }

        fn http_request(
            &self,
            method: &str,
            url: &str,
            _headers: &[(String, String)],
            _body: Option<&[u8]>,
            timeout_ms: u64,
            max_response_bytes: usize,
        ) -> Result<zelyra_runtime::HostHttpResponse, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            *self.request.lock().unwrap() = Some((
                method.to_owned(),
                url.to_owned(),
                timeout_ms,
                max_response_bytes,
            ));
            Ok(zelyra_runtime::HostHttpResponse {
                status: 200,
                headers: vec![("content-type".into(), "text/plain".into())],
                body: b"mock response".to_vec(),
            })
        }
    }

    let source = r#"
fn load() -> String uses Network { return http_get("https://api.example.test/data") }
fn side_effect_url() -> String { print("argument evaluated"); return "https://api.example.test/data" }
fn guarded() -> String uses Network { return http_get(side_effect_url()) }
fn main() {}
"#;
    let ast = parse(&lex(source).unwrap()).unwrap();
    let mir = crate::lower(&ast).unwrap();
    let grants = HashSet::from(["Network".to_owned()]);
    let empty_grants = HashSet::new();
    let policy = RuntimePolicy {
        network: Some(NetworkPolicy {
            allowed_hosts: vec!["api.example.test".into()],
            timeout_ms: 350,
            max_response_bytes: 128,
        }),
        ..RuntimePolicy::default()
    };
    let http_host = Arc::new(HttpHost {
        calls: AtomicUsize::new(0),
        request: std::sync::Mutex::new(None),
    });
    let host_services: Arc<dyn HostServices> = http_host.clone();
    let program = decode(&encode(&compile(&mir, "load").unwrap()).unwrap()).unwrap();
    let vm_result = execute_with_host_services_and_policy_captured(
        &program,
        Vec::new(),
        ExecutionLimits::default(),
        Some(&grants),
        Some(&policy),
        host_services.clone(),
    );
    let ast_result =
        zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
            &ast,
            "load",
            Vec::new(),
            None,
            Some(&grants),
            Some(&policy),
            host_services.clone(),
        );
    assert_eq!(vm_result.error, None);
    assert_eq!(vm_result.value, ast_result.value);
    assert_eq!(vm_result.value, Some(Value::String("mock response".into())));
    assert_eq!(http_host.calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        *http_host.request.lock().unwrap(),
        Some((
            "GET".into(),
            "https://api.example.test/data".into(),
            350,
            128,
        ))
    );

    let denied_policy = RuntimePolicy {
        network: Some(NetworkPolicy {
            allowed_hosts: vec!["other.example.test".into()],
            timeout_ms: 350,
            max_response_bytes: 128,
        }),
        ..RuntimePolicy::default()
    };
    let denied_vm = execute_with_host_services_and_policy_captured(
        &program,
        Vec::new(),
        ExecutionLimits::default(),
        Some(&grants),
        Some(&denied_policy),
        host_services.clone(),
    );
    let denied_ast =
        zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
            &ast,
            "load",
            Vec::new(),
            None,
            Some(&grants),
            Some(&denied_policy),
            host_services.clone(),
        );
    assert!(denied_vm
        .error
        .as_ref()
        .is_some_and(|error| error.message.contains("network access denied")));
    assert_eq!(
        denied_vm.error.as_ref().map(|error| error.message.as_str()),
        denied_ast
            .error
            .as_ref()
            .map(|error| error.message.as_str())
    );
    assert_eq!(http_host.calls.load(Ordering::SeqCst), 2);

    let guarded_program = decode(&encode(&compile(&mir, "guarded").unwrap()).unwrap()).unwrap();
    let guarded_vm = execute_with_host_services_and_policy_captured(
        &guarded_program,
        Vec::new(),
        ExecutionLimits::default(),
        Some(&empty_grants),
        Some(&policy),
        host_services.clone(),
    );
    let guarded_ast =
        zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
            &ast,
            "guarded",
            Vec::new(),
            None,
            Some(&empty_grants),
            Some(&policy),
            host_services,
        );
    assert!(guarded_vm.output.is_empty());
    assert_eq!(guarded_vm.output, guarded_ast.output);
    assert!(guarded_vm
        .error
        .as_ref()
        .is_some_and(|error| error.message.contains("Network")));
    assert!(guarded_ast
        .error
        .as_ref()
        .is_some_and(|error| error.message.contains("Network")));
    assert_eq!(http_host.calls.load(Ordering::SeqCst), 2);
}

#[test]
fn bytecode_http_request_matches_ast_value_and_host_policy() {
    type HttpRequest = (
        String,
        String,
        Vec<(String, String)>,
        Option<Vec<u8>>,
        u64,
        usize,
    );

    struct HttpHost {
        calls: AtomicUsize,
        request: std::sync::Mutex<Option<HttpRequest>>,
    }

    impl HostServices for HttpHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            Ok(0)
        }

        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }

        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }

        fn http_request(
            &self,
            method: &str,
            url: &str,
            headers: &[(String, String)],
            body: Option<&[u8]>,
            timeout_ms: u64,
            max_response_bytes: usize,
        ) -> Result<zelyra_runtime::HostHttpResponse, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            *self.request.lock().unwrap() = Some((
                method.to_owned(),
                url.to_owned(),
                headers.to_vec(),
                body.map(<[u8]>::to_vec),
                timeout_ms,
                max_response_bytes,
            ));
            Ok(zelyra_runtime::HostHttpResponse {
                status: 201,
                headers: vec![("x-result".into(), "created".into())],
                body: b"created".to_vec(),
            })
        }
    }

    let source = r#"
fn request(method: String, url: String) -> HttpResponse uses Network {
    return http_request(method, url, ["X-Test: yes"], Some("payload"))
}
fn side_effect_url() -> String { print("argument evaluated"); return "https://api.example.test/" }
fn guarded() -> HttpResponse uses Network {
    return http_request("GET", side_effect_url(), ["X-Test: no"], Some("body"))
}
fn main() {}
"#;
    let ast = parse(&lex(source).unwrap()).unwrap();
    let mir = crate::lower(&ast).unwrap();
    let invalid = parse(&lex(r#"fn bad() -> HttpResponse uses Network { return http_request("POST", "https://api.example.test/", ["X-Test: yes"], "not optional") } fn main() {}"#).unwrap()).unwrap();
    let errors = crate::lower(&invalid).unwrap_err();
    assert!(errors[0]
        .message
        .contains("expected `Option<String>`, found `String`"));
    let grants = HashSet::from(["Network".to_owned()]);
    let empty_grants = HashSet::new();
    let policy = RuntimePolicy {
        network: Some(NetworkPolicy {
            allowed_hosts: vec!["api.example.test".into()],
            timeout_ms: 500,
            max_response_bytes: 2048,
        }),
        ..RuntimePolicy::default()
    };
    let http_host = Arc::new(HttpHost {
        calls: AtomicUsize::new(0),
        request: std::sync::Mutex::new(None),
    });
    let host_services: Arc<dyn HostServices> = http_host.clone();
    let program = decode(&encode(&compile(&mir, "request").unwrap()).unwrap()).unwrap();
    let arguments = vec![
        Value::String("POST".into()),
        Value::String("https://api.example.test/orders".into()),
    ];
    let vm_result = execute_with_host_services_and_policy_captured(
        &program,
        arguments.clone(),
        ExecutionLimits::default(),
        Some(&grants),
        Some(&policy),
        host_services.clone(),
    );
    let ast_result =
        zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
            &ast,
            "request",
            arguments,
            None,
            Some(&grants),
            Some(&policy),
            host_services.clone(),
        );
    assert_eq!(vm_result.error, None);
    assert_eq!(vm_result.value, ast_result.value);
    assert_eq!(
        vm_result.value,
        Some(Value::Object {
            type_name: "HttpResponse".into(),
            type_args: Vec::new(),
            fields: HashMap::from([
                ("status".into(), Value::Int(201)),
                (
                    "headers".into(),
                    Value::Array(vec![Value::String("x-result: created".into())]),
                ),
                ("body".into(), Value::String("created".into())),
            ]),
        })
    );
    assert_eq!(
        *http_host.request.lock().unwrap(),
        Some((
            "POST".into(),
            "https://api.example.test/orders".into(),
            vec![("x-test".into(), "yes".into())],
            Some(b"payload".to_vec()),
            500,
            2048,
        ))
    );
    assert_eq!(http_host.calls.load(Ordering::SeqCst), 2);

    let denied_policy = RuntimePolicy {
        network: Some(NetworkPolicy {
            allowed_hosts: vec!["elsewhere.example.test".into()],
            timeout_ms: 500,
            max_response_bytes: 2048,
        }),
        ..RuntimePolicy::default()
    };
    let denied_vm = execute_with_host_services_and_policy_captured(
        &program,
        vec![
            Value::String("POST".into()),
            Value::String("https://api.example.test/orders".into()),
        ],
        ExecutionLimits::default(),
        Some(&grants),
        Some(&denied_policy),
        host_services.clone(),
    );
    let denied_ast =
        zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
            &ast,
            "request",
            vec![
                Value::String("POST".into()),
                Value::String("https://api.example.test/orders".into()),
            ],
            None,
            Some(&grants),
            Some(&denied_policy),
            host_services.clone(),
        );
    assert!(denied_vm
        .error
        .as_ref()
        .is_some_and(|error| error.message.contains("network access denied")));
    assert_eq!(
        denied_vm.error.as_ref().map(|error| error.message.as_str()),
        denied_ast
            .error
            .as_ref()
            .map(|error| error.message.as_str())
    );
    assert_eq!(http_host.calls.load(Ordering::SeqCst), 2);

    let guarded_program = decode(&encode(&compile(&mir, "guarded").unwrap()).unwrap()).unwrap();
    let guarded_vm = execute_with_host_services_and_policy_captured(
        &guarded_program,
        Vec::new(),
        ExecutionLimits::default(),
        Some(&empty_grants),
        Some(&policy),
        host_services.clone(),
    );
    let guarded_ast =
        zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
            &ast,
            "guarded",
            Vec::new(),
            None,
            Some(&empty_grants),
            Some(&policy),
            host_services,
        );
    assert!(guarded_vm.output.is_empty());
    assert_eq!(guarded_vm.output, guarded_ast.output);
    assert!(guarded_vm
        .error
        .as_ref()
        .is_some_and(|error| error.message.contains("Network")));
    assert!(guarded_ast
        .error
        .as_ref()
        .is_some_and(|error| error.message.contains("Network")));
    assert_eq!(http_host.calls.load(Ordering::SeqCst), 2);
}

#[test]
fn bytecode_typed_http_json_roundtrips_project_records_with_ast_parity() {
    struct JsonHost {
        requests: AtomicUsize,
        body: std::sync::Mutex<Option<Vec<u8>>>,
        status: u16,
    }
    impl HostServices for JsonHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            Ok(0)
        }
        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }
        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }
        fn http_request(
            &self,
            _method: &str,
            _url: &str,
            headers: &[(String, String)],
            body: Option<&[u8]>,
            _timeout_ms: u64,
            _max_response_bytes: usize,
        ) -> Result<zelyra_runtime::HostHttpResponse, String> {
            self.requests.fetch_add(1, Ordering::SeqCst);
            *self.body.lock().unwrap() = body.map(<[u8]>::to_vec);
            assert!(headers.iter().any(|(name, value)| {
                name.eq_ignore_ascii_case("content-type") && value == "application/json"
            }));
            Ok(zelyra_runtime::HostHttpResponse {
                status: self.status,
                headers: vec![("content-type".into(), "application/json".into())],
                body: br#"{"message":"Hello Ada"}"#.to_vec(),
            })
        }
    }

    let source = r#"
struct EchoRequest { name: String }
struct EchoResponse { message: String }
type EchoLabel = String
enum EchoState { Ready Failed(Int) }
fn call() -> EchoResponse uses Network {
    return http_json<EchoRequest, EchoResponse>(
        "POST",
        "https://api.example.test/echo",
        [],
        Some(EchoRequest { name: "Ada" })
    )
}
fn call_result() -> HttpResult<EchoResponse> uses Network {
    return http_result<EchoRequest, EchoResponse>(
        "POST",
        "https://api.example.test/echo",
        [],
        Some(EchoRequest { name: "Ada" })
    )
}
fn side_effect_url() -> String { print("argument evaluated"); return "https://api.example.test/echo" }
fn guarded() -> EchoResponse uses Network {
    return http_json<EchoRequest, EchoResponse>("POST", side_effect_url(), [], None)
}
fn main() {}
"#;
    let ast = parse(&lex(source).unwrap()).unwrap();
    let mir = crate::lower(&ast).unwrap();
    let program = decode(&encode(&compile(&mir, "call").unwrap()).unwrap()).unwrap();
    assert_eq!(program.records.len(), 2);
    assert_eq!(program.type_aliases.len(), 1);
    assert_eq!(program.enums.len(), 1);
    assert_eq!(program.enums[0].variants, vec!["Ready", "Failed"]);
    let mut malformed_definitions = program.clone();
    let duplicated_field = malformed_definitions.records[0].fields[0].clone();
    malformed_definitions.records[0]
        .fields
        .push(duplicated_field);
    assert!(encode(&malformed_definitions).is_err());
    let grants = HashSet::from(["Network".to_owned()]);
    let policy = RuntimePolicy {
        network: Some(NetworkPolicy {
            allowed_hosts: vec!["api.example.test".into()],
            timeout_ms: 600,
            max_response_bytes: 4096,
        }),
        ..RuntimePolicy::default()
    };
    let host = Arc::new(JsonHost {
        requests: AtomicUsize::new(0),
        body: std::sync::Mutex::new(None),
        status: 200,
    });
    let host_services: Arc<dyn HostServices> = host.clone();
    let vm = execute_with_host_services_and_policy_captured(
        &program,
        Vec::new(),
        ExecutionLimits::default(),
        Some(&grants),
        Some(&policy),
        host_services.clone(),
    );
    let ast_result =
        zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
            &ast,
            "call",
            Vec::new(),
            None,
            Some(&grants),
            Some(&policy),
            host_services.clone(),
        );
    assert_eq!(vm.error, None);
    assert_eq!(vm.value, ast_result.value);
    assert_eq!(
        *host.body.lock().unwrap(),
        Some(br#"{"name":"Ada"}"#.to_vec())
    );
    assert_eq!(host.requests.load(Ordering::SeqCst), 2);

    let result_program = decode(&encode(&compile(&mir, "call_result").unwrap()).unwrap()).unwrap();
    let result_vm = execute_with_host_services_and_policy_captured(
        &result_program,
        Vec::new(),
        ExecutionLimits::default(),
        Some(&grants),
        Some(&policy),
        host_services.clone(),
    );
    let result_ast =
        zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
            &ast,
            "call_result",
            Vec::new(),
            None,
            Some(&grants),
            Some(&policy),
            host_services.clone(),
        );
    assert_eq!(result_vm.error, None);
    assert_eq!(result_vm.value, result_ast.value);

    let error_host = Arc::new(JsonHost {
        requests: AtomicUsize::new(0),
        body: std::sync::Mutex::new(None),
        status: 422,
    });
    let error_host_services: Arc<dyn HostServices> = error_host.clone();
    let error_vm = execute_with_host_services_and_policy_captured(
        &result_program,
        Vec::new(),
        ExecutionLimits::default(),
        Some(&grants),
        Some(&policy),
        error_host_services.clone(),
    );
    let error_ast =
        zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
            &ast,
            "call_result",
            Vec::new(),
            None,
            Some(&grants),
            Some(&policy),
            error_host_services,
        );
    assert_eq!(error_vm.error, None);
    assert_eq!(error_vm.value, error_ast.value);
    assert!(matches!(
        error_vm.value,
        Some(Value::Object { ref fields, .. })
            if matches!(fields.get("data"), Some(Value::Option(None)))
                && matches!(fields.get("error"), Some(Value::Option(Some(_))))
    ));

    let guarded_program = decode(&encode(&compile(&mir, "guarded").unwrap()).unwrap()).unwrap();
    let no_grants = HashSet::new();
    let calls_before = host.requests.load(Ordering::SeqCst);
    let guarded_vm = execute_with_host_services_and_policy_captured(
        &guarded_program,
        Vec::new(),
        ExecutionLimits::default(),
        Some(&no_grants),
        Some(&policy),
        host_services.clone(),
    );
    let guarded_ast =
        zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
            &ast,
            "guarded",
            Vec::new(),
            None,
            Some(&no_grants),
            Some(&policy),
            host_services,
        );
    assert!(guarded_vm.output.is_empty());
    assert_eq!(guarded_vm.output, guarded_ast.output);
    assert!(guarded_vm
        .error
        .as_ref()
        .is_some_and(|e| e.message.contains("Network")));
    assert!(guarded_ast
        .error
        .as_ref()
        .is_some_and(|e| e.message.contains("Network")));
    assert_eq!(host.requests.load(Ordering::SeqCst), calls_before);
}

#[test]
fn bytecode_filesystem_reads_and_metadata_match_ast_and_enforce_roots() {
    static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "zelyra-bytecode-read-{}-{}",
        std::process::id(),
        NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&directory).expect("create isolated test directory");
    let project_root = directory.join("project");
    std::fs::create_dir(&project_root).expect("create isolated project root");
    let output_root = project_root.join("output");
    std::fs::create_dir(&output_root).expect("create isolated write root");
    std::fs::write(project_root.join("input.txt"), "local fixture\n").expect("write local fixture");
    std::fs::write(output_root.join("delete-me.txt"), "delete fixture\n")
        .expect("write deletion fixture");
    std::fs::write(output_root.join("atomic.txt"), "old atomic content")
        .expect("write atomic fixture");
    std::fs::write(output_root.join("append.txt"), "initial").expect("write append fixture");
    std::fs::write(directory.join("outside.txt"), "outside fixture\n")
        .expect("write outside fixture");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            directory.join("outside.txt"),
            project_root.join("escape.txt"),
        )
        .expect("create read-root symlink escape");
        std::os::unix::fs::symlink(
            directory.join("outside.txt"),
            output_root.join("escape.txt"),
        )
        .expect("create write-root symlink escape");
    }
    let canonical_root = std::fs::canonicalize(&project_root).expect("canonical test root");
    let canonical_output = std::fs::canonicalize(&output_root).expect("canonical write root");
    let policy = RuntimePolicy {
        filesystem: Some(FileSystemPolicy {
            base_dir: canonical_root.clone(),
            read_roots: vec![canonical_root],
            write_roots: vec![canonical_output],
        }),
        ..RuntimePolicy::default()
    };
    let grants = HashSet::from(["FileSystem".to_owned()]);
    let source = r#"
fn read() -> String uses FileSystem { return read_text("input.txt") }
fn read_result(path: String) -> Result<String, String> uses FileSystem { return read_text_result(path) }
fn file_exists_probe() -> Bool uses FileSystem { return file_exists("input.txt") }
fn dir_exists_probe() -> Bool uses FileSystem { return dir_exists(".") }
fn file_size_probe() -> Result<Int, String> uses FileSystem { return file_size("input.txt") }
fn list_dir_probe(path: String) -> String[] uses FileSystem { return list_dir(path) }
fn path_with_side_effect() -> String { print("path evaluated"); return "output" }
fn list_dir_preflight_probe() -> String[] uses FileSystem { return list_dir(path_with_side_effect()) }
fn delete_preflight_probe() uses FileSystem { delete_file(path_with_side_effect()) }
fn outside_exists_probe() -> Bool uses FileSystem { return file_exists("../outside.txt") }
fn write_probe(path: String, content: String) uses FileSystem { write_text(path, content) }
fn write_result_probe(path: String, content: String) -> Result<Unit, String> uses FileSystem { return write_text_result(path, content) }
fn write_new_probe(path: String, content: String) -> Result<Unit, String> uses FileSystem { return write_text_new_result(path, content) }
fn atomic_write_probe(path: String, content: String) -> Result<Unit, String> uses FileSystem { return atomic_write_text_result(path, content) }
fn append_probe(path: String, content: String) -> Result<Unit, String> uses FileSystem { return append_text_result(path, content) }
fn copy_probe(src_path: String, dest_path: String) -> Result<Unit, String> uses FileSystem { return copy_file_new_result(src_path, dest_path) }
fn rename_probe(src_path: String, dest_path: String) -> Result<Unit, String> uses FileSystem { return rename_file_result(src_path, dest_path) }
fn delete_probe(path: String) uses FileSystem { delete_file(path) }
fn delete_result_probe(path: String) -> Result<Unit, String> uses FileSystem { return delete_file_result(path) }
fn read_symlink_probe() -> Result<String, String> uses FileSystem { return read_text_result("escape.txt") }
fn write_symlink_probe() -> Result<Unit, String> uses FileSystem { return write_text_result("output/escape.txt", "must not escape") }
fn main() {}
"#;
    let (ast, mir, bytecode) = {
        let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
        let mir = crate::lower(&ast).expect("source type-checks and lowers");
        (
            ast,
            mir.clone(),
            compile(&mir, "read").expect("file-read entry compiles"),
        )
    };
    let encoded = encode(&bytecode).expect("filesystem capability encodes");
    let bytecode = decode(&encoded).expect("filesystem capability verifies on load");
    let host: Arc<dyn HostServices> = Arc::new(SystemHostServices);
    let bytecode_result = execute_with_host_services_and_policy_captured(
        &bytecode,
        Vec::new(),
        ExecutionLimits::default(),
        Some(&grants),
        Some(&policy),
        host.clone(),
    );
    let ast_result =
        zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
            &ast,
            "read",
            Vec::new(),
            None,
            Some(&grants),
            Some(&policy),
            host.clone(),
        );
    assert_eq!(bytecode_result.error, None);
    assert_eq!(bytecode_result.value, ast_result.value);
    assert_eq!(
        bytecode_result.value,
        Some(Value::String("local fixture\n".into()))
    );
    for (entry, expected) in [
        ("file_exists_probe", Value::Bool(true)),
        ("dir_exists_probe", Value::Bool(true)),
        (
            "file_size_probe",
            Value::Result(Ok(Box::new(Value::Int(14)))),
        ),
        ("outside_exists_probe", Value::Bool(false)),
    ] {
        let program = compile(&mir, entry).expect("filesystem metadata entry compiles");
        let program = decode(&encode(&program).unwrap()).unwrap();
        let vm_result = execute_with_host_services_and_policy_captured(
            &program,
            Vec::new(),
            ExecutionLimits::default(),
            Some(&grants),
            Some(&policy),
            host.clone(),
        );
        let ast_reference = zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
                &ast,
                entry,
                Vec::new(),
                None,
                Some(&grants),
                Some(&policy),
                host.clone(),
            );
        assert_eq!(vm_result.error, None, "{entry}");
        assert_eq!(vm_result.value, ast_reference.value, "{entry}");
        assert_eq!(vm_result.value, Some(expected), "{entry}");
    }
    for (entry, arguments, expected) in [
        (
            "write_probe",
            vec![
                Value::String("output/vm.txt".into()),
                Value::String("written by bytecode".into()),
            ],
            Value::Unit,
        ),
        (
            "write_result_probe",
            vec![
                Value::String("output/result.txt".into()),
                Value::String("result write".into()),
            ],
            Value::Result(Ok(Box::new(Value::Unit))),
        ),
        (
            "write_result_probe",
            vec![
                Value::String("input.txt".into()),
                Value::String("must stay denied".into()),
            ],
            Value::Result(Err(Box::new(Value::String(
                "file-system write denied for input.txt: path is outside the configured roots"
                    .into(),
            )))),
        ),
    ] {
        let bytecode_program = decode(&encode(&compile(&mir, entry).unwrap()).unwrap()).unwrap();
        let vm_result = execute_with_host_services_and_policy_captured(
            &bytecode_program,
            arguments.clone(),
            ExecutionLimits::default(),
            Some(&grants),
            Some(&policy),
            host.clone(),
        );
        let ast_reference = zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
                &ast,
                entry,
                arguments,
                None,
                Some(&grants),
                Some(&policy),
                host.clone(),
            );
        assert_eq!(vm_result.error, None, "{entry}");
        assert_eq!(vm_result.value, ast_reference.value, "{entry}");
        assert_eq!(vm_result.value, Some(expected), "{entry}");
    }
    assert_eq!(
        std::fs::read_to_string(output_root.join("vm.txt")).unwrap(),
        "written by bytecode"
    );
    assert_eq!(
        std::fs::read_to_string(output_root.join("result.txt")).unwrap(),
        "result write"
    );
    for (entry, path, content, initial) in [
        ("write_new_probe", "output/new.txt", "created", None),
        (
            "atomic_write_probe",
            "output/atomic.txt",
            "replacement",
            Some("old atomic content"),
        ),
        (
            "append_probe",
            "output/append.txt",
            "-appended",
            Some("initial"),
        ),
    ] {
        let target = project_root.join(path);
        match initial {
            Some(initial) => std::fs::write(&target, initial).unwrap(),
            None => {
                let _ = std::fs::remove_file(&target);
            }
        }
        let bytecode_program = decode(&encode(&compile(&mir, entry).unwrap()).unwrap()).unwrap();
        let arguments = vec![Value::String(path.into()), Value::String(content.into())];
        let vm_result = execute_with_host_services_and_policy_captured(
            &bytecode_program,
            arguments.clone(),
            ExecutionLimits::default(),
            Some(&grants),
            Some(&policy),
            host.clone(),
        );
        match initial {
            Some(initial) => std::fs::write(&target, initial).unwrap(),
            None => {
                let _ = std::fs::remove_file(&target);
            }
        }
        let ast_reference = zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
                &ast,
                entry,
                arguments,
                None,
                Some(&grants),
                Some(&policy),
                host.clone(),
            );
        assert_eq!(vm_result.error, ast_reference.error, "{entry}");
        assert_eq!(vm_result.value, ast_reference.value, "{entry}");
        assert_eq!(
            vm_result.value,
            Some(Value::Result(Ok(Box::new(Value::Unit)))),
            "{entry}"
        );
    }
    let collision_path = project_root.join("output/new-existing.txt");
    std::fs::write(&collision_path, "keep existing content").unwrap();
    let new_write_program =
        decode(&encode(&compile(&mir, "write_new_probe").unwrap()).unwrap()).unwrap();
    let collision_arguments = vec![
        Value::String("output/new-existing.txt".into()),
        Value::String("must not replace".into()),
    ];
    let vm_collision = execute_with_host_services_and_policy_captured(
        &new_write_program,
        collision_arguments.clone(),
        ExecutionLimits::default(),
        Some(&grants),
        Some(&policy),
        host.clone(),
    );
    std::fs::write(&collision_path, "keep existing content").unwrap();
    let ast_collision =
        zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
            &ast,
            "write_new_probe",
            collision_arguments,
            None,
            Some(&grants),
            Some(&policy),
            host.clone(),
        );
    assert_eq!(vm_collision.error, ast_collision.error);
    assert_eq!(vm_collision.value, ast_collision.value);
    assert!(matches!(vm_collision.value, Some(Value::Result(Err(_)))));
    assert_eq!(
        std::fs::read_to_string(collision_path).unwrap(),
        "keep existing content"
    );
    assert_eq!(
        std::fs::read_to_string(output_root.join("new.txt")).unwrap(),
        "created"
    );
    assert_eq!(
        std::fs::read_to_string(output_root.join("atomic.txt")).unwrap(),
        "replacement"
    );
    assert_eq!(
        std::fs::read_to_string(output_root.join("append.txt")).unwrap(),
        "initial-appended"
    );
    let copy_arguments = vec![
        Value::String("input.txt".into()),
        Value::String("output/copied.txt".into()),
    ];
    let copy_program = decode(&encode(&compile(&mir, "copy_probe").unwrap()).unwrap()).unwrap();
    let _ = std::fs::remove_file(output_root.join("copied.txt"));
    let copy_vm = execute_with_host_services_and_policy_captured(
        &copy_program,
        copy_arguments.clone(),
        ExecutionLimits::default(),
        Some(&grants),
        Some(&policy),
        host.clone(),
    );
    let _ = std::fs::remove_file(output_root.join("copied.txt"));
    let copy_ast =
        zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
            &ast,
            "copy_probe",
            copy_arguments,
            None,
            Some(&grants),
            Some(&policy),
            host.clone(),
        );
    assert_eq!(copy_vm.error, copy_ast.error);
    assert_eq!(copy_vm.value, copy_ast.value);
    assert_eq!(
        copy_vm.value,
        Some(Value::Result(Ok(Box::new(Value::Unit))))
    );
    assert_eq!(
        std::fs::read_to_string(output_root.join("copied.txt")).unwrap(),
        "local fixture\n"
    );

    let copy_source_denied_args = vec![
        Value::String("../outside.txt".into()),
        Value::String("output/denied-copy.txt".into()),
    ];
    for entry_args in [
        ("copy_probe", copy_source_denied_args),
        (
            "copy_probe",
            vec![
                Value::String("input.txt".into()),
                Value::String("../denied-copy.txt".into()),
            ],
        ),
        (
            "rename_probe",
            vec![
                Value::String("input.txt".into()),
                Value::String("output/denied-rename.txt".into()),
            ],
        ),
    ] {
        let program = decode(&encode(&compile(&mir, entry_args.0).unwrap()).unwrap()).unwrap();
        let vm_result = execute_with_host_services_and_policy_captured(
            &program,
            entry_args.1.clone(),
            ExecutionLimits::default(),
            Some(&grants),
            Some(&policy),
            host.clone(),
        );
        let ast_reference = zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
                &ast,
                entry_args.0,
                entry_args.1,
                None,
                Some(&grants),
                Some(&policy),
                host.clone(),
            );
        assert_eq!(vm_result.error, ast_reference.error, "{}", entry_args.0);
        assert_eq!(vm_result.value, ast_reference.value, "{}", entry_args.0);
        assert!(matches!(vm_result.value, Some(Value::Result(Err(_)))));
    }

    let rename_source = output_root.join("rename-me.txt");
    let rename_destination = output_root.join("renamed.txt");
    let rename_arguments = vec![
        Value::String("output/rename-me.txt".into()),
        Value::String("output/renamed.txt".into()),
    ];
    let rename_program = decode(&encode(&compile(&mir, "rename_probe").unwrap()).unwrap()).unwrap();
    std::fs::write(&rename_source, "rename fixture").unwrap();
    let _ = std::fs::remove_file(&rename_destination);
    let rename_vm = execute_with_host_services_and_policy_captured(
        &rename_program,
        rename_arguments.clone(),
        ExecutionLimits::default(),
        Some(&grants),
        Some(&policy),
        host.clone(),
    );
    std::fs::write(&rename_source, "rename fixture").unwrap();
    let _ = std::fs::remove_file(&rename_destination);
    let rename_ast =
        zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
            &ast,
            "rename_probe",
            rename_arguments,
            None,
            Some(&grants),
            Some(&policy),
            host.clone(),
        );
    assert_eq!(rename_vm.error, rename_ast.error);
    assert_eq!(rename_vm.value, rename_ast.value);
    assert_eq!(
        rename_vm.value,
        Some(Value::Result(Ok(Box::new(Value::Unit))))
    );
    assert!(!rename_source.exists());
    assert_eq!(
        std::fs::read_to_string(rename_destination).unwrap(),
        "rename fixture"
    );
    let mut expected_output_entries = vec![
        "delete-me.txt".to_owned(),
        "atomic.txt".to_owned(),
        "append.txt".to_owned(),
        "new-existing.txt".to_owned(),
        "new.txt".to_owned(),
        "copied.txt".to_owned(),
        "renamed.txt".to_owned(),
        "result.txt".to_owned(),
        "vm.txt".to_owned(),
    ];
    #[cfg(unix)]
    expected_output_entries.push("escape.txt".to_owned());
    expected_output_entries.sort();
    for path in ["output", "../outside.txt"] {
        let program = decode(&encode(&compile(&mir, "list_dir_probe").unwrap()).unwrap()).unwrap();
        let arguments = vec![Value::String(path.to_owned())];
        let vm_result = execute_with_host_services_and_policy_captured(
            &program,
            arguments.clone(),
            ExecutionLimits::default(),
            Some(&grants),
            Some(&policy),
            host.clone(),
        );
        let ast_reference = zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
                &ast,
                "list_dir_probe",
                arguments,
                None,
                Some(&grants),
                Some(&policy),
                host.clone(),
            );
        assert_eq!(vm_result.error, ast_reference.error, "{path}");
        assert_eq!(vm_result.value, ast_reference.value, "{path}");
        if path == "output" {
            assert_eq!(
                vm_result.value,
                Some(Value::Array(
                    expected_output_entries
                        .iter()
                        .cloned()
                        .map(Value::String)
                        .collect()
                ))
            );
        } else {
            assert!(vm_result.error.is_some(), "outside path must be denied");
        }
    }
    let preflight_program =
        decode(&encode(&compile(&mir, "list_dir_preflight_probe").unwrap()).unwrap()).unwrap();
    let no_grants = HashSet::new();
    let preflight_result = execute_with_host_services_and_policy_captured(
        &preflight_program,
        Vec::new(),
        ExecutionLimits::default(),
        Some(&no_grants),
        Some(&policy),
        host.clone(),
    );
    let preflight_ast =
        zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
            &ast,
            "list_dir_preflight_probe",
            Vec::new(),
            None,
            Some(&no_grants),
            Some(&policy),
            host.clone(),
        );
    assert!(preflight_result.output.is_empty());
    assert!(preflight_ast.output.is_empty());
    assert!(preflight_result
        .error
        .is_some_and(|error| error.message.contains("FileSystem")));
    assert!(preflight_ast
        .error
        .is_some_and(|error| error.message.contains("FileSystem")));
    let delete_preflight_program =
        decode(&encode(&compile(&mir, "delete_preflight_probe").unwrap()).unwrap()).unwrap();
    let delete_preflight_result = execute_with_host_services_and_policy_captured(
        &delete_preflight_program,
        Vec::new(),
        ExecutionLimits::default(),
        Some(&no_grants),
        Some(&policy),
        host.clone(),
    );
    assert!(delete_preflight_result.output.is_empty());
    assert!(delete_preflight_result
        .error
        .is_some_and(|error| error.message.contains("FileSystem")));
    let mut deletion_cases = vec![
            ("output/delete-me.txt", Some(Value::Unit)),
            (
                "../outside.txt",
                Some(Value::Result(Err(Box::new(Value::String(
                    "file-system delete denied for ../outside.txt: path is outside the configured roots"
                        .into(),
                ))))),
            ),
        ];
    #[cfg(unix)]
    deletion_cases.push((
        "output/escape.txt",
        Some(Value::Result(Err(Box::new(Value::String(
            "file-system delete denied for output/escape.txt: path is outside the configured roots"
                .into(),
        ))))),
    ));
    for (path, expected_result) in deletion_cases {
        let is_successful_delete = expected_result == Some(Value::Unit);
        if is_successful_delete {
            std::fs::write(output_root.join("delete-me.txt"), "delete fixture\n")
                .expect("reset deletion fixture before VM");
        }
        let program = decode(
            &encode(
                &compile(
                    &mir,
                    if is_successful_delete {
                        "delete_probe"
                    } else {
                        "delete_result_probe"
                    },
                )
                .unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        let entry = if is_successful_delete {
            "delete_probe"
        } else {
            "delete_result_probe"
        };
        let arguments = vec![Value::String(path.to_owned())];
        let vm_result = execute_with_host_services_and_policy_captured(
            &program,
            arguments.clone(),
            ExecutionLimits::default(),
            Some(&grants),
            Some(&policy),
            host.clone(),
        );
        if is_successful_delete {
            std::fs::write(output_root.join("delete-me.txt"), "delete fixture\n")
                .expect("reset deletion fixture before AST");
        }
        let ast_reference = zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
                &ast,
                entry,
                arguments,
                None,
                Some(&grants),
                Some(&policy),
                host.clone(),
            );
        assert_eq!(vm_result.error, ast_reference.error, "{path}");
        assert_eq!(vm_result.value, ast_reference.value, "{path}");
        assert_eq!(vm_result.value, expected_result, "{path}");
    }
    assert!(!output_root.join("delete-me.txt").exists());
    assert!(directory.join("outside.txt").exists());
    #[cfg(unix)]
    assert!(output_root.join("escape.txt").symlink_metadata().is_ok());
    #[cfg(unix)]
    for (entry, expected_type) in [
        (
            "read_symlink_probe",
            Type::Result(Box::new(Type::String), Box::new(Type::String)),
        ),
        (
            "write_symlink_probe",
            Type::Result(Box::new(Type::Unit), Box::new(Type::String)),
        ),
    ] {
        let program = decode(&encode(&compile(&mir, entry).unwrap()).unwrap()).unwrap();
        let vm_result = execute_with_host_services_and_policy_captured(
            &program,
            Vec::new(),
            ExecutionLimits::default(),
            Some(&grants),
            Some(&policy),
            host.clone(),
        );
        let ast_reference = zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
                &ast,
                entry,
                Vec::new(),
                None,
                Some(&grants),
                Some(&policy),
                host.clone(),
            );
        assert_eq!(vm_result.error, None, "{entry}");
        assert_eq!(vm_result.value, ast_reference.value, "{entry}");
        assert!(
            matches!(
                vm_result.value,
                Some(Value::Result(Err(error))) if matches!(&*error, Value::String(message) if message.contains("outside the configured roots"))
            ),
            "unexpected `{entry}` result, expected {expected_type:?}"
        );
    }
    assert_eq!(
        std::fs::read_to_string(directory.join("outside.txt")).unwrap(),
        "outside fixture\n",
        "symlink writes must not escape the configured roots"
    );

    let outside_source = r#"
fn main(path: String) -> Result<String, String> uses FileSystem {
    return read_text_result(path)
}
"#;
    let outside_ast = parse(&lex(outside_source).unwrap()).unwrap();
    let outside_mir = crate::lower(&outside_ast).unwrap();
    let outside_bytecode = compile(&outside_mir, "main").unwrap();
    let outside_arguments = vec![Value::String("../outside.txt".into())];
    let outside = execute_with_host_services_and_policy_captured(
        &outside_bytecode,
        outside_arguments.clone(),
        ExecutionLimits::default(),
        Some(&grants),
        Some(&policy),
        host.clone(),
    );
    let outside_reference =
        zelyra_runtime::execute_function_captured_with_host_services_and_capabilities_and_policies(
            &outside_ast,
            "main",
            outside_arguments,
            None,
            Some(&grants),
            Some(&policy),
            host,
        );
    assert_eq!(outside.error, None);
    assert_eq!(outside.value, outside_reference.value);
    assert!(matches!(
        outside.value,
        Some(Value::Result(Err(error))) if matches!(&*error, Value::String(message) if message.contains("outside the configured roots"))
    ));
    std::fs::remove_dir_all(directory).expect("remove isolated test directory");
}

#[test]
fn bytecode_denies_file_read_before_evaluating_its_path() {
    let source = r#"
fn path() -> String uses Console { print("path evaluated"); return "input.txt" }
fn main() -> String uses Console, FileSystem { return read_text(path()) }
"#;
    let (_, bytecode) = compile_source(source, "main");
    let outcome = execute_with_host_services_captured(
        &bytecode,
        Vec::new(),
        ExecutionLimits::default(),
        Some(&HashSet::from(["Console".to_owned()])),
        Arc::new(SystemHostServices),
    );
    assert!(outcome
        .error
        .as_ref()
        .is_some_and(|error| error.message.contains("FileSystem")));
    assert!(outcome.output.is_empty());
}

#[test]
fn bytecode_denies_file_write_before_evaluating_either_argument() {
    let source = r#"
fn path() -> String { print("path evaluated"); return "output.txt" }
fn content() -> String { print("content evaluated"); return "secret" }
fn main() -> Result<Unit, String> uses FileSystem {
    return write_text_result(path(), content())
}
"#;
    let (_, bytecode) = compile_source(source, "main");
    let outcome = execute_with_host_services_captured(
        &bytecode,
        Vec::new(),
        ExecutionLimits::default(),
        None,
        Arc::new(SystemHostServices),
    );
    assert!(outcome
        .error
        .as_ref()
        .is_some_and(|error| error.message.contains("FileSystem")));
    assert!(outcome.output.is_empty());
}

#[test]
fn bytecode_constructs_records_and_reads_typed_fields_with_ast_parity() {
    let source = r#"
struct Customer { name: String age: Int }
fn make_label(name: String, age: Int) -> String {
    customer = Customer { name: name, age: age }
    return customer.name + " #" + to_string(customer.age)
}
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source type-checks and lowers");
    let bytecode = compile(&mir, "make_label").expect("records lower to bytecode");
    let arguments = vec![Value::String("Ada".into()), Value::Int(42)];
    let expected = zelyra_runtime::execute_function(&ast, "make_label", arguments.clone(), None)
        .expect("AST interpreter executes record path");
    let actual = execute(&bytecode, arguments, ExecutionLimits::default())
        .expect("bytecode executes record path");
    assert_eq!(actual.value, expected);
    assert_eq!(actual.value, Value::String("Ada #42".into()));

    let round_trip = decode(&encode(&bytecode).expect("record bytecode encodes"))
        .expect("record bytecode decodes");
    let decoded = execute(
        &round_trip,
        vec![Value::String("Ada".into()), Value::Int(42)],
        ExecutionLimits::default(),
    )
    .expect("decoded record bytecode executes");
    assert_eq!(decoded.value, expected);

    let mut mismatched_record = bytecode;
    let record_type = mismatched_record.functions[0]
        .code
        .iter_mut()
        .find_map(|instruction| match &mut instruction.operation {
            BytecodeOp::BuildRecord { record_type, .. } => Some(record_type),
            _ => None,
        })
        .expect("record construction instruction exists");
    *record_type = Type::Named("OtherRecord".into());
    let error = execute(
        &mismatched_record,
        vec![Value::String("Ada".into()), Value::Int(42)],
        ExecutionLimits::default(),
    )
    .expect_err("field access rejects an unexpected record type");
    assert!(error
        .message
        .contains("expected `Customer`, got `OtherRecord`"));

    let generic_source = r#"
struct Box<T> { value: T }
fn explicit_box(value: Int) -> Int {
    boxed: Box<Int> = Box<Int> { value: value }
    return boxed.value
}
fn inferred_box(value: String) -> String {
    boxed = Box { value: value }
    return boxed.value
}
fn main() {}
"#;
    let generic_ast =
        parse(&lex(generic_source).expect("generic source lexes")).expect("generic source parses");
    let generic_mir = crate::lower(&generic_ast).expect("generic source lowers");
    for (entry, argument, expected) in [
        ("explicit_box", Value::Int(73), Value::Int(73)),
        (
            "inferred_box",
            Value::String("inferred".into()),
            Value::String("inferred".into()),
        ),
    ] {
        let generic_bytecode =
            compile(&generic_mir, entry).expect("generic records lower to bytecode");
        let reference =
            zelyra_runtime::execute_function(&generic_ast, entry, vec![argument.clone()], None)
                .expect("AST interpreter executes generic record");
        let actual = execute(
            &generic_bytecode,
            vec![argument],
            ExecutionLimits::default(),
        )
        .expect("bytecode executes generic record");
        assert_eq!(actual.value, reference);
        assert_eq!(actual.value, expected);
    }
}

#[test]
fn bytecode_lowers_record_copy_updates_with_ast_and_mir_parity() {
    let source = r#"
struct Customer { name: String age: Int active: Bool }
struct Box<T> { value: T caption: String }
fn update_customer(customer: Customer, age: Int) -> Customer {
    return Customer { ..customer, age: age }
}
fn copy_customer(customer: Customer) -> Customer {
    return Customer { ..customer }
}
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source type-checks and lowers");
    let bytecode =
        compile(&mir, "update_customer").expect("typed record update lowers to bytecode");
    let customer = Value::Object {
        type_name: "Customer".into(),
        type_args: vec![],
        fields: HashMap::from([
            ("name".into(), Value::String("Ada".into())),
            ("age".into(), Value::Int(37)),
            ("active".into(), Value::Bool(true)),
        ]),
    };
    let arguments = vec![customer, Value::Int(38)];
    let ast_value =
        zelyra_runtime::execute_function(&ast, "update_customer", arguments.clone(), None)
            .expect("AST executes record update");
    let mir_value = execution::execute_function(
        &mir,
        "update_customer",
        arguments.clone(),
        ExecutionLimits::default(),
    )
    .expect("MIR executes record update");
    let encoded = encode(&bytecode).expect("record-update bytecode encodes");
    let decoded = decode(&encoded).expect("record-update bytecode decodes");
    let bytecode_value = execute(&decoded, arguments, ExecutionLimits::default())
        .expect("decoded bytecode executes record update");

    assert_eq!(ast_value, mir_value.value);
    assert_eq!(bytecode_value.value, ast_value);
    assert!(matches!(
        ast_value,
        Value::Object { ref fields, .. }
            if fields.get("name") == Some(&Value::String("Ada".into()))
                && fields.get("age") == Some(&Value::Int(38))
                && fields.get("active") == Some(&Value::Bool(true))
    ));

    let mut forged = decoded.clone();
    let update = forged.functions[0]
        .code
        .iter_mut()
        .find_map(|instruction| match &mut instruction.operation {
            BytecodeOp::RecordUpdate { fields, .. } => fields.first_mut(),
            _ => None,
        })
        .expect("record update instruction exists");
    update.1 = Type::String;
    let invalid_base_type = Value::Object {
        type_name: "Customer".into(),
        type_args: vec![],
        fields: HashMap::from([
            ("name".into(), Value::String("Ada".into())),
            ("age".into(), Value::Int(37)),
            ("active".into(), Value::Bool(true)),
        ]),
    };
    let error = execute(
        &forged,
        vec![invalid_base_type, Value::Int(38)],
        ExecutionLimits::default(),
    )
    .expect_err("runtime rejects forged field metadata against the base value");
    assert!(error.message.contains("invalid base type"));

    let customer = Value::Object {
        type_name: "Customer".into(),
        type_args: vec![],
        fields: HashMap::from([
            ("name".into(), Value::String("Ada".into())),
            ("age".into(), Value::Int(37)),
            ("active".into(), Value::Bool(true)),
        ]),
    };
    let copied = execute(
        &decode(&encode(&compile(&mir, "copy_customer").unwrap()).unwrap()).unwrap(),
        vec![customer.clone()],
        ExecutionLimits::default(),
    )
    .expect("record spread without overrides copies the record");
    assert_eq!(copied.value, customer);

    let generic_source = r#"
struct Box<T> { value: T caption: String }
fn update_box(item: Box<Int>, caption: String) -> Box<Int> {
    return Box<Int> { ..item, caption: caption }
}
fn main() {}
"#;
    let generic_ast = parse(&lex(generic_source).unwrap()).unwrap();
    let generic_mir = crate::lower(&generic_ast).unwrap();
    let generic_code =
        decode(&encode(&compile(&generic_mir, "update_box").unwrap()).unwrap()).unwrap();
    let generic_item = Value::Object {
        type_name: "Box".into(),
        type_args: vec![Type::Int],
        fields: HashMap::from([
            ("value".into(), Value::Int(7)),
            ("caption".into(), Value::String("old".into())),
        ]),
    };
    let generic_args = vec![generic_item, Value::String("new".into())];
    let generic_expected =
        zelyra_runtime::execute_function(&generic_ast, "update_box", generic_args.clone(), None)
            .unwrap();
    let generic_mir_value = execution::execute_function(
        &generic_mir,
        "update_box",
        generic_args.clone(),
        ExecutionLimits::default(),
    )
    .unwrap();
    let generic_actual = execute(&generic_code, generic_args, ExecutionLimits::default()).unwrap();
    assert_eq!(generic_mir_value.value, generic_expected);
    assert_eq!(generic_actual.value, generic_expected);
}

#[test]
fn record_copy_update_rejects_fields_with_mismatched_types() {
    let source = r#"
struct Customer { name: String age: Int }
fn invalid(customer: Customer) -> Customer {
    return Customer { ..customer, age: "thirty" }
}
fn main() {}
"#;
    let ast = parse(&lex(source).unwrap()).unwrap();
    let errors = crate::lower(&ast).expect_err("mismatched update must fail type checking");
    assert!(errors.iter().any(|error| error.code == "E-TYPE-001"));
}

#[test]
fn bytecode_iterates_record_arrays_and_reads_fields_with_ast_parity() {
    let source = r#"
struct InvoiceLine { total: Int }
fn sum_lines() -> Int {
    lines = [InvoiceLine { total: 1200 }, InvoiceLine { total: 3400 }]
    mutable result = 0
    for line in lines { result = result + line.total }
    return result
}
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source type-checks and lowers");
    let bytecode = compile(&mir, "sum_lines").expect("record array iteration lowers");
    let expected = zelyra_runtime::execute_function(&ast, "sum_lines", vec![], None)
        .expect("AST interpreter executes record-array loop");
    assert_eq!(expected, Value::Int(4600));
    let encoded = encode(&bytecode).expect("record-array bytecode encodes");
    let decoded = decode(&encoded).expect("record-array bytecode decodes");
    let actual = execute(&decoded, vec![], ExecutionLimits::default())
        .expect("bytecode executes record-array loop");
    assert_eq!(actual.value, expected);
}

#[test]
fn bytecode_executes_enum_match_statements_with_guards_and_roundtrips() {
    let source = "enum Outcome { Draft Paid(Int) } fn main() -> Int { outcome = Outcome::Paid(7) match outcome { Outcome::Draft => { return 0 } Outcome::Paid(value) if value > 4 => { return value } Outcome::Paid(_) => { return 1 } } }";
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source type-checks and lowers");
    let bytecode = compile(&mir, "main").expect("enum match compiles");
    let ast_value = zelyra_runtime::execute_function(&ast, "main", vec![], None)
        .expect("AST interpreter executes enum match");
    let mir_value = execution::execute_function(&mir, "main", vec![], ExecutionLimits::default())
        .expect("MIR interpreter executes enum match");
    let decoded = decode(&encode(&bytecode).expect("enum match bytecode encodes"))
        .expect("enum match bytecode decodes");
    let bytecode_value = execute(&decoded, vec![], ExecutionLimits::default())
        .expect("bytecode executes enum match");
    assert_eq!(ast_value, Value::Int(7));
    assert_eq!(mir_value.value, ast_value);
    assert_eq!(bytecode_value.value, ast_value);
}

#[test]
fn bytecode_executes_enum_match_expressions_with_guards_and_roundtrips() {
    let source = "enum Outcome { Draft Paid(Int) } fn main() -> Int { outcome = Outcome::Paid(7) return match outcome { Outcome::Draft => 0 Outcome::Paid(value) if value > 4 => value Outcome::Paid(_) => 1 } } fn guard_fallback() -> Int { outcome = Outcome::Paid(2) return match outcome { Outcome::Draft => 0 Outcome::Paid(value) if value > 4 => value Outcome::Paid(_) => 1 } }";
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source type-checks and lowers");
    for (entry, expected) in [("main", 7), ("guard_fallback", 1)] {
        let bytecode = compile(&mir, entry).expect("enum match expression compiles");
        let ast_value = zelyra_runtime::execute_function(&ast, entry, vec![], None)
            .expect("AST interpreter executes enum match expression");
        let mir_value =
            execution::execute_function(&mir, entry, vec![], ExecutionLimits::default())
                .expect("MIR interpreter executes enum match expression");
        let decoded = decode(&encode(&bytecode).expect("enum match bytecode encodes"))
            .expect("enum match bytecode decodes");
        let bytecode_value = execute(&decoded, vec![], ExecutionLimits::default())
            .expect("bytecode executes enum match expression");
        assert_eq!(ast_value, Value::Int(expected));
        assert_eq!(mir_value.value, ast_value);
        assert_eq!(bytecode_value.value, ast_value);
    }
}

#[test]
fn bytecode_executes_option_result_and_record_match_patterns() {
    let source = r#"
struct Order { status: String total: Int }
enum Notice { Count(Option<Int>) }
fn option_value(value: Option<Int>) -> Int {
    return match value { Some(found) => found None => 0 }
}
fn result_value(value: Result<Int, String>) -> Int {
    return match value { Ok(found) => found Err(_) => 0 }
}
fn record_value() -> Int {
    order = Order { status: "paid", total: 42 }
    return match order {
        Order { status: "paid", total } if total > 50 => total
        Order { status: "paid", total } => total
        _ => 0
    }
}
fn nested_value() -> Int {
    notice = Notice::Count(Some(3))
    return match notice {
        Notice::Count(Some(value)) => value
        Notice::Count(None) => 0
    }
}
fn main() -> Int {
    return option_value(Some(7)) + option_value(None)
        + result_value(Ok(5)) + result_value(Err("invalid"))
        + record_value() + nested_value()
}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source type-checks and lowers");
    let bytecode = compile(&mir, "main").expect("pattern matching compiles");
    let ast_value = zelyra_runtime::execute_function(&ast, "main", vec![], None)
        .expect("AST interpreter executes patterns");
    let mir_value = execution::execute_function(&mir, "main", vec![], ExecutionLimits::default())
        .expect("MIR interpreter executes patterns");
    let decoded = decode(&encode(&bytecode).expect("pattern bytecode encodes"))
        .expect("pattern bytecode decodes");
    let bytecode_value =
        execute(&decoded, vec![], ExecutionLimits::default()).expect("bytecode executes patterns");
    assert_eq!(ast_value, Value::Int(57));
    assert_eq!(mir_value.value, ast_value);
    assert_eq!(bytecode_value.value, ast_value);
}

#[test]
fn bytecode_builds_and_looks_up_typed_maps_with_ast_and_mir_parity() {
    let source = r#"
fn read_duplicate() -> Option<Int> {
    values: Map<String, Int> = Map {
        "x": 1
        "y": 2
        "x": 3
    }
    return get(values, "x")
}
fn read_missing() -> Option<Int> {
    values: Map<String, Int> = Map { "x": 1 }
    return get(values, "missing")
}
fn empty_count() -> Int {
    values: Map<String, Int> = Map {}
    return len(values)
}
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source type-checks and lowers");
    for (entry, expected) in [
        (
            "read_duplicate",
            Value::Option(Some(Box::new(Value::Int(3)))),
        ),
        ("read_missing", Value::Option(None)),
        ("empty_count", Value::Int(0)),
    ] {
        let bytecode = compile(&mir, entry).expect("typed map path compiles");
        let ast_result = zelyra_runtime::execute_function(&ast, entry, vec![], None)
            .expect("AST interpreter executes map path");
        let mir_result =
            execution::execute_function(&mir, entry, vec![], ExecutionLimits::default())
                .expect("MIR interpreter executes map path");
        let decoded = decode(&encode(&bytecode).expect("map bytecode encodes"))
            .expect("map bytecode decodes");
        let bytecode_result = execute(&decoded, vec![], ExecutionLimits::default())
            .expect("bytecode executes map path");
        assert_eq!(ast_result, expected);
        assert_eq!(mir_result.value, ast_result);
        assert_eq!(bytecode_result.value, ast_result);
    }
}

#[test]
fn bytecode_iterates_arrays_and_sets_with_ast_parity() {
    let source = r#"
fn sum_array(values: Int[]) -> Int {
    mutable total = 0
    for value in values { total = total + value }
    return total
}
fn sum_set(values: Set<Int>) -> Int {
    mutable total = 0
    for value in values { total = total + value }
    return total
}
fn main() {}
"#;
    let (mir, array_bytecode) = compile_source(source, "sum_array");
    let set_bytecode = compile(&mir, "sum_set").expect("set iterator compiles");
    let array_bytecode = decode(&encode(&array_bytecode).unwrap()).unwrap();
    let set_bytecode = decode(&encode(&set_bytecode).unwrap()).unwrap();

    for values in [vec![], vec![1, 2, 3, 4], vec![-5, 9, 12]] {
        let arguments = vec![Value::Array(
            values.iter().copied().map(Value::Int).collect(),
        )];
        let expected = values.iter().sum::<i64>();
        let reference = execution::execute_function(
            &mir,
            "sum_array",
            arguments.clone(),
            ExecutionLimits::default(),
        )
        .unwrap();
        let actual = execute(&array_bytecode, arguments, ExecutionLimits::default()).unwrap();
        assert_eq!(actual.value, reference.value);
        assert_eq!(actual.value, Value::Int(expected));
    }

    for values in [vec![], vec![3, 1, 7], vec![-4, 6, 20]] {
        let arguments = vec![Value::Set(values.iter().copied().map(Value::Int).collect())];
        let expected = values.iter().sum::<i64>();
        let reference = execution::execute_function(
            &mir,
            "sum_set",
            arguments.clone(),
            ExecutionLimits::default(),
        )
        .unwrap();
        let actual = execute(&set_bytecode, arguments, ExecutionLimits::default()).unwrap();
        assert_eq!(actual.value, reference.value);
        assert_eq!(actual.value, Value::Int(expected));
    }

    let exhausted = execute(
        &array_bytecode,
        vec![Value::Array(vec![Value::Int(1), Value::Int(2)])],
        ExecutionLimits {
            max_steps: 2,
            max_stack_depth: 20,
        },
    )
    .expect_err("collection iteration must consume the step budget");
    assert!(exhausted.message.contains("step budget"));
}

#[test]
fn bytecode_destructures_tuple_bound_collection_iterations() {
    let source = r#"
fn weighted_sum(values: Tuple<Int, Int>[]) -> Int {
    mutable total = 0
    for (a, b) in values { total = total + a * b }
    return total
}
fn main() {}
"#;
    let (mir, bytecode) = compile_source(source, "weighted_sum");
    let bytecode = decode(&encode(&bytecode).expect("encode tuple loop")).unwrap();
    for (pairs, expected) in [
        (vec![], 0),
        (vec![(1, 4), (2, 5), (3, 6)], 32),
        (vec![(-2, 7), (5, 3), (9, -1)], -8),
    ] {
        let arguments = vec![Value::Array(
            pairs
                .into_iter()
                .map(|(a, b)| Value::Tuple(vec![Value::Int(a), Value::Int(b)]))
                .collect(),
        )];
        let reference = execution::execute_function(
            &mir,
            "weighted_sum",
            arguments.clone(),
            ExecutionLimits::default(),
        )
        .expect("AST reference execution");
        let actual = execute(&bytecode, arguments.clone(), ExecutionLimits::default())
            .expect("tuple-bound bytecode execution");
        assert_eq!(actual.value, reference.value);
        assert_eq!(actual.value, Value::Int(expected));
    }
}

#[test]
fn bytecode_tuple_bound_iteration_rejects_non_tuple_runtime_values() {
    let source = r#"
fn first_sum(values: Tuple<Int, Int>[]) -> Int {
    mutable total = 0
    for (left, right) in values { total = total + left + right }
    return total
}
fn main() {}
"#;
    let (_mir, mut bytecode) = compile_source(source, "first_sum");
    bytecode.functions[bytecode.entry].parameters[0].1 = Type::Array(Box::new(Type::Int));
    let error = execute(
        &bytecode,
        vec![Value::Array(vec![Value::Int(1)])],
        ExecutionLimits::default(),
    )
    .expect_err("malformed runtime input should fail closed");
    assert!(error.message.contains("requires a Tuple"));
}

#[test]
fn bytecode_constructs_and_indexes_arrays_and_tuples_with_ast_parity() {
    let source = r#"
fn pair(left: Int, right: Int) -> Tuple<Int, Int> {
    return (left, right)
}
fn tuple_sum(left: Int, right: Int) -> Int {
    pair_value = pair(left, right)
    return pair_value[0] + pair_value[1]
}
fn array_sum(left: Int, right: Int) -> Int {
    values = [left, right]
    return values[0] + values[1]
}
fn main() {}
"#;
    let (mir, tuple_code) = compile_source(source, "tuple_sum");
    let array_code = compile(&mir, "array_sum").expect("array entry compiles");
    let tuple_code = decode(&encode(&tuple_code).expect("encode tuple program")).unwrap();
    let array_code = decode(&encode(&array_code).expect("encode array program")).unwrap();

    for (entry, code) in [("tuple_sum", &tuple_code), ("array_sum", &array_code)] {
        for (left, right) in [(0, 0), (20, 22), (-17, 9)] {
            let arguments = vec![Value::Int(left), Value::Int(right)];
            let reference = execution::execute_function(
                &mir,
                entry,
                arguments.clone(),
                ExecutionLimits::default(),
            )
            .expect("AST reference execution");
            let actual =
                execute(code, arguments, ExecutionLimits::default()).expect("bytecode execution");
            assert_eq!(actual.value, reference.value);
            assert_eq!(actual.value, Value::Int(left + right));
        }
    }
}

#[test]
fn bytecode_collection_indexing_preserves_source_mapped_bounds_errors() {
    let source =
        "fn at(values: Int[], position: Int) -> Int { return values[position] } fn main() {}";
    let (mir, bytecode) = compile_source(source, "at");
    let arguments = vec![Value::Array(vec![Value::Int(4)]), Value::Int(1)];
    let expected =
        execution::execute_function(&mir, "at", arguments.clone(), ExecutionLimits::default())
            .unwrap_err();
    let actual = execute(&bytecode, arguments, ExecutionLimits::default()).unwrap_err();
    assert_eq!(actual.message, expected.message);
    assert_eq!(actual.span, expected.span);
}

#[test]
fn bytecode_executes_mutable_array_index_set_and_add_with_ast_parity() {
    let source = r#"
fn add_at(initial: Int, position: Int, amount: Int) -> Int {
    mutable values: Int[] = [initial, 2]
    values[position] += amount
    return values[0] + values[1]
}

fn set_at(initial: Int, position: Int, replacement: Int) -> Int {
    mutable values: Int[] = [initial, 2]
    values[position] = replacement
    return values[0] + values[1]
}
fn main() {}
"#;
    let (mir, add_code) = compile_source(source, "add_at");
    let set_code = compile(&mir, "set_at").expect("set-index function compiles");
    let add_code = decode(&encode(&add_code).expect("encode add-index function")).unwrap();
    let set_code = decode(&encode(&set_code).expect("encode set-index function")).unwrap();

    for (entry, code) in [("add_at", &add_code), ("set_at", &set_code)] {
        for arguments in [
            vec![Value::Int(5), Value::Int(0), Value::Int(3)],
            vec![Value::Int(-7), Value::Int(1), Value::Int(9)],
        ] {
            let reference = execution::execute_function(
                &mir,
                entry,
                arguments.clone(),
                ExecutionLimits::default(),
            )
            .expect("AST reference execution");
            let actual = execute(code, arguments, ExecutionLimits::default())
                .expect("bytecode mutable array execution");
            assert_eq!(actual.value, reference.value);
        }
    }

    let arguments = vec![Value::Int(5), Value::Int(8), Value::Int(1)];
    let expected = execution::execute_function(
        &mir,
        "add_at",
        arguments.clone(),
        ExecutionLimits::default(),
    )
    .unwrap_err();
    let actual = execute(&add_code, arguments, ExecutionLimits::default()).unwrap_err();
    assert_eq!(actual.message, expected.message);
    assert_eq!(actual.span, expected.span);
}

#[test]
fn bytecode_push_mut_and_pop_mut_match_ast_and_mir_after_roundtrip() {
    let source = r#"
fn exercise_stack() -> Int {
    mutable values: Int[] = []
    push_mut(values, 4)
    push_mut(values, 8)
    last = pop_mut(values)
    first = pop_mut(values)
    empty = pop_mut(values)
    last_value = match last { Some(value) => value None => 0 }
    first_value = match first { Some(value) => value None => 0 }
    if is_none(empty) { return last_value * 10 + first_value }
    return 0
}
fn main() {}
"#;
    let (mir, code) = compile_source(source, "exercise_stack");
    let code = decode(&encode(&code).expect("encode mutable array bytecode"))
        .expect("decode mutable array bytecode");
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let expected = zelyra_runtime::execute_function(&ast, "exercise_stack", vec![], None)
        .expect("AST execution");
    let mir_result =
        execution::execute_function(&mir, "exercise_stack", vec![], ExecutionLimits::default())
            .expect("MIR execution");
    let bytecode_result =
        execute(&code, vec![], ExecutionLimits::default()).expect("bytecode execution");

    assert_eq!(expected, Value::Int(84));
    assert_eq!(mir_result.value, expected);
    assert_eq!(bytecode_result.value, expected);
}

#[test]
fn bytecode_verifier_rejects_pop_mut_on_an_immutable_local() {
    let source = "fn main() { mutable values: Int[] = [1] pop_mut(values) }";
    let (_mir, mut bytecode) = compile_source(source, "main");
    let function = &mut bytecode.functions[bytecode.entry];
    let target = function
        .code
        .iter()
        .find_map(|instruction| match &instruction.operation {
            BytecodeOp::ArrayPopMut { local, .. } => Some(*local),
            _ => None,
        })
        .expect("source lowers the checked array-pop operation");
    let target_mutability = function
        .local_mutability
        .iter_mut()
        .find(|(local, _)| *local == target)
        .expect("array target has local mutability metadata");
    target_mutability.1 = false;

    let errors =
        verify_bytecode(&bytecode).expect_err("a mutated artifact cannot pop an immutable local");
    assert!(errors
        .iter()
        .any(|error| error.message.contains("mutate immutable local")));
}

#[test]
fn bytecode_supports_new_and_existing_tuple_bindings_with_ast_parity() {
    let source = r#"
fn pair() -> Tuple<Int, Int> { return (20, 22) }
fn bind_new() -> Int {
    (left, right) = pair()
    return left + right
}
fn bind_existing() -> Int {
    mutable left: Int = 0
    mutable right: Int = 0
    (left, right) = pair()
    return left + right
}
fn main() {}
"#;
    let (mir, new_code) = compile_source(source, "bind_new");
    let existing_code = compile(&mir, "bind_existing").expect("existing binding compiles");
    let new_code = decode(&encode(&new_code).expect("encode new tuple binding")).unwrap();
    let existing_code =
        decode(&encode(&existing_code).expect("encode existing tuple binding")).unwrap();

    for (entry, code) in [("bind_new", &new_code), ("bind_existing", &existing_code)] {
        let reference =
            execution::execute_function(&mir, entry, vec![], ExecutionLimits::default())
                .expect("AST reference execution");
        let actual =
            execute(code, vec![], ExecutionLimits::default()).expect("bytecode tuple binding");
        assert_eq!(actual.value, reference.value);
        assert_eq!(actual.value, Value::Int(42));
    }
}

#[test]
fn bytecode_tuple_binding_rejects_runtime_arity_mismatch() {
    let source = "fn bind() -> Int { (left, right) = (1, 2) return left + right } fn main() {}";
    let (_mir, mut bytecode) = compile_source(source, "bind");
    let (instruction_index, removed_local) = {
        let function = &mut bytecode.functions[bytecode.entry];
        let (instruction_index, binding) = function
            .code
            .iter_mut()
            .enumerate()
            .find_map(|(index, instruction)| match &mut instruction.operation {
                BytecodeOp::TupleBind { locals } => Some((index, locals)),
                _ => None,
            })
            .expect("tuple binding instruction");
        let removed_local = binding.pop().expect("second tuple binding local");
        (instruction_index, removed_local)
    };
    bytecode.functions[bytecode.entry]
        .initialization_writes
        .retain(|(index, local)| *index != instruction_index || *local != removed_local);
    let error = execute(&bytecode, vec![], ExecutionLimits::default())
        .expect_err("tuple arity mismatch must fail closed");
    assert!(error.message.contains("expects 1 values, received 2"));
}

#[test]
fn bytecode_calls_shared_collection_builtins_with_ast_parity() {
    let source = r#"
fn analyze(values: Int[], needle: Int) -> Int {
    return len(set_add(set(append(values, needle)), needle))
}
fn has(values: Int[], needle: Int) -> Bool { return contains(values, needle) }
fn main() {}
"#;
    let (mir, bytecode) = compile_source(source, "analyze");
    let has_bytecode = compile(&mir, "has").expect("contains function compiles");
    let bytecode = decode(&encode(&bytecode).expect("encode builtin program")).unwrap();
    let has_bytecode = decode(&encode(&has_bytecode).expect("encode contains program")).unwrap();
    for (values, needle, expected) in [(vec![1, 2], 7, 3), (vec![], 7, 1), (vec![2, 2], 2, 1)] {
        let expected_contains = values.contains(&needle);
        let arguments = vec![
            Value::Array(values.iter().copied().map(Value::Int).collect()),
            Value::Int(needle),
        ];
        let reference = execution::execute_function(
            &mir,
            "analyze",
            arguments.clone(),
            ExecutionLimits::default(),
        )
        .expect("AST builtin reference");
        let actual = execute(&bytecode, arguments.clone(), ExecutionLimits::default())
            .expect("bytecode builtin execution");
        assert_eq!(actual.value, reference.value);
        assert_eq!(actual.value, Value::Int(expected));
        let has_arguments = vec![arguments[0].clone(), Value::Int(needle)];
        let contains_reference = execution::execute_function(
            &mir,
            "has",
            has_arguments.clone(),
            ExecutionLimits::default(),
        )
        .expect("AST contains reference");
        let contains_actual = execute(&has_bytecode, has_arguments, ExecutionLimits::default())
            .expect("bytecode contains execution");
        assert_eq!(contains_actual.value, contains_reference.value);
        assert_eq!(contains_actual.value, Value::Bool(expected_contains));
    }
}

#[test]
fn bytecode_v15_lowers_inline_higher_order_lambdas_with_ast_parity() {
    let source = r#"
fn map_values(values: Int[], factor: Int) -> Int[] {
    return values |> map(|n| n * factor)
}
fn filter_values(values: Int[], minimum: Int) -> Int[] {
    return values |> filter(|n| n >= minimum)
}
fn find_value(values: Int[], target: Int) -> Int? {
    return values |> find(|n| n == target)
}
fn reduce_values(values: Int[], initial: Int, offset: Int) -> Int {
    return values |> reduce(initial, |total, n| total + n + offset)
}
fn main() {}
"#;
    let (mir, _) = compile_source(source, "map_values");
    let cases = [
        (
            "map_values",
            vec![1, 2, 3],
            vec![Value::Int(3)],
            Value::Array(vec![Value::Int(3), Value::Int(6), Value::Int(9)]),
        ),
        (
            "filter_values",
            vec![1, 2, 3, 4],
            vec![Value::Int(3)],
            Value::Array(vec![Value::Int(3), Value::Int(4)]),
        ),
        (
            "filter_values",
            vec![],
            vec![Value::Int(3)],
            Value::Array(vec![]),
        ),
        (
            "find_value",
            vec![1, 2, 3],
            vec![Value::Int(2)],
            Value::Option(Some(Box::new(Value::Int(2)))),
        ),
        (
            "find_value",
            vec![1, 2, 3],
            vec![Value::Int(9)],
            Value::Option(None),
        ),
        (
            "reduce_values",
            vec![1, 2, 3],
            vec![Value::Int(10), Value::Int(2)],
            Value::Int(22),
        ),
        (
            "reduce_values",
            vec![],
            vec![Value::Int(10), Value::Int(2)],
            Value::Int(10),
        ),
    ];
    for (name, values, mut tail, expected) in cases {
        let mut arguments = vec![Value::Array(values.into_iter().map(Value::Int).collect())];
        arguments.append(&mut tail);
        let bytecode = compile(&mir, name).expect("inline lambda lowers to bytecode");
        let decoded = decode(&encode(&bytecode).expect("higher-order bytecode encodes"))
            .expect("higher-order bytecode decodes");
        let reference =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .expect("MIR higher-order reference execution");
        let actual = execute(&decoded, arguments, ExecutionLimits::default())
            .expect("bytecode higher-order execution");
        assert_eq!(actual.value, reference.value, "function {name}");
        assert_eq!(actual.value, expected, "function {name}");
    }

    let budget_program = compile(&mir, "map_values").unwrap();
    let error = execute(
        &budget_program,
        vec![
            Value::Array((0..10).map(Value::Int).collect()),
            Value::Int(2),
        ],
        ExecutionLimits {
            max_steps: 12,
            max_stack_depth: 20,
        },
    )
    .expect_err("each lambda iteration must consume the bytecode step budget");
    assert!(error.message.contains("step budget"));
}

#[test]
fn bytecode_snapshot_capture_write_detector_covers_mutating_opcodes() {
    let captured = LocalId(7);
    let captured_set = HashSet::from([captured]);
    let pattern = MirPattern {
        kind: crate::MirPatternKind::Constructor {
            name: "Some".into(),
            inner: Some(Box::new(MirPattern {
                kind: crate::MirPatternKind::Record {
                    type_name: "Pair".into(),
                    fields: vec![(
                        "value".into(),
                        MirPattern {
                            kind: crate::MirPatternKind::Binding {
                                name: "value".into(),
                                local: captured,
                            },
                            span: Span::default(),
                        },
                    )],
                },
                span: Span::default(),
            })),
        },
        span: Span::default(),
    };
    let operations = [
        BytecodeOp::StoreLocal(captured),
        BytecodeOp::ArrayIndexAssign {
            local: captured,
            operation: ArrayIndexAssignOp::Set,
        },
        BytecodeOp::ArrayAppend {
            local: captured,
            element_type: Type::Int,
        },
        BytecodeOp::TupleBind {
            locals: vec![LocalId(2), captured],
        },
        BytecodeOp::RangeNext {
            iterator: IteratorId(0),
            local: captured,
            body: 1,
            exit: 2,
        },
        BytecodeOp::CollectionNext {
            iterator: IteratorId(0),
            locals: vec![captured],
            body: 1,
            exit: 2,
        },
        BytecodeOp::Parallel {
            bindings: vec![BytecodeParallelBinding {
                target: captured,
                is_new: false,
                function: 0,
                captures: Vec::new(),
                result_type: Type::Int,
            }],
        },
        BytecodeOp::MatchTest {
            match_id: MatchId(0),
            pattern,
            matched: 1,
            unmatched: 2,
        },
    ];
    for operation in operations {
        assert_eq!(
            bytecode_captured_write(&operation, &captured_set),
            Some(captured),
            "{operation:?} must not write a snapshot capture"
        );
    }
    assert_eq!(
        bytecode_captured_write(&BytecodeOp::LoadLocal(captured), &captured_set),
        None
    );
}

#[test]
fn bytecode_verifier_rejects_mutation_of_immutable_locals() {
    let source = "fn main() { immutable_value = 1 mutable changing_value = 2 changing_value = 3 }";
    let (_mir, mut bytecode) = compile_source(source, "main");
    let function = &mut bytecode.functions[bytecode.entry];
    let immutable = function
        .local_mutability
        .iter()
        .find_map(|(local, mutable)| (!mutable).then_some(*local))
        .expect("source has an immutable local");
    let mutable = function
        .local_mutability
        .iter()
        .find_map(|(local, mutable)| (*mutable).then_some(*local))
        .expect("source has a mutable local");
    let initialization_writes = function
        .initialization_writes
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let assignment = function
        .code
        .iter_mut()
        .enumerate()
        .find_map(|(index, instruction)| match &mut instruction.operation {
            BytecodeOp::StoreLocal(local)
                if *local == mutable && !initialization_writes.contains(&(index, mutable)) =>
            {
                Some(local)
            }
            _ => None,
        })
        .expect("source has a non-initializing mutable assignment");
    *assignment = immutable;
    let errors = verify_bytecode(&bytecode)
        .expect_err("an artifact cannot redirect a mutable assignment to an immutable local");
    assert!(errors
        .iter()
        .any(|error| error.message.contains("mutate immutable local")));
}

#[test]
fn bytecode_stored_capturing_lambdas_match_ast_and_mir_after_roundtrip() {
    let source = r#"
fn make_adder(base: Int) -> Function<(Int) -> Int> {
    mutable offset = base
    callback: Function<(Int) -> Int> = |value| value + offset
    offset = offset + 100
    return callback
}
fn invoke(callback: Function<(Int) -> Int>, value: Int) -> Int {
    return callback(value)
}
fn apply(values: Int[], callback: Function<(Int) -> Int>) -> Int[] {
    return values |> map(callback)
}
fn main() -> Int {
    add_twenty = make_adder(20)
    add_fifty = make_adder(50)
    mapped = apply([2, 3], add_twenty)
    return mapped[0] + mapped[1] + invoke(add_fifty, 3)
}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source type checks and lowers");
    let bytecode = compile(&mir, "main").expect("capturing closure compiles");
    assert!(bytecode
        .functions
        .iter()
        .any(|function| !function.captures.is_empty()));
    let encoded = encode(&bytecode).expect("closure bytecode encodes");
    let decoded = decode(&encoded).expect("closure bytecode decodes");

    let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None)
        .expect("AST closure execution");
    let mir_result = execution::execute_function(&mir, "main", vec![], ExecutionLimits::default())
        .expect("MIR closure execution");
    let actual = execute(&decoded, vec![], ExecutionLimits::default())
        .expect("roundtrip bytecode closure execution");
    assert_eq!(mir_result.value, expected);
    assert_eq!(actual.value, expected);
    assert_eq!(actual.value, Value::Int(98));

    let mut invalid_layout = decoded.clone();
    let mut changed = false;
    for function in &mut invalid_layout.functions {
        for instruction in &mut function.code {
            if let BytecodeOp::MakeClosure { captures, .. } = &mut instruction.operation {
                captures[0] = LocalId(usize::MAX);
                changed = true;
                break;
            }
        }
        if changed {
            break;
        }
    }
    assert!(changed, "compiled program contains a closure construction");
    assert!(verify_bytecode(&invalid_layout).is_err());

    let mut elevated = decoded.clone();
    let closure_target = elevated
        .functions
        .iter()
        .find(|function| !function.captures.is_empty())
        .expect("generated capturing closure function")
        .id;
    let closure_target = elevated
        .functions
        .iter()
        .position(|function| function.id == closure_target)
        .unwrap();
    let mut mutating_capture = decoded.clone();
    let captured_local = mutating_capture.functions[closure_target].captures[0].0;
    let mut replaced_read = false;
    for instruction in &mut mutating_capture.functions[closure_target].code {
        if matches!(instruction.operation, BytecodeOp::LoadLocal(local) if local == captured_local)
        {
            instruction.operation = BytecodeOp::StoreLocal(captured_local);
            replaced_read = true;
            break;
        }
    }
    assert!(replaced_read, "closure body reads its captured local");
    let error = verify_bytecode(&mutating_capture)
        .expect_err("verified closure bytecode cannot mutate a captured snapshot");
    assert!(error[0].message.contains("mutate immutable captured local"));

    elevated.functions[closure_target]
        .declared_capabilities
        .push("Clock".into());
    assert!(verify_bytecode(&elevated)
        .expect_err("closure cannot elevate its creator's capabilities")
        .iter()
        .any(|error| error.message.contains("exceed declared capabilities")));

    let mut wrong_capture_type = decoded;
    wrong_capture_type.functions[closure_target].captures[0].1 = Type::String;
    let error = execute(&wrong_capture_type, vec![], ExecutionLimits::default())
        .expect_err("runtime validates the captured local's type");
    assert!(error.message.contains("closure capture expected"));
}

#[test]
fn bytecode_nested_capturing_lambdas_forward_outer_captures() {
    let source = r#"
fn make_adder(base: Int) -> Function<(Int) -> Function<(Int) -> Int>> {
    return |offset| |value| base + offset + value
}
fn main() -> Int {
    add_twenty = make_adder(20)
    add_two = add_twenty(2)
    return add_two(20)
}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("nested closure source type checks");
    let bytecode = compile(&mir, "main").expect("nested closure compiles");
    let decoded = decode(&encode(&bytecode).expect("nested closure encodes"))
        .expect("nested closure decodes");
    let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None)
        .expect("AST nested closure execution");
    let mir_result = execution::execute_function(&mir, "main", vec![], ExecutionLimits::default())
        .expect("MIR nested closure execution");
    let actual = execute(&decoded, vec![], ExecutionLimits::default())
        .expect("bytecode nested closure execution");
    assert_eq!(expected, Value::Int(42));
    assert_eq!(mir_result.value, expected);
    assert_eq!(actual.value, expected);
}

#[test]
fn bytecode_higher_order_calls_execute_dynamic_function_values_with_ast_parity() {
    let source = r#"
fn double(value: Int) -> Int { return value * 2 }
fn triple(value: Int) -> Int { return value * 3 }
fn keep_even(value: Int) -> Bool { return value % 2 == 0 }
fn is_three(value: Int) -> Bool { return value == 3 }
fn add(accumulator: Int, value: Int) -> Int { return accumulator + value }
fn apply(values: Int[], callback: Function<(Int) -> Int>) -> Int[] {
    return values |> map(callback)
}
fn apply_filter(values: Int[], callback: Function<(Int) -> Bool>) -> Int[] {
    return values |> filter(callback)
}
fn apply_find(values: Int[], callback: Function<(Int) -> Bool>) -> Int? {
    return values |> find(callback)
}
fn apply_reduce(values: Int[], callback: Function<(Int, Int) -> Int>) -> Int {
    return values |> reduce(7, callback)
}
fn apply_named(values: Int[]) -> Int[] { return apply(values, &double) }
fn filter_named(values: Int[]) -> Int[] { return apply_filter(values, &keep_even) }
fn find_named(values: Int[]) -> Int? { return apply_find(values, &is_three) }
fn reduce_named(values: Int[]) -> Int { return apply_reduce(values, &add) }
fn apply_mutable(values: Int[]) -> Int[] {
    mutable callback: Function<(Int) -> Int> = &double
    callback = &triple
    return apply(values, callback)
}
fn main() {}
"#;
    let ast = parse(&lex(source).unwrap()).unwrap();
    let mir = crate::lower(&ast).unwrap();
    for (name, expected) in [
        (
            "apply_named",
            Value::Array(vec![Value::Int(2), Value::Int(4)]),
        ),
        ("filter_named", Value::Array(vec![Value::Int(2)])),
        ("find_named", Value::Option(None)),
        ("reduce_named", Value::Int(10)),
        (
            "apply_mutable",
            Value::Array(vec![Value::Int(3), Value::Int(6)]),
        ),
    ] {
        let bytecode = compile(&mir, name).expect("dynamic callback compiles");
        let bytecode = decode(&encode(&bytecode).expect("dynamic callback encodes"))
            .expect("dynamic callback decodes");
        let arguments = vec![Value::Array(vec![Value::Int(1), Value::Int(2)])];
        let reference =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .expect("AST callback reference");
        let actual = execute(&bytecode, arguments, ExecutionLimits::default())
            .expect("bytecode callback execution");
        assert_eq!(actual.value, reference.value);
        assert_eq!(actual.value, expected);
    }
    let empty_arguments = vec![Value::Array(vec![])];
    let find_empty = compile(&mir, "find_named").expect("dynamic find compiles");
    let find_empty = decode(&encode(&find_empty).unwrap()).unwrap();
    assert_eq!(
        execute(&find_empty, empty_arguments, ExecutionLimits::default())
            .unwrap()
            .value,
        Value::Option(None)
    );
    let found_arguments = vec![Value::Array(vec![Value::Int(3), Value::Int(4)])];
    let found_reference = execution::execute_function(
        &mir,
        "find_named",
        found_arguments.clone(),
        ExecutionLimits::default(),
    )
    .unwrap();
    let found_actual = execute(&find_empty, found_arguments, ExecutionLimits::default()).unwrap();
    assert_eq!(found_actual.value, found_reference.value);
    assert_eq!(
        found_actual.value,
        Value::Option(Some(Box::new(Value::Int(3))))
    );
}

#[test]
fn bytecode_v15_lowers_named_function_callbacks_with_ast_parity() {
    let source = r#"
fn double(value: Int) -> Int { return value * 2 }
fn keep_even(value: Int) -> Bool { return value % 2 == 0 }
fn is_three(value: Int) -> Bool { return value == 3 }
fn add(accumulator: Int, value: Int) -> Int { return accumulator + value }
fn mapped(values: Int[]) -> Int[] { return values |> map(&double) }
fn mapped_via_immutable_alias(values: Int[]) -> Int[] {
    callback: Function<(Int) -> Int> = &double
    return values |> map(callback)
}
fn filtered(values: Int[]) -> Int[] { return values |> filter(&keep_even) }
fn found(values: Int[]) -> Int? { return values |> find(&is_three) }
fn reduced(values: Int[]) -> Int { return values |> reduce(7, &add) }
fn main() {}
"#;
    let (mir, _) = compile_source(source, "mapped");
    for (name, input, expected) in [
        (
            "mapped",
            vec![1, 2, 3],
            Value::Array(vec![Value::Int(2), Value::Int(4), Value::Int(6)]),
        ),
        (
            "mapped_via_immutable_alias",
            vec![2, 3],
            Value::Array(vec![Value::Int(4), Value::Int(6)]),
        ),
        (
            "filtered",
            vec![1, 2, 3, 4],
            Value::Array(vec![Value::Int(2), Value::Int(4)]),
        ),
        (
            "found",
            vec![1, 2, 3],
            Value::Option(Some(Box::new(Value::Int(3)))),
        ),
        ("found", vec![1, 2], Value::Option(None)),
        ("reduced", vec![1, 2, 3], Value::Int(13)),
    ] {
        let arguments = vec![Value::Array(input.into_iter().map(Value::Int).collect())];
        let bytecode = compile(&mir, name).expect("statically named callback compiles");
        let decoded = decode(&encode(&bytecode).unwrap()).unwrap();
        let reference =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .unwrap();
        let actual = execute(&decoded, arguments, ExecutionLimits::default()).unwrap();
        assert_eq!(actual.value, reference.value, "{name}");
        assert_eq!(actual.value, expected, "{name}");
    }
}

#[test]
fn bytecode_inline_lambda_host_calls_keep_capability_preflight() {
    let source = r#"
fn randomized(values: Int[]) -> Int[] uses Random {
    return values |> map(|n| random_int(0, n))
}
fn main() {}
"#;
    let (mir, bytecode) = compile_source(source, "randomized");
    let make_host = || {
        Arc::new(FixedHost {
            timestamp: 0,
            environment: None,
            calls: AtomicUsize::new(0),
        })
    };
    let arguments = vec![Value::Array(vec![Value::Int(2), Value::Int(4)])];
    let denied_host = make_host();
    let denied = execute_with_host_services(
        &bytecode,
        arguments.clone(),
        ExecutionLimits::default(),
        None,
        denied_host.clone(),
    )
    .expect_err("the lambda must not bypass project capability grants");
    assert!(denied.message.contains("`Random` is not granted"));
    assert_eq!(denied_host.calls.load(Ordering::SeqCst), 0);

    let grants = HashSet::from(["Random".to_owned()]);
    let host = make_host();
    let actual = execute_with_host_services(
        &decode(&encode(&bytecode).unwrap()).unwrap(),
        arguments.clone(),
        ExecutionLimits::default(),
        Some(&grants),
        host.clone(),
    )
    .expect("granted random calls in inline lambda execute");
    assert_eq!(host.calls.load(Ordering::SeqCst), 2);
    let expected = execution::execute_function_with_host_services(
        &mir,
        "randomized",
        arguments,
        ExecutionLimits::default(),
        Some(&grants),
        make_host(),
    )
    .expect("MIR reference random execution");
    assert_eq!(actual.value, expected.value);
}

#[test]
fn bytecode_calls_shared_range_sort_and_reverse_builtins() {
    let source = r#"
fn calculate(start: Int, end: Int) -> Int {
    values = range(start, end)
    ordered = sort(values)
    reversed = reverse(ordered)
    if len(reversed) == 0 { return 0 } else { return reversed[0] }
}
fn main() {}
"#;
    let (mir, bytecode) = compile_source(source, "calculate");
    let bytecode = decode(&encode(&bytecode).expect("encode range builtin")).unwrap();
    for (start, end, expected) in [(2, 6, 5), (-3, 0, -1), (4, 4, 0)] {
        let arguments = vec![Value::Int(start), Value::Int(end)];
        let reference = execution::execute_function(
            &mir,
            "calculate",
            arguments.clone(),
            ExecutionLimits::default(),
        )
        .expect("AST builtin reference");
        let actual = execute(&bytecode, arguments, ExecutionLimits::default())
            .expect("bytecode builtin execution");
        assert_eq!(actual.value, reference.value);
        assert_eq!(actual.value, Value::Int(expected));
    }
}

#[test]
fn bytecode_csv_encode_and_decode_match_ast_and_mir() {
    let source = r#"
fn main() -> String {
    rows = [["number", "customer", "note"], ["INV-1", "Acme, Inc.", "Paid\nby transfer"]]
    encoded = match csv_encode(rows, ',') {
        Ok(text) => text
        Err(message) => message
    }
    return match csv_decode_result(encoded, ',') {
        Ok(decoded) => decoded[1][2]
        Err(message) => message
    }
}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let (mir, bytecode) = compile_source(source, "main");
    let bytecode =
        decode(&encode(&bytecode).expect("CSV bytecode encodes")).expect("CSV bytecode decodes");
    let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None)
        .expect("AST CSV builtins execute");
    let mir_result = execution::execute_function(&mir, "main", vec![], ExecutionLimits::default())
        .expect("MIR CSV builtins execute");
    let actual = execute(&bytecode, vec![], ExecutionLimits::default())
        .expect("bytecode CSV builtins execute");
    assert_eq!(expected, Value::String("Paid\nby transfer".into()));
    assert_eq!(mir_result.value, expected);
    assert_eq!(actual.value, expected);
}

#[test]
fn bytecode_fixed_point_money_builtins_match_ast_and_mir() {
    let source = r#"
fn money_text(input_text: String, scale: Int) -> String {
    return match decimal_scaled_result(input_text, scale) {
        Ok(value) => match format_scaled_result(value, scale) {
            Ok(text) => text
            Err(message) => message
        }
        Err(message) => message
    }
}
fn rounded(amount: Decimal, mode: String) -> Decimal {
    return match decimal_quantize_result(amount, 2, mode) {
        Ok(value) => value
        Err(_) => decimal("0")
    }
}
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let (mir, bytecode) = compile_source(source, "money_text");
    let bytecode = decode(&encode(&bytecode).expect("money bytecode encodes"))
        .expect("money bytecode decodes");
    for (source, scale, expected) in [("119.00", 2, "119.00"), ("1,2", 2, "1.20")] {
        let arguments = vec![Value::String(source.into()), Value::Int(scale)];
        let expected_value =
            zelyra_runtime::execute_function(&ast, "money_text", arguments.clone(), None)
                .expect("AST money formatting executes");
        let mir_result = execution::execute_function(
            &mir,
            "money_text",
            arguments.clone(),
            ExecutionLimits::default(),
        )
        .expect("MIR money formatting executes");
        let actual = execute(&bytecode, arguments, ExecutionLimits::default())
            .expect("bytecode money formatting executes");
        assert_eq!(expected_value, Value::String(expected.into()));
        assert_eq!(mir_result.value, expected_value);
        assert_eq!(actual.value, expected_value);
    }
    let arguments = vec![
        Value::Decimal(1_245_000_000),
        Value::String("half_even".into()),
    ];
    let ast_value = zelyra_runtime::execute_function(&ast, "rounded", arguments.clone(), None)
        .expect("AST quantization executes");
    let mir_value = execution::execute_function(
        &mir,
        "rounded",
        arguments.clone(),
        ExecutionLimits::default(),
    )
    .expect("MIR quantization executes");
    let bytecode = decode(
        &encode(&compile(&mir, "rounded").expect("rounding function compiles"))
            .expect("rounding bytecode encodes"),
    )
    .expect("rounding bytecode decodes");
    let bytecode_value = execute(&bytecode, arguments, ExecutionLimits::default())
        .expect("bytecode quantization executes");
    assert_eq!(ast_value, Value::Decimal(1_240_000_000));
    assert_eq!(mir_value.value, ast_value);
    assert_eq!(bytecode_value.value, ast_value);
}

#[test]
fn bytecode_calls_shared_option_and_result_constructors() {
    let source = r#"
fn present(value: Int) -> Option<Int> { return Some(value) }
fn main() {}
"#;
    let (mir, bytecode) = compile_source(source, "present");
    let bytecode = decode(&encode(&bytecode).expect("encode Some constructor")).unwrap();
    let arguments = vec![Value::Int(42)];
    let reference = execution::execute_function(
        &mir,
        "present",
        arguments.clone(),
        ExecutionLimits::default(),
    )
    .expect("AST constructor reference");
    let actual = execute(&bytecode, arguments, ExecutionLimits::default())
        .expect("bytecode constructor execution");
    assert_eq!(actual.value, reference.value);
}

#[test]
fn bytecode_preserves_short_circuit_and_branch_behavior() {
    let source = r#"
fn choose(value: Int) -> Bool {
    return value < 0 || value > 10 && value != 99
}
fn main() { print(choose(11)) }
"#;
    let (mir, bytecode) = compile_source(source, "choose");
    for value in [-1, 5, 11, 99] {
        let args = vec![Value::Int(value)];
        let reference =
            execution::execute_function(&mir, "choose", args.clone(), ExecutionLimits::default())
                .unwrap();
        let actual = execute(&bytecode, args, ExecutionLimits::default()).unwrap();
        assert_eq!(actual.value, reference.value, "input {value}");
    }
}

#[test]
fn bytecode_executes_scalar_loop_control_flow_with_ast_parity() {
    let source = r#"
fn sum_to(limit: Int) -> Int {
    mutable current = 0
    mutable total = 0
    while current < limit {
        total = total + current
        current = current + 1
    }
    return total
}
fn sum_odds(limit: Int) -> Int {
    mutable current = 0
    mutable total = 0
    loop {
        if current >= limit { break }
        current = current + 1
        if current % 2 == 0 { continue }
        total = total + current
    }
    return total
}
fn sum_range(start: Int, end: Int, step: Int) -> Int {
    mutable total = 0
    for value in range(start, end, step) {
        total = total + value
    }
    return total
}
fn main() {}
"#;
    let (mir, bytecode) = compile_source(source, "sum_to");
    for limit in [0, 1, 2, 10, 100] {
        let arguments = vec![Value::Int(limit)];
        let reference = execution::execute_function(
            &mir,
            "sum_to",
            arguments.clone(),
            ExecutionLimits::default(),
        )
        .expect("AST reference execution");
        let actual = execute(&bytecode, arguments, ExecutionLimits::default())
            .expect("bytecode while execution");
        assert_eq!(actual.value, reference.value, "limit {limit}");
        assert_eq!(actual.value, Value::Int(limit * (limit - 1) / 2));
    }

    let loop_bytecode = compile(&mir, "sum_odds").expect("loop entry compiles");
    for limit in [0, 1, 2, 10, 100] {
        let arguments = vec![Value::Int(limit)];
        let reference = execution::execute_function(
            &mir,
            "sum_odds",
            arguments.clone(),
            ExecutionLimits::default(),
        )
        .expect("AST reference execution");
        let actual = execute(&loop_bytecode, arguments, ExecutionLimits::default())
            .expect("bytecode loop/break/continue execution");
        assert_eq!(actual.value, reference.value, "limit {limit}");
        let expected = (1..=limit).filter(|value| value % 2 == 1).sum::<i64>();
        assert_eq!(actual.value, Value::Int(expected));
    }

    let range_bytecode = compile(&mir, "sum_range").expect("integer range compiles");
    let serialized = encode(&range_bytecode).expect("range bytecode encodes");
    let decoded = decode(&serialized).expect("range bytecode decodes");
    for (start, end, step, expected) in [(1, 5, 1, 10), (5, 0, -2, 9), (4, 4, 1, 0)] {
        let arguments = vec![Value::Int(start), Value::Int(end), Value::Int(step)];
        let reference = execution::execute_function(
            &mir,
            "sum_range",
            arguments.clone(),
            ExecutionLimits::default(),
        )
        .expect("AST range reference execution");
        let actual = execute(&decoded, arguments, ExecutionLimits::default())
            .expect("serialized bytecode range execution");
        assert_eq!(actual.value, reference.value);
        assert_eq!(actual.value, Value::Int(expected));
    }
    let zero_step = execute(
        &decoded,
        vec![Value::Int(0), Value::Int(5), Value::Int(0)],
        ExecutionLimits::default(),
    )
    .expect_err("zero range step must fail");
    assert!(zero_step.message.contains("step must not be zero"));

    let error = execute(
        &bytecode,
        vec![Value::Int(100)],
        ExecutionLimits {
            max_steps: 5,
            max_stack_depth: 20,
        },
    )
    .expect_err("while loops must consume the deterministic step budget");
    assert!(error.message.contains("step budget"));
}

#[test]
fn bytecode_matches_scalar_and_array_operator_semantics() {
    let source = r#"
fn uint_math(value: UInt, one: UInt, two: UInt) -> UInt { return (value + one) * two }
fn float_order(value: Float) -> Bool { return value >= 1.5 }
fn string_order(value: String) -> Bool { return value < "zelyra" }
fn array_concat(left: Int[], right: Int[]) -> Int[] { return left + right }
fn main() { print("kept outside the selected bytecode closure") }
"#;
    let (mir, bytecode) = compile_source(source, "uint_math");
    for (name, arguments) in [
        (
            "uint_math",
            vec![Value::UInt(20), Value::UInt(1), Value::UInt(2)],
        ),
        ("float_order", vec![Value::Float(2.0)]),
        ("string_order", vec![Value::String("alpha".into())]),
    ] {
        let function_bytecode = if name == "uint_math" {
            bytecode.clone()
        } else {
            compile(&mir, name).unwrap()
        };
        let reference =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .unwrap();
        let actual = execute(&function_bytecode, arguments, ExecutionLimits::default()).unwrap();
        assert_eq!(actual.value, reference.value, "function {name}");
    }
    let array_arguments = vec![
        Value::Array(vec![Value::Int(1), Value::Int(2)]),
        Value::Array(vec![Value::Int(3)]),
    ];
    let array_bytecode = compile(&mir, "array_concat").expect("array concatenation compiles");
    let array_bytecode =
        decode(&encode(&array_bytecode).expect("array concatenation bytecode encodes"))
            .expect("array concatenation bytecode decodes");
    let array_reference = execution::execute_function(
        &mir,
        "array_concat",
        array_arguments.clone(),
        ExecutionLimits::default(),
    )
    .unwrap();
    let array_actual = execute(&array_bytecode, array_arguments, ExecutionLimits::default())
        .expect("array concatenation executes");
    assert_eq!(array_actual.value, array_reference.value);
    assert_eq!(
        array_actual.value,
        Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn bytecode_matches_decimal_duration_and_temporal_operator_semantics() {
    let source = r#"
fn decimal_calculate(left: Decimal, right: Decimal) -> Decimal { return (left + right) * right }
fn decimal_order(left: Decimal, right: Decimal) -> Bool { return left >= right }
fn duration_calculate(left: Duration, right: Duration) -> Duration { return left + right }
fn duration_order(left: Duration, right: Duration) -> Bool { return left < right }
fn date_order(left: Date, right: Date) -> Bool { return left <= right }
fn time_order(left: Time, right: Time) -> Bool { return left > right }
fn timestamp_order(left: Timestamp, right: Timestamp) -> Bool { return left != right }
fn main() {}
"#;
    let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
    let mir = crate::lower(&ast).expect("source type-checks and lowers");
    let cases = [
        (
            "decimal_calculate",
            vec![Value::Decimal(1_500_000_000), Value::Decimal(2_000_000_000)],
        ),
        (
            "decimal_order",
            vec![Value::Decimal(2_000_000_000), Value::Decimal(1_500_000_000)],
        ),
        (
            "duration_calculate",
            vec![Value::Duration(3_000), Value::Duration(4_000)],
        ),
        (
            "duration_order",
            vec![Value::Duration(3_000), Value::Duration(4_000)],
        ),
        (
            "date_order",
            vec![
                Value::Date("2024-02-28".into()),
                Value::Date("2024-02-29".into()),
            ],
        ),
        (
            "time_order",
            vec![
                Value::Time("09:30:00".into()),
                Value::Time("08:30:00".into()),
            ],
        ),
        (
            "timestamp_order",
            vec![
                Value::Timestamp(1_700_000_001_000),
                Value::Timestamp(1_700_000_000_000),
            ],
        ),
    ];
    for (name, arguments) in cases {
        let compiled = compile(&mir, name).expect("business operator compiles");
        let decoded = decode(&encode(&compiled).expect("business operator encodes"))
            .expect("business operator decodes");
        let expected = zelyra_runtime::execute_function(&ast, name, arguments.clone(), None)
            .expect("AST reference executes");
        let mir_result =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .expect("MIR reference executes");
        let actual = execute(&decoded, arguments, ExecutionLimits::default())
            .expect("serialized bytecode executes");
        assert_eq!(mir_result.value, expected, "MIR function {name}");
        assert_eq!(actual.value, expected, "bytecode function {name}");
    }
    for (name, arguments) in [
        (
            "decimal_calculate",
            vec![Value::Decimal(i128::MAX), Value::Decimal(1)],
        ),
        (
            "duration_calculate",
            vec![Value::Duration(i64::MAX), Value::Duration(1)],
        ),
    ] {
        let compiled = compile(&mir, name).expect("overflow test function compiles");
        let decoded = decode(&encode(&compiled).expect("overflow test encodes"))
            .expect("overflow test decodes");
        let expected = zelyra_runtime::execute_function(&ast, name, arguments.clone(), None)
            .expect_err("AST reports checked overflow");
        let mir_error =
            execution::execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                .expect_err("MIR reports checked overflow");
        let actual = execute(&decoded, arguments, ExecutionLimits::default())
            .expect_err("bytecode reports checked overflow");
        assert_eq!(mir_error.message, expected.message);
        assert_eq!(mir_error.span, expected.span);
        assert_eq!(actual.message, expected.message);
        assert_eq!(actual.span, expected.span);
    }
}

#[test]
fn bytecode_and_mir_share_checked_unary_operator_semantics() {
    let (mir, bytecode) = compile_source(
        "fn negate(value: Int) -> Int { return -value }\nfn main() {}",
        "negate",
    );
    let bytecode_result =
        execute(&bytecode, vec![Value::Int(17)], ExecutionLimits::default()).unwrap();
    let mir_result = execution::execute_function(
        &mir,
        "negate",
        vec![Value::Int(17)],
        ExecutionLimits::default(),
    )
    .unwrap();
    assert_eq!(bytecode_result.value, mir_result.value);
    assert_eq!(bytecode_result.value, Value::Int(-17));

    let bytecode_error = execute(
        &bytecode,
        vec![Value::Int(i64::MIN)],
        ExecutionLimits::default(),
    )
    .unwrap_err();
    let mir_error = execution::execute_function(
        &mir,
        "negate",
        vec![Value::Int(i64::MIN)],
        ExecutionLimits::default(),
    )
    .unwrap_err();
    assert_eq!(bytecode_error.message, mir_error.message);
    assert_eq!(bytecode_error.span, mir_error.span);
}

#[test]
fn bytecode_rejects_unsupported_host_effects_and_bad_artifacts() {
    let source = r#"fn queue() uses Email, Database(write) { transaction { mail_queue("a@example.test", "Hello", "Body") } } fn main() {}"#;
    let ast = parse(&lex(source).unwrap()).unwrap();
    let mir = crate::lower(&ast).unwrap();
    let errors = compile(&mir, "deliver").unwrap_err();
    assert_eq!(errors[0].code, "E-MIR-BYTECODE-001");

    let (_, mut bytecode) = compile_source(
        "fn value() -> Int { return 7 } fn main() { print(value()) }",
        "value",
    );
    bytecode.version += 1;
    let errors = verify_bytecode(&bytecode).unwrap_err();
    assert!(errors[0].message.contains("unsupported bytecode version"));

    let (_, mut bytecode) = compile_source(
            "fn choose(value: Int) -> Bool { if value > 0 { return true } else { return false } } fn main() { print(choose(1)) }",
            "choose",
        );
    let jump = bytecode.functions[0]
        .code
        .iter_mut()
        .find_map(|instruction| match &mut instruction.operation {
            BytecodeOp::Jump(target) => Some(target),
            _ => None,
        })
        .expect("branch function has a jump");
    *jump = usize::MAX;
    let errors = verify_bytecode(&bytecode).unwrap_err();
    assert!(errors[0].message.contains("control flow falls outside"));
}

#[test]
fn bytecode_parallel_await_snapshots_captures_and_matches_ast() {
    let source = r#"
fn add_one(value: Int) -> Int { return value + 1 }
fn double(value: Int) -> Int { return value * 2 }
fn main() -> Int {
    mutable existing = 10
    parallel {
        first = await add_one(existing)
        existing = await double(existing)
    }
    return first + existing
}


"#;
    let ast = parse(&lex(source).unwrap()).unwrap();
    let mir = crate::lower(&ast).unwrap();
    let bytecode = compile(&mir, "main").expect("parallel bindings lower to bytecode");
    let decoded = decode(&encode(&bytecode).unwrap()).unwrap();
    let reference = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
    let actual = execute(&decoded, vec![], ExecutionLimits::default()).unwrap();
    assert_eq!(actual.value, reference);
    assert_eq!(actual.value, Value::Int(31));

    let mut malformed = bytecode;
    let parallel = malformed.functions.iter_mut().find_map(|function| {
        function.code.iter_mut().find_map(|instruction| {
            if let BytecodeOp::Parallel { bindings } = &mut instruction.operation {
                Some(bindings)
            } else {
                None
            }
        })
    });
    parallel.unwrap()[0].function = usize::MAX;
    let error = verify_bytecode(&malformed).expect_err("invalid branch indices are rejected");
    assert!(error[0].message.contains("parallel branch is outside"));
}

#[test]
fn bytecode_verifier_rejects_parallel_blocks_over_the_shared_branch_limit() {
    let source = "fn value() -> Int { return 1 } fn main() { parallel { only = await value() } }";
    let (mir, mut bytecode) = compile_source(source, "main");
    let parallel = bytecode
        .functions
        .iter_mut()
        .find_map(|function| {
            function.code.iter_mut().find_map(|instruction| {
                if let BytecodeOp::Parallel { bindings } = &mut instruction.operation {
                    Some(bindings)
                } else {
                    None
                }
            })
        })
        .expect("parallel instruction exists");
    let binding = parallel[0].clone();
    while parallel.len() <= zelyra_runtime::MAX_PARALLEL_BRANCHES {
        parallel.push(binding.clone());
    }
    let errors = verify_bytecode(&bytecode).expect_err("bytecode verifier must cap branches");
    assert!(errors[0].message.contains("maximum of 4 branches"));
    assert!(encode(&bytecode).is_err());
    assert!(crate::verify(&mir).is_ok());
}

#[test]
fn bytecode_parallel_branches_share_the_step_budget() {
    let source = r#"
fn spend() -> Int {
    mutable value = 0
    while value < 100 { value = value + 1 }
    return value
}
fn main() {
    parallel {
        left = await spend()
        right = await spend()
    }
}
"#;
    let (mir, bytecode) = compile_source(source, "main");
    let limits = ExecutionLimits {
        max_steps: 30,
        max_stack_depth: 20,
    };
    let expected = execution::execute_function(&mir, "main", vec![], limits)
        .expect_err("MIR must exceed the shared parallel step budget");
    let actual = execute(&bytecode, vec![], limits)
        .expect_err("bytecode branches must share the step budget");
    assert!(actual.message.contains("step budget exceeded"));
    assert_eq!(actual.span, expected.span);
}

#[test]
fn bytecode_parallel_discards_branch_output_when_a_branch_fails() {
    let source = r#"
fn succeeds() -> Int { print("branch output"); return 1 }
fn fails() -> Int { return 1 / 0 }
fn main() {
    parallel {
        first = await succeeds()
        second = await fails()
    }
}
"#;
    let ast = parse(&lex(source).unwrap()).unwrap();
    let mir = crate::lower(&ast).unwrap();
    let bytecode = compile(&mir, "main").expect("parallel bindings lower to bytecode");
    let expected = zelyra_runtime::execute_function_captured_with_capabilities_and_policies(
        &ast,
        "main",
        vec![],
        None,
        None,
        None,
    );
    let actual = execute_with_host_services_and_policy_captured(
        &bytecode,
        vec![],
        ExecutionLimits::default(),
        None,
        None,
        Arc::new(SystemHostServices),
    );
    assert_eq!(actual.output, expected.output);
    assert_eq!(actual.output, Vec::<String>::new());
    assert_eq!(
        actual.error.as_ref().map(|error| &error.message),
        expected.error.as_ref().map(|error| &error.message)
    );
}

#[test]
fn bytecode_parallel_host_calls_overlap_and_keep_capability_checks() {
    struct ConcurrentHost {
        arrivals: std::sync::Mutex<usize>,
        ready: std::sync::Condvar,
        calls: AtomicUsize,
    }

    impl HostServices for ConcurrentHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            Ok(0)
        }

        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }

        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }

        fn http_request(
            &self,
            _method: &str,
            _url: &str,
            _headers: &[(String, String)],
            _body: Option<&[u8]>,
            _timeout_ms: u64,
            _max_response_bytes: usize,
        ) -> Result<zelyra_runtime::HostHttpResponse, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let mut arrivals = self.arrivals.lock().unwrap();
            *arrivals += 1;
            self.ready.notify_all();
            let (arrivals, timeout) = self
                .ready
                .wait_timeout_while(arrivals, std::time::Duration::from_secs(2), |arrivals| {
                    *arrivals < 2
                })
                .unwrap();
            if timeout.timed_out() && *arrivals < 2 {
                return Err("parallel bytecode workers did not overlap".into());
            }
            Ok(zelyra_runtime::HostHttpResponse {
                status: 200,
                headers: vec![],
                body: b"ok".to_vec(),
            })
        }
    }

    let source = r#"
fn main() -> String uses Network {
    parallel {
        first = await http_get("https://api.example.test/first")
        second = await http_get("https://api.example.test/second")
    }
    return first + second
}
"#;
    let (_, bytecode) = compile_source(source, "main");
    let host = Arc::new(ConcurrentHost {
        arrivals: std::sync::Mutex::new(0),
        ready: std::sync::Condvar::new(),
        calls: AtomicUsize::new(0),
    });
    let policy = RuntimePolicy {
        network: Some(NetworkPolicy {
            allowed_hosts: vec!["api.example.test".into()],
            timeout_ms: 1000,
            max_response_bytes: 1024,
        }),
        ..RuntimePolicy::default()
    };
    let grants = HashSet::from(["Network".to_owned()]);
    let actual = execute_with_host_services_and_policy_captured(
        &bytecode,
        vec![],
        ExecutionLimits::default(),
        Some(&grants),
        Some(&policy),
        host.clone(),
    );
    assert_eq!(actual.error, None);
    assert_eq!(actual.value, Some(Value::String("okok".into())));
    assert_eq!(host.calls.load(Ordering::SeqCst), 2);

    let denied_host = Arc::new(ConcurrentHost {
        arrivals: std::sync::Mutex::new(0),
        ready: std::sync::Condvar::new(),
        calls: AtomicUsize::new(0),
    });
    let denied = execute_with_host_services_and_policy_captured(
        &bytecode,
        vec![],
        ExecutionLimits::default(),
        Some(&HashSet::new()),
        Some(&policy),
        denied_host.clone(),
    );
    assert!(denied
        .error
        .is_some_and(|error| error.message.contains("`Network` is not granted")));
    assert_eq!(denied_host.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn bytecode_loop_invariants_match_ast_and_mir_in_all_loop_exit_paths() {
    let cases = [
            (
                "count",
                "fn count() -> Int { mutable value: Int = 0 while value < 3 invariant { value >= 0 } { value = value + 1 } return value } fn main() {}",
                true,
            ),
            (
                "before_bad",
                "fn before_bad() -> Int { while false invariant { false } { } return 0 } fn main() {}",
                false,
            ),
            (
                "break_bad",
                "fn break_bad() -> Int { mutable value: Int = 0 loop invariant { value == 0 } { value = 1; break } return value } fn main() {}",
                false,
            ),
            (
                "continue_bad",
                "fn continue_bad() -> Int { mutable value: Int = 0 while value < 2 invariant { value < 1 } { value = value + 1; continue } return value } fn main() {}",
                false,
            ),
            (
                "return_bad",
                "fn return_bad() -> Int { mutable value: Int = 0 loop invariant { value == 0 } { value = 1; return value } return 0 } fn main() {}",
                false,
            ),
            (
                "nested_return_bad",
                "fn nested_return_bad() -> Int { mutable value: Int = 0 loop invariant { value == 0 } { loop invariant { value == 0 } { value = 1; return value } } return 0 } fn main() {}",
                false,
            ),
        ];

    for (name, source, succeeds) in cases {
        let ast = parse(&lex(source).unwrap()).unwrap();
        let mir = crate::lower(&ast).unwrap();
        let bytecode = compile(&mir, name).expect("loop invariant program compiles");
        let decoded = decode(&encode(&bytecode).unwrap()).unwrap();
        let ast_result = zelyra_runtime::execute_function(&ast, name, vec![], None);
        let mir_result =
            execution::execute_function(&mir, name, vec![], ExecutionLimits::default());
        let bytecode_result = execute(&bytecode, vec![], ExecutionLimits::default());
        let decoded_result = execute(&decoded, vec![], ExecutionLimits::default());

        assert_eq!(ast_result.is_ok(), succeeds, "AST case {name}");
        assert_eq!(mir_result.is_ok(), succeeds, "MIR case {name}");
        assert_eq!(bytecode_result.is_ok(), succeeds, "bytecode case {name}");
        assert_eq!(decoded_result.is_ok(), succeeds, "decoded case {name}");
        match (ast_result, mir_result, bytecode_result, decoded_result) {
            (Ok(expected), Ok(mir), Ok(bytecode), Ok(decoded)) => {
                assert_eq!(mir.value, expected, "MIR case {name}");
                assert_eq!(bytecode.value, expected, "bytecode case {name}");
                assert_eq!(decoded.value, expected, "decoded case {name}");
            }
            (Err(expected), Err(mir), Err(bytecode), Err(decoded)) => {
                assert_eq!(mir.message, expected.message, "MIR case {name}");
                assert_eq!(mir.span, expected.span, "MIR case {name}");
                assert_eq!(bytecode.message, expected.message, "bytecode case {name}");
                assert_eq!(bytecode.span, expected.span, "bytecode case {name}");
                assert_eq!(decoded.message, expected.message, "decoded case {name}");
                assert_eq!(decoded.span, expected.span, "decoded case {name}");
            }
            _ => panic!("backends disagreed on loop invariant outcome for {name}"),
        }
    }
}

#[test]
fn bytecode_executes_function_contracts_with_ast_and_mir_parity() {
    let source = "fn is_nonnegative(value: Int) -> Bool { return value >= 0 } fn increment(value: Int) -> Int requires { is_nonnegative(value) } ensures { result > value } { return value + 1 } fn broken(value: Int) -> Int ensures { result > value } { return value } fn main() {}";
    let ast = parse(&lex(source).unwrap()).unwrap();
    let mir = crate::lower(&ast).unwrap();
    let bytecode = compile(&mir, "increment").unwrap();
    let decoded = decode(&encode(&bytecode).unwrap()).unwrap();

    for program in [&bytecode, &decoded] {
        let actual = execute(program, vec![Value::Int(4)], ExecutionLimits::default()).unwrap();
        let expected =
            zelyra_runtime::execute_function(&ast, "increment", vec![Value::Int(4)], None).unwrap();
        let mir_result = execution::execute_function(
            &mir,
            "increment",
            vec![Value::Int(4)],
            ExecutionLimits::default(),
        )
        .unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, mir_result.value);
        assert_eq!(actual.value, Value::Int(5));

        let precondition =
            execute(program, vec![Value::Int(-1)], ExecutionLimits::default()).unwrap_err();
        let ast_precondition =
            zelyra_runtime::execute_function(&ast, "increment", vec![Value::Int(-1)], None)
                .unwrap_err();
        assert!(precondition.message.contains("precondition failed"));
        assert_eq!(precondition.message, ast_precondition.message);
        assert_eq!(precondition.span, ast_precondition.span);
    }

    let broken = compile(&mir, "broken").unwrap();
    let broken_decoded = decode(&encode(&broken).unwrap()).unwrap();
    let ast_postcondition =
        zelyra_runtime::execute_function(&ast, "broken", vec![Value::Int(4)], None).unwrap_err();
    for program in [&broken, &broken_decoded] {
        let postcondition =
            execute(program, vec![Value::Int(4)], ExecutionLimits::default()).unwrap_err();
        assert!(postcondition.message.contains("postcondition failed"));
        assert_eq!(postcondition.message, ast_postcondition.message);
        assert_eq!(postcondition.span, ast_postcondition.span);
    }
}

#[test]
fn bytecode_enforces_step_and_call_depth_budgets() {
    let source = r#"
fn descend(value: Int) -> Int {
    if value == 0 { return 0 } else { return descend(value - 1) }
}
fn main() { print(descend(3)) }
"#;
    let (_, bytecode) = compile_source(source, "descend");
    let step_error = execute(
        &bytecode,
        vec![Value::Int(3)],
        ExecutionLimits {
            max_steps: 3,
            max_stack_depth: 20,
        },
    )
    .unwrap_err();
    assert!(step_error.message.contains("step budget"));
    let depth_error = execute(
        &bytecode,
        vec![Value::Int(3)],
        ExecutionLimits {
            max_steps: 100,
            max_stack_depth: 2,
        },
    )
    .unwrap_err();
    assert!(depth_error.message.contains("call stack depth"));
}

#[test]
fn bytecode_verifier_checks_range_iterator_identity_and_targets() {
    let source = r#"
fn total(end: Int) -> Int {
    mutable result = 0
    for value in range(0, end) { result = result + value }
    return result
}
fn main() {}
"#;
    let (_, mut bytecode) = compile_source(source, "total");
    let next = bytecode.functions[0]
        .code
        .iter_mut()
        .find_map(|instruction| match &mut instruction.operation {
            BytecodeOp::RangeNext { iterator, .. } => Some(iterator),
            _ => None,
        })
        .expect("range loop emits a next instruction");
    *next = IteratorId(usize::MAX);
    let errors = verify_bytecode(&bytecode).unwrap_err();
    assert!(errors[0]
        .message
        .contains("advances an uninitialized or incompatible iterator"));

    let (_, mut bytecode) = compile_source(source, "total");
    let next = bytecode.functions[0]
        .code
        .iter_mut()
        .find_map(|instruction| match &mut instruction.operation {
            BytecodeOp::RangeNext { body, .. } => Some(body),
            _ => None,
        })
        .expect("range loop emits a next instruction");
    *next = usize::MAX;
    let errors = verify_bytecode(&bytecode).unwrap_err();
    assert!(errors[0].message.contains("falls outside the function"));
}

#[test]
fn bytecode_binary_format_is_deterministic_bounded_and_verified_on_load() {
    let (_, bytecode) = compile_source(
        "fn twice(x: Int) -> Int { if x > 0 { return x * 2 } else { return x } }\nfn main() {}",
        "twice",
    );
    let bytes = encode(&bytecode).expect("encoding succeeds");
    assert_eq!(bytes, encode(&bytecode).expect("repeat encoding succeeds"));
    let decoded = decode(&bytes).expect("round trip decodes");
    assert_eq!(
        execute(&decoded, vec![Value::Int(21)], ExecutionLimits::default())
            .unwrap()
            .value,
        Value::Int(42)
    );

    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(decode(&trailing).unwrap_err()[0]
        .message
        .contains("trailing bytes"));
    assert!(decode(&bytes[..bytes.len() - 1]).unwrap_err()[0]
        .message
        .contains("truncated"));
    assert!(decode_with_limits(
        &bytes,
        BytecodeFormatLimits {
            max_bytes: bytes.len() - 1,
            ..Default::default()
        }
    )
    .unwrap_err()[0]
        .message
        .contains("byte limit"));

    let mut bad_magic = bytes.clone();
    bad_magic[0] ^= 0xff;
    assert!(decode(&bad_magic).unwrap_err()[0].message.contains("magic"));
    let mut unsupported_version = bytes.clone();
    unsupported_version[8..10].copy_from_slice(&(BYTECODE_VERSION + 1).to_le_bytes());
    assert!(decode(&unsupported_version).unwrap_err()[0]
        .message
        .contains("version"));
    for previous in 1..BYTECODE_VERSION {
        let mut previous_version = bytes.clone();
        previous_version[8..10].copy_from_slice(&previous.to_le_bytes());
        assert!(decode(&previous_version).unwrap_err()[0]
            .message
            .contains("version"));
    }

    let mut invalid_branch = bytecode.clone();
    let branch = invalid_branch.functions[0]
        .code
        .iter_mut()
        .find(|i| {
            matches!(
                i.operation,
                BytecodeOp::Jump(_) | BytecodeOp::JumpIfFalse(_) | BytecodeOp::JumpIfTrue(_)
            )
        })
        .expect("conditional function has a branch");
    const MARKER: usize = 0x3344_5566;
    branch.span.start = MARKER;
    branch.span.end = MARKER;
    let mut malformed = encode(&invalid_branch).expect("the in-memory branch is valid");
    let span_offset = malformed
        .windows(8)
        .position(|window| window == MARKER.to_le_bytes())
        .expect("marker source span was serialized");
    let opcode_offset = span_offset + 36;
    malformed[opcode_offset] = 13; // Jump
    malformed[opcode_offset + 1..opcode_offset + 9].copy_from_slice(&(u64::MAX).to_le_bytes());
    let rejected = decode(&malformed).unwrap_err();
    assert_eq!(rejected[0].code, "E-MIR-BYTECODE-002");
    assert!(
        rejected[0].message.contains("control flow") || rejected[0].message.contains("target"),
        "{rejected:?}"
    );
}
