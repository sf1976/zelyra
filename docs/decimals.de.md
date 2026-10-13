# Decimal und Geldbeträge

Diese Regeln beschreiben den unveröffentlichten Entwicklungszweig 0.9.9.
[English](decimals.en.md)

## Deklaration und Berechnung

```zelyra
betrag: Decimal = decimal("119.00")
steuersatz: Decimal = decimal("0.19")
steuer: Decimal = betrag * steuersatz
```

`decimal` verlangt einen Dezimalstring. Ein Literal wie `119.00` ist `Float`
und wird nicht implizit zu `Decimal`. Der Festkommawert hat neun Nachkommastellen
und einen skalierten, vorzeichenbehafteten 128-Bit-Wertebereich:
`-170141183460469231731687303715.884105728` bis
`170141183460469231731687303715.884105727`. Überlauf des Ergebnisses und Division
oder Restbildung durch null sind kontrollierte Laufzeitfehler mit Quellposition.
Multiplikation und Division verwenden bei Bedarf einen exakten, auf 256 Bit
begrenzten Zwischenwert; ein gültiges Ergebnis scheitert nicht allein an einem
128-Bit-Zwischenüberlauf. Überschüssige Nachkommastellen werden weiterhin gegen
null abgeschnitten. Das ist keine automatische Geldrundung.

Konstruktor und Formulare akzeptieren einen Punkt als Dezimaltrennzeichen,
keine Exponenten, kein führendes Plus, keine Leerzeichen, kein NaN/Unendlich
und höchstens neun Nachkommastellen. `decimal_scaled_result` behält seinen
separaten Vertrag für geprüfte Cent-Eingaben, einschließlich Dezimalkomma.

## Geld ausdrücklich runden

`decimal_quantize_result(betrag, 2, "half_up")` liefert
`Result<Decimal, String>`. Modi: `toward_zero`, `floor`, `ceiling`, `half_up`
(Halbwerte vom Nullpunkt weg) und `half_even` (Halbwerte zur geraden letzten
Ziffer). Sie gelten auch für negative Werte. Skalen von 0 bis 9 sind zulässig.
Das Runden beseitigt keinen bereits beim Rechnen eingetretenen Präzisionsverlust.

`format_text` entfernt bedeutungslose Nachkommastellen. Für eine feste
Geldanzeige wie `119.00` den gerundeten Wert über `decimal_scaled_result` und
`format_scaled_result` ausgeben. Ein ausführbares Beispiel ist
[decimal_money.zyl](../examples/decimal_money.zyl):

```bash
zelyra check examples/decimal_money.zyl
zelyra run examples/decimal_money.zyl
```

## Datenbank, Formulare und HTTP

| Deklaration | MariaDB / MySQL | PostgreSQL | Formularvertrag |
| --- | --- | --- | --- |
| `betrag: Decimal required` | `DECIMAL(39,9)` | `NUMERIC(39,9)` | Exakter Festkommawert mit höchstens 9 Nachkommastellen |
| `preis: Money required` | `DECIMAL(19,4)` | `NUMERIC(19,4)` | Höchstens 15 Vor- und 4 Nachkommastellen |
| `anzahl: UInt required` | `DECIMAL(20,0)` | `NUMERIC(20,0)` | Ganzzahl von 0 bis `18446744073709551615` |

`Money` bleibt ein spezialisierter Schema-/Formulartyp; er ersetzt den
numerischen Runtime-Typ `Decimal` nicht und bedeutet keine automatische
Cent-Rundung. Sein bestehender Vierstellenvertrag wird nicht eingeschränkt.
Optionale leere numerische Formularfelder werden als SQL-`NULL` gebunden.
An Tabellen gebundene Decimal-/Money-Aliasse behalten ihren Speichervertrag,
auch als optionale Spalten. Exakte Standard-Eingabefelder verwenden Text mit
`inputmode="decimal"`, ohne JavaScript-Zahlenumwandlung. Ein ausdrücklich
gewähltes Widget bleibt eine eigene Darstellungsentscheidung.

SQL-Parameter für Decimal, Money und zugehörige Filter werden als geprüfter
Dezimaltext transportiert und ausdrücklich numerisch gebunden, ohne Float.
PostgreSQL-NUMERIC-Ergebnisse werden exakt dekodiert; nicht-endliche numerische
Ergebnisse werden kontrolliert abgewiesen. Einfache PostgreSQL-Formularaktionen
laufen atomar. Mandanten-/Empfängerprüfungen, Edit-Snapshots und Audit-Entscheidungen
haben weiterhin einen ausdrücklichen PostgreSQL-Blocker; dieses Paket belegt
keine vollständige Backend-Parität. SQLite-Schemawerkzeuge garantieren keine
exakte Decimal-Runtime-Speicherung durch ihre NUMERIC-Affinität.

Der Sprachcodec `json_encode` schreibt Decimal weiterhin als String;
`json_decode<Decimal>` verlangt diesen String. Die HTTP-API akzeptiert zusätzlich
exakt darstellbare JSON-Zahlen, einschließlich Exponenten, und weist erforderliche
Rundung ab. Antworten, OpenAPI und TypeScript verwenden Dezimalstrings. Empfohlen:

```json
{"betrag":"119.00"}
```

Ein Browser kann eine JavaScript-Zahl schon vor dem Absenden runden; der Server
kann verlorene Ziffern nicht wiederherstellen. JSON-Zahlen bleiben beim Parsen
verlustfrei erhalten, und Objektschlüssel wie `$serde_json::private::Number`
werden als gewöhnliche Schlüssel behandelt. Die JSON-Tiefengrenze bleibt 128.

## Bestehende Projekte aktualisieren

Ein altes MariaDB-`Decimal` konnte als `DECIMAL(10,0)` gespeichert sein. Die
Schemaänderung ist sichtbar in `zelyra db plan`; sie wird nicht automatisch
freigegeben. Vor SQL-Ausführung und vor `serve` mit freigegebener
Datenbankfähigkeit wird die deklarierte feste Präzision geprüft. Abweichung oder
fehlende Metadaten verhindern den Betrieb; `E-DB-PRECISION` markiert bei `serve`
die betroffene Quellspalte, die Runtime meldet die SQL-Fundstelle.

Zuerst sichern, den Plan prüfen und erst danach bewusst anwenden. Bereits früher
abgeschnittene Nachkommastellen lassen sich durch eine Schemaänderung nicht
zurückholen. Die Prüfung erfolgt beim Serverstart beziehungsweise vor der ersten
SQL-Ausführung eines Interpreters; sie ersetzt keine Kontrolle konkurrierender
Schemaänderungen. Metadatenrechte sind erforderlich.

Decimal-Aliasse werden in Typprüfung, Rückgaben und SQL gegen den Grundtyp
geprüft. Zyklische Aliasse erhalten `E-TYPE-ALIAS-001`; Expansionen sind auf 64
Ebenen und 4.096 Typknoten begrenzt. Unpassende String-/Float-Werte werden durch
Aliasse nicht gültig. Nominale Records, Klassen und Enums behalten ihre Identität.

Rohe SQL-Zeilen behalten ihre bisherige Textdarstellung. Ein SQL-Ergebnistyp
allein garantiert derzeit keine automatisch dekodierte Skalar-/Record-Nutzung;
diese weitergehende Datenpfadprüfung ist ein separates 0.9.9-Gate.
