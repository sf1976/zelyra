# Zelyra Studio

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
resolved function definition in the current or an imported file. Shift+F12
shows references to that function and its declaration. These two actions use
the compiler's linked AST and accept an unsaved current buffer. Other symbol
kinds are not yet supported by this semantic index. If another tab has unsaved
changes, save it first so cross-file positions stay accurate. Project-wide
free-text search remains a separate tool.

Compiler diagnostics update while editing Zelyra project files. For existing
`.zyl` files the unsaved buffer is checked through an in-memory compiler
overlay; generated source previews for new files still use a private temporary
project copy. Neither path writes the unsaved buffer to the project. **Run** is
an explicit action for `main.zyl`, bounded to 15 seconds and
64 KiB per output stream. For the marked invoice tutorial, Run starts its
isolated Compose database, applies the schema and sample data, and then starts
the web preview. Other web projects need a reachable database. Open **Logs**
to search recent structured Run/setup events and program or preview output.
Recognized environment and local `.env` values are redacted. The program does
not receive interactive input.

The **Project map** shows only unambiguous relationships from the compiler
project model and links nodes to saved source locations. Dynamic and
unresolved relationships remain marked incomplete instead of being inferred.

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
events; there is no active watcher for filesystem changes made by other tools.

### Runtime budget and recursion

Open **Budget** to configure a console run. A switch enables `--max-steps`;
the slider covers 1,000 to 1,000,000 steps and the numeric field accepts
integers from 1 to 100,000,000. With the switch off, runs retain their
existing behavior without a step budget. **Run** passes the chosen value as
separate CLI arguments to `zelyra run`. The server allows one Studio run at a
time, and the output dialog can cancel a running console program. The existing
15-second timeout and output limits still apply. A budget overrun displays
the actual `E-RUNTIME-001` output. Web previews do not use `--max-steps`.

The view reads `[runtime] max_stack_depth` from `zelyra.toml`, showing the
runtime default of 20 when absent. The permitted configuration range is 1 to
500; invalid configuration is reported. A compiler-backed call graph marks
possible direct and mutual recursion, including the current unsaved buffer.
If the project has more than 500 functions, Studio marks the recursion
analysis incomplete. The call graph cannot prove an exact maximum depth;
Studio does not invent one or a measured step count. The duration in
milliseconds comes from the actual program run.

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
explicitly confirmed source-file operations; rename does not update imports.
`main.zyl` and `zelyra.toml` are protected. Paths stay inside the project, and
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
more than 25 rows. It is read-only; table and column names come from the loaded
schema, SQL identifiers are quoted for the selected backend, and values are
rendered as text. Opening the workbench does not fetch records automatically.

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

## Scope

Studio is a local IDE foundation, not yet a complete IDE. The database
workbench supports selected schema and database actions, not unrestricted
database design. CRUD generation works from existing declared tables and
validates generated code before writing it.
