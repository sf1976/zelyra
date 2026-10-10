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
./install.sh --release v0.3.0
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
.\install.ps1 -Release v0.3.0
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

🧪 **Unreleased 0.4 development branch only:** the MariaDB client receives a
connection timeout, and native runtime queries receive a MariaDB statement
timeout. Override them through the process environment:

~~~bash
export ZELYRA_DB_CONNECT_TIMEOUT_SECS=10
export ZELYRA_DB_QUERY_TIMEOUT_SECS=30
~~~

Connection establishment defaults to 10 seconds and accepts integers from 1
to 300. The statement limit defaults to 30 seconds and accepts 1 to 3600.
Invalid values are rejected without echoing the supplied value. MariaDB itself
enforces the statement limit; it is not a general deadline for transferring a
very large result set. Schema changes performed by `db apply` do not receive
this statement limit yet, so a cancelled DDL operation is never described as
safely rolled back.

The CLI also sets `--skip-reconnect`: the MariaDB client must not silently
reconnect after a lost connection and automatically replay a statement. Zelyra
does not implement automatic retries or a connection pool here; each database
operation still starts a MariaDB client process. The published `0.3.0` release
does not yet support these timeout options.

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

MariaDB records each `db apply` plan and completed DDL step in
`_zelyra_schema_history`. Inspect it after a deployment or interruption:

~~~bash
zelyra db history src/main.zyl
zelyra db history src/main.zyl --format=json
~~~

If a process stopped between DDL steps, inspect the live schema, generate a
fresh plan, review it, and apply it. The history shows the last checkpoint; it
cannot determine whether a statement interrupted while running partially
changed the schema. The reserved history table is omitted from normal schema
inspection.

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

🧪 Edit forms whose name ends in `Edit` now carry a signed snapshot of their
displayed values. On submit, Zelyra locks the row in the same MariaDB
transaction as the form action and audit entries. If a value changed since the
form loaded, it returns `409 Conflict` and does not run the action. This guards
the generated edit path against stale submissions; it does not cover writes
that bypass the form action or establish a general transaction contract for
the application.

Domain actions remain POST-only, parameterized, and protected:

~~~zelyra
crud Customer -> customers {
    action deactivate {
        label: "Deactivate customer"
        confirm: "Really deactivate this customer?"
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

### Optional local TOTP MFA (0.9.0 work in progress)

The current 0.9.0 branch adds opt-in TOTP for MariaDB-backed authentication.
Declare both dedicated MFA tables and add a non-null `mfa_verified` boolean to
the persistent session table. The account's factor row is keyed by `user_id`;
recovery-code rows have their own primary `id`:

~~~zelyra
auth users {
    table: users
    sessions: auth_sessions
    audit: auth_audit_log
    mfa: user_mfa
    mfa_recovery: user_mfa_recovery
}

table auth_sessions {
    id: Id primary auto
    user: User required
    token_hash: String(64) required unique
    expires_at: Timestamp required
    mfa_verified: Bool default false
}

table user_mfa {
    user_id: Int primary
    secret_ciphertext: String(1024) required
    enabled_at: Timestamp?
    last_totp_step: Int?
    failed_attempts: Int default 0
    locked_until: Timestamp?
    enrollment_expires_at: Timestamp?
}

table user_mfa_recovery {
    id: Id primary auto
    user_id: Int required
    code_hash: String(255) required
    used_at: Timestamp?
}

table auth_audit_log {
    id: Id primary auto
    actor_user_id: Int?
    event: String(100) required
    target_user_id: Int?
    details: String(1000) required
    created_at: Timestamp default now
}
~~~

Set `ZELYRA_MFA_ENCRYPTION_KEY` in the server process environment to 64
hexadecimal characters from an operator-managed secret source (for example
`openssl rand -hex 32`). When loading it from the project `.env` on a Unix
shell, export the file before starting the server with `set -a; . ./.env;
set +a`. Keep the key out of source control and backups that are accessible to
the application database. Losing it makes enrolled factors unreadable;
restoring a database backup therefore also requires restoring the matching
key. MFA requires persistent MariaDB sessions and an audit table for login and
factor-change events. Password-only sessions remain unverified until a
valid TOTP or unused recovery code is accepted. Enrollment, disable, and code
replacement require a password and a recent TOTP code. Recovery codes are
shown once and only their Argon2 hashes are stored.

Emergency recovery for a user who has lost both authenticator and recovery
codes is an operator database procedure, not an application screen. Record the
approval and affected account, then use the configured table names and numeric
user ID in a maintenance window:

~~~sql
START TRANSACTION;
DELETE FROM user_mfa_recovery WHERE user_id = 123;
DELETE FROM user_mfa WHERE user_id = 123;
DELETE FROM auth_sessions WHERE user_id = 123;
COMMIT;
~~~

This removes the factor and recovery verifiers and revokes that account's
sessions; the account can then log in with its password and enroll again. If
the encryption key is lost, apply the same removal to every affected account.
Preserve an operator audit record. Do not copy or export encrypted factors as a
recovery method.

This branch is still under implementation. Integration coverage, a rehearsal
of the operator recovery procedure, and independent security review remain
release gates; do not treat this section as a production security guarantee.

Login throttling can be tuned on the `auth` definition with
`login_rate_limit: attempts per seconds` and `login_block_seconds: seconds`.
The defaults are `5 per 900` and `60`. The in-memory limiter is process-local
and resets on restart; it does not replace a deployment-level distributed
rate limiter. Its table is capped at 4,096 keys; new keys are rejected while
the table is full, until expired windows are cleaned up.

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

An API can declare version and deprecation metadata and a bounded per-client
quota:

~~~zelyra
api GET "/api/v1/customers" {
    version "v1"
    deprecated
    rate_limit 100 per 60
    output Customer[]
}
~~~

Responses expose the version and deprecation marker through
`X-Zelyra-API-Version` and `X-Zelyra-API-Deprecated`; `zelyra doc --openapi`
includes the same metadata. The quota is tracked per route and TCP client IP
in the current process. An exhausted quota returns HTTP 429 and `Retry-After`.
Limits allow 1 to 1,000,000 requests per 1 to 86,400 seconds. The server does
not trust `X-Forwarded-For`, so clients behind one reverse proxy share its
quota. Quotas are memory-only and reset when the server restarts.
The table is capped at 4,096 client/route pairs; while full, new pairs receive
HTTP 429 until a window expires.

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
Database Database(read) Database(write) Network FileSystem Environment Process Clock Random Console
~~~

For function-level SQL, `Database(read)` allows checked `SELECT` statements;
`Database(write)` allows checked `INSERT`, `UPDATE`, and `DELETE` statements.
Unclassified SQL requires the broad `Database` capability. The broad form
remains supported for existing projects and satisfies either scoped effect.
Project settings can grant the scoped effects independently:

~~~zelyra
fn list_machines() -> Machine[] uses Database(read) {
    return sql<Machine[]> {
        SELECT id, number, name FROM machines
    }
}
~~~

~~~toml
[capabilities]
database_read = true
database_write = false
~~~

These are compiler/runtime source grants, not MariaDB account permissions.
Framework-managed CRUD, forms, authentication, and page data still require
`Database`.

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
version = "0.1.50"
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

### Complete `.env` reference for the current code

| Variable | Default in generated project | Usage | Secret |
|---|---:|---|---|
| `ZELYRA_WEB_PORT` | `3000` | Port of internal web server inside container | no |
| `ZELYRA_HOST_PORT` | `3000` (or auto-selected free port) | Locally published web port | no |
| `ZELYRA_DB_HOST_PORT` | `3306` (or auto-selected free port) | Locally published MariaDB port | no |
| `ZELYRA_DB_CONNECT_TIMEOUT_SECS` | 0.4 development branch: `10` | MariaDB connection timeout in seconds; valid `1`–`300` | no |
| `ZELYRA_DB_QUERY_TIMEOUT_SECS` | 0.4 development branch: `30` | MariaDB runtime statement limit in seconds; valid `1`–`3600`; not a pool or result-transfer limit | no |
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
| Email | external SMTP/mail crates | no integrated email construct | do not invent `uses Email` | ❌ |
| Jobs | external job/queue systems | no background-job construct | no stable job syntax | ❌ |
| Audit | external logging/audit crates | audit table, CLI inspection, and optional hash chain | tied to auth/CRUD and still experimental | 🧪 |
| Deployment | Cargo, containers, CI, and infrastructure are free choices | generated Docker/Compose template exists | template is a development start, not a production platform | 🧪 |
| Production maturity | widely used in production | Zelyra compiler 0.3.0 is experimental | maturity and ecosystem are not comparable | 🧪 |
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
| Comments | `// comment` | `// comment`, `/* ... */` | current Zelyra lexer has line comments | ✅ |
| Newlines | usually separators; continuation after operators | usually whitespace | parser rules are independent | ✅ |
| Semicolons | accepted as statement separators, not required | often statement separators | Zelyra is not semicolon-dependent | ✅ |
| Blocks | `{ ... }` | `{ ... }` | braces determine blocks in both | ✅ |
| Indentation | readability, not block semantics | readability, not block semantics | whitespace is not Python-style structure | ✅ |
| Error handling | `Result<T, E>`, `Some`/`None` | `Result<T, E>`, `?`, `panic!` | Zelyra has no Rust `?` operator | ✅ |
| String interpolation | HTML bodies may use `{name}` | `format!("{name}")` or `println!("{}", name)` | no general Zelyra string interpolation | 🧪 |
| Modules | `pub fn` and other declarations in imported files (development branch) | `mod name {}`, files and modules | experimental; release 0.3.0 has none, and views/components still lack visibility modifiers | 🧪 |
| Imports | `import "src/math.zyl" as math`, `math::add()` (development branch) | `use crate::module::Item;` | project-local imports; `check`, `build`, `run`, `serve`, `context`, `verify`, and `impact` load the graph | 🧪 |
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
| Email | Not yet specified | external crate/API | no `Email` capability key | ❌ |
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
library APIs. `Email` is not a Zelyra capability.

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
and [roadmap](#22-roadmap-from-the-current-repository). The live repository status is also shown on
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

## 22. Roadmap from the current repository

This summary comes from `docs/ROADMAP.md` in the current Zelyra repository. It
is a development plan, not a release-date promise.

| Area | Current focus | Open expansion |
|---|---|---|
| Beginner experience and distribution | source and release installers (Linux/Windows x86_64 via SHA-256), `zelyra new/init` with starter templates (`minimal`, `mariadb-crud`, `mariadb-auth`, `mariadb-business`), Docker/DB ports, `zelyra setup`, `zelyra doctor`, E2E tests | signed binaries, interactive connection assistant, reverse-proxy automation |
| Language and compiler | lexer, parser, AST/HIR, type checking, `Option`, `Result`, pattern matching, expression typed holes (`_`), canonical `zelyra fmt`; limited project-local imports are experimental in the unreleased 0.4 branch | broader module visibility and tooling integration, generics, declaration holes, and complete formal verification |
| Database platform | MariaDB, SQLite, and PostgreSQL schema CLI; typed SQL | broader schema coverage and stronger production workflows |
| Views and web | pages, named views, components, default and named slots with fallback content, safe output | themes, view inheritance, free-form styling components |
| Forms and CRUD | validation, CSRF, search, filters, pagination, actions, soft delete, shared CRUD view field profile (`view.fields`) | permanent deletion, retention, archiving, and broader view customization |
| Authentication and audit | login, sessions, roles, permissions, browser admin, audit, and hash chains | self-service, broader policy management, and archive strategies |
| APIs and integration | typed APIs, OpenAPI, TypeScript client, and CORS | versioning, rate limits, and OAuth/integration features |
| Verification and operations | contracts, capability checks, and first concurrency building blocks | cancellation, timeouts, database-pool integration, and production performance |
| AI-native interfaces | `zelyra fmt` (Stage B ✅), expression holes `_` (Stage C 🧪), `zelyra impact` with `--symbol` (Stage D 🧪), `zelyra edit` rename (Stage E 🧪) | declaration holes, schema/runtime impact, complex edit operations, AI benchmark |
| Quality and governance | tests, documentation, and reproducible checks | broader acceptance applications and production hardening |

Do not document email or background-job systems, complete modules/imports,
multi-slot free-form `view` components, or automatic production migrations as
available. Each remains 🗺️ until the current CLI and runtime implement it.

For a learning project, the useful order is:

1. `check` and `run` for language basics;
2. `db create`, `db inspect`, `db plan`, and a reviewed `db apply`;
3. a small `page`, `form`, or `crud` application;
4. only then authentication, roles, audit, and API integration.

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
and source spans. In the experimental 0.4 development branch, `database` also
reports the database declaration, its owning provider module, and the direct
or transitive import binding for each database consumer. It exposes only the
environment-variable name, such as `ZELYRA_DATABASE_MAIN_URL`—never the
connection value or password. For a named database, the report also identifies
`DATABASE_URL` as the fallback accepted by the runtime. Without a `database`
declaration, `DATABASE_URL` is the primary variable; the binding is marked
`legacy_project_environment` and `configuration` remains `null`. The report
explicitly describes one connection per process; it does not enable
multi-database routing.

#### Deterministic impact analysis

🧪 Source dependencies can be deterministically analyzed:

~~~bash
zelyra impact examples/auth_crud_api.zyl --format=json
zelyra impact examples/auth_crud_api.zyl --symbol table:customers --format=json
~~~

The impact response reports source-based tables, SQL, forms, CRUD resources,
views, APIs, permissions, contracts, and a deterministic `references` edge
list for known relationships. For loaded projects it also lists each module's
path, imports, exports, and declarations, with `module_import` edges in the
reference list. Module edges come from the validated project graph and have no
source span in this version. Other source edges include source, target, kind,
and source span. Email, job, test, and live schema impacts remain explicitly
empty or unavailable; the command never connects to MariaDB.

Using `--symbol <kind:name>` focuses the output on a known node such as
`table:customers` or `module:src/storage.zyl`. The focused response contains
only directly connected references and related node IDs. Unknown nodes return
`E-IMPACT-001` and a non-zero exit code.

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
source fingerprint. It also reports `affected_files`; because this version
edits one source file per request, that list contains the entry file. Dependent
tests and schema references are not yet calculated by edit previews. For
function renames, `affected_effects` lists the declared capabilities of the
renamed function and capability-bearing callers found in that same source
file. These declarations remain unchanged by the rename; cross-module effects
are not included.

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
