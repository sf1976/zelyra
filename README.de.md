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

Die gepflegte [Roadmap](docs/ROADMAP.de.md) enthält alle geplanten Pflicht- und
optionalen Arbeiten, einschließlich Views-System, KI-nativer
Compiler-Schnittstellen und kryptografischer Audit-Verkettung.

## KI-native Entwicklung

Zelyras strategische Richtung lautet: **Die KI schreibt. Zelyra prüft.** Ein
Mensch oder ein KI-System darf Zelyra-Code schreiben; der Compiler bleibt aber
maßgeblich für Syntax, Namen, Typen, SQL, Schema, Formulare, Views,
Berechtigungen, Contracts, Capabilities, Tests und die Freigabe destruktiver
Änderungen. Zelyra bleibt vollständig ohne KI-Anbieter oder Cloud-Dienst
nutzbar.

Die ersten maschinenorientierten Schnittstellen sind verfügbar:

~~~bash
zelyra check examples/fibonacci.zyl --format=json
zelyra context examples/auth_crud_api.zyl --format=json
zelyra fmt examples/fibonacci.zyl --check
zelyra impact examples/auth_crud_api.zyl --format=json
zelyra impact examples/auth_crud_api.zyl --symbol table:customers --format=json
~~~

Diese Maschinenschnittstellen verwenden Schema-Version `1`, soweit sie JSON
ausgeben. JSON wird ausschließlich auf stdout
ausgegeben; Diagnosen besitzen stabile Codes und UTF-8-Byte-Offsets, technische
Meldungen bleiben auf stderr. `context` ist schreibgeschützt und meldet
Deklarationen, ohne eine Datenbank zu verbinden oder Secrets auszugeben.
`zelyra fmt` schreibt eine kanonische Formatierung; `--check` meldet nicht
formatierte Dateien, ohne sie zu verändern. Ausdrucks-Lücken mit `_` liefern
Kontextdiagnosen und werden von baubaren Befehlen abgelehnt. Die erste
quelltextbasierte Wirkungsanalyse mit deterministischen semantischen
Referenzkanten und validierte semantische Umbenennungen mit
ausdrücklichen Vorschau-/Anwende-Modi sind verfügbar, einschließlich
AST-basierter Umbenennungen von Funktionen, Typen, Records, Tabellen, Views,
Forms, CRUDs und Komponenten; Tabellenumbenennungen aktualisieren außerdem
geprüfte SQL-Tabellenpositionen, ohne Literale oder Kommentare zu verändern,
während Komponenten-Umbenennungen bekannte Komponententags in HTML-Bodies
aktualisieren. Eine
vollständige Laufzeit-/Schemaanalyse und vergleichende KI-Benchmarks bleiben
geplant. Siehe die
[KI-native Architektur](docs/architecture/ai-native-development.de.md) und die
[Benchmark-Spezifikation](docs/benchmarks/ai-authoring.de.md).

Vor dem Schreiben oder Ändern von `.zyl`-Quellcode ist die
[Quellenlandkarte und Prüfanleitung](docs/source-authority.de.md) zu verwenden.
Sie ordnet Spezifikation, Parser, Tests, den Status der Standardbibliothek,
geprüfte Beispiele und Dokumentation und beschreibt den Umgang mit Unsicherheit.

## Einfacher Einstieg, optionale Möglichkeiten

Zelyra hält das erste Projekt klein. Erweiterte Projektbereiche sind optional:
Dauerhafte Feature-Entscheidungen gehören in `zelyra.toml`,
umgebungsabhängige Überschreibungen in `.env` oder in die Prozessumgebung.
Bestehende Projekte benötigen keinen zusätzlichen Abschnitt; die sicheren
Standards lassen Web, API, CRUD, Authentifizierung und Audit-Deklarationen zu.

Beispiel:

~~~toml
[features]
api = false
crud = false
~~~

Die entsprechenden nicht geheimen Umgebungsüberschreibungen heißen
`ZELYRA_FEATURE_WEB`, `ZELYRA_FEATURE_API`, `ZELYRA_FEATURE_CRUD`,
`ZELYRA_FEATURE_AUTH` und `ZELYRA_FEATURE_AUDIT`. Die Reihenfolge lautet:
Prozessumgebung, `.env`, `zelyra.toml`, dann Standardwerte. Die wirksame
Konfiguration kann ohne Ausgabe von Secrets angezeigt werden:

~~~bash
zelyra config main.zyl --format=json
~~~

Deaktivierte Bereiche werden vom Compiler abgelehnt, wenn der Quellcode sie
verwendet; Sicherheitsprüfungen und Capabilities können damit nicht abgeschaltet
werden. Diese optionale Komfortschicht ist keine Voraussetzung für Einsteiger.
Die vollständige Referenz für Umgebung und Konfiguration steht in
[docs/env.md](docs/env.md) (zusätzlich auf
[Englisch](docs/env.en.md)). Neue Einstellungen müssen dort vor dem Commit
ergänzt werden.

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
  Transaktionen, HTTP-Weiterleitungen sowie aktionsbezogenen
  Authentifizierungs- und Berechtigungsprüfungen.
- eine erste CRUD-Ressource mit MariaDB-Suche, konfigurierbaren Listen-,
  Such- und Filterspalten, Sortierung, Pagination, erzeugten Create-/Edit-
  Formularen und CSRF-geschütztem Löschen mit getrennten Berechtigungen für
  View/Create/Edit/Delete sowie abwärtskompatiblem Berechtigungs-Fallback;
  nicht verfügbare Aktionen werden in erzeugten Ansichten ausgeblendet,
  direkte Requests bleiben geschützt.
- erste Authentifizierungssperren, Argon2-Login gegen eine MariaDB-
  Benutzertabelle, HttpOnly-Sessions, Logout sowie direkte und rollenbasierte
  Berechtigungsprüfungen.
- CLI-Befehle zum Gewähren und Entziehen von Rollen und
  Rollenberechtigungen mit Schemavalidierung und idempotenten MariaDB-
  Schreibvorgängen.
- optionale browserbasierte Rollenverwaltung mit CSRF-Schutz,
  Berechtigungssperren, Benutzeranlage, Passwort-Zurücksetzung, Aktivierung
  und Deaktivierung sowie Schutz der letzten Administrationszuordnung.
- optionales MariaDB-Audit-Logging für Login-, Logout-, Passwort-, Benutzer-,
  Rollen- und Berechtigungsereignisse mit Anzeige der letzten 100 Einträge sowie
  CLI-Befehlen zum Inspizieren und Exportieren.
- typisierte API-Deklarationen mit Routen-/Typprüfung, optionalen ausführbaren
  Handlern, OpenAPI-3.0.3-Ausgabe über `zelyra doc`, Authentifizierungs- und
  Berechtigungssperren, strukturierten JSON-Fehlern und deklarierter
  `Result`-Fehlerzuordnung einschließlich typisierter API-Arrays und
  verschachtelter JSON-Objekte über `struct`-Records;
- einen TypeScript-Client-Generator ohne zusätzliche Abhängigkeiten über
  `zelyra doc <file.zyl> --typescript`, einschließlich typisierter API-
  Fehlercodes und strukturierter HTTP-Fehlerauswertung;
- typisierte API-Fehler-Payloads über `errors { 422 ValidationError: Problem }`,
  einschließlich `error.details`, OpenAPI-Schemas und TypeScript-Payload-Typen;
- exakte CORS-Origin-Konfiguration für Browser-APIs mit automatischer
  `OPTIONS`-Preflight-Verarbeitung, standardmäßig deaktiviert;
- Array-Literale, Indexzugriff, `len`, `append`, `contains`, `first`, `last`
  und Array-Verkettung mit `+`; `first` und `last` liefern bei leeren Arrays
  sicher ein `Option`-Ergebnis;
- deterministische typisierte Maps mit `Map<Schlüssel, Wert>`-Deklarationen,
  `Map { ... }`-Literalen, nullsicherem `get`, funktionalem `put`, `keys`,
  `values` und JSON-Konvertierung für Maps mit String-Schlüsseln;
- Maps mit String-Schlüsseln werden an typisierten API-Grenzen einmal geprüft
  und konsistent in JSON, OpenAPI-`additionalProperties` und erzeugte
  TypeScript-Clients als `Record<string, Wert>` übernommen;
- strukturierte `for ... in`-Iteration über Arrays mit `break` und `continue`;
- Record-Literale und geprüfter Feldzugriff für verschachtelte Fachobjekte;
- erste Capability-Deklarationen und statische Weitergabe über
  Funktionsaufrufe; natives SQL benötigt `Database`, mit Projektfreigaben aus
  `zelyra.toml`.
- Runtime-Durchsetzung deklarierter Funktions-Capabilities sowie nativer SQL-,
  Formular-, CRUD- und Authentifizierungs-Datenbankzugriffe, wenn die CLI
  Projektfreigaben übergibt;
- sichere Host-APIs für Clock und Environment über now() und env(name); beide
  benötigen eine ausdrückliche Funktionsdeklaration und Projektfreigabe;
- sichere Zufallszahlen über random_int(min, max) und die Random-Capability;
  inklusive Grenzen und ungültige Bereiche werden zur Laufzeit geprüft;
- read_text(path), write_text(path, content), delete_file(path) und
  list_dir(path) über die FileSystem-Capability mit Projektpfadgrenzen;
- http_get(url) über die Network-Capability mit Projekt-Host-Allowlists,
  Zeitlimits und Antwortgrößenbegrenzung;
- http_request(method, url, headers, body) mit typisierten HttpResponse-Werten;
- json_encode(value) und json_decode<Typ>(text) für geprüfte JSON-Konvertierung
  von Records, Arrays, Optionen und Skalarwerten;
- http_json<Request, Response>(...) für automatische typisierte JSON-HTTP-
  Anfragen und -Antworten;
- http_result<Request, Response>(...) mit typisiertem `HttpResult<Response>`
  für Status, Header, Body, Daten und strukturierte HTTP-Fehler;
- run_process(command, args) über Process mit exakter Befehls-Allowlist,
  ohne Shell, Timeout und begrenzter Ausgabe;
- erste zur Laufzeit geprüfte Funktions-Contracts mit `requires` und `ensures`;
  diese Prüfungen werden nicht als formale Beweise ausgegeben.
- einen ersten Baustein für strukturierte Nebenläufigkeit mit `parallel` und
  `await`: Branches laufen mit unveränderlichem Umgebungs-Snapshot, werden vor
  der Fortsetzung zusammengeführt und in Quellreihenfolge übernommen;
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
  sicher gefunden werden kann. Die aktuelle Suche umfasst bis zu drei lineare
  Integer-Variablen und meldet auch Zeugen für fehlgeschlagene
  Schleifeninvarianten. `zelyra verify <file.zyl> --json` liefert
  strukturierte Daten mit `message` und `counterexample` für IDEs und CI.

Weitergehende CRUD-Erzeugung, Benutzerverwaltung, umfangreichere fachliche
Fehlerwerte, allgemeine formale Verifikation, Betriebssystem-Integration der
Capabilities und Produktionswerkzeuge werden noch entwickelt. Siehe die
[Roadmap](#roadmap) und den ausführlichen
[Getting-Started-Leitfaden](docs/getting-started.de.md).

Die konkreten Alleinstellungsmerkmale sind im
[Leitfaden zur Positionierung](docs/positioning.de.md) dokumentiert.

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

Vor dem Start einer Webanwendung die Bereitschaft prüfen:

~~~bash
zelyra doctor examples/machine_management.zyl
~~~

Für CI oder IDE-Werkzeuge liefert `zelyra doctor ... --json` maschinenlesbare
Prüfungen, ohne Datenbankzugangsdaten offenzulegen.

Der Installer baut Zelyra reproduzierbar für den aktuellen Benutzer und
installiert das Programm in einem benutzerlokalen bin-Verzeichnis. Er benötigt
weder sudo, eine globale Rust-Installation, Apache noch einen Datenbankserver
für die Sprachkern-Beispiele. `./install.sh --help` zeigt Dry-Run-, Prüf-,
Zielordner-, No-Rustup-, Offline-, Release- und Deinstallationsoptionen; veraltete
`cargo`-PATH-Einträge werden abgelehnt statt blind ausgeführt. Veröffentlichte
Linux-x86_64- und Windows-x86_64-Releases können mit
`./install.sh --release TAG` beziehungsweise `install.ps1 -Release TAG` ohne
Rust installiert werden; die passende SHA-256-Checksumme wird vor der
Installation geprüft. Unter Windows `install.ps1` in PowerShell oder
`install.cmd` verwenden; der Installer aktualisiert den Benutzer-PATH ohne
Administratorrechte.

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

Wiederverwendbare Views ermöglichen individuelle Seiteninhalte bei gemeinsamem
Anwendungsrahmen:

~~~zelyra
view SiteShell {
    html {
        <html><body><main><slot /></main></body></html>
    }
}

page "/customers" {
    view: SiteShell
    html { <h1>Customers</h1> }
}
~~~

Der Compiler verlangt in jedem benannten View genau einen `<slot />`-Slot. Der
Seiteninhalt wird vor dem Routing in diesen Slot eingesetzt; Authentifizierung
und Escaping bleiben dadurch in der bestehenden sicheren Web-Pipeline. Ein
vollständiges Beispiel steht in `examples/views.zyl`.

Views können außerdem typisierte, wiederverwendbare Komponenten deklarieren:

~~~zelyra
component Badge {
    props { text: String }
    html { <span class="badge">{text}</span> }
}

page "/status" {
    html { <Badge text="Ready" /> }
}
~~~

Komponenten können über einen Default-Slot auch beliebiges HTML umschließen:

~~~zelyra
component Panel {
    html { <section class="panel"><slot /></section> }
}

page "/dashboard" {
    html { <Panel><h1>Dashboard</h1></Panel> }
}
~~~

View-Interpolationen werden geprüft, bevor der Server startet. Eine Seite darf
ihre Routenparameter verwenden, eine Komponente ihre deklarierten Properties
und eine dynamische Component-Property muss typkompatibel sein. Eine Seite
kann außerdem ausdrücklich einen typisierten Datensatz laden und geprüften
Feldzugriff verwenden:

~~~zelyra
page "/customers/{name}" {
    load customer = sql<Customer> {
        SELECT id, name FROM customers WHERE name = :name
    }
    html { <h1>{customer.name}</h1> }
}
~~~

SQL wird gegen das Schema geprüft, Routenparameter werden sicher gebunden und
Authentifizierung, Berechtigungen sowie die `Database`-Capability werden vor
der Abfrage erzwungen. Geladene Werte werden im HTML escaped. Collections
können mit einer typisierten serverseitigen Schleife gerendert werden:

~~~zelyra
page "/customers" {
    load customers = sql<Customer[]> { SELECT id, name FROM customers }
    html { <ul>for customer in customers { <li>{customer.name}</li> }</ul> }
}
~~~

Option-aware Feld-Ausdrücke und reichere View-Daten bleiben geplant. Siehe
`examples/view_data.zyl` und `examples/view_collection.zyl`.

Seiten können außerdem typisierte Query-Eingaben für ausdrücklich
serverseitiges SQL deklarieren:

~~~zelyra
page "/customers" {
    input {
        search: String?
    }

    load customers = sql<Customer[]> {
        SELECT id, name FROM customers
        WHERE (:search IS NULL OR name LIKE CONCAT('%', :search, '%'))
        ORDER BY name
    }

    html { <p>Suche: {search}</p> }
}
~~~

Query-Werte werden gegen den deklarierten Zelyra-Typ geprüft und als
Datenbankparameter gebunden. Eine fehlende optionale Eingabe wird zu SQL
`NULL`; fehlende Pflichtwerte und ungültige Werte erzeugen eine kontrollierte
HTTP-400-Antwort. Eine automatische Erzeugung von Query-Steuerungen für
beliebige Seiten bleibt geplant. Page-Collections können bereits sicher
serverseitig sortiert und paginiert werden:

~~~zelyra
page "/customers" {
    sort { name }
    paginated 25
    load customers = sql<Customer[]> { SELECT id, name FROM customers }
    html { <p>Seite: {page}, Sortierung: {sort}, Reihenfolge: {order}</p> }
}
~~~

`page` wird als positive Ganzzahl geprüft, verwendet standardmäßig `1` und
wird als parametrisierter `LIMIT`-/`OFFSET`-Wrapper um das Collection-SQL
angewendet. `sort` akzeptiert nur deklarierte Ergebnisfelder und `order` nur
`asc` oder `desc`; beides kann sicher in URLs wie
`/customers?sort=name&order=desc` verwendet werden. Siehe
`examples/view_query_input.zyl`.

Komponenten können mit `<slot name="header" />` auch benannte Slots deklarieren;
Aufrufer übergeben sie mit Blöcken wie
`<slot name="header">...</slot>`. Verschachtelte Komponenten werden von innen
nach außen erweitert, unbekannter oder ungenutzter Inhalt wird abgelehnt.
Benannte Slots dürfen sicher escapte, deterministische Fallback-Inhalte besitzen;
Aufrufer können sie explizit überschreiben. Siehe
`examples/component_slots.zyl`.

Das zusammengefasste Beispiel `examples/view_showcase.zyl` zeigt den
vorgesehenen Release-Pfad in einem kleinen Programm: einen benannten
Seitenrahmen, typisierte Komponenten, Default- und benannte Slots sowie eine
schemabasierte CRUD-Ressource mit unabhängig anpassbaren Listen-, Detail-,
Formular- und Ladeansichten. Prüfung:

~~~bash
zelyra check examples/view_showcase.zyl --format=json
~~~

Suche, Filter, Sortierung und Pagination sind in erzeugten CRUD-Listen bereits
verfügbar. Filter bieten typabhängige Operatoren wie `contains`, `gte` und
`is_null`; die Bedienelemente erhalten ihren Zustand über die URL. Beispiele:

~~~text
/customers?filter_name__contains=Presse
/customers?filter_quantity__gte=10
~~~

Erzeugte Such- und Filtersteuerungen verwenden ein semantisches Fieldset und
getrennte Beschriftungen für jeden Operator und Wert. Filterverarbeitung und
bewahrter URL-Zustand werden deterministisch sortiert, sodass dieselbe Anfrage
immer dieselbe Reihenfolge von Steuerungen und Pagination erzeugt.

Die Darstellung einer CRUD-Liste kann deklarativ angepasst werden, ohne die
geprüfte Daten- oder Autorisierungspipeline zu ersetzen:

~~~zelyra
crud Customer -> customers {
    view {
        list {
            mode: cards
            empty: "Keine Kunden gefunden."
        }
    }
}
~~~

Die generierten Modi `table` (Standard) und `cards` behalten Suche, typisierte
Filter, erlaubte Sortierung, Pagination, URL-Zustand, Escaping und
Berechtigungsprüfungen bei.

Auch die Detaildarstellung kann unabhängig angepasst werden:

~~~zelyra
crud Customer -> customers {
    view {
        detail {
            mode: cards
            title: "Kundendetails"
        }
    }
}
~~~

Der Standardmodus für Details ist `standard`. Beide Modi behalten generierte
Edit-, Create- und Delete-Aktionen, CSRF, Escaping und Berechtigungsprüfungen
bei.

CRUD-Formulare können dasselbe kontrollierte Layoutsystem verwenden:

~~~zelyra
view {
    form {
        mode: cards
        title: "Kundenformular"
        submit: "Kunden speichern"
    }
}
~~~

Überschrift und Absende-Schaltfläche werden escaped. Schema-Validierung,
Readonly-Prüfungen, CSRF-Schutz, Parameterbindung und Aktionsberechtigungen
bleiben aktiv.

Auch die Löschbestätigung kann angepasst werden:

~~~zelyra
view {
    delete {
        title: "Kunden löschen"
        message: "Dieser Vorgang kann nicht rückgängig gemacht werden."
        submit: "Jetzt löschen"
    }
}
~~~

Die generierte Route bleibt POST-only und verlangt weiterhin CSRF- sowie
Löschberechtigungsprüfungen.

Für den häufigen Fall kann ein gemeinsames Feldprofil die erzeugte Liste,
Detailansicht sowie Create-/Edit-Formulare steuern:

~~~zelyra
crud Customer -> customers {
    view {
        fields { name email active }
    }
}
~~~

Ein explizites `list { ... }` bleibt eine Überschreibung für Liste und Detail.
Primärschlüssel und automatisch erzeugte Felder bleiben in Formularen
automatisch ausgeschlossen; unbekannte Profilfelder weist der Compiler zurück.

Auch Lade- und Fehlerzustände von CRUD können konfiguriert werden:

~~~zelyra
view {
    loading { message: "Kunden werden geladen ..." }
    error {
        title: "Kunden nicht verfügbar"
        message: "Bitte später erneut versuchen."
    }
}
~~~

Die Lademeldung wird als escaped Metadatum für Progressive Enhancement
ausgegeben. Die serverseitige Antwort behauptet niemals fälschlich, dass ein
Ladezustand aktiv ist. Konfigurierte Fehlermeldungen ersetzen generische CRUD-
Datenbankfehler; interne Datenbankdetails bleiben verborgen.

Eigene CRUD-Aktionen ergänzen fachliche Operationen, ohne die generierte
Sicherheitspipeline zu ersetzen:

~~~zelyra
crud Customer -> customers {
    action deactivate {
        label: "Kunden deaktivieren"
        confirm: "Diesen Kunden wirklich deaktivieren?"
        permits "customers.edit"
        sql {
            UPDATE customers
            SET active = false
            WHERE id = :id
        }
        success "Kunde deaktiviert."
        redirect "/customers"
    }
}
~~~

Zelyra erzeugt dafür den POST-only-Endpunkt
`/customers/{id}/deactivate` und zeigt die Schaltfläche in der Detailansicht.
Die Anfrage verlangt die CRUD-Datenbank-Capability, CSRF-Schutz,
Authentifizierung und die deklarierte Berechtigung. `:id` wird aus der Route
gebunden; SQL bleibt parametrisiert. Aktionsnamen werden derzeit auch als
Schaltflächenbeschriftung verwendet. `label` steuert die escaped Beschriftung;
`confirm` ergänzt eine escaped Browser-Bestätigung vor dem Absenden. Ohne
`label` bleibt der Aktionsname die Beschriftung.

Aktionen können außerdem typisierte Eingabefelder deklarieren. Sie verwenden
dieselbe Validierung und Parameterbindung wie normale Formulare:

~~~zelyra
action set_active {
    label: "Aktivstatus setzen"
    icon: "check"
    field active: Bool { required }
    sql {
        UPDATE customers SET active = :active WHERE id = :id
    }
}
~~~

`icon` ergänzt einen zugänglichen, escaped Hook (`data-icon`) für das erzeugte
Aktionsfeld. Nach erfolgreicher Ausführung wird `success` als escaped
Statusmeldung an den konfigurierten Redirect angehängt, sodass CRUD-Listen
eine sichere Bestätigung ohne eigenes JavaScript anzeigen können.

Beziehungsfelder erhalten automatisch ein geprüftes Auswahlfeld. Zum Beispiel
lädt `field department: Department { required }` die Anzeigewerte aus der
referenzierten MariaDB-Tabelle, sendet die gespeicherte ID und lehnt nicht
mehr vorhandene IDs ab, bevor das SQL der Aktion ausgeführt wird.

`confirm_page` macht aus dem Aktionsbutton einen GET-Link zu einer
serverseitig gerenderten Bestätigungsseite. Diese enthält die konfigurierte
Meldung, typisierte Aktionsfelder, frischen CSRF-Schutz und einen ausdrücklichen
POST-Submit-Button. Die bisherige Kurzform `confirm: "..."` bleibt als leichte
Browser-Bestätigung erhalten.

Für ausführlichere Rückmeldungen ergänzt `success_page { title: "..."
message: "..." }` eine strukturierte escaped Erfolgsmeldung.
`error_page { title: "..." message: "..." }` ersetzt die generische
Aktionsfehlerseite, ohne Datenbankdetails an die Antwort weiterzugeben.

CRUD-Ressourcen können außerdem ein reversibles Soft Delete verwenden. Die
konfigurierte Zeitstempelspalte wird in erzeugten Formularen und Standardlisten
ausgeblendet:

~~~zelyra
table customers {
    id: Id primary auto
    name: String(100) required
    deleted_at: Timestamp?
}

crud Customer -> customers {
    soft_delete { column: deleted_at }
}
~~~

Die Löschaktion setzt dann den Zeitstempel, statt die Zeile zu entfernen.
`/customers?archived=true` zeigt archivierte Datensätze; deren Detailseite
bietet eine CSRF-geschützte `restore`-Aktion. Endgültiges Löschen,
Aufbewahrungsregeln und Massenarchivierung bleiben geplant.

Wenn eine `auth`-Definition eine `audit`-Tabelle angibt, schreiben erzeugte
CRUD-Mutationen zusätzlich Audit-Ereignisse in derselben MariaDB-Transaktion.
Erstellen, Ändern, Löschen, Archivieren, Wiederherstellen und eigene Aktionen
protokollieren Akteur, Operation, Tabelle, Zieldatensatz und Feldänderungen.
Passwörter, Tokens, Secrets und Hashes werden aus Änderungsdetails entfernt.

Für manipulationssichtbare Protokolle kann die kryptografische Verkettung
aktiviert werden:

~~~zelyra
auth users {
    table: users
    audit: auth_audit_log
    audit_chain: true
}
~~~

Die Audit-Tabelle benötigt dann zusätzlich eine `id`-Spalte sowie die
Pflichtspalten `previous_hash` und `entry_hash`, üblicherweise als `String(64)`.
Zelyra speichert kleingeschriebene
SHA-256-Hexwerte. Jeder Eintrag hasht den vorherigen Hash, die Akteur-ID (oder
`NULL`), das Ereignis, die Ziel-ID (oder `NULL`), die Details und den MariaDB-
Zeitstempel im kanonischen Format `YYYY-MM-DD HH:MM:SS`, getrennt durch `|`.
Die Kette wird gesperrt und in derselben Transaktion wie die Fachänderung
erweitert. `zelyra audit verify` prüft Verknüpfungen und Hashes. Bereinigen
wird bei verketteten Protokollen abgelehnt, weil das Löschen die Kette brechen
würde.

Der erste typisierte Teil der einheitlichen View-Datenpipeline ist jetzt für
`tableview`-Routen und explizite seitenlokale Datensatzabfragen verfügbar.
Collection-Operationen und reichere Datenkomposition für beliebige Views
bleiben geplant.

Eigenständige typisierte Tabellenansichten können bereits eine geprüfte
MariaDB-Abfrage bereitstellen. Das Ergebnis darf ein deklarierter Tabellentyp
oder ein eigener `struct` für Joins und Aggregationen sein:

~~~zelyra
tableview Customers {
    source sql<CustomerOverview[]> {
        SELECT c.id, c.name, COUNT(o.id) AS orders
        FROM customers c LEFT JOIN orders o ON o.customer_id = c.id
        GROUP BY c.id, c.name
    }
    columns { id name orders }
    filter { name orders }
    searchable
    sortable
    paginated 25
}
~~~

Dadurch entsteht eine serverseitig gerenderte Ansicht unter
`/views/customers`. SQL-Quelle, Aliase, Ergebnisfelder und deklarierte Spalten
werden gegen Schema und Struct geprüft; typisierte Filter, Suche, Sortierung,
Pagination, URL-Zustand und HTML-Escaping bleiben Teil des sicheren
Serverpfads. Ein
vollständiges MariaDB-Beispiel steht in `examples/tableview.zyl`.

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

Derselbe SQLite-Pfad wird durch `tests/sqlite-e2e.sh` geprüft: Das Skript
erstellt eine temporäre Datenbank, inspiziert das Schema, prüft einen
idempotenten Plan und kontrolliert die erzeugten Foreign Keys. Es verwendet
keine Anwendungsdaten oder Zugangsdaten aus der Host-Umgebung.

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
zelyra new <directory> [--mariadb] [--template minimal|mariadb-crud|mariadb-auth|mariadb-business] [--web-port <port>] [--host-port <port>] [--db-host-port <port>]
zelyra init [directory] [--mariadb] [--template minimal|mariadb-crud|mariadb-auth|mariadb-business] [--web-port <port>] [--host-port <port>] [--db-host-port <port>]
zelyra setup [directory]
zelyra check <file.zyl> [--format human|json]
zelyra fmt <file.zyl> [--check]
zelyra context <file.zyl> [--format human|json]
zelyra build <file.zyl>
zelyra run <file.zyl>
zelyra serve <file.zyl> [address]
zelyra doctor [file.zyl] [--env-file <path>] [--port <port>] [--json]
zelyra verify <file.zyl> [--json]
zelyra doc <file.zyl> [--openapi|--typescript]
zelyra auth hash-password [--stdin]
zelyra auth role <grant|revoke> <file.zyl> <user-id> <role>
zelyra auth role-permission <grant|revoke> <file.zyl> <role> <permission>
zelyra audit inspect <file.zyl> [--limit <n>]
zelyra audit export <file.zyl> [--limit <n>] [--format json|csv]
zelyra audit verify <file.zyl>
zelyra audit prune <file.zyl> --before <timestamp> [--confirm]
zelyra form validate <file.zyl> <FormName> [field=value ...]
zelyra db create <file.zyl>
zelyra db setup <file.zyl>
zelyra db bootstrap <file.zyl>
zelyra db inspect <file.zyl>
zelyra db plan <file.zyl>
zelyra db apply <file.zyl> [--allow-destructive]
~~~

Die Befehle sind bewusst klein und ausdrücklich. Apache, PHP, ein ORM und ein
Frontend-Framework sind für die obigen Beispiele keine Voraussetzungen.

Für den einfachsten lokalen Start mit MariaDB und dem integrierten Webserver
ein Projekt mit `zelyra new meine-app --mariadb --web-port 8080 --host-port
18080 --db-host-port 3307` erstellen. Der erzeugte Compose-Stack verwendet
intern Port 8080, veröffentlicht den Webserver als
`http://127.0.0.1:18080` und MariaDB auf Host-Port 3307. Mit
`zelyra setup meine-app` wird eine geschützte `.env` mit zufälligen lokalen
MariaDB-Zugangsdaten erzeugt. Eine vorhandene `.env` wird niemals
überschrieben; `ZELYRA_WEB_PORT`, `ZELYRA_HOST_PORT` und
`ZELYRA_DB_HOST_PORT` können dort unabhängig geändert werden.

Nach dem Start des erzeugten Compose-Stacks mit
`zelyra doctor main.zyl --env-file .env --port 18080 --json` Quellcode, Schema,
MariaDB-Verbindung, Docker Compose und den veröffentlichten Host-Port
schreibgeschützt prüfen. Die geladenen Zugangsdaten werden niemals ausgegeben.

Für eine sofort erkundbare MariaDB-Geschäftsanwendung das optionale
CRUD-Starterprojekt verwenden:

~~~bash
zelyra new maschinenverwaltung --template mariadb-crud \
    --web-port 8080 --host-port 18080 --db-host-port 3307
cd maschinenverwaltung
zelyra setup .
docker compose --env-file .env -f docker-compose.mariadb.yml up -d --build
set -a; . ./.env; set +a
zelyra db setup main.zyl
~~~

Das Starterprojekt enthält Abteilungen und Maschinen, eine Foreign-Key-
Beziehung, schemaabhängige Formulare, CRUD-Seiten, Suche, Filterung,
Pagination und eigene Aktionen. Das Standardprojekt bleibt das kleinere
Willkommensseiten-Scaffolding.

Für Authentifizierung, Sessions und Berechtigungsprüfungen verwenden:

~~~bash
zelyra new sichere-app --template mariadb-auth \
    --web-port 8080 --host-port 18080 --db-host-port 3307
~~~

Das Authentifizierungs-Starterprojekt enthält Benutzer, persistente Sessions,
datenbankbasierte Berechtigungen, den automatischen `/login`- und `/logout`-
Ablauf sowie eine geschützte `/admin`-Seite.

Für einen vollständigen Business-Starter mit diesen Funktionen sowie
geschütztem CRUD, Audit-Protokoll, schema-basiertem Formular und typisierter API:

~~~bash
zelyra new business-app --template mariadb-business \
    --web-port 8080 --host-port 18080 --db-host-port 3307
~~~

Der Business-Starter ist der kürzeste Weg von einem neuen Projekt zu einer
datenbankgestützten Anwendung, die mit normalem Zelyra-Code erweitert werden kann.

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
   Pagination, konfigurierbare Spalten, Beziehungslabels und -Selects sowie
   CSRF-geschütztes Löschen und getrennte Aktionsberechtigungen vorhanden.
8. Authentifizierung und Autorisierung — Argon2-Login, persistente MariaDB-
   Sessions, Logout, Routensperren, direkte und rollenbasierte
   Berechtigungsabfragen sowie optionale Rollenverwaltung vorhanden.
9. Capabilities und Contracts — erste Deklarationen, statische Prüfungen,
   Runtime-Contracts, begrenzte symbolische Verifikation,
   Runtime-Capability-Grenzen sowie ein erster `parallel`/`await`-Baustein für
   strukturierte Nebenläufigkeit sind vorhanden; Betriebssystemrechte und
   allgemeine formale Verifikation folgen.
10. Typisierte API-Deklarationen, ausführbare Handler, API-Authentifizierung
    und Berechtigungen sowie OpenAPI-3.0.3-Erzeugung sind vorhanden; Client
    State, WebAssembly und Optimierungsschnittstellen folgen.
11. Browser-API-Integration mit ausdrücklichen CORS-Origins und automatischer
    Preflight-Verarbeitung ist vorhanden.
12. API-Validierung von Medientypen und Body-Größe, vollständiges Einlesen
    über mehrere Netzwerk-Reads sowie sichere Standard-Response-Header sind
    vorhanden.
13. MariaDB-CRUD-End-to-End-Abdeckung und GitHub-Actions-CI sind vorhanden.
14. Linux- und Windows-Release-Archive mit SHA-256-Prüfsummen werden
    automatisch aus Tags erzeugt.
15. Die MariaDB-Tableview-Abdeckung prüft struct-basierte Joins, Aggregate,
    Escaping, Suche, Sortierung und Pagination über den laufenden Webserver.
16. Bootstrap-Unabhängigkeit und ein selbsthostender Compiler sind strategische
    Ziele; der aktuelle Compiler-Bootstrap bleibt Rust, während Nutzer
    veröffentlichter Zelyra-Versionen Rust nicht installieren müssen.

Jedes Feature soll Syntax, AST/HIR-Unterstützung, Diagnosen, positive und
negative Tests, Dokumentation und Beispiele enthalten.

Für eine vollständig getrennte lokale MariaDB-Testinstanz steht die Compose-
Datei im Repository bereit. Sie veröffentlicht ausschließlich Port `3308` auf
dem Host und verwendet einen eigenen Container sowie ein eigenes Volume:

~~~bash
export ZELYRA_MARIADB_ROOT_PASSWORD='<test-passwort>'
export ZELYRA_MARIADB_PASSWORD='<test-passwort>'
docker compose -f tests/docker-compose.mariadb.yml up -d
DATABASE_URL='mariadb://root:<test-passwort>@127.0.0.1:3308/zelyra_test' \
    ./tests/mariadb-e2e.sh
~~~

Die Testinstanz heißt `zelyra-mariadb-tests`; sie verwendet keine andere
MariaDB-Installation und verändert deren Konfiguration nicht.

Um den vollständigen Ersteinrichtungsweg gegen eine frische Datenbank zu
prüfen, die CLI bauen und das Root-Passwort dieser isolierten MariaDB-Instanz
setzen:

~~~bash
cargo build -p zelyra-cli
export ZELYRA_GENERATED_E2E_ROOT_PASSWORD='<test-passwort>'
./tests/generated-project-mariadb-e2e.sh
~~~

Der Test erzeugt mit `zelyra new` ein temporäres Projekt, führt `zelyra setup`
aus, prüft die erzeugte Compose- und `doctor`-Konfiguration, legt eine
temporäre Datenbank auf dem isolierten MariaDB-Server an und führt den
vollständigen CRUD-HTTP-Test aus. Projekt und Datenbank werden automatisch
entfernt. Das Passwort wird weder ausgegeben noch vom Test gespeichert.

Die erzeugte Docker-Laufzeit kann separat geprüft werden:

~~~bash
./tests/generated-project-docker-e2e.sh
~~~

Dabei wird der erzeugte Dockerfile gegen den veröffentlichten Zelyra-Tag
gebaut, MariaDB und Webserver werden standardmäßig auf den eigenen Ports 3309
und 18082 gestartet, die Willkommensseite und Port-Zuordnungen werden geprüft
und Container, Netzwerk sowie Volume anschließend automatisch entfernt.

## Mitwirken

Das Repository wird bewusst in kleinen, testbaren Phasen entwickelt. Vor
Änderungen:

~~~bash
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
~~~

GitHub Actions führt zusätzlich die MariaDB-, Docker-, Authentifizierungs- und Business-
End-to-End-Tests für erzeugte Projekte sowie den zugangsdatenfreien MariaDB-
CRUD-Integrationstest aus
`tests/mariadb-e2e.sh` gegen einen isolierten MariaDB-11-Service aus.
Der Tableview-Integrationstest aus `tests/mariadb-tableview-e2e.sh` führt
zusätzlich eine struct-basierte Join- und Aggregatansicht über den laufenden
Webserver aus und prüft Suche, Sortierung, Pagination und HTML-Escaping.
Der MariaDB-Authentifizierungs-Integrationstest aus
`tests/mariadb-auth-e2e.sh` prüft außerdem Login, persistente Sessions,
Berechtigungsablehnung und Logout.
Der geschützte CRUD-/API-Test aus `tests/mariadb-protected-e2e.sh` prüft die
Berechtigungsgrenze für HTML-CRUD- und JSON-API-Endpunkte einschließlich
getrennter Create-, Edit- und Delete-Berechtigungen sowie einer geschützten
eigenen Formularaktion.
Der Test für den generierten Business-Starter prüft außerdem
`zelyra doctor --json`, OpenAPI 3.0.3 und den generierten TypeScript-Client,
bevor der geschützte Anwendungsablauf ausgeführt wird.
Der Chain-Audit-Test aus `tests/mariadb-audit-chain-e2e.sh` prüft
transaktionale Hash-Anhänge, Manipulationserkennung und die sichere Ablehnung
des Bereinigens.

Deutsche und englische Benutzerdokumentation sollen synchron bleiben.
Architekturentscheidungen sollen Sicherheit, Kontrolle und Erweiterbarkeit
erhalten.

## Lizenz

Zelyra wird unter der MIT-Lizenz veröffentlicht. Siehe [LICENSE](LICENSE).
