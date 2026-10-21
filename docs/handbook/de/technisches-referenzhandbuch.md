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
./install.sh --release v1.0.5
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
.\install.ps1 -Release v1.0.5
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

Wähle den Projekttyp vor dem Setup:

- Ein minimales Projekt ohne Datenbank braucht kein `setup`:

  ~~~bash
  zelyra init ./test_a
  cd ./test_a
  zelyra check main.zyl
  zelyra run main.zyl
  ~~~

- Für ein neues MariaDB-Projekt erzeugt `zelyra new test_a --mariadb` die
  benötigten Compose- und Konfigurationsdateien. Danach kannst du mit
  `cd test_a && zelyra setup --all` Datenbank und Schema einrichten. Alternativ
  unterstützt `zelyra init ./test_a --mariadb` die MariaDB-Option beim Init.

Projektpfade sind relativ oder absolut, nicht austauschbar:

~~~bash
zelyra setup ./test_a
zelyra setup /test_a
~~~

Der erste Aufruf sucht ein Unterverzeichnis des aktuellen Ordners, der zweite
das Verzeichnis `/test_a` im Wurzelverzeichnis. Existiert
der angegebene Pfad nicht, meldet Zelyra `E-SETUP-001`; wenn ein gleichnamiges
relatives Verzeichnis existiert, schlägt die Meldung den passenden relativen
Aufruf vor.

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

### Prüfen, ausführen, formatieren und verifizieren

Das dokumentierte, benutzerseitige Kommandozeilenprogramm heißt `zelyra`.
Mit `zelyra check <file.zyl>` prüfst du Syntax, Typen, Verträge und
Capabilities, bevor du ein Programm mit `zelyra run <file.zyl>` ausführst.
`zelyra fmt <file.zyl>` formatiert den Quelltext; `--check` prüft die
Formatierung, ohne die Datei zu ändern. `zelyra verify <file.zyl>` führt die
unterstützten Vertrags- und Invariantenprüfungen aus.

Die aktuelle CLI-Referenz führt keinen Unterbefehl `zelyra test` auf.
Projektbezogene Testroutinen kannst du in einer ausführbaren Quelldatei
ablegen und mit `zelyra run` starten; assertionsbasierte Prüfungen beschreibt
Kapitel 19. Ebenso dokumentiert die Referenz kein separates Programm
`zelyrac build`: Der Compiler wird über `zelyra check` und `zelyra run`
aufgerufen.

LSP steht für Language Server Protocol und bezeichnet den Standard für die
Kommunikation zwischen Editor und Sprachserver. Der dokumentierte Einstieg in
den Editor ist `zelyra editor`, das Studio lokal startet. Einen separaten
Unterbefehl `zelyra lsp` führt die aktuelle CLI-Referenz nicht auf.

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

Typnamen dokumentieren die fachliche Bedeutung von IDs:

~~~zelyra
type MachineId = Id
type OrderId = Id
~~~

Diese Aliase besitzen keine eigene Runtimeidentität. Prüfe ID-Zuordnungen und Mandantengrenzen in der Geschäftslogik; die Bezeichnung allein beweist keinen Zugriffsschutz.

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

### Deklarierte Nummernkreise

Ein `String`-Feld kann für generierte CRUD-Anlegeformulare einen
transaktionalen Nummernkreis deklarieren:

~~~zelyra
table customers {
    id: Id primary auto
    customer_number: String(32) required auto_sequence("KD-{0001}")
    name: String(255) required
}

table quotes {
    id: Id primary auto
    quote_number: String(40) required auto_sequence("AG-{year}-{0001}")
}
~~~

`{0001}` bedeutet einen mindestens vier Stellen breiten, mit Nullen
aufgefüllten Zähler. Der Zähler darf nicht breiter als der reservierte Teil
des Formats werden. `{year}` wird durch das aktuelle UTC-Kalenderjahr ersetzt
und führt pro Jahr einen eigenen Zähler. Zulässig sind Literaltext, genau ein
Zählertoken und höchstens ein Jahrestoken. Unbekannte oder unvollständige
Tokens sowie Formate, die nicht in die String-Spaltenlänge passen, werden
abgewiesen.

Der Schema-Plan ergänzt `_zelyra_number_sequences`. Prüfe den Plan und wende
ihn mit `zelyra db apply main.zyl` bewusst an, bevor das CRUD-Formular
verwendet wird. MariaDB und PostgreSQL unterstützen die Laufzeitvergabe;
SQLite wird mit einer klaren Fehlermeldung abgewiesen. Die Vergabe läuft in
derselben Transaktion wie das Speichern. Vom Browser übermittelte Werte
werden ignoriert, und generierte Nummern sind im Bearbeiten-Formular nicht
schreibbar. Zelyra prüft vorhandene Nummern vor der Vergabe. Nummern sind
nicht lückenlos garantiert und stornierte Belege dürfen nicht still
wiederverwendet werden.

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

6. Die Verbindung testen. `zelyra serve`, `doctor` und projektbezogene
   Datenbankbefehle lesen die Projekt-`.env` selbst. Prozessvariablen haben
   Vorrang; andernfalls wird `ZELYRA_DATABASE_MAIN_URL` vor `DATABASE_URL`
   verwendet. Für Befehle, die Zelyra-Code direkt ausführen, die URL weiterhin
   ausdrücklich in die Prozessumgebung laden:

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
Standalone-Formulare können die URL mit `route: "/kontakt"` überschreiben.
Eine Route kann auch ein Segment erfassen, zum Beispiel
`route: "/abmelden/{token}"`; der erfasste Wert steht SQL-Aktionen als
gleichnamiger gebundener Parameter zur Verfügung (ganzzahlig wirkende Werte
werden als Integer, andere als Strings gebunden). Die Route muss ein absoluter
lokaler Pfad sein. Query-Strings, Fragmente, kodierte Pfade, doppelte
Parameter, ungültige Segmente und Kollisionen mit anderen Routen werden
abgewiesen. GET rendert das Formular weiterhin regulär; POST behält CSRF- und
Validierungsprüfungen.

Für opake Routentokens kann ein Standalone-Formular den Token beim GET prüfen
und im POST innerhalb derselben Transaktion nochmals einen Treffer verlangen,
bevor spätere Anweisungen ausgeführt werden:

~~~zelyra
form Abmeldung -> suppressions {
    route: "/abmelden/{token}"
    verify sql<Int[]> {
        SELECT id FROM suppressions WHERE token_hash = SHA2(:token, 256)
    }

    action unsubscribe {
        require_match
        confirm_page {
            title: "Abmeldung bestätigen"
            message: "Möchtest du diese E-Mails abbestellen?"
            submit: "Abmelden"
        }
        sql<Int[]> {
            SELECT id FROM suppressions WHERE token_hash = SHA2(:token, 256)
        }
        sql {
            UPDATE suppressions
            SET unsubscribed_at = COALESCE(unsubscribed_at, CURRENT_TIMESTAMP)
            WHERE token_hash = SHA2(:token, 256)
        }
        redirect "/abgemeldet"
    }
}
~~~

`verify` akzeptiert nur ein compilergeprüftes `SELECT` und führt es vor dem
Rendern des Formulars in einer schreibgeschützten Transaktion aus. Erfasste
Routenwerte werden gebunden; ohne Treffer folgt HTTP 404. Verifizierte
Token-Seiten erhalten `Cache-Control: no-store`. `require_match` gilt nur für
direkte SQL-Formularaktionen: Die erste Anweisung muss ein `SELECT` sein und
wird in derselben Transaktion wie die folgenden Anweisungen ausgeführt. Gibt
die Abfrage keine Zeile zurück, folgt HTTP 404 und kein Schreibzugriff. Damit
sind idempotente Aktionen möglich, wenn auch das folgende SQL idempotent
formuliert ist; es entsteht keine allgemeine Workflow-Engine. Ein Formular mit
`post_only` besitzt keine GET-Formularroute; eine Seite am selben Pfad übernimmt
GET und muss ihre Daten selbst prüfen. Deshalb darf ein solches Formular kein
`verify` deklarieren. Ein ausführbares MariaDB-Beispiel liegt in
[`examples/csv_mailer_unsubscribe.zyl`](../../examples/csv_mailer_unsubscribe.zyl),
der isolierte Datenbank-/Browserablauf in
[`tests/mariadb-form-route-e2e.sh`](../../tests/mariadb-form-route-e2e.sh).

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

Eigene fachliche Aktionen bleiben POST-only, parametrisiert und geschützt. Eine Action kann mehrere SQL-Anweisungen bündeln, die in einer gemeinsamen MariaDB-Transaktion atomar ausgeführt werden:

~~~zelyra
crud Invoice -> invoices {
    action lock {
        label: "Festschreiben (GoBD)"
        confirm: "Möchten Sie diese Rechnung unwiderruflich festschreiben?"
        permits "invoices.lock"
        field reason: String required
        sql {
            UPDATE invoices SET is_locked = true WHERE id = :id
            INSERT INTO invoice_audit_logs (invoice_id, action, message) VALUES (:id, "locked", :reason)
        }
        success "Rechnung wurde festgeschrieben."
        redirect "/invoices"
    }
}
~~~

#### Ausführungsmodell und Transaktionsverhalten von Actions

- **Atomare Datenbanktransaktion:** Alle SQL-Anweisungen innerhalb von `sql { ... }` laufen gebündelt in einer einzigen MariaDB-Transaktion (`with_decision_transaction`). Schlägt eine Anweisung fehl (z. B. durch einen Constraint-Fehler), wird die gesamte Transaktion atomar zurückgerollt.
- **Parameterbindung:** Parameter wie `:id` werden automatisch aus dem URL-Pfad der CRUD-Ressource gebunden. Über `field <name>: <Typ>` deklarierte Zusatzfelder werden im Bestätigungsdialog als typisierte Eingabefelder gerendert, vorab geprüft und als Parameter `:name` an die SQL-Statements übergeben.
- **Seit 1.2.28:** Actions können `guard when ... { error: ... }` und einen typisierten `execute { ... }`-Body verwenden. Der Compiler prüft `row` anhand der Ressourcentabelle; `session` enthält optionale authentifizierte Mandanten-/Benutzeridentitäten. Die Web-Runtime führt unterstützte Queries innerhalb der gesperrten Request-Transaktion aus und erhält Capability-, Berechtigungs-, CSRF-, Mandanten- und Auditprüfungen. MariaDB- und PostgreSQL-HTTP-E2Es prüfen SQL-Atomarität, konkurrierende Guards und Rollback. Das MariaDB-E2E prüft zusätzlich eine echte PDF-Outbox-Anlage; es wird keine E-Mail versendet.

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
    login_enabled: is_approved
}

page "/admin" {
    requires auth
    permits "machines.manage"

    html {
        <h1>Maschinenverwaltung</h1>
    }
}
~~~

`login_enabled` ist optional. Wenn es angegeben ist, muss es auf eine
erforderliche Bool-Spalte der Benutzertabelle zeigen; nur Datensätze mit dem
Wert `true` können sich anmelden. Ohne diese Angabe bleibt die bestehende
Konvention `active: Bool` erhalten. Der Laravel-Importer setzt
`login_enabled` nur, wenn er eine passende Freigabeprüfung statisch im
verknüpften Login-Controller erkennt. Projektspezifische Admin-Ausnahmen,
Registrierungsabläufe und Zwei-Faktor-Logik müssen weiterhin separat geprüft
und gegebenenfalls als Zelyra-Funktion umgesetzt werden.

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
Database Network FileSystem Environment Process Clock Random Console Email
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
version = "0.9.9"
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

### `.env`-Werte des generierten Projekts

| Variable | Standard im generierten Projekt | Verwendung | Geheim |
|---|---:|---|---|
| `ZELYRA_WEB_PORT` | `3000` | Port des internen Webservers im Container | nein |
| `ZELYRA_HOST_PORT` | `3000` (oder autom. freier Port) | lokal veröffentlichter Webport | nein |
| `ZELYRA_DB_HOST_PORT` | `3306` (oder autom. freier Port) | lokal veröffentlichter MariaDB-Port | nein |
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
| fehlende Umgebungsvariable | der Befehl braucht eine Prozessvariable oder es ist keine Projekt-URL gesetzt | projektbezogenes `serve`, `doctor` und Datenbankbefehle lesen die Projekt-`.env`; für direkte Codeausführung `.env` laden oder Prozessvariable setzen |
| `.env` wird nicht gefunden | falsches Arbeitsverzeichnis oder keine Projekt-Umgebungsdatei | `pwd`, `ls -la`; benötigte Prozessvariable setzen oder Projekt-`.env` anlegen |
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
- sichere benannte E-Mail-Vorlagen und Anhänge;
- persönliche Lesebestätigungen und Empfängerautorisierung im Benachrichtigungszentrum;
- allgemeine Hintergrundaufgaben über die vorhandene begrenzte MariaDB-Outbox hinaus;
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
| E-Mails | externe SMTP-/Mail-Crates | `mail_queue`, `mail_deliver`, `uses Email` | MariaDB-Outbox, TLS, Hostfreigabe und ausdrücklich gestarteter Worker | 🧪 |
| Jobs | externe Job-/Queue-Systeme | kein Hintergrundjob-Konstrukt | keine stabile Job-Syntax | ❌ |
| Audit | externe Logs oder Audit-Crates | Audit-Tabelle, CLI-Auswertung und optionale Hash-Kette | an Auth/CRUD gebunden und noch experimentell | 🧪 |
| Deployment | Cargo, Container, CI und Infrastruktur frei wählbar | generierte Docker-/Compose-Vorlage vorhanden | Vorlage ist Entwicklungsstart, keine Produktionsplattform | 🧪 |
| Produktionsreife | Rust ist breit produktiv eingesetzt | Zelyra ist experimentell | Reife und Ökosystem sind nicht vergleichbar | 🧪 |
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
| Kommentare | `# Kommentar`, alleinstehendes `##` als Blockgrenze | `// Kommentar`, `/* ... */` | `#` ist Standard; `//` bleibt kompatibel; `##`-Blöcke sind nicht verschachtelt | ✅ |
| Zeilenumbrüche | meist Trennzeichen; nach Operatoren fortsetzbar | meist Whitespace | Parserregeln sind eigenständig | ✅ |
| Semikolons | werden als Statementtrenner akzeptiert, aber nicht benötigt | häufig Statementtrenner | Zelyra ist nicht semikolonpflichtig | ✅ |
| Blockstruktur | `{ ... }` | `{ ... }` | Klammern bestimmen in beiden die Blöcke | ✅ |
| Einrückung | Lesbarkeit, keine Blocksemantik | Lesbarkeit, keine Blocksemantik | Leerzeichen/Tabs werden nicht zu Python-Blöcken | ✅ |
| Fehlerbehandlung | `Result<T, E>`, `Some`/`None` | `Result<T, E>`, `?`, `panic!` | Zelyra hat keinen Rust-Operator `?` | ✅ |
| Stringinterpolation | HTML kann `{name}` in `html`-Bodies verwenden | `format!("{name}")` oder `println!("{}", name)` | keine allgemeine Zelyra-Stringinterpolation dokumentieren | 🧪 |
| Module | Noch nicht festgelegt | `mod name {}`, Dateien und Module | keine `mod`-Syntax im Zelyra-Parser | ❌ |
| Imports | Noch nicht festgelegt | `use crate::module::Item;` | keine Importsyntax | ❌ |
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
| E-Mails | `mail_queue` und `mail_deliver` | externe Crate/API | `Email`-Capability; Einreihen in `transaction` mit `Database(write)` | 🧪 |
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
Werte und Bibliotheks-APIs organisiert. `Email` ist die Capability für Anwendungs-Mail. `mail_queue` benötigt zusätzlich `Database(write)` und eine Transaktion; die SMTP-Zustellung prüft den erlaubten Host.

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
und [Roadmap](#22-unterstutzung-und-technische-nachweise). Der laufende Repository-Stand steht
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

## 22. Unterstützung und technische Nachweise

Wähle den Backendpfad anhand der [Datenbankmatrix](https://github.com/sf1976/zelyra/blob/main/docs/database-support.de.md). MariaDB 11.4.13 (sowie 10.11.19, 11.8.9 und 12.3.3), MySQL Server 8.4.11 und PostgreSQL 16.15 bilden die verifizierte Multi-Backend-Servermatrix. Relationales CRUD mit echten Fremdschlüsseln (`fk_...`), Preflight-Waisenabweisung (`error[E-DB-005]`) unter Datenerhalt, relationalen Joins und restriktivem Löschschutz (RESTRICT-Delete-Policy) ist für alle drei Engines dokumentiert und getestet. Auf MySQL sind Schreibsperren über `innodb_lock_wait_timeout` dynamisch an Transaktions-Deadlines gebunden. Native Entscheidungstransaktionen (`with_decision_transaction`) sichern Rollback, Unwind und Deadlines auf einer geliehenen Verbindung ab. SQLite ergänzt lokale Schema- und grundlegende CRUD-Abläufe.

Diese Nachweise gelten für die genannten Abläufe, nicht für beliebige Datenbankfunktionen. Erweiterte Authentifizierungsmerkmale, Mail-Outbox und Benachrichtigungsbeispiele verwenden vorrangig MariaDB und MySQL.

Installation, Upgrade, Rollback, Migration und Wiederherstellung müssen zur eingesetzten Plattform und Serverversion passen. Prüfsummen, SBOMs und Attestierungen helfen, das Release-Artefakt zuzuordnen. Sie bestätigen nicht die fachliche Korrektheit einer Anwendung. Die [Betriebs- und Prüfziele](https://github.com/sf1976/zelyra/blob/main/docs/release-plans/0.9.9.de.md) verlangen reproduzierbare Proben; ein Ziel allein ist kein bestandener Nachweis.

Für ein Lernprojekt empfiehlt sich: zuerst `check` und `run`, dann Schema-Vorschau und Plan, danach Seiten/Formulare/CRUD und schließlich Authentifizierung, Mail und Integrationen.

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
Formulare, APIs und Source-Spans.

#### Deterministische Wirkungsanalyse

🧪 Quelltextabhängigkeiten eines Programms lassen sich deterministisch prüfen:

~~~bash
zelyra impact examples/auth_crud_api.zyl --format=json
zelyra impact examples/auth_crud_api.zyl --symbol table:customers --format=json
~~~

Die Wirkungsantwort meldet quelltextbasierte Tabellen, SQL, Formulare, CRUD-
Ressourcen, Views, APIs, Berechtigungen, Contracts und eine deterministische
`references`-Kantenliste für bekannte Beziehungen. Jede bekannte Kante enthält
Quelle, Ziel, Art und Quelltextspanne. E-Mail-, Job-, Test- und
Live-Schemaauswirkungen bleiben ausdrücklich leer oder nicht verfügbar; der
Befehl verbindet sich nie mit MariaDB.

Mit `--symbol <kind:name>` kann die Ausgabe auf einen bekannten Knoten wie
`table:customers` fokussiert werden. Die fokussierte Antwort enthält nur direkt
verbundene Referenzen und zugehörige Knoten-IDs. Unbekannte Knoten liefern
`E-IMPACT-001` und einen Exit-Code ungleich null.

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
genauen Token-Spans und einen deterministischen Quelltext-Fingerprint.

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

---

## 24. Verbindliche Quellen und Compiler-Prüfung (Source Authority)

> **Grundsatz:** Zelyra ist eine eigenständige Sprache. Parser und geprüfte Tests entscheiden, was existiert.

Bei Widersprüchen gilt immer folgende verbindliche Reihenfolge:

1. **[Formale Sprachspezifikation](https://github.com/sf1976/zelyra/blob/main/docs/specification.de.md) und Phasendokumente**
2. **Compiler-Code:** Lexer-, AST-, Parser-, Namensauflösungs-, Typprüfungs- und semantischer Code in den Crates `lexer`, `parser`, `ast`, `hir`, `cli`, `runtime`, `database`, `web` und `forms`
3. **Offizielle automatisierte Sprach- und Integrationstests:** Workspace-Tests (`cargo test --workspace`), Machine-Interface-Tests und E2E-Shellskripte in `tests/`
4. **Die offizielle Standardbibliothek:** (sobald eigenständig strukturiert)
5. **Offizielle Zelyra-Beispiele:** `.zyl`-Dateien in `examples/`, die mit dem aktuellen Compiler erfolgreich verifiziert wurden
6. **Dokumentation und Handbuch**

### Wichtige Invarianten für Entwickler und KI-Assistenten

- **Roadmap ist Planung, keine Syntax:** Zukünftige Phasenvorschläge dürfen erst nach Implementierung in Lexer/Parser als verfügbare Syntax dargestellt werden.
- **Compiler-Implementierungssprache Rust ist kein Zelyra:** Zelyra wird in Rust entwickelt, aber Rust-Syntax in einer `.zyl`-Datei ist ungültig, es sei denn, die Zelyra-Grammatik definiert sie ausdrücklich.
- **Keine erfundenen Befehle:** Alle CLI-Befehle müssen in `cli/src/main.rs` existieren.
- **Prüfzyklus:** Jede Erweiterung durchläuft Formatierung (`cargo fmt`), Typ- und Crate-Prüfung (`cargo check`), Linter (`cargo clippy`) und Tests (`cargo test`).

---

## 25. Anwendungs-Mail und SMTP

Anwendungs-Mail wird in derselben MariaDB-Transaktion wie eine Geschäftsänderung eingereiht. Der Webrequest wartet nicht auf SMTP. Ein separater Worker stellt fällige Nachrichten zu.

### Projekt und Transport konfigurieren

```toml
[capabilities]
email = true
database = true

[network]
allowed_hosts = ["smtp.example.test"]
```

```dotenv
ZELYRA_SMTP_HOST=smtp.example.test
ZELYRA_SMTP_SECURITY=starttls
ZELYRA_SMTP_PORT=587
ZELYRA_SMTP_FROM="Zelyra <noreply@example.test>"
ZELYRA_SMTP_USERNAME=app-user
ZELYRA_SMTP_PASSWORD=durch-ein-secret-ersetzen
```

`implicit_tls` ist der Standard und verwendet Port 465; `starttls` verwendet Port 587. Zertifikats- und Hostnamenprüfung bleiben aktiv. Optional ergänzt `ZELYRA_SMTP_CA_CERT` ein PEM-Wurzelzertifikat aus einer regulären Datei ohne Symlink bis 128 KiB. `local_plaintext` ist nur für Loopback ohne Zugangsdaten zulässig. Der genaue SMTP-Host muss in der Allowlist stehen. Speichere Zugangsdaten in der lokalen `.env` oder Prozessumgebung; übernimm sie nicht in die Versionsverwaltung.

### Nachrichten einreihen

Speichere folgendes Beispiel als `main.zyl` in einem entsprechend konfigurierten MariaDB-Projekt:

```zelyra
pub fn deliver_mail(event_id: String, event_type: String, payload_json: String) -> Bool uses Email {
  if event_type == "zelyra.mail.send" {
    match mail_deliver(event_id, payload_json) {
      Ok(_) => { return true }
      Err(_) => { return false }
    }
  }
  return false
}

fn main() uses Email, Database(write) {
  transaction {
    mail_queue(
      "customer@example.test",
      "Your Zelyra order",
      "Your order has been recorded.",
      "<p>Your order has been recorded.</p>"
    )
  }
}
```

`mail_queue(to, subject, text, html?)` ist eine Anweisung direkt innerhalb von `transaction`. Die Funktion benötigt `Email` und `Database(write)`. Die API prüft Header-Injection und Größen; Argumente müssen Werte oder nebenwirkungsfreie String-Ausdrücke sein. Bei einer zurückgerollten Transaktion bleibt keine zugehörige Mail eingereiht. HTML-Inhalte musst du vor dem Einreihen passend escapen, wenn sie nicht vertrauenswürdig sind.

### Outbox und Worker

```bash
zelyra outbox setup main.zyl
zelyra run main.zyl
zelyra outbox list main.zyl --limit 25
zelyra outbox run main.zyl --handler zelyra.mail.send=deliver_mail --once
```

Richte die Outbox vor dem ersten Programmlauf ein. `--once` verarbeitet einen fälligen Eintrag; ohne diese Option verarbeitet der Worker zugeordnete fällige Ereignisse, bis keines mehr ansteht. `mail_deliver(event_id, payload_json)` liefert einen `Result`; der Handler bestätigt Erfolg mit `true` und erlaubt Wiederholung mit `false`. Der SMTP-Transport hat ein Zeitlimit von fünf Sekunden und gibt redigierte Fehler aus.

Zustellung erfolgt mindestens einmal. Nach SMTP-Annahme und vor Outbox-Quittierung kann ein Worker ausfallen; dadurch sind Duplikate möglich. Die Ereignis-ID wird als stabile Nachrichten-ID verwendet, garantiert aber keine einmalige Zustellung. Die dokumentierte API unterstützt Text-/HTML-Alternativen über die MariaDB-Outbox. Benannte Templates, Anhänge und PostgreSQL-Mail-Outbox sind nicht als verfügbare API belegt. Siehe die [Mail-Referenz](https://github.com/sf1976/zelyra/blob/main/docs/mail.de.md).

### Persistente Passwort-Reset-Quoten

Zur Härtung gegen Brute-Force- und Denial-of-Service-Angriffe speichert Zelyra Passwort-Reset-Anfragen neustartsicher in der Tabelle `password_reset_limits`. Die Drosselung bleibt auch bei Prozessneustarts aktiv. Abgelaufene Zeitfenster werden in Batch-Zyklen automatisch aufgeräumt, während Diagnoseausgaben für Operatoren strikt sanitisiert bleiben, um keine E-Mail-Adressen oder Tokens im Protokoll preiszugeben.

## 26. Mandantengebundene Benachrichtigungen

Das In-App-Zentrum funktioniert ohne SMTP. Es verwendet authentifizierte Mandantenzugehörigkeit, eine gemeinsame Benachrichtigungstabelle und eine mandantengebundene CRUD-Ressource. Der aktive Mandant begrenzt Listen, direkte Zugriffe und den ungelesenen Zähler. Abgelaufene und archivierte Einträge werden ausgeblendet.

### Vollständiges MariaDB-Beispiel

Speichere das Beispiel in einem MariaDB-Projekt mit aktivierten Datenbank-, Web-, CRUD- und Auth-Funktionen. Richte Schema, Benutzer, Passwort-Hash, Mandantenzugehörigkeit und Berechtigungen ein, bevor du die Anwendung im Browser verwendest:

```zelyra
# Tenant-scoped notification inbox starter for MariaDB projects.
# Every generated notification route verifies the authenticated membership.

auth users {
    table: users
    sessions: auth_sessions
    permissions: user_permissions
    memberships: tenant_memberships
}

table users {
    id: Id primary auto
    email: Email required unique
    password_hash: String(255) required
    active: Bool default true
}

table auth_sessions {
    id: Id primary auto
    user: User required
    token_hash: String(64) required unique
    expires_at: Timestamp required
}

table user_permissions {
    id: Id primary auto
    user_id: Id required
    permission: String(100) required
}

table tenant_memberships {
    id: Id primary auto
    user_id: Id required
    tenant_id: Id required
    active: Bool required
}

table notifications {
    id: Id primary auto
    tenant_id: Id required
    title: String(160) required
    message: String(4000) required
    category: String(40) required
    read_at: Timestamp?
    archived_at: Timestamp?
    expires_at: Timestamp?
    created_at: Timestamp default now
}

crud Notification -> notifications {
    title: "Notifications"
    tenant: tenant_id
    unread: read_at
    expires: expires_at
    requires auth
    permits "notifications.view"
    permits create "notifications.manage"
    permits edit "notifications.manage"
    permits delete "notifications.manage"

    soft_delete {
        column: archived_at
    }

    list {
        title
        category
        created_at
        read_at
    }

    search {
        title
        message
    }

    filter {
        category
        read_at
    }

    view {
        fields {
            title
            message
            category
            read_at
            expires_at
        }
    }
}

table notification_preferences {
    id: Id primary auto
    tenant_id: Id required
    email_enabled: Bool default false
    retention_days: Int default 90
    updated_at: Timestamp default now
}

crud NotificationPreference -> notification_preferences {
    title: "Notification preferences"
    tenant: tenant_id
    requires auth
    permits "notifications.manage"
    list {
        email_enabled
        retention_days
        updated_at
    }
    view {
        fields {
            email_enabled
            retention_days
        }
    }
}

fn main() {
}
```

`tenant: tenant_id` bindet die Ressource an den aktiven Mandanten und seine Mitgliedschaft. `unread: read_at` stellt den ungelesenen Zähler bereit; `expires: expires_at` blendet abgelaufene Einträge aus. `soft_delete` archiviert über `archived_at`. `notifications.view` erlaubt Lesen, `notifications.manage` schützt Verwaltung und Änderungen. Ein Tenant-Feld allein ohne die Auth-/Mitgliedschaftskonfiguration ist keine Mandantenisolierung.

### Lesestatus, Aufbewahrung und Mail

`read_at` ist ein Zeitstempel am Benachrichtigungseintrag. Über die Empfängerbindung (`recipient: user_id`) wird sichergestellt, dass Benachrichtigungen nur für den berechtigten Benutzer sichtbar sind. Persönliche Lesebestätigungen werden mit CSRF-Schutz und atomarem Audit-Logging quittiert. Die Einstellungstabelle speichert `email_enabled` und `retention_days`. Ablauf wird beim Lesen berücksichtigt; Archivierung ist keine physische Löschung. Das [Beispiel](https://github.com/sf1976/zelyra/blob/main/examples/notification_center.zyl) zeigt den vorhandenen Umfang.

### Lokales TOTP-MFA

Für mehrstufige Authentifizierung unterstützt Zelyra zeitbasierte Einmalkennwörter (TOTP nach RFC 6238). Die geheimen Schlüssel werden mit AES-256-GCM verschlüsselt in der Datenbank abgelegt. Einmalige Wiederherstellungscodes werden mit Argon2id sicher gehasht. Das System wehrt Token-Wiederholungsangriffe (Replay Defense) sowie Rennbedingungen ab und sperrt Konten nach fünf aufeinanderfolgenden Fehlversuchen automatisch temporär (HTTP 429).

## 27. Sprache, Laufzeit und PDF-Mail

Dieses Kapitel dokumentiert die Spracherweiterungen aus Zelyra 1.0.7. Die Version erweitert
Sprache und Runtime für gut prüfbare Businesslogik und ergänzt eine lokale,
serverseitige Text-PDF-Pipeline für Views und transaktionale E-Mail-Anhänge.

### Wertliefernde Ausdrücke und Record-Updates

Ein `if`-Ausdruck liefert in beiden Zweigen denselben typgeprüften Wert. Records
lassen sich mit `..old` funktional kopieren und selektiv aktualisieren:

```zelyra
fn invoice_label(paid: Bool) -> String {
    return if paid { "Bezahlt" } else { "Offen" }
}

struct Customer {
    id: Int
    name: String
    active: Bool
    version: Int
}

fn activate_customer(c: Customer) -> Customer {
    return Customer { ..c, active: true, version: c.version + 1 }
}
```

### Typisierte Lambdas und Pipelines

Lambdas sind lexikalische Closures. Der Compiler prüft Callback-Eingaben und
Ergebnisse statisch; Pipelines bleiben synchron und deterministisch:

```zelyra
fn calculate_tax_totals(amounts: Int[]) -> Int[] {
    return amounts
        |> filter(|a| a > 0)
        |> map(|a| a + (a * 19 / 100))
}
```

### Verschachtelte Record-Muster, Match-Guards und Tupel

`match` unterstützt verschachtelte Record-Muster und boolesche Wächter (Guards).
Tupel können als reguläre Rückgabewerte dienen und mehrfach destrukturiert werden:

```zelyra
struct Order { status: String total: Int }

fn inspect_order(order: Order) -> Tuple<String, Bool> {
    return match order {
        Order { status: "paid", total } if total >= 1000 => ("VIP-Grossauftrag", true)
        Order { status: "paid", total }                  => ("Standardauftrag", true)
        _                                                => ("Ausstehend", false)
    }
}
```

### Begrenzte Ausführung mit Step-Budget

`zelyra run datei.zyl --max-steps N` begrenzt Anweisungen und
Ausdrucksauswertungen über alle `parallel`-Zweige hinweg und meldet
Überschreitungen als `E-RUNTIME-001` an der Quellstelle:

```bash
zelyra run main.zyl --max-steps 5000
```

### Serverseitiges Text-PDF und transaktionale Outbox-E-Mail

`render_pdf("InvoicePrint", invoice)` rendert eine deklarierte View lokal in
`Bytes`. Innerhalb derselben Datenbanktransaktion kann `mail_queue` einen
PDF-Anhang in der MariaDB-Outbox einreihen:

```zelyra
fn send_invoice(recipient: String, invoice: Invoice) uses Email, Database(write) {
    transaction {
        mail_queue(
            recipient,
            "Rechnung " + invoice.number,
            "Ihre Rechnung ist beigefügt.",
            "invoice-" + invoice.number + ".pdf",
            render_pdf("InvoicePrint", invoice)
        )
    }
}
```

## 28. Systemd-Dienste, Scheduler und Jobs

Zelyra integriert Betriebssystemdienste, permanente Hintergrund-Zeitpläne und asynchrone Jobs direkt in die Toolchain, ohne externe Cron-Dämonen oder unzuverlässige Skriptschleifen.

### Linux-systemd-Dienstverwaltung

Mit `zelyra service` wird für ein Projekt ein dedizierter Linux-systemd-Benutzerdienst eingerichtet. Der Service verwendet einen fest konfigurierten Port:

```bash
# Dienst auf festem Port 8080 installieren
zelyra service install --port 8080

# Status des Dienstes und HTTP-Erreichbarkeit prüfen
zelyra service status

# Dienst neu starten (wartet auf lokale Socket-Erreichbarkeit)
zelyra service restart

# Port unterbrechungsfrei und mit atomarem Rollback ändern
zelyra service change --port 8081

# Dienst stoppen und Unit-Datei entfernen
zelyra service remove
```

Ist der angegebene Port belegt, schlägt die Installation fehl (`fail-closed`). Zelyra weicht niemals stillschweigend auf einen anderen Port aus. Bei `service change` wird die alte Unit erst dann überschrieben und der neue Dienst gestartet, wenn der Zielport frei ist; bei Fehlern wird der vorherige Zustand automatisch wiederhergestellt.

### Residente Zeitpläne und MariaDB-Outbox-Jobs

Zeitpläne werden deklarativ im Zelyra-Quellcode definiert:

```zelyra
schedule weekly_report {
    cron: "0 8 * * 1"
    timezone: "Europe/Berlin"
    command: generate_reports
}
```

Der residente Scheduler läuft ausschließlich innerhalb des aktiven Zelyra-Projektdienstes. An jeder Minutengrenze prüft er anstehende Ausführungen und reiht geplante Jobs transaktional in die MariaDB-Outbox ein. Ein serieller Worker übernimmt anstehende Aufgaben mit erneuerbaren Leases und begrenzten Wiederholungen.

```bash
# Alle anstehenden und ausgeführten Jobs des Projekts einsehen
zelyra schedule jobs main.zyl

# Status des residenten Schedulers abfragen
zelyra schedule status main.zyl

# Zeitpläne als crontab-kompatible Vorschau exportieren (z. B. für externe Systeme)
zelyra schedule export main.zyl --format cron
```

## 29. Kanonische Projektstruktur und Vertical Slices

Ab Version 1.2.13 unterstützt Zelyra ein verbindliches, feature-orientiertes Standard-Projektlayout (**Vertical Slice Architecture**), das in Version 1.3.0 zum Standard avanciert. Es trennt Routing strikt von Geschäfts- und Datenmodell.

### Das Standardlayout anlegen

```bash
zelyra new mein-erp --layout standard
```

Ein Projekt mit Standard-Layout besitzt folgende verbindliche Struktur:

```text
mein-erp/
├── zelyra.toml                 # Projekt-Manifest (layout = "standard")
├── schema/                     # Single Source of Schema
│   └── tables.zyl              # Tabellen, Indizes, Relationen
├── domain/                     # Fachdomänen (Vertical Feature Slices)
│   ├── invoices/               # Feature-Modul Rechnungen
│   │   ├── actions.zyl         # Geschäftsvorfälle & Rechenregeln
│   │   └── views.zyl           # UI-Komponenten & Ansichten
│   └── customers/              # Feature-Modul Kunden
│       ├── actions.zyl
│       └── views.zyl
├── routes/                     # Schlanke HTTP-Gatekeeper
│   ├── web.zyl                 # Web-Routen & Formular-Handling
│   └── api.zyl                 # REST-/JSON-Endpunkte
├── public/                     # Statische Dateien (vom Server direkt ausgeliefert)
└── assets/                     # Web-Assets (fließen in das Binary ein)
    ├── css/
    └── js/
```

### Namespace-Importe und strikte Architekturprüfung

Im Standardlayout löst Zelyra Importe direkt über die Namensraumwurzeln auf:

```zelyra
use schema::tables::*
use domain::invoices::actions as invoice_ops
```

Wird in der `zelyra.toml` der Schalter `strict_architecture = true` aktiviert, erzwingt der Zelyra-Linter folgende Kardinalregel:

> **Keine direkten Datenbankmutationen in Routen-Dateien (`routes/*.zyl`).**

Schreibende Operationen (`insert`, `update`, `delete`) innerhalb von Routen führen zum Abbruch mit dem Compilerfehler `E-ARCH-001`. Fachlogik muss in `domain/<feature>/actions.zyl` implementiert und getestet werden.

```bash
# Migrationsvorschau auf das Standardlayout prüfen (schreibgeschützt, verändert keine Dateien)
zelyra migrate-layout --check main.zyl
```

## 30. Erweiterbare CRUD- und TableView-Seiten

Zelyra 1.2.15 führt typgeprüfte Erweiterungspunkte für automatisch generierte CRUD-Seiten und Tabellenansichten ein. Entwickler können Standard-Masken flexibel anpassen, ohne das sichere Fundament aus Validierung, Berechtigungen und CSRF-Schutz aufgeben zu müssen.

### Create- und Edit-Seiten anpassen

Über `crud.view.create` und `crud.view.edit` lassen sich Seitentitel, Schaltflächentexte und ergänzende Inhaltsblöcke vor oder nach dem Formular einbinden:

```zelyra
crud KundenVerwaltung for Customer {
    view create {
        title: "Neuen Kunden registrieren"
        submit_label: "Kundenkonto anlegen"
        help: "Bitte prüfen Sie die USt-IdNr. vor dem Speichern."
    }
    view edit {
        title: "Kundendaten aktualisieren"
        submit_label: "Änderungen übernehmen"
    }
}
```

Zusätzliche Inhalte verwenden denselben sicheren Template-Kontext und unterliegen voller XSS-Escaping- und Typprüfung. Scheitert eine Formularvalidierung, bleiben die ergänzten Inhalte und Hinweistexte zusammen mit den Fehlermeldungen erhalten.

### Erweiterte TableView-Listen und Zeilenaktionen

Tabellenansichten (`tableview`) können mit Kopf- und Fußbereichen sowie individuellen Zeilenaktionen versehen werden:

```zelyra
tableview RechnungsListe for Invoice {
    columns: [number, customer_name, total_gross, status]
    row_actions: [
        action "Details" -> "/invoices/show/{id}",
        action "PDF" -> "/invoices/{id}/pdf"
    ]
}
```

Die Zeilenaktionen erzeugen saubere, auditierbare URLs. Sicherheitsprüfungen und Mandanten-Filter bleiben vollständig wirksam.

## 31. Dual-Tier-Ausführungsmodell: AST, MIR und Bytecode-VM

Auf dem Weg zu Version 1.3.0 implementiert Zelyra eine zukunftsweisende Zweipfad-Architektur, die sofortiges Feedback in der Entwicklung mit maximaler Geschwindigkeit in Produktionsumgebungen vereint.

```text
Quellcode (.zyl)
      │
      ▼
Lexer & Parser
      │
      ▼
AST mit Source-Spans
      │
Gemeinsame statische Prüfungen (Types, Contracts, Scopes)
      │
      ├───────────────────────────────┐
      ▼                               ▼
[1. AST-Referenzinterpreter]    [2. High-Speed Pipeline]
- Schnelles Hot-Reloading       - HIR (Symbolauflösung)
- Unbestechliches Test-Orakel   - Typisierte MIR (CFG, Basisblöcke)
- Entwickler-Diagnostik         - MIR-Verifier
                                - Bytecode-Compiler (Format v24)
                                - Bytecode-VM (Iterative Rust-Callframes)
```

1. **AST-Referenz:** Dient während der Entwicklung als unverzüglich einsatzbereites Orakel und im Test-Harness als Prüfmaßstab.
2. **Typisierte MIR:** Bildet den Kontrollflussgraph (CFG) in expliziten Basisblöcken ab und verifiziert SQL-Operationen (`E-MIR-VERIFY-004`), Typen und Capabilities.
3. **Bytecode-VM v24:** Führt kompilierte Instruktionen in iterativen Schleifen auf nativer Rust-Ebene aus – ohne Rekursionsgrenzen und mit minimalem Arbeitsspeicherbedarf (< 45 MB RAM im Leerlauf).

Differentialtests stellen sicher, dass AST und Bytecode-VM für dieselben Programme identische Ergebnisse, Fehlercodes und Nebeneffekte liefern.

## 32. Mandantensicherheit und zusammengesetzte Fremdschlüssel

In mandantenfähigen B2B-Anwendungen darf kein Datensatz ohne Tenant-Zugehörigkeit existieren, und Fremdschlüssel dürfen niemals mandantenübergreifend referenzieren. Zelyra verankert Mandantensicherheit direkt im Tabellenschema (`scoped by tenant_id: TenantId`).

### Deklaration von Mandantentabellen

```zelyra
type TenantId = Id

table customers scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    customer_number: String(50) required unique within tenant
    name: String required
}

table invoices scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    invoice_number: String(50) required unique within tenant
    customer: Customer?
    total_net: Decimal(12, 2) required default 0
    total_gross: Decimal(12, 2) required default 0
}
```

### Automatische Integritätsgarantien auf Datenbankebene

1. **Zusammengesetzter Primärschlüssel-Schutz:** Zelyra erzeugt automatisch `uq_{table}_tenant_id UNIQUE (tenant_id, id)`.
2. **Mandantenisolierte Fremdschlüssel:** Verknüpfungen zwischen mandantenisolierten Tabellen (`customer: Customer?`) generieren automatisch zusammengesetzte Fremdschlüssel:
   ```sql
   FOREIGN KEY (tenant_id, customer_id) REFERENCES customers (tenant_id, id) ON DELETE RESTRICT
   ```
   Dadurch ist ein Cross-Tenant-Angriff (z. B. Zuweisen eines Kunden von Mandant A zu einer Rechnung von Mandant B) auf physischer Datenbankebene unmöglich.
3. **Mandantenbezogene Eindeutigkeit:** Mit `unique within tenant` deklarierte Felder erzeugen `UNIQUE KEY (tenant_id, customer_number)`. Unterschiedliche Mandanten können dieselbe Belegnummer führen, innerhalb eines Mandanten sind Duplikate ausgeschlossen.
4. **Optionale Relationen und kaufmännische Dezimalzahlen:** Optionale Verknüpfungen (`Customer?`) erzeugen typsicher `BIGINT NULL`. `Decimal(p, s)` (z. B. `Decimal(12, 2)`) wird ohne Gleitkomma-Ungenauigkeit gespeichert. Formulare weisen Werte ab, die die deklarierte Präzision oder Skala überschreiten; berechnete Felder werden vor dem Speichern exakt berechnet und auf ihre Zielskala gerundet.

## 33. Tabelleninvarianten und Validierungsregeln

Geschäftsregeln gehören direkt an die Datenstruktur. Statt Validierungsregeln in Controllern, Requests und Formularen redundant zu wiederholen, deklariert Zelyra typgeprüfte Invarianten im `table`-Block.

### Deklaration von Tabelleninvarianten

```zelyra
table invoices scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    invoice_date: Date required
    due_date: Date required
    discount_days: Int required default 0

    validate {
        due_date >= invoice_date: "Das Fälligkeitsdatum muss am oder nach dem Rechnungsdatum liegen."
        discount_days >= 0: "Skontofrist darf nicht negativ sein."
    }
}
```

### Durchsetzung in Formularen und APIs

- **Deklaratives CRUD und Formulare:** Vor jedem Schreibvorgang (Create/Edit) evaluiert Zelyra die Invarianten atomar. Werden Invarianten verletzt, wird der Schreibvorgang verhindert und das Formular liefert die deklarierte Fehlermeldung mit HTTP-Status 422.
- **Typisierte API-Eingabeprüfung:** APIs können Request-Bodies an Tabellen binden (`validates`). Die Validierung liefert standardisierte HTTP-422-Fehlerverträge, bevor der Handler ausgeführt wird.

Tabellen mit berechneten Spalten dürfen diese Spalten nicht als API-Eingaben
deklarieren. Zelyra berechnet sie serverseitig aus den typisierten Eingaben neu,
prüft danach die Tabelleninvarianten und übergibt die Ergebnisse als zusätzliche
Handlerparameter in Tabellenspaltenreihenfolge. Das gilt für geprüfte API-POST-
und -PUT-Routen. Ein Update muss alle Berechnungsabhängigkeiten und Ergebnisse
an die gleichnamigen typisierten Parameter binden. Manipulierte Werte aus dem
Request werden nicht verwendet. `examples/computed_api.zyl` enthält ein
compilergeprüftes Create-/Update-Beispiel. Für diesen Berechnungspfad ist API-
PATCH noch nicht unterstützt.

## 34. Lifecycle-Guards, Policies und Master-Detail-CRUD

Revisionssicherheit (z. B. nach GoBD) verlangt, dass festgeschriebene Belege nicht mehr nachträglich verändert oder gelöscht werden dürfen. Gleichzeitig müssen komplexe Belege wie Rechnungen oder Aufträge Kopf- und Positionsdaten atomar erfassen.

### Datensatzabhängige Lifecycle-Guards

```zelyra
table invoices scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    invoice_date: Date required
    is_locked: Bool required default false
}

crud Invoice -> invoices {
    title: "Rechnungen"

    guard edit when is_locked == true {
        error: "Festgeschriebene Rechnungen können nicht mehr bearbeitet werden (GoBD-Schutz)."
    }

    guard delete when is_locked == true {
        error: "Festgeschriebene Rechnungen dürfen nicht gelöscht werden."
    }
}
```

- **Guard-Semantik als Abweisungsbedingung:** Ein Guard weist eine Aktion ab, wenn seine Bedingung **wahr** (`true`) ist. Für den Schutz vor unbefugter Bearbeitung oder Festschreibung prüft man daher `guard edit when is_locked == true`. Eine Bedingung wie `is_locked == false` würde fälschlicherweise noch offene Entwürfe abweisen.
- **Transaktionale Prüfung:** Die Sperrbedingung wird nach Erwerb der Zeilensperre innerhalb der Schreibtransaktion evaluiert. Direkte POST-Requests können den Schutz nicht umgehen.
- **UI-Integration:** In `tableview`-Aktionsslots und Detailansichten werden Bearbeiten- und Löschen-Aktionen abhängig vom Datensatzstatus automatisch deaktiviert oder ausgeblendet.

### Zusammenspiel mit GoBD, Prüfsummen und Capabilities

Reale Geschäftsanwendungen erfordern neben Lifecycle-Sperren weitere Bausteine, die Zelyra über die Standardbibliothek und Capabilities bereitstellt:

1. **Kryptografische Prüfsummen (`hash_sha256`):**
   Die Funktion `hash_sha256(text: String) -> String` aus der Standardbibliothek berechnet den SHA-256-Hash eines Strings. Sie dient dazu, den Inhalt eines Belegs bei der Festschreibung kryptografisch zu fixieren:
   ```zelyra
   fn beleg_hash(beleg_nr: String, brutto: Decimal) -> String {
       return hash_sha256(beleg_nr + "#" + brutto)
   }
   ```
2. **Revisionssichere Zeitstempel (`now()` und `uses Clock`):**
   Funktionen, die die aktuelle Zeit ermitteln, müssen die Capability `Clock` deklarieren (`fn festschreiben(...) uses Clock`). Dadurch bleibt Zeitverhalten in Unittests deterministisch überprüfbar und reproduzierbar.
3. **Beleg-PDFs und Outbox-Mail (`render_pdf` und `mail_queue`):**
   Über `render_pdf("RechnungsDruck", invoice)` wird eine View serverseitig in PDF-Bytes umgewandelt. Innerhalb einer MariaDB-`transaction` reiht `mail_queue(to, subject, text, filename, pdf_bytes)` den Beleg atomar in die Outbox ein.
4. **Typisierte CRUD-Actions (seit 1.2.28):**
   `execute { ... }` akzeptiert compilergeprüfte Ausdrücke und SQL. Ein Guard wird vor Schreibvorgängen gegen den gespeicherten und gesperrten Datensatz ausgewertet. Der Web-Handler verwendet dieselbe Decision-Transaktion für typisiertes SQL und konfigurierte Audit-Einträge. MariaDB- und PostgreSQL-HTTP-E2Es prüfen `hash_sha256`, `now()`, Guards, Datensatzänderung, Audit-Eintrag, konkurrierende Anfragen und Rollback nach einem späteren Runtime-Fehler. Das MariaDB-E2E prüft außerdem das Einreihen eines echten PDF in die Outbox; es startet keinen Mailversand.

### Transaktionale Master-Detail-CRUD-Formulare (`has_many`)

Mit `has_many` deklarieren CRUD-Ressourcen direkte 1:N-Kinddatensammlungen:

```zelyra
table invoice_items scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    invoice: Invoice required on_delete cascade
    position: Int required
    description: String(160) required
    total_net: Decimal(12, 2) required
    total_gross: Decimal(12, 2) required

    validate {
        total_gross >= total_net: "Bruttobetrag darf nicht kleiner als der Nettobetrag sein."
    }
}

crud Invoice -> invoices {
    title: "Rechnungen"

    view {
        fields { invoice_date }
        form {
            title: "Rechnung"
            submit: "Rechnung speichern"
        }
    }

    has_many items: InvoiceItem {
        foreign_key: invoice_id
        order_by: position asc
        cascade: delete
        aggregates {
            net_amount: sum(total_net)
            tax_amount: sum(total_gross - total_net)
            gross_amount: sum(total_gross)
        }
    }
}
```

1. **Serverseitiges Rendering:** Kindzeilen werden serverseitig in HTML generiert. Zeilen können ohne Client-JavaScript hinzugefügt oder entfernt werden.
2. **Atomare Transaktion:** Kopfdaten, Kindpositionen und berechnete Aggregate (`aggregates`) werden in einer einzigen Datenbanktransaktion gespeichert. Ein Fehler in einer Position rollt den gesamten Vorgang zurück.
3. **Automatische Aggregation:** Summenfelder wie `net_amount` oder `gross_amount` werden serverseitig aus den Kindzeilen berechnet und atomar im Kopfsatz aktualisiert.

### Mehrere Kind-Sammlungen pro CRUD-Ressource (ab Zelyra 1.2.24)

Seit Zelyra 1.2.24 können generierte CRUD-Anlegen- und Bearbeiten-Formulare **mehrere direkte `has_many`-Sammlungen** desselben Elternsatzes verwalten (z. B. Positionen und Dateianhänge einer Rechnung in einem gemeinsamen Formular):

```zelyra
table invoices {
    id: Id primary auto
    reference: String(100) required
    invoice_date: Date required
    net_amount: Decimal(12, 2) required default 0
    gross_amount: Decimal(12, 2) required default 0
}

table invoice_items {
    id: Id primary auto
    invoice: Invoice required on_delete cascade
    position: Int required
    description: String(160) required
    total_net: Decimal(12, 2) required
    total_gross: Decimal(12, 2) required

    validate {
        total_gross >= total_net: "Line gross amount cannot be below its net amount."
    }
}

table invoice_attachments {
    id: Id primary auto
    invoice: Invoice required on_delete cascade
    file_name: String(255) required
    size_bytes: Int required

    validate {
        size_bytes > 0: "Attachment size must be positive."
    }
}

crud Invoice -> invoices {
    title: "Rechnungen mit Positionen und Anhängen"

    view {
        fields { reference invoice_date }
        form {
            title: "Rechnung"
            submit: "Rechnung speichern"
        }
    }

    has_many items: InvoiceItem {
        foreign_key: invoice_id
        order_by: position asc
        cascade: delete
        aggregates {
            net_amount: sum(total_net)
            gross_amount: sum(total_gross)
        }
    }

    has_many attachments: InvoiceAttachment {
        foreign_key: invoice_id
        order_by: file_name asc
        cascade: delete
    }
}
```

- **Unabhängige Abschnitte und Aktionen:** Jede Sammlung erscheint in einem eigenen Formularabschnitt mit eigener Zeilenanzahl und unabhängigen Zeilenaktionsnamen (`add_item`, `remove_item`, `add_attachment`, etc.).
- **Unabhängiges Zeilenlimit:** Das serverseitige Limit von 50 Zeilen gilt pro deklarierter Sammlung separat.
- **Atomare Validierung und Speicherung:** Zelyra validiert alle Sammlungen vor dem Schreiben. Elterndaten, alle Kindzeilen beider Sammlungen und deklarierte Aggregate werden in einer einzigen Datenbanktransaktion gespeichert. Ein Fehler in einer beliebigen Sammlung rollt den gesamten Vorgang atomar zurück.
- **Eindeutigkeit von Kindtabellen:** Jede Kindtabelle darf nur einmal an eine CRUD-Ressource gebunden werden; mehrere Sammlungen dürfen dasselbe Aggregatziel nicht mehrdeutig beschreiben.


## 35. Das Zelyra-Ökosystem: Pakete, Module und Einstellungen

Das Zelyra-Modul- und Paketökosystem ermöglicht die saubere Modularisierung von Anwendungen in wiederverwendbare, unabhängig testbare und kryptografisch prüfbare Komponenten. Es verbindet Offline-First-Prinzipien mit strikter Typisierung und Compiler-Garantien.

### Grundlagen und Philosophie des Zelyra-Ökosystems

1. **Offline-First und Deterministisch:** Ein Zelyra-Projekt baut und prüft vollständig offline. Es gibt keine impliziten Downloads beim Kompilieren (`zelyra check` / `zelyra run`), keine unkontrollierten Netzwerkzugriffe und keine Ausführung beliebiger Build-Skripte von Drittanbietern.
2. **Integrität und Lockfile:** Jede Abhängigkeit wird mit einer kryptografischen SHA-256-Prüfsumme in `zelyra.lock` fixiert. Änderungen an Quellcode oder Paketmanifest werden vom Compiler sofort erkannt und erfordern eine explizite Aktualisierung (`zelyra package lock --update`).
3. **Kryptografisches Vertrauensmodell (Ed25519):** Für die Weitergabe und Registrierung nutzt Zelyra signierte `.zpkg`-Quellarchive. Ein Archiv wird mit dem privaten Ed25519-Schlüssel eines Herausgebers signiert. Empfänger hinterlegen autorisierte öffentliche Schlüssel in `zelyra.trust.toml`. Ohne explizites Vertrauen verweigert Zelyra die Installation.
4. **Compiler-Effektobergrenzen (`effects`):** Ein Paket kann im Manifest deklarieren, welche Capabilities seine Funktionen maximal beanspruchen dürfen (z. B. `effects = ["Database(read)"]` oder `effects = []` für rein rechnerische Bibliotheken). Überschreitet eine Funktion im Paket diese Obergrenze, meldet der Compiler einen Fehler.

### Ein neues Modul erstellen: zelyra module init

Ein neues Modulpaket wird mit dem CLI-Befehl `zelyra module init` initialisiert:

```sh
zelyra module init packages/finanz_tools --name finanz_tools --version 0.1.0
```

Der Befehl erstellt die Standardstruktur ohne Netzwerkverbindung und ohne bestehende Dateien zu überschreiben:

```
packages/finanz_tools/
├── zelyra.toml
├── README.md
└── src/
    └── lib.zyl
```

### Das Paketmanifest: zelyra.toml

Das Manifest `zelyra.toml` deklariert die Identität, Versionskompatibilität und Effektobergrenzen des Pakets:

```toml
[package]
name = "finanz_tools"
version = "0.1.0"
entry = "src/lib.zyl"
zelyra = ">=1.1.0, <2.0.0"
effects = ["Database(read)"]

[settings.api_timeout]
type = "Int"
required = false
default = 15
description = "Zeitlimit für externe Abfragen in Sekunden"

[settings.api_token]
type = "String"
required = true
secret = true
description = "Geheimer API-Schlüssel für Finanzdaten"
```

- `name`: Paketname in Kleinbuchstaben mit Ziffern, `_` und `-`.
- `version`: Exakte SemVer-Version (z. B. `0.1.0`).
- `entry`: Einstiegsdatei, standardmäßig `src/lib.zyl` für Bibliotheken oder `main.zyl` für eigenständige Anwendungen.
- `zelyra`: SemVer-Vorgabe für die Zelyra-CLI. Liegt die laufende Compiler-Version außerhalb dieses Bereichs, verweigert Zelyra die Übersetzung.
- `effects`: Optionale Liste erlaubter Capability-Namen. `[]` erzwingt seiteneffektfreie reine Funktionen. Fehlt der Eintrag, gilt keine zusätzliche Paketobergrenze.

### Modulcode schreiben und exportieren

Funktionen, Records und Enums, die von außen erreichbar sein sollen, müssen mit dem Schlüsselwort `pub` exportiert werden:

```zelyra
pub record TaxResult {
    net: Decimal,
    tax: Decimal,
    gross: Decimal,
}

pub fn calculate_tax(net: Decimal, rate: Decimal) -> TaxResult
    requires { net >= 0 && rate >= 0 }
    ensures { result.gross >= result.net }
{
    let tax = (net * rate) / 100
    return TaxResult {
        net: net,
        tax: tax,
        gross: net + tax,
    }
}
```

Nicht als `pub` deklarierte Deklarationen bleiben paketintern geschützt. Verträge (`requires`, `ensures`) werden beim Import vollständig vom Compiler geprüft.

### Typisierte Moduleinstellungen (Settings)

Zelyra trennt Paket-Konfigurationsschemata strikt von Anwendungsdaten und Geheimnissen:

1. **Schemadeklaration:** Das Schema wird im Paketmanifest unter `[settings.<feld>]` beschrieben. Unterstützt werden `String` (bis 4 KiB), `Bool`, `Int` und `Float`. Geheimnisse deklarieren `secret = true` und dürfen keinen Standardwert besitzen.
2. **Anwendungs-Konfiguration in `zelyra.settings.toml`:** Werte liegen im Workspace der Hauptanwendung, getrennt von `zelyra.toml`, damit sich Paketprüfsummen nicht verändern:
   ```toml
   [package_settings."packages/finanz_tools"]
   api_timeout = 30
   api_token = { env = "FINANZ_API_TOKEN" }
   ```
   Geheimnisse werden ausschließlich über `{ env = "ENV_VAR" }` referenziert. Die Zelyra-Laufzeit liest den Wert aus der Prozessumgebung und verhindert die Protokollierung in Fehlermeldungen oder Logs.
3. **Abruf im Zelyra-Code:** Paketfunktionen lesen konfigurierte Werte typisiert über die interne generische Funktion `module_setting<T>`:
   ```zelyra
   fn get_timeout_seconds() -> Int {
       let timeout = module_setting<Int>("api_timeout")
       return timeout
   }
   ```
4. **Verwaltung über Web und Studio:**
   - **CLI-Prüfung:** `zelyra module settings check main.zyl` prüft Typen, Pflichtfelder und die Verfügbarkeit von Umgebungsvariablen.
   - **Zelyra Studio:** Unter *Moduleinstellungen* bietet Studio einen revisionsgesicherten Einstellungseditor mit `.zelyra.settings.lock`.
   - **Laufzeit-Webseite:** Ist in einer Anwendung Authentifizierung aktiv und `admin_permission` definiert, steht unter `/__zelyra/module-settings` eine CSRF-geschützte Webverwaltung für Administratoren bereit.

### Einbinden in eine Zelyra-Anwendung

Um ein Modul in einer Hauptanwendung zu verwenden, wird es im Anwendungsmanifest `zelyra.toml` unter `[dependencies.<alias>]` eingetragen:

```toml
[package]
name = "meine_anwendung"
version = "1.0.0"
entry = "main.zyl"
zelyra = ">=1.2.0"

[dependencies.finanz]
path = "packages/finanz_tools"
version = "^0.1"
```

Im Quellcode erfolgt der Import über den Namensraum-Präfix `pkg:<alias>`:

```zelyra
import "pkg:finanz" as finanz

fn main() {
    let result = finanz::calculate_tax(200.0, 19.0)
    print(result.gross)
}
```

Nach dem Hinzufügen der Abhängigkeit wird das Lockfile synchronisiert und geprüft:

```sh
zelyra package lock .
zelyra check main.zyl
```

### Modul-Inspektion, Planung und Bundling

Zelyra stellt spezialisierte Befehle zur Analyse und Distribution von Modulen bereit:

1. **Modul-Katalog anzeigen (`module list`):**
   ```sh
   zelyra module list main.zyl
   zelyra module list main.zyl --format json
   ```
   Listet alle Quellmodule, Importaliase, öffentlichen Exporte, Prüfsummen, Effektobergrenzen und Einstellungs-Schemas deterministisch auf.
2. **Abhängigkeitsgraph analysieren (`module plan`):**
   ```sh
   zelyra module plan main.zyl pkg:finanz
   ```
   Berechnet den statischen Quellabschluss für das angegebene Modul und meldet fehlende oder zirkuläre Referenzen vor dem Bauen.
3. **Modul bündeln (`module bundle`):**
   ```sh
   zelyra module bundle main.zyl pkg:finanz --output ./dist/finanz-bundle
   ```
   Erstellt ein eigenständiges Quellbundle mit isolierten Paketabhängigkeiten und neu berechnetem Lockfile, das offline und portabel deployt werden kann.

### Signierte Pakete, Registry und Transaktionale Wiederherstellung

Für teams- und organisationsübergreifende Verteilungen stellt Zelyra kryptografische Paketwerkzeuge bereit:

- **Schlüsselpaar und Signatur:**
  ```sh
  zelyra package keygen --publisher finanzteam --output finanzteam.key
  zelyra package sign packages/finanz_tools --publisher finanzteam --key finanzteam.key --output finanz_tools-0.1.0.zpkg
  ```
- **Herausgebervertrauen einrichten:**
  ```sh
  zelyra package trust add finanzteam <oeffentlicher-schluessel-hex>
  zelyra package verify finanz_tools-0.1.0.zpkg
  ```
- **Statische Registry aufbauen und durchsuchen:**
  ```sh
  zelyra package registry build ./veroeffentlichte-pakete
  zelyra package search https://pakete.example.test/index.json finanz
  ```
- **Verifizierte Installation:**
  ```sh
  zelyra package add https://pakete.example.test/index.json finanz_tools 0.1.0 --as finanz
  zelyra package update https://pakete.example.test/index.json finanz_tools 0.2.0 --as finanz
  zelyra package remove finanz
  ```
- **Transaktionale Wiederherstellung (`package recover`):**
  Alle mutierenden Paketbefehle schreiben ein geschütztes Transaktionsjournal. Bricht ein Installations- oder Update-Vorgang (z. B. durch Stromausfall oder Verbindungsabbruch) ab, blockieren Folgeoperationen, bis der Zustand geprüft ist:
  ```sh
  zelyra package recover
  zelyra package recover --apply
  ```
  `zelyra package recover --apply` stellt Manifest und `zelyra.lock` atomar auf den letzten fehlerfreien Stand zurück und bereinigt unvollständige Cache-Pfade.

## 36. CSS-Designsystem und Oberflächengestaltung für CRUD und TableViews

Zelyra verfolgt beim Frontend denselben Grundsatz wie bei Sprache und Datenbank: **Konstruierte Sicherheit ohne Ballast**. Statt schwergewichtige JavaScript-Build-Pipelines, Node.js-Abhängigkeiten oder npm-Pakete vorauszusetzen, liefert Zelyra ein integriertes, performantes CSS-Designsystem (`zelyra-ui.v1.css`), das direkt vom integrierten Webserver ausgeliefert wird.

Dieses Kapitel beschreibt die Architektur des Zelyra-UI-Systems, alle verfügbaren Design-Tokens und Klassen sowie konkrete, produktionsreife CSS-Rezepte zur individuellen Gestaltung von CRUD-Seiten, TableViews, Master-Detail-Formularen und Filtern.

---

### 1. Architektur des Zelyra-UI-Designsystems

Das Zelyra-Designsystem basiert auf modernen Webstandards und modernen CSS-Kaskadierungs-Ebenen (CSS Cascade Layers `@layer`).

```
┌────────────────────────────────────────────────────────────────────────┐
│                        CSS Cascade Layers                              │
│                                                                        │
│  @layer reset         Browser-Normalisierung & Box-Sizing              │
│       ↓                                                                │
│  @layer base          Root-Variablen, Typografie, Standardelemente     │
│       ↓                                                                │
│  @layer theme         Farbschemata (Light, Dark, System), Dichte       │
│       ↓                                                                │
│  @layer layout        App-Shell, Grids, Container, Scrollbereiche      │
│       ↓                                                                │
│  @layer components    Karten, Buttons, Badges, Tabellen, Formulare     │
│       ↓                                                                │
│  @layer utilities     Hilfsklassen (Abstände, Textausrichtung)         │
│       ↓                                                                │
│  @layer overrides     Zelyra-Standardseiten (CRUD, TableViews)         │
│       ↓                                                                │
│  [Projekt-CSS]        Eigene zelyra.theme.css (höchste Priorität)      │
└────────────────────────────────────────────────────────────────────────┘
```

#### Der Root-Selektor und Spezifität

Alle Standardstile von Zelyra UI sind mit dem CSS-Pseudoselektor `:where(html[data-z-ui])` gekapselt:

```css
:where(html[data-z-ui]) .z-table {
  width: 100%;
  border-collapse: collapse;
}
```

Die `:where()`-Pseudoklasse verleiht dem Selektor eine Spezifität von **0**. Dadurch können Entwickler beliebige Zelyra-Stile in ihrer eigenen `zelyra.theme.css` mit gewöhnlichen Klassenselektoren (z. B. `.z-table`) überschreiben, ohne zu `!important` oder unleserlichen Selektorketten greifen zu müssen.

#### Einbindung von Projekt-Stylesheets

Ein Zelyra-Projekt kann projektspezifisches CSS auf zwei Wegen einbinden:

1. **Automatische Erkennung:** Liegt im Wurzelverzeichnis des Projekts eine Datei namens `zelyra.theme.css` oder `assets/css/theme.css`, wird diese vom integrierten Server automatisch nach dem Zelyra-Basissystem in den `<head>` aller generierten Seiten injiziert.
2. **Umgebungsvariable / Konfiguration:** Über die Umgebungsvariable `ZELYRA_UI_STYLESHEET=/pfad/zu/style.css` oder den CLI-Parameter `--stylesheet` kann ein beliebiger alternativer Pfad deklariert werden.

Da ungelayertes CSS laut CSS-Spezifikation Vorrang vor allen `@layer`-Regeln hat, überschreiben Regeln in `zelyra.theme.css` automatisch die Framework-Vorgaben:

```css
/* In zelyra.theme.css: Überschreibt Standardstile sauber und konfliktfrei */
.z-table th {
  background: #1a2234;
  color: #ffffff;
}
```

Alternativ können eigene Regeln explizit im `@layer overrides` platziert werden:

```css
@layer overrides {
  .z-table th {
    background: #1a2234;
    color: #ffffff;
  }
}
```

---

### 2. Design-Tokens und CSS Custom Properties

Zelyra UI definiert ein durchgängiges System typisierter CSS-Variablen auf dem `:root`- bzw. `html[data-z-ui]`-Element.

#### Farbtokens (Color Tokens)

| Token | Standardwert (Light) | Zweck / Verwendung |
| :--- | :--- | :--- |
| `--z-color-canvas` | `#f4f6fa` | Hintergrundfarbe des gesamten Dokuments |
| `--z-color-surface` | `#ffffff` | Hintergrund von Karten, Formularen und Tabellen |
| `--z-color-surface-raised` | `#ffffff` | Leicht erhabene Bereiche (z. B. Tabellenköpfe, Dropdowns) |
| `--z-color-text` | `#182230` | Primäre Schriftfarbe für Fließtext und Titel |
| `--z-color-text-muted` | `#526174` | Abgeschwächte Schrift für Hinweise, Labels, Metadaten |
| `--z-color-border` | `#d8e0e9` | Standard-Rahmenfarbe für Tabellen, Inputs und Karten |
| `--z-color-primary` | `#3157a4` | Primärfarbe für Buttons, aktive Tabs, Fokusrahmen |
| `--z-color-on-primary` | `#ffffff` | Kontrast-Textfarbe auf Primärflächen |
| `--z-color-primary-hover`| `#274889` | Hover-Zustand für Primärschaltflächen |
| `--z-color-primary-soft` | `#eaf0fb` | Zarter Hintergrund für Zeilen-Hover und aktive Elemente |
| `--z-color-success` | `#16734c` | Erfolgsmeldungen, Freigabestatus, bezahlte Rechnungen |
| `--z-color-success-soft`| `#e9f6ef` | Weicher Hintergrund für Erfolgs-Badges und Notizen |
| `--z-color-warning` | `#795000` | Warnungen, Entwürfe, Mahnungen |
| `--z-color-warning-soft`| `#fff4d8` | Weicher Hintergrund für Warn-Badges |
| `--z-color-danger` | `#a12e39` | Fehler, Stornos, Löschaktionen, Invarianten-Fehler |
| `--z-color-danger-soft` | `#fff0f0` | Weicher Hintergrund für Fehlermeldungen |
| `--z-color-focus` | `#275fc5` | Farbe des barrierefreien Fokus-Rings (`:focus-visible`) |
| `--z-color-link` | `#244f9a` | Textlinks und Zeilenverweise |

#### Maße, Abstände und Radien

```css
:where(html[data-z-ui]) {
  /* Typografie */
  --z-font-sans: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  --z-font-mono: ui-monospace, "Cascadia Code", "Segoe UI Mono", monospace;
  --z-font-size: 1rem;
  --z-line-height: 1.5;

  /* Höhen und Inhaltsbreite */
  --z-control-height: 2.75rem; /* Standard-Eingabehöhe */
  --z-content-width: 76rem;    /* Maximale Inhaltsbreite */

  /* Abstände */
  --z-space-1: 0.25rem;  /* 4px */
  --z-space-2: 0.5rem;   /* 8px */
  --z-space-3: 0.75rem;  /* 12px */
  --z-space-4: 1rem;     /* 16px */
  --z-space-5: 1.25rem;  /* 20px */
  --z-space-6: 1.5rem;   /* 24px */
  --z-space-8: 2rem;     /* 32px */

  /* Radien */
  --z-radius-sm: 0.35rem;  /* Inputs, Buttons */
  --z-radius-md: 0.65rem;  /* Karten, Dialoge, Badges */
  --z-radius-lg: 1rem;     /* AppShell-Hauptcontainer */
  --z-radius-pill: 999px;  /* Runde Pill-Badges, Schalter */

  /* Z-Index Ebenen */
  --z-layer-sticky: 10;
  --z-layer-dropdown: 20;
  --z-layer-dialog: 100;
  --z-layer-toast: 120;
}
```

#### Kompakte Datendichte (`compact`)

In administrativen Dashboards, Buchhaltungsprogrammen oder ERP-Modulen wird häufig eine hohe Informationsdichte gewünscht. Zelyra UI unterstützt dies nativ über das HTML-Attribut `data-z-density="compact"`:

```html
<html data-z-ui data-z-density="compact">
```

Im Compact-Modus reduziert Zelyra UI die Eingabehöhe automatisch von `2.75rem` auf `2.25rem` und strafft die Innenabstände aller Tabellenzeilen und Formularfelder. Entwickler können die Dichte auch gezielt für einzelne Sektionen (z. B. nur für eine bestimmte Datentabelle) aktivieren:

```html
<div class="z-table-scroll" data-z-density="compact">
  <table class="z-table">...</table>
</div>
```

---

### 3. TableViews formatieren und optimieren

Tabellenansichten (`tableview`) gehören zu den zentralen Oberflächenelementen von Zelyra. Sie stellen relationale Daten dar, bieten Such-, Filter- und Sortierfunktionen und enthalten Zeilenaktionen.

#### Die Standard-HTML-Struktur

Ein Zelyra-TableView erzeugt typischerweise folgende HTML-Hierarchie:

```html
<div class="z-generated-page">
  <header class="z-page-header">
    <div class="z-page-header__copy">
      <h1 class="z-page-header__title">Rechnungen</h1>
      <p class="z-page-header__description">Alle erfassten Rechnungen des aktuellen Mandanten.</p>
    </div>
    <div class="z-page-header__actions">
      <a class="z-button z-button--primary" href="/invoices/new">+ Neue Rechnung</a>
    </div>
  </header>

  <form class="zelyra-query-controls" method="get">
    <fieldset>
      <legend>Filter & Suche</legend>
      <div class="z-query-controls__fields">
        <!-- Filterfelder -->
      </div>
    </fieldset>
  </form>

  <div class="z-table-scroll">
    <table class="z-table">
      <thead>
        <tr>
          <th>Nummer</th>
          <th>Kunde</th>
          <th>Datum</th>
          <th>Status</th>
          <th style="text-align: right;">Bruttobetrag</th>
          <th class="z-table__actions-head">Aktionen</th>
        </tr>
      </thead>
      <tbody>
        <tr>
          <td><strong>RE-2026-0042</strong></td>
          <td>Musterfirma GmbH</td>
          <td>19.10.2026</td>
          <td><span class="z-badge z-badge--success">Bezahlt</span></td>
          <td style="text-align: right;">1.428,00 €</td>
          <td class="z-table__actions">
            <a class="z-button z-button--sm" href="/invoices/42">Anzeigen</a>
            <a class="z-button z-button--sm" href="/invoices/42/edit">Bearbeiten</a>
          </td>
        </tr>
      </tbody>
    </table>
  </div>

  <nav class="z-pagination" aria-label="Seitennavigation">
    <div class="z-pagination__summary">Zeige 1 bis 25 von 148 Einträgen</div>
    <div class="z-pagination__controls">...</div>
  </nav>
</div>
```

#### Sticky Header (Fixierter Tabellenkopf)

Wenn Tabellen hunderte Einträge umfassen, scrollt der Tabellenkopf oft aus dem sichtbaren Bereich. Mit Sticky Headers bleibt die Kopfzeile dauerhaft sichtbar:

```css
/* In zelyra.theme.css: Fixierter Tabellenkopf */
.z-table-scroll {
  max-height: 70vh; /* Scrollbarer Bereich mit fester Maximalhöhe */
  overflow-y: auto;
  position: relative;
}

.z-table thead th {
  position: sticky;
  top: 0;
  z-index: var(--z-layer-sticky);
  background: var(--z-color-surface-raised);
  border-bottom: 2px solid var(--z-color-border);
  box-shadow: 0 2px 4px rgb(0 0 0 / 6%);
}
```

#### Zebra-Streifen und interaktive Zeilenhervorhebung

Zebra-Streifen verbessern das zeilenweise Erfassen komplexer Zahlenkolonnen dramatisch:

```css
/* Zebra-Streifen für gerade Zeilen */
.z-table tbody tr:nth-child(even) {
  background-color: rgb(0 0 0 / 2%);
}

[data-z-theme="dark"] .z-table tbody tr:nth-child(even) {
  background-color: rgb(255 255 255 / 2.5%);
}

/* Intensiverer Zeilen-Hover */
.z-table tbody tr:hover {
  background-color: var(--z-color-primary-soft);
  transition: background-color var(--z-motion-fast) ease-in-out;
}
```

#### Rechtsbündige Zahlen und Währungen (Tabular Nums)

In Tabellen dürfen Zahlen und Beträge niemals linksbündig oder zentriert gesetzt werden, da das menschliche Auge Dezimalstellen untereinander vergleichen muss. Zudem verhindert die Schriftart-Eigenschaft `tabular-nums`, dass Ziffern wie die schmale `1` das Schriftbild verschieben:

```css
/* Exakte numerische Ausrichtung für Betrags- und Mengenspalten */
.z-table td.col-number,
.z-table td.col-currency,
.z-table th.col-number,
.z-table th.col-currency {
  text-align: right;
  font-variant-numeric: tabular-nums;
  font-feature-settings: "tnum" 1;
}

/* Währungsbeträge dezent hervorheben */
.z-table td.col-currency {
  font-weight: 600;
  letter-spacing: -0.01em;
}
```

#### Formatierung der Aktionsspalte (`.z-table__actions`)

Die Zeilenaktionsleiste bündelt Bearbeiten-, Anzeigen- und Löschschaltflächen:

```css
/* Kompakte, sauber angeordnete Zeilenaktionsknöpfe */
.z-table td.z-table__actions {
  white-space: nowrap;
  text-align: right;
  vertical-align: middle;
}

.z-table td.z-table__actions .z-button,
.z-table td.z-table__actions a.z-button {
  min-height: 1.85rem;
  padding: 0.2rem 0.6rem;
  font-size: 0.82rem;
  font-weight: 500;
  border-radius: var(--z-radius-sm);
}

.z-table td.z-table__actions .z-button:hover {
  transform: translateY(-1px);
}
```

---

### 4. Such-, Filter- und Abfragewerkzeuge

Die Filterleiste über einer TableView wird in Zelyra über `.zelyra-query-controls` und `.z-query-controls__fields` gerendert.

```css
/* Modernes, aufgeräumtes Suchleisten-Design */
.zelyra-query-controls {
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-md);
  background: var(--z-color-surface-raised);
  padding: 1rem 1.25rem;
  margin-bottom: 1.5rem;
}

.zelyra-query-controls legend {
  font-size: 0.88rem;
  font-weight: 700;
  color: var(--z-color-text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding: 0 0.4rem;
}

.z-query-controls__fields {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(100%, 14rem), 1fr)) auto;
  align-items: end;
  gap: 1rem;
}

/* Eingabefelder in der Filterleiste */
.zelyra-query-controls .z-input,
.zelyra-query-controls .z-select {
  background: var(--z-color-surface);
  border-color: var(--z-color-border);
  box-shadow: inset 0 1px 2px rgb(0 0 0 / 4%);
}

.zelyra-query-controls .z-input:focus,
.zelyra-query-controls .z-select:focus {
  border-color: var(--z-color-primary);
  outline: 3px solid var(--z-color-primary-soft);
}
```

---

### 5. Paginierung und leere Tabellenzustände

#### Paginierung (`.z-pagination`)

```css
.z-pagination {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding-top: 1.25rem;
  border-top: 1px solid var(--z-color-border);
}

.z-pagination__summary {
  font-size: 0.88rem;
  color: var(--z-color-text-muted);
}

.z-pagination__controls {
  display: inline-flex;
  gap: 0.35rem;
}

.z-pagination__controls a,
.z-pagination__controls span {
  display: inline-flex;
  min-width: 2.25rem;
  height: 2.25rem;
  align-items: center;
  justify-content: center;
  padding: 0 0.5rem;
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-sm);
  background: var(--z-color-surface);
  color: var(--z-color-text);
  font-size: 0.88rem;
  text-decoration: none;
}

.z-pagination__controls a:hover {
  background: var(--z-color-primary-soft);
  border-color: var(--z-color-primary);
}

.z-pagination__controls [aria-current="page"] {
  background: var(--z-color-primary);
  color: var(--z-color-on-primary);
  border-color: var(--z-color-primary);
  font-weight: 700;
}
```

#### Leere Zustände (`.z-empty-state`)

Liefert eine Abfrage keine Ergebnisse, zeigt Zelyra den Zustand `.z-empty-state`:

```css
.z-empty-state {
  text-align: center;
  padding: 3.5rem 1.5rem;
  background: var(--z-color-surface);
  border: 1px dashed var(--z-color-border);
  border-radius: var(--z-radius-md);
  margin-block: 1rem;
}

.z-empty-state h3 {
  margin: 0 0 0.5rem;
  font-size: 1.2rem;
  color: var(--z-color-text);
}

.z-empty-state p {
  max-width: 30rem;
  margin: 0 auto 1.5rem;
  color: var(--z-color-text-muted);
  font-size: 0.95rem;
  line-height: 1.5;
}

.z-empty-state__actions {
  display: flex;
  justify-content: center;
  gap: 0.75rem;
}
```

---

### 6. CRUD- und Master-Detail-Formulare gestalten

Automatisch generierte Formulare (`crud.view.create`, `crud.view.edit`) werden im Container `.zelyra-crud-form-card` eingebettet.

#### Zwei-Spalten-Formularlayout mit responsivem Umbruch

```css
/* Card-Hülle des Formulars */
.zelyra-crud-form-card {
  max-width: 68rem;
  margin: 1.5rem auto 3rem;
  padding: 2rem;
  background: var(--z-color-surface);
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-lg);
  box-shadow: var(--z-shadow-sm);
}

/* Zweispaltiges Formular-Grid */
.zelyra-crud-form-card form {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 1.25rem 1.5rem;
}

/* Vollbreite Felder (z. B. Bemerkungen, Textareas) */
.zelyra-crud-form-card .z-field--full,
.zelyra-crud-form-card .z-fieldset,
.zelyra-crud-form-card .z-form-actions {
  grid-column: 1 / -1;
}

/* Auf mobilen Displays einspaltig umbrechen */
@media (max-width: 768px) {
  .zelyra-crud-form-card form {
    grid-template-columns: 1fr;
  }
}
```

#### Formularfelder, Labels und Hilfetexte

```css
.z-field {
  display: grid;
  gap: 0.4rem;
  min-width: 0;
}

.z-field__label {
  font-size: 0.92rem;
  font-weight: 650;
  color: var(--z-color-text);
}

/* Pflichtfeld-Sternchen hervorheben */
.z-field__label[data-required="true"]::after {
  content: " *";
  color: var(--z-color-danger);
}

.z-field__hint {
  font-size: 0.82rem;
  color: var(--z-color-text-muted);
}

.z-input,
.z-select,
.z-textarea {
  min-height: var(--z-control-height);
  padding: 0.55rem 0.8rem;
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-sm);
  background: var(--z-color-surface);
  color: var(--z-color-text);
  transition: border-color var(--z-motion-fast), box-shadow var(--z-motion-fast);
}

.z-input:focus,
.z-select:focus,
.z-textarea:focus {
  border-color: var(--z-color-focus);
  box-shadow: 0 0 0 3px var(--z-color-primary-soft);
  outline: none;
}
```

#### Validierungsfehler und Invarianten-Feedback (`[aria-invalid="true"]`)

Meldet die Zelyra-Validierung einen Fehler (z. B. eine verletzte Tabelleninvariante wie `total_gross >= total_net`), setzt Zelyra das Attribut `aria-invalid="true"`:

```css
/* Roter Signalfokus für fehlerhafte Felder */
.z-input[aria-invalid="true"],
.z-select[aria-invalid="true"],
.z-textarea[aria-invalid="true"] {
  border: 2px solid var(--z-color-danger);
  background-color: var(--z-color-danger-soft);
}

.z-field__error {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--z-color-danger);
}

/* Gesamtübersicht der Validierungsfehler oben am Formular */
.z-error-summary {
  margin-bottom: 1.5rem;
  padding: 1.25rem;
  border: 1px solid var(--z-color-danger);
  border-left-width: 6px;
  border-radius: var(--z-radius-md);
  background: var(--z-color-danger-soft);
  color: var(--z-color-danger);
}
```

#### Master-Detail-Kindtabellen (`has_many`) und Summenboxen

In Transaktionsformularen mit `has_many`-Relationen (wie Rechnungspositionen) wird die Kindtabelle nahtlos in das Formular integriert:

```css
/* Kinddatensatz-Tabelle im Formular */
.z-fieldset.z-master-detail-items {
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-md);
  padding: 1.25rem;
  background: var(--z-color-surface-raised);
}

.z-master-detail-items table {
  width: 100%;
  border-collapse: collapse;
}

.z-master-detail-items th,
.z-master-detail-items td {
  padding: 0.6rem 0.5rem;
  border-bottom: 1px solid var(--z-color-border);
}

/* Kompakte Inputs in der Positionstabelle */
.z-master-detail-items input.z-input {
  min-height: 2.15rem;
  padding: 0.25rem 0.5rem;
  font-size: 0.9rem;
}

/* Summenbox für automatische Aggregat-Werte (Netto, MwSt, Brutto) */
.z-form-summary-card {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  max-width: 22rem;
  margin-left: auto;
  margin-top: 1rem;
  padding: 1.25rem;
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-md);
  background: var(--z-color-surface);
}

.z-form-summary-card__row {
  display: flex;
  justify-content: space-between;
  font-size: 0.95rem;
}

.z-form-summary-card__row--total {
  border-top: 2px solid var(--z-color-border);
  padding-top: 0.65rem;
  margin-top: 0.25rem;
  font-size: 1.15rem;
  font-weight: 750;
  color: var(--z-color-primary);
}
```

---

### 7. Modaldialoge und Status-Badges

#### Modaldialoge (`.z-dialog`)

Zelyra verwendet standardkonforme HTML5-`<dialog>`-Elemente mit nativem Backdrop:

```css
.z-dialog {
  width: min(38rem, calc(100vw - 2rem));
  max-height: min(90vh, 48rem);
  padding: 0;
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-lg);
  background: var(--z-color-surface);
  color: var(--z-color-text);
  box-shadow: var(--z-shadow-md);
  overflow: hidden;
}

/* Dunkel abgedunkelter Hintergrund mit Unschärfe */
.z-dialog::backdrop {
  background: rgb(10 20 35 / 65%);
  backdrop-filter: blur(3px);
}

.z-dialog__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1.1rem 1.5rem;
  border-bottom: 1px solid var(--z-color-border);
  background: var(--z-color-surface-raised);
}

.z-dialog__body {
  padding: 1.5rem;
  overflow-y: auto;
}

.z-dialog__footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  padding: 1rem 1.5rem;
  border-top: 1px solid var(--z-color-border);
  background: var(--z-color-surface-raised);
}
```

#### Status-Badges (`.z-badge`)

```css
.z-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.25rem 0.65rem;
  font-size: 0.8rem;
  font-weight: 650;
  line-height: 1;
  border-radius: var(--z-radius-pill);
  border: 1px solid transparent;
  white-space: nowrap;
}

/* Farbvarianten für Beleg- und Buchungszustände */
.z-badge--success {
  background: var(--z-color-success-soft);
  color: var(--z-color-success);
  border-color: rgb(22 115 76 / 20%);
}

.z-badge--warning {
  background: var(--z-color-warning-soft);
  color: var(--z-color-warning);
  border-color: rgb(121 80 0 / 20%);
}

.z-badge--danger {
  background: var(--z-color-danger-soft);
  color: var(--z-color-danger);
  border-color: rgb(161 46 57 / 20%);
}

.z-badge--neutral {
  background: var(--z-color-canvas);
  color: var(--z-color-text-muted);
  border-color: var(--z-color-border);
}
```

---

### 8. Vollständige Praxisrezepte für zelyra.theme.css

Die folgenden Rezepte können direkt in die Datei `zelyra.theme.css` im Projektverzeichnis übernommen werden.

#### Rezept 1: Modernes Dark Mode & Accent Theme

Ein sattes, augenschonendes dunkles Theme mit Indigo- und Cyan-Akzenten für Entwicklungs- und Leitstandanwendungen:

```css
/* ==========================================================================
   Rezept 1: Modern Dark Theme für zelyra.theme.css
   ========================================================================== */
:where(html[data-z-ui][data-z-theme="dark"]),
:where(html[data-z-ui]:not([data-z-theme])) {
  /* Tiefes Schieferblau als Basisfarben */
  --z-color-canvas: #0b0f17;
  --z-color-surface: #121824;
  --z-color-surface-raised: #1a2233;
  --z-color-border: #28354b;

  /* Typografie */
  --z-color-text: #f1f5f9;
  --z-color-text-muted: #94a3b8;

  /* Strahlendes Cyan/Indigo als Primärakzent */
  --z-color-primary: #38bdf8;
  --z-color-on-primary: #082f49;
  --z-color-primary-hover: #7dd3fc;
  --z-color-primary-soft: #0c2b45;

  /* Statusfarben */
  --z-color-success: #4ade80;
  --z-color-success-soft: #0e3720;
  --z-color-warning: #facc15;
  --z-color-warning-soft: #3d330c;
  --z-color-danger: #f87171;
  --z-color-danger-soft: #3d1417;

  /* Schatten mit sanftem Schein */
  --z-shadow-sm: 0 1px 3px rgb(0 0 0 / 50%);
  --z-shadow-md: 0 12px 32px rgb(0 0 0 / 70%);
}

/* Akzentuierter Tabellenkopf im Dark Mode */
.z-table thead th {
  background: #172030;
  color: #38bdf8;
  border-bottom: 2px solid #28354b;
}

/* Sanfter Zeilen-Hover */
.z-table tbody tr:hover {
  background-color: #17243b;
}
```

#### Rezept 2: Kompakte Finanz- und Buchhaltungstabelle

Optimiert für Buchhaltungslisten, Kassenbücher und Belegübersichten mit fixiertem Tabellenkopf, Währungsausrichtung und schnellem visuellen Scannen:

```css
/* ==========================================================================
   Rezept 2: Finanz- und Buchhaltungstabelle mit Sticky Header
   ========================================================================== */
.z-table-scroll {
  max-height: 72vh;
  overflow: auto;
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-md);
  box-shadow: 0 4px 12px rgb(0 0 0 / 4%);
}

.z-table {
  font-size: 0.9rem;
  border-collapse: separate;
  border-spacing: 0;
}

/* Fixierter Tabellenkopf */
.z-table thead th {
  position: sticky;
  top: 0;
  z-index: 10;
  background: var(--z-color-surface-raised);
  border-bottom: 2px solid var(--z-color-primary);
  padding: 0.65rem 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  font-size: 0.78rem;
  letter-spacing: 0.04em;
  color: var(--z-color-text);
}

/* Schmale, platzsparende Zeilen */
.z-table td {
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--z-color-border);
  vertical-align: middle;
}

/* Subtile Streifung */
.z-table tbody tr:nth-child(even) td {
  background-color: rgb(0 0 0 / 1.5%);
}

/* Numerische Spalten rechtsbündig mit fester Ziffernbreite */
.z-table th:nth-child(5),
.z-table td:nth-child(5),
.z-table th:nth-child(6),
.z-table td:nth-child(6) {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.z-table td:nth-child(5) {
  font-weight: 700;
  color: var(--z-color-text);
}

/* Zeilenaktionen rechtsbündig und kompakt */
.z-table td:last-child {
  text-align: right;
  white-space: nowrap;
}

.z-table td:last-child .z-button {
  min-height: 1.85rem;
  padding: 0.15rem 0.55rem;
  font-size: 0.8rem;
}
```

#### Rezept 3: Master-Detail-Rechnungsformular mit Card-Stil

Dieses Rezept gestaltet Belegformulare mit geteilten Kopfdaten, strukturierter Positionstabelle und hervorgehobener Rechnungssummenkarte:

```css
/* ==========================================================================
   Rezept 3: Master-Detail Rechnungsformular mit Card-Design
   ========================================================================== */
.zelyra-crud-form-card {
  max-width: 72rem;
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-lg);
  padding: 2.25rem;
  background: var(--z-color-surface);
  box-shadow: 0 10px 30px rgb(0 0 0 / 5%);
}

/* Headerbereich des Formulars */
.zelyra-crud-form-card > header {
  margin-bottom: 2rem;
  padding-bottom: 1.25rem;
  border-bottom: 1px solid var(--z-color-border);
}

.zelyra-crud-form-card > header h1 {
  font-size: 1.6rem;
  margin: 0 0 0.4rem;
  color: var(--z-color-text);
}

/* Gruppierung der Stammdaten in einer erhabenen Karte */
.z-form-master-section {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 1.25rem;
  padding: 1.5rem;
  background: var(--z-color-surface-raised);
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-md);
  margin-bottom: 2rem;
}

@media (max-width: 900px) {
  .z-form-master-section {
    grid-template-columns: 1fr;
  }
}

/* Positionstabelle (Kind-Sammlung) */
.z-form-items-table {
  width: 100%;
  border-collapse: collapse;
  margin-block: 1rem 1.5rem;
}

.z-form-items-table th {
  padding: 0.65rem;
  border-bottom: 2px solid var(--z-color-border);
  font-weight: 700;
  font-size: 0.85rem;
  color: var(--z-color-text-muted);
}

.z-form-items-table td {
  padding: 0.5rem;
  border-bottom: 1px solid var(--z-color-border);
}

/* Hervorgehobene Summen-Box rechts unten */
.z-invoice-totals-box {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 0.6rem 2rem;
  width: min(100%, 24rem);
  margin-left: auto;
  padding: 1.25rem 1.5rem;
  background: var(--z-color-surface-raised);
  border: 1px solid var(--z-color-border);
  border-radius: var(--z-radius-md);
}

.z-invoice-totals-box dt {
  color: var(--z-color-text-muted);
  font-size: 0.95rem;
}

.z-invoice-totals-box dd {
  margin: 0;
  font-weight: 650;
  font-variant-numeric: tabular-nums;
  text-align: right;
}

.z-invoice-totals-box dt.total-gross,
.z-invoice-totals-box dd.total-gross {
  border-top: 2px solid var(--z-color-border);
  padding-top: 0.75rem;
  margin-top: 0.25rem;
  font-size: 1.25rem;
  font-weight: 800;
  color: var(--z-color-primary);
}

/* Formular-Aktionsleiste */
.z-form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 1rem;
  margin-top: 2.5rem;
  padding-top: 1.5rem;
  border-top: 1px solid var(--z-color-border);
}

.z-form-actions .z-button--primary {
  min-width: 11rem;
  font-size: 1rem;
}
```

## FileRef-Anhänge im generierten CRUD (1.2.32)

Eine undurchsichtige `FileRef`-Spalte wird mit ihrer Upload-Richtlinie als
generiertes CRUD-Dateifeld verwendet:

```zelyra
table contracts {
    id: Id primary auto
    title: String(200) required
    signed_pdf: FileRef? attachment {
        max_size: "20MB"
        allowed_types: ["application/pdf"]
    }
}

crud Contract -> contracts {}
```

Das vollständige, compilergeprüfte Beispiel liegt in
`examples/file_ref_crud.zyl`.

Der `attachment`-Block nutzt dieselbe geprüfte Größen- und MIME-Richtlinie wie
tabellengebundene `File`-Formularfelder. Generierte Anlegen-/Bearbeiten-Seiten
zeigen ein natives Datei-Eingabefeld. Wird beim Bearbeiten keine neue Datei
gesendet, bleibt die bestehende Referenz erhalten. Ein neuer Upload verwendet
den vorhandenen privaten, integritätsgeprüften Speicher und wird erst über die
normale CSRF-geschützte CRUD-Aktion veröffentlicht. Listen und Detailseiten
zeigen einen Download-Link statt der internen FileRef. Downloads laufen über
`/contracts/{id}/files/signed_pdf` und durchlaufen CRUD-Rechte, Datensatz-,
Mandanten- und Empfängerprüfung. Rohe FileRef-Werte werden nicht als normale
Textfelder akzeptiert. FileRef-Spalten können nicht für CRUD-Suche oder Filter
verwendet werden.

Unterstützte MIME-Typen sind derzeit PDF, PNG und JPEG. Die Größe ist durch die
vorhandene Upload-Richtlinie begrenzt (höchstens 100 MiB pro Feld sowie das
bestehende Request-Gesamtlimit). Projekte mit gewöhnlichen `FileRef`-Spalten
ohne generiertes CRUD können weiterhin explizite, geprüfte Upload-Formulare
verwenden.
