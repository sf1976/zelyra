# The Zelyra Handbook

**From database to application. Describe intent. Prove correctness.**

English · [Deutsche Ausgabe](../de/README.md)

Welcome to Zelyra, the programming language for people who wanted to build a
customer database and somehow ended up maintaining seven frameworks, three
configuration formats, and a small existential crisis.

Zelyra combines a statically typed language with MariaDB schemas, checked SQL,
web pages, forms, CRUD, authentication, capabilities, and contracts. This
handbook builds a small machine-management application, starting with the first
program and ending with a database-backed web application.

> **Project status:** Zelyra 0.1 is experimental. Many foundations described
> here are implemented, but the project is not ready for production use.

The binding product goals for digital sovereignty and evidence-based
correctness claims are described in the [Zelyra manifesto](../../MANIFESTO.md).

## Status marks

- ✅ **Implemented:** available in the current repository.
- 🧪 **Experimental:** available, but young or limited.
- 🗺️ **Planned:** part of the language vision, not reliably available yet.

## Contents

1. [What makes Zelyra different](#1-what-makes-zelyra-different)
2. [Installation](#2-installation)
3. [Your first program](#3-your-first-program)
4. [Projects and the CLI](#4-projects-and-the-cli)
5. [Variables, types, and functions](#5-variables-types-and-functions)
6. [Option, Result, and pattern matching](#6-option-result-and-pattern-matching)
7. [MariaDB and tables](#7-mariadb-and-tables)
8. [Inspecting and applying schemas](#8-inspecting-and-applying-schemas)
9. [Native SQL](#9-native-sql)
10. [Web pages](#10-web-pages)
11. [Forms](#11-forms)
12. [CRUD](#12-crud)
13. [Authentication and permissions](#13-authentication-and-permissions)
14. [Capabilities](#14-capabilities)
15. [Contracts and verification](#15-contracts-and-verification)
16. [Configuration and secrets](#16-configuration-and-secrets)
17. [Diagnostics and troubleshooting](#17-diagnostics-and-troubleshooting)
18. [Testing and contributing](#18-testing-and-contributing)
19. [What comes next](#19-what-comes-next)
20. [AI-native development with Zelyra](#20-ai-native-development-with-zelyra)

## 1. What makes Zelyra different

A typical business application describes the same fact repeatedly: in the
database, backend, form, and API. Zelyra aims to turn those copies into one
traceable chain:

~~~text
Table → Types → SQL → Forms → CRUD → Web page → API/OpenAPI
~~~

This field:

~~~zelyra
email: Email?
~~~

already says that the value is an email address, may be absent, affects SQL
nullability, can select a suitable form control, and must be handled safely by
views.

SQL remains SQL. Zelyra does not force a respectable `JOIN` to disguise itself
as a 38-link method chain. SQL has suffered enough.

## 2. Installation

For language-only examples you need Linux or macOS, `curl`, and a working
shell. MariaDB is required for database actions, authentication, and CRUD.

~~~bash
git clone https://github.com/sf1976/zelyra.git
cd zelyra
./install.sh
zelyra --help
~~~

The installer is repeatable and user-local. Inspect or control it with:

~~~bash
./install.sh --help
./install.sh --dry-run --root "$HOME/.local"
./install.sh --check
zelyra update --check
zelyra update
./install.sh --uninstall
~~~

Use `--no-rustup` to disable automatic Rust installation, `--no-path` to
silence PATH guidance, or `--root PATH` / `ZELYRA_INSTALL_ROOT` to choose a
different user-owned target. Stale `cargo` PATH entries are detected instead
of being executed blindly.

Use `zelyra update --check` to check for a newer stable release without
changing files. `zelyra update` downloads the standalone Linux or Windows
x86_64 binary, verifies its SHA-256 checksum, and replaces only the executable
that was launched. It does not downgrade a newer local build or change project
files. On Windows, replacement completes immediately after the update command
exits. Other platforms use their documented installation method until release
binaries are published for them.

Published Linux x86_64 and Windows x86_64 releases can be installed without
Rust or Cargo. The installer downloads the selected archive over HTTPS and
verifies its SHA-256 checksum:

~~~bash
./install.sh --release v0.1.50
~~~

On Windows, use `-Release v0.1.50` with `install.ps1` in PowerShell. macOS
currently uses the source installer.

If the shell cannot find Zelyra:

~~~bash
export PATH="$HOME/.local/bin:$PATH"
~~~

Run directly from the source tree:

~~~bash
cargo run -p zelyra-cli -- run examples/fibonacci.zyl
~~~

It is longer, but ideal when you miss Cargo terribly.

## 3. Your first program

Create `hello.zyl`:

~~~zelyra
fn main() {
    print("Hello from Zelyra")
}
~~~

Run it:

~~~bash
zelyra run hello.zyl
~~~

A slightly more ambitious program:

~~~zelyra
fn fibonacci(n: Int) -> Int {
    if n <= 1 {
        return n
    }

    return fibonacci(n - 1) + fibonacci(n - 2)
}

fn main() {
    print(fibonacci(10))
}
~~~

The result is `55`. The computer survived. We may continue.

## 4. Projects and the CLI

Create a project:

~~~bash
zelyra new machine-management
cd machine-management
zelyra run main.zyl
~~~

Or initialize an existing directory:

~~~bash
mkdir machine-management
cd machine-management
zelyra init
~~~

For a local MariaDB and web-server template, use `zelyra new machine-management
--mariadb --web-port 8080 --host-port 18080 --db-host-port 3307`. This creates `.env.example`,
`Dockerfile`, and `docker-compose.mariadb.yml`. The options choose the internal
Zelyra server port and its local published port independently. You can change
`ZELYRA_WEB_PORT`, `ZELYRA_HOST_PORT`, and `ZELYRA_DB_HOST_PORT` later in
`.env`; the web ports default to 3000 and the MariaDB host port defaults to
3306.
Current `zelyra new --mariadb` and `zelyra init --mariadb` create this protected
file with random local MariaDB credentials immediately. Existing `.env` files
are never overwritten and credentials are never printed. Only required database
values are active; ports, feature
switches, authentication, and other options are documented as commented
examples. `zelyra setup` remains an idempotent recovery command.
When it creates a missing `.env`, setup selects free published web and MariaDB
ports if defaults are occupied. Use `--host-port` and `--db-host-port` for
strict choices; existing `.env` files are never modified by these flags.
If an existing project declares MariaDB in `zelyra.toml` but has no
`.env.example`, setup uses the same safe built-in defaults. A project without
MariaDB configuration receives a concrete `zelyra new --mariadb` remedy.
For a non-interactive console setup use `zelyra setup --all`. To use the same
actions in a local browser, run `zelyra setup --web`; the CLI prints a
tokenized `127.0.0.1` URL. See [`docs/setup-web.md`](../../setup-web.md).
If Linux denies access to the Docker socket after adding your user to the
`docker` group, run `newgrp docker`, then verify with `id -nG` and `docker ps`.
Opening another terminal window alone may not refresh group membership.
After starting Compose, run `zelyra doctor main.zyl --env-file .env --port
18080` to check source, schema, MariaDB connectivity, Docker Compose, and the
published port without changing the database.

For a complete CRUD starter instead of the minimal welcome page:

~~~bash
zelyra new machine-management --template mariadb-crud \
    --web-port 8080 --host-port 18080 --db-host-port 3307
cd machine-management
docker compose --env-file .env -f docker-compose.mariadb.yml up -d --build
set -a; . ./.env; set +a
zelyra db setup main.zyl
~~~

If `docker compose` is unavailable, use the legacy command
`docker-compose --env-file .env -f docker-compose.mariadb.yml up -d --build`.

The starter includes related departments and machines, forms, CRUD pages,
search, filtering, pagination, and custom actions.

### Interface language and learning guide

Generated MariaDB projects use German and show the guided help by default.
Change `.env` to use English or hide the guide:

```dotenv
ZELYRA_LANGUAGE=en
ZELYRA_LEVEL=work
```

`zelyra serve` reads process variables before the project's `.env`. Its direct
fallback is English with `work`; new MariaDB scaffolds explicitly select
German and `learn`. The shipped UI copy is maintained in
`web/locales/en.json` and `web/locales/de.json`; the CRUD sample view and
framework controls use those catalogs. Learning help is explanatory only and
does not grant capabilities or permissions. See [the environment reference](../../env.en.md).

For an authentication starter with persistent sessions and permissions:

~~~bash
zelyra new secure-app --template mariadb-auth \
    --web-port 8080 --host-port 18080 --db-host-port 3307
~~~

It includes users, sessions, permissions, the automatic login/logout flow, and
a protected `/admin` page.

For a complete business starter with authentication, protected CRUD, an audit
log, a schema-mapped form, and a typed API:

~~~bash
zelyra new business-app --template mariadb-business \
    --web-port 8080 --host-port 18080 --db-host-port 3307
~~~

This is the recommended starting point for a database-backed business
application that will be extended with ordinary Zelyra code.

For repository integration tests, an isolated MariaDB instance is available:

~~~bash
export ZELYRA_MARIADB_ROOT_PASSWORD='<test-password>'
export ZELYRA_MARIADB_PASSWORD='<test-password>'
docker compose -f tests/docker-compose.mariadb.yml up -d
DATABASE_URL='mariadb://root:<test-password>@127.0.0.1:3308/zelyra_test' \
    ./tests/mariadb-e2e.sh
~~~

It uses the `zelyra-mariadb-tests` container and a dedicated volume, publishing
MariaDB on host port `3308` without changing any other database installation.

To test the complete generated-project path against a fresh database:

~~~bash
cargo build -p zelyra-cli
export ZELYRA_GENERATED_E2E_ROOT_PASSWORD='<test-password>'
./tests/generated-project-mariadb-e2e.sh
~~~

The test creates a temporary project with `zelyra new`, runs `zelyra setup`,
validates the generated Compose and `doctor` configuration, and removes the
temporary project and database after the CRUD HTTP test. The password is never
printed or stored.

The generated Docker runtime can also be verified:

~~~bash
./tests/generated-project-docker-e2e.sh
~~~

It builds the generated image from the published Zelyra tag, starts MariaDB
and the web server on host ports 3309 and 18082 by default, checks the welcome
page and port mappings, and removes all temporary Docker resources.

Users who do not want to install Rust can download the prebuilt Linux or
Windows archive from the [GitHub Releases page](https://github.com/sf1976/zelyra/releases).
Each archive includes a SHA-256 checksum, the CLI, both README languages, and
the license notices.

Minimal `zelyra.toml`:

~~~toml
[project]
name = "machine-management"
version = "0.1.45"
zelyra = "0.1"

[capabilities]
database = true
network = false
clock = true
environment = true
~~~

Important commands:

| Command | Purpose |
|---|---|
| `zelyra check app.zyl` | lex, parse, resolve, and type-check |
| `zelyra build app.zyl` | check and build an application |
| `zelyra run app.zyl` | run a program |
| `zelyra serve app.zyl` | start the HTTP server |
| `zelyra doctor app.zyl [--json]` | check project, database, and web readiness |
| `zelyra setup --all` | prepare `.env`, start MariaDB, and apply the schema |
| `zelyra setup --web` | open the local browser setup assistant |
| `zelyra verify app.zyl` | classify contracts |
| `zelyra doc app.zyl --openapi` | generate an OpenAPI document |
| `zelyra db create app.zyl` | emit checked CREATE SQL without connecting |
| `zelyra db setup app.zyl` | create MariaDB and its initial schema |
| `zelyra db bootstrap app.zyl` | bootstrap MariaDB or SQLite schema |
| `zelyra db inspect app.zyl` | inspect the live schema |
| `zelyra db plan app.zyl` | display schema changes |
| `zelyra db apply app.zyl` | apply an approved plan |
| `zelyra audit inspect app.zyl` | inspect the latest audit events |
| `zelyra audit export app.zyl --format json` | export audit events as JSON |
| `zelyra audit verify app.zyl` | verify audit fields and optional hash chain |
| `zelyra audit prune app.zyl --before <timestamp> --confirm` | remove old audit events |

## 5. Variables, types, and functions

Values are immutable by default:

~~~zelyra
machine_name = "Press 7"
capacity: Int = 120
active = true
~~~

Mutation is explicit:

~~~zelyra
mutable completed = 0
completed = completed + 1
~~~

The compiler is not paranoid. It has simply seen things.

Core types include:

~~~text
Int UInt Float Decimal Bool String Char Bytes
Timestamp Date Time Duration Email Url Uuid Money
~~~

Functions are typed:

~~~zelyra
fn available_capacity(total: Int, reserved: Int) -> Int {
    return total - reserved
}
~~~

Nominal IDs keep domains separate:

~~~zelyra
type MachineId = Id
type OrderId = Id
~~~

An `OrderId` is not a `MachineId`, even when both look similar underneath.
Business mistakes do not become correct by sharing a storage type.

## 6. Option, Result, and pattern matching

Ordinary types are never null. Absence is explicit:

~~~zelyra
email: Email?
~~~

Handle both cases:

~~~zelyra
match email {
    Some(value) => print(value)
    None => print("No email address")
}
~~~

Pattern matching must be exhaustive, preventing the forgotten case that would
otherwise introduce itself during a Friday deployment.

Functions can expose expected failures:

~~~zelyra
fn load_machine(id: MachineId)
    -> Machine
    throws DatabaseError | NotFound
{
    // implementation
}
~~~

## 7. MariaDB and tables

✅ MariaDB is the default backend and primary runtime reference.

~~~zelyra
database main {
    engine: mariadb
}

table departments {
    id: Id primary auto
    name: String(100) required unique
}

table machines {
    id: Id primary auto
    number: String(30) required unique
    name: String(100) required
    department: Department required
    active: Bool default true
}
~~~

Zelyra recognizes the relationship and can use it for foreign keys, forms, and
select controls.

| Zelyra | MariaDB |
|---|---|
| `Id primary auto` | generated primary ID |
| `String(100)` | `VARCHAR(100)` |
| `String` | `TEXT` |
| `Email` | email-compatible text column |
| `Bool` | Boolean database value |
| `Timestamp` | timestamp value |

## 8. Inspecting and applying schemas

Supply the connection through the environment:

~~~bash
export DATABASE_URL='mariadb://user:password@127.0.0.1:3306/zelyra_demo'
~~~

Bootstrap, inspect, and plan:

~~~bash
zelyra db bootstrap examples/machine_management_mariadb.zyl
zelyra db inspect examples/machine_management_mariadb.zyl
zelyra db plan examples/machine_management_mariadb.zyl
~~~

Apply an approved plan:

~~~bash
zelyra db apply examples/machine_management_mariadb.zyl
~~~

Destructive changes require explicit permission:

~~~bash
zelyra db apply examples/machine_management_mariadb.zyl --allow-destructive
~~~

The database command contract is explicit: `create` emits checked DDL without
connecting; `setup` creates a MariaDB database when needed and applies the
initial schema; `bootstrap` applies an initial MariaDB or SQLite schema;
`inspect` reads the live schema; `plan` displays the deterministic diff; and
`apply` executes it after refusing destructive changes unless
`--allow-destructive` is supplied. The live commands require `DATABASE_URL`.

That flag does not mean “probably fine.” It means “I read the plan, have a
backup, and my pulse is normal.”

## 9. Native SQL

✅ SQL is a language element:

~~~zelyra
fn load_active_machines() -> Machine[]
    uses Database
{
    return sql<Machine[]> {
        SELECT id, number, name, department_id, active
        FROM machines
        WHERE active = true
        ORDER BY number
    }
}
~~~

Named parameters are bound safely:

~~~zelyra
fn load_machine(id: MachineId) -> Machine?
    uses Database
{
    return sql<Machine?> {
        SELECT id, number, name, department_id, active
        FROM machines
        WHERE id = :id
    }
}
~~~

Where schema information is available, Zelyra checks tables, columns, aliases,
parameters, nullability, result mappings, and the `Database` capability.

Writes can be grouped in transactions:

~~~zelyra
transaction {
    sql {
        UPDATE machines
        SET active = false
        WHERE id = :id
    }
}
~~~

## 10. Web pages

✅ An initial safe web layer is implemented.

~~~zelyra
page "/machines/{name}" {
    html {
        <html>
            <body>
                <h1>Machine {name}</h1>
                <p>It is running. Hopefully not away.</p>
            </body>
        </html>
    }
}
~~~

Start the server:

~~~bash
zelyra serve app.zyl
~~~

Open `http://127.0.0.1:3000/machines/Press-7`.

Path values are HTML-escaped by default. The current Web Core includes GET
routes, path parameters, query-string handling, HTTP parsing, and HTML
responses. A full component and client-state system remains on the roadmap.

### Reusable views

Named views provide a safe layout boundary for page-specific design:

~~~zelyra
view SiteShell {
    html {
        <html><body><header>Zelyra</header><main><slot /></main></body></html>
    }
}

page "/customers" {
    view: SiteShell
    html { <h1>Customers</h1> }
}
~~~

The compiler requires exactly one default `<slot />` in a named view. Layouts
may also declare named slots with fallback content:

~~~zelyra
view AppShell {
    html {
        <header><slot name="header"><h1>Zelyra</h1></slot></header>
        <main><slot /></main>
    }
}

page "/dashboard" {
    view: AppShell
    html {
        <slot name="header"><h1>Dashboard</h1></slot>
        <p>Page content</p>
    }
}
~~~

A page may provide only slots declared by its selected view; duplicate or
unknown slots are rejected during checking. Unprovided named slots use their
fallback. Page content is composed before routing, while existing
authentication, authorization, and escaping remain active. Typed self-closing
components with declared properties are available as well:

~~~zelyra
component Badge {
    props { text: String }
    html { <span class="badge">{text}</span> }
}

page "/status" {
    html { <Badge text="Ready" /> }
}
~~~

The same shell can wrap generated CRUD pages. Use `layout: ViewName` on the
CRUD resource:

~~~zelyra
view AppShell {
    html { <html><body><main><slot /></main></body></html> }
}

crud Customer -> customers {
    layout: AppShell
}
~~~

The default slot receives the generated list, detail, create/edit, and custom
action form content. The selected view must exist and is checked at compile
time. SQL, validation, CSRF, authorization, and escaping remain generated and
enforced; redirects are not wrapped as HTML. See
`examples/view_showcase.zyl`.

Each CRUD resource can fill named slots in its outer layout. The default slot
remains reserved for safely generated CRUD content:

~~~zelyra
view BusinessShell {
    html {
        <html><body>
            <header><slot name="resource_heading"><h1>Business data</h1></slot></header>
            <main><slot /></main>
            <aside><slot name="resource_help"><p>Resource help</p></slot></aside>
        </body></html>
    }
}

crud Machine -> machines {
    layout: BusinessShell
    slots {
        resource_heading { html { <h1>Machines</h1> } }
        resource_help { html { <p>Use search and filters.</p> } }
    }
}
~~~

The compiler checks that the layout and named slots exist and are not supplied
more than once. Unfilled slots retain their fallback. Static slot content may
use checked components, but cannot access records, request values, or CRUD
actions. SQL, validation, CSRF, permissions, and escaping remain owned by the
generated CRUD. Invalid CRUD slot configuration reports E-VIEW-031.

View interpolations are checked before the server starts. A page may use its
route parameters, a component may use its declared properties, and a dynamic
component property must have a compatible type. A page may also load one
typed record explicitly and use checked field access:

~~~zelyra
page "/customers/{name}" {
    load customer = sql<Customer> {
        SELECT id, name FROM customers WHERE name = :name
    }
    html { <h1>{customer.name}</h1> }
}
~~~

The SQL is checked against the schema, route parameters are safely bound, and
authentication, permissions, and the `Database` capability are enforced
before the query runs. Loaded values are HTML-escaped. Collections can be
rendered with a typed server-side loop:

~~~zelyra
page "/customers" {
    load customers = sql<Customer[]> { SELECT id, name FROM customers }
    html { <ul>for customer in customers { <li>{customer.name}</li> }</ul> }
}
~~~

Option-aware field expressions and richer view data remain planned. See
`examples/view_data.zyl` and `examples/view_collection.zyl`.

Pages can declare typed query inputs for explicit server-side SQL:

~~~zelyra
page "/customers" {
    input { search: String? }

    load customers = sql<Customer[]> {
        SELECT id, name FROM customers
        WHERE (:search IS NULL OR name LIKE CONCAT('%', :search, '%'))
        ORDER BY name
    }

    html { <p>Search: {search}</p> }
}
~~~

Query values are checked against their Zelyra type and safely bound as
database parameters. A missing optional input becomes SQL `NULL`; a missing
required input or invalid value produces a controlled HTTP 400 response.
For page collections that declare search, filters, sorting, or pagination,
Zelyra automatically renders a semantic query-control form and preserves URL
state. Page collections can already opt into safe server-side sorting and
pagination:

~~~zelyra
page "/customers" {
    search { name }
    sort { name }
    paginated 25
    load customers = sql<Customer[]> { SELECT id, name FROM customers }
    html { <p>Page: {page}, sort: {sort}, order: {order}</p> }
}
~~~

`page` is validated as a positive integer, defaults to `1`, and is applied as
a parameterized `LIMIT`/`OFFSET` wrapper around the collection SQL. `sort`
accepts only declared result fields and `order` only `asc` or `desc`; both are
safe to use in URLs such as `/customers?sort=name&order=desc`. See
`examples/view_query_input.zyl`.

`search { name email }` provides the same compiler-checked whitelist for the
`search` URL value. Search terms are bound as parameters and applied with
server-side `LIKE` conditions; unchecked SQL fragments are never created.

Paginated page collections also expose `total` and `pages` as `UInt` bindings
after a safe count query. Pages using only explicit `input` declarations remain
manual and do not receive generated controls.

Page collections can also declare typed filters:

~~~zelyra
page "/customers" {
    filter { name quantity }
    load customers = sql<Customer[]> { SELECT id, name, quantity FROM customers }
    html { <p>{filter_name}</p> }
}
~~~

The compiler checks filter fields against the collection result type. Text
fields support `eq`, `contains`, `starts_with`, `ends_with`, and null checks;
numeric fields also support `gt`, `gte`, `lt`, and `lte`; booleans and other
values support equality and null checks. Values remain parameterized and
fields/operators are whitelisted. Examples include
`/customers?filter_name__contains=Acme` and
`/customers?filter_quantity__gte=10`. Unsupported operators and unknown fields
return a controlled HTTP 400 response.

Components may accept child HTML through a default slot or named slots:

~~~zelyra
component Panel {
    html { <section class="panel"><slot /></section> }
}

page "/dashboard" {
    html { <Panel><h1>Dashboard</h1></Panel> }
}
~~~

Named slots are declared and provided explicitly:

~~~zelyra
component Layout {
    html { <header><slot name="header" /></header><main><slot /></main> }
}

page "/dashboard" {
    html {
        <Layout>
            <slot name="header"><h1>Dashboard</h1></slot>
            <p>Content</p>
        </Layout>
    }
}
~~~

Nested components are expanded from the inside out. Passing child content to a
component without a matching default or named `<slot />` is a compile-time
error. Named slots may provide escaped, deterministic fallback content; a
caller can override that slot explicitly.

CRUD list views can be adjusted without replacing the generated query or
authorization pipeline. The default is an HTML table; a card layout and a
custom empty-state message are available:

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

`mode` accepts `table` or `cards`. Search, typed filters, allowlisted sorting,
pagination, URL state, HTML escaping, and permission checks remain generated
in both modes. Detail, form, loading, and error view overrides are also
available.

For the common case, one shared field profile can drive the generated list,
detail, and create/edit forms:

~~~zelyra
crud Customer -> customers {
    view {
        fields { name email active }
    }
}
~~~

An explicit `list { ... }` remains an override for the list/detail selection.
Primary-key and auto-generated fields stay out of forms automatically, and
unknown profile fields are rejected by the compiler.

Detail views support the same controlled presentation choice and an explicit
heading:

~~~zelyra
view {
    detail {
        mode: cards
        title: "Customer details"
    }
}
~~~

The default is `standard`. Generated edit, create, delete, CSRF, escaping, and
authorization safeguards remain active.

CRUD forms can define a controlled layout, heading, and submit label:

~~~zelyra
view {
    form {
        mode: cards
        title: "Customer form"
        submit: "Save customer"
    }
}
~~~

The title and submit label are escaped. Schema validation, readonly checks,
CSRF protection, parameter binding, and action permissions remain active.

The delete confirmation can define its own heading, warning, and submit label:

~~~zelyra
view {
    delete {
        title: "Delete customer"
        message: "This cannot be undone."
        submit: "Delete now"
    }
}
~~~

The generated endpoint remains POST-only and continues to enforce CSRF and
delete authorization checks.

CRUD loading and error states can also be configured:

~~~zelyra
view {
    loading { message: "Loading customers..." }
    error {
        title: "Customer unavailable"
        message: "Please try again later."
    }
}
~~~

The loading message is emitted as escaped metadata for progressive enhancement;
the server-rendered response does not claim that loading is active. Configured
error messages replace generic CRUD database-error pages without exposing
internal database details.

Custom CRUD actions add focused business operations while retaining the same
authorization and parameter-binding pipeline:

~~~zelyra
crud Customer -> customers {
    action deactivate {
        label: "Deactivate customer"
        confirm: "Deactivate this customer?"
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

This generates a POST-only `/customers/{id}/deactivate` route and a detail
view button. CSRF, the database capability, authentication, and declared
permissions are checked. The route ID is bound to `:id`, so the SQL remains
parameterized. Action names are currently used as button labels.
`label` overrides the escaped button text, and `confirm` adds an escaped
browser confirmation before submission. Without `label`, the action name is
used as the label.

Actions may also declare typed fields. They use the regular form validation
and are bound as SQL parameters:

~~~zelyra
action set_active {
    icon: "check"
    field active: Bool { required }
    sql {
        UPDATE customers SET active = :active WHERE id = :id
    }
}
~~~

`icon` adds an escaped `data-icon` hook to the generated action button.
`success` is carried to the redirect and rendered as an escaped status notice
on CRUD lists.

Use `confirm_page { title: "..." message: "..." submit: "..." }` for a
server-rendered confirmation step. The detail action becomes a GET link, and
the confirmation page renders the fields with a fresh CSRF-protected POST
form. Authorization is checked for both requests.

`success_page` configures a structured escaped success notice, while
`error_page` provides a safe action-specific failure page without exposing
database details.

CRUD resources can opt into reversible soft deletion with a nullable timestamp:

~~~zelyra
table customers {
    id: Id primary auto
    name: String(100) required
    deleted_at: Timestamp?
}

crud Customer -> customers {
    soft_delete { column: deleted_at }
}
~~~

Delete sets the timestamp instead of removing the row. Normal lists exclude
archived records; `/customers?archived=true` shows them, and the archived
detail view provides a CSRF-protected restore action. The marker is omitted
from generated forms and default list/filter columns. Permanent purge and
retention policies are still planned.

If the project also declares an authentication audit table, generated CRUD
mutations append audit events in the same MariaDB transaction. Create, update,
delete, archive, restore, and custom actions record the actor, operation, table,
target record, and field-level changes. Sensitive values such as passwords,
tokens, secrets, and hashes are redacted.

Relationship fields such as `department: Department` are rendered as checked
select fields. Zelyra loads their labels from the referenced MariaDB table,
submits the stored ID, and rejects stale or unknown IDs before executing the
action SQL.

## 11. Forms

✅ Forms can inherit table rules:

~~~zelyra
form MachineCreate -> machines {
    fields {
        number
        name
        department
        active
    }
}
~~~

An explicit definition can add presentation and validation rules:

~~~zelyra
form ContactForm {
    field email: Email {
        label: "Email"
        required
        max: 255
        widget: email
    }
}
~~~

Validate without a server:

~~~bash
zelyra form validate examples/customer_form.zyl CustomerCreate \
    name="Example Ltd" email=info@example.test
~~~

With `zelyra serve`, Zelyra exposes a form at `/forms/FormName`. GET renders a
CSRF token; POST checks the token and validates the submitted values.

Database-backed action:

~~~zelyra
form CustomerCreate -> customers {
    fields { name email }

    action save {
        requires auth
        permits "customers.save"
        sql {
            INSERT INTO customers (name, email)
            VALUES (:name, :email)
        }

        redirect "/customers"
    }
}
~~~

Form actions may declare their own authorization. The permission is checked
when the form is rendered and again before submission.

## 12. CRUD

✅ The shortest case is satisfyingly short:

~~~zelyra
crud Machine -> machines
~~~

Configured resource:

~~~zelyra
crud Machine -> machines {
    title: "Machines"
    list { number name department active }
    search { number name }
    filter { department active }
}
~~~

Zelyra provides list and detail pages, Create/Edit forms, search, filters,
allowlisted sorting, pagination, and CSRF-protected deletion. Configured fields
are checked against the schema.

Generated CRUD pages use the responsive Zelyra application shell by default.
Standalone generated forms and tableviews use it too. An explicit
`layout: ViewName` on a CRUD replaces the default shell; custom authored pages
are not silently wrapped. The generated navigation follows declared resources,
and framework copy follows `ZELYRA_LANGUAGE`. Keyboard users get visible focus
styles and a localized skip-to-content link.

The shared palette is customizable in the generated project's
`zelyra.theme.css`. For example, this changes the accent and card shape without
replacing the generated CRUD views:

~~~css
:root {
    --zelyra-color-accent: #087f8c;
    --zelyra-color-accent-strong: #066b76;
    --zelyra-radius-card: 22px;
}
~~~

`zelyra serve` links the optional file after the built-in design system. New
projects include a commented starter, and Docker scaffolds copy it into the
image. CSS is public browser content and is not type-checked; see the
[theme and environment reference](../../env.en.md) for the available tokens
and safety limits.

Example URLs:

~~~text
/machines
/machines?search=Press
/machines?filter_active=true
/machines?filter_number__contains=CNC
/machines?filter_quantity__gte=10
/machines?sort=number&order=asc
/machines/new
/machines/42/edit
~~~

Filter operators are selected from controls generated from the schema. Text
columns support `eq`, `contains`, `starts_with`, and `ends_with`; numeric
columns support `eq`, `gt`, `gte`, `lt`, and `lte`. All supported columns also
provide `is_null` and `is_not_null`. The explicit URL form
`filter_<column>__<operator>=<value>` is useful for links and saved searches.
Values remain bound parameters and column names are validated against the
schema.

🗺️ Fully custom typed components and fine-grained view overrides are part of
the continuing view roadmap.

### Standalone table views

`tableview` exposes a checked MariaDB query without requiring a CRUD resource.
The result can be a table type or a dedicated struct for joins and aggregates:

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

The generated route is `/views/customers`. The SQL result type, projection
aliases, and declared columns are checked against the schema and the target
struct. Search terms are applied to the
declared result columns, sorting is restricted to the declared allowlist, and
pagination values are bound parameters. See `examples/tableview.zyl`.

Optional `filter` fields use the same safe URL contract as CRUD filters:
`filter_<column>=<value>` defaults to equality, while
`filter_<column>__<operator>=<value>` selects an operator such as `contains`,
`gte`, or `is_null`. The compiler derives the available operators from the
typed result field; unknown fields, unsupported operators, and invalid numeric
or boolean values are rejected with HTTP 400.

Generated CRUD and tableview controls use semantic fieldsets and separate
labels for the operator and value of each filter. Filter processing and
preserved pagination URLs use deterministic ordering.

## 13. Authentication and permissions

🧪 Zelyra supports Argon2 login, persistent MariaDB sessions, logout, route
guards, and database-backed permission checks. Five failed attempts for the
same normalized e-mail address within 15 minutes trigger a 60-second HTTP 429
lockout. A successful login rotates and invalidates the previous browser
session token.

Create a value for the required `password_hash` column with the CLI. The
interactive command disables password echo and asks for confirmation:

~~~bash
zelyra auth hash-password
~~~

For deliberate automation, pass one password line through `--stdin`. Do not
put real passwords in command arguments or commit generated hashes to source
control:

~~~bash
printf '%s\n' 'change-this-password' | zelyra auth hash-password --stdin
~~~

~~~zelyra
auth users {
    table: users
    permissions: user_permissions
    roles: user_roles
    role_permissions: role_permissions
    audit: auth_audit_log
    admin_path: "/admin/access"
    admin_permission: "auth.manage"
    admin_role: admin
}

page "/admin" {
    requires auth
    permits "machines.manage"

    html {
        <h1>Machine management</h1>
    }
}
~~~

The optional `permissions` table contains `user_id` and `permission` for
direct grants. Role groups are enabled with `roles` and `role_permissions`:
the first table contains `user_id` and `role`, and the second contains `role`
and `permission`. Effective permissions are the union of direct grants and
all permissions inherited from the user's roles.

When all three `admin_*` options are present, Zelyra also provides an opt-in
browser administration screen at the configured path. It is protected by the
declared permission and CSRF tokens. Administrators can create users, reset
passwords, activate or deactivate users, and manage roles and role
permissions. With an `active` column, deactivated users cannot log in and
their persistent sessions are removed on deactivation. Password resets also
remove all persistent sessions belonging to that user. The last assignment
The last assignment of the configured administrator role and the last active
administrator are protected. User deletion and self-service account
management remain future work.

The optional `auth_audit_log` table records login, logout, password, user,
role, and permission events with actor, event, target, details, and timestamp.
CLI role changes use a nullable actor and mark `details` with `source=cli`.
The administration screen displays the latest 100 entries. Use
`zelyra audit inspect app.zyl` for a human-readable view or
`zelyra audit export app.zyl --format json|csv` for a bounded export; the
default limit is 100 and the maximum is 10,000. `zelyra audit verify` checks
that every row contains an event, details, and timestamp. `zelyra audit prune`
requires `--before <timestamp>` and never deletes anything without the explicit
`--confirm` flag; the prune operation is recorded as an audit event.

For tamper-evident history, add `audit_chain: true` to the `auth` definition.
The audit table must then also contain an `id` column plus `previous_hash` and `entry_hash`,
normally `String(64)`. Zelyra calculates lowercase SHA-256 hashes from the
canonical pipe-separated payload
`previous_hash|actor_user_id|event|target_user_id|details|created_at`; a
missing ID is written as `NULL` and the timestamp uses
`YYYY-MM-DD HH:MM:SS`. Appending locks the latest row and shares the business
transaction. `audit verify` checks links and hashes, while pruning is refused
for chained logs because it would break the chain.

Use the CLI to maintain assignments without writing SQL. The project is
validated before the MariaDB write, and repeated grants are safe:

~~~bash
DATABASE_URL='mariadb://user:password@127.0.0.1:3306/app' \
  zelyra auth role grant app.zyl 42 manager
DATABASE_URL='mariadb://user:password@127.0.0.1:3306/app' \
  zelyra auth role-permission revoke app.zyl manager customers.edit
~~~

The same guards protect typed API handlers:

~~~zelyra
api GET "/api/machines/{id}" {
    handler get_machine
    requires auth
    permits "machines.view"
    input { id: MachineId }
    output Machine
    errors { 404 NotFound }
}
~~~

Protected API failures use JSON with an error `code` and `message`. A handler
can return `Err("NotFound")` to select a matching status from the declared
`errors` block; undeclared errors become 500 responses. Richer domain-error
values remain future work. JSON arrays can be bound to typed fields such as
`Int[]` or `MachineId[]`. Nested JSON input uses declared records:

~~~zelyra
struct Address { city: String }
struct CustomerInput { name: String address: Address }

api POST "/customers" {
    handler echo_customer
    input { customer: CustomerInput }
    output CustomerInput
}
~~~

Unknown record fields and missing required fields are rejected. In the language
core, arrays support literals, indexing, `len`, `append`, `contains`, `first`,
`last`, and concatenation with `+`. Record literals and checked field access
are available for nested values:

~~~zelyra
customer = CustomerInput {
    name: "Anna"
    address: Address { city: "Berlin" }
}

print(customer.address.city)
~~~

Array iteration uses `for ... in`; the loop variable is immutable and scoped to
the loop body. `break` and `continue` are supported.

Typed maps are available for scalar keys. Use `Map<Key, Value>` for the type
and `Map { ... }` for a literal:

~~~zelyra
prices: Map<String, Int> = Map { "standard": 10 "premium": 20 }
current = put(prices, "standard", 12)

match get(current, "standard") {
    Some(price) => {
        print(price)
    }
    None => {
        print(0)
    }
}
~~~

`get` returns an `Option`, `put` returns a new map, and `keys`, `values`,
`contains`, and `len` preserve deterministic behavior. JSON conversion uses
JSON objects for `Map<String, Value>`; non-string map keys are not JSON maps.

The same string-keyed map type can be exposed directly through a typed API:

~~~zelyra
fn echo_settings(settings: Map<String, Int>) -> Map<String, Int> {
    return settings
}

api POST "/settings" {
    handler echo_settings
    input { settings: Map<String, Int> }
    output Map<String, Int>
}
~~~

The compiler validates the JSON boundary, OpenAPI describes the map as an
object with integer values, and the generated TypeScript client uses
`Record<string, number>`. A non-string map key is rejected for API input or
output before the server can start.

Generate a browser or Node-compatible TypeScript client from the same API
declarations:

~~~bash
zelyra doc examples/api_records.zyl --typescript > customer-client.ts
~~~

The generated client uses standard `fetch`, includes declared records and
tables as TypeScript types, and handles path/query parameters, JSON bodies,
bearer tokens, response types, and HTTP errors. Declared API error names are
available through `ZelyraApiErrorCode`; `ZelyraApiError.fromResponse` extracts
the status, code, and server message while preserving the raw response body.

API errors may also carry a checked payload. Add the payload type after a
colon and use the same type as the error side of the handler's `Result`:

~~~zelyra
struct ValidationProblem {
    field: String
    message: String
}

api POST "/customers/validate" {
    handler validate_customer
    output Result<String, ValidationProblem>
    errors { 422 ValidationError: ValidationProblem }
}
~~~

The response retains `error.code` and `error.message` and adds the serialized
payload as `error.details`. OpenAPI includes the details schema, while the
generated client exposes it through `ZelyraApiErrorPayloads` and the generic
`ZelyraApiError.details` field. Existing untyped API errors remain compatible.

Browser access is disabled by default. Enable exact origins in the project
configuration when a separate frontend needs to call an API:

~~~toml
[web]
allowed_origins = ["http://localhost:5173"]
allow_credentials = false
~~~

Zelyra answers API `OPTIONS` preflight requests automatically and adds CORS
headers only to declared API routes. Wildcard origins are rejected, and CORS
does not bypass authentication or permissions. Enable credentials only when
browser session cookies are required; the client must also use
`credentials: "include"`.

A hidden button is not a security boundary. Authorization must be enforced on
the server-side action. Browsers become remarkably creative when trusted.

## 14. Capabilities

✅ External powers are declared explicitly:

~~~zelyra
fn load_machines() -> Machine[]
    uses Database
{
    return sql<Machine[]> {
        SELECT id, number, name FROM machines
    }
}
~~~

Known capabilities:

~~~text
Database Network FileSystem Environment Process Clock Random
~~~

Calling functions must propagate required capabilities. Projects grant them
through `zelyra.toml`:

~~~toml
[capabilities]
database = true
network = false
~~~

Static checking and runtime enforcement at function, native-SQL, forms, CRUD,
and authentication database boundaries are implemented when project grants are
supplied. A complete operating-system sandbox for every capability is not yet
available.

Two safe host APIs are implemented:

~~~zelyra
fn runtime_timestamp() -> Timestamp uses Clock {
    return now()
}

fn configured_mode() -> String? uses Environment {
    return env("ZELYRA_MODE")
}
~~~

now() requires Clock and returns Unix-epoch milliseconds. env(name) requires
Environment and returns String?; a missing variable becomes None. Values are
not logged or exposed automatically. Network, file-system, process, and random
host APIs remain planned.

The Random capability provides secure integer generation:

~~~zelyra
fn dice_roll() -> Int uses Random {
    return random_int(1, 6)
}
~~~

The range is inclusive on both sides. Invalid ranges fail at runtime, and
random values are not emitted implicitly. Process execution is available only
through the explicitly bounded API below.

The first Network host API is `http_get`:

~~~zelyra
fn load_status(url: String) -> String uses Network {
    return http_get(url)
}
~~~

Project execution uses an exact host allowlist and bounded resources:

~~~toml
[network]
allowed_hosts = ["127.0.0.1:8080", "api.example.com"]
timeout_ms = 5000
max_response_bytes = 1048576
~~~

Without `[network]`, a project allows no hosts. The transport supports
`http://` and `https://` with Rustls certificate verification enabled by
default. It does not follow redirects and returns only successful UTF-8 GET
response bodies within the configured limits. The `http_get` helper remains
the simple GET-only convenience API; use `http_request` when request headers,
request bodies, or response status are needed.

Typed requests use `http_request`:

~~~zelyra
fn create_customer(url: String) -> HttpResponse uses Network {
    return http_request(
        "POST",
        url,
        ["Content-Type: application/json"],
        Some("{\"name\":\"Anna\"}")
    )
}
~~~

The method accepts `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, and `HEAD`.
Headers are `Name: value` strings, the body is `String?`, and the typed result
contains `status: Int`, `headers: String[]`, and `body: String`. GET/HEAD
requests cannot carry a body.

JSON values can be converted to and from checked Zelyra values. Records and
their nested fields are validated against the declared schema:

~~~zelyra
struct Customer { name: String tags: String[] nickname: String? }

fn decode_customer(body: String) -> Customer {
    return json_decode<Customer>(body)
}

fn encode_customer(customer: Customer) -> String {
    return json_encode(customer)
}
~~~

`json_decode<Type>(text)` requires one target type argument and supports
records, nested records, arrays, options, and scalar values. `json_encode`
serializes the same value families. Malformed JSON, type mismatches, unknown
record fields, and missing required fields are explicit runtime errors.

For a complete typed JSON request/response flow, use `http_json` with separate
request and response type arguments:

~~~zelyra
struct CustomerCreate { name: String }
struct Customer { id: Int name: String }

fn create_customer(url: String, payload: CustomerCreate) -> Customer uses Network {
    return http_json<CustomerCreate, Customer>("POST", url, [], Some(payload))
}
~~~

The request record is serialized automatically and the response body is
decoded into the response record. If no `Content-Type` header is supplied,
`application/json` is added. Non-2xx responses are explicit runtime errors;
the helper returns the decoded value rather than response headers.

When response metadata must remain available, use `http_result`:

~~~zelyra
fn submit(url: String, payload: CustomerCreate) -> HttpResult<Customer> uses Network {
    return http_result<CustomerCreate, Customer>("POST", url, [], Some(payload))
}
~~~

`HttpResult<Response>` contains `status: Int`, `headers: String[]`,
`body: String`, `data: Response?`, and `error: HttpError?`. Successful 2xx
responses populate `data`; non-2xx responses populate `error` with status,
headers, body, and message. Transport failures and invalid success JSON remain
runtime errors.

The Process capability exposes a shell-free command API:

~~~zelyra
fn render_report(input: String) -> String uses Process {
    return run_process("/usr/bin/printf", ["%s", input])
}
~~~

Project execution requires an exact command allowlist:

~~~toml
[process]
allowed_commands = ["/usr/bin/printf"]
timeout_ms = 5000
max_output_bytes = 1048576
~~~

Without `[process]`, no command may run. The child environment is cleared,
stdin is closed, timeouts terminate the process, and stdout/stderr are
bounded. Shell execution, environment forwarding, working directories, and
pipelines are future work.

The FileSystem host APIs are:

~~~zelyra
fn read_source(path: String) -> String uses FileSystem {
    return read_text(path)
}

fn write_note(path: String, content: String) uses FileSystem {
    write_text(path, content)
}

fn entries(path: String) -> String[] uses FileSystem {
    return list_dir(path)
}

fn remove_note(path: String) uses FileSystem {
    delete_file(path)
}
~~~

All four APIs require FileSystem. Reads and directory listings use read_roots;
writes and deletes use write_roots. Relative paths are resolved from the
project directory, and existing symlink targets are canonicalized before
access. Without a filesystem section, project reads are limited to the
project directory while writes and deletes are denied:

~~~toml
[filesystem]
read_roots = ["."]
write_roots = ["data"]
~~~

The configured directories must already exist. A new write target must have an
existing parent directory.

## 15. Contracts and verification

🧪 Preconditions and postconditions:

~~~zelyra
fn reserve(stock: Int, amount: Int) -> Int
    requires {
        amount > 0
        stock >= amount
    }
    ensures {
        result >= 0
        result == stock - amount
    }
{
    return stock - amount
}
~~~

`requires` runs before the body; `ensures` runs afterward, with `result`
representing the returned value.

~~~bash
zelyra verify examples/contracts.zyl
~~~

Statuses:

~~~text
PROVEN
RUNTIME_CHECK
UNPROVEN
FAILED
~~~

Only `PROVEN` means proven. `RUNTIME_CHECK` does not wear a fake moustache and
claim to be mathematics.

The verifier also summarizes bounded function calls. A callee with multiple
return paths, such as an absolute-value function, contributes its path
conditions when a caller returns that call. Callee `requires` clauses are
checked after argument substitution; caller `requires` clauses are assumptions
when proving caller `ensures`. Complex, recursive, or unresolved cases remain
`RUNTIME_CHECK`.

Local state is included in these summaries. Both `next: Int = value + 1` and
the concise `next = value + 1`, followed by `return next`, are analyzed like a
direct return. Simple linear mutable assignments such as `next = next + 1` are
also tracked. Statically bounded loops with a linear counter are unfolded;
`break` exits the current loop and `continue` starts its next iteration as
separate symbolic paths. Nonlinear assignments and unbounded loops without a
proven invariant remain conservative.

### Loop invariants

A `while` or unconditional `loop` may declare one or more explicit invariants:

~~~zelyra
while current > 0
    invariant { current >= 0 }
{
    current = current - 1
}
~~~

The verifier checks the invariant at entry and after supported body paths. A
proven invariant can summarize an otherwise unbounded linear `while` loop; an
unconditional `loop` can use it with a modeled `break` exit. Runtime execution
checks it before and after each iteration. Unsupported or unproven invariants
remain conservative and do not produce `PROVEN` results.

`zelyra verify` reports each declared invariant separately, after a function's
`ensures` results. Invariant indexes are zero-based:

~~~text
PROVEN [V-001]: reduce.ensures[0] (src/reduce.zyl:3:5-3:21)
PROVEN [V-001]: reduce.invariant[0] (src/reduce.zyl:7:21-7:33)
FAILED [V-004]: reduce.invariant[1] (src/reduce.zyl:8:21-8:34)
~~~

Every result includes a stable code and a source range as
`(file.zyl:start-line:start-column-end-line:end-column)`. The codes are
`V-001` (`PROVEN`), `V-002` (`RUNTIME_CHECK`), `V-003` (`UNPROVEN`), and
`V-004` (`FAILED`). For IDEs and CI, use `zelyra verify app.zyl --json`; the
JSON output contains the same result data, a human-readable `message`, an
optional `counterexample` object, and a structured `location` object. A
counterexample is emitted only when a bounded search verifies a small linear
integer witness. The current search covers up to three linear variables in the
range `-32..=32`, including failed loop-invariant checks; otherwise it is
`null`. Text output also shows an explanation and a source-line excerpt with a
caret marker for every result.

`FAILED` means that the invariant is false on a feasible analyzed path or is
not preserved by the loop body. `RUNTIME_CHECK` means that runtime checking
is required because the symbolic verifier cannot complete the proof. Only
`PROVEN` is a mathematical proof.

## 16. Configuration and secrets

Project configuration belongs in `zelyra.toml`; secrets do not:

~~~toml
[project]
name = "machine-management"
version = "0.1.45"
zelyra = "0.1"

[capabilities]
database = true
network = false
~~~

Connections currently use protected environment variables:

~~~bash
export DATABASE_URL='mariadb://user:password@127.0.0.1:3306/zelyra_demo'
~~~

Rules worth keeping:

- never commit `.env`;
- never put production credentials in examples;
- do not log secrets;
- separate development, test, and production databases;
- never run destructive tests against production.

### Simple defaults, optional power

The beginner path does not require a feature configuration. Advanced project
surfaces can be selected in `zelyra.toml`, while environment-specific,
non-secret overrides can be placed in `.env` or the process environment:

~~~toml
[features]
api = false
crud = false
~~~

Supported switches are `web`, `api`, `crud`, `auth`, and `audit`. Precedence is
the process environment, `.env`, `zelyra.toml`, and then the safe defaults.
Inspect the effective values without displaying secrets:

~~~bash
zelyra config main.zyl --format=json
~~~

When a source file uses a disabled surface, the compiler reports a stable
feature diagnostic. Capabilities, type checking, SQL checks, and security
rules cannot be disabled here. This is an optional convenience layer, not a
new requirement for simple projects. The complete [environment and
configuration reference](../env.en.md) lists every supported setting and must
be updated before a new setting is committed.

🗺️ Typed connections, encrypted secret stores, an SMTP assistant, and ODBC
discovery are planned.

## 17. Diagnostics and troubleshooting

Zelyra aims to explain errors without requiring an archaeological excavation
through a stack trace.

~~~bash
zelyra check app.zyl
~~~

Common error classes include unknown names and types, mutation of immutable
values, incomplete pattern matches, unknown tables or columns, missing SQL
parameters, incompatible result mappings, missing capabilities, invalid form
fields, and failed contracts.

Language-only checks work without `DATABASE_URL`. Database operations report a
missing connection in a controlled way.

## 18. Testing and contributing

Before every commit:

~~~bash
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
~~~

A language feature is complete only when it has documented syntax, AST/HIR
support, static checks, useful diagnostics, positive and negative tests, a
runnable example, and synchronized German and English documentation.

Tests that are green only because they never ran are decoration.

## 19. What comes next

Major planned areas include:

- fully customizable typed view components;
- email templates and SMTP;
- an in-application notification center;
- background jobs and a transactional outbox;
- activity, audit, and technical logs;
- typed connections and secret providers;
- ODBC and external read-only databases;
- richer domain-error values beyond typed API payloads and advanced runtime
  request/response processing;
- cancellation and database-pool integration for structured concurrency;
- broader formal verification;
- optimization models for real planning problems.

Zelyra keeps the common case short without locking the door when the special
case arrives:

> **Generated when possible. Custom where needed. Verified everywhere.**

Now build a table, read the SQL, and check the backup. In that order.

## 20. AI-native development with Zelyra

Zelyra is AI-native but AI-independent. A human or an AI system may author
the same readable `.zyl` source. The compiler remains the authority:

> **AI writes. Zelyra verifies.**

An AI provider is never part of compilation, and source code is not sent to an
external service by the compiler. The machine interfaces are versioned and
vendor-neutral so local tools can use them as well.

### What is available now

- ✅ **Implemented:** deterministic, versioned JSON diagnostics;
- ✅ **Implemented:** stable diagnostic codes and source spans;
- ✅ **Implemented:** read-only structured project context;
- ✅ **Implemented:** deterministic `zelyra fmt` formatting with `--check` for
  CI, preserving comments and opaque SQL/HTML bodies;
- ✅ **Implemented:** human-readable output remains the default;
- 🧪 **Experimental:** the current JSON interface is schema version `1` and
  covers the `check`, `context`, source-only `impact`, and preview-only `edit`
  commands;
- ✅ **Implemented:** expression typed holes written as `_` with contextual
  diagnostics; buildable commands reject incomplete code;
- 🗺️ **Planned:** richer typed gaps, complete runtime/schema impact analysis,
  and the reproducible AI authoring benchmark;
- ❌ **Not available:** automatic production changes, automatic permission
  escalation, or compiler decisions delegated to an AI service.

Check a program in the human-oriented default format:

~~~bash
zelyra check examples/fibonacci.zyl
~~~

Format source canonically, or check formatting without writing:

~~~bash
zelyra fmt examples/fibonacci.zyl
zelyra fmt examples/fibonacci.zyl --check
~~~

For tools, request JSON explicitly:

~~~bash
zelyra check examples/fibonacci.zyl --format=json
~~~

The current successful response has this shape:

~~~json
{
  "schema_version": "1",
  "command": "check",
  "success": true,
  "diagnostics": []
}
~~~

Errors use stable codes such as `E-LEX-001`, `E-PARSE-001`, and
`E-SQL-004`. JSON is written only to `stdout`; technical logs belong on
`stderr`. A failed check returns a non-zero exit code. Offsets are UTF-8 byte
offsets, lines and columns are one-based, and repeated checks produce the same
bytes.

Inspect the understood, read-only project structure:

~~~bash
zelyra context examples/auth_crud_api.zyl --format=json
~~~

The context response contains the project entry and only declarations the
current compiler can actually understand, including tables, fields, pages with
typed data bindings, CRUD resources, forms, APIs, and source spans. It does not connect to MariaDB,
execute email, expose credentials, or include rendered confidential content.

Inspect deterministic source dependencies for a program:

~~~bash
zelyra impact examples/auth_crud_api.zyl --format=json
zelyra impact examples/auth_crud_api.zyl --symbol table:customers --format=json
~~~

The impact response lists source-level tables, SQL, forms, CRUD resources,
views, APIs, permissions, contracts, and a deterministic `references` edge
list for known relationships. Email, job, test, and live schema impact are
explicitly empty or marked unavailable; the command never connects to MariaDB.
Use `--symbol <kind:name>` to focus the result on one known node, for example
`table:customers`. The focused response contains only directly connected
references and related node IDs. Unknown nodes produce `E-IMPACT-001` and a
non-zero exit code.

Preview a validated symbol rename without modifying the source:

~~~json
{
  "schema_version": "1",
  "entry": "examples/fibonacci.zyl",
  "expected_source_fingerprint": "fnv1a64:18f35ecb3e2f99c4",
  "operations": [
    {"kind": "rename", "symbol": "function", "from": "fibonacci", "to": "fib"}
  ]
}
~~~

Save that request as `change.json` and run:

~~~bash
zelyra edit --format=json change.json
~~~

The request is versioned and may only name an existing `.zyl` file inside the
resolved Zelyra project root. The original and proposed source must pass the
compiler checks. The result reports exact token spans and a deterministic source
fingerprint. For `--apply`, the request must carry the fingerprint from the
preview, which prevents overwriting a file changed in the meantime. Applying is
explicit:

~~~bash
zelyra edit --format=json --apply change.json
~~~

The source is reparsed and fully checked before the atomic replacement, so an
invalid or semantically unsafe proposal cannot be written.

Function, type, and record renames are AST-aware: declarations and known
references are renamed, while local bindings that shadow the symbol remain
unchanged. Table, view, form, and CRUD declarations plus their structured
references are also supported. Component renames update the declaration and
known opening or closing component tags in HTML bodies. Table renames update
checked SQL table positions but leave literals, comments, parameters, and HTML
unchanged.

### Safe automation boundary

Generated code must pass the compiler and tests. An AI must not be trusted
because its output looks plausible. It may not silently add capabilities,
weaken diagnostics, disable tests, reveal secrets, or approve destructive
schema changes. Human approval remains required for risky database and
security operations.

The next planned machine interfaces are richer typed gaps and complete impact
analysis. They will extend the versioned common JSON envelope rather than
replace it. Benchmark results will be published only after reproducible
experiments; this handbook contains no invented comparison.
