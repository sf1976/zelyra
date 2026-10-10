# Zelyra Studio: local web editor

**Target version:** Zelyra 0.7.0 · **Status:** experimental development

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

The CLI prints a local URL containing a random session token. Open it in a
browser and stop the editor process with `Ctrl+C` when finished.

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

The 0.7 editor is the coding and checking foundation. The planned 0.8 step adds
a read-only database browser and relationship map, then a guided CRUD builder:
select a table, choose visible fields and views, inspect the generated Zelyra
source diff, run the compiler, and apply the change explicitly. Schema writes
continue through a reviewed migration plan. Version 0.9 is planned to guide
users through invoice and other small business workflows and help them add
business rules. See the [invoice tutorial](tutorials/invoice.en.md).
