# Zelyra coverage-guided fuzzing

This isolated nightly workspace fuzzes the lexer/parser, SQL named-parameter
binder, generated form renderer, HTTP request parser and multipart decoder, bounded bytecode
artifact decoder plus accepted-artifact round trips, and differential
execution of generated array programs on AST, MIR, and Bytecode. The ordinary stable
workspace and lockfile are unchanged. CI uses pinned `cargo-fuzz` 0.13.2 and
runs each target for 15 seconds from `fuzz/corpus/<target>`.

## Run locally

If the bytecode target corpus is empty, generate a valid seed first:

```sh
cargo run --manifest-path fuzz/Cargo.toml --example bytecode_seed -- \
  fuzz/corpus/bytecode_decode/minimal_main.zbc
```

Install nightly Rust with `rust-src`, Clang, and `cargo-fuzz` 0.13.2, then run:

```sh
for target in lexer_parser sql_binder template_renderer http_request multipart_decoder bytecode_decode backend_differential; do
  cargo +nightly fuzz run "$target" "fuzz/corpus/$target" -- \
    -max_total_time=60 -max_len=16384 -timeout=5
done
```

When a run finds a crash, minimize it before adding it as a regression seed:

```sh
cargo +nightly fuzz tmin TARGET fuzz/artifacts/TARGET/CRASH
```

Review the minimized input, add it to the matching corpus directory, and add a
focused regression test where the bug can be expressed without libFuzzer.
Never delete a failing artifact before its minimized regression has been
reviewed and preserved.
