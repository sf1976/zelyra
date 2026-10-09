Warning: truncated output (original token count: 14728)
Total output lines: 905

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

Digitale Souveränität ist eine querschnittliche Produktanforderung, kein
bereits vollständig umgesetztes Feature: lokale Kontrolle, keine verpflichtende
Cloud oder KI, keine ungefragte Telemetrie, explizite Effekte, portable Daten,
Ressourcendisziplin und ehrliche Beweisaussagen leiten jede Phase. Das
[Manifest](MANIFESTO.de.md) beschreibt die Prinzipien; die Roadmap kennzeichnet
den tatsächlich implementierten Stand.

## Aktuelle Meilensteine

- [~] Der 0.4-Entwicklungszweig ergänzt Anzeige und Sperrung dauerhafter
  Sitzungen mit Verwaltungsrecht, CSRF-Prüfung und Audit-Anfrageereignis.
  Unter `/account/sessions` können angemeldete Nutzer außerdem eigene
  dauerhafte und flüchtige Sitzungen mit CSRF- und Same-Origin-Prüfung anzeigen
  und beenden. Speichersitzungen laufen nach 24 Stunden ab. Gerätemetadaten
  bleiben offen. Passwortwiederherstellung ist im 0.4-Entwicklungszweig
  teilweise umgesetzt:
  MariaDB speichert nur Einweg-Token-Hashes, generische Antworten, Audit,
  Loopback-/HTTPS-SMTP-Konfiguration, begrenzte asynchrone Zustellung und
  Sitzungswiderruf sind vorhanden und werden durch ein MariaDB-/SMTP-Senken-E2E
  mit verzögerter Zustellung geprüft. Der E2E-Test sendet außerdem zwei
  gleichzeitige Einlösungen desselben Tokens und prüft genau einen Erfolg,
  eine Ablehnung und die Authentifizierung mit nur dem siegreichen Passwort.
  Token-Ersetzung und FIFO-Mailqueue sind innerhalb eines Prozesses
  serialisiert. MariaDB-Advisory-Locks koordinieren Ausstellung und Zustellung
  auch über mehrere Instanzen; ein Zwei-Instanzen-E2E verzögert das erste
  SMTP-Relay, während die zweite Instanz das Token ersetzt, und prüft, dass
  die spätere E-Mail zum gespeicherten Token gehört. Veraltete Tokens in der
  Mailqueue werden übersprungen. Das MariaDB-E2E lehnt ein Token mit
  `expires_at = NOW()` ab und friert die Datenbankzeit ein, um das produktive
  Token-Prädikat vor, exakt bei und nach Ablauf zu prüfen. Gleichheit wird
  abgelehnt. Prozesslokale Reset-Limits, dauerhafte SMTP-Zustellwiederherstellung
  und unabhängige Sicherheitsprüfung bleiben offen; der Kontolebenszyklus ist
  nicht abgeschlossen.
  Bestehende Sitzungstabellen ohne `id`-Spalte behalten ihr bisheriges Verhalten;
  Anzeige und Sperrung im Verwaltungsbereich setzen diese Spalte weiterhin voraus.

- [~] Die [Docker-Modulabnahme](module-docker-acceptance.de.md) prüft die
  Gesamtanwendung, getrennte Rechnungs-/Inventarexporte, fehlende `.env`,
  ausgeschlossene Routen und verweigerte MariaDB-Zugriffe. Die Testkonten
  erlauben nur Lesen; vollständige schreibende CRUD-Abnahme bleibt offen.

- [~] Der 0.4-Release-Workflow erstellt deterministische SPDX-2.3-SBOMs für
  Linux und Windows und prüft den Binary-Hash. CI führt `cargo-deny` mit fester
  Version für Linux- und Windows-Abhängigkeitsgraphen aus. Die Policy prüft
  Advisories, Lizenzen, Quellen, Wildcard-Anforderungen und doppelte Versionen;
  doppelte Versionen von `base64` und `getrandom` warnen, und für
  `rustls-pemfile` besteht eine dokumentierte Ausnahme zum Unmaintained-Advisory.
  Menschlicher Dependency-/Lizenzaudit, eine Attestierungsprobe mit einem
  veröffentlichten Kandidaten und die Prüfung der nativen Artefakte bleiben
  offen. Getaggte Builds attestieren beide Plattformarchive samt SBOM; siehe
  die [Anleitung zur Artefaktprüfung](release-readiness/artefaktverifikation.de.md).

- [~] Der HTTP-Server verarbeitet bis zu 64 Verbindungen parallel und verwirft
  weitere Verbindungen, statt eine unbegrenzte Worker-Warteschlange anzulegen.
  Lesen und Schreiben je Verbindung sind auf 30 Sekunden begrenzt; verspätete
  Antworten entfallen. Synchrone Handler lassen sich nicht abbrechen und können
  weiterhin einen Worker belegen. Die
  [HTTP-Betriebsanleitung](http-operations.de.md) definiert TCP-Listener-
  Erreichbarkeit, die eingebaute Prozess-Liveness-Route, anwendungseigene
  Readiness und begrenzte Retries für sichere oder idempotente Anfragen.
  Abhängigkeits-Readiness und ihre Abnahme in der erzeugten Anwendung bleiben
  offen; der Server wiederholt nicht automatisch.

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
  Properties sind verfügbar. Benannte Slots mit Fallback-Inhalt, CRUD-View-
  Überschreibungen und ressourcenspezifische Inhalte für benannte CRUD-Layout-
  Slots sind verfügbar. Statische Slot-Markups dürfen geprüfte Komponenten
  verwenden; datensatzgebundene Slot-Inhalte bleiben offen. Projektlokale
  CSS-Token-Overrides sind implementiert; vollständige Theme-Erstellung und
  -Auswahl bleiben offen.
- [~] Eingebaute deutsche/englische UI-Kataloge werden über
  `ZELYRA_LANGUAGE` gewählt; `ZELYRA_LEVEL=learn|work` steuert die
  kontextbezogene Lernhilfe. Die minimalen und
  Maschinenverwaltungs-MariaDB-Starter sowie erzeugte CRUD-, Formular-,
  Tableview-, Login- und Authentifizierungsverwaltungsseiten haben
  standardmäßig einen responsiven Zelyra-Anwendungsrahmen. Explizite CRUD-
  Layouts haben Vorrang; selbst verfasste Seiten bleiben unverändert.
  Projektlokale CSS-Overrides für dokumentierte Design-Tokens sind jetzt
  verfügbar. Projektlokale `locales/de.json`- und `locales/en.json`-Overlays
  können alle kataloggebundenen generierten Texte für Rahmen, CRUD, Formulare,
  Tableviews, Login, Auth-Verwaltung, Validierung und Lernhilfe ergänzen oder
  überschreiben, zusätzlich zu markierten View-/Texteinträgen. Parametrisierte
  Beschriftungen und generierte Feldnamen werden unterstützt; HTML-Escaping
  und englischer Fallback bleiben aktiv. Fachdatensätze und unmarkierte eigene
  Texte werden nicht übersetzt. Umfassendere Vorlagenabdeckung und ein
  vollständiger Theme-Editor bleiben offen.
- [x] Authentifizierung, persistente Sessions, Argon2-Passwörter, direkte und
  rollenbasierte Berechtigungen, Browserverwaltung und MariaDB-Audit.
  Schreibende Browseranfragen benötigen zusätzlich gleichursprüngliche
  Nachweise; das 0.2.0-Release ergänzt eine Loopback-Host-Allowlist gegen
  manipulierte Hosts und DNS-Rebinding.
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
  `context --format=json` und Format-Tests sind implementiert. Projektdateien
  verwenden projektrelative portable Pfade; weitere Befehle und eine
  vollständige Secret-Redaction bleiben offen. Im unveröffentlichten
  0.4-Entwicklungszweig enthält der deterministische Modulkontext außerdem
  Importkanten und die derzeit unterstützten öffentlichen Funktionen, Typen
  und Records je Quelldatei (`modules[].exports`). `zelyra module plan` zeigt
  zusätzlich schreibgeschützt den Abschluss expliziter Importe und statisch
  erkannter Verweise aus dem Wirkungsgraphen – wahlweise ab einer Quelldatei
  oder einer unterstützten Seiten-/API-/CRUD-/Formular-/Tableview-Ressource.
  `modules[].declarations` macht alle erkannten Deklarationen der einbezogenen
  Quelldateien sichtbar. `declaration_closure` trennt davon den ausgewählten
  Startpunkt und statisch erreichbare Deklarationen von zusätzlichen
  Deklarationen in den einbezogenen Dateien; `configuration_edges` weist die
  Datenbankkonfiguration separat aus. `database.configurations` nennt Backend,
  logischen Datenbanknamen, Quelldatei und die bevorzugte Laufzeitvariable
  `ZELYRA_DATABASE_<NAME>_URL` (`DATABASE_URL` bleibt Fallback). Die Laufzeit
  hat weiterhin genau eine projektweite Verbindung; die Namenszuordnung ist
  noch keine benannte, wiederverwendbare
  Datenbankschnittstelle. Die Analyse bleibt unvollständig.
  Enthalten sind bekannte Kanten
  von Seiten zu Views/Komponenten, Page-SQL und Formular-/CRUD-Aktions-SQL zu
  Tabellen, API-Handlern zu Funktionen, geschützten Ressourcen zur
  Authentifizierung, Authentifizierung zu Tabellen sowie Typreferenzen aus
  APIs, Funktionssignaturen und Funktionskörpern (einschließlich expliziter
  lokaler Typen, Record-Literalen und SQL-Ergebnistypen), Records und Type
  Aliases (einschließlich Verweisen aus Tabellenspalten und typisierten
  Ressourcenfeldern), Tabellenrelationen und
  Datenbankkonfiguration; nicht auflösbare Verweise werden ausgegeben.
  SQL-Tabellenkanten zeigen außerdem den konservativ erkannten Modus
  `read`, `write`, `read_write` oder `unknown`; komplexe verbundene
  `UPDATE`-/`DELETE`-Formen bleiben `unknown`. `schema_ownership` meldet den
  Tabellenbesitzer nur als aus der Quelldatei abgeleitete Vermutung und setzt
  `enforced: false`. Diese Analyse ist Beobachtung, keine Berechtigungs- oder
  Eigentumsdurchsetzung; nicht erkannte SQL-Formen können fehlen.
  Dynamische oder nicht modellierte Abhängigkeiten, Assets,
  Laufzeitkonfiguration, externe Dienstverträge und Docker-Paketierung bleiben
  außerhalb dieser Vorschau. `complete_deployment` bleibt `false`; der Plan ist
  weder ein vollständiges Exportmanifest noch Teil des veröffentlichten 0.3.0.
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
- [~] Release-Archive mit SHA-256-Prüfsummen sind für Linux und Windows x86_64
  verfügbar. Das öffentliche [0.2.0-Release][release-020] enthält festgelegte
  Linux-/Windows-Builds, byte-identische Wiederholungs-Builds, normalisierte
  Archive und SHA-256-Prüfsummen. Signierte Binärdateien und Prüfsummen für
  alle unterstützten Plattformen fehlen noch.

[release-020]: https://github.com/sf1976/zelyra/releases/tag/v0.2.0
- [~] Benutzerlokale Installations-, Prüf-, Update- und Deinstallationswege mit
  Dry-Run und sicheren Diagnosen sind verfügbar. Zusätzlich prüft
  `zelyra update [--check]` stabile GitHub-Releases und verifiziert eine
  SHA-256-Checksumme vor dem Austausch der eigenen Linux-/Windows-x86_64-
  Binärdatei; unter Windows wird der Austausch bis zum Ende des laufenden
  Prozesses vorgemerkt. Der Release-Workflow wird solche eigenständigen
  Update-Dateien mit künftigen Releases veröffentlichen; automatische Updates
  für andere Ziele bleiben nicht verfügbar.
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
  Webserver mit unabhängig konfigurierbaren Web-, MariaDB- und Container-Ports.
  `tests/generated-project-docker-e2e.sh` erzeugt jetzt ein frisches CRUD-Projekt
  und führt `zelyra setup --all` zweimal aus. Geprüft werden
  schemabasierte Maschinen-/Abteilungsseiten, die ausgegebene lokale Adresse,
  geschützte `.env`-Rechte, geheimnisfreie Ausgabe, unveränderte Zugangsdaten
  beim erneuten Setup, Portzuordnungen und die Bereinigung ausschließlich des
  eindeutig benannten Compose-Projekts samt Volume. Abdeckung über diesen
  isolierten Docker-Ablauf hinaus und Produktionshärtung bleiben offen.
- [ ] Optionale automatische Reverse-Proxy-Einrichtung für Apache und Nginx
  mit sicheren Defaults und Vorschau der Konfiguration.
- [~] `zelyra doctor` prüft Projektgültigkeit, Datenbankverbindung, Docker
  Compose, eine optionale `.env` ohne Zugangsdaten auszugeben, und die
  Host-Port-Bereitschaft. JSON-Checks enthalten nun stabile Kategorien für
  Konfiguration, Projekt, Verbindung, Authentifizierung, Timeout, Schema und Werkzeuge;
  Datenbankfehler werden geheimnisfrei normalisiert. TLS, Dateirechte und
  umfassendere Werkzeug-Hinweise bleiben offen.
- [~] Projektvorlagen: minimales Skript sowie MariaDB-CRUD-,
  MariaDB-Authentifizierungs- und MariaDB-Business-Starter sind vorhanden;
  API- und Produktionsdeployment-Vorlagen folgen.
- [ ] Offline-Installationspaket und reproduzierbare Toolchain-Metadaten.
- [?] Paketmanager-Distribution, soweit sinnvoll (Homebrew, winget,
  Debian-Pakete und Container-Images).

## 2. Sprache und Compiler

- [ ] Stabile Grammatik-Spezifikation und versionierte Kompatibilitätsregeln.
- [~] Experimentelle Funktions-/Typ-/Record-/Tabellen-/View-/Komponenten-Imports und projektweite
  Datenbankkonfiguration im aktuellen
  Entwicklungszweig unterstützen projektrelative Imports, `pub`-Deklarationen,
  qualifizierte Aufrufe und Typreferenzen, Zyklenerkennung,
  Projektstamm-/Symlink-Schutz, dateibezogene Quell-IDs sowie
  Typ-, Capability- und Contract-Prüfungen über importierte Aufrufe hinweg.
  `check`, `build`, `run`, `serve`, `context` und `verify` prüfen diesen Graphen; der
  maschinenlesbare Kontext enthält nun einen deterministischen, sortierten
  Modul-/Import-Überblick. Importierte Datenbankdefinitionen und Tabellen
  fließen in das gemeinsame Schema ein; Views und Komponenten werden in die
  Anwendung integriert und durch `serve` gerendert. Ihre Kontextspannen
  enthalten den projektrelativen Dateipfad. Importierte Seiten werden in die
  Anwendungsrouten integriert; überlappende Seitenpfade werden abgelehnt.
  Bei Template-Diagnosen und Verifikationsergebnissen fehlt noch eine
  vollständige Quellzuordnung pro Modul. Datenbankkonfiguration wird nicht über den Alias angesprochen und ist
  pro Projekt auf eine Verbindung begrenzt. Importierte Tabellen behalten
  globale SQL-Namen; importierte `tableview`-, View- und Komponentennamen sind global, Kollisionen
  werden abgelehnt. Formulare, CRUD-Deklarationen, API-Routen und
  Authentifizierungskonfiguration werden ebenfalls aus importierten Dateien
  zusammengesetzt. API-Handler und Typen werden im jeweiligen Modulkontext
  aufgelöst; Authentifizierungstabellen werden gegen das gemeinsame Schema
  geprüft. MariaDB-gestützte `tableview`s werden nun
  zusammengesetzt und bereitgestellt.
  Datenbankbefehle laden den verknüpften Graphen für das gemeinsame Schema;
  Schemafehler behalten Quelldatei und Span importierter Module.
  Template- und Verifi…4728 tokens truncated….
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
- [ ] Ausdrückliche, anwenderkontrollierte Audit-Exporte an Ziele wie Syslog,
  Object Storage oder SIEM mit Zustellstatus und Retries; Nutzungs-Telemetrie
  und versteckte externe Erfassung sind ausgeschlossen.
- [🧪] Passwort-Reset und Sitzungsverwaltung sind im unveröffentlichten 0.4
  teilweise umgesetzt. Gleichzeitige Einlösungen desselben Tokens sind im
  MariaDB-E2E abgedeckt. Token-Ausstellung und E-Mail-Queue sind pro Prozess
  serialisiert; Mehrprozessbetrieb bleibt offen. Persistente
  Reset-Limits, Gerätemetadaten, MFA/WebAuthn und
  Login-Benachrichtigungen bleiben offen.
- [ ] Feingranulare Policy-Ausdrücke, Policy-Tests und Erklärungen effektiver
  Berechtigungen.
- [ ] Security Review, Threat Model, Dependency Audit und Penetrationstests.

## 7. APIs und Integration

- [x] Typisierte API-Routen, Request-Validierung, JSON, OpenAPI und TypeScript-
  Clientgenerierung.
- [x] Typisierte Maps mit String-Schlüsseln werden an der API-Grenze geprüft
  und konsistent in JSON, OpenAPI und erzeugten TypeScript-Clients dargestellt.
- [🧪] API-Request-IDs sowie Versions-, Deprecation- und prozesslokale
  Kontingentmetadaten je Route sind im unveröffentlichten 0.4-Entwicklungsstand
  umgesetzt. Kontingente nutzen die TCP-Peer-IP, eine begrenzte
  In-Memory-Clienttabelle und keine vertrauten Forwarded-IP-Header; sie werden
  beim Neustart zurückgesetzt. Persistente/verteilte Kontingente, Reset-Limits
  und Kompatibilitätsnachweise bleiben offen. Login-Limits sind pro
  Auth-Definition konfigurierbar; ihre In-Memory-Tabelle ist auf 4096 Schlüssel
  begrenzt und lehnt neue Schlüssel bei voller Tabelle ab.
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
- [ ] Lokales, ausdrücklich aktiviertes Profiling und Diagnostik ohne
  versteckte Erfassung oder automatische externe Telemetrie.
- [?] CP-SAT-, MILP- und SMT-Optimierung mit reproduzierbaren Solverinputs
  und begrenzter Laufzeit.

## 9. Qualität, Betrieb und Governance

- [ ] Vollständige Integrationsmatrix für OS, Datenbanken, Browser und Runtime.
- [~] Begrenzte deterministische Mutationsregressionen prüfen Lexer/Parser,
  SQL-Binder, Template-Renderer und HTTP-Parser im normalen CI-Lauf (2.048
  Eingaben je Pfad). Coverage-guided-Fuzzing fand einen Parser-Stack-Overflow;
  rekursive Parser-Aufrufe haben nun eine 32-Frame-Grenze mit Diagnose,
  gezielte Regressionen und einen aufbewahrten Crash-Seed. Vier lokale 60-Sekunden-Läufe bestanden
  danach; der 15-Sekunden-CI-Lauf auf dem korrigierten Stand steht noch aus.
- [~] Security-Regressionstests und Dependency-/Lizenzprüfung laufen teilweise
  in CI; ein vollständiger menschlicher Audit und weitere Security-Regressionen
  bleiben offen.
- [ ] Regressionstest gegen ungefragte Netzwerk- oder Telemetrieaktivität;
  explizite Netzwerk-Capabilities der Anwendung und benutzerinitiierte
  Update- oder Installationsbefehle müssen klar getrennt bleiben.
- [ ] Reproduzierbare Releases, SBOMs, Provenance-Nachweise und signierte Artefakte.
- [x] Release-Artefakte für Linux und Windows können bei relevanten Pull
  Requests und manuell ohne Veröffentlichung gebaut werden; veröffentlicht
  wird ausschließlich bei geprüften Versions-Tag-Pushes. Siehe die
  zweisprachige [Release-Anleitung](releasing.de.md).
- [ ] Synchron gehaltene deutsche und englische Dokumentation einschließlich
  Migrations- und Upgrade-Anleitungen.
- [ ] Contributing Guide, Architecture Decision Records, Code of Conduct und
  transparente Issue-Labels.
- [ ] SemVer-Regeln, Deprecation-Fenster und Kompatibilitätstests.
- [ ] Kriterien für öffentliche Alpha, Beta und Stable Releases auf Grundlage
  echter Businessanwendungen statt nur Sprachbeispielen.

## Release-Meilenstein 0.2.0

Zelyra 0.2.0 wurde am 2026-09-20 veröffentlicht. Dieser Abschnitt hält den
gelieferten Umfang und die Release-Nachweise fest; das Release bleibt
experimentell und ist nicht für den Produktiveinsatz freigegeben. Die
bestandenen Release-Kriterien bedeuten nicht, dass alle folgenden Roadmap-
Arbeiten abgeschlossen sind.

- [~] **Eigenständiges Views-System:** Wiederverwendbare Layouts/Komponenten,
  typisierte CRUD-Darstellungsoptionen und ressourcenspezifische benannte
  Layout-Slots sind verfügbar. Projektlokale deutsche/englische Kataloge
  können alle kataloggebundenen generierten Standardtexte sowie ausdrücklich
  markierte View-/Texteinträge ergänzen oder überschreiben. Parametrisierte
  Feld- und generierte Identifier-Beschriftungen werden unterstützt;
  Fachdatensätze und unmarkierte eigene Texte bleiben unverändert.
  Weitergehende Theme-Erstellung und datensatzgebundene benutzerdefinierte
  Slots bleiben offen.
- [~] **Vorzeige-Businessanwendung:** Das MariaDB-Business-Template und der
  Integrationstestpfad für erzeugte Projekte existieren. Ein datenbankfreier
  CLI-/HTTP-Test deckt jetzt das generierte CRUD-Formular, Ablehnungen ohne
  Token und ohne ausreichende API-Berechtigung, autorisierten Zugriff sowie
  projektlokale deutsche/englische Texte ab. Zusätzlich wurde der
  MariaDB-gestützte Pfad aus `tests/generated-project-business-e2e.sh` mit einer
  isolierten Testdatenbank erfolgreich ausgeführt und die Bereinigung geprüft.
  Der MariaDB-Akzeptanztest für die Maschinenverwaltung prüft jetzt die
  lokalisierten Maschinen- und Abteilungsansichten in allen vier Kombinationen
  aus Deutsch/Englisch und Learn/Work – einschließlich Anzeige, Ausblendung und
  Übersetzung der Lernhilfe. Einsteigergeprüfter Einstieg und eine umfassendere
  dokumentierte Nutzerabnahme bleiben Release-Arbeiten. Eine geschützte
  Maschinenverwaltungs-Fixture und ein Generated-Project-E2E prüfen jetzt
  zusätzlich anonyme Ablehnung, Viewer-Zugriff, Ablehnung von
  Erstellen/Bearbeiten, autorisierte CRUD-Formulare, Suche und Löschen.
- [~] **Einfacher Erststart:** zelyra new und zelyra setup, geschützte
  `.env`, Docker Compose, freie Portwahl, konkrete Docker-Berechtigungshinweise
  und die Ausgabe der App-Adresse sind vorhanden. Der erzeugte CRUD-Stack hat
  einen isolierten Erststart- und Wiederholungstest bestanden, einschließlich
  schemaabhängiger HTTP-Seiten, unveränderter Zugangsdaten, geheimnisfreier
  Setup-Ausgabe, Portzuordnungen und Bereinigung. Prüfungen auf sauberen Hosts
  und Wiederherstellungsfälle über diesen Docker-Ablauf hinaus bleiben offen.
- [~] **Datenbanksicherheit:** MariaDB ist die Runtime-Referenz; für SQLite
  gibt es End-to-End-Pfade. Der Schema-Sicherheitstest prüft auf beiden
  Backends: destruktive Löschungen benötigen Freigabe; Pflichtspalten ohne
  Standardwert, neue Unique-Constraints sowie MariaDB-Foreign-Key-Ergänzungen
  und -Entfernungen erscheinen als `REVIEW`. Eine Pflichtspalte ohne
  Standardwert kann einer befüllten Tabelle erst nach einem lesenden Preflight
  hinzugefügt werden; bei vorhandenen Zeilen wird der gesamte Plan vor
  jeglichem SQL abgelehnt. Doppelte beziehungsweise
  verwaiste Zeilen bleiben erhalten, wenn Index- oder Foreign-Key-Änderungen
  scheitern. Unbekannte externe Indizes bleiben bestehen, nicht verfolgte
  Foreign-Key-Entfernungen werden fail-closed blockiert. Default-Drift bei
  MariaDB und PostgreSQL wird als `REVIEW` geführt; SQLite-Defaultänderungen
  sowie Primärschlüssel-/Auto-Increment-Änderungen ohne unterstützte Migration
  werden mit `E-DB-006` abgelehnt. `--allow-destructive` genehmigt
  keine `REVIEW`-Änderungen; diese benötigen nach Prüfung `--allow-risky`.
  Nullbarkeitsänderungen bei MariaDB und PostgreSQL benötigen Freigabe; vor
  einer Verschärfung prüft Zelyra vorhandene NULL-Werte, bevor irgendein SQL
  des Plans läuft. SQLite-Nullbarkeit sowie SQLite-Typ-, Foreign-Key- und
  Unique-Constraint-Änderungen werden noch nicht unterstützt. Typ- und
  Nullbarkeitsänderungen werden unabhängig bewertet. Die
  [Versionsmatrix](database-compatibility.de.md) führt vier MariaDB-
  Community-LTS-Patch-Images und die getesteten Datenbank-/CRUD-Pfade auf;
  sie ist keine MySQL-Kompatibilitäts- oder vollständige Funktionsgarantie.
  PostgreSQL-Runtime-Parität, allgemeine Daten-Backfills, Zeilen-/Lock-
  Risikoeinschätzungen und betriebliche Risikoanalyse bleiben außerhalb der
  0.2.0-Aussage. PostgreSQL-Drift bei
  Primärschlüsseln und Serial-/Identity-Eigenschaften wird erkannt und
  abgelehnt; Migrationen dieser Metadatenänderungen werden noch nicht
  unterstützt.
- [x] **Release-Nachweise:** Zweisprachige Quickstarts, internes
  Sicherheitsreview, vollständige Branch-CI, Workspace-Prüfungen, die
  MariaDB-Kompatibilitätsmatrix mit vier Versionen und datenbankgestützte
  Integrationstests sind erfolgreich. Eine frisch erzeugte MariaDB-CRUD-
  Anwendung bestand Erststart und wiederholtes Setup im isolierten Docker-E2E-
  Test. Linux- und Windows-Release-Binärdateien wurden in den festgelegten
  CI-Toolchains byte-identisch erneut gebaut; beide Release-Pakete bestanden
  die Prüfung. Das ist technische Abnahme für ein experimentelles Release,
  keine Einsteigerstudie, Windows-Ersteinstiegsstudie auf einem sauberen Host,
  Produktionsfreigabe oder externes Sicherheitsaudit; solche Aussagen werden
  nicht gemacht.

Für 0.2.0 sind weder ein visueller Drag-and-drop-Editor noch eine Anbindung an
einen KI-Anbieter, Compiler-Self-Hosting, vollständige formale Verifikation
oder PostgreSQL-Runtime-Parität erforderlich. Diese Ziele bleiben separat in
der Roadmap; geplante Fähigkeiten dürfen nicht Teil des Releaseversprechens
werden.

## Release-Meilenstein 0.3.0 (veröffentlicht)

Zelyra [`v0.3.0`](https://github.com/sf1976/zelyra/releases/tag/v0.3.0) wurde
am 03.10.2026 als experimentelles Release veröffentlicht. Der abgeschlossene
[Releaseplan](release-plans/0.3.0.de.md) dokumentiert Umfang, Prüfnachweise und
bekannte Grenzen. Der Release-Workflow baute und prüfte Linux- und
Windows-x86_64-Artefakte; der veröffentlichte Linux-Installer einschließlich
Prüfsumme, Wiederholungsinstallation und Update-Check wurde anschließend
erfolgreich smoke-getestet.

Der unabhängige menschliche Einsteigertest fand für 0.3.0 nicht statt. Der
Projektverantwortliche hat ihn ausdrücklich auf 0.4.0 verschoben. Das ist eine
Risikoentscheidung, kein Nutzertestnachweis; die menschliche Abnahme ist vor
dem finalen 0.4.0-Release verpflichtend. Siehe das
[Entscheidungsprotokoll](release-readiness/0.3.0-human-gate-decision.de.md).

Der aktuelle Datenbank-Abnahmenachweis ist im [CI-Lauf
35571692858](https://github.com/sf1976/zelyra/actions/runs/35571692858) grün:
Die MariaDB-Matrix mit vier Versionen, Schema-Sicherheitsprüfungen, der
SQLite-Regressionstest, erzeugte Anwendungen und der isolierte Kunden-/Auftrags-
Workflow waren erfolgreich.
Der Clean-Onboardingpfad mit installierter CLI ist außerdem im [CI-Lauf
35573616419](https://github.com/sf1976/zelyra/actions/runs/35573616419) grün.

## Vorgeschlagener Release-Meilenstein 0.4.0

Der separate [Releaseplan 0.4.0](release-plans/0.4.0.de.md) ist ein
Arbeitsvorschlag für den nächsten Meilenstein nach dem finalen 0.3.0-Release.
Er priorisiert Module und deterministische Mehrdateiprojekte, einen
zuverlässigen Datenbank-/Runtime-Lebenszyklus, sicherere Konto- und
API-Lebenszyklen, granulare AI-native Effekte sowie Nachweise für Lieferkette
und Barrierefreiheit. PostgreSQL-Runtime-Parität, Multi-Tenancy,
Hintergrundjobs, MFA, OIDC oder ein visueller Editor werden dadurch bewusst
nicht automatisch zu 0.4.0-Versprechen.

Das stabile `v0.3.0` wurde am 03.10.2026 veröffentlicht. Der Projektverantwortliche
hat den menschlichen Einsteigertest für 0.3.0 ausdrücklich auf 0.4.0 vertagt;
dies ist eine Risikoakzeptanz, kein Testnachweis. Der 0.4.0-Plan verlangt diese
unabhängige Abnahme vor seinem finalen Release. Umfang und Status bleiben dem
Risikoregister und den Abnahme-Gates des Plans unterstellt.

## Vorgeschlagener Release-Meilenstein 0.5.0

Der zweisprachige [0.5.0-Roadmapentwurf](release-plans/0.5.0.de.md) ist ein
Ausblick, keine Implementierungs- oder Releasezusage. Voraussetzung sind das
finale 0.3.0-Release und eine akzeptierte 0.4.0-Basis. Zentrales Produktziel
sind automatisch verdrahtete Module und der geprüfte Export vollständiger
Anwendungsteile als eigenständige Docker-Deployments. Ein wiederverwendbares,
je Anwendung separat konfigurierbares Datenbankmodul, ausdrückliches
Schemaeigentum, reproduzierbare Extraktion und unabhängiger Ende-zu-Ende-Start
sind P0-Abnahmeanforderungen. Nicht umgesetzte Funktionen bleiben als geplant
gekennzeichnet; PostgreSQL-Runtime-Parität oder Produktionsreife werden nicht
unterstellt.

- [🧪] Im unveröffentlichten 0.4-Zweig erzeugt `zelyra module bundle` nun auf
  Wunsch ein commit-gepinntes Dockerfile, eine Compose-App und ein
  geheimnisfreies `.env.example`; `--dry-run` liefert außerdem eine
  deterministische JSON-Dateiliste der geplanten Ausgaben, ohne das angeforderte
  Ziel anzulegen; das temporäre Testpaket wird geprüft und entfernt. Locale-
  JSON-Dateien werden im Manifest sortiert; ein Wiederholungstest bestätigt
  byte-identische Dateien zweier Exporte desselben Bundles. Ein eigenständiger
  Container-Smoke-Test für eine importierte Route ist grün. Die
  exportierte Anwendung erhält ihre
  `DATABASE_URL` separat, MariaDB bleibt ein externer Dienst. Der Compiler
  lehnt erkannte modulübergreifende Tabellenverweise ohne erreichbaren
  Importabhängigkeitspfad mit `E-MOD-019` ab, auch für Tabellen im Einstieg;
  gemeinsam genutzte Tabellen müssen in einem importierbaren Schema-Modul
  liegen. Tabellen unterstützen außerdem explizite `access`-Listen für
  `read`, `write` und `read_write`; erkannte modulübergreifende Zugriffe ohne
  passenden Grant scheitern mit `E-MOD-021`. CRUD/Formular/Auth sowie unbekannte
  SQL-Zugriffe verlangen `read_write`. Das sind Compiler-Verträge, keine
  MariaDB-Grants; Schemaänderungsrechte und unbekannte SQL-Formen bleiben
  ungeprüft. Datenbanknutzende Module müssen den Provider außerdem direkt oder
  transitiv importieren (`E-MOD-022`); ein Import nur im Einstieg wird nicht
  vererbt. Eine Einstiegskonfiguration muss dafür in ein importierbares Modul
  verschoben werden. `context --format=json` weist Provider, DB-Verbraucher,
  direkte/transitive Bindung und Verbindungsvariable samt tatsächlichem
  `DATABASE_URL`-Fallback ohne Zugangsdaten aus. Die Laufzeit nutzt weiterhin
  eine Verbindung pro Prozess; das ist Auskunft, kein Routing. Der
  MariaDB-Projektgenerator legt die Datenbankdeklaration jetzt
  in `src/database.zyl` ab und importiert sie aus `main.zyl`; das trennt nur
  die Quelle, nicht mehrere unabhängig konfigurierbare Verbindungen. Mehrere
  Datenbankdefinitionen im Graphen scheitern mit `E-DB-001`, bevor ein Bundle
  Dateien schreibt. Das Manifest
  weist weiterhin `source_closure_complete: false` und
  `complete_deployment: false` aus. Vollständige Abhängigkeitsanalyse,
  modulare Datenbankschnittstelle und vollständige 0.5.0-Abnahme bleiben offen.
  `module plan` verfolgt jetzt auch Funktionsaufrufe in Formular- und
  CRUD-Aktionen bis zu den aufgerufenen Deklarationen und deren Quelldateien;
  ein Mehrmodul-Integrationstest prüft beide Fälle. Das erweitert die bekannte
  Kantenmenge, beweist aber keine vollständige Abhängigkeitsanalyse.
  Tabellenspalten verweisen jetzt außerdem auf verwendete benannte Type Aliases;
  dadurch nimmt die bekannte Deklarationsschließung beim Schema-Modul auch die
  Aliasdeklaration auf. Ein Mehrmodul-Regressionstest prüft Ressourcen-Kante
  und Schließung. Typisierte Felder von Formularen, Formularaktionen und CRUD-
  Aktionen verweisen jetzt ebenfalls auf ihre benannten Type Aliases; ein
  Integrationstest prüft Formular- und CRUD-Ressourcenpläne. Auch dies bleibt
  eine statisch erkannte Teilmenge. Typannotationen in Funktionskörpern,
  Record-Konstruktoren und SQL-Ergebnistypen liefern jetzt ebenfalls Typkanten.
  Die MariaDB-Ergebnistypprüfung akzeptiert qualifizierte Record-Namen, indem
  sie vor der Zuordnung zur Tabelle den Modulpräfix entfernt; gezielte
  Tests prüfen den Resolver und den zusammengesetzten Projektpfad.
  Die Veröffentlichung des fertigen Staging-Verzeichnisses erfolgt auf Linux,
  macOS und Windows atomar ohne Ersetzen eines inzwischen angelegten
  Zielverzeichnisses; ein Regressionstest prüft sowohl den Konfliktfall als auch
  die Veröffentlichung bei freiem Ziel.
  Unabhängig davon unterstützt derselbe unveröffentlichte Zweig nun `pub view` und
  `pub component`; importierte Page-/CRUD-Layout-Verweise und erkannte
  Komponenten-Tags in Page-, View-, Komponenten- und CRUD-Slot-HTML verlangen
  eine öffentliche Deklaration und einen ausdrücklichen Importpfad
  (`E-MOD-007` / `E-MOD-020`). Dies ist ein getesteter Sichtbarkeitsschritt,
  kein vollständiges HTML-Namespace- oder Modulvertragssystem.

## Akzeptanzanwendungen aus der Praxis

- [~] Die Maschinenverwaltung ist ein MariaDB-Template mit lokalisierten
  Maschinen-/Bereichs-Views, Suche, Kategorie-/Status-/Bereichsfiltern und
  einer optionalen SQL-Fixture mit 30 fiktionalen Datensätzen. Der
  MariaDB-Integrationstest importiert die Fixture zweimal und prüft die Daten
  über HTTP; die echte Anwendung wird außerdem in allen Deutsch/Englisch- und
  Learn/Work-Kombinationen geprüft. Einsteigergeprüfter Erststart auf sauberen
  Systemen und Produktionshärtung bleiben offen. Native Seed-/Fixture-Befehle
  bleiben optionale Roadmap-Arbeit.
- [~] Kunden-/Auftrags-Akzeptanzanwendung mit Joins, Aggregaten, CRUD-Formularen
  und eigener Projekt-Shell ist enthalten und besteht den wiederverwendbaren
  MariaDB-Tableview-E2E-Pfad; Berechtigungen und der saubere CI-Nachweis für
  die primäre Anwendung bleiben offen.
- [ ] Mehrbenutzer-Inventar mit Transaktionen und parallelen Änderungen.
- [~] Ein erzeugtes Docker-Compose-Deployment mit unabhängig konfigurierbarem
  Web-Port ist getestet; Produktionshärtung bleibt offen.
- [~] Rust-freie Self-Hosted-Installation ist für veröffentlichte Linux- und
  Windows-x86_64-Assets verfügbar; weitere Plattformen bleiben offen.

Diese Datei wird bei jeder Statusänderung eines Meilensteins und bei jeder
neuen verpflichtenden oder optionalen Architekturentscheidung aktualisiert.
