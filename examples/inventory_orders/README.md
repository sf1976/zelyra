# Lager- und Bestellverwaltung

Ein modular aufgebautes Zelyra-Konsolenbeispiel mit Artikeln, Kunden, Bestellungen, Lagerbewegungen, Rabatt- und Steuerberechnung, JSON-Dateiablage und Umsatzbericht.

## Start

Im Repository ausfuehren:

```sh
cargo run -p zelyra-cli -- run examples/inventory_orders/main.zyl
```

Oder mit installierter CLI im Projektverzeichnis:

```sh
cd examples/inventory_orders
zelyra run main.zyl
```

Die Daten stehen in `data/state.json`. Die Projektkonfiguration erlaubt Lesezugriff im Beispielordner und Schreibzugriff nur in `data/`.

Geldbetraege werden als ganzzahlige Cent eingegeben und gespeichert. So vermeidet das Beispiel Rundungsfehler durch binaere Gleitkommazahlen. Die Steuer betraegt 19%; der Rabatt wird vor der Steuer abgezogen. Im Bericht zaehlen nur offene Bestellungen als Umsatz.

Die Konsolenmenues bieten Artikelerfassung/Bearbeitung/Suche/Loeschung, Sortierung nach Name/Preis/Bestand und Filter, Kundenanlage/Bearbeitung/Suche/Loeschung, mehrteilige Bestellungen und Stornierung. Bestellpositionen speichern den damaligen Artikelpreis. Eine Stornierung lagert Positionen genau einmal wieder ein. Artikel mit Bestellreferenz sowie Kunden mit Bestellungen lassen sich nicht loeschen.

## Tests

```sh
cargo run -p zelyra-cli -- run examples/inventory_orders/tests.zyl
```

Die Geschaeftstests pruefen Rabatt und 19% Steuer, Bestandsgrenzen und Stornierung einschliesslich Schutz vor doppelter Einlagerung.

## Fehlerbehandlung und Protokoll

Numerische Konsoleneingaben werden als JSON-Ganzzahlen validiert; ungueltige Eingaben werden gemeldet. Datei- und JSON-Fehler werden ueber Result-basierte Runtime-Aufrufe abgefangen und gemeldet. Wenn die gespeicherte JSON-Datei fehlt oder ungueltig ist, startet die Sitzung leer und speichert nicht automatisch, damit vorhandene Daten nicht ueberschrieben werden. Aktionen werden mit Zeitstempel in `data/state.json` protokolliert.

Zeitstempel werden als UTC-ISO-8601-Datum mit Millisekunden angezeigt. Intern speichert Zelyra sie als Unix-Zeit in Millisekunden.
