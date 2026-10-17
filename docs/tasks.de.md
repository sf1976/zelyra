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
installieren und aktivieren keinen Scheduler. `CRON_TZ` wird in der Vorschau ausgegeben, aber
Cron-Implementierungen unterscheiden sich bei Zeitzonen, Sommerzeit und
Umgebungsvariablen; diese Eigenschaften müssen auf dem Zielsystem geprüft
werden. Für produktive Deployments sind auch systemd-Timer oder Plattform-
Scheduler mögliche Hosts. Zelyra startet standardmäßig keinen residenten
Scheduler.

Dies beschreibt das veröffentlichte Verhalten von 1.1.5. Im aktuellen,
unveröffentlichten 1.2.5-Arbeitsstand prüfen `zelyra setup --schema`,
`--all` und `zelyra serve` die Zeitpläne aus `main.zyl` und richten für
geplante `Email`-Handler die MariaDB-Outbox idempotent ein. Das Setup erzeugt
keine Cron-Zeilen; alte, von Zelyra markierte Dateien
`.zelyra/schedules.cron` werden entfernt. Fremde Dateien und Symlinks bleiben
geschützt. Der installierte und aktive Linux-systemd-Userdienst startet den
Scheduler im selben `serve`-Prozess. Er liest und prüft das Projekt an jeder
Minutengrenze neu; entfernte, geänderte oder gerade ungültige Deklarationen
werden deshalb nicht aus einem alten Startzustand ausgeführt. Manuell
gestartetes `serve` führt keine Zeitpläne aus. Zeitpläne folgen ihrer IANA-
Zeitzone; ohne Angabe gilt UTC. Höchstens vier unterschiedliche Schedule-
Handler laufen gleichzeitig; ein weiterer Lauf desselben Handlers wird
übersprungen, solange er noch aktiv ist. Fehler und Programmausgabe gehen an
stdout/stderr des Dienstes (bei systemd typischerweise ins Journal).

Die Cron-Ausgabe ist in diesem Arbeitsstand nur eine prüfbare Vorschau; sie richtet
keine externe Ausführung ein. Deklarierte Zeitpläne laufen ausschließlich im
installierten, aktiven Linux-systemd-Dienst.
Zelyra installiert oder bearbeitet keine Benutzer-Crontab. Frühere manuelle
Crontab-Zeilen kann Zelyra nicht aus der fremdverwalteten Crontab löschen; der
Legacy-Trigger wird im aktiven Dienst übersprungen, damit keine doppelte
Ausführung entsteht. Der Scheduler holt während eines Dienstausfalls verpasste
Minuten nicht nach und bietet noch keine dauerhafte verteilte Lease- oder
Retry-Garantie; das gehört in die nächste Jobsystem-Etappe. `zelyra run` liest
die Einstellungen weiterhin aus der Projekt-`.env`.

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
