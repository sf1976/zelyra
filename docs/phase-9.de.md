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
~~~

Wenn eine Projektdatei vorhanden ist, muss jede deklarierte Capability dort
aktiviert sein. Ein fehlender Eintrag oder der Wert `false` wird von
`zelyra check`, `build`, `run` und `serve` abgelehnt. Einzelne Quelldateien
ohne Projektdatei behalten das Entwicklungsverhalten und prüfen nur die
Deklaration.

Diese Phase vergibt noch keine Betriebssystemrechte und implementiert noch
keine Netzwerk- oder Datei-APIs. Runtime-Durchsetzung für diese APIs und
capability-bewusste Nebenläufigkeit folgen später.

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

Einfache Funktionsaufrufe mit direkter Rückgabe werden ebenfalls als
Zusammenfassung in das Integer-Modell eingesetzt. Verschachtelte Aufrufe wie
`increment(increment(value))` werden mit begrenzter Tiefe verarbeitet. Vor der
Verwendung einer Callee-Zusammenfassung prüft der Verifier die `requires`-
Bedingungen der aufgerufenen Funktion nach Argumentsubstitution. Die
`requires`-Bedingungen des Aufrufers stehen beim Beweis seiner `ensures` als
Annahmen zur Verfügung. Aufrufe komplexer, rekursiver oder nicht auflösbarer
Funktionen sowie nicht beweisbare Aufrufbedingungen bleiben `RUNTIME_CHECK`.

Der Befehl endet bei einem fehlgeschlagenen konstanten Contract oder einem
Compilerfehler mit einem Fehlerstatus. Kein Status außer `PROVEN` ist ein
mathematischer Beweis.
