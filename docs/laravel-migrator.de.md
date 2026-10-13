# Laravel-Migrator

Der Laravel-Migrator erstellt zunächst einen statischen Analysebericht. Das
Quellprojekt, seine Datenbank und seine Umgebung werden nicht verändert.

```sh
zelyra migrate laravel ./altes-projekt --report ./berichte/migration.md
zelyra migrate laravel ./altes-projekt --report ./berichte/migration.json --format json
```

Der Berichtspfad muss außerhalb des Quellprojekts liegen, sein Elternordner
muss bereits vorhanden sein, und die Zieldatei darf noch nicht existieren.
Zelyra führt weder PHP noch Artisan aus, öffnet keine Datenbankverbindung und
liest keine `.env`-Dateien. Symbolische Links, `vendor`, `node_modules`,
`storage` und `bootstrap/cache` werden ausgelassen. Die Analyse ist auf 10.000
PHP-Dateien und 2 MiB pro Datei begrenzt.

## Erkennungsumfang und Regelwerk

Die Analyse erkennt Laravel-Versionsvorgaben aus `composer.json` (Laravel 10.x und 11.x),
relevante Paketabhängigkeiten (`spatie/*`, `stripe/stripe-php`, `inertiajs/*`, `livewire/*`),
einfache `Route`-Deklarationen, Eloquent-Modelle, Controllerklassen, `Schema::create`,
Beziehungen (`belongsTo`, `hasMany`, `hasOne`, etc.), Validierungs-Einstiegspunkte,
`ShouldQueue`-Jobs, Autorisierungsprüfungen (`authorize`, `Gate`, Policies),
Mandantengrenzen (`tenant_id`, `account_id`, etc.), Geldberechnungen und -aggregate,
Dateioperationen (`store`, `download`), Zahlungs- und Billingaufrufe (`charge`, `checkout`, `subscription`),
syntaktische und dynamische Mehrdeutigkeiten (`DB::raw`, `whereRaw`, `call_user_func`),
direkte Umgebungszugriffe (`env(...)`), Integrationsaufrufe (Mail, Notification, HTTP, Storage),
Datenbanktransaktionen (`DB::transaction`) sowie Blade-Views.
Jedes Ergebnis enthält Datei, Zeile, Status, Beschreibung und nächsten Prüfschritt. Der JSON-Bericht
ist für nachgelagerte Werkzeuge gedacht; der Markdown-Bericht lässt sich direkt lesen.

Analyzer- und Regelwerksversion liegen in Zelyra 0.9.9 einheitlich auf `0.9.9`.
Die 18 Befundkategorien dokumentieren Prüfhinweise; sie entscheiden nicht, ob eine
Kontrolle vollständig oder korrekt ist. Kein Befund wird als vollständig unterstützt (`supported`)
deklariert; alle Ergebnisse werden ehrlich als `manual` oder `partial` ausgewiesen.

## Sicherheits- und Ausgabegrenzen

1. **Reine Leseoperation (Immutabilität):** Das Quellprojekt, seine Dateien und seine Umgebung
   bleiben vollständig unverändert.
2. **Pfadisolierung:** Berichtsdateien und Gerüst-Zielverzeichnisse müssen außerhalb des
   Quellprojekts liegen; Pfadsprünge (`..`) und existierende Zielordner/Dateien werden verweigert,
   um versehentliches Überschreiben auszuschließen.
3. **Geheimnisfreiheit:** `.env`-Dateien, `APP_KEY`, Passwörter und Tokens werden weder
   ausgelesen noch in Berichte oder Gerüste übernommen. Direkte `env(...)`-Aufrufe erfassen nur
   den Variablennamen ohne den Umgebungswert.
4. **Separates Gerüst:** Ein mit `--output` angefordertes Gerüst erzeugt ein eigenständiges,
   mit `zelyra check` prüfbares Projekt (`zelyra.toml`, `main.zyl`, `README.md`, `MIGRATION_REPORT.md`,
   `MIGRATION_TODO.md`), enthält jedoch ausdrücklich keine übertragene Geschäftslogik.

## Grenzen und akzeptierte Zuordnungen

Der Scanner ist muster- und zeilenbasiert und kein PHP-Parser. Er verknüpft noch keinen
vollständigen Graphen aus Route, Middleware, Controller, Modell, Migration und View.
Erkannte Syntax beweist weder funktionale noch sicherheitstechnische Parität.

Akzeptierte Zuordnungen (Modelle/Migrationen mit Fremdschlüsseln und `Decimal`, relationale
CRUD-Routen mit Mandantentrennung, CSRF, Validierung, relationalem Join und restriktivem
Löschschutz) sind in Zelyra als lauffähige Projekte auf MariaDB 11.4.13, MySQL 8.4.11 und
PostgreSQL 16.15 belegt (`tests/laravel-migrator-business-e2e.py`).
Zudem ist die Kapselung modularer Geschäftsanwendungen mit eigener Datenbankkonfiguration
und Docker-Export über `zelyra module bundle` End-to-End nachgewiesen.
