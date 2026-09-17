# Zelyra 0.1

![Zelyra-Logo](assets/zelyra-logo.png)

**Von der Datenbank zur Anwendung.**
**Absicht beschreiben. Korrektheit beweisen.**

Deutsch · [English](README.md)

Zelyra ist eine statisch typisierte Programmiersprache für Business-,
Datenbank- und Webanwendungen. Die Sprache folgt einer einfachen Idee:
Informationen, die ein Fachobjekt beschreiben, sollen in der gesamten
Anwendung wiederverwendbar sein.

Eine Tabellendefinition soll die Grundlage für Typen, SQL-Prüfung,
Validierung, Formulare, APIs und CRUD bilden können. Gleichzeitig bleiben
normale Programmierung, natives SQL und eigene Geschäftslogik jederzeit
möglich.

## Aktueller Stand

Zelyra 0.1 ist eine aktive frühe Implementierung. Das Repository enthält echten,
kompilierbaren und getesteten Rust-Code. Die vollständige langfristige
Sprachspezifikation ist jedoch noch nicht vollständig umgesetzt.

## Lizenz und Implementierung

Zelyra ist in Rust implementiert. Rust ist die Implementierungssprache;
Zelyra ist kein offizielles Rust-Projekt und verwendet den Namen oder das
Logo von Rust nicht als Produktkennzeichen.

Der Zelyra-Quellcode steht nach Wahl des Lizenznehmers unter der MIT-Lizenz
oder der Apache License, Version 2.0. Siehe [LICENSE-MIT](LICENSE-MIT) und
[LICENSE](LICENSE). Hinweise zu Drittanbieter-Abhängigkeiten stehen in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

Heute implementiert:

- Sprachkern mit Variablen, Funktionen, Ausdrücken, Kontrollfluss und
  unveränderlichen Variablen als Standard;
- statische Typprüfung, nominale Typen, Option, Result und Pattern Matching;
- Schemadefinitionen und Schema-DDL-Planung für MariaDB, SQLite und
  PostgreSQL;
- MariaDB-Inspektion, Schema-Anwendung und Runtime-Ausführung von nativem SQL;
- native SQL-Blöcke mit Prüfung von Schema, Spalten, Parametern und Ergebnissen;
- ein erster Web Core mit Seitendefinitionen, GET-Routing, Pfadparametern und
  eingebautem HTTP-Server;
- ein erster Forms Core mit schemaabhängigen Feldern und Validierung;
- validierte Formularaktionen mit sicherer MariaDB-Parameterbindung,
  Transaktionen und HTTP-Weiterleitungen.
- eine erste CRUD-Ressource mit MariaDB-Suche, konfigurierbaren Listen-,
  Such- und Filterspalten, Sortierung, Pagination, erzeugten Create-/Edit-
  Formularen und CSRF-geschütztem Löschen.
- erste Authentifizierungssperren, Argon2-Login gegen eine MariaDB-
  Benutzertabelle, HttpOnly-Sessions, Logout und Berechtigungsprüfungen.
- erste Capability-Deklarationen und statische Weitergabe über
  Funktionsaufrufe; natives SQL benötigt `Database`, mit Projektfreigaben aus
  `zelyra.toml`.
- erste zur Laufzeit geprüfte Funktions-Contracts mit `requires` und `ensures`;
  diese Prüfungen werden nicht als formale Beweise ausgegeben.
- ein erster `zelyra verify`-Befehl, der Contract-Ausdrücke zwischen `PROVEN`,
  `RUNTIME_CHECK`, `UNPROVEN` und `FAILED` unterscheidet, einschließlich
  einfacher symbolischer Integer-Beziehungen bei direkten Rückgaben,
  grundlegenden Kontrollflusspfaden, `Option`-/`Result`-Konstruktorpfaden,
  bekannten Payload-Bindings, begrenzten pfadsensitiven Zusammenfassungen von
  Funktionsaufrufen, lokalen Bindings mit einfachen linearen Zuweisungen,
  begrenzten Schleifen sowie modellierten `break`-/`continue`-Pfaden und
  expliziten Schleifeninvarianten mit individuellen Prüfstatus sowie
  aufruferabhängiger Prüfung von Callee-Vorbedingungen. Jedes Ergebnis enthält
  einen stabilen Code, Quellbereich, Erklärung und markierten
  Quellzeilenausschnitt; ein begrenztes Gegenbeispiel wird ausgegeben, wenn es
  sicher gefunden werden kann. `zelyra verify <file.zyl> --json` liefert
  strukturierte Daten mit `message` und `counterexample` für IDEs und CI.

Vollständige CRUD-Erzeugung, Datenbankrollen, APIs, allgemeine formale Verifikation und
Produktionswerkzeuge werden noch entwickelt. Siehe die
[Roadmap](#roadmap) und den ausführlichen
[Getting-Started-Leitfaden](docs/getting-started.de.md).

## Schnelleinstieg

Der einfachste Weg aus einem Quellcode-Checkout:

~~~bash
git clone https://github.com/sf1976/zelyra.git
cd zelyra
./install.sh
zelyra run examples/fibonacci.zyl
~~~

Erwartete Ausgabe:

~~~text
55
~~~

Der Installer baut Zelyra für den aktuellen Benutzer und installiert das
Programm in einem benutzerlokalen bin-Verzeichnis. Er benötigt weder sudo,
eine globale Rust-Installation, Apache noch einen Datenbankserver für die
Sprachkern-Beispiele.

Eine Quelldatei direkt aus dem Repository ausführen:

~~~bash
cargo run -p zelyra-cli -- run examples/fibonacci.zyl
~~~

Ein Programm prüfen, ohne es auszuführen:

~~~bash
zelyra check examples/fibonacci.zyl
~~~

Der vollständige Einsteigerweg mit Fehlerbehebung und Datenbankeinrichtung
steht in [Erste Schritte](docs/getting-started.de.md) oder auf
[Englisch](docs/getting-started.md).

## Eine erste Web-Seite

Zelyra enthält einen kleinen eingebauten HTTP-Server. Apache ist optional und
für den Einstieg nicht erforderlich.

~~~zelyra
page "/hello/{name}" {
    html {
        <html>
            <body>
                <h1>Hello, {name}!</h1>
            </body>
        </html>
    }
}
~~~

Starten:

~~~bash
zelyra serve examples/hello_web.zyl
~~~

http://127.0.0.1:3000/hello/Zelyra öffnen. Routenparameter werden standardmäßig
HTML-escaped. Der aktuelle Web Core unterstützt den ersten sicheren vertikalen
Schritt: GET-Routen, Pfadparameter, Query-String-Verarbeitung, Request-Parsing
und HTML-Responses.

## Ein erstes schemaabhängiges Formular

Formulare können Einschränkungen einer Tabelle wiederverwenden:

~~~zelyra
table customers {
    id: Id primary auto
    name: String(100) required
    email: Email?
}

form CustomerCreate -> customers {
    fields {
        name
        email
    }
}
~~~

Eingaben lokal validieren:

~~~bash
zelyra form validate examples/customer_form.zyl CustomerCreate \
  name=Anna email=anna@example.test
~~~

Das Formular übernimmt das Pflichtfeld name und dessen maximale Länge aus dem
Schema. Unbekannte Felder, fehlende Werte, ungültige E-Mail-Adressen,
ungültige Zahlen, ungültige boolesche Werte und übermittelte Readonly-Felder
werden abgelehnt.

Den Formularserver ohne Apache starten:

~~~bash
zelyra serve examples/customer_form.zyl
~~~

Das Formular ist unter http://127.0.0.1:3000/forms/CustomerCreate erreichbar.
GET rendert die Felder und ein CSRF-Token; POST prüft das Token und validiert
die Eingaben. Ein Formular ohne Aktion dient nur der Validierung.

## Eine erste MariaDB-Formularaktion

`examples/customer_form_action.zyl` zeigt ein vollständiges
datenbankgestütztes Formular:

~~~bash
export DATABASE_URL='mariadb://root:<passwort>@127.0.0.1:3306/zelyra_forms'
zelyra db bootstrap examples/customer_form_action.zyl
zelyra serve examples/customer_form_action.zyl
~~~

Öffne http://127.0.0.1:3000/forms/CustomerCreate. Die POST-Anfrage wird auf
CSRF und Schema-Validierung geprüft, bindet ausschließlich deklarierte
Formularfelder als Datenbankparameter, führt das SQL in einer MariaDB-
Transaktion aus und liefert HTTP 303 zur angegebenen Weiterleitung. Ohne
`DATABASE_URL` wird HTTP 503 geliefert; Datenbankfehler werden kontrolliert
als HTTP 500 ausgegeben, ohne Zugangsdaten oder SQL-Details offenzulegen.
Apache ist nicht erforderlich.

Beziehungsfelder werden automatisch zu Select-Feldern. Das Beispiel
`examples/machine_form.zyl` definiert `department: Department required`.
Zelyra lädt IDs und Anzeigenamen der Abteilungen aus MariaDB, rendert ein
`<select>` und lehnt IDs ab, die in der Datenbank nicht vorhanden sind.

## Datenbankorientierte Entwicklung

Der vorgesehene Zelyra-Ablauf:

~~~text
Datenbankdefinition
        ↓
Schema-Modell
        ↓
Typen und Beziehungen
        ↓
Geprüftes SQL
        ↓
Formulare und Validierung
        ↓
Seiten, APIs und CRUD
~~~

MariaDB ist das Standard-Backend für neue Zelyra-Definitionen und die primäre
Runtime-Referenz. SQLite steht für kleine lokale Anwendungen und Tests zur
Verfügung. PostgreSQL gehört ebenfalls zum Database Core.

Beispiel:

~~~zelyra
database main {
    engine: mariadb
}

table customers {
    id: Id primary auto
    customer_number: String(20) required unique
    name: String(100) required
    email: Email?
    active: Bool default true
}
~~~

Gewünschtes Schema inspizieren oder anwenden:

~~~bash
export DATABASE_URL='mariadb://user:password@127.0.0.1:3306/meine_app'
zelyra db inspect examples/machine_management_mariadb.zyl
zelyra db plan examples/machine_management_mariadb.zyl
zelyra db apply examples/machine_management_mariadb.zyl
~~~

Für SQLite:

~~~bash
export DATABASE_URL='sqlite:///tmp/meine-app.sqlite3'
zelyra db bootstrap examples/machine_management_sqlite.zyl
~~~

Keine echten Zugangsdaten committen. Umgebungsvariablen oder einen
Secret-Manager verwenden. Die Beispiele verwenden zuerst MariaDB, weil dies
das Standard-Backend des Projekts ist.

## Natives SQL

SQL ist ein Sprachelement und kein untypisierter String:

~~~zelyra
customer = sql<Customer?> {
    SELECT id, name, email
    FROM customers
    WHERE id = :id
}
~~~

Wenn das Schema verfügbar ist, prüft Zelyra Tabellen, Spalten, Aliase,
Parameter, NULL-Fähigkeit und Ergebnismappings. Benannte Parameter werden
sicher gebunden. Komplexes SQL bleibt möglich; Zelyra erzwingt keine
ORM-Methodenkette.

## Sprachprinzipien

- Werte sind standardmäßig unveränderlich. Veränderlichkeit wird ausdrücklich
  mit mutable markiert.
- Normale Typen können niemals null sein. Optionale Werte verwenden die
  explizite Option-Schreibweise.
- Fachliche IDs können nominal unterschieden werden. Eine UserId kann daher
  nicht versehentlich als OrderId verwendet werden.
- Funktionen beschreiben Fehler ausdrücklich, statt versteckte Exceptions als
  normale Kontrollsteuerung zu verwenden.
- SQL-Parameter werden sicher gebunden.
- HTML-Ausgaben werden standardmäßig escaped.
- Schemaänderungen werden geprüft; destruktive Änderungen benötigen eine
  ausdrückliche Freigabe.
- Capabilities sind teilweise implementiert: Deklarationen, Prüfung bekannter
  Namen, Weitergabe über Aufrufe und die `Database`-Pflicht für natives SQL
  sind aktiv. Erweiterte Contracts und allgemeine formale Verifikation bleiben
  Roadmap-Ziele.

## CLI

Aktuell verfügbar:

~~~text
zelyra new <directory>
zelyra init [directory]
zelyra check <file.zyl>
zelyra build <file.zyl>
zelyra run <file.zyl>
zelyra serve <file.zyl> [address]
zelyra verify <file.zyl> [--json]
zelyra form validate <file.zyl> <FormName> [field=value ...]
zelyra db create <file.zyl>
zelyra db bootstrap <file.zyl>
zelyra db inspect <file.zyl>
zelyra db plan <file.zyl>
zelyra db apply <file.zyl> [--allow-destructive]
~~~

Die Befehle sind bewusst klein und ausdrücklich. Apache, PHP, ein ORM und ein
Frontend-Framework sind für die obigen Beispiele keine Voraussetzungen.

## Repository-Struktur

~~~text
zelyra/
├── ast/          Abstract Syntax Tree und Sprachdatenmodell
├── lexer/        Tokenisierung von Quelle, SQL und HTML
├── parser/       Parser für die Zelyra-Syntax
├── hir/          Namensauflösung und High-Level-IR
├── database/     Schema-Modell, SQL-Prüfung und Datenbank-Backends
├── forms/        Schemaabhängige Formularprüfung und Validierung
├── web/          Router, HTTP-Modell, Escaping und Server
├── runtime/      Typprüfung und Interpreter
├── cli/          Kommandozeilenprogramm zelyra
├── examples/     Kleine ausführbare Beispiele
├── docs/         Deutsche und englische Dokumentation
└── tests/        Crate-übergreifende Akzeptanztests
~~~

Der Bootstrap-Compiler wird in Rust entwickelt und als Cargo-Workspace gebaut.

## Roadmap

Die langfristige Spezifikation ist in folgende Phasen gegliedert:

1. Language Core — implementierte Grundlage.
2. Type System — erste Implementierung vorhanden.
3. Database Core — erste Unterstützung für MariaDB, SQLite und PostgreSQL.
4. Native SQL — statische Prüfung und MariaDB-Ausführung vorhanden.
5. Web Core — erste Seiten und HTTP-Server vorhanden.
6. Forms — schemaabhängige Syntax, Validierung, Web-Rendering, Aktionen und
   Beziehungs-Selects vorhanden.
7. CRUD — Liste, Details, Erstellen, Bearbeiten, Suche, Filter, Sortierung,
   Pagination, konfigurierbare Spalten und CSRF-geschütztes Löschen vorhanden;
   Autorisierung folgt.
8. Authentifizierung und Autorisierung — Argon2-Login, persistente MariaDB-
   Sessions, Logout, Routensperren und datenbankgestützte
   Berechtigungsabfragen vorhanden.
9. Capabilities und Contracts — erste Deklarationen, statische Prüfungen,
   Runtime-Contracts und begrenzte symbolische Verifikation sind vorhanden;
   Runtime-Rechte, allgemeine formale Verifikation und strukturierte
   Nebenläufigkeit folgen.
10. APIs, OpenAPI, Client State, WebAssembly und Optimierungsschnittstellen.

Jedes Feature soll Syntax, AST/HIR-Unterstützung, Diagnosen, positive und
negative Tests, Dokumentation und Beispiele enthalten.

## Mitwirken

Das Repository wird bewusst in kleinen, testbaren Phasen entwickelt. Vor
Änderungen:

~~~bash
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
~~~

Deutsche und englische Benutzerdokumentation sollen synchron bleiben.
Architekturentscheidungen sollen Sicherheit, Kontrolle und Erweiterbarkeit
erhalten.

## Lizenz

Zelyra wird unter der MIT-Lizenz veröffentlicht. Siehe [LICENSE](LICENSE).
