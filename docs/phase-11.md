# Zelyra 0.1 — Phase 11: Browser API integration and CORS

[Deutsch](phase-11.de.md) · English

Zelyra APIs can be called directly from browser applications. Cross-origin
access is disabled by default. A project must explicitly list exact origins in
`zelyra.toml`:

~~~toml
[web]
allowed_origins = ["http://localhost:5173", "https://app.example"]
allow_credentials = false
~~~

Origins must use `http://` or `https://` and may not contain a path, query,
fragment, wildcard, or trailing slash. Wildcard origins are deliberately not
supported. This keeps the common development setup simple while preventing a
configuration from silently opening the API to every website.

For an allowed origin, API responses receive `Access-Control-Allow-Origin` and
`Vary: Origin`. When credentials are explicitly enabled, responses also carry
`Access-Control-Allow-Credentials: true`. This is required for browser session
cookies; clients must still opt into credentials themselves.

Browser preflight requests are handled automatically:

~~~http
OPTIONS /customers HTTP/1.1
Origin: http://localhost:5173
Access-Control-Request-Method: POST
Access-Control-Request-Headers: content-type
~~~

The server returns `204 No Content` with the allowed methods, requested
headers, and a bounded `Access-Control-Max-Age`. A disallowed origin or method
receives a structured JSON error. Method mismatches on API routes include an
`Allow` header.

Every HTTP response also contains exactly one `X-Request-ID` header. An
incoming `X-Request-ID` is reused only when it contains at most 128 ASCII
letters, digits, dots, underscores, or hyphens; otherwise the server generates
a new ID. API handlers receive the same sanitized value as
`request.headers["x-request-id"]`. This lets clients correlate failures with
operator logs or diagnostics without reflecting arbitrary header values. The
ID is not authentication and does not claim distributed request tracing.

API declarations can also set a version, a deprecation marker, and a bounded
request quota:

~~~zelyra
api GET "/api/v1/customers" {
    version "v1"
    deprecated
    rate_limit 100 per 60
    output Customer[]
}
~~~

The version and deprecation marker appear in the
`X-Zelyra-API-Version` and `X-Zelyra-API-Deprecated` response headers and in
OpenAPI metadata. The quota applies per API route and client IP within the
server process; exceeding it returns HTTP 429 with `Retry-After`. The window
must be 1 to 86400 seconds and the limit 1 to 1,000,000 requests. The server
does not trust `X-Forwarded-For`. Behind a reverse proxy, clients therefore
share the proxy IP's quota; persistent and distributed limiters are not
included. The table is bounded to 4,096 client/route pairs; while it is full,
new pairs receive HTTP 429 until a window expires.

No CORS headers are added to pages, forms, or CRUD responses by this phase.
The policy is only applied to declared API routes, and adding an origin does
not bypass authentication or permission checks.

The generated TypeScript client remains dependency-free and can use the same
origin policy. Configure its `fetch` call with `credentials: "include"` only
when the project explicitly enables credentialed CORS.
