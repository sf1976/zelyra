# Scoped native decision transactions

Snapshot: unreleased 0.9.9 development branch. [Deutsch](decision-transactions.de.md)

The bootstrap database layer exposes `with_decision_transaction(url, closure)`
and a borrowed `DecisionTransaction::execute(Query)`. All conditional reads and
writes inside the closure use one connection. `Ok` commits; an error, expired
request deadline or unwind rolls back. The handle cannot escape the closure.
This is an internal native API, not new Zelyra syntax or full backend parity.

An execution error poisons the transaction even if a callback ignores it;
subsequent statements and commit are refused. SELECT/INSERT/UPDATE/DELETE are
accepted. Transaction-control and implicit-commit DDL are rejected. Existing
MariaDB/MySQL generated audit session initialization is explicitly retained.
SQL binding and exact result/NULL codecs are shared with normal native queries.
Application SQL still needs the normal compiler/schema/capability checks.

PostgreSQL uses the driver's scoped transaction, `SET LOCAL statement_timeout`
from the configured timeout and remaining request budget before each operation,
and a fresh deadline check before commit. Local timeout settings do not escape
the transaction. PostgreSQL and MariaDB/MySQL pool acquisition retain existing
TLS/settings/UTC policy. Ordinary MariaDB closure transactions now roll back and
discard their connection on unwind/failed commit as well as on returned errors.
No statement is retried automatically; connection/commit failures are errors,
not proof that an externally observed commit outcome is always knowable.

Deadline checks before statements and commit do not establish hard interruption
bounds for every MySQL write. Existing SELECT timer/driver limits remain; the
separate MySQL write-bound gate stays open. PostgreSQL guarded forms (tenant,
recipient, edit snapshot, audit decisions) still require their adapter/guard
integration. Auth/MFA/notification parity and chained PostgreSQL audits are not
established by this native foundation. No existing guard is removed.

No dependency, environment switch, migration, telemetry, provider or website
change. `DATABASE_URL`, timeout/TLS settings and pool controls remain the existing
ones. [Native evidence](release-readiness/0.9.9-decision-transactions.en.md) records
actual three-backend commit/rollback/deadline probes with a one-connection pool.
