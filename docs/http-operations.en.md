# HTTP health checks, deadlines, and retries

This guide defines how to monitor Zelyra's built-in HTTP server and how
clients should handle transient failures. It describes current behavior; it
does not promise that a request handler can be cancelled.

## Health-check categories

- **Listener reachability:** check whether the configured TCP endpoint can be
  reached. Zelyra does not expose a built-in liveness URL. A successful TCP
  handshake does not prove that Zelyra processed the connection, and says
  nothing about database availability or application routes.
- **Application readiness:** provide an application-owned, read-only `GET`
  API route for the dependencies required to serve normal traffic. Zelyra
  does not reserve or generate a readiness route. Keep its response generic
  and do not return database diagnostics, credentials, or internal hostnames.
- **Dependency readiness:** if an application depends on MariaDB, its own
  readiness handler may perform one bounded read-only query. Do not run schema
  inspection, migrations, or writes from a probe. `zelyra doctor --json` is a
  point-in-time CLI diagnostic, not an HTTP probe or a liveness signal.

Use separate liveness and readiness decisions. A database outage should make
an application that requires that database unready; it should not cause an
orchestrator to repeatedly restart a healthy Zelyra process. Projects without
a database can define readiness from their own required dependencies.

## Connection and request deadlines

The listener serves at most 64 concurrent connections and drops excess
connections. Each connection exchange has one absolute 30-second deadline,
including request reading and response writing. A timeout while reading can
close the connection without an HTTP response; a response that finishes after
the deadline is dropped.

Request handlers are synchronous and cannot currently be interrupted. A
handler that exceeds the socket deadline can continue occupying one of the 64
worker slots until its operation returns. Database connect, pool-wait, and
statement timeouts provide additional bounds for configured MariaDB
operations, but they do not cancel arbitrary application code. A client
timeout therefore does not prove that a write was rolled back.

## Retry rules

Zelyra does not automatically retry HTTP requests, SQL statements, or
transactions. It does not reuse a connection after an uncertain transaction
failure. A project or client must make any retry policy explicit.

- `GET` and `HEAD` requests are normally safe to retry after a connection
  failure or a transient `502`/`503`/`504`. Use a small retry limit and
  exponential backoff with jitter; honor `Retry-After` when present.
- Retry a write only when the operation is idempotent or protected by an
  application-level idempotency key or conditional update. A timed-out write
  can have committed even when the client did not receive its response.
- Do not blindly replay a transaction after a timeout or connection loss.
  First determine whether its effects committed; if that cannot be established,
  reconcile application state before submitting another write.
- `429` responses from configured API quotas include `Retry-After`. Respect
  that delay and retain a bounded retry count.

API and login quotas are process-local. Separate instances do not share quota
state, and a restart clears it. These limits are not a distributed abuse
control.

## Current boundary

The documented probe categories and retry rules do not add built-in HTTP
health routes, handler cancellation, or distributed rate limits. Readiness
routes remain application-owned and need acceptance tests in the generated
business application.
