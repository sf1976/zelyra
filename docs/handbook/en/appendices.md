# APPENDICES

---

## 24. Authoritative Sources and Compiler Verification (Source Authority)

> **Principle:** Zelyra is its own language. The parser and verified tests decide what exists.

When sources disagree, report the disagreement explicitly and use this authoritative order:

1. **[Formal Language Specification](specification.md) and phase documents**
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

## Appendix A: Quickstart / Cheat Sheet (Syntax Cheat Sheet)

### Basic Syntax
```zelyra
// Functions with contracts
fn sum(a: Int, b: Int) -> Int
    requires { a >= 0 && b >= 0 }
    ensures { result >= 0 }
{
    return a + b
}

// Entry point and variables
fn main() {
    x = 10                  // Type inference (immutable)
    mutable counter = 0     // Mutable
    name: String = "Zelyra" // Explicit type

    print(sum(3, 7))
}
```

### Types
- Numbers: `Int` (64-bit signed integer), `UInt` (unsigned integer), `Float`, `Decimal` (fixed-point arithmetic)
- Text & Characters: `String`, `Char`
- Booleans: `Bool` (`true`, `false`)
- Collections: `Int[]`, `String[]`
- Optionality: `Option<T>` (`Some(x)`, `None`), shorthand `T?`
- Error Handling: `Result<T, E>` (`Ok(x)`, `Err(e)`)
- System & Chronology: `Timestamp`, `Date`, `Time`, `Duration`

### Control Flow
```zelyra
fn control_flow(x: Int) {
    if x > 10 {
        print("Large")
    } else {
        print("Small")
    }

    match x {
        1 => { print("One") }
        2 => { print("Two") }
        _ => { print("Other") }
    }

    mutable i = 0
    while i < 3 invariant { i >= 0 } {
        i = i + 1
    }

    for n in [1, 2, 3] {
        print(n)
    }
}

fn main() {
    control_flow(1)
}
```

### Database & Web
```zelyra
database main {
    engine: mariadb
    database: "app"
}

table items {
    id: Id primary auto
    description: String required
}

page "/items" {
    html {
        <h1>Item List</h1>
    }
}
```

---

## Appendix B: Complete Zelyra Diagnostic & Error Code Reference

| Error Code | Category | Description | Typical Fix |
| :--- | :--- | :--- | :--- |
| `E-LEX-001` | Lexer | Unexpected character / lexical error | Remove typographical or illegal special characters |
| `E-PARSE-001` | Parser | Syntax error (e.g. missing brace, invalid token) | Correct syntax per Zelyra grammar |
| `E-NAME-001` | Resolution | Unknown name / variable not declared | Check declaration or correct typo |
| `E-TYPE-001` | Type checking | Type mismatch (e.g. String assigned to Int) | Align types or add conversion |
| `E-FEATURE-001` | Feature switch | Access to disabled language surface (`web`, `api`, `crud`, `auth`, `audit`) | Enable feature in `zelyra.toml` or `.env` |
| `E-CAP-001` / `E-CAP-002` | Capabilities | Missing capability permission (e.g. `database`, `network`) | Grant capability in `zelyra.toml` under `[capabilities]` |
| `E-POLICY-001` / `002` | Policy | Security or audit policy violation | Review security policy declarations |
| `E-DB-001` - `E-DB-005` | Database | Database connection or driver error | Verify `DATABASE_URL`, ensure MariaDB is running |
| `E-SQL-001` - `E-SQL-004` | SQL | Invalid SQL / schema mismatch / unknown column | Check SQL statement against table schema |
| `E-VIEW-001` - `E-VIEW-009` | Views & Pages | Error in view interpolation, slots, or data binding | Check slot names and binding data types |
| `E-VIEW-010` - `E-VIEW-015` | Query controls | Invalid search, sort, pagination, or filter fields | Check declared whitelist (`search`, `sort`, `filter`) |
| `E-FORM-001` - `E-FORM-004` | Forms | Validation error or invalid form field types | Review form declaration and input payload |
| `E-CRUD-001` - `E-CRUD-006` | CRUD | Invalid CRUD resource, schema conflict, or layout error | Verify table binding and layout slots |
| `E-AUTH-001` - `E-AUTH-028` | Authentication | Session, password, or permission conflict | Check roles (`permits`), `requires auth`, and password hashes |
| `E-AUDIT-001` - `E-AUDIT-010` | Audit trail | Failure in cryptographic hash chain of audit log | Validate checksums and audit table integrity |
| `E-SETUP-001` - `E-SETUP-006` | Setup flow | Port conflict, socket error, or Compose failure | Choose free ports, check Docker socket permissions |
| `E-SETUP-WEB-001` | Web setup | Invalid or expired setup token | Restart setup assistant and use tokenized URL |
| `E-IMPACT-001` | Impact analysis | Cyclical or invalid dependencies | Untangle code dependencies |
| `E-MOD-019` | Module/table dependency | Table owner is missing from the consumer's import closure | Import the table-owning module directly or transitively |
| `E-MOD-020` | Module/UI dependency | Referenced public view or component is outside the consumer's import graph | Import the UI-owning module directly or transitively |
| `E-MOD-021` | Module/table grant | A recognized read/write access has no matching grant from the table owner | Check the table's `access` list and exact module path |
| `E-MOD-022` | Module/database dependency | A database-using module does not import the provider | Import the provider directly or transitively; move configuration out of `main.zyl` |
| `E-RUNTIME-001` | Runtime | Unhandled runtime error | Check contracts (`requires`, `ensures`) or error values |

---

## Appendix C: Zelyra CLI Command Reference

| Command | Option / Flag | Description |
| :--- | :--- | :--- |
| `zelyra --version` | | Outputs full compiler and package version |
| `zelyra new <dir>` | `--template minimal\|mariadb-crud\|...` | Creates a new Zelyra project with template |
| | `--mariadb` | Generates MariaDB project with Compose, Dockerfile, and `.env` |
| | `--web-port <p> --host-port <p> --db-host-port <p>` | Configures container and host ports |
| `zelyra init` | `[--mariadb]` | Initializes current directory as Zelyra project |
| `zelyra check <file.zyl>` | `[--format json]` | Statically checks syntax, types, contracts, and capabilities |
| `zelyra run <file.zyl>` | | Compiles and executes a Zelyra program |
| `zelyra serve <file.zyl>` | `[host:port]` | Starts the built-in HTTP web server |
| `zelyra module plan <entry> <module-or-resource>` | | Experimental read-only preview of known module dependencies |
| `zelyra module bundle <entry> <module-or-resource>` | `--output <dir> [--dry-run] [--docker --compiler-ref <commit>]` | Writes a checked experimental bundle or previews its JSON file plan; not a completeness claim |
| | `--docker --compiler-ref <40-character-commit>` | Generates Dockerfile, Compose app, and `.env.example`; closure remains incomplete |
| `zelyra setup` | `[--database]` | Starts Docker Compose / MariaDB |
| | `[--schema]` | Starts environment and applies database schema |
| | `[--all]` | Executes configuration, container startup, and schema migration |
| | `[--host-port <p>] [--db-host-port <p>]` | Enforces exact host ports for new `.env` |
| `zelyra setup --web` | `[--port <p>]` | Starts local token-protected browser setup assistant |
| `zelyra config <file.zyl>` | `[--format json]` | Safely displays effective configuration and feature switches |
| `zelyra doctor <file.zyl>` | `[--port <p>] [--json]` | Checks toolchain, MariaDB, Docker, and ports non-destructively |
| | `[--env-file <file>]` | Reads `DATABASE_URL` from specified file |
| `zelyra fmt <file.zyl>` | `[--check]` | Formats source code according to official standards |
| `zelyra verify <file.zyl>` | | Performs formal contract verification |
| `zelyra doc <file.zyl>` | `--openapi` | Generates OpenAPI 3.0 specifications |
| | `--typescript` | Generates typed dependency-free TypeScript client |
| `zelyra db init <file.zyl>` | | Initializes database and base tables |
| `zelyra db setup <file.zyl>` | | Sets up MariaDB database initially |
| `zelyra db apply <file.zyl>` | | Applies schema migrations safely |
| `zelyra auth hash-password` | `[--stdin]` | Generates secure Argon2 password hashes |
| `zelyra form validate <file> <Form>` | | Tests forms with sample values on console |
| `zelyra context <file.zyl>` | `[--format json]` | Emits semantic source context for developer tools |
| `zelyra module plan <entry> <module-or-resource-id>` | | Shows known dependency closure from a source module or application resource; not a deployment export |

---

## Appendix D: Standard Library Overview

### Pure Core Functions (Zero Capabilities Required)
- `print(value)`: Prints any value to standard output.
- `len(array)`: Returns the number of elements in an array as an `Int`.
- `append(array, element)`: Produces a new array with the appended value.
- `contains(array, element)` -> `Bool`: Checks whether an item is present in an array.
- `first(array)` -> `Option<T>`: Returns the first item wrapped in `Some`, or `None`.
- `last(array)` -> `Option<T>`: Returns the final item wrapped in `Some`, or `None`.
- `get(map, key)` -> `Option<V>`: Looks up a key within a `Map<K, V>`.
- `put(map, key, value)` -> `Map<K, V>`: Inserts or updates a key-value pair functionally.
- `keys(map)` -> `K[]`: Returns all keys of a map as an array.
- `values(map)` -> `V[]`: Returns all values of a map as an array.
- `Some(value)` / `None`: Value constructors for the `Option<T>` type.
- `Ok(value)` / `Err(error)`: Value constructors for the `Result<T, E>` type.
- `json_encode(value)` -> `String`: Serializes typed data into a JSON string.
- `json_decode<T>(text)` -> `T`: Parses typed JSON; invalid data are reported as runtime errors.

### Capability-Guarded Functions
- `uses Console`:
  - `read_console(prompt: String)` -> `String?`: Displays the prompt and reads one line; `None` represents EOF.
- `uses Clock`:
  - `now()` -> `Timestamp`: Returns current system timestamp.
- `uses Random`:
  - `random_int(min: Int, max: Int)` -> `Int`: Generates an integer in the given range.
- `uses Environment`:
  - `env(name: String)` -> `Option<String>`: Reads a host environment variable.
- `uses FileSystem`:
  - `read_text(path: String)` -> `String`: Reads file contents as text.
  - `write_text(path: String, content: String)`: Writes text content to file.
  - `delete_file(path: String)`: Deletes a target file.
  - `list_dir(dir: String)` -> `String[]`: Lists directory filenames.
- `uses Database`:
  - `sql<T[]> { SELECT ... }`: Executes type-checked SQL queries returning entity records.
  - `transaction { ... }`: Wraps multiple SQL operations inside an atomic transaction.

---

## Appendix E: SQL Cheat Sheet for Zelyra Developers

SQL statements embedded directly within Zelyra are executed with `sql<T[]>` or `sql`:

```zelyra
database main {
    engine: mariadb
    database: "app"
}

table tasks {
    id: Id primary auto
    name: String required
    completed: Bool default false
}

fn sql_examples() uses Database {
    // 1. SELECT with typed return type and safe parameter
    status = false
    filtered = sql<Task[]> {
        SELECT id, name, completed
        FROM tasks
        WHERE completed = :status
    }

    // 2. INSERT within a transaction
    text = "New task"
    transaction {
        sql {
            INSERT INTO tasks (name, completed)
            VALUES (:text, false)
        }
    }

    // 3. UPDATE
    target_id = 1
    transaction {
        sql {
            UPDATE tasks
            SET completed = true
            WHERE id = :target_id
        }
    }
}

fn main() uses Database {
    print("SQL cheat sheet validated.")
}
```

---

## Appendix F: HTML and Web Reference in Zelyra

### Web structures and declarations

| Element | Declaration | Purpose |
| :--- | :--- | :--- |
| **Page** | `page "/path/{param}" { ... }` | Defines an HTTP GET route with path parameters and HTML response |
| **View Layout** | `view LayoutName { html { ... <slot /> ... } }` | Reusable layout with default and named slots |
| **Component** | `component Name { props { ... } html { ... } }` | Reusable HTML component with typed properties |
| **Named Slot** | `<slot name="header">Fallback</slot>` | Placeholder in layout/component with optional default content |
| **Slot Injection** | `<slot name="header">Content</slot>` | Passes child content into matching slot |
| **Data Loading** | `load item = sql<Item> { SELECT ... }` | Typed loading of single record with field access `{item.field}` |
| **Collection Loop**| `for item in items { <li>{item.name}</li> }` | Typed server-side iteration over loaded collection |
| **Search Control** | `search { col1 col2 }` | Whitelist-checked URL search with parameterized `LIKE` query |
| **Sort Control** | `sort { col1 col2 }` | Typed sorting via `?sort=col&order=asc\|desc` |
| **Pagination** | `paginated 25` | Pagination with `LIMIT`/`OFFSET`, `page`, `pages`, and `total` |
| **Filter Control** | `filter { col1 col2 }` | Typed filter operators (`eq`, `contains`, `starts_with`, `gt`, `lte` etc.) |
| **CRUD Layout** | `crud Res { table tbl layout: LayoutName }` | Embeds CRUD views into slots `title`, `nav`, `content`, `actions` |

---

## Appendix G: Glossary of Technical Terms

- **AST (Abstract Syntax Tree):** The hierarchical tree data structure representing the syntactic elements of your source code following lexical and grammatical analysis.
- **Capability:** An explicit permission marker (`uses FileSystem`, `uses Database`, etc.) required on a function's signature before it is permitted to touch guarded external system resources.
- **Design by Contract:** A formal software design methodology where functions establish strict preconditions (`requires`) and postconditions (`ensures`) enforced by static analysis and runtime checks.
- **Immutable:** Unchangeable after initial assignment. In Zelyra, all variables are immutable by default unless explicitly declared with the `mutable` keyword.
- **Invariant:** A logical condition (such as in a loop) that is guaranteed to evaluate to true before and after every execution cycle.
- **Option:** A algebraic data type (`Some(v)` or `None`) representing the potential absence of a value—Zelyra's robust replacement for dreaded null-pointer exceptions.
- **Result:** A type (`Ok(v)` or `Err(e)`) that encapsulates the outcome of an operation that might fail, treating errors as first-class recoverable values rather than untyped exceptions.
- **Typed Hole (`_`):** A compiler-supported placeholder indicating incomplete code, which signals the compiler and AI tooling to display the required type, local scope, and applicable contracts for that location.

---

## Appendix H: Solutions to Chapter Exercises

### Chapter 1: Greeting
```zelyra
fn main() {
    print("Hello World from Zelyra!")
}
```

### Chapter 5: Calculate Discount Price
```zelyra
fn calculate_discount(original: Float, percent: Float) -> Float {
    return original * (1.0 - (percent / 100.0))
}

fn main() {
    print(calculate_discount(100.0, 20.0))
}
```

### Chapter 11: Double Numbers
```zelyra
fn double_number(number: Int) -> Int {
    return number * 2
}

fn main() {
    print(double_number(21))
}
```

### Chapter 12: Pre- and Postconditions
```zelyra
fn clamp(value: Int, min_val: Int, max_val: Int) -> Int
    requires { min_val <= max_val }
    ensures { result >= min_val && result <= max_val }
{
    if value < min_val { return min_val }
    if value > max_val { return max_val }
    return value
}

fn main() {
    print(clamp(120, 0, 100))
}
```

### Chapter 13: Sum an Array
```zelyra
fn sum_array(numbers: Int[]) -> Int {
    mutable total = 0
    for n in numbers {
        total = total + n
    }
    return total
}

fn main() {
    items: Int[] = [1, 2, 3, 4, 5]
    print(sum_array(items))
}
```

---

## Appendix I: Frequently Asked Questions (FAQ)

**Question: Why is there no `import` statement in Zelyra 0.3.0?**
*Answer:* The published 0.3.0 binary has no module imports. A CLI invocation
checks only the explicitly named `.zyl` source file. The current unreleased
development branch has experimental imports for functions, types, records,
tables, views, components, and project-wide database configuration. Commands
including `serve` can use the linked project graph, but complete stable modules
and package exports remain planned work.

**Question: Can I build command-line applications with Zelyra?**
*Answer:* Yes. `print()` emits values. `read_console("Prompt: ")` reads a line and returns `String?`; the function needs `uses Console` and the project may also need `console = true`.

**Question: Why does Zelyra emphasize MariaDB as its primary database engine?**
*Answer:* MariaDB provides exceptional transactional performance, open-source licensing, robust cloud compatibility, and rock-solid reliability for enterprise web applications.

---

## Appendix J: Next Resources and Community

- **Official GitHub Repository:** [https://github.com/sf1976/zelyra](https://github.com/sf1976/zelyra)
- **Documentation & Online Handbook:** [https://siedelmann.com/handbook](https://siedelmann.com/handbook) / [https://siedelmann.com/handbuch](https://siedelmann.com/handbuch)
- **Examples & Templates:** Check the `examples/` directory in the official repository for runnable templates covering authentication, CRUD views, REST APIs, and database migrations.
