# Zelyra 0.1 — Phase 9: Capabilities

Deutsch · [English](phase-9.md)

Zelyra-Funktionen können die benötigten externen Capabilities ausdrücklich
deklarieren:

~~~zelyra
fn send_invoice(invoice: String)
    uses Network
{
    print(invoice)
}
~~~

Die erste Capability-Menge ist:

~~~text
Database
Network
FileSystem
Environment
Process
Clock
Random
~~~

Der Compiler lehnt unbekannte oder doppelt deklarierte Capability-Namen ab.
Anforderungen werden außerdem über Funktionsaufrufe weitergegeben: Eine
Funktion, die `send_invoice` aufruft, muss selbst `uses Network` deklarieren.

Natives SQL benötigt `Database`:

~~~zelyra
fn load_customer() -> Customer[]
    uses Database
{
    return sql<Customer[]> {
        SELECT id, name FROM customers
    }
}
~~~

Die Deklaration wird statisch geprüft und ist in AST und HIR sichtbar. Ein
Projekt kann Capabilities ausdrücklich in `zelyra.toml` freigeben:

~~~toml
[capabilities]
database = true
network = false
clock = true
environment = true
~~~

Wenn eine Projektdatei vorhanden ist, muss jede deklarierte Capability dort
aktiviert sein. Ein fehlender Eintrag oder der Wert `false` wird von
`zelyra check`, `build`, `run` und `serve` abgelehnt. Einzelne Quelldateien
ohne Projektdatei behalten das Entwicklungsverhalten und prüfen nur die
Deklaration.

Diese Phase vergibt keine Betriebssystemrechte. Capability-Grenzen werden zur Laufzeit dennoch
durchgesetzt: Ein Funktionsaufruf mit Projektfreigaben wird abgewiesen, wenn
die deklarierten Capabilities der Funktion nicht freigegeben sind; natives SQL
wird abgewiesen, wenn die aktuelle Funktion nicht `Database` deklariert. Die
CLI übergibt die Freigaben aus `zelyra.toml` an `run`, `serve` und ausführbare
API-Handler, Formulare, CRUD und datenbankgestützte Authentifizierung.

Zwei sichere Host-APIs sind jetzt verfügbar:

~~~zelyra
fn runtime_timestamp() -> Timestamp uses Clock {
    return now()
}

fn configured_mode() -> String? uses Environment {
    return env("ZELYRA_MODE")
}
~~~

now() benötigt Clock und liefert den aktuellen Unix-Epoch-Zeitstempel in
Millisekunden. env(name) benötigt Environment und liefert None, wenn die
Variable fehlt. Keine der beiden APIs protokolliert oder veröffentlicht Werte
automatisch; eine Ausgabe erfolgt nur, wenn das Programm ausdrücklich print
oder eine andere Anwendungsoperation verwendet. Datei- und Netzwerk-APIs
benötigen eigene Ressourcenrichtlinien.

Die Random-Capability stellt sichere Ganzzahl-Erzeugung bereit:

~~~zelyra
fn dice_roll() -> Int uses Random {
    return random_int(1, 6)
}
~~~

Beide Grenzen sind inklusiv. Die Runtime verwendet die sichere
Zufallsquelle des Betriebssystems und lehnt Bereiche ab, bei denen das
Minimum größer als das Maximum ist. Zufallswerte werden niemals automatisch
protokolliert oder ausgegeben. Prozess-APIs benötigen weiterhin eigene
Ressourcen- und Fehlerverträge.

Die erste Netzwerk-Host-API ist `http_get`:

~~~zelyra
fn load_status(url: String) -> String uses Network {
    return http_get(url)
}
~~~

`http_get` benötigt die Capability `Network` und liefert den UTF-8-Body einer
erfolgreichen HTTP-Anfrage. Für Projekte ist zusätzlich eine ausdrückliche
Host-Allowlist erforderlich:

~~~toml
[network]
allowed_hosts = ["127.0.0.1:8080", "api.example.com"]
timeout_ms = 5000
max_response_bytes = 1048576
~~~

Die Allowlist vergleicht Host oder Host-mit-Port exakt. Ohne Abschnitt
`[network]` sind in einem Projekt keine Hosts erlaubt. Der Transport
unterstützt `http://` und `https://`; die Zertifikatsprüfung über Rustls ist
standardmäßig aktiviert. Es werden keine Redirects verfolgt und Antwortgröße
sowie gesamte Anfragezeit begrenzt. Der Helper `http_get` bleibt die einfache
GET-Komfort-API; für Request-Header, Request-Bodies oder den Response-Status
wird `http_request` verwendet.

Für typisierte Anfragen gibt es `http_request`:

~~~zelyra
fn create_customer(url: String) -> HttpResponse uses Network {
    return http_request(
        "POST",
        url,
        ["Content-Type: application/json"],
        Some("{\"name\":\"Anna\"}")
    )
}
~~~

Die Request-API akzeptiert `GET`, `POST`, `PUT`, `PATCH`, `DELETE` und `HEAD`.
Header werden als Strings im Format `Name: value` übergeben, der Body ist
`String?`. Die zurückgegebene `HttpResponse` besitzt die typisierten Felder
`status: Int`, `headers: String[]` und `body: String`. Redirects bleiben
deaktiviert; GET- und HEAD-Anfragen dürfen keinen Body enthalten.

JSON-Werte können in geprüfte Zelyra-Werte umgewandelt werden und umgekehrt.
Records und verschachtelte Felder werden gegen das deklarierte Schema geprüft;
unbekannte oder fehlende Pflichtfelder sind Laufzeitfehler:

~~~zelyra
struct Customer {
    name: String
    tags: String[]
    nickname: String?
}

fn decode_customer(body: String) -> Customer {
    return json_decode<Customer>(body)
}

fn encode_customer(customer: Customer) -> String {
    return json_encode(customer)
}
~~~

`json_decode<Typ>(text)` benötigt genau ein Zieltypargument. Unterstützt werden
Records, verschachtelte Records, Arrays, Optionen und Skalarwerte.
`json_encode` serialisiert dieselben Werte; eine nicht vorhandene Option wird
zu JSON `null`. Ungültiges JSON, Typfehler, unbekannte Record-Felder und
fehlende Pflichtfelder werden als ausdrückliche Laufzeitfehler gemeldet.

Die Process-Capability stellt eine bewusst enge Befehls-API bereit:

~~~zelyra
fn render_report(input: String) -> String uses Process {
    return run_process("/usr/bin/printf", ["%s", input])
}
~~~

`run_process(command, args)` startet niemals eine Shell. Jedes Argument wird
als separates Betriebssystemargument übergeben. Für Projekte sind eine exakte
Befehls-Allowlist und begrenzte Ressourcen erforderlich:

~~~toml
[process]
allowed_commands = ["/usr/bin/printf"]
timeout_ms = 5000
max_output_bytes = 1048576
~~~

Ohne `[process]` darf kein Befehl laufen. Die Runtime leert die Umgebung des
Kindprozesses, stellt kein stdin bereit, beendet Prozesse nach dem Timeout und
begrenzt stdout und stderr. Exit-Status ungleich null, ungültiges UTF-8 und
Ressourcenverletzungen werden zu ausdrücklichen Runtime-Fehlern.
Shell-Ausführung, beliebige Umgebungsweitergabe, Arbeitsverzeichnisse und
Prozess-Pipelines folgen später.

Die FileSystem-Host-APIs sind:

~~~zelyra
fn read_source(path: String) -> String uses FileSystem {
    return read_text(path)
}

fn write_note(path: String, content: String) uses FileSystem {
    write_text(path, content)
}

fn entries(path: String) -> String[] uses FileSystem {
    return list_dir(path)
}

fn remove_note(path: String) uses FileSystem {
    delete_file(path)
}
~~~

Alle vier APIs benötigen FileSystem. read_text(path) liest UTF-8-Text,
write_text(path, content) erstellt oder ersetzt eine Datei, delete_file(path)
löscht eine Datei und list_dir(path) liefert sortierte Eintragsnamen. Leere
Pfade, fehlende Dateien, fehlende Berechtigungen, als Dateien verwendete
Verzeichnisse und ungültiges UTF-8 werden zu ausdrücklichen Runtime-Fehlern.

Projekte können den Zugriff mit bereits existierenden Verzeichnisgrenzen
beschränken:

~~~toml
[filesystem]
read_roots = ["."]
write_roots = ["data"]
~~~

Relative Pfade werden ausgehend vom Projektverzeichnis aufgelöst. Lesen und
Verzeichnislisten verwenden read_roots; Schreiben und Löschen verwenden
write_roots. Symlink-Ziele werden vor dem Zugriff kanonisiert, und ein neues
Schreibziel benötigt ein bereits existierendes Elternverzeichnis. Ohne
filesystem-Abschnitt sind Projektlesezugriffe auf das Projektverzeichnis
begrenzt; Schreiben und Löschen sind gesperrt.

Der erste Structured-Concurrency-Schnitt ist über `parallel` und `await`
verfügbar:

~~~zelyra
parallel {
    customer = await load_customer()
    orders = await load_orders()
}
~~~

Jeder Zweig muss ein eigenes Ergebnis mit `await` binden. Die Zweige erhalten
eine unveränderliche Momentaufnahme des umgebenden Zustands und werden vor dem
Fortsetzen der Ausführung zusammengeführt. Ergebnisse werden in
Quelltextreihenfolge übernommen; ein Fehler in einem Zweig lässt den gesamten
Block fehlschlagen, nachdem alle Zweige beendet wurden. `await` außerhalb eines
`parallel`-Blocks weist der Type Checker zurück. Diese erste Runtime verwendet
einen Worker-Thread pro Zweig; Abbruch und Integration in den Datenbank-
Connection-Pool bleiben zukünftige Aufgaben.

## Diagnosen

Fehlendes `Database` bei SQL erzeugt beispielsweise:

~~~text
error[E-CAP-001]: function `load_customer` uses SQL but does not declare capability `Database`; add `uses Database`
~~~

Die CLI prüft Capabilities als Teil von `zelyra check`, `zelyra build`,
`zelyra run` und `zelyra serve`.

## Contracts

Funktionen können außerdem zur Laufzeit geprüfte Vor- und Nachbedingungen
deklarieren:

~~~zelyra
fn increment(value: Int) -> Int
    requires { value >= 0 }
    ensures { result > value }
{
    return value + 1
}
~~~

`requires`-Ausdrücke werden vor dem Funktionskörper ausgeführt. `ensures`-
Ausdrücke werden danach ausgewertet; der Rückgabewert steht dort als `result`
zur Verfügung. Beide Ausdrücke müssen den Typ `Bool` besitzen. Eine falsche
Bedingung beendet die Ausführung mit einem Runtime-Fehler. Diese Prüfungen
sind Runtime-Checks und keine mathematischen Beweise. Formale Verifikation und
Beweisausgabe folgen später.

## Verifikationsbefehl

Der erste Verifikationsbefehl ist verfügbar über:

~~~bash
zelyra verify examples/contracts.zyl
~~~

Konstante Bool-Ausdrücke können als `PROVEN` oder `FAILED` klassifiziert
werden. Der erste symbolische Verifier setzt direkte ganzzahlige
Rückgabeausdrücke in `ensures` ein und beweist einfache affine Beziehungen.
Außerdem analysiert er `if`-/`else`-Rückgabepfade und verwendet deren einfache
Integer-Vergleiche als Pfadannahmen.

Beispielsweise kann er `result >= 0` für eine Absolutwertfunktion mit den
beiden Zweigen `return value` und `return -value` beweisen. Vollständige
`match`-Ausdrücke mit Integer- oder Bool-Literal-Mustern und einem Wildcard-
Zweig werden ebenfalls als getrennte Rückgabepfade gesammelt. Constructor-
Muster von `Option` und `Result` wie `Some`, `None`, `Ok` und `Err` tragen
außerdem Konstruktor-Fakten zur Pfadprüfung bei. Nicht entscheidbare
Payload-Bindings werden eingesetzt, wenn der gematchte Wert ein bekannter
Konstruktor ist. Dadurch kann `Some(4)` mit anschließendem `return number` als
`return 4` geprüft werden. Unbekannte Payloads und nicht unterstützte
Binding-Beziehungen bleiben `RUNTIME_CHECK`; Funktionen ohne Contracts werden
als `UNPROVEN` gemeldet.

Einfache Funktionsaufrufe werden ebenfalls zusammengefasst und in das
Integer-Modell eingesetzt. Direkte Rückgaben und pfadsensitive Aufrufe wie
`return absolute(value)` werden mit begrenzter Tiefe verarbeitet; die
Rückgabepfade der Callee liefern dabei eigene Pfadbedingungen. Vor der
Verwendung einer Callee-Zusammenfassung prüft der Verifier die `requires`-
Bedingungen der aufgerufenen Funktion nach Argumentsubstitution. Die
`requires`-Bedingungen des Aufrufers stehen beim Beweis seiner `ensures` als
Annahmen zur Verfügung. Aufrufe komplexer, rekursiver oder nicht auflösbarer
Funktionen sowie nicht beweisbare Aufrufbedingungen bleiben `RUNTIME_CHECK`.

Lokaler Zustandsfluss wird ebenfalls durch den Funktionskörper verfolgt.
Sowohl `next: Int = value + 1` als auch die Kurzform `next = value + 1` mit
anschließendem `return next` können wie die direkte Rückgabe zusammengefasst
werden. Einfache lineare Initialisierung und Zuweisung bei Mutable-Variablen
werden ebenfalls verfolgt, etwa `next = next + 1`. Statisch begrenzte
Schleifen mit linear verändertem Zähler werden pfadweise entfaltet. `break`
beendet dabei die aktuelle Schleife, `continue` startet ihren nächsten
Durchlauf; beide werden als eigene symbolische Kontrollflusspfade modelliert.
Explizite Schleifeninvarianten können direkt an einer `while`- oder `loop`-
Schleife stehen:

~~~zelyra
while current > 0
    invariant { current >= 0 }
{
    current = current - 1
}
~~~

Der Verifier prüft die Invariante beim Schleifeneintritt und nach unterstützten
Körperpfaden. Wenn sie bewiesen ist, kann sie eine ansonsten unbeschränkte
lineare `while`-Schleife zusammenfassen. Eine unbedingte `loop`-Schleife kann
die Invariante zusammen mit einem symbolisch modellierten `break` zum Beweis
ihrer Austrittspfade verwenden. Nichtlineare Zuweisungen, ungültige oder nicht
unterstützte Invarianten und andere nicht unterstützte Zustandsflüsse bleiben
`RUNTIME_CHECK`. Die Runtime prüft die Invariante ebenfalls vor und nach jedem
Durchlauf.

`zelyra verify` meldet jede deklarierte Invariante separat, nach den
`ensures`-Ergebnissen der Funktion. Der Name verwendet den nullbasierten Index
der Invariante, zum Beispiel:

~~~text
PROVEN [V-001]: reduce.ensures[0] (src/reduce.zyl:3:5-3:21)
PROVEN [V-001]: reduce.invariant[0] (src/reduce.zyl:7:21-7:33)
FAILED [V-004]: reduce.invariant[1] (src/reduce.zyl:8:21-8:34)
~~~

Jede Zeile enthält einen stabilen Verifikationscode und einen Quellbereich im
Format `(datei.zyl:startzeile:startspalte-endzeile:endspalte)`, sodass das
Ergebnis direkt im Editor gefunden werden kann. Die Codes sind `V-001` für
`PROVEN`, `V-002` für `RUNTIME_CHECK`, `V-003` für `UNPROVEN` und `V-004` für
`FAILED`.

Die Textausgabe zeigt anschließend eine kurze Erklärung und die betroffene
Quellzeile mit einem Caret-Marker:

~~~text
  = The loop invariant is false on a feasible path or is not preserved by the loop body.
    |
  8 |     invariant { current == value }
    |                ^^^^^^^^^^^^^^^^^^^
~~~

Für IDEs und CI kann eine maschinenlesbare Ausgabe angefordert werden:

~~~bash
zelyra verify examples/contracts.zyl --json
~~~

Das JSON-Ergebnis enthält `status`, `code`, `message`, `function`, `kind`,
`index`, `counterexample` sowie ein `location`-Objekt mit Datei und
Start-/Endposition in Zeile und Spalte. `counterexample` ist ein Objekt, wenn
ein kleines lineares Integer-Gegenbeispiel gefunden wurde, sonst `null`. Die
aktuelle begrenzte Suche verarbeitet höchstens drei lineare Variablen im
Bereich `-32..=32` für fehlgeschlagene Vorbedingungen, Nachbedingungen und
Schleifeninvarianten; kein Gegenbeispiel bedeutet daher nicht, dass keines
existiert.

Das enthaltene negative Beispiel zeigt einen konkreten Zeugen:

~~~bash
zelyra verify examples/counterexample.zyl
~~~

Für den fehlgeschlagenen Postcondition-Contract wird `value = 0` gemeldet. Der
Befehl endet mit einem Fehlerstatus, weil ein Contract widerlegt wurde.

`FAILED` bedeutet, dass die Invariante auf einem möglichen analysierten Pfad
falsch ist oder vom Schleifenkörper nicht erhalten bleibt. `RUNTIME_CHECK`
bedeutet, dass eine Laufzeitprüfung erforderlich ist, weil der aktuelle
symbolische Verifier den Beweis nicht vollständig führen kann. Keiner dieser
Statuswerte außer `PROVEN` ist ein mathematischer Beweis.

Der Befehl endet bei einem fehlgeschlagenen konstanten Contract oder einem
Compilerfehler mit einem Fehlerstatus. Kein Status außer `PROVEN` ist ein
mathematischer Beweis.
