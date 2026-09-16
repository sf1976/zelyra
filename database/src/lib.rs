use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::Write;
use std::process::{Command, Stdio};
use zelyra_ast::*;

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
    pub column: String,
    pub referenced_table: String,
    pub referenced_column: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Index {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
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
    let resolved = resolve_type(&definition.ty, type_definitions);
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
        Type::Named(_) | Type::Option(_) | Type::Result(_, _) | Type::Unit | Type::Unknown => {
            "TEXT"
        }
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

fn quote_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

impl Schema {
    pub fn create_sql(&self) -> String {
        let backend = self.backend();
        let mut statements = Vec::new();
        for table in &self.tables {
            statements.push(table_create_sql(table, backend));
            for index in &table.indexes {
                statements.push(index_sql(table, index, backend));
            }
        }
        statements.join("\n\n")
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
        format!(
            "CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} ({})",
            quote_identifier(
                &format!("fk_{}_{}", table.name, foreign_key.column),
                backend
            ),
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
    Destructive,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaChange {
    pub description: String,
    pub sql: String,
    pub risk: Risk,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SchemaPlan {
    pub changes: Vec<SchemaChange>,
}

impl SchemaPlan {
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
                sql: table_create_sql(desired_table, backend),
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
            match current_columns.get(desired_column.name.as_str()) {
                None => plan.changes.push(SchemaChange {
                    description: format!(
                        "add column {}.{}",
                        desired_table.name, desired_column.name
                    ),
                    sql: format!(
                        "ALTER TABLE {} ADD COLUMN {};",
                        quote_identifier(&desired_table.name, backend),
                        column_sql(desired_column, backend)
                    ),
                    risk: Risk::Safe,
                }),
                Some(current_column) if current_column.sql_type != desired_column.sql_type => {
                    plan.changes.push(SchemaChange {
                        description: format!(
                            "change type of {}.{}",
                            desired_table.name, desired_column.name
                        ),
                        sql: alter_column_sql(desired_table, desired_column, backend),
                        risk: type_change_risk(&current_column.sql_type, &desired_column.sql_type),
                    })
                }
                _ => {}
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
        let current_indexes = current_table
            .indexes
            .iter()
            .chain(current_table.uniques.iter())
            .map(|index| index.name.as_str())
            .collect::<HashSet<_>>();
        for index in desired_table
            .indexes
            .iter()
            .chain(desired_table.uniques.iter())
        {
            if !current_indexes.contains(index.name.as_str()) {
                plan.changes.push(SchemaChange {
                    description: format!("add index {}", index.name),
                    sql: index_sql(desired_table, index, backend),
                    risk: Risk::Safe,
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
    plan
}

fn type_change_risk(current: &str, desired: &str) -> Risk {
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
    let query = "SELECT table_name, column_name, data_type, udt_name, is_nullable, COALESCE(character_maximum_length::text, '') FROM information_schema.columns WHERE table_schema = 'public' ORDER BY table_name, ordinal_position";
    let output = run_psql(database_url, query)?;
    let mut schema = parse_inspection_output(&output)?;
    let index_output = run_psql(
        database_url,
        "SELECT tablename, indexname, indexdef FROM pg_indexes WHERE schemaname = 'public' ORDER BY tablename, indexname",
    )?;
    parse_index_output(&mut schema, &index_output)?;
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
        if fields.len() != 6 {
            return Err(DatabaseError {
                message: format!("unexpected psql inspection row: {line}"),
            });
        }
        let table_name = fields[0].to_string();
        let sql_type = inspected_sql_type(fields[2], fields[3], fields[5]);
        let column = Column {
            name: fields[1].to_string(),
            sql_type,
            nullable: fields[4] == "YES",
            primary_key: false,
            auto: false,
            unique: false,
            default: None,
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
        if fields.len() != 3 {
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
        "SELECT TABLE_NAME, COLUMN_NAME, DATA_TYPE, IS_NULLABLE, COLUMN_KEY, EXTRA, COALESCE(CHARACTER_MAXIMUM_LENGTH, ''), COALESCE(NUMERIC_PRECISION, ''), COALESCE(NUMERIC_SCALE, '') FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() ORDER BY TABLE_NAME, ORDINAL_POSITION",
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
        "SELECT TABLE_NAME, COLUMN_NAME, REFERENCED_TABLE_NAME, REFERENCED_COLUMN_NAME FROM information_schema.KEY_COLUMN_USAGE WHERE TABLE_SCHEMA = DATABASE() AND REFERENCED_TABLE_NAME IS NOT NULL ORDER BY TABLE_NAME, CONSTRAINT_NAME, ORDINAL_POSITION",
        None,
    )?;
    parse_mariadb_foreign_keys(&mut schema, &foreign_keys)?;
    Ok(schema)
}

pub fn apply_mariadb(database_url: &str, sql: &str) -> Result<(), DatabaseError> {
    run_mariadb_sql(database_url, sql, None)
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
        let pragma = format!(
            "PRAGMA table_info({});",
            quote_identifier(table_name, Backend::Sqlite)
        );
        let output = run_sqlite(&path, &pragma)?;
        let columns = parse_sqlite_columns(&output)?;
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
        if fields.len() != 9 {
            return Err(DatabaseError {
                message: format!("unexpected MariaDB column row: {line}"),
            });
        }
        let sql_type = mariadb_sql_type(fields[2], fields[6], fields[7], fields[8]);
        let column = Column {
            name: fields[1].into(),
            sql_type,
            nullable: fields[3] == "YES",
            primary_key: fields[4] == "PRI",
            auto: fields[5].contains("auto_increment"),
            unique: fields[4] == "UNI",
            default: None,
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

fn parse_mariadb_foreign_keys(schema: &mut Schema, output: &str) -> Result<(), DatabaseError> {
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 4 {
            return Err(DatabaseError {
                message: format!("unexpected MariaDB foreign-key row: {line}"),
            });
        }
        if let Some(table) = schema
            .tables
            .iter_mut()
            .find(|table| table.name == fields[0])
        {
            table.foreign_keys.push(ForeignKey {
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

fn parse_sqlite_columns(output: &str) -> Result<Vec<Column>, DatabaseError> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 6 {
                return Err(DatabaseError {
                    message: format!("unexpected SQLite column row: {line}"),
                });
            }
            Ok(Column {
                name: fields[1].into(),
                sql_type: fields[2].to_ascii_uppercase(),
                nullable: fields[3] != "1",
                primary_key: fields[5] != "0",
                auto: fields[5] != "0" && fields[2].eq_ignore_ascii_case("INTEGER"),
                unique: false,
                default: None,
            })
        })
        .collect()
}

fn parse_sqlite_indexes(table: &mut Table, path: &str, output: &str) -> Result<(), DatabaseError> {
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() < 3 || fields[1].starts_with("sqlite_autoindex") {
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
            "machines\tid\tbigint\tint8\tNO\t\nmachines\tname\tcharacter varying\tvarchar\tYES\t100\n",
        )
        .unwrap();
        parse_index_output(
            &mut schema,
            "machines\tidx_machines_name\tCREATE INDEX idx_machines_name ON public.machines USING btree (name)\n",
        )
        .unwrap();
        assert_eq!(schema.tables[0].columns[1].sql_type, "VARCHAR(100)");
        assert_eq!(schema.tables[0].indexes[0].columns, ["name"]);
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
    fn defaults_new_schemas_to_mariadb() {
        let schema = schema("table users { id: Id primary auto name: String }");
        assert_eq!(schema.backend(), Backend::MariaDb);
        assert!(schema.create_sql().contains("`id` BIGINT"));
    }

    #[test]
    fn parses_sqlite_columns_foreign_keys_and_indexes() {
        let columns =
            parse_sqlite_columns("0\tid\tINTEGER\t1\t\t1\n1\tdepartment_id\tINTEGER\t1\t\t0\n")
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
    fn parses_mariadb_foreign_keys() {
        let mut schema =
            parse_mariadb_columns("machines\tid\tbigint\tNO\tPRI\tauto_increment\t\t\t\n").unwrap();
        parse_mariadb_foreign_keys(&mut schema, "machines\tdepartment_id\tdepartments\tid\n")
            .unwrap();
        assert_eq!(schema.tables[0].foreign_keys[0].column, "department_id");
    }
}
