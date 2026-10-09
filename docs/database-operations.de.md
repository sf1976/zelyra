# MariaDB-Backup und Wiederherstellung

Diese Anleitung beschreibt einen vom Betreiber ausgeführten Backup- und
Restore-Ablauf. `zelyra db plan` und `zelyra db apply` erstellen **kein**
Backup; eine bereits ausgeführte DDL-Änderung lässt sich nicht allgemein
zurückrollen. Ein Migrationsplan ersetzt keine unabhängige, geprüfte
Datenbanksicherung.

## Vor einer Schemaänderung

1. Prüfe Zielserver, Datenbankname und SQL-Plan. Gib `DATABASE_URL` nicht in
   Diagnoseausgaben oder Screenshots aus.
2. Verwende ein eigenes Backup-Konto mit den dafür nötigen MariaDB-Rechten.
   Nutze weder das Zelyra-Anwendungskonto noch `root`, wenn ein eingeschränktes
   Konto genügt.
3. Bewahre die Sicherung verschlüsselt und getrennt vom Datenbankserver auf;
   lege Aufbewahrungsdauer und Zugriffsschutz fest.
4. Stelle sie zuerst auf einer isolierten, entbehrlichen MariaDB wieder her
   und prüfe Schema sowie repräsentative Daten.
5. Prüfe den Zelyra-Plan auf dieser Wegwerfkopie, bevor du ihn für das
   eigentliche Ziel freigibst.

## Sicherung erstellen

Das Beispiel setzt MariaDB unter `127.0.0.1:3307`, die Datenbank
`adressverwaltung` und ein eingerichtetes Konto `zelyra_backup` voraus. Bei
Docker-Projekten ist `3307` nur dann der Host-Port, wenn Compose ihn so
veröffentlicht; verwende andernfalls den konfigurierten Host-Port. Das Passwort
wird interaktiv abgefragt und steht nicht im Befehl.

```bash
set -eu
umask 077
mkdir -p backups
backup_file="backups/adressverwaltung-$(date -u +%Y%m%dT%H%M%SZ)-$$.sql"
temporary_file="${backup_file}.partial"
trap 'rm -f -- "$temporary_file"' EXIT

mariadb-dump \
    --host=127.0.0.1 \
    --port=3307 \
    --user=zelyra_backup \
    --password \
    --single-transaction \
    --skip-lock-tables \
    adressverwaltung > "$temporary_file"

chmod 600 "$temporary_file"
mv -- "$temporary_file" "$backup_file"
sha256sum "$backup_file"
```

Die Sicherung wird erst nach erfolgreichem Dump unter ihrem endgültigen Namen
abgelegt. Ein abgebrochener Dump hinterlässt daher keine Datei, die wie ein
fertiges Backup aussieht.

`--single-transaction` liefert einen Snapshot für transaktionale InnoDB-
Tabellen, sofern während des Dumps keine Schemaänderungen stattfinden. Das ist
keine allgemeine Konsistenzgarantie für nichttransaktionale Tabellen, externe
Dateien oder andere Datenbanken. Routinen und Events werden nur gesichert,
wenn die nötigen Optionen und Berechtigungen bewusst ergänzt wurden.

## Wiederherstellung auf einer Wegwerf-Datenbank

Verwende dafür möglichst einen isolierten MariaDB-Server, nicht Produktion.
Prüfe Host und Port vor jedem Schritt. Ein Datenbankadministrator legt das
leere Testziel an; zum Einspielen verwendest du ein getrenntes Konto, dessen
Rechte auf diese Wegwerf-Datenbank beschränkt sind:

```bash
mariadb \
    --host=127.0.0.1 \
    --port=3307 \
    --user=zelyra_restore_admin \
    --password \
    --database=mysql \
    --execute='CREATE DATABASE zelyra_restore_check CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci'
```

> **Achtung:** Der nächste Befehl verändert `zelyra_restore_check`. Führe ihn
> nur aus, wenn Host, Port und Datenbank sicher auf das entbehrliche
> Restore-Testziel zeigen.

```bash
mariadb \
    --host=127.0.0.1 \
    --port=3307 \
    --user=zelyra_restore \
    --password \
    --database=zelyra_restore_check < "$backup_file"

mariadb \
    --host=127.0.0.1 \
    --port=3307 \
    --user=zelyra_restore \
    --password \
    --database=zelyra_restore_check \
    --execute='SHOW TABLES'
```

Prüfe zusätzlich erwartete Tabellen, Schlüssel, Zeilenanzahlen und
repräsentative Fachdaten. Ein erfolgreicher Client-Exit allein beweist keine
vollständige Wiederherstellung der Anwendung. Entferne die Testdatenbank erst
nach Abschluss der Prüfung und erneuter Kontrolle des Ziels.

## Freigabe, Unterbrechung und Grenzen

- `zelyra db plan <datei.zyl> --format=json` gibt einen versionierten
  `zelyra.schema-plan/v1`-Plan mit SHA-256-Fingerprints des beobachteten und
  gewünschten Schemas, stabiler Plan-ID, Drift, SQL-Schritten, Vorprüfungen
  und Freigabebedarf aus. Die Datenbank-URL wird nicht ausgegeben.
- Der JSON-Plan erzeugt noch keinen Datenbank-Migrationsverlauf und kein
  sicheres inverses SQL. `rollback.generated` bleibt `false`; sichere
  Wiederherstellung benötigt weiterhin ein geprüftes Betreiber-Backup.
- `zelyra db plan` zeigt den erkannten Schemaunterschied; es sichert keine
  Daten und reserviert den Datenbankzustand nicht.
- `zelyra db apply` führt unterstützte SQL-Schritte aus. PostgreSQL-Pläne
  laufen in einer einzigen Transaktion. SQLite startet mit `BEGIN IMMEDIATE`,
  bricht beim ersten SQL-Fehler ab und rollt ohne Commit alle Schritte zurück.
  Integrationstests prüfen, dass ein Fehler
  vorherige DDL-Schritte zurücksetzt und ein reparierter Plan erneut
  ausgeführt werden kann. MariaDB-DDL kann implizit Transaktionen abschließen;
  ein späterer Fehler kann bereits ausgeführte Schritte zurücklassen.
- Prüfe nach einem Abbruch den tatsächlichen Datenbankzustand erneut. Verlasse
  dich nicht darauf, dass ein alter Plan einen Teilzustand sicher repariert.
- `--allow-risky` und `--allow-destructive` sind keine Backup- oder
  Rollback-Optionen. Nutze sie nur nach Planprüfung und mit einer unabhängig
  verifizierten Sicherung.
- Zelyra bietet keine automatische Produktions-Backup-, Restore- oder
  Rollback-Garantie. Aufbewahrung, Verschlüsselung, Wiederherstellungszeit und
  regelmäßige Restore-Proben bleiben Verantwortung des Betreibers.

Siehe auch [Datenbankkompatibilität](database-compatibility.de.md) und den
[0.5.0-Plan für sichere Datenbankabläufe](release-plans/0.5.0.de.md).
