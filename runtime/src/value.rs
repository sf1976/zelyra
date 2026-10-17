//! Shared runtime values and identity-preserving object references.

use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct SqlRowsMetadata {
    pub nulls: Vec<Vec<bool>>,
    pub column_types: Vec<zelyra_database::QueryColumnType>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    UInt(u64),
    Float(f64),
    Decimal(i128),
    Bool(bool),
    String(String),
    Char(char),
    Enum {
        type_name: String,
        type_args: Vec<Type>,
        variant: String,
        payloads: Vec<Value>,
    },
    Timestamp(i64),
    Date(String),
    Time(String),
    Duration(i64),
    Bytes(Vec<u8>),
    Json(serde_json::Value),
    Array(Vec<Value>),
    Tuple(Vec<Value>),
    Set(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Function(Box<FunctionValue>),
    Object {
        type_name: String,
        type_args: Vec<Type>,
        fields: HashMap<String, Value>,
    },
    ClassRef {
        reference: ClassObjectRef,
        type_name: String,
        type_args: Vec<Type>,
    },
    Option(Option<Box<Value>>),
    Result(Result<Box<Value>, Box<Value>>),
    Rows {
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
        metadata: Box<SqlRowsMetadata>,
        result_type: Box<Type>,
    },
    Unit,
}

#[derive(Clone, Debug)]
pub struct FunctionValue {
    pub name: String,
    pub params: Vec<Type>,
    pub return_type: Type,
    pub capabilities: Vec<String>,
    pub lambda_params: Vec<String>,
    pub lambda_body: Option<Box<Expr>>,
    pub captures: Vec<(String, Value)>,
    /// Index into the currently executing verified bytecode program. AST and
    /// MIR function values leave this unset and cannot be called by the VM.
    pub bytecode_target: Option<usize>,
    /// Captured VM locals for a bytecode closure, keyed by verified LocalId.
    /// Named function values and AST/MIR closures leave this empty.
    pub bytecode_captures: Vec<(usize, Value)>,
}

impl PartialEq for FunctionValue {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.params == other.params
            && self.return_type == other.return_type
            && self.capabilities == other.capabilities
            && self.lambda_params == other.lambda_params
            && self.captures == other.captures
            && self.bytecode_target == other.bytecode_target
            && self.bytecode_captures == other.bytecode_captures
            && self.lambda_body.is_some() == other.lambda_body.is_some()
    }
}

impl Value {
    pub fn ty(&self) -> Type {
        match self {
            Value::Int(_) => Type::Int,
            Value::UInt(_) => Type::UInt,
            Value::Float(_) => Type::Float,
            Value::Decimal(_) => Type::Decimal,
            Value::Bool(_) => Type::Bool,
            Value::String(_) => Type::String,
            Value::Char(_) => Type::Char,
            Value::Enum {
                type_name,
                type_args,
                ..
            } => {
                if type_args.is_empty() {
                    Type::Named(type_name.clone())
                } else {
                    Type::Applied {
                        name: type_name.clone(),
                        args: type_args.clone(),
                    }
                }
            }
            Value::Function(function) => Type::Function {
                params: function.params.clone(),
                return_type: Box::new(function.return_type.clone()),
                capabilities: function.capabilities.clone(),
            },
            Value::Timestamp(_) => Type::Timestamp,
            Value::Date(_) => Type::Date,
            Value::Time(_) => Type::Time,
            Value::Duration(_) => Type::Duration,
            Value::Bytes(_) => Type::Bytes,
            Value::Json(_) => Type::Json,
            Value::Array(values) => Type::Array(Box::new(
                values.first().map(Value::ty).unwrap_or(Type::Unknown),
            )),
            Value::Tuple(values) => Type::Tuple(values.iter().map(Value::ty).collect()),
            Value::Set(values) => Type::Set(Box::new(
                values.first().map(Value::ty).unwrap_or(Type::Unknown),
            )),
            Value::Map(entries) => Type::Map(
                Box::new(
                    entries
                        .first()
                        .map(|(key, _)| key.ty())
                        .unwrap_or(Type::Unknown),
                ),
                Box::new(
                    entries
                        .first()
                        .map(|(_, value)| value.ty())
                        .unwrap_or(Type::Unknown),
                ),
            ),
            Value::Object {
                type_name,
                type_args,
                ..
            } => {
                if type_args.is_empty() {
                    Type::Named(type_name.clone())
                } else {
                    Type::Applied {
                        name: type_name.clone(),
                        args: type_args.clone(),
                    }
                }
            }
            Value::ClassRef {
                type_name,
                type_args,
                ..
            } => {
                if type_args.is_empty() {
                    Type::Named(type_name.clone())
                } else {
                    Type::Applied {
                        name: type_name.clone(),
                        args: type_args.clone(),
                    }
                }
            }
            Value::Option(Some(value)) => Type::Option(Box::new(value.ty())),
            Value::Option(None) => Type::Option(Box::new(Type::Unknown)),
            Value::Result(Ok(value)) => Type::Result(Box::new(value.ty()), Box::new(Type::Unknown)),
            Value::Result(Err(value)) => {
                Type::Result(Box::new(Type::Unknown), Box::new(value.ty()))
            }
            Value::Rows { result_type, .. } => result_type.as_ref().clone(),
            Value::Unit => Type::Unit,
        }
    }
    pub fn output(&self) -> String {
        match self {
            Value::Int(v) => v.to_string(),
            Value::UInt(v) => v.to_string(),
            Value::Float(v) => v.to_string(),
            Value::Decimal(v) => format_decimal(*v),
            Value::Bool(v) => v.to_string(),
            Value::String(v) => v.clone(),
            Value::Char(v) => v.to_string(),
            Value::Enum {
                type_name,
                type_args,
                variant,
                payloads,
            } => {
                let type_name = if type_args.is_empty() {
                    type_name.clone()
                } else {
                    format!(
                        "{type_name}<{}>",
                        type_args
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                if payloads.is_empty() {
                    format!("{type_name}::{variant}")
                } else {
                    format!(
                        "{type_name}::{variant}({})",
                        payloads
                            .iter()
                            .map(Value::output)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
            Value::Timestamp(v) => v.to_string(),
            Value::Date(v) | Value::Time(v) => v.clone(),
            Value::Duration(v) => format!("{v}ms"),
            Value::Json(v) => v.to_string(),
            Value::Bytes(values) => format!(
                "Bytes[{}]",
                values
                    .iter()
                    .map(u8::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::ClassRef {
                reference,
                type_name,
                type_args,
            } => format!(
                "<{}#{}>",
                if type_args.is_empty() {
                    type_name.clone()
                } else {
                    format!(
                        "{type_name}<{}>",
                        type_args
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                },
                reference.object_id
            ),
            Value::Array(values) => format!(
                "[{}]",
                values
                    .iter()
                    .map(Value::output)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Tuple(values) => format!(
                "({})",
                values
                    .iter()
                    .map(Value::output)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Set(values) => format!(
                "Set{{{}}}",
                values
                    .iter()
                    .map(Value::output)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Map(entries) => format!(
                "Map{{{}}}",
                entries
                    .iter()
                    .map(|(key, value)| format!("{}={}", key.output(), value.output()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Function(function) => format!("<function {}>", function.name),
            Value::Object {
                type_name,
                type_args,
                fields,
            } => format!(
                "{}{{{}}}",
                if type_args.is_empty() {
                    type_name.clone()
                } else {
                    format!(
                        "{type_name}<{}>",
                        type_args
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                },
                fields
                    .iter()
                    .map(|(name, value)| format!("{name}={}", value.output()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Option(Some(v)) => format!("Some({})", v.output()),
            Value::Option(None) => "None".into(),
            Value::Result(Ok(v)) => format!("Ok({})", v.output()),
            Value::Result(Err(v)) => format!("Err({})", v.output()),
            Value::Rows { columns, rows, .. } => {
                let rendered_rows = rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .enumerate()
                            .map(|(index, value)| {
                                let column = columns.get(index).map(String::as_str).unwrap_or("?");
                                format!("{column}={value}")
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                format!("[{rendered_rows}]")
            }
            Value::Unit => "()".into(),
        }
    }
}

/// Normalize literal map entries using the language rule that the last value
/// for a duplicate key wins while the key retains its original position.
pub fn normalize_map_entries(entries: Vec<(Value, Value)>) -> Vec<(Value, Value)> {
    let mut normalized: Vec<(Value, Value)> = Vec::with_capacity(entries.len());
    for (key, value) in entries {
        if let Some((_, existing_value)) = normalized
            .iter_mut()
            .find(|(existing_key, _)| existing_key == &key)
        {
            *existing_value = value;
        } else {
            normalized.push((key, value));
        }
    }
    normalized
}

pub(crate) struct RuntimeClassObject {
    pub(crate) fields: HashMap<String, Value>,
}

#[derive(Clone)]
pub struct ClassObjectRef {
    pub(crate) object_id: u64,
    pub(crate) object: Arc<Mutex<RuntimeClassObject>>,
}

impl fmt::Debug for ClassObjectRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ClassObjectRef({})", self.object_id)
    }
}

impl PartialEq for ClassObjectRef {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.object, &other.object)
    }
}
