# Commands und Zeitpläne in Zelyra

Commands und Zeitpläne sind Deklarationen in der Zelyra-Quelldatei. Der
Compiler prüft Command-Namen, Handler, Schedule-Referenzen und die begrenzte
numerische Fünf-Feld-Cron-Syntax.

```zelyra
command process_recurring_invoices {
    handler: run_recurring_invoices
    description: "Wiederkehrende Rechnungen erzeugen"
}

schedule recurring_invoices {
    command: process_recurring_invoices
    cron: "0 4 * * 1-5"
    timezone: "Europe/Berlin"
}

fn run_recurring_invoices() {
    // Fachliche Arbeit; Datenbank- und andere Effekte bleiben capabilitygeprüft.
}

fn main() { }
```

Ein Schedule darf nur auf einen Command mit parameterlosem Handler zeigen.
Commands können Handlerargumente besitzen. `task run --args` übergibt sie als
JSON-Array; Zelyra prüft sie gegen die Funktionsparameter, bevor der normale
Runtime-Aufruf erfolgt.

```sh
zelyra check main.zyl
zelyra task list main.zyl
zelyra task list main.zyl --json
zelyra task run main.zyl process_recurring_invoices
zelyra schedule list main.zyl --json
zelyra schedule export main.zyl --format cron
zelyra schedule export main.zyl --format cron --output deploy/zelyra.cron
```

Ohne `--output` schreibt der Export eine Cron-Vorschau nach stdout. Mit
`--output` wird die Vorschau als neue, projektinterne Datei geschrieben; der
Export überschreibt keine vorhandene Datei und folgt keinen Symlinks aus dem
Projekt heraus. Das Zielverzeichnis muss bereits vorhanden sein. Beide Formen
installieren oder aktivieren keinen Host-Cronjob. `CRON_TZ` wird in der
Vorschau ausgegeben, aber Cron-Implementierungen unterscheiden sich bei
Zeitzonen, Sommerzeit und Umgebungsvariablen; diese Eigenschaften müssen auf
dem Zielsystem geprüft werden. Zelyras eigener residenter Scheduler läuft
ausschließlich im installierten und aktiven Linux-systemd-Benutzerdienst des
Projekts. Ein manuell gestartetes `zelyra serve` führt deklarierte Zeitpläne
nicht aus. Für externe Deployments bleiben systemd-Timer oder Plattform-
Scheduler mögliche Alternativen; die Cron-Vorschau muss dort bewusst geprüft
und aktiviert werden.

Dieser Abschnitt beschreibt den mit 1.2.9 eingeführten dauerhaften Scheduler-
Implementierung. `zelyra setup --all` legt die MariaDB-Outbox für jeden
deklarierten Schedule wiederholbar an oder aktualisiert sie. Dafür ist eine
MariaDB-URL des Projekts erforderlich. Die Outbox speichert weder Handlercode
noch Zugangsdaten. Alte, von Zelyra markierte Dateien
`.zelyra/schedules.cron` werden erst nach erfolgreicher Datenbankprüfung
entfernt; fremde Dateien und Symlinks bleiben geschützt.

Nur der installierte und aktive Linux-systemd-Userdienst startet den residenten
Scheduler. Er prüft den aktuellen Projektquelltext an jeder Minutengrenze und
fügt fällige Vorkommen mit einer deterministischen ID aus Projekt, Schedule
und UTC-Minute ein. Wiederholte Ticks und Neustarts erzeugen keine Duplikate.
Ein einzelner dauerhafter Worker verarbeitet die Vorkommen nacheinander, prüft
den aktuellen Schedule vor dem Start erneut, erneuert seine Datenbank-Lease und
beachtet `scheduler_job_timeout_seconds`. Entfernte, geänderte oder ungültige
Deklarationen werden abgebrochen, statt gespeicherten Handlertext auszuführen.
Fehler werden mit begrenzten Wiederholungen und einem Erschöpft-Status in der
Outbox behandelt. Manuell gestartetes `serve` führt keine Zeitpläne aus.
Programmausgabe und Workerfehler stehen in stdout/stderr des Dienstes
(typischerweise im systemd-Journal).

Die Cron-Ausgabe ist nur eine prüfbare Vorschau; sie richtet keine externe
Ausführung ein. Zelyra installiert oder bearbeitet keine Benutzer-Crontab und
kann fremdverwaltete Cronzeilen nicht entfernen. Der Legacy-Trigger führt keine
Jobs aus. Während eines Dienstausfalls verpasste Minuten werden nicht
nachgeholt. Die Zustellung erfolgt mindestens einmal: Wenn ein Handler eine
externe Nebenwirkung ausführt und der Prozess vor der Bestätigung abstürzt, kann
ein Retry die Nebenwirkung wiederholen. Verwende idempotente Handler oder eine
eigene Anwendungstransaktion. `zelyra run` liest Einstellungen weiterhin aus
der Projekt-`.env`.

`zelyra schedule status main.zyl [--json]` zeigt die compilergeprüften
Zeitpläne, den Installations-/Aktivzustand des Projekt-Dienstes und den daraus
ermittelten Schedulerzustand. Es startet oder verändert nichts. Die JSON-
Ausgabe ist Version 1 und enthält `platform_service_supported`,
`declared_schedule_count`, `service_installed`, `service_active`,
`scheduler_active` und die Zeitplanmetadaten. Auf nicht unterstützten
Plattformen wird der Dienst als nicht verfügbar und der Scheduler als inaktiv
gemeldet.

`zelyra schedule jobs main.zyl [--limit 50] [--offset 0] [--json]` zeigt eine
begrenzte Seite dauerhafter Schedule-Metadaten ohne Job-Payloads. JSON verwendet
`schema_version: 1`. Ein erschöpfter Job kann nach Prüfung ausdrücklich mit
`zelyra outbox requeue <event-id> main.zyl` freigegeben werden; dadurch wird er
nicht sofort ausgeführt.

Ein Schedule mit `cron` verwendet fünf numerische Felder für Minute, Stunde,
Monatstag, Monat und Wochentag. Unterstützt werden Zahlen, `*`, Listen,
Bereiche und `/Schritt`; Monats- und Wochentagsnamen sind derzeit nicht
unterstützt. Das Feld- und Wertebereichsformat wird compilergeprüft. Der
integrierte Scheduler verwendet die üblichen Cron-Feldbereiche; sind sowohl
Monatstag als auch Wochentag eingeschränkt, genügt eine der beiden
Übereinstimmungen. Ohne Zeitzonenangabe gilt UTC. Die IANA-Zeitzone wird beim
Start des Dienst-Schedulers tatsächlich aufgelöst; unbekannte Zonen verhindern
dessen Start.

`schedule trigger` führt keine Zeitpläne aus. Er verwirft alte exportierte
Cron-Aufrufe und gibt bei aktivem Projekt-Dienst ausdrücklich aus, dass der
integrierte Scheduler die Deklarationen verwaltet. Ohne aktiven Dienst bleiben
Zeitpläne ebenfalls inaktiv. Secrets gehören
nicht in Command-Zeilen oder Cron-Ausgaben.
