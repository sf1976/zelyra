use std::{env, fs, process::ExitCode};
use zelyra_database::{apply_postgres, build_schema, diff, inspect_postgres, Risk, Schema};
use zelyra_hir::lower;
use zelyra_lexer::lex;
use zelyra_parser::parse;
use zelyra_runtime::{check, execute};

fn usage() {
    eprintln!("Zelyra 0.1\n\nUsage:\n  zelyra new <directory>\n  zelyra init [directory]\n  zelyra check <file.zyl>\n  zelyra build <file.zyl>\n  zelyra run <file.zyl>\n  zelyra db <create|inspect|plan|apply> <file.zyl>");
}

fn database_usage() {
    eprintln!(
        "Usage:\n  zelyra db create <file.zyl>\n  zelyra db inspect <file.zyl>\n  zelyra db plan <file.zyl>\n  zelyra db apply <file.zyl> [--allow-destructive]\n\nDATABASE_URL is used by inspect and apply."
    );
}

fn create_project(path: &str, allow_current_directory: bool) -> ExitCode {
    let directory = std::path::Path::new(path);
    if directory.exists() && !allow_current_directory {
        eprintln!("error[E-INIT-001]: directory `{path}` already exists");
        return ExitCode::from(1);
    }
    if let Err(error) = fs::create_dir_all(directory) {
        eprintln!("error[E-INIT-002]: cannot create `{path}`: {error}");
        return ExitCode::from(1);
    }
    let files = [
        (
            "zelyra.toml",
            "[project]\nname = \"zelyra-app\"\nversion = \"0.1.0\"\nzelyra = \"0.1\"\n",
        ),
        (
            "main.zyl",
            "fn main() {\n    print(\"Hello from Zelyra\")\n}\n",
        ),
    ];
    for (name, contents) in files {
        let file = directory.join(name);
        if file.exists() && allow_current_directory {
            continue;
        }
        if let Err(error) = fs::write(&file, contents) {
            eprintln!(
                "error[E-INIT-003]: cannot write `{}`: {error}",
                file.display()
            );
            return ExitCode::from(1);
        }
    }
    println!("created Zelyra project in {}", directory.display());
    println!("next: cd {} && zelyra run main.zyl", path);
    ExitCode::SUCCESS
}

fn diagnostic(path: &str, code: &str, message: &str, line: usize, column: usize) {
    eprintln!("error[{code}]: {message}\n\n --> {path}:{line}:{column}");
}

fn load(path: &str) -> Result<zelyra_ast::Program, ()> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error[E-IO-001]: cannot read `{path}`: {error}");
            return Err(());
        }
    };
    let tokens = match lex(&source) {
        Ok(tokens) => tokens,
        Err(error) => {
            diagnostic(
                path,
                "E-LEX-001",
                &error.message,
                error.span.line,
                error.span.column,
            );
            return Err(());
        }
    };
    match parse(&tokens) {
        Ok(program) => Ok(program),
        Err(error) => {
            diagnostic(
                path,
                "E-PARSE-001",
                &error.message,
                error.span.line,
                error.span.column,
            );
            Err(())
        }
    }
}

fn validate(path: &str) -> Result<zelyra_ast::Program, ()> {
    let program = load(path)?;
    if let Err(errors) = lower(&program) {
        for error in errors {
            diagnostic(
                path,
                "E-NAME-001",
                &error.message,
                error.span.line,
                error.span.column,
            );
        }
        return Err(());
    }
    if let Err(errors) = check(&program) {
        for error in errors {
            diagnostic(
                path,
                "E-TYPE-001",
                &error.message,
                error.span.line,
                error.span.column,
            );
        }
        return Err(());
    }
    Ok(program)
}

fn load_schema(path: &str) -> Result<Schema, ()> {
    let program = load(path)?;
    match build_schema(&program) {
        Ok(schema) => Ok(schema),
        Err(errors) => {
            for error in errors {
                diagnostic(
                    path,
                    "E-DB-001",
                    &error.message,
                    error.span.line,
                    error.span.column,
                );
            }
            Err(())
        }
    }
}

fn print_plan(plan: &zelyra_database::SchemaPlan) {
    if plan.changes.is_empty() {
        println!("No schema changes.");
        return;
    }
    for change in &plan.changes {
        let risk = match change.risk {
            Risk::Safe => "SAFE",
            Risk::Destructive => "DESTRUCTIVE",
        };
        println!("[{risk}] {}\n{}\n", change.description, change.sql);
    }
}

fn database_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(subcommand) = args.next() else {
        database_usage();
        return ExitCode::from(2);
    };
    let path = args.next().unwrap_or_else(|| "main.zyl".into());
    let allow_destructive = args.any(|arg| arg == "--allow-destructive");
    let schema = match load_schema(&path) {
        Ok(schema) => schema,
        Err(()) => return ExitCode::from(1),
    };
    match subcommand.as_str() {
        "create" => {
            println!("{}", schema.create_sql());
            ExitCode::SUCCESS
        }
        "inspect" => match env::var("DATABASE_URL") {
            Ok(url) => match inspect_postgres(&url) {
                Ok(current) => {
                    println!("{}", current.summary());
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("error[E-DB-002]: {error}");
                    ExitCode::from(1)
                }
            },
            Err(_) => {
                eprintln!("error[E-DB-003]: DATABASE_URL is required for db inspect");
                ExitCode::from(1)
            }
        },
        "plan" => {
            let current = match env::var("DATABASE_URL") {
                Ok(url) => match inspect_postgres(&url) {
                    Ok(current) => current,
                    Err(error) => {
                        eprintln!("error[E-DB-002]: {error}");
                        return ExitCode::from(1);
                    }
                },
                Err(_) => {
                    eprintln!("note: DATABASE_URL is not set; planning against an empty database");
                    Schema {
                        database: None,
                        tables: Vec::new(),
                    }
                }
            };
            print_plan(&diff(&schema, &current));
            ExitCode::SUCCESS
        }
        "apply" => {
            let Ok(url) = env::var("DATABASE_URL") else {
                eprintln!("error[E-DB-003]: DATABASE_URL is required for db apply");
                return ExitCode::from(1);
            };
            let current = match inspect_postgres(&url) {
                Ok(current) => current,
                Err(error) => {
                    eprintln!("error[E-DB-002]: {error}");
                    return ExitCode::from(1);
                }
            };
            let plan = diff(&schema, &current);
            print_plan(&plan);
            if plan.is_destructive() && !allow_destructive {
                eprintln!("error[E-DB-004]: destructive changes refused; use --allow-destructive after review");
                return ExitCode::from(1);
            }
            if plan.changes.is_empty() {
                return ExitCode::SUCCESS;
            }
            match apply_postgres(&url, &plan.sql()) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("error[E-DB-005]: {error}");
                    ExitCode::from(1)
                }
            }
        }
        _ => {
            database_usage();
            ExitCode::from(2)
        }
    }
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        usage();
        return ExitCode::from(2);
    };
    if command == "--help" || command == "-h" {
        usage();
        return ExitCode::SUCCESS;
    }
    if command == "db" {
        return database_command(args);
    }
    if command == "new" {
        let Some(path) = args.next() else {
            usage();
            return ExitCode::from(2);
        };
        if args.next().is_some() {
            usage();
            return ExitCode::from(2);
        }
        return create_project(&path, false);
    }
    if command == "init" {
        let path = args.next().unwrap_or_else(|| ".".into());
        if args.next().is_some() {
            usage();
            return ExitCode::from(2);
        }
        return create_project(&path, true);
    }
    let Some(path) = args.next() else {
        usage();
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        usage();
        return ExitCode::from(2);
    }
    match command.as_str() {
        "check" | "build" => {
            if validate(&path).is_ok() {
                println!("ok: {path}");
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        "run" => match validate(&path).and_then(|program| {
            execute(&program).map_err(|error| {
                diagnostic(
                    &path,
                    "E-RUNTIME-001",
                    &error.message,
                    error.span.line,
                    error.span.column,
                );
            })
        }) {
            Ok(output) => {
                for line in output {
                    println!("{line}");
                }
                ExitCode::SUCCESS
            }
            Err(()) => ExitCode::from(1),
        },
        _ => {
            usage();
            ExitCode::from(2)
        }
    }
}
