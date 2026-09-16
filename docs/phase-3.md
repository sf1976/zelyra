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

Destructive changes are visible in the plan and are refused by default:

```bash
zelyra db apply examples/machine_management.zyl --allow-destructive
```

The flag is required only after reviewing the generated plan. Connection
credentials are read from the environment and are never stored in the schema
source.

## Current limitations

PostgreSQL remains the primary reference backend. SQL Server is not integrated
yet. SQLite requires a table rebuild for some destructive changes; those
changes are marked destructive in the plan and need a future specialized
migration implementation.
