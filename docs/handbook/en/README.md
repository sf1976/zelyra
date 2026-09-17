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
Table → Types → SQL → Forms → CRUD → Web page
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

Minimal `zelyra.toml`:

~~~toml
[project]
name = "machine-management"
version = "0.1.0"
zelyra = "0.1"

[capabilities]
database = true
network = false
~~~

Important commands:

| Command | Purpose |
|---|---|
| `zelyra check app.zyl` | lex, parse, resolve, and type-check |
| `zelyra build app.zyl` | check and build an application |
| `zelyra run app.zyl` | run a program |
| `zelyra serve app.zyl` | start the HTTP server |
| `zelyra verify app.zyl` | classify contracts |
| `zelyra db inspect app.zyl` | inspect the live schema |
| `zelyra db plan app.zyl` | display schema changes |
| `zelyra db apply app.zyl` | apply an approved plan |

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
        sql {
            INSERT INTO customers (name, email)
            VALUES (:name, :email)
        }

        redirect "/customers"
    }
}
~~~

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
/machines?sort=number&order=asc
/machines/new
/machines/42/edit
~~~

🗺️ Fully custom typed components and fine-grained view overrides are part of
the continuing view roadmap.

## 13. Authentication and permissions

🧪 Zelyra supports Argon2 login, persistent MariaDB sessions, logout, route
guards, and database-backed permission checks.

~~~zelyra
auth users {
    table: users
}

page "/admin" {
    requires auth
    permits "machines.manage"

    html {
        <h1>Machine management</h1>
    }
}
~~~

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

Static checking is implemented. A complete operating-system sandbox for every
capability is not yet available.

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
PROVEN: reduce.ensures[0]
PROVEN: reduce.invariant[0]
FAILED: reduce.invariant[1]
~~~

`FAILED` means that the invariant is false on a feasible analyzed path or is
not preserved by the loop body. `RUNTIME_CHECK` means that runtime checking
is required because the symbolic verifier cannot complete the proof. Only
`PROVEN` is a mathematical proof.

## 16. Configuration and secrets

Project configuration belongs in `zelyra.toml`; secrets do not:

~~~toml
[project]
name = "machine-management"
version = "0.1.0"
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
- APIs and OpenAPI;
- structured concurrency;
- broader formal verification;
- optimization models for real planning problems.

Zelyra keeps the common case short without locking the door when the special
case arrives:

> **Generated when possible. Custom where needed. Verified everywhere.**

Now build a table, read the SQL, and check the backup. In that order.
