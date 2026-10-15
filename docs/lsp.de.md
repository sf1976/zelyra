# Zelyra Language Server (LSP)

`zelyra lsp` spricht Language Server Protocol (LSP) über Standard-Ein- und
Ausgabe. Es benötigt keine zusätzliche Serverinstallation. Öffne das
Projektverzeichnis als Workspace, damit Zelyra Imports und Projektkonfiguration
auflösen kann. Das Öffnen einer Quelldatei führt das Programm nicht aus.

Unterstützt sind Initialisierung und Shutdown, versionierte Dokumentänderungen,
Diagnosen, Completion, Hover, Definition, Referenzen, Rename, Dokumentensymbole
und Workspace-Symbole. Die Positionen folgen LSP UTF-16. Der Server prüft
geöffnete Quelldateien aus dem Editorpuffer; Änderungen werden nicht auf die
Festplatte geschrieben.

## VS Code und Cursor

Installiere `editors/zelyra.vsix` über **Extensions: Install from VSIX...**.
Die Erweiterung enthält den LSP-Client. Installiere die Zelyra-CLI und stelle
sicher, dass `zelyra` im `PATH` liegt. Für einen abweichenden Pfad setze:

```json
{
  "zelyra.languageServer.path": "/opt/zelyra/bin/zelyra"
}
```

Die Erweiterung startet `zelyra lsp` im geöffneten Projektverzeichnis. Logs
findest du unter **View: Output** im Kanal **Zelyra Language Server**.

## Neovim

Für Neovim 0.11 oder neuer:

```lua
vim.filetype.add({ extension = { zyl = "zelyra" } })

vim.lsp.config("zelyra", {
  cmd = { "zelyra", "lsp" },
  filetypes = { "zelyra" },
  root_markers = { "zelyra.toml", ".git" },
})
vim.lsp.enable("zelyra")
```

Für ältere Neovim-Versionen mit `nvim-lspconfig` registriere den lokalen
Server so:

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

In jedem Fall muss die CLI über Neovims `PATH` erreichbar sein.

## Protokollgrenzen

Der Server verwendet JSON-RPC `Content-Length`-Framing und schreibt
Protokollnachrichten ausschließlich nach stdout. Diagnosen tragen
Dokumentversionen; veraltete Änderungen werden nicht als aktueller Dokumentstand
behandelt. Allgemeine lokale Bindings sind noch nicht vollständig semantisch
aufgelöst. Anfragen werden derzeit synchron verarbeitet; LSP-Abbruch während
einer laufenden Analyse ist daher noch nicht verfügbar. Workspace-Symbole und
semantische Refactorings sind auf die vom Compilerindex sicher auflösbaren
Symbolarten begrenzt.
