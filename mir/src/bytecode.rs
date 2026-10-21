//! Versioned bytecode slice compiled from verified typed MIR.
//!
//! This first VM backend intentionally accepts only a checked subset:
//! scalar expressions, aggregate construction/indexing, local assignments,
//! mutable array index updates, direct function calls, supported MIR control
//! flow, range/collection iterators, and explicitly granted host calls. It has
//! a bounded versioned binary codec, but no persistent
//! cache. Other unsupported operations fail closed so the AST interpreter
//! remains the reference backend.

use crate::{
    execution::{
        array_assignment_index, bounded_execution_limits, evaluate_binary, evaluate_unary,
        execute_pure_builtin, index_value, is_pure_builtin, runtime_error, runtime_type_matches,
        ExecutionLimits, ExecutionOutcome, ExecutionResult,
    },
    verify, BlockId, ExpressionId, IteratorId, LoopInvariantPhase, MatchId, MirError,
    MirExpressionKind, MirInstruction, MirIteratorSource, MirPattern, MirProgram, MirTerminator,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use zelyra_ast::{ArrayIndexAssignOp, BinaryOp, EnumDef, RecordDef, Span, Type, TypeDef, UnaryOp};
use zelyra_hir::{FunctionId, LocalId};
use zelyra_runtime::{
    decode_json_value, deliver_outbox_message_with_host, encode_json_value_with_definitions,
    execute_http_request_with_host, http_response_value_from_host,
    inspect_filesystem_copy_or_rename_destination_with_host,
    inspect_filesystem_copy_source_with_host, inspect_filesystem_delete_path_with_host,
    inspect_filesystem_directory_with_host, inspect_filesystem_path_with_host,
    inspect_filesystem_rename_source_with_host, normalize_map_entries,
    random_int_with_host_services, render_pdf_view, run_process_with_host_services,
    uuid_v4_with_host_services, FileSystemPolicy, FunctionValue, HostServices, MailPolicy,
    NetworkPolicy, PackageSettings, ProcessPolicy, RuntimeError, RuntimePolicy, StepBudget,
    StepBudgetError, SystemHostServices, Value,
};

mod format;
pub use format::{decode, decode_with_limits, encode, encode_with_limits, BytecodeFormatLimits};
mod compiler;
pub use compiler::compile;

pub const BYTECODE_VERSION: u16 = 27;
const MAX_BYTECODE_OPERATIONS_PER_EXECUTION: usize = 10_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BytecodeIteratorKind {
    Range,
    Collection,
}

#[derive(Clone, Debug)]
pub struct BytecodeProgram {
    pub version: u16,
    pub functions: Vec<BytecodeFunction>,
    pub entry: usize,
    pub type_aliases: Vec<TypeDef>,
    pub records: Vec<RecordDef>,
    pub enums: Vec<EnumDef>,
    pub views: Vec<zelyra_ast::ViewDef>,
    /// Tenant-scope metadata required to reject unscoped SQL at verification
    /// and execution time.
    pub scoped_tables: Vec<String>,
}

/// Database inputs for explicitly enabled bytecode SQL execution. The schema
/// is checked through the same read-only storage validation used by MIR.
#[derive(Clone, Copy)]
pub struct BytecodeDatabaseExecution<'a> {
    pub url: &'a str,
    pub schema: &'a Result<zelyra_database::Schema, Vec<zelyra_database::SchemaError>>,
}

#[derive(Clone, Debug)]
pub struct BytecodeFunction {
    pub id: FunctionId,
    pub name: String,
    pub parameters: Vec<(LocalId, Type, Span)>,
    /// Locals captured when this function is materialized as a closure.
    pub captures: Vec<(LocalId, Type, Span)>,
    /// Local mutability retained by the compiler so the verifier can reject
    /// writes that the source type checker would disallow.
    pub local_mutability: Vec<(LocalId, bool)>,
    /// `(instruction, local)` pairs that initialize a lexical binding. These
    /// are distinct from later writes to an immutable binding.
    pub initialization_writes: Vec<(usize, LocalId)>,
    pub return_type: Type,
    /// Source-declared effects retained so decoded artifacts cannot erase the
    /// language-level capability boundary.
    pub declared_capabilities: Vec<String>,
    pub code: Vec<BytecodeInstruction>,
    pub max_stack: usize,
}

#[derive(Clone, Debug)]
pub struct BytecodeParallelBinding {
    pub target: LocalId,
    pub is_new: bool,
    pub function: usize,
    pub captures: Vec<LocalId>,
    pub result_type: Type,
}

#[derive(Clone, Debug)]
pub struct BytecodeSqlStatement {
    pub query: String,
    pub parameters: Vec<(String, LocalId)>,
    pub result_type: Type,
}

#[derive(Clone, Debug)]
pub enum BytecodeTransactionOperation {
    Sql(BytecodeSqlStatement),
    MailQueue {
        arguments: usize,
        argument_spans: Vec<Span>,
        call_span: Span,
    },
}

#[derive(Clone, Debug)]
pub struct BytecodeInstruction {
    pub operation: BytecodeOp,
    pub span: Span,
    /// A source-level step charged immediately before this operation.
    pub source_step_span: Option<Span>,
}

#[derive(Clone, Debug)]
pub enum BytecodeOp {
    /// Counts one Zelyra source-level execution step, independent of VM lowering.
    Step,
    Constant(Value),
    LoadLocal(LocalId),
    StoreLocal(LocalId),
    Pop,
    Unary(UnaryOp),
    Binary(BinaryOp),
    BuildArray {
        items: usize,
    },
    BuildMap {
        entries: usize,
        key_type: Type,
        value_type: Type,
    },
    BuildTuple {
        items: usize,
    },
    Index,
    ArrayIndexAssign {
        local: LocalId,
        operation: ArrayIndexAssignOp,
    },
    /// Append one checked element to a compiler-generated collection result.
    ArrayAppend {
        local: LocalId,
        element_type: Type,
    },
    ArrayPopMut {
        local: LocalId,
        element_type: Type,
    },
    /// Wrap the top stack value as `Some<T>` for a lowered `find` expression.
    OptionSome {
        value_type: Type,
    },
    OptionNone {
        value_type: Type,
    },
    TupleBind {
        locals: Vec<LocalId>,
    },
    HostCallPreflight {
        name: String,
        arguments: usize,
    },
    Builtin {
        name: String,
        arguments: usize,
        type_arguments: Vec<Type>,
        result_type: Type,
    },
    BuildRecord {
        record_type: Type,
        fields: Vec<(String, Type)>,
    },
    /// Copy a checked record value and replace the named fields.
    RecordUpdate {
        record_type: Type,
        fields: Vec<(String, Type)>,
    },
    Parallel {
        bindings: Vec<BytecodeParallelBinding>,
    },
    BuildEnum {
        enum_type: Type,
        variant: String,
        payload_types: Vec<Type>,
    },
    Sql {
        query: String,
        parameters: Vec<(String, LocalId)>,
        result_type: Type,
    },
    /// Execute checked SQL and supported standalone mail_queue statements as
    /// one provider transaction. Other transaction effects remain unsupported.
    SqlTransaction {
        operations: Vec<BytecodeTransactionOperation>,
    },
    /// Render a checked local view to deterministic PDF bytes.
    RenderPdf,
    GetField {
        record_type: Type,
        field: String,
        result_type: Type,
    },
    Call {
        function: usize,
        arguments: usize,
    },
    FunctionRef {
        function: usize,
    },
    /// Create a closure over the selected locals for a generated lambda body.
    MakeClosure {
        function: usize,
        captures: Vec<LocalId>,
    },
    /// Invoke a typed function value after the arguments have been pushed.
    CallValue {
        arguments: usize,
    },
    ContractCheck {
        postcondition: bool,
    },
    LoopInvariantCheck {
        phase: LoopInvariantPhase,
    },
    Jump(usize),
    JumpIfFalse(usize),
    JumpIfTrue(usize),
    RangeInit {
        iterator: IteratorId,
    },
    RangeNext {
        iterator: IteratorId,
        local: LocalId,
        body: usize,
        exit: usize,
    },
    CollectionInit {
        iterator: IteratorId,
    },
    CollectionNext {
        iterator: IteratorId,
        locals: Vec<LocalId>,
        body: usize,
        exit: usize,
    },
    MatchInit {
        match_id: MatchId,
    },
    MatchTest {
        match_id: MatchId,
        pattern: MirPattern,
        matched: usize,
        unmatched: usize,
    },
    MatchFailure,
    Return,
    Trap,
}

/// Validate bytecode version, jump targets, function references, call arity,
/// and stack-height consistency across control-flow joins.
pub fn verify_bytecode(program: &BytecodeProgram) -> Result<(), Vec<MirError>> {
    if program.version != BYTECODE_VERSION {
        return Err(vec![bytecode_error(
            format!("unsupported bytecode version {}", program.version),
            Span::default(),
        )]);
    }
    if program.entry >= program.functions.len() {
        return Err(vec![bytecode_error(
            "bytecode entry index is outside the function table",
            Span::default(),
        )]);
    }
    let scoped_names = program
        .scoped_tables
        .iter()
        .map(|name| name.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    if scoped_names.len() != program.scoped_tables.len()
        || program
            .scoped_tables
            .iter()
            .any(|name| name.trim().is_empty())
    {
        return Err(vec![bytecode_error(
            "bytecode tenant-scope metadata is empty or duplicated",
            Span::default(),
        )]);
    }
    let mut view_names = HashSet::new();
    if program
        .views
        .iter()
        .any(|view| view.name.is_empty() || !view_names.insert(view.name.as_str()))
    {
        return Err(vec![bytecode_error(
            "bytecode view definitions contain an empty or duplicate name",
            Span::default(),
        )]);
    }
    let mut type_names = HashSet::new();
    for name in program
        .type_aliases
        .iter()
        .map(|definition| definition.name.as_str())
        .chain(
            program
                .records
                .iter()
                .map(|definition| definition.name.as_str()),
        )
        .chain(
            program
                .enums
                .iter()
                .map(|definition| definition.name.as_str()),
        )
    {
        if name.is_empty() || !type_names.insert(name) {
            return Err(vec![bytecode_error(
                "bytecode type definitions contain an empty or duplicate name",
                Span::default(),
            )]);
        }
    }
    for record in &program.records {
        let fields = record
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<HashSet<_>>();
        let type_params = record
            .type_params
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        if fields.len() != record.fields.len() || type_params.len() != record.type_params.len() {
            return Err(vec![bytecode_error(
                "bytecode record definition contains duplicate fields or type parameters",
                record.span,
            )]);
        }
    }
    for definition in &program.enums {
        let variants = definition
            .variants
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let type_params = definition
            .type_params
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let payload_variants = definition
            .variant_payloads
            .iter()
            .map(|(variant, _)| variant.as_str())
            .collect::<HashSet<_>>();
        if variants.len() != definition.variants.len()
            || type_params.len() != definition.type_params.len()
            || payload_variants.len() != definition.variant_payloads.len()
            || payload_variants
                .iter()
                .any(|variant| !variants.contains(variant))
        {
            return Err(vec![bytecode_error(
                "bytecode enum definition contains duplicate or unknown variants/type parameters",
                definition.span,
            )]);
        }
    }
    for function in &program.functions {
        let mut frame_locals = HashSet::new();
        if function
            .parameters
            .iter()
            .map(|(local, _, _)| local)
            .chain(function.captures.iter().map(|(local, _, _)| local))
            .any(|local| !frame_locals.insert(*local))
            || function
                .captures
                .iter()
                .any(|(_, ty, _)| *ty == Type::Unknown)
        {
            return Err(vec![bytecode_error(
                "bytecode function has duplicate frame locals or an untyped capture",
                function
                    .code
                    .first()
                    .map_or(Span::default(), |item| item.span),
            )]);
        }
        let local_mutability = function
            .local_mutability
            .iter()
            .map(|(local, mutable)| (*local, *mutable))
            .collect::<HashMap<_, _>>();
        if local_mutability.len() != function.local_mutability.len()
            || function
                .parameters
                .iter()
                .chain(&function.captures)
                .any(|(local, _, _)| local_mutability.get(local) != Some(&false))
        {
            return Err(vec![bytecode_error(
                "bytecode function has duplicate or inconsistent local mutability metadata",
                function
                    .code
                    .first()
                    .map_or(Span::default(), |item| item.span),
            )]);
        }
        let mut initialization_writes = HashSet::new();
        for (instruction_index, local) in &function.initialization_writes {
            let Some(item) = function.code.get(*instruction_index) else {
                return Err(vec![bytecode_error(
                    "bytecode local initialization points outside the function",
                    function
                        .code
                        .first()
                        .map_or(Span::default(), |item| item.span),
                )]);
            };
            if !initialization_writes.insert((*instruction_index, *local))
                || !local_mutability.contains_key(local)
                || !bytecode_written_locals(&item.operation).contains(local)
            {
                return Err(vec![bytecode_error(
                    "bytecode local initialization metadata is duplicate or inconsistent",
                    item.span,
                )]);
            }
        }
        let capabilities = function
            .declared_capabilities
            .iter()
            .collect::<HashSet<_>>();
        if capabilities.len() != function.declared_capabilities.len()
            || function.declared_capabilities.iter().any(|capability| {
                !matches!(
                    capability.as_str(),
                    "Clock"
                        | "Environment"
                        | "Console"
                        | "Random"
                        | "FileSystem"
                        | "Process"
                        | "Network"
                        | "Email"
                        | "Database"
                        | "Database(read)"
                        | "Database(write)"
                )
            })
        {
            return Err(vec![bytecode_error(
                "bytecode function has duplicate or unsupported capabilities",
                function
                    .code
                    .first()
                    .map_or(Span::default(), |item| item.span),
            )]);
        }
        let initialized_iterators = function
            .code
            .iter()
            .filter_map(|instruction| match instruction.operation {
                BytecodeOp::RangeInit { iterator } => Some((iterator, BytecodeIteratorKind::Range)),
                BytecodeOp::CollectionInit { iterator } => {
                    Some((iterator, BytecodeIteratorKind::Collection))
                }
                _ => None,
            })
            .collect::<HashMap<_, _>>();
        let initializer_count = function
            .code
            .iter()
            .filter(|instruction| {
                matches!(
                    instruction.operation,
                    BytecodeOp::RangeInit { .. } | BytecodeOp::CollectionInit { .. }
                )
            })
            .count();
        if initialized_iterators.len() != initializer_count {
            return Err(vec![bytecode_error(
                "bytecode iterator is initialized more than once",
                function.code[0].span,
            )]);
        }
        let captured_locals = function
            .captures
            .iter()
            .map(|(local, _, _)| *local)
            .collect::<HashSet<_>>();
        for (instruction_index, instruction) in function.code.iter().enumerate() {
            if let Some(local) = bytecode_captured_write(&instruction.operation, &captured_locals) {
                return Err(vec![bytecode_error(
                    format!(
                        "bytecode closure attempts to mutate immutable captured local {}",
                        local.0
                    ),
                    instruction.span,
                )]);
            }
            for local in bytecode_written_locals(&instruction.operation) {
                let Some(mutable) = local_mutability.get(&local) else {
                    return Err(vec![bytecode_error(
                        format!(
                            "bytecode write references local {} without mutability metadata",
                            local.0
                        ),
                        instruction.span,
                    )]);
                };
                if !mutable && !initialization_writes.contains(&(instruction_index, local)) {
                    return Err(vec![bytecode_error(
                        format!("bytecode attempts to mutate immutable local {}", local.0),
                        instruction.span,
                    )]);
                }
            }
            match &instruction.operation {
                BytecodeOp::BuildRecord {
                    record_type,
                    fields,
                } => {
                    if record_type_parts(record_type).is_none() {
                        return Err(vec![bytecode_error(
                            "bytecode record construction has a non-record result type",
                            instruction.span,
                        )]);
                    }
                    let names = fields.iter().map(|(name, _)| name).collect::<HashSet<_>>();
                    if names.len() != fields.len() {
                        return Err(vec![bytecode_error(
                            "bytecode record construction has duplicate field names",
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::RecordUpdate {
                    record_type,
                    fields,
                } => {
                    if record_type_parts(record_type).is_none() {
                        return Err(vec![bytecode_error(
                            "bytecode record update has an invalid record type or no fields",
                            instruction.span,
                        )]);
                    }
                    let names = fields.iter().map(|(name, _)| name).collect::<HashSet<_>>();
                    if names.len() != fields.len()
                        || fields.iter().any(|(_, ty)| matches!(ty, Type::Unknown))
                    {
                        return Err(vec![bytecode_error(
                            "bytecode record update has duplicate fields or an unknown field type",
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::BuildMap {
                    key_type,
                    value_type,
                    ..
                } if matches!(key_type, Type::Unknown) || matches!(value_type, Type::Unknown) => {
                    return Err(vec![bytecode_error(
                        "bytecode map construction has an unknown key or value type",
                        instruction.span,
                    )]);
                }
                BytecodeOp::ArrayAppend {
                    element_type: Type::Unknown,
                    ..
                } => {
                    return Err(vec![bytecode_error(
                        "bytecode array append has an unknown element type",
                        instruction.span,
                    )]);
                }
                BytecodeOp::ArrayPopMut {
                    element_type: Type::Unknown,
                    ..
                } => {
                    return Err(vec![bytecode_error(
                        "bytecode mutable array pop has an unknown element type",
                        instruction.span,
                    )]);
                }
                BytecodeOp::OptionSome {
                    value_type: Type::Unknown,
                } => {
                    return Err(vec![bytecode_error(
                        "bytecode option construction has an unknown payload type",
                        instruction.span,
                    )]);
                }
                BytecodeOp::OptionNone {
                    value_type: Type::Unknown,
                } => {
                    return Err(vec![bytecode_error(
                        "bytecode empty option has an unknown payload type",
                        instruction.span,
                    )]);
                }
                BytecodeOp::GetField {
                    record_type,
                    result_type,
                    ..
                } if record_type_parts(record_type).is_none()
                    || matches!(result_type, Type::Unknown) =>
                {
                    return Err(vec![bytecode_error(
                        "bytecode field access has an invalid static type",
                        instruction.span,
                    )]);
                }
                BytecodeOp::Sql {
                    query,
                    parameters,
                    result_type,
                } => {
                    if let Err(message) =
                        zelyra_runtime::check_scoped_sql(query, &program.scoped_tables)
                    {
                        return Err(vec![bytecode_error(&message, instruction.span)]);
                    }
                    let required = zelyra_runtime::sql_required_capability(query);
                    if matches!(result_type, Type::Unknown)
                        || !function
                            .declared_capabilities
                            .iter()
                            .any(|capability| capability == required)
                    {
                        return Err(vec![bytecode_error(
                            "bytecode SQL instruction has an unknown result type or lacks its declared database capability",
                            instruction.span,
                        )]);
                    }
                    let names = parameters
                        .iter()
                        .map(|(name, _)| name)
                        .collect::<HashSet<_>>();
                    if names.len() != parameters.len() {
                        return Err(vec![bytecode_error(
                            "bytecode SQL instruction has duplicate parameter bindings",
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::SqlTransaction { operations } => {
                    if operations.is_empty() {
                        return Err(vec![bytecode_error(
                            "bytecode transaction has no operations",
                            instruction.span,
                        )]);
                    }
                    for operation in operations {
                        match operation {
                            BytecodeTransactionOperation::Sql(statement) => {
                                if let Err(message) = zelyra_runtime::check_scoped_sql(
                                    &statement.query,
                                    &program.scoped_tables,
                                ) {
                                    return Err(vec![bytecode_error(&message, instruction.span)]);
                                }
                                let required =
                                    zelyra_runtime::sql_required_capability(&statement.query);
                                let names = statement
                                    .parameters
                                    .iter()
                                    .map(|(name, _)| name)
                                    .collect::<HashSet<_>>();
                                if matches!(statement.result_type, Type::Unknown)
                                    || !function
                                        .declared_capabilities
                                        .iter()
                                        .any(|capability| capability == required)
                                    || names.len() != statement.parameters.len()
                                {
                                    return Err(vec![bytecode_error(
                                        "bytecode SQL transaction has an unknown result type, duplicate parameter, or undeclared database capability",
                                        instruction.span,
                                    )]);
                                }
                            }
                            BytecodeTransactionOperation::MailQueue {
                                arguments,
                                argument_spans,
                                ..
                            } => {
                                if !matches!(arguments, 3..=6)
                                    || *arguments != argument_spans.len()
                                    || !function
                                        .declared_capabilities
                                        .iter()
                                        .any(|capability| capability == "Email")
                                    || !function
                                        .declared_capabilities
                                        .iter()
                                        .any(|capability| capability == "Database(write)")
                                {
                                    return Err(vec![bytecode_error(
                                        "bytecode mail_queue has invalid arity, spans, or declared capabilities",
                                        instruction.span,
                                    )]);
                                }
                            }
                        }
                    }
                }
                BytecodeOp::Builtin {
                    name,
                    arguments,
                    type_arguments,
                    result_type,
                    ..
                } if name == "sql_decode" => {
                    if *arguments != 1
                        || type_arguments.len() != 1
                        || result_type != &type_arguments[0]
                    {
                        return Err(vec![bytecode_error(
                            "bytecode `sql_decode` has invalid arity, type arguments, or result type",
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::Builtin {
                    name,
                    arguments,
                    type_arguments,
                    result_type,
                    ..
                } if name == "module_setting" => {
                    if *arguments != 1
                        || type_arguments.len() != 1
                        || result_type != &type_arguments[0]
                        || !matches!(
                            result_type,
                            Type::String | Type::Bool | Type::Int | Type::Float
                        )
                    {
                        return Err(vec![bytecode_error(
                            "bytecode `module_setting` has invalid arity, type arguments, or result type",
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::Builtin {
                    name,
                    arguments,
                    result_type,
                    ..
                } if name == "print" => {
                    if *arguments != 1 || result_type != &Type::Unit {
                        return Err(vec![bytecode_error(
                            "bytecode `print` builtin has invalid arity or result type",
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::Builtin {
                    name,
                    arguments,
                    result_type,
                    ..
                } if name == "now" => {
                    if *arguments != 0
                        || result_type != &Type::Timestamp
                        || !function
                            .declared_capabilities
                            .iter()
                            .any(|capability| capability == "Clock")
                    {
                        return Err(vec![bytecode_error(
                            "bytecode `now` builtin has invalid arity, type, or capability",
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::Builtin {
                    name,
                    arguments,
                    result_type,
                    ..
                } if name == "env" => {
                    if *arguments != 1
                        || result_type != &Type::Option(Box::new(Type::String))
                        || !function
                            .declared_capabilities
                            .iter()
                            .any(|capability| capability == "Environment")
                    {
                        return Err(vec![bytecode_error(
                            "bytecode `env` builtin has invalid arity, type, or capability",
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::Builtin {
                    name,
                    arguments,
                    result_type,
                    ..
                } if name == "read_console" => {
                    if *arguments != 1
                        || result_type != &Type::Option(Box::new(Type::String))
                        || !function
                            .declared_capabilities
                            .iter()
                            .any(|capability| capability == "Console")
                    {
                        return Err(vec![bytecode_error(
                            "bytecode `read_console` builtin has invalid arity, type, or capability",
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::Builtin {
                    name,
                    arguments,
                    result_type,
                    ..
                } if name == "random_int" => {
                    if *arguments != 2
                        || result_type != &Type::Int
                        || !function
                            .declared_capabilities
                            .iter()
                            .any(|capability| capability == "Random")
                    {
                        return Err(vec![bytecode_error(
                            "bytecode `random_int` builtin has invalid arity, type, or capability",
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::Builtin {
                    name,
                    arguments,
                    result_type,
                    ..
                } if name == "uuid_v4" => {
                    if *arguments != 0
                        || result_type != &Type::String
                        || !function
                            .declared_capabilities
                            .iter()
                            .any(|capability| capability == "Random")
                    {
                        return Err(vec![bytecode_error(
                            "bytecode `uuid_v4` builtin has invalid arity, type, or capability",
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::Builtin {
                    name,
                    arguments,
                    result_type,
                    ..
                } if name == "read_text" || name == "read_text_result" => {
                    let expected_type = if name == "read_text" {
                        Type::String
                    } else {
                        Type::Result(Box::new(Type::String), Box::new(Type::String))
                    };
                    if *arguments != 1
                        || result_type != &expected_type
                        || !function
                            .declared_capabilities
                            .iter()
                            .any(|capability| capability == "FileSystem")
                    {
                        return Err(vec![bytecode_error(
                            format!(
                                "bytecode `{name}` builtin has invalid arity, type, or capability"
                            ),
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::Builtin {
                    name,
                    arguments,
                    result_type,
                    ..
                } if name == "file_exists"
                    || name == "dir_exists"
                    || name == "file_size"
                    || name == "list_dir" =>
                {
                    let expected_type = if name == "file_size" {
                        Type::Result(Box::new(Type::Int), Box::new(Type::String))
                    } else if name == "list_dir" {
                        Type::Array(Box::new(Type::String))
                    } else {
                        Type::Bool
                    };
                    if *arguments != 1
                        || result_type != &expected_type
                        || !function
                            .declared_capabilities
                            .iter()
                            .any(|capability| capability == "FileSystem")
                    {
                        return Err(vec![bytecode_error(
                            format!(
                                "bytecode `{name}` builtin has invalid arity, type, or capability"
                            ),
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::Builtin {
                    name,
                    arguments,
                    result_type,
                    ..
                } if name == "write_text"
                    || name == "write_text_result"
                    || name == "write_text_new_result"
                    || name == "atomic_write_text_result"
                    || name == "append_text_result" =>
                {
                    let expected_type = if name == "write_text" {
                        Type::Unit
                    } else {
                        Type::Result(Box::new(Type::Unit), Box::new(Type::String))
                    };
                    if *arguments != 2
                        || result_type != &expected_type
                        || !function
                            .declared_capabilities
                            .iter()
                            .any(|capability| capability == "FileSystem")
                    {
                        return Err(vec![bytecode_error(
                            format!(
                                "bytecode `{name}` builtin has invalid arity, type, or capability"
                            ),
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::Builtin {
                    name,
                    arguments,
                    result_type,
                    ..
                } if name == "delete_file" || name == "delete_file_result" => {
                    let expected_type = if name == "delete_file" {
                        Type::Unit
                    } else {
                        Type::Result(Box::new(Type::Unit), Box::new(Type::String))
                    };
                    if *arguments != 1
                        || result_type != &expected_type
                        || !function
                            .declared_capabilities
                            .iter()
                            .any(|capability| capability == "FileSystem")
                    {
                        return Err(vec![bytecode_error(
                            format!(
                                "bytecode `{name}` builtin has invalid arity, type, or capability"
                            ),
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::Builtin {
                    name,
                    arguments,
                    result_type,
                    ..
                } if name == "copy_file_new_result" || name == "rename_file_result" => {
                    let expected_type = Type::Result(Box::new(Type::Unit), Box::new(Type::String));
                    if *arguments != 2
                        || result_type != &expected_type
                        || !function
                            .declared_capabilities
                            .iter()
                            .any(|capability| capability == "FileSystem")
                    {
                        return Err(vec![bytecode_error(
                            format!(
                                "bytecode `{name}` builtin has invalid arity, type, or capability"
                            ),
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::Builtin {
                    name,
                    arguments,
                    type_arguments,
                    result_type,
                } if name == "mail_deliver" => {
                    if *arguments != 2
                        || !type_arguments.is_empty()
                        || result_type
                            != &Type::Result(Box::new(Type::Unit), Box::new(Type::String))
                        || !function
                            .declared_capabilities
                            .iter()
                            .any(|capability| capability == "Email")
                    {
                        return Err(vec![bytecode_error(
                            "bytecode `mail_deliver` builtin has invalid arity, type, or capability",
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::Builtin {
                    name,
                    arguments,
                    result_type,
                    ..
                } if name == "run_process" => {
                    if *arguments != 2
                        || result_type != &Type::String
                        || !function
                            .declared_capabilities
                            .iter()
                            .any(|capability| capability == "Process")
                    {
                        return Err(vec![bytecode_error(
                            "bytecode `run_process` builtin has invalid arity, type, or capability",
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::Builtin {
                    name,
                    arguments,
                    result_type,
                    ..
                } if name == "http_get" => {
                    if *arguments != 1
                        || result_type != &Type::String
                        || !function
                            .declared_capabilities
                            .iter()
                            .any(|capability| capability == "Network")
                    {
                        return Err(vec![bytecode_error(
                            "bytecode `http_get` builtin has invalid arity, type, or capability",
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::Builtin {
                    name,
                    arguments,
                    type_arguments,
                    result_type,
                } if name == "http_request" => {
                    if *arguments != 4
                        || !type_arguments.is_empty()
                        || result_type != &Type::Named("HttpResponse".into())
                        || !function
                            .declared_capabilities
                            .iter()
                            .any(|capability| capability == "Network")
                    {
                        return Err(vec![bytecode_error(
                            "bytecode `http_request` builtin has invalid arity, type, or capability",
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::Builtin {
                    name,
                    arguments,
                    type_arguments,
                    result_type,
                } if name == "http_json" || name == "http_result" => {
                    let expected_type = type_arguments.get(1).map(|response_type| {
                        if name == "http_json" {
                            response_type.clone()
                        } else {
                            Type::HttpResult(Box::new(response_type.clone()))
                        }
                    });
                    if *arguments != 4
                        || type_arguments.len() != 2
                        || expected_type.as_ref() != Some(result_type)
                        || !function
                            .declared_capabilities
                            .iter()
                            .any(|capability| capability == "Network")
                    {
                        return Err(vec![bytecode_error(
                            format!("bytecode `{name}` builtin has invalid arity, type arguments, result type, or capability"),
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::HostCallPreflight { name, arguments } => {
                    let Some((capability, _, expected_arguments)) =
                        bytecode_host_call_security(name)
                    else {
                        return Err(vec![bytecode_error(
                            format!("bytecode host-call preflight `{name}` is unsupported"),
                            instruction.span,
                        )]);
                    };
                    if *arguments != expected_arguments
                        || !function
                            .declared_capabilities
                            .iter()
                            .any(|declared| declared == capability)
                    {
                        return Err(vec![bytecode_error(
                            "bytecode host-call preflight has invalid arity or capability",
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::Builtin { name, .. }
                    if name != "print" && name != "sql_decode" && !is_pure_builtin(name) =>
                {
                    return Err(vec![bytecode_error(
                        format!("bytecode builtin `{name}` is not supported"),
                        instruction.span,
                    )]);
                }
                _ => {}
            }
            if let BytecodeOp::CollectionNext { locals, .. } = &instruction.operation {
                let unique = locals.iter().copied().collect::<HashSet<_>>();
                if locals.is_empty() || unique.len() != locals.len() {
                    return Err(vec![bytecode_error(
                        "bytecode collection iterator has an empty or duplicate binding list",
                        instruction.span,
                    )]);
                }
            }
            if let BytecodeOp::TupleBind { locals } = &instruction.operation {
                let unique = locals.iter().copied().collect::<HashSet<_>>();
                if locals.is_empty() || unique.len() != locals.len() {
                    return Err(vec![bytecode_error(
                        "bytecode tuple binding has an empty or duplicate destination list",
                        instruction.span,
                    )]);
                }
            }
            let (iterator, expected_kind) = match instruction.operation {
                BytecodeOp::RangeNext { iterator, .. } => {
                    (Some(iterator), Some(BytecodeIteratorKind::Range))
                }
                BytecodeOp::CollectionNext { iterator, .. } => {
                    (Some(iterator), Some(BytecodeIteratorKind::Collection))
                }
                _ => (None, None),
            };
            if let (Some(iterator), Some(expected_kind)) = (iterator, expected_kind) {
                if initialized_iterators.get(&iterator) != Some(&expected_kind) {
                    return Err(vec![bytecode_error(
                        "bytecode advances an uninitialized or incompatible iterator",
                        instruction.span,
                    )]);
                }
            }
        }
        let required_stack = verify_function(
            function,
            function.code.first().map_or(Span::default(), |i| i.span),
        )?;
        if function.max_stack != required_stack {
            return Err(vec![bytecode_error(
                "bytecode function declares an incorrect maximum stack height",
                function.code[0].span,
            )]);
        }
        for instruction in &function.code {
            if instruction
                .source_step_span
                .is_some_and(|span| span.start > span.end)
            {
                return Err(vec![bytecode_error(
                    "bytecode source-step span is invalid",
                    instruction.span,
                )]);
            }
            match &instruction.operation {
                BytecodeOp::Call {
                    function: target,
                    arguments,
                } => {
                    let Some(target) = program.functions.get(*target) else {
                        return Err(vec![bytecode_error(
                            "bytecode call target is outside the function table",
                            instruction.span,
                        )]);
                    };
                    if *arguments != target.parameters.len() {
                        return Err(vec![bytecode_error(
                            "bytecode call argument count does not match the target signature",
                            instruction.span,
                        )]);
                    }
                    if !target.captures.is_empty() {
                        return Err(vec![bytecode_error(
                            "capturing bytecode function cannot be called without a closure",
                            instruction.span,
                        )]);
                    }
                    if target
                        .declared_capabilities
                        .iter()
                        .any(|capability| !function.declared_capabilities.contains(capability))
                    {
                        return Err(vec![bytecode_error(
                            "bytecode call would escalate declared capabilities",
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::FunctionRef { function: target } => {
                    if *target >= program.functions.len() {
                        return Err(vec![bytecode_error(
                            "bytecode function reference is outside the function table",
                            instruction.span,
                        )]);
                    }
                    if !program.functions[*target].captures.is_empty() {
                        return Err(vec![bytecode_error(
                            "capturing bytecode function cannot be referenced without a closure",
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::MakeClosure {
                    function: target,
                    captures,
                } => {
                    let Some(target_function) = program.functions.get(*target) else {
                        return Err(vec![bytecode_error(
                            "bytecode closure target is outside the function table",
                            instruction.span,
                        )]);
                    };
                    let expected = target_function
                        .captures
                        .iter()
                        .map(|(local, _, _)| *local)
                        .collect::<Vec<_>>();
                    if *captures != expected
                        || captures.iter().copied().collect::<HashSet<_>>().len() != captures.len()
                    {
                        return Err(vec![bytecode_error(
                            "bytecode closure capture layout does not match its target",
                            instruction.span,
                        )]);
                    }
                    if target_function
                        .declared_capabilities
                        .iter()
                        .any(|capability| !function.declared_capabilities.contains(capability))
                    {
                        return Err(vec![bytecode_error(
                            "bytecode closure creation would exceed declared capabilities",
                            instruction.span,
                        )]);
                    }
                }
                BytecodeOp::Parallel { bindings } => {
                    if bindings.len() > zelyra_runtime::MAX_PARALLEL_BRANCHES {
                        return Err(vec![bytecode_error(
                            format!(
                                "bytecode parallel block exceeds the maximum of {} branches",
                                zelyra_runtime::MAX_PARALLEL_BRANCHES
                            ),
                            instruction.span,
                        )]);
                    }
                    let mut targets = HashSet::new();
                    for binding in bindings {
                        if !targets.insert(binding.target)
                            || binding
                                .captures
                                .iter()
                                .copied()
                                .collect::<HashSet<_>>()
                                .len()
                                != binding.captures.len()
                        {
                            return Err(vec![bytecode_error(
                                "bytecode parallel instruction has duplicate targets or captures",
                                instruction.span,
                            )]);
                        }
                        let Some(branch) = program.functions.get(binding.function) else {
                            return Err(vec![bytecode_error(
                                "bytecode parallel branch is outside the function table",
                                instruction.span,
                            )]);
                        };
                        if branch
                            .parameters
                            .iter()
                            .map(|(local, _, _)| *local)
                            .ne(binding.captures.iter().copied())
                            || branch.return_type != binding.result_type
                        {
                            return Err(vec![bytecode_error(
                                "bytecode parallel branch signature does not match its capture and result metadata",
                                instruction.span,
                            )]);
                        }
                        if branch
                            .declared_capabilities
                            .iter()
                            .any(|capability| !function.declared_capabilities.contains(capability))
                        {
                            return Err(vec![bytecode_error(
                                "bytecode parallel branch would escalate declared capabilities",
                                instruction.span,
                            )]);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn bytecode_pattern_binds_local(
    pattern: &MirPattern,
    locals: &HashSet<LocalId>,
) -> Option<LocalId> {
    match &pattern.kind {
        crate::MirPatternKind::Binding { local, .. } if locals.contains(local) => Some(*local),
        crate::MirPatternKind::Constructor {
            inner: Some(inner), ..
        } => bytecode_pattern_binds_local(inner, locals),
        crate::MirPatternKind::EnumVariantPayload { payloads, .. } => payloads
            .iter()
            .find_map(|pattern| bytecode_pattern_binds_local(pattern, locals)),
        crate::MirPatternKind::Record { fields, .. } => fields
            .iter()
            .find_map(|(_, pattern)| bytecode_pattern_binds_local(pattern, locals)),
        _ => None,
    }
}

fn bytecode_pattern_bound_locals(pattern: &MirPattern, output: &mut Vec<LocalId>) {
    match &pattern.kind {
        crate::MirPatternKind::Binding { local, .. } => output.push(*local),
        crate::MirPatternKind::Constructor {
            inner: Some(inner), ..
        } => bytecode_pattern_bound_locals(inner, output),
        crate::MirPatternKind::EnumVariantPayload { payloads, .. } => {
            for pattern in payloads {
                bytecode_pattern_bound_locals(pattern, output);
            }
        }
        crate::MirPatternKind::Record { fields, .. } => {
            for (_, pattern) in fields {
                bytecode_pattern_bound_locals(pattern, output);
            }
        }
        _ => {}
    }
}

fn bytecode_written_locals(operation: &BytecodeOp) -> Vec<LocalId> {
    match operation {
        BytecodeOp::StoreLocal(local)
        | BytecodeOp::ArrayIndexAssign { local, .. }
        | BytecodeOp::ArrayAppend { local, .. }
        | BytecodeOp::ArrayPopMut { local, .. }
        | BytecodeOp::RangeNext { local, .. } => vec![*local],
        BytecodeOp::TupleBind { locals } | BytecodeOp::CollectionNext { locals, .. } => {
            locals.clone()
        }
        BytecodeOp::Parallel { bindings } => {
            bindings.iter().map(|binding| binding.target).collect()
        }
        BytecodeOp::MatchTest { pattern, .. } => {
            let mut locals = Vec::new();
            bytecode_pattern_bound_locals(pattern, &mut locals);
            locals
        }
        _ => Vec::new(),
    }
}

fn bytecode_captured_write(operation: &BytecodeOp, captured: &HashSet<LocalId>) -> Option<LocalId> {
    match operation {
        BytecodeOp::StoreLocal(local)
        | BytecodeOp::ArrayIndexAssign { local, .. }
        | BytecodeOp::ArrayAppend { local, .. }
        | BytecodeOp::ArrayPopMut { local, .. }
        | BytecodeOp::RangeNext { local, .. } => captured.contains(local).then_some(*local),
        BytecodeOp::TupleBind { locals } | BytecodeOp::CollectionNext { locals, .. } => locals
            .iter()
            .find(|local| captured.contains(local))
            .copied(),
        BytecodeOp::Parallel { bindings } => bindings
            .iter()
            .find_map(|binding| captured.contains(&binding.target).then_some(binding.target)),
        BytecodeOp::MatchTest { pattern, .. } => bytecode_pattern_binds_local(pattern, captured),
        _ => None,
    }
}

fn verify_function(function: &BytecodeFunction, span: Span) -> Result<usize, Vec<MirError>> {
    if function.code.is_empty() {
        return Err(vec![bytecode_error(
            "bytecode function has no instructions",
            span,
        )]);
    }
    let mut heights = HashMap::from([(0usize, (0usize, Vec::<(String, usize)>::new()))]);
    let mut pending = VecDeque::from([0usize]);
    let mut maximum = 0;
    let mut saw_return = false;
    while let Some(pc) = pending.pop_front() {
        let (height, mut pending_host_calls) = heights[&pc].clone();
        let instruction = &function.code[pc];
        let mut next_height = height;
        let successors = match instruction.operation {
            BytecodeOp::Constant(_) | BytecodeOp::LoadLocal(_) => {
                next_height += 1;
                vec![pc + 1]
            }
            BytecodeOp::Sql { .. } => {
                next_height += 1;
                vec![pc + 1]
            }
            BytecodeOp::SqlTransaction { ref operations } => {
                let arguments = operations
                    .iter()
                    .try_fold(0usize, |total, operation| match operation {
                        BytecodeTransactionOperation::Sql(_) => Some(total),
                        BytecodeTransactionOperation::MailQueue { arguments, .. } => {
                            total.checked_add(*arguments)
                        }
                    })
                    .ok_or_else(|| {
                        vec![bytecode_error(
                            "bytecode transaction argument count overflows",
                            instruction.span,
                        )]
                    })?;
                require_stack(height, arguments, instruction.span)?;
                next_height = height - arguments;
                for operation in operations.iter().rev() {
                    if matches!(operation, BytecodeTransactionOperation::MailQueue { .. }) {
                        for expected in ["mail_queue_db_write", "mail_queue"] {
                            let Some((name, pending_arguments)) = pending_host_calls.pop() else {
                                return Err(vec![bytecode_error(
                                    "bytecode transactional mail_queue is missing a capability preflight",
                                    instruction.span,
                                )]);
                            };
                            if name != expected || pending_arguments != 0 {
                                return Err(vec![bytecode_error(
                                    "bytecode transactional mail_queue preflight is out of order",
                                    instruction.span,
                                )]);
                            }
                        }
                    }
                }
                vec![pc + 1]
            }
            BytecodeOp::Step => vec![pc + 1],
            BytecodeOp::StoreLocal(_) | BytecodeOp::Pop => {
                next_height = next_height.checked_sub(1).ok_or_else(|| {
                    vec![bytecode_error("bytecode stack underflow", instruction.span)]
                })?;
                vec![pc + 1]
            }
            BytecodeOp::Unary(_) => {
                require_stack(height, 1, instruction.span)?;
                vec![pc + 1]
            }
            BytecodeOp::Binary(_) => {
                require_stack(height, 2, instruction.span)?;
                next_height -= 1;
                vec![pc + 1]
            }
            BytecodeOp::BuildArray { items } | BytecodeOp::BuildTuple { items } => {
                require_stack(height, items, instruction.span)?;
                next_height = height - items + 1;
                vec![pc + 1]
            }
            BytecodeOp::BuildMap { entries, .. } => {
                let operands = entries.checked_mul(2).ok_or_else(|| {
                    vec![bytecode_error(
                        "bytecode map entry count overflows",
                        instruction.span,
                    )]
                })?;
                require_stack(height, operands, instruction.span)?;
                next_height = height - operands + 1;
                vec![pc + 1]
            }
            BytecodeOp::BuildRecord { ref fields, .. } => {
                require_stack(height, fields.len(), instruction.span)?;
                next_height = height - fields.len() + 1;
                vec![pc + 1]
            }
            BytecodeOp::RecordUpdate { ref fields, .. } => {
                let field_operands = fields.len().checked_mul(2).ok_or_else(|| {
                    vec![bytecode_error(
                        "bytecode record update field count overflows",
                        instruction.span,
                    )]
                })?;
                let operands = field_operands.checked_add(1).ok_or_else(|| {
                    vec![bytecode_error(
                        "bytecode record update field count overflows",
                        instruction.span,
                    )]
                })?;
                require_stack(height, operands, instruction.span)?;
                next_height = height - field_operands;
                vec![pc + 1]
            }
            BytecodeOp::BuildEnum {
                ref payload_types, ..
            } => {
                require_stack(height, payload_types.len(), instruction.span)?;
                next_height = height - payload_types.len() + 1;
                vec![pc + 1]
            }
            BytecodeOp::GetField { .. } => {
                require_stack(height, 1, instruction.span)?;
                vec![pc + 1]
            }
            BytecodeOp::RenderPdf => {
                require_stack(height, 2, instruction.span)?;
                next_height -= 1;
                vec![pc + 1]
            }
            BytecodeOp::Index => {
                require_stack(height, 2, instruction.span)?;
                next_height -= 1;
                vec![pc + 1]
            }
            BytecodeOp::ArrayIndexAssign { .. } => {
                require_stack(height, 2, instruction.span)?;
                next_height -= 2;
                vec![pc + 1]
            }
            BytecodeOp::ArrayAppend { .. } => {
                require_stack(height, 1, instruction.span)?;
                next_height -= 1;
                vec![pc + 1]
            }
            BytecodeOp::ArrayPopMut { .. } => {
                next_height += 1;
                vec![pc + 1]
            }
            BytecodeOp::OptionSome { .. } => {
                require_stack(height, 1, instruction.span)?;
                vec![pc + 1]
            }
            BytecodeOp::OptionNone { .. } => {
                next_height += 1;
                vec![pc + 1]
            }
            BytecodeOp::TupleBind { .. } => {
                next_height = next_height.checked_sub(1).ok_or_else(|| {
                    vec![bytecode_error(
                        "bytecode tuple binding stack underflow",
                        instruction.span,
                    )]
                })?;
                vec![pc + 1]
            }
            BytecodeOp::Parallel { .. } => vec![pc + 1],
            BytecodeOp::Call { arguments, .. } => {
                require_stack(height, arguments, instruction.span)?;
                next_height = height - arguments + 1;
                vec![pc + 1]
            }
            BytecodeOp::FunctionRef { .. } => {
                next_height += 1;
                vec![pc + 1]
            }
            BytecodeOp::MakeClosure { .. } => {
                next_height += 1;
                vec![pc + 1]
            }
            BytecodeOp::CallValue { arguments } => {
                let operands = arguments.checked_add(1).ok_or_else(|| {
                    vec![bytecode_error(
                        "bytecode function-value arity overflows",
                        instruction.span,
                    )]
                })?;
                require_stack(height, operands, instruction.span)?;
                next_height = height - arguments;
                vec![pc + 1]
            }
            BytecodeOp::HostCallPreflight {
                ref name,
                arguments,
            } => {
                pending_host_calls.push((name.clone(), arguments));
                vec![pc + 1]
            }
            BytecodeOp::ContractCheck { .. } => {
                next_height = next_height.checked_sub(1).ok_or_else(|| {
                    vec![bytecode_error(
                        "bytecode contract check stack underflow",
                        instruction.span,
                    )]
                })?;
                vec![pc + 1]
            }
            BytecodeOp::LoopInvariantCheck { .. } => {
                next_height = next_height.checked_sub(1).ok_or_else(|| {
                    vec![bytecode_error(
                        "bytecode loop invariant check stack underflow",
                        instruction.span,
                    )]
                })?;
                vec![pc + 1]
            }
            BytecodeOp::Builtin {
                ref name,
                arguments,
                ..
            } => {
                require_stack(height, arguments, instruction.span)?;
                next_height = height - arguments + 1;
                if bytecode_host_call_security(name).is_some() {
                    let Some((pending_name, pending_arguments)) = pending_host_calls.pop() else {
                        return Err(vec![bytecode_error(
                            format!("bytecode host builtin `{name}` has no preflight"),
                            instruction.span,
                        )]);
                    };
                    if pending_name != *name || pending_arguments != arguments {
                        return Err(vec![bytecode_error(
                            format!("bytecode host builtin `{name}` does not match its preflight"),
                            instruction.span,
                        )]);
                    }
                }
                vec![pc + 1]
            }
            BytecodeOp::Jump(target) => vec![target],
            BytecodeOp::JumpIfFalse(target) | BytecodeOp::JumpIfTrue(target) => {
                next_height = next_height.checked_sub(1).ok_or_else(|| {
                    vec![bytecode_error(
                        "bytecode branch stack underflow",
                        instruction.span,
                    )]
                })?;
                vec![target, pc + 1]
            }
            BytecodeOp::RangeInit { .. } => {
                require_stack(height, 3, instruction.span)?;
                next_height -= 3;
                vec![pc + 1]
            }
            BytecodeOp::RangeNext { body, exit, .. } => vec![body, exit],
            BytecodeOp::CollectionInit { .. } => {
                next_height = next_height.checked_sub(1).ok_or_else(|| {
                    vec![bytecode_error("bytecode stack underflow", instruction.span)]
                })?;
                vec![pc + 1]
            }
            BytecodeOp::CollectionNext { body, exit, .. } => vec![body, exit],
            BytecodeOp::MatchInit { .. } => {
                next_height = next_height.checked_sub(1).ok_or_else(|| {
                    vec![bytecode_error(
                        "bytecode match stack underflow",
                        instruction.span,
                    )]
                })?;
                vec![pc + 1]
            }
            BytecodeOp::MatchTest {
                matched, unmatched, ..
            } => vec![matched, unmatched],
            BytecodeOp::MatchFailure => Vec::new(),
            BytecodeOp::Return => {
                if height != 1 {
                    return Err(vec![bytecode_error(
                        "bytecode return must have exactly one value on the operand stack",
                        instruction.span,
                    )]);
                }
                if !pending_host_calls.is_empty() {
                    return Err(vec![bytecode_error(
                        "bytecode function returns with an unfinished host-call preflight",
                        instruction.span,
                    )]);
                }
                saw_return = true;
                Vec::new()
            }
            BytecodeOp::Trap => Vec::new(),
        };
        maximum = maximum.max(next_height);
        for successor in successors {
            if successor >= function.code.len() {
                return Err(vec![bytecode_error(
                    "bytecode control flow falls outside the function",
                    instruction.span,
                )]);
            }
            let next_state = (next_height, pending_host_calls.clone());
            match heights.get(&successor) {
                Some(existing) if *existing != next_state => {
                    return Err(vec![bytecode_error(
                        "bytecode stack or host-call preflight state disagrees at a control-flow join",
                        function.code[successor].span,
                    )]);
                }
                Some(_) => {}
                None => {
                    heights.insert(successor, next_state);
                    pending.push_back(successor);
                }
            }
        }
    }
    if !saw_return {
        return Err(vec![bytecode_error(
            "bytecode function has no reachable return",
            span,
        )]);
    }
    Ok(maximum)
}

fn require_stack(height: usize, required: usize, span: Span) -> Result<(), Vec<MirError>> {
    if height < required {
        Err(vec![bytecode_error("bytecode stack underflow", span)])
    } else {
        Ok(())
    }
}

fn bytecode_host_call_security(name: &str) -> Option<(&'static str, &'static str, usize)> {
    match name {
        "now" => Some(("Clock", "clock access", 0)),
        "env" => Some(("Environment", "environment access", 1)),
        "read_console" => Some(("Console", "console input", 1)),
        "random_int" | "uuid_v4" => Some((
            "Random",
            "random access",
            if name == "random_int" { 2 } else { 0 },
        )),
        "read_text" | "read_text_result" | "file_exists" | "dir_exists" | "file_size"
        | "list_dir" => Some(("FileSystem", "file-system read access", 1)),
        "write_text"
        | "write_text_result"
        | "write_text_new_result"
        | "atomic_write_text_result"
        | "append_text_result" => Some(("FileSystem", "file-system write access", 2)),
        "delete_file" | "delete_file_result" => {
            Some(("FileSystem", "file-system delete access", 1))
        }
        "copy_file_new_result" | "rename_file_result" => {
            Some(("FileSystem", "file-system write access", 2))
        }
        "run_process" => Some(("Process", "process execution", 2)),
        "http_get" => Some(("Network", "network access", 1)),
        "http_request" | "http_json" | "http_result" => Some(("Network", "network access", 4)),
        "mail_deliver" => Some(("Email", "email delivery", 2)),
        "mail_queue" => Some(("Email", "email outbox", 0)),
        "mail_queue_db_write" => Some(("Database(write)", "transactional mail queue write", 0)),
        _ => None,
    }
}

fn require_vm_capability(
    function: &BytecodeFunction,
    grants: Option<&HashSet<String>>,
    capability: &str,
    operation: &str,
    span: Span,
) -> Result<(), RuntimeError> {
    if !function
        .declared_capabilities
        .iter()
        .any(|declared| declared == capability)
    {
        return Err(runtime_error(
            span,
            format!(
                "runtime capability denied: {operation} requires `{capability}` in the current function"
            ),
        ));
    }
    if !grants.is_some_and(|grants| zelyra_runtime::project_grants_capability(grants, capability)) {
        return Err(runtime_error(
            span,
            format!("runtime capability denied: `{capability}` is not granted"),
        ));
    }
    Ok(())
}

/// Execute the verified bytecode with an iterative Rust-side call stack.
pub fn execute(
    program: &BytecodeProgram,
    arguments: Vec<Value>,
    limits: ExecutionLimits,
) -> Result<ExecutionResult, RuntimeError> {
    execute_with_host_services(
        program,
        arguments,
        limits,
        None,
        Arc::new(SystemHostServices),
    )
}

/// Execute bytecode with explicitly injected host services and project grants.
/// Clock, environment, console, and random effects always require both a source-declared
/// function capability retained in the artifact and an explicit matching grant here.
pub fn execute_with_host_services(
    program: &BytecodeProgram,
    arguments: Vec<Value>,
    limits: ExecutionLimits,
    grants: Option<&HashSet<String>>,
    host_services: Arc<dyn HostServices>,
) -> Result<ExecutionResult, RuntimeError> {
    let outcome =
        execute_with_host_services_captured(program, arguments, limits, grants, host_services);
    if let Some(error) = outcome.error {
        return Err(error);
    }
    Ok(ExecutionResult {
        value: outcome.value.unwrap_or(Value::Unit),
        steps: outcome.steps,
    })
}

/// Execute bytecode and preserve output emitted before either success or error.
/// This matches the captured-output contract used by the AST and MIR runners.
pub fn execute_with_host_services_captured(
    program: &BytecodeProgram,
    arguments: Vec<Value>,
    limits: ExecutionLimits,
    grants: Option<&HashSet<String>>,
    host_services: Arc<dyn HostServices>,
) -> ExecutionOutcome {
    execute_with_host_services_and_policy_captured(
        program,
        arguments,
        limits,
        grants,
        None,
        host_services,
    )
}

/// Execute bytecode with explicit project grants, host services, and runtime policy.
/// File-system builtins use the same path containment and symlink checks as the AST runtime.
pub fn execute_with_host_services_and_policy_captured(
    program: &BytecodeProgram,
    arguments: Vec<Value>,
    limits: ExecutionLimits,
    grants: Option<&HashSet<String>>,
    runtime_policy: Option<&RuntimePolicy>,
    host_services: Arc<dyn HostServices>,
) -> ExecutionOutcome {
    execute_with_database_context(
        program,
        arguments,
        limits,
        grants,
        runtime_policy,
        None,
        host_services,
    )
}

/// Execute bytecode with a database URL and checked project schema. SQL
/// expressions still require the function capability and project grant.
pub fn execute_with_database_and_host_services_and_policy_captured(
    program: &BytecodeProgram,
    arguments: Vec<Value>,
    limits: ExecutionLimits,
    grants: Option<&HashSet<String>>,
    runtime_policy: Option<&RuntimePolicy>,
    database: BytecodeDatabaseExecution<'_>,
    host_services: Arc<dyn HostServices>,
) -> ExecutionOutcome {
    execute_with_database_context(
        program,
        arguments,
        limits,
        grants,
        runtime_policy,
        Some(database),
        host_services,
    )
}

fn execute_with_database_context(
    program: &BytecodeProgram,
    arguments: Vec<Value>,
    limits: ExecutionLimits,
    grants: Option<&HashSet<String>>,
    runtime_policy: Option<&RuntimePolicy>,
    database: Option<BytecodeDatabaseExecution<'_>>,
    host_services: Arc<dyn HostServices>,
) -> ExecutionOutcome {
    let limits = bounded_execution_limits(limits);
    execute_bytecode_entry(
        program,
        program.entry,
        arguments,
        BytecodeExecutionContext {
            limits,
            grants,
            runtime_policy,
            database,
            host_services,
            shared_steps: StepBudget::new(Some(limits.max_steps)),
            shared_budget_active: false,
        },
    )
}

#[derive(Clone)]
struct BytecodeExecutionContext<'a> {
    limits: ExecutionLimits,
    grants: Option<&'a HashSet<String>>,
    runtime_policy: Option<&'a RuntimePolicy>,
    database: Option<BytecodeDatabaseExecution<'a>>,
    host_services: Arc<dyn HostServices>,
    shared_steps: StepBudget,
    shared_budget_active: bool,
}

fn consume_bytecode_step(budget: &StepBudget, span: Span) -> Result<(), RuntimeError> {
    match budget.consume() {
        Ok(_) => Ok(()),
        Err(StepBudgetError::LimitExceeded { limit, .. }) => Err(runtime_error(
            span,
            format!("execution step budget exceeded (limit {limit})"),
        )),
        Err(StepBudgetError::CounterOverflow) => {
            Err(runtime_error(span, "execution step counter overflow"))
        }
    }
}

fn consumed_bytecode_steps(
    shared_budget_active: bool,
    shared_steps: &StepBudget,
    local_steps: usize,
) -> usize {
    if shared_budget_active {
        shared_steps.consumed()
    } else {
        local_steps
    }
}

fn charge_bytecode_step(
    span: Span,
    shared_budget_active: bool,
    shared_steps: &StepBudget,
    local_steps: &mut usize,
    limit: usize,
) -> Result<(), RuntimeError> {
    if shared_budget_active {
        return consume_bytecode_step(shared_steps, span);
    }
    *local_steps = local_steps
        .checked_add(1)
        .ok_or_else(|| runtime_error(span, "execution step counter overflow"))?;
    if *local_steps > limit {
        return Err(runtime_error(
            span,
            format!("execution step budget exceeded (limit {limit})"),
        ));
    }
    Ok(())
}

fn execute_bytecode_entry(
    program: &BytecodeProgram,
    entry_index: usize,
    arguments: Vec<Value>,
    context: BytecodeExecutionContext<'_>,
) -> ExecutionOutcome {
    let BytecodeExecutionContext {
        limits,
        grants,
        runtime_policy,
        database,
        host_services,
        mut shared_steps,
        mut shared_budget_active,
    } = context;
    let verification_error = verify_bytecode(program).err().map(|errors| {
        let error = errors.into_iter().next();
        error.map_or_else(
            || runtime_error(Span::default(), "bytecode verification failed"),
            |error| runtime_error(error.span, format!("{}: {}", error.code, error.message)),
        )
    });
    if let Some(error) = verification_error {
        return ExecutionOutcome {
            output: Vec::new(),
            value: None,
            steps: 0,
            error: Some(error),
        };
    }
    if let Some(database) = database {
        if let Ok(schema) = database.schema {
            let mut schema_scopes = schema
                .tables
                .iter()
                .filter(|table| table.tenant_scope_column.is_some())
                .map(|table| table.name.to_ascii_lowercase())
                .collect::<Vec<_>>();
            let mut artifact_scopes = program
                .scoped_tables
                .iter()
                .map(|table| table.to_ascii_lowercase())
                .collect::<Vec<_>>();
            schema_scopes.sort();
            artifact_scopes.sort();
            if schema_scopes != artifact_scopes {
                return ExecutionOutcome {
                    output: Vec::new(),
                    value: None,
                    steps: 0,
                    error: Some(runtime_error(
                        Span::default(),
                        "bytecode tenant-scope metadata does not match the checked project schema",
                    )),
                };
            }
        }
    }
    if limits.max_stack_depth == 0 {
        return ExecutionOutcome {
            output: Vec::new(),
            value: None,
            steps: 0,
            error: Some(runtime_error(
                Span::default(),
                "bytecode maximum call stack depth exceeded",
            )),
        };
    }
    let Some(entry) = program.functions.get(entry_index) else {
        return ExecutionOutcome {
            output: Vec::new(),
            value: None,
            steps: shared_steps.consumed(),
            error: Some(runtime_error(
                Span::default(),
                "bytecode entry index is outside the function table",
            )),
        };
    };
    let frame = match make_frame(entry_index, entry, arguments, 0) {
        Ok(frame) => frame,
        Err(error) => {
            return ExecutionOutcome {
                output: Vec::new(),
                value: None,
                steps: 0,
                error: Some(error),
            };
        }
    };
    let mut frames = vec![frame];
    let mut stack = Vec::<Value>::new();
    let mut pending_host_calls = Vec::<(usize, String, usize)>::new();
    let mut output = Vec::new();
    let mut executed_operations = 0usize;
    let mut local_steps = 0usize;
    let execution = (|| -> Result<ExecutionResult, RuntimeError> {
        loop {
            let frame_index = frames.len() - 1;
            let function_index = frames[frame_index].function;
            let function = &program.functions[function_index];
            let frame = &mut frames[frame_index];
            let instruction = function.code.get(frame.pc).cloned().ok_or_else(|| {
                runtime_error(
                    Span::default(),
                    "bytecode program counter is outside the function",
                )
            })?;
            executed_operations = executed_operations.saturating_add(1);
            if executed_operations > MAX_BYTECODE_OPERATIONS_PER_EXECUTION {
                return Err(runtime_error(
                    instruction.span,
                    "bytecode operation safety limit exceeded",
                ));
            }
            if let Some(span) = instruction.source_step_span {
                charge_bytecode_step(
                    span,
                    shared_budget_active,
                    &shared_steps,
                    &mut local_steps,
                    limits.max_steps,
                )?;
            }
            if matches!(instruction.operation, BytecodeOp::Step) {
                charge_bytecode_step(
                    instruction.span,
                    shared_budget_active,
                    &shared_steps,
                    &mut local_steps,
                    limits.max_steps,
                )?;
            }
            frame.pc += 1;
            match instruction.operation {
                BytecodeOp::Step => {}
                BytecodeOp::Constant(value) => stack.push(value),
                BytecodeOp::FunctionRef { function: target } => {
                    let target_index = target;
                    let target = program.functions.get(target).ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode function reference is invalid")
                    })?;
                    if !target.captures.is_empty() {
                        return Err(runtime_error(
                            instruction.span,
                            "capturing bytecode function must be created as a closure",
                        ));
                    }
                    stack.push(Value::Function(Box::new(FunctionValue {
                        name: target.name.clone(),
                        params: target
                            .parameters
                            .iter()
                            .map(|(_, ty, _)| ty.clone())
                            .collect(),
                        return_type: target.return_type.clone(),
                        capabilities: target.declared_capabilities.clone(),
                        lambda_params: Vec::new(),
                        lambda_body: None,
                        captures: Vec::new(),
                        bytecode_target: Some(target_index),
                        bytecode_captures: Vec::new(),
                    })));
                }
                BytecodeOp::MakeClosure {
                    function: target_index,
                    captures,
                } => {
                    let target = program.functions.get(target_index).ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode closure target is invalid")
                    })?;
                    if target.captures.len() != captures.len() {
                        return Err(runtime_error(
                            instruction.span,
                            "bytecode closure capture count does not match its target",
                        ));
                    }
                    let mut captured_values = Vec::with_capacity(captures.len());
                    for ((expected_local, expected_type, capture_span), local) in
                        target.captures.iter().zip(captures)
                    {
                        if *expected_local != local {
                            return Err(runtime_error(
                                instruction.span,
                                "bytecode closure capture layout does not match its target",
                            ));
                        }
                        let value = frame.locals.get(&local).cloned().ok_or_else(|| {
                            runtime_error(
                                *capture_span,
                                "bytecode closure capture is not initialized",
                            )
                        })?;
                        if !runtime_type_matches(expected_type, &value.ty()) {
                            return Err(runtime_error(
                                *capture_span,
                                format!(
                                    "bytecode closure capture expected `{expected_type}` but received `{}`",
                                    value.ty()
                                ),
                            ));
                        }
                        captured_values.push((local.0, value));
                    }
                    stack.push(Value::Function(Box::new(FunctionValue {
                        name: target.name.clone(),
                        params: target
                            .parameters
                            .iter()
                            .map(|(_, ty, _)| ty.clone())
                            .collect(),
                        return_type: target.return_type.clone(),
                        capabilities: target.declared_capabilities.clone(),
                        lambda_params: Vec::new(),
                        lambda_body: None,
                        captures: Vec::new(),
                        bytecode_target: Some(target_index),
                        bytecode_captures: captured_values,
                    })));
                }
                BytecodeOp::LoadLocal(local) => {
                    let value = frame.locals.get(&local).cloned().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode local is not initialized")
                    })?;
                    stack.push(value);
                }
                BytecodeOp::StoreLocal(local) => {
                    let value = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode stack underflow")
                    })?;
                    frame.locals.insert(local, value);
                }
                BytecodeOp::Pop => {
                    stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode stack underflow")
                    })?;
                }
                BytecodeOp::Unary(operator) => {
                    let value = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode stack underflow")
                    })?;
                    stack.push(evaluate_unary(operator, value, instruction.span)?);
                }
                BytecodeOp::Binary(operator) => {
                    let right = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode stack underflow")
                    })?;
                    let left = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode stack underflow")
                    })?;
                    stack.push(evaluate_binary(left, operator, right, instruction.span)?);
                }
                BytecodeOp::BuildArray { items } => {
                    let start = stack.len().checked_sub(items).ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode collection stack underflow")
                    })?;
                    let values = stack.drain(start..).collect::<Vec<_>>();
                    stack.push(Value::Array(values));
                }
                BytecodeOp::BuildMap {
                    entries,
                    key_type,
                    value_type,
                } => {
                    let operands = entries.checked_mul(2).ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode map entry count overflows")
                    })?;
                    let start = stack.len().checked_sub(operands).ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode map stack underflow")
                    })?;
                    let values = stack.drain(start..).collect::<Vec<_>>();
                    let mut map_entries = Vec::with_capacity(entries);
                    let mut values = values.into_iter();
                    for _ in 0..entries {
                        let key = values.next().ok_or_else(|| {
                            runtime_error(instruction.span, "bytecode map key is missing")
                        })?;
                        let value = values.next().ok_or_else(|| {
                            runtime_error(instruction.span, "bytecode map value is missing")
                        })?;
                        if !runtime_type_matches(&key_type, &key.ty()) {
                            return Err(runtime_error(
                                instruction.span,
                                format!(
                                    "bytecode map key expected `{key_type}`, got `{}`",
                                    key.ty()
                                ),
                            ));
                        }
                        if !runtime_type_matches(&value_type, &value.ty()) {
                            return Err(runtime_error(
                                instruction.span,
                                format!(
                                    "bytecode map value expected `{value_type}`, got `{}`",
                                    value.ty()
                                ),
                            ));
                        }
                        map_entries.push((key, value));
                    }
                    stack.push(Value::Map(normalize_map_entries(map_entries)));
                }
                BytecodeOp::BuildTuple { items } => {
                    let start = stack.len().checked_sub(items).ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode tuple stack underflow")
                    })?;
                    let values = stack.drain(start..).collect::<Vec<_>>();
                    stack.push(Value::Tuple(values));
                }
                BytecodeOp::BuildRecord {
                    record_type,
                    fields,
                } => {
                    let start = stack.len().checked_sub(fields.len()).ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode record stack underflow")
                    })?;
                    let values = stack.drain(start..).collect::<Vec<_>>();
                    let (type_name, type_args) =
                        record_type_parts(&record_type).ok_or_else(|| {
                            runtime_error(instruction.span, "bytecode record type is invalid")
                        })?;
                    let mut record_fields = HashMap::with_capacity(fields.len());
                    for ((name, expected), value) in fields.into_iter().zip(values) {
                        if !runtime_type_matches(&expected, &value.ty()) {
                            return Err(runtime_error(
                                instruction.span,
                                format!(
                                "bytecode record field `{name}` expected `{expected}`, got `{}`",
                                value.ty()
                            ),
                            ));
                        }
                        record_fields.insert(name, value);
                    }
                    stack.push(Value::Object {
                        type_name: type_name.to_owned(),
                        type_args: type_args.to_vec(),
                        fields: record_fields,
                    });
                }
                BytecodeOp::RecordUpdate {
                    record_type,
                    fields,
                } => {
                    let operand_count = fields.len().checked_mul(2).ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode record update count overflows")
                    })?;
                    let start = stack.len().checked_sub(operand_count).ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode record update stack underflow")
                    })?;
                    let operands = stack.drain(start..).collect::<Vec<_>>();
                    let base = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode record update base is missing")
                    })?;
                    if !runtime_type_matches(&record_type, &base.ty()) {
                        return Err(runtime_error(
                            instruction.span,
                            format!(
                                "bytecode record update expected `{record_type}`, got `{}`",
                                base.ty()
                            ),
                        ));
                    }
                    let Value::Object {
                        type_name,
                        type_args,
                        fields: mut record_fields,
                    } = base
                    else {
                        return Err(runtime_error(
                            instruction.span,
                            "bytecode record update requires a record value",
                        ));
                    };
                    for ((name, expected), pair) in fields.into_iter().zip(operands.chunks(2)) {
                        let Value::String(actual_name) = &pair[0] else {
                            return Err(runtime_error(
                                instruction.span,
                                "bytecode record update field name is not a string",
                            ));
                        };
                        if actual_name != &name {
                            return Err(runtime_error(
                            instruction.span,
                            "bytecode record update field name does not match verified metadata",
                        ));
                        }
                        let value = pair[1].clone();
                        let Some(existing_value) = record_fields.get(&name) else {
                            return Err(runtime_error(
                                instruction.span,
                                format!(
                                "bytecode record update field `{name}` is missing from the base"
                            ),
                            ));
                        };
                        if !runtime_type_matches(&expected, &existing_value.ty()) {
                            return Err(runtime_error(
                                instruction.span,
                                format!(
                                "bytecode record update field `{name}` has invalid base type `{}`",
                                existing_value.ty()
                            ),
                            ));
                        }
                        if !runtime_type_matches(&expected, &value.ty()) {
                            return Err(runtime_error(
                            instruction.span,
                            format!(
                                "bytecode record update field `{name}` expected `{expected}`, got `{}`",
                                value.ty()
                            ),
                        ));
                        }
                        record_fields.insert(name, value);
                    }
                    stack.push(Value::Object {
                        type_name,
                        type_args,
                        fields: record_fields,
                    });
                }
                BytecodeOp::Parallel { bindings } => {
                    for binding in &bindings {
                        if !binding.is_new && !frame.locals.contains_key(&binding.target) {
                            return Err(runtime_error(
                                instruction.span,
                                "bytecode parallel assignment targets an uninitialized local",
                            ));
                        }
                    }
                    let branch_inputs = bindings
                        .iter()
                        .map(|binding| {
                            binding
                                .captures
                                .iter()
                                .map(|local| {
                                    frame.locals.get(local).cloned().ok_or_else(|| {
                                        runtime_error(
                                            instruction.span,
                                            format!(
                                                "bytecode parallel capture local {} is not initialized",
                                                local.0
                                            ),
                                        )
                                    })
                                })
                                .collect::<Result<Vec<_>, _>>()
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    let was_shared_budget_active = shared_budget_active;
                    if !shared_budget_active {
                        shared_steps = StepBudget::starting_at(Some(limits.max_steps), local_steps);
                        shared_budget_active = true;
                    }
                    let results = std::thread::scope(|scope| {
                        let handles = bindings
                            .iter()
                            .zip(branch_inputs)
                            .map(|(binding, arguments)| {
                                let host_services = Arc::clone(&host_services);
                                let shared_steps = shared_steps.clone();
                                std::thread::Builder::new()
                                    .name("zelyra-bytecode-parallel".into())
                                    .stack_size(2 * 1024 * 1024)
                                    .spawn_scoped(scope, move || {
                                        let outcome = execute_bytecode_entry(
                                            program,
                                            binding.function,
                                            arguments,
                                            BytecodeExecutionContext {
                                                limits,
                                                grants,
                                                runtime_policy,
                                                database,
                                                host_services,
                                                shared_steps,
                                                shared_budget_active: true,
                                            },
                                        );
                                        (binding.target, outcome)
                                    })
                            })
                            .collect::<Vec<_>>();
                        let mut assignments = Vec::with_capacity(handles.len());
                        let mut branch_outputs = Vec::with_capacity(handles.len());
                        let mut first_error = None;
                        for handle in handles {
                            match handle {
                                Ok(handle) => match handle.join() {
                                    Ok((target, outcome)) => {
                                        branch_outputs.push(outcome.output);
                                        if let Some(error) = outcome.error {
                                            if first_error.is_none() {
                                                first_error = Some(error);
                                            }
                                        } else if let Some(value) = outcome.value {
                                            assignments.push((target, value));
                                        } else if first_error.is_none() {
                                            first_error = Some(runtime_error(
                                                instruction.span,
                                                "bytecode parallel branch returned no value",
                                            ));
                                        }
                                    }
                                    Err(_) if first_error.is_none() => {
                                        first_error = Some(runtime_error(
                                            instruction.span,
                                            "bytecode parallel branch panicked",
                                        ));
                                    }
                                    Err(_) => {}
                                },
                                Err(_) if first_error.is_none() => {
                                    first_error = Some(runtime_error(
                                        instruction.span,
                                        "bytecode parallel worker could not be started",
                                    ));
                                }
                                Err(_) => {}
                            }
                        }
                        (assignments, branch_outputs, first_error)
                    });
                    if !was_shared_budget_active {
                        local_steps = shared_steps.consumed();
                        shared_budget_active = false;
                    }
                    if let Some(mut error) = results.2 {
                        if error.message.contains("execution step budget exceeded") {
                            error = runtime_error(instruction.span, error.message);
                        }
                        return Err(error);
                    }
                    for branch_output in results.1 {
                        output.extend(branch_output);
                    }
                    for (target, value) in results.0 {
                        if !runtime_type_matches(
                            &bindings
                                .iter()
                                .find(|binding| binding.target == target)
                                .expect("verified parallel target exists")
                                .result_type,
                            &value.ty(),
                        ) {
                            return Err(runtime_error(
                                instruction.span,
                                "bytecode parallel branch returned an invalid result type",
                            ));
                        }
                        frame.locals.insert(target, value);
                    }
                }
                BytecodeOp::BuildEnum {
                    enum_type,
                    variant,
                    payload_types,
                } => {
                    let start = stack
                        .len()
                        .checked_sub(payload_types.len())
                        .ok_or_else(|| {
                            runtime_error(instruction.span, "bytecode enum payload stack underflow")
                        })?;
                    let values = stack.drain(start..).collect::<Vec<_>>();
                    for (expected, value) in payload_types.iter().zip(&values) {
                        if !runtime_type_matches(expected, &value.ty()) {
                            return Err(runtime_error(
                                instruction.span,
                                format!("enum payload expected `{expected}`, got `{}`", value.ty()),
                            ));
                        }
                    }
                    let (type_name, type_args) =
                        record_type_parts(&enum_type).ok_or_else(|| {
                            runtime_error(instruction.span, "bytecode enum type is invalid")
                        })?;
                    stack.push(Value::Enum {
                        type_name: type_name.to_owned(),
                        type_args: type_args.to_vec(),
                        variant,
                        payloads: values,
                    });
                }
                BytecodeOp::RenderPdf => {
                    let data = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "render_pdf data stack underflow")
                    })?;
                    let view_name = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "render_pdf view-name stack underflow")
                    })?;
                    let Value::String(view_name) = view_name else {
                        return Err(runtime_error(
                            instruction.span,
                            "render_pdf view name must be a string",
                        ));
                    };
                    let view = program
                        .views
                        .iter()
                        .find(|view| view.name == view_name)
                        .ok_or_else(|| {
                            runtime_error(
                                instruction.span,
                                format!("PDF view `{view_name}` was not found"),
                            )
                        })?;
                    let pdf = render_pdf_view(view, &data).map_err(|message| {
                        runtime_error(instruction.span, format!("PDF rendering failed: {message}"))
                    })?;
                    stack.push(Value::Bytes(pdf));
                }
                BytecodeOp::GetField {
                    record_type,
                    field,
                    result_type,
                } => {
                    let value = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode field stack underflow")
                    })?;
                    if !runtime_type_matches(&record_type, &value.ty()) {
                        return Err(runtime_error(
                            instruction.span,
                            format!(
                                "bytecode field access expected `{record_type}`, got `{}`",
                                value.ty()
                            ),
                        ));
                    }
                    let Value::Object { fields, .. } = value else {
                        return Err(runtime_error(
                            instruction.span,
                            "bytecode field access requires a record value",
                        ));
                    };
                    let field_value = fields.get(&field).cloned().ok_or_else(|| {
                        runtime_error(
                            instruction.span,
                            format!("record field `{field}` is missing"),
                        )
                    })?;
                    if !runtime_type_matches(&result_type, &field_value.ty()) {
                        return Err(runtime_error(
                            instruction.span,
                            format!(
                                "bytecode field `{field}` returned `{}`, expected `{result_type}`",
                                field_value.ty()
                            ),
                        ));
                    }
                    stack.push(field_value);
                }
                BytecodeOp::Index => {
                    let index = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode index stack underflow")
                    })?;
                    let target = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode index stack underflow")
                    })?;
                    stack.push(index_value(target, index, instruction.span)?);
                }
                BytecodeOp::ArrayIndexAssign { local, operation } => {
                    let value = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode array-update stack underflow")
                    })?;
                    let index = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode array-update stack underflow")
                    })?;
                    let values_len = match frame.locals.get(&local) {
                        Some(Value::Array(values)) => values.len(),
                        Some(other) => {
                            return Err(runtime_error(
                                instruction.span,
                                format!("indexed assignment requires Array, got `{}`", other.ty()),
                            ));
                        }
                        None => {
                            return Err(runtime_error(
                                instruction.span,
                                "bytecode indexed assignment targets an uninitialized local",
                            ));
                        }
                    };
                    let index = array_assignment_index(index, values_len, instruction.span)?;
                    let (expected_type, current) = match frame.locals.get(&local) {
                        Some(Value::Array(values)) => {
                            let current = values.get(index).ok_or_else(|| {
                                runtime_error(
                                    instruction.span,
                                    "bytecode mutable array index is out of bounds",
                                )
                            })?;
                            (
                                current.ty(),
                                (operation == ArrayIndexAssignOp::Add).then(|| current.clone()),
                            )
                        }
                        _ => {
                            return Err(runtime_error(
                                instruction.span,
                                "bytecode array update target changed during evaluation",
                            ));
                        }
                    };
                    let value = match current {
                        None => value,
                        Some(current) => {
                            evaluate_binary(current, BinaryOp::Add, value, instruction.span)?
                        }
                    };
                    if !runtime_type_matches(&expected_type, &value.ty()) {
                        return Err(runtime_error(
                            instruction.span,
                            format!(
                                "bytecode array update expected `{expected_type}`, got `{}`",
                                value.ty()
                            ),
                        ));
                    }
                    let Some(Value::Array(values)) = frame.locals.get_mut(&local) else {
                        return Err(runtime_error(
                            instruction.span,
                            "bytecode array update target changed during evaluation",
                        ));
                    };
                    let Some(element) = values.get_mut(index) else {
                        return Err(runtime_error(
                            instruction.span,
                            "bytecode mutable array index is out of bounds",
                        ));
                    };
                    *element = value;
                }
                BytecodeOp::ArrayAppend {
                    local,
                    element_type,
                } => {
                    let value = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode array append stack underflow")
                    })?;
                    if !runtime_type_matches(&element_type, &value.ty()) {
                        return Err(runtime_error(
                            instruction.span,
                            format!(
                                "bytecode array append expected `{element_type}`, got `{}`",
                                value.ty()
                            ),
                        ));
                    }
                    let Some(Value::Array(values)) = frame.locals.get_mut(&local) else {
                        return Err(runtime_error(
                            instruction.span,
                            "bytecode array append targets an uninitialized or non-array local",
                        ));
                    };
                    values.push(value);
                }
                BytecodeOp::ArrayPopMut {
                    local,
                    element_type,
                } => {
                    let Some(Value::Array(values)) = frame.locals.get_mut(&local) else {
                        return Err(runtime_error(
                            instruction.span,
                            "bytecode array pop targets an uninitialized or non-array local",
                        ));
                    };
                    let value = values.pop();
                    if let Some(value) = &value {
                        if !runtime_type_matches(&element_type, &value.ty()) {
                            return Err(runtime_error(
                                instruction.span,
                                format!(
                                    "bytecode array pop expected `{element_type}`, got `{}`",
                                    value.ty()
                                ),
                            ));
                        }
                    }
                    stack.push(Value::Option(value.map(Box::new)));
                }
                BytecodeOp::OptionSome { value_type } => {
                    let value = stack.pop().ok_or_else(|| {
                        runtime_error(
                            instruction.span,
                            "bytecode option construction stack underflow",
                        )
                    })?;
                    if !runtime_type_matches(&value_type, &value.ty()) {
                        return Err(runtime_error(
                            instruction.span,
                            format!(
                                "bytecode option payload expected `{value_type}`, got `{}`",
                                value.ty()
                            ),
                        ));
                    }
                    stack.push(Value::Option(Some(Box::new(value))));
                }
                BytecodeOp::OptionNone { .. } => stack.push(Value::Option(None)),
                BytecodeOp::TupleBind { locals } => {
                    let value = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode tuple binding stack underflow")
                    })?;
                    let Value::Tuple(values) = value else {
                        return Err(runtime_error(
                            instruction.span,
                            "MIR tuple binding requires a Tuple value",
                        ));
                    };
                    if locals.len() != values.len() {
                        return Err(runtime_error(
                            instruction.span,
                            format!(
                                "MIR tuple binding expects {} values, received {}",
                                locals.len(),
                                values.len()
                            ),
                        ));
                    }
                    frame.locals.extend(locals.into_iter().zip(values));
                }
                BytecodeOp::HostCallPreflight { name, arguments } => {
                    let Some((capability, operation, expected_arguments)) =
                        bytecode_host_call_security(&name)
                    else {
                        return Err(runtime_error(
                            instruction.span,
                            format!("bytecode host-call preflight `{name}` is unsupported"),
                        ));
                    };
                    if arguments != expected_arguments {
                        return Err(runtime_error(
                            instruction.span,
                            format!("bytecode host-call preflight `{name}` has invalid arity"),
                        ));
                    }
                    require_vm_capability(
                        function,
                        grants,
                        capability,
                        operation,
                        instruction.span,
                    )?;
                    pending_host_calls.push((function_index, name, arguments));
                }
                BytecodeOp::Sql {
                    query,
                    parameters,
                    result_type,
                } => {
                    zelyra_runtime::check_scoped_sql(&query, &program.scoped_tables)
                        .map_err(|message| runtime_error(instruction.span, message))?;
                    let capability = zelyra_runtime::sql_required_capability(&query);
                    require_vm_capability(
                        function,
                        grants,
                        capability,
                        "SQL access",
                        instruction.span,
                    )?;
                    let database = database.ok_or_else(|| {
                        runtime_error(
                            instruction.span,
                            "SQL execution requires a database runtime",
                        )
                    })?;
                    let mut named_values = HashMap::new();
                    let mut parameter_types = HashMap::new();
                    for (name, local) in parameters {
                        let value = frame.locals.get(&local).cloned().ok_or_else(|| {
                            runtime_error(
                                instruction.span,
                                format!("SQL parameter `:{name}` is not available"),
                            )
                        })?;
                        parameter_types.insert(name.clone(), value.ty());
                        named_values.insert(name, value);
                    }
                    let schema = database.schema.as_ref().map_err(|errors| {
                        runtime_error(
                            instruction.span,
                            errors
                                .first()
                                .map_or("invalid database schema", |error| error.message.as_str()),
                        )
                    })?;
                    zelyra_database::sql::check_sql_query(
                        &query,
                        &result_type,
                        schema,
                        &parameter_types,
                        instruction.span,
                    )
                    .map_err(|errors| {
                        runtime_error(
                            instruction.span,
                            format!(
                                "SQL validation failed: {}",
                                errors
                                    .first()
                                    .map_or("invalid SQL query", |error| error.message.as_str())
                            ),
                        )
                    })?;
                    let params = zelyra_runtime::sql_parameters_from_named_values(
                        &query,
                        &named_values,
                        instruction.span,
                    )?;
                    zelyra_runtime::validate_database_storage_with_host(
                        database.url,
                        database.schema,
                        host_services.as_ref(),
                        instruction.span,
                    )?;
                    let result = zelyra_runtime::execute_database_query_with_host(
                        database.url,
                        &zelyra_database::Query { sql: query, params },
                        host_services.as_ref(),
                        instruction.span,
                    )?;
                    stack.push(Value::Rows {
                        columns: result.columns,
                        rows: result.rows,
                        metadata: Box::new(zelyra_runtime::SqlRowsMetadata {
                            nulls: result.nulls,
                            column_types: result.column_types,
                        }),
                        result_type: Box::new(result_type),
                    });
                }
                BytecodeOp::SqlTransaction { operations } => {
                    let database = database.ok_or_else(|| {
                        runtime_error(
                            instruction.span,
                            "SQL transaction requires a database runtime",
                        )
                    })?;
                    let schema = database.schema.as_ref().map_err(|errors| {
                        runtime_error(
                            instruction.span,
                            errors
                                .first()
                                .map_or("invalid database schema", |error| error.message.as_str()),
                        )
                    })?;
                    let value_count = operations
                        .iter()
                        .map(|operation| match operation {
                            BytecodeTransactionOperation::Sql(_) => 0,
                            BytecodeTransactionOperation::MailQueue { arguments, .. } => *arguments,
                        })
                        .sum::<usize>();
                    let start = stack.len().checked_sub(value_count).ok_or_else(|| {
                        runtime_error(
                            instruction.span,
                            "bytecode transaction argument stack underflow",
                        )
                    })?;
                    let values = stack.drain(start..).collect::<Vec<_>>();
                    let mut value_cursor = 0usize;
                    let mut queries = Vec::with_capacity(operations.len());
                    for operation in operations {
                        match operation {
                            BytecodeTransactionOperation::Sql(statement) => {
                                zelyra_runtime::check_scoped_sql(
                                    &statement.query,
                                    &program.scoped_tables,
                                )
                                .map_err(|message| runtime_error(instruction.span, message))?;
                                let capability =
                                    zelyra_runtime::sql_required_capability(&statement.query);
                                require_vm_capability(
                                    function,
                                    grants,
                                    capability,
                                    "transactional SQL access",
                                    instruction.span,
                                )?;
                                let mut named_values = HashMap::new();
                                let mut parameter_types = HashMap::new();
                                for (name, local) in statement.parameters {
                                    let value = frame.locals.get(&local).cloned().ok_or_else(|| {
                                        runtime_error(
                                            instruction.span,
                                            format!("SQL transaction parameter `:{name}` is not available"),
                                        )
                                    })?;
                                    parameter_types.insert(name.clone(), value.ty());
                                    named_values.insert(name, value);
                                }
                                zelyra_database::sql::check_sql_query(
                                    &statement.query,
                                    &statement.result_type,
                                    schema,
                                    &parameter_types,
                                    instruction.span,
                                )
                                .map_err(|errors| {
                                    runtime_error(
                                        instruction.span,
                                        format!(
                                            "SQL transaction validation failed: {}",
                                            errors.first().map_or("invalid SQL query", |error| {
                                                error.message.as_str()
                                            })
                                        ),
                                    )
                                })?;
                                let params = zelyra_runtime::sql_parameters_from_named_values(
                                    &statement.query,
                                    &named_values,
                                    instruction.span,
                                )?;
                                queries.push(zelyra_database::Query {
                                    sql: statement.query,
                                    params,
                                });
                            }
                            BytecodeTransactionOperation::MailQueue {
                                arguments,
                                argument_spans,
                                call_span,
                            } => {
                                for expected in ["mail_queue_db_write", "mail_queue"] {
                                    let Some((
                                        preflight_function,
                                        preflight_name,
                                        preflight_arguments,
                                    )) = pending_host_calls.pop()
                                    else {
                                        return Err(runtime_error(
                                            call_span,
                                            "transactional mail_queue has no capability preflight",
                                        ));
                                    };
                                    if preflight_function != function_index
                                        || preflight_name != expected
                                        || preflight_arguments != 0
                                    {
                                        return Err(runtime_error(call_span, "transactional mail_queue preflight does not match its operation"));
                                    }
                                }
                                if !zelyra_runtime::supports_mail_outbox_database(database.url) {
                                    return Err(runtime_error(
                                        call_span,
                                        "mail_queue currently requires MariaDB or MySQL; PostgreSQL outbox support is not available",
                                    ));
                                }
                                let end = value_cursor.checked_add(arguments).ok_or_else(|| {
                                    runtime_error(
                                        call_span,
                                        "transactional mail_queue argument count overflows",
                                    )
                                })?;
                                let mail_values =
                                    values.get(value_cursor..end).ok_or_else(|| {
                                        runtime_error(
                                            call_span,
                                            "transactional mail_queue arguments are missing",
                                        )
                                    })?;
                                queries.push(zelyra_runtime::prepare_mail_queue_query(
                                    mail_values,
                                    &argument_spans,
                                    call_span,
                                    host_services.as_ref(),
                                )?);
                                value_cursor = end;
                            }
                        }
                    }
                    if value_cursor != values.len() {
                        return Err(runtime_error(
                            instruction.span,
                            "bytecode transaction has unused argument values",
                        ));
                    }
                    zelyra_runtime::validate_database_storage_with_host(
                        database.url,
                        database.schema,
                        host_services.as_ref(),
                        instruction.span,
                    )?;
                    host_services
                        .execute_database_batch(database.url, &queries, true)
                        .map_err(|error| {
                            runtime_error(
                                instruction.span,
                                format!("database transaction failed: {error}"),
                            )
                        })?;
                }
                BytecodeOp::Builtin {
                    name,
                    arguments,
                    type_arguments,
                    result_type,
                } => {
                    if bytecode_host_call_security(&name).is_some() {
                        let Some((preflight_function, preflight_name, preflight_arguments)) =
                            pending_host_calls.pop()
                        else {
                            return Err(runtime_error(
                                instruction.span,
                                format!("bytecode host builtin `{name}` has no preflight"),
                            ));
                        };
                        if preflight_function != function_index
                            || preflight_name != name
                            || preflight_arguments != arguments
                        {
                            return Err(runtime_error(
                                instruction.span,
                                format!(
                                    "bytecode host builtin `{name}` does not match its preflight"
                                ),
                            ));
                        }
                    }
                    let start = stack.len().checked_sub(arguments).ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode builtin stack underflow")
                    })?;
                    let values = stack.drain(start..).collect::<Vec<_>>();
                    let value = if name == "print" {
                        if arguments != 1 || result_type != Type::Unit {
                            return Err(runtime_error(
                                instruction.span,
                                "bytecode `print` builtin has invalid arity or result type",
                            ));
                        }
                        let value = values.into_iter().next().ok_or_else(|| {
                            runtime_error(instruction.span, "bytecode print argument is missing")
                        })?;
                        output.push(value.output());
                        Value::Unit
                    } else {
                        execute_vm_builtin(
                            &name,
                            values,
                            &type_arguments,
                            &result_type,
                            function,
                            BytecodeHostContext {
                                grants,
                                filesystem_policy: runtime_policy
                                    .and_then(|policy| policy.filesystem.as_ref()),
                                process_policy: runtime_policy
                                    .and_then(|policy| policy.process.as_ref()),
                                network_policy: runtime_policy
                                    .and_then(|policy| policy.network.as_ref()),
                                mail_policy: runtime_policy.and_then(|policy| policy.mail.as_ref()),
                                type_aliases: &program.type_aliases,
                                records: &program.records,
                                enums: &program.enums,
                                package_settings: runtime_policy
                                    .and_then(|policy| policy.package_settings.as_ref()),
                                host_services: host_services.as_ref(),
                            },
                            instruction.span,
                        )?
                    };
                    if !runtime_type_matches(&result_type, &value.ty()) {
                        return Err(runtime_error(
                            instruction.span,
                            format!(
                                "bytecode builtin `{name}` returned `{}`, expected `{result_type}`",
                                value.ty()
                            ),
                        ));
                    }
                    stack.push(value);
                }
                BytecodeOp::ContractCheck { postcondition } => {
                    let condition = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode contract check stack underflow")
                    })?;
                    let Value::Bool(condition) = condition else {
                        return Err(runtime_error(
                            instruction.span,
                            "bytecode function contract did not produce Bool",
                        ));
                    };
                    if !condition {
                        let contract_kind = if postcondition {
                            "postcondition"
                        } else {
                            "precondition"
                        };
                        return Err(runtime_error(
                            instruction.span,
                            format!("{contract_kind} failed for function `{}`", function.name),
                        ));
                    }
                }
                BytecodeOp::LoopInvariantCheck { phase } => {
                    let condition = stack.pop().ok_or_else(|| {
                        runtime_error(
                            instruction.span,
                            "bytecode loop invariant check stack underflow",
                        )
                    })?;
                    let Value::Bool(condition) = condition else {
                        return Err(runtime_error(
                            instruction.span,
                            "bytecode loop invariant did not produce Bool",
                        ));
                    };
                    if !condition {
                        let phase = match phase {
                            LoopInvariantPhase::Before => "before",
                            LoopInvariantPhase::After => "after",
                        };
                        return Err(runtime_error(
                            instruction.span,
                            format!("loop invariant failed {phase} loop iteration"),
                        ));
                    }
                }
                BytecodeOp::Call {
                    function: target,
                    arguments: count,
                } => {
                    if frames.len() >= limits.max_stack_depth {
                        return Err(runtime_error(
                            instruction.span,
                            "bytecode maximum call stack depth exceeded",
                        ));
                    }
                    let target_function = &program.functions[target];
                    if target_function
                        .declared_capabilities
                        .iter()
                        .any(|capability| !function.declared_capabilities.contains(capability))
                    {
                        return Err(runtime_error(
                            instruction.span,
                            format!(
                                "runtime capability denied: call from `{}` cannot escalate to `{}`",
                                function.name, target_function.name
                            ),
                        ));
                    }
                    let mut args = Vec::with_capacity(count);
                    for _ in 0..count {
                        args.push(stack.pop().ok_or_else(|| {
                            runtime_error(instruction.span, "bytecode stack underflow")
                        })?);
                    }
                    args.reverse();
                    let next = make_frame(target, target_function, args, stack.len())?;
                    frames.push(next);
                }
                BytecodeOp::CallValue { arguments: count } => {
                    let mut args = Vec::with_capacity(count);
                    for _ in 0..count {
                        args.push(stack.pop().ok_or_else(|| {
                            runtime_error(instruction.span, "bytecode stack underflow")
                        })?);
                    }
                    args.reverse();
                    let callback = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode function-value stack underflow")
                    })?;
                    let Value::Function(callback) = callback else {
                        return Err(runtime_error(
                            instruction.span,
                            "bytecode callback is not a function value",
                        ));
                    };
                    let target = callback.bytecode_target.ok_or_else(|| {
                        runtime_error(instruction.span, "function value has no bytecode target")
                    })?;
                    let target_function = program.functions.get(target).ok_or_else(|| {
                        runtime_error(
                            instruction.span,
                            "bytecode function-value target is invalid",
                        )
                    })?;
                    let expected_params = target_function
                        .parameters
                        .iter()
                        .map(|(_, ty, _)| ty)
                        .collect::<Vec<_>>();
                    if callback.params.iter().ne(expected_params)
                        || callback.return_type != target_function.return_type
                        || callback.capabilities != target_function.declared_capabilities
                        || count != target_function.parameters.len()
                        || callback.bytecode_captures.len() != target_function.captures.len()
                    {
                        return Err(runtime_error(
                            instruction.span,
                            "bytecode function value does not match its target signature",
                        ));
                    }
                    if frames.len() >= limits.max_stack_depth {
                        return Err(runtime_error(
                            instruction.span,
                            "bytecode maximum call stack depth exceeded",
                        ));
                    }
                    if target_function
                        .declared_capabilities
                        .iter()
                        .any(|capability| !function.declared_capabilities.contains(capability))
                    {
                        return Err(runtime_error(
                            instruction.span,
                            format!(
                                "runtime capability denied: call from `{}` cannot escalate to `{}`",
                                function.name, target_function.name
                            ),
                        ));
                    }
                    let mut next = make_frame(target, target_function, args, stack.len())?;
                    for ((local, expected, capture_span), (captured_local, value)) in
                        target_function
                            .captures
                            .iter()
                            .zip(&callback.bytecode_captures)
                    {
                        if local.0 != *captured_local
                            || !runtime_type_matches(expected, &value.ty())
                        {
                            return Err(runtime_error(
                                *capture_span,
                                "bytecode closure capture does not match the target layout or type",
                            ));
                        }
                        next.locals.insert(*local, value.clone());
                    }
                    frames.push(next);
                }
                BytecodeOp::Jump(target) => frames[frame_index].pc = target,
                BytecodeOp::JumpIfFalse(target) | BytecodeOp::JumpIfTrue(target) => {
                    let condition = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode branch stack underflow")
                    })?;
                    let Value::Bool(condition) = condition else {
                        return Err(runtime_error(
                            instruction.span,
                            "bytecode conditional branch expected Bool",
                        ));
                    };
                    let should_jump = match instruction.operation {
                        BytecodeOp::JumpIfFalse(_) => !condition,
                        BytecodeOp::JumpIfTrue(_) => condition,
                        _ => unreachable!(),
                    };
                    if should_jump {
                        frames[frame_index].pc = target;
                    }
                }
                BytecodeOp::RangeInit { iterator } => {
                    let step_value = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode stack underflow")
                    })?;
                    let end_value = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode stack underflow")
                    })?;
                    let start_value = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode stack underflow")
                    })?;
                    let step = match step_value {
                        Value::Int(step) => step,
                        other => {
                            return Err(runtime_error(
                                instruction.span,
                                format!("MIR integer range step must be Int, got `{}`", other.ty()),
                            ));
                        }
                    };
                    let (Value::Int(start), Value::Int(end)) = (start_value, end_value) else {
                        return Err(runtime_error(
                            instruction.span,
                            "MIR integer range requires Int start and end values",
                        ));
                    };
                    if step == 0 {
                        return Err(runtime_error(
                            instruction.span,
                            "MIR integer range step must not be zero",
                        ));
                    }
                    frame.iterators.insert(
                        iterator,
                        BytecodeIterator::Range(BytecodeRange {
                            current: Some(start),
                            end,
                            step,
                        }),
                    );
                }
                BytecodeOp::RangeNext {
                    iterator,
                    local,
                    body,
                    exit,
                } => {
                    let next = frame.iterators.get_mut(&iterator).ok_or_else(|| {
                        runtime_error(
                            instruction.span,
                            "bytecode range iterator is not initialized",
                        )
                    })?;
                    let BytecodeIterator::Range(range) = next else {
                        return Err(runtime_error(
                            instruction.span,
                            "bytecode range iterator was initialized as a collection",
                        ));
                    };
                    let next = range.next();
                    if let Some(value) = next {
                        frame.locals.insert(local, Value::Int(value));
                        frame.pc = body;
                    } else {
                        frame.pc = exit;
                    }
                }
                BytecodeOp::CollectionInit { iterator } => {
                    let value = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode stack underflow")
                    })?;
                    let values = match value {
                        Value::Array(values) | Value::Set(values) => values,
                        other => {
                            return Err(runtime_error(
                                instruction.span,
                                format!(
                                    "MIR collection iterator requires Array or Set, got `{}`",
                                    other.ty()
                                ),
                            ));
                        }
                    };
                    frame
                        .iterators
                        .insert(iterator, BytecodeIterator::Collection(values.into_iter()));
                }
                BytecodeOp::CollectionNext {
                    iterator,
                    locals,
                    body,
                    exit,
                } => {
                    let next = frame.iterators.get_mut(&iterator).ok_or_else(|| {
                        runtime_error(
                            instruction.span,
                            "bytecode collection iterator is not initialized",
                        )
                    })?;
                    let BytecodeIterator::Collection(values) = next else {
                        return Err(runtime_error(
                            instruction.span,
                            "bytecode collection iterator was initialized as an integer range",
                        ));
                    };
                    if let Some(value) = values.next() {
                        if locals.len() == 1 {
                            frame.locals.insert(locals[0], value);
                        } else {
                            let Value::Tuple(items) = value else {
                                return Err(runtime_error(
                                    instruction.span,
                                    "bytecode tuple-bound collection loop requires a Tuple value",
                                ));
                            };
                            if items.len() != locals.len() {
                                return Err(runtime_error(
                                instruction.span,
                                format!(
                                    "bytecode tuple-bound collection loop expects {} values, received {}",
                                    locals.len(),
                                    items.len()
                                ),
                            ));
                            }
                            frame.locals.extend(locals.into_iter().zip(items));
                        }
                        frame.pc = body;
                    } else {
                        frame.pc = exit;
                    }
                }
                BytecodeOp::MatchInit { match_id } => {
                    let value = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode match stack underflow")
                    })?;
                    frame.matches.insert(match_id, value);
                }
                BytecodeOp::MatchTest {
                    match_id,
                    pattern,
                    matched,
                    unmatched,
                } => {
                    let value = frame.matches.get(&match_id).cloned().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode match value is not initialized")
                    })?;
                    let mut bindings = HashMap::new();
                    if crate::execution::match_pattern(&pattern, &value, &mut bindings) {
                        frame.locals.extend(bindings);
                        frame.pc = matched;
                    } else {
                        frame.pc = unmatched;
                    }
                }
                BytecodeOp::MatchFailure => {
                    return Err(runtime_error(
                        instruction.span,
                        "non-exhaustive match reached at runtime",
                    ))
                }
                BytecodeOp::Return => {
                    let result = stack.pop().ok_or_else(|| {
                        runtime_error(instruction.span, "bytecode return stack underflow")
                    })?;
                    let return_type = &function.return_type;
                    if !runtime_type_matches(return_type, &result.ty()) {
                        return Err(runtime_error(
                            instruction.span,
                            format!(
                                "bytecode return expected `{return_type}` but returned `{}`",
                                result.ty()
                            ),
                        ));
                    }
                    let base = frame.stack_base;
                    frames.pop();
                    if frames.is_empty() {
                        return Ok(ExecutionResult {
                            value: result,
                            steps: consumed_bytecode_steps(
                                shared_budget_active,
                                &shared_steps,
                                local_steps,
                            ),
                        });
                    }
                    stack.truncate(base);
                    stack.push(result);
                }
                BytecodeOp::Trap => {
                    return Err(runtime_error(
                        instruction.span,
                        "bytecode reached an unreachable MIR block",
                    ));
                }
            }
            let frame = &frames[frames.len() - 1];
            let current_function = &program.functions[frame.function];
            if stack.len().saturating_sub(frame.stack_base) > current_function.max_stack {
                return Err(runtime_error(
                    instruction.span,
                    "bytecode operand stack exceeded its verified maximum",
                ));
            }
        }
    })();
    match execution {
        Ok(result) => ExecutionOutcome {
            output,
            value: Some(result.value),
            steps: consumed_bytecode_steps(shared_budget_active, &shared_steps, local_steps),
            error: None,
        },
        Err(error) => ExecutionOutcome {
            output,
            value: None,
            steps: consumed_bytecode_steps(shared_budget_active, &shared_steps, local_steps),
            error: Some(error),
        },
    }
}

struct BytecodeHostContext<'a> {
    grants: Option<&'a HashSet<String>>,
    filesystem_policy: Option<&'a FileSystemPolicy>,
    process_policy: Option<&'a ProcessPolicy>,
    network_policy: Option<&'a NetworkPolicy>,
    mail_policy: Option<&'a MailPolicy>,
    type_aliases: &'a [TypeDef],
    records: &'a [RecordDef],
    enums: &'a [EnumDef],
    package_settings: Option<&'a PackageSettings>,
    host_services: &'a dyn HostServices,
}

fn execute_vm_builtin(
    name: &str,
    arguments: Vec<Value>,
    type_arguments: &[Type],
    result_type: &Type,
    function: &BytecodeFunction,
    context: BytecodeHostContext<'_>,
    span: Span,
) -> Result<Value, RuntimeError> {
    let require_capability = |capability: &str, operation: &str| -> Result<(), RuntimeError> {
        if !function
            .declared_capabilities
            .iter()
            .any(|declared| declared == capability)
        {
            return Err(runtime_error(
                span,
                format!(
                    "runtime capability denied: {operation} requires `{capability}` in the current function"
                ),
            ));
        }
        if !context
            .grants
            .is_some_and(|grants| zelyra_runtime::project_grants_capability(grants, capability))
        {
            return Err(runtime_error(
                span,
                format!("runtime capability denied: `{capability}` is not granted"),
            ));
        }
        Ok(())
    };

    match name {
        "sql_decode" => {
            if arguments.len() != 1
                || type_arguments.len() != 1
                || result_type != &type_arguments[0]
            {
                return Err(runtime_error(
                    span,
                    "sql_decode expects one type argument and one SQL expression",
                ));
            }
            let aliases = context
                .type_aliases
                .iter()
                .map(|alias| (alias.name.clone(), alias.target.clone()))
                .collect::<HashMap<_, _>>();
            let records = context
                .records
                .iter()
                .map(|record| (record.name.clone(), record.clone()))
                .collect::<HashMap<_, _>>();
            let enums = context
                .enums
                .iter()
                .map(|definition| definition.name.clone())
                .collect::<HashSet<_>>();
            let rows = arguments
                .into_iter()
                .next()
                .expect("one argument was checked");
            zelyra_runtime::decode_native_sql_rows(
                rows,
                &type_arguments[0],
                &records,
                &aliases,
                &enums,
                span,
            )
        }
        "module_setting" => {
            if arguments.len() != 1
                || type_arguments.len() != 1
                || result_type != &type_arguments[0]
            {
                return Err(runtime_error(
                    span,
                    "module_setting expects one type argument and one setting name",
                ));
            }
            let Value::String(setting_name) = &arguments[0] else {
                return Err(runtime_error(
                    span,
                    "module_setting expects a String setting name",
                ));
            };
            context
                .package_settings
                .ok_or_else(|| runtime_error(span, "package settings are unavailable"))?
                .get_value(span.source_id, setting_name, &type_arguments[0])
                .map_err(|error| runtime_error(span, error))
        }
        "now" => {
            if !arguments.is_empty() || result_type != &Type::Timestamp {
                return Err(runtime_error(span, "now expects no arguments"));
            }
            require_capability("Clock", "clock access")?;
            let timestamp = context
                .host_services
                .unix_time_millis()
                .map_err(|error| runtime_error(span, format!("clock access failed: {error}")))?;
            if !zelyra_ast::temporal::valid_timestamp_millis(timestamp) {
                return Err(runtime_error(
                    span,
                    "clock access failed: timestamp is outside years 0001 through 9999",
                ));
            }
            Ok(Value::Timestamp(timestamp))
        }
        "env" => {
            if arguments.len() != 1 || result_type != &Type::Option(Box::new(Type::String)) {
                return Err(runtime_error(
                    span,
                    "env expects exactly one String variable name",
                ));
            }
            require_capability("Environment", "environment access")?;
            let Value::String(name) = &arguments[0] else {
                return Err(runtime_error(span, "env expects a String variable name"));
            };
            if name.is_empty() {
                return Err(runtime_error(span, "env expects a non-empty variable name"));
            }
            let value = context
                .host_services
                .environment_variable(name)
                .map_err(|error| runtime_error(span, error))?;
            Ok(Value::Option(
                value.map(|value| Box::new(Value::String(value))),
            ))
        }
        "read_console" => {
            if arguments.len() != 1 || result_type != &Type::Option(Box::new(Type::String)) {
                return Err(runtime_error(
                    span,
                    "read_console expects exactly one String prompt",
                ));
            }
            require_capability("Console", "console input")?;
            let Value::String(prompt) = &arguments[0] else {
                return Err(runtime_error(span, "read_console expects a String prompt"));
            };
            let value = context
                .host_services
                .read_console_line(prompt)
                .map_err(|error| runtime_error(span, format!("console input failed: {error}")))?;
            Ok(Value::Option(
                value.map(|value| Box::new(Value::String(value))),
            ))
        }
        "random_int" => {
            if arguments.len() != 2 || result_type != &Type::Int {
                return Err(runtime_error(
                    span,
                    "random_int expects minimum and maximum Int arguments",
                ));
            }
            require_capability("Random", "random access")?;
            let (Value::Int(minimum), Value::Int(maximum)) = (&arguments[0], &arguments[1]) else {
                return Err(runtime_error(span, "random_int expects Int arguments"));
            };
            random_int_with_host_services(context.host_services, *minimum, *maximum)
                .map(Value::Int)
                .map_err(|error| runtime_error(span, error))
        }
        "uuid_v4" => {
            if !arguments.is_empty() || result_type != &Type::String {
                return Err(runtime_error(span, "uuid_v4 expects no arguments"));
            }
            require_capability("Random", "random access")?;
            uuid_v4_with_host_services(context.host_services)
                .map(Value::String)
                .map_err(|error| runtime_error(span, format!("random access failed: {error}")))
        }
        "http_get" => {
            if arguments.len() != 1 || result_type != &Type::String {
                return Err(runtime_error(
                    span,
                    "http_get expects exactly one String URL",
                ));
            }
            require_capability("Network", "network access")?;
            let Value::String(url) = &arguments[0] else {
                return Err(runtime_error(span, "http_get expects a String URL"));
            };
            let response = execute_http_request_with_host(
                "GET",
                url,
                &[],
                None,
                context.network_policy,
                context.host_services,
            )
            .map_err(|error| runtime_error(span, error))?;
            if !(200..300).contains(&response.status) {
                return Err(runtime_error(
                    span,
                    format!("http_get received HTTP status {}", response.status),
                ));
            }
            String::from_utf8(response.body)
                .map(Value::String)
                .map_err(|_| runtime_error(span, "http_get received an invalid body"))
        }
        "http_request" => {
            if arguments.len() != 4 || result_type != &Type::Named("HttpResponse".into()) {
                return Err(runtime_error(
                    span,
                    "http_request expects String method, String URL, String[] headers, and String? body",
                ));
            }
            require_capability("Network", "network access")?;
            let Value::String(method) = &arguments[0] else {
                return Err(runtime_error(span, "http_request expects a String method"));
            };
            let Value::String(url) = &arguments[1] else {
                return Err(runtime_error(span, "http_request expects a String URL"));
            };
            let Value::Array(header_values) = &arguments[2] else {
                return Err(runtime_error(span, "http_request expects String[] headers"));
            };
            let headers = header_values
                .iter()
                .map(|header| match header {
                    Value::String(header) => Ok(header.clone()),
                    _ => Err(runtime_error(span, "http_request expects String[] headers")),
                })
                .collect::<Result<Vec<_>, _>>()?;
            let request_body = match &arguments[3] {
                Value::Option(None) => None,
                Value::Option(Some(value)) => match value.as_ref() {
                    Value::String(value) => Some(value.clone()),
                    _ => {
                        return Err(runtime_error(span, "http_request expects a String? body"));
                    }
                },
                _ => return Err(runtime_error(span, "http_request expects a String? body")),
            };
            let response = execute_http_request_with_host(
                method,
                url,
                &headers,
                request_body.as_deref(),
                context.network_policy,
                context.host_services,
            )
            .map_err(|error| runtime_error(span, error))?;
            http_response_value_from_host(response).map_err(|error| runtime_error(span, error))
        }
        "http_json" | "http_result" => {
            if arguments.len() != 4 || type_arguments.len() != 2 {
                return Err(runtime_error(
                    span,
                    format!("{name} expects request and response type arguments plus four values"),
                ));
            }
            let expected_type = if name == "http_json" {
                type_arguments[1].clone()
            } else {
                Type::HttpResult(Box::new(type_arguments[1].clone()))
            };
            if result_type != &expected_type {
                return Err(runtime_error(
                    span,
                    format!("{name} has an invalid result type"),
                ));
            }
            require_capability("Network", "network access")?;
            let Value::String(method) = &arguments[0] else {
                return Err(runtime_error(
                    span,
                    format!("{name} expects a String method"),
                ));
            };
            let Value::String(url) = &arguments[1] else {
                return Err(runtime_error(span, format!("{name} expects a String URL")));
            };
            let Value::Array(header_values) = &arguments[2] else {
                return Err(runtime_error(
                    span,
                    format!("{name} expects String[] headers"),
                ));
            };
            let mut headers = Vec::with_capacity(header_values.len() + 1);
            let mut has_content_type = false;
            for header in header_values {
                let Value::String(header) = header else {
                    return Err(runtime_error(
                        span,
                        format!("{name} expects String[] headers"),
                    ));
                };
                has_content_type |= header
                    .split_once(':')
                    .is_some_and(|(name, _)| name.trim().eq_ignore_ascii_case("content-type"));
                headers.push(header.clone());
            }
            if !has_content_type {
                headers.push("Content-Type: application/json".into());
            }
            let request_body = match &arguments[3] {
                Value::Option(None) => None,
                Value::Option(Some(value)) => Some(value.as_ref()),
                _ => {
                    return Err(runtime_error(
                        span,
                        format!("{name} expects a request Option"),
                    ))
                }
            };
            let encoded_body = request_body
                .map(|value| {
                    encode_json_value_with_definitions(
                        value,
                        context.records,
                        context.type_aliases,
                        context.enums,
                        span,
                    )
                })
                .transpose()?;
            let response = execute_http_request_with_host(
                method,
                url,
                &headers,
                encoded_body.as_deref(),
                context.network_policy,
                context.host_services,
            )
            .map_err(|error| runtime_error(span, error))?;
            let response = http_response_value_from_host(response)
                .map_err(|error| runtime_error(span, error))?;
            let Value::Object { fields, .. } = response else {
                return Err(runtime_error(
                    span,
                    format!("{name} received an invalid response"),
                ));
            };
            let status = match fields.get("status") {
                Some(Value::Int(status)) => *status,
                _ => {
                    return Err(runtime_error(
                        span,
                        format!("{name} received an invalid status"),
                    ))
                }
            };
            let response_headers = match fields.get("headers") {
                Some(Value::Array(headers)) => headers.clone(),
                _ => {
                    return Err(runtime_error(
                        span,
                        format!("{name} received invalid headers"),
                    ))
                }
            };
            let body = match fields.get("body") {
                Some(Value::String(body)) => body.clone(),
                _ => {
                    return Err(runtime_error(
                        span,
                        format!("{name} received an invalid body"),
                    ))
                }
            };
            let successful = (200..300).contains(&status);
            if name == "http_json" {
                if !successful {
                    return Err(runtime_error(
                        span,
                        format!("http_json received HTTP status {status}"),
                    ));
                }
                return decode_json_value(
                    &body,
                    &type_arguments[1],
                    context.records,
                    context.type_aliases,
                    context.enums,
                    span,
                );
            }
            let data = if successful {
                let decoded = if matches!(type_arguments[1], Type::Unit) && body.trim().is_empty() {
                    Value::Unit
                } else {
                    decode_json_value(
                        &body,
                        &type_arguments[1],
                        context.records,
                        context.type_aliases,
                        context.enums,
                        span,
                    )?
                };
                Value::Option(Some(Box::new(decoded)))
            } else {
                Value::Option(None)
            };
            let error = if successful {
                Value::Option(None)
            } else {
                Value::Option(Some(Box::new(Value::Object {
                    type_name: "HttpError".into(),
                    type_args: Vec::new(),
                    fields: HashMap::from([
                        ("status".into(), Value::Int(status)),
                        ("headers".into(), Value::Array(response_headers.clone())),
                        ("body".into(), Value::String(body.clone())),
                        (
                            "message".into(),
                            Value::String(format!("HTTP status {status}")),
                        ),
                    ]),
                })))
            };
            Ok(Value::Object {
                type_name: "HttpResult".into(),
                type_args: Vec::new(),
                fields: HashMap::from([
                    ("status".into(), Value::Int(status)),
                    ("headers".into(), Value::Array(response_headers)),
                    ("body".into(), Value::String(body)),
                    ("data".into(), data),
                    ("error".into(), error),
                ]),
            })
        }
        "mail_deliver" => {
            let expected_type = Type::Result(Box::new(Type::Unit), Box::new(Type::String));
            if arguments.len() != 2 || result_type != &expected_type {
                return Err(runtime_error(
                    span,
                    "mail_deliver expects an event ID and payload JSON and returns Result<Unit, String>",
                ));
            }
            require_capability("Email", "email delivery")?;
            let (Value::String(event_id), Value::String(payload_json)) =
                (&arguments[0], &arguments[1])
            else {
                return Err(runtime_error(span, "mail_deliver expects String arguments"));
            };
            let delivery = deliver_outbox_message_with_host(
                event_id,
                payload_json,
                context.mail_policy,
                context.network_policy,
                context.host_services,
            );
            Ok(Value::Result(
                delivery
                    .map(|()| Box::new(Value::Unit))
                    .map_err(|error| Box::new(Value::String(error))),
            ))
        }
        "run_process" => {
            if arguments.len() != 2 || result_type != &Type::String {
                return Err(runtime_error(
                    span,
                    "run_process expects a String command and String[] arguments",
                ));
            }
            require_capability("Process", "process execution")?;
            let Value::String(command) = &arguments[0] else {
                return Err(runtime_error(span, "run_process expects a String command"));
            };
            let Value::Array(arguments) = &arguments[1] else {
                return Err(runtime_error(
                    span,
                    "run_process expects String[] arguments",
                ));
            };
            let arguments = arguments
                .iter()
                .map(|argument| match argument {
                    Value::String(argument) => Ok(argument.clone()),
                    _ => Err(runtime_error(
                        span,
                        "run_process expects String[] arguments",
                    )),
                })
                .collect::<Result<Vec<_>, _>>()?;
            run_process_with_host_services(
                command,
                &arguments,
                context.process_policy,
                context.host_services,
            )
            .map(Value::String)
            .map_err(|error| runtime_error(span, error))
        }
        "file_exists" | "dir_exists" => {
            if arguments.len() != 1 || result_type != &Type::Bool {
                return Err(runtime_error(
                    span,
                    format!("{name} expects one String path"),
                ));
            }
            require_capability("FileSystem", "file-system read access")?;
            let Value::String(path) = &arguments[0] else {
                return Err(runtime_error(span, format!("{name} expects a String path")));
            };
            let exists = inspect_filesystem_path_with_host(
                context.filesystem_policy,
                path,
                false,
                true,
                context.host_services,
            )
            .ok()
            .and_then(|resolved| context.host_services.file_metadata(&resolved).ok())
            .is_some_and(|metadata| {
                if name == "file_exists" {
                    metadata.is_file
                } else {
                    metadata.is_directory
                }
            });
            Ok(Value::Bool(exists))
        }
        "file_size" => {
            let expected_type = Type::Result(Box::new(Type::Int), Box::new(Type::String));
            if arguments.len() != 1 || result_type != &expected_type {
                return Err(runtime_error(span, "file_size expects one String path"));
            }
            require_capability("FileSystem", "file-system read access")?;
            let Value::String(path) = &arguments[0] else {
                return Err(runtime_error(span, "file_size expects a String path"));
            };
            let resolved = inspect_filesystem_path_with_host(
                context.filesystem_policy,
                path,
                false,
                false,
                context.host_services,
            )
            .map_err(|error| runtime_error(span, error))?;
            let outcome = context
                .host_services
                .file_metadata(&resolved)
                .map(|metadata| Box::new(Value::Int(metadata.length as i64)))
                .map_err(|error| Box::new(Value::String(error)));
            Ok(Value::Result(outcome))
        }
        "list_dir" => {
            if arguments.len() != 1 || result_type != &Type::Array(Box::new(Type::String)) {
                return Err(runtime_error(span, "list_dir expects one String path"));
            }
            require_capability("FileSystem", "file-system read access")?;
            let Value::String(path) = &arguments[0] else {
                return Err(runtime_error(span, "list_dir expects a String path"));
            };
            if path.is_empty() {
                return Err(runtime_error(span, "list_dir expects a non-empty path"));
            }
            let resolved = inspect_filesystem_directory_with_host(
                context.filesystem_policy,
                path,
                context.host_services,
            )
            .map_err(|error| runtime_error(span, error))?;
            let entries = context
                .host_services
                .list_directory(&resolved)
                .map_err(|error| {
                    runtime_error(span, format!("file-system list failed: {error}"))
                })?;
            Ok(Value::Array(
                entries.into_iter().map(Value::String).collect(),
            ))
        }
        "write_text"
        | "write_text_result"
        | "write_text_new_result"
        | "atomic_write_text_result"
        | "append_text_result" => {
            let result_call = name != "write_text";
            let expected_type = if result_call {
                Type::Result(Box::new(Type::Unit), Box::new(Type::String))
            } else {
                Type::Unit
            };
            if arguments.len() != 2 || result_type != &expected_type {
                return Err(runtime_error(
                    span,
                    format!("{name} expects a String path and String content"),
                ));
            }
            require_capability("FileSystem", "file-system write access")?;
            let (Value::String(path), Value::String(content)) = (&arguments[0], &arguments[1])
            else {
                return Err(runtime_error(
                    span,
                    format!("{name} expects String arguments"),
                ));
            };
            let result = if path.is_empty() {
                Err(format!("{name} expects a non-empty path"))
            } else {
                match inspect_filesystem_path_with_host(
                    context.filesystem_policy,
                    path,
                    true,
                    true,
                    context.host_services,
                ) {
                    Err(error) => Err(error),
                    Ok(resolved) => {
                        let result = match name {
                            "write_text" | "write_text_result" => {
                                context.host_services.write_text(&resolved, content)
                            }
                            "write_text_new_result" => {
                                context.host_services.write_text_new(&resolved, content)
                            }
                            "atomic_write_text_result" => {
                                context.host_services.atomic_write_text(&resolved, content)
                            }
                            "append_text_result" => {
                                context.host_services.append_text(&resolved, content)
                            }
                            _ => unreachable!("guard lists supported text writes"),
                        };
                        result.map_err(|error| {
                            if name == "write_text" || name == "write_text_result" {
                                format!("file-system write failed: {error}")
                            } else {
                                format!("file-system {name} failed: {error}")
                            }
                        })
                    }
                }
            };
            if result_call {
                Ok(Value::Result(
                    result
                        .map(|()| Box::new(Value::Unit))
                        .map_err(|error| Box::new(Value::String(error))),
                ))
            } else {
                result
                    .map(|()| Value::Unit)
                    .map_err(|error| runtime_error(span, error))
            }
        }
        "delete_file" | "delete_file_result" => {
            let result_call = name == "delete_file_result";
            let expected_type = if result_call {
                Type::Result(Box::new(Type::Unit), Box::new(Type::String))
            } else {
                Type::Unit
            };
            if arguments.len() != 1 || result_type != &expected_type {
                return Err(runtime_error(
                    span,
                    format!("{name} expects one String path"),
                ));
            }
            require_capability("FileSystem", "file-system delete access")?;
            let Value::String(path) = &arguments[0] else {
                return Err(runtime_error(span, format!("{name} expects a String path")));
            };
            let result = if path.is_empty() {
                Err(format!("{name} expects a non-empty path"))
            } else {
                inspect_filesystem_delete_path_with_host(
                    context.filesystem_policy,
                    path,
                    context.host_services,
                )
                .and_then(|resolved| {
                    context
                        .host_services
                        .remove_file(&resolved)
                        .map_err(|error| format!("file-system delete failed: {error}"))
                })
            };
            if result_call {
                Ok(Value::Result(
                    result
                        .map(|()| Box::new(Value::Unit))
                        .map_err(|error| Box::new(Value::String(error))),
                ))
            } else {
                result
                    .map(|()| Value::Unit)
                    .map_err(|error| runtime_error(span, error))
            }
        }
        "copy_file_new_result" | "rename_file_result" => {
            let expected_type = Type::Result(Box::new(Type::Unit), Box::new(Type::String));
            if arguments.len() != 2 || result_type != &expected_type {
                return Err(runtime_error(
                    span,
                    format!("{name} expects source and destination String paths"),
                ));
            }
            require_capability("FileSystem", "file-system write access")?;
            let (Value::String(source), Value::String(destination)) =
                (&arguments[0], &arguments[1])
            else {
                return Err(runtime_error(span, format!("{name} expects String paths")));
            };
            let result = if source.is_empty() || destination.is_empty() {
                Err(format!("{name} expects non-empty paths"))
            } else {
                let source_path = if name == "copy_file_new_result" {
                    inspect_filesystem_copy_source_with_host(
                        context.filesystem_policy,
                        source,
                        context.host_services,
                    )
                } else {
                    inspect_filesystem_rename_source_with_host(
                        context.filesystem_policy,
                        source,
                        context.host_services,
                    )
                };
                source_path.and_then(|source_path| {
                    inspect_filesystem_copy_or_rename_destination_with_host(
                        context.filesystem_policy,
                        destination,
                        context.host_services,
                    )
                    .and_then(|destination_path| {
                        let operation = if name == "copy_file_new_result" {
                            context
                                .host_services
                                .copy_file_new(&source_path, &destination_path)
                        } else {
                            context
                                .host_services
                                .rename_file_new(&source_path, &destination_path)
                        };
                        operation.map_err(|error| format!("file-system {name} failed: {error}"))
                    })
                })
            };
            Ok(Value::Result(
                result
                    .map(|()| Box::new(Value::Unit))
                    .map_err(|error| Box::new(Value::String(error))),
            ))
        }
        "read_text" | "read_text_result" => {
            let result_call = name == "read_text_result";
            let expected_type = if result_call {
                Type::Result(Box::new(Type::String), Box::new(Type::String))
            } else {
                Type::String
            };
            if arguments.len() != 1 || result_type != &expected_type {
                return Err(runtime_error(
                    span,
                    format!("{name} expects exactly one String path"),
                ));
            }
            require_capability("FileSystem", "file-system read access")?;
            let Value::String(path) = &arguments[0] else {
                return Err(runtime_error(span, format!("{name} expects a String path")));
            };
            let result = if path.is_empty() {
                Err(format!("{name} expects a non-empty path"))
            } else {
                match inspect_filesystem_path_with_host(
                    context.filesystem_policy,
                    path,
                    false,
                    false,
                    context.host_services,
                ) {
                    Err(error) => Err(error),
                    Ok(resolved) => context
                        .host_services
                        .read_text(&resolved)
                        .map_err(|error| format!("file-system read failed: {error}")),
                }
            };
            if result_call {
                Ok(Value::Result(
                    result
                        .map(|text| Box::new(Value::String(text)))
                        .map_err(|error| Box::new(Value::String(error))),
                ))
            } else {
                result
                    .map(Value::String)
                    .map_err(|error| runtime_error(span, error))
            }
        }
        _ => execute_pure_builtin(name, arguments, span),
    }
}

struct Frame {
    function: usize,
    pc: usize,
    locals: HashMap<LocalId, Value>,
    iterators: HashMap<IteratorId, BytecodeIterator>,
    matches: HashMap<MatchId, Value>,
    stack_base: usize,
}

enum BytecodeIterator {
    Range(BytecodeRange),
    Collection(std::vec::IntoIter<Value>),
}

struct BytecodeRange {
    current: Option<i64>,
    end: i64,
    step: i64,
}

impl BytecodeRange {
    fn next(&mut self) -> Option<i64> {
        let value = self.current.take()?;
        let before_end = |value: i64| {
            if self.step > 0 {
                value < self.end
            } else {
                value > self.end
            }
        };
        if !before_end(value) {
            return None;
        }
        self.current = value
            .checked_add(self.step)
            .filter(|next| before_end(*next));
        Some(value)
    }
}

fn make_frame(
    function_index: usize,
    function: &BytecodeFunction,
    arguments: Vec<Value>,
    stack_base: usize,
) -> Result<Frame, RuntimeError> {
    if arguments.len() != function.parameters.len() {
        return Err(runtime_error(
            function.code[0].span,
            format!(
                "bytecode function `{}` expects {} arguments but received {}",
                function.name,
                function.parameters.len(),
                arguments.len()
            ),
        ));
    }
    let mut locals = HashMap::new();
    for ((local, expected, span), value) in function.parameters.iter().zip(arguments) {
        if !runtime_type_matches(expected, &value.ty()) {
            return Err(runtime_error(
                *span,
                format!(
                    "bytecode parameter expected `{expected}` but received `{}`",
                    value.ty()
                ),
            ));
        }
        locals.insert(*local, value);
    }
    Ok(Frame {
        function: function_index,
        pc: 0,
        locals,
        iterators: HashMap::new(),
        matches: HashMap::new(),
        stack_base,
    })
}

fn function_return_type(program: &MirProgram, function: &crate::MirFunction) -> Type {
    function
        .declared_return_type
        .clone()
        .or_else(|| {
            function
                .blocks
                .iter()
                .find_map(|block| match block.terminator {
                    Some(MirTerminator::Return(Some(value))) => program
                        .expressions
                        .get(value.0)
                        .map(|expression| expression.ty.clone()),
                    Some(MirTerminator::Return(None)) => Some(Type::Unit),
                    _ => None,
                })
        })
        .unwrap_or(Type::Unit)
}

fn named_record_type(name: &str, args: &[Type]) -> Type {
    if args.is_empty() {
        Type::Named(name.to_owned())
    } else {
        Type::Applied {
            name: name.to_owned(),
            args: args.to_vec(),
        }
    }
}

fn record_type_parts(ty: &Type) -> Option<(&str, &[Type])> {
    match ty {
        Type::Named(name) => Some((name, &[])),
        Type::Applied { name, args } => Some((name, args)),
        _ => None,
    }
}

fn is_pure_mail_mir_expression(program: &MirProgram, id: ExpressionId) -> bool {
    is_pure_mail_mir_expression_inner(program, id, &mut HashSet::new())
}

fn is_pure_mail_mir_expression_inner(
    program: &MirProgram,
    id: ExpressionId,
    active: &mut HashSet<ExpressionId>,
) -> bool {
    if !active.insert(id) {
        return false;
    }
    let Some(expression) = program.expressions.get(id.0) else {
        active.remove(&id);
        return false;
    };
    let pure = match &expression.kind {
        MirExpressionKind::Int(_)
        | MirExpressionKind::UInt(_)
        | MirExpressionKind::Float(_)
        | MirExpressionKind::Bool(_)
        | MirExpressionKind::String(_)
        | MirExpressionKind::Char(_)
        | MirExpressionKind::Local(_) => true,
        MirExpressionKind::Array(values) => values
            .iter()
            .all(|value| is_pure_mail_mir_expression_inner(program, *value, active)),
        MirExpressionKind::Map(entries) => entries.iter().all(|(key, value)| {
            is_pure_mail_mir_expression_inner(program, *key, active)
                && is_pure_mail_mir_expression_inner(program, *value, active)
        }),
        MirExpressionKind::Record { fields, .. } => fields
            .iter()
            .all(|(_, value)| is_pure_mail_mir_expression_inner(program, *value, active)),
        MirExpressionKind::Call {
            name,
            function: None,
            args,
            ..
        } if name == "render_pdf" => {
            args.len() == 2
                && matches!(
                    program.expressions.get(args[0].0).map(|item| &item.kind),
                    Some(MirExpressionKind::String(_))
                )
                && is_pure_mail_mir_expression_inner(program, args[1], active)
        }
        MirExpressionKind::Field { target, .. } => {
            is_pure_mail_mir_expression_inner(program, *target, active)
        }
        MirExpressionKind::Binary {
            left,
            op: BinaryOp::Add,
            right,
        } => {
            is_pure_mail_mir_expression_inner(program, *left, active)
                && is_pure_mail_mir_expression_inner(program, *right, active)
        }
        _ => false,
    };
    active.remove(&id);
    pure
}

pub(super) fn substitute_record_type(ty: &Type, substitutions: &HashMap<String, Type>) -> Type {
    match ty {
        Type::Named(name) => substitutions
            .get(name)
            .cloned()
            .unwrap_or_else(|| ty.clone()),
        Type::Option(inner) => Type::Option(Box::new(substitute_record_type(inner, substitutions))),
        Type::Result(ok, err) => Type::Result(
            Box::new(substitute_record_type(ok, substitutions)),
            Box::new(substitute_record_type(err, substitutions)),
        ),
        Type::Array(inner) => Type::Array(Box::new(substitute_record_type(inner, substitutions))),
        Type::Tuple(items) => Type::Tuple(
            items
                .iter()
                .map(|item| substitute_record_type(item, substitutions))
                .collect(),
        ),
        Type::Set(inner) => Type::Set(Box::new(substitute_record_type(inner, substitutions))),
        Type::Map(key, value) => Type::Map(
            Box::new(substitute_record_type(key, substitutions)),
            Box::new(substitute_record_type(value, substitutions)),
        ),
        Type::HttpResult(inner) => {
            Type::HttpResult(Box::new(substitute_record_type(inner, substitutions)))
        }
        Type::Applied { name, args } => Type::Applied {
            name: name.clone(),
            args: args
                .iter()
                .map(|item| substitute_record_type(item, substitutions))
                .collect(),
        },
        Type::Function {
            params,
            return_type,
            capabilities,
        } => Type::Function {
            params: params
                .iter()
                .map(|item| substitute_record_type(item, substitutions))
                .collect(),
            return_type: Box::new(substitute_record_type(return_type, substitutions)),
            capabilities: capabilities.clone(),
        },
        _ => ty.clone(),
    }
}

fn called_functions(program: &MirProgram, function: &crate::MirFunction) -> Vec<FunctionId> {
    let mut roots = Vec::new();
    roots.extend(function.requires.iter().copied());
    roots.extend(function.ensures.iter().copied());
    for block in &function.blocks {
        roots.extend(block.loop_invariants.iter().copied());
        for instruction in &block.instructions {
            match instruction {
                MirInstruction::SourceStep { .. } => {}
                MirInstruction::Let { value, .. }
                | MirInstruction::Assign { value, .. }
                | MirInstruction::Evaluate(value)
                | MirInstruction::MatchInit { value, .. }
                | MirInstruction::TupleBind { value, .. } => roots.push(*value),
                MirInstruction::ArrayIndexAssign { index, value, .. } => {
                    roots.push(*index);
                    roots.push(*value);
                }
                MirInstruction::IteratorInit { source, .. } => match source {
                    crate::MirIteratorSource::Collection(value) => roots.push(*value),
                    crate::MirIteratorSource::IntegerRange { start, end, step } => {
                        roots.extend([*start, *end]);
                        roots.extend(step.iter().copied());
                    }
                },
                MirInstruction::Transaction { operations, .. } => roots.extend(operations),
                MirInstruction::Parallel { bindings, .. } => {
                    roots.extend(bindings.iter().map(|binding| binding.value));
                }
                MirInstruction::LoopInvariantCheck { expressions, .. } => {
                    roots.extend(expressions.iter().copied());
                }
            }
        }
        match &block.terminator {
            Some(MirTerminator::Branch { condition, .. }) => roots.push(*condition),
            Some(MirTerminator::Return(Some(value))) => roots.push(*value),
            Some(MirTerminator::ReturnAfterLoopInvariants { value, invariants }) => {
                roots.extend(value.iter().copied());
                roots.extend(invariants.iter().map(|invariant| invariant.expression));
            }
            Some(MirTerminator::IteratorNext { .. })
            | Some(MirTerminator::MatchTest { .. })
            | Some(MirTerminator::Goto(_))
            | Some(MirTerminator::MatchFailure { .. })
            | Some(MirTerminator::Return(None))
            | Some(MirTerminator::Unreachable)
            | None => {}
        }
    }
    let mut calls = HashSet::new();
    let mut visited = HashSet::new();
    while let Some(id) = roots.pop() {
        if !visited.insert(id) {
            continue;
        }
        let Some(expression) = program.expressions.get(id.0) else {
            continue;
        };
        use MirExpressionKind as K;
        match &expression.kind {
            K::Call {
                function: Some(function),
                args,
                ..
            } => {
                calls.insert(*function);
                roots.extend(args);
            }
            K::FunctionValue {
                function: Some(function),
                ..
            } => {
                calls.insert(*function);
            }
            K::Array(items) | K::Call { args: items, .. } => roots.extend(items),
            K::Map(items) => {
                for (key, value) in items {
                    roots.extend([*key, *value]);
                }
            }
            K::Record { fields, .. } => roots.extend(fields.iter().map(|(_, value)| *value)),
            K::Lambda { body, .. } | K::Await(body) => roots.push(*body),
            K::Index { target, index } => roots.extend([*target, *index]),
            K::Field { target, .. } => roots.push(*target),
            K::Unary { expr, .. } => roots.push(*expr),
            K::Binary { left, right, .. } => roots.extend([*left, *right]),
            K::BinaryChain { first, operations } => {
                roots.push(*first);
                roots.extend(operations.iter().map(|(_, value, _)| *value));
            }
            K::Match { value, arms } => {
                roots.push(*value);
                for arm in arms {
                    if let Some(guard) = arm.guard {
                        roots.push(guard);
                    }
                    roots.push(arm.value);
                }
            }
            K::Int(_)
            | K::UInt(_)
            | K::Float(_)
            | K::Bool(_)
            | K::String(_)
            | K::Char(_)
            | K::Local(_)
            | K::EnumVariant { .. }
            | K::FunctionValue { .. }
            | K::Sql { .. }
            | K::Uninitialized => {}
        }
    }
    calls.into_iter().collect()
}

fn bytecode_error(message: impl Into<String>, span: Span) -> MirError {
    MirError {
        code: "E-MIR-BYTECODE-001",
        message: message.into(),
        span,
    }
}

fn mir_expression_name(expression: &MirExpressionKind) -> &'static str {
    match expression {
        MirExpressionKind::Uninitialized => "uninitialized",
        MirExpressionKind::Int(_) => "Int literal",
        MirExpressionKind::UInt(_) => "UInt literal",
        MirExpressionKind::Float(_) => "Float literal",
        MirExpressionKind::Bool(_) => "Bool literal",
        MirExpressionKind::String(_) => "String literal",
        MirExpressionKind::Char(_) => "Char literal",
        MirExpressionKind::Array(_) => "array",
        MirExpressionKind::Map(_) => "map",
        MirExpressionKind::Record { .. } => "record",
        MirExpressionKind::Lambda { .. } => "lambda",
        MirExpressionKind::Local(_) => "local",
        MirExpressionKind::EnumVariant { .. } => "enum variant",
        MirExpressionKind::FunctionValue { .. } => "function value",
        MirExpressionKind::Index { .. } => "index",
        MirExpressionKind::Field { .. } => "field",
        MirExpressionKind::Call { .. } => "call",
        MirExpressionKind::Unary { .. } => "unary",
        MirExpressionKind::Binary { .. } => "binary",
        MirExpressionKind::BinaryChain { .. } => "binary chain",
        MirExpressionKind::Sql { .. } => "SQL",
        MirExpressionKind::Await(_) => "await",
        MirExpressionKind::Match { .. } => "match expression",
    }
}

#[cfg(test)]
mod tests;
