# Zelyra Studio

For VS Code, Cursor, and Neovim, Zelyra also provides the standalone language
server `zelyra lsp`. Setup, supported features, and limits are documented in
the [LSP guide](lsp.en.md).

Zelyra Studio is the local web editor included with the CLI. Start it from a
project directory:

```sh
zelyra editor
```

The CLI starts local Studio and opens the default browser. If no graphical
browser session is available, it prints the URL for manual use. On SSH or
headless systems, skip browser launch with `zelyra editor --no-open`. Editing
does not require a running database or the web application.

The editor listens only on `127.0.0.1`. Its startup address contains a random
session token. Mutating requests require that token and the same browser
origin.

Studio opens with a project-aware welcome screen. From there, open a Zelyra
source file, check the project, start the console, or open Database Studio.
The page presents compiler, SQL, and application preview as one Zelyra
workflow.

The welcome page also reads the local compiler project map and shows recognized
modules, data models, web surfaces, and SQL queries. Its suggested next action
leads to the project check, project map, Database Studio, or entry point. When
analysis is incomplete, Studio says so and recommends the compiler check first.
The overview does not edit source files and needs no AI or cloud service.

The application menu groups common actions under **File**, **Edit**, **View**,
**Run**, **Database**, and **Help**. Start and Check remain available as quick
actions; specialist tools live in their matching menus or the command palette
(**Ctrl+Shift+P**). **F5** starts the project and **F8** checks it. Menus work
with the keyboard and mouse.

Open the optional terminal with **View → Integrated Zelyra Terminal** or
**Ctrl+`**. **Start Console** launches `zelyra console` in the current project
folder. Enter Zelyra expressions interactively, follow their output, and stop
the session when finished. This terminal does not run operating-system shell
commands; use Zelyra tools and Studio actions to manage files and processes.
Output is bounded and known local credentials are redacted.
For PostgreSQL projects, the console reports the current Zelyra REPL limit:
sandbox transactions are currently available only with MariaDB/MySQL.

## Choose an appearance

Use the color control in the upper-right corner to switch between light and
dark appearances. Studio stores the choice in this browser and applies it
before loading the stylesheet on the next visit, avoiding a flash from the dark
default. The control is a standard button and works with Tab, Enter, or
Space. The editor, syntax colors, file tree, help panel, database workbench,
and dialogs share the same theme tokens. The browser E2E check requires at
least 4.5:1 contrast for light-theme text and syntax colors on the work
surfaces. At the supported minimum width of 360 CSS pixels, the appearance
control remains reachable without horizontal overflow.

## Use the IDE tools

Press **F1** or choose the help action to see a description, options, and an
example for a supported language term. **Ctrl+Shift+P** opens the searchable
action finder. **Ctrl+Space** requests completion; available suggestions include
project declarations and supported local parameters and bindings. F12 opens a
resolved function or record definition in the current or an imported file.
Shift+F12 shows indexed references. These actions use the compiler's linked AST
and analyze all open, unsaved `.zyl` files together. Record references include
record literals, local record patterns, type aliases, function parameter and
return types, local binding annotations, and record-field, table-column,
component-property, form, API-input, page-input, and tableview type annotations.
Hover over a resolved symbol or press **Alt+Shift+H** to show its compiler
description; Escape closes a keyboard-opened hover.
Record field rename covers declarations, record literal labels, and accesses
whose receiver type resolves from a function parameter, a local type
annotation, a directly constructed record, or the return type of a uniquely
resolved function. Field labels in resolvable record patterns are renamed
separately from local aliases. No rename is offered for a receiver without a
statically resolvable type.
Project-wide free-text search remains a separate tool.

Completion uses declarations resolved from the active module and its imports.
After `import "src/customer.zyl" as customer`, **Ctrl+Space** offers exported
names as `customer::Name`. While entering an import path, Studio suggests
matching `.zyl` files from the project. SQL blocks offer SQL keywords and
declared tables. Within a SQL body, completion also offers declared columns
after a table reference or alias, and table names after `FROM` or `JOIN`. These
suggestions come from the local compiler project map, including open source
overlays; ambiguous table names are omitted and no live database is queried.
An empty file or a database, table, or function declaration can offer a
matching starter block. After confirming a completed import, Studio opens
suggestions in the next editor section. Nothing is inserted
without a selection, and Undo remains available.

Press **F2** on a resolved function, record type, or indexed record field to rename it. Studio
previews every affected file and checks the candidate with the compiler. If an
unindexed field use would break, the compiler rejects the plan before it can be
applied. Applying a valid change updates open buffers;
each file can be undone and saved separately. **Save all** writes up to 16
files together and first creates a private recovery journal under `.zelyra/`.
On restart, Studio rolls back an interrupted batch.
F2 also supports tables, views, forms, CRUDs, tableviews, and components. When a
resource name resolves uniquely in the loaded project, indexed uses in imported
files are updated together. Rename is not offered for ambiguous resource names.
The preview recompiles the full changed project.
Before renaming a `.zyl` file, Studio previews detected import files, checks
the moved project with the compiler, and applies the file and import edits
through the journal. Open changes are saved first and stale previews are
rejected. The operation is limited to 64 files and 8 MiB total. Other file
types are renamed without import edits.

Compiler diagnostics update while editing Zelyra project files. For existing
`.zyl` files the current buffer and other changed `.zyl` tabs are checked through
in-memory compiler overlays (up to 16 files and 1 MiB of source per analysis);
generated source previews for new files still use a private temporary
project copy. Neither path writes the unsaved buffer to the project. **Run** is
an explicit action for `main.zyl`, bounded to 15 seconds and
64 KiB per output stream. For the marked invoice tutorial, Run starts its
isolated Compose database, applies the schema and sample data, and then starts
the web preview. Other web projects need a reachable database. Open **Logs**
to search recent structured Run/setup events and program or preview output.
Recognized environment and local `.env` values are redacted. The program does
not receive interactive input.

The **Project map** shows unambiguous relationships from the compiler project
model: modules, tables, SQL queries, records, forms, table views, CRUD, pages,
views, APIs, and functions. It supports multiple consumers of one table and
all open unsaved `.zyl` files as overlays. Nodes lead to their actual
declaration or SQL token. The editor header shows related nodes for the
current cursor context and links to their sources. The NULL radar shows only
static nullability diagnostics from the compiler under `E-SQL-004`; it does
not inspect a live database. Dynamic relationships or queries unresolved by
the conservative SQL analyzer remain marked incomplete.

For a parseable project, the contract gutter displays results from Zelyra's
actual verifier on the current unsaved buffer. `PROVEN` (green, `V-001`) means
the obligation was proved; `RUNTIME_CHECK` (amber, `V-002`) needs a runtime
check; `FAILED` (red, `V-004`) is contradicted; `UNPROVEN` (neutral, `V-003`)
does not establish a proof. Activate a marker with click, Enter, or Space to
read its explanation. Syntax or type errors clear older markers rather than
showing a stale proof. The gutter follows the current document; background
checks use document versions to discard late responses.

At an expression hole `_`, **Ctrl+Space** opens a CodeMirror completion menu
from the compiler's hole context. Suggestions use visible bindings with known
matching types and zero-argument functions whose declared Capabilities fit the
current function. Candidates with contracts are labeled for review. Choose a
suggestion with the keyboard or click; only the `_` span is replaced, and
ordinary Undo restores it. A `_` wildcard in a match pattern is not a hole.
Type inference for unannotated local values and general subtyping are not yet
used for hole suggestions, so the list may omit valid expressions.

### Local event stream

Studio opens `GET /__zelyra/editor/api/events` as an SSE stream through
`fetch`, putting its session token in the `X-Zelyra-Editor-Token` header rather
than the stream URL. The server checks the local Host, token, and any supplied
browser Origin/Fetch metadata. Mutating APIs still require the header and the
exact editor origin. Studio reconnects with a bounded backoff; individual
analysis requests remain usable without the stream.

Each data event is a JSON object containing `schema: 1`, `kind`, `document`
(`path`, `version`, and `clientId` for browser analyses), `workspaceRevision`,
and `payload`. Kinds are `analysis.progress`, `diagnostics`, `verification`,
`workspace.changed`, `analysis.invalidated`, and `preview.changed`. Studio
accepts analysis results only for the current client and document version.
The server permits four streams, each with a queue of 32 messages, and drops
slow receivers. Studio requests and Studio file actions currently produce
events. A bounded metadata watcher also reports external changes to `.zyl`,
`.css`, and `zelyra.toml` files in the project tree. Clean open buffers reload;
dirty buffers are preserved and show a notice. The watcher does not follow
symlinks and may miss a same-size edit with an unchanged timestamp on
filesystems with coarse timestamp resolution.

### Runtime budget and recursion

Open **Budget** to configure a console run. A switch enables `--max-steps`;
the slider covers 1,000 to 1,000,000 steps and the numeric field accepts
integers from 1 to 100,000,000. With the switch off, runs retain their
existing behavior without a step budget. **Run** passes the chosen value as
separate CLI arguments to `zelyra run`. The server allows one Studio run at a
time, and the output dialog can cancel a running console program. The existing
15-second timeout and output limits still apply. A budget overrun displays
the actual `E-RUNTIME-001` output, and lines printed before the failure remain
visible. Studio also displays the project limit from `[runtime] max_steps`, or
shows that none is set. This TOML limit applies to `zelyra run`, executed API
handlers, and scheduled commands; `.env` does not override it. The static
Canvas preview does not execute project functions.

### Interactive console input

When a console program calls `read_console(prompt)`, **Run** shows its prompt
output live and reveals a labeled input field. **Send input** sends one line to
the same running process; another `read_console` call can read the next line.
**End input (EOF)** closes standard input as end-of-file. **Cancel** stops the
run. A line can contain at most 4,096 UTF-8 bytes and cannot contain line
breaks; the existing 15-second runtime limit still applies. Input stays in the
local Studio process and is not written to project files or logs. Output is
redacted against recognized project secrets before display and logging. CLI
input through `zelyra run` remains available as before.

The view reads `[runtime] max_stack_depth` from `zelyra.toml`, showing the
runtime default of 20 when absent. The permitted configuration range is 1 to
500; invalid configuration is reported. A compiler-backed call graph marks
possible direct and mutual recursion, including the current unsaved buffer.
If the project has more than 500 functions, Studio marks the recursion
analysis incomplete. The call graph cannot prove an exact maximum depth;
Studio does not invent one or a measured step count. The duration in
milliseconds comes from the actual program run.

The runtime profile also displays `[runtime].scheduler_job_timeout_seconds`.
Its default is 300 seconds; a configured value limits only scheduler jobs in
the installed, active Linux systemd service.

### Server-side PDF preview

Open **PDF preview**, choose a declared `view`, and enter example data as a
JSON object, such as `{"number":"R-104"}` for a view containing `{number}`.
Only **Render PDF** starts rendering. Studio checks the project, including the
current unsaved `.zyl` buffer, and uses the same local renderer as
`render_pdf("ViewName", record)`. It shows the actual PDF file in an isolated
browser frame and offers a download. Studio does not execute project
functions or query a database for this preview. Example data stays on the
local Studio server and is not written to project files.

The current renderer creates a searchable text PDF from visible view content;
it does not reproduce CSS, images, or charts as graphical layout. Embedded PDF
display depends on browser support; the download contains the same generated
PDF bytes. Invalid data, unknown views, and compiler errors are reported.
This is separate from the existing presentation print view, which still uses
the browser print dialog.

### Inspect the MariaDB outbox

**Outbox** reads the existing transactional MariaDB outbox when opened and
when you choose **Refresh**. It shows 25 events per page with status,
attempts, recipient, timestamps, a redacted error code, and the name of the
one currently supported PDF attachment. Message bodies and attachment bytes
are not sent to the browser. The local Studio token also protects this read
API. Without a configured MariaDB connection, the view shows a configuration
hint; `zelyra outbox setup main.zyl` explicitly creates the table.
If a configured connection cannot be reached or the table is missing, Studio
shows a warning with the same setup command. Close it when the project does not
use mail/outbox features.

Only `exhausted` events offer **Requeue**. After your confirmation, Studio
calls the existing domain requeue operation. It releases the event for a later
worker run and sends no message. Studio neither starts nor automatically
configures a worker. Count and rows are read in two database queries; a
concurrent worker can change the count between those queries.

## Start the invoice tutorial in Studio

For a generated invoice tutorial, Studio starts its own isolated MariaDB
service and applies the schema when you click **Run**. After creating it with
`--no-start`, open the project this way:

```sh
zelyra editor .
```

Select `main.zyl` and click **Run**. Studio starts the database, applies the
schema and sample data, then opens the workflow home page. **Logs** shows
structured startup and preview output. Invoice dates accept `YYYY-MM-DD` or
`JJJJ-MM-TT` according to the selected language, and the server assigns an
invoice number when saving. The local preview reads the project connection
from its protected `.env` file. If a CRUD page cannot load data, check **Logs**
for setup errors; the page does not reveal credentials.

## Edit source code

The file tree lists Zelyra source files in the project. Tabs keep several files
open. Search, completion, formatting, compiler diagnostics, undo/redo, and
version-aware saving support the edit cycle. **Rename** and **Delete** provide
explicitly confirmed source-file operations. Renaming a `.zyl` file updates
detected `import` paths after compiler validation and applies them with the
file move. `main.zyl` and `zelyra.toml` are protected. Paths stay inside the project, and
file operations compare the saved content hash. If another application changes
a file after Studio loads it, the operation is rejected; reload before retrying.

## Inspect the database schema

Choose **Database** and load the schema. Studio needs `DATABASE_URL` from the
process environment or the local project `.env` file. It reads table and
column metadata. It does not apply database changes without a separate,
confirmed action.

The overview includes a clickable relationship map with tables and foreign-key
arrows. It groups declared tables by their known source module and compares
columns, types, nullability, primary keys, automatic IDs, uniqueness, and
foreign keys with the live database. Missing tables and columns, as well as
live-only tables and columns, are marked. Source-module mapping is guidance;
it does not enforce access rights.

The **record browser** loads rows only after an explicit click and pages at no
more than 25 rows. You can create, edit, and delete records in declared tables
that match the live schema. Create and update require confirmation; delete is
identified as permanent and cannot be undone. Changes use only schema fields
and bound parameters. Edit and delete require a primary key. Unknown tables,
undeclared fields, and tables without a live connection are rejected.
Credentials stay on the local Studio server, and values are rendered as text.
Opening the workbench does not fetch records automatically. The browser does
not accept arbitrary SQL or cross-table writes.

The **chart assistant** creates a grouped bar or line chart from a declared
live table. Choose the grouping and measure from fields. Available operations
are count, sum, average, minimum, and maximum. Sum and average require a
numeric database column. Studio builds a read-only query from schema
identifiers and displays at most 20 groups. It does not accept arbitrary SQL.
You can set an exact text filter and an inclusive range on a `DATE` column.
Values are bound as parameters, and table/column selections stay limited to
declared project fields that also exist in the live database. The chart
configuration is saved as `chart-<name>.zchart.json`; queried rows are not
included. Loading a saved chart validates its shape and reruns the preview
against current database contents. An existing Zelyra chart report can be
updated; a same-named unrelated file is not overwritten.

Add several charts as slides to a local presentation with a title and
introduction. The configuration is saved as
`presentation-<name>.zelyra-presentation.json`. **Print view / PDF** creates a
slide preview and opens the browser print dialog, where the PDF can be saved
locally. Slides store reviewed chart configurations rather than a database
snapshot. Loading a presentation restores its title, introduction, and slides;
chart queries run again when the print view is created.

This Studio slice does not provide arbitrary filters, timestamp or timezone
ranges, embedding saved charts into running application pages, or compiler
type checking of these JSON files. The presentation feature is a simple slide
and print workflow, not a general chart or presentation editor.

The guided **column editor** can add table fields and edit simple existing
fields. Key, unique, and defaulted columns are excluded from visual editing.
Before applying a source change, Zelyra checks it with the project compiler
and shows a diff, diagnostics, and a reminder to review the schema plan. The
operation changes only the Zelyra source file. Review `zelyra db plan main.zyl`
and apply any database migration separately.

For a MariaDB project with an existing connection configuration, **Create
MariaDB database** can explicitly create or ensure the declared database
exists. Studio asks for confirmation first. For a locally generated Compose
project, it uses the protected `MARIADB_ROOT_PASSWORD` from `.env`; otherwise
it tries the configured database connection. Credentials stay on the server.
The action does not apply table migrations. Backend and credentials cannot yet
be configured in Studio; PostgreSQL and MySQL database creation are not
included.

In this release, visual live introspection supports MariaDB and PostgreSQL as
covered by the tested `db map` support. Studio clearly reports that MySQL
schema introspection is not yet supported. This is separate from the MySQL
runtime's typed parameterized query support.

## Generate CRUD source

For a declared table that also exists in the live database and has no CRUD
resource yet, choose an imported source file, resource name, title, and visible
fields. Columns missing from the live database cannot be selected.

Save edits to the selected file in Studio first. **Check code and preview**
builds the candidate in a private temporary project on the same machine and
runs the normal Zelyra project compiler. The preview shows generated source,
the source change, and compiler diagnostics. It does not modify the project
file.

Only **Apply change** writes the checked source. Studio checks the file hash
again immediately before writing. If the file changed after preview, the write
is rejected and a new preview is required. Generated CRUD is source code; it
does not migrate the database or change records.

## Static canvas preview

**Canvas** shows a declared page or view beside the source. Page composition
and data substitution use the same functions as web delivery. Values in
**Sample data (JSON)** are HTML escaped; arrays of objects can populate
existing `for` blocks. The current unsaved `.zyl` buffer is compiled through
overlays along with other open unsaved `.zyl` files. The preview does not run
project queries, handlers, `main()`, or JavaScript. Its frame has an empty
`sandbox` attribute and restrictive CSP.
Only explicitly supplied sample data are displayed.

Canvas is an inspection preview, not the running application. To execute the
project's web routes and handlers, choose **Start** in Studio's top bar, then
use **Open application in browser** in the run panel. The local web application
opens in its own browser tab; its server startup message is informational
output in Studio Logs.

The canvas uses the configured stylesheet choice and `zelyra.theme.css`.
Unsaved theme changes refresh the canvas without reloading Studio. The color
picker recognizes hexadecimal `--z-color-*` declarations outside CSS
comments. **Edit source** opens the theme file and makes an undoable editor
edit. For primary and status backgrounds it chooses black or white for the
paired `--z-color-on-*` text color based on contrast. Hover and soft colors,
and other related tokens, still need manual adjustment. The source button
opens the page or view declaration. A component selector lists components
reachable from it, including nested components. Separate actions open either
their declaration or an actual `<Component>` use in the source. Both jumps
target the exact source column.

## Capability and path matrix

The **Permissions** button analyzes the current `.zyl` file, including
unsaved changes. The matrix shows `uses` declarations, capability operations
found by the compiler, and grants from `zelyra.toml`. Grants with no detected
function use are suggestions only; generated routes may still require them.
Literal file paths use
the same canonical root check as the runtime. Dynamic paths remain unknown;
paths allowed at inspection time are checked again during execution. A
compiler error marks the report as incomplete.

**Review grant** prepares a missing grant and displays the complete proposed
`zelyra.toml`. Only **Apply grant** saves it. Studio rechecks the file hash,
preserves TOML comments, and rejects stale configuration. Without
`zelyra.toml`, there is no project-wide runtime capability restriction, so
Studio does not pretend to add a grant. The report covers analyzed function
calls; dynamic calls and generated web routes are not fully resolved into a
static permission chain.

Studio scans `.zyl`, `.css`, and `zelyra.toml` files in the allowed project
tree every 1.5 seconds for external changes. The watcher reads metadata only
and does not follow symlinks. Clean open buffers reload; unsaved edits stay
in place with a warning. It obeys the editor's existing tree-depth and file
count limits. On file systems with coarse timestamps, a same-length edit with
an unchanged timestamp may be missed.

## Scope

Studio is a local, language-specific IDE with the editor, compiler-backed
language tools, LSP, project map, preview, runtime controls, and database
workbench described above. Some analysis is intentionally conservative:
unannotated local type inference is limited, dynamic call chains are not fully
resolved for capability reports, and rename is offered only for statically
resolved supported symbols. The database workbench provides selected schema
and data actions, not unrestricted database design. CRUD generation works from
existing declared tables and validates generated code before writing it.
