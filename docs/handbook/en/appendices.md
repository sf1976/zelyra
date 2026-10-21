# APPENDICES

## Appendix A: Quickstart / Cheat Sheet (Syntax Cheat Sheet)

### Basic Syntax

`# comment` continues to the end of the line. For a block, opening and closing `##` each stand alone on their own line, with comment text between them.
```zelyra
# Functions with contracts
fn sum(a: Int, b: Int) -> Int
    requires { a >= 0 && b >= 0 }
    ensures { result >= 0 }
{
    return a + b
}

# Entry point and variables
fn main() {
    x = 10                  # Type inference (immutable)
    mutable counter = 0     # Mutable
    name: String = "Zelyra" # Explicit type

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
| `E-VIEW-041` | View localization | Invalid static translation key | Use only letters, digits, periods, underscores, and hyphens (up to 128 characters) |
| `E-VIEW-042` | View localization | Translation used in a non-HTML response | Use `t`/`i18n` only in HTML pages |
| `E-FORM-001` - `E-FORM-004` | Forms | Validation error or invalid form field types | Review form declaration and input payload |
| `E-CRUD-001` - `E-CRUD-006` | CRUD | Invalid CRUD resource, schema conflict, or layout error | Verify table binding and layout slots |
| `E-AUTH-001` - `E-AUTH-028` | Authentication | Session, password, or permission conflict | Check roles (`permits`), `requires auth`, and password hashes |
| `E-AUDIT-001` - `E-AUDIT-010` | Audit trail | Failure in cryptographic hash chain of audit log | Validate checksums and audit table integrity |
| `E-SETUP-001` - `E-SETUP-006` | Setup flow | Port conflict, socket error, or Compose failure | Choose free ports, check Docker socket permissions |
| `E-SETUP-WEB-001` | Web setup | Invalid or expired setup token | Restart setup assistant and use tokenized URL |
| `E-IMPACT-001` | Impact analysis | Cyclical or invalid dependencies | Untangle code dependencies |
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
| `zelyra files backup [entry.zyl]` | `--output <new-directory>` | Backs up private Zelyra file storage and manifests; excludes database, configuration, and secrets |
| `zelyra files restore [entry.zyl]` | `--from <directory> --apply` | Verifies checksums before restore; restore the database separately first and stop Zelyra servers |
| `zelyra db console [file.zyl]` | | Starts the native database client with project credentials |
| `zelyra db query <file.zyl> <SQL>` | `[--format table|json|csv]` | Runs a checked query read-only by default |
| `zelyra serve <file.zyl>` | `[host:port]` | Starts the built-in HTTP web server |
See the [language console guide](../../language-console.en.md) for REPL commands, transaction boundaries, and limitations.

| `zelyra module plan <entry> <module-or-resource>` | | Experimental read-only preview of known module dependencies |
| `zelyra module bundle <entry> <module-or-resource>` | `--output <dir> [--dry-run] [--docker --compiler-ref <commit>]` | Writes a checked experimental bundle or previews its JSON file plan; not a completeness claim |
| | `--docker --compiler-ref <40-character-commit>` | Generates Dockerfile, Compose app, and `.env.example`; closure remains incomplete |
| `zelyra setup` | `[--database]` | Starts Docker Compose / MariaDB |
| `zelyra routes <entry.zyl>` | `[--format human\|json]` | Lists recognized routes with HTTP methods and source locations; same-method overlaps fail with `E-ROUTE-001` |
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
| `zelyra db create <file.zyl>` | | Generates database setup SQL |
| `zelyra db setup <file.zyl>` | | Sets up MariaDB database initially |
| `zelyra db map <file.zyl>` | `[--format=text\|json]` | Read-only comparison of declared tables with live tables, columns, and foreign keys |
| `zelyra db plan <file.zyl>` | | Shows the schema changes that would be applied; review the plan before migration |
| `zelyra db apply <file.zyl>` | | Applies schema migrations safely |
| `zelyra auth hash-password` | `[--stdin]` | Generates secure Argon2 password hashes |
| `zelyra form validate <file> <Form>` | | Tests forms with sample values on console |
| `zelyra context <file.zyl>` | `[--format json]` | Emits semantic source context for developer tools |
| `zelyra editor [directory]` | `[--port <p>] [--no-open]` | starts Studio |
| `zelyra --tutorial invoice [directory]` | `[--no-start]` | creates the invoice tutorial |
| `zelyra outbox setup <file.zyl>` | | sets up the MariaDB outbox |
| `zelyra outbox list <file.zyl>` | `[--limit <1..200>] [--format text\|json]` | lists entries |
| `zelyra outbox run <file.zyl>` | `--handler <event.type=function> [--once]` | processes due events |
| `zelyra outbox requeue <event-id> <file.zyl>` | | requeues an event |
| `zelyra task <list|run> <file.zyl>` | `[command] [options]` | Lists declared tasks and runs them with compiler verification |
| `zelyra service <install|status|restart|change|remove>` | `[--port <port>]` | Installs and manages Linux systemd user services |
| `zelyra schedule <list|export> <file.zyl>` | `[--format cron\|json]` | Lists resident schedules or exports them as a cron preview |
| `zelyra module init <dir> --name <n>` | `[--version <semver>]` | Scaffolds an offline-first local Zelyra module package |
| `zelyra module list <file.zyl>` | `[--format human\|json]` | Inspects source modules, exports, checksums, and settings schemas |
| `zelyra module settings check <file.zyl>` | `[--format json]` | Validates typed module settings and environment secret availability |
| `zelyra module plan <entry.zyl> <target>` | | Analyzes static module source closure and dependency graph |
| `zelyra module bundle <entry> <target>` | `--output <dir> [--docker]` | Creates a portable source bundle with isolated package dependencies |
| `zelyra package lock [file.zyl]` | `[--check\|--update]` | Verifies or updates SHA-256 package digests in `zelyra.lock` |
| `zelyra package keygen\|sign\|verify` | `[options]` | Creates Ed25519 keypairs and signs/verifies `.zpkg` source archives |
| `zelyra package trust <add\|list\|remove>` | `[publisher] [key-hex]` | Manages trusted publisher public keys in `zelyra.trust.toml` |
| `zelyra package registry build <dir>` | | Builds a static index (`index.json`) from signed `.zpkg` files |
| `zelyra package search <url> [filter]` | | Searches a static package index by name |
| `zelyra package add <url> <pkg> <ver>` | `--as <alias>` | Installs an exact, verified package version |
| `zelyra package update <url> <pkg> <ver>` | `--as <alias> [--plan <file>]` | Deterministically upgrades package dependencies |
| `zelyra package remove <alias>` | | Removes a package dependency and refreshes `zelyra.lock` |
| `zelyra package cache clean` | `[--apply]` | Safely previews and prunes unreferenced cached package versions |
| `zelyra package recover` | `[--apply]` | Restores manifests and lockfiles after interrupted transactions |
| `zelyra reflect <entry.zyl>` | `[--format=json\|human]` | Exports symbols, schemas, routes, and contracts as versioned JSON |
| `zelyra audit <inspect\|export\|verify\|prune>` | `<file.zyl> [options]` | Inspects, verifies, or prunes application audit logs |
| `zelyra migrate-layout --check` | `[entry.zyl] [--format human\|json]` | Verifies compliance with the canonical standard project layout |
| `zelyra db history <file.zyl>` | | Displays applied database migration history |
| `zelyra update` | `[--check]` | Checks or installs the stable release |

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
  `parse_decimal(text) -> Result<Decimal, String>` (exact base-10 parsing with
  an optional minus sign, at most nine fractional digits, and no floating-point
  intermediate; whitespace, a leading plus, exponent notation, and overflow
  return `Err`),
  `parse_date(text) -> Result<Date, String>`, and
  `parse_time(text) -> Result<Time, String>` validate the unambiguous forms
  `YYYY-MM-DD` and `HH:MM:SS`, without inferring a locale or timezone. Invalid
  calendar dates and clock values return `Err`. `parse_float(text) ->
  Result<Float, String>` and `parse_bool(text) -> Result<Bool, String>` parse
  without throwing; invalid values return `Err`. `parse_bool` accepts `true`,
  `false`, `1`, and `0`, case-insensitively for words.

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
- `uses Process`:
  `run_process(command: String, args: String[]) -> String` starts an explicitly
  permitted process without a shell and returns stdout for exit code 0.
  `run_process_piped(command: String, args: String[], stdin: String,
  timeout_ms: Int) -> ProcessResult` sends bounded text to stdin and returns
  `exit_code`, `stdout`, and `stderr`. The command must be in the project
  process allowlist; the effective timeout is the lower of the argument and
  policy limits.
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
    # 1. SELECT with typed return type and safe parameter
    status = false
    filtered = sql<Task[]> {
        SELECT id, name, completed
        FROM tasks
        WHERE completed = :status
    }

    # 2. INSERT within a transaction
    text = "New task"
    transaction {
        sql {
            INSERT INTO tasks (name, completed)
            VALUES (:text, false)
        }
    }

    # 3. UPDATE
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

Each entry links to the chapter that explains the term. Unsupported language constructs and CLI commands are marked as such.

- **Value:** A named piece of data. In Zelyra, values are immutable by default to prevent hidden side effects and state bugs. [Chapter 5: Values and Data Types](#chapter-5-values-and-data-types).
- **mutable (Mutability):** Keyword required to declare variables that may be reassigned. Makes state mutations explicitly visible to developers and compilers. [Chapter 6: Variables and Immutability](#chapter-6-variables-and-immutability).
- **Function (fn):** A named operation with explicit parameter types and a definite return type. Functions are the foundational building blocks of Zelyra programs. [Chapter 11: Functions and Procedures](#chapter-11-functions-and-procedures).
- **Pure Function:** A function without side effects: its result depends only on its inputs, so identical inputs produce the same result. [Chapter 11: Functions and Procedures](#chapter-11-functions-and-procedures).
- **Type Inference:** The compiler's capability to automatically deduce data types from values and expressions, combining static safety with concise syntax. [Chapter 5: Values and Data Types](#chapter-5-values-and-data-types).
- **Typed Hole (_):** The underscore `_` as a formal typed placeholder. Instructs compiler and AI tools where implementation is needed, with expected types inferred from surrounding context. [Chapter 32: AI-Nativity](#chapter-32-ai-nativity-why-zelyra-is-built-for-ai-assistants).
- **Block & Scope:** The lexical scope of identifiers, bounded by curly braces `{ ... }`. Names declared inside a block are bound to that scope. [Chapter 6: Variables and Immutability](#chapter-6-variables-and-immutability).
- **Data Type:** A precise description of allowable values and valid operations. Includes scalars (Int, Float, Bool, String, Decimal, Id) and compound types. [Chapter 5: Values and Data Types](#chapter-5-values-and-data-types).
- **Option (Some / None):** Type representing a value that may be present (`Some(v)`) or absent (`None`). The question mark suffix `Type?` guarantees absence is handled explicitly. [Chapter 18: Option](#chapter-18-nothingness-does-not-exist-working-safely-with-option).
- **Result (Ok / Err):** A type for operations that may fail. It returns success as `Ok(data)` and an error as `Err(reason)` explicitly. [Chapter 17: Result Pattern](#chapter-17-errors-as-values-the-result-pattern).
- **Null Safety (Zero null):** Zelyra has no concept of `null`, `nil`, or `undefined`. The "billion-dollar mistake" of null-pointer exceptions is structurally excluded by the type system. [Chapter 18: Option](#chapter-18-nothingness-does-not-exist-working-safely-with-option).
- **Pattern Matching (match):** Exhaustive pattern evaluation across data structures, options, and results. The compiler rejects code if any branch is left unhandled. [Chapter 14: Custom Data Types](#chapter-14-creating-custom-data-types-records-tables).
- **Domain Types (Nominal):** Nominal type definitions like `type UserId = Id` and `type OrderId = Id`. Prevents accidentally swapping identically represented primitives in business logic. [Chapter 14: Custom Data Types](#chapter-14-creating-custom-data-types-records-tables).
- **Record / Struct:** A structured collection of named, strongly typed fields representing a concrete entity, with constructor and destructuring support. [Chapter 14: Custom Data Types](#chapter-14-creating-custom-data-types-records-tables).
- **Enum / Sum Type:** An enum describes a fixed set of distinct variants. User-defined `enum` declarations are not part of the currently documented Zelyra language surface; the conceptual pattern is not executable Zelyra syntax. [Chapter 14: Custom Data Types](#chapter-14-creating-custom-data-types-records-tables).
- **Design by Contract:** Formal software engineering method ensuring correctness: functions declare rigorous preconditions (`requires`) and postconditions (`ensures`). [Chapter 12: Contracts and Preconditions](#chapter-12-contracts-and-preconditions-design-by-contract).
- **Precondition (requires):** Contract clause attached to a function signature enforcing valid caller inputs before body execution. Shields functions from invalid state. [Chapter 12: Contracts and Preconditions](#chapter-12-contracts-and-preconditions-design-by-contract).
- **Postcondition (ensures & result):** Contractual guarantee about the computation. The keyword `result` references the returned value and verifies its invariants. [Chapter 12: Contracts and Preconditions](#chapter-12-contracts-and-preconditions-design-by-contract).
- **Loop Invariant (invariant):** Formal assertion inside loops that must remain true before and after each iteration, facilitating mathematical verification. [Chapter 12: Contracts and Preconditions](#chapter-12-contracts-and-preconditions-design-by-contract).
- **Capability:** Explicit permission clause (`uses FileSystem`, `uses Database`) without which code cannot perform I/O or access external resources. [Chapter 33: Capabilities](#chapter-33-safety-through-capabilities).
- **Sandboxing & Least Privilege:** The capability model limits system access to capabilities requested by functions and enabled for the project. [Chapter 33: Capabilities](#chapter-33-safety-through-capabilities).
- **Array (Type[]):** A homogeneous ordered collection of elements. Supports indexed access and standard utilities like `len()`, `append()`, `first()`, and `contains()`. [Chapter 13: Arrays & Maps](#chapter-13-collections-lists-and-dictionaries-arrays-maps).
- **Map (Map<K, V>):** Associative key-value dictionary. Querying via `get(map, key)` always returns a safe `Option<V>`, eliminating crashes on missing keys. [Chapter 13: Arrays & Maps](#chapter-13-collections-lists-and-dictionaries-arrays-maps).
- **Immutable Collections:** Immutable collection operations do not change the existing value directly; they return a new value for a change. [Chapter 13: Arrays & Maps](#chapter-13-collections-lists-and-dictionaries-arrays-maps).
- **for .. in Iteration:** A loop form that visits collection elements in sequence and avoids manual index management. [Chapter 10: Repetition and Loops](#chapter-10-repetition-and-loops).
- **Table Schema (table):** Direct relational table declaration in Zelyra syntax including types, primary keys, uniqueness constraints, and default values. [Chapter 24: Defining Tables](#chapter-24-defining-tables-and-data-modeling).
- **Migration (db plan & apply):** Controlled schema changes: `zelyra db plan` shows the difference between the source model and database; `zelyra db apply` applies the reviewed plan. Review possible effects first, since interrupted DDL can leave a partially changed state. [Chapter 24: Defining Tables](#chapter-24-defining-tables-and-data-modeling).
- **CRUD Resource (crud):** Create, Read, Update, Delete: Declarative resource binding generating query routines, forms, and layouts without repetitive boilerplate. [Chapter 28: CRUD Pattern](#chapter-28-the-complete-crud-pattern).
- **Native SQL (sql { ... }):** First-class SQL validated against schemas at compile-time. Delivers raw query performance and expressiveness without ORM friction. [Chapter 25: Querying Data](#chapter-25-querying-and-modifying-data).
- **Transaction (transaction):** Atomic database transaction block: all database operations succeed together or automatically roll back upon any failure. [Chapter 25: Querying Data](#chapter-25-querying-and-modifying-data).
- **Foreign Key (fk):** Declarative relational link between tables enforcing referential integrity and cascade rules directly at database level. [Chapter 24: Defining Tables](#chapter-24-defining-tables-and-data-modeling).
- **Component (component):** Reusable UI template unit with typed props, slots, and automated contextual HTML escaping against XSS vulnerabilities. [Chapter 26: Rendering Web Pages](#chapter-26-rendering-web-pages).
- **Slot (slot: name):** A named injection slot in container components enabling flexible layout composition with dynamic child elements. [Chapter 26: Rendering Web Pages](#chapter-26-rendering-web-pages).
- **Form & Validation (form):** Declarative web form specification providing unified server and browser validation with structured error feedback. [Chapter 27: Forms](#chapter-27-forms-and-user-inputs).
- **CSRF Protection:** Built-in cryptographic token mechanism preventing Cross-Site Request Forgery on all state-modifying requests. [Chapter 27: Forms](#chapter-27-forms-and-user-inputs).
- **Route:** Explicit binding of HTTP method and URL endpoint to an executable controller handler or CRUD resource. [Chapter 26: Rendering Web Pages](#chapter-26-rendering-web-pages).
- **Authentication (auth):** Native declarative block configuring user login, session lifecycle, and credentials without third-party frameworks. [Chapter 29: Authentication](#chapter-29-users-passwords-and-sessions).
- **Argon2 Password Hashing:** State-of-the-art memory-hard hashing algorithm natively enforced across Zelyra authentication modules. [Chapter 29: Authentication](#chapter-29-users-passwords-and-sessions).
- **requires auth & permits:** Declarative route authorization attribute restricting endpoints to logged-in users or specific privilege roles. [Chapter 29: Authentication](#chapter-29-users-passwords-and-sessions).
- **uses FileSystem:** Security capability governing storage I/O. Functions reading or writing files must declare this permission at the boundary. [Chapter 20: Files](#chapter-20-working-with-files).
- **read_text & write_text:** Standard library utilities for atomic, safe UTF-8 file reading and writing with automatic resource closing. [Chapter 20: Files](#chapter-20-working-with-files).
- **uses Clock & now():** System time capability: querying current timestamps via `now()` requires `uses Clock`, enabling deterministic mocking in automated tests. [Chapter 21: Date & Time](#chapter-21-date-time-randomness-and-structured-data).
- **uses Random & random_int():** Randomness capability: grants access to pseudo-random generators while keeping pure algorithms cleanly separated. [Chapter 21: Date & Time](#chapter-21-date-time-randomness-and-structured-data).
- **uses Environment & env():** Environment variable capability: `env("KEY")` queries runtime configuration or `.env` and yields a null-safe `Option<String>`. [Chapter 37: Configuration](#chapter-37-configuration-and-environment-variables).
- **parallel & await:** A `parallel` block starts independent tasks and uses `await` to wait for their results. The task scope remains visible in the block. [Chapter 22: Concurrency](#chapter-22-concurrency-and-background-tasks).
- **json_encode & json_decode:** Type-safe JSON serialization and deserialization yielding explicit `Result<T, JsonError>` containers. [Chapter 21: JSON & Data](#chapter-21-date-time-randomness-and-structured-data).
- **zelyra.toml (Project Manifest):** Project manifest for metadata, capability grants, and supported project settings. Zelyra modules are included by project path; a package registry is outside the documented scope. [Chapter 15: Modules & Projects](#chapter-15-modules-and-code-organization).
- **zelyra verify (Contract Prover):** CLI command for the documented contract and invariant checks. Formal analysis is limited to the supported verification scope and does not guarantee overall program correctness. [Chapter 19: QA & Testing](#chapter-19-testing-and-quality-assurance).
- **assert (Test Assertion):** Checks a condition and reports an error when it is false. Project-specific assertions can run in an executable `.zyl` file with `zelyra run`; a separate test subcommand is not documented. [Chapter 19: QA & Testing](#chapter-19-testing-and-quality-assurance).
- **Source Authority Principle:** A rule for resolving conflicting sources. The handbook reference orders them as language specification, compiler implementation, official tests, standard library, verified examples, and documentation. [Ref 24: Source Authority](#24-authoritative-sources-and-compiler-verification-source-authority).
- **Three-Layer Architecture:** Canonical Zelyra architectural pattern strictly isolating UI presentation, pure business logic, and schema persistence. [Chapter 36: Architecture](#chapter-36-architecture-and-clean-code-structure).
- **TDD (Test-Driven Development):** Methodology where formal contracts and automated test assertions precede the actual function implementation. [Chapter 19: QA & Testing](#chapter-19-testing-and-quality-assurance).
- **Regression Prevention:** Prevention of bugs reappearing after refactoring. Static checks, contract verification, and project-specific test runs help catch unintended changes. [Chapter 19: QA & Testing](#chapter-19-testing-and-quality-assurance).
- **AST (Abstract Syntax Tree):** The hierarchical syntax tree built by the parser from source code. Compiler phases such as name resolution and type checking continue from this structured representation. [Chapter 24: Source Authority](#24-authoritative-sources-and-compiler-verification-source-authority).
- **Zelyra CLI (zelyra):** The command-line interface for Zelyra projects: it creates projects with `new`, checks source with `check`, runs programs with `run`, and provides other documented subcommands. The current CLI reference does not list a dedicated `test` subcommand. [Chapter 4: The First Zelyra Project](#chapter-4-the-first-zelyra-project).
- **zelyrac (Compiler):** The compiler processes Zelyra source behind the `zelyra` CLI. The current CLI reference does not document a separate `zelyrac` executable or `zelyrac build`; use `zelyra check` and `zelyra run` to check and execute programs. [Chapter 4: The First Zelyra Project](#chapter-4-the-first-zelyra-project).
- **zelyra fmt (Formatter):** Built-in source formatter with deterministic output. `--check` checks formatting without changing the file. [Chapter 4: The First Zelyra Project](#chapter-4-the-first-zelyra-project).
- **LSP (Language Server Protocol):** The Language Server Protocol standardizes communication between a language and an editor. The current CLI reference documents `zelyra editor` for Studio, but no separate `zelyra lsp` subcommand. [Chapter 4: The First Zelyra Project](#chapter-4-the-first-zelyra-project).
- **AI-Native Language Design:** Language and tooling features such as structured diagnostics, contracts, and typed holes can support AI-assisted work. They do not guarantee error-free code or replace human review. [Chapter 32: AI-Nativity](#chapter-32-ai-nativity-why-zelyra-is-built-for-ai-assistants).
- **Standard Library (std):** Overview of documented standard functions and capability-guarded functions such as filesystem, time, randomness, and database access. [Appendix D: Standard Library](#appendix-d-standard-library-overview-zelyra-103).

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

**Question: How do I use imports?**
*Answer:* `import "src/models.zyl" as models` includes a project-local source file. Access supported public declarations through the module name. A package registry, dynamic imports, and complete deployment closure are not included.

**Question: Can I build command-line applications with Zelyra?**
*Answer:* Yes. `print()` emits values. `read_console("Prompt: ")` reads a line and returns `String?`; the function needs `uses Console` and the project may also need `console = true`.

**Question: Why does Zelyra emphasize MariaDB as its primary database engine?**
*Answer:* MariaDB provides exceptional transactional performance, open-source licensing, robust cloud compatibility, and rock-solid reliability for enterprise web applications.

---

## Appendix J: Next Resources and Community

- **Official GitHub Repository:** [https://github.com/sf1976/zelyra](https://github.com/sf1976/zelyra)
- **Documentation & Online Handbook:** [https://siedelmann.com/handbook](https://siedelmann.com/handbook) / [https://siedelmann.com/handbuch](https://siedelmann.com/handbuch)
- **Examples & Templates:** Check the `examples/` directory in the official repository for runnable templates covering authentication, CRUD views, REST APIs, and database migrations.

---

## Appendix K: The Zelyra Cookbook (Practical Recipes)

The Zelyra Cookbook provides a curated collection of proven patterns, architectural recipes, and concrete solutions for real-world business and web engineering challenges. Each recipe follows a practical three-tier format: **Problem**, **Solution** (contract-verified Zelyra code), and **Explanation**.

---

### Part 1: Data Processing & Standard Library

#### Recipe K.1: Exact Commercial Rounding & Cash Discount Calculations

**Problem:**
In commercial and accounting applications, binary floating-point numbers (`Float`) introduce rounding errors into discounts, VAT splits, and ledger totals.

**Solution:**
```zelyra
pub record DiscountResult {
    base_net: Decimal,
    discount_rate: Decimal,
    discount_amount: Decimal,
    final_net: Decimal,
}

pub fn apply_cash_discount(amount: Decimal, rate: Decimal) -> DiscountResult
    requires { amount >= decimal("0.00") && rate >= decimal("0.00") && rate <= decimal("100.00") }
    ensures { result.final_net <= result.base_net && result.discount_amount >= decimal("0.00") }
{
    let discount = (amount * rate) / decimal("100.00")
    let final_amount = amount - discount
    return DiscountResult {
        base_net: amount,
        discount_rate: rate,
        discount_amount: discount,
        final_net: final_amount,
    }
}
```

**Explanation:**
- Zelyra's `Decimal` type implements high-precision fixed-point arithmetic without binary representation errors.
- The precondition contract `requires` guarantees that negative discount rates or values above 100% cannot be passed into the function.
- The postcondition `ensures` verifies mathematically that the final discounted total can never exceed the starting net amount.

---

#### Recipe K.2: Safely Parsing & Typing External Webhook Payloads

**Problem:**
An external payment provider (such as Stripe or PayPal) transmits event notifications via HTTP POST webhooks. Missing or malicious attributes must be rejected with full type safety.

**Solution:**
```zelyra
pub enum PaymentStatus {
    Paid,
    Pending,
    Failed,
}

pub record WebhookPayload {
    event_id: String,
    payment_reference: String,
    amount: Decimal,
    status: PaymentStatus,
}

pub fn parse_payment_status(raw: String) -> Option<PaymentStatus> {
    match raw {
        "succeeded" => { return Option::Some(PaymentStatus::Paid) }
        "processing" => { return Option::Some(PaymentStatus::Pending) }
        "requires_payment_method" => { return Option::Some(PaymentStatus::Failed) }
        _ => { return Option::None }
    }
}
```

**Explanation:**
- Zelyra requires exhaustive `match` checking. Unrecognized status payloads hit the fallback wildcard `_` and return `Option::None`, eliminating unhandled runtime crashes.

---

#### Recipe K.3: Computing Payment Due Dates and Business Days

**Problem:**
An invoice requires a 14-day payment window. If the due date falls on a weekend, the payment deadline must automatically shift to the subsequent business day (Monday).

**Solution:**
```zelyra
pub fn calculate_due_date(invoice_date: Date, net_days: Int) -> Date
    requires { net_days >= 0 }
{
    let target = invoice_date.add_days(net_days)
    let weekday = target.day_of_week()
    if weekday == 6 {
        return target.add_days(2)
    } else if weekday == 7 {
        return target.add_days(1)
    } else {
        return target
    }
}
```

**Explanation:**
- Calendar arithmetic in Zelyra operates on typed `Date` records and accounts for calendar months and leap years without timezone ambiguity.

---

### Part 2: Database, Transactions & Multi-Tenancy

#### Recipe K.4: Gapless Invoice Number Sequences (Atomic Sequence)

**Problem:**
Tax authorities and accounting regulations require gapless, strictly sequential document numbering. Under high concurrency, naive increments create race conditions and duplicates.

**Solution:**
```zelyra
table invoice_sequences scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    fiscal_year: Int required
    current_value: Int required default 0

    unique within tenant fiscal_year
}

fn next_invoice_number(t_id: TenantId, year: Int) -> String uses Database {
    transaction {
        let seq = query_one(
            "SELECT id, current_value FROM invoice_sequences WHERE tenant_id = ? AND fiscal_year = ? FOR UPDATE",
            [t_id, year]
        )
        let next_val = seq.current_value + 1
        execute(
            "UPDATE invoice_sequences SET current_value = ? WHERE id = ?",
            [next_val, seq.id]
        )
        return "INV-" + year.to_string() + "-" + next_val.to_string()
    }
}
```

**Explanation:**
- The `FOR UPDATE` query inside an explicit `transaction` block acquires an exclusive row lock.
- `scoped by tenant_id` paired with `unique within tenant fiscal_year` isolates number series between separate tenants and fiscal years.

---

#### Recipe K.5: Immutable Audit Trail (Temporal Ledger)

**Problem:**
Mutations to sensitive business data (prices, bank routing numbers, administrative permissions) must be recorded permanently with the acting user and timestamp.

**Solution:**
```zelyra
table audit_ledger scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    actor_id: Id required
    entity_name: String(64) required
    entity_id: Id required
    action_type: String(32) required
    payload_hash: String(64) required
    created_at: Instant required
}

fn record_audit_entry(
    t_id: TenantId,
    actor: Id,
    entity: String,
    target_id: Id,
    action: String,
    raw_payload: String
) uses Database, Clock {
    let now_ts = now()
    let hash = hash_sha256(raw_payload)
    execute(
        "INSERT INTO audit_ledger (tenant_id, actor_id, entity_name, entity_id, action_type, payload_hash, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        [t_id, actor, entity, target_id, action, hash, now_ts]
    )
}
```

**Explanation:**
- Explicit capabilities `Database` and `Clock` guarantee reproducible unit tests via virtual clock mocks while recording strict UTC timestamps in production.
- Computing a SHA-256 hash fixes the payload cryptographically for forensic audits.

---

#### Recipe K.6: High-Performance Keyset Pagination for Large Datasets

**Problem:**
On tables with millions of rows, `OFFSET`-based pagination (`SELECT ... LIMIT 50 OFFSET 500000`) degrades query performance because the engine must traverse all preceding rows.

**Solution:**
```zelyra
table customer_logs scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    created_at: Instant required
    message: String(255) required

    index idx_tenant_cursor (tenant_id, created_at, id)
}

fn fetch_log_page(t_id: TenantId, last_ts: Instant, last_id: Id, page_size: Int) -> CustomerLog[] uses Database {
    return query(
        "SELECT id, tenant_id, created_at, message FROM customer_logs WHERE tenant_id = ? AND (created_at < ? OR (created_at = ? AND id < ?)) ORDER BY created_at DESC, id DESC LIMIT ?",
        [t_id, last_ts, last_ts, last_id, page_size]
    )
}
```

**Explanation:**
- Keyset (cursor) pagination uses the compound index `(tenant_id, created_at, id)` to seek directly to the next page, maintaining sub-millisecond execution regardless of table size.

---

### Part 3: Security, Authentication & Governance

#### Recipe K.7: Two-Factor Authentication with TOTP & Recovery Codes

**Problem:**
Sensitive user profiles must enforce RFC 6238 time-based one-time passwords (TOTP) alongside cryptographic recovery codes.

**Solution:**
```zelyra
table user_mfa_settings scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    user_id: Id required unique
    secret_key: String(64) required
    is_enabled: Bool required default false
    backup_code_hash: String(128) required
}

fn verify_totp_attempt(user_secret: String, user_token: String) -> Bool uses Clock {
    let valid = totp_verify(user_secret, user_token, 30)
    return valid
}
```

**Explanation:**
- `totp_verify` accepts a 30-second skew window (±1 step) to accommodate minor drift between client mobile authenticators and servers.

---

#### Recipe K.8: Tenant-Bound API with Role Verification

**Problem:**
A JSON API approving accounting vouchers must ensure that users can only mutate records belonging to their own tenant and strictly possess the `vouchers.approve` permission.

**Solution:**
```zelyra
table vouchers scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    voucher_number: String(40) required
    status: String(20) required default "draft"
    total_amount: Decimal(12, 2) required
}

api approve_voucher(voucher_id: Id) scoped by tenant_id {
    permission: "vouchers.approve"
    validates {
        voucher_id > 0: "Invalid voucher ID."
    }

    execute uses Database {
        transaction {
            let row = query_one(
                "SELECT id, status FROM vouchers WHERE id = ? AND tenant_id = ? FOR UPDATE",
                [voucher_id, session.tenant_id]
            )
            if row.status == "approved" {
                return { success: false, error: "Voucher has already been approved." }
            }
            execute(
                "UPDATE vouchers SET status = 'approved' WHERE id = ? AND tenant_id = ?",
                [voucher_id, session.tenant_id]
            )
            return { success: true, message: "Voucher approved successfully." }
        }
    }
}
```

**Explanation:**
- `scoped by tenant_id` guarantees that requests are bound to the caller's authenticated tenant (`session.tenant_id`), rendering cross-tenant data leakage impossible at compile time.

---

#### Recipe K.9: Rate Limiting & Brute-Force Defense

**Problem:**
Authentication and credential endpoints require rate limiting to prevent brute-force attacks and credential stuffing.

**Solution:**
```zelyra
table rate_limits {
    id: Id primary auto
    client_ip: String(45) required
    action: String(30) required
    attempts: Int required default 1
    locked_until: Instant?

    index idx_client_action (client_ip, action)
}

fn check_rate_limit(ip: String, action_name: String, max_attempts: Int) -> Bool uses Database, Clock {
    let now_ts = now()
    let record = query_one_optional(
        "SELECT id, attempts, locked_until FROM rate_limits WHERE client_ip = ? AND action = ?",
        [ip, action_name]
    )
    match record {
        Option::Some(r) => {
            if r.attempts >= max_attempts {
                return false
            }
            execute("UPDATE rate_limits SET attempts = attempts + 1 WHERE id = ?", [r.id])
            return true
        }
        Option::None => {
            execute("INSERT INTO rate_limits (client_ip, action, attempts) VALUES (?, ?, 1)", [ip, action_name])
            return true
        }
    }
}
```

**Explanation:**
- Once `max_attempts` is reached, further invocations are blocked immediately, mitigating credential stuffing and conserving backend resources.

---

### Part 4: Web, CRUD & Master-Detail

#### Recipe K.10: Master-Detail Invoice Form with Line Items (`has_many`)

**Problem:**
An invoice requires header fields (customer, invoice date) alongside a variable collection of line items, committed atomically with server-calculated summary aggregates.

**Solution:**
```zelyra
table invoices scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    invoice_number: String(32) required unique within tenant
    invoice_date: Date required
    customer_name: String(100) required
    total_net: Decimal(12, 2) required default decimal("0.00")
    total_gross: Decimal(12, 2) required default decimal("0.00")
}

table invoice_items scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    invoice: Invoice required on_delete cascade
    description: String(120) required
    quantity: Decimal(8, 2) required
    unit_price: Decimal(10, 2) required
    line_total: Decimal(12, 2) required
}

crud Invoice -> invoices {
    title: "Create Invoice"

    has_many items: InvoiceItem {
        title: "Line Items"
        fields: [description, quantity, unit_price, line_total]
    }

    aggregates {
        total_net = sum(items.line_total)
        total_gross = sum(items.line_total) * decimal("1.19")
    }
}
```

**Explanation:**
- `has_many items` generates child item grids directly within the parent form.
- The `aggregates` block computes monetary sums inside the storage transaction on the server, preventing client-side price tampering.

---

#### Recipe K.11: Lifecycle Guards for Immutable Finalized Documents

**Problem:**
Invoices that have been finalized or paid must be permanently protected against subsequent editing or deletion.

**Solution:**
```zelyra
table sales_invoices scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    doc_number: String(30) required
    is_finalized: Bool required default false
    is_cancelled: Bool required default false
}

crud SalesInvoice -> sales_invoices {
    title: "Sales Invoices"

    guard edit when is_finalized == true {
        error: "Finalized documents cannot be edited. Create a cancellation credit note instead."
    }

    guard delete when is_finalized == true {
        error: "Finalized invoices cannot be deleted under regulatory compliance rules."
    }
}
```

**Explanation:**
- Zelyra evaluates guards within the database mutation transaction.
- In addition, generated `tableview` action columns automatically hide or disable edit and delete actions when the condition evaluates to `true`.

---

#### Recipe K.12: Cross-Field Invariant Enforcement (`validate`)

**Problem:**
In hotel and rental reservations, the departure date must always be after the arrival date.

**Solution:**
```zelyra
table hotel_bookings scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    guest_name: String(100) required
    check_in: Date required
    check_out: Date required
    rooms_count: Int required default 1

    validate {
        check_out > check_in: "Check-out date must be after check-in date."
        rooms_count >= 1 && rooms_count <= 10: "Between 1 and 10 rooms can be booked per reservation."
    }
}
```

**Explanation:**
- Invariants are enforced across forms, CRUD actions, and API mutations before committing writes, returning standardized HTTP 422 errors and setting `aria-invalid="true"`.

---

### Part 5: Background Services, Email & Export

#### Recipe K.13: Transactional PDF Invoicing & Email Outbox

**Problem:**
When an order completes, an official PDF receipt must be generated and queued for email delivery without risking orphaned emails if the database transaction rolls back.

**Solution:**
```zelyra
fn finalize_order(order_id: Id, recipient_mail: String) uses Database {
    transaction {
        let order = query_one("SELECT id, order_number, total_gross FROM orders WHERE id = ?", [order_id])
        let pdf_bytes = render_pdf("templates/OrderConfirmation.html", order)
        mail_queue(
            recipient_mail,
            "Your Order Confirmation " + order.order_number,
            "Thank you for your order. Please find your official confirmation attached.",
            order.order_number + ".pdf",
            pdf_bytes
        )
        execute("UPDATE orders SET status = 'confirmed' WHERE id = ?", [order_id])
    }
}
```

**Explanation:**
- `mail_queue` enqueues the message directly into MariaDB's `mail_queue` table inside the active transaction. If the transaction rolls back, no email is sent.

---

#### Recipe K.14: Idempotent Background Cron Jobs with Renewable Leases

**Problem:**
A nightly reminder job must execute at 02:00 UTC without duplicate processing across clustered servers.

**Solution:**
```zelyra
schedule "0 2 * * *" {
    name: "nightly_payment_reminders"
    timeout: 3600

    execute uses Database, Clock {
        transaction {
            let pending_invoices = query(
                "SELECT id, customer_mail, due_date FROM invoices WHERE status = 'open' AND due_date < CURRENT_DATE() FOR UPDATE SKIP LOCKED",
                []
            )
            for inv in pending_invoices {
                execute("UPDATE invoices SET status = 'reminded' WHERE id = ?", [inv.id])
            }
        }
    }
}
```

**Explanation:**
- Combining `FOR UPDATE SKIP LOCKED` with Zelyra's leased scheduler ensures that multiple server nodes work concurrently without overlapping task execution.

---

#### Recipe K.15: Streaming CSV Export for Accounting Integration

**Problem:**
Exporting accounting journals to CSV for tax advisors must maintain low memory usage even with tens of thousands of records.

**Solution:**
```zelyra
fn export_datev_csv(t_id: TenantId, year: Int) -> String uses Database {
    mutable csv_output = "Date;Voucher;Account;ContraAccount;Amount;Notice\n"
    let entries = query(
        "SELECT booking_date, voucher_no, account, contra_account, amount, notice FROM bookings WHERE tenant_id = ? AND fiscal_year = ? ORDER BY booking_date ASC",
        [t_id, year]
    )
    for row in entries {
        csv_output = csv_output + row.booking_date.to_string() + ";" + row.voucher_no + ";" + row.account.to_string() + ";" + row.contra_account.to_string() + ";" + row.amount.to_string() + ";" + row.notice + "\n"
    }
    return csv_output
}
```

**Explanation:**
- Explicit types guarantee that numerical values and dates are exported using deterministic formatting standards.
