use zelyra_lexer::lex;
use zelyra_parser::parse;
use zelyra_runtime::{check, execute};

fn run(source: &str) -> Vec<String> {
    let program =
        parse(&lex(source).expect("lexing should succeed")).expect("parsing should succeed");
    check(&program).expect("type checking should succeed");
    execute(&program).expect("execution should succeed")
}

#[test]
fn accepts_the_specification_fibonacci_program() {
    let source = r#"
        fn fibonacci(n: Int) -> Int {
            if n <= 1 {
                return n
            }
            return fibonacci(n - 1) + fibonacci(n - 2)
        }

        fn main() {
            print(fibonacci(10))
        }
    "#;
    assert_eq!(run(source), vec!["55"]);
}

#[test]
fn supports_strings_booleans_and_if_else() {
    let source = r#"
        fn main() {
            greeting: String = "Hello"
            if true {
                print(greeting + " from Zelyra")
            } else {
                print("unreachable")
            }
        }
    "#;
    assert_eq!(run(source), vec!["Hello from Zelyra"]);
}

#[test]
fn reports_unknown_variables() {
    let program = parse(&lex("fn main() { print(missing) }").unwrap()).unwrap();
    let errors = check(&program).expect_err("unknown variable must be rejected");
    assert!(errors
        .iter()
        .any(|error| error.message.contains("unknown variable `missing`")));
}
