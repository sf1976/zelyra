# Zelyra source authority and verification guide

This document maps the binding sources of Zelyra syntax and semantics. It
prepares source access and review; it does not add syntax, change the compiler,
or make roadmap proposals executable.

> Zelyra is its own language. The parser and verified tests decide what exists.

## Authority order

When sources disagree, report the disagreement explicitly and use this order:

1. [Formal language specification](specification.md) and phase documents
2. Lexer, AST, parser, name-resolution, type-checking, and semantic compiler code
3. Official automated language and integration tests
4. The official standard library, when one exists
5. Official Zelyra examples that pass the current compiler
6. Documentation and handbook explanations

The roadmap is planning information, not a syntax authority. Other languages,
including Rust, are comparison material only and never evidence for Zelyra.

## Repository source map

### Formal specification

- [`docs/specification.md`](specification.md)
- [`docs/specification.de.md`](specification.de.md)
- [`docs/phase-2.md`](phase-2.md) and [`docs/phase-2.de.md`](phase-2.de.md) — language core and types
- [`docs/phase-3.md`](phase-3.md) and [`docs/phase-3.de.md`](phase-3.de.md) — databases and schemas
- [`docs/phase-4.md`](phase-4.md) and [`docs/phase-4.de.md`](phase-4.de.md) — native SQL
- [`docs/phase-5.md`](phase-5.md) through [`docs/phase-8.md`](phase-8.md) — web, forms, CRUD, auth
- [`docs/phase-9.md`](phase-9.md) through [`docs/phase-12.md`](phase-12.md) — verification and platform work

German and English documents are synchronized explanations. If their wording
differs, consult the parser and tests before drawing a conclusion.

### Grammar and compiler implementation

- [`lexer/src/lib.rs`](../lexer/src/lib.rs) — tokens and lexical boundaries
- [`parser/src/lib.rs`](../parser/src/lib.rs) — accepted grammar and AST construction
- [`ast/src/lib.rs`](../ast/src/lib.rs) — source model and declarations
- [`hir/src/lib.rs`](../hir/src/lib.rs) — name resolution and lowering
- [`cli/src/main.rs`](../cli/src/main.rs) — validation, diagnostics, semantic checks
- [`cli/src/edit.rs`](../cli/src/edit.rs), [`cli/src/impact.rs`](../cli/src/impact.rs), [`cli/src/holes.rs`](../cli/src/holes.rs), and [`cli/src/formatter.rs`](../cli/src/formatter.rs) — machine interfaces
- [`database/src/lib.rs`](../database/src/lib.rs) and [`database/src/sql.rs`](../database/src/sql.rs) — schema and SQL
- [`forms/src/lib.rs`](../forms/src/lib.rs) — form validation
- [`web/src/lib.rs`](../web/src/lib.rs) — web, views, CRUD, rendering
- [`runtime/src/lib.rs`](../runtime/src/lib.rs) — values, capabilities, contracts, host functions

Rust code in these files implements Zelyra; Rust syntax in a `.zyl` file is
not valid merely because the compiler is written in Rust.

### Official tests

The test authority is distributed across crate unit tests in `src/lib.rs`,
[`cli/tests/machine_interfaces.rs`](../cli/tests/machine_interfaces.rs),
[`runtime/tests/phase1.rs`](../runtime/tests/phase1.rs), and the integration
scripts in [`tests/`](../tests/). Workspace and crate manifests are rooted at
[`Cargo.toml`](../Cargo.toml).

Run the narrowest relevant test first, then the complete quality gate:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Standard library status

There is currently no standalone `stdlib/` directory. This is a documented
gap, not permission to invent a Rust-like standard library. Until one is
formally introduced, trace built-ins through HIR, runtime, parser tests,
runtime tests, and the specification.

### Official examples

Official examples are the `.zyl` files in [`examples/`](../examples/). They
are evidence only after passing the current compiler. Files beginning with
`invalid_` and the typed-hole example are deliberately negative examples.

For a valid example:

```bash
cargo build -p zelyra-cli
target/debug/zelyra check examples/fibonacci.zyl --format=json
target/debug/zelyra fmt examples/fibonacci.zyl --check
```

For a negative example, verify the expected non-zero exit code and diagnostic.
Database and web examples may require MariaDB, SQLite, Docker, or environment
configuration; static checking and runtime integration are separate claims.

### Documentation and handbook

- [`README.md`](../README.md) and [`README.de.md`](../README.de.md)
- [`docs/handbook/en/README.md`](handbook/en/README.md) and [`docs/handbook/de/README.md`](handbook/de/README.md)
- [`docs/env.md`](env.md) and [`docs/env.en.md`](env.en.md)
- [`docs/architecture/`](architecture/) and [`docs/benchmarks/`](benchmarks/)
- [`docs/ROADMAP.md`](ROADMAP.md) and [`docs/ROADMAP.de.md`](ROADMAP.de.md)

Documentation labels features as implemented, experimental, planned, or
unavailable. A documented proposal is not a compiler feature.

## Review procedure before writing `.zyl`

1. Identify the relevant specification phase.
2. Read the corresponding lexer/parser and AST definitions.
3. Search the relevant positive and negative tests.
4. Check a current official example with the local compiler.
5. If no rule is found, stop and name the missing language decision.
6. Never fill the gap by copying Rust or another language.

Useful searches:

```bash
rg -n "keyword|syntax|parse|type|diagnostic" docs lexer parser ast hir cli tests examples
rg -n "fn test_|#\[test\]|assert!|assert_eq!" parser/src lexer/src hir/src cli/src database/src forms/src runtime/src web/src
```

## Status vocabulary

- **Implemented** — accepted and checked by the current compiler and tests.
- **Specified, not implemented** — specified but rejected or unavailable now.
- **Planned** — present only in the roadmap.
- **Proposal** — a new design requiring an explicit language task.
- **Unclear** — conflicting or insufficient authority; work pauses.

## Completion report for Zelyra source changes

Every source change should report changed `.zyl` files, specification rules
and parser/test sources consulted, exact validation commands, test results,
and any specified-but-unimplemented, planned, proposed, or unclear function.
Unverified Zelyra code must not be described as valid Zelyra code.
