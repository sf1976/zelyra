# Zelyra language support for VS Code and Cursor

The extension provides syntax highlighting and starts Zelyra's standalone LSP
server for diagnostics, completion, hover, go to definition, references,
rename, and document/workspace symbols. Cursor uses the same VS Code extension
format and settings.

## Requirements

- Install the Zelyra CLI and make `zelyra` available on `PATH`, or set
  `zelyra.languageServer.path` to the CLI executable.
- Install the packaged `zelyra.vsix` from **Extensions: Install
  from VSIX...**. Users do not need Node.js, npm, or a separate language-server
  package; the VSIX contains the language client and the server is the CLI.

For a non-default executable path, add this to VS Code or Cursor settings:

```json
{
  "zelyra.languageServer.path": "/opt/zelyra/bin/zelyra"
}
```

The extension launches `zelyra lsp` as a child process over standard input and
output. Open a folder as the workspace so the server can resolve project files
and imports. Server logs are available in **View: Output**, channel
**Zelyra Language Server**. The extension does not run the project.

## Neovim

With `nvim-lspconfig`, configure the standalone server as follows:

```lua
vim.filetype.add({ extension = { zyl = "zelyra" } })

vim.lsp.config("zelyra", {
  cmd = { "zelyra", "lsp" },
  filetypes = { "zelyra" },
  root_markers = { "zelyra.toml", ".git" },
})
vim.lsp.enable("zelyra")
```

For older Neovim releases that do not provide `vim.lsp.config`, use the
equivalent `require("lspconfig").zelyra.setup({ ... })` configuration supported
by the installed `nvim-lspconfig` version. Ensure the CLI is on Neovim's PATH.

## Building the VSIX (maintainers)

The checked-in VSIX is self-contained. To rebuild it, use Node.js/npm only as a
development tool:

```sh
npm ci --prefix editors
npm run package --prefix editors
```

This creates `editors/zelyra.vsix`. The extension's runtime
dependencies are packaged inside the VSIX. The Zelyra CLI remains a single
binary and its Studio assets remain embedded; neither requires npm at runtime.
