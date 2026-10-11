# Datenbankmatrix

Diese Matrix nennt konkrete Server und die Workflows, die Zelyras CI gegen
diese Server ausführt. Ein Eintrag bedeutet keine vollständige SQL- oder
Runtime-Parität. Die CI-Matrix steht in
[`ci.yml`](../.github/workflows/ci.yml). Der veröffentlichte 0.9.0-Kandidat
bestand seine Prüfungen am exakten Stand; Meilenstein 0.9.7 erweitert die
Matrix auf die für 1.0.0 zugesagten Abläufe.

| Server | Geprüfte Abläufe | Grenzen des Nachweises |
| --- | --- | --- |
| MariaDB 10.11.19, 11.4.13, 11.8.9, 12.3.3 | Studio-Datenbankansicht und CRUD-Assistent; lokales TOTP-MFA; Parameter, Poolgrenzen und Statement-Timeouts; TLS; Datenbank-/CRUD-HTTP-E2E; Schema-Sicherheitsprüfung; Backup und Wiederherstellung; Handbuchbeispiele | MFA bleibt eine ausdrücklich konfigurierte Vorschau. CI ersetzt weder ein unabhängiges Sicherheitsreview noch die Produktabnahme. Nicht jeder optionale DB-Treiberpfad wird auf jeder Version geprüft. |
| MySQL Server 8.4.11 | Parametrisierte Zelyra-SQL-Abfragen gegen den echten MySQL-Server | Begrenzter Kompatibilitätsnachweis; kein Nachweis für Studio, vollständiges CRUD, Authentifizierung, MFA, Schema-Setup, TLS oder Backup/Restore. Keine vollständige MariaDB-Parität behaupten. |
| PostgreSQL 16 | Studio-Introspektion, parametrisierte Runtime-Abfragen und Schema-Sicherheitsprüfung | Kein Nachweis für vollständiges CRUD, Authentifizierung, MFA, TLS, Backup/Restore oder allgemeine SQL-Portabilität. |
| SQLite (Systemversion des CI-Images) | Schema-Integration, Parametrisierung und grundlegende CRUD-Läufe | Lokale Entwicklungsoption; kein Ersatz für die geprüften Serverdatenbanken und kein 1.0-Produktionsversprechen. |

## Konsequenz für Zelyra 1.0.0

MariaDB, MySQL und PostgreSQL bleiben Ziele für die erste produktive Version.
Der aktuelle 0.9-Nachweis ist je Backend unterschiedlich. Die Releases 0.9.3
und 0.9.7 ergänzen und prüfen die benötigten Produktabläufe und dokumentieren
ihre Einschränkungen; 0.9.9 darf den 1.0.0-Supportumfang erst danach einfrieren.
Die bloße Annahme einer URL wie `mysql://` ist kein Nachweis für MySQL Server.
