use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::Write;
use std::process::{Command, Stdio};
use zelyra_ast::*;

pub mod sql;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseConfig {
    pub name: String,
    pub engine: String,
    pub database: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Schema {
    pub database: Option<DatabaseConfig>,
    pub tables: Vec<Table>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
    pub foreign_keys: Vec<ForeignKey>,
    pub indexes: Vec<Index>,
    pub uniques: Vec<Index>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    pub sql_type: String,
    pub nullable: bool,
    pub primary_key: bool,
    pub auto: bool,
    pub unique: bool,
    pub default: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForeignKey {
    pub name: Option<String>,
    pub column: String,
    pub referenced_table: String,
    pub referenced_column: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Index {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub constraint_owned: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub fn build_schema(program: &Program) -> Result<Schema, Vec<SchemaError>> {
    let mut errors = Vec::new();
    if program.databases.len() > 1 {
        errors.push(SchemaError {
            message: "only one database definition is supported in Phase 3".into(),
            span: program.databases[1].span,
        });
    }
    let backend = program
        .databases
        .first()
        .map(|database| Backend::from_engine(&database.engine));
    if let Some(Err(error)) = &backend {
        let database = program.databases.first().expect("backend has a database");
        errors.push(SchemaError {
            message: error.message.clone(),
            span: database.span,
        });
    }
    let backend = backend.and_then(Result::ok).unwrap_or(Backend::MariaDb);
    let table_names = program
        .tables
        .iter()
        .map(|table| table.name.clone())
        .collect::<HashSet<_>>();
    let mut tables = Vec::new();
    for table in &program.tables {
        let mut column_names = HashSet::new();
        let mut columns = Vec::new();
        let mut foreign_keys = Vec::new();
        for definition in &table.columns {
            if !column_names.insert(definition.name.clone()) {
                errors.push(SchemaError {
                    message: format!(
                        "duplicate column {} in table {}",
                        definition.name, table.name
                    ),
                    span: definition.span,
                });
                continue;
            }
            if let Type::Named(name) = &definition.ty {
                let known_named_type = ["Id", "Email", "Url", "Uuid", "Money"]
                    .contains(&name.as_str())
                    || program
                        .types
                        .iter()
                        .any(|type_definition| type_definition.name == *name);
                if table_for_type(name, &table_names).is_none() && !known_named_type {
                    errors.push(SchemaError {
                        message: format!(
                            "unknown column type or relation target {} for {}.{}",
                            name, table.name, definition.name
                        ),
                        span: definition.span,
                    });
                    continue;
                }
            }
            let (storage_name, sql_type, relation) =
                column_mapping(definition, &table_names, &program.types, backend);
            if let Some(relation) = relation {
                if !table_names.contains(&relation) {
                    errors.push(SchemaError {
                        message: format!(
                            "relation target {} for column {} does not exist",
                            relation, definition.name
                        ),
                        span: definition.span,
                    });
                } else {
                    foreign_keys.push(ForeignKey {
                        name: Some(format!("fk_{}_{}", table.name, storage_name)),
                        column: storage_name.clone(),
                        referenced_table: relation,
                        referenced_column: "id".into(),
                    });
                }
            }
            let nullable = !definition.required && !definition.primary_key && !definition.auto;
            let default = definition.default.as_ref().map(default_sql).or_else(|| {
                (definition.auto && matches!(definition.ty, Type::Timestamp))
                    .then(|| "CURRENT_TIMESTAMP".into())
            });
            columns.push(Column {
                name: storage_name,
                sql_type,
                nullable,
                primary_key: definition.primary_key,
                auto: definition.auto,
                unique: definition.unique,
                default,
            });
        }
        let storage_names = columns
            .iter()
            .map(|column| column.name.clone())
            .collect::<HashSet<_>>();
        let indexes = table
            .indexes
            .iter()
            .filter_map(|index| {
                build_index(table, index, false, &storage_names, &columns, &mut errors)
            })
            .collect();
        let uniques = table
            .uniques
            .iter()
            .filter_map(|index| {
                build_index(table, index, true, &storage_names, &columns, &mut errors)
            })
            .collect();
        tables.push(Table {
            name: table.name.clone(),
            columns,
            foreign_keys,
            indexes,
            uniques,
        });
    }
    if errors.is_empty() {
        Ok(Schema {
            database: program.databases.first().map(|database| DatabaseConfig {
                name: database.name.clone(),
                engine: database.engine.clone(),
                database: database.database.clone(),
            }),
            tables,
        })
    } else {
        Err(errors)
    }
}

fn build_index(
    table: &TableDef,
    definition: &IndexDef,
    unique: bool,
    storage_names: &HashSet<String>,
    columns: &[Column],
    errors: &mut Vec<SchemaError>,
) -> Option<Index> {
    let mut storage_columns = Vec::new();
    for name in &definition.columns {
        let storage_name = if storage_names.contains(name) {
            name.clone()
        } else {
            format!("{}_id", name)
        };
        if !columns.iter().any(|column| column.name == storage_name) {
            errors.push(SchemaError {
                message: format!(
                    "index refers to unknown column {} in table {}",
                    name, table.name
                ),
                span: definition.span,
            });
            return None;
        }
        storage_columns.push(storage_name);
    }
    let prefix = if unique { "uq" } else { "idx" };
    Some(Index {
        name: format!("{}_{}_{}", prefix, table.name, storage_columns.join("_")),
        columns: storage_columns,
        unique,
        constraint_owned: unique,
    })
}

fn column_mapping(
    definition: &ColumnDef,
    table_names: &HashSet<String>,
    type_definitions: &[TypeDef],
    backend: Backend,
) -> (String, String, Option<String>) {
    if let Type::Named(name) = &definition.ty {
        if let Some(table) = table_for_type(name, table_names) {
            return (
                format!("{}_id", definition.name),
                match backend {
                    Backend::Sqlite => "INTEGER",
                    Backend::Postgres | Backend::MariaDb => "BIGINT",
                }
                .into(),
                Some(table),
            );
        }
    }
    let resolved = match resolve_type(&definition.ty, type_definitions) {
        Type::Option(inner) => *inner,
        resolved => resolved,
    };
    let sql_type = match resolved {
        Type::Int => {
            if definition.auto {
                match backend {
                    Backend::Postgres => "BIGSERIAL",
                    Backend::MariaDb => "BIGINT",
                    Backend::Sqlite => "INTEGER",
                }
            } else {
                match backend {
                    Backend::Sqlite => "INTEGER",
                    Backend::Postgres | Backend::MariaDb => "BIGINT",
                }
            }
        }
        Type::UInt => match backend {
            Backend::Postgres | Backend::Sqlite => "NUMERIC(20,0)",
            Backend::MariaDb => "DECIMAL(20,0)",
        },
        Type::Float => match backend {
            Backend::Postgres => "DOUBLE PRECISION",
            Backend::MariaDb => "DOUBLE",
            Backend::Sqlite => "REAL",
        },
        Type::Decimal => match backend {
            Backend::MariaDb => "DECIMAL",
            Backend::Postgres | Backend::Sqlite => "NUMERIC",
        },
        Type::Bool => match backend {
            Backend::Postgres | Backend::MariaDb => "BOOLEAN",
            Backend::Sqlite => "INTEGER",
        },
        Type::String => match definition.length {
            Some(length) => return (definition.name.clone(), format!("VARCHAR({length})"), None),
            None => "TEXT",
        },
        Type::Char => "CHAR(1)",
        Type::Bytes => match backend {
            Backend::Postgres => "BYTEA",
            Backend::MariaDb => "BLOB",
            Backend::Sqlite => "BLOB",
        },
        Type::Timestamp => match backend {
            Backend::Postgres => "TIMESTAMPTZ",
            Backend::MariaDb => "TIMESTAMP",
            Backend::Sqlite => "TEXT",
        },
        Type::Date => "DATE",
        Type::Time => "TIME",
        Type::Duration => match backend {
            Backend::Sqlite => "INTEGER",
            Backend::Postgres | Backend::MariaDb => "BIGINT",
        },
        Type::Named(ref name) if name == "Id" => {
            if definition.auto {
                match backend {
                    Backend::Postgres => "BIGSERIAL",
                    Backend::MariaDb => "BIGINT",
                    Backend::Sqlite => "INTEGER",
                }
            } else {
                match backend {
                    Backend::Sqlite => "INTEGER",
                    Backend::Postgres | Backend::MariaDb => "BIGINT",
                }
            }
        }
        Type::Named(ref name) if name == "Email" => "VARCHAR(255)",
        Type::Named(ref name) if name == "Url" => "VARCHAR(2048)",
        Type::Named(ref name) if name == "Uuid" => match backend {
            Backend::Postgres => "UUID",
            Backend::MariaDb => "CHAR(36)",
            Backend::Sqlite => "TEXT",
        },
        Type::Named(ref name) if name == "Money" => match backend {
            Backend::MariaDb => "DECIMAL(19,4)",
            Backend::Postgres | Backend::Sqlite => "NUMERIC(19,4)",
        },
        Type::Named(_)
        | Type::Option(_)
        | Type::Result(_, _)
        | Type::Array(_)
        | Type::Map(_, _)
        | Type::HttpResult(_)
        | Type::Unit
        | Type::Unknown => "TEXT",
    };
    (definition.name.clone(), sql_type.into(), None)
}

fn resolve_type(ty: &Type, definitions: &[TypeDef]) -> Type {
    if let Type::Named(name) = ty {
        if let Some(definition) = definitions
            .iter()
            .find(|definition| definition.name == *name)
        {
            return resolve_type(&definition.target, definitions);
        }
    }
    ty.clone()
}

fn table_for_type(name: &str, table_names: &HashSet<String>) -> Option<String> {
    let snake = name.to_ascii_lowercase();
    if table_names.contains(&snake) {
        return Some(snake);
    }
    let plural = if snake.ends_with('y') {
        format!("{}ies", &snake[..snake.len() - 1])
    } else {
        format!("{snake}s")
    };
    table_names.contains(&plural).then_some(plural)
}

fn default_sql(value: &DefaultValue) -> String {
    match value {
        DefaultValue::Int(value) => value.to_string(),
        DefaultValue::Bool(value) => value.to_string().to_uppercase(),
        DefaultValue::String(value) => quote_string(value),
        DefaultValue::Ident(value) if value.eq_ignore_ascii_case("now") => {
            "CURRENT_TIMESTAMP".into()
        }
        DefaultValue::Ident(value) => quote_string(value),
    }
}

fn quote_identifier(value: &str, backend: Backend) -> String {
    match backend {
        Backend::MariaDb => format!("`{}`", value.replace('`', "``")),
        Backend::Postgres | Backend::Sqlite => {
            format!("\"{}\"", value.replace('"', "\"\""))
        }
    }
}

fn alter_column_default_sql(table: &Table, column: &Column, backend: Backend) -> String {
    let table_name = quote_identifier(&table.name, backend);
    let column_name = quote_identifier(&column.name, backend);
    match &column.default {
        Some(default) => {
            format!("ALTER TABLE {table_name} ALTER COLUMN {column_name} SET DEFAULT {default};")
        }
        None => format!("ALTER TABLE {table_name} ALTER COLUMN {column_name} DROP DEFAULT;"),
    }
}

fn alter_column_nullability_sql(table: &Table, column: &Column, backend: Backend) -> String {
    let table_name = quote_identifier(&table.name, backend);
    let column_name = quote_identifier(&column.name, backend);
    match backend {
        Backend::Postgres => format!(
            "ALTER TABLE {table_name} ALTER COLUMN {column_name} {};",
            if column.nullable {
                "DROP NOT NULL"
            } else {
                "SET NOT NULL"
            }
        ),
        Backend::MariaDb => {
            let mut definition = format!("{column_name} {}", column.sql_type);
            definition.push_str(if column.nullable { " NULL" } else { " NOT NULL" });
            if column.auto {
                definition.push_str(" AUTO_INCREMENT");
            }
            if let Some(default) = &column.default {
                definition.push_str(" DEFAULT ");
                definition.push_str(default);
            }
            format!("ALTER TABLE {table_name} MODIFY COLUMN {definition};")
        }
        Backend::Sqlite => format!(
            "-- Changing nullability of {}.{} requires a SQLite table rebuild, which is not supported by the current schema planner.",
            table.name, column.name
        ),
    }
}

fn nullability_count_sql(table: &str, column: &str, backend: Backend) -> String {
    format!(
        "SELECT COUNT(*) FROM {} WHERE {} IS NULL",
        quote_identifier(table, backend),
        quote_identifier(column, backend)
    )
}

fn table_has_rows_sql(table: &str, backend: Backend) -> String {
    format!(
        "SELECT CASE WHEN EXISTS (SELECT 1 FROM {} LIMIT 1) THEN 1 ELSE 0 END",
        quote_identifier(table, backend)
    )
}

fn quote_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

impl Schema {
    pub fn create_sql(&self) -> String {
        let backend = self.backend();
        self.tables
            .iter()
            .map(|table| table_create_with_indexes_sql(table, backend))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    pub fn summary(&self) -> String {
        let columns = self
            .tables
            .iter()
            .map(|table| table.columns.len())
            .sum::<usize>();
        let foreign_keys = self
            .tables
            .iter()
            .map(|table| table.foreign_keys.len())
            .sum::<usize>();
        let indexes = self
            .tables
            .iter()
            .map(|table| table.indexes.len() + table.uniques.len())
            .sum::<usize>();
        format!(
            "{} tables\n{} foreign keys\n{} indexes\n{} columns",
            self.tables.len(),
            foreign_keys,
            indexes,
            columns
        )
    }
}

fn table_create_sql(table: &Table, backend: Backend) -> String {
    let mut definitions = table
        .columns
        .iter()
        .map(|column| column_sql(column, backend))
        .collect::<Vec<_>>();
    definitions.extend(table.uniques.iter().map(|index| {
        let columns = index
            .columns
            .iter()
            .map(|column| quote_identifier(column, backend))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "CONSTRAINT {} UNIQUE ({columns})",
            quote_identifier(&index.name, backend)
        )
    }));
    definitions.extend(table.foreign_keys.iter().map(|foreign_key| {
        let name = foreign_key
            .name
            .clone()
            .unwrap_or_else(|| format!("fk_{}_{}", table.name, foreign_key.column));
        format!(
            "CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} ({})",
            quote_identifier(&name, backend),
            quote_identifier(&foreign_key.column, backend),
            quote_identifier(&foreign_key.referenced_table, backend),
            quote_identifier(&foreign_key.referenced_column, backend)
        )
    }));
    format!(
        "CREATE TABLE IF NOT EXISTS {} (\n    {}\n);",
        quote_identifier(&table.name, backend),
        definitions.join(",\n    ")
    )
}

fn table_create_with_indexes_sql(table: &Table, backend: Backend) -> String {
    let mut statements = vec![table_create_sql(table, backend)];
    statements.extend(
        table
            .indexes
            .iter()
            .map(|index| index_sql(table, index, backend)),
    );
    statements.join("\n\n")
}

fn index_sql(table: &Table, index: &Index, backend: Backend) -> String {
    let columns = index
        .columns
        .iter()
        .map(|column| quote_identifier(column, backend))
        .collect::<Vec<_>>()
        .join(", ");
    let unique = if index.unique { " UNIQUE" } else { "" };
    format!(
        "CREATE{unique} INDEX IF NOT EXISTS {} ON {} ({columns});",
        quote_identifier(&index.name, backend),
        quote_identifier(&table.name, backend)
    )
}

fn column_sql(column: &Column, backend: Backend) -> String {
    let mut sql = format!(
        "{} {}",
        quote_identifier(&column.name, backend),
        column.sql_type
    );
    if backend == Backend::Sqlite && column.primary_key && column.auto {
        sql.push_str(" PRIMARY KEY AUTOINCREMENT");
        return sql;
    }
    if column.primary_key {
        sql.push_str(" PRIMARY KEY");
    }
    if !column.nullable {
        sql.push_str(" NOT NULL");
    }
    if column.unique {
        sql.push_str(" UNIQUE");
    }
    if backend == Backend::MariaDb && column.auto {
        sql.push_str(" AUTO_INCREMENT");
    }
    if let Some(default) = &column.default {
        sql.push_str(" DEFAULT ");
        sql.push_str(default);
    }
    sql
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Risk {
    Safe,
    RequiresApproval,
    Destructive,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaChange {
    pub description: String,
    pub sql: String,
    pub risk: Risk,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NullabilityPreflight {
    pub table: String,
    pub column: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiredColumnPreflight {
    pub table: String,
    pub column: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SchemaPlan {
    pub changes: Vec<SchemaChange>,
    pub nullability_preflights: Vec<NullabilityPreflight>,
    pub required_column_preflights: Vec<RequiredColumnPreflight>,
}

impl SchemaPlan {
    pub fn requires_approval(&self) -> bool {
        self.changes
            .iter()
            .any(|change| matches!(change.risk, Risk::RequiresApproval | Risk::Destructive))
    }

    pub fn has_unsupported(&self) -> bool {
        self.changes
            .iter()
            .any(|change| change.risk == Risk::Unsupported)
    }

    pub fn is_destructive(&self) -> bool {
        self.changes
            .iter()
            .any(|change| change.risk == Risk::Destructive)
    }
    pub fn sql(&self) -> String {
        self.changes
            .iter()
            .map(|change| change.sql.clone())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

pub fn diff(desired: &Schema, current: &Schema) -> SchemaPlan {
    let backend = desired.backend();
    let mut plan = SchemaPlan::default();
    let current_tables = current
        .tables
        .iter()
        .map(|table| (table.name.as_str(), table))
        .collect::<HashMap<_, _>>();
    for desired_table in &desired.tables {
        let Some(current_table) = current_tables.get(desired_table.name.as_str()) else {
            plan.changes.push(SchemaChange {
                description: format!("create table {}", desired_table.name),
                sql: table_create_with_indexes_sql(desired_table, backend),
                risk: Risk::Safe,
            });
            continue;
        };
        let current_columns = current_table
            .columns
            .iter()
            .map(|column| (column.name.as_str(), column))
            .collect::<HashMap<_, _>>();
        let desired_columns = desired_table
            .columns
            .iter()
            .map(|column| (column.name.as_str(), column))
            .collect::<HashMap<_, _>>();
        for desired_column in &desired_table.columns {
            if let Some(current_column) = current_columns.get(desired_column.name.as_str()) {
                let type_changed = !sql_types_equivalent(
                    &current_column.sql_type,
                    &desired_column.sql_type,
                    backend,
                );
                if type_changed {
                    plan.changes.push(SchemaChange {
                        description: format!(
                            "change type of {}.{}",
                            desired_table.name, desired_column.name
                        ),
                        sql: alter_column_sql(desired_table, desired_column, backend),
                        risk: type_change_risk(
                            &current_column.sql_type,
                            &desired_column.sql_type,
                            backend,
                        ),
                    });
                }
                if current_column.nullable != desired_column.nullable {
                    if current_column.nullable
                        && !desired_column.nullable
                        && backend != Backend::Sqlite
                    {
                        plan.nullability_preflights.push(NullabilityPreflight {
                            table: desired_table.name.clone(),
                            column: desired_column.name.clone(),
                        });
                    }
                    match backend {
                        Backend::MariaDb | Backend::Postgres => {
                            if backend == Backend::MariaDb && type_changed {
                                if let Some(change) = plan.changes.iter_mut().find(|change| {
                                    change.description
                                        == format!(
                                            "change type of {}.{}",
                                            desired_table.name, desired_column.name
                                        )
                                }) {
                                    change.description = format!(
                                        "change type and nullability of {}.{}",
                                        desired_table.name, desired_column.name
                                    );
                                    if change.risk == Risk::Safe {
                                        change.risk = Risk::RequiresApproval;
                                    }
                                }
                            } else {
                                plan.changes.push(SchemaChange {
                                    description: format!(
                                        "change nullability of {}.{}",
                                        desired_table.name, desired_column.name
                                    ),
                                    sql: alter_column_nullability_sql(
                                        desired_table,
                                        desired_column,
                                        backend,
                                    ),
                                    risk: Risk::RequiresApproval,
                                });
                            }
                        }
                        Backend::Sqlite => plan.changes.push(SchemaChange {
                            description: format!(
                                "change nullability of {}.{}",
                                desired_table.name, desired_column.name
                            ),
                            sql: format!(
                                "-- Changing nullability of {}.{} requires a SQLite table rebuild, which is not supported by the current schema planner.",
                                desired_table.name, desired_column.name
                            ),
                            risk: Risk::Unsupported,
                        }),
                    }
                }
            } else {
                if !desired_column.nullable && desired_column.default.is_none() {
                    plan.required_column_preflights
                        .push(RequiredColumnPreflight {
                            table: desired_table.name.clone(),
                            column: desired_column.name.clone(),
                        });
                }
                plan.changes.push(SchemaChange {
                    description: if !desired_column.nullable && desired_column.default.is_none() {
                        format!(
                            "add required column {}.{} without a default; review existing rows",
                            desired_table.name, desired_column.name
                        )
                    } else {
                        format!("add column {}.{}", desired_table.name, desired_column.name)
                    },
                    sql: format!(
                        "ALTER TABLE {} ADD COLUMN {};",
                        quote_identifier(&desired_table.name, backend),
                        column_sql(desired_column, backend)
                    ),
                    risk: if !desired_column.nullable && desired_column.default.is_none() {
                        Risk::RequiresApproval
                    } else {
                        Risk::Safe
                    },
                });
            }
            if let Some(current_column) = current_columns.get(desired_column.name.as_str()) {
                if current_column.primary_key != desired_column.primary_key {
                    plan.changes.push(SchemaChange {
                        description: format!(
                            "change primary-key status of {}.{}",
                            desired_table.name, desired_column.name
                        ),
                        sql: format!(
                            "-- Changing primary-key status of {}.{} is not supported by the current schema planner.",
                            desired_table.name, desired_column.name
                        ),
                        risk: Risk::Unsupported,
                    });
                }
                if current_column.auto != desired_column.auto {
                    plan.changes.push(SchemaChange {
                        description: format!(
                            "change auto-increment status of {}.{}",
                            desired_table.name, desired_column.name
                        ),
                        sql: format!(
                            "-- Changing auto-increment status of {}.{} is not supported by the current schema planner.",
                            desired_table.name, desired_column.name
                        ),
                        risk: Risk::Unsupported,
                    });
                }
                let defaults_match = if backend == Backend::Postgres {
                    postgres_defaults_equivalent(
                        current_column.default.as_deref(),
                        desired_column.default.as_deref(),
                    )
                } else {
                    defaults_equivalent(
                        current_column.default.as_deref(),
                        desired_column.default.as_deref(),
                    )
                };
                if !defaults_match {
                    let (sql, risk) = match backend {
                        Backend::MariaDb | Backend::Postgres => (
                            alter_column_default_sql(desired_table, desired_column, backend),
                            Risk::RequiresApproval,
                        ),
                        Backend::Sqlite => (
                            format!(
                                "-- Changing the default of {}.{} requires a SQLite table rebuild, which is not supported by the current schema planner.",
                                desired_table.name, desired_column.name
                            ),
                            Risk::Unsupported,
                        ),
                    };
                    plan.changes.push(SchemaChange {
                        description: format!(
                            "change default of {}.{}",
                            desired_table.name, desired_column.name
                        ),
                        sql,
                        risk,
                    });
                }
            }
        }
        for current_column in &current_table.columns {
            if !desired_columns.contains_key(current_column.name.as_str()) {
                plan.changes.push(SchemaChange {
                    description: format!(
                        "drop column {}.{}",
                        desired_table.name, current_column.name
                    ),
                    sql: format!(
                        "ALTER TABLE {} DROP COLUMN {};",
                        quote_identifier(&desired_table.name, backend),
                        quote_identifier(&current_column.name, backend)
                    ),
                    risk: Risk::Destructive,
                });
            }
        }
        let current_indexes = table_indexes(current_table);
        let desired_indexes = table_indexes(desired_table);
        for index in &desired_indexes {
            if !current_indexes
                .iter()
                .any(|current| same_index(current, index))
            {
                plan.changes.push(SchemaChange {
                    description: if index.unique {
                        format!(
                            "add unique index {} on {}; existing duplicates may block it",
                            index.name,
                            index.columns.join(", ")
                        )
                    } else {
                        format!("add index {}", index.name)
                    },
                    sql: index_sql(desired_table, index, backend),
                    risk: if index.unique {
                        Risk::RequiresApproval
                    } else {
                        Risk::Safe
                    },
                });
            }
        }
        for index in &current_indexes {
            if !desired_indexes
                .iter()
                .any(|desired| same_index(index, desired))
            {
                if !is_zelyra_managed_index(desired_table, index, backend) {
                    continue;
                }
                let (sql, risk) = drop_index_sql(desired_table, index, backend);
                plan.changes.push(SchemaChange {
                    description: format!(
                        "drop {}index {}",
                        if index.unique { "unique " } else { "" },
                        index.name
                    ),
                    sql,
                    risk,
                });
            }
        }

        for foreign_key in &desired_table.foreign_keys {
            if !current_table
                .foreign_keys
                .iter()
                .any(|current| same_foreign_key(current, foreign_key))
            {
                let sql = add_foreign_key_sql(desired_table, foreign_key, backend);
                let risk = if backend == Backend::Sqlite {
                    Risk::Unsupported
                } else {
                    Risk::RequiresApproval
                };
                plan.changes.push(SchemaChange {
                    description: format!(
                        "add foreign key {}.{}; existing rows must satisfy it",
                        desired_table.name, foreign_key.column
                    ),
                    sql,
                    risk,
                });
            }
        }
        for foreign_key in &current_table.foreign_keys {
            if !desired_table
                .foreign_keys
                .iter()
                .any(|desired| same_foreign_key(foreign_key, desired))
            {
                if !is_zelyra_managed_foreign_key(desired_table, foreign_key) {
                    plan.changes.push(SchemaChange {
                        description: format!(
                            "remove untracked foreign key {}.{}",
                            desired_table.name, foreign_key.column
                        ),
                        sql: format!(
                            "-- Cannot safely remove the untracked foreign key on {}.{}.",
                            desired_table.name, foreign_key.column
                        ),
                        risk: Risk::Unsupported,
                    });
                    continue;
                }
                let (sql, risk) = drop_foreign_key_sql(desired_table, foreign_key, backend);
                plan.changes.push(SchemaChange {
                    description: format!(
                        "drop foreign key {}.{}",
                        desired_table.name, foreign_key.column
                    ),
                    sql,
                    risk,
                });
            }
        }
    }
    for current_table in &current.tables {
        if !desired
            .tables
            .iter()
            .any(|table| table.name == current_table.name)
        {
            plan.changes.push(SchemaChange {
                description: format!("drop table {}", current_table.name),
                sql: format!(
                    "DROP TABLE {};",
                    quote_identifier(&current_table.name, backend)
                ),
                risk: Risk::Destructive,
            });
        }
    }
    // Stable ordering preserves source/database order within each operation
    // class (notably table creation order for foreign-key dependencies).
    plan.changes
        .sort_by_key(|change| change_order(&change.description));
    plan.nullability_preflights.sort_by(|left, right| {
        left.table
            .cmp(&right.table)
            .then_with(|| left.column.cmp(&right.column))
    });
    plan.required_column_preflights.sort_by(|left, right| {
        left.table
            .cmp(&right.table)
            .then_with(|| left.column.cmp(&right.column))
    });
    plan
}

fn type_change_risk(current: &str, desired: &str, backend: Backend) -> Risk {
    if backend == Backend::Sqlite {
        return Risk::Unsupported;
    }
    let parse_varchar = |value: &str| {
        value
            .strip_prefix("VARCHAR(")
            .and_then(|value| value.strip_suffix(')'))
            .and_then(|value| value.parse::<u32>().ok())
    };
    match (parse_varchar(current), parse_varchar(desired)) {
        (Some(current), Some(desired)) if desired >= current => Risk::Safe,
        _ => Risk::Destructive,
    }
}

fn sql_types_equivalent(current: &str, desired: &str, backend: Backend) -> bool {
    if backend != Backend::Postgres {
        return current == desired;
    }
    postgres_storage_type(current) == postgres_storage_type(desired)
}

fn postgres_storage_type(sql_type: &str) -> &str {
    match sql_type {
        "BIGSERIAL" => "BIGINT",
        "SERIAL" => "INTEGER",
        "SMALLSERIAL" => "SMALLINT",
        other => other,
    }
}

fn defaults_equivalent(current: Option<&str>, desired: Option<&str>) -> bool {
    let Some(current) = current else {
        return desired.is_none();
    };
    let Some(desired) = desired else {
        return false;
    };
    let current = normalize_default(current);
    let desired = normalize_default(desired);
    current == desired
        || matches!(
            (current.as_str(), desired.as_str()),
            ("1", "TRUE") | ("TRUE", "1")
        )
        || matches!(
            (current.as_str(), desired.as_str()),
            ("0", "FALSE") | ("FALSE", "0")
        )
}

fn postgres_defaults_equivalent(current: Option<&str>, desired: Option<&str>) -> bool {
    let normalize = |value: &str| {
        let Some((literal, cast_type)) = value.rsplit_once("::") else {
            return value.trim().to_string();
        };
        let cast_type = cast_type.trim().to_ascii_lowercase();
        if is_postgres_literal_cast(&cast_type) && is_simple_sql_literal(literal.trim()) {
            literal.trim().to_string()
        } else {
            value.trim().to_string()
        }
    };
    let current = current.map(normalize);
    let desired = desired.map(normalize);
    defaults_equivalent(current.as_deref(), desired.as_deref())
}

fn is_postgres_literal_cast(cast_type: &str) -> bool {
    matches!(
        cast_type,
        "text"
            | "character varying"
            | "character"
            | "varchar"
            | "char"
            | "boolean"
            | "bool"
            | "smallint"
            | "integer"
            | "bigint"
            | "int2"
            | "int4"
            | "int8"
            | "numeric"
            | "decimal"
            | "real"
            | "double precision"
            | "float4"
            | "float8"
            | "date"
            | "time without time zone"
            | "time with time zone"
            | "timestamp without time zone"
            | "timestamp with time zone"
            | "uuid"
            | "bytea"
    ) || [
        "character varying",
        "character",
        "varchar",
        "char",
        "numeric",
        "decimal",
    ]
    .iter()
    .any(|prefix| {
        cast_type
            .strip_prefix(prefix)
            .is_some_and(|suffix| suffix.starts_with('(') && suffix.ends_with(')'))
    })
}

fn is_simple_sql_literal(value: &str) -> bool {
    if matches!(
        value.to_ascii_lowercase().as_str(),
        "true" | "false" | "null"
    ) {
        return true;
    }
    let bytes = value.as_bytes();
    if bytes.len() >= 2 && bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\'' {
        let mut index = 1;
        while index < bytes.len() - 1 {
            if bytes[index] == b'\'' {
                if bytes.get(index + 1) != Some(&b'\'') || index + 1 >= bytes.len() - 1 {
                    return false;
                }
                index += 2;
            } else {
                index += 1;
            }
        }
        return true;
    }

    is_numeric_sql_literal(bytes)
}

fn is_numeric_sql_literal(bytes: &[u8]) -> bool {
    let mut index = 0;
    if matches!(bytes.first(), Some(b'+') | Some(b'-')) {
        index += 1;
    }
    let mut digits = 0;
    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
        digits += 1;
        index += 1;
    }
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            digits += 1;
            index += 1;
        }
    }
    if digits == 0 {
        return false;
    }
    if matches!(bytes.get(index), Some(b'e') | Some(b'E')) {
        index += 1;
        if matches!(bytes.get(index), Some(b'+') | Some(b'-')) {
            index += 1;
        }
        let exponent_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == exponent_start {
            return false;
        }
    }
    index == bytes.len()
}

fn normalize_default(value: &str) -> String {
    let trimmed = value.trim();
    let upper = trimmed.to_ascii_uppercase();
    match upper.as_str() {
        "CURRENT_TIMESTAMP()" => "CURRENT_TIMESTAMP".into(),
        "TRUE" | "FALSE" | "CURRENT_TIMESTAMP" => upper,
        _ => trimmed.into(),
    }
}

fn table_indexes(table: &Table) -> Vec<Index> {
    let mut indexes = table
        .indexes
        .iter()
        .chain(table.uniques.iter())
        .cloned()
        .collect::<Vec<_>>();
    for column in table.columns.iter().filter(|column| column.unique) {
        if !indexes
            .iter()
            .any(|index| index.unique && index.columns.as_slice() == [column.name.as_str()])
        {
            indexes.push(Index {
                name: format!("ux_{}_{}", table.name, column.name),
                columns: vec![column.name.clone()],
                unique: true,
                constraint_owned: true,
            });
        }
    }
    indexes.sort_by(|left, right| {
        left.unique
            .cmp(&right.unique)
            .then_with(|| left.columns.cmp(&right.columns))
            .then_with(|| left.name.cmp(&right.name))
    });
    indexes.dedup_by(|left, right| same_index(left, right));
    indexes
}

fn same_index(left: &Index, right: &Index) -> bool {
    left.unique == right.unique && left.columns == right.columns
}

fn is_zelyra_managed_index(table: &Table, index: &Index, backend: Backend) -> bool {
    if backend == Backend::Sqlite && index.name.starts_with("sqlite_autoindex") {
        return index.unique;
    }
    let columns = index.columns.join("_");
    let mut generated_names = vec![
        format!("idx_{}_{}", table.name, columns),
        format!("uq_{}_{}", table.name, columns),
    ];
    if index.unique && index.columns.len() == 1 {
        let column = &index.columns[0];
        generated_names.push(format!("ux_{}_{}", table.name, column));
        // MariaDB and PostgreSQL generate these names for inline UNIQUE
        // declarations created by Zelyra's initial CREATE TABLE statement.
        generated_names.push(column.clone());
        generated_names.push(format!("{}_{}_key", table.name, column));
    }
    generated_names.iter().any(|name| name == &index.name)
}

fn is_zelyra_managed_foreign_key(table: &Table, foreign_key: &ForeignKey) -> bool {
    let expected = format!("fk_{}_{}", table.name, foreign_key.column);
    foreign_key.name.as_deref() == Some(expected.as_str())
}

fn drop_index_sql(table: &Table, index: &Index, backend: Backend) -> (String, Risk) {
    if backend == Backend::Sqlite && index.name.starts_with("sqlite_autoindex") {
        return (
            format!(
                "-- SQLite does not support dropping the automatically-created unique index {}.",
                index.name
            ),
            Risk::Unsupported,
        );
    }
    let index_name = quote_identifier(&index.name, backend);
    if backend == Backend::Postgres && index.constraint_owned {
        return (
            format!(
                "ALTER TABLE {} DROP CONSTRAINT {};",
                quote_identifier(&table.name, backend),
                index_name
            ),
            Risk::RequiresApproval,
        );
    }
    let sql = match backend {
        Backend::MariaDb => format!(
            "ALTER TABLE {} DROP INDEX {};",
            quote_identifier(&table.name, backend),
            index_name
        ),
        Backend::Postgres | Backend::Sqlite => {
            format!("DROP INDEX IF EXISTS {index_name};")
        }
    };
    (sql, Risk::RequiresApproval)
}

fn same_foreign_key(left: &ForeignKey, right: &ForeignKey) -> bool {
    left.column == right.column
        && left.referenced_table == right.referenced_table
        && left.referenced_column == right.referenced_column
}

fn add_foreign_key_sql(table: &Table, foreign_key: &ForeignKey, backend: Backend) -> String {
    let name = foreign_key
        .name
        .clone()
        .unwrap_or_else(|| format!("fk_{}_{}", table.name, foreign_key.column));
    format!(
        "ALTER TABLE {} ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} ({});",
        quote_identifier(&table.name, backend),
        quote_identifier(&name, backend),
        quote_identifier(&foreign_key.column, backend),
        quote_identifier(&foreign_key.referenced_table, backend),
        quote_identifier(&foreign_key.referenced_column, backend)
    )
}

fn drop_foreign_key_sql(
    table: &Table,
    foreign_key: &ForeignKey,
    backend: Backend,
) -> (String, Risk) {
    let Some(name) = &foreign_key.name else {
        return (
            format!(
                "-- Cannot safely drop the foreign key on {}.{} because its database name is unknown.",
                table.name, foreign_key.column
            ),
            Risk::Unsupported,
        );
    };
    let sql = match backend {
        Backend::MariaDb => format!(
            "ALTER TABLE {} DROP FOREIGN KEY {};",
            quote_identifier(&table.name, backend),
            quote_identifier(name, backend)
        ),
        Backend::Postgres => format!(
            "ALTER TABLE {} DROP CONSTRAINT {};",
            quote_identifier(&table.name, backend),
            quote_identifier(name, backend)
        ),
        Backend::Sqlite => {
            return (
                format!(
                    "-- SQLite cannot drop foreign key {} without rebuilding table {}.",
                    name, table.name
                ),
                Risk::Unsupported,
            );
        }
    };
    (sql, Risk::RequiresApproval)
}

fn change_order(description: &str) -> u8 {
    if description.starts_with("create table ") {
        0
    } else if description.starts_with("drop foreign key ") {
        1
    } else if description.starts_with("drop ") && description.contains("index ") {
        2
    } else if description.starts_with("add column ")
        || description.starts_with("add required column ")
    {
        3
    } else if description.starts_with("change ") {
        4
    } else if description.starts_with("add ") && description.contains("index ") {
        5
    } else if description.starts_with("add foreign key ") {
        6
    } else if description.starts_with("drop column ") {
        7
    } else {
        8
    }
}

fn alter_column_sql(table: &Table, column: &Column, backend: Backend) -> String {
    match backend {
        Backend::Postgres => format!(
            "ALTER TABLE {} ALTER COLUMN {} TYPE {};",
            quote_identifier(&table.name, backend),
            quote_identifier(&column.name, backend),
            column.sql_type
        ),
        Backend::MariaDb => format!(
            "ALTER TABLE {} MODIFY COLUMN {};",
            quote_identifier(&table.name, backend),
            column_sql(column, backend)
        ),
        Backend::Sqlite => format!(
            "-- SQLite requires a table rebuild to change {}.{} from the current type to {};",
            table.name, column.name, column.sql_type
        ),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseError {
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Postgres,
    MariaDb,
    Sqlite,
}

impl Backend {
    pub fn from_engine(engine: &str) -> Result<Self, DatabaseError> {
        match engine.to_ascii_lowercase().as_str() {
            "postgres" | "postgresql" => Ok(Self::Postgres),
            "mariadb" | "mysql" => Ok(Self::MariaDb),
            "sqlite" | "sqlite3" => Ok(Self::Sqlite),
            other => Err(DatabaseError {
                message: format!(
                    "unsupported database engine {other}; supported engines are postgres, mariadb, and sqlite"
                ),
            }),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Postgres => "postgres",
            Self::MariaDb => "mariadb",
            Self::Sqlite => "sqlite",
        }
    }
}

impl Schema {
    pub fn backend(&self) -> Backend {
        self.database
            .as_ref()
            .and_then(|database| Backend::from_engine(&database.engine).ok())
            .unwrap_or(Backend::MariaDb)
    }
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub fn inspect_postgres(database_url: &str) -> Result<Schema, DatabaseError> {
    let query = r#"
        WITH inspected_columns AS (
            SELECT
                c.table_schema,
                c.table_name,
                c.column_name,
                c.data_type,
                c.udt_name,
                c.is_nullable,
                COALESCE(c.character_maximum_length::text, '') AS character_maximum_length,
                c.ordinal_position,
                EXISTS (
                    SELECT 1
                    FROM pg_constraint constraint_row
                    JOIN pg_class table_row
                        ON table_row.oid = constraint_row.conrelid
                    JOIN pg_namespace schema_row
                        ON schema_row.oid = table_row.relnamespace
                    JOIN pg_attribute column_row
                        ON column_row.attrelid = table_row.oid
                        AND column_row.attname = c.column_name
                        AND NOT column_row.attisdropped
                    WHERE constraint_row.contype = 'p'
                        AND schema_row.nspname = c.table_schema
                        AND table_row.relname = c.table_name
                        AND column_row.attnum = ANY (constraint_row.conkey)
                ) AS is_primary_key,
                COALESCE((
                    c.is_identity = 'YES'
                    OR left(lower(c.column_default), 8) = 'nextval('
                ), false) AS is_auto_generated,
                c.column_default
            FROM information_schema.columns c
            WHERE c.table_schema = 'public'
        )
        SELECT
            table_name,
            column_name,
            data_type,
            udt_name,
            is_nullable,
            character_maximum_length,
            is_primary_key,
            is_auto_generated,
            CASE
                WHEN is_auto_generated OR column_default IS NULL THEN 'N'
                ELSE 'V' || encode(convert_to(column_default, 'UTF8'), 'hex')
            END
        FROM inspected_columns
        ORDER BY table_name, ordinal_position
    "#;
    let output = run_psql(database_url, query)?;
    let mut schema = parse_inspection_output(&output)?;
    let index_output = run_psql(
        database_url,
        "SELECT t.relname, i.relname, pg_get_indexdef(i.oid), EXISTS (SELECT 1 FROM pg_constraint c WHERE c.conindid = i.oid AND c.contype = 'u') FROM pg_index x JOIN pg_class t ON t.oid = x.indrelid JOIN pg_class i ON i.oid = x.indexrelid JOIN pg_namespace n ON n.oid = t.relnamespace WHERE n.nspname = 'public' AND NOT EXISTS (SELECT 1 FROM pg_constraint c WHERE c.conindid = i.oid AND c.contype = 'p') ORDER BY t.relname, i.relname",
    )?;
    parse_index_output(&mut schema, &index_output)?;
    let foreign_keys = run_psql(
        database_url,
        "SELECT child.relname, child_column.attname, parent.relname, parent_column.attname, constraint_row.conname FROM pg_constraint constraint_row JOIN pg_class child ON child.oid = constraint_row.conrelid JOIN pg_namespace child_schema ON child_schema.oid = child.relnamespace JOIN pg_class parent ON parent.oid = constraint_row.confrelid JOIN LATERAL unnest(constraint_row.conkey) WITH ORDINALITY AS child_key(attnum, position) ON TRUE JOIN LATERAL unnest(constraint_row.confkey) WITH ORDINALITY AS parent_key(attnum, position) ON parent_key.position = child_key.position JOIN pg_attribute child_column ON child_column.attrelid = child.oid AND child_column.attnum = child_key.attnum JOIN pg_attribute parent_column ON parent_column.attrelid = parent.oid AND parent_column.attnum = parent_key.attnum WHERE constraint_row.contype = 'f' AND child_schema.nspname = 'public' ORDER BY child.relname, constraint_row.conname, child_key.position",
    )?;
    parse_foreign_key_output(&mut schema, &foreign_keys)?;
    Ok(schema)
}

fn run_psql(database_url: &str, query: &str) -> Result<String, DatabaseError> {
    let output = Command::new("psql")
        .args(["-X", "-At", "-F", "\t", "-d", database_url, "-c", query])
        .output()
        .map_err(|error| DatabaseError {
            message: format!("could not start psql: {error}"),
        })?;
    if !output.status.success() {
        return Err(DatabaseError {
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn parse_inspection_output(output: &str) -> Result<Schema, DatabaseError> {
    let mut tables: Vec<Table> = Vec::new();
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 9 {
            return Err(DatabaseError {
                message: format!(
                    "unexpected PostgreSQL column metadata with {} fields",
                    fields.len()
                ),
            });
        }
        let table_name = fields[0].to_string();
        let sql_type = inspected_sql_type(fields[2], fields[3], fields[5]);
        let primary_key = parse_postgres_bool(fields[6])?;
        let auto = parse_postgres_bool(fields[7])?;
        let column = Column {
            name: fields[1].to_string(),
            sql_type,
            nullable: fields[4] == "YES",
            primary_key,
            auto,
            unique: false,
            default: postgres_default_value(fields[8], auto)?,
        };
        if let Some(table) = tables.iter_mut().find(|table| table.name == table_name) {
            table.columns.push(column);
        } else {
            tables.push(Table {
                name: table_name,
                columns: vec![column],
                foreign_keys: Vec::new(),
                indexes: Vec::new(),
                uniques: Vec::new(),
            });
        }
    }
    Ok(Schema {
        database: None,
        tables,
    })
}

fn parse_postgres_bool(value: &str) -> Result<bool, DatabaseError> {
    match value {
        "t" => Ok(true),
        "f" => Ok(false),
        _ => Err(DatabaseError {
            message: "unexpected PostgreSQL boolean metadata".into(),
        }),
    }
}

fn postgres_default_value(
    encoded: &str,
    auto_generated: bool,
) -> Result<Option<String>, DatabaseError> {
    if auto_generated || encoded == "N" {
        return Ok(None);
    }
    let Some(hex) = encoded.strip_prefix('V') else {
        return Err(DatabaseError {
            message: "unexpected PostgreSQL default metadata marker".into(),
        });
    };
    decode_hex_metadata(hex).map(Some)
}

fn inspected_sql_type(data_type: &str, _udt_name: &str, length: &str) -> String {
    match data_type {
        "character varying" if !length.is_empty() => format!("VARCHAR({length})"),
        "character varying" => "TEXT".into(),
        "timestamp with time zone" => "TIMESTAMPTZ".into(),
        "double precision" => "DOUBLE PRECISION".into(),
        "integer" => "INTEGER".into(),
        "bigint" => "BIGINT".into(),
        "boolean" => "BOOLEAN".into(),
        "numeric" => "NUMERIC".into(),
        "text" => "TEXT".into(),
        other => other.to_uppercase(),
    }
}

fn parse_index_output(schema: &mut Schema, output: &str) -> Result<(), DatabaseError> {
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() < 3 {
            return Err(DatabaseError {
                message: format!("unexpected psql index row: {line}"),
            });
        }
        if fields[1].ends_with("_pkey") {
            continue;
        }
        let definition = fields[2];
        let Some(open) = definition.rfind('(') else {
            continue;
        };
        let Some(close) = definition[open..].find(')') else {
            continue;
        };
        let columns = definition[open + 1..open + close]
            .split(',')
            .map(|column| column.trim().trim_matches('"').to_string())
            .collect::<Vec<_>>();
        if let Some(table) = schema
            .tables
            .iter_mut()
            .find(|table| table.name == fields[0])
        {
            table.indexes.push(Index {
                name: fields[1].to_string(),
                columns,
                unique: definition.starts_with("CREATE UNIQUE INDEX"),
                constraint_owned: fields.get(3) == Some(&"t"),
            });
        }
    }
    Ok(())
}

pub fn apply_postgres(database_url: &str, sql: &str) -> Result<(), DatabaseError> {
    let mut child = Command::new("psql")
        .args(["-X", "-v", "ON_ERROR_STOP=1", "-d", database_url, "-f", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| DatabaseError {
            message: format!("could not start psql: {error}"),
        })?;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(sql.as_bytes())
        .map_err(|error| DatabaseError {
            message: format!("could not send SQL to psql: {error}"),
        })?;
    let output = child.wait_with_output().map_err(|error| DatabaseError {
        message: format!("could not wait for psql: {error}"),
    })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(DatabaseError {
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        })
    }
}

pub fn inspect_mariadb(database_url: &str) -> Result<Schema, DatabaseError> {
    let output = run_mariadb(
        database_url,
        "SELECT TABLE_NAME, COLUMN_NAME, DATA_TYPE, IS_NULLABLE, COLUMN_KEY, EXTRA, COALESCE(CHARACTER_MAXIMUM_LENGTH, ''), COALESCE(NUMERIC_PRECISION, ''), COALESCE(NUMERIC_SCALE, ''), CASE WHEN COLUMN_DEFAULT IS NULL THEN 'N' ELSE CONCAT('V', HEX(CAST(COLUMN_DEFAULT AS CHAR CHARACTER SET utf8mb4))) END FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() ORDER BY TABLE_NAME, ORDINAL_POSITION",
        None,
    )?;
    let mut schema = parse_mariadb_columns(&output)?;
    let indexes = run_mariadb(
        database_url,
        "SELECT TABLE_NAME, INDEX_NAME, NON_UNIQUE, SEQ_IN_INDEX, COLUMN_NAME FROM information_schema.STATISTICS WHERE TABLE_SCHEMA = DATABASE() AND INDEX_NAME <> 'PRIMARY' ORDER BY TABLE_NAME, INDEX_NAME, SEQ_IN_INDEX",
        None,
    )?;
    parse_mariadb_indexes(&mut schema, &indexes)?;
    let foreign_keys = run_mariadb(
        database_url,
        "SELECT TABLE_NAME, COLUMN_NAME, REFERENCED_TABLE_NAME, REFERENCED_COLUMN_NAME, CONSTRAINT_NAME FROM information_schema.KEY_COLUMN_USAGE WHERE TABLE_SCHEMA = DATABASE() AND REFERENCED_TABLE_NAME IS NOT NULL ORDER BY TABLE_NAME, CONSTRAINT_NAME, ORDINAL_POSITION",
        None,
    )?;
    parse_foreign_key_output(&mut schema, &foreign_keys)?;
    Ok(schema)
}

pub fn apply_mariadb(database_url: &str, sql: &str) -> Result<(), DatabaseError> {
    run_mariadb_sql(database_url, sql, None)
}

pub fn count_null_values(
    database_url: &str,
    backend: Backend,
    table: &str,
    column: &str,
) -> Result<u64, DatabaseError> {
    let query = nullability_count_sql(table, column, backend);
    let output = match backend {
        Backend::MariaDb => run_mariadb(database_url, &query, None)?,
        Backend::Postgres => run_psql(database_url, &query)?,
        Backend::Sqlite => {
            return Err(DatabaseError {
                message: "nullability preflight is not supported for SQLite schema changes".into(),
            });
        }
    };
    output.trim().parse().map_err(|_| DatabaseError {
        message: "database returned an invalid NULL row count".into(),
    })
}

pub fn table_has_rows(
    database_url: &str,
    backend: Backend,
    table: &str,
) -> Result<bool, DatabaseError> {
    let query = table_has_rows_sql(table, backend);
    let output = match backend {
        Backend::MariaDb => run_mariadb(database_url, &query, None)?,
        Backend::Postgres => run_psql(database_url, &query)?,
        Backend::Sqlite => run_sqlite(&sqlite_path(database_url)?, &query)?,
    };
    match output.trim().parse::<u8>() {
        Ok(0) => Ok(false),
        Ok(1) => Ok(true),
        _ => Err(DatabaseError {
            message: "database returned an invalid table row-presence result".into(),
        }),
    }
}

#[derive(Clone, Debug)]
pub enum QueryValue {
    Null,
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    String(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Clone, Debug)]
pub struct Query {
    pub sql: String,
    pub params: Vec<(String, QueryValue)>,
}

pub fn execute_mariadb_query(
    database_url: &str,
    sql: &str,
    params: Vec<(String, QueryValue)>,
) -> Result<QueryResult, DatabaseError> {
    let mut results = execute_mariadb_queries(
        database_url,
        &[Query {
            sql: sql.into(),
            params,
        }],
        false,
    )?;
    Ok(results.pop().unwrap_or_default())
}

pub fn execute_mariadb_queries(
    database_url: &str,
    queries: &[Query],
    transaction: bool,
) -> Result<Vec<QueryResult>, DatabaseError> {
    let _ = parse_mariadb_url(database_url)?;
    let mut script = String::new();
    if transaction {
        script.push_str("START TRANSACTION;\n");
    }
    for (query_index, query) in queries.iter().enumerate() {
        let (sql, parameter_order) = bind_named_parameters(&query.sql)?;
        for (parameter_index, parameter_name) in parameter_order.iter().enumerate() {
            let value = query
                .params
                .iter()
                .find(|(name, _)| name == parameter_name)
                .map(|(_, value)| value)
                .ok_or_else(|| DatabaseError {
                    message: format!("missing SQL parameter `:{parameter_name}`"),
                })?;
            script.push_str(&format!(
                "SET @zelyra_p{query_index}_{parameter_index} = {};\n",
                query_value_sql(value)
            ));
        }
        let statement_name = format!("zelyra_stmt_{query_index}");
        let prepared_sql = quote_string(&sql.replace('\\', "\\\\"));
        script.push_str(&format!("PREPARE {statement_name} FROM {prepared_sql};\n"));
        if parameter_order.is_empty() {
            script.push_str(&format!("EXECUTE {statement_name};\n"));
        } else {
            let variables = parameter_order
                .iter()
                .enumerate()
                .map(|(parameter_index, _)| format!("@zelyra_p{query_index}_{parameter_index}"))
                .collect::<Vec<_>>()
                .join(", ");
            script.push_str(&format!("EXECUTE {statement_name} USING {variables};\n"));
        }
        script.push_str(&format!("DEALLOCATE PREPARE {statement_name};\n"));
    }
    if transaction {
        script.push_str("COMMIT;\n");
    }
    let output = run_mariadb_query(database_url, &script)?;
    if transaction {
        return Ok(Vec::new());
    }
    Ok(vec![parse_query_result(&output)])
}

fn bind_named_parameters(sql: &str) -> Result<(String, Vec<String>), DatabaseError> {
    let bytes = sql.as_bytes();
    let mut bound = String::with_capacity(sql.len());
    let mut parameters = Vec::new();
    let mut index = 0;
    let mut quote = None;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(active_quote) = quote {
            bound.push(byte as char);
            if byte == active_quote {
                if bytes.get(index + 1) == Some(&active_quote) {
                    bound.push(active_quote as char);
                    index += 2;
                    continue;
                }
                quote = None;
            }
            index += 1;
            continue;
        }
        if byte == b'\'' || byte == b'"' {
            quote = Some(byte);
            bound.push(byte as char);
            index += 1;
            continue;
        }
        if byte == b':' {
            let start = index + 1;
            let mut end = start;
            while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                end += 1;
            }
            if end == start {
                return Err(DatabaseError {
                    message: "SQL contains a colon without a parameter name".into(),
                });
            }
            let name = sql[start..end].to_owned();
            parameters.push(name);
            bound.push('?');
            index = end;
            continue;
        }
        bound.push(byte as char);
        index += 1;
    }
    Ok((bound, parameters))
}

fn query_value_sql(value: &QueryValue) -> String {
    match value {
        QueryValue::Null => "NULL".into(),
        QueryValue::Int(value) => value.to_string(),
        QueryValue::UInt(value) => value.to_string(),
        QueryValue::Float(value) if value.is_finite() => value.to_string(),
        QueryValue::Float(_) => "NULL".into(),
        QueryValue::Bool(value) => if *value { "1" } else { "0" }.into(),
        QueryValue::String(value) => {
            format!("'{}'", value.replace('\\', "\\\\").replace('\'', "''"))
        }
    }
}

fn run_mariadb_query(database_url: &str, script: &str) -> Result<String, DatabaseError> {
    let connection = parse_mariadb_url(database_url)?;
    let mut command = Command::new("mariadb");
    command
        .args([
            "--batch",
            "--raw",
            "--host",
            &connection.host,
            "--port",
            &connection.port,
            "--user",
            &connection.user,
            "--database",
            &connection.database,
        ])
        .env("MYSQL_PWD", &connection.password)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| DatabaseError {
        message: format!("could not start mariadb: {error}"),
    })?;
    child
        .stdin
        .take()
        .expect("mariadb stdin was piped")
        .write_all(script.as_bytes())
        .map_err(|error| DatabaseError {
            message: format!("could not send SQL to mariadb: {error}"),
        })?;
    let output = child.wait_with_output().map_err(|error| DatabaseError {
        message: format!("could not wait for mariadb: {error}"),
    })?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(DatabaseError {
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        })
    }
}

fn parse_query_result(output: &str) -> QueryResult {
    let mut lines = output.lines();
    let Some(header) = lines.next() else {
        return QueryResult::default();
    };
    QueryResult {
        columns: header.split('\t').map(str::to_owned).collect(),
        rows: lines
            .map(|line| line.split('\t').map(str::to_owned).collect())
            .collect(),
    }
}

pub fn create_mariadb_database(database_url: &str) -> Result<(), DatabaseError> {
    let connection = parse_mariadb_url(database_url)?;
    let database = &connection.database;
    if database.is_empty() {
        return Err(DatabaseError {
            message: "MariaDB DATABASE_URL must include a database name".into(),
        });
    }
    let sql = format!(
        "CREATE DATABASE IF NOT EXISTS {} CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;",
        quote_identifier(database, Backend::MariaDb)
    );
    run_mariadb_sql(database_url, &sql, Some("mysql"))
}

pub fn inspect_sqlite(database_url: &str) -> Result<Schema, DatabaseError> {
    let path = sqlite_path(database_url)?;
    let tables_output = run_sqlite(&path, "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name;")?;
    let mut schema = Schema {
        database: Some(DatabaseConfig {
            name: "sqlite".into(),
            engine: "sqlite".into(),
            database: Some(path.clone()),
        }),
        tables: Vec::new(),
    };
    for table_name in tables_output.lines().filter(|line| !line.is_empty()) {
        let create_sql = run_sqlite(
            &path,
            &format!(
                "SELECT CASE WHEN sql IS NULL THEN 'N' ELSE 'V' || hex(CAST(sql AS BLOB)) END FROM sqlite_master WHERE type = 'table' AND name = {};",
                quote_string(table_name)
            ),
        )?;
        let has_autoincrement = sqlite_table_has_autoincrement(&create_sql)?;
        let pragma = format!(
            "SELECT cid, name, type, \"notnull\", CASE WHEN dflt_value IS NULL THEN 'N' ELSE 'V' || hex(CAST(dflt_value AS BLOB)) END, pk FROM pragma_table_info({}) ORDER BY cid;",
            quote_string(table_name)
        );
        let output = run_sqlite(&path, &pragma)?;
        let columns = parse_sqlite_columns(&output, has_autoincrement)?;
        schema.tables.push(Table {
            name: table_name.into(),
            columns,
            foreign_keys: Vec::new(),
            indexes: Vec::new(),
            uniques: Vec::new(),
        });
    }
    for table in &mut schema.tables {
        let pragma = format!(
            "PRAGMA index_list({});",
            quote_identifier(&table.name, Backend::Sqlite)
        );
        let output = run_sqlite(&path, &pragma)?;
        parse_sqlite_indexes(table, &path, &output)?;
        let pragma = format!(
            "PRAGMA foreign_key_list({});",
            quote_identifier(&table.name, Backend::Sqlite)
        );
        let output = run_sqlite(&path, &pragma)?;
        parse_sqlite_foreign_keys(table, &output)?;
    }
    Ok(schema)
}

pub fn apply_sqlite(database_url: &str, sql: &str) -> Result<(), DatabaseError> {
    let path = sqlite_path(database_url)?;
    run_sqlite_sql(&path, &format!("PRAGMA foreign_keys = ON;\n{sql}"))
}

fn parse_mariadb_url(url: &str) -> Result<MariaConnection, DatabaseError> {
    let rest = url
        .strip_prefix("mariadb://")
        .or_else(|| url.strip_prefix("mysql://"))
        .ok_or_else(|| DatabaseError {
            message: "MariaDB URL must use mariadb:// or mysql://".into(),
        })?;
    let (authority, database) = rest.split_once('/').ok_or_else(|| DatabaseError {
        message: "MariaDB URL must include a database name".into(),
    })?;
    let (credentials, hostport) = authority.split_once('@').ok_or_else(|| DatabaseError {
        message: "MariaDB URL must include user and host".into(),
    })?;
    let (user, password) = credentials.split_once(':').unwrap_or((credentials, ""));
    let (host, port) = hostport.split_once(':').unwrap_or((hostport, "3306"));
    if user.is_empty() || host.is_empty() || database.is_empty() {
        return Err(DatabaseError {
            message: "MariaDB URL contains an empty user, host, or database".into(),
        });
    }
    Ok(MariaConnection {
        user: percent_decode(user),
        password: percent_decode(password),
        host: percent_decode(host),
        port: port.into(),
        database: percent_decode(database.trim_start_matches('/')),
    })
}

#[derive(Debug)]
struct MariaConnection {
    user: String,
    password: String,
    host: String,
    port: String,
    database: String,
}

fn run_mariadb(
    database_url: &str,
    query: &str,
    database_override: Option<&str>,
) -> Result<String, DatabaseError> {
    let connection = parse_mariadb_url(database_url)?;
    let database = database_override.unwrap_or(&connection.database);
    let mut command = Command::new("mariadb");
    command
        .args([
            "--batch",
            "--skip-column-names",
            "--raw",
            "--host",
            &connection.host,
            "--port",
            &connection.port,
            "--user",
            &connection.user,
            "--database",
            database,
            "--execute",
            query,
        ])
        .env("MYSQL_PWD", &connection.password);
    let output = command.output().map_err(|error| DatabaseError {
        message: format!("could not start mariadb: {error}"),
    })?;
    if !output.status.success() {
        return Err(DatabaseError {
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn run_mariadb_sql(
    database_url: &str,
    sql: &str,
    database_override: Option<&str>,
) -> Result<(), DatabaseError> {
    let connection = parse_mariadb_url(database_url)?;
    let database = database_override.unwrap_or(&connection.database);
    let mut command = Command::new("mariadb");
    command
        .args([
            "--batch",
            "--host",
            &connection.host,
            "--port",
            &connection.port,
            "--user",
            &connection.user,
            "--database",
            database,
        ])
        .env("MYSQL_PWD", &connection.password)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| DatabaseError {
        message: format!("could not start mariadb: {error}"),
    })?;
    child
        .stdin
        .take()
        .expect("mariadb stdin was piped")
        .write_all(sql.as_bytes())
        .map_err(|error| DatabaseError {
            message: format!("could not send SQL to mariadb: {error}"),
        })?;
    let output = child.wait_with_output().map_err(|error| DatabaseError {
        message: format!("could not wait for mariadb: {error}"),
    })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(DatabaseError {
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        })
    }
}

fn parse_mariadb_columns(output: &str) -> Result<Schema, DatabaseError> {
    let mut schema = Schema {
        database: None,
        tables: Vec::new(),
    };
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 10 {
            return Err(DatabaseError {
                message: format!(
                    "unexpected MariaDB column metadata with {} fields",
                    fields.len()
                ),
            });
        }
        let sql_type = mariadb_sql_type(fields[2], fields[6], fields[7], fields[8]);
        let default = mariadb_default_value(fields[9], &sql_type, fields[3] == "YES")?;
        let column = Column {
            name: fields[1].into(),
            sql_type,
            nullable: fields[3] == "YES",
            primary_key: fields[4] == "PRI",
            auto: fields[5].contains("auto_increment"),
            unique: fields[4] == "UNI",
            default,
        };
        if let Some(table) = schema
            .tables
            .iter_mut()
            .find(|table| table.name == fields[0])
        {
            table.columns.push(column);
        } else {
            schema.tables.push(Table {
                name: fields[0].into(),
                columns: vec![column],
                foreign_keys: Vec::new(),
                indexes: Vec::new(),
                uniques: Vec::new(),
            });
        }
    }
    Ok(schema)
}

fn mariadb_default_value(
    encoded: &str,
    sql_type: &str,
    nullable: bool,
) -> Result<Option<String>, DatabaseError> {
    let Some(hex) = encoded.strip_prefix('V') else {
        if encoded == "N" {
            return Ok(None);
        }
        return Err(DatabaseError {
            message: "unexpected MariaDB default metadata marker".into(),
        });
    };
    let value = decode_hex_metadata(hex)?;
    if nullable && value.eq_ignore_ascii_case("NULL") {
        return Ok(None);
    }
    let default = if sql_type == "TIMESTAMP"
        && (value.eq_ignore_ascii_case("current_timestamp")
            || value.eq_ignore_ascii_case("current_timestamp()"))
    {
        "CURRENT_TIMESTAMP".into()
    } else if sql_type == "BOOLEAN" && (value == "1" || value == "0") {
        if value == "1" {
            "TRUE".into()
        } else {
            "FALSE".into()
        }
    } else {
        value
    };
    Ok(Some(default))
}

fn decode_hex_metadata(hex: &str) -> Result<String, DatabaseError> {
    let bytes = hex
        .as_bytes()
        .chunks(2)
        .map(|pair| {
            if pair.len() != 2 {
                return Err(DatabaseError {
                    message: "invalid encoded schema metadata".into(),
                });
            }
            let digits = std::str::from_utf8(pair).map_err(|_| DatabaseError {
                message: "invalid encoded schema metadata".into(),
            })?;
            u8::from_str_radix(digits, 16).map_err(|_| DatabaseError {
                message: "invalid encoded schema metadata".into(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| DatabaseError {
        message: "encoded schema metadata is not valid UTF-8".into(),
    })
}

fn mariadb_sql_type(data_type: &str, length: &str, precision: &str, scale: &str) -> String {
    match data_type.to_ascii_lowercase().as_str() {
        "varchar" | "char" if !length.is_empty() => {
            format!("{}({length})", data_type.to_ascii_uppercase())
        }
        "decimal" | "numeric" if !precision.is_empty() && !scale.is_empty() => {
            format!("DECIMAL({precision},{scale})")
        }
        "decimal" | "numeric" if !precision.is_empty() => format!("DECIMAL({precision})"),
        "tinyint" => "BOOLEAN".into(),
        "int" | "integer" => "INTEGER".into(),
        "double" => "DOUBLE".into(),
        "longblob" | "mediumblob" | "tinyblob" => "BLOB".into(),
        other => other.to_uppercase(),
    }
}

fn parse_mariadb_indexes(schema: &mut Schema, output: &str) -> Result<(), DatabaseError> {
    let mut grouped: HashMap<(String, String), (bool, Vec<String>)> = HashMap::new();
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 5 {
            return Err(DatabaseError {
                message: format!("unexpected MariaDB index row: {line}"),
            });
        }
        let key = (fields[0].into(), fields[1].into());
        let entry = grouped
            .entry(key)
            .or_insert_with(|| (fields[2] == "0", Vec::new()));
        entry.1.push(fields[4].into());
    }
    for ((table_name, index_name), (unique, columns)) in grouped {
        if let Some(table) = schema
            .tables
            .iter_mut()
            .find(|table| table.name == table_name)
        {
            let index = Index {
                name: index_name,
                columns,
                unique,
                constraint_owned: unique,
            };
            if unique {
                table.uniques.push(index);
            } else {
                table.indexes.push(index);
            }
        }
    }
    Ok(())
}

fn parse_foreign_key_output(schema: &mut Schema, output: &str) -> Result<(), DatabaseError> {
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 5 {
            return Err(DatabaseError {
                message: format!("unexpected foreign-key inspection row: {line}"),
            });
        }
        if let Some(table) = schema
            .tables
            .iter_mut()
            .find(|table| table.name == fields[0])
        {
            table.foreign_keys.push(ForeignKey {
                name: Some(fields[4].into()),
                column: fields[1].into(),
                referenced_table: fields[2].into(),
                referenced_column: fields[3].into(),
            });
        }
    }
    Ok(())
}

fn sqlite_path(database_url: &str) -> Result<String, DatabaseError> {
    let path = database_url
        .strip_prefix("sqlite://")
        .or_else(|| database_url.strip_prefix("sqlite:"))
        .ok_or_else(|| DatabaseError {
            message: "SQLite DATABASE_URL must use sqlite:// or sqlite:".into(),
        })?;
    let path = percent_decode(path);
    if path.is_empty() {
        return Err(DatabaseError {
            message: "SQLite DATABASE_URL must include a database path".into(),
        });
    }
    Ok(path)
}

fn run_sqlite(path: &str, query: &str) -> Result<String, DatabaseError> {
    let output = Command::new("sqlite3")
        .args(["-batch", "-tabs", "-noheader", path, query])
        .output()
        .map_err(|error| DatabaseError {
            message: format!("could not start sqlite3: {error}"),
        })?;
    if !output.status.success() {
        return Err(DatabaseError {
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn run_sqlite_sql(path: &str, sql: &str) -> Result<(), DatabaseError> {
    let mut child = Command::new("sqlite3")
        .args(["-batch", path])
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| DatabaseError {
            message: format!("could not start sqlite3: {error}"),
        })?;
    child
        .stdin
        .take()
        .expect("sqlite3 stdin was piped")
        .write_all(sql.as_bytes())
        .map_err(|error| DatabaseError {
            message: format!("could not send SQL to sqlite3: {error}"),
        })?;
    let output = child.wait_with_output().map_err(|error| DatabaseError {
        message: format!("could not wait for sqlite3: {error}"),
    })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(DatabaseError {
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        })
    }
}

fn parse_sqlite_columns(
    output: &str,
    table_has_autoincrement: bool,
) -> Result<Vec<Column>, DatabaseError> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 6 {
                return Err(DatabaseError {
                    message: format!(
                        "unexpected SQLite column metadata with {} fields",
                        fields.len()
                    ),
                });
            }
            let default = if fields[4] == "N" {
                None
            } else {
                let Some(hex) = fields[4].strip_prefix('V') else {
                    return Err(DatabaseError {
                        message: "unexpected SQLite default metadata marker".into(),
                    });
                };
                Some(decode_hex_metadata(hex)?)
            };
            Ok(Column {
                name: fields[1].into(),
                sql_type: fields[2].to_ascii_uppercase(),
                // SQLite reports `notnull = 0` for `INTEGER PRIMARY KEY`,
                // although the primary key itself cannot be null.
                nullable: fields[3] != "1" && fields[5] == "0",
                primary_key: fields[5] != "0",
                auto: table_has_autoincrement
                    && fields[5] != "0"
                    && fields[2].eq_ignore_ascii_case("INTEGER"),
                unique: false,
                default,
            })
        })
        .collect()
}

fn sqlite_table_has_autoincrement(encoded_sql: &str) -> Result<bool, DatabaseError> {
    let Some(hex) = encoded_sql.trim().strip_prefix('V') else {
        if encoded_sql.trim() == "N" {
            return Ok(false);
        }
        return Err(DatabaseError {
            message: "unexpected SQLite table metadata marker".into(),
        });
    };
    let sql = decode_hex_metadata(hex)?;
    Ok(sql_contains_keyword(&sql, "AUTOINCREMENT"))
}

fn sql_contains_keyword(sql: &str, expected: &str) -> bool {
    let bytes = sql.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\'' | b'"' | b'`' => {
                let quote = bytes[index];
                index += 1;
                while index < bytes.len() {
                    if bytes[index] == quote {
                        if bytes.get(index + 1) == Some(&quote) {
                            index += 2;
                        } else {
                            index += 1;
                            break;
                        }
                    } else {
                        index += 1;
                    }
                }
            }
            b'[' => {
                index += 1;
                while index < bytes.len() && bytes[index] != b']' {
                    index += 1;
                }
                index = (index + 1).min(bytes.len());
            }
            b'-' if bytes.get(index + 1) == Some(&b'-') => {
                index += 2;
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index += 2;
                while index + 1 < bytes.len() && &bytes[index..index + 2] != b"*/" {
                    index += 1;
                }
                index = (index + 2).min(bytes.len());
            }
            byte if is_sql_identifier_byte(byte) => {
                let start = index;
                index += 1;
                while index < bytes.len() && is_sql_identifier_byte(bytes[index]) {
                    index += 1;
                }
                if sql[start..index].eq_ignore_ascii_case(expected) {
                    return true;
                }
            }
            _ => index += 1,
        }
    }
    false
}

fn is_sql_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$') || byte >= 0x80
}

fn parse_sqlite_indexes(table: &mut Table, path: &str, output: &str) -> Result<(), DatabaseError> {
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() < 3 || fields.get(3) == Some(&"pk") {
            continue;
        }
        let index_name = fields[1];
        let info = run_sqlite(
            path,
            &format!(
                "PRAGMA index_info({});",
                quote_identifier(index_name, Backend::Sqlite)
            ),
        )?;
        let columns = info
            .lines()
            .filter_map(|row| row.split('\t').nth(2))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let index = Index {
            name: index_name.into(),
            columns,
            unique: fields[2] == "1",
            constraint_owned: fields.get(3) == Some(&"u"),
        };
        if index.unique {
            table.uniques.push(index);
        } else {
            table.indexes.push(index);
        }
    }
    Ok(())
}

fn parse_sqlite_foreign_keys(table: &mut Table, output: &str) -> Result<(), DatabaseError> {
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() < 5 {
            return Err(DatabaseError {
                message: format!("unexpected SQLite foreign-key row: {line}"),
            });
        }
        table.foreign_keys.push(ForeignKey {
            name: None,
            column: fields[3].into(),
            referenced_table: fields[2].into(),
            referenced_column: fields[4].into(),
        });
    }
    Ok(())
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = &value[index + 1..index + 3];
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                decoded.push(byte);
                index += 3;
                continue;
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use zelyra_lexer::lex;
    use zelyra_parser::parse;

    fn schema(source: &str) -> Schema {
        let program = parse(&lex(source).unwrap()).unwrap();
        build_schema(&program).unwrap()
    }

    #[test]
    fn builds_postgres_schema_with_relation_and_indexes() {
        let schema = schema(
            r#"
            database main { engine: postgres database: "production" }
            table departments {
                id: Id primary auto
                name: String(100) required unique
            }
            table machines {
                id: Id primary auto
                number: String(30) required unique
                department: Department required
                active: Bool default true
                index { department }
                unique { number department }
            }
        "#,
        );
        let machines = &schema.tables[1];
        assert_eq!(machines.foreign_keys[0].column, "department_id");
        assert!(schema.create_sql().contains("REFERENCES \"departments\""));
        assert!(schema.create_sql().contains("VARCHAR(30)"));
        assert!(schema.create_sql().contains("DEFAULT TRUE"));
    }

    #[test]
    fn detects_destructive_schema_changes() {
        let desired = schema("table users { id: Id primary auto name: String(200) required }");
        let current =
            schema("table users { id: Id primary auto name: String(100) required old: Bool }");
        let plan = diff(&desired, &current);
        assert!(plan
            .changes
            .iter()
            .any(|change| change.description.contains("change type")));
        assert!(plan
            .changes
            .iter()
            .any(|change| change.description.contains("drop column")));
        assert!(plan.is_destructive());
    }

    #[test]
    fn plans_new_indexes_without_dropping_unmanaged_indexes() {
        let desired = schema("table users { id: Id primary auto name: String index { name } }");
        let current = schema("table users { id: Id primary auto name: String }");
        let plan = diff(&desired, &current);
        assert_eq!(plan.changes.len(), 1);
        assert_eq!(plan.changes[0].risk, Risk::Safe);
        assert!(plan.changes[0].sql.contains("CREATE INDEX"));
    }

    #[test]
    fn requires_review_for_required_column_without_default() {
        let desired = schema(
            "table users { id: Id primary auto name: String required active: Bool required }",
        );
        let current = schema("table users { id: Id primary auto name: String required }");
        let plan = diff(&desired, &current);
        assert_eq!(plan.changes.len(), 1);
        assert_eq!(plan.changes[0].risk, Risk::RequiresApproval);
        assert!(plan.requires_approval());
        assert!(!plan.has_unsupported());
    }

    #[test]
    fn detects_default_primary_key_and_mariadb_auto_increment_drift() {
        let desired = schema(
            "database main { engine: mariadb } table users { id: Id primary auto active: Bool default true name: String default \"new\" }",
        );
        let current = schema(
            "database main { engine: mariadb } table users { id: Id primary active: Bool name: String }",
        );
        let plan = diff(&desired, &current);
        assert!(plan.has_unsupported());
        assert!(plan
            .changes
            .iter()
            .any(|change| change.description == "change auto-increment status of users.id"));
        assert!(plan.changes.iter().any(|change| {
            change.description == "change default of users.active"
                && change.risk == Risk::RequiresApproval
                && change.sql.contains("SET DEFAULT TRUE")
        }));
        assert!(plan.changes.iter().any(|change| {
            change.description == "change default of users.name"
                && change.risk == Risk::RequiresApproval
                && change.sql.contains("SET DEFAULT 'new'")
        }));

        let current = schema(
            "database main { engine: mariadb } table users { id: Id primary auto active: Bool default true name: String default \"new\" }",
        );
        let desired = schema(
            "database main { engine: mariadb } table users { id: Id active: Bool default true name: String default \"new\" }",
        );
        let plan = diff(&desired, &current);
        assert!(plan.has_unsupported());
        assert!(plan
            .changes
            .iter()
            .any(|change| change.description == "change primary-key status of users.id"));
    }

    #[test]
    fn plans_reviewable_default_set_change_and_drop_for_mariadb_and_postgres() {
        for (engine, quote) in [("mariadb", '`'), ("postgres", '"')] {
            let source = |active_default: &str, name_default: &str| {
                format!(
                    "database main {{ engine: {engine} }} table users {{ id: Id primary auto active: Bool required{active_default} name: String(80) required{name_default} }}"
                )
            };
            let identifier = |name: &str| format!("{quote}{name}{quote}");
            let no_defaults = source("", "");
            let initial_defaults = source(" default true", " default \"first\"");
            let changed_defaults = source(" default false", " default \"O'Reilly\"");

            let add = diff(&schema(&initial_defaults), &schema(&no_defaults));
            assert_eq!(add.changes.len(), 2);
            assert!(add
                .changes
                .iter()
                .all(|change| change.risk == Risk::RequiresApproval));
            assert!(add.changes[0].sql.contains(&format!(
                "ALTER TABLE {} ALTER COLUMN {} SET DEFAULT TRUE;",
                identifier("users"),
                identifier("active")
            )));
            assert!(add.changes[1].sql.contains(&format!(
                "ALTER TABLE {} ALTER COLUMN {} SET DEFAULT 'first';",
                identifier("users"),
                identifier("name")
            )));

            let change = diff(&schema(&changed_defaults), &schema(&initial_defaults));
            assert_eq!(change.changes.len(), 2);
            assert!(change
                .changes
                .iter()
                .all(|change| change.risk == Risk::RequiresApproval));
            assert!(change.changes[0].sql.contains("SET DEFAULT FALSE"));
            assert!(change.changes[1].sql.contains("SET DEFAULT 'O''Reilly'"));

            let drop = diff(&schema(&no_defaults), &schema(&initial_defaults));
            assert_eq!(drop.changes.len(), 2);
            assert!(drop
                .changes
                .iter()
                .all(|change| change.risk == Risk::RequiresApproval));
            assert!(drop
                .changes
                .iter()
                .all(|change| change.sql.contains("DROP DEFAULT;")));
        }
    }

    #[test]
    fn detects_sqlite_default_primary_key_and_autoincrement_drift() {
        let desired = schema(
            "database main { engine: sqlite } table users { id: Id active: Bool default true }",
        );
        let mut current = schema(
            "database main { engine: sqlite } table users { id: Id primary auto active: Bool }",
        );
        current.tables[0].columns[1].default = None;
        let plan = diff(&desired, &current);
        assert!(plan.has_unsupported());
        assert!(plan
            .changes
            .iter()
            .any(|change| change.description == "change primary-key status of users.id"));
        assert!(plan
            .changes
            .iter()
            .any(|change| change.description == "change default of users.active"));
        current.tables[0].columns[0].auto = false;
        let plan = diff(
            &schema("database main { engine: sqlite } table users { id: Id primary auto }"),
            &current,
        );
        assert!(plan.has_unsupported());
        assert!(plan
            .changes
            .iter()
            .any(|change| { change.description == "change auto-increment status of users.id" }));
    }

    #[test]
    fn treats_equivalent_boolean_and_timestamp_defaults_as_equal() {
        assert!(defaults_equivalent(Some("1"), Some("TRUE")));
        assert!(defaults_equivalent(Some("0"), Some("FALSE")));
        assert!(defaults_equivalent(
            Some("current_timestamp()"),
            Some("CURRENT_TIMESTAMP")
        ));
        assert!(!defaults_equivalent(Some("'true'"), Some("TRUE")));
        assert!(!defaults_equivalent(None, Some("0")));
    }

    #[test]
    fn compares_postgres_defaults_after_removing_only_known_literal_casts() {
        assert!(postgres_defaults_equivalent(
            Some("'pending'::character varying"),
            Some("'pending'")
        ));
        assert!(postgres_defaults_equivalent(
            Some("'O''Reilly'::text"),
            Some("'O''Reilly'")
        ));
        assert!(postgres_defaults_equivalent(Some("true"), Some("TRUE")));
        assert!(!postgres_defaults_equivalent(
            Some("'first'::text"),
            Some("'second'")
        ));
        assert!(!postgres_defaults_equivalent(
            Some("'first'::custom_domain"),
            Some("'first'")
        ));
        assert!(!postgres_defaults_equivalent(
            Some("lower('first'::text)"),
            Some("'first'")
        ));
    }

    #[test]
    fn compares_postgres_serial_aliases_by_their_storage_types() {
        assert!(sql_types_equivalent(
            "BIGINT",
            "BIGSERIAL",
            Backend::Postgres
        ));
        assert!(sql_types_equivalent("INTEGER", "SERIAL", Backend::Postgres));
        assert!(sql_types_equivalent(
            "SMALLINT",
            "SMALLSERIAL",
            Backend::Postgres
        ));
        assert!(!sql_types_equivalent(
            "BIGINT",
            "INTEGER",
            Backend::Postgres
        ));
        assert!(!sql_types_equivalent(
            "BIGINT",
            "BIGSERIAL",
            Backend::MariaDb
        ));
    }

    #[test]
    fn keeps_timestamp_looking_string_defaults_as_literals() {
        assert_eq!(
            mariadb_default_value(
                "V2743555252454e545f54494d455354414d5027",
                "VARCHAR(40)",
                false,
            )
            .unwrap()
            .as_deref(),
            Some("'CURRENT_TIMESTAMP'")
        );
        assert_eq!(
            mariadb_default_value(
                "V63757272656e745f74696d657374616d702829",
                "TIMESTAMP",
                false,
            )
            .unwrap()
            .as_deref(),
            Some("CURRENT_TIMESTAMP")
        );
        assert_eq!(
            mariadb_default_value("V4E554C4C", "TIMESTAMP", true).unwrap(),
            None
        );
        assert!(mariadb_default_value("V0", "TEXT", false).is_err());
    }

    #[test]
    fn reviews_nullability_changes_and_only_preflights_tightening() {
        for (engine, quote) in [("mariadb", '`'), ("postgres", '"')] {
            let source = |required: bool| {
                format!(
                    "database main {{ engine: {engine} }} table users {{ id: Id primary auto email: Email{} }}",
                    if required { " required" } else { "" }
                )
            };
            let tighten = diff(&schema(&source(true)), &schema(&source(false)));
            assert_eq!(tighten.changes.len(), 1);
            assert_eq!(tighten.changes[0].risk, Risk::RequiresApproval);
            assert!(!tighten.has_unsupported());
            assert_eq!(
                tighten.nullability_preflights,
                [NullabilityPreflight {
                    table: "users".into(),
                    column: "email".into(),
                }]
            );
            if engine == "postgres" {
                assert_eq!(
                    tighten.changes[0].sql,
                    format!(
                        "ALTER TABLE {quote}users{quote} ALTER COLUMN {quote}email{quote} SET NOT NULL;"
                    )
                );
            } else {
                assert_eq!(
                    tighten.changes[0].sql,
                    format!(
                        "ALTER TABLE {quote}users{quote} MODIFY COLUMN {quote}email{quote} VARCHAR(255) NOT NULL;"
                    )
                );
            }

            let relax = diff(&schema(&source(false)), &schema(&source(true)));
            assert_eq!(relax.changes.len(), 1);
            assert_eq!(relax.changes[0].risk, Risk::RequiresApproval);
            assert!(relax.nullability_preflights.is_empty());
            assert!(
                relax.changes[0].sql.contains("DROP NOT NULL")
                    || relax.changes[0].sql.contains("VARCHAR(255) NULL")
            );
        }

        let sqlite = diff(
            &schema("database main { engine: sqlite } table users { id: Id primary auto email: Email required }"),
            &schema("database main { engine: sqlite } table users { id: Id primary auto email: Email? }"),
        );
        assert_eq!(sqlite.changes[0].risk, Risk::Unsupported);
        assert!(sqlite.has_unsupported());
        assert!(sqlite.nullability_preflights.is_empty());
    }

    #[test]
    fn nullability_preflight_queries_quote_backend_identifiers() {
        assert_eq!(
            nullability_count_sql("user`data", "e`mail", Backend::MariaDb),
            "SELECT COUNT(*) FROM `user``data` WHERE `e``mail` IS NULL"
        );
        assert_eq!(
            nullability_count_sql("user\"data", "e\"mail", Backend::Postgres),
            "SELECT COUNT(*) FROM \"user\"\"data\" WHERE \"e\"\"mail\" IS NULL"
        );
    }

    #[test]
    fn required_column_additions_to_existing_tables_require_an_empty_table_preflight() {
        for engine in ["mariadb", "postgres", "sqlite"] {
            let desired = schema(&format!(
                "database main {{ engine: {engine} }} table users {{ id: Id primary auto name: String(80) required }}"
            ));
            let current = schema(&format!(
                "database main {{ engine: {engine} }} table users {{ id: Id primary auto }}"
            ));
            let plan = diff(&desired, &current);
            assert_eq!(plan.changes.len(), 1, "{engine}");
            assert_eq!(plan.changes[0].risk, Risk::RequiresApproval, "{engine}");
            assert_eq!(
                plan.required_column_preflights,
                [RequiredColumnPreflight {
                    table: "users".into(),
                    column: "name".into(),
                }],
                "{engine}"
            );
        }
    }

    #[test]
    fn defaults_and_new_tables_do_not_require_required_column_preflights() {
        let desired = schema(
            "database main { engine: mariadb } table users { id: Id primary auto name: String(80) required default \"new\" }",
        );
        let current =
            schema("database main { engine: mariadb } table users { id: Id primary auto }");
        assert!(diff(&desired, &current)
            .required_column_preflights
            .is_empty());

        let new_table = schema(
            "database main { engine: mariadb } table users { id: Id primary auto name: String(80) required }",
        );
        let empty_schema = schema("database main { engine: mariadb }");
        assert!(diff(&new_table, &empty_schema)
            .required_column_preflights
            .is_empty());
    }

    #[test]
    fn table_presence_queries_quote_backend_identifiers() {
        assert_eq!(
            table_has_rows_sql("user`data", Backend::MariaDb),
            "SELECT CASE WHEN EXISTS (SELECT 1 FROM `user``data` LIMIT 1) THEN 1 ELSE 0 END"
        );
        assert_eq!(
            table_has_rows_sql("user\"data", Backend::Postgres),
            "SELECT CASE WHEN EXISTS (SELECT 1 FROM \"user\"\"data\" LIMIT 1) THEN 1 ELSE 0 END"
        );
        assert_eq!(
            table_has_rows_sql("user\"data", Backend::Sqlite),
            "SELECT CASE WHEN EXISTS (SELECT 1 FROM \"user\"\"data\" LIMIT 1) THEN 1 ELSE 0 END"
        );
    }

    #[test]
    fn type_widening_does_not_hide_reviewable_nullability_drift() {
        let desired = schema(
            "database main { engine: mariadb } table users { id: Id primary auto name: String(200) required }",
        );
        let mut current = schema(
            "database main { engine: mariadb } table users { id: Id primary auto name: String(100) required }",
        );
        current.tables[0].columns[1].nullable = true;
        let plan = diff(&desired, &current);
        assert!(!plan.has_unsupported());
        assert!(plan.changes.iter().any(|change| {
            change.description == "change type and nullability of users.name"
                && change.risk == Risk::RequiresApproval
        }));
        assert_eq!(plan.nullability_preflights.len(), 1);
    }

    #[test]
    fn detects_unique_index_addition_and_removal_by_semantics() {
        let desired = schema("table users { id: Id primary auto email: Email unique }");
        let current = schema("table users { id: Id primary auto email: Email }");
        let addition = diff(&desired, &current);
        assert_eq!(addition.changes.len(), 1);
        assert_eq!(addition.changes[0].risk, Risk::RequiresApproval);
        assert!(addition.changes[0].sql.contains("CREATE UNIQUE INDEX"));

        let removal = diff(&current, &desired);
        assert_eq!(removal.changes.len(), 1);
        assert_eq!(removal.changes[0].risk, Risk::RequiresApproval);
        assert!(removal.changes[0].sql.contains("DROP INDEX"));
    }

    #[test]
    fn detects_removed_indexes_and_keeps_output_deterministic() {
        let desired = schema("table users { id: Id primary auto name: String }");
        let current = schema("table users { id: Id primary auto name: String index { name } }");
        let plan = diff(&desired, &current);
        assert_eq!(plan.changes.len(), 1);
        assert_eq!(plan.changes[0].risk, Risk::RequiresApproval);
        assert!(plan.changes[0].sql.contains("DROP INDEX"));
        assert_eq!(plan, diff(&desired, &current));
    }

    #[test]
    fn preserves_indexes_not_owned_by_zelyra() {
        let desired = schema("table users { id: Id primary auto name: String }");
        let mut current = schema("table users { id: Id primary auto name: String index { name } }");
        current.tables[0].indexes[0].name = "externally_managed_name_idx".into();
        assert!(diff(&desired, &current).changes.is_empty());
    }

    #[test]
    fn refuses_sqlite_unique_constraint_removal_without_table_rebuild() {
        let desired = schema(
            "database main { engine: sqlite } table users { id: Id primary auto email: Email }",
        );
        let mut current = desired.clone();
        current.tables[0].uniques.push(Index {
            name: "sqlite_autoindex_users_1".into(),
            columns: vec!["email".into()],
            unique: true,
            constraint_owned: true,
        });
        let plan = diff(&desired, &current);
        assert_eq!(plan.changes.len(), 1);
        assert_eq!(plan.changes[0].risk, Risk::Unsupported);
        assert!(plan.has_unsupported());
    }

    #[test]
    fn detects_foreign_key_addition_and_removal() {
        let desired = schema(
            "table departments { id: Id primary auto } table machines { id: Id primary auto department: Department required }",
        );
        let current = schema(
            "table departments { id: Id primary auto } table machines { id: Id primary auto department_id: Id required }",
        );
        let addition = diff(&desired, &current);
        let foreign_key_add = addition
            .changes
            .iter()
            .find(|change| change.description.starts_with("add foreign key"))
            .unwrap();
        assert_eq!(foreign_key_add.risk, Risk::RequiresApproval);
        assert!(foreign_key_add
            .sql
            .contains("ADD CONSTRAINT `fk_machines_department_id`"));

        let mut current = Schema {
            tables: desired
                .tables
                .iter()
                .cloned()
                .map(|mut table| {
                    if table.name == "machines" {
                        table.foreign_keys[0].name = Some("fk_machines_department_id".into());
                    }
                    table
                })
                .collect(),
            database: desired.database.clone(),
        };
        let empty_fk_schema = schema(
            "table departments { id: Id primary auto } table machines { id: Id primary auto department_id: Id required }",
        );
        let removal = diff(&empty_fk_schema, &current);
        let foreign_key_drop = removal
            .changes
            .iter()
            .find(|change| change.description.starts_with("drop foreign key"))
            .unwrap();
        assert_eq!(foreign_key_drop.risk, Risk::RequiresApproval);
        assert!(foreign_key_drop
            .sql
            .contains("DROP FOREIGN KEY `fk_machines_department_id`"));

        current.tables[1].foreign_keys[0].name = Some("custom_department_fk".into());
        let untracked_removal = diff(&empty_fk_schema, &current);
        assert!(untracked_removal.has_unsupported());
        assert!(untracked_removal.changes[0]
            .description
            .starts_with("remove untracked foreign key"));
    }

    #[test]
    fn refuses_to_apply_unidentified_or_sqlite_foreign_key_changes() {
        let mut current = schema(
            "database main { engine: mariadb } table departments { id: Id primary auto } table machines { id: Id primary auto department: Department required }",
        );
        current.tables[1].foreign_keys[0].name = None;
        let desired = schema(
            "database main { engine: mariadb } table departments { id: Id primary auto } table machines { id: Id primary auto department_id: Id required }",
        );
        assert!(diff(&desired, &current).has_unsupported());

        let sqlite_current = schema(
            "database main { engine: sqlite } table departments { id: Id primary auto } table machines { id: Id primary auto department: Department required }",
        );
        let sqlite_desired = schema(
            "database main { engine: sqlite } table departments { id: Id primary auto } table machines { id: Id primary auto department_id: Id required }",
        );
        let plan = diff(&sqlite_desired, &sqlite_current);
        assert!(plan.has_unsupported());
    }

    #[test]
    fn sqlite_type_changes_are_explicitly_unsupported() {
        let desired = schema(
            "database main { engine: sqlite } table users { id: Id primary auto name: String(200) }",
        );
        let current = schema(
            "database main { engine: sqlite } table users { id: Id primary auto name: String(100) }",
        );
        let plan = diff(&desired, &current);
        assert_eq!(plan.changes.len(), 1);
        assert_eq!(plan.changes[0].risk, Risk::Unsupported);
        assert!(plan.has_unsupported());
    }

    #[test]
    fn includes_declared_indexes_when_planning_a_new_table() {
        let desired = schema("table users { id: Id primary auto name: String index { name } }");
        let current = Schema {
            database: desired.database.clone(),
            tables: Vec::new(),
        };
        let plan = diff(&desired, &current);
        assert_eq!(plan.changes.len(), 1);
        assert!(plan.changes[0].sql.contains("CREATE INDEX IF NOT EXISTS"));
    }

    #[test]
    fn drops_postgres_constraint_owned_unique_indexes_as_constraints() {
        let table = Table {
            name: "users".into(),
            columns: Vec::new(),
            foreign_keys: Vec::new(),
            indexes: Vec::new(),
            uniques: Vec::new(),
        };
        let index = Index {
            name: "users_email_key".into(),
            columns: vec!["email".into()],
            unique: true,
            constraint_owned: true,
        };
        let (sql, risk) = drop_index_sql(&table, &index, Backend::Postgres);
        assert_eq!(risk, Risk::RequiresApproval);
        assert!(sql.contains("ALTER TABLE \"users\" DROP CONSTRAINT \"users_email_key\""));
    }

    #[test]
    fn escapes_sql_strings() {
        let schema =
            schema("table users { id: Id primary auto name: String default \"O'Reilly\" }");
        assert!(schema.create_sql().contains("DEFAULT 'O''Reilly'"));
    }

    #[test]
    fn rejects_unknown_relation_targets() {
        let program = parse(
            &lex("table machines { id: Id primary auto department: MissingDepartment required }")
                .unwrap(),
        )
        .unwrap();
        let errors = build_schema(&program).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("MissingDepartment")));
    }

    #[test]
    fn parses_postgres_inspection_rows_and_indexes() {
        let mut schema = parse_inspection_output(
            "machines\tid\tbigint\tint8\tNO\t\tt\tt\tN\nmachines\tname\tcharacter varying\tvarchar\tYES\t100\tf\tf\tN\n",
        )
        .unwrap();
        parse_index_output(
            &mut schema,
            "machines\tidx_machines_name\tCREATE INDEX idx_machines_name ON public.machines USING btree (name)\n",
        )
        .unwrap();
        assert!(schema.tables[0].columns[0].primary_key);
        assert!(schema.tables[0].columns[0].auto);
        assert!(!schema.tables[0].columns[1].primary_key);
        assert_eq!(schema.tables[0].columns[1].sql_type, "VARCHAR(100)");
        assert_eq!(schema.tables[0].indexes[0].columns, ["name"]);
    }

    #[test]
    fn postgres_inspection_decodes_defaults_and_redacts_malformed_metadata() {
        let schema = parse_inspection_output(
            "users\tactive\tboolean\tbool\tNO\t\tf\tf\tV74727565\nusers\tid\tbigint\tint8\tNO\t\tt\tt\tN\n",
        )
        .unwrap();
        assert_eq!(schema.tables[0].columns[0].default.as_deref(), Some("true"));
        assert_eq!(schema.tables[0].columns[1].default, None);

        let malformed = parse_inspection_output("sensitive-default-value").unwrap_err();
        assert!(!malformed.message.contains("sensitive-default-value"));
        let malformed_boolean =
            parse_inspection_output("users\tid\tbigint\tint8\tNO\t\tsecret\tf\tN\n").unwrap_err();
        assert!(!malformed_boolean.message.contains("secret"));
    }

    #[test]
    fn renders_mariadb_and_sqlite_dialects() {
        let mariadb = schema(
            r#"
            database main { engine: mariadb database: "test" }
            table users { id: Id primary auto name: String(100) active: Bool default true }
            "#,
        );
        let mariadb_sql = mariadb.create_sql();
        assert!(mariadb_sql.contains("`id` BIGINT PRIMARY KEY NOT NULL AUTO_INCREMENT"));
        assert!(mariadb_sql.contains("`active` BOOLEAN DEFAULT TRUE"));

        let sqlite = schema(
            r#"
            database main { engine: sqlite database: "test.sqlite3" }
            table users { id: Id primary auto name: String(100) active: Bool default true }
            "#,
        );
        let sqlite_sql = sqlite.create_sql();
        assert!(sqlite_sql.contains("\"id\" INTEGER PRIMARY KEY AUTOINCREMENT"));
        assert!(sqlite_sql.contains("\"active\" INTEGER DEFAULT TRUE"));
    }

    #[test]
    fn maps_optional_timestamp_columns_to_timestamp_storage() {
        let schema = schema(
            r#"
            table users {
                id: Id primary auto
                deleted_at: Timestamp?
            }
            "#,
        );
        assert!(schema.create_sql().contains("`deleted_at` TIMESTAMP"));
    }

    #[test]
    fn defaults_new_schemas_to_mariadb() {
        let schema = schema("table users { id: Id primary auto name: String }");
        assert_eq!(schema.backend(), Backend::MariaDb);
        assert!(schema.create_sql().contains("`id` BIGINT"));
    }

    #[test]
    fn parses_sqlite_columns_foreign_keys_and_indexes() {
        let columns = parse_sqlite_columns(
            "0\tid\tINTEGER\t1\tN\t1\n1\tdepartment_id\tINTEGER\t1\tN\t0\n",
            true,
        )
        .unwrap();
        let mut table = Table {
            name: "machines".into(),
            columns,
            foreign_keys: Vec::new(),
            indexes: Vec::new(),
            uniques: Vec::new(),
        };
        parse_sqlite_foreign_keys(
            &mut table,
            "0\t0\tdepartments\tdepartment_id\tid\tNO ACTION\tNO ACTION\tNONE\n",
        )
        .unwrap();
        assert_eq!(table.foreign_keys[0].referenced_table, "departments");
        assert!(table.columns[0].auto);
    }

    #[test]
    fn parses_sqlite_default_metadata() {
        let columns = parse_sqlite_columns(
            "0\tid\tINTEGER\t1\tN\t1\n1\tname\tTEXT\t0\tV2768656c6c6f27\t0\n2\tactive\tINTEGER\t1\tV54525545\t0\n",
            false,
        )
        .unwrap();
        assert_eq!(columns[0].default, None);
        assert!(!columns[0].auto);
        assert_eq!(columns[1].default.as_deref(), Some("'hello'"));
        assert_eq!(columns[2].default.as_deref(), Some("TRUE"));
        assert_eq!(decode_hex_metadata("276109620a6327").unwrap(), "'a\tb\nc'");
        let error = parse_sqlite_columns("secret-default-metadata", false).unwrap_err();
        assert!(!error.message.contains("secret-default-metadata"));
    }

    #[test]
    fn recognizes_sqlite_autoincrement_as_a_sql_keyword_only() {
        assert!(sql_contains_keyword(
            "CREATE TABLE items (id INTEGER PRIMARY KEY AUTOINCREMENT)",
            "AUTOINCREMENT"
        ));
        assert!(sql_contains_keyword(
            "CREATE TABLE items (id INTEGER PRIMARY KEY /* marker */ AUTOINCREMENT)",
            "AUTOINCREMENT"
        ));
        for sql in [
            "CREATE TABLE items (id INTEGER PRIMARY KEY)",
            "CREATE TABLE items (id INTEGER PRIMARY KEY, note TEXT DEFAULT 'AUTOINCREMENT')",
            "CREATE TABLE items (id INTEGER PRIMARY KEY) -- AUTOINCREMENT\n",
            "CREATE TABLE items (id INTEGER PRIMARY KEY /* AUTOINCREMENT */)",
            "CREATE TABLE items (\"AUTOINCREMENT\" TEXT, id INTEGER PRIMARY KEY)",
            "CREATE TABLE items ([AUTOINCREMENT] TEXT, id INTEGER PRIMARY KEY)",
            "CREATE TABLE items (id INTEGER PRIMARY KEY, autoincrement_note TEXT)",
            "CREATE TABLE items (id INTEGER PRIMARY KEY, note TEXT DEFAULT 'xAUTOINCREMENT')",
        ] {
            assert!(
                !sql_contains_keyword(sql, "AUTOINCREMENT"),
                "unexpected keyword in {sql}"
            );
        }
    }

    #[test]
    fn parses_sqlite_autoincrement_metadata_without_exposing_table_sql() {
        assert!(sqlite_table_has_autoincrement("V435245415445205441424c45206974656d732028696420494e5445474552205052494d415259204b4559204155544f494e4352454d454e5429").unwrap());
        assert!(!sqlite_table_has_autoincrement("N").unwrap());
        let error = sqlite_table_has_autoincrement("private-table-definition").unwrap_err();
        assert!(!error.message.contains("private-table-definition"));
    }

    #[test]
    fn parses_mariadb_default_metadata_without_exposing_encoded_values() {
        let schema = parse_mariadb_columns(
            "users\tname\tvarchar\tNO\t\t\t80\t\t\tV2768656c6c6f27\nusers\tactive\ttinyint\tNO\t\t\t1\t\t\tV31\nusers\tempty\tvarchar\tNO\t\t\t80\t\t\tV2727\nusers\tmissing\tvarchar\tYES\t\t\t80\t\t\tN\nusers\tdeleted_at\ttimestamp\tYES\t\t\t\t\t\tV4E554C4C\n",
        )
        .unwrap();
        let columns = &schema.tables[0].columns;
        assert_eq!(columns[0].default.as_deref(), Some("'hello'"));
        assert_eq!(columns[1].default.as_deref(), Some("TRUE"));
        assert_eq!(columns[2].default.as_deref(), Some("''"));
        assert_eq!(columns[3].default, None);
        assert_eq!(columns[4].default, None);
        assert!(!columns[1].primary_key);
    }

    #[test]
    fn malformed_mariadb_metadata_errors_do_not_echo_row_contents() {
        let error = parse_mariadb_columns("secret-default-metadata").unwrap_err();
        assert!(!error.message.contains("secret-default-metadata"));
    }

    #[test]
    fn parses_mariadb_foreign_keys() {
        let mut schema =
            parse_mariadb_columns("machines\tid\tbigint\tNO\tPRI\tauto_increment\t\t\t\tN\n")
                .unwrap();
        parse_foreign_key_output(
            &mut schema,
            "machines\tdepartment_id\tdepartments\tid\tfk_machines_department_id\n",
        )
        .unwrap();
        assert_eq!(schema.tables[0].foreign_keys[0].column, "department_id");
    }

    #[test]
    fn binds_named_parameters_outside_sql_literals() {
        let (sql, parameters) = bind_named_parameters(
            "SELECT ':literal', name FROM customers WHERE id = :id OR id = :id",
        )
        .unwrap();
        assert_eq!(
            sql,
            "SELECT ':literal', name FROM customers WHERE id = ? OR id = ?"
        );
        assert_eq!(parameters, ["id", "id"]);
    }
}
