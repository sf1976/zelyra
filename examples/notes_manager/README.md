# Zelyra Notizverwaltung

Ein mehrteiliges Konsolenbeispiel für sichere UTF-8-Notizen. Jede Notiz ist
eine JSON-formatierte Textdatei mit Titel, Inhalt, Erstellungszeit und letzter
Änderungszeit. Die Texte bleiben lesbar und Zeilenumbrüche werden durch JSON
verlustfrei gespeichert.

## Start

Im Verzeichnis `examples/notes_manager` ausführen:

```sh
zelyra check main.zyl
zelyra run main.zyl
```

Das Menü bietet Anlegen, Lesen, Anhängen, wörtliches Ersetzen, Entfernen eines
Absatzes, Umbenennen, Kopieren, bestätigtes Löschen, Suche und Wiederherstellen.
Wähle `12`, um die automatisierten Beispieltests laufen zu lassen.

## Dateisicherheit

- Notizen liegen in `data/notes`; Sicherungen in `data/notes/backups`.
- Der Projektzugriff ist in `zelyra.toml` auf `data` begrenzt.
- Neue Notizen und Kopien werden exklusiv angelegt und überschreiben keine
  vorhandene Datei.
- Jede Inhaltsänderung und das Löschen legen zuerst eine Sicherung an.
- Inhaltsänderungen schreiben in eine temporäre Datei im selben Ordner und
  ersetzen die Notiz erst nach erfolgreichem Schreiben.
- Der getrennte `actions.jsonl`-Protokollstrom enthält jede im Menü ausgeführte
  Dateiaktion als UTF-8-JSON-Zeile.

Das Beispiel nutzt Zelyras eingebauten JSON- und Datei-APIs; es benötigt keine
Cloud-Dienste oder externen Pakete.
