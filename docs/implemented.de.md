# In Zelyra 0.2.0 umgesetzt

**Stand:** Compiler-Release `0.2.0` · Sprachkompatibilitätslinie `0.1` · 2026-09-20
**Reifegrad:** experimentell; nicht für den Produktiveinsatz freigegeben

Diese Übersicht führt Funktionen auf, die im Zelyra-Code für Version 0.2.0
tatsächlich implementiert sind. Sie beschreibt den ausgelieferten Umfang,
nicht die gesamte Sprachvision oder zukünftige Vorhaben. Die
[Roadmap](ROADMAP.de.md) dokumentiert offene Arbeiten; das
[Changelog](../CHANGELOG.md) hält die Änderungen der Releases fest. Die Links
führen zu ausführlichen Anleitungen und Testnachweisen, damit diese Seite das
Handbuch nicht doppelt wiedergibt.

## Statuszeichen

- ✅ **Umgesetzt und geprüft:** im angegebenen Umfang verfügbar und durch Tests
  oder dokumentierte Release-/CI-Prüfungen belegt.
- 🧪 **Teilweise oder experimentell umgesetzt:** der funktionierende Teilumfang
  wird ausdrücklich beschrieben; daraus folgt keine breitere Unterstützung.

Kein Punkt bedeutet allgemeine Produktionsreife, universelle Portabilität oder
den Beweis, dass beliebige Anwendungen korrekt sind.

## Sprache und Compiler

- ✅ Lexer, Parser, Syntaxbaum, Namens- und Typprüfung, Funktionen, Ausdrücke,
  Bedingungen, Schleifen und unveränderliche Bindings als Standard;
  Veränderlichkeit muss ausdrücklich markiert werden.
- ✅ Nominale Fachdaten-Typen, Records mit geprüftem Feldzugriff, Arrays mit
  gebräuchlichen Operationen, deterministische typisierte Maps, `Option`,
  `Result` und Pattern Matching im implementierten Sprachumfang.
- ✅ `zelyra check`, `build`, `run` und `serve`; menschenlesbare Diagnosen,
  stabile Diagnosecodes in unterstützten Maschinenformaten und Quellpositionen.
- ✅ Kanonische Formatierung mit `zelyra fmt` und nicht veränderndem
  `--check`-Modus.
- 🧪 Funktions-Capabilities und zur Laufzeit geprüfte Contracts sind für
  unterstützte Operationen implementiert. `zelyra verify` unterscheidet
  `PROVEN`, `RUNTIME_CHECK`, `UNPROVEN` und `FAILED` für eine begrenzte Menge
  von Ausdrücken und Pfaden; es ist kein allgemeiner Theorembeweiser.

Siehe die [formale Sprachspezifikation](specification.de.md), die
[Quellenlandkarte](source-authority.de.md) und die
[Sprachkapitel im Handbuch](handbook/de/handbuch.md).

## Datenbanken und SQL

- ✅ MariaDB ist das primäre getestete Runtime-Backend. Die CLI implementiert
  `zelyra db create|setup|bootstrap|inspect|plan|apply` für unterstützte
  Operationen.
- ✅ Tabellenschemata, Spalten, Schlüssel, Beziehungen, Indizes,
  Schema-Inspektion, Abgleich zwischen Soll- und Ist-Schema sowie SQL-DDL-
  Planung und -Anwendung sind im jeweils dokumentierten Backend-Umfang
  implementiert.
- ✅ Natives SQL wird anhand bekannter Schemata auf Tabellen, Spalten,
  Parameter, Nullability und Ergebnis-Mapping geprüft. Parameter werden sicher
  gebunden; Transaktionen stehen zur Verfügung.
- ✅ SQLite besitzt getestete Schema- und lokale Datenbankabläufe. PostgreSQL-
  Schema-Inspektion/-Planung und ausgewählte Sicherheitsprüfungen sind
  getestet; das belegt keine PostgreSQL-Runtime-Parität.
- 🧪 Der Schema-Planer stuft ausgewählte Änderungen als sicher,
  prüfungsbedürftig, destruktiv oder nicht unterstützt ein. Destruktive und
  nicht unterstützte Änderungen werden fail-closed behandelt; ausgewählte
  Änderungen an befüllten Tabellen und Nullability erhalten schreibgeschützte
  Vorprüfungen. Prüfungsbedürftige Änderungen benötigen eine ausdrückliche
  Freigabe.

Die genauen Backend-Grenzen und getesteten MariaDB-Versionen stehen in der
[Datenbank-Kompatibilitätsmatrix](database-compatibility.de.md).

## Webanwendungen, Views, Formulare und CRUD

- ✅ Integrierter HTTP-Server, Seiten, Routenparameter, typisierte
  View-Interpolationen, sicheres HTML-Escaping und projektlokale
  Design-Token-Overrides.
- ✅ Schemaabhängige Formulare mit Validierung, parametrierten Datenbankaktionen,
  Transaktionen, Weiterleitungen und geprüften Beziehungs-Auswahlelementen für
  unterstützte Fälle.
- ✅ Erzeugte CRUD-Abläufe für Liste, Detail, Anlegen, Bearbeiten und Löschen
  mit Suche, typisierten Filtern, Sortierung, Pagination, Berechtigungs- und
  CSRF-Schutz.
- 🧪 Wiederverwendbare benannte Views/Layouts, typisierte Component-Properties,
  Default- und benannte Slots mit Fallback sowie CRUD-Darstellungsoptionen
  funktionieren in den dokumentierten Kompositionspfaden. Generierte
  CRUD-Inhalte werden in den Default-Slot eingesetzt; unterstützte eigene
  Inhalte benannter Slots sind compile-geprüftes statisches Markup bzw.
  Components. Projektlokale CSS-Token-Overrides funktionieren; ein vollständiger
  Theme-Editor ist dies nicht.
- ✅ Eingebaute deutsche und englische Kataloge decken die erzeugte Oberfläche
  ab. Projektlokale Sprachdateien können unterstützte Katalogtexte ergänzen
  oder überschreiben. `ZELYRA_LANGUAGE=de|en` wählt die Sprache;
  `ZELYRA_LEVEL=learn|work` schaltet die kontextbezogene Lernhilfe.
- ✅ Erzeugte MariaDB-Starter umfassen Minimal-, CRUD-, Authentifizierungs- und
  Businessvarianten; die Maschinenverwaltung enthält optionale fiktionale
  Beispieldaten.

Details stehen im [Schnelleinstieg für Web und Datenbank](getting-started.de.md),
der [Setup-Anleitung](setup-web.de.md) und den
[Webkapiteln des Handbuchs](handbook/de/handbuch.md).

## APIs, Authentifizierung, Berechtigungen und Audit

- ✅ Typisierte API-Deklarationen prüfen unterstützte Eingaben, Ausgaben und
  Fehler. JSON-Serialisierung, OpenAPI-3.0.3-Erzeugung und TypeScript-
  Clientgenerierung stehen über `zelyra doc` zur Verfügung.
- ✅ MariaDB-basierte Passwortauthentifizierung verwendet Argon2 und persistente
  HttpOnly-Sessions. Direkte und rollenbasierte Berechtigungen, geschützte
  Aktionen, CSRF-geschützte Browserverwaltung und Schutz der letzten
  Administratorzuordnung sind implementiert.
- ✅ Optionales MariaDB-Audit-Logging unterstützt Inspektion, begrenzten
  JSON-/CSV-Export, strukturelle Prüfung und ausdrücklich bestätigtes Bereinigen.
- 🧪 Manipulationsnachweis durch kryptografische Audit-Verkettung ist optional
  verfügbar und durch Hash-Integritätstests abgedeckt; dadurch wird die
  Datenbank weder unveränderlich noch ersetzt dies unabhängige Audit-Kontrollen.

## Setup, Konfiguration und Distribution

- ✅ `zelyra new` und `zelyra init` erzeugen Projekte und Templates. Erzeugte
  MariaDB-Projekte enthalten Compose-Konfiguration und eine lokale `.env`;
  Zugangsdaten werden nicht ausgegeben, neu erzeugte Unix-`.env`-Dateien
  erhalten Modus `0600`, und `.env` wird aus erzeugten Git-/Docker-Kontexten
  ausgeschlossen.
- ✅ `zelyra setup` bietet Konsolenaktionen und einen nur an Loopback gebundenen
  Browser-Assistenten, startet erzeugte lokale MariaDB-/Web-Stacks, wendet das
  Anfangsschema an, unterstützt Portwahl und gibt die App-Adresse aus.
- ✅ `zelyra doctor` prüft Projekt-, Datenbank- und Compose-Bereitschaft im
  dokumentierten schreibgeschützten Umfang. `zelyra update` prüft
  Release-Prüfsummen vor dem Austausch unterstützter Linux-/Windows-x86_64-
  Binärdateien.
- ✅ Benutzerlokale Installation aus dem Quellcode ist verfügbar.
  Veröffentlichte Linux- und Windows-x86_64-Release-Archive enthalten
  SHA-256-Prüfsummen; wiederholte Builds waren in den festgelegten
  Release-CI-Toolchains byte-identisch.

Konfigurationsvorrang, Umgang mit Secrets und Plattformgrenzen erläutern die
[Umgebungsreferenz](env.md) und die [Setup-Anleitung](setup-web.de.md).

## KI-native Compiler-Schnittstellen

- ✅ `zelyra check --format=json` und `zelyra context --format=json` liefern
  versionierte maschinenlesbare Ausgaben für ihren dokumentierten Umfang.
- 🧪 `zelyra impact --format=json` liefert deterministische,
  quelltextbasierte Wirkungsinformationen zu unterstützten Deklarationen und
  Referenzen.
- ✅ `zelyra edit --format=json` zeigt unterstützte semantische
  Umbenennungen als Vorschau; die Anwendung ist ausdrücklich und das geänderte
  Projekt wird erneut vom Compiler geprüft.
- ✅ Typisierte Ausdruckslücken (`_`) liefern kontextbezogene Diagnosen und
  verhindern, dass unvollständige Programme gebaut oder ausgeführt werden.

Diese Werkzeuge rufen keinen KI-Anbieter auf. Siehe die
[KI-native Architektur](architecture/ai-native-development.de.md) und deren
[Benchmark-Spezifikation](benchmarks/ai-authoring.de.md); hier werden keine
vergleichenden Benchmark-Ergebnisse behauptet.

## Test- und Release-Nachweise

Das Repository enthält Workspace-Tests, Tests für Erststart und Wiederherstellung
erzeugter Projekte, MariaDB-Integrationen für CRUD/Auth/API/Tableview/Audit,
SQLite-Schema-Integration sowie PostgreSQL-Schema-Sicherheitsprüfungen. Die
0.2.0-CI-Matrix prüft zentrale MariaDB-Abläufe mit `10.11.19`, `11.4.13`,
`11.8.9` und `12.3.3`. Release-Nachweise stehen im
[Roadmap-Eintrag zu 0.2.0](ROADMAP.de.md#release-meilenstein-020) und im
[Changelog](../CHANGELOG.md).

Diese Übersicht wird angepasst, wenn sich ausgelieferte Funktionen oder ihre
Nachweise ändern. Ein Spezifikations- oder Roadmap-Eintrag allein wird hier
nicht als Implementierung ausgegeben.
