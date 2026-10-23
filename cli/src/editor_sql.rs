use super::{database_cli, database_url_for_project, EditorState};
use serde_json::{json, Value};
use std::time::Instant;
use zelyra_database::Backend;

const MAX_QUERY_BYTES: usize = 16_384;
const MAX_RESULT_ROWS: usize = 200;
const MAX_RESULT_COLUMNS: usize = 50;
const MAX_CELL_CHARACTERS: usize = 512;

fn prepare_read_only_query(
    sql: &str,
    requested_limit: Option<u64>,
    postgres: bool,
) -> Result<(String, usize), (u16, String)> {
    let sql = sql.trim();
    if sql.is_empty() || sql.len() > MAX_QUERY_BYTES {
        return Err((400, "SQL must contain 1 to 16,384 bytes".into()));
    }
    if !database_cli::single_sql_statement(sql, postgres) {
        return Err((400, "Studio accepts exactly one SQL statement".into()));
    }
    if !database_cli::read_only_statement(sql, postgres)
        || !matches!(
            database_cli::first_sql_keyword(sql).as_str(),
            "SELECT" | "WITH"
        )
        || sql
            .split_whitespace()
            .any(|word| word.eq_ignore_ascii_case("INTO"))
    {
        return Err((
            403,
            "Studio SQL currently accepts read-only SELECT and WITH queries only".into(),
        ));
    }

    let limit = requested_limit
        .unwrap_or(MAX_RESULT_ROWS as u64)
        .min(MAX_RESULT_ROWS as u64)
        .max(1) as usize;
    let normalized = database_cli::strip_sql_terminator(sql, postgres);
    let bounded_sql = format!(
        "SELECT * FROM (\n{normalized}\n) AS _zelyra_studio_result LIMIT {}",
        limit + 1
    );
    Ok((bounded_sql, limit))
}

pub(super) fn query(
    state: &EditorState,
    request: &Value,
) -> Result<(u16, &'static str, String), (u16, String)> {
    let sql = request
        .get("sql")
        .and_then(Value::as_str)
        .ok_or((400, "sql is required".into()))?;
    let entry = state.root.join("main.zyl");
    let declared = database_cli::load_schema(&entry.to_string_lossy()).map_err(|_| {
        (
            422,
            "the project database declaration could not be loaded".into(),
        )
    })?;
    if declared.backend() == Backend::Sqlite {
        return Err((
            501,
            "SQL workbench currently supports MariaDB and PostgreSQL projects".into(),
        ));
    }
    let database_name = declared
        .database
        .as_ref()
        .map(|database| database.name.as_str());
    let database_url = database_url_for_project(&state.root, database_name)
        .map_err(|_| (422, "the project environment file could not be read".into()))?
        .ok_or((422, "database connection is not configured".into()))?;
    let postgres = zelyra_database::is_postgres_url(&database_url);
    let (bounded_sql, limit) =
        prepare_read_only_query(sql, request.get("limit").and_then(Value::as_u64), postgres)?;

    let started = Instant::now();
    let bounded_result = zelyra_database::execute_read_only_query_with_cell_limit(
        &database_url,
        &bounded_sql,
        Vec::new(),
        MAX_CELL_CHARACTERS,
    )
    .map_err(|_| {
        (
            422,
            "query failed; check the SQL, connection, and database permissions".into(),
        )
    })?;
    let database_cells_truncated = bounded_result.cells_truncated;
    let result = bounded_result.result;
    let elapsed_ms = started.elapsed().as_millis();
    let rows_truncated = result.rows.len() > limit;
    let columns_truncated = result.columns.len() > MAX_RESULT_COLUMNS;
    let mut cells_truncated = database_cells_truncated;
    let columns = result
        .columns
        .iter()
        .take(MAX_RESULT_COLUMNS)
        .cloned()
        .collect::<Vec<_>>();
    let rows = result
        .rows
        .iter()
        .take(limit)
        .map(|row| {
            row.iter()
                .take(MAX_RESULT_COLUMNS)
                .map(|cell| {
                    let (cell, was_truncated) = truncate_cell(cell);
                    cells_truncated |= was_truncated;
                    cell
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let nulls = result
        .nulls
        .iter()
        .take(limit)
        .map(|row| {
            row.iter()
                .take(MAX_RESULT_COLUMNS)
                .copied()
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let row_count = rows.len();
    let response = json!({
        "readOnly": true,
        "columns": columns,
        "rows": rows,
        "nulls": nulls,
        "rowCount": row_count,
        "limit": limit,
        "columnLimit": MAX_RESULT_COLUMNS,
        "truncated": rows_truncated || columns_truncated || cells_truncated,
        "rowsTruncated": rows_truncated,
        "columnsTruncated": columns_truncated,
        "cellsTruncated": cells_truncated,
        "cellLimit": MAX_CELL_CHARACTERS,
        "elapsedMs": elapsed_ms,
    });
    Ok((200, "application/json; charset=utf-8", response.to_string()))
}

fn truncate_cell(value: &str) -> (String, bool) {
    let mut characters = value.chars();
    let text = characters
        .by_ref()
        .take(MAX_CELL_CHARACTERS)
        .collect::<String>();
    if characters.next().is_some() {
        (format!("{text}…"), true)
    } else {
        (text, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_cells_are_bounded_without_splitting_unicode() {
        let cell = "ä".repeat(MAX_CELL_CHARACTERS + 4);
        let (bounded, truncated) = truncate_cell(&cell);
        assert_eq!(bounded.chars().count(), MAX_CELL_CHARACTERS + 1);
        assert!(bounded.ends_with('…'));
        assert!(truncated);
        assert_eq!(truncate_cell("short"), ("short".into(), false));
    }

    #[test]
    fn sql_workbench_accepts_only_one_read_only_select_family_statement() {
        for query in ["SELECT 1", "WITH item AS (SELECT 1) SELECT * FROM item"] {
            assert!(database_cli::single_sql_statement(query, false));
            assert!(database_cli::read_only_statement(query, false));
        }
        for query in [
            "DELETE FROM customers",
            "SELECT 1; DELETE FROM customers",
            "DROP TABLE customers",
            "SELECT id INTO @customer_id FROM customers",
            "SELECT SLEEP (30)",
            "SELECT SLEEP/* comment */(30)",
            r#"SELECT 'a\'b' AS x, SLEEP(30)"#,
            "SELECT LOAD_FILE ('/etc/passwd')",
            "SELECT LOAD_FILE/**/('/etc/passwd')",
            "SELECT /*!80000 SLEEP(30) */ 1",
            "SELECT pg_notify('channel', 'payload')",
            "SELECT pg_advisory_lock_shared(1)",
            "SELECT * FROM dblink('remote', 'DELETE FROM customers') AS result(id INT)",
            "SELECT LAST_INSERT_ID(123)",
        ] {
            assert!(prepare_read_only_query(query, None, false).is_err());
        }
        assert!(prepare_read_only_query("SELECT pg_sleep--hidden\n(30)", None, true).is_err());
    }

    #[test]
    fn read_only_queries_are_wrapped_with_a_hard_result_limit() {
        let (bounded, limit) =
            prepare_read_only_query("SELECT id FROM customers;", Some(9000), false).unwrap();
        assert_eq!(limit, MAX_RESULT_ROWS);
        assert!(bounded.ends_with("LIMIT 201"));
        assert!(bounded.contains("SELECT id FROM customers"));
    }
}
