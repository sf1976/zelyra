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

## Native SQL-Speicherung im Entwicklungszweig

| Zelyra | MariaDB/MySQL | PostgreSQL |
| --- | --- | --- |
| Date | DATE, Jahre 1000–9999 | DATE, Jahre 0001–9999 |
| Time | TIME, ausschließlich ganze Sekunden | TIME(0), ausschließlich ganze Sekunden |
| Timestamp | DATETIME(3), UTC-Konvention, Jahre 1000–9999 | TIMESTAMPTZ(3), Jahre 0001–9999 |
| Duration | vorzeichenbehaftete BIGINT-Millisekunden | vorzeichenbehaftete BIGINT-Millisekunden |

Date-/Time-/Timestamp-Parameter verwenden native Treiberwerte. PostgreSQL erhält
binäre Tage, Mikrosekunden der Uhrzeit oder UTC-Epochenwerte mit expliziter
Prüfung. Es werden keine allgemeinen Strings oder Float-Zwischenwerte benutzt.
PostgreSQL-Leser weisen Unendlichkeit, Zeitstempel unter Millisekundenauflösung,
Bruchteile bei Uhrzeitsekunden und Timestamp ohne Zeitzone ab. MariaDB-/MySQL-
Datums- und Zeitstempelparameter vor Jahr 1000 scheitern vor der Anweisung;
diese Speichergrenze verkleinert den Kalenderbereich der Sprache nicht.

Native Runtime-Verbindungen starten in UTC, auch wenn die PostgreSQL-Datenbank
eine andere Standardzeitzone hat. Bestehende PostgreSQL-URL-Startoptionen bleiben
erhalten; UTC wird ergänzt. Das ist die Konvention der Anwendungsverbindung;
es gibt weiterhin keine lokale Zeitzonen-/Sommerzeitumrechnung. DATETIME besitzt
selbst keine Zeitzonenmarkierung, daher müssen externe Schreiber ebenfalls UTC
verwenden.

Vor `serve` oder der ersten Interpreter-SQL-Operation prüft ein schreibfreier
Metadatenzugriff die deklarierten DATE-, ganzsekündigen TIME- und millisekündigen
Timestamp-Spalten. Fehlende Spalten, falsche SQL-Typen oder unpassende
Zeitpräzision werden abgewiesen. `serve` meldet `E-DB-TEMPORAL` an der
Quellposition der Spalte; bei Programmausführung entsteht ein Runtime-Fehler
mit Fundstelle. Metadatenrechte sind nötig. Die Prüfung legt keine Datenbank
still an und führt keine Migration aus.

Ältere TIMESTAMP-/TIMESTAMPTZ-Spalten und Skalen benötigen eine geprüfte
Migration. Mit `zelyra db plan` beginnen, die Datenbank sichern und das erzeugte
SQL vor Anwendung prüfen. Bei MariaDB-TIMESTAMP → DATETIME muss die bisherige
Verbindungszeitzone und Bedeutung der gespeicherten Zeitpunkte ausdrücklich
berücksichtigt werden. `Timestamp default now` erzeugt CURRENT_TIMESTAMP(3), passend zur Spalte und
mit Millisekunden. Die Schema-Inspektion erhält die Zeitpräzision; ein
aktuelles Schema zeigt keine falsche Migrationsabweichung. Die Änderung betrifft
auch Timestamp-Spalten für Authentifizierung und Passwort-Reset.

Siehe [calendar_storage.zyl](../examples/calendar_storage.zyl) und die
[SQL-Paketnachweise](release-readiness/0.9.9-calendar-sql.de.md).
`tests/temporal-sql-e2e.py` prüft reale Speicherung, Formulare, Filter, native
Funktionsparameter, NULL, Werte vor 1970/nach 2038, Millisekunden und alte
Schemas. Die Probe besitzt eine eigene temporäre Datenbank; der Testzugang
muss diese Datenbank anlegen und löschen dürfen.

## Typisierte SQL-Ergebnisse und Kompatibilität

Die Rohzeilen von `sql<T>` behalten ihren bisherigen Treibertext: MariaDB/MySQL
verwenden bei Mitternacht auch reine Datumsausgabe, PostgreSQL UTC-Zeitstempel
mit Z. JSON-/HTTP-Werte bleiben Strings einschließlich des alten NULL-Markers
`\N`. Native NULL-Werte werden nun in Metadaten getrennt vom Literaltext erhalten.
[`sql_decode<T>`](sql-results.de.md) erzeugt geprüfte Skalar-/Record-/Array-/
Option-Ergebnisse. Die decodierten Date-/Time-/Timestamp-Werte unterstützen
normale Kalender- und UTC-Daueroperationen; exakte Zahlen verwenden keinen Float.
Siehe [ausführbares Beispiel](../examples/typed_sql_results.zyl) und
[Drei-Backend-Nachweise](release-readiness/0.9.9-typed-sql-results.de.md).
