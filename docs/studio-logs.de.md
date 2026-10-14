# Lokale Zelyra-Logs in Studio

Studio zeichnet Start-/Setup-Ereignisse sowie Programm- und
Webvorschauausgaben als strukturierte JSONL-Einträge auf. Öffne **Logs** in der Kopfzeile, um die
Einträge nach Stufe zu filtern oder Text zu suchen. Es werden bis zu 500
aktuelle Einträge der geöffneten Projektdatei angezeigt.

Jeder Datensatz enthält `timestamp_ms` (Unix-Zeit in Millisekunden), `level`
(`info` oder `error`), `source` (z. B. `program.stdout` oder `preview.stderr`)
und `message`. Zugangsdaten aus Prozessumgebung und `.env` werden vor dem
Schreiben redigiert. Logdateien liegen nur auf dem lokalen Rechner:

- Linux und andere Unix-Systeme: `$XDG_STATE_HOME/zelyra/logs/`; ohne gesetztes
  `XDG_STATE_HOME` unter `~/.local/state/zelyra/logs/`.
- macOS: `~/Library/Logs/Zelyra/`.
- Windows: `%LOCALAPPDATA%\Zelyra\Logs\`.

Der Dateiname enthält einen kurzen Hash des Projektpfads, nicht den Pfad
selbst. Dateien sind unter Unix nur für den Eigentümer lesbar (`0600`), das
Protokollverzeichnis ist auf `0700` gesetzt. Die aktive Datei wird bei mehr als
5 MiB rotiert; eine vorherige Datei bleibt erhalten. Logs werden bei
`zelyra uninstall` nicht gelöscht.

Die Protokolle enthalten Studio-Start-/Setup-Ereignisse sowie stdout/stderr
von **Starten** und lokaler Vorschau. Andere Studio-Aktionen und die
Kommandozeile außerhalb von Studio werden nicht aufgezeichnet. Es gibt keine
Telemetrie oder Übertragung an einen Server.
