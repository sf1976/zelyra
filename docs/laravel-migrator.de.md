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

## Erster Erkennungsumfang

Die frühe Analyse erkennt Laravel-Versionsvorgaben aus `composer.json`,
einfache `Route`-Deklarationen, Eloquent-Modelle, Controllerklassen,
`Schema::create`, einige Beziehungen, Validierungs-Einstiegspunkte,
`ShouldQueue`-Jobs, Blade-Views und Aufrufe ausgewählter Mail-, HTTP-, Datei-
und Transaktions-APIs. Jedes Ergebnis enthält Datei, Zeile, Status,
Beschreibung und nächsten Prüfschritt. Der JSON-Bericht ist für weitere
Werkzeuge gedacht; der Markdown-Bericht lässt sich direkt lesen.

## Grenzen

Der Scanner ist muster- und zeilenbasiert und kein PHP-Parser. Er verknüpft
noch keinen vollständigen Graphen aus Route, Middleware, Controller, Modell,
Migration und View. Dynamische Routen, bedingte Regeln, Policies, verschachtelte
Geschäftslogik und implizite Paketfunktionen können fehlen. Erkannte Syntax
beweist weder funktionale noch sicherheitstechnische Parität. Ein nicht
erkanntes Muster ist keine Aussage, dass die Funktion im Projekt nicht
existiert. Prüfe insbesondere Mandantengrenzen, Berechtigungen, Transaktionen,
Geldbeträge, Dateiablage und externe Nebenwirkungen manuell.

Die Analyse verändert den Laravel-Ordner nicht. Die optionalen
Projektgerüste, ein versionierter Kompatibilitätskatalog und die
Quellgraph-Verknüpfung werden in späteren 0.9.x-Versionen ergänzt.
