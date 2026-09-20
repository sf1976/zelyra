# MariaDB compatibility matrix

**Scope:** Zelyra 0.2.0 database and CRUD runtime paths | **Matrix reviewed:** 2026-09-20 | **Evidence:** exact official MariaDB Docker image tags below; Linux x86_64

This matrix records tested Zelyra behavior. It is not a MariaDB certification,
a promise that every SQL feature works on every server, or a MySQL compatibility
claim. MariaDB is Zelyra's primary runtime reference. PostgreSQL runtime parity
and MySQL Server are outside this matrix.

## Tested versions

| MariaDB Community LTS line | Exact image tag used for tests | Zelyra test status | Community maintenance through* |
|---|---|---|---|
| 10.11 | `mariadb:10.11.19` | Locally verified; exact tag is in CI matrix | 2028-02-16 |
| 11.4 | `mariadb:11.4.13` | Locally verified; exact tag is in CI matrix | 2029-05-29 |
| 11.8 | `mariadb:11.8.9` | Locally verified; exact tag is in CI matrix | 2028-06-04 |
| 12.3 | `mariadb:12.3.3` | Locally verified; exact tag is in CI matrix | 2029-06-12 |

The GitHub Actions job `mariadb-compatibility` runs the same core integrations
on each exact image tag. It checks the server-reported version and logs the
resolved image digests. A green run of that job is required before treating a
change to the database runtime as verified across the matrix. Refresh the
patch tags when the MariaDB Foundation publishes newer maintenance releases,
then rerun the matrix before release.

The current lines and maintenance dates above are a dated snapshot of the
[MariaDB Foundation maintenance policy](https://mariadb.org/about/#maintenance-policy).
The Foundation's [Q3 2026 maintenance announcement](https://mariadb.org/mariadb-server-12-3-11-8-11-4-and-10-11-q3-2026-maintenance-releases-and-goodbye-10-6/)
lists the patch releases used here. The exact tags are published in the
[official MariaDB Docker image](https://hub.docker.com/_/mariadb).

## What the matrix tests

The CI matrix runs these integrations for every listed image. Local checks
can run the same scripts against a disposable MariaDB service:

- `tests/mariadb-e2e.sh`: schema setup, inspection, idempotent planning,
  relationship-backed CRUD over HTTP, and search/filter/sort/pagination.
- `tests/schema-safety-e2e.sh`: destructive column drops require approval;
  required columns without defaults, new unique constraints, MariaDB foreign-
  key additions/removals, and MariaDB default changes are marked `REVIEW`.
  Tests verify that setting/changing/removing defaults requires approval,
  retains existing values, and produces an idempotent next plan. Duplicate
  rows survive a rejected unique-index creation and orphan rows survive a
  rejected foreign-key addition. MariaDB and PostgreSQL nullability changes
  require `REVIEW`; tightening runs a NULL-row preflight before any plan SQL,
  refuses the whole plan when existing NULL rows need repair, and tests verify
  loosening/tightening after approval. MariaDB's strict DDL guard is also
  tested against silent NULL coercion. SQLite nullability and type,
  foreign-key, unique-constraint, and default alterations are blocked by
  `E-DB-006` even with approval. MariaDB primary-key/auto-increment and SQLite
  key/explicit `AUTOINCREMENT` drift remain blocked.
- `tests/postgres-schema-safety-e2e.sh` checks PostgreSQL default changes under
  `REVIEW` approval and primary-key, serial, and identity metadata drift against
  PostgreSQL 16. It is a schema safety check, not a PostgreSQL runtime
  compatibility claim.

Each CI matrix job has its own ephemeral MariaDB service and database. The
schema-safety test creates and drops only a uniquely named test database. It
also checks that MariaDB's behavior when adding a required column without a
default is never invoked without explicit approval; after approval, existing
values must be reviewed before application code relies on them. Zelyra-created
indexes are identified by generated names for managed removal; unrecognized
indexes are preserved. The legacy `--allow-destructive` option does not
approve `REVIEW` changes; use `--allow-risky` after reviewing the plan.

## What this does not establish

- MySQL Server compatibility. Accepting a `mysql://` connection URL does not
  mean MySQL is part of this tested matrix.
- Support for MariaDB versions or patch releases not listed above.
- Runtime parity for PostgreSQL or SQL Server.
- Compatibility of every MariaDB-specific SQL feature, plugins, Galera,
  replication, failover, production backup/restore, or upgrades of existing
  production data.
- Performance, capacity, or production-hardening guarantees.

Use the database version deployed in production for acceptance tests as well.
Before upgrading MariaDB or changing Zelyra's schema/runtime behavior, take a
verified backup and test the resulting plan against a disposable copy of the
real schema and representative data.
