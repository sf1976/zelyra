# Zelyra 0.1 — Phase 10: Typisierte API-Deklarationen und OpenAPI

[English](phase-10.md) · Deutsch

Phase 10 beginnt die API-Schicht mit einer typisierten Deklaration, die ohne
laufenden Server geprüft werden kann:

~~~zelyra
type CustomerId = Id

api GET "/customers/{id}" {
    input {
        id: CustomerId
    }
    output Customer
    errors {
        403 Forbidden
        404 NotFound
    }
}
~~~

Die Deklaration beschreibt HTTP-Methode, Route, Eingabetypen, Antworttyp und
dokumentierte Fehlerstatus. HTTP-Methoden werden in Großbuchstaben
normalisiert. Die erste Implementierung unterstützt `GET`, `POST`, `PUT`,
`PATCH` und `DELETE`.

`zelyra check` prüft API-Deklarationen. Abgelehnt werden doppelte Routen,
unbekannte Ein- oder Ausgabetypen, doppelte Eingaben oder Fehlerstatus,
fehlerhafte Pfadparameter sowie Pfadparameter, die nicht im `input`-Block
deklariert sind. Ein Pfadparameter wie `{id}` benötigt also ein Eingabefeld
mit dem Namen `id`.

OpenAPI-3.0.3-Dokument erzeugen:

~~~bash
zelyra doc examples/api.zyl --openapi > openapi.json
~~~

Das erzeugte Dokument enthält Pfadoperationen, Pfadparameter,
Query-Parameter für `GET` und `DELETE`, JSON-Request-Bodies für die übrigen
Methoden, typisierte Erfolgsantworten, deklarierte Fehlerantworten sowie
grundlegende Schemas für deklarierte Typen und Tabellen. `zelyra doc
file.zyl` entspricht der ausdrücklichen Variante mit `--openapi`.

Diese Phase bildet bewusst zunächst die Grenze zwischen Deklaration und
Dokumentation. Ausführbare Handler, Request-Decoding zur Laufzeit und die
Serialisierung von Datenbankzeilen sind noch nicht angeschlossen. Die späteren
Web/API-Phasen werden dasselbe API-Modell dafür verwenden.
