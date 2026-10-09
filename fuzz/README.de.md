# Coverage-guided Fuzzing für Zelyra

Dieses isolierte Nightly-Workspace fuzzed Lexer/Parser, SQL-
Named-Parameter-Binder, Formular-Renderer und HTTP-Request-Parser. Das normale
Stable-Workspace und dessen Lockdatei bleiben unverändert. CI verwendet die
festgelegte Version `cargo-fuzz` 0.13.2 und lässt jedes Ziel 15 Sekunden mit
`fuzz/corpus/<target>` laufen.

## Lokal ausführen

Installiere Nightly Rust mit `rust-src`, Clang und `cargo-fuzz` 0.13.2. Starte
dann:

```sh
for target in lexer_parser sql_binder template_renderer http_request; do
  cargo +nightly fuzz run "$target" "fuzz/corpus/$target" -- \
    -max_total_time=60 -max_len=16384 -timeout=5
done
```

Minimiere einen gefundenen Absturz, bevor du ihn als Regression-Seed ablegst:

```sh
cargo +nightly fuzz tmin TARGET fuzz/artifacts/TARGET/CRASH
```

Prüfe die minimierte Eingabe, lege sie im passenden Korpusverzeichnis ab und
ergänze einen gezielten Regressionstest, wenn sich der Fehler ohne libFuzzer
ausdrücken lässt. Lösche ein Fehlerartefakt erst, nachdem die minimierte
Regression geprüft und erhalten wurde.
