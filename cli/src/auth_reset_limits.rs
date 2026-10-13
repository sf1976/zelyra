//! Validate the explicitly declared storage for persistent password-reset limits.

use super::{diagnostic_with_span, Schema};
use zelyra_ast::AuthDef;

pub(super) fn validate(path: &str, auth: &AuthDef, schema: &Schema) -> bool {
    let report = |code, message: &str| diagnostic_with_span(path, code, message, auth.span);
    let Some(reset_table) = auth.reset_tokens_table.as_deref() else {
        if auth.reset_rate_limits_table.is_some() {
            report(
                "E-AUTH-042",
                "reset_rate_limits requires reset_tokens to be enabled",
            );
            return false;
        }
        return true;
    };
    let Some(table_name) = auth.reset_rate_limits_table.as_deref() else {
        report(
            "E-AUTH-039",
            "password reset requires an explicit reset_rate_limits table for persistent throttling",
        );
        return false;
    };
    if [
        Some(auth.table.as_str()),
        Some(reset_table),
        auth.session_table.as_deref(),
        auth.membership_table.as_deref(),
        auth.permissions_table.as_deref(),
        auth.roles_table.as_deref(),
        auth.role_permissions_table.as_deref(),
        auth.audit_table.as_deref(),
        auth.mfa_table.as_deref(),
        auth.mfa_recovery_table.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|other| table_name == other)
    {
        report("E-AUTH-039", "password reset rate limits must use a separate table from other authentication storage");
        return false;
    }
    let Some(table) = schema.tables.iter().find(|table| table.name == table_name) else {
        report(
            "E-AUTH-039",
            &format!(
                "authentication refers to unknown password reset rate-limit table {table_name}"
            ),
        );
        return false;
    };
    let key = table
        .columns
        .iter()
        .find(|column| column.name == "bucket_key");
    let window = table
        .columns
        .iter()
        .find(|column| column.name == "window_started");
    let count = table
        .columns
        .iter()
        .find(|column| column.name == "request_count");
    let blocked = table
        .columns
        .iter()
        .find(|column| column.name == "blocked_until");
    let unique_key = key.is_some_and(|column| {
        column.primary_key
            || column.unique
            || table
                .indexes
                .iter()
                .chain(&table.uniques)
                .any(|index| index.unique && index.columns == ["bucket_key"])
    });
    let compatible_columns = unique_key
        && key.is_some_and(|column| {
            column.sql_type.eq_ignore_ascii_case("varchar(64)") && !column.nullable && !column.auto
        })
        && window.is_some_and(|column| {
            column.sql_type.eq_ignore_ascii_case("datetime(3)") && !column.nullable && !column.auto
        })
        && count.is_some_and(|column| {
            column.sql_type.eq_ignore_ascii_case("bigint") && !column.nullable && !column.auto
        })
        && blocked.is_some_and(|column| {
            column.sql_type.eq_ignore_ascii_case("datetime(3)") && column.nullable && !column.auto
        });
    let mut valid = true;
    if !compatible_columns {
        report("E-AUTH-040", &format!("password reset rate-limit table {table_name} requires unique non-null bucket_key VARCHAR(64), non-null window_started DATETIME(3) and request_count BIGINT, and nullable blocked_until DATETIME(3); these columns cannot be auto-generated"));
        valid = false;
    }
    if !table.indexes.iter().chain(&table.uniques).any(|index| {
        index
            .columns
            .first()
            .is_some_and(|column| column == "window_started")
    }) {
        report("E-AUTH-041", &format!("password reset rate-limit table {table_name} requires an index beginning with window_started for bounded cleanup"));
        valid = false;
    }
    valid
}
