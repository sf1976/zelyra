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
then prints the combined `zelyra setup --all` startup instruction.

The ready application runs at the local address printed by the command
(`http://127.0.0.1:3000` by default). It contains five connected areas:

1. **Customers** — create and search customer records at `/customers`.
2. **Items** — maintain reusable products, prices, and active status at
   `/items`.
3. **Dashboard** — start at `/` and inspect the searchable invoice overview at
   `/views/invoicedashboard`.
4. **Invoices** — create a draft, select its customer, and manage its number
   and status at `/invoices`.
5. **Invoice lines** — select an invoice and item, then enter quantity and net
   unit price at `/invoice_lines`. The invoice view calculates net, 19% VAT,
   and gross amounts from these lines.

The responsive theme styles navigation, forms, buttons, tables, and a dedicated
print view. Run `zelyra editor invoice-tutorial` to open Studio and inspect or
edit the program and its database declarations.

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
