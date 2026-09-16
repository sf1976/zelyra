# Mit Zelyra starten

Deutsch · [English](getting-started.md)

Zelyra ist so konzipiert, dass auch Einsteiger mit möglichst wenig
Systemkonfiguration vom Download zu einem laufenden Programm gelangen.

## 1. Installieren

Repository herunterladen oder klonen, in das Verzeichnis wechseln und
ausführen:

```bash
./install.sh
```

Der Installer arbeitet lokal für den aktuellen Benutzer. Er benötigt kein
`sudo`, kein Apache, keine Datenbank und keine vorhandene Rust-Installation.
Wenn Rust fehlt, wird die offizielle stabile Toolchain über `rustup` lokal für
den aktuellen Benutzer installiert.

Das Programm wird standardmäßig unter `~/.local/bin/zelyra` installiert. Falls
dieses Verzeichnis noch nicht im `PATH` enthalten ist, einmalig ergänzen:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Für Entwickler und Mitwirkende lautet der entsprechende Entwicklungsbefehl:

```bash
cargo run -p zelyra-cli -- run examples/fibonacci.zyl
```

## 2. Erstes Programm ausführen

```bash
zelyra run examples/fibonacci.zyl
```

Das Ergebnis ist:

```text
55
```

Mit `zelyra check program.zyl` kann ein Programm geprüft werden, ohne es
auszuführen.

## 3. Webserver-Konzept

Die Webplattform wird einen eingebauten Entwicklungsserver bereitstellen.
Neue Benutzer können damit Anwendungen starten, ohne Apache, PHP, einen
separaten Frontend-Server oder einen Reverse Proxy installieren zu müssen.

Für den Produktivbetrieb wird Zelyra zwei gleichwertige Wege unterstützen:

1. den eigenständigen Zelyra-Server direkt ausführen;
2. Zelyra hinter einem vorhandenen Apache, nginx, Caddy oder Cloud-
   Load-Balancer betreiben.

Wenn gewünscht, erzeugt Zelyra die Apache-Konfiguration automatisch. Apache
ist dabei ein Adapter und keine verpflichtende Abhängigkeit der
Sprachlaufzeit. TLS, Prozessüberwachung, Firewall-Regeln und
Datenbankzugangsdaten bleiben ausdrücklich sichtbare Betriebsaufgaben.

Die Befehle `zelyra dev`, `zelyra serve` und `zelyra web apache` sind für die
Web-Core-Phasen vorgesehen. Phase 1 enthält bewusst noch keinen Webserver.

## 4. Anforderungen an zukünftige Installer

Release-Versionen sollen plattformspezifische eigenständige Programme
bereitstellen, damit Endbenutzer Rust nicht selbst installieren müssen.
Paketmanager und Container-Images können später hinzukommen. Der erste
Einstieg soll jedoch kurz bleiben:

```text
download → zelyra new meine-app → zelyra dev
```

Für eine benutzerlokale Installation soll kein Kontopasswort erforderlich
sein.

## 5. Dokumentationssprache

Die zentrale Dokumentation wird immer auf Deutsch und Englisch gepflegt. Neue
Dokumente erhalten eine gleichnamige deutsche Variante mit dem Suffix `.de.md`
oder verlinken auf eine zweisprachige Fassung. Änderungen an Installation,
CLI und Benutzerführung müssen in beiden Sprachversionen nachgezogen werden.
