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

For a scoped table, declare `requires auth`, and configure an `auth` block with
an active membership table. Do not put the tenant key in the API `input` block.
Zelyra resolves the optional `tenant_id` selector against the authenticated
session and active memberships, inserts the verified tenant value into rule
evaluation, and passes it as the handler's final parameter using the declared
tenant-column name and type:

```zelyra
type TenantId = Id

api POST "/api/invoice-check" {
    requires auth
    handler check_invoice
    validates invoices
    input { amount: Int }
    output Result<TenantId, String>
    errors { 422 ValidationError: String }
}

fn check_invoice(amount: Int, tenant_id: TenantId) -> Result<TenantId, String> {
    return Ok(tenant_id)
}
```

If a caller supplies the tenant key in the request body, Zelyra rejects the
request. Missing, conflicting, or unauthorized tenant selectors fail before
the handler. Tenant validation still covers only the declared input and
verified tenant context. It is not a database constraint: raw SQL writes and
unbound handlers are not covered. Use database constraints for invariants that
must hold for every writer.

A complete compiler-checked example is available in
[`examples/tenant_api_validation.zyl`](../examples/tenant_api_validation.zyl).
