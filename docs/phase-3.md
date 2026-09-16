# Zelyra Phase 3: Database Core

[Deutsch](phase-3.de.md) · English

Phase 3 makes the database schema part of the Zelyra program and generates
PostgreSQL DDL from it.

## Schema definition

```zelyra
database main {
    engine: postgres
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

## PostgreSQL DDL

Generate the initial DDL without connecting to a database:

```bash
zelyra db create examples/machine_management.zyl
```

The output includes `CREATE TABLE`, PostgreSQL types, `NOT NULL`, defaults,
foreign keys, unique constraints, and indexes.

## Inspect, plan, and apply

Set an explicit PostgreSQL connection string for commands that access a live
database:

```bash
export DATABASE_URL='postgres://user:password@localhost/machine_management'
zelyra db inspect examples/machine_management.zyl
zelyra db plan examples/machine_management.zyl
zelyra db apply examples/machine_management.zyl
```

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

PostgreSQL is the Phase 3 reference backend. MariaDB, MySQL, SQLite, and SQL
Server adapters are planned for later phases. Live inspection currently reads
tables, columns, and indexes; richer constraint metadata will be expanded in
future iterations.
