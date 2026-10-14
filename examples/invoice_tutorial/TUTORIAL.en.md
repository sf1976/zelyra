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

- Line 1 states the entry file's role.
- Lines 2–7 import the database, shared layout, customers, items, invoices,
  and invoice overview. Aliases make these modules available to the project
  graph while keeping their declarations in `src/`.
- Lines 9–11 declare `GET /` and choose `InvoiceTutorialShell` as its shared
  layout.
- Line 12 begins the page content. Lines 13–21 create the home hero.
  `data-zelyra-i18n` binds its title, description, and links to the bilingual
  entries in `locales/en.json` and `locales/de.json`.
- Lines 23–33 create the customer card. `col-md-4` places three equal cards on medium and
  larger screens. Lines 27–30 provide its badge, title, description, and link
  to `/customers/new`.
- Lines 34–43 repeat the card structure for items and link to `/items/new`.
- Lines 44–53 create the invoice card and link to `/invoices/new`.
- Lines 54–57 close the cards section, main element, HTML output, and page
  declaration. Each opening tag is paired with its closing tag.

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

- Line 1 describes the layout's purpose.
- Line 2 declares the public view `InvoiceTutorialShell`.
- Lines 3–4 begin the HTML output and document.
- Lines 5–10 provide document language/container, mobile viewport, and title.
- Lines 11–18 begin the page body and brand. The mark is text; the subtitle is
  read from `locales/en.json` or `locales/de.json`.
- Lines 19–25 provide navigation to home, customers, items, invoice overview,
  and line items. `data-zelyra-i18n` translates labels; each `href` remains a
  concrete URL.
- Line 26 adds the prominent create-invoice link.
- Line 27 closes the navbar's inner container. Line 28 closes the header.
- Lines 29–31 create the main content area. `<slot />` is where Zelyra places
  the current page or generated CRUD screen.
- Lines 32–34 create the localized footer.
- Lines 35–39 close the HTML body, document, HTML block, and view.

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
- Lines 17–18 declare `InvoiceDashboard` and its typed SQL source. The name
  generates `/views/invoicedashboard`.
- Lines 19–30 select invoice number, customer, and status. Aggregate SQL
  calculates net, tax, and gross values. `LEFT JOIN` keeps invoices without
  line items in the overview.
- Line 31 closes the SQL query block.
- Lines 32–34 select the displayed columns.
- Line 35 enables filtering by status. Line 36 enables search. Line 37 enables
  sorting. Line 38 limits each page to 20 rows.
- Line 39 closes the table view.

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

### `zelyra.theme.css` — local appearance

- Line 1 enables the locally bundled Tabler baseline.
- Line 2 records that the tutorial does not request a CDN.
- Lines 3–12 set light appearance, primary color, background, text, card radius,
  and shadow.
- Lines 14–21 style the body, remove default margins, and choose a readable
  system font.
- Lines 23–28 remove the editor grid from the standalone web application.
- Lines 30–38 reset the outer page so Tabler controls its layout.
- Lines 40–50 style the page wrapper, header, brand, and navigation.
- Line 51 adds space around page content.
- Lines 52–55 style the home hero, its heading and description, and its action links.
- Lines 56–59 style the workflow cards, headings, descriptions, and links.
- Line 60 styles the footer.
- Lines 61–65 standardize spacing, forms, tables, and focus indicators.
- Lines 66–68 style the net/tax totals on the print view.
- Lines 70–83 adapt navigation and content for narrower screens.
- Lines 84–88 hide navigation and footer when printing and remove unnecessary
  padding and shadows.

### `locales/en.json` and `locales/de.json` — translated labels

Both JSON files have the same 30-line structure. Each key is identical at the
same line in both files; only its displayed English or German value differs.
The key names connect the translations to the `data-zelyra-i18n` attributes
and localized CRUD fields. None of these values changes routes or database
behavior.

- Line 1 opens the JSON object. Line 30 closes it.
- Line 2 (`invoice.number.label`) labels an invoice number.
- Line 3 (`invoice.number.auto_hint`) says the server assigns that number when
  saving.
- Line 4 (`invoice.issued_at.label`) labels the issue date.
- Line 5 (`invoice.overview.title`) names the invoice overview.
- Line 6 (`invoice.nav.document_title`) sets the browser document title.
- Line 7 (`invoice.nav.subtitle`) sets the navigation subtitle.
- Line 8 (`invoice.nav.home`) labels the home link.
- Line 9 (`invoice.nav.customers`) labels the customer link.
- Line 10 (`invoice.nav.items`) labels the item link.
- Line 11 (`invoice.nav.invoices`) labels the invoice overview link.
- Line 12 (`invoice.nav.lines`) labels the line-item link.
- Line 13 (`invoice.nav.new_invoice`) labels the new-invoice button.
- Line 14 (`invoice.nav.footer`) supplies the footer text.
- Line 15 (`invoice.home.eyebrow`) supplies the small label above the home title.
- Line 16 (`invoice.home.title`) supplies the main home-page heading.
- Line 17 (`invoice.home.intro`) explains the workflow in one sentence.
- Line 18 (`invoice.home.new_invoice`) labels the create-invoice action.
- Line 19 (`invoice.home.overview`) labels the overview action.
- Line 20 (`invoice.home.workflow`) names the workflow section.
- Line 21 (`invoice.home.customer_title`) names the customer card.
- Line 22 (`invoice.home.customer_text`) explains the customer step.
- Line 23 (`invoice.home.customer_link`) labels its form link.
- Line 24 (`invoice.home.item_title`) names the item card.
- Line 25 (`invoice.home.item_text`) explains the item step.
- Line 26 (`invoice.home.item_link`) labels its form link.
- Line 27 (`invoice.home.invoice_title`) names the invoice card.
- Line 28 (`invoice.home.invoice_text`) explains invoice creation and automatic
  numbering.
- Line 29 (`invoice.home.invoice_link`) labels its form link.
