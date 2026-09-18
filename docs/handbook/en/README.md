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
--mariadb`. This creates `.env.example`, `Dockerfile`, and
`docker-compose.mariadb.yml`. Set `ZELYRA_WEB_PORT` in `.env` to choose the
port used by the internal Zelyra server and its local published port.

Users who do not want to install Rust can download the prebuilt Linux or
Windows archive from the [GitHub Releases page](https://github.com/sf1976/zelyra/releases).
Each archive includes a SHA-256 checksum, the CLI, both README languages, and
the license notices.

Minimal `zelyra.toml`:

~~~toml
[project]
name = "machine-management"
version = "0.1.37"
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
| `zelyra verify app.zyl` | classify contracts |
| `zelyra doc app.zyl --openapi` | generate an OpenAPI document |
| `zelyra db inspect app.zyl` | inspect the live schema |
| `zelyra db setup app.zyl` | create a MariaDB database and initial schema |
| `zelyra db plan app.zyl` | display schema changes |
| `zelyra db apply app.zyl` | apply an approved plan |
| `zelyra audit inspect app.zyl` | inspect the latest audit events |
| `zelyra audit export app.zyl --format json` | export audit events as JSON |
| `zelyra audit verify app.zyl` | verify required audit fields |
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

The compiler requires exactly one `<slot />` in a named view. The page content
is inserted before routing, while existing authentication, authorization, and
escaping remain active. Typed self-closing components with declared properties
are available as well:

~~~zelyra
component Badge {
    props { text: String }
    html { <span class="badge">{text}</span> }
}

page "/status" {
    html { <Badge text="Ready" /> }
}
~~~

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
error. Fallback slot content remains planned.

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
    field active: Bool { required }
    sql {
        UPDATE customers SET active = :active WHERE id = :id
    }
}
~~~

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
version = "0.1.37"
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
