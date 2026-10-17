use std::{env, fs, process};

fn main() {
    let Some(output) = env::args_os().nth(1) else {
        eprintln!("usage: cargo run --example bytecode_seed -- <output-path>");
        process::exit(2);
    };

    let source = "fn main() -> Int { return 7 }";
    let tokens = zelyra_lexer::lex(source).unwrap_or_else(|error| fail(error));
    let program = zelyra_parser::parse(&tokens).unwrap_or_else(|error| fail(error));
    let mir = zelyra_mir::lower(&program).unwrap_or_else(|errors| fail(errors));
    let bytecode =
        zelyra_mir::bytecode::compile(&mir, "main").unwrap_or_else(|errors| fail(errors));
    let artifact = zelyra_mir::bytecode::encode(&bytecode).unwrap_or_else(|errors| fail(errors));
    fs::write(output, artifact).unwrap_or_else(|error| fail(error));
}

fn fail(error: impl std::fmt::Debug) -> ! {
    eprintln!("could not generate bytecode fuzz seed: {error:?}");
    process::exit(1);
}
