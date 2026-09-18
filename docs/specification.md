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
for search, filtering, sorting, and pagination on arbitrary pages are not yet
part of this feature.

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
