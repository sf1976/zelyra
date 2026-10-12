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
saved function or type definition in the current or a directly imported file.
Shift+F12 finds exact lexical identifier occurrences in saved `.zyl` files.
These occurrence results are not scope-aware semantic references. Save changed
files before navigating so source locations stay accurate.

Compiler diagnostics update while editing saved Zelyra project files; they are
checked against a private temporary project copy and do not write the unsaved
buffer. **Run** is an explicit action for `main.zyl`, bounded to 15 seconds and
64 KiB per output stream. For web projects it starts a local web preview; Studio
does not start the database automatically. Recognized environment and local
`.env` values are redacted. The program does not receive interactive input.

The **Project map** shows only unambiguous relationships from the compiler
project model and links nodes to saved source locations. Dynamic and
unresolved relationships remain marked incomplete instead of being inferred.

## Start the invoice tutorial in Studio

The tutorial needs a running MariaDB and an applied schema before customer and
item lists can load data. After creating it with `--no-start`, run this from the
project directory:

```sh
zelyra setup --all
zelyra editor .
```

`setup --all` starts MariaDB and the web application, applies the schema, and
shows startup progress and the application address. In Studio, run `main.zyl`
and open `/customers`. The local preview reads the project connection from its
protected `.env` file. If a CRUD page cannot load data, check that MariaDB is
running and the project schema has been applied; the page does not reveal
connection errors or credentials.

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
column metadata only. It does not read records or apply schema changes.

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
