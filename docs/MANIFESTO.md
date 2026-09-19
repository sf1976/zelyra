# The Zelyra Manifesto: Digital Sovereignty and Masterful Engineering

Zelyra stands for digital self-determination: software should serve people
and their applications, not a platform they do not control. This manifesto is
a binding compass for architecture, product decisions, and public claims.

The values below describe a destination, not a blanket certification of the
current implementation. Implemented capabilities, open work, and evidenced
guarantees are tracked in the [roadmap](ROADMAP.md), the
[language specification](specification.md), and test results.

> **Zelyra — From database to application.**
>
> **Describe intent. Prove correctness.**

## Z — Reliability (Zero Compromise)

Zelyra favors explicit types, null safety, checked SQL and schema relations,
contracts, capabilities, and reproducible tests. The verifier distinguishes
`PROVEN`, `RUNTIME_CHECK`, `UNPROVEN`, and `FAILED`. A proof applies only to
its stated scope. Design by contract is not a universal mathematical proof
that an entire application is defect-free; Zelyra must not claim otherwise.

## E — Independence (Sovereignty)

Zelyra should be usable and self-hostable without a mandatory cloud account,
AI provider, or closed runtime platform. Interfaces should be open, external
effects visible, and dependencies inspectable and replaceable. Independence
does not mean that a real application may never use libraries, networks, or
external services.

## L — Logical Clarity

Source should say what it does. Types, data models, permissions, and effects
should remain visible and checkable. Convenient abstractions such as CRUD must
not conceal SQL, business logic, or security boundaries or take control away
from the developer.

## Y — Yield (Productive Leverage and Precision)

A domain definition should safely inform as much of the application as
possible: schema, types, SQL checking, validation, forms, views, CRUD, and
APIs. Reuse should reduce duplication without turning business logic into
opaque magic. Claims about speed, memory use, or developer productivity need
reproducible measurements.

## R — Robustness (Bootstrapped in Rust today)

Zelyra should earn robustness through careful implementation, security
boundaries, regression tests, and measured resource budgets. The current
bootstrap compiler is written in Rust. Rust is not the Zelyra language or a
required end-user runtime dependency; the long-term self-hosting path is
planned, not complete.

## A — Autonomy (Data Sovereignty)

Users should control their data, infrastructure, and deployment location.
Local execution, open data formats, and documented backend boundaries are
guiding principles. MariaDB is currently the primary backend; SQLite is
supported, and PostgreSQL has schema and planning support but not full runtime
parity.

Product policy: Zelyra must collect no telemetry. Analytics, crash reporting,
and background phone-home behavior are prohibited. Network access for
explicitly chosen application features or deliberately invoked tools is
different, but must remain visible, bounded, and reviewable. An automated CI
regression guard against future unsolicited network activity remains planned;
see the roadmap.
Exports of the user's own application or audit data, explicitly initiated by
the user, are data-sovereignty operations, not usage telemetry.

## Binding decision questions

Every new feature, dependency, default, and public claim must be reviewed:

1. Do application code, data, and infrastructure remain under user control?
2. Does the design create mandatory cloud, vendor, or network dependence?
3. Is every external effect explicit, with understandable data flows?
4. Are dependencies, data formats, and backend boundaries open and maintainable?
5. Does the simple default path remain simple, with added complexity optional?
6. Are correctness, security, performance, and independence claims limited to
   a tested scope?
7. Are memory and runtime costs measured instead of advertised with
   superlatives?

If an answer misses the goal, state that plainly and record it as a limitation
or roadmap item. The [contribution rules](../AGENTS.md) make this review
mandatory.
