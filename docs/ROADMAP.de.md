# Zelyra Roadmap

Dies ist die dauerhaft gepflegte Roadmap für Zelyra. Sie enthält notwendige
Arbeiten ebenso wie optionale Ideen. Ein Punkt gilt erst dann als umgesetzt,
wenn — soweit anwendbar — Syntax oder API-Dokumentation, Implementierung,
positive und negative Tests, Diagnostik und ein funktionierendes Beispiel
vorhanden sind.

Statuslegende:

- ✅ umgesetzt (`[x]`)
- 🧪 teilweise umgesetzt oder experimentell (`[~]`)
- 🗺️ geplant (`[ ]`)
- ◻️ optional oder noch in Prüfung (`[?]`)
- ⛔ blockiert oder bewusst zurückgestellt (Grund direkt am Punkt nennen)

Die sichtbaren Emojis machen den Status auf GitHub eindeutig; die
Klammermarker bleiben vorerst maschinell durchsuchbar.

## Aktuelle Meilensteine

- [x] Sprachkern mit Lexer, Parser, AST, Funktionen, Ausdrücken, Kontrollfluss,
  unveränderlichen Bindings als Standard, Arrays, deterministischen typisierten
  Maps, Records, Option, Result und Pattern Matching.
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
  Properties sind verfügbar. Benannte Slots mit Fallback-Inhalt und CRUD-
  View-Überschreibungen sind verfügbar; Themes bleiben offen.
- [~] Eingebaute deutsche/englische UI-Kataloge werden über
  `ZELYRA_LANGUAGE` gewählt; `ZELYRA_LEVEL=learn|work` steuert die
  kontextbezogene Lernhilfe. Die minimalen und
  Maschinenverwaltungs-MariaDB-Starter haben einen responsiven
  Zelyra-Anwendungsrahmen; projektlokale Kataloge, umfassendere
  Vorlagenabdeckung und austauschbare Themes bleiben offen.
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
  Formulare, CRUD, Views, APIs, Berechtigungen, Contracts und eine
  strukturierte, deterministische `references`-Kantenliste. Eine fokussierte
  `--symbol <kind:name>`-Abfrage für direkt verbundene Referenzen ist verfügbar;
  E-Mails, Jobs, Tests und Live-Schemaänderungen bleiben anzubinden.
- [~] **Stufe E — semantische Änderungen:** Eine validierte, atomare
  `zelyra edit --format=json`-Umbenennung für deklarierte Funktionen, Typen,
  Records, Tabellen, Tableviews, Formulare, CRUDs, Views und Komponenten ist
  als versionierte Vorschau und mit ausdrücklichem `--apply` verfügbar.
  Projektlokale `.zyl`-Grenzen, Stale-Source-Fingerprints sowie vollständige
  Compilerprüfungen vor und nach der Änderung sind aktiv; umfangreichere,
  namensauflösungsbasierte Operationen bleiben offen. Umbenennungen von
  Funktionen, Typen und Records lösen Deklarationen und bekannte Referenzen
  jetzt über den AST auf, ohne überschattete lokale Bindungen zu verändern;
  Tabellenumbenennungen aktualisieren außerdem geprüfte SQL-Tabellenpositionen
  und bewahren Literale, Kommentare, Parameter und HTML; eine umfassendere
  ressourcenübergreifende Referenzauflösung bleibt offen.
- [~] **Stufe F — Contracts und Effekte:** Contracts und Capability-Prüfung
  existieren; feinere Effekte wie `Database(read)`, `Database(write)`,
  `Email` und `Jobs` sind geplant. KI darf Effekte niemals unbemerkt ergänzen.
- [ ] **Stufe G — Benchmark:** Reproduzierbaren KI-Autorenschaftsbenchmark
  etablieren, bevor vergleichende Eignungsbehauptungen veröffentlicht werden.
  Ergebnisse bleiben bis zu echten kontrollierten Versuchen leer.

## 1. Einstieg und Distribution

- [~] Einfache Standards mit optionalen, dokumentierten Projekt-Feature-
  Schaltern über `zelyra.toml`, `.env` und Prozessüberschreibungen sind
  verfügbar; umfassendere Profile und interaktive Konfiguration bleiben offen.
  Die vollständige Referenz wird in `docs/env.md` und `docs/env.en.md`
  gepflegt.

- [~] Eine Ein-Befehl-Quellcodeinstallation für Linux, Windows und macOS ist
  mit benutzerlokalen, wiederholbaren Bash-/PowerShell-Installern verfügbar;
  eine Rust-freie Release-Installation ist für veröffentlichte Linux-/Windows-
  x86_64-Assets verfügbar.
- [~] Release-Archive mit SHA-256-Checksums sind für Linux und Windows x86_64
  verfügbar; signierte Binaries und Checksums für jede unterstützte Plattform
  bleiben offen.
- [~] Quellcode-Installer mit Prüfung, Update, Deinstallation, Dry-Run und
  sicheren Diagnosen sind verfügbar; der Rust-freie Release-Modus deckt
  veröffentlichte x86_64-Assets ab.
- [~] Ein deterministischer Projektstart unterstützt MariaDB-Scaffolding,
  sichere lokale `.env`-Erzeugung direkt bei `zelyra new` und `zelyra init`,
  ausführlich kommentierte optionale Einstellungen und validierte wählbare
  Ports; bei neuen Projekten und beim Anlegen einer fehlenden `.env` durch
  Setup werden belegte Standardports automatisch durch freie Host-Ports
  ersetzt. Explizite Setup-Portwerte bleiben verbindlich und vorhandene
  `.env`-Dateien geschützt. Eine interaktive Verbindungs-Konfiguration bleibt
  offen.
- [~] Ein gemeinsamer Setup-Assistent für Konsole und Browser ist über
  `zelyra setup --database|--schema|--all` und `zelyra setup --web` verfügbar;
  erzeugte MariaDB-Compose-Projekte werden gestartet und das Anfangsschema
  angewendet; Konsolen- und Browser-Aktionen sind jetzt durch HTTP- und
  Integrationstests auf isolierten Ports abgedeckt und zeigen bei fehlendem
  Compose plattformspezifische Docker-Installationshinweise sowie erkannte
  Compose-Befehle und sichere Hinweise zu Docker-Berechtigungen,
  Gruppenaktualisierung und Zugriffsprüfung sowie Portkonflikten. Docker-
  Installation, Fernadministration und produktives Deployment bleiben bewusst
  außerhalb des Assistenten.
- [~] Die erzeugte Docker-Compose-Vorlage startet MariaDB und den internen
  Webserver mit unabhängig konfigurierbaren Web-, MariaDB- und Container-Ports;
  die Docker-Laufzeitprüfung für erzeugte Projekte ist vorhanden,
  Produktionshärtung bleibt offen.
- [ ] Optionale automatische Reverse-Proxy-Einrichtung für Apache und Nginx
  mit sicheren Defaults und Vorschau der Konfiguration.
- [~] `zelyra doctor` prüft Projektgültigkeit, Datenbankverbindung, Docker
  Compose, eine optionale `.env` ohne Zugangsdaten auszugeben, und die
  Host-Port-Bereitschaft; TLS, Dateirechte und umfassendere Werkzeug-Hinweise
  bleiben offen.
- [~] Projektvorlagen: minimales Skript sowie MariaDB-CRUD-,
  MariaDB-Authentifizierungs- und MariaDB-Business-Starter sind vorhanden;
  API- und Produktionsdeployment-Vorlagen folgen.
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
- [~] Ein erstes Capability-/Effect-Modell für Database, Network, FileSystem,
  Environment, Process, Clock und Random ist vorhanden; feinere Effekte wie
  `Database(read)` und `Database(write)` bleiben geplant.
- [ ] Strukturierte Fehlerweitergabe und eigene Fehlertypen.
- [ ] Deterministischer Build-Graph, inkrementelle und parallele Kompilierung.
- [~] Der deterministische Formatter ist umgesetzt; Language Server,
  Editor-Erweiterungen, Linter und Debugger bleiben geplant.
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
- [x] Datenbank-CLI: `create`, `setup`, `bootstrap`, `inspect`, `plan` und das
  geschützte `apply` sind mit ausdrücklichem Backend-Verhalten und Schutz vor
  destruktiven Änderungen umgesetzt.
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

- [~] Das minimale und das Maschinenverwaltungs-Starterprojekt enthalten
  responsive, gebrandete Rahmen mit kataloggebundenen deutschen/englischen
  UI-Texten; das Maschinenverwaltungs-Starterprojekt ergänzt die Lernhilfe im
  `learn`-Modus. CRUD-, Formular-, Authentifizierungs-, Validierungs- und
  Standard-HTTP-Fehlertexte nutzen dieselben eingebauten Sprachkataloge;
  umfassender Theme-Austausch, weitere Vorlagen und projektlokale
  Katalogerweiterungen bleiben offen.

- [~] Benannte Views/Layout mit `view: Name`, einem validierten Default-
  `<slot />`-Inhaltsslot und validierten benannten Slots mit Fallback-Inhalten.
- [~] Typisierte View-Ausdrücke prüfen Identifier- und
  Record-Feldinterpolationen, Seiten-Routenbindungen, Component-Properties
  und dynamische Property-Typen. Option-aware Feld-Ausdrücke und reichere
  View-Daten bleiben offen.
- [~] Benannte Komponenten mit typisierten Properties sind verfügbar; typisierte
  Events bleiben geplant.
- [~] Deklarative MariaDB-`tableview`-Routen mit geprüften SQL-Quellen,
  deklarierten Spalten, typisierten Filtern, Suche, Sortierung, Pagination,
  URL-Zustand und Escaping sind für tabellen- und struct-basierte Ergebnistypen
  verfügbar.
- [~] Komponenten und benannte Views unterstützen Default- und benannte Slots,
  sichere Fallback-Inhalte sowie verschachtelte Komposition. View-Layouts
  prüfen deklarierte Slotnamen und ersetzen sie deterministisch ohne globalen
  Zustand; View-Vererbung und reichere verschachtelte Szenarien bleiben offen.
- [~] CRUD-Ressourcen können mit `layout: ViewName` einen geprüften benannten
  View wiederverwenden. Der Default-Slot erhält erzeugte Listen, Details und
  CRUD-Formulare, ohne SQL-, Validierungs-, CSRF-, Autorisierungs- oder
  Escaping-Prüfungen zu umgehen; benannte Slot-Anpassungen für erzeugte CRUD-
  Inhalte bleiben offen.
- [~] View-lokales Laden unterstützt explizite, schema-geprüfte Abfragen für
  einzelne Datensätze und Record-Collections mit `load name = sql<Type> { ... }`.
  Array-Ergebnisse können mit typisierten
  `for item in collection { ... }`-Blöcken gerendert werden.
  Routenautorisierung, `Database`-Capability, Parameterbindung, generische
  Fehlergrenzen und HTML-Escaping werden erzwungen; Option-aware Feld-Ausdrücke
  und reichere View-Komposition bleiben geplant.
- [~] Typisierte CRUD-Filteroperatoren (`eq`, Textsuche, Zahlenvergleiche und
  NULL-Prüfungen) werden in sichere serverseitige SQL-Abfragen kompiliert.
- [~] Erzeugte CRUD-Filtersteuerungen bewahren Operator- und Wertzustand in
  URLs; deterministische Filterreihenfolge, semantische Fieldsets sowie
  getrennte Operator-/Wertbeschriftungen sind verfügbar, weitergehende
  Barrierefreiheitsverbesserungen bleiben offen.
- [~] Die einheitliche typisierte View-Pipeline umfasst deklarative
  `tableview`-Steuerungen, explizites seitenlokales Laden einzelner Datensätze
  und typisierte Collection-Schleifen; Filter, Sortierung, Suche und Pagination
  sind für deklarierte Seiten-Collections verfügbar, reichere Daten für
  beliebige Views bleiben geplant.
- [~] Seitenlokale typisierte Query-Eingaben (`input { search: String? }`) werden
  geprüft, sicher an natives SQL gebunden, für HTML-Interpolationen verfügbar
  gemacht und bei fehlenden Pflichtwerten oder ungültigen skalaren Werten mit
  kontrolliertem HTTP 400 abgelehnt. Automatisch erzeugte Steuerungen decken
  deklarierte Seiten-Collections ab; reine input-Seiten bleiben bewusst manuell.
- [~] Seitenlokale Collection-Pagination über `paginated <size>` ist verfügbar.
  Ein positiver URL-Wert `page` wird geprüft, als `UInt` bereitgestellt und als
  parametrisierter `LIMIT`-/`OFFSET`-Wrapper angewendet; erzeugte Steuerungen
  und sichere Gesamt-/Seitenzahlen sind für deklarierte Collections verfügbar.
- [~] Seitenlokale Collection-Sortierung über `sort { field ... }` ist verfügbar.
  Nur vom Compiler geprüfte Ergebnisfelder sowie `asc`/`desc` werden akzeptiert;
  erzeugte Sortiersteuerungen bewahren den URL-Zustand.
- [~] Seitenlokale Collection-Suche über `search { field ... }` ist verfügbar.
  Suchbegriffe werden parametrisiert und mit serverseitigen `LIKE`-Bedingungen
  auf compiler-geprüfte Felder angewendet; erzeugte Suchsteuerungen bewahren
  den URL-Zustand.
- [~] Seitenlokale typisierte Filter über `filter { field ... }` sind verfügbar.
  Operatoren werden aus den deklarierten Ergebnistypen abgeleitet, Werte als
  Parameter gebunden und unbekannte Felder oder nicht unterstützte Operatoren
  abgelehnt; erzeugte Filtersteuerungen bewahren den URL-Zustand.
- [ ] Zusammensetzbare Filterausdrücke mit typisierten Operatoren für
  Datumswerte, Booleans, Beziehungen und Volltextsuche.
- [ ] Wiederverwendbare Navigation, Tabellen, Formulare, Dialoge, Hinweise,
  Pagination und Validierungsfehler-Komponenten.
- [~] CRUD-View-Überschreibungen für Listen unterstützen sicher die Modi
  `table`/`cards` und eine eigene Leerzustandsmeldung bei Erhalt der
  generierten Abfrage-, Auth- und Aktionsprüfungen.
- [~] Gemeinsame schema-basierte CRUD-View-Felder können erzeugte Liste,
  Detailansicht und Create-/Edit-Formulare steuern; explizite Listenauswahl
  bleibt eine lokale Überschreibung.
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
- [~] `audit verify` erkennt gebrochene Verbindungen und ungültige Entry-Hashes;
  genaue Positionen des ersten Fehlers und unabhängige Diagnosen bleiben offen.
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
- [x] Typisierte Maps mit String-Schlüsseln werden an der API-Grenze geprüft
  und konsistent in JSON, OpenAPI und erzeugten TypeScript-Clients dargestellt.
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
- [~] Klare Trennung von `PROVEN`, `RUNTIME_CHECK`, `UNPROVEN` und `FAILED` ist
  in CLI und Dokumentation vorhanden; IDE-Integration bleibt geplant.
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

- [~] Die Maschinenverwaltung ist als MariaDB-Template vorhanden und durch
  Integrationstests für erzeugte Projekte abgedeckt; Produktionshärtung bleibt
  offen.
- [ ] Kunden-/Auftragsanwendung mit komplexen Joins, Aggregaten, Formularen,
  API und individuellen Views.
- [ ] Mehrbenutzer-Inventar mit Transaktionen und parallelen Änderungen.
- [~] Ein erzeugtes Docker-Compose-Deployment mit unabhängig konfigurierbarem
  Web-Port ist getestet; Produktionshärtung bleibt offen.
- [~] Rust-freie Self-Hosted-Installation ist für veröffentlichte Linux- und
  Windows-x86_64-Assets verfügbar; weitere Plattformen bleiben offen.

Diese Datei wird bei jeder Statusänderung eines Meilensteins und bei jeder
neuen verpflichtenden oder optionalen Architekturentscheidung aktualisiert.
