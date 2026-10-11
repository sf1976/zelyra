# Implemented in Zelyra 0.9.2

**Snapshot:** stable 0.9.2 · language compatibility line `0.1` · 2026-10-11
**Maturity:** experimental; not approved for production use

This is an inventory of capabilities implemented in the stable Zelyra 0.9.2
release. It does not claim human acceptance; that remains scheduled for 1.0.0. It records delivered scope, not the full language vision
or future plans. The [roadmap](ROADMAP.md) remains the source for unfinished
work; the [changelog](../CHANGELOG.md) records what changed in each release.
Links below lead to detailed instructions and test evidence so this page does
not duplicate the handbook.

## Changes in 0.9.2

- ✅ Studio provides contextual help and a searchable action finder.
  Completion includes compiler-known project declarations and local AST
  bindings; live diagnostics also check unsaved edits without modifying project
  files.
- ✅ F12 opens unambiguous definitions in the current or a directly imported
  module. Shift+F12 finds lexically identical identifiers in saved `.zyl`
  files; it does not resolve scopes.
- ✅ The deterministic project map displays compiler-known modules, imports,
  tables, views, CRUD resources, pages, APIs, and functions with source
  positions. Ambiguous, dynamic, or limit-truncated relationships are not
  guessed.
- ✅ The explicit Studio **Run** action limits programs to 15 seconds and 64
  KiB per output stream, and redacts recognized secrets from the process
  environment and `.env`. Studio does not run programs automatically and does
  not provide interactive stdin.
- ✅ `zelyra --tutorial invoice` creates the invoice tutorial, starts its local
  MariaDB/web Compose application, and applies its schema. `--no-start` skips
  Docker startup. The example calculates net, 19% VAT, and gross and provides a
  browser-printable invoice view.
- ✅ Automated browser and keyboard checks cover the new editor language
  services. Independent screen-reader, contrast, and product acceptance remain
  scheduled for 1.0.0.

## Changes in 0.9.1

- ✅ Studio restores its local session after a tab reload. It removes the
  session token from the address bar and keeps it for the current tab.
- ✅ Unsaved edits are visible; `Ctrl+S` saves the current editor content.
  Compiler diagnostics appear in the Problems panel.
- ✅ Chromium E2E covers saving, reload, diagnostics, and restart with MariaDB
  and PostgreSQL Studio workflows.
- ✅ The invoice tutorial calculates net, 19% VAT, and gross from linked lines.
  Its print view uses browser PDF export; there is no native PDF engine or legal
  compliance claim.
- ✅ `#` line comments and standalone `##` block delimiters are part of language
  line 0.1. Existing `//` comments remain compatible.

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
  matching as either a statement or expression; a `match` expression returns
  a value whose type is checked consistently across all arms.
- ✅ `Int` and `UInt` arithmetic is bounded to their 64-bit ranges; overflow,
  underflow, and division by zero return runtime errors. The modular
  `examples/type_system` checks supported types and expected compiler/runtime
  errors, and documents partial or missing types. `Decimal` now uses fixed
  point with nine fractional digits and exact base-10 arithmetic; it serializes
  as a decimal string for lossless JSON round trips. `Date`, `Time`, `Duration`,
  and `Bytes` now have runtime values, validating constructors, and JSON round
  trips. Enums, tuples, sets, class identity, and user-defined generics remain
  open.
- ✅ Strings support lexicographic comparisons. `json_decode_result<T>`,
  `read_text_result`, and `write_text_result` return expected parse and file
  errors as `Result`. `format_timestamp` renders UTC timestamps as ISO 8601;
  capability grants and project filesystem boundaries still apply. Exclusive
  file creation, temporary writes with atomic replacement, append, copy, rename,
  and Result-based deletion are available. Literal text search, replacement,
  and splitting support multi-file text workflows. `examples/notes_manager`
  demonstrates UTF-8 notes, pre-change backups, restore, action logging, and
  automated tests.
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
  web/CRUD paths do not use this runtime yet. 0.7.0 adds typed MySQL queries
  tested only against MySQL Server 8.4.11; schema/administration commands,
  migrations, generated CRUD/web paths, and authentication are not included.
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
- 🧪 0.7.0 adds a tenant-aware CRUD path for exactly one explicitly declared
  MariaDB table. Session, membership, permission, and generated queries are
  checked in the supported path. This is not full multi-tenancy; relations,
  custom SQL actions, TableViews, exports, migrations, and backfills are not
  supported.

For usage details see the [web and database quickstart](getting-started.md),
[setup guide](setup-web.md), and [web chapters](handbook/en/handbook.md).

## Zelyra Studio and invoice tutorial

- 🧪 `zelyra editor [directory] [--port <port>] [--no-open]` starts local Studio on
  `127.0.0.1` with a project tree, tabs, search/replace, completion, compiler
  diagnostics, and formatting. The CLI opens the system browser by default;
  `--no-open` skips it. Writes require a session token and same browser origin;
  the database and web application are not started automatically. Version 0.8.0 adds a
  read-only clickable relationship map for supported MariaDB/PostgreSQL
  schemas and a guided CRUD source builder. Version 0.9.0 adds an impact view
  for affected files, tables, declared relationships, and routes. The compiler
  checks a candidate, Studio shows its diff, and writes only after explicit
  apply with a fresh hash check. MySQL live introspection and schema or record
  changes are out of scope here. See the [Studio guide](studio.en.md).
  Version 0.9.2 adds contextual help, an action finder, AST-backed local
  completion, live diagnostics, F12 definitions, lexical Shift+F12 occurrences,
  a deterministic compiler project map, and a bounded explicit Run action.
  Type inference, full semantic references, and dynamic project relationships
  are not included.
- ✅ `zelyra text edit <plan.zedit>` previews text edits from TOML plans using
  linear regular expressions, expected match counts, a source-bound plan ID,
  and explicit apply. Project boundaries and unchanged inputs are checked
  before mutation; see [Text edit](text-edit.en.md) for syntax and limits.
- 🧪 Optional local MariaDB TOTP supports encrypted factors, recovery codes,
  replay protection, lockout, and audit. It is an experimental preview without
  independent production approval.
- The 0.1 language compatibility corpus adds typed API errors and runtime
  cases for the invoice application and `Result`.
- 🧪 `zelyra --tutorial invoice` now creates the app and starts its local
  MariaDB/web Compose stack, including its schema. `--no-start` creates only
  the scaffold. The tutorial guides users through customers, items, a dashboard,
  and invoice creation. The invoice page totals net line prices,
  calculates 19% VAT in this learning example, and shows the gross amount.
  At `/invoices/INVOICE-NUMBER/print`, the browser can save the print-ready
  invoice as a PDF. This is a learning path, not production or deployment
  approval; see the [invoice tutorial](tutorials/invoice.en.md) for its scope
  and limits.
- 🧪 `zelyra --help` ends with German and English getting-started paths for a
  minimal project or invoice tutorial, followed by short explanations of the
  purpose of key commands. `zelyra editor` is shown as a direct launch in the
  current project folder.
- Studio is not yet a complete IDE or unrestricted database designer.
- ✅ `examples/inventory_orders/` provides a modular console project for stock,
  customers, and orders with JSON persistence, cent-exact discount/VAT
  calculations, cancellation, an action log, and runnable business checks.

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

The earlier feature implementation passed CI on PR head `a35a5ee` in run
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
Zelyra 0.9.1 was published from merge commit `aeba34eb73d327cf9d2b65bfaa0e2ce477802af6`. Exact CI, artifact, checksum, SBOM, attestation, and website evidence is in the [0.9.1 release verification record](release-readiness/0.9.1-candidate-verification.md). Independent human acceptance remains reserved for 1.0.0.

This page is updated when a delivered capability or its evidence changes. It
does not turn a specification or roadmap entry into an implementation claim.
