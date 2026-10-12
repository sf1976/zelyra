# Invoice tutorial

`zelyra --tutorial invoice` creates a new project and prepares it immediately:
Zelyra writes the application, local `.env`, Docker Compose files, and polished
theme, starts MariaDB and the web application, and applies the database schema.
You do not need to set `DATABASE_URL` manually or run separate `db setup` or
`serve` commands.

A working Docker installation with Docker Compose is required. Zelyra does not
install Docker. If Compose is unavailable or a container cannot start, the
project files remain in place; after fixing Docker, run
`cd invoice-tutorial && zelyra setup --all` to start the database and
application and apply the schema together. For a different target directory,
pass its path, for example `zelyra --tutorial invoice ./my-invoices`.
Append `--no-start` to generate the project without starting Docker; the command
still creates a protected local `.env` with `ZELYRA_DATABASE_MAIN_URL` and the
compatible `DATABASE_URL` fallback; it does not print the credentials. The
database and app start later with `zelyra setup --all`. The command prints this
startup instruction and the expected browser address. `zelyra new ...
--mariadb` shows the address when creating a MariaDB project; it is reachable
after startup. If schema setup fails after the app starts, the error output
still includes the address.

The ready application runs at the local address printed by the command
(`http://127.0.0.1:3000` by default). It contains five connected areas:

1. **Customers** — create and search customer records at `/customers`.
2. **Items** — maintain reusable products, prices, and active status at
   `/items`.
3. **Dashboard** — start at `/`. Real data shows customer and invoice counts,
   stored net and gross invoice values, and VAT. Totals include drafts and are
   not a posted-revenue report. A guided four-step flow links to the forms and
   invoice overview.
4. **Invoices** — create a draft, select its customer, and manage its number
   and status at `/invoices`.
5. **Invoice lines** — select an invoice and item, then enter quantity and net
   unit price at `/invoice_lines`. The invoice view calculates net, 19% VAT,
   and gross amounts from these lines.

## Project structure: one responsibility per file

The tutorial is a real multi-file project. `main.zyl` is the entry point and
imports the feature modules. Each business area has its own source file:

~~~text
invoice-tutorial/
├── main.zyl                 entry point and home page
├── zelyra.toml              project and runtime configuration
├── zelyra.theme.css         design overrides
└── src/
    ├── database.zyl         shared MariaDB declaration
    ├── shell.zyl            reusable page layout
    ├── customers.zyl        customer model, table, and CRUD pages
    ├── items.zyl            item model, table, and CRUD pages
    ├── invoices.zyl          invoices, lines, and print view
    └── dashboard.zyl        totals query and invoice overview
~~~

This lets a beginner read or change one part at a time while the compiler and
runtime check the complete imported project together. Run `zelyra check
main.zyl` and `zelyra routes main.zyl` from the project directory. Studio shows
the same source files in its project tree.

The desktop dashboard uses four metric cards, a guided workflow, and a summary
of customer, invoice, and line relationships. Navigation and CRUD views adapt
to tablet and narrow screens; tables remain horizontally usable. Tabler Core
1.6.1 ships with Zelyra and is served locally. The application does not load a
CDN or Tabler JavaScript. The print view hides navigation and controls. Run
`zelyra editor invoice-tutorial` to open Studio and inspect or edit the source
and database declarations.

## View and save an invoice as PDF

Open `/invoices/INVOICE-NUMBER/print` with the invoice number. The page totals
its lines and shows net, 19% VAT, and gross amounts. Select **Print invoice or
save as PDF** to open the browser's print dialog, then choose **Save as PDF**.
The print stylesheet hides the navigation and print button.

This example assumes the German standard VAT rate of 19% for every line and
stores item prices as net prices. Other rates, exemptions, and special cases
must be represented correctly in a real application. This is a learning
example: it does not validate every legally required invoice detail, provide
accounting compliance features, send email, or generate a native PDF inside
Zelyra. Add and review the applicable requirements before production use.
