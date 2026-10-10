# Implemented in Zelyra 0.6.0

**Snapshot:** stable 0.6.0 · language compatibility line `0.1` · 2026-10-10
**Maturity:** experimental; not approved for production use

This is an inventory of capabilities implemented in the stable Zelyra 0.6.0
release. It does not claim human acceptance; that remains scheduled for 1.0.0. It records delivered scope, not the full language vision
or future plans. The [roadmap](ROADMAP.md) remains the source for unfinished
work; the [changelog](../CHANGELOG.md) records what changed in each release.
Links below lead to detailed instructions and test evidence so this page does
not duplicate the handbook.

## How to read this inventory

- ✅ **Implemented and exercised:** available in the stated scope, with tests
  or a documented release/CI check.
- 🧪 **Implemented in a limited or experimental scope:** the working subset is
  stated explicitly; do not infer broader support.

No item here implies general production readiness, universal portability, or
proof that arbitrary applications are correct.

## Language and compiler

- ✅ Lexer, parser, syntax tree, name and type checking, functions, expressions,
  conditionals, loops, and immutable-by-default bindings; mutation must be
  explicit.
- ✅ Nominal domain types, records and checked field access, arrays and common
  array operations, deterministic typed maps, `Option`, `Result`, and pattern
  matching in the implemented language subset.
- ✅ `zelyra check`, `build`, `run`, and `serve`; human-readable diagnostics,
  stable diagnostic codes in supported machine output, and source locations.
- ✅ Canonical formatting with `zelyra fmt` and non-mutating `--check` mode.
- 🧪 Function capabilities and runtime-checked contracts are implemented for
  supported operations. `zelyra verify` distinguishes `PROVEN`,
  `RUNTIME_CHECK`, `UNPROVEN`, and `FAILED` for a bounded set of expressions and
  paths; it is not a general-purpose theorem prover.

See the [formal specification](specification.md),
[source-authority guide](source-authority.md), and
[language chapters in the handbook](handbook/en/handbook.md).

## Databases and SQL

- ✅ MariaDB is the primary tested runtime backend. The CLI implements
  `zelyra db create|setup|bootstrap|inspect|map|plan|apply|history` for
  supported operations. `zelyra db map <file.zyl>` compares declared tables by
  source module with live tables, columns, and foreign keys. It is read-only;
  module ownership is advisory and does not create authorization rules.
- ✅ Table schemas, columns, keys, relationships, indexes, schema inspection,
  desired/current schema comparison, and SQL DDL planning/application are
  implemented for their documented backend scopes.
- ✅ Native SQL is checked against known schemas for tables, columns,
  parameters, nullability, and result mapping. Parameters are bound safely;
  transactions are available.
- ✅ SQLite has tested schema and local database workflows. PostgreSQL schema
  inspection/planning and selected safety checks are tested. 0.6.0 adds an
  experimental native PostgreSQL 16 runtime slice for direct parameterized SQL
  and transactions. It does not establish full PostgreSQL parity; generated
  web/CRUD paths do not use this runtime yet.
- 🧪 Schema planning classifies selected changes as safe, requiring review,
  destructive, or unsupported. Destructive and unsupported changes fail
  closed; selected populated-table and nullability changes receive read-only
  preflight checks. Review-gated operations require explicit approval.

The exact backend boundaries and tested MariaDB versions are in the
[database compatibility matrix](database-compatibility.en.md).

## Web applications, views, forms, and CRUD

- ✅ Built-in HTTP server, pages, route parameters, typed view interpolation,
  safe HTML escaping, and project-local theme-token overrides. `zelyra routes
  <entry.zyl>` lists recognized page, API, form, tableview, CRUD, and
  authentication routes with HTTP methods and source locations. Imported and
  generated resource routes are included; overlapping paths with the same HTTP
  method fail with `E-ROUTE-001`. JSON output is available.
- ✅ Schema-aware forms with validation, parameterized database actions,
  transactions, redirects, and generated relationship selectors for supported
  cases.
- ✅ Generated CRUD list, detail, create, edit, and delete flows with search,
  typed filters, sorting, pagination, authorization, and CSRF protections.
- 🧪 Reusable named views/layouts, typed component properties, default and
  named slots with fallback content, and CRUD presentation overrides work in
  the documented composition paths. Generated CRUD content is supplied to the
  default slot; supported custom named-slot content is compile-checked static
  markup/components. Project CSS token overrides work, but this is not a full
  theme editor.
- ✅ Built-in German and English catalogs cover generated application UI.
  Project-local locale overlays can override supported catalog-backed labels.
  `ZELYRA_LANGUAGE=de|en` selects language; `ZELYRA_LEVEL=learn|work` toggles
  the contextual learning guide.
- ✅ Generated MariaDB starters include minimal, CRUD, authentication, and
  business-application variants; the machine-management starter includes an
  optional fictional-data fixture.

For usage details see the [web and database quickstart](getting-started.md),
[setup guide](setup-web.md), and [web chapters](handbook/en/handbook.md).

## APIs, authentication, permissions, and audit

- ✅ Typed API declarations validate supported inputs, outputs, and errors;
  JSON serialization, OpenAPI 3.0.3 generation, and TypeScript client
  generation are available through `zelyra doc`.
- ✅ MariaDB-backed password authentication uses Argon2 and persistent
  HttpOnly sessions. Direct and role-derived permissions, guarded actions,
  CSRF-protected browser administration, and last-administrator protection
  are implemented.
- ✅ Optional MariaDB audit logging supports inspection, bounded JSON/CSV
  export, structural verification, and explicitly confirmed pruning.
- 🧪 Tamper-evident audit chaining is available as an explicit opt-in and has
  hash-integrity tests; this does not make the database immutable or replace
  independent audit controls.

## Setup, configuration, and distribution

- ✅ `zelyra new` and `zelyra init` scaffold projects and templates. Generated
  MariaDB projects include Compose configuration and a local `.env`; secrets
  are not printed, newly created Unix `.env` files use mode `0600`, and
  `.env` is excluded from generated Git/Docker contexts.
- ✅ `zelyra setup` supports console actions and a loopback-only browser
  assistant, starts generated local MariaDB/web stacks, applies the initial
  schema, handles selectable ports, and reports the application URL.
- ✅ `zelyra doctor --json` provides read-only readiness checks with stable
  configuration, project, connectivity, authentication, timeout, schema, and tooling
  categories; database diagnostics are normalized without credentials.
  `zelyra update` checks release checksums
  before replacing supported Linux/Windows x86_64 binaries.
- ✅ User-local installation is available from source. Published Linux and
  Windows x86_64 release archives include SHA-256 checksums; repeat builds
  were byte-identical in the pinned release CI toolchains.

Configuration precedence, secret handling, and platform caveats are documented
in the [environment reference](env.en.md) and [setup guide](setup-web.md).

## AI-native compiler interfaces

- ✅ `zelyra check --format=json` and `zelyra context --format=json` provide
  versioned machine-readable output for their documented scope.
- 🧪 `zelyra impact --format=json` provides deterministic, source-only impact
  information for supported declarations and references.
- ✅ `zelyra edit --format=json` previews supported semantic rename edits;
  applying them is explicit and the changed project is compiler-validated.
- ✅ Typed expression holes (`_`) produce contextual diagnostics and prevent
  incomplete programs from building or running.

These tools do not call an AI provider. See the
[AI-native architecture](architecture/ai-native-development.md) and its
[benchmark specification](benchmarks/ai-authoring.md); no comparative
benchmark result is claimed here.

## Test and release evidence

The feature implementation passed CI on PR head `a35a5ee` in run
[#38057534730](https://github.com/sf1976/zelyra/actions/runs/38057534730):
workspace tests, four MariaDB compatibility jobs, PostgreSQL 16 runtime
integration, fuzzing, dependency audit, and Linux/Windows CI. Package validation
[#38057534736](https://github.com/sf1976/zelyra/actions/runs/38057534736) built
deterministic Linux/Windows packages and verified checksums and SPDX SBOMs on
the same feature head. Those runs verified the feature implementation before the version bump. The
version-bumped candidate then passed CI #38058823265. The immutable tag
`v0.6.0-rc.1` passed tag CI #38059555233; RC release workflow #38059555140 built the candidate packages. Stable release
workflow #38060275164 built Linux and Windows packages, verified checksums and
228-package SPDX SBOMs, and published GitHub attestations. Downloaded release assets passed checksum and
SBOM verification; the Linux binary reports `zelyra 0.6.0`. See the
[release verification record](release-readiness/0.6.0-candidate-verification.md).
Independent human acceptance remains scheduled for 1.0.0.

This page is updated when a delivered capability or its evidence changes. It
does not turn a specification or roadmap entry into an implementation claim.
