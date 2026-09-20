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

Für CRUD-Ressourcen enthält die Übersicht das ausgewählte äußere Layout sowie
Namen und Source-Spans der gelieferten benannten Layout-Slots. Die
HTML-Inhalte dieser Slots werden bewusst ausgelassen; der Projektkontext bleibt
begrenzt und gibt beliebige Darstellungsinhalte nicht standardmäßig aus.

## Einfacher Standard und optionale Möglichkeiten

Zelyra verwendet ein zweistufiges Konfigurationsmodell. `zelyra.toml`
enthält dauerhafte, nicht geheime Projektentscheidungen; `.env` und die
Prozessumgebung enthalten umgebungsabhängige Werte und dürfen nur ausdrücklich
unterstützte, nicht geheime Feature-Schalter überschreiben. Bestehende
Projekte benötigen keinen `[features]`-Abschnitt. Die Priorität lautet:
Prozessumgebung, `.env`, `zelyra.toml`, dann sichere Standardwerte.

Die ersten unterstützten Schalter sind `web`, `api`, `crud`, `auth` und
`audit`. `zelyra config <file.zyl> --format=json` ist eine schreibgeschützte,
versionierte Inspektionsschnittstelle und gibt keine Secret-Werte aus. Wenn
eine Quelldeklaration einen deaktivierten Bereich verwendet, schlägt die
Kompilierung mit einer stabilen Diagnose fehl. Capabilities, Typprüfung,
SQL-Prüfung, Contracts und Sicherheitsregeln können damit nicht deaktiviert
werden. Künftige optionale Funktionen können dasselbe Modell verwenden; eine
Funktion gilt aber nicht als implementiert, nur weil sie in einer
Konfigurationsdatei erwähnt wird. Siehe die gepflegte
[Umgebungsreferenz](../env.md).

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
Views, APIs, Berechtigungen, Contracts sowie eine strukturierte
`references`-Kantenliste. Jede bekannte Kante enthält Quelle, Ziel, Art und
Quelltextspanne. E-Mails, Jobs, Tests, Live-Schemaänderungen und tiefere
Laufzeitabhängigkeiten bleiben geplant.
`zelyra edit --format=json change.json` liefert für Symbol-Umbenennungen in
Funktionen, Typen, Records, Tabellen, Tableviews, Formularen, CRUDs, Views und
Komponenten eine versionierte, validierte, atomare Vorschau sowie einen
deterministischen Quelltext-Fingerprint. Die Anfrage muss
`"schema_version": "1"` enthalten; der Einstieg muss eine existierende
`.zyl`-Datei innerhalb der aufgelösten Zelyra-Projektwurzel sein. Quelltext vor
und nach der Änderung wird vollständig mit den Compilerprüfungen validiert.
Die Anfrage muss diesen Fingerprint bei ausdrücklichem `--apply` zurücksenden;
so wird das validierte Ergebnis atomar geschrieben, ohne zwischenzeitliche
Änderungen zu überschreiben. Umbenennungen von Funktionen, Typen und Records
verwenden den AST und typisierte Syntaxkontexte, um Deklarationen und bekannte
Referenzen zu ändern, ohne überschattete lokale Bindungen zu verändern.
Tabellenumbenennungen aktualisieren außerdem Tabellenpositionen in geprüften
SQL-Abfragen (`FROM`, `JOIN`, `INTO` und `UPDATE`), während SQL-Literale,
Kommentare, Parameter und HTML unverändert bleiben. Weitere Operationen folgen
später.

Die optionale Impact-Abfrage `--symbol <kind:name>` begrenzt die
Maschinenausgabe auf einen bekannten Knoten und seine direkt verbundenen
Referenzen. Unbekannte Knoten liefern die stabile Diagnose `E-IMPACT-001`; die
Abfrage führt weder Datenbank- noch Netzwerkzugriffe aus.

Komponenten-Umbenennungen aktualisieren die Deklaration sowie bekannte öffnende
und schließende Komponententags in opaken HTML-Bodies. Gewöhnliche HTML-
Elemente, Text, Attribute, SQL und nicht erkannte Markup-Strukturen werden
nicht als Symbolreferenzen behandelt.

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
