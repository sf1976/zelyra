# Datenbankmatrix

Diese Matrix nennt konkrete Server und die Workflows, die Zelyras CI gegen
diese Server ausführt. Ein Eintrag bedeutet keine vollständige SQL- oder
Runtime-Parität. Die CI-Matrix steht in
[`ci.yml`](../.github/workflows/ci.yml). Diese Tabelle nennt den derzeit
geprüften Umfang und seine Grenzen je Backend.

| Server | Geprüfte Abläufe | Grenzen des Nachweises |
| --- | --- | --- |
| MariaDB 10.11.19, 11.4.13, 11.8.9, 12.3.3 | Studio-Datenbankansicht und CRUD-Assistent; lokales TOTP-MFA; Parameter, Poolgrenzen und Statement-Timeouts; TLS; Datenbank-/CRUD-HTTP-E2E; relationales Mandanten-CRUD mit echten Fremdschlüsseln und Waisenabweisung (E-DB-005); Schema-Sicherheitsprüfung; Backup und Wiederherstellung; Handbuchbeispiele | MFA bleibt eine ausdrücklich konfigurierte Vorschau. CI ersetzt weder ein unabhängiges Sicherheitsreview noch die Produktabnahme. Nicht jeder optionale DB-Treiberpfad wird auf jeder Version geprüft. |
| MySQL Server 8.4.11 | Parametrisierte Zelyra-SQL-Abfragen; Schema-Setup, Inspektion, schreibgeschützte Projektkarte sowie geprüfte sichere Spaltenmigration mit Historie; relationales Mandanten-CRUD mit echten Fremdschlüsseln, Waisenabweisung (E-DB-005) und gehärtete Schreiblock-Wartezeitgrenzen (`innodb_lock_wait_timeout`) | Begrenzter Kompatibilitätsnachweis. Kein Nachweis für Authentifizierung außerhalb relationaler CRUD-Sessions, MFA, TLS oder Backup/Wiederherstellung. Die Schema-E2E deckt additive sichere Spaltenänderung ab; keine vollständige MariaDB-Parität behaupten. |
| PostgreSQL 16.15 | Studio-Introspektion, parametrisierte Runtime-Abfragen, Schema-Sicherheitsprüfung, native Formularentscheidungen mit unverkettetem Audit sowie relationales Mandanten-CRUD mit echten Fremdschlüsseln und Waisenabweisung (E-DB-005) | Kein Nachweis für vollständiges Authentifizierungs-Backend, MFA, TLS, Backup/Restore oder allgemeine SQL-Portabilität. Verkettetes Audit bleibt gesperrt. CI pinnt die Minor-Version und prüft sie vor den Integrationsschritten. |
| SQLite (Systemversion des CI-Images) | Schema-Integration, Parametrisierung und grundlegende CRUD-Läufe | Lokale Entwicklungsoption; kein Ersatz für die geprüften Serverdatenbanken und kein 1.0-Produktionsversprechen. |

## Konsequenz für Zelyra 1.0.0

MariaDB, MySQL und PostgreSQL bleiben Ziele für die erste produktive Version.
Der geprüfte Umfang unterscheidet sich je Backend. 0.9.9 darf den
1.0.0-Supportumfang erst einfrieren, wenn jeder zugesagte Ablauf belegt ist.
Die bloße Annahme einer URL wie `mysql://` ist kein Nachweis für MySQL Server.
Für die Weiterentwicklung nach 1.0.0 ist eine modulare Adapter-Architektur
(`DatabaseAdapter` SPI) sowie Multi-Datenbank-Unterstützung (Einbindung mehrerer
getrennter Datenquellen pro Anwendung) in der Roadmap verankert.

## Zusätzlich lokal geprüft: exakte Dezimalgrenzen (0.9.9-Entwicklung)

MariaDB 11.4.13, MySQL 8.4.11 und PostgreSQL 16.15 bestehen getrennte
Rundläufe für Decimal-/Money-Formulare, SQL-Speicherung, präzise CRUD-Filter,
HTTP-/generische JSON-Daten, optionale NULL-Werte und das Abweisen alter Skalen.
Die neuen CI-Schritte führen dieselbe Probe aus.
Siehe [Paketnachweis](release-readiness/0.9.9-exact-decimals.de.md).
Einfache PostgreSQL-Formularaktionen sind transaktional; Mandanten-/Empfänger-,
Edit-Snapshot- und Audit-Entscheidungen sind weiterhin gesonderte technische
Gates. Der MySQL-Server-Zeittimer betrifft SELECT; harte Schreib-Abbruchgrenzen
sind damit nicht nachgewiesen. Es gibt keine allgemeine Backend-Freigabe.

## Native Entscheidungsgrundlage (0.9.9-Entwicklung)

Eine Scoped-Callback-API besteht Commit, bedingte Lese-/Schreibsichtbarkeit,
Rollback bei Rückgabe-/SQL-/ignorierten Fehlern und Unwind, Deadline-Prüfung vor
Statements/Commit und Wiederverwendung mit Ein-Verbindungs-Pool auf MariaDB
11.4.13, MySQL 8.4.11 und PostgreSQL 16.15. PostgreSQL-Statementabbruch ist geprüft.
Keine Freigabe für allgemeine Auth-Adapterparität oder harte MySQL-
Schreibgrenzen. [Regeln](decision-transactions.de.md),
[Nachweise](release-readiness/0.9.9-decision-transactions.de.md).

## PostgreSQL-Formularentscheidungen (0.9.9-Entwicklung)

Formularentscheidungen für PostgreSQL verbinden Mandantenmitgliedschaft,
Empfängerberechtigung, Zeilensperrung, optimistische Snapshotvergleiche
(NULL vs. `\N`), Geschäftsänderung und atomare unverkettete Audits innerhalb
einer einzigen Transaktion und Verbindung. Manipulierte Mandanten-/Fremdschlüssel,
Mitgliedschaftsentzug während Lock-Wait (Zwei-Verbindungs-Rennen) und Audit-
Fehler rollen die Transaktion atomar zurück. Verkettetes PostgreSQL-Audit
bleibt gesperrt und wird mit klarer Diagnose abgewiesen.
Siehe [Nachweise](release-readiness/0.9.9-postgres-form-decisions.de.md).

## Datenbankmatrix, Transaktionen und relationales CRUD (0.9.9-Entwicklung)

Auf MariaDB 11.4.13, MySQL 8.4.11 und PostgreSQL 16.15 ist der relationale
CRUD-Kreislauf mit echten Fremdschlüssel-Constraints nachgewiesen:
- Migrations-Preflight mit Waisenprüfung (`error[E-DB-005]`) bewahrt Daten und
  verhindert unzulässige Constraint-Erzeugung.
- Dynamische Fremdschlüssel-Optionen (`load_relation_options`) filtern strikt
  nach aktiver Mandantenmitgliedschaft.
- Manipulierte Fremdschlüssel fremder Mandanten liefern HTTP 422 Unprocessable Entity.
- Relationale Join-Darstellung rendert Elterndisplay-Werte statt roher IDs.
- Restriktives Löschen blockiert Löschungen referenzierter Datensätze sicher mit
  HTTP 500 ohne Internaleck; geordnetes Löschen (Kind vor Eltern) gelingt.
- Auf MySQL und MariaDB erzwingt `innodb_lock_wait_timeout` harte, konfigurierte
  Abbruchgrenzen für Zeilensperren (~1 s statt 50 s).
Siehe [Nachweise](release-readiness/0.9.9-relational-crud-matrix.de.md).
