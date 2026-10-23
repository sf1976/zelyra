//! A scoped decision-making transaction with backend-neutral query results.
use super::*;

trait Executor: Send {
    fn allows_generated_audit_init(&self) -> bool {
        false
    }
    fn execute(&mut self, query: &Query) -> Result<QueryResult, DatabaseError>;
}

/// One borrowed connection. This handle cannot escape the transaction closure.
pub struct DecisionTransaction<'a> {
    executor: &'a mut dyn Executor,
    failed: Option<DatabaseError>,
}
impl DecisionTransaction<'_> {
    pub fn execute(&mut self, query: &Query) -> Result<QueryResult, DatabaseError> {
        if let Some(error) = &self.failed {
            return Err(error.clone());
        }
        let result = remaining_query_budget().and_then(|_| {
            let command = sql::analyze_table_access(&query.sql, &[]).0;
            let audit_init = self.executor.allows_generated_audit_init()
                && query.sql.trim().eq_ignore_ascii_case("SET @zelyra_prev_hash = '';");
            if command.is_none() && !audit_init {
                return Err(DatabaseError { message: "decision transactions accept only validated SELECT/INSERT/UPDATE/DELETE statements; transaction control and implicit-commit DDL are not allowed".into() });
            }
            self.executor.execute(query)
        });
        if let Err(error) = &result {
            self.failed = Some(error.clone());
        }
        result
    }
}
impl Executor for MariaDbTransaction<'_> {
    fn allows_generated_audit_init(&self) -> bool {
        true
    }
    fn execute(&mut self, query: &Query) -> Result<QueryResult, DatabaseError> {
        MariaDbTransaction::execute(self, query)
    }
}

/// Run conditional reads/writes on one connection; Ok commits, Err rolls back.
/// An expired request deadline prevents commit. No statement is retried.
pub fn with_decision_transaction<T>(
    database_url: &str,
    operation: impl FnOnce(&mut DecisionTransaction<'_>) -> Result<T, DatabaseError>,
) -> Result<T, DatabaseError> {
    remaining_query_budget()?;
    if !is_postgres_url(database_url) {
        return with_mariadb_transaction(database_url, |transaction| {
            let value = run_operation(transaction, operation)?;
            remaining_query_budget()?;
            Ok(value)
        });
    }
    let settings = MariaDbTimeouts::from_env()?;
    let tls = MariaDbTlsSettings::from_env()?;
    let pool = postgres_pool(database_url, settings, tls)?;
    let wait = postgres_pool_wait_timeout(settings)?;
    match &pool {
        PostgresPool::Plain(pool) => {
            let mut client = pool.get_timeout(wait).map_err(postgres_pool_error)?;
            on_postgres_client(&mut client, settings, operation)
        }
        PostgresPool::Tls(pool) => {
            let mut client = pool.get_timeout(wait).map_err(postgres_pool_error)?;
            on_postgres_client(&mut client, settings, operation)
        }
    }
}

fn run_operation<T>(
    executor: &mut dyn Executor,
    operation: impl FnOnce(&mut DecisionTransaction<'_>) -> Result<T, DatabaseError>,
) -> Result<T, DatabaseError> {
    let mut transaction = DecisionTransaction {
        executor,
        failed: None,
    };
    let result = operation(&mut transaction);
    match transaction.failed {
        Some(error) => Err(error),
        None => result,
    }
}

struct PostgresDecision<'a> {
    transaction: postgres::Transaction<'a>,
    settings: MariaDbTimeouts,
}
impl PostgresDecision<'_> {
    fn set_timeout(&mut self) -> Result<(), DatabaseError> {
        let millis = postgres_statement_timeout_millis(self.settings)?;
        self.transaction
            .batch_execute(&format!("SET LOCAL statement_timeout = {millis}"))
            .map_err(postgres_driver_error)
    }
}
impl Executor for PostgresDecision<'_> {
    fn execute(&mut self, query: &Query) -> Result<QueryResult, DatabaseError> {
        self.set_timeout()?;
        let (sql, params) = prepare_postgres_query(query)?;
        execute_prepared_postgres_query(&mut self.transaction, &sql, &params, None, None)
            .map(|(result, _)| result)
    }
}
fn on_postgres_client<T>(
    client: &mut PostgresClient,
    settings: MariaDbTimeouts,
    operation: impl FnOnce(&mut DecisionTransaction<'_>) -> Result<T, DatabaseError>,
) -> Result<T, DatabaseError> {
    remaining_query_budget()?;
    let mut executor = PostgresDecision {
        transaction: client.transaction().map_err(postgres_driver_error)?,
        settings,
    };
    let value = run_operation(&mut executor, operation)?;
    executor.set_timeout()?;
    executor
        .transaction
        .commit()
        .map_err(postgres_driver_error)?;
    Ok(value)
    // postgres::Transaction rolls back on every uncommitted Drop, including unwind.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[test]
    fn expired_decision_deadline_rejects_before_connection_access() {
        let _deadline = set_query_deadline(Instant::now() - Duration::from_millis(1));
        let result =
            with_decision_transaction::<()>("postgres://invalid@127.0.0.1:1/no_database", |_| {
                panic!("must not run")
            });
        assert!(result.unwrap_err().message.contains("deadline"));
    }

    #[test]
    #[ignore = "requires DATABASE_URL identifying a disposable test database"]
    fn decision_transactions_commit_rollback_unwind_and_deadline_on_native_backend() {
        let url = env::var("DATABASE_URL").expect("disposable DATABASE_URL required");
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let table = format!("zelyra_decision_{suffix}");
        execute_query(
            &url,
            &format!("CREATE TABLE {table} (id BIGINT PRIMARY KEY, value BIGINT NOT NULL)"),
            Vec::new(),
        )
        .unwrap();
        struct Cleanup<'a> {
            url: &'a str,
            table: String,
        }
        impl Drop for Cleanup<'_> {
            fn drop(&mut self) {
                let _ = execute_query(self.url, &format!("DROP TABLE {}", self.table), Vec::new());
            }
        }
        let _cleanup = Cleanup {
            url: &url,
            table: table.clone(),
        };
        execute_query(
            &url,
            &format!("INSERT INTO {table} (id,value) VALUES (1,0)"),
            Vec::new(),
        )
        .unwrap();
        let update = |value| Query {
            sql: format!("UPDATE {table} SET value=:value WHERE id=1"),
            params: vec![("value".into(), QueryValue::Int(value))],
        };
        let read = || {
            execute_query(
                &url,
                &format!("SELECT value FROM {table} WHERE id=1"),
                Vec::new(),
            )
            .unwrap()
            .rows[0][0]
                .clone()
        };
        with_decision_transaction(&url, |transaction| {
            let selected = transaction.execute(&Query {
                sql: format!("SELECT value FROM {table} WHERE id=1 FOR UPDATE"),
                params: Vec::new(),
            })?;
            assert_eq!(selected.rows, [vec!["0".to_owned()]]);
            assert_eq!(selected.nulls, [vec![false]]);
            transaction.execute(&update(1))?;
            let selected = transaction.execute(&Query {
                sql: format!("SELECT value FROM {table} WHERE id=1"),
                params: Vec::new(),
            })?;
            assert_eq!(
                selected.rows,
                [vec!["1".to_owned()]],
                "read must see the same transaction's write"
            );
            Ok(())
        })
        .unwrap();
        assert_eq!(read(), "1");
        let denied = with_decision_transaction::<()>(&url, |transaction| {
            transaction.execute(&update(2))?;
            Err(DatabaseError {
                message: "decision denied".into(),
            })
        });
        assert_eq!(denied.unwrap_err().message, "decision denied");
        assert_eq!(read(), "1");
        let sql_failure = with_decision_transaction::<()>(&url, |transaction| {
            transaction.execute(&update(3))?;
            transaction.execute(&Query {
                sql: format!("INSERT INTO {table} (id,value) VALUES (1,99)"),
                params: Vec::new(),
            })?;
            Ok(())
        });
        assert!(sql_failure.is_err());
        assert_eq!(read(), "1");
        assert!(catch_unwind(AssertUnwindSafe(|| {
            let _ = with_decision_transaction::<()>(&url, |transaction| {
                transaction.execute(&update(4))?;
                panic!("controlled transaction unwind");
            });
        }))
        .is_err());
        assert_eq!(
            read(),
            "1",
            "unwind must not leak a transaction into the pool"
        );
        let expired = {
            let _deadline = set_query_deadline(Instant::now() + Duration::from_millis(250));
            with_decision_transaction(&url, |transaction| {
                transaction.execute(&update(5))?;
                std::thread::sleep(Duration::from_millis(300));
                Ok(())
            })
        };
        assert!(expired.unwrap_err().message.contains("deadline"));
        assert_eq!(read(), "1");
        let expired_statement = {
            let _deadline = set_query_deadline(Instant::now() + Duration::from_millis(250));
            with_decision_transaction(&url, |transaction| {
                transaction.execute(&update(6))?;
                std::thread::sleep(Duration::from_millis(300));
                transaction.execute(&update(7))
            })
        };
        assert!(expired_statement.unwrap_err().message.contains("deadline"));
        assert_eq!(read(), "1");
        let swallowed = with_decision_transaction(&url, |transaction| {
            transaction.execute(&update(9))?;
            let _ignored = transaction.execute(&Query {
                sql: format!("INSERT INTO {table} (id,value) VALUES (1,99)"),
                params: Vec::new(),
            });
            Ok(())
        });
        assert!(
            swallowed.is_err(),
            "a swallowed SQL error must poison the decision transaction"
        );
        assert_eq!(read(), "1");
        for control in [
            "COMMIT".to_owned(),
            "ROLLBACK".to_owned(),
            format!("ALTER TABLE {table} ADD COLUMN escaped BIGINT"),
            "SET autocommit=1".to_owned(),
        ] {
            let escaped = with_decision_transaction(&url, |transaction| {
                transaction.execute(&update(10))?;
                let _ignored = transaction.execute(&Query {
                    sql: control.clone(),
                    params: Vec::new(),
                });
                Ok(())
            });
            assert!(
                escaped.is_err(),
                "transaction control/DDL must not escape the closure"
            );
            assert_eq!(read(), "1");
        }
        if is_postgres_url(&url) {
            let started = Instant::now();
            let slow = {
                let _deadline = set_query_deadline(Instant::now() + Duration::from_millis(250));
                with_decision_transaction(&url, |transaction| {
                    transaction.execute(&update(11))?;
                    transaction.execute(&Query {
                        sql: "SELECT pg_sleep(2)".into(),
                        params: Vec::new(),
                    })
                })
            };
            assert!(slow.is_err());
            assert!(
                started.elapsed() < Duration::from_secs(1),
                "PG statement timeout must cancel the long query"
            );
            assert_eq!(read(), "1");
        }
        with_decision_transaction(&url, |transaction| {
            transaction.execute(&update(8))?;
            Ok(())
        })
        .unwrap();
        assert_eq!(
            read(),
            "8",
            "pool must remain reusable after rollback/unwind/deadline"
        );
        println!("native decisions: commit, same-connection visibility, denied decision, SQL error, unwind, pre-statement/pre-commit deadline and reuse passed");
    }
}
