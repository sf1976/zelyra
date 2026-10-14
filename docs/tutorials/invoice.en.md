# Invoice tutorial

This multi-file example includes customers, items, invoices, line items, a
filterable invoice overview, and a printable invoice. The full bilingual
command and source walkthrough is in the
[generated project guide](../../examples/invoice_tutorial/TUTORIAL.en.md).
`zelyra --tutorial invoice` writes both `TUTORIAL.en.md` and
`TUTORIAL.de.md` into the generated project.

## Requirements and startup

Install Docker Engine with the Compose plugin v2 or newer. Check it with
`docker compose version`. Python Compose v1 is unsupported; Zelyra does not
install Docker.

~~~sh
zelyra --tutorial invoice
~~~

The command creates `invoice-tutorial`, writes local credentials to the
Git-ignored `.env`, configures MariaDB and the schema, adds sample data, and
starts the web application. Open the printed URL if a browser does not open.
The sample data has two customers, three items, one draft invoice, and two line
items. Repeated starts do not insert duplicates.

Generate files only:

~~~sh
zelyra --tutorial invoice ./my-invoices --no-start
cd ./my-invoices
zelyra setup . --all
~~~

`--no-start` skips Docker and the web preview but still writes `.env`, Compose,
schema, seed, and application files. Later, `zelyra setup . --all` performs
the full local startup. When a managed application or MariaDB port is busy,
the tutorial selects another free port and prints the new URL.

## Run from Studio and edit

~~~sh
zelyra editor ./invoice-tutorial
~~~

Open `main.zyl` in the file tree and click **Run**. Studio starts the matching
local tutorial database, waits for MariaDB, applies the schema, loads the
repeatable seed, and then opens the web preview. Use **Stop** to end the
preview. **Logs** in the Studio header opens local rotating JSONL records for
program and preview output, with project-scoped access and text filters.

The `/` home page guides the customer, item, and invoice steps. The filterable
table is named **Invoice overview** and remains at `/views/invoicedashboard`.
`GET /invoices/new` is generated from `crud Invoice -> invoices` in
`src/invoices.zyl`; there is no separate `page` declaration for it.
`zelyra routes main.zyl` lists declared and generated routes with their source
files.

**Issued on** uses the `Date` type and accepts a date without a time. Its
localized format hint is `YYYY-MM-DD` or `JJJJ-MM-TT`. The server assigns the
invoice number from the newly generated record ID and preserves it during later
edits. `TUTORIAL.en.md` and `TUTORIAL.de.md` explain each source section and
the command options.

## Troubleshooting

- **Compose is missing:** Install Docker Compose plugin v2 using the
  [Docker instructions](https://docs.docker.com/engine/install/) and check
  again with `docker compose version`.
- **A port is busy:** For managed `.env` ports, Zelyra tries a free next port
  and updates the matching database addresses. An explicit process variable
  such as `ZELYRA_DB_HOST_PORT` takes precedence; remove or change it if it
  forces a busy port.
- **`ERROR 1045 (28000): Access denied`:** The credentials do not match the
  already initialized MariaDB volume. Credentials are applied when a volume is
  first created. New tutorials get an isolated Compose namespace. For older
  marked projects, recovery starts a separate database and leaves the old
  volume untouched.
- **Studio output looks empty:** Open **Logs** to check whether the process
  emitted output. For a web app, **Run** shows the local preview URL; open it or
  click **Open application in browser**.

`.env` contains local secrets and must not be committed or shared. The example
uses a flat 19% tax rate for every invoice line and is not complete accounting
or invoicing software.
