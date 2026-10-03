Warning: truncated output (original token count: 86642)
Total output lines: 8843

# Das Zelyra-Handbuch

**Von den Grundlagen bis zur datenbankgestützten Webanwendung. Absicht beschreiben. Korrektheit beweisen.**

[English edition](/handbook) · Deutsch

Willkommen beim vollständigen Zelyra-Handbuch. Es umfasst sowohl das didaktische Lehrbuch **»Zelyra lernen – Verständlich programmieren von den Grundlagen bis zur eigenen Anwendung«** (Teil I bis X, Kapitel 1 bis 42) als auch das **technische Referenzhandbuch** (Kapitel 1 bis 23) sowie ausführliche **Anhänge** (A bis J).

> **Projektstatus:** Compiler 0.3.0 implementiert einen geprüften, experimentellen Teil der Sprachlinie 0.1. Zelyra ist noch nicht für den Produktionseinsatz freigegeben.

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
  - [Kapitel 41: Die Zelyra-Roadmap (Von 0.3.0 bis 1.0)](#kapitel-41-die-zelyra-roadmap-von-030-bis-10)
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
- **Stufe 2 (Mittel):** Ändere in `zelyra.toml` die Version auf `0.3.0` und gib in `main()` die neue Versionsnummer aus.
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
In unserer Aufgabenverwaltung wollen wir zähle…66642 tokens truncated…üssel ab | ✅ |
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
| `zelyra module plan <entry> <module-or-resource>` | | Experimentelle, schreibgeschützte Vorschau bekannter Modulabhängigkeiten |
| `zelyra module bundle <entry> <module-or-resource>` | `--output <dir>` | Erzeugt ein geprüftes experimentelles Quellpaket; kein Vollständigkeitsnachweis |
| | `--docker --compiler-ref <40-stelliger-Commit>` | Erzeugt Dockerfile, Compose-App und `.env.example`; Abhängigkeitsschluss bleibt unvollständig |
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
| `zelyra module plan <entry> <module-or-resource-id>` | | Zeigt bekannte Abhängigkeiten ab Quelldatei oder Anwendungsressource; kein Deploymentexport |

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
- `json_decode<T>(text)` -> `T`: Parst typisiertes JSON; ungültige Daten werden als Laufzeitfehler gemeldet.

### Funktionen mit Capabilities
- `uses Console`:
  - `read_console(prompt: String)` -> `String?`: Zeigt den Prompt an und liest eine Zeile; `None` bedeutet EOF.
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

**Frage: Warum gibt es in Zelyra 0.3.0 keine `import`-Anweisung?**
*Antwort:* Das veröffentlichte 0.3.0-Binary enthält keine Modulimporte. Ein
CLI-Aufruf prüft die ausdrücklich angegebene `.zyl`-Datei. Im aktuellen,
unveröffentlichten Entwicklungszweig gibt es experimentelle Imports für
Funktionen, Typen, Records, Tabellen, Views, Komponenten und eine projektweite
Datenbankkonfiguration. Unter anderem `serve` kann den verknüpften
Projektgraphen verwenden; ein vollständiges, stabiles Modul- und Paketmodell
bleibt geplante Arbeit.

**Frage: Kann ich mit Zelyra auch reine Konsolenprogramme schreiben?**
*Antwort:* Ja. `print()` gibt Werte aus. `read_console("Prompt: ")` liest eine Zeile und liefert `String?`; dafür braucht die Funktion `uses Console` und das Projekt gegebenenfalls `console = true`.

**Frage: Warum unterstützt Zelyra MariaDB als bevorzugte Engine?**
*Antwort:* MariaDB bietet herausragende Performance, Open-Source-Freiheit, Stabilität und breite Cloud-Unterstützung für professionelle Webanwendungen.

---

## Anhang J: Weiterführende Ressourcen und Community

- **Offizielles GitHub-Repository:** [https://github.com/sf1976/zelyra](https://github.com/sf1976/zelyra)
- **Dokumentation & Online-Handbuch:** [https://siedelmann.com/handbuch](https://siedelmann.com/handbuch) / [https://siedelmann.com/handbook](https://siedelmann.com/handbook)
- **Beispiele & Vorlagen:** Im Verzeichnis `examples/` des Repositories findest du lauffähige Vorlagen für Authentifizierung, CRUD-Ansichten, APIs und Datenbanken.
