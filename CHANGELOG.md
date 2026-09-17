# Changelog

All notable changes to the Zelyra compiler and runtime are documented here.
The language line remains `0.1`; the patch version identifies the compatible
compiler and repository release.

## 0.1.4 — 2026-09-17

### Added

- Generated TypeScript clients now expose declared API error names through
  `ZelyraApiErrorCode`.
- `ZelyraApiError.fromResponse` extracts HTTP status, error code, and server
  message from Zelyra's structured JSON error responses and keeps the raw body.

### Documentation

- TypeScript client error handling documented in English and German.

## 0.1.3 — 2026-09-17

### Added

- `zelyra doc <file.zyl> --typescript` generates a dependency-free TypeScript
  client from API, record, type, and table declarations.
- Generated clients support standard `fetch`, path/query parameters, JSON
  request bodies, bearer tokens, typed responses, and HTTP errors.
- OpenAPI output now reports the compiler package version automatically.

### Dokumentation

- README, Getting Started, Handbook, and Phase 10 documentation updated in
  English and German.

## 0.1.2 — 2026-09-17

### Added

- Structured `for ... in` iteration over arrays.
- Immutable loop variables scoped to the loop body.
- `break` and `continue` support in array loops.
- Parser, HIR, type-checking, runtime, and integration tests for array loops.

### Dokumentation

- README, Getting Started, Handbook, and Phase 10 documentation updated in
  English and German.
- The runnable `examples/array_for.zyl` example demonstrates iteration and
  loop control.

## 0.1.1 — 2026-09-17

### Added

- Array literals, indexing, concatenation, `len`, `append`, `contains`,
  `first`, and `last`.
- Record literals and checked field access, including nested records.
- Safe `Option` results for `first` and `last` on empty arrays.
- Nested record JSON mapping at typed API boundaries and OpenAPI schemas.
- Parser, HIR, type-checking, runtime, CLI, and integration tests for these
  features.

### Documentation / Dokumentation

The README, Getting Started guide, Handbook, and Phase 10 documentation are
available in English and German and describe the new syntax and limitations.

README, Getting Started, Handbook und Phase-10-Dokumentation sind auf Deutsch
und Englisch aktualisiert und beschreiben die neue Syntax sowie die Grenzen.

## Versioning policy / Versionierungsrichtlinie

`0.1` is the language and project compatibility line. Patch releases such as
`0.1.1` contain compatible implementation improvements and additions. A change
to `0.2` will be used when the language contract or compatibility expectations
change materially.

`0.1` bezeichnet die Kompatibilitätslinie von Sprache und Projekt. Patch-
Versionen wie `0.1.1` enthalten kompatible Verbesserungen und Ergänzungen. Auf
`0.2` wird erhöht, wenn sich Sprachvertrag oder Kompatibilität wesentlich ändern.
