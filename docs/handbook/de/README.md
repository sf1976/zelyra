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
maschinenverwaltung --mariadb` verwenden. Dadurch entstehen `.env.example`,
`Dockerfile` und `docker-compose.mariadb.yml`. Mit `ZELYRA_WEB_PORT` in `.env`
wird der Port des internen Zelyra-Servers und seines lokalen veröffentlichten
Ports gewählt.

Nutzer, die Rust nicht installieren möchten, können das vorgefertigte Linux-
oder Windows-Archiv von der [GitHub-Releases-Seite](https://github.com/sf1976/zelyra/releases)
herunterladen. Jedes Archiv enthält eine SHA-256-Prüfsumme, die CLI, beide
README-Sprachen und die Lizenzhinweise.

Eine minimale `zelyra.toml`:

~~~toml
[project]
name = "maschinenverwaltung"
version = "0.1.37"
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
| `zelyra audit verify app.zyl` | Pflichtfelder im Audit prüfen |
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

Der Compiler verlangt in einem benannten View genau einen `<slot />`. Der
Seiteninhalt wird vor dem Routing eingesetzt; Authentifizierung,
Autorisierung und Escaping bleiben aktiv. Typisierte selbstschließende
Komponenten mit deklarierten Properties sind ebenfalls verfügbar:

~~~zelyra
component Badge {
    props { text: String }
    html { <span class="badge">{text}</span> }
}

page "/status" {
    html { <Badge text="Ready" /> }
}
~~~

Benannte Slots, verschachtelte Komposition, Themes und CRUD-View-
Überschreibungen sind geplante Erweiterungen.

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
version = "0.1.37"
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
