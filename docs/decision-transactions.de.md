# Begrenzte native Entscheidungstransaktionen

Stand: unveröffentlichter 0.9.9-Entwicklungsbranch. [English](decision-transactions.en.md)

Die Bootstrap-Datenbankschicht bietet `with_decision_transaction(url, closure)`
und ein geliehenes `DecisionTransaction::execute(Query)`. Bedingte Lese-/Schreib-
Schritte der Closure verwenden dieselbe Verbindung. `Ok` committet; Fehler,
abgelaufene Request-Deadline oder Unwind lösen Rollback aus. Das Handle kann die
Closure nicht verlassen. Interne native API, keine neue Zelyra-Syntax oder volle
Backend-Parität.

Ein Ausführungsfehler sperrt die Transaktion auch dann, wenn die Closure ihn
ignoriert; weitere Statements und Commit werden verweigert. SELECT/INSERT/
UPDATE/DELETE werden akzeptiert. Transaktionssteuerung und implizit committende
DDL werden abgewiesen. Bestehende erzeugte MariaDB-/MySQL-Audit-Initialisierung
mit Sessionvariable bleibt ausdrücklich erhalten. SQL-Bindung und exakte
Ergebnis-/NULL-Codecs werden mit normalen nativen Abfragen geteilt. Anwendungs-
SQL benötigt weiterhin die Compiler-/Schema-/Capability-Prüfungen.

PostgreSQL verwendet die Scoped-Transaktion des Treibers und vor jeder Operation
`SET LOCAL statement_timeout` aus konfiguriertem Timeout und verbleibendem
Request-Budget sowie eine frische Deadline-Prüfung vor Commit. Lokale Timeouts
verlassen die Transaktion nicht. Poolzugriff behält TLS-/Settings-/UTC-Regeln.
Gewöhnliche MariaDB-Closure-Transaktionen führen nun auch bei Unwind/Commitfehler
Rollback aus und verwerfen die Verbindung. Kein automatischer Statement-Retry.
Verbindungs-/Commitfehler bleiben Fehler; der extern beobachtete Commit-Ausgang
ist dadurch nicht immer sicher bestimmbar.

Deadline-Prüfungen vor Statements/Commit belegen keine harten Abbruchgrenzen für
alle MySQL-Schreiboperationen. Bestehende SELECT-Timer-/Treibergrenzen gelten;
das eigene MySQL-Schreibgrenzen-Gate bleibt offen. Geschützte PostgreSQL-Formulare
(Mandant, Empfänger, Edit-Snapshot, Audit) benötigen weiterhin Adapter-/Guard-
Integration. Auth-/MFA-/Benachrichtigungsparität und verkettete PostgreSQL-Audits
werden mit dieser Grundlage nicht belegt. Kein bestehender Guard wird entfernt.

Keine neue Abhängigkeit, Umgebungsvariable, Migration, Telemetrie, Anbieterbindung
oder Websiteänderung. `DATABASE_URL`, Timeout-/TLS-Einstellungen und Poolsteuerung
bleiben die bisherigen. [Native Nachweise](release-readiness/0.9.9-decision-transactions.de.md)
benennen echte Drei-Backend-Commit-/Rollback-/Deadline-Proben mit Ein-Verbindungs-Pool.
