use super::*;
use std::collections::HashMap;

#[path = "../../tests/support/bounded_mutations.rs"]
mod bounded_mutations;

#[test]
fn http_parser_and_template_renderer_handle_reproducible_mutations() {
    let seeds = [
        "GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n",
        "POST /items HTTP/1.1\r\nHost: example.test\r\nContent-Length: 3\r\n\r\na=b",
        "GET /customers/{id}?q=hello HTTP/1.1\r\nX-Request-ID: abc-123\r\n\r\n",
        "for item in items {<p>{item.name}</p>} ",
        "for item in items {\n  {item.name}\n}",
        "for x in xs { for y in ys { {x.name}{y.id} } }",
        "for invalid item {",
    ];
    let mut collections = HashMap::new();
    collections.insert(
        "items".into(),
        vec![HashMap::from([("name".into(), "sample".into())])],
    );
    collections.insert(
        "ys".into(),
        vec![HashMap::from([("id".into(), "1".into())])],
    );
    let params = HashMap::new();
    let values = HashMap::new();

    for input in bounded_mutations::corpus(&seeds, 1024, 12) {
        let first_request = parse_request(&input);
        assert_eq!(first_request, parse_request(&input));
        let first_render = render_template_fragment(&input, &params, &values, &collections);
        assert_eq!(
            first_render,
            render_template_fragment(&input, &params, &values, &collections)
        );
    }
}
