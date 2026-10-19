# APPENDICES

---

## 24. Authoritative Sources and Compiler Verification (Source Authority)

> **Principle:** Zelyra is its own language. The parser and verified tests decide what exists.

When sources disagree, report the disagreement explicitly and use this authoritative order:

1. **[Formal Language Specification](../../specification.md) and phase documents**
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
| `E-FORM-005` - `E-FORM-006` | Form UI | Invalid appearance variant or widget value | Use a documented appearance or widget value |
| `E-FORM-007` | Form redirect | Redirect is not a safe same-origin absolute path or contains control characters or backslashes | Use a local path such as `/invoices`; external and protocol-relative targets are rejected |
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
| `zelyra console [entry.zyl]` | `[--sandbox|--persist] [--tenant <id>]` | Starts the interactive AST language console; MariaDB/MySQL writes roll back by default |
| `zelyra db console [file.zyl]` | | Starts the native database client with project credentials |
| `zelyra db query <file.zyl> <SQL>` | `[--format table|json|csv]` | Runs a checked query read-only by default |
| `zelyra serve <file.zyl>` | `[host:port]` | Starts the built-in HTTP web server |
See the [language console guide](../../language-console.en.md) for REPL commands, transaction boundaries, and limitations.

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
| `zelyra db history <file.zyl> [--format=json]` | | Inspects MariaDB migration progress |
| `zelyra auth hash-password` | `[--stdin]` | Generates secure Argon2 password hashes |
| `zelyra form validate <file> <Form>` | | Tests forms with sample values on console |
| `zelyra context <file.zyl>` | `[--format json]` | Emits semantic source context for developer tools |
| `zelyra module plan <entry> <module-or-resource-id>` | | Shows known source dependencies and declared runtime effects; readiness blockers remain and it is not a deployment export |

See the [language console guide](../../language-console.en.md) for REPL commands, transaction boundaries, and limitations.

---

## Appendix D: Standard Library Overview (Zelyra 1.0.3 to 1.0.7)

Zelyra's standard library provides built-in functions for mathematics, text,
collections, sets, maps, date arithmetic, conversions, encodings, paths, and
file access. Calls can be chained in typed `|>` pipelines, which support lexical
lambdas since 1.0.7. Operations that access host resources or external services
(filesystem, clock, random, network, database, email/PDF) require their declared
capability.

### 1. Mathematics and numerics

- `abs(x: Int | Float | Decimal)`: Returns the absolute value. The minimum
  `Int` or `Decimal` and non-finite `Float` values produce runtime errors.
- `min(a, b)` and `max(a, b)`: Select the smaller or larger of two values with
  the same type: `Int`, `UInt`, `Float`, `Decimal`, or `String`. Non-finite
  `Float` values produce runtime errors.
- `clamp(value, minimum, maximum)`: Bounds a numeric value of type `Int`,
  `UInt`, `Float`, or `Decimal` to the inclusive interval. Non-finite floats or
  a minimum greater than the maximum produce runtime errors.
- `sqrt(x: Float)`: Computes the square root of a finite, non-negative float.
- `pow(base: Float, exponent: Float)`: Computes `base ^ exponent`; undefined or
  non-finite results produce runtime errors.
- `floor(x: Float)` and `ceil(x: Float)`: Round a finite float down or up.
- `round(x: Float, decimals: Int)`: Rounds halfway cases away from zero.
  `decimals` must be between `-15` and `15`; a non-finite result is an error.

### 2. Strings and text

- `len(text: String) -> Int`: Counts Unicode code points. For arrays, `len`
  returns the number of elements.
- `text_to_lower(text)` and `text_to_upper(text)`: Convert the whole string to
  lowercase or uppercase.
- `text_starts_with(text, prefix)` and `text_ends_with(text, suffix)`: Test a
  prefix or suffix.
- `text_replace(text, search, replacement)`: Replaces all matches.
- `text_split(text, delimiter)` and alias `split`: Split a string into a
  `String[]`.
- `text_pad_left(text, target_length, pad)` and
  `text_pad_right(text, target_length, pad)`: Pad to a Unicode code-point
  length, cycling through the pad string's code points. An empty pad string
  uses spaces. Negative lengths and output over 16 MiB are runtime errors.
- `text_index_of(text, search) -> Option<Int>`: Returns the zero-based Unicode
  code-point index of the first match, or `None`.
- `text_repeat(text, count)`: Repeats the text; negative counts and output over
  16 MiB are runtime errors.
- `text_is_empty(text)` and `text_is_blank(text)`: Check for an empty string or
  a string containing only whitespace.
- `text_capitalize(text)`: Uppercases the first Unicode character.
- `text_substring(text, start, length)`: Extracts a substring using Unicode
  code-point indexes. Negative indexes and lengths are treated as zero.
- `text_trim`, `text_trim_start`, and `text_trim_end`: Remove surrounding,
  leading, or trailing whitespace.
- `text_contains(text, search)` and `text_chars(text)`: Test containment or
  split text into one-code-point strings.

### 3. Arrays and collections

- `append(array, value) -> T[]` and `concat(left, right) -> T[]`: Return arrays
  with the value appended or both arrays joined.
- `contains(array, value) -> Bool`, `first(array) -> Option<T>`, and
  `last(array) -> Option<T>`: Search or retrieve an end element.
- `sort`, `sort_desc`, and `reverse`: Return sorted or reversed arrays.
- `slice(array, start, length)`: Extracts a bounded portion of an array.
- `join(array, delimiter) -> String`: Joins rendered elements with a delimiter.
- `unique(array)` and alias `dedup`: Keep the first occurrence of each value.
- `array_sum(array)`: Adds `Int`, `Float`, or `Decimal` elements. Integer and
  decimal overflow and non-finite float values produce runtime errors; an empty
  array returns `0`.
- `avg(array)`: Averages `Int` and `Float` arrays to `Float`, and `Decimal`
  arrays to `Decimal`. Decimal division truncates at the fixed Decimal scale.
  Empty arrays and overflow produce runtime errors.
- `min_of(array) -> Option<T>` and `max_of(array) -> Option<T>`: Select the
  smallest or largest number or string; an empty array returns `None`, and
  non-finite floats produce runtime errors.
- `chunk(array, size)`: Splits an array into chunks no larger than `size`;
  `size` must be greater than zero.
- `flatten(array)`: Flattens one level of nested arrays.
- `zip(left, right) -> Tuple<T, U>[]`: Pairs corresponding elements through the
  shorter input array.
- `map(array, transform) -> U[]`: Transforms array elements (supports lexical
  lambdas like `|x| x * 2` since 1.0.7).
- `filter(array, predicate) -> T[]`: Filters elements with a predicate or lambda
  (`|x| x > 0`).
- `find(array, predicate) -> Option<T>`: Returns the first matching element or
  `None`.
- `reduce(array, initial, accumulator) -> U`: Folds an array with an initial
  value and accumulator function.

### 4. Sets and maps

- `set_union`, `set_intersection`, and `set_difference`: Return set union,
  intersection, or difference.
- `get(map, key) -> Option<V>` and `get_or(map, key, default) -> V` (alias
  `get_or_default`): Look up a value, with or without a fallback.
- `has_key(map, key) -> Bool`: Checks whether a map contains a key.
- `put(map, key, value) -> Map<K, V>`: Functionally inserts or updates a pair.
- `map_remove(map, key)` and `map_merge(left, right)`: Return an updated map;
  values in the right map replace matching keys from the left.
- `map_increment(map: Map<K, Int>, key: K) -> Result<Map<K, Int>, String>`:
  Increments a typed counter or inserts it with value `1`. The input map stays
  unchanged; integer overflow is returned as `Err`.
- `keys(map) -> K[]` and `values(map) -> V[]`: Return the keys or values.
- `entries(map) -> Tuple<K, V>[]`: Returns key-value pairs that can be type
  checked and destructured in a loop: `for (key, value) in entries(map)`.

### 5. Dates, options, results, and conversions

- `date_add_days(date, days) -> Date`: Adds or subtracts calendar days; results
  must remain in years 0001 through 9999.
- `date_diff_days(start, end) -> Int`: Returns `start - end` in calendar days.
  For remaining time until a deadline, use `date_diff_days(deadline, today)`.
- `is_before(a, b)` and `is_after(a, b)`: Compare two dates chronologically.
- `is_ok`, `is_err`, `is_some`, and `is_none`: Test `Result` and `Option` values.
- `unwrap_or(option, fallback)`: Returns the contained value or fallback.
- `panic(message)`: Stops execution with a runtime error.
- `to_string(value)`: Returns the Zelyra display form of a value.
- `to_int(value) -> Int`: Converts `Int`, `UInt`, finite `Float`, `Bool`, or a
  numeric string. Floats truncate toward zero. Out-of-range values and invalid
  strings produce runtime errors.
- `to_float(value) -> Float`: Converts `Float`, `Int`, `UInt`, or a numeric
  string to a finite float; invalid or non-finite values produce runtime errors.
- `parse_int(text) -> Result<Int, String>`,
  `parse_float(text) -> Result<Float, String>`, and
  `parse_bool(text) -> Result<Bool, String>`: Parse strings without throwing;
  invalid values return `Err`. `parse_bool` accepts `true`, `false`, `1`, and
  `0`, case-insensitively for words.

### 6. Hashing, encodings, and paths

- `hash_sha256(text) -> String`: Returns the lowercase hexadecimal SHA-256 hash
  of the UTF-8 bytes.
- `base64_encode(text)` and `base64_decode(text)`: Encode UTF-8 text, or return
  `Result<String, String>` when decoding.
- `url_encode(text)`: Percent-encodes bytes except RFC 3986 unreserved bytes.
- `url_decode(text) -> Result<String, String>`: Decodes percent escapes; `+` is
  treated as a space.
- `uuid_v4() -> String` (`uses Random`): Creates a cryptographically secure
  version 4 UUID.
- `path_join`, `path_filename`, `path_extension`, and `path_dir`: Perform path
  operations using the host operating system's path conventions, without
  filesystem access or a capability.

### 7. Capability-guarded system APIs

- `uses Console`: `read_console(prompt: String) -> String?` prints a prompt to
  standard output and reads one line; `None` indicates EOF.
- `uses Clock`: `now() -> Timestamp` returns the current timestamp in
  milliseconds since the Unix epoch.
- `uses Random`: `random_int(min: Int, max: Int) -> Int` chooses from the
  inclusive range; `uuid_v4()` creates a version 4 UUID.
- `uses Environment`: `env(name: String) -> Option<String>` reads a host
  environment variable.
- `uses FileSystem`: `read_text`, `write_text`, `delete_file`, `list_dir`,
  `file_exists`, `dir_exists`, and `file_size` operate within the configured
  filesystem permissions. `write_text` replaces existing file contents inside
  allowed write paths; `file_size` returns `Result<Int, String>`.
- `uses Network`: `http_get(url: String) -> String` performs an allowed GET
  request. `http_request(method: String, url: String, headers: String[],
  body: String?) -> HttpResponse` sends a request with an optional text body.
  `http_json<Req, Res>(method, url, headers, body: Req?) -> Res` decodes a
  successful JSON response. `http_result<Req, Res>(method, url, headers,
  body: Req?) -> HttpResult<Res>` also returns status and headers.
- `uses Process`: `run_process(command: String, args: String[]) ->
  ProcessResult` starts an explicitly permitted process without a shell.
- `uses Database`: `sql<T[]> { SELECT ... }` executes typed queries with bound
  parameters; `transaction { ... }` groups database operations atomically.
- `uses Email`: `render_pdf(view_name, record_data) -> Bytes` (new in 1.0.7)
  renders a declared view with record fields into paginated text PDF bytes;
  `mail_queue(to, subject, text, filename, pdf_bytes)` enqueues transactional
  PDF email attachments in MariaDB outbox.

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

**Question: How do I import other source files in Zelyra 1.0.4?**
*Answer:* Use `import "src/math.zyl" as math` to add a file to the project
graph. Public functions and other supported declarations can then be referenced
through the alias. Module support has documented limits; see
[Chapter 15](textbook-foundations.md#chapter-15-modules-and-code-organization).

**Question: Can I build command-line applications with Zelyra?**
*Answer:* Yes. `print()` emits values. `read_console("Prompt: ")` reads a line and returns `String?`; the function needs `uses Console` and the project may also need `console = true`.

**Question: Why does Zelyra emphasize MariaDB as its primary database engine?**
*Answer:* MariaDB provides exceptional transactional performance, open-source licensing, robust cloud compatibility, and rock-solid reliability for enterprise web applications.

---

## Appendix J: Next Resources and Community

- **Official GitHub Repository:** [https://github.com/sf1976/zelyra](https://github.com/sf1976/zelyra)
- **Documentation & Online Handbook:** [https://siedelmann.com/handbook](https://siedelmann.com/handbook) / [https://siedelmann.com/handbuch](https://siedelmann.com/handbuch)
- **Examples & Templates:** Check the `examples/` directory in the official repository for runnable templates covering authentication, CRUD views, REST APIs, and database migrations.
