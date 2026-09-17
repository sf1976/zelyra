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

The current handler bridge is intentionally small: error declarations are
documented in OpenAPI, but application-specific error mapping, authentication
guards, and richer JSON decoding remain later Web/API work.
