use std::collections::HashMap;

use zelyra_ast::{
    BinaryOp, Block, Expr, ExprKind, Function, Pattern, PatternKind, Span, Stmt, Type,
};

#[derive(Clone, Debug)]
pub struct TypedHole {
    pub span: Span,
    pub expected_type: Option<Type>,
    pub visible_values: Vec<String>,
    pub visible_functions: Vec<String>,
    pub capabilities: Vec<String>,
    pub contract_spans: Vec<Span>,
}

pub fn collect_typed_holes(program: &zelyra_ast::Program) -> Vec<TypedHole> {
    let visible_functions = visible_functions(program);
    let function_parameters = program
        .functions
        .iter()
        .map(|function| {
            (
                function.name.clone(),
                function
                    .params
                    .iter()
                    .map(|parameter| parameter.ty.clone())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<HashMap<_, _>>();
    let record_fields = program
        .records
        .iter()
        .map(|record| {
            (
                record.name.clone(),
                record
                    .fields
                    .iter()
                    .map(|field| (field.name.clone(), field.ty.clone()))
                    .collect::<HashMap<_, _>>(),
            )
        })
        .collect::<HashMap<_, _>>();

    let mut collector = Collector {
        visible_functions,
        function_parameters,
        record_fields,
        holes: Vec::new(),
    };
    for function in &program.functions {
        collector.function(function);
    }
    collector.holes.sort_by_key(|hole| hole.span.start);
    collector.holes
}

struct Collector {
    visible_functions: Vec<String>,
    function_parameters: HashMap<String, Vec<Type>>,
    record_fields: HashMap<String, HashMap<String, Type>>,
    holes: Vec<TypedHole>,
}

#[derive(Clone, Default)]
struct Scope {
    values: Vec<String>,
    types: HashMap<String, Type>,
}

impl Scope {
    fn add(&mut self, name: &str, ty: Option<Type>) {
        if !self.values.iter().any(|value| value == name) {
            self.values.push(name.to_owned());
        }
        if let Some(ty) = ty {
            self.types.insert(name.to_owned(), ty);
        }
    }
}

impl Collector {
    fn function(&mut self, function: &Function) {
        let mut scope = Scope::default();
        for parameter in &function.params {
            scope.add(&parameter.name, Some(parameter.ty.clone()));
        }
        let contract_spans = function
            .requires
            .iter()
            .chain(function.ensures.iter())
            .map(|expression| expression.span)
            .collect::<Vec<_>>();
        let capabilities = function.capabilities.clone();

        for expression in &function.requires {
            self.expression(
                expression,
                Some(Type::Bool),
                &scope,
                &capabilities,
                &contract_spans,
            );
        }
        let mut ensures_scope = scope.clone();
        ensures_scope.add("result", function.return_type.clone());
        for expression in &function.ensures {
            self.expression(
                expression,
                Some(Type::Bool),
                &ensures_scope,
                &capabilities,
                &contract_spans,
            );
        }
        self.block(
            &function.body,
            &scope,
            function.return_type.clone(),
            &capabilities,
            &contract_spans,
        );
    }

    fn block(
        &mut self,
        block: &Block,
        parent_scope: &Scope,
        return_type: Option<Type>,
        capabilities: &[String],
        contract_spans: &[Span],
    ) {
        let mut scope = parent_scope.clone();
        for statement in &block.statements {
            self.statement(
                statement,
                &mut scope,
                return_type.clone(),
                capabilities,
                contract_spans,
            );
        }
    }

    fn statement(
        &mut self,
        statement: &Stmt,
        scope: &mut Scope,
        return_type: Option<Type>,
        capabilities: &[String],
        contract_spans: &[Span],
    ) {
        match statement {
            Stmt::Let {
                name, ty, value, ..
            } => {
                self.expression(value, ty.clone(), scope, capabilities, contract_spans);
                scope.add(name, ty.clone());
            }
            Stmt::BindOrAssign { name, value, .. } => {
                let expected = scope.types.get(name).cloned();
                self.expression(value, expected, scope, capabilities, contract_spans);
            }
            Stmt::Expr(expression) => {
                self.expression(expression, None, scope, capabilities, contract_spans)
            }
            Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    self.expression(value, return_type, scope, capabilities, contract_spans);
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expression(
                    condition,
                    Some(Type::Bool),
                    scope,
                    capabilities,
                    contract_spans,
                );
                self.block(
                    then_block,
                    scope,
                    return_type.clone(),
                    capabilities,
                    contract_spans,
                );
                if let Some(else_block) = else_block {
                    self.block(else_block, scope, return_type, capabilities, contract_spans);
                }
            }
            Stmt::While {
                condition,
                invariants,
                body,
                ..
            } => {
                self.expression(
                    condition,
                    Some(Type::Bool),
                    scope,
                    capabilities,
                    contract_spans,
                );
                for invariant in invariants {
                    self.expression(
                        invariant,
                        Some(Type::Bool),
                        scope,
                        capabilities,
                        contract_spans,
                    );
                }
                self.block(body, scope, return_type, capabilities, contract_spans);
            }
            Stmt::For {
                name,
                iterable,
                body,
                ..
            } => {
                self.expression(iterable, None, scope, capabilities, contract_spans);
                let mut body_scope = scope.clone();
                body_scope.add(name, None);
                self.block(body, &body_scope, return_type, capabilities, contract_spans);
            }
            Stmt::Loop {
                invariants, body, ..
            } => {
                for invariant in invariants {
                    self.expression(
                        invariant,
                        Some(Type::Bool),
                        scope,
                        capabilities,
                        contract_spans,
                    );
                }
                self.block(body, scope, return_type, capabilities, contract_spans);
            }
            Stmt::Match { value, arms, .. } => {
                self.expression(value, None, scope, capabilities, contract_spans);
                for arm in arms {
                    let mut arm_scope = scope.clone();
                    add_pattern_values(&mut arm_scope, &arm.pattern);
                    self.block(
                        &arm.body,
                        &arm_scope,
                        return_type.clone(),
                        capabilities,
                        contract_spans,
                    );
                }
            }
            Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => {
                self.block(body, scope, return_type, capabilities, contract_spans);
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
    }

    fn expression(
        &mut self,
        expression: &Expr,
        expected_type: Option<Type>,
        scope: &Scope,
        capabilities: &[String],
        contract_spans: &[Span],
    ) {
        match &expression.kind {
            ExprKind::Variable(name) if name == "_" => self.holes.push(TypedHole {
                span: expression.span,
                expected_type,
                visible_values: scope.values.clone(),
                visible_functions: self.visible_functions.clone(),
                capabilities: capabilities.to_vec(),
                contract_spans: contract_spans.to_vec(),
            }),
            ExprKind::Array(values) => {
                let item_type = match expected_type {
                    Some(Type::Array(inner)) => Some(*inner),
                    _ => None,
                };
                for value in values {
                    self.expression(
                        value,
                        item_type.clone(),
                        scope,
                        capabilities,
                        contract_spans,
                    );
                }
            }
            ExprKind::Map(entries) => {
                let (key_type, value_type) = match expected_type {
                    Some(Type::Map(key, value)) => (Some(*key), Some(*value)),
                    _ => (None, None),
                };
                for (key, value) in entries {
                    self.expression(key, key_type.clone(), scope, capabilities, contract_spans);
                    self.expression(
                        value,
                        value_type.clone(),
                        scope,
                        capabilities,
                        contract_spans,
                    );
                }
            }
            ExprKind::Record { type_name, fields } => {
                for (name, value) in fields {
                    let field_type = self
                        .record_fields
                        .get(type_name)
                        .and_then(|fields| fields.get(name))
                        .cloned();
                    self.expression(value, field_type, scope, capabilities, contract_spans);
                }
            }
            ExprKind::Index { target, index } => {
                self.expression(target, None, scope, capabilities, contract_spans);
                self.expression(index, Some(Type::Int), scope, capabilities, contract_spans);
            }
            ExprKind::Field { target, .. } => {
                self.expression(target, None, scope, capabilities, contract_spans)
            }
            ExprKind::Call { name, args, .. } => {
                let parameter_types = self.function_parameters.get(name).cloned();
                for (index, argument) in args.iter().enumerate() {
                    let argument_type = parameter_types
                        .as_ref()
                        .and_then(|parameters| parameters.get(index))
                        .cloned();
                    self.expression(argument, argument_type, scope, capabilities, contract_spans);
                }
            }
            ExprKind::Unary { op, expr } => {
                let inner_type = if matches!(op, zelyra_ast::UnaryOp::Not) {
                    Some(Type::Bool)
                } else {
                    None
                };
                self.expression(expr, inner_type, scope, capabilities, contract_spans);
            }
            ExprKind::Binary { left, op, right } => {
                let operand_type = if matches!(op, BinaryOp::And | BinaryOp::Or) {
                    Some(Type::Bool)
                } else {
                    None
                };
                self.expression(
                    left,
                    operand_type.clone(),
                    scope,
                    capabilities,
                    contract_spans,
                );
                self.expression(right, operand_type, scope, capabilities, contract_spans);
            }
            ExprKind::Await(inner) => {
                self.expression(inner, expected_type, scope, capabilities, contract_spans)
            }
            ExprKind::Int(_)
            | ExprKind::UInt(_)
            | ExprKind::Float(_)
            | ExprKind::Bool(_)
            | ExprKind::String(_)
            | ExprKind::Char(_)
            | ExprKind::Variable(_)
            | ExprKind::Sql { .. } => {}
        }
    }
}

fn visible_functions(program: &zelyra_ast::Program) -> Vec<String> {
    let mut functions = vec![
        "print",
        "now",
        "env",
        "random_int",
        "read_text",
        "write_text",
        "delete_file",
        "list_dir",
        "http_get",
        "http_request",
        "http_json",
        "http_result",
        "json_encode",
        "json_decode",
        "run_process",
        "len",
        "append",
        "contains",
        "get",
        "put",
        "keys",
        "values",
        "first",
        "last",
        "Some",
        "None",
        "Ok",
        "Err",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    for function in &program.functions {
        if !functions.iter().any(|name| name == &function.name) {
            functions.push(function.name.clone());
        }
    }
    functions
}

fn add_pattern_values(scope: &mut Scope, pattern: &Pattern) {
    match &pattern.kind {
        PatternKind::Variable(name) => scope.add(name, None),
        PatternKind::Constructor { inner, .. } => {
            if let Some(inner) = inner {
                add_pattern_values(scope, inner);
            }
        }
        PatternKind::Wildcard
        | PatternKind::Int(_)
        | PatternKind::Bool(_)
        | PatternKind::String(_)
        | PatternKind::Char(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zelyra_lexer::lex;
    use zelyra_parser::parse;

    #[test]
    fn reports_context_for_a_typed_hole() {
        let source = r#"
            fn greet(name: String) -> String
                requires { name != "" }
                ensures { result != "" }
            {
                message: String = _
                return message
            }

            fn main() {
                print(greet("Zelyra"))
            }
        "#;
        let program = parse(&lex(source).expect("source should lex")).expect("source should parse");
        let holes = collect_typed_holes(&program);
        assert_eq!(holes.len(), 1);
        assert_eq!(holes[0].expected_type, Some(Type::String));
        assert_eq!(holes[0].visible_values, vec!["name"]);
        assert!(holes[0]
            .visible_functions
            .iter()
            .any(|function| function == "greet"));
        assert_eq!(holes[0].capabilities, Vec::<String>::new());
        assert_eq!(holes[0].contract_spans.len(), 2);
    }
}
