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

The current stable release is 1.0.6. **Current milestones** is the source of
near-term commitments. Older numbered release sections below are retained as
historical planning and evidence; they do not by themselves define current
release targets. Longer-term ideas remain in the post-1.0 evolution section.

## Current milestones

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

- [ ] **Planned for 1.1.0 — Compose check in the installer:** Installers detect
  Docker Compose v2 or newer and distinguish it from obsolete Python Compose
  v1. Docker remains optional for the core installation. When Compose v2 is
  missing, interactive Linux installers offer to install the Compose v2 plugin
  from supported package sources after explicit confirmation; otherwise they
  show suitable installation guidance. No elevated changes happen without
  confirmation or in non-interactive runs. The guidance explains that
  Docker-based tutorials cannot start until Compose is installed.
- [ ] **Planned for 1.1.0 — interactive console menus:** A simple declarative
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
- [~] **Partially implemented in 1.0.3 — Zelyra-style language constructs:**
  Chained `else if` branches, typed `|>` pipelines, and type-checked tuple
  destructuring in `for` loops are implemented. Remaining work includes typed
  `if` expressions, labeled `break`/`continue` for nested loops, and explicit,
  type-checked rules for value-yielding loop exits. Keep brace-based blocks,
  static types, explicit effects, and checked errors. The parser suggests the
  expected block form and exact source span when an `else if` block is missing;
  add suggestions for other common mistakes. Stepped ranges, including negative
  steps and descending loops, are already in 1.0.2. User-defined operators
  still need an explicit, type-checked protocol with fixed precedence,
  unambiguous resolution, and effect rules; custom precedence declarations are
  not allowed.
- [~] **Partially implemented in 1.0.3 — native String and Math library:**
  Built-in String functions cover lowercase/uppercase, Unicode length, joining,
  trimming, containment, replacement, splitting, padding, and prefix/suffix
  checks; existing `text_*` names and Result variants remain available. `|>`
  composes direct function names and calls from left to right, type-checking
  each stage like an ordinary call. Native math functions now have specified
  types, rounding rules, and runtime errors for overflow or invalid domains.
  `concat<T>` is implemented. Readable, type-checked String interpolation with
  escaped literal braces remains open.
- [~] **Partially implemented in 1.0.3 — collections and functional operations:** Keep
  ordinary arrays and maps value-based. Provide opt-in typed mutable arrays and
  stacks with checked O(1) indexed updates and efficient `push`/`pop`; `pop`
  returns an explicit empty case. Mutable arrays must allow checked indexed
  compound updates such as `items[i] += 1`. Typed map iteration through
  `entries`, tuple destructuring, `get_or_default`, and `map_increment` with a
  checked overflow case are implemented. Maps, `Map<K, V>` and `Map { ... }` literals,
  `get`, `put`, `keys`, `values`, and `contains` already exist. Add typed
  anonymous functions with explicit capture/effect rules and
  standard `map`, `filter`, and `fold` combinators. Support composition through
  the Zelyra pipeline while preserving clear inference and diagnostics. The
  knapsack DP table must update in place and retain O(item-count × capacity)
  time and O(capacity) space rather than copying the full array per cell.
- [ ] **Planned for 1.0.3 — interactive Studio console:** Give Studio Run a
  real interactive input panel for `read_console`: display the pending prompt,
  accept and submit a line, resume the running program, and support explicit
  EOF/cancel and bounded execution. Do not silently turn an input request into
  EOF or a timeout. Keep CLI console input working, and test the same
  number-guessing program in both CLI and Studio.
- [ ] **Planned for 1.0.3 — idiomatic algorithm exercises:** Ship 17 complete,
  runnable console examples: FizzBuzz, palindrome checks, factorial, number
  guessing, word/character counts, list minimum/maximum, prime checks,
  reversed word order, Caesar shifts, bubble sort, quicksort, binary search,
  balanced-parentheses checking with a stack, iterative dynamic-programming
  Fibonacci, run-length encoding, N-Queens counting with recursive
  backtracking, and 0/1 knapsack with dynamic programming. Rewrite them to use
  the new Zelyra syntax and APIs where they improve clarity, rather than
  retaining Python/PHP workarounds such as JSON encoding for display, manual
  stack rebuilding, or repeated array-copy updates. Explain important lines,
  expected results, boundary cases, common mistakes, and time/space costs.
  N=4 and N=8 must count as 2 and 92; the documented knapsack fixture must
  return 8. Validate sizes and inputs, use checked arithmetic, bound work, and
  test compilation, expected output, and edge cases in CI.
- [~] **Partially implemented in 1.0.3 — compiler diagnostics:** The parser
  suggests a corrected block form and source position for a missing `else if`
  block. Remaining work includes nested syntax and
  type errors with targeted suggestions, source spans, and a short corrected
  Zelyra example. Pipeline type mismatches, collection callbacks, and invalid
  numeric inputs need targeted positive and negative tests.
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

- [🧪] **1.1.0 Studio/LSP foundation (unreleased):** Compiler-backed in-memory
  source overlays now feed Studio checks and a stdio `zelyra lsp` prototype.
  Studio analyzes up to 16 open unsaved Zelyra files together for diagnostics,
  navigation, verification, the project map, and static previews.
  The prototype implements versioned diagnostics, document/workspace symbols,
  and declaration completion. Resolved function definition and references now
  work in Studio and LSP; LSP also previews compiler-checked function renames.
  Studio now previews and applies resolved function renames across open
  buffers using the same compiler-checked plan. Durable journaled multi-file
  writes and import-aware file renames remain incomplete.
  A versioned gutter now displays real verifier outcomes and clears on invalid
  source. A compiler-backed typed-hole menu in Studio and LSP filters known
  exact types and declared Capabilities, with precise replacement spans. An
  authenticated local SSE stream reports versioned diagnostics, verifier, and
  workspace events. Studio can apply a real CLI step budget and cancel console
  runs; a call graph marks possible recursion and reads the configured stack
  depth. The project map now links tables, SQL, records, forms, table views,
  and APIs across modules and shows static compiler NULL diagnostics. Studio
  now explicitly renders declared views with example data through the
  same server-side PDF renderer as `render_pdf`, without executing project
  functions; download and an isolated PDF frame passed three browser engines.
  The MariaDB outbox view reads paginated metadata and releases exhausted
  events through the existing requeue operation without immediate delivery.
  Other symbol kinds, recoverable refactoring, full release gates, and
  artifacts remain
  open; see the [1.1.0 work log](release-plans/1.1.0-studio-deep-native-progress.en.md).

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
