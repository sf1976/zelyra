//! Deterministic, bounded binary format for the experimental bytecode slice.

use super::{
    verify_bytecode, BytecodeFunction, BytecodeInstruction, BytecodeOp, BytecodeParallelBinding,
    BytecodeProgram, BytecodeSqlStatement, BytecodeTransactionOperation, BYTECODE_VERSION,
};
use crate::{IteratorId, MirError};
use zelyra_ast::{
    BinaryOp, ComponentProp, EnumDef, RecordDef, RecordField, Span, Type, TypeDef, UnaryOp, ViewDef,
};
use zelyra_hir::{FunctionId, LocalId};
use zelyra_runtime::Value;

const MAGIC: &[u8; 8] = b"ZELYBC\0\0";
type TypeDefinitions = (
    Vec<TypeDef>,
    Vec<RecordDef>,
    Vec<EnumDef>,
    Vec<ViewDef>,
    Vec<String>,
);

#[derive(Clone, Copy, Debug)]
pub struct BytecodeFormatLimits {
    pub max_bytes: usize,
    pub max_functions: usize,
    pub max_instructions: usize,
    pub max_string_bytes: usize,
    pub max_type_depth: usize,
    pub max_type_nodes: usize,
}

impl Default for BytecodeFormatLimits {
    fn default() -> Self {
        Self {
            max_bytes: 16 * 1024 * 1024,
            max_functions: 10_000,
            max_instructions: 1_000_000,
            max_string_bytes: 1024 * 1024,
            max_type_depth: 64,
            max_type_nodes: 100_000,
        }
    }
}

pub fn encode(program: &BytecodeProgram) -> Result<Vec<u8>, Vec<MirError>> {
    encode_with_limits(program, BytecodeFormatLimits::default())
}

pub fn encode_with_limits(
    program: &BytecodeProgram,
    limits: BytecodeFormatLimits,
) -> Result<Vec<u8>, Vec<MirError>> {
    verify_bytecode(program).map_err(|mut e| {
        for x in &mut e {
            x.code = "E-MIR-BYTECODE-002";
        }
        e
    })?;
    let total_instructions = program
        .functions
        .iter()
        .try_fold(0usize, |total, f| total.checked_add(f.code.len()));
    if program.functions.len() > limits.max_functions
        || total_instructions.is_none_or(|n| n > limits.max_instructions)
    {
        return Err(error(
            "bytecode program exceeds configured function or instruction limit",
        ));
    }
    let mut w = Writer {
        bytes: Vec::new(),
        limits,
        type_nodes: 0,
    };
    w.raw(MAGIC)?;
    w.u16(program.version)?;
    w.usize(program.entry)?;
    w.definitions(
        &program.type_aliases,
        &program.records,
        &program.enums,
        &program.views,
        &program.scoped_tables,
    )?;
    w.usize(program.functions.len())?;
    for function in &program.functions {
        w.usize(function.id.0)?;
        w.string(&function.name)?;
        w.usize(function.declared_capabilities.len())?;
        for capability in &function.declared_capabilities {
            w.string(capability)?;
        }
        w.usize(function.parameters.len())?;
        for (id, ty, span) in &function.parameters {
            w.usize(id.0)?;
            w.ty(ty, 0)?;
            w.span(*span)?;
        }
        w.usize(function.captures.len())?;
        for (id, ty, span) in &function.captures {
            w.usize(id.0)?;
            w.ty(ty, 0)?;
            w.span(*span)?;
        }
        w.usize(function.local_mutability.len())?;
        for (local, mutable) in &function.local_mutability {
            w.usize(local.0)?;
            w.u8(u8::from(*mutable))?;
        }
        w.usize(function.initialization_writes.len())?;
        for (instruction, local) in &function.initialization_writes {
            w.usize(*instruction)?;
            w.usize(local.0)?;
        }
        w.ty(&function.return_type, 0)?;
        w.usize(function.max_stack)?;
        w.usize(function.code.len())?;
        for ins in &function.code {
            w.span(ins.span)?;
            if let Some(span) = ins.source_step_span {
                w.u8(1)?;
                w.span(span)?;
            } else {
                w.u8(0)?;
            }
            w.op(&ins.operation)?;
        }
    }
    Ok(w.bytes)
}

pub fn decode(bytes: &[u8]) -> Result<BytecodeProgram, Vec<MirError>> {
    decode_with_limits(bytes, BytecodeFormatLimits::default())
}

pub fn decode_with_limits(
    bytes: &[u8],
    limits: BytecodeFormatLimits,
) -> Result<BytecodeProgram, Vec<MirError>> {
    if bytes.len() > limits.max_bytes {
        return Err(error("bytecode artifact exceeds configured byte limit"));
    }
    let mut r = Reader {
        bytes,
        pos: 0,
        limits,
        type_nodes: 0,
        instructions: 0,
    };
    if r.raw(8)? != MAGIC {
        return Err(error("invalid bytecode magic"));
    }
    let version = r.u16()?;
    if version != BYTECODE_VERSION {
        return Err(error(format!("unsupported bytecode version {version}")));
    }
    let entry = r.usize()?;
    let (type_aliases, records, enums, views, scoped_tables) = r.definitions()?;
    let count = r.count(limits.max_functions, "function")?;
    let mut functions = Vec::with_capacity(count);
    for _ in 0..count {
        let id = FunctionId(r.usize()?);
        let name = r.string()?;
        let capability_count = r.count(64, "function capability")?;
        let mut declared_capabilities = Vec::with_capacity(capability_count);
        for _ in 0..capability_count {
            declared_capabilities.push(r.string()?);
        }
        let nparams = r.count(limits.max_instructions, "parameter")?;
        // The count is attacker-controlled. Grow only as records are parsed so
        // a tiny truncated artifact cannot force a large eager allocation.
        let mut parameters = Vec::new();
        for _ in 0..nparams {
            parameters.push((LocalId(r.usize()?), r.ty(0)?, r.span()?));
        }
        let capture_count = r.count(limits.max_instructions, "closure capture")?;
        let mut captures = Vec::new();
        for _ in 0..capture_count {
            captures.push((LocalId(r.usize()?), r.ty(0)?, r.span()?));
        }
        let local_count = r.count(limits.max_instructions, "local mutability")?;
        let mut local_mutability = Vec::new();
        for _ in 0..local_count {
            let local = LocalId(r.usize()?);
            let mutable = match r.u8()? {
                0 => false,
                1 => true,
                _ => return Err(error("invalid local mutability flag in bytecode")),
            };
            local_mutability.push((local, mutable));
        }
        let initialization_count = r.count(limits.max_instructions, "local initialization")?;
        let mut initialization_writes = Vec::new();
        for _ in 0..initialization_count {
            initialization_writes.push((r.usize()?, LocalId(r.usize()?)));
        }
        let return_type = r.ty(0)?;
        let max_stack = r.usize()?;
        let ncode = r.count(
            limits.max_instructions.saturating_sub(r.instructions),
            "instruction",
        )?;
        r.instructions = r.instructions.saturating_add(ncode);
        let mut code = Vec::new();
        for _ in 0..ncode {
            let span = r.span()?;
            let source_step_span = match r.u8()? {
                0 => None,
                1 => Some(r.span()?),
                _ => return Err(error("invalid source-step span flag in bytecode")),
            };
            let operation = r.op()?;
            code.push(BytecodeInstruction {
                operation,
                span,
                source_step_span,
            });
        }
        functions.push(BytecodeFunction {
            id,
            name,
            parameters,
            captures,
            local_mutability,
            initialization_writes,
            return_type,
            declared_capabilities,
            code,
            max_stack,
        });
    }
    if r.pos != bytes.len() {
        return Err(error("trailing bytes after bytecode artifact"));
    }
    let program = BytecodeProgram {
        version,
        functions,
        entry,
        type_aliases,
        records,
        enums,
        views,
        scoped_tables,
    };
    verify_bytecode(&program).map_err(|mut e| {
        for x in &mut e {
            x.code = "E-MIR-BYTECODE-002";
        }
        e
    })?;
    Ok(program)
}

struct Writer {
    bytes: Vec<u8>,
    limits: BytecodeFormatLimits,
    type_nodes: usize,
}
impl Writer {
    fn raw(&mut self, b: &[u8]) -> Result<(), Vec<MirError>> {
        if self.bytes.len().saturating_add(b.len()) > self.limits.max_bytes {
            return Err(error("encoded bytecode exceeds configured byte limit"));
        }
        self.bytes.extend_from_slice(b);
        Ok(())
    }
    fn u8(&mut self, x: u8) -> Result<(), Vec<MirError>> {
        self.raw(&[x])
    }
    fn u16(&mut self, x: u16) -> Result<(), Vec<MirError>> {
        self.raw(&x.to_le_bytes())
    }
    fn u32(&mut self, x: u32) -> Result<(), Vec<MirError>> {
        self.raw(&x.to_le_bytes())
    }
    fn u64(&mut self, x: u64) -> Result<(), Vec<MirError>> {
        self.raw(&x.to_le_bytes())
    }
    fn usize(&mut self, x: usize) -> Result<(), Vec<MirError>> {
        self.u64(u64::try_from(x).map_err(|_| error("integer does not fit bytecode format"))?)
    }
    fn string(&mut self, x: &str) -> Result<(), Vec<MirError>> {
        if x.len() > self.limits.max_string_bytes {
            return Err(error("string exceeds configured bytecode limit"));
        }
        self.usize(x.len())?;
        self.raw(x.as_bytes())
    }
    fn span(&mut self, s: Span) -> Result<(), Vec<MirError>> {
        if s.start > s.end {
            return Err(error("invalid source span in bytecode"));
        }
        self.usize(s.start)?;
        self.usize(s.end)?;
        self.usize(s.line)?;
        self.usize(s.column)?;
        self.u32(s.source_id)
    }
    fn definitions(
        &mut self,
        aliases: &[TypeDef],
        records: &[RecordDef],
        enums: &[EnumDef],
        views: &[ViewDef],
        scoped_tables: &[String],
    ) -> Result<(), Vec<MirError>> {
        self.usize(aliases.len())?;
        for alias in aliases {
            self.string(&alias.name)?;
            self.u8(u8::from(alias.is_public))?;
            self.ty(&alias.target, 0)?;
            self.span(alias.span)?;
        }
        self.usize(records.len())?;
        for record in records {
            self.string(&record.name)?;
            self.u8(u8::from(record.is_public))?;
            self.u8(u8::from(record.is_class))?;
            self.string_list(&record.type_params)?;
            self.usize(record.fields.len())?;
            for field in &record.fields {
                self.string(&field.name)?;
                self.ty(&field.ty, 0)?;
                self.span(field.span)?;
            }
            self.span(record.span)?;
        }
        self.usize(enums.len())?;
        for definition in enums {
            self.string(&definition.name)?;
            self.u8(u8::from(definition.is_public))?;
            self.string_list(&definition.type_params)?;
            self.string_list(&definition.variants)?;
            self.usize(definition.variant_payloads.len())?;
            for (variant, payloads) in &definition.variant_payloads {
                self.string(variant)?;
                self.usize(payloads.len())?;
                for payload in payloads {
                    self.ty(payload, 0)?;
                }
            }
            self.span(definition.span)?;
        }
        self.usize(views.len())?;
        for view in views {
            self.string(&view.name)?;
            self.u8(u8::from(view.is_public))?;
            self.usize(view.props.len())?;
            for prop in &view.props {
                self.string(&prop.name)?;
                self.ty(&prop.ty, 0)?;
                self.span(prop.span)?;
            }
            self.string(&view.html)?;
            self.span(view.span)?;
        }
        self.string_list(scoped_tables)?;
        Ok(())
    }
    fn string_list(&mut self, values: &[String]) -> Result<(), Vec<MirError>> {
        self.usize(values.len())?;
        for value in values {
            self.string(value)?;
        }
        Ok(())
    }
    fn ty(&mut self, t: &Type, depth: usize) -> Result<(), Vec<MirError>> {
        if depth > self.limits.max_type_depth {
            return Err(error("type nesting exceeds configured bytecode limit"));
        }
        self.type_nodes = self.type_nodes.saturating_add(1);
        if self.type_nodes > self.limits.max_type_nodes {
            return Err(error("type node count exceeds configured bytecode limit"));
        }
        match t {
            Type::Unknown => self.u8(0),
            Type::Int => self.u8(1),
            Type::UInt => self.u8(2),
            Type::Float => self.u8(3),
            Type::Decimal => self.u8(4),
            Type::Bool => self.u8(5),
            Type::String => self.u8(6),
            Type::Char => self.u8(7),
            Type::Bytes => self.u8(8),
            Type::Json => self.u8(9),
            Type::Timestamp => self.u8(10),
            Type::Date => self.u8(11),
            Type::Time => self.u8(12),
            Type::Duration => self.u8(13),
            Type::Unit => self.u8(14),
            Type::Option(a) => {
                self.u8(15)?;
                self.ty(a, depth + 1)
            }
            Type::Result(a, b) => {
                self.u8(16)?;
                self.ty(a, depth + 1)?;
                self.ty(b, depth + 1)
            }
            Type::Array(a) => {
                self.u8(17)?;
                self.ty(a, depth + 1)
            }
            Type::Tuple(xs) => {
                self.u8(18)?;
                self.usize(xs.len())?;
                for x in xs {
                    self.ty(x, depth + 1)?;
                }
                Ok(())
            }
            Type::Set(a) => {
                self.u8(19)?;
                self.ty(a, depth + 1)
            }
            Type::Map(a, b) => {
                self.u8(20)?;
                self.ty(a, depth + 1)?;
                self.ty(b, depth + 1)
            }
            Type::HttpResult(a) => {
                self.u8(21)?;
                self.ty(a, depth + 1)
            }
            Type::Function {
                params,
                return_type,
                capabilities,
            } => {
                self.u8(22)?;
                self.usize(params.len())?;
                for x in params {
                    self.ty(x, depth + 1)?;
                }
                self.ty(return_type, depth + 1)?;
                self.usize(capabilities.len())?;
                for x in capabilities {
                    self.string(x)?;
                }
                Ok(())
            }
            Type::Applied { name, args } => {
                self.u8(23)?;
                self.string(name)?;
                self.usize(args.len())?;
                for x in args {
                    self.ty(x, depth + 1)?;
                }
                Ok(())
            }
            Type::Named(n) => {
                self.u8(24)?;
                self.string(n)
            }
        }
    }
    fn op(&mut self, op: &BytecodeOp) -> Result<(), Vec<MirError>> {
        match op {
            BytecodeOp::Step => self.u8(50),
            BytecodeOp::Constant(Value::Int(x)) => {
                self.u8(0)?;
                self.raw(&x.to_le_bytes())
            }
            BytecodeOp::Constant(Value::UInt(x)) => {
                self.u8(1)?;
                self.u64(*x)
            }
            BytecodeOp::Constant(Value::Float(x)) => {
                self.u8(2)?;
                self.u64(x.to_bits())
            }
            BytecodeOp::Constant(Value::Bool(x)) => {
                self.u8(3)?;
                self.u8(u8::from(*x))
            }
            BytecodeOp::Constant(Value::String(x)) => {
                self.u8(4)?;
                self.string(x)
            }
            BytecodeOp::Constant(Value::Char(x)) => {
                self.u8(5)?;
                self.u32(*x as u32)
            }
            BytecodeOp::Constant(Value::Unit) => self.u8(6),
            BytecodeOp::Constant(_) => Err(error(
                "bytecode format does not support this constant value",
            )),
            BytecodeOp::LoadLocal(x) => {
                self.u8(7)?;
                self.usize(x.0)
            }
            BytecodeOp::StoreLocal(x) => {
                self.u8(8)?;
                self.usize(x.0)
            }
            BytecodeOp::Pop => self.u8(9),
            BytecodeOp::Unary(x) => {
                self.u8(10)?;
                self.u8(match x {
                    UnaryOp::Negate => 0,
                    UnaryOp::Not => 1,
                })
            }
            BytecodeOp::Binary(x) => {
                self.u8(11)?;
                self.u8(binary_tag(*x))
            }
            BytecodeOp::Call {
                function,
                arguments,
            } => {
                self.u8(12)?;
                self.usize(*function)?;
                self.usize(*arguments)
            }
            BytecodeOp::FunctionRef { function } => {
                self.u8(43)?;
                self.usize(*function)
            }
            BytecodeOp::CallValue { arguments } => {
                self.u8(44)?;
                self.usize(*arguments)
            }
            BytecodeOp::MakeClosure { function, captures } => {
                self.u8(48)?;
                self.usize(*function)?;
                self.usize(captures.len())?;
                for capture in captures {
                    self.usize(capture.0)?;
                }
                Ok(())
            }
            BytecodeOp::Sql {
                query,
                parameters,
                result_type,
            } => {
                self.u8(45)?;
                self.string(query)?;
                self.usize(parameters.len())?;
                for (name, local) in parameters {
                    self.string(name)?;
                    self.usize(local.0)?;
                }
                self.ty(result_type, 0)
            }
            BytecodeOp::SqlTransaction { operations } => {
                self.u8(46)?;
                self.usize(operations.len())?;
                for operation in operations {
                    match operation {
                        BytecodeTransactionOperation::Sql(statement) => {
                            self.u8(0)?;
                            self.string(&statement.query)?;
                            self.usize(statement.parameters.len())?;
                            for (name, local) in &statement.parameters {
                                self.string(name)?;
                                self.usize(local.0)?;
                            }
                            self.ty(&statement.result_type, 0)?;
                        }
                        BytecodeTransactionOperation::MailQueue {
                            arguments,
                            argument_spans,
                            call_span,
                        } => {
                            if argument_spans.len() != *arguments || !matches!(arguments, 3..=6) {
                                return Err(error(
                                    "invalid bytecode transactional mail_queue metadata",
                                ));
                            }
                            self.u8(1)?;
                            self.usize(*arguments)?;
                            for span in argument_spans {
                                self.span(*span)?;
                            }
                            self.span(*call_span)?;
                        }
                    }
                }
                Ok(())
            }
            BytecodeOp::RenderPdf => self.u8(47),
            BytecodeOp::Jump(x) => {
                self.u8(13)?;
                self.usize(*x)
            }
            BytecodeOp::JumpIfFalse(x) => {
                self.u8(14)?;
                self.usize(*x)
            }
            BytecodeOp::JumpIfTrue(x) => {
                self.u8(15)?;
                self.usize(*x)
            }
            BytecodeOp::Return => self.u8(16),
            BytecodeOp::Trap => self.u8(17),
            BytecodeOp::RangeInit { iterator } => {
                self.u8(18)?;
                self.usize(iterator.0)
            }
            BytecodeOp::RangeNext {
                iterator,
                local,
                body,
                exit,
            } => {
                self.u8(19)?;
                self.usize(iterator.0)?;
                self.usize(local.0)?;
                self.usize(*body)?;
                self.usize(*exit)
            }
            BytecodeOp::CollectionInit { iterator } => {
                self.u8(20)?;
                self.usize(iterator.0)
            }
            BytecodeOp::CollectionNext {
                iterator,
                locals,
                body,
                exit,
            } => {
                self.u8(21)?;
                self.usize(iterator.0)?;
                self.usize(locals.len())?;
                for local in locals {
                    self.usize(local.0)?;
                }
                self.usize(*body)?;
                self.usize(*exit)
            }
            BytecodeOp::BuildArray { items } => {
                self.u8(22)?;
                self.usize(*items)
            }
            BytecodeOp::BuildTuple { items } => {
                self.u8(23)?;
                self.usize(*items)
            }
            BytecodeOp::Index => self.u8(24),
            BytecodeOp::ArrayIndexAssign { local, operation } => {
                self.u8(25)?;
                self.usize(local.0)?;
                self.u8(match operation {
                    zelyra_ast::ArrayIndexAssignOp::Set => 0,
                    zelyra_ast::ArrayIndexAssignOp::Add => 1,
                })
            }
            BytecodeOp::ArrayIndexCheck { local } => {
                self.u8(51)?;
                self.usize(local.0)
            }
            BytecodeOp::ArrayAppend {
                local,
                element_type,
            } => {
                self.u8(38)?;
                self.usize(local.0)?;
                self.ty(element_type, 0)
            }
            BytecodeOp::ArrayPopMut {
                local,
                element_type,
            } => {
                self.u8(49)?;
                self.usize(local.0)?;
                self.ty(element_type, 0)
            }
            BytecodeOp::OptionSome { value_type } => {
                self.u8(39)?;
                self.ty(value_type, 0)
            }
            BytecodeOp::OptionNone { value_type } => {
                self.u8(40)?;
                self.ty(value_type, 0)
            }
            BytecodeOp::RecordUpdate {
                record_type,
                fields,
            } => {
                self.u8(41)?;
                self.ty(record_type, 0)?;
                self.usize(fields.len())?;
                for (name, ty) in fields {
                    self.string(name)?;
                    self.ty(ty, 0)?;
                }
                Ok(())
            }
            BytecodeOp::Parallel { bindings } => {
                if bindings.len() > self.limits.max_instructions
                    || bindings
                        .iter()
                        .any(|binding| binding.captures.len() > self.limits.max_instructions)
                {
                    return Err(error(
                        "parallel branch or capture count exceeds configured bytecode limit",
                    ));
                }
                self.u8(42)?;
                self.usize(bindings.len())?;
                for binding in bindings {
                    self.usize(binding.target.0)?;
                    self.u8(u8::from(binding.is_new))?;
                    self.usize(binding.function)?;
                    self.usize(binding.captures.len())?;
                    for capture in &binding.captures {
                        self.usize(capture.0)?;
                    }
                    self.ty(&binding.result_type, 0)?;
                }
                Ok(())
            }
            BytecodeOp::TupleBind { locals } => {
                self.u8(26)?;
                self.usize(locals.len())?;
                for local in locals {
                    self.usize(local.0)?;
                }
                Ok(())
            }
            BytecodeOp::Builtin {
                name,
                arguments,
                type_arguments,
                result_type,
            } => {
                self.u8(27)?;
                self.string(name)?;
                self.usize(*arguments)?;
                self.usize(type_arguments.len())?;
                for ty in type_arguments {
                    self.ty(ty, 0)?;
                }
                self.ty(result_type, 0)
            }
            BytecodeOp::BuildRecord {
                record_type,
                fields,
            } => {
                self.u8(28)?;
                self.ty(record_type, 0)?;
                self.usize(fields.len())?;
                for (name, ty) in fields {
                    self.string(name)?;
                    self.ty(ty, 0)?;
                }
                Ok(())
            }
            BytecodeOp::GetField {
                record_type,
                field,
                result_type,
            } => {
                self.u8(29)?;
                self.ty(record_type, 0)?;
                self.string(field)?;
                self.ty(result_type, 0)
            }
            BytecodeOp::BuildMap {
                entries,
                key_type,
                value_type,
            } => {
                self.u8(30)?;
                self.usize(*entries)?;
                self.ty(key_type, 0)?;
                self.ty(value_type, 0)
            }
            BytecodeOp::ContractCheck { postcondition } => {
                self.u8(31)?;
                self.u8(u8::from(*postcondition))
            }
            BytecodeOp::LoopInvariantCheck { phase } => {
                self.u8(32)?;
                self.u8(match phase {
                    crate::LoopInvariantPhase::Before => 0,
                    crate::LoopInvariantPhase::After => 1,
                })
            }
            BytecodeOp::HostCallPreflight { name, arguments } => {
                self.u8(33)?;
                self.string(name)?;
                self.usize(*arguments)
            }
            BytecodeOp::BuildEnum {
                enum_type,
                variant,
                payload_types,
            } => {
                self.u8(34)?;
                self.ty(enum_type, 0)?;
                self.string(variant)?;
                self.usize(payload_types.len())?;
                for ty in payload_types {
                    self.ty(ty, 0)?;
                }
                Ok(())
            }
            BytecodeOp::MatchInit { match_id } => {
                self.u8(35)?;
                self.usize(match_id.0)
            }
            BytecodeOp::MatchTest {
                match_id,
                pattern,
                matched,
                unmatched,
            } => {
                self.u8(36)?;
                self.usize(match_id.0)?;
                self.pattern(pattern, 0)?;
                self.usize(*matched)?;
                self.usize(*unmatched)
            }
            BytecodeOp::MatchFailure => self.u8(37),
        }
    }
    fn pattern(&mut self, pattern: &crate::MirPattern, depth: usize) -> Result<(), Vec<MirError>> {
        if depth > self.limits.max_type_depth {
            return Err(error("pattern nesting exceeds configured bytecode limit"));
        }
        use crate::MirPatternKind as K;
        match &pattern.kind {
            K::Wildcard => self.u8(0)?,
            K::Binding { name, local } => {
                self.u8(1)?;
                self.string(name)?;
                self.usize(local.0)?;
            }
            K::Constructor { name, inner } => {
                self.u8(2)?;
                self.string(name)?;
                self.u8(u8::from(inner.is_some()))?;
                if let Some(p) = inner {
                    self.pattern(p, depth + 1)?;
                }
            }
            K::EnumVariant { enum_name, variant } => {
                self.u8(3)?;
                self.string(enum_name)?;
                self.string(variant)?;
            }
            K::EnumVariantPayload {
                enum_name,
                variant,
                payloads,
            } => {
                self.u8(4)?;
                self.string(enum_name)?;
                self.string(variant)?;
                self.usize(payloads.len())?;
                for p in payloads {
                    self.pattern(p, depth + 1)?;
                }
            }
            K::Record { type_name, fields } => {
                self.u8(5)?;
                self.string(type_name)?;
                self.usize(fields.len())?;
                for (name, p) in fields {
                    self.string(name)?;
                    self.pattern(p, depth + 1)?;
                }
            }
            K::Int(value) => {
                self.u8(6)?;
                self.raw(&value.to_le_bytes())?;
            }
            K::Bool(value) => {
                self.u8(7)?;
                self.u8(u8::from(*value))?;
            }
            K::String(value) => {
                self.u8(8)?;
                self.string(value)?;
            }
            K::Char(value) => {
                self.u8(9)?;
                self.u32(*value as u32)?;
            }
        }
        self.span(pattern.span)
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    limits: BytecodeFormatLimits,
    type_nodes: usize,
    instructions: usize,
}
impl<'a> Reader<'a> {
    fn raw(&mut self, n: usize) -> Result<&'a [u8], Vec<MirError>> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or_else(|| error("bytecode offset overflow"))?;
        if end > self.bytes.len() {
            return Err(error("truncated bytecode artifact"));
        }
        let out = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(out)
    }
    fn u8(&mut self) -> Result<u8, Vec<MirError>> {
        Ok(self.raw(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, Vec<MirError>> {
        Ok(u16::from_le_bytes(self.raw(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, Vec<MirError>> {
        Ok(u32::from_le_bytes(self.raw(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, Vec<MirError>> {
        Ok(u64::from_le_bytes(self.raw(8)?.try_into().unwrap()))
    }
    fn usize(&mut self) -> Result<usize, Vec<MirError>> {
        usize::try_from(self.u64()?).map_err(|_| error("bytecode integer exceeds platform size"))
    }
    fn count(&mut self, max: usize, kind: &str) -> Result<usize, Vec<MirError>> {
        let n = self.usize()?;
        if n > max {
            Err(error(format!(
                "bytecode {kind} count exceeds configured limit"
            )))
        } else {
            Ok(n)
        }
    }
    fn string(&mut self) -> Result<String, Vec<MirError>> {
        let n = self.count(self.limits.max_string_bytes, "string byte")?;
        String::from_utf8(self.raw(n)?.to_vec())
            .map_err(|_| error("bytecode string is not valid UTF-8"))
    }
    fn span(&mut self) -> Result<Span, Vec<MirError>> {
        let s = Span {
            start: self.usize()?,
            end: self.usize()?,
            line: self.usize()?,
            column: self.usize()?,
            source_id: self.u32()?,
        };
        if s.start > s.end {
            return Err(error("invalid source span in bytecode"));
        }
        Ok(s)
    }
    fn boolean(&mut self) -> Result<bool, Vec<MirError>> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(error("invalid boolean in bytecode type definitions")),
        }
    }
    fn string_list(&mut self, kind: &str) -> Result<Vec<String>, Vec<MirError>> {
        let count = self.count(self.limits.max_instructions, kind)?;
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(self.string()?);
        }
        Ok(values)
    }
    fn definitions(&mut self) -> Result<TypeDefinitions, Vec<MirError>> {
        let alias_count = self.count(self.limits.max_functions, "type alias")?;
        let mut aliases = Vec::with_capacity(alias_count);
        for _ in 0..alias_count {
            aliases.push(TypeDef {
                name: self.string()?,
                is_public: self.boolean()?,
                target: self.ty(0)?,
                span: self.span()?,
            });
        }
        let record_count = self.count(self.limits.max_functions, "record definition")?;
        let mut records = Vec::with_capacity(record_count);
        for _ in 0..record_count {
            let name = self.string()?;
            let is_public = self.boolean()?;
            let is_class = self.boolean()?;
            let type_params = self.string_list("record type parameter")?;
            let field_count = self.count(self.limits.max_instructions, "record field")?;
            let mut fields = Vec::with_capacity(field_count);
            for _ in 0..field_count {
                fields.push(RecordField {
                    name: self.string()?,
                    ty: self.ty(0)?,
                    span: self.span()?,
                });
            }
            records.push(RecordDef {
                name,
                is_public,
                is_class,
                type_params,
                fields,
                span: self.span()?,
            });
        }
        let enum_count = self.count(self.limits.max_functions, "enum definition")?;
        let mut enums = Vec::with_capacity(enum_count);
        for _ in 0..enum_count {
            let name = self.string()?;
            let is_public = self.boolean()?;
            let type_params = self.string_list("enum type parameter")?;
            let variants = self.string_list("enum variant")?;
            let payload_count = self.count(self.limits.max_instructions, "enum variant payload")?;
            let mut variant_payloads = Vec::with_capacity(payload_count);
            for _ in 0..payload_count {
                let variant = self.string()?;
                let count = self.count(self.limits.max_instructions, "enum payload type")?;
                let mut payloads = Vec::with_capacity(count);
                for _ in 0..count {
                    payloads.push(self.ty(0)?);
                }
                variant_payloads.push((variant, payloads));
            }
            enums.push(EnumDef {
                name,
                is_public,
                type_params,
                variants,
                variant_payloads,
                span: self.span()?,
            });
        }
        let view_count = self.count(self.limits.max_functions, "view definition")?;
        let mut views = Vec::with_capacity(view_count);
        for _ in 0..view_count {
            let name = self.string()?;
            let is_public = self.boolean()?;
            let prop_count = self.count(self.limits.max_type_nodes, "view property")?;
            let mut props = Vec::with_capacity(prop_count);
            for _ in 0..prop_count {
                props.push(ComponentProp {
                    name: self.string()?,
                    ty: self.ty(0)?,
                    span: self.span()?,
                });
            }
            views.push(ViewDef {
                name,
                is_public,
                props,
                html: self.string()?,
                span: self.span()?,
            });
        }
        let scoped_tables = self.string_list("tenant-scoped table")?;
        Ok((aliases, records, enums, views, scoped_tables))
    }
    fn ty(&mut self, depth: usize) -> Result<Type, Vec<MirError>> {
        if depth > self.limits.max_type_depth {
            return Err(error("type nesting exceeds configured bytecode limit"));
        }
        self.type_nodes += 1;
        if self.type_nodes > self.limits.max_type_nodes {
            return Err(error("type node count exceeds configured bytecode limit"));
        }
        Ok(match self.u8()? {
            0 => Type::Unknown,
            1 => Type::Int,
            2 => Type::UInt,
            3 => Type::Float,
            4 => Type::Decimal,
            5 => Type::Bool,
            6 => Type::String,
            7 => Type::Char,
            8 => Type::Bytes,
            9 => Type::Json,
            10 => Type::Timestamp,
            11 => Type::Date,
            12 => Type::Time,
            13 => Type::Duration,
            14 => Type::Unit,
            15 => Type::Option(Box::new(self.ty(depth + 1)?)),
            16 => Type::Result(Box::new(self.ty(depth + 1)?), Box::new(self.ty(depth + 1)?)),
            17 => Type::Array(Box::new(self.ty(depth + 1)?)),
            18 => {
                let n = self.count(self.limits.max_type_nodes, "tuple type")?;
                let mut xs = Vec::with_capacity(n);
                for _ in 0..n {
                    xs.push(self.ty(depth + 1)?)
                }
                Type::Tuple(xs)
            }
            19 => Type::Set(Box::new(self.ty(depth + 1)?)),
            20 => Type::Map(Box::new(self.ty(depth + 1)?), Box::new(self.ty(depth + 1)?)),
            21 => Type::HttpResult(Box::new(self.ty(depth + 1)?)),
            22 => {
                let n = self.count(self.limits.max_type_nodes, "function parameter")?;
                let mut params = Vec::with_capacity(n);
                for _ in 0..n {
                    params.push(self.ty(depth + 1)?)
                }
                let return_type = Box::new(self.ty(depth + 1)?);
                let n = self.count(self.limits.max_type_nodes, "capability")?;
                let mut capabilities = Vec::with_capacity(n);
                for _ in 0..n {
                    capabilities.push(self.string()?)
                }
                Type::Function {
                    params,
                    return_type,
                    capabilities,
                }
            }
            23 => {
                let name = self.string()?;
                let n = self.count(self.limits.max_type_nodes, "generic type argument")?;
                let mut args = Vec::with_capacity(n);
                for _ in 0..n {
                    args.push(self.ty(depth + 1)?)
                }
                Type::Applied { name, args }
            }
            24 => Type::Named(self.string()?),
            _ => return Err(error("unknown bytecode type tag")),
        })
    }
    fn op(&mut self) -> Result<BytecodeOp, Vec<MirError>> {
        Ok(match self.u8()? {
            50 => BytecodeOp::Step,
            0 => BytecodeOp::Constant(Value::Int(i64::from_le_bytes(
                self.raw(8)?.try_into().unwrap(),
            ))),
            1 => BytecodeOp::Constant(Value::UInt(self.u64()?)),
            2 => BytecodeOp::Constant(Value::Float(f64::from_bits(self.u64()?))),
            3 => {
                let b = self.u8()?;
                if b > 1 {
                    return Err(error("invalid boolean bytecode constant"));
                }
                BytecodeOp::Constant(Value::Bool(b == 1))
            }
            4 => BytecodeOp::Constant(Value::String(self.string()?)),
            5 => BytecodeOp::Constant(Value::Char(
                char::from_u32(self.u32()?)
                    .ok_or_else(|| error("invalid Unicode scalar in bytecode"))?,
            )),
            6 => BytecodeOp::Constant(Value::Unit),
            7 => BytecodeOp::LoadLocal(LocalId(self.usize()?)),
            8 => BytecodeOp::StoreLocal(LocalId(self.usize()?)),
            9 => BytecodeOp::Pop,
            10 => BytecodeOp::Unary(match self.u8()? {
                0 => UnaryOp::Negate,
                1 => UnaryOp::Not,
                _ => return Err(error("unknown bytecode unary operator")),
            }),
            11 => BytecodeOp::Binary(
                binary_from_tag(self.u8()?)
                    .ok_or_else(|| error("unknown bytecode binary operator"))?,
            ),
            12 => BytecodeOp::Call {
                function: self.usize()?,
                arguments: self.usize()?,
            },
            13 => BytecodeOp::Jump(self.usize()?),
            14 => BytecodeOp::JumpIfFalse(self.usize()?),
            15 => BytecodeOp::JumpIfTrue(self.usize()?),
            16 => BytecodeOp::Return,
            17 => BytecodeOp::Trap,
            18 => BytecodeOp::RangeInit {
                iterator: IteratorId(self.usize()?),
            },
            19 => BytecodeOp::RangeNext {
                iterator: IteratorId(self.usize()?),
                local: LocalId(self.usize()?),
                body: self.usize()?,
                exit: self.usize()?,
            },
            20 => BytecodeOp::CollectionInit {
                iterator: IteratorId(self.usize()?),
            },
            21 => BytecodeOp::CollectionNext {
                iterator: IteratorId(self.usize()?),
                locals: {
                    let count = self.count(self.limits.max_instructions, "iterator binding")?;
                    let mut locals = Vec::with_capacity(count);
                    for _ in 0..count {
                        locals.push(LocalId(self.usize()?));
                    }
                    locals
                },
                body: self.usize()?,
                exit: self.usize()?,
            },
            22 => BytecodeOp::BuildArray {
                items: self.count(self.limits.max_instructions, "array item")?,
            },
            23 => BytecodeOp::BuildTuple {
                items: self.count(self.limits.max_instructions, "tuple item")?,
            },
            24 => BytecodeOp::Index,
            25 => BytecodeOp::ArrayIndexAssign {
                local: LocalId(self.usize()?),
                operation: match self.u8()? {
                    0 => zelyra_ast::ArrayIndexAssignOp::Set,
                    1 => zelyra_ast::ArrayIndexAssignOp::Add,
                    _ => return Err(error("unknown bytecode array update operation")),
                },
            },
            51 => BytecodeOp::ArrayIndexCheck {
                local: LocalId(self.usize()?),
            },
            26 => BytecodeOp::TupleBind {
                locals: {
                    let count = self.count(self.limits.max_instructions, "tuple destination")?;
                    let mut locals = Vec::with_capacity(count);
                    for _ in 0..count {
                        locals.push(LocalId(self.usize()?));
                    }
                    locals
                },
            },
            27 => BytecodeOp::Builtin {
                name: self.string()?,
                arguments: self.count(self.limits.max_instructions, "builtin argument")?,
                type_arguments: {
                    let count = self.count(self.limits.max_type_nodes, "builtin type argument")?;
                    let mut types = Vec::with_capacity(count);
                    for _ in 0..count {
                        types.push(self.ty(0)?);
                    }
                    types
                },
                result_type: self.ty(0)?,
            },
            28 => BytecodeOp::BuildRecord {
                record_type: self.ty(0)?,
                fields: {
                    let count = self.count(self.limits.max_instructions, "record field")?;
                    let mut fields = Vec::with_capacity(count);
                    for _ in 0..count {
                        fields.push((self.string()?, self.ty(0)?));
                    }
                    fields
                },
            },
            29 => BytecodeOp::GetField {
                record_type: self.ty(0)?,
                field: self.string()?,
                result_type: self.ty(0)?,
            },
            30 => BytecodeOp::BuildMap {
                entries: self.count(self.limits.max_instructions, "map entry")?,
                key_type: self.ty(0)?,
                value_type: self.ty(0)?,
            },
            31 => BytecodeOp::ContractCheck {
                postcondition: match self.u8()? {
                    0 => false,
                    1 => true,
                    _ => return Err(error("unknown bytecode contract kind")),
                },
            },
            32 => BytecodeOp::LoopInvariantCheck {
                phase: match self.u8()? {
                    0 => crate::LoopInvariantPhase::Before,
                    1 => crate::LoopInvariantPhase::After,
                    _ => return Err(error("unknown bytecode loop invariant phase")),
                },
            },
            33 => BytecodeOp::HostCallPreflight {
                name: self.string()?,
                arguments: self.count(self.limits.max_instructions, "host-call argument")?,
            },
            34 => BytecodeOp::BuildEnum {
                enum_type: self.ty(0)?,
                variant: self.string()?,
                payload_types: {
                    let n = self.count(self.limits.max_instructions, "enum payload")?;
                    let mut v = Vec::with_capacity(n);
                    for _ in 0..n {
                        v.push(self.ty(0)?);
                    }
                    v
                },
            },
            35 => BytecodeOp::MatchInit {
                match_id: crate::MatchId(self.usize()?),
            },
            36 => BytecodeOp::MatchTest {
                match_id: crate::MatchId(self.usize()?),
                pattern: self.pattern(0)?,
                matched: self.usize()?,
                unmatched: self.usize()?,
            },
            37 => BytecodeOp::MatchFailure,
            38 => BytecodeOp::ArrayAppend {
                local: LocalId(self.usize()?),
                element_type: self.ty(0)?,
            },
            39 => BytecodeOp::OptionSome {
                value_type: self.ty(0)?,
            },
            40 => BytecodeOp::OptionNone {
                value_type: self.ty(0)?,
            },
            41 => BytecodeOp::RecordUpdate {
                record_type: self.ty(0)?,
                fields: {
                    let count = self.count(self.limits.max_instructions, "record update field")?;
                    let mut fields = Vec::with_capacity(count);
                    for _ in 0..count {
                        fields.push((self.string()?, self.ty(0)?));
                    }
                    fields
                },
            },
            42 => BytecodeOp::Parallel {
                bindings: {
                    let count = self.count(self.limits.max_instructions, "parallel branch")?;
                    let mut bindings = Vec::with_capacity(count);
                    for _ in 0..count {
                        let target = LocalId(self.usize()?);
                        let is_new = match self.u8()? {
                            0 => false,
                            1 => true,
                            _ => return Err(error("invalid bytecode parallel binding flag")),
                        };
                        let function = self.usize()?;
                        let capture_count =
                            self.count(self.limits.max_instructions, "parallel capture")?;
                        let mut captures = Vec::with_capacity(capture_count);
                        for _ in 0..capture_count {
                            captures.push(LocalId(self.usize()?));
                        }
                        bindings.push(BytecodeParallelBinding {
                            target,
                            is_new,
                            function,
                            captures,
                            result_type: self.ty(0)?,
                        });
                    }
                    bindings
                },
            },
            43 => BytecodeOp::FunctionRef {
                function: self.usize()?,
            },
            49 => BytecodeOp::ArrayPopMut {
                local: LocalId(self.usize()?),
                element_type: self.ty(0)?,
            },
            44 => BytecodeOp::CallValue {
                arguments: self.count(self.limits.max_instructions, "function-value argument")?,
            },
            48 => BytecodeOp::MakeClosure {
                function: self.usize()?,
                captures: {
                    let count = self.count(self.limits.max_instructions, "closure capture")?;
                    let mut captures = Vec::with_capacity(count);
                    for _ in 0..count {
                        captures.push(LocalId(self.usize()?));
                    }
                    captures
                },
            },
            45 => BytecodeOp::Sql {
                query: self.string()?,
                parameters: {
                    let count = self.count(self.limits.max_instructions, "SQL parameter")?;
                    let mut parameters = Vec::with_capacity(count);
                    for _ in 0..count {
                        parameters.push((self.string()?, LocalId(self.usize()?)));
                    }
                    parameters
                },
                result_type: self.ty(0)?,
            },
            46 => BytecodeOp::SqlTransaction {
                operations: {
                    let count =
                        self.count(self.limits.max_instructions, "SQL transaction statement")?;
                    let mut operations = Vec::with_capacity(count);
                    for _ in 0..count {
                        match self.u8()? {
                            0 => {
                                let query = self.string()?;
                                let parameter_count = self.count(
                                    self.limits.max_instructions,
                                    "SQL transaction parameter",
                                )?;
                                let mut parameters = Vec::with_capacity(parameter_count);
                                for _ in 0..parameter_count {
                                    parameters.push((self.string()?, LocalId(self.usize()?)));
                                }
                                operations.push(BytecodeTransactionOperation::Sql(
                                    BytecodeSqlStatement {
                                        query,
                                        parameters,
                                        result_type: self.ty(0)?,
                                    },
                                ));
                            }
                            1 => {
                                let arguments = self.count(6, "mail_queue argument")?;
                                if !matches!(arguments, 3..=6) {
                                    return Err(error(
                                        "invalid bytecode transactional mail_queue arity",
                                    ));
                                }
                                let mut argument_spans = Vec::with_capacity(arguments);
                                for _ in 0..arguments {
                                    argument_spans.push(self.span()?);
                                }
                                operations.push(BytecodeTransactionOperation::MailQueue {
                                    arguments,
                                    argument_spans,
                                    call_span: self.span()?,
                                });
                            }
                            _ => return Err(error("unknown bytecode transaction operation")),
                        }
                    }
                    operations
                },
            },
            47 => BytecodeOp::RenderPdf,
            _ => return Err(error("unknown bytecode instruction tag")),
        })
    }
    fn pattern(&mut self, depth: usize) -> Result<crate::MirPattern, Vec<MirError>> {
        if depth > self.limits.max_type_depth {
            return Err(error("pattern nesting exceeds configured bytecode limit"));
        }
        use crate::MirPatternKind as K;
        let kind = match self.u8()? {
            0 => K::Wildcard,
            1 => K::Binding {
                name: self.string()?,
                local: LocalId(self.usize()?),
            },
            2 => {
                let name = self.string()?;
                let has_inner = self.u8()?;
                let inner = match has_inner {
                    0 => None,
                    1 => Some(Box::new(self.pattern(depth + 1)?)),
                    _ => return Err(error("invalid constructor pattern flag")),
                };
                K::Constructor { name, inner }
            }
            3 => K::EnumVariant {
                enum_name: self.string()?,
                variant: self.string()?,
            },
            4 => {
                let enum_name = self.string()?;
                let variant = self.string()?;
                let n = self.count(self.limits.max_instructions, "pattern payload")?;
                let mut payloads = Vec::with_capacity(n);
                for _ in 0..n {
                    payloads.push(self.pattern(depth + 1)?);
                }
                K::EnumVariantPayload {
                    enum_name,
                    variant,
                    payloads,
                }
            }
            5 => {
                let type_name = self.string()?;
                let n = self.count(self.limits.max_instructions, "record pattern field")?;
                let mut fields = Vec::with_capacity(n);
                for _ in 0..n {
                    fields.push((self.string()?, self.pattern(depth + 1)?));
                }
                K::Record { type_name, fields }
            }
            6 => K::Int(i64::from_le_bytes(self.raw(8)?.try_into().unwrap())),
            7 => match self.u8()? {
                0 => K::Bool(false),
                1 => K::Bool(true),
                _ => return Err(error("invalid boolean pattern")),
            },
            8 => K::String(self.string()?),
            9 => K::Char(
                char::from_u32(self.u32()?)
                    .ok_or_else(|| error("invalid Unicode scalar in pattern"))?,
            ),
            _ => return Err(error("unknown bytecode pattern tag")),
        };
        Ok(crate::MirPattern {
            kind,
            span: self.span()?,
        })
    }
}

fn binary_tag(x: BinaryOp) -> u8 {
    match x {
        BinaryOp::Add => 0,
        BinaryOp::Subtract => 1,
        BinaryOp::Multiply => 2,
        BinaryOp::Divide => 3,
        BinaryOp::Remainder => 4,
        BinaryOp::Equal => 5,
        BinaryOp::NotEqual => 6,
        BinaryOp::Less => 7,
        BinaryOp::LessEqual => 8,
        BinaryOp::Greater => 9,
        BinaryOp::GreaterEqual => 10,
        BinaryOp::And => 11,
        BinaryOp::Or => 12,
    }
}
fn binary_from_tag(x: u8) -> Option<BinaryOp> {
    Some(match x {
        0 => BinaryOp::Add,
        1 => BinaryOp::Subtract,
        2 => BinaryOp::Multiply,
        3 => BinaryOp::Divide,
        4 => BinaryOp::Remainder,
        5 => BinaryOp::Equal,
        6 => BinaryOp::NotEqual,
        7 => BinaryOp::Less,
        8 => BinaryOp::LessEqual,
        9 => BinaryOp::Greater,
        10 => BinaryOp::GreaterEqual,
        11 => BinaryOp::And,
        12 => BinaryOp::Or,
        _ => return None,
    })
}
fn error(message: impl Into<String>) -> Vec<MirError> {
    vec![MirError {
        code: "E-MIR-BYTECODE-002",
        message: message.into(),
        span: Span::default(),
    }]
}
