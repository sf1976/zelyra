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

Eine minimale `zelyra.toml`:

~~~toml
[project]
name = "maschinenverwaltung"
version = "0.1.0"
zelyra = "0.1"

[capabilities]
database = true
network = false
~~~

Die wichtigsten Befehle:

| Befehl | Zweck |
|---|---|
| `zelyra check app.zyl` | lexen, parsen, Namen und Typen prüfen |
| `zelyra build app.zyl` | Anwendung prüfen und bauen |
| `zelyra run app.zyl` | Programm ausführen |
| `zelyra serve app.zyl` | HTTP-Server starten |
| `zelyra verify app.zyl` | Contracts klassifizieren |
| `zelyra doc app.zyl --openapi` | OpenAPI-Dokument erzeugen |
| `zelyra db inspect app.zyl` | Ist-Schema lesen |
| `zelyra db plan app.zyl` | Schemaänderungen anzeigen |
| `zelyra db apply app.zyl` | geprüften Plan anwenden |

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
        sql {
            INSERT INTO customers (name, email)
            VALUES (:name, :email)
        }

        redirect "/customers"
    }
}
~~~

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
/machines?sort=number&order=asc
/machines/new
/machines/42/edit
~~~

🗺️ Vollständig eigene typisierte Komponenten und feingranulare View-Overrides
sind Teil der weiteren View-Roadmap.

## 13. Authentifizierung und Berechtigungen

🧪 Zelyra unterstützt Argon2-Login, persistente MariaDB-Sessions, Logout,
Routenschutz und datenbankgestützte Berechtigungsprüfungen.

~~~zelyra
auth users {
    table: users
}

page "/admin" {
    requires auth
    permits "machines.manage"

    html {
        <h1>Maschinenverwaltung</h1>
    }
}
~~~

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

Die statische Prüfung ist implementiert. Eine vollständige
Betriebssystem-Sandbox für alle Capabilities ist noch nicht vorhanden.

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
version = "0.1.0"
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
- weitergehende API-Fehler-/Authentifizierungsbehandlung und Request-/Response-Verarbeitung zur Laufzeit;
- strukturierte Nebenläufigkeit;
- weitergehende formale Verifikation;
- Optimierungsmodelle für reale Planungsprobleme.

Zelyra soll den Standardfall kurz halten und beim Sonderfall nicht plötzlich
die Tür abschließen:

> **Automatisch, wenn möglich. Anpassbar, wenn nötig. Überall geprüft.**

Und jetzt: eine Tabelle bauen, SQL lesen, Backup prüfen. In dieser Reihenfolge.
