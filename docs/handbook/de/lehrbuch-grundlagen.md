# TEIL I – ZELYRA UND PROGRAMMIERUNG VERSTEHEN

---

## Kapitel 1: Willkommen bei Zelyra

### 1. Was lerne ich in diesem Kapitel?
In diesem Einführungskapitel erfährst du:
- Was eine Programmiersprache im Kern ist und welche Aufgabe sie erfüllt.
- Was Zelyra besonders macht und warum es als eigenständige Sprache entwickelt wurde.
- Welche praktischen Ziele Zelyra verfolgt und für welche Aufgaben es sich besonders eignet.
- Wie Zelyra Lesbarkeit, Zuverlässigkeit und Sicherheit von vornherein garantiert.
- Warum Zelyra sowohl für menschliche Entwickler als auch für KI-Systeme entworfen wurde.
- Wie dieses Lehrbuch aufgebaut ist und wie du am besten damit arbeitest.

### 2. Warum ist das Thema wichtig?
Bevor du die erste Zeile Code schreibst, solltest du verstehen, welches Problem Zelyra löst. In der modernen Softwareentwicklung – besonders bei daten- und webgestützten Anwendungen – herrscht oft ein riesiges Durcheinander: Man modelliert eine Tabelle in SQL, schreibt dieselben Regeln noch einmal in einem Backend-Framework (wie Laravel, Express oder Django), validiert dieselben Daten ein drittes Mal im Frontend (HTML/JavaScript) und generiert mühsam API-Beschreibungen.
Zelyra bricht mit dieser Fragmentierung: Du beschreibst dein Datenmodell, deine Regeln und deine Schnittstellen an einer einzigen Stelle – und Zelyra leitet daraus geprüfte, sichere Bausteine ab.

### 3. Verständliche Erklärung
Eine **Programmiersprache** ist wie ein präzises Regelwerk. Sie erlaubt es dir, einem Computer eindeutige Anweisungen zu geben. Computer sind extrem schnell, aber sie besitzen keinen gesunden Menschenverstand: Wenn eine Anweisung zweideutig ist oder ein unerwarteter Zustand eintritt, stürzt das Programm ab oder produziert fatale Fehler.

**Zelyra** ist eine moderne, statisch typisierte Sprache. „Statisch typisiert“ bedeutet einfach: Bereits vor dem Start des Programms prüft der Zelyra-Compiler gründlich, ob alle Bausteine zusammenpassen. Wenn eine Funktion Text erwartet, du ihr aber versehentlich eine Zahl übergibst, weist Zelyra dich sofort darauf hin – bevor der Code jemals einen Server oder Nutzer erreicht.

Gleichzeitig ist Zelyra **datenbank- und webzentriert**:
- Ein Tabellenschema (`table`) ist keine isolierte SQL-Datei, sondern integraler Bestandteil der Sprache.
- Variablen sind standardmäßig **unveränderlich** (`immutable`). Dadurch kann sich ein Wert nicht plötzlich im Hintergrund ändern.
- Keine Überraschungen durch `null`: Fehlende Werte müssen ausdrücklich als `Option` deklariert werden.

### 4. Kleine, aufeinander aufbauende Beispiele

Schauen wir uns ein erstes winziges Zelyra-Programm an:

```zelyra
// Unser erstes Zelyra-Programm: Eine Begrüßung
fn main() {
    print("Willkommen bei Zelyra!")
}
```

Wenn du diesem Programm mehr Struktur geben möchtest, zerlegst du die Aufgabe in eine Funktion:

```zelyra
fn begruessen(name: String) -> String {
    return "Hallo, " + name + "! Willkommen in der Zelyra-Welt."
}

fn main() {
    nachricht = begruessen("Entwickler")
    print(nachricht)
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Ein Semikolon am Zeilenende setzen (wie in Java, C++ oder PHP).
  *Ursache:* In Zelyra trennen Zeilenumbrüche Anweisungen sauber ab. Überflüssige Satzzeichen stören das Schriftbild.
- **Fehler:** Annehmen, Zelyra sei nur ein Framework oder eine Skriptsprache.
  *Ursache:* Zelyra ist eine eigenständige, kompilierte Sprache mit eigenem Typ- und Prüfsystem.

### 6. Merksätze
1. Zelyra verbindet Datenmodell, Logik und Oberfläche in einer einzigen klaren Sprache.
2. Was du meinst, steht im Code: Keine versteckte Magie, keine impliziten Null-Werte.
3. Der Compiler ist dein Partner: Er findet Fehler früh, bevor sie Schaden anrichten können.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Ändere das Begrüßungsprogramm so, dass es deinen eigenen Vornamen und Wohnort ausgibt.
- **Stufe 2 (Mittel):** Schreibe eine zweite Funktion `verabschieden(name: String) -> String`, die einen Abschiedsgruß formuliert, und rufe beide Funktionen in `main()` auf.
- **Stufe 3 (Anspruchsvoll):** Überlege dir drei typische Probleme, die in anderen Sprachen durch Tippfehler entstehen (z. B. eine Zahl statt Text übergeben), und begründe, wie ein Compiler davor schützt.

### 8. Praxisaufgabe: Der Grundstein der Aufgabenverwaltung
In diesem Buch bauen wir Schritt für Schritt eine echte, praxistaugliche **Aufgabenverwaltung** (Task Management). Wir beginnen mit dem einfachsten Schritt:
Erstelle eine Datei `aufgaben_start.zyl`, die den Namen unseres Systems und die Versionsnummer sauber auf dem Bildschirm ausgibt:

```zelyra
fn main() {
    system_name = "Zelyra TaskManager"
    version = "0.1.50"
    print(system_name + " (Version " + version + ") gestartet.")
}
```

### 9. Zusammenfassung
- Zelyra ist eine statisch typisierte, sichere und lesbare Sprache für Geschäftslogik, Datenbanken und das Web.
- Zelyra eliminiert Redundanzen zwischen Datenbank-Definitionen, Validierung und API.
- Variablen sind standardmäßig unveränderlich; der Compiler garantiert Stabilität und Klarheit.

### 10. Kontrollfragen zur Selbstprüfung
1. Was unterscheidet eine statisch typisierte Sprache von einer dynamischen Sprache?
2. Warum ist es ein Vorteil, wenn Tabellenschemata direkt in der Programmiersprache definiert werden?
3. Warum sind unveränderliche Werte standardmäßig sicherer als veränderliche Variablen?

---

## Kapitel 2: Wie ein Programm funktioniert

### 1. Was lerne ich in diesem Kapitel?
- Wie dein geschriebener Quelltext Schritt für Schritt in ein ausgeführtes Programm verwandelt wird.
- Welche Rollen Lexer, Parser, Type Checker, Verifier und Interpreter in Zelyra spielen.
- Was genau beim Start eines Zelyra-Programms geschieht.
- Was der Unterschied zwischen Syntax (Form) und Semantik (Bedeutung) ist.
- Was ein Algorithmus ist und wie das EVA-Prinzip (Eingabe, Verarbeitung, Ausgabe) funktioniert.

### 2. Warum ist das Thema wichtig?
Programmierfehler zu beheben ist kinderleicht, wenn man versteht, an welcher Station des Compilers der Fehler gemeldet wird. Ein Syntaxfehler bedeutet, dass der Text unlesbar ist; ein Typfehler bedeutet, dass die Logik widersprüchlich ist; ein Laufzeitfehler bedeutet, dass während der Ausführung eine unvorhergesehene Bedingung eintrat. Wenn du diese Kette verstehst, verlierst du jegliche Scheu vor Fehlermeldungen.

### 3. Verständliche Erklärung
Ein Computerprozessor versteht nur Nullen und Einsen (Maschinencode). Wenn wir Menschen eine Textdatei mit Zelyra-Code schreiben (z. B. `programm.zyl`), durchläuft dieser Text mehrere Stationen:

```text
Quelltext (.zyl)
   │
   ▼
[1. Lexer]: Zerlegt den Text in Wörter/Symbole (Tokens)
   │
   ▼
[2. Parser]: Baut einen logischen Strukturbaum (AST = Abstract Syntax Tree)
   │
   ▼
[3. Type Checker]: Prüft alle Typen, Namen und Berechtigungen
   │
   ▼
[4. Verifier]: Beweist mathematisch Schleifen und Verträge (Contracts)
   │
   ▼
[5. Runtime / Interpreter]: Führt die geprüften Anweisungen aus
```

- **Syntax** ist die Grammatik: Setzt du Klammern richtig? Schreibst du Schlüsselwörter korrekt?
- **Semantik** ist der Sinn: Wenn du schreibst `alter = "fünfundzwanzig"`, ist das syntaktisch Text, aber wenn du damit rechnen willst, ergibt es semantisch keinen Sinn.
- **Algorithmus**: Eine präzise, endliche Schritt-für-Schritt-Anleitung zur Lösung eines Problems.

### 4. Kleine, aufeinander aufbauende Beispiele

Ein einfacher Algorithmus zur Berechnung der verbleibenden Tage bis zu einer Frist:

```zelyra
fn tage_bis_ziel(ziel_tag: Int, aktueller_tag: Int) -> Int {
    verbleibend = ziel_tag - aktueller_tag
    return verbleibend
}

fn main() {
    heute = 10
    abgabe = 24
    tage = tage_bis_ziel(abgabe, heute)
    print(tage)
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Ein geschweifte Klammer `{` öffnen, aber nicht schließen `}`.
  *Ursache:* Dies ist ein reiner **Syntaxfehler**. Der Parser bricht sofort ab (`E-PARSE-001`), weil der Baum unvollständig ist.
- **Fehler:** Einer Zahl eine Zeichenkette zuweisen: `alter: Int = "20"`.
  *Ursache:* Das ist ein **Typfehler** (`E-TYPE-001`). Der Parser versteht die Form, aber der Type Checker stoppt die Ausführung.

### 6. Merksätze
1. Der Lexer liest Zeichen, der Parser versteht Strukturen, der Type Checker prüft den Sinn.
2. Je früher ein Fehler abgefangen wird (beim Prüfen statt beim Kunden), desto günstiger und sicherer ist die Software.
3. Jedes Programm folgt dem Grundmuster: Eingabe empfangen, nach klaren Regeln verarbeiten, Ergebnis ausgeben.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Zeichne auf einem Blatt Papier den Ablauf von `main()` im obigen Fristen-Beispiel als Pfeildiagramm auf.
- **Stufe 2 (Mittel):** Erweitere die Funktion `tage_bis_ziel` um eine Prüfung mit `if`: Wenn `aktueller_tag > ziel_tag` ist, soll `0` zurückgegeben werden.
- **Stufe 3 (Anspruchsvoll):** Erkläre in eigenen Worten, warum Zelyra vor der Ausführung prüft (`zelyra check`), anstatt den Code Zeile für Zeile blind auszuführen.

### 8. Praxisaufgabe: Aufgaben-Priorität berechnen
In unserer Aufgabenverwaltung müssen wir die Dringlichkeit einer Aufgabe einstufen. Wenn weniger als 3 Tage verbleiben, ist die Aufgabe dringend:

```zelyra
fn ist_dringend(verbleibende_tage: Int) -> Bool {
    return verbleibende_tage <= 3
}

fn main() {
    frist_in_tagen = 2
    dringend = ist_dringend(frist_in_tagen)
    if dringend {
        print("Achtung: Aufgabe hat hohe Prioritaet!")
    } else {
        print("Aufgabe liegt im normalen Zeitplan.")
    }
}
```

### 9. Zusammenfassung
- Programme durchlaufen eine feste Kette: Lexing, Parsing, Typprüfung, Verifikation und Ausführung.
- Zelyra stellt sicher, dass Syntax und Typen stimmen, bevor ein Programm gestartet wird.
- Ein Algorithmus verwandelt Eingaben durch logische Einzelschritte in verlässliche Ausgaben.

### 10. Kontrollfragen zur Selbstprüfung
1. An welcher Stelle der Toolchain wird bemerkt, dass ein Anführungszeichen fehlt?
2. Was bedeutet das EVA-Prinzip?
3. Warum bricht ein statisch typisiertes Programm ab, wenn man Text und Zahl fehlerhaft verbindet?

---

## Kapitel 3: Zelyra installieren und einrichten

### 1. Was lerne ich in diesem Kapitel?
- Die Systemvoraussetzungen für die Zelyra-Entwicklungsumgebung auf Linux, macOS und Windows.
- Plattformspezifische Docker-Installation und Verifikation mit `docker compose version`.
- Wie du Zelyra über den Quellcode (`./install.sh` / `install.ps1`) oder das stabile Release-Archiv (`--release v0.3.0`) installierst.
- Vollständige Versionsabfrage mit `zelyra --version` und Systemdiagnose mit `zelyra doctor`.
- Den integrierten, token-geschützten Web-Setup-Assistenten (`zelyra setup --web`).
- Typische Berechtigungs- und Port-Konflikte (z. B. Docker-Socket-Rechte, automatische Port-Wahl).

### 2. Warum ist das Thema wichtig?
Eine reibungslos funktionierende Werkzeugkette ist die Grundlage jeder erfolgreichen Entwicklungsarbeit. Wenn Befehle nicht gefunden werden oder Umgebungsvariablen fehlen, verliert man wertvolle Zeit. Zelyra bringt einen schlanken, benutzerlokalen Installer mit, der ohne fremde Paketmanager oder Root-Rechte auskommt. Zudem führt der neue Web-Setup-Assistent Einsteiger visuell durch die Initialisierung von Datenbank und Containern.

### 3. Verständliche Erklärung
Zelyra benötigt für einfache Programme weder Apache noch fremde Laufzeiten. Das Zelyra-CLI (`zelyra`) ist ein einzelnes, hochoptimiertes Binärprogramm.

Je nach Betriebssystem wählst du den passenden Weg:
1. **Linux / macOS:** Hier lädst du das offizielle Repository herunter und führst `./install.sh` aus (oder lädst ein Release-Archiv). Zelyra wird benutzerlokal in `~/.local/bin` installiert. Für Docker nutzt man unter Linux die [offizielle Docker-Engine-Anleitung](https://docs.docker.com/engine/install/) und unter macOS [Docker Desktop für Mac](https://docs.docker.com/desktop/setup/install/mac-install/).
2. **Windows:** Unter Windows stehen `install.ps1` für PowerShell sowie `install.cmd` zur Verfügung. Für vollständige MariaDB-Projekte wird [Docker Desktop für Windows](https://docs.docker.com/desktop/setup/install/windows-install/) mit aktivierter Compose-Unterstützung empfohlen.
3. **Plattform-Docker-Prüfung:** Zelyra installiert Docker bewusst nicht eigenmächtig und fordert keine Root-Rechte an. Vor dem Start von Compose-Diensten prüfst du deine Umgebung einfach mit `docker compose version`. Fehlt Compose oder fehlen Berechtigungen auf den Docker-Socket, gibt Zelyra gezielte Plattform-Hilfestellungen aus.

### 4. Kleine, aufeinander aufbauende Beispiele

**Schritt 1: Zelyra installieren**
Aus dem Quelltext (Linux / macOS):
```bash
git clone https://github.com/sf1976/zelyra.git
cd zelyra
./install.sh
```
Oder direkt als vorkompiliertes Release ohne Rust-Toolchain:
```bash
./install.sh --release v0.3.0
```
Unter Windows (PowerShell):
```powershell
git clone https://github.com/sf1976/zelyra.git
Set-Location zelyra
.\install.ps1 -Release v0.3.0
```

**Schritt 2: Vollständige Version und Hilfe prüfen**
```bash
zelyra --version
zelyra --help
```
`zelyra --version` gibt den vollständigen Compiler- und Paketversionsstand aus (z. B. `zelyra 0.3.0`). Die Sprachkompatibilitätslinie bleibt 0.1.

**Schritt 3: Docker Compose prüfen (für MariaDB-Projekte)**
```bash
docker compose version
```

**Schritt 4: Systemdiagnose ausführen**
```bash
zelyra doctor
```
Dieser Befehl analysiert Umgebung, Pfade, Ports und Werkzeuge. Mit `zelyra doctor --json` erhältst du strukturierte Maschinendaten für IDEs.

**Schritt 5: Geführter Web-Setup-Assistent (Optional)**
In jedem MariaDB-Projektverzeichnis kannst du den grafischen Assistenten starten:
```bash
zelyra setup --web
```
Zelyra öffnet einen lokalen HTTP-Server auf `127.0.0.1:3030` mit einem zufälligen, einmaligen Sicherheitstoken. Dort kannst du mit einem Klick die `.env`-Konfiguration erzeugen, den MariaDB-Container starten und das Datenbankschema anwenden.

### 5. Typische Fehler und deren Ursachen
- **Fehler:** `zelyra: command not found`
  *Ursache:* Der Ordner `~/.local/bin` ist noch nicht in deiner `$PATH`-Variable. Führe `export PATH="$HOME/.local/bin:$PATH"` aus oder starte dein Terminal neu.
- **Fehler:** `permission denied while trying to connect to the Docker daemon socket`
  *Ursache:* Auf Linux-Systemen hat dein Benutzer noch keine Rechte auf den Docker-Socket. Führe `sudo usermod -aG docker $USER` aus und melde dich neu an. Zelyra fängt diesen Fehler ab und gibt einen klaren Hinweis.
- **Fehler:** Standard-Port 3000 oder 3306 ist belegt.
  *Ursache:* Ein anderer lokaler Dienst belegt den Port. Zelyra wählt bei `zelyra setup` und `zelyra new` automatisch den nächsten freien Host-Port, sodass kein Konflikt entsteht.

### 6. Merksätze
1. Das Zelyra-CLI bündelt Compiler, Runner, Formularprüfer, Migrator, Webserver und Setup-Assistenten in einem einzigen Werkzeug.
2. Mit `docker compose version` und `zelyra doctor` überprüfst du jederzeit den Zustand deiner Toolchain.
3. Das stabile Release kann mit `--release v0.3.0` direkt ohne Rust-Compiler installiert werden.
4. `zelyra setup --web` bietet eine intuitive, browserbasierte Ersteinrichtung mit sicherem Einmal-Token.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Führe `zelyra doctor` in deinem Terminal aus und notiere dir die Versionsnummer.
- **Stufe 2 (Mittel):** Erkunde die Hilfeseite mit `zelyra check --help` und schau dir die Option `--format json` an.
- **Stufe 3 (Anspruchsvoll):** Richte in deinem bevorzugten Editor (z. B. VS Code) eine Dateizuordnung ein, sodass `.zyl`-Dateien automatisch als Zelyra-Dateien erkannt werden.

### 8. Praxisaufgabe: Die Arbeitsumgebung für den TaskManager vorbereiten
Lege auf deinem Rechner einen neuen Ordner für unser Projekt an und teste, ob die Zelyra-Toolchain dort ordnungsgemäß funktioniert:

```bash
mkdir mein-taskmanager
cd mein-taskmanager
echo 'fn main() { print("TaskManager-Umgebung bereit.") }' > test.zyl
zelyra check test.zyl
zelyra run test.zyl
```
Wenn die Ausgabe `TaskManager-Umgebung bereit.` erscheint, ist dein System perfekt vorbereitet!

### 9. Zusammenfassung
- Zelyra wird über ein einfaches Skript (`./install.sh`) oder Docker eingerichtet.
- Das Kommandozeilenwerkzeug `zelyra` enthält alle notwendigen Funktionen.
- `zelyra doctor` stellt sicher, dass alles einwandfrei konfiguriert ist.

### 10. Kontrollfragen zur Selbstprüfung
1. Welcher Befehl zeigt alle verfügbaren CLI-Optionen an?
2. Warum benötigt Zelyra für Konsolen- und Web-Programme keinen externen Webserver wie Apache?
3. Was prüft der Befehl `zelyra doctor`?

---

## Kapitel 4: Das erste Zelyra-Projekt

### 1. Was lerne ich in diesem Kapitel?
- Wie man ein Zelyra-Projekt mit `zelyra new` oder `zelyra init` anlegt (inkl. `--mariadb`).
- Wie ein Standard-Projektordner strukturiert ist (`main.zyl`, `zelyra.toml`, `.env`).
- Automatische Portvergabe (`ZELYRA_HOST_PORT` und `ZELYRA_DB_HOST_PORT`) bei belegten Ports.
- Wie `zelyra setup --all` und `zelyra setup --web` den Erststart automatisieren.
- Optionale Feature-Schalter (`[features]` in `zelyra.toml` oder `.env`) und Prüfung mit `zelyra config`.
- Wann ein Programm eine `main()`-Funktion benötigt und wie man Programme prüft, ausführt und formatiert.

### 2. Warum ist das Thema wichtig?
Sobald Programme mehr als zehn Zeilen umfassen, gehören sie in eine saubere Projektstruktur. Eine gut organisierte Ordnerstruktur stellt sicher, dass Konfigurationen, Datenbankmodelle, Web-Routen und Geschäftslogik ihren festen Platz haben. Wer von Beginn an Projekte standardisiert anlegt, spart sich später aufwändige Aufräumarbeiten.

### 3. Verständliche Erklärung
Mit dem Befehl `zelyra new <projektname>` erzeugst du ein schlüsselfertiges Projekt:

- **`main.zyl`**: Die Hauptdatei deines Programms. Hier definierst du entweder den Einstiegspunkt `fn main()` oder deklarierst deine Tabellen, Webseiten und APIs.
- **`zelyra.toml`**: Die dauerhafte Projektkonfiguration (Name, Version, Capabilities und optionale Feature-Schalter wie `web`, `api`, `crud`, `auth`, `audit`).
- **`.env`**: Lokale, nicht in Git eingecheckte Geheimnisse und Ports (`DATABASE_URL`, `ZELYRA_HOST_PORT`, `ZELYRA_DB_HOST_PORT`).
- **Docker & MariaDB**: Mit `--mariadb` legt Zelyra zusätzlich `Dockerfile`, `docker-compose.mariadb.yml` und `.env.example` an.

**Automatische Portvergabe:** Sind die Standardports 3000 (Web) oder 3306 (MariaDB) auf deinem Entwicklungsrechner bereits belegt, scannt Zelyra automatisch und vergibt freie Ports in der neu erzeugten `.env`.

**Optionale Feature-Schalter:** Du kannst in `zelyra.toml` oder `.env` Teilbereiche aktivieren oder deaktivieren:
```toml
[features]
web = true
api = true
crud = true
auth = true
audit = true
```
Wird ein deaktivierter Bereich im Code genutzt, meldet der Compiler verlässlich `E-FEATURE-001`. Die wirksame Konfiguration kannst du jederzeit mit `zelyra config main.zyl` (oder `--format=json`) geheimnisfrei inspizieren.

### 4. Kleine, aufeinander aufbauende Beispiele

**Projekt erstellen (Minimal oder MariaDB):**
```bash
# Minimales Skriptprojekt:
zelyra new taskmanager --template minimal
cd taskmanager

# Oder vollständiges MariaDB-Webprojekt:
zelyra new taskmanager-web --mariadb
cd taskmanager-web
```

> **🧪 Unveröffentlichter 0.4-Entwicklungsstand:** Neu erzeugte MariaDB-Projekte
> legen die Datenbankdeklaration in `src/database.zyl` ab und importieren dieses
> Modul ausdrücklich aus `main.zyl`. Das trennt die Zuständigkeit im Quellbaum,
> bietet aber noch keine mehreren benannten Verbindungen; die Laufzeit nutzt
> weiterhin eine projektweite `DATABASE_URL`. Die veröffentlichte Version 0.3.0
> bleibt davon unverändert.

**Ersteinrichtung mit einem einzigen Befehl:**
```bash
zelyra setup --all
```
Dieser Befehl legt eine geschützte `.env` an, startet die MariaDB-Container-Umgebung und wendet das Schema an.

**Projektkonfiguration prüfen:**
```bash
zelyra config main.zyl
```

**Projekt prüfen und starten:**
```bash
zelyra check main.zyl
zelyra run main.zyl
```

**Projekt automatisch formatieren:**
```bash
zelyra fmt main.zyl
```
`zelyra fmt` sorgt dafür, dass aller Zelyra-Code im gesamten Team exakt denselben, sauberen Gestaltungsregeln folgt.

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Dateien ohne die Dateiendung `.zyl` anlegen.
  *Ursache:* Der Compiler erwartet ausdrücklich Zelyra-Quelldateien mit der Endung `.zyl`.
- **Fehler:** Ein CLI-Programm ohne `fn main()` starten.
  *Ursache:* Wenn Zelyra per `zelyra run` gestartet wird, sucht es nach `fn main()`. Fehlt diese Funktion, bricht die Ausführung ab.

### 6. Merksätze
1. `zelyra new` erstellt eine saubere, standardisierte Projektstruktur.
2. In `zelyra.toml` werden Name, Version und benötigte Berechtigungen (Capabilities) verwaltet.
3. `zelyra fmt` garantiert einen einheitlichen, gut lesbaren Programmierstil.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Erstelle ein neues Projekt `mein_erstes_projekt` mit `zelyra new` und führe es aus.
- **Stufe 2 (Mittel):** Ändere in `zelyra.toml` die Version auf `0.3.1` und gib in `main()` die neue Versionsnummer aus.
- **Stufe 3 (Anspruchsvoll):** Verändere die Einrückungen in `main.zyl` absichtlich unordentlich und beobachte, wie `zelyra fmt main.zyl` den Quelltext wieder perfekt ausrichtet.

### 8. Praxisaufgabe: Die Aufgabenverwaltung als echtes Projekt initialisieren
Erstelle das Projekt, das uns durch das gesamte Buch begleiten wird:

```bash
zelyra new zelyra-tasks --template minimal
cd zelyra-tasks
```

Schreibe in `main.zyl` ein erstes Menü:
```zelyra
fn zeige_menue() {
    print("=================================")
    print("   ZELYRA AUFGABENVERWALTUNG     ")
    print("=================================")
    print("1: Alle Aufgaben anzeigen")
    print("2: Neue Aufgabe anlegen")
    print("3: Programm beenden")
}

fn main() {
    zeige_menue()
}
```
Prüfe das Projekt mit `zelyra check main.zyl` und führe es mit `zelyra run main.zyl` aus.

### 9. Zusammenfassung
- Zelyra-Projekte besitzen eine klare Struktur aus Quelltext (`.zyl`) und Konfiguration (`zelyra.toml`).
- `zelyra check` prüft die Gültigkeit, `zelyra run` führt das Programm aus.
- `zelyra fmt` formatiert den Code automatisch nach einheitlichen Standards.

### 10. Kontrollfragen zur Selbstprüfung
1. Wozu dient die Datei `zelyra.toml`?
2. Warum ist `zelyra fmt` in Teams so wertvoll?
3. Wann benötigt ein Zelyra-Programm eine `main()`-Funktion?

# TEIL II – DIE GRUNDLAGEN DER SPRACHE

---

## Kapitel 5: Werte und Datentypen

### 1. Was lerne ich in diesem Kapitel?
- Was Werte und Datentypen sind und warum sie das Rückgrat sicherer Programme bilden.
- Die grundlegenden Zahlentypen: `Int`, `UInt`, `Float` und `Decimal`.
- Text- und Zeichentypen: `String` und `Char`.
- Wahrheitswerte (`Bool`) und der leere Typ (`Unit`).
- Zeit- und Datumstypen: `Timestamp`, `Date`, `Time`, `Duration`.
- Der Unterschied zwischen automatischer Typableitung und ausdrücklicher Typangabe.

### 2. Warum ist das Thema wichtig?
Im echten Leben kann man Äpfel nicht mit Birnen addieren. Ein Computer würde ohne Typen jedoch genau das tun: Er würde versuchen, eine Postleitzahl mit einem Preis zu multiplizieren oder einen Buchstabensalat als Datum zu interpretieren. In Zelyra verhindert das Typsystem solche Absurditäten von vornherein. Ein Datentyp legt exakt fest, welche Werte erlaubt sind und welche Operationen darauf ausgeführt werden dürfen.

### 3. Verständliche Erklärung
Jeder Wert in Zelyra besitzt einen Typ. Du kannst den Typ entweder explizit hinschreiben oder Zelyra ihn automatisch aus dem zugewiesenen Wert ableiten lassen:

```zelyra
fn main() {
    // Explizite Typangabe: Name gefolgt von Doppelpunkt und Typ
    anzahl: Int = 10

    // Automatische Typableitung: Zelyra erkennt sofort, dass dies ein String ist
    titel = "Wichtige Besprechung"

    print(titel)
}
```

Die wichtigsten Datentypen in Zelyra:
- **`Int`**: Ganze Zahlen mit Vorzeichen (64-Bit), z. B. `-5`, `0`, `42`.
- **`UInt`**: Ganze Zahlen ohne Vorzeichen (nur `>= 0`), z. B. IDs oder Zähler.
- **`Float`**: Fließkommazahlen für wissenschaftliche Berechnungen, z. B. `3.1415`.
- **`Decimal`**: Festkommazahlen mit garantierter Exaktheit – unverzichtbar für Geldbeträge, um Rundungsfehler von Fließkommazahlen zu vermeiden!
- **`Bool`**: Wahrheitswerte. Es gibt exakt zwei Zustände: `true` (wahr) oder `false` (falsch).
- **`String`**: Zeichenketten (Text) in doppelten Anführungszeichen: `"Hallo Welt"`.
- **`Char`**: Einzelne Zeichen in einfachen Anführungszeichen: `'A'`, `'z'`, `'✓'`.
- **`Unit`**: Steht für „kein Wert“, ähnlich wie `void` in anderen Sprachen. Wenn eine Funktion nur etwas ausgibt und nichts zurückliefert, ist ihr Typ `Unit`.

### 4. Kleine, aufeinander aufbauende Beispiele

```zelyra
fn main() {
    aufgabe_id: Int = 101
    aufgabe_name: String = "Server aktualisieren"
    ist_erledigt: Bool = false
    geschaetzte_stunden: Float = 2.5
    stundensatz: Float = 85.50

    print(aufgabe_name)
    print(ist_erledigt)
}
```

Zelyra schützt vor unpassenden Typen:
Wenn du versuchst, `aufgabe_id = "einhundert"` zu schreiben, verweigert der Compiler sofort den Dienst mit `E-TYPE-001`.

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Geldbeträge mit `Float` berechnen.
  *Ursache:* Fließkommazahlen nach IEEE-754 können krumme Rundungen wie `0.1 + 0.2 = 0.30000000000000004` erzeugen. In Zelyra nutzt du für Finanzen immer `Decimal`.
- **Fehler:** Ein einzelnes Zeichen in doppelte Anführungszeichen setzen, wenn ein `Char` erwartet wird.
  *Ursache:* `"A"` ist ein `String`, während `'A'` ein `Char` ist.

### 6. Merksätze
1. Datentypen schützen davor, unpassende Informationen miteinander zu verknüpfen.
2. Für Geldbeträge gilt: Immer `Decimal`, niemals `Float`.
3. Zelyra kann Typen intelligent ableiten, aber explizite Typen dokumentieren deine Absicht.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Deklariere drei Variablen für deinen Lieblingsfilm: Titel (`String`), Erscheinungsjahr (`Int`) und ob du ihn im Kino gesehen hast (`Bool`).
- **Stufe 2 (Mittel):** Berechne die Gesamtkosten einer Aufgabe aus `geschaetzte_stunden` und `stundensatz` und gib das Ergebnis aus.
- **Stufe 3 (Anspruchsvoll):** Erkläre, warum ein ID-Feld oft besser als `Int` oder als nominaler Typ `type TaskId = Id` deklariert wird, anstatt als beliebiger Text.

### 8. Praxisaufgabe: Aufgaben-Attribute definieren
Erweitere unser Aufgabenprojekt in `main.zyl`. Definiere die typisierten Grunddaten einer Aufgabe:

```zelyra
fn main() {
    task_id: Int = 1
    task_name: String = "Datenbankschema pruefen"
    is_done: Bool = false
    priority: Int = 1 // 1 = hoch, 2 = mittel, 3 = niedrig

    print("Aufgabe #" + "1" + ": " + task_name)
    if is_done {
        print("Status: Erledigt")
    } else {
        print("Status: Offen (Prioritaet: hohe Dringlichkeit)")
    }
}
```

### 9. Zusammenfassung
- Zelyra stellt eine reichhaltige Palette primitiver Datentypen für Zahlen, Text und Logik bereit.
- Typen werden entweder explizit angegeben oder vom Compiler automatisch abgeleitet.
- Strenge Typprüfung verhindert logische Fehler zur Entwurfszeit.

### 10. Kontrollfragen zur Selbstprüfung
1. Warum sollte man für Geldwerte `Decimal` statt `Float` verwenden?
2. Was ist der Unterschied zwischen `"Z"` und `'Z'`?
3. Welche beiden Werte kann ein `Bool` annehmen?

---

## Kapitel 6: Variablen und Unveränderlichkeit

### 1. Was lerne ich in diesem Kapitel?
- Was eine Variable im Speicher eines Computers bedeutet.
- Warum Variablen in Zelyra standardmäßig unveränderlich (`immutable`) sind.
- Wie man veränderbare Variablen ausdrücklich mit `mutable` kennzeichnet.
- Gültigkeitsbereiche (`Scopes`) und Lebensdauer von Variablen.
- Warum Unveränderlichkeit Software dramatisch stabiler und fehlerfreier macht.

### 2. Warum ist das Thema wichtig?
Einer der häufigsten Gründe für schwer auffindbare Softwarefehler in Sprachen wie JavaScript, Python oder C++ ist unkontrollierte Veränderbarkeit: Eine Funktion verändert heimlich eine globale Variable, und an ganz anderer Stelle stürzt das Programm ab.
Zelyra folgt einem radikal klaren Prinzip: **Feste Werte sind der Normalfall.** Wenn sich ein Wert während der Programmlaufzeit ändern darf, musst du das ganz bewusst mit dem Schlüsselwort `mutable` ankündigen.

### 3. Verständliche Erklärung
Stell dir eine Variable wie eine beschriftete Schachtel im Arbeitsspeicher vor:

- **Unveränderliche Bindung (Standard):**
  ```zelyra
  fn main() {
      titel = "Steuererklaerung"
      print(titel)
  }
  ```
  Du legst den Text `"Steuererklärung"` in die Schachtel mit der Aufschrift `titel` und versiegelst sie. Niemand darf den Inhalt der Schachtel austauschen. Jeder, der die Schachtel liest, kann sich darauf verlassen, dass immer dasselbe darin liegt.

- **Veränderbare Variable (`mutable`):**
  ```zelyra
  fn main() {
      mutable zaehler = 0
      zaehler = zaehler + 1
      print(zaehler)
  }
  ```
  Hier bleibt die Schachtel offen. Du darfst den alten Wert herausnehmen und durch einen neuen ersetzen.

**Gültigkeitsbereich (Scope):**
Variablen leben immer nur innerhalb des Blocks `{ ... }`, in dem sie deklariert wurden. Wird der Block verlassen, vergisst Zelyra die Variable automatisch. Das schont den Speicher und verhindert Namenskonflikte.

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Unveränderlichkeit schützt vor versehentlichem Überschreiben**
```zelyra
fn main() {
    projekt = "Zelyra Kern"
    // projekt = "Neues Projekt" // FEHLER: Compiler blockiert Zuweisung an unveränderliche Variable!
    print(projekt)
}
```

**Beispiel 2: Wann `mutable` sinnvoll ist (z. B. Zähler und Schleifen)**
```zelyra
fn main() {
    mutable offene_aufgaben = 5
    print(offene_aufgaben)

    // Eine Aufgabe wurde erledigt:
    offene_aufgaben = offene_aufgaben - 1
    print(offene_aufgaben)
}
```

**Beispiel 3: Gültigkeitsbereiche (Scopes)**
```zelyra
fn main() {
    bereich = "Global im main"
    if true {
        lokal = "Nur im if sichtbar"
        print(lokal)
        print(bereich)
    }
    // print(lokal) // FEHLER: `lokal` existiert außerhalb des Blocks nicht mehr!
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Versuchen, einer Variablen ohne `mutable` einen neuen Wert zuzuweisen.
  *Ursache:* Zelyra meldet `cannot assign to immutable variable`. Wenn sich ein Wert ändern können muss, schreibe `mutable name = ...`.
- **Fehler:** Jede Variable aus Bequemlichkeit als `mutable` deklarieren.
  *Ursache:* Schlechter Stil. Verwende `mutable` nur dort, wo ein Wert sich tatsächlich im Ablauf ändern muss (z. B. in Schleifen oder Zwischenakkumulatoren).

### 6. Merksätze
1. In Zelyra sind Werte standardmäßig unveränderlich.
2. Wenn sich ein Wert ändern darf, steht `mutable` sichtbar davor.
3. Variablen existieren nur innerhalb ihres deklarierten Blocks `{ ... }`.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Erstelle eine unveränderliche Variable für deinen Benutzernamen und gib sie aus.
- **Stufe 2 (Mittel):** Erstelle einen `mutable punktestand = 100`, ziehe 15 Punkte ab, füge 30 Punkte hinzu und gib die Zwischenstände aus.
- **Stufe 3 (Anspruchsvoll):** Begründe, warum unveränderliche Variablen besonders bei Programmen helfen, die mehrere Aufgaben gleichzeitig (nebenläufig) ausführen.

### 8. Praxisaufgabe: Zähler für unsere Aufgabenverwaltung
In unserer Aufgabenverwaltung wollen wir zählen, wie viele Aufgaben noch zu erledigen sind:

```zelyra
fn main() {
    mutable anzahl_offen = 3
    print("Start: Aufgaben zu erledigen: ")
    print(anzahl_offen)

    // Erste Aufgabe erledigt:
    anzahl_offen = anzahl_offen - 1
    print("Zwischenstand: Noch offen:")
    print(anzahl_offen)

    // Zweite Aufgabe erledigt:
    anzahl_offen = anzahl_offen - 1
    print("Endstand: Noch offen:")
    print(anzahl_offen)
}
```

### 9. Zusammenfassung
- Unveränderlichkeit ist Zelyras Standard und verhindert unerwünschte Nebeneffekte.
- Veränderliche Variablen werden explizit mit `mutable` deklariert.
- Geschweifte Klammern begrenzen die Sichtbarkeit und Lebensdauer von Variablen.

### 10. Kontrollfragen zur Selbstprüfung
1. Was passiert, wenn du einer Variablen ohne `mutable` einen neuen Wert zuweist?
2. Warum ist Unveränderlichkeit ein Sicherheitsmerkmal?
3. Kann eine Variable aus einem inneren Block außerhalb dieses Blocks gelesen werden?

---

## Kapitel 7: Operatoren und Ausdrücke

### 1. Was lerne ich in diesem Kapitel?
- Was ein Operator und was ein Ausdruck (`Expression`) ist.
- Rechenoperatoren für Zahlen: `+`, `-`, `*`, `/`, `%`.
- Vergleichsoperatoren: `==`, `!=`, `<`, `<=`, `>`, `>=`.
- Logische Operatoren: `&&` (UND), `||` (ODER), `!` (NICHT).
- Operatorrangfolge und verständliche Klammersetzung.

### 2. Warum ist das Thema wichtig?
Programme bestehen nicht nur aus festen Werten, sondern berechnen Ergebnisse, treffen Vergleiche und kombinieren logische Aussagen. Ein Operator ist das Bindeglied, das aus einzelnen Werten neue Erkenntnisse formt. Wer Operatoren beherrscht, kann komplexe Geschäftsregeln präzise in einfache Ausdrücke übersetzen.

### 3. Verständliche Erklärung
- Ein **Ausdruck** ist jedes Stück Quelltext, das zu einem Wert ausgewertet werden kann. Zum Beispiel ist `5 + 3` ein Ausdruck, der zum Wert `8` wird.
- Ein **Operator** ist das Symbol, das die Operation beschreibt (z. B. `+` oder `==`).

**Rechenoperatoren:**
- `+`: Addition (auch für String- und Array-Verkettung)
- `-`: Subtraktion (oder Negation: `-x`)
- `*`: Multiplikation
- `/`: Division
- `%`: Modulo (Rest einer ganzzahligen Division, z. B. `7 % 3 == 1`)

**Vergleichsoperatoren (liefern immer `Bool`):**
- `==`: Ist gleich?
- `!=`: Ist ungleich?
- `<` / `<=`: Kleiner / Kleiner oder gleich?
- `>` / `>=`: Größer / Größer oder gleich?

**Logische Operatoren:**
- `&&` (UND): Nur wahr, wenn **beide** Seiten wahr sind (`true && true == true`).
- `||` (ODER): Wahr, wenn **mindestens eine** Seite wahr ist.
- `!` (NICHT): Kehrt einen Wahrheitswert um (`!true == false`).

### 4. Kleine, aufeinander aufbauende Beispiele

```zelyra
fn main() {
    // Rechnen
    grundzeit = 60
    puffer = 15
    gesamtzeit = grundzeit + puffer
    print(gesamtzeit)

    // Vergleichen
    ist_lang = gesamtzeit > 60
    print(ist_lang)

    // Logische Verknüpfung
    hat_puffer = puffer > 0
    ist_kritisch = gesamtzeit > 120 && !hat_puffer
    print(ist_kritisch)
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Ein einfaches Gleichheitszeichen `=` im Vergleich verwenden: `if x = 5`.
  *Ursache:* `=` ist die Zuweisung! Für Vergleiche verlangt Zelyra strikt das doppelte `==`.
- **Fehler:** Division durch Null (`x / 0`).
  *Ursache:* Führt zur Laufzeit zu einem Abbruch. Vor einer Division muss der Teiler geprüft werden.

### 6. Merksätze
1. Zuweisen mit `=`, Vergleichen mit `==`.
2. Rechnungen werten Punkt vor Strich aus; setze im Zweifel Klammern für maximale Klarheit.
3. `&&` verlangt beidseitige Wahrheit, `||` gibt sich mit einer zufriedenen Bedingung zufrieden.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Prüfe mit `==`, ob `10 % 2` gleich `0` ist (Gerade-Zahl-Prüfung).
- **Stufe 2 (Mittel):** Schreibe einen Ausdruck, der prüft, ob eine Zahl `alter` zwischen `18` und `65` (inklusive) liegt.
- **Stufe 3 (Anspruchsvoll):** Schreibe einen Ausdruck für eine Rabattregel: Ein Kunde erhält Rabatt, wenn er VIP ist (`is_vip == true`) ODER wenn sein Bestellwert über 100 liegt UND er kein Neukunde ist.

### 8. Praxisaufgabe: Fristen- und Status-Prüfung im TaskManager
In unserem TaskManager müssen wir prüfen, ob eine Aufgabe überfällig ist und sofortige Aufmerksamkeit verlangt:

```zelyra
fn main() {
    tage_verbleibend = -2
    ist_erledigt = false
    ist_blockiert = false

    // Eine Aufgabe ist ueberfaellig, wenn Tage < 0 und sie noch nicht erledigt ist
    ist_ueberfaellig = tage_verbleibend < 0 && !ist_erledigt

    // Hohe Dringlichkeit: Ueberfaellig und nicht durch andere blockiert
    braucht_eingriff = ist_ueberfaellig && !ist_blockiert

    print("Aufgabe ueberfaellig?")
    print(ist_ueberfaellig)
    print("Braucht sofortigen Eingriff?")
    print(braucht_eingriff)
}
```

### 9. Zusammenfassung
- Operatoren verknüpfen Werte zu aussagekräftigen Ausdrücken.
- Vergleichsoperatoren erzeugen `Bool`-Werte.
- Logische Operatoren (`&&`, `||`, `!`) erlauben komplexe Entscheidungsregeln.

### 10. Kontrollfragen zur Selbstprüfung
1. Was ist der Unterschied zwischen `=` und `==`?
2. Welches Ergebnis liefert der Ausdruck `5 > 3 && 2 > 10`?
3. Wofür steht der Operator `%`?

---

<a id="kapitel-8-ein--und-ausgaben"></a>

## Kapitel 8: Ein- und Ausgaben

### 1. Was lerne ich in diesem Kapitel?
- Wie man Informationen mit `print()` zuverlässig auf der Konsole ausgibt.
- Wie `read_console()` eine Zeile interaktiv vom Terminal einliest.
- Wie Text und Variablen durch Verkettung formatiert werden.
- Wie Zelyra Eingaben über Parameter, Dateien, Umgebungsvariablen, Terminal und Web-Routen empfängt.
- Warum Zelyra für sensible Außeninteraktionen ausdrückliche Berechtigungen (`Capabilities`) verlangt.

### 2. Warum ist das Thema wichtig?
Ein Programm, das weder Daten empfangen noch Ergebnisse mitteilen kann, ist für den Anwender nutzlos. Ein- und Ausgaben (I/O) verbinden die Logik deines Codes mit der Außenwelt. Weil Zugriffe auf Tastatur, Festplatte oder Netzwerk aber auch Sicherheitsrisiken darstellen, regelt Zelyra diese Zugriffe viel kontrollierter als ältere Sprachen.

### 3. Verständliche Erklärung
- **Ausgabe:** Der eingebaute Befehl `print(wert)` nimmt Zahlen, Wahrheitswerte, Zeichenketten oder zusammengesetzte Objekte entgegen und gibt sie auf dem Standard-Ausgabekanal (`stdout`) aus.
- **Formatierung:** Mehrere Texte und Werte verbindest du mit dem Plus-Operator `+`.
- **Eingabe in Zelyra:**
  Programme empfangen Eingaben über Parameter, Umgebungsvariablen, Dateien, Web-Anfragen oder interaktiv im Terminal:
  1. **Funktionsparameter:** Daten werden beim Aufruf übergeben.
  2. **Umgebungsvariablen:** `env("MEIN_KEY")` liest Konfigurationswerte aus dem System.
  3. **Dateien:** `read_text("eingabe.txt")` liest gespeicherte Daten ein.
  4. **Web-Anfragen:** Formulare (`form`) und URLs (`page "/user/{id}"`) empfangen Benutzereingaben im Browser.
  5. **Terminal:** `read_console("Prompt: ")` zeigt eine Eingabeaufforderung und liest eine Zeile. Das Ergebnis ist `String?`: `None` bedeutet Dateiende, eine leere Zeile ist `Some("")`.

Terminalzugriff ist eine Capability. Die aufrufende Funktion muss `uses
Console` deklarieren. In Projekten mit einer `[capabilities]`-Sektion muss
zusätzlich `console = true` gesetzt werden; neue Projektvorlagen lassen die
Freigabe standardmäßig aus. Konsoleneingabe ist für `zelyra run` gedacht.
Webanwendungen verwenden stattdessen typisierte Requests und Formulare.
Die Eingabe wird nicht verborgen; nutze `read_console()` daher nicht für
Passwörter oder andere Geheimnisse.

```zelyra
fn main() uses Console {
    datum = read_console("Datum: ")
    match datum {
        Some(wert) => {
            print("Eingegeben: " + wert)
        }
        None => {
            print("Keine Eingabe.")
        }
    }
}
```

```toml
[capabilities]
console = true
```

### 4. Kleine, aufeinander aufbauende Beispiele

**Ausgabe formatieren:**
```zelyra
fn main() {
    titel = "Release 1.0"
    prozent = 80
    print("Fortschritt fuer " + titel + ":")
    print(prozent)
}
```

**Eingabe über Funktionsparameter und Umgebungsvariablen:**
```zelyra
fn verarbeite_aufgabe(titel: String, prioritaet: Int) {
    print("Bearbeite: " + titel)
    print("Prioritaetsstufe:")
    print(prioritaet)
}

fn main() uses Environment {
    modus = env("APP_MODUS")
    print("Aktueller Modus:")
    print(modus)
    verarbeite_aufgabe("Backup erstellen", 1)
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Versuchen, eine Zahl direkt mit `+` an einen Text zu hängen: `"Zahl: " + 5`.
  *Ursache:* Zelyra verlangt Typkompatibilität. `5` ist ein `Int`, kein `String`. Gib die Zahl entweder separat mit `print(5)` aus oder nutze Hilfsfunktionen.
- **Fehler:** `env()` ohne `uses Environment` aufrufen.
  *Ursache:* Zelyras Sicherheitsmodell (Capabilities) verlangt, dass Funktionen, die auf das Betriebssystem zugreifen, dies ausdrücklich deklarieren.

### 6. Merksätze
1. `print()` gibt Werte verlässlich auf der Konsole aus.
2. Eingaben fließen über Parameter, Umgebungsvariablen, Dateien, Terminal oder Web-Routen.
3. Systemzugriffe benötigen die passende Capability (z. B. `uses Environment`).

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Gib eine formatierte Visitenkarte (Name, Beruf, E-Mail) mit mehreren `print()`-Befehlen aus.
- **Stufe 2 (Mittel):** Schreibe eine Funktion `drucke_aufgabe(id: Int, name: String, erledigt: Bool)`, die alle Details sauber untereinander darstellt.
- **Stufe 3 (Anspruchsvoll):** Schreibe ein CLI-Programm mit `read_console()` und erkläre, wann Terminaleingabe sinnvoll ist und wann strukturierte Web-Requests besser passen.

### 8. Praxisaufgabe: Ausgabeformatierung für den TaskManager
Erstelle eine Ausgabefunktion für unsere Aufgabenverwaltung:

```zelyra
fn drucke_kopfzeile(bereich: String) {
    print("----------------------------------------")
    print("BEREICH: " + bereich)
    print("----------------------------------------")
}

fn drucke_aufgabe_eintrag(nr: Int, titel: String, erledigt: Bool) {
    print("Aufgabe Nr: ")
    print(nr)
    print("Titel: " + titel)
    if erledigt {
        print("Status: [X] ERLEDIGT")
    } else {
        print("Status: [ ] OFFEN")
    }
    print("----------------------------------------")
}

fn main() {
    drucke_kopfzeile("HEUTIGE AUFGABEN")
    drucke_aufgabe_eintrag(1, "Handbuch durcharbeiten", true)
    drucke_aufgabe_eintrag(2, "Zelyra-Beispiele ueben", false)
}
```

### 9. Zusammenfassung
- Ausgaben erfolgen klar und unmissverständlich über `print()`.
- Externe Zugriffe sind durch Capabilities geschützt.
- Eingaben werden über Parameter, Dateien, Umgebungsvariablen, `read_console()` oder Web-Anfragen entgegengenommen.

### 10. Kontrollfragen zur Selbstprüfung
1. Welche Funktion nutzt man in Zelyra für Textausgaben?
2. Warum verlangt der Zugriff auf `env()` die Angabe `uses Environment`?
3. Welche Eingabewege sind in serverbasierten Zelyra-Anwendungen typisch?

---

## Kapitel 9: Entscheidungen mit Bedingungen

### 1. Was lerne ich in diesem Kapitel?
- Wie Programme mit `if` und `else` eigenständige Entscheidungen treffen.
- Wie man alternative Pfade mit `else if` formuliert.
- Wie man mehrere Bedingungen logisch kombiniert.
- Wie man mit `match` elegante und lückenlose Fallunterscheidungen schreibt.
- Typische Denkfehler bei Verschachtelungen und wie man sie vermeidet.

### 2. Warum ist das Thema wichtig?
Ohne Bedingungen wäre jedes Programm starr wie eine Musikwalze: Es würde immer exakt dieselben Schritte abspielen. Wirklich nützlich wird Software erst, wenn sie auf unterschiedliche Situationen reagiert: Ist der Nutzer eingeloggt? Ist die Frist abgelaufen? Reicht das Guthaben? Mit `if` und `match` verleihst du deinem Code Urteilsvermögen.

### 3. Verständliche Erklärung
- **`if` / `else`:** Prüft eine Bedingung. Wenn sie `true` ergibt, wird der erste Block ausgeführt; andernfalls der `else`-Block:
  ```zelyra
  fn pruefe_ergebnis(punkte: Int) {
      if punkte >= 50 {
          print("Bestanden!")
      } else {
          print("Leider nicht bestanden.")
      }
  }

  fn main() {
      pruefe_ergebnis(75)
  }
  ```
- **`match`:** Wenn du einen Wert gegen viele feste Möglichkeiten prüfen willst, ist `match` viel lesbarer als endlose `if / else if`-Ketten. In Zelyra stellt der Compiler sicher, dass alle möglichen Fälle abgedeckt sind (Exhaustiveness):
  ```zelyra
  fn zeige_status(status_code: Int) {
      match status_code {
          1 => { print("Neu") }
          2 => { print("In Bearbeitung") }
          3 => { print("Erledigt") }
          _ => { print("Unbekannter Status") }
      }
  }

  fn main() {
      zeige_status(2)
  }
  ```
  Der Unterstrich `_` ist das sogenannte **Wildcard-Muster**: Er greift für alle anderen, nicht ausdrücklich genannten Werte.

### 4. Kleine, aufeinander aufbauende Beispiele

**Einfache Verzweigung:**
```zelyra
fn main() {
    offen = 0
    if offen == 0 {
        print("Super! Alle Aufgaben sind erledigt.")
    } else {
        print("Es gibt noch offene Aufgaben.")
    }
}
```

**Mehrstufige Entscheidung (`else if`):**
```zelyra
fn bewerte_prioritaet(stufe: Int) -> String {
    match stufe {
        1 => { return "SEHR DRINGEND" }
        2 => { return "NORMAL" }
        3 => { return "NIEDRIG" }
        _ => { return "UNBEKANNT" }
    }
}

fn main() {
    print(bewerte_prioritaet(1))
    print(bewerte_prioritaet(2))
}
```

**Mustervergleich mit `match`:**
```zelyra
fn status_text(code: Int) -> String {
    match code {
        0 => { return "Entwurf" }
        1 => { return "Aktiv" }
        2 => { return "Archiviert" }
        _ => { return "Ungueltig" }
    }
}

fn main() {
    print(status_text(1))
    print(status_text(99))
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Den `_`-Fall in einem `match` auf Zahlen weglassen.
  *Ursache:* Eine Zahl kann unendlich viele Werte annehmen. Wenn du nur `1` und `2` abdeckst, meldet der Compiler `non-exhaustive match`.
- **Fehler:** Zu tiefe Verschachtelung (Pfeil-Anti-Pattern: `if { if { if { ... } } }`).
  *Ursache:* Schwer lesbar. Löse tiefe Verschachtelungen durch frühes Zurückkehren (`early return`) oder `match` auf.

### 6. Merksätze
1. `if` entscheidet anhand eines Wahrheitswertes (`Bool`).
2. `match` prüft Werte gegen Muster und garantiert Lückenlosigkeit.
3. Der Wildcard `_` fängt alle übrigen Fälle sicher ab.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Schreibe eine Funktion `ist_volljaehrig(alter: Int) -> Bool`, die prüft, ob `alter >= 18` ist.
- **Stufe 2 (Mittel):** Schreibe mit `match` eine Funktion `monats_tage(monat: Int) -> Int`, die für die Monate 1 bis 12 die Tage zurückgibt (Februar pauschal 28).
- **Stufe 3 (Anspruchsvoll):** Baue eine Validierungsfunktion, die prüft, ob ein Passwort mindestens 8 Zeichen lang ist (über einen String-Vergleich) und nicht `"12345678"` lautet.

### 8. Praxisaufgabe: Status-Logik für den TaskManager
Erstelle für unsere Aufgabenverwaltung die automatische Ampel-Einstufung:

```zelyra
fn berechne_ampel(verbleibende_tage: Int, ist_fertig: Bool) -> String {
    if ist_fertig {
        return "GRUEN: Aufgabe ist abgeschlossen"
    } else {
        if verbleibende_tage < 0 {
            return "ROT: Frist ist abgelaufen!"
        } else {
            if verbleibende_tage <= 2 {
                return "GELB: Bald faellig, bitte bearbeiten"
            } else {
                return "BLAU: Im Zeitplan"
            }
        }
    }
}

fn main() {
    print(berechne_ampel(5, false))
    print(berechne_ampel(1, false))
    print(berechne_ampel(-1, false))
    print(berechne_ampel(-1, true))
}
```

### 9. Zusammenfassung
- `if` und `else` steuern den Programmfluss anhand logischer Bedingungen.
- `match` ermöglicht saubere, compilergeprüfte Fallunterscheidungen.
- Klar gegliederte Verzweigungen machen Geschäftslogik verständlich und wartbar.

### 10. Kontrollfragen zur Selbstprüfung
1. Wann ist ein `match` einem `if / else if` vorzuziehen?
2. Welche Aufgabe erfüllt das `_`-Muster im `match`?
3. Warum verlangt Zelyra, dass bei `match` alle möglichen Fälle abgedeckt sind?

---

## Kapitel 10: Wiederholungen und Schleifen

### 1. Was lerne ich in diesem Kapitel?
- Warum Wiederholungen (`Loops`) in der Datenverarbeitung unentbehrlich sind.
- Schleifen mit Zählern: `while bedingung { ... }`.
- Iteration über Listen und Arrays: `for element in array { ... }`.
- Endlosschleifen: `loop { ... }`.
- Schleifen vorzeitig steuern mit `break` (Abbrechen) und `continue` (Überspringen).
- Schleifeninvarianten (`invariant`), mit denen Zelyra Korrektheit formal beweisen kann.

### 2. Warum ist das Thema wichtig?
Computer wurden erfunden, um monotone, wiederkehrende Aufgaben fehlerfrei und in Sekundenschnelle zu erledigen. Wenn du 1.000 Aufgaben aus einer Datenbank laden, prüfen und anzeigen willst, schreibst du den Code nicht tausendmal, sondern einmal innerhalb einer Schleife. Zelyra bietet dafür sichere Konstrukte und erlaubt es sogar, mathematische Garantien über Schleifendurchläufe abzugeben.

### 3. Verständliche Erklärung
Zelyra kennt drei Arten von Schleifen:

1. **`for ... in`:** Der einfachste und sicherste Weg, um eine Liste von Elementen abzuarbeiten. Die Schleife läuft automatisch über jedes Element und stoppt von selbst:
   ```zelyra
   fn main() {
       for zahl in [1, 2, 3] {
           print(zahl)
       }
   }
   ```
2. **`while bedingung`:** Wiederholt den Block so lange, wie die Bedingung wahr (`true`) bleibt. Ideal, wenn du vorher nicht weißt, wie viele Durchläufe nötig sind.
3. **`loop`:** Eine Dauerschleife. Sie läuft endlos weiter, bis sie im Inneren durch den Befehl `break` gestoppt wird.

**Steuerbefehle:**
- **`break`**: Beendet die Schleife sofort. Das Programm springt hinter die Schleife.
- **`continue`**: Bricht den aktuellen Durchlauf ab und springt sofort zum nächsten Element.

**Schleifeninvariante (`invariant`):**
Eine Invariante ist eine Bedingung, die **vor**, **während** und **nach** jedem Schleifendurchlauf wahr sein muss (z. B. `invariant { zaehler >= 0 }`). Der Zelyra-Verifier (`zelyra verify`) nutzt Invarianten, um mathematisch zu beweisen, dass die Schleife niemals in unzulässige Zustände gerät!

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: `for ... in` über ein Array**
```zelyra
fn main() {
    aufgaben = ["Planen", "Programmieren", "Testen", "Ausliefern"]
    for a in aufgaben {
        print("Schritt: " + a)
    }
}
```

**Beispiel 2: `while` mit Zähler und `invariant`**
```zelyra
fn main() {
    mutable zaehler = 1
    while zaehler <= 3
        invariant { zaehler >= 1 }
    {
        print(zaehler)
        zaehler = zaehler + 1
    }
}
```

**Beispiel 3: `break` und `continue` gezielt einsetzen**
```zelyra
fn main() {
    for zahl in [1, 2, 3, 4, 5] {
        if zahl == 2 {
            // Die 2 wollen wir auslassen:
            continue
        }
        if zahl == 4 {
            // Bei 4 brechen wir komplett ab:
            break
        }
        print(zahl)
    }
}
```
*Ausgabe:* Gibt `1` und `3` aus!

### 5. Typische Fehler und deren Ursachen
- **Fehler:** In einer `while`-Schleife vergessen, den Zähler zu erhöhen (`zaehler = zaehler + 1`).
  *Ursache:* Die Bedingung bleibt ewig wahr – eine **Endlosschleife** entsteht und das Programm friert ein.
- **Fehler:** Falsche Indexgrenzen bei manuellen Zählern.
  *Ursache:* Nutze wann immer möglich `for item in array`, um Grenzfehler (Off-by-one) komplett auszuschließen.

### 6. Merksätze
1. Nutze `for ... in` für Sammlungen und Arrays.
2. `break` beendet die Schleife sofort; `continue` springt zur nächsten Runde.
3. Invarianten dokumentieren und beweisen die Sicherheit deiner Schleife.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Gib mit einer `while`-Schleife die Zahlen von 10 rückwärts bis 1 aus.
- **Stufe 2 (Mittel):** Berechne mit einer `for`-Schleife die Summe aller Zahlen im Array `[10, 20, 30, 40]`.
- **Stufe 3 (Anspruchsvoll):** Durchsuche ein Array von Zahlen nach der Zahl `42`. Sobald sie gefunden wird, gib `"Gefunden!"` aus und beende die Schleife mit `break`. Wenn sie nicht vorkommt, gib am Ende `"Nicht gefunden"` aus.

### 8. Praxisaufgabe: Aufgaben filtern und zählen
Wir wenden Schleifen auf unsere Aufgabenverwaltung an. Wir zählen die erledigten Aufgaben und geben offene Aufgaben aus:

```zelyra
fn main() {
    aufgaben = ["Konzept schreiben", "Datenbank aufsetzen", "Tests schreiben"]
    mutable erledigt_zaehler = 0

    print("Aufgabenliste durchgehen:")
    for a in aufgaben {
        if a == "Konzept schreiben" {
            print("[X] " + a)
            erledigt_zaehler = erledigt_zaehler + 1
        } else {
            print("[ ] " + a)
        }
    }

    print("Erledigte Aufgaben insgesamt:")
    print(erledigt_zaehler)
}
```

### 9. Zusammenfassung
- Schleifen automatisieren monotone Wiederholungen.
- `for ... in` iteriert sicher über Arrays, `while` wiederholt nach Bedingungen.
- `break` und `continue` ermöglichen präzise Flusssteuerung.
- `invariant` ermöglicht formale Korrektheitsbeweise mit `zelyra verify`.

### 10. Kontrollfragen zur Selbstprüfung
1. Was ist der Unterschied zwischen `break` und `continue`?
2. Warum ist `for ... in` bei Arrays sicherer als eine manuelle `while`-Schleife?
3. Welche Rolle spielt eine `invariant` bei der Programmprüfung?

# TEIL III – PROGRAMME STRUKTURIEREN

---

## Kapitel 11: Funktionen und Prozeduren

### 1. Was lerne ich in diesem Kapitel?
In diesem Kapitel lernst du:
- Was Funktionen sind und wie sie Programme übersichtlich und wiederverwendbar machen.
- Wie man Parameter übergibt und Rückgabetypen mit `-> Typ` deklariert.
- Was reine Funktionen (*pure functions*) ohne Seiteneffekte sind und warum sie Gold wert sind.
- Was Prozeduren sind (Funktionen ohne Rückgabewert bzw. Typ `Unit`), die Aktionen ausführen.
- Wie Zelyra Namenskonventionen und sauberen Stil fördert.

### 2. Warum ist das Thema wichtig?
Wenn du jede Berechnung und jeden Bildschirmausdruck zehnmal an verschiedenen Stellen deines Codes wiederholst, entsteht das gefürchtete „Spaghetti-Phänomen“. Ändert sich eine Geschäftsregel (z. B. wie die Frist einer Aufgabe berechnet wird), müsstest du alle zehn Stellen suchen und anpassen – Fehler sind dabei unausweichlich. Funktionen fassen eine logische Aufgabe unter einem klaren Namen zusammen: Du schreibst sie einmal, testest sie gründlich und verwendest sie beliebig oft.

### 3. Verständliche Erklärung
Stell dir eine Funktion wie ein Küchengerät vor:
- Du gibst Zutaten hinein (**Parameter**).
- Das Gerät verarbeitet die Zutaten nach einem festen Rezept (**Funktionskörper**).
- Am Ende kommt ein fertiges Gericht heraus (**Rückgabewert**).

In Zelyra definierst du Funktionen mit dem Schlüsselwort `fn`:
```zelyra
fn addiere(a: Int, b: Int) -> Int {
    return a + b
}

fn main() {
    print(addiere(2, 3))
}
```
Besitzt eine Funktion keinen Rückgabewert, weil sie z. B. nur eine Zeile Text ausgibt, ist ihr Rückgabetyp `Unit` (oder kann weggelassen werden):
```zelyra
fn drucke_trennlinie() {
    print("----------------------------------------")
}

fn main() {
    drucke_trennlinie()
}
```

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Berechnung mit Rückgabewert**
```zelyra
fn berechne_tage_bis_frist(heute_tag: Int, frist_tag: Int) -> Int {
    return frist_tag - heute_tag
}

fn main() {
    tage = berechne_tage_bis_frist(10, 18)
    print(tage)
}
```

**Beispiel 2: Textformatierung in einer Hilfsfunktion**
```zelyra
fn format_aufgabe(id_text: String, text: String, fertig: Bool) -> String {
    mutable status_zeichen = "[ ]"
    if fertig {
        status_zeichen = "[X]"
    }
    return status_zeichen + " #" + id_text + ": " + text
}

fn main() {
    ausgabe = format_aufgabe("1", "Dokumentation lesen", true)
    print(ausgabe)
}
```

**Beispiel 3: Prozedur zur Menüausgabe**
```zelyra
fn zeige_kopfzeile(benutzer: String) {
    print("Angemeldet als: " + benutzer)
    print("========================================")
}

fn main() {
    zeige_kopfzeile("Sabine")
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Ein `return` vergessen, obwohl ein Rückgabetyp angegeben wurde.
  *Ursache:* Zelyra verlangt bei deklariertem Rückgabetyp zwingend ein passendes Ergebnis auf allen Ausführungspfaden.
- **Fehler:** Falsche Argumentreihenfolge beim Aufruf.
  *Ursache:* Zelyra prüft die Parametertypen strikt. Wenn der erste Parameter ein `Int` ist, darf kein `String` übergeben werden.
- **Fehler:** Versuchen, `String` und `Int` direkt mit `+` zu verketten.
  *Ursache:* Der Operator `+` verknüpft in Zelyra entweder zwei Strings oder zwei Zahlen gleichen Typs. Verwende Strings oder gebe Werte separat über `print()` aus.

### 6. Merksätze
1. Eine Funktion sollte genau eine einzige, klar benannte Aufgabe erfüllen.
2. Reine Funktionen erzeugen bei gleichen Eingaben immer die gleichen Ausgaben und haben keine Seiteneffekte.
3. Namen von Funktionen sollten aussagekräftige Verben sein (z. B. `berechne_differenz`, `format_aufgabe`).

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Schreibe eine Funktion `verdopple(zahl: Int) -> Int`, die das Doppelte einer Zahl zurückgibt.
- **Stufe 2 (Mittel):** Schreibe eine Funktion `ist_dringend(tage: Int) -> Bool`, die `true` liefert, wenn weniger als 3 Tage verbleiben.
- **Stufe 3 (Anspruchsvoll):** Schreibe eine Funktion `status_symbol(erledigt: Bool) -> String`, die `"[OK]"` oder `"[OFFEN]"` liefert.

### 8. Praxisaufgabe: Aufgabenanzeige modularisieren
Schreibe ein Zelyra-Programm, das drei Aufgaben mit ID-Text, Name und Erledigungsstatus über eine wiederverwendbare Formatierungsfunktion aufbereitet und ausgibt:

```zelyra
fn format_eintrag(id_text: String, name: String, erledigt: Bool) -> String {
    mutable symbol = "[OFFEN]"
    if erledigt {
        symbol = "[OK]"
    }
    return symbol + " Aufgabe " + id_text + ": " + name
}

fn main() {
    print(format_eintrag("1", "Post abholen", true))
    print(format_eintrag("2", "Rechnung bezahlen", false))
    print(format_eintrag("3", "Backup erstellen", false))
}
```

### 9. Zusammenfassung
- Funktionen gliedern Programme in logische, handhabbare Bausteine.
- Parameter und Rückgabetypen sind in Zelyra exakt typisiert.
- Reine Funktionen machen den Code wartungsfreundlich und fehlerresistent.

### 10. Kontrollfragen zur Selbstprüfung
1. Welchen Typ besitzt eine Funktion, die keinen Wert zurückgibt?
2. Warum sind reine Funktionen einfacher zu testen als Funktionen mit globalen Seiteneffekten?
3. Was prüft der Zelyra-Compiler bei jedem Funktionsaufruf?

---

## Kapitel 12: Verträge und Vorbedingungen (Design by Contract)

### 1. Was lerne ich in diesem Kapitel?
- Was das Konzept *Design by Contract* (Entwurf durch Vertrag) bedeutet.
- Wie du Vorbedingungen mit `requires` definierst.
- Wie du Nachbedingungen mit `ensures` und dem Schlüsselwort `result` formulierst.
- Wie Schleifeninvarianten mit `invariant` sichergestellt werden.
- Warum Verträge defensiven `if`-Kaskaden überlegen sind.

### 2. Warum ist das Thema wichtig?
Häufig entstehen Fehler, weil Entwickler Annahmen treffen, die nirgendwo festgeschrieben sind: „Diese Funktion darf niemals mit einer negativen Zahl aufgerufen werden!“ Wenn jemand die Funktion ein halbes Jahr später doch mit `-1` aufruft, kracht es unerwartet. In herkömmlichen Sprachen schreibt man seitenlange `if`-Prüfungen oder hofft auf gute Kommentare. Zelyra hebt Verträge auf Sprachebene: Vor- und Nachbedingungen sind Teil der Funktionssignatur und werden garantiert geprüft.

### 3. Verständliche Erklärung
Ein Vertrag in Zelyra funktioniert wie ein notarieller Vertrag zwischen dem Aufrufer und der Funktion:
- **`requires` (Vorbedingung):** Der Aufrufer verpflichtet sich, der Funktion nur Daten zu übergeben, die die Bedingung erfüllen (z. B. `wert > 0`).
- **`ensures` (Nachbedingung):** Im Gegenzug garantiert die Funktion, dass ihr Ergebnis (`result`) bestimmte Eigenschaften aufweist (z. B. `result >= 0`).

Wird ein Vertrag verletzt, stoppt Zelyra sofort mit einem präzisen Fehler (`E-CONTRACT-*`) und zeigt genau, wer den Vertrag gebrochen hat.

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Vorbedingung mit requires**
```zelyra
fn dividiere(zaehler: Int, nenner: Int) -> Int
    requires { nenner != 0 }
{
    return zaehler / nenner
}

fn main() {
    ergebnis = dividiere(100, 4)
    print(ergebnis)
}
```

**Beispiel 2: Vor- und Nachbedingung kombiniert**
```zelyra
fn prioritaet_anpassen(aktuelle_stufe: Int, delta: Int) -> Int
    requires { aktuelle_stufe >= 1 && delta >= 0 }
    ensures { result >= 1 }
{
    neue_stufe = aktuelle_stufe + delta
    return neue_stufe
}

fn main() {
    p = prioritaet_anpassen(2, 1)
    print(p)
}
```

**Beispiel 3: Schleifeninvariante**
```zelyra
fn zaehle_bis(grenze: Int) -> Int
    requires { grenze >= 0 }
    ensures { result == grenze }
{
    mutable i = 0
    while i < grenze
        invariant { i >= 0 }
    {
        i = i + 1
    }
    return i
}

fn main() {
    print(zaehle_bis(5))
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Benutzereingaben über `requires` abfangen wollen.
  *Ursache:* Verträge sind Schutzschilde gegen Programmierfehler im Code, nicht zur Validierung von unsicheren Benutzereingaben gedacht. Für Benutzereingaben nutzt man Validierungsregeln (`form` oder `Result`).
- **Fehler:** Ein `ensures` formulieren, das die Funktion logisch nicht einhalten kann.
  *Ursache:* Wenn die Funktion z. B. `-5` zurückgibt, das `ensures` aber `{ result >= 0 }` verlangt, schlägt die Nachbedingung fehl.

### 6. Merksätze
1. `requires` schützt die Funktion vor unzulässigen Eingaben des Aufrufers.
2. `ensures` garantiert dem Aufrufer ein korrektes Ergebnis über `result`.
3. Verträge machen implizite Annahmen zu überprüfbaren, lebendigen Spezifikationen.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Schreibe eine Funktion `quadratwurzel_naiv(x: Int) -> Int` mit der Vorbedingung `requires { x >= 0 }`.
- **Stufe 2 (Mittel):** Schreibe eine Funktion `begrenze(wert: Int, min_wert: Int, max_wert: Int) -> Int` mit Vorbedingungen und passender Nachbedingung.
- **Stufe 3 (Anspruchsvoll):** Sichere eine Funktion `prozentsatz(teil: Int, gesamt: Int) -> Int` so ab, dass niemals durch 0 geteilt wird und das Ergebnis stets zwischen 0 und 100 liegt.

### 8. Praxisaufgabe: Fortschrittsrechner für Aufgaben
Schreibe eine abgesicherte Funktion für die Aufgabenverwaltung:
```zelyra
fn berechne_fortschritt(erledigt: Int, gesamt: Int) -> Int
    requires { gesamt > 0 && erledigt >= 0 && erledigt <= gesamt }
    ensures { result >= 0 && result <= 100 }
{
    return (erledigt * 100) / gesamt
}

fn main() {
    quote = berechne_fortschritt(3, 4)
    print(quote)
}
```

### 9. Zusammenfassung
- Verträge (`requires`, `ensures`) dokumentieren und erzwingen Programmierannahmen zur Laufzeit.
- `result` verweist in `ensures` auf das Berechnungsergebnis der Funktion.
- Schleifeninvarianten sichern den inneren Zustand von Schleifendurchläufen ab.

### 10. Kontrollfragen zur Selbstprüfung
1. Wann wird eine `requires`-Bedingung geprüft: vor oder nach der Ausführung?
2. Worauf bezieht sich das Wort `result` in einem Vertrag?
3. Warum ersetzt ein Vertrag kein HTML-Formularvalidierungs-Muster?

---

<a id="kapitel-13-sammlungen-listen-und-worterbucher-arrays-und-maps"></a>

## Kapitel 13: Sammlungen, Listen und Wörterbücher (Arrays & Maps)

### 1. Was lerne ich in diesem Kapitel?
- Wie du mehrere gleichartige Werte in einem Array (`Typ[]`) speicherst.
- Die wichtigsten Listen-Funktionen: `len`, `append`, `contains`, `first`, `last`.
- Wie du Listen mit `for .. in` durchläufst.
- Wie assoziative Schlüssel-Wert-Sammlungen mit `Map<Schlüssel, Wert>` deklariert und genutzt werden.
- Die wichtigsten Wörterbuch-Funktionen: `get`, `put`, `contains`, `keys`, `values`.
- Warum Zelyras `get()` immer eine sichere `Option` liefert und Abstürze verhindert.

### 2. Warum ist das Thema wichtig?
Eine Anwendung, die nur einzelne Werte speichern kann, wäre nutzlos. In der Praxis arbeiten wir fast immer mit Sammlungen: Listen von Aufgaben, E-Mail-Adressen oder Tabellenzeilen aus einer Datenbank. 
Manchmal suchen wir Elemente nach ihrer Reihenfolge (Listen bzw. Arrays). Sehr oft wollen wir Daten jedoch direkt über einen eindeutigen Schlüssel nachschlagen – zum Beispiel die Benutzereinstellungen zu einer User-ID, ein Wörterbuch von Übersetzungstexten oder Ländervorwahlen. Dafür bietet Zelyra typisierte Wörterbücher (`Map`).

### 3. Verständliche Erklärung

#### Teil A: Arrays – Die geordnete Liste
Ein Array ist wie ein Setzkasten: Jedes Fach hat eine feste Nummer (Index) und enthält genau ein Element. In Zelyra müssen alle Elemente im selben Kasten denselben Typ haben (`Int[]`, `String[]` etc.):

```zelyra
fn main() {
    zahlen: Int[] = [10, 20, 30, 40]
    print(len(zahlen))
}
```

Zelyra stellt mächtige Hilfswerkzeuge für Arrays bereit:
- `len(liste)`: Gibt die Anzahl der Elemente zurück.
- `append(liste, wert)`: Liefert ein neues Array mit dem angehängten Element.
- `first(liste)`: Liefert das erste Element als `Option`.
- `last(liste)`: Liefert das letzte Element als `Option`.
- `contains(liste, wert)`: Prüft, ob ein Element vorhanden ist (`Bool`).

#### Teil B: Maps – Assoziative Schlüssel-Wert-Wörterbücher
Eine `Map` ordnet jedem eindeutigen Schlüssel (*Key*) genau einen Wert (*Value*) zu.
- **Typnotation:** `Map<KeyTyp, WertTyp>`, zum Beispiel `Map<String, Int>` oder `Map<String, String>`.
- **Literalschreibweise:** `Map { "schluessel": wert }`
- **Schlüsseltypen:** Alle Skalartypen (`String`, `Int`, `Id` etc.) sind zulässig.

```zelyra
fn main() {
    vorwahlen: Map<String, Int> = Map {
        "de": 49
        "at": 43
        "ch": 41
    }
}
```

Die wichtigsten Operationen auf Maps:
- **`get(map, key)`**: Schlägt einen Schlüssel nach. Weil ein Schlüssel fehlen könnte, liefert `get()` **immer** eine sichere `Option<Wert>` zurück (`Some(wert)` oder `None`) – niemals einen Null-Pointer-Absturz!
- **`put(map, key, wert)`**: Fügt ein Paar hinzu oder überschreibt den bestehenden Wert. Da Zelyra Unveränderlichkeit schätzt, liefert `put()` eine neue, aktualisierte Map zurück.
- **`contains(map, key)`**: Gibt `true` zurück, wenn der Schlüssel existiert.
- **`keys(map)`**: Gibt alle vorhandenen Schlüssel als typisiertes Array zurück (`KeyTyp[]`).
- **`values(map)`**: Gibt alle Werte als typisiertes Array zurück (`WertTyp[]`).

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Array anlegen und mit for-Schleife durchlaufen**
```zelyra
fn main() {
    aufgabe_ids: Int[] = [101, 102, 103, 104]
    for id in aufgabe_ids {
        print(id)
    }
}
```

**Beispiel 2: Elemente an ein Array anfügen**
```zelyra
fn main() {
    mutable liste: Int[] = [1, 2, 3]
    liste = append(liste, 4)
    print(len(liste))
}
```

**Beispiel 3: Wörterbuch (Map) anlegen, aktualisieren und abfragen**
```zelyra
fn main() {
    preise: Map<String, Int> = Map {
        "Kaffee": 3
        "Tee": 2
    }
    
    // Neues Element hinzufügen
    mutable aktion = put(preise, "Kuchen", 4)
    // Vorhandenen Preis anpassen
    aktion = put(aktion, "Kaffee", 4)
    
    // Nullsicher mit match abfragen
    match get(aktion, "Kaffee") {
        Some(preis) => {
            print("Kaffeepreis: " + str(preis) + " Euro")
        }
        None => {
            print("Artikel nicht gefunden.")
        }
    }
    
    // Alle Schlüssel und Werte ausgeben
    print(keys(aktion))
}
```

**Beispiel 4: Prüfen der Existenz in einer Map**
```zelyra
fn main() {
    einstellungen: Map<String, Bool> = Map {
        "dunkelmodus": true
        "benachrichtigungen": false
    }
    
    if contains(einstellungen, "dunkelmodus") {
        print("Dunkelmodus-Einstellung ist konfiguriert.")
    }
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Versuchen, verschiedene Typen in einem Array oder einer Map zu mischen (z. B. `[1, "Hallo"]`).
  *Ursache:* Zelyra ist strikt typrein. Alle Elemente eines Arrays müssen denselben Typ haben, und alle Schlüssel bzw. Werte einer Map müssen ihren deklarierten Typen entsprechen.
- **Fehler:** `map.get("key")` direkt als Wert behandeln wollen, ohne die `Option` auszupacken.
  *Ursache:* Zelyra garantiert Compile-Time-Sicherheit. Da ein Schlüssel in der Map fehlen könnte, zwingt der Compiler dich, mit `match` oder Standardwerten auf `Some` und `None` zu reagieren.
- **Fehler:** Nicht-skalare Typen (wie Arrays) als Map-Schlüssel verwenden.
  *Ursache:* Map-Schlüssel müssen skalare Typen sein (`String`, `Int`, `Id`), damit sie deterministisch vergleichbar und JSON-serialisierbar sind.

### 6. Merksätze
1. Arrays in Zelyra sind typrein: `Int[]`, `String[]`, `Bool[]`.
2. Das Durchlaufen von Arrays erfolgt elegant und sicher mit `for element in sammlung { ... }`.
3. `Map<Key, Value>` speichert eindeutige Schlüssel-Wert-Paare.
4. `get(map, key)` liefert immer eine `Option` (`Some` oder `None`) und schützt vor Laufzeitabstürzen.
5. `put(map, key, value)` liefert funktional ein neues, aktualisiertes Wörterbuch.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Erstelle ein Array mit drei Zeichenketten und gib jedes Element mit einer `for`-Schleife aus.
- **Stufe 2 (Mittel):** Erstelle eine `Map<String, Int>` mit drei Produktnamen und ihren Preisen. Frage einen vorhandenen und einen nicht vorhandenen Artikel mit `get()` und `match` ab.
- **Stufe 3 (Anspruchsvoll):** Schreibe eine Funktion `zaehle_woerter(liste: String[]) -> Map<String, Int>`, die zählt, wie oft jedes Wort in einer Liste vorkommt, und das Ergebnis als Map zurückgibt.

### 8. Praxisaufgabe: Aufgaben-Prioritäten nachschlagen
Erstelle eine Verwaltung von Prioritätsstufen für Aufgaben mit einer Map:
```zelyra
fn prioritaet_anzeigen(prioritaeten: Map<String, Int>, aufgabe: String) {
    match get(prioritaeten, aufgabe) {
        Some(stufe) => {
            print("Prioritaet fuer " + aufgabe + ": Stufe " + str(stufe))
        }
        None => {
            print("Keine Prioritaet hinterlegt fuer: " + aufgabe)
        }
    }
}

fn main() {
    prio_map: Map<String, Int> = Map {
        "Datenbankmigration": 1
        "CSS anpassen": 3
        "Dokumentation": 2
    }
    
    prioritaet_anzeigen(prio_map, "Datenbankmigration")
    prioritaet_anzeigen(prio_map, "Kaffeepause")
}
```

### 9. Zusammenfassung
- Arrays (`Typ[]`) speichern geordnete Folgen von Werten desselben Typs.
- Maps (`Map<Key, Value>`) speichern Schlüssel-Wert-Zuordnungen mit sicherem `get()`, `put()`, `contains()`, `keys()` und `values()`.
- Beide Sammlungen sind vollständig typsicher und arbeiten nahtlos mit Zelyras `Option`-System zusammen.

### 10. Kontrollfragen zur Selbstprüfung
1. Welchen Typ hat der Ausdruck `["A", "B", "C"]`?
2. Warum schlägt `append([1, 2], "Drei")` beim Kompilieren fehl?
3. Wie prüfst du effizient, ob ein Wert in einer Liste existiert?

---

## Kapitel 14: Eigene Datentypen erstellen (Records & Tables)

### 1. Was lerne ich in diesem Kapitel?
- Wie du eigene domänenspezifische Datentypen erstellst.
- Was nominale Typen (`type TaskId = Id`) sind und wie sie Verwechslungen verhindern.
- Wie Datensätze in Zelyra als `table` mit Attributen definiert werden (`id: Id primary auto`).
- Warum starke Typisierung die Softwarequalität revolutioniert.

### 2. Warum ist das Thema wichtig?
In schlechter Software wird fast alles als einfache Zahl oder Zeichenkette behandelt („Primitive Obsession“). Wenn eine Funktion `pruefe(benutzer_id: Int, aufgabe_id: Int)` erwartet, du aber versehentlich die IDs vertauschst, merkt der Computer bei einfachen `Int`-Typen gar nichts – die Software bucht womöglich fatale Daten falsch zu. Mit eigenen Typen unterscheidet Zelyra schon beim Kompilieren zwischen einer `UserId` und einer `TaskId`.

### 3. Verständliche Erklärung
Eigene Datentypen erlauben es dir, der realen Welt einen Namen zu geben:
1. **Nominale Aliase:**
   ```zelyra
   type TaskId = Id

   fn main() {
       print("Typalias TaskId aktiv")
   }
   ```
   Hierdurch wird `TaskId` ein eigener, eindeutiger Typ.
2. **Tabellen als strukturierte Datentypen:**
   ```zelyra
   table aufgaben {
       id: Id primary auto
       beschreibung: String required
       fertig: Bool
   }

   fn main() {
       print("Tabellenschema definiert")
   }
   ```
   Dieses Schema definiert nicht nur eine Datenbanktabelle, sondern stellt in Zelyra automatisch den Datentyp für eine Aufgabe bereit!

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Eigene Typnamen für IDs**
```zelyra
type TaskId = Id

fn main() {
    print("Typalias erfolgreich definiert")
}
```

**Beispiel 2: Ein Datenmodell als Tabelle definieren**
```zelyra
table aufgaben {
    id: Id primary auto
    beschreibung: String required
    erledigt: Bool
}

fn main() {
    print("Tabellenschema fuer Aufgaben definiert")
}
```

**Beispiel 3: Typisierte Attribute in Funktionen ansprechen**
```zelyra
table projekte {
    id: Id primary auto
    name: String required
    aktiv: Bool
}

fn zeige_projekt_status(p_name: String, p_aktiv: Bool) {
    mutable status = "Pausiert"
    if p_aktiv {
        status = "Aktiv"
    }
    print("Projekt: " + p_name + " [" + status + "]")
}

fn main() {
    zeige_projekt_status("Web-Portal", true)
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Ein Tabellenfeld mit einem reservierten Schlüsselwort wie `title` oder `list` benennen.
  *Ursache:* In Zelyra sind diese Wörter für Abfragen und Ansichten reserviert. Nutze stattdessen aussagekräftige Namen wie `name`, `text` oder `beschreibung`.
- **Fehler:** Den Primärschlüssel mit `primary key` statt `primary` angeben.
  *Ursache:* In Zelyra lautet die Spaltenspezifikation `id: Id primary auto`.

### 6. Merksätze
1. Eigene Datentypen spiegeln die Geschäftswelt wider und verhindern fatale Parameterverwechslungen.
2. `table`-Definitionen sind in Zelyra gleichzeitig Datenbankschema und Sprach-Datentyp.
3. Reservierte Wörter (`title`, `list`, etc.) dürfen nicht als Spalten- oder Variablennamen genutzt werden.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Erstelle einen Typalias `type UserId = Id`.
- **Stufe 2 (Mittel):** Modelliere eine Tabelle `kategorien` mit `id: Id primary auto` und `kategorie_name: String required`.
- **Stufe 3 (Anspruchsvoll):** Modelliere eine Tabelle `notizen`, die über ein Feld `aufgabe_id: Id` mit einer Aufgabe verknüpft werden kann.

### 8. Praxisaufgabe: Das Kernschema der Aufgabenverwaltung
Definiere das vollständige Zelyra-Schema für unsere Aufgabenverwaltung:

```zelyra
table aufgaben {
    id: Id primary auto
    name: String required
    beschreibung: String
    prioritaet: Int
    ist_erledigt: Bool
}

fn main() {
    print("Kernschema der Aufgabenverwaltung aktiv.")
}
```

### 9. Zusammenfassung
- Eigene Typen geben Daten eine klare, unmissverständliche Bedeutung.
- `table` vereint Schema-Deklaration und statische Typdefinition nahtlos in der Sprache.
- Statische Typen fangen Logikfehler bereits beim Schreiben des Codes ab.

### 10. Kontrollfragen zur Selbstprüfung
1. Welchen Vorteil bietet `type TaskId = Id` gegenüber einem einfachen `Int`?
2. Warum müssen Tabellenfelder in Zelyra stets einen festen Typ besitzen?
3. Welche Namen sollten als Feldbezeichnungen vermieden werden?

---

## Kapitel 15: Module und Code-Organisation

### 1. Was lerne ich in diesem Kapitel?
- Wie ein typisches Zelyra-Projekt strukturiert ist.
- Die Rolle der Projektkonfigurationsdatei `zelyra.toml`.
- Wie Code in logische Bestandteile (Schema, Logik, Ansichten) gegliedert wird.
- Wie ein Projekt organisiert ist und welche Grenze die CLI bei mehreren
  Quelldateien derzeit hat.
- Den aktuellen Entwicklungsstand und die Roadmap von Modulen und Imports.

### 2. Warum ist das Thema wichtig?
Zu Beginn schreibt man gerne alles in eine einzige Datei. Doch wenn deine Aufgabenverwaltung wächst – mit Datenbanktabellen, 20 Funktionen, Webformularen und Validierungen –, verliert man in einer 2000-Zeilen-Datei schnell den Überblick. Gute Softwareentwicklung bedeutet, Code so zu organisieren, dass man Neuerungen sofort an der richtigen Stelle findet.

### 3. Verständliche Erklärung
Ein Projekt besitzt eine klare Ablagestruktur. `zelyra.toml` enthält
Projektangaben und Capability-Freigaben; `main.zyl` ist die vom Starterprojekt
angelegte Quelldatei. Weitere Dateien oder Unterordner kannst du zur eigenen
Ordnung anlegen. Im veröffentlichten Release 0.3.0 verarbeitet jeder Befehl
nur die ausdrücklich angegebene Quelldatei; weitere `.zyl`-Dateien werden
weder automatisch gefunden noch zusammengeführt.

```toml
[project]
name = "aufgaben_planer"
version = "0.3.0"
zelyra = "0.1"

[capabilities]
database = true
network = false
console = false
```

**Stand des veröffentlichten Releases 0.3.0:** `import` und ein Modulsystem
sind nicht
implementiert. `zelyra check main.zyl` prüft `main.zyl`; eine zusätzliche Datei
wie `src/schema.zyl` wird nur mit einem eigenen Aufruf wie
`zelyra check src/schema.zyl` geprüft. Namen und Typen aus einer Datei stehen
der anderen dadurch nicht automatisch zur Verfügung. Das veröffentlichte
Release enthält keine Modulimporte. Ein experimenteller, begrenzter
Modul-Importpfad ist im aktuellen Entwicklungszweig vorhanden; vollständige Module und
deterministische Mehrdateiprojekte bleiben Arbeit für 0.4.0.

**🧪 Aktueller, noch unveröffentlichter Entwicklungszweig:** Ein erster
Projekt-Modulschnitt ist implementiert und getestet. Die Syntax lautet:

~~~zelyra
import "src/math.zyl" as math

fn main() {
    print(math::add(2, 3))
}
~~~

Die importierte Datei darf Funktionen, Typ-Aliase, Records, Tabellen,
`tableview`-Definitionen, Seiten, benannte Views, typisierte Komponenten,
Formulare, CRUD-Deklarationen, API-Routen und Authentifizierungsdefinitionen
enthalten. API-Handler und Typen werden im jeweiligen Modulkontext aufgelöst;
Authentifizierungstabellen werden gegen das gemeinsame Schema geprüft.
Funktionen, Typen, Records, Views und Komponenten brauchen `pub`, wenn andere
Dateien sie über einen Alias verwenden. Views und Komponenten sind
standardmäßig privat und werden mit `pub view` beziehungsweise
`pub component` exportiert. Modulübergreifende Page- und CRUD-Layout-Verweise
sowie erkannte Komponenten-Tags in Page-, View-, Komponenten- und CRUD-Slot-HTML
benötigen einen direkten oder transitiven Importpfad zum Besitzer. Private
UI-Verweise melden `E-MOD-007`, fehlende Kanten `E-MOD-020`. Die
Komponentenerkennung scannt bekannte Tagnamen im HTML-Text; sie ist keine
vollständige HTML- oder Namespace-Analyse.
Tabellen fließen in ein gemeinsames Schema ein und behalten globale SQL-Namen;
doppelte Tabellen- und Ressourcennamen werden abgelehnt. Eine
Datenbankdefinition gilt projektweit und höchstens eine ist zulässig.

`check`, `build`, `run`, `serve`, `context`, `verify`, `impact` und die
Datenbankbefehle laden den Projektgraphen. `serve` kann importierte
`tableview`s, Seiten, Views, Komponenten, Formulare, CRUD-Ressourcen,
API-Routen und Authentifizierungskonfiguration in die Anwendung integrieren.
Importierte Seiten werden projektweit als Routen zusammengesetzt;
überschneidende Seitenpfade werden mit Verweis auf die importierte Quelldatei
abgelehnt.
Tableview-Abfragen laufen derzeit über MariaDB; SQLite-Tableviews werden noch
nicht ausgeführt.
`context --format=json` zeigt den deterministisch sortierten Modulgraphen,
Importkanten und pro Datei die derzeit unterstützten öffentlichen Funktionen,
Typen und Records unter `modules[].exports`. Die Modul-Einträge enthalten auch
Importalias und projektrelativen Quellpfad. Importierte Tabellen,
`tableview`-Definitionen, Seiten, Views, Komponenten, Formulare, CRUD-, API-
und Authentifizierungsdefinitionen erscheinen mit ihrem
`span.file`-Quellpfad in den Deklarationen. Diese Exportliste ist eine
Introspektionshilfe; sie ist noch kein vollständiges Paket- oder
Deploymentmanifest und macht UI-Ressourcen nicht öffentlich. Typ-,
Capability- und Contract-Prüfungen bleiben aktiv; einige Template-Diagnosen
brauchen noch eine vollständigere Zuordnung zur Quelldatei. `verify` bewahrt
noch keine Modul-Quellzuordnung in seinen Ergebnissen. `fmt` und `edit`
bearbeiten weiterhin nur die ausdrücklich angegebene Quelldatei.
Datenbankbefehle bauen das Schema aus dem verknüpften Projektgraphen auf;
Quellzuordnung für Schemafehler aus importierten Dateien ist noch nicht in
allen Fällen vollständig. Dieses Branch-Verhalten ist experimentell und nicht im
veröffentlichten 0.3.0-Binary enthalten.

Das MariaDB-Dockerprojekt kopiert zusätzlich den konventionellen Ordner
`src/` in das Laufzeitimage. Mehrdateienprojekte mit importierten
Zelyra-Dateien unter `src/` können dadurch im Container geladen werden.
Das ist weder ein Export einzelner Module noch ein unabhängig konfigurierbares
Datenbankmodul; Zugangsdaten gehören weiterhin ausschließlich in die lokale
Laufzeitkonfiguration und nicht in Quelldateien.

Ein weiterer experimenteller Vorläufer für spätere Modul-Exporte ist:

~~~sh
zelyra module plan main.zyl src/invoices.zyl
~~~

Die Vorschau kann statt bei einer Quelldatei auch bei einer unterstützten
Anwendungsressource beginnen. IDs mit Leerzeichen müssen als ein Argument
zitiert werden:

~~~sh
zelyra module plan examples/modules/main.zyl 'page:/invoices'
~~~

Unterstützte Ressourcen-Wurzeln sind `page:<pfad>`, `api:<METHODE> <pfad>`,
`crud:<name>`, `form:<name>` und `tableview:<name>`. Der Plan startet beim
Quellmodul der Ressource und verfolgt dessen bekannte Abhängigkeiten; alle
Deklarationen derselben Quelldatei bleiben Teil der Vorschau. Das ist noch
keine isolierte Ressource und keine eigenständig deploybare Anwendung.
Unter `modules[].declarations` listet der Plan alle erkannten Deklarationen
der jeweils einbezogenen Quelldateien stabil sortiert auf. So ist sichtbar,
welche Funktionen, Typen, Records und Anwendungsressourcen durch die gemeinsame
Quelldatei zusätzlich enthalten sind.
`declaration_closure` trennt davon die ausgewählte Ressource und die über den
aktuellen statischen Wirkungsgraphen erreichbaren Deklarationen. `edges` zeigt
bekannte Sprach- und Typreferenzen, zum Beispiel API-Ein-/Ausgaben,
Funktionssignaturen, Records und Type Aliases. `configuration_edges` weist die
Datenbankkonfiguration separat aus. `database.configurations` nennt für jede
im Abschluss liegende Datenbankdeklaration ihren Namen, ihr Backend, den
logischen Datenbanknamen, die Quelldatei und `DATABASE_URL` als aktuelle
Laufzeitvariable. Der Plan kennzeichnet außerdem `connection_model` als
`single-project-wide-connection` und `supports_multiple_connections` als
`false`. Die Deklaration beschreibt also Schema-/Backend-Konfiguration; sie ist
noch keine separat adressierbare Datenbankschnittstelle. Zugangsdaten werden
nicht in den Plan aufgenommen.
`runtime_effects.effects` listet ausdrücklich deklarierte Funktions-Capabilities
in den einbezogenen Quellmodulen auf. `deployment_readiness.blockers` nennt
nicht deklarierte Datei-Umfänge, ausgehende Netzwerkverträge,
Prozessabhängigkeiten, Umgebungsvariablennamen und nicht aufgelöste statische
Verweise. `ready` bleibt `false`; implizite Ressourceneffekte sind noch nicht
vollständig modelliert, und der Plan beweist kein vollständiges Deployment.
Für einbezogene Auth-Module mit `reset_tokens` beschreibt
`external_service_contracts` öffentliche Basis-URL, SMTP-Einstellungen,
Schlüssel der verschlüsselten Outbox, Geheimnis-Variablen und die Zustellung
„at least once“. Docker-Bundles zeigen die passenden Felder in `.env.example`
mit dem bestehenden Standard `implicit_tls`/465; Zugangsdaten werden nie
kopiert. Der Dienstvertrag vervollständigt weder Quell- noch Deploymentgraph.
`additional_declarations_in_included_source_files`
führt Deklarationen auf, die wegen der einbezogenen Dateien zusätzlich
auftauchen. Das ist nur eine Analyse des bekannten Graphen: `complete` bleibt
`false`, und nicht erkannte Abhängigkeiten können fehlen.

Der JSON-Plan folgt schreibgeschützt und deterministisch den expliziten
Importen sowie den Verweisen, die der statische Wirkungsgraph derzeit erkennt.
Dazu gehören bekannte Verweise von Seiten zu Views und Komponenten, von
Page-SQL und Formular-/CRUD-Aktions-SQL zu Tabellen, von API-Handlern zu
Funktionen, benannte Funktionsaufrufe in Formular-/CRUD-Aktionen zu ihren
Funktionen, Typreferenzen aus API-Feldern, Funktionssignaturen, Records,
Type Aliases, Tabellenspalten sowie typisierten Feldern von Formularen,
Formularaktionen und CRUD-Aktionen, Typannotationen in Funktionskörpern,
Record-Literalen und SQL-Ergebnistypen, von geschützten Ressourcen zur
Authentifizierung, von
Authentifizierung zu Tabellen, erkannte Tabellenrelationen und
Datenbankkonfiguration. `database.configurations` macht die im Abschluss
liegenden Deklarationen mit Backend, logischem Datenbanknamen, Quelldatei und
der aktuellen Laufzeitvariable `DATABASE_URL` sichtbar. Der Plan weist
`connection_model: "single-project-wide-connection"` und
`supports_multiple_connections: false` aus: Mehrere unabhängig konfigurierbare
Verbindungen oder eine benannte DB-Schnittstelle werden noch nicht unterstützt.
Zugangsdaten erscheinen nicht im Plan.
Nicht auflösbare Verweise erscheinen in `unresolved_references`. Dynamische
oder nicht modellierte Abhängigkeiten, Assets, Laufzeitkonfiguration, externe
Dienste und Docker-Artefakte sind nicht enthalten. Aktionsaufrufe werden
statisch anhand der aufgelösten Funktionsnamen erkannt; das macht die Analyse
noch nicht vollständig. `complete_deployment` bleibt ausdrücklich `false`; der
Befehl exportiert oder startet keine Anwendung.

SQL-Tabellenkanten tragen zusätzlich `access`: `read`, `write`, `read_write`
oder `unknown`. Der Beobachter erkennt bekannte, nicht qualifizierte
Tabellennamen in den einfachen `SELECT`-, `INSERT`-, `UPDATE`- und `DELETE`-
Formen, die der Analyzer abdeckt. Bei `INSERT ... SELECT` wird die Zieltabelle
als Schreibzugriff und die gelesene Quelle als Lesezugriff ausgewiesen.
Komplexe verbundene `UPDATE`-/`DELETE`-Formen werden konservativ als
`unknown` markiert. Nicht erkannte SQL-Formen oder nicht zugeordnete Tabellen
können in der Analyse fehlen. Diese Angaben beschreiben beobachtete
Abhängigkeiten; sie sind weder Berechtigungsprüfung noch Durchsetzung von
Schemaeigentum und machen den Graphen nicht vollständig. `schema_ownership`
weist die Quelldatei einer Tabellendeklaration als
`inferred_owner_module` aus; `enforced` und `ownership_enforced` stehen
ausdrücklich auf `false`.

Der Compiler prüft jetzt zusätzlich erkannte modulübergreifende Tabellenkanten:
Das Modul, das eine Tabelle über SQL, CRUD, Formulare, Tableviews,
Authentifizierung oder eine Tabellenbeziehung verwendet, muss das Modul mit
der Tabellendeklaration direkt oder transitiv importieren. Andernfalls meldet
`zelyra check` `E-MOD-019`; bei erkanntem SQL-Zugriff nennt die Diagnose auch
`read`-/`write`-Modi. Auch Tabellen in `main.zyl` sind importierten Modulen
nicht implizit zugänglich: Ein Kind kann den Einstieg nicht zurückimportieren.
Lege gemeinsam genutzte Tabellen deshalb in ein eigenes Schema-Modul und
importiere es sowohl im Einstieg als auch in jedem Verbraucher.

Ein Verbraucher von Datenbankfunktionen braucht außerdem eine Importkante zum
Datenbankmodul selbst. Module mit SQL-Funktionen (`uses Database`), Daten-
Pages, Tableviews, Formularen, CRUD oder Authentifizierung müssen den Provider
direkt oder transitiv importieren. Ein Import der Datenbankkonfiguration nur in
`main.zyl` wird nicht seitwärts an andere Module vererbt. Andernfalls meldet
`zelyra check` `E-MOD-022`. Liegt die `database`-Deklaration noch im Einstieg,
verschiebe sie in eine eigene Datei wie `src/database.zyl` und importiere diese
aus den Modulen, die Datenbankfunktionen verwenden. Das ist eine geprüfte
Quellcodeabhängigkeit; es erzeugt weder mehrere Laufzeitverbindungen noch
MariaDB-Berechtigungen.

Die unveröffentlichte 0.4-Implementierung prüft außerdem modulbezogene
Tabellenfreigaben. Der Eigentümer kann sie direkt in der Tabellendeklaration
festlegen:

~~~zelyra
table customers {
    id: Id primary auto
    name: String(100) required
    access {
        read: ["src/reports.zyl"]
        write: ["src/importer.zyl"]
        read_write: ["src/customer_admin.zyl"]
    }
}
~~~

Die Pfade sind exakt projekt-relative `.zyl`-Dateipfade. Ein Verbraucher muss
weiterhin das Eigentümermodul importieren und zusätzlich in der passenden
Liste stehen. `read` erlaubt erkannte Lesezugriffe, `write` erkannte
Schreibzugriffe, `read_write` beides. Ein kombiniertes SQL-Muster braucht
entweder `read_write` oder getrennte Lese- und Schreibfreigaben. Bei
`unknown`-SQL-Zugriffen reicht nur `read_write`; CRUD-, Formular- und
Authentifizierungsressourcen benötigen ebenfalls `read_write`. Ohne passende
Freigabe meldet `zelyra check` `E-MOD-021`; eine fehlende Importkante bleibt
weiterhin `E-MOD-019`.

Die Freigaben sind Compiler-Verträge für erkannte Abhängigkeiten, keine
MariaDB-`GRANT`-Anweisungen und keine Datenbankkonto-Sicherheit. Unbekannte
SQL-Formen können der Analyse entgehen; Rechte für Schemaänderungen werden
nicht geprüft, `schema_ownership.enforced` bleibt `false`. Ein nicht exakt
passender Grant-Pfad erteilt keinen Zugriff; der Fehler erscheint beim
Verbraucher. Der maschinenlesbare Modulplan gibt erkannte Grants und die
unvollständige Analyse unter `table_access_contract` aus.

Als nächste experimentelle Stufe gibt es `zelyra module bundle`:

Die [Docker-Abnahme für Rechnungen und Inventar](../../module-docker-acceptance.de.md)
zeigt den wiederholbaren Test der Gesamtanwendung und der beiden Einzelexporte,
einschließlich eigener Zugangsdaten und negativer Berechtigungsprüfungen.

~~~sh
zelyra module bundle examples/modules/main.zyl 'page:/invoices' --output ../invoices-bundle --dry-run
~~~

Mit `--dry-run` gibt der Befehl einen JSON-Plan mit jedem relativen Dateinamen
und seinem vollständigen Zielpfad aus. Der Plan weist außerdem aus, dass keine
Geheimnisse übernommen und keine Zieldateien veröffentlicht werden. Das
gewünschte Ausgabeverzeichnis wird nicht angelegt; intern erzeugt Zelyra ein
temporäres Paket, prüft es mit `zelyra check` und entfernt es anschließend.
Der Plan bleibt experimentell und kennzeichnet weiterhin
`source_closure_complete: false` sowie `complete_deployment: false`.

Für den eigentlichen Export lässt du `--dry-run` weg:

~~~sh
zelyra module bundle examples/modules/main.zyl 'page:/invoices' --output ../invoices-bundle
~~~

Der Befehl materialisiert die im Plan bekannten Quelldateien in einem neuen
Zielordner, erzeugt einen Einstieg, prüft das Ergebnis mit `zelyra check` und
veröffentlicht den Ordner erst nach erfolgreicher Prüfung. Er kopiert außerdem
`zelyra.toml`, `zelyra.theme.css` und JSON-Sprachkataloge, sofern vorhanden.
Ein vorhandenes Ziel, ein nicht auflösbarer Verweis oder eine Abhängigkeit vom
ursprünglichen Einstieg wird abgelehnt. Die ausgewählte Quelldatei bleibt
vollständig enthalten. Enthält der geprüfte Projektgraph mehrere
Datenbankdeklarationen, stoppt die Planung mit `E-DB-001`, bevor Dateien
geschrieben werden. Derzeit ist nur eine projektweite `DATABASE_URL`-Verbindung
implementiert.

Das Ergebnis ist ausdrücklich nur ein experimentelles Quellpaket:
`zelyra.bundle.json` setzt sowohl `source_closure_complete` als auch
`complete_deployment` auf `false`. Es enthält weder Docker-/Compose-Dateien,
Compiler-Binärdatei, Datenbankdienst noch `.env`; Zugangsdaten werden nicht
kopiert. Für Betrieb und Datenbank sind weiterhin eine passende Zelyra-Version,
eine externe Laufzeitkonfiguration und gegebenenfalls eine MariaDB nötig. Ein
eigenständiger Dockerexport ist damit noch nicht erreicht.

Eine experimentelle Docker-Paketierung lässt sich zusätzlich erzeugen:

~~~sh
zelyra module bundle examples/modules/main.zyl 'page:/invoices' --output ../invoices-docker --docker --compiler-ref 0123456789012345678901234567890123456789
~~~

`--compiler-ref` muss ein vollständiger 40-stelliger Commit sein, der die
benötigte Modulsyntax enthält. Der erzeugte Docker-Build lädt genau diesen
Compiler-Commit aus dem öffentlichen Zelyra-Repository und baut daraus das
Runtime-CLI. Die Compose-Datei startet die extrahierte Anwendung; eine
MariaDB wird ausdrücklich nicht mitgeliefert. `.env.example` enthält nur
Platzhalter und eine leere `DATABASE_URL`. Kopiere sie nach `.env` und
konfiguriere dort die Datenbankverbindung, falls die Anwendung sie benötigt.
Die Zugangsdaten werden weder kopiert noch in das Image gebaut. Benötigt das
ausgewählte Modul MariaDB, installiert das Image den aktuell benötigten
`mariadb-client`; die Datenbank selbst bleibt extern. Die tatsächlich
eingeplanten Laufzeitpakete stehen in `docker.runtime_packages` im Manifest.
Jedes exportierte Compose-Paket hat sein eigenes `.env.example` und damit nach
dem lokalen Kopieren eine unabhängig konfigurierbare `DATABASE_URL`. Das ist
eine Verbindung pro laufender Anwendungseinheit, kein Datenbankmodul mit
mehreren benannten Verbindungen innerhalb desselben Prozesses. Das Manifest
weist diesen begrenzten Umfang unter
`docker.database_connection_scope: "per_exported_compose_project"` aus.

Auch diese Docker-Paketierung ist noch kein vollständiger Modul-Export:
`source_closure_complete` und `complete_deployment` bleiben `false`, weil der
statische Abhängigkeitsgraph noch nicht alle Laufzeit- und Asset-Abhängigkeiten
beweist. Ein erfolgreicher `zelyra check` ersetzt diesen Nachweis nicht.
Der Docker-End-to-End-Test exportiert zwei CRUD-Ressourcen aus getrennten
Quelldateien in separate Compose-Projekte. Beide Pakete enthalten automatisch
das Datenbank-Konfigurationsmodul, erhalten eigene `DATABASE_URL`-Werte und
lesen jeweils ihren Testdatensatz mit einem MariaDB-Benutzer, der nur auf die
jeweilige Tabelle lesend zugreifen darf. Das ist ein begrenzter CRUD- und
Datenbanknachweis für diese Testanwendung, nicht für vollständige Fachmodule
oder beliebige Projekte.

Importierte Oberflächenbausteine können von einer Seite der Einstiegsdatei
verwendet werden. Der Alias bindet die Datei ein; View- und Komponentennamen
werden in der HTML-Deklaration derzeit nicht mit `ui::` qualifiziert:

~~~zelyra
// src/ui.zyl
component Banner {
    props {
        title: String
    }
    html {
        <header><strong>{title}</strong></header>
    }
}

view Shell {
    html {
        <html><body><Banner title="Rechnungen" /><main><slot /></main></body></html>
    }
}
~~~

~~~zelyra
// main.zyl
import "src/ui.zyl" as ui

page "/" {
    view: Shell
    html {
        <p>Die Seite verwendet importierte Bausteine.</p>
    }
}
~~~

Mit `zelyra serve main.zyl` wird die Seite aus dem verknüpften Projektgraphen
gerendert. Die vollständige Beispieldatei mit Logik- und Datenbankmodulen liegt
unter `examples/modules/`.
Sie enthält außerdem `src/invoice_admin.zyl` mit importiertem Formular und
CRUD. Prüfen und ausführen kannst du sie mit `zelyra check examples/modules/main.zyl`
und `zelyra run examples/modules/main.zyl` (Ausgabe: `25`). Der experimentelle
Modulplan für die Verwaltung lautet:

~~~sh
zelyra module plan examples/modules/main.zyl src/invoice_admin.zyl
~~~

Er zeigt die Form-/CRUD-Abhängigkeit zur Rechnungstabelle und die separate
Datenbank-Konfigurationsdatei. Er ist weiterhin kein Docker-Export.

Ein Modul kann einen fachlichen Record exportieren, den ein anderes Modul in
einer Funktionssignatur verwendet:

~~~zelyra
pub struct Money {
    cents: Int
}
~~~

~~~zelyra
import "src/money.zyl" as money

pub fn total() -> money::Money {
    return money::Money { cents: 2500 }
}
~~~

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Eine saubere Hauptdatei**
```zelyra
fn main() {
    print("Zelyra Aufgaben-System bereit.")
}
```

**Beispiel 2: Trennung von Logikfunktionen**
```zelyra
fn format_system_status(status: String) -> String {
    return "[STATUS] " + status
}

fn main() {
    print(format_system_status("Datenbank verbunden"))
}
```

**Beispiel 3: Deklaration von Capabilities in der Projektdatei**
In `zelyra.toml` legst du fest, welche Systemzugriffe das Projekt überhaupt anfordern darf. Greift dein Code auf die Datenbank zu, muss `database = true` aktiviert sein.

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Modulimporte mit dem veröffentlichten Zelyra-0.3.0-Binary verwenden.
  *Ursache:* `import` gehört nicht zu diesem Release. Der Entwicklungszweig
  unterstützt experimentell mehrere Deklarationstypen, aber noch kein
  vollständiges, stabiles Modul- und Paketmodell.
- **Fehler:** `zelyra.toml` löschen oder im falschen Verzeichnis ausführen.
  *Ursache:* `zelyra run` sucht im aktuellen Verzeichnis nach der Konfiguration.

### 6. Merksätze
1. `zelyra.toml` steuert Metadaten und Sicherheitsrichtlinien des Projekts.
2. Trenne Datenmodell (`table`), Geschäftslogik (`fn`) und Darstellung sauber voneinander.
3. Ordnung im Projektverzeichnis schützt vor Flüchtigkeitsfehlern im Team.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Erstelle mit `zelyra new aufgaben_app` eine neue Projektstruktur und untersuche die erzeugten Dateien.
- **Stufe 2 (Mittel):** Konfiguriere in `zelyra.toml` eine Beschreibung und die Versionsnummer `0.3.1`.
- **Stufe 3 (Anspruchsvoll):** Schreibe ein Programm mit drei separaten Funktionen für Initialisierung, Verarbeitung und Ausgabe.

### 8. Praxisaufgabe: Projektstruktur für die Aufgabenverwaltung
Lege die Struktur fest mit folgendem Inhalt in `main.zyl`:
```zelyra
table aufgaben {
    id: Id primary auto
    name: String required
    erledigt: Bool
}

fn starte_system() {
    print("========================================")
    print("   AUFGABEN-MANAGER ERFOLGREICH GESTARTET")
    print("========================================")
}

fn main() {
    starte_system()
}
```

### 9. Zusammenfassung
- `zelyra.toml` steuert Metadaten und Capability-Sicherheitsfreigaben des Projekts.
- Release 0.3.0 prüft die ausdrücklich angegebene Quelldatei; der Entwicklungszweig ergänzt experimentelle Funktionsmodule.
- Eine modulare Denkweise erleichtert Erweiterungen und Teamarbeit.

### 10. Kontrollfragen zur Selbstprüfung
1. Welche Datei enthält die Metadaten eines Zelyra-Projekts?
2. Warum ist die Trennung von Datenmodell und Ausführungslogik sinnvoll?
3. Welche Befehle folgen derzeit den experimentellen Modul-Imports im Entwicklungszweig?

# TEIL IV – SICHERHEIT UND FEHLERBEHANDLUNG

---

## Kapitel 16: Fehlerarten und ihre Ursachen

### 1. Was lerne ich in diesem Kapitel?
In diesem Kapitel lernst du:
- Welche vier Hauptarten von Fehlern es beim Programmieren gibt.
- Wie sich Syntaxfehler, Typfehler, Logikfehler und Laufzeitfehler unterscheiden.
- Wie Zelyras Compiler dich durch präzise Fehlermeldungen mit Code-Ort und Hinweisen leitet.
- Warum frühe Fehlererkennung beim Kompilieren bares Geld spart.

### 2. Warum ist das Thema wichtig?
Fehler sind beim Programmieren völlig normal. Selbst erfahrene Programmierer machen dutzende Fehler pro Tag. Der Unterschied zwischen frustrierenden und erfolgreichen Entwicklern liegt nicht darin, keine Fehler zu machen, sondern darin, Fehlermeldungen lesen und verstehen zu können. Zelyra wurde so entworfen, dass möglichst viele Fehler bereits beim Kompilieren (`zelyra check`) abgefangen werden, bevor die Anwendung den ersten Nutzer erreicht.

### 3. Verständliche Erklärung
Wir unterscheiden vier Kategorien:
1. **Syntaxfehler (`E-LEX-*`, `E-PARSE-*`):** Du hast die Grammatik der Sprache verletzt – wie ein Rechtschreibfehler in einem Diktat (z. B. eine geschlossene Klammer vergessen).
2. **Typfehler (`E-TYPE-*`):** Die Grammatik stimmt, aber die Bedeutung passt nicht zusammen (z. B. Text zu einer Zahl addieren).
3. **Vertrags- und Berechtigungsfehler (`E-CONTRACT-*`, `E-CAP-*`):** Eine vereinbarte Vorbedingung wurde verletzt oder eine Funktion versucht, ohne Erlaubnis auf das Dateisystem zuzugreifen.
4. **Logikfehler:** Das Programm läuft fehlerfrei durch, tut aber nicht das, was du beabsichtigt hast (z. B. Rabatt addiert statt abgezogen).

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Typischer Syntaxfehler (und wie man ihn liest)**
Wenn du in Zelyra eine Klammer vergisst:
```zelyra
// Syntaktisch korrekt:
fn korrekte_klammern() {
    print("Alle Klammern sind geschlossen.")
}

fn main() {
    korrekte_klammern()
}
```

**Beispiel 2: Berechtigungsfehler (Capability-Prüfung)**
Versucht eine Funktion ohne deklarierte Rechte auf Systemressourcen zuzugreifen, stoppt Zelyra sofort:
```zelyra
fn lese_datei(pfad: String) -> String
    uses FileSystem
{
    return read_text(pfad)
}

fn main() {
    print("Dateizugriff sauber deklariert.")
}
```

**Beispiel 3: Logikfehler durch Verträge entlarven**
```zelyra
fn berechne_rabattpreis(original: Int, rabatt: Int) -> Int
    requires { original >= 0 && rabatt >= 0 && rabatt <= original }
    ensures { result <= original }
{
    return original - rabatt
}

fn main() {
    preis = berechne_rabattpreis(100, 20)
    print(preis)
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Fehlermeldungen wegklicken, ohne die Zeilennummer zu beachten.
  *Ursache:* Zelyra gibt immer die exakte Zeile und Spalte des Fehlers an.
- **Fehler:** Ein `+` zwischen Text und Zahl verwenden.
  *Ursache:* Zelyra erzwingt Typsicherheit. Formatiere Werte als String oder gib sie separat aus.

### 6. Merksätze
1. Ein Compilerfehler ist kein Scheitern, sondern ein wertvoller Hinweis.
2. Je früher ein Fehler gefunden wird (Compile-Zeit statt Laufzeit), desto sicherer ist die Software.
3. Verträge (`requires`, `ensures`) verwandeln tückische Logikfehler in sofort sichtbare Vertragsbrüche.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Provoziere absichtlich einen Syntaxfehler (z. B. Semikolon oder Klammer weglassen) und beobachte die Meldung von `zelyra check`.
- **Stufe 2 (Mittel):** Erstelle eine Funktion mit einem Typfehler und korrigiere sie nach dem Compiler-Hinweis.
- **Stufe 3 (Anspruchsvoll):** Schreibe eine Funktion zur Altersfreigabe mit Verträgen, die ungültige Alterseingaben (z. B. negative Werte) sofort abfangen.

### 8. Praxisaufgabe: Fehlertolerante Aufgabendauer-Berechnung
Schreibe eine Funktion für die Aufgabenverwaltung, die verhindert, dass negative Stunden erfasst werden:
```zelyra
fn erfasse_stunden(bisherige_stunden: Int, neue_stunden: Int) -> Int
    requires { bisherige_stunden >= 0 && neue_stunden >= 0 }
    ensures { result >= bisherige_stunden }
{
    return bisherige_stunden + neue_stunden
}

fn main() {
    gesamt = erfasse_stunden(5, 3)
    print(gesamt)
}
```

### 9. Zusammenfassung
- Zelyra unterscheidet strikt zwischen Syntax-, Typ-, Berechtigungs- und Logikfehlern.
- Durch statische Prüfung und Verträge werden die meisten Fehler vor dem Einsatz aufgedeckt.

### 10. Kontrollfragen zur Selbstprüfung
1. Welche Fehlermeldung (`E-...`) erzeugt ein vergessenes Schlüsselwort?
2. Warum kann ein Programm trotz fehlerfreier Kompilierung falsch rechnen?
3. Wie helfen Verträge beim Aufspüren logischer Denkfehler?

---

## Kapitel 17: Fehler als Werte – Das Result-Muster

### 1. Was lerne ich in diesem Kapitel?
- Was das `Result`-Muster ist und warum Zelyra keine unkontrollierten Exceptions (Ausnahmen) nutzt.
- Die beiden Zustände: `Ok(wert)` für Erfolg und `Err(meldung)` für Fehler.
- Wie du Ergebnisse mit `match` sicher zerlegst.
- Warum Fehler als Werte deinen Code transparent und absturzsicher machen.

### 2. Warum ist das Thema wichtig?
In vielen älteren Sprachen (wie Java oder Python) wirft eine Funktion im Fehlerfall eine „Exception“. Wenn irgendwo im Code eine solche Ausnahme vergessen wird, stürzt die gesamte Webanwendung mit einem Serverfehler ab. In Zelyra gibt es keine unkontrollierten Abstürze: Wenn eine Operation fehlschlagen kann (z. B. Datei nicht gefunden oder ungültige ID), gibt sie zwingend ein `Result<T, E>` zurück. Der Compiler zwingt dich, beide Fälle zu behandeln.

### 3. Verständliche Erklärung
Stelle dir ein Postpaket vor:
- Wenn der Zusteller das Paket erfolgreich abgibt, öffnest du es und findest den gewünschten Inhalt: `Ok(inhalt)`.
- Wenn die Adresse nicht existiert, kommt ein Rücksendebeleg mit Begründung: `Err("Adresse unbekannt")`.

Ein Paket kann niemals „explodieren“ – du musst es einfach nur annehmen und nachsehen:
```zelyra
fn dividiere_sicher(a: Int, b: Int) -> Result<Int, String> {
    if b == 0 {
        return Err("Division durch 0 nicht erlaubt")
    }
    return Ok(a / b)
}

fn main() {
    print("Sichere Division definiert.")
}
```

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Eine Funktion mit Result definieren**
```zelyra
fn pruefe_prioritaet(stufe: Int) -> Result<Int, String> {
    if stufe < 1 {
        return Err("Prioritaet zu niedrig (mindestens 1)")
    }
    if stufe > 3 {
        return Err("Prioritaet zu hoch (maximal 3)")
    }
    return Ok(stufe)
}

fn main() {
    res = pruefe_prioritaet(2)
    match res {
        Ok(stufe) => {
            print("Gueltige Prioritaet:")
            print(stufe)
        }
        Err(fehler) => {
            print(fehler)
        }
    }
}
```

**Beispiel 2: Fehlerfall behandeln**
```zelyra
fn hole_kontostand(pin: Int) -> Result<Int, String> {
    if pin != 1234 {
        return Err("Falsche PIN!")
    }
    return Ok(500)
}

fn main() {
    versuch = hole_kontostand(9999)
    match versuch {
        Ok(betrag) => {
            print(betrag)
        }
        Err(meldung) => {
            print("Abgewiesen: " + meldung)
        }
    }
}
```

**Beispiel 3: Sichere Werteumwandlung**
```zelyra
fn pruefe_titel_laenge(titel: String) -> Result<String, String> {
    if titel == "" {
        return Err("Aufgabentitel darf nicht leer sein.")
    }
    return Ok(titel)
}

fn main() {
    ergebnis = pruefe_titel_laenge("Projektbericht")
    match ergebnis {
        Ok(t) => {
            print("Gueltiger Titel: " + t)
        }
        Err(e) => {
            print("Fehler: " + e)
        }
    }
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Versuchen, direkt auf den inneren Wert zuzugreifen, ohne `match` zu verwenden.
  *Ursache:* Ein `Result<T, E>` ist eine Hülle. Du musst sie mit `match` öffnen.
- **Fehler:** Einen der beiden Zweige (`Ok` oder `Err`) im `match` vergessen.
  *Ursache:* Zelyra verlangt vollständige Musterabdeckung.

### 6. Merksätze
1. `Result<T, E>` macht Fehler zu regulären Rückgabewerten.
2. `Ok(v)` repräsentiert Erfolg, `Err(e)` den begründeten Fehlschlag.
3. Mit `match` müssen immer beide Ausgänge behandelt werden – das schützt vor Abstürzen.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Schreibe eine Funktion `pruefe_gerade(zahl: Int) -> Result<Int, String>`, die `Ok(zahl)` liefert, wenn sie durch 2 teilbar ist, sonst `Err("Ungerade")`.
- **Stufe 2 (Mittel):** Schreibe eine Funktion `validiere_benutzername(name: String) -> Result<String, String>`, die leere Namen oder `"admin"` ablehnt.
- **Stufe 3 (Anspruchsvoll):** Implementiere eine Rechenfunktion mit Fehlerprüfung, die zwei Zahlen dividiert und das Ergebnis bei Erfolg verdoppelt zurückgibt.

### 8. Praxisaufgabe: Validierung beim Anlegen einer neuen Aufgabe
Erstelle eine sichere Validierungsfunktion für neue Aufgaben:
```zelyra
fn erstelle_aufgabe_geprueft(name: String, prioritaet: Int) -> Result<String, String> {
    if name == "" {
        return Err("Name darf nicht leer sein!")
    }
    if prioritaet < 1 {
        return Err("Prioritaet muss mindestens 1 sein!")
    }
    return Ok("Aufgabe [" + name + "] erfolgreich angelegt.")
}

fn main() {
    treffer1 = erstelle_aufgabe_geprueft("Dokumentation fertigstellen", 1)
    match treffer1 {
        Ok(msg) => {
            print(msg)
        }
        Err(err) => {
            print("Fehler: " + err)
        }
    }

    treffer2 = erstelle_aufgabe_geprueft("", 0)
    match treffer2 {
        Ok(msg) => {
            print(msg)
        }
        Err(err) => {
            print("Fehler: " + err)
        }
    }
}
```

### 9. Zusammenfassung
- Das `Result`-Muster ersetzt unkontrollierte Ausnahmen durch typisierte Werte.
- Zelyra garantiert, dass kein Fehler unbehandelt im Programm übersehen wird.

### 10. Kontrollfragen zur Selbstprüfung
1. Wofür steht `T` und wofür steht `E` im Typ `Result<T, E>`?
2. Warum führt ein unbehandelter Fehler in Zelyra nicht zum plötzlichen Programmabsturz?
3. Wie verarbeitet man den Inhalt eines `Result` sicher?

---

## Kapitel 18: Das Nichts existiert nicht – Der sichere Umgang mit Option

### 1. Was lerne ich in diesem Kapitel?
- Warum der Wert `null` in der Informatik als „Milliarden-Dollar-Fehler“ bezeichnet wird.
- Wie Zelyra `null` vollständig eliminiert und durch den sicheren Typ `Option<T>` (Kurzform `T?`) ersetzt.
- Wie man Werte mit `Some(wert)` verpackt und das Nichtvorhandensein mit `None` signalisiert.
- Wie Standard-Listenoperationen wie `first` und `last` den Typ `Option` nutzen.

### 2. Warum ist das Thema wichtig?
In Sprachen wie JavaScript, Java, PHP oder C existiert `null`. Wenn ein Programm versucht, eine Methode auf einem `null`-Wert aufzurufen, stürzt es augenblicklich mit einer gefürchteten `NullPointerException` oder `Cannot read properties of null` ab. Zelyra besitzt schlicht kein `null`. Jeder Wert ist garantiert vorhanden. Wenn etwas fehlen kann, muss es ausdrücklich als `Option` deklariert werden.

### 3. Verständliche Erklärung
Stell dir eine Schachtel vor:
- Die Schachtel ist entweder mit einem Geschenk gefüllt: `Some("Smartphone")`.
- Oder die Schachtel ist leer: `None`.

Du kannst nicht versehentlich in ein „Nichts“ greifen, weil du die Schachtel erst mit `match` öffnen musst:
```zelyra
fn finde_aufgabe_nach_id(id: Int) -> Option<String> {
    if id == 42 {
        return Some("Server aufsetzen")
    }
    return None
}

fn main() {
    print("Aufgabensuche definiert.")
}
```

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Option erzeugen und auswerten**
```zelyra
fn main() {
    treffer: Option<String> = Some("Zelyra 0.1 Handbuch")
    match treffer {
        Some(titel) => {
            print("Gefunden: " + titel)
        }
        None => {
            print("Kein Treffer vorhanden")
        }
    }
}
```

**Beispiel 2: first() und last() auf Listen**
Greifst du auf eine leere Liste zu, stürzt Zelyra nicht ab – es liefert `None`:
```zelyra
fn main() {
    meine_liste: Int[] = [100, 200, 300]
    erstes_element = first(meine_liste)
    match erstes_element {
        Some(wert) => {
            print("Erster Wert:")
            print(wert)
        }
        None => {
            print("Die Liste ist leer!")
        }
    }
}
```

**Beispiel 3: Standardwert bereitstellen mit Option**
```zelyra
fn aufgabe_titel_oder_standard(opt_titel: Option<String>) -> String {
    match opt_titel {
        Some(t) => {
            return t
        }
        None => {
            return "Ohne Titel"
        }
    }
}

fn main() {
    print(aufgabe_titel_oder_standard(Some("Wichtiges Meeting")))
    print(aufgabe_titel_oder_standard(None))
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Denken, ein `Option<String>` sei direkt ein `String`.
  *Ursache:* Eine Schachtel ist nicht das Geschenk. Erst mit `match` entpackst du den Inhalt.
- **Fehler:** Versuchen, `None` einer normalen `String`-Variable zuzuweisen.
  *Ursache:* Reguläre Variablen sind garantiert niemals leer.

### 6. Merksätze
1. In Zelyra gibt es kein `null` und keinen `NullPointerException`-Absturz.
2. Wenn ein Wert fehlen kann, heißt der Typ `Option<T>` oder `T?`.
3. `Some(x)` verpackt den Wert, `None` signalisiert das Fehlen.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Schreibe eine Funktion `finde_partner(name: String) -> Option<String>`, die bei `"Romeo"` `Some("Julia")` liefert, sonst `None`.
- **Stufe 2 (Mittel):** Untersuche das letzte Element eines Zahlen-Arrays mit `last()` und gib seinen Wert oder eine Warnung aus.
- **Stufe 3 (Anspruchsvoll):** Schreibe eine Suchfunktion, die eine Liste von Aufgaben-IDs durchsucht und die Position (Index) als `Option<Int>` zurückgibt.

### 8. Praxisaufgabe: Aufgabendetails sicher abfragen
Implementiere die Nachschlagefunktion für unsere Aufgabenverwaltung:
```zelyra
fn suche_aufgabe_beschreibung(id: Int) -> Option<String> {
    if id == 1 {
        return Some("Datenbankschema fuer Zelyra anlegen")
    }
    if id == 2 {
        return Some("Weboberflaeche gestalten")
    }
    return None
}

fn main() {
    suche1 = suche_aufgabe_beschreibung(1)
    match suche1 {
        Some(text) => {
            print("Aufgabe 1: " + text)
        }
        None => {
            print("Aufgabe 1 nicht gefunden!")
        }
    }

    suche99 = suche_aufgabe_beschreibung(99)
    match suche99 {
        Some(text) => {
            print("Aufgabe 99: " + text)
        }
        None => {
            print("Aufgabe 99 nicht gefunden!")
        }
    }
}
```

### 9. Zusammenfassung
- `Option<T>` schützt deine Anwendung vor den verheerenden Folgen unvorhergesehener Leerwerte.
- Zelyra garantiert zur Compile-Zeit, dass jeder mögliche `None`-Fall behandelt wird.

### 10. Kontrollfragen zur Selbstprüfung
1. Warum gibt es in Zelyra kein `null`?
2. Was ist der Unterschied zwischen `String` und `Option<String>`?
3. Welche zwei Muster werden in einem `match` über eine `Option` immer abgefragt?

---

## Kapitel 19: Tests und Qualitätssicherung

### 1. Was lerne ich in diesem Kapitel?
- Warum automatisierte Prüfungen das Rückgrat moderner, langlebiger Software sind.
- Wie Zelyras Verifikations-Befehl `zelyra verify` Verträge formal analysiert.
- Wie man eigene Test- und Prüffunktionen strukturiert.
- Was die Philosophie von *Test-Driven Development* (TDD) bedeutet.
- Der Ausblick auf das zukünftige integrierte Testmodul.

### 2. Warum ist das Thema wichtig?
Manuelle Tests (Klicken im Browser oder wiederholtes manuelles Aufrufen) sind mühsam, fehleranfällig und unvollständig. Sobald eine Software komplexer wird, führt jede kleine Änderung an einer Stelle unweigerlich zu neuen Fehlern an einer anderen Stelle („Regressionen“). Automatisierte Tests stellen sicher, dass alle bereits gebauten Funktionen auch nach Wochen und Monaten noch exakt wie vereinbart arbeiten.

### 3. Verständliche Erklärung
Testen in Zelyra stützt sich auf zwei kraftvolle Säulen:
1. **Formale Vertragsverifikation mit `zelyra verify`:**
   Der Compiler prüft, ob die Vor- und Nachbedingungen (`requires`, `ensures`) deiner Funktionen mathematisch und logisch haltbar sind.
2. **Prüffunktionen mit Erwartungsabgleich:**
   Du schreibst kleine Prüffunktionen, die bestimmte Eingaben in deine Funktionen schicken und das tatsächliche Ergebnis mit dem erwarteten Ergebnis vergleichen.

*Hinweis zur Roadmap:* Das integrierte CLI-Testframework `zelyra test` befindet sich laut Roadmap in Phase 11/12. In Zelyra 0.1 erfolgt die Qualitätssicherung über `zelyra check`, `zelyra verify` sowie gezielte Test-Hauptroutinen.
`// [Platzhalter: Zelyra Test Framework - in 0.1 ueber verify und Test-Runner realisiert; siehe Roadmap Phase 11]`

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Eine einfache Assert-Prüffunktion**
```zelyra
fn pruefe(test_name: String, bedingung: Bool) {
    if bedingung {
        print("[PASS] " + test_name)
    } else {
        print("[FAIL] " + test_name)
    }
}

fn verdopple(x: Int) -> Int {
    return x * 2
}

fn main() {
    pruefe("Verdopple 5 ergibt 10", verdopple(5) == 10)
    pruefe("Verdopple 0 ergibt 0", verdopple(0) == 0)
}
```

**Beispiel 2: Testen von Option-Rückgaben**
```zelyra
fn ist_volljaehrig(alter: Int) -> Option<Bool> {
    if alter < 0 {
        return None
    }
    return Some(alter >= 18)
}

fn main() {
    test1 = ist_volljaehrig(20)
    match test1 {
        Some(ok) => {
            if ok {
                print("[PASS] 20 Jahre ist volljaehrig")
            } else {
                print("[FAIL] Unerwarteter Zustand")
            }
        }
        None => {
            print("[FAIL] Alter ungueltig")
        }
    }
}
```

**Beispiel 3: Verifikation mit Verträgen absichern**
```zelyra
fn berechne_ueberstunden(stunden: Int, regelarbeitszeit: Int) -> Int
    requires { stunden >= 0 && regelarbeitszeit >= 0 }
    ensures { result >= 0 }
{
    if stunden > regelarbeitszeit {
        return stunden - regelarbeitszeit
    }
    return 0
}

fn main() {
    print(berechne_ueberstunden(45, 40))
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Nur den „Gut-Fall“ testen und Randfälle (Grenzwerte, 0, leere Listen) ignorieren.
  *Ursache:* Die meisten Fehler treten an den Grenzen des Definitionsbereichs auf.
- **Fehler:** Tests nach einer Code-Änderung nicht erneut ausführen.
  *Ursache:* Gewöhne dir an, `zelyra check` und deinen Test-Runner nach jeder Anpassung zu starten.

### 6. Merksätze
1. Ungesteter Code ist kaputter Code, von dem du es nur noch nicht weißt.
2. `zelyra verify` prüft Funktionsverträge direkt auf Sprachebene.
3. Schreibe Tests, die Randbedingungen und Fehlerpfade gezielt herausfordern.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Schreibe drei Testfälle für eine Funktion `addiere(a: Int, b: Int) -> Int`.
- **Stufe 2 (Mittel):** Schreibe Testfälle für die Option-Suchfunktion aus Kapitel 18.
- **Stufe 3 (Anspruchsvoll):** Implementiere eine vollständige Test-Suite für eine Funktion, die prüft, ob ein Aufgabentitel den Qualitätsregeln entspricht (nicht leer, keine Sonderzeichen).

### 8. Praxisaufgabe: Test-Runner für die Aufgaben-Geschäftslogik
Baue eine kleine Test-Suite für die Kernlogik der Aufgabenverwaltung:
```zelyra
fn test_fall(beschreibung: String, ok: Bool) {
    if ok {
        print("OK: " + beschreibung)
    } else {
        print("FEHLER: " + beschreibung)
    }
}

fn filter_prioritaet(p: Int) -> Bool {
    return p == 1
}

fn main() {
    print("Starte Test-Suite: Aufgabenlogik")
    test_fall("Prioritaet 1 wird gefiltert", filter_prioritaet(1) == true)
    test_fall("Prioritaet 2 wird ignoriert", filter_prioritaet(2) == false)
    print("Test-Suite abgeschlossen.")
}
```

### 9. Zusammenfassung
- Automatisierte Prüfungen sichern langfristige Softwarequalität.
- `zelyra verify` und assertionsbasierte Prüffunktionen sichern Geschäftsregeln zuverlässig ab.

### 10. Kontrollfragen zur Selbstprüfung
1. Was versteht man unter einer „Regression“ in der Softwareentwicklung?
2. Welche Aufgabe übernimmt der CLI-Befehl `zelyra verify`?
3. Warum sind Grenzwerte (z. B. 0 oder maximale Kapazität) besonders testrelevant?

# TEIL V – PRAKTISCHE DATENVERARBEITUNG

---

## Kapitel 20: Arbeiten mit Dateien

### 1. Was lerne ich in diesem Kapitel?
In diesem Kapitel lernst du:
- Wie du mit Zelyra Textdateien erstellst, liest, auflistest und löschst.
- Warum Dateizugriffe in Zelyra zwingend die Fähigkeit (*Capability*) `uses FileSystem` verlangen.
- Die wichtigsten Bibliotheksfunktionen: `read_text`, `write_text`, `delete_file`, `list_dir`.
- Wie du Aufgabenlisten persistent als Datei auf der Festplatte speicherst.

### 2. Warum ist das Thema wichtig?
Variablen im Arbeitsspeicher gehen verloren, sobald ein Programm beendet wird oder der Rechner neu startet. Um Daten dauerhaft zu sichern – z. B. Exporte, Konfigurationsdateien oder Protokolle –, müssen sie auf die Festplatte geschrieben werden. Gleichzeitig stellen Dateizugriffe ein Sicherheitsrisiko dar. Zelyra schützt das System, indem Funktionen ihre Zugriffsrechte explizit deklarieren müssen.

### 3. Verständliche Erklärung
Stell dir das Dateisystem wie ein Archiv vor:
- Wenn du eine Akte ablegen willst, schreibst du Text hinein (`write_text`).
- Wenn du nachsehen willst, liest du die Akte (`read_text`).
- Du darfst das Archiv aber nur betreten, wenn du den Archivschlüssel besitzt: `uses FileSystem`.

```zelyra
fn speichere_notiz(pfad: String, inhalt: String)
    uses FileSystem
{
    write_text(pfad, inhalt)
}

fn main() uses FileSystem {
    speichere_notiz("notiz.txt", "Einkaufsliste: Milch, Brot")
    print("Notiz gespeichert.")
}
```

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Text schreiben und lesen**
```zelyra
fn datei_workflow() uses FileSystem {
    pfad = "aufgaben_export.txt"
    write_text(pfad, "Aufgabe 1: Zelyra lernen")
    text = read_text(pfad)
    print("Gelesener Inhalt: " + text)
}

fn main() uses FileSystem {
    datei_workflow()
}
```

**Beispiel 2: Datei aufräumen mit delete_file**
```zelyra
fn aufraeumen(pfad: String) uses FileSystem {
    delete_file(pfad)
    print("Datei geloescht.")
}

fn main() uses FileSystem {
    write_text("temp.txt", "Kurzlebig")
    aufraeumen("temp.txt")
}
```

**Beispiel 3: Verzeichnisinhalte auflisten**
```zelyra
fn zeige_dateien(ordner: String) uses FileSystem {
    dateien = list_dir(ordner)
    for datei in dateien {
        print("Gefundene Datei: " + datei)
    }
}

fn main() uses FileSystem {
    zeige_dateien(".")
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Aufruf von `read_text` oder `write_text` ohne Deklaration von `uses FileSystem`.
  *Ursache:* Zelyras Sicherheitssystem (`E-CAP-001`) verhindert jeden unberechtigten Zugriff auf die Festplatte.
- **Fehler:** Vergessen, dass auch `main()` die Berechtigung deklarieren muss, wenn sie aufrufende Funktionen ausführt.
  *Ursache:* Berechtigungen vererben sich entlang der Aufruf-Kette nach oben.

### 6. Merksätze
1. Jede Funktion, die Dateien berührt, muss `uses FileSystem` deklarieren.
2. `write_text` legt Dateien an oder überschreibt sie vollständig.
3. `read_text` liefert den gesamten Dateiinhalt als `String`.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Schreibe ein Programm, das eine Begrüßungsnachricht in `hallo.txt` schreibt.
- **Stufe 2 (Mittel):** Schreibe eine Funktion, die prüft, ob eine exportierte Datei existiert und deren Inhalt ausgibt.
- **Stufe 3 (Anspruchsvoll):** Implementiere eine einfache Log-Funktion, die Statusmeldungen zeilenweise aneinanderhängt und sichert.

### 8. Praxisaufgabe: Aufgabenliste in eine Textdatei exportieren
Speichere die offenen Aufgaben unserer Anwendung als Datei:
```zelyra
fn exportiere_aufgaben(pfad: String) uses FileSystem {
    inhalt = "[ ] Dokumentation fertigstellen\n[OK] Zelyra-Compiler installieren\n[ ] Backup konfigurieren"
    write_text(pfad, inhalt)
    print("Aufgaben erfolgreich nach " + pfad + " exportiert.")
}

fn main() uses FileSystem {
    exportiere_aufgaben("aufgaben_heute.txt")
}
```

### 9. Zusammenfassung
- Zelyra bietet schlanke, sichere Funktionen für Datei-Ein-/Ausgabe.
- Das Capability-System verhindert verdeckte Spionage oder Datenmanipulation.

### 10. Kontrollfragen zur Selbstprüfung
1. Welche Fähigkeit muss eine Funktion anfordern, um `write_text` aufzurufen?
2. Warum verlangt Zelyra `uses FileSystem` auch für `main()`?
3. Welche Funktion liefert eine Liste aller Dateinamen in einem Ordner?

---

## Kapitel 21: Datum, Uhrzeit, Zufall und strukturierte Daten

### 1. Was lerne ich in diesem Kapitel?
- Wie du mit `now()` den aktuellen Zeitstempel abfragst (`uses Clock`).
- Wie Zufallswerte mit `random_int(min, max)` erzeugt werden (`uses Random`).
- Wie Datenstrukturen mit `json_encode` in universelles JSON umgewandelt werden.
- Wie JSON-Text mit `json_decode<T>` wieder typisiert zurückgewandelt wird.

### 2. Warum ist das Thema wichtig?
Praktisch jede reale Software benötigt Zeitangaben: Wann wurde eine Aufgabe erstellt? Wann ist die Frist abgelaufen? Auch strukturierte Datenformate wie JSON sind im Internet Standard – von REST-APIs bis hin zu Speicherständen. Zelyra integriert Zeit, Zufall und JSON nahtlos und typgeprüft.

### 3. Verständliche Erklärung
- **Zeitstempel:** `now()` liefert die exakte aktuelle Systemzeit als `Timestamp`. Dafür benötigt deine Funktion die Erlaubnis `uses Clock`.
- **Zufallszahlen:** `random_int(1, 10)` liefert eine unvorhersehbare Zahl zwischen 1 und 10 (`uses Random`).
- **JSON:** JSON ist ein einfaches Textformat, das Menschen und Computer gleichermaßen lesen können. Mit `json_encode` machst du aus einem Zelyra-Array einen Textstring, den du übers Netzwerk verschicken kannst.

```zelyra
fn zeige_zeit() uses Clock {
    jetzt = now()
    print("Aktuelle Systemzeit erfasst")
}

fn main() uses Clock {
    zeige_zeit()
}
```

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Fristen und Zeitmessung mit Clock**
```zelyra
fn protokolliere_erstellung(aufgabe: String) uses Clock {
    erstellt_um = now()
    print("Aufgabe angelegt: " + aufgabe)
}

fn main() uses Clock {
    protokolliere_erstellung("Server patchen")
}
```

**Beispiel 2: Zufällige Ticket-Nummern erzeugen**
```zelyra
fn generiere_ticket_nummer() -> Int uses Random {
    return random_int(1000, 9999)
}

fn main() uses Random {
    ticket = generiere_ticket_nummer()
    print("Dein Ticket-Code:")
    print(ticket)
}
```

**Beispiel 3: Daten als JSON exportieren**
```zelyra
fn exportiere_ids_als_json(ids: Int[]) -> String {
    return json_encode(ids)
}

fn main() {
    ids: Int[] = [101, 102, 103]
    json_text = exportiere_ids_als_json(ids)
    print("JSON-Ausgabe: " + json_text)
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** `now()` aufrufen, ohne `uses Clock` im Funktionskopf zu deklarieren.
  *Ursache:* Zeitabfragen sind nicht-deterministisch und erfordern in Zelyra eine Berechtigung.
- **Fehler:** Ungültigen JSON-Text in `json_decode` übergeben.
  *Ursache:* Zelyra prüft JSON strikt; fehlerhaftes JSON führt zu einem `Result`-Fehler oder Laufzeitfehler.

### 6. Merksätze
1. `now()` liefert die aktuelle Zeit und erfordert `uses Clock`.
2. `random_int` erzeugt Zufallszahlen und verlangt `uses Random`.
3. `json_encode` wandelt Zelyra-Datenstrukturen in standardisierten JSON-Text um.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Erzeuge eine Zufallszahl zwischen 1 und 6 (Würfel) und gib sie aus.
- **Stufe 2 (Mittel):** Schreibe eine Funktion, die ein Array von Status-Codes in JSON umwandelt.
- **Stufe 3 (Anspruchsvoll):** Kombiniere `uses Clock` und `uses FileSystem`, um einen Zeitstempel in eine Datei `log.txt` zu schreiben.

### 8. Praxisaufgabe: Aufgaben-Snapshot als JSON sichern
Erstelle einen JSON-Snapshot der Aufgaben-IDs und sichere ihn als Datei:
```zelyra
fn sichere_snapshot(dateiname: String, ids: Int[])
    uses Clock, FileSystem
{
    json_daten = json_encode(ids)
    write_text(dateiname, json_daten)
    print("Snapshot erfolgreich gesichert.")
}

fn main() uses Clock, FileSystem {
    aktuelle_ids: Int[] = [1, 2, 5, 8]
    sichere_snapshot("aufgaben_snapshot.json", aktuelle_ids)
}
```

### 9. Zusammenfassung
- Zelyra bietet integrierte, typsichere Unterstützung für Zeitstempel, Zufall und JSON.
- Berechtigungen (`Clock`, `Random`) sorgen für vollständige Nachvollziehbarkeit.

### 10. Kontrollfragen zur Selbstprüfung
1. Welche Capability benötigt eine Funktion, die `now()` aufruft?
2. Warum verlangt Zelyra für Zufallszahlen die Berechtigung `uses Random`?
3. In welches Format wandelt `json_encode` Zelyra-Objekte um?

---

## Kapitel 22: Nebenläufigkeit und Hintergrundaufgaben

### 1. Was lerne ich in diesem Kapitel?
- Was Nebenläufigkeit bedeutet und wann Aufgaben parallel ausgeführt werden sollten.
- Wie Zelyras `parallel`-Block funktioniert: `parallel { a = await ...; b = await ... }`.
- Warum Zelyra unkontrollierte Threads oder „Callback-Hölle“ vermeidet.
- Wie deterministische Nebenläufigkeit deine Anwendung schnell und sicher hält.

### 2. Warum ist das Thema wichtig?
Moderne Computer und Server besitzen viele Rechenkerne. Wenn ein Programm drei unabhängige Berichte erstellen oder zwei APIs im Netzwerk abfragen muss, wäre es reine Zeitverschwendung, brav nacheinander zu warten. Führt man die Abfragen gleichzeitig aus, ist das Programm doppelt oder dreifach so schnell. In vielen Sprachen führt Nebenläufigkeit jedoch zu gefürchteten Fehlern („Race Conditions“). Zelyra verhindert diese Risiken durch einen strukturierten, sicheren Ansatz.

### 3. Verständliche Erklärung
Stell dir ein Restaurant vor:
- Wenn der Koch erst das Steak brät, danach die Pommes frittiert und danach den Salat wäscht, wird das Essen kalt.
- Ein guter Küchenchef startet alle drei Schritte parallel und wartet, bis alle drei Schüsseln bereitstehen.

In Zelyra nutzt du dafür den `parallel`-Block mit `await`:
```zelyra
fn berechne_teil_1() -> Int {
    return 40
}

fn berechne_teil_2() -> Int {
    return 60
}

fn main() {
    parallel {
        ergebnis_1 = await berechne_teil_1()
        ergebnis_2 = await berechne_teil_2()
    }
    print("Beide Teilaufgaben parallel abgeschlossen.")
}
```

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Parallele Teilberechnungen**
```zelyra
fn berechne_statistiken() -> String {
    return "Statistiken berechnet"
}

fn lade_archiv() -> String {
    return "Archiv geladen"
}

fn main() {
    parallel {
        stats = await berechne_statistiken()
        archiv = await lade_archiv()
    }
    print("Paralleles Laden erfolgreich.")
}
```

**Beispiel 2: Unabhängige Datenbeschaffung**
```zelyra
fn summe_a() -> Int {
    return 100
}

fn summe_b() -> Int {
    return 250
}

fn main() {
    parallel {
        wert_a = await summe_a()
        wert_b = await summe_b()
    }
    print("Summen ermittelt.")
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Versuchen, `await` außerhalb eines `parallel`-Blocks zu nutzen.
  *Ursache:* Zelyra erlaubt `await` ausschließlich innerhalb von `parallel { ... }`.
- **Fehler:** Beliebige Anweisungen im `parallel`-Block platzieren.
  *Ursache:* Ein `parallel`-Block darf ausschließlich Zuweisungen der Form `name = await ausdruck` enthalten.

### 6. Merksätze
1. `parallel { ... }` führt unabhängige Operationen gleichzeitig aus.
2. Jede Zeile im `parallel`-Block folgt dem Muster `variable = await aufruf()`.
3. Strukturierte Nebenläufigkeit verhindert Deadlocks und unkontrollierte Hintergrundprozesse.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Definiere zwei einfache Berechnungsfunktionen und führe sie parallel aus.
- **Stufe 2 (Mittel):** Erstelle zwei Funktionen, die jeweils eine Zeichenkette erzeugen, und führe beide im `parallel`-Block aus.
- **Stufe 3 (Anspruchsvoll):** Simuliere das parallele Prüfen von zwei Bedingungen vor dem Start eines Projekts.

### 8. Praxisaufgabe: Paralleles Laden von Aufgaben und Benutzerdaten
Beschleunige den Systemstart unserer Aufgabenverwaltung:
```zelyra
fn lade_benutzerprofil() -> String {
    return "Profil: Entwickler"
}

fn lade_aufgabenliste() -> String {
    return "5 Aufgaben geladen"
}

fn main() {
    print("Starte parallelen Abruf...")
    parallel {
        profil = await lade_benutzerprofil()
        aufgaben = await lade_aufgabenliste()
    }
    print("Dashboard bereit.")
}
```

### 9. Zusammenfassung
- Nebenläufigkeit in Zelyra ist strukturiert, deterministisch und sicher vor Race Conditions.
- Der `parallel`-Block bündelt asynchrone Berechnungen sauber an einer Stelle.

### 10. Kontrollfragen zur Selbstprüfung
1. Wo darf das Schlüsselwort `await` in Zelyra verwendet werden?
2. Welchem Format müssen die Anweisungen innerhalb eines `parallel`-Blocks folgen?
3. Welcher Vorteil ergibt sich aus parallelen Abrufen gegenüber sequenzieller Abarbeitung?
