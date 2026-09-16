# Zelyra 0.1

Deutsch · [English](README.md)

Zelyra ist eine statisch typisierte Programmiersprache für Business-,
Datenbank- und Webanwendungen. Dieses Repository enthält den Sprachkern der
Phase 1: Lexer, Parser, AST, statische Typprüfung, Interpreter und die
`zelyra`-CLI.

## Schnelleinstieg

Für einen ausgecheckten Quellcode ist die einfachste Installation:

```bash
./install.sh
zelyra run examples/fibonacci.zyl
```

Der Installer verwendet das aktuelle Benutzerkonto, installiert Rust nur bei
Bedarf lokal und legt das Programm standardmäßig unter `~/.local/bin` ab. Für
den Sprachkern der Phase 1 sind weder Root-Rechte noch Apache, ein
Datenbankserver oder eine globale Systemkonfiguration erforderlich.

Eine ausführliche Anleitung gibt es im [deutschen Getting-Started-Leitfaden](docs/getting-started.de.md)
oder im [englischen Getting-Started-Leitfaden](docs/getting-started.md).

```bash
cargo run --bin zelyra -- run examples/fibonacci.zyl
```

Erwartete Ausgabe:

```text
55
```

Ein Programm prüfen, ohne es auszuführen:

```bash
cargo run --bin zelyra -- check examples/fibonacci.zyl
```

## Syntax der Phase 1

Variablen sind standardmäßig unveränderlich. Eine Variable wird mit
`name = wert` oder einer expliziten Typangabe eingeführt. Veränderliche
Variablen werden mit `mutable` markiert:

```zelyra
fn main() {
    name: String = "Zelyra"
    mutable counter = 0

    while counter < 3 {
        print(name)
        counter = counter + 1
    }
}
```

Unterstützt werden primitive Werte, Funktionen, Aufrufe, arithmetische und
boolesche Ausdrücke, `if`/`else`, `while`, `loop`, `break`, `return` und
`print`.

## Aktuelle Grenzen

Die Implementierung der Phase 1 ist ein compilerartiger Prototyp mit
Interpreter. Datenbank, SQL, Web, Formulare, CRUD, Capabilities, Contracts und
Codegenerierung sind noch nicht enthalten. Diese Funktionen gehören zu den
späteren Phasen der Spezifikation.

## Prinzip für eine einfache Bereitstellung

Webanwendungen sollen während der Entwicklung mit einem einzigen Zelyra-Befehl
starten können. Ein Zelyra-Projekt darf Apache nicht voraussetzen. Geplant
sind:

```text
zelyra dev                 Eingebauter Entwicklungsserver
zelyra serve               Eigenständiger Produktionsserver
zelyra web apache          Apache-Reverse-Proxy-Konfiguration erzeugen
```

Apache bleibt eine optionale Integration für bestehende Infrastruktur. Die
gleiche Anwendung soll auch hinter nginx, Caddy, einem Cloud-Load-Balancer
oder direkt über den Zelyra-Server betrieben werden können. Diese Befehle sind
bewusst als Roadmap dokumentiert, bis die Web-Core-Phase sie implementiert.
