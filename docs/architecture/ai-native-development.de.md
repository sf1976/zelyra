# Architektur: KI-native Entwicklung

Status: grundlegende Architekturentscheidung für Zelyra 0.1

## Ziel und Grenze

Zelyra soll besonders gut für KI-gestützte Entwicklung datenbankgestützter
Business- und Webanwendungen geeignet sein. Das ist ein strategisches Ziel und
noch keine öffentliche Benchmark-Tatsache. Der Produktvertrag lautet:

> Die KI schreibt. Zelyra prüft.

Menschen und KI-Systeme dürfen denselben Quellcode schreiben. Der Compiler
bleibt die Vertrauensgrenze. Zelyra muss ohne KI-Anbieter, Cloud-Konto,
Netzwerkzugriff oder Modellaufruf vollständig nutzbar bleiben.

## Autoren und Vertrauensgrenze

Für menschlichen und KI-generierten Code gelten dieselben Lexer-, Parser-,
Namensauflösungs-, Typ-, SQL-, Capability-, Contract-, Test- und Laufzeitregeln.
Eine Erklärung eines Modells ist kein Korrektheitsnachweis. Destruktive SQL-
oder Schemaänderungen, neue Capabilities, Berechtigungserweiterungen,
deaktivierte Prüfungen und geschwächte Tests benötigen sichtbare menschliche
Freigaben.

## Versionierte Maschinen-Schnittstellen

Maschinenformate verwenden ein gemeinsames Grundformat:

```json
{
  "schema_version": "1",
  "command": "check",
  "success": false,
  "diagnostics": []
}
```

`schema_version` ist verpflichtend. Neue optionale Felder sind innerhalb einer
Version erlaubt; inkompatible Änderungen benötigen eine neue Version.
Unbekannte optionale Felder müssen ignoriert werden. JSON geht ausschließlich
auf stdout, technische Meldungen auf stderr. Menschliche Ausgabe bleibt der
Standard.

Offsets sind nullbasierte UTF-8-Byte-Offsets. Zeilen und Spalten beginnen bei
eins und folgen der aktuellen Lexer-Konvention, also ebenfalls Byte-Spalten.
Spans sind halb-offen: Start inklusive, Ende exklusive. Diagnosen und
Deklarationen werden deterministisch ausgegeben; Zeitstempel, Zufalls-IDs,
maschinenabhängige absolute Pfade, Secrets und Datenbankinhalte gehören nicht
in Compilerergebnisse.

## Projektkontext und geplante Erweiterungen

`zelyra context --format=json` ist eine schreibgeschützte Übersicht der vom
Compiler verstandenen Deklarationen. Der Befehl verbindet sich nicht mit einer
Datenbank, nutzt kein Netzwerk, führt keine E-Mail oder Jobs aus und gibt keine
Secrets aus. Nicht unterstützte Details werden nicht erfunden.

## Kanonische Quellformatierung

`zelyra fmt <file.zyl>` erzeugt nach erfolgreichem Lexen und Parsen eine
deterministische Quellformatierung. `zelyra fmt <file.zyl> --check` schreibt
keine Dateien und liefert einen Fehlercode, wenn eine Änderung nötig wäre;
damit kann CI kanonischen Quellcode erzwingen. Der Formatter bewahrt
Zeilenkommentare und behandelt SQL- und HTML-Blöcke als opaken Quelltext. Er
ist idempotent: Ein bereits formatiertes Dokument erzeugt byte-identisch
dieselbe Ausgabe.

Ausdrucks-Typed-Holes mit `_` sind als erste sichere Stufe verfügbar. Der
Compiler meldet Kontexttyp, sichtbare Werte und Funktionen, aktive
Capabilities, Contract-Pflichten und Source-Span. Baubare Befehle lehnen
unvollständigen Code vor Lowering und Ausführung ab. Typed Holes in
Deklarationskontexte bleiben geplante Schnittstellen.

`zelyra impact --format=json` ist als deterministische, quelltextbasierte
erste Stufe verfügbar und meldet betroffene Tabellen, SQL, Formulare, CRUD,
Views, APIs, Berechtigungen und Contracts. E-Mails, Jobs, Tests,
Live-Schemaänderungen und tiefere Laufzeitabhängigkeiten bleiben geplant.
`zelyra edit --format=json change.json` liefert für Symbol-Umbenennungen in
Funktionen, Typen, Records, Tabellen, Tableviews, Formularen, CRUDs, Views und
Komponenten eine versionierte, validierte, atomare Vorschau sowie einen
deterministischen Quelltext-Fingerprint. Die Anfrage muss
`"schema_version": "1"` enthalten; der Einstieg muss eine existierende
`.zyl`-Datei innerhalb der aufgelösten Zelyra-Projektwurzel sein. Quelltext vor
und nach der Änderung wird vollständig mit den Compilerprüfungen validiert.
Die Anfrage muss diesen Fingerprint bei ausdrücklichem `--apply` zurücksenden;
so wird das validierte Ergebnis atomar geschrieben, ohne zwischenzeitliche
Änderungen zu überschreiben. Funktionsumbenennungen verwenden den AST und
ändern Deklarationen sowie Aufrufstellen, ohne überschattete lokale Bindungen
zu verändern; die übrigen Ressourcenarten verwenden weiterhin die bestehende
Token-Abdeckung, bis ihr Referenzmodell erweitert ist. Weitere Operationen
folgen später.

## Sicherheit, Datenschutz und Benchmarks

Zelyra sendet keinen Quellcode an externe KI-Dienste. Künftige KI-Integrationen
müssen offen, lokal nutzbar, herstellerneutral und versioniert sein.
Maschinenformate dürfen keine Datenbankpasswörter, Tokens, SMTP-Zugangsdaten,
privaten Schlüssel, Session-Secrets oder `.env`-Inhalte enthalten.

Der geplante Zelyra-AI-Benchmark misst reproduzierbar unter anderem
Erstversuchskompilierung, Korrekturschleifen, Zeit bis zu bestandenen Tests,
Tokens, Codeumfang, Sicherheitsfehler, übersehene Abhängigkeiten, fehlerhafte
Datenbankänderungen und menschlichen Prüfaufwand. Vergleichende Behauptungen
werden erst nach kontrollierten Ergebnissen veröffentlicht.
