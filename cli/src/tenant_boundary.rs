use std::collections::{HashMap, HashSet};
use zelyra_database::Schema;

use crate::diagnostic_with_span;

pub(crate) fn validate_tenant_access_boundaries(
    path: &str,
    program: &zelyra_ast::Program,
    schema: &Schema,
) -> bool {
    let scoped_tenant_tables = program
        .tables
        .iter()
        .filter_map(|table| {
            table
                .tenant_scope
                .as_ref()
                .map(|scope| (table.name.clone(), scope.column.clone()))
        })
        .collect::<HashMap<_, _>>();
    let tenant_tables = program
        .cruds
        .iter()
        .filter(|crud| crud.tenant_column.is_some())
        .map(|crud| crud.table.clone())
        .chain(scoped_tenant_tables.keys().cloned())
        .collect::<HashSet<_>>();
    let recipient_tables = program
        .cruds
        .iter()
        .filter(|crud| crud.recipient_column.is_some())
        .map(|crud| crud.table.clone())
        .collect::<HashSet<_>>();
    if tenant_tables.is_empty() && recipient_tables.is_empty() {
        return true;
    }
    let protected_tables = program
        .auth
        .iter()
        .filter_map(|auth| auth.membership_table.clone())
        .chain(tenant_tables.iter().cloned())
        .chain(recipient_tables.iter().cloned())
        .collect::<HashSet<_>>();
    let mut valid = true;
    for table in program
        .tables
        .iter()
        .filter(|table| table.tenant_scope.is_some())
    {
        if program.auth.iter().any(|auth| {
            auth.membership_table
                .as_deref()
                .is_some_and(|membership| membership.eq_ignore_ascii_case(&table.name))
        }) {
            diagnostic_with_span(
                path,
                "E-TENANT-001",
                "the tenant membership table cannot itself be tenant-scoped",
                table.tenant_scope.as_ref().unwrap().span,
            );
            valid = false;
        }
    }
    let known_tables = schema
        .tables
        .iter()
        .map(|table| table.name.clone())
        .collect::<Vec<_>>();
    let membership_tables = program
        .auth
        .iter()
        .filter_map(|auth| auth.membership_table.clone())
        .collect::<HashSet<_>>();
    let tenant_requirements = program
        .tables
        .iter()
        .filter_map(|table| {
            let scope = table.tenant_scope.as_ref()?;
            if membership_tables
                .iter()
                .any(|m| m.eq_ignore_ascii_case(&table.name))
                || recipient_tables
                    .iter()
                    .any(|r| r.eq_ignore_ascii_case(&table.name))
            {
                return None;
            }
            Some(zelyra_database::sql::TenantScopeRequirement {
                table: table.name.clone(),
                column: scope.column.clone(),
                tenant_type: scope.tenant_type.clone(),
            })
        })
        .collect::<Vec<_>>();
    let check_query = |query: &str,
                       span: zelyra_ast::Span,
                       allowed_table: Option<&str>,
                       function_params: Option<&HashMap<String, zelyra_ast::Type>>|
     -> bool {
        let (operation, accesses) =
            zelyra_database::sql::analyze_table_access(query, &known_tables);
        if operation.is_none() {
            diagnostic_with_span(
                path,
                if recipient_tables.is_empty() {
                    "E-TENANT-003"
                } else {
                    "E-CRUD-008"
                },
                "projects with tenant- or recipient-scoped data may use only SQL forms whose table access can be checked",
                span,
            );
            return false;
        }
        if let Some(params) = function_params {
            let touches_only_verifiable_scoped = accesses.iter().all(|access| {
                if access.table.is_empty() {
                    return false;
                }
                if recipient_tables
                    .iter()
                    .any(|table| table.eq_ignore_ascii_case(&access.table))
                    || membership_tables
                        .iter()
                        .any(|table| table.eq_ignore_ascii_case(&access.table))
                {
                    return false;
                }
                if protected_tables
                    .iter()
                    .any(|table| table.eq_ignore_ascii_case(&access.table))
                {
                    return tenant_requirements
                        .iter()
                        .any(|req| req.table.eq_ignore_ascii_case(&access.table));
                }
                true
            });
            let touches_any_scoped = accesses.iter().any(|access| {
                tenant_requirements
                    .iter()
                    .any(|req| req.table.eq_ignore_ascii_case(&access.table))
            });
            if touches_any_scoped && touches_only_verifiable_scoped {
                match zelyra_database::sql::verify_tenant_scoped_sql(
                    query,
                    &tenant_requirements,
                    Some(params),
                ) {
                    Ok(_) => return true,
                    Err(reason) => {
                        diagnostic_with_span(path, "E-TENANT-003", &reason, span);
                        return false;
                    }
                }
            }
        }
        let mut query_valid = true;
        for access in accesses {
            if access.table.is_empty() {
                diagnostic_with_span(
                    path,
                    if recipient_tables.is_empty() {
                        "E-TENANT-003"
                    } else {
                        "E-CRUD-008"
                    },
                    "quoted SQL table references cannot be verified in a project with scoped data",
                    span,
                );
                query_valid = false;
                continue;
            }
            if let Some(allowed) = allowed_table {
                if allowed.eq_ignore_ascii_case(&access.table) {
                    continue;
                }
            }
            if recipient_tables
                .iter()
                .any(|table| table.eq_ignore_ascii_case(&access.table))
            {
                diagnostic_with_span(
                    path,
                    "E-CRUD-008",
                    &format!(
                        "unscoped SQL access to `{}` is not allowed; use its recipient-scoped CRUD route",
                        access.table
                    ),
                    span,
                );
                query_valid = false;
            } else if protected_tables
                .iter()
                .any(|table| table.eq_ignore_ascii_case(&access.table))
            {
                diagnostic_with_span(
                    path,
                    "E-TENANT-003",
                    &format!(
                        "unscoped SQL access to `{}` is not allowed in a tenant-enabled project; use its generated tenant CRUD route",
                        access.table
                    ),
                    span,
                );
                query_valid = false;
            }
        }
        query_valid
    };

    for form in &program.forms {
        if let Some(public_token) = &form.public_token {
            if let Err(message) = zelyra_forms::validate_public_token_form(form, program) {
                diagnostic_with_span(path, "E-FORM-010", &message, public_token.span);
                valid = false;
            }
            // A valid public-token form is executed by a dedicated runtime
            // path that resolves one globally unique token and scopes all
            // reads and writes to the resulting row and tenant key. It never
            // executes user-provided SQL.
            continue;
        }
        if form.table.as_ref().is_some_and(|table| {
            protected_tables
                .iter()
                .any(|protected| protected.eq_ignore_ascii_case(table))
        }) {
            let recipient_form = form.table.as_ref().is_some_and(|table| {
                recipient_tables
                    .iter()
                    .any(|protected| protected.eq_ignore_ascii_case(table))
            });
            diagnostic_with_span(
                path,
                if recipient_form {
                    "E-CRUD-008"
                } else {
                    "E-TENANT-003"
                },
                if recipient_form {
                    "forms cannot access recipient-owned tables outside their generated recipient CRUD route"
                } else {
                    "forms cannot access tenant-owned or membership tables outside the generated tenant CRUD route"
                },
                form.span,
            );
            valid = false;
        }
        for action in &form.actions {
            visit_block_sql(
                &zelyra_ast::Block {
                    statements: action.statements.clone(),
                    span: action.span,
                },
                &mut |query, span| valid &= check_query(query, span, None, None),
            );
            if let Some(body) = &action.execute_body {
                visit_block_sql(body, &mut |query, span| {
                    valid &= check_query(query, span, None, None)
                });
            }
            if let Some(guard) = &action.guard {
                visit_expression_sql(&guard.condition, &mut |query, span| {
                    valid &= check_query(query, span, None, None)
                });
            }
        }
    }
    for crud in &program.cruds {
        if let Some(expected_column) = scoped_tenant_tables
            .iter()
            .find(|(table, _)| table.eq_ignore_ascii_case(&crud.table))
            .map(|(_, column)| column.as_str())
        {
            if crud.tenant_column.as_deref() != Some(expected_column) {
                diagnostic_with_span(
                    path,
                    "E-TENANT-001",
                    &format!(
                        "CRUD for scoped table `{}` must use its declared tenant column `{expected_column}`",
                        crud.table
                    ),
                    crud.span,
                );
                valid = false;
            }
        }
        if tenant_tables
            .iter()
            .any(|table| table.eq_ignore_ascii_case(&crud.table))
            && crud.tenant_column.is_none()
        {
            diagnostic_with_span(
                path,
                "E-TENANT-003",
                "every CRUD resource for a tenant-owned table must use its declared tenant column",
                crud.span,
            );
            valid = false;
        }
        if let Some(expected_recipient_column) = program
            .cruds
            .iter()
            .find(|candidate| {
                candidate.recipient_column.is_some()
                    && candidate.table.eq_ignore_ascii_case(&crud.table)
            })
            .and_then(|candidate| candidate.recipient_column.as_deref())
        {
            if crud.recipient_column.as_deref() != Some(expected_recipient_column) {
                diagnostic_with_span(
                    path,
                    "E-CRUD-008",
                    "every CRUD resource for a recipient-owned table must use the same declared recipient column",
                    crud.span,
                );
                valid = false;
            }
            if crud.tenant_column.is_none() && !crud.actions.is_empty() {
                diagnostic_with_span(
                    path,
                    "E-CRUD-008",
                    "recipient-scoped CRUD does not allow custom SQL actions; use generated edit forms so record ownership remains enforced",
                    crud.span,
                );
                valid = false;
            }
        }
        if crud.tenant_column.is_some()
            || !recipient_tables
                .iter()
                .any(|table| table.eq_ignore_ascii_case(&crud.table))
        {
            for action in &crud.actions {
                visit_block_sql(
                    &zelyra_ast::Block {
                        statements: action.statements.clone(),
                        span: action.span,
                    },
                    &mut |query, span| valid &= check_query(query, span, Some(&crud.table), None),
                );
                if let Some(body) = &action.execute_body {
                    visit_block_sql(body, &mut |query, span| {
                        valid &= check_query(query, span, Some(&crud.table), None)
                    });
                }
                if let Some(guard) = &action.guard {
                    visit_expression_sql(&guard.condition, &mut |query, span| {
                        valid &= check_query(query, span, Some(&crud.table), None)
                    });
                }
            }
        }
    }
    for page in &program.pages {
        for data in &page.data {
            valid &= check_query(&data.query, data.span, None, None);
        }
    }
    for tableview in &program.tableviews {
        valid &= check_query(&tableview.source, tableview.span, None, None);
    }
    for function in &program.functions {
        let fn_params = function
            .params
            .iter()
            .map(|param| (param.name.clone(), param.ty.clone()))
            .collect::<HashMap<_, _>>();
        for expression in function.requires.iter().chain(&function.ensures) {
            visit_expression_sql(expression, &mut |query, span| {
                valid &= check_query(query, span, None, None)
            });
        }
        visit_block_sql(&function.body, &mut |query, span| {
            valid &= check_query(query, span, None, Some(&fn_params))
        });
    }
    valid
}

fn visit_expression_sql<'a>(
    expression: &'a zelyra_ast::Expr,
    visitor: &mut impl FnMut(&'a str, zelyra_ast::Span),
) {
    use zelyra_ast::ExprKind;
    match &expression.kind {
        ExprKind::Array(values) => {
            for value in values {
                visit_expression_sql(value, visitor);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                visit_expression_sql(key, visitor);
                visit_expression_sql(value, visitor);
            }
        }
        ExprKind::Record { fields, .. } => {
            for (_, value) in fields {
                visit_expression_sql(value, visitor);
            }
        }
        ExprKind::Index { target, index } => {
            visit_expression_sql(target, visitor);
            visit_expression_sql(index, visitor);
        }
        ExprKind::Field { target, .. }
        | ExprKind::Unary { expr: target, .. }
        | ExprKind::Await(target)
        | ExprKind::Try(target) => visit_expression_sql(target, visitor),
        ExprKind::Call { args, .. } => {
            for argument in args {
                visit_expression_sql(argument, visitor);
            }
        }
        ExprKind::Binary { left, right, .. } => {
            visit_expression_sql(left, visitor);
            visit_expression_sql(right, visitor);
        }
        ExprKind::Sql { query, .. } => visitor(query, expression.span),
        ExprKind::Match { value, arms } => {
            visit_expression_sql(value, visitor);
            for arm in arms {
                visit_expression_sql(&arm.value, visitor);
            }
        }
        ExprKind::Lambda { body, .. } => visit_expression_sql(body, visitor),
        ExprKind::Int(_)
        | ExprKind::UInt(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::String(_)
        | ExprKind::Char(_)
        | ExprKind::Variable(_)
        | ExprKind::FunctionValue(_) => {}
    }
}

fn visit_block_sql<'a>(
    block: &'a zelyra_ast::Block,
    visitor: &mut impl FnMut(&'a str, zelyra_ast::Span),
) {
    use zelyra_ast::Stmt;
    for statement in &block.statements {
        match statement {
            Stmt::Let { value, .. }
            | Stmt::BindOrAssign { value, .. }
            | Stmt::TupleBindOrAssign { value, .. }
            | Stmt::Expr(value)
            | Stmt::Return {
                value: Some(value), ..
            } => visit_expression_sql(value, visitor),
            Stmt::ArrayIndexAssign { index, value, .. } => {
                visit_expression_sql(index, visitor);
                visit_expression_sql(value, visitor);
            }
            Stmt::Return { value: None, .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                visit_expression_sql(condition, visitor);
                visit_block_sql(then_block, visitor);
                if let Some(else_block) = else_block {
                    visit_block_sql(else_block, visitor);
                }
            }
            Stmt::While {
                condition,
                invariants,
                body,
                ..
            } => {
                visit_expression_sql(condition, visitor);
                for invariant in invariants {
                    visit_expression_sql(invariant, visitor);
                }
                visit_block_sql(body, visitor);
            }
            Stmt::For { iterable, body, .. } => {
                visit_expression_sql(iterable, visitor);
                visit_block_sql(body, visitor);
            }
            Stmt::Loop {
                invariants, body, ..
            } => {
                for invariant in invariants {
                    visit_expression_sql(invariant, visitor);
                }
                visit_block_sql(body, visitor);
            }
            Stmt::Match { value, arms, .. } => {
                visit_expression_sql(value, visitor);
                for arm in arms {
                    visit_block_sql(&arm.body, visitor);
                }
            }
            Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => {
                visit_block_sql(body, visitor)
            }
        }
    }
}
