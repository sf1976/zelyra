# Typabdeckung für Zelyra

Dieses Beispiel prüft die Typen, die der Zelyra-Compiler und die Runtime in
diesem Repository tatsächlich bereitstellen. Es bildet andere Sprachen nicht
nach und meldet fehlende Konzepte ausdrücklich als nicht unterstützt.

## Ausführen

```sh
zelyra check main.zyl
zelyra run main.zyl
```

Die positiven Prüfungen sind in mehrere Module aufgeteilt. Compilerfehler und
erwartete Runtimefehler liegen separat unter `expected/`; der CLI-Integrationstest
prüft sie mit:

```sh
cargo test -p zelyra-cli --test type_system
```

Ein Fehler in `expected/` gilt als bestanden, wenn Zelyra mit dem vorgesehenen
Diagnosetext fehlschlägt. Diese Fälle gehören nicht in den erfolgreichen
Programmlauf.

## Typinventar und Ergebnis

| Typ oder Konzept | Status | Beobachtetes Verhalten |
|---|---|---|
| `Int` | ✅ unterstützt | Vorzeichenbehaftetes 64-Bit-Intervall; auch `-9223372036854775808` ist direkt schreibbar. Addition, Subtraktion, Multiplikation, Division, Rest und Vergleiche; Überlauf und Division durch null ergeben Runtimefehler. |
| `UInt` | ✅ unterstützt | Vorzeichenloses 64-Bit-Intervall; Rechenoperationen und Vergleiche; Überlauf, Unterlauf und Division durch null ergeben Runtimefehler. |
| `Float` | ✅ unterstützt | IEEE-754 binary64; Rundungsabweichungen, Unendlich und NaN sind möglich. JSON lehnt nicht-endliche Werte ab. |
| `Decimal` | ✅ unterstützt | `decimal("0.10")` erstellt einen Festkommawert mit neun Nachkommastellen und exakter Basis-10-Arithmetik. Multiplikation und Division können Nachkommastellen nach neun Stellen abschneiden. `decimal_quantize_result(value, scale, mode)` rundet explizit ohne Float-Zwischenwert (`toward_zero`, `floor`, `ceiling`, `half_up`, `half_even`). JSON speichert Decimal als exakten Dezimalstring. |
| `Bool` | ✅ unterstützt | `true`/`false`, logische Operationen und Gleichheit. |
| `Char` | ✅ unterstützt | Ein Unicode-Skalarwert; JSON verwendet eine einstellige JSON-Zeichenkette. Graphemcluster mit mehreren Codepoints sind kein einzelnes `Char`. |
| `String` | ✅ unterstützt | UTF-8, Unicode, Escape-Sequenzen, Gleichheit, lexikografische Vergleiche und Verkettung. `text_chars` zählt Unicode-Skalarwerte; eine Byte-Längenfunktion gibt es nicht. |
| Null und optionale Werte | ✅ unterstützt | Kein eigenständiger Nulltyp. `Option<T>`, `Some` und `None` unterscheiden fehlende Werte von `0`, `false` und `""`. JSON bildet `None` auf `null` ab. |
| Arrays | ✅ unterstützt | Typisierte, verschachtelte und leere Arrays; `append`, Verkettung und `array_replace_result` liefern neue Werte. Doppelte Elemente sind erlaubt. Falsche Typen werden beim Kompilieren, ungültige Indizes als `Err` erkannt. |
| Listen | ✅ Alias | `List<T>` ist ein Typsynonym für `T[]`; Literale bleiben `[a, b]`, und Arrayoperationen gelten unverändert. Es gibt bewusst nur eine geordnete Sequenzsemantik. |
| Tupel | ✅ unterstützt | `Tuple<A, B, ...>` und `tuple(a, b, ...)` bilden geordnete heterogene Werte. Konstante Indizes werden statisch typgeprüft; dynamische Tuple-Indizes sind nicht erlaubt. JSON verwendet ein positionsgetreues Array mit geprüfter Länge. |
| Mengen | ✅ unterstützt | `Set<T>`, `set([a, b])`, `contains`, `set_add` und `set_remove`. Werte sind deterministisch sortiert, Duplikate werden entfernt. Unterstützt werden die skalar vergleichbaren Schlüsseltypen; JSON ist ein sortiertes Array. |
| `Map<K,V>` | ✅ unterstützt | Typisierte Maps; erlaubte Schlüsseltypen sind `Int`, `UInt`, `String`, `Bool`, `Char` und `Timestamp`. Doppelte Literalschlüssel behalten einen Eintrag und der letzte Wert gewinnt. Ein fehlender `get`-Schlüssel ergibt `None`. JSON unterstützt nur String-Schlüssel. |
| Records (`struct`) | ✅ unterstützt | Eigene, verschachtelte Records, Felder, Wertgleichheit und JSON-Roundtrip. `record_with(record, "feld", wert)` liefert einen typgeprüften neuen Record; der ursprüngliche Wert bleibt bestehen. |
| Enums | ✅ unterstützt | Eigene payloadfreie und payloadtragende Varianten mit `enum Event { Created(Int) Idle }`, typgeprüften Konstruktoren, Bindings in `match`, Exhaustivität, Wertgleichheit und JSON-Roundtrip. Öffentliche Enums funktionieren auch über Modulimporte. |
| Klassen/Objektidentität/Referenzen | — nicht vorhanden | Zelyra verwendet Records mit Wertsemantik und unveränderliche Funktionsabläufe. Klassen, beobachtbare Objektidentität und veränderliche Referenzen gehören derzeit nicht zur Sprachlinie. |
| `Timestamp` | ✅ teilweise nutzbar | `now()`, Gleichheit, chronologische Vergleiche, UTC-Formatierung, JSON als Unix-Millisekunden sowie `timestamp_add_result` und `timestamp_difference_result` für UTC-Dauern. Operationen sind auf ISO-Jahre 0001–9999 begrenzt. Zeitzonen und lokale Sommerzeit sind nicht enthalten. |
| `Date`, `Time`, `Duration`, `Bytes` | ✅ unterstützt | Validierte ISO-Daten und Uhrzeiten, chronologische Vergleiche, JSON-Roundtrips und geprüfte Daueraddition/-subtraktion. `date_add_days_result` und `date_days_between_result` rechnen Kalender-Tage inklusive Schaltjahren und Jahreswechseln. `Bytes` akzeptiert Werte 0–255. |
| Bereichsschleife | ✅ unterstützt | `for i in start..end` verwendet Int-Grenzen mit exklusivem Ende. Negative Grenzen, leere Bereiche, `break` und `continue` funktionieren. Der Bereich wird in einer Schleife nicht als Array materialisiert. |
| `Unit` | ✅ unterstützt | Leerer Rückgabewert, explizite Funktionstypen und JSON-Roundtrip mit `null`. |
| `Result<T,E>` | ✅ unterstützt | `Ok`/`Err`, Pattern Matching und Rückgabe aus Funktionen. Das bisherige `json_encode`-Format bleibt unverändert; `json_encode_tagged_result` und `json_decode_tagged_result<T,E>` tauschen Results verlustfrei als `{"$result":"ok","value":...}` oder `{"$result":"err","error":...}` aus. `json_decode_result<T>` bleibt der Result-basierte Decoder für gewöhnliche JSON-Ziele. |
| `HttpResult<T>`, `HttpResponse`, `HttpError` | 🧪 spezialisiert | HTTP-Subsystem-Typen für Netzwerkantworten und Fehler; keine allgemeinen Literal- oder Konstruktoren. |
| Generics | 🧪 teilweise | Generische Standardtypen sowie eigene generische Funktionen (`fn identity<T>`) mit Typinferenz oder expliziten Typargumenten sind verfügbar. Eigene generische Records/Enums und allgemeine Constraints sind noch nicht verfügbar. |
| Typ-Aliase und eigene Typen | 🧪 teilweise | `type Name = ...` sowie Records sind vorhanden. Records werden ausgeführt und serialisiert; Aliase haben keine eigene Runtimeidentität. |
| `Id`, `Email`, `Url`, `Uuid`, `Money` | 🧪 spezialisiert | Benannte String-Domainwerte für API-/Datenschema-Nutzung; keine eigenen numerischen oder Identitätsregeln in der Runtime. |
| Funktionen als Werte | — nicht vorhanden | Benannte Funktionen können aufgerufen, aber noch nicht als Werte gespeichert oder übergeben werden. Das bleibt eine offene Sprachlücke in 0.9.9. |

## Konvertierungen, Standardwerte und Kopien

Implizite numerische Konvertierungen wie `Int` zu `Float` sind nicht erlaubt.
`json_decode<T>` ist ein explizites Parsen aus JSON, kein allgemeiner Cast.
Normale Bindings brauchen einen initialen Wert; `Option` ist die vorhandene
Möglichkeit, das Fehlen eines Werts ausdrücklich darzustellen. Fehlende
optionale Recordfelder erhalten beim JSON-Decodieren `None`.

Zuweisungen und Parameterübergaben verhalten sich wertbasiert. Arrays und
Records werden intern geklont; die Sprache bietet keine Referenzidentität und
keine flachen oder tiefen Kopien als getrennte Operationen. `record_with` und
`array_replace_result` liefern geänderte Werte zurück und erhalten das Original.

## Automatisierte Abnahme

Der erfolgreiche Programmlauf prüft Zahlenbereiche, Rechenoperationen,
Rundungsverhalten, Unicode, optionale Werte, Arrays, Listen-Aliase, Tupel,
Mengen, Records, payloadfreie und payloadtragende Enums,
Kalender-/Zeitstempeloperationen, Decimal-Quantisierung, Bereiche, JSON und
benutzerdefinierte generische Funktionen.
Der CLI-Integrationstest erwartet zusätzlich **15 Compilerfehler** und
**18 Runtimefehler** (Typfehler, gemischte Arrays, unvollständiges Matching,
Überlauf/Unterlauf, Division durch null, ungültiger Index, ungültige
Date-/Time-/Bytes-/Decimal-Werte, falsche Tuple-Länge, nicht-endliche
JSON-Zahl, nicht-stringbasierte Map-Serialisierung sowie nicht unterstützte allgemeine
`json_decode<Result<...>>`-Decodierung; der separate getaggte Codec ist verfügbar). Ein erwarteter Fehler ist dabei ein
positives Testergebnis. Bei erfolgreichem Integrationstest sind alle 11
positiven Testgruppen bestanden und es gibt 0 unerwartete Fehler. Die
aktuelle Matrix umfasst 18 unterstützte, 5 teilweise unterstützte und 2 noch
nicht verfügbare Einträge; das Programm gibt diese drei Zahlen aus. Eigene
generische Funktionen und payloadtragende Enums werden im positiven
Type-System-Beispiel sowie mit expliziten Compiler-Negativfällen geprüft.
Generische Datentypen und Funktionswerte fehlen noch. Klassen und
Referenzidentität sind eine bewusste Abweichung vom wertorientierten Recordmodell;
sie sind keine Voraussetzung für Records, CRUD oder Datenbankbeziehungen.

Das ist eine Abdeckung der vorhandenen Sprachoberfläche, keine Behauptung, dass
alle Typen oder Konvertierungen bereits implementiert sind.
