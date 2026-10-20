# Database support matrix

This matrix names the concrete servers and workflows that Zelyra CI runs
against them. An entry does not imply complete SQL or runtime parity. The CI
matrix is in [`ci.yml`](../.github/workflows/ci.yml). This table describes the
currently checked scope and its limits for each backend.

| Server | Verified workflows | Limits of the evidence |
| --- | --- | --- |
| MariaDB 10.11.19, 11.4.13, 11.8.9, 12.3.3 | Studio database map and CRUD builder; local TOTP MFA; parameters, pool limits, and statement timeouts; TLS; database/CRUD HTTP E2E; relational tenant CRUD with real foreign keys and orphan rejection (E-DB-005); schema safety; backup and restore; handbook examples | MFA remains an explicitly configured preview. CI is neither an independent security review nor product acceptance. Not every optional database driver path is run on every version. |
| MySQL Server 8.4.11 | Parameterized Zelyra SQL queries; schema setup, inspection, read-only project map, and a reviewed safe column migration with history; relational tenant CRUD with real foreign keys, orphan rejection (E-DB-005), and hardened write lock wait bounds (`innodb_lock_wait_timeout`) | Limited compatibility evidence. No proof for authentication outside relational CRUD sessions, MFA, TLS, or backup/restore. The schema E2E covers additive safe column changes; do not infer full MariaDB parity. |
| PostgreSQL 16.15 | Studio introspection, parameterized runtime queries, schema safety, native form decisions with unchained audit, and relational tenant CRUD with real foreign keys and orphan rejection (E-DB-005) | No proof for full authentication backend, MFA, TLS, backup/restore, or general SQL portability. Chained audit remains locked. CI pins the minor server version and checks it before the integration steps. |
| SQLite (CI image system version) | Schema integration, parameterization, and basic CRUD workflows | Local development option; not a substitute for the tested server databases and not a 1.0 production promise. |

## Consequence for Zelyra 1.0.0

MariaDB, MySQL, and PostgreSQL remain targets for the first productive release.
The verified scope differs by backend. 0.9.9 can freeze the 1.0.0 support
scope only after each promised workflow has evidence. Accepting a URL such as
`mysql://` is not evidence for MySQL Server support.
For post-1.0 development, a modular database adapter architecture
(`DatabaseAdapter` SPI) and multi-database support (binding multiple
independent data sources per application) are formally anchored in the roadmap.

## Additional local evidence: exact decimals (0.9.9 development)

MariaDB 11.4.13, MySQL 8.4.11, and PostgreSQL 16.15 pass isolated round trips
for Decimal/Money forms, SQL storage, exact CRUD filters, HTTP/generic JSON
data, optional NULLs, and legacy-scale rejection. New CI steps execute the
same probe. See the [package evidence](release-readiness/0.9.9-exact-decimals.en.md).
Plain PostgreSQL form actions are transactional; tenant/recipient, edit-snapshot,
and audit decisions remain separate technical gates. MySQL's server timer
applies to SELECT; this does not prove hard interruption bounds for writes.
It is not general backend approval.

## Native decision foundation (0.9.9 development)

A scoped callback API now passes commit, conditional read/write visibility,
rollback on returned/SQL/ignored errors and unwind, deadline checks before
statements/commit and connection reuse on MariaDB 11.4.13, MySQL 8.4.11 and
PostgreSQL 16.15 with a one-connection pool. PostgreSQL statement cancellation
is tested. This does not close general auth adapter parity or MySQL hard
write bounds. [Rules](decision-transactions.en.md),
[evidence](release-readiness/0.9.9-decision-transactions.en.md).

## PostgreSQL form decisions (0.9.9 development)

Form decisions on PostgreSQL combine tenant membership, recipient authorization,
row locking, optimistic snapshot comparisons (NULL vs. `\N`), business mutation,
and atomic unchained audit logging within a single transaction and connection.
Tampered tenant/foreign keys, membership revocation during lock wait (two-connection
race), and audit errors roll back the transaction atomically. Chained PostgreSQL
audit remains locked and diagnostically rejected.
See [evidence](release-readiness/0.9.9-postgres-form-decisions.en.md).

## Database matrix, transactions, and relational CRUD (0.9.9 development)

On MariaDB 11.4.13, MySQL 8.4.11, and PostgreSQL 16.15, the relational CRUD cycle
with real foreign key constraints is proven:
- Migration preflight with orphan checks (`error[E-DB-005]`) preserves data and
  prevents invalid constraint creation.
- Dynamic foreign-key options (`load_relation_options`) filter strictly by active
  tenant membership.
- Tampered foreign keys belonging to other tenants return HTTP 422 Unprocessable Entity.
- Relational join display renders parent display values instead of raw IDs.
- Restrictive deletion safely halts deletion of referenced records with HTTP 500
  without internal leakage; ordered deletion (child before parent) succeeds.
- On MySQL and MariaDB, `innodb_lock_wait_timeout` enforces hard, bounded abort
  thresholds for row locks (~1s instead of 50s).
See [evidence](release-readiness/0.9.9-relational-crud-matrix.en.md).

## Typed scalar references and tenant keys (1.2.30 development)

A scalar nominal ID can retain its Zelyra type while declaring an explicit
database reference, for example `customer_group_id: CustomerGroupId?
references customer_groups`. The target table must declare an `id` primary
key, and both source and target IDs must resolve to `Id`. The reference is
checked by the compiler and emitted as a database foreign key; an optional
source ID becomes a nullable SQL column. If both tables are tenant-scoped
with the same nominal tenant type, the FK is composite over
`(tenant_id, relation_id) -> (tenant_id, id)`, and the target receives the
required unique `(tenant_id, id)` key. This prevents cross-tenant references
at the database boundary. Different tenant identity aliases are rejected.

MariaDB, PostgreSQL, and SQLite DDL-generation tests cover this shape. A live
MariaDB 11.8 check accepted an in-tenant reference and rejected a cross-tenant
one. This is bounded evidence for the explicit scalar-reference form, not a
claim of general database-backend parity. The Laravel importer emits this
syntax only when a static migration proves the FK. A composite tenant FK
cannot preserve `ON DELETE SET NULL`, because that would also clear the
required tenant key; the importer marks the generated `RESTRICT` behavior as a
reviewed difference rather than claiming equivalence. See the
[1.2.30 work plan](release-plans/1.2.30-typed-schema-references.en.md).

## Number series in generated CRUD (1.2.31 development)

The unreleased development branch lets a string column declare a number series,
for example `customer_number: String(32) required
auto_sequence("KD-{0001}")` or `quote_number: String(40) required
auto_sequence("AG-{year}-{0001}")`. `{0001}` sets the zero-padded counter
width; `{year}` uses the UTC calendar year and starts a separate counter each
year. Schema planning includes the internally managed
`_zelyra_number_sequences` table. Projects must review and deliberately apply
that schema change before create requests use these fields. Generated create
forms ignore forged client values; edit forms omit the number. MariaDB and
PostgreSQL 16 have been tested for transactions, rollback, tenant separation,
existing-value collisions, and concurrent allocation. SQLite is rejected for
this runtime feature. Numbering is not guaranteed gapless: voided records must
not silently reuse numbers. This syntax is not available in Zelyra 1.2.30.
