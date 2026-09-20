# Zelyra 0.1 specification index

The repository's implementation specification is maintained across the phase
documents, the bilingual handbook, and the roadmap. This file defines the
cross-cutting AI-native contract that applies to all of them.

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

When a page collection declares search, filters, sorting, or pagination,
Zelyra automatically renders a semantic query-control form before the page
content and preserves the current URL state. Pagination also performs a safe
count query and exposes `total` and `pages` as `UInt` page bindings. Pages that
use only explicit `input` declarations remain manual by design.
