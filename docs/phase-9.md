# Zelyra 0.1 — Phase 9: Capabilities

[Deutsch](phase-9.de.md) · English

Zelyra functions can explicitly declare the external capabilities they need:

~~~zelyra
fn send_invoice(invoice: String)
    uses Network
{
    print(invoice)
}
~~~

The initial capability set is:

~~~text
Database
Network
FileSystem
Environment
Process
Clock
Random
~~~

The compiler rejects unknown or duplicate capability names. Capability
requirements are also propagated through function calls: a function that calls
`send_invoice` must itself declare `uses Network`.

Native SQL requires `Database`:

~~~zelyra
fn load_customer() -> Customer[]
    uses Database
{
    return sql<Customer[]> {
        SELECT id, name FROM customers
    }
}
~~~

The declaration is checked statically and is visible in the AST and HIR. This
phase does not yet grant operating-system privileges or implement network and
file APIs. Runtime capability enforcement for those APIs, project-level grants,
and capability-aware concurrency remain future work.

## Diagnostics

Missing `Database` for SQL produces an error such as:

~~~text
error[E-CAP-001]: function `load_customer` uses SQL but does not declare capability `Database`; add `uses Database`
~~~

The CLI checks capabilities as part of `zelyra check`, `zelyra build`, `zelyra
run`, and `zelyra serve`.
