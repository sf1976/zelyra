# Zelyra 0.1 — Phase 10: Typed API declarations and OpenAPI

[Deutsch](phase-10.de.md) · English

Phase 10 starts the API layer with a typed declaration that can be checked
without a running server:

~~~zelyra
type CustomerId = Id

api GET "/customers/{id}" {
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

This phase is deliberately a declaration and documentation boundary. It does
not yet attach executable handlers, perform request decoding at runtime, or
serialize database rows. Those pieces will use the same API model in a later
Web/API phase.
