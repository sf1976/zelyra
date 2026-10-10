Warning: truncated output (original token count: 14079)
Total output lines: 883

# Zelyra Roadmap

This is the maintained roadmap for Zelyra. It records both required work and
optional ideas. An item is not considered complete until it has syntax or API
documentation, implementation, positive and negative tests, diagnostics, and a
working example where applicable.

Status legend:

- ✅ implemented (`[x]`)
- 🧪 partially implemented or experimental (`[~]`)
- 🗺️ planned (`[ ]`)
- ◻️ optional or under evaluation (`[?]`)
- ⛔ blocked or deliberately deferred (document the reason beside the item)

The visible emoji makes the status unambiguous on GitHub; the bracket markers
remain machine-searchable for now.

Digital sovereignty is a cross-cutting product constraint, not a completed
feature: local control, no mandatory cloud or AI provider, no unsolicited
telemetry, explicit effects, portable data, resource discipline, and honest
proof claims guide every phase. The [manifesto](MANIFESTO.md) states the
principles; the roadmap below tracks what is actually implemented.

## Current milestones

- [~] The 0.4 development branch adds administrative listing/revocation of
  persistent sessions, CSRF/permission checks and an audit request event.
  `/account/sessions` also lists and revokes the signed-in user's sessions,
  including in-memory sessions, with CSRF and same-origin checks. In-memory
  sessions expire after 24 hours. A bounded, escaped `User-Agent` label is
  optionally stored when the session table declares `device_label`; it is not a
  verified device identity. Password recovery remains experimental. An
  AES-256-GCM-encrypted MariaDB outbox now retries after SMTP failure and
  process restart; the E2E verifies recovery after restart with no older worker
  active. Delivery is at least once, so the SMTP-acceptance/database-acknowledgement
  crash window can duplicate mail; key loss strands pending messages. Persistent
  throttling and independent security review remain open, so this does not
  complete the account lifecycle milestone.
  Legacy session tables without an `id` column retain their previous behavior;
  administrative listing and revocation still require that column.

- [~] The [Docker module acceptance](module-docker-acceptance.en.md) checks
  the combined application, separate invoice/inventory exports, missing `.env`,
  excluded routes and denied MariaDB access. Fixture accounts are read-only;
  full writable CRUD acceptance remains open.

- [~] The 0.4 release workflow creates deterministic SPDX-2.3 SBOMs for Linux
  and Windows and verifies each binary hash. CI runs pinned `cargo-deny` over
  Linux and Windows dependency graphs. Its policy checks advisories, licenses,
  sources, wildcard requirements, and duplicate versions; duplicate `base64`
  and `getrandom` versions warn, and `rustls-pemfile` has a documented
  unmaintained advisory exception. Human dependency/license review, a
  published-candidate attestation rehearsal, and native artifact review remain
  open. Tagged release builds attest both platform archives and their SBOMs;
  see the [verification guide](release-readiness/artifact-verification.en.md).

- [~] The HTTP server handles up to 64 connections concurrently and drops
  excess connections instead of creating an unbounded worker queue. Connection
  reads and writes are limited to 30 seconds and late responses are dropped.
  MariaDB pool waits and statements are now bounded by the remaining exchange
  deadline, and timed-out statements are aborted server-side. CPU-bound or other
  blocking handler work remains synchronous and can still occupy a worker slot;
  the [HTTP operations guide](http-operations.en.md) defines TCP listener reachability,
  the built-in process liveness route, application-owned readiness, and bounded
  retries only for safe/idempotent requests. The generated MariaDB business
  project now includes an application-owned readiness query; its E2E covers
  reachable and unreachable MariaDB. Other dependencies remain application-
  owned; the server does not retry automatically.

- [x] Language core: lexer, parser, AST, functions, expressions, control flow,
  immutable-by-default bindings, arrays, deterministic typed maps, records,
  Option, Result, and pattern matching.
- [x] Static checking, nominal domain types, capabilities, contracts, runtime
  checks, and an initial verifier.
- [x] MariaDB-first schema definitions, inspection, DDL planning, schema
  application, SQLite support, and PostgreSQL schema support.
- [x] Typed native SQL with schema, column, parameter, nullability, result,
  transaction, and safe parameter checks.
- [x] HTTP server, pages, forms, CRUD, search, filtering, sorting, pagination,
  APIs, OpenAPI, and TypeScript client generation.
- [~] Reusable web views: named layouts, page composition, a validated content
  slot, and typed self-closing components with properties are available.
  Named slots with fallback content, CRUD view overrides, and per-resource
  content for named CRUD-layout slots are available. Static slot markup may
  use checked components; record-bound custom slot content remains open.
  Project-local CSS token overrides are implemented, while full theme
  authoring and selection remain open.
- [~] Built-in German/English UI catalogs are selected by `ZELYRA_LANGUAGE`,
  and `ZELYRA_LEVEL=learn|work` controls the contextual learning guide. The
  minimal and machine-management MariaDB starters and generated CRUD, form,
  tableview, login, and authentication-admin pages have a responsive Zelyra
  application shell by default. Explicit CRUD layouts take precedence, and
  authored pages remain untouched. Project-local `locales/de.json` and
  `locales/en.json` overlays can add or override all catalog-backed generated
  shell, CRUD, form, tableview, login, authentication-admin, validation, and
  learning-guide copy, as well as explicitly marked view/text references.
  Parameterized labels and generated field identifiers are supported; resolved
  values are escaped and German falls back through the project English catalog
  to built-in translations. Business records and unmarked user-authored content
  are not translated. Broader template coverage and a full theme editor remain
  open. Project-local CSS overrides for documented visual tokens are available.
- [x] Authentication, persistent sessions, Argon2 passwords, direct and
  role-based permissions, browser administration, and MariaDB audit logging.
  Browser writes also require same-origin evidence; the 0.2.0 release adds a
  default loopback Host allowlist to reject forged hosts and DNS rebinding.
- [~] Audit operations: inspect, JSON/CSV export, structural verification, and
  confirmed pruning are available. Tamper-evident chaining is available as an
  explicit opt-in; archival and retention remain.

## Cross-cutting: AI-native development

The strategic product goal is: **AI writes. Zelyra verifies.** People and AI
systems are equal code authors, but the compiler, tests, capabilities, and
security rules remain authoritative. This is an AI-native and AI-independent
architecture requirement for every phase, not a provider-specific feature.

- [~] **Stage A — machine foundation:** versioned JSON diagnostics for
  `check --format=json`, stable codes, source spans, deterministic output, a
  read-only `context --format=json` project summary, and machine-format tests
  are implemented. Project files use project-relative portable paths; other
  commands and complete secret-redaction coverage remain. The unreleased 0.4
  development branch also includes import edges and the currently supported
  public functions, types, and records per file (`modules[].exports`) in its
  deterministic module context. `zelyra module plan` also provides a read-only
  closure of explicit imports and statically recognized references from the
  impact graph, starting either from a source module or a supported
  page/API/CRUD/form/tableview resource root. It lists declaration inventories
  for included source files and distinguishes the statically reachable
  declaration closure from additional declarations in those files. This
  analysis is incomplete; `configuration_edges` reports database configuration
  separately, and `database.configurations` identifies the declared backend,
  logical database name, source file, and preferred
  `ZELYRA_DATABASE_<NAME>_URL` runtime setting (`DATABASE_URL` remains a
  fallback). The runtime still has one project-wide connection; this name
  mapping is not a reusable multi-database interface. Known
  page-to-view/component, page-SQL and
  form/CRUD-action-SQL-to-table, API-handler-to-function, type references in
  API fields, function signatures and bodies (including explicit local types,
  record literals, and SQL result types), records, and aliases (including
  aliases referenced by table columns and typed resource fields),
  protected-resource-to-authentication,
  authentication-to-table,
  table-relation, and database-configuration edges, and reports references it
  cannot resolve. SQL-to-table edges additionally report conservative
  `read`, `write`, `read_write`, or `unknown` access modes; complex joined
  `UPDATE`/`DELETE` forms remain `unknown`. `schema_ownership` reports only an
  inferred table owner based on declaration source and explicitly sets
  `enforced` to `false`. This is observational metadata, not permission or
  ownership enforcement; unrecognized SQL forms may be absent. Dynamic or
  unmodeled dependencies, assets, runtime
  configuration, external service contracts, and Docker packaging remain
  outside this preview. `complete_deployment` remains `false`; it is neither a
  complete export manifest nor part of published 0.3.0.
- [x] **Stage B — canonical source:** deterministic `zelyra fmt` formats
  parseable source, supports `--check` for CI, preserves comments and raw
  SQL/HTML bodies, and has idempotence and semantic-preservation coverage.
- [~] **Stage C — typed gaps:** expression holes written as `_` now report
  contextual expected types, visible values/functions, capabilities, contract
  obligations, and source spans; incomplete code cannot build or run. Typed
  holes in more declaration contexts and richer edit integration remain.
- [~] **Stage D — impact analysis:** a deterministic source-only
  `zelyra impact --format=json` slice now reports tables, SQL, forms, CRUD,
  views, APIs, permissions, contracts, and a structured deterministic
  `references` edge list. A focused `--symbol <kind:name>` query is available
  for directly connected references; emails, jobs, tests, and live schema
  changes remain to be connected.
- [~] **Stage E — semantic edits:** a validated, atomic rename operation for
  declared functions, types, records, tables, tableviews, forms, CRUDs, views,
  and components is available as a versioned preview and explicit `--apply`.
  Project-local `.zyl` boundaries, stale-source fingerprints, and full
  compiler validation before and after the edit are enforced; richer
  scope-aware operations remain. Function, type, and record renames now
  resolve declarations and known references through the AST without changing
  shadowing local bindings. Table renames also update checked SQL table
  positions while preserving literals, comments, parameters, and HTML; richer
  cross-resource reference resolution remains open.
- [~] **Stage F — contracts and effects:** contracts and capability checks
  exist; granular effects such as `Database(read)`, `Database(write)`,
  `Email`, and `Jobs` remain planned. AI must never add an effect silently.
- [ ] **Stage G — benchmark:** establish a reproducible AI-authoring
  benchmark before making comparative suitability claims. Results are empty
  until real controlled experiments exist.

## 1. Beginner experience and distribution

- [~] Simple defaults with optional, documented project feature switches are
  available through `zelyra.toml`, `.env`, and process overrides; broader
  profile management and interactive configuration remain open. The complete
  setting reference is maintained in `docs/env.en.md` and `docs/env.md`.

- [~] One-command source installation for Linux, Windows, and macOS is
  available with user-local, repeatable Bash/PowerShell installers; Rust-free
  release installation is available for published Linux/Windows x86_64 assets.
- [~] Release archives with SHA-256 checksums are available for Linux and
  Windows x86_64. The public [0.2.0 release][release-020] includes pinned
  Linux/Windows builds, byte-identical repeated binaries, normalized archives,
  and SHA-256 sidecars. Signed binaries and checksums for every supported
  platform remain.

[release-020]: https://github.com/sf1976/zelyra/releases/tag/v0.2.0
- [~] User-local install/check/update/uninstall scripts remain available.
  `zelyra update [--check]` also checks stable GitHub releases and verifies a
  SHA-256 checksum before replacing its own Linux/Windows x86_64 executable;
  Windows replacement is staged until the running process exits. The release
  workflow is configured to publish standalone update assets with future
  releases; automatic updates for other targets remain unavailable.
- [~] A deterministic first-run project flow supports MariaDB scaffolding,
  secure local `.env` creation directly during `zelyra new` and `zelyra init`,
  extensively commented optional settings, and validated selectable ports;
  free host ports are selected automatically for new projects and when setup
  creates a missing `.env` while defaults are occupied; explicit setup port
  choices remain strict and existing `.env` files remain protected. Interactive
  connection configuration remains open.
- [~] A shared console/browser setup assistant is available through
  `zelyra setup --database|--schema|--all` and `zelyra setup --web`; it starts
  generated MariaDB Compose projects and applies the initial schema; console
  and browser actions are now HTTP-/integration-tested on isolated ports, with
  platform-specific Docker installation guidance when Compose is missing,
  compose-command detection, safe Docker-permission/port-conflict errors, and
  actionable Linux group refresh and access verification steps.
  Docker installation, remote administration, and production deployment remain
  intentionally outside the assistant.
- [~] The generated Docker Compose template starts MariaDB and the internal
  web server with independently configurable web and MariaDB host ports plus
  container ports. `tests/generated-project-docker-e2e.sh` now creates a fresh
  CRUD project and runs `zelyra setup --all` twice. It verifies schema-backed
  machine/department pages, the reported local URL, owner-only `.env` mode,
  secret-free output, unchanged credentials across recovery, port mappings,
  and cleanup of only its uniquely named Compose project and volume. Clean-host
  coverage beyond this isolated Docker workflow and production hardening remain
  open.
- [ ] Optional automatic reverse-proxy setup for Apache and Nginx, with safe
  defaults and generated configuration previews.
- [~] `zelyra doctor` checks project validity, database connectivity, Docker
  Compose availability, an optional `.env` without exposing credentials, and
  host-port readiness. JSON checks now carry stable configuration,
  project, connectivity, authentication, timeout, schema, and tooling categories with
  normalized secret-free database errors; TLS, permissions, and broader
  external-tool guidance remain open.
- [~] Project templates: minimal, MariaDB CRUD, MariaDB authentication, and
  MariaDB business starters are available; API and production-deployment
  templates remain.
- [ ] Offline installation bundle and reproducible toolchain metadata.
- [?] Package-manager distribution where practical (Homebrew, winget,
  Debian packages, and container images).

## 2. Language and compiler

- [ ] Stable grammar specification and versioned compatibility rules.
- [~] Experimental function/type/record/table/view/component imports and project-wide database
  configuration in the current development
  branch support project-root-relative imports, `pub` declarations, qualified
  calls and type references, dependency-cycle rejection, project-root/symlink
  containment, per-file source IDs, and
  type/capability/contract checks across imported calls. `check`, `build`,
  `run`, `serve`, `context`, and `verify` validate this graph; machine context now emits
  a deterministic, sorted module/import inventory. Imported database
  definitions and tables join the shared schema; imported views/components
  compose into the application and are rendered through `serve`. Their context
  spans include project-relative file paths. Imported pages join the
  application route set; forms, CRUD declarations, API routes, and authentication
  configuration also compose from imported files. API handler references and
  declared types resolve in their owning module; some template diagnostics and
  verification results still lack complete per-module source
  provenance.
  Database configuration is not addressed through its alias and is
  limited to one connection per project. Imported tables retain global SQL
  names; imported tableview, view, and component names are global, with
  collisions rejected.
  Forms, CRUD declarations, API routes, and authentication configuration compose
  from imported files; auth tables are checked against the shared schema.
  Imported pages join the route set, with overlapping patterns rejected;
  MariaDB-backed tableviews are composed and served. Database commands load
  the linked graph when building the shared schema, and schema-building errors
  retain imported source paths and spans. Template and verification diagnostics
  still need complete module-level source attribution. `impact` analyzes the linked graph
  with file-aware spans; `fmt` and `edit` remain file-local. This
  is not part of the published 0.3.0 binary.
- [ ] Generics, interfaces/traits, enums, ta…5079 tokens truncated…es.
- [ ] Generated SDKs for additional languages where justified.

## 8. Verification, concurrency, and performance

- [ ] More complete contract language for collections, records, errors, and
  database-independent business rules.
- [ ] Proof-result caching and explicit trusted assumptions.
- [ ] SMT/SMT-LIB integration and solver timeout/resource diagnostics.
- [~] Clear separation of `PROVEN`, `RUNTIME_CHECK`, `UNPROVEN`, and `FAILED`
  exists in the CLI and documentation; IDE integration remains planned.
- [ ] Structured concurrency cancellation, timeouts, supervision, and database
  pool integration.
- [ ] Shared-state rules, channels, actors, and data-race testing.
- [ ] Benchmark suite for compile time, startup, routing, SQL, forms, CRUD, and
  memory use.
- [ ] Local, opt-in profiling and diagnostics without hidden collection or
  automatic remote telemetry.
- [?] CP-SAT, MILP, and SMT optimization interface with reproducible solver
  inputs and bounded execution.

## 9. Quality, operations, and governance

- [ ] Full integration matrix for supported OS, database, browser, and runtime
  versions.
- [~] Deterministic bounded mutation regressions exercise the lexer/parser, SQL
  binder, template renderer, and HTTP parser in normal CI (2,048 inputs per
  path). Coverage-guided runs found a parser stack overflow; recursive syntax
  now has a 32-frame recursion limit, focused regressions, and a retained crash
  seed. Four post-fix 60-second local runs passed; the 15-second CI rerun for
  the fixed revision remains open.
- [~] Security regression coverage and dependency/license scanning run in CI.
  Coverage includes credential redaction, network capability enforcement,
  CSRF/origin/host boundaries, release-archive traversal and duplicate-member
  rejection, checksum validation, and atomic updater replacement. A separate
  unsolicited-network/telemetry regression guard and independent security
  review remain open.
- [ ] Regression guard against unsolicited network or telemetry activity;
  explicit application network capabilities and user-initiated update or
  installation commands must remain separately visible.
- [ ] Reproducible release builds, SBOMs, provenance attestations, and signed
  artifacts.
- [x] Release assets can be built for Linux and Windows on relevant pull
  requests and through a manual, non-publishing workflow run; publication is
  restricted to validated version-tag pushes. See the bilingual
  [release guide](releasing.md).
- [ ] Bilingual documentation kept in sync, including migration and upgrade
  guides.
- [ ] Contributor guide, architecture decision records, code of conduct, and
  transparent issue labels.
- [ ] Semantic versioning policy, deprecation window, and compatibility tests.
- [ ] Public alpha, beta, and stable release criteria based on real business
  applications, not only language demonstrations.

## 0.2.0 release milestone

Zelyra 0.2.0 was published on 2026-09-20. This section records its delivered
scope and release evidence; the release is experimental and is not approved
for production use. The release-gate results do not imply completion of the
remaining roadmap items below.

- [~] **Distinctive Views system:** reusable layouts/components, typed CRUD
  presentation controls, and per-resource named layout slots are available.
  Project-local German/English catalogs can add or override all
  catalog-backed generated labels, including parameterized field labels and
  generated identifier labels, as well as explicitly marked view/text
  references. Business records and unmarked authored text remain unchanged;
  deeper theme authoring and record-bound custom slots remain open.
- [~] **Flagship business application:** the MariaDB business template and
  generated-project integration path exist. A database-free CLI/HTTP test now
  covers its generated CRUD form, missing-token and insufficient-API-permission
  denials, authorized access, and project-local German/English copy overrides;
  the MariaDB-backed path in `tests/generated-project-business-e2e.sh` has also
  passed against an isolated test database with cleanup verified. The
  MariaDB machine-management acceptance test now checks localized machine and
  department views in all four German/English × Learn/Work combinations,
  including presence/absence and localization of the learning guide. Novice-
  tested onboarding and broader documented user acceptance remain release
  work. A protected machine-management fixture and generated-project E2E now
  also cover anonymous denial, viewer access, create/edit denial, authorized
  CRUD forms, search, and deletion.
- [~] **Simple first run:** the zelyra new and zelyra setup commands, generated
  protected `.env`, Docker Compose, free-port selection, actionable Docker
  permission guidance, and a printed application URL exist. The generated
  CRUD stack has passed an isolated first-run and repeat-setup test, including
  schema-backed HTTP pages, unchanged credentials, secret-free setup output,
  ports, and cleanup. Setup also explains that minimal `init` projects need no
  database and suggests a likely relative path when an absolute path is
  missing. Generated Docker builds can pin a published tag, branch, or commit;
  the exact reference is fetched and checked out detached. Clean-host
  installation and recovery coverage across supported platforms remain open.
- [~] **Database safety:** MariaDB is the reference runtime and SQLite has
  end-to-end paths. The schema-safety test exercises both backends: destructive
  drops require approval; required columns without defaults, new unique
  constraints, and MariaDB foreign-key additions/removals are marked `REVIEW`;
  adding a required no-default column to a populated table is blocked by a
  read-only preflight before any plan SQL;
  MariaDB/PostgreSQL default drift is marked `REVIEW`; SQLite default changes
  and primary-key/auto-increment changes without supported migrations remain
  `UNSUPPORTED`;
  duplicate rows and orphan rows remain intact when index/foreign-key changes
  fail; unknown external indexes are preserved and untracked foreign-key
  removals fail closed. Unsupported changes fail closed with `E-DB-006`.
  The legacy `--allow-destructive` option does not approve `REVIEW` changes;
  `--allow-risky` is required after reviewing them. MariaDB/PostgreSQL
  nullability changes require review and tightening checks existing NULL rows
  before any SQL; SQLite nullability and type, foreign-key, and unique-
  constraint alterations remain unsupported. Type and nullability drift are
  evaluated independently. The
  generated MariaDB business acceptance test has passed. The [version
  matrix](database-compatibility.en.md) lists four
  MariaDB Community LTS patch images and the core database/CRUD paths they
  test; this is not MySQL or full feature certification. PostgreSQL runtime
  parity, general data backfills, row/lock risk estimates, and operational risk
  analysis remain outside the 0.2.0 claim. PostgreSQL primary-key and
  serial/identity metadata drift is
  detected and refused; migrations for those metadata changes remain
  unsupported.
- [x] **Release evidence:** the bilingual quickstarts, internal security review,
  full branch CI, workspace checks, four-version MariaDB compatibility matrix,
  and database-backed integration tests pass. A fresh generated MariaDB CRUD
  application passed first-run and repeat-setup acceptance in the isolated
  Docker E2E test. Linux and Windows release binaries were rebuilt byte-for-byte
  identically in their pinned CI toolchains, and both release packages passed
  validation. This is technical acceptance for an experimental release, not a
  novice-user study, Windows clean-host onboarding study, production approval,
  or external security audit; those are not claimed.

The 0.2.0 scope does not require a visual drag-and-drop editor, a model-provider
integration, compiler self-hosting, full formal verification, or PostgreSQL
runtime parity. Keep those as separate roadmap goals; do not blur planned
capabilities into the 0.2.0 release claim.

## Release milestone 0.3.0 (published)

Zelyra [`v0.3.0`](https://github.com/sf1976/zelyra/releases/tag/v0.3.0) was
published as an experimental release on 2026-10-03. The completed [release
plan](release-plans/0.3.0.en.md) records scope, verification evidence, and known
limits. The release workflow built and verified Linux and Windows x86_64
artifacts; the published Linux installer, including checksum, repeat install,
and update check, subsequently passed its smoke test.

The independent human onboarding study was not conducted for 0.3.0. The project
owner explicitly deferred it to 0.4.0. This is risk acceptance, not user-test
evidence; human acceptance is mandatory before the final 0.4.0 release. See the
[decision record](release-readiness/0.3.0-human-gate-decision.en.md).

The current database acceptance evidence is green in [CI run
35571692858](https://github.com/sf1976/zelyra/actions/runs/35571692858): the
four-version MariaDB matrix, schema-safety checks, SQLite regression path,
generated applications, and isolated customer/order workflow all passed.
The clean installed-CLI onboarding path is also green in [CI run
35573616419](https://github.com/sf1976/zelyra/actions/runs/35573616419).

## Release milestone 0.4.0 (published, not fully closed)

Stable `v0.4.0` is published and its artifacts passed technical verification.
The [0.4.0 release plan](release-plans/0.4.0.en.md) remains open: independent
human onboarding acceptance was deferred first to 0.5.0 and, by the newer
project-owner decision, is now deferred to 1.0.0. It has not been conducted.
Manual feedback using the published 0.4.0 CLI also exposed an incorrect
absolute-path invocation for `setup` and unclear expectations for `setup` in a
minimal `init` project. The diagnosis and documentation were corrected for
0.5.0 preparation and regression-checked; published 0.4.0 artifacts do not
contain these corrections. See the bilingual 0.5.0
[study protocol](release-readiness/0.5.0-onboarding-study.en.md) for the record
and its limits. It is not evidence of human acceptance.

## Release milestone 0.5.0 (published)

Stable `v0.5.0` is published from commit `6c0c0e6`. It delivers experimental
Docker bundles for a statically recognized subset, separate schemas and
least-privilege database accounts, and a modular customer/order workflow.
Complete dependency/effect closure, shared schema ownership, and a reusable
multi-connection database module are not included. Technical release gates,
artifact verification, and published install/update/rollback smoke passed.
Human acceptance was not conducted and remains deferred to 1.0.0. See the
[release notes](release-notes/0.5.0.en.md) and
[verification record](release-readiness/0.5.0-candidate-verification.md).

- [🧪] The unreleased 0.4 branch can now generate a commit-pinned Dockerfile,
  a Compose app, and a secret-free `.env.example` from `zelyra module bundle`;
  `--dry-run` also returns a deterministic JSON inventory of intended output
  paths without creating the requested destination; the temporary candidate
  package is checked and removed. Locale JSON files are sorted in the manifest,
  and a repeated-export integration test verifies byte-identical bundle files.
  A separate-container smoke test for an
  imported route passes. Each exported
  app receives its own `DATABASE_URL`, while MariaDB remains external. The
  compiler also rejects recognized cross-module table references without an
  import dependency path (`E-MOD-019`), including tables declared in the entry
  module; shared tables must live in an importable schema module. Tables also
  accept explicit `access` lists for `read`, `write`, and `read_write`;
  recognized cross-module accesses without a matching grant fail with
  `E-MOD-021`. CRUD/forms/auth and unknown SQL accesses require `read_write`.
  These are compiler contracts, not MariaDB grants; schema-change rights and
  unknown SQL forms remain unchecked. Database-consuming modules must also
  directly or transitively import the provider (`E-MOD-022`); an entry-only
  import is not inherited. An entry-local declaration must move to an
  importable module. `context --format=json` now lists the provider, each
  database-consuming module, its direct/transitive binding, and the runtime
  variable and actual `DATABASE_URL` fallback without exposing credentials.
  Runtime remains one connection per process; this is introspection, not
  routing. The MariaDB project generator now places
  the database declaration in
  `src/database.zyl` and imports it from `main.zyl`; this is source separation,
  not multiple independently configurable connections. The
  bundle rejects multiple database definitions with `E-DB-001` before writing
  files; only one project-wide connection is supported. The manifest still
  records `source_closure_complete: false` and
  `complete_deployment: false`. Complete dependency analysis, a modular
  database interface, and technical candidate verification remain open. Human
  acceptance is deferred to 1.0.0. The same
  dependency preview now follows named function calls in form and CRUD action
  bodies to their declarations and source modules; a multi-module integration
  test covers both cases. This expands known edges but does not establish
  complete dependency analysis. Table columns now also link to named type
  aliases, so selecting a schema module includes the alias declaration in its
  known declaration closure; a multi-module regression test verifies the
  resource edge and closure. Typed fields on forms, form actions, and CRUD
  actions now link to their named type aliases as well; an integration test
  checks both form and CRUD resource plans. This remains a statically recognized
  subset. Function-body annotations, record constructors, and SQL result types
  also contribute type edges. MariaDB SQL result mapping now accepts a
  module-qualified record name by removing the module qualifier before
  matching the backing table; focused tests cover the resolver and composed
  project path.
  staged-directory publication now uses atomic no-replace operations on Linux,
  macOS, and Windows, so a destination created during export cannot be
  overwritten; a regression test covers both the collision and successful
  publication cases. Separately, the unreleased branch now supports `pub view`
  and `pub component`; imported
  page/CRUD layout references and recognized component tags in page, view,
  component, and CRUD slot HTML require a public declaration plus an explicit
  import path (`E-MOD-007` / `E-MOD-020`). This is a tested visibility
  increment, not a complete HTML namespace or module contract system.
- [🧪] The generated customer/order Docker rehearsal now backs up and restores
  its business rows with scoped MariaDB accounts, applies an additive schema
  migration, then kills MariaDB during an uncommitted customer update. After
  restart it verifies the committed customer/order rows and migrated table
  remain while the interrupted update is rolled back. This is bounded to a
  disposable MariaDB 11 fixture; complete module closure and schema ownership
  enforcement remain open.

## Release 0.6.0

The scoped [0.6.0 release](release-plans/0.6.0.en.md) shipped a native, pooled
PostgreSQL 16 direct-SQL path for a bounded set of parameter types and
transactions. It does not promise complete PostgreSQL parity or generated
web/CRUD support. The release also added a deterministic route inventory with
collision errors and `zelyra db map`, a read-only mapping of tables and
relationships to source modules that shows differences from the live database.
Module ownership remains advisory and changes neither schema nor permissions.
Independent human acceptance remains deferred to 1.0.0.

After 0.6, 0.7 plans a real MySQL server path, bounded tenant context, local
developer studio, and the invoice tutorial. Version 0.8 adds durable tasks,
outbox, controlled studio editing, and text changes with named regex patterns,
preview, and explicit application. Version 0.9 verifies core workflows for
MariaDB, MySQL, and PostgreSQL and may add optional MFA/WebAuthn and OIDC.
Version 0.99 freezes features; only independent human acceptance remains before
1.0.0, the first productive release. Complete SQL parity, broad visual editing, arbitrary client hydration, compiler
self-hosting, native backend, mandatory cloud/AI services, telemetry, and
unsupported performance claims remain excluded.
## First productive release 1.0.0

`v1.0.0` is the first productive publication and the mandatory target for all
independent human onboarding and acceptance studies deferred from 0.3.0,
0.4.0, and 0.5.0. No earlier release may imply those studies were completed.
Before stable `v1.0.0`, publish an immutable release candidate after the
technical gates pass, have independent non-developer participants use the
candidate and shipped documentation, fix and re-verify any blockers, and record
redacted observations and the owner decision against the exact tested commit.
The study scope must match the final 1.0.0 product; automated CI, internal
rehearsals, and owner feedback do not substitute for participant testing.

## Real-world acceptance applications

- [~] The machine-management application is available as a MariaDB template
  with localized machine/department views, search and category/status/area
  filters, plus an optional 30-record fictional SQL fixture. Generated-project
  MariaDB integration tests import the fixture twice and verify the records
  through HTTP; the real application is also tested in all German/English and
  Learn/Work combinations. Novice-tested clean-machine onboarding and
  production hardening remain open. Native seed/fixture commands remain
  optional roadmap work.
- [~] Customer/order acceptance application with joins, aggregates, CRUD
  forms, and a custom project shell is included and passes the reusable
  MariaDB tableview E2E path; permissions and clean CI evidence for the
  primary application remain open.
- [ ] Multi-user inventory application with transactions and concurrent edits.
- [~] A generated Docker Compose deployment with an independently configurable
  web port is tested; production hardening remains open.
- [~] Rust-free self-hosted installation is available for published Linux and
  Windows x86_64 assets; more platforms remain open.

This file must be updated whenever a milestone changes status or a design
decision creates a new required or optional work item.
