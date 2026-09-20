# Zelyra Phase 3: Database Core

[Deutsch](phase-3.de.md) · English

Phase 3 makes the database schema part of the Zelyra program and generates DDL
for PostgreSQL, MariaDB, and SQLite.

## Schema definition

```zelyra
database main {
    engine: mariadb
    database: "machine_management"
}

table departments {
    id: Id primary auto
    name: String(100) required unique
}

table machines {
    id: Id primary auto
    number: String(30) required unique
    department: Department required
    active: Bool default true

    index {
        department
    }
}
```

`Department` is resolved to the `departments` table. The generated storage
column is `department_id` and receives a foreign key to `departments.id`.

## Database backends

MariaDB is now the default backend for new Zelyra projects. The backend can be
selected explicitly with `engine`:

```zelyra
database main { engine: postgres database: "machine_management" }
database main { engine: mariadb database: "machine_management" }
database main { engine: sqlite database: "machine_management.sqlite3" }
```

`mysql` is accepted as a compatible name for the MariaDB backend. Connection
strings are passed through `DATABASE_URL` only and do not belong in source or
version control.

## Generate DDL

Generate the initial DDL without connecting to a database:

```bash
zelyra db create examples/machine_management.zyl
```

The output includes `CREATE TABLE`, backend-specific types, `NOT NULL`,
defaults, foreign keys, unique constraints, and indexes.

For a simple local setup, MariaDB and SQLite can be initialized directly:

```bash
export DATABASE_URL='mariadb://user:password@127.0.0.1:3306/machine_management'
zelyra db bootstrap examples/machine_management_mariadb.zyl

export DATABASE_URL='sqlite:///tmp/machine_management.sqlite3'
zelyra db bootstrap examples/machine_management_sqlite.zyl
```

`db bootstrap` creates the named MariaDB database and applies the schema. For
SQLite it creates the database file. Set credentials interactively or through
a secret manager.

## Inspect, plan, and apply

Set a matching connection string for commands that access a live database:

```bash
export DATABASE_URL='postgres://user:password@localhost/machine_management'
zelyra db inspect examples/machine_management.zyl
zelyra db plan examples/machine_management.zyl
zelyra db apply examples/machine_management.zyl
```

MariaDB uses `mariadb://` or `mysql://`; SQLite uses `sqlite://` followed by a
file path. `db inspect` reads tables, columns, foreign keys, and indexes from
all three backends.

`db plan` can also run without `DATABASE_URL`; it then plans against an empty
database and is useful for reviewing initial DDL. `db apply` refuses to run
without a live connection.

Changes marked `REVIEW` or `DESTRUCTIVE` are visible in the plan and refused
by default. Approve only after review:

```bash
zelyra db apply examples/machine_management.zyl --allow-risky
```

`--allow-destructive` remains for destructive-only plans and does not approve
`REVIEW` changes. `UNSUPPORTED` changes are never applied. Connection
credentials are read from the environment and are never stored in the schema
source.

## Current limitations

MariaDB is the primary runtime reference backend; PostgreSQL schema inspection
and planning are available, but runtime parity is not. SQL Server is not
integrated yet. Nullability changes and SQLite type, foreign-key, and
unique-constraint alterations are marked unsupported and require a future
specialized migration implementation. Adding or removing MariaDB foreign keys
requires `REVIEW`; SQLite foreign-key alterations are blocked until table
rebuilds are supported. The planner preserves unrecognized external indexes
and refuses untracked foreign-key removals rather than guessing ownership.
MariaDB and PostgreSQL default additions, changes, and removals produce
`REVIEW` plans and require `--allow-risky`; integration tests verify that
existing values survive and that replanning is idempotent. SQLite default
changes and primary-key/auto-increment metadata changes, as well as
primary-key/auto-increment changes on MariaDB and PostgreSQL, remain
`UNSUPPORTED`. PostgreSQL schema-safety integration runs against PostgreSQL 16
in CI; this does not establish PostgreSQL runtime parity. Type changes and
nullability changes are classified independently, preventing a safe type
widening from masking unsupported nullability drift.
