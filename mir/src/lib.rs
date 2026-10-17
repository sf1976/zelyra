//! First source-mapped MIR slice for Zelyra.
//!
//! This crate lowers successfully checked functions into typed expression
//! records and explicit basic-block control flow. Its experimental
//! `execution` module interprets a bounded subset of that MIR; unsupported
//! operations fail closed. The representation is structural MIR, not bytecode,
//! and the experimental interpreter is not the default runtime. The verifier
//! checks graph integrity and control-flow type invariants before execution.

use std::collections::{HashMap, HashSet};
use std::fmt;
use zelyra_ast::{ArrayIndexAssignOp, BinaryOp, Span, Type, UnaryOp};
use zelyra_hir::{
    self, FunctionId, HirBlock, HirExpr, HirExprKind, HirPattern, HirPatternKind, HirProgram,
    HirStmt, LocalId,
};
use zelyra_runtime::{
    check_capabilities, check_with_inferred_types, evaluate_binary_value, evaluate_unary_value,
    InferredExpressionType, Value,
};

pub mod bytecode;
pub mod execution;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExpressionId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IteratorId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MatchId(pub usize);

#[derive(Clone, Debug, PartialEq)]
pub struct MirError {
    pub code: &'static str,
    pub message: String,
    pub span: Span,
}

impl fmt::Display for MirError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

#[derive(Clone, Debug)]
pub struct MirProgram {
    pub expressions: Vec<MirExpression>,
    pub functions: Vec<MirFunction>,
    /// Type declarations retained for shared runtime codecs and host services.
    pub type_aliases: Vec<zelyra_ast::TypeDef>,
    pub records: Vec<zelyra_ast::RecordDef>,
    /// Synthetic record metadata used only for decoding table-backed SQL rows.
    pub sql_records: Vec<zelyra_ast::RecordDef>,
    /// Tables whose tenant scopes are enforced before SQL host operations.
    pub scoped_tables: Vec<String>,
    pub enums: Vec<zelyra_ast::EnumDef>,
    /// Checked views required by the shared server-side PDF renderer.
    pub views: Vec<zelyra_ast::ViewDef>,
    /// Checked project schema used by the runtime's read-only storage checks.
    pub database_schema: Result<zelyra_database::Schema, Vec<zelyra_database::SchemaError>>,
}

/// Summary of a verified, semantics-preserving MIR optimization pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OptimizationReport {
    pub folded_scalar_expressions: usize,
}

#[derive(Clone, Debug)]
pub struct MirExpression {
    pub id: ExpressionId,
    pub span: Span,
    pub ty: Type,
    pub kind: MirExpressionKind,
}

/// Typed, source-mapped expression operations with children addressed through
/// the program expression table. This representation is structural MIR; it is
/// not yet an executable instruction sequence or bytecode.
#[derive(Clone, Debug)]
pub enum MirExpressionKind {
    /// Internal construction marker; a verified program must never contain it.
    #[doc(hidden)]
    Uninitialized,
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    String(String),
    Char(char),
    Array(Vec<ExpressionId>),
    Map(Vec<(ExpressionId, ExpressionId)>),
    Record {
        type_name: String,
        type_args: Vec<Type>,
        fields: Vec<(String, ExpressionId)>,
    },
    Lambda {
        params: Vec<(LocalId, String, Type)>,
        body: ExpressionId,
    },
    Local(LocalId),
    EnumVariant {
        enum_name: String,
        variant: String,
    },
    FunctionValue {
        name: String,
        function: Option<FunctionId>,
    },
    Index {
        target: ExpressionId,
        index: ExpressionId,
    },
    Field {
        target: ExpressionId,
        field: String,
    },
    Call {
        name: String,
        function: Option<FunctionId>,
        callee_local: Option<LocalId>,
        type_args: Vec<Type>,
        args: Vec<ExpressionId>,
    },
    Unary {
        op: UnaryOp,
        expr: ExpressionId,
    },
    Binary {
        left: ExpressionId,
        op: BinaryOp,
        right: ExpressionId,
    },
    BinaryChain {
        first: ExpressionId,
        operations: Vec<(BinaryOp, ExpressionId, Span)>,
    },
    Sql {
        result_type: Type,
        query: String,
        parameters: Vec<(String, LocalId)>,
    },
    Await(ExpressionId),
    Match {
        value: ExpressionId,
        arms: Vec<MirMatchExpressionArm>,
    },
}

#[derive(Clone, Debug)]
pub enum MirIteratorSource {
    Collection(ExpressionId),
    IntegerRange {
        start: ExpressionId,
        end: ExpressionId,
        step: Option<ExpressionId>,
    },
}

#[derive(Clone, Debug)]
pub struct MirIteratorBinding {
    pub local: LocalId,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct MirTupleBinding {
    pub local: LocalId,
    pub name: String,
    pub is_new: bool,
}

#[derive(Clone, Debug)]
pub struct MirParallelBinding {
    pub local: LocalId,
    pub name: String,
    pub is_new: bool,
    pub value: ExpressionId,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct MirMatchExpressionArm {
    pub pattern: MirPattern,
    pub guard: Option<ExpressionId>,
    pub value: ExpressionId,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct MirPattern {
    pub kind: MirPatternKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum MirPatternKind {
    Wildcard,
    Binding {
        name: String,
        local: LocalId,
    },
    Constructor {
        name: String,
        inner: Option<Box<MirPattern>>,
    },
    EnumVariant {
        enum_name: String,
        variant: String,
    },
    EnumVariantPayload {
        enum_name: String,
        variant: String,
        payloads: Vec<MirPattern>,
    },
    Record {
        type_name: String,
        fields: Vec<(String, MirPattern)>,
    },
    Int(i64),
    Bool(bool),
    String(String),
    Char(char),
}

#[derive(Clone, Debug)]
pub struct MirFunction {
    pub id: FunctionId,
    pub name: String,
    pub parameters: Vec<(LocalId, String, Type, Span)>,
    pub declared_return_type: Option<Type>,
    pub declared_capabilities: Vec<String>,
    pub requires: Vec<ExpressionId>,
    pub ensures: Vec<ExpressionId>,
    /// Local used by `result` references in postconditions.
    pub contract_result_local: LocalId,
    pub entry: BlockId,
    pub blocks: Vec<MirBasicBlock>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct MirBasicBlock {
    pub id: BlockId,
    pub instructions: Vec<MirInstruction>,
    /// Loop invariants checked before evaluating a loop condition or body.
    pub loop_invariants: Vec<ExpressionId>,
    /// Source span of the loop statement for invariant-failure diagnostics.
    pub loop_span: Option<Span>,
    pub terminator: Option<MirTerminator>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopInvariantPhase {
    Before,
    After,
}

#[derive(Clone, Debug)]
pub struct MirLoopInvariant {
    pub expression: ExpressionId,
    pub loop_span: Span,
}

#[derive(Clone, Debug)]
pub enum MirInstruction {
    IteratorInit {
        iterator: IteratorId,
        source: MirIteratorSource,
        span: Span,
    },
    MatchInit {
        match_id: MatchId,
        value: ExpressionId,
        span: Span,
    },
    Let {
        local: LocalId,
        name: String,
        mutable: bool,
        declared_type: Option<Type>,
        value: ExpressionId,
        span: Span,
    },
    Assign {
        local: LocalId,
        name: String,
        value: ExpressionId,
        span: Span,
    },
    ArrayIndexAssign {
        local: LocalId,
        name: String,
        index: ExpressionId,
        value: ExpressionId,
        operation: ArrayIndexAssignOp,
        span: Span,
    },
    /// Evaluates `value` once, then assigns its tuple elements left-to-right.
    TupleBind {
        bindings: Vec<MirTupleBinding>,
        value: ExpressionId,
        span: Span,
    },
    /// Executes the checked SQL statements in one database transaction.
    /// Expressions are direct SQL statements or standalone `mail_queue` calls.
    Transaction {
        operations: Vec<ExpressionId>,
        span: Span,
    },
    /// Evaluate each checked `await` expression against the same lexical
    /// snapshot, then bind all results after every branch has completed.
    Parallel {
        bindings: Vec<MirParallelBinding>,
        span: Span,
    },
    LoopInvariantCheck {
        expressions: Vec<ExpressionId>,
        phase: LoopInvariantPhase,
        span: Span,
    },
    Evaluate(ExpressionId),
}

#[derive(Clone, Debug)]
pub enum MirTerminator {
    Goto(BlockId),
    Branch {
        condition: ExpressionId,
        then_block: BlockId,
        else_block: BlockId,
    },
    IteratorNext {
        iterator: IteratorId,
        bindings: Vec<MirIteratorBinding>,
        body_block: BlockId,
        exit_block: BlockId,
    },
    /// A successful test binds its pattern locals before entering the matched
    /// block; a failed test leaves those locals unbound.
    MatchTest {
        match_id: MatchId,
        pattern: MirPattern,
        matched_block: BlockId,
        unmatched_block: BlockId,
    },
    MatchFailure {
        span: Span,
    },
    Return(Option<ExpressionId>),
    ReturnAfterLoopInvariants {
        value: Option<ExpressionId>,
        invariants: Vec<MirLoopInvariant>,
    },
    Unreachable,
}

/// Build a typed control-flow MIR from a fully type-checked program.
///
/// The API intentionally returns errors instead of emitting an incomplete
/// graph for statement kinds that are not represented yet.
pub fn lower(program: &zelyra_ast::Program) -> Result<MirProgram, Vec<MirError>> {
    let inferred = check_with_inferred_types(program).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| MirError {
                code: error.code,
                message: error.message,
                span: error.span,
            })
            .collect::<Vec<_>>()
    })?;
    check_capabilities(program).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| MirError {
                code: "E-MIR-EFFECT-001",
                message: error.message,
                span: error.span,
            })
            .collect::<Vec<_>>()
    })?;
    let hir = zelyra_hir::lower(program).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| MirError {
                code: "E-MIR-RESOLVE-001",
                message: error.message,
                span: error.span,
            })
            .collect::<Vec<_>>()
    })?;
    lower_checked(
        &hir,
        &inferred,
        zelyra_database::build_schema(program),
        zelyra_runtime::sql_projection_records(program),
        program
            .tables
            .iter()
            .filter(|table| table.tenant_scope.is_some())
            .map(|table| table.name.clone())
            .collect(),
        program.views.clone(),
    )
}

fn lower_checked(
    hir: &HirProgram,
    inferred: &[InferredExpressionType],
    database_schema: Result<zelyra_database::Schema, Vec<zelyra_database::SchemaError>>,
    sql_records: Vec<zelyra_ast::RecordDef>,
    scoped_tables: Vec<String>,
    views: Vec<zelyra_ast::ViewDef>,
) -> Result<MirProgram, Vec<MirError>> {
    let mut builder = ProgramBuilder {
        expressions: Vec::new(),
        expression_ids: HashMap::new(),
        inferred: inferred
            .iter()
            .map(|item| (span_key(item.span), item.ty.clone()))
            .collect(),
        errors: Vec::new(),
    };

    let mut functions = Vec::with_capacity(hir.functions.len());
    for function in &hir.functions {
        functions.push(builder.function(function));
    }
    if !builder.errors.is_empty() {
        return Err(builder.errors);
    }

    let mir = MirProgram {
        expressions: builder.expressions,
        functions,
        type_aliases: hir.types.clone(),
        records: hir.records.clone(),
        sql_records,
        scoped_tables,
        enums: hir.enums.clone(),
        views,
        database_schema,
    };
    verify(&mir)?;
    Ok(mir)
}

pub fn verify(program: &MirProgram) -> Result<(), Vec<MirError>> {
    let mut errors = Vec::new();
    if let Ok(schema) = &program.database_schema {
        let mut schema_scopes = schema
            .tables
            .iter()
            .filter(|table| table.tenant_scope_column.is_some())
            .map(|table| table.name.to_ascii_lowercase())
            .collect::<Vec<_>>();
        let mut program_scopes = program
            .scoped_tables
            .iter()
            .map(|table| table.to_ascii_lowercase())
            .collect::<Vec<_>>();
        schema_scopes.sort();
        program_scopes.sort();
        if schema_scopes != program_scopes {
            errors.push(MirError {
                code: "E-MIR-VERIFY-003",
                message: "MIR tenant-scope metadata does not match the checked project schema"
                    .into(),
                span: Span::default(),
            });
        }
    }
    for (index, expression) in program.expressions.iter().enumerate() {
        if expression.id.0 != index {
            errors.push(MirError {
                code: "E-MIR-VERIFY-001",
                message: "expression table contains a non-canonical id".into(),
                span: expression.span,
            });
        }
        verify_expression_kind(program, expression, &mut errors);
    }
    verify_capability_call_edges(program, &mut errors);
    for function in &program.functions {
        let iterator_ids = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter_map(|instruction| match instruction {
                MirInstruction::IteratorInit { iterator, .. } => Some(*iterator),
                _ => None,
            })
            .collect::<std::collections::HashSet<_>>();
        let iterator_init_count = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter(|instruction| matches!(instruction, MirInstruction::IteratorInit { .. }))
            .count();
        if iterator_ids.len() != iterator_init_count {
            errors.push(MirError {
                code: "E-MIR-VERIFY-001",
                message: format!(
                    "function `{}` initializes an iterator more than once",
                    function.name
                ),
                span: function.span,
            });
        }
        let match_ids = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter_map(|instruction| match instruction {
                MirInstruction::MatchInit { match_id, .. } => Some(*match_id),
                _ => None,
            })
            .collect::<std::collections::HashSet<_>>();
        let match_init_count = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter(|instruction| matches!(instruction, MirInstruction::MatchInit { .. }))
            .count();
        if match_ids.len() != match_init_count {
            errors.push(MirError {
                code: "E-MIR-VERIFY-001",
                message: format!(
                    "function `{}` initializes a match more than once",
                    function.name
                ),
                span: function.span,
            });
        }
        if function.entry.0 >= function.blocks.len() {
            errors.push(MirError {
                code: "E-MIR-VERIFY-001",
                message: format!("function `{}` has an invalid entry block", function.name),
                span: function.span,
            });
            continue;
        }
        let dominators = compute_dominators(function);
        let iterator_init_blocks = function
            .blocks
            .iter()
            .enumerate()
            .flat_map(|(index, block)| {
                block
                    .instructions
                    .iter()
                    .filter_map(move |instruction| match instruction {
                        MirInstruction::IteratorInit { iterator, .. } => {
                            Some((*iterator, BlockId(index)))
                        }
                        _ => None,
                    })
            })
            .collect::<HashMap<_, _>>();
        let match_init_blocks = function
            .blocks
            .iter()
            .enumerate()
            .flat_map(|(index, block)| {
                block
                    .instructions
                    .iter()
                    .filter_map(move |instruction| match instruction {
                        MirInstruction::MatchInit { match_id, .. } => {
                            Some((*match_id, BlockId(index)))
                        }
                        _ => None,
                    })
            })
            .collect::<HashMap<_, _>>();
        let mutable_locals = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter_map(|instruction| match instruction {
                MirInstruction::Let {
                    local,
                    mutable: true,
                    ..
                } => Some(*local),
                _ => None,
            })
            .collect::<HashSet<_>>();
        let mut local_types = collect_local_types(program, function, &mut errors);
        let result_type = function.declared_return_type.clone().unwrap_or(Type::Unit);
        if local_types
            .insert(function.contract_result_local, result_type)
            .is_some()
        {
            errors.push(MirError {
                code: "E-MIR-VERIFY-001",
                message: format!(
                    "function `{}` contract result local conflicts with another local",
                    function.name
                ),
                span: function.span,
            });
        }
        for (index, block) in function.blocks.iter().enumerate() {
            if block.id.0 != index {
                errors.push(MirError {
                    code: "E-MIR-VERIFY-001",
                    message: format!("function `{}` has a non-canonical block id", function.name),
                    span: block.span,
                });
            }
            for expression in &block.loop_invariants {
                verify_expression(program, *expression, block, &mut errors);
                verify_expression_type(
                    program,
                    *expression,
                    &Type::Bool,
                    block.span,
                    "loop invariant",
                    &mut errors,
                );
            }
            match &block.terminator {
                Some(MirTerminator::Goto(target)) => {
                    verify_target(function, block, *target, &mut errors)
                }
                Some(MirTerminator::Branch {
                    condition,
                    then_block,
                    else_block,
                }) => {
                    verify_expression(program, *condition, block, &mut errors);
                    verify_expression_type(
                        program,
                        *condition,
                        &Type::Bool,
                        block.span,
                        "branch condition",
                        &mut errors,
                    );
                    verify_target(function, block, *then_block, &mut errors);
                    verify_target(function, block, *else_block, &mut errors);
                }
                Some(MirTerminator::IteratorNext {
                    iterator,
                    body_block,
                    exit_block,
                    ..
                }) => {
                    if !iterator_ids.contains(iterator) {
                        errors.push(MirError {
                            code: "E-MIR-VERIFY-001",
                            message: format!(
                                "block {} advances an iterator that was not initialized",
                                block.id.0
                            ),
                            span: block.span,
                        });
                    } else if let Some(initializer) = iterator_init_blocks.get(iterator) {
                        let use_block = BlockId(index);
                        if dominators.is_reachable(use_block)
                            && !dominators.dominates(*initializer, use_block)
                        {
                            errors.push(MirError {
                                code: "E-MIR-VERIFY-001",
                                message: format!(
                                    "iterator {} initialization does not dominate its use",
                                    iterator.0
                                ),
                                span: block.span,
                            });
                        }
                    }
                    verify_target(function, block, *body_block, &mut errors);
                    verify_target(function, block, *exit_block, &mut errors);
                }
                Some(MirTerminator::MatchTest {
                    match_id,
                    matched_block,
                    unmatched_block,
                    ..
                }) => {
                    if !match_ids.contains(match_id) {
                        errors.push(MirError {
                            code: "E-MIR-VERIFY-001",
                            message: format!(
                                "block {} tests a match value that was not initialized",
                                block.id.0
                            ),
                            span: block.span,
                        });
                    } else if let Some(initializer) = match_init_blocks.get(match_id) {
                        let use_block = BlockId(index);
                        if dominators.is_reachable(use_block)
                            && !dominators.dominates(*initializer, use_block)
                        {
                            errors.push(MirError {
                                code: "E-MIR-VERIFY-001",
                                message: format!(
                                    "match {} initialization does not dominate its test",
                                    match_id.0
                                ),
                                span: block.span,
                            });
                        }
                    }
                    verify_target(function, block, *matched_block, &mut errors);
                    verify_target(function, block, *unmatched_block, &mut errors);
                }
                Some(MirTerminator::MatchFailure { .. }) => {}
                Some(MirTerminator::Return(value)) => {
                    if let Some(value) = value {
                        verify_expression(program, *value, block, &mut errors);
                    }
                    if let Some(expected) = &function.declared_return_type {
                        let actual = value
                            .and_then(|value| program.expressions.get(value.0))
                            .map_or(&Type::Unit, |expression| &expression.ty);
                        if actual != expected {
                            errors.push(MirError {
                                code: "E-MIR-VERIFY-002",
                                message: format!(
                                    "function `{}` returns `{actual}` but declares `{expected}`",
                                    function.name
                                ),
                                span: block.span,
                            });
                        }
                    }
                }
                Some(MirTerminator::ReturnAfterLoopInvariants { value, invariants }) => {
                    if let Some(value) = value {
                        verify_expression(program, *value, block, &mut errors);
                    }
                    for invariant in invariants {
                        verify_expression(program, invariant.expression, block, &mut errors);
                        verify_expression_type(
                            program,
                            invariant.expression,
                            &Type::Bool,
                            invariant.loop_span,
                            "loop invariant",
                            &mut errors,
                        );
                    }
                    if let Some(expected) = &function.declared_return_type {
                        let actual = value
                            .and_then(|value| program.expressions.get(value.0))
                            .map_or(&Type::Unit, |expression| &expression.ty);
                        if actual != expected {
                            errors.push(MirError {
                                code: "E-MIR-VERIFY-002",
                                message: format!(
                                    "function `{}` returns `{actual}` but declares `{expected}`",
                                    function.name
                                ),
                                span: block.span,
                            });
                        }
                    }
                }
                Some(MirTerminator::Unreachable) => {}
                None => errors.push(MirError {
                    code: "E-MIR-VERIFY-001",
                    message: format!(
                        "function `{}` has unterminated block {}",
                        function.name, block.id.0
                    ),
                    span: block.span,
                }),
            }
            for instruction in &block.instructions {
                let expression = match instruction {
                    MirInstruction::IteratorInit { source, .. } => {
                        match source {
                            MirIteratorSource::Collection(expression) => {
                                verify_expression(program, *expression, block, &mut errors)
                            }
                            MirIteratorSource::IntegerRange { start, end, step } => {
                                verify_expression(program, *start, block, &mut errors);
                                verify_expression(program, *end, block, &mut errors);
                                if let Some(step) = step {
                                    verify_expression(program, *step, block, &mut errors);
                                }
                            }
                        }
                        None
                    }
                    MirInstruction::MatchInit { value, .. } => {
                        verify_expression(program, *value, block, &mut errors);
                        None
                    }
                    MirInstruction::ArrayIndexAssign { index, value, .. } => {
                        verify_expression(program, *index, block, &mut errors);
                        Some(*value)
                    }
                    MirInstruction::Transaction { operations, span } => {
                        for operation in operations {
                            verify_expression(program, *operation, block, &mut errors);
                            let supported =
                                match program.expressions.get(operation.0).map(|item| &item.kind) {
                                    Some(MirExpressionKind::Sql { .. }) => true,
                                    Some(MirExpressionKind::Call { name, .. }) => {
                                        name == "mail_queue"
                                    }
                                    _ => false,
                                };
                            if !supported {
                                errors.push(MirError {
                                    code: "E-MIR-VERIFY-001",
                                    message: "transaction entries must reference SQL expressions or standalone mail_queue calls".into(),
                                    span: *span,
                                });
                            }
                        }
                        None
                    }
                    MirInstruction::Parallel { bindings, .. } => {
                        for binding in bindings {
                            verify_expression(program, binding.value, block, &mut errors);
                        }
                        None
                    }
                    MirInstruction::LoopInvariantCheck { expressions, .. } => {
                        for expression in expressions {
                            verify_expression(program, *expression, block, &mut errors);
                        }
                        None
                    }
                    MirInstruction::Let { value, .. }
                    | MirInstruction::Assign { value, .. }
                    | MirInstruction::TupleBind { value, .. }
                    | MirInstruction::Evaluate(value) => Some(*value),
                };
                if let Some(expression) = expression {
                    verify_expression(program, expression, block, &mut errors);
                }
                verify_instruction_mutability(instruction, &mutable_locals, &mut errors);
                verify_instruction_types(program, instruction, &local_types, &mut errors);
            }
        }
        for expression in function.requires.iter().chain(&function.ensures) {
            if expression.0 >= program.expressions.len() {
                errors.push(MirError {
                    code: "E-MIR-VERIFY-001",
                    message: format!(
                        "function `{}` references a missing contract expression",
                        function.name
                    ),
                    span: function.span,
                });
            } else {
                verify_expression_type(
                    program,
                    *expression,
                    &Type::Bool,
                    program.expressions[expression.0].span,
                    "function contract",
                    &mut errors,
                );
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn verify_capability_call_edges(program: &MirProgram, errors: &mut Vec<MirError>) {
    for caller in &program.functions {
        let local_types = capability_local_types(program, caller);
        let mut declared = HashSet::new();
        for capability in &caller.declared_capabilities {
            if !zelyra_runtime::KNOWN_CAPABILITIES.contains(&capability.as_str()) {
                errors.push(MirError {
                    code: "E-MIR-EFFECT-001",
                    message: format!(
                        "function `{}` declares unknown capability `{capability}`",
                        caller.name
                    ),
                    span: caller.span,
                });
            } else if !declared.insert(capability.as_str()) {
                errors.push(MirError {
                    code: "E-MIR-EFFECT-001",
                    message: format!(
                        "function `{}` declares capability `{capability}` more than once",
                        caller.name
                    ),
                    span: caller.span,
                });
            }
        }

        for expression in program.expressions.iter().filter(|expression| {
            expression.span.source_id == caller.span.source_id
                && expression.span.start >= caller.span.start
                && expression.span.end <= caller.span.end
        }) {
            match &expression.kind {
                MirExpressionKind::Call {
                    name,
                    function: None,
                    callee_local: Some(local),
                    type_args,
                    args,
                } => {
                    if !type_args.is_empty() {
                        errors.push(MirError {
                            code: "E-MIR-VERIFY-002",
                            message: "indirect function calls cannot have explicit type arguments"
                                .into(),
                            span: expression.span,
                        });
                    }
                    match local_types.get(local) {
                        Some(Type::Function {
                            params,
                            return_type,
                            capabilities,
                        }) => {
                            if params.len() != args.len() {
                                errors.push(MirError {
                                    code: "E-MIR-VERIFY-002",
                                    message: format!(
                                        "indirect call `{name}` expects {} arguments, found {}",
                                        params.len(),
                                        args.len()
                                    ),
                                    span: expression.span,
                                });
                            }
                            for (argument, expected) in args.iter().zip(params) {
                                if let Some(actual) = program.expressions.get(argument.0) {
                                    if &actual.ty != expected {
                                        errors.push(MirError {
                                            code: "E-MIR-VERIFY-002",
                                            message: format!(
                                                "indirect call argument has type `{}`, expected `{expected}`",
                                                actual.ty
                                            ),
                                            span: actual.span,
                                        });
                                    }
                                }
                            }
                            if return_type.as_ref() != &expression.ty {
                                errors.push(MirError {
                                    code: "E-MIR-VERIFY-002",
                                    message: format!(
                                        "indirect call returns `{return_type}`, expression declares `{}`",
                                        expression.ty
                                    ),
                                    span: expression.span,
                                });
                            }
                            for capability in capabilities {
                                verify_required_capability(
                                    &declared,
                                    capability,
                                    format!("function `{}` invokes `{name}`", caller.name),
                                    expression.span,
                                    errors,
                                );
                            }
                        }
                        Some(other) => errors.push(MirError {
                            code: "E-MIR-VERIFY-002",
                            message: format!(
                                "indirect call target `{name}` has non-function type `{other}`"
                            ),
                            span: expression.span,
                        }),
                        None => errors.push(MirError {
                            code: "E-MIR-VERIFY-001",
                            message: format!("indirect call references unknown local {}", local.0),
                            span: expression.span,
                        }),
                    }
                }
                MirExpressionKind::Call {
                    name,
                    function: Some(callee_id),
                    ..
                } => {
                    let Some(callee) = program
                        .functions
                        .iter()
                        .find(|function| function.id == *callee_id)
                    else {
                        errors.push(MirError {
                            code: "E-MIR-VERIFY-001",
                            message: format!("reference to `{name}` uses an unknown MIR function"),
                            span: expression.span,
                        });
                        continue;
                    };
                    for capability in &callee.declared_capabilities {
                        verify_required_capability(
                            &declared,
                            capability,
                            format!("function `{}` calls `{name}`", caller.name),
                            expression.span,
                            errors,
                        );
                    }
                }
                MirExpressionKind::Call {
                    name,
                    function: None,
                    args,
                    ..
                } => {
                    for (capability, operation) in
                        zelyra_runtime::builtin_capability_requirements(name)
                    {
                        verify_required_capability(
                            &declared,
                            capability,
                            format!("function `{}` uses {operation}", caller.name),
                            expression.span,
                            errors,
                        );
                    }
                    if matches!(name.as_str(), "map" | "filter" | "find" | "reduce") {
                        let callback_index = if name == "reduce" { 2 } else { 1 };
                        if let Some(callback_id) = args.get(callback_index) {
                            if let Some(callback) = program.expressions.get(callback_id.0) {
                                if let Type::Function { capabilities, .. } = &callback.ty {
                                    for capability in capabilities {
                                        verify_required_capability(
                                            &declared,
                                            capability,
                                            format!(
                                                "function `{}` passes a callback to `{name}`",
                                                caller.name
                                            ),
                                            callback.span,
                                            errors,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                MirExpressionKind::Sql { query, .. } => {
                    let capability = zelyra_runtime::sql_required_capability(query);
                    verify_required_capability(
                        &declared,
                        capability,
                        format!("function `{}` executes SQL", caller.name),
                        expression.span,
                        errors,
                    );
                }
                MirExpressionKind::Lambda { .. } => {
                    if let Type::Function { capabilities, .. } = &expression.ty {
                        for capability in capabilities {
                            verify_required_capability(
                                &declared,
                                capability,
                                format!("function `{}` creates a lambda", caller.name),
                                expression.span,
                                errors,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn capability_local_types(program: &MirProgram, function: &MirFunction) -> HashMap<LocalId, Type> {
    let mut local_types = function
        .parameters
        .iter()
        .map(|(local, _, ty, _)| (*local, ty.clone()))
        .collect::<HashMap<_, _>>();
    for block in &function.blocks {
        for instruction in &block.instructions {
            match instruction {
                MirInstruction::Let {
                    local,
                    declared_type,
                    value,
                    ..
                } => {
                    if let Some(ty) = declared_type.clone().or_else(|| {
                        program
                            .expressions
                            .get(value.0)
                            .map(|expression| expression.ty.clone())
                    }) {
                        local_types.insert(*local, ty);
                    }
                }
                MirInstruction::TupleBind {
                    bindings, value, ..
                } => {
                    if let Some(Type::Tuple(types)) = program
                        .expressions
                        .get(value.0)
                        .map(|expression| &expression.ty)
                    {
                        for (position, binding) in bindings.iter().enumerate() {
                            if let Some(ty) = types.get(position) {
                                local_types.insert(binding.local, ty.clone());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    local_types
}

fn verify_required_capability(
    declared: &HashSet<&str>,
    required: &str,
    operation: String,
    span: Span,
    errors: &mut Vec<MirError>,
) {
    if !zelyra_runtime::KNOWN_CAPABILITIES.contains(&required)
        || !mir_has_capability(declared, required)
    {
        errors.push(MirError {
            code: "E-MIR-EFFECT-001",
            message: format!("{operation} requires capability `{required}`"),
            span,
        });
    }
}

fn mir_has_capability(declared: &HashSet<&str>, required: &str) -> bool {
    declared.contains(required)
        || (required.starts_with("Database(") && declared.contains("Database"))
        || (required == "Database"
            && declared.contains("Database(read)")
            && declared.contains("Database(write)"))
}

/// Fold constant Int, UInt, Float, Bool, and Char unary/binary expressions in
/// an already lowered program. The pass uses the same value operators as the
/// AST and bytecode runtimes. It only evaluates supported literal operands;
/// calls, awaits, reads, string operations, and other nonliteral expressions
/// are never evaluated or removed.
/// Expressions whose operator would fail at runtime are left unchanged so the
/// runtime error and source span remain observable.
pub fn optimize_scalar_constants(
    program: &mut MirProgram,
) -> Result<OptimizationReport, Vec<MirError>> {
    verify(program)?;

    let mut report = OptimizationReport::default();
    // Compiler-produced expression references form a DAG. Repeated passes
    // allow parents created before their children to fold after those children.
    // The bounded iteration count keeps this API safe for manually-built MIR
    // values even if they contain cycles.
    for _ in 0..=program.expressions.len() {
        let mut folded_this_pass = 0;
        for index in 0..program.expressions.len() {
            let expression = &program.expressions[index];
            let span = expression.span;
            let ty = expression.ty.clone();
            let replacement = match &expression.kind {
                MirExpressionKind::Unary { op, expr } => program
                    .expressions
                    .get(expr.0)
                    .and_then(scalar_constant)
                    .and_then(|value| evaluate_unary_value(*op, value, span).ok())
                    .and_then(|value| scalar_expression(value, &ty)),
                MirExpressionKind::Binary { left, op, right } => {
                    let left = program.expressions.get(left.0).and_then(scalar_constant);
                    let right = program.expressions.get(right.0).and_then(scalar_constant);
                    left.zip(right)
                        .and_then(|(left, right)| {
                            evaluate_binary_value(left, *op, right, span).ok()
                        })
                        .and_then(|value| scalar_expression(value, &ty))
                }
                MirExpressionKind::BinaryChain { first, operations } => {
                    let mut value = program.expressions.get(first.0).and_then(scalar_constant);
                    for (op, operand, operation_span) in operations {
                        value = value
                            .zip(program.expressions.get(operand.0).and_then(scalar_constant))
                            .and_then(|(left, right)| {
                                evaluate_binary_value(left, *op, right, *operation_span).ok()
                            });
                    }
                    value.and_then(|value| scalar_expression(value, &ty))
                }
                _ => None,
            };

            if let Some(replacement) = replacement {
                program.expressions[index].kind = replacement;
                folded_this_pass += 1;
            }
        }
        report.folded_scalar_expressions += folded_this_pass;
        if folded_this_pass == 0 {
            break;
        }
    }

    verify(program)?;
    Ok(report)
}

fn scalar_constant(expression: &MirExpression) -> Option<Value> {
    match &expression.kind {
        MirExpressionKind::Int(value) => Some(Value::Int(*value)),
        MirExpressionKind::UInt(value) => Some(Value::UInt(*value)),
        MirExpressionKind::Float(value) => Some(Value::Float(*value)),
        MirExpressionKind::Bool(value) => Some(Value::Bool(*value)),
        MirExpressionKind::Char(value) => Some(Value::Char(*value)),
        _ => None,
    }
}

fn scalar_expression(value: Value, ty: &Type) -> Option<MirExpressionKind> {
    match (value, ty) {
        (Value::Int(value), Type::Int) => Some(MirExpressionKind::Int(value)),
        (Value::UInt(value), Type::UInt) => Some(MirExpressionKind::UInt(value)),
        (Value::Float(value), Type::Float) => Some(MirExpressionKind::Float(value)),
        (Value::Bool(value), Type::Bool) => Some(MirExpressionKind::Bool(value)),
        (Value::Char(value), Type::Char) => Some(MirExpressionKind::Char(value)),
        _ => None,
    }
}

fn collect_local_types(
    program: &MirProgram,
    function: &MirFunction,
    errors: &mut Vec<MirError>,
) -> HashMap<LocalId, Type> {
    let mut local_types = HashMap::new();
    for (local, name, ty, span) in &function.parameters {
        register_local_type(&mut local_types, *local, ty.clone(), name, *span, errors);
    }
    for block in &function.blocks {
        for instruction in &block.instructions {
            let (local, name, ty, span) = match instruction {
                MirInstruction::Let {
                    local,
                    name,
                    declared_type,
                    value,
                    span,
                    ..
                } => {
                    let value_type = program.expressions.get(value.0).map(|item| item.ty.clone());
                    let ty = declared_type.clone().or(value_type);
                    (Some(*local), name.as_str(), ty, *span)
                }
                MirInstruction::TupleBind {
                    bindings,
                    value,
                    span,
                } => {
                    let tuple_types = program.expressions.get(value.0).and_then(|item| match &item
                        .ty
                    {
                        Type::Tuple(types) => Some(types.clone()),
                        _ => None,
                    });
                    for (position, binding) in bindings.iter().enumerate() {
                        if let Some(ty) = tuple_types.as_ref().and_then(|types| types.get(position))
                        {
                            if binding.is_new {
                                register_local_type(
                                    &mut local_types,
                                    binding.local,
                                    ty.clone(),
                                    &binding.name,
                                    *span,
                                    errors,
                                );
                            }
                        }
                    }
                    (None, "", None, *span)
                }
                MirInstruction::Parallel { bindings, span } => {
                    for binding in bindings {
                        if binding.is_new {
                            if let Some(ty) = program
                                .expressions
                                .get(binding.value.0)
                                .map(|item| item.ty.clone())
                            {
                                register_local_type(
                                    &mut local_types,
                                    binding.local,
                                    ty,
                                    &binding.name,
                                    binding.span,
                                    errors,
                                );
                            }
                        }
                    }
                    (None, "", None, *span)
                }
                _ => (None, "", None, block.span),
            };
            if let (Some(local), Some(ty)) = (local, ty) {
                register_local_type(&mut local_types, local, ty, name, span, errors);
            }
        }
    }
    local_types
}

fn register_local_type(
    local_types: &mut HashMap<LocalId, Type>,
    local: LocalId,
    ty: Type,
    name: &str,
    span: Span,
    errors: &mut Vec<MirError>,
) {
    if let Some(previous) = local_types.get(&local) {
        let detail = if previous == &ty {
            format!("local `{name}` is declared more than once")
        } else {
            format!("local `{name}` has conflicting MIR types `{previous}` and `{ty}`")
        };
        errors.push(MirError {
            code: "E-MIR-VERIFY-002",
            message: detail,
            span,
        });
    } else {
        local_types.insert(local, ty);
    }
}

fn verify_instruction_mutability(
    instruction: &MirInstruction,
    mutable_locals: &HashSet<LocalId>,
    errors: &mut Vec<MirError>,
) {
    let mut reject = |local: LocalId, name: &str, span: Span| {
        if !mutable_locals.contains(&local) {
            errors.push(MirError {
                code: "E-MIR-VERIFY-002",
                message: format!("cannot mutate immutable local `{name}`"),
                span,
            });
        }
    };
    match instruction {
        MirInstruction::Assign {
            local, name, span, ..
        }
        | MirInstruction::ArrayIndexAssign {
            local, name, span, ..
        } => reject(*local, name, *span),
        MirInstruction::TupleBind { bindings, span, .. } => {
            for binding in bindings.iter().filter(|binding| !binding.is_new) {
                reject(binding.local, &binding.name, *span);
            }
        }
        MirInstruction::Parallel { bindings, span } => {
            for binding in bindings.iter().filter(|binding| !binding.is_new) {
                reject(binding.local, &binding.name, *span);
            }
        }
        _ => {}
    }
}

fn verify_instruction_types(
    program: &MirProgram,
    instruction: &MirInstruction,
    local_types: &HashMap<LocalId, Type>,
    errors: &mut Vec<MirError>,
) {
    let expression_type = |id: ExpressionId| program.expressions.get(id.0).map(|item| &item.ty);
    match instruction {
        MirInstruction::Let {
            name,
            declared_type: Some(expected),
            value,
            span,
            ..
        } => {
            if let Some(actual) = expression_type(*value) {
                if actual != expected {
                    errors.push(MirError {
                        code: "E-MIR-VERIFY-002",
                        message: format!(
                            "initializer for local `{name}` has type `{actual}`, expected `{expected}`"
                        ),
                        span: *span,
                    });
                }
            }
        }
        MirInstruction::Let {
            declared_type: None,
            ..
        } => {}
        MirInstruction::Assign {
            local,
            name,
            value,
            span,
        } => {
            if let (Some(expected), Some(actual)) =
                (local_types.get(local), expression_type(*value))
            {
                if actual != expected {
                    errors.push(MirError {
                        code: "E-MIR-VERIFY-002",
                        message: format!(
                            "assignment to local `{name}` has type `{actual}`, expected `{expected}`"
                        ),
                        span: *span,
                    });
                }
            }
        }
        MirInstruction::ArrayIndexAssign {
            local,
            name,
            index,
            value,
            span,
            ..
        } => {
            if let Some(target_type) = local_types.get(local) {
                if let Type::Array(element_type) = target_type {
                    if let Some(actual) = expression_type(*index) {
                        if actual != &Type::Int {
                            errors.push(MirError {
                                code: "E-MIR-VERIFY-002",
                                message: format!(
                                    "array index for local `{name}` must have type `Int`, found `{actual}`"
                                ),
                                span: *span,
                            });
                        }
                    }
                    if let Some(actual) = expression_type(*value) {
                        if actual != element_type.as_ref() {
                            errors.push(MirError {
                                code: "E-MIR-VERIFY-002",
                                message: format!(
                                    "array update for local `{name}` has type `{actual}`, expected `{element_type}`"
                                ),
                                span: *span,
                            });
                        }
                    }
                } else {
                    errors.push(MirError {
                        code: "E-MIR-VERIFY-002",
                        message: format!(
                            "indexed assignment target `{name}` must be an array, found `{target_type}`"
                        ),
                        span: *span,
                    });
                }
            }
        }
        MirInstruction::TupleBind {
            bindings,
            value,
            span,
        } => {
            if let Some(Type::Tuple(items)) = expression_type(*value) {
                if items.len() != bindings.len() {
                    errors.push(MirError {
                        code: "E-MIR-VERIFY-002",
                        message: format!(
                            "tuple binding has {} targets but the value contains {} items",
                            bindings.len(),
                            items.len()
                        ),
                        span: *span,
                    });
                }
                for (binding, expected) in bindings.iter().zip(items) {
                    if !binding.is_new {
                        if let Some(actual) = local_types.get(&binding.local) {
                            if actual != expected {
                                errors.push(MirError {
                                    code: "E-MIR-VERIFY-002",
                                    message: format!(
                                        "tuple assignment to local `{}` has type `{expected}`, expected `{actual}`",
                                        binding.name
                                    ),
                                    span: *span,
                                });
                            }
                        }
                    }
                }
            } else if expression_type(*value).is_some() {
                errors.push(MirError {
                    code: "E-MIR-VERIFY-002",
                    message: "tuple binding value must have a tuple type".into(),
                    span: *span,
                });
            }
        }
        MirInstruction::Transaction { operations, span } => {
            for operation in operations {
                let Some(expression) = program.expressions.get(operation.0) else {
                    continue;
                };
                if let MirExpressionKind::Sql { parameters, .. } = &expression.kind {
                    for (name, local) in parameters {
                        if !local_types.contains_key(local) {
                            errors.push(MirError {
                                code: "E-MIR-VERIFY-002",
                                message: format!(
                                    "transaction SQL parameter `:{name}` references unknown local {}",
                                    local.0
                                ),
                                span: *span,
                            });
                        }
                    }
                }
            }
        }
        MirInstruction::Parallel { bindings, span } => {
            if bindings.len() > zelyra_runtime::MAX_PARALLEL_BRANCHES {
                errors.push(MirError {
                    code: "E-MIR-VERIFY-002",
                    message: format!(
                        "parallel block exceeds the maximum of {} branches",
                        zelyra_runtime::MAX_PARALLEL_BRANCHES
                    ),
                    span: *span,
                });
            }
            let mut seen = HashSet::new();
            for binding in bindings {
                if !seen.insert(binding.local) {
                    errors.push(MirError {
                        code: "E-MIR-VERIFY-002",
                        message: format!(
                            "parallel block assigns local `{}` more than once",
                            binding.name
                        ),
                        span: binding.span,
                    });
                }
                let Some(expression) = program.expressions.get(binding.value.0) else {
                    continue;
                };
                if !matches!(expression.kind, MirExpressionKind::Await(_)) {
                    errors.push(MirError {
                        code: "E-MIR-VERIFY-002",
                        message: format!(
                            "parallel binding `{}` must contain an await expression",
                            binding.name
                        ),
                        span: binding.span,
                    });
                }
                if !binding.is_new {
                    if let Some(expected) = local_types.get(&binding.local) {
                        if expected != &expression.ty {
                            errors.push(MirError {
                                code: "E-MIR-VERIFY-002",
                                message: format!(
                                    "parallel assignment to `{}` has type `{}`, expected `{expected}`",
                                    binding.name, expression.ty
                                ),
                                span: binding.span,
                            });
                        }
                    }
                }
            }
        }
        MirInstruction::LoopInvariantCheck {
            expressions, span, ..
        } => {
            for expression in expressions {
                if let Some(actual) = expression_type(*expression) {
                    if actual != &Type::Bool {
                        errors.push(MirError {
                            code: "E-MIR-VERIFY-002",
                            message: format!("loop invariant has type `{actual}`, expected `Bool`"),
                            span: *span,
                        });
                    }
                }
            }
        }
        MirInstruction::IteratorInit { .. }
        | MirInstruction::MatchInit { .. }
        | MirInstruction::Evaluate(_) => {}
    }
}

fn verify_expression_kind(
    program: &MirProgram,
    expression: &MirExpression,
    errors: &mut Vec<MirError>,
) {
    let mut references = Vec::new();
    match &expression.kind {
        MirExpressionKind::Uninitialized => errors.push(MirError {
            code: "E-MIR-VERIFY-001",
            message: format!("expression {} was not fully lowered", expression.id.0),
            span: expression.span,
        }),
        MirExpressionKind::Array(items) => references.extend(items.iter().copied()),
        MirExpressionKind::Map(entries) => {
            references.extend(entries.iter().flat_map(|(key, value)| [*key, *value]))
        }
        MirExpressionKind::Record { fields, .. } => {
            references.extend(fields.iter().map(|(_, value)| *value));
        }
        MirExpressionKind::Lambda { params, body } => {
            references.push(*body);
            match &expression.ty {
                Type::Function {
                    params: parameter_types,
                    return_type,
                    ..
                } => {
                    if parameter_types.len() != params.len() {
                        errors.push(MirError {
                            code: "E-MIR-VERIFY-002",
                            message: "lambda parameter count does not match its function type"
                                .into(),
                            span: expression.span,
                        });
                    }
                    for ((_, name, actual), expected) in params.iter().zip(parameter_types) {
                        if actual != &Type::Unknown && actual != expected {
                            errors.push(MirError {
                                code: "E-MIR-VERIFY-002",
                                message: format!(
                                    "lambda parameter `{name}` has type `{actual}`, function type declares `{expected}`"
                                ),
                                span: expression.span,
                            });
                        }
                    }
                    if let Some(body) = program.expressions.get(body.0) {
                        if body.ty != Type::Unknown
                            && return_type.as_ref() != &Type::Unknown
                            && body.ty != **return_type
                        {
                            errors.push(MirError {
                                code: "E-MIR-VERIFY-002",
                                message: format!(
                                    "lambda body has type `{}`, function type declares `{return_type}`",
                                    body.ty
                                ),
                                span: body.span,
                            });
                        }
                    }
                }
                other => errors.push(MirError {
                    code: "E-MIR-VERIFY-002",
                    message: format!("lambda expression has non-function type `{other}`"),
                    span: expression.span,
                }),
            }
        }
        MirExpressionKind::Index { target, index } => references.extend([*target, *index]),
        MirExpressionKind::Field { target, .. } => references.push(*target),
        MirExpressionKind::Call { args, .. } => references.extend(args.iter().copied()),
        MirExpressionKind::Unary { expr, .. } | MirExpressionKind::Await(expr) => {
            references.push(*expr);
        }
        MirExpressionKind::Binary { left, right, .. } => references.extend([*left, *right]),
        MirExpressionKind::BinaryChain { first, operations } => {
            references.push(*first);
            references.extend(operations.iter().map(|(_, value, _)| *value));
        }
        MirExpressionKind::Match { value, arms } => {
            references.push(*value);
            for arm in arms {
                references.extend(arm.guard);
                if let Some(guard) = arm.guard {
                    verify_expression_type(
                        program,
                        guard,
                        &Type::Bool,
                        arm.span,
                        "match guard",
                        errors,
                    );
                }
                references.push(arm.value);
            }
        }
        MirExpressionKind::Sql { query, .. } => {
            if let Err(message) = zelyra_runtime::check_scoped_sql(query, &program.scoped_tables) {
                errors.push(MirError {
                    code: "E-MIR-VERIFY-003",
                    message,
                    span: expression.span,
                });
            }
        }
        MirExpressionKind::Int(_)
        | MirExpressionKind::UInt(_)
        | MirExpressionKind::Float(_)
        | MirExpressionKind::Bool(_)
        | MirExpressionKind::String(_)
        | MirExpressionKind::Char(_)
        | MirExpressionKind::Local(_)
        | MirExpressionKind::EnumVariant { .. }
        | MirExpressionKind::FunctionValue { .. } => {}
    }
    for reference in references {
        if reference.0 >= program.expressions.len() {
            errors.push(MirError {
                code: "E-MIR-VERIFY-001",
                message: format!(
                    "expression {} contains a child reference outside the expression table",
                    expression.id.0
                ),
                span: expression.span,
            });
        }
    }
}

fn verify_expression_type(
    program: &MirProgram,
    expression: ExpressionId,
    expected: &Type,
    span: Span,
    context: &str,
    errors: &mut Vec<MirError>,
) {
    let Some(actual) = program
        .expressions
        .get(expression.0)
        .map(|expression| &expression.ty)
    else {
        return;
    };
    if actual != expected {
        errors.push(MirError {
            code: "E-MIR-VERIFY-002",
            message: format!("{context} must have type `{expected}`, found `{actual}`"),
            span,
        });
    }
}

fn verify_expression(
    program: &MirProgram,
    expression: ExpressionId,
    block: &MirBasicBlock,
    errors: &mut Vec<MirError>,
) {
    if expression.0 >= program.expressions.len() {
        errors.push(MirError {
            code: "E-MIR-VERIFY-001",
            message: "basic block references a missing expression".into(),
            span: block.span,
        });
    }
}

fn verify_target(
    function: &MirFunction,
    block: &MirBasicBlock,
    target: BlockId,
    errors: &mut Vec<MirError>,
) {
    if target.0 >= function.blocks.len() {
        errors.push(MirError {
            code: "E-MIR-VERIFY-001",
            message: format!(
                "block {} branches outside function `{}`",
                block.id.0, function.name
            ),
            span: block.span,
        });
    }
}

struct Dominators {
    reachable: Vec<bool>,
    by_block: Vec<HashSet<usize>>,
}

impl Dominators {
    fn is_reachable(&self, block: BlockId) -> bool {
        self.reachable.get(block.0).copied().unwrap_or(false)
    }

    fn dominates(&self, definition: BlockId, usage: BlockId) -> bool {
        self.by_block
            .get(usage.0)
            .is_some_and(|dominators| dominators.contains(&definition.0))
    }
}

fn compute_dominators(function: &MirFunction) -> Dominators {
    let count = function.blocks.len();
    let successors = function
        .blocks
        .iter()
        .map(|block| {
            cfg_successors(block)
                .into_iter()
                .filter_map(|target| (target.0 < count).then_some(target.0))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut predecessors = vec![Vec::new(); count];
    for (source, targets) in successors.iter().enumerate() {
        for target in targets {
            predecessors[*target].push(source);
        }
    }
    let mut reachable = vec![false; count];
    let mut pending = vec![function.entry.0];
    while let Some(index) = pending.pop() {
        if index >= count || std::mem::replace(&mut reachable[index], true) {
            continue;
        }
        pending.extend(successors[index].iter().copied());
    }
    let reachable_blocks = reachable
        .iter()
        .enumerate()
        .filter_map(|(index, is_reachable)| is_reachable.then_some(index))
        .collect::<HashSet<_>>();
    let mut by_block = (0..count)
        .map(|index| {
            if index == function.entry.0 || !reachable[index] {
                HashSet::from([index])
            } else {
                reachable_blocks.clone()
            }
        })
        .collect::<Vec<_>>();

    loop {
        let mut changed = false;
        for index in 0..count {
            if !reachable[index] || index == function.entry.0 {
                continue;
            }
            let reachable_predecessors = predecessors[index]
                .iter()
                .copied()
                .filter(|predecessor| reachable[*predecessor])
                .collect::<Vec<_>>();
            let mut next = if let Some(first) = reachable_predecessors.first() {
                by_block[*first].clone()
            } else {
                HashSet::new()
            };
            for predecessor in reachable_predecessors.iter().skip(1) {
                next.retain(|candidate| by_block[*predecessor].contains(candidate));
            }
            next.insert(index);
            if next != by_block[index] {
                by_block[index] = next;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    Dominators {
        reachable,
        by_block,
    }
}

fn cfg_successors(block: &MirBasicBlock) -> Vec<BlockId> {
    match &block.terminator {
        Some(MirTerminator::Goto(target)) => vec![*target],
        Some(MirTerminator::Branch {
            then_block,
            else_block,
            ..
        }) => vec![*then_block, *else_block],
        Some(MirTerminator::IteratorNext {
            body_block,
            exit_block,
            ..
        }) => vec![*body_block, *exit_block],
        Some(MirTerminator::MatchTest {
            matched_block,
            unmatched_block,
            ..
        }) => vec![*matched_block, *unmatched_block],
        Some(
            MirTerminator::Return(_)
            | MirTerminator::ReturnAfterLoopInvariants { .. }
            | MirTerminator::MatchFailure { .. }
            | MirTerminator::Unreachable,
        )
        | None => Vec::new(),
    }
}

struct ProgramBuilder {
    expressions: Vec<MirExpression>,
    expression_ids: HashMap<(u32, usize, usize), ExpressionId>,
    inferred: HashMap<(u32, usize, usize), Type>,
    errors: Vec<MirError>,
}

impl ProgramBuilder {
    fn expression(&mut self, expression: &HirExpr) -> Option<ExpressionId> {
        let key = span_key(expression.span);
        if let Some(id) = self.expression_ids.get(&key) {
            return Some(*id);
        }
        let Some(ty) = self.inferred.get(&key).cloned() else {
            self.errors.push(MirError {
                code: "E-MIR-TYPE-001",
                message: "type checker did not provide a type for this expression".into(),
                span: expression.span,
            });
            return None;
        };
        let id = ExpressionId(self.expressions.len());
        self.expression_ids.insert(key, id);
        self.expressions.push(MirExpression {
            id,
            span: expression.span,
            ty,
            kind: MirExpressionKind::Uninitialized,
        });
        let kind = self.lower_expression_kind(expression);
        self.expressions[id.0].kind = kind;
        Some(id)
    }

    fn expression_id(&mut self, expression: &HirExpr) -> ExpressionId {
        self.expression(expression)
            .unwrap_or(ExpressionId(usize::MAX))
    }

    fn lower_expression_kind(&mut self, expression: &HirExpr) -> MirExpressionKind {
        match &expression.kind {
            HirExprKind::Int(value) => MirExpressionKind::Int(*value),
            HirExprKind::UInt(value) => MirExpressionKind::UInt(*value),
            HirExprKind::Float(value) => MirExpressionKind::Float(*value),
            HirExprKind::Bool(value) => MirExpressionKind::Bool(*value),
            HirExprKind::String(value) => MirExpressionKind::String(value.clone()),
            HirExprKind::Char(value) => MirExpressionKind::Char(*value),
            HirExprKind::Array(items) => MirExpressionKind::Array(
                items.iter().map(|item| self.expression_id(item)).collect(),
            ),
            HirExprKind::Map(entries) => MirExpressionKind::Map(
                entries
                    .iter()
                    .map(|(key, value)| (self.expression_id(key), self.expression_id(value)))
                    .collect(),
            ),
            HirExprKind::Record {
                type_name,
                type_args,
                fields,
            } => MirExpressionKind::Record {
                type_name: type_name.clone(),
                type_args: type_args.clone(),
                fields: fields
                    .iter()
                    .map(|(name, value)| (name.clone(), self.expression_id(value)))
                    .collect(),
            },
            HirExprKind::Lambda { params, body } => MirExpressionKind::Lambda {
                params: params
                    .iter()
                    .map(|parameter| {
                        (
                            parameter.local,
                            parameter.name.clone(),
                            parameter.ty.clone(),
                        )
                    })
                    .collect(),
                body: self.expression_id(body),
            },
            HirExprKind::Local(local) => MirExpressionKind::Local(*local),
            HirExprKind::EnumVariant { enum_name, variant } => MirExpressionKind::EnumVariant {
                enum_name: enum_name.clone(),
                variant: variant.clone(),
            },
            HirExprKind::FunctionValue { name, function } => MirExpressionKind::FunctionValue {
                name: name.clone(),
                function: *function,
            },
            HirExprKind::Index { target, index } => MirExpressionKind::Index {
                target: self.expression_id(target),
                index: self.expression_id(index),
            },
            HirExprKind::Field { target, field } => MirExpressionKind::Field {
                target: self.expression_id(target),
                field: field.clone(),
            },
            HirExprKind::Call {
                name,
                function,
                callee_local,
                type_args,
                args,
            } => MirExpressionKind::Call {
                name: name.clone(),
                function: *function,
                callee_local: *callee_local,
                type_args: type_args.clone(),
                args: args
                    .iter()
                    .map(|argument| self.expression_id(argument))
                    .collect(),
            },
            HirExprKind::Unary { op, expr } => MirExpressionKind::Unary {
                op: *op,
                expr: self.expression_id(expr),
            },
            HirExprKind::Binary { left, op, right } => MirExpressionKind::Binary {
                left: self.expression_id(left),
                op: *op,
                right: self.expression_id(right),
            },
            HirExprKind::BinaryChain { first, operations } => MirExpressionKind::BinaryChain {
                first: self.expression_id(first),
                operations: operations
                    .iter()
                    .map(|(op, value, span)| (*op, self.expression_id(value), *span))
                    .collect(),
            },
            HirExprKind::Sql {
                result_type,
                query,
                parameters,
            } => MirExpressionKind::Sql {
                result_type: result_type.clone(),
                query: query.clone(),
                parameters: parameters.clone(),
            },
            HirExprKind::Await(value) => MirExpressionKind::Await(self.expression_id(value)),
            HirExprKind::Match { value, arms } => MirExpressionKind::Match {
                value: self.expression_id(value),
                arms: arms
                    .iter()
                    .map(|arm| MirMatchExpressionArm {
                        pattern: lower_pattern(&arm.pattern),
                        guard: arm.guard.as_ref().map(|guard| self.expression_id(guard)),
                        value: self.expression_id(&arm.value),
                        span: arm.span,
                    })
                    .collect(),
            },
        }
    }

    fn function(&mut self, function: &zelyra_hir::HirFunction) -> MirFunction {
        let mut cfg = FunctionBuilder {
            program: self,
            blocks: Vec::new(),
            loop_targets: Vec::new(),
            next_iterator: 0,
            next_match: 0,
            return_type: function.return_type.clone(),
        };
        let entry = cfg.block(function.span);
        let requires = function
            .requires
            .iter()
            .filter_map(|expr| cfg.program.expression(expr))
            .collect();
        let ensures = function
            .ensures
            .iter()
            .filter_map(|expr| cfg.program.expression(expr))
            .collect();
        let end = cfg.lower_block(&function.body, Some(entry));
        if let Some(block) = end {
            cfg.terminate(block, MirTerminator::Return(None));
        }
        MirFunction {
            id: function.id,
            name: function.name.clone(),
            parameters: function
                .params
                .iter()
                .map(|parameter| {
                    (
                        parameter.local,
                        parameter.name.clone(),
                        parameter.ty.clone(),
                        parameter.span,
                    )
                })
                .collect(),
            declared_return_type: function.return_type.clone(),
            declared_capabilities: function.capabilities.clone(),
            requires,
            ensures,
            contract_result_local: function.contract_result_local,
            entry,
            blocks: cfg.blocks,
            span: function.span,
        }
    }
}

fn lower_pattern(pattern: &HirPattern) -> MirPattern {
    let kind = match &pattern.kind {
        HirPatternKind::Wildcard => MirPatternKind::Wildcard,
        HirPatternKind::Binding { name, local } => MirPatternKind::Binding {
            name: name.clone(),
            local: *local,
        },
        HirPatternKind::Constructor { name, inner } => MirPatternKind::Constructor {
            name: name.clone(),
            inner: inner.as_deref().map(lower_pattern).map(Box::new),
        },
        HirPatternKind::EnumVariant { enum_name, variant } => MirPatternKind::EnumVariant {
            enum_name: enum_name.clone(),
            variant: variant.clone(),
        },
        HirPatternKind::EnumVariantPayload {
            enum_name,
            variant,
            payloads,
        } => MirPatternKind::EnumVariantPayload {
            enum_name: enum_name.clone(),
            variant: variant.clone(),
            payloads: payloads.iter().map(lower_pattern).collect(),
        },
        HirPatternKind::Record { type_name, fields } => MirPatternKind::Record {
            type_name: type_name.clone(),
            fields: fields
                .iter()
                .map(|(name, pattern)| (name.clone(), lower_pattern(pattern)))
                .collect(),
        },
        HirPatternKind::Int(value) => MirPatternKind::Int(*value),
        HirPatternKind::Bool(value) => MirPatternKind::Bool(*value),
        HirPatternKind::String(value) => MirPatternKind::String(value.clone()),
        HirPatternKind::Char(value) => MirPatternKind::Char(*value),
    };
    MirPattern {
        kind,
        span: pattern.span,
    }
}

struct FunctionBuilder<'a> {
    program: &'a mut ProgramBuilder,
    blocks: Vec<MirBasicBlock>,
    loop_targets: Vec<LoopTarget>,
    next_iterator: usize,
    next_match: usize,
    return_type: Option<Type>,
}

#[derive(Clone)]
struct LoopTarget {
    break_target: BlockId,
    continue_target: BlockId,
    invariants: Vec<ExpressionId>,
    loop_span: Span,
}

impl FunctionBuilder<'_> {
    fn block(&mut self, span: Span) -> BlockId {
        let id = BlockId(self.blocks.len());
        self.blocks.push(MirBasicBlock {
            id,
            instructions: Vec::new(),
            loop_invariants: Vec::new(),
            loop_span: None,
            terminator: None,
            span,
        });
        id
    }

    fn terminate(&mut self, block: BlockId, terminator: MirTerminator) {
        self.blocks[block.0].terminator = Some(terminator);
    }

    fn emit(&mut self, block: BlockId, instruction: MirInstruction) {
        self.blocks[block.0].instructions.push(instruction);
    }

    fn lower_block(&mut self, source: &HirBlock, mut current: Option<BlockId>) -> Option<BlockId> {
        for statement in &source.statements {
            let Some(block) = current else { break };
            current = self.statement(statement, block);
        }
        current
    }

    fn statement(&mut self, statement: &HirStmt, block: BlockId) -> Option<BlockId> {
        match statement {
            HirStmt::Let {
                name,
                local,
                ty,
                value,
                mutable,
                span,
            } => {
                if let Some(value) = self.program.expression(value) {
                    if let Some(expected @ Type::Map(_, _)) = ty.as_ref() {
                        if let Some(expression) = self.program.expressions.get_mut(value.0) {
                            if matches!(&expression.kind, MirExpressionKind::Map(entries) if entries.is_empty())
                                && expression.ty
                                    == Type::Map(Box::new(Type::Unknown), Box::new(Type::Unknown))
                            {
                                // An empty literal has no element evidence of its own;
                                // use the already type-checked declaration context.
                                expression.ty = expected.clone();
                            }
                        }
                    }
                    self.emit(
                        block,
                        MirInstruction::Let {
                            local: *local,
                            name: name.clone(),
                            mutable: *mutable,
                            declared_type: ty.clone(),
                            value,
                            span: *span,
                        },
                    );
                }
                Some(block)
            }
            HirStmt::Assign {
                name,
                local,
                value,
                span,
            } => {
                if let Some(value) = self.program.expression(value) {
                    self.emit(
                        block,
                        MirInstruction::Assign {
                            local: *local,
                            name: name.clone(),
                            value,
                            span: *span,
                        },
                    );
                }
                Some(block)
            }
            HirStmt::ArrayIndexAssign {
                name,
                local,
                index,
                value,
                operation,
                span,
            } => {
                let index = self.program.expression(index);
                let value = self.program.expression(value);
                if let (Some(index), Some(value)) = (index, value) {
                    self.emit(
                        block,
                        MirInstruction::ArrayIndexAssign {
                            local: *local,
                            name: name.clone(),
                            index,
                            value,
                            operation: *operation,
                            span: *span,
                        },
                    );
                }
                Some(block)
            }
            HirStmt::TupleBindOrAssign {
                bindings,
                value,
                span,
            } => {
                if let Some(value) = self.program.expression(value) {
                    self.emit(
                        block,
                        MirInstruction::TupleBind {
                            bindings: bindings
                                .iter()
                                .map(|binding| MirTupleBinding {
                                    local: binding.local,
                                    name: binding.name.clone(),
                                    is_new: binding.is_new,
                                })
                                .collect(),
                            value,
                            span: *span,
                        },
                    );
                }
                Some(block)
            }
            HirStmt::Expr(expression) => {
                if let Some(expression) = self.program.expression(expression) {
                    self.emit(block, MirInstruction::Evaluate(expression));
                }
                Some(block)
            }
            HirStmt::Transaction { body, span } => {
                let mut operations = Vec::with_capacity(body.statements.len());
                for statement in &body.statements {
                    let (expression, allow_mail_queue) = match statement {
                        HirStmt::Expr(expression) => (Some(expression), true),
                        HirStmt::Let { value, .. } => (Some(value), false),
                        _ => (None, false),
                    };
                    let Some(expression) = expression else {
                        self.program.errors.push(MirError {
                            code: "E-MIR-UNSUPPORTED-001",
                            message: "MIR transactions currently support only direct SQL statements and SQL let-expressions".into(),
                            span: stmt_span(statement),
                        });
                        return Some(block);
                    };
                    let direct_sql = matches!(expression.kind, HirExprKind::Sql { .. });
                    let standalone_mail_queue = allow_mail_queue
                        && matches!(
                            &expression.kind,
                            HirExprKind::Call { name, args, .. }
                                if name == "mail_queue"
                                    && matches!(args.len(), 3..=6)
                                    && args.iter().all(is_pure_mail_expression)
                        );
                    if !direct_sql && !standalone_mail_queue {
                        self.program.errors.push(MirError {
                            code: "E-MIR-UNSUPPORTED-001",
                            message: "MIR transactions currently support direct SQL statements, SQL let-expressions, and pure standalone mail_queue calls with optional render_pdf attachments".into(),
                            span: expression.span,
                        });
                        return Some(block);
                    }
                    if let Some(expression) = self.program.expression(expression) {
                        operations.push(expression);
                    }
                }
                if operations.is_empty() {
                    self.program.errors.push(MirError {
                        code: "E-MIR-UNSUPPORTED-001",
                        message:
                            "MIR transactions require at least one SQL or mail_queue operation"
                                .into(),
                        span: *span,
                    });
                    return Some(block);
                }
                self.emit(
                    block,
                    MirInstruction::Transaction {
                        operations,
                        span: *span,
                    },
                );
                Some(block)
            }
            HirStmt::Parallel { body, span } => {
                let mut bindings = Vec::with_capacity(body.statements.len());
                for statement in &body.statements {
                    let (local, name, is_new, value, binding_span) = match statement {
                        HirStmt::Let {
                            local,
                            name,
                            value,
                            span,
                            ..
                        } => (*local, name.clone(), true, value, *span),
                        HirStmt::Assign {
                            local,
                            name,
                            value,
                            span,
                        } => (*local, name.clone(), false, value, *span),
                        _ => {
                            self.program.errors.push(MirError {
                                code: "E-MIR-UNSUPPORTED-001",
                                message: "parallel blocks support only `name = await expression` bindings".into(),
                                span: stmt_span(statement),
                            });
                            return Some(block);
                        }
                    };
                    if !matches!(value.kind, HirExprKind::Await(_)) {
                        self.program.errors.push(MirError {
                            code: "E-MIR-UNSUPPORTED-001",
                            message: "parallel bindings must await their expression".into(),
                            span: binding_span,
                        });
                        return Some(block);
                    }
                    if let Some(value) = self.program.expression(value) {
                        bindings.push(MirParallelBinding {
                            local,
                            name,
                            is_new,
                            value,
                            span: binding_span,
                        });
                    }
                }
                self.emit(
                    block,
                    MirInstruction::Parallel {
                        bindings,
                        span: *span,
                    },
                );
                Some(block)
            }
            HirStmt::Return { value, .. } => {
                let value = value.as_ref().and_then(|expression| {
                    let id = self.program.expression(expression)?;
                    let is_contextual_constructor = matches!(
                        &expression.kind,
                        HirExprKind::Call { name, .. }
                            if matches!(name.as_str(), "Ok" | "Err" | "None")
                    );
                    if is_contextual_constructor {
                        if let Some(expected) = &self.return_type {
                            // The full compiler check above already validated this
                            // constructor against the function's declared return
                            // type. Preserve that contextual generic information
                            // in MIR (for example Err(String) -> Result<Int, String>).
                            self.program.expressions[id.0].ty = expected.clone();
                        }
                    }
                    Some(id)
                });
                let invariants = self
                    .loop_targets
                    .iter()
                    .rev()
                    .flat_map(|target| {
                        target
                            .invariants
                            .iter()
                            .copied()
                            .map(|expression| MirLoopInvariant {
                                expression,
                                loop_span: target.loop_span,
                            })
                    })
                    .collect::<Vec<_>>();
                let terminator = if invariants.is_empty() {
                    MirTerminator::Return(value)
                } else {
                    MirTerminator::ReturnAfterLoopInvariants { value, invariants }
                };
                self.terminate(block, terminator);
                None
            }
            HirStmt::If {
                condition,
                then_block,
                else_block,
                span,
            } => {
                let Some(condition) = self.program.expression(condition) else {
                    return Some(block);
                };
                let then_id = self.block(then_block.span);
                let else_id = else_block
                    .as_ref()
                    .map(|item| self.block(item.span))
                    .unwrap_or_else(|| self.block(*span));
                let merge_id = self.block(*span);
                self.terminate(
                    block,
                    MirTerminator::Branch {
                        condition,
                        then_block: then_id,
                        else_block: else_id,
                    },
                );
                let then_end = self.lower_block(then_block, Some(then_id));
                if let Some(end) = then_end {
                    self.terminate(end, MirTerminator::Goto(merge_id));
                }
                let else_end = if let Some(else_block) = else_block {
                    self.lower_block(else_block, Some(else_id))
                } else {
                    Some(else_id)
                };
                if let Some(end) = else_end {
                    self.terminate(end, MirTerminator::Goto(merge_id));
                }
                if then_end.is_some() || else_end.is_some() {
                    Some(merge_id)
                } else {
                    self.terminate(merge_id, MirTerminator::Unreachable);
                    None
                }
            }
            HirStmt::While {
                condition,
                invariants,
                body,
                span,
            } => {
                let condition_id = self.block(condition.span);
                let body_id = self.block(body.span);
                let exit_id = self.block(*span);
                self.terminate(block, MirTerminator::Goto(condition_id));
                let condition_invariants = invariants
                    .iter()
                    .filter_map(|invariant| self.program.expression(invariant))
                    .collect();
                self.blocks[condition_id.0].loop_invariants = condition_invariants;
                self.blocks[condition_id.0].loop_span = Some(*span);
                if let Some(condition) = self.program.expression(condition) {
                    self.terminate(
                        condition_id,
                        MirTerminator::Branch {
                            condition,
                            then_block: body_id,
                            else_block: exit_id,
                        },
                    );
                }
                self.loop_targets.push(LoopTarget {
                    break_target: exit_id,
                    continue_target: condition_id,
                    invariants: self.blocks[condition_id.0].loop_invariants.clone(),
                    loop_span: *span,
                });
                let body_end = self.lower_block(body, Some(body_id));
                self.loop_targets.pop();
                if let Some(end) = body_end {
                    let invariants = self.blocks[condition_id.0].loop_invariants.clone();
                    if !invariants.is_empty() {
                        self.emit(
                            end,
                            MirInstruction::LoopInvariantCheck {
                                expressions: invariants,
                                phase: LoopInvariantPhase::After,
                                span: *span,
                            },
                        );
                    }
                    self.terminate(end, MirTerminator::Goto(condition_id));
                }
                Some(exit_id)
            }
            HirStmt::For {
                name,
                local,
                additional_names,
                additional_locals,
                iterable,
                body,
                span,
            } => {
                if additional_names.len() != additional_locals.len() {
                    self.program.errors.push(MirError {
                        code: "E-MIR-RESOLVE-001",
                        message: "tuple loop binding names and local IDs have different lengths"
                            .into(),
                        span: *span,
                    });
                    return Some(block);
                }
                let source = match &iterable.kind {
                    HirExprKind::Call {
                        name: call, args, ..
                    } if call == "range" => {
                        if !(2..=3).contains(&args.len()) {
                            self.program.errors.push(MirError {
                                code: "E-MIR-LOWER-001",
                                message:
                                    "type-checked range loop has an unsupported argument count"
                                        .into(),
                                span: iterable.span,
                            });
                            return Some(block);
                        }
                        MirIteratorSource::IntegerRange {
                            start: self.program.expression_id(&args[0]),
                            end: self.program.expression_id(&args[1]),
                            step: args.get(2).map(|step| self.program.expression_id(step)),
                        }
                    }
                    _ => MirIteratorSource::Collection(self.program.expression_id(iterable)),
                };
                let iterator = IteratorId(self.next_iterator);
                self.next_iterator += 1;
                self.emit(
                    block,
                    MirInstruction::IteratorInit {
                        iterator,
                        source,
                        span: *span,
                    },
                );
                let header_id = self.block(*span);
                let body_id = self.block(body.span);
                let exit_id = self.block(*span);
                self.terminate(block, MirTerminator::Goto(header_id));
                let mut bindings = vec![MirIteratorBinding {
                    local: *local,
                    name: name.clone(),
                }];
                bindings.extend(additional_names.iter().zip(additional_locals).map(
                    |(name, local)| MirIteratorBinding {
                        local: *local,
                        name: name.clone(),
                    },
                ));
                self.terminate(
                    header_id,
                    MirTerminator::IteratorNext {
                        iterator,
                        bindings,
                        body_block: body_id,
                        exit_block: exit_id,
                    },
                );
                self.loop_targets.push(LoopTarget {
                    break_target: exit_id,
                    continue_target: header_id,
                    invariants: Vec::new(),
                    loop_span: *span,
                });
                let body_end = self.lower_block(body, Some(body_id));
                self.loop_targets.pop();
                if let Some(end) = body_end {
                    self.terminate(end, MirTerminator::Goto(header_id));
                }
                Some(exit_id)
            }
            HirStmt::Match { value, arms, span } => {
                let Some(value) = self.program.expression(value) else {
                    return Some(block);
                };
                let match_id = MatchId(self.next_match);
                self.next_match += 1;
                self.emit(
                    block,
                    MirInstruction::MatchInit {
                        match_id,
                        value,
                        span: *span,
                    },
                );
                let merge_id = self.block(*span);
                let mut test_block = block;
                let mut has_fallthrough = false;
                for arm in arms {
                    let matched_id = self.block(arm.body.span);
                    let next_test_id = self.block(*span);
                    self.terminate(
                        test_block,
                        MirTerminator::MatchTest {
                            match_id,
                            pattern: lower_pattern(&arm.pattern),
                            matched_block: matched_id,
                            unmatched_block: next_test_id,
                        },
                    );
                    let body_entry = if let Some(guard) = &arm.guard {
                        let Some(guard) = self.program.expression(guard) else {
                            test_block = next_test_id;
                            continue;
                        };
                        let body_id = self.block(arm.body.span);
                        self.terminate(
                            matched_id,
                            MirTerminator::Branch {
                                condition: guard,
                                then_block: body_id,
                                else_block: next_test_id,
                            },
                        );
                        body_id
                    } else {
                        matched_id
                    };
                    if let Some(end) = self.lower_block(&arm.body, Some(body_entry)) {
                        self.terminate(end, MirTerminator::Goto(merge_id));
                        has_fallthrough = true;
                    }
                    test_block = next_test_id;
                }
                self.terminate(test_block, MirTerminator::MatchFailure { span: *span });
                if has_fallthrough {
                    Some(merge_id)
                } else {
                    self.terminate(merge_id, MirTerminator::Unreachable);
                    None
                }
            }
            HirStmt::Loop {
                invariants,
                body,
                span,
            } => {
                let body_id = self.block(body.span);
                let exit_id = self.block(*span);
                self.terminate(block, MirTerminator::Goto(body_id));
                let loop_invariants = invariants
                    .iter()
                    .filter_map(|invariant| self.program.expression(invariant))
                    .collect();
                self.blocks[body_id.0].loop_invariants = loop_invariants;
                self.blocks[body_id.0].loop_span = Some(*span);
                self.loop_targets.push(LoopTarget {
                    break_target: exit_id,
                    continue_target: body_id,
                    invariants: self.blocks[body_id.0].loop_invariants.clone(),
                    loop_span: *span,
                });
                let body_end = self.lower_block(body, Some(body_id));
                self.loop_targets.pop();
                if let Some(end) = body_end {
                    let invariants = self.blocks[body_id.0].loop_invariants.clone();
                    if !invariants.is_empty() {
                        self.emit(
                            end,
                            MirInstruction::LoopInvariantCheck {
                                expressions: invariants,
                                phase: LoopInvariantPhase::After,
                                span: *span,
                            },
                        );
                    }
                    self.terminate(end, MirTerminator::Goto(body_id));
                }
                Some(exit_id)
            }
            HirStmt::Break { span } => self.loop_jump(block, true, *span),
            HirStmt::Continue { span } => self.loop_jump(block, false, *span),
        }
    }

    fn loop_jump(&mut self, block: BlockId, is_break: bool, span: Span) -> Option<BlockId> {
        if let Some(target) = self.loop_targets.last().cloned() {
            if !target.invariants.is_empty() {
                self.emit(
                    block,
                    MirInstruction::LoopInvariantCheck {
                        expressions: target.invariants,
                        phase: LoopInvariantPhase::After,
                        span: target.loop_span,
                    },
                );
            }
            self.terminate(
                block,
                MirTerminator::Goto(if is_break {
                    target.break_target
                } else {
                    target.continue_target
                }),
            );
        } else {
            self.program.errors.push(MirError {
                code: "E-MIR-CFG-001",
                message: "loop jump has no enclosing loop target".into(),
                span,
            });
        }
        None
    }
}

fn is_pure_mail_expression(expression: &HirExpr) -> bool {
    match &expression.kind {
        HirExprKind::String(_) | HirExprKind::Local(_) => true,
        HirExprKind::Field { target, .. } => is_pure_mail_expression(target),
        HirExprKind::Record { fields, .. } => fields
            .iter()
            .all(|(_, value)| is_pure_mail_expression(value)),
        HirExprKind::Map(entries) => entries
            .iter()
            .all(|(key, value)| is_pure_mail_expression(key) && is_pure_mail_expression(value)),
        HirExprKind::Call { name, args, .. }
            if name == "render_pdf"
                && args.len() == 2
                && matches!(args[0].kind, HirExprKind::String(_)) =>
        {
            is_pure_mail_expression(&args[1])
        }
        HirExprKind::Binary { left, op, right } if *op == BinaryOp::Add => {
            is_pure_mail_expression(left) && is_pure_mail_expression(right)
        }
        _ => false,
    }
}

fn stmt_span(statement: &HirStmt) -> Span {
    match statement {
        HirStmt::Let { span, .. }
        | HirStmt::Assign { span, .. }
        | HirStmt::ArrayIndexAssign { span, .. }
        | HirStmt::TupleBindOrAssign { span, .. }
        | HirStmt::Return { span, .. }
        | HirStmt::If { span, .. }
        | HirStmt::While { span, .. }
        | HirStmt::For { span, .. }
        | HirStmt::Loop { span, .. }
        | HirStmt::Break { span }
        | HirStmt::Continue { span }
        | HirStmt::Match { span, .. }
        | HirStmt::Transaction { span, .. }
        | HirStmt::Parallel { span, .. } => *span,
        HirStmt::Expr(expression) => expression.span,
    }
}

fn span_key(span: Span) -> (u32, usize, usize) {
    (span.source_id, span.start, span.end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zelyra_lexer::lex;
    use zelyra_parser::parse;

    fn parse_program(source: &str) -> zelyra_ast::Program {
        parse(&lex(source).expect("lex source")).expect("parse source")
    }

    #[test]
    fn lowering_reuses_checked_capability_rules_before_creating_mir() {
        let missing_caller_capability = parse_program(
            "fn current_time() -> Timestamp uses Clock { return now() } fn main() { current_time() }",
        );
        let errors = lower(&missing_caller_capability).expect_err(
            "a caller cannot invoke a capability-bearing function without declaring it",
        );
        assert!(
            errors.iter().any(|error| {
                error.code == "E-MIR-EFFECT-001"
                    && error.message.contains("requires capability `Clock`")
            }),
            "unexpected capability diagnostics: {errors:?}"
        );

        let declared_capability = parse_program(
            "fn current_time() -> Timestamp uses Clock { return now() } fn main() uses Clock { current_time() }",
        );
        lower(&declared_capability).expect("valid capability declarations lower");
    }

    #[test]
    fn lowering_rejects_raw_sql_against_tenant_scoped_tables() {
        let source = parse_program(
            "type TenantId = Id table invoices scoped by tenant_id: TenantId { id: Id primary auto tenant_id: TenantId required } fn main() uses Database(read) { rows = sql<Int[]> { SELECT id FROM invoices } }",
        );
        let errors = lower(&source).expect_err("MIR must fail closed for scoped raw SQL");
        assert!(errors.iter().any(|error| {
            error.code == "E-MIR-VERIFY-003"
                && error.message.contains("tenant-scoped table `invoices`")
        }));

        let scoped_program = parse_program(
            "type TenantId = Id table invoices scoped by tenant_id: TenantId { id: Id primary auto tenant_id: TenantId required } fn main() {}",
        );
        let mut mir = lower(&scoped_program).expect("scoped metadata lowers");
        mir.scoped_tables.clear();
        let errors = verify(&mir).expect_err("MIR cannot strip checked scope metadata");
        assert!(errors.iter().any(|error| {
            error.code == "E-MIR-VERIFY-003"
                && error
                    .message
                    .contains("does not match the checked project schema")
        }));
    }

    #[test]
    fn verifier_rejects_call_edges_that_exceed_the_callers_capabilities() {
        let source = parse_program(
            "fn current_time() -> Timestamp uses Clock { return now() } fn main() uses Clock { current_time() }",
        );
        let mut mir = lower(&source).expect("valid capability declarations lower");
        mir.functions
            .iter_mut()
            .find(|function| function.name == "main")
            .unwrap()
            .declared_capabilities
            .clear();

        let errors = verify(&mir).expect_err("caller cannot lose a callee capability");
        assert!(
            errors.iter().any(|error| {
                error.code == "E-MIR-EFFECT-001"
                    && error.message.contains("calls `current_time`")
                    && error.message.contains("`Clock`")
            }),
            "unexpected verifier errors: {errors:?}"
        );
    }

    #[test]
    fn verifier_rejects_direct_host_and_sql_effects_after_mir_mutation() {
        let clock_source = parse_program("fn main() uses Clock { now() }");
        let mut clock_mir = lower(&clock_source).expect("declared clock effect lowers");
        clock_mir.functions[0].declared_capabilities.clear();
        let errors = verify(&clock_mir).expect_err("direct clock effect needs a declaration");
        assert!(errors
            .iter()
            .any(|error| { error.code == "E-MIR-EFFECT-001" && error.message.contains("Clock") }));

        let sql_source = parse_program(
            "database main { engine: mariadb database: \"effect_test\" } table users { id: Id primary auto } fn read_users() -> Int[] uses Database(read) { return sql<Int[]> { SELECT id FROM users } } fn main() {}",
        );
        let mut sql_mir = lower(&sql_source).expect("declared SQL read effect lowers");
        sql_mir
            .functions
            .iter_mut()
            .find(|function| function.name == "read_users")
            .unwrap()
            .declared_capabilities
            .clear();
        let errors = verify(&sql_mir).expect_err("SQL effect needs a declaration");
        assert!(errors.iter().any(|error| {
            error.code == "E-MIR-EFFECT-001" && error.message.contains("Database(read)")
        }));
    }

    #[test]
    fn verifier_rejects_effectful_higher_order_callbacks_after_mir_mutation() {
        for source in [
            "fn clocked(value: Int) -> Int uses Clock { now(); return value } fn main() uses Clock { mapped = map([1], &clocked) }",
            "fn clocked(value: Int) -> Int uses Clock { now(); return value } fn main() uses Clock { callback = &clocked; mapped = map([1], callback) }",
        ] {
            let source = parse_program(source);
            let mut mir = lower(&source).expect("effectful callback with a grant lowers");
            mir.functions
                .iter_mut()
                .find(|function| function.name == "main")
                .unwrap()
                .declared_capabilities
                .clear();

            let errors =
                verify(&mir).expect_err("callback capability must be declared by its caller");
            assert!(
                errors.iter().any(|error| {
                    error.code == "E-MIR-EFFECT-001"
                        && error.message.contains("passes a callback to `map`")
                        && error.message.contains("Clock")
                }),
                "unexpected verifier errors: {errors:?}"
            );
        }
    }

    #[test]
    fn lowers_checked_if_and_loop_statements_to_verified_cfg() {
        let source = r#"
fn count(limit: Int) -> Int {
    mutable total: Int = 0
    while total < limit invariant { total >= 0 } {
        if total == 2 { total = total + 1; continue }
        total = total + 1
    }
    return total
}
fn main() { print(count(4)) }
"#;
        let mir = lower(&parse_program(source)).expect("supported control flow lowers");
        verify(&mir).expect("lowered graph verifies");
        let function = mir
            .functions
            .iter()
            .find(|function| function.name == "count")
            .unwrap();
        assert!(function.blocks.len() >= 6);
        assert!(function
            .blocks
            .iter()
            .any(|block| matches!(block.terminator, Some(MirTerminator::Branch { .. }))));
        assert!(function
            .blocks
            .iter()
            .any(|block| !block.loop_invariants.is_empty()));
        assert!(mir
            .expressions
            .iter()
            .any(|expression| expression.ty == Type::Bool));
    }

    #[test]
    fn verifier_rejects_control_flow_and_return_type_inconsistency() {
        let source = r#"
fn positive(value: Int) -> Bool {
    if value > 0 { return true } else { return false }
}
fn main() { print(positive(1)) }
"#;
        let valid = lower(&parse_program(source)).expect("source types check and lower");
        verify(&valid).expect("lowered branch and returns are correctly typed");

        let mut invalid_branch = valid.clone();
        let positive = invalid_branch
            .functions
            .iter()
            .find(|function| function.name == "positive")
            .unwrap();
        let condition = positive
            .blocks
            .iter()
            .find_map(|block| match block.terminator {
                Some(MirTerminator::Branch { condition, .. }) => Some(condition),
                _ => None,
            })
            .unwrap();
        invalid_branch.expressions[condition.0].ty = Type::Int;
        let errors = verify(&invalid_branch)
            .expect_err("the verifier must reject a non-Bool branch condition");
        assert!(errors.iter().any(|error| {
            error.code == "E-MIR-VERIFY-002" && error.message.contains("branch condition")
        }));

        let mut invalid_return = valid;
        let positive = invalid_return
            .functions
            .iter()
            .find(|function| function.name == "positive")
            .unwrap();
        let returned = positive
            .blocks
            .iter()
            .find_map(|block| match block.terminator {
                Some(MirTerminator::Return(Some(value))) => Some(value),
                _ => None,
            })
            .unwrap();
        invalid_return.expressions[returned.0].ty = Type::String;
        let errors = verify(&invalid_return)
            .expect_err("the verifier must reject a return type that contradicts the signature");
        assert!(errors.iter().any(|error| {
            error.code == "E-MIR-VERIFY-002" && error.message.contains("declares `Bool`")
        }));
    }

    #[test]
    fn verifier_checks_contract_boolean_types_and_result_local_identity() {
        let source = "fn identity(value: Int) -> Int requires { value >= 0 } ensures { result >= value } { return value } fn main() {}";
        let valid = lower(&parse_program(source)).expect("contract source lowers");
        verify(&valid).expect("well-typed contracts verify");

        let identity = valid
            .functions
            .iter()
            .find(|function| function.name == "identity")
            .unwrap();
        let contract_id = identity.ensures[0];
        let mut invalid_type = valid.clone();
        invalid_type.expressions[contract_id.0].ty = Type::String;
        let errors = verify(&invalid_type).expect_err("non-Bool contract MIR must be rejected");
        assert!(errors.iter().any(|error| {
            error.code == "E-MIR-VERIFY-002" && error.message.contains("function contract")
        }));

        let mut invalid_result_local = valid;
        let identity = invalid_result_local
            .functions
            .iter_mut()
            .find(|function| function.name == "identity")
            .unwrap();
        identity.contract_result_local = identity.parameters[0].0;
        let errors = verify(&invalid_result_local)
            .expect_err("contract result local must be distinct from parameter locals");
        assert!(errors.iter().any(|error| {
            error.code == "E-MIR-VERIFY-001"
                && error.message.contains("contract result local conflicts")
        }));
    }

    #[test]
    fn verifier_rejects_local_initializer_and_assignment_type_mismatches() {
        let source = r#"
fn main() {
    mutable count: Int = 0
    flag = true
    count = 1
}
"#;
        let valid = lower(&parse_program(source)).expect("source types check and lower");
        verify(&valid).expect("well-typed local operations verify");

        let mut invalid_initializer = valid.clone();
        let initialized = invalid_initializer
            .functions
            .iter()
            .flat_map(|function| &function.blocks)
            .flat_map(|block| &block.instructions)
            .find_map(|instruction| match instruction {
                MirInstruction::Let {
                    name,
                    declared_type: Some(Type::Int),
                    value,
                    ..
                } if name == "count" => Some(*value),
                _ => None,
            })
            .unwrap();
        invalid_initializer.expressions[initialized.0].ty = Type::Bool;
        let errors = verify(&invalid_initializer)
            .expect_err("initializer type must agree with its declared type");
        assert!(errors.iter().any(|error| {
            error.code == "E-MIR-VERIFY-002"
                && error.message.contains("initializer for local `count`")
        }));

        let mut invalid_assignment = valid;
        let boolean = invalid_assignment
            .expressions
            .iter()
            .find(|expression| matches!(expression.kind, MirExpressionKind::Bool(true)))
            .map(|expression| expression.id)
            .unwrap();
        let assignment = invalid_assignment
            .functions
            .iter_mut()
            .flat_map(|function| &mut function.blocks)
            .flat_map(|block| &mut block.instructions)
            .find_map(|instruction| match instruction {
                MirInstruction::Assign { name, value, .. } if name == "count" => Some(value),
                _ => None,
            })
            .unwrap();
        *assignment = boolean;
        let errors = verify(&invalid_assignment)
            .expect_err("assignment type must agree with the local's declared type");
        assert!(errors.iter().any(|error| {
            error.code == "E-MIR-VERIFY-002"
                && error.message.contains("assignment to local `count`")
        }));
    }

    #[test]
    fn lowers_expressions_to_typed_operations_with_explicit_child_ids() {
        let source = r#"
fn add(left: Int, right: Int) -> Int { return left + right }
fn main() {
    value = add(20, 22)
    print(value)
}
"#;
        let mir = lower(&parse_program(source)).expect("typed expressions lower");
        let binary = mir
            .expressions
            .iter()
            .find(|expression| {
                matches!(
                    expression.kind,
                    MirExpressionKind::Binary { .. } | MirExpressionKind::BinaryChain { .. }
                )
            })
            .expect("addition is a MIR operation");
        let (left, right) = match &binary.kind {
            MirExpressionKind::Binary { left, right, .. } => (*left, *right),
            MirExpressionKind::BinaryChain {
                first, operations, ..
            } => (*first, operations[0].1),
            _ => unreachable!(),
        };
        assert!(left.0 < mir.expressions.len());
        assert!(right.0 < mir.expressions.len());
        assert!(mir.expressions.iter().any(|expression| matches!(
            expression.kind,
            MirExpressionKind::Call {
                function: Some(_),
                ..
            }
        )));
        verify(&mir).expect("explicit expression references verify");
    }

    #[test]
    fn lowers_match_statements_with_patterns_guards_and_fallback_error() {
        let source = r#"
enum Maybe<T> { Some(T) None }
fn main() {
    value: Maybe<Int> = Maybe::Some(2)
    match value {
        Maybe::Some(number) if number > 0 => { print(number) }
        Maybe::Some(_) => {}
        Maybe::None => {}
    }
}
"#;
        let mir = lower(&parse_program(source)).expect("match statement lowers");
        let main = mir
            .functions
            .iter()
            .find(|function| function.name == "main")
            .unwrap();
        assert!(main
            .blocks
            .iter()
            .any(|block| matches!(block.terminator, Some(MirTerminator::MatchTest { .. }))));
        assert!(main
            .blocks
            .iter()
            .any(|block| matches!(block.terminator, Some(MirTerminator::MatchFailure { .. }))));
        assert!(main
            .blocks
            .iter()
            .any(|block| matches!(block.terminator, Some(MirTerminator::Branch { .. }))));
        assert_eq!(
            main.blocks
                .iter()
                .flat_map(|block| &block.instructions)
                .filter(|instruction| matches!(instruction, MirInstruction::MatchInit { .. }))
                .count(),
            1,
            "the scrutinee must be evaluated once"
        );
        verify(&mir).expect("match CFG verifies");

        let mut missing_initialization = mir.clone();
        let main = missing_initialization
            .functions
            .iter_mut()
            .find(|function| function.name == "main")
            .unwrap();
        for block in &mut main.blocks {
            block
                .instructions
                .retain(|instruction| !matches!(instruction, MirInstruction::MatchInit { .. }));
        }
        let errors = verify(&missing_initialization)
            .expect_err("a pattern test cannot use an uninitialized match value");
        assert!(errors.iter().any(|error| {
            error.code == "E-MIR-VERIFY-001"
                && error
                    .message
                    .contains("match value that was not initialized")
        }));

        let mut late_initialization = mir.clone();
        let main = late_initialization
            .functions
            .iter_mut()
            .find(|function| function.name == "main")
            .unwrap();
        let match_initializer = main.blocks[0]
            .instructions
            .iter()
            .position(|instruction| matches!(instruction, MirInstruction::MatchInit { .. }))
            .map(|position| main.blocks[0].instructions.remove(position))
            .unwrap();
        let matched_block = main
            .blocks
            .iter()
            .find_map(|block| match &block.terminator {
                Some(MirTerminator::MatchTest { matched_block, .. }) => Some(*matched_block),
                _ => None,
            })
            .unwrap();
        main.blocks[matched_block.0]
            .instructions
            .push(match_initializer);
        let errors = verify(&late_initialization)
            .expect_err("match initialization must dominate every pattern test");
        assert!(errors.iter().any(|error| {
            error.code == "E-MIR-VERIFY-001"
                && error
                    .message
                    .contains("initialization does not dominate its test")
        }));
    }

    #[test]
    fn refuses_statements_without_a_defined_mir_lowering() {
        let source = "fn main() { transaction { print(1) } }";
        let errors = lower(&parse_program(source)).expect_err("unsupported semantics fail closed");
        assert!(errors
            .iter()
            .any(|error| error.code == "E-MIR-UNSUPPORTED-001"));
    }

    #[test]
    fn verifier_rejects_invalid_transaction_expression_references() {
        let source = r#"
database main { engine: mariadb database: "test" }
table accounts { id: Id primary auto balance: Int required }
fn update_balance(account_id: Int) uses Database(write) {
    transaction { sql { UPDATE accounts SET balance = 0 WHERE id = :account_id } }
}
fn main() {}
"#;
        let mut mir = lower(&parse_program(source)).expect("SQL transaction lowers");
        let transaction = mir
            .functions
            .iter_mut()
            .flat_map(|function| &mut function.blocks)
            .flat_map(|block| &mut block.instructions)
            .find_map(|instruction| match instruction {
                MirInstruction::Transaction { operations, .. } => Some(operations),
                _ => None,
            })
            .expect("transaction instruction exists");
        transaction[0] = ExpressionId(usize::MAX);

        let errors = verify(&mir).expect_err("invalid transaction references are rejected");
        assert!(errors.iter().any(|error| error.code == "E-MIR-VERIFY-001"));

        let mut invalid_parameter = lower(&parse_program(source)).expect("SQL transaction lowers");
        let query = invalid_parameter
            .functions
            .iter()
            .flat_map(|function| &function.blocks)
            .flat_map(|block| &block.instructions)
            .find_map(|instruction| match instruction {
                MirInstruction::Transaction { operations, .. } => operations.first().copied(),
                _ => None,
            })
            .expect("transaction query exists");
        let MirExpressionKind::Sql { parameters, .. } =
            &mut invalid_parameter.expressions[query.0].kind
        else {
            panic!("transaction query is SQL");
        };
        parameters[0].1 = LocalId(usize::MAX);
        let errors =
            verify(&invalid_parameter).expect_err("unresolved transaction parameters are rejected");
        assert!(errors.iter().any(|error| {
            error.code == "E-MIR-VERIFY-002" && error.message.contains("unknown local")
        }));
    }

    #[test]
    fn lowers_collection_range_and_tuple_for_loops_to_iterator_cfg() {
        let source = r#"
fn main() {
    for number in range(1, 4) { print(number) }
    for (left, right) in zip([1, 2], [3, 4]) { print(left + right) }
}
"#;
        let mir = lower(&parse_program(source)).expect("for loops lower");
        let initializers = mir
            .functions
            .iter()
            .flat_map(|function| &function.blocks)
            .flat_map(|block| &block.instructions)
            .filter_map(|instruction| match instruction {
                MirInstruction::IteratorInit {
                    iterator, source, ..
                } => Some((*iterator, source)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(initializers.len(), 2);
        assert!(initializers
            .iter()
            .any(|(_, source)| matches!(source, MirIteratorSource::IntegerRange { .. })));
        assert!(initializers
            .iter()
            .any(|(_, source)| matches!(source, MirIteratorSource::Collection(_))));
        let next_blocks = mir
            .functions
            .iter()
            .flat_map(|function| &function.blocks)
            .filter_map(|block| match &block.terminator {
                Some(MirTerminator::IteratorNext {
                    iterator, bindings, ..
                }) => Some((*iterator, bindings.len())),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(next_blocks.len(), 2);
        assert!(next_blocks
            .iter()
            .any(|(_, binding_count)| *binding_count == 2));
        verify(&mir).expect("iterator CFG verifies");

        let mut late_initialization = mir.clone();
        let main = late_initialization
            .functions
            .iter_mut()
            .find(|function| function.name == "main")
            .unwrap();
        let (iterator, body_block) = main
            .blocks
            .iter()
            .find_map(|block| match &block.terminator {
                Some(MirTerminator::IteratorNext {
                    iterator,
                    body_block,
                    ..
                }) => Some((*iterator, *body_block)),
                _ => None,
            })
            .unwrap();
        let (init_block, init_instruction) = main
            .blocks
            .iter()
            .enumerate()
            .find_map(|(block_index, block)| {
                block
                    .instructions
                    .iter()
                    .position(|instruction| {
                        matches!(instruction, MirInstruction::IteratorInit { iterator: id, .. } if *id == iterator)
                    })
                    .map(|instruction_index| (block_index, instruction_index))
            })
            .unwrap();
        let initializer = main.blocks[init_block]
            .instructions
            .remove(init_instruction);
        main.blocks[body_block.0].instructions.push(initializer);
        let errors = verify(&late_initialization)
            .expect_err("iterator initialization must dominate its next operation");
        assert!(errors.iter().any(|error| {
            error.code == "E-MIR-VERIFY-001"
                && error
                    .message
                    .contains("iterator 0 initialization does not dominate its use")
        }));
    }

    #[test]
    fn lowers_index_updates_and_tuple_assignment_as_explicit_instructions() {
        let source = r#"
fn coordinates() -> Tuple<Int, String> { return (4, "A-12") }
fn main() {
    mutable totals: Int[] = [1, 2]
    totals[0] += 3
    (latitude, city) = coordinates()
    mutable current_latitude: Int = 0
    mutable current_city: String = ""
    (current_latitude, current_city) = coordinates()
}
"#;
        let mir = lower(&parse_program(source)).expect("typed mutations lower");
        let main = mir
            .functions
            .iter()
            .find(|function| function.name == "main")
            .expect("main function");
        let instructions = main
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .collect::<Vec<_>>();
        let indexed = instructions
            .iter()
            .find_map(|instruction| match instruction {
                MirInstruction::ArrayIndexAssign {
                    name,
                    index,
                    value,
                    operation,
                    ..
                } => Some((name, *index, *value, *operation)),
                _ => None,
            })
            .expect("index update instruction");
        assert_eq!(indexed.0, "totals");
        assert_eq!(indexed.3, ArrayIndexAssignOp::Add);
        assert!(indexed.1 .0 < mir.expressions.len());
        assert!(indexed.2 .0 < mir.expressions.len());

        let tuple_bindings = instructions
            .iter()
            .filter_map(|instruction| match instruction {
                MirInstruction::TupleBind {
                    bindings, value, ..
                } => Some((bindings, *value)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(tuple_bindings.len(), 2);
        assert_eq!(tuple_bindings[0].0[0].name, "latitude");
        assert!(tuple_bindings[0].0.iter().all(|binding| binding.is_new));
        assert_eq!(tuple_bindings[1].0[0].name, "current_latitude");
        assert!(tuple_bindings[1].0.iter().all(|binding| !binding.is_new));
        assert!(tuple_bindings
            .iter()
            .all(|(_, value)| value.0 < mir.expressions.len()));
        verify(&mir).expect("mutation instructions verify");
    }

    #[test]
    fn type_errors_prevent_mir_emission() {
        let source = "fn main() { value: Int = \"wrong\" }";
        let errors = lower(&parse_program(source)).expect_err("invalid source cannot produce MIR");
        assert!(errors.iter().any(|error| error.code == "E-TYPE-001"));
    }

    #[test]
    fn verifier_rejects_unterminated_blocks() {
        let source = "fn main() { print(1) }";
        let mut mir = lower(&parse_program(source)).expect("program lowers");
        mir.functions[0].blocks[0].terminator = None;
        let errors = verify(&mir).expect_err("malformed graph must fail verification");
        assert!(errors.iter().any(|error| error.code == "E-MIR-VERIFY-001"));
    }

    #[test]
    fn verifier_rejects_duplicate_and_non_await_parallel_bindings() {
        let source = "fn value() -> Int { return 1 } fn main() { parallel { left = await value() right = await value() } }";
        let mut duplicate = lower(&parse_program(source)).expect("parallel block lowers");
        let bindings = duplicate
            .functions
            .iter_mut()
            .flat_map(|function| &mut function.blocks)
            .flat_map(|block| &mut block.instructions)
            .find_map(|instruction| match instruction {
                MirInstruction::Parallel { bindings, .. } => Some(bindings),
                _ => None,
            })
            .expect("parallel instruction exists");
        bindings[1].local = bindings[0].local;
        let errors = verify(&duplicate).expect_err("duplicate parallel targets are rejected");
        assert!(errors.iter().any(|error| {
            error.code == "E-MIR-VERIFY-002" && error.message.contains("more than once")
        }));

        let mut non_await = lower(&parse_program(source)).expect("parallel block lowers");
        let int_literal = non_await
            .expressions
            .iter()
            .find(|expression| matches!(expression.kind, MirExpressionKind::Int(1)))
            .expect("integer literal exists")
            .id;
        let bindings = non_await
            .functions
            .iter_mut()
            .flat_map(|function| &mut function.blocks)
            .flat_map(|block| &mut block.instructions)
            .find_map(|instruction| match instruction {
                MirInstruction::Parallel { bindings, .. } => Some(bindings),
                _ => None,
            })
            .expect("parallel instruction exists");
        bindings[0].value = int_literal;
        let errors = verify(&non_await).expect_err("non-await parallel values are rejected");
        assert!(errors.iter().any(|error| {
            error.code == "E-MIR-VERIFY-002" && error.message.contains("must contain an await")
        }));
    }

    #[test]
    fn verifier_rejects_mutation_of_immutable_locals() {
        fn local(program: &MirProgram, name: &str) -> LocalId {
            program
                .functions
                .iter()
                .flat_map(|function| &function.blocks)
                .flat_map(|block| &block.instructions)
                .find_map(|instruction| match instruction {
                    MirInstruction::Let {
                        local,
                        name: candidate,
                        ..
                    } if candidate == name => Some(*local),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("local `{name}` exists"))
        }

        let mut duplicate_mutability = lower(&parse_program(
            "fn main() { immutable_value = 1 mutable mutable_value = 2 mutable_value = 3 }",
        ))
        .expect("source with distinct local declarations lowers");
        let immutable = local(&duplicate_mutability, "immutable_value");
        let mutable = local(&duplicate_mutability, "mutable_value");
        assert_ne!(immutable, mutable);
        let declaration = duplicate_mutability
            .functions
            .iter_mut()
            .flat_map(|function| &mut function.blocks)
            .flat_map(|block| &mut block.instructions)
            .find_map(|instruction| match instruction {
                MirInstruction::Let {
                    local,
                    name,
                    mutable: false,
                    ..
                } if name == "immutable_value" => {
                    *local = mutable;
                    Some(())
                }
                _ => None,
            });
        assert!(declaration.is_some());
        assert!(verify(&duplicate_mutability)
            .expect_err("duplicate local IDs cannot merge immutable and mutable declarations")
            .iter()
            .any(|error| error.message.contains("declared more than once")));

        let mut assignment_program = lower(&parse_program(
            "fn main() { fixed = 1 mutable changeable = 2 changeable = 3 }",
        ))
        .expect("source with a mutable assignment lowers");
        let fixed = local(&assignment_program, "fixed");
        let assignment = assignment_program
            .functions
            .iter_mut()
            .flat_map(|function| &mut function.blocks)
            .flat_map(|block| &mut block.instructions)
            .find_map(|instruction| match instruction {
                MirInstruction::Assign { local, name, .. } if name == "changeable" => {
                    *local = fixed;
                    *name = "fixed".into();
                    Some(())
                }
                _ => None,
            });
        assert!(assignment.is_some());
        assert!(verify(&assignment_program)
            .expect_err("assignment cannot mutate an immutable local")
            .iter()
            .any(|error| error
                .message
                .contains("cannot mutate immutable local `fixed`")));

        let mut index_update = lower(&parse_program(
            "fn main() { fixed: Int[] = [1] mutable changeable: Int[] = [2] changeable[0] = 3 }",
        ))
        .expect("source with a mutable array update lowers");
        let fixed = local(&index_update, "fixed");
        let indexed = index_update
            .functions
            .iter_mut()
            .flat_map(|function| &mut function.blocks)
            .flat_map(|block| &mut block.instructions)
            .find_map(|instruction| match instruction {
                MirInstruction::ArrayIndexAssign { local, name, .. } if name == "changeable" => {
                    *local = fixed;
                    *name = "fixed".into();
                    Some(())
                }
                _ => None,
            });
        assert!(indexed.is_some());
        assert!(verify(&index_update)
            .expect_err("immutable arrays cannot be mutated by index")
            .iter()
            .any(|error| error
                .message
                .contains("cannot mutate immutable local `fixed`")));

        let mut tuple = lower(&parse_program(
            "fn pair() -> Tuple<Int, Int> { return (4, 5) } fn main() { fixed = 0 mutable current = 1 (current, second) = pair() }",
        ))
        .expect("source with a mutable tuple assignment lowers");
        let fixed = local(&tuple, "fixed");
        let tuple_binding = tuple
            .functions
            .iter_mut()
            .flat_map(|function| &mut function.blocks)
            .flat_map(|block| &mut block.instructions)
            .find_map(|instruction| match instruction {
                MirInstruction::TupleBind { bindings, .. }
                    if bindings.iter().any(|binding| binding.name == "current") =>
                {
                    let binding = bindings
                        .iter_mut()
                        .find(|binding| binding.name == "current")
                        .unwrap();
                    binding.local = fixed;
                    binding.name = "fixed".into();
                    Some(())
                }
                _ => None,
            });
        assert!(tuple_binding.is_some());
        assert!(verify(&tuple)
            .expect_err("tuple assignment cannot mutate an immutable local")
            .iter()
            .any(|error| error
                .message
                .contains("cannot mutate immutable local `fixed`")));

        let mut parallel = lower(&parse_program(
            "fn fetch() -> Int { return 2 } fn main() { fixed = 0 mutable current = 1 parallel { current = await fetch() } }",
        ))
        .expect("source with a mutable parallel assignment lowers");
        let fixed = local(&parallel, "fixed");
        let parallel_binding = parallel
            .functions
            .iter_mut()
            .flat_map(|function| &mut function.blocks)
            .flat_map(|block| &mut block.instructions)
            .find_map(|instruction| match instruction {
                MirInstruction::Parallel { bindings, .. } => bindings.first_mut().map(|binding| {
                    binding.local = fixed;
                    binding.name = "fixed".into();
                }),
                _ => None,
            });
        assert!(parallel_binding.is_some());
        assert!(verify(&parallel)
            .expect_err("parallel assignment cannot mutate an immutable local")
            .iter()
            .any(|error| error
                .message
                .contains("cannot mutate immutable local `fixed`")));
    }

    #[test]
    fn verifier_rejects_parallel_blocks_over_the_shared_branch_limit() {
        let source =
            "fn value() -> Int { return 1 } fn main() { parallel { only = await value() } }";
        let mut mir = lower(&parse_program(source)).expect("valid one-branch MIR");
        let bindings = mir
            .functions
            .iter_mut()
            .flat_map(|function| &mut function.blocks)
            .flat_map(|block| &mut block.instructions)
            .find_map(|instruction| match instruction {
                MirInstruction::Parallel { bindings, .. } => Some(bindings),
                _ => None,
            })
            .expect("parallel instruction exists");
        let binding = bindings[0].clone();
        while bindings.len() <= zelyra_runtime::MAX_PARALLEL_BRANCHES {
            bindings.push(binding.clone());
        }
        let errors = verify(&mir).expect_err("verified MIR must cap parallel branches");
        assert!(errors.iter().any(|error| {
            error.code == "E-MIR-VERIFY-002" && error.message.contains("maximum of 4 branches")
        }));
    }

    #[test]
    fn verifier_rejects_out_of_range_expression_operands() {
        let source = "fn main() { value = 1 + 2 }";
        let mut mir = lower(&parse_program(source)).expect("program lowers");
        let expression = mir
            .expressions
            .iter_mut()
            .find(|expression| {
                matches!(
                    expression.kind,
                    MirExpressionKind::Binary { .. } | MirExpressionKind::BinaryChain { .. }
                )
            })
            .unwrap();
        match &mut expression.kind {
            MirExpressionKind::Binary { right, .. } => *right = ExpressionId(usize::MAX),
            MirExpressionKind::BinaryChain { operations, .. } => {
                operations[0].1 = ExpressionId(usize::MAX)
            }
            _ => unreachable!(),
        }
        let errors = verify(&mir).expect_err("out-of-range child is invalid");
        assert!(errors.iter().any(|error| {
            error.code == "E-MIR-VERIFY-001"
                && error
                    .message
                    .contains("child reference outside the expression table")
        }));
    }
}
