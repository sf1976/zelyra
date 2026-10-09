use super::*;

use std::net::TcpListener;
use std::thread;
use zelyra_lexer::lex;
use zelyra_parser::parse;

fn run(source: &str) -> Vec<String> {
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    execute(&program).unwrap()
}

#[test]
fn fibonacci_acceptance_test() {
    let output = run("fn fibonacci(n: Int) -> Int { if n <= 1 { return n } return fibonacci(n - 1) + fibonacci(n - 2) } fn main() { print(fibonacci(10)) }");
    assert_eq!(output, ["55"]);
}

#[test]
fn mutable_while_loop_works() {
    let output = run("fn main() { mutable i = 0 while i < 3 { print(i) i = i + 1 } }");
    assert_eq!(output, ["0", "1", "2"]);
}

#[test]
fn supports_array_literals_indexing_length_append_and_concatenation() {
    let output = run(
            "fn main() { values = [1, 2] extended = append(values, 3) combined = extended + [4] print(combined[2]) print(len(combined)) }",
        );
    assert_eq!(output, ["3", "4"]);
}

#[test]
fn supports_array_for_iteration_and_loop_control() {
    let output = run(
            "fn main() { mutable total = 0 for value in [1, 2, 3, 4] { if value == 2 { continue } if value == 4 { break } total = total + value } print(total) }",
        );
    assert_eq!(output, ["4"]);
}

#[test]
fn executes_parallel_await_bindings_and_merges_results() {
    let output = run(
            "fn load_customer() -> String { return \"customer\" } fn load_orders() -> Int { return 3 } fn main() { parallel { customer = await load_customer() orders = await load_orders() } print(customer) print(orders) }",
        );
    assert_eq!(output, ["customer", "3"]);
}

#[test]
fn rejects_await_outside_parallel_block() {
    let program =
        parse(&lex("fn load() -> Int { return 1 } fn main() { value = await load() }").unwrap())
            .unwrap();
    let errors = check(&program).unwrap_err();
    assert!(errors.iter().any(|error| error
        .message
        .contains("only valid inside a `parallel` block")));
}

#[test]
fn rejects_parallel_bindings_without_await() {
    let program = parse(
        &lex("fn load() -> Int { return 1 } fn main() { parallel { value = load() } }").unwrap(),
    )
    .unwrap();
    let errors = check(&program).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("must await their expression")));
}

#[test]
fn rejects_non_array_for_iteration() {
    let program = parse(&lex("fn main() { for value in 1 { print(value) } }").unwrap()).unwrap();
    let errors = check(&program).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("expects an array")));
}

#[test]
fn supports_record_literals_field_access_and_array_queries() {
    let output = run(
            "struct Address { city: String } struct Customer { name: String address: Address } fn main() { customer = Customer { name: \"Anna\", address: Address { city: \"Berlin\" } } print(customer.address.city) print(contains([1, 2, 3], 2)) print(first([4, 5])) print(last([4, 5])) }",
        );
    assert_eq!(output, ["Berlin", "true", "Some(4)", "Some(5)"]);
}

#[test]
fn supports_typed_maps_get_put_keys_values_and_json() {
    let output = run(r#"fn main() {
                values: Map<String, Int> = Map { "one": 1, "two": 2 }
                mutable updated = put(values, "two", 20)
                updated = put(updated, "three", 3)
                print(get(updated, "two"))
                print(get(updated, "missing"))
                print(contains(updated, "three"))
                print(len(updated))
                print(keys(updated))
                print(values(updated))
                encoded = json_encode(updated)
                decoded = json_decode<Map<String, Int>>(encoded)
                print(decoded)
            }"#);
    assert_eq!(
        output,
        [
            "Some(20)",
            "None",
            "true",
            "3",
            "[one, two, three]",
            "[1, 20, 3]",
            "Map{one=1, three=3, two=20}",
        ]
    );
}

#[test]
fn rejects_incompatible_map_entries_and_keys() {
    let program =
        parse(&lex("fn main() { values = Map { \"one\": 1, \"two\": \"not an Int\" } }").unwrap())
            .unwrap();
    let errors = check(&program).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("type mismatch")));

    let program = parse(
        &lex("struct Key { value: Int } fn main() { values: Map<Key, Int> = Map {} }").unwrap(),
    )
    .unwrap();
    let errors = check(&program).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("Map keys")));
}

#[test]
fn map_literals_keep_the_last_value_for_duplicate_keys() {
    let output = run(
            "fn main() { values: Map<String, Int> = Map { \"same\": 1, \"same\": 2 } print(get(values, \"same\")) print(keys(values)) }",
        );
    assert_eq!(output, ["Some(2)", "[same]"]);
}

#[test]
fn rejects_unknown_record_fields() {
    let program = parse(
        &lex("struct Address { city: String } fn main() { address = Address { country: \"DE\" } }")
            .unwrap(),
    )
    .unwrap();
    let errors = check(&program).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("unknown field `country`")));
}

#[test]
fn rejects_array_index_out_of_bounds_at_runtime() {
    let program = parse(&lex("fn main() { values = [1] print(values[1]) }").unwrap()).unwrap();
    check(&program).unwrap();
    let error = execute(&program).unwrap_err();
    assert!(error.message.contains("out of bounds"));
}

#[test]
fn loop_continue_skips_remaining_body() {
    let output = run(
            "fn main() { mutable i = 0 mutable sum = 0 while i < 5 { i = i + 1 if i == 3 { continue } sum = sum + i } print(sum) }",
        );
    assert_eq!(output, ["12"]);
}

#[test]
fn loop_break_exits_the_current_loop() {
    let output = run("fn main() { mutable i = 0 while i < 5 { i = i + 1 break } print(i) }");
    assert_eq!(output, ["1"]);
}

#[test]
fn rejects_loop_continue_outside_a_loop() {
    let program = parse(&lex("fn main() { continue }").unwrap()).unwrap();
    let errors = check(&program).unwrap_err();
    assert!(errors.iter().any(|error| error
        .message
        .contains("`continue` is only valid inside a loop")));
}

#[test]
fn checks_loop_invariants_at_runtime() {
    let output =
        run("fn main() { mutable i = 0 while i < 2 invariant { i >= 0 } { i = i + 1 } print(i) }");
    assert_eq!(output, ["2"]);

    let program = parse(
        &lex("fn main() { mutable i = 0 while i < 1 invariant { i > 0 } { i = i + 1 } }").unwrap(),
    )
    .unwrap();
    check(&program).unwrap();
    let error = execute(&program).unwrap_err();
    assert!(error.message.contains("loop invariant failed"));
}

#[test]
fn checks_invariants_on_unconditional_loops() {
    let output = run(
            "fn main() { mutable i = 0 loop invariant { i >= 0 } { i = i + 1 if i == 2 { break } } print(i) }",
        );
    assert_eq!(output, ["2"]);
}

#[test]
fn rejects_non_boolean_loop_invariants() {
    let program =
        parse(&lex("fn main() { while true invariant { 1 } { break } }").unwrap()).unwrap();
    let errors = check(&program).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("expected `Bool`")));
}

#[test]
fn supports_float_comparisons_and_inferred_returns() {
    let output = run(
            "fn twice(value: Float) { return value * 2.0 } fn main() { if twice(1.5) < 4.0 { print(twice(1.5)) } }",
        );
    assert_eq!(output, ["3"]);
}

#[test]
fn rejects_immutable_assignment() {
    let program = parse(&lex("fn main() { value = 1 value = 2 }").unwrap()).unwrap();
    let errors = check(&program).unwrap_err();
    assert!(errors.iter().any(|e| e.message.contains("immutable")));
}

#[test]
fn accepts_declared_database_capability_for_sql() {
    let program = parse(
            &lex("fn load() uses Database { rows = sql<Int> { SELECT 1 } } fn main() uses Database { load() }").unwrap(),
        )
        .unwrap();
    check_capabilities(&program).unwrap();
}

#[test]
fn scopes_sql_to_read_or_write_database_effects() {
    let read = parse(
        &lex("fn report() uses Database(read) { rows = sql<Int> { SELECT 1 } } fn main() { }")
            .unwrap(),
    )
    .unwrap();
    check_capabilities(&read).unwrap();

    let denied_read = parse(
        &lex("fn report() uses Database(read) { rows = sql<Int> { UPDATE customers SET id = 1 } } fn main() { }")
            .unwrap(),
    )
    .unwrap();
    let errors = check_capabilities(&denied_read).unwrap_err();
    assert!(errors.iter().any(|error| error
        .message
        .contains("does not declare capability `Database(write)`")));

    let denied_write = parse(
        &lex("fn report() uses Database(write) { rows = sql<Int> { SELECT 1 } } fn main() { }")
            .unwrap(),
    )
    .unwrap();
    let errors = check_capabilities(&denied_write).unwrap_err();
    assert!(
        errors.iter().any(|error| error
            .message
            .contains("does not declare capability `Database(read)`")),
        "{errors:#?}"
    );
}

#[test]
fn scoped_database_effects_require_matching_project_grants() {
    let program = parse(
        &lex("fn report() uses Database(read) { rows = sql<Int> { SELECT 1 } } fn main() { }")
            .unwrap(),
    )
    .unwrap();
    let grants = HashSet::from([String::from("Database(write)")]);
    let errors = check_capabilities_with_grants(&program, Some(&grants)).unwrap_err();
    assert!(errors.iter().any(|error| {
        error
            .message
            .contains("set `database_read` to true in zelyra.toml")
    }));

    let legacy_grants = HashSet::from([String::from("Database")]);
    check_capabilities_with_grants(&program, Some(&legacy_grants)).unwrap();
}

#[test]
fn rejects_sql_without_database_capability() {
    let program = parse(&lex("fn main() { rows = sql<Int> { SELECT 1 } }").unwrap()).unwrap();
    let errors = check_capabilities(&program).unwrap_err();
    assert!(errors.iter().any(|error| {
        error
            .message
            .contains("does not declare capability `Database(read)`")
    }));
}

#[test]
fn propagates_called_function_capabilities() {
    let program =
        parse(&lex("fn send() uses Network { print(1) } fn main() { send() }").unwrap()).unwrap();
    let errors = check_capabilities(&program).unwrap_err();
    assert!(errors.iter().any(|error| {
        error
            .message
            .contains("calls `send`, which requires capability `Network`")
    }));
}

#[test]
fn rejects_unknown_and_duplicate_capabilities() {
    let program =
        parse(&lex("fn main() uses Network, Network, Telepathy { print(1) }").unwrap()).unwrap();
    let errors = check_capabilities(&program).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("declared more than once")));
    assert!(errors
        .iter()
        .any(|error| error.message.contains("unknown capability `Telepathy`")));
}

#[test]
fn rejects_capability_not_granted_by_project() {
    let program = parse(&lex("fn main() uses Network { print(1) }").unwrap()).unwrap();
    let grants = HashSet::new();
    let errors = check_capabilities_with_grants(&program, Some(&grants)).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("not enabled by the project")));
}

#[test]
fn enforces_runtime_capability_for_called_function() {
    let program =
        parse(&lex("fn send() uses Network { print(1) } fn main() { send() }").unwrap()).unwrap();
    let grants = HashSet::new();
    let error = execute_with_capabilities(&program, Some(&grants)).unwrap_err();
    assert!(error.message.contains("function `send` requires `Network`"));
}

#[test]
fn enforces_runtime_database_capability_before_connecting() {
    let program = parse(&lex("fn main() { rows = sql<Int> { SELECT 1 } }").unwrap()).unwrap();
    let grants = HashSet::new();
    let error = execute_with_database_and_capabilities(
        &program,
        "mariadb://invalid:invalid@127.0.0.1:1/invalid",
        Some(&grants),
    )
    .unwrap_err();
    assert!(error
        .message
        .contains("SQL access requires `Database(read)` in the current function"));
}

#[test]
fn enforces_scoped_database_grants_before_connecting() {
    let program =
        parse(&lex("fn main() uses Database(read) { rows = sql<Int> { SELECT 1 } }").unwrap())
            .unwrap();
    let grants = HashSet::from([String::from("Database(write)")]);
    let error = execute_with_database_and_capabilities(
        &program,
        "mariadb://invalid:invalid@127.0.0.1:1/invalid",
        Some(&grants),
    )
    .unwrap_err();
    assert!(error
        .message
        .contains("function `main` requires `Database(read)`"));
}

#[test]
fn accepts_runtime_capability_grant() {
    let program = parse(&lex("fn main() uses Network { print(1) }").unwrap()).unwrap();
    let grants = HashSet::from([String::from("Network")]);
    assert_eq!(
        execute_with_capabilities(&program, Some(&grants)).unwrap(),
        ["1"]
    );
}

#[test]
fn requires_network_capability_for_http_get() {
    let program =
        parse(&lex("fn main() { print(http_get(\"http://example.test\")) }").unwrap()).unwrap();
    let errors = check_capabilities(&program).unwrap_err();
    assert!(errors.iter().any(|error| error
        .message
        .contains("does not declare capability `Network`")));
}

#[test]
fn requires_network_capability_for_http_json() {
    let source = "struct Customer { name: String } fn main() { customer = http_json<Customer, Customer>(\"POST\", \"http://example.test\", [], None) }";
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    let errors = check_capabilities(&program).unwrap_err();
    assert!(errors.iter().any(|error| error
        .message
        .contains("does not declare capability `Network`")));
}

#[test]
fn performs_http_get_with_network_policy() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request).unwrap();
        std::io::Write::write_all(
            &mut stream,
            b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
        )
        .unwrap();
    });
    let source =
        "fn fetch(url: String) -> String uses Network { return http_get(url) } fn main() { }";
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();
    let grants = HashSet::from([String::from("Network")]);
    let policy = RuntimePolicy {
        filesystem: None,
        network: Some(NetworkPolicy {
            allowed_hosts: vec![format!("127.0.0.1:{port}")],
            timeout_ms: 1_000,
            max_response_bytes: 100,
        }),
        process: None,
    };
    let body = execute_function_with_capabilities_and_policies(
        &program,
        "fetch",
        vec![Value::String(format!("http://127.0.0.1:{port}/health"))],
        None,
        Some(&grants),
        Some(&policy),
    )
    .unwrap();
    assert_eq!(body, Value::String("hello".into()));
    server.join().unwrap();
}

#[test]
fn rejects_http_get_outside_network_allowlist() {
    let source =
            "fn fetch() -> String uses Network { return http_get(\"http://example.test\") } fn main() { }";
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    let grants = HashSet::from([String::from("Network")]);
    let policy = RuntimePolicy {
        filesystem: None,
        network: Some(NetworkPolicy {
            allowed_hosts: Vec::new(),
            timeout_ms: 100,
            max_response_bytes: 100,
        }),
        process: None,
    };
    let error = execute_function_with_capabilities_and_policies(
        &program,
        "fetch",
        Vec::new(),
        None,
        Some(&grants),
        Some(&policy),
    )
    .unwrap_err();
    assert!(error.message.contains("network access denied"));
}

#[test]
fn sends_typed_http_request_and_returns_structured_response() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        let mut chunk = [0_u8; 512];
        while !request
            .windows(b"{\"name\":\"Anna\"}".len())
            .any(|window| window == b"{\"name\":\"Anna\"}")
        {
            let size = stream.read(&mut chunk).unwrap();
            if size == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..size]);
        }
        let request = String::from_utf8_lossy(&request);
        let request_lower = request.to_ascii_lowercase();
        assert!(request.starts_with("POST /customers HTTP/1.1"));
        assert!(request_lower.contains("x-request-id: test-1"));
        assert!(request.contains("{\"name\":\"Anna\"}"));
        std::io::Write::write_all(
                &mut stream,
                b"HTTP/1.1 201 Created\r\nX-Request-ID: test-1\r\nContent-Length: 7\r\nConnection: close\r\n\r\ncreated",
            )
            .unwrap();
    });
    let source =
            "fn request(url: String) -> HttpResponse uses Network { return http_request(\"POST\", url, [\"X-Request-ID: test-1\", \"Content-Type: application/json\"], Some(\"{\\\"name\\\":\\\"Anna\\\"}\")) } fn status(url: String) -> Int uses Network { response = request(url) return response.status } fn main() { }";
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();
    let grants = HashSet::from([String::from("Network")]);
    let policy = RuntimePolicy {
        filesystem: None,
        network: Some(NetworkPolicy {
            allowed_hosts: vec![format!("127.0.0.1:{port}")],
            timeout_ms: 1_000,
            max_response_bytes: 100,
        }),
        process: None,
    };
    let response = execute_function_with_capabilities_and_policies(
        &program,
        "request",
        vec![Value::String(format!("http://127.0.0.1:{port}/customers"))],
        None,
        Some(&grants),
        Some(&policy),
    )
    .unwrap();
    let Value::Object { fields, .. } = response else {
        panic!("expected HttpResponse object");
    };
    assert_eq!(fields.get("status"), Some(&Value::Int(201)));
    assert_eq!(fields.get("body"), Some(&Value::String("created".into())));
    assert!(
        matches!(fields.get("headers"), Some(Value::Array(headers)) if headers.iter().any(|header| header == &Value::String("x-request-id: test-1".into())))
    );
    server.join().unwrap();
}

#[test]
fn sends_and_decodes_typed_json_http_request() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        let mut chunk = [0_u8; 512];
        while !request
            .windows(b"{\"name\":\"Anna\"}".len())
            .any(|window| window == b"{\"name\":\"Anna\"}")
        {
            let size = stream.read(&mut chunk).unwrap();
            if size == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..size]);
        }
        let request = String::from_utf8_lossy(&request);
        let request_lower = request.to_ascii_lowercase();
        assert!(request.starts_with("POST /customers HTTP/1.1"));
        assert!(request_lower.contains("content-type: application/json"));
        assert!(request.contains("{\"name\":\"Anna\"}"));
        let body = b"{\"id\":42,\"name\":\"Anna\"}";
        let response = format!(
                "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                String::from_utf8_lossy(body)
            );
        std::io::Write::write_all(&mut stream, response.as_bytes()).unwrap();
    });
    let source = r#"
            struct CustomerCreate { name: String }
            struct Customer { id: Int name: String }
            fn create(url: String, payload: CustomerCreate) -> Customer uses Network {
                return http_json<CustomerCreate, Customer>("POST", url, [], Some(payload))
            }
            fn main() { }
        "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();
    let grants = HashSet::from([String::from("Network")]);
    let policy = RuntimePolicy {
        filesystem: None,
        network: Some(NetworkPolicy {
            allowed_hosts: vec![format!("127.0.0.1:{port}")],
            timeout_ms: 1_000,
            max_response_bytes: 100,
        }),
        process: None,
    };
    let customer = execute_function_with_capabilities_and_policies(
        &program,
        "create",
        vec![
            Value::String(format!("http://127.0.0.1:{port}/customers")),
            Value::Object {
                type_name: "CustomerCreate".into(),
                fields: HashMap::from([(String::from("name"), Value::String("Anna".into()))]),
            },
        ],
        None,
        Some(&grants),
        Some(&policy),
    )
    .unwrap();
    let Value::Object { fields, .. } = customer else {
        panic!("expected decoded Customer object");
    };
    assert_eq!(fields.get("id"), Some(&Value::Int(42)));
    assert_eq!(fields.get("name"), Some(&Value::String("Anna".into())));
    server.join().unwrap();
}

#[test]
fn returns_structured_http_result_for_http_errors() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 512];
        let _ = stream.read(&mut request).unwrap();
        std::io::Write::write_all(
                &mut stream,
                b"HTTP/1.1 422 Unprocessable Entity\r\nX-Reason: invalid\r\nContent-Length: 21\r\nConnection: close\r\n\r\n{\"message\":\"invalid\"}",
            )
            .unwrap();
    });
    let source = r#"
            struct Customer { id: Int name: String }
            fn submit(url: String) -> HttpResult<Customer> uses Network {
                return http_result<Customer, Customer>("POST", url, [], None)
            }
            fn main() { }
        "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();
    let grants = HashSet::from([String::from("Network")]);
    let policy = RuntimePolicy {
        filesystem: None,
        network: Some(NetworkPolicy {
            allowed_hosts: vec![format!("127.0.0.1:{port}")],
            timeout_ms: 1_000,
            max_response_bytes: 100,
        }),
        process: None,
    };
    let result = execute_function_with_capabilities_and_policies(
        &program,
        "submit",
        vec![Value::String(format!("http://127.0.0.1:{port}/customers"))],
        None,
        Some(&grants),
        Some(&policy),
    )
    .unwrap();
    let Value::Object { fields, .. } = result else {
        panic!("expected HttpResult object");
    };
    assert_eq!(fields.get("status"), Some(&Value::Int(422)));
    assert_eq!(fields.get("data"), Some(&Value::Option(None)));
    let Value::Option(Some(error)) = fields.get("error").unwrap() else {
        panic!("expected structured HttpError");
    };
    let Value::Object { fields, .. } = &**error else {
        panic!("expected HttpError object");
    };
    assert_eq!(fields.get("status"), Some(&Value::Int(422)));
    assert_eq!(
        fields.get("body"),
        Some(&Value::String(r#"{"message":"invalid"}"#.into()))
    );
    server.join().unwrap();
}

#[test]
fn rejects_http_request_body_for_get() {
    let source = "fn request() -> HttpResponse uses Network { return http_request(\"GET\", \"http://example.test\", [], Some(\"body\")) } fn main() { }";
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();
    let grants = HashSet::from([String::from("Network")]);
    let policy = RuntimePolicy {
        filesystem: None,
        network: Some(NetworkPolicy {
            allowed_hosts: vec![String::from("example.test")],
            timeout_ms: 100,
            max_response_bytes: 100,
        }),
        process: None,
    };
    let error = execute_function_with_capabilities_and_policies(
        &program,
        "request",
        Vec::new(),
        None,
        Some(&grants),
        Some(&policy),
    )
    .unwrap_err();
    assert!(error.message.contains("does not allow a body with GET"));
}

#[test]
fn decodes_and_encodes_typed_json_records() {
    let source = r#"
            struct Address { city: String }
            struct Customer {
                name: String
                address: Address
                tags: String[]
                nickname: String?
            }
            fn decode(body: String) -> Customer {
                return json_decode<Customer>(body)
            }
            fn encode(customer: Customer) -> String {
                return json_encode(customer)
            }
            fn main() { }
        "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    let decoded = execute_function(
        &program,
        "decode",
        vec![Value::String(
            r#"{"name":"Anna","address":{"city":"Berlin"},"tags":["vip","de"]}"#.into(),
        )],
        None,
    )
    .unwrap();
    let Value::Object { fields, .. } = &decoded else {
        panic!("expected decoded Customer object");
    };
    assert_eq!(fields.get("name"), Some(&Value::String("Anna".into())));
    assert_eq!(fields.get("nickname"), Some(&Value::Option(None)));
    let Value::Object {
        fields: address, ..
    } = fields.get("address").unwrap()
    else {
        panic!("expected decoded Address object");
    };
    assert_eq!(address.get("city"), Some(&Value::String("Berlin".into())));

    let encoded = execute_function(&program, "encode", vec![decoded], None).unwrap();
    let Value::String(encoded) = encoded else {
        panic!("expected JSON string");
    };
    let json: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    assert_eq!(json["name"], "Anna");
    assert_eq!(json["address"]["city"], "Berlin");
    assert_eq!(json["tags"][0], "vip");
    assert!(json["nickname"].is_null());
}

#[test]
fn rejects_json_decode_unknown_record_field() {
    let source = "struct Customer { name: String } fn decode(body: String) -> Customer { return json_decode<Customer>(body) } fn main() { }";
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    let error = execute_function(
        &program,
        "decode",
        vec![Value::String(r#"{"name":"Anna","admin":true}"#.into())],
        None,
    )
    .unwrap_err();
    assert!(error.message.contains("unknown field `admin`"));
}

#[test]
fn rejects_json_decode_without_target_type() {
    let source = "fn decode(body: String) -> String { return json_decode(body) } fn main() { }";
    let program = parse(&lex(source).unwrap()).unwrap();
    let errors = check(&program).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("exactly one type argument")));
}

#[test]
fn requires_process_capability_for_run_process() {
    let program =
        parse(&lex("fn main() { print(run_process(\"/usr/bin/printf\", [\"hello\"])) }").unwrap())
            .unwrap();
    let errors = check_capabilities(&program).unwrap_err();
    assert!(errors.iter().any(|error| error
        .message
        .contains("does not declare capability `Process`")));
}

#[test]
fn runs_allowlisted_process_without_a_shell() {
    let source =
            "fn run() -> String uses Process { return run_process(\"/usr/bin/printf\", [\"%s\", \"hello\"]) } fn main() { }";
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();
    let grants = HashSet::from([String::from("Process")]);
    let policy = RuntimePolicy {
        filesystem: None,
        network: None,
        process: Some(ProcessPolicy {
            allowed_commands: vec![String::from("/usr/bin/printf")],
            timeout_ms: 1_000,
            max_output_bytes: 100,
        }),
    };
    let output = execute_function_with_capabilities_and_policies(
        &program,
        "run",
        Vec::new(),
        None,
        Some(&grants),
        Some(&policy),
    )
    .unwrap();
    assert_eq!(output, Value::String("hello".into()));
}

#[test]
fn rejects_processes_outside_allowlist_and_output_limit() {
    let source =
            "fn run(command: String) -> String uses Process { return run_process(command, [\"%s\", \"hello\"]) } fn main() { }";
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    let grants = HashSet::from([String::from("Process")]);
    let error = execute_function_with_capabilities_and_policies(
        &program,
        "run",
        vec![Value::String(String::from("/usr/bin/printf"))],
        None,
        Some(&grants),
        None,
    )
    .unwrap_err();
    assert!(error.message.contains("no process policy"));

    let policy = RuntimePolicy {
        filesystem: None,
        network: None,
        process: Some(ProcessPolicy {
            allowed_commands: vec![String::from("/usr/bin/printf")],
            timeout_ms: 1_000,
            max_output_bytes: 3,
        }),
    };
    let error = execute_function_with_capabilities_and_policies(
        &program,
        "run",
        vec![Value::String(String::from("/usr/bin/echo"))],
        None,
        Some(&grants),
        Some(&policy),
    )
    .unwrap_err();
    assert!(error.message.contains("process execution denied"));

    let error = execute_function_with_capabilities_and_policies(
        &program,
        "run",
        vec![Value::String(String::from("/usr/bin/printf"))],
        None,
        Some(&grants),
        Some(&policy),
    )
    .unwrap_err();
    assert!(error.message.contains("output limit"));
}

#[test]
fn accepts_clock_and_environment_host_apis() {
    let program = parse(
            &lex(
                "fn current_time() -> Timestamp uses Clock { return now() } fn read_env(name: String) -> String? uses Environment { return env(name) } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();

    let grants = HashSet::from([String::from("Clock"), String::from("Environment")]);
    let timestamp = execute_function_with_capabilities(
        &program,
        "current_time",
        Vec::new(),
        None,
        Some(&grants),
    )
    .unwrap();
    assert!(matches!(timestamp, Value::Timestamp(value) if value > 0));

    let environment = execute_function_with_capabilities(
        &program,
        "read_env",
        vec![Value::String(String::from("PATH"))],
        None,
        Some(&grants),
    )
    .unwrap();
    assert!(matches!(
        environment,
        Value::Option(Some(value)) if matches!(*value, Value::String(_))
    ));
}

#[test]
fn requires_clock_and_environment_capabilities_for_host_apis() {
    let program = parse(&lex("fn main() { print(now()) print(env(\"PATH\")) }").unwrap()).unwrap();
    let errors = check_capabilities(&program).unwrap_err();
    assert!(errors.iter().any(|error| {
        error.message.contains("does not declare capability") && error.message.contains("Clock")
    }));
    assert!(errors.iter().any(|error| {
        error.message.contains("does not declare capability")
            && error.message.contains("Environment")
    }));
}

#[test]
fn read_console_is_typed_and_requires_its_capability() {
    let program = parse(
            &lex(
                "fn main() uses Console { value = read_console(\"Date: \") match value { Some(date) => { print(date) } None => { print(\"No input\") } } }",
            )
            .unwrap(),
        )
        .unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();

    let denied_at_runtime = execute_with_capabilities(&program, Some(&HashSet::new())).unwrap_err();
    assert!(denied_at_runtime.message.contains("Console"));
    assert!(denied_at_runtime
        .message
        .contains("runtime capability denied"));

    let missing_capability =
        parse(&lex("fn main() { value = read_console(\"Date: \") }").unwrap()).unwrap();
    let errors = check_capabilities(&missing_capability).unwrap_err();
    assert!(errors.iter().any(|error| {
        error.message.contains("Console") && error.message.contains("console input")
    }));

    let invalid_prompt =
        parse(&lex("fn main() uses Console { value = read_console(42) }").unwrap()).unwrap();
    assert!(check(&invalid_prompt)
        .unwrap_err()
        .iter()
        .any(|error| error.message.contains("expected `String`, found `Int`")));
}

#[test]
fn enforces_runtime_host_api_capabilities() {
    let clock_program = parse(&lex("fn main() { print(now()) }").unwrap()).unwrap();
    let clock_error = execute(&clock_program).unwrap_err();
    assert!(clock_error.message.contains("clock access requires"));
    assert!(clock_error.message.contains("Clock"));

    let environment_program =
        parse(&lex("fn main() uses Environment { print(env(\"PATH\")) }").unwrap()).unwrap();
    let grants = HashSet::new();
    let environment_error =
        execute_with_capabilities(&environment_program, Some(&grants)).unwrap_err();
    assert!(environment_error.message.contains("function"));
    assert!(environment_error.message.contains("Environment"));
}

#[test]
fn accepts_secure_random_int_with_random_capability() {
    let program = parse(
        &lex("fn roll() -> Int uses Random { return random_int(1, 6) } fn main() { }").unwrap(),
    )
    .unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();

    let grants = HashSet::from([String::from("Random")]);
    let value =
        execute_function_with_capabilities(&program, "roll", Vec::new(), None, Some(&grants))
            .unwrap();
    assert!(matches!(value, Value::Int(value) if (1..=6).contains(&value)));
}

#[test]
fn rejects_invalid_random_range() {
    let program =
        parse(&lex("fn main() uses Random { print(random_int(6, 1)) }").unwrap()).unwrap();
    let grants = HashSet::from([String::from("Random")]);
    let error = execute_with_capabilities(&program, Some(&grants)).unwrap_err();
    assert!(error.message.contains("minimum <= maximum"));
}

#[test]
fn requires_random_capability_for_random_int() {
    let program = parse(&lex("fn main() { print(random_int(1, 6)) }").unwrap()).unwrap();
    let errors = check_capabilities(&program).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("does not declare capability")));
    assert!(errors.iter().any(|error| error.message.contains("Random")));

    let grants = HashSet::new();
    let error = execute_with_capabilities(&program, Some(&grants)).unwrap_err();
    assert!(error.message.contains("random access requires"));
}

#[test]
fn reads_utf8_text_with_file_system_capability() {
    let program = parse(
            &lex("fn read(path: String) -> String uses FileSystem { return read_text(path) } fn main() { }")
                .unwrap(),
        )
        .unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();

    let grants = HashSet::from([String::from("FileSystem")]);
    let path = format!("{}/../examples/fibonacci.zyl", env!("CARGO_MANIFEST_DIR"));
    let value = execute_function_with_capabilities(
        &program,
        "read",
        vec![Value::String(path)],
        None,
        Some(&grants),
    )
    .unwrap();
    assert!(matches!(value, Value::String(value) if value.contains("fibonacci")));
}

#[test]
fn enforces_file_system_capability_and_read_errors() {
    let program = parse(&lex("fn main() { print(read_text(\"README.md\")) }").unwrap()).unwrap();
    let errors = check_capabilities(&program).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("FileSystem")));

    let grants = HashSet::new();
    let capability_error = execute_with_capabilities(&program, Some(&grants)).unwrap_err();
    assert!(capability_error
        .message
        .contains("file-system read access requires"));

    let missing = parse(
        &lex(
            "fn main() uses FileSystem { print(read_text(\"/zelyra/path-that-does-not-exist\")) }",
        )
        .unwrap(),
    )
    .unwrap();
    let grants = HashSet::from([String::from("FileSystem")]);
    let read_error = execute_with_capabilities(&missing, Some(&grants)).unwrap_err();
    assert!(read_error.message.contains("file-system read failed"));
}

#[test]
fn writes_lists_and_deletes_with_a_file_system_policy() {
    let root = std::env::temp_dir().join(format!(
        "zelyra-filesystem-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let program = parse(
            &lex(
                "fn write_file(path: String) uses FileSystem { write_text(path, \"hello\") } fn read_file(path: String) -> String uses FileSystem { return read_text(path) } fn list_entries(path: String) -> String[] uses FileSystem { return list_dir(path) } fn remove_file(path: String) uses FileSystem { delete_file(path) } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();
    let policy = FileSystemPolicy {
        base_dir: root.clone(),
        read_roots: vec![root.clone()],
        write_roots: vec![root.clone()],
    };
    let grants = HashSet::from([String::from("FileSystem")]);
    let path = Value::String(String::from("note.txt"));
    execute_function_with_capabilities_and_filesystem_policy(
        &program,
        "write_file",
        vec![path.clone()],
        None,
        Some(&grants),
        Some(&policy),
    )
    .unwrap();
    let content = execute_function_with_capabilities_and_filesystem_policy(
        &program,
        "read_file",
        vec![path.clone()],
        None,
        Some(&grants),
        Some(&policy),
    )
    .unwrap();
    assert_eq!(content, Value::String(String::from("hello")));
    let entries = execute_function_with_capabilities_and_filesystem_policy(
        &program,
        "list_entries",
        vec![Value::String(String::from("."))],
        None,
        Some(&grants),
        Some(&policy),
    )
    .unwrap();
    assert_eq!(
        entries,
        Value::Array(vec![Value::String(String::from("note.txt"))])
    );
    execute_function_with_capabilities_and_filesystem_policy(
        &program,
        "remove_file",
        vec![path],
        None,
        Some(&grants),
        Some(&policy),
    )
    .unwrap();
    assert!(!root.join("note.txt").exists());
    std::fs::remove_dir(&root).unwrap();
}

#[test]
fn rejects_file_system_writes_outside_configured_roots() {
    let program =
        parse(&lex("fn main() uses FileSystem { write_text(\"blocked.txt\", \"no\") }").unwrap())
            .unwrap();
    let root = std::env::current_dir().unwrap();
    let policy = FileSystemPolicy {
        base_dir: root.clone(),
        read_roots: vec![root.clone()],
        write_roots: Vec::new(),
    };
    let grants = HashSet::from([String::from("FileSystem")]);
    let error =
        execute_with_capabilities_and_filesystem_policy(&program, Some(&grants), Some(&policy))
            .unwrap_err();
    assert!(error.message.contains("outside the configured roots"));
}

#[test]
fn enforces_preconditions_and_postconditions() {
    let source = "fn increment(value: Int) -> Int requires { value >= 0 } ensures { result > value } { return value + 1 } fn main() { print(increment(1)) }";
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    assert_eq!(execute(&program).unwrap(), ["2"]);

    let failing_precondition =
            parse(&lex("fn increment(value: Int) -> Int requires { value >= 0 } { return value + 1 } fn main() { increment(-1) }").unwrap()).unwrap();
    check(&failing_precondition).unwrap();
    let error = execute(&failing_precondition).unwrap_err();
    assert!(error.message.contains("precondition failed"));

    let failing_postcondition =
            parse(&lex("fn broken(value: Int) -> Int ensures { result > value } { return value } fn main() { broken(1) }").unwrap()).unwrap();
    check(&failing_postcondition).unwrap();
    let error = execute(&failing_postcondition).unwrap_err();
    assert!(error.message.contains("postcondition failed"));
}

#[test]
fn requires_and_ensures_must_be_bool() {
    let program =
        parse(&lex("fn main() requires { 1 } ensures { \"done\" } { print(1) }").unwrap()).unwrap();
    let errors = check(&program).unwrap_err();
    assert_eq!(errors.len(), 2);
    assert!(errors
        .iter()
        .all(|error| error.message.contains("expected `Bool`")));
}

#[test]
fn classifies_verification_results_without_overclaiming() {
    let program = parse(
            &lex("fn proven() requires { 1 < 2 } { } fn dynamic(value: Int) requires { value >= 0 } { } fn failed() ensures { 1 > 2 } { } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[0].status, VerificationStatus::Proven);
    assert_eq!(results[1].status, VerificationStatus::RuntimeCheck);
    assert_eq!(results[2].status, VerificationStatus::Failed);
    assert_eq!(
        results[2].message,
        "Constant contradiction: this condition evaluates to false for every input."
    );
    assert_eq!(results[3].status, VerificationStatus::Unproven);
}

#[test]
fn reports_a_small_linear_counterexample_for_a_failed_postcondition() {
    let program = parse(
            &lex("fn bad(value: Int) -> Int requires { value >= 0 } ensures { result > value } { return value } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[1].status, VerificationStatus::Failed);
    assert_eq!(results[1].counterexample, Some(vec![("value".into(), 0)]));
}

#[test]
fn reports_a_small_linear_counterexample_with_three_variables() {
    let program = parse(
            &lex("fn bad(a: Int, b: Int, c: Int) -> Int requires { a >= 0 && c <= b } ensures { result > b } { return c } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[1].status, VerificationStatus::Failed);
    assert_eq!(
        results[1].counterexample,
        Some(vec![("a".into(), 0), ("b".into(), 0), ("c".into(), 0)])
    );
}

#[test]
fn proves_simple_integer_postconditions_from_direct_returns() {
    let program = parse(
            &lex("fn increment(value: Int) -> Int ensures { result > value } { return value + 1 } fn unchanged(value: Int) -> Int ensures { result > value } { return value } fn absolute(value: Int) -> Int ensures { result >= 0 } { if value >= 0 { return value } else { return -value } } fn classify(value: Int) -> Int ensures { result >= 0 } { match value { 0 => { return 0 } _ => { return 1 } } } fn bool_value(value: Bool) -> Int ensures { result >= 0 } { match value { true => { return 1 } false => { return 0 } } } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[0].status, VerificationStatus::Proven);
    assert_eq!(results[1].status, VerificationStatus::Failed);
    assert_eq!(results[2].status, VerificationStatus::Proven);
    assert_eq!(results[3].status, VerificationStatus::Proven);
    assert_eq!(results[4].status, VerificationStatus::Proven);
    assert_eq!(results[5].status, VerificationStatus::Unproven);
}

#[test]
fn proves_immutable_local_bindings_in_contracts() {
    let program = parse(
            &lex("fn increment_local(value: Int) -> Int ensures { result > value } { next: Int = value + 1 return next } fn caller(value: Int) -> Int ensures { result > value } { next = value + 1 return next } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[0].status, VerificationStatus::Proven);
    assert_eq!(results[1].status, VerificationStatus::Proven);
    assert_eq!(results[2].status, VerificationStatus::Unproven);
}

#[test]
fn proves_mutable_local_initialization() {
    let program = parse(
            &lex("fn increment_local(value: Int) -> Int ensures { result > value } { mutable next = value + 1 return next } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[0].status, VerificationStatus::Proven);
    assert_eq!(results[1].status, VerificationStatus::Unproven);
}

#[test]
fn proves_linear_mutable_assignments() {
    let program = parse(
            &lex("fn increment_mutable(value: Int) -> Int ensures { result > value } { mutable next = value next = next + 1 return next } fn caller(value: Int) -> Int ensures { result > value } { mutable next = value next = next + 1 return next } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[0].status, VerificationStatus::Proven);
    assert_eq!(results[1].status, VerificationStatus::Proven);
    assert_eq!(results[2].status, VerificationStatus::Unproven);
}

#[test]
fn keeps_non_linear_mutable_assignments_unproven() {
    let program = parse(
            &lex("fn multiply_mutable(value: Int) -> Int ensures { result > value } { mutable next = value next = next * value return next } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[0].status, VerificationStatus::RuntimeCheck);
    assert_eq!(results[1].status, VerificationStatus::Unproven);
}

#[test]
fn proves_bounded_linear_while_loops() {
    let program = parse(
            &lex("fn add_three(value: Int) -> Int ensures { result >= value + 3 } { mutable total = value mutable count = 0 while count < 3 { total = total + 1 count = count + 1 } return total } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[0].status, VerificationStatus::Proven);
    assert_eq!(results[1].status, VerificationStatus::Unproven);
}

#[test]
fn preserves_mutable_condition_snapshots() {
    let program = parse(
            &lex("fn avoid_zero(value: Int) -> Int ensures { result != 0 } { mutable current = value if current >= 0 { current = current + 1 } else { current = current - 1 } return current } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[0].status, VerificationStatus::Proven);
    assert_eq!(results[1].status, VerificationStatus::Unproven);
}

#[test]
fn keeps_unbounded_while_loops_unproven() {
    let program = parse(
            &lex("fn reduce(value: Int) -> Int ensures { result == 0 } { mutable current = value while current > 0 { current = current - 1 } return current } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[0].status, VerificationStatus::RuntimeCheck);
    assert_eq!(results[1].status, VerificationStatus::Unproven);
}

#[test]
fn proves_unbounded_linear_loops_with_invariants() {
    let program = parse(
            &lex("fn reduce(value: Int) -> Int requires { value >= 0 } ensures { result == 0 } { mutable current = value while current > 0 invariant { current >= 0 } { current = current - 1 } return current } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[0].status, VerificationStatus::RuntimeCheck);
    assert_eq!(results[1].status, VerificationStatus::Proven);
    assert_eq!(results[2].kind, ContractKind::LoopInvariant);
    assert_eq!(results[2].status, VerificationStatus::Proven);
    assert_eq!(results[3].status, VerificationStatus::Unproven);
}

#[test]
fn proves_invariant_controlled_unconditional_loops() {
    let program = parse(
            &lex("fn stop_after_one() -> Int ensures { result == 1 } { mutable i = 0 loop invariant { i >= 0 } { i = i + 1 break } return i } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[0].status, VerificationStatus::Proven);
    assert_eq!(results[1].kind, ContractKind::LoopInvariant);
    assert_eq!(results[1].status, VerificationStatus::Proven);
    assert_eq!(results[2].status, VerificationStatus::Unproven);
}

#[test]
fn does_not_prove_a_non_preserved_loop_invariant() {
    let program = parse(
            &lex("fn reduce(value: Int) -> Int requires { value > 0 } ensures { result == 0 } { mutable current = value while current > 0 invariant { current == value } { current = current - 1 } return current } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[0].status, VerificationStatus::RuntimeCheck);
    assert_eq!(results[1].status, VerificationStatus::RuntimeCheck);
    assert_eq!(results[2].kind, ContractKind::LoopInvariant);
    assert_eq!(results[2].status, VerificationStatus::Failed);
    assert!(results[2].message.contains("not preserved"));
    assert!(results[2].counterexample.is_some());
    assert_eq!(results[3].status, VerificationStatus::Unproven);
}

#[test]
fn reports_each_loop_invariant_status() {
    let program = parse(
            &lex("fn reduce(value: Int) -> Int requires { value >= 0 } ensures { result == 0 } { mutable current = value while current > 0 invariant { current >= 0 } invariant { current == value } { current = current - 1 } return current } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[2].kind, ContractKind::LoopInvariant);
    assert_eq!(results[2].index, 0);
    assert_eq!(results[2].status, VerificationStatus::Proven);
    assert_eq!(results[3].kind, ContractKind::LoopInvariant);
    assert_eq!(results[3].index, 1);
    assert_eq!(results[3].status, VerificationStatus::Failed);
}

#[test]
fn proves_break_and_continue_paths_in_bounded_loops() {
    let program = parse(
            &lex("fn stop_after_one() -> Int ensures { result == 1 } { mutable i = 0 while i < 5 { i = i + 1 break } return i } fn skip_one() -> Int ensures { result == 2 } { mutable i = 0 mutable total = 0 while i < 3 { i = i + 1 if i == 2 { continue } total = total + 1 } return total } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[0].status, VerificationStatus::Proven);
    assert_eq!(results[1].status, VerificationStatus::Proven);
    assert_eq!(results[2].status, VerificationStatus::Unproven);
}

#[test]
fn collects_option_and_result_constructor_paths() {
    let program = parse(
            &lex("fn option_flag(value: Int?) -> Int ensures { result >= 0 } { match value { Some(number) => { return 1 } None => { return 0 } } } fn result_flag(value: Result<Int, String>) -> Int ensures { result >= 0 } { match value { Ok(number) => { return 1 } Err(message) => { return 0 } } } fn known_option() -> Int ensures { result == 4 } { match Some(4) { Some(number) => { return number } None => { return 0 } } } fn known_result() -> Int ensures { result == 7 } { match Ok(7) { Ok(number) => { return number } Err(message) => { return 0 } } } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[0].status, VerificationStatus::Proven);
    assert_eq!(results[1].status, VerificationStatus::Proven);
    assert_eq!(results[2].status, VerificationStatus::Proven);
    assert_eq!(results[3].status, VerificationStatus::Proven);
    assert_eq!(results[4].status, VerificationStatus::Unproven);
}

#[test]
fn proves_direct_function_call_summaries() {
    let program = parse(
            &lex("fn increment(value: Int) -> Int { return value + 1 } fn twice(value: Int) -> Int ensures { result > value } { return increment(increment(value)) } fn known() -> Int ensures { result == 5 } { return increment(4) } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[0].status, VerificationStatus::Unproven);
    assert_eq!(results[1].status, VerificationStatus::Proven);
    assert_eq!(results[2].status, VerificationStatus::Proven);
    assert_eq!(results[3].status, VerificationStatus::Unproven);
}

#[test]
fn proves_path_sensitive_function_call_summaries() {
    let program = parse(
            &lex("fn absolute(value: Int) -> Int ensures { result >= 0 } { if value >= 0 { return value } else { return -value } } fn caller(value: Int) -> Int ensures { result >= 0 } { return absolute(value) } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[0].status, VerificationStatus::Proven);
    assert_eq!(results[1].status, VerificationStatus::Proven);
    assert_eq!(results[2].status, VerificationStatus::Unproven);
}

#[test]
fn rejects_impossible_postcondition_for_path_sensitive_call() {
    let program = parse(
            &lex("fn absolute(value: Int) -> Int ensures { result >= 0 } { if value >= 0 { return value } else { return -value } } fn caller(value: Int) -> Int ensures { result < 0 } { return absolute(value) } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[0].status, VerificationStatus::Proven);
    assert_eq!(results[1].status, VerificationStatus::Failed);
    assert_eq!(results[2].status, VerificationStatus::Unproven);
}

#[test]
fn uses_function_preconditions_as_postcondition_assumptions() {
    let program = parse(
            &lex("fn non_negative(value: Int) -> Int requires { value >= 0 } ensures { result >= 0 } { return value } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[0].status, VerificationStatus::RuntimeCheck);
    assert_eq!(results[1].status, VerificationStatus::Proven);
    assert_eq!(results[2].status, VerificationStatus::Unproven);
}

#[test]
fn checks_called_function_preconditions_under_caller_assumptions() {
    let program = parse(
            &lex("fn non_negative(value: Int) -> Int requires { value >= 0 } { return value } fn safe_call(value: Int) -> Int requires { value >= 0 } ensures { result >= 0 } { return non_negative(value) } fn unchecked_call(value: Int) -> Int ensures { result >= 0 } { return non_negative(value) } fn main() { }").unwrap(),
        )
        .unwrap();
    let results = verify(&program);
    assert_eq!(results[0].status, VerificationStatus::RuntimeCheck);
    assert_eq!(results[1].status, VerificationStatus::RuntimeCheck);
    assert_eq!(results[2].status, VerificationStatus::Proven);
    assert_eq!(results[3].status, VerificationStatus::RuntimeCheck);
    assert_eq!(results[4].status, VerificationStatus::Unproven);
}

#[test]
fn validates_api_types_and_path_parameters() {
    let program = parse(
            &lex("type CustomerId = Id table customers { id: Id primary auto } api GET \"/customers/{id}\" { input { id: CustomerId } output Customer errors { 404 NotFound } } fn main() { }").unwrap(),
        )
        .unwrap();
    assert!(check_apis(&program).is_ok());
}

#[test]
fn validates_api_handler_signature() {
    let program = parse(
            &lex("type CustomerId = Id table customers { id: Id primary auto } fn get_customer(id: CustomerId) -> Customer { } api GET \"/customers/{id}\" { handler get_customer input { id: CustomerId } output Customer } fn main() { }").unwrap(),
        )
        .unwrap();
    assert!(check_apis(&program).is_ok());
}

#[test]
fn rejects_non_string_map_keys_at_the_api_boundary() {
    let program = parse(
            &lex("api POST \"/settings\" { input { settings: Map<Int, String> } output Map<Int, String> } fn main() { }").unwrap(),
        )
        .unwrap();
    let errors = check_apis(&program).unwrap_err();
    assert_eq!(errors.len(), 2);
    assert!(errors
        .iter()
        .all(|error| error.message.contains("JSON API maps require")));
}

#[test]
fn rejects_api_paths_without_declared_inputs() {
    let program =
        parse(&lex("api GET \"/customers/{id}\" { output String } fn main() { }").unwrap())
            .unwrap();
    let errors = check_apis(&program).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("path parameter `id`")));
}
