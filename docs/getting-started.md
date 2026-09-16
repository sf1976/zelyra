# Getting started with Zelyra

[Deutsch](getting-started.de.md) · English

This guide takes a new user from a repository checkout to the first Zelyra
program, web page, schema, SQL block, and form validation. It describes the
current Zelyra 0.1 implementation, not only the long-term language vision.

## What is Zelyra?

Zelyra is a statically typed language for applications that work with business
data. The language is intended to bring together:

- general-purpose programming;
- database schema definitions;
- checked native SQL;
- web pages and HTTP routing;
- forms and validation;
- initial CRUD lists, details, Create/Edit forms, configurable columns, and
  delete actions;
- later, full CRUD, APIs, authentication, authorization, and verification.

The central design goal is to define important information once. For example,
a required String with a maximum length in a table can also provide the basis
for form validation. The compiler and supporting tools should make that
connection explicit and checkable.

Zelyra is not a low-code editor and not an ORM-only language. A complete
programming language remains available. Developers can write functions, native
SQL, custom pages, actions, and business rules.

## What works in 0.1?

The current repository contains:

- a Rust lexer, parser, AST, HIR, type checker, interpreter, and CLI;
- immutable-by-default variables, functions, expressions, conditions, loops,
  Option, Result, nominal types, and pattern matching;
- MariaDB as the default database backend;
- schema planning and DDL support for MariaDB, SQLite, and PostgreSQL;
- MariaDB inspection, schema application, and native SQL execution;
- checked SQL blocks with named parameters;
- an initial built-in HTTP server and GET router;
- an initial schema-aware form parser and validator;
- initial CRUD lists with MariaDB search, filters, sorting, pagination, and
  CSRF-protected delete actions.

The following are not complete yet: full CRUD generation, database-backed
login and sessions, APIs, capabilities, contracts, formal verification, and
production packaging. The current authentication adapter provides route
guards, but it is not a user login system.

## 1. Requirements

For the easiest source installation you need:

- a Unix-like shell with Bash;
- Git;
- curl, if Rust is not already installed;
- a network connection for the first Rust toolchain installation.

The installer is designed for Linux and macOS style environments. It installs
only for the current user and does not use sudo. A platform-native standalone
release installer for users who do not want Rust is planned.

Apache is not required. A database server is not required for the language
core, web, and local form examples. MariaDB is needed only when you want to
run database operations against MariaDB.

## 2. Download and install

Clone the repository:

~~~bash
git clone https://github.com/sf1976/zelyra.git
cd zelyra
~~~

Install the command:

~~~bash
./install.sh
~~~

The script builds the CLI in release mode and installs it at:

~~~text
~/.local/bin/zelyra
~~~

If the shell cannot find zelyra, add the directory to PATH for the current
shell:

~~~bash
export PATH="$HOME/.local/bin:$PATH"
~~~

To make that permanent, put the export in the startup file used by your shell,
such as ~/.bashrc or ~/.zshrc.

Verify the installation:

~~~bash
zelyra --help
~~~

The installation does not require an account password. Do not run the installer
as root unless there is a separate system-wide packaging reason.

## 3. Run the first program

Run the Fibonacci example:

~~~bash
zelyra run examples/fibonacci.zyl
~~~

Expected output:

~~~text
55
~~~

The source is ordinary Zelyra:

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

Check without running:

~~~bash
zelyra check examples/fibonacci.zyl
~~~

The check command lexes, parses, resolves, and type-checks the program. It
should report:

~~~text
ok: examples/fibonacci.zyl
~~~

## 4. Create a new project

Create a project directory:

~~~bash
zelyra new my-app
cd my-app
zelyra run main.zyl
~~~

The generated project contains a minimal zelyra.toml and main.zyl. To
initialize the current directory instead:

~~~bash
mkdir another-app
cd another-app
zelyra init
~~~

The current project file is intentionally small:

~~~toml
[project]
name = "my-app"
version = "0.1.0"
zelyra = "0.1"
~~~

## 5. Language basics

Bindings are immutable by default:

~~~zelyra
name = "Anna"
age: Int = 25
~~~

Mutation is explicit:

~~~zelyra
mutable counter = 0
counter = counter + 1
~~~

Functions have typed parameters and can have a typed result:

~~~zelyra
fn add(a: Int, b: Int) -> Int {
    return a + b
}
~~~

Normal values are not null. Optional values use the question-mark form and
must be handled explicitly:

~~~zelyra
name: String?

match name {
    Some(value) => print(value)
    None => print("Unknown")
}
~~~

The language also distinguishes nominal domain types:

~~~zelyra
type UserId = Id
type OrderId = Id
~~~

A UserId and an OrderId are different types even though both are based on Id.
This prevents an important class of business-logic mistakes.

## 6. Start a web page without Apache

Zelyra 0.1 includes an initial built-in HTTP server. The example is:

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

Open this address in a browser:

~~~text
http://127.0.0.1:3000/hello/Zelyra
~~~

Use another local port if needed:

~~~bash
zelyra serve examples/hello_web.zyl 127.0.0.1:8080
~~~

The current server supports GET routes, literal path segments, path
parameters, query-string removal for matching, basic HTTP parsing, and HTML
responses. Values inserted from path parameters are HTML-escaped by default.

Apache, nginx, Caddy, TLS termination, process supervision, and firewall
configuration are deployment concerns. They are not needed for this first
local step.

## 7. Define a MariaDB schema

MariaDB is the default Zelyra backend and the primary runtime reference. A
schema can be described directly:

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

The existing machine-management example contains related tables:

~~~text
examples/machine_management_mariadb.zyl
~~~

Set a MariaDB connection string through the environment:

~~~bash
export DATABASE_URL='mariadb://user:password@127.0.0.1:3306/my_app'
~~~

Inspect the current database:

~~~bash
zelyra db inspect examples/machine_management_mariadb.zyl
~~~

Preview the schema difference:

~~~bash
zelyra db plan examples/machine_management_mariadb.zyl
~~~

Bootstrap a new MariaDB database and apply its initial schema:

~~~bash
zelyra db bootstrap examples/machine_management_mariadb.zyl
~~~

Apply a reviewed plan:

~~~bash
zelyra db apply examples/machine_management_mariadb.zyl
~~~

Destructive changes are refused unless explicitly approved:

~~~bash
zelyra db apply examples/machine_management_mariadb.zyl --allow-destructive
~~~

Review destructive plans carefully. Never place real passwords in a committed
Zelyra file, documentation example, or shell script. Environment variables,
secret managers, and restricted deployment configuration are preferred.

## 8. Use SQLite locally

SQLite is useful when a project needs a local file database without a running
database service:

~~~bash
export DATABASE_URL='sqlite:///tmp/my-app.sqlite3'
zelyra db bootstrap examples/machine_management_sqlite.zyl
zelyra db inspect examples/machine_management_sqlite.zyl
~~~

SQLite and MariaDB are both part of the supported database direction. The
current native SQL runtime path is primarily exercised with MariaDB; schema
DDL and inspection support are broader than runtime query support at this
stage.

## 9. Write native SQL

SQL is written in a native block:

~~~zelyra
customer = sql<Customer?> {
    SELECT id, name, email
    FROM customers
    WHERE id = :id
}
~~~

Named parameters are bound as parameters rather than concatenated into SQL.
When the schema is available, Zelyra can check tables, columns, aliases,
parameters, nullability, and result mappings.

Insert, update, and transaction blocks are also part of the current syntax:

~~~zelyra
transaction {
    sql {
        UPDATE inventory
        SET quantity = quantity - :amount
        WHERE product_id = :product
    }
}
~~~

Zelyra respects complex SQL. It does not force developers to replace every
query with an ORM abstraction.

## 10. Validate a schema-aware form

A form can point to a table and inherit its fields:

~~~zelyra
form CustomerCreate -> customers {
    fields {
        name
        email
    }
}
~~~

An explicit field can add presentation and validation settings:

~~~zelyra
form CustomerForm {
    field email: Email {
        label: "E-mail"
        required
        max: 255
        widget: email
    }
}
~~~

Run local validation:

~~~bash
zelyra form validate examples/customer_form.zyl CustomerCreate \
  name=Anna email=anna@example.test
~~~

Successful output:

~~~text
valid: CustomerCreate
~~~

Try invalid input:

~~~bash
zelyra form validate examples/customer_form.zyl CustomerCreate \
  email=not-an-email
~~~

The validator reports the missing required name and the invalid email. It also
rejects unknown fields, overlong values, invalid numbers, invalid booleans,
and submitted readonly fields.

The built-in server also exposes each form at /forms/FormName. GET renders an
HTML form with escaped values and a per-server CSRF token. POST checks the
token, parses URL-encoded data, and validates the fields. Invalid input returns
field errors with HTTP 422. A form without an action returns HTTP 202 as a
validation confirmation.

For a database-backed action, use the MariaDB example:

~~~bash
export DATABASE_URL='mariadb://root:<password>@127.0.0.1:3306/zelyra_forms'
zelyra db bootstrap examples/customer_form_action.zyl
zelyra serve examples/customer_form_action.zyl
~~~

After a valid POST, Zelyra binds the declared fields as prepared parameters,
executes the action in a MariaDB transaction, and returns HTTP 303 to its
redirect. Without `DATABASE_URL` the action returns HTTP 503; database failures
return a generic HTTP 500. The action SQL is statically checked against the
source schema before serving.

Relationship fields are loaded from MariaDB automatically. In
`examples/machine_form.zyl`, `department: Department required` becomes a
`<select>` using the department ID and name. Zelyra rejects a submitted ID
that is not present in the current database, before executing the form action.

A minimal CRUD resource can be added with:

~~~zelyra
crud Machine -> machines
~~~

For a tailored resource, configure the title, visible columns, searchable
columns, and filters:

~~~zelyra
crud Machine -> machines {
    title: "Machines"
    list { number name department active }
    search { number name }
    filter { department active }
}
~~~

All blocks are optional. Configured columns are checked against the schema;
relationship fields are mapped to their stored foreign-key columns. The
minimal form keeps the default list, text search, and non-ID filters.

This exposes `GET /machines` with escaped output, search, exact filters,
allowlisted sorting, pagination, linked detail pages, and generated Create/Edit
forms at `/machines/new` and `/machines/<id>/edit`. Filters use
`filter_<column>`; sorting uses `sort=<column>&order=asc|desc`.

## 11. Useful commands

~~~text
zelyra new <directory>                  create a project
zelyra init [directory]                 initialize a project
zelyra check <file.zyl>                 check source
zelyra build <file.zyl>                 build/check source
zelyra run <file.zyl>                   execute a program
zelyra serve <file.zyl> [address]       start the built-in HTTP server
zelyra form validate <file> <Form> ...  validate form input
zelyra db create <file.zyl>             print schema DDL
zelyra db bootstrap <file.zyl>          create/apply initial schema
zelyra db inspect <file.zyl>            inspect current database
zelyra db plan <file.zyl>               show schema changes
zelyra db apply <file.zyl>              apply reviewed changes
~~~

For Rust contributors, use cargo commands from the repository root:

~~~bash
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
~~~

## 12. Common problems

### zelyra: command not found

The user-local bin directory is not on PATH:

~~~bash
export PATH="$HOME/.local/bin:$PATH"
~~~

### Rust or curl is missing

The source installer needs curl only when Rust is missing. Install curl using
the operating system package manager, then run install.sh again. Alternatively,
install Rust through the official rustup process first.

### DATABASE_URL is required

Database inspection, bootstrap, plan against a live database, and apply need a
connection string:

~~~bash
export DATABASE_URL='mariadb://user:password@127.0.0.1:3306/my_app'
~~~

The form validation command and the language examples do not need a database
connection.

### A destructive schema change is refused

This is intentional. Run db plan, inspect the affected rows and SQL, then
repeat db apply with the explicit allow-destructive flag only after review.

### A page starts but the browser shows 404

Check the exact route, including every path parameter. The example route is
hello followed by one name segment. Query strings are ignored for matching, but
the path itself must still match.

## 13. Where to go next

Read the phase documents for implementation details:

- [Phase 2: Type System](phase-2.md);
- [Phase 3: Database Core](phase-3.md);
- [Phase 4: Native SQL](phase-4.md);
- [Phase 5: Web Core](phase-5.md);
- [Phase 6: Forms](phase-6.md);
- [Phase 7: CRUD](phase-7.md).
- [Phase 8: Authentication and authorization](phase-8.md).

The German versions use the same filenames with the .de.md suffix.

The CRUD list now also provides detail, Create/Edit forms, configurable
columns, and a CSRF-protected Delete action. The next authentication step is
database-backed login, secure sessions, and role/permission lookup.
