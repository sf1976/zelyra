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

#[test]
fn supports_option_match_and_pattern_bindings() {
    let source = r#"
        fn lookup(flag: Bool) -> Int? {
            if flag {
                return Some(42)
            }
            return None
        }

        fn main() {
            result = lookup(true)
            match result {
                Some(value) => {
                    print(value)
                }
                None => {
                    print(0)
                }
            }
        }
    "#;
    assert_eq!(run(source), vec!["42"]);
}

#[test]
fn supports_result_match() {
    let source = r#"
        fn load(ok: Bool) -> Result<Int, String> {
            if ok {
                return Ok(7)
            }
            return Err("failed")
        }

        fn main() {
            match load(false) {
                Ok(value) => { print(value) }
                Err(message) => { print(message) }
            }
        }
    "#;
    assert_eq!(run(source), vec!["failed"]);
}

#[test]
fn rejects_non_exhaustive_option_match() {
    let source =
        "fn main() { value: Int? = None match value { Some(number) => { print(number) } } }";
    let program = parse(&lex(source).unwrap()).unwrap();
    let errors = check(&program).expect_err("missing None arm must be rejected");
    assert!(errors
        .iter()
        .any(|error| error.message.contains("non-exhaustive match")));
}

#[test]
fn keeps_nominal_types_distinct() {
    let source = "type UserId = Id type OrderId = Id fn load_user(id: UserId) -> Int { return 1 } fn main() { load_user(1) }";
    let program = parse(&lex(source).unwrap()).unwrap();
    let errors = check(&program).expect_err("raw Int must not become UserId implicitly");
    assert!(errors
        .iter()
        .any(|error| error.message.contains("expected `UserId`")));
}
