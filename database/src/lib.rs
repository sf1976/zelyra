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
    if let Some(database) = program.databases.first() {
        if !database.engine.eq_ignore_ascii_case("postgres") {
            errors.push(SchemaError {
                message: format!(
                    "unsupported database engine {}; Phase 3 supports postgres",
                    database.engine
                ),
                span: database.span,
            });
        }
    }
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
                column_mapping(definition, &table_names, &program.types);
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
) -> (String, String, Option<String>) {
    if let Type::Named(name) = &definition.ty {
        if let Some(table) = table_for_type(name, table_names) {
            return (
                format!("{}_id", definition.name),
                "BIGINT".into(),
                Some(table),
            );
        }
    }
    let resolved = resolve_type(&definition.ty, type_definitions);
    let sql_type = match resolved {
        Type::Int => {
            if definition.auto {
                "BIGSERIAL"
            } else {
                "BIGINT"
            }
        }
        Type::UInt => "NUMERIC(20,0)",
        Type::Float => "DOUBLE PRECISION",
        Type::Decimal => "NUMERIC",
        Type::Bool => "BOOLEAN",
        Type::String => match definition.length {
            Some(length) => return (definition.name.clone(), format!("VARCHAR({length})"), None),
            None => "TEXT",
        },
        Type::Char => "CHAR(1)",
        Type::Bytes => "BYTEA",
        Type::Timestamp => "TIMESTAMPTZ",
        Type::Date => "DATE",
        Type::Time => "TIME",
        Type::Duration => "BIGINT",
        Type::Named(ref name) if name == "Id" => {
            if definition.auto {
                "BIGSERIAL"
            } else {
                "BIGINT"
            }
        }
        Type::Named(ref name) if name == "Email" => "VARCHAR(255)",
        Type::Named(ref name) if name == "Url" => "VARCHAR(2048)",
        Type::Named(ref name) if name == "Uuid" => "UUID",
        Type::Named(ref name) if name == "Money" => "NUMERIC(19,4)",
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

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn quote_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

impl Schema {
    pub fn create_sql(&self) -> String {
        let mut statements = Vec::new();
        for table in &self.tables {
            statements.push(table_create_sql(table));
            for index in &table.indexes {
                statements.push(index_sql(table, index));
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

fn table_create_sql(table: &Table) -> String {
    let mut definitions = table.columns.iter().map(column_sql).collect::<Vec<_>>();
    definitions.extend(table.uniques.iter().map(|index| {
        let columns = index
            .columns
            .iter()
            .map(|column| quote_identifier(column))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "CONSTRAINT {} UNIQUE ({columns})",
            quote_identifier(&index.name)
        )
    }));
    definitions.extend(table.foreign_keys.iter().map(|foreign_key| {
        format!(
            "CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} ({})",
            quote_identifier(&format!("fk_{}_{}", table.name, foreign_key.column)),
            quote_identifier(&foreign_key.column),
            quote_identifier(&foreign_key.referenced_table),
            quote_identifier(&foreign_key.referenced_column)
        )
    }));
    format!(
        "CREATE TABLE IF NOT EXISTS {} (\n    {}\n);",
        quote_identifier(&table.name),
        definitions.join(",\n    ")
    )
}

fn index_sql(table: &Table, index: &Index) -> String {
    let columns = index
        .columns
        .iter()
        .map(|column| quote_identifier(column))
        .collect::<Vec<_>>()
        .join(", ");
    let unique = if index.unique { " UNIQUE" } else { "" };
    format!(
        "CREATE{unique} INDEX IF NOT EXISTS {} ON {} ({columns});",
        quote_identifier(&index.name),
        quote_identifier(&table.name)
    )
}

fn column_sql(column: &Column) -> String {
    let mut sql = format!("{} {}", quote_identifier(&column.name), column.sql_type);
    if column.primary_key {
        sql.push_str(" PRIMARY KEY");
    }
    if !column.nullable {
        sql.push_str(" NOT NULL");
    }
    if column.unique {
        sql.push_str(" UNIQUE");
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
                sql: table_create_sql(desired_table),
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
                        quote_identifier(&desired_table.name),
                        column_sql(desired_column)
                    ),
                    risk: Risk::Safe,
                }),
                Some(current_column) if current_column.sql_type != desired_column.sql_type => {
                    plan.changes.push(SchemaChange {
                        description: format!(
                            "change type of {}.{}",
                            desired_table.name, desired_column.name
                        ),
                        sql: format!(
                            "ALTER TABLE {} ALTER COLUMN {} TYPE {};",
                            quote_identifier(&desired_table.name),
                            quote_identifier(&desired_column.name),
                            desired_column.sql_type
                        ),
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
                        quote_identifier(&desired_table.name),
                        quote_identifier(&current_column.name)
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
                    sql: index_sql(desired_table, index),
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
                sql: format!("DROP TABLE {};", quote_identifier(&current_table.name)),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseError {
    pub message: String,
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
}
