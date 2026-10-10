# Invoice tutorial

Run `zelyra --tutorial invoice` to create `invoice-tutorial`, or pass another
new directory: `zelyra --tutorial invoice ./my-invoices`. The command refuses
to write into a directory that already exists and checks the generated
application with the compiler.

The tutorial produces a local MariaDB project with five connected parts:

1. **Customers** — create and search customer records at `/customers`.
2. **Items** — maintain reusable products, prices, and active status at `/items`.
3. **Dashboard** — start at `/` and inspect the searchable invoice overview at
   `/views/invoicedashboard`.
4. **Invoices** — create a draft, select its customer, enter its number and
   currency-neutral total, and track its status at `/invoices`.
5. **Invoice lines** — select an invoice and item, then enter quantity and
   unit price at `/invoice_lines`.

The command prints the same local MariaDB start and setup instructions used by
`zelyra new --template mariadb-crud`, followed by the application start command.
Once the database is running and `zelyra db setup main.zyl` succeeds, run:

```sh
zelyra serve main.zyl
```

Open `http://127.0.0.1:3000`, follow the five steps in order, and use
`zelyra editor .` to inspect the source while you work. The generated program
is in `main.zyl`; the database declaration and connection settings are in the
usual project files.

## Learning scope

This is a compact CRUD learning project. It records a total entered by the
user and does not calculate tax or line totals. It does not provide legal
invoice numbering, PDF generation, email delivery, payment processing, or
production accounting rules. Review local requirements and add those features
before using an application for real invoices.
