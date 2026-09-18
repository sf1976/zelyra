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

See the maintained [roadmap](docs/ROADMAP.md) for required and optional future
work, including the Views System and cryptographic audit chaining.

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
- an initial Web Core with page definitions, reusable named views with content
  slots, GET routing, path parameters, and a built-in HTTP server;
- an initial Forms Core with schema-aware field definitions and validation;
- validated form actions with safe MariaDB parameter binding, transactions,
  HTTP redirects, and action-level authentication/permission checks.
- an initial CRUD resource with MariaDB search, configurable list/search/filter
  columns, sorting, pagination, generated Create/Edit forms, and
  CSRF-protected delete with separate view/create/edit/delete permissions and
  backward-compatible permission fallback; generated views hide unavailable
  actions while direct requests remain protected.
- initial authentication guards, Argon2 login against a MariaDB user table,
  HttpOnly sessions, logout, direct and role-based permission checks.
- CLI role and role-permission grant/revoke commands with schema validation and
  idempotent MariaDB writes.
- opt-in browser user and role administration with CSRF protection, permission
  guards, password reset, activation/deactivation, and last-administrator
  protection.
- optional MariaDB audit logging for login, logout, password, user, role, and
  permission events, with the latest 100 events visible in administration and
  inspectable/exportable through the CLI.
- typed API declarations with route/type validation, optional executable
  handlers, OpenAPI 3.0.3 output through `zelyra doc`, authentication and
  permission guards, structured JSON errors, and declared `Result` error
  mapping, including typed API arrays and nested JSON objects via `struct`
  records;
- a dependency-free TypeScript client generator via
  `zelyra doc <file.zyl> --typescript`, including typed declared API error
  codes and structured HTTP-error parsing;
- typed API error payloads through `errors { 422 ValidationError: Problem }`,
  including `error.details`, OpenAPI schemas, and TypeScript payload types;
- exact-origin CORS configuration for browser APIs with automatic `OPTIONS`
  preflight handling, disabled by default;
- array literals, indexing, `len`, `append`, `contains`, `first`, `last`, and
  array concatenation with `+`, with `Option` results for empty-safe queries;
- structured `for ... in` array iteration with `break` and `continue`;
- record literals and checked field access for nested business values;
- initial capability declarations and static propagation through function calls;
  native SQL requires the `Database` capability, with project grants from
  `zelyra.toml`.
- runtime enforcement of declared function capabilities, native SQL, forms,
  CRUD, and authentication database access when project grants are supplied by
  the CLI;
- safe Clock and Environment host APIs through now() and env(name); both
  require explicit function declarations and project grants;
- secure random_int(min, max) through the Random capability, with inclusive
  bounds and runtime rejection of invalid ranges;
- read_text(path), write_text(path, content), delete_file(path), and
  list_dir(path) through the FileSystem capability with project path roots;
- http_get(url) through the Network capability with project host allowlists,
  timeouts, and response-size limits;
- http_request(method, url, headers, body) with typed HttpResponse values;
- json_encode(value) and json_decode<Type>(text) for checked JSON conversion
  of records, arrays, options, and scalar values;
- http_json<Request, Response>(...) for automatic typed JSON HTTP requests and
  responses;
- http_result<Request, Response>(...) returning a typed `HttpResult<Response>`
  with status, headers, body, data, and structured HTTP errors;
- run_process(command, args) through Process with an exact command allowlist,
  no shell, timeout, and bounded output;
- initial runtime-checked function contracts with `requires` and `ensures`;
  these checks are not presented as formal proofs.
- an initial structured-concurrency slice with `parallel` and `await`: branches
  run from an immutable environment snapshot, are joined before continuation,
  and merge their results in source order;
- an initial `zelyra verify` command distinguishing `PROVEN`, `RUNTIME_CHECK`,
  `UNPROVEN`, and `FAILED` for contract expressions, including simple symbolic
  integer relationships in direct-return postconditions, basic control-flow
  paths, `Option`/`Result` constructor paths, known payload bindings, and
  bounded path-sensitive function-call summaries, local bindings with simple
  linear assignments, bounded loops, and modeled `break`/`continue` paths;
  explicit loop invariants with individual verification statuses and
  caller-assumption-aware callee precondition checks are included. Each result
  includes a stable code, source range, explanation, and marked source excerpt;
  a bounded counterexample is included when it can be safely found. The
  current search covers up to three linear integer variables and also reports
  witnesses for failed loop invariants.
  `zelyra verify <file.zyl> --json` provides structured output with `message`
  and `counterexample` for IDEs and CI.

Full CRUD generation, user administration and richer language-level domain-error values,
general formal verification, operating-system capability integration, and
production deployment tooling are still being developed. See
the
[roadmap](#roadmap) and the detailed
[Getting Started guide](docs/getting-started.md).

Zelyra's concrete product differentiation is documented in the
[positioning and unique strengths guide](docs/positioning.md).

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

Check project readiness before starting a web application:

~~~bash
zelyra doctor examples/machine_management.zyl
~~~

For CI or IDE tooling, use `zelyra doctor ... --json` for machine-readable
checks without exposing database credentials.

The installer builds Zelyra for the current user and installs the executable
in the user-local bin directory. It does not require sudo, a global Rust
installation, Apache, or a database server for the language-core examples.
On Windows, run `install.ps1` in PowerShell or use `install.cmd`; it installs
to the user's local application directory and updates the user PATH without
administrator privileges.

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

Reusable views let a page customize its own content while sharing a safe
application shell:

~~~zelyra
view SiteShell {
    html {
        <html><body><main><slot /></main></body></html>
    }
}

page "/customers" {
    view: SiteShell
    html { <h1>Customers</h1> }
}
~~~

The compiler requires exactly one `<slot />` in every named view. The page
content is composed into that slot before routing, so authentication and
escaping continue to use the existing web pipeline. See
`examples/views.zyl` for a complete example.

Views can also declare typed, reusable components:

~~~zelyra
component Badge {
    props { text: String }
    html { <span class="badge">{text}</span> }
}

page "/status" {
    html { <Badge text="Ready" /> }
}
~~~

Components can also wrap arbitrary HTML through one default slot:

~~~zelyra
component Panel {
    html { <section class="panel"><slot /></section> }
}

page "/dashboard" {
    html { <Panel><h1>Dashboard</h1></Panel> }
}
~~~

Components may also declare named slots with `<slot name="header" />`; callers
provide them with `<slot name="header">...</slot>` blocks. Nested components
are expanded from the inside out, and unknown or unused content is rejected.
See `examples/component_slots.zyl`.

Search, filtering, sorting, and pagination are already available on generated
CRUD lists. Filters expose type-aware operators such as `contains`, `gte`, and
`is_null`; the same controls preserve their state in the URL. For example:

~~~text
/customers?filter_name__contains=Press
/customers?filter_quantity__gte=10
~~~

CRUD list presentation can be changed declaratively without replacing the
checked data or authorization pipeline:

~~~zelyra
crud Customer -> customers {
    view {
        list {
            mode: cards
            empty: "No customers found."
        }
    }
}
~~~

The generated `table` (default) and `cards` modes keep search, typed filters,
allowlisted sorting, pagination, URL state, escaping, and permission checks.

Detail presentation can be customized independently:

~~~zelyra
crud Customer -> customers {
    view {
        detail {
            mode: cards
            title: "Customer details"
        }
    }
}
~~~

The default detail mode is standard. Both modes retain generated edit, create,
delete, CSRF, escaping, and authorization safeguards.

CRUD forms can use the same controlled layout system:

~~~zelyra
view {
    form {
        mode: cards
        title: "Customer form"
        submit: "Save customer"
    }
}
~~~

The title and submit label are escaped, and the form keeps schema validation,
readonly checks, CSRF protection, parameter binding, and action permissions.

The delete confirmation can also be customized:

~~~zelyra
view {
    delete {
        title: "Delete customer"
        message: "This cannot be undone."
        submit: "Delete now"
    }
}
~~~

The generated endpoint remains POST-only and continues to require CSRF and
delete authorization checks.

CRUD loading and error states can be configured as well:

~~~zelyra
view {
    loading { message: "Loading customers..." }
    error {
        title: "Customer unavailable"
        message: "Please try again later."
    }
}
~~~

The loading message is emitted as escaped metadata for progressive enhancement.
The server-rendered response never pretends that a loading state is active.
Configured error messages replace generic CRUD database-error pages while
internal database details remain hidden.

Custom CRUD actions can add focused business operations without replacing the
generated safety pipeline:

~~~zelyra
crud Customer -> customers {
    action deactivate {
        permits "customers.edit"
        sql {
            UPDATE customers
            SET active = false
            WHERE id = :id
        }
        success "Customer deactivated."
        redirect "/customers"
    }
}
~~~

Zelyra generates a POST-only endpoint at
`/customers/{id}/deactivate` and renders its button on the detail view. The
request requires the CRUD database capability, CSRF protection, authentication
and the declared permission. `:id` is bound from the route; SQL remains
parameterized. Action names are currently also used as button labels.

The first typed slice of the unified view data pipeline is now available on
`tableview` routes; applying the same operations to arbitrary views remains
planned.

Standalone typed table views can already expose a checked MariaDB query. Their
result may be a declared table type or a dedicated `struct` for joins and
aggregates:

~~~zelyra
tableview Customers {
    source sql<CustomerOverview[]> {
        SELECT c.id, c.name, COUNT(o.id) AS orders
        FROM customers c LEFT JOIN orders o ON o.customer_id = c.id
        GROUP BY c.id, c.name
    }
    columns { id name orders }
    filter { name orders }
    searchable
    sortable
    paginated 25
}
~~~

This creates a server-rendered view at `/views/customers`. Its SQL source,
aliases, result fields, and declared columns are checked against the schema and
the struct; typed filters, search, sorting, pagination, URL state, and HTML
escaping remain on the safe server-side path. See
`examples/tableview.zyl` for a complete MariaDB example.

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
zelyra new <directory> [--mariadb]
zelyra init [directory]
zelyra check <file.zyl>
zelyra build <file.zyl>
zelyra run <file.zyl>
zelyra serve <file.zyl> [address]
zelyra doctor [file.zyl] [--port <port>] [--json]
zelyra verify <file.zyl> [--json]
zelyra doc <file.zyl> [--openapi|--typescript]
zelyra auth hash-password [--stdin]
zelyra auth role <grant|revoke> <file.zyl> <user-id> <role>
zelyra auth role-permission <grant|revoke> <file.zyl> <role> <permission>
zelyra audit inspect <file.zyl> [--limit <n>]
zelyra audit export <file.zyl> [--limit <n>] [--format json|csv]
zelyra audit verify <file.zyl>
zelyra audit prune <file.zyl> --before <timestamp> [--confirm]
zelyra form validate <file.zyl> <FormName> [field=value ...]
zelyra db create <file.zyl>
zelyra db setup <file.zyl>
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
   configurable columns, relationship labels and selects, and CSRF-protected
   delete, and separate action permissions implemented.
8. Authentication and authorization — Argon2 login, persistent MariaDB
   sessions, logout, route guards, direct and role-based permission lookup,
   and opt-in role administration.
9. Capabilities and contracts — initial declarations, static checks, runtime
   contracts, limited symbolic verification, runtime capability boundaries,
   and an initial `parallel`/`await` structured-concurrency slice are
   implemented; operating-system privilege integration and general formal
   verification remain.
10. Typed API declarations, executable handlers, API authentication and
    permissions, and OpenAPI 3.0.3 generation are implemented; client state,
    WebAssembly, and optimization interfaces remain.
11. Browser API integration with explicit CORS origins and automatic preflight
    handling is implemented.
12. API request media-type and body-size validation, complete multi-read
    request handling, and secure default response headers are implemented.
13. MariaDB CRUD end-to-end coverage and GitHub Actions CI are implemented.
14. Tag-based Linux and Windows release archives with SHA-256 checksums are
    generated automatically.
15. MariaDB tableview end-to-end coverage now verifies struct-backed joins,
    aggregates, escaping, search, sorting, and pagination through the web
    server.
16. Bootstrap independence and a self-hosting compiler are strategic goals;
    the current compiler bootstrap remains Rust while released users do not
    need Rust installed.

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

GitHub Actions additionally runs the credential-free MariaDB CRUD integration
test from `tests/mariadb-e2e.sh` against an isolated MariaDB 11 service.
The tableview integration test from `tests/mariadb-tableview-e2e.sh` additionally
executes a struct-backed join and aggregate view through the running web server,
including search, sorting, pagination, and HTML escaping.
The MariaDB authentication integration test from
`tests/mariadb-auth-e2e.sh` additionally covers login, persistent sessions,
permission denial, and logout.
The protected CRUD/API test from `tests/mariadb-protected-e2e.sh` verifies the
permission boundary for HTML CRUD and JSON API endpoints, including separate
Create, Edit, and Delete permissions, plus a protected custom form action.

Please keep German and English user documentation synchronized. Architectural
decisions should preserve safety, control, and extensibility.

## License

Zelyra is released under the MIT License. See [LICENSE](LICENSE).
