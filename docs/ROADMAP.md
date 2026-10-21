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

The current stable release is 1.2.35, published at https://github.com/sf1976/zelyra/releases/tag/v1.2.35. **Current milestones** is the source of
near-term commitments. Older numbered release sections below are retained as
historical planning and evidence; they do not by themselves define current
release targets. Longer-term ideas remain in the post-1.0 evolution section.

## Release sequence through 1.3.0

This sequence is the authoritative schedule for active work. Every active open
item below is assigned to a release; historical backlog entries are retained
for context and are not additional unscheduled commitments. Release completion
requires implementation, documentation, positive and negative tests, and the
security and compatibility gates stated for that milestone.

- [ ] **1.1.5 — reusable Zelyra modules and packages:** Complete the local,
  offline package workflow, typed public APIs and effects, deterministic locks,
  reusable business modules, safe settings management, and documented
  deployment evidence. Close remaining package trust, extension-boundary,
  module configuration, and integration gaps. CSV is the first exchange
  module; ODBC, Excel-compatible export, PDF workflows, and additional
  backends are only included when their own bounded, tested integrations are
  ready. No Laravel-specific implementation is part of this release.
- [~] **1.2.0 — application operations and deployment foundation:** Add the
  declarative command/scheduler/job lifecycle and management views in bounded
  stages; typed request/forms and scoped query foundations; and the static
  `public/` directory. Provide production `serve` configuration and reviewed
  Apache/Nginx reverse-proxy examples. Apache/Nginx proxy HTTP traffic to the
  Zelyra process; they do not execute `.zyl` files. `main.zyl` remains the
  application entry point and its routes handle `/`. Add
  `zelyra service install --port <port>` with a fixed port and an OS service
  definition (Linux systemd first). A busy port is an error; installation must
  never silently select another port. Persist project, executable, working
  directory, and safe environment references without copying secret values.
- [x] **1.2.5 — operations and package foundations:** Deliver signed source
  packages with publisher trust and exact registry resolution, a Linux
  systemd service with a fixed port, its resident scheduler, static `public/`
  serving, and verified deployment bundles/runtime improvements. Package
  recovery is delivered in 1.2.6; transitive resolution, typed query/forms,
  durable jobs, and remaining ecosystem work are staged through 1.2.15. See the
  [package ecosystem plan](release-plans/1.2.5-module-ecosystem.en.md).
  `zelyra service change --port <port>`
  validates the new port, stops the old service, updates its configuration,
  and starts the new one; on failure it restores the prior configuration and
  service. Include explicit `status`, `restart`, and `remove` operations where
  the platform service manager supports them. Extend service support to other
  operating systems only with lifecycle and upgrade tests.
- [x] **1.2.6 — recoverable package mutations:** Serialize package mutations
  per project and recover interrupted add/update/remove/cache operations from a
  versioned journal. `zelyra package recover` previews by default and changes
  files only with `--apply`; cache cleanup uses validated quarantine paths.
  Full PR CI passed on `abd2dee` and the change is integrated in `main` as
  `fb330e0`. Recovery is not a filesystem-wide atomic transaction. See the
  [recovery work log](release-plans/1.2.6-package-recovery.en.md).
- [x] **1.2.7 — trusted transitive registry packages:** Published at
  https://github.com/sf1976/zelyra/releases/tag/v1.2.7. The full PR CI passed
  before merge, and the release workflow built and attested reproducible Linux
  and Windows artifacts with verified SHA-256 sidecars and SPDX SBOMs. A
  three-level signed package closure is installed, compiler-checked, run and
  bundled offline; update plans resolve the closure, and cycle rejection
  preserves project state. The signing command remains offline, so full
  compiler validation for registry-linked packages happens after trusted
  closure installation. The 1.2.6 journal records each new staging and cache
  path so failed updates can roll back the whole closure.
- [~] **1.2.8 — typed query, tenant, request and form pipeline:** The first
  query-HIR slice now carries the checked SQL operation, deterministically
  ordered table accesses, parameter locals, and resolved parameter types from
  HIR into MIR. The MIR verifier recomputes operation/table/parameter metadata
  from the SQL source and rejects stale plans with `E-MIR-VERIFY-004`. This
  adds no parallel query syntax. Existing authenticated generated CRUD remains
  the supported tenant-scoped read/write path; raw SQL for scoped tables stays
  rejected in AST, MIR, and bytecode. The current API error/output contract,
  typed page/form inputs, server-side form validation, CSRF, and safe-value
  preservation remain the request/form path. Form-action redirects are
  statically constrained to safe same-origin absolute paths (`E-FORM-007`).
  Full web-runtime parity in MIR/bytecode remains a 1.3.0 gate; the HTTP server
  continues to use AST.
- [x] **1.2.9 — durable scheduler jobs:** The installed, active Linux systemd
  project service writes deterministic scheduled occurrences to the MariaDB
  outbox. A serialized worker validates current declarations, renews leases,
  applies bounded retries, and exposes payload-free status through
  `zelyra schedule jobs`. Removed schedules cancel unleased pending jobs during
  setup or service startup. No host cron entries are installed; delivery is
  at-least-once and missed offline minutes are not backfilled.
- [~] **1.2.10 — safe module configuration and reusable packages:** Complete
  the protected Studio settings page for compiler-checked package settings,
  including cross-process concurrent-save serialization, stale-revision
  rejection, secret redaction, and no implicit capability grants. Keep
  HostServices documented as a versioned embedding API, not a stable native
  ABI or package plugin contract. Verify reusable CSV import/export packages
  across supported execution backends. Ordinary application-served settings
  pages and cross-process settings transactions move to 1.2.12.
- [✅] **1.2.11 — tutorial UI and CLI reliability:** Released the consistent
  invoice tutorial design, actionable missing-outbox-table guidance,
  Debian/Ubuntu Compose-v2 version recognition, project `.env` resolution for
  audit/auth-role commands, and normal Unix SIGPIPE behavior. Further invoice
  layout and project-theme editor refinements are tracked under 1.2.12. The
  wider application-settings, ecosystem parity, and deployment-security work
  remains in 1.2.12.
- [✅] **1.2.11 invoice tutorial CSS subtask:** Centralize generated-page,
  CRUD, form, filter, and table styling in Zelyra UI; give the invoice tutorial
  one responsive design and generated forms the same page frame. Keep
  project-specific design in the synchronized theme template.
- [x] **1.2.12 — application settings and tutorial polish:** Released as
  [`v1.2.12`](https://github.com/sf1976/zelyra/releases/tag/v1.2.12). The app
  settings page, dialog defaults, Studio access to `zelyra.theme.css`,
  responsive invoice CRUD, and related security tests passed the full release
  gates. Apache/TLS testing, full package/backend parity, and the remaining
  business workflows stayed outside this scoped release and are assigned to
  1.2.14–1.2.15.
- [x] **1.2.13 — canonical project structure:** Released as [`v1.2.13`](https://github.com/sf1976/zelyra/releases/tag/v1.2.13). `zelyra new --layout standard`
  creates a checkable minimal `routes/`, `domain/`, `schema/`, `public/`, and
  `assets/` tree. `[project].layout = "standard"` opts in; `check`, `run`, and
  `serve` require these roots to be real directories contained in the project.
  The existing `import "domain::feature::actions" as actions` syntax now
  resolves project namespaces under `schema/`, `domain/`, and `routes/` only in
  this profile; legacy imports remain unchanged. Database templates and deeper
  architecture checks remain open; `plugins/` is deferred until it has a
  secured loader.
- [~] **1.2.14 — invoice tutorial and project architecture:** Give the invoice
  tutorial a consistent responsive Zelyra UI theme, a designed sample invoice,
  aligned CRUD cards with clear labels, and a shared source-code dialog.
  `zelyra migrate-layout --check [entry.zyl] [--format human|json]` provides a
  deterministic AST-based read-only preview and flags mixed or ambiguous files
  for manual review. It rejects source symlinks and does not modify files.
  `strict_architecture = true` checks declaration kinds under `schema/`,
  `domain/`, and `routes/` (`E-ARCH-001`); it is not a complete call-graph or
  effect analysis. The tutorial was tested with sample data in Chromium,
  Firefox, and WebKit. Apache/TLS integration and deeper architecture checks
  continue after this release.
- [x] **1.2.15 — business workflows and backend parity:** Prove settings,
  effects, jobs, data access, and HostServices across supported AST/MIR/Bytecode
  paths, or fail closed with stable diagnostics. Complete CSV modules, bounded
  state transitions, audit/history, balanced ledger operations, reflection and
  export metadata, health checks, and modular bundles with concrete integration
  tests. Complete Apache/TLS loopback tests and reverse-proxy configurations.
  Add composable, type-checked extension points to generated CRUD and tableview
  surfaces for create/edit pages, field help, page content, and row-level UI
  actions while keeping routing, persistence, and security checks centralized.
  The page-customization subfeature was tested in the merged change and shipped
  in [`v1.2.15`](https://github.com/sf1976/zelyra/releases/tag/v1.2.15). Other
  work originally grouped under 1.2.15 remains open and is reassigned below;
  Wasm/sidecars remain optional and require their own sandbox/ABI gate.
- [~] **1.2.16 — composable TableView cells:** Add typed supporting content to
  displayed columns and explicit non-displayed result fields for row templates.
  The parser, SQL projection checking, web query selection, and safe HTML
  rendering are implemented with positive and negative tests. Workspace tests,
  Clippy, handbook checks, the MariaDB/browser invoice E2E across Chromium,
  Firefox, and WebKit, and Linux artifact smoke passed. The PR's Windows CI and
  release workflow remain pending. This does not add arbitrary per-request
  SQL, a write action, or access to undeclared result fields.
- [x] **1.2.17 — tenant-safe relations:** Optional table relations produce
  nullable foreign-key columns. Relations between tables with the same tenant
  identity use a composite `(tenant_id, relation_id) -> (tenant_id, id)` key;
  incompatible identities and tenant-key `SET NULL` are rejected. The
  MariaDB end-to-end test checks cross-tenant rejection.
- [~] **1.2.18 — backend differential coverage:** Expand the bounded MIR and
  bytecode differential corpus for tuples, conditional expressions, record
  updates, closures and guarded matches. Experimental backends remain opt-in;
  this milestone does not imply full language parity.
- [~] **1.2.19 — tenant-scoped unique values:** Add `unique within tenant` for
  scoped tables. It generates a composite unique index, rejects ambiguous
  declarations, and requires review plus a duplicate-row preflight when
  changing an existing schema. Existing-row repair remains an explicit
  operator task.
- [~] **1.2.20 — table and form invariants (released in 1.2.21):** Add typed
  cross-field validation
  with source-mapped diagnostics and consistent generated CRUD/API checks
  before database writes. A backend that cannot preserve the checked
  validation semantics must reject that path.
  Declarative forms and generated CRUD run rules before write actions; APIs can
  explicitly bind POST/PUT/PATCH input to unscoped tables and return a typed
  HTTP 422 error. Scoped API bindings are rejected until a verified tenant
  context is available, and raw SQL writes are not covered. See
  [`table-invariants.en.md`](table-invariants.en.md).
- [x] **1.2.21 — lifecycle guards and row-conditioned CRUD policies:** The
  parser and type checker now lower row conditions to ordinary typed Boolean
  functions. Edit guards and conditional permissions are evaluated for edit
  forms, including again under the edit transaction's row lock; delete guards
  and permissions are checked inside the delete transaction. Detail-page edit
  links and explicitly scoped TableView edit/delete slots use the same
  evaluator. TableView operation slots must name their CRUD model and select
  the complete typed row; incomplete policy rows are rejected by the compiler.
  MariaDB direct-request and row-lock race coverage runs in the CI workflow;
  local tests and the merged PR's complete external CI passed. Relation values
  are also resolved correctly in table-invariant checks, and tables without
  declared rules skip that evaluator.
  Keep custom actions bound to typed, capability-checked functions. See the
  [1.2.21 work log](release-plans/1.2.21-crud-lifecycle-policies.en.md).
- [x] **1.2.22 — transactional master-detail CRUD (released):** A typed
  `has_many` collection is checked and rendered as bounded server-side rows on
  Create/Edit. Parent/child writes, child invariants, and aggregate
  recomputation share one transaction. MariaDB E2E covers invalid-submit
  rollback, create, edit/recompute, and schema cascade. Multiple child
  collections and more than 50 rows per submit remain unsupported. See the
  [1.2.22 work plan](release-plans/1.2.22-business-invariants-and-master-detail.en.md).
- [x] **1.2.23 — authenticated tenant-scoped API invariants (released):** `validates`
  binds scoped table rules only on authenticated APIs with a configured active
  membership table. Zelyra rejects tenant keys from request input, resolves the
  selected tenant against the session and memberships, evaluates the rule with
  that verified tenant, and passes it as a typed final handler parameter.
  Missing or unauthorized context fails before the handler. This validates
  declared inputs, not arbitrary handler transformations or raw SQL writes.
  Released as [`v1.2.23`](https://github.com/sf1976/zelyra/releases/tag/v1.2.23);
  the full PR and release workflows passed. See the
  [1.2.23 plan](release-plans/1.2.23-scoped-api-validation.en.md).
- [x] **1.2.24 — multiple transactional CRUD child collections (released):** Extend the
  existing typed `has_many` path from one child collection to multiple named
  collections on Create/Edit. Validate all parent and child values before
  writing, then save or replace every declared collection in the same database
  transaction, recompute each collection's aggregates, and roll back all rows
  when any collection fails. Preserve tenant scoping, per-table invariants,
  row limits, and existing single-collection behavior. Do not add arbitrary
  CRUD action code as part of this change. Require a MariaDB HTTP E2E covering
  two collections, an invalid second collection with full rollback, and edit
  replacement without cross-parent access. See the
  [1.2.24 work plan](release-plans/1.2.24-multiple-crud-children.en.md).
  Released as [`v1.2.24`](https://github.com/sf1976/zelyra/releases/tag/v1.2.24);
  Linux and Windows packages, SHA-256 sidecars, and SPDX SBOMs are available.
- [x] **1.2.25 — native database console and query commands:** Add host client
  passthrough for MariaDB/MySQL and PostgreSQL, credential lookup from the
  project's named database URL and `.env`, and direct single-statement queries
  with table/JSON/CSV output. Queries default to database-enforced read-only
  transactions; mutation is a visible opt-in. Tenant ids bind only to explicit
  parameters and never imply automatic row filtering. See the
  [1.2.25 work plan](release-plans/1.2.25-db-cli-console-query.en.md).
  Released as [`v1.2.25`](https://github.com/sf1976/zelyra/releases/tag/v1.2.25).
- [~] **1.2.26 — interactive language console:** Add an incremental AST
  interpreter session with persistent typed bindings, table and field-name
  tab completion, line continuation,
  recoverable two-line source-located diagnostics, safe project context, and explicit backend selection
  only where equivalent semantics are implemented. `--backend bytecode` now
  runs the supported MIR/VM subset through the shared REPL frontend; it remains
  experimental and unsupported operations fail closed. Sandbox writes inside a
  pinned MariaDB/MySQL transaction and roll them back on exit unless `--persist`
  is explicit. `:commit` and `:rollback` rotate the pinned transaction after
  each explicit operation. `--tenant` and `:tenant` expose both the legacy
  `tenant_id` binding and typed `session.tenant_id: Option<Int>` context,
  without adding authorization or row filtering. AST and bytecode SQL share
  the same pinned MariaDB transaction and per-input savepoints. PostgreSQL
  sandbox remains unsupported. Preserve compiler, capability, contract,
  step-budget, and stack
  depth checks. If an input savepoint cannot be safely created, released, or
  rolled back, the session fails closed and refuses later expressions until
  exit triggers the outer rollback.
  `tests/language-console-mariadb-e2e.sh` verifies exit rollback, explicit
  `:rollback`, batch commit, and `--persist` against an isolated disposable
  MariaDB; the same E2E verifies bytecode SQL rollback. Pipe tests cover
  persistent bindings and error recovery on both backends. PostgreSQL sandbox
  transactions remain open. The record pretty-printer marks Decimal values
  with `[Decimal]`; it does not invent scale absent from the runtime value.
- [x] **1.2.27 — Laravel website import foundation:** Reintroduce a new,
  read-only-first importer for migrating selected website behavior into native
  Zelyra modules; this is not Laravel framework compatibility or PHP runtime
  support. Generation requires `--table <name>` or an explicit
  `--all-supported`; it no longer silently selects every recognized subset.
  Analyze routes, controller actions, services, models, migrations,
  Blade layouts/views/assets, and optional live database schema metadata as a
  source-evidenced graph. Read `.env` only for an explicitly requested
  read-only schema connection; never print credentials, read application rows,
  mutate the source, or run imported PHP. Generate only explicitly selected,
  supported modules into a new destination, then run Zelyra compiler checks.
  Current generation emits selected schemas, narrow pure model/service helpers,
  view/layout and asset candidates, plus `ROUTE_CANDIDATES.json` with resolved
  resource routes and model consumers; route handlers and behavior are still
  not generated. Explicit read-only database inspection resolves an unresolved
  MariaDB Compose alias within the source project's Compose working directory,
  requests no container environment values, detects the server vendor for
  bounded metadata queries, and reports 80 tables/1,215 columns on the first
  fixture. This host's local Docker DB required an explicit TLS-disabled
  invocation; TLS is never disabled automatically. A full scaffold compared
  78 migration-table candidates with zero schema differences; two live tables
  were outside the migration scanner. Its `main.zyl` and separate validation
  candidate module both passed compiler checks. This does not prove route or
  workflow migration. A resolved conventional `tenant_user` pivot and
  `tenants.user_id` owner relation are now included with `users` in the reusable
  `imported_identity` package, so the package's auth declaration can validate
  membership and direct ownership without a cross-package table dependency.
  The actual Laravel active/default tenant selection is still not transferred.
  The 1.2.27 fixture generated 71 schema declarations and passed `zelyra check`;
  its auth package owns `users`, `tenant_user`, `tenants`, and Zelyra sessions.
  A disposable MariaDB 11.4 runtime test also passed with a membership pivot
  that has no `active` column: a user with direct `tenants.user_id` ownership
  could create a tenant-scoped invoice without a pivot row, while an account
  with neither membership nor ownership remained denied. Write transactions
  revalidate the same membership/ownership rules before mutation.
  Module interoperability was exercised end to end: the generated package
  resolved through `module plan`, passed source-closure bundling, and was
  copied into a separate consumer project using a relative `pkg:` dependency.
  The consumer lock and compiler check passed and it exposed the five native
  auth/session routes. The bundle still reports incomplete deployment because
  runtime effects, project configuration, and deployment artifacts are not
  fully modeled. A CLI regression test repeats the package-consumer path with a
  minimal Laravel fixture.
  Keep unresolved behavior and exact source locations visible in a versioned
  report. The `/opt/siedelmann-laravel/app` project is the acceptance fixture;
  broader route/workflow coverage continues in 1.2.30. Released as
  [`v1.2.27`](https://github.com/sf1976/zelyra/releases/tag/v1.2.27); its PR
  and release workflows passed all checks. See the
  [1.2.27 importer plan](release-plans/1.2.27-laravel-website-importer.en.md).
- [x] **1.2.28 — typed CRUD domain actions and REPL safety (released):** Add a checked execution context
  for action statements with explicit row, authenticated user, and verified
  tenant values. Reuse the language type/effect checker, require explicit
  permissions, support transaction-safe SQL plus allowlisted typed calls, and
  evaluate an action guard while holding the target row lock. Retain one
  transaction for business writes and audit insertion. Use existing
  `hash_sha256` and `now()` spellings and their capability rules; PDF rendering
  and mail outbox calls must follow their existing transaction and attachment
  restrictions. No implicit `crypto::` namespace or raw SQL interpolation.
  The parser stores explicit execute/guard nodes; CLI and web use common AST
  lowering for typed `row`/`session` scopes. Web execution now loads the target
  under `FOR UPDATE`, revalidates tenant membership/ownership in the same
  transaction, evaluates the pure guard, then executes typed SQL and automatic
  audit insertion through the borrowed transaction bridge. A `transaction`
  block inside an action is a logical batch on the caller-owned transaction;
  it cannot commit or open a second connection. Its checked `mail_queue` path
  supports locally rendered PDF attachments. MariaDB HTTP E2E proves
  state/hash/time update, one audit insert, repeat-guard rejection, rollback
  after a later runtime failure, and an outbox row with a real PDF attachment
  that remains pending for a separately started worker. Granular
  `Database(read/write)` grants also open the web database gate correctly.
  The tenant HTTP E2E also verifies that `session.tenant_id` and
  `session.user_id` come from the authenticated selected tenant, a wrong-tenant
  target fails with 404, and direct POST without the action permission returns
  403; invalid CSRF also fails without a write. The permitted user sees the
  action control, while a CRUD viewer without the action permission does not.
  A separate-database PostgreSQL HTTP E2E now proves successful row/hash/
  time and audit updates plus rollback after a runtime error. Both database
  E2Es also send two simultaneous lock requests and verify exactly one success,
  one guarded conflict, and one audit row. The REPL sandbox now rejects
  MariaDB/MySQL DDL, transaction-control, and session-control statements that
  could bypass rollback; tests cover denied grants, rollback, and explicit
  commit. `:schema` prints invariant messages and source positions. The REPL
  required the aggregate `Database` grant at release time; the bytecode REPL
  was added later as an experimental 1.2.26 follow-up and uses the same grant
  checks. Released as [`v1.2.28`](https://github.com/sf1976/zelyra/releases/tag/v1.2.28); release gates passed. The broader security/permission matrix remains an open follow-up. See the
  [1.2.28 work plan](release-plans/1.2.28-typed-crud-actions.en.md).
- [x] **1.2.29 — bounded secure file upload foundation (released):** Architecture inventory
  is complete. The HTTP server now reads bounded byte requests and sends valid
  multipart bodies through the decoder; the existing public text `Request` API
  and 1 MiB global limit remain unchanged. Multipart receives HTTP 415 on
  routes without declared file fields; malformed multipart
  receives HTTP 400. POST multipart requests matching a declared File form get
  a route-derived body ceiling; all other routes remain at 1 MiB. A global
  256 MiB in-flight body budget bounds aggregate buffering. Parser and live HTTP tests cover binary payloads,
  malformed boundaries, duplicate headers, and limit failures. A
  coverage-guided fuzz target completed a local six-second run with 647,431
  inputs and no crash. Compiler-checked `File` form metadata now accepts bounded
  sizes and the PDF/PNG/JPEG allowlist, rejects table-backed forms and
  storage/action contexts, and renders a multipart file input without echoing
  client filenames. Matching POST form routes now validate and stage uploads
  after authorization and CSRF; successful actions publish before SQL, failed
  actions remove them. A Unix-only private storage primitive now
  provides staged publication, opaque tenant-scoped references, private
  sidecar manifests, verified reads, file modes, no-follow opens,
  signature-based PDF/PNG/JPEG selection, and size/SHA-256 checks. Partial
  publication failures remove both final entries. Removal checks both entry
  types before mutation and removes the manifest first, preventing an invalid
  manifest from causing partial deletion. The generated
  `/crud/{id}/files/{field}` download route requires authentication, uses the
  CRUD read permission, verifies the owning row and tenant/recipient scope, and
  returns only checksum-verified bytes with private attachment headers. CRUD
  detail pages show a localized download link and never display the opaque
  FileRef. The HTTP/storage path is not crash-atomic across filesystem and
  database commits; a crash can leave an orphan. `zelyra files cleanup`
  previews old unreferenced blobs and abandoned staging pairs, rechecks declared
  FileRef columns, caps filesystem entries and fetched reference rows at 100,000
  each, and refuses symlinks or inconsistent paths. It defaults to 30 days and
  requires both `--apply` and `--maintenance-window` to delete. Upload-enabled
  Zelyra servers hold a shared local file lock; cleanup requires the exclusive
  lock, rechecks references under it, and prevents another Zelyra server from
  starting during deletion. This coordinates processes on one local host, not
  unrelated programs or arbitrary network filesystems. A disposable MariaDB
  E2E proves preview, referenced-file preservation, and explicit orphan removal;
  it also starts a real upload-enabled `zelyra serve`, verifies cleanup is
  refused while that process holds the lock, then verifies cleanup succeeds
  after the server exits. `zelyra files backup --output <new-directory>` now
  creates a versioned, private directory snapshot of verified blobs and
  manifests under the exclusive lock; it refuses missing database references
  and never overwrites an existing destination. `zelyra files restore --from
  <directory> --apply` stages and checks every size, media type, manifest and
  SHA-256 before atomically publishing the tenant tree, and refuses missing
  database references or existing uploads. A disposable MariaDB E2E verifies
  byte/manifest restoration and that corrupted input publishes nothing. These
  commands cover only the private file-storage component: database contents,
  app configuration and secrets are excluded and must be backed up separately.
  Replacing a file on an authorized edit now removes the previous blob only
  after the database action commits; cleanup failure leaves a managed orphan
  for the explicit reference-aware cleanup command. Permanent CRUD deletion removes an upload after commit only when no declared
  FileRef still uses it; soft-delete retains the recoverable file. Coordinated
  whole-application database-plus-file backup and crash/failure recovery remain
  open. Non-Unix
  creation fails closed pending ACL support.
  `FileRef` maps to `CHAR(36)`/`UUID`/`TEXT` by backend, checked file forms must
  match a same-named reference column, automatic CRUD cannot render it as text,
  and JSON API inputs reject the opaque reference. A MariaDB E2E verifies that
  the reference and private bytes persist after a successful POST, while a later
  SQL failure rolls back the row and removes only the failed upload. Unscoped
  projects use a project-local storage tenant id of 0; scoped forms require the
  authenticated tenant. Edit forms retain an authorized existing FileRef when
  no replacement is selected, even for required file fields; request-supplied
  references cannot override it. A MariaDB E2E verifies the previous file
  remains readable after changing another field. GitHub CI's rolling toolchain
  advanced to Rust 1.99.1, where Clippy rejects the deprecated atomic
  `fetch_update` API.
  Request-budget reservation now uses a bounded `compare_exchange_weak` loop.
  A concurrent regression test exposed eager `then_some` guard construction,
  which could release bytes after a failed reservation and admit
  more than the configured in-flight budget. The code also checks with local
  Rust 1.98.
  A static analysis of
  `/opt/siedelmann-laravel/app` identified upload
  flows in invoices,
  contracts, settings, postal letters, and inbound mail; the importer does not
  generate them. `tests/mariadb-upload-cleanup-e2e.sh` now simulates a crashed
  upload leaving a staged blob/manifest pair and verifies preview and explicit
  cleanup. The upload POST E2E sends a successful multipart request over a
  local HTTP connection and verifies the stored reference/bytes, then separately
  checks rollback cleanup. File-storage backup intentionally excludes the
  database, configuration, and secrets. A later disposable-MariaDB E2E now
  verifies the paired manual recovery sequence: stop Zelyra, dump the database,
  back up files, remove both data sets, restore the database, then restore and
  checksum-check the referenced files. This is not a single archive or a
  crash-atomic application snapshot; project configuration and secrets remain
  separate. See the
  [1.2.29 secure upload plan](release-plans/1.2.29-secure-uploads.en.md). Released as
  [`v1.2.29`](https://github.com/sf1976/zelyra/releases/tag/v1.2.29); all
  release gates passed, and Linux/Windows artifacts and checksums were
  published.
- [~] **1.2.30 — Laravel import workflow coverage:** The release candidate adds
  tenant-safe scalar references and the checked token-form foundation below.
  This advances, but does not complete, import of the acceptance website.
  Standalone forms now
  support checked local routes with captured path parameters, a compiler-checked
  read-only `verify sql<T>` gate before GET rendering, and `require_match` for
  direct SQL actions. The required first `SELECT` and all later SQL run on the
  same transaction; no match returns 404 before writes, CSRF remains required,
  and verified-token pages are marked `Cache-Control: no-store`. A repeatable
  MariaDB/HTTP E2E exercises the flow against the explicitly unscoped example
  `examples/csv_mailer_unsubscribe.zyl`. For the real Laravel project
  `/opt/siedelmann-laravel/app`, this workflow is currently not generated:
  `csv_mailer_suppressions` is tenant-owned, while its public token route has no
  authenticated tenant context. `tests/laravel_import_unsubscribe_e2e.py` now
  verifies that the table remains tenant-scoped, the unsafe route is omitted,
  and the scaffold documents this limitation. It does not connect to a
  database; the actual project workflow remains an open migration item.
  Tenant-scoped invoice-number collision checks and updates now include the
  authenticated tenant key, and generated master-detail legend locale markers
  survive HTML escaping. Generic number-series definitions, per-tenant
  counters, computed table fields, and FileRef-aware CRUD controls remain
  unsupported; standalone checked file forms and private FileRef storage do
  exist.
  Scalar nominal ID columns may now add `references <table>` while preserving
  their Zelyra type and emitting a real SQL FK. The project linker now rewrites
  module-local tenant aliases on both columns and scope declarations. Based on
  static migration evidence, the importer maps
  `foreignId('tenant_id')->constrained('tenants')` to `TenantId`, table scopes,
  and the real reference. Scoped-to-scoped relations use composite foreign
  keys and the required `UNIQUE (tenant_id, id)` target key. A live MariaDB 11.8
  run confirmed same-tenant references succeed and cross-tenant references are
  rejected by the FK; the repeatable
  `tests/mariadb-tenant-relations-e2e.sh` checks both record-typed and scalar
  nominal references. Generated composite-key names now avoid collisions with
  existing single-column tenant unique constraints. The Laravel importer auth
  E2E sets up the generated identity package in a separate disposable MariaDB
  database and verifies native login, session persistence, and logout; it does
  not import Laravel credentials or rows. Laravel `SET NULL` on such a composite key cannot be
  preserved safely: the review candidate emits `RESTRICT` and records the
  behavioral difference. No Laravel database contents or source files were
  modified.
  Every generated scaffold now includes `IMPORT_COVERAGE.json`, which
  separates source inventory, finding classifications, generated artifacts,
  compiler status, and behavior-test status. The source-only integration test
  checks it against the source-file plan and generation manifest and confirms
  generation did not claim workflow runtime coverage. Its latest run against
  `/opt/siedelmann-laravel/app` accounted for 572 PHP files and 2,263 findings
  (1,387 manual/unresolved and 876 partial).
  The source-only full-project check also asserts the generated native
  login/logout and unsubscribe routes, validates the package lock offline, and
  runs module plan for pkg:imported_identity to verify its package identity
  and declaration closure, then creates an offline source bundle and checks
  its entry and package lock. This verifies module/package distribution
  wiring, not authentication runtime behavior or deployment readiness.
  `tests/laravel-import-auth-e2e.sh` adds the separate runtime proof: against
  both a compact CI fixture and `/opt/siedelmann-laravel/app`, it provisions a
  new destination-only test account and verifies invalid login, successful
  login, persisted session creation, logout, and session revocation through
  local HTTP and disposable MariaDB. It copies no Laravel credentials or user
  rows. The importer now recognizes the project's simple required-Boolean
  `is_approved` guard and emits `auth users { login_enabled: is_approved }`;
  the E2E proves an unapproved test account is rejected and the same account
  can sign in after an explicit approval update. The special administrator
  exception, custom messages, registration, tenant-selection, and 2FA flows
  are still not transferred. The local MariaDB client reports that TLS
  certificate verification is disabled for this password-authenticated
  disposable test connection.
  PHP method recognizer uses the existing lexical source map and balanced code
  braces; regression tests reject method-shaped comments and cover nested
  blocks, typed return declarations, and lookup/update/view patterns that occur
  only in comments or string literals. The real Laravel source was regenerated
  after this change and its main scaffold passed `zelyra check`. Repeat the
  source-only generation and check with
  `python3 tests/laravel_import_project_check.py /opt/siedelmann-laravel/app`;
  the script strips ambient database URLs and removes its temporary scaffold.
  This check does not run the generated HTTP workflow or connect to a database.
  The scaffold
  remains an incomplete migration: a fresh `--all-supported` generation on
  2026-10-21 passed `zelyra check` for its `main.zyl` scaffold, but generated no
  route workflow (`no_complete_supported_workflow_found`). It translated two
  static presentation components, neither wired to a route. The report lists
  2,268 findings (1,391 manual/unresolved and 877 partial across 575 source
  files). These are analysis findings, not a migrated-feature count. A
  standalone translated success component is reused only when the narrowly
  verified unsubscribe workflow is generated; that workflow was not eligible
  in this full-project run.
  Laravel, Composer packages, PHP controllers, Blade runtime files, and source
  credentials are not copied. The source-only integration check compares an
  independently traversed PHP source tree with `SOURCE_FILE_PLAN.json` and
  `MIGRATION_FILES.md`, rejects duplicate or missing entries, and requires a
  category, translation status, and proposed action for each file. This proves
  coverage of the detected PHP file set, not completeness of the behavior
  graph or that source behavior has been translated.
  The latest explicit live-schema run now compares known MariaDB storage types
  (including Decimal precision/scale) and full foreign-key source/target
  column tuples, including composite tenant keys, as well as
  presence, nullability, and indexes. Unknown mappings remain listed
  separately; default values, engine, charset, collation, and triggers are not
  compared. Against the real database it connected read-only, read no
  application rows, inspected 89 tables/1,293 columns, and compared all 80
  generated migration candidates. It reported 297 SQL-type differences, 117
  static-default differences, and 59 foreign-key differences (473 total).
  This supersedes the earlier “zero schema differences” result, which covered
  only the previously documented metadata subset. Static integer, boolean,
  text, Decimal, Time, and `now` defaults are now compared where Zelyra can
  interpret them safely; reports omit the actual default text, and unsupported
  expressions are listed as unclassified. All 59 foreign-key differences are
  live single-column references where generated tenant-scoped relations
  require the composite `(tenant_id, relation_id) -> (tenant_id, id)` key; 29
  also differ in delete action because the safe candidate uses `RESTRICT` where
  Laravel declares `SET NULL`. MariaDB cannot null the relation key while
  preserving the required tenant key under a composite `ON DELETE SET NULL`.
  The importer records these differences for review rather than claiming
  behavioral parity.
  Engine, charset, collation, and triggers remain unchecked. This read-only
  snapshot is specific to the inspected source database and can change as its
  schema changes.
- [ ] **1.3.0 — canonical structure and Bytecode default:** Complete the
  opt-in versioned project layout (`routes/`, `domain/`, `schema/`, `public/`,
  `assets/`, and `plugins/`), architecture checks, and safe layout migration
  preview. Make verified Bytecode the default for `run`, `serve`, and declared
  task execution only after supported-language, SQL, effects, capability,
  contracts, resource limits, diagnostics, and host-operation parity gates
  pass across AST/MIR/Bytecode. Keep AST available as the reference and for
  differential tests. Until parity passes, unsupported Bytecode operations
  must fail closed and the default must remain unchanged; do not claim 1.3.0
  completion by switching the default early. This milestone targets portable
  verified Bytecode, not native machine-code generation.

- [~] **1.3.0 — verified web/compiler capabilities and remaining gaps:** SQL-loaded `Option<Record>` page data, dynamic `page content_type`, static `public/` assets, bounded `run_process_piped` for allowlisted processes, typed successful API responses via `ApiResponse<T>`, typed properties for reusable `view` layouts, and bounded safe CommonMark/GFM rendering are implemented on `main`. Piped process input does not itself execute Zelyra source or provide a web sandbox. Checked static locale references and `/{locale}/...` route selection were merged in PR #155. `HttpResult<T>` remains the result type for outbound HTTP calls. Typed translation calls in arbitrary expressions, Markdown table-of-contents generation, plural rules, and code highlighting remain open.

**Post-1.3.0 or explicitly reprioritized:** a native AOT backend is optional
and depends on benchmark evidence. No open task is silently dropped: work that cannot meet its release
gate stays open and blocks the relevant milestone instead of being represented
as complete.

## Reprioritized work — Laravel website importer

The earlier experimental migrator was removed in v1.1.2 at the owner's request
and its source, tests, reports, and generated scaffold are retained in a local
archive. On 2026-10-20 the owner reprioritized importing Laravel-built websites
for Zelyra migration testing. The archived analyzer is reference material, not
an accepted implementation. The new work must generate idiomatic reusable
Zelyra modules from selected source-backed website behavior and state exactly
what remains unmigrated. It must not recreate Laravel or claim complete PHP
translation. The bounded plan is in the linked 1.2.27 work plan.

### 1.2.34 — ordered, wholly guarded migration callbacks

The current importer hardening change statically skips a `Schema::table`
callback only when its complete supported body is one
`if (!Schema::hasColumn(...))` guard and an earlier migration has already
declared that exact column. Mixed or unsupported callbacks remain subject to
the existing fail-closed review. This improves schema reconstruction for the
acceptance project; it does not import the guarded callback's skipped fields
or claim full Laravel parity.

### ✅ [x] 1.2.35 — preserve unsigned integer storage widths (released)

Released as [`v1.2.35`](https://github.com/sf1976/zelyra/releases/tag/v1.2.35).
The Laravel schema importer preserves the MariaDB physical widths of
`id()`, `foreignId()`, and Laravel's `unsignedTinyInteger()` through
`unsignedBigInteger()` methods while retaining Zelyra's nominal/value types.
The `unsigned_storage` table-column modifier is compiler checked and affects
DDL only: `UInt`, `Id`, and table relations retain their existing Zelyra type
semantics. MariaDB receives the matching `UNSIGNED` type and live-schema
comparison recognizes display-width spellings without conflating integer
widths. Ordinary `UInt` fields use `NUMERIC(20,0)` on PostgreSQL and SQLite;
auto-incrementing IDs and relation columns keep matching native backend key
types. This does not claim MariaDB range parity on those backends. Signed
Laravel integer widths and full schema parity remain separate open importer
work.

### 1.2.36 — reuse AST runtime context across guarded CRUD actions

The release candidate reuses a single AST interpreter and derived program
context across a generated guard and its handler while preserving guard
capabilities, short-circuit behavior, and request-owned SQL transactions. The
change is an allocation/setup reduction; no performance improvement is claimed
without a benchmark. Full CI is required before release.

## Active architecture milestones (1.1.5 through 1.3.0)

These milestones are in progress and unreleased. Items with `[~]` are partially implemented; open subgoals remain explicit. Entries for released versions remain historical evidence.

### 🗺️ [ ] Zelyra 1.1.5 — backend-independent modules and packages

- [~] **Local package import slice:** Existing `import "path.zyl" as alias` remains compatible; experimental `import "pkg:name" as alias` resolves the declared local package entry by dependency key, which may differ from the package identity, and uses existing `pub`, type, cycle, and capability checks. Package entries and internal source imports resolve within their package root and reject `.git/` and `target/`, which are deliberately omitted from package checksums. A generated bundle marks its source closure complete only after an empty unresolved-reference inventory and a successful compiler check; this does not mark deployment complete. A transitive three-source local package example also passes package locking, project linking, MIR verification, and AST/MIR result comparison for pure cross-package record construction and a field read; MIR remains experimental and is not selected by CLI execution.
- [~] **Signed source artifact and static registry foundation — unreleased:** `zelyra package keygen`, `sign`, `verify`, and `trust add|list|remove` use Ed25519 and an explicit project trust file. Versioned source artifacts bind publisher, exact identity/version, compatibility manifest, normalized digest, and compiler-checked source, with limits of 32 MiB and 4,096 files; package-local dependencies are supported only when their full source closure is contained under the package root; separately published transitive dependencies are implemented in the unreleased 1.2.7 worktree. `zelyra package registry build <dir>` creates a deterministic static index, and `package search` displays index metadata as untrusted discovery data. `zelyra package add <index-url> <name> <exact-version> --as <alias>` downloads one exact trusted artifact over HTTPS (loopback HTTP for tests), checks the index and signature, installs without overwrite, records registry/publisher/key/digest provenance in `zelyra.lock`, compiler-validates the tentative project, and rolls back a failed manifest/lock/project update. Bundles relocate cache-backed package roots to portable `packages/<name>/<version>` paths while retaining source provenance. `package update` changes one alias to an exact release or accepts `--plan <file.toml>` for up to 128 aliases. A plan pins every package/version, verifies all artifacts before switching, preserves each publisher key, caps downloads at 256 MiB and five minutes, validates the complete project, and rolls back reported failures. Its integration test covers two aliases, duplicate-alias rejection, an untrusted publisher, digest rejection before commit, compiler rejection after staging, manifest/lock rollback, and a successful offline-locked update. `package remove` validates and rolls back project metadata changes; `package cache clean` previews unused versions and `--apply` removes only lock-unreferenced cache roots after stale-lock and symlink checks. Docker bundles now reject a selected closure without a routable web declaration instead of emitting a `serve` application that fails at startup. Ordinary application-page module settings and integration packages remain open; this is not a completed package ecosystem.
- [x] **Offline package scaffolding:** `zelyra module init <directory> --name <package-name> [--version <semver>]` creates a small, compiler-checkable local package, pins its CLI compatibility to the current minor line, starts with no declared effects, and refuses existing targets. The end-to-end integration test locks, checks, and runs the generated package and verifies invalid names, repeated version flags, and overwrite attempts are rejected.
- [~] Typed public functions/records/enums/generics, exports, dependency graphs, cycle/visibility diagnostics, and capability propagation through imports are implemented and tested. Optional `[package].effects` ceilings reject function effects outside a package's declared set; they do not grant application runtime capabilities.
- [~] **Reusable application modules and web configuration:** `zelyra module list` now shows the loaded module graph, public exports, local package identity/version/checksum, dependency aliases, and declared effect ceilings in deterministic human or JSON output. It never grants runtime capabilities or includes secrets. Login/authentication, import, export, reporting, and similar business workflows should be versioned Zelyra modules reused across projects, not Laravel-specific copies. Packages expose typed settings-schema metadata through `zelyra module list`; required/default/type consistency is checked, secret defaults are prohibited, and values are never included. Non-secret values and environment references for secrets can be persisted in `zelyra.settings.toml`; environment secrets are converted and validated according to their declared scalar type without emitting variable names or values. Invalid, empty, or oversized values are rejected before execution; intermediate values are zeroized. `zelyra module settings check` validates types, required fields, defaults, and secret availability without emitting values. JSON output explicitly includes whether each setting is required. `zelyra run` and `zelyra serve` refuse to start with missing or invalid required values without disclosing secret names. Package code can read its own declared values using `module_setting<T>("name")`: literal names and exact schema types are checked statically, runtime values are scoped by source/package and redacted in Debug output. This access is integrated with AST, MIR, and bytecode; this does not establish complete parity of the experimental backends. Zelyra Studio now provides a protected local editor for typed package values with revision checks, atomic writes, and rollback on invalid configuration; existing secret values and environment-variable names are neither read nor displayed. When package settings and an auth `admin_permission` are configured, `zelyra serve` also exposes the schema-backed `/__zelyra/module-settings` page, protected by that permission, same-origin CSRF checks, server-side schema validation, and revision checks. It never displays stored secret values or environment-variable references; changes apply to new runs and require a server restart. Missing required settings must be provisioned before `serve` starts, using Studio or the CLI. Secure secret management beyond process environment variables and general-purpose capability-aware settings forms remain open. No UI grants capabilities implicitly. Local package reuse and a CLI inventory alone do not fulfill this goal.
- [~] **Business data exchange modules:** Reusable local `csv_export` and pure `csv_import` packages are included in `examples/business_modules`. Export provides deterministic UTF-8 encoding and root-checked file output under an explicit `FileSystem` effect; import decodes bounded input, requires non-empty headers, rejects rows with mismatched column counts, previews at most 100 data rows, and maps uniquely named source columns to explicit target columns. A CLI integration test checks valid and malformed input, preview limits, missing/ambiguous columns, and duplicate targets against AST, MIR, and Bytecode. CSV documents are currently bounded to 16 MiB and handled in memory. A preview UI, type conversion, ODBC import, transactional database apply, PDF printing, Excel-compatible export, and wider backend compatibility remain open; the example does not complete the business-data exchange goal.
- [~] **Runnable modular deployment evidence:** `examples/customer_orders_modules` has been checked and exported as a standalone Docker source application. The generated-project Docker E2E also ran the multi-module customer/order workflow and separately exported and started the customer and inventory modules against an isolated MariaDB test service. This verifies the exercised paths only; the bundle manifest correctly keeps `complete_deployment: false`, and other backends and application configurations remain unverified. A package-backed example composes the root app, a billing package, and a separate storage/database package. Its runtime now uses `sql_decode` before treating SQL rows as an array. An optional isolated-MariaDB integration case bootstraps a uniquely named database and executes the root → billing → storage call; it passes and cleans up. A Docker image built from the current local source snapshot also runs this call against the disposable MariaDB and prints `0`. The Dockerfile now runs `zelyra check main.zyl` with its pinned compiler during image construction. A negative Docker build using the older `e33e101` pin fails there with `E-MOD-001` for `pkg:storage`, so an incompatible pin cannot produce an image. A package-aware published compiler pin still needs validation. A Compose smoke with commit `bfc177655edae4e175014bff1aa0baedcc63b192` built and started the package-backed web bundle; `GET /` returned the rendered page. Docker bundles without pages, forms, CRUD, API, or auth routes are now rejected before writing. The generated bundle still advertises `complete_deployment: false`, and resource-specific HTTP behavior and published-image compatibility remain open. Revalidated on 2026-10-17 with `tests/generated-project-docker-e2e.sh`, a local CLI built from this worktree, Compose test ports 28081–28084 and 23309, and the full module compiler pin `84c1d4e2a0f9f7b968d91dfd51a121a2cf2ad1c1`. The run passed generated-app setup/recovery/CRUD, the multi-module business workflow, and separate build/start/HTTP and permission checks for the customer and inventory exports. Only test-owned containers, networks, volumes and temporary files were removed; unrelated running containers were left untouched. This is evidence for those source-module and MariaDB paths, not general package/backend or deployment parity.
- [~] Versioned Rust HostServices API v7 is connected to the AST interpreter for clock, environment, randomness, console, HTTP, mail, processes, filesystem, and database ([current contract](runtime-host-services.en.md)). MIR and bytecode now use the same provider for their implemented slices. Bytecode v24 supports local mutability metadata and inline and stored capturing lambdas with typed capture layouts, plus static and dynamic named function-value `map`/`filter`/`find`/`reduce` lowering and direct calls through typed named function values held in locals with capability checks in addition to enum payload construction and MIR-lowered match statements/expressions, and `now()`/Clock, `env(name)`/Environment, and `read_console(prompt)`/Console and `random_int`/`uuid_v4`/Random, plus the complete implemented root-checked FileSystem set: `read_text`/`read_text_result`, `file_exists`/`dir_exists`, `file_size`, `list_dir`, `write_text`/`write_text_result`, `write_text_new_result`, `atomic_write_text_result`, `append_text_result`, `delete_file`/`delete_file_result`, `copy_file_new_result`, and `rename_file_result`; all use the injected host and configured filesystem policy. Bytecode also supports allowlisted `run_process` through the injected host; artifact capabilities are verified and execution requires explicit grants, while `execute` defaults to no grants. Bytecode supports `run_process`, `http_get`, `http_request`, `http_json`, and `http_result` in addition to the listed Clock, Environment, Console, Random, and FileSystem calls; `mail_deliver` also uses the injected host after explicit `Email` grant and shared SMTP/network policy validation; tests use a fake transport. Bytecode v21 executes checked SQL and `mail_queue` operations with optional PDF attachments in one injected transaction batch; referenced views are embedded and use the shared bounded text renderer. `Email` and `Database(write)` are preflighted before message fields are evaluated. Other transaction effects remain unsupported. MIR has broader but still partial coverage. SQL, network, and mail policy checks remain in the runtime before calls. This API is not a stable native plugin ABI.
- [ ] Support pure Zelyra packages first. Design a safe, explicit extension mechanism for Rust-hosted integrations; do not expose Rust's unstable native ABI as a public plugin contract. Consider versioned host APIs and later a stable C ABI or component model if dynamic loading is justified.
- [ ] Preserve compile-time SQL checks, contract semantics, explicit effects, sandboxing, and human approval for destructive operations. Modules must not bypass security through native entry points.
- [ ] Prove extensibility with a reference database-driver adapter and a read-only banking integration design (FinTS/HBCI and EBICS), including explicit authorization, secret isolation, audit, and mock-based tests. Do not claim universal banking support without provider-specific interoperability tests.
- [ ] Document package publishing/trust policy, dependency and supply-chain security, compatibility rules, CLI/Studio integration, diagnostics, positive and negative tests, and runnable examples.
- [ ] Release gate: no 1.1.5 completion claim without passing compatibility, capability-denial, dependency, offline/reproducibility, and integration tests.

### ✅ [x] Business CRUD findings from the 1.2.30 review

The five findings were completed in published releases 1.2.31 and 1.2.32.

| Finding | Delivered result | Remaining limit |
| --- | --- | --- |
| Reusable sequential document numbers | 1.2.31 provides declarative `auto_sequence`, transactional allocation, tenant/year scope, forged-value protection, and a source-mapped SQLite rejection (`E-DB-001`). | Numbering is intentionally not gapless. |
| Tenant scope for number allocation | 1.2.31 applies the verified tenant to both legacy invoice-number checks and generic sequence allocation. Database CI covers MariaDB, MySQL, and PostgreSQL. | None for the reported finding. |
| Server-computed fields | 1.2.32 recomputes typed values for generated CRUD and validated API POST/PUT/PATCH, rejects forged writes, checks dependencies, and validates Decimal precision/rounding. Partial PATCH merges omitted inputs from the existing tenant-scoped row before validation. | AST/MIR/Bytecode parity remains part of the 1.3.0 gate. |
| Localized master-detail labels | 1.2.32 resolves escaped locale tokens in child-collection legends; fallback/project catalogs and invoice-tutorial locale references are covered. | None for the reported finding. |
| CRUD attachments | 1.2.32 adds bounded `FileRef` policies, generated upload controls, private storage, and permission/tenant-checked downloads. MariaDB and headless Chromium E2E cover upload, download, MIME, redaction, and cross-tenant denial. | Separate DB/file backups are required; no single crash-atomic snapshot is provided. |

Both releases passed their release gates and are published. This summary does
not claim crash-atomic combined backups or full AST/MIR/Bytecode parity. See the
[business CRUD completion plan](release-plans/1.2.31-1.2.32-business-crud.en.md).

### ✅ [x] Zelyra 1.2.33 — experimental backend safety checks (released)

Released as [`v1.2.33`](https://github.com/sf1976/zelyra/releases/tag/v1.2.33).
The release aligns source-level `--max-steps` accounting for the supported
AST, MIR, and bytecode execution slices while keeping the VM instruction cap.
Differential coverage compares one transaction containing a SQL audit insert
and one queued PDF email across AST, MIR, and encoded/decoded bytecode. This is
a bounded test scope, not general backend, transaction, rollback, or host
effect parity. AST remains the default; MIR and bytecode remain explicit
experimental opt-ins. Linux and Windows archives, checksums, SPDX SBOMs, and
attestations were published; downloaded release assets passed SHA-256 checks.

### 🗺️ [ ] Zelyra 1.3.0 — typed MIR and Rust bytecode VM

AST interpreter scopes, path/host built-ins, and filesystem metadata/I/O have
been modularized incrementally. Native SQL row decoding is now shared by AST,
MIR, and bytecode. Table-derived `Invoice[]` projections travel as separate
SQL metadata through MIR and bytecode serialization. The `examples/business_modules`
path runs on MariaDB in all three backends. This is not complete backend
semantic parity.

- [~] **Typed MIR and experimental execution slice:** `zelyra-runtime` exposes source-mapped inferred types only after a successful full-program type check. `zelyra-mir` builds and verifies explicit basic blocks for `if`, `while`, `loop`, `for`, `match`, `break`, `continue`, and `return`, and structurally lowers expressions into typed MIR nodes with child IDs and source spans. Its verifier checks CFG dominators, branch and invariant types, signatures, typed assignments, and array-index/tuple updates (`E-MIR-VERIFY-002`). The experimental MIR interpreter executes verified control flow, user-function calls, records, fields and checked copy updates, arrays and indexes, opt-in mutable array updates, tuple construction, indexing and destructuring, collection/range loops, enum payload construction, lambda callbacks with lexical captures in `map`/`filter`/`find`/`reduce`, and stored capturing lambdas that can be passed, returned, and invoked within a MIR execution, and expression and statement matches with guards for a differential-tested subset of scalar semantics. It includes pure implementations for `Some`, `None`, `Ok`, `Err`, `len`, `append`, `contains`, `range`, `to_string`, Unicode text operations (`text_contains`, `text_trim`, `text_chars`, case conversion, prefix/suffix checks, replacement, and split), and `parse_int`/`parse_float` with Result errors. It also implements `abs`, `min`, `max`, `clamp`, `sqrt`, `pow`, `floor`, `ceil`, `round`, and the `decimal` constructor, the array built-ins `sort`, `sort_desc`, `reverse`, `slice`, `concat`, and `join`, and the set built-ins `set`, `set_add`, and `set_remove`. Date built-ins include `date_add_days`, `date_diff_days`, their Result-returning variants, and the Option/Result/Date predicates `is_ok`, `is_err`, `is_some`, `is_none`, `is_before`, and `is_after`. Step and call-depth limits are enforced on a bounded runtime thread stack; tests compare supported executions to the AST interpreter. Clock, environment, random, console, process, filesystem, HTTP, simple SQL query execution, and `mail_deliver` use injected HostServices; capability declarations, optional project grants, and the relevant process, filesystem-root, network, and database policies are checked before provider access. HTTP policy and response validation, SQL parameter conversion, and database schema/storage validation use shared AST runtime helpers. MIR's SQL slice captures parameter LocalIds during HIR resolution, binds those values through the shared placeholder scanner, validates schema/storage once per execution, and routes queries through the host. SQL transaction blocks lower to a verified MIR transaction node and execute through one injected `execute_database_batch(..., true)` call. The supported body contains direct SQL expressions, SQL let-statements, and standalone `mail_queue` calls with optional PDF attachments; PDF bytes use the same runtime view renderer as AST and the shared outbox validator. SQL retains HIR-resolved parameter LocalIds, runtime capability checks, and shared database storage validation. Differential fake-host tests compare SQL batches, outbox inserts with and without PDF attachments, typed SQL lets, provider failures, missing database context, denied grants, and invalid MIR references. Parallel `await` bindings now execute in scoped MIR worker threads with snapshot inputs, join-before-binding behavior, shared step accounting, and the same capability/policy host checks. Other transaction contents remain fail-closed. A transitive local `pkg:` module graph now loads through the project linker, lowers to verified MIR and bytecode, and executes with the AST-equivalent result in both backend tests and a CLI `--backend bytecode` integration test. An injected-host differential test also verifies imported `Console` access with a grant and zero provider calls when denied. The exercised paths remain experimental; the structural MIR interpreter is distinct from bytecode. `zelyra run --backend mir` now opts into MIR for its supported subset, while AST remains the default and `--backend ast` explicitly selects it. MIR rejects unsupported operations and defaults to a 100,000-step budget when none is configured. The versioned bytecode v28 slice is selectable only through the experimental opt-in `zelyra run --backend bytecode`; AST remains the default and unsupported programs fail closed. MIR and bytecode now execute stored capturing lambdas within the same program run; bytecode v22 encodes typed capture layouts and enforces capture types and capability ceilings. Snapshot closure captures now match static mutability rules: lambdas may read captured mutable bindings but writes through captures are rejected by the type checker; shared mutable capture cells remain open. Full closure/backend parity remains open. Complete built-in semantics, effect inference, optimizations, broad differential coverage, and default backend selection remain open; Bytecode v20 supports verified parallel `await` branches with snapshot captures and a shared step budget; a shared four-branch maximum is checked by the type checker, MIR and bytecode verifiers, and each runtime before workers start; the experimental CLI opt-in does not establish feature parity. Bytecode v24 currently captures `print` output through success and runtime errors, supports inline-lambda plus static and dynamic named function-value `map`/`filter`/`find`/`reduce` lowering and direct calls through typed named function values held in locals, type-checked record copy updates, enum payload construction, and MIR-lowered match statements, and supports injected HostServices for clock, environment, console, random, process, HTTP, mail delivery, and every currently implemented filesystem operation (`read_text`, `read_text_result`, `file_exists`, `dir_exists`, `file_size`, `list_dir`, `write_text`, `write_text_result`, `write_text_new_result`, `atomic_write_text_result`, `append_text_result`, `delete_file`, `delete_file_result`, `copy_file_new_result`, and `rename_file_result`), with serialized declarations, explicit grants, and configured filesystem-root validation; a verified preflight authorizes each host call before argument evaluation. A bounded deterministic bytecode v22 encoder/decoder is implemented for this supported subset; lowering now lives in `mir/src/bytecode/compiler.rs` and its differential/verifier suite in `mir/src/bytecode/tests.rs`, while `mir/src/bytecode.rs` retains the shared instruction model, verifier, and VM. This is an internal organization change with no intended API or language-semantic change. Persistent caching remains open. MIR and bytecode execute function preconditions and postconditions at runtime, but do not prove them. Runtime HostServices API v7 abstracts clock, environment, randomness, processes, filesystem, database access, console input, HTTP transport, and mail delivery. See the [1.1.5–1.3.0 work log](release-plans/1.1.5-to-1.3.0-progress.en.md).
- [~] **Execution model audit:** The AST reference backend in `runtime/src/ast_interpreter.rs` and its evaluation modules recursively evaluates typed AST expressions/statements; public execution APIs and interpreter initialization live in `runtime/src/execution_api.rs`. It enforces bounded steps/call depth, runtime contracts, and injected HostServices with capability and resource policies. MIR lowers checked HIR to verified source-mapped CFG and executes a tested subset; bytecode v28 uses an iterative frame stack and a verified bounded artifact format; v28 serializes typed property metadata for embedded views. Shared operator, pure-builtin, and atomic step-budget implementations reduce semantic duplication. Runtime values (`Value`, `FunctionValue`, `SqlRowsMetadata`, `ClassObjectRef`) and map normalization now live in `runtime/src/value.rs`; filesystem, network, process, mail, and aggregate runtime policies live in `runtime/src/policies.rs`. Existing public re-exports remain unchanged. AST evaluation components now live in `runtime/src/builtin_evaluation.rs`, `runtime/src/call_evaluation.rs`, `runtime/src/expression_evaluation.rs`, `runtime/src/statement_execution.rs`, `runtime/src/json_codec.rs`, `runtime/src/host_effects.rs`, `runtime/src/host_builtins.rs`, `runtime/src/filesystem_metadata_builtins.rs`, `runtime/src/function_execution.rs`, `runtime/src/environment.rs`, and `runtime/src/path_builtins.rs` under the internal `ast_interpreter` namespace. `Environment::with_scope` centralizes cleanup of block, loop, match, and lambda scopes even when runtime errors return; pure path helpers, filesystem metadata operations, and clock/environment/random/console host built-ins are split out of the large builtin dispatcher into focused interpreter components. Shared capability analysis and static type checking live separately in `runtime/src/capabilities.rs` and `runtime/src/type_checker.rs`; both retain shared language semantics for AST, MIR, and CLI. The formal contract verifier is isolated in `runtime/src/verifier.rs`; this is a structural change with no intended language-semantic or API change. Further decomposition and full backend parity remain open. AST, MIR, and bytecode now share overflow and over-limit attempt accounting, including parallel branches; the first over-limit attempt is counted and later concurrent failures cannot inflate the counter. The number of charged operations remains backend-specific. A coverage-guided fuzzer now targets the bounded bytecode decoder and accepted-artifact encode/decode round trips. One real 15-second run on this host processed 783,917 inputs without a crash. A second 10-second run started from a valid artifact generated from `fn main() -> Int { return 7 }`, exercising the accepted encode/decode path as well. These are bounded runs, not a broad security proof. AST remains the complete reference and default. Full AST/HIR/SQL/effect/closure/concurrency parity, bytecode policy surfaces for unsupported effects, broad property/fuzz coverage, and reproducible performance benchmarks remain open; see the execution audit in the 1.1.5–1.3.0 work log.
- [~] **Bytecode mail delivery:** Bytecode v19 supports `mail_deliver(event_id, payload_json)` through the shared HostServices mail path. Its verifier enforces arity, `Result<Unit, String>`, `Email` declaration, and preflight before argument evaluation; execution also requires an explicit grant and the common SMTP/network policies. AST/bytecode differential tests use fake transports. Bytecode v24 supports checked SQL and `mail_queue` operations with optional PDF attachments in one transaction batch. The binary artifact carries referenced local views, and `render_pdf` uses the same bounded, text-oriented renderer as AST and MIR. The VM queues messages only; it does not send them. Other transaction effects remain unsupported, and static SQL expressions require an explicit database context.
- [~] **Typed MIR foundation:** checked HIR lowers into source-mapped typed expressions and explicit CFG; lowering reuses the AST capability analyzer and reports capability violations as `E-MIR-EFFECT-001` before producing MIR. The verifier checks control-flow, signatures, locals, expression references, local mutability for assignments, array-index updates, and existing tuple/parallel targets, selected update invariants, capability declarations, static function-call edges, and calls through typed local function values (`E-MIR-EFFECT-001`). For indirect calls it validates the local function type, arity, argument types, result type, and required capabilities; runtime call entry rechecks the caller's declared capability ceiling. Direct built-in and SQL effects are also checked through the shared AST capability mapping. MIR and bytecode now execute stored capturing lambdas within the same program run. Bytecode v24 encodes typed capture layouts, enforces capture types and capability ceilings, and rejects writes to snapshot captures; format v25 also serializes local mutability and exact initialization sites, rejecting later writes to immutable locals; full closure/backend parity and broader MIR coverage remain open.
- [~] **MIR scalar constant folding:** The explicit `optimize_scalar_constants` API folds verified `Int`, `UInt`, `Float`, `Bool`, and `Char` unary, binary, and binary-chain expressions through shared AST/VM operator helpers. It leaves calls and other nonliteral/effectful operands untouched, preserves runtime failures and their spans, and verifies MIR before and after transformation. An explicit `zelyra run --backend mir --optimize-mir` opt-in invokes this pass; ordinary `run` and unoptimized MIR remain unchanged. The step budget counts executed MIR steps after optimization; a differential test confirms the optimized sample consumes fewer steps. Tests also compare AST and optimized MIR results, division-by-zero diagnostics, observable function output, and CLI rejection of incompatible backends. Broader optimization passes remain open.
- [~] **Bytecode v28 execution slice:** format v28 serializes typed view properties (name, type, and source span) for embedded view definitions; this preserves checked property metadata without adding VM instructions. The supported execution slice in `mir/src/bytecode.rs` compiles a reachable call closure over the supported scalar and aggregate subset from verified MIR into a deterministic versioned binary format and executes it with an iterative frame stack, source spans, explicit source-step budget markers, a separate VM operation ceiling, and stack/control-flow/call-arity verification. Bounded encode/decode APIs structurally verify artifacts and reject malformed input. AST, MIR, and bytecode delegate value-level unary/binary operators and shared pure built-ins for conversions and Unicode text operations including `text_contains`, `text_trim`, `text_chars`, `text_to_lower`, `text_to_upper`, `text_ucfirst`, `text_starts_with`, `text_ends_with`, `text_replace`, `text_split`, `text_index_of`, `text_repeat`, `text_is_empty`, `text_capitalize`, `text_trim_start`, `text_trim_end`, `text_is_blank`, `text_substring`, `parse_bool`, `text_replace_result`, `text_split_result`, `text_pad_left`, and `text_pad_right`—to shared runtime helpers; the bytecode compiler still applies its documented supported-type subset. Enum payloads and MIR-lowered match statements/expressions with guards have encode/decode differential coverage. Differential tests cover arithmetic, branches, short-circuit logic, scalar `while`/`loop` control with `break` and `continue`, single-binding integer `range` loops (including descending and empty ranges), array/set iteration with tuple destructuring, array/tuple construction and indexing, array concatenation, Decimal arithmetic, Duration addition/subtraction, temporal ordering, mutable array updates including `push_mut`/`pop_mut`, structural Array/Tuple/Option/Result equality, new/existing tuple assignment, typed generic record construction, record spread/copy-with-update including generic type arguments, field reads and Array<Record> iteration, typed Map literals/lookup with duplicate-key semantics, inline-lambda `map`/`filter`/`find`/`reduce` with lexical capture and encode/decode AST/MIR parity, function pre/postcondition and loop-invariant execution (including break, continue, returns, and nested returns), shared pure built-ins, budget failures, malformed tuple rejection, fail-closed unsupported effects, explicitly granted Clock/Environment/Console/Random/FileSystem/Process host calls plus `http_get` and `http_request`, with verified preflight before argument evaluation, and SQL expressions and SQL transaction batches revalidated against the checked schema and actual parameter types before database host calls; no-attachment outbox operations are capability-preflighted and batched without sending email. See the [bytecode guide](bytecode.en.md). Persistent caching, signatures, effect inference, PDF-attachment `mail_queue` transactions and mixed effects, full language support, broad differential coverage, and default selection of the bytecode backend remain open; the experimental CLI opt-in does not establish feature parity. Trusted runtime policy must remain authoritative.
- [~] Share runtime semantics and host services between the reference AST interpreter and VM. Pure standard-library operations and value-level unary/binary operators now use shared runtime dispatch across AST, MIR, and bytecode execution. Bytecode compilation still restricts operators to its documented subset; broader helpers do not silently expand the accepted bytecode format. Host services and selected SQL/policy helpers are also shared. A bounded transaction mixing a SQL audit insert with a PDF-attachment `mail_queue` now has differential AST/MIR/decoded-bytecode evidence: the test compares transaction mode, SQL order, and complete serialized parameters across both writes. This covers one queued message only; multiple-message, failure/rollback, and other transaction effects still need explicit parity tests. Complete capability/effect handling, contracts, broader error behavior, resource accounting, and module APIs still need a systematic parity pass.
- [~] **AST runtime construction shared:** Program execution and direct function execution use one interpreter constructor. HostServices, capability grants, and filesystem/network/process/mail/package policies are grouped in an internal `ExecutionContext`; parallel AST workers clone this context as one unit instead of reconstructing each security field independently. Immutable functions, record definitions, enum definitions, and view ASTs are borrowed from the program into a shared `Arc<ProgramContext>`; names and scoped-table metadata are indexed once, so workers do not clone those declaration trees. Type aliases and derived SQL projection metadata remain indexed runtime data. Runtime type-alias expansion is precomputed once per program context rather than rebuilding and cloning the alias map for each value check. Database transaction handles remain separately scoped so borrowed request transactions are not copied into workers. This is a structural change with no language-semantic or public API change; broader backend parity and iterative AST evaluation remain open.
- [~] **Shared interpreter resource limits:** AST and MIR now use the same helper for the default and hard-capped call depth and for calculating bounded worker-stack allocation; MIR parallel branches use the same stack policy. This removes backend-local cap literals but does not make the recursive interpreters iterative.
- [~] **Structured AST type checker:** Control-flow/match-exhaustiveness analysis lives in `runtime/src/type_checker/flow.rs`; type relations, generic inference, alias expansion, and type-annotation validation live in `runtime/src/type_checker/type_relations.rs`; binary operator checking lives in `runtime/src/type_checker/expression_operators.rs`; built-in, user-function, and constructor call checking lives in `runtime/src/type_checker/call_expressions.rs`. Return/fallthrough, loop, guard, operator, and call rules are unchanged by these structural splits. Runtime and phase-one tests pass. This is internal restructuring; further decomposition, flow diagnostics, and complete MIR/bytecode parity remain open.
- [ ] Keep the AST interpreter selectable until the VM reaches complete supported-language compatibility. Add differential, property-based, fuzz, security, and regression tests.
- [~] Opt-in CLI execution is available for AST, MIR, and the supported bytecode subset. MIR/bytecode inspection, safe versioned caching, stable machine-readable diagnostics, and Studio integration remain open without breaking existing CLI behavior.
- [~] A reproducible CPU benchmark compares mutable array updates across AST, MIR, and Bytecode; its method and three release runs are recorded in the [benchmark report](benchmarks/mutable-array-backends.en.md). I/O workloads, startup, memory, throughput, and compilation cost remain open.
- [~] A deterministic, bounded scalar differential corpus now generates 96 integer expressions and checks each result against an independent host-language oracle, AST, MIR, encoded/decoded Bytecode, and VM execution. This increases arithmetic regression coverage only; it does not establish full operator, error, resource, or language parity.
- [~] Checked `Int` arithmetic failures are compared across AST, MIR, and decoded Bytecode: add/subtract/multiply overflow, negation of `Int::MIN`, division/remainder by zero, and `Int::MIN / -1` or `% -1` preserve diagnostic messages and exact source spans. This covers these eight cases only; backend-specific step accounting and broader diagnostics remain open.
- [~] A deterministic boundary corpus checks add/subtract/multiply/divide/remainder across seven values around `i64::MIN`, zero, and `i64::MAX` against Rust checked-arithmetic results. Successful values and rejected-operation messages/spans are compared across AST, MIR, and decoded Bytecode; this is a bounded integer-operator slice, not general numeric or runtime parity.
- [~] A second boundary corpus checks UInt add/subtract/multiply/divide/remainder across zero, small values, and `u64::MAX` boundaries against Rust checked arithmetic. AST, MIR, and decoded Bytecode must agree on successful values, error messages, and exact spans. UInt underflow is kept distinct from overflow; this adds bounded unsigned-integer evidence only.
- [~] A Decimal boundary corpus compares add/subtract/multiply/divide/remainder across AST, MIR, and encoded-then-decoded Bytecode. It checks ordinary fixed-point values, `i128` addition/subtraction/multiplication overflow, division by zero, and exact error messages/source spans. This is targeted Decimal operator evidence only; it does not establish scale/formatting or general numeric parity.
- [~] Configured recursion-depth limits are compared across AST, MIR, and decoded Bytecode: the same recursive function succeeds at depth 24 with limit 40 and fails at limit 20 with the same call-site source span. This verifies the tested depth boundary, not complete stack-resource parity.
- [~] **Common source-level step budget:** MIR now charges source-level statements and loop iterations, and Bytecode v27 embeds source-step spans on the following instruction where possible and uses a local serial counter, avoiding standalone `Step` dispatch in the common case. Differential tests sweep budgets 0–63 over a while loop, 0–95 through function calls/contracts, 0–127 over collection loops, match/if expressions including a bound guarded match arm, and 0–128 for `break`/`continue`. SQL-only transaction batches are checked at 0–63. Parallel `await` workers run unequal 40- and 80-iteration loops under budgets 0–300; a built CLI sweep also checks recursive and parallel programs across all three backends. Tested results, success/failure boundaries, diagnostics, and source spans match, and no database batch starts after budget exhaustion. Bytecode keeps a separate 10,000,000-op execution ceiling for malformed or hand-built artifacts. Mixed-effect transactions, parallel host effects, and all remaining source constructs still need coverage; the default-backend gate is not met. See the [progress log](release-plans/1.1.5-to-1.3.0-progress.en.md). The v27 instruction-step fusion and serial local counter reduced the measured v1.2.28-relative slowdown from 25.8% to 12.7% on that mutable-array workload; the remaining difference and other workloads need measurement before backend-default consideration.
- [~] **Host-effect and transaction parity:** Differential tests cover two parallel awaited `http_get` calls and a SQL audit insert combined with a rendered-PDF outbox write across AST, MIR, and encoded/decoded Bytecode. Parallel HTTP preserves branch binding and rejects missing `Network` grants or disallowed hosts before transport. For the PDF batch, a fake host verifies that all backends send one two-write transaction and produce the same injected database-error message and source span. This checks the batch boundary, not a real database rollback or external-side-effect rollback. Other host operations, timeout/cancellation, race behavior, and broader transaction parity remain open.
- [~] The zero-step source-span mismatch is closed, and source-step markers now align tested statement/loop budgets across AST, MIR, and decoded Bytecode. The budget parity scope and remaining gaps are recorded in the progress log; this is not full runtime parity.
- [ ] Release gate: no default-VM switch or production-readiness claim before feature parity, safety checks, differential tests, documented limitations, and reproducible builds.

### Cross-cutting business-platform extensions added to the 1.2.0–1.3.0 goal

These requirements come from the project owner's `/root/5erweiterungen.md` brief and are now part of the active objective. Existing foundations are partial; none of the rows below means the whole feature is complete. See the [detailed bilingual plan](release-plans/1.3.0-business-platform-extensions.en.md).

The AST implementation remains the reference, but it is not sufficient by itself: every language or AST semantic added for these extensions must also be safely represented and differentially checked in MIR and bytecode, or fail closed with a clear unsupported diagnostic in that backend.

- [~] **1.2.0–1.3.0 compiler-enforced tenancy and typed queries:** First scope slice: `table ... scoped by tenant_id: TenantId` is parsed and type-checked (`TenantId` must resolve nominally to `Id`; the column must be required). `zelyra check`, AST, MIR verifier/runtime, and bytecode verifier/VM reject raw SQL against these tables; bytecode v24 carries the metadata and checks it against the checked project schema at database execution. Authenticated tenant CRUD remains allowed only with the same scope column. Since 1.2.17, optional table relations are nullable foreign-key columns, and relations between tables with the same tenant identity use composite tenant/record foreign keys plus a unique `(tenant_id, id)` target key; incompatible tenant identity types and tenant-key `SET NULL` are rejected. The authorized tenant context, query HIR and safe scoped reads/writes are assigned to 1.2.8; final backend parity is a 1.3.0 gate. Do not claim automatic N+1 prevention without independent evidence.
- [~] **1.2.0–1.2.5 request/response and forms:** Existing `Request`, Page/Form/CRUD and CSRF paths are the foundation. Form `widget` values are finite and checked; `password` renders as a password input, and forms do not replay passwords or values from fields named with `password`, `token`, `secret`, or `hash` (`E-FORM-006` rejects invalid widgets). The 422 validation path preserves safe submitted values, escapes them, marks invalid fields, and has an integration test that also verifies secret redaction. A typed handler/middleware/response lifecycle and complete redirect/notice contract remain open; credentials and sensitive input must never be replayed.
- [x] **1.2.0–1.2.9 durable jobs and scheduler runtime:** Declarative
  `command`/`schedule` syntax, service lifecycle, and MariaDB-backed durable
  delivery are implemented. Only the installed active Linux systemd project
  service enqueues deterministic per-minute occurrences. A serial worker
  revalidates current source, renews leases, uses bounded retries/dead-letter
  handling, and reports jobs without payloads. `schedule status` reports the
  service state; `schedule jobs` reports durable occurrences. Manual `serve`
  does not run the scheduler. `[runtime].max_steps` limits evaluation steps;
  `[runtime].scheduler_job_timeout_seconds` bounds handler wall time (default
  300 seconds, range 1–86,400). Setup and service startup cancel pending,
  unleased jobs for removed declarations. No host cron is installed and
  foreign crontabs are untouched. Delivery is at-least-once; missed offline
  minutes are not backfilled, and exactly-once external effects are not
  promised.
- [~] **1.2.8–1.3.0 Wasm and sidecar bridge:** Versioned HostServices and local locked source packages exist. Sandboxed/versioned Wasm computation and explicitly launched, authenticated local sidecar RPC are assigned to 1.2.8, with cross-backend and release parity in 1.3.0. No stable Rust ABI, auto-download, or implicit capability grant. A local mock integration precedes any real provider claims.
- [ ] **Cross-backend release gate:** Query, tenant, request, and job semantics must match across AST, MIR, and bytecode or be rejected with explicit unsupported diagnostics. Add security, MariaDB integration, compatibility, offline, and adversarial tests. Do not claim 1.3.0 completion or production readiness until these gates and the detailed plan are satisfied.

### Additional enterprise extensions through 1.3.0

The six work packages from `/root/weitereerweiterungen.md` are now staged roadmap goals, not claims about implemented behavior. Dependencies, security boundaries, and acceptance steps are recorded in the [detailed plan](release-plans/1.3.0-business-platform-extensions.en.md).

- [ ] **1.1.5 — architecture contracts:** Define syntax and semantics for state machines, auditable/bitemporal tables, ledger, LiveUI, and bundled deployment. The deterministic checked JSON `zelyra reflect` slice is already implemented (`docs/reflect.en.md`); its schema evolution and remaining export formats need versioning rules. Design work alone does not count as implementation.
- [ ] **1.2.0 — safe foundations:** Prepare typed metadata and compiler diagnostics for managed transitions, audit tables, and ledger. Specify LiveUI protocol/session boundaries and bundle format, embedded assets, health endpoints, and explicit migration behavior. Live database schema inspection, Markdown/OpenAPI and richer semantic relations remain open.
- [ ] **1.2.8 and 1.3.0 — verified vertical slices:** Deliver tested, bounded state transitions with contracts/outbox, transactional ledger balancing, audit/history queries, versioned `reflect` exports, LiveUI, Wasm/sidecar integration, and deployment bundles. Each executable path must be covered by AST/MIR/bytecode or rejected fail-closed by unsupported backends; final parity and distribution gates belong to 1.3.0. Bitemporal `as_of` queries depend on Query HIR. Universal compliance, high-load, diff-latency, or zero-ops claims are not acceptance criteria without measurements and backend evidence.
- [ ] **Continue after 1.3.0 if gates remain open:** Full bitemporal history, production-wide LiveUI, portable static bundles, and additional `reflect` formats must not be declared complete merely because a version milestone has arrived.

**Shared safeguards:** Managed state writes and ledger postings are checked before commit and are atomic. Direct UPDATE/DELETE paths cannot bypass audit history; history DDL and `as_of` respect schema and database boundaries. LiveUI sessions, queues, and patches are bounded and isolated per session. `--auto-migrate` is opt-in and cannot open the application port before plan/policy checks succeed. `reflect` exports checked project metadata and does not claim immutability or completeness without evidence.

### Canonical project layout and architecture rules through 1.3.0

The requirements in `/root/struktur.md` are staged roadmap work. Its tree and
syntax examples are a target design, not evidence that the manifest fields,
autoloading, scheduler syntax, or `zelyra migrate-layout` already exist.
Verify the current parser, module resolution, project configuration, and
compatibility behavior before changing syntax. See the detailed
[project-layout plan](release-plans/1.3.0-project-layout.en.md).

- [ ] **1.1.5 — Inventory and architecture contract:** Inventory project files,
  `zelyra.toml`, `use`/package resolution, `zelyra new`, build/bundle paths,
  and route/SQL/domain boundaries. Define `routes/`, `domain/`, `schema/`,
  `public/`, `assets/`, and `plugins/` responsibilities without implicitly migrating
  existing projects.
- [ ] **1.2.0 — Opt-in profile and module foundation:** Implement a versioned,
  backward-compatible profile and explicit namespace resolution where they fit
  the existing module architecture. Let `zelyra new` create it as an explicit
  template. Avoid implicit autoloading until import graphs, ambiguity, and
  diagnostics are specified.
- [ ] **1.3.0 — Checker and safe migration:** Add source-mapped architecture
  rules for route mutations, HTTP-independent domain code, and declared state
  machines. Initially, `migrate-layout --check` reports deterministic
  findings and suggestions only; writes require preview, conflict checks, and
  recovery. Accept bundles and starter projects only for supported
  assets/plugins. Projects without the opt-in profile remain in legacy mode.
- [ ] **After 1.3.0 if needed:** Automatic multi-file moves remain follow-up
  work until recovery and platform tests pass. The bounded Wasm/sidecar scope
  is assigned above; broader runtime support is not implied.

### ◻️ [?] Post-1.3.0 — optional native AOT backend

- [ ] Evaluate Cranelift after profiling shows a material need. Reuse typed MIR and the shared runtime, preserve capability and contract guarantees, and avoid an independent divergent language implementation.
- [ ] Decide on AOT only from benchmark evidence, deployment requirements, and maintenance costs.

### Language features audit — four gaps closed; VM backend remains experimental

- [x] User-defined payload enums and exhaustive matching are parsed, checked,
  and executable. The type-system examples and AST/MIR/bytecode tests cover
  payload arity/types, guards, round trips, and rejection of non-exhaustive
  matches.
- [x] String interpolation type-checks embedded expressions, supports literal
  braces with `{{`/`}}`, and reports source-mapped errors. The technical
  reference documents that interpolated text is not HTML-escaped output.
- [x] Generic structs and records support explicit/inferred type arguments,
  field checks, updates, and JSON round trips; the bytecode test compares the
  supported subset against AST execution.
- [x] Opt-in mutable arrays support checked O(1) index assignment and
  `push_mut`/`pop_mut`. AST, MIR, and bytecode tests cover supported updates,
  immutability errors, bounds errors, and differential results. This does not
  claim a general array optimizer or a benchmarked numeric-performance target.
- [~] Portable versioned bytecode and an iterative VM are implemented for a
  documented subset, with differential tests. AST remains the default and
  reference; complete language/effect/security parity and all 1.3.0 gates are
  still open. Native machine-code AOT remains an optional post-1.3.0 decision
  based on measured need.

## Current milestones

- [x] **1.2.12 application settings and tutorial polish — released:** [v1.2.12](https://github.com/sf1976/zelyra/releases/tag/v1.2.12) includes the authenticated, CSRF-protected, revision-checked settings page with schema validation, secret redaction, and safe atomic writes; simpler dialog defaults; Studio access to `zelyra.theme.css`; responsive invoice CRUD styles; and integration tests. PRs #107 and #108 passed full CI. Release workflows built reproducible Linux and Windows archives and attested them; all four archives and installers passed SHA-256 verification. The Linux binary reports `zelyra 1.2.12`. Apache/TLS testing and MIR/Bytecode parity remain open.

- [x] **1.2.13 project layout foundation — released:** [`v1.2.13`](https://github.com/sf1976/zelyra/releases/tag/v1.2.13) adds `zelyra new --layout standard`, validates real layout roots, and resolves schema/domain/route namespaces only for opted-in projects. Full PR CI passed; Linux/Windows archives were built reproducibly and attested, all four archives/installers passed SHA-256 verification, and the Linux binary reported `zelyra 1.2.13`. Architecture rules and migration preview stay open until their implementation is verified; `plugins/` is not included.






- [x] **1.0.5 Studio logs:** Added the Studio Logs panel and a local JSONL
  store for Studio run/setup events and application or preview output, with
  level and text filters, secret redaction, bounded rotation, and owner-only
  Unix permissions. See the [logs guide](studio-logs.en.md).
- [x] **1.0.5 installation-aware uninstall:** Added `zelyra uninstall` for the
  active user-owned executable, including binaries outside the default install
  directory. It reports duplicate PATH entries, identifies package-manager-
  owned installs, and requires confirmation before removal; project files and
  data remain untouched. See the [uninstall guide](uninstall.en.md).
- [x] **1.0.5 invoice data entry:** `issued_at` is a localized date-only field
  with a matching German or English format hint. The server assigns a unique
  invoice number from the inserted record ID and keeps it unchanged on edits.
- [x] **1.0.5 invoice home and overview:** `/` now guides the customer, item,
  and invoice workflow; `/views/invoicedashboard` is presented as the invoice
  overview, with consistent German and English labels.
- [x] **1.0.5 invoice tutorial walkthrough:** Rewrote the generated English
  and German guides to explain setup commands, options, source files, and
  important lines. They trace generated `/invoices/new` routes to
  `crud Invoice -> invoices` and document `zelyra routes main.zyl`.

- [x] **1.0.6 Zelyra UI:** Integrated a
  locally bundled, versioned UI framework with compiler-checked components,
  semantic presentation variants, themes, and progressive interaction.
  Generated CRUD, Studio layouts, the invoice tutorial, and MariaDB starters
  use Zelyra UI; Tabler loads only when explicitly selected as the stylesheet.
  Bar, line, and donut charts plus a print-ready invoice with browser PDF output
  are available. The email component opens a local draft; attach a saved PDF
  manually. Automatic PDF attachment delivery and a server-side PDF renderer
  are not included. Full Tabler class compatibility is not claimed. See the
  [Zelyra UI guide](ui.en.md) and the
  [work log](release-plans/1.0.6-zelyra-ui-progress.en.md).

- [x] **1.0.7 language, runtime, and PDF mail — released:** Adds
  value-producing if expressions, record updates, typed lambda pipelines,
  match guards and nested record patterns, tuple assignment, and a bounded,
  configurable recursion depth. `--max-steps` counts interpreter statements
  and evaluations under one shared budget, including `parallel` branches.
  Tests also protect nominal IDs, `Result`-bound API errors, offline
  transaction checking, view slot fallbacks, auth handlers, and `parallel`
  joining. `render_pdf("ViewName", record)` creates a local searchable text
  PDF; the mail outbox can attach a validated PDF. Server PDF output does not
  render CSS, images, or charts. Released as
  [`v1.0.7`](https://github.com/sf1976/zelyra/releases/tag/v1.0.7); see the
  [1.0.7 work log](release-plans/1.0.7-language-runtime-progress.en.md).

- [x] **1.1.0 Studio/LSP — released:** Offline single-binary Studio with
  embedded CodeMirror 6, compiler-backed analysis, shared overlays, verifier
  and typed-hole views, pipeline/nullability map, isolated static canvas,
  theme and capability tools, budget execution, PDF/outbox views, semantic
  rename and recovery-journaled file rename. `zelyra lsp` uses the same
  semantic service. Adds checked string interpolation and opt-in mutable local
  arrays. Known boundaries: analysis jobs are bounded but not cooperatively
  preemptible; rename is limited to supported statically resolved symbols;
  server PDF output is text-oriented. See the
  [1.1.0 release notes](release-notes/1.1.0.en.md) and
  [work log](release-plans/1.1.0-studio-deep-native-progress.en.md).

- [x] **1.1.1 Studio and port-conflict handling — released:** Expected browser
  disconnects no longer clutter Studio stderr, successful web-server startup
  is informational output, and Canvas explains the separate explicit action
  needed to open the running application. `zelyra serve` binds a free local
  port when its requested port is occupied; `zelyra doctor` reports that case
  as a warning when a fallback is available and reads the project `.env` by
  default. Outbox connection/table warnings
  explain the optional setup and can be dismissed when unused. Verified with
  Chromium against a running local web app and unreachable MariaDB, fallback
  port unit tests, and the cold Compose invoice Studio flow. See
  [1.1.1 release notes](release-notes/1.1.1.en.md).

- [x] **1.1.2 Laravel migrator removal — released:** Removed the experimental
  Laravel migration tool from the product and preserved its source and prior
  analysis in an archive outside the repository. Published as
  [`v1.1.2`](https://github.com/sf1976/zelyra/releases/tag/v1.1.2); its release
  workflow completed for Linux and Windows artifacts.

- [~] **1.1.5 reusable modules and packages — active:** Finish the offline,
  compiler-checked local package system, typed module APIs, backend-independent
  host services, safe settings, and runnable modular deployment evidence.
  Package trust, extension boundaries, application-page settings, exchange
  modules, and release gates remain open; see the detailed 1.1.5 section above.
  No Laravel import is part of this milestone.
- [~] **1.2.0 operations and deployment foundation:** Add staged command,
  scheduler and job workflows, typed request/forms and scoped query
  foundations, `public/`, production serving guidance, Apache/Nginx reverse
  proxy examples, and Linux systemd service installation with an explicit
  fixed port.
- [x] **1.2.5 operations and package foundations:** Fixed-port Linux service,
  resident scheduler, static `public/`, signed registry foundation, and
  portable package bundles are implemented and tested. Query/request/job
  workflows are now staged across 1.2.7–1.2.9.
- [x] **1.2.6 package recovery:** Recover interrupted package mutations; see the
  release note and recovery work log above.
- [x] **1.2.7 trusted transitive packages:** Published with successful PR CI
  and reproducible, attested Linux/Windows release artifacts. Three-level add,
  update-plan, offline run/check, bundle, provenance, bounded resolution, and
  cycle rollback have integration coverage.
- [ ] **1.2.8 typed query/request/forms:** Complete bounded query HIR and typed
  tenant/request/form lifecycle across supported backends.
- [x] **1.2.9 durable jobs:** Transactional scheduler occurrences, leases,
  bounded retries, idempotency, dead-letter handling, and service lifecycle
  are delivered; see the release plan and 1.2.9 notes.
- [~] **1.2.10–1.2.15:** Deliver module settings/HostServices, backend parity,
  tested business verticals, canonical project structure, and Bytecode gates as
  assigned in the release sequence at the top of this roadmap.
- [~] **1.3.0 interpreter architecture — active after 1.2.0 foundations:**
  Continue typed MIR and the opt-in bytecode VM while keeping AST as the
  reference. The goal is verified Bytecode by default for `run`, `serve`, and
  tasks only after full parity and security gates pass. Until then, unsupported
  operations fail closed and the current default stays in place. This targets
  portable Bytecode, not native machine-code generation. See the detailed
  1.3.0 section above.
- [~] **Laravel website import — active through 1.3.0:** Replace the removed framework-oriented migrator with a source-evidenced website importer that emits compiler-checked reusable Zelyra modules. Analyze first, require explicit selections, preserve unsupported behavior as findings, and keep the Laravel source read-only. Use `/opt/siedelmann-laravel/app` as the first integration fixture. It must compose with the 1.1.5 package/module system and 1.2.0 operations foundation rather than introducing a parallel plugin/runtime model. Release 1.2.34 preserves Laravel `mediumText`/`longText` capacity as `String text_storage medium|long` for MariaDB, rejects unsupported MariaDB indexing of those columns, and reports backend-specific PostgreSQL/SQLite limits. Release 1.2.35 also preserves supported unsigned integer widths. This does not make the site importer complete; route behavior and workflow migration remain open.

- [ ] **1.2.0 — Compose check in the installer:** Installers detect
  Docker Compose v2 or newer and distinguish it from obsolete Python Compose
  v1. Docker remains optional for the core installation. When Compose v2 is
  missing, interactive Linux installers offer to install the Compose v2 plugin
  from supported package sources after explicit confirmation; otherwise they
  show suitable installation guidance. No elevated changes happen without
  confirmation or in non-interactive runs. The guidance explains that
  Docker-based tutorials cannot start until Compose is installed.
- [ ] **1.2.0 — interactive console menus:** A simple declarative
  menu API and `zelyra new --template console-menu` create a complete terminal
  menu from a title, description, and choices in only a few lines. The menu
  provides a polished presentation with a clear selection highlight, keyboard
  navigation, a layout for narrow terminals, and a numbered fallback. Color
  and Unicode output respect terminal capabilities, `ZELYRA_COLOR`, and
  `NO_COLOR`; terminals without ANSI support receive a plain-text menu. EOF
  exits cleanly without hanging. The starter, a runnable example, and tests
  cover selection, EOF, narrow widths, color, and fallback behavior.
- [x] **1.0.2 Studio and invoice tutorial reliability:** Resolve generated
  MariaDB port placeholders consistently in Studio and the launched app. On
  first Run, start Compose and apply the schema only when the active URL matches
  the generated local database URL in `.env`; retry transient startup
  disconnects and show persistent setup errors. External or overridden URLs do
  not trigger automatic schema changes. The invoice database map and row preview
  work after startup. Program output is more readable at 13 px without changing
  the code editor font. Browser tests cover a fresh Compose database and an
  external MariaDB project; MariaDB and PostgreSQL Studio flows also pass.
- [x] **1.0.2 stepped integer ranges:** Support `for i in 1..21 step 2` with
  exclusive ends, a default step of 1, positive and negative steps, empty
  direction mismatches, and a checked nonzero step. `range(start, end, step)`
  provides the same sequence as a bounded array. Parser, runtime, type, and
  boundary tests pass.
- [~] **Language ergonomics follow-up:** Typed `if` expressions, record updates,
  lexical lambdas, guarded record patterns, tuple returns/destructuring, and
  context-sensitive field names are implemented and tested in 1.0.7. User-
  defined operators and labeled loop control remain outside the supported
  grammar; no custom operator precedence is configurable.
- [x] **String and math library:** Unicode-aware text helpers, checked math,
  rounding rules, error results, typed interpolation with escaped literal
  braces, and pipeline composition have compiler/runtime coverage. Interpolated
  strings are plain text; HTML escaping still occurs at the view boundary.
- [~] **Collections and algorithms:** Typed `map`, `filter`, `find`, and
  `reduce` pipelines, captured lexical lambdas, tuple `entries`, checked
  `map_increment`, and opt-in mutable arrays with indexed updates and
  `push_mut`/`pop_mut` are implemented. The runnable exercise project covers
  sorting, search, stack behavior, recursion, and dynamic programming; dedicated
  stack types remain follow-up work. The project now runs with matching
  expected output on AST, MIR, and Bytecode. Verified MIR and Bytecode v25
  support `push_mut`/`pop_mut`; the bytecode artifact encodes and verifies
  those mutations. Functional arrays and maps otherwise retain value semantics.
- [x] **1.2.5 — interactive Studio console:** Studio Run streams prompt output
  from the real Zelyra process, accepts one bounded input line, resumes the
  same process, and supports explicit EOF, cancellation, a 15-second limit, and
  secret-redacted output. The CLI remains interactive; browser E2E runs the
  same console program in CLI and Studio and compares their output.
- [x] **1.2.5 — idiomatic algorithm exercises:** `examples/algorithms/` now
  contains executable insertion sort, linear search, an array-backed stack,
  bounded recursive factorial, and bottom-up Fibonacci. The project has a
  20,000-step budget, per-algorithm input limits, documented expected output,
  and CLI integration tests for edge cases.
- [~] **Compiler diagnostics:** Targeted errors and source spans exist for
  syntax, types, callbacks, pipelines, and runtime limits. Broader recovery
  suggestions for nested errors remain an improvement area.
- [x] **1.0.4 invoice tutorial restart reliability:** Give each newly generated
  tutorial a unique stable Compose project namespace. For older marked invoice
  tutorials, recover from stale database credentials by starting an isolated
  database while preserving the previous volume. Accept Compose v2 minor-only
  version output from either output stream. Compose 5.6.0 / Engine 29.8.2
  verified both this recovery and a fresh start; all six tutorial routes
  returned HTTP 200 and sample counts were 2 customers, 3 items, 1 invoice,
  and 2 lines. The port-fallback path selected DB port 3307 when 3306 was busy.
- [x] **1.0.3 invoice tutorial startup through CLI and Studio:** On Compose
  port conflicts, automatically select a free next port for MariaDB or the web
  application, update the local `.env` and matching MariaDB URLs, and retry a
  bounded number of times. The invoice tutorial now has repeatable local sample
  data. Studio Run starts the matching local database service, applies the
  schema, loads the seed only for the marked local tutorial project, and then
  starts the preview. CLI and Studio were run against Docker Compose 5.6.0 /
  Engine 29.8.2; both HTTP dashboards and the stored sample records were
  checked.
- [x] **1.0.1 published-artifact language review:** 80 cases, 100 task repeats,
  600 numeric comparisons and 1,408 calendar/UTC reference cases passed with
  no unexpected findings; published installation/upgrade/rollback and
  MariaDB/PostgreSQL HTTP/SQL checks also passed. See the
  [verification report](release-readiness/1.0.1-language-review.en.md).
- [x] **1.0.1 tutorial startup fix:** Require Compose v2 or newer and reject
  Python Compose v1 before startup. Distinguish `ContainerConfig` failures and
  Ubuntu crash-report permissions from Docker socket access. Retain generated
  projects and database volumes for retry. Fix Studio Run entry-point selection
  to restore console execution and web previews; see the [tutorial guide](tutorials/invoice.en.md).
- [x] **0.8.0 stably released:** Zelyra Studio includes a local read-only
  database map for MariaDB and PostgreSQL plus guided CRUD source preview,
  compiler checking, and explicit apply. It also includes a bounded MariaDB
  outbox, compatibility fixtures for language line 0.1, and published
  Linux/Windows artifacts with SBOMs. Exact evidence is in the
  [0.8.0 candidate verification](release-readiness/0.8.0-candidate-verification.md).
- [x] **0.9.0 stable release published:** Guided Studio business workflows,
  invoice tutorial improvements, preview-bound `zelyra text edit`, and the
  expanded language corpus are released. Optional local TOTP is limited to
  MariaDB with persistent sessions and audit. Exact CI, Linux/Windows artifacts,
  checksums, SBOMs, attestations, and the bilingual website handbook are
  verified in the [0.9.0 release record](release-readiness/0.9.0-candidate-verification.md).
  Human acceptance stays reserved for 1.0.0.
- [x] **0.9.1 stable release published:** Studio session, save, Problems panel,
  and invoice tutorial workflows are extended; language line 0.1 uses `#` and
  `##` comments. Full CI, Linux/Windows artifacts, checksums, SBOMs,
  attestations, and website updates are verified in the
  [0.9.1 release record](release-readiness/0.9.1-candidate-verification.md).
  Independent human acceptance remains reserved for 1.0.0.

- [x] **0.9.3 stable release published:** Studio read-only database browsing
  with bounded pages, checked source-column edits, confirmed MariaDB creation,
  a read-only Laravel inventory, native values, and CSV/labyrinth examples are
  released. Main CI and release packaging passed on the exact tag commit;
  artifacts, checksums, SBOMs, and attestations are published. Type coverage
  remains 17 supported, 5 partial, and 3 unavailable. See the
  [0.9.3 candidate verification](release-readiness/0.9.3-candidate-verification.md). Human product acceptance
  remains reserved for 1.0.0.
- [x] **0.9.5 stable release published:** typed application mail queues through
  the MariaDB transactional outbox and an explicit SMTP worker with required
  TLS, authentication, private CA roots, bounded messages, and redacted errors.
  Versioned Laravel findings cover authorization, tenant-like fields, money,
  and file operations. The release remains experimental, mail delivery is at
  least once, and PostgreSQL outbox support, templates, and attachments are
  deferred. Exact release CI, MariaDB 10.11/11.4/11.8/12.3, MySQL 8.4,
  Linux/Windows builds, SBOMs, and attestations are attached to the
  [0.9.5 release](https://github.com/sf1976/zelyra/releases/tag/v0.9.5).
  Human acceptance remains reserved for 1.0.0.

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
  throttling now uses a declared MariaDB table and a keyed fingerprint shared
  across instances; independent security review and the complete account
  lifecycle evidence remain open, so this does not complete the milestone.
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
  Option, Result, and exhaustive pattern matching as statements and expressions.
  `Int`/`UInt` overflow, division by zero, and more than 20 nested function
  calls return controlled runtime errors. `Unit` is usable in type signatures.
  `examples/type_system` records tested coverage and unsupported areas.
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
  Views also declare typed properties, and pages pass statically checked
  literal or page-scope values for shared layout data such as titles and meta
  content. Optional properties may be used by CRUD and TableView layouts.
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
  exist; `Database(read)`, `Database(write)`, and `Email` are implemented with
  runtime grants and policy checks. A separate `Jobs` effect remains planned.
  AI must never add an effect silently.
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

- [🧪] **Language compatibility:** the initial enforcement in 0.7 accepts
  `zelyra = "0.1"`, rejects unsupported explicit lines with `E-LANG-001`, and
  preserves older projects without the field. The language line remains `0.1`
  through 0.9.9; compiler release numbers do not change it automatically. 0.7
  now includes the first
  runnable accepted/rejected/runtime corpus. Version 0.8 expands grammar,
  typing, and formatter coverage; 0.9 adds runtime, module, effect, contract,
  and SQL fixtures; 0.9.7–0.9.8 audit reference and corpus gaps. Version 0.9.9
  freezes
  grammar and semantics as a release candidate and allows only documented
  corrections. Only 1.0.0, after its independent human acceptance gate, makes
  the contract production-ready.
  See the [language compatibility guide](language-compatibility.en.md) for the
  rules and acceptance criteria.
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
  now has a 24-frame recursion limit, focused regressions, and a retained crash
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

## Release 0.7.0

- [🧪] **Experimental release:** local Studio, bounded typed MySQL 8.4.11 SQL,
  invoice tutorial, initial language-0.1 corpus, and a MariaDB tenant CRUD path
  for one declared table. The maintainer security review and mutation rollback
  E2E are recorded in the 0.7 plan. This is limited to MariaDB, one tenant-
  owned table, and CRUD without relations; it is not the complete multi-
  database tenant CRUD required for 1.0.0. No tenant migration or backfill is added.
  Independent security review and human acceptance remain pre-production gates
  for 0.9.9/1.0.0. See the [0.7.0 plan](release-plans/0.7.0.en.md).

## Release 0.8.0

- [🗺️] Add a read-only visual database map and guided CRUD builder. A developer
  selects a connection and table, inspects fields and relationships, previews
  compiler-checked source, and explicitly applies the change. Expand language
  fixtures across grammar, typing, and formatting. Include the bounded local
  MariaDB transactional outbox already under review, with explicit worker
  invocation and at-least-once delivery semantics. See the
  [0.8.0 plan](release-plans/0.8.0.en.md).

## Release 0.9.0

- [x] **Stable release published:** guided
  Studio business workflows, the invoice path, preview-first readable regex
  text editing, and expanded language fixtures are implemented. Database
  claims are limited to the exact server matrix and workflow evidence. Optional
  MariaDB TOTP remains a preview pending independent production security
  review. See the [0.9.0 plan](release-plans/0.9.0.en.md), release notes, and
  [release verification record](release-readiness/0.9.0-candidate-verification.md).

## Product direction: language, compiler, database, and Studio

Zelyra's core product idea is one connected path from readable source through
the compiler and database to a finished business application. Studio should
take on as many common, safely modelled language and framework tasks as possible:
explain valid choices, guide workflows, show errors, and generate checked,
readable Zelyra source. Diagrams and presentations are examples of that broader
direction. Shared JavaScript modules should remain ordinary reusable project
files, with cross-file navigation and diagnostics from Studio. The roadmap
stages these goals across 0.9.2–0.9.8; they are not capabilities of the
released 0.9.0.

The Laravel Migrator is also an implementation driver for Zelyra itself. When
an accepted Laravel mapping is blocked by a missing Zelyra language, compiler,
database, runtime, or Studio capability, record that gap against the release
that needs it and implement it within 0.9.3–0.9.8. Do not label a mapping
"direct" until its generated project runs through the promised workflow. Keep
genuinely ambiguous or unsafe business behavior as explicit manual work, with
its source evidence and reason.

## Release 0.9.1 — IDE foundation

- [🧪] Studio already provides the project explorer, tabs, visible save state,
  undo/redo, project-root containment, and file/span diagnostics. Source-file
  rename and delete require visible confirmation and a current content hash;
  existing destinations are never overwritten, and `main.zyl` and
  `zelyra.toml` are protected. Rename does not update import references. Browser
  end-to-end verification now covers session start and reload, unsaved changes,
  keyboard save, compiler diagnostics, and restart. The session token is removed
  from the address bar and kept for that tab session. API and filesystem checks
  cover stale edits, path traversal, secret files, protected files, and explicit
  CRUD apply. Access remains loopback-bound, and the compiler remains
  authoritative. The tagged `v0.9.1` passed the full CI; Linux/Windows artifacts,
  checksums, SPDX SBOMs, and attestations were published. The bilingual website
  handbook is live; see the [0.9.1 plan](release-plans/0.9.1.en.md).
- Exit evidence: Chromium browser coverage, editor API/security checks, and
  MariaDB/PostgreSQL Studio workflows pass locally. CI must repeat them against
  the exact release candidate. The invoice example opens and runs from the
  workspace.
- [🧪] The invoice tutorial shows net, 19% VAT, and gross amounts from stored
  lines and provides a print-ready view the browser can save as PDF. The German
  standard rate is only an assumption for this learning example; this is not a
  native PDF engine or a complete legally compliant invoice template.
- [🧪] Language line 0.1 uses `#` for line comments and standalone `##` lines
  as non-nesting block-comment delimiters. Existing `//` comments remain
  compatible.

## Release 0.9.2 — IDE language services

- [✅] Add safe UTF-8 text-file operations for exclusive creation, atomic file
  replacement, append, copy, rename, and Result-based deletion. Literal text
  search, replacement, and splitting support the multi-file console notes
  manager in `examples/notes_manager`, including pre-change backups, restore,
  filesystem boundaries, action logging, and automated regression tests.
- [✅] Add the type-checked `assert(condition[, message])` runtime intrinsic.
  A false condition becomes a runtime error, so the inventory/orders, CSV, and
  notes-manager suites produce a failing process status instead of only printing
  a failure line. Inventory state uses atomic replacement and has a Zelyra-level
  save/load round-trip check. `text_trim` removes Unicode whitespace at text
  boundaries; CSV import uses it to reject required fields containing only
  whitespace.

- [🧪] The guided tutorial command starts MariaDB and the web app, then applies
  the schema. The Compose web app gets its database URL from project setup, so
  beginners need not set `DATABASE_URL` or run separate `db setup` and `serve`
  commands. Its responsive theme styles the dashboard, CRUD forms, lists, and
  print view.

- [🧪] Grow Studio into an IDE with a context-sensitive properties sidebar.
  Selecting a file, route, table, field, or diagram element shows the relevant
  valid settings in clear language. Common workflows should use selection and
  properties instead of requiring users to memorize commands or chart APIs.
  While writing, users can request the description and options for the current
  command. The current work already adds file properties, F1 help for core
  language terms, a filterable action finder on Ctrl+Shift+P, and
  compiler-checked diagnostics while typing, suggestions from the compiler
  project model, and F12 navigation to definitions in saved project files. A
  literal project search and Shift+F12 lexical identifier occurrences are also
  available for saved `.zyl` files. Occurrences are not scope-aware semantic
  references. Ctrl+Space also completes parameters and in-scope `let`, loop,
  and match bindings from valid unsaved source. Type inference and
  symbol-level properties remain open. Studio can explicitly run `main.zyl`
  with a 15-second timeout, bounded output, and redaction of recognized
  environment and `.env` secrets. This remains an experimental local task. A
  compiler-backed project map now displays module imports, declaration
  ownership, CRUD/table links, page/view links, and uniquely resolved API
  handlers, with source navigation and explicit incomplete status. It omits
  dynamic and database-query relationships.
- [🧪] Continue the IDE language services: add scope-aware semantic references,
  type-aware completion, symbol/route properties, integrated verify/test tasks,
  and diagnostics that link across the project. Complete deterministic project,
  module, route, and resource diagrams. Keep claims scoped to language 0.1 and
  mark unsupported relationships incomplete instead of guessing.
- Exit evidence: positive and negative multi-file projects verify contextual
  properties and help, completions, live diagnostic ranges, navigation,
  formatter idempotence, task output, and deterministic diagram data;
  unsupported syntax is identified instead of guessed.

## Release 0.9.3 — database workbench

Scope and technical gates are in the [0.9.3 release plan](release-plans/0.9.3.en.md).

- [✅] Add CSV foundations for import and export: parse `String[][]` as a
  nested type, correctly decode and encode quoted delimiters/quotes/newlines,
  and convert decimal text to scaled integers without Float. The
  `examples/csv_catalog` project creates source data, validates fields, logs
  errors, sorts/exports valid records, and automatically checks the round trip.
  CSV is processed as a complete UTF-8 string in memory; streaming is not
  included. The Zelyra tests use `assert` so failures are reflected in process
  status.
- [✅] Add `text_chars(text)` to return Unicode code points as `String[]`. The
  multi-file `examples/labyrinth_solver` uses it to load and validate variable-
  sized grids, run breadth-first search for a shortest path, render results and
  counters, and verify a file round trip.

- [🧪] Fix the invoice tutorial startup defect found after 0.9.2: declarative
  web layouts must load the project theme; the CLI prints the local browser
  address when scaffolding and preserves it when schema setup fails. Studio's
  **Run** action executes console programs and starts a local preview for web
  projects with a clickable address. A new preview replaces the prior one and
  Studio stops the active preview when it closes; **Stop preview** ends it
  immediately. Databases still start explicitly and separately. The tutorial
  is generated as a real imported multi-file project with separate database,
  customer, item, invoice, layout, and dashboard modules. CLI and Studio
  browser E2E evidence covers the web workflow; human acceptance remains for
  1.0.0.
- [🧪] Expand CLI help: `zelyra <command> --help` and nested calls such as
  `zelyra db plan --help` explain purpose, options, examples, and prerequisites
  in German or English without running the command.
- [🧪] Start a read-only Laravel project analyzer in the Zelyra CLI:
  `zelyra migrate laravel <project> --report <path>`. The first static inventory
  already detects
  Laravel/package versions, routes and middleware, controllers, models,
  migrations, validation, views, jobs, and integrations, and links findings to
  source locations. It must not boot the application, invoke Artisan, connect to
  a database, or read `.env` files. Reports distinguish directly mappable,
  partially mappable, manual, and unresolved items and identify a safe next
  step. Start with a documented supported-pattern subset; ambiguous matches
  remain unresolved rather than guessed. The first release scans Composer
  constraints, routes, models, controllers, schema creation, relationships,
  validation entry points, queued jobs, Blade views, and selected integrations
  with source locations in Markdown or JSON. It creates only a new report
  outside the source project. Versioned compatibility mapping, linked project
  graph, package/rule analysis, and scaffold generation continue in 0.9.4–0.9.8.
- [🧪] Grow the IDE database view into a safe workbench: inspect tables,
  columns, indexes, and declared relationships; browse records read-only by
  default; show how source modules own each object. Make the ER diagram
  filterable and navigable so selecting a field or relationship reveals its
  declaration module and using resources. Studio now offers a session-protected
  read-only data browser capped at 25 rows per request and a compiler-checked
  helper to add/edit simple source columns. A declared MariaDB database can be
  created after confirmation when a connection is already configured.
  Connection selection, PostgreSQL/MySQL database creation, and the complete
  reviewed plan/risk view remain open.
- [🗺️] Deliver foreign-key relationships as an end-to-end database feature by
  1.0.0: declare references in Zelyra table fields, validate target table and
  column types in the compiler, create and inspect real constraints, and show
  relationship metadata in the Studio map. Ship the supported create/change
  workflows separately for MariaDB, MySQL, and PostgreSQL. Schema plans must
  detect orphan rows before adding a constraint, preserve data on failure, and
  require review for risky changes; unsupported actions fail closed. The
  default delete behavior is restrictive. Any explicit cascade or set-null
  behavior must be declared, type-checked, and covered by each backend's
  integration matrix before it is advertised.
- [🗺️] Extend tenant-filtered generated CRUD beyond the experimental MariaDB
  path: bind tenant selection to the authenticated user's active membership,
  require the resource permission, and enforce the tenant predicate in list,
  detail, create, edit, update, delete, and restore operations. Tenant keys are
  server-bound and cannot be selected through submitted fields or exposed as
  ordinary CRUD filters. Once foreign keys land, related-record choices and
  writes must also prove that referenced rows belong to the active tenant.
  Custom SQL actions on tenant-owned data must be tenant-aware or be rejected
  at compile time. Exit evidence includes two-tenant positive and negative
  integration cases for every mutation and read path, including membership
  revocation and forged/conflicting selectors.
- [🧪] Let users add and edit table columns through the properties sidebar.
  Show the complete schema plan, impact, and risks; run compiler checks and
  require explicit confirmation before applying changes. Let users choose the
  database backend and connection, then create the project database from Studio
  where the backend and granted privileges support it. Clearly explain
  unsupported combinations and never store or print credentials in the browser.
  Add/Edit for simple source columns is compiler-checked, hash-bound, and
  explicitly confirmed. Key, unique, and defaulted columns cannot be edited
  visually. Backend/connection selection and PostgreSQL/MySQL database
  creation remain open; MariaDB creation uses an existing configured URL.
- [🗺️] Prepare typed, read-only data sources for metrics so 0.9.4 can offer
  selections without free-form SQL. Mutations and generated schema changes
  require a reviewed plan and explicit confirmation.
- [🗺️] Plan reusable JavaScript ES modules as project files: keep shared
  functions once in a clear asset/client area and import them from multiple
  pages. Studio should find imports and exports, show cross-file references,
  and provide relevant help and error locations. Common browser tasks can also
  have simple Zelyra building blocks; those must not replace normal JavaScript
  or existing JS libraries. Specify module wiring, asset output, security
  boundaries, and interoperability before implementation.
- Exit evidence: separate MariaDB, MySQL, and PostgreSQL tests match the published
  database matrix. Unsupported schema actions fail closed, stale plan hashes or
  database state block apply, read-only browsing is enforced, and credentials
  never enter editor output, URLs, diagnostics, or browser storage.

## Release 0.9.4 — CLI, setup, and tutorial reliability (complete)

- [✅] Audited every documented top-level and nested CLI command, its options,
  help output, successful path, and expected error path. Reproduce the reported
  setup and database problems, correct the behavior and explanations, and keep
  an English/German command matrix with regression coverage. `db create` must
  clearly emit SQL only; `db setup` and `setup <directory> --all` must identify
  their actual database and schema effects.
- [✅] Made setup progress visible while Docker Compose builds and starts the
  application. Surface the final schema-setup failure details without exposing
  credentials, and print the actual application URL after startup.
- [✅] Kept the invoice tutorial's protected `.env` available with and without
  `--no-start`. Verify from Studio that the preview uses the project database,
  then exercise customer form load, create, redirect, and list. When a query
  fails, show a safe next step to check the database and applied schema. Open
  the default browser after successful tutorial startup and print the URL in
  headless environments.
- [✅] Create a missing MariaDB database from Studio: after explicit user
  confirmation, use protected local Compose admin credentials when available,
  otherwise try the configured account. Do not require a host `mariadb`
  executable or send credentials to the browser.
- Exit evidence: bilingual command/help coverage has no missing documented
  command; `setup --all` is exercised against an isolated MariaDB Compose
  project; the Studio-compatible customer flow passes against MariaDB; and the
  complete workspace test suite and release metadata checks pass.

## Work staged for 0.9.5–0.9.8

The product work below remains part of the 1.0.0 plan, but is staged so each
release has testable acceptance criteria instead of treating the full IDE and
language backlog as one 0.9.4 change.

- [🧪] Language type coverage: exclusive range loops, payload-free nominal
  enums, calendar/UTC timestamp operations, Decimal quantization, and safe
  value-semantic record/array updates are added in 0.9.8. The 0.9.9 branch adds
  named typed function values, bounded generic functions, and payload enums.
  Generic user-defined records/enums remain open; a Zelyra-native
  identity-bearing business entity is not required unless a concrete use case
  demonstrates a need. Keep records value-based and preserve the explicit
  `Int`/`UInt` and `Float`/`Decimal` model. Update compiler, runtime,
  diagnostics, compatibility fixtures, Studio help, and `examples/type_system`.
  Before the 0.9.9 freeze, positive, expected-compiler-error,
  expected-runtime-error, and serialization checks must pass with no unexpected
  failures.
- [🗺️] Extend the Laravel analyzer from inventories to evidence-linked route,
  middleware, controller, model/relation, migration/table, and view graphs.
  Emit Markdown and JSON with source locations, package and validation findings,
  unresolved edges, versioned mappings, and explicit manual-work and security
  findings. Later add opt-in, separate-project scaffolding with provenance;
  never copy secrets or production data or claim automatic business parity.
- [🗺️] Grow Studio from its existing safe CRUD/schema tools into reviewed
  relational CRUD design, foreign-key selection, and typed chart configuration
  through the properties sidebar. Then add compiler-checked diagrams and
  presentation exports. Keep generated queries typed and read-only, enforce
  permissions and tenant boundaries, inspect the complete diff, and require
  explicit apply. Verify each database backend only for the operations it
  supports; arbitrary client hydration remains out of scope.

## Release 0.9.5 — first-party mail and SMTP

- [x] Expanded the existing Laravel analysis into a versioned compatibility
  ruleset for authentication, policies/permissions, business logic,
  transactions, money fields, jobs, integrations, file handling, and views.
  Mark security-sensitive or behavior-dependent findings for human review,
  including status changes, totals, side effects, and failure handling. Keep
  static analysis independent of PHP packages and a running Laravel environment.
- [x] Added the first explicit application mail API without a mandatory
  provider: MIME text/HTML, SMTP authentication, implicit TLS, STARTTLS, private
  certificate authorities, bounded messages, and integration with the existing
  MariaDB outbox. Dispatch remains explicit; TLS never silently downgrades.
  Document email capabilities and the local test sink.
- [ ] Add safe named templates and bounded file attachments in 0.9.7 alongside
  recipient-aware notification delivery; use shared size, escaping, and source
  checks.
- Exit evidence passed on the exact candidate: local SMTP end-to-end test with a trusted local CA, STARTTLS,
  and authentication; plus tests for rejected configuration, size limits,
  header injection, secret redaction, capability checks, and transactional
  enqueueing. Document delivery and its duplicate window in both languages.
  See the [0.9.5 plan](release-plans/0.9.5.en.md).

## Release 0.9.6 — notification center

- [🗺️] Add opt-in scaffold generation with
  `zelyra migrate laravel <project> --output <target>`; allow `--report` and
  `--output` together. Generate a separate Zelyra project only when requested,
  preserve source attribution and mapping-rule identifiers in generated files,
  and never copy secrets, `.env` values, personal records, or production data.
  Refuse overlapping source/report/output paths and unsafe existing targets.
  The report must distinguish generated scaffolding from transferred business
  behavior, and every generated project must pass the matching Zelyra compiler.
  Treat each blocker in an accepted mapping as a Zelyra implementation item in
  the same release train, and close it before calling that mapping supported.
- [🗺️] Add a tenant-scoped shared in-app notification feed with unread counts,
  pagination, expiry filtering, soft-delete archive, and editable preference
  records. This first slice does not claim per-user read receipts, scheduled
  retention deletion, or event-triggered email; those require recipient-aware
  delivery and are planned for 0.9.8. Keep this slice useful with SMTP disabled.
- Exit evidence: MariaDB integration proves a member cannot list or count a
  different tenant's notifications, archived and expired items are excluded as
  configured, and guessed cross-tenant IDs stay hidden. Localization, generated
  forms, and failure states have coverage. No external push service is required.

## Release 0.9.7 — Studio, invoice tutorial, and hardening

- [✅] Split the invoice tutorial between `main.zyl` and feature modules, with
  a desktop dashboard backed by real metrics and a guided customer/item/invoice
  flow. The Tabler CSS component is optional, local, and MIT-licensed; no CDN
  dependency. Keep the learning path's net, 19% VAT, and gross print output.
- [✅] Fix project-map gaps: resolve views across modules only within the local
  module or through imported public views, include CRUD layouts, and show table
  views. Explain limits and unresolved relationships with localized stable
  reasons; do not guess SQL relationships.
- [✅] Add responsive browser E2E for desktop and narrow viewports, real
  metrics, generated CRUD forms, offline local Tabler delivery, and printable
  invoices.
- [🗺️] Deliver personal read receipts and recipient authorization, bounded
  retention, safe templates/attachments, and optional SMTP in 0.9.8. Verify
  atomic enqueueing, idempotent retries, cross-tenant/recipient IDs, and
  duplicate delivery behavior.
- [🗺️] Harden Laravel analysis for tenant boundaries, authorization,
  downloads/uploads, payments/accounting, and public endpoints by 0.9.8.
  Missing or ambiguous evidence requires manual review.
- [🗺️] Close the promised MariaDB, MySQL, and PostgreSQL workflow matrix for
  1.0; document exact server versions, supported workflows, and limitations.
  Verify relations, TLS, backup/restore, and tenant-filtered CRUD with negative
  authorization tests.
- Exit criteria: the project-map regression resolves the imported tutorial
  layout, browser checks show real metrics, and all findings/backend limits are
  documented. Full scope: [0.9.7 plan](release-plans/0.9.7.en.md).

- [🗺️] Extend the notification center with per-user read receipts, recipient
  authorization, bounded retention, safe named templates, and optional SMTP
  delivery through the transactional outbox. Verify atomic enqueueing,
  idempotent retries, cross-tenant and cross-recipient ID attacks, and duplicate
  SMTP delivery behavior.
- [🗺️] Harden Laravel migration analysis for tenant boundaries, authorization,
  downloads/uploads, payments/accounting, and public endpoints. Trace evidence
  through routes, middleware, controllers, models, and relations where the
  supported syntax permits; flag manipulated-ID and cross-tenant risks. When
  static evidence is missing or ambiguous, require manual review and never
  silently omit the behavior. Add fixtures for tenant isolation, foreign keys,
  transactions, decimal money, permissions, and file storage.
- [🗺️] Close the promised MariaDB, MySQL, and PostgreSQL workflow matrix for
  1.0, including schema inspection, migrations, CRUD, transactions, TLS, backup,
  and restore where each backend claims support. Complete independent review
  of authentication, MFA, tenant isolation, authorization, CSRF, SMTP secrets,
  and Studio mutation boundaries. Resolve every release-blocking finding.
- [🗺️] Run the complete tenant-filtered CRUD authorization matrix against the
  exact supported MariaDB, MySQL, and PostgreSQL versions. Cover tenant
  selection, membership and permission checks, generated list/detail and
  create/update/delete/restore paths, relation ownership, and fail-closed
  behavior when context or membership is missing or revoked. Document the
  supported tenant model and migration expectations; do not imply that
  existing installations are automatically backfilled.
- Exit evidence: exact server versions and limitations are published; each
  supported workflow has CI evidence, negative authorization tests, and a
  maintainer-reviewed disposition for every security finding. Do not claim
  universal SQL parity.

## Release 0.9.8 — production-readiness rehearsal

- [✅] Delivered the language-stabilization slice: controlled deep-call
  limits, complete return paths, direct `Int::MIN`, calendar/UTC arithmetic,
  Decimal quantization, payload-free enums, exclusive integer ranges,
  value-semantic record/array updates, a tagged Result JSON codec, and
  iterative handling of long expressions. Workspace tests, Clippy, formatting,
  324 handbook examples, all 80 corpus cases, 100 repeat runs, and 300 integer
  division/remainder plus 100 Decimal reference checks passed. The 0.9.9
  rerun closes the ten earlier language-gap probes; four bounded observations
  are documented in the specification and type matrix.
- [✅] Studio's light appearance now covers the editor, project navigation,
  properties, help, database workbench, task finder, and dialogs. Browser E2E
  verifies persistence, keyboard access, consistent surfaces, and at least
  4.5:1 contrast for text and syntax tokens. At 360 CSS pixels, the work
  surface and theme control stay usable without horizontal overflow. The
  broader IDE work continues with the visual relational and chart workflows
  planned for 0.9.9.
- [✅] MySQL Server 8.4.11 now passes an isolated schema workflow through
  `db setup`, `inspect`, read-only `map`, reviewed `plan`, explicitly approved
  `apply`, and `history`. The CI job also checks parameterized SQL queries.
  This does not prove full CRUD/auth, MFA, TLS, backup/restore, or general
  MariaDB parity; the backend matrix states the limits.
- [🗺️] Carried to 0.9.9: complete the supported Laravel Migrator scope and its versioned
  compatibility matrix. Verify read-only analysis, secret/PII redaction,
  source-location accuracy, route-to-data relationship reports, ambiguity
  handling, path separation, provenance in generated files, and compiler-valid
  output on representative Laravel fixtures. Run generated applications through
  real CRUD workflows on the supported databases, including the supported
  relation, tenant, permission, transaction, and money cases. Resolve any
  Zelyra capability that blocks an accepted mapping before 0.9.9 exits. Document
  supported Laravel and package patterns plus known limits. No automatic
  business-logic or security parity claim is made; the tool supplies an
  auditable migration starting point.
- [🗺️] Carried to 0.9.9: rehearse clean installation, upgrades, rollback, migration, backup and
  restore on supported operating systems and databases. Finish operator and
  user documentation, dependency/license review, resource limits, error
  redaction, reproducible artifacts, SBOM/provenance, and offline workflows.
  Verify diagrams and presentations in clean environments, including accessible
  reading, stable exports, and secret redaction. Measure startup, common CRUD
  paths, and bounded resource use; publish results without unsupported
  performance promises.
- [🗺️] Close the mandatory 0.9.8 carry-over in 0.9.9: clean-environment and
  database rehearsals, security/dependency dispositions, integrated
  Studio/tutorial/mail/notification flows, Laravel Migrator E2E, and chart and
  presentation evidence. See the [0.9.8 plan](release-plans/0.9.8.en.md) and
  [0.9.9 plan](release-plans/0.9.9.en.md).

## Release 0.9.9 — feature freeze and final technical candidate

- [✅] Typed Decimal tableview presentation retains exact digits without added
  fractional padding; ordinary text and raw data keep their representation.

- [✅] Correct setup dependency after precision hardening: MariaDB first,
  schema in a separate container run, web service only after success;
  migration approvals remain enforced.

- [✅] Decimal package: explicit SQL scale, lossless JSON transport, exact
  forms/filters and PostgreSQL NUMERIC decoding; aliases, numeric boundaries,
  intermediate overflow and controlled resource errors are tested.
  See [rules](decimals.en.md) and [package report](release-readiness/0.9.9-exact-decimals.en.md).
- [✅] Calendar/UTC arithmetic, typed form/HTTP input and native SQL
  Date/Time/Timestamp parameters/storage pass on MariaDB, MySQL and PostgreSQL;
  UTC sessions and exact time precision are enforced. See [rules](dates.en.md)
  and [SQL evidence](release-readiness/0.9.9-calendar-sql.en.md).
  The SMTP/reset fixture accounts for guarded suppression of superseded
  delivery jobs and verifies the retained replacement token before continuing.
  Generated-business readiness checks honor mandatory metadata access before
  startup, including controlled, redacted failure when the database is absent.
- [✅] Explicit `sql_decode<T>(sql<…> { … })` materializes scalar/record/array
  values with checked cardinality, projection, native type families and NULL
  flags. Exact numbers and downstream calendar operations pass on three real
  backends; legacy raw-row HTTP/JSON is retained. See [rules](sql-results.en.md)
  and [evidence](release-readiness/0.9.9-typed-sql-results.en.md).

- [✅] Edit snapshots preserve native NULL flags, reject stale NULL/literal-text
  changes before writing, and render NULL separately from literal `\N`.
  Omitted implicitly nullable fields bind SQL NULL; required/readonly rules stay.
  MariaDB/MySQL HTTP and malformed-metadata regressions pass. See
  [rules](edit-snapshots.en.md) and [evidence](release-readiness/0.9.9-null-edit-snapshots.en.md).

- [✅] Native decision transaction foundation covers PostgreSQL/MariaDB/MySQL
  on one connection, rollback/unwind, swallowed SQL errors and pre-commit/request
  deadlines. [Rules](decision-transactions.en.md), [evidence](release-readiness/0.9.9-decision-transactions.en.md).

- [✅] PostgreSQL form decisions execute tenant membership, resource recipient
  checks, locked row snapshots (NULL vs. `\N`), business mutations, and atomic
  unchained audit writes within the same native transaction and connection.
  Tampered tenant/foreign keys, revoked membership during lock wait (two-connection
  race), and audit errors roll back atomically. Chained PostgreSQL audit remains
  diagnostically rejected. [Evidence](release-readiness/0.9.9-postgres-form-decisions.en.md).

- [✅] Database matrix, transactions, and relational CRUD on MariaDB 11.4.13,
  MySQL 8.4.11, and PostgreSQL 16.15: setup/introspection of real foreign keys,
  migration preflight orphan rejection (`error[E-DB-005]`) with data preservation,
  hardened MySQL write locks (`innodb_lock_wait_timeout`), tenant-isolated
  selection lists, tamper protection (422), relational join display, and
  restrictive deletion with ordered child-then-parent deletion.
  See [evidence](release-readiness/0.9.9-relational-crud-matrix.en.md).

- [🗺️] Freeze language, CLI, runtime, databases, IDE, mail, notification, and
  tutorial scope. Complete every accepted 0.9.1–0.9.8 requirement and run the
  complete technical gates on an immutable candidate. Prepare exact 1.0.0
  human-study materials. At this point independent human acceptance should be
  the only remaining gate; do not claim production approval or run acceptance
  against 0.9.9.
- [✅] Language review against the 0.9.9 source build: 80 cases with no
  unexpected outcomes, 20 tasks, 35 controlled negative cases, 100 repeats,
  and 400 independent numeric comparisons passed. The four observations and
  Linux-only scope are recorded in the
  [review report](release-readiness/0.9.9-language-review.en.md). The corpus
  was rerun after the Clippy compatibility fix in `1bf09ea`, using compiler
  SHA `89b880d753e12ac09b2fbb4dc754586d18c7a808845b991b331cb1ae00153c0d`.
- [🧪] Rust package tests, formatting, and Clippy plus installer, clean MariaDB
  onboarding, and generated Docker/module bundle E2E checks pass locally on
  `1bf09ea`. CI and packaging passed on documentation descendant `8db093c`;
  later source packages and the final candidate need their own CI. See the
  [technical rehearsal](release-readiness/0.9.9-technical-rehearsal-2026-10-12.en.md).
- [🗺️] Close the remaining CLI, setup, database, migrator, security,
  documentation, and packaging gates. General constraints/traits, closures,
  and generic function references remain explicit language boundaries. The
  [0.9.9 plan](release-plans/0.9.9.en.md) lists the testable criteria.
- [✅] Add and verify an optional colored CLI help mode with `ZELYRA_COLOR`,
  `NO_COLOR`, and `CLICOLOR_FORCE`, plus a brief Studio startup splash that
  respects the persisted appearance and reduced-motion preference.
- [✅] Generic functions, records, and enums are implemented on the 0.9.9
  branch. Type arguments are inferred or explicit, including through nested
  records, arrays, `Option`, and `Result`. Conflicting, missing, and wrong-arity
  types have regression coverage. The type-system example includes module
  imports and positive/negative cases. Persistent SQL columns explicitly reject
  generic records. Payload enums and named typed function values/indirect calls
  also pass focused positive and negative tests, including capability
  propagation. General constraints/traits, generic aliases, closures, and
  generic function references remain outside this supported scope.
- [🧪] Studio charts now support fixed aggregates, exact text filters,
  inclusive `DATE` ranges, safe parameter binding, saving and reloading chart
  configurations, and local presentation/PDF previews. MariaDB and PostgreSQL
  browser/database E2E checks pass. Saved SQL views, application-page embedding,
  compiler validation of report files, and full accessibility/export evidence
  remain open release criteria.
- [✅] Persistent MariaDB reset limits, bounded cleanup/capacity, safe operator
  errors, and reset-storage module dependencies pass positive and negative
  checks. Generated SQL names respect backend limits throughout creation and
  planning; long declarations fail before SQL execution. Rust, MariaDB SMTP/
  quota, and PostgreSQL schema checks are recorded in the
  [package report](release-readiness/0.9.9-reset-and-schema.en.md).
  Independent security review remains open.
- [🗺️] Add visual form and output editors with a property sidebar for fields,
  validation, database/table binding, column selection, filters, sorting,
  search, pagination, and charts. Generate editable Zelyra source and local
  styles with compiler-backed previews and protection for manual edits.
- [✅] A bounded Zelyra-style class model now provides typed constructors,
  shared reference identity, identity equality, field reads/updates, generic
  classes, and optional fields. Runtime checks reject reference cycles and
  parallel mutation; JSON/API/SQL boundaries reject class references. Behavior
  uses ordinary functions with explicit receiver parameters. Methods, closures,
  and inheritance syntax remain outside the 0.9.9 scope.
- Exit criteria: no open implementation, security, database, migration,
  packaging, or documentation blocker remains. If one remains, 0.9.9 is not
  complete. See the [0.9.9 plan](release-plans/0.9.9.en.md).

A general queue platform remains beyond 1.0.0. The 0.8 outbox is a bounded,
local MariaDB feature and does not imply a scheduler, distributed workers, or
exactly-once delivery. Complete SQL parity, arbitrary client hydration,
compiler self-hosting, native backend, mandatory cloud/AI services, telemetry,
and unsupported performance claims remain excluded.
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

## Future language, database, and ergonomics evolution (post-1.0.0)

- [x] **1.1.0 Studio and standalone LSP (released):** See the verified
  implementation summary in **Current milestones**, the bilingual
  [release notes](release-notes/1.1.0.en.md), and the detailed
  [work log](release-plans/1.1.0-studio-deep-native-progress.en.md). The
  work log records tested limits for semantic rename, preview isolation,
  analysis cancellation, bundle size, browser behavior, and release artifacts.

To boost developer ergonomics, flexibility, and architectural modularity without compromising
Zelyra's core tenets (static security boundaries, deterministic business arithmetic, digital
sovereignty), the following extensions are explicitly scheduled for post-1.0 development:

- [ ] **Modular database adapter architecture (`DatabaseAdapter` SPI / traits):**
  Replacing hardcoded backend branching in core crates with an extensible adapter interface.
  Each driver adapter encapsulates introspection, schema migration (`db apply`), data type mapping,
  transaction lifecycle, lock management (`FOR UPDATE`, query deadline and timeout steering),
  and dialect-specific SQL emission. This enables integrating additional relational and specialized
  databases (e.g. Microsoft SQL Server, Oracle, CockroachDB) via modular adapters without modifying
  the compiler core or web framework.
- [ ] **Multi-database integration (multiple databases / polyglot persistence):**
  Support for declaring and managing multiple independent database connections within a single
  Zelyra application (e.g. primary relational OLTP database for tenant CRUD transactions, dedicated
  read replicas, or separate databases for analytics, reporting, cold storage, or third-party legacy
  data). Declarative binding of schemas, modules, and queries to named database adapters
  (`database "secondary" { ... }`) with full static tenant isolation and capability enforcement.
- [ ] **Unified native Query AST:** Replacing residual string-interpolated query paths with
  an internal query syntax tree targeting MariaDB, MySQL, PostgreSQL, and future adapters in
  a dialect-neutral, type-checked manner.
- [ ] **Non-capturing lambdas (`|x| expr`):** Syntactic support for compact, anonymous
  transformation functions without outer environment capture. Incurs zero heap allocation or GC
  overhead and preserves the deterministic memory model.
- [ ] **Headless background tasks (`task Name { ... }`):** Dedicated syntax and runtime
  support for periodic jobs, batch pipelines, and background processing while enforcing the
  same static tenant and database capability boundaries without HTTP routing constraints.
- [ ] **Actionable compiler diagnostics ("Did you mean?"):** Enriching type and schema
  mismatch diagnostics (`E-SQL`, `E-CRUD`) with exact, source-anchored fix suggestions to
  streamline developer experience while maintaining strict type enforcement.

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
