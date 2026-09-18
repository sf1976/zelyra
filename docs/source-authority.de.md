# Verbindliche Quellen und Prüfung für Zelyra

Dieses Dokument ist die Quellenlandkarte des Repositorys für verbindliche
Zelyra-Syntax und -Semantik. Es bereitet Zugriff und Prüfung vor; es fügt
keine Syntax hinzu, ändert keinen Compiler und aktiviert keine Roadmap-Vorschläge.

> Zelyra ist eine eigene Sprache. Parser und geprüfte Tests entscheiden, was existiert.

## Reihenfolge der Verbindlichkeit

Bei Widersprüchen muss der Widerspruch ausdrücklich gemeldet werden. Es gilt:

1. [Formale Sprachspezifikation](specification.de.md) und Phasendokumente
2. Lexer-, AST-, Parser-, Namensauflösungs-, Typprüfungs- und semantischer Compiler-Code
3. Offizielle automatisierte Sprach- und Integrationstests
4. Die offizielle Standardbibliothek, sobald eine existiert
5. Offizielle Zelyra-Beispiele, die mit dem aktuellen Compiler erfolgreich geprüft wurden
6. Dokumentation und Handbuch

Die Roadmap ist Planung und keine Syntaxquelle. Fremdsprachen, einschließlich
Rust, sind nur in ausdrücklich gekennzeichneten Vergleichen zulässig und nie
ein Beleg für Zelyra-Syntax.

## Quellenlandkarte des Repositorys

### Formale Spezifikation

- [`docs/specification.de.md`](specification.de.md)
- [`docs/specification.md`](specification.md)
- [`docs/phase-2.de.md`](phase-2.de.md) und [`docs/phase-2.md`](phase-2.md) — Sprachkern und Typen
- [`docs/phase-3.de.md`](phase-3.de.md) und [`docs/phase-3.md`](phase-3.md) — Datenbanken und Schemata
- [`docs/phase-4.de.md`](phase-4.de.md) und [`docs/phase-4.md`](phase-4.md) — natives SQL
- [`docs/phase-5.de.md`](phase-5.de.md) bis [`docs/phase-8.de.md`](phase-8.de.md) — Web, Formulare, CRUD, Auth
- [`docs/phase-9.de.md`](phase-9.de.md) bis [`docs/phase-12.de.md`](phase-12.de.md) — Verifikation und Plattformarbeit

Deutsche und englische Dokumente sind synchronisierte Erklärungen. Bei
abweichender Formulierung müssen Parser und Tests geprüft werden.

### Grammatik und Compilerimplementierung

- [`lexer/src/lib.rs`](../lexer/src/lib.rs) — Tokens und lexikalische Grenzen
- [`parser/src/lib.rs`](../parser/src/lib.rs) — akzeptierte Grammatik und AST-Erzeugung
- [`ast/src/lib.rs`](../ast/src/lib.rs) — Quellmodell und Deklarationen
- [`hir/src/lib.rs`](../hir/src/lib.rs) — Namensauflösung und Lowering
- [`cli/src/main.rs`](../cli/src/main.rs) — Validierung, Diagnosen, semantische Prüfungen
- [`cli/src/edit.rs`](../cli/src/edit.rs), [`cli/src/impact.rs`](../cli/src/impact.rs), [`cli/src/holes.rs`](../cli/src/holes.rs) und [`cli/src/formatter.rs`](../cli/src/formatter.rs) — Maschineninterfaces
- [`database/src/lib.rs`](../database/src/lib.rs) und [`database/src/sql.rs`](../database/src/sql.rs) — Schema und SQL
- [`forms/src/lib.rs`](../forms/src/lib.rs) — Formularvalidierung
- [`web/src/lib.rs`](../web/src/lib.rs) — Web, Views, CRUD, Rendering
- [`runtime/src/lib.rs`](../runtime/src/lib.rs) — Werte, Capabilities, Contracts, Hostfunktionen

Rust-Code in diesen Dateien implementiert Zelyra. Rust-Syntax in einer
`.zyl`-Datei ist nicht deshalb gültig, weil der Compiler in Rust geschrieben ist.

### Offizielle Tests

Die Testquelle verteilt sich auf Unit-Tests in den `src/lib.rs` der Crates,
[`cli/tests/machine_interfaces.rs`](../cli/tests/machine_interfaces.rs),
[`runtime/tests/phase1.rs`](../runtime/tests/phase1.rs) und die
Integrationsskripte in [`tests/`](../tests/). Workspace- und Crate-Manifeste
stehen in [`Cargo.toml`](../Cargo.toml).

Zuerst den engsten relevanten Test, danach den vollständigen Qualitätslauf:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Status der Standardbibliothek

Aktuell existiert kein eigenständiges Verzeichnis `stdlib/`. Das ist eine
dokumentierte Lücke und keine Erlaubnis, eine Rust-ähnliche Standardbibliothek
zu erfinden. Bis dahin müssen Built-ins über HIR, Runtime, Parsertests,
Runtimetests und Spezifikation nachvollzogen werden.

### Offizielle Beispiele

Offizielle Beispiele sind die `.zyl`-Dateien in [`examples/`](../examples/).
Sie sind erst nach erfolgreicher Prüfung mit dem aktuellen Compiler Belege.
Dateien mit Präfix `invalid_` und das Typed-Hole-Beispiel sind absichtlich
negative Beispiele.

Für ein gültiges Beispiel:

```bash
cargo build -p zelyra-cli
target/debug/zelyra check examples/fibonacci.zyl --format=json
target/debug/zelyra fmt examples/fibonacci.zyl --check
```

Bei einem negativen Beispiel müssen erwarteter Exit-Code und Diagnose geprüft
werden. Datenbank- und Web-Beispiele können MariaDB, SQLite, Docker oder
Konfiguration benötigen; statische Prüfung und Runtime-Integration sind
getrennte Aussagen.

### Dokumentation und Handbuch

- [`README.de.md`](../README.de.md) und [`README.md`](../README.md)
- [`docs/handbook/de/README.md`](handbook/de/README.md) und [`docs/handbook/en/README.md`](handbook/en/README.md)
- [`docs/env.md`](env.md) und [`docs/env.en.md`](env.en.md)
- [`docs/architecture/`](architecture/) und [`docs/benchmarks/`](benchmarks/)
- [`docs/ROADMAP.de.md`](ROADMAP.de.md) und [`docs/ROADMAP.md`](ROADMAP.md)

Dokumente müssen Funktionen als implementiert, experimentell, geplant oder
nicht verfügbar kennzeichnen. Ein dokumentierter Vorschlag ist kein Compilerfeature.

## Vorgehen vor dem Schreiben von `.zyl`

1. Relevante Spezifikationsphase bestimmen.
2. Zugehörige Lexer-/Parser- und AST-Definitionen lesen.
3. Relevante positive und negative Tests durchsuchen.
4. Ein aktuelles offizielles Beispiel mit dem lokalen Compiler prüfen.
5. Wird keine Regel gefunden, pausieren und die fehlende Sprachentscheidung nennen.
6. Niemals Rust- oder andere Fremdsyntax als Lückenfüller verwenden.

Nützliche Suchen:

```bash
rg -n "keyword|syntax|parse|type|diagnostic" docs lexer parser ast hir cli tests examples
rg -n "fn test_|#\[test\]|assert!|assert_eq!" parser/src lexer/src hir/src cli/src database/src forms/src runtime/src web/src
```

## Statusbegriffe

- **Implementiert** — vom aktuellen Compiler und relevanten Tests akzeptiert.
- **Spezifiziert, nicht implementiert** — spezifiziert, aber aktuell abgelehnt oder nicht verfügbar.
- **Geplant** — nur in der Roadmap vorhanden.
- **Vorschlag** — neue Designidee mit ausdrücklicher Sprachentwicklungsaufgabe.
- **Unklar** — widersprüchliche oder unzureichende Quellen; Arbeit pausiert.

## Abschlussbericht für Zelyra-Quelländerungen

Anzugeben sind geänderte `.zyl`-Dateien, gelesene Spezifikations- und
Parser-/Testquellen, exakte Prüf-Befehle, Testergebnisse sowie jede
spezifizierte, aber nicht implementierte, geplante, vorgeschlagene oder unklare
Funktion. Ungeprüfter Zelyra-Code darf nicht als gültiger Zelyra-Code bezeichnet werden.
