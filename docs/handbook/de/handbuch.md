# Das Zelyra-Handbuch

**Von den Grundlagen bis zur datenbankgestützten Webanwendung. Absicht beschreiben. Korrektheit beweisen.**

[English edition](/handbook) · Deutsch

Willkommen beim vollständigen Zelyra-Handbuch. Es umfasst sowohl das didaktische Lehrbuch **»Zelyra lernen – Verständlich programmieren von den Grundlagen bis zur eigenen Anwendung«** (Teil I bis X, Kapitel 1 bis 42) als auch das **technische Referenzhandbuch** (Kapitel 1 bis 23) sowie ausführliche **Anhänge** (A bis J).

> **Projektstatus:** Compiler 0.2.0 implementiert einen geprüften, experimentellen Teil der Sprachlinie 0.1. Zelyra ist noch nicht für den Produktionseinsatz freigegeben.

## Statuszeichen

- ✅ **Implementiert und geprüft:** im aktuellen Repository vorhanden und in diesem Arbeitslauf erfolgreich ausgeführt.
- 🧪 **Experimentell:** vorhanden, aber noch jung oder eingeschränkt.
- 🗺️ **Geplant:** Teil der Sprachvision, noch nicht zuverlässig verfügbar.
- ❌ **Derzeit nicht verfügbar:** im aktuellen CLI nicht vorhanden.

## Inhaltsverzeichnis

### Zelyra lernen – Das Lehrbuch

- **[TEIL I – ZELYRA UND PROGRAMMIERUNG VERSTEHEN](#teil-i-zelyra-und-programmierung-verstehen)**
  - [Kapitel 1: Willkommen bei Zelyra](#kapitel-1-willkommen-bei-zelyra)
  - [Kapitel 2: Wie ein Programm funktioniert](#kapitel-2-wie-ein-programm-funktioniert)
  - [Kapitel 3: Zelyra installieren und einrichten](#kapitel-3-zelyra-installieren-und-einrichten)
  - [Kapitel 4: Das erste Zelyra-Projekt](#kapitel-4-das-erste-zelyra-projekt)
- **[TEIL II – DIE GRUNDLAGEN DER SPRACHE](#teil-ii-die-grundlagen-der-sprache)**
  - [Kapitel 5: Werte und Datentypen](#kapitel-5-werte-und-datentypen)
  - [Kapitel 6: Variablen und Unveränderlichkeit](#kapitel-6-variablen-und-unveranderlichkeit)
  - [Kapitel 7: Operatoren und Ausdrücke](#kapitel-7-operatoren-und-ausdrucke)
  - [Kapitel 8: Ein- und Ausgaben](#kapitel-8-ein-und-ausgaben)
  - [Kapitel 9: Entscheidungen mit Bedingungen](#kapitel-9-entscheidungen-mit-bedingungen)
  - [Kapitel 10: Wiederholungen und Schleifen](#kapitel-10-wiederholungen-und-schleifen)
- **[TEIL III – PROGRAMME STRUKTURIEREN](#teil-iii-programme-strukturieren)**
  - [Kapitel 11: Funktionen und Prozeduren](#kapitel-11-funktionen-und-prozeduren)
  - [Kapitel 12: Verträge und Vorbedingungen (Design by Contract)](#kapitel-12-vertrage-und-vorbedingungen-design-by-contract)
  - [Kapitel 13: Sammlungen, Listen und Wörterbücher (Arrays & Maps)](#kapitel-13-sammlungen-listen-und-worterbucher-arrays-und-maps)
  - [Kapitel 14: Eigene Datentypen erstellen (Records & Tables)](#kapitel-14-eigene-datentypen-erstellen-records-tables)
  - [Kapitel 15: Module und Code-Organisation](#kapitel-15-module-und-code-organisation)
- **[TEIL IV – SICHERHEIT UND FEHLERBEHANDLUNG](#teil-iv-sicherheit-und-fehlerbehandlung)**
  - [Kapitel 16: Fehlerarten und ihre Ursachen](#kapitel-16-fehlerarten-und-ihre-ursachen)
  - [Kapitel 17: Fehler als Werte – Das Result-Muster](#kapitel-17-fehler-als-werte-das-result-muster)
  - [Kapitel 18: Das Nichts existiert nicht – Der sichere Umgang mit Option](#kapitel-18-das-nichts-existiert-nicht-der-sichere-umgang-mit-option)
  - [Kapitel 19: Tests und Qualitätssicherung](#kapitel-19-tests-und-qualitatssicherung)
- **[TEIL V – PRAKTISCHE DATENVERARBEITUNG](#teil-v-praktische-datenverarbeitung)**
  - [Kapitel 20: Arbeiten mit Dateien](#kapitel-20-arbeiten-mit-dateien)
  - [Kapitel 21: Datum, Uhrzeit, Zufall und strukturierte Daten](#kapitel-21-datum-uhrzeit-zufall-und-strukturierte-daten)
  - [Kapitel 22: Nebenläufigkeit und Hintergrundaufgaben](#kapitel-22-nebenlaufigkeit-und-hintergrundaufgaben)
- **[TEIL VI – DATENBANKEN MIT ZELYRA](#teil-vi-datenbanken-mit-zelyra)**
  - [Kapitel 23: Warum Zelyra die Datenbank direkt versteht](#kapitel-23-warum-zelyra-die-datenbank-direkt-versteht)
  - [Kapitel 24: Tabellen definieren und Daten modellieren](#kapitel-24-tabellen-definieren-und-daten-modellieren)
  - [Kapitel 25: Daten abfragen und verändern](#kapitel-25-daten-abfragen-und-verandern)
- **[TEIL VII – WEBANWENDUNGEN UND FORMULARE](#teil-vii-webanwendungen-und-formulare)**
  - [Kapitel 26: Webseiten ausgeben](#kapitel-26-webseiten-ausgeben)
  - [Kapitel 27: Formulare und Benutzereingaben](#kapitel-27-formulare-und-benutzereingaben)
  - [Kapitel 28: Das vollständige CRUD-Muster](#kapitel-28-das-vollstandige-crud-muster)
  - [Kapitel 29: Benutzer, Passwörter und Sitzungen](#kapitel-29-benutzer-passworter-und-sitzungen)
  - [Kapitel 30: APIs und Datenaustausch](#kapitel-30-apis-und-datenaustausch)
- **[TEIL VIII – DIE BESONDERHEITEN VON ZELYRA](#teil-viii-die-besonderheiten-von-zelyra)**
  - [Kapitel 31: Lesbarkeit als oberstes Gebot](#kapitel-31-lesbarkeit-als-oberstes-gebot)
  - [Kapitel 32: KI-Nativität – Warum Zelyra perfekt für KI-Assistenten ist](#kapitel-32-ki-nativitat-warum-zelyra-perfekt-fur-ki-assistenten-ist)
  - [Kapitel 33: Sicherheit durch Fähigkeiten (Capabilities)](#kapitel-33-sicherheit-durch-fahigkeiten-capabilities)
  - [Kapitel 34: Zelyra im Vergleich](#kapitel-34-zelyra-im-vergleich)
- **[TEIL IX – VOM ENTWURF ZUR FERTIGEN ANWENDUNG](#teil-ix-vom-entwurf-zur-fertigen-anwendung)**
  - [Kapitel 35: Software planen – Von der Idee zum Entwurf](#kapitel-35-software-planen-von-der-idee-zum-entwurf)
  - [Kapitel 36: Architektur und saubere Codestruktur](#kapitel-36-architektur-und-saubere-codestruktur)
  - [Kapitel 37: Konfiguration und Umgebungsvariablen](#kapitel-37-konfiguration-und-umgebungsvariablen)
  - [Kapitel 38: Fehlersuche und Optimierung](#kapitel-38-fehlersuche-und-optimierung)
  - [Kapitel 39: Bereitstellung und Betrieb](#kapitel-39-bereitstellung-und-betrieb)
- **[TEIL X – ABSCHLUSSPROJEKT UND WEITERFÜHRUNG](#teil-x-abschlussprojekt-und-weiterfuhrung)**
  - [Kapitel 40: Das große Abschlussprojekt: Vollständige Aufgabenverwaltung](#kapitel-40-das-grosse-abschlussprojekt-vollstandige-aufgabenverwaltung)
  - [Kapitel 41: Die Zelyra-Roadmap (Von 0.1 bis 1.0)](#kapitel-41-die-zelyra-roadmap-von-01-bis-10)
  - [Kapitel 42: Dein Weg als Zelyra-Entwickler](#kapitel-42-dein-weg-als-zelyra-entwickler)

### Technisches Referenzhandbuch

- [1. Was Zelyra anders macht](#1-was-zelyra-anders-macht)
- [2. Installation](#2-installation)
- [3. Das erste Programm](#3-das-erste-programm)
- [4. Neues Projekt anlegen und CLI](#4-neues-projekt-anlegen-und-cli)
- [5. Variablen, Typen und Funktionen](#5-variablen-typen-und-funktionen)
- [6. Option, Result und Pattern Matching](#6-option-result-und-pattern-matching)
- [7. MariaDB und Tabellen](#7-mariadb-und-tabellen)
- [8. Schema prüfen und anwenden](#8-schema-prufen-und-anwenden)
- [9. Natives SQL](#9-natives-sql)
- [10. Webseiten](#10-webseiten)
- [11. Formulare](#11-formulare)
- [12. CRUD](#12-crud)
- [13. Authentifizierung und Berechtigungen](#13-authentifizierung-und-berechtigungen)
- [14. Capabilities](#14-capabilities)
- [15. Contracts und Verify](#15-contracts-und-verify)
- [16. Konfiguration und Geheimnisse](#16-konfiguration-und-geheimnisse)
- [17. Diagnosen und Fehlersuche](#17-diagnosen-und-fehlersuche)
- [18. Testen und Mitentwickeln](#18-testen-und-mitentwickeln)
- [19. Was als Nächstes kommt](#19-was-als-nachstes-kommt)
- [20. Zelyra im Vergleich zu Rust](#20-zelyra-im-vergleich-zu-rust)
- [21. Positionierung und aktueller Entwicklungsstand](#21-positionierung-und-aktueller-entwicklungsstand)
- [22. Roadmap aus dem aktuellen Repository](#22-roadmap-aus-dem-aktuellen-repository)
- [23. KI-native Entwicklung](#23-ki-native-entwicklung)
- [24. Verbindliche Quellen und Compiler-Prüfung (Source Authority)](#24-verbindliche-quellen-und-compiler-prufung-source-authority)

### Anhänge

- [Anhang A: Schnelleinstieg / Spickzettel (Syntax-Cheat-Sheet)](#anhang-a-schnelleinstieg-spickzettel-syntax-cheat-sheet)
- [Anhang B: Alle Fehlermeldungen von Zelyra auf einen Blick](#anhang-b-alle-fehlermeldungen-von-zelyra-auf-einen-blick)
- [Anhang C: Zelyra-CLI-Referenz](#anhang-c-zelyra-cli-referenz)
- [Anhang D: Die Standardbibliothek im Überblick](#anhang-d-die-standardbibliothek-im-uberblick)
- [Anhang E: SQL-Spickzettel für Zelyra-Entwickler](#anhang-e-sql-spickzettel-fur-zelyra-entwickler)
- [Anhang F: HTML- und Web-Referenz in Zelyra](#anhang-f-html-und-web-referenz-in-zelyra)
- [Anhang G: Glossar der Fachbegriffe](#anhang-g-glossar-der-fachbegriffe)
- [Anhang H: Lösungen zu den Übungsaufgaben der Kapitel](#anhang-h-losungen-zu-den-ubungsaufgaben-der-kapitel)
- [Anhang I: Häufige Fragen und Antworten (FAQ)](#anhang-i-haufige-fragen-und-antworten-faq)
- [Anhang J: Weiterführende Ressourcen und Community](#anhang-j-weiterfuhrende-ressourcen-und-community)

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
- Wie du Zelyra über den Quellcode (`./install.sh` / `install.ps1`) oder vorkompilierte Release-Archive (`--release v0.2.0`) installierst.
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
./install.sh --release v0.2.0
```
Unter Windows (PowerShell):
```powershell
git clone https://github.com/sf1976/zelyra.git
Set-Location zelyra
.\install.ps1 -Release v0.2.0
```

**Schritt 2: Vollständige Version und Hilfe prüfen**
```bash
zelyra --version
zelyra --help
```
`zelyra --version` gibt den vollständigen Compiler- und Paketversionsstand aus (z. B. `zelyra 0.2.0`). Die Sprachkompatibilitätslinie bleibt 0.1.

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
3. Releases können mit `--release v0.2.0` direkt ohne Rust-Compiler installiert werden.
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
- **Stufe 2 (Mittel):** Ändere in `zelyra.toml` die Version auf `0.2.0` und gib in `main()` die neue Versionsnummer aus.
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

## Kapitel 8: Ein- und Ausgaben

### 1. Was lerne ich in diesem Kapitel?
- Wie man Informationen mit `print()` zuverlässig auf der Konsole ausgibt.
- Wie Text und Variablen durch Verkettung formatiert werden.
- Wie Zelyra in Version 0.1 Daten aus der Umwelt empfängt (Parameter, Dateien, Umgebungsvariablen, Web-Routen).
- Warum Zelyra für sensible Außeninteraktionen ausdrückliche Berechtigungen (`Capabilities`) verlangt.
- Der aktuelle Status und Ausblick für interaktive Konsoleneingaben.

### 2. Warum ist das Thema wichtig?
Ein Programm, das weder Daten empfangen noch Ergebnisse mitteilen kann, ist für den Anwender nutzlos. Ein- und Ausgaben (I/O) verbinden die Logik deines Codes mit der Außenwelt. Weil Zugriffe auf Tastatur, Festplatte oder Netzwerk aber auch Sicherheitsrisiken darstellen, regelt Zelyra diese Zugriffe viel kontrollierter als ältere Sprachen.

### 3. Verständliche Erklärung
- **Ausgabe:** Der eingebaute Befehl `print(wert)` nimmt Zahlen, Wahrheitswerte, Zeichenketten oder zusammengesetzte Objekte entgegen und gibt sie auf dem Standard-Ausgabekanal (`stdout`) aus.
- **Formatierung:** Mehrere Texte und Werte verbindest du mit dem Plus-Operator `+`.
- **Eingabe in Zelyra 0.1:**
  Im modernen Softwarebau laufen die allermeisten Programme auf Servern, in Containern oder als Hintergrunddienste. Sie fragen selten interaktiv den Benutzer an der Tastatur („Geben Sie Ihren Namen ein:“). Stattdessen empfangen sie Eingaben über vier sichere Wege:
  1. **Funktionsparameter:** Daten werden beim Aufruf übergeben.
  2. **Umgebungsvariablen:** `env("MEIN_KEY")` liest Konfigurationswerte aus dem System.
  3. **Dateien:** `read_text("eingabe.txt")` liest gespeicherte Daten ein.
  4. **Web-Anfragen:** Formulare (`form`) und URLs (`page "/user/{id}"`) empfangen Benutzereingaben im Browser.

> **Status-Hinweis zu interaktiver Konsoleneingabe:**
> Im Compiler 0.2.0 gibt es bewusst noch keine blockierende `read_line()`-Funktion für die Terminal-Tastatur. Die Zelyra-Spezifikation konzentriert sich primär auf deklarative Web-Eingaben und deterministische Datenflüsse.
> *(Roadmap-Platzhalter: `// [Platzhalter: Interaktives read_line() über Standard-Eingabe wird in Phase 11 spezifiziert]`)*.

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
2. Eingaben fließen in Zelyra über Parameter, Umgebungsvariablen, Dateien oder Web-Routen.
3. Systemzugriffe benötigen die passende Capability (z. B. `uses Environment`).

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Gib eine formatierte Visitenkarte (Name, Beruf, E-Mail) mit mehreren `print()`-Befehlen aus.
- **Stufe 2 (Mittel):** Schreibe eine Funktion `drucke_aufgabe(id: Int, name: String, erledigt: Bool)`, die alle Details sauber untereinander darstellt.
- **Stufe 3 (Anspruchsvoll):** Begründe, warum interaktive Terminal-Eingaben (`read_line`) in modernen Cloud- und Docker-Umgebungen kaum noch eine Rolle spielen und durch strukturierte APIs ersetzt wurden.

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
- Eingaben werden typisiert über Parameter, Dateien, Umgebungsvariablen oder Web-Anfragen entgegengenommen.

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
- Wie Zelyras CLI zusammenhängende Projektdateien prüft und baut.
- Den aktuellen Entwicklungsstand und die Roadmap von Modulen und Imports.

### 2. Warum ist das Thema wichtig?
Zu Beginn schreibt man gerne alles in eine einzige Datei. Doch wenn deine Aufgabenverwaltung wächst – mit Datenbanktabellen, 20 Funktionen, Webformularen und Validierungen –, verliert man in einer 2000-Zeilen-Datei schnell den Überblick. Gute Softwareentwicklung bedeutet, Code so zu organisieren, dass man Neuerungen sofort an der richtigen Stelle findet.

### 3. Verständliche Erklärung
Ein professionelles Zelyra-Projekt folgt einer klaren Ablagestruktur:
- `zelyra.toml`: Die Geburtsurkunde des Projekts. Hier stehen Name, Version und erforderliche Berechtigungen.
- `src/schema.zyl`: Enthält alle Tabellendefinitionen (`table`) und Domänentypen.
- `src/main.zyl`: Enthält die Hauptlogik und den Einstiegspunkt (`fn main()`).

```toml
[package]
name = "aufgaben_planer"
version = "0.1.0"
authors = ["Entwickler <dev@example.com>"]

[capabilities]
filesystem = false
database = true
```

*Hinweis zur Sprachversion 0.1:* Das Schlüsselwort `import` zur feingliedrigen Modularisierung externer Pakete befindet sich laut Zelyra-Roadmap aktuell in Entwicklung (Phase 11/12). In der aktuellen Version 0.1 werden die Quelldateien eines Projekts vom Zelyra-Compiler gemeinsam im Projektkontext übersetzt.
`// [Platzhalter: Modul-Importe - in Zelyra 0.1 noch nicht spezifiziert; siehe Roadmap Phase 11/12]`

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
- **Fehler:** Ein nicht vorhandenes `import`-Schlüsselwort aus anderen Sprachen wie Python oder JS verwenden.
  *Ursache:* Zelyra 0.1 verwendet Projekt-Kompilierung; externe Import-Syntax ist Gegenstand der Roadmap Phase 11.
- **Fehler:** `zelyra.toml` löschen oder im falschen Verzeichnis ausführen.
  *Ursache:* `zelyra run` sucht im aktuellen Verzeichnis nach der Konfiguration.

### 6. Merksätze
1. `zelyra.toml` steuert Metadaten und Sicherheitsrichtlinien des Projekts.
2. Trenne Datenmodell (`table`), Geschäftslogik (`fn`) und Darstellung sauber voneinander.
3. Ordnung im Projektverzeichnis schützt vor Flüchtigkeitsfehlern im Team.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Erstelle mit `zelyra new aufgaben_app` eine neue Projektstruktur und untersuche die erzeugten Dateien.
- **Stufe 2 (Mittel):** Konfiguriere in `zelyra.toml` eine Beschreibung und die Versionsnummer `0.2.0`.
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
- Projekte werden über `zelyra.toml` gesteuert und konfiguriert.
- Zelyra prüft Projektdateien als ganzheitliche Einheit.
- Eine modulare Denkweise erleichtert Erweiterungen und Teamarbeit.

### 10. Kontrollfragen zur Selbstprüfung
1. Welche Datei enthält die Metadaten eines Zelyra-Projekts?
2. Warum ist die Trennung von Datenmodell und Ausführungslogik sinnvoll?
3. Wie prüft die Zelyra-CLI das gesamte Projekt auf einmal?

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

# TEIL VI – DATENBANKEN MIT ZELYRA

---

## Kapitel 23: Warum Zelyra die Datenbank direkt versteht

### 1. Was lerne ich in diesem Kapitel?
In diesem Kapitel lernst du:
- Warum die Verbindung zwischen Programmiersprache und relationaler Datenbank traditionell oft fehleranfällig ist.
- Was das Problem der „ORM-Kluft“ (*Object-Relational Impedance Mismatch*) ist.
- Wie Zelyra Datenbanken als Bürger erster Klasse (*First-Class Citizen*) in die Sprache integriert.
- Wie Zelyra SQL-Befehle bereits zur Compile-Zeit auf syntaktische und typbezogene Korrektheit prüft.

### 2. Warum ist das Thema wichtig?
In fast allen gängigen Web-Frameworks (PHP/Laravel, Python/Django, Node/TypeORM) existiert eine unsaubere Trennung: Entwickler schreiben SQL-Strings oder nutzen komplexe Abstraktionsschichten (ORMs). Tippfehler in Spaltennamen wie `user.emaiil` werden oft erst bemerkt, wenn ein Nutzer im laufenden Betrieb einen Fehler 500 erhält. Zelyra beendet dieses Risiko: Wenn eine SQL-Abfrage nicht zum definierten Tabellenschema passt, verweigert der Compiler den Build sofort.

### 3. Verständliche Erklärung
In Zelyra definierst du deine Datenbank-Konfiguration direkt im Quelltext mit dem Schlüsselwort `database`:

```zelyra
database main {
    engine: mariadb
    database: "aufgaben_db"
}

fn main() {
    print("Datenbank-Konfiguration initialisiert.")
}
```

Wenn du Daten aus der Datenbank abfragst, schreibst du echtes SQL – aber der Compiler weiß genau, welche Spalten existieren:
- Schreibt man `SELECT id, beschreibung FROM tasks`, ist das gültig.
- Schreibt man `SELECT gibts_nicht FROM tasks`, meldet Zelyra schon beim Prüfen: `unknown column gibts_nicht`.

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Datenbank und Tabelle deklarieren**
```zelyra
database main {
    engine: mariadb
    database: "test_db"
}

table tasks {
    id: Id primary auto
    beschreibung: String(255) required
}

fn main() {
    print("Datenbank und Tabelle geprueft.")
}
```

**Beispiel 2: SQL-Prüfung zur Compile-Zeit**
```zelyra
database main {
    engine: mariadb
    database: "test_db"
}

table tasks {
    id: Id primary auto
    beschreibung: String(255) required
}

fn zeige_aufgaben() uses Database {
    daten = sql<Task[]> {
        SELECT id, beschreibung
        FROM tasks
    }
    print("SQL typgeprueft.")
}

fn main() uses Database {
    zeige_aufgaben()
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** SQL-Abfragen ohne Deklaration von `uses Database` in der Funktion ausführen.
  *Ursache:* Zelyras Capability-System schützt vor unerlaubten Datenbankzugriffen.
- **Fehler:** Den Datenbankblock `database main` vergessen.
  *Ursache:* Ohne Ziel-Engine kann Zelyra das SQL-Schema nicht verifizieren.

### 6. Merksätze
1. Zelyra schließt die Kluft zwischen Code und Datenbank.
2. SQL-Abfragen werden zur Compile-Zeit typgeprüft.
3. Datenbankzugriffe erfordern zwingend `uses Database`.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Erstelle einen `database`-Block für SQLite oder MariaDB.
- **Stufe 2 (Mittel):** Modelliere eine Tabelle `benutzer` und schreibe eine SQL-Abfrage, die alle Benutzer selektiert.
- **Stufe 3 (Anspruchsvoll):** Provoziere absichtlich einen Tippfehler in einem SQL-Spaltennamen und beobachte, wie Zelyra den Fehler exakt meldet.

### 8. Praxisaufgabe: Die Datenbank der Aufgabenverwaltung anbinden
Erstelle das Fundament unserer Aufgabenverwaltung:
```zelyra
database main {
    engine: mariadb
    database: "zelyra_tasks"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    erledigt: Bool
}

fn status_bericht() uses Database {
    liste = sql<Task[]> {
        SELECT id, name, erledigt
        FROM tasks
    }
    print("Datenbank fuer Aufgabenverwaltung einsatzbereit.")
}

fn main() uses Database {
    status_bericht()
}
```

### 9. Zusammenfassung
- Datenbanken und Schemas sind in Zelyra integraler Bestandteil der Sprache.
- Tippfehler in SQL werden bereits zur Entwicklungszeit verhindert.

### 10. Kontrollfragen zur Selbstprüfung
1. Welches Problem lösen Zelyras typgeprüfte SQL-Blöcke gegenüber gewöhnlichen SQL-Strings?
2. Welche Fähigkeit muss eine Funktion deklarieren, die `sql` ausführt?
3. Wie leitet Zelyra den Typ `Aufgabe` aus der Tabelle `aufgaben` ab?

---

## Kapitel 24: Tabellen definieren und Daten modellieren

### 1. Was lerne ich in diesem Kapitel?
- Wie relationale Tabellen mit `table` deklariert werden.
- Die Syntax für Primärschlüssel: `id: Id primary auto`.
- Wie Spalteneigenschaften definiert werden: `required`, Längenbegrenzung `String(100)`, Standardwerte.
- Wie Beziehungen zwischen Tabellen modelliert werden.

### 2. Warum ist das Thema wichtig?
Das Datenmodell ist das Fundament jeder Anwendung. Wenn das Schema unsauber entworfen ist, schleppt man Datenmüll und Performanceprobleme über Jahre mit sich herum. Zelyra erzwingt von Anfang an klare Pflichtfelder, Typen und Integrität.

### 3. Verständliche Erklärung
Eine Tabelle (`table`) ist wie ein Aktenordner für gleichartige Datenblätter:
- Jedes Datenblatt hat eine eindeutige laufende Nummer: `id: Id primary auto`.
- Bestimmte Angaben dürfen niemals fehlen: `required`.
- Für Texte kannst du Längenbegrenzungen angeben: `String(100)`.

```zelyra
table kategorien {
    id: Id primary auto
    bezeichnung: String(50) required
}
```

Aus der Tabellendefinition `table kategorien` generiert Zelyra automatisch den Datentyp `Kategorie` mit den exakten Feldern.

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Einfache Tabelle mit Pflichtfeldern**
```zelyra
database main {
    engine: mariadb
    database: "app_db"
}

table projekte {
    id: Id primary auto
    name: String(80) required
    aktiv: Bool
}

fn main() {
    print("Tabelle projekte deklariert.")
}
```

**Beispiel 2: Tabelle mit Datums- und Zahlenfeldern**
```zelyra
database main {
    engine: mariadb
    database: "app_db"
}

table zeiterfassungen {
    id: Id primary auto
    stunden: Float
    erfasst_am: Timestamp
}

fn main() {
    print("Tabelle zeiterfassungen deklariert.")
}
```

**Beispiel 3: Verknüpfung zweier Tabellen über IDs**
```zelyra
database main {
    engine: mariadb
    database: "app_db"
}

table tasks {
    id: Id primary auto
    beschreibung: String(200) required
    projekt_id: Id
}

fn main() {
    print("Beziehung aufgaben -> projekt_id angelegt.")
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Ein Pflichtfeld beim Anlegen weglassen.
  *Ursache:* Felder mit `required` müssen in jedem Datensatz gültige Werte besitzen.
- **Fehler:** Reservierte Wörter wie `action`, `field`, `title` als Spaltennamen wählen.
  *Ursache:* Diese Bezeichner sind für Zelyra-Sprachkonstrukte reserviert.

### 6. Merksätze
1. Jede Tabelle benötigt einen Primärschlüssel `id: Id primary auto`.
2. Das Attribut `required` verbietet leere Einträge auf Datenbank- und Sprachebene.
3. Der Singularname der Tabelle (z. B. `Aufgabe` für `aufgaben`) wird zum automatischen Datentyp.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Definiere eine Tabelle `etiketten` mit einem Pflichtfeld `name: String(30) required`.
- **Stufe 2 (Mittel):** Ergänze eine Tabelle `kunden` um `email: Email` und `telefon: String(30)`.
- **Stufe 3 (Anspruchsvoll):** Modelliere eine Tabelle `kommentare`, die per `aufgabe_id: Id` mit einer Aufgabe verknüpft ist und einen `erstellt_am: Timestamp` besitzt.

### 8. Praxisaufgabe: Vollständiges Schema für die Aufgabenverwaltung
Erstelle das produktive Datenmodell unserer Aufgabenverwaltung:
```zelyra
database main {
    engine: mariadb
    database: "zelyra_tasks"
}

table tasks {
    id: Id primary auto
    name: String(120) required
    beschreibung: String(500)
    prioritaet: Int
    ist_erledigt: Bool
}

fn main() {
    print("Vollstaendiges Aufgaben-Schema aktiv.")
}
```

### 9. Zusammenfassung
- `table` definiert Struktur, Typen und Beschränkungen der Daten.
- Zelyra sorgt dafür, dass Datenbank-Struktur und Code-Typen immer synchron bleiben.

### 10. Kontrollfragen zur Selbstprüfung
1. Wozu dient die Kennzeichnung `auto` beim Primärschlüssel?
2. Was bewirkt das Schlüsselwort `required` an einer Tabellenspalte?
3. Welcher Typname entsteht automatisch aus der Tabelle `projekte`?

---

## Kapitel 25: Daten abfragen und verändern

### 1. Was lerne ich in diesem Kapitel?
- Wie du mit `sql<T[]>` Datensätze sicher aus der Datenbank liest.
- Wie SQL-Injection durch parametrisierte Abfragen (`:param`) unmöglich gemacht wird.
- Wie man Datensätze mit `INSERT`, `UPDATE` und `DELETE` verändert.
- Warum Änderungen in `transaction { ... }`-Blöcken zusammengefasst werden.
- Die CLI-Migrationswerkzeuge (`zelyra db setup`, `zelyra db apply`).

### 2. Warum ist das Thema wichtig?
SQL-Injection gehört seit über 20 Jahren zu den gefährlichsten Sicherheitslücken im Web: Ein Angreifer gibt in ein Suchfeld manipulierten Text ein und liest fremde Passwörter aus oder löscht Tabellen. Zelyra schützt deine Software konstruktiv: Parameter in SQL-Blöcken werden mit Doppelpunkt (`:name`) gebunden und von der Engine immer sicher escaped. Gleichzeitig sichern Transaktionen ab, dass bei Fehlern keine halben Buchungen stehenbleiben.

### 3. Verständliche Erklärung
- **Lesen mit sql<T[]>:**
  ```zelyra
  database main {
      engine: mariadb
      database: "tasks_db"
  }
  table tasks {
      id: Id primary auto
      name: String required
      ist_erledigt: Bool
  }
  fn lade(filter_wert: Bool) uses Database {
      meine_tasks = sql<Task[]> {
          SELECT id, name, ist_erledigt
          FROM tasks
          WHERE ist_erledigt = :filter_wert
      }
      print("Tasks geladen")
  }
  fn main() uses Database { lade(false) }
  ```
- **Schreiben in einer Transaktion:**
  ```zelyra
  database main {
      engine: mariadb
      database: "tasks_db"
  }
  table tasks {
      id: Id primary auto
      name: String required
      ist_erledigt: Bool
  }
  fn anlegen(neuer_name: String) uses Database {
      fertig_flag = false
      transaction {
          sql {
              INSERT INTO tasks (name, ist_erledigt)
              VALUES (:neuer_name, :fertig_flag)
          }
      }
  }
  fn main() uses Database { anlegen("Test") }
  ```
  Sollte während der Transaktion etwas schiefgehen, macht die Datenbank alle Änderungen ungeschehen (*Rollback*).

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Einen neuen Datensatz einfügen**
```zelyra
database main {
    engine: mariadb
    database: "tasks_db"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    fertig: Bool
}

fn fuege_aufgabe_ein(text: String) uses Database {
    fertig_status = false
    transaction {
        sql {
            INSERT INTO tasks (name, fertig)
            VALUES (:text, :fertig_status)
        }
    }
    print("Aufgabe gespeichert.")
}

fn main() uses Database {
    fuege_aufgabe_ein("E-Mail beantworten")
}
```

**Beispiel 2: Datensätze typisiert abfragen**
```zelyra
database main {
    engine: mariadb
    database: "tasks_db"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    fertig: Bool
}

fn lade_alle() uses Database {
    liste = sql<Task[]> {
        SELECT id, name, fertig
        FROM tasks
    }
    print("Aufgabenliste geladen.")
}

fn main() uses Database {
    lade_alle()
}
```

**Beispiel 3: Datensatz aktualisieren**
```zelyra
database main {
    engine: mariadb
    database: "tasks_db"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    fertig: Bool
}

fn markiere_als_fertig(aufgabe_id: Int) uses Database {
    transaction {
        sql {
            UPDATE tasks
            SET fertig = true
            WHERE id = :aufgabe_id
        }
    }
    print("Status aktualisiert.")
}

fn main() uses Database {
    markiere_als_fertig(1)
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Werte mit String-Verkettung in SQL einfügen wollen (`"WHERE id = " + id`).
  *Ursache:* In Zelyra gibt es keine String-Zusammenstückelung in SQL. Nutze immer Parameter mit Doppelpunkt (`:id`).
- **Fehler:** Vergessen, dass Änderungen in `transaction { ... }` gekapselt sein müssen.
  *Ursache:* Zelyra verlangt für Schreiboperationen klare Transaktionsgrenzen.

### 6. Merksätze
1. Binde Eingabewerte in SQL stets mit `:parameter` – das schützt vor SQL-Injection.
2. Schreibende Operationen gehören in `transaction { ... }`.
3. Mit `zelyra db apply` wird das definierte Schema auf die Datenbank übertragen.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Schreibe eine SQL-Abfrage, die alle unfertigen Aufgaben (`fertig = false`) selektiert.
- **Stufe 2 (Mittel):** Schreibe eine Funktion zum Löschen einer Aufgabe anhand ihrer ID (`DELETE FROM tasks WHERE id = :id`).
- **Stufe 3 (Anspruchsvoll):** Implementiere eine Funktion, die innerhalb einer einzigen Transaktion eine alte Aufgabe archiviert und eine neue Nachfolge-Aufgabe anlegt.

### 8. Praxisaufgabe: Vollständige Datenbank-Operationen der Aufgabenverwaltung
Schreibe den Datenzugriff für unsere Aufgabenverwaltung:
```zelyra
database main {
    engine: mariadb
    database: "zelyra_tasks"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    ist_erledigt: Bool
}

fn aufgabe_anlegen(aufgabe_name: String) uses Database {
    status_initial = false
    transaction {
        sql {
            INSERT INTO tasks (name, ist_erledigt)
            VALUES (:aufgabe_name, :status_initial)
        }
    }
    print("Aufgabe angelegt.")
}

fn aufgabe_abschliessen(ziel_id: Int) uses Database {
    transaction {
        sql {
            UPDATE tasks
            SET ist_erledigt = true
            WHERE id = :ziel_id
        }
    }
    print("Aufgabe abgeschlossen.")
}

fn main() uses Database {
    aufgabe_anlegen("Erstes Zelyra Projekt starten")
    aufgabe_abschliessen(1)
}
```

### 9. Zusammenfassung
- SQL in Zelyra ist nativ, typsicher und automatisch vor Angriffen geschützt.
- `transaction` schützt die Datenbankkonsistenz bei allen Änderungen.

### 10. Kontrollfragen zur Selbstprüfung
1. Wie schützt Zelyra vor bösartigen SQL-Injection-Angriffen?
2. Warum müssen schreibende SQL-Befehle in einem `transaction`-Block stehen?
3. Welcher CLI-Befehl richtet die Datenbanktabellen anhand des Codes ein?

# TEIL VII – WEBANWENDUNGEN UND FORMULARE

---

## Kapitel 26: Webseiten ausgeben

### 1. Was lerne ich in diesem Kapitel?
In diesem Kapitel lernst du:
- Wie du mit `page` blitzschnell Webseiten und Routen erstellst.
- Wie URL-Parameter (z. B. `/tasks/{id}`) dynamisch übergeben werden.
- Wie HTML-Vorlagen direkt im Zelyra-Code definiert werden (`html { ... }`).
- Wie Zelyra Cross-Site Scripting (XSS) durch automatisches HTML-Escaping verhindert.

### 2. Warum ist das Thema wichtig?
Im klassischen Web-Development muss man oft drei verschiedene Welten verbinden: Einen Webserver (wie Nginx oder Apache), einen Router, eine Template-Engine (Blade, Jinja, Twig) und den Anwendungscode. Wenn man an einer Stelle vergisst, HTML-Sonderzeichen zu maskieren, können Angreifer bösartiges JavaScript einschleusen (XSS). In Zelyra ist der Webserver direkt integriert (`zelyra serve`), und das HTML-Escaping geschieht automatisch und unumgänglich.

### 3. Verständliche Erklärung
Mit dem Schlüsselwort `page` definierst du eine Webroute und das dazugehörige HTML:

```zelyra
page "/willkommen" {
    html {
        <h1>Willkommen bei Zelyra</h1>
        <p>Deine moderne Webanwendung laeuft!</p>
    }
}
```

Wenn du dynamische Werte anzeigen möchtest, setzt du sie einfach in geschweifte Klammern: `{name}`. Zelyra ersetzt den Platzhalter sicher durch den echten Text.

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Einfache statische Begrüßungsseite**
```zelyra
page "/hallo" {
    html {
        <html>
            <body>
                <h1>Hallo Zelyra-Welt!</h1>
            </body>
        </html>
    }
}
```

**Beispiel 2: Dynamische Route mit URL-Parameter**
```zelyra
page "/benutzer/{name}" {
    html {
        <html>
            <body>
                <h1>Profil von {name}</h1>
                <p>Willkommen zurueck im Dashboard.</p>
            </body>
        </html>
    }
}
```

**Beispiel 3: Sicheres Escaping gegen XSS-Angriffe**
Übergibt ein Nutzer als Namen `<script>alert('hack')</script>`, gibt Zelyra dies im Browser als harmlosen Text aus – das Skript wird niemals ausgeführt:
```zelyra
page "/sicher/{eingabe}" {
    html {
        <div>Eingabe: {eingabe}</div>
    }
}
```

**Beispiel 4: Wiederverwendbare View-Layouts und Slots (ab Zelyra 0.1.41)**
Statt auf jeder Seite `<html>`, `<head>`, Header und Footer neu zu schreiben, definierst du mit `view` ein Layout. Ein View besitzt genau einen Hauptslot `<slot />` sowie optionale benannte Slots mit sicherem Standardinhalt:
```zelyra
view AppShell {
    html {
        <html lang="de">
            <head><title>Zelyra Anwendung</title></head>
            <body>
                <header>
                    <slot name="header"><h1>Zelyra Portal</h1></slot>
                </header>
                <main>
                    <slot />
                </main>
                <footer>
                    <slot name="footer"><p>Erstellt mit Zelyra</p></slot>
                </footer>
            </body>
        </html>
    }
}

page "/dashboard" {
    view: AppShell
    html {
        <slot name="header"><h1>Mein Dashboard</h1></slot>
        <p>Der eigentliche Inhalt wird im Haupt-Slot der AppShell platziert.</p>
    }
}
```

**Beispiel 5: Deklarative Suche, Filterung und Pagination (ab Zelyra 0.1.40)**
Für datengetriebene Seiten erzeugt Zelyra semantische Steuerungen für Suche, Sortierung und Seitenaufteilung automatisch, inklusive URL-Zustandserhalt:
```zelyra
database main {
    engine: mariadb
}

table tasks {
    id: Id primary auto
    title: String(100) required
    done: Bool default false
}

page "/tasks" {
    search { title }
    filter { done }
    sort { title }
    paginated 25

    load tasks = sql<Task[]> {
        SELECT id, title, done
        FROM tasks
        ORDER BY title
    }

    html {
        <h1>Aufgaben ({total} gesamt, Seite {page} von {pages})</h1>
        <ul>
            for task in tasks {
                <li>{task.title}</li>
            }
        </ul>
    }
}
```
Zelyra führt im Hintergrund automatisch die optimierte Zählabfrage (`COUNT(*)`) aus, bindet `total` und `pages` als sichere `UInt`-Variablen und rendert semantische Filter-Fieldsets.

### 5. Typische Fehler und deren Ursachen
- **Fehler:** HTML-Tags nicht ordnungsgemäß schließen (z. B. `<h1>` ohne `</h1>`).
  *Ursache:* Zelyra prüft den HTML-Baum syntaktisch auf Wohlgeformtheit.
- **Fehler:** URL-Parameter in geschweiften Klammern falsch benennen.
  *Ursache:* Der Parameter in der Route (z. B. `{id}`) muss mit der Variablen im HTML übereinstimmen.
- **Fehler `E-VIEW-010` bis `E-VIEW-015`:** Unbekannte Variablen oder falsche Typen in der View-Interpolation.
  *Ursache:* Der Zelyra-Compiler prüft View-Bindungen und Component-Properties bereits zur Compile-Zeit strikt gegen deklarierte Routen, Typen und SQL-Loads.
- **Fehler:** Unbekannte oder doppelte Slots in `page` angeben.
  *Ursache:* Eine Seite darf nur benannte Slots befüllen, die der ausgewählte `view` auch tatsächlich deklariert.

### 6. Merksätze
1. `page "/pfad"` definiert eine Route und liefert geprüften HTML-Code aus.
2. Variablen im HTML werden mit `{variable}` sicher interpoliert und automatisch escaped.
3. `view Name { ... }` definiert wiederverwendbare Master-Layouts mit `<slot />` und benannten Slots (`<slot name="...">`).
4. `search`, `filter` und `paginated` erzeugen vollautomatische, semantische Query-Steuerungen mit Zustandsbewahrung in der URL.
5. Mit `zelyra serve` startest du den integrierten HTTP-Server ohne externe Webserver-Konfiguration.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Erstelle eine `page "/ueber-uns"`, die eine Firmenbeschreibung anzeigt.
- **Stufe 2 (Mittel):** Erstelle eine dynamische Route `/produkt/{nummer}`, die eine Produkt-Detailansicht darstellt.
- **Stufe 3 (Anspruchsvoll):** Gestalte eine Übersichtsseite mit Überschrift, Navigation und Aufzählungsliste im HTML-Block.

### 8. Praxisaufgabe: Startseite für die Aufgabenverwaltung
Erstelle die Web-Startseite unserer Aufgabenverwaltung:
```zelyra
page "/tasks" {
    html {
        <html>
            <head>
                <title>Zelyra Aufgabenverwaltung</title>
            </head>
            <body>
                <h1>Meine Aufgaben</h1>
                <p>Willkommen in deiner persoenlichen Aufgabenverwaltung.</p>
                <a href="/tasks/neu">Neue Aufgabe erstellen</a>
            </body>
        </html>
    }
}
```

Starte den Server mit `zelyra serve main.zyl` und öffne `http://localhost:8080/tasks` im Browser!

### 9. Zusammenfassung
- Webseiten werden mit `page` und `html` direkt deklariert.
- Automatisches Escaping schützt deine Nutzer vor Sicherheitsrisiken.

### 10. Kontrollfragen zur Selbstprüfung
1. Welches Schlüsselwort leitet eine Webseiten-Definition ein?
2. Wie bindet man dynamische Werte in den HTML-Quelltext ein?
3. Warum ist XSS bei der HTML-Ausgabe in Zelyra standardmäßig ausgeschlossen?

---

## Kapitel 27: Formulare und Benutzereingaben

### 1. Was lerne ich in diesem Kapitel?
- Wie du mit `form` sichere Eingabemasken für deine Datenbanktabellen definierst.
- Wie automatische CSRF-Schutzmechanismen funktionieren.
- Wie Zelyra Eingabedaten typisiert validiert (z. B. `Email`, Mindestlängen).
- Wie du Formulare mit dem CLI-Befehl `zelyra form validate` vorab testen kannst.

### 2. Warum ist das Thema wichtig?
Eingaben von Nutzern sind die Hauptursache für Sicherheitslücken im Web: Angreifer übermitteln leere Pflichtfelder, manipulierte IDs oder nutzen fremde Browser-Sitzungen aus (CSRF-Attacken). In anderen Frameworks muss man Formulare mühsam von Hand mit Validierungsregeln, Fehleranzeigen und CSRF-Tokens zusammenbauen. Zelyras `form`-Konstrukt leitet die Eingabemaske direkt aus der Datenbanktabelle ab und sichert alles automatisch ab.

### 3. Verständliche Erklärung
Ein Formular verknüpft eine Eingabemaske mit einer Zieltabelle:

```zelyra
database main {
    engine: mariadb
}

table tasks {
    id: Id primary auto
    name: String(100) required
    beschreibung: String(500)
}

form TaskCreate -> tasks {
    fields {
        name
        beschreibung
    }
}
```

Zelyra generiert daraus:
- Die HTML-Eingabefelder mit passenden Typen (`<input type="text">`, etc.).
- Ein unsichtbares, kryptografisches CSRF-Token, das Angriffe verhindert.
- Server-seitige Validierungsprüfungen (z. B. `name` darf maximal 100 Zeichen haben und nicht fehlen).

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Basis-Formular für Kundendaten**
```zelyra
database main {
    engine: mariadb
}

table customers {
    id: Id primary auto
    name: String(80) required
    email: Email?
}

form CustomerForm -> customers {
    fields {
        name
        email
    }
}
```

**Beispiel 2: Formular mit benutzerdefinierten Aktionen**
```zelyra
database main {
    engine: mariadb
}

table tasks {
    id: Id primary auto
    name: String(100) required
}

form NewTaskForm -> tasks {
    fields {
        name
    }
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Ein Feld im Formular aufführen, das in der Zieltabelle gar nicht existiert.
  *Ursache:* Zelyra prüft `fields` strikt gegen die Spalten der Tabelle.
- **Fehler:** CSRF-Schutz manuell deaktivieren wollen.
  *Ursache:* In Zelyra ist der CSRF-Schutz unverzichtbarer Sicherheitsstandard.

### 6. Merksätze
1. `form Name -> zieltabelle` generiert eine sichere Eingabemaske.
2. Alle Validierungsregeln der Tabelle (Länge, Pflichtfeld, Typ) gelten automatisch.
3. CSRF- und XSS-Schutz sind integral eingebaut.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Definiere ein Formular `KategorieErstellen` für eine Tabelle `kategorien`.
- **Stufe 2 (Mittel):** Teste das Formular auf der Kommandozeile mit `zelyra form validate`.
- **Stufe 3 (Anspruchsvoll):** Ergänze das Aufgaben-Formular um ein Prioritätsfeld und validiere fehlerhafte Eingaben.

### 8. Praxisaufgabe: Das Erstellungsformular für Aufgaben
Definiere das Eingabeformular für neue Aufgaben:
```zelyra
database main {
    engine: mariadb
}

table tasks {
    id: Id primary auto
    name: String(100) required
    prioritaet: Int
}

form TaskCreate -> tasks {
    fields {
        name
        prioritaet
    }
}

fn main() {
    print("Aufgaben-Formular bereit.")
}
```

### 9. Zusammenfassung
- Formulare verknüpfen Tabellen mit sicheren Web-Eingabemasken.
- Zelyra erledigt Validierung, CSRF-Schutz und Fehlerbehandlung automatisch.

### 10. Kontrollfragen zur Selbstprüfung
1. Wofür steht der Pfeil `->` in `form TaskCreate -> tasks`?
2. Warum müssen Entwickler in Zelyra keine manuellen CSRF-Tokens im HTML einfügen?
3. Welche Spaltenprüfungen werden automatisch auf das Formular angewendet?

---

## Kapitel 28: Das vollständige CRUD-Muster

### 1. Was lerne ich in diesem Kapitel?
- Was CRUD (Create, Read, Update, Delete) bedeutet und warum es das Herzstück von Business-Web-Apps ist.
- Wie Zelyra ein vollständiges Verwaltungsinterface mit nur einem `crud`-Block erzeugt.
- Wie du Listen-, Detail-, Formular- und Löschansichten konfigurierst.
- Wie benutzerdefinierte Aktionen mit `action` hinzugefügt werden.

### 2. Warum ist das Thema wichtig?
Über 80 % der Arbeit an Webanwendungen besteht aus dem immer gleichen Muster: Eine Tabelle anzeigen, Datensätze anlegen, bearbeiten und löschen. Entwickler verbringen Wochen damit, Controller, Routen, Formulare und Bestätigungsdialoge zu schreiben. In Zelyra erledigst du das in wenigen Zeilen deklarativem Code – absolut fehlerfrei, sicher und konsistent.

### 3. Verständliche Erklärung
Das Schlüsselwort `crud` fasst alle Operationen für eine Entität zusammen:
- **C**reate: Neue Datensätze anlegen.
- **R**ead: Liste durchsuchen und Details ansehen.
- **U**pdate: Vorhandene Daten ändern.
- **D**elete: Datensätze mit Sicherheitsabfrage entfernen.

```zelyra
database main {
    engine: mariadb
}

table tasks {
    id: Id primary auto
    name: String(100) required
    erledigt: Bool default false
}

crud Task -> tasks {
    title: "Aufgabenverwaltung"
    view {
        fields {
            name
            erledigt
        }
        list {
            mode: cards
            empty: "Keine Aufgaben vorhanden."
        }
    }
}
```

> **Automatische Detail-Verlinkung bei ausgeblendeter ID (ab Version 0.1.50):**
> Wenn die technische `id`-Spalte im Block `fields` nicht aufgeführt ist (wie hier, wo nur `name` und `erledigt` sichtbar sind), verlinkt Zelyra automatisch das erste angezeigte Feld (`name`) mit der Detailseite des Datensatzes. Dies gilt sowohl für Tabellen- (`table`) als auch für Kachel-Layouts (`cards`).

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Ein vollständiges CRUD-Modul**
```zelyra
database main {
    engine: mariadb
}

table tasks {
    id: Id primary auto
    name: String(100) required
    aktiv: Bool default true
}

crud Task -> tasks {
    title: "Aufgaben"

    view {
        fields {
            name
            aktiv
        }

        list {
            mode: cards
            empty: "Keine Aufgaben vorhanden."
        }

        detail {
            mode: cards
            title: "Aufgabendetails"
        }

        form {
            mode: cards
            title: "Aufgabe bearbeiten"
            submit: "Speichern"
        }

        delete {
            title: "Aufgabe loeschen"
            message: "Diese Aktion kann nicht rueckgaengig gemacht werden."
            submit: "Jetzt loeschen"
        }
    }
}
```

**Beispiel 2: Benutzerdefinierte Aktionen im CRUD-Interface**
Du kannst individuelle Schaltflächen hinzufügen, z. B. um eine Aufgabe sofort als erledigt zu markieren:
```zelyra
database main {
    engine: mariadb
}

table tasks {
    id: Id primary auto
    name: String(100) required
    erledigt: Bool default false
}

crud Task -> tasks {
    title: "Aufgaben"
    view {
        fields {
            name
            erledigt
        }
    }

    action erledigen {
        label: "Als erledigt markieren"
        confirm: "Moechtest du diese Aufgabe abschliessen?"

        sql {
            UPDATE tasks
            SET erledigt = true
            WHERE id = :id
        }

        success "Aufgabe erfolgreich abgeschlossen."
        redirect "/tasks"
    }
}
```

**Beispiel 3: CRUD-Ressource mit wiederverwendbarem View-Layout (ab Zelyra 0.1.43)**
Mit `layout: ViewName` bettest du alle generierten CRUD-Ansichten (Listen-, Detail-, Formular- und Löschdialoge) automatisch in ein zuvor deklariertes Seitenlayout ein:
```zelyra
view AppShell {
    html {
        <html lang="de">
            <body>
                <nav><a href="/">Start</a> | <a href="/tasks">Aufgaben</a></nav>
                <main>
                    <slot />
                </main>
            </body>
        </html>
    }
}

crud Task -> tasks {
    title: "Aufgabenverwaltung"
    layout: AppShell

    view {
        fields {
            name
            erledigt
        }
    }
}
```
Die generierte CRUD-Ressource übernimmt das Navigationsgerüst von `AppShell`, während alle Sicherheitsprüfungen, Rollen und CSRF-Tokens voll aktiv bleiben.

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Den `view`-Block oder die `fields`-Deklaration im CRUD weglassen.
  *Ursache:* Zelyra muss wissen, welche Spalten in den generierten Ansichten angezeigt werden sollen.
- **Fehler:** Ein `UPDATE` in einer `action` ohne `WHERE id = :id` ausführen.
  *Ursache:* Aktionen beziehen sich immer auf den aktuell ausgewählten Datensatz mit `:id`.

### 6. Merksätze
1. `crud Name -> tabelle` generiert eine vollständige, sichere Verwaltungsoberfläche.
2. Ansichten (`list`, `detail`, `form`, `delete`) lassen sich flexibel anpassen.
3. Benutzerdefinierte Aktionen (`action`) erweitern den Standard um individuelle Geschäftsregeln.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Erstelle ein CRUD-Interface für eine Tabelle `kategorien`.
- **Stufe 2 (Mittel):** Passe die Hinweistexte und Titel der Ansichten an.
- **Stufe 3 (Anspruchsvoll):** Implementiere eine benutzerdefinierte Aktion `duplizieren`, die eine Kopie der ausgewählten Aufgabe anlegt.

### 8. Praxisaufgabe: Das produktive CRUD-Interface unserer Aufgabenverwaltung
Kombiniere Datenbank, Tabelle und CRUD zu einem vollständigen System:
```zelyra
database main {
    engine: mariadb
    database: "tasks_app"
}

table tasks {
    id: Id primary auto
    name: String(120) required
    erledigt: Bool default false
}

crud Task -> tasks {
    title: "Meine Aufgabenverwaltung"

    view {
        fields {
            name
            erledigt
        }

        list {
            mode: cards
            empty: "Grossartig! Alle Aufgaben sind erledigt."
        }

        detail {
            mode: cards
            title: "Aufgabe ansehen"
        }

        form {
            mode: cards
            title: "Aufgabe erfassen oder bearbeiten"
            submit: "Aufgabe sichern"
        }

        delete {
            title: "Aufgabe loeschen"
            message: "Soll diese Aufgabe unwiderruflich entfernt werden?"
            submit: "Loeschen"
        }
    }
}
```

### 9. Zusammenfassung
- Das CRUD-Muster beschleunigt die Entwicklung von Datenbank-Web-Apps dramatisch.
- Zelyra generiert fehlerfreie Routen, HTML-Masken und Datenbankaufrufe vollautomatisch.

### 10. Kontrollfragen zur Selbstprüfung
1. Welche vier Grundfunktionen umfasst die Abkürzung CRUD?
2. Wie reagiert Zelyra, wenn in `fields` ein ungültiger Spaltenname steht?
3. Welche Aufgabe erfüllt das Schlüsselwort `action` in einem CRUD-Block?

---

## Kapitel 29: Benutzer, Passwörter und Sitzungen

### 1. Was lerne ich in diesem Kapitel?
- Wie Zelyra Benutzerauthentifizierung mit dem `auth`-Block nativ bereitstellt.
- Wie Passwörter mit dem modernen Argon2-Algorithmus gehasht werden.
- Wie Sitzungen (*Sessions*) und Rollenrechte abgesichert werden.
- Wie Seiten und Aktionen mit `requires auth` und `permits` geschützt werden.

### 2. Warum ist das Thema wichtig?
Sicherheit ist kein nachträgliches Add-on. Wer Passwörter im Klartext oder mit veralteten Algorithmen (wie MD5 oder SHA1) speichert, riskiert Datenschutz-Katastrophen. In Zelyra ist Authentifizierung fest in der Spracharchitektur verankert: Passwörter werden standardmäßig mit Argon2 gehasht, Sitzungs-Tokens werden kryptografisch geschützt und Routenrechte werden deklarativ geprüft.

### 3. Verständliche Erklärung
Ein Authentifizierungsblock verbindet Benutzer, Sitzungen und Berechtigungen:

```zelyra
auth users {
    table: users
    sessions: auth_sessions
    permissions: user_permissions
}

table users {
    id: Id primary auto
    email: Email required unique
    password_hash: String(255) required
}

table auth_sessions {
    id: Id primary auto
    user: User required
    token_hash: String(64) required unique
    expires_at: Timestamp required
}

table user_permissions {
    id: Id primary auto
    user: User required
    permission: String(100) required
}
```

Um eine Webroute nur für angemeldete Nutzer freizugeben, schreibst du einfach:
```zelyra
auth users {
    table: users
    sessions: auth_sessions
    permissions: user_permissions
}

table users {
    id: Id primary auto
    email: Email required unique
    password_hash: String(255) required
}

table auth_sessions {
    id: Id primary auto
    user: User required
    token_hash: String(64) required unique
    expires_at: Timestamp required
}

table user_permissions {
    id: Id primary auto
    user: User required
    permission: String(100) required
}

page "/geheim" {
    requires auth
    html {
        <h1>Nur fuer angemeldete Nutzer sichtbar!</h1>
    }
}
```

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Die Standardtabellen für Authentifizierung**
```zelyra
auth users {
    table: users
    sessions: auth_sessions
    permissions: user_permissions
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
    user: User required
    permission: String(100) required
}
```

**Beispiel 2: Geschützte Seite mit Rechteprüfung**
```zelyra
auth users {
    table: users
    sessions: auth_sessions
    permissions: user_permissions
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
    user: User required
    permission: String(100) required
}

page "/dashboard" {
    requires auth
    permits "tasks.view"

    html {
        <h1>Aufgaben-Dashboard</h1>
        <p>Du bist autorisiert.</p>
    }
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Passwörter im Klartext in der Tabelle ablegen.
  *Ursache:* Zelyra verlangt ein Spaltenfeld `password_hash` und stellt mit `zelyra auth hash-password` ein Hashing-Tool bereit.
- **Fehler:** Die Sitzungstabelle `auth_sessions` vergessen.
  *Ursache:* Zelyra benötigt eine dedizierte Tabelle zur sicheren Token-Verwaltung.

### 6. Merksätze
1. `auth` deklariert Benutzer, Sitzungen und Berechtigungen an einer zentralen Stelle.
2. Geschützte Seiten erfordern `requires auth` und optional `permits "recht"`.
3. Passwörter werden ausschließlich als sichere Argon2-Hashes gespeichert.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Erzeuge ein Passwort-Hash über den CLI-Befehl `zelyra auth hash-password`.
- **Stufe 2 (Mittel):** Schütze eine Seite `/einstellungen` mit `requires auth`.
- **Stufe 3 (Anspruchsvoll):** Vergib einer Rolle ein Berechtigungsrecht über `zelyra auth role-permission`.

### 8. Praxisaufgabe: Geschützte Aufgabenverwaltung
Schütze die Aufgabenverwaltung vor unbefugtem Zugriff:
```zelyra
auth users {
    table: users
    sessions: auth_sessions
    permissions: user_permissions
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
    user: User required
    permission: String(100) required
}

page "/meine-aufgaben" {
    requires auth

    html {
        <h1>Geschuetzter Aufgabenbereich</h1>
        <p>Nur fuer authentifizierte Benutzer zugaenglich.</p>
    }
}
```

### 9. Zusammenfassung
- Authentifizierung und Autorisierung sind vollwertige Bestandteile von Zelyra.
- Moderne Sicherheitsstandards (Argon2, Session-Tokens, RBAC) sind ohne Drittanbieter-Bibliotheken einsatzbereit.

### 10. Kontrollfragen zur Selbstprüfung
1. Welcher moderne Hash-Algorithmus wird von Zelyra für Passwörter verwendet?
2. Mit welchem Befehl wird eine Webroute für unbefugte Besucher gesperrt?
3. Wozu dient die Tabelle `auth_sessions`?

---

## Kapitel 30: APIs und Datenaustausch

### 1. Was lerne ich in diesem Kapitel?
- Wie du typisierte REST-Schnittstellen mit dem Schlüsselwort `api` definierst.
- Wie HTTP-Methoden (`GET`, `POST`, `PUT`, `DELETE`) und Eingabe/Ausgabe typisiert werden.
- Wie automatische Fehler-Codes (`404 NotFound`, `400 ValidationError`) deklariert werden.
- Wie Zelyra vollständige OpenAPI/Swagger-Dokumentationen auf Knopfdruck generiert (`zelyra doc --openapi`).

### 2. Warum ist das Thema wichtig?
Moderne Software lebt nicht isoliert: Mobile Apps (iOS/Android), Frontend-Frameworks (Vue, React) oder Partnersysteme müssen mit deinem Backend kommunizieren. Bei herkömmlichen APIs veraltet die Dokumentation oft schon am Tag nach dem Release. In Zelyra ist die API-Definition der Code selbst: Jede Route, jedes Eingabefeld und jeder Fehlercode ist exakt typisiert – und die OpenAPI-Spezifikation wird daraus vollautomatisch abgeleitet.

### 3. Verständliche Erklärung
Mit dem Schlüsselwort `api` deklarierst du einen Web-Endpunkt:
- **Methode und Pfad:** z. B. `GET "/tasks/{id}"`
- **Input:** Welche Parameter erwartet die API?
- **Output:** Welcher Typ wird als JSON zurückgegeben?
- **Errors:** Welche HTTP-Statuscodes können im Fehlerfall auftreten?

```zelyra
type TaskId = Id

table tasks {
    id: TaskId primary auto
    name: String required
}

api GET "/api/tasks/{id}" {
    input {
        id: TaskId
    }
    output Task
    errors {
        404 NotFound
    }
}

fn main() {
    print("API-Endpunkt definiert.")
}
```

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Ein GET-Endpunkt mit Rückgabetyp**
```zelyra
type TaskId = Id

table tasks {
    id: TaskId primary auto
    name: String required
}

api GET "/tasks/{id}" {
    input {
        id: TaskId
    }
    output Task
    errors {
        404 NotFound
    }
}

fn main() {
    print("GET API geprueft.")
}
```

**Beispiel 2: Ein POST-Endpunkt zum Anlegen von Daten**
```zelyra
type TaskId = Id

table tasks {
    id: TaskId primary auto
    name: String(100) required
}

api POST "/tasks" {
    input {
        name: String
    }
    output Task
    errors {
        400 ValidationError
    }
}

fn main() {
    print("POST API geprueft.")
}
```

**Beispiel 3: OpenAPI-Dokumentation erzeugen**
Mit dem Befehl:
```bash
zelyra doc main.zyl --openapi
```
generiert Zelyra eine normgerechte `openapi.json`, die du direkt in Swagger-UI oder Postman importieren kannst.

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Den Ausgabetyp `output` mit einem unbekannten Typen belegen.
  *Ursache:* Zelyra prüft, ob der Datentyp (z. B. `Task`) als Tabelle oder Record existiert.
- **Fehler:** Nicht deklarierte HTTP-Fehlercodes zurücksenden wollen.
  *Ursache:* Deklarierte Schnittstellen fordern vollständige Spezifikation aller Statuscodes.

### 6. Merksätze
1. `api METHOD "/pfad"` deklariert eine typsichere REST-Schnittstelle.
2. `input`, `output` und `errors` beschreiben den Datenvertrag der Schnittstelle lückenlos.
3. `zelyra doc --openapi` erzeugt OpenAPI-Spezifikationen direkt aus dem Quelltext.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Definiere einen Endpunkt `GET "/api/version"`, der einen Versions-String liefert.
- **Stufe 2 (Mittel):** Erstelle einen Endpunkt `DELETE "/api/tasks/{id}"` mit Fehlercode `404 NotFound`.
- **Stufe 3 (Anspruchsvoll):** Generiere mit `zelyra doc --openapi` eine API-Dokumentation und untersuche die JSON-Ausgabe.

### 8. Praxisaufgabe: Die REST-API für unsere Aufgabenverwaltung
Definiere die öffentliche Programmierschnittstelle unserer Aufgabenverwaltung:
```zelyra
type TaskId = Id

table tasks {
    id: TaskId primary auto
    name: String(120) required
    erledigt: Bool
}

api GET "/api/tasks/{id}" {
    input {
        id: TaskId
    }
    output Task
    errors {
        404 NotFound
    }
}

api POST "/api/tasks" {
    input {
        name: String
    }
    output Task
    errors {
        400 ValidationError
    }
}

fn main() {
    print("Aufgaben REST-API einsatzbereit.")
}
```

### 9. Zusammenfassung
- APIs in Zelyra sind typsicher, selbstdokumentierend und standardkonform.
- Mit minimalem Aufwand entstehen robuste Endpunkte für Web- und Mobilanwendungen.

### 10. Kontrollfragen zur Selbstprüfung
1. Welche vier Abschnitte umfasst eine vollständige `api`-Deklaration?
2. Warum ist die automatische OpenAPI-Generierung gegenüber manuellen Dokumentationen überlegen?
3. Was geschieht, wenn ein Client ungültige Daten an einen `input`-Block sendet?

# TEIL VIII – DIE BESONDERHEITEN VON ZELYRA

---

## Kapitel 31: Lesbarkeit als oberstes Gebot

### 1. Was lerne ich in diesem Kapitel?
In diesem Kapitel lernst du:
- Warum Code in der Realität bis zu zehnmal öfter gelesen als geschrieben wird.
- Welche Designentscheidungen Zelyra bewusst getroffen hat, um maximale Klarheit zu schaffen.
- Warum Zelyra auf unlesbare Syntaxakrobatik und kryptische Symbole verzichtet.
- Wie der integrierte Code-Formatierer `zelyra fmt` für einen einheitlichen Standard sorgt.

### 2. Warum ist das Thema wichtig?
Viele Programmiersprachen erlauben es, denselben Sachverhalt auf zehn verschiedene Arten auszudrücken – oft in ultrakompakten Einzeilern mit Sonderzeichen, die nach drei Monaten niemand mehr versteht. In großen Teams und langfristigen Projekten führt das zu enormen Wartungskosten. Zelyra folgt dem Grundsatz: Es gibt genau einen offensichtlichen, klaren Weg, eine Aufgabe zu lösen.

### 3. Verständliche Erklärung
Lesbarkeit in Zelyra bedeutet:
- **Ausdrückliche Namen statt Abkürzungen:** Funktionen und Variablen sprechen Klartext (`prioritaet` statt `prio_lvl_fn()`).
- **Klare Blöcke:** Jede Bedingung, Schleife und Funktion besitzt eindeutige geschweifte Klammern.
- **Automatische Formatierung:** Niemand im Team muss über Einrückungen oder Leerzeichen diskutieren. Der Befehl `zelyra fmt` rückt jede Zeile exakt nach dem einheitlichen Zelyra-Standard ein.

```zelyra
fn berechne_gesamtzeit(aufgaben_dauern: Int[]) -> Int {
    mutable summe = 0
    for dauer in aufgaben_dauern {
        summe = summe + dauer
    }
    return summe
}

fn main() {
    zeiten: Int[] = [15, 30, 45]
    print(berechne_gesamtzeit(zeiten))
}
```

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Selbsterklärende Funktionssignaturen**
```zelyra
fn ist_aufgabe_ueberfaellig(frist_tage: Int) -> Bool {
    return frist_tage < 0
}

fn main() {
    print(ist_aufgabe_ueberfaellig(-2))
}
```

**Beispiel 2: Verständliche Kontrollstrukturen**
```zelyra
fn status_anzeige(status_code: Int) -> String {
    match status_code {
        1 => {
            return "Neu"
        }
        2 => {
            return "In Bearbeitung"
        }
        3 => {
            return "Erledigt"
        }
        _ => {
            return "Unbekannt"
        }
    }
}

fn main() {
    print(status_anzeige(2))
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Variablen mit einzelnen Buchstaben (`a`, `x`, `tmp`) benennen, deren Bedeutung unklar ist.
  *Ursache:* Zelyra-Code soll wie verständliche Prosa gelesen werden können.
- **Fehler:** Uneinheitliche Einrückungen manuell korrigieren.
  *Ursache:* Führe einfach `zelyra fmt main.zyl` aus – der Compiler formatiert alles automatisch.

### 6. Merksätze
1. Schreibe Code für den Menschen, der ihn in sechs Monaten warten muss.
2. `zelyra fmt` garantiert einen einheitlichen, lesbaren Programmierstil im gesamten Projekt.
3. Klare, sprechende Bezeichner sind die beste Dokumentation.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Formatiere eine unordentliche Quellcodedatei mit `zelyra fmt`.
- **Stufe 2 (Mittel):** Refaktoriere eine Funktion mit unklaren Variablennamen zu sprechendem Zelyra-Code.
- **Stufe 3 (Anspruchsvoll):** Schreibe eine komplexe Berechnungsfunktion so sauber gegliedert, dass keine einzige Zeile mehr als 80 Zeichen benötigt.

### 8. Praxisaufgabe: Lesbare Status- und Filterlogik
Gestalte die Filterlogik unserer Aufgabenverwaltung maximal verständlich:
```zelyra
fn ist_dringend_und_offen(prioritaet: Int, erledigt: Bool) -> Bool {
    ist_prioritaet_eins = prioritaet == 1
    ist_noch_nicht_erledigt = !erledigt
    return ist_prioritaet_eins && ist_noch_nicht_erledigt
}

fn main() {
    print(ist_dringend_und_offen(1, false))
    print(ist_dringend_und_offen(2, false))
}
```

### 9. Zusammenfassung
- Lesbarkeit ist der wichtigste Schutzfaktor gegen schleichende Software-Fäulnis.
- Zelyra erzwingt Klarheit durch Sprachdesign und Werkzeuge.

### 10. Kontrollfragen zur Selbstprüfung
1. Warum spart gut lesbarer Code auf lange Sicht Zeit und Geld?
2. Welcher Zelyra-CLI-Befehl formatiert Quelldateien automatisch?
3. Warum verzichtet Zelyra auf übermäßig viele alternative Schreibweisen für dieselbe Logik?

---

## Kapitel 32: KI-Nativität – Warum Zelyra perfekt für KI-Assistenten ist

### 1. Was lerne ich in diesem Kapitel?
- Was eine „KI-native Programmiersprache“ bedeutet.
- Warum moderne LLMs (wie Claude, GPT, Gemini) bei Zelyra weniger halluzinieren.
- Wie typisierte Löcher (*Typed Holes* `_`) Entwicklern und KIs bei der Codegenerierung helfen.
- Wie CLI-Befehle mit `--format json` strukturierte Schnittstellen für Werkzeuge bieten.
- Wie `zelyra impact` und `zelyra edit` automatisierte Code-Änderungen absichern.

### 2. Warum ist das Thema wichtig?
Die meisten Programmiersprachen wurden vor Jahrzehnten entwickelt – ausschließlich für menschliche Tastatureingaben. Wenn moderne KI-Assistenten Code in Python oder JavaScript schreiben, erfinden sie oft Methoden, verwechseln Typen oder übersehen Seiteneffekte. Zelyra wurde von Grund auf so entworfen, dass menschliche Entwickler und KI-Assistenten optimal zusammenarbeiten können.

### 3. Verständliche Erklärung
Zelyra unterstützt KI-Entwicklung durch vier Schlüsselmerkmale:
1. **Eindeutige, kontextfreie Grammatik:** Die Sprache hat keine Mehrdeutigkeiten.
2. **Maschinenlesbare JSON-Ausgabe:** Fast alle Befehle bieten `--format json` (z. B. `zelyra check --format json`), sodass KIs Fehlermeldungen direkt als strukturierte Daten erfassen.
3. **Typed Holes (`_`):** Wenn du oder die KI nicht genau wissen, wie ein Wert berechnet wird, setzt man einen Unterstrich `_` ein. Der Compiler meldet sofort exakt: Welcher Typ wird erwartet? Welche Variablen sind verfügbar? Welche Verträge gelten?
4. **Auswirkungsanalyse (`zelyra impact`):** Zelyra berechnet vorab genau, welche Programmteile von einer Änderung betroffen sind.

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Typisierte Verträge leiten die KI fehlerfrei**
Durch Verträge weiß die KI ohne Raten, welche Randbedingungen gelten:
```zelyra
fn normalisiere_skala(wert: Int) -> Int
    requires { wert >= 0 && wert <= 100 }
    ensures { result >= 0 && result <= 10 }
{
    return wert / 10
}

fn main() {
    print(normalisiere_skala(85))
}
```

**Beispiel 2: Maschinenlesbare Fehleranalyse**
Führt ein KI-Tool den Befehl:
```bash
zelyra check main.zyl --format json
```
aus, erhält es präzise JSON-Objekte mit exaktem Fehlercode, Zeile, Spalte und Lösungshinweis.

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Typed Holes (`_`) in Produktionscode belassen.
  *Ursache:* Typed Holes sind Entwicklungshilfen. Vor dem finalen Kompilieren müssen alle `_` durch gültigen Code ersetzt werden.
- **Fehler:** KI-Code ohne `zelyra check` ungeprüft übernehmen.
  *Ursache:* Nutze immer den Zelyra-Compiler als unbestechlichen Richter.

### 6. Merksätze
1. Zelyra ist die erste KI-native Sprache: Eindeutig, strukturiert und werkzeugfreundlich.
2. Typed Holes `_` dienen als präzise Arbeitsaufträge an den Compiler und KI-Assistenten.
3. `--format json` ermöglicht nahtlose Integration in moderne KI-Agenten und IDEs.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Führe `zelyra check` mit der Option `--format json` aus und studiere die Ausgabe.
- **Stufe 2 (Mittel):** Analysiere eine Datei mit `zelyra impact`.
- **Stufe 3 (Anspruchsvoll):** Setze in einer Funktion ein Typed Hole `_` ein und beobachte die detaillierten Kontextinformationen des Compilers.

### 8. Praxisaufgabe: KI-unterstützte Erweiterung der Aufgabenverwaltung
Schreibe eine wohlstrukturierte Funktionssignatur mit Vertrag, die sich perfekt von einer KI vervollständigen lässt:
```zelyra
fn berechne_restzeit(ziel_stunde: Int, aktuelle_stunde: Int) -> Int
    requires { ziel_stunde >= aktuelle_stunde }
    ensures { result >= 0 }
{
    return ziel_stunde - aktuelle_stunde
}

fn main() {
    rest = berechne_restzeit(18, 14)
    print(rest)
}
```

### 9. Zusammenfassung
- Zelyra beseitigt Sprach-Mehrdeutigkeiten, die KI-Systeme traditionell verwirren.
- Typed Holes und strukturierte JSON-Ausgaben machen Pair-Programming mit KIs extrem zuverlässig.

### 10. Kontrollfragen zur Selbstprüfung
1. Wofür steht das Konzept der „Typed Holes“ in Zelyra?
2. Warum profitieren KI-Systeme von der `--format json`-Option des Compilers?
3. Wie helfen Verträge (`requires`, `ensures`) einer KI beim Erzeugen von korrektem Code?

---

## Kapitel 33: Sicherheit durch Fähigkeiten (Capabilities)

### 1. Was lerne ich in diesem Kapitel?
- Was das Capability-Sicherheitsmodell ist und warum es herkömmlichen Berechtigungskonzepten überlegen ist.
- Die fünf Kernfähigkeiten: `FileSystem`, `Database`, `Network`, `Process`, `Environment`.
- Wie Capabilities deklariert, vererbt und in `zelyra.toml` beschränkt werden.
- Warum Zelyra gegen Supply-Chain-Angriffe (bösartige Pakete) immun ist.

### 2. Warum ist das Thema wichtig?
In heutigen Ökosystemen wie npm (JavaScript) oder PyPI (Python) bindet man oft hunderte Bibliotheken von Drittanbietern ein. Wenn ein Paket manipuliert wird, kann es unbemerkt Passwörter auslesen, Dateien verschlüsseln oder Daten ins Internet funken. In Zelyra ist das unmöglich: Eine Funktion kann ohne ausdrückliche Deklaration von `uses Network` kein einziges Byte übers Netz senden – der Compiler verweigert den Dienst!

### 3. Verständliche Erklärung
Stell dir Zelyras Capability-System wie ein Sicherheitsschloss vor:
- Wenn eine Funktion auf die Festplatte schreiben will, muss sie den Schlüssel `uses FileSystem` am Revers tragen.
- Hat sie diesen Schlüssel nicht, kann sie keine Dateien anrühren – selbst wenn sie es versucht.
- Und das Beste: Wenn Funktion A die Funktion B aufruft, muss auch A die Berechtigung deklarieren. So siehst du auf den ersten Blick in `main()`, was das gesamte Programm überhaupt darf!

```zelyra
fn sichere_operation() {
    // Diese Funktion hat KEINE Capabilities.
    // Sie kann unmoeglich Schaden auf der Festplatte oder im Netzwerk anrichten!
    print("Garantiert seiteneffektfrei.")
}

fn main() {
    sichere_operation()
}
```

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Saubere Deklaration von Umgebungsvariablen**
```zelyra
fn lese_konfiguration(schluessel: String) -> Option<String>
    uses Environment
{
    return env(schluessel)
}

fn main() uses Environment {
    print("Konfigurationszugriff erlaubt.")
}
```

**Beispiel 2: Datenbankzugriff mit Capability**
```zelyra
database main {
    engine: mariadb
    database: "tasks_db"
}

table tasks {
    id: Id primary auto
    name: String required
}

fn lade_daten() uses Database {
    daten = sql<Task[]> {
        SELECT id, name
        FROM tasks
    }
    print("Datenbankzugriff gewaehrt.")
}

fn main() uses Database {
    lade_daten()
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Vergessen, Capabilities an den Aufrufer weiterzureichen.
  *Ursache:* Wenn Funktion `a()` die Funktion `b() uses FileSystem` aufruft, muss auch `a()` mit `uses FileSystem` versehen sein.
- **Fehler:** Fehlende Freigabe in `zelyra.toml`.
  *Ursache:* Das Projekt manifestiert seine maximalen Rechte in der Konfiguration.

### 6. Merksätze
1. Keine Funktion kann heimlich auf Dateien, Datenbanken oder das Netzwerk zugreifen.
2. Alle Seiteneffekte sind transparent in der Funktionssignatur dokumentiert.
3. Reine Funktionen ohne Capabilities sind garantiert manipulationssicher.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Identifiziere in einem bestehenden Code alle Funktionen mit Capabilities.
- **Stufe 2 (Mittel):** Schreibe eine Funktion, die `uses Clock` und `uses Environment` kombiniert.
- **Stufe 3 (Anspruchsvoll):** Entwirf ein Programm so, dass alle Geschäftsrechenlogik in Funktionen völlig ohne Capabilities ausgelagert wird.

### 8. Praxisaufgabe: Berechtigungs-Architektur der Aufgabenverwaltung
Isoliere die Berechtigungen in unserer Aufgabenverwaltung:
```zelyra
// 1. Reine Logik: KEINE Berechtigungen noetig
fn ist_bereit_fuer_export(anzahl_aufgaben: Int) -> Bool {
    return anzahl_aufgaben > 0
}

// 2. I/O-Logik: Explizite Dateisystem-Berechtigung
fn fuehre_export_durch(datei: String, inhalt: String) uses FileSystem {
    write_text(datei, inhalt)
    print("Export vollzogen.")
}

fn main() uses FileSystem {
    if ist_bereit_fuer_export(5) {
        fuehre_export_durch("aufgaben.txt", "Aufgabe 1")
    }
}
```

### 9. Zusammenfassung
- Das Capability-Modell von Zelyra schützt vor bösartigen Bibliotheken und unkontrollierten Seiteneffekten.
- Software wird durch explizite Rechtevergabe von Grund auf sicher (*Secure by Design*).

### 10. Kontrollfragen zur Selbstprüfung
1. Welche fünf Systemfähigkeiten kennt Zelyra?
2. Warum müssen auch übergeordnete Aufrufer-Funktionen Capabilities deklarieren?
3. Wie schützt Zelyra vor schadhaftem Fremdcode aus Paketmanagern?

---

## Kapitel 34: Zelyra im Vergleich

### 1. Was lerne ich in diesem Kapitel?
- Wie sich Zelyra im direkten Vergleich zu Python, PHP/Laravel, Rust und TypeScript schlägt.
- Welche Stärken die jeweiligen Sprachen haben und warum Zelyra für moderne Web- & KI-Systeme maßgeschneidert wurde.
- Warum Zelyra die Typsicherheit von Rust mit der Entwicklungsgeschwindigkeit von Python verbindet.

### 2. Warum ist das Thema wichtig?
Keine Programmiersprache ist für jeden Zweck perfekt: C und Rust sind unschlagbar für Betriebssystemkerne, Python dominiert die Datenwissenschaft, JavaScript das Frontend. Wer aber moderne, datenbankgestützte Geschäftsanwendungen und Web-Backends bauen will, kämpft in diesen Sprachen oft mit historischem Ballast. Zelyra vereint das Beste aus diesen Welten.

### 3. Verständliche Erklärung: Der Sprachvergleich

| Merkmal | Python | PHP / Laravel | TypeScript / Node | Rust | Zelyra |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Typisierung** | Dynamisch | Dynamisch/Optional | Statisch (wird zu JS) | Statisch (sehr streng) | **Statisch & Eindeutig** |
| **Null-Sicherheit** | `None`-Crashes möglich | `null`-Crashes möglich | `undefined`-Crashes | Absolut (`Option`) | **Absolut (`Option`)** |
| **SQL-Integration** | ORM (Strings) | ORM (Eloquent) | ORM (Prisma/TypeORM) | Diesel/SQLx | **Nativ & Typgeprüft** |
| **Verträge (Contracts)**| Nein (nur `assert`) | Nein | Nein | Dritt-Bibliotheken | **Eingebaut (`requires`)** |
| **Sicherheits-Capabilities**| Nein (Vollzugriff) | Nein (Vollzugriff) | Nein (Vollzugriff) | Nein | **Eingebaut (`uses ...`)** |
| **KI-Werkzeugunterstützung**| Mittel (Mehrdeutig) | Mittel | Mittel | Schwer für KIs | **Nativ (JSON, Typed Holes)** |

### 4. Kleine, aufeinander aufbauende Beispiele

**Vergleich: Wie Zelyra Fehler verhindert, die in anderen Sprachen passieren**

*In Python/JS (potenzieller Laufzeitabsturz bei `null`):*
Ein unbemerkter fehlender Wert führt zum Servercrash: `AttributeError: 'NoneType' object has no attribute 'title'`.

*In Zelyra (garantiert abgefangen zur Compilezeit):*
```zelyra
fn zeige_titel(opt_titel: Option<String>) {
    match opt_titel {
        Some(t) => {
            print("Titel: " + t)
        }
        None => {
            print("Kein Titel vorhanden.")
        }
    }
}

fn main() {
    zeige_titel(Some("Projekt X"))
    zeige_titel(None)
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Zelyra wie Python schreiben wollen (Einrückung statt Klammern, dynamische Typänderung).
  *Ursache:* Zelyra nutzt geschweifte Klammern und verlangt statische Typstabilität.
- **Fehler:** Zelyra mit Low-Level-Rust verwechseln (komplexe Lifetime-Annotationen suchen).
  *Ursache:* Zelyra nimmt Entwicklern die Speicherverwaltung vollautomatisch ab.

### 6. Merksätze
1. Zelyra vereint die Einfachheit von Skriptsprachen mit der Unbestechlichkeit statischer Typsysteme.
2. Datenbanken, Webformulare und Schnittstellen sind native Sprachbausteine statt externer Bibliotheken.
3. Sicherheit und Korrektheit werden nicht nachträglich hineingetestet, sondern von Anfang an erzwungen.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Übersetze eine einfache Python-Berechnungsfunktion in sauberen Zelyra-Code.
- **Stufe 2 (Mittel):** Vergleiche eine Datenbankabfrage in PHP/Laravel Eloquent mit Zelyras `sql<T[]>`.
- **Stufe 3 (Anspruchsvoll):** Diskutiere anhand eines Praxisbeispiels, warum Zelyras Capabilities Supply-Chain-Angriffe verhindern.

### 8. Praxisaufgabe: Die Aufgabenverwaltung als Zelyra-Vorzeigeprojekt
Führe alle Kernstärken in einem prägnanten Ausschnitt zusammen:
```zelyra
database main {
    engine: mariadb
    database: "tasks_demo"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    erledigt: Bool default false
}

fn zaehle_offene() -> Int uses Database {
    offene = sql<Task[]> {
        SELECT id, name, erledigt
        FROM tasks
        WHERE erledigt = false
    }
    return len(offene)
}

fn main() uses Database {
    anzahl = zaehle_offene()
    print("Offene Aufgaben ermittelt.")
}
```

### 9. Zusammenfassung
- Zelyra schließt die Lücke zwischen zu komplexen System-Sprachen und zu fehleranfälligen Skriptsprachen.
- Für moderne Web-, Daten- und KI-Anwendungen bietet Zelyra eine unübertroffen robuste Plattform.

### 10. Kontrollfragen zur Selbstprüfung
1. Welchen Vorteil bietet Zelyras `Option`-Typ gegenüber Pythons `None` oder JavaScripts `null`?
2. Warum ist die native Datenbankintegration in Zelyra sicherer als traditionelle ORM-Bibliotheken?
3. Welche Rolle spielen Capabilities beim Schutz vor bösartigen Fremdpaketen?

# TEIL IX – VOM ENTWURF ZUR FERTIGEN ANWENDUNG

---

## Kapitel 35: Software planen – Von der Idee zum Entwurf

### 1. Was lerne ich in diesem Kapitel?
In diesem Kapitel lernst du:
- Wie du von einer vagen Idee zu einem präzisen, umsetzbaren Software-Entwurf gelangst.
- Wie man Entitäten und deren Beziehungen auf Papier oder im Editor skizziert.
- Warum die frühe Definition des Schemas in Zelyra den gesamten weiteren Entwicklungsverlauf vereinfacht.
- Wie man Anforderungen in kleine, testbare Meilensteine zerlegt.

### 2. Warum ist das Thema wichtig?
Der größte Fehler von Anfängern (und unvorsichtigen Profis) ist es, sofort loszutippen, ohne den Datenfluss zu planen. Wer während des Programmierens merkt, dass ein zentrales Tabellenfeld oder eine Beziehung fehlt, muss oft Tage damit verbringen, bestehenden Code mühsam umzuschreiben. Zelyra belohnt gründliche Planung: Sobald dein `table`-Schema steht, leiten sich Formulare, Validierungen und APIs fast wie von selbst ab.

### 3. Verständliche Erklärung
Jedes gute Softwareprojekt durchläuft vier Planungsphasen:
1. **Zweck und Zielgruppe klären:** Wer nutzt das System? Welche Kernaufgabe muss gelöst werden? (z. B. „Ein Teamleiter möchte Aufgaben anlegen, zuweisen und als erledigt markieren“).
2. **Datenmodell entwerfen:** Welche Objekte gibt es? Welche Felder sind Pflicht? (z. B. `tasks` mit `name`, `prioritaet`, `ist_erledigt`).
3. **Sicherheits- und Zugriffsregeln:** Wer darf was tun? Benötigen wir Authentifizierung?
4. **Schrittweise Implementierung:** Erst das Schema (`table`), dann die Logik (`fn`), dann die Webansichten (`crud`/`page`).

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Der erste Meilenstein – Das Datenmodell**
```zelyra
database main {
    engine: mariadb
    database: "aufgaben_planer"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    prioritaet: Int default 2
    erledigt: Bool default false
}

fn main() {
    print("Planungsschritt 1: Schema steht.")
}
```

**Beispiel 2: Planung der Geschäftslogik als reine Funktionen**
```zelyra
fn validiere_frist(tage: Int) -> Bool {
    return tage >= 0
}

fn main() {
    print(validiere_frist(3))
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Alles auf einmal programmieren wollen, bevor die Grundlagen getestet sind.
  *Ursache:* Baue Software schrittweise: Teste jede Funktion sofort mit `zelyra check`.
- **Fehler:** Unklare Pflichtfelder im Datenmodell.
  *Ursache:* Lege von Anfang an fest, welche Felder `required` sind und welche leer sein dürfen (`Option`).

### 6. Merksätze
1. Wer die Planung vernachlässigt, plant das Scheitern.
2. Ein klares Datenmodell ist das Rückgrat jeder erfolgreichen Anwendung.
3. Zerlege große Probleme in kleine, unabhängig überprüfbare Funktionen.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Skizziere den Funktionsumfang einer einfachen Notiz-App in Stichpunkten.
- **Stufe 2 (Mittel):** Entwirf ein Tabellenschema für Benutzer, Aufgaben und Kategorien mit passenden Datentypen.
- **Stufe 3 (Anspruchsvoll):** Formuliere für alle Kernfunktionen deiner geplanten App Vor- und Nachbedingungen (`requires`, `ensures`).

### 8. Praxisaufgabe: Der vollständige Architekturplan unserer Aufgabenverwaltung
Führe alle Planungselemente der Aufgabenverwaltung zusammen:
```zelyra
database main {
    engine: mariadb
    database: "tasks_pro"
}

table tasks {
    id: Id primary auto
    name: String(120) required
    prioritaet: Int default 1
    erledigt: Bool default false
}

fn berechne_dringlichkeit(prioritaet: Int, verbleibende_tage: Int) -> String {
    if prioritaet == 1 {
        return "HOECHSTE PRIORITAET"
    }
    if verbleibende_tage <= 1 {
        return "DRINGEND WEGEN FRIST"
    }
    return "NORMAL"
}

fn main() {
    status = berechne_dringlichkeit(1, 5)
    print("Architektur-Plan verifiziert: " + status)
}
```

### 9. Zusammenfassung
- Strukturierte Planung spart Entwicklungszeit und verhindert Architekturfehler.
- Zelyras deklarative Sprachstruktur passt sich nahtlos an agile Planungsphasen an.

### 10. Kontrollfragen zur Selbstprüfung
1. Welche vier Phasen durchläuft ein professioneller Software-Entwurf?
2. Warum sollte das Datenmodell vor der Benutzeroberfläche entworfen werden?
3. Wie helfen Vorbedingungen bei der präzisen Anforderungsdefinition?

---

## Kapitel 36: Architektur und saubere Codestruktur

### 1. Was lerne ich in diesem Kapitel?
- Wie du deinen Zelyra-Code nach dem bewährten Schichtenmodell strukturierst.
- Die klare Trennung von Persistenz (`table`), Geschäftslogik (`fn`) und Darstellung (`page`, `crud`).
- Wie du Kopplungen vermeidest und Module wartungsfreundlich hältst.
- Warum saubere Architektur vor bösen Überraschungen bei späteren Erweiterungen schützt.

### 2. Warum ist das Thema wichtig?
Wenn Datenzugriff, Geschäftsregeln und HTML-Ausgabe wild durcheinandergewürfelt werden, entsteht unwartbarer Code. Wenn sich später das Datenbanklayout ändert, zerbricht plötzlich die Weboberfläche. Eine saubere Architektur zieht klare Trennlinien: Jede Schicht hat eine einzige Verantwortlichkeit.

### 3. Verständliche Erklärung
Eine saubere Zelyra-Anwendung gliedert sich in drei klare Schichten:
1. **Daten- und Persistenzschicht:** Tabellendefinitionen (`table`) und typisierte SQL-Abfragen.
2. **Geschäftslogikschicht:** Reine Rechen- und Validierungsfunktionen mit Verträgen (`requires`, `ensures`).
3. **Präsentations- und Schnittstellenschicht:** Weboberflächen (`crud`, `page`) und REST-Endpunkte (`api`).

```zelyra
// 1. Datenmodell
table tasks {
    id: Id primary auto
    name: String required
}

// 2. Geschäftslogik
fn formatiere_name(rohtext: String) -> String {
    return "[AUFGABE] " + rohtext
}

// 3. Einstieg / Ablauf
fn main() {
    print(formatiere_name("Server pruefen"))
}
```

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Trennung von Logik und I/O**
```zelyra
// Reine Rechenfunktion: Keine Capabilities noetig
fn berechne_prozent(wert: Int, max_wert: Int) -> Int
    requires { max_wert > 0 && wert >= 0 }
{
    return (wert * 100) / max_wert
}

// I/O-Funktion: Nutzt die Logik und gibt sie aus
fn main() {
    prozent = berechne_prozent(45, 50)
    print(prozent)
}
```

**Beispiel 2: Strukturierung durch aussagekräftige Namen**
```zelyra
table settings {
    id: Id primary auto
    app_name: String(60) required
}

fn zeige_systeminfo(name: String) {
    print("System laeuft: " + name)
}

fn main() {
    zeige_systeminfo("Zelyra Task Suite")
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Geschäftslogik direkt in SQL-Strings oder HTML-Blöcke stopfen.
  *Ursache:* Trenne Berechnungen in eigene Hilfsfunktionen aus, damit sie unabhängig testbar bleiben.
- **Fehler:** Zirkuläre Abhängigkeiten erzeugen.
  *Ursache:* Der Datenfluss sollte immer von oben nach unten verlaufen (Präsentation -> Logik -> Daten).

### 6. Merksätze
1. Trenne Datenmodell, Geschäftsregeln und Darstellung strikt voneinander.
2. Geschäftslogik sollte möglichst frei von Seiteneffekten und Capabilities sein.
3. Saubere Schichten machen Anwendungen zukunftssicher und einfach erweiterbar.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Identifiziere in einem bestehenden Codebeispiel die drei Schichten.
- **Stufe 2 (Mittel):** Lagere alle Berechnungen aus einer Web-Route in separate reine Funktionen aus.
- **Stufe 3 (Anspruchsvoll):** Entwirf ein Schichtenmodell für ein Zeiterfassungssystem.

### 8. Praxisaufgabe: Schichtenarchitektur für die Aufgabenverwaltung
Implementiere das Schichtenmodell für unsere Aufgabenverwaltung:
```zelyra
database main {
    engine: mariadb
    database: "tasks_architecture"
}

// Schicht 1: Persistenz
table tasks {
    id: Id primary auto
    name: String required
    erledigt: Bool default false
}

// Schicht 2: Geschaeftslogik
fn ist_aufgabe_wichtig(name: String, dringend: Bool) -> Bool {
    return dringend
}

// Schicht 3: Anwendung / Ausfuehrung
fn main() uses Database {
    wichtig = ist_aufgabe_wichtig("Steuern einreichen", true)
    print("Aufgaben-Architektur geprueft.")
}
```

### 9. Zusammenfassung
- Das Drei-Schichten-Modell garantiert Übersichtlichkeit und langfristige Wartbarkeit.
- Zelyra unterstützt diese Struktur auf natürliche Weise durch sein klares Typsystem.

### 10. Kontrollfragen zur Selbstprüfung
1. Welche drei Schichten bilden das Fundament einer sauberen Zelyra-Anwendung?
2. Warum sollte Kern-Geschäftslogik möglichst ohne Capabilities auskommen?
3. Welche Vorteile bietet die Trennung von Präsentation und Datenzugriff bei Re-Designs?

---

## Kapitel 37: Konfiguration und Umgebungsvariablen

### 1. Was lerne ich in diesem Kapitel?
- Wie Konfigurationswerte sicher über Umgebungsvariablen (`.env`) verwaltet werden.
- Die Bibliotheksfunktion `env(schluessel)` und die benötigte Fähigkeit `uses Environment`.
- Wie du mit dem `Option<String>`-Rückgabewert von `env()` sicher umgehst.
- Warum Passwörter und API-Schlüssel niemals im Quelltext stehen dürfen.

### 2. Warum ist das Thema wichtig?
Einer der schwersten Sicherheitsverstöße ist das Versehentliche Einchecken von Datenbank-Passwörtern oder geheimen API-Keys in öffentliche Git-Repositories. Zudem muss sich eine Anwendung in Entwicklung, Test und Produktion unterschiedlich verhalten (z. B. andere Datenbank-Hosts). Umgebungsvariablen trennen Code und geheime Konfiguration sauber voneinander.

### 3. Verständliche Erklärung
In Zelyra greifst du über die Funktion `env()` auf Umgebungsvariablen zu.
Weil eine Variable in der Umgebung existieren kann oder fehlen kann, liefert `env()` immer ein `Option<String>` zurück:

```zelyra
fn lese_port() -> String uses Environment {
    opt_port = env("APP_PORT")
    match opt_port {
        Some(p) => {
            return p
        }
        None => {
            return "8080"
        }
    }
}

fn main() uses Environment {
    port = lese_port()
    print("Server lauscht auf Port: " + port)
}
```

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Datenbank-Host konfigurieren**
```zelyra
fn hole_db_host() -> String uses Environment {
    match env("DB_HOST") {
        Some(host) => {
            return host
        }
        None => {
            return "127.0.0.1"
        }
    }
}

fn main() uses Environment {
    print("Verbinde mit: " + hole_db_host())
}
```

**Beispiel 2: Debug-Modus dynamisch abfragen**
```zelyra
fn ist_debug_aktiv() -> Bool uses Environment {
    match env("APP_DEBUG") {
        Some(wert) => {
            return wert == "true"
        }
        None => {
            return false
        }
    }
}

fn main() uses Environment {
    if ist_debug_aktiv() {
        print("Debug-Modus ist AN")
    } else {
        print("Debug-Modus ist AUS")
    }
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Geheime Passwörter fest in `.zyl`-Dateien hineinschreiben.
  *Ursache:* Nutze immer eine `.env`-Datei und lese sensible Werte mit `env()`.
- **Fehler:** `env()` ohne `uses Environment` aufrufen.
  *Ursache:* Zelyras Sicherheitssystem schützt Systemumgebungen vor unbefugtem Zugriff.

### 6. Merksätze
1. Sensible Zugangsdaten gehören niemals in den Versionskontroll-Quelltext.
2. `env(name)` liefert ein `Option<String>` und erfordert `uses Environment`.
3. Stelle für fehlende Umgebungsvariablen immer sichere Standardwerte bereit.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Lese eine Umgebungsvariable `USER_NAME` aus und gib eine persönliche Begrüßung aus.
- **Stufe 2 (Mittel):** Schreibe eine Hilfsfunktion `hole_env_oder_standard(schluessel: String, standard: String) -> String`.
- **Stufe 3 (Anspruchsvoll):** Konfiguriere ein Programm so, dass es zwischen Entwicklungs- und Produktions-Modus umschaltet.

### 8. Praxisaufgabe: Konfigurationszentrale für die Aufgabenverwaltung
Schreibe den Konfigurations-Loader für unsere Aufgabenverwaltung:
```zelyra
fn lade_app_titel() -> String uses Environment {
    match env("APP_TITLE") {
        Some(titel) => {
            return titel
        }
        None => {
            return "Zelyra Aufgabenverwaltung 0.1"
        }
    }
}

fn main() uses Environment {
    print("System gestartet: " + lade_app_titel())
}
```

### 9. Zusammenfassung
- Umgebungsvariablen ermöglichen flexible, sichere Konfiguration für verschiedene Serverumgebungen.
- Zelyras `Option`-Typ zwingt dich, das Fehlen von Einstellungen elegant zu behandeln.

### 10. Kontrollfragen zur Selbstprüfung
1. Warum dürfen geheime API-Schlüssel niemals im Quelltext fest hinterlegt sein?
2. Welchen Datentyp liefert die Funktion `env()` zurück?
3. Welche Capability wird für den Zugriff auf Umgebungsvariablen benötigt?

---

## Kapitel 38: Fehlersuche und Optimierung

### 1. Was lerne ich in diesem Kapitel?
- Wie du mit dem CLI-Befehl `zelyra doctor` dein Projekt auf Herz und Nieren prüfst.
- Wie man Compilerhinweise und Diagnosen effektiv nutzt.
- Strategien zum systematischen Aufspüren von Fehlern (*Debugging*).
- Wie du Performance-Engpässe erkennst und eliminierst.

### 2. Warum ist das Thema wichtig?
Selbst bei sorgfältigster Programmierung läuft nicht immer alles auf Anhieb glatt. Vielleicht ist der Datenbankport blockiert, eine Konfiguration unvollständig oder eine Schleife berechnet unnötige Schritte. Wer planlos herumprobiert, verliert Stunden. Systematisches Debugging mit den passenden Zelyra-Werkzeugen führt dagegen in wenigen Minuten zur Lösung.

### 3. Verständliche Erklärung
Zelyra gibt dir ein Schweizer Taschenmesser für die Fehlerdiagnose an die Hand:
- `zelyra check`: Prüft Syntax, Typen, Berechtigungen und Verträge.
- `zelyra doctor`: Prüft die Systemumgebung, Datenbankverbindungen, Ports und Richtlinien.
- `zelyra impact`: Zeigt an, welche Funktionen durch eine geplante Änderung beeinflusst werden.

```bash
zelyra doctor main.zyl
```
Wenn die MariaDB-Verbindung nicht erreichbar ist, meldet `doctor` sofort die Ursache, anstatt dich im Unklaren zu lassen.

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Systematische Kontrollausgaben**
```zelyra
fn berechne_summe(zahlen: Int[]) -> Int {
    mutable summe = 0
    for z in zahlen {
        summe = summe + z
    }
    return summe
}

fn main() {
    werte: Int[] = [10, 20, 30]
    ergebnis = berechne_summe(werte)
    print("Berechnetes Ergebnis:")
    print(ergebnis)
}
```

**Beispiel 2: Absicherung vor Endlosschleifen durch Invarianten**
```zelyra
fn sichere_zaehlung(grenze: Int) -> Int
    requires { grenze > 0 }
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
    print(sichere_zaehlung(10))
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Bei unerwartetem Verhalten wahllos Zeilen im Code ändern.
  *Ursache:* Lokalisiere das Problem erst exakt mit `zelyra check` und Ausgaben.
- **Fehler:** Datenbankfehler vermuten, wenn lediglich Berechtigungen in `zelyra.toml` fehlen.
  *Ursache:* Führe `zelyra doctor` aus, um Umgebungsfehler sofort zu erkennen.

### 6. Merksätze
1. `zelyra doctor` ist der erste Schritt bei Verbindungsproblemen und Umgebungsfehlern.
2. Invarianten und Verträge verhindern logische Fehlberechnungen.
3. Systematische Fehlersuche ist schneller und sicherer als blindes Ausprobieren.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Führe `zelyra doctor` für dein Aufgaben-Projekt aus.
- **Stufe 2 (Mittel):** Isoliere eine bewusst fehlerhafte Berechnung in einer Testfunktion.
- **Stufe 3 (Anspruchsvoll):** Schreibe ein Programm mit detaillierten Statusmeldungen für jeden Einzelschritt.

### 8. Praxisaufgabe: Selbstdiagnose-Routine der Aufgabenverwaltung
Baue einen internen Gesundheits-Check für die Aufgabenverwaltung:
```zelyra
fn fuehre_selbsttest_durch() -> Bool {
    test_ok = 1 + 1 == 2
    return test_ok
}

fn main() {
    print("Starte System-Selbsttest...")
    if fuehre_selbsttest_durch() {
        print("[OK] System arbeitet einwandfrei.")
    } else {
        print("[FEHLER] Interner Systemfehler.")
    }
}
```

### 9. Zusammenfassung
- Zelyras Werkzeugkette (`doctor`, `check`, `impact`) liefert schnelle Klarheit bei Fehlern.
- Verträge und Invarianten fangen Probleme ab, bevor sie zu schwer auffindbaren Bugs werden.

### 10. Kontrollfragen zur Selbstprüfung
1. Welche Aspekte überprüft der CLI-Befehl `zelyra doctor`?
2. Wie grenzt man einen Fehler in einer längeren Berechnung am besten ein?
3. Welche Rolle spielen Schleifeninvarianten bei der Fehlersuche?

---

## Kapitel 39: Bereitstellung und Betrieb

### 1. Was lerne ich in diesem Kapitel?
- Wie Zelyra-Anwendungen für den Produktivbetrieb bereitgestellt (*deployed*) werden.
- Wie du deine Webanwendung im Docker-Container betreibst.
- Wie Zelyra mit einer Produktions-MariaDB verbunden wird.
- Wie du den Server-Prozess mit `zelyra serve` zuverlässig am Laufen hältst.

### 2. Warum ist das Thema wichtig?
Eine Software nützt niemandem, wenn sie nur auf dem Entwickler-Laptop läuft. Sie muss auf einem Server oder in der Cloud rund um die Uhr stabil erreichbar sein. In anderen Umgebungen erfordert das Deployment oft komplizierte Anleitungen mit PHP-FPM, Webserver-Vhosts und Prozessmanagern. Zelyra vereinfacht den Betrieb radikal: Eine einzelne Konfiguration und ein schlanker Container genügen.

### 3. Verständliche Erklärung
Zelyra bringt seinen eigenen Hochleistungs-Webserver direkt mit:
```bash
zelyra serve src/main.zyl 0.0.0.0:8080
```
Für den professionellen Betrieb verpackst du dein Projekt in einen Docker-Container:
- Der Container enthält das Zelyra-Binary, deine Quelldateien und `zelyra.toml`.
- Beim Start führt der Container automatisch `zelyra db apply` aus und startet den Webserver.

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Produktionsreife Projektstruktur**
```toml
# zelyra.toml
[package]
name = "aufgaben_produktion"
version = "1.0.0"

[capabilities]
database = true
filesystem = false
network = true
```

**Beispiel 2: Saubere Hauptdatei für den Webbetrieb**
```zelyra
database main {
    engine: mariadb
    database: "tasks_prod"
}

table tasks {
    id: Id primary auto
    name: String(120) required
    fertig: Bool default false
}

page "/" {
    html {
        <h1>Zelyra Aufgabenverwaltung live</h1>
        <p>Produktivsystem aktiv und sicher.</p>
    }
}
```

**Beispiel 3: Startbefehl für den Server**
Auf dem Produktionsserver genügt:
```bash
zelyra db apply src/main.zyl
zelyra serve src/main.zyl 0.0.0.0:80
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Vergessen, die Datenbankmigrationen (`zelyra db apply`) vor dem Serverstart auszuführen.
  *Ursache:* Neue Tabellen und Spalten müssen in der Datenbank existieren, bevor Anfragen eingehen.
- **Fehler:** In Docker den Port `8080` nicht nach außen freigeben.
  *Ursache:* Nutze im `docker run`-Befehl das Port-Mapping `-p 8080:8080`.

### 6. Merksätze
1. `zelyra serve` startet den integrierten HTTP-Server ohne externe Webserver-Abhängigkeiten.
2. `zelyra db apply` bringt das Produktivschema sicher auf den neuesten Stand.
3. Klare Capability-Beschränkungen in `zelyra.toml` sichern den Server vor Angriffen ab.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Starte deine Webanwendung lokal auf Port 3000 mit `zelyra serve main.zyl 127.0.0.1:3000`.
- **Stufe 2 (Mittel):** Erstelle eine `docker-compose.yml`, die MariaDB und deine Zelyra-App verbindet.
- **Stufe 3 (Anspruchsvoll):** Simuliere ein Update mit einer Schema-Änderung und führe `zelyra db plan` und `zelyra db apply` aus.

### 8. Praxisaufgabe: Das produktionsfertige Aufgaben-Paket
Führe alle Einstellungen für das finale Deployment zusammen:
```zelyra
database main {
    engine: mariadb
    database: "tasks_production"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    erledigt: Bool default false
}

page "/" {
    html {
        <html>
            <body>
                <h1>Aufgabenverwaltung - Produktivsystem</h1>
                <p>System bereit fuer Benutzeranfragen.</p>
            </body>
        </html>
    }
}
```

### 9. Zusammenfassung
- Zelyra-Anwendungen lassen sich ohne komplizierte Server-Stacks direkt und performant betreiben.
- Datenbank-Migrationen und Webbetrieb greifen nahtlos ineinander.

### 10. Kontrollfragen zur Selbstprüfung
1. Welcher Befehl startet die Webanwendung auf einem Server?
2. Warum sollte `zelyra db apply` vor dem Starten des Webservers ausgeführt werden?
3. Welche Vorteile bietet der integrierte HTTP-Server gegenüber externen Server-Setups?

# TEIL X – ABSCHLUSSPROJEKT UND WEITERFÜHRUNG

---

## Kapitel 40: Das große Abschlussprojekt: Vollständige Aufgabenverwaltung

### 1. Was lerne ich in diesem Kapitel?
In diesem großen Finale lernst du:
- Wie alle gelernten Bausteine zu einer vollständigen, produktionsreifen Anwendung verschmelzen.
- Wie Datenbank, Authentifizierung, Rollen, CRUD-Interface, REST-API und Geschäftslogik ineinandergreifen.
- Wie du den kompletten Code liest, verstehst und auf deinem Server in Betrieb nimmst.

### 2. Warum ist das Thema wichtig?
Einzelne Code-Snippets zu verstehen ist eine Sache – eine echte, zusammenhängende Anwendung aus einem Guss zu bauen, ist die eigentliche Kunst der Softwareentwicklung. Dieses Abschlussprojekt beweist die Eleganz von Zelyra: In einer einzigen, übersichtlichen Datei entsteht eine datenbankgestützte, authentifizierte Webanwendung mit UI und REST-API, für die man in herkömmlichen Frameworks dutzende Dateien anlegen müsste.

### 3. Verständliche Erklärung des Gesamtprojekts
Unsere Aufgabenverwaltung umfasst:
1. **Datenbank & Schemas:** Ziel-Engine MariaDB mit Tabellen für Aufgaben (`tasks`), Benutzer (`users`), Sitzungen (`auth_sessions`) und Berechtigungen (`user_permissions`).
2. **Authentifizierung:** `auth users` mit geschützten Routen und Argon2-Passwortschutz.
3. **Geschäftslogik:** Reine Funktionen mit Verträgen (`requires`, `ensures`) zur Prioritätsprüfung und Fortschrittsberechnung.
4. **CRUD-Interface:** Das administrative Web-Dashboard mit Listen, Formularen und Löschdialogen.
5. **REST-API:** JSON-Endpunkte für den programmatischen Zugriff.

### 4. Das vollständige Projekt: Der finale Quelltext

```zelyra
database main {
    engine: mariadb
    database: "zelyra_tasks_app"
}

// ==========================================
// 1. AUTHENTIFIZIERUNG & BENUTZERVERWALTUNG
// ==========================================

auth users {
    table: users
    sessions: auth_sessions
    permissions: user_permissions
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
    user: User required
    permission: String(100) required
}

// ==========================================
// 2. AUFGABEN-DATENMODELL
// ==========================================

type TaskId = Id

table tasks {
    id: TaskId primary auto
    name: String(120) required
    beschreibung: String(500)
    prioritaet: Int default 2
    erledigt: Bool default false
}

// ==========================================
// 3. GESCHÄFTSLOGIK MIT VERTRÄGEN
// ==========================================

fn berechne_erfolgsquote(erledigte: Int, gesamt: Int) -> Int
    requires { gesamt > 0 && erledigte >= 0 && erledigte <= gesamt }
    ensures { result >= 0 && result <= 100 }
{
    return (erledigte * 100) / gesamt
}

fn prioritaet_label(stufe: Int) -> String {
    match stufe {
        1 => {
            return "HOCH"
        }
        2 => {
            return "MITTEL"
        }
        3 => {
            return "NIEDRIG"
        }
        _ => {
            return "NORMAL"
        }
    }
}

// ==========================================
// 4. WEBOBERFLÄCHE & CRUD-SCHNITTSTELLE
// ==========================================

crud Task -> tasks {
    title: "Zelyra Aufgabenverwaltung"

    view {
        fields {
            name
            prioritaet
            erledigt
        }

        list {
            mode: cards
            empty: "Keine Aufgaben vorhanden. Erstelle deine erste Aufgabe!"
        }

        detail {
            mode: cards
            title: "Aufgabendetails"
        }

        form {
            mode: cards
            title: "Aufgabe bearbeiten"
            submit: "Aufgabe sichern"
        }

        delete {
            title: "Aufgabe entfernen"
            message: "Moechtest du diese Aufgabe wirklich loeschen?"
            submit: "Jetzt loeschen"
        }
    }

    action abschliessen {
        label: "Als erledigt markieren"
        confirm: "Aufgabe abschliessen?"

        sql {
            UPDATE tasks
            SET erledigt = true
            WHERE id = :id
        }

        success "Aufgabe erfolgreich abgeschlossen."
        redirect "/tasks"
    }
}

// ==========================================
// 5. REST-API ENDPUNKTE
// ==========================================

api GET "/api/tasks/{id}" {
    input {
        id: TaskId
    }
    output Task
    errors {
        404 NotFound
    }
}

api POST "/api/tasks" {
    input {
        name: String
        prioritaet: Int
    }
    output Task
    errors {
        400 ValidationError
    }
}

// ==========================================
// 6. STARTSEITE
// ==========================================

page "/" {
    html {
        <html>
            <head>
                <title>Zelyra Aufgaben-System</title>
            </head>
            <body>
                <h1>Zelyra Aufgabenverwaltung</h1>
                <p>Das vollstaendige Abschlussprojekt ist einsatzbereit.</p>
                <a href="/tasks">Zur Aufgabenuebersicht</a>
            </body>
        </html>
    }
}

// ==========================================
// 7. EINSTIEGSPUNKT
// ==========================================

fn main() {
    print("Zelyra Aufgabenverwaltung vollstaendig initialisiert.")
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Den Code starten, ohne vorher `zelyra db apply` ausgeführt zu haben.
  *Ursache:* Die Tabellen in MariaDB müssen angelegt sein, bevor der Webserver Anfragen verarbeitet.
- **Fehler:** Fehlende MariaDB-Zugangsdaten in `.env`.
  *Ursache:* Hinterlege `DB_HOST`, `DB_USER` und `DB_PASSWORD` in deiner Umgebungsdatei.

### 6. Merksätze
1. In Zelyra entsteht eine vollständige, sichere Web-App in einer einzigen, harmonischen Datei.
2. Typsicherheit, Verträge, Authentifizierung und APIs greifen lückenlos ineinander.
3. Dieser Code ist sofort mit `zelyra check` prüfbar und mit `zelyra serve` startbar.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Kompiliere das Gesamtprojekt mit `zelyra check` und überprüfe, dass 0 Fehler auftreten.
- **Stufe 2 (Mittel):** Ergänze die Tabelle `tasks` um ein Feld `faellig_am: Date` und passe Formular und Ansichten an.
- **Stufe 3 (Anspruchsvoll):** Richte MariaDB lokal ein, spiele das Schema mit `zelyra db apply` ein und lege die ersten echten Aufgaben über den Webbrowser an.

### 8. Praxisaufgabe: Dein eigener produktiver Server-Start
Initialisiere das Projekt und starte es:
```bash
zelyra new task_manager --template minimal
cd task_manager
# Quellcode in src/main.zyl einfuegen
zelyra check src/main.zyl
zelyra serve src/main.zyl 0.0.0.0:8080
```
Öffne `http://localhost:8080` – deine eigene Zelyra-Anwendung ist live!

### 9. Zusammenfassung
- Das Abschlussprojekt vereint alle 9 vorangegangenen Teile dieses Lehrbuchs.
- Du hast gelernt, wie man eine moderne, fehlertolerante Webanwendung von Grund auf baut.

### 10. Kontrollfragen zur Selbstprüfung
1. Welche Komponenten wurden im Abschlussprojekt kombiniert?
2. Warum genügen in Zelyra so wenige Zeilen für ein vollwertiges CRUD-System?
3. Welche Schritte sind erforderlich, um das Projekt auf einem neuen Server zu deployen?

---

## Kapitel 41: Die Zelyra-Roadmap (Von 0.1 bis 1.0)

### 1. Was lerne ich in diesem Kapitel?
- Die Entwicklungsphasen von Zelyra: Wo die Sprache heute steht (Phase 1 bis 9) und was als Nächstes kommt.
- Geplante Features für Phase 10 bis 12: Externe Modul-Imports, Package-Management, WebAssembly-Support.
- Wie Abwärtskompatibilität und Stabilitätsgarantien bis Version 1.0 gewährleistet werden.

### 2. Warum ist das Thema wichtig?
Eine Programmiersprache ist ein lebendiges Ökosystem. Wer heute Zeit investiert, um Zelyra zu lernen, möchte sicher sein, dass die Sprache eine klare Zukunft hat, professionell weiterentwickelt wird und bestehender Code auch in kommenden Versionen lauffähig bleibt.

### 3. Verständliche Erklärung: Die Roadmap im Überblick
Die Entwicklung von Zelyra gliedert sich in 12 präzise geplante Phasen:
- **Phase 1 bis 3 (Fundament):** Lexer, Parser, AST, Typsystem, Kontrollstrukturen, Funktionen und Verträge (`requires`, `ensures`). *(Abgeschlossen)*
- **Phase 4 bis 6 (Datenbank & Daten):** MariaDB/SQLite-Engine, typisiertes `sql<T>`, Migrationen, Transaktionen, Dateisystem- und Zeit-Capabilities. *(Abgeschlossen)*
- **Phase 7 bis 9 (Web & Sicherheit):** `page`, `html`, `form` mit CSRF/XSS-Schutz, `crud`-Views, `auth` mit Argon2, `api` mit OpenAPI-Generierung, Typed Holes, JSON-Compilerdiagnostik. *(Abgeschlossen)*
- **Phase 10 (Aktuell in Version 0.1.50 umgesetzt):** Typisierte `Map<Key, Value>`-Wörterbücher, deklarative Page-Collections mit automatischer Suche, Filterung und Pagination, wiederverwendbare View-Layouts mit benannten Slots, `layout: ViewName` für CRUD-Ressourcen mit automatischer Detail-Verlinkung, `zelyra setup` mit web-basiertem Setup-Assistenten (`zelyra setup --web`), automatische Portvergabe, CSRF-geschützter Rechte-Entzug und plattformspezifische Docker-Unterstützung.
- **Phase 11 (Vorbereitung 0.9):** Feingranulares Modul- und Import-System (`import`), integriertes Test-Framework `zelyra test`.
- **Phase 12 (Auf dem Weg zu 1.0):** Offizieller Paketmanager, WebAssembly-Kompilierung (Zelyra im Browser), garantierte Langzeit-Stabilität (LTS).

### 4. Zelyras Versprechen an Entwickler
- **Keine Breaking Changes ohne Deprecation:** Änderungen an der Syntax werden mit klaren Übergangsfristen und Compiler-Hinweisen eingeführt.
- **Verlässliche Spezifikation:** Jedes Sprachmerkmal ist in der formalen Grammatik festgeschrieben.

### 5. Typische Missverständnisse
- **Missverständnis:** „Zelyra 0.1 ist nur ein Prototyp.“
  *Richtigstellung:* Zelyra 0.1 besitzt bereits einen voll funktionsfähigen Compiler, Typechecker, MariaDB-Treiber, Webserver und OpenAPI-Generator.
- **Missverständnis:** Sprachsyntax aus anderen Sprachen blind voraussetzen.
  *Richtigstellung:* Zelyra ist bewusst eigenständig entworfen. Nicht unterstützte Konstrukte (wie `enum` oder dynamisches `import`) sind in Phase 11/12 der Roadmap geplant.

### 6. Merksätze
1. Zelyra besitzt einen klaren, transparenten Entwicklungsplan von Version 0.1 bis 1.0.
2. Der Kern (Datenbank, Web, Typsicherheit, KI-Tools) ist bereits heute vollständig einsatzbereit.
3. Modul-Imports und Package-Management folgen in den Phasen 11 und 12.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Lies das offizielle `CHANGELOG.md` im Zelyra-Repository.
- **Stufe 2 (Mittel):** Vergleiche die Features von Phase 9 mit den Planungen für Phase 11.
- **Stufe 3 (Anspruchsvoll):** Schreibe ein kurzes Konzept für ein zukünftiges Zelyra-Paket, das du in Phase 12 veröffentlichen möchtest.

### 8. Praxisaufgabe: Zelyra-Version und Umgebung auditieren
Überprüfe die installierte Version und den Zustand deines Systems:
```bash
zelyra --version
zelyra doctor
```

### 9. Zusammenfassung
- Zelyra schreitet zielstrebig auf Version 1.0 zu.
- Das modulare Phasenkonzept garantiert kontinuierliche, stabile Weiterentwicklung.

### 10. Kontrollfragen zur Selbstprüfung
1. Welche Kernbausteine sind in Version 0.1 bereits vollständig implementiert?
2. Für welche Phase der Roadmap sind feingliedrige `import`-Anweisungen geplant?
3. Was bedeutet das Versprechen der Stabilitätsgarantie für bestehenden Code?

---

## Kapitel 42: Dein Weg als Zelyra-Entwickler

### 1. Was lerne ich in diesem Kapitel?
- Wie du dein erworbenes Wissen vertiefst und in eigenen Projekten anwendest.
- Die wichtigsten Prinzipien für professionelle, langlebige Softwarearchitektur.
- Wie du Teil der Zelyra-Community wirst und zum Open-Source-Ökosystem beiträgst.

### 2. Warum ist das Thema wichtig?
Programmieren lernt man nicht durch bloßes Lesen, sondern durch Machen. Dieses Buch hat dir das Fundament vermittelt – jetzt beginnt deine persönliche Reise als Entwickler. Mit Zelyra besitzt du eine moderne, sichere und zukunftsfähige Sprache, die dich bei jedem Schritt unterstützt.

### 3. Die fünf goldenen Regeln für Zelyra-Entwickler
1. **Model First:** Beginne jedes Projekt mit dem `table`-Schema. Ein klares Datenmodell löst die Hälfte aller späteren Probleme.
2. **Definiere Verträge:** Sichere wichtige Funktionen mit `requires` und `ensures` ab. Sie sind Dokumentation und Test in einem.
3. **Rechte bewusst vergeben:** Halte den Großteil deines Codes rein (ohne Capabilities) und deklariere Seiteneffekte gezielt.
4. **Fehler als Werte behandeln:** Nutze `Result` und `Option`. Verbanne Ausreden für unkontrollierte Abstürze.
5. **Gemeinsam mit KI arbeiten:** Nutze Zelyras `--format json` und Typed Holes `_`, um KI-Assistenten als produktive Co-Piloten einzusetzen.

### 4. Dein nächstes Projekt: Ideen zum Weiterprogrammieren
- **Persönliches Haushaltsbuch:** Einnahmen, Ausgaben, Kategorien und Monatsberichte mit Zelyra CRUD.
- **Support-Ticket-System:** Kundenanfragen, Prioritäten, Zuweisungen und E-Mail-Benachrichtigungen.
- **Kunden- und Projektzeiterfassung:** Stundenerfassung mit `uses Clock` und Rechnungs-Export als JSON.

### 5. Zusammenfassung des Lehrbuchs
Herzlichen Glückwunsch! Du hast alle 10 Teile und 42 Kapitel von *Zelyra lernen* erfolgreich gemeistert. Du beherrschst die Grundlagen, das Typsystem, Fehlerbehandlung, Datenbanken, Webanwendungen, APIs und sichere Softwarearchitektur. Du bist jetzt bereit, eigene, robuste Anwendungen mit Zelyra zu erschaffen!

### 6. Kontrollfragen zum Abschluss
1. Welche Zelyra-Besonderheit schätzt du nach diesem Lehrgang am meisten?
2. Warum ist das Zusammenspiel von Sprache und Datenbank in Zelyra so revolutionär?
3. Welches Projekt wirst du als Nächstes mit Zelyra umsetzen?

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
./install.sh --release v0.2.0
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
.\install.ps1 -Release v0.2.0
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
);
~~~

⚠️ Diese Ausgabe ist aktuell **nicht vollständig MariaDB-spezifisch**: Der
Generator hängt weder `ENGINE=InnoDB` noch
`DEFAULT CHARACTER SET=utf8mb4 COLLATE=utf8mb4_unicode_ci` an. Übernimm sie
deshalb nicht als fertige, organisationsweite MariaDB-DDL. Zeige das SQL
zuerst an, prüfe es, ergänze die gewünschten MariaDB-Tabellenoptionen manuell
und wende es erst dann kontrolliert an. Die Datenbankerzeugung in `db setup`
verwendet für `CREATE DATABASE` bereits `utf8mb4` und `utf8mb4_unicode_ci`.

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
| Produktionsreife | Rust ist breit produktiv eingesetzt | Zelyra Compiler 0.2.0 ist experimentell | Reife und Ökosystem sind nicht vergleichbar | 🧪 |
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
`Network`, `FileSystem`, `Environment`, `Process`, `Clock` und `Random` sind im
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
| Sprache und Compiler | Lexer, Parser, AST/HIR, Typprüfung, `Option`, `Result`, Pattern Matching, Ausdrucks-Typed-Holes (`_`), kanonisches `zelyra fmt` | Module, Imports, Generics, Lücken in Deklarationen und vollständige formale Verifikation |
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

# ANHÄNGE

---

## 24. Verbindliche Quellen und Compiler-Prüfung (Source Authority)

> **Grundsatz:** Zelyra ist eine eigenständige Sprache. Parser und geprüfte Tests entscheiden, was existiert.

Bei Widersprüchen gilt immer folgende verbindliche Reihenfolge:

1. **[Formale Sprachspezifikation](docs/specification.de.md) und Phasendokumente**
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

## Anhang A: Schnelleinstieg / Spickzettel (Syntax-Cheat-Sheet)

### Grundlegende Syntax
```zelyra
// Funktionen mit Vertraegen
fn summe(a: Int, b: Int) -> Int
    requires { a >= 0 && b >= 0 }
    ensures { result >= 0 }
{
    return a + b
}

// Einstiegspunkt und Variablen
fn main() {
    x = 10                  // Typableitung (unveraenderlich)
    mutable zaehler = 0     // Veraenderlich
    name: String = "Zelyra" // Expliziter Typ

    print(summe(3, 7))
}
```

### Typen
- Zahlen: `Int` (64-Bit vorzeichenbehaftet), `UInt` (vorzeichenlos), `Float`, `Decimal` (Festkomma)
- Text & Zeichen: `String`, `Char`
- Wahrheitswerte: `Bool` (`true`, `false`)
- Sammlungen: `Int[]`, `String[]`
- Abwesenheit: `Option<T>` (`Some(x)`, `None`), Kurzform `T?`
- Fehler: `Result<T, E>` (`Ok(x)`, `Err(e)`)
- System & Zeit: `Timestamp`, `Date`, `Time`, `Duration`

### Kontrollstrukturen
```zelyra
fn kontrolle(x: Int) {
    if x > 10 {
        print("Gross")
    } else {
        print("Klein")
    }

    match x {
        1 => { print("Eins") }
        2 => { print("Zwei") }
        _ => { print("Andere") }
    }

    mutable i = 0
    while i < 3 invariant { i >= 0 } {
        i = i + 1
    }

    for n in [1, 2, 3] {
        print(n)
    }
}

fn main() {
    kontrolle(1)
}
```

### Datenbank & Web
```zelyra
database main {
    engine: mariadb
    database: "app"
}

table items {
    id: Id primary auto
    bezeichnung: String required
}

page "/items" {
    html {
        <h1>Artikelliste</h1>
    }
}
```

---

## Anhang B: Alle Fehlermeldungen von Zelyra auf einen Blick

| Fehlercode | Kategorie | Beschreibung | Typische Behebung |
| :--- | :--- | :--- | :--- |
| `E-LEX-001` | Lexer | Unerwartetes Zeichen / Lexikalischer Fehler | Tippfehler oder unzulässiges Sonderzeichen entfernen |
| `E-PARSE-001` | Parser | Syntaxfehler (z. B. fehlende Klammer, falsches Token) | Syntax gemäß Zelyra-Grammatik korrigieren |
| `E-NAME-001` | Auflösung | Unbekannter Name / Variable nicht gefunden | Deklaration prüfen oder Tippfehler korrigieren |
| `E-TYPE-001` | Typprüfung | Typkonflikt (z. B. String zugewiesen an Int) | Typen anpassen oder Konvertierung vornehmen |
| `E-FEATURE-001` | Feature-Schalter | Zugriff auf eine deaktivierte Sprachoberfläche (`web`, `api`, `crud`, `auth`, `audit`) | Feature in `zelyra.toml` oder `.env` aktivieren |
| `E-CAP-001` / `E-CAP-002` | Capabilities | Fehlende Capability-Berechtigung (z. B. `database`, `network`) | In `zelyra.toml` unter `[capabilities]` freigeben |
| `E-POLICY-001` / `002` | Richtlinien | Verstoß gegen Sicherheits- oder Audit-Richtlinien | Sicherheitsdeklaration prüfen |
| `E-DB-001` - `E-DB-005` | Datenbank | Datenbankverbindungs- oder Treiberfehler | `DATABASE_URL` prüfen, MariaDB-Dienst starten |
| `E-SQL-001` - `E-SQL-004` | SQL | Ungültiges SQL / Schema-Misfit / Spalte nicht existent | SQL-Anweisung gegen Tabellendefinition prüfen |
| `E-VIEW-001` - `E-VIEW-009` | Views & Pages | Fehler in View-Interpolation, Slots oder Datenbindung | Slot-Namen und Datentypen der Page-Bindung prüfen |
| `E-VIEW-010` - `E-VIEW-015` | Query-Controls | Ungültige Such-, Sortier-, Paginierungs- oder Filterfelder | Deklarierte Whitelist (`search`, `sort`, `filter`) prüfen |
| `E-FORM-001` - `E-FORM-004` | Formulare | Validierungsfehler oder ungültige Feldtypen | Formular-Deklaration und Eingabedaten anpassen |
| `E-CRUD-001` - `E-CRUD-006` | CRUD | Ungültige CRUD-Ressource, Schema-Konflikt oder Layout-Fehler | Tabellenverknüpfung und Layout-Slots prüfen |
| `E-AUTH-001` - `E-AUTH-028` | Authentifizierung | Session-, Passwort- oder Berechtigungskonflikt | Rollen (`permits`), `requires auth` und Hashes prüfen |
| `E-AUDIT-001` - `E-AUDIT-010` | Audit-Trail | Fehler in der kryptografischen Hash-Kette des Audit-Logs | Prüfsummen und Audit-Tabelle validieren |
| `E-SETUP-001` - `E-SETUP-006` | Setup-Flow | Portkonflikt, Socket-Fehler oder Compose-Problem | Freie Ports wählen, Docker-Berechtigungen prüfen |
| `E-SETUP-WEB-001` | Web-Setup | Ungültiges oder abgelaufenes Setup-Token | Setup-Assistenten neu starten und Token-URL nutzen |
| `E-IMPACT-001` | Impact-Analyse | Zyklische oder ungültige Abhängigkeiten | Quellcode-Abhängigkeiten entflechten |
| `E-RUNTIME-001` | Laufzeit | Unbehandelter Laufzeitfehler | Verträge (`requires`, `ensures`) oder Fehlerwerte prüfen |

---

## Anhang C: Zelyra-CLI-Referenz

| Befehl | Option / Flag | Beschreibung |
| :--- | :--- | :--- |
| `zelyra --version` | | Gibt den vollständigen Compiler- und Paketversionsstand aus |
| `zelyra new <dir>` | `--template minimal\|mariadb-crud\|...` | Erstellt ein neues Zelyra-Projekt mit Vorlage |
| | `--mariadb` | Erzeugt MariaDB-Projekt mit Compose, Dockerfile und `.env` |
| | `--web-port <p> --host-port <p> --db-host-port <p>` | Konfiguriert Container- und Host-Ports |
| `zelyra init` | `[--mariadb]` | Initialisiert das aktuelle Verzeichnis als Zelyra-Projekt |
| `zelyra check <file.zyl>` | `[--format json]` | Prüft Syntax, Typen, Verträge und Capabilities statisch |
| `zelyra run <file.zyl>` | | Kompiliert und führt ein Zelyra-Programm aus |
| `zelyra serve <file.zyl>` | `[host:port]` | Startet den integrierten HTTP-Webserver |
| `zelyra setup` | `[--database]` | Startet Docker Compose / MariaDB |
| | `[--schema]` | Startet Umgebung und wendet Datenbankschema an |
| | `[--all]` | Führt Konfiguration, Start und Migration in einem Schritt aus |
| | `[--host-port <p>] [--db-host-port <p>]` | Setzt verbindliche Host-Ports für die neue `.env` |
| `zelyra setup --web` | `[--port <p>]` | Startet den lokalen, token-geschützten Browser-Setup-Assistenten |
| `zelyra config <file.zyl>` | `[--format json]` | Zeigt die wirksame Konfiguration und Feature-Schalter geheimnisfrei an |
| `zelyra doctor <file.zyl>` | `[--port <p>] [--json]` | Prüft Toolchain, MariaDB, Docker und Ports ohne DB-Änderung |
| | `[--env-file <file>]` | Liest gezielt `DATABASE_URL` aus der angegebenen Datei |
| `zelyra fmt <file.zyl>` | `[--check]` | Formatiert Quellcode nach dem offiziellen Standard |
| `zelyra verify <file.zyl>` | | Führt formale Vertragsverifikation durch |
| `zelyra doc <file.zyl>` | `--openapi` | Generiert OpenAPI-3.0-Spezifikationen |
| | `--typescript` | Generiert typisierten, abhängigkeitsfreien TypeScript-Client |
| `zelyra db init <file.zyl>` | | Initialisiert Datenbank und Basistabellen |
| `zelyra db setup <file.zyl>` | | Richtet die MariaDB-Datenbank initial ein |
| `zelyra db apply <file.zyl>` | | Wendet Schema-Migrationen sicher an |
| `zelyra auth hash-password` | `[--stdin]` | Erzeugt sichere Argon2-Passworthashes |
| `zelyra form validate <file> <Form>` | | Prüft Formulare mit Testwerten auf der Konsole |
| `zelyra context <file.zyl>` | `[--format json]` | Gibt den semantischen Quellcode-Kontext für Tools aus |

---

## Anhang D: Die Standardbibliothek im Überblick

### Grundfunktionen (ohne Capabilities)
- `print(wert)`: Gibt einen beliebigen Wert auf der Standardausgabe aus.
- `len(array)`: Liefert die Anzahl der Elemente in einem Array als `Int`.
- `append(array, element)`: Erzeugt ein neues Array mit angehängtem Wert.
- `contains(array, element)` -> `Bool`: Prüft, ob ein Wert im Array enthalten ist.
- `first(array)` -> `Option<T>`: Liefert das erste Element oder `None`.
- `last(array)` -> `Option<T>`: Liefert das letzte Element oder `None`.
- `get(map, key)` -> `Option<V>`: Schlägt einen Schlüssel in einer `Map<K, V>` nach.
- `put(map, key, value)` -> `Map<K, V>`: Fügt ein Schlüssel-Wert-Paar hinzu oder aktualisiert es funktional.
- `keys(map)` -> `K[]`: Liefert alle Schlüssel einer Map als Array.
- `values(map)` -> `V[]`: Liefert alle Werte einer Map als Array.
- `Some(wert)` / `None`: Konstruktoren für den Typ `Option<T>`.
- `Ok(wert)` / `Err(fehler)`: Konstruktoren für den Typ `Result<T, E>`.
- `json_encode(wert)` -> `String`: Wandelt Daten in JSON um.
- `json_decode<T>(text)` -> `Result<T, String>`: Parst typisiertes JSON.

### Funktionen mit Capabilities
- `uses Clock`:
  - `now()` -> `Timestamp`: Aktueller Systemzeitstempel.
- `uses Random`:
  - `random_int(min: Int, max: Int)` -> `Int`: Zufallszahl im Intervall.
- `uses Environment`:
  - `env(name: String)` -> `Option<String>`: Liest eine Umgebungsvariable.
- `uses FileSystem`:
  - `read_text(pfad: String)` -> `String`: Liest Dateiinhalt als Text.
  - `write_text(pfad: String, inhalt: String)`: Schreibt Inhalt in Datei.
  - `delete_file(pfad: String)`: Löscht eine Datei.
  - `list_dir(ordner: String)` -> `String[]`: Listet Dateinamen auf.
- `uses Database`:
  - `sql<T[]> { SELECT ... }`: Führt typisierte SQL-Abfragen aus.
  - `transaction { ... }`: Fasst Abfragen transaktional zusammen.

---

## Anhang E: SQL-Spickzettel für Zelyra-Entwickler

In Zelyra eingebettetes SQL wird mit `sql<T[]>` oder `sql` ausgeführt:

```zelyra
database main {
    engine: mariadb
    database: "app"
}

table tasks {
    id: Id primary auto
    name: String required
    erledigt: Bool default false
}

fn sql_beispiele() uses Database {
    // 1. SELECT mit typisiertem Rueckgabetyp und sicherem Parameter
    status = false
    gefiltert = sql<Task[]> {
        SELECT id, name, erledigt
        FROM tasks
        WHERE erledigt = :status
    }

    // 2. INSERT in einer Transaktion
    text = "Neue Aufgabe"
    transaction {
        sql {
            INSERT INTO tasks (name, erledigt)
            VALUES (:text, false)
        }
    }

    // 3. UPDATE
    ziel_id = 1
    transaction {
        sql {
            UPDATE tasks
            SET erledigt = true
            WHERE id = :ziel_id
        }
    }
}

fn main() uses Database {
    print("SQL Spickzettel validiert.")
}
```

---

## Anhang F: HTML- und Web-Referenz in Zelyra

### Web-Strukturen und Deklarationen

| Element | Deklaration | Zweck |
| :--- | :--- | :--- |
| **Page** | `page "/pfad/{param}" { ... }` | Definiert eine HTTP-GET-Route mit Pfadparametern und HTML-Antwort |
| **View-Layout** | `view LayoutName { html { ... <slot /> ... } }` | Wiederverwendbares Layout mit Standard- und benannten Slots |
| **Component** | `component Name { props { ... } html { ... } }` | Wiederverwendbare HTML-Komponente mit typisierten Eigenschaften |
| **Named Slot** | `<slot name="header">Fallback</slot>` | Platzhalter im Layout/Komponente mit optionalem Standardinhalt |
| **Slot Injection** | `<slot name="header">Inhalt</slot>` | Übergabe von Kindinhalten an den passenden Slot |
| **Data Loading** | `load item = sql<Item> { SELECT ... }` | Typisiertes Laden eines Einzeldatensatzes mit Feldzugriff `{item.field}` |
| **Collection Loop**| `for item in items { <li>{item.name}</li> }` | Typisierte serverseitige Iteration über geladene Datensätze |
| **Search Control** | `search { col1 col2 }` | Whitelist-geprüfte URL-Suche mit parametrisierter `LIKE`-Abfrage |
| **Sort Control** | `sort { col1 col2 }` | Typisierte Sortierung über `?sort=col&order=asc\|desc` |
| **Pagination** | `paginated 25` | Paginierung mit `LIMIT`/`OFFSET`, `page`, `pages` und `total` |
| **Filter Control** | `filter { col1 col2 }` | Typisierte Filteroperatoren (`eq`, `contains`, `starts_with`, `gt`, `lte` etc.) |
| **CRUD Layout** | `crud Res { table tbl layout: LayoutName }` | Bindet generierte CRUD-Ansichten in Slots `title`, `nav`, `content`, `actions` ein |

---

## Anhang G: Glossar der Fachbegriffe

- **AST (Abstract Syntax Tree):** Die hierarchische Baumstruktur, in die der Compiler deinen Quellcode übersetzt.
- **Capability (Fähigkeit):** Ausdrückliche Berechtigung (`uses FileSystem`, etc.), ohne die eine Funktion keine geschützten Ressourcen berühren darf.
- **Design by Contract:** Entwurfsmethode, bei der Funktionen über Vorbedingungen (`requires`) und Nachbedingungen (`ensures`) vertraglich abgesichert werden.
- **Immutable (Unveränderlich):** Variablen können nach der ersten Zuweisung nicht mehr verändert werden. In Zelyra Standard, es sei denn, sie werden mit `mutable` deklariert.
- **Invariant:** Eine Bedingung (z. B. in einer Schleife), die vor und nach jedem Durchlauf garantiert wahr sein muss.
- **Option:** Typ (`Some(v)` oder `None`), der das mögliche Fehlen eines Wertes darstellt – Zelyras Antwort auf gefürchtete `null`-Pointer-Crashes.
- **Result:** Typ (`Ok(v)` oder `Err(e)`), der das Scheitern einer Operation als sicheren Wert zurückgibt, statt unkontrollierte Abstürze auszulösen.
- **Typed Hole (`_`):** Platzhalter im Code, der dem Compiler und KI-Werkzeugen signalisiert, an dieser Stelle eine passende Implementierung zu erwarten.

---

## Anhang H: Lösungen zu den Übungsaufgaben der Kapitel

### Kapitel 1: Begrüßung
```zelyra
fn main() {
    print("Hallo Welt aus Zelyra!")
}
```

### Kapitel 5: Rabattpreis berechnen
```zelyra
fn berechne_rabatt(original: Float, prozent: Float) -> Float {
    return original * (1.0 - (prozent / 100.0))
}

fn main() {
    print(berechne_rabatt(100.0, 20.0))
}
```

### Kapitel 11: Zahlen verdoppeln
```zelyra
fn verdopple(zahl: Int) -> Int {
    return zahl * 2
}

fn main() {
    print(verdopple(21))
}
```

### Kapitel 12: Vor- und Nachbedingungen
```zelyra
fn begrenze(wert: Int, min_w: Int, max_w: Int) -> Int
    requires { min_w <= max_w }
    ensures { result >= min_w && result <= max_w }
{
    if wert < min_w { return min_w }
    if wert > max_w { return max_w }
    return wert
}

fn main() {
    print(begrenze(120, 0, 100))
}
```

### Kapitel 13: Array summieren
```zelyra
fn summe_array(zahlen: Int[]) -> Int {
    mutable gesamt = 0
    for z in zahlen {
        gesamt = gesamt + z
    }
    return gesamt
}

fn main() {
    liste: Int[] = [1, 2, 3, 4, 5]
    print(summe_array(liste))
}
```

---

## Anhang I: Häufige Fragen und Antworten (FAQ)

**Frage: Warum gibt es in Zelyra 0.1 keine `import`-Anweisung?**
*Antwort:* In Version 0.1 übersetzt der Zelyra-Compiler alle `.zyl`-Quelldateien im Projektkontext als ein einheitliches System. Ein feingranulares Modul- und Import-System befindet sich laut Entwicklungsplan in Phase 11.

**Frage: Kann ich mit Zelyra auch reine Konsolenprogramme schreiben?**
*Antwort:* Ja! Mit `print()` kannst du Ausgaben erzeugen und über Parameter oder `env()` Eingaben entgegennehmen. Interaktive Terminal-Eingaben (`read_line`) folgen in späteren Phasen.

**Frage: Warum unterstützt Zelyra MariaDB als bevorzugte Engine?**
*Antwort:* MariaDB bietet herausragende Performance, Open-Source-Freiheit, Stabilität und breite Cloud-Unterstützung für professionelle Webanwendungen.

---

## Anhang J: Weiterführende Ressourcen und Community

- **Offizielles GitHub-Repository:** [https://github.com/sf1976/zelyra](https://github.com/sf1976/zelyra)
- **Dokumentation & Online-Handbuch:** [https://siedelmann.com/handbuch](https://siedelmann.com/handbuch) / [https://siedelmann.com/handbook](https://siedelmann.com/handbook)
- **Beispiele & Vorlagen:** Im Verzeichnis `examples/` des Repositories findest du lauffähige Vorlagen für Authentifizierung, CRUD-Ansichten, APIs und Datenbanken.
