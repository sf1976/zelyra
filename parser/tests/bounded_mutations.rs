#[path = "../../tests/support/bounded_mutations.rs"]
mod bounded_mutations;

use zelyra_lexer::lex;
use zelyra_parser::parse;

#[test]
fn lexer_and_parser_handle_reproducible_mutated_sources() {
    let seeds = [
        "fn main() { print(1 + 2) }",
        "type CustomerId = Int\nrecord Customer { id: CustomerId, name: String }",
        "database app { engine: mariadb }\ntable customers { id: Int primary key }",
        "page /customers { title: \"Customers\" }",
        "api GET /customers { output Customer[] }",
        "sql<Customer[]> { SELECT id, name FROM customers WHERE id = :id }",
        "for customer in customers { {customer.name} }",
        "fn main( { while true { break }",
    ];
    for source in bounded_mutations::corpus(&seeds, 1024, 12) {
        let first = lex(&source);
        assert_eq!(first, lex(&source), "lexer output changed for {source:?}");
        if let Ok(tokens) = first {
            let _ = parse(&tokens);
        }
    }
}
