use std::hint::black_box;
use std::time::{Duration, Instant};
use zelyra_mir::bytecode;
use zelyra_mir::execution::ExecutionLimits;
use zelyra_runtime::{check, execute_function, Value};

const SOURCE: &str = r#"
fn main() -> Int {
    mutable values: Int[] = range(0, 20000)
    for position in 0..100000 {
        values[position % 20000] += 1
    }
    return values[0] + values[19999]
}
"#;

const SAMPLES: usize = 9;
const LIMITS: ExecutionLimits = ExecutionLimits {
    max_steps: 5_000_000,
    max_stack_depth: 64,
};

fn summarize(mut samples: Vec<Duration>) -> (Duration, Duration, Duration) {
    samples.sort_unstable();
    (
        samples[0],
        samples[samples.len() / 2],
        samples[samples.len() - 1],
    )
}

fn sample(run: &mut impl FnMut() -> Value, expected: &Value) -> Duration {
    let started = Instant::now();
    let value = run();
    let elapsed = started.elapsed();
    assert_eq!(&value, expected, "backend result changed during benchmark");
    black_box(value);
    elapsed
}

fn main() {
    let tokens = zelyra_lexer::lex(SOURCE).expect("benchmark source should lex");
    let program = zelyra_parser::parse(&tokens).expect("benchmark source should parse");
    check(&program).expect("benchmark source should type-check");
    let mir = zelyra_mir::lower(&program).expect("benchmark source should lower to MIR");
    let bytecode = bytecode::compile(&mir, "main").expect("benchmark source should compile");
    let expected = execute_function(&program, "main", Vec::new(), None)
        .expect("AST benchmark execution should succeed");

    let mut ast_run =
        || execute_function(&program, "main", Vec::new(), None).expect("AST execution");
    let mut mir_run = || {
        zelyra_mir::execution::execute_function(&mir, "main", Vec::new(), LIMITS)
            .expect("MIR execution")
            .value
    };
    let mut bytecode_run = || {
        bytecode::execute(&bytecode, Vec::new(), LIMITS)
            .expect("Bytecode execution")
            .value
    };
    sample(&mut ast_run, &expected);
    sample(&mut mir_run, &expected);
    sample(&mut bytecode_run, &expected);
    let mut ast_samples = Vec::with_capacity(SAMPLES);
    let mut mir_samples = Vec::with_capacity(SAMPLES);
    let mut bytecode_samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        ast_samples.push(sample(&mut ast_run, &expected));
        mir_samples.push(sample(&mut mir_run, &expected));
        bytecode_samples.push(sample(&mut bytecode_run, &expected));
    }

    println!("Zelyra mutable-array backend benchmark");
    println!("workload: 20,000 Int values; 100,000 indexed += updates");
    println!("samples: {SAMPLES} per backend after one warm-up; statistic: min / median / max; result: {expected:?}");
    println!("AST reference: {:?}", summarize(ast_samples));
    println!("MIR:           {:?}", summarize(mir_samples));
    println!("Bytecode:      {:?}", summarize(bytecode_samples));
}
