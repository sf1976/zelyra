# TECHNICAL REFERENCE MANUAL

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

### Prerequisites

For language examples, you need:

- Linux, macOS, or Windows (PowerShell/WSL);
- `curl`;
- a working shell.

MariaDB is required only for database, form actions, auth, and CRUD examples.

### Install from the repository

~~~bash
git clone https://github.com/sf1976/zelyra.git
cd zelyra
./install.sh
~~~

The installer is repeatable and user-local. Control options:

~~~bash
./install.sh --help
./install.sh --dry-run --root "$HOME/.local"
./install.sh --check
./install.sh --uninstall
~~~

Use `--no-rustup` to disable automatic Rust installation; `--no-path` suppresses PATH instructions. With `--root PATH` or `ZELYRA_INSTALL_ROOT`, you can choose a different user-local target. Outdated `cargo` PATH entries are detected and not executed blindly.

Published releases for Linux x86_64 and Windows x86_64 can be installed without Rust or Cargo. The archive is downloaded over HTTPS and verified with SHA-256:

~~~bash
./install.sh --release v1.0.5
~~~

On Windows, `install.ps1` is available for PowerShell and `install.cmd` for the Command Prompt:

~~~powershell
git clone https://github.com/sf1976/zelyra.git
Set-Location zelyra
.\install.ps1
zelyra --version
~~~

Release archive installation on Windows:

~~~powershell
.\install.ps1 -Release v1.0.5
~~~

Then:

~~~bash
zelyra --version
zelyra --help
~~~

If your shell cannot find `zelyra`:

~~~bash
export PATH="$HOME/.local/bin:$PATH"
~~~

### Docker and container environment

Zelyra does not install Docker itself, modify operating-system packages, or request root privileges. For container and MariaDB workflows, Docker with Compose support is required:

- **Linux:** Follow the [official Linux installation guide](https://docs.docker.com/engine/install/).
- **Windows:** Use [Docker Desktop for Windows](https://docs.docker.com/desktop/setup/install/windows-install/) with Compose support.
- **macOS:** Use [Docker Desktop for Mac](https://docs.docker.com/desktop/setup/install/mac-install/).

Verify Docker Compose before first run:

~~~bash
docker compose version
~~~

If Docker is installed but access to its socket is denied, Zelyra reports a safe Linux group-membership remedy (`sudo usermod -aG docker $USER`) instead of raw Docker errors. Port collisions are also reported safely without exposing credentials.

### Updating Zelyra

An update consists of two separate parts: updating the compiler in the compiler repository, and verifying your application project. Your `.zyl` files, `zelyra.toml`, and `.env` live in your application project and are never overwritten by `install.sh`.

#### Update from the Git repository

~~~bash
cd /path/to/zelyra
git status --short
git pull --ff-only origin main
cargo check --workspace
./install.sh
zelyra doctor /path/to/your-project/main.zyl --json
~~~

Check the installed version with `zelyra --version`:

~~~bash
zelyra --version
~~~

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

## 4. Create a project and use the CLI

### Compiler repository and application projects

The GitHub repository `sf1976/zelyra` is the compiler repository. It contains lexer, parser, runtime, database, and web modules, as well as the CLI. Your own application is a separate directory. You do not need to work inside the Zelyra compiler source, and you should never store credentials there.

The project root is the directory containing your Zelyra source file and, if present, `zelyra.toml`.

### ✅ Create projects with the CLI

~~~bash
zelyra new addressbook
cd addressbook
zelyra run main.zyl
~~~

Initialize an existing directory:

~~~bash
mkdir addressbook
cd addressbook
zelyra init
~~~

For a local MariaDB and web-server template:

~~~bash
zelyra new machine-management --mariadb
cd machine-management
~~~

This creates `main.zyl`, `.env.example`, `Dockerfile`, `docker-compose.mariadb.yml`, and a protected `.env` with random passwords.

**Automatic Port Selection on Conflict:**
If default ports `3000` (web) or `3306` (MariaDB) are occupied, `zelyra new`, `zelyra init`, and `zelyra setup` automatically select the next free host ports and record them in `.env`. The optional `--web-port <p>`, `--host-port <p>`, and `--db-host-port <p>` flags enforce exact ports.

### ✅ Zelyra Setup Assistant (Console and Web)

`zelyra setup` provides unified setup actions across CLI and browser:

#### Console setup

From a MariaDB project directory:

~~~bash
zelyra setup
zelyra setup --database
zelyra setup --schema
zelyra setup --all
zelyra setup --host-port 18080 --db-host-port 3308
~~~

- `zelyra setup`: Creates a protected `.env` if missing. Existing `.env` files are never overwritten.
- `zelyra setup --database`: Starts Compose services (detects `docker compose` or legacy `docker-compose`).
- `zelyra setup --schema`: Starts services and safely applies the schema from `main.zyl`.
- `zelyra setup --all`: Executes configuration, container startup, and schema migration in one step.

Choose the project type before running setup:

- A minimal project without a database does not need `setup`:

  ~~~bash
  zelyra init ./test_a
  cd ./test_a
  zelyra check main.zyl
  zelyra run main.zyl
  ~~~

- For a new MariaDB project, `zelyra new test_a --mariadb` creates the required
  Compose and configuration files. Then `cd test_a && zelyra setup --all`
  prepares the database and schema. Alternatively, `zelyra init ./test_a
  --mariadb` enables MariaDB when initializing a project.

Project paths are relative or absolute, and those forms are not interchangeable:

~~~bash
zelyra setup ./test_a
zelyra setup /test_a
~~~

The first command looks for a directory under the current directory; the second
looks for `/test_a` under the filesystem root. If the
requested path does not exist, Zelyra reports `E-SETUP-001`; when a same-named
relative directory exists, the error suggests the matching relative command.

#### Local browser setup (`zelyra setup --web`)

~~~bash
zelyra setup --web
~~~

The server binds strictly to `127.0.0.1:3030` by default and outputs a one-time URL with a secure random token:

~~~text
Zelyra setup web is running on http://127.0.0.1:3030/
open: http://127.0.0.1:3030/?token=<local-token>
~~~

- Provides browser actions to configure `.env`, start containers, and apply schemas.
- Strictly local-only for security; never expose to public interfaces.
- If port `3030` is busy, the assistant automatically chooses the next free port. Stop with `Ctrl+C`.

Verify your project state non-destructively:

~~~bash
zelyra doctor main.zyl --env-file .env
~~~

For a complete business starter project:

~~~bash
zelyra new machine-management --template mariadb-crud
cd machine-management
zelyra setup --all
~~~

### Check, run, format, and verify

`zelyra` is the documented user-facing CLI. Use `zelyra check <file.zyl>` to
check syntax, types, contracts, and capabilities before running a program with
`zelyra run <file.zyl>`. The formatter is `zelyra fmt <file.zyl>`; add
`--check` to check formatting without writing changes. Use
`zelyra verify <file.zyl>` for the supported contract and invariant checks.

The current CLI reference does not list a `zelyra test` subcommand. Put
project-specific test routines in an executable source file and run that file
with `zelyra run`; Chapter 19 explains assertion-based checks. Likewise, the
reference does not list a separate `zelyrac build` executable: the compiler is
invoked through `zelyra check` and `zelyra run`.

LSP means Language Server Protocol, the standard for editor-to-language-server
communication. The documented editor entry point is `zelyra editor`, which
starts local Studio; the CLI reference does not list a separate `zelyra lsp`
subcommand.

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

Type names document the meaning of IDs:

~~~zelyra
type MachineId = Id
type OrderId = Id
~~~

These aliases have no separate runtime identity. Validate ID assignments and tenant boundaries in business logic; a type name alone does not establish access protection.
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

Functions expose failures through their return type. The current language core
uses `Result<T, E>` with `Ok` or `Err`:

~~~zelyra
fn load_number(found: Bool) -> Result<Int, String> {
    if found {
        return Ok(42)
    }
    return Err("not found")
}
~~~

## 7. MariaDB and tables

✅ MariaDB is the default backend and primary runtime reference. The generated
Compose template uses `mariadb:11`; the compiler does not enforce a specific
MariaDB server version. Database commands call the external `mariadb` client.

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

### Declared number series

A `String` field can declare a transactional number series for generated CRUD
create forms:

~~~zelyra
table customers {
    id: Id primary auto
    customer_number: String(32) required auto_sequence("KD-{0001}")
    name: String(255) required
}

table quotes {
    id: Id primary auto
    quote_number: String(40) required auto_sequence("AG-{year}-{0001}")
}
~~~

`{0001}` means a counter padded with zeroes to at least four digits. The
counter cannot grow beyond the width reserved by the format. `{year}` is
replaced by the current UTC calendar year and gets a separate counter for each
year. Supported formats contain literal text, exactly one counter token, and
at most one year token. Unknown or incomplete tokens and formats too long for
the String column are rejected.

The schema plan adds `_zelyra_number_sequences`. Review the plan and
deliberately apply it with `zelyra db apply main.zyl` before using the CRUD
form. MariaDB and PostgreSQL support runtime allocation; SQLite returns a
clear unsupported-backend error. Allocation shares the transaction used to
save the record. Browser-submitted values are ignored, and generated numbers
cannot be edited in the edit form. Zelyra checks for existing values before
allocation. Numbering is not guaranteed gapless, and voided documents must not
silently reuse their numbers.

### Prepare MariaDB safely

Create a dedicated database and application user. Do not run the Zelyra web
server as the MariaDB `root` account. The following is run as an administrative
MariaDB user; the password is requested interactively:

~~~bash
mariadb --host=127.0.0.1 --port=3307 --user=root --password
~~~

~~~sql
CREATE DATABASE `address_book`
    CHARACTER SET utf8mb4
    COLLATE utf8mb4_unicode_ci;

CREATE USER 'zelyra'@'127.0.0.1'
    IDENTIFIED BY 'ENTER_A_LOCAL_PASSWORD_HERE';

GRANT SELECT, INSERT, UPDATE, DELETE, CREATE, ALTER, INDEX, REFERENCES
    ON `address_book`.* TO 'zelyra'@'127.0.0.1';

SHOW GRANTS FOR 'zelyra'@'127.0.0.1';
~~~

Use `IF NOT EXISTS` when the database must be repeatably prepared. The grants
are deliberately limited to this database; never use `GRANT ALL ON *.*` in an
application quickstart. InnoDB is the expected engine for transactions and
foreign keys, but the current Zelyra SQL generator does not add
`ENGINE=InnoDB` itself. Inspect and supplement generated SQL before applying it
if your server policy requires that clause.

The listed grants cover normal CRUD work and the initial schema. A destructive
`db apply --allow-destructive` may additionally need `DROP`; grant it only
deliberately and, where possible, temporarily. `db setup` is an administrator
operation because it creates the database itself.

The generated Compose file publishes `127.0.0.1:3306` by default. If that host
port is occupied, change its binding to `127.0.0.1:3307:3306`: the host port is
then `3307`, while MariaDB remains on `3306` inside the container. Another
Compose service connects to host `mariadb` and port `3306`; a host process uses
`127.0.0.1` and the published port.

### Inspect the address schema

For the example, `zelyra db create src/main.zyl` currently produces, in essence:

~~~sql
CREATE TABLE IF NOT EXISTS `addresses` (
    `id` BIGINT PRIMARY KEY NOT NULL AUTO_INCREMENT,
    `first_name` VARCHAR(100) NOT NULL,
    `last_name` VARCHAR(100) NOT NULL,
    `street` VARCHAR(150) NOT NULL,
    `postal_code` VARCHAR(10) NOT NULL,
    `city` VARCHAR(100) NOT NULL,
    `email` VARCHAR(255)
) ENGINE=InnoDB DEFAULT CHARACTER SET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
~~~

For MariaDB, the generated table statements include
`ENGINE=InnoDB DEFAULT CHARACTER SET=utf8mb4 COLLATE=utf8mb4_unicode_ci`.
The output is therefore ready for the documented MariaDB baseline, but it is
still a reviewed schema proposal: inspect `db plan` before applying changes to
an existing database.

### Additional database backends

Besides MariaDB, the schema CLI can currently process SQLite and PostgreSQL.
Select the backend in the `.zyl` file:

~~~zelyra
database main { engine: sqlite database: "address-book.sqlite3" }
database main { engine: postgres database: "address-book" }
~~~

For SQLite, `DATABASE_URL` uses the `sqlite://` or `sqlite:` scheme; MariaDB
accepts `mariadb://` and the compatible `mysql://` name. The `create`,
`inspect`, `plan`, and `apply` schema operations account for tables, columns,
foreign keys, and indexes. Automatic database creation through `db setup` and
`db bootstrap` currently supports MariaDB and SQLite; for PostgreSQL use
`db create`, `db inspect`, `db plan`, and then `db apply`.

~~~bash
export DATABASE_URL='sqlite:///tmp/address-book.sqlite3'
zelyra db bootstrap src/main.zyl
zelyra db inspect src/main.zyl
~~~

The native SQL runtime is still primarily exercised with MariaDB. A supported
schema backend therefore does not automatically mean that every runtime query
behaves identically on every backend.

## 8. Inspecting and applying schemas

Zelyra uses no migration classes. The source file is the desired schema; the
CLI compares it with the current database. The actual workflow is:

1. Check the source: `zelyra check src/main.zyl`.
2. Print SQL only: `zelyra db create src/main.zyl`.
3. Read the current schema: `zelyra db inspect src/main.zyl`.
4. Plan the difference: `zelyra db plan src/main.zyl`.
5. Read the plan and assess its risk.
6. Apply only after review: `zelyra db apply src/main.zyl`.

Supply the connection only through the process environment:

~~~bash
export DATABASE_URL='mariadb://zelyra:ENTER_A_LOCAL_PASSWORD_HERE@127.0.0.1:3307/address_book'
~~~

⚠️ Zelyra does **not** load `.env` automatically. It is a safe local place to
store values, but the process must receive the variables before the CLI runs;
see section 16.

Print the desired SQL:

~~~bash
zelyra db create src/main.zyl > sql/addresses.generated.sql
~~~

If you added `ENGINE=InnoDB` and the charset clauses after review, apply that
exact file manually with the MariaDB client:

~~~bash
mariadb --host=127.0.0.1 --port=3307 --user=zelyra --password \
    address_book < sql/addresses.generated.sql
~~~

`zelyra db apply` does not read an edited SQL file; it regenerates its plan from
the `.zyl` source. Use either the unchanged Zelyra plan or the reviewed manual
client path, rather than blindly doing both.

Inspect, plan, and apply:

~~~bash
zelyra db inspect src/main.zyl
zelyra db plan src/main.zyl
zelyra db apply src/main.zyl
~~~

Destructive changes require explicit permission:

~~~bash
zelyra db apply src/main.zyl --allow-destructive
~~~

That flag does not mean “probably fine.” It means “I read the plan, have a
backup, and my pulse is normal.”

For MariaDB, `db setup` and its `db bootstrap` alias first try to create the
database and then apply the generated schema. `DATABASE_URL` therefore needs
administrative privileges for that operation. An application user with limited
grants should use `db create` and have an administrator apply the SQL instead.
Without `DATABASE_URL`, `db plan` can plan against an empty schema; that is a
preview, not a connection test.

❌ There is no `zelyra db check`, `zelyra schema inspect`, `zelyra schema plan`,
or `zelyra schema apply` command. The actual readiness test is
`zelyra doctor src/main.zyl`; it checks static rules, Cargo, `DATABASE_URL`
when set, and the local web port.

### In 10 minutes to a first Zelyra application

These steps consistently use the directory `address-book`, source file
`src/main.zyl`, database `address_book`, user `zelyra`, and host port `3307`.

1. Create the project and copy the environment template:

   ~~~bash
   zelyra new address-book --mariadb
   cd address-book
   cp .env.example .env
   ~~~

2. Put local placeholder values into `.env`. Compose reads the `MARIADB_*`
   values when the container is first initialized; Zelyra itself reads only
   `DATABASE_URL`. Change the Compose binding to `127.0.0.1:3307:3306` before
   starting it.

3. Start MariaDB:

   ~~~bash
   docker compose -f docker-compose.mariadb.yml up -d mariadb
   docker compose -f docker-compose.mariadb.yml ps
   ~~~

4. As a MariaDB administrator, create the database and user with the SQL from
   section 7. Use an obvious local placeholder, never a real password in docs.

5. Save the valid address-book source as `src/main.zyl` and check it:

   ~~~bash
   zelyra check src/main.zyl
   ~~~

6. Load `DATABASE_URL` into the process and test readiness. `.env` is not
   automatically loaded by Zelyra:

   ~~~bash
   set -a
   . ./.env
   set +a
   zelyra doctor src/main.zyl --json
   ~~~

7. Generate and inspect SQL:

   ~~~bash
   zelyra db create src/main.zyl > sql/addresses.generated.sql
   sed -n '1,160p' sql/addresses.generated.sql
   ~~~

8. Inspect and plan:

   ~~~bash
   zelyra db inspect src/main.zyl
   zelyra db plan src/main.zyl
   ~~~

9. Apply only after review:

   ~~~bash
   zelyra db apply src/main.zyl
   ~~~

10. Start the web server. A CRUD route also needs a web definition accepted by
    the server; `crud` alone is not a static export:

    ~~~bash
    zelyra serve src/main.zyl 127.0.0.1:3000
    ~~~

On Windows, use `Copy-Item` and set the process variable in PowerShell:

~~~powershell
Copy-Item .env.example .env
$env:DATABASE_URL = 'mariadb://zelyra:ENTER_A_LOCAL_PASSWORD_HERE@127.0.0.1:3307/address_book'
zelyra doctor src/main.zyl --json
~~~

The generated Compose template publishes host port `3306` by default. `3307`
in this handbook is a deliberate alternative to avoid a collision; change the
Compose port binding before copying the quickstart.

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

✅ A complete, safe web layer with typed data bindings, declarative query controls, and reusable views is implemented.

~~~zelyra
page "/machines/{name}" {
    html {
        <html>
            <body>
                <h1>Machine {name}</h1>
                <p>Running smoothly.</p>
            </body>
        </html>
    }
}
~~~

Start the web server:

~~~bash
zelyra serve app.zyl
~~~

Then visit:

~~~text
http://127.0.0.1:3000/machines/Press-7
~~~

Path parameters are HTML-escaped by default. The web core handles GET routes, path parameters, query strings, and HTML responses.

### Typed view data loading and collection loops

View interpolations are checked before the server starts. A page may load a typed record and use checked field access:

~~~zelyra
page "/customers/{name}" {
    load customer = sql<Customer> {
        SELECT id, name FROM customers WHERE name = :name
    }
    html { <h1>{customer.name}</h1> }
}
~~~

SQL is checked against the schema, route parameters are safely bound, and capabilities and permissions are enforced. Collections can be rendered with a typed server-side loop:

~~~zelyra
page "/customers" {
    load customers = sql<Customer[]> { SELECT id, name FROM customers }
    html { <ul>for customer in customers { <li>{customer.name}</li> }</ul> }
}
~~~

### Declarative web query controls: Search, sort, pagination, and filter

Pages can declare typed query inputs for explicit SQL:

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

For page collections declaring search, sorting, pagination, or filtering, Zelyra automatically generates semantic query controls and preserves URL state:

~~~zelyra
page "/customers" {
    search { name email }
    sort { name }
    paginated 25
    filter { name quantity }
    load customers = sql<Customer[]> { SELECT id, name, quantity FROM customers }
    html {
        <p>Page: {page} of {pages} (Total: {total})</p>
        <p>Sort: {sort} ({order})</p>
    }
}
~~~

- `search { name email }`: Compiler-checked whitelist for `?search=...`, parameterized `LIKE` query.
- `sort { name }`: Accepts declared result fields; `order` accepts `asc` or `desc` (`/customers?sort=name&order=desc`).
- `paginated 25`: Validates `page` as positive integer, applies parameterized `LIMIT`/`OFFSET`, and exposes `page`, `pages`, and `total` as `UInt`.
- `filter { ... }`: Supports typed operators: text fields support `eq`, `contains`, `starts_with`, `ends_with`, and null checks; numeric fields additionally support `gt`, `gte`, `lt`, `lte`. Examples: `/customers?filter_name__contains=Acme` or `/customers?filter_quantity__gte=10`. Invalid operators return controlled HTTP 400.

### Reusable view layouts and components

A named view provides a reusable page layout with slots:

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

Typed components declare properties via `props`:

~~~zelyra
component Badge {
    props { text: String }
    html { <span class="badge">{text}</span> }
}

page "/status" {
    html { <Badge text="Ready" /> }
}
~~~

Components accept child HTML through default and named slots with fallback content:

~~~zelyra
component Panel {
    html {
        <section class="panel">
            <header><slot name="header">Default Header</slot></header>
            <div class="body"><slot /></div>
        </section>
    }
}

page "/dashboard" {
    html {
        <Panel>
            <slot name="header"><h1>My Dashboard</h1></slot>
            <p>Main panel body content.</p>
        </Panel>
    }
}
~~~

*Note:* Component slots nested inside a component invocation no longer require an outer page `view:` layout.

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
Standalone forms may override that URL with `route: "/contact"`. A route may
also capture one segment, for example `route: "/unsubscribe/{token}"`;
captured values are available to SQL actions as same-named bound parameters
(integer-shaped values bind as integers, other values as strings). The route
must be an absolute local path. Query strings, fragments, encoded paths,
duplicate parameters, malformed segments, and collisions with other routes
are rejected. GET still renders the form normally, and POST retains the
standard CSRF and validation checks.

For an opaque, one-time route token, a standalone form can verify it on GET and
require a matching row again inside the POST transaction before any later
statement is executed:

~~~zelyra
form Unsubscribe -> suppressions {
    route: "/unsubscribe/{token}"
    verify sql<Int[]> {
        SELECT id FROM suppressions WHERE token_hash = SHA2(:token, 256)
    }

    action unsubscribe {
        require_match
        confirm_page {
            title: "Confirm unsubscribe"
            message: "Stop these emails?"
            submit: "Unsubscribe"
        }
        sql<Int[]> {
            SELECT id FROM suppressions WHERE token_hash = SHA2(:token, 256)
        }
        sql {
            UPDATE suppressions
            SET unsubscribed_at = COALESCE(unsubscribed_at, CURRENT_TIMESTAMP)
            WHERE token_hash = SHA2(:token, 256)
        }
        redirect "/unsubscribed"
    }
}
~~~

`verify` accepts only a compiler-checked `SELECT` and runs it in a
read-only transaction before rendering the form. Captured route values are
bound parameters; no matching row produces 404. Verified-token pages use
`Cache-Control: no-store`. `require_match` is limited to direct SQL actions: its
first statement must be a `SELECT`, run in the same transaction as later
statements. If it returns no rows, the request is 404 and all writes are
skipped. This supports idempotent actions when the subsequent SQL is written
idempotently; it does not provide a general workflow engine. A form with
`post_only` has no GET form route and delegates GET handling to a page at the
same path, so it cannot declare `verify`; validate that page's GET data there.
An executable MariaDB example is in
[`examples/csv_mailer_unsubscribe.zyl`](../../examples/csv_mailer_unsubscribe.zyl);
the isolated database/browser-flow check is
[`tests/mariadb-form-route-e2e.sh`](../../tests/mariadb-form-route-e2e.sh).

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
/machines?sort=number&order=asc
/machines/new
/machines/42/edit
~~~

🗺️ Fully custom typed components and fine-grained view overrides are part of
the continuing view roadmap.

### CRUD views, actions, and soft delete

🧪 The current CRUD layer can be customized inside the safe default paths. A
list can use cards without losing search, filters, sorting, pagination, output
escaping, or permission checks:

~~~zelyra
crud Customer -> customers {
    view {
        list {
            mode: cards
            empty: "No customers found."
        }
        detail {
            mode: cards
            title: "Customer details"
        }
        form {
            mode: cards
            title: "Customer form"
            submit: "Save customer"
        }
        delete {
            title: "Delete customer"
            message: "This action cannot be undone."
            submit: "Delete now"
        }
        loading { message: "Loading customers ..." }
        error {
            title: "Customers unavailable"
            message: "Please try again later."
        }
    }
}
~~~

For the common case, a shared field profile can uniformly control the generated
list, detail view, and create/edit forms:

~~~zelyra
crud Customer -> customers {
    view {
        fields { name email active }
    }
}
~~~

An explicit `list { ... }` remains a local override for list/detail. Primary keys
and auto-generated fields are automatically excluded from forms; the compiler
rejects unknown profile fields. When the technical `id` column is omitted from
visible `fields` (as in `name email active` above), Zelyra automatically links
the first displayed field (`name`) to the record detail page for both table and
card layouts.

Domain actions remain POST-only, parameterized, and protected. An action can bundle multiple SQL statements executed atomically within a shared MariaDB transaction:

~~~zelyra
crud Invoice -> invoices {
    action lock {
        label: "Post and lock (audit compliance)"
        confirm: "Do you really want to permanently post and lock this invoice?"
        permits "invoices.lock"
        field reason: String required
        sql {
            UPDATE invoices SET is_locked = true WHERE id = :id
            INSERT INTO invoice_audit_logs (invoice_id, action, message) VALUES (:id, "locked", :reason)
        }
        success "Invoice successfully posted and locked."
        redirect "/invoices"
    }
}
~~~

#### Execution Model and Transactional Behavior of Actions

- **Atomic database transaction:** All SQL statements within `sql { ... }` run together in a single MariaDB transaction (`with_decision_transaction`). If any statement fails (e.g., due to a constraint violation), the entire transaction is rolled back atomically.
- **Parameter binding:** Parameters such as `:id` are bound automatically from the URL path of the target row. Custom fields declared with `field <name>: <Type>` are rendered in the confirmation dialog as typed inputs, validated upfront, and passed as `:name` parameters to the SQL statements.
- **Since 1.2.28:** Actions may use `guard when ... { error: ... }` and a typed `execute { ... }` body. The compiler checks `row` against the resource table and `session` contains optional authenticated tenant/user identities. The web runtime executes supported queries through the locked request transaction and keeps its capability, permission, CSRF, tenant, and audit checks. MariaDB and PostgreSQL HTTP E2Es cover SQL atomicity, concurrent guards, and rollback. The MariaDB E2E also verifies a real PDF outbox insert; it does not send an email.

The runtime enforces the database capability, CSRF protection,
authentication, and the declared permission. Actions can also declare typed
fields, a server-side confirmation page, and custom success or error pages.
This extends the existing CRUD runtime; it is not a freely programmable
frontend generator.

Reversible deletion is available through soft delete:

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

Normal lists and details show only rows whose column is `NULL`; archived
records are available through `?archived=true` and can be restored through a
CSRF-protected action. Permanent deletion, retention rules, and bulk archiving
remain planned.

When an auth definition declares an audit table, CRUD create, update, delete,
archive, restore, and custom actions write events in the same MariaDB
transaction. Passwords, tokens, secrets, and hashes are removed from change
details.

## 13. Authentication and permissions

🧪 Zelyra supports Argon2 login, persistent MariaDB sessions, logout, route
guards, and database-backed permission checks. Five failed attempts for the
same normalized e-mail address within 15 minutes trigger a 60-second HTTP 429
lockout. A successful login rotates and invalidates the previous browser
session token.

Every state-changing browser form requires its CSRF token and a same-origin
`Origin` or `Referer` matching the request `Host` and effective scheme.
Malformed, missing, or cross-origin evidence is rejected, so copying a token
from another browser cannot authorize a cross-site form submission. API
requests carrying browser-origin information or a browser session cookie also
receive this check, including `GET` requests because handlers are not yet
statically restricted to read-only behavior. Cross-origin API access is
possible only for an exact origin explicitly listed in the project's CORS
policy; a session cookie additionally requires credentialed CORS. The current
CSRF token is process-scoped rather than individually stored per session, so
these origin checks are a required part of the protection.

The Zelyra server currently speaks plain HTTP. Put it behind a trusted TLS
terminating proxy before exposing it beyond a local development machine. The
proxy must preserve the public `Host`, overwrite `X-Forwarded-Proto` with the
actual external scheme, and prevent direct public access to the application
port. Zelyra uses that header to validate the effective origin and to add the
`Secure` attribute to session cookies for HTTPS requests. Never trust a
client-supplied forwarded header at an exposed proxy boundary.

The server also checks every supplied `Host` header against
`ZELYRA_ALLOWED_HOSTS`. The default permits only `localhost`, `127.0.0.1`, and
`[::1]`, which also blocks DNS rebinding through arbitrary hostnames. Add the
actual hostname explicitly to the comma-separated `.env` setting when using a
custom domain or LAN host. Schemes, ports, and wildcards are not accepted.
Duplicate security-sensitive request headers such as `Host`, `Origin`,
`Referer`, `Cookie`, and `Authorization` are rejected to avoid ambiguous
parsing. The response policy `Referrer-Policy: same-origin` allows
same-origin API GETs to provide that evidence without sending referrer
information to other origins.
Process environment overrides the project `.env`, which overrides the
fallback. The allowlist does not replace TLS or origin/CSRF checks.

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
all permissions inherited from the user's roles. In the built-in administration
view, the permission revocation form transmits the valid CSRF token and
reliably removes selected permissions.

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

An origin must begin with `http://` or `https://`. Paths, queries, fragments,
wildcards, and a trailing slash are not allowed. Allowed responses receive
`Access-Control-Allow-Origin` and `Vary: Origin`; enabled credentials add
`Access-Control-Allow-Credentials: true`. A preflight response returns HTTP 204
with the allowed methods and requested headers. Forbidden origins or methods
return structured JSON errors; method errors include the `Allow` header.

A hidden button is not a security boundary. Authorization must be enforced on
the server-side action. Browsers become remarkably creative when trusted.

### API input and secure response defaults

API request bodies for methods other than `GET` and `DELETE` may currently use
`application/json` or `application/x-www-form-urlencoded`. Unsupported media
types return HTTP 415 as a structured JSON error. JSON bodies must be objects;
each declared field is then converted to and checked against its Zelyra type.

The HTTP parser checks `Content-Length`, reads complete bodies across multiple
network reads, and limits request bodies to 1 MiB. Headers are limited to 64
KiB. An oversized body is rejected with HTTP 413 before the handler runs.

All HTML, JSON, redirect, error, and preflight responses receive these secure
default headers:

~~~http
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Referrer-Policy: same-origin
~~~

These defaults do not replace TLS, authentication, authorization, CSRF
protection, or an appropriate Content-Security-Policy.

### Role management and tamper-evident audit

🧪 Roles and role permissions can be managed with the available CLI commands
when the `auth` definition configures the corresponding tables:

~~~zelyra
auth users {
    table: users
    sessions: auth_sessions
    permissions: user_permissions
    roles: user_roles
    role_permissions: role_permissions
    audit: auth_audit_log
    admin_path: "/admin/access"
    admin_permission: "auth.manage"
    admin_role: admin
}
~~~

~~~bash
DATABASE_URL='mariadb://user:password@127.0.0.1:3306/app' \
  zelyra auth role grant app.zyl 42 manager
DATABASE_URL='mariadb://user:password@127.0.0.1:3306/app' \
  zelyra auth role-permission grant app.zyl manager customers.edit
~~~

`revoke` removes the respective assignment. The commands bind values as SQL
parameters and check the project schema first. Do not use a real password in
the example value.

With `audit: auth_audit_log`, authentication, role, and CRUD events can be
inspected or exported:

~~~bash
DATABASE_URL='mariadb://user:password@127.0.0.1:3306/app' \
  zelyra audit inspect app.zyl --limit 100
DATABASE_URL='mariadb://user:password@127.0.0.1:3306/app' \
  zelyra audit export app.zyl --format json > audit.json
zelyra audit verify app.zyl
~~~

For visible tamper detection, enable chaining explicitly:

~~~zelyra
auth users {
    table: users
    audit: auth_audit_log
    audit_chain: true
}
~~~

The audit table then needs `id`, `previous_hash`, and `entry_hash`, usually
`String(64)`. Zelyra stores lowercase SHA-256 hex values. The hash covers the
canonical pipe-separated sequence
`previous_hash|actor_user_id|event|target_user_id|details|created_at`.
`zelyra audit verify` checks both links and hashes. Pruning is deliberately
disabled for chained logs because deleting an entry would break the chain.
Non-chained old entries can be deliberately removed with
`zelyra audit prune ... --before ... --confirm`.

An optional browser administration page is enabled by `admin_path`,
`admin_permission`, and `admin_role`. It can manage users, passwords, active
status, roles, and role permissions; its forms are CSRF-protected. The last
active administrator remains protected.

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
Database Network FileSystem Environment Process Clock Random Console Email
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
host APIs are implemented, but each requires its own resource grant and
remains deliberately limited in this first version.

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

### Structured concurrency

An initial, limited concurrency flow is available through `parallel` and
`await`:

~~~zelyra
parallel {
    customer = await load_customer()
    orders = await load_orders()
}
~~~

Each branch binds its result with `await`. Branches receive an immutable
snapshot of surrounding values and are merged in source order before execution
continues. An error in one branch fails the whole block after the started
branches finish. The type checker rejects `await` outside a `parallel` block.
The current runtime uses one worker thread per branch; cancellation and
database connection-pool integration are not implemented yet.

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
version = "0.9.9"
zelyra = "0.1"

[capabilities]
database = true
network = false
~~~

Connections and secrets are supplied via protected environment variables:

~~~bash
export DATABASE_URL='mariadb://user:password@127.0.0.1:3306/zelyra_demo'
~~~

Rules:
- never commit `.env` to version control;
- never place production credentials in code examples;
- do not log secrets;
- separate development, test, and production databases;
- never run destructive tests against production.

### Simple defaults, optional feature switches

The beginner path does not require a feature configuration. Advanced project surfaces can be selected in `zelyra.toml`, while environment-specific, non-secret overrides can be placed in `.env` or the process environment:

~~~toml
[features]
web = true
api = true
crud = true
auth = true
audit = true
~~~

| Switch | `.env` / Process variable | Default | Meaning |
|---|---|---:|---|
| `web` | `ZELYRA_FEATURE_WEB` | `true` | Pages, forms, and web resources |
| `api` | `ZELYRA_FEATURE_API` | `true` | `api` declarations and API surface |
| `crud` | `ZELYRA_FEATURE_CRUD` | `true` | `crud` declarations and generated CRUD surface |
| `auth` | `ZELYRA_FEATURE_AUTH` | `true` | `auth` declarations and authentication surface |
| `audit` | `ZELYRA_FEATURE_AUDIT` | `true` | Audit trail configuration |

Precedence order:
```text
Process environment → .env → zelyra.toml → safe defaults
```

When source code uses a disabled surface, the compiler reports `E-FEATURE-001`. Feature switches cannot disable type checking, SQL validation, capabilities, contracts, CSRF protection, or security rules.

Inspect effective configuration safely without exposing secrets:

~~~bash
zelyra config main.zyl
zelyra config main.zyl --format=json
~~~

### `.env` values in generated projects

| Variable | Default in generated project | Usage | Secret |
|---|---:|---|---|
| `ZELYRA_WEB_PORT` | `3000` | Port of internal web server inside container | no |
| `ZELYRA_HOST_PORT` | `3000` (or auto-selected free port) | Locally published web port | no |
| `ZELYRA_DB_HOST_PORT` | `3306` (or auto-selected free port) | Locally published MariaDB port | no |
| `DATABASE_URL` | project-dependent | MariaDB connection URI (`mariadb://user:pass@host:port/db`) | yes |
| `MARIADB_DATABASE` | `zelyra_app` | Compose: database name | no |
| `MARIADB_USER` | `zelyra` | Compose: application user | no |
| `MARIADB_PASSWORD` | randomly generated | Compose: user password | yes |
| `MARIADB_ROOT_PASSWORD` | randomly generated | Compose: root password | yes |
| `ZELYRA_AUTH_TOKEN` | none | Optional local bearer token for protected endpoints | yes |
| `ZELYRA_AUTH_PERMISSIONS` | empty list | Comma-separated local permission allowlist | no |

### Test and CI variables

Variables prefixed with `ZELYRA_INSTALL_ROOT`, `ZELYRA_BIN`, `*_E2E_*`, and `GENERATED_*` serve internal CI and local integration tests (such as `tests/generated-project-docker-e2e.sh`, `tests/sqlite-e2e.sh`). They are not application configuration and must never contain production secrets.

### Environment access inside the language

Via the built-in `env(name)` function, Zelyra code can read environment variables if `uses Environment` and `[capabilities] environment = true` are declared:

~~~zelyra
fn configured_mode() -> String? uses Environment {
    return env("ZELYRA_MODE")
}
~~~

`DATABASE_URL` and sensitive secrets must never be exposed via `env(...)` to unverified code.

## 17. Diagnostics and troubleshooting

Zelyra aims to explain errors without requiring an archaeological excavation
through a stack trace.

### Test the connection independently

Test MariaDB without Zelyra first. The password is requested interactively and
is not written into shell history:

~~~bash
mariadb \
    --host=127.0.0.1 \
    --port=3307 \
    --user=zelyra \
    --password \
    address_book
~~~

Then run `zelyra doctor src/main.zyl --json` (optionally with `--env-file .env`
and `--port 18080`), the implemented Zelyra readiness test. There is no
`zelyra db check` command. `doctor` checks source code, schema, DB
connectivity, Docker Compose, and host ports. Without `DATABASE_URL`, `doctor`
reports a warning; with an unreachable URL it reports a failure. A running
database container alone does not prove that host, port, user, and database
match.

### Safe diagnostic commands

~~~bash
pwd
ls -la
docker compose ps
docker compose logs mariadb
ss -ltn
mariadb --version
~~~

In PowerShell, use `Get-Location`, `Get-ChildItem`, `docker compose ps`, and
`mariadb --version` as the first checks. Never print `DATABASE_URL` or a
password in diagnostics.

### Common failures

| Error | Likely cause | Fix |
|---|---|---|
| `Permission denied` | insufficient file permissions or no access to the client | inspect `ls -la`, use `chmod 600 .env`, and check the install path |
| `Access denied for user` | wrong password or user is granted for another host | inspect `SHOW GRANTS FOR 'zelyra'@'127.0.0.1';` and rotate the password |
| `Connection refused` | no service listens on the host/port | inspect `docker compose ps`, `ss -ltn`, and the published port |
| `Can't connect to server` | wrong host/port or container is not ready | inspect `docker compose logs mariadb`; use `127.0.0.1:3307` from the host and `mariadb:3306` inside Compose |
| `Unknown database` | URL and MariaDB use different database names | run `SHOW DATABASES;` and correct `DATABASE_URL` |
| wrong port | confused host `3307` with container `3306` | host uses `3307`; a Compose service uses `3306` |
| MariaDB container not started | Compose error, occupied port, or unhealthy volume | inspect `docker compose ps` and `docker compose logs mariadb` |
| user granted only for another host | `'zelyra'@'localhost'` is not always `'zelyra'@'127.0.0.1'` | grant the exact host and inspect grants |
| missing environment variable | `DATABASE_URL` was not exported | load `.env` or set the process variable; Zelyra does not load it itself |
| `.env` not found | wrong working directory or assumed automatic loader | use `pwd`, `ls -la`, and export the variable explicitly |
| invalid numeric port | URI port is not numeric | use digits such as `3307`; otherwise URL parsing fails |
| wrong charset | database uses another charset/collation | inspect the database; `db setup` uses `utf8mb4`/`utf8mb4_unicode_ci` |
| TLS error | unsupported TLS parameters were appended to the URL | use the current URL form; TLS configuration is planned |
| test database rejected for safety | a protection mechanism is expected but not implemented | Zelyra does not automatically prevent production access in tests; check variables manually |
| PostgreSQL SQL sent to MariaDB | wrong backend or incompatible DDL | inspect the `.zyl` backend and read `zelyra db create` output before applying |

If `mariadb` cannot be started at all, Zelyra reports the external process
start error. The CLI does not contain its own MariaDB driver.

~~~bash
zelyra check app.zyl
~~~

Common error classes include unknown names and types, mutation of immutable
values, incomplete pattern matches, unknown tables or columns, missing SQL
parameters, incompatible result mappings, missing capabilities, invalid form
fields, failed contracts, and incomplete typed holes (`_`).

During development, you can place `_` as a placeholder for an unfinished
expression (typed hole). While `zelyra check` intentionally rejects incomplete
code for compilation, it reports helpful contextual diagnostics: the expected
type, visible variables and functions, active capabilities, contract obligations,
and source spans.

Language-only checks work without `DATABASE_URL`. Database operations report a
missing connection in a controlled way. Without the variable, `run` uses the
pure runtime; `serve` can start routes, but database-backed pages return a
controlled error.

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
- safe named email templates and attachments;
- personal read receipts and recipient authorization in the notification center;
- general background jobs beyond the existing bounded MariaDB outbox;
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

## 20. Zelyra compared with Rust

### The most important statement first

Zelyra is developed with Rust. The Zelyra compiler, language crates, and parts
of the runtime are Rust crates in the compiler repository. That does not mean
that Zelyra changes Rust or that Zelyra applications are Rust programs.

> **Zelyra is not modified Rust. Zelyra is an independent language whose
> compiler and runtime are developed in Rust.**

The Rust compiler is not forked or extended with Zelyra keywords. Zelyra is not
a Rust library and not a preprocessor that rewrites ordinary Rust code. A
`.zyl` file is read by Zelyra's own lexer and parser, converted into its own
AST/HIR structures, type-checked, and then processed by the Zelyra runtime.

Similar spelling in `fn`, braces, `if`, `match`, or static typing is a design
choice, not proof of identity. Grammar, semantics, and the programming model
are what matter. Zelyra is still an experimental prototype; its independence
grows as its own type system, SQL checking, runtime, and language constructs
are implemented.

The statements in this chapter were checked against the current source:
the lexer and `TokenKind` define Zelyra tokens, the parser produces its own
AST structures, and later modules perform resolution, type checking, contract
checking, SQL analysis, web processing, and runtime execution. The CLI and
database modules were reviewed as well. Where a feature exists only as a token,
AST node, or target design, it is not presented as a fully executable language
feature.

### General comparison

| Area | Rust | Zelyra | Essential difference | Zelyra status |
|---|---|---|---|---|
| Language category | general-purpose systems and application language | independent declarative language for business and web applications | different grammar and semantics | 🧪 |
| Main use | systems, services, CLI, embedded, WebAssembly, web | database-backed business and web applications | Zelyra bundles application layers | 🧪 |
| Compiler | `rustc` and Cargo ecosystem | its own Rust program in the Zelyra repository | Rust compiles the compiler; `rustc` does not compile `.zyl` | ✅ |
| Runtime | native Rust or a chosen runtime | its own Zelyra runtime, implemented in Rust | Zelyra executes its own values and rules | 🧪 |
| Memory management | ownership, borrowing, lifetimes | largely automatic/hidden for application code | no Rust borrow checker in `.zyl` code | 🧪 |
| Ownership | central Rust semantics | no equivalent `.zyl` construct | memory rules are not the same | 🗺️ |
| Borrowing | references and borrow checker | no equivalent `.zyl` construct | no Rust reference syntax | 🗺️ |
| Lifetimes | explicit or inferred lifetimes | no lifetime syntax | Zelyra does not expose this layer yet | 🗺️ |
| Static typing | mature, generic, trait-based | own checker with `Int`, `String`, `Option`, records, and more | Zelyra types are not Rust types | ✅ |
| Nullability | `Option<T>` | `T?`, such as `Email?` | Zelyra can derive schema/form rules | ✅ |
| Error handling | `Result<T, E>`, `Option<T>`, `?` operator | `Result<T, E>`, `Some`/`None`, runtime diagnostics | no Rust `?` operator syntax in Zelyra | ✅ |
| Database integration | external crates such as SQLx, Diesel, or SeaORM | intended as part of language, CLI, and runtime | different integration model; external client today | 🧪 |
| SQL | libraries, macros, or strings | native `sql<T> { ... }` with schema/parameter checks | Zelyra knows SQL as an expression | ✅ |
| MariaDB schema | not a Rust-language responsibility | tables are described in Zelyra | generator prints visible SQL | 🧪 |
| Forms | connect framework, templates, and validation yourself | `form` construct and table rules | the standard path is part of the model | 🧪 |
| CRUD | must be programmed or generated by a framework | declarative `crud Name -> table` with shared view field profile | current runtime serves CRUD routes | 🧪 |
| Views | external libraries or frameworks | `page`/`html`, named `view` layouts, typed `component` blocks, and named slots with fallbacks exist | no Rust equivalent; free styling components are still missing | 🧪 |
| Code formatting | `cargo fmt`, `rustfmt` | `zelyra fmt <file.zyl> [--check]` | deterministic Zelyra formatter, protects SQL and HTML | ✅ |
| Refactoring/impact | `rust-analyzer`, compiler APIs | `zelyra impact`, `zelyra edit` | versioned atomic JSON machine interfaces | 🧪 |
| Authentication | external web/auth crates | auth definition, sessions, and permission checks in web crate | Zelyra bundles the standard case | 🧪 |
| Permissions | user-designed types and middleware | `requires auth`, `permits`, and CRUD action permissions | declarative rules are checked server-side | 🧪 |
| Contracts | manual or library-based | native `requires {}` and `ensures {}` plus `verify` | contract syntax belongs to Zelyra | ✅ |
| Capabilities | APIs and libraries regulate effects | `uses Database`, `uses Network`, and more | effect declarations are language syntax | ✅ |
| Email | external SMTP/mail crates | `mail_queue`, `mail_deliver`, `uses Email` | MariaDB outbox, TLS, host permission, and an explicitly started worker | 🧪 |
| Jobs | external job/queue systems | no background-job construct | no stable job syntax | ❌ |
| Audit | external logging/audit crates | audit table, CLI inspection, and optional hash chain | tied to auth/CRUD and still experimental | 🧪 |
| Deployment | Cargo, containers, CI, and infrastructure are free choices | generated Docker/Compose template exists | template is a development start, not a production platform | 🧪 |
| Production maturity | widely used in production | Zelyra is experimental | maturity and ecosystem are not comparable | 🧪 |
| Ecosystem | very large: crates, tools, frameworks | small repository and few integrations | Zelyra cannot directly import Rust crates | 🧪 |

Rust is the technical foundation, not the application language behind Zelyra. A
Rust `Customer` struct and a Zelyra `customers` table may describe similar data,
but they do not create the same behavior.

### Detailed syntax comparison

In this chapter, `✅` means that the construct is present in the current source
and was checked with the installed Rust toolchain or Zelyra CLI. `🧪`, `🗺️`, and
`❌` continue to identify limited, planned, or currently unavailable features.

| Language feature | Zelyra syntax | Rust syntax | Semantic difference | Zelyra status |
|---|---|---|---|---|
| File extension | `app.zyl` | `main.rs` | own lexer and file type | ✅ |
| Program entry | `fn main() { ... }` | `fn main() { ... }` | same spelling, different language | ✅ |
| Function definition | `fn add(a: Int) -> Int { ... }` | `fn add(a: i64) -> i64 { ... }` | own AST types | ✅ |
| Parameters | `name: String` | `name: String` | similar position, different type semantics | ✅ |
| Return type | `-> Int` | `-> i64` | Zelyra abstracts a business type | ✅ |
| Return value | `return value` | `value` or `return value;` | final Rust expression returns a value | ✅ |
| Immutable variable | `value = 1` | `let value = 1;` | Zelyra is immutable without `mutable` | ✅ |
| Mutable variable | `mutable value = 1` | `let mut value = 1;` | different marker for mutation | ✅ |
| Integer | `Int` or `UInt` | `i32`, `i64`, `u32`, `u64` | Rust requires a concrete width; Zelyra abstracts it today | ✅ |
| Decimal | `Float` or `Decimal` | `f64` or `f32` | Zelyra width and exact overflow rules are not fully specified | 🧪 |
| Boolean | `true`, `false`, `Bool` | `true`, `false`, `bool` | own base-type model | ✅ |
| String | `String` and character literals | `String`, `&str`, character literals | Rust distinguishes ownership and borrowing | ✅ |
| Optional value | `Email?` or `Option<String>` | `Option<String>` | `?` is Zelyra's Option shorthand | ✅ |
| Missing value | `None` | `None` | same name in different enum models | ✅ |
| Lists/arrays | `Int[]`, `[1, 2, 3]` | `Vec<i64>`, `vec![1, 2, 3]` | no Rust macro syntax in Zelyra | ✅ |
| Named data types | `type CustomerId = Id`, `struct Customer { ... }` | `type CustomerId = u64`, `struct Customer { ... }` | records and structs are not interchangeable | ✅ |
| Conditions | `if ok { ... }` | `if ok { ... }` | block and expression rules differ | ✅ |
| `else` | `else { ... }` | `else { ... }` | similar control-flow form | ✅ |
| `match` | `match value { Some(x) => ... None => ... }` | `match value { Some(x) => ..., None => ... }` | both require exhaustive cases | ✅ |
| Loops | `for item in items`, `while`, `loop` | `for item in items`, `while`, `loop` | no Rust iterator traits in Zelyra | ✅ |
| Function call | `add(1, 2)` | `add(1, 2)` | same surface, different name resolution | ✅ |
| Output | `print(value)` | `println!("{}", value);` | Rust uses a macro with `!` | ✅ |
| Comments | `# comment`, standalone `##` as block delimiters | `// comment`, `/* ... */` | `#` is standard; `//` remains compatible; `##` blocks are not nested | ✅ |
| Newlines | usually separators; continuation after operators | usually whitespace | parser rules are independent | ✅ |
| Semicolons | accepted as statement separators, not required | often statement separators | Zelyra is not semicolon-dependent | ✅ |
| Blocks | `{ ... }` | `{ ... }` | braces determine blocks in both | ✅ |
| Indentation | readability, not block semantics | readability, not block semantics | whitespace is not Python-style structure | ✅ |
| Error handling | `Result<T, E>`, `Some`/`None` | `Result<T, E>`, `?`, `panic!` | Zelyra has no Rust `?` operator | ✅ |
| String interpolation | HTML bodies may use `{name}` | `format!("{name}")` or `println!("{}", name)` | no general Zelyra string interpolation | 🧪 |
| Modules | Not yet specified | `mod name {}`, files and modules | no `mod` syntax in parser | ❌ |
| Imports | Not yet specified | `use crate::module::Item;` | no import syntax | ❌ |
| Generics | `Option<T>`, `Result<T, E>`, limited built-in type arguments | general generics and traits | no user-defined Zelyra generics | 🧪 |
| Async functions | no `async fn`; `await`/`parallel` are limited | `async fn`, `.await`, futures | no stable Zelyra async model | 🧪 |
| Tables | `table customers { ... }` | no language construct | Zelyra connects table and schema | ✅ |
| Database types | `Id`, `String(100)`, `Email`, `Bool` | Rust types and external mapping crates | Zelyra emits SQL types from a table | ✅ |
| Relationships | `department: Department required` | field plus query/mapping logic | Zelyra derives foreign keys | ✅ |
| SQL queries | `sql<Customer[]> { SELECT ... }` | string/macro from a DB crate | Zelyra checks schema, parameters, result | ✅ |
| Forms | `form CustomerCreate -> customers { ... }` | no native form | web framework and validation required | ✅ |
| CRUD | `crud Customer -> customers` | no native CRUD | Zelyra runtime serves standard routes | 🧪 |
| Views | `page`, `view SiteShell`, and `component Badge` | no native view construct | named views/components exist; `input`/`render`/`??` remain target syntax | 🧪 |
| Preconditions | `requires { amount > 0 }` | no built-in equivalent | contract is part of the Zelyra function | ✅ |
| Postconditions | `ensures { result >= 0 }` | no built-in equivalent | verifier and runtime know Zelyra contracts | ✅ |
| Old values | Not yet specified; `old(...)` is not parsed | no general built-in contract standard | do not invent `old` syntax | ❌ |
| Capabilities | `uses Database` | no identical language construct | effects are declared visibly in Zelyra | ✅ |
| Email | `mail_queue` and `mail_deliver` | external crate/API | `Email` capability; enqueue inside `transaction` with `Database(write)` | 🧪 |
| Background jobs | Not yet specified | external queue/runtime crate | no job syntax | ❌ |
| Audit | `auth users { audit: auth_audit_log }` | external logging/audit crate | Zelyra ties audit to auth and CRUD events | 🧪 |
| API definitions | `api GET "/customers" { ... }` | router, handler, and types separately | Zelyra bundles route and contract | ✅ |

### Functions: same idea, different language

Both snippets express a small function in their own language. The Zelyra form
uses function syntax processed by the parser and was checked with the current
CLI.

~~~zelyra
fn add(a: Int, b: Int) -> Int {
    return a + b
}
~~~

~~~rust
fn add(a: i64, b: i64) -> i64 {
    a + b
}
~~~

Rust uses concrete integer types such as `i32`, `i64`, `u32`, or `u64`.
Zelyra offers `Int` for typical business logic; its definitive size, overflow
behavior, and database mapping still need complete specification.

### Fibonacci

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

~~~rust
fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }

    fibonacci(n - 1) + fibonacci(n - 2)
}

fn main() {
    println!("{}", fibonacci(10));
}
~~~

✅ The Zelyra version uses `Int`, `print`, and explicit `return`; Rust uses
`u64`, the `println!` macro, and the final expression as the return value. Rust
macros carry `!`. Semicolons are not required in Zelyra. Braces determine block
structure in both examples; indentation is for readability. A newline after `+`
continues the expression because the parser skips it there. Recursion works in
Zelyra only because function resolution and the runtime support it. This code
was not executed in this work run.

### Optional values

~~~zelyra
email: Email?
~~~

~~~rust
email: Option<String>
~~~

`Email?` is shorter and also expresses the domain type. Rust uses the general
generic `Option<T>`. Zelyra can account for `Email?` in table, form, and SQL
checking; a general automatic view/validation derivation is not available for
every surface.

### Tables

~~~zelyra
table customers {
    id: Id primary auto
    name: String(100) required
    email: Email?
    active: Bool default true
}
~~~

~~~rust
struct Customer {
    id: u64,
    name: String,
    email: Option<String>,
    active: bool,
}
~~~

The Rust struct creates no database table, SQL columns, validation, form, or
CRUD interface. Rust needs additional crates, macros, queries, handlers, and
templates. The Zelyra table is included in schema derivation and, where the web
features exist, in forms and CRUD.

### CRUD

~~~zelyra
crud Customer -> customers
~~~

Rust has no native equivalent. A typical Rust implementation combines a web
framework, routing, database crate, data model, queries, request types,
validation, handlers, templates or a frontend, error handling, and authorization.
Zelyra parses this declarative definition and the current runtime serves CRUD
routes, forms, search, filters, and CSRF-protected actions. This is experimental;
it is not a static frontend generator.

### SQL

The target syntax from the request included `with { ... }`. That is not in the
current parser. Parameters currently arrive as function parameters:

~~~zelyra
struct Customer { id: Int name: String email: Email? active: Bool }

fn active_customers(active: Bool) -> Customer[]
    uses Database
{
    return sql<Customer[]> {
        SELECT id, name, email, active
        FROM customers
        WHERE active = :active
        ORDER BY name
    }
}
~~~

~~~rust
let customers = sqlx::query_as!(
    Customer,
    r#"
        SELECT id, name, email, active
        FROM customers
        WHERE active = ?
        ORDER BY name
    "#,
    true
)
.fetch_all(&pool)
.await?;
~~~

Rust does not have SQL as a language feature; SQLx and other crates can provide
additional compile-time checks. Zelyra's current SQL checker validates, where
schema and types are known, tables, columns, aliases, parameters, nullability,
result mapping, and the `Database` capability. This does not automatically make
Zelyra better than SQLx. The current MariaDB generator and its missing table
options remain documented in section 7.

### Views: current syntax, not the target model

The target syntax with `view`, `input`, `render`, components, and `??` is not
current parser syntax. The current web core uses:

~~~zelyra
page "/customers/{name}" {
    html {
        <h1>Customer {name}</h1>
    }
}
~~~

Rust has no built-in HTML or component syntax; templates and web frameworks are
added externally. Zelyra's `page`/`html` form, named views, and typed components
exist. Multiple slots, nested components, and the target `input`/`render`
syntax are not stable yet; `??` is not an implemented Zelyra operator.

### Contracts

The target syntax `require amount > 0` and `old(...)` is not current syntax. The
parser accepts `requires {}` and `ensures {}`:

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

Rust has no direct built-in equivalent. `requires` describes preconditions and
`ensures` postconditions. Runtime checks and formal verification differ:
`zelyra verify` can report `PROVEN`, `RUNTIME_CHECK`, `UNPROVEN`, or `FAILED`.
Accessing an old value with `old(...)` is not implemented.

### Capabilities

~~~zelyra
fn load_customers() -> Customer[] uses Database {
    return sql<Customer[]> {
        SELECT id, name, email, active FROM customers
    }
}
~~~

`uses` makes allowed side effects visible in a signature. `Database`, `Network`,
`FileSystem`, `Environment`, `Process`, `Clock`, `Random`, and `Console` are known
capabilities in the current runtime code. Rust has no identical built-in
capability system; access is usually organized through types, values, and
library APIs. `Email` is the application-mail capability. `mail_queue` also requires `Database(write)` and a transaction; SMTP delivery checks the allowed host.

### What Rust developers should not look for in Zelyra

For typical business applications, Zelyra is not intended to require explicit
lifetimes in ordinary application code, borrowing debug sessions for simple
forms, choosing among many integer widths, or assembling a web framework from
many crates. That is an abstraction, not a claim that memory and runtime
concerns disappear.

Zelyra is inspired by static typing, useful diagnostics, safe defaults, explicit
mutation, pattern matching, records/enums, clear boundaries, reproducible
tooling, and formal checks.

### What makes Zelyra its own language

| Zelyra feature | Benefit | Status |
|---|---|---|
| One domain definition | fewer contradictory duplicate definitions | 🧪 |
| native checked SQL | catch database errors before execution where possible | ✅ |
| declarative CRUD | standard administration with little code | 🧪 |
| `page`/`html` web core | simple typed path values and HTML responses | 🧪 |
| visible MariaDB SQL | traceable schema changes | 🧪 |
| `requires` and `ensures` | state business rules explicitly | ✅ |
| capabilities | make allowed effects visible | ✅ |
| integrated audit | trace changes | 🧪 |

### Honest conclusion

> Zelyra looks similar to Rust in places because both are modern, statically
> typed languages with braces and clear function signatures. Zelyra nevertheless
> follows a different programming model: database, SQL, forms, CRUD, views, and
> business rules are intended to be parts of one language system. Rust is the
> technical foundation of the compiler—not the language Zelyra application
> developers write.

> **Status:** Zelyra is currently an experimental language prototype. Some
> language features shown here describe the binding target and are not fully
> implemented. The status next to each example shows what exists in current
> source and was checked in this work run.

Further reading: [Introduction](#1-what-makes-zelyra-different), [language
basics](#5-variables-types-and-functions), [MariaDB](#7-mariadb-and-tables),
[SQL](#9-native-sql), [forms](#11-forms), [views/web pages](#10-web-pages),
[CRUD](#12-crud), [contracts](#15-contracts-and-verification),
[capabilities](#14-capabilities), [implementation status](#17-diagnostics-and-troubleshooting),
and [roadmap](#22-support-and-technical-evidence). The live repository status is also shown on
the [status page](https://siedelmann.com/en/status).

## 21. Positioning and current development status

The current `docs/positioning.md` describes Zelyra as an independent language
for database-backed business applications. It does not replace compiler tests;
it explains how the implemented pieces are intended to fit together.

### What makes Zelyra distinct

1. **One source of truth:** schema, types, SQL, forms, CRUD, views, and APIs
   should come from definitions that can be checked together.
2. **SQL stays first-class:** SQL is not hidden behind an ORM abstraction; the
   program can check tables, parameters, and result shapes.
3. **Business features are language primitives:** tables, forms, CRUD, pages,
   authentication, permissions, and contracts belong to one model.
4. **Secure defaults are visible:** HTML escaping, parameterized SQL, CSRF
   protection, null safety, and server-side permissions are more than advice.
5. **Proof claims are honest:** `PROVEN`, `RUNTIME_CHECK`, `UNPROVEN`, and
   `FAILED` distinguish static proofs from runtime checks and open cases.
6. **A short path and a full language:** the declarative standard case is short,
   while custom functions and native SQL remain available for domain logic.
7. **A small starting stack:** the built-in server and CLI aim to support local
   learning and development without Apache, PHP, or a mandatory framework set.

The project is nevertheless an experimental prototype. The status marks and
the roadmap therefore matter more than a broad product claim.

## 22. Support and technical evidence

Choose a backend using the [database matrix](https://github.com/sf1976/zelyra/blob/main/docs/database-support.en.md). MariaDB 11.4.13 (alongside 10.11.19, 11.8.9, and 12.3.3), MySQL Server 8.4.11, and PostgreSQL 16.15 form the verified multi-backend server matrix. Relational CRUD featuring true foreign key constraints (`fk_...`), preflight orphan rejection (`error[E-DB-005]`) with data preservation, relational joins, and restrictive deletion protection (RESTRICT policy) is documented and proven across all three engines. On MySQL, write locks are dynamically bound to transaction deadlines via `innodb_lock_wait_timeout`. Native decision transactions (`with_decision_transaction`) guarantee rollback, unwinding, and deadline safety on a borrowed connection. SQLite supplements local schema and basic CRUD workflows.

This evidence applies to the listed workflows, not arbitrary database features. Advanced authentication, mail outbox, and notification examples primarily utilize MariaDB and MySQL.

Installation, upgrade, rollback, migration, and restore must match the platform and server version in use. Checksums, SBOMs, and attestations identify release artifacts; they do not establish application correctness. The [operational verification goals](https://github.com/sf1976/zelyra/blob/main/docs/release-plans/0.9.9.en.md) require reproducible rehearsals; a goal alone is not a passing result.

For a learning project, begin with `check` and `run`, then schema preview and planning, then pages/forms/CRUD, and finally authentication, mail, and integrations.

## 23. AI-native development

The current architecture and specification documents add an important
principle:

> **The AI writes. Zelyra checks.**

Zelyra should be useful to humans and AI systems alike while remaining fully
independent of AI. The compiler and tests are the trust boundary; a plausible
model explanation is not proof of correctness. Human-written and generated
code go through the same lexer, parser, name, type, SQL, capability, contract,
test, and runtime checks.

### Machine interfaces available today

🧪 Tool-facing JSON uses a common envelope with `schema_version: "1"`. Human
output remains the default; request JSON explicitly with `--format=json`:

~~~json
{
    "schema_version": "1",
    "command": "check",
    "success": false,
    "diagnostics": []
}
~~~

Output is deterministic. `schema_version` is mandatory; new optional fields
may be added within a version, while incompatible changes require a new
version. JSON goes only to `stdout`, technical messages to `stderr`. Source
spans use zero-based UTF-8 byte offsets, one-based line/byte columns, and a
half-open interval. Secrets, timestamps, random IDs, machine-dependent
absolute paths, and live database contents do not belong in these outputs.

#### Canonical code formatting

✅ `zelyra fmt <file.zyl>` produces deterministic code formatting after
successful lexing and parsing. `zelyra fmt <file.zyl> --check` writes no files
and returns an error code if reformatting is required, allowing CI to enforce
canonical source formatting.

~~~bash
zelyra fmt examples/fibonacci.zyl
zelyra fmt examples/fibonacci.zyl --check
~~~

The formatter preserves line comments and treats SQL and HTML blocks as opaque
source. It is idempotent: formatting an already formatted document yields
byte-for-byte identical output.

#### Typed holes

✅ Expression typed holes with `_` are available as an initial safe stage. The
compiler reports expected contextual types, visible values and functions, active
capabilities, contract obligations, and source spans:

~~~zelyra
fn double(x: Int) -> Int {
    return _
}
~~~

`zelyra check` reports diagnosis `E-HOLE-001` with the expected type `Int` and
visible identifiers. Buildable commands (`build`, `run`, `serve`) reject
incomplete code before lowering and execution. Declaration-level holes remain
planned.

#### Structured project context

✅ `context` is read-only: it does not connect to MariaDB, use the network, send
email, run jobs, or expose secrets:

~~~bash
zelyra context examples/auth_crud_api.zyl --format=json
~~~

It reports declared functions, tables, SQL queries, CRUD resources, forms, APIs,
and source spans.

#### Deterministic impact analysis

🧪 Source dependencies can be deterministically analyzed:

~~~bash
zelyra impact examples/auth_crud_api.zyl --format=json
zelyra impact examples/auth_crud_api.zyl --symbol table:customers --format=json
~~~

The impact response reports source-based tables, SQL, forms, CRUD resources,
views, APIs, permissions, contracts, and a deterministic `references` edge
list for known relationships. Each known edge includes source, target, kind,
and source span. Email, job, test, and live schema impacts remain explicitly
empty or unavailable; the command never connects to MariaDB.

Using `--symbol <kind:name>` focuses the output on a known node such as
`table:customers`. The focused response contains only directly connected
references and related node IDs. Unknown nodes return `E-IMPACT-001` and a
non-zero exit code.

#### Atomic semantic edits

🧪 A validated symbol rename can be previewed without modifying source:

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

Save this as `change.json` and run:

~~~bash
zelyra edit --format=json change.json
~~~

The request is versioned and must point to an existing `.zyl` file inside the
resolved Zelyra project root. Source code before and after the change must pass
all compiler checks. The result reports exact token spans and a deterministic
source fingerprint.

When applying changes with `--apply`, the request must include the fingerprint
from the preview to prevent overwriting concurrently modified files (stale-source
protection). Without the explicit `--apply` flag, it remains a preview:

~~~bash
zelyra edit --format=json --apply change.json
~~~

Before atomic replacement, the source is re-parsed and fully validated; an
invalid or semantically unsafe change will not be written.

Renames of functions, types, and records are AST-based: declarations and known
references are updated while local bindings of the same name remain untouched.
Table, view, form, and CRUD declarations and their structured references are
also supported. Table renames update checked SQL table positions (`FROM`,
`JOIN`, `INTO`, `UPDATE`) while preserving literals, comments, parameters, and
HTML. Component renames update both the declaration and known opening and
closing component tags in HTML bodies.

### Safety boundary and benchmark

AI tools must not silently add capabilities, widen permissions, execute
destructive SQL, weaken diagnostics, disable tests, or reveal secrets.
Destructive schema changes and security-sensitive changes require visible human
approval. Zelyra does not automatically send source code to external AI
services; planned integrations should be open, local-capable, vendor-neutral,
and versioned.

The new AI-authoring benchmark is a specification in
`docs/benchmarks/ai-authoring.md`. With versioned fixtures and identical tasks,
it is intended to measure first-pass compilation, correction loops, time to
passing tests, tokens, security failures, missed dependencies, unsafe schema
changes, and human review effort. No comparative results have been published.
A secret leak or unapproved destructive change is a security failure and cannot
be balanced by claimed productivity.

---

## 24. Authoritative Sources and Compiler Verification (Source Authority)

> **Principle:** Zelyra is its own language. The parser and verified tests decide what exists.

When sources disagree, report the disagreement explicitly and use this authoritative order:

1. **[Formal Language Specification](https://github.com/sf1976/zelyra/blob/main/docs/specification.md) and phase documents**
2. **Compiler implementation code:** Lexer, AST, parser, name resolution, type checking, and semantic compiler code across `lexer`, `parser`, `ast`, `hir`, `cli`, `runtime`, `database`, `web`, and `forms`
3. **Official automated language and integration tests:** Workspace tests (`cargo test --workspace`), machine-interface tests, and E2E scripts in `tests/`
4. **Official standard library:** (once formally introduced)
5. **Official Zelyra examples:** `.zyl` files in `examples/` verified by the current compiler
6. **Documentation and handbook explanations**

### Invariants for developers and AI agents

- **Roadmap is planning, not syntax:** Future phase proposals become valid syntax only after implementation in lexer and parser.
- **Rust is the implementation language, not Zelyra:** Zelyra is developed in Rust, but Rust syntax in a `.zyl` file is invalid unless Zelyra's grammar explicitly defines it.
- **No invented commands:** Every CLI command must exist in `cli/src/main.rs`.
- **Quality gate:** Every feature undergoes formatting (`cargo fmt`), workspace checks (`cargo check`), clippy linter (`cargo clippy`), and test suite execution (`cargo test`).

---

## 25. Application mail and SMTP

Application mail is enqueued in the same MariaDB transaction as the business change. The web request does not wait for SMTP; a separate worker delivers due messages.

### Configure project and transport

```toml
[capabilities]
email = true
database = true

[network]
allowed_hosts = ["smtp.example.test"]
```

```dotenv
ZELYRA_SMTP_HOST=smtp.example.test
ZELYRA_SMTP_SECURITY=starttls
ZELYRA_SMTP_PORT=587
ZELYRA_SMTP_FROM="Zelyra <noreply@example.test>"
ZELYRA_SMTP_USERNAME=app-user
ZELYRA_SMTP_PASSWORD=replace-with-a-secret
```

The default `implicit_tls` uses port 465; `starttls` uses port 587. Certificate and hostname verification remain active. Optional `ZELYRA_SMTP_CA_CERT` adds a PEM root certificate from a regular file without symlinks, up to 128 KiB. `local_plaintext` is restricted to loopback without credentials. The exact SMTP host must appear in the allowlist. Store credentials in local `.env` or the process environment and keep them out of version control.

### Enqueue mail

Save this example as `main.zyl` in a configured MariaDB project:

```zelyra
pub fn deliver_mail(event_id: String, event_type: String, payload_json: String) -> Bool uses Email {
  if event_type == "zelyra.mail.send" {
    match mail_deliver(event_id, payload_json) {
      Ok(_) => { return true }
      Err(_) => { return false }
    }
  }
  return false
}

fn main() uses Email, Database(write) {
  transaction {
    mail_queue(
      "customer@example.test",
      "Your Zelyra order",
      "Your order has been recorded.",
      "<p>Your order has been recorded.</p>"
    )
  }
}
```

`mail_queue(to, subject, text, html?)` is a statement directly inside `transaction`. The function requires `Email` and `Database(write)`. Header injection and oversized messages are rejected; arguments must be values or side-effect-free string expressions. Rolling back the transaction also rolls back its queued mail. Escape untrusted HTML before enqueueing it.

### Outbox and worker

```bash
zelyra outbox setup main.zyl
zelyra run main.zyl
zelyra outbox list main.zyl --limit 25
zelyra outbox run main.zyl --handler zelyra.mail.send=deliver_mail --once
```

Set up the outbox before running the program. `--once` processes one due entry; without it, the worker processes mapped due events until none remain. `mail_deliver(event_id, payload_json)` returns a `Result`; the handler acknowledges success with `true` and permits retry with `false`. SMTP has a five-second transport timeout and redacted errors.

Delivery is at least once. A worker can fail after SMTP acceptance and before outbox acknowledgement, allowing duplicates. The event ID serves as a stable message ID but does not guarantee exactly-once delivery. The documented API supports text/HTML alternatives through the MariaDB outbox. Named templates, attachments, and a PostgreSQL mail outbox are not established as available APIs. See the [mail reference](https://github.com/sf1976/zelyra/blob/main/docs/mail.en.md).

### Persistent password reset limits

To defend against brute-force and denial-of-service attempts, Zelyra persists password reset request limits in the `password_reset_limits` table. Rate limits remain effective across application process restarts. Expired time windows are purged automatically in batch cycles, while operator diagnostics remain strictly sanitized to avoid leaking email addresses or tokens into logs.

## 26. Tenant-scoped notifications

The in-app center works without SMTP. It uses authenticated tenant membership, a shared notification table, and tenant-scoped CRUD. The active tenant bounds lists, direct access, and the unread count. Expired and archived entries are hidden.

### Complete MariaDB example

Save this example in a MariaDB project with database, web, CRUD, and authentication enabled. Set up its schema, users, password hashes, memberships, and permissions before using it in the browser:

```zelyra
# Tenant-scoped notification inbox starter for MariaDB projects.
# Every generated notification route verifies the authenticated membership.

auth users {
    table: users
    sessions: auth_sessions
    permissions: user_permissions
    memberships: tenant_memberships
}

table users {
    id: Id primary auto
    email: Email required unique
    password_hash: String(255) required
    active: Bool default true
}

table auth_sessions {
    id: Id primary auto
    user: User required
    token_hash: String(64) required unique
    expires_at: Timestamp required
}

table user_permissions {
    id: Id primary auto
    user_id: Id required
    permission: String(100) required
}

table tenant_memberships {
    id: Id primary auto
    user_id: Id required
    tenant_id: Id required
    active: Bool required
}

table notifications {
    id: Id primary auto
    tenant_id: Id required
    title: String(160) required
    message: String(4000) required
    category: String(40) required
    read_at: Timestamp?
    archived_at: Timestamp?
    expires_at: Timestamp?
    created_at: Timestamp default now
}

crud Notification -> notifications {
    title: "Notifications"
    tenant: tenant_id
    unread: read_at
    expires: expires_at
    requires auth
    permits "notifications.view"
    permits create "notifications.manage"
    permits edit "notifications.manage"
    permits delete "notifications.manage"

    soft_delete {
        column: archived_at
    }

    list {
        title
        category
        created_at
        read_at
    }

    search {
        title
        message
    }

    filter {
        category
        read_at
    }

    view {
        fields {
            title
            message
            category
            read_at
            expires_at
        }
    }
}

table notification_preferences {
    id: Id primary auto
    tenant_id: Id required
    email_enabled: Bool default false
    retention_days: Int default 90
    updated_at: Timestamp default now
}

crud NotificationPreference -> notification_preferences {
    title: "Notification preferences"
    tenant: tenant_id
    requires auth
    permits "notifications.manage"
    list {
        email_enabled
        retention_days
        updated_at
    }
    view {
        fields {
            email_enabled
            retention_days
        }
    }
}

fn main() {
}
```

`tenant: tenant_id` binds the resource to the active tenant and its membership. `unread: read_at` provides the unread count; `expires: expires_at` hides expired entries. `soft_delete` archives through `archived_at`. `notifications.view` permits reading, while `notifications.manage` protects administration and changes. A tenant field without authentication/membership configuration does not establish isolation.

### Read state, retention, and mail

`read_at` records the timestamp on notification items. Through recipient scoping (`recipient: user_id`), notifications are restricted to their authenticated recipient. Personal read receipts are confirmed with CSRF protection and atomic audit logging. The preference table stores `email_enabled` and `retention_days`. Expiry is checked on reads; archiving is not physical deletion. The [example](https://github.com/sf1976/zelyra/blob/main/examples/notification_center.zyl) shows the existing scope.

### Local TOTP MFA

For multi-factor authentication, Zelyra supports time-based one-time passwords (TOTP per RFC 6238). Secret keys are encrypted at rest using AES-256-GCM. One-time recovery codes are securely hashed with Argon2id. The implementation defends against replay attacks and race conditions, automatically enforcing a temporary lockout (HTTP 429) after five consecutive failed attempts.

## 27. Language, Runtime, and PDF Mail

This section documents features introduced in Zelyra 1.0.7. The release extends
the language and runtime for testable business logic and adds a local server-side
text-PDF pipeline for views and transactional email attachments.

### Value-producing expressions and record updates

An `if` expression yields the same checked type in both branches. Records can be
functionally copied and updated using `..old`:

```zelyra
fn invoice_label(paid: Bool) -> String {
    return if paid { "Paid" } else { "Open" }
}

struct Customer {
    id: Int
    name: String
    active: Bool
    version: Int
}

fn activate_customer(c: Customer) -> Customer {
    return Customer { ..c, active: true, version: c.version + 1 }
}
```

### Typed lambdas and pipelines

Lambdas are lexical closures. The compiler statically validates callback inputs
and returns; pipelines remain synchronous and deterministic:

```zelyra
fn calculate_tax_totals(amounts: Int[]) -> Int[] {
    return amounts
        |> filter(|a| a > 0)
        |> map(|a| a + (a * 19 / 100))
}
```

### Nested record patterns, match guards, and tuples

`match` supports nested record patterns and boolean guards. Tuples can serve as
first-class return values and destructuring targets:

```zelyra
struct Order { status: String total: Int }

fn inspect_order(order: Order) -> Tuple<String, Bool> {
    return match order {
        Order { status: "paid", total } if total >= 1000 => ("VIP large order", true)
        Order { status: "paid", total }                  => ("Standard order", true)
        _                                                => ("Pending", false)
    }
}
```

### Bounded execution budgets

`zelyra run file.zyl --max-steps N` bounds statement and expression evaluations
across all `parallel` tasks and reports overruns as `E-RUNTIME-001` at the source
expression:

```bash
zelyra run main.zyl --max-steps 5000
```

### Server-side text PDF and transactional outbox email

`render_pdf("InvoicePrint", invoice)` renders a declared view locally into
`Bytes`. Within the same database transaction, `mail_queue` can enqueue a PDF
attachment into MariaDB's outbox:

```zelyra
fn send_invoice(recipient: String, invoice: Invoice) uses Email, Database(write) {
    transaction {
        mail_queue(
            recipient,
            "Invoice " + invoice.number,
            "Your invoice is attached.",
            "invoice-" + invoice.number + ".pdf",
            render_pdf("InvoicePrint", invoice)
        )
    }
}
```

## 28. Systemd Services, Scheduler, and Jobs

Zelyra integrates operating system service management, resident background schedules, and persistent jobs directly into its toolchain, without depending on external cron daemons.

### Linux systemd Service Management

Use `zelyra service` to manage a dedicated Linux systemd user service with an explicit, fixed port:

```bash
# Install systemd user service on a verified port 8080
zelyra service install --port 8080

# Check service status and HTTP listener reachability
zelyra service status

# Restart service (synchronously confirms socket binding)
zelyra service restart

# Change port with atomic verification and rollback
zelyra service change --port 8081

# Stop and remove the service unit
zelyra service remove
```

If the chosen port is occupied, installation fails (`fail-closed`). Zelyra never silently rebinds to an arbitrary port. During `service change`, the old unit and process are preserved until the new listener is verified.

### Resident Scheduler and MariaDB Outbox Jobs

Schedules are declared natively in Zelyra code:

```zelyra
schedule weekly_report {
    cron: "0 8 * * 1"
    timezone: "Europe/Berlin"
    command: generate_reports
}
```

The resident scheduler runs strictly inside the active project systemd service. At every minute boundary, it evaluates declared schedules and enqueues due jobs into the MariaDB outbox. A background worker acquires jobs using renewable leases and bounded retries.

```bash
# Inspect all queued, running, and completed jobs for the project
zelyra schedule jobs main.zyl

# Check resident scheduler status
zelyra schedule status main.zyl

# Export a standard crontab preview for external inspection
zelyra schedule export main.zyl --format cron
```

## 29. Canonical Project Layout and Vertical Slices

Starting in version 1.2.13, Zelyra introduces a validated **Vertical Slice Architecture** designed to separate HTTP transport from pure domain calculations.

### Scaffolding Standard Projects

```bash
zelyra new my-erp --layout standard
```

A standard layout application follows this verified structure:

```text
my-erp/
├── zelyra.toml                 # Project manifest (layout = "standard")
├── schema/                     # Single source of database truth
│   └── tables.zyl              # Table declarations, indices, relationships
├── domain/                     # Vertical feature slices
│   ├── invoices/               # Invoices feature module
│   │   ├── actions.zyl         # Business logic and calculations
│   │   └── views.zyl           # UI templates and components
│   └── customers/              # Customers feature module
│       ├── actions.zyl
│       └── views.zyl
├── routes/                     # Lean HTTP gatekeepers
│   ├── web.zyl                 # Web routes and form handlers
│   └── api.zyl                 # REST / JSON endpoints
├── public/                     # Static files (served directly)
└── assets/                     # Embedded web assets (CSS/JS)
    ├── css/
    └── js/
```

### Namespace Imports and Strict Architecture Linting

Imports resolve cleanly through canonical module roots:

```zelyra
use schema::tables::*
use domain::invoices::actions as invoice_ops
```

Setting `strict_architecture = true` in `zelyra.toml` activates compile-time architecture linting:

> **Direct database mutations inside route handlers (`routes/*.zyl`) are strictly forbidden.**

Mutating queries (`insert`, `update`, `delete`) in route files fail the build with `E-ARCH-001`. Domain logic must reside in `domain/<feature>/actions.zyl`.

```bash
# Check layout migration without moving or modifying files
zelyra migrate-layout --check main.zyl
```

## 30. Extensible CRUD and TableViews

Zelyra 1.2.15 introduces compiler-checked extension regions for generated CRUD views and data tables, allowing deep UI customization while keeping routing, validation, permissions, and CSRF protection centrally enforced.

### Customizing Create and Edit Views

`crud.view.create` and `crud.view.edit` configure page titles, submit labels, and surrounding content blocks:

```zelyra
crud CustomerManagement for Customer {
    view create {
        title: "Register New Customer"
        submit_label: "Create Account"
        help: "Please verify tax ID before submitting."
    }
    view edit {
        title: "Edit Customer Details"
        submit_label: "Save Changes"
    }
}
```

Custom view regions use the checked template context with automatic XSS protection. If validation fails, customized content and field help remain visible alongside error bags.

### TableView Enhancements and Row Actions

Tabular data views (`tableview`) support header/footer additions and contextual row actions:

```zelyra
tableview InvoiceList for Invoice {
    columns: [number, customer_name, total_gross, status]
    row_actions: [
        action "Details" -> "/invoices/show/{id}",
        action "PDF" -> "/invoices/{id}/pdf"
    ]
}
```

Row actions generate clean URLs that respect tenant isolation and permission policies.

## 31. Dual-Tier Execution Pipeline: AST, MIR, and Bytecode VM

Targeting version 1.3.0, Zelyra implements a dual-tier execution pipeline combining fast developer feedback with enterprise runtime performance.

```text
Source Code (.zyl)
      │
      ▼
Lexer & Parser
      │
      ▼
AST with Source Spans
      │
Shared Semantic Checks (Types, Contracts, Scopes)
      │
      ├───────────────────────────────┐
      ▼                               ▼
[1. AST Reference Interpreter]  [2. High-Speed Pipeline]
- Instant hot-reloading         - HIR (Symbol resolution)
- Test oracle                   - Typed MIR (CFG, basic blocks)
- Clear diagnostics             - MIR verifier
                                - Bytecode compiler (format v24)
                                - Bytecode VM (Iterative Rust frames)
```

1. **AST Reference:** Retained as an oracle for interactive development and differential testing.
2. **Typed MIR:** Represents control flow in explicit basic blocks and validates SQL operations (`E-MIR-VERIFY-004`), types, and capability bounds.
3. **Bytecode VM v24:** Executes instructions in iterative native Rust loops with sub-millisecond dispatch and low memory overhead (< 45 MB idle RAM).

Differential tests run against both backends to guarantee parity across results, errors, limits, and host interactions.

## 32. Tenant Isolation and Composite Foreign Keys

In multi-tenant business software, no entity may exist without tenant affiliation, and foreign key relations must never point across tenant boundaries. Zelyra embeds tenant scoping directly into table definitions (`scoped by tenant_id: TenantId`).

### Declaring Multi-Tenant Tables

```zelyra
type TenantId = Id

table customers scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    customer_number: String(50) required unique within tenant
    name: String required
}

table invoices scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    invoice_number: String(50) required unique within tenant
    customer: Customer?
    total_net: Decimal(12, 2) required default 0
    total_gross: Decimal(12, 2) required default 0
}
```

### Database-Level Integrity Guarantees

1. **Composite Primary Keys & Unique Indexes:** Zelyra automatically creates `uq_{table}_tenant_id UNIQUE (tenant_id, id)`.
2. **Tenant-Isolated Foreign Keys:** Relationships between scoped tables (`customer: Customer?`) automatically generate composite foreign keys:
   ```sql
   FOREIGN KEY (tenant_id, customer_id) REFERENCES customers (tenant_id, id) ON DELETE RESTRICT
   ```
   This physically blocks cross-tenant access attacks (such as assigning Tenant A's customer to Tenant B's invoice) at the database layer.
3. **Tenant-Scoped Uniqueness:** Fields marked `unique within tenant` produce `UNIQUE KEY (tenant_id, customer_number)`. Different tenants can use identical numbering schemes without collisions.
4. **Optional Relations & Decimals:** Optional relationships (`Customer?`) generate typed `BIGINT NULL`. `Decimal(p, s)` (e.g. `Decimal(12, 2)`) guarantees exact commercial rounding without IEEE 754 floating-point inaccuracies.

## 33. Table Invariants and Validation Rules

Business logic belongs directly on the data model. Instead of duplicating validation rules in controllers, form requests, and UI forms, Zelyra allows you to declare typed table invariants inside the `table` declaration.

### Declaring Table Invariants

```zelyra
table invoices scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    invoice_date: Date required
    due_date: Date required
    discount_days: Int required default 0

    validate {
        due_date >= invoice_date: "Due date must be on or after the invoice date."
        discount_days >= 0: "Discount days cannot be negative."
    }
}
```

### Enforcement in Forms and APIs

- **Declarative CRUD & Forms:** Prior to committing create or edit writes, Zelyra evaluates all declared invariants atomically. If any condition fails, the write is aborted, and the form renders the message with HTTP status 422.
- **Typed API Input Validation:** APIs can bind request payloads to tables (`validates`). Validation evaluates table invariants and returns standardized HTTP 422 error contracts before executing the handler.

If a validated table has computed columns, do not declare them as API request
inputs. Declare only the source fields. Zelyra recalculates computed values
before table invariants and passes them to the handler as additional typed
parameters after the declared request inputs, in table-column order. Clients
cannot select the computed value. For example, a table with `quantity`,
`unit_price`, and `total: Decimal compute(quantity * unit_price)` can use:

```zelyra
api POST "/invoice-items" {
    handler create_item
    input { quantity: Decimal unit_price: Decimal }
    validates invoice_items
    output Result<Decimal, String>
    errors { 422 ValidationError: String }
}

fn create_item(quantity: Decimal, unit_price: Decimal, total: Decimal) -> Result<Decimal, String> {
    // Persist :total through a parameter-bound SQL statement here.
    return Ok(total)
}
```

Handlers that persist the row must use the injected computed parameter, never
a client-provided value. Missing required source fields remain compile-time
errors; optional and defaulted source fields follow their table declarations.
The same checked input and recomputation path is available to validated API
`PUT` handlers. Their SQL update must bind every computed dependency and output
to the corresponding typed handler parameter. `examples/computed_api.zyl`
contains runnable create and update handlers; `PATCH` is not supported by this
computed-field path yet.

## 34. Lifecycle Guards, Policies, and Master-Detail CRUD

Compliance regulations (e.g., GoBD) require that posted and locked business documents cannot be modified or deleted. Additionally, business documents like invoices and sales orders require parent records and line items to be saved in an atomic transaction.

### Record-Dependent Lifecycle Guards

```zelyra
table invoices scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    invoice_date: Date required
    is_locked: Bool required default false
}

crud Invoice -> invoices {
    title: "Invoices"

    guard edit when is_locked == true {
        error: "Locked invoices cannot be edited (audit compliance protection)."
    }

    guard delete when is_locked == true {
        error: "Locked invoices cannot be deleted."
    }
}
```

- **Guard semantics as a rejection condition:** A guard blocks an operation whenever its condition evaluates to **true**. To protect against unauthorized editing or re-locking, Zelyra evaluates `guard edit when is_locked == true`. A condition like `is_locked == false` would erroneously reject open drafts.
- **Transactional Evaluation:** The guard condition is evaluated after row locks are acquired within the database transaction. Direct POST/DELETE requests cannot bypass this protection.
- **UI Integration:** In `tableview` row action slots and detail views, edit and delete actions are automatically disabled or hidden when the condition matches.

### Interplay with GoBD Compliance, Hashes, and Capabilities

Production business applications require complementary primitives alongside lifecycle guards, provided directly through Zelyra's standard library and capability system:

1. **Cryptographic Checksums (`hash_sha256`):**
   The standard library function `hash_sha256(text: String) -> String` computes the hexadecimal SHA-256 digest of a string, fixing document integrity upon final posting:
   ```zelyra
   fn document_hash(doc_number: String, gross: Decimal) -> String {
       return hash_sha256(doc_number + "#" + gross)
   }
   ```
2. **Deterministic Time (`now()` and `uses Clock`):**
   Functions querying the current system timestamp must declare the `Clock` capability (`fn post_invoice(...) uses Clock`). This guarantees time remains strictly mockable and deterministic across automated tests.
3. **Document PDFs and Outbox Mail (`render_pdf` and `mail_queue`):**
   `render_pdf("InvoicePrint", invoice)` compiles a view into PDF bytes server-side. Within an atomic MariaDB `transaction`, `mail_queue(to, subject, text, filename, pdf_bytes)` enqueues the document and its binary attachment transactionally into the outbox.
4. **Typed CRUD Actions (since 1.2.28):**
   `execute { ... }` accepts compiler-checked expressions and SQL. A guard is evaluated against the persisted, locked row before writes. The web handler uses a single decision transaction for typed SQL and configured audit insertion. MariaDB and PostgreSQL HTTP E2Es cover `hash_sha256`, `now()`, guards, row updates, audit insertion, concurrent requests, and rollback after a later runtime failure. The MariaDB E2E also verifies enqueueing a real PDF in the outbox; it does not start mail delivery.

### Transactional Master-Detail CRUD Forms (`has_many`)

With `has_many`, CRUD resources declare direct 1:N line-item collections:

```zelyra
table invoice_items scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    invoice: Invoice required on_delete cascade
    position: Int required
    description: String(160) required
    total_net: Decimal(12, 2) required
    total_gross: Decimal(12, 2) required

    validate {
        total_gross >= total_net: "Line gross cannot be less than line net."
    }
}

crud Invoice -> invoices {
    title: "Invoices"

    view {
        fields { invoice_date }
        form {
            title: "Invoice"
            submit: "Save invoice"
        }
    }

    has_many items: InvoiceItem {
        foreign_key: invoice_id
        order_by: position asc
        cascade: delete
        aggregates {
            net_amount: sum(total_net)
            tax_amount: sum(total_gross - total_net)
            gross_amount: sum(total_gross)
        }
    }
}
```

1. **Server-Side Rendering:** Child rows are generated server-side in HTML. Rows can be added or removed without requiring client-side JavaScript.
2. **Atomic Transaction:** Header fields, child items, and computed summaries (`aggregates`) are committed together in a single database transaction. An invalid line item rolls back the entire submission.
3. **Automatic Aggregation:** Summary columns such as `net_amount` or `gross_amount` are recalculated server-side from child lines and updated atomically on the parent invoice.

### Multiple Child Collections per CRUD Resource (from Zelyra 1.2.24)

Starting in Zelyra 1.2.24, generated CRUD create and edit forms can manage **multiple direct `has_many` collections** for the same parent record (such as line items and attachments on an invoice within a unified form):

```zelyra
table invoices {
    id: Id primary auto
    reference: String(100) required
    invoice_date: Date required
    net_amount: Decimal(12, 2) required default 0
    gross_amount: Decimal(12, 2) required default 0
}

table invoice_items {
    id: Id primary auto
    invoice: Invoice required on_delete cascade
    position: Int required
    description: String(160) required
    total_net: Decimal(12, 2) required
    total_gross: Decimal(12, 2) required

    validate {
        total_gross >= total_net: "Line gross amount cannot be below its net amount."
    }
}

table invoice_attachments {
    id: Id primary auto
    invoice: Invoice required on_delete cascade
    file_name: String(255) required
    size_bytes: Int required

    validate {
        size_bytes > 0: "Attachment size must be positive."
    }
}

crud Invoice -> invoices {
    title: "Invoices with items and attachments"

    view {
        fields { reference invoice_date }
        form {
            title: "Invoice"
            submit: "Save invoice"
        }
    }

    has_many items: InvoiceItem {
        foreign_key: invoice_id
        order_by: position asc
        cascade: delete
        aggregates {
            net_amount: sum(total_net)
            gross_amount: sum(total_gross)
        }
    }

    has_many attachments: InvoiceAttachment {
        foreign_key: invoice_id
        order_by: file_name asc
        cascade: delete
    }
}
```

- **Independent Sections and Actions:** Each collection appears in its own distinct form section with its own row count and independent row action identifiers (`add_item`, `remove_item`, `add_attachment`, etc.).
- **Independent Row Limits:** The server-side limit of 50 rows applies to each declared collection independently.
- **Atomic Validation and Persistence:** Zelyra validates all collections before committing writes. Parent data, all child rows across every collection, and declared aggregate expressions are saved in a single database transaction. An invalid collection or database error rolls back the entire submission atomically.
- **Child Table Uniqueness:** A child table may only be bound once to a CRUD resource; multiple collections may not ambiguously target the same aggregate destinations.


## 35. The Zelyra Ecosystem: Packages, Modules, and Settings

The Zelyra module and package ecosystem enables clean modularization of applications into reusable, independently testable, and cryptographically verifiable components. It combines offline-first principles with strict typing and compile-time guarantees.

### Foundations and Ecosystem Philosophy

1. **Offline-First and Deterministic:** A Zelyra project builds and verifies completely offline. There are no implicit downloads during compilation (`zelyra check` / `zelyra run`), no uncontrolled network requests, and no arbitrary third-party build scripts.
2. **Integrity and Lockfile:** Every dependency is locked with a cryptographic SHA-256 digest in `zelyra.lock`. Any unauthorized modification to source code or package manifests is caught immediately by the compiler and requires an explicit update (`zelyra package lock --update`).
3. **Cryptographic Trust Model (Ed25519):** For sharing and registries, Zelyra uses signed `.zpkg` source archives. An archive is signed with a publisher's private Ed25519 key. Consumers explicitly record authorized public keys in `zelyra.trust.toml`. Without explicit publisher trust, Zelyra refuses installation.
4. **Compiler Effect Ceilings (`effects`):** A package can declare upper bounds on capabilities that its functions may request (for example, `effects = ["Database(read)"]` or `effects = []` for purely computational libraries). If any function inside the package exceeds this ceiling, the compiler produces an error.

### Initializing a New Package: zelyra module init

A new module package is initialized using the CLI command `zelyra module init`:

```sh
zelyra module init packages/finance_tools --name finance_tools --version 0.1.0
```

The command generates standard scaffolding without network access and without overwriting existing files:

```
packages/finance_tools/
├── zelyra.toml
├── README.md
└── src/
    └── lib.zyl
```

### The Package Manifest: zelyra.toml

The `zelyra.toml` manifest declares the package identity, version compatibility, and effect ceilings:

```toml
[package]
name = "finance_tools"
version = "0.1.0"
entry = "src/lib.zyl"
zelyra = ">=1.1.0, <2.0.0"
effects = ["Database(read)"]

[settings.api_timeout]
type = "Int"
required = false
default = 15
description = "Timeout for external queries in seconds"

[settings.api_token]
type = "String"
required = true
secret = true
description = "Private API key for financial data"
```

- `name`: Package name in lowercase ASCII with digits, `_`, and `-`.
- `version`: Exact SemVer version (e.g. `0.1.0`).
- `entry`: Entry source file, defaulting to `src/lib.zyl` for libraries or `main.zyl` for standalone apps.
- `zelyra`: SemVer constraint for the Zelyra CLI. If the running compiler version falls outside this range, Zelyra rejects compilation upfront.
- `effects`: Optional list of allowed capability names. `[]` enforces pure, side-effect-free functions. Omission allows all capabilities permitted by the host application.

### Writing and Exporting Module Code

Functions, records, and enums intended for external use must be exported with the `pub` keyword:

```zelyra
pub record TaxResult {
    net: Decimal,
    tax: Decimal,
    gross: Decimal,
}

pub fn calculate_tax(net: Decimal, rate: Decimal) -> TaxResult
    requires { net >= 0 && rate >= 0 }
    ensures { result.gross >= result.net }
{
    let tax = (net * rate) / 100
    return TaxResult {
        net: net,
        tax: tax,
        gross: net + tax,
    }
}
```

Declarations without `pub` remain strictly private to the package. Contracts (`requires`, `ensures`) are fully evaluated and enforced by the compiler across package boundaries.

### Typed Module Settings

Zelyra strictly decouples package configuration schemas from application data and secrets:

1. **Schema Declaration:** The schema is defined in the package manifest under `[settings.<field>]`. Supported scalar types are `String` (up to 4 KiB), `Bool`, `Int`, and `Float`. Secrets declare `secret = true` and must not have default values.
2. **Application Configuration in `zelyra.settings.toml`:** Values reside in the main application workspace, decoupled from `zelyra.toml` so package digests remain immutable:
   ```toml
   [package_settings."packages/finance_tools"]
   api_timeout = 30
   api_token = { env = "FINANCE_API_TOKEN" }
   ```
   Secrets are strictly referenced via `{ env = "ENV_VAR" }`. The Zelyra runtime resolves the value from process environment variables and prevents secret leakage into error output or logs.
3. **Retrieval in Zelyra Code:** Package functions read configured values with full type safety using the internal generic function `module_setting<T>`:
   ```zelyra
   fn get_timeout_seconds() -> Int {
       let timeout = module_setting<Int>("api_timeout")
       return timeout
   }
   ```
4. **Management via Web and Studio:**
   - **CLI Validation:** `zelyra module settings check main.zyl` validates types, required fields, and environment variable presence.
   - **Zelyra Studio:** Under *Module Settings*, Studio provides a revision-locked settings editor backed by `.zelyra.settings.lock`.
   - **Runtime Web Page:** When authentication is active and `admin_permission` is defined, `/__zelyra/module-settings` provides a CSRF-protected web management portal for administrators.

### Integrating a Module into a Zelyra Application

To integrate a module into a main application, declare it in the application's `zelyra.toml` under `[dependencies.<alias>]`:

```toml
[package]
name = "my_app"
version = "1.0.0"
entry = "main.zyl"
zelyra = ">=1.2.0"

[dependencies.finance]
path = "packages/finance_tools"
version = "^0.1"
```

In your Zelyra source code, import the package using the `pkg:<alias>` prefix:

```zelyra
import "pkg:finance" as finance

fn main() {
    let result = finance::calculate_tax(200.0, 19.0)
    print(result.gross)
}
```

After adding a dependency, synchronize and verify the lockfile:

```sh
zelyra package lock .
zelyra check main.zyl
```

### Module Inspection, Planning, and Bundling

Zelyra provides dedicated commands to inspect, verify, and bundle modules:

1. **List Modules and Exports (`module list`):**
   ```sh
   zelyra module list main.zyl
   zelyra module list main.zyl --format json
   ```
   Deterministically inspects source paths, import aliases, public exports, checksums, effect ceilings, and settings schemas.
2. **Analyze Dependency Graph (`module plan`):**
   ```sh
   zelyra module plan main.zyl pkg:finance
   ```
   Computes the static source closure for the target module and catches unresolved or circular dependencies before deployment.
3. **Bundle for Portable Deployment (`module bundle`):**
   ```sh
   zelyra module bundle main.zyl pkg:finance --output ./dist/finance-bundle
   ```
   Produces a self-contained source bundle with isolated package dependencies and fresh lockfile, ready for offline execution.

### Signed Packages, Registries, and Transactional Recovery

For cross-team or cross-organization distribution, Zelyra includes cryptographic package tooling:

- **Keypairs and Signatures:**
  ```sh
  zelyra package keygen --publisher financeteam --output financeteam.key
  zelyra package sign packages/finance_tools --publisher financeteam --key financeteam.key --output finance_tools-0.1.0.zpkg
  ```
- **Authorizing Publisher Trust:**
  ```sh
  zelyra package trust add financeteam <public-key-hex>
  zelyra package verify finance_tools-0.1.0.zpkg
  ```
- **Building and Searching Static Registries:**
  ```sh
  zelyra package registry build ./published-packages
  zelyra package search https://packages.example.test/index.json finance
  ```
- **Verified Installation:**
  ```sh
  zelyra package add https://packages.example.test/index.json finance_tools 0.1.0 --as finance
  zelyra package update https://packages.example.test/index.json finance_tools 0.2.0 --as finance
  zelyra package remove finance
  ```
- **Transactional Recovery (`package recover`):**
  All mutating package operations write a local transaction journal. If an operation is interrupted, package commands block until checked:
  ```sh
  zelyra package recover
  zelyra package recover --apply
  ```
  `zelyra package recover --apply` restores manifests and `zelyra.lock` to the previous verified state and cleans up incomplete cache artifacts.

## 36. CSS Design System and UI Styling for CRUD and TableViews

Zelyra applies the same design philosophy to its web frontend as it does to the language and database engines: **built-in safety without runtime bloat**. Rather than requiring heavy JavaScript toolchains, Node.js runtimes, or npm dependencies, Zelyra ships with a compiled, standalone CSS design system (`zelyra-ui.v1.css`) served directly by the built-in HTTP server.

This chapter details the architecture of Zelyra UI, its design tokens and class naming conventions, and provides complete, production-ready CSS recipes for tailoring CRUD pages, TableViews, master-detail forms, and query toolbars.

---

### 1. Architecture of the Zelyra UI Design System

Zelyra UI is built entirely upon standard web specifications and CSS Cascade Layers (`@layer`).

```
┌────────────────────────────────────────────────────────────────────────┐
│                        CSS Cascade Layers                              │
│                                                                        │
│  @layer reset         Browser normalization & box-sizing               │
│       ↓                                                                │
│  @layer base          Root tokens, typography, standard HTML elements  │
│       ↓                                                                │
│  @layer theme         Color schemes (light, dark, system), density     │
│       ↓                                                                │
│  @layer layout        App shell, grids, containers, scrolling regions  │
│       ↓                                                                │
│  @layer components    Cards, buttons, badges, tables, form controls    │
│       ↓                                                                │
│  @layer utilities     Utility helper classes (margins, alignment)      │
│       ↓                                                                │
│  @layer overrides     Zelyra standard views (CRUD, TableViews)         │
│       ↓                                                                │
│  [Project CSS]        Custom zelyra.theme.css (highest precedence)     │
└────────────────────────────────────────────────────────────────────────┘
```

#### The Root Selector and Specificity

All Zelyra UI styles are scoped using the CSS pseudo-class `:where(html[data-z-ui])`:

```css
:where(html[data-z-ui]) .z-table {
  width: 100%;
  border-collapse: collapse;
}
```

The `:where()` pseudo-class reduces its selector's specificity to **0**. Consequently, developers can override any default Zelyra UI styling in their project's `zelyra.theme.css` using standard single-class selectors (such as `.z-table`), completely avoiding `!important` flags or unwieldy specificity battles.

#### Loading Project Stylesheets

Zelyra automatically loads custom styles through two mechanisms:

1. **Automatic Discovery:** If a file named `zelyra.theme.css` or `assets/css/theme.css` exists in your project root, the built-in server injects it into the `<head>` of all rendered pages immediately following the base framework styles.
2. **Environment Variable / CLI Flag:** You can specify an explicit stylesheet path using the `ZELYRA_UI_STYLESHEET=/path/to/custom.css` environment variable or the `--stylesheet` CLI argument.

Per the CSS Cascade Layers specification, any unlayered CSS rule takes precedence over all `@layer` declarations. Thus, rules written directly in `zelyra.theme.css` naturally override framework defaults:

```css
/* In zelyra.theme.css: cleanly overrides framework defaults */
.z-table th {
  background: #1a2234;
  color: #ffffff;
}
```

Alternatively, rules may explicitly target `@layer overrides`:

```css
@layer overrides {
  .z-table th {
    background: #1a2234;
    color: #ffffff;
  }
}
```

---

### 2. Design Tokens and CSS Custom Properties

Zelyra UI declares a typed system of CSS custom properties on `html[data-z-ui]`.

#### Color Tokens

| Token | Default (Light) | Purpose / Usage |
| :--- | :--- | :--- |
| `--z-color-canvas` | `#f4f6fa` | Document canvas background color |
| `--z-color-surface` | `#ffffff` | Background for cards, forms, tables, and dialogs |
| `--z-color-surface-raised` | `#ffffff` | Elevated areas (table headers, popovers, dropdowns) |
| `--z-color-text` | `#182230` | Primary text and heading color |
| `--z-color-text-muted` | `#526174` | Subdued text for helper hints, metadata, and labels |
| `--z-color-border` | `#d8e0e9` | Border color for tables, inputs, and container cards |
| `--z-color-primary` | `#3157a4` | Accent color for buttons, active tabs, focus rings |
| `--z-color-on-primary` | `#ffffff` | Contrasting text color on primary backgrounds |
| `--z-color-primary-hover`| `#274889` | Hover state for primary buttons |
| `--z-color-primary-soft` | `#eaf0fb` | Soft background for row hovers and active states |
| `--z-color-success` | `#16734c` | Success badges, paid invoices, confirmation banners |
| `--z-color-success-soft`| `#e9f6ef` | Soft background for success badges and alerts |
| `--z-color-warning` | `#795000` | Warning badges, draft records, payment reminders |
| `--z-color-warning-soft`| `#fff4d8` | Soft background for warning alerts |
| `--z-color-danger` | `#a12e39` | Errors, cancellation notices, delete buttons |
| `--z-color-danger-soft` | `#fff0f0` | Soft background for field error states and summaries |
| `--z-color-focus` | `#275fc5` | Accessible focus indicator ring (`:focus-visible`) |
| `--z-color-link` | `#244f9a` | Hyperlinks and table detail links |

#### Dimensions, Spacing, and Radii

```css
:where(html[data-z-ui]) {
  /* Typography */
  --z-font-sans: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  --z-font-mono: ui-monospace, "Cascadia Code", "Segoe UI Mono", monospace;
  --z-font-size: 1rem;
  --z-line-height: 1.5;

  /* Controls and Canvas Width */
  --z-control-height: 2.75rem; /* Standard input & button height */
  --z-content-width: 76rem;    /* Maximum page content width */

  /* Spacing Scale */
  --z-space-1: 0.25rem;  /* 4px */
  --z-space-2: 0.5rem;   /* 8px */
  --z-space-3: 0.75rem;  /* 12px */
  --z-space-4: 1rem;     /* 16px */
  --z-space-5: 1.25rem;  /* 20px */
  --z-space-6: 1.5rem;   /* 24px */
  --z-space-8: 2rem;     /* 32px */

  /* Border Radii */
  --z-radius-sm: 0.35rem;  /* Inputs, buttons */
  --z-radius-md: 0.65rem;  /* Cards, dialogs, badges */
  --z-radius-lg: 1rem;     /* Shell containers */
  --z-radius-pill: 999px;  /* Pill badges, switch tracks */

  /* Z-Index Hierarchy */
  --z-layer-sticky: 10;
  --z-layer-dropdown: 20;
  --z-layer-dialog: 100;
  --z-layer-toast: 120;
}
```

#### Compact Density (`compact`)

In data-dense applications such as accounting portals or ERP backoffices, compact spacing minimizes unnecessary scrolling. Zelyra UI supports this via the `data-z-density="compact"` attribute:

```html
<html data-z-ui data-z-density="compact">
```

In compact mode, control heights shrink from `2.75rem` to `2.25rem`, and cell paddings are tightened across all form controls and table views. You can also scope compact density to a specific table container:

```html
<div class="z-table-scroll" data-z-density="compact">
  <table class="z-table">...</table>
</div>
```

---

### 3. Styling and Optimizing TableViews

Table views (`tableview`) represent core workhorses in business software, displaying relational query results with filtering, sorting, pagination, and inline actions.

#### Standard HTML Hierarchy

A standard Zelyra TableView generates the following markup:

```html
<div class="z-generated-page">
  <header class="z-page-header">
    <div class="z-page-header__copy">
      <h1 class="z-page-header__title">Invoices</h1>
      <p class="z-page-header__description">All recorded invoices for the active tenant.</p>
    </div>
    <div class="z-page-header__actions">
      <a class="z-button z-button--primary" href="/invoices/new">+ New Invoice</a>
    </div>
  </header>

  <form class="zelyra-query-controls" method="get">
    <fieldset>
      <legend>Search & Filters</legend>
      <div class="z-query-controls__fields">
        <!-- Filter controls -->
      </div>
    </fieldset>
  </form>

  <div class="z-table-scroll">
    <table class="z-table">
      <thead>
        <tr>
          <th>Invoice #</th>
          <th>Customer</th>
          <th>Date</th>
          <th>Status</th>
          <th style="text-align: right;">Gross Amount</th>
          <th class="z-table__actions-head">Actions</th>
        </tr>
      </thead>
      <tbody>
        <tr>
          <td><strong>INV-2026-0042</strong></td>
          <td>Acme Corp</td>
          <td>2026-10-19</td>
          <td><span class="z-badge z-badge--success">Paid</span></td>
          <td style="text-align: right;">$1,428.00</td>
          <td class="z-table__actions">
            <a class="z-button z-button--sm" href="/invoices/42">View</a>
            <a class="z-button z-button--sm" href="/invoices/42/edit">Edit</a>
          </td>
        </tr>
      </tbody>
    </table>
  </div>

  <nav class="z-pagination" aria-label="Pagination">
    <div class="z-pagination__summary">Showing 1 to 25 of 148 entries</div>
    <div class="z-pagination__controls">...</div>
  </nav>
</div>
```

#### Sticky Table Headers

When working with long result sets, sticky headers prevent the loss of column context:

```css
/* In zelyra.theme.css: Sticky Table Header */
.z-table-scroll {
  max-height: 70vh;
  overflow-y: auto;
  position: relative;
}

.z-table thead th {
  position: sticky;
  top: 0;
  z-index: var(--z-layer-sticky);
  background: var(--z-color-surface-raised);
  border-bottom: 2px solid var(--z-color-border);
  box-shadow: 0 2px 4px rgb(0 0 0 / 6%);
}
```

#### Zebra Striping and Interactive Row Hover

Zebra striping assists in scanning across wide rows:

```css
/* Zebra striping for even rows */
.z-table tbody tr:nth-child(even) {
  background-color: rgb(0 0 0 / 2%);
}

[data-z-theme="dark"] .z-table tbody tr:nth-child(even) {
  background-color: rgb(255 255 255 / 2.5%);
}

/* Distinct row hover */
.z-table tbody tr:hover {
  background-color: var(--z-color-primary-soft);
  transition: background-color var(--z-motion-fast) ease-in-out;
}
```

#### Right-Aligned Tabular Numerals (Tabular Nums)

Financial amounts and decimal numbers must always be right-aligned so decimal points align vertically. Enabling `tabular-nums` prevents variable-width digits (such as `1`) from disrupting alignment:

```css
/* Exact numeric alignment for currency and quantity columns */
.z-table td.col-number,
.z-table td.col-currency,
.z-table th.col-number,
.z-table th.col-currency {
  text-align: right;
  font-variant-numeric: tabular-nums;
  font-feature-settings: "tnum" 1;
}

/* Subtly emphasize monetary amounts */
.z-table td.col-currency {
  font-weight: 600;
  letter-spacing: -0.01em;
}
```

#### Styling Row Actions (`.z-table__actions`)

Row action columns group viewing, editing, and deletion triggers:

```css
/* Compact, neatly aligned row action buttons */
.z-table td.z-table__actions {
  white-space: nowrap;
  text-align: right;
  vertical-align: middle;
}

.z-table td.z-table__actions .z-button,
.z-table td.z-table__actions a.z-button {
  min-height: 1.85rem;
  padding: 0.2rem 0.6rem;
  font-size: 0.82rem;
  font-weight: 500;
  border-radius: var(--z-radius-sm);
}

.z-table td.z-table__actions .z-button:hover {
  transform: translateY(-1px);
}
```

---

### 4. Search, Filter, and Query Tools

Query toolbars rendered above TableViews use `.zelyra-query-controls` and `.z-query-controls__fields`:

```css
/* Polished query filter bar */
.zelyra-query-controls {
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-md);
  background: var(--z-color-surface-raised);
  padding: 1rem 1.25rem;
  margin-bottom: 1.5rem;
}

.zelyra-query-controls legend {
  font-size: 0.88rem;
  font-weight: 700;
  color: var(--z-color-text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding: 0 0.4rem;
}

.z-query-controls__fields {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(100%, 14rem), 1fr)) auto;
  align-items: end;
  gap: 1rem;
}

/* Filter inputs */
.zelyra-query-controls .z-input,
.zelyra-query-controls .z-select {
  background: var(--z-color-surface);
  border-color: var(--z-color-border);
  box-shadow: inset 0 1px 2px rgb(0 0 0 / 4%);
}

.zelyra-query-controls .z-input:focus,
.zelyra-query-controls .z-select:focus {
  border-color: var(--z-color-primary);
  outline: 3px solid var(--z-color-primary-soft);
}
```

---

### 5. Pagination and Empty States

#### Pagination (`.z-pagination`)

```css
.z-pagination {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding-top: 1.25rem;
  border-top: 1px solid var(--z-color-border);
}

.z-pagination__summary {
  font-size: 0.88rem;
  color: var(--z-color-text-muted);
}

.z-pagination__controls {
  display: inline-flex;
  gap: 0.35rem;
}

.z-pagination__controls a,
.z-pagination__controls span {
  display: inline-flex;
  min-width: 2.25rem;
  height: 2.25rem;
  align-items: center;
  justify-content: center;
  padding: 0 0.5rem;
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-sm);
  background: var(--z-color-surface);
  color: var(--z-color-text);
  font-size: 0.88rem;
  text-decoration: none;
}

.z-pagination__controls a:hover {
  background: var(--z-color-primary-soft);
  border-color: var(--z-color-primary);
}

.z-pagination__controls [aria-current="page"] {
  background: var(--z-color-primary);
  color: var(--z-color-on-primary);
  border-color: var(--z-color-primary);
  font-weight: 700;
}
```

#### Empty States (`.z-empty-state`)

When an inquiry yields no records, Zelyra renders the `.z-empty-state` container:

```css
.z-empty-state {
  text-align: center;
  padding: 3.5rem 1.5rem;
  background: var(--z-color-surface);
  border: 1px dashed var(--z-color-border);
  border-radius: var(--z-radius-md);
  margin-block: 1rem;
}

.z-empty-state h3 {
  margin: 0 0 0.5rem;
  font-size: 1.2rem;
  color: var(--z-color-text);
}

.z-empty-state p {
  max-width: 30rem;
  margin: 0 auto 1.5rem;
  color: var(--z-color-text-muted);
  font-size: 0.95rem;
  line-height: 1.5;
}

.z-empty-state__actions {
  display: flex;
  justify-content: center;
  gap: 0.75rem;
}
```

---

### 6. Styling CRUD and Master-Detail Forms

Compiler-generated create and edit views (`crud.view.create`, `crud.view.edit`) are encapsulated inside the `.zelyra-crud-form-card` container.

#### Responsive Two-Column Grid

```css
/* Outer form card */
.zelyra-crud-form-card {
  max-width: 68rem;
  margin: 1.5rem auto 3rem;
  padding: 2rem;
  background: var(--z-color-surface);
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-lg);
  box-shadow: var(--z-shadow-sm);
}

/* Two-column layout */
.zelyra-crud-form-card form {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 1.25rem 1.5rem;
}

/* Spanning full-width fields */
.zelyra-crud-form-card .z-field--full,
.zelyra-crud-form-card .z-fieldset,
.zelyra-crud-form-card .z-form-actions {
  grid-column: 1 / -1;
}

/* Single column on mobile devices */
@media (max-width: 768px) {
  .zelyra-crud-form-card form {
    grid-template-columns: 1fr;
  }
}
```

#### Fields, Labels, and Hints

```css
.z-field {
  display: grid;
  gap: 0.4rem;
  min-width: 0;
}

.z-field__label {
  font-size: 0.92rem;
  font-weight: 650;
  color: var(--z-color-text);
}

/* Required asterisk indicator */
.z-field__label[data-required="true"]::after {
  content: " *";
  color: var(--z-color-danger);
}

.z-field__hint {
  font-size: 0.82rem;
  color: var(--z-color-text-muted);
}

.z-input,
.z-select,
.z-textarea {
  min-height: var(--z-control-height);
  padding: 0.55rem 0.8rem;
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-sm);
  background: var(--z-color-surface);
  color: var(--z-color-text);
  transition: border-color var(--z-motion-fast), box-shadow var(--z-motion-fast);
}

.z-input:focus,
.z-select:focus,
.z-textarea:focus {
  border-color: var(--z-color-focus);
  box-shadow: 0 0 0 3px var(--z-color-primary-soft);
  outline: none;
}
```

#### Validation Errors and Invariant Indicators (`[aria-invalid="true"]`)

When a typed invariant or field contract fails (e.g., `total_gross >= total_net`), Zelyra applies `aria-invalid="true"`:

```css
/* Signal borders for invalid fields */
.z-input[aria-invalid="true"],
.z-select[aria-invalid="true"],
.z-textarea[aria-invalid="true"] {
  border: 2px solid var(--z-color-danger);
  background-color: var(--z-color-danger-soft);
}

.z-field__error {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--z-color-danger);
}

/* Form error summary */
.z-error-summary {
  margin-bottom: 1.5rem;
  padding: 1.25rem;
  border: 1px solid var(--z-color-danger);
  border-left-width: 6px;
  border-radius: var(--z-radius-md);
  background: var(--z-color-danger-soft);
  color: var(--z-color-danger);
}
```

#### Master-Detail Child Tables (`has_many`) and Aggregate Summaries

In transactional master-detail forms declaring `has_many` collections (such as line items), child grids are integrated cleanly:

```css
/* Child collection table inside form */
.z-fieldset.z-master-detail-items {
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-md);
  padding: 1.25rem;
  background: var(--z-color-surface-raised);
}

.z-master-detail-items table {
  width: 100%;
  border-collapse: collapse;
}

.z-master-detail-items th,
.z-master-detail-items td {
  padding: 0.6rem 0.5rem;
  border-bottom: 1px solid var(--z-color-border);
}

/* Compact inputs in line items table */
.z-master-detail-items input.z-input {
  min-height: 2.15rem;
  padding: 0.25rem 0.5rem;
  font-size: 0.9rem;
}

/* Aggregate totals box (net, tax, gross) */
.z-form-summary-card {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  max-width: 22rem;
  margin-left: auto;
  margin-top: 1rem;
  padding: 1.25rem;
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-md);
  background: var(--z-color-surface);
}

.z-form-summary-card__row {
  display: flex;
  justify-content: space-between;
  font-size: 0.95rem;
}

.z-form-summary-card__row--total {
  border-top: 2px solid var(--z-color-border);
  padding-top: 0.65rem;
  margin-top: 0.25rem;
  font-size: 1.15rem;
  font-weight: 750;
  color: var(--z-color-primary);
}
```

---

### 7. Modal Dialogs and Status Badges

#### Native Dialogs (`.z-dialog`)

Zelyra uses standard HTML5 `<dialog>` elements with built-in backdrop support:

```css
.z-dialog {
  width: min(38rem, calc(100vw - 2rem));
  max-height: min(90vh, 48rem);
  padding: 0;
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-lg);
  background: var(--z-color-surface);
  color: var(--z-color-text);
  box-shadow: var(--z-shadow-md);
  overflow: hidden;
}

/* Dark backdrop with blur */
.z-dialog::backdrop {
  background: rgb(10 20 35 / 65%);
  backdrop-filter: blur(3px);
}

.z-dialog__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1.1rem 1.5rem;
  border-bottom: 1px solid var(--z-color-border);
  background: var(--z-color-surface-raised);
}

.z-dialog__body {
  padding: 1.5rem;
  overflow-y: auto;
}

.z-dialog__footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  padding: 1rem 1.5rem;
  border-top: 1px solid var(--z-color-border);
  background: var(--z-color-surface-raised);
}
```

#### Status Badges (`.z-badge`)

```css
.z-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.25rem 0.65rem;
  font-size: 0.8rem;
  font-weight: 650;
  line-height: 1;
  border-radius: var(--z-radius-pill);
  border: 1px solid transparent;
  white-space: nowrap;
}

/* Badges for status flags */
.z-badge--success {
  background: var(--z-color-success-soft);
  color: var(--z-color-success);
  border-color: rgb(22 115 76 / 20%);
}

.z-badge--warning {
  background: var(--z-color-warning-soft);
  color: var(--z-color-warning);
  border-color: rgb(121 80 0 / 20%);
}

.z-badge--danger {
  background: var(--z-color-danger-soft);
  color: var(--z-color-danger);
  border-color: rgb(161 46 57 / 20%);
}

.z-badge--neutral {
  background: var(--z-color-canvas);
  color: var(--z-color-text-muted);
  border-color: var(--z-color-border);
}
```

---

### 8. Complete Real-World CSS Recipes for zelyra.theme.css

The following recipes are ready to copy directly into your project's `zelyra.theme.css`.

#### Recipe 1: Modern Dark Theme with Accent Colors

A rich, eye-friendly dark theme with cyan and indigo accents for administration dashboards:

```css
/* ==========================================================================
   Recipe 1: Modern Dark Theme for zelyra.theme.css
   ========================================================================== */
:where(html[data-z-ui][data-z-theme="dark"]),
:where(html[data-z-ui]:not([data-z-theme])) {
  /* Slate base palette */
  --z-color-canvas: #0b0f17;
  --z-color-surface: #121824;
  --z-color-surface-raised: #1a2233;
  --z-color-border: #28354b;

  /* Typography */
  --z-color-text: #f1f5f9;
  --z-color-text-muted: #94a3b8;

  /* Radiant Cyan/Indigo Primary Accent */
  --z-color-primary: #38bdf8;
  --z-color-on-primary: #082f49;
  --z-color-primary-hover: #7dd3fc;
  --z-color-primary-soft: #0c2b45;

  /* Status Colors */
  --z-color-success: #4ade80;
  --z-color-success-soft: #0e3720;
  --z-color-warning: #facc15;
  --z-color-warning-soft: #3d330c;
  --z-color-danger: #f87171;
  --z-color-danger-soft: #3d1417;

  /* Shadows */
  --z-shadow-sm: 0 1px 3px rgb(0 0 0 / 50%);
  --z-shadow-md: 0 12px 32px rgb(0 0 0 / 70%);
}

/* Distinct Table Header in Dark Mode */
.z-table thead th {
  background: #172030;
  color: #38bdf8;
  border-bottom: 2px solid #28354b;
}

/* Smooth Row Hover */
.z-table tbody tr:hover {
  background-color: #17243b;
}
```

#### Recipe 2: Compact Financial and Accounting Table

Designed for financial lists and accounting ledgers requiring high data density, sticky headers, and clear monetary alignment:

```css
/* ==========================================================================
   Recipe 2: Financial and Accounting Table with Sticky Header
   ========================================================================== */
.z-table-scroll {
  max-height: 72vh;
  overflow: auto;
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-md);
  box-shadow: 0 4px 12px rgb(0 0 0 / 4%);
}

.z-table {
  font-size: 0.9rem;
  border-collapse: separate;
  border-spacing: 0;
}

/* Sticky Header */
.z-table thead th {
  position: sticky;
  top: 0;
  z-index: 10;
  background: var(--z-color-surface-raised);
  border-bottom: 2px solid var(--z-color-primary);
  padding: 0.65rem 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  font-size: 0.78rem;
  letter-spacing: 0.04em;
  color: var(--z-color-text);
}

/* Compact Rows */
.z-table td {
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--z-color-border);
  vertical-align: middle;
}

/* Subtle Striping */
.z-table tbody tr:nth-child(even) td {
  background-color: rgb(0 0 0 / 1.5%);
}

/* Right-aligned numeric columns with tabular figures */
.z-table th:nth-child(5),
.z-table td:nth-child(5),
.z-table th:nth-child(6),
.z-table td:nth-child(6) {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.z-table td:nth-child(5) {
  font-weight: 700;
  color: var(--z-color-text);
}

/* Compact Row Actions */
.z-table td:last-child {
  text-align: right;
  white-space: nowrap;
}

.z-table td:last-child .z-button {
  min-height: 1.85rem;
  padding: 0.15rem 0.55rem;
  font-size: 0.8rem;
}
```

#### Recipe 3: Master-Detail Invoice Form with Card Design

Provides an elevated, modern card layout for master-detail forms with partitioned header metadata, a line-item grid, and a summary totals card:

```css
/* ==========================================================================
   Recipe 3: Master-Detail Invoice Form Card Design
   ========================================================================== */
.zelyra-crud-form-card {
  max-width: 72rem;
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-lg);
  padding: 2.25rem;
  background: var(--z-color-surface);
  box-shadow: 0 10px 30px rgb(0 0 0 / 5%);
}

/* Form Header */
.zelyra-crud-form-card > header {
  margin-bottom: 2rem;
  padding-bottom: 1.25rem;
  border-bottom: 1px solid var(--z-color-border);
}

.zelyra-crud-form-card > header h1 {
  font-size: 1.6rem;
  margin: 0 0 0.4rem;
  color: var(--z-color-text);
}

/* Master Header Partition */
.z-form-master-section {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 1.25rem;
  padding: 1.5rem;
  background: var(--z-color-surface-raised);
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-md);
  margin-bottom: 2rem;
}

@media (max-width: 900px) {
  .z-form-master-section {
    grid-template-columns: 1fr;
  }
}

/* Line-Item Grid */
.z-form-items-table {
  width: 100%;
  border-collapse: collapse;
  margin-block: 1rem 1.5rem;
}

.z-form-items-table th {
  padding: 0.65rem;
  border-bottom: 2px solid var(--z-color-border);
  font-weight: 700;
  font-size: 0.85rem;
  color: var(--z-color-text-muted);
}

.z-form-items-table td {
  padding: 0.5rem;
  border-bottom: 1px solid var(--z-color-border);
}

/* Highlighted Totals Card */
.z-invoice-totals-box {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 0.6rem 2rem;
  width: min(100%, 24rem);
  margin-left: auto;
  padding: 1.25rem 1.5rem;
  background: var(--z-color-surface-raised);
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-md);
}

.z-invoice-totals-box dt {
  color: var(--z-color-text-muted);
  font-size: 0.95rem;
}

.z-invoice-totals-box dd {
  margin: 0;
  font-weight: 650;
  font-variant-numeric: tabular-nums;
  text-align: right;
}

.z-invoice-totals-box dt.total-gross,
.z-invoice-totals-box dd.total-gross {
  border-top: 2px solid var(--z-color-border);
  padding-top: 0.75rem;
  margin-top: 0.25rem;
  font-size: 1.25rem;
  font-weight: 800;
  color: var(--z-color-primary);
}

/* Form Actions */
.z-form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 1rem;
  margin-top: 2.5rem;
  padding-top: 1.5rem;
  border-top: 1px solid var(--z-color-border);
}

.z-form-actions .z-button--primary {
  min-width: 11rem;
  font-size: 1rem;
}
```

## FileRef attachments in generated CRUD (1.2.33)

Declare an opaque `FileRef` column with its upload policy to make it available
as a generated CRUD upload field:

```zelyra
table contracts {
    id: Id primary auto
    title: String(200) required
    signed_pdf: FileRef? attachment {
        max_size: "20MB"
        allowed_types: ["application/pdf"]
    }
}

crud Contract -> contracts {}
```

The full compiler-checked example is `examples/file_ref_crud.zyl`.

The attachment block uses the same checked size and MIME constraints as
table-backed `File` form fields. Generated create/edit pages render a native
file input. An omitted file on edit keeps the existing reference; a new upload
uses the existing private, integrity-checked storage and is published only
through the normal CSRF-protected CRUD action. Detail and list pages show a
download link rather than the internal FileRef. Downloads are addressed as
`/contracts/{id}/files/signed_pdf` and pass through the CRUD permission,
record, tenant and recipient checks. Raw FileRef values are not accepted as
ordinary text inputs. FileRef columns cannot be used for CRUD search or
filtering.

Supported MIME values currently include PDF, PNG and JPEG. The size limit is
bounded by the existing upload policy (at most 100 MiB per field and the
existing per-request aggregate limit). Projects using ordinary `FileRef`
columns without generated CRUD can continue to use explicit checked upload
forms.
