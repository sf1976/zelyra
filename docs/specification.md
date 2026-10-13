# Zelyra 0.1 specification index

The repository's implementation specification is maintained across the phase
documents, the bilingual handbook, and the roadmap. This file defines the
cross-cutting AI-native contract that applies to all of them.

The language compatibility declaration and policy are described in the
[language compatibility guide](language-compatibility.en.md).

## AI-native, AI-independent development

> AI writes. Zelyra verifies.

Zelyra accepts people and AI systems as code authors without trusting either
author automatically. The compiler and tests decide whether source is valid.
The language remains fully usable without a model, provider, cloud service, or
network connection.

AI-facing interfaces are open, deterministic, machine-readable, versioned,
local-capable, and vendor-neutral. The current machine-oriented interfaces are:

```text
zelyra check <file.zyl> --format=json
zelyra context <file.zyl> --format=json
zelyra impact <file.zyl> --format=json
zelyra impact <file.zyl> --symbol <kind:name> --format=json
zelyra edit --format=json change.json
zelyra fmt <file.zyl> --check
```

The JSON interfaces use machine schema version `1`. JSON is stdout-only; logs are stderr-only.
Diagnostics have stable codes and half-open source spans using zero-based UTF-8
byte offsets and one-based line/byte columns. Outputs are deterministic and do
not contain secrets, timestamps, random IDs, absolute paths, or live database
contents.

The compiler must continue to enforce names, types, nullability, SQL, schemas,
forms, views, APIs, permissions, contracts, capabilities, tests, and
destructive-change approvals. AI output may not silently add capabilities,
expand permissions, execute destructive SQL, weaken checks, or expose secrets.
Expression typed holes, source-only impact analysis with optional focused
queries, and versioned semantic rename previews are implemented slices;
declaration-context holes, complete
runtime/schema impact, richer edit operations, granular effects, and
benchmarks remain roadmap work.

String and character literals preserve valid Unicode characters as written.
Source spans continue to use UTF-8 byte offsets.

## Comments in language line 0.1

`#` starts a line comment through the end of the line. A standalone `##` line
opens a block comment; the next standalone `##` line closes it. Block comments
do not nest. If a closing delimiter is missing, the lexer reports an error at
the opening delimiter. The older `//` line-comment syntax remains available
for valid existing programs; new Zelyra source uses `#` and `##`.

## Interactive console input

CLI programs read one line with `read_console(prompt: String) -> String?`.
The function writes the prompt to `stdout` and flushes it before waiting on
`stdin`. It removes a line ending (`LF` or `CRLF`) and preserves all other
whitespace. An empty line is `Some("")`; end of input is `None`. An I/O
failure is reported as a runtime error. Terminal input is not hidden, so do
not use `read_console` for passwords or other secrets.

Access is explicit: the enclosing function must declare `uses Console`, and a
project with a `[capabilities]` section must set `console = true`. New project
templates leave this project grant disabled. `.env` cannot grant a capability.
Interactive input is intended for `zelyra run`; web applications receive input
through typed requests and forms.

~~~zelyra
fn main() uses Console {
    input = read_console("Date: ")
    match input {
        Some(date) => {
            print("Entered: " + date)
        }
        None => {
            print("No input.")
        }
    }
}
~~~

## Localized web interface and learning level

The built-in web UI is catalog-driven. German and English UI copy is maintained
in `web/locales/de.json` and `web/locales/en.json`; both files must keep the
same keys. CRUD, form, authentication, validation, standard HTTP-error, and
learning-guide copy is rendered from these catalogs. The bundled
machine-management view marks catalog-backed HTML with
`data-zelyra-i18n="app.home_title"`; configurable Zelyra text may use
`@i18n:app.home_title`.

`ZELYRA_LANGUAGE=de|en` selects the UI catalog. `ZELYRA_LEVEL=learn|work`
shows or hides the contextual learning guide; it never grants capabilities or
permissions. For `zelyra serve`, process variables override the project's
`.env`. New MariaDB projects default to `de`/`learn`; direct serving without
either value defaults to `en`/`work`. These are presentation settings only.
Generated CRUD, standalone form, tableview, login, and authentication-admin
pages receive the responsive Zelyra application shell when no explicit custom
layout is selected. The shell provides labeled navigation, keyboard focus
styles, and a localized skip-to-content link. Authored page HTML remains under
the page author's control.

Missing German entries fall back to English. A key missing from both built-in
catalogs renders `[missing translation]`; a consistency test checks literal
references against both catalogs. `zelyra new` and `zelyra init` create
optional `locales/de.json` and `locales/en.json` project catalogs. These can
add keys or override built-in keys when referenced by
`data-zelyra-i18n="key"`, `@i18n:key` text settings, or catalog-backed standard
HTTP errors. The same project catalogs can override every catalog-backed
generated shell, CRUD, form, tableview, login, authentication-admin,
validation, and learning-guide label. Generated field identifiers and
parameterized labels (for example, a filter label containing a field name) are
also resolved through the catalogs. German lookup uses the project German
catalog, then project English, then the built-in German catalog and its English
fallback. Files must be UTF-8 JSON objects with nonempty string values and are
limited to 256 KiB each; invalid files and symbolic links are rejected by
`serve`. Resolved text is HTML-escaped. Business records and unmarked
project-authored content are not automatically translated. API and compiler
machine contracts are not localized by these settings.

### Browser request origins and allowed hosts

Every supplied HTTP `Host` header must match the server's configured
`ZELYRA_ALLOWED_HOSTS`. The default is `localhost,127.0.0.1,[::1]`; configure
additional actual hostnames or IP addresses explicitly for a LAN or reverse
proxy. Values are comma-separated ASCII hostnames or IP addresses (IDNs use
punycode) and must not contain schemes, ports, wildcards, or empty entries.
Precedence is process environment, project `.env`, then the
loopback-only default. Invalid or empty settings prevent server startup. This
allowlist rejects forged hosts and DNS-rebinding requests; it does not provide
TLS or replace origin checks. Duplicate `Host`, `Origin`, `Referer`,
`X-Forwarded-Proto`, `Cookie`, and `Authorization` request headers are rejected
to avoid ambiguous security decisions. Responses use
`Referrer-Policy: same-origin`, allowing same-origin API GETs to provide a
`Referer` while not sending referrer information to other origins.

State-changing browser forms require both a valid CSRF token and a same-origin
`Origin` or `Referer` matching the request host and effective scheme. API
requests carrying browser-origin headers or the Zelyra session cookie receive
the origin check for routed API methods, including `GET`, because handlers are
not yet statically restricted to read-only behavior. `OPTIONS` preflight is
handled separately by the CORS policy. Cross-origin API requests are allowed
only when their exact origin is configured in the CORS policy; a session cookie
additionally requires credentialed CORS. Behind a TLS-terminating
proxy, preserve the public `Host`, overwrite `X-Forwarded-Proto`, and block
direct access to the application port. The built-in server does not terminate
TLS.

### Project-local theme overrides

Projects may customize the built-in visual design with a root-level
`zelyra.theme.css`. `zelyra serve` loads it from the directory containing the
selected `.zyl` file and links it after Zelyra's built-in stylesheet on branded
pages. Missing files leave the default design unchanged. `zelyra new` and
`zelyra init` scaffold a commented starter file, and generated Dockerfiles copy
it into the runtime image.

The stylesheet may override documented `--zelyra-*` design tokens or add
project CSS. Zelyra does not parse or type-check CSS. It is public browser
content, is limited to 128 KiB of UTF-8, and must be a regular non-symlink
file. It is served as `text/css` at the reserved GET-only route
`/__zelyra/theme.css`; when a theme file exists, pages and APIs may not claim
that route. Do not put secrets in the stylesheet. Browser requests introduced
by CSS such as external `url(...)` or `@import` are controlled by the project
author, not by the Zelyra server's `Network` capability.

## Typed page data loading

Pages may declare one or more explicit record loads using the native SQL
boundary:

~~~zelyra
page "/customers/{name}" {
    load customer = sql<Customer> {
        SELECT id, name FROM customers WHERE name = :name
    }
    html { <h1>{customer.name}</h1> }
}
~~~

Collection results use an array result type and a typed server-side loop:

~~~zelyra
page "/customers" {
    load customers = sql<Customer[]> { SELECT id, name FROM customers }
    html { <ul>for customer in customers { <li>{customer.name}</li> }</ul> }
}
~~~

The compiler checks the query against the declared schema, exposes route
parameters as typed SQL parameters, and validates record-field interpolations.
At runtime authorization and the `Database` capability are checked before the
query. Results are rendered with HTML escaping; missing required records and
database failures produce generic HTTP boundaries without exposing database
details. Collection loops are supported for arrays of records. Optional-aware
field expressions and richer view-local data remain planned.

Function-level SQL may declare `uses Database(read)` for classified `SELECT`
statements or `uses Database(write)` for classified `INSERT`, `UPDATE`, and
`DELETE` statements. Unclassified statements require `uses Database`. The
broad form remains compatible with existing programs. Projects may grant the
scoped effects independently through `database_read` and `database_write` in
`zelyra.toml`; these grants do not change MariaDB account permissions.

### Reusable view layouts

Named views are deterministic page layouts. Each view must declare exactly one
default `<slot />`; it may additionally declare each named slot once. Named
slots can contain fallback HTML:

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

The page's named slot blocks must refer to slots declared by the selected view;
duplicate blocks and unknown slots are compile-time errors. Unprovided named
slots use their declared fallback. Composition happens before component
expansion and routing, without global view state; authentication, capability
checks, SQL checks, and HTML escaping remain active.

Generated CRUD, form, and tableview pages use the built-in Zelyra application
shell by default. A CRUD's explicit `layout: ViewName` replaces that default
shell with the checked project view. Authored pages are never silently
rewritten and may choose their own page-level view.

A CRUD may supply content for declared named slots in that outer layout:

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
        resource_heading {
            html { <h1>Machine register</h1> }
        }
        resource_help {
            html { <p>Use search and filters to find a machine.</p> }
        }
    }
}
~~~

The compiler rejects an unknown layout, an undeclared slot, duplicate slot
content, or slot content without a layout (diagnostic E-VIEW-031). Omitted
named slots keep the layout's fallback. The layout's default slot remains
reserved for generated CRUD output; custom slot markup can use checked
components but has no record, request, or CRUD-action bindings. The generated
CRUD continues to own its SQL, validation, CSRF, authorization, and escaping
boundaries.

CRUD resources may reuse a project-defined application shell with
`layout: ViewName`:

~~~zelyra
crud Customer -> customers {
    layout: AppShell
}
~~~

The referenced view is checked like a page view and must exist. Its default
slot receives the generated CRUD list, detail, and generated create/edit or
custom-action form content. Named slots keep their declared fallback content.
Composition is deterministic and does not replace generated SQL, validation,
CSRF, authentication, authorization, or HTML escaping. Redirect responses
remain redirects and are never wrapped as page HTML.

Pages may declare typed query inputs:

~~~zelyra
page "/customers" {
    input { search: String? }
    load customers = sql<Customer[]> {
        SELECT id, name FROM customers
        WHERE (:search IS NULL OR name LIKE CONCAT('%', :search, '%'))
    }
    html { <p>{search}</p> }
}
~~~

Query inputs are available to page SQL and HTML interpolation. The compiler
checks their declared scalar type, and the runtime binds decoded URL values as
database parameters. Missing optional inputs are bound as SQL `NULL`; missing
required inputs and invalid scalar values produce HTTP 400. Automatic controls
for declared page collections are described below; arbitrary input-only pages
remain manual.

Collection pages may opt into server-side pagination:

~~~zelyra
page "/customers" {
    paginated 25
    load customers = sql<Customer[]> { SELECT id, name FROM customers }
    html { <p>{page}</p> }
}
~~~

The `paginated` size must be between 1 and 100. The optional `page` URL value
is a positive integer with default `1` and is exposed to page HTML as `UInt`.
The runtime applies a parameterized `LIMIT`/`OFFSET` wrapper to collection
queries. Pagination of arbitrary record loads is rejected by the compiler.

Collection pages may also declare a sort whitelist:

~~~zelyra
page "/customers" {
    sort { name }
    load customers = sql<Customer[]> { SELECT id, name FROM customers }
    html { <p>{sort} {order}</p> }
}
~~~

The runtime accepts only declared result fields for `sort` and only `asc` or
`desc` for `order`. Identifiers are quoted after compiler validation; URL
values are never concatenated into SQL as unchecked identifiers. Sorting and
pagination may be combined.

Collection pages may declare searchable result fields as well:

~~~zelyra
page "/customers" {
    search { name email }
    load customers = sql<Customer[]> { SELECT id, name, email FROM customers }
    html { <p>{search}</p> }
}
~~~

The `search` URL value is bound as a parameter and applied to the declared
fields with server-side `LIKE` conditions. Empty search values do not add a
condition; unknown search fields are compiler errors.

Collection pages may also declare typed filters:

~~~zelyra
page "/customers" {
    filter { name quantity }
    load customers = sql<Customer[]> { SELECT id, name, quantity FROM customers }
    html { <p>{filter_name}</p> }
}
~~~

The compiler checks every filter field against the collection result type. The
runtime accepts only the operators supported by that type: text fields support
`eq`, `contains`, `starts_with`, `ends_with`, and null checks; numeric fields
also support `gt`, `gte`, `lt`, and `lte`; booleans and other values support
equality and null checks. Values are bound parameters, while field names and
operators are validated against the declaration. URL examples are
`/customers?filter_name__contains=Acme` and
`/customers?filter_quantity__gte=10`. Unsupported operators and undeclared
fields produce a controlled HTTP 400 response.

## Experimental project module imports

The current development branch has an experimental file-import slice. It is
not included in the published 0.3.0 release and is not yet a stable language
compatibility promise.

Imports use a path relative to the directory of the explicitly selected entry
source file. An alias is required, and imports must precede declarations:

~~~zelyra
import "src/math.zyl" as math

pub fn add(left: Int, right: Int) -> Int {
    return left + right
}

fn main() {
    print(math::add(2, 3))
}
~~~

An imported source currently may declare functions, type aliases, records,
tables, tableviews, named views, typed components, and one project-wide database
connection definition. Database configuration is
composed into the application and is not accessed through the import alias;
the composed project may define at most one database. Tables in imported files
are composed into the application's shared physical schema; table names are
global SQL identifiers rather than module-qualified names, and duplicate table
names are rejected. Imported tableviews, views, and components join the
composed application under their declared, unqualified names. Like other
module resources, they are private by default and exported with `pub view` or
`pub component`. Cross-module page and CRUD layout references, plus recognized
component tags in page, view, component, and CRUD slot HTML, require a direct
or transitive import path to the owner. Private UI references report
`E-MOD-007`; missing import edges report `E-MOD-020`. Component tags are
currently discovered by matching known names in HTML text; this is not a
complete HTML or namespace analysis. Duplicate resource names are rejected.
MariaDB-backed imported tableviews can be served as application routes; the
current tableview query runtime does not yet execute against SQLite. Imported
pages are composed into the application's route set; overlapping page route
patterns are rejected with the imported source location. Forms, CRUD
declarations, APIs, and authentication resources also compose from imported
files. Function, type, and record declarations are private by default;
declarations cross a module boundary only with a `pub` modifier and an explicit
import alias. For example, use `pub fn`, `pub type`, `pub struct`, `pub view`,
or `pub component`. A qualified type reference such as
`money::Amount` resolves through the current file's `money` import. Private
helpers and types remain available within their own file. Public signatures
may not expose private types. The compiler rejects missing files, duplicate
aliases, import cycles, path traversal, symlinks that resolve outside the
project root, and calls or type references to private or unknown declarations.
Module loading performs no network lookup.

The database declaration is project-wide configuration, not a module-qualified
resource. The alias is still required to include the file, but application
code does not refer to `storage::main`:

~~~zelyra
import "src/database.zyl" as storage

fn main() {}
~~~

~~~zelyra
database main {
    engine: mariadb
    database: "invoices"
}
~~~

For example, `src/money.zyl` may export a record and `src/invoice.zyl` may use
that public type in a function signature:

~~~zelyra
pub struct Money {
    cents: Int
}
~~~

~~~zelyra
import "src/money.zyl" as money

pub fn total() -> money::Money {
    return money::Money { cents: 2500 }
}
~~~

`check`, `build`, `run`, `serve`, `context`, and `verify` currently validate this
project graph. The machine-readable context document includes every reachable
module and its sorted import edges. Imported database definitions and tables
join the composed project schema; their context spans identify the
project-relative source path in `span.file`. Context also includes imported
views and components with their source spans. `serve` composes imported
views/components into pages and starts the application from the linked graph.
Some template-validation diagnostics still need more complete per-module source
attribution. `verify` checks linked functions, but verification results do not
yet preserve per-module source provenance. Pages, APIs, and other route-bound
application resources remain unsupported in imported files.
`fmt` and `edit` remain source-file-local. Database commands now load the
linked project graph when building the composed schema; their output and some
schema diagnostics still have incomplete module-level source attribution.
`impact` analyzes the linked module graph and attributes its spans to their
source files. These limits make the implementation experimental rather than a
complete multi-file project model. The full requirements are tracked in the
[0.4.0 release plan](release-plans/0.4.0.en.md).

When a page collection declares search, filters, sorting, or pagination,
Zelyra automatically renders a semantic query-control form before the page
content and preserves the current URL state. Pagination also performs a safe
count query and exposes `total` and `pages` as `UInt` page bindings. Pages that
use only explicit `input` declarations remain manual by design.

## Language and runtime additions in 0.9.8

The 0.9.8 development line adds tested language behavior without changing
existing syntax: a directly written `-9223372036854775808` is `Int::MIN`;
non-`Unit` functions must return a value on every reachable path; and
non-exhaustive enum matches are rejected. The interpreter bounds nested function
calls and reports the attempted call site before exceeding that limit. Extra
0.9.9 tests found that the original bound could still overflow smaller runtime
stacks, so the maximum is now 24 active frames. This conservative bound protects
Rust-backed execution stacks but does not remove recursion depth limits.

The line also adds unit enums, exclusive integer range loops, value-returning
`record_with` and `array_replace_result` updates, Gregorian calendar-day
helpers, UTC timestamp ordering and duration arithmetic, explicit fixed-point
Decimal quantization, and an optional `zelyra run --max-steps N` execution
budget. Date arithmetic is distinct from elapsed UTC durations; local time
zones and daylight-saving rules are not included. Decimal multiplication and
division still have nine fractional digits and may truncate beyond that
precision. The step budget is not process isolation and is unlimited unless
enabled. Payload-carrying enums are specified separately below.

## Generic functions in 0.9.9

The 0.9.9 development branch supports user-defined generic functions. Type
parameters follow the function name. Calls may infer them from arguments or
provide them explicitly:

~~~zelyra
fn identity<T>(value: T) -> T {
    return value
}

fn first_or<T>(values: T[], fallback: T) -> T {
    if len(values) > 0 { return values[0] }
    return fallback
}

fn main() {
    print(identity(42))
    print(identity<String>("Zelyra"))
    print(first_or(["Customer", "Item"], "unknown"))
}
~~~

A type parameter must be inferable from the arguments or supplied explicitly.
Repeated uses of one parameter must resolve to the same type; otherwise the
compiler reports `E-TYPE-001`. Uninferable types, an incorrect number of
explicit type arguments, and type parameters that shadow declared types use
`E-TYPE-GENERIC-001`. Generic functions may call other generic functions. The
type-system example covers inference, explicit types, arrays, `Option`, module
imports, and expected compiler errors. Generic functions, records, and enums are
available; general constraints remain outside the current scope.

### Generic user-defined types in 0.9.9

Records and enums may declare type parameters. Explicit type arguments use angle
brackets. For record literals and enum payloads, the compiler infers arguments
when every parameter can be derived from the fields or payload values:

~~~zelyra
struct Box<T> { value: T }
enum Maybe<T> { Some(T) None }

fn unwrap_or<T>(value: Maybe<T>, fallback: T) -> T {
    match value {
        Maybe::Some(found) => { return found }
        Maybe::None => { return fallback }
    }
}

fn main() {
    box: Box<Int> = Box<Int> { value: 7 }
    inferred = Box { value: "Item" }
    some = Maybe::Some(42)
    none: Maybe<String> = Maybe::None<String>()
    print(unwrap_or(some, 0))
    print(unwrap_or(none, "empty"))
}
~~~

A generic enum unit constructor has no payload from which to infer its type,
so it uses the form `Maybe::None<String>()`. In `match`, payload patterns are
checked against the enum's concrete type arguments, and every variant must
still be covered. `json_decode<Box<Int>>` and `json_decode<Maybe<Int>>` validate
payloads against the explicit target type. The existing JSON format gains no
type-argument fields. Generic records are not SQL column types and are rejected
explicitly at that boundary.

General constraints/traits (`T: Ord`), generic type aliases, and generic
function references are not supported. A generic parameter may use only
operations whose types are known for the concrete value. Missing or conflicting
inference produces `E-TYPE-GENERIC-001` or a type mismatch diagnostic.

### Records and classes

`struct` records are values: assignment and parameter passing copy the record
value, equality compares its fields, and `record_with` returns a changed copy.
`class` instances instead have reference identity. `class` is contextual only
before a class declaration; elsewhere it remains an ordinary identifier so
valid language line `0.1` source can keep using `class` as a variable or
function name. Constructors use the same readable field syntax:

~~~zelyra
class Counter {
    value: Int
}

fn increment(counter: Counter) {
    counter.value = counter.value + 1
}

fn main() {
    first = Counter { value: 0 }
    alias = first
    increment(first)
    print(alias.value) # 1
    print(first == alias) # true
}
~~~

Every non-optional field must be initialized. Field assignment is statically
checked for its receiver, field name, and value type and updates the same
instance; binding copies, function returns, and parameter passing preserve its
identity. `==` and `!=` compare class instance identity, even when two
instances have equal field values. Generic classes are supported when their
type arguments can be inferred from the constructor or are explicit.

Classes do not have method or inheritance syntax. Behavior is expressed as an
ordinary function with an explicitly typed receiver parameter, keeping data
flow visible and avoiding implicit `this`. `record_with` cannot update a
class. A field update that would create a reference cycle is rejected with a
controlled runtime diagnostic; field mutation inside a `parallel` block is
also rejected at runtime. This prevents unbounded reference cycles and
nondeterministic concurrent mutation.

Class references cannot be serialized directly through JSON, HTTP APIs, or
SQL because those formats do not preserve runtime object identity. Explicitly
convert the fields needed for exchange into a `struct` DTO. Records and enum
variants containing class references are also rejected at JSON boundaries.
These rules describe the 0.9.9 development branch; inheritance, methods,
closures, and a general object/ORM mapper remain outside this scope.

### Named function values in 0.9.9

The 0.9.9 development branch supports storing, passing, and indirectly calling
references to named, non-generic functions. Prefix `&` marks a reference; it
does not execute the function:

~~~zelyra
fn increment(value: Int) -> Int { return value + 1 }

fn apply(callback: Function<(Int) -> Int>, value: Int) -> Int {
    return callback(value)
}

fn main() {
    transform = &increment
    print(apply(transform, 8))
}
~~~

Function values have explicit parameter and return types. An effectful function
type also names its required capabilities with `uses {Capability}`. The
referenced function's requirements must fit within the declared function type,
and the caller of an indirect call must declare those requirements too. The
checker follows function values through local bindings and assignments; the
runtime repeats the capability check before dispatch. Public functions can be
referenced through an explicit module alias, as in `&math::increment`; private
functions cannot cross module boundaries. The bare identifier `function` is
not a reserved word.

This bounded step does not include closures, capturing local functions, or
generic function references. Function values do not have equality or identity
semantics; they are not JSON or SQL data and are rejected at those boundaries.
Their signatures are available to compiler tools, but not exposed as HTTP
schema values.

## Payload-carrying enums in 0.9.9

An enum variant may declare the types of the values it stores:

~~~zelyra
enum OrderEvent { Created(Int) Paid(Date, Decimal) Rejected(String) }

fn describe(event: OrderEvent) -> String {
    match event {
        OrderEvent::Created(id) => { return "created" }
        OrderEvent::Paid(day, total) => { return "paid" }
        OrderEvent::Rejected(reason) => { return "rejected" }
    }
}
~~~

Constructors and patterns are checked against each variant's declared arity
and payload types. Non-exhaustive matches report `E-TYPE-MATCH-001`; invalid
variant or payload use reports `E-TYPE-ENUM-001`. Public enums, constructors,
and patterns can cross explicit module imports. Payload variants use an
explicit JSON object such as `{"$enum":"OrderEvent","payload":[7],"variant":"Created"}`;
decoding validates each payload against the enum definition. The existing JSON
shape for unit variants is unchanged.

### Explicit scalar text formatting in 0.9.9

`format_text(value)` explicitly converts one scalar value to display text. It
accepts `Int`, `UInt`, `Float`, `Decimal`, `Bool`, `Char`, `String`, `Date`,
`Time`, `Timestamp`, and `Duration`; it does not add JSON quotes and does not
enable implicit conversions in assignments or string concatenation. Arrays,
records, enums, option/result values, unit, and function values have no implicit
display contract and are rejected. Decimal display omits insignificant trailing
zeroes, so fixed-scale monetary output must use `format_scaled_result`.

## Persistent password-reset quotas in 0.9.9

An auth declaration that enables `reset_tokens` also requires
`reset_rate_limits: <table>`. This is an explicit project schema dependency;
the reset runtime creates no tables. The limiter table has a unique, non-null
`bucket_key: String(64)`, required `window_started: Timestamp` and
`request_count: Int`, nullable `blocked_until: Timestamp?`, and an index starting
with `window_started`. Its core columns cannot use `auto`. The complete source
is in [`examples/password_reset.zyl`](../examples/password_reset.zyl).

The default quota remains three requests per 900 seconds, followed by a
900-second block. Valid-CSRF forgot-password requests consume quota before an
account lookup; reset-password submissions with valid CSRF and a syntactically
valid token use the same bucket. A bucket follows the TCP peer IP, regardless
of source port; forwarded IP headers are ignored. Persistent buckets survive
restart and are shared by instances using the same writable MariaDB, quota settings, and
`ZELYRA_RESET_DELIVERY_KEY`. HMAC-SHA256 with a purpose-specific domain derives
the stored key; the database does not receive the peer IP. This is pseudonymous
security state, not an anonymity guarantee.

An advisory lock and transaction serialize quota updates and cap the table at
4,096 buckets. Lock acquisition waits at most one second; database, lock, or
key errors reject the reset attempt and emit sanitized operator diagnostic
`E-AUTH-043` without account, token, IP, or connection details. The generic forgot-password HTTP response
remains `202`; a rejected reset-password submission remains `400`. A block
takes precedence over an expired request window. An unblocked expired window
starts again with the current request counted as its first request. MariaDB's
clock is authoritative. Each request removes at most 100 inactive buckets with
a window older than 48 hours; no scheduled cleanup runs in the absence of
requests. A full table rejects new buckets while retaining existing quotas.

Existing experimental reset projects must add the auth option and table and
review their additive migration before deploying the new compiler. Missing or
incompatible storage reports `E-AUTH-039`/`E-AUTH-040`, a missing cleanup index
reports `E-AUTH-041`, and an option without `reset_tokens` reports `E-AUTH-042`.
These diagnostics retain the auth declaration's source file and span. Ordinary
login declarations are unaffected. `context` adds optional reset-storage fields
within schema version `1`; `impact`, module plans, table rights, and table
renames now include reset, tenant-membership, and MFA storage references.
Previously omitted cross-module auth references may require an explicit
import path and `read_write` grant. See [configuration](env.en.md).

### SQL identifier bounds

Declared table and storage-column identifiers are checked before SQL execution:
MariaDB/MySQL permit at most 64 bytes, PostgreSQL at most 63 bytes for Zelyra's
ASCII identifiers. The storage column includes the generated `_id` suffix of
a relationship. An excessive name reports `E-DB-001` at its declaration.
SQLite does not receive this limit.

Generated index, unique-index, and foreign-key names retain their established
short spelling. Longer names use a bounded prefix plus the first 16 hexadecimal
characters of the full name's SHA-256 digest. Schema creation, migration plans,
and recognition of managed objects use the same deterministic rule. Existing
PostgreSQL objects whose old names were silently truncated by the server may
require reviewed adoption; they are not silently renamed or dropped.

## Exact decimal boundaries in 0.9.9

`amount: Decimal = decimal("119.00")` declares exact fixed point. Forms, HTTP,
and SQL use validated decimal text. MariaDB/MySQL store `Decimal` explicitly
as `DECIMAL(39,9)`, PostgreSQL as `NUMERIC(39,9)`. Legacy or mismatched fixed
scales prevent SQL execution or server startup with source-related diagnostics.
`Money` retains `(19,4)`. Multiplication/division check the final result with a
bounded exact intermediate; remainder at the signed minimum no longer panics.
JSON outputs remain decimal strings. See the [full rules](decimals.en.md) and
[example](../examples/decimal_money.zyl). This does not establish complete
PostgreSQL CRUD/auth parity or production approval.

## Date/time boundaries on the development branch

Calendar-day arithmetic and UTC milliseconds are checked against independent
Python references. Forms/APIs reject invalid calendar/clock values; Timestamp
is bounded to ISO years 0001–9999. API handlers receive real Date/Time/Timestamp/
Duration values. JSON output stays unchanged; generated clients match it.
Native Date/Time/UTC Timestamp parameters and millisecond storage are verified
on MariaDB, MySQL and PostgreSQL. Incompatible schema types/precision fail
with a source position before application use. Explicit
[`sql_decode<T>`](sql-results.en.md) provides typed scalar/record/array results,
cardinality and NULL checks; the legacy raw-row JSON format stays unchanged. See [rules](dates.en.md), [HTTP evidence](release-readiness/0.9.9-calendar-http.en.md)
and [SQL evidence](release-readiness/0.9.9-calendar-sql.en.md).

## NULL-aware optimistic edits in 0.9.9

MariaDB/MySQL generated edit forms sign native NULL separately from text and
check the locked current row before writing. NULL/literal `\N` transitions
return 409 for stale forms. Omitted nullable fields bind SQL NULL; required and
readonly checks remain. [Rules and limits](edit-snapshots.en.md).
