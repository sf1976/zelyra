# Zelyra-Release-Workflow

Diese Anleitung beschreibt die Release-Prüfungen des Repositorys. Eine manuelle
Probe baut dieselben Linux- und Windows-Pakete wie ein Release, kann aber kein
GitHub-Release veröffentlichen.

Der aktuelle zweisprachige [Entwurf der 0.3.0-Release-Notizen](release-notes/0.3.0.de.md)
führt Umfang, Supportgrenzen, Installations-/Upgrade-Checkliste und bekannte
Einschränkungen auf. Die englische Fassung ist
[hier verfügbar](release-notes/0.3.0.en.md). Er ist keine Release-Ankündigung.
Das ergänzende [Risiko- und Abnahmeregister](release-readiness/0.3.0-risikoregister.de.md)
ordnet jede öffentliche Aussage einem Nachweis zu und benennt die verbleibenden
Kandidatengates.
Das [Sicherheitsgrenzen-Review 0.3.0](security-review-0.3.0.md) dokumentiert
die experimentelle Sicherheitsentscheidung und verbleibende Deploymentrisiken.

## Vor dem Release

1. Die Workspace-Version in `Cargo.toml` setzen und `Cargo.lock` aktualisieren.
2. Die zweisprachigen `Unreleased`-Einträge in `CHANGELOG.md` abschließen.
3. Formatierung, Workspace-Prüfung, Clippy und die vollständige Testsuite
   ausführen.
4. Den Release-Vorbereitungsbranch pushen und einen Pull Request nach `main`
   öffnen.

Vor dem Artefaktbau `python3 scripts/check_release_metadata.py` ausführen. Das
Skript vergleicht die Workspace-Version mit allen lokalen Paketen in
`Cargo.lock`, prüft den `Unreleased`-Abschnitt und kann mit `--binary` auch die
Version des gebauten CLI prüfen. `--print-tag` liefert das exakt passende Tag
für nachgelagerte Checks.

Unter Linux führt `./tests/release-artifact-smoke.sh` einen echten Smoke-Test
aus: Es baut oder verwendet die Release-CLI, erzeugt Archiv und SHA-256-Dateien,
prüft Prüfsummen und Archivpfade, startet die paketierte CLI und prüft den
Installer-Dry-Run. Für das Windows-Artefakt bleibt der Release-Workflow
maßgeblich.

Der Release-Workflow führt zusätzlich `scripts/verify_release_artifacts.py` auf
beiden Plattformen aus. Dieser Check verlangt genau die erwarteten Binär- und
Archivdateien, verifiziert jedes SHA-256-Sidecar, lehnt unsichere oder unerwartete
Archivpfade ab und prüft die CLI-Version des Zielartefakts.

Für ein veröffentlichtes Release kann auf einem Linux-x86_64-System
`./tests/published-release-smoke.sh v0.2.0` ausgeführt werden. Der Test lädt das
veröffentlichte Archiv über den echten Installer, prüft die installierte
Version, wiederholt die Installation und führt `zelyra update --check` mit der
installierten Binärdatei aus. Für die abschließende Abnahme ist das Kandidaten-
Tag einzusetzen; ein erfolgreicher Quellcode-/Packaging-Test beweist allein
noch nicht, dass die veröffentlichten GitHub-Artefakte erreichbar und nutzbar
sind.

Das Release-Tag muss exakt zur Workspace-Paketversion in `Cargo.toml` passen
(zum Beispiel `v0.2.0` für Version `0.2.0`). Bei Abweichungen lehnt der
Workflow das Tag ab, statt Archive mit widersprüchlichem Namen und eingebauter
CLI-Version zu veröffentlichen.

Bei Änderungen an Release-Automatisierung, Paketmanifesten oder Quellcode baut
der Release-Workflow die Linux- und Windows-Artefakte bereits im Pull Request.
Das sind ausschließlich Prüf-Artefakte; der Veröffentlichungsjob läuft bei
Pull Requests nicht.

Release-Builds verwenden Rust 1.98.1 und festgelegte, auf der jeweiligen
Plattform verfügbare Python-Patchversionen (3.12.11 unter Linux und 3.12.10
unter Windows) sowie `Cargo.lock`. Jeder Plattformjob baut die CLI zweimal in
getrennten Target-Verzeichnissen und verlangt byte-identische Binärdateien. Der
Windows-MSVC-Build übergibt `/Brepro` an den Linker, damit PE-Zeitstempel
gleichwertige Builds nicht unterschiedlich erscheinen lassen. Das
Packaging-Skript vereinheitlicht
Archivsortierung, Eigentümer, Rechte und Zeitstempel. Seine Tests verlangen bei
wiederholtem Packaging byte-identische Archive und SHA-256-Dateien. Das prüft
die Wiederholbarkeit innerhalb derselben Runner-/Toolchain-Umgebung; es ist kein
unabhängiger, hersteller- oder toolchainübergreifender
Reproduzierbarkeitsnachweis.

## Manuelle Probe ohne Veröffentlichung

In GitHub **Actions → Zelyra Release → Run workflow** öffnen, den zu prüfenden
Branch auswählen und das Release-Tag eingeben, zum Beispiel `v0.2.0`. Der
Workflow prüft das Tag-Format und baut Plattformarchive, eigenständige
Updater-Binärdateien und SHA-256-Dateien. Er lädt sie als Workflow-Artefakte
hoch, erstellt aber weder ein Tag noch ein Release.

Prüfung und Build besitzen im Repository ausschließlich Leserechte. Nur der
durch einen Tag-Push ausgelöste Veröffentlichungsjob erhält die Berechtigung
`contents: write`.

## Veröffentlichen

Nach Review und Merge nach `main` zuerst sicherstellen, dass CI, Datenbanktests
und die Ersteinstiegsprüfung des Release-Kandidaten erfolgreich waren. Danach
das passende Versions-Tag pushen, zum Beispiel:

~~~bash
git switch main
git pull --ff-only
git tag -a v0.2.0 -m "Zelyra 0.2.0"
git push origin v0.2.0
~~~

Das Tag muss `vMAJOR.MINOR.PATCH` entsprechen; optional ist ein
Prerelease-Suffix wie `-alpha.1`. GitHub Actions baut beide unterstützten
Release-Ziele und veröffentlicht das Release erst, wenn beide Builds
erfolgreich waren. Bereits verwendete Release-Tags dürfen nicht wiederverwendet
oder verschoben werden.

Der Release-Workflow stellt derzeit Linux- und Windows-x86_64-Artefakte bereit.
Andere Plattformen gelten dadurch nicht als unterstützt.
