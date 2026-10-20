# Experimentelles Zelyra-Bytecode v26

`zelyra-mir::bytecode` bietet das erste ausführbare Bytecode-Backend für
bereits typgeprüfte und verifizierte MIR. Der AST-Interpreter bleibt die Referenzimplementierung und das normale
Runtime-Backend. Die Bytecode-VM kann für ihre dokumentierte Teilmenge mit
`zelyra run <datei.zyl> --backend bytecode` ausdrücklich ausgewählt werden;
nicht unterstützte Programme brechen mit einer quellpositionsgebundenen
Diagnose ab. AST bleibt Standard.

## Aktueller Umfang

Der Bytecode-Compiler wählt eine benannte Einstiegsfunktion und deren direkte
Aufrufabhängigkeiten einschließlich Funktionen aus, die aus Vor- oder
Nachbedingungen aufgerufen werden. Er senkt skalare Literale (`Int`, `UInt`, `Float`,
`Bool`, `String`, `Char`), lokale Variablen, unäre und binäre Operationen,
kurzschließende boolesche Operatoren, direkte reine Funktionsaufrufe, lokale
Zuweisungen, Verzweigungen und Rückgaben. `print(value)` ist als erfasste
Ausgabe unterstützt: Die CLI gibt jede Zeile aus und die erfasste
Ausführungs-API bewahrt bereits erzeugte Ausgabe auch bei späteren
Laufzeitfehlern. Wie im AST-Interpreter benötigt `print` keine Console-
Capability. Arithmetik und Ordnung sind für
`Int`, `UInt` und `Float` unterstützt; Strings unterstützen Verkettung und
Ordnung, Boolesche Werte Logik/Vergleich und `Char` Gleichheit. Array- und
Tupelliterale, generische Record-Literale (mit expliziten oder inferierten
Typargumenten), typisierte Record-Feldzugriffe sowie Array-/Tupel-Lesezugriffe
per Index werden ebenfalls unterstützt. Arrays mit demselben Elementtyp lassen
sich mit `+` verketten. Typisierte Map-Literale, leere Maps
mit explizitem Map-Typ und Map-Lookups über das gemeinsame Built-in
`get(map, key) -> Option<V>` sind unterstützt. Bei doppelten Literalschlüsseln
bleibt die ursprüngliche Position erhalten und der letzte Wert gewinnt. Direkte
Map-Klammerindexierung gehört nicht zur Zelyra-Syntax. Bei Record-Konstruktion wird jedes
Feld gegen die geprüfte Deklaration validiert; Feldzugriffe prüfen zur Laufzeit
Empfänger- und Ergebnistyp. Record-Updates mit `..basis` kopieren einen
geprüften Wert und ersetzen ausdrücklich angegebene Felder; generische
Recordtypen werden nach ihren konkreten Typargumenten geprüft. JSON-Feldzugriffe
gehören nicht zu diesem Umfang. Record-Arrays können durchlaufen werden; ihre
typisierten Felder lassen sich im Schleifenkörper lesen. Sets/Maps mit Records
sind nicht als unterstützter Pfad geprüft. Ausdrücklich veränderliche Arrays
unterstützen indizierte Ersetzung, `+=`-Updates und `push_mut`/`pop_mut` auf
explizit mutable Bindungen. Bytecode v25 codiert `pop_mut` als eigene
verifizierte Instruktion; der Verifier prüft Zielmutabilität und Payloadtyp.
Indizierte Änderungen aktualisieren das vorhandene lokale Array in AST, MIR
und Bytecode direkt; dabei wird nicht das vollständige Array kopiert.
Array-, Tuple-, Option- und Resultwerte gleichen Typs können mit `==`/`!=`
verglichen werden. Die Indexierung verwendet die gemeinsame MIR-Runtime-Semantik,
einschließlich quellpositionsgebundener Fehler für negative und ungültige
Indizes. Tupelindizes müssen, wie vom Typprüfer verlangt, nicht-negative
Integerliterale sein. Decimal-Arithmetik, Addition/Subtraktion von `Duration`
und Ordnungsvergleiche für `Decimal`, `Duration`, `Date`, `Time` und `Timestamp`
sind unterstützt. Arithmetik mit Date, Time und Timestamp wird beim
Kompilieren weiterhin abgelehnt. AST, MIR und Bytecode delegieren Operatoren auf
Runtime-Werten für unäre und binäre Operationen an gemeinsame Runtime-Helfer.
Der Bytecode-Compiler erlaubt weiterhin nur die hier aufgeführten
Typ-/Operator-Kombinationen; Decimal-, Duration-, Array- oder Zeitvergleiche
im Helfer bedeuten keine Bytecode-Unterstützung. Skalare `while`- und
`loop`-Kontrollflüsse einschließlich `break` und `continue` werden über
verifizierte MIR-Verzweigungen und Sprünge ausgeführt; ihre Instruktionen
verbrauchen dasselbe deterministische Schrittbudget. `for value in
range(start, end[, step])` wird mit genau einer Bindung für `Int`-Bereiche
unterstützt. Grenzen und Schritt werden bei der Initialisierung einmal
ausgewertet; das Ende ist exklusiv, absteigende Bereiche benötigen einen
negativen Schritt und ein Schritt von null löst einen Laufzeitfehler aus.
`for value in values`-Schleifen über `Array<T>` und `Set<T>` sind ebenfalls
unterstützt; die Collection wird einmal ausgewertet und die Iterationsreihenfolge
der Runtime bleibt erhalten. Collection-Elemente können in Tupelbindungen destrukturiert werden; normale
Tupelzuweisungen binden neue oder bestehende lokale Variablen. Die VM prüft
Tupeltyp und Anzahl der Werte zusätzlich zur Laufzeit.
Die VM
verwendet iterative Rust-Aufrufrahmen, Quellspans an Bytecode-Instruktionen, einen Verifier für
Operandenstack, Sprungziele und Aufrufargumente sowie Schritt- und
Aufruftiefenlimits.

MIR und Bytecode prüfen Schleifeninvarianten vor jeder Bedingungs-/Körperrunde
und danach, auch auf `break`-, `continue`- und `return`-Pfaden. Der Rückgabewert
wird vor den aktiven Invarianten ausgewertet, wie im AST-Interpreter.
Verschachtelte Rückgaben prüfen die Invarianten von der innersten Schleife nach
außen. Fehler zeigen auf die jeweilige Schleifenanweisung.

Unterstützte HostServices-Effekte sind `Clock` (`now()`), `Environment`
(`env(name)`), `Console` (`read_console(prompt)`) und `Random`
(`random_int(min, max)` und `uuid_v4()`) sowie `Process` über
`run_process(command, arguments)`. Der Prozessaufruf benötigt eine deklarierte
und ausdrücklich freigegebene `Process`-Capability sowie eine Runtime-Policy
mit exakt erlaubtem Befehl, positivem Timeout und Ausgabelimit. Der Host startet
den Befehl mit getrennten Argumenten und ohne Shell. Dateisystem-Built-ins umfassen Lesen
mit `read_text(path)` und `read_text_result(path)`, Metadaten mit
`file_exists(path)`, `dir_exists(path)` und `file_size(path)` sowie
Verzeichnislisten mit `list_dir(path)` und Schreibzugriffe nur innerhalb der
Schreib-Roots mit `write_text(path, content)` und
`write_text_result(path, content)`, `write_text_new_result(path, content)`,
`atomic_write_text_result(path, content)` und `append_text_result(path, content)`
sowie Löschen innerhalb der Schreib-Roots mit
`delete_file(path)`, `delete_file_result(path)`, `copy_file_new_result(source, destination)` und `rename_file_result(source, destination)`. Sie nutzen den injizierten Host und
dieselben Kanonisierungs-, Symlink- und konfigurierten Root-Prüfungen wie die
AST-Runtime. `read_text_result` gibt Pfad- und E/A-Fehler
als `Result` zurück; `read_text` meldet sie als Laufzeitfehler. Die lesenden
Metadatenfunktionen umfassen `file_exists(path)`, `dir_exists(path)` und
`file_size(path)`; letzteres liefert `Result<Int, String>`. Existenzprüfungen
liefern `false`, wenn ein Pfad außerhalb konfigurierter Roots liegt oder nicht
untersucht werden kann. `write_text_result` gibt Schreibfehler als `Result`
zurück; `write_text` meldet sie als Laufzeitfehler. `delete_file_result` gibt
Pfad-/E/A-Fehler als `Result` zurück; `delete_file` meldet sie als
Laufzeitfehler. Beide Löschvarianten verlangen die `FileSystem`-Capability und
liegen innerhalb der konfigurierten Schreib-Roots. Exklusives Schreiben, atomisches Ersetzen, Anhängen, Kopieren und Umbenennen
sind über die dafür vorgesehenen Built-ins verfügbar. Alle genannten
Dateisystemaufrufe verwenden die konfigurierte Runtime-Policy. `list_dir(path)` gibt die vom Host gelieferten Eintragsnamen als `String[]` zurück; der Systemhost sortiert sie.
Pfad-, Berechtigungs- oder E/A-Fehler führen zu einem Laufzeitfehler.
Eine verifizierte
`HostCallPreflight`-Anweisung prüft Deklaration und ausdrückliche
Projektfreigabe, bevor Aufrufargumente ausgewertet werden. Der Verifier verlangt
eine passende Vorprüfung für jeden Host-Built-in-Aufruf und weist uneinheitliche
offene Aufrufe an Kontrollflussverbindungen zurück. So gilt die Autorisierungs-
reihenfolge von AST/MIR, und Argumente können vor einer Ablehnung keine
Seiteneffekte ausführen. Funktions-Capabilities werden im Bytecode
gespeichert und geprüft; zur Ausführung sind sowohl die passende deklarierte
Capability als auch eine ausdrücklich übergebene Projektfreigabe und ein
injizierter Hostprovider nötig. Die Komfortfunktion `execute` erteilt keine
Hostfreigaben. `http_get(url)` und
`http_request(method, url, headers, body)` sind die unterstützten Network-
Operationen. `http_json<Request, Response>(method, url, headers, body)` und
`http_result<Request, Response>(...)` verwenden den gemeinsamen URL-,
Host-Allowlist-, Timeout-, Header-, Antwortgrößen- und UTF-8-Prüfer. Beide
serialisieren und dekodieren Records, Aliase und Enums anhand der geprüften
Typdefinitionen aus dem Bytecode-Artefakt. Wie im AST-Interpreter wird bei
fehlendem Content-Type `application/json` ergänzt. `http_json` akzeptiert nur
2xx-Antworten; `http_result` bildet Status, Header, Body sowie optionale Daten
oder einen `HttpError` ab. `mail_deliver(event_id, payload_json)` ist ebenfalls
unterstützt und liefert `Result<Unit, String>`. Es benötigt die deklarierte und
ausdrücklich freigegebene `Email`-Capability sowie konfigurierte SMTP- und
Netzwerk-Policies; die gemeinsame Runtime prüft diese vor dem injizierten
Hostaufruf. Tests verwenden einen Fake-Host und versenden keine E-Mail.
Transaktionales `mail_queue` bleibt im Bytecode abgelehnt. Einfache statische
SQL-Ausdrücke sind in Bytecode v19 verfügbar, wenn die Anwendung einen
`BytecodeDatabaseExecution`-Kontext mit `DATABASE_URL` und geprüftem Projektschema
übergibt. Die VM prüft beim Ausführen erneut SQL-Syntax, Schema, Parametertypen,
Capability und optionale Projektfreigabe, bevor sie Speicherprüfung oder
Datenbank-Host aufruft. `zelyra run --backend bytecode` übergibt diesen Kontext,
wenn eine Datenbankverbindung konfiguriert ist. Bytecode v21 unterstützt
Transaktionen mit geprüften SQL-Anweisungen und `mail_queue`-Aufrufen mit
optionalen HTML- und PDF-Anhängen. Das Artefakt enthält die referenzierten
Views; `render_pdf` nutzt denselben begrenzten, lokalen und textorientierten
Renderer wie AST und MIR. Bei einem
Outbox-Eintrag prüft die VM `Email`- und `Database(write)`-Freigaben, bevor sie
Nachrichtenfelder auswertet. Sie validiert die Nachricht und übergibt den
Outbox-Eintrag zusammen mit SQL in einem Batch; sie versendet die E-Mail nicht.
SQL-Schema und Speicherkompatibilität werden vor dem Host-Batch geprüft.
Andere Transaktionseffekte bleiben sicher abgelehnt.
Enum-Payload-Konstruktoren und Match-Anweisungen mit
Enum-/Option-/Result-/Record-/Literal-/Wildcardmustern einschließlich Guards sind
unterstützt. Inline-Lambda-Callbacks, benannte Funktionsreferenzen wie `&double` und lokale Callback-Variablen arbeiten mit `map`, `filter`, `find` und `reduce`. Benannte Funktionswerte dürfen auch als typisierte Parameter weitergereicht und zur Laufzeit aufgerufen werden; die VM prüft Signatur und Capabilities des Ziels erneut. Bytecode v22 materialisiert gespeicherte Lambdas als Funktionswerte: Capture-LocalIds und Typen stehen im Artefakt, der Verifier prüft Capture-Layout und Capability-Obergrenze und weist Schreiboperationen auf eingefangene LocalIds zurück; die VM kopiert Capture-Werte beim Erzeugen der Closure. Seit v23 enthält jede Funktion außerdem Mutabilitätsmetadaten für lokale Variablen und die genauen Instruktionsstellen, an denen neue Bindungen initialisiert werden. Der Verifier lehnt nachträglich manipulierte Zuweisungen an immutable Locals ab, auch bei Arrayindex-/Append-, Tupel-, `parallel`-, Schleifen- und Match-Bindungen. Der Compiler prüft diese Stellen zusätzlich gegen die jeweiligen Deklarationen. Innerhalb der unterstützten Bytecode-Teilmenge können Closures über typisierte Funktionsparameter weitergereicht, zurückgegeben und aufgerufen werden. Der bestehende Inline-Lambda-Pfad bleibt unverändert. Nicht unterstützte Operationen im Lambda-Körper, dynamische JSON-Feldzugriffe und nicht unterstützte Transaktionseffekte bleiben abgelehnt.
`E-MIR-BYTECODE-001` meldet nicht unterstützte Operationen. Der Compiler
erstellt nur die erreichbare Aufrufmenge; nicht erreichbarer, nicht
unterstützter Code blockiert daher keinen ausgewählten Einstieg.

## Parallele `await`-Blöcke

Bytecode v19 senkt geprüfte Bindungen der Form
`parallel { name = await ausdruck }` in getrennte, verifizierte Branch-
Funktionen ab und führt sie in gescopten Worker-Threads aus. Jeder Zweig erhält
einen Snapshot der gelesenen lokalen Variablen. Ergebnisse werden erst nach
Erfolg aller Zweige im umgebenden Scope sichtbar; ein vorhandenes
Zuweisungsziel muss initialisiert sein. Die Ausgabe wird in
Deklarationsreihenfolge gesammelt. Alle Zweige teilen ein atomares
`--max-steps`-Budget, die deklarierten Capabilities des Aufrufers, ausdrückliche
Projektfreigaben, Runtime-Policies und denselben injizierten Hostprovider.
Hostaufruf-Vorprüfung und Policy-Kontrollen laufen innerhalb jedes Zweigs vor
dem Providerzugriff. Ein abgewiesener Zweig oder ein überschrittenes Budget
bricht den gesamten Block ab; Budgetfehler verweisen auf den Parallelblock.
Ein Block darf höchstens vier Zweige enthalten. Type Checker, MIR- und
Bytecode-Verifier sowie Runtime erzwingen dieselbe Grenze, bevor Worker
gestartet werden; sie begrenzt die reservierten Worker-Stacks.
Erstklassige oder entkommende Closures kommen dadurch nicht hinzu; die
experimentelle VM ist weiterhin nicht funktionsvollständig.

## SQL-Ausdrücke mit explizitem Datenbankkontext

Der öffentliche Bytecode-Einstieg `execute_with_database_and_host_services_and_policy_captured`
nimmt `BytecodeDatabaseExecution { url, schema }` entgegen. `schema` ist das
geprüfte Projektschema aus `MirProgram::database_schema`; ohne diesen Kontext
scheitert ein SQL-Ausdruck mit einer Laufzeitdiagnose. Datenbankzugriffe
benötigen sowohl die im Funktionskopf deklarierte Capability (zum Beispiel
`Database(read)`) als auch die Projektfreigabe. Vor Providerzugriff prüft die
VM den SQL-Ausdruck erneut gegen das Schema und die tatsächlichen Parametertypen
und führt die read-only Speicherprüfung aus. Der Bytecode-CLI-Pfad nutzt diesen
Einstieg bei gesetzter `DATABASE_URL`.

`sql_decode<T>(sql<T> { SELECT ... })` verwendet denselben nativen
Zeilendecoder wie AST und MIR. Er prüft Spaltennamen, native Spaltenfamilien,
Nullwerte, skalare/Record-Projektionen und Ergebniskardinalität, bevor typisierte
Zelyra-Werte zurückgegeben werden. Tabellenbasierte Row-Records liegen in MIR
und serialisiertem Bytecode als SQL-Projektionsmetadaten vor; sie werden dadurch
nicht als gewöhnliche Quell-Records konstruierbar. Der Compiler verlangt derzeit einen direkten
`SELECT`-Ausdruck; Bytecode bleibt eine experimentelle Teilmenge.

```rust
let database = BytecodeDatabaseExecution {
    url: &database_url,
    schema: &mir.database_schema,
};
let outcome = execute_with_database_and_host_services_and_policy_captured(
    &bytecode, arguments, limits, Some(&grants), policy.as_ref(), database, host,
);
```

Bytecode v21 unterstützt Transaktionen mit geprüften SQL-Anweisungen und
`mail_queue`-Aufrufen mit optionalen PDF-Anhängen. Der Verifier prüft SQL-Capabilities,
Ergebnistypen und Parameternamen. Bei Outbox-Einträgen prüft die VM `Email` und
`Database(write)` vor der Auswertung der Nachrichtenfelder, validiert die
Nachricht und übergibt den Outbox-Eintrag mit SQL in einem Transaktions-Batch.
Die VM versendet keine E-Mail. Andere Transaktionseffekte bleiben nicht
unterstützt.

## Rust-API-Skizze

```rust,ignore
let mir = zelyra_mir::lower(&ast)?;
let bytecode = zelyra_mir::bytecode::compile(&mir, "calculate")?;
let result = zelyra_mir::bytecode::execute(
    &bytecode,
    vec![zelyra_runtime::Value::Int(40)],
    zelyra_mir::execution::ExecutionLimits::default(),
)?;
assert_eq!(result.value, zelyra_runtime::Value::Int(42));
```

Die konkrete API steht in `mir/src/bytecode.rs`; die Tests sind ausführbare
Beispiele der unterstützten Syntax. Ein geprüfter Ausdruck wie `values |> filter(|n| n > minimum) |> map(|n| n * factor)` wird als Schleife mit Zugriff auf eingefangene lokale Werte abgesenkt; dieser Inline-Callback wird dabei kein Funktionswert. Eine gespeicherte oder zurückgegebene Lambda wird mit typisiertem Capture-Layout materialisiert und kann über einen Funktionswert aufgerufen werden. AST/MIR/Bytecode-Differentialtests prüfen mehrere unabhängige Closures nach Encode/Decode. Benannte Funktionsreferenzen wie `&double`, veränderliche oder unveränderliche lokale Callback-Variablen und Parameter vom Typ `Function<(Int) -> Int>` funktionieren als `map`/`filter`/`find`/`reduce`-Callbacks. Dynamische Aufrufe sind auf Funktionen im geprüften Bytecode-Programm begrenzt. Für Hostzugriffe gibt es
`execute_with_host_services`; sie verlangt explizite Projektfreigaben und einen
Provider. Die Übersetzung beginnt bei geprüfter MIR
und verwendet den vorhandenen MIR-Verifier. Vor der Ausführung prüft der
Bytecode-Verifier Kontrollfluss und Stackstruktur; Laufzeitprüfungen sichern
Werttypen, Funktionsargumente, Rückgabewerte, deklarierte Capabilities und
Hostfreigaben.

## Format- und Sicherheitsgrenzen

Bytecode v26 verwendet explizite Quellschrittmarkierungen für das
benutzerseitige `--max-steps`-Budget. VM-Befehle bestimmen damit nicht mehr
die Sprachschritt-Einheit. Die getesteten Grenzen für Anweisungen, Ausdrücke, While-/Collections-
Schleifen, Match und If-Ausdrücke stimmen einschließlich Fehlerpositionen
zwischen AST und MIR überein; Aufrufe und Funktionsvor-/Nachbedingungen sind
durch weitere Budgetläufe abgedeckt. Reine SQL-Transaktionsblöcke und parallele `await`-Bindings haben eigene Grenzwertläufe; der SQL-Test prüft außerdem, dass bei erschöpftem Budget kein Datenbank-Host-Batch startet. Gemischte Transaktionseffekte, parallele Hosteffekte und weitere Sprachkonstrukte benötigen noch umfassendere Grenzwerttests. Für
fehlerhafte oder handgebaute Artefakte begrenzt die VM die Ausführung zusätzlich
auf 10.000.000 Befehle.

`BYTECODE_VERSION` bezeichnet Version `26` des deterministischen Binärformats.
Version 26 ergänzt Quellschrittmarkierungen. Version 25 ergänzte die
verifizierte mutable Array-Pop-Instruktion und typisierte strukturelle
Array-/Tuple-/Option-/Result-Gleichheitsoperationen.
Version 24 serialisiert die Namen tenant-gescopter Tabellen. Verifier und VM
weisen rohes SQL auf solchen Tabellen ab; bei Datenbankausführung muss diese
Liste mit dem aus dem geprüften Projekt stammenden Schema übereinstimmen.
Typisierte Tenant-Queries werden noch nicht unterstützt.
Funktionsreferenzen und unterstützte Capturing-Lambdas können als typisierte
Laufzeitwerte weitergereicht und in `map`, `filter`, `find` und `reduce`
dynamisch aufgerufen werden. Die VM prüft Zielsignatur, Capture-Layout und
Capabilities beim Aufruf erneut. Die unterstützten Capture-Werte sind
Snapshots; beliebige AST-Operationen sind weiterhin nicht Teil der
Bytecode-Teilmenge.
`encode` und `decode` sind als Rust-API verfügbar; `decode_with_limits` erlaubt
explizite Grenzen. Standardmäßig gelten 16 MiB Artefaktgröße, 10.000 Funktionen,
1.000.000 Instruktionen, 1 MiB pro Zeichenkette, 64 Ebenen Typverschachtelung
und 100.000 Typknoten. Kodierung und Laden führen den strukturellen Bytecode-
Verifier aus; der Decoder lehnt unbekannte Tags, ungültige UTF-8-Daten,
abgeschnittene oder zusätzliche Bytes und ungültige Kontrollflussstrukturen ab.
Version 13 ergänzte Enum-Payloads und serialisierte Match-Muster. Version 14
unterstützt Inline-Lambdas bei den vier Array-Transformationen sowie typgeprüfte
intern verwendete Array- und Option-Instruktionen. Version 15 ergänzt
typgeprüfte Record-Kopierupdates. Version 17 ergänzt verifizierte Metadaten für parallele Branches und serialisiert
geprüfte Record-, Alias- und Enumdefinitionen sowie typisierte `http_json`- und
`http_result`-Aufrufe. Frühere Versionen ergänzten
die Vorabprüfung von Hostaufrufen vor der Argumentauswertung. Dieses Format
enthält außerdem die oben beschriebene Teilmenge einschließlich der
Capability-geprüften Uhr-, Umgebungs-, Konsolen- und Zufallszugriffe. Es gibt weiterhin keinen
persistenten Cache, keinen Ablauf zum Laden gespeicherter Artefakte und keine
automatische Artefaktverteilung. Der CLI-Schalter kompiliert das geprüfte
Projekt bei jedem Lauf; extern bereitgestellte Bytecode-Artefakte werden nicht
geladen. Das Format ist nicht signiert; ein erfolgreicher
Strukturcheck beweist weder Herkunft noch vollständige statische Typ-, Effekt-
oder Vertragsprüfung. Runtime-Ressourcenlimits und Laufzeitprüfungen bleiben
erforderlich.

Das Backend ist experimentell und belegt weder vollständige AST-/MIR-Parität
noch einen Leistungsvorteil. Für die beschriebenen Clock-, Environment-,
Console-, Random-, Process-, FileSystem-, Network- und Email-Operationen gibt
es unterschiedlich weit reichende Fake-Host- oder Fixture-Tests; dies belegt
keine vollständige Effektparität. SQL- und Outbox-Transaktionen mit optionalen
PDF-Anhängen werden ab Bytecode v21 unterstützt; andere Transaktionseffekte
bleiben nicht unterstützt. `to_float` wird wie im AST-Interpreter
über den gemeinsamen reinen Built-in-Pfad ausgeführt: endliche numerische Werte
und parsebare endliche Zahlenstrings werden akzeptiert; ungültige Werte und
nicht endliche Ergebnisse liefern dieselben Laufzeitfehler. Differentialtests decken ausgewählte skalare
Aufrufe, Verzweigungen, Kurzschlussauswertung, skalare `while`- und `loop`-
Kontrollflüsse mit `break` und `continue`, auf- und absteigende sowie leere
Integer-Range-`for`-Schleifen, Array- und Set-Iteration (auch leere Collections),
tupelgebundene Collection-Schleifen, Inline-Lambda-`map`/`filter`/`find`/`reduce`
mit lexikalischen Captures, leeren Eingaben und statischen Funktionsreferenzen, Array-/Tupelkonstruktion und -indexierung, Tupelzuweisung, veränderliche Array-Ersetzung
und `+=`, gemeinsame reine Built-ins (Option/Result-Konstruktoren, Collections, Range, `to_float`, Text, CSV, Festkomma-Geldfunktionen, Mathematik und Datum), serialisierte Ausführung, quellpositionsgebundene Indexfehler,
Schritt-null-Fehler, Schritt- und Aufruftiefenlimits, fail-closed-Behandlung
von Effekten und fehlerhaften Tupelwerten, parallele `await`-Zweige mit
Snapshot-Captures, geordneter Ausgabe, Capability-Verweigerung, gemeinsamem
Schrittbudget und Encode/Decode-Prüfung sowie strukturell fehlerhaften Bytecode
ab. Funktionsvorbedingungen (`requires`) werden nach dem Binden der
Argumente ausgeführt; Nachbedingungen (`ensures`) laufen bei jeder Rückkehr
mit dem Rückgabewert in `result`. Die Prüfungen verbrauchen das normale
Schrittbudget. Sie erzwingen Laufzeitbedingungen, sind aber keine formalen
Beweise. Der AST-Interpreter bleibt die semantische Referenz. Differentialtests decken Encode/Decode für Enum-Payloads, Option/Result-Konstruktoren, Record-Destrukturierung, verschachtelte Enum-/Option-Payloads, Literale und Wildcards sowie erfolgreiche und abgelehnte Guards ab.
