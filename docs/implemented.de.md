# In Zelyra 0.9.2 umgesetzt

**Stand:** stabiles Release 0.9.2 · Sprachkompatibilitätslinie `0.1` · 2026-10-11
**Reifegrad:** experimentell; nicht für den Produktiveinsatz freigegeben

Diese Übersicht führt Funktionen auf, die im stabilen Release Zelyra 0.9.2
tatsächlich implementiert sind. Eine menschliche Abnahme wird nicht behauptet;
sie bleibt bis 1.0.0 offen. Sie beschreibt den ausgelieferten Umfang,
nicht die gesamte Sprachvision oder zukünftige Vorhaben. Die
[Roadmap](ROADMAP.de.md) dokumentiert offene Arbeiten; das
[Changelog](../CHANGELOG.md) hält die Änderungen der Releases fest. Die Links
führen zu ausführlichen Anleitungen und Testnachweisen, damit diese Seite das
Handbuch nicht doppelt wiedergibt.

## Änderungen in 0.9.2

- ✅ Studio zeigt Kontexthilfe und einen durchsuchbaren Aktionsfinder. Die
  Vervollständigung berücksichtigt deklarierte Projektbegriffe und lokale
  Bindungen aus dem Compiler-AST; Live-Diagnosen prüfen auch ungespeicherte
  Änderungen, ohne Projektdateien zu verändern.
- ✅ F12 öffnet eindeutige Definitionen im aktuellen oder direkt importierten
  Modul. Umschalt+F12 findet lexikalisch gleiche Bezeichner in gespeicherten
  `.zyl`-Dateien; die Suche löst keine Gültigkeitsbereiche auf.
- ✅ Die deterministische Projektkarte zeigt Compiler-erfasste Module,
  Importe, Tabellen, Views, CRUD-Ressourcen, Seiten, APIs und Funktionen mit
  Quellpositionen. Mehrdeutige, dynamische oder begrenzungsbedingt fehlende
  Beziehungen werden nicht geraten.
- ✅ Die ausdrückliche Studio-Aktion **Starten** begrenzt Programme auf
  15 Sekunden und 64 KiB je Ausgabekanal und redigiert erkannte Geheimnisse aus
  Prozessumgebung und `.env`. Studio startet nichts selbstständig; interaktive
  stdin-Eingabe ist nicht verfügbar.
- ✅ `zelyra --tutorial invoice` erzeugt das Rechnungstutorial, startet die
  MariaDB-/Web-Compose-Anwendung und richtet ihr Schema ein. `--no-start`
  überspringt den Docker-Start. Das Beispiel berechnet Netto, 19% Umsatzsteuer
  und Brutto und stellt eine druckbare Browseransicht bereit.
- ✅ Automatisierte Browser- und Tastaturprüfungen ergänzen die Editor-
  Sprachdienste; unabhängige Screenreader-, Kontrast- und Produktabnahme bleibt
  für 1.0.0 vorgesehen.

## Änderungen in 0.9.1

- ✅ Studio setzt seine lokale Sitzung nach dem Neuladen eines Tabs fort. Das
  Sitzungstoken wird aus der Adresszeile entfernt und tab-lokal gehalten.
- ✅ Ungespeicherte Änderungen werden angezeigt; `Strg+S` speichert den aktuellen
  Editorinhalt. Compilerdiagnosen erscheinen im Problems-Bereich.
- ✅ Chromium-E2E deckt Speichern, Neuladen, Fehlerdiagnosen und Neustart mit
  MariaDB- und PostgreSQL-Studioabläufen ab.
- ✅ Das Rechnungstutorial berechnet Netto, 19% Umsatzsteuer und Brutto aus
  verknüpften Positionen. Die druckoptimierte Ansicht nutzt den Browser-PDF-
  Export; es gibt keine native PDF-Engine oder Rechtskonformitätszusage.
- ✅ `#`-Zeilenkommentare und alleinstehende `##`-Blockgrenzen gehören zur
  Sprachlinie 0.1. `//` bleibt für vorhandene Programme kompatibel.

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
  `Result` und exhaustives Pattern Matching als Anweisung oder Ausdruck; ein
  `match`-Ausdruck liefert einen einheitlich typgeprüften Wert.
- ✅ `Int` und `UInt` rechnen innerhalb ihrer 64-Bit-Bereiche; Überlauf,
  Unterlauf und Division durch null werden als Runtimefehler gemeldet. Das
  mehrteilige `examples/type_system` prüft unterstützte Typen, erwartete
  Compiler-/Runtimefehler und dokumentiert partielle sowie fehlende Typen.
  `Unit` ist in Typsignaturen und JSON-`null`-Roundtrips nutzbar. `Decimal`
  verwendet nun Festkomma mit neun Nachkommastellen und exakter Basis-10-
  Arithmetik; für verlustfreie JSON-Roundtrips wird es als Dezimalstring
  serialisiert. `Date`, `Time`, `Duration` und `Bytes` haben jetzt echte
  Runtimewerte, geprüfte Konstruktoren und JSON-Roundtrips. `List<T>` ist ein
  Alias für `T[]`; heterogene `Tuple<A, B, ...>`-Werte und deterministische
  `Set<T>`-Werte lassen sich erstellen, prüfen, iterieren und als JSON
  austauschen. Benutzerdefinierte Enums/Generics und beobachtbare
  Klassenidentität bleiben offen.
- ✅ Stringwerte lassen sich lexikografisch vergleichen. `json_decode_result<T>`,
  `read_text_result` und `write_text_result` geben erwartbare Parse- und
  Dateifehler als `Result` zurück. `format_timestamp` zeigt UTC-Zeitstempel als
  ISO-8601; Capability- und Projektgrenzen bleiben aktiv. Exklusive
  Dateierstellung, temporäres Schreiben mit atomarem Austausch, Anhängen,
  Kopieren, Umbenennen und Result-basiertes Löschen sind verfügbar. Wörtliche
  Textsuche, Ersetzen und Teilen unterstützen mehrteilige Textdateiabläufe.
  `examples/notes_manager` demonstriert UTF-8-Notizen, Sicherungen vor
  Änderungen, Wiederherstellung, Protokollierung und automatisierte Tests.
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
  `zelyra db create|setup|bootstrap|inspect|map|plan|apply|history` für
  unterstützte Operationen. `zelyra db map <file.zyl>` vergleicht deklarierte
  Tabellen nach Quellmodul mit Live-Tabellen, Spalten und Fremdschlüsseln. Die
  Karte ist schreibgeschützt; Modulzuordnung ist unverbindlich und keine
  Autorisierungsregel.
- ✅ Tabellenschemata, Spalten, Schlüssel, Beziehungen, Indizes,
  Schema-Inspektion, Abgleich zwischen Soll- und Ist-Schema sowie SQL-DDL-
  Planung und -Anwendung sind im jeweils dokumentierten Backend-Umfang
  implementiert.
- ✅ Natives SQL wird anhand bekannter Schemata auf Tabellen, Spalten,
  Parameter, Nullability und Ergebnis-Mapping geprüft. Parameter werden sicher
  gebunden; Transaktionen stehen zur Verfügung.
- ✅ SQLite besitzt getestete Schema- und lokale Datenbankabläufe. PostgreSQL-
  Schema-Inspektion/-Planung und ausgewählte Sicherheitsprüfungen sind
  getestet. 0.6.0 ergänzt eine experimentelle native PostgreSQL-16-Runtime-
  Teilmenge für direkte parametrisierte SQL-Abfragen und Transaktionen. Sie
  belegt keine vollständige PostgreSQL-Parität; generierte Web-/CRUD-Pfade
  nutzen sie noch nicht. 0.7.0 ergänzt getestete typisierte MySQL-Abfragen
  ausschließlich auf MySQL Server 8.4.11; Schema-/Administrationsbefehle,
  Migrationen, generiertes CRUD/Web und Authentifizierung sind nicht enthalten.
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
  Design-Token-Overrides. `zelyra routes <entry.zyl>` erstellt eine sortierte
  Übersicht erkannter Seiten-, API-, Formular-, Tableview-, CRUD- und
  Authentifizierungsrouten mit HTTP-Methode und Quellfundstelle. Importierte
  und generierte Ressourcenrouten sind eingeschlossen; überlappende Pfade
  derselben HTTP-Methode führen zu `E-ROUTE-001`. JSON-Ausgabe ist verfügbar.
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
- 🧪 0.7.0 ergänzt einen mandantenbewussten CRUD-Pfad für genau eine
  ausdrücklich deklarierte MariaDB-Tabelle. Sitzung, Membership, Berechtigung
  und generierte Abfragen werden im unterstützten Pfad geprüft. Das ist keine
  vollständige Multi-Tenancy; Beziehungen, eigene SQL-Aktionen, TableViews,
  Exporte, Migrationen und Backfills sind nicht unterstützt.

Details stehen im [Schnelleinstieg für Web und Datenbank](getting-started.de.md),
der [Setup-Anleitung](setup-web.de.md) und den
[Webkapiteln des Handbuchs](handbook/de/handbuch.md).

## Zelyra Studio und Rechnungstutorial

- 🧪 `zelyra editor [directory] [--port <port>] [--no-open]` startet ein lokales Studio
  auf `127.0.0.1` mit Dateibaum, Tabs, Suche/Ersetzen, Vervollständigung,
  Compilerdiagnosen und Formatierung. Standardmäßig öffnet die CLI den
  Systembrowser; `--no-open` überspringt dies. Schreibzugriffe benötigen
  Sitzungstoken und denselben Browser-Origin; Datenbank/Webanwendung starten nicht mit.
  0.8.0 ergänzt eine lesende, anklickbare Beziehungskarte für unterstützte
  MariaDB-/PostgreSQL-Schemata sowie einen geführten CRUD-Quelltextassistenten.
  0.9.0 zeigt dazu betroffene Dateien, Tabellen, deklarierte Beziehungen und
  Routen. Der Kandidat wird mit dem Compiler geprüft, als Diff angezeigt und
  erst nach ausdrücklichem Anwenden mit erneutem Hashvergleich geschrieben.
  0.9.2 ergänzt Kontexthilfe, Aktionsfinder, AST-basierte lokale
  Vervollständigung, Live-Diagnosen, F12-Definitionen, lexikalische
  Umschalt+F12-Fundstellen, eine deterministische Compiler-Projektkarte und
  einen begrenzten ausdrücklichen Run-Befehl. Typinferenz, vollständige
  semantische Referenzen und dynamische Projektbeziehungen sind nicht enthalten.
  MySQL-Live-Introspektion und Änderungen an Schema oder Datensätzen sind hier
  nicht enthalten. Einzelheiten: [Studio-Anleitung](studio.de.md).
- ✅ `zelyra text edit <plan.zedit>` bietet vorschau-basierte Textänderungen aus
  TOML-Plänen mit linearen regulären Ausdrücken, erwarteter Trefferzahl,
  quellgebundener Plan-ID und ausdrücklichem Anwenden. Projektgrenzen und
  unveränderte Eingaben werden vor der Änderung geprüft; Details und Grenzen
  stehen in [Text edit](text-edit.de.md).
- 🧪 Optionales lokales TOTP für MariaDB unterstützt verschlüsselte Faktoren,
  Recovery-Codes, Replay-Schutz, Sperren und Audit. Es ist ein experimenteller
  Vorschaupfad ohne unabhängige Produktionsfreigabe.
- Das Sprachkompatibilitätskorpus 0.1 umfasst in 0.9 zusätzliche typisierte
  API-Fehler sowie Laufzeitfälle der Rechnungsanwendung und von `Result`.
- 🧪 `zelyra --tutorial invoice` erzeugt jetzt die Anwendung und startet ihre
  lokale MariaDB-/Web-Compose-Umgebung einschließlich Schema. `--no-start`
  erstellt nur das Projektgerüst. Das Tutorial führt durch Kunden, Artikel,
  Dashboard und Rechnungserstellung. Die Rechnungsansicht addiert
  Positionspreise netto, berechnet im Lernbeispiel 19% Umsatzsteuer und zeigt
  den Bruttobetrag. Unter `/invoices/RECHNUNGSNUMMER/print` kann der Browser
  die druckoptimierte Rechnung als PDF speichern. Es ist ein Lernpfad, keine
  Produktions- oder Deploymentfreigabe; Details und Grenzen stehen im
  [Rechnungstutorial](tutorials/invoice.de.md).
- 🧪 `zelyra --help` schließt mit einem deutschen und englischen Schnelleinstieg:
  minimales Projekt oder Rechnungstutorial, gefolgt von Befehlen mit kurzer
  Erklärung ihres Zwecks. `zelyra editor` ist als direkter Start im aktuellen
  Projektordner ausgewiesen.
- Studio ist noch keine vollständige IDE oder beliebiger Datenbankdesigner.
- ✅ `examples/inventory_orders/` enthält ein mehrteiliges Konsolenprojekt für
  Lager, Kunden und Bestellungen mit JSON-Speicherung, Cent-genauer
  Rabatt-/Umsatzsteuerberechnung, Stornierung, Aktionsprotokoll und ausführbaren
  Geschäftsprüfungen.

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
- ✅ `zelyra doctor --json` liefert schreibgeschützte Bereitschaftsprüfungen
  mit stabilen Kategorien für Konfiguration, Projekt, Verbindung, Authentifizierung,
  Timeout, Schema und Werkzeuge; Datenbankdiagnosen werden ohne Zugangsdaten
  normalisiert. `zelyra update` prüft
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

Der historische Funktionsstand wurde auf PR-Head `a35a5ee` durch CI-Lauf
[#38057534730](https://github.com/sf1976/zelyra/actions/runs/38057534730)
geprüft: Workspace-Tests, vier MariaDB-Kompatibilitätsjobs, PostgreSQL-16-
Runtime-Integration, Fuzzing, Dependency-Audit sowie Linux-/Windows-CI
bestanden. Paketprüfung [#38057534736](https://github.com/sf1976/zelyra/actions/runs/38057534736)
erstellte deterministische Linux-/Windows-Pakete und prüfte Prüfsummen sowie
SPDX-SBOMs für denselben Funktions-Head. Diese Läufe prüften die Funktionsimplementierung vor der Versionsanhebung.
Der versionsangehobene Kandidat bestand anschließend CI-Lauf #38058823265.
Der unveränderliche Tag `v0.6.0-rc.1` bestand Tag-CI #38059555233; Release-
Workflow #38059555140 erstellte die RC-Pakete. Der stabile Release-Workflow
#38060275164 erstellte Linux-/Windows-Pakete, prüfte Prüfsummen und SPDX-SBOMs
mit 228 Paketen und veröffentlichte GitHub-Attestierungen.
Heruntergeladene Release-Artefakte bestanden Prüfsummen- und SBOM-Prüfung; das
Linux-Binary meldet `zelyra 0.6.0`. Siehe den
[Release-Prüfnachweis](release-readiness/0.6.0-candidate-verification.md).
Zelyra 0.9.1 wurde in Merge-Commit `aeba34eb73d327cf9d2b65bfaa0e2ce477802af6` veröffentlicht. Exakte CI-, Artefakt-, Prüfsummen-, SBOM-, Attestierungs- und Website-Nachweise stehen im [0.9.1-Releaseprüfnachweis](release-readiness/0.9.1-candidate-verification.md). Die unabhängige menschliche Abnahme bleibt für 1.0.0 reserviert.

Diese Übersicht wird angepasst, wenn sich ausgelieferte Funktionen oder ihre
Nachweise ändern. Ein Spezifikations- oder Roadmap-Eintrag allein wird hier
nicht als Implementierung ausgegeben.
