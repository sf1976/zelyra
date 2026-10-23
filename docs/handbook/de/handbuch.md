# Das Zelyra-Handbuch

**Von den Grundlagen bis zur datenbankgestützten Webanwendung. Absicht beschreiben. Korrektheit beweisen.**

[English edition](/handbook) · Deutsch

Willkommen beim vollständigen Zelyra-Handbuch. Es umfasst sowohl das didaktische Lehrbuch **»Zelyra lernen – Verständlich programmieren von den Grundlagen bis zur eigenen Anwendung«** (Teil I bis X, Kapitel 1 bis 43) als auch das **technische Referenzhandbuch** (Abschnitte 1 bis 36) sowie ausführliche **Anhänge** (A bis K).

## Zelyra 1.2.24: aktueller Funktionsumfang

Dieses Handbuch beschreibt den Stand **Zelyra 1.2.24** (Stand: 19. Oktober 2026) mit Ausblick auf das Roadmap-Meilensteinziel 1.3.0. Zelyra 1.0.0 war die erste stabile Hauptveröffentlichung; die Sprachkompatibilitätslinie bleibt `0.1`. Der Umfang einzelner Compiler-, Datenbank-, Betriebs- und IDE-Funktionen ist präzise definiert und wird in den jeweiligen Abschnitten beschrieben. Der frühere, experimentelle Laravel-Migrator (`zelyra migrate laravel`) wurde in Version 1.1.2 vollständig aus dem Zelyra-Produkt entfernt; Zelyra konzentriert sich kompromisslos auf native, typ- und vertragsgestützte Anwendungsarchitektur.

- **Sprache und Typsystem:** typisierte Funktionen, unveränderliche Bindings, benutzerdefinierte generische Records und Enums, payloadtragende Enums, benannte Funktionsreferenzen (`&name`), exhaustive `match`-Prüfung, exakte 9-stellige Dezimalrechnung (`Decimal`), typisierte Kalender- und Datums-/Zeitstrukturen (`Date`, `Time`, `DateTime`, `Instant`), Contracts (`requires`, `ensures`) und deklarierte Capabilities.
- **Spracherweiterungen und Laufzeit:** typisierte wertliefernde `if`-Ausdrücke, funktionale Record-Updates (`Customer { ..old, active: true }`), typisierte lexikalische Lambdas in Pipelines (`map`, `filter`, `find`, `reduce`), boolesche Match-Guards und verschachtelte Record-Muster, First-Class-Tupel als Rückgabewerte und Zuweisungsziele, kontextuelle Feldnamen (`version`, `type`), nominale Id-Aliase (`type OrderId = Id`), konfigurierbare globale Schrittbegrenzung (`--max-steps N`), serverseitiges lokales Text-PDF-Rendering (`render_pdf`) und transaktionale PDF-Mail-Anhänge über die MariaDB-Outbox.
- **Datenbanken und relationales CRUD:** Multi-Backend-Unterstützung für geprüfte Abläufe mit Servermatrix für MariaDB 11.4.13, MySQL 8.4.11 und PostgreSQL 16.15. Typisiertes parametrisiertes SQL, Transaktionen mit dynamischen Deadlines (`innodb_lock_wait_timeout`), Fremdschlüssel-Constraints mit Waisenprüfung (`error[E-DB-005]`), relationale Joins, mandantenisoliertes CRUD, geordneter restriktiver Löschschutz (RESTRICT) und Schema-Vorschau mit kontrolliertem Anwenden (`zelyra db plan/apply/history`).
- **Mandantensicherheit und zusammengesetzte Fremdschlüssel (v1.2.17 – v1.2.19):** `table ... scoped by tenant_id: TenantId` erzwingt die Mandantenzugehörigkeit auf Datenbankebene. Zelyra ergänzt `uq_{table}_tenant_id UNIQUE (tenant_id, id)` und generiert zusammengesetzte Foreign Keys `FOREIGN KEY (tenant_id, fk_id) REFERENCES target (tenant_id, id) ON DELETE RESTRICT`. `unique within tenant` deklariert mandantenspezifische Eindeutigkeit (`customer_number: String(50) required unique within tenant`), und optionale Relationen (`Customer?`) erzeugen typsicher `BIGINT NULL`.
- **Tabelleninvarianten und typisierte Validierung (v1.2.20):** `table ... { validate { due_date >= invoice_date: "..." } }` verankert typgeprüfte, feldübergreifende Invarianten direkt am Datenmodell. Formulare, CRUD und schreibende APIs (`validates`) erzwingen diese Regeln vor dem Schreibvorgang und liefern standardisierte HTTP-422-Verträge.
- **Lifecycle-Guards und Zeilenpolicies (v1.2.21):** Deklarative Sperren für `edit` und `delete` (`guard edit when locked == true { error: "..." }`) blockieren illegale Mutationen direkt in der Mutationstransaktion. `tableview`-Aktionsslots und Detailseiten blenden Bearbeiten- und Löschen-Aktionen zustandsabhängig aus.
- **Transaktionale Master-Detail-CRUD-Formulare und mehrere Kind-Sammlungen (v1.2.22 – v1.2.24):** `has_many items: InvoiceItem { ... }` und zusätzliche Sammlungen (`has_many attachments: InvoiceAttachment { ... }`) ermöglichen die serverseitig gerenderte Erfassung von Kopfdaten und mehreren unabhängigen 1:N-Kinddatensammlungen in einem einzigen Formular mit atomarer Transaktion und automatischer Summenaktualisierung (`aggregates`).
- **Geprüfte Mandanten-APIs (v1.2.23):** Mandantengebundene APIs (`scoped by tenant_id`) validieren den Mandantenkontext verbindlich aus geprüften Benutzersitzungen.
- **Kanonische Projektstruktur (v1.2.13 – v1.2.14):** Mit `zelyra new --layout standard` wird ein architekturgeprüftes Standardlayout angelegt (`routes/`, `domain/`, `schema/`, `public/`, `assets/`). Der Linter `strict_architecture = true` prüft Deklarationsbereiche (`E-ARCH-001`), schreibendes oder nicht sicher klassifizierbares SQL in statisch erreichbaren API-Handlern und Form-/CRUD-Aktionen (`E-ARCH-002`/`003`) und verbietet Route-Importe in Domain-Modulen (`E-ARCH-004`); `zelyra migrate-layout --check` liefert eine schreibgeschützte AST-Migrationsvorschau.
- **Erweiterbare CRUD- und TableView-Seiten (v1.2.15):** Automatisch generierte Create/Edit-Seiten (`crud.view.create`, `crud.view.edit`) und `tableview`-Listen unterstützen compilergeprüfte Inhaltsbereiche, Feldhilfen und Zeilenaktionen, während Validierung, CSRF-Schutz, Berechtigungen und Persistenz zentral abgesichert bleiben.
- **Dienste, Scheduler und Hintergrund-Jobs (v1.2.5 – v1.2.9):** `zelyra service install --port <port>` installiert Linux-systemd-Dienste mit festem Port und Statusüberwachung. Deklarierte Zeitpläne laufen im residenten Scheduler; der Hintergrund-Worker wickelt Jobs mit erneuerbaren Leases transaktional über die MariaDB-Outbox ab (`zelyra schedule jobs`).
- **Paket- und Modulökosystem (v1.1.5 – v1.2.12):** Wiederverwendbare Zelyra-Pakete (`pkg:`), Offline-Lockfile (`zelyra.lock`), Ed25519-signierte Registry-Pakete mit zyklusfreier Auflösung, transaktionale Wiederherstellung (`zelyra package recover`) und eine authentifizierte Web-Einstellungsseite (`/__zelyra/module-settings`).
- **Zelyra UI & Designsystem:** lokal gebündeltes, compilergeprüftes Designsystem ohne npm oder Webpack (`zelyra-ui.v1.css`). Typisierte UI-Komponenten, Themes (`light`, `dark`, `system`), harmonisiertes AppShell-Design, Modal-Dialoge und responsives Drucklayout.
- **Zelyra Studio (IDE):** lokaler Editor mit Sitzungsschutz, Speichern, Live-Diagnosen, Kontexthilfe, Aktionsfinder, Code-Vervollständigung, Navigation, strukturiertem Log-Fenster, Theme-Editor und CRUD-Vorschau.
- **Rechnungstutorial:** Kunden, Artikel, Rechnungen und Positionen im einheitlichen Zelyra-UI-Theme, mit interaktivem „Quelltext erklärt“-Dialog, sauber ausgerichteten Belegkarten und browserbasiertem Druck.
- **E-Mail und Sicherheit:** transaktionale Outbox (`mail_queue`), SMTP-Worker mit TLS und CA-Validierung, Header-Injection-Schutz, persistente Passwort-Reset-Quoten (`password_reset_limits`) und integrierte TOTP-Zwei-Faktor-Authentifizierung (RFC 6238, AES-256-GCM, Argon2id Recovery-Codes).
- **Benachrichtigungen:** mandanten- und empfängerisoliertes In-App-Zentrum (`recipient: user_id`), dynamischer Zähler ungelesener Nachrichten (`unread: read_at`), persönliche CSRF-geschützte Lesebestätigung mit Audit-Protokollierung.
- **Dual-Tier-Ausführungsarchitektur (Roadmap 1.3.0):** AST-Referenzinterpreter als transparentes Entwicklungs- und Test-Orakel, kombiniert mit einer High-Speed-Pipeline über HIR, typisierte MIR (CFG, Basisblöcke) und iterative Bytecode-VM (v24-Format) für native Ausführungseffizienz.

### Änderungen seit Version 1.0.0

- **1.0.1:** Compose v2 wird unterstützt; Python Compose v1 wird mit einem Update-Hinweis abgewiesen. Der Ausführen-Knopf in Studio startet den Projekteinstieg.
- **1.0.2:** Studio kann beim lokalen Rechnungstutorial MariaDB starten, das Schema anwenden und Beispieldaten laden. `for`-Bereiche unterstützen Schrittweiten; `range(start, end, step)` liefert die entsprechende Folge.
- **1.0.3:** Die Standardbibliothek wurde für Mathematik, Text, Arrays, Sets, Maps, Datum, Parsing, Encodings und Pfade erweitert. Hinzu kommen typgeprüfte `|>`-Pipelines, flache `else if`-Kaskaden, Schemaanzeige bei nicht erreichbarer Datenbank, automatische Tutorialeinrichtung, freie Ports, wiederholbare Beispieldaten und begrenzte Rekursion. Die vollständigen Signaturen stehen in [Anhang D](#anhang-d-die-standardbibliothek-im-uberblick-zelyra-103).
- **1.0.4:** Rechnungstutorials erhalten einen eigenen Compose-Projektnamen. Bei älteren Volumes kann der Start auf eine isolierte Datenbank ausweichen, ohne das bisherige Volume zu löschen oder zu verändern. Die Compose-Erkennung akzeptiert Kurzversionen wie `2.40`.
- **1.0.5:** Studio ergänzt lokale strukturierte Logs; `zelyra uninstall` entfernt die aktive benutzereigene Programmdatei nach Bestätigung. Das Rechnungstutorial erhält Datumseingabe ohne Uhrzeit, serverseitig vergebene Rechnungsnummern, eine Ablauf-Startseite, eine benannte Rechnungsübersicht und ausführliche deutsche und englische Quelltexterklärungen.
- **1.0.6:** Zelyra UI stellt compilergeprüfte Komponenten und lokal gebündelte Stile für erzeugtes CRUD, Projektvorlagen, Studio-Eigenschaften und Rechnungstutorial bereit. Dazu kommen Themes, responsive Layouts, Diagrammkomponenten, Browserdruck/PDF und vorausgefüllte E-Mail-Entwürfe.
- **1.0.7:** Wertliefernde `if`-Ausdrücke, funktionale Record-Updates (`Customer { ..old, active: true }`), typisierte lexikalische Lambdas für Pipelines (`map`, `filter`, `find`, `reduce`), verschachtelte Record-Muster und boolesche Match-Guards, First-Class-Tupel als Rückgabewerte und Zuweisungsziele, kontextuelle Feldnamen (`version`, `type`), konfigurierbare Schrittbegrenzung (`--max-steps`), nominale Id-Aliase (`type OrderId = Id`), serverseitiges lokales Text-PDF-Rendering (`render_pdf`) und transaktionale PDF-Mail-Anhänge über die MariaDB-Outbox.
- **1.1.0:** Studio-Verbesserungen mit Typed-Hole-Vorschlägen, Datenkarte für Schemata, statischer Seitenvorschau ohne Codeausführung und Live-Theme-Editor für `zelyra.theme.css`.
- **1.1.1:** CLI- und Editor-Stabilitätsverbesserungen bei JSON-RPC-Diagnosen und Pfadauflösungen.
- **1.1.2:** **Vollständige Entfernung des experimentellen Laravel-Analysators und Gerüstbefehls (`zelyra migrate laravel`).** Der Migrator und zugehörige Berichte wurden aus Zelyra entfernt. Zelyra setzt konsequent auf native, compilergeprüfte Verträge und eigene Schemaplanung statt Code-Portierungen.
- **1.1.5:** Lokale wiederverwendbare Pakete (`pkg:`), Offline-Lockfile (`zelyra.lock`), Paketmanifeste mit deklarierten Effekten, `zelyra module init/list/plan/bundle`, compilergeprüfte Paketeinstellungen und CSV-Beispielmodule.
- **1.2.0:** Betriebs- und Deployment-Grundlagen, deklarative Commands und Zeitpläne, statischer Ordner `public/` mit MIME-Auslieferung in `serve`, Apache-/Nginx-Reverse-Proxy-Konfigurationen und Systemdienst-Installation mit festem Port.
- **1.2.5:** Feste Linux-systemd-Dienste (`zelyra service install/status/restart/change/remove`), residenter Scheduler, Ed25519-signierte Quellpakete mit Registryauflösung und idempotente Outbox-Einrichtung.
- **1.2.6:** Wiederherstellbare Paketänderungen: serialisierte Mutationen, versioniertes Transaktionsjournal und Quarantäne-Rollback via `zelyra package recover`.
- **1.2.7:** Kryptografisch signierte transitive Registry-Pakete mit Trustprüfung der gesamten Closure, zyklusfreier Auflösung und deterministischen Updateplänen.
- **1.2.8:** Typisierte SQL-Query-Pläne von HIR nach MIR mit Re-Verifikation (`E-MIR-VERIFY-004`) und strikte Absicherung von Formular-Redirects auf lokale absolute Pfade (`E-FORM-007`).
- **1.2.9:** Dauerhafte Scheduler-Jobs in der MariaDB-Outbox; Ausführung über serielle Lease-Worker; Job-Statusinspektion über `zelyra schedule jobs`.
- **1.2.10:** Revisionsgesicherte Speicherung von Paketeinstellungen in Zelyra Studio ohne Exposition von Geheimniswerten; HostServices Versionierungsvertrag.
- **1.2.11:** Durchgängige Zelyra-UI-Harmonisierung für das Rechnungstutorial, CLI-SIGPIPE-Stabilität und verbesserte Compose-v2-Paketerkennung.
- **1.2.12:** Authentifizierte Weboberfläche zur Verwaltung von Moduleinstellungen (`/__zelyra/module-settings`), automatische Standardwerte für Zelyra-UI-Dialoge und direkter Theme-Zugriff in Studio.
- **1.2.13:** Kanonische Projektstruktur (`zelyra new --layout standard`) mit verbindlichen Wurzeln `routes/`, `domain/`, `schema/`, `public/` und `assets/` sowie sauberer Namensraumauflösung (`import "domain::sales::actions"`).
- **1.2.14:** Deterministische Migrationsvorschau `zelyra migrate-layout --check`, `strict_architecture = true` Architekturprüfung (`E-ARCH-001`), gestaltete Beispielrechnung und Quelltextdialog im Rechnungstutorial.
- **1.2.15:** Typgeprüfte Erweiterungspunkte für generierte CRUD-Create-/Edit-Seiten (`crud.view.create`, `crud.view.edit`), Feldhilfen und Zeilenaktionen in `tableview`.
- **1.2.16:** Typisierte TableView-Zellinhalte und formatierte Zeilenaktionsslots mit HTML-Vorlagen und Escaping.
- **1.2.17:** Erzwungene Mandantenrelationen (`scoped by tenant_id: TenantId`), automatische zusammengesetzte Foreign Keys `FOREIGN KEY (tenant_id, fk_id) REFERENCES target (tenant_id, id) ON DELETE RESTRICT`, typsichere optionale Relationen (`Customer?`) und exakte kaufmännische Dezimalskalierung (`Decimal(p, s)`).
- **1.2.18 / 1.2.19:** Mandantenbezogene Eindeutigkeit `unique within tenant` mit automatischem zusammengesetztem MariaDB-Unique-Index `UNIQUE (tenant_id, column)`.
- **1.2.20:** Typgeprüfte feldübergreifende Tabelleninvarianten (`validate { condition: "error" }`) in Formularen und CRUD sowie typisierte API-Eingabeprüfung mit `validates`.
- **1.2.21:** Datensatzabhängige Lifecycle-Guards (`guard edit when ...`, `guard delete when ...`) und Policy-Auswertung in Detailansichten und TableView-Slots.
- **1.2.22:** Generierte transaktionale Master-Detail-CRUD-Formulare (`has_many`) mit atomarer Speicherung von Kopf- und Positionsdaten, Kaskadierung und serverseitigen Aggregaten (`aggregates`).
- **1.2.23:** Validierung mandantengebundener APIs (`scoped by tenant_id`) aus geprüften Benutzersitzungen; zuverlässige Mandantentrennung auf API-Ebene.
- **1.2.24:** Unterstützung mehrerer direkter `has_many`-Kinddatensammlungen pro CRUD-Ressource (z. B. Positionen und Anhänge) mit atomarer Speicherung, unabhängigen Zeilenlimits und serverseitigen Aggregaten.
- **Ausblick Meilenstein 1.3.0:** Verbindliches Standard-Layout, Dual-Tier-Pipeline mit Bytecode-VM v24, Single-Binary Bundle (`zelyra build --bundle`), GoBD-Revisionssicherheit (`auditable` mit `as_of`-Zeitreisen) und Hauptbuch-Primitive (`ledger`).

Release-Details: [1.0.1](https://github.com/sf1976/zelyra/blob/main/docs/release-notes/1.0.1.de.md), [1.0.2](https://github.com/sf1976/zelyra/blob/main/docs/release-notes/1.0.2.de.md), [1.0.3](https://github.com/sf1976/zelyra/blob/main/docs/release-notes/1.0.3.de.md), [1.0.4](https://github.com/sf1976/zelyra/blob/main/docs/release-notes/1.0.4.de.md), [1.0.5](https://github.com/sf1976/zelyra/blob/main/docs/release-notes/1.0.5.de.md), [1.0.6](https://github.com/sf1976/zelyra/blob/main/docs/release-notes/1.0.6.de.md) und [1.0.7](https://github.com/sf1976/zelyra/blob/main/docs/release-notes/1.0.7.de.md). Die [Studio-Anleitung](https://github.com/sf1976/zelyra/blob/main/docs/studio.de.md), [Logs-Anleitung](https://github.com/sf1976/zelyra/blob/main/docs/studio-logs.de.md), [Deinstallationsleitfaden](https://github.com/sf1976/zelyra/blob/main/docs/uninstall.de.md), [Mail-Anleitung](https://github.com/sf1976/zelyra/blob/main/docs/mail.de.md) und [Datenbankmatrix](https://github.com/sf1976/zelyra/blob/main/docs/database-support.de.md) beschreiben weitere Funktionen und Grenzen.

### Entwicklungsstand und vereinbarte Architektur

Zelyra befindet sich im stabilen Release **1.2.24** mit aktiver Ausrichtung auf Meilenstein **1.3.0**. Den laufenden Stand hält die [Zelyra-Roadmap](https://github.com/sf1976/zelyra/blob/main/docs/ROADMAP.de.md) fest.

- **Residente Zeitpläne und integrierter Scheduler (✅ Implementiert seit 1.2.5/1.2.9):** Commands und fünfteilige numerische Zeitpläne (`schedule <name> { cron: "..."; timezone: "..."; command: ... }`) werden direkt im Zelyra-Quelltext deklariert. Eine wöchentliche Ausführung samstags um 12:00 Uhr verwendet beispielsweise `cron: "0 12 * * 6"` und `timezone: "Europe/Berlin"`. Der integrierte residente Scheduler läuft automatisch innerhalb des aktiven Zelyra-Dienstes (`zelyra service` bzw. `zelyra serve`). An jeder Minutengrenze wertet er fällige Zeitpläne aus und reiht Jobs transaktional in die MariaDB-Outbox ein. Ein serieller Lease-Worker (`zelyra schedule jobs`) verarbeitet Aufgaben zuverlässig mit automatischer Verlängerung und begrenzten Wiederholungen. Eine manuelle `crontab -e`-Konfiguration auf dem Host ist für den regulären Zelyra-Betrieb **nicht erforderlich**. Der Befehl `zelyra schedule export` dient lediglich als optionale Vorschau oder Export für externe Drittsysteme; Zelyra überschreibt niemals ungefragt fremde Crontabs.
- **Betriebssystemdienste (✅ Implementiert):** `zelyra service install --port <port>` richtet einen systemd-Benutzerdienst auf einem festen Port ein. `zelyra service status`, `restart`, `change --port <port>` und `remove` verwalten den Lebenszyklus mit atomarem Rollback bei Fehlern.
- **Kanonische Projektstruktur und Vertical Slices (✅ Implementiert seit 1.2.13/1.2.14):** `--layout standard` erzwingt eine getrennte Verzeichnisstruktur (`schema/`, `domain/`, `routes/`, statisches `public/`). `strict_architecture = true` prüft Architekturverletzungen (`E-ARCH-001`), und `zelyra migrate-layout --check` bietet eine deterministische Migrationsvorschau.
- **Erweiterbare CRUD- und TableView-Seiten (✅ Implementiert seit 1.2.15/1.2.16):** Generierte Create- und Edit-Seiten stellen typgeprüfte Erweiterungspunkte bereit (`crud.view.create`, `crud.view.edit`), und Tabellenansichten unterstützen Feldhilfen und formatierte Zeilenaktionsslots.
- **Mandantensicherheit und zusammengesetzte Fremdschlüssel (✅ Implementiert seit 1.2.17/1.2.19):** `table ... scoped by tenant_id: TenantId` erzwingt die Mandantentrennung auf Datenbankebene über zusammengesetzte Fremdschlüssel. `unique within tenant` verhindert Kollisionen bei mandanteneigenen Nummernfolgen.
- **Tabelleninvarianten und Validierungsregeln (✅ Implementiert seit 1.2.20):** Typgeprüfte Invarianten auf Tabellenebene (`validate`) werden in Formularen und CRUD vor jedem Schreibvorgang evaluiert.
- **Lifecycle-Guards und Master-Detail-CRUD mit mehreren Kind-Sammlungen (✅ Implementiert seit 1.2.21 – 1.2.24):** GoBD-Sperren für `edit` und `delete` (`guard`), transaktionale Erfassung mehrerer 1:N-Kinddatensammlungen (`has_many`) mit serverseitigen Aggregaten und API-Mandantenvalidierung aus geprüften Sitzungen.
- **Generierte `.env`:** Bei Projektvorlagen enthält `.env` neben aktiven Einstellungen auch auskommentierte Konfigurationsbeispiele. Kommentarzeilen dienen der Dokumentation: Sie werden weder als Shell-Skripte noch von Compose als Befehle interpretiert. Zugangsdaten gehören in die geschützte lokale `.env`, niemals in Git oder Quellcode.
- **Datenbankentwicklung:** Zelyra-Projekte beschreiben Schemata deklarativ über Zelyra-Tabellen. `zelyra db plan`, `apply` und `history` sind die maßgeblichen Werkzeuge; Laravel-Migrationsdateien sind keine Laufzeitvoraussetzung. Der frühere experimentelle Laravel-Migrator wurde aus dem Zelyra-Produkt entfernt.
- **Ausblick Meilenstein 1.3.0:** Dual-Tier-Ausführungsmodell mit Bytecode-VM v24, compiler-erzwungene Mandantensicherheit (`table ... scoped by TenantId`), GoBD-konforme Revisionssicherheit (`auditable` mit `as_of`-Abfragen), Zero-Sum-Hauptbuch-Primitive (`ledger`) und Single-Binary Deployment (`zelyra build --bundle`).

### Zeitpläne und Outbox im Betrieb

In Zelyra-Projekten richtet das System bei der Bereitstellung alle erforderlichen Outbox- und Scheduler-Tabellen idempotent ein. Der residente Scheduler wird automatisch gestartet, sobald das Projekt über den Dienst (`zelyra service`) oder `zelyra serve` betrieben wird.

Folgende Befehle stehen zur Inspektion und Verwaltung bereit:

~~~bash
zelyra check main.zyl
zelyra task list main.zyl
zelyra schedule list main.zyl
zelyra schedule jobs main.zyl
zelyra schedule status main.zyl
~~~

Ein manueller Cron-Eintrag auf dem Host (`crontab -e`) ist für den regulären Betrieb nicht erforderlich. Für den Fall, dass ein bestehendes externes Überwachungssystem oder ein externer Server-Scheduler die Zelyra-Befehle ansteuern soll, kann mit:

~~~bash
zelyra schedule export main.zyl --format cron --output deploy/zelyra.cron
~~~

eine exportierbare Cron-Vorschau erzeugt werden. `zelyra schedule export` überschreibt keine vorhandene Datei und greift nicht in die System-Crontab ein. Die unterstützten Umgebungsvariablen stehen in der [Environment-Referenz](https://github.com/sf1976/zelyra/blob/main/docs/env.md); die detaillierte Scheduler- und Task-Architektur wird in [Kapitel 28](#28-systemd-dienste-scheduler-und-jobs) behandelt.

## Statuszeichen

- ✅ **Implementiert und geprüft:** im dokumentierten Funktionsumfang vorhanden und durch die angegebenen Quellen belegt.
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
  - [Kapitel 13: Sammlungen, Listen und Wörterbücher (Arrays & Maps)](lehrbuch-grundlagen.md#kapitel-13-sammlungen-listen-und-worterbucher-arrays-maps)
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
  - [Kapitel 41: Funktionsumfang und sicherer Betrieb](lehrbuch-anwendungen.md#kapitel-41-funktionsumfang-und-sicherer-betrieb)
  - [Kapitel 42: Dein Weg als Zelyra-Entwickler](lehrbuch-anwendungen.md#kapitel-42-dein-weg-als-zelyra-entwickler)
  - [Kapitel 43: Mit Zelyra Studio arbeiten](lehrbuch-anwendungen.md#kapitel-43-mit-zelyra-studio-arbeiten)

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
- [22. Unterstützung und technische Nachweise](technisches-referenzhandbuch.md#22-unterstutzung-und-technische-nachweise)
- [23. KI-native Entwicklung](technisches-referenzhandbuch.md#23-ki-native-entwicklung)
- [24. Verbindliche Quellen und Compiler-Prüfung (Source Authority)](technisches-referenzhandbuch.md#24-verbindliche-quellen-und-compiler-prufung-source-authority)

- [25. Anwendungs-Mail und SMTP](technisches-referenzhandbuch.md#25-anwendungs-mail-und-smtp)
- [26. Mandantengebundene Benachrichtigungen](technisches-referenzhandbuch.md#26-mandantengebundene-benachrichtigungen)
- [27. Sprache, Laufzeit und PDF-Mail](technisches-referenzhandbuch.md#27-sprache-laufzeit-und-pdf-mail)
- [28. Systemd-Dienste, Scheduler und Jobs](technisches-referenzhandbuch.md#28-systemd-dienste-scheduler-und-jobs)
- [29. Kanonische Projektstruktur und Vertical Slices](technisches-referenzhandbuch.md#29-kanonische-projektstruktur-und-vertical-slices)
- [30. Erweiterbare CRUD- und TableView-Seiten](technisches-referenzhandbuch.md#30-erweiterbare-crud-und-tableview-seiten)
- [31. Dual-Tier-Ausführungsmodell: AST, MIR und Bytecode-VM](technisches-referenzhandbuch.md#31-dual-tier-ausfuhrungsmodell-ast-mir-und-bytecode-vm)
- [32. Mandantensicherheit und zusammengesetzte Fremdschlüssel](technisches-referenzhandbuch.md#32-mandantensicherheit-und-zusammengesetzte-fremdschlussel)
- [33. Tabelleninvarianten und Validierungsregeln](technisches-referenzhandbuch.md#33-tabelleninvarianten-und-validierungsregeln)
- [34. Lifecycle-Guards, Policies und Master-Detail-CRUD](technisches-referenzhandbuch.md#34-lifecycle-guards-policies-und-master-detail-crud)
- [35. Das Zelyra-Ökosystem: Pakete, Module und Einstellungen](technisches-referenzhandbuch.md#35-das-zelyra-okosystem-pakete-module-und-einstellungen)
- [36. CSS-Designsystem und Oberflächengestaltung für CRUD und TableViews](technisches-referenzhandbuch.md#36-css-designsystem-und-oberflachengestaltung-fur-crud-und-tableviews)

### Anhänge

- [Anhang A: Schnelleinstieg / Spickzettel (Syntax-Cheat-Sheet)](anhaenge.md#anhang-a-schnelleinstieg-spickzettel-syntax-cheat-sheet)
- [Anhang B: Alle Fehlermeldungen von Zelyra auf einen Blick](anhaenge.md#anhang-b-alle-fehlermeldungen-von-zelyra-auf-einen-blick)
- [Anhang C: Zelyra-CLI-Referenz](anhaenge.md#anhang-c-zelyra-cli-referenz)
- [Anhang D: Die Standardbibliothek im Überblick](anhaenge.md#anhang-d-die-standardbibliothek-im-uberblick-zelyra-103)
- [Anhang E: SQL-Spickzettel für Zelyra-Entwickler](anhaenge.md#anhang-e-sql-spickzettel-fur-zelyra-entwickler)
- [Anhang F: HTML- und Web-Referenz in Zelyra](anhaenge.md#anhang-f-html-und-web-referenz-in-zelyra)
- [Anhang G: Glossar der Fachbegriffe](anhaenge.md#anhang-g-glossar-der-fachbegriffe)
- [Anhang H: Lösungen zu den Übungsaufgaben der Kapitel](anhaenge.md#anhang-h-losungen-zu-den-ubungsaufgaben-der-kapitel)
- [Anhang I: Häufige Fragen und Antworten (FAQ)](anhaenge.md#anhang-i-haufige-fragen-und-antworten-faq)
- [Anhang J: Weiterführende Ressourcen und Community](anhaenge.md#anhang-j-weiterfuhrende-ressourcen-und-community)
- [Anhang K: Das Zelyra-Cookbook (Praxisrezepte)](anhaenge.md#anhang-k-das-zelyra-cookbook-praxisrezepte)
