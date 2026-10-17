use crate::packages::PackageGraph;
use crate::project::ProjectSource;
use std::path::Path;
use zelyra_ast::{Block, Expr, ExprKind, Program, Span, Stmt};

pub(crate) struct SettingUseError {
    pub(crate) code: &'static str,
    pub(crate) message: String,
    pub(crate) path: String,
    pub(crate) span: Span,
}

pub(crate) fn resolve_runtime_settings(
    entry: &str,
    sources: &[ProjectSource],
) -> Result<Option<zelyra_runtime::PackageSettings>, String> {
    let root = Path::new(entry)
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let Some(graph) = PackageGraph::load(root).map_err(|error| error.message)? else {
        return Ok(None);
    };
    let source_paths = sources
        .iter()
        .enumerate()
        .map(|(source_id, source)| (source_id as u32, source.path.clone()))
        .collect::<Vec<_>>();
    let settings = graph
        .runtime_settings(&source_paths, |name| {
            std::env::var(name).ok().filter(|value| !value.is_empty())
        })
        .map_err(|error| error.message)?;
    Ok((!settings.is_empty()).then_some(settings))
}

pub(crate) fn validate_usage(
    program: &Program,
    sources: &[ProjectSource],
    packages: Option<&PackageGraph>,
) -> Result<(), SettingUseError> {
    for function in &program.functions {
        visit_block(&function.body, &mut |expression| {
            let ExprKind::Call {
                name,
                type_args,
                args,
            } = &expression.kind
            else {
                return Ok(());
            };
            if name != "module_setting" || type_args.len() != 1 || args.len() != 1 {
                return Ok(());
            }
            let source = sources
                .get(expression.span.source_id as usize)
                .ok_or_else(|| SettingUseError {
                    code: "E-PKG-SETTINGS-003",
                    message: "module_setting call has no owning source file".into(),
                    path: "<project>".into(),
                    span: expression.span,
                })?;
            let ExprKind::String(setting_name) = &args[0].kind else {
                return Err(SettingUseError {
                    code: "E-PKG-SETTINGS-003",
                    message: "module_setting requires a literal setting name".into(),
                    path: source.path.clone(),
                    span: args[0].span,
                });
            };
            let Some((package_path, declared_type)) = packages
                .and_then(|packages| packages.setting_type_for_source(&source.path, setting_name))
            else {
                return Err(SettingUseError {
                    code: "E-PKG-SETTINGS-003",
                    message: format!(
                        "setting `{setting_name}` is not declared by the package owning `{}`",
                        source.path
                    ),
                    path: source.path.clone(),
                    span: expression.span,
                });
            };
            if type_args[0].to_string() != declared_type {
                return Err(SettingUseError {
                    code: "E-PKG-SETTINGS-004",
                    message: format!(
                        "setting `{setting_name}` in package `{package_path}` has type `{declared_type}`, but module_setting requested `{}`",
                        type_args[0]
                    ),
                    path: source.path.clone(),
                    span: expression.span,
                });
            }
            Ok(())
        })?;
    }
    Ok(())
}

fn visit_block(
    block: &Block,
    visitor: &mut impl FnMut(&Expr) -> Result<(), SettingUseError>,
) -> Result<(), SettingUseError> {
    for statement in &block.statements {
        visit_statement(statement, visitor)?;
    }
    Ok(())
}

fn visit_statement(
    statement: &Stmt,
    visitor: &mut impl FnMut(&Expr) -> Result<(), SettingUseError>,
) -> Result<(), SettingUseError> {
    match statement {
        Stmt::Let { value, .. }
        | Stmt::BindOrAssign { value, .. }
        | Stmt::TupleBindOrAssign { value, .. }
        | Stmt::Expr(value) => visit_expression(value, visitor)?,
        Stmt::ArrayIndexAssign { index, value, .. } => {
            visit_expression(index, visitor)?;
            visit_expression(value, visitor)?;
        }
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                visit_expression(value, visitor)?;
            }
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            visit_expression(condition, visitor)?;
            visit_block(then_block, visitor)?;
            if let Some(block) = else_block {
                visit_block(block, visitor)?;
            }
        }
        Stmt::While {
            condition,
            invariants,
            body,
            ..
        } => {
            visit_expression(condition, visitor)?;
            for invariant in invariants {
                visit_expression(invariant, visitor)?;
            }
            visit_block(body, visitor)?;
        }
        Stmt::For { iterable, body, .. } => {
            visit_expression(iterable, visitor)?;
            visit_block(body, visitor)?;
        }
        Stmt::Loop {
            invariants, body, ..
        } => {
            for invariant in invariants {
                visit_expression(invariant, visitor)?;
            }
            visit_block(body, visitor)?;
        }
        Stmt::Match { value, arms, .. } => {
            visit_expression(value, visitor)?;
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    visit_expression(guard, visitor)?;
                }
                visit_block(&arm.body, visitor)?;
            }
        }
        Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => {
            visit_block(body, visitor)?;
        }
        Stmt::Break { .. } | Stmt::Continue { .. } => {}
    }
    Ok(())
}

fn visit_expression(
    expression: &Expr,
    visitor: &mut impl FnMut(&Expr) -> Result<(), SettingUseError>,
) -> Result<(), SettingUseError> {
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        visitor(expression)?;
        match &expression.kind {
            ExprKind::Array(values) => pending.extend(values.iter().rev()),
            ExprKind::Map(entries) => {
                for (key, value) in entries.iter().rev() {
                    pending.push(value);
                    pending.push(key);
                }
            }
            ExprKind::Record { fields, .. } => {
                pending.extend(fields.iter().rev().map(|(_, value)| value));
            }
            ExprKind::Lambda { body, .. } => pending.push(body),
            ExprKind::Index { target, index } => {
                pending.push(index);
                pending.push(target);
            }
            ExprKind::Field { target, .. } | ExprKind::Await(target) => pending.push(target),
            ExprKind::Call { args, .. } => pending.extend(args.iter().rev()),
            ExprKind::Unary { expr, .. } => pending.push(expr),
            ExprKind::Binary { left, right, .. } => {
                pending.push(right);
                pending.push(left);
            }
            ExprKind::Match { value, arms } => {
                for arm in arms.iter().rev() {
                    pending.push(&arm.value);
                    if let Some(guard) = &arm.guard {
                        pending.push(guard);
                    }
                }
                pending.push(value);
            }
            ExprKind::Int(_)
            | ExprKind::UInt(_)
            | ExprKind::Float(_)
            | ExprKind::Bool(_)
            | ExprKind::String(_)
            | ExprKind::Char(_)
            | ExprKind::FunctionValue(_)
            | ExprKind::Variable(_)
            | ExprKind::Sql { .. } => {}
        }
    }
    Ok(())
}
