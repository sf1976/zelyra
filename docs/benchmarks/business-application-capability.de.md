# Funktionsabgleich mit einer realen Geschäftsanwendung

**Stand:** 23. Oktober 2026  
**Referenz:** `/opt/siedelmann-laravel/app`

Dieser Abgleich prüft die fachlichen Fähigkeiten von Zelyra. Er ist keine
Laravel-Portierung: Laravel-Code, Architektur und Views sind keine
Implementierungsvorgabe. Bewertet werden Abläufe, Datenbeziehungen,
Berechtigungen, Validierungen, Nebenwirkungen und gespeicherte Ergebnisse,
die mit Zelyras Sprache, Laufzeit und Datenbankkonzepten umgesetzt werden.

## Erster Abgleich

| Geschäftsablauf | Zelyra-Nachweis | Ergebnis |
| --- | --- | --- |
| Kunden, Rechnungen, Positionen, verbundene Daten und berechnete Summen | `examples/invoice_tutorial/`; MariaDB-Webprüfung in `tests/invoice-tutorial-e2e.sh` | Im Beispiel abgebildet. Der Nachweis deckt nicht alle Buchhaltungsregeln der Referenzanwendung ab. |
| Mandantengeschützte CRUDs und Berechtigungen | `examples/tenant_crud.zyl`; `tests/mariadb-tenant-crud-e2e.sh` | Der deklarierte CRUD- und Isolationstest ist Teil der CI. |
| Gesicherte Aktionen, Audit-Einträge, atomare Transaktionen und PDF-Mail | `examples/typed_crud_actions/main.zyl`; `tests/mariadb-typed-crud-actions-e2e.sh` | Der Test prüft Berechtigung, Schutz vor Wiederholung, Rollback und den PDF-Anhang in der Outbox. |
| Abmeldung und Auswahl zulässiger Kampagnenempfänger | Referenz: `/opt/siedelmann-laravel/app/app/Http/Controllers/App/CsvMailerController.php` und `tests/Feature/CsvMailerImmediateSendingTest.php`. Zelyra: `examples/csv_mailer_unsubscribe.zyl`; `tests/mariadb-form-route-e2e.sh`. | Abmeldung vorhanden; das E2E-Skript wird mit dieser Änderung an die MariaDB-CI angebunden. Ein separater Versuch, Empfänger über mandantengeschützte Tabellen auszuwählen, wird mit `E-TENANT-003` zurückgewiesen. Kunden ohne E-Mail, Abmeldungen, Kampagnenausschlüsse und Tenant-Isolation sind noch nicht als vollständiger Ablauf nachgewiesen. |
| Faktur-X-XML zusätzlich zum Rechnungs-PDF versenden | Outbox-Vertrag in `docs/mail.de.md`; Referenz unter `/opt/siedelmann-laravel/app/app/Services/DocumentMailService.php` | **Nicht unterstützt:** Zelyra-Mailanhänge sind derzeit auf validierte PDF-Dateien begrenzt. Aufnahme als Abnahmekriterium für 1.7.0. |
| Terminplanung, Verfügbarkeitsregeln, Rückruf-/Nachbearbeitungsfälle, Banken, DATEV/ELSTER und GoBD-Stornoabläufe | Referenzcontroller, Services und bestehende Feature-/Smoke-Tests; noch kein entsprechender Zelyra-Gesamtworkflow | **Nicht bewertet.** Vor einer Aussage über Unterstützung muss jeder Ablauf als Zelyra-Anwendung reproduziert und geprüft werden. |

Die bestehende CI enthält bereits funktionale Prüfungen für wichtige Teilbereiche
wie Rechnungen, mandantengeschützte CRUDs und transaktionale PDF-Mail. Eine
erfolgreiche Typprüfung allein gilt nicht als Funktionsnachweis. Für jeden
weiteren Ablauf sind mindestens ein ausführbares Zelyra-Beispiel und passende
positive sowie negative Integrationstests erforderlich.

### Durchgeführte Prüfungen

Auf dem Quellstand `5ebe066a` (Codebasis `1.2.55`) wurden
`examples/invoice_tutorial/main.zyl`, `examples/tenant_crud.zyl`,
`examples/typed_crud_actions/main.zyl`,
`examples/csv_mailer_unsubscribe.zyl`,
`examples/pdf_mail_outbox/main.zyl` und
`examples/customer_orders_modules/main.zyl` mit dem daraus gebauten CLI-Binary
erfolgreich durch `zelyra check` geprüft. Zusätzlich bestanden die beiden
gezielten Laufzeittests
`pdf_bytes_are_accepted_as_pure_transactional_outbox_attachments` und
`mail_queue_creates_a_validated_outbox_row_inside_one_database_batch` in
`zelyra-runtime`. Der erste Test deckt auch ab, dass beliebige Bytes nicht als
PDF-Anhang zugelassen werden. Die MariaDB-Webtests laufen in CI; die lokale
Laravel-Produktivdatenbank wurde nicht angesprochen.

Ein zusätzlicher Kompilierungsversuch für die Kampagnenempfängerauswahl
scheiterte erwartungsgemäß mit `E-TENANT-003`, weil die Abfrage verbundene
mandantengeschützte Tabellen per benutzerdefiniertem SQL liest. Der Compiler
weist diese Form zurück, statt einen vom Aufrufer gelieferten Tenant-Wert als
vertrauenswürdig zu behandeln. Dadurch ist die Lücke konkret reproduzierbar.

## Wiederholbarer Prüfablauf

1. Einen konkreten Geschäftsablauf aus der Referenz auswählen und gewünschtes
   Verhalten, Datenfluss, Berechtigungen und Fehlerfälle festhalten.
2. Den Ablauf mit Zelyra-Konzepten modellieren; keine Laravel-Schichten oder
   Views nachzeichnen.
3. Das vollständige Zelyra-Projekt kompilieren und gegen eine isolierte,
   wegwerfbare Datenbank ausführen.
4. Erfolgsfall, ungültige Eingaben, Mandantengrenzen, Wiederholungen,
   Rollback und tatsächlich gespeicherte Daten prüfen, soweit sie zum Ablauf
   gehören.
5. Den Nachweis hier aktualisieren und reproduzierbare Regressionen in die CI
   übernehmen. Kleine Fähigkeitslücken werden direkt behoben; größere
   Sprach-, Laufzeit- oder Architekturdefizite erhalten einen Roadmap-Eintrag
   mit Beleg und vorgeschlagenem Release.

Die Laravel-Arbeitskopie kann ungesicherte Änderungen enthalten. Sie wird bei
diesem Abgleich nur als lesbare fachliche Referenz verwendet. Tests gegen ihre
Produktivdatenbank oder Änderungen an der Anwendung sind kein Teil dieses
Prüfablaufs.
