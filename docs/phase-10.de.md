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

Einen TypeScript-Client ohne zusätzliche Abhängigkeiten aus denselben
Deklarationen erzeugen:

~~~bash
zelyra doc examples/api_records.zyl --typescript > customer-client.ts
~~~

Der erzeugte Client enthält TypeScript-Aliase und Interfaces für Zelyra-Typen,
Records und Tabellen sowie einen `ZelyraClient` auf Basis der standardmäßigen
`fetch`-API. Pfadkodierung, Query-Parameter, JSON-Request-Bodies, Bearer-
Tokens, typisierte Response-Promises und strukturierte HTTP-Fehler werden
behandelt. Deklarierte API-Fehlernamen werden als `ZelyraApiErrorCode`
erzeugt; `ZelyraApiError.fromResponse` liest HTTP-Status, Fehlercode und
Servermeldung aus und behält zusätzlich den unveränderten Response-Body für
die Diagnose.

Das Array-Beispiel kann ohne Datenbank geprüft und gestartet werden:

~~~bash
zelyra check examples/api_arrays.zyl
zelyra serve examples/api_arrays.zyl
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

Wenn ein Handler `Err("NotFound")` zurückgibt und `NotFound` mit einem Status
deklariert ist, gibt die Laufzeit diesen Status als strukturierten JSON-Fehler
zurück. Ein nicht deklarierter Fehler wird niemals erraten und führt zu einer
500-Antwort.

Der Handler deklariert dafür ein `Result`, wenn er einen dieser fachlichen
Fehler zurückgeben kann:

~~~zelyra
api GET "/customers/{id}" {
    handler find_customer
    input { id: CustomerId }
    output Result<Customer, String>
    errors { 404 NotFound }
}
~~~

Typisierte Arrays werden für API-Eingaben und -Ausgaben unterstützt:

~~~zelyra
api POST "/customer-ids" {
    handler echo_ids
    input { ids: CustomerId[] }
    output CustomerId[]
}
~~~

Array-Literale und die ersten allgemeinen Array-Operationen sind im
Sprachkern verfügbar:

~~~zelyra
numbers = [1, 2, 3]
first = numbers[0]
count = len(numbers)
extended = append(numbers, 4)
combined = numbers + [5, 6]
has_two = contains(numbers, 2)
first = first(numbers)
last = last(numbers)
~~~

`first` und `last` liefern `Option<T`; leere Arrays werden dadurch ohne einen
Laufzeit-Nullwert sicher behandelt.

Maps mit String-Schlüsseln überqueren die API-Grenze als JSON-Objekte. Dieselbe
Typdeklaration steuert Request-Validierung, Response-Serialisierung, OpenAPI
und den erzeugten TypeScript-Client:

~~~zelyra
fn echo_settings(settings: Map<String, Int>) -> Map<String, Int> {
    return settings
}

api POST "/settings" {
    handler echo_settings
    input { settings: Map<String, Int> }
    output Map<String, Int>
}
~~~

`zelyra check examples/api_maps.zyl` prüft die Deklaration. Das OpenAPI-Schema
beschreibt ein Objekt mit ganzzahligen `additionalProperties`, und der
TypeScript-Client verwendet `Record<string, number>`. `Map<Int, String>` ist im
Sprachkern gültig, wird an einer API-Grenze jedoch vom Compiler abgelehnt, weil
JSON-Objektschlüssel Zeichenketten sind.

Arrays können mit strukturierten `for ... in`-Schleifen durchlaufen werden. Die
Schleifenvariable ist unveränderlich und nur im Schleifenkörper sichtbar:

~~~zelyra
for customer in customers {
    print(customer.name)
}
~~~

`break` und `continue` funktionieren innerhalb von Array-Schleifen wie in den
anderen Schleifen.

Records bilden das deklarierte Modell für verschachtelte JSON-Objekte.
Fehlende optionale Felder werden zu `None`; unbekannte Felder und fehlende
Pflichtfelder werden an der API-Grenze abgelehnt:

~~~zelyra
struct Address {
    city: String
}

struct CustomerInput {
    name: String
    address: Address
}

api POST "/customers" {
    handler create_customer
    input { customer: CustomerInput }
    output CustomerInput
}
~~~

Record-Werte werden sowohl an der JSON-API-Grenze als auch im Sprachkern
unterstützt. Record-Literale und Feldzugriff werden gegen die deklarierte
Record-Definition geprüft.

API-Fehler können nach einem Doppelpunkt optional einen Payload-Typ angeben.
Dieser Typ muss zum Fehlertyp im `Result`-Ausgabetyp des Handlers passen:

~~~zelyra
struct ValidationProblem {
    field: String
    message: String
}

fn validate_customer() -> Result<String, ValidationProblem> {
    return Err(ValidationProblem {
        field: "email"
        message: "invalid address"
    })
}

api POST "/customers/validate" {
    handler validate_customer
    output Result<String, ValidationProblem>
    errors {
        422 ValidationError: ValidationProblem
    }
}
~~~

Die Laufzeit behält die stabilen Felder `code` und `message` und ergänzt den
typisierten Wert unter `error.details`. Ungetypte Deklarationen bleiben
unverändert. OpenAPI beschreibt das Details-Schema; generierte TypeScript-
Clients stellen den Payload über `ZelyraApiErrorPayloads` und
`ZelyraApiError.details` bereit.
