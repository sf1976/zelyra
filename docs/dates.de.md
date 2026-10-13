# Datum, Uhrzeit und UTC-Dauern

Stand: unveröffentlichter 0.9.9-Entwicklungszweig.
[English](dates.en.md)

```zelyra
issued: Date = date("2026-10-12")
match date_add_days_result(issued, 14) {
    Ok(due) => { print(due) }
    Err(reason) => { print(reason) }
}
```

Das Ergebnis ist `2026-10-26`. `date_days_between_result(start, end)` liefert
`end - start` in Kalendertagen. Negative Werte und Rückwärtsrechnung sind
zulässig. Der proleptische gregorianische Kalender unterstützt die Jahre
0001 bis 9999; 1900 und 2100 sind keine Schaltjahre, 2000 und 2400 dagegen
schon. Monat, Tag und Schaltjahr werden geprüft. Außerhalb des Bereichs liefern
die Rechenfunktionen `Err`; es gibt keine automatische Monatskorrektur.

`Date` benutzt ausschließlich `YYYY-MM-DD`, `Time` ausschließlich `HH:MM:SS`
von `00:00:00` bis `23:59:59`. Constructoren `date`/`time` und JSON-Eingaben
prüfen diese Werte. Schaltsekunden, Zeitzonen, Sommerzeit und lokale
Uhrzeitumrechnung sind nicht implementiert. Kalender-Tagesrechnung ist von
UTC-Dauerrechnung getrennt.

## Timestamp und Duration

`Timestamp` ist ein UTC-Instant in ganzzahligen Unix-Millisekunden.
`Duration` ist eine vorzeichenbehaftete ganzzahlige Millisekunden-Dauer (`Int`-
Wertebereich). Timestamp reicht von `-62135596800000`
(`0001-01-01T00:00:00.000Z`) bis `253402300799999`
(`9999-12-31T23:59:59.999Z`). `json_decode<Timestamp>` verlangt eine JSON-
Ganzzahl innerhalb dieses Bereichs; Werte außerhalb werden nun kontrolliert
mit Quellposition abgewiesen. Der zuvor akzeptierte ganze i64-Bereich wird
nicht als gültiger ISO-Kalenderbereich weitergeführt.

`timestamp_add_result(stamp, duration)` und
`timestamp_difference_result(end, start)` liefern geprüfte `Result`-Werte.
Chronologische Vergleiche verwenden die Millisekundenwerte.
`format_timestamp` gibt UTC mit genau drei Nachkommastellen und `Z` aus,
auch vor 1970. Keine dieser Berechnungen verwendet Float.

## Formulare, HTTP und Codegenerierung

Date-/Time-Formularwerte werden unabhängig vom Browser gegen den Kalender
und die Uhrzeit geprüft. Auch an Tabellen gebundene Aliasse behalten diese
Prüfung. Timestamp-Formulare akzeptieren Unix-Millisekunden oder einen
expliziten UTC-ISO-Wert mit `Z`; lokale Uhrzeiten und Offsetzonen werden nicht
erraten. Zulässig sind ganze Sekunden oder bis zu drei Nachkommastellen.
Duration-Formulare verlangen ganzzahlige Millisekunden.

Die HTTP-API erzeugt echte Runtime-Werte für Date, Time, Timestamp und Duration;
sie übergibt diese nicht mehr als allgemeine Strings an Handler. Ungültige
Daten werden mit HTTP 400 abgefangen. Date und Time bleiben JSON-Strings;
Timestamp und Duration bleiben JSON-Ganzzahlen in Millisekunden. Timestamp-
API-Eingaben können zusätzlich explizites UTC-ISO-Textformat oder numerischen
Text verwenden. OpenAPI und TypeScript beschreiben die vorhandenen Ausgabe-
Wireformate korrekt. Der Sprach-JSON-Codec wird nicht still geändert.

## Prüfungen und verbleibende Datenpfade

`scripts/check_calendar_references.py` prüft unabhängig mit Python `datetime`:
1.008 Datumspaare, 200 UTC-Dauerpaare und 200 UTC-Formatwerte über den ganzen
Kalenderbereich. Ein ausführbares Beispiel ist
[calendar_due_date.zyl](../examples/calendar_due_date.zyl).
`tests/temporal-api-e2e.py` prüft reale HTTP-Rundläufe, Date-Aliasse, optionale
Werte, die Jahresgrenzen und 13 ungültige Eingaben.

Diese Prüfung bestätigt die Kalender-/UTC-Rechnung und die aufgeführten
Eingabe-/HTTP-Pfade. Datenbank-Speicherung, PostgreSQL-Datumscodecs,
Timestamp-Skalierung und typisierte SQL-Ergebnisverwendung sind eigene,
noch offene technische 0.9.9-Gates. Der gesamte Datumspfad wird erst nach
Schließen dieser Punkte als geprüft bezeichnet. Eine SQL-Typannotation allein
ist derzeit kein Nachweis einer automatisch dekodierten Datumsvariable.
