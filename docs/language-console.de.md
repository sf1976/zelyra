# Zelyra-Sprachkonsole

`zelyra console [entry.zyl]` startet eine interaktive REPL. Ohne Dateiangabe wird `main.zyl` verwendet. Die Datei wird wie bei `zelyra check` vollständig geladen und geprüft; ihre Deklarationen und Funktionen stehen der Sitzung zur Verfügung. AST ist der Standard; `--backend bytecode` wählt die experimentelle Bytecode-VM für deren unterstützte Teilmenge. Nicht unterstützte Programme werden mit einer Backenddiagnose abgewiesen. Beide Backends verwenden denselben Parser, Typprüfer, persistenten Scope und dieselben Runtime-Policies.

Ein ausführbares Beispiel liegt unter [examples/language_console.zyl](../examples/language_console.zyl).

```sh
zelyra console main.zyl                 # Sandbox ist Standard
zelyra console main.zyl --persist       # Datenbankänderungen dauerhaft übernehmen
zelyra console main.zyl --tenant 42
zelyra console main.zyl --backend bytecode
```

Eingaben werden einzeln geparst, typgeprüft und ausgeführt. Erfolgreiche Bindungen bleiben erhalten:

```text
>> x = 10
>> y = 20
>> x + y
=> 30
```

Datensätze werden strukturiert mehrzeilig dargestellt. Decimal-Werte behalten im Pretty-Printer eine sichtbare Typmarkierung, zum Beispiel `gross_amount: 119 [Decimal]`; die Anzeige ergänzt keine nicht gespeicherte Nachkommastellenskala.

Im interaktiven Terminal bietet Reedline ANSI-Farben für Schlüsselwörter, Zeichenketten und Zahlen, Verlauf in `~/.zelyra_history`, Tab-Vervollständigung für Projektfunktionen, Tabellen, Records, Felder und häufige Standardfunktionen sowie `>>`/`...`-Prompts. `NO_COLOR` und `ZELYRA_COLOR=never` schalten Farben ab. Bei Pipe-Eingabe werden die Prompts ausgelassen. Eingaben mit Anführungszeichen, SQL oder erkennbaren Geheimnisbegriffen werden nicht in der Historie gespeichert. Syntax-, Typ- und Laufzeitfehler melden eine zweizeilige Diagnose mit Code sowie Quellposition und Meldung; die nächste Eingabe kann danach fortgesetzt werden.

## Befehle

- `:help` zeigt die Befehle.
- `:tables` listet deklarierte Tabellen auf. `:schema <tabelle>` zeigt Spalten, Typen, Schlüssel-/Pflichtangaben und deklarierte Invarianten mit Quellposition und Fehlermeldung.
- `:tenant <id>` setzt `tenant_id` und `session.tenant_id`. Das typisierte Feld `session.tenant_id` hat den Typ `Option<Int>` (`None`, solange keine Mandanten-ID gesetzt ist); dies erzeugt keine zusätzliche Autorisierung oder automatische Filterung.
- `:commit` bestätigt die laufende Sandbox-Transaktion und öffnet eine neue.
- `:rollback` verwirft die laufende Sandbox-Transaktion und öffnet eine neue.
- `:clear`, `:quit` und `:exit` leeren beziehungsweise beenden die Sitzung.

## Datenbanksicherheit und Grenzen

Standardmäßig hält die Sitzung für MariaDB/MySQL eine echte Transaktion auf einer einzelnen Verbindung. Jede Eingabe hat zusätzlich einen Savepoint: Schlägt eine Eingabe zur Laufzeit fehl, werden nur deren Datenbankänderungen zurückgenommen. In der Sandbox sind nur `SELECT`, `INSERT`, `UPDATE` und `DELETE` erlaubt. DDL wie `CREATE`, `ALTER`, `DROP` sowie Transaktions- und Sitzungsbefehle werden abgewiesen, weil MariaDB/MySQL sie teilweise implizit festschreiben kann. Beim Beenden wird die noch offene Transaktion zurückgerollt. `:commit` ist eine ausdrückliche Ausnahme: Bereits bestätigte Änderungen bleiben bestehen; spätere Änderungen werden beim Beenden weiterhin zurückgerollt. `--persist` verwendet den normalen Datenbankpfad ohne Sandbox-Transaktion.

Kann Zelyra einen Eingabe-Savepoint nicht sicher anlegen, freigeben oder zurückrollen, wird die Sitzung für weitere Ausdrücke gesperrt. Beende die Konsole, damit die äußere Sandbox-Transaktion zurückgerollt und die betroffene Verbindung nötigenfalls verworfen wird. Zelyra führt nach einem unklaren Rollback-Zustand keine weiteren SQL-Eingaben aus.

Die Sandbox-Transaktion wird nur gestartet, wenn eine Projektverbindung aus Prozessumgebung oder `.env` konfiguriert ist. Ohne Verbindung sind datenbankfreie Ausdrücke nutzbar; Datenbankzugriff scheitert wie bei normaler Runtime-Ausführung. SQL in der Konsole benötigt derzeit den aggregierten Capability-Grant `Database` (in `zelyra.toml`: `database = true` oder zusammen `database_read = true` und `database_write = true`). Ein reiner Lese-Grant erlaubt aktuell keine Datenbankabfragen aus der REPL; die Projektgrants werden nicht erweitert. AST- und Bytecode-Sandbox-SQL verwenden dieselbe MariaDB/MySQL-Verbindung und dieselben Eingabe-Savepoints. PostgreSQL-Sandbox-Transaktionen werden nicht unterstützt. `--tenant` stellt den Wert als `tenant_id` im Scope bereit, führt aber keine Datenbank-Row-Level-Security ein. Capabilities, Projekt-Runtime-Policy, Compilerprüfung und SQL-Parameterbindung bleiben wirksam.

Der MariaDB-Integrationstest `tests/language-console-mariadb-e2e.sh` erstellt eine isolierte Wegwerf-Datenbank und prüft implizites Rollback, `:rollback`, `:commit` sowie `--persist`. Er benötigt Docker und das MariaDB-Image `mariadb:11.8` (oder ein über `ZELYRA_REPL_MARIADB_IMAGE` angegebenes kompatibles Image).

## Verhalten bei mehrzeiliger Eingabe

Die Fortsetzung wird anhand nicht abgeschlossener Zeichenketten sowie geschweifter, eckiger und runder Klammern erkannt. Zeichen in Zeichenketten und einzeiligen Kommentaren zählen nicht als Klammern. Nicht abgeschlossene Eingabe am Ende einer Pipe wird als Parsefehler gemeldet. `Ctrl+C` verwirft nur den aktuellen Eingabepuffer; `Ctrl+D` beziehungsweise `:quit` beendet die Sitzung.
