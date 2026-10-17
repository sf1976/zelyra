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
unveröffentlichten 1.2.5-Arbeitsstand erkennen `zelyra setup --schema` und
`--all` geprüfte Zeitpläne aus `main.zyl`, erzeugen einmalig
`.zelyra/schedules.cron` und richten die MariaDB-Outbox ein, wenn ein
geplanter Handler die Capability `Email` deklariert. Symlinks für den
Schedulerstatus oder den Export werden abgewiesen; Zelyra-eigene Exporte
werden nach geänderten Deklarationen aktualisiert, fremde Dateien nicht
überschrieben. Dieser getestete Vorbereitungsschritt aktiviert aber
weder einen Host-Cronjob noch führt er Zeitpläne in `serve` aus. Als nächster
1.2.5-Schritt folgt die koordinierte Ausführung über `serve`. Der generierte
Befehl enthält keine Geheimnisse; `zelyra run` liest Datenbank- und
Maileinstellungen aus der geschützten lokalen `.env`. Zelyra ändert die
Crontab des Benutzers nicht stillschweigend.

Ein Schedule mit `cron` verwendet fünf numerische Felder für Minute, Stunde,
Monatstag, Monat und Wochentag. Unterstützt werden Zahlen, `*`, Listen,
Bereiche und `/Schritt`; Monats- und Wochentagsnamen sind derzeit nicht
unterstützt. Das Feld- und Wertebereichsformat wird compilergeprüft. Die
konkrete Ausführungssemantik bleibt Aufgabe des Host-Schedulers. Ohne
Zeitzonenangabe verwendet der Export UTC. Der Compiler prüft die Form des
Zeitzonennamens, aber nicht, ob das Zielsystem diesen Eintrag kennt.

`task run` startet Zelyra erneut mit dem deklarierten Handler. Dadurch gelten
die normalen Projektprüfungen, Runtime-Budgets und Capability-Freigaben.
Secrets gehören nicht in Command-Zeilen oder Cron-Ausgaben.
