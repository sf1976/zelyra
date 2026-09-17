# Zelyra 0.1 — Phase 10: Typed API declarations and OpenAPI

[Deutsch](phase-10.de.md) · English

Phase 10 starts the API layer with a typed declaration that can be checked
without a running server:

~~~zelyra
type CustomerId = Id

api GET "/customers/{id}" {
    handler get_customer
    input {
        id: CustomerId
    }
    output Customer
    errors {
        403 Forbidden
        404 NotFound
    }
}
~~~

The declaration records the HTTP method, route, input types, response type, and
documented error statuses. HTTP methods are normalized to uppercase and the
initial implementation supports `GET`, `POST`, `PUT`, `PATCH`, and `DELETE`.

API routes can use the same authentication and permission guards as pages and
CRUD resources:

~~~zelyra
api DELETE "/customers/{id}" {
    handler delete_customer
    requires auth
    permits "customers.delete"
    input { id: CustomerId }
    output Unit
}
~~~

`requires auth` requires an authenticated session or a valid configured bearer
token. Each `permits` declaration requires the named permission. A protected
API requires an `auth` definition in the project; otherwise `zelyra check`
rejects the program. API authorization failures are returned as JSON with a
stable `code` and human-readable `message`.

An executable route can name a Zelyra function with `handler`. Its parameters
must have the same names and types as the API input fields, in the same order,
and its return type must match `output`:

~~~zelyra
fn get_customer(id: CustomerId) -> Customer uses Database {
    return sql<Customer> {
        SELECT id, name, email FROM customers WHERE id = :id
    }
}
~~~

`zelyra check` validates API declarations. It rejects duplicate routes,
unknown input or output types, duplicate input names and error statuses,
malformed path parameters, and path parameters that are not declared in the
`input` block. A path parameter such as `{id}` must have a corresponding input
field named `id`.

Generate an OpenAPI 3.0.3 document:

~~~bash
zelyra doc examples/api.zyl --openapi > openapi.json
~~~

The array example can be checked and served without a database:

~~~bash
zelyra check examples/api_arrays.zyl
zelyra serve examples/api_arrays.zyl
~~~

The generated document contains path operations, path parameters, query
parameters for `GET` and `DELETE`, JSON request bodies for other methods, typed
success responses, declared error responses, and basic schemas for declared
types and tables. `zelyra doc file.zyl` is equivalent to the explicit
`--openapi` form.

When a handler is present, `zelyra serve` exposes the route. Path and query
values are converted according to the declared input types; JSON request bodies
are supported for non-GET methods, and handler results are returned as JSON.
Database-backed handlers use the configured `DATABASE_URL`, which is MariaDB
by default in the Zelyra runtime.

Input and runtime failures use the same transport shape, for example:

~~~json
{"error":{"code":"BadRequest","message":"missing API input `id`"}}
~~~

The declared `errors` block documents possible HTTP responses in OpenAPI.
When a handler returns `Err("NotFound")`, and `NotFound` is declared with a
status, the runtime returns that status as a structured JSON error. An
undeclared error is never guessed and becomes a 500 response.

The handler therefore declares a `Result` when it can return one of these
application errors:

~~~zelyra
api GET "/customers/{id}" {
    handler find_customer
    input { id: CustomerId }
    output Result<Customer, String>
    errors { 404 NotFound }
}
~~~

Typed arrays are supported for API input and output:

~~~zelyra
api POST "/customer-ids" {
    handler echo_ids
    input { ids: CustomerId[] }
    output CustomerId[]
}
~~~

Array literals and the first general array operations are available in the
language core:

~~~zelyra
numbers = [1, 2, 3]
first = numbers[0]
count = len(numbers)
extended = append(numbers, 4)
combined = numbers + [5, 6]
~~~

Records provide the declared model for nested JSON objects. Missing optional
fields become `None`; unknown fields and missing required fields are rejected
at the API boundary:

~~~zelyra
struct Address {
    city: String
}

struct CustomerInput {
    name: String
    address: Address
}

api POST "/customers" {
    handler create_customer
    input { customer: CustomerInput }
    output CustomerInput
}
~~~

The current handler bridge intentionally keeps the first record release small:
record values are fully supported at the JSON API boundary, while source-level
record literals, field access, generated client bindings, and richer
domain-error values remain later Web/API work.
