# Zelyra-Sprachkonsole

`zelyra console [entry.zyl]` startet eine interaktive AST-REPL. Ohne Dateiangabe wird `main.zyl` verwendet. Die Datei wird wie bei `zelyra check` vollständig geladen und geprüft; ihre Deklarationen und Funktionen stehen der Sitzung zur Verfügung.

Ein ausführbares Beispiel liegt unter [examples/language_console.zyl](../examples/language_console.zyl).

```sh
zelyra console main.zyl                 # Sandbox ist Standard
zelyra console main.zyl --persist       # Datenbankänderungen dauerhaft übernehmen
zelyra console main.zyl --tenant 42
```

Eingaben werden einzeln geparst, typgeprüft und ausgeführt. Erfolgreiche Bindungen bleiben erhalten:

```text
>> x = 10
>> y = 20
>> x + y
=> 30
```

Im interaktiven Terminal bietet Rustyline ANSI-Farben für Schlüsselwörter, Zeichenketten und Zahlen, Verlauf in `~/.zelyra_history`, Tab-Vervollständigung für Projektfunktionen, Tabellen, Records, Felder und häufige Standardfunktionen sowie `>>`/`...`-Prompts. `NO_COLOR` und `ZELYRA_COLOR=never` schalten Farben ab. Bei Pipe-Eingabe werden die Prompts ausgelassen. Eingaben mit Anführungszeichen, SQL oder erkennbaren Geheimnisbegriffen werden nicht in der Historie gespeichert. Syntax-, Typ- und Laufzeitfehler melden eine Diagnose; die nächste Eingabe kann danach fortgesetzt werden.

## Befehle

- `:help` zeigt die Befehle.
- `:tables` und `:schema <tabelle>` zeigen deklarierte Tabellen und Spalten.
- `:tenant <id>` setzt die REPL-Bindung `tenant_id`; dies erzeugt keine zusätzliche Autorisierung oder automatische Filterung.
- `:commit` bestätigt die laufende Sandbox-Transaktion und öffnet eine neue.
- `:rollback` verwirft die laufende Sandbox-Transaktion und öffnet eine neue.
- `:clear`, `:quit` und `:exit` leeren beziehungsweise beenden die Sitzung.

## Datenbanksicherheit und Grenzen

Standardmäßig hält die Sitzung für MariaDB/MySQL eine echte Transaktion auf einer einzelnen Verbindung. Jede Eingabe hat zusätzlich einen Savepoint: Schlägt eine Eingabe zur Laufzeit fehl, werden nur deren Datenbankänderungen zurückgenommen. Beim Beenden wird die noch offene Transaktion zurückgerollt. `:commit` ist eine ausdrückliche Ausnahme: Bereits bestätigte Änderungen bleiben bestehen; spätere Änderungen werden beim Beenden weiterhin zurückgerollt. `--persist` verwendet den normalen Datenbankpfad ohne Sandbox-Transaktion.

Die Sandbox-Transaktion wird nur gestartet, wenn eine Projektverbindung aus Prozessumgebung oder `.env` konfiguriert ist. Ohne Verbindung sind datenbankfreie Ausdrücke nutzbar; Datenbankzugriff scheitert wie bei normaler Runtime-Ausführung. PostgreSQL-Sandbox-Transaktionen und `--backend bytecode` sind noch nicht verfügbar. `--tenant` stellt den Wert als `tenant_id` im Scope bereit, führt aber keine Datenbank-Row-Level-Security ein. Capabilities, Projekt-Runtime-Policy, Compilerprüfung und SQL-Parameterbindung bleiben wirksam.

## Verhalten bei mehrzeiliger Eingabe

Die Fortsetzung wird anhand nicht abgeschlossener Zeichenketten und geschweifter Blöcke erkannt. Nicht abgeschlossene Eingabe am Ende einer Pipe wird als Parsefehler gemeldet. `Ctrl+C` verwirft nur den aktuellen Eingabepuffer; `Ctrl+D` beziehungsweise `:quit` beendet die Sitzung.
