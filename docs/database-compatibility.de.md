# MariaDB-Kompatibilitätsmatrix

**Umfang:** Datenbank- und CRUD-Laufzeitpfade für Zelyra 0.2.0 | **Matrix geprüft:** 20.09.2026 | **Nachweis:** exakte offizielle MariaDB-Docker-Tags; Linux x86_64

Diese Matrix dokumentiert getestetes Zelyra-Verhalten. Sie ist weder eine
MariaDB-Zertifizierung noch ein Versprechen, dass jede SQL-Funktion auf jedem
Server funktioniert, noch ein MySQL-Kompatibilitätsnachweis. MariaDB ist
Zelyras primäre Laufzeitreferenz. PostgreSQL-Laufzeitparität und MySQL Server
sind nicht Teil dieser Matrix.

## Getestete Versionen

| MariaDB-Community-LTS-Reihe | Exakter Test-Image-Tag | Zelyra-Teststatus | Community-Wartung bis* |
|---|---|---|---|
| 10.11 | `mariadb:10.11.19` | Lokal geprüft; exakter Tag in der CI-Matrix | 16.02.2028 |
| 11.4 | `mariadb:11.4.13` | Lokal geprüft; exakter Tag in der CI-Matrix | 29.05.2029 |
| 11.8 | `mariadb:11.8.9` | Lokal geprüft; exakter Tag in der CI-Matrix | 04.06.2028 |
| 12.3 | `mariadb:12.3.3` | Lokal geprüft; exakter Tag in der CI-Matrix | 12.06.2029 |

Der GitHub-Actions-Job `mariadb-compatibility` führt dieselben zentralen
Integrationstests mit jedem exakten Image-Tag aus. Er prüft die vom Server
gemeldete Version und protokolliert die aufgelösten Image-Digests. Ein
erfolgreicher Job ist erforderlich, bevor eine Änderung am Datenbank-
Laufzeitpfad als über die Matrix hinweg geprüft gilt. Wenn die MariaDB
Foundation neuere Wartungsreleases veröffentlicht, sind die Patch-Tags zu
aktualisieren und die Matrix vor einem Release erneut auszuführen.

Die aktuellen Reihen und Wartungsdaten sind eine datierte Momentaufnahme der
[Wartungsrichtlinie der MariaDB Foundation](https://mariadb.org/about/#maintenance-policy).
Die [Wartungsankündigung für Q3 2026](https://mariadb.org/mariadb-server-12-3-11-8-11-4-and-10-11-q3-2026-maintenance-releases-and-goodbye-10-6/)
führt die hier verwendeten Patch-Releases auf. Die exakten Tags werden im
[offiziellen MariaDB-Docker-Image](https://hub.docker.com/_/mariadb)
veröffentlicht.

## Was die Matrix testet

Die CI-Matrix führt diese Integrationen für jedes aufgeführte Image aus. Lokal
können dieselben Skripte gegen einen wegwerfbaren MariaDB-Dienst laufen:

- `tests/mariadb-e2e.sh`: Schema-Setup, Inspektion, idempotente Planung,
  beziehungsbasiertes CRUD über HTTP sowie Suche, Filter, Sortierung und
  Paginierung.
- `tests/schema-safety-e2e.sh`: Destruktive Spalten-Drops benötigen Freigabe;
  Pflichtspalten ohne Standardwert, neue Unique-Constraints sowie MariaDB-
  Foreign-Key-Ergänzungen und -Entfernungen erscheinen als `REVIEW`; doppelte
  Zeilen bleiben bei einem abgelehnten Unique-Index erhalten, verwaiste Zeilen
  bei einem abgelehnten Foreign-Key nicht verloren; nicht unterstützte
  Nullbarkeitsänderungen sowie SQLite-Typ-, Foreign-Key- und
  Unique-Constraint-Änderungen werden auch mit Freigabe durch `E-DB-006`
  blockiert.

Jeder CI-Matrixjob besitzt einen eigenen kurzlebigen MariaDB-Dienst und eine
eigene Datenbank. Der Schema-Sicherheitstest erstellt und entfernt nur eine
eindeutig benannte Testdatenbank. Der Sicherheitstest prüft außerdem, dass
MariaDBs Verhalten beim Hinzufügen einer Pflichtspalte ohne Standardwert nie
ohne ausdrückliche Freigabe ausgelöst wird; nach Freigabe müssen vorhandene
Werte geprüft werden, bevor Anwendungscode sich darauf verlässt. Zelyra-Indizes
werden anhand ihrer generierten Namen als verwaltet erkannt; unbekannte Indizes
bleiben erhalten. `--allow-destructive` genehmigt keine `REVIEW`-Änderungen;
dafür ist nach Prüfung des Plans `--allow-risky` erforderlich.

## Was damit nicht nachgewiesen wird

- Kompatibilität mit MySQL Server. Die Annahme einer `mysql://`-URL bedeutet
  nicht, dass MySQL Teil dieser Testmatrix ist.
- Unterstützung anderer MariaDB-Versionen oder nicht aufgeführter
  Patch-Releases.
- PostgreSQL- oder SQL-Server-Laufzeitparität.
- Kompatibilität jeder MariaDB-spezifischen SQL-Funktion, von Plugins, Galera,
  Replikation, Failover, Produktions-Backup/Wiederherstellung oder Upgrades
  bestehender Produktionsdaten.
- Leistungs-, Kapazitäts- oder Produktionshärtungsgarantien.

Für Abnahmetests zusätzlich die in Produktion eingesetzte Datenbankversion
verwenden. Vor einem MariaDB-Upgrade oder einer Änderung an Zelyras Schema- und
Laufzeitverhalten ein verifiziertes Backup erstellen und den resultierenden
Plan mit einer wegwerfbaren Kopie des echten Schemas und repräsentativen Daten
prüfen.
