use std::collections::{HashMap, HashSet};
use zelyra_ast::{Program, Type};
use zelyra_runtime::Value as RuntimeValue;

use crate::{
    audit_insert_queries_for, form_audit_details, runtime_value_for_column, runtime_value_for_type,
    tenant_access_queries, FormRoute, TenantContext,
};

/// Enforce `requires role(...)` and `requires four_eyes(<creator_col>)` rules
/// on a declarative `states` transition under the `SELECT ... FOR UPDATE` lock.
/// Fails closed when authentication, creator identity, or required role is absent.
pub(crate) fn evaluate_state_transition_policy(
    machine: &zelyra_ast::TableStateMachineDef,
    from_state: &str,
    to_state: &str,
    row_values: &HashMap<String, String>,
    actor_user_id: Option<i64>,
    actor_roles: &[String],
) -> Option<String> {
    let rule = machine
        .transitions
        .iter()
        .find(|rule| rule.from == from_state && rule.to.iter().any(|target| target == to_state))?;
    if !rule.required_roles.is_empty() {
        let has_role = rule
            .required_roles
            .iter()
            .any(|required| actor_roles.iter().any(|role| role == required));
        if !has_role {
            return Some(format!(
                "state transition from `{from_state}` to `{to_state}` requires role: {}",
                rule.required_roles.join(" or ")
            ));
        }
    }
    if let Some(creator_col) = &rule.four_eyes_column {
        let Some(actor_id) = actor_user_id else {
            return Some(format!(
                "four-eyes transition from `{from_state}` to `{to_state}` requires an authenticated user"
            ));
        };
        let Some(creator_value) = row_values
            .get(creator_col)
            .map(String::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Some(format!(
                "four-eyes check failed: creator column `{creator_col}` is missing on this record"
            ));
        };
        let actor_str = actor_id.to_string();
        if creator_value == actor_str
            || creator_value
                .parse::<i64>()
                .ok()
                .is_some_and(|creator_id| creator_id == actor_id)
        {
            return Some(format!(
                "four-eyes violation: user `{actor_id}` cannot approve their own record (`{creator_col}`)"
            ));
        }
    }
    None
}

pub(crate) fn evaluate_state_transition_guard(
    program: &Program,
    mappings: &[zelyra_ast::StateTransitionGuardFunction],
    table_name: &str,
    column_name: &str,
    from_state: &str,
    to_state: &str,
    row_values: &HashMap<String, String>,
) -> Result<Option<String>, String> {
    let Some(table_index) = program
        .tables
        .iter()
        .position(|table| table.name == table_name)
    else {
        return Ok(None);
    };
    let table = &program.tables[table_index];
    let Some(machine_index) = table
        .state_machines
        .iter()
        .position(|machine| machine.column == column_name)
    else {
        return Ok(None);
    };
    let machine = &table.state_machines[machine_index];
    let Some(transition_index) = machine.transitions.iter().position(|rule| {
        rule.from == from_state
            && rule.to.iter().any(|target| target == to_state)
            && rule.guard.is_some()
    }) else {
        return Ok(None);
    };
    let Some(mapping) = mappings.iter().find(|mapping| {
        mapping.table_index == table_index
            && mapping.machine_index == machine_index
            && mapping.transition_index == transition_index
    }) else {
        return Ok(None);
    };
    let mut args = Vec::new();
    for column in table.columns.iter().filter(|column| !column.auto) {
        let value = runtime_value_for_column(
            program,
            column,
            row_values.get(&column.name).map(String::as_str),
        )?;
        args.push(value);
    }
    let policy = zelyra_runtime::RuntimePolicy {
        step_budget: Some(10_000),
        ..zelyra_runtime::RuntimePolicy::default()
    };
    let result = zelyra_runtime::execute_function_with_capabilities_and_policies(
        program,
        &mapping.function_name,
        args,
        None,
        None,
        Some(&policy),
    )
    .map_err(|error| error.message)?;
    match result {
        RuntimeValue::Bool(true) => Ok(None),
        RuntimeValue::Bool(false) => Ok(Some(
            machine.transitions[transition_index]
                .guard
                .as_ref()
                .map(|guard| guard.message.clone())
                .unwrap_or_else(|| "state transition guard rejected this change".into()),
        )),
        other => Err(format!(
            "state transition guard `{}` must return Bool, got {}",
            mapping.function_name,
            other.ty()
        )),
    }
}

pub(crate) fn runtime_value_for_form_field(
    program: &Program,
    table: Option<&zelyra_ast::TableDef>,
    field_name: &str,
    field_ty: &Type,
    submitted: Option<&str>,
) -> Result<RuntimeValue, String> {
    if let Some(column) = table.and_then(|table| {
        table
            .columns
            .iter()
            .find(|column| column.name == field_name)
    }) {
        if column.ty == *field_ty {
            if submitted.is_none() && matches!(field_ty, Type::Bool) && column.default.is_none() {
                return Ok(RuntimeValue::Bool(false));
            }
            return runtime_value_for_column(program, column, submitted);
        }
    }
    match (field_ty, submitted) {
        (Type::Option(_), None | Some("")) => Ok(RuntimeValue::Option(None)),
        (Type::Option(inner), Some(value)) => Ok(RuntimeValue::Option(Some(Box::new(
            runtime_value_for_type(program, inner, value)?,
        )))),
        (Type::Bool, None) => Ok(RuntimeValue::Bool(false)),
        (_, Some(value)) => runtime_value_for_type(program, field_ty, value),
        (_, None) => Err(format!("missing value for typed form field `{field_name}`")),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_typed_form_action(
    form: &FormRoute,
    action: &zelyra_ast::FormAction,
    values: &HashMap<String, String>,
    path_params: &HashMap<String, String>,
    database_url: &str,
    actor_user_id: Option<i64>,
    tenant_context: Option<&TenantContext>,
    program: Option<&Program>,
    mappings: &[zelyra_ast::FormActionFunction],
    grants: Option<&HashSet<String>>,
    policy: Option<&zelyra_runtime::RuntimePolicy>,
) -> Result<(), String> {
    let program = program.ok_or_else(|| "E-FORM-002: action program is unavailable".to_owned())?;
    let form_index = program
        .forms
        .iter()
        .position(|candidate| candidate.name == form.form.name)
        .ok_or_else(|| format!("E-FORM-002: form `{}` is unavailable", form.form.name))?;
    let action_index = program.forms[form_index]
        .actions
        .iter()
        .position(|candidate| candidate.name == action.name)
        .ok_or_else(|| format!("E-FORM-002: form action `{}` is unavailable", action.name))?;
    let mapping = mappings
        .iter()
        .find(|mapping| mapping.form_index == form_index && mapping.action_index == action_index)
        .ok_or_else(|| format!("E-FORM-002: form action `{}` was not lowered", action.name))?;
    let table = form
        .form
        .table
        .as_deref()
        .and_then(|name| program.tables.iter().find(|table| table.name == name));
    let mut args = Vec::with_capacity(
        mapping.field_params.len()
            + mapping.path_params.len()
            + usize::from(mapping.has_session_param),
    );
    for (field_name, field_ty) in &mapping.field_params {
        args.push(runtime_value_for_form_field(
            program,
            table,
            field_name,
            field_ty,
            values.get(field_name).map(String::as_str),
        )?);
    }
    for (param_name, param_ty) in &mapping.path_params {
        let raw = path_params.get(param_name).ok_or_else(|| {
            format!("missing route parameter `{param_name}` for typed form action")
        })?;
        args.push(runtime_value_for_type(program, param_ty, raw)?);
    }
    if mapping.has_session_param {
        args.push(RuntimeValue::Object {
            type_name: mapping.session_type_name.clone(),
            type_args: Vec::new(),
            fields: [
                (
                    "tenant_id".into(),
                    RuntimeValue::Option(
                        tenant_context.map(|ctx| Box::new(RuntimeValue::Int(ctx.tenant_id))),
                    ),
                ),
                (
                    "user_id".into(),
                    RuntimeValue::Option(actor_user_id.map(|id| Box::new(RuntimeValue::Int(id)))),
                ),
            ]
            .into_iter()
            .collect(),
        });
    }
    let is_postgres = zelyra_database::is_postgres_url(database_url);
    zelyra_database::with_decision_transaction(database_url, |transaction| {
        if let Some(context) = tenant_context {
            let mut authorized = false;
            for query in tenant_access_queries(context, is_postgres, Some(context.tenant_id), true)
            {
                if !transaction.execute(&query)?.rows.is_empty() {
                    authorized = true;
                    break;
                }
            }
            if !authorized {
                return Err(zelyra_database::DatabaseError {
                    message:
                        "ZELYRA_TENANT_DENIED: tenant ownership or membership is no longer valid"
                            .into(),
                });
            }
        }
        let mut query = |query: &zelyra_database::Query| {
            transaction
                .execute(query)
                .map_err(|error| error.to_string())
        };
        let outcome = zelyra_runtime::execute_function_captured_with_transactional_query(
            program,
            &mapping.execute_function,
            args,
            database_url,
            grants,
            policy,
            &mut query,
        );
        if let Some(error) = outcome.error {
            return Err(zelyra_database::DatabaseError {
                message: format!("typed form action failed: {}", error.message),
            });
        }
        if let (Some(audit_table), Some(audit_event)) =
            (form.audit_table.as_deref(), form.audit_event.as_deref())
        {
            let (record_id, mut details) =
                form_audit_details(form, audit_event, path_params, None, values);
            if let Some(context) = tenant_context {
                details.push_str(&format!(";tenant_id={}", context.tenant_id));
            }
            for query in audit_insert_queries_for(
                audit_table,
                form.audit_chain,
                actor_user_id,
                audit_event,
                record_id,
                &details,
                is_postgres,
            )
            .map_err(|message| zelyra_database::DatabaseError { message })?
            {
                transaction.execute(&query)?;
            }
        }
        Ok(())
    })
    .map_err(|error| error.message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zelyra_ast::{Span, TableStateMachineDef, TableStateTransitionDef};

    fn sample_machine() -> TableStateMachineDef {
        TableStateMachineDef {
            column: "status".into(),
            transitions: vec![
                TableStateTransitionDef {
                    from: "draft".into(),
                    to: vec!["submitted".into()],
                    is_final: false,
                    guard: None,
                    required_roles: Vec::new(),
                    four_eyes_column: None,
                    span: Span::default(),
                },
                TableStateTransitionDef {
                    from: "submitted".into(),
                    to: vec!["approved".into()],
                    is_final: false,
                    guard: None,
                    required_roles: vec!["approver".into()],
                    four_eyes_column: Some("created_by".into()),
                    span: Span::default(),
                },
                TableStateTransitionDef {
                    from: "approved".into(),
                    to: Vec::new(),
                    is_final: true,
                    guard: None,
                    required_roles: Vec::new(),
                    four_eyes_column: None,
                    span: Span::default(),
                },
            ],
            span: Span::default(),
        }
    }

    #[test]
    fn four_eyes_rejects_self_approval_unauthenticated_and_missing_creator_and_allows_second_approver(
    ) {
        let machine = sample_machine();
        let mut row = HashMap::new();
        row.insert("created_by".into(), "42".into());

        // Self-approval by creator (user 42) with approver role -> REJECTED!
        let self_approval = evaluate_state_transition_policy(
            &machine,
            "submitted",
            "approved",
            &row,
            Some(42),
            &["approver".into()],
        );
        assert!(
            self_approval
                .as_deref()
                .is_some_and(|msg| msg.contains("four-eyes violation")),
            "{self_approval:?}"
        );

        // Missing role for user 99 -> REJECTED!
        let missing_role = evaluate_state_transition_policy(
            &machine,
            "submitted",
            "approved",
            &row,
            Some(99),
            &["viewer".into()],
        );
        assert!(
            missing_role
                .as_deref()
                .is_some_and(|msg| msg.contains("requires role: approver")),
            "{missing_role:?}"
        );

        // Unauthenticated actor with role -> REJECTED!
        let unauth = evaluate_state_transition_policy(
            &machine,
            "submitted",
            "approved",
            &row,
            None,
            &["approver".into()],
        );
        assert!(
            unauth
                .as_deref()
                .is_some_and(|msg| msg.contains("requires an authenticated user")),
            "{unauth:?}"
        );

        // Missing creator column on record -> REJECTED fail-closed!
        let empty_row = HashMap::new();
        let missing_creator = evaluate_state_transition_policy(
            &machine,
            "submitted",
            "approved",
            &empty_row,
            Some(99),
            &["approver".into()],
        );
        assert!(
            missing_creator
                .as_deref()
                .is_some_and(|msg| msg.contains("creator column `created_by` is missing")),
            "{missing_creator:?}"
        );

        // Second authorized user (user 99 with approver role) -> ALLOWED!
        let allowed = evaluate_state_transition_policy(
            &machine,
            "submitted",
            "approved",
            &row,
            Some(99),
            &["approver".into()],
        );
        assert!(allowed.is_none(), "{allowed:?}");
    }
}
