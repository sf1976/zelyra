# Zelyra

![Zelyra-Logo](assets/zelyra-logo.png)

**Von der Datenbank zur Anwendung.** · **Die KI schreibt. Zelyra prüft.**

Deutsch · [English](README.md)

Zelyra ist eine eigenständige, statisch typisierte Sprache und Plattform für
Business-, Datenbank- und Webanwendungen. Schemaabhängiges SQL, Formulare,
CRUD, Views und Geschäftslogik greifen ineinander; gewöhnlicher Programmcode
und natives SQL bleiben jederzeit möglich.

> **Aktueller Compilerstand: 0.2.0 · Sprachkompatibilitätslinie: 0.1 · experimentell**
>
> Zelyra 0.2.0 ist nicht für den Produktiveinsatz freigegeben. „Korrektheit
> beweisen“ ist ein Entwicklungsziel: Der aktuelle Verifier deckt nur einen
> begrenzten Teil ab und beweist nicht die Korrektheit beliebiger Anwendungen.

## Was ist neu in 0.2.0?

Dieses Release ergänzt wiederverwendbare View-Layouts und benannte
Komponenten-Slots, projektlokale deutsche und englische UI-Kataloge,
strengere Prüfungen von Schemaabweichungen mit Freigabeschritten und sicheren
Vorprüfungen sowie zusätzliche Tests für MariaDB-Businessanwendungen.
Browser-Origin-Prüfungen und eine konfigurierbare Host-Allowlist härten die
Web-Laufzeit. Linux- und Windows-Release-Archive enthalten SHA-256-Prüfsummen;
wiederholte Builds sind in den festgelegten CI-Toolchains byte-identisch.

Die [Changelog-Einträge zu 0.2.0](CHANGELOG.md) enthalten die Release-Notizen;
die [Roadmap](docs/ROADMAP.de.md) hält Implementierungsstand und Grenzen fest.
Diese Quellen enthalten die Details. Das README ist ein Projektüberblick und
soll kein zweites Handbuch oder Changelog sein.

Der [Entwurf der 0.3.0-Release-Notizen](docs/release-notes/0.3.0.de.md) führt
geplanten Umfang und Grenzen auf; er ist keine veröffentlichte Release-
Ankündigung.

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
[I/O-Kapitel im Handbuch](docs/handbook/de/handbuch.md#kapitel-8-ein--und-ausgaben)
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
0.2.0 ist nicht für den Produktiveinsatz freigegeben.

Ein neu erzeugtes MariaDB-Projekt verwendet standardmäßig Deutsch und den
Lernmodus. Mit `ZELYRA_LANGUAGE=de|en` und `ZELYRA_LEVEL=learn|work` in der
`.env` lässt sich die generierte Oberfläche einstellen. Vorrangregeln und
weitere Einstellungen stehen in der Umgebungsreferenz.

## Dokumentation

| Wenn du … | lies … |
| --- | --- |
| Zelyra Schritt für Schritt lernen möchtest | [Getting Started](docs/getting-started.de.md) · [Deutsches Handbuch](docs/handbook/de/handbuch.md) · [English handbook](docs/handbook/en/handbook.md) |
| sehen möchtest, was Compiler 0.2.0 tatsächlich kann | [Umgesetzte Funktionen](docs/implemented.de.md) · [English](docs/implemented.en.md) |
| wissen möchtest, was die Sprache spezifiziert | [Sprachspezifikation](docs/specification.de.md) · [Quellenlandkarte und Prüfanleitung](docs/source-authority.de.md) |
| Projekt oder Umgebung konfigurieren möchtest | [Umgebung und Konfiguration](docs/env.md) · [English reference](docs/env.en.md) |
| Datenbankunterstützung prüfen möchtest | [MariaDB-Kompatibilitätsmatrix](docs/database-compatibility.de.md) · [English](docs/database-compatibility.en.md) |
| aktuelle und geplante Arbeiten prüfen möchtest | [Roadmap](docs/ROADMAP.de.md) · [Releaseplan 0.3.0](docs/release-plans/0.3.0.de.md) |
| die Entwicklungsrichtung verstehen möchtest | [Manifest](docs/MANIFESTO.de.md) · [KI-native Architektur](docs/architecture/ai-native-development.de.md) |
| konkrete Release-Änderungen suchst | [Changelog](CHANGELOG.md) · [GitHub-Releases](https://github.com/sf1976/zelyra/releases) |

Das Handbuch enthält Lernkapitel und detaillierte Sprach-/Laufzeitbeispiele.
Die formale Spezifikation und die Compilertests bestimmen, was aktuell
akzeptiert wird. Die Roadmap unterscheidet zwischen umgesetzt, teilweise,
geplant, optional und zurückgestellt. Zelyra-Syntax darf nicht aus Rust oder
anderen Sprachen abgeleitet werden.

## Leitprinzipien

- **Datenbankorientiert, MariaDB-zuerst:** Schema, geprüftes natives SQL,
  Formulare und erzeugte Businessoberflächen verwenden gemeinsame Typen.
  SQLite ist für getestete Abläufe unterstützt; PostgreSQL-Schemaunterstützung
  bedeutet weder vollständige Runtime-Parität noch allgemeine Portabilität.
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
