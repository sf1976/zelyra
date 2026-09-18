# Zelyra Roadmap

This is the maintained roadmap for Zelyra. It records both required work and
optional ideas. An item is not considered complete until it has syntax or API
documentation, implementation, positive and negative tests, diagnostics, and a
working example where applicable.

Status: `[x]` implemented, `[~]` in progress, `[ ]` planned, `[?]` optional or
under evaluation.

## Current milestones

- [x] Language core: lexer, parser, AST, functions, expressions, control flow,
  immutable-by-default bindings, arrays, records, Option, Result, and pattern
  matching.
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
  Multiple slots, themes, and CRUD view overrides remain open.
- [x] Authentication, persistent sessions, CSRF, Argon2 passwords, direct and
  role-based permissions, browser administration, and MariaDB audit logging.
- [~] Audit operations: inspect, JSON/CSV export, structural verification, and
  confirmed pruning are available. Tamper-evident chaining and archival remain.

## 1. Beginner experience and distribution

- [ ] One-command installation for Linux, Windows, and macOS.
- [ ] Signed release binaries and checksums for every supported platform.
- [ ] Install/update/uninstall scripts that do not require Rust or Cargo.
- [ ] First-run wizard for project creation, MariaDB connection, secrets, and
  selectable web port.
- [ ] Clear Docker Compose templates for MariaDB and the internal web server,
  including configurable host and container ports.
- [ ] Optional automatic reverse-proxy setup for Apache and Nginx, with safe
  defaults and generated configuration previews.
- [ ] `zelyra doctor` checks for database, port, TLS, permissions, and required
  external tools with actionable bilingual messages.
- [ ] Project templates: minimal script, MariaDB CRUD, API, authentication,
  and production deployment.
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
- [ ] Explicit resource/effect model for Database, Network, FileSystem,
  Environment, Process, Clock, and Random.
- [ ] Structured error propagation and user-defined error types.
- [ ] Deterministic build graph, incremental compilation, caching, and parallel
  compilation.
- [ ] Language server, editor extensions, formatter, linter, and debugger.
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
- [ ] Complete PostgreSQL runtime parity.
- [ ] MariaDB/MySQL compatibility matrix and version-specific diagnostics.
- [ ] SQL Server backend evaluation and implementation if demand justifies it.
- [ ] Reversible migration plans, rollback guidance, backups, and drift reports.
- [ ] Better destructive-change analysis, row estimates, lock warnings, and
  maintenance-window planning.
- [ ] Connection pooling, retry policies, timeouts, cancellation, and health
  checks.
- [ ] Streaming large results and bounded memory behavior.
- [ ] N+1 query detection, query-plan hints, slow-query diagnostics, and query
  observability.
- [ ] Typed relations, joins, aggregates, subqueries, CTEs, unions, and
  database-specific extensions.
- [ ] Read replicas, read/write routing, tenant isolation, and migration
  environments.
- [?] Database seed, fixture, snapshot, and anonymized test-data commands.

## 4. Views and web presentation

- [~] Named views/layouts with a page-level `view: Name` assignment and a
  validated `<slot />` content insertion point.
- [ ] Typed view expressions with compile-time checking of variables, fields,
  optional values, and output escaping.
- [~] Named components with typed properties are available; typed events remain
  planned.
- [~] Declarative MariaDB-backed `tableview` routes with checked SQL sources,
  declared columns, typed filters, search, sorting, pagination, URL state, and
  escaping are available for table- and struct-backed result types.
- [~] Components support default and named slots plus nested composition;
  fallback content and nested views remain planned.
- [ ] View inheritance/composition without hidden global state.
- [ ] View-local data loading with explicit query boundaries and authorization.
- [~] Typed CRUD filter operators (`eq`, text matching, numeric comparisons,
  and null checks) are compiled to safe server-side SQL.
- [~] Generated CRUD filter controls preserve operator and value state in URLs;
  stable ordering and accessibility improvements remain open.
- [~] The first unified typed view pipeline slice is available on `tableview`
  routes through declarative filters, search, sorting, and pagination; the
  same pipeline for arbitrary views remains planned.
- [ ] Composable filter expressions with typed operators for dates, booleans,
  relations, and full-text search.
- [ ] Reusable navigation, tables, forms, dialogs, alerts, pagination, and
  validation-error components.
- [~] CRUD list view overrides support a safe `table`/`cards` mode and a custom
  empty-state message while preserving generated query, auth, and action guards.
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
- [ ] A design-token system for colors, spacing, typography, breakpoints, and
  density.
- [ ] Per-view themes, application themes, dark mode, and user-selectable
  appearance.
- [ ] Scoped CSS, asset pipelines, cache-busting, static files, and CSP-aware
  inline assets.
- [ ] Responsive layouts, keyboard navigation, semantic HTML, ARIA guidance,
  and automated accessibility checks.
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
- [ ] Server-rendered confirmation views and richer action-specific
  success/error presentation.
- [ ] Soft delete, restore, archive, and retention policies.
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
- [ ] Cryptographically chained audit entries using a documented hash format,
  explicit canonical serialization, and a concurrency-safe append strategy.
- [ ] `audit verify` support for detecting broken hash chains and reporting the
  first invalid entry with a source-independent diagnostic.
- [ ] Immutable or append-only database privileges for audit tables.
- [ ] Configurable retention policies, scheduled pruning, and archive export.
- [ ] Encrypted archives, key rotation, restore verification, and offline
  integrity checks.
- [ ] External audit sinks (syslog, OpenTelemetry, object storage, SIEM) with
  delivery status and retry behavior.
- [ ] MFA/WebAuthn, password reset flows, session/device management, and login
  notifications.
- [ ] Fine-grained policy expressions, policy testing, and permission explain
  output.
- [ ] Security review, threat model, dependency audit, and penetration testing.

## 7. APIs and integration

- [x] Typed API routes, request validation, JSON serialization, OpenAPI, and
  TypeScript client generation.
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
- [ ] Clear separation of `PROVEN`, `RUNTIME_CHECK`, `UNPROVEN`, and `FAILED`
  in CLI, IDE, and documentation.
- [ ] Structured concurrency cancellation, timeouts, supervision, and database
  pool integration.
- [ ] Shared-state rules, channels, actors, and data-race testing.
- [ ] Benchmark suite for compile time, startup, routing, SQL, forms, CRUD, and
  memory use.
- [ ] Profiling and observability hooks without changing application semantics.
- [?] CP-SAT, MILP, and SMT optimization interface with reproducible solver
  inputs and bounded execution.

## 9. Quality, operations, and governance

- [ ] Full integration matrix for supported OS, database, browser, and runtime
  versions.
- [ ] Fuzzing for lexer, parser, SQL binder, template renderer, and HTTP parser.
- [ ] Security regression suite and dependency/license scanning in CI.
- [ ] Reproducible release builds, SBOMs, provenance attestations, and signed
  artifacts.
- [ ] Bilingual documentation kept in sync, including migration and upgrade
  guides.
- [ ] Contributor guide, architecture decision records, code of conduct, and
  transparent issue labels.
- [ ] Semantic versioning policy, deprecation window, and compatibility tests.
- [ ] Public alpha, beta, and stable release criteria based on real business
  applications, not only language demonstrations.

## Real-world acceptance applications

- [ ] Machine management with departments, machines, CRUD, search, filters,
  permissions, audit, and MariaDB deployment.
- [ ] Customer/order application with complex joins, aggregates, forms, API,
  and custom views.
- [ ] Multi-user inventory application with transactions and concurrent edits.
- [ ] A production deployment using Docker Compose and a configurable web port.
- [ ] A self-hosted deployment without Rust or Cargo installed on the target.

This file must be updated whenever a milestone changes status or a design
decision creates a new required or optional work item.
