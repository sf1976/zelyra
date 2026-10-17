# Lokale Zelyra-Pakete (experimentell)

Diese unveröffentlichte Arbeit für 1.1.5 ergänzt Offline-Pakete aus lokalen
Pfaden auf Basis der vorhandenen expliziten Modulimporte. Zelyra lädt keine
Pakete herunter und führt keine Build-Skripte aus. Projekte ohne `[package]`
behalten das bisherige Verhalten mit Dateiimporten.

Lokale Pakete sind derzeit Quellcode-Abhängigkeiten und keine installierbaren
oder über normale Anwendungsseiten verwalteten Module. `zelyra module list`
bietet einen CLI-Katalog mit Quellmodulen, Exporten, Paketversionen,
Prüfsummen, Effektobergrenzen und typisierten Einstellungs-Schemas aus
Paketmanifesten.
Konfigurationswerte lassen sich in `zelyra.settings.toml` speichern und
Paketcode kann sie mit `module_setting<T>` lesen. Geheimnisse werden dabei nur
als Prozessumgebungsreferenzen eingebunden. Studio enthält eine geschützte
lokale Einstellungsoberfläche. Von Anwendungen ausgelieferte Einstellungsseiten
und eine Secret-Verwaltung jenseits dieser Prozessumgebung fehlen noch.
Wiederverwendbare Geschäfts- und Integrationsmodule (unter anderem Auth,
ODBC, CSV, PDF und Excel) sind Roadmap-Ziele; das Paketformat allein stellt
diese Funktionen nicht bereit.

## Modulübersicht

```sh
zelyra module list main.zyl
zelyra module list main.zyl --format json
```

Die JSON-Ausgabe ist deterministisch und enthält Quellpfade relativ zum
Projekt, Importaliasnamen, öffentliche Exporte, lokale Paketidentitäten und
Prüfsummen sowie deklarierte Effektobergrenzen. Diese Obergrenzen erteilen
keine Laufzeit-Capabilities. Die Ausgabe enthält keine `.env`-Werte,
Zugangsdaten oder sonstigen Geheimnisse. Bei Paketabhängigkeiten wird der
Lockfile-Status nur als aktuell gemeldet, wenn der normale Compilercheck den
Lock bereits akzeptiert hat.

## Manifest

Ein lokales Paket lässt sich ohne Installer, Registry, Build-Skript oder
Netzwerkverbindung erstellen:

```sh
zelyra module init packages/customer_tools --name customer_tools --version 0.1.0
```

Der Befehl legt eine getypte öffentliche Beispielfunktion, ein Paketmanifest
mit Kompatibilitätsbereich der aktuellen CLI und leerer Effektobergrenze sowie
eine kurze README an. Vorhandene Zielpfade werden nicht überschrieben. Danach
werden Pfad und SemVer-Version unter `[dependencies.<alias>]` in der Anwendung
eingetragen und das Paket mit `import "pkg:<alias>" as tools` importiert.
Anschließend `zelyra package lock .` und `zelyra check main.zyl` ausführen.

Eine Anwendung deklariert Identität und lokale Abhängigkeiten in
`zelyra.toml`:

```toml
[package]
name = "invoice_app"
version = "1.0.0"
entry = "main.zyl"
zelyra = ">=1.1.0, <1.2.0"

[dependencies.money]
path = "packages/money"
version = "^1.2"
```

Das Paket unter `packages/money/zelyra.toml` deklariert seine Identität und
seinen Einstieg. `[package]` akzeptiert `name`, `version`, optional `entry`,
`zelyra` und `effects`; Schreibfehler werden ausdrücklich gemeldet. Das
optionale Feld `zelyra` ist eine SemVer-Vorgabe, die beim Laden des
Paketgraphen gegen die Version der laufenden Zelyra-CLI geprüft wird. Zum
Beispiel akzeptiert `">=1.1.0, <1.2.0"` kompatible 1.1-Versionen und weist
andere CLI-Versionen zurück, bevor das Paket geprüft oder ausgeführt wird.
Die Vorgabe fließt in die Paketmanifest-Prüfsumme ein; eine Änderung macht das
Lockfile ungültig. `zelyra module list` zeigt die Vorgabe in der Text- und
JSON-Übersicht:

```toml
[package]
name = "money"
version = "1.2.3"
entry = "src/lib.zyl"
zelyra = ">=1.1.0, <1.2.0"
effects = ["Database(read)"]
```

`effects` ist optional. Ist es gesetzt, begrenzt es die `uses`-Effekte jeder
Funktion im Paket auf die aufgeführten bekannten Capability-Namen. Die
Obergrenze muss zu den deklarierten Capabilities passen; für SQL-Lesezugriffe
ist `Database(read)` zu verwenden, wenn die Funktion genau diese feinere
Capability deklariert. `[]` erlaubt reine Funktionen ohne deklarierte Effekte.
Ist es nicht gesetzt, gilt
keine zusätzliche Paketobergrenze. Der Eintrag erteilt der Anwendung keine
Laufzeitrechte: Die Konfiguration der Anwendung muss den Effekt weiterhin
freigeben. Unbekannte und doppelte Namen werden abgelehnt. Da das Manifest
gehasht wird, macht eine Änderung das Lockfile ungültig.

## Vertrauen und Integrität

Paketquellen sind ausführbarer Anwendungscode. Eine SHA-256-Prüfsumme im
Lockfile erkennt Änderungen relativ zu einem Lockfile, dem das Projekt bereits
vertraut. Sie authentifiziert weder den Herausgeber noch die Herkunft der
Dateien und macht ein schädliches Paket nicht sicher. `zelyra package lock
--update` übernimmt bewusst die aktuellen lokalen Dateien als neuen gesperrten
Stand; prüfe daher Quelle und Diff vor diesem Schritt. Der stabile Paketablauf
in 1.1.5 enthält keine Registry und keine Downloads aus dem Netz. Die
unveröffentlichte Ökosystemarbeit ergänzt signierte Quellartefakte, eine
ausdrückliche lokale Herausgeber-Vertrauensdatei und einen deterministischen
Generator für statische Registry-Indizes; Netzsuche und verifizierte
Installation sind noch offen. Eine Signatur weist nach, dass der
zugehörige vertrauenswürdige Schlüssel den Artefakt-Hash signiert hat. Sie
beweist weder, dass der Code sicher ist, noch dass ein Herausgeber
vertrauenswürdig handelt. Paketobergrenzen für `effects` sind zusätzliche
Compilergrenzen; sie erteilen oder umgehen weder Anwendungs-Capabilities noch
SQL-Prüfungen, Contracts oder Laufzeitregeln.

Im aktuellen Entwicklungszweig kann ein Herausgeber einen Schlüssel anlegen
und ein Quellpaket samt vollständig enthaltenen Paketabhängigkeiten signieren:

```sh
zelyra package keygen --publisher rechnungsteam --output rechnungsteam.key
zelyra package sign packages/billing --publisher rechnungsteam \
  --key rechnungsteam.key --output billing-1.0.0.zpkg
```

Die private Schlüsseldatei wird ohne Überschreiben angelegt und erhält unter
Unix nur Rechte für den Eigentümer. Bewahre sie außerhalb des Projekts auf und
sichere sie geschützt. `keygen` gibt den öffentlichen Schlüssel aus. Eine
Projektverantwortliche Person muss ihn ausdrücklich hinzufügen, bevor die
Signatur als vertrauenswürdig gilt:

```sh
zelyra package trust add rechnungsteam <public-key-hex>
zelyra package trust list
zelyra package verify billing-1.0.0.zpkg
zelyra package registry build ./veroeffentlichte-pakete
```

`registry build` durchsucht einen echten Verzeichnisbaum ohne Symlinks nach
`.zpkg`-Dateien, prüft Signatur und Quellprüfsumme und schreibt deterministisch
eine `index.json`, die statisch gehostet werden kann. Der Index dient nur zur
Suche und begründet kein Vertrauen: Clients müssen dem Herausgeberschlüssel
weiterhin ausdrücklich vertrauen. Der Befehl lädt nichts hoch. Artefakte und
Index müssen gemeinsam mit einem Hostingdienst eigener Wahl veröffentlicht
werden.

Eine exakte, vertrauenswürdige Version kann in den lokalen Paketspeicher des
Projekts geladen werden:

```sh
zelyra package add https://pakete.example.test/index.json rechnung 1.0.0 --as billing
zelyra package lock --check
zelyra check main.zyl
```

Der Installer verlangt HTTPS (unverschlüsseltes HTTP ist nur beim Test über
Loopback erlaubt), folgt keinen Weiterleitungen, begrenzt Antwortgröße und
Laufzeit, verlangt eine normalisierte exakte SemVer-Version, prüft Index-Hash
und Artefaktsignatur gegen `zelyra.trust.toml` und hält Artefaktdownloads auf
dem Ursprung des Index. Er schreibt ohne Überschreiben nach
`.zelyra/packages/<name>/<version>`, aktualisiert Manifest und Lockfile mit
Rollback und vermerkt Registry, Signierer, öffentlichen Schlüssel und
Artefakt-Hash im `zelyra.lock`. Transitive Abhängigkeiten werden noch nicht
separat aufgelöst oder geladen. Paketabhängigkeiten müssen bereits im signierten
Paketverzeichnis enthalten sein. Prüfung und Ausführung installierter Pakete sind
offline möglich.
Nach der vorläufigen Lock-Aktualisierung prüft der Installer außerdem das
gesamte Projekt mit dem Compiler und rollt bei Fehlern zurück. `zelyra module
bundle` verschiebt Pakete aus dem Cache in portable Pfade unter
`packages/<name>/<version>`, erhält die Herkunft im Lock und hält das Bundle
offline ausführbar.

Eine Abhängigkeit wird ausdrücklich mit `zelyra package remove <alias>`
entfernt. Der Befehl prüft das verbleibende Projekt und aktualisiert
`zelyra.toml` sowie `zelyra.lock` mit Rollback bei Fehlern. Dateien im Cache
bleiben erhalten, damit gemeinsam genutzte oder Nutzerdaten nicht ungefragt
gelöscht werden. Nicht mehr referenzierte Versionen lassen sich erst anzeigen
und nach ausdrücklicher Prüfung entfernen:

```sh
zelyra package cache clean
zelyra package cache clean --apply
```

Die Bereinigung verweigert ein veraltetes Lockfile und Symlinks im Paketcache,
erhält alle dort referenzierten Pakete und verschiebt Kandidaten vor dem Löschen
in ein temporäres Quarantäneverzeichnis. Prüfe zuerst die Vorschau. Scheitert
das abschließende Löschen, nennt der Befehl den Wiederherstellungspfad.

Ein vorhandener Alias lässt sich ausdrücklich auf eine andere exakte Version
aktualisieren: `zelyra package update <index-url> <name> <exact-version> --as
<alias>`. Das neue Artefakt muss dieselbe Paketidentität wie die bisherige
Abhängigkeit haben und dieselben Vertrauens- und Integritätsprüfungen bestehen
wie bei `add`. Manifest und Lock werden mit Rollback aktualisiert. Vorherige
Cacheversionen bleiben bis zum ausdrücklichen Aufruf von
`zelyra package cache clean --apply` erhalten; es gibt weder SemVer-Bereich noch
implizite Auswahl der neuesten Version.

Die Standarddatei heißt `zelyra.trust.toml` im aktuellen Verzeichnis. Mit
`--file <pfad>` kann eine andere Vertrauensdatei verwaltet oder für die Prüfung
verwendet werden. Signierte Quellartefakte enthalten ausschließlich
`zelyra.toml` und `.zyl`-Quellen, sind auf 32 MiB und 4.096 Dateien begrenzt
und unterstützen Paketabhängigkeiten, wenn deren Dateien vollständig im
Paketverzeichnis enthalten sind und relative Pfade verwenden. Externe Pfad-
Abhängigkeiten scheitern an der Compilerprüfung. Das versionierte Signaturformat
bindet Herausgeber-ID, Paketname, exakte Version, Kompatibilitätsmanifest und
normalisierten Quell-Hash. Entfernte transitive Auflösung, Updates und eine
veröffentlichte stabile Schnittstelle fehlen noch.

Pakete können Konfigurationsfelder unter `[settings.<name>]` beschreiben:

```toml
[settings.api_timeout]
type = "Int"
required = false
default = 15
description = "Zeitlimit für Anfragen in Sekunden"

[settings.api_token]
type = "String"
required = true
secret = true
description = "Privater API-Schlüssel"
```

Das Schema akzeptiert derzeit `String`, `Bool`, `Int` und `Float`.
`required` muss ausdrücklich angegeben werden. Optionale Einstellungen
benötigen einen typgerechten Standardwert. Geheimnisse dürfen jeden
unterstützten Skalartyp verwenden, aber keinen Standardwert haben. Env-Werte
werden streng als `Bool`, `Int` oder endlicher `Float` geparst; geheime
Strings dürfen einschließlich JSON-Kodierung höchstens 8 KiB belegen. Namen
verwenden Kleinbuchstaben in Snake-Case und sind auf 64 Bytes begrenzt;
String-Standardwerte auf 4 KiB.
Beschreibungen sind optional und auf 512 Bytes begrenzt.
`module list` gibt dieses Schema deterministisch als JSON und in der
Menschenansicht aus, aber keine konfigurierten Werte. Änderungen am Schema
fließen in die Manifest-Prüfsumme ein und machen das Lockfile ungültig.

## Anwendungseinstellungen prüfen (experimentell)

Projektwerte liegen getrennt von `zelyra.toml` im optionalen
`zelyra.settings.toml`. Dadurch verändern sie weder Paketprüfsummen noch das
Lockfile. Pakete werden mit ihrem normalisierten Workspace-Pfad adressiert;
der Projekt-Root verwendet `.`:

```toml
[package_settings."packages/ledger"]
api_timeout = 15
api_token = { env = "LEDGER_API_TOKEN" }
```

`zelyra module settings check main.zyl [--format=json]` vergleicht die Werte
mit den Schemafeldern aller geladenen Pakete. Unterstützt werden derzeit
`String` (bis 4 KiB), `Bool`, `Int` und endliche `Float`-Werte. Nicht gesetzte
optionale Werte verwenden den deklarierten Standard; fehlende Pflichtwerte
führen zu `E-PKG-SETTINGS-002`. Unbekannte Pakete/Felder, falsche Typen,
Symlinks und Dateien über 1 MiB werden abgewiesen. Das neue Settings-Dokument
ist nicht Teil der Paket-Prüfsumme.
Die JSON-Ausgabe von `module settings check` enthält außerdem das Feld
`required`, damit Werkzeuge Pflicht- und optionale Standards unterscheiden
können.

Geheimnisse müssen als `{ env = "GROSSBUCHSTABEN_NAME" }` referenziert werden.
Der Check liest den benannten Eintrag aus der Prozessumgebung, prüft und
konvertiert ihn passend zum deklarierten Skalartyp (`String`, `Bool`, `Int` oder
endlicher `Float`), gibt weder Variablenname noch Geheimniswert aus und meldet
`secret_available` oder `missing`. Leere oder typungültige Env-Werte werden
mit einer wertfreien Diagnose abgewiesen. Variablen müssen vor dem Zelyra-Aufruf in dessen Prozessumgebung
vorhanden sein; die CLI lädt keine `.env`-Datei für diese Einstellung. Auch
`zelyra run` und `zelyra serve` prüfen vor der Ausführung, ob alle erforderlichen
Paketwerte gesetzt und typgültig sind. Ein fehlendes Pflichtfeld führt zu
`E-PKG-SETTINGS-002`; der Variablenname eines Geheimnisses wird nicht ausgegeben.
Paketcode liest einen Wert explizit und typisiert mit
`module_setting<Int>("api_timeout")`. Der Name muss ein Stringliteral sein,
das Feld muss im eigenen Paketmanifest deklariert sein und der angegebene Typ
muss exakt zum Schema passen. Fremde Paketeinstellungen sind nicht erreichbar.
Optionale Felder verwenden ihren Manifest-Standardwert; Geheimnisse werden aus
der referenzierten Prozessvariablen gelesen. Die Laufzeit redigiert Werte in
Debug-Ausgaben. Der Zugriff funktioniert mit AST, MIR und Bytecode. MIR und
Bytecode bleiben insgesamt experimentelle Backends; diese einzelne Einstellung
ist kein Nachweis vollständiger Backend-Parität. Zelyra Studio bietet unter **Moduleinstellungen** eine geschützte lokale Oberfläche zum Anzeigen und Bearbeiten dieser Werte. Nicht geheime Werte werden typisiert geprüft und mit Revisionsabgleich atomar gespeichert; unbekannte Felder, ungültige Typen und Symlinks werden abgewiesen. Die Oberfläche zeigt keine vorhandenen Geheimniswerte oder gespeicherten Umgebungsvariablennamen an. Für Geheimnisse kann eine neue Env-Referenz eingetragen werden; ein leeres Feld lässt die bisherige Referenz unverändert. Die Verwaltung über normale, von Anwendungen ausgelieferte Zelyra-Seiten sowie sichere Secret-Verwaltung jenseits von Prozessumgebungsvariablen bleiben offen.

Ohne Angabe gilt `main.zyl` für eine Anwendung, wenn diese Datei vorhanden
ist, sonst `src/lib.zyl` für eine Bibliothek. Paketnamen verwenden kleine
ASCII-Buchstaben, Ziffern, `_` und `-` und beginnen mit einem Kleinbuchstaben.
Der Abhängigkeitsschlüssel ist ein lokaler Alias und darf vom Paketnamen
abweichen; die konkrete Paketversion muss die angegebene SemVer-Vorgabe
erfüllen. Unterstützt
werden nur projekt-relative `path`-Abhängigkeiten mit expliziter `version`.
Pfade verwenden `/`; `..` darf auf Geschwisterverzeichnisse verweisen, solange
das aufgelöste Paket im Projekt-Workspace bleibt. Symlink-Traversierung,
Workspace-Ausbrüche, Backslashes, Registries, Git-Quellen und native Plugins
werden nicht unterstützt.

## Import und Lockfile

Eine Abhängigkeit wird über ihren Manifest-Schlüssel mit der vorhandenen
Namespace-Syntax importiert. Der Abhängigkeitsschlüssel darf sich von der
Paketidentität unterscheiden:

```zelyra
import "pkg:money_core" as money

fn main() {
    print(money::add_cents(1250, 50))
}
```

Bei diesem Import darf `[dependencies.money_core]` auf ein Paket mit
`[package].name = "money"` verweisen. Der Importname bleibt explizit;
Paketidentität und Versionskompatibilität werden unabhängig geprüft.

```toml
[dependencies.money_core]
path = "packages/money"
version = "^2.1"
```

Der Paketeinstieg muss importierte Deklarationen mit `pub` exportieren. Dateien
innerhalb eines Pakets werden relativ zu diesem Paket aufgelöst;
Anwendungsimporte bleiben relativ zum Projektverzeichnis. Vorhandene
Zyklus-, Sichtbarkeits-, Typ- und Capability-Prüfungen gelten weiter.

Zwei Abhängigkeitsaliase dürfen auf verschiedene lokale Versionen mit
demselben `[package].name` zeigen, zum Beispiel `math@1.x` unter
`packages/math-v1` und `math@2.x` unter `packages/math-v2`. Transitive Importe
verwenden den jeweiligen Alias aus dem Manifest des importierenden Pakets.
Das Lockfile hält beide Pfade und Versionen fest; eine Version wird nicht
implizit ausgewählt.

`zelyra module bundle` meldet einen vollständigen Quellabschluss nur, wenn der
statische Plan keine ungelösten Verweise enthält und der generierte Einstieg
die Compilerprüfung besteht. Das Manifest hält diesen Nachweis fest.
`complete_deployment` bleibt `false`; Laufzeiteffekte und Betriebsanforderungen
müssen weiterhin geprüft werden.

Die Modulplanung akzeptiert `pkg:<abhängigkeitsschlüssel>` als Auswahl und
führt zum Paketeinstieg. Ein Bundle nimmt die im Modulabschluss verwendeten
Pakete und deren transitive Abhängigkeiten mit. Paketmanifeste und alle zu ihrer
Prüfsumme gehörenden Zelyra-Quelldateien bleiben erhalten; das erzeugte
Anwendungsmanifest enthält nur diese Paketabhängigkeiten und erhält ein dazu
neu berechnetes Lockfile. Nicht verwendete Projektpakete werden ausgelassen.
Der Quellabschluss wird compilergeprüft, ist aber weiterhin keine Garantie
eines vollständigen oder betriebsbereiten Deployments.

Bei `zelyra module bundle ... --docker --compiler-ref <40-stelliger-Commit>`
muss der Commit alle Sprach- und Paketfunktionen des ausgewählten Quellabschlusses
enthalten. Der erzeugte Docker-Build kompiliert genau diesen Compiler, kopiert
das Bundle in die Build-Stufe und führt vor dem Runtime-Image
`zelyra check main.zyl` aus. Ein veralteter oder inkompatibler Compiler-Pin
bricht daher den Image-Build ab. `complete_deployment` bleibt `false`, bis
externe Effekte und Betriebsanforderungen geprüft sind.

Ein reproduzierbares lokales Lockfile erzeugen und prüfen:

```sh
zelyra package lock main.zyl
zelyra package lock main.zyl --check
zelyra check main.zyl
zelyra run main.zyl
```

Das erzeugte `zelyra.lock` führt Paketinstanzen in normalisierter Workspace-
Pfadreihenfolge mit Name, Version, Pfad, Abhängigkeitskanten und SHA-256-
Prüfsummen auf. Verschiedene Versionen dürfen denselben Paketnamen tragen.
Eine Prüfsumme umfasst
`zelyra.toml` und `.zyl`-Dateien jedes Pakets in sortierter UTF-8-Relativpfad-
Reihenfolge; CRLF-Zeilenenden werden vor dem Hashen zu LF normalisiert.
Paket-Symlinks werden abgelehnt. Änderungen am Manifest oder
Quelltext machen das Lockfile ungültig. Nach Prüfung der Änderung aktualisierst
du es mit `zelyra package lock main.zyl --update`. `zelyra check`, `build`,
`run` und `serve` lehnen ein fehlendes oder veraltetes Lockfile ab, wenn
Abhängigkeiten deklariert sind. Die Prüfung erfolgt lokal und offline.

Die Prüfsumme lässt generierte Inhalte unter `target/` und Git-Metadaten unter
`.git/` bewusst aus. Paketeinstieg und paketinterne Quellimporte dürfen diese
Verzeichnisse nicht betreten. Damit können ausführbare Zelyra-Quellen die
Prüfsumme des Lockfiles nicht umgehen.

## Umfang und Sicherheitsgrenzen

Dies ist der erste Teil der Paketauflösung, nicht der vollständige Modul- und
Paketmeilenstein 1.1.5. Mehrere lokal aufgelöste Versionen desselben
Paketnamens sind möglich, wenn sie über unterschiedliche Abhängigkeitsschlüssel
und lokale Pfade erreicht werden. Paketinstanzen werden nach Pfad, Name, Version und
Inhaltsprüfsumme gesperrt; zwei Pfade mit gleichem Paketnamen und gleicher
Version werden als mehrdeutig abgelehnt. Der Resolver unterstützt lokale
Workspace-Pfade, Inhaltsprüfsummen und die bestehende `pub`-Sichtbarkeit.
Noch nicht enthalten sind Registry, signierte Herkunft oder Vertrauensdialoge.
HostServices ist eine versionierte Rust-Embedding-API (aktuell v7), aber keine
stabile native ABI und kein Pluginvertrag für Pakete. Paketprüfsummen erkennen
Änderungen gegenüber einem geprüften Lockfile; sie belegen weder den Herausgeber
noch die Sicherheit des Verhaltens. Prüfe Paketquellen, Effekte und Änderungen
am Lockfile, bevor du eine Anwendung mit diesem Paket ausführst. Paketquellen
sind lokale Workspace-Pfade; Zelyra lädt sie nicht aus einer nicht
vertrauenswürdigen Remote-Registry.

Das Projekt `examples/business_modules` enthält außerdem ein reines Paket
`csv_import`. `decode_table(source, delimiter)` nutzt den begrenzten CSV-Parser,
verlangt eine Kopfzeile mit nicht leeren Spaltennamen und lehnt Datensätze mit
abweichender Spaltenzahl ab.
Es liefert die geparsten Zeilen zurück und schreibt nicht in die Datenbank.
Das Beispiel führt das Paket mit AST, MIR und Bytecode aus. Spaltenzuordnung,
Vorschauoberfläche und transaktionaler Import in eine Anwendungsdatenbank sind
damit nicht enthalten.
Paketweite `effects`-Obergrenzen beschränken deklarierte Funktionseffekte,
ersetzen aber keine Capability-Freigabe der Anwendung. Paketquelltext wird zur Analyse
geladen und normal kompiliert; `check` führt ihn nicht aus. `run` führt die
Anwendung aus wie bei gewöhnlichen importierten Modulen.
