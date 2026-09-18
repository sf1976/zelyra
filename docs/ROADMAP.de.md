# Zelyra Roadmap

Dies ist die dauerhaft gepflegte Roadmap für Zelyra. Sie enthält notwendige
Arbeiten ebenso wie optionale Ideen. Ein Punkt gilt erst dann als umgesetzt,
wenn — soweit anwendbar — Syntax oder API-Dokumentation, Implementierung,
positive und negative Tests, Diagnostik und ein funktionierendes Beispiel
vorhanden sind.

Status: `[x]` umgesetzt, `[~]` in Arbeit, `[ ]` geplant, `[?]` optional oder
noch in Prüfung.

## Aktuelle Meilensteine

- [x] Sprachkern mit Lexer, Parser, AST, Funktionen, Ausdrücken, Kontrollfluss,
  unveränderlichen Bindings als Standard, Arrays, Records, Option, Result und
  Pattern Matching.
- [x] Statische Prüfung, nominale Fachdaten-Typen, Capabilities, Contracts,
  Runtime-Prüfungen und ein erster Verifier.
- [x] MariaDB als primäres Schema-Backend, Inspektion, DDL-Planung,
  Schema-Anwendung, SQLite-Unterstützung und PostgreSQL-Schema-Unterstützung.
- [x] Typisiertes natives SQL mit Schema-, Spalten-, Parameter-, Nullability-,
  Ergebnis-, Transaktions- und sicherer Parameterprüfung.
- [x] HTTP-Server, Seiten, Formulare, CRUD, Suche, Filter, Sortierung,
  Pagination, APIs, OpenAPI und TypeScript-Client-Erzeugung.
- [~] Wiederverwendbare Web-Views: benannte Layouts, Seitenkomposition, ein
  validierter Content-Slot und typisierte selbstschließende Komponenten mit
  Properties sind verfügbar. Mehrere Slots, Themes und CRUD-View-
  Überschreibungen folgen.
- [x] Authentifizierung, persistente Sessions, CSRF, Argon2-Passwörter, direkte
  und rollenbasierte Berechtigungen, Browserverwaltung und MariaDB-Audit.
- [~] Audit-Befehle: Anzeige, JSON/CSV-Export, strukturelle Prüfung und
  bestätigtes Bereinigen sind verfügbar. Manipulationssichere Verkettung ist
  optional verfügbar; Archivierung und Aufbewahrung bleiben offen.

## Querschnitt: KI-native Entwicklung

Das strategische Produktziel lautet: **Die KI schreibt. Zelyra prüft.**
Menschen und KI-Systeme sind gleichwertige Codeautoren; Compiler, Tests,
Capabilities und Sicherheitsregeln bleiben jedoch maßgeblich. Dies ist eine
KI-native und KI-unabhängige Architekturanforderung für jede Phase, kein
Feature eines bestimmten Anbieters.

- [~] **Stufe A — Maschinenbasis:** Versionierte JSON-Diagnosen für
  `check --format=json`, stabile Codes, Source-Spans, deterministische
  Ausgabe, eine schreibgeschützte Projektübersicht mit
  `context --format=json` und Format-Tests sind implementiert. Weitere
  Befehle und eine vollständige Secret-Redaction bleiben offen.
- [x] **Stufe B — kanonischer Quellcode:** Das deterministische `zelyra fmt`
  formatiert parsebaren Quellcode, unterstützt `--check` für CI, bewahrt
  Kommentare sowie rohe SQL-/HTML-Blöcke und besitzt Idempotenz- und
  Semantiktests.
- [~] **Stufe C — typisierte Lücken:** Ausdrucks-Lücken mit `_` melden jetzt
  erwartete Kontexttypen, sichtbare Werte/Funktionen, Capabilities,
  Contract-Pflichten und Source-Spans; unvollständiger Code darf nicht gebaut
  oder ausgeführt werden. Lücken in weiteren Deklarationskontexten und eine
  umfassendere Edit-Integration bleiben offen.
- [~] **Stufe D — Wirkungsanalyse:** Eine deterministische, quelltextbasierte
  erste Stufe von `zelyra impact --format=json` meldet Tabellen, SQL,
  Formulare, CRUD, Views, APIs, Berechtigungen und Contracts; E-Mails, Jobs,
  Tests und Live-Schemaänderungen bleiben anzubinden.
- [ ] **Stufe E — semantische Änderungen:** Validierte, atomare,
  vorschau-fähige und versionierte `zelyra edit --format=json`-Operationen
  zusätzlich zu Textpatches.
- [~] **Stufe F — Contracts und Effekte:** Contracts und Capability-Prüfung
  existieren; feinere Effekte wie `Database(read)`, `Database(write)`,
  `Email` und `Jobs` sind geplant. KI darf Effekte niemals unbemerkt ergänzen.
- [ ] **Stufe G — Benchmark:** Reproduzierbaren KI-Autorenschaftsbenchmark
  etablieren, bevor vergleichende Eignungsbehauptungen veröffentlicht werden.
  Ergebnisse bleiben bis zu echten kontrollierten Versuchen leer.

## 1. Einstieg und Distribution

- [ ] Ein-Befehl-Installation für Linux, Windows und macOS.
- [ ] Signierte Release-Binaries und Checksums für jede unterstützte Plattform.
- [ ] Installations-, Update- und Deinstallationsskripte ohne Rust oder Cargo.
- [ ] First-Run-Assistent für Projekt, MariaDB, Secrets und Webserver-Port.
- [ ] Verständliche Docker-Compose-Vorlagen für MariaDB und internen Webserver
  mit konfigurierbaren Host- und Container-Ports.
- [ ] Optionale automatische Reverse-Proxy-Einrichtung für Apache und Nginx
  mit sicheren Defaults und Vorschau der Konfiguration.
- [ ] `zelyra doctor` für Datenbank, Ports, TLS, Dateirechte und Werkzeuge mit
  verständlichen zweisprachigen Handlungsempfehlungen.
- [ ] Projektvorlagen: minimales Skript, MariaDB-CRUD, API, Authentifizierung
  und Produktionsdeployment.
- [ ] Offline-Installationspaket und reproduzierbare Toolchain-Metadaten.
- [?] Paketmanager-Distribution, soweit sinnvoll (Homebrew, winget,
  Debian-Pakete und Container-Images).

## 2. Sprache und Compiler

- [ ] Stabile Grammatik-Spezifikation und versionierte Kompatibilitätsregeln.
- [ ] Module, Imports, Sichtbarkeit, Namespaces und Multi-File-Projekte.
- [ ] Generics, Interfaces/Traits, Enums, Tagged Unions und Pattern Matching
  für alle Fachdaten-Typen.
- [ ] Bessere Typinferenz mit präzisen Quellpositionen und Fix-Vorschlägen.
- [ ] Typisierte Literale und Konversionen für Decimal, Money, Date, Time,
  UUID, URL, Email, Bytes und Duration.
- [ ] Explizites Resource-/Effect-Modell für Database, Network, FileSystem,
  Environment, Process, Clock und Random.
- [ ] Strukturierte Fehlerweitergabe und eigene Fehlertypen.
- [ ] Deterministischer Build-Graph, inkrementelle und parallele Kompilierung.
- [ ] Language Server, Editor-Erweiterungen, Formatter, Linter und Debugger.
- [ ] Stabile IR und ein backendunabhängiges Runtime-ABI.
- [ ] Langfristiger Self-Hosting-Pfad: Compiler-Werkzeuge schrittweise aus Rust
  nach Zelyra verlagern, mit einem kleinen vertrauenswürdigen Bootstrapcompiler.
- [?] Native Codegenerierung über das Bootstrap-Backend hinaus (LLVM,
  Cranelift oder ein anderes gepflegtes Backend).
- [?] Paket-Registry und Lockfile-Konzept.

## 3. Datenbankplattform

- [x] MariaDB als primäres getestetes Backend.
- [x] SQLite für lokale und eingebettete Anwendungen.
- [x] PostgreSQL für Schema und Planung.
- [ ] Vollständige PostgreSQL-Runtime-Parität.
- [ ] MariaDB-/MySQL-Kompatibilitätsmatrix und versionsabhängige Diagnostik.
- [ ] SQL-Server-Backend prüfen und bei ausreichendem Bedarf implementieren.
- [ ] Reversible Migrationspläne, Rollback-Hinweise, Backups und Driftberichte.
- [ ] Bessere Analyse destruktiver Änderungen, Zeilenschätzungen,
  Lock-Warnungen und Planung von Wartungsfenstern.
- [ ] Connection Pooling, Retries, Timeouts, Abbruch und Health Checks.
- [ ] Streaming großer Ergebnisse und begrenzter Speicherverbrauch.
- [ ] N+1-Erkennung, Query-Plan-Hinweise, Slow-Query-Diagnostik und Monitoring.
- [ ] Typisierte Relationen, Joins, Aggregate, Subqueries, CTEs, Unions und
  datenbankspezifische Erweiterungen.
- [ ] Read Replicas, Read/Write-Routing, Mandantentrennung und Umgebungen.
- [?] Seed-, Fixture-, Snapshot- und anonymisierte Testdaten-Befehle.

## 4. Views und Webdarstellung

- [~] Benannte Views/Layout mit `view: Name` an Seiten und validiertem
  `<slot />`-Inhaltsslot.
- [ ] Typisierte View-Ausdrücke mit Prüfung von Variablen, Feldern, Option-Werten
  und sicherem Escaping.
- [~] Benannte Komponenten mit typisierten Properties sind verfügbar; typisierte
  Events bleiben geplant.
- [~] Deklarative MariaDB-`tableview`-Routen mit geprüften SQL-Quellen,
  deklarierten Spalten, typisierten Filtern, Suche, Sortierung, Pagination,
  URL-Zustand und Escaping sind für tabellen- und struct-basierte Ergebnistypen
  verfügbar.
- [~] Komponenten unterstützen Default- und benannte Slots sowie verschachtelte
  Komposition; Fallback-Inhalte und verschachtelte Views bleiben geplant.
- [ ] View-Vererbung/-Komposition ohne versteckten globalen Zustand.
- [ ] View-lokales Laden von Daten mit expliziten Query- und Berechtigungsgrenzen.
- [~] Typisierte CRUD-Filteroperatoren (`eq`, Textsuche, Zahlenvergleiche und
  NULL-Prüfungen) werden in sichere serverseitige SQL-Abfragen kompiliert.
- [~] Erzeugte CRUD-Filtersteuerungen bewahren Operator- und Wertzustand in
  URLs; stabile Sortierung und weitere Barrierefreiheitsverbesserungen bleiben
  offen.
- [~] Der erste Teil einer einheitlichen typisierten View-Pipeline ist für
  `tableview`-Routen mit deklarativen Filtern, Suche, Sortierung und Pagination
  verfügbar; dieselbe Pipeline für beliebige Views bleibt geplant.
- [ ] Zusammensetzbare Filterausdrücke mit typisierten Operatoren für
  Datumswerte, Booleans, Beziehungen und Volltextsuche.
- [ ] Wiederverwendbare Navigation, Tabellen, Formulare, Dialoge, Hinweise,
  Pagination und Validierungsfehler-Komponenten.
- [~] CRUD-View-Überschreibungen für Listen unterstützen sicher die Modi
  `table`/`cards` und eine eigene Leerzustandsmeldung bei Erhalt der
  generierten Abfrage-, Auth- und Aktionsprüfungen.
- [~] CRUD-View-Überschreibungen für Details unterstützen sicher die Modi
  `standard`/`cards` und eine eigene Überschrift bei Erhalt der generierten
  Aktions-, CSRF-, Auth- und Escaping-Prüfungen.
- [~] CRUD-View-Überschreibungen für Formulare unterstützen sicher die Modi
  `standard`/`cards` sowie eigene Überschriften und Absende-Beschriftungen bei
  Erhalt der Validierungs-, CSRF-, Parameter- und Berechtigungsprüfungen.
- [~] CRUD-View-Überschreibungen für Löschbestätigungen unterstützen eigene
  Überschriften, Warnungen und Absende-Beschriftungen bei Erhalt der POST-only-,
  CSRF- und Berechtigungsprüfungen.
- [~] CRUD-Lademetadaten und konfigurierbare Fehleransichten erhalten Escaping
  und generische Datenbankfehlergrenzen; eine clientseitige Ladeanzeige bleibt
  offen.
- [~] Eigene CRUD-Aktionen können parametrisiertes, POST-only-Geschäfts-SQL mit
  CSRF-, Datenbank-Capability-, Authentifizierungs- und Berechtigungsprüfung
  ausführen; eigene Beschriftungen und Browser-Bestätigungen sind verfügbar,
  aktionsspezifische Views bleiben offen.
- [ ] Design-Token-System für Farben, Abstände, Typografie, Breakpoints und
  Dichte.
- [ ] View- und globale Themes, Dark Mode und benutzerwählbare Darstellung.
- [ ] Scoped CSS, Asset-Pipeline, Cache-Busting, statische Dateien und CSP.
- [ ] Responsive Layouts, Tastaturbedienung, semantisches HTML, ARIA-Hinweise
  und automatische Accessibility-Prüfungen.
- [ ] Lokalisierung, Pluralisierung, Zeitzonen-/Locale-Formatierung und RTL.
- [ ] Sicherer Raw-HTML-Escape-Hatch mit Diagnostik und Review-Markierung.
- [ ] Progressive Enhancement: zuerst servergerendertes HTML, danach optional
  Client-State und Hydration.
- [ ] WebSocket-/SSE-Unterstützung und typisierte Client-Server-Events.
- [~] Browser-Integrationstests decken jetzt MariaDB-basierte struct-Tableviews
  ab; View-Snapshots und deterministische Rendering-Tests bleiben geplant.
- [?] Optionale weitere Renderer (E-Mail, PDF, Text, Desktop).

## 5. Formulare, CRUD und Businessanwendungen

- [ ] Verschachtelte Formulare, wiederholbare Felder, Uploads, mehrstufige
  Workflows und bedingte Felder.
- [ ] Cross-Field- und datenbankgestützte Validierung mit klaren Transaktionen.
- [ ] Optimistic Locking und konfliktbewusstes Bearbeiten.
- [ ] Bulk-Aktionen, Im-/Export, gespeicherte Suchen, Spaltenpräferenzen und
  serverseitige Reports.
- [~] Eigene Aktionsbeschriftungen und Browser-Bestätigungen sind verfügbar.
- [x] Typisierte Eingaben eigener Aktionen verwenden normale
  Formularvalidierung und Parameterbindung; Beziehungsfelder werden als
  geprüfte, MariaDB-gestützte Auswahlfelder dargestellt.
- [x] Eigene Aktions-Icons und escaped Erfolgsmeldungen sind verfügbar.
- [x] Serverseitige `confirm_page`-Ansichten mit frischer
  CSRF-geschützter POST-Bestätigung sind verfügbar.
- [x] Strukturierte `success_page`-Meldungen und sichere
  aktionsspezifische `error_page`-Antworten sind verfügbar.
- [x] Reversibles CRUD-Soft-Delete mit Archivlisten und CSRF-geschützten
  Wiederherstellungsaktionen.
- [x] Audit-fähige CRUD-Ereignisse für Erstellen/Ändern/Löschen/Archivieren/
  Wiederherstellen und eigene Aktionen mit Feldänderungen und Redaction
  sensibler Werte.
- [ ] Endgültiges Bereinigen, Aufbewahrungsregeln, Archivexport und
  Massenarchivierungs-Workflows.
- [ ] Audit-fähige CRUD-Historie und Feldänderungs-Diffs.
- [ ] Hintergrundjobs, geplante Tasks, Retries und transaktionale Outbox.
- [ ] Benachrichtigungen, E-Mail-Vorlagen, SMTP und Provider-Abstraktion.
- [ ] Grundbausteine für Multi-Tenancy und mandantenbewusste Autorisierung.

## 6. Authentifizierung, Autorisierung und Audit

- [x] Passwort-Login, persistente Sessions, CSRF, Account-Aktivierung und
  Schutz des letzten Administrators.
- [x] Direkte und rollenbasierte Berechtigungen.
- [x] Browserverwaltung und CLI-Rollenverwaltung.
- [x] Audit-Anzeige, begrenzter Export, strukturelle Prüfung und sicheres
  Bereinigen.
- [x] Kryptografisch verkettete Audit-Einträge mit dokumentiertem SHA-256-
  Hashformat, kanonischer Serialisierung und transaktionsgesichertem Append.
- [x] `audit verify` erkennt gebrochene Verbindungen und ungültige Entry-Hashes;
  die genaue Position des ersten Fehlers und unabhängige Diagnosen bleiben offen.
- [ ] Unveränderliche bzw. Append-only-Datenbankrechte für Audit-Tabellen.
- [ ] Konfigurierbare Aufbewahrung, geplantes Bereinigen und Archivexport.
- [ ] Verschlüsselte Archive, Schlüsselrotation, Restore-Prüfung und Offline-
  Integritätsprüfung.
- [ ] Externe Audit-Ziele (Syslog, OpenTelemetry, Object Storage, SIEM) mit
  Zustellstatus und Retries.
- [ ] MFA/WebAuthn, Passwort-Reset-Flows, Geräte-/Sessionverwaltung und
  Login-Benachrichtigungen.
- [ ] Feingranulare Policy-Ausdrücke, Policy-Tests und Erklärungen effektiver
  Berechtigungen.
- [ ] Security Review, Threat Model, Dependency Audit und Penetrationstests.

## 7. APIs und Integration

- [x] Typisierte API-Routen, Request-Validierung, JSON, OpenAPI und TypeScript-
  Clientgenerierung.
- [ ] API-Versionierung, Deprecation-Metadaten, Rate Limits, Quotas und
  Request-Correlation-IDs.
- [ ] API Keys, OAuth2/OIDC und Service Accounts.
- [ ] Webhooks, signierte Callbacks, Idempotency Keys und retry-sichere Handler.
- [ ] GraphQL oder eine andere Query-API nur bei Erhalt der Zelyra-Typ- und
  Capability-Garantien.
- [ ] Weitere SDKs bei nachgewiesenem Bedarf.

## 8. Verifikation, Nebenläufigkeit und Performance

- [ ] Vollständigere Contracts für Collections, Records, Fehler und
  datenbankunabhängige Geschäftsregeln.
- [ ] Cache für Beweisergebnisse und explizite vertrauenswürdige Annahmen.
- [ ] SMT-/SMT-LIB-Integration sowie Diagnose für Solver-Ressourcen und
  Timeouts.
- [ ] Klare Trennung von `PROVEN`, `RUNTIME_CHECK`, `UNPROVEN` und `FAILED` in
  CLI, IDE und Dokumentation.
- [ ] Abbruch, Timeouts, Supervision und DB-Pool-Integration für Structured
  Concurrency.
- [ ] Regeln für Shared State, Channels, Actors und Race-Tests.
- [ ] Benchmark-Suite für Compilezeit, Startup, Routing, SQL, Forms, CRUD und
  Speicherverbrauch.
- [ ] Profiling und Observability-Hooks ohne Änderung der Semantik.
- [?] CP-SAT-, MILP- und SMT-Optimierung mit reproduzierbaren Solverinputs
  und begrenzter Laufzeit.

## 9. Qualität, Betrieb und Governance

- [ ] Vollständige Integrationsmatrix für OS, Datenbanken, Browser und Runtime.
- [ ] Fuzzing für Lexer, Parser, SQL-Binder, Template-Renderer und HTTP-Parser.
- [ ] Security-Regression-Suite und Dependency-/Lizenzprüfung in CI.
- [ ] Reproduzierbare Releases, SBOMs, Provenance-Nachweise und signierte Artefakte.
- [ ] Synchron gehaltene deutsche und englische Dokumentation einschließlich
  Migrations- und Upgrade-Anleitungen.
- [ ] Contributing Guide, Architecture Decision Records, Code of Conduct und
  transparente Issue-Labels.
- [ ] SemVer-Regeln, Deprecation-Fenster und Kompatibilitätstests.
- [ ] Kriterien für öffentliche Alpha, Beta und Stable Releases auf Grundlage
  echter Businessanwendungen statt nur Sprachbeispielen.

## Akzeptanzanwendungen aus der Praxis

- [ ] Maschinenverwaltung mit Abteilungen, Maschinen, CRUD, Suche, Filtern,
  Berechtigungen, Audit und MariaDB-Deployment.
- [ ] Kunden-/Auftragsanwendung mit komplexen Joins, Aggregaten, Formularen,
  API und individuellen Views.
- [ ] Mehrbenutzer-Inventar mit Transaktionen und parallelen Änderungen.
- [ ] Produktionsdeployment mit Docker Compose und konfigurierbarem Web-Port.
- [ ] Self-Hosted-Deployment ohne Rust oder Cargo auf dem Zielsystem.

Diese Datei wird bei jeder Statusänderung eines Meilensteins und bei jeder
neuen verpflichtenden oder optionalen Architekturentscheidung aktualisiert.
