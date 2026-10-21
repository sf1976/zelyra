//! Expression type inference and checking for the shared compiler checker.

use super::*;

impl Checker<'_> {
    pub(super) fn check_expr_inner(
        &mut self,
        expr: &Expr,
        scopes: &[HashMap<String, Variable>],
    ) -> Type {
        match &expr.kind {
            ExprKind::Int(_) => Type::Int,
            ExprKind::UInt(_) => Type::UInt,
            ExprKind::Float(_) => Type::Float,
            ExprKind::Bool(_) => Type::Bool,
            ExprKind::String(_) => Type::String,
            ExprKind::Char(_) => Type::Char,
            ExprKind::FunctionValue(name) => {
                let Some(signature) = self.functions.get(name).cloned() else {
                    self.error_with_code(
                        "E-TYPE-FUNCTION-001",
                        expr.span,
                        format!("unknown function value `{name}`"),
                    );
                    return Type::Unknown;
                };
                if !signature.type_params.is_empty() {
                    self.error_with_code(
                        "E-TYPE-FUNCTION-001",
                        expr.span,
                        format!(
                            "generic function `{name}` cannot be stored as a function value yet"
                        ),
                    );
                    return Type::Unknown;
                }
                Type::Function {
                    params: signature.params,
                    return_type: Box::new(signature.return_type),
                    capabilities: signature.capabilities,
                }
            }
            ExprKind::Lambda { params, body } => {
                let mut lambda_scopes = lambda_capture_scopes(scopes);
                let mut parameter_types = Vec::with_capacity(params.len());
                let mut parameter_scope = HashMap::new();
                for parameter in params {
                    parameter_types.push(parameter.ty.clone());
                    parameter_scope.insert(
                        parameter.name.clone(),
                        Variable {
                            ty: parameter.ty.clone(),
                            mutable: false,
                        },
                    );
                }
                lambda_scopes.push(parameter_scope);
                let return_type = self.check_expr(body, &lambda_scopes);
                Type::Function {
                    params: parameter_types,
                    return_type: Box::new(return_type),
                    capabilities: Vec::new(),
                }
            }
            ExprKind::Array(values) => {
                let mut element = Type::Unknown;
                for value in values {
                    let value_type = self.check_expr(value, scopes);
                    if element == Type::Unknown {
                        element = value_type;
                    } else {
                        self.expect_type(&element, &value_type, value.span);
                    }
                }
                Type::Array(Box::new(element))
            }
            ExprKind::Map(entries) => {
                let mut key_type = Type::Unknown;
                let mut value_type = Type::Unknown;
                for (key, value) in entries {
                    let actual_key = self.check_expr(key, scopes);
                    let actual_value = self.check_expr(value, scopes);
                    if key_type == Type::Unknown {
                        key_type = actual_key;
                    } else {
                        self.expect_type(&key_type, &actual_key, key.span);
                    }
                    if value_type == Type::Unknown {
                        value_type = actual_value;
                    } else {
                        self.expect_type(&value_type, &actual_value, value.span);
                    }
                }
                if key_type != Type::Unknown && !is_map_key_type(&key_type) {
                    self.error(
                        expr.span,
                        format!(
                            "Map keys must be Int, UInt, String, Bool, Char, or Timestamp; found `{key_type}`"
                        ),
                    );
                }
                Type::Map(Box::new(key_type), Box::new(value_type))
            }
            ExprKind::Record {
                type_name,
                type_args,
                fields,
            } => {
                let Some(record) = self
                    ._program
                    .records
                    .iter()
                    .find(|record| record.name == *type_name)
                    .cloned()
                else {
                    for (_, value) in fields {
                        self.check_expr(value, scopes);
                    }
                    self.error(expr.span, format!("unknown record type `{type_name}`"));
                    return Type::Unknown;
                };
                let generic_parameters = record.type_params.iter().cloned().collect::<HashSet<_>>();
                if type_args.len() != record.type_params.len() {
                    if record.type_params.is_empty() && !type_args.is_empty() {
                        self.error_with_code(
                            "E-TYPE-GENERIC-001",
                            expr.span,
                            format!("record `{type_name}` does not accept type arguments"),
                        );
                    } else if !type_args.is_empty() {
                        self.error_with_code(
                            "E-TYPE-GENERIC-001",
                            expr.span,
                            format!(
                                "record `{type_name}` expects {} type argument(s), found {}",
                                record.type_params.len(),
                                type_args.len()
                            ),
                        );
                    }
                }
                for argument in type_args {
                    self.check_type(argument, expr.span);
                }
                let mut substitutions = record
                    .type_params
                    .iter()
                    .zip(type_args)
                    .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                    .collect::<HashMap<_, _>>();
                let mut provided = HashSet::new();
                let mut checked_fields = Vec::new();
                for (field_name, value) in fields {
                    let actual = self.check_expr(value, scopes);
                    if !provided.insert(field_name) {
                        self.error(
                            value.span,
                            format!("record field `{field_name}` is specified more than once"),
                        );
                        continue;
                    }
                    let Some(field) = record.fields.iter().find(|field| field.name == *field_name)
                    else {
                        self.error(
                            value.span,
                            format!("unknown field `{field_name}` in record `{type_name}`"),
                        );
                        continue;
                    };
                    infer_generic_types(
                        &field.ty,
                        &actual,
                        &generic_parameters,
                        &mut substitutions,
                    );
                    checked_fields.push((field, value, actual));
                }
                for (field, value, actual) in checked_fields {
                    let expected = substitute_generic_type(&field.ty, &substitutions);
                    self.expect_type(&expected, &actual, value.span);
                }
                for field in &record.fields {
                    if !provided.contains(&field.name) && !matches!(field.ty, Type::Option(_)) {
                        self.error(
                            expr.span,
                            format!("missing field `{}` in record `{type_name}`", field.name),
                        );
                    }
                }
                let applied_args = record
                    .type_params
                    .iter()
                    .map(|parameter| {
                        substitutions
                            .get(parameter)
                            .cloned()
                            .unwrap_or(Type::Unknown)
                    })
                    .collect::<Vec<_>>();
                if let Some(missing) = record
                    .type_params
                    .iter()
                    .find(|parameter| !substitutions.contains_key(*parameter))
                {
                    self.error_with_code(
                        "E-TYPE-GENERIC-001",
                        expr.span,
                        format!(
                            "cannot infer type parameter `{missing}` for record `{type_name}`; use `{type_name}<...>`"
                        ),
                    );
                }
                if record.type_params.is_empty() {
                    Type::Named(type_name.clone())
                } else {
                    Type::Applied {
                        name: type_name.clone(),
                        args: applied_args,
                    }
                }
            }
            ExprKind::Variable(name) => {
                if let Some((enum_name, variant)) = name.rsplit_once("::") {
                    if let Some(definition) = self
                        ._program
                        .enums
                        .iter()
                        .find(|definition| definition.name == enum_name)
                        .filter(|definition| definition.variants.iter().any(|item| item == variant))
                    {
                        if definition
                            .variant_payloads
                            .iter()
                            .find(|(name, _)| name == variant)
                            .is_some_and(|(_, payloads)| !payloads.is_empty())
                        {
                            self.error_with_code(
                                "E-TYPE-ENUM-001",
                                expr.span,
                                format!(
                                    "enum variant `{enum_name}::{variant}` requires payload values"
                                ),
                            );
                            return Type::Unknown;
                        }
                        if !definition.type_params.is_empty() {
                            self.error_with_code(
                                "E-TYPE-GENERIC-001",
                                expr.span,
                                format!(
                                    "generic enum variant `{name}` requires explicit type arguments; use `{enum_name}::{variant}<...>()`"
                                ),
                            );
                            Type::Unknown
                        } else {
                            Type::Named(enum_name.to_owned())
                        }
                    } else {
                        self.error_with_code(
                            "E-TYPE-ENUM-001",
                            expr.span,
                            format!("unknown enum variant `{name}`"),
                        );
                        Type::Unknown
                    }
                } else {
                    self.lookup(scopes, name)
                        .map(|v| v.ty.clone())
                        .or_else(|| {
                            if name == "None" {
                                Some(Type::Option(Box::new(Type::Unknown)))
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(|| {
                            self.error(expr.span, format!("unknown variable `{name}`"));
                            Type::Unknown
                        })
                }
            }
            ExprKind::Call {
                name,
                type_args,
                args,
            } => {
                if matches!(name.as_str(), "map" | "filter" | "find" | "reduce") {
                    return self.check_higher_order_call(name, type_args, args, expr.span, scopes);
                }
                if name == "render_pdf" {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            "render_pdf expects a named View and one record value",
                        );
                        return Type::Unknown;
                    }
                    let view_type = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &view_type, args[0].span);
                    if let ExprKind::String(view_name) = &args[0].kind {
                        if !self
                            ._program
                            .views
                            .iter()
                            .any(|view| view.name == *view_name)
                        {
                            self.error(args[0].span, format!("unknown PDF view `{view_name}`"));
                        }
                    }
                    match self.check_expr(&args[1], scopes) {
                        Type::Named(name)
                            if self
                                ._program
                                .records
                                .iter()
                                .any(|record| record.name == name) => {}
                        Type::Map(key, _) if *key == Type::String => {}
                        Type::Unknown => {}
                        found => self.error(
                            args[1].span,
                            format!(
                                "render_pdf expects a record or Map<String, T>, found `{found}`"
                            ),
                        ),
                    }
                    return Type::Bytes;
                } else if let Some(ty) =
                    self.check_enum_constructor(name, type_args, args, expr.span, scopes)
                {
                    return ty;
                }
                if name == "read_console" {
                    if args.len() != 1 {
                        self.error(expr.span, "read_console expects exactly one String prompt");
                        return Type::Unknown;
                    }
                    let prompt = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &prompt, args[0].span);
                    Type::Option(Box::new(Type::String))
                } else if name == "print" {
                    if args.len() != 1 {
                        self.error(expr.span, "`print` expects exactly one argument");
                    }
                    for arg in args {
                        self.check_expr(arg, scopes);
                    }
                    Type::Unit
                } else if name == "assert" {
                    if !type_args.is_empty() {
                        self.error(expr.span, "`assert` does not accept type arguments");
                    }
                    if !(1..=2).contains(&args.len()) {
                        self.error(
                            expr.span,
                            "`assert` expects a Bool condition and an optional String message",
                        );
                        return Type::Unknown;
                    }
                    let condition = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::Bool, &condition, args[0].span);
                    if let Some(message) = args.get(1) {
                        let message_type = self.check_expr(message, scopes);
                        self.expect_type(&Type::String, &message_type, message.span);
                    }
                    Type::Unit
                } else if name == "range" {
                    if !type_args.is_empty() || !(2..=3).contains(&args.len()) {
                        self.error(
                            expr.span,
                            "range expects two Int bounds, an optional Int step, and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let start = self.check_expr(&args[0], scopes);
                    let end = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Int, &start, args[0].span);
                    self.expect_type(&Type::Int, &end, args[1].span);
                    if let Some(step) = args.get(2) {
                        let step_type = self.check_expr(step, scopes);
                        self.expect_type(&Type::Int, &step_type, step.span);
                    }
                    Type::Array(Box::new(Type::Int))
                } else if name == "array_replace_result" {
                    if !type_args.is_empty() || args.len() != 3 {
                        self.error(
                            expr.span,
                            "array_replace_result expects an array, Int index, and value",
                        );
                        return Type::Unknown;
                    }
                    let array = self.check_expr(&args[0], scopes);
                    let index = self.check_expr(&args[1], scopes);
                    let value = self.check_expr(&args[2], scopes);
                    self.expect_type(&Type::Int, &index, args[1].span);
                    match array {
                        Type::Array(element) => {
                            self.expect_type(&element, &value, args[2].span);
                            Type::Result(Box::new(Type::Array(element)), Box::new(Type::String))
                        }
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("array_replace_result expects an array, found `{other}`"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "object_set_field" {
                    if !type_args.is_empty() || args.len() != 3 {
                        self.error_with_code(
                            "E-TYPE-CLASS-001",
                            expr.span,
                            "class field update expects an object, literal field name, and value",
                        );
                        return Type::Unknown;
                    }
                    let object_type = self.check_expr(&args[0], scopes);
                    let field_name = match &args[1].kind {
                        ExprKind::String(name) => Some(name.as_str()),
                        _ => {
                            self.error_with_code(
                                "E-TYPE-CLASS-001",
                                args[1].span,
                                "class field name must be a String literal",
                            );
                            None
                        }
                    };
                    let value_type = self.check_expr(&args[2], scopes);
                    let (type_name, type_arguments) = match &object_type {
                        Type::Named(type_name) => (Some(type_name.as_str()), &[][..]),
                        Type::Applied { name, args } => (Some(name.as_str()), args.as_slice()),
                        _ => (None, &[][..]),
                    };
                    if let (Some(type_name), Some(field_name)) = (type_name, field_name) {
                        match self
                            ._program
                            .records
                            .iter()
                            .find(|record| record.name == type_name)
                        {
                            Some(class) if class.is_class => {
                                if let Some(field) =
                                    class.fields.iter().find(|field| field.name == field_name)
                                {
                                    let substitutions = class
                                        .type_params
                                        .iter()
                                        .zip(type_arguments)
                                        .map(|(parameter, argument)| {
                                            (parameter.clone(), argument.clone())
                                        })
                                        .collect::<HashMap<_, _>>();
                                    let field_type =
                                        substitute_generic_type(&field.ty, &substitutions);
                                    let field_type =
                                        self.normalize_alias(&field_type, args[2].span);
                                    if !compatible(&field_type, &value_type) {
                                        self.error_with_code(
                                            "E-TYPE-CLASS-001",
                                            args[2].span,
                                            format!(
                                                "type mismatch: expected `{field_type}`, found `{value_type}`"
                                            ),
                                        );
                                    }
                                } else {
                                    self.error_with_code(
                                        "E-TYPE-CLASS-001",
                                        args[1].span,
                                        format!(
                                            "unknown field `{field_name}` in class `{type_name}`"
                                        ),
                                    );
                                }
                            }
                            _ => self.error_with_code(
                                "E-TYPE-CLASS-001",
                                args[0].span,
                                "field assignment requires a class instance",
                            ),
                        }
                    }
                    Type::Unit
                } else if name == "record_update" {
                    if type_args.len() != 1 || args.is_empty() || args.len() % 2 == 0 {
                        self.error(
                            expr.span,
                            "record update expects a base record followed by field/value pairs",
                        );
                        return Type::Unknown;
                    }
                    self.check_type(&type_args[0], expr.span);
                    let expected_record_type = self.normalize_alias(&type_args[0], expr.span);
                    let actual_record_type = self.check_expr(&args[0], scopes);
                    self.expect_type(&expected_record_type, &actual_record_type, args[0].span);
                    let (record_name, type_arguments) = match &expected_record_type {
                        Type::Named(name) => (Some(name.as_str()), &[][..]),
                        Type::Applied { name, args } => (Some(name.as_str()), args.as_slice()),
                        _ => (None, &[][..]),
                    };
                    let Some(record_name) = record_name else {
                        self.error(
                            args[0].span,
                            "record update base must have a declared record type",
                        );
                        return Type::Unknown;
                    };
                    let Some(record) = self
                        ._program
                        .records
                        .iter()
                        .find(|record| record.name == record_name)
                        .cloned()
                    else {
                        self.error(
                            args[0].span,
                            format!("unknown record type `{record_name}` in record update"),
                        );
                        return Type::Unknown;
                    };
                    if record.is_class {
                        self.error_with_code(
                            "E-TYPE-CLASS-001",
                            args[0].span,
                            "record update cannot copy or update a class reference",
                        );
                        return Type::Unknown;
                    }
                    let substitutions = record
                        .type_params
                        .iter()
                        .zip(type_arguments)
                        .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                        .collect::<HashMap<_, _>>();
                    for pair in args[1..].chunks(2) {
                        let field_name = match &pair[0].kind {
                            ExprKind::String(name) => Some(name.as_str()),
                            _ => {
                                self.error(
                                    pair[0].span,
                                    "record update field name must be a literal field name",
                                );
                                None
                            }
                        };
                        // Field-name literals are represented as expressions for the shared
                        // `record_update` intrinsic. Record their type as well as the value
                        // type so source-mapped MIR does not lose this child expression.
                        self.check_expr(&pair[0], scopes);
                        let value_type = self.check_expr(&pair[1], scopes);
                        if let Some(field_name) = field_name {
                            if let Some(field) =
                                record.fields.iter().find(|field| field.name == field_name)
                            {
                                let field_type = substitute_generic_type(&field.ty, &substitutions);
                                self.expect_type(&field_type, &value_type, pair[1].span);
                            } else {
                                self.error(
                                    pair[0].span,
                                    format!(
                                        "unknown field `{field_name}` in record `{record_name}`"
                                    ),
                                );
                            }
                        }
                    }
                    expected_record_type
                } else if name == "record_with" {
                    if !type_args.is_empty() || args.len() != 3 {
                        self.error(
                            expr.span,
                            "record_with expects a record, literal field name, and value",
                        );
                        return Type::Unknown;
                    }
                    let record_type = self.check_expr(&args[0], scopes);
                    let field_name = match &args[1].kind {
                        ExprKind::String(name) => Some(name.as_str()),
                        _ => {
                            self.error(
                                args[1].span,
                                "record_with field name must be a String literal",
                            );
                            None
                        }
                    };
                    let value = self.check_expr(&args[2], scopes);
                    let (type_name, type_arguments) = match &record_type {
                        Type::Named(type_name) => (Some(type_name.as_str()), &[][..]),
                        Type::Applied { name, args } => (Some(name.as_str()), args.as_slice()),
                        _ => (None, &[][..]),
                    };
                    if let (Some(type_name), Some(field_name)) = (type_name, field_name) {
                        if let Some(record) = self
                            ._program
                            .records
                            .iter()
                            .find(|record| record.name == type_name)
                        {
                            if record.is_class {
                                self.error_with_code(
                                    "E-TYPE-CLASS-001",
                                    args[0].span,
                                    "record_with cannot copy or update a class reference",
                                );
                                return Type::Unknown;
                            }
                            if let Some(field) =
                                record.fields.iter().find(|field| field.name == field_name)
                            {
                                let substitutions = record
                                    .type_params
                                    .iter()
                                    .zip(type_arguments)
                                    .map(|(parameter, argument)| {
                                        (parameter.clone(), argument.clone())
                                    })
                                    .collect::<HashMap<_, _>>();
                                let field_type = substitute_generic_type(&field.ty, &substitutions);
                                self.expect_type(&field_type, &value, args[2].span);
                            } else {
                                self.error(
                                    args[1].span,
                                    format!("unknown field `{field_name}` in record `{type_name}`"),
                                );
                            }
                        } else {
                            self.error(
                                args[0].span,
                                format!("record_with expects a record, found `{record_type}`"),
                            );
                        }
                    }
                    record_type
                } else if name == "len" {
                    if args.len() != 1 {
                        self.error(expr.span, "`len` expects exactly one argument");
                        return Type::Unknown;
                    }
                    let argument = self.check_expr(&args[0], scopes);
                    if !matches!(
                        argument,
                        Type::Array(_)
                            | Type::Tuple(_)
                            | Type::Set(_)
                            | Type::Map(_, _)
                            | Type::String
                            | Type::Bytes
                    ) && argument != Type::Unknown
                    {
                        self.error(
                            args[0].span,
                            "`len` expects an array, Tuple, Set, Map, String or Bytes",
                        );
                    }
                    Type::Int
                } else if name == "append" {
                    if args.len() != 2 {
                        self.error(expr.span, "`append` expects an array and one value");
                        return Type::Unknown;
                    }
                    let array_type = self.check_expr(&args[0], scopes);
                    let value_type = self.check_expr(&args[1], scopes);
                    match array_type {
                        Type::Array(inner) => {
                            self.expect_type(&inner, &value_type, args[1].span);
                            Type::Array(inner)
                        }
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("`append` expects an array, found `{other}`"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "push_mut" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "`push_mut` expects a mutable array and one value",
                        );
                        return Type::Unknown;
                    }
                    let array_type = self.check_expr(&args[0], scopes);
                    let value_type = self.check_expr(&args[1], scopes);
                    if let ExprKind::Variable(binding) = &args[0].kind {
                        if self
                            .lookup(scopes, binding)
                            .is_none_or(|variable| !variable.mutable)
                        {
                            self.error(args[0].span, "`push_mut` requires a mutable array binding");
                        }
                    } else {
                        self.error(
                            args[0].span,
                            "`push_mut` requires the name of a mutable array binding",
                        );
                    }
                    match array_type {
                        Type::Array(inner) => {
                            self.expect_type(&inner, &value_type, args[1].span);
                        }
                        Type::Unknown => {}
                        other => self.error(
                            args[0].span,
                            format!("`push_mut` expects an array, found `{other}`"),
                        ),
                    }
                    Type::Unit
                } else if name == "pop_mut" {
                    if args.len() != 1 {
                        self.error(expr.span, "`pop_mut` expects one mutable array");
                        return Type::Unknown;
                    }
                    let array_type = self.check_expr(&args[0], scopes);
                    if let ExprKind::Variable(binding) = &args[0].kind {
                        if self
                            .lookup(scopes, binding)
                            .is_none_or(|variable| !variable.mutable)
                        {
                            self.error(args[0].span, "`pop_mut` requires a mutable array binding");
                        }
                    } else {
                        self.error(
                            args[0].span,
                            "`pop_mut` requires the name of a mutable array binding",
                        );
                    }
                    match array_type {
                        Type::Array(inner) => Type::Option(inner),
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("`pop_mut` expects an array, found `{other}`"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "contains" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "`contains` expects an array, Set or Map and one value",
                        );
                        return Type::Unknown;
                    }
                    let array_type = self.check_expr(&args[0], scopes);
                    let value_type = self.check_expr(&args[1], scopes);
                    match array_type {
                        Type::Array(inner) => {
                            self.expect_type(&inner, &value_type, args[1].span);
                            Type::Bool
                        }
                        Type::Map(key, _) => {
                            self.expect_type(&key, &value_type, args[1].span);
                            Type::Bool
                        }
                        Type::Set(inner) => {
                            self.expect_type(&inner, &value_type, args[1].span);
                            Type::Bool
                        }
                        Type::Unknown => Type::Bool,
                        other => {
                            self.error(
                                args[0].span,
                                format!(
                                    "`contains` expects an array, Set, or Map, found `{other}`"
                                ),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "tuple" {
                    if args.len() < 2 {
                        self.error(expr.span, "tuple expects at least two values");
                        return Type::Unknown;
                    }
                    Type::Tuple(
                        args.iter()
                            .map(|arg| self.check_expr(arg, scopes))
                            .collect(),
                    )
                } else if name == "set" {
                    if args.len() != 1 {
                        self.error(expr.span, "set expects one array");
                        return Type::Unknown;
                    }
                    match self.check_expr(&args[0], scopes) {
                        Type::Array(inner) => {
                            if !matches!(inner.as_ref(), Type::Unknown) && !is_map_key_type(&inner)
                            {
                                self.error(args[0].span, format!("Set elements must be scalar comparable types; found `{inner}`"));
                            }
                            Type::Set(inner)
                        }
                        Type::Unknown => Type::Set(Box::new(Type::Unknown)),
                        other => {
                            self.error(
                                args[0].span,
                                format!("set expects an array, found `{other}`"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "set_add" || name == "set_remove" {
                    if args.len() != 2 {
                        self.error(expr.span, format!("{name} expects a Set and one element"));
                        return Type::Unknown;
                    }
                    let collection = self.check_expr(&args[0], scopes);
                    let element = self.check_expr(&args[1], scopes);
                    match collection {
                        Type::Set(inner) => {
                            self.expect_type(&inner, &element, args[1].span);
                            Type::Set(inner)
                        }
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("{name} expects a Set, found `{other}`"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "text_contains" {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            "text_contains expects two arguments and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    let search = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    self.expect_type(&Type::String, &search, args[1].span);
                    Type::Bool
                } else if name == "text_trim" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            "text_trim expects one argument and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    Type::String
                } else if name == "text_chars" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            "text_chars expects one argument and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    Type::Array(Box::new(Type::String))
                } else if name == "abs" {
                    if args.len() != 1 {
                        self.error(expr.span, "abs expects exactly one numeric argument");
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    if matches!(ty, Type::Int | Type::Float | Type::Decimal) {
                        ty
                    } else if ty == Type::Unknown {
                        Type::Unknown
                    } else {
                        self.error(
                            args[0].span,
                            format!("abs expects Int, Float, or Decimal, found {ty}"),
                        );
                        Type::Unknown
                    }
                } else if name == "min" || name == "max" {
                    if args.len() != 2 {
                        self.error(expr.span, format!("{name} expects exactly two arguments"));
                        return Type::Unknown;
                    }
                    let a = self.check_expr(&args[0], scopes);
                    let b = self.check_expr(&args[1], scopes);
                    self.expect_type(&a, &b, args[1].span);
                    if !matches!(
                        &a,
                        Type::Int
                            | Type::UInt
                            | Type::Float
                            | Type::Decimal
                            | Type::String
                            | Type::Unknown
                    ) {
                        self.error(
                            args[0].span,
                            format!(
                                "{name} expects matching numeric or String arguments, found {a}"
                            ),
                        );
                        return Type::Unknown;
                    }
                    a
                } else if name == "clamp" {
                    if args.len() != 3 {
                        self.error(expr.span, "clamp expects val, min, and max");
                        return Type::Unknown;
                    }
                    let val = self.check_expr(&args[0], scopes);
                    let min_val = self.check_expr(&args[1], scopes);
                    let max_val = self.check_expr(&args[2], scopes);
                    self.expect_type(&val, &min_val, args[1].span);
                    self.expect_type(&val, &max_val, args[2].span);
                    if !matches!(
                        &val,
                        Type::Int | Type::UInt | Type::Float | Type::Decimal | Type::Unknown
                    ) {
                        self.error(
                            args[0].span,
                            format!("clamp expects matching numeric arguments, found {val}"),
                        );
                        return Type::Unknown;
                    }
                    val
                } else if name == "to_string" {
                    if args.len() != 1 {
                        self.error(expr.span, "to_string expects exactly one argument");
                        return Type::Unknown;
                    }
                    let _ = self.check_expr(&args[0], scopes);
                    Type::String
                } else if name == "parse_int" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            "parse_int expects exactly one String and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &ty, args[0].span);
                    Type::Result(Box::new(Type::Int), Box::new(Type::String))
                } else if name == "parse_float" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            "parse_float expects exactly one String and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &ty, args[0].span);
                    Type::Result(Box::new(Type::Float), Box::new(Type::String))
                } else if name == "sort" || name == "sort_desc" || name == "reverse" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("{name} expects exactly one array"));
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    match ty {
                        Type::Array(inner) => Type::Array(inner),
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("{name} expects an array, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "slice" {
                    if args.len() != 3 {
                        self.error(expr.span, "slice expects array, start index, and length");
                        return Type::Unknown;
                    }
                    let arr_ty = self.check_expr(&args[0], scopes);
                    let start_ty = self.check_expr(&args[1], scopes);
                    let len_ty = self.check_expr(&args[2], scopes);
                    self.expect_type(&Type::Int, &start_ty, args[1].span);
                    self.expect_type(&Type::Int, &len_ty, args[2].span);
                    match arr_ty {
                        Type::Array(inner) => Type::Array(inner),
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("slice expects an array, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "concat" {
                    if args.len() != 2 {
                        self.error(expr.span, "concat expects two arrays of the same type");
                        return Type::Unknown;
                    }
                    let a_ty = self.check_expr(&args[0], scopes);
                    let b_ty = self.check_expr(&args[1], scopes);
                    self.expect_type(&a_ty, &b_ty, args[1].span);
                    a_ty
                } else if name == "join" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "join expects an array of Strings and a delimiter",
                        );
                        return Type::Unknown;
                    }
                    let arr_ty = self.check_expr(&args[0], scopes);
                    let sep_ty = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Array(Box::new(Type::String)), &arr_ty, args[0].span);
                    self.expect_type(&Type::String, &sep_ty, args[1].span);
                    Type::String
                } else if name == "text_to_lower"
                    || name == "text_to_upper"
                    || name == "text_ucfirst"
                {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            format!("{name} expects one argument and no type arguments"),
                        );
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &ty, args[0].span);
                    Type::String
                } else if name == "text_starts_with" || name == "text_ends_with" {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            format!("{name} expects two arguments and no type arguments"),
                        );
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    let pat = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    self.expect_type(&Type::String, &pat, args[1].span);
                    Type::Bool
                } else if name == "text_split" || name == "split" {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            format!("{name} expects two arguments and no type arguments"),
                        );
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    let sep = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    self.expect_type(&Type::String, &sep, args[1].span);
                    Type::Array(Box::new(Type::String))
                } else if name == "text_replace" {
                    if !type_args.is_empty() || args.len() != 3 {
                        self.error(
                            expr.span,
                            "text_replace expects three arguments and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    let old = self.check_expr(&args[1], scopes);
                    let new = self.check_expr(&args[2], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    self.expect_type(&Type::String, &old, args[1].span);
                    self.expect_type(&Type::String, &new, args[2].span);
                    Type::String
                } else if name == "text_pad_left" || name == "text_pad_right" {
                    if !type_args.is_empty() || args.len() != 3 {
                        self.error(
                            expr.span,
                            format!("{name} expects three arguments and no type arguments"),
                        );
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    let target_len = self.check_expr(&args[1], scopes);
                    let pad_char = self.check_expr(&args[2], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    self.expect_type(&Type::Int, &target_len, args[1].span);
                    self.expect_type(&Type::String, &pad_char, args[2].span);
                    Type::String
                } else if name == "get_or" || name == "get_or_default" {
                    if !type_args.is_empty() || args.len() != 3 {
                        self.error(
                            expr.span,
                            format!("{name} expects a Map, key, default fallback value, and no type arguments"),
                        );
                        return Type::Unknown;
                    }
                    let map_ty = self.check_expr(&args[0], scopes);
                    let key_ty = self.check_expr(&args[1], scopes);
                    let fallback_ty = self.check_expr(&args[2], scopes);
                    match map_ty {
                        Type::Map(k, v) => {
                            self.expect_type(&k, &key_ty, args[1].span);
                            self.expect_type(&v, &fallback_ty, args[2].span);
                            *v
                        }
                        Type::Unknown => fallback_ty,
                        other => {
                            self.error(
                                args[0].span,
                                format!("{name} expects a Map, found {other}"),
                            );
                            fallback_ty
                        }
                    }
                } else if name == "map_increment" {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            "map_increment expects a Map<Int>, key, and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let map_ty = self.check_expr(&args[0], scopes);
                    let key_ty = self.check_expr(&args[1], scopes);
                    match map_ty {
                        Type::Map(key, value) => {
                            self.expect_type(&key, &key_ty, args[1].span);
                            self.expect_type(&Type::Int, &value, args[0].span);
                            Type::Result(
                                Box::new(Type::Map(key, Box::new(Type::Int))),
                                Box::new(Type::String),
                            )
                        }
                        Type::Unknown => Type::Result(
                            Box::new(Type::Map(Box::new(Type::Unknown), Box::new(Type::Int))),
                            Box::new(Type::String),
                        ),
                        other => {
                            self.error(
                                args[0].span,
                                format!("map_increment expects a Map, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "has_key" {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            "has_key expects a Map, key, and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let map_ty = self.check_expr(&args[0], scopes);
                    let key_ty = self.check_expr(&args[1], scopes);
                    match map_ty {
                        Type::Map(k, _) => {
                            self.expect_type(&k, &key_ty, args[1].span);
                            Type::Bool
                        }
                        Type::Unknown => Type::Bool,
                        other => {
                            self.error(
                                args[0].span,
                                format!("has_key expects a Map, found {other}"),
                            );
                            Type::Bool
                        }
                    }
                } else if name == "sqrt" || name == "floor" || name == "ceil" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("{name} expects exactly one Float"));
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::Float, &ty, args[0].span);
                    Type::Float
                } else if name == "pow" {
                    if args.len() != 2 {
                        self.error(expr.span, "pow expects base (Float) and exponent (Float)");
                        return Type::Unknown;
                    }
                    let base = self.check_expr(&args[0], scopes);
                    let exp = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Float, &base, args[0].span);
                    self.expect_type(&Type::Float, &exp, args[1].span);
                    Type::Float
                } else if name == "round" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "round expects number (Float) and decimal places (Int)",
                        );
                        return Type::Unknown;
                    }
                    let num = self.check_expr(&args[0], scopes);
                    let decimals = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Float, &num, args[0].span);
                    self.expect_type(&Type::Int, &decimals, args[1].span);
                    Type::Float
                } else if name == "text_index_of" {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            "text_index_of expects two arguments and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    let sub = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    self.expect_type(&Type::String, &sub, args[1].span);
                    Type::Option(Box::new(Type::Int))
                } else if name == "text_repeat" {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            "text_repeat expects two arguments and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    let count = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    self.expect_type(&Type::Int, &count, args[1].span);
                    Type::String
                } else if name == "text_is_empty" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            "text_is_empty expects one argument and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    Type::Bool
                } else if name == "text_capitalize" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            "text_capitalize expects one argument and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    Type::String
                } else if name == "unique" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(expr.span, "unique expects one array and no type arguments");
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    match ty {
                        Type::Array(inner) => Type::Array(inner),
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("unique expects an array, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "array_sum" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            "sum expects one array of numbers and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    match ty {
                        Type::Array(inner)
                            if matches!(*inner, Type::Int | Type::Float | Type::Decimal) =>
                        {
                            *inner
                        }
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!(
                                    "sum expects array of Int, Float, or Decimal, found {other}"
                                ),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "avg" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            "avg expects one array of numbers and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    match ty {
                        Type::Array(inner) if matches!(*inner, Type::Int | Type::Float) => {
                            Type::Float
                        }
                        Type::Array(inner) if matches!(*inner, Type::Decimal) => Type::Decimal,
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("avg expects array of numbers, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "min_of" || name == "max_of" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            format!("{name} expects one array and no type arguments"),
                        );
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    match ty {
                        Type::Array(inner)
                            if matches!(
                                *inner,
                                Type::Int | Type::UInt | Type::Float | Type::Decimal | Type::String
                            ) =>
                        {
                            Type::Option(inner)
                        }
                        Type::Array(inner) if inner.as_ref() == &Type::Unknown => {
                            Type::Option(inner)
                        }
                        Type::Array(inner) => {
                            self.error(
                                args[0].span,
                                format!(
                                    "{name} expects an array of numbers or String, found {}",
                                    inner
                                ),
                            );
                            Type::Option(inner)
                        }
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("{name} expects an array, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "chunk" {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            "chunk expects an array, chunk size (Int), and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let arr_ty = self.check_expr(&args[0], scopes);
                    let size_ty = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Int, &size_ty, args[1].span);
                    match arr_ty {
                        Type::Array(inner) => Type::Array(Box::new(Type::Array(inner))),
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("chunk expects an array, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "flatten" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            "flatten expects one nested array and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let arr_ty = self.check_expr(&args[0], scopes);
                    match arr_ty {
                        Type::Array(inner) => {
                            match *inner {
                                Type::Array(element) => Type::Array(element),
                                other => {
                                    self.error(args[0].span, format!("flatten expects array of arrays, found array of {other}"));
                                    Type::Unknown
                                }
                            }
                        }
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("flatten expects an array, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "set_union"
                    || name == "set_intersection"
                    || name == "set_difference"
                {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            format!("{name} expects two Sets and no type arguments"),
                        );
                        return Type::Unknown;
                    }
                    let a_ty = self.check_expr(&args[0], scopes);
                    let b_ty = self.check_expr(&args[1], scopes);
                    self.expect_type(&a_ty, &b_ty, args[1].span);
                    match a_ty {
                        Type::Set(inner) => Type::Set(inner),
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(args[0].span, format!("{name} expects Sets, found {other}"));
                            Type::Unknown
                        }
                    }
                } else if name == "map_remove" {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            "map_remove expects a Map, key, and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let map_ty = self.check_expr(&args[0], scopes);
                    let key_ty = self.check_expr(&args[1], scopes);
                    match map_ty {
                        Type::Map(k, v) => {
                            self.expect_type(&k, &key_ty, args[1].span);
                            Type::Map(k, v)
                        }
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("map_remove expects a Map, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "map_merge" {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            "map_merge expects two Maps of the same type and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let a_ty = self.check_expr(&args[0], scopes);
                    let b_ty = self.check_expr(&args[1], scopes);
                    self.expect_type(&a_ty, &b_ty, args[1].span);
                    a_ty
                } else if name == "unwrap_or" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "unwrap_or expects an Option and fallback default",
                        );
                        return Type::Unknown;
                    }
                    let opt_ty = self.check_expr(&args[0], scopes);
                    let fallback_ty = self.check_expr(&args[1], scopes);
                    match opt_ty {
                        Type::Option(inner) => {
                            self.expect_type(&inner, &fallback_ty, args[1].span);
                            *inner
                        }
                        Type::Unknown => fallback_ty,
                        other => {
                            self.error(
                                args[0].span,
                                format!("unwrap_or expects Option, found {other}"),
                            );
                            fallback_ty
                        }
                    }
                } else if name == "text_replace_result" {
                    if !type_args.is_empty() || args.len() != 3 {
                        self.error(
                            expr.span,
                            "text_replace_result expects three arguments and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    for arg in args {
                        let value = self.check_expr(arg, scopes);
                        self.expect_type(&Type::String, &value, arg.span);
                    }
                    Type::Result(Box::new(Type::String), Box::new(Type::String))
                } else if name == "text_split_result" {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            "text_split_result expects two arguments and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    for arg in args {
                        let value = self.check_expr(arg, scopes);
                        self.expect_type(&Type::String, &value, arg.span);
                    }
                    Type::Result(
                        Box::new(Type::Array(Box::new(Type::String))),
                        Box::new(Type::String),
                    )
                } else if name == "get" {
                    if args.len() != 2 {
                        self.error(expr.span, "`get` expects a Map and one key");
                        return Type::Unknown;
                    }
                    let map_type = self.check_expr(&args[0], scopes);
                    let key_type = self.check_expr(&args[1], scopes);
                    match map_type {
                        Type::Map(expected_key, value) => {
                            self.expect_type(&expected_key, &key_type, args[1].span);
                            Type::Option(value)
                        }
                        Type::Unknown => Type::Option(Box::new(Type::Unknown)),
                        other => {
                            self.error(
                                args[0].span,
                                format!("`get` expects a Map, found `{other}`"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "put" {
                    if args.len() != 3 {
                        self.error(expr.span, "`put` expects a Map, a key, and a value");
                        return Type::Unknown;
                    }
                    let map_type = self.check_expr(&args[0], scopes);
                    let key_type = self.check_expr(&args[1], scopes);
                    let value_type = self.check_expr(&args[2], scopes);
                    match map_type {
                        Type::Map(expected_key, expected_value) => {
                            self.expect_type(&expected_key, &key_type, args[1].span);
                            self.expect_type(&expected_value, &value_type, args[2].span);
                            Type::Map(expected_key, expected_value)
                        }
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("`put` expects a Map, found `{other}`"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "keys" || name == "values" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            format!("`{name}` expects exactly one Map and no type arguments"),
                        );
                        return Type::Unknown;
                    }
                    let map_type = self.check_expr(&args[0], scopes);
                    match map_type {
                        Type::Map(key, _) if name == "keys" => Type::Array(key),
                        Type::Map(_, value) => Type::Array(value),
                        Type::Unknown => Type::Array(Box::new(Type::Unknown)),
                        other => {
                            self.error(
                                args[0].span,
                                format!("`{name}` expects a Map, found `{other}`"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "first" || name == "last" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("`{name}` expects exactly one argument"));
                        return Type::Unknown;
                    }
                    let argument = self.check_expr(&args[0], scopes);
                    match argument {
                        Type::Array(inner) => Type::Option(inner),
                        Type::Unknown => Type::Option(Box::new(Type::Unknown)),
                        other => {
                            self.error(
                                args[0].span,
                                format!("`{name}` expects an array, found `{other}`"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "Some" || name == "Ok" || name == "Err" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("`{name}` expects exactly one argument"));
                        return Type::Unknown;
                    }
                    let inner = self.check_expr(&args[0], scopes);
                    if name == "Some" {
                        Type::Option(Box::new(inner))
                    } else if name == "Ok" {
                        Type::Result(Box::new(inner), Box::new(Type::Unknown))
                    } else {
                        Type::Result(Box::new(Type::Unknown), Box::new(inner))
                    }
                } else if name == "now" {
                    if !args.is_empty() {
                        self.error(expr.span, "now expects no arguments");
                    }
                    Type::Timestamp
                } else if name == "date_add_days" {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            "date_add_days expects Date, Int days, and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let d = self.check_expr(&args[0], scopes);
                    let days = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Date, &d, args[0].span);
                    self.expect_type(&Type::Int, &days, args[1].span);
                    Type::Date
                } else if name == "date_diff_days" {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            "date_diff_days expects two Dates and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let a = self.check_expr(&args[0], scopes);
                    let b = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Date, &a, args[0].span);
                    self.expect_type(&Type::Date, &b, args[1].span);
                    Type::Int
                } else if name == "is_ok" || name == "is_err" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            format!("{name} expects one Result and no type arguments"),
                        );
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    if !matches!(ty, Type::Result(_, _)) && ty != Type::Unknown {
                        self.error(args[0].span, format!("{name} expects Result, found {ty}"));
                    }
                    Type::Bool
                } else if name == "is_some" || name == "is_none" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            format!("{name} expects one Option and no type arguments"),
                        );
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    if !matches!(ty, Type::Option(_)) && ty != Type::Unknown {
                        self.error(args[0].span, format!("{name} expects Option, found {ty}"));
                    }
                    Type::Bool
                } else if name == "is_before" || name == "is_after" {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            format!("{name} expects two Dates and no type arguments"),
                        );
                        return Type::Unknown;
                    }
                    let a = self.check_expr(&args[0], scopes);
                    let b = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Date, &a, args[0].span);
                    self.expect_type(&Type::Date, &b, args[1].span);
                    Type::Bool
                } else if name == "base64_encode" || name == "hash_sha256" || name == "url_encode" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            format!("{name} expects one String and no type arguments"),
                        );
                        return Type::Unknown;
                    }
                    let s = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &s, args[0].span);
                    Type::String
                } else if name == "base64_decode" || name == "url_decode" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            format!("{name} expects one String and no type arguments"),
                        );
                        return Type::Unknown;
                    }
                    let s = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &s, args[0].span);
                    Type::Result(Box::new(Type::String), Box::new(Type::String))
                } else if name == "uuid_v4" {
                    if !args.is_empty() {
                        self.error(expr.span, "uuid_v4 expects no arguments");
                        return Type::Unknown;
                    }
                    Type::String
                } else if name == "text_trim_start" || name == "text_trim_end" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            format!("{name} expects one argument and no type arguments"),
                        );
                        return Type::Unknown;
                    }
                    let s = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &s, args[0].span);
                    Type::String
                } else if name == "text_is_blank" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            "text_is_blank expects one argument and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let s = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &s, args[0].span);
                    Type::Bool
                } else if name == "text_substring" {
                    if !type_args.is_empty() || args.len() != 3 {
                        self.error(
                            expr.span,
                            "text_substring expects three arguments and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let s = self.check_expr(&args[0], scopes);
                    let start = self.check_expr(&args[1], scopes);
                    let len = self.check_expr(&args[2], scopes);
                    self.expect_type(&Type::String, &s, args[0].span);
                    self.expect_type(&Type::Int, &start, args[1].span);
                    self.expect_type(&Type::Int, &len, args[2].span);
                    Type::String
                } else if name == "to_int" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            "to_int expects one argument and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let _ = self.check_expr(&args[0], scopes);
                    Type::Int
                } else if name == "to_float" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            "to_float expects one argument and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let _ = self.check_expr(&args[0], scopes);
                    Type::Float
                } else if name == "parse_bool" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            "parse_bool expects one argument and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let s = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &s, args[0].span);
                    Type::Result(Box::new(Type::Bool), Box::new(Type::String))
                } else if name == "panic" {
                    if args.len() != 1 {
                        self.error(expr.span, "panic expects one String message");
                        return Type::Unknown;
                    }
                    let _ = self.check_expr(&args[0], scopes);
                    Type::Unknown
                } else if name == "entries" {
                    if args.len() != 1 {
                        self.error(expr.span, "entries expects a Map");
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    match ty {
                        Type::Map(k, v) => Type::Array(Box::new(Type::Tuple(vec![*k, *v]))),
                        Type::Unknown => Type::Array(Box::new(Type::Unknown)),
                        other => {
                            self.error(
                                args[0].span,
                                format!("entries expects a Map, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "zip" {
                    if args.len() != 2 {
                        self.error(expr.span, "zip expects two Arrays");
                        return Type::Unknown;
                    }
                    let a = self.check_expr(&args[0], scopes);
                    let b = self.check_expr(&args[1], scopes);
                    match (a, b) {
                        (Type::Array(ta), Type::Array(tb)) => {
                            Type::Array(Box::new(Type::Tuple(vec![*ta, *tb])))
                        }
                        _ => Type::Array(Box::new(Type::Unknown)),
                    }
                } else if name == "dedup" {
                    if args.len() != 1 {
                        self.error(expr.span, "dedup expects an Array");
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    match ty {
                        Type::Array(inner) => Type::Array(inner),
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("dedup expects an Array, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "path_join" {
                    if args.len() != 2 {
                        self.error(expr.span, "path_join expects two String path segments");
                        return Type::Unknown;
                    }
                    let a = self.check_expr(&args[0], scopes);
                    let b = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &a, args[0].span);
                    self.expect_type(&Type::String, &b, args[1].span);
                    Type::String
                } else if matches!(
                    name.as_str(),
                    "path_filename" | "path_extension" | "path_dir"
                ) {
                    if args.len() != 1 {
                        self.error(expr.span, format!("{name} expects one String path"));
                        return Type::Unknown;
                    }
                    let s = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &s, args[0].span);
                    Type::String
                } else if name == "file_exists" || name == "dir_exists" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("{name} expects one String path"));
                        return Type::Unknown;
                    }
                    let s = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &s, args[0].span);
                    Type::Bool
                } else if name == "file_size" {
                    if args.len() != 1 {
                        self.error(expr.span, "file_size expects one String path");
                        return Type::Unknown;
                    }
                    let s = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &s, args[0].span);
                    Type::Result(Box::new(Type::Int), Box::new(Type::String))
                } else if name == "date" {
                    if args.len() != 1 {
                        self.error(expr.span, "date expects one ISO date string (YYYY-MM-DD)");
                        return Type::Unknown;
                    }
                    let value = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &value, args[0].span);
                    Type::Date
                } else if name == "decimal" {
                    if args.len() != 1 {
                        self.error(expr.span, "decimal expects one decimal string");
                        return Type::Unknown;
                    }
                    let value = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &value, args[0].span);
                    Type::Decimal
                } else if name == "time" {
                    if args.len() != 1 {
                        self.error(expr.span, "time expects one ISO time string (HH:MM:SS)");
                        return Type::Unknown;
                    }
                    let value = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &value, args[0].span);
                    Type::Time
                } else if name == "duration_ms" {
                    if args.len() != 1 {
                        self.error(expr.span, "duration_ms expects one Int");
                        return Type::Unknown;
                    }
                    let value = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::Int, &value, args[0].span);
                    Type::Duration
                } else if name == "bytes" {
                    if args.len() != 1 {
                        self.error(
                            expr.span,
                            "bytes expects one Int array with values from 0 to 255",
                        );
                        return Type::Unknown;
                    }
                    let value = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::Array(Box::new(Type::Int)), &value, args[0].span);
                    Type::Bytes
                } else if name == "format_timestamp" {
                    if args.len() != 1 {
                        self.error(expr.span, "format_timestamp expects exactly one Timestamp");
                        return Type::Unknown;
                    }
                    let timestamp = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::Timestamp, &timestamp, args[0].span);
                    Type::String
                } else if name == "format_text" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error_with_code(
                            "E-TYPE-FORMAT-001",
                            expr.span,
                            "format_text expects one scalar value and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let value = self.check_expr(&args[0], scopes);
                    if !matches!(
                        value,
                        Type::Int
                            | Type::UInt
                            | Type::Float
                            | Type::Decimal
                            | Type::Bool
                            | Type::Char
                            | Type::String
                            | Type::Timestamp
                            | Type::Date
                            | Type::Time
                            | Type::Duration
                    ) {
                        self.error_with_code(
                            "E-TYPE-FORMAT-001",
                            args[0].span,
                            format!("format_text does not support `{value}`"),
                        );
                    }
                    Type::String
                } else if name == "env" {
                    if args.len() != 1 {
                        self.error(expr.span, "env expects exactly one argument");
                        return Type::Unknown;
                    }
                    let argument = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &argument, args[0].span);
                    Type::Option(Box::new(Type::String))
                } else if name == "random_int" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "random_int expects minimum and maximum Int arguments",
                        );
                        return Type::Unknown;
                    }
                    let minimum = self.check_expr(&args[0], scopes);
                    let maximum = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Int, &minimum, args[0].span);
                    self.expect_type(&Type::Int, &maximum, args[1].span);
                    Type::Int
                } else if name == "http_get" {
                    if args.len() != 1 {
                        self.error(expr.span, "http_get expects exactly one String URL");
                        return Type::Unknown;
                    }
                    let url = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &url, args[0].span);
                    Type::String
                } else if name == "json_encode" {
                    if !type_args.is_empty() {
                        self.error(expr.span, "json_encode does not accept type arguments");
                    }
                    if args.len() != 1 {
                        self.error(expr.span, "json_encode expects exactly one value");
                        return Type::Unknown;
                    }
                    let value_type = self.check_expr(&args[0], scopes);
                    if contains_class_reference_type(&value_type, self._program) {
                        self.error_with_code(
                            "E-TYPE-CLASS-001",
                            args[0].span,
                            "class references cannot be serialized directly; convert them to a record",
                        );
                    }
                    Type::String
                } else if name == "json_encode_tagged_result" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            "json_encode_tagged_result expects one Result value and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let value = self.check_expr(&args[0], scopes);
                    if !matches!(value, Type::Result(_, _)) {
                        self.error(
                            args[0].span,
                            "json_encode_tagged_result expects a Result value",
                        );
                    }
                    if contains_class_reference_type(&value, self._program) {
                        self.error_with_code(
                            "E-TYPE-CLASS-001",
                            args[0].span,
                            "class references cannot be serialized directly; convert them to a record",
                        );
                    }
                    Type::String
                } else if name == "json_decode_tagged_result" {
                    if type_args.len() != 2 {
                        self.error(
                            expr.span,
                            "json_decode_tagged_result expects Ok and Err type arguments",
                        );
                    } else {
                        self.check_type(&type_args[0], expr.span);
                        self.check_type(&type_args[1], expr.span);
                    }
                    if args.len() != 1 {
                        self.error(
                            expr.span,
                            "json_decode_tagged_result expects one String value",
                        );
                        return Type::Unknown;
                    }
                    let source = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &source, args[0].span);
                    Type::Result(
                        Box::new(Type::Result(
                            Box::new(type_args.first().cloned().unwrap_or(Type::Unknown)),
                            Box::new(type_args.get(1).cloned().unwrap_or(Type::Unknown)),
                        )),
                        Box::new(Type::String),
                    )
                } else if name == "sql_decode" {
                    if type_args.len() != 1 || args.len() != 1 {
                        self.error_with_code(
                            "E-TYPE-SQL-DECODE",
                            expr.span,
                            "sql_decode expects one type argument and one direct SQL expression",
                        );
                        for argument in args {
                            self.check_expr(argument, scopes);
                        }
                        return Type::Unknown;
                    }
                    self.check_type(&type_args[0], expr.span);
                    self.check_expr(&args[0], scopes);
                    if let ExprKind::Sql { query, .. } = &args[0].kind {
                        if zelyra_database::sql::analyze_table_access(query, &[]).0
                            != Some("SELECT")
                        {
                            self.error_with_code(
                                "E-TYPE-SQL-DECODE",
                                args[0].span,
                                "sql_decode requires a SELECT query",
                            );
                        }
                    }
                    if !matches!(args[0].kind, ExprKind::Sql { .. }) {
                        self.error_with_code(
                            "E-TYPE-SQL-DECODE",
                            args[0].span,
                            "sql_decode requires a direct SQL expression",
                        );
                    }
                    if let Err(message) = sql_results::validate_target(&type_args[0], self._program)
                    {
                        self.error_with_code("E-TYPE-SQL-DECODE", expr.span, message);
                    }
                    type_args[0].clone()
                } else if name == "json_decode" {
                    if type_args.len() != 1 {
                        self.error(expr.span, "json_decode expects exactly one type argument");
                    } else {
                        self.check_type(&type_args[0], expr.span);
                    }
                    if args.len() != 1 {
                        self.error(expr.span, "json_decode expects exactly one String value");
                        return Type::Unknown;
                    }
                    let source = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &source, args[0].span);
                    let target = type_args.first().cloned().unwrap_or(Type::Unknown);
                    if contains_class_reference_type(&target, self._program) {
                        self.error_with_code(
                            "E-TYPE-CLASS-001",
                            expr.span,
                            "json_decode does not support class types because it cannot restore object identity",
                        );
                    }
                    target
                } else if name == "module_setting" {
                    if type_args.len() != 1 {
                        self.error_with_code(
                            "E-PKG-SETTINGS-003",
                            expr.span,
                            "module_setting expects exactly one type argument",
                        );
                    } else {
                        self.check_type(&type_args[0], expr.span);
                    }
                    if args.len() != 1 {
                        self.error_with_code(
                            "E-PKG-SETTINGS-003",
                            expr.span,
                            "module_setting expects exactly one String setting name",
                        );
                        return Type::Unknown;
                    }
                    let name_type = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &name_type, args[0].span);
                    type_args.first().cloned().unwrap_or(Type::Unknown)
                } else if name == "json_decode_result" {
                    if type_args.len() != 1 {
                        self.error(
                            expr.span,
                            "json_decode_result expects exactly one type argument",
                        );
                    } else {
                        self.check_type(&type_args[0], expr.span);
                    }
                    if args.len() != 1 {
                        self.error(
                            expr.span,
                            "json_decode_result expects exactly one String value",
                        );
                        return Type::Unknown;
                    }
                    let source = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &source, args[0].span);
                    Type::Result(
                        Box::new(type_args.first().cloned().unwrap_or(Type::Unknown)),
                        Box::new(Type::String),
                    )
                } else if name == "csv_decode_result" {
                    if !type_args.is_empty() {
                        self.error(
                            expr.span,
                            "csv_decode_result does not accept type arguments",
                        );
                    }
                    if args.len() != 2 {
                        self.error(expr.span, "csv_decode_result expects a String and Char");
                        return Type::Unknown;
                    }
                    let source = self.check_expr(&args[0], scopes);
                    let delimiter = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &source, args[0].span);
                    self.expect_type(&Type::Char, &delimiter, args[1].span);
                    Type::Result(
                        Box::new(Type::Array(Box::new(Type::Array(Box::new(Type::String))))),
                        Box::new(Type::String),
                    )
                } else if name == "csv_encode" {
                    if !type_args.is_empty() {
                        self.error(expr.span, "csv_encode does not accept type arguments");
                    }
                    if args.len() != 2 {
                        self.error(expr.span, "csv_encode expects String[][] rows and a Char");
                        return Type::Unknown;
                    }
                    let rows = self.check_expr(&args[0], scopes);
                    let delimiter = self.check_expr(&args[1], scopes);
                    self.expect_type(
                        &Type::Array(Box::new(Type::Array(Box::new(Type::String)))),
                        &rows,
                        args[0].span,
                    );
                    self.expect_type(&Type::Char, &delimiter, args[1].span);
                    Type::Result(Box::new(Type::String), Box::new(Type::String))
                } else if name == "decimal_scaled_result" {
                    if !type_args.is_empty() {
                        self.error(
                            expr.span,
                            "decimal_scaled_result does not accept type arguments",
                        );
                    }
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "decimal_scaled_result expects a String and scale Int",
                        );
                        return Type::Unknown;
                    }
                    let source = self.check_expr(&args[0], scopes);
                    let scale = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &source, args[0].span);
                    self.expect_type(&Type::Int, &scale, args[1].span);
                    Type::Result(Box::new(Type::Int), Box::new(Type::String))
                } else if name == "format_scaled_result" {
                    if !type_args.is_empty() {
                        self.error(
                            expr.span,
                            "format_scaled_result does not accept type arguments",
                        );
                    }
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "format_scaled_result expects an Int and scale Int",
                        );
                        return Type::Unknown;
                    }
                    let value = self.check_expr(&args[0], scopes);
                    let scale = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Int, &value, args[0].span);
                    self.expect_type(&Type::Int, &scale, args[1].span);
                    Type::Result(Box::new(Type::String), Box::new(Type::String))
                } else if matches!(
                    name.as_str(),
                    "date_add_days_result" | "date_days_between_result"
                ) {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            format!("{name} expects two arguments and no type arguments"),
                        );
                        return Type::Unknown;
                    }
                    let left = self.check_expr(&args[0], scopes);
                    let right = self.check_expr(&args[1], scopes);
                    if name == "date_add_days_result" {
                        self.expect_type(&Type::Date, &left, args[0].span);
                        self.expect_type(&Type::Int, &right, args[1].span);
                        Type::Result(Box::new(Type::Date), Box::new(Type::String))
                    } else {
                        self.expect_type(&Type::Date, &left, args[0].span);
                        self.expect_type(&Type::Date, &right, args[1].span);
                        Type::Result(Box::new(Type::Int), Box::new(Type::String))
                    }
                } else if matches!(
                    name.as_str(),
                    "timestamp_add_result" | "timestamp_difference_result"
                ) {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            format!("{name} expects two arguments and no type arguments"),
                        );
                        return Type::Unknown;
                    }
                    let left = self.check_expr(&args[0], scopes);
                    let right = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Timestamp, &left, args[0].span);
                    if name == "timestamp_add_result" {
                        self.expect_type(&Type::Duration, &right, args[1].span);
                        Type::Result(Box::new(Type::Timestamp), Box::new(Type::String))
                    } else {
                        self.expect_type(&Type::Timestamp, &right, args[1].span);
                        Type::Result(Box::new(Type::Duration), Box::new(Type::String))
                    }
                } else if name == "decimal_quantize_result" {
                    if !type_args.is_empty() || args.len() != 3 {
                        self.error(
                            expr.span,
                            "decimal_quantize_result expects Decimal, scale Int, and mode String",
                        );
                        return Type::Unknown;
                    }
                    let value = self.check_expr(&args[0], scopes);
                    let scale = self.check_expr(&args[1], scopes);
                    let mode = self.check_expr(&args[2], scopes);
                    self.expect_type(&Type::Decimal, &value, args[0].span);
                    self.expect_type(&Type::Int, &scale, args[1].span);
                    self.expect_type(&Type::String, &mode, args[2].span);
                    Type::Result(Box::new(Type::Decimal), Box::new(Type::String))
                } else if name == "http_json" {
                    if type_args.len() != 2 {
                        self.error(
                            expr.span,
                            "http_json expects request and response type arguments",
                        );
                    } else {
                        self.check_type(&type_args[0], expr.span);
                        self.check_type(&type_args[1], expr.span);
                    }
                    if args.len() != 4 {
                        self.error(
                            expr.span,
                            "http_json expects String method, String URL, String[] headers, and Request? body",
                        );
                        return Type::Unknown;
                    }
                    let method = self.check_expr(&args[0], scopes);
                    let url = self.check_expr(&args[1], scopes);
                    let headers = self.check_expr(&args[2], scopes);
                    let body = self.check_expr(&args[3], scopes);
                    self.expect_type(&Type::String, &method, args[0].span);
                    self.expect_type(&Type::String, &url, args[1].span);
                    self.expect_type(&Type::Array(Box::new(Type::String)), &headers, args[2].span);
                    if let Some(request_type) = type_args.first() {
                        self.expect_type(
                            &Type::Option(Box::new(request_type.clone())),
                            &body,
                            args[3].span,
                        );
                    }
                    type_args.get(1).cloned().unwrap_or(Type::Unknown)
                } else if name == "http_result" {
                    if type_args.len() != 2 {
                        self.error(
                            expr.span,
                            "http_result expects request and response type arguments",
                        );
                    } else {
                        self.check_type(&type_args[0], expr.span);
                        self.check_type(&type_args[1], expr.span);
                    }
                    if args.len() != 4 {
                        self.error(
                            expr.span,
                            "http_result expects String method, String URL, String[] headers, and Request? body",
                        );
                        return Type::Unknown;
                    }
                    let method = self.check_expr(&args[0], scopes);
                    let url = self.check_expr(&args[1], scopes);
                    let headers = self.check_expr(&args[2], scopes);
                    let body = self.check_expr(&args[3], scopes);
                    self.expect_type(&Type::String, &method, args[0].span);
                    self.expect_type(&Type::String, &url, args[1].span);
                    self.expect_type(&Type::Array(Box::new(Type::String)), &headers, args[2].span);
                    if let Some(request_type) = type_args.first() {
                        self.expect_type(
                            &Type::Option(Box::new(request_type.clone())),
                            &body,
                            args[3].span,
                        );
                    }
                    Type::HttpResult(Box::new(type_args.get(1).cloned().unwrap_or(Type::Unknown)))
                } else if name == "http_request" {
                    if args.len() != 4 {
                        self.error(
                            expr.span,
                            "http_request expects String method, String URL, String[] headers, and String? body",
                        );
                        return Type::Unknown;
                    }
                    let method = self.check_expr(&args[0], scopes);
                    let url = self.check_expr(&args[1], scopes);
                    let headers = self.check_expr(&args[2], scopes);
                    let body = self.check_expr(&args[3], scopes);
                    self.expect_type(&Type::String, &method, args[0].span);
                    self.expect_type(&Type::String, &url, args[1].span);
                    self.expect_type(&Type::Array(Box::new(Type::String)), &headers, args[2].span);
                    self.expect_type(&Type::Option(Box::new(Type::String)), &body, args[3].span);
                    Type::Named("HttpResponse".into())
                } else if name == "mail_queue" {
                    if !type_args.is_empty() {
                        self.error(expr.span, "mail_queue does not accept type arguments");
                    }
                    if self.transaction_depth == 0 {
                        self.error(expr.span, "mail_queue is only allowed inside a transaction");
                    }
                    if !matches!(args.len(), 3..=6) {
                        self.error(
                            expr.span,
                            "mail_queue expects recipient, subject, text body, optional HTML body, and optional PDF filename and Bytes",
                        );
                        return Type::Unknown;
                    }
                    for (index, argument) in args.iter().enumerate() {
                        let is_pdf_argument = matches!((args.len(), index), (5, 4) | (6, 5));
                        if is_pdf_argument
                            && !matches!(
                                &argument.kind,
                                ExprKind::Call { name, .. } if name == "render_pdf"
                            )
                        {
                            self.error(
                                argument.span,
                                "mail_queue PDF attachments must be produced directly by render_pdf",
                            );
                        }
                        if !is_pure_mail_expression(argument) {
                            self.error(
                                argument.span,
                                "mail_queue arguments must be values or pure expressions; PDF attachments must use render_pdf",
                            );
                        }
                        let actual = self.check_expr(argument, scopes);
                        let expected = match (args.len(), index) {
                            (5, 4) | (6, 5) => Type::Bytes,
                            _ => Type::String,
                        };
                        self.expect_type(&expected, &actual, argument.span);
                    }
                    Type::Unit
                } else if name == "mail_deliver" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "mail_deliver expects an outbox event ID and payload JSON",
                        );
                        return Type::Unknown;
                    }
                    for argument in args {
                        let actual = self.check_expr(argument, scopes);
                        self.expect_type(&Type::String, &actual, argument.span);
                    }
                    Type::Result(Box::new(Type::Unit), Box::new(Type::String))
                } else if name == "run_process" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "run_process expects a String command and String[] arguments",
                        );
                        return Type::Unknown;
                    }
                    let command = self.check_expr(&args[0], scopes);
                    let arguments = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &command, args[0].span);
                    self.expect_type(
                        &Type::Array(Box::new(Type::String)),
                        &arguments,
                        args[1].span,
                    );
                    Type::String
                } else if name == "run_process_piped" {
                    if args.len() != 4 {
                        self.error(
                            expr.span,
                            "run_process_piped expects command, arguments, stdin, and timeout_ms",
                        );
                        return Type::Unknown;
                    }
                    let command = self.check_expr(&args[0], scopes);
                    let arguments = self.check_expr(&args[1], scopes);
                    let stdin = self.check_expr(&args[2], scopes);
                    let timeout_ms = self.check_expr(&args[3], scopes);
                    self.expect_type(&Type::String, &command, args[0].span);
                    self.expect_type(
                        &Type::Array(Box::new(Type::String)),
                        &arguments,
                        args[1].span,
                    );
                    self.expect_type(&Type::String, &stdin, args[2].span);
                    self.expect_type(&Type::Int, &timeout_ms, args[3].span);
                    Type::Named("ProcessResult".into())
                } else if name == "read_text" {
                    if args.len() != 1 {
                        self.error(expr.span, "read_text expects exactly one String path");
                        return Type::Unknown;
                    }
                    let path = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &path, args[0].span);
                    Type::String
                } else if name == "read_text_result" {
                    if args.len() != 1 {
                        self.error(
                            expr.span,
                            "read_text_result expects exactly one String path",
                        );
                        return Type::Unknown;
                    }
                    let path = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &path, args[0].span);
                    Type::Result(Box::new(Type::String), Box::new(Type::String))
                } else if name == "write_text" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "write_text expects a String path and String content",
                        );
                        return Type::Unknown;
                    }
                    let path = self.check_expr(&args[0], scopes);
                    let content = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &path, args[0].span);
                    self.expect_type(&Type::String, &content, args[1].span);
                    Type::Unit
                } else if name == "write_text_result" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "write_text_result expects a String path and String content",
                        );
                        return Type::Unknown;
                    }
                    let path = self.check_expr(&args[0], scopes);
                    let content = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &path, args[0].span);
                    self.expect_type(&Type::String, &content, args[1].span);
                    Type::Result(Box::new(Type::Unit), Box::new(Type::String))
                } else if matches!(
                    name.as_str(),
                    "write_text_new_result" | "atomic_write_text_result" | "append_text_result"
                ) {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            format!("{name} expects a String path and String content"),
                        );
                        return Type::Unknown;
                    }
                    let path = self.check_expr(&args[0], scopes);
                    let content = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &path, args[0].span);
                    self.expect_type(&Type::String, &content, args[1].span);
                    Type::Result(Box::new(Type::Unit), Box::new(Type::String))
                } else if matches!(name.as_str(), "copy_file_new_result" | "rename_file_result") {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            format!("{name} expects source and destination String paths"),
                        );
                        return Type::Unknown;
                    }
                    let source = self.check_expr(&args[0], scopes);
                    let destination = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &source, args[0].span);
                    self.expect_type(&Type::String, &destination, args[1].span);
                    Type::Result(Box::new(Type::Unit), Box::new(Type::String))
                } else if name == "delete_file" || name == "delete_file_result" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("{name} expects one String path"));
                        return Type::Unknown;
                    }
                    let path = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &path, args[0].span);
                    if name == "delete_file_result" {
                        Type::Result(Box::new(Type::Unit), Box::new(Type::String))
                    } else {
                        Type::Unit
                    }
                } else if name == "list_dir" {
                    if args.len() != 1 {
                        self.error(expr.span, "list_dir expects one String path");
                        return Type::Unknown;
                    }
                    let path = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &path, args[0].span);
                    Type::Array(Box::new(Type::String))
                } else if let Some(variable) = self.lookup(scopes, name).cloned() {
                    let Type::Function {
                        params,
                        return_type,
                        ..
                    } = variable.ty
                    else {
                        self.error(expr.span, format!("value `{name}` is not callable"));
                        return Type::Unknown;
                    };
                    if !type_args.is_empty() {
                        self.error_with_code(
                            "E-TYPE-FUNCTION-001",
                            expr.span,
                            "indirect function calls do not accept explicit type arguments",
                        );
                    }
                    if args.len() != params.len() {
                        self.error(
                            expr.span,
                            format!(
                                "function value `{name}` expects {} argument(s), found {}",
                                params.len(),
                                args.len()
                            ),
                        );
                    }
                    for (index, argument) in args.iter().enumerate() {
                        let actual = self.check_expr(argument, scopes);
                        if let Some(expected) = params.get(index) {
                            self.expect_type(expected, &actual, argument.span);
                        }
                    }
                    *return_type
                } else if let Some(signature) = self.functions.get(name).cloned() {
                    if args.len() != signature.params.len() {
                        self.error(
                            expr.span,
                            format!(
                                "function `{name}` expects {} argument(s), found {}",
                                signature.params.len(),
                                args.len()
                            ),
                        );
                    }
                    let actual_types = args
                        .iter()
                        .map(|argument| self.check_expr(argument, scopes))
                        .collect::<Vec<_>>();
                    if signature.type_params.is_empty() {
                        if !type_args.is_empty() {
                            self.error(
                                expr.span,
                                format!("function `{name}` does not accept type arguments"),
                            );
                        }
                        for (index, actual) in actual_types.iter().enumerate() {
                            if let Some(expected) = signature.params.get(index) {
                                self.expect_type(expected, actual, args[index].span);
                            }
                        }
                        signature.return_type
                    } else {
                        let parameters = signature
                            .type_params
                            .iter()
                            .cloned()
                            .collect::<HashSet<_>>();
                        let mut substitutions = HashMap::new();
                        if !type_args.is_empty() {
                            if type_args.len() != signature.type_params.len() {
                                self.error_with_code(
                                    "E-TYPE-GENERIC-001",
                                    expr.span,
                                    format!(
                                        "generic function `{name}` expects {} type argument(s), found {}",
                                        signature.type_params.len(),
                                        type_args.len()
                                    ),
                                );
                            }
                            for argument in type_args {
                                self.check_type(argument, expr.span);
                            }
                            for (parameter, argument) in signature.type_params.iter().zip(type_args)
                            {
                                substitutions.insert(parameter.clone(), argument.clone());
                            }
                        }
                        for (index, actual) in actual_types.iter().enumerate() {
                            if let Some(expected) = signature.params.get(index) {
                                if type_args.is_empty() {
                                    infer_generic_types(
                                        expected,
                                        actual,
                                        &parameters,
                                        &mut substitutions,
                                    );
                                }
                            }
                        }
                        for parameter in &signature.type_params {
                            if !substitutions.contains_key(parameter) {
                                self.error_with_code(
                                    "E-TYPE-GENERIC-001",
                                    expr.span,
                                    format!(
                                        "cannot infer type parameter `{parameter}` for `{name}`; provide explicit type arguments"
                                    ),
                                );
                                substitutions.insert(parameter.clone(), Type::Unknown);
                            }
                        }
                        for (index, actual) in actual_types.iter().enumerate() {
                            if let Some(expected) = signature.params.get(index) {
                                let expected = substitute_generic_type(expected, &substitutions);
                                self.expect_type(&expected, actual, args[index].span);
                            }
                        }
                        substitute_generic_type(&signature.return_type, &substitutions)
                    }
                } else {
                    self.error(expr.span, format!("unknown function `{name}`"));
                    Type::Unknown
                }
            }
            ExprKind::Unary { op, expr: inner } => {
                let ty = self.check_expr(inner, scopes);
                match op {
                    UnaryOp::Negate => {
                        if !is_numeric(&ty) && ty != Type::Unknown {
                            self.error(expr.span, "unary `-` requires a numeric value");
                        }
                        ty
                    }
                    UnaryOp::Not => {
                        self.expect_type(&Type::Bool, &ty, inner.span);
                        Type::Bool
                    }
                }
            }
            ExprKind::Index { target, index } => {
                let target_type = self.check_expr(target, scopes);
                let index_type = self.check_expr(index, scopes);
                if !matches!(index_type, Type::Int | Type::UInt | Type::Unknown) {
                    self.error(
                        index.span,
                        format!("array index must be an integer, found `{index_type}`"),
                    );
                }
                match target_type {
                    Type::Array(inner) => *inner,
                    Type::Tuple(items) => match index.kind {
                        ExprKind::Int(value) if value >= 0 => {
                            items.get(value as usize).cloned().unwrap_or_else(|| {
                                self.error(
                                    index.span,
                                    format!("tuple index {value} is out of bounds"),
                                );
                                Type::Unknown
                            })
                        }
                        ExprKind::UInt(value) => {
                            items.get(value as usize).cloned().unwrap_or_else(|| {
                                self.error(
                                    index.span,
                                    format!("tuple index {value} is out of bounds"),
                                );
                                Type::Unknown
                            })
                        }
                        _ => {
                            self.error(
                                index.span,
                                "tuple index must be a non-negative integer literal",
                            );
                            Type::Unknown
                        }
                    },
                    Type::Unknown => Type::Unknown,
                    other => {
                        self.error(
                            target.span,
                            format!("indexing requires an array or Tuple, found `{other}`"),
                        );
                        Type::Unknown
                    }
                }
            }
            ExprKind::Field { target, field } => {
                let target_type = self.check_expr(target, scopes);
                match self.record_field_type(&target_type, field) {
                    Some(field_type) => field_type,
                    None if target_type == Type::Unknown => Type::Unknown,
                    None => {
                        self.error(
                            expr.span,
                            format!("unknown field `{field}` on `{target_type}`"),
                        );
                        Type::Unknown
                    }
                }
            }
            ExprKind::Binary { left, op, right } => {
                let _ = (left, op, right);
                self.check_binary_chain(expr, scopes)
            }
            ExprKind::Sql { result_type, .. } => {
                self.check_type(result_type, expr.span);
                result_type.clone()
            }
            ExprKind::Await(inner) => {
                if self.parallel_depth == 0 {
                    self.error(expr.span, "`await` is only valid inside a `parallel` block");
                }
                self.check_expr(inner, scopes)
            }
            ExprKind::Match { value, arms } => {
                let value_type = self.check_expr(value, scopes);
                let mut covered = std::collections::HashSet::new();
                let mut result_type = Type::Unknown;
                for arm in arms {
                    let mut pattern_scope = HashMap::new();
                    self.check_pattern(
                        &arm.pattern,
                        &value_type,
                        &mut pattern_scope,
                        &mut covered,
                        arm.guard.is_none(),
                    );
                    let mut arm_scopes = scopes.to_vec();
                    arm_scopes.push(pattern_scope);
                    if let Some(guard) = &arm.guard {
                        let guard_type = self.check_expr(guard, &arm_scopes);
                        self.expect_type(&Type::Bool, &guard_type, guard.span);
                    }
                    let arm_type = self.check_expr(&arm.value, &arm_scopes);
                    if result_type == Type::Unknown {
                        result_type = arm_type;
                    } else {
                        self.expect_type(&result_type, &arm_type, arm.value.span);
                    }
                }
                self.check_exhaustiveness(&value_type, &covered, expr.span);
                result_type
            }
        }
    }
    fn check_binary_chain(&mut self, expr: &Expr, scopes: &[HashMap<String, Variable>]) -> Type {
        let mut cursor = expr;
        let mut operations = Vec::new();
        while let ExprKind::Binary { left, op, right } = &cursor.kind {
            operations.push((*op, right.as_ref(), cursor.span, left.span));
            cursor = left;
        }
        let mut left_type = self.check_expr(cursor, scopes);
        for (op, right, span, left_span) in operations.into_iter().rev() {
            let right_type = if matches!(op, BinaryOp::Equal | BinaryOp::NotEqual) {
                self.check_expr_expected(right, scopes, Some(&left_type))
            } else {
                self.check_expr(right, scopes)
            };
            left_type = match op {
                BinaryOp::Add => {
                    if left_type == Type::String && right_type == Type::String {
                        Type::String
                    } else if left_type == Type::Duration && right_type == Type::Duration {
                        Type::Duration
                    } else if let (Type::Array(left), Type::Array(right)) =
                        (&left_type, &right_type)
                    {
                        self.expect_type(left, right, span);
                        Type::Array(left.clone())
                    } else {
                        self.numeric_result(&left_type, &right_type, span)
                    }
                }
                BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::Divide
                | BinaryOp::Remainder => {
                    if op == BinaryOp::Subtract
                        && left_type == Type::Duration
                        && right_type == Type::Duration
                    {
                        Type::Duration
                    } else {
                        self.numeric_result(&left_type, &right_type, span)
                    }
                }
                BinaryOp::Equal | BinaryOp::NotEqual => {
                    if matches!(&left_type, Type::Function { .. })
                        || matches!(&right_type, Type::Function { .. })
                    {
                        self.error_with_code(
                            "E-TYPE-FUNCTION-001",
                            span,
                            "function values cannot be compared",
                        );
                    }
                    self.expect_type(&left_type, &right_type, right.span);
                    Type::Bool
                }
                BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual => {
                    if left_type == right_type
                        && matches!(
                            left_type,
                            Type::String
                                | Type::Date
                                | Type::Time
                                | Type::Duration
                                | Type::Timestamp
                        )
                    {
                        Type::Bool
                    } else {
                        self.numeric_result(&left_type, &right_type, span);
                        Type::Bool
                    }
                }
                BinaryOp::And | BinaryOp::Or => {
                    self.expect_type(&Type::Bool, &left_type, left_span);
                    self.expect_type(&Type::Bool, &right_type, right.span);
                    Type::Bool
                }
            };
        }
        left_type
    }
    fn numeric_result(&mut self, left: &Type, right: &Type, span: Span) -> Type {
        if left == &Type::Unknown || right == &Type::Unknown {
            return Type::Unknown;
        }
        if is_numeric(left) && left == right {
            left.clone()
        } else {
            self.error(
                span,
                format!("numeric operands must have the same type, found `{left}` and `{right}`"),
            );
            Type::Unknown
        }
    }

    fn record_field_type(&self, ty: &Type, field: &str) -> Option<Type> {
        if let Type::HttpResult(response) = ty {
            return match field {
                "status" => Some(Type::Int),
                "headers" => Some(Type::Array(Box::new(Type::String))),
                "body" => Some(Type::String),
                "data" => Some(Type::Option(response.clone())),
                "error" => Some(Type::Option(Box::new(Type::Named("HttpError".into())))),
                _ => None,
            };
        }
        let (initial_name, substitutions) = match ty {
            Type::Named(name) => (name.as_str(), HashMap::new()),
            Type::Applied { name, args } => {
                let record = self
                    ._program
                    .records
                    .iter()
                    .find(|record| record.name == *name)?;
                let substitutions = record
                    .type_params
                    .iter()
                    .zip(args)
                    .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                    .collect::<HashMap<_, _>>();
                let field_type = record
                    .fields
                    .iter()
                    .find(|candidate| candidate.name == field)?
                    .ty
                    .clone();
                return Some(substitute_generic_type(&field_type, &substitutions));
            }
            _ => return None,
        };
        if initial_name == "HttpError" {
            return match field {
                "status" => Some(Type::Int),
                "headers" => Some(Type::Array(Box::new(Type::String))),
                "body" | "message" => Some(Type::String),
                _ => None,
            };
        }
        let mut name = initial_name;
        if name == "HttpResponse" {
            return match field {
                "status" => Some(Type::Int),
                "headers" => Some(Type::Array(Box::new(Type::String))),
                "body" => Some(Type::String),
                _ => None,
            };
        }
        if name == "ProcessResult" {
            return match field {
                "exit_code" => Some(Type::Int),
                "stdout" | "stderr" => Some(Type::String),
                _ => None,
            };
        }
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(name) {
                return None;
            }
            if let Some(record) = self
                ._program
                .records
                .iter()
                .find(|record| record.name == name)
            {
                return record
                    .fields
                    .iter()
                    .find(|candidate| candidate.name == field)
                    .map(|candidate| substitute_generic_type(&candidate.ty, &substitutions));
            }
            if let Some(record) = sql_results::definitions(self._program).get(name) {
                return record
                    .fields
                    .iter()
                    .find(|candidate| candidate.name == field)
                    .map(|candidate| candidate.ty.clone());
            }
            let alias = self
                ._program
                .types
                .iter()
                .find(|definition| definition.name == name)?;
            let Type::Named(next_name) = &alias.target else {
                return None;
            };
            name = next_name;
        }
    }
    pub(super) fn check_expr(&mut self, expr: &Expr, scopes: &[HashMap<String, Variable>]) -> Type {
        self.check_expr_expected(expr, scopes, None)
    }

    pub(super) fn check_expr_expected(
        &mut self,
        expr: &Expr,
        scopes: &[HashMap<String, Variable>],
        expected: Option<&Type>,
    ) -> Type {
        let contextual_type = expected.map(|ty| self.normalize_alias(ty, expr.span));
        let ty = match (&expr.kind, contextual_type) {
            (ExprKind::Array(values), Some(Type::Array(expected_element))) => {
                for value in values {
                    let actual = self.check_expr_expected(value, scopes, Some(&expected_element));
                    self.expect_type(&expected_element, &actual, value.span);
                }
                Type::Array(expected_element)
            }
            (ExprKind::Map(entries), Some(Type::Map(expected_key, expected_value))) => {
                for (key, value) in entries {
                    let actual_key = self.check_expr_expected(key, scopes, Some(&expected_key));
                    let actual_value =
                        self.check_expr_expected(value, scopes, Some(&expected_value));
                    self.expect_type(&expected_key, &actual_key, key.span);
                    self.expect_type(&expected_value, &actual_value, value.span);
                }
                Type::Map(expected_key, expected_value)
            }
            (ExprKind::Call { name, args, .. }, Some(Type::Option(expected_value)))
                if name == "Some" && args.len() == 1 =>
            {
                let actual = self.check_expr_expected(&args[0], scopes, Some(&expected_value));
                self.expect_type(&expected_value, &actual, args[0].span);
                Type::Option(expected_value)
            }
            (ExprKind::Call { name, args, .. }, Some(Type::Option(expected_value)))
                if name == "None" && args.is_empty() =>
            {
                Type::Option(expected_value)
            }
            (ExprKind::Variable(name), Some(Type::Option(expected_value))) if name == "None" => {
                Type::Option(expected_value)
            }
            (
                ExprKind::Call { name, args, .. },
                Some(Type::Result(expected_ok, expected_error)),
            ) if (name == "Ok" || name == "Err") && args.len() == 1 => {
                let expected_argument = if name == "Ok" {
                    &expected_ok
                } else {
                    &expected_error
                };
                let actual = self.check_expr_expected(&args[0], scopes, Some(expected_argument));
                self.expect_type(expected_argument, &actual, args[0].span);
                Type::Result(expected_ok, expected_error)
            }
            (ExprKind::Match { value, arms }, Some(expected_result)) => {
                let value_type = self.check_expr(value, scopes);
                let mut covered = std::collections::HashSet::new();
                for arm in arms {
                    let mut pattern_scope = HashMap::new();
                    self.check_pattern(
                        &arm.pattern,
                        &value_type,
                        &mut pattern_scope,
                        &mut covered,
                        arm.guard.is_none(),
                    );
                    let mut arm_scopes = scopes.to_vec();
                    arm_scopes.push(pattern_scope);
                    if let Some(guard) = &arm.guard {
                        let guard_type = self.check_expr(guard, &arm_scopes);
                        self.expect_type(&Type::Bool, &guard_type, guard.span);
                    }
                    let arm_type =
                        self.check_expr_expected(&arm.value, &arm_scopes, Some(&expected_result));
                    self.expect_type(&expected_result, &arm_type, arm.value.span);
                }
                self.check_exhaustiveness(&value_type, &covered, expr.span);
                expected_result
            }
            (
                ExprKind::Lambda { params, body },
                Some(Type::Function {
                    params: expected_params,
                    return_type: expected_return,
                    capabilities,
                }),
            ) => {
                if params.len() != expected_params.len() {
                    self.error(
                        expr.span,
                        format!(
                            "lambda expects {} parameter(s) from its function type, found {}",
                            expected_params.len(),
                            params.len()
                        ),
                    );
                }
                let mut parameter_scope = HashMap::new();
                let mut parameter_types = Vec::with_capacity(params.len());
                for (index, parameter) in params.iter().enumerate() {
                    let expected_parameter =
                        expected_params.get(index).cloned().unwrap_or(Type::Unknown);
                    let parameter_type = if parameter.ty == Type::Unknown {
                        expected_parameter
                    } else {
                        self.expect_type(&expected_parameter, &parameter.ty, parameter.span);
                        parameter.ty.clone()
                    };
                    parameter_types.push(parameter_type.clone());
                    parameter_scope.insert(
                        parameter.name.clone(),
                        Variable {
                            ty: parameter_type,
                            mutable: false,
                        },
                    );
                }
                let mut lambda_scopes = lambda_capture_scopes(scopes);
                lambda_scopes.push(parameter_scope);
                let inferred_return =
                    self.check_expr_expected(body, &lambda_scopes, Some(&expected_return));
                self.expect_type(&expected_return, &inferred_return, body.span);
                let required_capabilities =
                    self.expression_required_capabilities(body, &lambda_scopes);
                let declared_capabilities = capabilities
                    .iter()
                    .map(String::as_str)
                    .collect::<HashSet<_>>();
                for capability in required_capabilities {
                    if !function_declares_capability(&declared_capabilities, &capability) {
                        self.error_with_code(
                            "E-TYPE-FUNCTION-001",
                            expr.span,
                            format!(
                                "lambda requires capability `{capability}`; declare it in the function type with `uses {{{capability}}}`"
                            ),
                        );
                    }
                }
                Type::Function {
                    params: parameter_types,
                    return_type: Box::new(if inferred_return == Type::Unknown {
                        *expected_return
                    } else {
                        inferred_return
                    }),
                    capabilities,
                }
            }
            _ => self.check_expr_inner(expr, scopes),
        };
        let ty = self.normalize_alias(&ty, expr.span);
        self.record_inferred_expression_type(expr.span, &ty);
        ty
    }

    /// Find effects executed by a lambda body without descending into nested
    /// lambda bodies, which are separate function values with their own
    /// capability contracts.
    fn expression_required_capabilities(
        &self,
        expression: &Expr,
        scopes: &[HashMap<String, Variable>],
    ) -> HashSet<String> {
        let mut required = HashSet::new();
        let mut pending = vec![expression];
        while let Some(expression) = pending.pop() {
            match &expression.kind {
                ExprKind::Sql { query, .. } => {
                    required.insert(sql_required_capability(query).to_owned());
                }
                ExprKind::Call { name, args, .. } => {
                    required.extend(
                        builtin_capability_requirements(name)
                            .iter()
                            .map(|(capability, _)| (*capability).to_owned()),
                    );
                    if let Some(signature) = self.functions.get(name) {
                        required.extend(signature.capabilities.iter().cloned());
                    } else if let Some(Type::Function { capabilities, .. }) =
                        self.lookup(scopes, name).map(|variable| &variable.ty)
                    {
                        required.extend(capabilities.iter().cloned());
                    }
                    pending.extend(args);
                }
                ExprKind::Array(values) => pending.extend(values),
                ExprKind::Map(entries) => {
                    for (key, value) in entries {
                        pending.push(key);
                        pending.push(value);
                    }
                }
                ExprKind::Record { fields, .. } => {
                    pending.extend(fields.iter().map(|(_, value)| value));
                }
                ExprKind::Index { target, index } => {
                    pending.push(target);
                    pending.push(index);
                }
                ExprKind::Field { target, .. } | ExprKind::Unary { expr: target, .. } => {
                    pending.push(target);
                }
                ExprKind::Binary { left, right, .. } => {
                    pending.push(left);
                    pending.push(right);
                }
                ExprKind::Await(inner) => pending.push(inner),
                ExprKind::Match { value, arms } => {
                    pending.push(value);
                    pending.extend(arms.iter().map(|arm| &arm.value));
                }
                ExprKind::Lambda { .. }
                | ExprKind::Int(_)
                | ExprKind::UInt(_)
                | ExprKind::Float(_)
                | ExprKind::Bool(_)
                | ExprKind::String(_)
                | ExprKind::Char(_)
                | ExprKind::FunctionValue(_)
                | ExprKind::Variable(_) => {}
            }
        }
        required
    }

    fn record_inferred_expression_type(&mut self, span: Span, ty: &Type) {
        if self.capture_inferred_expression_types {
            self.inferred_expression_types.push(InferredExpressionType {
                span,
                ty: ty.clone(),
            });
        }
    }
    fn check_callable(
        &mut self,
        expression: &Expr,
        expected_params: &[Type],
        scopes: &[HashMap<String, Variable>],
    ) -> Type {
        if let ExprKind::Lambda { params, body } = &expression.kind {
            if params.len() != expected_params.len() {
                self.error(
                    expression.span,
                    format!(
                        "lambda expects {} parameter(s) in this operation, found {}",
                        expected_params.len(),
                        params.len()
                    ),
                );
            }
            let mut parameter_scope = HashMap::new();
            let mut parameter_types = Vec::new();
            for (index, parameter) in params.iter().enumerate() {
                let expected = expected_params.get(index).cloned().unwrap_or(Type::Unknown);
                let ty = if parameter.ty == Type::Unknown {
                    expected
                } else {
                    self.expect_type(&expected, &parameter.ty, parameter.span);
                    parameter.ty.clone()
                };
                parameter_types.push(ty.clone());
                parameter_scope.insert(parameter.name.clone(), Variable { ty, mutable: false });
            }
            let mut lambda_scopes = lambda_capture_scopes(scopes);
            lambda_scopes.push(parameter_scope);
            let return_type = self.check_expr(body, &lambda_scopes);
            let callback = Type::Function {
                params: parameter_types,
                return_type: Box::new(return_type),
                capabilities: Vec::new(),
            };
            self.record_inferred_expression_type(expression.span, &callback);
            return callback;
        }
        let callable = self.check_expr(expression, scopes);
        let Type::Function {
            params,
            return_type,
            ..
        } = callable
        else {
            self.error(
                expression.span,
                "higher-order operation expects a function or lambda",
            );
            return Type::Function {
                params: expected_params.to_vec(),
                return_type: Box::new(Type::Unknown),
                capabilities: Vec::new(),
            };
        };
        if params.len() != expected_params.len() {
            self.error(
                expression.span,
                format!(
                    "callback expects {} parameter(s), expected {}",
                    params.len(),
                    expected_params.len()
                ),
            );
        }
        for (expected, actual) in expected_params.iter().zip(&params) {
            self.expect_type(expected, actual, expression.span);
        }
        Type::Function {
            params,
            return_type,
            capabilities: Vec::new(),
        }
    }

    fn check_higher_order_call(
        &mut self,
        name: &str,
        type_args: &[Type],
        args: &[Expr],
        span: Span,
        scopes: &[HashMap<String, Variable>],
    ) -> Type {
        if !type_args.is_empty() {
            self.error(
                span,
                format!("`{name}` does not accept explicit type arguments"),
            );
        }
        let (expected_count, callback_index) = match name {
            "map" | "filter" | "find" => (2, 1),
            "reduce" => (3, 2),
            _ => unreachable!(),
        };
        if args.len() != expected_count {
            self.error(
                span,
                format!("`{name}` expects {expected_count} argument(s)"),
            );
            return Type::Unknown;
        }
        let (items, item_type) = match self.check_expr(&args[0], scopes) {
            Type::Array(item) => (true, *item),
            Type::Unknown => (true, Type::Unknown),
            other => {
                self.error(
                    args[0].span,
                    format!("`{name}` expects an array, found `{other}`"),
                );
                (false, Type::Unknown)
            }
        };
        if !items {
            return Type::Unknown;
        }
        let callback_params = if name == "reduce" {
            let accumulator_type = self.check_expr(&args[1], scopes);
            vec![accumulator_type.clone(), item_type.clone()]
        } else {
            vec![item_type.clone()]
        };
        let callback = self.check_callable(&args[callback_index], &callback_params, scopes);
        let Type::Function { return_type, .. } = callback else {
            unreachable!()
        };
        match name {
            "map" => Type::Array(return_type),
            "filter" => {
                self.expect_type(&Type::Bool, &return_type, args[callback_index].span);
                Type::Array(Box::new(item_type))
            }
            "find" => {
                self.expect_type(&Type::Bool, &return_type, args[callback_index].span);
                Type::Option(Box::new(item_type))
            }
            "reduce" => {
                let accumulator_type = self.check_expr(&args[1], scopes);
                self.expect_type(&accumulator_type, &return_type, args[callback_index].span);
                accumulator_type
            }
            _ => Type::Unknown,
        }
    }
}
