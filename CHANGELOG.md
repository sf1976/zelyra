# Changelog

All notable changes to the Zelyra compiler and runtime are documented here.
The language line remains `0.1`; the patch version identifies the compatible
compiler and repository release.

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
