//! Control-flow outcome and match-exhaustiveness analysis for type checking.
//!
//! This analysis is deliberately separate from expression/type inference. It
//! answers whether a checked statement can fall through, return, break, or
//! continue; it does not execute the program or infer expression types.

use std::collections::HashSet;
use zelyra_ast::{Block, ExprKind, MatchArm, PatternKind, Program, Stmt};

pub(super) const FLOW_FALLTHROUGH: u8 = 1;
const FLOW_RETURN: u8 = 2;
const FLOW_BREAK: u8 = 4;
const FLOW_CONTINUE: u8 = 8;

pub(super) fn flow_outcomes(block: &Block, program: &Program) -> u8 {
    let mut outcomes = FLOW_FALLTHROUGH;
    for statement in &block.statements {
        if outcomes & FLOW_FALLTHROUGH == 0 {
            break;
        }
        outcomes = (outcomes & !FLOW_FALLTHROUGH) | statement_flow(statement, program);
    }
    outcomes
}

fn statement_flow(statement: &Stmt, program: &Program) -> u8 {
    match statement {
        Stmt::Return { .. } => FLOW_RETURN,
        Stmt::Break { .. } => FLOW_BREAK,
        Stmt::Continue { .. } => FLOW_CONTINUE,
        Stmt::If {
            then_block,
            else_block,
            ..
        } => {
            flow_outcomes(then_block, program)
                | else_block
                    .as_ref()
                    .map(|block| flow_outcomes(block, program))
                    .unwrap_or(FLOW_FALLTHROUGH)
        }
        Stmt::Match { arms, .. } => {
            let mut outcomes = arms
                .iter()
                .map(|arm| flow_outcomes(&arm.body, program))
                .fold(0, |all, arm| all | arm);
            if !match_arms_are_exhaustive(arms, program) {
                outcomes |= FLOW_FALLTHROUGH;
            }
            outcomes
        }
        Stmt::Loop { body, .. } => {
            let body_outcomes = flow_outcomes(body, program);
            let mut outcomes = body_outcomes & FLOW_RETURN;
            if body_outcomes & FLOW_BREAK != 0 {
                outcomes |= FLOW_FALLTHROUGH;
            }
            outcomes
        }
        Stmt::While {
            condition, body, ..
        } => {
            let body_outcomes = flow_outcomes(body, program);
            let mut outcomes = body_outcomes & FLOW_RETURN;
            if !matches!(condition.kind, ExprKind::Bool(true)) || body_outcomes & FLOW_BREAK != 0 {
                outcomes |= FLOW_FALLTHROUGH;
            }
            outcomes
        }
        Stmt::For { body, .. } => FLOW_FALLTHROUGH | (flow_outcomes(body, program) & FLOW_RETURN),
        Stmt::Transaction { body, .. } => flow_outcomes(body, program),
        Stmt::Let { .. }
        | Stmt::BindOrAssign { .. }
        | Stmt::ArrayIndexAssign { .. }
        | Stmt::TupleBindOrAssign { .. }
        | Stmt::Expr(_)
        | Stmt::Parallel { .. } => FLOW_FALLTHROUGH,
    }
}

fn match_arms_are_exhaustive(arms: &[MatchArm], program: &Program) -> bool {
    let mut covered = HashSet::new();
    let mut enum_covered = HashSet::new();
    for arm in arms {
        if arm.guard.is_some() {
            continue;
        }
        match &arm.pattern.kind {
            PatternKind::Wildcard | PatternKind::Variable(_) => return true,
            PatternKind::Bool(value) => {
                covered.insert(if *value { "true" } else { "false" });
            }
            PatternKind::Constructor { name, inner } => {
                let irrefutable_inner = inner.as_deref().is_none_or(|inner| {
                    matches!(inner.kind, PatternKind::Wildcard | PatternKind::Variable(_))
                });
                if irrefutable_inner {
                    covered.insert(name.as_str());
                }
            }
            PatternKind::EnumVariant { enum_name, variant } => {
                enum_covered.insert(format!("{enum_name}::{variant}"));
            }
            PatternKind::EnumVariantPayload {
                enum_name,
                variant,
                payloads,
            } => {
                if payloads.iter().all(|payload| {
                    matches!(
                        payload.kind,
                        PatternKind::Wildcard | PatternKind::Variable(_)
                    )
                }) {
                    enum_covered.insert(format!("{enum_name}::{variant}"));
                }
            }
            PatternKind::Int(_)
            | PatternKind::String(_)
            | PatternKind::Char(_)
            | PatternKind::Record { .. } => {}
        }
    }
    (covered.contains("true") && covered.contains("false"))
        || (covered.contains("Some") && covered.contains("None"))
        || (covered.contains("Ok") && covered.contains("Err"))
        || program.enums.iter().any(|definition| {
            definition
                .variants
                .iter()
                .all(|variant| enum_covered.contains(&format!("{}::{variant}", definition.name)))
        })
}
