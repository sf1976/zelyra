# Zelyra-Release-Workflow

Diese Anleitung beschreibt die Release-Prüfungen des Repositorys. Eine manuelle
Probe baut dieselben Linux- und Windows-Pakete wie ein Release, kann aber kein
GitHub-Release veröffentlichen.

## Vor dem Release

1. Die Workspace-Version in `Cargo.toml` setzen und `Cargo.lock` aktualisieren.
2. Die zweisprachigen `Unreleased`-Einträge in `CHANGELOG.md` abschließen.
3. Formatierung, Workspace-Prüfung, Clippy und die vollständige Testsuite
   ausführen.
4. Den Release-Vorbereitungsbranch pushen und einen Pull Request nach `main`
   öffnen.

Bei Änderungen an Release-Automatisierung, Paketmanifesten oder Quellcode baut
der Release-Workflow die Linux- und Windows-Artefakte bereits im Pull Request.
Das sind ausschließlich Prüf-Artefakte; der Veröffentlichungsjob läuft bei
Pull Requests nicht.

## Manuelle Probe ohne Veröffentlichung

In GitHub **Actions → Zelyra Release → Run workflow** öffnen, den zu prüfenden
Branch auswählen und das Release-Tag eingeben, zum Beispiel `v0.1.50`. Der
Workflow prüft das Tag-Format und baut Plattformarchive, eigenständige
Updater-Binärdateien und SHA-256-Dateien. Er lädt sie als Workflow-Artefakte
hoch, erstellt aber weder ein Tag noch ein Release.

Prüfung und Build besitzen im Repository ausschließlich Leserechte. Nur der
durch einen Tag-Push ausgelöste Veröffentlichungsjob erhält die Berechtigung
`contents: write`.

## Veröffentlichen

Nach Review und Merge nach `main` das passende Versions-Tag pushen, zum Beispiel:

~~~bash
git switch main
git pull --ff-only
git tag -a v0.1.50 -m "Zelyra 0.1.50"
git push origin v0.1.50
~~~

Das Tag muss `vMAJOR.MINOR.PATCH` entsprechen; optional ist ein
Prerelease-Suffix wie `-alpha.1`. GitHub Actions baut beide unterstützten
Release-Ziele und veröffentlicht das Release erst, wenn beide Builds
erfolgreich waren. Bereits verwendete Release-Tags dürfen nicht wiederverwendet
oder verschoben werden.

Der Release-Workflow stellt derzeit Linux- und Windows-x86_64-Artefakte bereit.
Andere Plattformen gelten dadurch nicht als unterstützt.
