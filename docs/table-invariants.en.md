# Table invariants

Tables may declare pure, typed cross-field rules. These predicates are lowered
to ordinary private `Bool` functions that receive the table's declared fields;
the normal type checker and runtime evaluate them. Messages are static string
literals and are HTML-escaped when shown.

```zelyra
table invoices {
    invoice_date: Date required
    due_date: Date?

    validate {
        match due_date {
            Some(due) => due >= invoice_date
            None => true
        }: "The due date must not precede the invoice date."
    }
}
```

Rules run after ordinary form-field and relation validation and before a
declarative form action or generated CRUD write. A failed rule returns HTTP
422, preserves safe form values, and appears in the form error summary. Rules
must evaluate to `Bool`; unknown fields and effectful operations are rejected
by the compiler. Each rule runs with a fixed 10,000-step budget. Optional
fields should be handled with `match` so the rule is defined for both `Some`
and `None`.

An API may explicitly bind its checked JSON input fields to a table's rules.
Typed values are checked before the handler runs; violations use the declared
HTTP 422 error contract. This validates submitted values, not arbitrary later
handler transformations or raw SQL statements.

~~~zelyra
table invoices {
    id: Id primary auto
    amount: Int required
    validate { amount > 0: "Amount must be positive." }
}

api POST "/invoices" {
    handler create_invoice
    input { amount: Int }
    validates invoices
    output Result<Int, String>
    errors { 422 ValidationError: String }
}
~~~

This binding currently supports POST, PUT, and PATCH for tables without a
tenant scope. The compiler rejects scoped tables until the API runtime can pass
a verified tenant context to the rule evaluation. This is not a database
constraint: raw SQL writes and other unbound handlers are not covered. Use
database constraints for invariants that must hold for every writer.
