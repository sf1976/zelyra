# Checking invoices and inventory as separate Docker applications

🧪 Experimental module-export rehearsal for 0.5.0; not a general export guarantee.
[Deutsch](module-docker-acceptance.de.md)

✅ This bounded rehearsal passed locally on 2026-10-10, including writable
CRUD in both exports and the negative checks listed below. The rehearsal source
was `c7718d5774b3ed10ca0ff1984a545acf35c6341f`; the generated application used
the published `v0.4.0` runtime, and both export images built their compiler
from published commit `507c29e95084a59029d6b436bd69d9132c3a8937`. An earlier
rehearsal passed on 2026-10-09 with commit
`917e8c707e49332b323e51e4dcbd39a4f37ced96`.

The reference test generates a combined application with two business modules
and a shared database module. It then exports both business modules and starts
them sequentially as separate Docker applications. The original web server is
stopped first. MariaDB remains an external service.

## What is separated?

| File | Purpose | Invoice export | Inventory export |
| --- | --- | --- | --- |
| `main.zyl` | combined application entry | newly generated entry | newly generated entry |
| `src/database.zyl` | shared MariaDB declaration | included | included |
| `src/invoices.zyl` | invoice table and CRUD | included | excluded |
| `src/inventory.zyl` | inventory table and CRUD | excluded | included |

Both business modules import `src/database.zyl`. The `database main`
declaration determines the runtime variable name `ZELYRA_DATABASE_MAIN_URL`.
Each exported process receives its own value. This allows different databases
per application; it does not implement multiple connections within one process.

## Run the complete test

Requirements are a checkout of this development version, Rust/Cargo, Docker
with Compose, Bash and curl. Docker must be accessible. Local ports 18082,
18083, 18084 and 3309 must be free. The test builds compiler images from GitHub,
requiring network access and several minutes of build time. It does not access
an existing MariaDB installation.

From the repository root:

```bash
cargo build --locked -p zelyra-cli
ZELYRA_DOCKER_E2E_REF=v0.4.0 \
ZELYRA_DOCKER_E2E_MODULE_COMMIT=507c29e95084a59029d6b436bd69d9132c3a8937 \
bash tests/generated-project-docker-e2e.sh
```

These values pin the runtime compiler and the compiler built into both export
images to the published `v0.4.0` commit. The current 0.5 branch generates the
application and export packages; a 0.5.0 release rehearsal must pin both
compiler references to the exact candidate.

The script creates a temporary project, its own Compose project names and a
disposable MariaDB volume. It creates test data and credentials exclusively
for this fixture. On exit it removes its containers, volume and temporary
project; generated Docker images remain cached. An interrupted test is not a
successful result.

## What the workflow checks

1. The combined application serves invoice and inventory records over HTTP.
2. `module bundle ... crud:Invoice --docker --compiler-ref ...` generates the
   invoice package; `crud:Inventory` generates its counterpart. Each contains
   the database source module and excludes the other business module.
3. Without the required `.env`, Compose rejects configuration with a diagnostic
   identifying that file. This does not test every invalid or empty connection
   value.
4. Both packages build and start with separate credentials and separate schemas
   on the same test MariaDB server. Each list displays its expected record
   while the original web server is stopped. The test creates, updates, and
   deletes a record through each generated HTTP CRUD form, including CSRF and
   same-origin checks.
5. The other business module's route returns HTTP 404. The image contains no
   `/app/.env`; Compose injects configuration only at runtime.
6. Each test account has `SELECT`, `INSERT`, `UPDATE`, and `DELETE` only on its
   own table. MariaDB denies reading and deleting from the other module's
   table. Only an actual permission diagnostic passes the negative test, not
   an arbitrary connection error.

## Limits and troubleshooting

The fixture proves writable CRUD for these two bounded exports; it does not
prove role management, schema upgrades between versions, or complete
extraction of arbitrary projects. Compiler contracts and MariaDB grants are
separate security boundaries. `complete_deployment` remains `false`.

For occupied ports, inspect your test processes first; do not stop unrelated
services. `docker compose ps` in the respective project shows its containers.
Missing Docker permissions, compiler-build network failures and an incorrect
compiler commit must be resolved before evaluating results. Never copy `.env`
or full connection strings into reports. TLS is explicitly disabled only on
the isolated test network; a normal export uses `ZELYRA_DB_TLS_MODE=auto`.

Next: [0.4.0 acceptance](release-plans/0.4.0.en.md),
[0.5.0 module goal](release-plans/0.5.0.en.md),
[handbook](handbook/en/handbook.md).
