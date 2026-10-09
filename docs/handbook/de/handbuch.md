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

- **[TEIL I – ZELYRA UND PROGRAMMIERUNG VERSTEHEN](lehrbuch-grundlagen.md#teil-i-zelyra-und-programmierung-verstehen)**
  - [Kapitel 1: Willkommen bei Zelyra](lehrbuch-grundlagen.md#kapitel-1-willkommen-bei-zelyra)
  - [Kapitel 2: Wie ein Programm funktioniert](lehrbuch-grundlagen.md#kapitel-2-wie-ein-programm-funktioniert)
  - [Kapitel 3: Zelyra installieren und einrichten](lehrbuch-grundlagen.md#kapitel-3-zelyra-installieren-und-einrichten)
  - [Kapitel 4: Das erste Zelyra-Projekt](lehrbuch-grundlagen.md#kapitel-4-das-erste-zelyra-projekt)
- **[TEIL II – DIE GRUNDLAGEN DER SPRACHE](lehrbuch-grundlagen.md#teil-ii-die-grundlagen-der-sprache)**
  - [Kapitel 5: Werte und Datentypen](lehrbuch-grundlagen.md#kapitel-5-werte-und-datentypen)
  - [Kapitel 6: Variablen und Unveränderlichkeit](lehrbuch-grundlagen.md#kapitel-6-variablen-und-unveranderlichkeit)
  - [Kapitel 7: Operatoren und Ausdrücke](lehrbuch-grundlagen.md#kapitel-7-operatoren-und-ausdrucke)
  - [Kapitel 8: Ein- und Ausgaben](lehrbuch-grundlagen.md#kapitel-8-ein-und-ausgaben)
  - [Kapitel 9: Entscheidungen mit Bedingungen](lehrbuch-grundlagen.md#kapitel-9-entscheidungen-mit-bedingungen)
  - [Kapitel 10: Wiederholungen und Schleifen](lehrbuch-grundlagen.md#kapitel-10-wiederholungen-und-schleifen)
- **[TEIL III – PROGRAMME STRUKTURIEREN](lehrbuch-grundlagen.md#teil-iii-programme-strukturieren)**
  - [Kapitel 11: Funktionen und Prozeduren](lehrbuch-grundlagen.md#kapitel-11-funktionen-und-prozeduren)
  - [Kapitel 12: Verträge und Vorbedingungen (Design by Contract)](lehrbuch-grundlagen.md#kapitel-12-vertrage-und-vorbedingungen-design-by-contract)
  - [Kapitel 13: Sammlungen, Listen und Wörterbücher (Arrays & Maps)](lehrbuch-grundlagen.md#kapitel-13-sammlungen-listen-und-worterbucher-arrays-und-maps)
  - [Kapitel 14: Eigene Datentypen erstellen (Records & Tables)](lehrbuch-grundlagen.md#kapitel-14-eigene-datentypen-erstellen-records-tables)
  - [Kapitel 15: Module und Code-Organisation](lehrbuch-grundlagen.md#kapitel-15-module-und-code-organisation)
- **[TEIL IV – SICHERHEIT UND FEHLERBEHANDLUNG](lehrbuch-grundlagen.md#teil-iv-sicherheit-und-fehlerbehandlung)**
  - [Kapitel 16: Fehlerarten und ihre Ursachen](lehrbuch-grundlagen.md#kapitel-16-fehlerarten-und-ihre-ursachen)
  - [Kapitel 17: Fehler als Werte – Das Result-Muster](lehrbuch-grundlagen.md#kapitel-17-fehler-als-werte-das-result-muster)
  - [Kapitel 18: Das Nichts existiert nicht – Der sichere Umgang mit Option](lehrbuch-grundlagen.md#kapitel-18-das-nichts-existiert-nicht-der-sichere-umgang-mit-option)
  - [Kapitel 19: Tests und Qualitätssicherung](lehrbuch-grundlagen.md#kapitel-19-tests-und-qualitatssicherung)
- **[TEIL V – PRAKTISCHE DATENVERARBEITUNG](lehrbuch-grundlagen.md#teil-v-praktische-datenverarbeitung)**
  - [Kapitel 20: Arbeiten mit Dateien](lehrbuch-grundlagen.md#kapitel-20-arbeiten-mit-dateien)
  - [Kapitel 21: Datum, Uhrzeit, Zufall und strukturierte Daten](lehrbuch-grundlagen.md#kapitel-21-datum-uhrzeit-zufall-und-strukturierte-daten)
  - [Kapitel 22: Nebenläufigkeit und Hintergrundaufgaben](lehrbuch-grundlagen.md#kapitel-22-nebenlaufigkeit-und-hintergrundaufgaben)
- **[TEIL VI – DATENBANKEN MIT ZELYRA](lehrbuch-anwendungen.md#teil-vi-datenbanken-mit-zelyra)**
  - [Kapitel 23: Warum Zelyra die Datenbank direkt versteht](lehrbuch-anwendungen.md#kapitel-23-warum-zelyra-die-datenbank-direkt-versteht)
  - [Kapitel 24: Tabellen definieren und Daten modellieren](lehrbuch-anwendungen.md#kapitel-24-tabellen-definieren-und-daten-modellieren)
  - [Kapitel 25: Daten abfragen und verändern](lehrbuch-anwendungen.md#kapitel-25-daten-abfragen-und-verandern)
- **[TEIL VII – WEBANWENDUNGEN UND FORMULARE](lehrbuch-anwendungen.md#teil-vii-webanwendungen-und-formulare)**
  - [Kapitel 26: Webseiten ausgeben](lehrbuch-anwendungen.md#kapitel-26-webseiten-ausgeben)
  - [Kapitel 27: Formulare und Benutzereingaben](lehrbuch-anwendungen.md#kapitel-27-formulare-und-benutzereingaben)
  - [Kapitel 28: Das vollständige CRUD-Muster](lehrbuch-anwendungen.md#kapitel-28-das-vollstandige-crud-muster)
  - [Kapitel 29: Benutzer, Passwörter und Sitzungen](lehrbuch-anwendungen.md#kapitel-29-benutzer-passworter-und-sitzungen)
  - [Kapitel 30: APIs und Datenaustausch](lehrbuch-anwendungen.md#kapitel-30-apis-und-datenaustausch)
- **[TEIL VIII – DIE BESONDERHEITEN VON ZELYRA](lehrbuch-anwendungen.md#teil-viii-die-besonderheiten-von-zelyra)**
  - [Kapitel 31: Lesbarkeit als oberstes Gebot](lehrbuch-anwendungen.md#kapitel-31-lesbarkeit-als-oberstes-gebot)
  - [Kapitel 32: KI-Nativität – Warum Zelyra perfekt für KI-Assistenten ist](lehrbuch-anwendungen.md#kapitel-32-ki-nativitat-warum-zelyra-perfekt-fur-ki-assistenten-ist)
  - [Kapitel 33: Sicherheit durch Fähigkeiten (Capabilities)](lehrbuch-anwendungen.md#kapitel-33-sicherheit-durch-fahigkeiten-capabilities)
  - [Kapitel 34: Zelyra im Vergleich](lehrbuch-anwendungen.md#kapitel-34-zelyra-im-vergleich)
- **[TEIL IX – VOM ENTWURF ZUR FERTIGEN ANWENDUNG](lehrbuch-anwendungen.md#teil-ix-vom-entwurf-zur-fertigen-anwendung)**
  - [Kapitel 35: Software planen – Von der Idee zum Entwurf](lehrbuch-anwendungen.md#kapitel-35-software-planen-von-der-idee-zum-entwurf)
  - [Kapitel 36: Architektur und saubere Codestruktur](lehrbuch-anwendungen.md#kapitel-36-architektur-und-saubere-codestruktur)
  - [Kapitel 37: Konfiguration und Umgebungsvariablen](lehrbuch-anwendungen.md#kapitel-37-konfiguration-und-umgebungsvariablen)
  - [Kapitel 38: Fehlersuche und Optimierung](lehrbuch-anwendungen.md#kapitel-38-fehlersuche-und-optimierung)
  - [Kapitel 39: Bereitstellung und Betrieb](lehrbuch-anwendungen.md#kapitel-39-bereitstellung-und-betrieb)
- **[TEIL X – ABSCHLUSSPROJEKT UND WEITERFÜHRUNG](lehrbuch-anwendungen.md#teil-x-abschlussprojekt-und-weiterfuhrung)**
  - [Kapitel 40: Das große Abschlussprojekt: Vollständige Aufgabenverwaltung](lehrbuch-anwendungen.md#kapitel-40-das-grosse-abschlussprojekt-vollstandige-aufgabenverwaltung)
  - [Kapitel 41: Die Zelyra-Roadmap (Von 0.3.0 bis 1.0)](lehrbuch-anwendungen.md#kapitel-41-die-zelyra-roadmap-von-030-bis-10)
  - [Kapitel 42: Dein Weg als Zelyra-Entwickler](lehrbuch-anwendungen.md#kapitel-42-dein-weg-als-zelyra-entwickler)

### Technisches Referenzhandbuch

- [1. Was Zelyra anders macht](technisches-referenzhandbuch.md#1-was-zelyra-anders-macht)
- [2. Installation](technisches-referenzhandbuch.md#2-installation)
- [3. Das erste Programm](technisches-referenzhandbuch.md#3-das-erste-programm)
- [4. Neues Projekt anlegen und CLI](technisches-referenzhandbuch.md#4-neues-projekt-anlegen-und-cli)
- [5. Variablen, Typen und Funktionen](technisches-referenzhandbuch.md#5-variablen-typen-und-funktionen)
- [6. Option, Result und Pattern Matching](technisches-referenzhandbuch.md#6-option-result-und-pattern-matching)
- [7. MariaDB und Tabellen](technisches-referenzhandbuch.md#7-mariadb-und-tabellen)
- [8. Schema prüfen und anwenden](technisches-referenzhandbuch.md#8-schema-prufen-und-anwenden)
- [9. Natives SQL](technisches-referenzhandbuch.md#9-natives-sql)
- [10. Webseiten](technisches-referenzhandbuch.md#10-webseiten)
- [11. Formulare](technisches-referenzhandbuch.md#11-formulare)
- [12. CRUD](technisches-referenzhandbuch.md#12-crud)
- [13. Authentifizierung und Berechtigungen](technisches-referenzhandbuch.md#13-authentifizierung-und-berechtigungen)
- [14. Capabilities](technisches-referenzhandbuch.md#14-capabilities)
- [15. Contracts und Verify](technisches-referenzhandbuch.md#15-contracts-und-verify)
- [16. Konfiguration und Geheimnisse](technisches-referenzhandbuch.md#16-konfiguration-und-geheimnisse)
- [17. Diagnosen und Fehlersuche](technisches-referenzhandbuch.md#17-diagnosen-und-fehlersuche)
- [18. Testen und Mitentwickeln](technisches-referenzhandbuch.md#18-testen-und-mitentwickeln)
- [19. Was als Nächstes kommt](technisches-referenzhandbuch.md#19-was-als-nachstes-kommt)
- [20. Zelyra im Vergleich zu Rust](technisches-referenzhandbuch.md#20-zelyra-im-vergleich-zu-rust)
- [21. Positionierung und aktueller Entwicklungsstand](technisches-referenzhandbuch.md#21-positionierung-und-aktueller-entwicklungsstand)
- [22. Roadmap aus dem aktuellen Repository](technisches-referenzhandbuch.md#22-roadmap-aus-dem-aktuellen-repository)
- [23. KI-native Entwicklung](technisches-referenzhandbuch.md#23-ki-native-entwicklung)
- [24. Verbindliche Quellen und Compiler-Prüfung (Source Authority)](anhaenge.md#24-verbindliche-quellen-und-compiler-prufung-source-authority)

### Anhänge

- [Anhang A: Schnelleinstieg / Spickzettel (Syntax-Cheat-Sheet)](anhaenge.md#anhang-a-schnelleinstieg-spickzettel-syntax-cheat-sheet)
- [Anhang B: Alle Fehlermeldungen von Zelyra auf einen Blick](anhaenge.md#anhang-b-alle-fehlermeldungen-von-zelyra-auf-einen-blick)
- [Anhang C: Zelyra-CLI-Referenz](anhaenge.md#anhang-c-zelyra-cli-referenz)
- [Anhang D: Die Standardbibliothek im Überblick](anhaenge.md#anhang-d-die-standardbibliothek-im-uberblick)
- [Anhang E: SQL-Spickzettel für Zelyra-Entwickler](anhaenge.md#anhang-e-sql-spickzettel-fur-zelyra-entwickler)
- [Anhang F: HTML- und Web-Referenz in Zelyra](anhaenge.md#anhang-f-html-und-web-referenz-in-zelyra)
- [Anhang G: Glossar der Fachbegriffe](anhaenge.md#anhang-g-glossar-der-fachbegriffe)
- [Anhang H: Lösungen zu den Übungsaufgaben der Kapitel](anhaenge.md#anhang-h-losungen-zu-den-ubungsaufgaben-der-kapitel)
- [Anhang I: Häufige Fragen und Antworten (FAQ)](anhaenge.md#anhang-i-haufige-fragen-und-antworten-faq)
- [Anhang J: Weiterführende Ressourcen und Community](anhaenge.md#anhang-j-weiterfuhrende-ressourcen-und-community)
