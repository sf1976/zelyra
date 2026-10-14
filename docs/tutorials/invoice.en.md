# Invoice tutorial

This tutorial walks through a multi-file invoice application: customers,
items, invoices, line items, a live dashboard, and a printable invoice view.
The generated files remain local and can be started from either the command
line or Zelyra Studio.

## Requirements and first start

Install Docker and Docker Compose v2 or newer. Verify the installation with
`docker compose version`. Zelyra does not install Docker for you.

Generate, initialize, and start the tutorial with:

~~~sh
zelyra --tutorial invoice
~~~

The command creates `invoice-tutorial`, writes a protected `.env`, starts
MariaDB, applies the database schema and sample data, starts the application,
and prints its address. Open that address if a browser does not open
automatically. The sample data contains two customers, three items, and the
draft invoice `RE-2026-001` with two line items. Repeated starts do not add
duplicates.

Generate the project files without starting Docker:

~~~sh
zelyra --tutorial invoice ./my-invoices --no-start
cd ./my-invoices
zelyra setup . --all
~~~

`--no-start` still creates `.env`, the Compose file, schema, seed SQL, and
application. `zelyra setup . --all` later performs the full local startup.
Credentials stay in the local `.env`, which Git ignores.

## Start from Zelyra Studio

Open the project in Studio:

~~~sh
zelyra editor ./invoice-tutorial
~~~

Select `main.zyl` in the project tree and click **Run**. Studio starts the
local MariaDB service, waits for the connection, applies the declared schema,
loads the repeatable sample data, and then starts a local web preview. Run
shows the preview address; **Stop preview** ends it. This path needs Docker
Compose and the local tutorial configuration in `.env`.

For a different database configured through process environment variables,
Studio does not start a local database service or insert tutorial data. That
keeps external databases unchanged.

## Commands to explore the project

Run these commands from the tutorial directory:

~~~sh
zelyra check main.zyl
zelyra fmt main.zyl --check
zelyra routes main.zyl
zelyra db map main.zyl
zelyra setup . --database
zelyra setup . --schema
zelyra setup . --all
~~~

`check` validates the imported project. `fmt --check` reports formatting
changes without editing files. `routes` lists pages, APIs, and generated CRUD
routes. `db map` shows declared tables and their module ownership. The setup
modes start MariaDB, apply the schema, or run the complete workflow.

Before a later schema change, inspect its plan and then apply the reviewed
plan id:

~~~sh
zelyra db plan main.zyl --format=json
zelyra db apply main.zyl --plan-id sha256:<plan-id>
~~~

Risky changes require a separate `--allow-risky` flag. Review the plan before
applying it.

## Project files and workflow

~~~text
invoice-tutorial/
├── main.zyl                      entry point and dashboard
├── zelyra.toml                   project configuration
├── zelyra.theme.css              local style overrides
├── invoice-tutorial.seed.sql     repeatable sample data
├── zelyra.tutorial               tutorial marker for the seed step
└── src/
    ├── database.zyl              MariaDB declaration
    ├── shell.zyl                 shared layout
    ├── customers.zyl              customers and CRUD pages
    ├── items.zyl                  items and CRUD pages
    ├── invoices.zyl               invoices, lines, print view
    └── dashboard.zyl              invoice summary
~~~

The dashboard shows the customer and invoice counts along with net, tax, and
gross totals. Start with a customer at `/customers` and an item at `/items`.
Then create a draft at `/invoices` and add its lines at `/invoice_lines`. The
sample records make the dashboard, search, and print view useful immediately.

`zelyra --help` and `zelyra db --help` list commands and options. Studio shows
the imported source files in its project tree. Use `main.zyl` as the entry
point so the compiler and formatter process the whole project.

## Ports and troubleshooting

When creating a new local configuration, Zelyra selects available MariaDB and
web application ports. If Docker Compose still reports a port conflict,
Zelyra automatically selects another free port for a managed `.env`, updates
the matching local database URL, and retries. It prints the resulting
application address. Process environment variables such as
`ZELYRA_DB_HOST_PORT` take precedence; if one explicitly fixes the occupied
port, change or unset it and rerun the command.

Compose v1 is unsupported. If the Compose plugin is missing, install Compose
v2 using the [Docker instructions](https://docs.docker.com/engine/install/)
and verify `docker compose version` again. Existing `.env` files and database
volumes are preserved when a port changes.

## Limits of the example

The template assumes a flat 19% VAT rate for every line. It is a learning
example, not a complete invoicing or accounting system. Tax cases, required
legal fields, GoBD requirements, email delivery, and native PDF generation are
not fully implemented. Review and add those requirements before production
use.
