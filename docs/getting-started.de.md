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
- erste CRUD-Listen, Details, Create-/Edit-Formulare, konfigurierbare Spalten
  und Löschaktionen;
- später vollständiges CRUD, APIs, Authentifizierung, Autorisierung und
  Verifikation.

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
- erste CRUD-Listen mit MariaDB-Suche, Filtern, Sortierung, Pagination und
  CSRF-geschützten Löschaktionen.
- ein datenbankgestützter Login mit Argon2-Passwortprüfung, persistenten
  HttpOnly-Sessions, Logout, datenbankgestützten Berechtigungen und
  geschützten Routen.
- erste Capability-Deklarationen, Weitergabe über Funktionsaufrufe und
  statische Durchsetzung von `Database` für natives SQL sowie Projektfreigaben
  in `zelyra.toml`.
- zur Laufzeit geprüfte Funktions-Contracts mit `requires` und `ensures`.

Noch nicht vollständig sind: vollständige CRUD-Erzeugung, Datenbankrollen,
Login-Drosselung, Passwortverwaltungs-Kommandos, APIs, formale Verifikation,
Runtime-Capability-Durchsetzung, strukturierte
Nebenläufigkeit und Produktionspaketierung.

## 1. Voraussetzungen

Für die einfachste Installation aus dem Quellcode werden benötigt:

- eine Unix-ähnliche Shell mit Bash;
- Git;
- curl, falls Rust noch nicht installiert ist;
- eine Netzwerkverbindung für die erste Installation der Rust-Toolchain.

Der Installer ist für Linux- und macOS-ähnliche Umgebungen ausgelegt. Er
installiert nur für den aktuellen Benutzer und verwendet kein sudo. Ein
plattformnativer Installer für eigenständige Releases ohne Rust ist geplant.

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

Das Script baut die CLI im Release-Modus und installiert sie unter:

~~~text
~/.local/bin/zelyra
~~~

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

Die aktuelle Projektdatei ist bewusst klein:

~~~toml
[project]
name = "meine-app"
version = "0.1.0"
zelyra = "0.1"

[capabilities]
database = true
network = false
~~~

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

Neue MariaDB-Datenbank und Anfangsschema erzeugen:

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

## 11. Nützliche Befehle

~~~text
zelyra new <directory>                  Projekt erstellen
zelyra init [directory]                 Projekt initialisieren
zelyra check <file.zyl>                 Quellcode prüfen
zelyra build <file.zyl>                 Quellcode prüfen/bauen
zelyra run <file.zyl>                   Programm ausführen
zelyra serve <file.zyl> [address]       eingebauten HTTP-Server starten
zelyra verify <file.zyl>                Contract-Prüfungen klassifizieren
zelyra form validate <file> <Form> ...  Formularwerte validieren
zelyra db create <file.zyl>             Schema-DDL ausgeben
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

Die englischen Fassungen verwenden dieselben Namen ohne das Suffix .de.md.

Die CRUD-Liste bietet jetzt außerdem Details, Create-/Edit-Formulare,
konfigurierbare Spalten und eine CSRF-geschützte Löschaktion. Die
Authentifizierung unterstützt jetzt persistente Sessions und
Berechtigungsabfragen über MariaDB-Tabellen; Datenbankrollen und
Passwortverwaltungs-Workflows bleiben zukünftige Aufgaben.
