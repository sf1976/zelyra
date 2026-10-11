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
4. **Invoices** — create a draft, select its customer, and manage its number
   and status at `/invoices`.
5. **Invoice lines** — select an invoice and item, then enter quantity and
   net unit price at `/invoice_lines`. The invoice view calculates the net
   amount, 19% VAT, and gross amount from these lines.

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

## View and save an invoice as PDF

Open `/invoices/INVOICE-NUMBER/print` with the invoice number. The page totals its
lines and shows net, 19% VAT, and gross amounts. Select **Print invoice or save
as PDF** to open the browser's print dialog, then choose **Save as PDF**. The
print stylesheet hides the navigation and print button.

This example assumes the German standard VAT rate of 19% for every line and
stores item prices as net prices. Other rates, exemptions, and special cases
must be represented correctly in a real application. This is a learning
example: it does not validate every legally required invoice detail, provide
accounting compliance features, send email, or generate a native PDF inside
Zelyra. Add and review the applicable requirements before production use.
