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
  commands and complete secret-redaction coverage remain.
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
  host-port readiness; TLS, permissions, and broader external-tool guidance
  remain open.
- [~] Project templates: minimal, MariaDB CRUD, MariaDB authentication, and
  MariaDB business starters are available; API and production-deployment
  templates remain.
- [ ] Offline installation bundle and reproducible toolchain metadata.
- [?] Package-manager distribution where practical (Homebrew, winget,
  Debian packages, and container images).

## 2. Language and compiler

- [ ] Stable grammar specification and versioned compatibility rules.
- [ ] Modules, imports, visibility, namespaces, and multi-file projects.
- [ ] Generics, interfaces/traits, enums, tagged unions, and pattern matching
  across all domain types.
- [ ] Better type inference with precise source spans and fix suggestions.
- [ ] Typed literals and conversions for Decimal, Money, Date, Time, UUID,
  URL, Email, Bytes, and Duration.
- [~] An initial capability/effect model for Database, Network, FileSystem,
  Environment, Process, Clock, Random, and Console exists; interactive
  terminal input through `read_console(prompt) -> String?` is implemented for
  `zelyra run` and is gated by `uses Console` plus the project grant. Granular
  effects such as `Database(read)` and `Database(write)` remain planned.
- [ ] Structured error propagation and user-defined error types.
- [ ] Deterministic build graph, incremental compilation, caching, and parallel
  compilation.
- [~] The deterministic formatter is implemented; language server, editor
  extensions, linter, and debugger remain planned.
- [ ] Stable intermediate representation and backend-independent runtime ABI.
- [ ] Long-term self-hosting path: progressively move compiler tooling from the
  Rust bootstrap implementation into Zelyra while retaining a small trusted
  bootstrap compiler.
- [?] Native code generation beyond the bootstrap backend (LLVM, Cranelift,
  or another maintained backend).
- [?] A package registry and dependency lockfile design.

## 3. Database platform

- [x] MariaDB as the primary tested backend.
- [x] SQLite support for local and embedded applications.
- [x] PostgreSQL schema and planning support.
- [x] Database CLI flow: `create`, `setup`, `bootstrap`, `inspect`, `plan`,
  and guarded `apply` are implemented with explicit backend behavior and
  destructive-change protection. MariaDB `create` and `setup` DDL explicitly
  use InnoDB with utf8mb4/utf8mb4_unicode_ci; SQLite and PostgreSQL output keep
  their backend-specific behavior.
- [ ] Complete PostgreSQL runtime parity.
- [~] MariaDB compatibility is explicitly tested against official image tags
  `10.11.19`, `11.4.13`, `11.8.9`, and `12.3.3` for schema/CRUD HTTP and
  destructive-change approval. This is not a MySQL compatibility claim;
  version-specific diagnostics remain planned. See the [English
  compatibility matrix](database-compatibility.en.md) and [German
  compatibility matrix](database-compatibility.de.md).
- [ ] SQL Server backend evaluation and implementation if demand justifies it.
- [ ] Reversible migration plans, rollback guidance, backups, and drift reports.
- [~] Live schema inspection detects MariaDB default, primary-key, and
  auto-increment drift; SQLite default, primary-key, and explicit
  `AUTOINCREMENT` drift; and PostgreSQL default, primary-key, and
  serial/identity drift. MariaDB and PostgreSQL default additions, changes,
  and removals now generate `REVIEW` plans and require `--allow-risky`; E2E
  tests verify retained rows and idempotent replanning. SQLite default changes
  and key/auto-increment metadata changes remain `UNSUPPORTED`. MariaDB and
  PostgreSQL key/auto-increment changes also remain `UNSUPPORTED`. PostgreSQL
  schema safety is exercised against PostgreSQL 16 in CI, not as a
  runtime-parity claim.
- [~] The schema planner classifies required columns without defaults, unique
  constraints, and generated-name-managed index/FK additions and removals;
  unknown external indexes are preserved and untracked FK removals fail closed.
  Adding a required no-default column to an existing table now runs a read-only
  empty-table preflight; a populated table blocks the entire plan before SQL.
  MariaDB and PostgreSQL nullability changes require `REVIEW`; tightening to
  `NOT NULL` performs a read-only NULL-row preflight before any plan SQL and
  fails closed if rows need repair. SQLite nullability and type/FK/unique-
  constraint alterations remain unsupported. General row estimates, lock
  warnings, data backfill plans, and maintenance-window planning remain planned.
- [ ] Connection pooling, retry policies, timeouts, cancellation, and health
  checks.
- [ ] Streaming large results and bounded memory behavior.
- [ ] N+1 query detection, query-plan hints, slow-query diagnostics, and
  application-owner-controlled, locally inspectable query observability.
- [ ] Typed relations, joins, aggregates, subqueries, CTEs, unions, and
  database-specific extensions.
- [ ] Read replicas, read/write routing, tenant isolation, and migration
  environments.
- [?] Database seed, fixture, snapshot, and anonymized test-data commands.

## 4. Views and web presentation

- [~] The minimal and machine-management starters include a responsive branded
  shell and catalog-backed German/English copy; the machine starter uses the
  learn-mode guide. The `mariadb-crud` starter now defines richer machine and
  department records, localized CRUD list/detail/form/delete states, card views,
  and a repeatable optional fixture containing six fictional areas and 30
  machines. Generated CRUD, standalone form, tableview, login, and
  authentication-admin pages now also receive that responsive default shell;
  explicit CRUD layouts win and authored pages are not rewritten. CRUD, form,
  authentication, validation, learning-guide, and standard HTTP-error copy use
  the same catalogs. Project locale overlays can add or override every
  catalog-backed generated label, including parameterized field labels and
  generated identifier labels, in addition to explicitly marked view/text
  entries. Business records and unmarked user-authored content remain
  unchanged. Project-local `zelyra.theme.css` token overrides are available;
  broader template coverage and full theme replacement remain open.

- [~] Named views/layouts with a page-level `view: Name` assignment, a
  validated default `<slot />` content insertion point, and validated named
  slots with fallback content.
- [~] Typed view expressions check identifier and record-field interpolations,
  page route bindings, component properties, and dynamic component-property
  types. Optional field-aware expressions and richer view data remain open.
- [~] Named components with typed properties are available; typed events remain
  planned.
- [~] Declarative MariaDB-backed `tableview` routes with checked SQL sources,
  declared columns, typed filters, search, sorting, pagination, URL state, and
  escaping are available for table- and struct-backed result types.
- [~] Components and named views support default and named slots, safe fallback
  content, and nested composition. View layouts validate declared slot names
  and replace them deterministically without global state; view inheritance and
  richer nested scenarios remain open.
- [~] CRUD resources can reuse a validated named view with `layout: ViewName`.
  The default slot receives generated lists, details, and generated CRUD forms
  without bypassing SQL, validation, CSRF, authorization, or escaping; named
  slots may be filled per resource with compile-checked static markup and
  components. Generated CRUD content remains confined to the layout's default
  slot; record-bound custom slot content remains open.
- [~] View-local data loading supports explicit, schema-checked record and
  record-collection queries using `load name = sql<Type> { ... }`. Array
  results can be rendered with typed `for item in collection { ... }` blocks.
  Route authorization, the `Database` capability, parameter binding, generic
  error boundaries, and HTML escaping are enforced; optional field handling
  and richer view composition remain planned.
- [~] Typed CRUD filter operators (`eq`, text matching, numeric comparisons,
  and null checks) are compiled to safe server-side SQL.
- [~] Generated CRUD filter controls preserve operator and value state in URLs;
  deterministic filter ordering, semantic fieldsets, and separate operator/value
  labels are available; broader accessibility improvements remain open.
- [~] The unified typed view pipeline covers declarative `tableview` controls,
  explicit page-local record loading, and typed collection loops; filters,
  sorting, search, and pagination are available for declared page collections;
  richer arbitrary-view data remains planned.
- [~] Page-local typed query inputs (`input { search: String? }`) are checked,
  safely bound to native SQL, exposed to HTML interpolation, and rejected with
  controlled HTTP 400 responses when required values are missing or scalar
  values are invalid. Automatically generated controls now cover declared page
  collections; arbitrary input-only pages remain manual by design.
- [~] Page-local collection pagination via `paginated <size>` is available.
  It validates a positive `page` URL value, exposes it as `UInt`, and applies a
  parameterized `LIMIT`/`OFFSET` wrapper; generated controls and safe total/page
  counts are available for declared page collections.
- [~] Page-local collection sorting via `sort { field ... }` is available.
  Only compiler-validated result fields and `asc`/`desc` order values are
  accepted; generated sort controls preserve URL state.
- [~] Page-local collection search via `search { field ... }` is available.
  Search terms are parameterized and applied to compiler-validated fields with
  server-side `LIKE` conditions; generated search controls preserve URL state.
- [~] Page-local typed filters via `filter { field ... }` are available.
  Operators are derived from the declared result types, values are bound as
  parameters, and undeclared fields or unsupported operators are rejected;
  generated filter controls preserve URL state.
- [ ] Composable filter expressions with typed operators for dates, booleans,
  relations, and full-text search.
- [ ] Reusable navigation, tables, forms, dialogs, alerts, pagination, and
  validation-error components.
- [~] CRUD list view overrides support a safe `table`/`cards` mode and a custom
  empty-state message while preserving generated query, auth, and action guards.
- [~] Shared schema-based CRUD view fields can drive generated list, detail, and
  create/edit forms; explicit list selections remain local overrides.
- [~] CRUD detail view overrides support a safe `standard`/`cards` mode and a
  custom heading while preserving generated action, CSRF, auth, and escaping
  guards.
- [~] CRUD form view overrides support a safe `standard`/`cards` mode and
  custom heading/submit labels while preserving validation, CSRF, parameter,
  and permission guards.
- [~] CRUD delete confirmation overrides support custom headings, warning
  messages, and submit labels while preserving POST-only, CSRF, and auth guards.
- [~] CRUD loading metadata and configurable error views preserve escaping and
  generic database-error boundaries; client-side loading UI remains open.
- [~] Custom CRUD actions can execute parameterized, POST-only business SQL
  with CSRF, database-capability, authentication, and permission checks;
  custom labels and browser confirmations are available; action-specific view
  configuration remains open.
- [~] A first design-token system exposes colors, typeface, card/control
  radii, and content width; spacing, breakpoints, density, and coverage across
  all components remain open.
- [~] Optional project-local `zelyra.theme.css` is loaded after the built-in
  design, bounded to 128 KiB, and copied by generated Dockerfiles. A full theme
  engine/editor, dark mode, built-in theme choices, and user-selectable
  appearance remain open.
- [ ] Scoped CSS, asset pipelines, cache-busting, static files, and CSP-aware
  inline assets.
- [~] Responsive generated shells, labeled navigation, keyboard focus styles,
  and a localized skip link are implemented; broader semantic/ARIA review and
  automated accessibility checks remain open.
- [ ] Localization, pluralization, timezone/locale formatting, and RTL support.
- [ ] Secure raw HTML escape hatch with diagnostics and review markers.
- [ ] Progressive enhancement: server-rendered HTML first, optional client
  state and hydration second.
- [ ] WebSocket/SSE support and typed client-server events where needed.
- [~] Browser integration coverage now includes MariaDB-backed struct tableviews;
  view snapshots and deterministic rendering tests remain planned.
- [?] Optional alternate renderers (email, PDF, text, native desktop).

## 5. Forms, CRUD, and business applications

- [ ] Nested forms, repeatable fields, file uploads, multi-step workflows, and
  conditional fields.
- [ ] Cross-field and database-backed validation with explicit transaction
  boundaries.
- [ ] Optimistic locking and conflict-aware editing.
- [ ] Bulk actions, import/export, saved searches, column preferences, and
  server-side reporting.
- [~] Custom action labels and browser confirmations are available.
- [x] Typed custom action inputs use normal form validation and parameter
  binding; relationship fields render checked MariaDB-backed select widgets.
- [x] Custom action icons and escaped success notices are available.
- [x] Server-rendered `confirm_page` views with fresh CSRF-protected POST
  confirmation are available.
- [x] Structured `success_page` notices and safe action-specific `error_page`
  responses are available.
- [x] Reversible CRUD soft delete with archived lists and CSRF-protected
  restore actions.
- [x] Audit-aware CRUD create/update/delete/archive/restore and custom-action
  events with field-level change details and sensitive-value redaction.
- [ ] Permanent purge, retention policies, archive export, and bulk archive
  workflows.
- [ ] Audit-aware CRUD history and field-level change diffs.
- [ ] Background jobs, scheduled tasks, retries, and transactional outbox.
- [ ] Notifications, email templates, SMTP, and provider abstraction.
- [ ] Multi-tenant application primitives and tenant-aware authorization.

## 6. Authentication, authorization, and audit

- [x] Password login, persistent sessions, CSRF, account activation, and
  last-administrator protection.
- [x] Direct permissions and role-derived permissions.
- [x] Browser administration and CLI role management.
- [x] Audit inspection, bounded export, structural verification, and safe prune.
- [x] Cryptographically chained audit entries using a documented SHA-256 hash
  format, explicit canonical serialization, and a transaction-locked append
  strategy.
- [~] `audit verify` detects broken hash links and invalid entry hashes; precise
  first-invalid-entry locations and source-independent diagnostics remain open.
- [ ] Immutable or append-only database privileges for audit tables.
- [ ] Configurable retention policies, scheduled pruning, and archive export.
- [ ] Encrypted archives, key rotation, restore verification, and offline
  integrity checks.
- [ ] Explicit, user-controlled audit exports to destinations such as syslog,
  object storage, or SIEM, with delivery status and retry behavior; usage
  telemetry and hidden remote collection are out of scope.
- [ ] MFA/WebAuthn, password reset flows, session/device management, and login
  notifications.
- [ ] Fine-grained policy expressions, policy testing, and permission explain
  output.
- [ ] Security review, threat model, dependency audit, and penetration testing.

## 7. APIs and integration

- [x] Typed API routes, request validation, JSON serialization, OpenAPI, and
  TypeScript client generation.
- [x] String-keyed typed maps are checked at the API boundary and represented
  consistently in JSON, OpenAPI, and generated TypeScript clients.
- [ ] API versioning, deprecation metadata, rate limits, quotas, and request
  correlation IDs.
- [ ] Authentication schemes for API keys, OAuth2/OIDC, and service accounts.
- [ ] Webhooks, signed callbacks, idempotency keys, and retry-safe handlers.
- [ ] GraphQL or another query API only if it can preserve Zelyra's type and
  capability guarantees.
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
- [ ] Fuzzing for lexer, parser, SQL binder, template renderer, and HTTP parser.
- [ ] Security regression suite and dependency/license scanning in CI.
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
  ports, and cleanup. Clean-host installation and recovery coverage across
  supported platforms remain open.
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

## Proposed release milestone 0.3.0

The separate [0.3.0 release plan](release-plans/0.3.0.en.md) is a working
proposal, not a release promise. It prioritizes beginner onboarding, an
extensible distinctive Views/template system, MariaDB/schema safety, a
reproducible real-application acceptance path, dependable compiler interfaces,
and honest release evidence. Scope and status must be reviewed as work proceeds;
the plan does not make all long-term roadmap items 0.3.0 requirements.

The current database acceptance evidence is green in [CI run
35571692858](https://github.com/sf1976/zelyra/actions/runs/35571692858): the
four-version MariaDB matrix, schema-safety checks, SQLite regression path,
generated applications, and isolated customer/order workflow all passed.
The clean installed-CLI onboarding path is also green in [CI run
35573616419](https://github.com/sf1976/zelyra/actions/runs/35573616419).

## Proposed release milestone 0.4.0

The separate [0.4.0 release plan](release-plans/0.4.0.en.md) is a working
proposal for the next milestone after the final 0.3.0 release. It prioritizes
modules and deterministic multi-file projects, reliable database/runtime
lifecycle management, safer account and API lifecycle controls, granular
AI-native effects, and supply-chain/accessibility evidence. It deliberately
does not turn PostgreSQL runtime parity, multi-tenancy, background jobs, MFA,
OIDC, or a visual editor into automatic 0.4.0 promises.

The 0.4.0 plan must not be treated as started until 0.3.0 has a final release
and its human onboarding evidence is recorded. Scope and status remain subject
to the plan's risk register and acceptance gates.

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
