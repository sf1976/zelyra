# Zelyra

![Zelyra-Logo](assets/zelyra-logo.png)

**Von der Datenbank zur Anwendung.** · **Die KI schreibt. Zelyra prüft.**

Deutsch · [English](README.md)

Zelyra ist eine eigenständige, statisch typisierte Sprache und Plattform für
Business-, Datenbank- und Webanwendungen. Schemaabhängiges SQL, Formulare,
CRUD, Views und Geschäftslogik greifen ineinander; gewöhnlicher Programmcode
und natives SQL bleiben jederzeit möglich.

> **Release 1.1.1 · Sprachkompatibilitätslinie: 0.1** · [GitHub-Release](https://github.com/sf1976/zelyra/releases/tag/v1.1.1)
>
> Die Compilerprüfung und der Verifier decken klar abgegrenzte Bereiche ab. Sie beweisen nicht die Korrektheit beliebiger Anwendungen. Prüfe vor dem Einsatz die unterstützten Abläufe und Grenzen in der jeweiligen Dokumentation.

Zelyra 1.1.1 enthält die Studio-Korrekturen aus den
[Release-Notizen 1.1.1](docs/release-notes/1.1.1.de.md). Zelyra 1.1.0 ergänzte
ein offlinefähiges, eingebettetes Studio mit Sprachdiensten und einen
eigenständigen LSP sowie typgeprüfte
Stringinterpolation und ausdrücklich aktivierte In-place-Änderungen lokaler
Arrays. Die Sprachfunktionen aus 1.0.7 bleiben verfügbar. Details stehen in
den [Release-Notizen 1.1.0](docs/release-notes/1.1.0.de.md), im
[UI-Leitfaden](docs/ui.de.md), in der
[Datenbank-Supportmatrix](docs/database-support.de.md) und der
[Implementierungsübersicht](docs/implemented.de.md).

Die Sprachkompatibilitätslinie bleibt `0.1`; Compiler-Releases ändern sie nicht automatisch. „Korrektheit beweisen“ ist ein Entwicklungsziel: Der Verifier unterscheidet geprüfte, zur Laufzeit geprüfte und nicht bewiesene Bereiche, ist aber kein allgemeiner Theorembeweiser.

## Was Zelyra 1.1.1 bietet

Zelyra verbindet Sprache, Compiler, Datenbank, Webanwendung und Studio in
einem Arbeitsablauf. Release 1.1.1 umfasst die Funktionen aus 1.1.0 und die
oben genannten Studio-Korrekturen:

- statisch geprüfte Geschäftslogik und schemaabhängiges SQL;
- MariaDB als primäre Laufzeit sowie geprüfte MySQL- und PostgreSQL-Abläufe;
- generierte Formulare, Tabellenansichten, CRUD, Beziehungen und APIs;
- die erweiterte Standardbibliothek aus 1.0.3 für Mathematik, Text, Arrays,
  Sets, Maps, Datumsrechnung, Parsing, Encodings und Pfade sowie `|>`-Pipelines
  und flache `else if`-Kaskaden;
- offlinefähiges Studio mit CodeMirror 6, Compilerdiagnostik und Sprachdiensten,
  semantischer Navigation/Umbenennung, Verifier-Status, Projekt-Datenkarte,
  Typed-Hole-Hilfe, isolierter UI-/Theme-Vorschau, Capability-Prüfung,
  Laufzeitbudgets, PDF-Vorschau und MariaDB-Outbox-Anzeige mit erneutem Einreihen;
- eigenständiges `zelyra lsp` mit denselben Projekt-Overlays und Sprachdiensten;
- lokale, um Geheimnisse bereinigte JSONL-Logs für Programmlauf und Vorschau,
  abrufbar in Studio, sowie `zelyra uninstall` für die aktive benutzereigene
  Installation;
- Zelyra UI mit compilergeprüften Komponenten, Gestaltung der erzeugten CRUD-
  Seiten, hellen/dunklen Themes, kompakter Dichte, lokaler Asset-Auslieferung,
  Druckstilen und textuell gleichwertigen Balken-, Linien- und Ringdiagrammen;
- ein Rechnungstutorial mit wiederholbaren Beispieldaten, freier Portwahl,
  isolierten Compose-Volumes, aufgabenorientierter Startseite, reinem Datumsfeld,
  serverseitiger Rechnungsnummer und vollständiger deutscher und englischer
  Quelltexterklärung;
- CLI-Werkzeuge für Routen, Setup, Diagnose, Datenbank, Audit, SMTP und statische
  Laravel-Migrationsberichte.
- If-Ausdrücke, Record-Updates, typisierte Lambdas, geschützte verschachtelte
  Muster, Tupel, Stringinterpolation und ausdrücklich veränderliche lokale
  Arrays; `--max-steps` stoppt unbeschränkte Auswertung mit quellgenauer Diagnose.
- Lokales Text-PDF-Rendering und geprüfte PDF-Outbox-Anhänge mit Größen- und
  Zeilenlimits. Das Renderer-PDF übernimmt keine CSS-Layouts, Bilder oder
  Diagramme; für grafische Ausdrucke bleibt die Browser-Druckansicht verfügbar.

Docker Engine und das Docker-Compose-Plugin v2 oder neuer müssen installiert
sein; Python Compose v1 wird nicht unterstützt. Zelyra installiert diese
Pakete nicht selbst. Prüfe mit `docker compose version`, ob das Plugin
verfügbar ist. Die [Rechnungstutorial-Anleitung](docs/tutorials/invoice.de.md)
erklärt Start, Bearbeitung in Studio, Beispieldaten und Fehlerbehebung.

Die [Implementierungsübersicht](docs/implemented.de.md) beschreibt den
Funktionsumfang und seine Grenzen. Die [Roadmap](docs/ROADMAP.de.md) führt
geplante Arbeiten; [Release-Notizen](docs/release-notes/) und
[Changelog](CHANGELOG.md) dokumentieren veröffentlichte Änderungen.

Mit `zelyra uninstall --dry-run` lässt sich anzeigen, welche aktive
benutzereigene Programmdatei entfernt würde. Der
[Deinstallationsleitfaden](docs/uninstall.de.md) erklärt paketverwaltete
Installationen und doppelte PATH-Einträge.

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
Prüfe für produktive Anwendungen die konkret unterstützten Abläufe,
Sicherheitsanforderungen und Grenzen in den zugehörigen Handbüchern.

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
| Zelyra UI verwenden möchtest | [Komponenten-, Formular-, Theme-, Druck-, Diagramm- und E-Mail-Referenz](docs/ui.de.md) · [English](docs/ui.en.md) |
| aktuelle und geplante Arbeiten prüfen möchtest | [Roadmap](docs/ROADMAP.de.md) · [Release-Notizen 1.0.7](docs/release-notes/1.0.7.de.md) |
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
