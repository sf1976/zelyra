//! Lower verified MIR into the supported versioned bytecode subset.

use super::*;

/// Compile one named function and its direct-call closure from already checked
/// and verified MIR. Unreachable functions with unsupported operations do not
/// prevent compiling this entry point.
pub fn compile(program: &MirProgram, entry_name: &str) -> Result<BytecodeProgram, Vec<MirError>> {
    verify(program)?;
    let Some(entry_function) = program.functions.iter().find(|f| f.name == entry_name) else {
        return Err(vec![bytecode_error(
            "bytecode entry function was not found",
            Span::default(),
        )]);
    };
    let mut reachable = HashSet::new();
    let mut pending = VecDeque::from([entry_function.id]);
    while let Some(function_id) = pending.pop_front() {
        if !reachable.insert(function_id) {
            continue;
        }
        let Some(function) = program.functions.iter().find(|f| f.id == function_id) else {
            return Err(vec![bytecode_error(
                "MIR call refers to a missing function",
                entry_function.span,
            )]);
        };
        for called in called_functions(program, function) {
            pending.push_back(called);
        }
    }
    let selected = program
        .functions
        .iter()
        .filter(|function| reachable.contains(&function.id))
        .collect::<Vec<_>>();
    let function_indices = selected
        .iter()
        .enumerate()
        .map(|(index, function)| (function.id, index))
        .collect::<HashMap<_, _>>();
    let mut functions = Vec::with_capacity(selected.len());
    let mut generated_functions = Vec::new();
    let mut next_generated_index = selected.len();
    for function in &selected {
        if function.declared_capabilities.iter().any(|capability| {
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
        }) {
            return Err(vec![bytecode_error(
                "bytecode backend does not support one or more declared capabilities",
                function.span,
            )]);
        }
        functions.push(compile_function(
            program,
            function,
            &function_indices,
            &mut next_generated_index,
            &mut generated_functions,
        )?);
    }
    functions.extend(generated_functions);
    let entry = *function_indices
        .get(&entry_function.id)
        .expect("entry function is included in the reachable set");
    let mut records = program.records.clone();
    records.extend(program.sql_records.iter().cloned());
    let output = BytecodeProgram {
        version: BYTECODE_VERSION,
        functions,
        entry,
        type_aliases: program.type_aliases.clone(),
        records,
        enums: program.enums.clone(),
        views: program.views.clone(),
        scoped_tables: program.scoped_tables.clone(),
    };
    verify_bytecode(&output)?;
    Ok(output)
}

fn compile_function(
    program: &MirProgram,
    function: &crate::MirFunction,
    function_indices: &HashMap<FunctionId, usize>,
    next_generated_index: &mut usize,
    generated_functions: &mut Vec<BytecodeFunction>,
) -> Result<BytecodeFunction, Vec<MirError>> {
    let immutable_function_aliases = immutable_function_aliases(program, function);
    let mut compiler = FunctionCompiler {
        program,
        function_indices,
        immutable_function_aliases,
        code: Vec::new(),
        blocks: HashMap::new(),
        iterators: HashMap::new(),
        patches: Vec::new(),
        next_match_id: next_match_id(function),
        next_local_id: maximum_local_id(program).saturating_add(1),
        next_iterator_id: maximum_iterator_id(function).saturating_add(1),
        contract_result_local: function.contract_result_local,
        ensures: function.ensures.clone(),
        declared_capabilities: function.declared_capabilities.clone(),
        initialization_writes: Vec::new(),
        generated_locals: Vec::new(),
        next_generated_index,
        generated_functions,
        pending_source_step: None,
    };
    for contract in &function.requires {
        let span = program
            .expressions
            .get(contract.0)
            .map_or(function.span, |item| item.span);
        compiler.expression(*contract, &mut HashSet::new())?;
        compiler.emit(
            BytecodeOp::ContractCheck {
                postcondition: false,
            },
            span,
        );
    }
    for block in &function.blocks {
        compiler.blocks.insert(block.id, compiler.code.len());
        if let Some(loop_span) = block.loop_span {
            compiler.emit(BytecodeOp::Step, loop_span);
        }
        for invariant in &block.loop_invariants {
            compiler.expression(*invariant, &mut HashSet::new())?;
            compiler.emit(
                BytecodeOp::LoopInvariantCheck {
                    phase: LoopInvariantPhase::Before,
                },
                block.loop_span.unwrap_or(block.span),
            );
        }
        for instruction in &block.instructions {
            let source_span = mir_instruction_source_span(instruction, program);
            if !matches!(
                instruction,
                MirInstruction::SourceStep { .. } | MirInstruction::LoopInvariantCheck { .. }
            ) {
                if let Some(span) = source_span {
                    compiler.emit(BytecodeOp::Step, span);
                }
            }
            compiler.instruction(instruction)?;
        }
        compiler.terminator(block.terminator.as_ref(), block.id, block.span)?;
    }
    compiler.flush_pending_source_step();
    for (instruction, target, second_target) in compiler.patches {
        let Some(address) = compiler.blocks.get(&target).copied() else {
            return Err(vec![bytecode_error(
                "MIR branch target is missing during bytecode lowering",
                function.span,
            )]);
        };
        match &mut compiler.code[instruction].operation {
            BytecodeOp::Jump(address_slot)
            | BytecodeOp::JumpIfFalse(address_slot)
            | BytecodeOp::JumpIfTrue(address_slot) => *address_slot = address,
            BytecodeOp::RangeNext { body, exit, .. } => {
                if second_target {
                    *exit = address;
                } else {
                    *body = address;
                }
            }
            BytecodeOp::CollectionNext { body, exit, .. } => {
                if second_target {
                    *exit = address;
                } else {
                    *body = address;
                }
            }
            BytecodeOp::MatchTest {
                matched, unmatched, ..
            } => {
                if second_target {
                    *unmatched = address;
                } else {
                    *matched = address;
                }
            }
            _ => {
                return Err(vec![bytecode_error(
                    "bytecode jump patch points to a non-branch instruction",
                    compiler.code[instruction].span,
                )]);
            }
        }
    }
    let mut output = BytecodeFunction {
        id: function.id,
        name: function.name.clone(),
        parameters: function
            .parameters
            .iter()
            .map(|(local, _, ty, span)| (*local, ty.clone(), *span))
            .collect(),
        captures: Vec::new(),
        local_mutability: function_local_mutability(program, function)
            .into_iter()
            .chain(
                compiler
                    .generated_locals
                    .iter()
                    .copied()
                    .map(|local| (local, true)),
            )
            .chain(std::iter::once((function.contract_result_local, true)))
            .collect(),
        initialization_writes: compiler.initialization_writes,
        return_type: function_return_type(program, function),
        declared_capabilities: function.declared_capabilities.clone(),
        code: compiler.code,
        max_stack: 0,
    };
    output.max_stack = verify_function(&output, function.span)?;
    Ok(output)
}

/// Resolve only immutable local aliases whose initializer is a named function
/// value. Mutable locals and locals with any assignment are deliberately left
/// dynamic so lowering cannot change callback selection at runtime.
fn immutable_function_aliases(
    program: &MirProgram,
    function: &crate::MirFunction,
) -> HashMap<LocalId, FunctionId> {
    let mut candidates = HashMap::new();
    let mut reassigned = HashSet::new();
    for block in &function.blocks {
        for instruction in &block.instructions {
            match instruction {
                MirInstruction::Let {
                    local,
                    mutable: false,
                    value,
                    ..
                } => {
                    if let Some(MirExpressionKind::FunctionValue {
                        function: Some(function),
                        ..
                    }) = program
                        .expressions
                        .get(value.0)
                        .map(|expression| &expression.kind)
                    {
                        candidates.insert(*local, *function);
                    }
                }
                MirInstruction::Let { local, .. }
                | MirInstruction::Assign { local, .. }
                | MirInstruction::ArrayIndexAssign { local, .. } => {
                    reassigned.insert(*local);
                }
                _ => {}
            }
        }
    }
    candidates.retain(|local, _| !reassigned.contains(local));
    candidates
}

fn function_local_mutability(
    program: &MirProgram,
    function: &crate::MirFunction,
) -> Vec<(LocalId, bool)> {
    fn add_pattern(pattern: &MirPattern, locals: &mut HashMap<LocalId, bool>) {
        match &pattern.kind {
            crate::MirPatternKind::Binding { local, .. } => {
                locals.insert(*local, false);
            }
            crate::MirPatternKind::Constructor {
                inner: Some(inner), ..
            } => add_pattern(inner, locals),
            crate::MirPatternKind::EnumVariantPayload { payloads, .. } => {
                for pattern in payloads {
                    add_pattern(pattern, locals);
                }
            }
            crate::MirPatternKind::Record { fields, .. } => {
                for (_, pattern) in fields {
                    add_pattern(pattern, locals);
                }
            }
            _ => {}
        }
    }

    let mut locals = HashMap::new();
    for (local, _, _, _) in &function.parameters {
        locals.insert(*local, false);
    }
    for expression in program.expressions.iter().filter(|expression| {
        expression.span.source_id == function.span.source_id
            && expression.span.start >= function.span.start
            && expression.span.end <= function.span.end
    }) {
        if let MirExpressionKind::Lambda { params, .. } = &expression.kind {
            for (local, _, _) in params {
                locals.insert(*local, false);
            }
        }
        if let MirExpressionKind::Match { arms, .. } = &expression.kind {
            for arm in arms {
                add_pattern(&arm.pattern, &mut locals);
            }
        }
    }
    for block in &function.blocks {
        for instruction in &block.instructions {
            match instruction {
                MirInstruction::Let { local, mutable, .. } => {
                    locals.insert(*local, *mutable);
                }
                MirInstruction::TupleBind { bindings, .. } => {
                    for binding in bindings.iter().filter(|binding| binding.is_new) {
                        locals.insert(binding.local, false);
                    }
                }
                MirInstruction::Parallel { bindings, .. } => {
                    for binding in bindings.iter().filter(|binding| binding.is_new) {
                        locals.insert(binding.local, false);
                    }
                }
                _ => {}
            }
        }
        match block.terminator.as_ref() {
            Some(MirTerminator::IteratorNext { bindings, .. }) => {
                for binding in bindings {
                    locals.insert(binding.local, false);
                }
            }
            Some(MirTerminator::MatchTest { pattern, .. }) => add_pattern(pattern, &mut locals),
            _ => {}
        }
    }
    let mut locals = locals.into_iter().collect::<Vec<_>>();
    locals.sort_by_key(|(local, _)| local.0);
    locals
}

fn mir_pattern_bound_locals(pattern: &MirPattern, output: &mut Vec<LocalId>) {
    match &pattern.kind {
        crate::MirPatternKind::Binding { local, .. } => output.push(*local),
        crate::MirPatternKind::Constructor {
            inner: Some(inner), ..
        } => mir_pattern_bound_locals(inner, output),
        crate::MirPatternKind::EnumVariantPayload { payloads, .. } => {
            for pattern in payloads {
                mir_pattern_bound_locals(pattern, output);
            }
        }
        crate::MirPatternKind::Record { fields, .. } => {
            for (_, pattern) in fields {
                mir_pattern_bound_locals(pattern, output);
            }
        }
        _ => {}
    }
}

fn next_match_id(function: &crate::MirFunction) -> MatchId {
    let maximum = function
        .blocks
        .iter()
        .flat_map(|block| {
            let instruction_ids =
                block
                    .instructions
                    .iter()
                    .filter_map(|instruction| match instruction {
                        MirInstruction::MatchInit { match_id, .. } => Some(match_id.0),
                        _ => None,
                    });
            let terminator_ids = match block.terminator.as_ref() {
                Some(MirTerminator::MatchTest { match_id, .. }) => Some(match_id.0),
                _ => None,
            };
            instruction_ids.chain(terminator_ids)
        })
        .max()
        .unwrap_or(0);
    MatchId(maximum.saturating_add(1))
}

fn maximum_local_id(program: &MirProgram) -> usize {
    fn add_pattern(pattern: &MirPattern, maximum: &mut usize) {
        match &pattern.kind {
            crate::MirPatternKind::Binding { local, .. } => *maximum = (*maximum).max(local.0),
            crate::MirPatternKind::Constructor {
                inner: Some(inner), ..
            } => add_pattern(inner, maximum),
            crate::MirPatternKind::EnumVariantPayload { payloads, .. } => {
                for payload in payloads {
                    add_pattern(payload, maximum);
                }
            }
            crate::MirPatternKind::Record { fields, .. } => {
                for (_, field) in fields {
                    add_pattern(field, maximum);
                }
            }
            _ => {}
        }
    }

    let mut maximum = 0;
    for expression in &program.expressions {
        match &expression.kind {
            MirExpressionKind::Local(local) => maximum = maximum.max(local.0),
            MirExpressionKind::Lambda { params, .. } => {
                for (local, _, _) in params {
                    maximum = maximum.max(local.0);
                }
            }
            _ => {}
        }
    }
    for function in &program.functions {
        maximum = maximum.max(function.contract_result_local.0);
        for (local, _, _, _) in &function.parameters {
            maximum = maximum.max(local.0);
        }
        for block in &function.blocks {
            for instruction in &block.instructions {
                match instruction {
                    MirInstruction::Let { local, .. }
                    | MirInstruction::Assign { local, .. }
                    | MirInstruction::ArrayIndexAssign { local, .. } => {
                        maximum = maximum.max(local.0)
                    }
                    MirInstruction::TupleBind { bindings, .. } => {
                        for binding in bindings {
                            maximum = maximum.max(binding.local.0);
                        }
                    }
                    MirInstruction::Parallel { bindings, .. } => {
                        for binding in bindings {
                            maximum = maximum.max(binding.local.0);
                        }
                    }
                    _ => {}
                }
            }
            if let Some(terminator) = &block.terminator {
                match terminator {
                    MirTerminator::IteratorNext { bindings, .. } => {
                        for binding in bindings {
                            maximum = maximum.max(binding.local.0);
                        }
                    }
                    MirTerminator::MatchTest { pattern, .. } => add_pattern(pattern, &mut maximum),
                    _ => {}
                }
            }
        }
    }
    maximum
}

fn parallel_captures(
    program: &MirProgram,
    root: ExpressionId,
) -> Result<Vec<(LocalId, Type, Span)>, Vec<MirError>> {
    capture_locals_with_bound(program, root, HashSet::new())
}

fn capture_locals_with_bound(
    program: &MirProgram,
    root: ExpressionId,
    mut bound: HashSet<LocalId>,
) -> Result<Vec<(LocalId, Type, Span)>, Vec<MirError>> {
    fn collect_pattern(pattern: &MirPattern, locals: &mut HashSet<LocalId>) {
        match &pattern.kind {
            crate::MirPatternKind::Binding { local, .. } => {
                locals.insert(*local);
            }
            crate::MirPatternKind::Constructor {
                inner: Some(inner), ..
            } => collect_pattern(inner, locals),
            crate::MirPatternKind::EnumVariantPayload { payloads, .. } => {
                for payload in payloads {
                    collect_pattern(payload, locals);
                }
            }
            crate::MirPatternKind::Record { fields, .. } => {
                for (_, field) in fields {
                    collect_pattern(field, locals);
                }
            }
            _ => {}
        }
    }

    fn walk(
        program: &MirProgram,
        id: ExpressionId,
        bound: &mut HashSet<LocalId>,
        visited: &mut HashSet<ExpressionId>,
        captures: &mut HashMap<LocalId, (Type, Span)>,
    ) -> Result<(), Vec<MirError>> {
        if !visited.insert(id) {
            return Ok(());
        }
        let Some(expression) = program.expressions.get(id.0) else {
            return Err(vec![bytecode_error(
                "parallel branch expression is outside the MIR expression table",
                Span::default(),
            )]);
        };
        use MirExpressionKind as K;
        let mut children = Vec::new();
        match &expression.kind {
            K::Array(items) => children.extend(items.iter().copied()),
            K::Map(entries) => {
                children.extend(entries.iter().flat_map(|(key, value)| [*key, *value]));
            }
            K::Record { fields, .. } => children.extend(fields.iter().map(|(_, value)| *value)),
            K::Lambda { params, body } => {
                let inserted = params
                    .iter()
                    .map(|(local, _, _)| *local)
                    .filter(|local| bound.insert(*local))
                    .collect::<Vec<_>>();
                walk(program, *body, bound, visited, captures)?;
                for local in inserted {
                    bound.remove(&local);
                }
            }
            K::Local(local) if !bound.contains(local) => {
                captures
                    .entry(*local)
                    .or_insert_with(|| (expression.ty.clone(), expression.span));
            }
            K::Index { target, index } => children.extend([*target, *index]),
            K::Field { target, .. } => children.push(*target),
            K::Call { args, .. } => children.extend(args.iter().copied()),
            K::Unary { expr, .. } => children.push(*expr),
            K::Binary { left, right, .. } => children.extend([*left, *right]),
            K::BinaryChain { first, operations } => {
                children.push(*first);
                children.extend(operations.iter().map(|(_, value, _)| *value));
            }
            K::Sql { parameters, .. } => {
                for (_, local) in parameters {
                    if !bound.contains(local) {
                        if let Some((_, ty, span)) =
                            program.expressions.iter().find_map(|item| {
                                matches!(&item.kind, K::Local(found) if *found == *local)
                                    .then_some((item.id, item.ty.clone(), item.span))
                            })
                        {
                            captures.entry(*local).or_insert((ty, span));
                        }
                    }
                }
            }
            K::Await(value) => children.push(*value),
            K::Match { value, arms } => {
                children.push(*value);
                for arm in arms {
                    let mut arm_bound = HashSet::new();
                    collect_pattern(&arm.pattern, &mut arm_bound);
                    let inserted = arm_bound
                        .into_iter()
                        .filter(|local| bound.insert(*local))
                        .collect::<Vec<_>>();
                    if let Some(guard) = arm.guard {
                        walk(program, guard, bound, visited, captures)?;
                    }
                    walk(program, arm.value, bound, visited, captures)?;
                    for local in inserted {
                        bound.remove(&local);
                    }
                }
            }
            K::Uninitialized
            | K::Int(_)
            | K::UInt(_)
            | K::Float(_)
            | K::Bool(_)
            | K::String(_)
            | K::Char(_)
            | K::Local(_)
            | K::EnumVariant { .. }
            | K::FunctionValue { .. } => {}
        }
        for child in children {
            walk(program, child, bound, visited, captures)?;
        }
        Ok(())
    }

    let mut captures = HashMap::new();
    walk(
        program,
        root,
        &mut bound,
        &mut HashSet::new(),
        &mut captures,
    )?;
    let mut captures = captures.into_iter().collect::<Vec<_>>();
    captures.sort_by_key(|(local, _)| local.0);
    Ok(captures
        .into_iter()
        .map(|(local, (ty, span))| (local, ty, span))
        .collect())
}

fn maximum_iterator_id(function: &crate::MirFunction) -> usize {
    function
        .blocks
        .iter()
        .flat_map(|block| {
            let instruction_ids =
                block
                    .instructions
                    .iter()
                    .filter_map(|instruction| match instruction {
                        MirInstruction::IteratorInit { iterator, .. } => Some(iterator.0),
                        _ => None,
                    });
            let terminator_id = match block.terminator.as_ref() {
                Some(MirTerminator::IteratorNext { iterator, .. }) => Some(iterator.0),
                _ => None,
            };
            instruction_ids.chain(terminator_id)
        })
        .max()
        .unwrap_or(0)
}

struct FunctionCompiler<'a> {
    program: &'a MirProgram,
    function_indices: &'a HashMap<FunctionId, usize>,
    immutable_function_aliases: HashMap<LocalId, FunctionId>,
    code: Vec<BytecodeInstruction>,
    blocks: HashMap<BlockId, usize>,
    iterators: HashMap<IteratorId, BytecodeIteratorKind>,
    patches: Vec<(usize, BlockId, bool)>,
    next_match_id: MatchId,
    next_local_id: usize,
    next_iterator_id: usize,
    contract_result_local: LocalId,
    ensures: Vec<ExpressionId>,
    declared_capabilities: Vec<String>,
    initialization_writes: Vec<(usize, LocalId)>,
    generated_locals: Vec<LocalId>,
    next_generated_index: &'a mut usize,
    generated_functions: &'a mut Vec<BytecodeFunction>,
    pending_source_step: Option<Span>,
}

fn mir_instruction_source_span(instruction: &MirInstruction, program: &MirProgram) -> Option<Span> {
    Some(match instruction {
        MirInstruction::SourceStep { span }
        | MirInstruction::IteratorInit { span, .. }
        | MirInstruction::MatchInit { span, .. }
        | MirInstruction::Let { span, .. }
        | MirInstruction::Assign { span, .. }
        | MirInstruction::ArrayIndexAssign { span, .. }
        | MirInstruction::TupleBind { span, .. }
        | MirInstruction::Transaction { span, .. }
        | MirInstruction::Parallel { span, .. }
        | MirInstruction::LoopInvariantCheck { span, .. } => *span,
        MirInstruction::Evaluate(expression) => program.expressions.get(expression.0)?.span,
    })
}

impl FunctionCompiler<'_> {
    fn emit(&mut self, operation: BytecodeOp, span: Span) -> usize {
        if matches!(operation, BytecodeOp::Step) {
            if let Some(previous) = self.pending_source_step.replace(span) {
                self.code.push(BytecodeInstruction {
                    operation: BytecodeOp::Step,
                    span: previous,
                    source_step_span: None,
                });
            }
            return self.code.len();
        }
        let id = self.code.len();
        self.code.push(BytecodeInstruction {
            operation,
            span,
            source_step_span: self.pending_source_step.take(),
        });
        id
    }

    fn flush_pending_source_step(&mut self) {
        if let Some(span) = self.pending_source_step.take() {
            self.code.push(BytecodeInstruction {
                operation: BytecodeOp::Step,
                span,
                source_step_span: None,
            });
        }
    }

    fn lambda_expression(
        &mut self,
        params: Vec<(LocalId, String, Type)>,
        body: ExpressionId,
        function_type: Type,
        span: Span,
    ) -> Result<(), Vec<MirError>> {
        let Type::Function {
            params: parameter_types,
            return_type,
            capabilities,
        } = function_type
        else {
            return Err(vec![bytecode_error(
                "typed MIR lambda does not have a function type",
                span,
            )]);
        };
        if params.len() != parameter_types.len()
            || params
                .iter()
                .zip(&parameter_types)
                .any(|((_, _, actual), expected)| actual != expected && *actual != Type::Unknown)
        {
            return Err(vec![bytecode_error(
                "typed MIR lambda parameters do not match its function type",
                span,
            )]);
        }
        let bound = params.iter().map(|(local, _, _)| *local).collect();
        let captures = capture_locals_with_bound(self.program, body, bound)?;
        let synthetic = crate::MirFunction {
            id: FunctionId(usize::MAX.saturating_sub(*self.next_generated_index)),
            name: format!("<lambda:{}>", *self.next_generated_index),
            parameters: params
                .iter()
                .zip(&parameter_types)
                .map(|((local, name, _), ty)| (*local, name.clone(), ty.clone(), span))
                .collect(),
            declared_return_type: Some(*return_type.clone()),
            declared_capabilities: capabilities.clone(),
            requires: Vec::new(),
            ensures: Vec::new(),
            contract_result_local: LocalId(usize::MAX),
            entry: BlockId(0),
            blocks: vec![crate::MirBasicBlock {
                id: BlockId(0),
                instructions: Vec::new(),
                loop_invariants: Vec::new(),
                loop_span: None,
                terminator: Some(MirTerminator::Return(Some(body))),
                span,
            }],
            span,
        };
        let mut function = compile_function(
            self.program,
            &synthetic,
            self.function_indices,
            self.next_generated_index,
            self.generated_functions,
        )?;
        function.captures = captures;
        function.local_mutability.extend(
            function
                .captures
                .iter()
                .map(|(local, _, _)| (*local, false)),
        );
        function.local_mutability.sort_by_key(|(local, _)| local.0);
        let function_index = *self.next_generated_index;
        function.id = FunctionId(function_index);
        function.name = format!("<lambda:{function_index}>");
        *self.next_generated_index = function_index.saturating_add(1);
        let capture_ids = function
            .captures
            .iter()
            .map(|(local, _, _)| *local)
            .collect();
        self.generated_functions.push(function);
        self.emit(
            BytecodeOp::MakeClosure {
                function: function_index,
                captures: capture_ids,
            },
            span,
        );
        Ok(())
    }

    fn parallel_instruction(
        &mut self,
        bindings: &[crate::MirParallelBinding],
        span: Span,
    ) -> Result<(), Vec<MirError>> {
        let mut compiled_bindings = Vec::with_capacity(bindings.len());
        for binding in bindings {
            let Some(expression) = self.program.expressions.get(binding.value.0) else {
                return Err(vec![bytecode_error(
                    "parallel binding expression is outside the MIR expression table",
                    binding.span,
                )]);
            };
            let MirExpressionKind::Await(body) = &expression.kind else {
                return Err(vec![bytecode_error(
                    "parallel binding does not contain a checked await expression",
                    binding.span,
                )]);
            };
            let captures = parallel_captures(self.program, *body)?;
            let result_type = self
                .program
                .expressions
                .get(body.0)
                .ok_or_else(|| {
                    vec![bytecode_error(
                        "parallel await body is outside the MIR expression table",
                        binding.span,
                    )]
                })?
                .ty
                .clone();
            let mut compiler = FunctionCompiler {
                program: self.program,
                function_indices: self.function_indices,
                immutable_function_aliases: HashMap::new(),
                code: Vec::new(),
                blocks: HashMap::new(),
                iterators: HashMap::new(),
                patches: Vec::new(),
                next_match_id: MatchId(0),
                next_local_id: maximum_local_id(self.program).saturating_add(1),
                next_iterator_id: 0,
                contract_result_local: LocalId(0),
                ensures: Vec::new(),
                declared_capabilities: self.declared_capabilities.clone(),
                initialization_writes: Vec::new(),
                generated_locals: Vec::new(),
                next_generated_index: self.next_generated_index,
                generated_functions: self.generated_functions,
                pending_source_step: None,
            };
            compiler.expression(*body, &mut HashSet::new())?;
            compiler.emit(BytecodeOp::Return, binding.span);
            compiler.flush_pending_source_step();
            let mut branch_function = BytecodeFunction {
                id: FunctionId(*compiler.next_generated_index),
                name: format!(
                    "<parallel:{}:{}>",
                    binding.name, *compiler.next_generated_index
                ),
                parameters: captures
                    .iter()
                    .map(|(local, ty, capture_span)| (*local, ty.clone(), *capture_span))
                    .collect(),
                captures: Vec::new(),
                local_mutability: captures
                    .iter()
                    .map(|(local, _, _)| (*local, false))
                    .chain(
                        compiler
                            .generated_locals
                            .iter()
                            .copied()
                            .map(|local| (local, true)),
                    )
                    .collect(),
                initialization_writes: compiler.initialization_writes.clone(),
                return_type: result_type.clone(),
                declared_capabilities: compiler.declared_capabilities.clone(),
                code: std::mem::take(&mut compiler.code),
                max_stack: 0,
            };
            branch_function.max_stack = verify_function(&branch_function, binding.span)?;
            let function_index = *compiler.next_generated_index;
            *compiler.next_generated_index = (*compiler.next_generated_index).saturating_add(1);
            compiled_bindings.push(BytecodeParallelBinding {
                target: binding.local,
                is_new: binding.is_new,
                function: function_index,
                captures: captures.iter().map(|(local, _, _)| *local).collect(),
                result_type,
            });
            compiler.generated_functions.push(branch_function);
        }
        let instruction = self.emit(
            BytecodeOp::Parallel {
                bindings: compiled_bindings,
            },
            span,
        );
        self.initialization_writes.extend(
            bindings
                .iter()
                .filter(|binding| binding.is_new)
                .map(|binding| (instruction, binding.local)),
        );
        Ok(())
    }

    fn enum_payload_types(
        &self,
        enum_type: &Type,
        enum_name: &str,
        variant: &str,
        span: Span,
    ) -> Result<Vec<Type>, Vec<MirError>> {
        let Some(declaration) = self
            .program
            .enums
            .iter()
            .find(|item| item.name == enum_name)
        else {
            return Err(vec![bytecode_error(
                format!("enum `{enum_name}` is not declared"),
                span,
            )]);
        };
        let Some((_, payloads)) = declaration
            .variant_payloads
            .iter()
            .find(|(name, _)| name == variant)
        else {
            if declaration.variants.iter().any(|name| name == variant) {
                return Ok(Vec::new());
            }
            return Err(vec![bytecode_error(
                format!("enum `{enum_name}` has no variant `{variant}`"),
                span,
            )]);
        };
        let (actual_name, args) = record_type_parts(enum_type).ok_or_else(|| {
            vec![bytecode_error(
                "enum constructor has a non-enum result type",
                span,
            )]
        })?;
        if actual_name != enum_name || args.len() != declaration.type_params.len() {
            return Err(vec![bytecode_error(
                "enum constructor result type does not match its declaration",
                span,
            )]);
        }
        let substitutions = declaration
            .type_params
            .iter()
            .cloned()
            .zip(args.iter().cloned())
            .collect::<HashMap<_, _>>();
        Ok(payloads
            .iter()
            .map(|ty| substitute_record_type(ty, &substitutions))
            .collect())
    }

    fn higher_order_call(
        &mut self,
        name: &str,
        arguments: &[ExpressionId],
        result_type: &Type,
        span: Span,
        active: &mut HashSet<ExpressionId>,
    ) -> Result<(), Vec<MirError>> {
        let expected_arguments = if name == "reduce" { 3 } else { 2 };
        if arguments.len() != expected_arguments {
            return Err(vec![bytecode_error(
                format!("`{name}` expects {expected_arguments} arguments"),
                span,
            )]);
        }
        let Type::Array(item_type) = self.expression_type(arguments[0], span)? else {
            return Err(vec![bytecode_error(
                format!("`{name}` requires an array input"),
                self.program.expressions[arguments[0].0].span,
            )]);
        };
        let callback_index = if name == "reduce" { 2 } else { 1 };
        let callback_span = self.program.expressions[arguments[callback_index].0].span;
        let callback_kind = self.program.expressions[arguments[callback_index].0]
            .kind
            .clone();
        let callback_function_id = match &callback_kind {
            MirExpressionKind::FunctionValue { function, .. } => *function,
            MirExpressionKind::Local(local) => self.immutable_function_aliases.get(local).copied(),
            _ => None,
        };
        let callback_type = self.expression_type(arguments[callback_index], span)?;
        let Type::Function {
            params: typed_params,
            return_type: typed_return,
            ..
        } = callback_type
        else {
            return Err(vec![bytecode_error(
                format!("`{name}` callback has no checked function type"),
                callback_span,
            )]);
        };
        let expected_parameters = if name == "reduce" { 2 } else { 1 };
        if typed_params.len() != expected_parameters {
            return Err(vec![bytecode_error(
                format!("`{name}` callback expects {expected_parameters} parameter(s)"),
                callback_span,
            )]);
        }

        let (lambda_params, lambda_body, callback_function, dynamic_callback) = match callback_kind
        {
            MirExpressionKind::Lambda { params, body } => (Some(params), Some(body), None, false),
            _ if callback_function_id.is_some() => {
                let function = callback_function_id.expect("checked function-value callback");
                let Some(target) = self
                    .program
                    .functions
                    .iter()
                    .find(|item| item.id == function)
                else {
                    return Err(vec![bytecode_error(
                        "bytecode function-value callback refers to a missing function",
                        callback_span,
                    )]);
                };
                if target.parameters.len() != expected_parameters
                    || target
                        .parameters
                        .iter()
                        .map(|(_, _, ty, _)| ty)
                        .ne(typed_params.iter())
                    || target.declared_return_type.as_ref() != Some(typed_return.as_ref())
                {
                    return Err(vec![bytecode_error(
                        "bytecode function-value callback signature does not match its checked type",
                        callback_span,
                    )]);
                }
                let Some(function_index) = self.function_indices.get(&function).copied() else {
                    return Err(vec![bytecode_error(
                        "bytecode function-value callback was not included in the call closure",
                        callback_span,
                    )]);
                };
                (None, None, Some(function_index), false)
            }
            _ => (None, None, None, true),
        };

        let accumulator_type = if name == "reduce" {
            self.expression_type(arguments[1], span)?
        } else {
            Type::Unit
        };
        if name == "reduce" {
            if typed_params[0] != accumulator_type || typed_params[1] != *item_type {
                return Err(vec![bytecode_error(
                    "`reduce` callback parameter types do not match the accumulator and array element",
                    callback_span,
                )]);
            }
        } else {
            if typed_params[0] != *item_type {
                return Err(vec![bytecode_error(
                    format!(
                        "`{name}` callback parameter type `{}` does not match array element type `{item_type}`",
                        typed_params[0]
                    ),
                    callback_span,
                )]);
            }
        }
        let body_type = if let Some(body) = lambda_body {
            let body_type = self.expression_type(body, callback_span)?;
            if body_type != *typed_return {
                return Err(vec![bytecode_error(
                    "bytecode lambda body type does not match its checked function type",
                    callback_span,
                )]);
            }
            body_type
        } else {
            typed_return.as_ref().clone()
        };
        match name {
            "map" if result_type == &Type::Array(Box::new(body_type.clone())) => {}
            "filter"
                if body_type == Type::Bool && result_type == &Type::Array(item_type.clone()) => {}
            "find"
                if body_type == Type::Bool && result_type == &Type::Option(item_type.clone()) => {}
            "reduce" if body_type == accumulator_type && result_type == &accumulator_type => {}
            "map" => {
                return Err(vec![bytecode_error(
                    "`map` lambda result does not match the array result type",
                    callback_span,
                )]);
            }
            "filter" | "find" => {
                return Err(vec![bytecode_error(
                    format!("`{name}` lambda must return Bool and have the matching result type"),
                    callback_span,
                )]);
            }
            "reduce" => {
                return Err(vec![bytecode_error(
                    "`reduce` lambda result must match its accumulator type",
                    callback_span,
                )]);
            }
            _ => unreachable!("higher-order call name was checked by the caller"),
        }

        let item_parameter = if let Some(params) = &lambda_params {
            params[expected_parameters - 1].0
        } else {
            self.allocate_local(callback_span)?
        };

        let collection_local = self.allocate_local(span)?;
        self.expression(arguments[0], active)?;
        self.emit(BytecodeOp::StoreLocal(collection_local), span);

        let output_local = self.allocate_local(span)?;
        if name == "reduce" {
            self.expression(arguments[1], active)?;
        } else if name == "find" {
            self.emit(
                BytecodeOp::OptionNone {
                    value_type: item_type.as_ref().clone(),
                },
                span,
            );
        } else {
            self.emit(BytecodeOp::BuildArray { items: 0 }, span);
        }
        self.emit(BytecodeOp::StoreLocal(output_local), span);

        let callback_local = if dynamic_callback {
            let local = self.allocate_local(callback_span)?;
            self.expression(arguments[callback_index], active)?;
            self.emit(BytecodeOp::StoreLocal(local), callback_span);
            Some(local)
        } else {
            None
        };

        let iterator = self.allocate_iterator(span)?;
        self.emit(BytecodeOp::LoadLocal(collection_local), span);
        self.emit(BytecodeOp::CollectionInit { iterator }, span);
        let loop_start = self.code.len();
        let next = self.emit(
            BytecodeOp::CollectionNext {
                iterator,
                locals: vec![item_parameter],
                body: usize::MAX,
                exit: usize::MAX,
            },
            span,
        );
        self.initialization_writes.push((next, item_parameter));
        let body_start = self.code.len();
        if let Some(function) = callback_function {
            if name == "reduce" {
                self.emit(BytecodeOp::LoadLocal(output_local), span);
            }
            self.emit(BytecodeOp::LoadLocal(item_parameter), span);
            self.emit(
                BytecodeOp::Call {
                    function,
                    arguments: expected_parameters,
                },
                callback_span,
            );
        } else if let Some(callback_local) = callback_local {
            self.emit(BytecodeOp::LoadLocal(callback_local), callback_span);
            if name == "reduce" {
                self.emit(BytecodeOp::LoadLocal(output_local), span);
            }
            self.emit(BytecodeOp::LoadLocal(item_parameter), span);
            self.emit(
                BytecodeOp::CallValue {
                    arguments: expected_parameters,
                },
                callback_span,
            );
        } else {
            if name == "reduce" {
                let params = lambda_params
                    .as_ref()
                    .expect("inline lambda callback retains its parameters");
                self.emit(BytecodeOp::LoadLocal(output_local), span);
                let initialize_accumulator = self.emit(BytecodeOp::StoreLocal(params[0].0), span);
                self.initialization_writes
                    .push((initialize_accumulator, params[0].0));
            }
            let body = lambda_body.expect("inline lambda callback retains its body");
            self.expression(body, active)?;
        }

        let mut exit_edges = Vec::new();
        match name {
            "map" => {
                self.emit(
                    BytecodeOp::ArrayAppend {
                        local: output_local,
                        element_type: body_type,
                    },
                    callback_span,
                );
            }
            "filter" => {
                let skip = self.emit(BytecodeOp::JumpIfFalse(usize::MAX), callback_span);
                self.emit(BytecodeOp::LoadLocal(item_parameter), callback_span);
                self.emit(
                    BytecodeOp::ArrayAppend {
                        local: output_local,
                        element_type: *item_type,
                    },
                    callback_span,
                );
                let next_item = self.code.len();
                self.code[skip].operation = BytecodeOp::JumpIfFalse(next_item);
            }
            "find" => {
                let not_found = self.emit(BytecodeOp::JumpIfFalse(usize::MAX), callback_span);
                self.emit(BytecodeOp::LoadLocal(item_parameter), callback_span);
                self.emit(
                    BytecodeOp::OptionSome {
                        value_type: item_type.as_ref().clone(),
                    },
                    callback_span,
                );
                self.emit(BytecodeOp::StoreLocal(output_local), callback_span);
                exit_edges.push(self.emit(BytecodeOp::Jump(usize::MAX), callback_span));
                let next_item = self.code.len();
                self.code[not_found].operation = BytecodeOp::JumpIfFalse(next_item);
            }
            "reduce" => {
                self.emit(BytecodeOp::StoreLocal(output_local), callback_span);
            }
            _ => unreachable!("higher-order call name was checked by the caller"),
        }
        self.emit(BytecodeOp::Jump(loop_start), span);
        let loop_exit = self.code.len();
        self.code[next].operation = BytecodeOp::CollectionNext {
            iterator,
            locals: vec![item_parameter],
            body: body_start,
            exit: loop_exit,
        };
        for branch in exit_edges {
            self.code[branch].operation = BytecodeOp::Jump(loop_exit);
        }
        self.emit(BytecodeOp::LoadLocal(output_local), span);
        Ok(())
    }

    fn record_update_call(
        &mut self,
        type_args: &[Type],
        arguments: &[ExpressionId],
        result_type: &Type,
        span: Span,
        active: &mut HashSet<ExpressionId>,
    ) -> Result<(), Vec<MirError>> {
        if type_args.len() != 1 || arguments.is_empty() || arguments.len().is_multiple_of(2) {
            return Err(vec![bytecode_error(
                "record update expects a base record followed by field/value pairs",
                span,
            )]);
        }
        let record_type = type_args[0].clone();
        if &record_type != result_type {
            return Err(vec![bytecode_error(
                "record update result type does not match its declared record type",
                span,
            )]);
        }
        let (record_name, type_arguments) = record_type_parts(&record_type)
            .ok_or_else(|| vec![bytecode_error("record update has a non-record type", span)])?;
        let declaration = self
            .program
            .records
            .iter()
            .find(|record| record.name == record_name)
            .ok_or_else(|| {
                vec![bytecode_error(
                    format!("typed MIR record `{record_name}` is not declared"),
                    span,
                )]
            })?;
        if declaration.is_class || declaration.type_params.len() != type_arguments.len() {
            return Err(vec![bytecode_error(
                "record update cannot copy a class reference or malformed generic record",
                span,
            )]);
        }
        if self.expression_type(arguments[0], span)? != record_type {
            return Err(vec![bytecode_error(
                "record update base has a mismatched record type",
                self.program.expressions[arguments[0].0].span,
            )]);
        }
        let substitutions = declaration
            .type_params
            .iter()
            .cloned()
            .zip(type_arguments.iter().cloned())
            .collect::<HashMap<_, _>>();
        let mut fields = Vec::with_capacity((arguments.len() - 1) / 2);
        let mut seen = HashSet::new();
        for pair in arguments[1..].chunks(2) {
            let field_span = self.program.expressions[pair[0].0].span;
            let Some(MirExpressionKind::String(field_name)) = self
                .program
                .expressions
                .get(pair[0].0)
                .map(|expression| &expression.kind)
            else {
                return Err(vec![bytecode_error(
                    "record update field name must be a checked string literal",
                    field_span,
                )]);
            };
            if !seen.insert(field_name.clone()) {
                return Err(vec![bytecode_error(
                    format!("record update repeats field `{field_name}`"),
                    field_span,
                )]);
            }
            let field = declaration
                .fields
                .iter()
                .find(|field| field.name == *field_name)
                .ok_or_else(|| {
                    vec![bytecode_error(
                        format!("record `{record_name}` has no field `{field_name}`"),
                        field_span,
                    )]
                })?;
            let expected = substitute_record_type(&field.ty, &substitutions);
            let actual = self.expression_type(pair[1], span)?;
            if actual != expected {
                return Err(vec![bytecode_error(
                    format!(
                        "record update field `{field_name}` expects `{expected}`, found `{actual}`"
                    ),
                    self.program.expressions[pair[1].0].span,
                )]);
            }
            fields.push((field_name.clone(), expected));
        }
        self.expression(arguments[0], active)?;
        for pair in arguments[1..].chunks(2) {
            if self.expression_type(pair[0], span)? != Type::String {
                return Err(vec![bytecode_error(
                    "record update field name must have String type",
                    self.program.expressions[pair[0].0].span,
                )]);
            }
            self.expression(pair[0], active)?;
            self.expression(pair[1], active)?;
        }
        self.emit(
            BytecodeOp::RecordUpdate {
                record_type,
                fields,
            },
            span,
        );
        Ok(())
    }

    fn allocate_local(&mut self, span: Span) -> Result<LocalId, Vec<MirError>> {
        let id = self.next_local_id;
        self.next_local_id = id.checked_add(1).ok_or_else(|| {
            vec![bytecode_error(
                "bytecode temporary local identifier overflow",
                span,
            )]
        })?;
        let local = LocalId(id);
        self.generated_locals.push(local);
        Ok(local)
    }

    fn allocate_iterator(&mut self, span: Span) -> Result<IteratorId, Vec<MirError>> {
        let id = self.next_iterator_id;
        self.next_iterator_id = id.checked_add(1).ok_or_else(|| {
            vec![bytecode_error(
                "bytecode iterator identifier overflow",
                span,
            )]
        })?;
        Ok(IteratorId(id))
    }

    fn match_expression(
        &mut self,
        value: ExpressionId,
        arms: &[crate::MirMatchExpressionArm],
        span: Span,
        active: &mut HashSet<ExpressionId>,
    ) -> Result<(), Vec<MirError>> {
        let match_id = self.next_match_id;
        self.next_match_id.0 = self
            .next_match_id
            .0
            .checked_add(1)
            .ok_or_else(|| vec![bytecode_error("bytecode match identifier overflow", span)])?;
        self.expression(value, active)?;
        self.emit(BytecodeOp::MatchInit { match_id }, span);
        let mut tests = Vec::with_capacity(arms.len());
        let mut guard_branches = Vec::new();
        let mut result_branches = Vec::new();
        for arm in arms {
            let test = self.emit(
                BytecodeOp::MatchTest {
                    match_id,
                    pattern: arm.pattern.clone(),
                    matched: usize::MAX,
                    unmatched: usize::MAX,
                },
                arm.pattern.span,
            );
            let mut bound_locals = Vec::new();
            mir_pattern_bound_locals(&arm.pattern, &mut bound_locals);
            self.initialization_writes
                .extend(bound_locals.into_iter().map(|local| (test, local)));
            tests.push(test);
            if let Some(guard) = arm.guard {
                self.expression(guard, active)?;
                let branch = self.emit(BytecodeOp::JumpIfFalse(usize::MAX), arm.span);
                guard_branches.push((branch, tests.len() - 1));
            }
            self.expression(arm.value, active)?;
            result_branches.push(self.emit(BytecodeOp::Jump(usize::MAX), arm.span));
        }
        let failure = self.emit(BytecodeOp::MatchFailure, span);
        for (index, test) in tests.iter().copied().enumerate() {
            let matched = test + 1;
            let unmatched = tests.get(index + 1).copied().unwrap_or(failure);
            let BytecodeOp::MatchTest {
                matched: matched_slot,
                unmatched: unmatched_slot,
                ..
            } = &mut self.code[test].operation
            else {
                unreachable!("match test index identifies a match-test instruction")
            };
            *matched_slot = matched;
            *unmatched_slot = unmatched;
        }
        // Each guard-false edge advances to the next arm, or to the terminal
        // failure instruction for the last arm.
        for (branch, guarded_arm) in guard_branches {
            let target = tests.get(guarded_arm + 1).copied().unwrap_or(failure);
            self.code[branch].operation = BytecodeOp::JumpIfFalse(target);
        }
        let end = self.code.len();
        for branch in result_branches {
            self.code[branch].operation = BytecodeOp::Jump(end);
        }
        Ok(())
    }

    fn instruction(&mut self, instruction: &MirInstruction) -> Result<(), Vec<MirError>> {
        match instruction {
            MirInstruction::SourceStep { span } => {
                self.emit(BytecodeOp::Step, *span);
            }
            MirInstruction::IteratorInit {
                iterator,
                source: MirIteratorSource::IntegerRange { start, end, step },
                span,
            } => {
                self.expression(*start, &mut HashSet::new())?;
                self.expression(*end, &mut HashSet::new())?;
                if let Some(step) = step {
                    self.expression(*step, &mut HashSet::new())?;
                } else {
                    self.emit(BytecodeOp::Constant(Value::Int(1)), *span);
                }
                self.emit(
                    BytecodeOp::RangeInit {
                        iterator: *iterator,
                    },
                    *span,
                );
                self.iterators
                    .insert(*iterator, BytecodeIteratorKind::Range);
            }
            MirInstruction::IteratorInit {
                iterator,
                source: MirIteratorSource::Collection(expression),
                span,
            } => {
                self.expression(*expression, &mut HashSet::new())?;
                self.emit(
                    BytecodeOp::CollectionInit {
                        iterator: *iterator,
                    },
                    *span,
                );
                self.iterators
                    .insert(*iterator, BytecodeIteratorKind::Collection);
            }
            MirInstruction::MatchInit {
                match_id,
                value,
                span,
            } => {
                self.expression(*value, &mut HashSet::new())?;
                self.emit(
                    BytecodeOp::MatchInit {
                        match_id: *match_id,
                    },
                    *span,
                );
            }
            MirInstruction::Let {
                local, value, span, ..
            } => {
                self.expression(*value, &mut HashSet::new())?;
                let instruction = self.emit(BytecodeOp::StoreLocal(*local), *span);
                self.initialization_writes.push((instruction, *local));
            }
            MirInstruction::Assign {
                local, value, span, ..
            } => {
                self.expression(*value, &mut HashSet::new())?;
                self.emit(BytecodeOp::StoreLocal(*local), *span);
            }
            MirInstruction::ArrayIndexAssign {
                local,
                index,
                value,
                operation,
                span,
                ..
            } => {
                self.expression(*index, &mut HashSet::new())?;
                self.expression(*value, &mut HashSet::new())?;
                self.emit(
                    BytecodeOp::ArrayIndexAssign {
                        local: *local,
                        operation: *operation,
                    },
                    *span,
                );
            }
            MirInstruction::TupleBind {
                bindings,
                value,
                span,
            } => {
                if bindings.is_empty() {
                    return Err(vec![bytecode_error(
                        "bytecode tuple binding has no destinations",
                        *span,
                    )]);
                }
                self.expression(*value, &mut HashSet::new())?;
                let instruction = self.emit(
                    BytecodeOp::TupleBind {
                        locals: bindings.iter().map(|binding| binding.local).collect(),
                    },
                    *span,
                );
                self.initialization_writes.extend(
                    bindings
                        .iter()
                        .filter(|binding| binding.is_new)
                        .map(|binding| (instruction, binding.local)),
                );
            }
            MirInstruction::Parallel { bindings, span } => {
                self.parallel_instruction(bindings, *span)?;
            }
            MirInstruction::Transaction { operations, span } => {
                let mut transaction_operations = Vec::with_capacity(operations.len());
                for operation in operations {
                    let Some(expression) = self.program.expressions.get(operation.0) else {
                        return Err(vec![bytecode_error(
                            "bytecode transaction references a missing MIR expression",
                            *span,
                        )]);
                    };
                    match &expression.kind {
                        MirExpressionKind::Sql {
                            query,
                            parameters,
                            result_type,
                            ..
                        } => transaction_operations.push(BytecodeTransactionOperation::Sql(
                            BytecodeSqlStatement {
                                query: query.clone(),
                                parameters: parameters.clone(),
                                result_type: result_type.clone(),
                            },
                        )),
                        MirExpressionKind::Call { name, args, .. }
                            if name == "mail_queue" && matches!(args.len(), 3..=6) =>
                        {
                            if !args.iter().all(|argument| {
                                is_pure_mail_mir_expression(self.program, *argument)
                            }) {
                                return Err(vec![bytecode_error(
                                    "bytecode mail_queue arguments must be pure expressions",
                                    expression.span,
                                )]);
                            }
                            self.emit(
                                BytecodeOp::HostCallPreflight {
                                    name: "mail_queue".into(),
                                    arguments: 0,
                                },
                                expression.span,
                            );
                            self.emit(
                                BytecodeOp::HostCallPreflight {
                                    name: "mail_queue_db_write".into(),
                                    arguments: 0,
                                },
                                expression.span,
                            );
                            let mut argument_spans = Vec::with_capacity(args.len());
                            for argument in args {
                                let Some(value) = self.program.expressions.get(argument.0) else {
                                    return Err(vec![bytecode_error(
                                        "bytecode mail_queue references a missing argument expression",
                                        expression.span,
                                    )]);
                                };
                                argument_spans.push(value.span);
                                self.expression(*argument, &mut HashSet::new())?;
                            }
                            transaction_operations.push(BytecodeTransactionOperation::MailQueue {
                                arguments: args.len(),
                                argument_spans,
                                call_span: expression.span,
                            });
                        }
                        _ => {
                            return Err(vec![bytecode_error(
                                "bytecode v21 transactions support SQL statements and 3-to-6-argument mail_queue calls",
                                expression.span,
                            )]);
                        }
                    }
                }
                if transaction_operations.is_empty() {
                    return Err(vec![bytecode_error(
                        "bytecode transaction must contain at least one SQL statement",
                        *span,
                    )]);
                }
                self.emit(
                    BytecodeOp::SqlTransaction {
                        operations: transaction_operations,
                    },
                    *span,
                );
            }
            MirInstruction::Evaluate(expression) => {
                let span = self.program.expressions[expression.0].span;
                self.expression(*expression, &mut HashSet::new())?;
                self.emit(BytecodeOp::Pop, span);
            }
            MirInstruction::LoopInvariantCheck {
                expressions,
                phase,
                span,
            } => {
                for expression in expressions {
                    self.expression(*expression, &mut HashSet::new())?;
                    self.emit(BytecodeOp::LoopInvariantCheck { phase: *phase }, *span);
                }
            }
        }
        Ok(())
    }

    fn terminator(
        &mut self,
        terminator: Option<&MirTerminator>,
        _block: BlockId,
        span: Span,
    ) -> Result<(), Vec<MirError>> {
        match terminator {
            Some(MirTerminator::Goto(target)) => {
                let id = self.emit(BytecodeOp::Jump(usize::MAX), span);
                self.patches.push((id, *target, false));
            }
            Some(MirTerminator::Branch {
                condition,
                then_block,
                else_block,
            }) => {
                self.expression(*condition, &mut HashSet::new())?;
                let otherwise = self.emit(BytecodeOp::JumpIfFalse(usize::MAX), span);
                self.patches.push((otherwise, *else_block, false));
                let then = self.emit(BytecodeOp::Jump(usize::MAX), span);
                self.patches.push((then, *then_block, false));
            }
            Some(MirTerminator::IteratorNext {
                iterator,
                bindings,
                body_block,
                exit_block,
            }) => {
                let Some(kind) = self.iterators.get(iterator).copied() else {
                    return Err(vec![bytecode_error(
                        "bytecode iterator initialization is missing",
                        span,
                    )]);
                };
                if bindings.is_empty() {
                    return Err(vec![bytecode_error(
                        "bytecode collection loops require at least one binding",
                        span,
                    )]);
                }
                if kind == BytecodeIteratorKind::Range && bindings.len() != 1 {
                    return Err(vec![bytecode_error(
                        "bytecode integer range loops require exactly one binding",
                        span,
                    )]);
                }
                let locals: Vec<LocalId> = bindings.iter().map(|binding| binding.local).collect();
                let operation = match kind {
                    BytecodeIteratorKind::Range => BytecodeOp::RangeNext {
                        iterator: *iterator,
                        local: locals[0],
                        body: usize::MAX,
                        exit: usize::MAX,
                    },
                    BytecodeIteratorKind::Collection => BytecodeOp::CollectionNext {
                        iterator: *iterator,
                        locals,
                        body: usize::MAX,
                        exit: usize::MAX,
                    },
                };
                let id = self.emit(operation, span);
                self.initialization_writes
                    .extend(bindings.iter().map(|binding| (id, binding.local)));
                self.patches.push((id, *body_block, false));
                self.patches.push((id, *exit_block, true));
            }
            Some(MirTerminator::MatchTest {
                match_id,
                pattern,
                matched_block,
                unmatched_block,
            }) => {
                let id = self.emit(
                    BytecodeOp::MatchTest {
                        match_id: *match_id,
                        pattern: pattern.clone(),
                        matched: usize::MAX,
                        unmatched: usize::MAX,
                    },
                    pattern.span,
                );
                let mut bound_locals = Vec::new();
                mir_pattern_bound_locals(pattern, &mut bound_locals);
                self.initialization_writes
                    .extend(bound_locals.into_iter().map(|local| (id, local)));
                self.patches.push((id, *matched_block, false));
                self.patches.push((id, *unmatched_block, true));
            }
            Some(MirTerminator::MatchFailure { span }) => {
                self.emit(BytecodeOp::MatchFailure, *span);
            }
            Some(MirTerminator::Return(value)) => {
                if let Some(value) = value {
                    self.expression(*value, &mut HashSet::new())?;
                } else {
                    self.emit(BytecodeOp::Constant(Value::Unit), span);
                }
                self.emit(BytecodeOp::StoreLocal(self.contract_result_local), span);
                for contract in self.ensures.clone() {
                    let contract_span = self
                        .program
                        .expressions
                        .get(contract.0)
                        .map_or(span, |item| item.span);
                    self.expression(contract, &mut HashSet::new())?;
                    self.emit(
                        BytecodeOp::ContractCheck {
                            postcondition: true,
                        },
                        contract_span,
                    );
                }
                self.emit(BytecodeOp::LoadLocal(self.contract_result_local), span);
                self.emit(BytecodeOp::Return, span);
            }
            Some(MirTerminator::ReturnAfterLoopInvariants { value, invariants }) => {
                if let Some(value) = value {
                    self.expression(*value, &mut HashSet::new())?;
                } else {
                    self.emit(BytecodeOp::Constant(Value::Unit), span);
                }
                self.emit(BytecodeOp::StoreLocal(self.contract_result_local), span);
                for invariant in invariants {
                    self.expression(invariant.expression, &mut HashSet::new())?;
                    self.emit(
                        BytecodeOp::LoopInvariantCheck {
                            phase: LoopInvariantPhase::After,
                        },
                        invariant.loop_span,
                    );
                }
                for contract in self.ensures.clone() {
                    let contract_span = self
                        .program
                        .expressions
                        .get(contract.0)
                        .map_or(span, |item| item.span);
                    self.expression(contract, &mut HashSet::new())?;
                    self.emit(
                        BytecodeOp::ContractCheck {
                            postcondition: true,
                        },
                        contract_span,
                    );
                }
                self.emit(BytecodeOp::LoadLocal(self.contract_result_local), span);
                self.emit(BytecodeOp::Return, span);
            }
            Some(MirTerminator::Unreachable) => {
                self.emit(BytecodeOp::Trap, span);
            }
            None => {
                return Err(vec![bytecode_error(
                    "bytecode backend requires a supported, terminated basic block",
                    span,
                )]);
            }
        }
        Ok(())
    }

    fn expression(
        &mut self,
        id: ExpressionId,
        active: &mut HashSet<ExpressionId>,
    ) -> Result<(), Vec<MirError>> {
        if !active.insert(id) {
            return Err(vec![bytecode_error(
                "cyclic MIR expression graph cannot be lowered to bytecode",
                self.program
                    .expressions
                    .get(id.0)
                    .map_or(Span::default(), |expression| expression.span),
            )]);
        }
        let Some(expression) = self.program.expressions.get(id.0) else {
            return Err(vec![bytecode_error(
                "MIR expression id is outside the expression table",
                Span::default(),
            )]);
        };
        let span = expression.span;
        self.emit(BytecodeOp::Step, span);
        let expression_type = expression.ty.clone();
        let kind = expression.kind.clone();
        let result = match kind {
            MirExpressionKind::Int(value) => {
                self.emit(BytecodeOp::Constant(Value::Int(value)), span)
            }
            MirExpressionKind::UInt(value) => {
                self.emit(BytecodeOp::Constant(Value::UInt(value)), span)
            }
            MirExpressionKind::Float(value) => {
                self.emit(BytecodeOp::Constant(Value::Float(value)), span)
            }
            MirExpressionKind::Bool(value) => {
                self.emit(BytecodeOp::Constant(Value::Bool(value)), span)
            }
            MirExpressionKind::String(value) => {
                self.emit(BytecodeOp::Constant(Value::String(value)), span)
            }
            MirExpressionKind::Char(value) => {
                self.emit(BytecodeOp::Constant(Value::Char(value)), span)
            }
            MirExpressionKind::EnumVariant { enum_name, variant } => {
                let payload_types =
                    self.enum_payload_types(&expression_type, &enum_name, &variant, span)?;
                if !payload_types.is_empty() {
                    return Err(vec![bytecode_error(
                        format!("enum variant `{enum_name}::{variant}` requires payload values"),
                        span,
                    )]);
                }
                self.emit(
                    BytecodeOp::BuildEnum {
                        enum_type: expression_type,
                        variant,
                        payload_types,
                    },
                    span,
                )
            }
            MirExpressionKind::Record {
                type_name,
                type_args,
                fields,
            } => {
                let record = self
                    .program
                    .records
                    .iter()
                    .find(|record| record.name == type_name)
                    .ok_or_else(|| {
                        vec![bytecode_error(
                            format!("typed MIR record `{type_name}` is not declared"),
                            span,
                        )]
                    })?;
                let (result_name, inferred_type_args) = record_type_parts(&expression_type)
                    .ok_or_else(|| {
                        vec![bytecode_error(
                            "typed MIR record expression has a non-record result type",
                            span,
                        )]
                    })?;
                if result_name != type_name {
                    return Err(vec![bytecode_error(
                        "typed MIR record expression names a different result record",
                        span,
                    )]);
                }
                let resolved_type_args = if type_args.is_empty() && !inferred_type_args.is_empty() {
                    inferred_type_args.to_vec()
                } else {
                    type_args
                };
                if record.type_params.len() != resolved_type_args.len() {
                    return Err(vec![bytecode_error(
                        format!("typed MIR record `{type_name}` has incorrect type argument count"),
                        span,
                    )]);
                }
                let record_type = named_record_type(&type_name, &resolved_type_args);
                if expression_type != record_type {
                    return Err(vec![bytecode_error(
                        "typed MIR record expression has a mismatched result type",
                        span,
                    )]);
                }
                if fields.len() != record.fields.len() {
                    return Err(vec![bytecode_error(
                        "typed MIR record expression does not provide every declared field",
                        span,
                    )]);
                }
                let substitutions = record
                    .type_params
                    .iter()
                    .cloned()
                    .zip(resolved_type_args.iter().cloned())
                    .collect::<HashMap<_, _>>();
                let mut seen = HashSet::new();
                let mut encoded_fields = Vec::with_capacity(fields.len());
                for (field, value) in &fields {
                    if !seen.insert(field.clone()) {
                        return Err(vec![bytecode_error(
                            format!("typed MIR record repeats field `{field}`"),
                            span,
                        )]);
                    }
                    let Some(declaration) = record.fields.iter().find(|item| item.name == *field)
                    else {
                        return Err(vec![bytecode_error(
                            format!("typed MIR record `{type_name}` has no field `{field}`"),
                            span,
                        )]);
                    };
                    let expected = substitute_record_type(&declaration.ty, &substitutions);
                    if self.expression_type(*value, span)? != expected {
                        return Err(vec![bytecode_error(
                            format!("typed MIR record field `{field}` has a mismatched type"),
                            span,
                        )]);
                    }
                    self.expression(*value, active)?;
                    encoded_fields.push((field.clone(), expected));
                }
                self.emit(
                    BytecodeOp::BuildRecord {
                        record_type,
                        fields: encoded_fields,
                    },
                    span,
                )
            }
            MirExpressionKind::Array(items) => {
                let Type::Array(element_type) = &expression_type else {
                    return Err(vec![bytecode_error(
                        "typed MIR array literal does not have an array type",
                        span,
                    )]);
                };
                for item in &items {
                    if self.expression_type(*item, span)? != **element_type {
                        return Err(vec![bytecode_error(
                            "typed MIR array literal contains an item with a mismatched type",
                            span,
                        )]);
                    }
                    self.expression(*item, active)?;
                }
                self.emit(BytecodeOp::BuildArray { items: items.len() }, span)
            }
            MirExpressionKind::Sql {
                result_type,
                query,
                parameters,
                ..
            } => self.emit(
                BytecodeOp::Sql {
                    query,
                    parameters,
                    result_type,
                },
                span,
            ),
            MirExpressionKind::Map(entries) => {
                let Type::Map(key_type, value_type) = &expression_type else {
                    return Err(vec![bytecode_error(
                        "typed MIR map literal does not have a Map type",
                        span,
                    )]);
                };
                for (key, value) in &entries {
                    if self.expression_type(*key, span)? != **key_type
                        || self.expression_type(*value, span)? != **value_type
                    {
                        return Err(vec![bytecode_error(
                            "typed MIR map literal contains an entry with a mismatched type",
                            span,
                        )]);
                    }
                    self.expression(*key, active)?;
                    self.expression(*value, active)?;
                }
                self.emit(
                    BytecodeOp::BuildMap {
                        entries: entries.len(),
                        key_type: key_type.as_ref().clone(),
                        value_type: value_type.as_ref().clone(),
                    },
                    span,
                )
            }
            MirExpressionKind::Local(local) => self.emit(BytecodeOp::LoadLocal(local), span),
            MirExpressionKind::Field { target, field } => {
                let target_type = self.expression_type(target, span)?;
                let (type_name, type_args) = record_type_parts(&target_type).ok_or_else(|| {
                    vec![bytecode_error(
                        format!("bytecode field access is unsupported for `{target_type}`"),
                        span,
                    )]
                })?;
                let record = self
                    .program
                    .records
                    .iter()
                    .find(|record| record.name == type_name)
                    .ok_or_else(|| {
                        vec![bytecode_error(
                            format!("typed MIR record `{type_name}` is not declared"),
                            span,
                        )]
                    })?;
                if record.type_params.len() != type_args.len() {
                    return Err(vec![bytecode_error(
                        format!("typed MIR record `{type_name}` has incorrect type argument count"),
                        span,
                    )]);
                }
                let declaration = record
                    .fields
                    .iter()
                    .find(|item| item.name == field)
                    .ok_or_else(|| {
                        vec![bytecode_error(
                            format!("typed MIR record `{type_name}` has no field `{field}`"),
                            span,
                        )]
                    })?;
                let substitutions = record
                    .type_params
                    .iter()
                    .cloned()
                    .zip(type_args.iter().cloned())
                    .collect::<HashMap<_, _>>();
                let result_type = substitute_record_type(&declaration.ty, &substitutions);
                if expression_type != result_type {
                    return Err(vec![bytecode_error(
                        format!("typed MIR field `{field}` has a mismatched result type"),
                        span,
                    )]);
                }
                self.expression(target, active)?;
                self.emit(
                    BytecodeOp::GetField {
                        record_type: target_type,
                        field,
                        result_type,
                    },
                    span,
                )
            }
            MirExpressionKind::Index { target, index } => {
                let target_type = self.expression_type(target, span)?;
                let index_type = self.expression_type(index, span)?;
                let result_type = match &target_type {
                    Type::Array(element) => {
                        if !matches!(index_type, Type::Int | Type::UInt) {
                            return Err(vec![bytecode_error(
                                "bytecode array index must have type Int or UInt",
                                span,
                            )]);
                        }
                        element.as_ref().clone()
                    }
                    Type::Tuple(items) => {
                        if !matches!(index_type, Type::Int | Type::UInt) {
                            return Err(vec![bytecode_error(
                                "bytecode tuple index must have type Int or UInt",
                                span,
                            )]);
                        }
                        let tuple_index = match self
                            .program
                            .expressions
                            .get(index.0)
                            .map(|index| &index.kind)
                        {
                            Some(MirExpressionKind::Int(value)) if *value >= 0 => {
                                usize::try_from(*value).ok()
                            }
                            Some(MirExpressionKind::UInt(value)) => usize::try_from(*value).ok(),
                            _ => None,
                        };
                        let Some(tuple_index) = tuple_index else {
                            return Err(vec![bytecode_error(
                                "bytecode tuple index must be a non-negative integer literal",
                                span,
                            )]);
                        };
                        items.get(tuple_index).cloned().ok_or_else(|| {
                            vec![bytecode_error(
                                "bytecode tuple index is outside the tuple type",
                                span,
                            )]
                        })?
                    }
                    _ => {
                        return Err(vec![bytecode_error(
                            format!("bytecode backend does not support indexing `{target_type}`"),
                            span,
                        )]);
                    }
                };
                if result_type != expression_type {
                    return Err(vec![bytecode_error(
                        "typed MIR index result does not match the indexed type",
                        span,
                    )]);
                }
                self.expression(target, active)?;
                self.expression(index, active)?;
                self.emit(BytecodeOp::Index, span)
            }
            MirExpressionKind::Unary { op, expr } => {
                self.expression(expr, active)?;
                self.emit(BytecodeOp::Unary(op), span)
            }
            MirExpressionKind::Binary { left, op, right } => {
                let result_type = self.binary_result_type(left, op, right, span)?;
                if result_type != expression_type {
                    return Err(vec![bytecode_error(
                        "typed MIR binary result does not match its inferred result type",
                        span,
                    )]);
                }
                if matches!(op, BinaryOp::And | BinaryOp::Or) {
                    self.expression(left, active)?;
                    let branch = if op == BinaryOp::And {
                        BytecodeOp::JumpIfFalse(usize::MAX)
                    } else {
                        BytecodeOp::JumpIfTrue(usize::MAX)
                    };
                    let short_circuit = self.emit(branch, span);
                    self.expression(right, active)?;
                    let join = self.emit(BytecodeOp::Jump(usize::MAX), span);
                    let constant = self.code.len();
                    self.code[short_circuit].operation = if op == BinaryOp::And {
                        BytecodeOp::JumpIfFalse(constant)
                    } else {
                        BytecodeOp::JumpIfTrue(constant)
                    };
                    self.emit(BytecodeOp::Constant(Value::Bool(op == BinaryOp::Or)), span);
                    let end = self.code.len();
                    self.code[join].operation = BytecodeOp::Jump(end);
                } else {
                    self.expression(left, active)?;
                    self.expression(right, active)?;
                    self.emit(BytecodeOp::Binary(op), span);
                }
                self.code.len()
            }
            MirExpressionKind::BinaryChain { first, operations } => {
                let mut current_type = self.expression_type(first, span)?;
                self.expression(first, active)?;
                for (op, right, operation_span) in operations {
                    current_type = self.binary_result_type_from_types(
                        &current_type,
                        op,
                        &self.expression_type(right, span)?,
                        operation_span,
                    )?;
                    if matches!(op, BinaryOp::And | BinaryOp::Or) {
                        let branch = if op == BinaryOp::And {
                            BytecodeOp::JumpIfFalse(usize::MAX)
                        } else {
                            BytecodeOp::JumpIfTrue(usize::MAX)
                        };
                        let short_circuit = self.emit(branch, operation_span);
                        self.expression(right, active)?;
                        let join = self.emit(BytecodeOp::Jump(usize::MAX), operation_span);
                        let constant = self.code.len();
                        self.code[short_circuit].operation = if op == BinaryOp::And {
                            BytecodeOp::JumpIfFalse(constant)
                        } else {
                            BytecodeOp::JumpIfTrue(constant)
                        };
                        self.emit(
                            BytecodeOp::Constant(Value::Bool(op == BinaryOp::Or)),
                            operation_span,
                        );
                        let end = self.code.len();
                        self.code[join].operation = BytecodeOp::Jump(end);
                    } else {
                        self.expression(right, active)?;
                        self.emit(BytecodeOp::Binary(op), operation_span);
                    }
                }
                if current_type != expression_type {
                    return Err(vec![bytecode_error(
                        "typed MIR chain result does not match its inferred result type",
                        span,
                    )]);
                }
                self.code.len()
            }
            MirExpressionKind::Match { value, arms } => {
                self.match_expression(value, &arms, span, active)?;
                self.code.len()
            }
            MirExpressionKind::Call {
                callee_local: Some(callee_local),
                function: None,
                type_args,
                args,
                ..
            } => {
                if !type_args.is_empty() {
                    return Err(vec![bytecode_error(
                        "generic function values cannot be called with explicit type arguments",
                        span,
                    )]);
                }
                self.emit(BytecodeOp::LoadLocal(callee_local), span);
                for argument in &args {
                    self.expression(*argument, active)?;
                }
                self.emit(
                    BytecodeOp::CallValue {
                        arguments: args.len(),
                    },
                    span,
                )
            }
            MirExpressionKind::Call {
                function: Some(function),
                args,
                ..
            } => {
                let Some(target) = self.function_indices.get(&function).copied() else {
                    return Err(vec![bytecode_error(
                        "call target was not included in bytecode closure",
                        span,
                    )]);
                };
                for argument in &args {
                    self.expression(*argument, active)?;
                }
                self.emit(
                    BytecodeOp::Call {
                        function: target,
                        arguments: args.len(),
                    },
                    span,
                )
            }
            MirExpressionKind::FunctionValue {
                function: Some(function),
                ..
            } => {
                let Some(function) = self.function_indices.get(&function).copied() else {
                    return Err(vec![bytecode_error(
                        "function value target was not included in bytecode closure",
                        span,
                    )]);
                };
                self.emit(BytecodeOp::FunctionRef { function }, span)
            }
            MirExpressionKind::Lambda { params, body } => {
                self.lambda_expression(params, body, expression_type, span)?;
                self.code.len()
            }
            MirExpressionKind::Call {
                name,
                type_args,
                args,
                ..
            } if name == "record_update" => {
                self.record_update_call(&type_args, &args, &expression_type, span, active)?;
                self.code.len()
            }
            MirExpressionKind::Call {
                name,
                function: None,
                args,
                ..
            } if name == "tuple" => {
                let Type::Tuple(item_types) = &expression_type else {
                    return Err(vec![bytecode_error(
                        "typed MIR tuple constructor does not have a tuple result type",
                        span,
                    )]);
                };
                if args.len() != item_types.len() {
                    return Err(vec![bytecode_error(
                        "typed MIR tuple constructor arity does not match its result type",
                        span,
                    )]);
                }
                for (argument, expected) in args.iter().zip(item_types) {
                    if self.expression_type(*argument, span)? != *expected {
                        return Err(vec![bytecode_error(
                            "typed MIR tuple constructor argument has a mismatched type",
                            span,
                        )]);
                    }
                    self.expression(*argument, active)?;
                }
                self.emit(BytecodeOp::BuildTuple { items: args.len() }, span)
            }
            MirExpressionKind::Call {
                name,
                function: None,
                type_args,
                args,
                ..
            } if name == "render_pdf" => {
                if !type_args.is_empty() || args.len() != 2 || expression_type != Type::Bytes {
                    return Err(vec![bytecode_error(
                        "render_pdf requires a view name and data and must return Bytes",
                        span,
                    )]);
                }
                let view_name_type = self.expression_type(args[0], span)?;
                let data_type = self.expression_type(args[1], span)?;
                let is_declared_record = record_type_parts(&data_type).is_some_and(|(name, _)| {
                    self.program
                        .records
                        .iter()
                        .any(|record| record.name == name)
                });
                if view_name_type != Type::String
                    || (!is_declared_record
                        && !matches!(data_type, Type::Map(key, _) if *key == Type::String))
                {
                    return Err(vec![bytecode_error(
                        "render_pdf requires a String view name and a record or string-keyed map",
                        span,
                    )]);
                }
                self.expression(args[0], active)?;
                self.expression(args[1], active)?;
                self.emit(BytecodeOp::RenderPdf, span)
            }
            MirExpressionKind::Call {
                name,
                function: None,
                type_args,
                args,
                ..
            } if name == "sql_decode" => {
                if type_args.len() != 1
                    || args.len() != 1
                    || self.expression_type(args[0], span)? != type_args[0]
                {
                    return Err(vec![bytecode_error(
                        "sql_decode expects one type argument and one matching SQL expression",
                        span,
                    )]);
                }
                let query = match self
                    .program
                    .expressions
                    .get(args[0].0)
                    .map(|item| &item.kind)
                {
                    Some(MirExpressionKind::Sql { query, .. }) => query,
                    _ => {
                        return Err(vec![bytecode_error(
                            "sql_decode requires a SELECT query",
                            self.program.expressions[args[0].0].span,
                        )]);
                    }
                };
                if zelyra_database::sql::analyze_table_access(query, &[]).0 != Some("SELECT") {
                    return Err(vec![bytecode_error(
                        "sql_decode requires a SELECT query",
                        self.program.expressions[args[0].0].span,
                    )]);
                }
                self.expression(args[0], active)?;
                self.emit(
                    BytecodeOp::Builtin {
                        name,
                        arguments: 1,
                        type_arguments: type_args,
                        result_type: expression_type,
                    },
                    span,
                )
            }
            MirExpressionKind::Call {
                name,
                function: None,
                type_args,
                args,
                ..
            } if name == "module_setting" => {
                if type_args.len() != 1
                    || args.len() != 1
                    || !matches!(
                        type_args[0],
                        Type::String | Type::Bool | Type::Int | Type::Float
                    )
                    || self.expression_type(args[0], span)? != Type::String
                    || expression_type != type_args[0]
                {
                    return Err(vec![bytecode_error(
                        "module_setting expects a String key and one supported scalar type argument",
                        span,
                    )]);
                }
                self.expression(args[0], active)?;
                self.emit(
                    BytecodeOp::Builtin {
                        name,
                        arguments: args.len(),
                        type_arguments: type_args,
                        result_type: expression_type,
                    },
                    span,
                )
            }
            MirExpressionKind::Call {
                name,
                function: None,
                args,
                ..
            } if name.rsplit_once("::").is_some() => {
                let (enum_name, variant) = name.rsplit_once("::").expect("guarded split");
                let payload_types =
                    self.enum_payload_types(&expression_type, enum_name, variant, span)?;
                if args.len() != payload_types.len() {
                    return Err(vec![bytecode_error(
                        format!(
                            "enum variant `{enum_name}::{variant}` expects {} payload value(s), found {}",
                            payload_types.len(),
                            args.len()
                        ),
                        span,
                    )]);
                }
                for (argument, expected) in args.iter().zip(&payload_types) {
                    if self.expression_type(*argument, span)? != *expected {
                        return Err(vec![bytecode_error(
                            format!("enum variant `{enum_name}::{variant}` has a mismatched payload type"),
                            self.program.expressions[argument.0].span,
                        )]);
                    }
                    self.expression(*argument, active)?;
                }
                self.emit(
                    BytecodeOp::BuildEnum {
                        enum_type: expression_type,
                        variant: variant.to_owned(),
                        payload_types,
                    },
                    span,
                )
            }
            MirExpressionKind::Call {
                name,
                function: None,
                type_args,
                args,
                ..
            } if matches!(
                name.as_str(),
                "now"
                    | "env"
                    | "read_console"
                    | "random_int"
                    | "uuid_v4"
                    | "read_text"
                    | "read_text_result"
                    | "file_exists"
                    | "dir_exists"
                    | "file_size"
                    | "list_dir"
                    | "write_text"
                    | "write_text_result"
                    | "write_text_new_result"
                    | "atomic_write_text_result"
                    | "append_text_result"
                    | "delete_file"
                    | "delete_file_result"
                    | "copy_file_new_result"
                    | "rename_file_result"
                    | "run_process"
                    | "http_get"
                    | "http_request"
                    | "http_json"
                    | "http_result"
                    | "mail_deliver"
            ) =>
            {
                let expected_type = match name.as_str() {
                    "http_get"
                        if args.len() == 1
                            && self.expression_type(args[0], span)? == Type::String =>
                    {
                        Type::String
                    }
                    "http_request"
                        if args.len() == 4
                            && self.expression_type(args[0], span)? == Type::String
                            && self.expression_type(args[1], span)? == Type::String
                            && self.expression_type(args[2], span)?
                                == Type::Array(Box::new(Type::String))
                            && self.expression_type(args[3], span)?
                                == Type::Option(Box::new(Type::String)) =>
                    {
                        Type::Named("HttpResponse".into())
                    }
                    "http_json" if type_args.len() == 2 && args.len() == 4 => {
                        type_args[1].clone()
                    }
                    "http_result" if type_args.len() == 2 && args.len() == 4 => {
                        Type::HttpResult(Box::new(type_args[1].clone()))
                    }
                    "mail_deliver" if type_args.is_empty() && args.len() == 2 => {
                        Type::Result(Box::new(Type::Unit), Box::new(Type::String))
                    }
                    "now" if args.is_empty() => Type::Timestamp,
                    "env"
                        if args.len() == 1
                            && self.expression_type(args[0], span)? == Type::String =>
                    {
                        Type::Option(Box::new(Type::String))
                    }
                    "read_console"
                        if args.len() == 1
                            && self.expression_type(args[0], span)? == Type::String =>
                    {
                        Type::Option(Box::new(Type::String))
                    }
                    "random_int"
                        if args.len() == 2
                            && self.expression_type(args[0], span)? == Type::Int
                            && self.expression_type(args[1], span)? == Type::Int =>
                    {
                        Type::Int
                    }
                    "uuid_v4" if args.is_empty() => Type::String,
                    "read_text" | "read_text_result"
                        if args.len() == 1
                            && self.expression_type(args[0], span)? == Type::String =>
                    {
                        if name == "read_text" {
                            Type::String
                        } else {
                            Type::Result(Box::new(Type::String), Box::new(Type::String))
                        }
                    }
                    "file_exists" | "dir_exists"
                        if args.len() == 1
                            && self.expression_type(args[0], span)? == Type::String =>
                    {
                        Type::Bool
                    }
                    "list_dir"
                        if args.len() == 1
                            && self.expression_type(args[0], span)? == Type::String =>
                    {
                        Type::Array(Box::new(Type::String))
                    }
                    "file_size"
                        if args.len() == 1
                            && self.expression_type(args[0], span)? == Type::String =>
                    {
                        Type::Result(Box::new(Type::Int), Box::new(Type::String))
                    }
                    "write_text"
                    | "write_text_result"
                    | "write_text_new_result"
                    | "atomic_write_text_result"
                    | "append_text_result"
                        if args.len() == 2
                            && self.expression_type(args[0], span)? == Type::String
                            && self.expression_type(args[1], span)? == Type::String =>
                    {
                        if name == "write_text" {
                            Type::Unit
                        } else {
                            Type::Result(Box::new(Type::Unit), Box::new(Type::String))
                        }
                    }
                    "delete_file" | "delete_file_result"
                        if args.len() == 1
                            && self.expression_type(args[0], span)? == Type::String =>
                    {
                        if name == "delete_file" {
                            Type::Unit
                        } else {
                            Type::Result(Box::new(Type::Unit), Box::new(Type::String))
                        }
                    }
                    "copy_file_new_result" | "rename_file_result"
                        if args.len() == 2
                            && self.expression_type(args[0], span)? == Type::String
                            && self.expression_type(args[1], span)? == Type::String =>
                    {
                        Type::Result(Box::new(Type::Unit), Box::new(Type::String))
                    }
                    "run_process"
                        if args.len() == 2
                            && self.expression_type(args[0], span)? == Type::String
                            && self.expression_type(args[1], span)?
                                == Type::Array(Box::new(Type::String)) =>
                    {
                        Type::String
                    }
                    "now" => {
                        return Err(vec![bytecode_error("now expects no arguments", span)]);
                    }
                    "env" => return Err(vec![bytecode_error("env expects one String name", span)]),
                    "read_console" => {
                        return Err(vec![bytecode_error(
                            "read_console expects one String prompt",
                            span,
                        )])
                    }
                    "random_int" => {
                        return Err(vec![bytecode_error(
                            "random_int expects two Int arguments",
                            span,
                        )])
                    }
                    "uuid_v4" => {
                        return Err(vec![bytecode_error("uuid_v4 expects no arguments", span)])
                    }
                    "read_text" | "read_text_result" => {
                        return Err(vec![bytecode_error(
                            format!("{name} expects one String path"),
                            span,
                        )])
                    }
                    "file_exists" | "dir_exists" | "file_size" | "list_dir" => {
                        return Err(vec![bytecode_error(
                            format!("{name} expects one String path"),
                            span,
                        )])
                    }
                    "write_text"
                    | "write_text_result"
                    | "write_text_new_result"
                    | "atomic_write_text_result"
                    | "append_text_result" => {
                        return Err(vec![bytecode_error(
                            format!("{name} expects a String path and String content"),
                            span,
                        )])
                    }
                    "delete_file" | "delete_file_result" => {
                        return Err(vec![bytecode_error(
                            format!("{name} expects one String path"),
                            span,
                        )])
                    }
                    "copy_file_new_result" | "rename_file_result" => {
                        return Err(vec![bytecode_error(
                            format!("{name} expects source and destination String paths"),
                            span,
                        )])
                    }
                    "run_process" => {
                        return Err(vec![bytecode_error(
                            "run_process expects a String command and String[] arguments",
                            span,
                        )])
                    }
                    "http_get" => {
                        return Err(vec![bytecode_error(
                            "http_get expects one String URL",
                            span,
                        )])
                    }
                    "http_request" => {
                        return Err(vec![bytecode_error(
                            "http_request expects String method, String URL, String[] headers, and String? body",
                            span,
                        )])
                    }
                    "http_json" | "http_result" => {
                        return Err(vec![bytecode_error(
                            format!("{name} expects request and response type arguments plus four values"),
                            span,
                        )])
                    }
                    "mail_deliver" => {
                        return Err(vec![bytecode_error(
                            "mail_deliver expects an event ID and payload JSON",
                            span,
                        )])
                    }
                    _ => unreachable!("guard lists all supported host builtins"),
                };
                if expression_type != expected_type {
                    return Err(vec![bytecode_error(
                        "typed MIR host builtin has an unexpected result type",
                        span,
                    )]);
                }
                self.emit(
                    BytecodeOp::HostCallPreflight {
                        name: name.clone(),
                        arguments: args.len(),
                    },
                    span,
                );
                for argument in &args {
                    self.expression(*argument, active)?;
                }
                self.emit(
                    BytecodeOp::Builtin {
                        name,
                        arguments: args.len(),
                        type_arguments: type_args,
                        result_type: expression_type,
                    },
                    span,
                )
            }
            MirExpressionKind::Call {
                name,
                function: None,
                args,
                ..
            } if matches!(name.as_str(), "map" | "filter" | "find" | "reduce") => {
                self.higher_order_call(&name, &args, &expression_type, span, active)?;
                self.code.len()
            }
            MirExpressionKind::Call {
                name,
                function: None,
                args,
                ..
            } if name == "push_mut" => {
                if args.len() != 2 || expression_type != Type::Unit {
                    return Err(vec![bytecode_error(
                        "push_mut expects a mutable array and one value and returns Unit",
                        span,
                    )]);
                }
                let Some(MirExpressionKind::Local(local)) = self
                    .program
                    .expressions
                    .get(args[0].0)
                    .map(|target| &target.kind)
                else {
                    return Err(vec![bytecode_error(
                        "push_mut must target a mutable array local",
                        span,
                    )]);
                };
                let local = *local;
                let Some(Type::Array(element_type)) = self
                    .program
                    .expressions
                    .get(args[0].0)
                    .map(|target| target.ty.clone())
                else {
                    return Err(vec![bytecode_error(
                        "push_mut target does not have an array type",
                        span,
                    )]);
                };
                if self.expression_type(args[1], span)? != *element_type {
                    return Err(vec![bytecode_error(
                        "push_mut value does not match its array element type",
                        span,
                    )]);
                }
                self.expression(args[1], active)?;
                self.emit(
                    BytecodeOp::ArrayAppend {
                        local,
                        element_type: *element_type,
                    },
                    span,
                );
                self.emit(BytecodeOp::Constant(Value::Unit), span)
            }
            MirExpressionKind::Call {
                name,
                function: None,
                args,
                ..
            } if name == "pop_mut" => {
                if args.len() != 1 {
                    return Err(vec![bytecode_error(
                        "pop_mut expects one mutable array local",
                        span,
                    )]);
                }
                let Some(MirExpressionKind::Local(local)) = self
                    .program
                    .expressions
                    .get(args[0].0)
                    .map(|target| &target.kind)
                else {
                    return Err(vec![bytecode_error(
                        "pop_mut must target a mutable array local",
                        span,
                    )]);
                };
                let local = *local;
                let Some(Type::Array(element_type)) = self
                    .program
                    .expressions
                    .get(args[0].0)
                    .map(|target| target.ty.clone())
                else {
                    return Err(vec![bytecode_error(
                        "pop_mut target does not have an array type",
                        span,
                    )]);
                };
                if expression_type != Type::Option(element_type.clone()) {
                    return Err(vec![bytecode_error(
                        "pop_mut result does not match its array element type",
                        span,
                    )]);
                }
                self.emit(
                    BytecodeOp::ArrayPopMut {
                        local,
                        element_type: *element_type,
                    },
                    span,
                )
            }
            MirExpressionKind::Call {
                name,
                function: None,
                type_args,
                args,
                ..
            } if name == "print" || is_pure_builtin(&name) => {
                for argument in &args {
                    self.expression(*argument, active)?;
                }
                self.emit(
                    BytecodeOp::Builtin {
                        name,
                        arguments: args.len(),
                        type_arguments: type_args,
                        result_type: expression_type,
                    },
                    span,
                )
            }
            unsupported => {
                return Err(vec![bytecode_error(
                    format!(
                        "bytecode backend does not support MIR expression `{}`",
                        mir_expression_name(&unsupported)
                    ),
                    span,
                )]);
            }
        };
        active.remove(&id);
        let _ = result;
        Ok(())
    }

    fn expression_type(&self, id: ExpressionId, span: Span) -> Result<Type, Vec<MirError>> {
        self.program
            .expressions
            .get(id.0)
            .map(|expression| expression.ty.clone())
            .ok_or_else(|| {
                vec![bytecode_error(
                    "binary operand id is outside the MIR expression table",
                    span,
                )]
            })
    }

    fn binary_result_type(
        &self,
        left: ExpressionId,
        operator: BinaryOp,
        right: ExpressionId,
        span: Span,
    ) -> Result<Type, Vec<MirError>> {
        let left = self.expression_type(left, span)?;
        let right = self.expression_type(right, span)?;
        self.binary_result_type_from_types(&left, operator, &right, span)
    }

    fn binary_result_type_from_types(
        &self,
        left: &Type,
        operator: BinaryOp,
        right: &Type,
        span: Span,
    ) -> Result<Type, Vec<MirError>> {
        let supported = if left != right {
            None
        } else {
            match left {
                Type::Int | Type::UInt | Type::Float => match operator {
                    BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Remainder => Some(left.clone()),
                    BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => Some(Type::Bool),
                    BinaryOp::And | BinaryOp::Or => None,
                },
                Type::Decimal => match operator {
                    BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Remainder => Some(Type::Decimal),
                    BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => Some(Type::Bool),
                    BinaryOp::And | BinaryOp::Or => None,
                },
                Type::Duration => match operator {
                    BinaryOp::Add | BinaryOp::Subtract => Some(Type::Duration),
                    BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => Some(Type::Bool),
                    _ => None,
                },
                Type::Date | Type::Time | Type::Timestamp => match operator {
                    BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => Some(Type::Bool),
                    _ => None,
                },
                Type::String => match operator {
                    BinaryOp::Add => Some(Type::String),
                    BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => Some(Type::Bool),
                    _ => None,
                },
                Type::Bool => match operator {
                    BinaryOp::And | BinaryOp::Or | BinaryOp::Equal | BinaryOp::NotEqual => {
                        Some(Type::Bool)
                    }
                    _ => None,
                },
                Type::Char => match operator {
                    BinaryOp::Equal | BinaryOp::NotEqual => Some(Type::Bool),
                    _ => None,
                },
                Type::Array(_) if operator == BinaryOp::Add => Some(left.clone()),
                Type::Array(_) | Type::Option(_) | Type::Result(_, _) | Type::Tuple(_)
                    if matches!(operator, BinaryOp::Equal | BinaryOp::NotEqual) =>
                {
                    Some(Type::Bool)
                }
                _ => None,
            }
        };
        supported.ok_or_else(|| {
            vec![bytecode_error(
                format!(
                    "bytecode backend does not support operator {operator:?} for `{left}` and `{right}`"
                ),
                span,
            )]
        })
    }
}
