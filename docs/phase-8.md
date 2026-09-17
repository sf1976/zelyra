# Zelyra 0.1 — Phase 8: Authentication and authorization

[Deutsch](phase-8.de.md) · English

Phase 8 now has its first security boundary. Authentication can be declared
against a user table:

~~~zelyra
auth users {
    table: users
    sessions: auth_sessions
    permissions: user_permissions
}
~~~

Pages and CRUD resources can require authentication and permissions:

~~~zelyra
page "/admin" {
    requires auth
    permits "admin.view"

    html {
        <h1>Admin</h1>
    }
}
~~~

The compiler validates that the configured user table exists and that
protected routes have an auth definition. The web runtime denies protected
routes by default. It now provides a database-backed login at /login. The
configured user table must contain id, email, and password_hash columns;
password_hash values use Argon2. An optional active column disables inactive
users. Failed login attempts are tracked per normalized e-mail address. After
five failures within 15 minutes, further attempts are rejected with HTTP 429
for 60 seconds and include a `Retry-After: 60` header.

Successful login creates an HttpOnly, SameSite session cookie and rotates any
previous session token from that browser. When the
optional sessions table is configured, only a Blake2s-256 hash of the session
token is stored in MariaDB; the cookie itself is never stored in the database.
Sessions expire after 24 hours and logout removes the database record. Without
the sessions option, the explicit development fallback keeps sessions in
process memory.

When the optional permissions table is configured, it must contain user_id and
permission columns. Permissions are loaded from that table for every
protected request. An authenticated user with no matching permission receives
HTTP 403. Without the permissions option, the explicit
ZELYRA_AUTH_PERMISSIONS allowlist remains available for local development and
reverse-proxy deployments.

For deployments that authenticate users in a reverse proxy, the server also
accepts a Bearer token when ZELYRA_AUTH_TOKEN is explicitly configured. The
server-side permission allowlist is configured with
ZELYRA_AUTH_PERMISSIONS, for example:

~~~bash
export ZELYRA_AUTH_TOKEN='local-development-secret'
export ZELYRA_AUTH_PERMISSIONS='admin.view,customers.view'
zelyra serve examples/auth.zyl
~~~

Send the token with a request:

~~~bash
curl -H 'Authorization: Bearer local-development-secret' \
  http://127.0.0.1:3000/admin
~~~

Without the token the response is HTTP 401. With a valid token but without a
declared permission the response is HTTP 403. Tokens and permissions are never
stored in source code by the compiler.

To create the Argon2 value required by the `password_hash` column, use:

~~~bash
zelyra auth hash-password
~~~

The interactive command does not echo the password and asks for confirmation.
For deliberate automation, `zelyra auth hash-password --stdin` reads one
password line from standard input. Do not place real passwords in command-line
arguments or source control.

This is the first working authentication slice with persistent sessions and
database-backed permission lookup. Login throttling and session rotation are
implemented; database roles remain a future authentication step.

The repository test `tests/mariadb-auth-e2e.sh` exercises this flow against
MariaDB with two temporary users: anonymous access is rejected, invalid
credentials fail, the permitted user receives a persistent session and can
access `/admin`, a logged-in user without `admin.view` receives HTTP 403, and
logout removes the session from MariaDB.

The combined example `examples/auth_crud_api.zyl` applies the same boundary to
both a CRUD resource and a typed API:

~~~zelyra
crud Customer -> customers {
    requires auth
    permits "customers.view"
}

api GET "/api/customers/{id}" {
    handler get_customer
    requires auth
    permits "customers.view"
    input { id: CustomerId }
    output Customer
}
~~~

The integration test `tests/mariadb-protected-e2e.sh` verifies that anonymous
requests receive HTTP 401, the permitted session receives HTTP 200 for both
endpoints, and an authenticated user without `customers.view` receives HTTP
403 from both the HTML CRUD route and the JSON API. It also verifies separate
`customers.create`, `customers.edit`, and `customers.delete`
permissions on the generated CRUD actions.
The same test also covers an action-level permission on a custom form action.
