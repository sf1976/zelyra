# Zelyra-0.1-Spezifikationsindex

Die Implementierungsspezifikation des Repositorys verteilt sich auf
Phasendokumente, das zweisprachige Handbuch und die Roadmap. Diese Datei legt
den querschnittlichen KI-nativen Vertrag fest.

## KI-native, KI-unabhängige Entwicklung

> Die KI schreibt. Zelyra prüft.

Menschen und KI-Systeme dürfen Code schreiben, aber keiner von beiden wird
automatisch als vertrauenswürdig behandelt. Compiler und Tests entscheiden,
ob Quellcode gültig ist. Die Sprache bleibt ohne Modell, Anbieter, Cloud oder
Netzwerk vollständig nutzbar.

KI-Schnittstellen sind offen, deterministisch, maschinenlesbar, versioniert,
lokal nutzbar und herstellerneutral. Die aktuell verfügbaren
maschinenorientierten Schnittstellen sind:

```text
zelyra check <file.zyl> --format=json
zelyra context <file.zyl> --format=json
zelyra impact <file.zyl> --format=json
zelyra impact <file.zyl> --symbol <kind:name> --format=json
zelyra edit --format=json change.json
zelyra fmt <file.zyl> --check
```

Die JSON-Schnittstellen verwenden Maschinen-Schema-Version `1`. JSON geht ausschließlich auf
stdout, Logs auf stderr. Diagnosen besitzen stabile Codes und halb-offene
Source-Spans mit nullbasierten UTF-8-Byte-Offsets sowie einsbasierten
Zeilen-/Byte-Spalten. Ausgaben sind deterministisch und enthalten keine
Secrets, Zeitstempel, Zufalls-IDs, absoluten Pfade oder Live-Datenbankinhalte.

Der Compiler muss weiterhin Namen, Typen, Nullability, SQL, Schemata,
Formulare, Views, APIs, Berechtigungen, Contracts, Capabilities, Tests und
Freigaben destruktiver Änderungen prüfen. KI-Code darf Capabilities nicht
unbemerkt ergänzen, Berechtigungen nicht erweitern, kein destruktives SQL
ausführen, Prüfungen nicht abschwächen und keine Secrets ausgeben. Expression-
Typed-Holes, quelltextbasierte Wirkungsanalyse mit optionaler Fokussierung und
versionierte Vorschauen für semantische Umbenennungen sind als erste Stufen
implementiert. Typed Holes in
Deklarationskontexten, vollständige Laufzeit-/Schema-Wirkungsanalyse,
umfangreichere Edit-Operationen, feinere Effekte und Benchmarks bleiben geplant.

## Typisiertes Laden von Seitendaten

Seiten können einen oder mehrere explizite Datensätze über die native
SQL-Grenze laden:

~~~zelyra
page "/customers/{name}" {
    load customer = sql<Customer> {
        SELECT id, name FROM customers WHERE name = :name
    }
    html { <h1>{customer.name}</h1> }
}
~~~

Collection-Ergebnisse verwenden einen Array-Ergebnistyp und eine typisierte
serverseitige Schleife:

~~~zelyra
page "/customers" {
    load customers = sql<Customer[]> { SELECT id, name FROM customers }
    html { <ul>for customer in customers { <li>{customer.name}</li> }</ul> }
}
~~~

Der Compiler prüft die Abfrage gegen das deklarierte Schema, stellt
Routenparameter als typisierte SQL-Parameter bereit und validiert
Record-Feldinterpolationen. Zur Laufzeit werden Autorisierung und die
`Database`-Capability vor der Abfrage geprüft. Ergebnisse werden mit
HTML-Escaping gerendert; fehlende Pflichtdatensätze und Datenbankfehler
erzeugen generische HTTP-Grenzen, ohne Datenbankdetails preiszugeben.
Collection-Schleifen für Arrays von Records sind verfügbar. Option-aware
Feld-Ausdrücke und reichere lokale View-Daten bleiben geplant.

Seiten können typisierte Query-Eingaben deklarieren:

~~~zelyra
page "/customers" {
    input { search: String? }
    load customers = sql<Customer[]> {
        SELECT id, name FROM customers
        WHERE (:search IS NULL OR name LIKE CONCAT('%', :search, '%'))
    }
    html { <p>{search}</p> }
}
~~~

Query-Eingaben stehen dem SQL der Seite und HTML-Interpolationen zur
Verfügung. Der Compiler prüft ihren deklarierten skalaren Typ; die Laufzeit
bindet dekodierte URL-Werte als Datenbankparameter. Fehlende optionale Werte
werden als SQL `NULL` gebunden; fehlende Pflichtwerte und ungültige skalare
Werte erzeugen HTTP 400. Automatische Steuerungen für Suche, Filter,
Sortierung und Pagination beliebiger Seiten gehören noch nicht zu diesem
Feature.

Collection-Seiten können serverseitige Pagination aktivieren:

~~~zelyra
page "/customers" {
    paginated 25
    load customers = sql<Customer[]> { SELECT id, name FROM customers }
    html { <p>{page}</p> }
}
~~~

Die Größe nach `paginated` muss zwischen 1 und 100 liegen. Der optionale
URL-Wert `page` ist eine positive Ganzzahl mit dem Standardwert `1` und steht
im Seiten-HTML als `UInt` zur Verfügung. Die Laufzeit wendet einen
parametrisierten `LIMIT`-/`OFFSET`-Wrapper auf Collection-Abfragen an.
Pagination für beliebige Einzelabfragen weist der Compiler zurück.

Collection-Seiten können außerdem eine Sortier-Whitelist deklarieren:

~~~zelyra
page "/customers" {
    sort { name }
    load customers = sql<Customer[]> { SELECT id, name FROM customers }
    html { <p>{sort} {order}</p> }
}
~~~

Die Laufzeit akzeptiert für `sort` nur deklarierte Ergebnisfelder und für
`order` nur `asc` oder `desc`. Identifier werden nach der Compilerprüfung
quotiert; URL-Werte werden niemals als ungeprüfte SQL-Identifier in SQL
konkateniert. Sortierung und Pagination können kombiniert werden.

Collection-Seiten können außerdem durchsuchbare Ergebnisfelder deklarieren:

~~~zelyra
page "/customers" {
    search { name email }
    load customers = sql<Customer[]> { SELECT id, name, email FROM customers }
    html { <p>{search}</p> }
}
~~~

Der URL-Wert `search` wird als Parameter gebunden und mit serverseitigen
`LIKE`-Bedingungen auf die deklarierten Felder angewendet. Ein leerer
Suchwert fügt keine Bedingung hinzu; unbekannte Suchfelder sind Compilerfehler.

Collection-Seiten können außerdem typisierte Filter deklarieren:

~~~zelyra
page "/customers" {
    filter { name quantity }
    load customers = sql<Customer[]> { SELECT id, name, quantity FROM customers }
    html { <p>{filter_name}</p> }
}
~~~

Der Compiler prüft jedes Filterfeld gegen den Ergebnistyp der Collection. Für
Textfelder gelten `eq`, `contains`, `starts_with`, `ends_with` und
Nullprüfungen; numerische Felder unterstützen zusätzlich `gt`, `gte`, `lt`
und `lte`; Boolean- und andere Werte unterstützen Gleichheit und
Nullprüfungen. Werte werden als Parameter gebunden, Feldnamen und Operatoren
gegen die Deklaration geprüft. Beispiele sind
`/customers?filter_name__contains=Acme` und
`/customers?filter_quantity__gte=10`. Nicht unterstützte Operatoren und nicht
deklarierte Felder erzeugen eine kontrollierte HTTP-400-Antwort.
