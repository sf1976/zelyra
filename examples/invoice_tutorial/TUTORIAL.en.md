# Invoice tutorial: commands and source explained

This file is generated with the project. It explains startup, Studio, and the
source modules. Line numbers refer to the files shipped with the template.
Blank lines separate sections; braces, HTML tags, and SQL parentheses delimit
the blocks they surround.

## Start and edit

Install Docker Engine with the Compose plugin v2 or newer. Check it with
`docker compose version`. Python Compose v1 is unsupported.

~~~sh
zelyra --tutorial invoice
~~~

`zelyra` starts the CLI. `--tutorial` selects tutorial generation, and
`invoice` selects this template. Without a target directory, the new directory
is `invoice-tutorial`. The command writes the project, local credentials,
schema, and repeatable sample data, starts MariaDB and the web application, and
prints its local URL.

~~~sh
zelyra --tutorial invoice ./my-invoices --no-start
cd ./my-invoices
zelyra setup . --all
~~~

The first command names `./my-invoices` as its target. `--no-start` skips
Docker and the web preview, but still writes all project files and the
protected `.env`. `cd` changes into the project directory. `setup` reads the
local project configuration; `.` means the current directory and `--all`
starts the database, applies the schema, loads sample data, and starts the
application.

`zelyra setup [directory]` also accepts one of `--database` (database service),
`--schema` (apply the schema), or `--all` (the complete flow).
`--host-port <port>` selects the application host port and
`--db-host-port <port>` selects the MariaDB host port. If a managed port is
busy, Zelyra tries another free port. `--web [--port <port>]` starts a browser
application without starting a Compose database; the database must already be
reachable.

~~~sh
zelyra editor ./my-invoices
~~~

`editor` opens Zelyra Studio for that directory; without a directory it uses
`.`. `--port <port>` sets the Studio port (default `4177`); `--no-open` skips
automatic browser launch. Select `main.zyl` in the file tree and click
**Run**. Studio starts the matching tutorial database, applies the schema,
loads the seed, and opens a web preview. The **Logs** button in the top bar
opens local JSONL logs with severity and text filters.

Run these inspection commands from the project directory:

~~~sh
zelyra check main.zyl
zelyra fmt main.zyl --check
zelyra routes main.zyl
zelyra db map main.zyl
zelyra db plan main.zyl --format json
~~~

`check` compiles the imported project. `fmt` formats source; `--check` reports
differences without writing files. `routes` lists declared and generated URLs
with their source locations. `db map` shows tables and their source modules.
`db plan` compares the source model with the live schema without changing it;
`--format json` emits machine-readable output.

Apply a schema change only after reviewing its plan:

~~~sh
zelyra db apply main.zyl --plan-id sha256:<plan-id>
~~~

`--plan-id` binds the apply operation to the exact plan shown earlier. Risky
changes also require `--allow-risky` and an explicit confirmation. Review the
plan before applying it.

## Where `/invoices/new` comes from

There is no `page "/invoices/new"` declaration in the source. The route is
generated from `crud Invoice -> invoices` in `src/invoices.zyl`. `crud` asks
Zelyra to create the standard list, form, detail, and edit paths for the
`invoices` table. The resource name to the right of `->` becomes the URL base:

- `GET /invoices` shows the list.
- `GET /invoices/new` shows the create form.
- `POST /invoices/new` saves the invoice.
- `GET /invoices/{id}/edit` and `POST /invoices/{id}/edit` edit an existing
  record.

By contrast, `page "/invoices/{invoice_number}/print"` in the same file is an
explicit route declaration. `zelyra routes main.zyl` prints both kinds and
their source locations. `/views/invoicedashboard` comes from the
`tableview InvoiceDashboard` declaration in `src/dashboard.zyl`; the URL stays
for compatibility while its displayed title is “Invoice overview”.

## Invoice date and number

`issued_at` is declared as `Date` in both the record and database table. The
browser therefore shows a date picker without a time. Its hint says
`YYYY-MM-DD` in English and `JJJJ-MM-TT` in German; validation accepts that
same format. The database stores only the calendar date.

The create form makes `invoice_number` read-only. The server replaces its
submitted value with a random temporary value, inserts the invoice inside the
same database transaction, reads the generated ID, and stores `RE-<ID>` as the
final number. Existing numbers are checked and a suffix is added if needed.
The edit form omits the number, so it stays unchanged. The sample invoice keeps
the number `RE-2026-001`.

## Source files and line walkthrough

### `main.zyl` — project entry and home page

- Line 1 says this file connects the business modules and contains the home page.
- Line 2 imports the compiler-checked `zelyra/ui` standard library; HTML uses its component names directly. Lines 3–8 import the database, shared layout, customers, items, invoices, and overview as local module aliases.
- Lines 9–11 declare `GET /`, select `InvoiceTutorialShell`, and begin HTML output. The `/` route is explicit here.
- Line 12 opens the main area. Lines 13–14 start the two-column hero and sample-invoice layout.
- Lines 15–19 create the label, Z mark, and local-status badge. `data-zelyra-i18n` labels are replaced from the locale files.
- Line 20 sets the translated main heading; line 21 sets the introduction.
- Lines 22–25 create the action area and links: `/invoices/new` starts invoice creation and `/views/invoicedashboard` opens the overview.
- Lines 26–30 show the local-data note and close the hero.
- Lines 32–39 begin the sample invoice. Its visible number `RE-2026-001` matches the seed; its labels and draft status are translated.
- Lines 40–45 separate the heading from the hard-coded sample recipient. The `.test` email is sample data.
- Lines 46–55 show issue date and VAT rate in two columns. The date and labels are localized for English and German.
- Lines 56–71 display the two sample line items with quantities, unit prices, and line totals.
- Lines 72–76 show net, 19% VAT, and gross totals. They match the seed data (€298.00 net, €56.62 tax, €354.62 gross).
- Lines 77–79 link to the sample invoice's real print view and close the top grid.
- Lines 81–103 show three feature notes for date entry, automatic numbering, and printing; each `<article>` groups one note, `<strong>` gives its title, and `<p>` gives its explanation.
- Lines 105–112 set the next section's heading, caption, and link.
- Lines 114–142 use `Grid columns="three"` for the customer, item, and invoice steps. The component handles responsive columns; the links lead to `/customers/new`, `/items/new`, and `/invoices/new`.
- Lines 143–146 close the workflow section, home page, HTML output, and route.
- Lines 148–155 declare `/ui/components` as a component gallery and warn that presentation does not replace server validation.
- Lines 156–180 demonstrate the four compiler-checked appearances, action and submit buttons, a hint slot, a real error state, and a native GET form.
- Lines 182–209 demonstrate date, number, select, text, radio, checkbox, and switch fields. Appearance is separate from the `invalid` state.
- Lines 210–215 show a captioned responsive table with column and row headers and a real detail link.
- Lines 216–229 show server-rendered bar, donut, and line charts. The line chart has a readable table alternative.
- Lines 230–259 demonstrate progress, breadcrumbs, pagination, empty/loading/error states, toast, disclosure, dropdown, keyboard tabs, and dialog.
- Lines 260–269 explain browser print/PDF, a local email draft prefilled with the invoice number, and the gallery print action. Save the invoice as a PDF first, then attach it to the draft manually. The email link does not send mail itself; server email uses an authorized route, `mail_queue`, and the outbox worker, which does not yet support attachments.
- Lines 270–272 close the gallery and route.
- Line 274 declares `page "/tour"` as an interactive architecture and code tour with collapsible source disclosures (`Disclosure`), code walkthroughs for all core modules, and a direct comparison with traditional multi-layer web stacks.

### `src/database.zyl` — shared connection

- Line 1 explains why the declaration is shared.
- Line 2 starts database `main`.
- Line 3 selects the MariaDB backend.
- Line 4 closes the declaration. Credentials come from the local `.env`, not
  this source file.

### `src/customers.zyl` — customers

- Lines 1–2 import the database and shared layout.
- Lines 4–8 define public record `Customer`: `id` is the database identifier,
  `name` is text, and `email: Email?` is optional.
- Lines 10–17 declare table `customers`. The ID is generated; name is required
  and limited to 120 characters; email may be empty. `access.read` lists the
  modules allowed to read customers for their queries.
- Lines 19–30 generate customer CRUD. `layout` and `title` control its shell and
  title. `view.fields` limits visible fields; `mode: cards` selects card
  layouts for list, detail, and form. `list` selects list columns and `search`
  selects searchable columns.

### `src/items.zyl` — items

- Lines 1–2 import the database and shared layout.
- Lines 4–10 define `Item` with ID, SKU, name, exact `Decimal` net price, and
  Boolean active status.
- Lines 12–21 declare the table. SKU is required, at most 40 characters, and
  unique; name is required and at most 120 characters. Price is required and
  `active` defaults to `true`. The read grants allow invoice forms and the
  overview to load item data.
- Lines 23–35 generate item CRUD. Field lists control the form and list;
  `search` chooses searchable fields and `filter` enables the Boolean active
  filter.

### `src/shell.zyl` — shared layout

- Line 1 imports Zelyra UI for this module; line 3 describes its shared use.
- Line 4 declares the public view `InvoiceTutorialShell`.
- Lines 6–11 open an HTML document with `data-z-ui`, UTF-8, a mobile viewport, and a localized title.
- Lines 13–14 open the document body and `AppShell`. Its `id` connects the navigation toggle to the navigation; the label is accessible text.
- Lines 15–21 fill the named header slot with the brand and invoice action.
- Lines 22–29 fill navigation with real links to home, customers, items, invoices, line items, and the component gallery. `data-zelyra-i18n` translates labels while routes stay unchanged.
- Line 30 places the default content slot in the page area so authored and generated CRUD screens share the layout.
- Lines 31–33 add the footer. Lines 34–38 close AppShell, body, HTML, the view block, and the view.

### `src/invoices.zyl` — invoices and print view

- Lines 1–4 import the database, customer, item, and layout modules.
- Lines 6–12 define `Invoice`: number, customer, calendar date, and status.
- Lines 14–20 define `InvoiceLine`: invoice, item, quantity, and unit price.
- Lines 22–31 declare `invoices`. The ID is generated; invoice number is
  required and unique; customer is required; `issued_at` is a required `Date`
  defaulting to today; new invoices start as `draft`. The read grants permit
  the print view, overview, and home page to query invoice data.
- Lines 33–42 declare `invoice_lines` and its required invoice/item references,
  quantity, and price.
- Lines 44–56 generate invoice CRUD. Line 46 supplies a localized title. The
  field lists configure the form, detail, and search view. Zelyra marks the
  invoice number as generated; the edit form omits it so the value is retained.
- Lines 58–68 generate CRUD for invoice lines.
- Lines 70–79 type the print query results, including net, 19% tax, and gross
  totals.
- Lines 81–87 type each printed item's name, SKU, quantity, price, and net line
  amount.
- Lines 89–106 explicitly declare the print route. `invoice_number` binds the
  query to one invoice. `JOIN` loads its customer; `LEFT JOIN` keeps invoices
  that have no line items. `COALESCE` turns missing totals into zero, `SUM`
  adds line amounts, `ROUND(..., 2)` rounds displayed currency to cents, and
  `GROUP BY` creates one summary per invoice.
- Lines 107–119 load invoice lines. Joins combine item and invoice data;
  `ORDER BY` keeps their display order stable.
- Lines 120–141 build the printable page. The button calls browser printing;
  `no-print` hides it on paper. The loop renders each `line`. The totals card
  separates net, tax, and gross values.
- Line 142 closes the page.

### `src/dashboard.zyl` — filterable invoice table

- Lines 1–5 import the database, customer, item, invoice, and layout modules.
- Lines 7–15 type the returned columns and calculated decimal totals.
- Lines 17–19 declare `InvoiceDashboard`, attach the shared
  `InvoiceTutorialShell`, and define the typed SQL source. The layout keeps the
  overview's header, navigation, content, and footer consistent with the rest
  of the tutorial. The tableview name generates `/views/invoicedashboard`.
- Lines 20–32 select invoice number, customer, and status. Aggregate SQL
  calculates net, tax, and gross values. `LEFT JOIN` keeps invoices without
  line items in the overview.
- Line 33 closes the SQL query block.
- Lines 34–36 select the displayed columns.
- Line 37 enables filtering by status. Line 38 enables search. Line 39 enables
  sorting. Line 40 limits each page to 20 rows.
- Line 41 closes the table view.

### `invoice-tutorial.seed.sql` — sample data

- Lines 1–2 explain that stable email addresses, SKUs, and invoice number make
  the seed repeatable.
- Lines 3–7 insert `Beispiel GmbH` only when its email is absent.
- Lines 9–13 add `Nordlicht Büro` the same way.
- Lines 15–19 add the consulting item if its SKU is missing.
- Lines 21–25 add the maintenance item.
- Lines 27–31 add the workstation package.
- Lines 33–39 add a draft invoice for the first customer. The date contains
  only year, month, and day; `NOT EXISTS` prevents a duplicate.
- Lines 41–49 connect two consulting items to the draft if that line does not
  already exist.
- Lines 51–59 add a maintenance line. Each `INSERT ... SELECT` uses stored IDs
  and `NOT EXISTS`, so another start does not create duplicate data.

### `zelyra.theme.css` — one shared appearance

This file adapts Zelyra UI to the tutorial's green business style. It is both
the project theme and the template copied when the invoice tutorial is created.
The two repository copies are kept in sync.

- The `--z-color-*` variables set colors with their contrast pairs, surfaces,
  and focus. Corners and content width use Zelyra UI variables.
- `InvoiceTutorialShell` in `src/shell.zyl` provides the same header,
  navigation, content area, and footer on every page.
- One shared content frame styles generated table views, customer, item,
  invoice, and line-item pages, including their detail and form views.
- Search and filter fields use Zelyra UI component rules. Cards, details, and
  forms share the same surfaces, borders, and spacing as the invoice overview.
- Custom home and invoice-print styles extend the system. Layouts respond to
  smaller screens; wide tables can scroll within their own area.
- The dark theme uses adjusted color variables. Print rules hide navigation
  and explicitly marked actions.

`examples/invoice_tutorial.theme.css` is the template copied by `zelyra
--tutorial invoice` into the project as `zelyra.theme.css`.

### `locales/en.json` and `locales/de.json` — localized labels

Both files contain the same keys in the same order. The line walkthrough applies to both: `en.json` maps each stable key to English display text and `de.json` maps it to German. `main.zyl`, the shared layout, and generated forms refer to these keys; their values do not change routes or database access.

- Line 1 opens the JSON object; line 61 closes it.
- Line 2 (`invoice.number.label`) labels the invoice-number field.
- Line 3 (`invoice.number.auto_hint`) explains automatic assignment when saving.
- Line 4 (`invoice.issued_at.label`) labels the issue date.
- Line 5 (`invoice.overview.title`) sets the invoice overview title.
- Line 6 (`invoice.nav.document_title`) sets the browser-tab title.
- Line 7 (`invoice.nav.subtitle`) sets the subtitle beside the brand.
- Line 8 (`invoice.nav.home`) labels the home link.
- Line 9 (`invoice.nav.customers`) labels the customer link.
- Line 10 (`invoice.nav.items`) labels the items link.
- Line 11 (`invoice.nav.invoices`) labels the overview link.
- Line 12 (`invoice.nav.lines`) labels the invoice-line link.
- Line 13 (`invoice.nav.new_invoice`) labels the new-invoice button.
- Line 14 (`invoice.nav.footer`) supplies the footer note.
- Line 15 (`invoice.home.eyebrow`) supplies the small label above the heading.
- Line 16 (`invoice.home.title`) supplies the home-page heading.
- Line 17 (`invoice.home.intro`) explains the sample workflow.
- Line 18 (`invoice.home.new_invoice`) labels the primary action.
- Line 19 (`invoice.home.overview`) labels the invoice-overview link.
- Line 20 (`invoice.home.workflow`) names the invoice workflow.
- Line 21 (`invoice.home.customer_title`) names the customer step and card.
- Line 22 (`invoice.home.customer_text`) explains the customer step.
- Line 23 (`invoice.home.customer_link`) labels the customer-form link.
- Line 24 (`invoice.home.item_title`) names the item step and card.
- Line 25 (`invoice.home.item_text`) explains the item step.
- Line 26 (`invoice.home.item_link`) labels the item-form link.
- Line 27 (`invoice.home.invoice_title`) names the invoice step and card.
- Line 28 (`invoice.home.invoice_text`) explains customer selection and number assignment.
- Line 29 (`invoice.home.invoice_link`) labels the invoice-form link.
- Line 30 (`invoice.home.local_badge`) marks the local sample application as ready.
- Line 31 (`invoice.home.proof`) says that data stays in the local project.
- Line 32 (`invoice.home.sample_label`) marks the invoice as a sample.
- Line 33 (`invoice.home.sample_status`) shows the sample invoice draft status.
- Line 34 (`invoice.home.billed_to`) labels the invoice recipient.
- Line 35 (`invoice.home.issue_date`) labels the issue date.
- Line 36 (`invoice.home.tax_rate`) labels the VAT rate.
- Line 37 (`invoice.home.tax_rate_value`) shows the tax rate used on the print view.
- Line 38 (`invoice.home.sample_date`) shows the localized sample date.
- Line 39 (`invoice.home.sample_item_1`) names the first sample item.
- Line 40 (`invoice.home.sample_item_1_detail`) shows the first item's quantity and net unit price.
- Line 41 (`invoice.home.sample_amount_1`) shows the first item's net line total.
- Line 42 (`invoice.home.sample_item_2`) names the second sample item.
- Line 43 (`invoice.home.sample_item_2_detail`) shows the second item's quantity and net unit price.
- Line 44 (`invoice.home.sample_amount_2`) shows the second item's net line total.
- Line 45 (`invoice.home.net_total`) labels the net total.
- Line 46 (`invoice.home.sample_net`) shows the sample invoice net amount.
- Line 47 (`invoice.home.tax_total`) labels VAT and the tax rate.
- Line 48 (`invoice.home.sample_tax`) shows the sample invoice tax amount.
- Line 49 (`invoice.home.gross_total`) labels the gross amount due.
- Line 50 (`invoice.home.sample_gross`) shows the sample invoice gross amount.
- Line 51 (`invoice.home.open_sample`) labels the print-view link.
- Line 52 (`invoice.home.feature_date_title`) titles the date feature.
- Line 53 (`invoice.home.feature_date_text`) explains date-only entry without a time.
- Line 54 (`invoice.home.feature_number_title`) titles automatic numbering.
- Line 55 (`invoice.home.feature_number_text`) explains when the number is assigned.
- Line 56 (`invoice.home.feature_print_title`) titles the print feature.
- Line 57 (`invoice.home.feature_print_text`) explains the calculated totals.
- Line 58 (`invoice.home.workflow_kicker`) labels the workflow section as three steps.
- Line 59 (`invoice.home.workflow_title`) sets its main heading.
- Line 60 (`invoice.home.all_invoices`) labels the link to all invoices.
