# Zelyra 0.1

![Zelyra logo](assets/zelyra-logo.png)

**From database to application.**
**Describe intent. Prove correctness.**

[Deutsch](README.de.md) · English

Zelyra is a statically typed programming language for business, database, and
web applications. It is designed around a simple idea: the information that
describes a business object should be reusable throughout the application.

A table definition should be able to provide the foundation for types, SQL
checking, validation, forms, APIs, and CRUD — while ordinary programming,
native SQL, and custom business logic remain available whenever they are
needed.

## Current status

Zelyra 0.1 is an active early implementation. The repository is real,
buildable, tested Rust code, but the complete long-term language specification
is not implemented yet.

## License and implementation

Zelyra is implemented in Rust. Rust is used as the implementation language;
Zelyra is not an official Rust project and does not use the Rust name or logo
as a product mark.

The Zelyra source is licensed under either the MIT License or the Apache
License, Version 2.0, at the licensee's option. See [LICENSE-MIT](LICENSE-MIT)
and [LICENSE](LICENSE). Third-party dependency information is collected in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

Implemented today:

- language core with variables, functions, expressions, control flow, and
  immutable-by-default bindings;
- static type checking, nominal types, Option, Result, and pattern matching;
- schema definitions and schema DDL planning for MariaDB, SQLite, and
  PostgreSQL;
- MariaDB database inspection, schema application, and native SQL runtime
  execution;
- native SQL blocks with schema, column, parameter, and result checks;
- an initial Web Core with page definitions, GET routing, path parameters, and
  a built-in HTTP server;
- an initial Forms Core with schema-aware field definitions and validation;
- validated form actions with safe MariaDB parameter binding, transactions, and
  HTTP redirects.
- an initial CRUD resource with MariaDB search, configurable list/search/filter
  columns, sorting, pagination, generated Create/Edit forms, and
  CSRF-protected delete.
- initial authentication guards, Argon2 login against a MariaDB user table,
  HttpOnly sessions, logout, and permission checks.
- initial capability declarations and static propagation through function calls;
  native SQL requires the `Database` capability, with project grants from
  `zelyra.toml`.
- initial runtime-checked function contracts with `requires` and `ensures`;
  these checks are not presented as formal proofs.
- an initial `zelyra verify` command distinguishing `PROVEN`, `RUNTIME_CHECK`,
  `UNPROVEN`, and `FAILED` for contract expressions, including simple symbolic
  integer relationships in direct-return postconditions, basic control-flow
  paths, `Option`/`Result` constructor paths, known payload bindings, and
  bounded path-sensitive function-call summaries, local bindings with simple
  linear assignments, bounded loops, and modeled `break`/`continue` paths;
  explicit loop invariants with individual verification statuses and
  caller-assumption-aware callee precondition checks are included. Each result
  includes a stable code, source range, explanation, and marked source excerpt;
  `zelyra verify <file.zyl> --json` provides structured output with `message`
  for IDEs and CI.

Full CRUD generation, database roles, APIs, general formal verification,
and production deployment tooling are still being developed. See the
[roadmap](#roadmap) and the detailed
[Getting Started guide](docs/getting-started.md).

## Quick start

The easiest path from a source checkout is:

~~~bash
git clone https://github.com/sf1976/zelyra.git
cd zelyra
./install.sh
zelyra run examples/fibonacci.zyl
~~~

Expected output:

~~~text
55
~~~

The installer builds Zelyra for the current user and installs the executable
in the user-local bin directory. It does not require sudo, a global Rust
installation, Apache, or a database server for the language-core examples.

Run a source file without installing the binary:

~~~bash
cargo run -p zelyra-cli -- run examples/fibonacci.zyl
~~~

Check a program without executing it:

~~~bash
zelyra check examples/fibonacci.zyl
~~~

For the complete beginner path, including troubleshooting and database setup,
read [Getting Started](docs/getting-started.md) or
[Erste Schritte auf Deutsch](docs/getting-started.de.md).

## A first web page

Zelyra includes a small built-in HTTP server. Apache is optional; it is not
required to begin developing a web application.

~~~zelyra
page "/hello/{name}" {
    html {
        <html>
            <body>
                <h1>Hello, {name}!</h1>
            </body>
        </html>
    }
}
~~~

Start it:

~~~bash
zelyra serve examples/hello_web.zyl
~~~

Open http://127.0.0.1:3000/hello/Zelyra. Route parameters are HTML-escaped by
default. The current Web Core supports the first safe vertical slice: GET
routes, path parameters, query-string handling, request parsing, and HTML
responses.

## A first schema-aware form

Forms can reuse constraints from a table:

~~~zelyra
table customers {
    id: Id primary auto
    name: String(100) required
    email: Email?
}

form CustomerCreate -> customers {
    fields {
        name
        email
    }
}
~~~

Validate input locally:

~~~bash
zelyra form validate examples/customer_form.zyl CustomerCreate \
  name=Anna email=anna@example.test
~~~

The form inherits the required name field and its length from the schema.
Unknown fields, missing values, invalid email addresses, invalid numbers,
invalid booleans, and submitted readonly fields are rejected.

Start the form server without Apache:

~~~bash
zelyra serve examples/customer_form.zyl
~~

The form is available at http://127.0.0.1:3000/forms/CustomerCreate. GET
renders the fields and a CSRF token; POST checks the token and validates the
submitted values. The form without an action is validation-only.

## A first MariaDB form action

`examples/customer_form_action.zyl` demonstrates a complete database-backed
form:

~~~bash
export DATABASE_URL='mariadb://root:<password>@127.0.0.1:3306/zelyra_forms'
zelyra db bootstrap examples/customer_form_action.zyl
zelyra serve examples/customer_form_action.zyl
~~~

Open http://127.0.0.1:3000/forms/CustomerCreate. The POST request is checked
for CSRF and schema validation, binds only declared form fields as database
parameters, executes the SQL in a MariaDB transaction, and returns HTTP 303 to
the declared redirect. Missing `DATABASE_URL` returns HTTP 503; database
failures are returned as a controlled HTTP 500 without exposing credentials or
SQL details. Apache is not required.

Relationship fields become select controls automatically. The example
`examples/machine_form.zyl` defines `department: Department required`; Zelyra
loads the department IDs and display names from MariaDB, renders a `<select>`,
and rejects IDs that are not present in the database.

## Database-first development

The intended Zelyra flow is:

~~~text
Database definition
        ↓
Schema model
        ↓
Types and relationships
        ↓
Checked SQL
        ↓
Forms and validation
        ↓
Pages, APIs, and CRUD
~~~

MariaDB is the default backend for new Zelyra definitions and the primary
runtime reference. SQLite is available for small local applications and
testing. PostgreSQL schema support is also part of the Database Core.

Example:

~~~zelyra
database main {
    engine: mariadb
}

table customers {
    id: Id primary auto
    customer_number: String(20) required unique
    name: String(100) required
    email: Email?
    active: Bool default true
}
~~~

Inspect or apply the desired schema:

~~~bash
export DATABASE_URL='mariadb://user:password@127.0.0.1:3306/my_app'
zelyra db inspect examples/machine_management_mariadb.zyl
zelyra db plan examples/machine_management_mariadb.zyl
zelyra db apply examples/machine_management_mariadb.zyl
~~~

For SQLite:

~~~bash
export DATABASE_URL='sqlite:///tmp/my-app.sqlite3'
zelyra db bootstrap examples/machine_management_sqlite.zyl
~~~

Do not commit real credentials. Use environment variables or a secret manager.
The examples use MariaDB first because it is the default project backend.

## Native SQL

SQL is a language element, not an untyped string:

~~~zelyra
customer = sql<Customer?> {
    SELECT id, name, email
    FROM customers
    WHERE id = :id
}
~~~

Zelyra checks table names, columns, aliases, parameters, nullability, and
result mappings where the schema is available. Named parameters are bound
safely. Complex SQL remains possible; Zelyra does not require an ORM-style
method chain.

## Language principles

- Values are immutable by default. Mutation requires the explicit
  mutable declaration.
- Normal types are never null. Optional values use the explicit Option form.
- Domain identifiers can be nominally distinct, so a UserId cannot silently be
  used as an OrderId.
- Functions describe errors explicitly instead of relying on hidden exceptions
  as ordinary control flow.
- SQL parameters are bound safely.
- HTML output is escaped by default.
- Schema changes are inspected and destructive changes require explicit
  approval.
- Capabilities are partially implemented: declarations, known-name checking,
  call propagation, and the `Database` requirement for native SQL are active.
  Advanced contracts and general formal verification remain roadmap goals.

## CLI

Currently available:

~~~text
zelyra new <directory>
zelyra init [directory]
zelyra check <file.zyl>
zelyra build <file.zyl>
zelyra run <file.zyl>
zelyra serve <file.zyl> [address]
zelyra verify <file.zyl> [--json]
zelyra form validate <file.zyl> <FormName> [field=value ...]
zelyra db create <file.zyl>
zelyra db bootstrap <file.zyl>
zelyra db inspect <file.zyl>
zelyra db plan <file.zyl>
zelyra db apply <file.zyl> [--allow-destructive]
~~~

The commands are intentionally small and explicit. Apache, PHP, an ORM, and a
frontend framework are not prerequisites for the examples above.

## Project layout

~~~text
zelyra/
├── ast/          Abstract syntax tree and language data model
├── lexer/        Source and raw SQL/HTML tokenization
├── parser/       Zelyra syntax parser
├── hir/          Name resolution and high-level IR
├── database/     Schema model, SQL checking, and database backends
├── forms/        Schema-aware form checking and validation
├── web/          Router, HTTP model, escaping, and built-in server
├── runtime/      Type checker and interpreter
├── cli/          zelyra command-line interface
├── examples/     Small runnable language examples
├── docs/         German and English documentation
└── tests/        Cross-crate acceptance tests
~~~

The compiler bootstrap is written in Rust and built as a Cargo workspace.

## Roadmap

The long-term specification is organized into these phases:

1. Language Core — implemented foundation.
2. Type System — implemented initial form.
3. Database Core — implemented initial MariaDB, SQLite, and PostgreSQL
   support.
4. Native SQL — implemented static checking and MariaDB execution.
5. Web Core — initial pages and HTTP server implemented.
6. Forms — schema-aware parsing, validation, web rendering, actions, and
   relationship selects implemented.
7. CRUD — list, detail, create, edit, search, filter, sort, pagination,
   configurable columns, and CSRF-protected delete implemented; authorization
   remains.
8. Authentication and authorization — Argon2 login, persistent MariaDB
   sessions, logout, route guards, and database-backed permission lookup.
9. Capabilities and contracts — initial declarations, static checks, runtime
   contracts, and limited symbolic verification are implemented; runtime
   privilege enforcement, general formal verification, and structured
   concurrency remain.
10. APIs, OpenAPI, client state, WebAssembly, and optimization interfaces.

Each feature is expected to include syntax, AST/HIR support, diagnostics,
positive and negative tests, documentation, and examples.

## Contributing

The repository is intentionally developed in small, testable phases. Before
changing a phase:

~~~bash
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
~~~

Please keep German and English user documentation synchronized. Architectural
decisions should preserve safety, control, and extensibility.

## License

Zelyra is released under the MIT License. See [LICENSE](LICENSE).
