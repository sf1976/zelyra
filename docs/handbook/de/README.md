# Das Zelyra-Handbuch

**Von der Datenbank zur Anwendung. Absicht beschreiben. Korrektheit beweisen.**

[English edition](../en/README.md) · Deutsch

Willkommen bei Zelyra, der Programmiersprache für Menschen, die eine
Kundenverwaltung bauen wollten und plötzlich sieben Frameworks, drei
Konfigurationsdateien und eine existenzielle Krise besaßen.

Zelyra verbindet eine statisch typisierte Sprache mit MariaDB-Schemata,
geprüftem SQL, Webseiten, Formularen, CRUD, Authentifizierung, Capabilities und
Verträgen. Dieses Handbuch führt anhand einer kleinen Maschinenverwaltung vom
ersten Programm bis zur datenbankgestützten Webanwendung.

> **Projektstatus:** Zelyra 0.1 ist experimentell. Viele beschriebene Grundlagen
> sind implementiert, aber noch nicht für den Produktionseinsatz freigegeben.

Die verbindliche Reihenfolge der Sprachquellen und der Prüfablauf stehen in der
[Quellenlandkarte](../../source-authority.de.md) und der
[englischen Quellenübersicht](../../source-authority.md).

## Statuszeichen

- ✅ **Implementiert:** im aktuellen Repository vorhanden.
- 🧪 **Experimentell:** vorhanden, aber noch jung oder eingeschränkt.
- 🗺️ **Geplant:** Teil der Sprachvision, noch nicht zuverlässig verfügbar.

## Inhalt

1. [Was Zelyra anders macht](#1-was-zelyra-anders-macht)
2. [Installation](#2-installation)
3. [Das erste Programm](#3-das-erste-programm)
4. [Projekte und CLI](#4-projekte-und-cli)
5. [Variablen, Typen und Funktionen](#5-variablen-typen-und-funktionen)
6. [Option, Result und Pattern Matching](#6-option-result-und-pattern-matching)
7. [MariaDB und Tabellen](#7-mariadb-und-tabellen)
8. [Schema prüfen und anwenden](#8-schema-prüfen-und-anwenden)
9. [Natives SQL](#9-natives-sql)
10. [Webseiten](#10-webseiten)
11. [Formulare](#11-formulare)
12. [CRUD](#12-crud)
13. [Authentifizierung und Berechtigungen](#13-authentifizierung-und-berechtigungen)
14. [Capabilities](#14-capabilities)
15. [Contracts und Verify](#15-contracts-und-verify)
16. [Konfiguration und Geheimnisse](#16-konfiguration-und-geheimnisse)
17. [Diagnosen und Fehlersuche](#17-diagnosen-und-fehlersuche)
18. [Testen und Mitentwickeln](#18-testen-und-mitentwickeln)
19. [Was als Nächstes kommt](#19-was-als-nächstes-kommt)
20. [KI-native Entwicklung mit Zelyra](#20-ki-native-entwicklung-mit-zelyra)

## 1. Was Zelyra anders macht

In einer typischen Businessanwendung wird dieselbe Information mehrfach
beschrieben: einmal in der Datenbank, noch einmal im Backend, erneut im
Formular und schließlich in der API. Zelyra versucht, daraus eine einzige
nachvollziehbare Kette zu machen:

~~~text
Tabelle → Typen → SQL → Formulare → CRUD → Webseite → API/OpenAPI
~~~

Eine Spalte wie diese:

~~~zelyra
email: Email? 
~~~

sagt bereits:

- Der Wert ist eine E-Mail-Adresse.
- Der Wert darf fehlen.
- SQL-Ergebnisse müssen diese Nullfähigkeit beachten.
- Formulare können das passende Eingabefeld erzeugen.
- Views müssen mit dem optionalen Wert vernünftig umgehen.

SQL bleibt dabei echtes SQL. Zelyra zwingt niemanden, einen anspruchsvollen
`JOIN` in eine 38 Glieder lange Methodenkette zu verwandeln. SQL hat schon
genug erlebt.

## 2. Installation

### Voraussetzungen

Für Sprachbeispiele genügen:

- Linux oder macOS;
- `curl`;
- eine funktionierende Shell.

MariaDB wird erst für Datenbank-, Formularaktions-, Auth- und CRUD-Beispiele
benötigt.

### Aus dem Repository installieren

~~~bash
git clone https://github.com/sf1976/zelyra.git
cd zelyra
./install.sh
~~~

Der Installer baut Zelyra für den aktuellen Benutzer und installiert das
Programm benutzerlokal. Danach:

~~~bash
zelyra --help
~~~

Der Installer ist wiederholbar und benutzerlokal. Optionen zur Kontrolle:

~~~bash
./install.sh --help
./install.sh --dry-run --root "$HOME/.local"
./install.sh --check
./install.sh --uninstall
~~~

Mit `--no-rustup` wird die automatische Rust-Installation deaktiviert,
`--no-path` unterdrückt PATH-Hinweise. Mit `--root PATH` oder
`ZELYRA_INSTALL_ROOT` lässt sich ein anderes benutzerbezogenes Ziel wählen.
Veraltete `cargo`-PATH-Einträge werden erkannt und nicht blind ausgeführt.

Veröffentlichte Releases für Linux x86_64 und Windows x86_64 können ohne Rust
oder Cargo installiert werden. Das gewählte Archiv wird über HTTPS geladen und
per SHA-256 geprüft:

~~~bash
./install.sh --release v0.1.42
~~~

Unter Windows in PowerShell `-Release v0.1.42` mit `install.ps1` verwenden.
macOS nutzt derzeit weiterhin den Quellcode-Installer.

Wenn die Shell `zelyra` nicht findet:

~~~bash
export PATH="$HOME/.local/bin:$PATH"
~~~

### Direkt aus dem Quellbaum

~~~bash
cargo run -p zelyra-cli -- run examples/fibonacci.zyl
~~~

Das ist länger, aber hervorragend geeignet, wenn du Cargo sehr vermisst.

## 3. Das erste Programm

Datei `hello.zyl`:

~~~zelyra
fn main() {
    print("Hallo von Zelyra")
}
~~~

Ausführen:

~~~bash
zelyra run hello.zyl
~~~

Ausgabe:

~~~text
Hallo von Zelyra
~~~

Ein etwas ehrgeizigeres Beispiel:

~~~zelyra
fn fibonacci(n: Int) -> Int {
    if n <= 1 {
        return n
    }

    return fibonacci(n - 1) + fibonacci(n - 2)
}

fn main() {
    print(fibonacci(10))
}
~~~

Ergebnis: `55`. Der Rechner hat es geschafft. Wir dürfen fortfahren.

## 4. Projekte und CLI

Ein Projekt neu erstellen:

~~~bash
zelyra new maschinenverwaltung
cd maschinenverwaltung
zelyra run main.zyl
~~~

Ein vorhandenes Verzeichnis initialisieren:

~~~bash
mkdir maschinenverwaltung
cd maschinenverwaltung
zelyra init
~~~

Für eine lokale MariaDB- und Webserver-Vorlage `zelyra new
maschinenverwaltung --mariadb --web-port 8080 --host-port 18080
--db-host-port 3307` verwenden.
Dadurch entstehen `.env.example`, `Dockerfile` und
`docker-compose.mariadb.yml`. Die Optionen wählen den internen Zelyra-Port und
den lokal veröffentlichten Port unabhängig. `ZELYRA_WEB_PORT` und
`ZELYRA_HOST_PORT` und `ZELYRA_DB_HOST_PORT` können später in `.env` geändert
werden; die Web-Ports verwenden standardmäßig 3000, der MariaDB-Host-Port
3306.
Nach dem Scaffolding erzeugt `zelyra setup maschinenverwaltung` eine `.env` mit
zufälligen lokalen MariaDB-Zugangsdaten. Vorhandene `.env`-Dateien werden nie
überschrieben und Zugangsdaten nie ausgegeben.
Nach dem Start von Compose mit `zelyra doctor main.zyl --env-file .env
--port 18080` Quellcode, Schema, MariaDB-Verbindung, Docker Compose und den
veröffentlichten Port ohne Datenbankänderung prüfen.

Für ein vollständiges CRUD-Starterprojekt statt der minimalen Willkommensseite:

~~~bash
zelyra new maschinenverwaltung --template mariadb-crud \
    --web-port 8080 --host-port 18080 --db-host-port 3307
cd maschinenverwaltung
zelyra setup .
docker compose --env-file .env -f docker-compose.mariadb.yml up -d --build
set -a; . ./.env; set +a
zelyra db setup main.zyl
~~~

Das Starterprojekt enthält verbundene Abteilungen und Maschinen, Formulare,
CRUD-Seiten, Suche, Filterung, Pagination und eigene Aktionen.

Für ein Authentifizierungs-Starterprojekt mit persistenten Sessions und
Berechtigungen:

~~~bash
zelyra new sichere-app --template mariadb-auth \
    --web-port 8080 --host-port 18080 --db-host-port 3307
~~~

Es enthält Benutzer, Sessions und Berechtigungen, den automatischen Login- und
Logout-Ablauf sowie eine geschützte `/admin`-Seite.

Für einen vollständigen Business-Starter mit Authentifizierung, geschütztem
CRUD, Audit-Protokoll, schema-basiertem Formular und typisierter API:

~~~bash
zelyra new business-app --template mariadb-business \
    --web-port 8080 --host-port 18080 --db-host-port 3307
~~~

Dies ist der empfohlene Einstieg für eine datenbankgestützte
Businessanwendung, die mit normalem Zelyra-Code erweitert wird.

Für Repository-Integrationstests steht eine getrennte MariaDB-Instanz bereit:

~~~bash
export ZELYRA_MARIADB_ROOT_PASSWORD='<test-passwort>'
export ZELYRA_MARIADB_PASSWORD='<test-passwort>'
docker compose -f tests/docker-compose.mariadb.yml up -d
DATABASE_URL='mariadb://root:<test-passwort>@127.0.0.1:3308/zelyra_test' \
    ./tests/mariadb-e2e.sh
~~~

Sie verwendet den Container `zelyra-mariadb-tests` und ein eigenes Volume.
MariaDB wird auf Host-Port `3308` veröffentlicht, ohne eine andere
Datenbankinstallation zu verändern.

Den vollständigen Weg eines erzeugten Projekts gegen eine frische Datenbank
prüfen:

~~~bash
cargo build -p zelyra-cli
export ZELYRA_GENERATED_E2E_ROOT_PASSWORD='<test-passwort>'
./tests/generated-project-mariadb-e2e.sh
~~~

Der Test erzeugt mit `zelyra new` ein temporäres Projekt, führt `zelyra setup`
aus, prüft die erzeugte Compose- und `doctor`-Konfiguration und entfernt
temporäres Projekt und Datenbank nach dem CRUD-HTTP-Test wieder. Das Passwort
wird weder ausgegeben noch gespeichert.

Auch die erzeugte Docker-Laufzeit kann geprüft werden:

~~~bash
./tests/generated-project-docker-e2e.sh
~~~

Dabei wird das erzeugte Image aus dem veröffentlichten Zelyra-Tag gebaut,
MariaDB und Webserver werden standardmäßig auf Host-Port 3309 und 18082
gestartet, Willkommensseite und Port-Zuordnungen werden geprüft und alle
temporären Docker-Ressourcen anschließend entfernt.

Nutzer, die Rust nicht installieren möchten, können das vorgefertigte Linux-
oder Windows-Archiv von der [GitHub-Releases-Seite](https://github.com/sf1976/zelyra/releases)
herunterladen. Jedes Archiv enthält eine SHA-256-Prüfsumme, die CLI, beide
README-Sprachen und die Lizenzhinweise.

Eine minimale `zelyra.toml`:

~~~toml
[project]
name = "maschinenverwaltung"
version = "0.1.42"
zelyra = "0.1"

[capabilities]
database = true
network = false
clock = true
environment = true
~~~

Die wichtigsten Befehle:

| Befehl | Zweck |
|---|---|
| `zelyra check app.zyl` | lexen, parsen, Namen und Typen prüfen |
| `zelyra build app.zyl` | Anwendung prüfen und bauen |
| `zelyra run app.zyl` | Programm ausführen |
| `zelyra serve app.zyl` | HTTP-Server starten |
| `zelyra doctor app.zyl [--json]` | Projekt-, DB- und Web-Bereitschaft prüfen |
| `zelyra verify app.zyl` | Contracts klassifizieren |
| `zelyra doc app.zyl --openapi` | OpenAPI-Dokument erzeugen |
| `zelyra db inspect app.zyl` | Ist-Schema lesen |
| `zelyra db setup app.zyl` | MariaDB und Anfangsschema einrichten |
| `zelyra db plan app.zyl` | Schemaänderungen anzeigen |
| `zelyra db apply app.zyl` | geprüften Plan anwenden |
| `zelyra audit inspect app.zyl` | letzte Audit-Ereignisse anzeigen |
| `zelyra audit export app.zyl --format json` | Audit-Ereignisse als JSON exportieren |
| `zelyra audit verify app.zyl` | Audit-Pflichtfelder und optionale Hashkette prüfen |
| `zelyra audit prune app.zyl --before <timestamp> --confirm` | alte Audit-Ereignisse entfernen |

## 5. Variablen, Typen und Funktionen

Werte sind standardmäßig unveränderlich:

~~~zelyra
machine_name = "Presse 7"
capacity: Int = 120
active = true
~~~

Veränderung muss sichtbar sein:

~~~zelyra
mutable completed = 0
completed = completed + 1
~~~

Das verhindert versehentliche Änderungen. Der Compiler ist dabei nicht
misstrauisch; er hat nur schon Dinge gesehen.

Wichtige Typen:

~~~text
Int UInt Float Decimal Bool String Char Bytes
Timestamp Date Time Duration Email Url Uuid Money
~~~

Funktionen:

~~~zelyra
fn available_capacity(total: Int, reserved: Int) -> Int {
    return total - reserved
}
~~~

Nominale IDs verhindern Verwechslungen:

~~~zelyra
type MachineId = Id
type OrderId = Id
~~~

Eine `OrderId` ist dadurch nicht automatisch eine `MachineId`, auch wenn beide
intern ähnlich aussehen. Fachlich falsch bleibt fachlich falsch.

## 6. Option, Result und Pattern Matching

Normale Typen sind nicht `null`. Ein möglicher fehlender Wert wird markiert:

~~~zelyra
email: Email?
~~~

Behandlung:

~~~zelyra
match email {
    Some(value) => print(value)
    None => print("Keine E-Mail hinterlegt")
}
~~~

Pattern Matching muss vollständig sein. Sonst erinnert dich der Compiler an
den Fall, den der Freitagabend-Deploy vermutlich gefunden hätte.

Fehler werden als Teil der Funktionssignatur sichtbar:

~~~zelyra
fn load_machine(id: MachineId)
    -> Machine
    throws DatabaseError | NotFound
{
    // Implementierung
}
~~~

## 7. MariaDB und Tabellen

✅ MariaDB ist das Standardbackend und die primäre Runtime-Referenz.

~~~zelyra
database main {
    engine: mariadb
}

table departments {
    id: Id primary auto
    name: String(100) required unique
}

table machines {
    id: Id primary auto
    number: String(30) required unique
    name: String(100) required
    department: Department required
    active: Bool default true
}
~~~

Zelyra erkennt die Beziehung zwischen Maschine und Abteilung. Daraus können
Foreign Keys, Formulare und Auswahlfelder entstehen.

Typische Abbildung:

| Zelyra | MariaDB |
|---|---|
| `Id primary auto` | automatisch vergebene Primär-ID |
| `String(100)` | `VARCHAR(100)` |
| `String` | `TEXT` |
| `Email` | E-Mail-kompatible Textspalte |
| `Bool` | boolescher Datenbankwert |
| `Timestamp` | Zeitstempelwert |

## 8. Schema prüfen und anwenden

Verbindung ausschließlich über die Umgebung bereitstellen:

~~~bash
export DATABASE_URL='mariadb://user:password@127.0.0.1:3306/zelyra_demo'
~~~

Anfangsschema erzeugen:

~~~bash
zelyra db bootstrap examples/machine_management_mariadb.zyl
~~~

Bestehendes Schema inspizieren:

~~~bash
zelyra db inspect examples/machine_management_mariadb.zyl
~~~

Änderungen zunächst nur planen:

~~~bash
zelyra db plan examples/machine_management_mariadb.zyl
~~~

Danach anwenden:

~~~bash
zelyra db apply examples/machine_management_mariadb.zyl
~~~

Destruktive Änderungen werden abgelehnt, bis sie ausdrücklich freigegeben
werden:

~~~bash
zelyra db apply examples/machine_management_mariadb.zyl --allow-destructive
~~~

Dieses Flag bedeutet nicht „wird schon gutgehen“. Es bedeutet „ich habe den
Plan gelesen, ein Backup und einen vernünftigen Puls“.

## 9. Natives SQL

✅ SQL ist ein Sprachelement:

~~~zelyra
fn load_active_machines() -> Machine[]
    uses Database
{
    return sql<Machine[]> {
        SELECT id, number, name, department_id, active
        FROM machines
        WHERE active = true
        ORDER BY number
    }
}
~~~

Parameter werden benannt und sicher gebunden:

~~~zelyra
fn load_machine(id: MachineId) -> Machine?
    uses Database
{
    return sql<Machine?> {
        SELECT id, number, name, department_id, active
        FROM machines
        WHERE id = :id
    }
}
~~~

Zelyra prüft, soweit das Schema bekannt ist:

- Tabellen und Spalten;
- Aliase;
- Parameter;
- Nullfähigkeit;
- Ergebniszuordnung;
- erforderliche `Database`-Capability.

Schreibzugriff in einer Transaktion:

~~~zelyra
transaction {
    sql {
        UPDATE machines
        SET active = false
        WHERE id = :id
    }
}
~~~

## 10. Webseiten

✅ Eine erste sichere Webschicht ist implementiert.

~~~zelyra
page "/machines/{name}" {
    html {
        <html>
            <body>
                <h1>Maschine {name}</h1>
                <p>Sie läuft. Hoffentlich nicht weg.</p>
            </body>
        </html>
    }
}
~~~

Start:

~~~bash
zelyra serve app.zyl
~~~

Dann beispielsweise:

~~~text
http://127.0.0.1:3000/machines/Presse-7
~~~

Pfadwerte werden standardmäßig HTML-escaped. Der derzeitige Web Core umfasst
GET-Routen, Pfadparameter, Query-String-Behandlung, HTTP-Parsing und
HTML-Antworten. Ein vollständiges Komponenten- und Client-State-System steht
auf der Roadmap.

### Wiederverwendbare Views

Benannte Views bilden eine sichere Layout-Grenze für die individuelle
Seitengestaltung:

~~~zelyra
view SiteShell {
    html {
        <html><body><header>Zelyra</header><main><slot /></main></body></html>
    }
}

page "/customers" {
    view: SiteShell
    html { <h1>Customers</h1> }
}
~~~

Der Compiler verlangt in einem benannten View genau einen Default-`<slot />`.
Layouts dürfen zusätzlich benannte Slots mit Fallback-Inhalt deklarieren:

~~~zelyra
view AppShell {
    html {
        <header><slot name="header"><h1>Zelyra</h1></slot></header>
        <main><slot /></main>
    }
}

page "/dashboard" {
    view: AppShell
    html {
        <slot name="header"><h1>Dashboard</h1></slot>
        <p>Seiteninhalt</p>
    }
}
~~~

Die Seite darf nur Slots des ausgewählten Views liefern; doppelte oder
unbekannte Slots werden beim Prüfen abgelehnt. Nicht gelieferte benannte Slots
verwenden ihren Fallback. Der Seiteninhalt wird vor dem Routing eingesetzt;
Authentifizierung, Autorisierung und Escaping bleiben aktiv. Typisierte
selbstschließende Komponenten mit deklarierten Properties sind ebenfalls
verfügbar:

~~~zelyra
component Badge {
    props { text: String }
    html { <span class="badge">{text}</span> }
}

page "/status" {
    html { <Badge text="Ready" /> }
}
~~~

Komponenten können außerdem über einen Default-Slot oder benannte Slots
HTML-Kindelemente aufnehmen:

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

Seiten können typisierte Query-Eingaben für ausdrücklich serverseitiges SQL
deklarieren:

~~~zelyra
page "/customers" {
    input { search: String? }

    load customers = sql<Customer[]> {
        SELECT id, name FROM customers
        WHERE (:search IS NULL OR name LIKE CONCAT('%', :search, '%'))
        ORDER BY name
    }

    html { <p>Suche: {search}</p> }
}
~~~

Query-Werte werden gegen ihren Zelyra-Typ geprüft und sicher als
Datenbankparameter gebunden. Eine fehlende optionale Eingabe wird zu SQL
`NULL`; ein fehlender Pflichtwert oder ein ungültiger Wert erzeugt eine
kontrollierte HTTP-400-Antwort. Eine automatische Erzeugung von
Query-Steuerungen für deklarierte Page-Collections ist integriert; Suche,
Filter, Sortierung und Pagination bewahren dabei den URL-Zustand.
Page-Collections können bereits sicher serverseitig sortiert und paginiert werden:

~~~zelyra
page "/customers" {
    search { name }
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

`search { name email }` stellt dieselbe vom Compiler geprüfte Whitelist für
den URL-Wert `search` bereit. Suchbegriffe werden als Parameter gebunden und
mit serverseitigen `LIKE`-Bedingungen angewendet; ungeprüfte SQL-Fragmente
werden niemals erzeugt.

Collection-Seiten können außerdem typisierte Filter deklarieren:

~~~zelyra
page "/customers" {
    filter { name quantity }
    load customers = sql<Customer[]> { SELECT id, name, quantity FROM customers }
    html { <p>{filter_name}</p> }
}
~~~

Der Compiler prüft Filterfelder gegen den Ergebnistyp der Collection.
Textfelder unterstützen `eq`, `contains`, `starts_with`, `ends_with` und
Nullprüfungen; numerische Felder zusätzlich `gt`, `gte`, `lt` und `lte`;
Boolean- und andere Werte unterstützen Gleichheit und Nullprüfungen. Werte
bleiben parametrisiert, Felder und Operatoren werden als Whitelist geprüft.
Beispiele sind `/customers?filter_name__contains=Acme` und
`/customers?filter_quantity__gte=10`. Nicht unterstützte Operatoren und
unbekannte Felder erzeugen kontrolliertes HTTP 400.

Paginierte Page-Collections stellen nach einer sicheren Zählabfrage außerdem
`total` und `pages` als `UInt`-Bindings bereit. Seiten mit ausschließlich
expliziten `input`-Deklarationen bleiben manuell und erhalten keine erzeugten
Steuerungen.

Benannte Slots werden ausdrücklich deklariert und übergeben:

~~~zelyra
component Layout {
    html { <header><slot name="header" /></header><main><slot /></main> }
}

page "/dashboard" {
    html {
        <Layout>
            <slot name="header"><h1>Dashboard</h1></slot>
            <p>Inhalt</p>
        </Layout>
    }
}
~~~

Verschachtelte Komponenten werden von innen nach außen erweitert. Inhalt an
eine Komponente ohne passenden Default- oder benannten `<slot />` ist ein
Fehler zur Compile-Zeit. Benannte Slots dürfen sicher escapte,
deterministische Fallback-Inhalte besitzen und von einem Aufrufer explizit
überschrieben werden.

CRUD-Listenansichten können angepasst werden, ohne die generierte Abfrage- oder
Autorisierungspipeline zu ersetzen. Standardmäßig wird eine HTML-Tabelle
verwendet; zusätzlich gibt es Karten und eine eigene Leerzustandsmeldung:

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

`mode` akzeptiert `table` oder `cards`. Suche, typisierte Filter, erlaubte
Sortierung, Pagination, URL-Zustand, HTML-Escaping und Berechtigungsprüfungen
bleiben in beiden Modi aktiv. Detail-, Formular-, Lade- und Fehleransichten
können ebenfalls sicher überschrieben werden.

Für den häufigen Fall kann ein gemeinsames Feldprofil die erzeugte Liste,
Detailansicht sowie Create-/Edit-Formulare steuern:

~~~zelyra
crud Customer -> customers {
    view {
        fields { name email active }
    }
}
~~~

Ein explizites `list { ... }` bleibt eine Überschreibung für Liste/Detail.
Primärschlüssel und automatisch erzeugte Felder bleiben in Formularen
automatisch ausgeschlossen; unbekannte Profilfelder weist der Compiler zurück.

Detailansichten unterstützen dieselbe kontrollierte Darstellungsauswahl und
eine eigene Überschrift:

~~~zelyra
view {
    detail {
        mode: cards
        title: "Kundendetails"
    }
}
~~~

Der Standard ist `standard`. Generierte Edit-, Create- und Delete-Aktionen,
CSRF, Escaping und Berechtigungsprüfungen bleiben aktiv.

CRUD-Formulare können ein kontrolliertes Layout, eine Überschrift und eine
Beschriftung für die Absende-Schaltfläche festlegen:

~~~zelyra
view {
    form {
        mode: cards
        title: "Kundenformular"
        submit: "Kunden speichern"
    }
}
~~~

Überschrift und Beschriftung werden escaped. Schema-Validierung,
Readonly-Prüfungen, CSRF-Schutz, Parameterbindung und Aktionsberechtigungen
bleiben aktiv.

Die Löschbestätigung kann eine eigene Überschrift, Warnung und Beschriftung
für die Absende-Schaltfläche festlegen:

~~~zelyra
view {
    delete {
        title: "Kunden löschen"
        message: "Dieser Vorgang kann nicht rückgängig gemacht werden."
        submit: "Jetzt löschen"
    }
}
~~~

Die generierte Route bleibt POST-only und erzwingt weiterhin CSRF- und
Löschberechtigungsprüfungen.

CRUD-Lade- und Fehlerzustände können ebenfalls konfiguriert werden:

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
ausgegeben; die serverseitige Antwort behauptet nicht, dass gerade geladen
wird. Konfigurierte Fehlermeldungen ersetzen generische CRUD-Datenbankfehler,
ohne interne Datenbankdetails offenzulegen.

Eigene CRUD-Aktionen ergänzen fachliche Operationen und behalten dieselbe
Autorisierungs- und Parameterbindungspipeline:

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

Dadurch entstehen die POST-only-Route `/customers/{id}/deactivate` und eine
Schaltfläche in der Detailansicht. CSRF-Schutz, Datenbank-Capability,
Authentifizierung und deklarierte Berechtigungen werden geprüft. Die Routen-ID
wird an `:id` gebunden, daher bleibt SQL parametrisiert. Aktionsnamen werden
derzeit als Schaltflächenbeschriftung verwendet.
`label` überschreibt die escaped Schaltflächenbeschriftung; `confirm` ergänzt
eine escaped Browser-Bestätigung vor dem Absenden. Ohne `label` wird der
Aktionsname als Beschriftung verwendet.

Aktionen können auch typisierte Felder deklarieren. Sie verwenden die normale
Formularvalidierung und werden als SQL-Parameter gebunden:

~~~zelyra
action set_active {
    icon: "check"
    field active: Bool { required }
    sql {
        UPDATE customers SET active = :active WHERE id = :id
    }
}
~~~

`icon` ergänzt einen escaped `data-icon`-Hook am erzeugten Aktionsbutton.
`success` wird an den Redirect übergeben und auf CRUD-Listen als escaped
Statusmeldung dargestellt.

Mit `confirm_page { title: "..." message: "..." submit: "..." }` entsteht
eine serverseitige Bestätigungsstufe. Die Aktion in der Detailansicht wird zu
einem GET-Link; die Bestätigungsseite rendert die Felder mit einem frischen,
CSRF-geschützten POST-Formular. Die Autorisierung wird bei beiden Requests
geprüft.

`success_page` konfiguriert eine strukturierte escaped Erfolgsmeldung;
`error_page` stellt eine sichere aktionsspezifische Fehlerseite bereit, ohne
Datenbankdetails offenzulegen.

CRUD-Ressourcen können ein reversibles Soft Delete mit einem nullable
Zeitstempel aktivieren:

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

Die Löschaktion setzt den Zeitstempel, statt die Zeile zu entfernen. Normale
Listen schließen archivierte Datensätze aus; `/customers?archived=true` zeigt
sie, und die Detailansicht bietet eine CSRF-geschützte Wiederherstellung. Die
Markierung wird aus erzeugten Formularen sowie Standardlisten und -filtern
entfernt. Endgültiges Bereinigen und Aufbewahrungsregeln bleiben geplant.

Wenn das Projekt zusätzlich eine Authentifizierungs-Audit-Tabelle deklariert,
fügen erzeugte CRUD-Mutationen Audit-Ereignisse in derselben MariaDB-Transaktion
hinzu. Erstellen, Ändern, Löschen, Archivieren, Wiederherstellen und eigene
Aktionen protokollieren Akteur, Operation, Tabelle, Zieldatensatz und
Feldänderungen. Sensible Werte wie Passwörter, Tokens, Secrets und Hashes werden
entfernt.

Beziehungsfelder wie `department: Department` werden als geprüfte
Auswahlfelder dargestellt. Zelyra lädt ihre Beschriftungen aus der
referenzierten MariaDB-Tabelle, sendet die gespeicherte ID und lehnt veraltete
oder unbekannte IDs vor der Ausführung des Aktions-SQLs ab.

## 11. Formulare

✅ Formulare können Regeln aus Tabellen übernehmen:

~~~zelyra
form MachineCreate -> machines {
    fields {
        number
        name
        department
        active
    }
}
~~~

Explizite Definition:

~~~zelyra
form ContactForm {
    field email: Email {
        label: "E-Mail"
        required
        max: 255
        widget: email
    }
}
~~~

Werte ohne Server prüfen:

~~~bash
zelyra form validate examples/customer_form.zyl CustomerCreate \
    name="Muster GmbH" email=info@example.test
~~~

Mit `zelyra serve` stellt Zelyra das Formular unter `/forms/FormName` bereit.
GET rendert das Formular samt CSRF-Token; POST prüft Token und Werte.

Eine Aktion:

~~~zelyra
form CustomerCreate -> customers {
    fields { name email }

    action save {
        requires auth
        permits "customers.save"
        sql {
            INSERT INTO customers (name, email)
            VALUES (:name, :email)
        }

        redirect "/customers"
    }
}
~~~

Formularaktionen können eine eigene Autorisierung deklarieren. Die
Berechtigung wird beim Anzeigen des Formulars und erneut vor dem Absenden
geprüft.

## 12. CRUD

✅ Der kurze Fall ist erfreulich kurz:

~~~zelyra
crud Machine -> machines
~~~

Konfiguriert:

~~~zelyra
crud Machine -> machines {
    title: "Maschinen"
    list { number name department active }
    search { number name }
    filter { department active }
}
~~~

Zelyra stellt Listen, Details, Create/Edit-Formulare, Suche, Filter, Sortierung,
Pagination und eine CSRF-geschützte Löschaktion bereit. Spalten werden gegen
das Schema geprüft.

Beispiele für URLs:

~~~text
/machines
/machines?search=Presse
/machines?filter_active=true
/machines?filter_number__contains=CNC
/machines?filter_quantity__gte=10
/machines?sort=number&order=asc
/machines/new
/machines/42/edit
~~~

Filteroperatoren werden anhand des Schemas in den Bedienelementen angeboten.
Textspalten unterstützen `eq`, `contains`, `starts_with` und `ends_with`;
numerische Spalten unterstützen `eq`, `gt`, `gte`, `lt` und `lte`. Für alle
unterstützten Spalten gibt es außerdem `is_null` und `is_not_null`. Die
explizite URL-Form `filter_<column>__<operator>=<value>` eignet sich für Links
und gespeicherte Suchen. Werte bleiben gebundene Parameter, Spaltennamen werden
gegen das Schema geprüft.

Erzeugte CRUD- und Tableview-Steuerungen verwenden semantische Fieldsets und
getrennte Beschriftungen für Operator und Wert jedes Filters. Filterverarbeitung
und bewahrte Pagination-URLs verwenden eine deterministische Reihenfolge.

🗺️ Vollständig eigene typisierte Komponenten und feingranulare View-Overrides
sind Teil der weiteren View-Roadmap.

### Eigenständige Tabellenansichten

Mit `tableview` kann eine geprüfte MariaDB-Abfrage ohne CRUD-Ressource
bereitgestellt werden. Das Ergebnis kann ein Tabellentyp oder ein eigener
`struct` für Joins und Aggregationen sein:

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

Die erzeugte Route lautet `/views/customers`. Ergebnis-Typ, Projektionsaliase
und deklarierte Spalten werden gegen Schema und Ziel-Struct geprüft.
Suchbegriffe werden auf die
deklarierten Ergebnisspalten angewandt, Sortierung ist auf die Allowlist
beschränkt und Pagination-Werte werden gebunden. Siehe
`examples/tableview.zyl`.

Optionale `filter`-Felder verwenden denselben sicheren URL-Vertrag wie CRUD-
Filter: `filter_<column>=<value>` bedeutet standardmäßig Gleichheit,
`filter_<column>__<operator>=<value>` wählt einen Operator wie `contains`,
`gte` oder `is_null`. Der Compiler leitet die erlaubten Operatoren aus dem
typisierten Ergebnisfeld ab; unbekannte Felder, nicht unterstützte Operatoren
und ungültige Zahlen- oder Boolean-Werte werden mit HTTP 400 abgelehnt.

## 13. Authentifizierung und Berechtigungen

🧪 Zelyra unterstützt Argon2-Login, persistente MariaDB-Sessions, Logout,
Routenschutz und datenbankgestützte Berechtigungsprüfungen. Fünf
Fehlversuche für dieselbe normalisierte E-Mail-Adresse innerhalb von 15
Minuten lösen eine 60-sekündige HTTP-429-Sperre aus. Ein erfolgreicher Login
rotiert das vorherige Session-Token dieses Browsers und entwertet es.

Einen Wert für die erforderliche Spalte `password_hash` mit der CLI erzeugen.
Der interaktive Befehl schaltet die Passwortanzeige aus und verlangt eine
Bestätigung:

~~~bash
zelyra auth hash-password
~~~

Für bewusste Automatisierung eine Passwortzeile mit `--stdin` übergeben. Echte
Passwörter nicht als Kommandoargument verwenden und erzeugte Hashes nicht in
die Versionsverwaltung übernehmen:

~~~bash
printf '%s\n' 'dieses-passwort-aendern' | zelyra auth hash-password --stdin
~~~

~~~zelyra
auth users {
    table: users
    permissions: user_permissions
    roles: user_roles
    role_permissions: role_permissions
    audit: auth_audit_log
    admin_path: "/admin/access"
    admin_permission: "auth.manage"
    admin_role: admin
}

page "/admin" {
    requires auth
    permits "machines.manage"

    html {
        <h1>Maschinenverwaltung</h1>
    }
}
~~~

Die optionale Tabelle `permissions` enthält `user_id` und `permission` für
direkte Vergaben. Rollengruppen werden mit `roles` und `role_permissions`
aktiviert: Die erste Tabelle enthält `user_id` und `role`, die zweite `role`
und `permission`. Effektive Berechtigungen sind die Vereinigung direkter
Vergaben und aller Berechtigungen aus den Rollen des Benutzers.

Wenn alle drei `admin_*`-Optionen gesetzt sind, stellt Zelyra zusätzlich eine
optionale Browser-Administrationsseite unter dem konfigurierten Pfad bereit.
Sie ist durch die deklarierte Berechtigung und CSRF-Tokens geschützt.
Administratoren können dort Benutzer anlegen, Passwörter zurücksetzen,
Benutzer aktivieren oder deaktivieren sowie Rollen und
Rollenberechtigungen verwalten. Mit einer `active`-Spalte können deaktivierte
Benutzer sich nicht anmelden; ihre persistenten Sessions werden beim
Deaktivieren entfernt. Passwort-Resets entfernen ebenfalls alle persistenten
Sessions des betroffenen Benutzers. Die letzte Zuweisung der konfigurierten
Administrationsrolle und der letzte aktive Administrator sind geschützt.
Benutzerlöschung und Self-Service-Kontoverwaltung folgen später.

Die optionale Tabelle `auth_audit_log` protokolliert Login-, Logout-,
Passwort-, Benutzer-, Rollen- und Berechtigungsereignisse mit Akteur, Ereignis,
Ziel, Details und Zeitstempel. CLI-Rollenänderungen haben keinen
Sitzungs-Akteur und werden in `details` mit `source=cli` markiert. Die letzten
100 Einträge werden auf der Administrationsseite angezeigt. Mit
`zelyra audit inspect app.zyl` lässt sich das Protokoll lesbar anzeigen;
`zelyra audit export app.zyl --format json|csv` erzeugt einen begrenzten
Export. Das Standardlimit ist 100, maximal sind 10.000 Einträge erlaubt.
`zelyra audit verify` prüft, ob jeder Eintrag Ereignis, Details und Zeitstempel
enthält. `zelyra audit prune` verlangt `--before <timestamp>` und löscht ohne
das ausdrückliche Flag `--confirm` niemals Daten; die Bereinigung wird selbst
als Audit-Ereignis protokolliert.

Für eine manipulationssichtbare Historie ergänzt die `auth`-Definition
`audit_chain: true`. Die Audit-Tabelle benötigt dann zusätzlich eine `id`-Spalte
sowie `previous_hash` und `entry_hash`, üblicherweise `String(64)`. Zelyra berechnet kleingeschriebene
SHA-256-Hashes aus der kanonischen, durch `|` getrennten Nutzlast
`previous_hash|actor_user_id|event|target_user_id|details|created_at`; eine
fehlende ID wird als `NULL` geschrieben und der Zeitstempel verwendet
`YYYY-MM-DD HH:MM:SS`. Beim Anhängen wird die letzte Zeile gesperrt und die
Fachtransaktion geteilt. `audit verify` prüft Verbindungen und Hashes.
Bereinigen wird bei verketteten Protokollen abgelehnt, weil die Kette sonst
brechen würde.

Zuweisungen können ohne eigene SQL-Befehle über die CLI gepflegt werden. Das
Projekt wird vor jedem MariaDB-Schreibvorgang geprüft; wiederholte Grants sind
sicher:

~~~bash
DATABASE_URL='mariadb://user:password@127.0.0.1:3306/app' \
  zelyra auth role grant app.zyl 42 manager
DATABASE_URL='mariadb://user:password@127.0.0.1:3306/app' \
  zelyra auth role-permission revoke app.zyl manager customers.edit
~~~

Dieselben Schutzregeln sichern typisierte API-Handler:

~~~zelyra
api GET "/api/machines/{id}" {
    handler get_machine
    requires auth
    permits "machines.view"
    input { id: MachineId }
    output Machine
    errors { 404 NotFound }
}
~~~

Fehler bei geschützten APIs verwenden JSON mit `code` und `message`. Ein
Handler kann `Err("NotFound")` zurückgeben, um den passenden Status aus dem
deklarierten `errors`-Block zu wählen; nicht deklarierte Fehler führen zu 500.
JSON-Arrays können an typisierte Felder wie `Int[]` oder `MachineId[]` gebunden
werden. Verschachteltes JSON wird über deklarierte Records modelliert:

~~~zelyra
struct Address { city: String }
struct CustomerInput { name: String address: Address }

api POST "/customers" {
    handler echo_customer
    input { customer: CustomerInput }
    output CustomerInput
}
~~~

Unbekannte Record-Felder und fehlende Pflichtfelder werden abgelehnt. Im
Sprachkern unterstützen Arrays Literale, Indexzugriff, `len`, `append`,
`contains`, `first`, `last` und Verkettung mit `+`. Record-Literale und
geprüfter Feldzugriff stehen für verschachtelte Werte zur Verfügung:

~~~zelyra
customer = CustomerInput {
    name: "Anna"
    address: Address { city: "Berlin" }
}

print(customer.address.city)
~~~

Die Array-Iteration verwendet `for ... in`; die Schleifenvariable ist
unveränderlich und nur im Schleifenkörper sichtbar. `break` und `continue`
werden unterstützt.

Typisierte Maps sind für skalare Schlüssel verfügbar. Der Typ wird mit
`Map<Schlüssel, Wert>` angegeben, ein Literal mit `Map { ... }`:

~~~zelyra
prices: Map<String, Int> = Map { "standard": 10 "premium": 20 }
current = put(prices, "standard", 12)

match get(current, "standard") {
    Some(price) => {
        print(price)
    }
    None => {
        print(0)
    }
}
~~~

`get` liefert ein `Option`, `put` eine neue Map; `keys`, `values`, `contains`
und `len` arbeiten deterministisch. Bei der JSON-Konvertierung werden
`Map<String, Wert>` als JSON-Objekte behandelt; Maps mit anderen
Schlüsseltypen sind keine JSON-Maps.

Dieselbe Map mit String-Schlüsseln kann direkt über eine typisierte API
veröffentlicht werden:

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

Der Compiler prüft die JSON-Grenze, OpenAPI beschreibt die Map als Objekt mit
Ganzzahlwerten und der erzeugte TypeScript-Client verwendet
`Record<string, number>`. Ein nicht-String-Schlüssel wird für API-Eingaben oder
-Ausgaben abgelehnt, bevor der Server starten kann.

Einen mit Browsern und Node kompatiblen TypeScript-Client aus denselben
API-Deklarationen erzeugen:

~~~bash
zelyra doc examples/api_records.zyl --typescript > customer-client.ts
~~~

Der erzeugte Client verwendet die standardmäßige `fetch`-API, enthält
deklarierte Records und Tabellen als TypeScript-Typen und behandelt
Pfad-/Query-Parameter, JSON-Bodies, Bearer-Tokens, Response-Typen und
HTTP-Fehler. Deklarierte API-Fehlernamen sind über `ZelyraApiErrorCode`
verfügbar; `ZelyraApiError.fromResponse` liest Status, Code und Servermeldung
aus und bewahrt den unveränderten Response-Body auf.

API-Fehler können zusätzlich einen geprüften Payload enthalten. Der Payload-
Typ wird nach einem Doppelpunkt angegeben und muss dem Fehlertyp im `Result`
des Handlers entsprechen:

~~~zelyra
struct ValidationProblem {
    field: String
    message: String
}

api POST "/customers/validate" {
    handler validate_customer
    output Result<String, ValidationProblem>
    errors { 422 ValidationError: ValidationProblem }
}
~~~

Die Antwort behält `error.code` und `error.message` und ergänzt den
serialisierten Payload als `error.details`. OpenAPI enthält das Details-Schema;
der erzeugte Client stellt es über `ZelyraApiErrorPayloads` und das generische
Feld `ZelyraApiError.details` bereit. Bestehende ungetypte API-Fehler bleiben
kompatibel.

Browserzugriff ist standardmäßig deaktiviert. Wenn ein separates Frontend
eine API aufrufen soll, werden exakte Origins in der Projektkonfiguration
freigegeben:

~~~toml
[web]
allowed_origins = ["http://localhost:5173"]
allow_credentials = false
~~~

Zelyra beantwortet API-`OPTIONS`-Preflight-Anfragen automatisch und fügt
CORS-Header nur bei deklarierten API-Routen hinzu. Wildcard-Origins werden
abgelehnt; CORS umgeht weder Authentifizierung noch Berechtigungen. Aktiviere
Credentials nur für benötigte Browser-Session-Cookies; der Client muss dann
zusätzlich `credentials: "include"` verwenden.

Ein ausgeblendeter Button ist keine Sicherheitsgrenze. Berechtigungen müssen
serverseitig an der Aktion geprüft werden. Der Browser ist kreativ, besonders
wenn man ihm vertraut.

## 14. Capabilities

✅ Externe Fähigkeiten werden sichtbar deklariert:

~~~zelyra
fn load_machines() -> Machine[]
    uses Database
{
    return sql<Machine[]> {
        SELECT id, number, name FROM machines
    }
}
~~~

Bekannte Capabilities:

~~~text
Database Network FileSystem Environment Process Clock Random
~~~

Aufrufende Funktionen müssen benötigte Capabilities weiterführen. Projekte
können sie in `zelyra.toml` freigeben:

~~~toml
[capabilities]
database = true
network = false
~~~

Statische Prüfung und Runtime-Durchsetzung an Funktions-, nativen SQL-,
Formular-, CRUD- und Authentifizierungs-Datenbankgrenzen sind bei vorhandenen
Projektfreigaben implementiert. Eine vollständige Betriebssystem-Sandbox für
alle Capabilities ist noch nicht vorhanden.

Zwei sichere Host-APIs sind implementiert:

~~~zelyra
fn runtime_timestamp() -> Timestamp uses Clock {
    return now()
}

fn configured_mode() -> String? uses Environment {
    return env("ZELYRA_MODE")
}
~~~

now() benötigt Clock und liefert Unix-Epoch-Millisekunden. env(name) benötigt
Environment und liefert String?; eine fehlende Variable wird zu None. Werte
werden nicht automatisch protokolliert oder veröffentlicht. Netzwerk-, Datei-,
Prozess- und Zufalls-APIs sind weiterhin geplant.

Die Random-Capability erzeugt sichere Ganzzahlen:

~~~zelyra
fn dice_roll() -> Int uses Random {
    return random_int(1, 6)
}
~~~

Der Bereich ist auf beiden Seiten inklusiv. Ungültige Bereiche führen zu
einem Runtime-Fehler; Zufallswerte werden nicht implizit ausgegeben.
Prozessausführung ist nur über die folgende, ausdrücklich begrenzte API
verfügbar.

Die erste Network-Host-API ist `http_get`:

~~~zelyra
fn load_status(url: String) -> String uses Network {
    return http_get(url)
}
~~~

Projekte verwenden eine exakte Host-Allowlist und begrenzte Ressourcen:

~~~toml
[network]
allowed_hosts = ["127.0.0.1:8080", "api.example.com"]
timeout_ms = 5000
max_response_bytes = 1048576
~~~

Ohne `[network]` sind in einem Projekt keine Hosts erlaubt. Der Transport
unterstützt `http://` und `https://`; die Zertifikatsprüfung über Rustls ist
standardmäßig aktiviert. Es werden keine Redirects verfolgt und nur
erfolgreiche UTF-8-GET-Response-Bodies innerhalb der konfigurierten Grenzen
geliefert. Der Helper `http_get` bleibt die einfache GET-Komfort-API; für
Request-Header, Request-Bodies oder den Response-Status wird `http_request`
verwendet.

Typisierte Anfragen verwenden `http_request`:

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

Die Methode akzeptiert `GET`, `POST`, `PUT`, `PATCH`, `DELETE` und `HEAD`.
Header sind Strings im Format `Name: value`, der Body ist `String?`. Das
typisierte Ergebnis enthält `status: Int`, `headers: String[]` und
`body: String`. GET- und HEAD-Anfragen dürfen keinen Body enthalten.

JSON-Werte können in geprüfte Zelyra-Werte umgewandelt werden und umgekehrt.
Records und verschachtelte Felder werden gegen das deklarierte Schema geprüft:

~~~zelyra
struct Customer { name: String tags: String[] nickname: String? }

fn decode_customer(body: String) -> Customer {
    return json_decode<Customer>(body)
}

fn encode_customer(customer: Customer) -> String {
    return json_encode(customer)
}
~~~

`json_decode<Typ>(text)` benötigt genau ein Zieltypargument und unterstützt
Records, verschachtelte Records, Arrays, Optionen und Skalarwerte.
`json_encode` serialisiert dieselben Werte. Ungültiges JSON, Typfehler,
unbekannte Record-Felder und fehlende Pflichtfelder werden als ausdrückliche
Laufzeitfehler gemeldet.

Für einen vollständigen typisierten JSON-Request-/Response-Ablauf gibt es
`http_json` mit getrennten Request- und Response-Typargumenten:

~~~zelyra
struct CustomerCreate { name: String }
struct Customer { id: Int name: String }

fn create_customer(url: String, payload: CustomerCreate) -> Customer uses Network {
    return http_json<CustomerCreate, Customer>("POST", url, [], Some(payload))
}
~~~

Der Request-Record wird automatisch serialisiert und der Response-Body in den
Response-Record dekodiert. Wenn kein `Content-Type` angegeben ist, wird
`application/json` ergänzt. Nicht-2xx-Antworten sind ausdrückliche
Laufzeitfehler; der Helper liefert den dekodierten Wert und nicht die
Response-Header zurück.

Wenn die Response-Metadaten erhalten bleiben müssen, wird `http_result`
verwendet:

~~~zelyra
fn submit(url: String, payload: CustomerCreate) -> HttpResult<Customer> uses Network {
    return http_result<CustomerCreate, Customer>("POST", url, [], Some(payload))
}
~~~

`HttpResult<Response>` enthält `status: Int`, `headers: String[]`,
`body: String`, `data: Response?` und `error: HttpError?`. Erfolgreiche
2xx-Antworten setzen `data`; Nicht-2xx-Antworten setzen `error` mit Status,
Headern, Body und Meldung. Transportfehler und ungültiges Erfolgs-JSON bleiben
Laufzeitfehler.

Die Process-Capability stellt eine Befehls-API ohne Shell bereit:

~~~zelyra
fn render_report(input: String) -> String uses Process {
    return run_process("/usr/bin/printf", ["%s", input])
}
~~~

Für Projekte ist eine exakte Befehls-Allowlist erforderlich:

~~~toml
[process]
allowed_commands = ["/usr/bin/printf"]
timeout_ms = 5000
max_output_bytes = 1048576
~~~

Ohne `[process]` darf kein Befehl laufen. Die Umgebung des Kindprozesses wird
geleert, stdin geschlossen, Prozesse werden nach dem Timeout beendet und
stdout/stderr begrenzt. Shell-Ausführung, Umgebungsweitergabe,
Arbeitsverzeichnisse und Pipelines folgen später.

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

Alle vier APIs benötigen FileSystem. Lesen und Verzeichnislisten verwenden
read_roots; Schreiben und Löschen verwenden write_roots. Relative Pfade werden
ausgehend vom Projektverzeichnis aufgelöst, vorhandene Symlink-Ziele vor dem
Zugriff kanonisiert. Ohne filesystem-Abschnitt sind Projektlesezugriffe auf
das Projektverzeichnis begrenzt; Schreiben und Löschen sind gesperrt:

~~~toml
[filesystem]
read_roots = ["."]
write_roots = ["data"]
~~~

Die konfigurierten Verzeichnisse müssen bereits existieren. Ein neues
Schreibziel benötigt ein bereits existierendes Elternverzeichnis.

## 15. Contracts und Verify

🧪 Vor- und Nachbedingungen:

~~~zelyra
fn reserve(stock: Int, amount: Int) -> Int
    requires {
        amount > 0
        stock >= amount
    }
    ensures {
        result >= 0
        result == stock - amount
    }
{
    return stock - amount
}
~~~

`requires` wird vor dem Funktionskörper, `ensures` danach geprüft. In
`ensures` bezeichnet `result` den Rückgabewert.

~~~bash
zelyra verify examples/contracts.zyl
~~~

Mögliche Statuswerte:

~~~text
PROVEN
RUNTIME_CHECK
UNPROVEN
FAILED
~~~

Nur `PROVEN` bedeutet bewiesen. `RUNTIME_CHECK` trägt keinen falschen Schnurrbart
und behauptet nicht, Mathematik zu sein.

Der Verifier fasst außerdem Funktionsaufrufe mit begrenzter Tiefe zusammen.
Eine Callee mit mehreren Rückgabepfaden, etwa eine Absolutwertfunktion, liefert
ihre Pfadbedingungen an den aufrufenden Contract. `requires`-Bedingungen der
Callee werden nach Argumentsubstitution geprüft; `requires` des Aufrufers sind
Annahmen beim Beweis seiner `ensures`. Komplexe, rekursive oder nicht
auflösbare Fälle bleiben `RUNTIME_CHECK`.

Lokaler Zustandsfluss wird in diesen Zusammenfassungen berücksichtigt. Sowohl
`next: Int = value + 1` als auch die Kurzform `next = value + 1` mit
anschließendem `return next` werden wie eine direkte Rückgabe analysiert.
Einfache lineare Mutable-Zuweisungen wie `next = next + 1` werden ebenfalls
verfolgt. Statisch begrenzte Schleifen mit linearem Zähler werden entfaltet;
`break` beendet die aktuelle Schleife und `continue` startet ihren nächsten
Durchlauf als eigene symbolische Pfade. Nichtlineare Zuweisungen und
unbeschränkte Schleifen ohne bewiesene Invariante bleiben konservativ.

### Schleifeninvarianten

Eine `while`- oder unbedingte `loop`-Schleife kann eine oder mehrere explizite
Invarianten deklarieren:

~~~zelyra
while current > 0
    invariant { current >= 0 }
{
    current = current - 1
}
~~~

Der Verifier prüft die Invariante beim Eintritt und nach unterstützten
Körperpfaden. Eine bewiesene Invariante kann eine ansonsten unbeschränkte
lineare `while`-Schleife zusammenfassen; eine unbedingte `loop`-Schleife kann
sie mit einem modellierten `break`-Austritt verwenden. Die Runtime prüft sie
vor und nach jedem Durchlauf. Nicht unterstützte oder nicht beweisbare
Invarianten bleiben konservativ und erzeugen kein `PROVEN`-Ergebnis.

`zelyra verify` meldet jede deklarierte Invariante separat, nach den
`ensures`-Ergebnissen einer Funktion. Die Indizes der Invarianten beginnen bei
null:

~~~text
PROVEN [V-001]: reduce.ensures[0] (src/reduce.zyl:3:5-3:21)
PROVEN [V-001]: reduce.invariant[0] (src/reduce.zyl:7:21-7:33)
FAILED [V-004]: reduce.invariant[1] (src/reduce.zyl:8:21-8:34)
~~~

Jedes Ergebnis enthält einen stabilen Code und einen Quellbereich als
`(datei.zyl:startzeile:startspalte-endzeile:endspalte)`. Die Codes sind
`V-001` (`PROVEN`), `V-002` (`RUNTIME_CHECK`), `V-003` (`UNPROVEN`) und
`V-004` (`FAILED`). Für IDEs und CI kann `zelyra verify app.zyl --json`
verwendet werden; die JSON-Ausgabe enthält dieselben Ergebnisdaten, eine
verständliche `message`, ein optionales `counterexample`-Objekt und ein
strukturiertes `location`-Objekt. Ein Gegenbeispiel wird nur ausgegeben, wenn
eine begrenzte Suche einen kleinen linearen Integerzeugen sicher bestätigt.
Die aktuelle Suche umfasst bis zu drei lineare Variablen im Bereich
`-32..=32`, auch bei fehlgeschlagenen Schleifeninvarianten; sonst ist der Wert
`null`. Die Textausgabe zeigt außerdem für jedes Ergebnis eine Erklärung und
einen Quellzeilenausschnitt mit Caret-Marker.

`FAILED` bedeutet, dass die Invariante auf einem möglichen analysierten Pfad
falsch ist oder vom Schleifenkörper nicht erhalten bleibt. `RUNTIME_CHECK`
bedeutet, dass eine Laufzeitprüfung erforderlich ist, weil der symbolische
Verifier den Beweis nicht vollständig führen kann. Nur `PROVEN` ist ein
mathematischer Beweis.

## 16. Konfiguration und Geheimnisse

Projektkonfiguration gehört in `zelyra.toml`, Geheimnisse nicht:

~~~toml
[project]
name = "maschinenverwaltung"
version = "0.1.42"
zelyra = "0.1"

[capabilities]
database = true
network = false
~~~

Verbindungen derzeit über geschützte Umgebungsvariablen:

~~~bash
export DATABASE_URL='mariadb://user:password@127.0.0.1:3306/zelyra_demo'
~~~

Regeln:

- `.env` niemals committen;
- Produktionszugänge nie in Beispiele schreiben;
- Geheimnisse nicht loggen;
- getrennte Datenbanken für Entwicklung, Tests und Produktion verwenden;
- destruktive Tests niemals gegen Produktion ausführen.

### Einfacher Einstieg, optionale Möglichkeiten

Für den Einstieg ist keine Feature-Konfiguration erforderlich. Erweiterte
Projektbereiche können in `zelyra.toml` ausgewählt werden; umgebungsabhängige,
nicht geheime Überschreibungen gehören in `.env` oder die Prozessumgebung:

~~~toml
[features]
api = false
crud = false
~~~

Unterstützt werden `web`, `api`, `crud`, `auth` und `audit`. Die Priorität ist
Prozessumgebung, `.env`, `zelyra.toml` und danach sichere Standardwerte. Die
wirksamen Werte können ohne Anzeige von Secrets geprüft werden:

~~~bash
zelyra config main.zyl --format=json
~~~

Wenn der Quellcode einen deaktivierten Bereich verwendet, meldet der Compiler
eine stabile Feature-Diagnose. Capabilities, Typprüfung, SQL-Prüfung und
Sicherheitsregeln können damit nicht abgeschaltet werden. Diese optionale
Komfortschicht ist keine zusätzliche Pflicht für einfache Projekte. Die
vollständige [Referenz für Umgebung und Konfiguration](../../env.md) führt alle
unterstützten Einstellungen auf und muss vor dem Commit einer neuen
Einstellung aktualisiert werden.

🗺️ Typisierte Connections, verschlüsselte Secret Stores, SMTP-Assistent und
ODBC-Erkennung sind geplant.

## 17. Diagnosen und Fehlersuche

Zelyra möchte Fehler so erklären, dass man nicht erst eine archäologische
Ausgrabung im Stacktrace beginnen muss.

Quellcode prüfen:

~~~bash
zelyra check app.zyl
~~~

Typische Fehlerklassen:

- unbekannter Name oder Typ;
- Zuweisung an unveränderlichen Wert;
- unvollständiges Pattern Matching;
- unbekannte Tabelle oder Spalte;
- fehlender SQL-Parameter;
- falsche Ergebnisstruktur;
- fehlende Capability;
- ungültiges Formularfeld;
- nicht erfüllter Contract.

Wenn `DATABASE_URL` fehlt, funktionieren reine Sprachprüfungen weiterhin.
Datenbankoperationen melden den fehlenden Zugriff kontrolliert.

## 18. Testen und Mitentwickeln

Vor jedem Commit:

~~~bash
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
~~~

Ein Sprachfeature ist erst fertig, wenn es besitzt:

- dokumentierte Syntax;
- AST/HIR-Unterstützung;
- statische Prüfung;
- verständliche Diagnosen;
- positive Tests;
- negative Tests;
- ein ausführbares Beispiel;
- aktualisierte deutsche und englische Dokumentation.

Tests, die nur deshalb grün sind, weil sie nie liefen, sind Dekoration.

## 19. Was als Nächstes kommt

Die wichtigsten geplanten Bereiche:

- typisierte, vollständig anpassbare View-Komponenten;
- E-Mail-Vorlagen und SMTP;
- Benachrichtigungszentrum;
- Hintergrundaufgaben und transaktionale Outbox;
- Activity-, Audit- und technische Logs;
- typisierte Connections und Secret Provider;
- ODBC und externe Read-only-Datenbanken;
- umfangreichere fachliche Fehlerwerte über typisierte API-Payloads hinaus und
  weitergehende Request-/Response-Verarbeitung zur Laufzeit;
- Abbruch und Datenbank-Pool-Integration für strukturierte Nebenläufigkeit;
- weitergehende formale Verifikation;
- Optimierungsmodelle für reale Planungsprobleme.

Zelyra soll den Standardfall kurz halten und beim Sonderfall nicht plötzlich
die Tür abschließen:

> **Automatisch, wenn möglich. Anpassbar, wenn nötig. Überall geprüft.**

Und jetzt: eine Tabelle bauen, SQL lesen, Backup prüfen. In dieser Reihenfolge.

## 20. KI-native Entwicklung mit Zelyra

Zelyra ist KI-nativ, aber nicht KI-abhängig. Menschen und KI-Systeme können
denselben verständlichen `.zyl`-Quellcode schreiben. Maßgeblich bleibt der
Compiler:

> **Die KI schreibt. Zelyra prüft.**

Kein KI-Anbieter ist Bestandteil des Kompilierens. Der Compiler sendet keinen
Quellcode an externe Dienste. Die Maschinenschnittstellen sind versioniert und
anbieterneutral, sodass auch lokale Werkzeuge sie verwenden können.

### Was jetzt verfügbar ist

- ✅ **Implementiert:** deterministische, versionierte JSON-Diagnosen;
- ✅ **Implementiert:** stabile Fehlercodes und Source-Spans;
- ✅ **Implementiert:** schreibgeschützter strukturierter Projektkontext;
- ✅ **Implementiert:** deterministische Formatierung mit `zelyra fmt` und
  `--check` für CI; Kommentare und opake SQL-/HTML-Blöcke bleiben erhalten;
- ✅ **Implementiert:** menschenlesbare Ausgabe bleibt Standard;
- 🧪 **Experimentell:** die aktuelle JSON-Schnittstelle hat Schema-Version `1`
  und unterstützt `check` und `context`;
- ✅ **Implementiert:** Ausdrucks-Typed-Holes mit `_` und
  Kontextdiagnosen; baubare Befehle lehnen unvollständigen Code ab;
- 🗺️ **Geplant:** umfassendere Typed Holes, semantische Änderungen,
  Wirkungsanalyse und der reproduzierbare KI-Benchmark;
- ❌ **Nicht verfügbar:** automatische Änderungen an Produktionssystemen,
  automatische Berechtigungserweiterungen oder an einen KI-Dienst delegierte
  Compilerentscheidungen.

Prüfe ein Programm weiterhin standardmäßig menschenlesbar:

~~~bash
zelyra check examples/fibonacci.zyl
~~~

Quellcode kanonisch formatieren oder die Formatierung ohne Schreiben prüfen:

~~~bash
zelyra fmt examples/fibonacci.zyl
zelyra fmt examples/fibonacci.zyl --check
~~~

Für Werkzeuge kann JSON ausdrücklich angefordert werden:

~~~bash
zelyra check examples/fibonacci.zyl --format=json
~~~

Eine erfolgreiche Ausgabe hat aktuell diese Form:

~~~json
{
  "schema_version": "1",
  "command": "check",
  "success": true,
  "diagnostics": []
}
~~~

Fehler verwenden stabile Codes wie `E-LEX-001`, `E-PARSE-001` und
`E-SQL-004`. JSON wird ausschließlich auf `stdout` ausgegeben; technische
Logs gehören auf `stderr`. Eine fehlgeschlagene Prüfung liefert einen Exit-Code
ungleich null. Offsets sind UTF-8-Byte-Offsets, Zeilen und Spalten beginnen bei
eins, und wiederholte Prüfungen erzeugen byte-identische Ausgaben.

Den vom Compiler verstandenen, schreibgeschützten Projektaufbau kann man so
inspizieren:

~~~bash
zelyra context examples/auth_crud_api.zyl --format=json
~~~

Die Kontextantwort enthält Projekteinstieg und nur Deklarationen, die der
aktuelle Compiler tatsächlich versteht, darunter Tabellen, Felder, Seiten mit
typisierten Datenbindungen, CRUD-Ressourcen, Formulare, APIs und Source-Spans.
Sie verbindet sich nicht mit
MariaDB, führt keine E-Mail aus, gibt keine Zugangsdaten aus und enthält keine
gerenderten vertraulichen Inhalte.

Quelltextabhängigkeiten eines Programms lassen sich deterministisch prüfen:

~~~bash
zelyra impact examples/auth_crud_api.zyl --format=json
zelyra impact examples/auth_crud_api.zyl --symbol table:customers --format=json
~~~

Die Wirkungsantwort meldet quelltextbasierte Tabellen, SQL, Formulare, CRUD-
Ressourcen, Views, APIs, Berechtigungen, Contracts und eine deterministische
`references`-Kantenliste für bekannte Beziehungen. E-Mail-, Job-, Test- und
Live-Schemaauswirkungen bleiben ausdrücklich leer oder nicht verfügbar; der
Befehl verbindet sich nie mit MariaDB.
Mit `--symbol <kind:name>` kann die Ausgabe auf einen bekannten Knoten wie
`table:customers` fokussiert werden. Die fokussierte Antwort enthält nur direkt
verbundene Referenzen und zugehörige Knoten-IDs. Unbekannte Knoten liefern
`E-IMPACT-001` und einen Exit-Code ungleich null.

Eine validierte Symbol-Umbenennung kann ohne Änderung des Quelltexts
vorschaut werden:

~~~json
{
  "schema_version": "1",
  "entry": "examples/fibonacci.zyl",
  "expected_source_fingerprint": "fnv1a64:18f35ecb3e2f99c4",
  "operations": [
    {"kind": "rename", "symbol": "function", "from": "fibonacci", "to": "fib"}
  ]
}
~~~

Als `change.json` speichern und ausführen:

~~~bash
zelyra edit --format=json change.json
~~~

Die Anfrage ist versioniert und darf nur auf eine existierende `.zyl`-Datei
innerhalb der aufgelösten Zelyra-Projektwurzel zeigen. Quelltext vor und nach
der Änderung muss die Compilerprüfungen bestehen. Das Ergebnis meldet die
genauen Token-Spans und einen deterministischen Quelltext-Fingerprint. Für
`--apply` muss die Anfrage den Fingerprint aus der Vorschau enthalten; so wird
eine zwischenzeitlich geänderte Datei nicht überschrieben. Ohne den
ausdrücklichen `--apply`-Schalter bleibt es eine Vorschau:

~~~bash
zelyra edit --format=json --apply change.json
~~~

Vor dem atomaren Ersetzen wird der Quelltext erneut geparst und vollständig
geprüft; ein ungültiger oder semantisch unsicherer Vorschlag kann daher nicht
geschrieben werden.

Umbenennungen von Funktionen, Typen und Records sind AST-basiert:
Deklarationen und bekannte Referenzen werden umbenannt, während lokale
Bindungen mit demselben Namen unverändert bleiben. Tabellen-, View-, Form- und
CRUD-Deklarationen sowie ihre strukturierten Referenzen werden ebenfalls
unterstützt. Tabellenumbenennungen aktualisieren geprüfte SQL-Tabellenpositionen,
Komponenten-Umbenennungen aktualisieren die Deklaration sowie bekannte öffnende
und schließende Komponententags in HTML-Bodies. Tabellenumbenennungen
aktualisieren geprüfte SQL-Tabellenpositionen, lassen aber Literale, Kommentare,
Parameter und HTML unverändert.

### Sichere Automatisierungsgrenze

Generierter Code muss Compiler und Tests bestehen. Einer KI darf nicht vertraut
werden, nur weil ihr Ergebnis plausibel aussieht. Sie darf nicht unbemerkt
Capabilities hinzufügen, Diagnosen abschwächen, Tests deaktivieren, Geheimnisse
offenlegen oder destruktive Schemaänderungen freigeben. Für riskante Datenbank-
und Sicherheitsoperationen bleibt eine menschliche Freigabe erforderlich.

Die nächste geplante Ausbaustufe umfasst reichhaltigere typisierte Lücken und
eine vollständige Wirkungsanalyse. Sie erweitern das gemeinsame versionierte
JSON-Format, ersetzen es aber nicht.
Benchmark-Ergebnisse werden erst nach
reproduzierbaren Versuchen veröffentlicht; dieses Handbuch enthält keinen
erfundenen Vergleich.
