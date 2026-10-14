# Local Zelyra logs in Studio

Studio records run/setup events and program/web-preview output as structured
JSONL entries.
Open **Logs** in the header to filter by level or search text. The IDE shows up
to the latest 500 entries for the open project.

Each record contains `timestamp_ms` (Unix time in milliseconds), `level`
(`info` or `error`), `source` (for example, `program.stdout` or
`preview.stderr`), and `message`. Credentials from the process environment and
`.env` are redacted before the record is written. Log files stay on the local
computer:

- Linux and other Unix systems: `$XDG_STATE_HOME/zelyra/logs/`; if
  `XDG_STATE_HOME` is unset, `~/.local/state/zelyra/logs/`.
- macOS: `~/Library/Logs/Zelyra/`.
- Windows: `%LOCALAPPDATA%\Zelyra\Logs\`.

The filename contains a short hash of the project path, not the path itself.
On Unix, files are owner-readable only (`0600`) and the log directory is
owner-only (`0700`). The active file rotates after 5 MiB, keeping one previous
file. `zelyra uninstall` does not remove logs.

Logs cover Studio run/setup events and stdout/stderr from **Run** and the local
preview. Other Studio actions and CLI commands outside Studio are not recorded.
No telemetry or server upload is used.
