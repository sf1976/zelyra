Warning: truncated output (original token count: 15710)
Total output lines: 958

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

- [x] **0.8.0 stabil veröffentlicht:** Zelyra Studio enthält eine lokale,
  schreibgeschützte Datenbankkarte für MariaDB und PostgreSQL sowie eine
  geführte CRUD-Quelltextvorschau mit Compilerprüfung und ausdrücklicher
  Übernahme. Dazu kommen eine begrenzte MariaDB-Outbox, Kompatibilitäts-Fixtures
  für Sprachlinie 0.1 und veröffentlichte Linux-/Windows-Artefakte samt SBOM.
  Der genaue Nachweis steht in
  [0.8.0-Kandidatenprüfung](release-readiness/0.8.0-candidate-verification.md).
- [x] **Stabiles Release 0.9.0 veröffentlicht:** Geführte Studio-Geschäftsabläufe,
  Rechnungstutorial, vorschau-gebundene `zelyra text edit`-CLI und erweitertes
  Sprachkorpus sind integriert. Optionales lokales TOTP bleibt auf MariaDB mit
  persistenten Sessions und Audit begrenzt. Exakte CI, Linux-/Windows-Artefakte,
  Prüfsummen, SBOMs, Attestierungen und das zweisprachige Website-Handbuch sind
  im [0.9.0-Releaseprüfnachweis](release-readiness/0.9.0-candidate-verification.md)
  bestätigt. Die menschliche Abnahme bleibt für 1.0.0 reserviert.
- [x] **Stabiles Release 0.9.1 veröffentlicht:** Studio-Sitzung, Speichern,
  Problems-Bereich und Rechnerabläufe des Rechnungstutorials sind erweitert;
  Sprachlinie 0.1 verwendet `#`- und `##`-Kommentare. Vollständige CI,
  Linux-/Windows-Artefakte, Prüfsummen, SBOMs, Attestierungen und Website-Update
  sind im [0.9.1-Releaseprüfnachweis](release-readiness/0.9.1-candidate-verification.md)
  belegt. Die unabhängige menschliche Abnahme bleibt für 1.0.0 reserviert.

- [x] **Stabiles Release 0.9.3 veröffentlicht:** Studio-Datenbrowser mit
  begrenzten schreibgeschützten Seiten, geprüfte Quellspaltenänderungen,
  bestätigte MariaDB-Anlage, schreibgeschütztes Laravel-Inventar, native
  Typwerte sowie CSV- und Labyrinth-Beispiele. Main-CI und Releasebuild auf
  dem exakten Tag-Commit sind erfolgreich; Artefakte, Prüfsummen, SBOMs und
  Attestierungen sind veröffentlicht. Die Typmatrix bleibt bei 17 unterstützt,
  5 teilweise und 3 nicht verfügbar. Nachweis: [0.9.3-Kandidatenprüfung](release-readiness/0.9.3-candidate-verification.md). Die menschliche
  Produktabnahme bleibt für 1.0.0 reserviert.
- [x] **Stabiles Release 0.9.5 veröffentlicht:** Typisierte Anwendungs-Mails
  werden transaktional in die MariaDB-Outbox eingereiht und über einen
  ausdrücklich gestarteten SMTP-Worker mit verpflichtendem TLS,
  Authentifizierung, privaten CA-Wurzeln, begrenzten Nachrichten und redigierten
  Fehlern zugestellt. Versionierte Laravel-Prüffunde decken Autorisierung,
  mandantenähnliche Felder, Geldberechnungen und Dateioperationen ab. Das
  Release bleibt experimentell; Zustellung erfolgt mindestens einmal und
  PostgreSQL-Outbox, Templates und Anhänge bleiben offen. Exakte Release-CI,
  MariaDB 10.11/11.4/11.8/12.3, MySQL 8.4, Linux-/Windows-Builds, SBOMs und
  Attestierungen sind am [0.9.5-Release](https://github.com/sf1976/zelyra/releases/tag/v0.9.5)
  angehängt. Die menschliche Abnahme bleibt für 1.0.0 reserviert.

- [~] Der 0.4-Entwicklungszweig ergänzt Anzeige und Sperrung dauerhafter
  Sitzungen mit Verwaltungsrecht, CSRF-Prüfung und Audit-Anfrageereignis.
  Unter `/account/sessions` können angemeldete Nutzer außerdem eigene
  dauerhafte und flüchtige Sitzungen mit CSRF- und Same-Origin-Prüfung anzeigen
  und beenden. Speichersitzungen laufen nach 24 Stunden ab. Eine begrenzte,
  escaped `User-Agent`-Angabe wird optional gespeichert, wenn die Sitzungstabelle
  `device_label` deklariert; sie bestätigt keine Geräteidentität.
  Passwortwiederherstellung ist im 0.4-Entwicklungszweig teilweise umgesetzt:
  MariaDB speichert Einweg-Token-Hashes und eine AES-256-GCM-verschlüsselte
  Outbox, die nach SMTP-Ausfällen (mit 30 Sekunden Wartezeit) und
  Prozessneustarts erneut zustellt. Das E2E
  prüft die Wiederaufnahme nach Neustart, wenn kein älterer Worker aktiv ist.
  Zustellung ist mindestens einmalig; das Zeitfenster zwischen SMTP-Annahme und
  Datenbankbestätigung kann Duplikate erzeugen, Schlüsselverlust lässt wartende
  Nachrichten unlesbar zurück. Generische Antworten, Audit, CSRF, SMTP und
  Sitzungswiderruf werden weiter im MariaDB-/SMTP-Senken-E2E geprüft. Der E2E-Test sendet außerdem zwei
  gleichzeitige Einlösungen desselben Tokens und prüft genau einen Erfolg,
  eine Ablehnung und die Authentifizierung mit nur dem siegreichen Passwort.
  MariaDB-Advisory-Locks serialisieren Token-Ersetzung und Outbox-Einfügung;
  der Worker liest wartende Zeilen in ID-Reihenfolge. Advisory-Locks koordinieren
  Ausstellung und Zustellung auch über mehrere Instanzen; ein Zwei-Instanzen-E2E verzögert das erste
  SMTP-Relay, während die zweite Instanz das Token ersetzt, und prüft, dass
  die spätere E-Mail zum gespeicherten Token gehört. Veraltete Tokens in der
  Mailqueue werden übersprungen. Das MariaDB-E2E lehnt ein Token mit
  `expires_at = NOW()` ab und friert die Datenbankzeit ein, um das produktive
  Token-Prädikat vor, exakt bei und nach Ablauf zu prüfen. Gleichheit wird
  abgelehnt. Reset-Limits verwenden jetzt eine deklarierte MariaDB-Tabelle und
  einen instanzübergreifenden Schlüssel-Fingerabdruck. Unabhängige
  Sicherheitsprüfung und vollständige Kontolebenszyklus-Nachweise bleiben offen.
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
  Antworten entfallen. MariaDB-Poolwartezeiten und Abfragen sind jetzt durch
  die verbleibende Dialogfrist begrenzt; abgelaufene Abfragen werden serverseitig
  abgebrochen. CPU-intensive oder andere blockierende Handler-Arbeit bleibt
  synchron und kann weiterhin einen Worker belegen. Die
  [HTTP-Betriebsanleitung](http-operations.de.md) definiert TCP-Listener-
  Erreichbarkeit, die eingebaute Prozess-Liveness-Route, anwendungseigene
  Readiness und begrenzte Retries für sichere oder idempotente Anfragen.
  Abhängigkeits-Readiness und ihre Abnahme in der erzeugten Anwendung bleiben
  offen; der Server wiederholt nicht automatisch.

- [x] Sprachkern mit Lexer, Parser, AST, Funktionen, Ausdrücken, Kontrollfluss,
  unveränderlichen Bindings als Standard, Arrays, deterministischen typisierten
  Maps, Records, Option, Result und exhaustivem Pattern Matching als Anweisung
  und Ausdruck. `Int`/`UInt`-Überläufe und Division durch null liefern
  kontrollierte Runtimefehler; `Unit` ist in Typsignaturen nutzbar.
  `examples/type_system` hält die geprüfte Typabdeckung und nicht unterstützte
  Bereiche fest.
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
  Host-Port-Be…6710 tokens truncated…Update- oder Installationsbefehle müssen klar getrennt bleiben.
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
  Setup-Ausgabe, Portzuordnungen und Bereinigung. Setup erklärt außerdem, dass
  minimale `init`-Projekte keine Datenbank benötigen, und schlägt bei einem
  fehlenden absoluten Pfad einen wahrscheinlichen relativen Pfad vor. Generierte
  Docker-Builds können einen veröffentlichten Tag, Branch oder Commit festlegen;
  die Referenz wird geholt und detached ausgecheckt. Prüfungen auf sauberen
  Hosts und Wiederherstellungsfälle über diesen Docker-Ablauf hinaus bleiben
  offen.
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

## Release-Meilenstein 0.4.0 (veröffentlicht, nicht vollständig abgeschlossen)

Das stabile `v0.4.0` ist veröffentlicht und die Artefakte wurden technisch
geprüft. Der [Releaseplan 0.4.0](release-plans/0.4.0.de.md) bleibt dennoch
offen: Die unabhängige menschliche Einsteigerabnahme wurde zuerst auf 0.5.0
verschoben und ist durch die neuere Entscheidung des Projektverantwortlichen
auf 1.0.0 verschoben. Sie wurde nicht durchgeführt. Eine manuelle Rückmeldung
mit dem veröffentlichten
0.4.0-CLI deckte außerdem eine falsche absolute Pfadangabe bei `setup` sowie
unklare Erwartungen an `setup` für minimale `init`-Projekte auf. Die Diagnose
und Dokumentation wurden für die 0.5.0-Vorbereitung korrigiert und Regressionen
geprüft; die veröffentlichten 0.4.0-Artefakte enthalten diese Korrekturen
nicht. Details und Grenzen stehen im zweisprachigen 0.5.0-
[Studienprotokoll](release-readiness/0.5.0-onboarding-study.de.md). Es ist kein
Nachweis menschlicher Abnahme.

## Release-Meilenstein 0.5.0 (veröffentlicht)

Das stabile `v0.5.0` ist auf Commit `6c0c0e6` veröffentlicht. Es liefert
experimentelle Docker-Bundles für eine statisch erkannte Teilmenge,
getrennte Schemas und Datenbankkonten mit minimalen Rechten sowie einen
modularen Kunden-/Auftragsablauf. Vollständige Abhängigkeits-/Effektauflösung,
gemeinsames Schemaeigentum und ein wiederverwendbares Mehrverbindungs-
Datenbankmodul sind nicht enthalten. Die technischen Release-Gates,
Artefaktprüfungen und der Installations-/Update-/Rollback-Smoke bestanden.
Menschliche Abnahme fand nicht statt und ist auf 1.0.0 verschoben. Siehe
[Release-Notizen](release-notes/0.5.0.de.md) und
[Verifikationsnachweis](release-readiness/0.5.0-candidate-verification.md).

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
  modulare Datenbankschnittstelle und technische Kandidatenprüfung bleiben
  offen. Die menschliche Abnahme ist auf 1.0.0 verschoben.
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
- [🧪] Das generierte Kunden-/Auftrags-Docker-Rehearsal sichert und restauriert
  Geschäftszeilen nun mit begrenzt berechtigten MariaDB-Konten, wendet eine
  additive Schema-Migration an und beendet danach MariaDB während einer nicht
  bestätigten Kundenänderung hart. Nach dem Neustart prüft es, dass bestätigte
  Kunden-/Auftragszeilen und die migrierte Tabelle erhalten bleiben, während
  die abgebrochene Änderung zurückgerollt wurde. Der Nachweis ist auf eine
  wegwerfbare MariaDB-11-Fixture begrenzt; vollständige Modulschließung und
  Schemaeigentumsdurchsetzung bleiben offen.

## Release 0.6.0

Das stabile [0.6.0-Release](release-plans/0.6.0.de.md) ergänzt einen
nativen, gepoolten PostgreSQL-16-Pfad für direkte SQL-Abfragen mit begrenzten
Parametertypen und Transaktionen. Vollständige PostgreSQL-Parität und
generierte Web-/CRUD-Unterstützung werden nicht versprochen. Das Release
ergänzt eine deterministische Routenübersicht samt Kollisionsfehlern sowie
`zelyra db map`: eine schreibgeschützte Zuordnung von Tabellen und
Beziehungen zum Quellmodul, die Abweichungen zur Live-Datenbank zeigt.
Modulzuordnung bleibt ein Hinweis und ändert weder Schema noch Berechtigungen.
Die menschliche Abnahme bleibt auf 1.0.0 verschoben.

## Release 0.7.0

- [🧪] **Experimentelles Release:** lokales Studio, begrenztes typisiertes
  MySQL-8.4.11-SQL, Rechnungstutorial, erstes Sprachkorpus für 0.1 und ein
  MariaDB-Mandanten-CRUD-Pfad für eine deklarierte Tabelle. Maintainer-
  Sicherheitsprüfung und Mutation-Rollback-E2E sind im 0.7-Plan dokumentiert.
  Dieser Pfad ist auf MariaDB, eine mandanteneigene Tabelle und CRUD ohne
  Beziehungen begrenzt; er ist noch nicht die für 1.0.0 zugesagte vollständige
  mandantenfähige CRUD-Unterstützung für alle Datenbanken. Es gibt keine
  Mandantenmigration oder Backfill. Unabhängiges Sicherheitsreview
  und menschliche Abnahme bleiben Gates vor Produktiveinsatz in 0.9.9/1.0.0.
  Details im [0.7.0-Plan](release-plans/0.7.0.de.md).

## Release 0.8.0

- [🗺️] Eine schreibgeschützte visuelle Datenbankkarte und einen geführten
  CRUD-Assistenten ergänzen. Entwickler wählen Verbindung und Tabelle, prüfen
  Felder und Beziehungen, sehen compilergeprüften Quelltext und übernehmen die
  Änderung ausdrücklich. Sprach-Fixtures für Grammatik, Typprüfung und Formatter
  ausbauen. Dazu kommt die bereits in Prüfung befindliche begrenzte lokale
  MariaDB-Outbox mit ausdrücklich gestartetem Worker und At-least-once-Zustellung.
  Siehe [0.8.0-Plan](release-plans/0.8.0.de.md).

## Release 0.9.0

- [x] **Stabiles Release veröffentlicht:**
  geführte Studio-Geschäftsabläufe, Rechnungspfad, vorschau-basierte lesbare
  Regex-Textbearbeitung und erweitertes Sprachkorpus sind implementiert. Die
  Datenbankaussagen bleiben auf die genaue Servermatrix und belegten Abläufe
  beschränkt. Lokales MariaDB-TOTP bleibt Vorschau bis zum unabhängigen
  Sicherheitsreview vor Produktionsfreigabe. Siehe
  [0.9.0-Plan](release-plans/0.9.0.de.md), Release-Notizen und
  [Releaseprüfnachweis](release-readiness/0.9.0-candidate-verification.md).

## Produktlinie: Sprache, Compiler, Datenbank und Studio

Zelyras zentrale Produktidee ist ein zusammenhängender Weg von verständlichem
Quelltext über Compiler und Datenbank bis zur fertigen Geschäftsanwendung.
Studio soll möglichst alle häufigen, sicher modellierbaren Aufgaben der Sprache
und des Frameworks übernehmen: Eigenschaften auswählen, Abläufe führen, gültige
Optionen erklären, Fehler zeigen und geprüften Quelltext erzeugen. Diagramme und
Präsentationen sind nur Beispiele dafür. Jede generierte Änderung bleibt als
lesbarer Zelyra-Quelltext überprüfbar; folgenreicher Datenzugriff zeigt Plan,
Auswirkungen und eine ausdrückliche Bestätigung. Die Roadmap staffelt diese Ziele über
0.9.2–0.9.8; sie sind noch keine Fähigkeiten des veröffentlichten 0.9.0.

Der Laravel-Migrator ist zugleich ein Umsetzungstreiber für Zelyra selbst.
Blockiert eine angenommene Laravel-Zuordnung durch eine fehlende Zelyra-
Sprach-, Compiler-, Datenbank-, Laufzeit- oder Studio-Funktion, wird diese
Lücke dem dafür nötigen Release zugeordnet und innerhalb von 0.9.3–0.9.8
geschlossen. Eine Zuordnung gilt erst als „direkt abbildbar“, wenn das erzeugte
Projekt den zugesagten Ablauf tatsächlich ausführt. Echte Mehrdeutigkeiten und
unsichere Geschäftslogik bleiben mit Quellbeleg und Begründung ausdrücklich
manuell umzusetzen.

## Release 0.9.1 — IDE-Grundlage

- [🧪] Studio enthält bereits Projektbaum, Tabs, sichtbaren Speicherstatus,
  Rückgängig/Wiederholen, Projektpfadbegrenzung und datei-/positionsbezogene
  Diagnosen. Umbenennen und Löschen von Quelldateien benötigen eine sichtbare
  Bestätigung und einen aktuellen Datei-Hash; vorhandene Ziele werden nie
  überschrieben, `main.zyl` und `zelyra.toml` sind geschützt. Importverweise
  werden beim Umbenennen nicht angepasst. Browser-E2E prüft jetzt Sitzungsstart
  und Neuladen, ungespeicherte Änderungen, Tastaturspeichern, Compilerdiagnosen
  und Neustart. Das Sitzungstoken wird aus der Adresszeile entfernt und für
  diese Tab-Sitzung gehalten. API- und Dateisystemprüfungen decken veraltete
  Änderungen, Pfad-Traversal, Geheimdateien, geschützte Dateien und
  ausdrückliches Anwenden ab. Zugriff bleibt auf Loopback begrenzt; der
  Compiler bleibt maßgeblich. Der getaggte Stand `v0.9.1` bestand die vollständige
  CI; Linux-/Windows-Artefakte, Prüfsummen, SPDX-SBOMs und Attestierungen wurden
  veröffentlicht. Das zweisprachige Website-Handbuch ist live; siehe den
  [0.9.1-Plan](release-plans/0.9.1.de.md).
- Nachweis: Chromium-Browserabdeckung, Editor-API-/Sicherheitsprüfungen und
  MariaDB-/PostgreSQL-Studioabläufe bestehen lokal. CI muss diese für den
  exakten Releasekandidaten wiederholen. Das Rechnungsbeispiel lässt sich öffnen
  und ausführen.
- [🧪] Das Rechnungstutorial zeigt eine Netto-/19%-Umsatzsteuer-/Brutto-
  Rechnung aus den gespeicherten Positionen und eine druckoptimierte Ansicht,
  die der Browser als PDF speichern kann. Der angenommene Standardsatz gilt
  nur für das deutsche Lernbeispiel; dies ist keine native PDF-Engine und keine
  vollständige gesetzeskonforme Rechnungsvorlage.
- [🧪] Die Sprachzeile 0.1 verwendet `#` für einzeilige Kommentare und
  alleinstehende `##`-Zeilen als nicht verschachtelte Blockkommentar-Grenzen.
  `//` bleibt für vorhandene Programme kompatibel.

## Release 0.9.2 — Sprachdienste der IDE

- [✅] Sichere UTF-8-Textdateioperationen für exklusive Erstellung, atomaren
  Dateiaustausch, Anhängen, Kopieren, Umbenennen und Result-basiertes Löschen
  ergänzen. Wörtliche Textsuche, Ersetzen und Teilen unterstützen die
  mehrteilige Konsolen-Notizverwaltung unter `examples/notes_manager` mit
  Sicherungen vor Änderungen, Wiederherstellung, Dateigrenzen, Aktionsprotokoll
  und automatisierten Regressionstests.
- [✅] `assert(condition[, message])` typgeprüft ausführen und falsche
  Bedingungen als Laufzeitfehler melden. Die Tests für Lager/Bestellungen,
  CSV-Import und Notizverwaltung verwenden Assertions; `zelyra run tests.zyl`
  liefert bei fehlgeschlagener Prüfung einen Fehlerstatus. Der Lager-Spielstand
  wird über atomaren Dateiaustausch gespeichert und im Beispiel wieder geladen.
  `text_trim` entfernt Unicode-Leerraum am Rand; der CSV-Import weist damit
  Pflichtfelder zurück, die nur aus Leerraum bestehen.

- [🧪] Der geführte Tutorialbefehl startet MariaDB und Webanwendung und wendet
  das Schema an. Die Compose-Webanwendung bezieht die Datenbank-URL aus der
  Projekteinstellung; Einsteiger müssen weder `DATABASE_URL` setzen noch
  separat `db setup` und `serve` ausführen. Ein responsives Theme gestaltet
  Dashboard, CRUD-Formulare, Listen und Druckansicht.

- [🧪] Studio zur IDE mit einer kontextabhängigen Eigenschaftsseitenleiste und
  Aufgaben-/Befehlsfinder ausbauen. Häufige Abläufe zu Dateien, Symbolen, Routen,
  Views, Formularen, Ressourcen und Diagrammen lassen sich durch Auswahl und
  gültige Eigenschaften einstellen. Beim Schreiben können Beschreibung,
  Optionen und Beispiele zum Befehl am Cursor gezielt aufgerufen werden; Nutzer
  müssen die API nicht auswendig lernen. Die aktuelle Arbeitsfassung ergänzt
  bereits Dateieigenschaften, Hilfe zu zentralen Sprachbegriffen per F1, einen
  filterbaren Aktionsfinder per Strg+Umschalt+P und compilergeprüfte Diagnosen
  während der Eingabe sowie Vorschläge für Deklarationen aus dem Compiler-
  Projektmodell und F12-Sprünge zu Definitionen aus gespeicherten Projektdateien.
  Zusätzlich verfügbar sind eine Literalsuche und lexikalische Bezeichnerfundstellen
  per Umschalt+F12 in gespeicherten `.zyl`-Dateien. Diese Fundstellen lösen keine
  Gültigkeitsbereiche auf und sind keine semantischen Referenzen. Typisierte
  Vervollständigung und symbolbezogene Eigenschaften bleiben offen. Strg+Leertaste
  ergänzt zusätzlich Parameter sowie gültige, im aktuellen Block sichtbare
  `let`-, Schleifen- und Match-Bindungen aus ungespeichertem Quelltext. Typinferenz
  bleibt offen. Der Aktionsfinder startet `main.zyl` nach ausdrücklicher Auswahl
  mit 15-Sekunden-Limit, begrenzter Ausgabe und Ersetzung erkannter Geheimnisse
  aus Prozessumgebung und `.env`. Die Ausführung bleibt ein experimenteller
  lokaler IDE-Ablauf.
- [🧪] Die Projektkarte zeigt Compilerbeziehungen für Modulimporte,
  Deklarationen, CRUD/Tabellen, Seiten/Views und eindeutig aufgelöste
  API-Handler. Quellnavigation ist enthalten. Dynamische und
  Datenbankabfrage-Beziehungen bleiben ausgelassen; mehrdeutige oder begrenzte
  Karten sind als unvollständig markiert.
- [🗺️] IDE-Sprachdienste vervollständigen: scopebewusste semantische Referenzen,
  typisierte Vorschläge, Symbol-/Routen-Eigenschaften und integrierte Verify-
  und Test-Aufgaben ergänzen. Projekt-, Modul-, Routen- und Ressourcendiagramme
  deterministisch vervollständigen. Unbekannte Beziehungen bleiben sichtbar
  unvollständig, statt erraten zu werden.
- [🗺️] Compiler-gestützte Vervollständigung, Symbolnavigation, Suche/Referenzen,
  Formatierung, handlungsorientierte und während der Eingabe aktualisierte
  Diagnosen sowie integrierte Check-, Run- und Test-Aufgaben ergänzen.
  Fortschritt und Ausgabe bleiben sichtbar und geheimnisfrei. Die Funktionen
  decken dokumentierte Sprach-0.1-Konstrukte ab. Zusätzlich entstehen aus dem
  Compiler-Projektmodell verlässliche Architekturdiagramme für Module/Importe,
  Routen und Ressourcenbeziehungen. Knoten und Kanten führen zur passenden
  Quelldatei und Diagnose zurück; unbekannte oder dynamische Beziehungen werden
  sichtbar als unvollständig markiert.
- Nachweis: Positive und negative Mehrdateiprojekte prüfen kontextuelle
  Eigenschaften, Hilfetexte, Vorschläge, Live-Fehlerpositionen, Navigation,
  idempotente Formatierung, Aufgabenausgabe und deterministische Diagrammdaten;
  unbekannte Syntax wird kenntlich gemacht statt erraten.

## Release 0.9.3 — Datenbankwerkbank

Arbeitsumfang und technische Gates stehen im [Releaseplan 0.9.3](release-plans/0.9.3.de.md).

- [✅] CSV-Grundlagen für Import und Export ergänzen: `String[][]` als
  verschachtelten Typ parsen, zitierte Trennzeichen/Quotes/Zeilenumbrüche
  korrekt decodieren und encodieren sowie Dezimaltexte ohne Float exakt in
  skalierte Integer umwandeln. Das Beispiel `examples/csv_catalog` erstellt
  Quelldaten, validiert Felder, protokolliert Fehler, sortiert/exportiert
  gültige Datensätze und prüft den Roundtrip automatisiert. CSV wird als
  vollständiger UTF-8-Text im Speicher verarbeitet, nicht gestreamt. Die
  Zelyra-Tests verwenden `assert`, damit Fehlschläge im Prozessstatus sichtbar
  werden.
- [✅] `text_chars(text)` liefert Unicode-Codepoints als `String[]`. Das
  mehrteilige Beispiel `examples/labyrinth_solver` nutzt es, um unterschiedlich
  große Raster einzulesen, zu validieren und per Breitensuche einen kürzesten
  Weg mit Anzeige, Zählern und Datei-Roundtrip zu berechnen.

- [🧪] Den im 0.9.2-Rechnungstutorial gefundenen Startfehler beheben: Das
  deklarative Weblayout muss das Projekt-Theme laden; die CLI zeigt die lokale
  Browseradresse schon beim Anlegen und behält sie auch bei fehlgeschlagenem
  Schema-Setup bei. Studios **Starten** führt Konsolenprogramme aus und startet
  bei Webprojekten eine lokale Vorschau mit anklickbarer Adresse. Der Start
  beendet eine vorherige Vorschau und Studio beendet die aktive Vorschau beim
  Schließen; **Vorschau beenden** stoppt sie sofort. Datenbanken werden weiterhin
  ausdrücklich separat gestartet. Das Tutorial wird als echtes importiertes
  Mehrdateienprojekt mit eigenem Datenbank-, Kunden-, Artikel-, Rechnungs-,
  Layout- und Dashboard-Modul erzeugt. Der CLI- und Browser-E2E-Nachweis für
  den Studio-Webablauf ist ergänzt; menschliche Abnahme bleibt für 1.0.0.
- [🧪] Die CLI-Hilfe ausbauen: `zelyra <befehl> --help` und verschachtelte
  Aufrufe wie `zelyra db plan --help` erklären Zweck, Optionen, Beispiele und
  Voraussetzungen in Deutsch oder Englisch, ohne den Befehl auszuführen.
- [🧪] Einen schreibgeschützten Laravel-Projektanalysator in der Zelyra-CLI
  beginnen: `zelyra migrate laravel <projekt> --report <pfad>`. Der erste
  statische Inventarlauf erfasst
  Laravel-/Paketversionen, Routen und Middleware, Controller, Models,
  Migrationen, Validierungen, Views, Jobs und Integrationen statisch und
  verknüpft die Funde mit Quellpositionen. Er darf die Anwendung nicht starten,
  Artisan nicht aufrufen, keine Datenbankverbindung öffnen und keine `.env`-
  Dateien lesen. Berichte unterscheiden direkt abbildbare, teilweise
  abbildbare, manuell umzusetzen und nicht sicher zuordenbare Elemente und
  nennen den nächsten sicheren Schritt. Der unterstützte Musterumfang wird
  dokumentiert; mehrdeutige Funde bleiben offen statt erraten zu werden. Die
  erste Fassung erfasst bereits Composer-Versionen, Routen, Models,
  Controller, Schema-Erstellungen, Beziehungen, Validierungsaufrufe, Queued
  Jobs, Views und ausgewählte Integrationen mit Fundstellen in Markdown oder
  JSON. Sie bearbeitet ausschließlich einen neuen Bericht außerhalb des
  Quellprojekts. Versionsmatrix, verknüpfter Projektgraph, Paket-/Regelabgleich
  und Gerüstgenerierung folgen in 0.9.4–0.9.8.
- [🧪] Die Datenbankansicht der IDE zur sicheren Werkbank ausbauen: Tabellen,
  Spalten, Indizes und deklarierte Beziehungen ansehen, Datensätze standardmäßig
  schreibgeschützt durchsuchen und den zuständigen Quellmodulen zuordnen. Das
  ER-Diagramm wird filterbar und navigierbar; Auswahl eines Feldes oder einer
  Beziehung zeigt Definition, Quellmodul und verwendende Ressourcen. Studio
  bietet nun einen sitzungsgeschützten, schreibgeschützten Datenbrowser mit
  höchstens 25 Zeilen je Abruf sowie einen compilergeprüften Assistenten zum
  Ergänzen und Bearbeiten einfacher Quellspalten. Eine deklarierte MariaDB
  lässt sich mit vorhandener Verbindung nach Bestätigung anlegen. Die Auswahl
  der Verbindung, PostgreSQL-/MySQL-Anlage und die vollständige Plan-/Risiko-
  Ansicht bleiben offen.
- [🗺️] Fremdschlüssel bis 1.0.0 als durchgängige Datenbankfunktion liefern:
  Referenzen in Zelyra-Tabellenfeldern deklarieren, Zieltabellen und Feldtypen
  im Compiler prüfen, echte Constraints anlegen und auslesen sowie Beziehungen
  in der Studio-Karte anzeigen. Die unterstützten Abläufe werden getrennt für
  MariaDB, MySQL und PostgreSQL nachgewiesen. Vor dem Hinzufügen zu bestehenden
  Daten prüft der Schema-Plan auf verwaiste Zeilen; bei Fehlern bleiben Daten
  erhalten, riskante Änderungen brauchen eine Prüfung und nicht unterstützte
  Aktionen werden fail-closed abgelehnt. Löschungen sind standardmäßig
  eingeschränkt. Kaskaden oder `SET NULL` müssen ausdrücklich deklariert,
  typgeprüft und je Backend integriert getestet werden, bevor sie als
  unterstützt gelten.
- [🗺️] Das mandanten-gefilterte generierte CRUD über den experimentellen
  MariaDB-Pfad hinaus erweitern: Die Mandantenauswahl an die aktive Mitgliedschaft
  des angemeldeten Benutzers binden, die Ressourcenberechtigung verlangen und
  Mandantenfilter in Listen, Detailansichten, Erstellen, Bearbeiten, Aktualisieren,
  Löschen und Wiederherstellen serverseitig erzwingen. Mandantenschlüssel werden
  serverseitig gebunden und sind keine normalen CRUD-Felder oder Filter. Mit
  Fremdschlüsseln müssen Auswahl und Speicherung verknüpfter Datensätze außerdem
  nachweisen, dass diese zum aktiven Mandanten gehören. Eigene SQL-Aktionen auf
  Mandantendaten müssen den Mandanten sicher berücksichtigen oder vom Compiler
  abgelehnt werden. Der Nachweis umfasst positive und negative Integrationstests
  für zwei Mandanten, alle Lese- und Änderungswege, entzogene Mitgliedschaften
  sowie manipulierte oder widersprüchliche Mandantenauswahl.
- [🧪] Tabellenspalten über die Eigenschaftsseitenleiste grafisch ergänzen und
  bearbeiten. Studio zeigt den vollständigen Schema-Plan mit Auswirkungen und
  Risiken, lässt den Compiler prüfen und verlangt vor jeder Anwendung eine
  ausdrückliche Bestätigung. Außerdem kann der Nutzer den gewählten Datenbanktyp
  und die Verbindung einstellen und — soweit Backend und Rechte es unterstützen
  — die Projektdatenbank aus Studio anlegen. Nicht unterstützte Kombinationen
  werden klar benannt; Zugangsdaten werden nie im Browser gespeichert oder
  ausgegeben. Add/Edit für einfache Quellspalten ist compilergeprüft,
  hashgebunden und ausdrücklich bestätigt. Schlüssel-, eindeutige und
  Default-Felder sind vom visuellen Edit ausgeschlossen. Datenbanktyp,
  Verbindungsdialog sowie PostgreSQL-/MySQL-Datenbankanlage sind noch nicht
  umgesetzt; MariaDB-Anlage nutzt eine bereits konfigurierte Verbindung.
- [🗺️] Lesende, typisierte Datenquellen für Kennzahlen vorbereiten, damit sie
  in 0.9.4 ohne frei formulierte SQL-Ausdrücke gewählt werden können.
- [🗺️] Wiederverwendbare JavaScript-ES-Module als Projektdateien planen:
  gemeinsame Funktionen liegen einmalig in einem klaren Asset-/Client-Bereich
  und können von mehreren Seiten importiert werden. Studio soll Import-/Export-
  Verwendungen auffindbar machen, Referenzen über Dateien hinweg zeigen und
  passende Hilfe und Fehlerstellen anbieten. Häufige Browseraufgaben können
  zusätzlich als einfache Zelyra-Bausteine bereitstehen; sie ersetzen weder
  normales JavaScript noch bestehende JS-Bibliotheken. Die genaue Einbindung,
  Asset-Ausgabe, Sicherheitsgrenzen und Interoperabilität werden vor der
  Implementierung spezifiziert.
- Nachweis: Separate MariaDB-, MySQL- und PostgreSQL-Tests entsprechen der
  veröffentlichten Datenbankmatrix. Nicht verfügbare Schemaoperationen werden
  abgewiesen; Änderungen sind gegen Vorschau-Hash und aktuellen Datenbankstand
  geschützt. Schreibgeschütztes Browsen ist der Standard.

## Release 0.9.4 — CLI-, Setup- und Tutorial-Zuverlässigkeit (abgeschlossen)

- [✅] Jeden dokumentierten obersten und verschachtelten CLI-Befehl mit
  Optionen, Hilfe, Erfolgsweg und erwarteten Fehlern prüfen. Die gemeldeten
  Setup- und Datenbankprobleme nachvollziehen, Verhalten und Erklärungen
  korrigieren und eine zweisprachige Befehlsmatrix mit Regressionstests führen.
  `db create` muss eindeutig nur SQL ausgeben; `db setup` und
  `setup <verzeichnis> --all` müssen ihre tatsächlichen Datenbank- und
  Schemaaktionen benennen.
- [✅] Fortschritt sichtbar ausgeben, während Docker Compose baut und startet.
  Den letzten Fehler beim Schema-Setup ohne Zugangsdaten offenlegen und nach
  dem Start die tatsächliche Anwendungsadresse ausgeben.
- [✅] Die geschützte `.env` des Rechnungstutorials sowohl mit als auch ohne
  `--no-start` bereitstellen. Aus Studio prüfen, dass die Vorschau die
  Projektdatenbank verwendet, und Laden, Anlegen, Weiterleitung und Liste beim
  Kundenformular durchspielen. Bei fehlgeschlagener Abfrage einen sicheren
  nächsten Schritt für Datenbank und angewendetes Schema anzeigen. Nach
  erfolgreichem Tutorialstart den Standardbrowser öffnen und die Adresse in
  Headless-Umgebungen ausgeben.
- [✅] Aus Studio eine noch fehlende MariaDB-Datenbank anlegen: nach sichtbarer
  Bestätigung die geschützten lokalen Compose-Adminzugangsdaten verwenden,
  sonst das konfigurierte Konto versuchen. Kein Host-Programm `mariadb`
  voraussetzen und keine Zugangsdaten an den Browser senden.
- Abschlussnachweis: Zweisprachige Befehls- und Hilfeabdeckung ohne fehlenden
  dokumentierten Befehl; `setup --all` gegen ein isoliertes MariaDB-Compose-
  Projekt; Studio-kompatibler Kundenablauf gegen MariaDB; vollständige
  Workspace-Tests und konsistente Release-Metadaten.

## Gestaffelte Arbeit für 0.9.5–0.9.8

Die folgenden Produktarbeiten bleiben Teil des 1.0.0-Ziels. Sie werden
aufgeteilt, damit jede Version prüfbare Kriterien erhält und Sprach- und
IDE-Rückstand nicht als eine einzige Änderung in 0.9.4 behandelt wird.

- [🧪] Typabdeckung: exklusive Bereichsschleifen, payloadfreie nominale Enums,
  Kalender-/UTC-Zeitstempeloperationen, Decimal-Quantisierung und sichere
  wertsemantische Record-/Arrayänderungen sind in 0.9.8 ergänzt. Der 0.9.9-
  Branch ergänzt benannte typisierte Funktionswerte, begrenzte generische
  Funktionen und Payload-Enums. Generische nutzerdefinierte Records/Enums
  bleiben offen; eine Geschäftsentität mit eigener Identität wird nur bei einem
  belegten Anwendungsfall benötigt. Records bleiben Werttypen; `Int`/`UInt` und `Float`/`Decimal`
  bleiben ausdrücklich getrennt. Compiler, Runtime, Diagnosen,
  Kompatibilitäts-Fixtures, Studio-Hilfe und `examples/type_system` gemeinsam
  aktualisieren. Vor dem 0.9.9-Freeze müssen positive, erwartete
  Compiler-/Runtimefehler- und Serialisierungstests ohne unerwartete Fehler
  bestehen.
- [🗺️] Den Laravel-Analyzer von Inventaren zu belegten Graphen aus Route,
  Middleware, Controller, Model/Beziehung, Migration/Tabelle und View ausbauen.
  Markdown und JSON mit Fundstellen, Paket- und Validierungsfunden, offenen
  Kanten, versionierten Zuordnungen sowie manuellen und sicherheitsrelevanten
  Prüfhinweisen ausgeben. Später eine ausdrücklich angeforderte
  Gerüstgenerierung in ein separates Projekt mit Herkunftsangaben ergänzen;
  nie Geheimnisse oder Produktionsdaten kopieren und keine automatische
  Geschäftsparität behaupten.
- [🗺️] Studio von sicheren Schema- und CRUD-Werkzeugen zu geprüfter relationaler
  CRUD-Gestaltung, Fremdschlüsselauswahl und typisierten Diagrammoptionen in der
  Eigenschaftsseitenleiste ausbauen. Danach compilergeprüfte Diagramme und
  Präsentationsexporte ergänzen. Generierte Abfragen bleiben typisiert und
  lesend; Berechtigungen und Mandantengrenzen gelten. Vor dem ausdrücklich
  bestätigten Übernehmen den gesamten Diff zeigen. Datenbankoperationen nur je
  Backend-Zusage testen; beliebige Client-Hydration bleibt ausgeschlossen.

## Release 0.9.5 — Eigene Mail- und SMTP-Unterstützung

- [x] Die bestehende Laravel-Analyse wurde zu einem versionierten Regelwerk für
  Authentifizierung, Policies/Berechtigungen, Geschäftslogik, Transaktionen,
  Geldfelder, Jobs, Integrationen, Dateiablage und Views erweitern.
  Sicherheitskritische oder verhaltensabhängige Funde müssen zur menschlichen
  Prüfung markiert werden, darunter Statuswechsel, Summen, Nebenwirkungen und
  Fehlerbehandlung. Die statische Analyse bleibt ohne PHP-Pakete und laufende
  Laravel-Umgebung nutzbar.
- [x] Eine erste explizite Mail-API ohne verpflichtenden Anbieter wurde ergänzt:
  MIME-Text/HTML, SMTP-Authentifizierung, implizites TLS, STARTTLS, private
  Zertifizierungsstellen, begrenzte Nachrichten und Anbindung an die bestehende
  MariaDB-Outbox. Der Worker stellt nur ausdrücklich zu; TLS wird nie
  stillschweigend abgestuft. Mail-Capabilities und lokale Testsenke dokumentieren.
- [ ] Sichere benannte Templates und begrenzte Datei-Anhänge in 0.9.7 zusammen
  mit empfängergebundener Benachrichtigungszustellung ergänzen; gemeinsame
  Größen-, Escape- und Herkunftsprüfungen verwenden.
- Abschlusskriterien bestanden am exakten Kandidaten: lokaler SMTP-End-to-End-Test mit vertrauenswürdiger
  lokaler CA, STARTTLS und Authentifizierung; außerdem Tests für
  Konfigurationsablehnung, Größenlimit, Header-Injection, Geheimnisredaktion,
  Capability-Prüfung und transaktionales Einreihen. Zustellung und Duplikatfenster
  zweisprachig dokumentieren. Siehe den [0.9.5-Plan](release-plans/0.9.5.de.md).

## Release 0.9.6 — Benachrichtigungszentrum

- [🗺️] Eine ausdrücklich angeforderte Gerüsterzeugung mit
  `zelyra migrate laravel <projekt> --output <ziel>` ergänzen; `--report` und
  `--output` dürfen gemeinsam verwendet werden. Nur auf Wunsch ein separates
  Zelyra-Projekt erzeugen, Quellfundstellen und IDs der Zuordnungsregeln in
  generierten Dateien erhalten und niemals Geheimnisse, `.env`-Werte,
  personenbezogene Einträge oder Produktionsdaten kopieren. Überlappende
  Quell-/Berichts-/Zielpfade und unsichere bestehende Ziele ablehnen. Der Bericht
  muss Gerüst und tatsächlich übertragene Geschäftslogik unterscheiden; jedes
  erzeugte Projekt muss mit dem passenden Zelyra-Compiler geprüft werden.
  Blockaden angenommener Zuordnungen werden als Zelyra-Umsetzungsaufgabe in
  derselben Release-Reihe geführt und behoben, bevor die Zuordnung als
  unterstützt gilt.
- [🗺️] Einen mandantengebundenen gemeinsamen In-App-Benachrichtigungsfeed mit
  ungelesener Anzahl, Seitennavigation, Ablauf-Filter, Soft-Delete-Archiv und
  bearbeitbaren Einstellungsdatensätzen ergänzen. Dieser erste Schritt behauptet
  noch keine persönlichen Lesebestätigungen, geplante Löschung nach Ablauf oder
  ereignisgesteuerte E-Mail; dafür sind empfängergebundene Zustellung und 0.9.8
  vorgesehen. Der Feed bleibt ohne SMTP nutzbar.
- Nachweis: MariaDB-Integration belegt, dass Mitglieder Nachrichten anderer
  Mandanten weder auflisten noch zählen können, archivierte und abgelaufene
  Einträge wie konfiguriert ausgeblendet werden und erratene mandantenfremde IDs
  verborgen bleiben. Lokalisierung, erzeugte Formulare und Fehlerzustände sind
  abgedeckt. Ein externer Pushdienst ist nicht erforderlich.

## Release 0.9.7 — Studio, Rechnungstutorial und Härtung

- [✅] Das Rechnungstutorial wird in `main.zyl` und Fachmodule gegliedert und
  erhält ein Desktop-Dashboard mit echten Kennzahlen sowie einen geführten
  Kunden-/Artikel-/Rechnungsablauf. Der Tabler-CSS-Bestandteil ist lokal,
  optional und MIT-lizenziert; keine CDN-Abhängigkeit. Druckausgabe mit Netto,
  19% Umsatzsteuer und Brutto bleibt Teil des Lernpfads.
- [✅] Projektkarten-Fehler beheben: Views über Modulgrenzen nur innerhalb des
  eigenen Moduls oder über importierte öffentliche Views auflösen, CRUD-Layouts
  berücksichtigen und TableViews anzeigen. Begrenzungen und offene Beziehungen
  über lokalisierte, stabile Gründe erklären; keine SQL-Beziehungen erraten.
- [✅] Responsives Browser-E2E für Desktop und schmale Ansichten, reale
  Kennzahlen, generierte CRUD-Formulare, lokale Tabler-Auslieferung ohne
  Netzwerk und druckbare Rechnung ergänzen.
- [🗺️] Persönliche Lesebestätigungen und Empfängerautorisierung,
  Aufbewahrung, sichere Templates/Anhänge und optionale SMTP-Zustellung in
  begrenztem Umfang in 0.9.8 liefern. Atomare Einreihung, idempotente
  Wiederholungen, fremde Mandanten-/Empfänger-IDs und Zustellungsduplikate
  prüfen.
- [🗺️] Die Laravel-Analyse für Mandantengrenzen, Autorisierung,
  Datei-Downloads/-Uploads, Zahlungs-/Buchhaltungsabläufe und öffentliche
  Endpunkte bis 0.9.8 härten. Fehlende und mehrdeutige Belege erfordern
  manuelle Prüfung.
- [🗺️] Die für 1.0 zugesagte MariaDB-, MySQL- und PostgreSQL-Ablaufmatrix
  schließen; genaue Serverversionen, unterstützte Abläufe und Grenzen
  dokumentieren. Beziehungen, TLS, Backups, Wiederherstellung und
  mandanten-gefiltertes CRUD mit negativen Autorisierungstests prüfen.
- Abschlusskriterien: Der Projektkarten-Regressionstest löst das importierte
  Tutorial-Layout auf, die Browserprüfung zeigt echte Kennzahlen und alle
  Befunde/Backendgrenzen sind dokumentiert. Der vollständige Umfang steht im
  [0.9.7-Plan](release-plans/0.9.7.de.md).

- [🗺️] Das Benachrichtigungszentrum um persönliche Lesebestätigungen,
  Empfängerautorisierung, begrenzte Aufbewahrung, sichere benannte Templates und
  optionale SMTP-Zustellung über die transaktionale Outbox erweitern. Atomare
  Einreihung, idempotente Wiederholungen, mandanten- und empfängerfremde
  ID-Angriffe sowie SMTP-Duplikatverhalten prüfen.
- [🗺️] Die Laravel-Analyse für Mandantengrenzen, Autorisierung,
  Datei-Downloads/-Uploads, Zahlungs-/Buchhaltungsabläufe und öffentliche
  Endpunkte härten. Soweit die unterstützte Syntax reicht, Belege über Routen,
  Middleware, Controller, Models und Beziehungen verfolgen und manipulierte
  IDs sowie Mandantenüberschreitungen markieren. Bei fehlenden oder mehrdeutigen
  statischen Belegen ist eine manuelle Prüfung erforderlich; Verhalten darf
  nicht stillschweigend ausgelassen werden. Test-Fixtures für Mandantentrennung,
  Fremdschlüssel, Transaktionen, Dezimal-Geldbeträge, Berechtigungen und
  Dateiablage ergänzen.
- [🗺️] Die für 1.0 zugesagte MariaDB-, MySQL- und PostgreSQL-Ablaufmatrix
  schließen: Schemaansicht, Migrationen, CRUD, Transaktionen, TLS, Backup und
  Restore, soweit ein Backend diese Funktion beansprucht. Unabhängige Reviews
  von Authentifizierung, MFA, Mandantentrennung, Autorisierung, CSRF,
  SMTP-Geheimnissen und Studio-Änderungsgrenzen abschließen und alle
  releaseblockierenden Befunde beheben.
- [🗺️] Die vollständige Autorisierungsmatrix für mandanten-gefiltertes CRUD
  mit den festgelegten MariaDB-, MySQL- und PostgreSQL-Versionen durchlaufen.
  Mandantenauswahl, Mitgliedschaft und Berechtigung, generierte Listen und
  Detailansichten, Erstellen/Aktualisieren/Löschen/Wiederherstellen,
  Mandantenzugehörigkeit von Beziehungen und fail-closed-Verhalten bei fehlendem
  oder entzogenem Kontext abdecken. Mandantenmodell und Migrationserwartungen
  dokumentieren; keine automatische Migration bestehender Daten versprechen.
- Nachweis: Exakte Serverversionen und Grenzen sind veröffentlicht; jeder
  unterstützte Ablauf hat CI-Nachweise, negative Autorisierungstests und eine
  maintainergeprüfte Entscheidung zu Sicherheitsbefunden. Keine universelle
  SQL-Parität behaupten.

## Release 0.9.8 — Probe der Produktionsreife

- [✅] Sprachstabilisierungsumfang umgesetzt: kontrollierte Grenzen für tiefe
  Funktionsaufrufe, vollständige Rückgabepfade, direktes `Int::MIN`,
  Kalender-/UTC-Arithmetik, Decimal-Quantisierung, payloadfreie Enums,
  exklusive Integer-Bereiche, wertsemantische Record-/Arrayänderungen,
  getaggter Result-JSON-Codec und iterative Verarbeitung langer Ausdrücke.
  Workspace, Clippy, Formatierung, 324 Handbuchbeispiele, alle 80 Korpusfälle,
  100 Wiederholungsläufe sowie 300 Integer-Division/Rest- und 100 Decimal-
  Referenztests bestanden. Die 0.9.9-Wiederholung schließt die zehn früheren
  Sprachlückenproben; vier begrenzte Beobachtungen sind in Spezifikation und
  Typmatrix ausgewiesen.
- [✅] Die helle Studio-Darstellung deckt Editor, Projektnavigation,
  Eigenschaften, Hilfe, Datenbankwerkbank, Aktionssuche und Dialoge ab.
  Browser-E2E prüft Speicherung, Tastaturbedienung, einheitliche Flächen und
  mindestens 4,5:1 Kontrast für Text und Syntaxfarben. Bei 360 CSS-Pixeln
  bleiben die Arbeitsfläche und der Theme-Schalter ohne horizontalen Überlauf
  nutzbar. Die IDE-Arbeit geht mit den grafischen relationalen und
  Diagrammabläufen für 0.9.9 weiter.
- [✅] MySQL Server 8.4.11 besteht nun den isolierten Schemaablauf über
  `db setup`, `inspect`, schreibgeschütztes `map`, geprüften `plan`, bestätigtes
  `apply` und `history`. Der CI-Job prüft außerdem parametrisierte SQL-Abfragen.
  Der Nachweis deckt kein vollständiges CRUD/Auth, MFA, TLS, Backup/Restore oder
  allgemeine MariaDB-Parität ab; die Backendmatrix nennt die Grenzen.
- [🗺️] Nach 0.9.9 übertragen: Den unterstützten Umfang des Laravel-Migrators und seine versionierte
  Kompatibilitätsmatrix abschließen. Schreibgeschützte Analyse,
  Geheimnis-/PII-Redaktion, Quellpositionsgenauigkeit, Routen-Daten-Beziehungen,
  Behandlung von Mehrdeutigkeiten, Pfadtrennung, Herkunftsverweise in
  generierten Dateien und compilergeprüfte Ausgabe an repräsentativen Laravel-
  Fixtures prüfen. Erzeugte Anwendungen mit echten CRUD-Abläufen auf den
  unterstützten Datenbanken ausführen, einschließlich unterstützter Beziehungen,
  Mandanten, Berechtigungen, Transaktionen und Geldbeträge. Jede Zelyra-Funktion,
  die eine angenommene Zuordnung blockiert, vor dem Abschluss von 0.9.9
  implementieren. Unterstützte Laravel-/Paketmuster und Grenzen dokumentieren.
  Keine automatische Geschäftslogik- oder Sicherheitsparität behaupten; das
  Werkzeug liefert einen überprüfbaren Migrationsstartpunkt.
- [🗺️] Nach 0.9.9 übertragen: Saubere Installation, Upgrade, Rollback, Migration, Backup und Restore
  auf unterstützten Betriebssystemen und Datenbanken proben. Betreiber- und
  Nutzerdokumentation, Lizenz-/Abhängigkeitsreview, Ressourcenlimits,
  Geheimnisredaktion, reproduzierbare Artefakte, SBOM/Provenance und Offline-
  Abläufe abschließen. Präsentationen und Diagramme auf sauberen Umgebungen,
  barrierearme Lesbarkeit, stabile Exporte und Geheimnisfreiheit prüfen. Start,
  übliche CRUD-Abläufe und begrenzten Ressourcenbedarf messen und Ergebnisse
  ohne unbelegte Leistungsversprechen veröffentlichen.
- [🗺️] Verbindliche 0.9.8-Übernahmen in 0.9.9 schließen: saubere Umgebungs-
  und Datenbankproben, Sicherheits-/Abhängigkeitsentscheidungen, integrierter
  Studio-/Tutorial-/Mail-/Benachrichtigungsablauf, Laravel-Migrator-E2E sowie
  Diagramm- und Präsentationsnachweise. Siehe [0.9.8-Plan](release-plans/0.9.8.de.md)
  und [0.9.9-Plan](release-plans/0.9.9.de.md).

## Release 0.9.9 — Feature-Freeze und letzter technischer Kandidat

- [✅] Typisierte Decimal-Tabellenansicht zeigt exakte Werte ohne zusätzliche
  Nullauffüllung; normale Texte und Rohdaten behalten ihre Darstellung.

- [✅] Setup-Abhängigkeit nach Präzisionshärtung korrigieren: MariaDB zuerst,
  Schema im separaten Containerlauf, Webdienst erst nach Erfolg starten;
  Migrationsfreigaben bleiben wirksam.

- [✅] Dezimalpaket: feste SQL-Skala, verlustfreier JSON-Transport, exakte
  Formulare/Filter und PostgreSQL-NUMERIC-Dekodierung; Alias- und Grenzwerte,
  Zwischenüberlauf und kontrollierte Ressourcenfehler werden geprüft.
  Siehe [Regeln](decimals.de.md) und [Paketprotokoll](release-readiness/0.9.9-exact-decimals.de.md).
- [✅] Kalender-/UTC-Rechnung, typisierte Formular-/HTTP-Eingaben und native
  SQL-Date-/Time-/Timestamp-Parameter/Speicherung bestehen auf MariaDB, MySQL
  und PostgreSQL; UTC-Sitzungen und exakte Zeitpräzision werden geprüft.
  Siehe [Regeln](dates.de.md) und [SQL-Nachweise](release-readiness/0.9.9-calendar-sql.de.md).
  Die SMTP-/Reset-Fixture berücksichtigt unterdrückte ersetzte Zustelljobs
  und prüft vor dem Fortsetzen das tatsächlich gespeicherte Ersatztoken.
  Business-Readiness-Prüfungen beachten den nötigen Metadatenzugriff vor dem
  Start samt kontrolliertem, redigiertem Fehler bei unerreichbarer Datenbank.
- [✅] Explizites `sql_decode<T>(sql<…> { … })` erzeugt Skalar-/Record-/Array-
  Werte mit geprüfter Kardinalität, Projektion, nativen Typfamilien und NULL-
  Masken. Exakte Zahlen und Kalenderfolgeoperationen bestehen auf drei realen
  Backends; bestehendes Rohzeilen-HTTP-/JSON bleibt erhalten. Siehe
  [Regeln](sql-results.de.md) und [Nachweise](release-readiness/0.9.9-typed-sql-results.de.md).

- [🗺️] Sprach-, CLI-, Runtime-, Datenbank-, IDE-, Mail-, Benachrichtigungs- und
  Tutorialumfang einfrieren. Alle angenommenen Anforderungen aus 0.9.1–0.9.8
  umsetzen und die vollständigen technischen Gates an einem unveränderlichen
  Kandidaten ausführen. Materialien für die exakte 1.0.0-Menschenstudie
  vorbereiten. Danach sollte nur noch die unabhängige menschliche Abnahme offen
  sein; für 0.9.9 keine Abnahme und keine Produktivfreigabe behaupten.
- [✅] Sprachprüfung gegen den 0.9.9-Quellbuild: 80 Fälle ohne unerwartete
  Ergebnisse, 20 Aufgaben, 35 kontrollierte Negativfälle, 100 Wiederholungen
  und 400 unabhängige Zahlenvergleiche bestanden. Die vier Beobachtungen und
  der Linux-only-Umfang sind im [Prüfbericht](release-readiness/0.9.9-language-review.de.md)
  festgehalten. Nach dem Clippy-Kompatibilitätsfix in `1bf09ea` wurde die
  Prüfung mit Compiler-SHA `89b880d753e12ac09b2fbb4dc754586d18c7a808845b991b331cb1ae00153c0d`
  wiederholt.
- [🧪] Rust-Pakettests, Formatierung und Clippy sowie Installer-, sauberes
  MariaDB-Onboarding und generierte Docker-/Modul-Bundles bestehen auf `1bf09ea`
  lokal. CI und Paketierung bestehen auf dem Dokumentationsnachfolger `8db093c`;
  folgende Quellcodepakete und der endgültige Kandidat brauchen ihre eigene CI.
  Siehe das
  [Prüfprotokoll](release-readiness/0.9.9-technical-rehearsal-2026-10-12.de.md).
- [🗺️] Verbleibende CLI-, Setup-, Datenbank-, Migrator-, Sicherheits-,
  Dokumentations- und Paketierungsgates schließen. Allgemeine Constraints/
  Traits, Closures und generische Funktionsreferenzen bleiben ausdrücklich
  dokumentierte Grenzen der Sprachlinie. Der
  [0.9.9-Plan](release-plans/0.9.9.de.md) enthält die prüfbaren Kriterien.
- [✅] Eine optionale farbige CLI-Hilfe mit `ZELYRA_COLOR`, `NO_COLOR` und
  `CLICOLOR_FORCE` sowie einen kurzen Studio-Startbildschirm mit gespeicherter
  Darstellung und Reduced-Motion-Beachtung ergänzen und prüfen.
- [✅] Generische Funktionen, Records und Enums sind im 0.9.9-Branch umgesetzt:
  Typargumente werden abgeleitet oder ausdrücklich angegeben, auch in
  verschachtelten Records, Arrays, `Option` und `Result`. Konfliktierende,
  fehlende und falsch gezählte Typen haben Regressionstests. Das
  Type-System-Beispiel deckt generische Modulimporte sowie positive und
  negative Fälle ab. SQL-Spalten lehnen generische Records ausdrücklich ab.
  Payload-Enums, benannte typisierte Funktionswerte und indirekte Aufrufe
  bestehen ebenfalls gezielte Tests einschließlich Capability-Weitergabe.
  Allgemeine Constraints/Traits, generische Aliase, Closures und generische
  Funktionsreferenzen bleiben außerhalb des unterstützten Umfangs.
- [🧪] Studio-Diagramme unterstützen nun feste Aggregationen, exakte
  Textfilter, inklusive `DATE`-Zeiträume, sichere Parameterbindung, Speichern
  und erneutes Laden der Konfiguration sowie lokale Präsentations-/PDF-
  Vorschauen. MariaDB- und PostgreSQL-Browser-/Datenbank-E2E bestehen.
  Gespeicherte SQL-Ansichten, Einbindung in Anwendungsseiten, Compilerprüfung
  der Berichtsdateien sowie vollständige Barrierefreiheits- und Exportnachweise
  bleiben Releasekriterien.
- [✅] Persistente MariaDB-Reset-Limits, begrenzte Bereinigung/Kapazität,
  bereinigte Betreiberfehler und Reset-Speicherabhängigkeiten bestehen positive
  und negative Prüfungen. Erzeugte SQL-Namen halten Backend-Grenzen bei
  Erstellung und Planung ein; zu lange Deklarationen scheitern vor SQL.
  Rust-, MariaDB-SMTP-/Limit- und PostgreSQL-Schemaprüfungen stehen im
  [Paketprotokoll](release-readiness/0.9.9-reset-and-schema.de.md).
  Unabhängige Sicherheitsprüfung bleibt offen.
- [🗺️] Visuelle Formular- und Ausgabeeditoren mit Eigenschaftsleiste für
  Felder, Prüfung, Datenbank-/Tabellenbindung, Spaltenauswahl, Filter,
  Sortierung, Suche, Pagination und Diagramme ergänzen. Editierbaren
  Zelyra-Code und lokale Styles mit Compiler-Vorschau und Schutz manueller
  Änderungen erzeugen.
- [✅] Ein begrenztes, Zelyra-gerechtes Klassenmodell bietet nun typisierte
  Konstruktoren, geteilte Referenzidentität, Identitätsgleichheit, Feldzugriffe
  und -updates, generische Klassen sowie optionale Felder. Runtimeprüfungen
  verhindern Referenzzyklen und parallele Mutation; JSON-/API-/SQL-Grenzen
  lehnen Klassenreferenzen ab. Verhalten wird mit gewöhnlichen Funktionen und
  ausdrücklich übergebenem Empfänger geschrieben. Methoden-, Closure- und
  Vererbungssyntax bleiben außerhalb des 0.9.9-Umfangs.
- Abschlusskriterien: Keine offenen Implementierungs-, Sicherheits-, Datenbank-,
  Migrations-, Paketierungs- oder Dokumentationsblocker. Bleibt einer offen, ist
  0.9.9 nicht fertig. Siehe den [0.9.9-Plan](release-plans/0.9.9.de.md).

Eine allgemeine Queue-Plattform bleibt hinter 1.0.0. Die Outbox in 0.8 ist
lokal und auf MariaDB begrenzt; sie bietet weder Scheduler noch verteilte
Worker oder Exactly-once-Zustellung. Vollständige SQL-Parität, beliebige
Client-Hydration, Compiler-Self-Hosting, natives Backend, verpflichtende
Cloud-/KI-Dienste, Telemetrie und unbelegte Leistungsversprechen bleiben
ausgeschlossen.

Die detaillierten Abnahmekriterien stehen in den Plänen für [0.8.0](release-plans/0.8.0.de.md),
[0.9.0](release-plans/0.9.0.de.md), [0.9.5 Mail](release-plans/0.9.5.de.md),
[0.9.8 Stabilisierung](release-plans/0.9.8.de.md),
[0.9.9 Freeze](release-plans/0.9.9.de.md) und [1.0.0](release-plans/1.0.0.de.md).
## Erste produktive Veröffentlichung 1.0.0

`v1.0.0` ist die erste produktive Veröffentlichung und das verpflichtende Ziel für
alle unabhängigen menschlichen Einsteiger- und Abnahmestudien, die aus 0.3.0,
0.4.0 und 0.5.0 verschoben wurden. Kein früheres Release darf behaupten, diese
Studien seien abgeschlossen. Vor dem stabilen `v1.0.0` muss nach bestandenen
technischen Gates ein unveränderlicher Release Candidate veröffentlicht werden.
Unabhängige Personen ohne Entwicklerrolle prüfen den Kandidaten und die
mitgelieferte Dokumentation. Blocker werden behoben und erneut geprüft;
redigierte Beobachtungen und die Entscheidung des Projektverantwortlichen
werden dem exakt getesteten Commit zugeordnet. Der Studienumfang muss zum
endgültigen 1.0.0-Produkt passen. Automatisierte CI, interne Rehearsals und
Rückmeldungen des Projektverantwortlichen ersetzen keinen Teilnehmertest.

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
