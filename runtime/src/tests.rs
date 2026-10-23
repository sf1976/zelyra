use super::*;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{io::Read, net::TcpListener};
use zelyra_lexer::lex;
use zelyra_parser::parse;

#[test]
fn api_response_value_rejects_header_injection_and_server_control_headers() {
    let response = api_response_value(
        201,
        vec!["Cache-Control: no-store".into()],
        Value::String("created".into()),
    )
    .unwrap();
    assert_eq!(
        response.ty(),
        Type::Applied {
            name: "ApiResponse".into(),
            args: vec![Type::String],
        }
    );
    for headers in [
        vec!["Cache-Control: safe\r\nSet-Cookie: stolen=1".into()],
        vec!["Content-Length: 10".into()],
        vec!["Set-Cookie: session=unsafe".into()],
        vec!["X-Unreviewed: value".into()],
        vec!["Location: //evil.example/path".into()],
    ] {
        assert!(api_response_value(201, headers, Value::String("x".into())).is_err());
    }
    assert!(api_response_value(404, Vec::new(), Value::String("x".into())).is_err());
}

#[test]
fn shared_execution_limits_define_defaults_caps_and_worker_stack_budget() {
    assert_eq!(effective_function_call_depth(None), 20);
    assert_eq!(effective_function_call_depth(Some(37)), 37);
    assert_eq!(
        effective_function_call_depth(Some(usize::MAX)),
        MAX_CONFIGURED_FUNCTION_CALL_DEPTH
    );
    assert_eq!(runtime_stack_size_for_depth(0), 2 * 1024 * 1024);
    assert_eq!(
        runtime_stack_size_for_depth(usize::MAX),
        2 * 1024 * 1024 + MAX_CONFIGURED_FUNCTION_CALL_DEPTH * 192 * 1024
    );
}

fn run(source: &str) -> Vec<String> {
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    execute(&program).unwrap()
}

#[test]
fn explicit_database_references_are_typechecked_against_declared_id_keys() {
    let valid = r#"
        type TenantId = Id
        table tenants { id: TenantId primary auto }
        table customers scoped by tenant_id: TenantId {
            id: Id primary auto
            tenant_id: TenantId required references tenants
        }
        fn main() {}
    "#;
    check(&parse(&lex(valid).unwrap()).unwrap()).expect("valid scalar ID reference");

    for invalid in [
        r#"table sources { target_id: Id references missing } fn main() {}"#,
        r#"table targets { name: String primary } table sources { target_id: Id references targets } fn main() {}"#,
        r#"table targets { id: String primary } table sources { target_id: String references targets } fn main() {}"#,
        r#"table targets { id: Id primary auto } table sources { target: Target references targets } fn main() {}"#,
    ] {
        let errors = check(&parse(&lex(invalid).unwrap()).unwrap())
            .expect_err("invalid scalar references must fail the checker");
        assert!(
            errors.iter().any(|error| error.code == "E-REF-001"),
            "{invalid}"
        );
    }
}

#[test]
fn recursion_is_bounded_and_reports_the_call_site_without_crashing() {
    let direct = "fn dive(depth: Int) -> Int { if depth == 0 { return 0 } return dive(depth - 1) + 1 } fn main() { print(dive(10000)) }";
    let program = parse(&lex(direct).unwrap()).unwrap();
    check(&program).unwrap();
    let error = execute(&program).expect_err("deep recursion must be stopped by the runtime");
    assert!(
        error.message.contains("call depth limit"),
        "{}",
        error.message
    );
    assert!(
        error.span.start > 0,
        "the diagnostic should identify the recursive call"
    );

    let mutual = "fn even(depth: Int) -> Bool { if depth == 0 { return true } return odd(depth - 1) } fn odd(depth: Int) -> Bool { if depth == 0 { return false } return even(depth - 1) } fn main() { print(even(10000)) }";
    let program = parse(&lex(mutual).unwrap()).unwrap();
    check(&program).unwrap();
    let error = execute(&program).expect_err("mutual recursion shares the call depth bound");
    assert!(
        error.message.contains("call depth limit"),
        "{}",
        error.message
    );
    assert!(error.span.start > 0);

    assert_eq!(
        run("fn sum(depth: Int) -> Int { if depth == 0 { return 0 } return depth + sum(depth - 1) } fn main() { print(sum(16)) }"),
        ["136"]
    );
}

#[test]
fn embedding_policy_cannot_raise_ast_call_depth_above_the_runtime_cap() {
    let source = "fn dive(depth: Int) -> Int { if depth == 0 { return 0 } return dive(depth - 1) + 1 } fn main() { print(dive(600)) }";
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    let policy = RuntimePolicy {
        max_stack_depth: Some(usize::MAX),
        ..RuntimePolicy::default()
    };
    let outcome = execute_captured_with_capabilities_and_policies(&program, None, Some(&policy));
    let error = outcome
        .error
        .expect("the runtime must enforce its hard call-depth cap");
    assert!(error
        .message
        .contains("call depth limit exceeded (maximum 500)"));
    assert!(error.span.start > 0);
}

#[test]
fn if_expressions_return_the_selected_branch_value() {
    let source = "fn choose(ready: Bool) -> Int { return if ready { 7 } else { 2 } } fn main() { print(choose(true)); print(choose(false)) }";
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    assert_eq!(execute(&program).unwrap(), ["7", "2"]);
}

#[test]
fn numeric_conversion_builtins_reject_type_arguments() {
    for (source, builtin) in [
        ("fn main() { value = to_int<Int>(1) }", "to_int"),
        ("fn main() { value = to_float<Float>(1) }", "to_float"),
        ("fn main() { value = parse_int<Int>(\"1\") }", "parse_int"),
        (
            "fn main() { value = parse_float<Float>(\"1.0\") }",
            "parse_float",
        ),
        (
            "fn main() { value = parse_decimal<Decimal>(\"1.0\") }",
            "parse_decimal",
        ),
        (
            "fn main() { value = parse_date<Date>(\"2026-01-01\") }",
            "parse_date",
        ),
        (
            "fn main() { value = parse_time<Time>(\"12:00:00\") }",
            "parse_time",
        ),
    ] {
        let program = parse(&lex(source).unwrap()).unwrap();
        let errors = check(&program).expect_err("scalar conversions are not generic");
        assert!(
            errors.iter().any(|error| {
                error.message.contains(builtin) && error.message.contains("no type arguments")
            }),
            "expected a clear type-argument diagnostic for {builtin}, got {errors:?}"
        );
    }
}

#[test]
fn parse_date_and_time_return_typed_results_for_canonical_values() {
    let source = r#"
fn main() {
    match parse_date("2024-02-29") {
        Ok(value) => { print(value) }
        Err(_) => { print("unexpected date error") }
    }
    print(is_err(parse_date("1900-02-29")))
    print(is_err(parse_date("0000-01-01")))
    print(is_err(parse_date("2024-2-09")))
    match parse_time("23:59:59") {
        Ok(value) => { print(value) }
        Err(_) => { print("unexpected time error") }
    }
    print(is_err(parse_time("24:00:00")))
    print(is_err(parse_time("12:60:00")))
    print(is_err(parse_time("09:15")))
    print(is_err(parse_time("09:15:30.000")))
}
"#;
    assert_eq!(
        run(source),
        [
            "2024-02-29",
            "true",
            "true",
            "true",
            "23:59:59",
            "true",
            "true",
            "true",
            "true"
        ]
    );

    for source in [
        "fn main() { value = parse_date(42) }",
        "fn main() { value = parse_time(false) }",
    ] {
        let program = parse(&lex(source).unwrap()).unwrap();
        let errors = check(&program).expect_err("temporal parsing must require String");
        assert!(errors
            .iter()
            .any(|error| { error.message.contains("expected `String`") }));
    }
}

#[test]
fn parse_decimal_returns_a_typed_result_for_valid_and_invalid_values() {
    let source = r#"
fn main() {
    print(is_err(parse_decimal("+1.0")))
    print(is_err(parse_decimal(" 1.0")))
    print(is_err(parse_decimal("1e2")))
    print(is_err(parse_decimal("170141183460469231731687303715.884105728")))
    match parse_decimal("12345678901234567890.123456789") {
        Ok(value) => { print(value) }
        Err(_) => { print("unexpected decimal error") }
    }
    match parse_decimal("1.0000000001") {
        Ok(_) => { print("unexpected decimal success") }
        Err(message) => { print(message) }
    }
}
"#;
    assert_eq!(
        run(source),
        [
            "true",
            "true",
            "true",
            "true",
            "12345678901234567890.123456789",
            "invalid decimal: 1.0000000001"
        ]
    );

    let wrong_type = parse(&lex("fn main() { value = parse_decimal(42) }").unwrap()).unwrap();
    let errors = check(&wrong_type).expect_err("parse_decimal must require a String");
    assert!(errors.iter().any(|error| {
        error.message.contains("expected `String`") && error.message.contains("found `Int`")
    }));
}

#[test]
fn command_and_schedule_declarations_are_compiler_checked() {
    let valid = r#"
command process_recurring_invoices {
    handler: run_recurring_invoices
    description: "Create recurring invoices"
}
schedule recurring_invoices {
    command: process_recurring_invoices
    cron: "0 4 * * *"
    timezone: "Europe/Berlin"
}
fn run_recurring_invoices() { }
fn main() { }
"#;
    let program = parse(&lex(valid).unwrap()).unwrap();
    check(&program).unwrap();

    let unknown_handler = r#"
command process { handler: missing }
fn main() { }
"#;
    let errors = check(&parse(&lex(unknown_handler).unwrap()).unwrap()).unwrap_err();
    assert!(errors.iter().any(|error| {
        error.code == "E-TASK-001" && error.message.contains("unknown handler function")
    }));

    let invalid_schedule = r#"
command process { handler: run }
schedule recurring { command: absent cron: "61 * * * *" }
fn run() { }
fn main() { }
"#;
    let errors = check(&parse(&lex(invalid_schedule).unwrap()).unwrap()).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| { error.code == "E-TASK-001" && error.message.contains("unknown command") }));
    assert!(errors.iter().any(|error| {
        error.code == "E-TASK-001" && error.message.contains("invalid cron expression")
    }));

    let scheduled_arguments = r#"
command process { handler: run }
schedule recurring { command: process cron: "0 4 * * *" }
fn run(account_id: Int) { }
fn main() { }
"#;
    let errors = check(&parse(&lex(scheduled_arguments).unwrap()).unwrap()).unwrap_err();
    assert!(errors.iter().any(|error| {
        error.code == "E-TASK-001" && error.message.contains("zero-argument handler")
    }));
}

#[test]
fn json_values_round_trip_as_typed_json_not_strings() {
    let source = r#"
fn main() {
    payload: Json = json_decode<Json>("{\"items\":[1,true,null],\"name\":\"Ada\"}")
    print(json_encode(payload))
}
"#;
    assert_eq!(run(source), [r#"{"items":[1,true,null],"name":"Ada"}"#]);
}

#[test]
fn sql_decode_accepts_json_scalars_and_json_record_fields() {
    let source = r#"
database main { engine: mariadb }
table events { id: Id primary auto payload: Json required }
struct EventRecord { id: Id payload: Json }
fn main() uses Database(read) {
    scalar = sql_decode<Json>(sql<Json> { SELECT payload FROM events })
    record = sql_decode<EventRecord>(sql<EventRecord> { SELECT id, payload FROM events })
    print(json_encode(scalar))
    print(json_encode(record.payload))
}
"#;
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
}

#[test]
fn checked_program_exposes_source_mapped_inferred_expression_types() {
    let source =
        "fn increment(value: Int) -> Int { return value + 1 } fn main() { print(increment(41)) }";
    let program = parse(&lex(source).unwrap()).unwrap();
    let inferred = check_with_inferred_types(&program).unwrap();

    assert!(!inferred.is_empty());
    assert!(inferred.windows(2).all(|pair| {
        (pair[0].span.source_id, pair[0].span.start, pair[0].span.end)
            <= (pair[1].span.source_id, pair[1].span.start, pair[1].span.end)
    }));
    let expression_start = source.find("value + 1").unwrap();
    assert!(inferred.iter().any(|expression| {
        expression.span.start == expression_start && expression.ty == Type::Int
    }));
    assert!(inferred.iter().any(|expression| {
        expression.span.start == source.find("increment(41)").unwrap() && expression.ty == Type::Int
    }));
}

#[test]
fn type_compatibility_matches_checker_relations_and_rejects_nested_unknowns() {
    let expected_function = Type::Function {
        params: vec![],
        return_type: Box::new(Type::String),
        capabilities: vec!["Database(read)".into()],
    };
    let pure_function = Type::Function {
        params: vec![],
        return_type: Box::new(Type::String),
        capabilities: vec![],
    };
    assert!(types_compatible(&expected_function, &pure_function));
    assert!(types_compatible(
        &Type::Option(Box::new(Type::String)),
        &Type::Option(Box::new(Type::String))
    ));
    assert!(!types_compatible(
        &Type::Option(Box::new(Type::String)),
        &Type::Option(Box::new(Type::Unknown))
    ));
    assert!(!types_compatible(&Type::Unknown, &Type::String));
}

#[test]
fn inferred_expression_types_are_not_returned_for_invalid_programs() {
    let program = parse(&lex("fn main() { value: Int = \"wrong\" }").unwrap()).unwrap();
    let errors =
        check_with_inferred_types(&program).expect_err("type errors must block typed output");
    assert!(errors.iter().any(|error| error.code == "E-TYPE-001"));
    assert!(check(&program).is_err());
}

#[test]
fn partial_type_inspection_keeps_receiver_types_without_weakening_check() {
    let source = "struct Customer { name: String } fn main() { customer: Customer = Customer { name: \"Ada\" } print(customer.__zelyra_completion_hole) }";
    let program = parse(&lex(source).unwrap()).unwrap();
    let (inferred, errors) = inspect_expression_types(&program);
    assert!(errors
        .iter()
        .any(|error| error.message.contains("unknown field")));
    let receiver_start = source.rfind("customer.__zelyra_completion_hole").unwrap();
    assert!(inferred.iter().any(|item| {
        item.span.start == receiver_start
            && item.span.end == receiver_start + "customer".len()
            && item.ty == Type::Named("Customer".into())
    }));
    assert!(check_with_inferred_types(&program).is_err());
    assert!(check(&program).is_err());
}

#[test]
fn expected_types_flow_into_empty_collections_and_result_match_arms() {
    let source = r#"
fn as_result(flag: Bool) -> Result<Int, String> {
    if flag { return Ok(7) }
    return match Err("no") {
        Ok(value) => Ok(value)
        Err(message) => Err(message)
    }
}
fn main() {
    empty: Int[] = []
    settings: Map<String, Int> = Map {}
    failed: Result<Int, String> = Err("invalid")
    print(as_result(true))
    print(empty)
    print(settings)
    print(failed)
}
"#;
    let program = parse(&lex(source).unwrap()).unwrap();
    let inferred = check_with_inferred_types(&program).unwrap();

    for (expression, start, expected) in [
        (
            "[]",
            source.find("= []").unwrap() + 2,
            Type::Array(Box::new(Type::Int)),
        ),
        (
            "Map {}",
            source.find("= Map {}").unwrap() + 2,
            Type::Map(Box::new(Type::String), Box::new(Type::Int)),
        ),
        (
            "Err(\"invalid\")",
            source.find("= Err(\"invalid\")").unwrap() + 2,
            Type::Result(Box::new(Type::Int), Box::new(Type::String)),
        ),
        (
            "Ok(value)",
            source.find("=> Ok(value)").unwrap() + 3,
            Type::Result(Box::new(Type::Int), Box::new(Type::String)),
        ),
        (
            "Err(message)",
            source.find("=> Err(message)").unwrap() + 3,
            Type::Result(Box::new(Type::Int), Box::new(Type::String)),
        ),
    ] {
        assert!(
            inferred
                .iter()
                .any(|entry| entry.span.start == start && entry.ty == expected),
            "expected {expression:?} to infer as {expected}"
        );
    }
}

#[test]
fn expected_collection_and_result_types_reject_mismatched_elements() {
    let program = parse(&lex(
        "fn main() { values: Int[] = [1, \"wrong\"] settings: Map<String, Int> = Map { \"key\": \"wrong\" } result: Result<Int, String> = Err(9) }",
    ).unwrap()).unwrap();
    let errors = check(&program).expect_err("contextual collection element types must be checked");
    assert!(errors.iter().any(|error| error.code == "E-TYPE-001"));
}

#[test]
fn string_interpolation_type_checks_expressions_and_supports_literal_braces() {
    let source = r#"
struct Customer { name: String id: Int total: Float }
fn main() {
    customer = Customer { name: "Ada", id: 501, total: 12.50 }
    print("Customer {customer.name} (ID: {customer.id}) paid {customer.total} EUR. {{ok}}; state {if customer.id == 501 { \"accepted\" } else { \"unknown\" }}")
}
"#;
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    assert_eq!(
        execute(&program).unwrap(),
        ["Customer Ada (ID: 501) paid 12.5 EUR. {ok}; state accepted"]
    );

    let invalid = r#"fn main() { print("Hello {missing_name}") }"#;
    let program = parse(&lex(invalid).unwrap()).unwrap();
    let errors =
        check(&program).expect_err("interpolation expressions must resolve and type check");
    assert!(errors
        .iter()
        .any(|error| error.message.contains("unknown variable `missing_name`")));
}

#[test]
fn string_interpolation_diagnostics_point_to_the_expression_source() {
    let source = r#"fn main() { print("Grüße {to_string(\"quoted\") + missing}") }"#;
    let program = parse(&lex(source).unwrap()).unwrap();
    let errors = check(&program).expect_err("unresolved interpolation names must fail");
    let error = errors
        .iter()
        .find(|error| error.message.contains("unknown variable `missing`"))
        .expect("the missing interpolation variable should be diagnosed");
    assert_eq!(&source[error.span.start..error.span.end], "missing");
    assert_eq!(error.span.line, 1);
    let line_start = source[..error.span.start]
        .rfind('\n')
        .map_or(0, |newline| newline + 1);
    assert_eq!(
        error.span.column,
        source[line_start..error.span.start].chars().count() + 1
    );
}

#[test]
fn mutable_array_bindings_support_checked_in_place_index_updates() {
    let source = r#"
fn main() {
    mutable values: Int[] = [1, 2, 3]
    values[1] += 5
    values[0] = 9
    print(values[0])
    print(values[1])
}
"#;
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    assert_eq!(execute(&program).unwrap(), ["9", "7"]);

    let immutable = "fn main() { values = [1] values[0] += 1 }";
    let program = parse(&lex(immutable).unwrap()).unwrap();
    let errors =
        check(&program).expect_err("index writes must require an explicit mutable binding");
    assert!(errors
        .iter()
        .any(|error| error.message.contains("mutable array binding")));

    let invalid_add = "fn main() { mutable flags: Bool[] = [true] flags[0] += false }";
    let program = parse(&lex(invalid_add).unwrap()).unwrap();
    let errors = check(&program).expect_err("compound addition must support only defined types");
    assert!(errors.iter().any(|error| error.code == "E-TYPE-ARRAY-001"));

    let out_of_bounds = "fn main() { mutable values: Int[] = [1] values[2] = 3 }";
    let program = parse(&lex(out_of_bounds).unwrap()).unwrap();
    check(&program).unwrap();
    let error = execute(&program).expect_err("runtime must reject an out-of-range index");
    assert!(error.message.contains("out of bounds"));
}

#[test]
fn mutable_array_push_and_pop_use_mutable_bindings_and_explicit_options() {
    let source = r#"
fn main() {
    mutable values: Int[] = []
    push_mut(values, 4)
    push_mut(values, 8)
    popped = pop_mut(values)
    match popped {
        Some(value) => { print(value) }
        None => { print(0) }
    }
    print(len(values))
    empty = pop_mut(values)
    match empty {
        Some(value) => { print(value) }
        None => { print(0) }
    }
}
"#;
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    assert_eq!(execute(&program).unwrap(), ["8", "1", "4"]);

    let immutable = "fn main() { values = [1] push_mut(values, 2) }";
    let program = parse(&lex(immutable).unwrap()).unwrap();
    let errors = check(&program).expect_err("push_mut must require an explicit mutable binding");
    assert!(errors
        .iter()
        .any(|error| error.message.contains("mutable array binding")));

    let mismatch = "fn main() { mutable values: Int[] = [] push_mut(values, \"wrong\") }";
    let program = parse(&lex(mismatch).unwrap()).unwrap();
    assert!(
        check(&program).is_err(),
        "push_mut must preserve the element type"
    );
}

#[test]
fn snapshot_lambdas_reject_mutation_of_captured_bindings_statically() {
    let sources = [
        r#"
fn main() {
    mutable values: Int[] = []
    append_value: Function<(Int) -> Unit> = |value| push_mut(values, value)
}
"#,
        r#"
fn main() {
    mutable values: Int[] = []
    mapped = map([1], |value| push_mut(values, value))
}
"#,
        r#"
fn main() {
    mutable values: Int[] = [1]
    pop_value: Function<(Int) -> Int?> = |unused| pop_mut(values)
}
"#,
    ];

    for source in sources {
        let program = parse(&lex(source).unwrap()).unwrap();
        let errors = check(&program)
            .expect_err("snapshot closures cannot mutate bindings in their creator scope");
        assert!(
            errors.iter().any(|error| {
                error
                    .message
                    .contains("cannot assign to immutable variable `current`")
                    || error.message.contains("mutable array binding")
            }),
            "expected a static captured-mutation error, got: {errors:?}"
        );
    }
}

#[test]
fn render_pdf_uses_named_views_record_data_and_escapes_interpolations() {
    let source = r#"
struct Invoice { number: String customer: String }
view InvoicePrint {
    html { <html><body><h1>Rechnung {number}</h1><p>{customer}</p><slot /></body></html> }
}

fn main() {
    invoice = Invoice { number: "R-104", customer: "<script>alert(1)</script>" }
    document = render_pdf("InvoicePrint", invoice)
    print(len(document))
}
"#;
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap_or_else(|errors| panic!("PDF view check failed: {errors:?}"));
    let output = execute(&program).unwrap();
    let length = output[0].parse::<usize>().unwrap();
    assert!(length > 500, "expected a non-empty PDF document");
}

#[test]
fn render_markdown_is_a_typed_pure_builtin() {
    let source = r##"
        fn main() {
            print(render_markdown("# Guide\n\n**safe**"))
        }
    "##;
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap_or_else(|errors| panic!("Markdown check failed: {errors:?}"));
    let output = execute(&program).unwrap();
    assert_eq!(
        output,
        ["<h1 id=\"guide\">Guide</h1>\n<p><strong>safe</strong></p>\n"]
    );

    let invalid = parse(&lex("fn main() { render_markdown(42) }").unwrap()).unwrap();
    assert!(check(&invalid).is_err());
}

#[test]
fn pdf_view_templates_expand_zelyra_collection_blocks() {
    let row = |description: &str, amount: &str| Value::Object {
        type_name: "InvoiceLine".into(),
        type_args: Vec::new(),
        fields: HashMap::from([
            ("description".into(), Value::String(description.into())),
            ("amount".into(), Value::String(amount.into())),
        ]),
    };
    let invoice = Value::Object {
        type_name: "Invoice".into(),
        type_args: Vec::new(),
        fields: HashMap::from([(
            "lines".into(),
            Value::Array(vec![row("Consulting", "125.00"), row("Hosting", "25.00")]),
        )]),
    };
    let html = "<table>\nfor line in lines {\n<tr><td>{line.description}</td><td>{line.amount}</td></tr>\n}\n</table>";
    let rendered = interpolate_pdf_data(html, &invoice).unwrap();
    assert!(rendered.contains("Consulting"));
    assert!(rendered.contains("Hosting"));
    assert!(!rendered.contains("for line in lines"));
    let pdf = pdf::render_html_to_pdf(&rendered).unwrap();
    assert!(pdf
        .windows(b"Consulting".len())
        .any(|window| window == b"Consulting"));
    assert!(pdf
        .windows(b"Hosting".len())
        .any(|window| window == b"Hosting"));
}

#[test]
fn pdf_view_markdown_interpolation_uses_the_safe_renderer() {
    let data = Value::Object {
        type_name: "Article".into(),
        type_args: Vec::new(),
        fields: HashMap::from([(
            "body".into(),
            Value::String("# Help\n\n<script>alert(1)</script>\n\n**safe**".into()),
        )]),
    };
    let rendered =
        interpolate_pdf_data("<article>{render_markdown:body}</article>", &data).unwrap();
    assert!(rendered.contains("<h1 id=\"help\">Help</h1>"));
    assert!(rendered.contains("<strong>safe</strong>"), "{rendered}");
    assert!(!rendered.contains("<script"));
    assert!(rendered.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
}

#[test]
fn pdf_view_template_expansion_has_deterministic_output_and_row_limits() {
    let html = "for row in rows {x}\n";
    let rows = Value::Array(vec![
        Value::String("row".into());
        pdf_template::MAX_PDF_TEMPLATE_ROWS + 1
    ]);
    let data = Value::Object {
        type_name: "Report".into(),
        type_args: Vec::new(),
        fields: HashMap::from([("rows".into(), rows)]),
    };
    assert_eq!(
        interpolate_pdf_data(html, &data).unwrap_err(),
        "PDF view exceeds its 10000 row rendering limit"
    );

    let large_value = Value::Object {
        type_name: "Report".into(),
        type_args: Vec::new(),
        fields: HashMap::from([("label".into(), Value::String("&".repeat(600_000)))]),
    };
    assert_eq!(
        interpolate_pdf_data("{label}", &large_value).unwrap_err(),
        pdf_template::PDF_TEMPLATE_LIMIT_ERROR
    );
}

#[test]
fn render_pdf_requires_an_existing_view_and_record_data() {
    let missing_view = parse(
        &lex("struct Invoice { number: String } fn main() { print(render_pdf(\"Missing\", Invoice { number: \"1\" })) }")
            .unwrap(),
    )
    .unwrap();
    let diagnostics = check(&missing_view).expect_err("unknown views should fail at compile time");
    assert!(diagnostics
        .iter()
        .any(|error| error.message.contains("unknown PDF view")));

    let invalid_data = parse(
        &lex("view Receipt { html { <body><slot /></body> } } fn main() { print(render_pdf(\"Receipt\", 42)) }")
            .unwrap(),
    )
    .unwrap();
    let diagnostics = check(&invalid_data).expect_err("PDF data must be a record or string map");
    assert!(diagnostics
        .iter()
        .any(|error| error.message.contains("record or Map<String, T>")));
}

#[test]
fn pdf_bytes_are_accepted_as_pure_transactional_outbox_attachments() {
    let source = r#"
struct Invoice { number: String }
view InvoicePrint { html { <html><body><h1>Invoice {number}</h1><slot /></body></html> } }
fn main() uses Email, Database(write) {
    invoice = Invoice { number: "R-105" }
    transaction {
        mail_queue("customer@example.test", "Invoice", "Attached", "invoice-R-105.pdf", render_pdf("InvoicePrint", invoice))
    }
}
"#;
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap_or_else(|errors| panic!("PDF mail outbox check failed: {errors:?}"));

    let arbitrary_bytes = parse(
        &lex(
            r#"fn main() uses Email, Database(write) {
                document = bytes("not a PDF")
                transaction {
                    mail_queue("customer@example.test", "Invoice", "Attached", "invoice.pdf", document)
                }
            }"#,
        )
        .unwrap(),
    )
    .unwrap();
    let errors = check(&arbitrary_bytes).expect_err("attachments must come from the PDF renderer");
    assert!(errors
        .iter()
        .any(|error| error.message.contains("produced directly by render_pdf")));
}

#[test]
fn lambdas_capture_values_and_collection_pipelines_map_filter_find_and_reduce() {
    let source = r#"
fn main() {
    factor = 3
    values = [1, 2, 3, 4]
    transformed = values |> filter(|n| n > 1) |> map(|n| n * factor)
    total = transformed |> reduce(0, |sum, n| sum + n)
    print(transformed[0])
    print(transformed[2])
    print(total)
    match transformed |> find(|n| n == 9) {
        Some(value) => { print(value) }
        None => { print(0) }
    }
}
"#;
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap_or_else(|errors| panic!("lambda type check failed: {errors:?}"));
    assert_eq!(execute(&program).unwrap(), ["6", "12", "27", "9"]);
}

#[test]
fn returned_lambdas_use_the_declared_function_type_for_body_checking() {
    let source = r#"
fn make_adder(base: Int) -> Function<(Int) -> Int> {
    return |value| value + base
}
fn main() {
    add = make_adder(40)
    print(add(2))
}
"#;
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap_or_else(|errors| panic!("returned lambda check failed: {errors:?}"));
    assert_eq!(execute(&program).unwrap(), ["42"]);

    let initialized_from_context = r#"
fn make_adder(base: Int) -> Function<(Int) -> Int> {
    mutable offset = base
    callback: Function<(Int) -> Int> = |value| value + offset
    offset = offset + 100
    return callback
}
fn main() {
    add = make_adder(20)
    print(add(2))
}
"#;
    let initialized_from_context = parse(&lex(initialized_from_context).unwrap()).unwrap();
    check(&initialized_from_context)
        .unwrap_or_else(|errors| panic!("typed lambda initializer check failed: {errors:?}"));
    assert_eq!(execute(&initialized_from_context).unwrap(), ["22"]);

    let assigned_from_context = r#"
fn make_adder(base: Int) -> Function<(Int) -> Int> {
    mutable offset = base
    mutable callback: Function<(Int) -> Int> = |value| value + offset
    offset = offset + 100
    callback = |value| value + offset
    return callback
}
fn main() {
    add = make_adder(20)
    print(add(2))
}
"#;
    let assigned_from_context = parse(&lex(assigned_from_context).unwrap()).unwrap();
    check(&assigned_from_context)
        .unwrap_or_else(|errors| panic!("typed lambda assignment check failed: {errors:?}"));
    assert_eq!(execute(&assigned_from_context).unwrap(), ["122"]);

    let invalid_initializer = r#"
fn main() {
    callback: Function<(Int) -> Int> = |value| "not an Int"
}
"#;
    let invalid_initializer = parse(&lex(invalid_initializer).unwrap()).unwrap();
    let errors = check(&invalid_initializer)
        .expect_err("typed lambda initializer body must match its declared function type");
    assert!(errors.iter().any(|error| {
        error.message.contains("type mismatch")
            && error.message.contains("expected `Int`")
            && error.message.contains("found `String`")
    }));

    let invalid = r#"
fn wrong_result() -> Function<(Int) -> Int> {
    return |value: Int| "not an Int"
}
fn main() {}
"#;
    let invalid = parse(&lex(invalid).unwrap()).unwrap();
    let errors = check(&invalid).expect_err("lambda body must match the declared return type");
    assert!(errors.iter().any(|error| {
        error.message.contains("type mismatch")
            && error.message.contains("expected `Int`")
            && error.message.contains("found `String`")
    }));

    let invalid_parameter = r#"
fn wrong_parameter() -> Function<(Int) -> Int> {
    return |value: String| 1
}
fn main() {}
"#;
    let invalid_parameter = parse(&lex(invalid_parameter).unwrap()).unwrap();
    let errors = check(&invalid_parameter)
        .expect_err("lambda parameter must match the declared function type");
    assert!(errors.iter().any(|error| {
        error.message.contains("type mismatch")
            && error.message.contains("expected `Int`")
            && error.message.contains("found `String`")
    }));
}

#[test]
fn typed_lambdas_must_declare_effects_used_by_their_bodies() {
    let source = r#"
fn current(unused: Int) -> Timestamp uses Clock { return now() }
fn db_read(unused: Int) -> Int uses Database(read) { return 1 }
fn missing_builtin_effect() uses Clock {
    callback: Function<(Int) -> Timestamp> = |unused| now()
}
fn missing_callee_effect() uses Clock {
    callback: Function<(Int) -> Timestamp> = |unused| current(unused)
}
fn declared_effect() uses Clock {
    callback: Function<(Int) -> Timestamp> uses {Clock} = |unused| now()
}
fn broad_database_effect() uses Database {
    callback: Function<(Int) -> Int> uses {Database} = |unused| db_read(unused)
}
fn main() {}
"#;
    let program = parse(&lex(source).unwrap()).unwrap();
    let errors =
        check(&program).expect_err("a function type must declare effects performed by its lambda");
    let effect_errors = errors
        .iter()
        .filter(|error| error.code == "E-TYPE-FUNCTION-001")
        .collect::<Vec<_>>();
    assert_eq!(effect_errors.len(), 2, "unexpected diagnostics: {errors:?}");
    assert!(effect_errors
        .iter()
        .all(|error| error.message.contains("requires capability `Clock`")));

    let valid = r#"
fn declared_effect() uses Clock {
    callback: Function<(Int) -> Timestamp> uses {Clock} = |unused| now()
}
fn main() {}
"#;
    let valid = parse(&lex(valid).unwrap()).unwrap();
    check(&valid).expect("explicitly declared lambda effects type-check");

    let broad_database = r#"
fn db_read(unused: Int) -> Int uses Database(read) { return 1 }
fn main() uses Database {
    callback: Function<(Int) -> Int> uses {Database} = |unused| db_read(unused)
}
"#;
    let broad_database = parse(&lex(broad_database).unwrap()).unwrap();
    check(&broad_database).expect("broad Database authority covers read-only use");
}

#[test]
fn higher_order_functions_accept_typed_function_values() {
    let source = "fn double(value: Int) -> Int { return value * 2 } fn main() { values = [2, 4] |> map(&double) print(values[0]) print(values[1]) }";
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    assert_eq!(execute(&program).unwrap(), ["4", "8"]);
}

#[test]
fn match_guards_and_nested_record_patterns_select_and_bind_fields() {
    let source = r#"
struct Order { status: String total: Int }
fn main() {
    order = Order { status: "paid", total: 42 }
    match order {
        Order { status: "paid", total } if total > 40 => { print(total) }
        Order { status: "paid", total } => { print(total + 1) }
        _ => { print(0) }
    }
    print(match order {
        Order { status: "paid", total } if total > 50 => total
        Order { status: "paid", total } => total
        _ => 0
    })
}
"#;
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap_or_else(|errors| panic!("record match type check failed: {errors:?}"));
    assert_eq!(execute(&program).unwrap(), ["42", "42"]);
}

#[test]
fn guarded_match_arm_does_not_prove_exhaustiveness() {
    let program = parse(
        &lex("fn main() { value = true match value { true if false => { print(1) } false => { print(0) } } }")
            .unwrap(),
    )
    .unwrap();
    let errors = check(&program).expect_err("a guarded true pattern can still fall through");
    assert!(errors.iter().any(|error| {
        error.code == "E-TYPE-MATCH-001" && error.message.contains("missing: true")
    }));
}

#[test]
fn record_updates_copy_values_and_contextual_field_names_work() {
    let source = r#"
struct Invoice {
    version: Int
    type: String
    status: Bool
}
fn main() {
    original = Invoice { version: 7, type: "sale", status: true }
    changed = Invoice { ..original, status: false }
    print(original.status)
    print(changed.version)
    print(changed.type)
    print(changed.status)
}
"#;
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    assert_eq!(execute(&program).unwrap(), ["true", "7", "sale", "false"]);
}

#[test]
fn tuples_work_as_function_returns_and_indexable_values() {
    let source = r#"
fn coordinates() -> Tuple<Int, String> { return (4, "A-12") }
fn main() {
    point = coordinates()
    print(point[0])
    print(point[1])
}
"#;
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    assert_eq!(execute(&program).unwrap(), ["4", "A-12"]);
}

#[test]
fn tuple_destructuring_binds_and_updates_variables_once() {
    let source = r#"
fn coordinates() -> Tuple<Int, String> { return (4, "A-12") }
fn main() {
    (x, caption) = coordinates()
    mutable current_x: Int = 0
    mutable current_label: String = ""
    (current_x, current_label) = coordinates()
    print(x)
    print(caption)
    print(current_x)
    print(current_label)
}
"#;
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    assert_eq!(execute(&program).unwrap(), ["4", "A-12", "4", "A-12"]);
}

#[test]
fn tuple_destructuring_rejects_immutable_targets_and_arity_mismatches() {
    let immutable = parse(&lex("fn main() { x = 0 (x, y) = (1, 2) }").unwrap()).unwrap();
    let immutable_error = check(&immutable).expect_err("an immutable target cannot be reassigned");
    assert!(immutable_error.iter().any(|error| error
        .message
        .contains("cannot assign to immutable variable `x`")));

    let wrong_arity = parse(&lex("fn main() { (x, y) = (1, 2, 3) }").unwrap()).unwrap();
    let arity_error = check(&wrong_arity).expect_err("tuple assignment arity must match");
    assert!(arity_error.iter().any(|error| error
        .message
        .contains("has 2 variables but the value has 3 items")));
}

#[test]
fn classes_share_identity_and_mutate_checked_fields() {
    let source = r#"
class Counter { value: Int }

fn increment(counter: Counter) {
    counter.value = counter.value + 1
}

fn identity(counter: Counter) -> Counter {
    return counter
}

fn main() {
    original = Counter { value: 0 }
    alias = original
    increment(original)
    returned = identity(alias)
    assert(original == alias, "binding copies preserve object identity")
    assert(returned == original, "function returns preserve object identity")
    assert(alias.value == 1, "field mutation is shared through aliases")
    fresh = Counter { value: 1 }
    assert(fresh != original, "distinct objects with equal fields have distinct identity")
    print("class references passed")
}
"#;
    assert_eq!(run(source), ["class references passed"]);
}

#[test]
fn class_references_are_rejected_at_json_api_boundaries() {
    let program = parse(
        &lex(
            "class Customer { name: String } struct CustomerEnvelope { customer: Customer } api GET \"/customers\" { output CustomerEnvelope } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    let errors = check_apis(&program)
        .expect_err("class identity must not be silently flattened into API JSON");
    assert!(errors.iter().any(|error| {
        error
            .message
            .contains("class references of type `Customer` cannot cross an API JSON boundary")
    }));

    let alias = parse(
        &lex(
            "class Customer { name: String } type CustomerRef = Customer api GET \"/customers\" { output CustomerRef } fn main() { }",
        )
        .unwrap(),
    )
    .unwrap();
    let errors = check_apis(&alias).expect_err("class aliases must retain the JSON boundary rule");
    assert!(errors.iter().any(|error| {
        error
            .message
            .contains("class references of type `Customer` cannot cross an API JSON boundary")
    }));
}

#[test]
fn sql_rows_keep_the_declared_result_type_for_runtime_return_checks() {
    let rows = Value::Rows {
        columns: vec!["id".into(), "name".into()],
        rows: vec![vec!["1".into(), "Ada".into()]],
        metadata: Box::new(SqlRowsMetadata {
            nulls: vec![vec![false, false]],
            column_types: vec![
                zelyra_database::QueryColumnType::Integer,
                zelyra_database::QueryColumnType::Text,
            ],
        }),
        result_type: Box::new(Type::Named("Customer".into())),
    };

    assert_eq!(rows.ty(), Type::Named("Customer".into()));
    assert_eq!(
        rows.output(),
        "[id=1, name=Ada]",
        "type metadata must not change the existing row representation"
    );
}

#[test]
fn generic_functions_infer_and_accept_explicit_type_arguments() {
    let source = r#"
fn identity<T>(value: T) -> T {
    return value
}

fn head_value<T>(values: T[]) -> T {
    return values[0]
}

fn wrap<T>(value: T) -> Option<T> {
    return Some(value)
}

fn result_or<T, E>(value: Result<T, E>, fallback: T) -> T {
    match value {
        Ok(found) => { return found }
        Err(_) => { return fallback }
    }
}

fn main() {
    print(identity(42))
    print(identity<String>("Grüße"))
    print(head_value([7, 8]))
    match wrap(true) {
        Some(value) => { print(value) }
        None => { print(false) }
    }
    print(result_or<Int, String>(Ok(7), 0))
}
"#;
    assert_eq!(run(source), ["42", "Grüße", "7", "true", "7"]);
}

#[test]
fn generic_functions_reject_conflicting_and_invalid_type_arguments() {
    let conflicting = parse(
        &lex("fn same<T>(left: T, right: T) -> T { return left } fn main() { print(same(1, \"one\")) }")
            .unwrap(),
    )
    .unwrap();
    let errors = check(&conflicting).expect_err("different types must not unify as one T");
    assert!(errors.iter().any(|error| {
        error
            .message
            .contains("type mismatch: expected `Int`, found `String`")
    }));

    let explicit = parse(
        &lex("fn identity<T>(value: T) -> T { return value } fn main() { print(identity<Int>(\"one\")) }")
            .unwrap(),
    )
    .unwrap();
    let errors = check(&explicit).expect_err("an explicit type argument must be enforced");
    assert!(errors.iter().any(|error| {
        error
            .message
            .contains("type mismatch: expected `Int`, found `String`")
    }));

    let uninferred = parse(
        &lex("fn ignored<T>(value: Int) -> Int { return value } fn main() { print(ignored(1)) }")
            .unwrap(),
    )
    .unwrap();
    let errors = check(&uninferred).expect_err("unconstrained type parameters need explicit types");
    assert!(errors.iter().any(|error| {
        error.code == "E-TYPE-GENERIC-001"
            && error.message.contains("cannot infer type parameter `T`")
    }));

    let wrong_arity = parse(
        &lex("fn identity<T>(value: T) -> T { return value } fn main() { print(identity<Int, String>(1)) }")
            .unwrap(),
    )
    .unwrap();
    let errors = check(&wrong_arity).expect_err("generic type argument count must be checked");
    assert!(errors.iter().any(|error| {
        error.code == "E-TYPE-GENERIC-001"
            && error
                .message
                .contains("expects 1 type argument(s), found 2")
    }));

    let shadows_type = parse(
        &lex("struct Customer { id: Int } fn identity<Customer>(value: Customer) -> Customer { return value } fn main() { }")
            .unwrap(),
    )
    .unwrap();
    let errors = check(&shadows_type).expect_err("generic parameters may not shadow domain types");
    assert!(errors.iter().any(|error| {
        error.code == "E-TYPE-GENERIC-001" && error.message.contains("shadows a declared type")
    }));
}

#[test]
fn generic_records_infer_fields_update_copy_and_round_trip_json() {
    let source = r#"
struct Box<T> { value: T }
struct Pair<First, Second> { first: First second: Second }

fn main() {
    explicit: Box<Int> = Box<Int> { value: 7 }
    inferred = Box { value: "Grüße" }
    pair = Pair { first: 3 second: "drei" }
    print(explicit.value)
    print(inferred.value)
    print(pair.first)
    print(pair.second)
    changed = record_with(explicit, "value", 9)
    print(changed.value)
    print(explicit.value)
    encoded = json_encode(explicit)
    restored: Box<Int> = json_decode<Box<Int>>(encoded)
    print(restored.value)
    print(restored)
}
"#;
    assert_eq!(
        run(source),
        [
            "7",
            "Grüße",
            "3",
            "drei",
            "9",
            "7",
            "7",
            "Box<Int>{value=7}"
        ]
    );

    for (source, expected) in [
        (
            "struct Box<T> { value: T } fn main() { value: Box<Int> = Box<String> { value: \"wrong\" } }",
            "type mismatch",
        ),
        (
            "struct Box<T> { value: T } fn main() { value = Box<Int, String> { value: 1 } }",
            "expects 1 type argument(s), found 2",
        ),
        (
            "struct Pair<First, Second> { first: First second: Second } fn main() { value: Pair<Int> = Pair<Int> { first: 1 second: 2 } }",
            "expects 2 type argument(s), found 1",
        ),
        (
            "struct Box<T> { value: T } fn main() { value: Box = Box { value: 1 } }",
            "requires 1 type argument(s)",
        ),
        (
            "struct Box<T> { value: T } fn main() { value = Box<Int> { value: \"wrong\" } }",
            "type mismatch",
        ),
        (
            "struct Pair<T> { first: T second: T } fn main() { value = Pair { first: 1 second: \"wrong\" } }",
            "type mismatch",
        ),
        (
            "struct Box<T> { value: T } fn main() { value = Box { value: 1 } changed = record_with(value, \"value\", \"wrong\") }",
            "type mismatch",
        ),
        (
            "struct Empty<T> { marker: Int } fn main() { value = Empty { marker: 1 } }",
            "cannot infer type parameter `T`",
        ),
        (
            "struct Customer { name: String } fn main() { customer: Customer<Int> = Customer<Int> { name: \"Ada\" } }",
            "does not accept type arguments",
        ),
    ] {
        let program = parse(&lex(source).unwrap()).unwrap();
        let errors = check(&program).expect_err("invalid generic record use must be rejected");
        assert!(
            errors.iter().any(|error| error.message.contains(expected)),
            "expected {expected:?}, got {errors:?}"
        );
    }
}

#[test]
fn generic_enums_infer_payloads_match_exhaustively_and_round_trip_json() {
    let source = r#"
enum Maybe<T> { Some(T) None }

fn describe(value: Maybe<Int>) -> Int {
    match value {
        Maybe::Some(number) => { return number }
        Maybe::None => { return 0 }
    }
}

fn main() {
    some = Maybe::Some(42)
    none = Maybe::None<String>()
    print(describe(some))
    match none {
        Maybe::Some(text) => { print(text) }
        Maybe::None => { print("empty") }
    }
    encoded = json_encode(some)
    restored: Maybe<Int> = json_decode<Maybe<Int>>(encoded)
    print(restored == some)
}
"#;
    assert_eq!(run(source), ["42", "empty", "true"]);

    for (source, expected) in [
        (
            "enum Maybe<T> { Some(T) None } fn main() { value: Maybe<Int> = Maybe::Some(\"wrong\") }",
            "type mismatch",
        ),
        (
            "enum Maybe<T> { Some(T) None } fn main() { value = Maybe::None }",
            "requires explicit type arguments",
        ),
        (
            "enum Maybe<T> { Some(T) None } fn main() { value = Maybe::None<Int, String>() }",
            "expects 1 type argument(s), found 2",
        ),
        (
            "enum Maybe<T> { Some(T) None } fn main() { value: Maybe<Int> = Maybe::Some<Int, String>(1) }",
            "expects 1 type argument(s), found 2",
        ),
        (
            "enum Maybe<T> { Some(T) None } fn main() { value: Maybe<Int> = Maybe::Some<String>(\"wrong\") }",
            "type mismatch",
        ),
        (
            "enum Maybe<T> { Some(T) None } fn main() { value: Maybe<Int> = Maybe::Some(1) match value { Maybe::Some(number) => { print(number) } } }",
            "non-exhaustive match",
        ),
        (
            "enum Maybe<Int> { Some(Int) None } fn main() { }",
            "shadows a declared type",
        ),
    ] {
        let program = parse(&lex(source).unwrap()).unwrap();
        let errors = check(&program).expect_err("invalid generic enum use must be rejected");
        assert!(
            errors.iter().any(|error| error.message.contains(expected)),
            "expected {expected:?}, got {errors:?}"
        );
    }
}

#[test]
fn named_function_values_are_typed_passed_called_and_keep_capability_requirements() {
    let source = r#"
fn increment(value: Int) -> Int { return value + 1 }
fn provide_increment() -> Function<(Int) -> Int> { return &increment }
fn apply(callback: Function<(Int) -> Int>, value: Int) -> Int {
    return callback(value)
}
fn main() {
    saved = provide_increment()
    print(saved(4))
    print(apply(saved, 7))
    print(saved)
}
"#;
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();
    assert_eq!(
        execute(&program).unwrap(),
        ["5", "8", "<function increment>"]
    );

    let mismatch = parse(
        &lex("fn increment(value: Int) -> Int { return value + 1 } fn main() { saved = &increment print(saved(\"wrong\")) }")
            .unwrap(),
    )
    .unwrap();
    let errors = check(&mismatch).expect_err("indirect function arguments remain statically typed");
    assert!(errors.iter().any(|error| {
        error
            .message
            .contains("type mismatch: expected `Int`, found `String`")
    }));

    let hidden_effect = parse(
        &lex("fn read_clock() -> Timestamp uses Clock { return now() } fn apply(callback: Function<() -> Timestamp>) -> Timestamp { return callback() } fn main() { reader = &read_clock print(apply(reader)) }")
            .unwrap(),
    )
    .unwrap();
    let errors = check(&hidden_effect).expect_err("a function type may not hide a Clock effect");
    assert!(errors.iter().any(|error| {
        error.message.contains("type mismatch") && error.message.contains("Clock")
    }));

    let propagated_effect = parse(
        &lex("fn read_clock() -> Timestamp uses Clock { return now() } fn apply(callback: Function<() -> Timestamp> uses {Clock}) -> Timestamp { return callback() } fn main() uses Clock { reader = &read_clock print(apply(reader)) }")
            .unwrap(),
    )
    .unwrap();
    check(&propagated_effect).unwrap();
    let errors = check_capabilities(&propagated_effect)
        .expect_err("an indirect Clock call requires the containing function to declare it");
    assert!(errors.iter().any(|error| error.message.contains("Clock")));

    let authorized_effect = parse(
        &lex("fn read_clock() -> Timestamp uses Clock { return now() } fn apply(callback: Function<() -> Timestamp> uses {Clock}) -> Timestamp uses Clock { return callback() } fn main() uses Clock { reader = &read_clock print(apply(reader)) }")
            .unwrap(),
    )
    .unwrap();
    check(&authorized_effect).unwrap();
    check_capabilities(&authorized_effect).unwrap();
    assert_eq!(execute(&authorized_effect).unwrap().len(), 1);

    let encode = parse(
        &lex("fn increment(value: Int) -> Int { return value + 1 } fn provide() -> Function<(Int) -> Int> { return &increment } fn main() { saved = provide() print(json_encode(saved)) }")
            .unwrap(),
    )
    .unwrap();
    check(&encode).unwrap();
    let error = execute(&encode).expect_err("function values are not JSON data");
    assert!(error.message.contains("cannot serialize a function value"));
}

#[test]
fn format_text_is_an_explicit_scalar_conversion_without_json_quotes() {
    let source = r#"
fn main() {
    print(format_text(42))
    print(format_text(decimal("12.34")))
    print(format_text(true))
    print(format_text("Zelyra"))
}
"#;
    assert_eq!(run(source), ["42", "12.34", "true", "Zelyra"]);

    let non_scalar = parse(&lex("fn main() { print(format_text([1, 2])) }").unwrap()).unwrap();
    let errors =
        check(&non_scalar).expect_err("collections do not have an implicit display format");
    assert!(errors.iter().any(|error| {
        error.code == "E-TYPE-FORMAT-001" && error.message.contains("does not support `Int[]`")
    }));
}

#[test]
fn long_left_associative_expressions_are_checked_and_evaluated_without_stack_overflow() {
    let expression = std::iter::repeat_n("1", 5_000)
        .collect::<Vec<_>>()
        .join(" + ");
    let source = format!("fn main() {{ print({expression}) }}");
    let program = parse(&lex(&source).unwrap()).unwrap();
    check(&program).unwrap();
    let result = execute(&program).unwrap();
    assert_eq!(result, ["5000"]);
}

#[test]
fn calendar_timestamp_and_decimal_helpers_are_checked_and_lossless() {
    let source = r#"
fn main() {
    match date_add_days_result(date("2024-02-28"), 1) {
        Ok(value) => { print(value) }
        Err(reason) => { print(reason) }
    }
    match date_add_days_result(date("2024-02-28"), 2) {
        Ok(value) => { print(value) }
        Err(reason) => { print(reason) }
    }
    match date_days_between_result(date("2023-12-31"), date("2024-01-02")) {
        Ok(value) => { print(value) }
        Err(reason) => { print(reason) }
    }
    match date_add_days_result(date("9999-12-31"), 1) {
        Ok(value) => { print(value) }
        Err(reason) => { print("date-range-error") }
    }
    first: Timestamp = json_decode<Timestamp>("1704067200000")
    later: Timestamp = json_decode<Timestamp>("1704067200001")
    print(first < later)
    match timestamp_add_result(first, duration_ms(1)) {
        Ok(value) => { print(value == later) }
        Err(reason) => { print(reason) }
    }
    match timestamp_difference_result(later, first) {
        Ok(value) => { print(value) }
        Err(reason) => { print(reason) }
    }
    match decimal_quantize_result(decimal("1.245"), 2, "half_even") {
        Ok(value) => { print(value) }
        Err(reason) => { print(reason) }
    }
    match decimal_quantize_result(decimal("-1.245"), 2, "half_up") {
        Ok(value) => { print(value) }
        Err(reason) => { print(reason) }
    }
    match decimal_quantize_result(decimal("-1.231"), 2, "floor") {
        Ok(value) => { print(value) }
        Err(reason) => { print(reason) }
    }
}
"#;
    assert_eq!(
        run(source),
        [
            "2024-02-29",
            "2024-03-01",
            "2",
            "date-range-error",
            "true",
            "true",
            "1ms",
            "1.24",
            "-1.25",
            "-1.24"
        ]
    );
}

#[test]
fn decimal_quantize_rejects_invalid_scale_and_rounding_mode() {
    assert!(quantize_decimal(1, 10, "half_up").is_err());
    assert!(quantize_decimal(1, 2, "bankers_maybe").is_err());
    assert_eq!(
        quantize_decimal(1_245_000_000, 2, "half_even").unwrap(),
        1_240_000_000
    );
    assert_eq!(
        quantize_decimal(1_255_000_000, 2, "half_even").unwrap(),
        1_260_000_000
    );
    assert_eq!(
        quantize_decimal(-1_245_000_000, 2, "half_even").unwrap(),
        -1_240_000_000
    );
    assert_eq!(
        quantize_decimal(-1_245_000_000, 2, "half_up").unwrap(),
        -1_250_000_000
    );
}

#[test]
fn exclusive_integer_ranges_handle_empty_bounds_break_and_continue() {
    let source = r#"
fn main() {
    mutable total = 0
    for position in -2..3 {
        if position == 0 { continue }
        total = total + position
    }
    print(total)
    for position in 5..2 { print(position) }
    for position in 3..8 {
        if position == 5 { break }
        print(position)
    }
    print(range(2, 5))
}
"#;
    assert_eq!(run(source), ["0", "3", "4", "[2, 3, 4]"]);
}

#[test]
fn stepped_integer_ranges_support_ascending_descending_and_array_values() {
    let source = r#"
fn main() {
    mutable total = 0
    for position in 1..21 step 2 {
        total = total + position
    }
    print(total)
    for position in 10..0 step -3 { print(position) }
    for position in 1..10 step -1 { print(position) }
    print(range(1, 10, 3))
    print(range(5, 0, -2))
    mutable stride = 4
    for position in 0..10 step stride { print(position) }
    for position in 9223372036854775806..9223372036854775807 step 2 { print(position) }
    for position in -9223372036854775807..-9223372036854775808 step -2 { print(position) }
}
"#;
    assert_eq!(
        run(source),
        [
            "100",
            "10",
            "7",
            "4",
            "1",
            "[1, 4, 7]",
            "[5, 3, 1]",
            "0",
            "4",
            "8",
            "9223372036854775806",
            "-9223372036854775807"
        ]
    );
}

#[test]
fn stepped_integer_ranges_reject_zero_steps_and_non_int_steps() {
    let zero_step =
        parse(&lex("fn main() { for position in 1..5 step 0 { print(position) } }").unwrap())
            .unwrap();
    check(&zero_step).expect("an Int step has a valid static type");
    let error = execute(&zero_step).expect_err("a zero step must not create an infinite loop");
    assert!(error.message.contains("range step must not be zero"));

    let wrong_type =
        parse(&lex("fn main() { for position in 1..5 step \"two\" { print(position) } }").unwrap())
            .unwrap();
    let errors = check(&wrong_type).expect_err("the step must be an Int");
    assert!(errors
        .iter()
        .any(|error| error.message.contains("expected `Int`, found `String`")));
}

#[test]
fn value_semantic_array_and_record_updates_leave_originals_unchanged() {
    let source = r#"
struct Customer { name: String city: String }
fn main() {
    customer = Customer { name: "Ada" city: "Berlin" }
    updated = record_with(customer, "city", "Hamburg")
    print(customer.city)
    print(updated.city)
    numbers = [3, 4, 5]
    match array_replace_result(numbers, 1, 9) {
        Ok(changed) => { print(changed) }
        Err(reason) => { print(reason) }
    }
    print(numbers)
    match array_replace_result(numbers, 8, 9) {
        Ok(changed) => { print(changed) }
        Err(reason) => { print("bounds-error") }
    }
}
"#;
    assert_eq!(
        run(source),
        [
            "Berlin",
            "Hamburg",
            "[3, 9, 5]",
            "[3, 4, 5]",
            "bounds-error"
        ]
    );

    let wrong_field_type = parse(
        &lex(r#"struct Customer { city: String } fn main() { customer = Customer { city: "X" } changed = record_with(customer, "city", 1) }"#).unwrap(),
    )
    .unwrap();
    assert!(check(&wrong_field_type).is_err());
}

#[test]
fn unit_enums_support_typed_values_exhaustive_matches_and_json_round_trips() {
    let source = r#"
enum OrderStatus { Draft Pending Paid }
fn main() {
    status: OrderStatus = OrderStatus::Paid
    match status {
        OrderStatus::Draft => { print("draft") }
        OrderStatus::Pending => { print("pending") }
        OrderStatus::Paid => { print("paid") }
    }
    encoded = json_encode(status)
    print(encoded)
    restored: OrderStatus = json_decode<OrderStatus>(encoded)
    print(restored == status)
}
"#;
    assert_eq!(
        run(source),
        [
            "paid",
            r#"{"$enum":"OrderStatus","variant":"Paid"}"#,
            "true"
        ]
    );

    let incomplete = parse(
        &lex("enum Status { Draft Paid } fn main() { status: Status = Status::Draft match status { Status::Draft => { print(1) } } }").unwrap(),
    )
    .unwrap();
    let errors = check(&incomplete).expect_err("an enum match must cover every declared variant");
    assert!(errors
        .iter()
        .any(|error| error.message.contains("non-exhaustive match")
            && error.message.contains("Status::Paid")));
    assert!(errors.iter().any(|error| error.code == "E-TYPE-MATCH-001"));

    let unknown =
        parse(&lex("enum Status { Draft } fn main() { status = Status::Missing }").unwrap())
            .unwrap();
    assert!(
        check(&unknown).is_err(),
        "unknown enum variants must be rejected"
    );
}

#[test]
fn payload_enums_typecheck_match_serialize_and_round_trip() {
    let source = r#"
enum Event { Created(Int) Renamed(String, Date) Idle }

fn describe(event: Event) -> String {
    match event {
        Event::Created(id) => { return "created" }
        Event::Renamed(name, date) => { return "renamed" }
        Event::Idle => { return "idle" }
    }
}

fn main() {
    created = Event::Created(41)
    renamed = Event::Renamed("Lager", date("2026-10-12"))
    idle = Event::Idle
    assert(created != idle, "payload variants remain distinct")
    assert(describe(created) == "created", "payload is bound by pattern")
    assert(describe(renamed) == "renamed", "multiple payloads are bound")
    assert(describe(idle) == "idle", "unit variants remain compatible")
    encoded = json_encode(renamed)
    print(encoded)
    restored: Event = json_decode<Event>(encoded)
    assert(restored == renamed, "payload enum JSON round trip")
    print(restored)
}
"#;
    assert_eq!(
        run(source),
        [
            r#"{"$enum":"Event","payload":["Lager","2026-10-12"],"variant":"Renamed"}"#,
            "Event::Renamed(Lager, 2026-10-12)",
        ]
    );

    for (source, code) in [
        (
            "enum Event { Created(Int) Idle } fn main() { event = Event::Created(\"wrong\") }",
            "E-TYPE-001",
        ),
        (
            "enum Event { Created(Int) Idle } fn main() { event = Event::Created() }",
            "E-TYPE-ENUM-001",
        ),
        (
            "enum Event { Created(Int) Idle } fn main() { event = Event::Idle() }",
            "E-TYPE-ENUM-001",
        ),
        (
            "enum Event { Created(Int) Idle } fn main() { event = Event::Created(1) match event { Event::Created(value) => { print(value) } } }",
            "E-TYPE-MATCH-001",
        ),
        (
            "enum Event { Created(Int) Idle } fn main() { event: Event = Event::Created(1) match event { Event::Created => { print(1) } Event::Idle => { print(0) } } }",
            "E-TYPE-ENUM-001",
        ),
    ] {
        let program = parse(&lex(source).unwrap()).unwrap();
        let errors = check(&program).expect_err("invalid payload enum use must be rejected");
        assert!(errors.iter().any(|error| error.code == code), "{errors:?}");
    }

    let decode = parse(
        &lex("enum Event { Created(Int) } fn decode(text: String) -> Event { return json_decode<Event>(text) } fn main() { }")
            .unwrap(),
    )
    .unwrap();
    check(&decode).unwrap();
    let error = execute_function(
        &decode,
        "decode",
        vec![Value::String(
            r#"{"$enum":"Event","variant":"Created","payload":["wrong"]}"#.into(),
        )],
        None,
    )
    .expect_err("payload JSON must be decoded against the variant field type");
    assert!(error.message.contains("expected Int"), "{}", error.message);
}

#[test]
fn opt_in_step_budget_counts_expression_evaluations_and_reports_the_exact_span() {
    let source = "fn main() { value = 1 + 2 }";
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    let policy = RuntimePolicy {
        step_budget: Some(3),
        ..RuntimePolicy::default()
    };
    let error = execute_with_capabilities_and_policies(&program, None, Some(&policy))
        .expect_err("the expression node budget must stop the fourth evaluation step");
    assert!(error.message.contains("execution step budget exceeded"));
    assert_eq!(error.span.start, source.rfind('2').unwrap());

    let program = parse(&lex("fn main() { loop { } }").unwrap()).unwrap();
    check(&program).unwrap();
    let policy = RuntimePolicy {
        step_budget: Some(25),
        ..RuntimePolicy::default()
    };
    let error = execute_with_capabilities_and_policies(&program, None, Some(&policy))
        .expect_err("the activated step budget should stop an empty loop");
    assert!(error.message.contains("step budget exceeded"));
    assert!(error.span.start > 0);

    let finite = parse(&lex("fn main() { for value in 0..3 { print(value) } }").unwrap()).unwrap();
    let output = execute_with_capabilities_and_policies(&finite, None, Some(&policy)).unwrap();
    assert_eq!(output, ["0", "1", "2"]);
}

#[test]
fn module_setting_reads_an_explicitly_typed_source_scoped_value() {
    let program = parse(
        &lex(
            "fn timeout() -> Int { return module_setting<Int>(\"api_timeout\") } fn main() { print(timeout()) }",
        )
        .unwrap(),
    )
    .unwrap();
    check(&program).unwrap();
    let mut settings = PackageSettings::default();
    settings.insert_json(0, "api_timeout", "45".into()).unwrap();
    let policy = RuntimePolicy {
        package_settings: Some(settings),
        ..RuntimePolicy::default()
    };
    assert_eq!(
        execute_with_capabilities_and_policies(&program, None, Some(&policy)).unwrap(),
        ["45"]
    );

    let mut tokens = lex("fn main() { print(module_setting<Int>(\"api_timeout\")) }").unwrap();
    for token in &mut tokens {
        token.span.source_id = 1;
    }
    let wrong_source = parse(&tokens).unwrap();
    check(&wrong_source).unwrap();
    let error = execute_with_capabilities_and_policies(&wrong_source, None, Some(&policy))
        .expect_err("a different source must not inherit another source's settings");
    assert!(error.message.contains("unavailable for this source"));
}

#[test]
fn captured_execution_preserves_output_before_runtime_failure() {
    let source = "fn main() { print(\"before failure\") loop { } }";
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    let policy = RuntimePolicy {
        step_budget: Some(30),
        ..RuntimePolicy::default()
    };

    let outcome = execute_captured_with_capabilities_and_policies(&program, None, Some(&policy));

    assert_eq!(outcome.output, ["before failure"]);
    let error = outcome.error.expect("the step budget must stop the loop");
    assert!(error.message.contains("execution step budget exceeded"));
}

#[test]
fn configured_call_depth_can_raise_the_default_within_the_runtime_policy() {
    let source = "fn descend(n: Int) -> Int { if n == 0 { return 0 } return descend(n - 1) + 1 } fn main() { print(descend(24)) }";
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    let policy = RuntimePolicy {
        max_stack_depth: Some(40),
        ..RuntimePolicy::default()
    };
    assert_eq!(
        execute_with_capabilities_and_policies(&program, None, Some(&policy)).unwrap(),
        ["24"]
    );
    let error = execute(&program).expect_err("the conservative default remains in force");
    assert!(error.message.contains("maximum 20"));
}

#[test]
fn id_aliases_reject_integer_literals_but_integer_domain_aliases_remain_transparent() {
    let identity = parse(
        &lex("type OrderId = Id fn load(id: OrderId) -> OrderId { return id } fn main() { print(load(501)) }")
            .unwrap(),
    )
    .unwrap();
    let errors = check(&identity).expect_err("an integer literal cannot become a database Id");
    assert!(errors.iter().any(|error| {
        error.code == "E-TYPE-001"
            && error.message.contains("expected `OrderId`")
            && error.message.contains("found `Int`")
    }));

    let ordinary = parse(
        &lex("type OrderNumber = Int fn load(number: OrderNumber) -> OrderNumber { return number } fn main() { print(load(501)) }")
            .unwrap(),
    )
    .unwrap();
    check(&ordinary).expect("ordinary integer domain aliases accept integer literals");
}

#[test]
fn transaction_requires_a_database_runtime_only_when_executed() {
    let program = parse(&lex("fn main() { transaction { value = 1 } }").unwrap()).unwrap();
    check(&program).expect("transaction syntax and types can be checked offline");
    let error = execute(&program).expect_err("executing a transaction needs a database runtime");
    assert!(error
        .message
        .contains("transaction execution requires a database runtime"));
}

#[test]
fn api_error_payload_requires_a_result_output() {
    let missing_result = parse(
        &lex("struct Problem { message: String } api GET \"/items\" { output String errors { 400 BadRequest: Problem } } fn main() { }")
            .unwrap(),
    )
    .unwrap();
    let errors = check_apis(&missing_result).expect_err("payload errors require Result output");
    assert!(errors.iter().any(|error| {
        error
            .message
            .contains("declares a payload but API output is not a Result")
    }));

    let result_output = parse(
        &lex("struct Problem { message: String } api GET \"/items\" { output Result<String, Problem> errors { 400 BadRequest: Problem } } fn main() { }")
            .unwrap(),
    )
    .unwrap();
    check_apis(&result_output).expect("the Result error type must match its declared payload");
}

#[test]
fn api_table_validation_requires_safe_boundary_and_matching_contract() {
    let valid = parse(
        &lex(r#"
            table invoices { id: Id primary auto amount: Int required validate { amount > 0: "Amount must be positive" } }
            api POST "/invoices" { handler create_invoice validates invoices input { amount: Int } output Result<Int, String> errors { 422 ValidationError: String } }
            fn create_invoice(amount: Int) -> Result<Int, String> { return Ok(amount) }
            fn main() { }
        "#).unwrap(),
    ).unwrap();
    check_apis(&valid).expect("well-formed unscoped API validation binding");

    let scoped = parse(
        &lex(r#"
            type TenantId = Id
            auth users { table: users memberships: tenant_memberships }
            table invoices scoped by tenant_id: TenantId { id: Id primary auto tenant_id: TenantId required amount: Int required validate { amount > 0: "Invalid amount" } }
            api POST "/invoices" { requires auth handler create_invoice validates invoices input { amount: Int } output Result<Int, String> errors { 422 ValidationError: String } }
            fn create_invoice(amount: Int, tenant_id: TenantId) -> Result<Int, String> { return Ok(amount) }
            fn main() { }
        "#).unwrap(),
    ).unwrap();
    check_apis(&scoped).expect("authenticated APIs receive verified tenant scope separately");

    let forged_scope = parse(
        &lex(r#"
            type TenantId = Id
            auth users { table: users memberships: tenant_memberships }
            table invoices scoped by tenant_id: TenantId { id: Id primary auto tenant_id: TenantId required amount: Int required validate { amount > 0: "Invalid amount" } }
            api POST "/invoices" { requires auth handler create_invoice validates invoices input { tenant_id: TenantId amount: Int } output Result<Int, String> errors { 422 ValidationError: String } }
            fn create_invoice(tenant_id: TenantId, amount: Int) -> Result<Int, String> { return Ok(amount) }
            fn main() { }
        "#).unwrap(),
    ).unwrap();
    let errors = check_apis(&forged_scope).expect_err("client tenant keys must be rejected");
    assert!(errors
        .iter()
        .any(|error| error.message.contains("must not accept tenant key")));

    let missing_auth = parse(
        &lex(r#"
            type TenantId = Id
            auth users { table: users memberships: tenant_memberships }
            table invoices scoped by tenant_id: TenantId { id: Id primary auto tenant_id: TenantId required amount: Int required validate { amount > 0: "Invalid amount" } }
            api POST "/invoices" { handler create_invoice validates invoices input { amount: Int } output Result<Int, String> errors { 422 ValidationError: String } }
            fn create_invoice(amount: Int, tenant_id: TenantId) -> Result<Int, String> { return Ok(amount) }
            fn main() { }
        "#).unwrap(),
    ).unwrap();
    let errors = check_apis(&missing_auth).expect_err("scoped validation requires authentication");
    assert!(errors
        .iter()
        .any(|error| error.message.contains("requires `requires auth`")));

    let missing_contract = parse(
        &lex(r#"
            table invoices { amount: Int required validate { amount > 0: "Invalid amount" } }
            api POST "/invoices" { validates invoices input { amount: Int } output Int }
            fn main() { }
        "#)
        .unwrap(),
    )
    .unwrap();
    let errors = check_apis(&missing_contract).expect_err("422 error contract is required");
    assert!(errors
        .iter()
        .any(|error| error.message.contains("422 ValidationError")));
}

#[test]
fn declared_non_unit_return_types_require_all_reachable_paths_to_return() {
    for source in [
        "fn answer() -> Int { } fn main() { }",
        "fn answer(flag: Bool) -> Int { if flag { return 7 } } fn main() { }",
        "fn answer(flag: Bool) -> Int { while flag { return 7 } } fn main() { }",
        "fn answer(values: Int[]) -> Int { for value in values { return value } } fn main() { }",
        "fn answer() -> Int { loop { break } } fn main() { }",
    ] {
        let program = parse(&lex(source).unwrap()).unwrap();
        let errors = check(&program).expect_err("a reachable function end must be rejected");
        let error = errors
            .iter()
            .find(|error| error.code == "E-TYPE-RETURN-001")
            .expect("missing-return diagnostic should have its stable code");
        assert!(error.message.contains("can reach its end"));
        assert!(error.span.start > 0);
    }

    for source in [
        "fn answer(flag: Bool) -> Int { if flag { return 7 } else { return 9 } } fn main() { print(answer(true)) }",
        "fn answer(flag: Bool) -> Int { match flag { true => { return 7 } false => { return 9 } } } fn main() { print(answer(false)) }",
        "fn answer(value: Option<Int>) -> Int { match value { Some(number) => { return number } None => { return 0 } } } fn main() { print(answer(None)) }",
        "fn answer() -> Int { loop { return 7 } } fn main() { print(answer()) }",
        "fn answer() -> Int { while true { return 7 } } fn main() { print(answer()) }",
    ] {
        let program = parse(&lex(source).unwrap()).unwrap();
        check(&program).unwrap_or_else(|errors| panic!("valid control flow rejected: {errors:?}"));
    }
}

#[test]
fn runtime_defensively_rejects_invalid_non_unit_returns() {
    for (source, expected) in [
        ("fn answer() -> Int { } fn main() { }", "reached its end"),
        (
            "fn answer() -> Int { return \"wrong\" } fn main() { }",
            "returned `String`, expected `Int`",
        ),
    ] {
        let program = parse(&lex(source).unwrap()).unwrap();
        let error = execute_function(&program, "answer", Vec::new(), None)
            .expect_err("invalid return must be caught even when the caller skips check");
        assert!(error.message.contains(expected), "{}", error.message);
    }
}

#[test]
fn assert_is_type_checked_and_fails_execution_with_message() {
    let passing =
        run("fn main() { assert(2 + 2 == 4) assert(true, \"expected true\") print(\"done\") }");
    assert_eq!(passing, ["done"]);

    let program =
        parse(&lex("fn main() { assert(false, \"inventory stock invariant\") }").unwrap()).unwrap();
    check(&program).unwrap();
    let error = execute(&program).expect_err("a failed assertion must fail the run");
    assert!(error.message.contains("inventory stock invariant"));

    let program = parse(&lex("fn main() { assert(1) }").unwrap()).unwrap();
    assert!(check(&program)
        .unwrap_err()
        .iter()
        .any(|error| error.message.contains("expected `Bool`, found `Int`")));
}

#[test]
fn match_expressions_bind_patterns_and_return_typed_values() {
    let output = run(
        "fn unwrap(value: Result<Int, String>) -> Int { return match value { Ok(number) => number Err(_) => 0 } } fn main() { answer = match true { true => unwrap(Ok(42)) false => 0 } print(answer) }",
    );
    assert_eq!(output, ["42"]);

    let program =
        parse(&lex("fn main() { answer = match true { true => 1 false => \"wrong\" } }").unwrap())
            .unwrap();
    assert!(check(&program)
        .unwrap_err()
        .iter()
        .any(|error| error.message.contains("expected `Int`, found `String`")));

    let program = parse(&lex("fn main() { answer = match true { true => 1 } }").unwrap()).unwrap();
    assert!(check(&program)
        .unwrap_err()
        .iter()
        .any(|error| error.message.contains("non-exhaustive match")));
}

#[test]
fn unsigned_integer_arithmetic_comparisons_and_overflow_are_consistent() {
    let output = run(
        "fn increment(value: UInt) -> UInt { return value + json_decode<UInt>(\"1\") } fn main() { one = json_decode<UInt>(\"1\") two = increment(one) maximum = json_decode<UInt>(\"18446744073709551615\") assert(two > one) assert(maximum > two) print(json_encode(two)) print(json_encode(maximum)) }",
    );
    assert_eq!(output, ["2", "18446744073709551615"]);

    for (source, expected) in [
        (
            "fn main() { value = json_decode<Int>(\"9223372036854775807\") + 1 }",
            "Int overflow",
        ),
        (
            "fn main() { value = json_decode<UInt>(\"18446744073709551615\") + json_decode<UInt>(\"1\") }",
            "UInt overflow",
        ),
        (
            "fn main() { value = json_decode<UInt>(\"0\") - json_decode<UInt>(\"1\") }",
            "UInt underflow",
        ),
        (
            "fn main() { value = json_decode<Int>(\"-9223372036854775808\") / -1 }",
            "Int overflow",
        ),
        (
            "fn main() { value = -json_decode<Int>(\"-9223372036854775808\") }",
            "Int overflow",
        ),
    ] {
        let program = parse(&lex(source).unwrap()).unwrap();
        check(&program).unwrap();
        let error = execute(&program).expect_err("overflow must return a runtime error");
        assert!(error.message.contains(expected), "{}", error.message);
    }
}

#[test]
fn int_minimum_literal_is_valid_but_out_of_range_values_remain_errors() {
    assert_eq!(
        run("fn main() { value: Int = -9223372036854775808 print(value) }").as_slice(),
        ["-9223372036854775808"]
    );
    for source in [
        "fn main() { value = 9223372036854775808 }",
        "fn main() { value = -9223372036854775809 }",
    ] {
        let lexed = lex(source);
        if let Ok(tokens) = lexed {
            let parsed = parse(&tokens);
            assert!(
                parsed.is_err(),
                "out-of-range expression was parsed: {source}"
            );
        } else {
            assert!(lexed.is_err());
        }
    }
    let source = "fn main() { value = -(-9223372036854775808) }";
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    let error = execute(&program).expect_err("negating Int::MIN must overflow");
    assert!(error.message.contains("Int overflow"));
    assert!(error.span.start > 0);
}

#[test]
fn text_trim_removes_unicode_whitespace_without_changing_interior_text() {
    let output = run("fn main() { print(text_trim(\"  Grüße\u{00a0}\")) }");
    assert_eq!(output, ["Grüße"]);

    let program = parse(&lex("fn main() { text_trim(5) }").unwrap()).unwrap();
    assert!(check(&program)
        .unwrap_err()
        .iter()
        .any(|error| error.message.contains("expected `String`, found `Int`")));
}

#[test]
fn typed_pipelines_check_and_execute_native_and_user_functions() {
    let source = r#"
fn normalize(text: String) -> String {
    return text_to_lower(text)
}

fn main() {
    print("  GRÜSSE  " |> trim |> normalize)
    print("a,b" |> text_split(","))
}
"#;
    assert_eq!(run(source), ["grüsse", "[a, b]"]);
}

#[test]
fn typed_pipelines_report_input_type_mismatches() {
    let program = parse(&lex("fn main() { print(10 |> text_trim) }").unwrap()).unwrap();
    let errors = check(&program).expect_err("pipeline arguments must satisfy stage signatures");

    assert!(errors
        .iter()
        .any(|error| { error.message.contains("expected `String`, found `Int`") }));
}

#[test]
fn text_chars_splits_unicode_scalars_into_strings() {
    let output = run("fn main() { chars = text_chars(\"Aä🧭\") print(len(chars)) print(chars[1]) print(chars[2]) }");
    assert_eq!(output, ["3", "ä", "🧭"]);

    let program = parse(&lex("fn main() { text_chars(false) }").unwrap()).unwrap();
    assert!(check(&program)
        .unwrap_err()
        .iter()
        .any(|error| error.message.contains("expected `String`, found `Bool`")));
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
fn compares_strings_lexicographically() {
    let output = run(
        "fn main() { print(\"Apfel\" < \"Birne\") print(\"Birne\" > \"Apfel\") print(\"Apfel\" <= \"Apfel\") }",
    );
    assert_eq!(output, ["true", "true", "true"]);
}

#[test]
fn formats_timestamps_as_utc_iso8601() {
    assert_eq!(format_timestamp_utc(0), "1970-01-01T00:00:00.000Z");
    assert_eq!(format_timestamp_utc(-1), "1969-12-31T23:59:59.999Z");
    let output = run("fn main() uses Clock { print(format_timestamp(now())) }");
    assert!(output[0].contains('T'));
    assert!(output[0].ends_with('Z'));
}

#[test]
fn result_apis_report_json_and_file_errors_without_aborting() {
    let program = parse(
        &lex(
            "fn main() uses FileSystem { match json_decode_result<Int>(\"kaputt\") { Ok(value) => { print(value) } Err(message) => { print(\"JSON: \" + message) } } match read_text_result(\"/zelyra-result-api-no-such-file/state.json\") { Ok(value) => { print(value) } Err(message) => { print(\"Datei: \" + message) } } match write_text_result(\"/zelyra-result-api-no-such-dir/state.json\", \"{}\") { Ok(_) => { print(\"write ok\") } Err(message) => { print(\"Schreiben: \" + message) } } }",
        )
        .unwrap(),
    )
    .unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();
    let grants = HashSet::from([String::from("FileSystem")]);
    let output = execute_with_capabilities(&program, Some(&grants)).unwrap();
    assert!(output[0].starts_with("JSON: json_decode received invalid JSON"));
    assert!(output[1].starts_with("Datei: file-system read failed"));
    assert!(output[2].starts_with("Schreiben: file-system write failed"));
}

#[test]
fn csv_and_fixed_point_builtins_preserve_data_and_report_errors() {
    let output = run(r#"fn main() {
            source_text = "number;name;note\n42;\"Äpfel; \"\"Bio\"\"\";\"erste Zeile\nzweite Zeile\""
            match csv_decode_result(source_text, ';') {
                Ok(rows) => {
                    print(len(rows))
                    print(rows[1][1])
                    print(rows[1][2])
                    match csv_encode(rows, ';') {
                        Ok(encoded) => { print(encoded == source_text) }
                        Err(message) => { print(message) }
                    }
                }
                Err(message) => { print(message) }
            }
            match decimal_scaled_result("12,34", 2) {
                Ok(cents) => {
                    print(cents)
                    match format_scaled_result(cents, 2) {
                        Ok(text) => { print(text) }
                        Err(message) => { print(message) }
                    }
                }
                Err(message) => { print(message) }
            }
            match decimal_scaled_result("1.234", 2) {
                Ok(_) => { print("unexpected precision success") }
                Err(message) => { print(message) }
            }
            match csv_decode_result("a;b\n\"unfinished", ';') {
                Ok(_) => { print("unexpected CSV success") }
                Err(message) => { print(message) }
            }
        }"#);
    assert_eq!(
        output,
        [
            "2",
            "Äpfel; \"Bio\"",
            "erste Zeile\nzweite Zeile",
            "true",
            "1234",
            "12.34",
            "at most 2 fractional digits are allowed",
            "unterminated quoted field at end of input near line 2",
        ]
    );
}

#[test]
fn executes_parallel_await_bindings_and_merges_results() {
    let output = run(
            "fn load_customer() -> String { return \"customer\" } fn load_orders() -> Int { return 3 } fn main() { parallel { customer = await load_customer() orders = await load_orders() } print(customer) print(orders) }",
        );
    assert_eq!(output, ["customer", "3"]);
}

#[test]
fn parallel_branch_limit_is_checked_before_ast_execution() {
    let four_branches = r#"
fn value(arg: Int) -> Int { return arg }
fn main() {
    parallel {
        first = await value(1)
        second = await value(2)
        third = await value(3)
        fourth = await value(4)
    }
    print(first + second + third + fourth)
}
"#;
    let program = parse(&lex(four_branches).unwrap()).unwrap();
    check(&program).expect("the shared maximum number of branches is supported");
    assert_eq!(execute(&program).unwrap(), ["10"]);

    let five_branches = r#"
fn value(arg: Int) -> Int { return arg }
fn main() {
    parallel {
        first = await value(1)
        second = await value(2)
        third = await value(3)
        fourth = await value(4)
        fifth = await value(5)
    }
    print(first + second + third + fourth + fifth)
}
"#;
    let program = parse(&lex(five_branches).unwrap()).unwrap();
    let errors = check(&program).expect_err("the type checker must cap spawned branches");
    assert!(errors.iter().any(|error| {
        error.code == "E-TYPE-001" && error.message.contains("maximum of 4 branches")
    }));
    let error = execute(&program).expect_err("direct AST execution also enforces the cap");
    assert!(error.message.contains("maximum of 4 branches"));
}

#[test]
fn parallel_await_bindings_share_the_global_step_budget() {
    let source = "fn load_customer() -> String { return \"customer\" } fn load_orders() -> Int { return 3 } fn main() { parallel { customer = await load_customer() orders = await load_orders() } }";
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    let policy = RuntimePolicy {
        step_budget: Some(4),
        ..RuntimePolicy::default()
    };
    let error = execute_with_capabilities_and_policies(&program, None, Some(&policy))
        .expect_err("parallel work must consume the same total step budget");
    assert!(error.message.contains("execution step budget exceeded"));
    assert_eq!(error.span.start, source.find("parallel").unwrap());
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
fn ast_sql_uses_the_borrowed_request_transaction_for_each_query() {
    let program =
        parse(&lex("fn update() uses Database(read) { sql<Int> { SELECT 1 } sql<Int> { SELECT 2 } } fn main() uses Database(read) { update() }").unwrap()).unwrap();
    check(&program).expect("the SQL expression is statically checked");

    let mut observed = Vec::new();
    let mut query_executor = |query: &zelyra_database::Query| {
        observed.push(query.sql.clone());
        Ok(zelyra_database::QueryResult {
            columns: vec!["value".into()],
            rows: vec![vec!["1".into()]],
            nulls: vec![vec![false]],
            column_types: vec![zelyra_database::QueryColumnType::Integer],
        })
    };
    let outcome = execute_function_captured_with_transactional_query(
        &program,
        "update",
        Vec::new(),
        "mysql://zelyra:zelyra@127.0.0.1/zelyra",
        None,
        None,
        &mut query_executor,
    );

    assert!(outcome.error.is_none(), "{:?}", outcome.error);
    assert_eq!(observed, [" SELECT 1 ", " SELECT 2 "]);
}

#[test]
fn guarded_action_runs_guard_and_handler_in_one_interpreter_and_short_circuits() {
    let program = parse(
        &lex(
            "fn guard(value: Int) -> Bool { print(\"guard\") return value > 0 } fn action(value: Int) -> Int { print(\"action\") return value + 1 } fn main() {}",
        )
        .unwrap(),
    )
    .unwrap();
    check(&program).expect("guard and handler must be valid typed functions");

    let mut query = |_query: &zelyra_database::Query| Ok(zelyra_database::QueryResult::default());
    let accepted = execute_guarded_function_captured_with_transactional_query(
        &program,
        "guard",
        "action",
        vec![Value::Int(0)],
        "mariadb://zelyra:zelyra@127.0.0.1/zelyra",
        None,
        None,
        &mut query,
    );
    assert!(accepted.error.is_none(), "{:?}", accepted.error);
    assert!(!accepted.guard_rejected);
    assert_eq!(accepted.value, Some(Value::Int(1)));
    assert_eq!(accepted.output, ["guard", "action"]);

    let rejected = execute_guarded_function_captured_with_transactional_query(
        &program,
        "guard",
        "action",
        vec![Value::Int(1)],
        "mariadb://zelyra:zelyra@127.0.0.1/zelyra",
        None,
        None,
        &mut query,
    );
    assert!(rejected.error.is_none(), "{:?}", rejected.error);
    assert!(rejected.guard_rejected);
    assert_eq!(rejected.value, None);
    assert_eq!(rejected.output, ["guard"]);
}

#[test]
fn borrowed_request_transaction_bridge_runs_transaction_blocks_in_the_outer_transaction() {
    let transaction_program = parse(
        &lex("table invoices { id: Id primary auto status: String required } fn main() uses Database(write) { transaction { sql { UPDATE invoices SET status = 'locked' WHERE id = 1 } } }").unwrap(),
    )
    .unwrap();
    check(&transaction_program).expect("nested SQL transaction is otherwise valid Zelyra");

    let parallel_program =
        parse(&lex("fn main() { parallel { result = await 1 } }").unwrap()).unwrap();
    check(&parallel_program).expect("parallel expression is statically valid");

    let mut observed = Vec::new();
    let mut query_executor = |query: &zelyra_database::Query| {
        observed.push(query.sql.clone());
        Ok(zelyra_database::QueryResult::default())
    };
    let outcome = execute_function_captured_with_transactional_query(
        &transaction_program,
        "main",
        Vec::new(),
        "mariadb://zelyra:zelyra@127.0.0.1/zelyra",
        None,
        None,
        &mut query_executor,
    );
    assert!(outcome.error.is_none(), "{:?}", outcome.error);
    assert_eq!(
        observed,
        [" UPDATE invoices SET status = 'locked' WHERE id = 1 "]
    );

    let mut query_count = 0;
    let mut query_executor = |_query: &zelyra_database::Query| {
        query_count += 1;
        Ok(zelyra_database::QueryResult::default())
    };
    let outcome = execute_function_captured_with_transactional_query(
        &parallel_program,
        "main",
        Vec::new(),
        "mariadb://zelyra:zelyra@127.0.0.1/zelyra",
        None,
        None,
        &mut query_executor,
    );
    assert!(outcome.error.as_ref().is_some_and(|error| error
        .message
        .contains("parallel execution is unavailable inside a borrowed database transaction")));
    assert_eq!(
        query_count, 0,
        "parallel must not escape the outer transaction"
    );
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
fn scoped_sql_guard_rejects_reads_writes_and_unverifiable_sql() {
    let scoped = vec!["invoices".to_owned()];
    assert!(check_scoped_sql("SELECT id FROM customers", &scoped).is_ok());
    for query in [
        "SELECT id FROM invoices",
        "SELECT i.id FROM customers c JOIN invoices i ON i.id = c.id",
        "UPDATE invoices SET status = 'paid'",
        "INSERT INTO invoices (id) VALUES (1)",
        "DELETE FROM invoices WHERE id = 1",
        "CREATE VIEW public_invoices AS SELECT id FROM invoices",
        "SELECT id FROM `invoices`",
    ] {
        assert!(
            check_scoped_sql(query, &scoped).is_err(),
            "scope guard accepted {query:?}"
        );
    }
    assert!(check_scoped_sql("SELECT id FROM invoices", &[]).is_ok());
}

#[test]
fn tenant_scope_requires_a_required_nominal_id_column() {
    let valid = parse(
        &lex("type TenantId = Id table invoices scoped by tenant_id: TenantId { id: Id primary auto tenant_id: TenantId required } fn main() {}")
            .unwrap(),
    )
    .unwrap();
    check(&valid).expect("matching nominal Id column is accepted");

    for source in [
        "type TenantId = String table invoices scoped by tenant_id: TenantId { id: Id primary auto tenant_id: TenantId required } fn main() {}",
        "type TenantId = Id table invoices scoped by tenant_id: TenantId { id: Id primary auto tenant_id: TenantId? } fn main() {}",
        "type TenantId = Id table invoices scoped by tenant_id: TenantId { id: Id primary auto owner_id: TenantId required } fn main() {}",
    ] {
        let program = parse(&lex(source).unwrap()).unwrap();
        let errors = check(&program).expect_err("invalid tenant scope must be rejected");
        assert!(
            errors.iter().any(|error| error.code == "E-TENANT-001"),
            "unexpected diagnostics for {source}: {errors:?}"
        );
    }
}

#[test]
fn table_validation_rules_are_typechecked_in_a_pure_table_field_scope() {
    let valid = r#"
        table invoices {
            invoice_date: Date required
            due_date: Date?
            validate {
                match due_date {
                    Some(due) => due >= invoice_date
                    None => true
                }: "Due date must not precede invoice date."
            }
        }
        fn main() {}
    "#;
    let program = parse(&lex(valid).unwrap()).unwrap();
    check(&program).expect("table field references and Option patterns are typed");
    check_capabilities(&program).expect("pure predicates require no extra capabilities");

    let invalid_condition = valid.replace("due >= invoice_date", "due + invoice_date");
    let program = parse(&lex(&invalid_condition).unwrap()).unwrap();
    let errors = check(&program).expect_err("a numeric result is not a Boolean invariant");
    assert!(errors.iter().any(|error| error.span.start > 0));

    let unknown_field = valid.replace("due >= invoice_date", "due >= issued_at");
    let program = parse(&lex(&unknown_field).unwrap()).unwrap();
    let errors = check(&program).expect_err("unknown table fields must not become globals");
    assert!(errors
        .iter()
        .any(|error| { error.span.start > 0 && error.message.contains("issued_at") }));

    let effectful = r#"
        fn network_predicate() -> Bool uses Network { return true }
        table invoices {
            amount: Int required
            validate { network_predicate(): "Not allowed" }
        }
        fn main() {}
    "#;
    let program = parse(&lex(effectful).unwrap()).unwrap();
    check(&program).expect("the helper call is type-correct");
    let errors = check_capabilities(&program)
        .expect_err("table predicates must not inherit undeclared capabilities");
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("capability")),
        "an effectful helper call must be rejected for a table predicate"
    );
}

#[test]
fn crud_row_policies_are_typechecked_against_table_fields() {
    let valid = parse(
        &lex(r#"
            table invoices { id: Id primary auto status: String required locked: Bool required }
            crud Invoice -> invoices {
                permits edit "invoices.manage" when status == "draft"
                guard delete when locked == true { error: "Locked invoices cannot be deleted." }
            }
            fn main() { }
        "#)
        .unwrap(),
    )
    .unwrap();
    check(&valid).expect("CRUD policy predicates must type-check against their row");

    let invalid = parse(
        &lex(r#"
            table invoices { status: String required }
            crud Invoice -> invoices { guard edit when status > 0 { error: "bad" } }
            fn main() { }
        "#)
        .unwrap(),
    )
    .unwrap();
    let errors = check(&invalid).expect_err("row policy with mismatched operand types must fail");
    assert!(errors.iter().any(|error| error.code == "E-TYPE-001"));
}

#[test]
fn table_validation_check_does_not_clone_unrelated_deep_function_bodies() {
    let expression = std::iter::repeat_n("1", 5_000)
        .collect::<Vec<_>>()
        .join(" + ");
    let source = format!(
        "table invoices {{ amount: Int required validate {{ amount > 0: \"Must be positive\" }} }} fn main() {{ print({expression}) }}"
    );
    let program = parse(&lex(&source).unwrap()).unwrap();
    check(&program).expect("the invariant and unrelated long expression are checked");
}

#[test]
fn ast_runtime_rejects_scoped_sql_before_database_access() {
    let source = "type TenantId = Id table invoices scoped by tenant_id: TenantId { id: Id primary auto tenant_id: TenantId required } fn main() uses Database(read) { rows = sql<Int[]> { SELECT id FROM invoices } }";
    let program = parse(&lex(source).unwrap()).unwrap();
    let error = execute(&program).expect_err("unscoped SQL must fail closed");
    assert!(error.message.contains("tenant-scoped table `invoices`"));
}

#[test]
fn database_queries_and_transactions_use_the_injected_host_after_capability_checks() {
    struct RecordingDatabaseHost {
        queries: std::sync::Mutex<Vec<String>>,
        batch_transactions: std::sync::Mutex<Vec<bool>>,
        calls: AtomicUsize,
    }

    impl HostServices for RecordingDatabaseHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            Ok(0)
        }

        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }

        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }

        fn execute_database_query(
            &self,
            database_url: &str,
            query: &zelyra_database::Query,
        ) -> Result<zelyra_database::QueryResult, String> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            assert_eq!(database_url, "memory://business-db");
            self.queries
                .lock()
                .map_err(|_| "query log lock poisoned".to_owned())?
                .push(query.sql.clone());
            Ok(zelyra_database::QueryResult::default())
        }

        fn execute_database_batch(
            &self,
            database_url: &str,
            queries: &[zelyra_database::Query],
            transaction: bool,
        ) -> Result<Vec<zelyra_database::QueryResult>, String> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            assert_eq!(database_url, "memory://business-db");
            self.batch_transactions
                .lock()
                .map_err(|_| "transaction log lock poisoned".to_owned())?
                .push(transaction);
            self.queries
                .lock()
                .map_err(|_| "query log lock poisoned".to_owned())?
                .extend(queries.iter().map(|query| query.sql.clone()));
            Ok(vec![zelyra_database::QueryResult::default(); queries.len()])
        }
    }

    let program = parse(
        &lex("fn read_data() uses Database(read) { rows = sql<Int> { SELECT 1 } } fn write_data() uses Database(write) { transaction { sql { UPDATE inventory SET quantity = 1 } } } fn main() {}")
            .unwrap(),
    )
    .unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();
    let host = Arc::new(RecordingDatabaseHost {
        queries: std::sync::Mutex::new(Vec::new()),
        batch_transactions: std::sync::Mutex::new(Vec::new()),
        calls: AtomicUsize::new(0),
    });
    let host_services: Arc<dyn HostServices> = host.clone();

    let denied = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "read_data",
        Vec::new(),
        Some("memory://business-db"),
        Some(&HashSet::new()),
        None,
        Arc::clone(&host_services),
    )
    .expect_err("database access must be denied before contacting the host");
    assert!(denied.message.contains("Database(read)"));
    assert_eq!(host.calls.load(Ordering::Relaxed), 0);

    let read_grants = HashSet::from(["Database(read)".to_owned()]);
    execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "read_data",
        Vec::new(),
        Some("memory://business-db"),
        Some(&read_grants),
        None,
        Arc::clone(&host_services),
    )
    .unwrap();
    assert_eq!(host.calls.load(Ordering::Relaxed), 1);

    let write_grants = HashSet::from(["Database(write)".to_owned()]);
    let denied_transaction = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "write_data",
        Vec::new(),
        Some("memory://business-db"),
        Some(&HashSet::new()),
        None,
        Arc::clone(&host_services),
    )
    .expect_err("transaction SQL must enforce database capabilities at runtime");
    assert!(denied_transaction.message.contains("Database(write)"));
    assert_eq!(host.calls.load(Ordering::Relaxed), 1);

    execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "write_data",
        Vec::new(),
        Some("memory://business-db"),
        Some(&write_grants),
        None,
        host_services,
    )
    .unwrap();
    assert_eq!(host.calls.load(Ordering::Relaxed), 2);
    assert_eq!(*host.batch_transactions.lock().unwrap(), [true]);
    assert_eq!(
        *host.queries.lock().unwrap(),
        [" SELECT 1 ", " UPDATE inventory SET quantity = 1 "]
    );
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
fn postgres_casts_are_not_treated_as_named_parameters() {
    let parameters = query_parameters(
        "SELECT count(*)::bigint, 1::int4",
        &Environment::new(),
        Span {
            start: 0,
            end: 0,
            line: 1,
            column: 1,
            source_id: 0,
        },
    )
    .unwrap();
    assert!(parameters.is_empty());
}

#[test]
fn postgres_runtime_dispatches_queries_by_connection_url() {
    let Ok(database_url) = std::env::var("ZELYRA_POSTGRES_RUNTIME_TEST_URL") else {
        return;
    };
    let program = parse(
        &lex(
            "fn main() uses Database { name = \"before\" transaction { sql { CREATE TEMP TABLE zelyra_runtime_language_probe (name TEXT) } sql { INSERT INTO zelyra_runtime_language_probe (name) VALUES (:name) } sql { UPDATE zelyra_runtime_language_probe SET name = 'after' WHERE name = :name } sql { DELETE FROM zelyra_runtime_language_probe WHERE name = 'after' } } rows = sql<Int> { SELECT count(*)::bigint FROM zelyra_runtime_language_probe } }",
        )
        .unwrap(),
    )
    .unwrap();
    execute_with_database(&program, &database_url).unwrap();
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
        mail: None,
        step_budget: None,
        max_stack_depth: None,
        package_settings: None,
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
fn injected_network_host_receives_only_policy_checked_requests() {
    struct NetworkHost {
        calls: AtomicUsize,
        response_body: Vec<u8>,
        response_headers: Vec<(String, String)>,
    }
    impl HostServices for NetworkHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            Ok(0)
        }
        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }
        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }
        fn http_request(
            &self,
            method: &str,
            url: &str,
            headers: &[(String, String)],
            body: Option<&[u8]>,
            timeout_ms: u64,
            max_response_bytes: usize,
        ) -> Result<HostHttpResponse, String> {
            assert_eq!(method, "POST");
            assert_eq!(url, "https://api.example.test/customers");
            assert_eq!(headers, [("x-request-id".into(), "test-7".into())]);
            assert_eq!(body, Some(b"payload".as_slice()));
            assert_eq!(timeout_ms, 321);
            assert_eq!(max_response_bytes, 8);
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(HostHttpResponse {
                status: 202,
                headers: self.response_headers.clone(),
                body: self.response_body.clone(),
            })
        }
    }

    let program = parse(
        &lex("fn request() -> HttpResponse uses Network { return http_request(\"POST\", \"https://api.example.test/customers\", [\"X-Request-ID: test-7\"], Some(\"payload\")) } fn main() {}")
            .unwrap(),
    )
    .unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();
    let host = Arc::new(NetworkHost {
        calls: AtomicUsize::new(0),
        response_body: b"accepted".to_vec(),
        response_headers: vec![("content-type".into(), "text/plain".into())],
    });
    let grants = HashSet::from(["Network".to_owned()]);
    let policy = RuntimePolicy {
        filesystem: None,
        network: Some(NetworkPolicy {
            allowed_hosts: vec!["api.example.test".into()],
            timeout_ms: 321,
            max_response_bytes: 8,
        }),
        process: None,
        mail: None,
        step_budget: None,
        max_stack_depth: None,
        package_settings: None,
    };
    let response = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "request",
        Vec::new(),
        None,
        Some(&grants),
        Some(&policy),
        host.clone(),
    )
    .unwrap();
    let Value::Object { fields, .. } = response else {
        panic!("expected structured HTTP response");
    };
    assert_eq!(fields.get("status"), Some(&Value::Int(202)));
    assert_eq!(fields.get("body"), Some(&Value::String("accepted".into())));
    assert_eq!(host.calls.load(Ordering::Relaxed), 1);

    let denied_grant = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "request",
        Vec::new(),
        None,
        Some(&HashSet::new()),
        Some(&policy),
        host.clone(),
    )
    .expect_err("the runtime must check Network before calling the provider");
    assert!(denied_grant.message.contains("Network"));
    assert_eq!(host.calls.load(Ordering::Relaxed), 1);

    let denied_host = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "request",
        Vec::new(),
        None,
        Some(&grants),
        Some(&RuntimePolicy {
            network: Some(NetworkPolicy {
                allowed_hosts: vec!["other.example.test".into()],
                timeout_ms: 321,
                max_response_bytes: 8,
            }),
            ..policy.clone()
        }),
        host.clone(),
    )
    .expect_err("the runtime must enforce the host allowlist before provider access");
    assert!(denied_host.message.contains("network access denied"));
    assert_eq!(host.calls.load(Ordering::Relaxed), 1);

    let zero_timeout = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "request",
        Vec::new(),
        None,
        Some(&grants),
        Some(&RuntimePolicy {
            network: Some(NetworkPolicy {
                allowed_hosts: vec!["api.example.test".into()],
                timeout_ms: 0,
                max_response_bytes: 8,
            }),
            ..policy.clone()
        }),
        host.clone(),
    )
    .expect_err("the runtime must reject unbounded network timeouts");
    assert!(zero_timeout.message.contains("positive timeout"));
    assert_eq!(host.calls.load(Ordering::Relaxed), 1);

    let oversized = Arc::new(NetworkHost {
        calls: AtomicUsize::new(0),
        response_body: b"too large".to_vec(),
        response_headers: Vec::new(),
    });
    let size_error = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "request",
        Vec::new(),
        None,
        Some(&grants),
        Some(&policy),
        oversized.clone(),
    )
    .expect_err("the runtime must enforce response limits for injected providers");
    assert!(size_error.message.contains("exceeds the 8 byte limit"));
    assert_eq!(oversized.calls.load(Ordering::Relaxed), 1);

    let oversized_headers = Arc::new(NetworkHost {
        calls: AtomicUsize::new(0),
        response_body: b"ok".to_vec(),
        response_headers: vec![("x-large".into(), "x".repeat(65_537))],
    });
    let header_error = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "request",
        Vec::new(),
        None,
        Some(&grants),
        Some(&policy),
        oversized_headers.clone(),
    )
    .expect_err("the runtime must enforce response-header limits on injected hosts");
    assert!(header_error
        .message
        .contains("headers exceed the 65536 byte limit"));
    assert_eq!(oversized_headers.calls.load(Ordering::Relaxed), 1);
    assert_eq!(HOST_SERVICES_API_VERSION, 8);
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
        mail: None,
        step_budget: None,
        max_stack_depth: None,
        package_settings: None,
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
        mail: None,
        step_budget: None,
        max_stack_depth: None,
        package_settings: None,
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
        mail: None,
        step_budget: None,
        max_stack_depth: None,
        package_settings: None,
    };
    let customer = execute_function_with_capabilities_and_policies(
        &program,
        "create",
        vec![
            Value::String(format!("http://127.0.0.1:{port}/customers")),
            Value::Object {
                type_name: "CustomerCreate".into(),
                type_args: Vec::new(),
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
        mail: None,
        step_budget: None,
        max_stack_depth: None,
        package_settings: None,
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
        mail: None,
        step_budget: None,
        max_stack_depth: None,
        package_settings: None,
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
fn tagged_result_json_codec_preserves_ok_err_and_reports_invalid_data() {
    let source = r#"
        struct Customer { name: String }
        fn encode(value: Result<Customer, String>) -> String {
            return json_encode_tagged_result(value)
        }
        fn decode(payload: String) -> Result<Result<Customer, String>, String> {
            return json_decode_tagged_result<Customer, String>(payload)
        }
        fn main() { }
    "#;
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();

    let customer = Value::Object {
        type_name: "Customer".into(),
        type_args: Vec::new(),
        fields: HashMap::from([("name".into(), Value::String("Zoë".into()))]),
    };
    let encoded_ok = execute_function(
        &program,
        "encode",
        vec![Value::Result(Ok(Box::new(customer.clone())))],
        None,
    )
    .unwrap();
    let Value::String(encoded_ok) = encoded_ok else {
        panic!("expected JSON text");
    };
    let json: serde_json::Value = serde_json::from_str(&encoded_ok).unwrap();
    assert_eq!(
        json,
        serde_json::json!({"$result":"ok", "value":{"name":"Zoë"}})
    );
    let decoded_ok =
        execute_function(&program, "decode", vec![Value::String(encoded_ok)], None).unwrap();
    assert_eq!(
        decoded_ok,
        Value::Result(Ok(Box::new(Value::Result(Ok(Box::new(customer))))))
    );

    let encoded_err = execute_function(
        &program,
        "encode",
        vec![Value::Result(Err(Box::new(Value::String(
            "blocked".into(),
        ))))],
        None,
    )
    .unwrap();
    let Value::String(encoded_err) = encoded_err else {
        panic!("expected JSON text");
    };
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&encoded_err).unwrap(),
        serde_json::json!({"$result":"err", "error":"blocked"})
    );
    let decoded_err =
        execute_function(&program, "decode", vec![Value::String(encoded_err)], None).unwrap();
    let expected_err = Value::Result(Ok(Box::new(Value::Result(Err(Box::new(Value::String(
        "blocked".into(),
    )))))));
    assert_eq!(decoded_err, expected_err);

    let malformed = execute_function(
        &program,
        "decode",
        vec![Value::String(r#"{"$result":"maybe"}"#.into())],
        None,
    )
    .unwrap();
    assert!(matches!(malformed, Value::Result(Err(_))));
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
fn shared_json_codec_handles_checked_record_definitions() {
    let span = zelyra_ast::Span::default();
    let record = zelyra_ast::RecordDef {
        name: "CustomerDto".into(),
        is_public: true,
        is_class: false,
        type_params: Vec::new(),
        fields: vec![
            zelyra_ast::RecordField {
                name: "name".into(),
                ty: Type::String,
                span,
            },
            zelyra_ast::RecordField {
                name: "id".into(),
                ty: Type::Int,
                span,
            },
        ],
        span,
    };
    let value = Value::Object {
        type_name: "CustomerDto".into(),
        type_args: Vec::new(),
        fields: std::collections::HashMap::from([
            ("name".into(), Value::String("Ada".into())),
            ("id".into(), Value::Int(7)),
        ]),
    };

    let json =
        encode_json_value_with_definitions(&value, std::slice::from_ref(&record), &[], &[], span)
            .unwrap();
    assert_eq!(json, r#"{"id":7,"name":"Ada"}"#);

    let decoded = decode_json_value(
        &json,
        &Type::Named("CustomerDto".into()),
        &[record],
        &[],
        &[],
        span,
    )
    .unwrap();
    assert_eq!(decoded, value);
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
        mail: None,
        step_budget: None,
        max_stack_depth: None,
        package_settings: None,
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
fn runs_allowlisted_piped_process_and_returns_structured_result() {
    let source = "fn run() -> ProcessResult uses Process { return run_process_piped(\"/usr/bin/cat\", [\"-\"], \"piped input\", 1000) } fn main() {}";
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();
    let grants = HashSet::from(["Process".to_owned()]);
    let policy = RuntimePolicy {
        process: Some(ProcessPolicy {
            allowed_commands: vec!["/usr/bin/cat".into()],
            timeout_ms: 2_000,
            max_output_bytes: 100,
        }),
        ..RuntimePolicy::default()
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
    assert_eq!(
        output,
        Value::Object {
            type_name: "ProcessResult".into(),
            type_args: Vec::new(),
            fields: HashMap::from([
                ("exit_code".into(), Value::Int(0)),
                ("stdout".into(), Value::String("piped input".into())),
                ("stderr".into(), Value::String(String::new())),
            ]),
        }
    );
}

#[test]
fn process_execution_uses_injected_host_after_runtime_policy_checks() {
    struct ProcessHost {
        calls: std::sync::atomic::AtomicUsize,
    }

    impl HostServices for ProcessHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            Ok(0)
        }

        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }

        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }

        fn run_process(
            &self,
            command: &str,
            arguments: &[String],
            timeout_ms: u64,
            max_output_bytes: usize,
        ) -> Result<String, String> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            assert_eq!(command, "approved-tool");
            assert_eq!(arguments, ["--format", "json"]);
            assert_eq!(timeout_ms, 250);
            assert_eq!(max_output_bytes, 80);
            Ok("host-output".into())
        }
    }

    let program = parse(
        &lex("fn run() -> String uses Process { return run_process(\"approved-tool\", [\"--format\", \"json\"]) } fn main() {}")
            .unwrap(),
    )
    .unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();
    let host = Arc::new(ProcessHost {
        calls: AtomicUsize::new(0),
    });
    let host_services: Arc<dyn HostServices> = host.clone();
    let grants = HashSet::from(["Process".to_owned()]);
    let policy = RuntimePolicy {
        filesystem: None,
        network: None,
        process: Some(ProcessPolicy {
            allowed_commands: vec!["approved-tool".into()],
            timeout_ms: 250,
            max_output_bytes: 80,
        }),
        mail: None,
        step_budget: None,
        max_stack_depth: None,
        package_settings: None,
    };

    let denied = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "run",
        Vec::new(),
        None,
        Some(&HashSet::new()),
        Some(&policy),
        Arc::clone(&host_services),
    )
    .expect_err("capability denial must happen before the injected process host");
    assert!(denied.message.contains("Process"));
    assert_eq!(host.calls.load(Ordering::Relaxed), 0);

    let result = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "run",
        Vec::new(),
        None,
        Some(&grants),
        Some(&policy),
        Arc::clone(&host_services),
    )
    .unwrap();
    assert_eq!(result, Value::String("host-output".into()));
    assert_eq!(host.calls.load(Ordering::Relaxed), 1);
}

#[test]
fn filesystem_builtins_use_the_injected_host_after_capability_checks() {
    struct VirtualFiles {
        files: std::sync::Mutex<std::collections::HashMap<String, String>>,
        calls: AtomicUsize,
        canonicalizations: AtomicUsize,
        metadata_calls: AtomicUsize,
        directory_calls: AtomicUsize,
        root: std::path::PathBuf,
    }

    impl HostServices for VirtualFiles {
        fn unix_time_millis(&self) -> Result<i64, String> {
            Ok(0)
        }

        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }

        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }

        fn canonicalize_path(
            &self,
            path: &std::path::Path,
            allow_missing: bool,
        ) -> Result<std::path::PathBuf, String> {
            self.canonicalizations.fetch_add(1, Ordering::Relaxed);
            SystemHostServices.canonicalize_path(path, allow_missing)
        }

        fn write_text(&self, path: &std::path::Path, content: &str) -> Result<(), String> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            let path = path
                .to_str()
                .ok_or_else(|| "virtual path must be UTF-8".to_owned())?;
            self.files
                .lock()
                .map_err(|_| "virtual filesystem lock poisoned".to_owned())?
                .insert(path.to_owned(), content.to_owned());
            Ok(())
        }

        fn read_text(&self, path: &std::path::Path) -> Result<String, String> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            let path = path
                .to_str()
                .ok_or_else(|| "virtual path must be UTF-8".to_owned())?;
            self.files
                .lock()
                .map_err(|_| "virtual filesystem lock poisoned".to_owned())?
                .get(path)
                .cloned()
                .ok_or_else(|| format!("virtual file `{path}` does not exist"))
        }

        fn file_metadata(&self, path: &std::path::Path) -> Result<HostFileMetadata, String> {
            self.metadata_calls.fetch_add(1, Ordering::Relaxed);
            if path == self.root {
                Ok(HostFileMetadata {
                    is_file: false,
                    is_directory: true,
                    length: 0,
                })
            } else {
                Ok(HostFileMetadata {
                    is_file: true,
                    is_directory: false,
                    length: 23,
                })
            }
        }

        fn list_directory(&self, path: &std::path::Path) -> Result<Vec<String>, String> {
            self.directory_calls.fetch_add(1, Ordering::Relaxed);
            if path == self.root {
                Ok(vec!["virtual.txt".into()])
            } else {
                Err("unknown virtual directory".into())
            }
        }
    }

    let program = parse(
        &lex("fn roundtrip() -> String uses FileSystem { write_text(\"virtual.txt\", \"content\") return read_text(\"virtual.txt\") } fn read_path(path: String) -> String uses FileSystem { return read_text(path) } fn file_check(path: String) -> Bool uses FileSystem { return file_exists(path) } fn directory_check(path: String) -> Bool uses FileSystem { return dir_exists(path) } fn file_length(path: String) -> Result<Int, String> uses FileSystem { return file_size(path) } fn entries(path: String) -> String[] uses FileSystem { return list_dir(path) } fn main() {}")
            .unwrap(),
    )
    .unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();
    let root = std::env::temp_dir().join(format!(
        "zelyra-injected-filesystem-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("virtual.txt"), "placeholder").unwrap();
    let outside_name = format!("{}.outside", root.file_name().unwrap().to_string_lossy());
    let outside = root.parent().unwrap().join(&outside_name);
    std::fs::write(&outside, "outside").unwrap();
    let policy = RuntimePolicy {
        filesystem: Some(FileSystemPolicy {
            base_dir: root.clone(),
            read_roots: vec![root.clone()],
            write_roots: vec![root.clone()],
        }),
        network: None,
        process: None,
        mail: None,
        step_budget: None,
        max_stack_depth: None,
        package_settings: None,
    };
    let host = Arc::new(VirtualFiles {
        files: std::sync::Mutex::new(std::collections::HashMap::new()),
        calls: AtomicUsize::new(0),
        canonicalizations: AtomicUsize::new(0),
        metadata_calls: AtomicUsize::new(0),
        directory_calls: AtomicUsize::new(0),
        root: root.clone(),
    });
    let host_services: Arc<dyn HostServices> = host.clone();
    let denied = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "roundtrip",
        Vec::new(),
        None,
        Some(&HashSet::new()),
        Some(&policy),
        Arc::clone(&host_services),
    )
    .expect_err("capability denial must precede virtual filesystem access");
    assert!(denied.message.contains("FileSystem"));
    assert_eq!(host.calls.load(Ordering::Relaxed), 0);

    let grants = HashSet::from(["FileSystem".to_owned()]);
    let result = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "roundtrip",
        Vec::new(),
        None,
        Some(&grants),
        Some(&policy),
        Arc::clone(&host_services),
    )
    .unwrap();
    assert_eq!(result, Value::String("content".into()));
    assert_eq!(host.calls.load(Ordering::Relaxed), 2);
    assert_eq!(host.canonicalizations.load(Ordering::Relaxed), 2);

    let file_exists = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "file_check",
        vec![Value::String("virtual.txt".into())],
        None,
        Some(&grants),
        Some(&policy),
        Arc::clone(&host_services),
    )
    .unwrap();
    assert_eq!(file_exists, Value::Bool(true));
    let directory_exists = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "directory_check",
        vec![Value::String(".".into())],
        None,
        Some(&grants),
        Some(&policy),
        Arc::clone(&host_services),
    )
    .unwrap();
    assert_eq!(directory_exists, Value::Bool(true));
    let file_length = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "file_length",
        vec![Value::String("virtual.txt".into())],
        None,
        Some(&grants),
        Some(&policy),
        Arc::clone(&host_services),
    )
    .unwrap();
    assert_eq!(file_length, Value::Result(Ok(Box::new(Value::Int(23)))));
    let entries = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "entries",
        vec![Value::String(".".into())],
        None,
        Some(&grants),
        Some(&policy),
        Arc::clone(&host_services),
    )
    .unwrap();
    assert_eq!(
        entries,
        Value::Array(vec![Value::String("virtual.txt".into())])
    );
    assert_eq!(host.metadata_calls.load(Ordering::Relaxed), 3);
    assert_eq!(host.directory_calls.load(Ordering::Relaxed), 1);

    let denied_path = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "read_path",
        vec![Value::String(format!("../{outside_name}"))],
        None,
        Some(&grants),
        Some(&policy),
        Arc::clone(&host_services),
    )
    .expect_err("the runtime must reject a canonical path outside its configured roots");
    assert!(denied_path.message.contains("outside the configured roots"));
    assert_eq!(host.calls.load(Ordering::Relaxed), 2);
    assert_eq!(host.canonicalizations.load(Ordering::Relaxed), 7);
    std::fs::remove_file(root.join("virtual.txt")).unwrap();
    std::fs::remove_dir(root).unwrap();
    std::fs::remove_file(outside).unwrap();
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
        mail: None,
        step_budget: None,
        max_stack_depth: None,
        package_settings: None,
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
fn injected_host_services_drive_clock_environment_and_random_builtins() {
    struct FixedHost {
        calls: AtomicUsize,
    }

    impl HostServices for FixedHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(1_700_000_000_123)
        }

        fn environment_variable(&self, name: &str) -> Result<Option<String>, String> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok((name == "ZELYRA_TEST_HOST").then(|| "host-value".to_owned()))
        }

        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            output.fill(7);
            Ok(())
        }
    }

    let source = "fn clock_value() -> Timestamp uses Clock { return now() }\
        fn environment_value() -> Option<String> uses Environment { return env(\"ZELYRA_TEST_HOST\") }\
        fn random_value() -> Int uses Random { return random_int(10, 10) }\
        fn random_uuid() -> String uses Random { return uuid_v4() }\
        fn main() {}";
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();

    let host = Arc::new(FixedHost {
        calls: AtomicUsize::new(0),
    });
    let host_services: Arc<dyn HostServices> = host.clone();
    let denied = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "clock_value",
        Vec::new(),
        None,
        Some(&HashSet::new()),
        None,
        Arc::clone(&host_services),
    )
    .expect_err("runtime grants must be checked before calling an injected provider");
    assert!(denied.message.contains("Clock"), "{}", denied.message);
    assert_eq!(host.calls.load(Ordering::Relaxed), 0);

    let grants = HashSet::from([
        "Clock".to_owned(),
        "Environment".to_owned(),
        "Random".to_owned(),
    ]);
    let call = |name| {
        execute_function_with_host_services_and_capabilities_and_policies(
            &program,
            name,
            Vec::new(),
            None,
            Some(&grants),
            None,
            Arc::clone(&host_services),
        )
        .unwrap()
    };
    assert!(matches!(
        call("clock_value"),
        Value::Timestamp(1_700_000_000_123)
    ));
    assert!(matches!(
        call("environment_value"),
        Value::Option(Some(value)) if matches!(*value, Value::String(ref value) if value == "host-value")
    ));
    assert!(matches!(call("random_value"), Value::Int(10)));
    assert!(matches!(
        call("random_uuid"),
        Value::String(value) if value == "07070707-0707-4707-8707-070707070707"
    ));
    assert_eq!(host.calls.load(Ordering::Relaxed), 4);

    let parallel_source = "fn clock_value() -> Timestamp uses Clock { return now() }\
        fn main() uses Clock { parallel { observed = await clock_value() } print(observed) }";
    let parallel_program = parse(&lex(parallel_source).unwrap()).unwrap();
    check(&parallel_program).unwrap();
    check_capabilities(&parallel_program).unwrap();
    let output = execute_with_host_services(
        &parallel_program,
        Arc::clone(&host_services),
        Some(&grants),
        None,
    )
    .unwrap();
    assert_eq!(output, ["1700000000123"]);
    assert_eq!(host.calls.load(Ordering::Relaxed), 5);

    struct InvalidClockHost;
    impl HostServices for InvalidClockHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            Ok(i64::MAX)
        }
        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }
        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }
    }
    let invalid_time = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "clock_value",
        Vec::new(),
        None,
        Some(&grants),
        None,
        Arc::new(InvalidClockHost),
    )
    .expect_err("an embedding cannot introduce a Timestamp outside the language range");
    assert!(invalid_time
        .message
        .contains("outside years 0001 through 9999"));
    assert_eq!(HOST_SERVICES_API_VERSION, 8);
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

    struct ConsoleHost(AtomicUsize);
    impl HostServices for ConsoleHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            Ok(0)
        }
        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }
        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }
        fn read_console_line(&self, prompt: &str) -> Result<Option<String>, String> {
            assert_eq!(prompt, "Date: ");
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(Some("2026-10-16".into()))
        }
    }
    let host = Arc::new(ConsoleHost(AtomicUsize::new(0)));
    let grants = HashSet::from(["Console".to_owned()]);
    let output = execute_with_host_services(
        &program,
        Arc::clone(&host) as Arc<dyn HostServices>,
        Some(&grants),
        None,
    )
    .unwrap();
    assert_eq!(output, ["2026-10-16"]);
    assert_eq!(host.0.load(Ordering::Relaxed), 1);

    let denied = execute_with_host_services(&program, host.clone(), Some(&HashSet::new()), None)
        .expect_err("the runtime must deny console input before provider access");
    assert!(denied.message.contains("Console"));
    assert_eq!(host.0.load(Ordering::Relaxed), 1);
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
fn mail_api_is_typed_transactional_and_capability_checked() {
    let source = r#"
fn enqueue_mail(to: String, subject: String, text: String) uses Email, Database(write) {
    transaction {
        mail_queue(to, subject, text)
    }
}
fn deliver_mail(event_id: String, payload: String) -> Result<Unit, String> uses Email {
    return mail_deliver(event_id, payload)
}
fn main() { }
"#;
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();

    let missing_email = parse(
        &lex("fn main() { transaction { mail_queue(\"a@example.test\", \"Hi\", \"Body\") } }")
            .unwrap(),
    )
    .unwrap();
    check(&missing_email).unwrap();
    let diagnostics = check_capabilities(&missing_email).unwrap_err();
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("Email")));
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("Database(write)")));

    let outside_transaction = parse(
        &lex("fn main() uses Email, Database(write) { mail_queue(\"a@example.test\", \"Hi\", \"Body\") }")
            .unwrap(),
    )
    .unwrap();
    let type_errors = check(&outside_transaction).unwrap_err();
    assert!(type_errors.iter().any(|diagnostic| diagnostic
        .message
        .contains("only allowed inside a transaction")));

    let side_effecting_argument = parse(
        &lex(
            "fn recipient() -> String { return \"a@example.test\" } fn main() uses Email, Database(write) { transaction { mail_queue(recipient(), \"Hi\", \"Body\") } }",
        )
        .unwrap(),
    )
    .unwrap();
    let type_errors = check(&side_effecting_argument).unwrap_err();
    assert!(type_errors.iter().any(|diagnostic| diagnostic
        .message
        .contains("arguments must be values or pure expressions")));
}

#[test]
fn mail_queue_creates_a_validated_outbox_row_inside_one_database_batch() {
    struct OutboxHost {
        batches: std::sync::Mutex<Vec<(Vec<zelyra_database::Query>, bool)>>,
        calls: AtomicUsize,
    }

    impl HostServices for OutboxHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            Ok(0)
        }

        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }

        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }

        fn execute_database_batch(
            &self,
            database_url: &str,
            queries: &[zelyra_database::Query],
            transaction: bool,
        ) -> Result<Vec<zelyra_database::QueryResult>, String> {
            assert!(supports_mail_outbox_database(database_url));
            self.calls.fetch_add(1, Ordering::Relaxed);
            self.batches
                .lock()
                .map_err(|_| "outbox test lock poisoned".to_owned())?
                .push((queries.to_vec(), transaction));
            Ok(vec![zelyra_database::QueryResult::default(); queries.len()])
        }
    }

    let source = r#"
database main { engine: mariadb database: "outbox_test" }
table accounts { id: Id primary auto balance: Int required }
fn enqueue() uses Email, Database(write) {
    transaction {
        mail_queue("customer@example.test", "Invoice ready", "Your invoice is ready.")
        sql { UPDATE accounts SET balance = 1 WHERE id = 1 }
    }
}
fn invalid_recipient() uses Email, Database(write) {
    transaction { mail_queue("not-an-email", "Invoice ready", "Your invoice is ready.") }
}
fn main() {}
"#;
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();
    let host = Arc::new(OutboxHost {
        batches: std::sync::Mutex::new(Vec::new()),
        calls: AtomicUsize::new(0),
    });
    let host_services: Arc<dyn HostServices> = host.clone();
    let grants = HashSet::from(["Email".to_owned(), "Database(write)".to_owned()]);
    let database_url = "mariadb://user:secret@127.0.0.1:3306/outbox_test";

    execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "enqueue",
        Vec::new(),
        Some(database_url),
        Some(&grants),
        None,
        Arc::clone(&host_services),
    )
    .unwrap();
    assert_eq!(host.calls.load(Ordering::Relaxed), 1);
    let batches = host.batches.lock().unwrap();
    assert_eq!(batches.len(), 1);
    assert!(batches[0].1);
    assert_eq!(batches[0].0.len(), 2);
    assert!(batches[0].0[0].sql.contains("INSERT INTO `_zelyra_outbox`"));
    assert!(matches!(
        batches[0].0[0].params[0].1,
        zelyra_database::QueryValue::String(ref event_id)
            if event_id == "00000000000000000000000000000000"
    ));
    assert!(matches!(
        batches[0].0[0].params[2].1,
        zelyra_database::QueryValue::String(ref payload)
            if payload.contains("customer@example.test")
                && payload.contains("Invoice ready")
    ));
    drop(batches);

    let invalid = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "invalid_recipient",
        Vec::new(),
        Some(database_url),
        Some(&grants),
        None,
        host_services,
    )
    .unwrap_err();
    assert!(invalid.message.contains("invalid message"));
    assert_eq!(host.calls.load(Ordering::Relaxed), 1);
}

#[test]
fn mail_delivery_failure_returns_redacted_result() {
    let program = parse(
        &lex("fn deliver(id: String, payload: String) -> Result<Unit, String> uses Email { return mail_deliver(id, payload) } fn main() { }")
            .unwrap(),
    )
    .unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();
    let grants = HashSet::from([String::from("Email")]);
    let result = execute_function_with_capabilities_and_policies(
        &program,
        "deliver",
        vec![
            Value::String("event-01".into()),
            Value::String(r#"{"to":"a@example.test","subject":"Hi","text":"Body"}"#.into()),
        ],
        None,
        Some(&grants),
        None,
    )
    .unwrap();
    assert!(
        matches!(result, Value::Result(Err(message)) if matches!(*message, Value::String(ref message) if message.contains("SMTP") && !message.contains("secret")))
    );
}

#[test]
fn mail_delivery_uses_host_only_after_capability_and_smtp_allowlist_checks() {
    struct MailHost(AtomicUsize);
    impl HostServices for MailHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            Ok(0)
        }
        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }
        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }
        fn deliver_outbox_message(
            &self,
            event_id: &str,
            payload_json: &str,
            mail: &MailPolicy,
            network: &NetworkPolicy,
        ) -> Result<(), String> {
            assert_eq!(event_id, "event-01");
            assert!(payload_json.contains("a@example.test"));
            assert_eq!(mail.host.as_deref(), Some("smtp.example.test"));
            assert!(network
                .allowed_hosts
                .iter()
                .any(|host| host == "smtp.example.test"));
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    let program = parse(
        &lex("fn deliver(id: String, payload: String) -> Result<Unit, String> uses Email { return mail_deliver(id, payload) } fn main() {}")
            .unwrap(),
    )
    .unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();
    let grants = HashSet::from(["Email".to_owned()]);
    let mut mail_policy = MailPolicy::default();
    mail_policy.host = Some("smtp.example.test".into());
    mail_policy.from = Some("billing@example.test".into());
    let policy = RuntimePolicy {
        filesystem: None,
        network: Some(NetworkPolicy {
            allowed_hosts: vec!["smtp.example.test".into()],
            timeout_ms: 1_000,
            max_response_bytes: 128,
        }),
        process: None,
        mail: Some(mail_policy),
        step_budget: None,
        max_stack_depth: None,
        package_settings: None,
    };
    let host = Arc::new(MailHost(AtomicUsize::new(0)));
    let arguments = || {
        vec![
            Value::String("event-01".into()),
            Value::String(r#"{"to":"a@example.test","subject":"Hi","text":"Body"}"#.into()),
        ]
    };
    let delivered = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "deliver",
        arguments(),
        None,
        Some(&grants),
        Some(&policy),
        host.clone(),
    )
    .unwrap();
    assert!(matches!(delivered, Value::Result(Ok(value)) if matches!(*value, Value::Unit)));
    assert_eq!(host.0.load(Ordering::Relaxed), 1);

    let denied_smtp = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "deliver",
        arguments(),
        None,
        Some(&grants),
        Some(&RuntimePolicy {
            network: Some(NetworkPolicy {
                allowed_hosts: vec!["other.example.test".into()],
                timeout_ms: 1_000,
                max_response_bytes: 128,
            }),
            ..policy.clone()
        }),
        host.clone(),
    )
    .unwrap();
    assert!(
        matches!(denied_smtp, Value::Result(Err(message)) if matches!(*message, Value::String(ref message) if message.contains("not allowed")))
    );
    assert_eq!(host.0.load(Ordering::Relaxed), 1);

    let denied_capability = execute_function_with_host_services_and_capabilities_and_policies(
        &program,
        "deliver",
        arguments(),
        None,
        Some(&HashSet::new()),
        Some(&policy),
        host.clone(),
    )
    .expect_err("the Email capability must be checked before host mail delivery");
    assert!(denied_capability.message.contains("Email"));
    assert_eq!(host.0.load(Ordering::Relaxed), 1);
}

#[test]
fn mail_outbox_database_support_is_explicit() {
    assert!(supports_mail_outbox_database(
        "mariadb://user:secret@127.0.0.1:3306/app"
    ));
    assert!(supports_mail_outbox_database(
        "mysql://user:secret@127.0.0.1:3306/app"
    ));
    assert!(!supports_mail_outbox_database(
        "postgres://user:secret@127.0.0.1:5432/app"
    ));
    assert!(!supports_mail_outbox_database("sqlite:///tmp/app.db"));
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
fn safe_text_file_apis_preserve_utf8_and_reject_overwrites_and_path_escape() {
    let root = std::env::temp_dir().join(format!(
        "zelyra-safe-text-files-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let notes = root.join("notes");
    std::fs::create_dir_all(&notes).unwrap();
    let program = parse(
        &lex(r#"fn main() uses FileSystem {
                match write_text_new_result("notes/a.note", "Grüße; \"Zitat\"\nzweite Zeile") {
                    Ok(_) => { print("created") }
                    Err(message) => { print("create error: " + message) }
                }
                match write_text_new_result("notes/a.note", "overwrite") {
                    Ok(_) => { print("overwrite allowed") }
                    Err(_) => { print("overwrite refused") }
                }
                match atomic_write_text_result("notes/a.note", "neue Version äöü") {
                    Ok(_) => { print("updated") }
                    Err(message) => { print("update error: " + message) }
                }
                match read_text_result("notes/a.note") {
                    Ok(text) => { print(text) }
                    Err(message) => { print("read error: " + message) }
                }
                match append_text_result("notes/log.jsonl", "{\"event\":\"edit\"}\n") {
                    Ok(_) => { print("appended") }
                    Err(message) => { print("append error: " + message) }
                }
                match copy_file_new_result("notes/a.note", "notes/b.note") {
                    Ok(_) => { print("copied") }
                    Err(message) => { print("copy error: " + message) }
                }
                match rename_file_result("notes/b.note", "notes/c.note") {
                    Ok(_) => { print("renamed") }
                    Err(message) => { print("rename error: " + message) }
                }
                match delete_file_result("notes/c.note") {
                    Ok(_) => { print("deleted") }
                    Err(message) => { print("delete error: " + message) }
                }
                match write_text_new_result("../outside.note", "blocked") {
                    Ok(_) => { print("path escape allowed") }
                    Err(_) => { print("path escape refused") }
                }
            }"#)
        .unwrap(),
    )
    .unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();
    let policy = FileSystemPolicy {
        base_dir: root.clone(),
        read_roots: vec![notes.clone()],
        write_roots: vec![notes.clone()],
    };
    let grants = HashSet::from([String::from("FileSystem")]);
    let output =
        execute_with_capabilities_and_filesystem_policy(&program, Some(&grants), Some(&policy))
            .unwrap();
    assert_eq!(
        output,
        [
            "created",
            "overwrite refused",
            "updated",
            "neue Version äöü",
            "appended",
            "copied",
            "renamed",
            "deleted",
            "path escape refused"
        ]
    );
    assert_eq!(
        std::fs::read_to_string(notes.join("a.note")).unwrap(),
        "neue Version äöü"
    );
    assert_eq!(
        std::fs::read_to_string(notes.join("log.jsonl")).unwrap(),
        "{\"event\":\"edit\"}\n"
    );
    assert!(!notes.join("b.note").exists());
    assert!(!notes.join("c.note").exists());
    assert!(!root.join("outside.note").exists());
    std::fs::remove_dir_all(root).unwrap();

    let ungranted = parse(
        &lex("fn main() { match write_text_new_result(\"data.txt\", \"text\") { Ok(_) => {} Err(_) => {} } }").unwrap(),
    )
    .unwrap();
    check(&ungranted).unwrap();
    assert!(check_capabilities(&ungranted)
        .unwrap_err()
        .iter()
        .any(|error| error.message.contains("FileSystem")));
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
fn capability_use_inventory_uses_the_same_checked_operations() {
    let source = "fn helper() uses FileSystem { write_text(\"data/note.txt\", \"ok\") } fn main() uses FileSystem, Clock { helper() now() }";
    let program = parse(&lex(source).unwrap()).unwrap();
    check(&program).unwrap();
    check_capabilities(&program).unwrap();
    let uses = inspect_capability_uses(&program);
    assert!(uses.iter().any(|usage| usage.function == "helper"
        && usage.capability == "FileSystem"
        && usage.operation == "file-system write access"));
    let write = uses
        .iter()
        .find(|usage| usage.function == "helper" && usage.capability == "FileSystem")
        .unwrap();
    assert_eq!(write.file_paths.len(), 1);
    assert_eq!(write.file_paths[0].path.as_deref(), Some("data/note.txt"));
    assert!(write.file_paths[0].write);
    assert!(write.file_paths[0].allow_missing);
    assert!(uses.iter().any(|usage| usage.function == "main"
        && usage.capability == "FileSystem"
        && usage.operation == "call to `helper`"));
    assert!(uses.iter().any(|usage| usage.function == "main"
        && usage.capability == "Clock"
        && usage.operation == "clock access"));
}

#[test]
fn file_path_inspection_reuses_runtime_root_rules_for_missing_and_symlink_paths() {
    let root = std::env::temp_dir().join(format!(
        "zelyra-path-inspection-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let allowed = root.join("allowed");
    let outside = root.join("outside");
    std::fs::create_dir_all(&allowed).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    let policy = FileSystemPolicy {
        base_dir: root.clone(),
        read_roots: vec![allowed.clone()],
        write_roots: vec![allowed.clone()],
    };
    assert_eq!(
        inspect_filesystem_path(Some(&policy), "allowed/new.txt", true, true).unwrap(),
        allowed.join("new.txt")
    );
    assert!(inspect_filesystem_path(Some(&policy), "outside/new.txt", true, true).is_err());
    assert!(inspect_filesystem_path(Some(&policy), "allowed/new.txt", false, false).is_err());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&outside, allowed.join("escape")).unwrap();
        assert!(
            inspect_filesystem_path(Some(&policy), "allowed/escape/new.txt", true, true).is_err()
        );
    }
    std::fs::remove_dir_all(root).unwrap();
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
fn file_refs_cannot_be_declared_as_external_json_api_fields() {
    let program = parse(
        &lex("api POST \"/documents\" { input { document: FileRef } output Unit } fn main() {}")
            .unwrap(),
    )
    .unwrap();
    let errors = check_apis(&program)
        .expect_err("opaque file references cannot be forged in normal JSON requests");
    assert!(errors
        .iter()
        .any(|error| error.message.contains("unknown API input type `FileRef`")));
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
fn rejects_generic_functions_as_api_handlers() {
    let program = parse(
        &lex("fn get_customer<T>(id: Id) -> Customer { } api GET \"/customers/{id}\" { handler get_customer input { id: Id } output Customer } fn main() { }")
            .unwrap(),
    )
    .unwrap();
    let errors = check_apis(&program).expect_err("API routes have no generic type argument syntax");
    assert!(errors.iter().any(|error| {
        error.message.contains("must be non-generic")
            && error.message.contains("cannot supply type arguments")
    }));
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
fn generic_api_types_reject_function_payloads_and_allow_recursive_records() {
    let wrong_arity = parse(
        &lex("enum Maybe<T> { Some(T) None } api GET \"/maybe\" { output Maybe<Int, String> } fn main() { }")
            .unwrap(),
    )
    .unwrap();
    let errors =
        check_apis(&wrong_arity).expect_err("API generic types must use their declared arity");
    assert!(errors
        .iter()
        .any(|error| error.message.contains("unknown API output type")));

    let unsupported = parse(
        &lex("enum Callback<T> { Ready(T) } api GET \"/callback\" { output Callback<Function<() -> Int>> } fn main() { }")
            .unwrap(),
    )
    .unwrap();
    let errors = check_apis(&unsupported)
        .expect_err("function values nested in generic enum payloads cannot cross JSON APIs");
    assert!(
        errors.iter().any(|error| {
            error
                .message
                .contains("function values cannot cross an API JSON boundary")
        }),
        "{errors:?}"
    );

    let recursive = parse(
        &lex("struct Node<T> { value: T next: Node<T>? } api GET \"/nodes\" { output Node<Int> } fn main() { }")
            .unwrap(),
    )
    .unwrap();
    check_apis(&recursive).expect("recursive generic records must not recurse in type validation");
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

#[test]
fn lossless_json_keeps_reserved_object_keys_and_decimal_string_contract() {
    let output = run(r#"fn main() {
        payload = "{\"$serde_json::private::Number\":\"12.34\"}"
        object = json_decode<Map<String, String>>(payload)
        assert(get(object, "$serde_json::private::Number") == Some("12.34"))
        assert(json_encode(object) == payload)
        amount = json_decode<Decimal>("\"9007199254740993.123456789\"")
        assert(amount == decimal("9007199254740993.123456789"))
        print(json_encode(amount))
    }"#);
    assert_eq!(output, ["\"9007199254740993.123456789\""]);
    let program =
        parse(&lex("fn main() { value = json_decode<Decimal>(\"1.25\") }").unwrap()).unwrap();
    check(&program).unwrap();
    assert!(execute(&program)
        .unwrap_err()
        .message
        .contains("Decimal string"));
}

#[test]
fn decimal_alias_returns_are_checked_against_the_underlying_value_type() {
    let output = run(r#"type Amount = Decimal
        fn echo_amount(amount: Amount) -> Amount { return amount }
        fn echo_list(amounts: Amount[]) -> Amount[] { return amounts }
        fn main() {
            amount = decimal("9007199254740993.123456789")
            assert(echo_amount(amount) == amount)
            assert(echo_list([amount]) == [amount])
            print("PASS")
        }"#);
    assert_eq!(output, ["PASS"]);
    let bad = parse(
        &lex("type Amount = Decimal fn bad() -> Amount { return \"not money\" } fn main() {} ")
            .unwrap(),
    )
    .unwrap();
    assert!(check(&bad).is_err());
    let error = execute_function(&bad, "bad", Vec::new(), None).unwrap_err();
    assert!(error.message.contains("expected"));
}

#[test]
fn recursive_type_aliases_are_rejected_even_without_a_use_site() {
    for source in [
        "type A = A fn main() {}",
        "type A = B[] type B = A fn main() {}",
    ] {
        let program = parse(&lex(source).unwrap()).unwrap();
        let errors = check(&program).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.code == "E-TYPE-ALIAS-001" && error.span.line == 1));
    }
}

#[test]
fn decimal_boundary_arithmetic_checks_the_result_not_only_the_intermediate() {
    let output = run(r#"fn main() {
        largest = decimal("170141183460469231731687303715.884105727")
        smallest = decimal("-170141183460469231731687303715.884105728")
        one = decimal("1")
        assert(largest * one == largest)
        assert(largest / one == largest)
        assert(largest / largest == one)
        assert(smallest * one == smallest)
        assert(smallest / smallest == one)
        assert(largest * decimal("0.000000001") == decimal("170141183460469231731.687303715"))
        print("PASS")
    }"#);
    assert_eq!(output, ["PASS"]);
}

#[test]
fn decimal_extreme_remainder_and_invalid_operations_do_not_panic() {
    let valid = run(r#"fn main() {
        smallest = decimal("-170141183460469231731687303715.884105728")
        assert(smallest % decimal("-0.000000001") == decimal("0"))
        print("PASS")
    }"#);
    assert_eq!(valid, ["PASS"]);
    for expression in [
        "decimal(\"1\") / decimal(\"0\")",
        "decimal(\"1\") % decimal(\"0\")",
        "decimal(\"170141183460469231731687303715.884105727\") * decimal(\"2\")",
        "decimal(\"-170141183460469231731687303715.884105728\") / decimal(\"-1\")",
    ] {
        let program =
            parse(&lex(&format!("fn main() {{ print({expression}) }}")).unwrap()).unwrap();
        check(&program).unwrap();
        let error = execute(&program).unwrap_err();
        assert!(error.span.start > 0);
        assert!(error.message.contains("overflow") || error.message.contains("zero"));
    }
}

#[test]
fn timestamp_decoding_rejects_values_outside_the_documented_calendar_range() {
    for value in [
        "-62135596800001",
        "253402300800000",
        "9223372036854775807",
        "-9223372036854775808",
    ] {
        let program = parse(
            &lex(&format!(
                "fn main() {{ value = json_decode<Timestamp>(\"{value}\") }}"
            ))
            .unwrap(),
        )
        .unwrap();
        check(&program).unwrap();
        let error = execute(&program).unwrap_err();
        assert!(error.span.start > 0);
        assert!(error.message.contains("Timestamp"));
    }
    assert_eq!(
        run(r#"fn main() {
        print(format_timestamp(json_decode<Timestamp>("-62135596800000")))
        print(format_timestamp(json_decode<Timestamp>("253402300799999")))
        print(format_timestamp(json_decode<Timestamp>("-1")))
    }"#),
        [
            "0001-01-01T00:00:00.000Z",
            "9999-12-31T23:59:59.999Z",
            "1969-12-31T23:59:59.999Z"
        ]
    );
}

// Inject driver-shaped results into the decoder without opening a database.
// The public language entry point separately requires a direct SQL expression.
fn decode_sql_fixture(
    target: &str,
    definitions: &str,
    columns: &[&str],
    kinds: Vec<zelyra_database::QueryColumnType>,
    rows: Vec<Vec<&str>>,
    nulls: Vec<Vec<bool>>,
) -> Result<Value, RuntimeError> {
    let source = format!("{definitions} fn decode(rows: Int[]) {{ return sql_decode<{target}>(rows) }} fn main() {{ }}");
    let program = parse(&lex(&source).unwrap()).unwrap();
    execute_function(
        &program,
        "decode",
        vec![Value::Rows {
            columns: columns.iter().map(|value| (*value).into()).collect(),
            rows: rows
                .iter()
                .map(|row| row.iter().map(|value| (*value).into()).collect())
                .collect(),
            metadata: Box::new(SqlRowsMetadata {
                nulls,
                column_types: kinds,
            }),
            result_type: Box::new(Type::Array(Box::new(Type::Int))),
        }],
        None,
    )
}

#[test]
fn typed_sql_results_preserve_null_and_literal_sentinel() {
    use zelyra_database::QueryColumnType::Text;
    let value = decode_sql_fixture(
        "Option<String>[]",
        "",
        &["value"],
        vec![Text],
        vec![vec!["\\N"], vec!["\\N"]],
        vec![vec![true], vec![false]],
    )
    .unwrap();
    assert_eq!(
        value,
        Value::Array(vec![
            Value::Option(None),
            Value::Option(Some(Box::new(Value::String("\\N".into()))))
        ])
    );
}

#[test]
fn typed_sql_results_enforce_cardinality_and_projection_even_when_empty() {
    use zelyra_database::QueryColumnType::Integer;
    let error =
        decode_sql_fixture("Int", "", &["value"], vec![Integer], vec![], vec![]).unwrap_err();
    assert!(error.message.contains("exactly one row"));
    assert!(error.span.start > 0);
    assert_eq!(
        decode_sql_fixture("Option<Int>", "", &["value"], vec![Integer], vec![], vec![]).unwrap(),
        Value::Option(None)
    );
    let error = decode_sql_fixture(
        "Option<Int>",
        "",
        &["value"],
        vec![Integer],
        vec![vec!["1"], vec!["2"]],
        vec![vec![false], vec![false]],
    )
    .unwrap_err();
    assert!(error.message.contains("at most one row"));
    assert_eq!(
        decode_sql_fixture("Int[]", "", &["value"], vec![Integer], vec![], vec![]).unwrap(),
        Value::Array(vec![])
    );
    let error = decode_sql_fixture(
        "Customer[]",
        "struct Customer { id: Int name: String }",
        &["id"],
        vec![Integer],
        vec![],
        vec![],
    )
    .unwrap_err();
    assert!(error.message.contains("missing column `name`"));
}

#[test]
fn typed_sql_results_use_exact_numbers_and_reject_float_shortcuts() {
    use zelyra_database::QueryColumnType::{Decimal, Float, Integer};
    assert_eq!(
        decode_sql_fixture(
            "UInt",
            "",
            &["value"],
            vec![Decimal],
            vec![vec!["18446744073709551615"]],
            vec![vec![false]]
        )
        .unwrap(),
        Value::UInt(u64::MAX)
    );
    assert_eq!(
        decode_sql_fixture(
            "Decimal",
            "",
            &["value"],
            vec![Decimal],
            vec![vec!["119.000000000"]],
            vec![vec![false]]
        )
        .unwrap(),
        Value::Decimal(119_000_000_000)
    );
    for (target, kind, value) in [
        ("UInt", Decimal, "18446744073709551616"),
        ("UInt", Integer, "-1"),
        ("Int", Decimal, "1.5"),
        ("Decimal", Float, "0.1"),
        ("Decimal", Decimal, "0.0000000001"),
        ("Float", Float, "NaN"),
        ("Bool", Integer, "2"),
    ] {
        let error = decode_sql_fixture(
            target,
            "",
            &["value"],
            vec![kind],
            vec![vec![value]],
            vec![vec![false]],
        )
        .unwrap_err();
        assert!(
            error.message.contains("column `value`"),
            "{}",
            error.message
        );
    }
}

#[test]
fn typed_sql_opt_in_checks_direct_query_and_class_boundary() {
    for source in [
        "fn main() { value = sql_decode<Int>(1) }",
        "fn main() uses Database(write) { value = sql_decode<Int>(sql { DELETE FROM customers }) }",
        "fn main() uses Database(read) { value = sql_decode<Int>() }",
        "class Customer { name: String } fn main() uses Database(read) { value = sql_decode<Customer>(sql<Int> { SELECT 1 }) }",
    ] {
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(check(&program).is_err(), "accepted {source}");
    }
    let program = parse(&lex("fn main() uses Database(read) { value = sql_decode<Int>(sql<Int> { SELECT 1 }) print(value + 1) }").unwrap()).unwrap();
    check(&program).unwrap();
}

#[test]
fn typed_sql_records_aliases_generics_and_table_rows_are_usable() {
    use zelyra_database::QueryColumnType::{Decimal, Integer, Text};
    let value = decode_sql_fixture(
        "Envelope<Decimal>",
        "type Amount = Decimal struct Envelope<T> { total: T note: Option<String> }",
        &["note", "total"],
        vec![Text, Decimal],
        vec![vec!["\\N", "119.000000000"]],
        vec![vec![true, false]],
    )
    .unwrap();
    assert_eq!(
        value,
        Value::Object {
            type_name: "Envelope".into(),
            type_args: vec![Type::Decimal],
            fields: HashMap::from([
                ("total".into(), Value::Decimal(119_000_000_000)),
                ("note".into(), Value::Option(None))
            ])
        }
    );
    let value = decode_sql_fixture(
        "Customer",
        "table customers { id: Id primary auto name: String(100) required note: String(100) }",
        &["name", "id", "note"],
        vec![Text, Integer, Text],
        vec![vec!["Ada", "1", "\\N"]],
        vec![vec![false, false, true]],
    )
    .unwrap();
    let Value::Object { fields, .. } = value else {
        panic!("expected typed table record")
    };
    assert_eq!(fields["id"], Value::Int(1));
    assert_eq!(fields["note"], Value::Option(None));
    let program = parse(&lex("table customers { id: Id primary auto name: String(100) required } fn main() uses Database(read) { customers = sql_decode<Customer[]>(sql<Customer[]> { SELECT id, name FROM customers }) print(customers[0].name) }").unwrap()).unwrap();
    check(&program).unwrap();
}

#[test]
fn typed_sql_temporal_values_preserve_midnight_and_milliseconds() {
    use zelyra_database::QueryColumnType::{Date, Time, Timestamp};
    assert_eq!(
        decode_sql_fixture(
            "Date",
            "",
            &["value"],
            vec![Date],
            vec![vec!["2000-02-29"]],
            vec![vec![false]]
        )
        .unwrap(),
        Value::Date("2000-02-29".into())
    );
    assert_eq!(
        decode_sql_fixture(
            "Time",
            "",
            &["value"],
            vec![Time],
            vec![vec!["00:00:00"]],
            vec![vec![false]]
        )
        .unwrap(),
        Value::Time("00:00:00".into())
    );
    for (source, expected) in [
        ("1970-01-01", 0),
        ("1969-12-31 23:59:59.999000", -1),
        ("1969-12-31T23:59:59.999Z", -1),
        ("2039-03-01 00:00:00.001000", 2182550400001),
    ] {
        assert_eq!(
            decode_sql_fixture(
                "Timestamp",
                "",
                &["value"],
                vec![Timestamp],
                vec![vec![source]],
                vec![vec![false]]
            )
            .unwrap(),
            Value::Timestamp(expected)
        );
    }
    for (target, kind, source) in [
        ("Date", Date, "1900-02-29"),
        ("Time", Time, "24:00:00"),
        ("Timestamp", Timestamp, "1970-01-01 00:00:00.000001"),
        ("Timestamp", Timestamp, "0000-01-01"),
        ("Timestamp", Timestamp, "2039-03-01T00:00:00+00:00"),
    ] {
        assert!(
            decode_sql_fixture(
                target,
                "",
                &["value"],
                vec![kind],
                vec![vec![source]],
                vec![vec![false]]
            )
            .is_err(),
            "accepted {source}"
        );
    }
}

#[test]
fn typed_sql_domains_do_not_round_money_or_overflow_ids() {
    assert_eq!(
        run(
            r#"fn price_roundtrip(price: Money) -> Money { return json_decode<Money>(json_encode(price)) } fn id_roundtrip(id: Id) -> Id { return json_decode<Id>(json_encode(id)) } fn main() { price = json_decode<Money>("\"19.99\"") print(json_encode(price_roundtrip(price))) id = json_decode<Id>("9223372036854775807") print(json_encode(id_roundtrip(id))) }"#
        ),
        ["\"19.99\"", "9223372036854775807"]
    );
    use zelyra_database::QueryColumnType::{Decimal, Integer};
    assert_eq!(
        decode_sql_fixture(
            "Money",
            "",
            &["value"],
            vec![Decimal],
            vec![vec!["999999999999999.9999"]],
            vec![vec![false]]
        )
        .unwrap(),
        Value::Decimal(999999999999999999900000)
    );
    for source in ["1000000000000000.0000", "0.00001", "-1000000000000000.0000"] {
        assert!(decode_sql_fixture(
            "Money",
            "",
            &["value"],
            vec![Decimal],
            vec![vec![source]],
            vec![vec![false]]
        )
        .is_err());
    }
    assert_eq!(
        decode_sql_fixture(
            "Id",
            "",
            &["value"],
            vec![Integer],
            vec![vec!["9223372036854775807"]],
            vec![vec![false]]
        )
        .unwrap(),
        Value::Int(i64::MAX)
    );
    assert!(decode_sql_fixture(
        "Id",
        "",
        &["value"],
        vec![Integer],
        vec![vec!["9223372036854775808"]],
        vec![vec![false]]
    )
    .is_err());
}

#[test]
fn typed_sql_rejects_ambiguous_columns_and_invalid_metadata() {
    use zelyra_database::QueryColumnType::{Integer, Text};
    for (columns, kinds, rows, nulls, expected) in [
        (
            vec!["id", "id"],
            vec![Integer, Integer],
            vec![vec!["1", "2"]],
            vec![vec![false, false]],
            "duplicate column",
        ),
        (
            vec!["id", "other"],
            vec![Integer, Text],
            vec![vec!["1", "Ada"]],
            vec![vec![false, false]],
            "unexpected column",
        ),
        (
            vec!["id"],
            vec![Integer],
            vec![vec!["1"]],
            vec![],
            "inconsistent native",
        ),
        (
            vec!["id"],
            vec![Integer],
            vec![vec!["\\N"]],
            vec![vec![true]],
            "row 1, column",
        ),
    ] {
        let error = decode_sql_fixture(
            "Customer[]",
            "struct Customer { id: Int }",
            &columns,
            kinds,
            rows,
            nulls,
        )
        .unwrap_err();
        assert!(error.message.contains(expected), "{}", error.message);
    }
    for target in ["Bytes", "Int[][]", "Customer"] {
        let source = format!("struct Customer {{ id: Int nested: Int[] }} fn main() uses Database(read) {{ rows = sql_decode<{target}>(sql<Int> {{ SELECT 1 }}) }}");
        let program = parse(&lex(&source).unwrap()).unwrap();
        let errors = check(&program).unwrap_err();
        assert!(errors.iter().any(|error| error.code == "E-TYPE-SQL-DECODE"));
    }
}

#[test]
fn typed_sql_preserves_explicit_domain_aliases_and_rejects_nested_domain_records() {
    assert_eq!(
        run(
            r#"type Money = String fn echo(price: Money) -> Money { return json_decode<Money>(json_encode(price)) } fn main() { print(echo("legacy text")) print(json_decode<Id>("\"legacy-id\"")) }"#
        ),
        ["legacy text", "legacy-id"]
    );
    use zelyra_database::QueryColumnType::Float;
    assert_eq!(
        decode_sql_fixture(
            "Money",
            "type Money = Float",
            &["value"],
            vec![Float],
            vec![vec!["0.125"]],
            vec![vec![false]]
        )
        .unwrap(),
        Value::Float(0.125)
    );
    let program = parse(&lex("type Money = Decimal fn main() uses Database(read) { value = sql_decode<Money>(sql<Decimal> { SELECT 1 }) }").unwrap()).unwrap();
    check(&program).unwrap();
    for definitions in [
        "struct Money { value: Int } struct Packet { price: Money }",
        "enum Money { Zero } struct Packet { price: Money }",
    ] {
        let source = format!("{definitions} fn main() uses Database(read) {{ value = sql_decode<Packet>(sql<Int> {{ SELECT 1 }}) }}");
        let program = parse(&lex(&source).unwrap()).unwrap();
        assert!(check(&program).is_err());
    }
}

#[test]
fn standard_library_v103_builtins_work_properly() {
    let source = r#"
fn main() {
    // Math
    print(abs(-42))
    print(min(10, 20))
    print(max(10, 20))
    print(clamp(15, 0, 10))

    // Strings
    print(text_to_lower("ZELYRA"))
    print(text_to_upper("sicher"))
    print(text_ucfirst("status"))
    print(text_ucfirst("Über"))
    print(text_ucfirst("7days"))
    print(text_ucfirst(""))
    print(text_starts_with("test.zyl", "test"))
    print(text_ends_with("test.zyl", ".zyl"))
    print(text_replace("Hallo Welt", "Welt", "Zelyra"))
    print(text_pad_left("42", 5, "0"))
    print(text_pad_right("Tab", 6, "."))
    print(join(text_split("a,b,c", ","), "-"))

    // Collections
    nums = [5, 1, 4, 2, 8]
    print(sort(nums))
    print(sort_desc(nums))
    print(reverse(nums))
    print(slice(nums, 1, 3))
    print(concat([1, 2], [3, 4]))

    // Maps & Options
    mutable m: Map<String, Int> = Map { "key": 100 }
    print(has_key(m, "key"))
    print(has_key(m, "missing"))
    print(get_or(m, "key", 0))
    print(get_or(m, "missing", 404))
    print(get_or_default(m, "missing", 405))
    print(unwrap_or(Some(99), 0))
    print(unwrap_or(None, 123))
    print(to_string(42))
}
"#;
    assert_eq!(
        run(source),
        [
            "42",
            "10",
            "20",
            "10",
            "zelyra",
            "SICHER",
            "Status",
            "Über",
            "7days",
            "",
            "true",
            "true",
            "Hallo Zelyra",
            "00042",
            "Tab...",
            "a-b-c",
            "[1, 2, 4, 5, 8]",
            "[8, 5, 4, 2, 1]",
            "[8, 2, 4, 1, 5]",
            "[1, 4, 2]",
            "[1, 2, 3, 4]",
            "true",
            "false",
            "100",
            "404",
            "405",
            "99",
            "123",
            "42",
        ]
    );
}

#[test]
fn standard_library_v103_phase2_builtins_work_properly() {
    let source = r#"
fn main() {
    // Math
    print(sqrt(25.0))
    print(pow(3.0, 3.0))
    print(floor(4.7))
    print(ceil(4.2))
    print(round(2.71828, 2))

    // Strings
    match text_index_of("banana", "na") {
        Some(pos) => {
            print(pos)
        }
        None => {
            print(-1)
        }
    }
    print(unwrap_or(text_index_of("éx", "x"), -1))
    print(text_repeat("x", 4))
    print(text_is_empty(""))
    print(text_capitalize("hello"))

    // Collections
    print(unique([1, 2, 2, 3, 1]))
    print(array_sum([5, 10, 15]))
    print(avg([10, 20]))
    print(unwrap_or(min_of([9, 3, 7]), 0))
    print(unwrap_or(max_of([9, 3, 7]), 0))
    print(chunk([1, 2, 3, 4], 2))
    print(flatten([[1, 2], [3, 4]]))

    // Sets & Maps
    s1: Set<Int> = set([1, 2])
    s2: Set<Int> = set([2, 3])
    print(set_union(s1, s2))
    print(set_intersection(s1, s2))
    print(set_difference(s1, s2))

    mutable m1: Map<String, Int> = Map { "a": 1, "b": 2 }
    mutable m2: Map<String, Int> = Map { "b": 20, "c": 30 }
    print(map_remove(m1, "a"))
    print(map_merge(m1, m2))
}

"#;
    assert_eq!(
        run(source),
        [
            "5",
            "27",
            "4",
            "5",
            "2.72",
            "2",
            "1",
            "xxxx",
            "true",
            "Hello",
            "[1, 2, 3]",
            "30",
            "15",
            "3",
            "9",
            "[[1, 2], [3, 4]]",
            "[1, 2, 3, 4]",
            "Set{1, 2, 3}",
            "Set{2}",
            "Set{1}",
            "Map{b=2}",
            "Map{a=1, b=20, c=30}",
        ]
    );
}

#[test]
fn standard_library_math_errors_are_checked_and_types_are_restricted() {
    for (source, expected) in [
        (
            "fn main() { value = abs(json_decode<Int>(\"-9223372036854775808\")) }",
            "Int overflow in abs",
        ),
        (
            r#"fn main() { value = abs(json_decode<Decimal>("\"-170141183460469231731687303715.884105728\"")) }"#,
            "Decimal overflow in abs",
        ),
        (
            "fn main() { value = sqrt(-1.0) }",
            "sqrt requires a finite non-negative Float",
        ),
        (
            "fn main() { value = pow(-2.0, 0.5) }",
            "pow is undefined or outside the finite Float range",
        ),
        (
            "fn main() { value = pow(2.0, 1024.0) }",
            "pow is undefined or outside the finite Float range",
        ),
        (
            "fn main() { value = round(1.25, 16) }",
            "round decimal places must be between -15 and 15",
        ),
        (
            "fn main() { value = round(pow(2.0, 1023.0), 15) }",
            "round result is outside the finite Float range",
        ),
        (
            "fn main() { value = clamp(4, 10, 2) }",
            "clamp minimum must not exceed maximum",
        ),
        (
            "fn main() { value = to_int(json_decode<UInt>(\"18446744073709551615\")) }",
            "to_int cannot convert",
        ),
        (
            "fn main() { value = to_int(pow(2.0, 63.0)) }",
            "to_int Float must be finite and within the Int range",
        ),
        (
            "fn main() { value = to_float(\"inf\") }",
            "to_float cannot convert",
        ),
        (
            "fn main() { value = text_pad_left(\"x\", -1, \"0\") }",
            "padding target length must be non-negative",
        ),
        (
            "fn main() { value = text_repeat(\"xx\", 8388609) }",
            "text_repeat output exceeds the 16 MiB text limit",
        ),
        (
            "fn main() { value = array_sum([pow(2.0, 1023.0), pow(2.0, 1023.0)]) }",
            "array_sum Float overflow",
        ),
        (
            "fn main() { value = avg([json_decode<Decimal>(\"\\\"170141183460469231731687303715.884105727\\\"\"), json_decode<Decimal>(\"\\\"0.000000001\\\"\")]) }",
            "avg decimal overflow",
        ),
    ] {
        let program = parse(&lex(source).unwrap()).unwrap();
        check(&program).unwrap();
        let error =
            execute(&program).expect_err("invalid math input must be a checked runtime error");
        assert!(error.message.contains(expected), "{}", error.message);
    }

    for source in [
        "fn main() { value = min(true, false) }",
        "fn main() { value = max(true, false) }",
        "fn main() { value = clamp(true, false, true) }",
        "fn main() { value = min_of([true, false]) }",
    ] {
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(check(&program).is_err(), "unexpectedly accepted {source}");
    }

    assert_eq!(
        run("fn main() { print(round(1.25, 1)) print(round(-1.25, 1)) print(round(125.0, -1)) print(min(json_decode<UInt>(\"3\"), json_decode<UInt>(\"2\"))) print(clamp(json_decode<UInt>(\"7\"), json_decode<UInt>(\"2\"), json_decode<UInt>(\"5\"))) print(unwrap_or(min_of([json_decode<UInt>(\"3\"), json_decode<UInt>(\"2\")]), json_decode<UInt>(\"0\"))) print(is_err(parse_float(\"NaN\"))) print(text_pad_left(\"é\", 3, \"🙂\")) }")
            .as_slice(),
        ["1.3", "-1.3", "130", "2", "5", "2", "true", "🙂🙂é"]
    );
}

#[test]
fn for_loops_destructure_typed_tuples() {
    let source = r#"
fn main() {
    prices: Map<String, Int> = Map { "Coffee": 3 "Tea": 2 }
    for (name, price) in entries(prices) {
        print(name + ":" + to_string(price))
    }
    for (left, right) in zip([1, 2], [3, 4]) {
        print(to_string(left + right))
    }
}
"#;
    assert_eq!(run(source), ["Coffee:3", "Tea:2", "4", "6"]);

    for (source, expected) in [
        (
            "fn main() { for (key, value) in [1, 2] { print(key) } }",
            "tuple loop pattern expects tuple values",
        ),
        (
            "fn main() { values = Map { \"a\": 1 } for (key, value, extra) in entries(values) { print(key) } }",
            "tuple loop pattern has 3 variables but the value has 2 elements",
        ),
    ] {
        let program = parse(&lex(source).unwrap()).unwrap();
        let errors = check(&program).expect_err("invalid tuple pattern should fail type checking");
        assert!(errors.iter().any(|error| error.message.contains(expected)));
    }
}

#[test]
fn map_increment_counts_safely_without_mutating_the_input_map() {
    let source = r#"
fn main() {
    counts: Map<String, Int> = Map {}
    match map_increment(counts, "pear") {
        Ok(first) => {
            match map_increment(first, "pear") {
                Ok(second) => { print(get_or_default(second, "pear", 0)) }
                Err(message) => { print(message) }
            }
        }
        Err(message) => { print(message) }
    }
    print(get_or_default(counts, "pear", 0))

    maximum: Map<String, Int> = Map { "overflow": 9223372036854775807 }
    match map_increment(maximum, "overflow") {
        Ok(_) => { print("unexpected success") }
        Err(message) => { print(message) }
    }
}
"#;
    assert_eq!(run(source), ["2", "0", "map counter overflow"]);

    for source in [
        "fn main() { values: Map<String, Bool> = Map { \"enabled\": true } result = map_increment(values, \"enabled\") }",
        "fn main() { values: Map<String, Int> = Map {} result = map_increment(values, 1) }",
    ] {
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(check(&program).is_err(), "unexpectedly accepted {source}");
    }
}

#[test]
fn standard_library_v103_phase3_builtins_work_properly() {
    let source = r#"
fn main() {
    // Dates
    d1 = date("2026-10-14")
    d2 = date_add_days(d1, 30)
    print(d2)
    print(date_diff_days(d2, d1))

    // Result & Option Flow
    print(is_ok(parse_int("42")))
    print(is_err(parse_int("42")))
    print(is_ok(parse_int("bad")))
    print(is_err(parse_int("bad")))

    print(is_some(Some(123)))
    print(is_none(Some(123)))
    print(is_some(None))
    print(is_none(None))
}
"#;
    assert_eq!(
        run(source),
        [
            "2026-11-13",
            "30",
            "true",
            "false",
            "false",
            "true",
            "true",
            "false",
            "false",
            "true",
        ]
    );
}

#[test]
fn standard_library_v103_phase4_builtins_work_properly() {
    let source = r#"
fn main() uses Random, FileSystem {
    // Hashes & Encodings
    sha = hash_sha256("hello")
    print(sha)

    b64 = base64_encode("hello world")
    print(b64)
    print(unwrap_or(match base64_decode(b64) { Ok(val) => Some(val), Err(_) => None }, "err"))

    ue = url_encode("hello world & foo=bar")
    print(ue)
    print(unwrap_or(match url_decode(ue) { Ok(val) => Some(val), Err(_) => None }, "err"))

    u = uuid_v4()
    print(len(u))

    // Text helpers
    print(text_trim_start("   abc  "))
    print(text_trim_end("   abc  "))
    print(text_is_blank("    "))
    print(text_is_blank("  a  "))
    print(text_substring("Zelyra", 2, 3))

    // Conversions
    print(to_int(3.85))
    print(to_float(42) + 0.5)
    print(is_ok(parse_bool("true")))
    print(is_ok(parse_bool("0")))
    print(is_err(parse_bool("not_a_bool")))

    // Date comparisons
    d1 = date("2026-01-01")
    d2 = date("2026-12-31")
    print(is_before(d1, d2))
    print(is_after(d1, d2))

    // Collections: entries, zip, dedup
    m = Map{"a": 1, "b": 2}
    ent = entries(m)
    print(len(ent))
    print(ent[0][0])

    z = zip([1, 2], ["x", "y"])
    print(len(z))
    print(z[0][0])
    print(z[0][1])

    d = dedup([1, 2, 1, 3, 2])
    print(d)

    // Path helpers
    joined = path_join("usr/local", "bin")
    print(joined)
    print(path_filename("var/log/app.log"))
    print(path_extension("var/log/app.log"))
    print(path_dir("var/log/app.log"))
}
"#;
    assert_eq!(
        run(source),
        [
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
            "aGVsbG8gd29ybGQ=",
            "hello world",
            "hello%20world%20%26%20foo%3Dbar",
            "hello world & foo=bar",
            "36",
            "abc  ",
            "   abc",
            "true",
            "false",
            "lyr",
            "3",
            "42.5",
            "true",
            "true",
            "true",
            "true",
            "false",
            "2",
            "a",
            "2",
            "1",
            "x",
            "[1, 2, 3]",
            "usr/local/bin",
            "app.log",
            "log",
            "var/log",
        ]
    );
}
