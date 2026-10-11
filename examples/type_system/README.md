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
| `Int` | ✅ unterstützt | Vorzeichenbehaftetes 64-Bit-Intervall; Addition, Subtraktion, Multiplikation, Division, Rest und Vergleiche; Überlauf und Division durch null ergeben Runtimefehler. |
| `UInt` | ✅ unterstützt | Vorzeichenloses 64-Bit-Intervall; Rechenoperationen und Vergleiche; Überlauf, Unterlauf und Division durch null ergeben Runtimefehler. |
| `Float` | ✅ unterstützt | IEEE-754 binary64; Rundungsabweichungen, Unendlich und NaN sind möglich. JSON lehnt nicht-endliche Werte ab. |
| `Decimal` | 🧪 teilweise | Der Typname wird akzeptiert, die Runtime speichert und rechnet ihn aber als `Float`. Es gibt keine Dezimalarithmetik mit exakter Basis 10. Nicht für Geldbeträge verwenden. |
| `Bool` | ✅ unterstützt | `true`/`false`, logische Operationen und Gleichheit. |
| `Char` | ✅ unterstützt | Ein Unicode-Skalarwert; JSON verwendet eine einstellige JSON-Zeichenkette. Graphemcluster mit mehreren Codepoints sind kein einzelnes `Char`. |
| `String` | ✅ unterstützt | UTF-8, Unicode, Escape-Sequenzen, Gleichheit, lexikografische Vergleiche und Verkettung. `text_chars` zählt Unicode-Skalarwerte; eine Byte-Längenfunktion gibt es nicht. |
| Null und optionale Werte | ✅ unterstützt | Kein eigenständiger Nulltyp. `Option<T>`, `Some` und `None` unterscheiden fehlende Werte von `0`, `false` und `""`. JSON bildet `None` auf `null` ab. |
| Arrays | ✅ unterstützt | Typisierte, verschachtelte und leere Arrays; `append`/Verkettung liefern Werte zurück. Doppelte Elemente sind erlaubt. Falsche Typen werden beim Kompilieren, ungültige Indizes zur Laufzeit erkannt. |
| Listen | — nicht vorhanden | Arrays sind die vorhandene Sammlung; ein eigener `List<T>`-Typ existiert nicht. |
| Tupel | — nicht vorhanden | Kein Tupeltyp; die erwartete Typdatei wird als Compilerfehler geprüft. |
| Mengen | — nicht vorhanden | Kein Set-Typ. |
| `Map<K,V>` | ✅ unterstützt | Typisierte Maps; erlaubte Schlüsseltypen sind `Int`, `UInt`, `String`, `Bool`, `Char` und `Timestamp`. Doppelte Literalschlüssel behalten einen Eintrag und der letzte Wert gewinnt. Ein fehlender `get`-Schlüssel ergibt `None`. JSON unterstützt nur String-Schlüssel. |
| Records (`struct`) | ✅ unterstützt | Eigene, verschachtelte Records, Felder, Wertgleichheit und JSON-Roundtrip. Bindings sind standardmäßig unveränderlich; ein `mutable` Binding kann durch einen neuen Recordwert ersetzt werden. |
| Enums | — nicht vorhanden | Kein benutzerdefinierter Enumtyp. `Option` und `Result` sind eingebaute Varianten-Typen. |
| Klassen/Objektidentität/Referenzen | — nicht vorhanden | Records werden als Werte behandelt. Es gibt keine beobachtbare Objektidentität oder Referenzsemantik und keine Mutation einzelner Recordfelder. |
| `Timestamp` | ✅ teilweise nutzbar | `now()`, Gleichheit, UTC-Formatierung und JSON als Unix-Millisekunden. Keine allgemeine Datum-/Zeitrechnung oder geordnete Vergleiche im Sprachkern. |
| `Date`, `Time`, `Duration`, `Bytes` | 🧪 Typnamen vorhanden | Parser/Typmodell kennen die Namen, aber Runtimewerte und JSON-Roundtrips fehlen. Die Beispiel-Fixtures prüfen die abgewiesene Deserialisierung von `Date` und `Bytes`. |
| `Unit` | ✅ unterstützt | Leerer Rückgabewert, explizite Funktionstypen und JSON-Roundtrip mit `null`. |
| `Result<T,E>` | ✅ unterstützt | `Ok`/`Err`, Pattern Matching und Rückgabe aus Funktionen. JSON kodiert `Ok` als enthaltenen Wert und `Err` als Objekt mit `error`; ein generischer `Result`-Decoder fehlt. |
| `HttpResult<T>`, `HttpResponse`, `HttpError` | 🧪 spezialisiert | HTTP-Subsystem-Typen für Netzwerkantworten und Fehler; keine allgemeinen Literal- oder Konstruktoren. |
| Generics | 🧪 eingebaut | Generische Standardtypen wie `Option`, `Result`, `Map`, Arrays und `json_decode<T>` sind vorhanden. Eigene generische Funktionen und Datentypen sind nicht verfügbar. |
| Typ-Aliase und eigene Typen | 🧪 teilweise | `type Name = ...` sowie Records sind vorhanden. Records werden ausgeführt und serialisiert; Aliase haben keine eigene Runtimeidentität. |
| `Id`, `Email`, `Url`, `Uuid`, `Money` | 🧪 spezialisiert | Benannte String-Domainwerte für API-/Datenschema-Nutzung; keine eigenen numerischen oder Identitätsregeln in der Runtime. |
| Funktionen als Werte | — nicht vorhanden | Funktionen können aufgerufen, aber nicht als Werte gespeichert oder übergeben werden. |

## Konvertierungen, Standardwerte und Kopien

Implizite numerische Konvertierungen wie `Int` zu `Float` sind nicht erlaubt.
`json_decode<T>` ist ein explizites Parsen aus JSON, kein allgemeiner Cast.
Normale Bindings brauchen einen initialen Wert; `Option` ist die vorhandene
Möglichkeit, das Fehlen eines Werts ausdrücklich darzustellen. Fehlende
optionale Recordfelder erhalten beim JSON-Decodieren `None`.

Zuweisungen und Parameterübergaben verhalten sich wertbasiert. Arrays und
Records werden intern geklont; die Sprache bietet keine Referenzidentität und
keine flachen oder tiefen Kopien als getrennte Operationen. Der getestete
„Ändern“-Pfad ist das Neusetzen eines `mutable` Bindings, nicht das Ändern eines
Recordfelds oder eines Arrayelements über einen Index.

## Automatisierte Abnahme

Der erfolgreiche Programmlauf prüft Zahlenbereiche, Rechenoperationen,
Rundungsverhalten, Unicode, optionale Werte, Sammlungen, Records und JSON.
Der CLI-Integrationstest erwartet zusätzlich **7 Compilerfehler** und
**12 Runtimefehler** (Typfehler, gemischte Arrays, unvollständiges Matching,
Überlauf/Unterlauf, Division durch null, ungültiger Index, nicht-endliche
JSON-Zahl, nicht-stringbasierte Map-Serialisierung sowie nicht unterstützte
`Date`-/`Bytes`-/`Result`-Decodierung). Ein erwarteter Fehler ist dabei ein
positives Testergebnis. Bei erfolgreichem Integrationstest sind alle 7
positiven Testgruppen bestanden und es gibt 0 unerwartete Fehler.

Das ist eine Abdeckung der vorhandenen Sprachoberfläche, keine Behauptung, dass
alle Typen oder Konvertierungen bereits implementiert sind.
