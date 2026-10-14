# Typisierte SQL-Ergebniswerte

Stand: aktuelles stabiles Release Zelyra 1.0.4. [English](sql-results.en.md)

`sql<T> { … }` behält sein Rohzeilenverhalten. `sql_decode<T>` wandelt Ergebnisse
explizit in Werte für Berechnungen, Feldzugriff, Indexierung und Iteration um:

```zelyra
fn count_customers() -> Int uses Database(read) {
    return sql_decode<Int>(sql<Int> { SELECT COUNT(*) FROM customers })
}
```

Das Argument muss ein **direkter SELECT-SQL-Ausdruck** sein. Seine Annotation
nimmt weiterhin an der bestehenden SQL-/Schema-/Parameterprüfung teil. Der
Decodertyp beschreibt das umgewandelte Ergebnis und darf ein eigener flacher
Projektionsrecord sein. Zur Laufzeit werden native Spaltentypen, Namen,
NULL-Masken und exakte Werte geprüft. Datenbank-Capabilities und Projektfreigaben
gelten wie bisher für den inneren SQL-Ausdruck. `sql_decode` akzeptiert keine
SQL-Strings, Rohzeilenvariablen oder Schreibanweisungen.

| Ziel | Erforderliche Zeilen | Erforderliche Spalten |
| --- | --- | --- |
| Skalar `T` | Genau eine | Genau eine |
| Record `T` | Genau eine | Genau seine deklarierten Felder, nach Namen |
| `T[]` | Null oder mehr | Skalar- oder Record-Projektion je Zeile |
| `Option<T>` | Null oder eine | Skalar- oder Record-Projektion |

Keine Zeile ergibt bei `Option<T>` den Wert `None`; eine Zeile ergibt
`Some(wert)`. Eine NULL-Zelle verlangt einen optionalen Skalar bzw. ein
optionales Feld. Für nullable Skalare in einer Sammlung gilt `Option<Int>[]`.
Für eine möglicherweise fehlende Zeile und einen nullable Skalar gilt
`Option<Option<Int>>`: keine Zeile ergibt `None`, eine NULL-Zelle `Some(None)`
und ein Wert `Some(Some(wert))`. Fehlende Record-Spalten sind auch bei optionalen
Feldern und leeren Ergebnissen Fehler. Unerwartete oder doppelte Namen werden
abgewiesen; Spalten ausdrücklich aliasieren. Projektionsnamen beachten
Groß-/Kleinschreibung und sind von der Spaltenreihenfolge unabhängig.
Schema-/Typprüfung erfolgt vor Annahme der Zeilenkardinalität.

```zelyra
struct DueDate { issued: Date occurred: Timestamp amount: Decimal }
fn dates() -> DueDate[] uses Database(read) {
    return sql_decode<DueDate[]>(sql<calendar_events[]> {
        SELECT issued, occurred, amount FROM calendar_events ORDER BY id
    })
}
```

Aliase und Records mit konkreten generischen Argumenten werden unterstützt.
Tabellenzeilentypen verwenden ihre Tabellenspalten: nullable Speicherfelder
werden optional, Relationen erscheinen als gespeicherte `<relation>_id`-Felder.
Für Teilmengen einen eigenen DTO-Record definieren. Klassen, verschachtelte
Records/Sammlungen, Enums, Bytes, Funktionswerte und unaufgelöste generische
Decoderziele liegen außerhalb dieser flachen Ergebnis-API und werden
abgewiesen. Generische Records werden dadurch nicht zu SQL-Spaltentypen.

| Zelyra-Typ | Native SQL-Typfamilie und Wertprüfung |
| --- | --- |
| `Int`, `Id` | Integer/NUMERIC/DECIMAL; ganzzahlig im vorzeichenbehafteten 64-Bit-Bereich |
| `UInt` | Integer/NUMERIC/DECIMAL; ganzzahlig im vorzeichenlosen 64-Bit-Bereich |
| `Decimal` | Integer/NUMERIC/DECIMAL; exakte Skala bis 9 Stellen und bisherige Decimal-Grenzen |
| `Money` | Integer/NUMERIC/DECIMAL; exakter DECIMAL(19,4)-Bereich ohne Rundung |
| `Float` | Integer/NUMERIC/DECIMAL/Float; endliche 64-Bit-Float-Konvertierung |
| `Bool` | Boolean oder Integer; ausschließlich true/false oder 1/0 |
| `String`, `Char` | Text; Char verlangt genau einen Unicode-Skalar |
| `Email`, `Url`, `Uuid` | Textdarstellung; der Decoder ergänzt keine Domain-Inhaltsprüfung |
| `Date`, `Time`, `Timestamp` | Passende native Zeittypfamilie; geprüftes Kalenderdatum, volle Sekunden, UTC-Millisekunden |
| `Duration` | Integer/NUMERIC/DECIMAL; ganzzahlige vorzeichenbehaftete Millisekundendauer |

Float-Eingaben werden für exakte Zahlentypen abgewiesen, auch wenn ihr
angezeigter Text ganzzahlig wirkt. Binäre MySQL-/MariaDB-Strings werden nicht als
Text decodiert; gegebenenfalls ausdrücklich im SQL zu Text konvertieren. Werte
werden nie still gerundet, abgeschnitten oder begrenzt. Exakte Typen verwenden
keinen Float-Zwischenwert. Native NULL-Werte werden getrennt vom Literaltext
`\N` erhalten. PostgreSQLs binäre NUMERIC- und Kalender-/Zeitwerte behalten ihre
bestehenden exakten Codecs.

JSON-Eingaben für eingebautes Money sind exakte Dezimalstrings mit derselben
DECIMAL(19,4)-Prüfung. Id-JSON-Decodierung akzeptiert zusätzlich vorzeichenbehaftete
ganze Zahlen; bisherige Stringeingaben bleiben erhalten. Flache DTOs mit diesen
Typen bestehen `json_encode`-/`json_decode`-Roundtrips.

Die eingebauten Typen `Id`/`Money` verwenden zur Laufzeit Int/Decimal, eingebaute
Textdomains String. Funktionsgrenzen erkennen diese Darstellungen. Bestehende
ausdrückliche Aliase/Record-Definitionen haben weiterhin Vorrang. Neue
Id-Identitätsregeln oder Domainprüfungen werden nicht eingeführt. Zeitgrenzen
stehen in den [Kalenderregeln](dates.de.md), Zahlengrenzen in den
[Decimal-Regeln](decimals.de.md).

Die bisherige SQL-Ausgabe, `json_encode(sql<T> { … })` und bestehende
Rohzeilen-HTTP-Antworten bleiben Arrays aus Objekten mit **Stringwerten**,
einschließlich `\N` für NULL. Nach expliziter Konvertierung gelten die üblichen
typisierten JSON-/HTTP-Regeln: Zahlen und Boolean sind native JSON-Werte,
Date/Time Strings, Timestamp/Duration ganzzahlige Millisekunden, Decimal/Money
exakte Dezimalstrings und `None` JSON-null. Die Umstellung eines bestehenden
Endpoints auf decodierte Werte ist eine ausdrückliche Anwendungs-API-Änderung.
Es gibt keinen globalen Schalter und kein implizites Upgrade.

Ungültige Aufrufe/Zielformen liefern `E-TYPE-SQL-DECODE` mit Fundstelle.
Konvertierungsfehler verwenden das bestehende `E-RUNTIME-001` am Decoderaufruf,
mit Spaltennamen und bei ungültigen Zellen mit Zeilennummern ab eins. Zellinhalte
werden in Konvertierungsfehlern ausgelassen. Query-/Treiberfehler behalten ihre
bestehenden Diagnosen. Migrationsfreigaben und Backend-SQL-Unterstützung ändern
sich dadurch nicht.

Das [ausführbare Beispiel](../examples/typed_sql_results.zyl) läuft nach
`zelyra db apply` gegen eine eigene Testdatenbank. Echte Drei-Backend-HTTP-/SQL-
Nachweise und verbleibende Release-Gates stehen im
[Paketbericht](release-readiness/0.9.9-typed-sql-results.de.md).
