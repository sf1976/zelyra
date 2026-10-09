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
  sessions expire after 24 hours. Device metadata remain open, and password
  recovery is still experimental; this does not complete the account lifecycle
  milestone.
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
- [🧪] `zelyra db plan --format=json` emits a versioned
  `zelyra.schema-plan/v1` plan with a stable forward ID, schema fingerprints,
  drift, ordered SQL steps, data preflights, and an explicit approval flag.
  When every reverse operation is supported, it also emits a reverse schema
  diff with its own ID, expected fingerprints, preflights, and approval needs.
  Unsupported reversals fail closed. Reverse DDL is never automatic and may
  discard later data; every schema change still requires an independently
  verified backup. `db apply` accepts the reviewed plan ID and rejects stale
  plans before preflights or SQL; integration tests cover stale rejection,
  forward application, and explicit reverse application after restoring the
  prior source. All three backends journal schema fingerprints and outcomes in
  `_zelyra_schema_history`; `db history` reports applied, failed, and
  interrupted attempts. MariaDB records per-step checkpoints and uses a
  database-scoped advisory lock; E2E tests recover after interruption between
  steps and while DDL waits on a metadata lock. The MariaDB E2E also applies a
  reviewed reverse plan only after explicit approval and confirms that the
  added columns are removed while rows and IDs remain. A crash during actual
  MariaDB DDL execution can still leave that statement's result ambiguous. PostgreSQL
  and SQLite apply DDL transactionally. Read-only preflights check NULLs,
  required columns, duplicate unique-index values, and foreign-key orphans,
  but do not prevent concurrent-write races. Backups remain the operator's
  responsibility.
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
  New unique indexes and foreign keys also run read-only duplicate and orphan
  checks before any SQL; concurrent writes can still race these checks.
  MariaDB and PostgreSQL nullability changes require `REVIEW`; tightening to
  `NOT NULL` performs a read-only NULL-row preflight before any plan SQL and
  fails closed if rows need repair. SQLite nullability and type/FK/unique-
  constraint alterations remain unsupported. General row estimates, lock
  warnings, data backfill plans, and maintenance-window planning remain planned.
- [~] The development branch bounds MariaDB connection establishment (default
  10 s; allowed 1–300) and server-side runtime/read-query statements (default
  30 s; allowed 1–3600), rejects invalid values without echoing them, and
  disables transparent client reconnect. The runtime SQL path now has a hard-
  bounded process-wide pool (default 8; allowed 1–64), checkout health checks,
  bounded pool waits (default 10 s; allowed 1–300), and discards connections
  after statement failures. Timeout and pool integration tests pass locally
  against isolated MariaDB 11.4 and in [PR CI run
  37156746403](https://github.com/sf1976/zelyra/actions/runs/37156746403)
  across MariaDB 10.11.19, 11.4.13, 11.8.9, and 12.3.3. Runtime and CLI
  connections now support verified TLS: `auto` requires certificate- and
  hostname-verified TLS for non-local hosts, `required` forces it, and
  `disabled` is explicit. A custom CA path is supported. Positive handshake,
  CLI inspection, and untrusted-CA rejection passed locally on MariaDB 11.4
  and in PR CI run 37161345832 across all four MariaDB matrix versions.
  Standalone Docker module exports default to `auto`; only the full local
  Compose template
  opts out for its isolated internal database network.
  Schema inspection/DDL still uses the CLI; response transfer is not globally
  bounded, and there are no automatic retries. Windows TLS has not been tested
  separately. This is not in 0.3.0. Response deadlines and health diagnostics
  remain open.
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
- [🧪] Auth definitions can tune process-local login and reset failure windows
  and lockout duration. Password recovery is partially implemented in unreleased 0.4
  development branch: MariaDB stores single-use token hashes, generic
  responses, audit, loopback/HTTPS SMTP configuration, bounded asynchronous
  delivery, and session revocation are covered by a MariaDB/SMTP-sink E2E with
  deliberately delayed delivery. The E2E also races two same-token submissions
  and verifies one success, one rejection, and authentication with only the
  winning password. Token replacement and FIFO mail enqueue are serialized
  within one process. MariaDB advisory locks also coordinate issuance and
  delivery across instances; a two-instance E2E delays the first SMTP relay
  while the second instance replaces the token and verifies that the later
  email matches the stored token. Stale queued tokens are skipped. The MariaDB
  E2E rejects a token with `expires_at = NOW()` and freezes the database clock
  to verify the production lookup predicate before, at, and after expiry.
  Equality is rejected. Process-local reset throttling, durable SMTP delivery
  recovery, and independent security review remain open; the account lifecycle
  is not complete.
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
- [🧪] Password reset and session controls are partial in unreleased 0.4;
  concurrency, persistent reset throttling, device
  metadata, MFA/WebAuthn, and login notifications remain open.
- [ ] Fine-grained policy expressions, policy testing, and permission explain
  output.
- [ ] Security review, threat model, dependency audit, and penetration testing.

## 7. APIs and integration

- [x] Typed API routes, request validation, JSON serialization, OpenAPI, and
  TypeScript client generation.
- [x] String-keyed typed maps are checked at the API boundary and represented
  consistently in JSON, OpenAPI, and generated TypeScript clients.
- [🧪] API request IDs and per-route version, deprecation, and process-local
  quotas are implemented in unreleased 0.4 development. Quotas use TCP peer IP,
  a bounded in-memory client table, and no trusted forwarded-IP header; they
  reset on restart. Persistent/distributed quotas, reset limits, and
  compatibility evidence remain open. Login limits are configurable per auth
  definition; their in-memory table is capped at 4,096 keys and rejects new
  keys when full.
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
- [~] Deterministic bounded mutation regressions exercise the lexer/parser, SQL
  binder, template renderer, and HTTP parser in normal CI (2,048 inputs per
  path). Coverage-guided runs found a parser stack overflow; recursive syntax
  now has a 32-frame recursion limit, focused regressions, and a retained crash
  seed. Four post-fix 60-second local runs passed; the 15-second CI rerun for
  the fixed revision remains open.
- [~] Security regression coverage and dependency/license scanning run in CI;
  a complete human audit and the remaining security regressions are still open.
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

## Proposed release milestone 0.4.0

The separate [0.4.0 release plan](release-plans/0.4.0.en.md) is a working
proposal for the next milestone after the final 0.3.0 release. It prioritizes
modules and deterministic multi-file projects, reliable database/runtime
lifecycle management, safer account and API lifecycle controls, granular
AI-native effects, and supply-chain/accessibility evidence. It deliberately
does not turn PostgreSQL runtime parity, multi-tenancy, background jobs, MFA,
OIDC, or a visual editor into automatic 0.4.0 promises.

The stable `v0.3.0` release was published on 2026-10-03. The project owner
explicitly deferred the 0.3.0 human onboarding test to 0.4.0; this is risk
acceptance, not test evidence. The 0.4.0 plan requires that independent test
before its final release. Scope and status remain subject to the plan's risk
register and acceptance gates.

## Proposed release milestone 0.5.0

The bilingual [0.5.0 roadmap proposal](release-plans/0.5.0.en.md) is a
forward-looking plan, not an implementation or release claim. It is gated on
the final 0.3.0 release and an accepted 0.4.0 baseline. Its primary product
goal is automatic module wiring and verified export of complete application
slices as independent Docker deployments. A reusable, separately configurable
database module, explicit schema ownership, reproducible extraction, and
end-to-end independent startup are P0 acceptance requirements. The plan keeps
unimplemented features clearly marked and excludes any presumption of
PostgreSQL runtime parity or production readiness.

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
  database interface, and full 0.5.0 acceptance remain open. The same
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
