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

The declaration is checked statically and is visible in the AST and HIR. A
project can grant capabilities explicitly in `zelyra.toml`:

~~~toml
[capabilities]
database = true
network = false
~~~

When a project file is present, every declared capability must be enabled
there. A missing entry or `false` value is denied by `zelyra check`, `build`,
`run`, and `serve`. Standalone source files without a project file retain the
single-file development behavior and only receive declaration checking.

This phase does not yet grant operating-system privileges or implement network
and file APIs. Runtime capability enforcement for those APIs and
capability-aware concurrency remain future work.

## Diagnostics

Missing `Database` for SQL produces an error such as:

~~~text
error[E-CAP-001]: function `load_customer` uses SQL but does not declare capability `Database`; add `uses Database`
~~~

The CLI checks capabilities as part of `zelyra check`, `zelyra build`, `zelyra
run`, and `zelyra serve`.

## Contracts

Functions can also declare runtime-checked preconditions and postconditions:

~~~zelyra
fn increment(value: Int) -> Int
    requires { value >= 0 }
    ensures { result > value }
{
    return value + 1
}
~~~

`requires` expressions run before the function body. `ensures` expressions run
after it, with the returned value available as `result`. Both expressions must
have type `Bool`. A false condition stops execution with a diagnostic runtime
error. These checks are runtime checks, not mathematical proofs. Formal
verification and proof output remain future work.

## Verification command

The first verifier is available through:

~~~bash
zelyra verify examples/contracts.zyl
~~~

It reports one status for every contract. Constant boolean expressions can be
classified as `PROVEN` or `FAILED`. The initial symbolic verifier also
substitutes a direct integer return expression into `ensures` and proves simple
affine relationships such as `result > value` for `return value + 1`.
Conditions that cannot be decided remain `RUNTIME_CHECK`; functions without
contracts are reported as `UNPROVEN`.

The command exits unsuccessfully for a failed constant contract or a compiler
diagnostic. No status other than `PROVEN` is a mathematical proof.
