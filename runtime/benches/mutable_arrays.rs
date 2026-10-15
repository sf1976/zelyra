use std::hint::black_box;
use std::time::{Duration, Instant};
use zelyra_ast::Program;
use zelyra_lexer::lex;
use zelyra_parser::parse;
use zelyra_runtime::{check, execute};

const APPEND_SOURCE: &str = r#"
fn main() {
    mutable values: Int[] = []
    for value in 0..5000 {
        values = append(values, value)
    }
    print(len(values))
}
"#;

const PUSH_SOURCE: &str = r#"
fn main() {
    mutable values: Int[] = []
    for value in 0..5000 {
        push_mut(values, value)
    }
    print(len(values))
}
"#;

const INDEX_SOURCE: &str = r#"
fn main() {
    mutable values: Int[] = range(0, 5000)
    for position in 0..100000 {
        values[position % 5000] += 1
    }
    print(len(values))
}
"#;

fn compile(source: &str) -> Program {
    let tokens = lex(source).expect("benchmark source should lex");
    let program = parse(&tokens).expect("benchmark source should parse");
    check(&program).expect("benchmark source should type check");
    program
}

fn median(mut samples: Vec<Duration>) -> Duration {
    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn measure(program: &Program, iterations: usize, expected: &[&str]) -> Duration {
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        let output = execute(black_box(program)).expect("benchmark run should succeed");
        samples.push(started.elapsed());
        assert_eq!(output, expected);
        black_box(output);
    }
    median(samples)
}

fn main() {
    let append = compile(APPEND_SOURCE);
    let push = compile(PUSH_SOURCE);
    let index = compile(INDEX_SOURCE);
    let iterations = 7;

    println!(
        "Zelyra AST-runtime array benchmark; 5,000 values, 100,000 indexed updates, {iterations} samples, median"
    );
    println!(
        "append (copy-on-write): {:?}",
        measure(&append, iterations, &["5000"])
    );
    println!(
        "push_mut (in-place):    {:?}",
        measure(&push, iterations, &["5000"])
    );
    println!(
        "indexed += (in-place):  {:?}",
        measure(&index, iterations, &["5000"])
    );
}
