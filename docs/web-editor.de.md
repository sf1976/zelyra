# Zelyra Studio: lokaler Webeditor

**Version:** Zelyra 1.0.5 · **Status:** lokales Studio mit Datenbankwerkzeugen,
geführter CRUD-Vorschau, Tutorial-Start und strukturierten Logs

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

Die CLI öffnet automatisch den Standardbrowser. Falls das auf dem System nicht
möglich ist, zeigt sie die lokale URL mit dem Sitzungstoken zum manuellen Öffnen
an. Mit `--no-open` überspringst du den Browserstart, etwa per SSH oder ohne
grafische Oberfläche. Mit **Starten** führt Studio Konsolenprogramme aus oder
startet eine lokale Webanwendung und zeigt ihre Browseradresse mit einem
Öffnen-Link an. Beim markierten Rechnungstutorial startet **Starten** die
isolierte Compose-Datenbank, richtet Schema und Beispieldaten ein und öffnet
danach die Vorschau; andere Projekte brauchen eine bereits erreichbare
Datenbank. Ein erneuter Webstart ersetzt die vorherige Vorschau.
**Vorschau beenden** stoppt sie sofort; beim Beenden von Studio wird die aktive
Vorschau ebenfalls beendet. Beende Studio mit `Ctrl+C`.

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
- Ein **Logs**-Fenster für lokale strukturierte Run- und Vorschauausgaben mit
  Stufenfilter und Textsuche. Siehe [Studio-Logs](studio-logs.de.md).

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
beliebigen Shell-Befehle aus; die ausdrückliche Run-Aktion startet begrenzt ein
Zelyra-Programm oder eine Webvorschau. Version 1.0.5 bietet noch keinen
Language Server, Debugger, Versionskontroll-Client oder Multi-User-
Zusammenarbeit. Projektprüfung und Formatierung verwenden den lokalen
Zelyra-Compiler.

Die CodeMirror-Komponenten stehen unter MIT-Lizenz. Build-Abhängigkeiten und
Versionen sind in `editor/package-lock.json` festgehalten; die Browserdateien
werden vor dem Release lokal gebündelt.

## Geplanter Weg zum visuellen Datenbankassistenten

Seit 0.8 zeigt Studio eine schreibgeschützte Datenbankkarte für MariaDB und
PostgreSQL, erkennt Beziehungen und kann einen CRUD-Entwurf mit Compilerprüfung,
Quelltextvorschau und ausdrücklicher Übernahme erzeugen. Die Anwendung legt
keine Tabellen stillschweigend an. Schemaänderungen laufen weiterhin über einen
geprüften Migrationsplan. Die 0.9-Vorschau zeigt zusätzlich die betroffene
Quelldatei, Tabelle, deklarierten Beziehungen und erzeugten Routen, bevor du
CRUD-Quelltext übernimmst. Für wiederkehrende Regex-Änderungen gibt es den
Vorschauplan `zelyra text edit`; Studio und CLI schreiben ohne ausdrückliches
Anwenden nichts. Siehe das [Rechnungstutorial](tutorials/invoice.de.md).
