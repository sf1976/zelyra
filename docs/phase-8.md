# Zelyra 0.1 — Phase 8: Authentication and authorization

[Deutsch](phase-8.de.md) · English

Phase 8 now has its first security boundary. Authentication can be declared
against a user table:

~~~zelyra
auth users {
    table: users
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
users.

Successful login creates an HttpOnly, SameSite session cookie. Logout is
available as POST /logout. Sessions are kept in process memory for this
initial implementation and are invalidated when the server stops.

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

This is the first working authentication slice. Database-backed roles and
permission lookup, session persistence, login throttling, and a dedicated
password-management command remain the next authentication steps.
