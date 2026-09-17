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

No CORS headers are added to pages, forms, or CRUD responses by this phase.
The policy is only applied to declared API routes, and adding an origin does
not bypass authentication or permission checks.

The generated TypeScript client remains dependency-free and can use the same
origin policy. Configure its `fetch` call with `credentials: "include"` only
when the project explicitly enables credentialed CORS.
