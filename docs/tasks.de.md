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
unveröffentlichten 1.2.5-Arbeitsstand erkennen `zelyra setup --schema`,
`--all` und `zelyra serve` geprüfte Zeitpläne aus `main.zyl`, erzeugen
`.zelyra/schedules.cron` und richten für geplante `Email`-Handler die
MariaDB-Outbox idempotent ein. Geänderte Deklarationen aktualisieren nur den
Zelyra-eigenen Export; wenn keine Zeitpläne mehr deklariert sind, wird genau
dieser verwaltete Export entfernt. Fremde Dateien und Symlinks bleiben
geschützt.
`serve` erstellt zusätzlich `.zelyra/serve.json` mit Loopback-Adresse und
zufälligem Sitzungstoken (Verzeichnis `0700`, Datei `0600` auf Unix). Der
exportierte Cron-Aufruf führt `zelyra schedule trigger` aus: Dieser prüft mit
einem authentifizierten lokalen Aufruf, ob genau dieses Projekt gerade über
den installierten und aktiven Zelyra-Systemdienst läuft. Manuell gestartetes
`serve` reicht nicht. Ohne aktiven Dienst oder dessen laufenden Server wird der
Job übersprungen. Bei positivem Nachweis startet Zelyra den compilergeprüften
Command-Handler als Kindprozess.
Der Zeitplan wird weiterhin durch einen ausdrücklich installierten Host-Cron
ausgelöst; Zelyra bearbeitet die Benutzer-Crontab nicht stillschweigend. Ein
bereits manuell installierter Host-Cron-Eintrag wird durch Löschen der
Deklaration nicht aus der Crontab entfernt; sein Trigger scheitert dann an der
erneuten Prüfung der aktuellen Quelldatei und führt den Handler nicht aus.
Ein direkt in `serve` eingebauter Scheduler ist das Ziel für die nächste
Scheduler-Etappe und soll externe Cron-Einträge überflüssig machen. Bis dahin
gibt es keine Garantie gegen doppelte Ausführungen. Geheimnisse stehen weder
im Cron-Eintrag noch in der Kommandozeile;
`zelyra run` liest die Einstellungen aus der Projekt-`.env`.

Ein Schedule mit `cron` verwendet fünf numerische Felder für Minute, Stunde,
Monatstag, Monat und Wochentag. Unterstützt werden Zahlen, `*`, Listen,
Bereiche und `/Schritt`; Monats- und Wochentagsnamen sind derzeit nicht
unterstützt. Das Feld- und Wertebereichsformat wird compilergeprüft. Die
konkrete Ausführungssemantik bleibt Aufgabe des Host-Schedulers. Ohne
Zeitzonenangabe verwendet der Export UTC. Der Compiler prüft die Form des
Zeitzonennamens, aber nicht, ob das Zielsystem diesen Eintrag kennt.

`schedule trigger` akzeptiert ausschließlich deklarierte Zeitplannamen und
führt nichts aus, wenn der authentifizierte Servernachweis fehlt. Anschließend
startet es Zelyra erneut mit dem deklarierten Handler. Dadurch gelten die
normalen Projektprüfungen, Runtime-Budgets und Capability-Freigaben. Secrets
gehören nicht in Command-Zeilen oder Cron-Ausgaben.
