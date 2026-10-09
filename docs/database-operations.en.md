# MariaDB backup and recovery

This guide describes an operator-managed backup and restore workflow.
`zelyra db plan` and `zelyra db apply` do **not** create backups, and an
executed DDL change cannot generally be rolled back. A migration plan is not a
replacement for an independent, verified database backup.

## Before a schema change

1. Verify the target server, database name, and SQL plan. Do not print
   `DATABASE_URL` in diagnostics or screenshots.
2. Use a dedicated backup account with the MariaDB privileges required for
   that backup. Do not use the Zelyra application account or `root` when a
   restricted account is sufficient.
3. Store the backup encrypted and separately from the database server; define
   retention and access controls.
4. Restore it first to an isolated, disposable MariaDB instance and check the
   schema and representative data.
5. Test the Zelyra plan against that disposable copy before approving it for
   the intended target.

## Create a backup

This example assumes MariaDB at `127.0.0.1:3307`, database
`adressverwaltung`, and a provisioned `zelyra_backup` account. For Docker
projects, `3307` is the host port only if Compose publishes it that way;
otherwise use the configured host port. The password is prompted
interactively and is not part of the command.

```bash
set -eu
umask 077
mkdir -p backups
backup_file="backups/adressverwaltung-$(date -u +%Y%m%dT%H%M%SZ)-$$.sql"
temporary_file="${backup_file}.partial"
trap 'rm -f -- "$temporary_file"' EXIT

mariadb-dump \
    --host=127.0.0.1 \
    --port=3307 \
    --user=zelyra_backup \
    --password \
    --single-transaction \
    --skip-lock-tables \
    adressverwaltung > "$temporary_file"

chmod 600 "$temporary_file"
mv -- "$temporary_file" "$backup_file"
sha256sum "$backup_file"
```

The dump is published under its final name only after it succeeds. An
interrupted dump therefore does not leave a file that looks like a complete
backup.

`--single-transaction` provides a snapshot for transactional InnoDB tables
when no schema changes occur during the dump. It is not a general consistency
guarantee for non-transactional tables, external files, or other databases.
Stored routines and events are included only when their options and required
privileges are deliberately added.

## Restore to a disposable database

Prefer an isolated MariaDB server for this test, not production. Check the
host and port before each step. A database administrator creates the empty
test target; use a separate restore account scoped to that disposable database
to load and inspect the dump:

```bash
mariadb \
    --host=127.0.0.1 \
    --port=3307 \
    --user=zelyra_restore_admin \
    --password \
    --database=mysql \
    --execute='CREATE DATABASE zelyra_restore_check CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci'
```

> **Caution:** The next command changes `zelyra_restore_check`. Run it only
> after confirming that the host, port, and database identify the disposable
> restore target.

```bash
mariadb \
    --host=127.0.0.1 \
    --port=3307 \
    --user=zelyra_restore \
    --password \
    --database=zelyra_restore_check < "$backup_file"

mariadb \
    --host=127.0.0.1 \
    --port=3307 \
    --user=zelyra_restore \
    --password \
    --database=zelyra_restore_check \
    --execute='SHOW TABLES'
```

Also check expected tables, keys, row counts, and representative business
records. A successful client exit alone does not prove the application is
fully restored. Remove the test database only after checks finish and after
verifying the target again.

## Approval, interruption, and limits

- `zelyra db plan <file.zyl> --format=json` emits a versioned
  `zelyra.schema-plan/v1` document with SHA-256 fingerprints of the observed
  and desired schemas, a stable plan ID, drift, SQL steps, preflights, and
  approval requirements. It does not expose the database URL.
- MariaDB `db apply` stores each plan and per-step progress in the reserved
  `_zelyra_schema_history` table. `zelyra db history <file.zyl>` shows applied,
  failed, active, and interrupted runs; JSON is available with
  `--format=json`. The internal table is omitted from normal schema inspection.
  SQLite and PostgreSQL do not yet have a persistent migration history.
- The JSON plan does not generate safe inverse SQL. `rollback.generated`
  remains `false`; safe recovery still requires a verified operator-managed
  backup.
- `zelyra db plan` previews the detected schema difference; it does not back
  up data or reserve the database state.
- Before applying any plan SQL, `db apply` checks existing rows for NULLs
  before tightening nullability, checks table emptiness before adding a
  required column without a default, checks duplicate non-NULL value groups
  before adding a unique index, and checks existing foreign-key values for
  missing referenced rows. A failed or unavailable check blocks the whole
  plan. These read-only checks do not lock out concurrent writers; repair data
  and rerun the plan, and coordinate writes during a migration.
- `zelyra db apply` executes supported SQL steps. PostgreSQL plans run in a
  single transaction; SQLite plans use `BEGIN IMMEDIATE`, stop at the first SQL
  error, and roll back when a step fails. Integration tests verify that a
  failed step removes earlier DDL and that a corrected plan can be applied.
  MariaDB DDL may implicitly commit transactions, so a later error can leave
  earlier steps applied.
- MariaDB migrations take a database-scoped advisory lock and checkpoint each
  DDL step. After a process interruption, `db history` reports the last
  checkpoint. Inspect the live schema with `db inspect`, create and review a
  fresh plan, then apply that plan. Do not assume reapplying an old plan safely
  repairs a partial state. A crash during a DDL statement can still leave that
  statement's result ambiguous; verify the schema and restore from backup when
  needed.
- `--allow-risky` and `--allow-destructive` are not backup or rollback
  options. Use them only after reviewing the plan and verifying an independent
  backup.
- Zelyra provides no automatic production backup, restore, or rollback
  guarantees. Retention, encryption, recovery time, and regular restore
  rehearsals remain the operator's responsibility.

See also [database compatibility](database-compatibility.en.md) and the
[0.5.0 plan for safe database operations](release-plans/0.5.0.en.md).
