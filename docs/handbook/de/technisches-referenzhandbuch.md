# TECHNISCHES REFERENZHANDBUCH

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

- Linux, macOS oder Windows (PowerShell/WSL);
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
./install.sh --release v0.3.0
~~~

Unter Windows steht `install.ps1` für PowerShell und `install.cmd` für die
Eingabeaufforderung bereit:

~~~powershell
git clone https://github.com/sf1976/zelyra.git
Set-Location zelyra
.\install.ps1
zelyra --version
~~~

Das Release-Archiv unter Windows:

~~~powershell
.\install.ps1 -Release v0.3.0
~~~

Danach:

~~~bash
zelyra --version
zelyra --help
~~~

Wenn die Shell `zelyra` nicht findet:

~~~bash
export PATH="$HOME/.local/bin:$PATH"
~~~

### Docker und Container-Umgebung

Zelyra installiert Docker selbst nicht, verändert keine Betriebssystempakete und fordert keine Root-Rechte an. Für Container- und MariaDB-Workflows wird Docker mit Compose-Unterstützung benötigt:

- **Linux:** Verwende die [offizielle Linux-Anleitung](https://docs.docker.com/engine/install/).
- **Windows:** Verwende [Docker Desktop für Windows](https://docs.docker.com/desktop/setup/install/windows-install/) mit aktivierter Compose-Unterstützung.
- **macOS:** Verwende [Docker Desktop für Mac](https://docs.docker.com/desktop/setup/install/mac-install/).

Überprüfe die Docker-Umgebung vor dem ersten Start:

~~~bash
docker compose version
~~~

Ist Docker installiert, aber der Zugriff auf seinen Socket verweigert, meldet Zelyra einen sicheren Hinweis zur Linux-Gruppenmitgliedschaft (`sudo usermod -aG docker $USER`) statt der rohen Docker-Ausgabe. Auch Portkonflikte werden ohne Preisgabe von Zugangsdaten gemeldet.

### Zelyra aktualisieren

Ein Update besteht aus zwei getrennten Teilen: Du aktualisierst den Zelyra-
Compiler im Compiler-Repository und prüfst danach dein eigenes
Anwendungsprojekt. Deine `.zyl`-Dateien, `zelyra.toml` und `.env` liegen in
deinem Anwendungsprojekt und werden durch `install.sh` nicht überschrieben.

#### Installation aus dem Git-Repository

Arbeite zuerst im Compiler-Repository. Prüfe lokale Änderungen, bevor du sie
aktualisierst:

~~~bash
cd /pfad/zu/zelyra
git status --short
git pull --ff-only origin main
cargo check --workspace
./install.sh
zelyra doctor /pfad/zu/deinem-projekt/main.zyl --json
~~~

`git pull --ff-only` bricht ab, wenn du lokale Änderungen oder eine eigene
Historie hast. Sichere oder committe diese Änderungen zuerst und führe das
Update danach erneut aus. `./install.sh` baut die aktuelle CLI-Version und
installiert sie mit `cargo install --path cli --force` erneut in deinem
Benutzerverzeichnis.

Die installierte Versionsnummer kannst du über `zelyra --version` oder den JSON-Bericht von `doctor` ablesen:

~~~bash
zelyra --version
zelyra doctor /pfad/zu/deinem-projekt/main.zyl --json
~~~

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

## 4. Neues Projekt anlegen und CLI

### Compiler-Repository und Anwendungsprojekt

Das GitHub-Repository `sf1976/zelyra` ist das Compiler-Repository. Es enthält
Lexer, Parser, Runtime, Datenbank- und Webmodule sowie das CLI. Eine eigene
Anwendung ist ein davon getrenntes Verzeichnis. Du musst also nicht in der
Zelyra-Quelle arbeiten und solltest dort auch keine Zugangsdaten ablegen.

Der Projektstamm ist das Verzeichnis, in dem die Zelyra-Quelldatei und – wenn
vorhanden – `zelyra.toml` liegen. Der Speicherort ist frei wählbar, zum Beispiel
`~/projekte/adressverwaltung` oder `C:\\Users\\Du\\Projekte\\adressverwaltung`.

### ✅ Projekt mit dem vorhandenen CLI anlegen

Voraussetzungen für die CLI-Installation sind Rust/Cargo oder ein Release-Archiv; für relationale Datenbankbefehle der externe `mariadb`-Client oder Docker. Das CLI bietet aktuell diese Projektbefehle:

~~~bash
zelyra new adressverwaltung
cd adressverwaltung
zelyra run main.zyl
~~~

Ein vorhandenes Verzeichnis initialisieren:

~~~bash
mkdir adressverwaltung
cd adressverwaltung
zelyra init
~~~

Für eine lokale MariaDB- und Webserver-Vorlage `zelyra new maschinenverwaltung --mariadb` verwenden:

~~~bash
zelyra new maschinenverwaltung --mariadb
cd maschinenverwaltung
~~~

Dadurch entstehen `main.zyl`, `.env.example`, `Dockerfile` und `docker-compose.mariadb.yml` sowie direkt eine geschützte `.env` mit sicheren Zufallspasswörtern.

**Automatische Portvergabe bei Konflikten:**
Sind die Standardports `3000` (Web) oder `3306` (MariaDB) auf dem Rechner belegt, ermitteln `zelyra new`, `zelyra init` und `zelyra setup` automatisch den nächsten freien Host-Port und tragen ihn in die neue `.env` ein. Mit den optionalen Flags `--web-port <p>`, `--host-port <p>` und `--db-host-port <p>` können Ports verbindlich vorgegeben werden.

### ✅ Zelyra-Setup-Assistent (Konsole und Browser)

`zelyra setup` bietet konsistente Einrichtungsaktionen sowohl über die Befehlszeile als auch über eine benutzerfreundliche Weboberfläche:

#### Setup auf der Konsole

Aus einem MariaDB-Projektverzeichnis:

~~~bash
zelyra setup
zelyra setup --database
zelyra setup --schema
zelyra setup --all
zelyra setup --host-port 18080 --db-host-port 3308
~~~

- `zelyra setup`: Legt eine geschützte `.env` an, wenn sie fehlt. Vorhandene `.env`-Dateien werden niemals überschrieben.
- `zelyra setup --database`: Startet die Docker-Compose-Dienste (MariaDB und Zelyra-App). Erkennt automatisch `docker compose` oder den Legacy-Befehl `docker-compose`.
- `zelyra setup --schema`: Startet die Umgebung und wendet das Schema aus `main.zyl` sicher an.
- `zelyra setup --all`: Führt alle Schritte vollautomatisch in einem Zug aus.

#### Setup im lokalen Browser (`zelyra setup --web`)

~~~bash
zelyra setup --web
~~~

Der Assistent bindet standardmäßig ausschließlich an `127.0.0.1:3030` und erzeugt eine sichere, einmalige URL mit einem Zufallstoken:

~~~text
Zelyra setup web is running on http://127.0.0.1:3030/
open: http://127.0.0.1:3030/?token=<local-token>
~~~

- Der Browser bietet dieselben Aktionen wie die Konsole: Konfiguration vorbereiten, MariaDB und Anwendung starten, Schema anwenden oder alles auf Knopfdruck ausführen.
- Der Server ist aus Sicherheitsgründen **nur lokal** erreichbar und darf nicht öffentlich exponiert werden.
- Ist Port `3030` belegt, wählt der Assistent automatisch den nächsten freien Port (oder wird mit `--port <port>` fest vorgegeben).
- Nach Abschluss beendet `Ctrl+C` den Assistenten.

Nach dem Start prüft `zelyra doctor` den Status ohne destruktive Datenbankänderungen:

~~~bash
zelyra doctor main.zyl --env-file .env
~~~

Für ein vollständiges CRUD-Starterprojekt mit Geschäftslogik:

~~~bash
zelyra new maschinenverwaltung --template mariadb-crud
cd maschinenverwaltung
zelyra setup --all
~~~

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

Fehler werden als Teil der Funktionssignatur sichtbar. Der aktuelle Sprachkern
verwendet dafür `Result<T, E>` mit `Ok` oder `Err`:

~~~zelyra
fn load_number(found: Bool) -> Result<Int, String> {
    if found {
        return Ok(42)
    }
    return Err("nicht gefunden")
}
~~~

## 7. MariaDB und Tabellen

✅ MariaDB ist das Standardbackend und die primäre Runtime-Referenz. Die
generierte Compose-Vorlage verwendet `mariadb:11`; der Compiler erzwingt aber
keine konkrete MariaDB-Serverversion. Die Datenbankbefehle rufen den externen
`mariadb`-Client auf.

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

### MariaDB sicher vorbereiten

Lege für die Anwendung eine eigene Datenbank und einen eigenen Benutzer an.
Verwende nicht den MariaDB-Account `root` für den laufenden Zelyra-Webserver.
Die folgenden Befehle werden als administrativer MariaDB-Benutzer ausgeführt;
das Passwort wird interaktiv abgefragt:

~~~bash
mariadb --host=127.0.0.1 --port=3307 --user=root --password
~~~

~~~sql
CREATE DATABASE `adressverwaltung`
    CHARACTER SET utf8mb4
    COLLATE utf8mb4_unicode_ci;

CREATE USER 'zelyra'@'127.0.0.1'
    IDENTIFIED BY 'HIER_LOKALES_PASSWORT_EINTRAGEN';

GRANT SELECT, INSERT, UPDATE, DELETE, CREATE, ALTER, INDEX, REFERENCES
    ON `adressverwaltung`.* TO 'zelyra'@'127.0.0.1';

SHOW GRANTS FOR 'zelyra'@'127.0.0.1';
~~~

`CREATE DATABASE` kann bei einer bereits vorhandenen Datenbank mit
`IF NOT EXISTS` wiederholbar gemacht werden. Die Rechte sind bewusst auf diese
Datenbank begrenzt; `GRANT ALL ON *.*` gehört nicht in einen Anwendungs-
Schnellstart. `InnoDB` ist die für Transaktionen und Fremdschlüssel erwartete
Storage-Engine. Die aktuelle Zelyra-SQL-Ausgabe setzt `ENGINE=InnoDB` jedoch
nicht selbst; kontrolliere und ergänze das SQL vor dem Anwenden, wenn deine
Servervorgaben es verlangen.

Für reine Lese- und normale CRUD-Operationen reichen die aufgeführten Rechte.
`db apply --allow-destructive` kann zusätzlich `DROP`-Rechte benötigen; erteile
sie nur bewusst und möglichst zeitlich begrenzt. `db setup` ist ein
Administratorvorgang, weil dabei die Datenbank selbst angelegt wird.

Lokal kann MariaDB bereits auf Port `3306` laufen. Die generierte Compose-Datei
veröffentlicht standardmäßig `127.0.0.1:3306`. Wenn dieser Port belegt ist,
ändere die Zuordnung auf `127.0.0.1:3307:3306`: außen ist dann `3307`, im
Container bleibt MariaDB auf `3306`. Von einem anderen Compose-Service ist der
Host der Servicename `mariadb` und der Port weiterhin `3306`; vom Host ist es
`127.0.0.1` plus der veröffentlichte Port.

### Eine übersichtliche Datenbankstruktur planen

Ordne Tabellen nach fachlichen Bereichen und halte ihre Zelyra-Deklarationen
bei den Modulen, die diese Daten fachlich besitzen. Beispielsweise können
Kunden, Aufträge und Lager jeweils eigene Module bilden. Ein Modul darf die
Tabellen anderer Bereiche verwenden, sollte diese aber nicht nebenbei neu
deklarieren. So bleiben Tabellen, SQL und Zuständigkeit im Quellcode auffindbar.

Verwende eine einheitliche Sprache und Benennung, aussagekräftige Tabellen- und
Spaltennamen, explizite Primär- und Fremdschlüssel sowie nur tatsächlich
benötigte Indizes. Vermeide technische Präfixe wie `tbl_`, Sammel-Tabellen für
verschiedene Fachbereiche und eine Aufteilung allein nach Webseiten oder
Formularen. Ergänze Zeitstempel oder Statusfelder nur mit klarer fachlicher
Bedeutung. Kleine Lookup-Tabellen und Zuordnungstabellen sind sinnvoll, wenn
die Datenbeziehung sie erfordert.

Nutze zunächst eine Datenbankverbindung für die Anwendung und Module als
Ordnung im Zelyra-Quellcode. Datenbankschemas oder getrennte Datenbanken pro
Bereich sind eine bewusste Betriebsentscheidung, keine automatische Folge der
Modulaufteilung; ihre Bedeutung unterscheidet sich außerdem zwischen
Datenbank-Backends. Vorhandene Tabellen müssen nicht umbenannt werden, um sie
in Zelyra zu verwenden.

`zelyra db map src/main.zyl` bietet eine schreibgeschützte Übersicht der nach
Quellmodul gruppierten Tabellendeklarationen, erkannten Fremdschlüssel,
deklarierten, aber in der Live-Datenbank fehlenden Tabellen und Live-Tabellen
ohne Quelldeklaration. Bei zugeordneten Tabellen stellt er deklarierte und live
gefundene Spalten und Fremdschlüssel gegenüber. `--format=json` liefert
deterministische strukturierte Ausgabe. Der Befehl zeigt niemals die
Verbindungs-URL und ändert das Schema nicht. Eine Modulzuordnung ist ein
organisatorischer Hinweis, keine Datenbankberechtigung und keine erzwungene
Eigentumsregel. `db inspect` liest; `db plan` zeigt Unterschiede; Änderungen
erfolgen weiterhin nur über den ausdrücklich geprüften `db apply`-Ablauf.

~~~bash
zelyra db map src/main.zyl
zelyra db map src/main.zyl --format=json
~~~

### Das Adressschema kontrollieren

Für das Beispiel erzeugt `zelyra db create src/main.zyl` derzeit sinngemäß:

~~~sql
CREATE TABLE IF NOT EXISTS `addresses` (
    `id` BIGINT PRIMARY KEY NOT NULL AUTO_INCREMENT,
    `first_name` VARCHAR(100) NOT NULL,
    `last_name` VARCHAR(100) NOT NULL,
    `street` VARCHAR(150) NOT NULL,
    `postal_code` VARCHAR(10) NOT NULL,
    `city` VARCHAR(100) NOT NULL,
    `email` VARCHAR(255)
) ENGINE=InnoDB DEFAULT CHARACTER SET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
~~~

Für MariaDB enthalten die erzeugten Tabellenaussagen
`ENGINE=InnoDB DEFAULT CHARACTER SET=utf8mb4 COLLATE=utf8mb4_unicode_ci`.
Damit sind sie für die dokumentierte MariaDB-Basis vollständig; sie bleiben
dennoch ein zu prüfender Schema-Vorschlag. Vor Änderungen an einer bestehenden
Datenbank weiterhin `db plan` lesen und das Risiko ausdrücklich bewerten.

### Weitere Datenbank-Backends

Neben MariaDB kann das Schema-CLI derzeit auch SQLite und PostgreSQL
verarbeiten. Das Backend wird in der `.zyl`-Datei angegeben:

~~~zelyra
database main { engine: sqlite database: "adressverwaltung.sqlite3" }
database main { engine: postgres database: "adressverwaltung" }
~~~

Für SQLite verwendet `DATABASE_URL` das Schema `sqlite://` oder `sqlite:`; für
MariaDB sind `mariadb://` und der kompatible Name `mysql://` gültig. Die
Schemaoperationen `create`, `inspect`, `plan` und `apply` berücksichtigen
Tabellen, Spalten, Foreign Keys und Indizes. `db setup` und `db bootstrap`
unterstützen für das automatische Anlegen derzeit MariaDB und SQLite; für
PostgreSQL verwendest du das gewünschte Schema mit `db create`, `db inspect`,
`db plan` und anschließend `db apply`.

~~~bash
export DATABASE_URL='sqlite:///tmp/adressverwaltung.sqlite3'
zelyra db bootstrap src/main.zyl
zelyra db inspect src/main.zyl
~~~

Die native SQL-Runtime wird weiterhin hauptsächlich mit MariaDB eingesetzt.
Ein vorhandenes Schema-Backend ist daher nicht automatisch ein Beleg dafür,
dass jede Runtime-Abfrage mit jedem Backend gleich funktioniert.

## 8. Schema prüfen und anwenden

Zelyra verwendet keine Migrationsklassen. Die Quelldatei ist das gewünschte
Schema; das CLI vergleicht dieses mit dem Ist-Zustand. Der aktuelle Ablauf ist:

1. Quelldatei prüfen: `zelyra check src/main.zyl`.
2. SQL nur anzeigen: `zelyra db create src/main.zyl`.
3. Ist-Schema lesen: `zelyra db inspect src/main.zyl`.
4. Unterschied planen: `zelyra db plan src/main.zyl`.
5. Plan lesen und Risiko bewerten.
6. Nach der Kontrolle anwenden: `zelyra db apply src/main.zyl`.

Die Verbindung kommt ausschließlich aus der Prozessumgebung:

~~~bash
export DATABASE_URL='mariadb://zelyra:HIER_LOKALES_PASSWORT_EINTRAGEN@127.0.0.1:3307/adressverwaltung'
~~~

🧪 **Nur im unveröffentlichten 0.4-Entwicklungszweig:** Der MariaDB-Client
erhält ein Verbindungszeitlimit und native Runtime-Abfragen ein
MariaDB-Statement-Zeitlimit. Beide Werte können über die Prozessumgebung
angepasst werden:

~~~bash
export ZELYRA_DB_CONNECT_TIMEOUT_SECS=10
export ZELYRA_DB_QUERY_TIMEOUT_SECS=30
~~~

Der Standard für den Verbindungsaufbau beträgt 10 Sekunden; erlaubt sind
Ganzzahlen von 1 bis 300. Das Statement-Limit beträgt standardmäßig 30 Sekunden
und akzeptiert 1 bis 3600. Ungültige Werte werden abgelehnt, ohne den Eingabewert
in die Diagnose aufzunehmen. Das Statement-Limit wird von MariaDB selbst
durchgesetzt; es ist kein allgemeines Zeitlimit für das Übertragen sehr großer
Ergebnisdaten. Schemaänderungen durch `db apply` erhalten dieses
Statement-Limit derzeit nicht, damit ein abgebrochener DDL-Befehl nicht als
sicher zurückgerollt dargestellt wird.

Der CLI-Aufruf setzt außerdem `--skip-reconnect`: Der MariaDB-Client darf eine
abgebrochene Verbindung nicht unbemerkt neu herstellen und ein Statement
automatisch wiederholen. Zelyra implementiert hier keine automatischen Retries
und keinen Connection-Pool; jede DB-Operation startet weiterhin einen
MariaDB-Client-Prozess. Der veröffentlichte Stand `0.3.0` unterstützt diese
Timeout-Optionen noch nicht.

⚠️ Zelyra lädt `.env` derzeit **nicht automatisch**. Eine `.env` ist eine
sichere lokale Ablage, aber die Variablen müssen vor dem CLI-Aufruf in die
Prozessumgebung gelangen. Siehe Abschnitt 16.

Schema-SQL anzeigen:

~~~bash
zelyra db create src/main.zyl > sql/addresses.generated.sql
~~~

Wenn du diese Datei nach der Kontrolle manuell um `ENGINE=InnoDB` und die
Zeichensatzklauseln ergänzt hast, wendest du genau diese Datei mit dem
MariaDB-Client an:

~~~bash
mariadb --host=127.0.0.1 --port=3307 --user=zelyra --password \
    adressverwaltung < sql/addresses.generated.sql
~~~

`zelyra db apply` liest keine von dir bearbeitete SQL-Datei ein; es erzeugt
seinen Plan erneut aus der `.zyl`-Quelle. Verwende deshalb entweder den
unveränderten Zelyra-Plan oder den manuellen Clientweg – nicht beides blind
hintereinander.

Bedingt durch die aktuelle Implementierung kann `db plan` auch ohne
`DATABASE_URL` gegen ein leeres Schema planen. Für eine echte Bestandsaufnahme
und für `db apply` ist die Variable Pflicht:

~~~bash
zelyra db inspect src/main.zyl
zelyra db plan src/main.zyl
zelyra db apply src/main.zyl
~~~

Destruktive Änderungen werden abgelehnt, bis sie ausdrücklich freigegeben
werden:

~~~bash
zelyra db apply src/main.zyl --allow-destructive
~~~

Dieses Flag bedeutet nicht „wird schon gutgehen“. Es bedeutet „ich habe den
Plan gelesen, ein Backup und einen vernünftigen Puls“.

MariaDB speichert jeden `db apply`-Plan und jeden abgeschlossenen DDL-Schritt
in `_zelyra_schema_history`. Prüfe den Verlauf nach einem Deployment oder
Abbruch:

~~~bash
zelyra db history src/main.zyl
zelyra db history src/main.zyl --format=json
~~~

Wenn ein Prozess zwischen DDL-Schritten stoppt, prüfe das Live-Schema, erstelle
einen neuen Plan, kontrolliere ihn und wende ihn an. Der Verlauf zeigt den
letzten Prüfpunkt; bei einem Abbruch während eines SQL-Befehls kann er nicht
feststellen, ob das Schema teilweise geändert wurde. Die reservierte
Verlaufstabelle erscheint nicht in der normalen Schemaansicht.

`db setup` und sein Alias `db bootstrap` versuchen bei MariaDB zuerst die
Datenbank anzulegen und wenden anschließend das generierte Schema an. Dafür
braucht `DATABASE_URL` administrative Rechte. Ein gewöhnlicher
Anwendungsbenutzer mit begrenzten Rechten sollte stattdessen `db create`
verwenden und die SQL-Schritte durch einen Administrator ausführen lassen.

❌ Nicht vorhanden sind `zelyra db check` sowie `zelyra schema inspect`,
`zelyra schema plan` und `zelyra schema apply`. Der tatsächlich vorhandene
Bereitschaftstest ist `zelyra doctor src/main.zyl`; er prüft statische Regeln,
Cargo, `DATABASE_URL` (wenn gesetzt) und den lokalen Webport.

### In 10 Minuten zur ersten Zelyra-Anwendung

Die folgenden Schritte verwenden überall dieselben Werte: das Verzeichnis
`adressverwaltung`, die Datei `src/main.zyl`, die Datenbank
`adressverwaltung`, den Benutzer `zelyra` und den Host-Port `3307`.

1. Projektverzeichnis und Vorlage anlegen:

   ~~~bash
   zelyra new adressverwaltung --mariadb
   cd adressverwaltung
   cp .env.example .env
   ~~~

2. In `.env` die Platzhalter setzen. Wichtig: Compose liest die
   `MARIADB_*`-Werte beim Containerstart; Zelyra selbst liest nur
   `DATABASE_URL`. Ändere für den Host-Port in der Compose-Datei die Bindung
   auf `127.0.0.1:3307:3306` und verwende lokal `3307`.

3. MariaDB starten:

   ~~~bash
   docker compose -f docker-compose.mariadb.yml up -d mariadb
   docker compose -f docker-compose.mariadb.yml ps
   ~~~

4. Als MariaDB-Administrator Datenbank und Benutzer vorbereiten; verwende das
   SQL aus Abschnitt 7 und nur ein lokales Platzhalterpasswort.

5. Die gültige Adressverwaltung als `src/main.zyl` speichern und prüfen:

   ~~~bash
   zelyra check src/main.zyl
   ~~~

6. Die URL in die Prozessumgebung laden und die Verbindung testen. `.env` wird
   von Zelyra nicht automatisch geladen:

   ~~~bash
   set -a
   . ./.env
   set +a
   zelyra doctor src/main.zyl --json
   ~~~

7. Das gewünschte SQL erzeugen und vor dem Einsatz lesen:

   ~~~bash
   zelyra db create src/main.zyl > sql/addresses.generated.sql
   sed -n '1,160p' sql/addresses.generated.sql
   ~~~

8. Ist-Schema und Plan ansehen:

   ~~~bash
   zelyra db inspect src/main.zyl
   zelyra db plan src/main.zyl
   ~~~

9. Nach manueller Prüfung anwenden:

   ~~~bash
   zelyra db apply src/main.zyl
   ~~~

10. Eine Webanwendung starten. Für eine CRUD-Route braucht die Quelldatei
    zusätzlich eine vom Server akzeptierte Webdefinition; `crud` allein ist
    noch kein statischer Export:

    ~~~bash
    zelyra serve src/main.zyl 127.0.0.1:3000
    ~~~

Unter Windows ersetze `cp` durch `Copy-Item` und lade Variablen beispielsweise
so in PowerShell:

~~~powershell
Copy-Item .env.example .env
$env:DATABASE_URL = 'mariadb://zelyra:HIER_LOKALES_PASSWORT_EINTRAGEN@127.0.0.1:3307/adressverwaltung'
zelyra doctor src/main.zyl --json
~~~

Der Generator bindet in der ausgelieferten Compose-Vorlage standardmäßig Port
`3306`; `3307` ist dieses Handbuchs ein bewusst gewählter, kollisionsarmer
Host-Port. Passe die Compose-Portzeile an, bevor du den Schnellstart kopierst.

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

✅ Eine vollständige, sichere Webschicht mit typisierten Datenbindungen, Query-Steuerungen und Komponenten ist implementiert.

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

Pfadwerte werden standardmäßig HTML-escaped. Der derzeitige Web Core umfasst GET-Routen, Pfadparameter, Query-String-Behandlung, HTTP-Parsing und HTML-Antworten.

### Typisierte Datenbindung und Collections in Views

View-Interpolationen werden vor dem Serverstart statisch validiert. Eine Seite kann Routenparameter nutzen, geladene Datensätze über typisiertes SQL abrufen und auf deren Felder geprüft zugreifen:

~~~zelyra
page "/customers/{name}" {
    load customer = sql<Customer> {
        SELECT id, name FROM customers WHERE name = :name
    }
    html { <h1>{customer.name}</h1> }
}
~~~

SQL wird gegen das Schema geprüft, Parameter werden sicher gebunden und Capabilities sowie Berechtigungen vorab erzwungen. Collections können mit einer typisierten serverseitigen Schleife gerendert werden:

~~~zelyra
page "/customers" {
    load customers = sql<Customer[]> { SELECT id, name FROM customers }
    html { <ul>for customer in customers { <li>{customer.name}</li> }</ul> }
}
~~~

### Deklarative Query-Steuerungen: Suche, Sortierung, Pagination und Filter

Seiten können typisierte Query-Eingaben für ausdrückliches SQL deklarieren:

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

Für Page-Collections, die Suche, Filterung, Sortierung oder Pagination deklarieren, erzeugt Zelyra automatisch semantische Formulare und bewahrt den URL-Zustand:

~~~zelyra
page "/customers" {
    search { name email }
    sort { name }
    paginated 25
    filter { name quantity }
    load customers = sql<Customer[]> { SELECT id, name, quantity FROM customers }
    html {
        <p>Seite: {page} von {pages} (Gesamt: {total})</p>
        <p>Sortierung: {sort} ({order})</p>
    }
}
~~~

- `search { name email }`: Der Compiler prüft die Whitelist der durchsuchbaren Felder. Suchbegriffe werden sicher parametrisiert als `LIKE`-Bedingungen gebunden.
- `sort { name }`: Akzeptiert nur deklarierte Ergebnisfelder, `order` nur `asc` oder `desc` (z. B. `/customers?sort=name&order=desc`).
- `paginated 25`: Validiert `page` als positive Ganzzahl, nutzt parametrisiertes `LIMIT`/`OFFSET` und stellt `page`, `pages` und `total` als `UInt`-Werte bereit.
- `filter { ... }`: Unterstützt typisierte Operatoren: Textfelder unterstützen `eq`, `contains`, `starts_with`, `ends_with` und Null-Checks; numerische Felder zusätzlich `gt`, `gte`, `lt`, `lte`. Beispiele: `/customers?filter_name__contains=Acme` oder `/customers?filter_quantity__gte=10`. Unbekannte Felder und ungültige Werte werden mit kontrolliertem HTTP 400 beantwortet.

### Benannte Views und Komponenten

Ein benannter View bietet ein wiederverwendbares Seiten-Layout. Er deklariert Slots, in die Seiteninhalte eingefügt werden:

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

Typisierte Komponenten deklarieren Eigenschaften mit `props`:

~~~zelyra
component Badge {
    props { text: String }
    html { <span class="badge">{text}</span> }
}

page "/status" {
    html { <Badge text="Ready" /> }
}
~~~

Komponenten unterstützen Default-Slots und benannte Slots mit Fallback-Inhalten:

~~~zelyra
component Panel {
    html {
        <section class="panel">
            <header><slot name="header">Standard-Kopfzeile</slot></header>
            <div class="body"><slot /></div>
        </section>
    }
}

page "/dashboard" {
    html {
        <Panel>
            <slot name="header"><h1>Mein Dashboard</h1></slot>
            <p>Hauptinhalt des Panels.</p>
        </Panel>
    }
}
~~~

*Hinweis:* Verschachtelte Komponenten-Slots innerhalb einer Komponentenverwendung benötigen nicht fälschlich ein übergeordnetes Seiten-`view:`-Layout.

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
/machines?sort=number&order=asc
/machines/new
/machines/42/edit
~~~

### CRUD-Ressourcen mit wiederverwendbaren View-Layouts (`layout: ViewName`)

CRUD-Definitionen können über das Attribut `layout: ViewName` in das globale Seitenlayout eingebunden werden:

~~~zelyra
view AppLayout {
    html {
        <html>
            <head><title><slot name="title">Verwaltung</slot></title></head>
            <body>
                <nav><slot name="nav">Standard-Navigation</slot></nav>
                <main><slot name="content" /></main>
                <aside><slot name="actions" /></aside>
            </body>
        </html>
    }
}

crud Machine {
    table machines
    layout: AppLayout
}
~~~

Das System befüllt die benannten Slots `title`, `nav`, `content` und `actions` automatisch mit den generierten CRUD-Ansichten.

Erzeugte CRUD- und Tableview-Steuerungen verwenden semantische Fieldsets und getrennte Beschriftungen für Operator und Wert jedes Filters. Filterverarbeitung und bewahrte Pagination-URLs verwenden eine deterministische Reihenfolge.

🗺️ Vollständig eigene typisierte Komponenten und feingranulare View-Overrides
sind Teil der weiteren View-Roadmap.

### CRUD-Ansichten, Aktionen und Soft Delete

🧪 Die aktuelle CRUD-Schicht lässt sich innerhalb der sicheren Standardpfade
gezielt anpassen. Eine Listenansicht kann beispielsweise als Kartenansicht
erscheinen, ohne Suche, Filter, Sortierung, Pagination, Escaping oder
Berechtigungsprüfung zu verlieren:

~~~zelyra
crud Customer -> customers {
    view {
        list {
            mode: cards
            empty: "Keine Kunden gefunden."
        }
        detail {
            mode: cards
            title: "Kundendetails"
        }
        form {
            mode: cards
            title: "Kundenformular"
            submit: "Kunden speichern"
        }
        delete {
            title: "Kunden löschen"
            message: "Dieser Vorgang kann nicht rückgängig gemacht werden."
            submit: "Jetzt löschen"
        }
        loading { message: "Kunden werden geladen ..." }
        error {
            title: "Kunden nicht verfügbar"
            message: "Bitte später erneut versuchen."
        }
    }
}
~~~

Für den häufigen Fall kann ein gemeinsames Feldprofil die erzeugte Liste,
Detailansicht sowie Create-/Edit-Formulare einheitlich steuern:

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
Wenn die technische `id`-Spalte in `fields` ausgeblendet ist (wie im obigen
Beispiel mit `name email active`), verlinkt Zelyra in generierten CRUD-Listen
automatisch das erste angezeigte Feld (`name`) mit der Detailseite des
Datensatzes. Das gilt sowohl für Tabellen- als auch für Kachel-Layouts.

🧪 Edit-Formulare, deren Name auf `Edit` endet, enthalten nun einen signierten
Stand der angezeigten Werte. Beim Absenden sperrt Zelyra die Zeile in derselben
MariaDB-Transaktion wie Formularaktion und Audit-Einträge. Wurde ein Wert seit
dem Laden geändert, antwortet Zelyra mit `409 Conflict` und führt die Aktion
nicht aus. Dies schützt den erzeugten Bearbeitungspfad vor veralteten
Übermittlungen; Schreibzugriffe außerhalb der Formularaktion sind davon nicht
abgedeckt, und es entsteht kein allgemeiner Transaktionsvertrag für die
Anwendung.

Eigene fachliche Aktionen bleiben POST-only, parametrisiert und geschützt:

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

Die Laufzeit erzwingt für solche Aktionen Datenbank-Capability,
CSRF-Schutz, Authentifizierung und die deklarierte Berechtigung. Aktionen
können zusätzlich typisierte Felder, eine Bestätigungsseite sowie eigene
Erfolgs- und Fehlerseiten erhalten. Das ist eine Erweiterung der vorhandenen
CRUD-Runtime, kein frei programmierbarer Frontend-Generator.

Für reversible Löschungen gibt es eine Soft-Delete-Konfiguration:

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

Normale Listen und Details zeigen nur Zeilen mit `NULL`; archivierte Datensätze
sind über `?archived=true` erreichbar und können über eine CSRF-geschützte
Restore-Aktion wiederhergestellt werden. Endgültiges Löschen,
Aufbewahrungsregeln und Massenarchivierung sind weiterhin geplant.

Wenn eine Auth-Definition eine Audit-Tabelle angibt, schreiben CRUD-Erstellen,
Ändern, Löschen, Archivieren, Wiederherstellen und eigene Aktionen ihre
Ereignisse in derselben MariaDB-Transaktion. Passwörter, Tokens, Geheimnisse
und Hashes werden aus Änderungsdetails entfernt.

## 13. Authentifizierung und Berechtigungen

🧪 Zelyra unterstützt Argon2-Login, persistente MariaDB-Sessions, Logout,
Routenschutz und datenbankgestützte Berechtigungsprüfungen. Fünf
Fehlversuche für dieselbe normalisierte E-Mail-Adresse innerhalb von 15
Minuten lösen eine 60-sekündige HTTP-429-Sperre aus. Ein erfolgreicher Login
rotiert das vorherige Session-Token dieses Browsers und entwertet es.

Der Login-Grenzwert und die Sperrdauer lassen sich in `auth` konfigurieren:
`login_rate_limit: 5 per 900` setzt fünf Fehler je 900 Sekunden,
`login_block_seconds: 60` eine Sperre von 60 Sekunden. Das sind die Defaults.
Zeitfenster und Sperrdauer dürfen jeweils 1 bis 86400 Sekunden betragen; die
Versuchsanzahl darf 1 bis 1000 sein. Die Begrenzung liegt im Prozessspeicher,
wird beim Neustart zurückgesetzt und ersetzt keinen verteilten
Betriebs-Limiter. Die Tabelle ist auf 4096 Schlüssel begrenzt; bei voller
Tabelle werden neue Schlüssel abgelehnt, bis abgelaufene Zeitfenster bereinigt
wurden.

Jedes schreibende Browserformular benötigt sein CSRF-Token sowie einen
gleichursprünglichen `Origin`- oder `Referer`-Header, der zu `Host` und dem
effektiven Schema der Anfrage passt. Fehlende, fehlerhafte oder fremde Angaben
werden abgelehnt. Ein aus einem anderen Browser kopiertes Token reicht somit
nicht für eine Website-übergreifende Formularanfrage. Schreibende API-Anfragen
mit Browser-Origin-Angaben durchlaufen dieselbe Prüfung; Website-übergreifender
API-Zugriff ist nur für einen exakt in der CORS-Richtlinie freigegebenen Origin
möglich. API-Anfragen mit Browser-Origin-Angaben oder Browser-Session-Cookie
durchlaufen diese Prüfung ebenfalls. Das gilt auch für `GET`, weil Handler
noch nicht statisch auf schreibgeschütztes Verhalten beschränkt sind.
Website-übergreifende API-Aufrufe sind nur für einen exakt in der CORS-Richtlinie
freigegebenen Origin möglich; mit Session-Cookie muss CORS zusätzlich
Credentials erlauben. Das aktuelle CSRF-Token gilt pro Prozess und wird noch
nicht einzeln pro Session gespeichert; deshalb sind diese Origin-Prüfungen ein
notwendiger Bestandteil des Schutzes.

Der Zelyra-Server spricht derzeit ausschließlich unverschlüsseltes HTTP.
Schalte ihm einen vertrauenswürdigen TLS-terminierenden Proxy vor, bevor die
Anwendung außerhalb eines lokalen Entwicklungsrechners erreichbar ist. Der
Proxy muss den öffentlichen `Host` erhalten, `X-Forwarded-Proto` mit dem
tatsächlichen externen Schema überschreiben und direkten öffentlichen Zugriff
auf den Anwendungsport verhindern. Zelyra verwendet den Header für die Prüfung
des effektiven Origins und setzt bei HTTPS das Cookie-Attribut `Secure`.
Vertraue an einer öffentlich erreichbaren Proxy-Grenze niemals ungeprüften,
vom Client gelieferten Forwarded-Headern.

Zusätzlich prüft der Server jeden vorhandenen `Host`-Header gegen
`ZELYRA_ALLOWED_HOSTS`. Der Standard erlaubt nur `localhost`, `127.0.0.1` und
`[::1]`; das verhindert unter anderem DNS-Rebinding über frei gewählte Hosts.
Mehrfach vorhandene sicherheitsrelevante Request-Header wie `Host`, `Origin`,
`Referer`, `Cookie` und `Authorization` werden abgelehnt, damit keine
mehrdeutige Auswertung entsteht. Die Antwort-Policy `Referrer-Policy:
same-origin` ermöglicht gleichursprünglichen API-GETs diesen Nachweis, sendet
aber keine Referrer-Informationen an andere Origins.
Für eine eigene Domain oder einen LAN-Host muss der tatsächliche Hostname
explizit in der kommagetrennten `.env`-Einstellung ergänzt werden. Es werden
keine Schemes, Ports oder Wildcards akzeptiert. Prozessumgebung hat Vorrang
vor Projekt-`.env` und Standardwert. Die Allowlist ersetzt weder TLS noch die
Origin-/CSRF-Prüfung.

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
Vergaben und aller Berechtigungen aus den Rollen des Benutzers. In der
integrierten Administrationsansicht übermittelt auch das Formular zum Entziehen
von Berechtigungen das korrekte CSRF-Token und entfernt Berechtigungen
zuverlässig.

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

Eine Origin muss mit `http://` oder `https://` beginnen. Pfade, Query-Strings,
Fragmente, Wildcards und ein abschließender Slash sind nicht erlaubt. Erlaubte
Antworten erhalten `Access-Control-Allow-Origin` und `Vary: Origin`; bei
aktivierten Zugangsdaten kommt `Access-Control-Allow-Credentials: true` hinzu.
Eine Preflight-Antwort liefert HTTP 204 mit den erlaubten Methoden und
angeforderten Headern. Verbotene Origins oder Methoden werden als strukturierte
JSON-Fehler beantwortet; bei einem Methodenfehler enthält die Antwort den
`Allow`-Header.

Ein ausgeblendeter Button ist keine Sicherheitsgrenze. Berechtigungen müssen
serverseitig an der Aktion geprüft werden. Der Browser ist kreativ, besonders
wenn man ihm vertraut.

### Routenübersicht

`zelyra routes` prüft ein Projekt und zeigt deklarierte Seiten und APIs sowie
die daraus erzeugten Formular-, CRUD-, Tableview- und Authentifizierungsrouten
mit Methode, Pfad und Quelldatei/Zeile. Die JSON-Ausgabe ist deterministisch
und eignet sich für Werkzeuge:

~~~bash
zelyra routes src/main.zyl
zelyra routes src/main.zyl --format=json
~~~

Gleiche Routenmuster mit derselben HTTP-Methode werden bereits von
`zelyra check`, `build`, `run` und `serve` mit `E-ROUTE-001` abgelehnt. Derselbe
Pfad mit verschiedenen Methoden ist zulässig. Dynamische Parameternamen werden
normalisiert: `/customers/{id}` und `/customers/{slug}` sind dasselbe
Routenmuster. Importierte
Ressourcen erscheinen ebenfalls in der Übersicht.

### API-Eingaben und sichere Antwort-Defaults

API-Bodies für Methoden außer `GET` und `DELETE` dürfen aktuell
`application/json` oder `application/x-www-form-urlencoded` verwenden.
Nicht unterstützte Medientypen liefern HTTP 415 als strukturierten JSON-Fehler.
JSON-Bodies müssen ein Objekt sein; anschließend wird jedes deklarierte Feld
in den Zelyra-Typ umgewandelt und geprüft.

Der HTTP-Parser prüft `Content-Length`, liest vollständige Bodies auch über
mehrere Netzwerk-Reads ein und begrenzt Request-Bodies auf 1 MiB. Header sind
auf 64 KiB begrenzt. Ein zu großer Body wird vor dem Handler mit HTTP 413
abgelehnt.

Eine API kann Versions- und Deprecation-Metadaten sowie ein begrenztes
Kontingent je Client deklarieren:

~~~zelyra
api GET "/api/v1/customers" {
    version "v1"
    deprecated
    rate_limit 100 per 60
    output Customer[]
}
~~~

Antworten geben Version und Deprecation über `X-Zelyra-API-Version` und
`X-Zelyra-API-Deprecated` aus; `zelyra doc --openapi` nimmt dieselben Angaben
auf. Das Kontingent gilt je Route und TCP-Client-IP im aktuellen Prozess. Bei
Überschreitung liefert Zelyra HTTP 429 und `Retry-After`. Zulässig sind 1 bis
1000000 Anfragen pro 1 bis 86400 Sekunden. Der Server vertraut
`X-Forwarded-For` nicht; Clients hinter einem Reverse-Proxy teilen daher dessen
Kontingent. Die Kontingente liegen im Speicher und werden beim Neustart
zurückgesetzt.
Die Tabelle ist auf 4096 Client-/Routenpaare begrenzt; solange sie voll ist,
erhalten neue Paare HTTP 429, bis ein Kontingentfenster abläuft.

Alle HTML-, JSON-, Redirect-, Fehler- und Preflight-Antworten erhalten diese
sicheren Standard-Header:

~~~http
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Referrer-Policy: same-origin
~~~

Diese Defaults ersetzen weder TLS noch Authentifizierung, Autorisierung,
CSRF-Schutz oder eine geeignete Content-Security-Policy.

### Rollenverwaltung und manipulationssichtbares Audit

🧪 Rollen und Rollenberechtigungen können mit den vorhandenen CLI-Befehlen
gepflegt werden, wenn die `auth`-Definition die Tabellen dafür konfiguriert:

~~~zelyra
auth users {
    table: users
    sessions: auth_sessions
    permissions: user_permissions
    roles: user_roles
    role_permissions: role_permissions
    audit: auth_audit_log
    admin_path: "/admin/access"
    admin_permission: "auth.manage"
    admin_role: admin
}
~~~

~~~bash
DATABASE_URL='mariadb://user:passwort@127.0.0.1:3306/app' \
  zelyra auth role grant app.zyl 42 manager
DATABASE_URL='mariadb://user:passwort@127.0.0.1:3306/app' \
  zelyra auth role-permission grant app.zyl manager customers.edit
~~~

`revoke` entfernt die jeweilige Zuordnung wieder. Die Befehle binden Werte als
SQL-Parameter und prüfen zunächst das Projektschema. Verwende für echte
Passwörter niemals den Platzhalter direkt aus diesem Beispiel.

Mit `audit: auth_audit_log` lassen sich Authentifizierungs-, Rollen- und
CRUD-Ereignisse untersuchen oder exportieren:

~~~bash
DATABASE_URL='mariadb://user:passwort@127.0.0.1:3306/app' \
  zelyra audit inspect app.zyl --limit 100
DATABASE_URL='mariadb://user:passwort@127.0.0.1:3306/app' \
  zelyra audit export app.zyl --format json > audit.json
zelyra audit verify app.zyl
~~~

Für eine sichtbare Manipulationserkennung kann die Verkettung aktiviert werden:

~~~zelyra
auth users {
    table: users
    audit: auth_audit_log
    audit_chain: true
}
~~~

Die Audit-Tabelle benötigt dann `id`, `previous_hash` und `entry_hash`,
üblicherweise `String(64)`. Zelyra verwendet kleingeschriebene SHA-256-
Hexwerte. Der Hash bezieht sich auf die kanonische, mit `|` getrennte Folge
`previous_hash|actor_user_id|event|target_user_id|details|created_at`.
`zelyra audit verify` prüft Verknüpfungen und Hashes. Das Bereinigen ist für
verkettete Protokolle absichtlich deaktiviert, weil das Löschen eines Eintrags
die Kette brechen würde. Nicht verkettete alte Einträge können dagegen mit
`zelyra audit prune ... --before ... --confirm` kontrolliert entfernt werden.

Eine optionale Browser-Administrationsseite wird durch `admin_path`,
`admin_permission` und `admin_role` aktiviert. Sie kann Benutzer, Passwörter,
Aktivierung, Rollen und Rollenberechtigungen verwalten; die Formulare sind
CSRF-geschützt. Der Schutz des letzten aktiven Administrators bleibt aktiv.

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
Database Database(read) Database(write) Network FileSystem Environment Process Clock Random Console
~~~

Für Funktions-SQL erlaubt `Database(read)` geprüfte `SELECT`-Anweisungen;
`Database(write)` erlaubt geprüfte `INSERT`-, `UPDATE`- und `DELETE`-
Anweisungen. Nicht klassifizierbares SQL benötigt die breite Capability
`Database`. Die breite Form bleibt für bestehende Projekte verfügbar und
erfüllt beide engeren Effekte. Projekte können die Effekte einzeln freigeben:

~~~zelyra
fn maschinen_liste() -> Machine[] uses Database(read) {
    return sql<Machine[]> {
        SELECT id, number, name FROM machines
    }
}
~~~

~~~toml
[capabilities]
database_read = true
database_write = false
~~~

Das sind Quellfreigaben des Compilers und der Runtime, keine Rechte eines
MariaDB-Kontos. Framework-CRUD, Formulare, Authentifizierung und Page-Daten
benötigen weiterhin `Database`.

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
werden nicht automatisch protokolliert oder veröffentlicht. Die Netzwerk-,
Datei-, Prozess- und Zufalls-APIs sind implementiert, benötigen aber jeweils
eigene Ressourcenfreigaben und bleiben in ihrer ersten Fassung bewusst
eingeschränkt.

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

### Strukturierte Nebenläufigkeit

Für einen ersten eingeschränkten Nebenläufigkeitsablauf gibt es `parallel` und
`await`:

~~~zelyra
parallel {
    customer = await load_customer()
    orders = await load_orders()
}
~~~

Jeder Zweig bindet sein Ergebnis mit `await`. Die Zweige erhalten eine
unveränderliche Momentaufnahme der umgebenden Werte und werden vor der
Fortsetzung in Quelltextreihenfolge zusammengeführt. Ein Fehler in einem Zweig
lässt den gesamten Block fehlschlagen, nachdem die gestarteten Zweige beendet
wurden. `await` außerhalb eines `parallel`-Blocks weist der Type Checker ab.
Die aktuelle Runtime verwendet einen Worker-Thread pro Zweig; Abbruch und die
Verwendung eines Datenbank-Connection-Pools sind noch nicht umgesetzt.

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
version = "0.1.50"
zelyra = "0.1"

[capabilities]
database = true
network = false
~~~

Verbindungen und Passwörter werden über geschützte Umgebungsvariablen bereitgestellt:

~~~bash
export DATABASE_URL='mariadb://user:password@127.0.0.1:3306/zelyra_demo'
~~~

Regeln:
- `.env` niemals in Versionskontrolle committen;
- Produktionszugänge nie in Codebeispiele schreiben;
- Geheimnisse nicht loggen;
- getrennte Datenbanken für Entwicklung, Tests und Produktion verwenden;
- destruktive Tests niemals gegen Produktion ausführen.

### Einfacher Einstieg, optionale Feature-Schalter

Für den einfachen Einstieg ist keine zusätzliche Feature-Konfiguration erforderlich. Erweiterte Projektbereiche können in `zelyra.toml` ausgewählt werden; umgebungsabhängige, nicht geheime Überschreibungen gehören in `.env` oder die Prozessumgebung:

~~~toml
[features]
web = true
api = true
crud = true
auth = true
audit = true
~~~

| Schalter | `.env` / Prozessvariable | Standard | Bedeutung |
|---|---|---:|---|
| `web` | `ZELYRA_FEATURE_WEB` | `true` | Seiten, Formulare und Web-Ressourcen |
| `api` | `ZELYRA_FEATURE_API` | `true` | `api`-Deklarationen und API-Oberfläche |
| `crud` | `ZELYRA_FEATURE_CRUD` | `true` | `crud`-Deklarationen und generierte CRUD-Oberfläche |
| `auth` | `ZELYRA_FEATURE_AUTH` | `true` | `auth`-Deklarationen und Authentifizierungsoberfläche |
| `audit` | `ZELYRA_FEATURE_AUDIT` | `true` | Audit-Konfiguration innerhalb der Authentifizierung |

Auswertungsreihenfolge für Konfigurationswerte:
```text
Prozessumgebung → .env → zelyra.toml → sichere Standardwerte
```

Wenn der Quellcode einen deaktivierten Bereich verwendet, meldet der Compiler `E-FEATURE-001`. Feature-Schalter können niemals Typprüfung, SQL-Prüfung, Capabilities, Contracts, CSRF-Schutz oder Sicherheitsregeln abschalten.

Die wirksame Konfiguration kann jederzeit geheimnisfrei geprüft werden:

~~~bash
zelyra config main.zyl
zelyra config main.zyl --format=json
~~~

### Vollständige `.env`-Referenz des aktuellen Codes

| Variable | Standard im generierten Projekt | Verwendung | Geheim |
|---|---:|---|---|
| `ZELYRA_WEB_PORT` | `3000` | Port des internen Webservers im Container | nein |
| `ZELYRA_HOST_PORT` | `3000` (oder autom. freier Port) | lokal veröffentlichter Webport | nein |
| `ZELYRA_DB_HOST_PORT` | `3306` (oder autom. freier Port) | lokal veröffentlichter MariaDB-Port | nein |
| `ZELYRA_DB_CONNECT_TIMEOUT_SECS` | 0.4-Entwicklungszweig: `10` | MariaDB-Verbindungszeitlimit in Sekunden; gültig `1`–`300` | nein |
| `ZELYRA_DB_QUERY_TIMEOUT_SECS` | 0.4-Entwicklungszweig: `30` | MariaDB-Runtime-Statement-Limit in Sekunden; gültig `1`–`3600`; kein Pool- oder Ergebnisübertragungs-Limit | nein |
| `DATABASE_URL` | projektabhängig | MariaDB-Verbindungs-URI (`mariadb://user:pass@host:port/db`) | ja |
| `MARIADB_DATABASE` | `zelyra_app` | Compose: Datenbankname | nein |
| `MARIADB_USER` | `zelyra` | Compose: Anwendungsbenutzer | nein |
| `MARIADB_PASSWORD` | zufällig erzeugt | Compose: Passwort des Anwendungsbenutzers | ja |
| `MARIADB_ROOT_PASSWORD` | zufällig erzeugt | Compose: MariaDB-Root-Passwort | ja |
| `ZELYRA_AUTH_TOKEN` | keiner | optionaler lokaler Bearer-Token für geschützte Anfragen | ja |
| `ZELYRA_AUTH_PERMISSIONS` | leere Liste | kommagetrennte lokale Berechtigungs-Allowlist | nein |

### Test- und Entwicklungsvariablen

Die Testvariablen mit `ZELYRA_INSTALL_ROOT`, `ZELYRA_BIN`, `*_E2E_*` und `GENERATED_*` dienen internen CI- und lokalen Integrationstests (z. B. `tests/generated-project-docker-e2e.sh`, `tests/sqlite-e2e.sh`). Sie sind keine Anwendungskonfiguration und dürfen nie Produktionszugänge enthalten.

### Umgebungszugriff innerhalb der Sprache

Über die Built-in-Funktion `env(name)` kann Zelyra-Code Werte aus der Umgebung lesen, sofern `uses Environment` und `[capabilities] environment = true` deklariert sind:

~~~zelyra
fn configured_mode() -> String? uses Environment {
    return env("ZELYRA_MODE")
}
~~~

`DATABASE_URL` und sensible Schlüssel dürfen niemals per `env(...)` in ungesichertem Code ausgelesen werden.

## 17. Diagnosen und Fehlersuche

Zelyra möchte Fehler so erklären, dass man nicht erst eine archäologische
Ausgrabung im Stacktrace beginnen muss.

### Verbindung unabhängig testen

Teste zuerst MariaDB ohne Zelyra. Das Passwort wird interaktiv abgefragt und
landet nicht in der Shell-History:

~~~bash
mariadb \
    --host=127.0.0.1 \
    --port=3307 \
    --user=zelyra \
    --password \
    adressverwaltung
~~~

Danach ist `zelyra doctor src/main.zyl --json` (optional mit `--env-file .env`
und `--port 18080`) der vorhandene Zelyra-Test. Es gibt aktuell keinen
`zelyra db check`-Befehl. `doctor` prüft Quellcode, Schema, DB-Verbindung,
Docker Compose und Host-Ports. Ohne `DATABASE_URL` meldet er nur eine Warnung,
bei einer gesetzten, aber nicht erreichbaren Verbindung einen Fehler. Ein
laufender DB-Container allein beweist noch nicht, dass Host, Port, Benutzer und
Datenbank zusammenpassen.

### Diagnosebefehle ohne Geheimnisse

~~~bash
pwd
ls -la
docker compose ps
docker compose logs mariadb
ss -ltn
mariadb --version
~~~

Unter Windows in PowerShell sind `Get-Location`, `Get-ChildItem`,
`docker compose ps` und `mariadb --version` die entsprechenden ersten Schritte.
Gib niemals `DATABASE_URL` oder ein Passwort in eine Diagnoseausgabe aus.

### Typische Fehler

| Fehlermeldung | Wahrscheinliche Ursache | Lösung |
|---|---|---|
| `Permission denied` | fehlende Dateirechte, falscher Besitzer oder kein Zugriff auf den Client | `ls -la`, `chmod 600 .env` und Installationspfad prüfen |
| `Access denied for user` | Passwort stimmt nicht oder Benutzer ist für einen anderen Host angelegt | `SHOW GRANTS FOR 'zelyra'@'127.0.0.1';` prüfen; Passwort rotieren |
| `Connection refused` | auf Host/Port lauscht kein Dienst | `docker compose ps`, `ss -ltn` und den veröffentlichten Port prüfen |
| `Can't connect to server` | falscher Host, falscher Port oder Container noch nicht bereit | `docker compose logs mariadb`; vom Host `127.0.0.1:3307`, im Compose-Netz `mariadb:3306` verwenden |
| `Unknown database` | Datenbankname in URL und MariaDB unterscheiden sich | `SHOW DATABASES;` ausführen und `DATABASE_URL` korrigieren |
| falscher Port | außen `3307` mit innen `3306` verwechselt | Host nutzt `3307`, ein Compose-Service nutzt `3306` |
| MariaDB-Container nicht gestartet | Compose-Fehler, belegter Port oder ungesundes Volume | `docker compose ps` und `docker compose logs mariadb` prüfen |
| Benutzer nur für anderen Host freigegeben | `'zelyra'@'localhost'` ist nicht immer `'zelyra'@'127.0.0.1'` | Benutzer exakt für den verwendeten Host anlegen und Grants kontrollieren |
| fehlende Umgebungsvariable | `DATABASE_URL` wurde nicht exportiert | `.env` laden oder Variable für den Prozess setzen; Zelyra lädt sie nicht selbst |
| `.env` wird nicht gefunden | falsches Arbeitsverzeichnis oder Annahme eines automatischen Loaders | `pwd`, `ls -la`; im Projektstamm arbeiten und Variable explizit exportieren |
| ungültiger Zahlenwert beim Port | Portteil der URI ist kein gültiger MariaDB-Port | Ziffern verwenden, zum Beispiel `3307`; die URL wird ansonsten abgelehnt |
| falscher Zeichensatz | Datenbank mit anderem Charset/Kollation angelegt | Datenbankdefinition prüfen; `db setup` nutzt `utf8mb4`/`utf8mb4_unicode_ci` |
| TLS-Fehler | TLS-Parameter wurden an die URL angehängt, werden aber nicht unterstützt | aktuelle CLI-URL ohne TLS-Query-Option nutzen; TLS-Konfiguration ist geplant |
| Testdatenbank wird aus Sicherheitsgründen abgelehnt | Schutzmechanismus wird erwartet, ist aber nicht implementiert | Zelyra verhindert Produktionszugriff in Tests nicht automatisch; Variablen manuell prüfen |
| PostgreSQL-SQL gegen MariaDB | falsches Backend oder nicht passende DDL | Backend in `.zyl` prüfen und `zelyra db create`-Ausgabe vor Anwendung lesen |

Wenn `mariadb` gar nicht gestartet werden kann, nennt Zelyra den Startfehler
des externen Programms. Der CLI-Prozess enthält keinen eigenen MariaDB-Treiber.

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
- nicht erfüllter Contract;
- unvollständige typisierte Lücke (`_`).

Beim Entwickeln kannst du `_` als Platzhalter für einen unfertigen Ausdruck
einsetzen (Typed Hole). `zelyra check` lehnt unfertigen Code für den Bau zwar
ab, liefert aber kontextbezogene Diagnosen: erwarteter Typ, sichtbare Variablen
und Funktionen, aktive Capabilities, Contract-Pflichten und Quelltextposition.

Wenn `DATABASE_URL` fehlt, funktionieren reine Sprachprüfungen weiterhin.
Datenbankoperationen melden den fehlenden Zugriff kontrolliert. Das `run`-
Kommando verwendet ohne Variable die reine Runtime; `serve` startet zwar die
Routen, datenbankabhängige Seiten antworten aber mit einem kontrollierten
Fehler.

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

## 20. Zelyra im Vergleich zu Rust

### Die wichtigste Aussage zuerst

Zelyra wird mit Rust entwickelt. Der Zelyra-Compiler, die Sprachmodule und
Teile der Laufzeit liegen als Rust-Crates im Compiler-Repository. Das bedeutet
nicht, dass Zelyra Rust verändert oder dass Zelyra-Anwendungsprogramme Rust-
Programme sind.

> **Zelyra ist nicht verändertes Rust. Zelyra ist eine eigenständige Sprache,
> deren Compiler und Laufzeit in Rust entwickelt werden.**

Der Rust-Compiler wird für Zelyra nicht geforkt und nicht um Zelyra-Schlüssel-
wörter erweitert. Zelyra ist auch keine Rust-Bibliothek und kein Präprozessor,
der gewöhnlichen Rust-Code in etwas anderes umschreibt. Eine `.zyl`-Datei wird
vom eigenen Zelyra-Lexer und -Parser gelesen, in eigene AST-/HIR-Strukturen
überführt, typgeprüft und anschließend von der Zelyra-Runtime verarbeitet.

Die Ähnlichkeit bei `fn`, geschweiften Klammern, `if`, `match` oder statischer
Typisierung ist eine Designentscheidung, aber kein Abstammungsnachweis. Entschei-
dend sind Grammatik, Semantik und Programmiermodell. Zelyra ist derzeit noch
ein experimenteller Prototyp; die Eigenständigkeit wächst mit der Umsetzung des
eigenen Typsystems, der SQL-Prüfung, der Runtime und der Zelyra-spezifischen
Konstrukte.

Die Aussagen in diesem Kapitel wurden gegen den aktuellen Quellcode geprüft:
Lexer und `TokenKind` definieren die Zelyra-Tokens, der Parser erzeugt eigene
AST-Strukturen, und die nachfolgenden Module übernehmen Auflösung,
Typprüfung, Contract-Prüfung, SQL-Analyse, Webverarbeitung und Runtime. Die
CLI- und Datenbankmodule wurden ebenfalls berücksichtigt. Wo ein Merkmal nur
als Token, AST-Knoten oder Zielbild vorhanden ist, wird es nicht als vollständig
ausführbare Spracheigenschaft ausgegeben.

### Allgemeiner Vergleich

| Bereich | Rust | Zelyra | Wesentlicher Unterschied | Zelyra-Status |
|---|---|---|---|---|
| Sprachkategorie | universelle System- und Anwendungssprache | eigenständige deklarative Sprache für Business- und Webanwendungen | andere Grammatik und Semantik | 🧪 |
| Haupteinsatzgebiet | Systeme, Services, CLI, Embedded, WebAssembly | datenbankgestützte Business- und Webanwendungen | Zelyra bündelt Fachanwendungsebenen | 🧪 |
| Compiler | `rustc`, Cargo-Ökosystem | eigenes Rust-Programm im Zelyra-Repository | Rust kompiliert den Compiler; `rustc` kompiliert nicht `.zyl` | ✅ |
| Laufzeit | Rust-Code läuft nativ oder über gewählte Runtime | eigene Zelyra-Runtime, in Rust implementiert | Zelyra führt eigene Werte und Regeln aus | 🧪 |
| Speicherverwaltung | Ownership, Borrowing, Lifetimes | für Zelyra-Code weitgehend automatisch verborgen | kein Rust-Borrow-Checker im `.zyl`-Programm | 🧪 |
| Ownership | zentrale Rust-Semantik | kein entsprechendes `.zyl`-Konstrukt | Speicherregeln sind nicht dieselben | 🗺️ |
| Borrowing | Referenzen und Borrow-Checker | kein entsprechendes `.zyl`-Konstrukt | keine Rust-Referenzsyntax | 🗺️ |
| Lifetimes | explizite oder inferierte Lebensdauern | keine Lifetime-Syntax | Zelyra legt diese Ebene derzeit nicht offen | 🗺️ |
| statische Typisierung | sehr ausgereift, generisch und trait-basiert | eigener statischer Typechecker mit `Int`, `String`, `Option`, Records usw. | Zelyra-Typen sind nicht Rust-Typen | ✅ |
| Nullfähigkeit | `Option<T>` | `T?`, etwa `Email?` | Zelyra kann daraus Schema-/Formregeln ableiten | ✅ |
| Fehlerbehandlung | `Result<T, E>`, `Option<T>`, `?`-Operator | `Result<T, E>`, `Some`/`None`, Laufzeitdiagnosen | kein Rust-`?`-Operator als Zelyra-Syntax | ✅ |
| Datenbankintegration | externe Crates wie SQLx, Diesel oder SeaORM | vorgesehener Bestandteil von Sprache, CLI und Runtime | anderes Integrationsmodell; aktuell externer Client | 🧪 |
| SQL | Bibliotheken, Makros oder Strings | natives `sql<T> { ... }` mit Schema-/Parameterprüfung | Zelyra kennt SQL als AST-Ausdruck | ✅ |
| MariaDB-Schema | nicht Aufgabe der Rust-Sprache | Tabellen werden in Zelyra beschrieben | Generator erzeugt sichtbares SQL | 🧪 |
| Formulare | Framework, Templates und Validierung selbst verbinden | `form`-Konstrukt und Tabellenregeln | Standardpfad ist Teil des Sprachmodells | 🧪 |
| CRUD | muss programmiert oder über Frameworks erzeugt werden | deklaratives `crud Name -> table` mit gemeinsamem View-Feldprofil | aktuelle Runtime stellt CRUD-Routen bereit | 🧪 |
| Views | externe Bibliotheken oder Frameworks | `page`/`html`, benannte `view`-Layouts, typisierte `component`-Bausteine und benannte Slots mit Fallbacks sind vorhanden | kein Rust-Äquivalent; freie Styling-Komponenten fehlen noch | 🧪 |
| Quellformatierung | `cargo fmt`, `rustfmt` | `zelyra fmt <file.zyl> [--check]` | deterministischer Zelyra-Formatter, schützt SQL und HTML | ✅ |
| Refactoring/Wirkungsanalyse | `rust-analyzer`, Compiler-APIs | `zelyra impact`, `zelyra edit` | versionierte, atomare JSON-Maschinenschnittstellen | 🧪 |
| Authentifizierung | externe Web-/Auth-Crates | Auth-Definition, Sessions und Berechtigungsprüfungen im Webmodul | Zelyra bündelt den Standardfall | 🧪 |
| Berechtigungen | selbst entworfene Typen und Middleware | `requires auth`, `permits` und CRUD-Aktionsrechte | deklarative Regeln werden serverseitig geprüft | 🧪 |
| Contracts | manuell oder über Bibliotheken | native `requires {}` und `ensures {}` plus `verify` | Contract-Syntax gehört zu Zelyra | ✅ |
| Capabilities | APIs und Bibliotheken regeln Effekte | `uses Database`, `uses Network` usw. | sichtbare Effektdeklaration ist Sprachbestandteil | ✅ |
| E-Mails | externe SMTP-/Mail-Crates | kein integriertes E-Mail-Konstrukt | nicht mit `uses Email` vortäuschen | ❌ |
| Jobs | externe Job-/Queue-Systeme | kein Hintergrundjob-Konstrukt | keine stabile Job-Syntax | ❌ |
| Audit | externe Logs oder Audit-Crates | Audit-Tabelle, CLI-Auswertung und optionale Hash-Kette | an Auth/CRUD gebunden und noch experimentell | 🧪 |
| Deployment | Cargo, Container, CI und Infrastruktur frei wählbar | generierte Docker-/Compose-Vorlage vorhanden | Vorlage ist Entwicklungsstart, keine Produktionsplattform | 🧪 |
| Produktionsreife | Rust ist breit produktiv eingesetzt | Zelyra Compiler 0.3.0 ist experimentell | Reife und Ökosystem sind nicht vergleichbar | 🧪 |
| Ökosystem | sehr groß: Crates, Tools, Frameworks | kleines eigenes Repository und wenige Integrationen | Zelyra kann Rust-Crates nicht direkt importieren | 🧪 |

Rust ist also das technische Fundament, nicht die Anwendungssprache hinter
Zelyra. Eine Rust-Struktur `Customer` und eine Zelyra-Tabelle `customers` können
ähnliche Daten beschreiben, erzeugen aber nicht dasselbe Verhalten.

### Ausführlicher Syntaxvergleich

Die Statusangabe `✅` bedeutet in diesem Kapitel: im aktuellen Quellcode
vorhanden und mit der installierten Rust-Toolchain beziehungsweise dem Zelyra-
CLI geprüft. `🧪`, `🗺️` und `❌` kennzeichnen weiterhin eingeschränkte,
geplante oder derzeit nicht verfügbare Sprachmerkmale.

| Sprachmerkmal | Zelyra-Syntax | Rust-Syntax | Semantischer Unterschied | Zelyra-Status |
|---|---|---|---|---|
| Dateiendung | `app.zyl` | `main.rs` | eigener Lexer und eigener Dateityp | ✅ |
| Programmeinstieg | `fn main() { ... }` | `fn main() { ... }` | gleiche Schreibweise, andere Sprache | ✅ |
| Funktionsdefinition | `fn add(a: Int) -> Int { ... }` | `fn add(a: i64) -> i64 { ... }` | Zelyra-Typen sind eigene AST-Typen | ✅ |
| Parameter | `name: String` | `name: String` | ähnliche Position, andere Typsemantik | ✅ |
| Rückgabetyp | `-> Int` | `-> i64` | Zelyra abstrahiert den Geschäftstyp | ✅ |
| Rückgabewert | `return value` | `value` oder `return value;` | letzter Rust-Ausdruck ist Rückgabewert | ✅ |
| unveränderliche Variable | `value = 1` | `let value = 1;` | Zelyra bindet ohne `mutable` unveränderlich | ✅ |
| veränderliche Variable | `mutable value = 1` | `let mut value = 1;` | Veränderbarkeit wird anders markiert | ✅ |
| Ganzzahl | `Int` oder `UInt` | `i32`, `i64`, `u32`, `u64` | Rust verlangt konkrete Breite; Zelyra abstrahiert derzeit | ✅ |
| Dezimalzahl | `Float` oder `Decimal` | `f64` oder `f32` | Größe und genaue Überlaufregeln von Zelyra sind noch nicht vollständig spezifiziert | 🧪 |
| Boolean | `true`, `false`, `Bool` | `true`, `false`, `bool` | eigenes Zelyra-Basistypmodell | ✅ |
| Zeichenkette | `String` und Zeichenliterale | `String`, `&str`, Zeichenliterale | Rust unterscheidet Besitz und Borrowing | ✅ |
| optionale Werte | `Email?` oder `Option<String>` | `Option<String>` | `?` ist Zelyras Kurzform für Option | ✅ |
| fehlender Wert | `None` | `None` | gleiche Bezeichnung in verschiedenem Enum-Modell | ✅ |
| Listen/Arrays | `Int[]`, `[1, 2, 3]` | `Vec<i64>`, `vec![1, 2, 3]` | Zelyra bietet keine Rust-Makrosyntax | ✅ |
| benannte Datentypen | `type CustomerId = Id`, `struct Customer { ... }` | `type CustomerId = u64`, `struct Customer { ... }` | Zelyra-Records und Rust-Structs sind nicht austauschbar | ✅ |
| Bedingungen | `if ok { ... }` | `if ok { ... }` | Blocksemantik und Ausdrucksregeln unterscheiden sich | ✅ |
| `else` | `else { ... }` | `else { ... }` | ähnliche Kontrollflussform | ✅ |
| `match` | `match value { Some(x) => ... None => ... }` | `match value { Some(x) => ..., None => ... }` | Zelyra verlangt ebenfalls vollständige Fälle | ✅ |
| Schleifen | `for item in items`, `while`, `loop` | `for item in items`, `while`, `loop` | Zelyra unterstützt keine Rust-Iteratortraits | ✅ |
| Funktionsaufrufe | `add(1, 2)` | `add(1, 2)` | gleiche Oberfläche, andere Auflösung | ✅ |
| Ausgabe | `print(value)` | `println!("{}", value);` | Rust verwendet ein Makro mit `!` | ✅ |
| Kommentare | `// Kommentar` | `// Kommentar`, `/* ... */` | aktueller Zelyra-Lexer hat Zeilenkommentare | ✅ |
| Zeilenumbrüche | meist Trennzeichen; nach Operatoren fortsetzbar | meist Whitespace | Parserregeln sind eigenständig | ✅ |
| Semikolons | werden als Statementtrenner akzeptiert, aber nicht benötigt | häufig Statementtrenner | Zelyra ist nicht semikolonpflichtig | ✅ |
| Blockstruktur | `{ ... }` | `{ ... }` | Klammern bestimmen in beiden die Blöcke | ✅ |
| Einrückung | Lesbarkeit, keine Blocksemantik | Lesbarkeit, keine Blocksemantik | Leerzeichen/Tabs werden nicht zu Python-Blöcken | ✅ |
| Fehlerbehandlung | `Result<T, E>`, `Some`/`None` | `Result<T, E>`, `?`, `panic!` | Zelyra hat keinen Rust-Operator `?` | ✅ |
| Stringinterpolation | HTML kann `{name}` in `html`-Bodies verwenden | `format!("{name}")` oder `println!("{}", name)` | keine allgemeine Zelyra-Stringinterpolation dokumentieren | 🧪 |
| Module | `pub fn`, `pub view`, `pub component` und weitere Deklarationen in importierten Dateien (Entwicklungszweig) | `mod name {}`, Dateien und Module | experimentell; Release 0.3.0 hat keine Module; UI-Sichtbarkeit und Namespace-Auflösung bleiben eingeschränkt | 🧪 |
| Imports | `import "src/math.zyl" as math`, `math::add()` (Entwicklungszweig) | `use crate::module::Item;` | projektlokale Imports; `check`, `build`, `run`, `serve`, `context`, `verify`, `impact` verarbeiten den Graphen | 🧪 |
| Generics | `Option<T>`, `Result<T, E>` und begrenzte Built-in-Typargumente | allgemeine Generics und Traits | keine benutzerdefinierten Zelyra-Generics | 🧪 |
| asynchrone Funktionen | `async fn` nicht vorhanden; `await`/`parallel` nur eingeschränkt | `async fn`, `.await`, Futures | kein stabiles Zelyra-Async-Modell | 🧪 |
| Tabellen | `table customers { ... }` | kein Sprachkonstrukt | Zelyra verbindet Tabelle und Schema | ✅ |
| Datenbanktypen | `Id`, `String(100)`, `Email`, `Bool` | Rust-Typen und externe Mapping-Crates | Zelyra erzeugt SQL-Typen aus der Tabelle | ✅ |
| Beziehungen | `department: Department required` | Feld plus eigene Query-/Mappinglogik | Zelyra leitet Fremdschlüssel ab | ✅ |
| SQL-Abfragen | `sql<Customer[]> { SELECT ... }` | String/Makro einer DB-Crate | Zelyra prüft Schema, Parameter und Ergebnis | ✅ |
| Formulare | `form CustomerCreate -> customers { ... }` | kein natives Formular | Webframework und Validierung nötig | ✅ |
| CRUD | `crud Customer -> customers` | kein natives CRUD | Zelyra-Runtime stellt Standardrouten bereit | 🧪 |
| Views | `page`, `view SiteShell` und `component Badge` | kein natives View-Konstrukt | benannte Views/Komponenten sind vorhanden; `input`/`render`/`??` bleiben Zielsyntax | 🧪 |
| Vorbedingungen | `requires { amount > 0 }` | kein eingebautes Äquivalent | Contract ist Teil der Zelyra-Funktion | ✅ |
| Nachbedingungen | `ensures { result >= 0 }` | kein eingebautes Äquivalent | Verifier und Runtime kennen Zelyra-Contracts | ✅ |
| Zugriff auf alte Werte | Noch nicht festgelegt; `old(...)` nicht geparst | ebenfalls kein allgemeiner eingebauter Contractstandard | keine `old`-Syntax vortäuschen | ❌ |
| Capabilities | `uses Database` | kein identisches Sprachkonstrukt | Effekte werden in Zelyra sichtbar deklariert | ✅ |
| E-Mails | Noch nicht festgelegt | externe Crate/API | kein `Email`-Capability-Schlüssel | ❌ |
| Hintergrundjobs | Noch nicht festgelegt | externe Queue-/Runtime-Crate | keine Job-Syntax | ❌ |
| Audit | `auth users { audit: auth_audit_log }` | externe Logging-/Audit-Crate | Zelyra bindet Audit an Auth- und CRUD-Ereignisse | 🧪 |
| API-Definitionen | `api GET "/customers" { ... }` | Router, Handler und Typen separat | Zelyra bündelt Vertrag und Route | ✅ |

### Funktionen: derselbe Gedanke, andere Sprache

Beide folgenden Beispiele sind in ihrer jeweiligen Sprache typische kleine
Funktionen. Das Zelyra-Beispiel entspricht der vom Parser verarbeiteten
Funktionssyntax und wurde mit dem aktuellen CLI geprüft.

~~~zelyra
fn add(a: Int, b: Int) -> Int {
    return a + b
}
~~~

~~~rust
fn add(a: i64, b: i64) -> i64 {
    a + b
}
~~~

Rust verwendet konkrete Ganzzahltypen wie `i32`, `i64`, `u32` oder `u64`.
Zelyra bietet für typische Businesslogik den verständlichen Typ `Int`; die
verbindliche Größe, Überlaufbehandlung und jede Datenbankabbildung müssen noch
vollständig spezifiziert werden.

### Fibonacci

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

~~~rust
fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }

    fibonacci(n - 1) + fibonacci(n - 2)
}

fn main() {
    println!("{}", fibonacci(10));
}
~~~

✅ Die Zelyra-Fassung nutzt `Int`, `print` und explizites `return`; Rust nutzt
`u64`, das `println!`-Makro und den letzten Ausdruck als Rückgabewert. Rust-
Makros tragen das `!`. In Zelyra sind Semikolons nicht erforderlich. Klammern
bestimmen in beiden Beispielen die Blockstruktur, Einrückung dient nur der
Lesbarkeit. Ein Zeilenumbruch nach `+` setzt den Ausdruck fort; der Parser
überspringt an dieser Stelle Zeilenumbrüche. Rekursion funktioniert in Zelyra
nur, weil Funktionsauflösung und Runtime sie tatsächlich unterstützen. Der
Code wurde in diesem Arbeitslauf nicht ausgeführt.

### Optionale Werte

~~~zelyra
email: Email?
~~~

~~~rust
email: Option<String>
~~~

`Email?` ist fachlich kürzer und drückt neben der Optionalität den E-Mail-
Datentyp aus. Rust verwendet den allgemeinen generischen Typ `Option<T>`.
Zelyra kann `Email?` in der Tabellen-, Formular- und SQL-Prüfung berücksichtigen;
eine allgemeine automatische View- oder Validierungsableitung ist jedoch nicht
für jede Oberfläche vorhanden.

### Tabellen

~~~zelyra
table customers {
    id: Id primary auto
    name: String(100) required
    email: Email?
    active: Bool default true
}
~~~

~~~rust
struct Customer {
    id: u64,
    name: String,
    email: Option<String>,
    active: bool,
}
~~~

Die Rust-Struktur erzeugt keine Tabelle, keine SQL-Spalten, keine Validierung,
kein Formular und keine CRUD-Oberfläche. Dafür braucht Rust zusätzliche Crates,
Makros, Queries, Handler und Templates. Die Zelyra-Tabelle wird dagegen in die
Schemaableitung und – wo die Webfunktion vorhanden ist – in Formulare und CRUD
einbezogen.

### CRUD

~~~zelyra
crud Customer -> customers
~~~

Rust besitzt dafür kein natives Äquivalent. Typischerweise kommen in Rust ein
Webframework, Routing, eine Datenbank-Crate, ein Datenmodell, Abfragen,
Request-Typen, Validierung, Handler, Templates oder ein Frontend,
Fehlerbehandlung und Berechtigungsprüfung zusammen. Zelyra parst diese
deklarative Definition und die aktuelle Runtime stellt daraus CRUD-Routen,
Formulare, Suche, Filter und CSRF-geschützte Aktionen bereit. Das ist
experimentell; es ist kein statischer Frontend-Generator.

### SQL

Die Zielsyntax aus dem Auftrag enthielt `with { ... }`. Das ist im aktuellen
Parser nicht vorhanden. Parameter kommen derzeit als Funktionsparameter in die
SQL-Prüfung:

~~~zelyra
struct Customer { id: Int name: String email: Email? active: Bool }

fn active_customers(active: Bool) -> Customer[]
    uses Database
{
    return sql<Customer[]> {
        SELECT id, name, email, active
        FROM customers
        WHERE active = :active
        ORDER BY name
    }
}
~~~

~~~rust
let customers = sqlx::query_as!(
    Customer,
    r#"
        SELECT id, name, email, active
        FROM customers
        WHERE active = ?
        ORDER BY name
    "#,
    true
)
.fetch_all(&pool)
.await?;
~~~

Rust hat SQL nicht als Spracheigenschaft; SQLx oder andere Crates können
zusätzliche Compile-Time-Prüfungen anbieten. Zelyras aktueller SQL-Checker
prüft – wenn Schema und Typen bekannt sind – Tabellen, Spalten, Aliase,
Parameter, Nullfähigkeit, Ergebniszuordnung und `Database`-Capability. Das
macht Zelyra nicht automatisch besser als SQLx. Der aktuelle MariaDB-Generator
und seine fehlenden Tabellenoptionen sind in Abschnitt 7 offen dokumentiert.

### Views: aktuelle Syntax statt Zielbild

Die gezeigte Zielsyntax mit `view`, `input`, `render`, Komponenten und `??` ist
heute nicht Parser-Syntax. Der aktuelle Webkern verwendet stattdessen:

~~~zelyra
page "/customers/{name}" {
    html {
        <h1>Kunde {name}</h1>
    }
}
~~~

Rust besitzt keine eingebaute HTML- oder Komponenten-Syntax; dort kommen
Templates und Webframeworks hinzu. Zelyras `page`/`html`-Form sowie benannte
Views und typisierte Komponenten sind vorhanden. Mehrere Slots, verschachtelte
Komponenten und die Zielsyntax mit `input`/`render` sind noch nicht stabil;
`??` ist keine implementierte Zelyra-Operation.

### Verträge

Die Zielsyntax `require amount > 0` und `old(...)` ist nicht der aktuelle Stand.
Der Parser akzeptiert `requires {}` und `ensures {}`:

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

Rust besitzt hierfür kein direkt eingebautes Äquivalent. `requires` beschreibt
Vorbedingungen, `ensures` Nachbedingungen. Runtime-Prüfung und formale
Verifikation sind verschieden: `zelyra verify` kann `PROVEN`, `RUNTIME_CHECK`,
`UNPROVEN` oder `FAILED` melden. Ein Zugriff auf den alten Wert über `old(...)`
ist nicht implementiert.

### Capabilities

~~~zelyra
fn load_customers() -> Customer[] uses Database {
    return sql<Customer[]> {
        SELECT id, name, email, active FROM customers
    }
}
~~~

`uses` macht erlaubte Seiteneffekte in der Signatur sichtbar. `Database`,
`Network`, `FileSystem`, `Environment`, `Process`, `Clock`, `Random` und
`Console` sind im
aktuellen Runtime-Code bekannte Capabilities. Rust besitzt kein identisches
eingebautes Capability-System; dort werden Zugriffe typischerweise über Typen,
Werte und Bibliotheks-APIs organisiert. `Email` ist keine Zelyra-Capability.

### Was Rust-Kenner in Zelyra nicht suchen sollten

Zelyra soll für typische Businessanwendungen nicht verlangen, im gewöhnlichen
Anwendungscode explizite Lifetimes zu schreiben, Borrowing für einfache
Formulare zu debuggen, zwischen vielen Ganzzahlbreiten zu wählen oder ein
Webframework aus zahlreichen Crates zusammenzustellen. Das ist eine
Abstraktion, keine Behauptung, dass Speicher- und Laufzeitfragen verschwinden.

Von Rust inspiriert sind statische Typisierung, verständliche Diagnosen,
sichere Standardeinstellungen, explizite Veränderbarkeit, Pattern Matching,
Records/Enums, klare Grenzen, reproduzierbares Tooling und formale Prüfungen.

### Was Zelyra eigenständig macht

| Zelyra-Merkmal | Nutzen | Status |
|---|---|---|
| Eine fachliche Definition | weniger widersprüchliche Mehrfachdefinitionen | 🧪 |
| natives geprüftes SQL | Datenbankfehler möglichst vor Ausführung erkennen | ✅ |
| deklaratives CRUD | Standardverwaltungen mit wenig Code | 🧪 |
| `page`/`html`-Webkern | einfache typisierte Pfadwerte und HTML-Antworten | 🧪 |
| sichtbares MariaDB-SQL | nachvollziehbare Schemaänderungen | 🧪 |
| `requires` und `ensures` | Geschäftsregeln ausdrücklich festlegen | ✅ |
| Capabilities | erlaubte Seiteneffekte sichtbar machen | ✅ |
| integriertes Audit | Änderungen nachvollziehen | 🧪 |

### Ehrliches Fazit

> Zelyra sieht an einigen Stellen ähnlich aus wie Rust, weil beide moderne,
> statisch typisierte Sprachen mit geschweiften Klammern und klaren
> Funktionssignaturen sind. Zelyra verfolgt jedoch ein anderes
> Programmiermodell: Datenbank, SQL, Formulare, CRUD, Views und Geschäftsregeln
> sollen Bestandteile eines gemeinsamen Sprachsystems sein. Rust ist das
> technische Fundament des Compilers – nicht die Sprache, die
> Zelyra-Anwendungsentwickler schreiben.

> **Status:** Zelyra ist derzeit ein experimenteller Sprachprototyp. Einige
> gezeigte Sprachmerkmale beschreiben das verbindliche Zielbild und sind noch
> nicht vollständig implementiert. Der Status an jedem Beispiel zeigt, was
> heute im Code tatsächlich vorhanden und geprüft ist.

Weiterführend: [Einführung](#1-was-zelyra-anders-macht),
[Sprachgrundlagen](#5-variablen-typen-und-funktionen),
[MariaDB](#7-mariadb-und-tabellen), [SQL](#9-natives-sql),
[Formulare](#11-formulare), [Views/Webseiten](#10-webseiten), [CRUD](#12-crud),
[Contracts](#15-contracts-und-verify),
[Capabilities](#14-capabilities), [Implementierungsstatus](#17-diagnosen-und-fehlersuche)
und [Roadmap](#22-roadmap-aus-dem-aktuellen-repository). Der laufende Repository-Stand steht
zusätzlich auf der [Statusseite](https://siedelmann.com/status).

## 21. Positionierung und aktueller Entwicklungsstand

Die aktuelle Positionierung in `docs/positioning.de.md` beschreibt Zelyra als
eigenständige Sprache für datenbankgestützte Businessanwendungen. Sie ersetzt
nicht die technische Prüfung im Compiler; sie erklärt, wofür die Bausteine
zusammen gedacht sind.

### Was Zelyra unterscheidet

1. **Eine Quelle der Wahrheit:** Schema, Typen, SQL, Formulare, CRUD, Views und
   APIs sollen aus miteinander prüfbaren Definitionen entstehen.
2. **SQL bleibt First-Class:** SQL wird nicht hinter einer ORM-Abstraktion
   versteckt, sondern als Bestandteil des Programms mit Tabellen, Parametern und
   Ergebnisformen geprüft.
3. **Businessfunktionen sind Sprachbausteine:** Tabellen, Formulare, CRUD,
   Seiten, Authentifizierung, Berechtigungen und Contracts gehören zum selben
   Modell.
4. **Sichere Defaults sind sichtbar:** HTML-Escaping, parametrisierte SQL-
   Werte, CSRF-Schutz, Null-Sicherheit und serverseitige Berechtigungen sind
   keine bloßen Empfehlungen.
5. **Beweise werden ehrlich bezeichnet:** `PROVEN`, `RUNTIME_CHECK`,
   `UNPROVEN` und `FAILED` unterscheiden echte statische Beweise von
   Laufzeitprüfungen und offenen Fällen.
6. **Kurzer Einstieg, vollständige Sprache:** Der deklarative Standardfall ist
   kurz; eigene Funktionen und native SQL bleiben für komplexe Fachlogik
   verfügbar.
7. **Wenig Infrastruktur für den Start:** Der eingebaute Server und die CLI
   sollen den Lern- und Entwicklungsweg ohne Apache, PHP oder ein verpflichtendes
   Framework-Bündel ermöglichen.

Der aktuelle Stand ist trotzdem ein experimenteller Prototyp. Die Roadmap und
die einzelnen Statuszeichen sind deshalb wichtiger als eine allgemeine
Produktbehauptung.

## 22. Roadmap aus dem aktuellen Repository

Die folgende Zusammenfassung stammt aus `docs/ROADMAP.de.md` im aktuellen
Zelyra-Repository. Sie ist eine Entwicklungsplanung, keine Zusage für ein
Release-Datum.

| Bereich | Aktueller Schwerpunkt | Noch offene Ausbaustufen |
|---|---|---|
| Einstieg und Distribution | Quellcode- und Release-Installer (Linux/Windows x86_64 per SHA-256), `zelyra new/init` mit Starter-Templates (`minimal`, `mariadb-crud`, `mariadb-auth`, `mariadb-business`), Docker-/DB-Ports, `zelyra setup`, `zelyra doctor`, E2E-Tests | signierte Binaries, interaktiver Verbindungsassistent, Reverse-Proxy-Automatisierung |
| Sprache und Compiler | Lexer, Parser, AST/HIR, Typprüfung, `Option`, `Result`, Pattern Matching, Ausdrucks-Typed-Holes (`_`), kanonisches `zelyra fmt`; begrenzte projektlokale Importe sind im unveröffentlichten 0.4-Zweig experimentell | umfassendere Modul-Sichtbarkeit und Werkzeug-Integration, Generics, Lücken in Deklarationen und vollständige formale Verifikation |
| Datenbankplattform | MariaDB, SQLite und PostgreSQL im Schema-CLI; typisiertes SQL | weitere Schemaabdeckung, robustere Produktionsabläufe |
| Views und Web | Seiten, benannte Views, Komponenten, Default- und benannte Slots mit Fallback-Inhalten, sicherer Output | Themes, View-Vererbung, freie Styling-Komponenten |
| Formulare und CRUD | Validierung, CSRF, Suche, Filter, Pagination, Aktionen, Soft Delete, gemeinsames CRUD-View-Feldprofil (`view.fields`) | permanente Löschung, Aufbewahrung, Archivierung und breitere View-Anpassung |
| Authentifizierung und Audit | Login, Sessions, Rollen, Berechtigungen, Browser-Admin, Audit und Hash-Kette | Self-Service, noch umfassendere Policy-Verwaltung und Archivstrategien |
| APIs und Integration | typisierte APIs, OpenAPI, TypeScript-Client und CORS | Versionierung, Rate Limits und OAuth-/Integrationsbausteine |
| Verifikation und Betrieb | Contracts, Capability-Prüfung und erste Nebenläufigkeitsbausteine | Abbruch, Timeouts, Datenbank-Pool-Integration und belastbare Performancepfade |
| KI-native Schnittstellen | `zelyra fmt` (Stufe B ✅), Ausdrucks-Lücken `_` (Stufe C 🧪), `zelyra impact` mit `--symbol` (Stufe D 🧪), `zelyra edit` Umbenennung (Stufe E 🧪) | Lücken in Deklarationen, Schema-/Laufzeit-Impact, komplexere Edit-Operationen, KI-Benchmark |
| Qualität und Governance | Tests, Dokumentation und reproduzierbare Prüfungen | breitere Akzeptanzanwendungen und Produktionshärtung |

Nicht als verfügbar dokumentieren: E-Mail- und Hintergrundjob-Systeme,
vollständige Module/Imports, frei definierbare `view`-Komponenten mit mehreren
Slots oder eine automatische Produktionsmigration. Für jeden dieser Bereiche
gilt 🗺️, solange der aktuelle CLI-Code die Funktion nicht vollständig trägt.

Die sinnvollste Reihenfolge für ein eigenes Lernprojekt bleibt daher:

1. `check` und `run` für die Sprachgrundlagen;
2. `db create`, `db inspect`, `db plan` und kontrolliertes `db apply`;
3. eine kleine `page`-, `form`- oder `crud`-Anwendung;
4. erst danach Authentifizierung, Rollen, Audit und API-Integration.

## 23. KI-native Entwicklung

Die aktuellen Architektur- und Spezifikationsdokumente ergänzen ein wichtiges
Prinzip:

> **Die KI schreibt. Zelyra prüft.**

Zelyra soll für Menschen und KI-Systeme gleichermaßen nutzbar sein, bleibt aber
vollständig KI-unabhängig. Der Compiler und die Tests sind die Vertrauensgrenze;
eine plausible Erklärung eines Modells ist kein Korrektheitsnachweis. Für
menschlichen und generierten Code gelten dieselben Prüfungen für Lexer, Parser,
Namen, Typen, SQL, Capabilities, Contracts, Tests und Laufzeit.

### Heute verfügbare Maschinenschnittstellen

🧪 Die JSON-Ausgaben für Werkzeuge verwenden das gemeinsame Format mit
`schema_version: "1"`. Menschliche Ausgabe bleibt Standard; JSON wird nur mit
`--format=json` angefordert:

~~~json
{
    "schema_version": "1",
    "command": "check",
    "success": false,
    "diagnostics": []
}
~~~

Die Ausgabe ist deterministisch. `schema_version` ist verpflichtend; neue
optionale Felder dürfen innerhalb einer Version ergänzt werden, inkompatible
Änderungen benötigen eine neue Version. JSON gehört ausschließlich auf
`stdout`, technische Meldungen auf `stderr`. Source-Spans verwenden
nullbasierte UTF-8-Byte-Offsets, einsbasierte Zeilen-/Byte-Spalten und ein
halb-offenes Intervall. Secrets, Zeitstempel, Zufalls-IDs, absolute
maschinenabhängige Pfade und Live-Datenbankinhalte gehören nicht in diese
Ausgaben.

#### Kanonische Quellformatierung

✅ `zelyra fmt <file.zyl>` erzeugt nach erfolgreichem Lexen und Parsen eine
deterministische Quellformatierung. `zelyra fmt <file.zyl> --check` schreibt
keine Dateien und liefert einen Fehlercode, wenn eine Änderung nötig wäre;
damit kann CI kanonischen Quellcode erzwingen.

~~~bash
zelyra fmt examples/fibonacci.zyl
zelyra fmt examples/fibonacci.zyl --check
~~~

Der Formatter bewahrt Zeilenkommentare und behandelt SQL- und HTML-Blöcke als
opaken Quelltext. Er ist idempotent: Ein bereits formatiertes Dokument erzeugt
byte-identisch dieselbe Ausgabe.

#### Typisierte Lücken (Typed Holes)

✅ Ausdrucks-Typed-Holes mit `_` sind als erste sichere Stufe verfügbar. Der
Compiler meldet Kontexttyp, sichtbare Werte und Funktionen, aktive
Capabilities, Contract-Pflichten und Source-Span:

~~~zelyra
fn double(x: Int) -> Int {
    return _
}
~~~

`zelyra check` meldet die Diagnose `E-HOLE-001` mit dem erwarteten Typ `Int`
und den sichtbaren Bezeichnern. Baubare Befehle (`build`, `run`, `serve`) lehnen
unvollständigen Code vor Lowering und Ausführung ab. Lücken in
Deklarationskontexten bleiben geplant.

#### Strukturierte Projektübersicht

✅ `context` ist schreibgeschützt und verbindet sich nicht mit MariaDB, nutzt kein
Netzwerk, führt keine E-Mail oder Jobs aus und gibt keine Geheimnisse aus:

~~~bash
zelyra context examples/auth_crud_api.zyl --format=json
~~~

Sie meldet deklarierte Funktionen, Tabellen, SQL-Abfragen, CRUD-Ressourcen,
Formulare, APIs und Source-Spans. Im experimentellen 0.4-Entwicklungsstand
enthält `database` außerdem die Datenbankdeklaration, das besitzende
Provider-Modul und je datenbanknutzendem Modul dessen direkte oder transitive
Importbindung. Sichtbar ist nur der Name der Umgebungsvariablen, etwa
`ZELYRA_DATABASE_MAIN_URL` – niemals der Verbindungswert oder ein Passwort.
Bei einer benannten Datenbank weist `fallback_connection_environment` zusätzlich
auf den von der Laufzeit akzeptierten Fallback `DATABASE_URL` hin. Gibt es keine
`database`-Deklaration, ist `DATABASE_URL` die primäre Variable; der Kontext
kennzeichnet die Bindung als `legacy_project_environment` und lässt
`configuration` auf `null`.
Der Bericht weist ausdrücklich eine Verbindung pro Prozess aus; er aktiviert
kein Routing mehrerer Datenbanken.

#### Deterministische Wirkungsanalyse

🧪 Quelltextabhängigkeiten eines Programms lassen sich deterministisch prüfen:

~~~bash
zelyra impact examples/auth_crud_api.zyl --format=json
zelyra impact examples/auth_crud_api.zyl --symbol table:customers --format=json
~~~

Die Wirkungsantwort meldet quelltextbasierte Tabellen, SQL, Formulare, CRUD-
Ressourcen, Views, APIs, Berechtigungen, Contracts und eine deterministische
`references`-Kantenliste für bekannte Beziehungen. Für geladene Projekte listet
sie außerdem Pfad, Importe, Exporte und Deklarationen jedes Moduls sowie
`module_import`-Kanten. Modul-Kanten stammen aus dem validierten Projektgraphen
und haben in dieser Version keine Quelltextspanne. Andere Quelltext-Kanten
enthalten Quelle, Ziel, Art und Quelltextspanne. E-Mail-, Job-, Test- und
Live-Schemaauswirkungen bleiben ausdrücklich leer oder nicht verfügbar; der
Befehl verbindet sich nie mit MariaDB.

Mit `--symbol <kind:name>` kann die Ausgabe auf einen bekannten Knoten wie
`table:customers` oder `module:src/storage.zyl` fokussiert werden. Die
fokussierte Antwort enthält nur direkt verbundene Referenzen und zugehörige
Knoten-IDs. Unbekannte Knoten liefern `E-IMPACT-001` und einen Exit-Code
ungleich null.

#### Atomare semantische Änderungen

🧪 Eine validierte Symbol-Umbenennung kann ohne Änderung des Quelltexts
als Vorschau berechnet werden:

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
genauen Token-Spans und einen deterministischen Quelltext-Fingerprint. Es
meldet außerdem `affected_files`; da diese Version pro Anfrage eine Quelldatei
ändert, enthält die Liste die Entry-Datei. Abhängige Tests und Schema-Referenzen
berechnet die Edit-Vorschau noch nicht. Bei
Funktionsumbenennungen listet `affected_effects` die deklarierten Capabilities
der umbenannten Funktion und betroffener aufrufender Funktionen derselben
Quelldatei. Die Umbenennung selbst ändert diese Deklarationen nicht;
modulübergreifende Effekte sind nicht enthalten.

Für `--apply` muss die Anfrage den Fingerprint aus der Vorschau enthalten; so
wird eine zwischenzeitlich geänderte Datei nicht überschrieben (Stale-Source-
Schutz). Ohne den ausdrücklichen `--apply`-Schalter bleibt es eine reine
Vorschau:

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
unterstützt. Tabellenumbenennungen aktualisieren geprüfte SQL-Tabellenpositionen
(`FROM`, `JOIN`, `INTO`, `UPDATE`), lassen aber Literale, Kommentare,
Parameter und HTML unverändert. Komponenten-Umbenennungen aktualisieren die
Deklaration sowie bekannte öffnende und schließende Komponententags in
HTML-Bodies.

### Sicherheitsgrenze und Benchmark

KI-Werkzeuge dürfen nicht unbemerkt Capabilities hinzufügen, Berechtigungen
erweitern, destruktives SQL ausführen, Diagnosen abschwächen, Tests deaktivieren
oder Geheimnisse ausgeben. Destruktive Schemaänderungen und sicherheitsrelevante
Änderungen brauchen eine sichtbare menschliche Freigabe. Zelyra sendet keinen
Quelltext automatisch an externe KI-Dienste; geplante Integrationen sollen
offen, lokal nutzbar, herstellerneutral und versioniert sein.

Der neue KI-Autorenschaftsbenchmark ist eine Spezifikation in
`docs/benchmarks/ai-authoring.de.md`. Er soll mit versionierten Fixtures und
identischen Aufgaben unter anderem Erstversuchskompilierung, Korrekturschleifen,
Zeit bis zu bestandenen Tests, Tokens, Sicherheitsfehler, übersehene
Abhängigkeiten, unsichere Schemaänderungen und menschlichen Prüfaufwand messen.
Es gibt noch keine veröffentlichten Vergleichsergebnisse. Ein Secret-Leak oder
eine nicht freigegebene destruktive Änderung bleibt ein Sicherheitsfehler und
wird nicht durch vermeintliche Produktivität aufgewogen.
