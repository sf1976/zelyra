# Kompatibilitätsregeln und Formatversionen

**Geltungsstand:** Arbeitsrichtlinie für die Entwicklung nach 0.4.0. Dieses
Dokument hält bestehende Garantien und Vorschläge fest; es erweitert keine
Zusage über den exakt veröffentlichten Release hinaus.

## Veröffentlichte Basis

Die maßgebliche veröffentlichte Basis ist [`v0.4.0`](https://github.com/sf1976/zelyra/releases/tag/v0.4.0),
Quell-Commit [`507c29e95084a59029d6b436bd69d9132c3a8937`](https://github.com/sf1976/zelyra/commit/507c29e95084a59029d6b436bd69d9132c3a8937).
Dies ist der aktuelle stabile Release. Ein stabiler Release macht nicht
automatisch jeden experimentellen Befehl, jedes generierte Format oder jedes
JSON-Feld zu einer Kompatibilitätszusage; für den konkreten Umfang gelten die
Tabelle und der exakte Tag. Die Arbeit nach dieser Basis liegt im separaten
0.5.0-Entwicklungszweig.

Bis ein Format ausdrücklich als stabil bezeichnet und durch einen
veröffentlichten Release gedeckt ist, gilt es als experimentell. Die
Versionsnummer eines Git-Tags, die Versionsnummer eines lokalen Binaries und
die Version eines Formats sind verschiedene Angaben.

## Tatsächlicher Formatstand im Repository

| Schnittstelle | Nachgewiesener Stand | Bedeutung |
|---|---|---|
| CLI-Maschinenausgaben | Gemeinsamer Umschlag `schema_version: "1"` mit `command`, `success` und `diagnostics` | JSON-Befehle wie `check`, `context` und `impact` verwenden diesen Umschlag. Menschliche Terminalausgaben sind keine Maschinen-API. |
| Modulübergreifende Tabellenverweise | In 0.4.0 veröffentlicht; `E-MOD-019`/`E-MOD-021` prüfen erkannte Zugriffe | Erkannte Verweise benötigen das importierte Tabellenmodul und einen passenden `access`-Grant (`read`, `write`, `read_write`) mit exaktem projekt-relativem Modulpfad. CRUD, Formulare und Authentifizierung benötigen `read_write`; unbekannte SQL-Zugriffe ebenso. Das ist eine Compiler-Prüfung erkannter Kanten, keine MariaDB-Berechtigung; unbekannte SQL-Formen und Schemaänderungsrechte bleiben ungeprüft. |
| Datenbank-Provider im Modulgraphen | In 0.4.0 veröffentlichte `E-MOD-022`-Prüfung | SQL-Funktionen und DB-gestützte Ressourcen benötigen einen direkten oder transitiven Importpfad zum Datenbankmodul. Ein Import nur durch `main.zyl` wird nicht vererbt. Das erzwingt keine Laufzeitverbindungsauswahl. |
| Datenbankbindungen in `context` | In 0.4.0 veröffentlichtes additives Feld, weiter `schema_version: "1"` | Nennt Provider-Modul, DB-Verbraucher, Importauflösung und Verbindungsvariable samt tatsächlichem `DATABASE_URL`-Fallback. Ohne Deklaration wird `DATABASE_URL` als Legacy-Konfiguration markiert. Zugangsdaten bleiben verborgen; eine Verbindung pro Prozess. |
| Modulübergreifende Views und Komponenten | In 0.4.0 veröffentlichte experimentelle UI-Sichtbarkeit | Für Page-/CRUD-Layouts und erkannte Tags in Page-, View-, Komponenten- und CRUD-Slot-HTML sind `pub view` / `pub component` und ein direkter oder transitiver Importpfad nötig. `E-MOD-007` meldet private UI-Deklarationen, `E-MOD-020` eine fehlende Abhängigkeitskante. Die HTML-Komponentenerkennung scannt bekannte Tagnamen; sie ist keine vollständige HTML- oder Namespace-Analyse. |
| `module bundle --dry-run` | Experimenteller Umschlag `schema_version: "1"` mit sortierter Dateiliste und `writes_performed: false` | Listet Zielpfade nach Prüfung eines temporären Pakets auf; keine stabile veröffentlichte Schnittstelle und kein Nachweis eines vollständigen Deployments. |
| Edit-Anfrage | Exakte `schema_version: "1"` | Andere oder fehlende Versionen werden abgelehnt. |
| `zelyra.toml` | Kein allgemein ausgewertetes Formatversionsfeld nachgewiesen | Die vorhandene Feature-Auswertung kennt `[features]`; daraus folgt keine Versionierung des gesamten Projektmanifests. |
| Generierte Projektdateien | Kein Generator-/Vorlagenversionsfeld nachgewiesen | `zelyra new` legt Projektdateien an; daraus folgt kein automatischer Upgrade- oder Migrationsmechanismus. |
| `db plan` | In 0.4.0 veröffentlichter experimenteller `--format=json`-Plan `zelyra.schema-plan/v1` | JSON enthält eine stabile Vorwärts-Plan-ID, Schema-SHA-256-Fingerprints, Drift, geordnete SQL-Schritte und Vorprüfungen auf NULL-Werte, Pflichtspalten, Dubletten und verwaiste Fremdschlüsselwerte. `rollback` enthält einen an Fingerprints gebundenen Rückwärtsplan mit eigener ID, wenn alle Rückwärtsoperationen unterstützt sind; nicht unterstützte Umkehrungen werden fail-closed ausgewiesen. Jede Schemaänderung benötigt ein unabhängig geprüftes Backup, und Rückwärts-DDL kann spätere Daten löschen. Alle drei Schema-Backends protokollieren Pläne und Ergebnisse in `_zelyra_schema_history`; MariaDB protokolliert Prüfpunkte je DDL-Schritt. Lesende Vorprüfungen verhindern keine parallelen Schreibzugriffe. |
| Modul-Bundle-Manifest | `format_version: 1` im unveröffentlichten 0.5.0-Vorbereitungszweig | Experimentelles Manifest; nicht Teil von 0.4.0 und keine Kompatibilitätszusage zwischen Entwicklungsständen. |
| API-Routenmetadaten | In 0.4.0 veröffentlichte Quellfelder `version`, `deprecated` und `rate_limit` | Antworten geben API-Metadaten aus; OpenAPI enthält dieselben Angaben. Kontingente gelten prozesslokal, je Route und TCP-Peer-IP, sind auf 4096 Client-Buckets begrenzt und werden beim Neustart zurückgesetzt. Reverse-Proxy-Weiterleitungsheader werden nicht vertraut. |
| Auth-Limit-Konfiguration | In 0.4.0 veröffentlichte Quellfelder `login_rate_limit` und `login_block_seconds` | Defaults bleiben fünf Fehlversuche je 900 Sekunden und 60 Sekunden Sperre. Der prozesslokale Limiter ist auf 4096 Schlüssel begrenzt, lehnt bei voller Tabelle neue Schlüssel ab und wird beim Neustart zurückgesetzt; Reset-Flows werden dadurch nicht implementiert. |
| Sitzungs-Geräteangabe | In 0.4.0 veröffentlichte optionale Spalte `device_label: String(255)` in der Sitzungstabelle | Neue Sitzungen speichern nur bei vorhandener Spalte einen begrenzten, von Steuerzeichen bereinigten Client-`User-Agent`. Bestehende Tabellen bleiben ohne Spalte gültig. Der Wert ist nicht vertrauenswürdige Anzeigeinformation und keine Geräteauthentifizierung; IP-Adressen werden nicht gespeichert. |

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
6. **Datenbankschemata:** 0.4.0 veröffentlicht das experimentelle Format
   `zelyra.schema-plan/v1`. Plan und Vorprüfungen machen eine Schemaänderung
   allein weder sicher noch umkehrbar; Grenzen bei Backups, parallelen
   Schreibzugriffen und unterbrochenen Updates bleiben ausdrücklich bestehen.
   Weitere Wiederherstellungsnachweise sind ein 0.5.0-Release-Gate.
7. **Dokumentation und Tests:** Jede als stabil bezeichnete Schnittstelle
   erhält positive und negative Versions-/Kompatibilitätstests und wird in
   Deutsch und Englisch beschrieben. Ein grüner Test für eine Einzelversion
   belegt keine Kompatibilität zu einem früheren Release.

Diese Regeln versprechen weder, dass 0.4.0 alle Formate stabilisiert, noch,
dass der unveröffentlichte 0.5.0-Entwicklungszweig rückwärtskompatibel ist. Sie
legen fest, welche Nachweise vor einem entsprechenden künftigen Versprechen
nötig sind.
