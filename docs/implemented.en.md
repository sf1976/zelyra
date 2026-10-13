# Implemented in Zelyra 0.9.9

**Snapshot:** 0.9.9 development branch · language compatibility line `0.1` · 2026-10-12
**Maturity:** pre-release engineering work; not approved for production use

This is a development snapshot, not a published 0.9.9 release. Its remaining
technical release gates are listed in the [0.9.9 plan](release-plans/0.9.9.en.md).
Human acceptance remains reserved for the exact 1.0.0 candidate.

This inventory describes what the current Zelyra release can do. Capabilities
are grouped by use, not by historical release number. Older and newer releases
are listed in the [changelog](../CHANGELOG.md); unfinished work is tracked in
the [roadmap](ROADMAP.md). Independent human acceptance is reserved for 1.0.0.

## What the 0.9.9 development branch provides

- **Language and compiler:** a statically checked language with functions,
  generic records, enums, and classes, named function values, typed collections,
  `Option` and `Result`, pattern matching, modules, contracts, effects, and
  compiler-checked SQL/web declarations. Explicit scalar text formatting,
  calendar-day operations, Decimal rounding, and controlled recursion limits
  are included. See
  [Language and compiler](#language-and-compiler) for the supported subset and
  its limits.
- **Databases:** schema-aware queries, relational foreign keys, database
  inspection, and checked setup/migration plans. MariaDB is the primary runtime
  reference. MySQL 8.4.11 has an isolated schema workflow as well as
  parameterized queries; PostgreSQL is verified for the workflows listed in
  the [support matrix](database-support.en.md).
- **Business applications:** pages, reusable views, forms, table views, search,
  filtering, sorting, pagination, CRUD, and typed APIs with OpenAPI output.
- **Zelyra Studio:** a local web editor with a persisted light/dark appearance,
  compiler diagnostics, formatting, navigation, completion, task help, a
  database workbench, and a project map. Startup includes a short branded
  splash; the CLI uses a terminal palette when enabled by its output mode.
  The map shows known relationships and explains unresolved or bounded links
  rather than guessing.
- **Invoice tutorial:** a multi-file customer, item, and invoice application
  with metrics from stored records. Tabler Core 1.6.1 is loaded locally as the
  CSS baseline. The dashboard, CRUD paths, and printable net, 19% tax, and gross
  view are designed for desktop, narrow screens, and print. Totals include
  drafts and are not posted revenue.
- **Operations and security:** CLI project creation and setup, route inventory,
  formatting, diagnostics, database/schema tools, audit, and release checks.
  Authentication, roles, tenant-filtered CRUD, optional local MFA, and SMTP mail
  through a transactional MariaDB outbox are available within their documented
  scope.
- **Migration:** static Laravel reports and an explicitly requested separate
  Zelyra scaffold with source locations and unresolved mappings. It does not
  claim business or security parity.

The editor and tools can run locally. Required cloud services, telemetry, and
external AI providers are not part of the product. Zelyra remains experimental:
CI and automated checks do not replace human product or security acceptance.

## Status legend

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
  a value whose type is checked consistently across all arms. `class` instances
  preserve checked reference identity across assignments, parameters, returns,
  and collections; typed field updates mutate the shared instance. Records
  remain value-based. Classes explicitly have no method or inheritance syntax,
  cyclic reference graphs, or direct JSON, HTTP, or SQL serialization; API data
  is exchanged through records.
- ✅ `Int` and `UInt` arithmetic is bounded to their 64-bit ranges; the direct
  literal `-9223372036854775808` is supported. Overflow,
  underflow, and division by zero return runtime errors. The modular
  `examples/type_system` checks supported types and expected compiler/runtime
  errors, and documents partial or missing types. `Decimal` uses fixed point
  with nine fractional digits; `decimal_quantize_result` applies explicit
  rounding without a Float intermediary, while multiplication/division may
  truncate beyond nine fractional digits. Decimal serializes as an exact
  decimal string. Calendar-day addition/difference, ordered Timestamp
  comparisons, and checked UTC-duration arithmetic are available; time zones
  and local DST are not. `Date`, `Time`, `Duration`,
  and `Bytes` now have runtime values, validating constructors, and JSON round
  trips. `List<T>` aliases `T[]`; heterogeneous `Tuple<A, B, ...>` and
  deterministic `Set<T>` values can be created, checked, iterated, and passed
  through JSON. User-defined enums support payloads, generics, equality, JSON,
  and exhaustive matching. User-defined generic functions, records, and enums
  accept explicit or inferred type arguments; general constraints, generic
  aliases, and generic function references are unavailable. Named typed
  function values are implemented; closures remain out of scope. `record_with` and
  `array_replace_result` return checked new values with value semantics.
  `for i in start..end` iterates with an exclusive end.
- ✅ Non-Unit functions must return a compatible value on every reachable path
  (`E-TYPE-RETURN-001`); non-exhaustive matches report
  `E-TYPE-MATCH-001`. Nested calls have a controlled runtime depth limit;
  `zelyra run --max-steps <n>` opts into a statement/loop step budget, which is
  unlimited by default.
- ✅ Strings support lexicographic comparisons. `format_text(value)` explicitly
  formats scalar values without implicit coercion; fixed-scale monetary text
  uses `format_scaled_result`. Collections, records, classes, enums, `Result`,
  and functions have no display conversion. `json_decode_result<T>`,
  `read_text_result`, and `write_text_result` return expected parse and file
  errors as `Result`. The explicit tagged Result codec,
  `json_encode_tagged_result` / `json_decode_tagged_result<T,E>`, preserves the
  Ok or Err variant and its payload without changing existing `json_encode`
  output. `format_timestamp` renders UTC timestamps as ISO 8601;
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
- 🧪 Typed text/HTML application messages can be queued transactionally in a
  MariaDB outbox and delivered through SMTP with authentication, STARTTLS,
  implicit TLS, or explicitly configured private CA roots. A five-second
  transport timeout, message bounds, and a loopback test sink are covered.
  Delivery is at least once; exactly-once delivery, PostgreSQL outbox, named
  templates, and file attachments are not promised in 0.9.9.
- ✅ SQLite provides tested local schema workflows. PostgreSQL supports
  selected runtime, schema, and safety checks. MySQL Server 8.4.11 supports
  typed parameterized queries and an isolated schema workflow covering setup,
  inspection, read-only project map, reviewed column plan, explicit apply, and
  history. None of these results establishes complete
  backend parity; see the exact [compatibility matrix](database-support.en.md).
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
- 🧪 Tenant-filtered CRUD checks authentication, membership, permission, and
  generated database access in its documented MariaDB scope. This is not
  general multi-tenancy for arbitrary SQL, exports, migrations, or jobs.

For usage details see the [web and database quickstart](getting-started.md),
[setup guide](setup-web.md), and [web chapters](handbook/en/handbook.md).

## Zelyra Studio and invoice tutorial

- 🧪 `zelyra editor [directory] [--port <port>] [--no-open]` starts local Studio on
  `127.0.0.1` with a project tree, tabs, search/replace, completion, compiler
  diagnostics, formatting, and a stored light/dark appearance. Theme controls
  are keyboard-operable; browser checks cover contrast and the 360 CSS-pixel
  layout. The CLI opens the system browser by default;
  `--no-open` skips it. Writes require a session token and same browser origin;
  the database and web application are not started automatically. The editor
  includes file search, local completion, formatting, compiler diagnostics,
  contextual help, an action finder, definition navigation, lexical occurrences,
  and an explicit bounded Run action. A deterministic project map shows
  resolvable relationships and reports analysis limits. The database workbench
  compares schema metadata, displays foreign keys, loads records only on request
  (at most 25 per page), and can apply simple column changes as checked source
  previews. It does not apply migrations. MySQL live introspection, full
  semantic references, and an unrestricted visual database designer are not
  included. See the [Studio guide](studio.en.md).
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

## Laravel Migrator

- 🧪 `zelyra migrate laravel <project> --report <path>` statically inventories
  recognized Laravel patterns. PHP, Artisan, databases, and `.env`
  files are not run, contacted, or read. The new report is written outside the
  source project and includes source locations, status, and next steps in
  Markdown or JSON. The scanner covers Composer constraints, simple
  routes, models, controllers, `Schema::create`, selected relationships,
  validation entry points, jobs, Blade views, and integration calls. The 0.9.8
  analyzer version is separate from the 0.9.6 mapping ruleset; fixture coverage
  names syntactic findings without claiming a route-to-data graph or behavior
  transfer. It is an experimental pattern scanner, not a PHP parser or proof of
  business/security parity. Its separate compiler-checked scaffold contains no
  copied business behavior. [Migrator details](laravel-migrator.en.md).

## APIs, authentication, permissions, and audit

- ✅ Typed API declarations validate supported inputs, outputs, and errors;
  JSON serialization, OpenAPI 3.0.3 generation, and TypeScript client
  generation are available through `zelyra doc`.
- ✅ MariaDB-backed password authentication uses Argon2 and persistent
  HttpOnly sessions. Direct and role-derived permissions, guarded actions,
  CSRF-protected browser administration, and last-administrator protection
  are implemented.
- 🧪 Password recovery in the 0.9.9 development branch uses a transactional,
  cross-instance reset limiter. Its table stores an HMAC fingerprint instead
  of an IP address; database failures reject the operation. Independent
  security review and complete release E2E remain open.
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

The stable [Zelyra 0.9.7 release](https://github.com/sf1976/zelyra/releases/tag/v0.9.7)
includes Linux and Windows x86_64 artifacts, checksums, and SPDX SBOMs. Current
backend checks and their limits are listed in the
[database support matrix](database-support.en.md). Independent human product
acceptance remains reserved for 1.0.0; automated evidence does not replace it.

This page is updated when a delivered capability or its evidence changes. It
does not turn a specification or roadmap entry into an implementation claim.

## Exact decimal boundaries in 0.9.9

`amount: Decimal = decimal("119.00")` declares exact fixed point. Forms, HTTP,
and SQL use validated decimal text. MariaDB/MySQL store `Decimal` explicitly
as `DECIMAL(39,9)`, PostgreSQL as `NUMERIC(39,9)`. Legacy or mismatched fixed
scales prevent SQL execution or server startup with source-related diagnostics.
`Money` retains `(19,4)`. Multiplication/division check the final result with a
bounded exact intermediate; remainder at the signed minimum no longer panics.
JSON outputs remain decimal strings. See the [full rules](decimals.en.md) and
[example](../examples/decimal_money.zyl). This does not establish complete
PostgreSQL CRUD/auth parity or production approval.

## Date/time boundaries on the development branch

Calendar-day arithmetic and UTC milliseconds are checked against independent
Python references. Forms/APIs reject invalid calendar/clock values; Timestamp
is bounded to ISO years 0001–9999. API handlers receive real Date/Time/Timestamp/
Duration values. JSON output stays unchanged; generated clients match it.
Native Date/Time/UTC Timestamp parameters and millisecond storage are verified
on MariaDB, MySQL and PostgreSQL. Incompatible schema types/precision fail
with a source position before application use. Explicit
[`sql_decode<T>`](sql-results.en.md) provides typed scalar/record/array results,
cardinality and NULL checks; the legacy raw-row JSON format stays unchanged. See [rules](dates.en.md), [HTTP evidence](release-readiness/0.9.9-calendar-http.en.md)
and [SQL evidence](release-readiness/0.9.9-calendar-sql.en.md).
