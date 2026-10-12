# Zelyra

![Zelyra-Logo](assets/zelyra-logo.png)

**Von der Datenbank zur Anwendung.** · **Die KI schreibt. Zelyra prüft.**

Deutsch · [English](README.md)

Zelyra ist eine eigenständige, statisch typisierte Sprache und Plattform für
Business-, Datenbank- und Webanwendungen. Schemaabhängiges SQL, Formulare,
CRUD, Views und Geschäftslogik greifen ineinander; gewöhnlicher Programmcode
und natives SQL bleiben jederzeit möglich.

> **Aktueller stabiler Compilerstand: 0.9.8 · Sprachkompatibilitätslinie: 0.1 · experimentell**
>
> Zelyra 0.9.8 bleibt experimentell und ist nicht für den Produktiveinsatz freigegeben. „Korrektheit
> beweisen“ ist ein Entwicklungsziel: Der aktuelle Verifier deckt nur einen
> begrenzten Teil ab und beweist nicht die Korrektheit beliebiger Anwendungen.

Das stabile [`v0.9.8`-Release](https://github.com/sf1976/zelyra/releases/tag/v0.9.8) bleibt experimentell und ist nicht für den Produktiveinsatz freigegeben. Der aktuelle Branch `release/0.9.9-pre-release` ergänzt generische Records und Enums, benannte Funktionswerte, explizite skalare Textformatierung sowie optische Verbesserungen im Studio und in der CLI; die technische Prüfung läuft noch. Die unabhängige menschliche Abnahme ist dem konkreten 1.0.0-Kandidaten vorbehalten. Siehe die [Release-Notizen 0.9.8](docs/release-notes/0.9.8.de.md), den [Entwicklungsplan 0.9.9](docs/release-plans/0.9.9.de.md), die [Datenbank-Supportmatrix](docs/database-support.de.md) und den [1.0.0-Releaseplan](docs/release-plans/1.0.0.de.md).

## Was Zelyra 0.9.8 kann

Zelyra verbindet Sprache, Compiler, Datenbank, Webanwendung und Studio in
Einem Arbeitsablauf. Die aktuelle Version bietet:

- statisch geprüfte Geschäftslogik und schemaabhängiges SQL;
- MariaDB als primäre Laufzeit sowie geprüfte MySQL- und PostgreSQL-Abläufe;
- generierte Formulare, Tabellenansichten, CRUD, Beziehungen und APIs;
- Studio mit Editor, Compilerhinweisen, Datenbankwerkbank und Projektkarte;
- ein mehrteiliges Rechnungstutorial mit lokalem Tabler-CSS, Desktop-Dashboard
  und druckbarer Netto-/Steuer-/Bruttoansicht;
- CLI-Werkzeuge für Routen, Setup, Diagnose, Datenbank, Audit, SMTP und statische
  Laravel-Migrationsberichte.

Zelyra ist weiterhin experimentell und nicht produktiv freigegeben. Die
Supportmatrix beschreibt die geprüften Datenbankabläufe und Grenzen. Die
[Implementierungsübersicht](docs/implemented.de.md) beschreibt den Entwicklungszweig 0.9.9;
Funktionsumfang; die [Roadmap](docs/ROADMAP.de.md) und das
[Changelog](CHANGELOG.md) enthalten offene beziehungsweise historische Arbeit.
Die unabhängige menschliche Abnahme bleibt für 1.0.0 vorgesehen.

## Schnellstart

### Die Sprache ausprobieren

Aus einem Quellcode-Checkout:

~~~bash
git clone https://github.com/sf1976/zelyra.git
cd zelyra
./install.sh
zelyra run examples/fibonacci.zyl
~~~

Erwartete Ausgabe: `55`. Veröffentlichte Linux- und Windows-x86_64-Binärdateien
können auch ohne Rust-Toolchain installiert werden. Details stehen bei den
[Releases](https://github.com/sf1976/zelyra/releases) und in der
Installationsanleitung.

Ein interaktives CLI-Beispiel startest du mit
`zelyra run examples/console_input.zyl`. Es verwendet die ausdrückliche
Capability `Console`; die Projektfreigabe ist in der Repository-
`zelyra.toml` aktiviert. Für neue Projekte erklärt das
[I/O-Kapitel im Handbuch](docs/handbook/de/lehrbuch-grundlagen.md#kapitel-8-ein--und-ausgaben)
die nötige Einrichtung.

### Eine MariaDB-CRUD-Anwendung starten

~~~bash
zelyra new maschinenverwaltung --template mariadb-crud
cd maschinenverwaltung
zelyra setup --all
~~~

Der Setup-Befehl zeigt die lokale Anwendungsadresse an. Docker Engine und
Compose müssen dafür bereits installiert und zugänglich sein; Zelyra
installiert oder verändert Docker nicht. Die [Setup-Anleitung](docs/setup-web.de.md)
erklärt Konsolen- und Browser-Setup, Berechtigungen, Ports und Fehlersuche.

Erzeugte `.env`-Dateien enthalten lokale Zugangsdaten im Klartext. Git und der
Docker-Build-Kontext schließen die Datei aus; neu angelegte Dateien erhalten
unter Unix nur Besitzerrechte (`0600`). Unter Windows gelten die ACLs des
Verzeichnisses; vorhandene `.env`-Dateien werden nicht nachträglich mit neuen
Rechten versehen. Die Datei darf nicht committet oder geteilt werden. Für den
Produktiveinsatz geeignete Secret-Verwaltung und eigene Zugangsdaten nutzen;
0.9.8 ist nicht für den Produktiveinsatz freigegeben.

Ein neu erzeugtes MariaDB-Projekt verwendet standardmäßig Deutsch und den
Lernmodus. Mit `ZELYRA_LANGUAGE=de|en` und `ZELYRA_LEVEL=learn|work` in der
`.env` lässt sich die generierte Oberfläche einstellen. Vorrangregeln und
weitere Einstellungen stehen in der Umgebungsreferenz.

## Dokumentation

| Wenn du … | lies … |
| --- | --- |
| Zelyra Schritt für Schritt lernen möchtest | [Getting Started](docs/getting-started.de.md) · [Deutsches Handbuch](docs/handbook/de/handbuch.md) · [English handbook](docs/handbook/en/handbook.md) |
| den aktuellen Entwicklungsstand und seine Grenzen sehen möchtest | [Umgesetzte Funktionen](docs/implemented.de.md) · [English](docs/implemented.en.md) |
| wissen möchtest, was die Sprache spezifiziert | [Sprachspezifikation](docs/specification.de.md) · [Quellenlandkarte und Prüfanleitung](docs/source-authority.de.md) |
| Projekt oder Umgebung konfigurieren möchtest | [Umgebung und Konfiguration](docs/env.md) · [English reference](docs/env.en.md) |
| Datenbankunterstützung prüfen möchtest | [Datenbank-Supportmatrix](docs/database-support.de.md) · [English](docs/database-support.en.md) |
| den lokalen Editor oder sichere Regex-Änderungen nutzen möchtest | [Zelyra Studio](docs/web-editor.de.md) · [Textänderungsbefehl](docs/text-edit.de.md) |
| aktuelle und geplante Arbeiten prüfen möchtest | [Roadmap](docs/ROADMAP.de.md) · [Entwicklungsplan 0.9.9](docs/release-plans/0.9.9.de.md) · [Releaseplan 1.0.0](docs/release-plans/1.0.0.de.md) |
| die Entwicklungsrichtung verstehen möchtest | [Manifest](docs/MANIFESTO.de.md) · [KI-native Architektur](docs/architecture/ai-native-development.de.md) · [Entwurf lesbarer Regex-Textbearbeitung](docs/design/readable-regex-text-editing.de.md) |
| konkrete Release-Änderungen suchst | [Changelog](CHANGELOG.md) · [GitHub-Releases](https://github.com/sf1976/zelyra/releases) |

Das Handbuch enthält Lernkapitel und detaillierte Sprach-/Laufzeitbeispiele.
Die formale Spezifikation und die Compilertests bestimmen, was aktuell
akzeptiert wird. Die Roadmap unterscheidet zwischen umgesetzt, teilweise,
geplant, optional und zurückgestellt. Zelyra-Syntax darf nicht aus Rust oder
anderen Sprachen abgeleitet werden.

## Leitprinzipien

- **Datenbankorientiert, MariaDB-zuerst:** Schema, geprüftes natives SQL,
  Formulare und erzeugte Businessoberflächen verwenden gemeinsame Typen.
  SQLite ist für getestete Abläufe unterstützt; PostgreSQL 16 bietet eine
  begrenzte experimentelle Runtime für direkte SQL-Abfragen, aber keine
  vollständige Parität oder allgemeine Datenbankportabilität.
- **Views gehören zur Plattform:** Wiederverwendbare Layouts, Komponenten,
  Slots, generierte CRUD-Views, Lokalisierung und lokale Designanpassung
  entwickeln sich gemeinsam weiter. Offene Punkte stehen in der Roadmap.
- **KI-nativ, nicht KI-abhängig:** Compilerdiagnosen, Projektkontext,
  Wirkungsanalyse und Formatierung bieten maschinenverwendbare Schnittstellen.
  KI-Anbieter oder Cloud sind nicht erforderlich; Compiler und Tests bleiben
  maßgeblich.
- **Digitale Souveränität:** Lokale Ausführung und Datenkontrolle, keine
  ungefragte Telemetrie, sichtbare Sicherheitsgrenzen und belegte Aussagen
  leiten das Projekt. Das sind Anforderungen und Ziele, keine Behauptung, dass
  bereits jedes Ziel erreicht ist.

## Lizenz und Implementierung

Der Compiler ist derzeit in Rust implementiert; Rust ist weder Zelyras
Quellsprache noch die Produktidentität. Zelyra-Quellcode steht nach Wahl des
Lizenznehmers unter der MIT-Lizenz oder der Apache License 2.0. Siehe
[LICENSE](LICENSE), [LICENSE-MIT](LICENSE-MIT) und
[Drittanbieterhinweise](THIRD_PARTY_NOTICES.md).
