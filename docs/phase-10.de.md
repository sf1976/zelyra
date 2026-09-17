# Zelyra 0.1 — Phase 10: Typisierte API-Deklarationen und OpenAPI

[English](phase-10.md) · Deutsch

Phase 10 beginnt die API-Schicht mit einer typisierten Deklaration, die ohne
laufenden Server geprüft werden kann:

~~~zelyra
type CustomerId = Id

api GET "/customers/{id}" {
    handler get_customer
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

API-Routen können dieselben Authentifizierungs- und Berechtigungsschutzregeln
wie Seiten und CRUD-Ressourcen verwenden:

~~~zelyra
api DELETE "/customers/{id}" {
    handler delete_customer
    requires auth
    permits "customers.delete"
    input { id: CustomerId }
    output Unit
}
~~~

`requires auth` verlangt eine authentifizierte Session oder ein konfiguriertes
gültiges Bearer-Token. Jede `permits`-Angabe verlangt die genannte
Berechtigung. Eine geschützte API benötigt eine `auth`-Definition im Projekt;
andernfalls weist `zelyra check` das Programm zurück. Fehlgeschlagene
API-Autorisierung wird als JSON mit stabilem `code` und verständlicher
`message` zurückgegeben.

Eine ausführbare Route kann mit `handler` eine Zelyra-Funktion benennen. Ihre
Parameter müssen Namen und Typen der API-Eingabefelder in derselben Reihenfolge
besitzen; der Rückgabetyp muss `output` entsprechen:

~~~zelyra
fn get_customer(id: CustomerId) -> Customer uses Database {
    return sql<Customer> {
        SELECT id, name, email FROM customers WHERE id = :id
    }
}
~~~

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

Ist ein Handler vorhanden, stellt `zelyra serve` die Route bereit. Pfad- und
Querywerte werden entsprechend den deklarierten Eingabetypen konvertiert; für
nicht-GET-Methoden werden JSON-Request-Bodies unterstützt und Handler-Ergebnisse
als JSON zurückgegeben. Datenbank-Handler verwenden `DATABASE_URL`, im
Zelyra-Runtime standardmäßig MariaDB.

Eingabe- und Laufzeitfehler verwenden dieselbe Transportstruktur, zum Beispiel:

~~~json
{"error":{"code":"BadRequest","message":"missing API input `id`"}}
~~~

Der `errors`-Block dokumentiert mögliche HTTP-Antworten in OpenAPI. Die erste
Laufzeitimplementierung leitet aus einem fachlichen Fehlerwert des Handlers
noch keinen anwendungsspezifischen Statuscode ab; eine explizite
Fehlerzuordnung ist geplant.

Die Handler-Brücke bleibt bewusst klein: verschachteltes JSON-Decoding,
generierte Client-Bindings und anwendungsspezifische Fehlerzuordnung folgen in
späteren Web/API-Schritten.
