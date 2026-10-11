# CSV-Katalogimport

Dieses mehrdateilige Konsolenbeispiel erstellt eine UTF-8-CSV-Datei mit
Semikolon, liest sie mit einem typisierten CSV-Parser ein, wandelt Felder um,
validiert alle Zeilen und exportiert gültige Artikel sowie Fehler getrennt.

## Start und Tests

Im Repository:

```sh
cargo run -p zelyra-cli -- run examples/csv_catalog/main.zyl
cargo run -p zelyra-cli -- run examples/csv_catalog/tests.zyl
```

Oder im Beispielordner mit installierter CLI:

```sh
cd examples/csv_catalog
zelyra run main.zyl
zelyra run tests.zyl
```

Das Beispiel schreibt `data/articles.csv`, `data/articles_valid.csv` und
`data/import_errors.csv`. Die Projektkonfiguration erlaubt Dateizugriffe
ausschließlich im Ordner `data/`.

## Importregeln

Die Kopfzeile lautet `Artikelnummer;Name;Kategorie;Preis;Lagerbestand;Aktiv`.
Artikelnummer, Name, Preis, Bestand und Aktivstatus sind Pflichtfelder. Die
Artikelnummer muss eine positive Ganzzahl und innerhalb der importierten Daten
eindeutig sein. Ein leerer Kategorienwert wird zu `Allgemein`. Für Aktiv sind
`true` und `false` erlaubt. Preise akzeptieren Punkt oder Komma und höchstens
zwei Nachkommastellen; sie werden ohne binäre Gleitkommazahlen in Cent
umgerechnet. Lagerbestände müssen nichtnegative Ganzzahlen sein.

Pro ungültigem Datensatz werden alle gefundenen Feldfehler mit logischer
Datensatznummer, Feldname und Grund protokolliert. Der erste erfolgreich
importierte Artikel mit einer Nummer bleibt erhalten; ein späteres Duplikat
wird übersprungen. Gültige Artikel werden vor dem Export numerisch nach ihrer
Artikelnummer sortiert. Eingebettete Trennzeichen, Quotes und Zeilenumbrüche
werden gemäß CSV-Quoting-Regeln erhalten.

Leere Dateien, unerwartete Kopfzeilen, nicht passende Feldanzahl, beschädigte
Quote-Struktur, fehlende Dateien und nicht darstellbare Summen erhalten
verständliche Fehlermeldungen. Die Anwendung verarbeitet höchstens 10.000
Datensätze pro Datei und lehnt Summen ab, die den sicheren Ganzzahlbereich
überschreiten. Der CSV-Parser liest jeweils die gesamte Datei in den Speicher;
Streaming ist nicht Teil dieses Beispiels.

## Prüfungen

`tests.zyl` führt zehn Anwendungstests aus. Sie prüfen erzeugte Beispieldaten,
Sonderzeichen und UTF-8, Default-Kategorie, Feldvalidierung, Duplikate, Sortierung,
Fehlerexport, Dateifehler und den Export-/Import-Roundtrip. Rust-Tests ergänzen
die CSV-Parser- und Festkomma-Randfälle.
