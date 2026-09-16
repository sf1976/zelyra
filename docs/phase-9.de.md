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

Die Deklaration wird statisch geprüft und ist in AST und HIR sichtbar. Diese
Phase vergibt noch keine Betriebssystemrechte und implementiert noch keine
Netzwerk- oder Datei-APIs. Runtime-Durchsetzung für diese APIs,
projektweite Freigaben und capability-bewusste Nebenläufigkeit folgen später.

## Diagnosen

Fehlendes `Database` bei SQL erzeugt beispielsweise:

~~~text
error[E-CAP-001]: function `load_customer` uses SQL but does not declare capability `Database`; add `uses Database`
~~~

Die CLI prüft Capabilities als Teil von `zelyra check`, `zelyra build`,
`zelyra run` und `zelyra serve`.
