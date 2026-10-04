# Kompatibilitätsregeln und Formatversionen

**Geltungsstand:** Vorschlag für die Entwicklung ab 0.4.0; noch kein
Kompatibilitätsversprechen für unveröffentlichte Entwicklungs-Builds.

## Veröffentlichte Basis

Die maßgebliche veröffentlichte Compilerbasis ist [`v0.3.0`](https://github.com/sf1976/zelyra/releases/tag/v0.3.0),
Quell-Commit [`cd0600a73c22ed84f40aef870c8128ce2cb45be0`](https://github.com/sf1976/zelyra/commit/cd0600a73c22ed84f40aef870c8128ce2cb45be0).
Das ist ein experimenteller Release, keine Produktionsfreigabe. Der offene
0.4-Entwicklungszweig enthält Änderungen, die nicht Bestandteil dieses
Releases sind. Ein Branch-Build darf nicht allein deshalb als 0.4.0 behandelt
werden, weil er neuere Funktionen enthält.

Bis ein Format ausdrücklich als stabil bezeichnet und durch einen
veröffentlichten Release gedeckt ist, gilt es als experimentell. Die
Versionsnummer eines Git-Tags, die Versionsnummer eines lokalen Binaries und
die Version eines Formats sind verschiedene Angaben.

## Tatsächlicher Formatstand im Repository

| Schnittstelle | Nachgewiesener Stand | Bedeutung |
|---|---|---|
| CLI-Maschinenausgaben | Gemeinsamer Umschlag `schema_version: "1"` mit `command`, `success` und `diagnostics` | JSON-Befehle wie `check`, `context` und `impact` verwenden diesen Umschlag. Menschliche Terminalausgaben sind keine Maschinen-API. |
| Modulübergreifende Tabellenverweise | Experimentelle `E-MOD-019`-Prüfung im unveröffentlichten 0.4-Entwicklungsstand | Erkannte Tabellenverweise verlangen das Besitzermodul im Importabschluss des Verbrauchers; Tabellen im Einstieg sind ausgenommen. Lese-/Schreibberechtigungen werden nicht erzwungen. |
| `module bundle --dry-run` | Experimenteller Umschlag `schema_version: "1"` mit sortierter Dateiliste und `writes_performed: false` | Listet Zielpfade nach Prüfung eines temporären Pakets auf; keine stabile veröffentlichte Schnittstelle und kein Nachweis eines vollständigen Deployments. |
| Edit-Anfrage | Exakte `schema_version: "1"` | Andere oder fehlende Versionen werden abgelehnt. |
| `zelyra.toml` | Kein allgemein ausgewertetes Formatversionsfeld nachgewiesen | Die vorhandene Feature-Auswertung kennt `[features]`; daraus folgt keine Versionierung des gesamten Projektmanifests. |
| Generierte Projektdateien | Kein Generator-/Vorlagenversionsfeld nachgewiesen | `zelyra new` legt Projektdateien an; daraus folgt kein automatischer Upgrade- oder Migrationsmechanismus. |
| `db plan` | Textausgabe mit Vorprüfungen, Risiko, Beschreibung und SQL | Es gibt hier derzeit kein versioniertes, gespeichertes Maschinenformat für Schema-Pläne. Ausgabe nicht mit einem stabilen Parser auswerten. |
| Modul-Bundle-Manifest | `format_version: 1` im unveröffentlichten Modul-Entwicklungszweig | Experimentelles Manifest; kein Teil von 0.3.0 und keine Kompatibilitätszusage zwischen Entwicklungsständen. |

Die Versionsfelder sind anhand der Implementierung in `cli/src/main.rs`,
`cli/src/edit.rs` und der vorhandenen CLI-Tests verifiziert. Die Modul-Bundle-
Version darf nicht mit `schema_version` der allgemeinen CLI-Ausgabe verwechselt
werden.

## Vorgeschlagene Regeln für veröffentlichte Formate

1. **Releasegrenze:** Kompatibilität wird gegen einen exakten veröffentlichten
   Tag geprüft. Vorab- und Branch-Builds dürfen Projekt- oder Manifestformate
   ändern und müssen diese als experimentell kennzeichnen.
2. **Maschinenlesbares JSON:** Ein bestehendes `schema_version` behält seine
   Bedeutung und Typen. Optionale zusätzliche Felder sind innerhalb derselben
   Schema-Hauptversion zulässig; Verbraucher sollen unbekannte Felder
   ignorieren. Eine inkompatible Änderung an erforderlichen Feldern, Typen
   oder Semantik benötigt eine neue Schema-Hauptversion und eine verständliche
   Fehlermeldung für nicht unterstützte Versionen.
3. **Eingaben:** Versionierte Anfragen werden fail-closed validiert. Eine
   unbekannte Version darf nicht stillschweigend als aktuelle Version
   interpretiert werden. Versionsfeld, erlaubte Werte und Fehlerfall werden in
   Tests festgeschrieben.
4. **Projektdateien und Quellsprache:** Vor einer inkompatiblen Änderung an
   stabil veröffentlichten Syntax oder Konfiguration sind betroffene Dateien,
   Diagnose, Upgrade-Anleitung und unterstützter Übergang zu dokumentieren.
   Experimentelle Konstrukte können sich vor einem stabilen Release ändern,
   müssen aber im Handbuch und in Release-Notizen entsprechend markiert sein.
5. **Generierte Ausgaben:** Eine neue Generatorversion überschreibt keine
   bestehende Anwendung stillschweigend. Änderungen an Vorlagen und erzeugten
   Metadaten benötigen einen überprüfbaren Diff und Tests gegen bereits
   veröffentlichte Beispielprojekte.
6. **Datenbankschemata:** `db plan` ist derzeit eine lesbare Vorschau und kein
   versionsstabiles Migrationsartefakt. Schemaänderungen werden nicht allein
   durch diese Richtlinie umkehrbar. Versionierte Pläne, Vorprüfungen,
   Freigabe, Driftbehandlung und Wiederherstellung bleiben Implementierungs-
   und Release-Gates des 0.4-/0.5-Plans.
7. **Dokumentation und Tests:** Jede als stabil bezeichnete Schnittstelle
   erhält positive und negative Versions-/Kompatibilitätstests und wird in
   Deutsch und Englisch beschrieben. Ein grüner Test für eine Einzelversion
   belegt keine Kompatibilität zu einem früheren Release.

Diese Regeln versprechen weder, dass 0.3.0 alle Formate stabilisiert, noch,
dass der aktuelle 0.4-Entwicklungszweig rückwärtskompatibel ist. Sie legen
fest, welche Nachweise vor einem entsprechenden künftigen Versprechen nötig
sind.
