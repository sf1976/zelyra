# Zelyra Studio: lokaler Webeditor

**Zielversion:** Zelyra 0.7.0 · **Status:** experimenteller Ausbau

Zelyra Studio ist die integrierte Browseroberfläche zum Bearbeiten eines
Zelyra-Projekts. Sie läuft als Bestandteil der CLI und bündelt alle benötigten
Editorressourcen lokal. Der Editor braucht weder ein Konto noch einen externen
Dienst oder ein CDN.

## Start

Im Projektverzeichnis:

```sh
zelyra editor
```

Oder mit einem Projektpfad und einem abweichenden Port:

```sh
zelyra editor ./rechnung --port 4177
```

Die CLI zeigt eine lokale URL mit einem zufälligen Sitzungstoken an. Öffne sie
im Browser und beende den Editorprozess mit `Ctrl+C`, wenn du fertig bist.

## Arbeitsoberfläche

- Projektdateibaum mit Filter, Dateiwechsel, mehreren Tabs und neuen Dateien.
- CodeMirror-Editor mit Zeilennummern, Faltung, Klammerzuordnung, Auswahl,
  Verlauf, Suche/Ersetzen und Vervollständigung für Zelyra-Schlüsselwörter.
- Syntaxfarben, helles und dunkles Farbschema, Tastenkürzel für Speichern,
  Suchen und Dateiöffnung.
- Speichern mit Hashvergleich: Wenn eine Datei seit dem Öffnen von einem
  anderen Programm geändert wurde, lehnt Studio das Überschreiben ab.
- Projektprüfung mit Compilerdiagnosen und Sprung zur betroffenen Datei.
- Explizite Formatierung einer geöffneten `.zyl`-Datei mit dem Zelyra-Formatter.

Unterstützt werden Textdateien mit den Erweiterungen `.zyl`, `.toml`, `.css`,
`.json`, `.md`, `.html`, `.sql`, `.txt`, `.yaml` und `.yml`. Einzelne Dateien
sind auf 1 MiB begrenzt. Versteckte Pfade, Secrets, generierte Verzeichnisse
und symbolische Links werden nicht angezeigt oder bearbeitet.

## Sicherheitsgrenzen

Der Server bindet ausschließlich an `127.0.0.1`; es gibt keine Option, ihn im
Netzwerk freizugeben. Jede API-Anfrage braucht das zufällige Sitzungstoken.
Schreibende Anfragen verlangen zusätzlich denselben Browser-Origin. Es gibt
keine CORS-Freigabe. Antwortheader verhindern Caching, MIME-Sniffing und das
Einbetten des Editors in fremde Frames.

Der Dateizugriff bleibt auf den Projektordner begrenzt. Studio führt keine
Shell-Befehle aus und öffnet keine Projektvorschau. Es bietet in 0.7.0 noch
keinen Language Server, Debugger, Versionskontroll-Client oder Multi-User-
Zusammenarbeit. Projektprüfung und Formatierung verwenden den lokalen
Zelyra-Compiler.

Die CodeMirror-Komponenten stehen unter MIT-Lizenz. Build-Abhängigkeiten und
Versionen sind in `editor/package-lock.json` festgehalten; die Browserdateien
werden vor dem Release lokal gebündelt.
