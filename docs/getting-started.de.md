# Mit Zelyra starten

Deutsch · [English](getting-started.md)

Dieser Leitfaden führt vom Repository-Checkout zum ersten Zelyra-Programm,
zur ersten Web-Seite, zum ersten Schema, zu nativem SQL und zur ersten
Formularvalidierung. Er beschreibt die aktuelle Implementierung von Zelyra
0.1 und nicht nur die langfristige Sprachvision.

## Was ist Zelyra?

Zelyra ist eine statisch typisierte Sprache für Anwendungen, die mit
Geschäftsdaten arbeiten. Die Sprache soll zusammenführen:

- allgemeine Programmierung;
- Definitionen von Datenbankschemata;
- geprüftes natives SQL;
- Web-Seiten und HTTP-Routing;
- Formulare und Validierung;
- erste CRUD-Listen, Details, Create-/Edit-Formulare, konfigurierbare Spalten,
  Löschaktionen und reversible Wiederherstellung;
- erste Versionen von CRUD, Authentifizierung, Autorisierung, Contract-
  Verifikation, typisierten API-Deklarationen und OpenAPI-Erzeugung sind
  vorhanden; weitergehende Ausbaustufen sind weiterhin geplant.

Das zentrale Ziel ist, wichtige Informationen nur einmal zu definieren. Ein
Pflichtfeld mit maximaler String-Länge in einer Tabelle kann beispielsweise
auch die Grundlage für die Formularvalidierung bilden. Compiler und Werkzeuge
sollen diese Verbindung ausdrücklich und prüfbar machen.

Zelyra ist weder ein Low-Code-Editor noch eine reine ORM-Sprache. Es bleibt
eine vollständige Programmiersprache. Entwickler können Funktionen, natives
SQL, eigene Seiten, Aktionen und Geschäftsregeln schreiben.

## Was funktioniert in 0.1?

Das aktuelle Repository enthält:

- Rust-Lexer, Parser, AST, HIR, Type Checker, Interpreter und CLI;
- standardmäßig unveränderliche Variablen, Funktionen, Ausdrücke,
  Bedingungen, Schleifen, Option, Result, nominale Typen und Pattern Matching;
- MariaDB als Standard-Backend;
- Schema-Planung und DDL-Unterstützung für MariaDB, SQLite und PostgreSQL;
- MariaDB-Inspektion, Schema-Anwendung und native SQL-Ausführung;
- geprüfte SQL-Blöcke mit benannten Parametern;
- einen ersten eingebauten HTTP-Server und GET-Router;
- einen ersten schemaabhängigen Formular-Parser und Validator;
- erste CRUD-Listen mit MariaDB-Suche, Filtern, Sortierung, Pagination,
  reversibler Archivierung, Wiederherstellung und CSRF-Schutz.
- ein datenbankgestützter Login mit Argon2-Passwortprüfung, persistenten
  HttpOnly-Sessions, Logout, datenbankgestützten Berechtigungen und
  geschützten Routen.
- typisierte API-Deklarationen mit optionalen ausführbaren Handlern,
  OpenAPI-Erzeugung, Authentifizierungs- und Berechtigungssperren sowie
  strukturierten JSON-Fehlern und deklarierter `Result`-Fehlerzuordnung
  einschließlich typisierter API-Arrays und verschachtelter JSON-Objekte über
  `struct`-Records;
- typisierte API-Fehler-Payloads mit `error.details`, OpenAPI-Schemas und
  TypeScript-Payload-Typen;
- Browser-API-Integration mit exakter CORS-Origin-Allowlist und automatischer
  `OPTIONS`-Preflight-Verarbeitung;
- API-Validierung von Medientypen und Body-Größe mit sicheren Standard-
  Response-Headern;
- einen TypeScript-Client-Generator ohne zusätzliche Abhängigkeiten mit
  deklarierten API-Fehlercodes und strukturierter HTTP-Fehlerauswertung;
- Array-Literale, Indexzugriff, `len`, `append`, `contains`, `first`, `last`
  und Array-Verkettung mit `+`;
- strukturierte `for ... in`-Iteration über Arrays mit `break` und `continue`;
- Record-Literale und geprüfter Feldzugriff für verschachtelte Werte;
- erste Capability-Deklarationen, Weitergabe über Funktionsaufrufe und
  statische Durchsetzung von `Database` für natives SQL sowie Projektfreigaben
  in `zelyra.toml`.
- Runtime-Durchsetzung von Funktions-Capabilities und nativen SQL-Grenzen beim
  Ausführen mit Projektfreigaben;
- sichere Host-APIs für Clock und Environment: now() liefert einen Unix-Epoch-
  Zeitstempel in Millisekunden, env(name) liefert String?; beide benötigen
  Funktionsdeklaration und Projektfreigabe;
- sichere Zufallszahlen mit random_int(min, max) über die Random-Capability;
  die Grenzen sind inklusiv, ein ungültiger Bereich ist ein Runtime-Fehler;
- read_text(path), write_text(path, content), delete_file(path) und
  list_dir(path) über FileSystem mit Projektpfadgrenzen;
- http_get(url) über Network mit ausdrücklicher Host-Allowlist sowie
  begrenzter Zeit und Antwortgröße;
- http_request(method, url, headers, body) mit typisierten HttpResponse-
  Ergebnissen;
- json_encode(value) und json_decode<Typ>(text) für geprüfte JSON-Konvertierung
  von Records, Arrays, Optionen und Skalarwerten;
- http_json<Request, Response>(...) für automatische typisierte JSON-HTTP-
  Anfragen und -Antworten;
- http_result<Request, Response>(...) mit typisierten `HttpResult<Response>`-
  Werten für Status, Header, Body, Daten und strukturierte HTTP-Fehler;
- run_process(command, args) über Process ohne Shell, mit exakter
  Befehls-Allowlist und begrenzter Ausführung;
- zur Laufzeit geprüfte Funktions-Contracts mit `requires` und `ensures`.
- einen ersten Baustein für strukturierte Nebenläufigkeit mit `parallel` und
  `await`; jeder Branch verwendet einen unveränderlichen Umgebungs-Snapshot,
  bevor alle Branches vor der Fortsetzung zusammengeführt werden.
- erste `zelyra verify`-Unterstützung für symbolische Integer-Pfade, begrenzte
  Schleifen, `break`/`continue` und explizite `while`-Schleifeninvarianten.
  Nur `PROVEN` ist ein Beweis; nicht unterstützte Fälle bleiben
  `RUNTIME_CHECK` oder `UNPROVEN`.

Noch nicht vollständig sind: Rollenverwaltung, Login-Drosselung,
Passwortverwaltungs-Kommandos, umfangreichere fachliche
Fehlerwerte, weitergehende formale Verifikation,
Betriebssystem-Integration für Capabilities, Abbruch laufender Branches,
Datenbank-Pool-Integration für parallele Arbeit und Produktionspaketierung.

## 1. Voraussetzungen

Für die einfachste Installation aus dem Quellcode werden benötigt:

- Bash unter Linux/macOS oder PowerShell unter Windows;
- Git;
- curl, falls Rust noch nicht installiert ist;
- eine Netzwerkverbindung für die erste Installation der Rust-Toolchain.

Die Installer bauen Zelyra für den aktuellen Benutzer und verwenden weder sudo
noch Administratorrechte. Falls Rust fehlt, wird es automatisch installiert.
Die Quellcode-Installer benötigen für die erste Toolchain-Installation eine
Internetverbindung. Ein eigenständiger Release-Installer ohne Rust bleibt
geplant.

Apache ist nicht erforderlich. Für Sprachkern-, Web- und lokale
Formularbeispiele wird kein Datenbankserver benötigt. MariaDB wird nur
benötigt, wenn Datenbankoperationen gegen MariaDB ausgeführt werden sollen.

## 2. Herunterladen und installieren

Repository klonen:

~~~bash
git clone https://github.com/sf1976/zelyra.git
cd zelyra
~~~

Befehl installieren:

~~~bash
./install.sh
~~~

Der Installer ist wiederholbar und benutzerlokal. Nützliche Optionen sind:

~~~bash
./install.sh --help
./install.sh --dry-run --root "$HOME/.local"
./install.sh --check
./install.sh --uninstall
~~~

Mit `--no-rustup` wird bei fehlendem Rust nur eine klare Fehlermeldung
ausgegeben, `--no-path` unterdrückt PATH-Hinweise. Mit `--root PATH` oder der
Umgebungsvariable `ZELYRA_INSTALL_ROOT` lässt sich ein anderes
benutzerbezogenes Installationsverzeichnis wählen. Mit `--offline` werden nur
bereits gecachte Rust-Abhängigkeiten verwendet. Ein veralteter oder defekter
`cargo`-PATH-Eintrag wird erkannt und niemals als ausführbarer Compiler
verwendet.

Unter Windows PowerShell aus dem Repository-Verzeichnis verwenden:

~~~powershell
.\install.ps1
~~~

Alternativ kann `install.cmd` doppelt angeklickt oder aus `cmd.exe` gestartet
werden. Falls die PowerShell-Ausführungsrichtlinie das Script blockiert, gilt
für das aktuelle Terminal:

~~~powershell
Set-ExecutionPolicy -Scope Process Bypass
.\install.ps1
~~~

Der Windows-Installer verwendet das benutzerbezogene Verzeichnis
`%LOCALAPPDATA%\Zelyra\bin` und ergänzt den Benutzer-PATH. Ein
Administratorpasswort ist nicht erforderlich.

Das Script baut die CLI reproduzierbar mit gesperrten Abhängigkeiten im
Release-Modus und installiert sie unter:

~~~text
~/.local/bin/zelyra
~~~

Für veröffentlichte Linux-x86_64- und Windows-x86_64-Releases ist keine
Rust- oder Cargo-Installation nötig. Unter Linux den exakten Tag angeben; das
passende Archiv wird über HTTPS geladen, per SHA-256 geprüft und atomar
ausgetauscht:

~~~bash
./install.sh --release v0.1.45
~~~

Unter Windows:

~~~powershell
.\install.ps1 -Release v0.1.45
~~~

Der Release-Modus unterstützt derzeit Linux x86_64 und Windows x86_64. macOS
verwendet weiterhin den Quellcode-Installer, bis ein natives Release-Ziel
veröffentlicht ist. Die Archive enthalten die CLI, beide README-Sprachen und
die Lizenzhinweise.

Falls die Shell zelyra nicht findet, das Verzeichnis für die aktuelle Shell
zum PATH hinzufügen:

~~~bash
export PATH="$HOME/.local/bin:$PATH"
~~~

Für eine dauerhafte Einstellung gehört der Export in die Startdatei der
verwendeten Shell, beispielsweise ~/.bashrc oder ~/.zshrc.

Installation prüfen:

~~~bash
zelyra --help
~~~

Prüfen, ob ein Projekt startbereit ist:

~~~bash
zelyra doctor examples/machine_management.zyl
~~~

`doctor` prüft Quellcode und Schema, meldet die konfigurierte
Datenbankverbindung ohne Änderungen und testet, ob der Standard-Webport frei
ist. Eine fehlende `DATABASE_URL` wird als Warnung gemeldet; eine nicht
erreichbare konfigurierte Datenbank oder ein ungültiges Projekt als Fehler.

Für CI- und IDE-Integrationen kann eine maschinenlesbare Ausgabe angefordert
werden:

~~~bash
zelyra doctor examples/machine_management.zyl --json
~~~

Das JSON-Dokument enthält `version`, `project`, `status`, `warnings` und ein
`checks`-Array. Zugangsdaten aus `DATABASE_URL` werden niemals ausgegeben.

Die Installation benötigt kein Kontopasswort. Den Installer nicht als root
ausführen, außer es gibt einen gesonderten Grund für eine systemweite
Paketierung.

## 3. Erstes Programm ausführen

Fibonacci-Beispiel ausführen:

~~~bash
zelyra run examples/fibonacci.zyl
~~~

Erwartete Ausgabe:

~~~text
55
~~~

Der Quellcode ist normales Zelyra:

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

Prüfen, ohne auszuführen:

~~~bash
zelyra check examples/fibonacci.zyl
~~~

Der Check-Befehl lexed, parst, löst Namen auf und prüft die Typen. Er sollte
folgendes melden:

~~~text
ok: examples/fibonacci.zyl
~~~

## 4. Neues Projekt erstellen

Projektverzeichnis erstellen:

~~~bash
zelyra new meine-app
cd meine-app
zelyra run main.zyl
~~~

Das erzeugte Projekt enthält eine minimale zelyra.toml und main.zyl. Das
aktuelle Verzeichnis kann stattdessen initialisiert werden:

~~~bash
mkdir andere-app
cd andere-app
zelyra init
~~~

Für eine fertige lokale MariaDB- und Webserver-Vorlage verwenden:

~~~bash
zelyra new meine-app --mariadb --web-port 8080 --host-port 18080 --db-host-port 3307
cd meine-app
zelyra setup .
docker compose --env-file .env -f docker-compose.mariadb.yml up -d --build
set -a; . ./.env; set +a
zelyra doctor main.zyl --env-file .env --port 18080
zelyra db setup main.zyl
~~~

Die erzeugte Compose-Datei startet MariaDB und den Zelyra-Webserver. Die mit
`--mariadb` verwendete Option `--web-port 8080` wählt beim Erstellen den
internen Serverport; `--host-port 18080` wählt den lokal veröffentlichten
Web-Port und `--db-host-port 3307` den lokal veröffentlichten MariaDB-Port.
Später können `ZELYRA_WEB_PORT`, `ZELYRA_HOST_PORT` und
`ZELYRA_DB_HOST_PORT` in `.env` unabhängig geändert werden. Nach dem obigen
Beispiel ist `http://127.0.0.1:18080` erreichbar. Wenn die Host-Port-Optionen
fehlen, wählen neue Projekte bei belegten Standardports automatisch freie
Host-Ports. Ausdrücklich gesetzte Host-Port-Optionen werden niemals still
geändert. Die Vorlage ist für lokale
Entwicklung gedacht; für Produktion Secret-Manager und TLS verwenden.

`zelyra setup .` erzeugt aus der Vorlage eine geschützte `.env` mit zufälligen
lokalen MariaDB-Passwörtern und schützt die Datei unter Unix. Eine vorhandene
`.env` wird nicht überschrieben, und Zugangsdaten werden nicht ausgegeben. Die
lokalen Zugangsdaten nicht als Produktions-Secrets verwenden.

Muss Setup eine fehlende `.env` erzeugen, wählt es freie veröffentlichte Web-
und MariaDB-Ports, wenn die Vorlagen-Standardports belegt sind. Mit
`zelyra setup . --host-port 18080 --db-host-port 3308` können genaue Ports
vorgegeben werden; nicht verfügbare explizite Ports werden abgelehnt und bei
einer vorhandenen `.env` niemals angewendet.

Für eine fertige CRUD-Anwendung statt der minimalen Willkommensseite das
MariaDB-CRUD-Template verwenden:

~~~bash
zelyra new maschinenverwaltung --template mariadb-crud \
    --web-port 8080 --host-port 18080 --db-host-port 3307
cd maschinenverwaltung
zelyra setup .
docker compose --env-file .env -f docker-compose.mariadb.yml up -d --build
set -a; . ./.env; set +a
zelyra db setup main.zyl
~~~

Es enthält Abteilungen, Maschinen, eine Beziehung, schemaabhängige Formulare,
CRUD-Seiten, Suche, Filter, Pagination und eigene Aktionen.

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

Die aktuelle Projektdatei ist bewusst klein:

~~~toml
[project]
name = "meine-app"
version = "0.1.45"
zelyra = "0.1"

[capabilities]
database = true
network = false
~~~

Browserzugriff auf APIs ist standardmäßig deaktiviert. Aktiviere nur die
exakten Origins deiner Anwendung:

~~~toml
[web]
allowed_origins = ["http://localhost:5173"]
allow_credentials = false
~~~

Wildcard-Origins werden abgelehnt. Credentialed Browser-Anfragen benötigen
`allow_credentials = true` und zusätzlich die clientseitige Einstellung
`credentials: "include"`.

Host-APIs benötigen ausdrücklich freigegebene Berechtigungen. Ergänze die
für deine Funktionen benötigten Freigaben und starte anschließend das Beispiel:

~~~toml
[capabilities]
clock = true
environment = true
~~~

Schreib- und Löschzugriffe benötigen zusätzlich eine Allowlist. Die Pfade
beziehen sich auf das Projektverzeichnis und die Verzeichnisse müssen bereits
existieren:

~~~toml
[filesystem]
read_roots = ["."]
write_roots = ["data"]
~~~

~~~bash
zelyra run examples/host_apis.zyl
~~~

Das Beispiel gibt den aktuellen Unix-Epoch-Zeitstempel und den optionalen
Umgebungswert ZELYRA_MODE aus. Fehlende Werte werden als None dargestellt.

## 5. Grundlagen der Sprache

Variablen sind standardmäßig unveränderlich:

~~~zelyra
name = "Anna"
age: Int = 25
~~~

Veränderlichkeit wird ausdrücklich markiert:

~~~zelyra
mutable counter = 0
counter = counter + 1
~~~

Funktionen besitzen typisierte Parameter und können einen typisierten
Rückgabewert haben:

~~~zelyra
fn add(a: Int, b: Int) -> Int {
    return a + b
}
~~~

Normale Werte sind nicht null. Optionale Werte verwenden die Fragezeichen-
Schreibweise und müssen ausdrücklich behandelt werden:

~~~zelyra
name: String?

match name {
    Some(value) => print(value)
    None => print("Unknown")
}
~~~

Die Sprache unterscheidet außerdem nominale Fachtypen:

~~~zelyra
type UserId = Id
type OrderId = Id
~~~

UserId und OrderId sind unterschiedliche Typen, obwohl beide auf Id basieren.
Damit wird eine wichtige Klasse von Fehlern in der Geschäftslogik verhindert.

Schleifen unterstützen expliziten Kontrollfluss und optionale Invarianten bei
`while` und unbedingtem `loop`:

~~~zelyra
mutable current = 3

while current > 0
    invariant { current >= 0 }
{
    current = current - 1
}
~~~

`break` beendet die aktuelle Schleife und `continue` startet ihren nächsten
Durchlauf. Die Runtime prüft deklarierte Invarianten vor und nach den
Durchläufen. Der Verifier kann eine bewiesene Invariante verwenden, um
unterstützte lineare Schleifen zusammenzufassen; bei `loop` wird dafür ein
modellierter `break`-Austritt benötigt. `zelyra verify` meldet außerdem jede
deklarierte Invariante einzeln, mit nullbasierten Namen wie
`reduce.invariant[0]`. `FAILED` bedeutet, dass eine Invariante auf einem
analysierten Pfad falsch ist oder nicht erhalten bleibt; `RUNTIME_CHECK`
bedeutet, dass wegen eines unvollständigen symbolischen Beweises eine
Laufzeitprüfung nötig ist. Jedes Ergebnis enthält einen Verifikationscode und
einen Quellbereich als `(datei.zyl:startzeile:startspalte-endzeile:endspalte)`.
Für IDEs oder CI kann `--json` verwendet werden. Die Textausgabe enthält
zusätzlich eine Erklärung, ein optionales Gegenbeispiel und einen
Quellzeilenausschnitt mit Caret-Marker; JSON liefert dieselbe Erklärung im Feld
`message` sowie ein Gegenbeispiel als Objekt oder `null`. Nur `PROVEN` ist ein
Beweis:

~~~bash
zelyra verify examples/loop_control.zyl
zelyra verify examples/loop_control.zyl --json
~~~

## 6. Web-Seite ohne Apache starten

Zelyra 0.1 enthält einen ersten eingebauten HTTP-Server. Das Beispiel:

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

Diese Adresse im Browser öffnen:

~~~text
http://127.0.0.1:3000/hello/Zelyra
~~~

Bei Bedarf einen anderen lokalen Port verwenden:

~~~bash
zelyra serve examples/hello_web.zyl 127.0.0.1:8080
~~~

Der aktuelle Server unterstützt GET-Routen, feste Pfadsegmente,
Pfadparameter, das Entfernen von Query-Strings für das Routing,
grundlegendes HTTP-Parsing und HTML-Responses. Werte aus Pfadparametern
werden standardmäßig HTML-escaped.

Apache, nginx, Caddy, TLS-Terminierung, Prozessüberwachung und
Firewall-Konfiguration sind Aufgaben für die Bereitstellung. Für diesen
lokalen ersten Schritt werden sie nicht benötigt.

## 7. MariaDB-Schema definieren

MariaDB ist das Standard-Backend und die primäre Runtime-Referenz von Zelyra.
Ein Schema wird direkt beschrieben:

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

Das vorhandene Maschinenverwaltungsbeispiel enthält verbundene Tabellen:

~~~text
examples/machine_management_mariadb.zyl
~~~

Eine MariaDB-Verbindungszeichenfolge über die Umgebung setzen:

~~~bash
export DATABASE_URL='mariadb://user:password@127.0.0.1:3306/meine_app'
~~~

Aktuelle Datenbank inspizieren:

~~~bash
zelyra db inspect examples/machine_management_mariadb.zyl
~~~

Schemaunterschiede planen:

~~~bash
zelyra db plan examples/machine_management_mariadb.zyl
~~~

`DATABASE_URL` in der Shell setzen und anschließend mit dem einsteigerfreundlichen
Setup-Befehl eine neue MariaDB-Datenbank samt Anfangsschema erzeugen:

~~~bash
export DATABASE_URL='mariadb://user:<passwort>@127.0.0.1:3306/meine_app'
zelyra db setup examples/machine_management_mariadb.zyl
~~~

`db bootstrap` bleibt als kompatibler Alias verfügbar:

~~~bash
zelyra db bootstrap examples/machine_management_mariadb.zyl
~~~

Geprüften Plan anwenden:

~~~bash
zelyra db apply examples/machine_management_mariadb.zyl
~~~

Destruktive Änderungen werden ohne ausdrückliche Freigabe abgelehnt:

~~~bash
zelyra db apply examples/machine_management_mariadb.zyl --allow-destructive
~~~

Destruktive Pläne sorgfältig prüfen. Niemals echte Passwörter in eine
committete Zelyra-Datei, Dokumentation oder ein Shell-Script schreiben.
Umgebungsvariablen, Secret-Manager und geschützte Deployment-Konfiguration
sind vorzuziehen.

## 8. SQLite lokal verwenden

SQLite ist nützlich, wenn eine lokale Datei-Datenbank ohne laufenden
Datenbankdienst benötigt wird:

~~~bash
export DATABASE_URL='sqlite:///tmp/meine-app.sqlite3'
zelyra db bootstrap examples/machine_management_sqlite.zyl
zelyra db inspect examples/machine_management_sqlite.zyl
~~~

SQLite und MariaDB gehören beide zur unterstützten Datenbankrichtung. Der
aktuelle Runtime-Pfad für natives SQL wird hauptsächlich mit MariaDB erprobt;
Schema-DDL und Inspektion sind in dieser Phase breiter unterstützt als die
Runtime-Abfrageausführung.

## 9. Natives SQL schreiben

SQL wird in einem nativen Block geschrieben:

~~~zelyra
customer = sql<Customer?> {
    SELECT id, name, email
    FROM customers
    WHERE id = :id
}
~~~

Benannte Parameter werden als Parameter gebunden und nicht in SQL
zusammenkonkateniert. Wenn das Schema verfügbar ist, kann Zelyra Tabellen,
Spalten, Aliase, Parameter, NULL-Fähigkeit und Ergebnismappings prüfen.

Auch INSERT-, UPDATE- und Transaktionsblöcke gehören zur aktuellen Syntax:

~~~zelyra
transaction {
    sql {
        UPDATE inventory
        SET quantity = quantity - :amount
        WHERE product_id = :product
    }
}
~~~

Zelyra respektiert komplexes SQL. Entwickler müssen nicht jede Abfrage durch
eine ORM-Abstraktion ersetzen.

## 10. Schemaabhängiges Formular validieren

Ein Formular kann auf eine Tabelle zeigen und ihre Felder übernehmen:

~~~zelyra
form CustomerCreate -> customers {
    fields {
        name
        email
    }
}
~~~

Ein ausdrücklich definiertes Feld kann Darstellung und Validierungsregeln
ergänzen:

~~~zelyra
form CustomerForm {
    field email: Email {
        label: "E-Mail"
        required
        max: 255
        widget: email
    }
}
~~~

Lokale Validierung ausführen:

~~~bash
zelyra form validate examples/customer_form.zyl CustomerCreate \
  name=Anna email=anna@example.test
~~~

Erfolgreiche Ausgabe:

~~~text
valid: CustomerCreate
~~~

Ungültige Eingaben testen:

~~~bash
zelyra form validate examples/customer_form.zyl CustomerCreate \
  email=keine-email
~~~

Der Validator meldet das fehlende Pflichtfeld name und die ungültige
E-Mail-Adresse. Zusätzlich werden unbekannte Felder, zu lange Werte, ungültige
Zahlen, ungültige boolesche Werte und übermittelte Readonly-Felder abgelehnt.

Der eingebaute Server stellt jedes Formular zusätzlich unter /forms/FormName
bereit. GET rendert ein HTML-Formular mit escaped Werten und einem CSRF-Token
pro Serverstart. POST prüft das Token, parst URL-encoded Daten und validiert die
Felder. Ungültige Eingaben liefern Feldfehler mit HTTP 422. Ein Formular ohne
Aktion liefert HTTP 202 als Validierungsbestätigung.

Für eine datenbankgestützte Aktion verwende das MariaDB-Beispiel:

~~~bash
export DATABASE_URL='mariadb://root:<passwort>@127.0.0.1:3306/zelyra_forms'
zelyra db bootstrap examples/customer_form_action.zyl
zelyra serve examples/customer_form_action.zyl
~~~

Nach einer gültigen POST-Anfrage bindet Zelyra die deklarierten Felder als
Prepared-Statement-Parameter, führt die Aktion in einer MariaDB-Transaktion
aus und liefert HTTP 303 zur Weiterleitung. Ohne `DATABASE_URL` liefert die
Aktion HTTP 503; Datenbankfehler werden als allgemeiner HTTP-500-Fehler
ausgegeben. Das Aktions-SQL wird vor dem Serverstart statisch gegen das
Quellschema geprüft.

Beziehungsfelder werden automatisch aus MariaDB geladen. In
`examples/machine_form.zyl` wird `department: Department required` zu einem
`<select>` mit Abteilungs-ID und -Name. Zelyra lehnt eine übermittelte ID ab,
die in der aktuellen Datenbank nicht vorhanden ist, bevor die Formularaktion
ausgeführt wird.

Eine minimale CRUD-Ressource wird so ergänzt:

~~~zelyra
crud Machine -> machines
~~~

Für eine angepasste Ressource können Titel, sichtbare Spalten, Suchspalten und
Filter konfiguriert werden:

~~~zelyra
crud Machine -> machines {
    title: "Machines"
    list { number name department active }
    search { number name }
    filter { department active }
}
~~~

Alle Blöcke sind optional. Konfigurierte Spalten werden gegen das Schema
geprüft; Beziehungsfelder werden auf ihre gespeicherten Foreign-Key-Spalten
abgebildet. Die Kurzform behält die Defaults für Liste, Textsuche und Filter
außer der ID.

Damit wird `GET /machines` mit escaped Ausgabe, Suche, exakten Filtern,
Allowlist-Sortierung, Pagination, verlinkten Detailseiten und automatisch
erzeugten Create-/Edit-Formularen unter `/machines/new` und
`/machines/<id>/edit` bereitgestellt. Filter verwenden `filter_<spalte>`;
Sortierung verwendet `sort=<spalte>&order=asc|desc`.

Für ein kompaktes gemeinsames View-Profil genügt `view { fields { ... } }`
innerhalb eines CRUDs. Es steuert erzeugte Liste, Detailansicht und Formulare;
ein explizites `list { ... }` überschreibt nur die Auswahl für Liste/Detail.

## 11. Nützliche Befehle

~~~text
zelyra new <directory> [--mariadb] [--template minimal|mariadb-crud|mariadb-auth|mariadb-business] [--web-port <port>] [--host-port <port>] [--db-host-port <port>]
                                         Projekt erstellen und Ports wählen
zelyra init [directory] [--mariadb] [--template minimal|mariadb-crud|mariadb-auth|mariadb-business] [--web-port <port>] [--host-port <port>] [--db-host-port <port>]
                                         Projekt initialisieren und Ports wählen
zelyra setup [directory] [--database|--schema|--all] [--host-port <port>] [--db-host-port <port>]
                                         .env, MariaDB oder Schema vorbereiten
zelyra setup --web [directory] [--port <port>]
                                         lokalen Browser-Setup-Assistenten öffnen
zelyra check <file.zyl> [--format human|json]
                                         Quellcode prüfen; JSON ist versioniert und maschinenlesbar
zelyra context <file.zyl> [--format human|json]
                                         schreibgeschützten Projektkontext inspizieren
zelyra build <file.zyl>                 Quellcode prüfen/bauen
zelyra run <file.zyl>                   Programm ausführen
zelyra serve <file.zyl> [address]       eingebauten HTTP-Server starten
zelyra doctor [file.zyl] [--env-file <path>] [--port <port>] [--json]
                                         Projekt, DB, Docker und Web-Bereitschaft prüfen
zelyra verify <file.zyl>                Contract-Prüfungen klassifizieren
zelyra doc <file.zyl> [--openapi]       OpenAPI-Dokument erzeugen
zelyra doc <file.zyl> --typescript     TypeScript-Client erzeugen
zelyra form validate <file> <Form> ...  Formularwerte validieren
zelyra db create <file.zyl>             Schema-DDL ausgeben
zelyra db setup <file.zyl>              MariaDB und Anfangsschema einrichten
zelyra db bootstrap <file.zyl>          Anfangsschema erzeugen/anwenden
zelyra db inspect <file.zyl>            aktuelle Datenbank inspizieren
zelyra db plan <file.zyl>               Schemaänderungen anzeigen
zelyra db apply <file.zyl>              geprüfte Änderungen anwenden
~~~

Für Rust-Mitwirkende:

~~~bash
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
~~~

Für einen echten MariaDB-CRUD-Test wird die CLI gebaut und anschließend das
zugangsdatenfreie Integrationsskript mit einer eigenen Testdatenbank gestartet:

~~~bash
cargo build -p zelyra-cli
export DATABASE_URL='mariadb://user:password@127.0.0.1:3306/zelyra_e2e'
./tests/mariadb-e2e.sh
~~~

Das Skript prüft Schema-Setup und -Inspektion, startet den Zelyra-Server und
testet verbundene CRUD-Abläufe zum Erstellen, Lesen, Bearbeiten und Löschen.
Eigene Testdatensätze werden wieder entfernt; das Datenbankpasswort wird weder
ausgegeben noch gespeichert.

Für den vollständigen Test des erzeugten Projekts gegen eine frische Datenbank
die isolierte MariaDB-Instanz und deren Root-Passwort verwenden:

~~~bash
export ZELYRA_GENERATED_E2E_ROOT_PASSWORD='<test-passwort>'
./tests/generated-project-mariadb-e2e.sh
~~~

Der Test erzeugt ein temporäres Projekt, führt `zelyra setup` aus, prüft die
erzeugte Compose- und `doctor`-Konfiguration und entfernt temporäre Datenbank
und Projekt nach dem CRUD-HTTP-Test wieder.

Zusätzlich kann die erzeugte Docker-Laufzeit geprüft werden:

~~~bash
./tests/generated-project-docker-e2e.sh
~~~

Der Test baut das erzeugte Image, startet MariaDB und Webserver standardmäßig
auf den Host-Ports 3309 und 18082, prüft Willkommensseite und Port-Zuordnung
und entfernt alle temporären Docker-Ressourcen anschließend wieder.

## 12. Häufige Probleme

### zelyra: command not found

Das benutzerlokale bin-Verzeichnis ist nicht im PATH:

~~~bash
export PATH="$HOME/.local/bin:$PATH"
~~~

### Rust oder curl fehlt

Der Quellcode-Installer benötigt curl nur, wenn Rust fehlt. curl mit dem
Paketmanager des Betriebssystems installieren und install.sh erneut ausführen.
Alternativ Rust zuerst über den offiziellen rustup-Weg installieren.

### DATABASE_URL wird benötigt

Datenbank-Inspektion, Bootstrap, Planung gegen eine laufende Datenbank und
Apply benötigen eine Verbindungszeichenfolge:

~~~bash
export DATABASE_URL='mariadb://user:password@127.0.0.1:3306/meine_app'
~~~

Formularvalidierung und Sprachbeispiele benötigen keine Datenbankverbindung.

### Destruktive Schemaänderung wird abgelehnt

Das ist beabsichtigt. db plan ausführen, betroffene Zeilen und SQL prüfen und
db apply erst danach mit dem ausdrücklichen Flag allow-destructive wiederholen.

### Eine Seite startet, aber der Browser zeigt 404

Die exakte Route einschließlich aller Pfadparameter prüfen. Die Beispielroute
besteht aus hello und genau einem Namenssegment. Query-Strings werden beim
Matching ignoriert, der Pfad selbst muss aber übereinstimmen.

## 13. Wie geht es weiter?

Weitere technische Details stehen in den Phasendokumenten:

- [Phase 2: Type System](phase-2.de.md);
- [Phase 3: Database Core](phase-3.de.md);
- [Phase 4: Native SQL](phase-4.de.md);
- [Phase 5: Web Core](phase-5.de.md);
- [Phase 6: Forms](phase-6.de.md);
- [Phase 7: CRUD](phase-7.de.md).
- [Phase 8: Authentifizierung und Autorisierung](phase-8.de.md).
- [Phase 9: Capabilities](phase-9.de.md).
- [Phase 10: Typisierte APIs und OpenAPI](phase-10.de.md).
- [Phase 11: Browser-API-Integration und CORS](phase-11.de.md).
- [Phase 12: Request-Validierung und sichere Antwort-Defaults](phase-12.de.md).

Die englischen Fassungen verwenden dieselben Namen ohne das Suffix .de.md.

Die CRUD-Liste bietet jetzt außerdem Details, Create-/Edit-Formulare,
konfigurierbare Spalten und eine CSRF-geschützte Löschaktion. Die
Authentifizierung unterstützt persistente Sessions und
Berechtigungsabfragen über MariaDB-Tabellen.

Für einen Hash in der erforderlichen Spalte `password_hash` verwenden:

~~~bash
zelyra auth hash-password
~~~

Das Passwort wird nicht angezeigt und muss zweimal eingegeben werden. Für
bewusste Automatisierung kann eine Passwortzeile mit `--stdin` übergeben
werden; echte Passwörter niemals als Kommandoargument verwenden oder den
erzeugten Wert in die Versionsverwaltung übernehmen:

~~~bash
printf '%s\n' 'dieses-passwort-aendern' | zelyra auth hash-password --stdin
~~~
