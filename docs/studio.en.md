# Zelyra Studio

Zelyra Studio is the local web editor included with the CLI. Start it from a
project directory:

```sh
zelyra editor
```

The editor listens only on `127.0.0.1`. Its startup address contains a random
session token. Mutating requests require that token and the same browser
origin.

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

Studio is a local development tool, not yet a complete IDE. Its database view
is a read-only schema overview, not an unrestricted database designer. CRUD
generation works from existing declared tables and validates generated code
before writing it.
