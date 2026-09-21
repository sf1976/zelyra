# Zelyra 0.3.0 security-boundary review / Sicherheitsgrenzen-Review

**Status:** internal release-boundary review, not an external audit. The
repository is still version `0.2.0`; this document supports the 0.3.0
milestone decision and does not approve a release.

## English

### Scope and evidence

This review covers the boundaries that a 0.3.0 claim could otherwise overstate:

- generated CRUD/forms/layouts, HTML escaping, CSRF, same-origin and host
  checks, authentication, permissions, and protected machine-management paths;
- SQL parameter binding, schema-safety planning, destructive/review gates,
  database error redaction, and isolated MariaDB/SQLite/PostgreSQL checks;
- installer behavior, release checksums, archive path safety, update checks,
  and the separation between source/release installation.

The evidence is repository source inspection, focused unit tests, generated
application E2E tests, the internal 0.2.0 review, and the full green [CI run
35585277742](https://github.com/sf1976/zelyra/actions/runs/35585277742). The
protected machine-management acceptance path specifically checks anonymous
denial, viewer access, create/edit denial, authorized CRUD, search, and
deletion. No independent security audit, penetration test, threat-model review
by an external party, or production deployment review has been performed.

### Boundary decisions

1. **Browser and authorization boundary — accepted for the experimental
   scope.** Generated writes remain CSRF- and same-origin-protected; host
   allowlists reject forged or rebinding hosts; protected CRUD and API routes
   require their declared authentication and permissions. Custom layouts and
   forms retain escaping, validation, CSRF, and authorization checks in the
   tested paths.
2. **Database boundary — accepted for the documented scope.** Query values are
   bound separately from SQL. Destructive and unsupported schema changes fail
   closed; review-required changes and nullability preflights require explicit
   handling. MariaDB is the primary runtime reference; SQLite and PostgreSQL
   evidence must not be presented as full MariaDB/PostgreSQL runtime parity.
3. **Secrets and installation boundary — accepted for the documented scope.**
   Generated `.env` files are protected, credentials are redacted from tested
   diagnostics, release sidecars are verified, and archive paths are checked.
   Install/update operations replace only the intended user-local executable;
   application data is not migrated automatically.
4. **Deployment boundary — explicitly limited.** Zelyra does not terminate
   TLS. A trusted TLS proxy, explicit host configuration, and protected network
   access are deployment responsibilities. Docker-group membership grants
   effectively root-equivalent privileges and must not be presented as a
   harmless default.

### Residual risks and release rule

- CSRF state remains process-scoped, so origin and host validation are required
  controls rather than optional defense in depth.
- Login throttling is process-local and is not a distributed rate limiter.
- Native release artifacts cover Linux and Windows x86_64 only; other targets
  use the source installer.
- The tested database matrix is not a compatibility claim for MySQL Server,
  arbitrary MariaDB patch levels, PostgreSQL runtime behavior, or production
  backup/restore and upgrade operations.
- The product remains experimental and is not production-approved.

Any failed required CI, schema-safety, authorization, secret-redaction,
artifact, or published-candidate smoke test is a release blocker. Passing this
review does not override the remaining candidate-tag and published-artifact
gates.

## Deutsch

### Umfang und Nachweise

Dieses Review betrachtet Grenzen, die eine 0.3.0-Aussage sonst überschreiten
könnte:

- erzeugte CRUDs/Formulare/Layouts, HTML-Escaping, CSRF, Same-Origin- und
  Host-Prüfung, Authentifizierung, Berechtigungen und geschützte
  Maschinenverwaltung;
- SQL-Parameterbindung, Schema-Sicherheitsplanung, Destruktiv-/Review-Gates,
  Bereinigungen von Datenbankfehlern sowie isolierte MariaDB-/SQLite-/PostgreSQL-
  Prüfungen;
- Installer, Release-Prüfsummen, sichere Archivpfade, Update-Checks und die
  Trennung von Quellcode- und Release-Installation.

Die Nachweise sind Quellcodeprüfung, fokussierte Unit-Tests, Generated-
Application-E2E-Tests, das interne 0.2.0-Review und der vollständige grüne
[CI-Lauf 35585277742](https://github.com/sf1976/zelyra/actions/runs/35585277742).
Der geschützte Maschinenverwaltungstest prüft ausdrücklich anonyme Ablehnung,
Viewer-Zugriff, Ablehnung von Erstellen/Bearbeiten, autorisiertes CRUD, Suche
und Löschen. Ein unabhängiges Sicherheitsaudit, Penetrationstest, externes
Threat-Model-Review oder Review eines Produktionsdeployments wurde nicht
durchgeführt.

### Grenzentscheidungen

1. **Browser- und Berechtigungsgrenze — für den experimentellen Umfang
   akzeptiert.** Erzeugte Schreibvorgänge bleiben CSRF- und
   Same-Origin-geschützt; Host-Allowlists lehnen gefälschte oder per Rebinding
   manipulierte Hosts ab; geschützte CRUD-/API-Routen verlangen ihre erklärten
   Authentifizierungs- und Berechtigungsnachweise. Getestete eigene Layouts und
   Formulare behalten Escaping, Validierung, CSRF und Autorisierung.
2. **Datenbankgrenze — für den dokumentierten Umfang akzeptiert.** Querywerte
   werden getrennt vom SQL gebunden. Destruktive und nicht unterstützte
   Schemaänderungen werden fail-closed abgelehnt; prüfpflichtige Änderungen
   und Nullbarkeits-Vorprüfungen benötigen ausdrückliche Behandlung. MariaDB
   ist die primäre Laufzeitreferenz; SQLite- und PostgreSQL-Nachweise dürfen
   nicht als vollständige MariaDB-/PostgreSQL-Runtime-Parität erscheinen.
3. **Secrets- und Installationsgrenze — für den dokumentierten Umfang
   akzeptiert.** Erzeugte `.env`-Dateien sind geschützt, Zugangsdaten werden in
   geprüften Diagnosen bereinigt, Sidecars werden geprüft und Archivpfade
   validiert. Installation/Update ersetzt nur die beabsichtigte benutzerlokale
   Binärdatei; Anwendungsdaten werden nicht automatisch migriert.
4. **Deploymentgrenze — ausdrücklich eingeschränkt.** Zelyra terminiert TLS
   nicht. Ein vertrauenswürdiger TLS-Proxy, explizite Hostkonfiguration und
   geschützter Netzwerkzugriff bleiben Betriebsverantwortung. Die
   Docker-Gruppenmitgliedschaft verleiht praktisch root-äquivalente Rechte und
   darf nicht als harmlose Standardmaßnahme erscheinen.

### Restgefahren und Release-Regel

- Der CSRF-Zustand gilt weiterhin pro Prozess; Origin- und Hostprüfung sind
  daher notwendige Kontrollen und keine optionale zusätzliche Absicherung.
- Die Login-Drosselung ist prozesslokal und kein verteilter Rate-Limiter.
- Native Releaseartefakte decken nur Linux und Windows x86_64 ab; andere Ziele
  verwenden den Quellcode-Installer.
- Die getestete Datenbankmatrix ist keine Kompatibilitätsaussage für MySQL
  Server, beliebige MariaDB-Patchstände, PostgreSQL-Runtimeverhalten oder
  produktive Backup-/Restore- und Upgradevorgänge.
- Das Produkt bleibt experimentell und ist nicht für Produktion freigegeben.

Jeder fehlgeschlagene erforderliche CI-, Schema-Sicherheits-,
Autorisierungs-, Secret-Redaktions-, Artefakt- oder Published-Candidate-
Smoke-Test ist ein Release-Blocker. Dieses Review hebt die noch offenen
Kandidaten- und Published-Artifact-Gates nicht auf.
