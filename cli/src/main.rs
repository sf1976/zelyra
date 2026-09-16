use std::{env, fs, process::ExitCode};
use zelyra_lexer::lex;
use zelyra_parser::parse;
use zelyra_runtime::{check, execute};

fn usage() {
    eprintln!("Zelyra 0.1\n\nUsage:\n  zelyra new <directory>\n  zelyra init [directory]\n  zelyra check <file.zyl>\n  zelyra build <file.zyl>\n  zelyra run <file.zyl>");
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
                ()
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
