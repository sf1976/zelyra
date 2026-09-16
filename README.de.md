# Zelyra 0.1

Deutsch · [English](README.md)

Zelyra ist eine statisch typisierte Programmiersprache für Business-,
Datenbank- und Webanwendungen. Dieses Repository enthält den Sprachkern der
Phasen 1 und 2 sowie den Database Core der Phase 3: Lexer, Parser, AST, HIR,
statische Typprüfung, Interpreter, Schema-Compiler und die `zelyra`-CLI.

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

Phase 2 ergänzt nominale Typen, `Option`/`Result` und vollständiges Pattern
Matching. Siehe [den deutschen Phase-2-Leitfaden](docs/phase-2.de.md) oder die
[englische Fassung](docs/phase-2.md).

Phase 3 ergänzt den Database Core für MariaDB, PostgreSQL und SQLite. MariaDB
ist das Standard-Backend für neue Projekte. Siehe
[den deutschen Phase-3-Leitfaden](docs/phase-3.de.md) oder die [englische
Fassung](docs/phase-3.md).

Phase 4 ergänzt native SQL-Blöcke mit Schema-, Spalten- und
Parameterprüfung sowie MariaDB-Runtime-Ausführung. Siehe [den deutschen
Phase-4-Leitfaden](docs/phase-4.de.md)
oder die [englische Fassung](docs/phase-4.md).

## Aktuelle Grenzen

SQL, Web, Formulare, CRUD, Capabilities, Contracts und Codegenerierung gehören
zu den folgenden Phasen. Der Database Core unterstützt bereits Schema-DDL,
Inspektion, Diff, Plan und Apply für PostgreSQL, MariaDB und SQLite.

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

Phase 5 stellt jetzt den ersten Web-Core-Schritt bereit: Seitendefinitionen,
HTML-Blöcke, GET-Routen mit Parametern, sicheres HTML-Escaping und den
eingebauten HTTP-Server. Das Beispiel startet ohne Apache:

~~~bash
zelyra serve examples/hello_web.zyl
# http://127.0.0.1:3000/hello/Zelyra öffnen
~~~

Formulare, CRUD, Sessions, CSRF, APIs und datenbankgestützte Seiten folgen in
weiteren Web-Core-Schritten.
