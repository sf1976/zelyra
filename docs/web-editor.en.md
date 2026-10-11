# Zelyra Studio: local web editor

**Target version:** Zelyra 0.8.0 · **Status:** local Studio, database map, and
guided CRUD preview released as stable; continued development is planned to 1.0

Zelyra Studio is the integrated browser interface for editing a Zelyra
project. It runs as part of the CLI and bundles its editor assets locally. It
needs no account, external service, or CDN.

## Start

From a project directory:

```sh
zelyra editor
```

Or choose a project path and port:

```sh
zelyra editor ./invoices --port 4177
```

The CLI opens the default browser automatically. If that is unavailable, it
prints the local URL containing the session token for manual use. Pass
`--no-open` to skip browser launch, for example over SSH or without a graphical
desktop. Studio does not start a database. **Run** executes console programs or
starts a local web application and shows its browser address with an open link.
The application's database must already be reachable. Starting a web app again
replaces the previous preview. **Stop preview** ends it immediately; closing
Studio also stops the active preview. Stop Studio with `Ctrl+C`.

## Workspace

- Project file tree with filtering, file switching, multiple tabs, and new
  files.
- CodeMirror editor with line numbers, folding, bracket matching, selection,
  history, search/replace, and Zelyra keyword completion.
- Syntax colors, light and dark themes, and shortcuts for saving, searching,
  and opening a file.
- Hash-checked saves: Studio refuses to overwrite a file changed by another
  program since it was opened.
- Project checking with compiler diagnostics and navigation to the affected
  file.
- Explicit formatting of an open `.zyl` file with the Zelyra formatter.

Editable text extensions are `.zyl`, `.toml`, `.css`, `.json`, `.md`, `.html`,
`.sql`, `.txt`, `.yaml`, and `.yml`. Files are limited to 1 MiB. Hidden paths,
secrets, generated directories, and symbolic links are not listed or editable.

## Security boundaries

The server binds only to `127.0.0.1`; it has no network-listening option. Every
API request needs the random session token. Write requests also require the
same browser origin. There is no CORS grant. Response headers prevent caching,
MIME sniffing, and embedding the editor in another site's frame.

File access stays inside the project directory. Studio does not run shell
commands or launch a project preview. Version 0.7.0 does not include a language
server, debugger, source-control client, or multi-user collaboration. Project
checking and formatting use the local Zelyra compiler.

CodeMirror components are MIT-licensed. Build dependencies and versions are
recorded in `editor/package-lock.json`; browser assets are bundled locally
before release.

## Planned path to the visual database builder

Since 0.8, Studio provides a read-only database map for MariaDB and PostgreSQL,
identifies relationships, and can produce a CRUD draft with compiler checking,
source preview, and explicit apply. It does not silently create tables. Schema
changes continue through a reviewed migration plan. The 0.9 preview also lists
the affected source file, table, declared relationships, and generated routes
before CRUD source is applied. `zelyra text edit` provides preview-bound plans
for repeatable regex changes; neither workflow writes until explicitly applied.
See the [invoice tutorial](tutorials/invoice.en.md).
