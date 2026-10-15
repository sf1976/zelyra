# Zelyra Language Server (LSP)

`zelyra lsp` speaks the Language Server Protocol (LSP) over standard input and
output. It does not require a separate server installation. Open the project
folder as a workspace so Zelyra can resolve imports and project configuration.
Opening a source file does not run the program.

Supported features include initialization and shutdown, versioned document
changes, diagnostics, completion, hover, definition, references, rename,
document symbols, and workspace symbols. Positions use LSP UTF-16. The server
analyzes open source buffers without writing their contents to disk.

## VS Code and Cursor

Install `editors/zelyra.vsix` with **Extensions: Install from VSIX...**. The
extension includes the LSP client. Install the Zelyra CLI and make sure
`zelyra` is available on `PATH`. To use a different executable path, set:

```json
{
  "zelyra.languageServer.path": "/opt/zelyra/bin/zelyra"
}
```

The extension starts `zelyra lsp` in the opened project folder. View its logs
under **View: Output**, channel **Zelyra Language Server**.

## Neovim

For Neovim 0.11 or newer:

```lua
vim.filetype.add({ extension = { zyl = "zelyra" } })

vim.lsp.config("zelyra", {
  cmd = { "zelyra", "lsp" },
  filetypes = { "zelyra" },
  root_markers = { "zelyra.toml", ".git" },
})
vim.lsp.enable("zelyra")
```

For older Neovim versions with `nvim-lspconfig`, register the local server:

```lua
local lspconfig = require("lspconfig")

if not lspconfig.configs.zelyra then
  lspconfig.configs.zelyra = {
    default_config = {
      cmd = { "zelyra", "lsp" },
      filetypes = { "zelyra" },
      root_dir = lspconfig.util.root_pattern("zelyra.toml", ".git"),
    },
  }
end

lspconfig.zelyra.setup({})
```

The CLI must be available on Neovim's `PATH` in either configuration.

## Protocol limits

The server uses JSON-RPC `Content-Length` framing and writes protocol messages
only to stdout. Diagnostics carry document versions; stale edits are not
treated as the current document state. General local bindings are not yet fully
resolved semantically. Requests are currently processed synchronously, so an
LSP request cannot be cancelled while analysis is running. Workspace symbols
and semantic refactorings are limited to symbol kinds that the compiler index
can resolve safely.
