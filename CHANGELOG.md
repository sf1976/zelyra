# Changelog

All notable changes to the Zelyra compiler and runtime are documented here.
The language line remains `0.1`; the patch version identifies the compatible
compiler and repository release.

## Unreleased

### Added

- Made function, type, and record renames AST-aware: declarations and known
  references are changed while shadowing local bindings remain untouched.
- Made multi-operation edits position-stable by sorting replacements before
  applying them from the end of the source.
- Extended AST-aware resource renames to tables, named views, forms, CRUDs,
  and structured table references; table renames update checked SQL table
  positions without changing literals, comments, parameters, or HTML.
- Component renames now update declarations and known opening/closing component
  tags in HTML bodies without treating ordinary HTML markup as symbol references.
- Extended source-only impact JSON with a deterministic `references` edge list
  for known table, view, component, handler, SQL, and function-call relations.
- Hardened semantic edit requests with required schema version `1`, project-
  local `.zyl` entry checks, full compiler validation before and after a
  proposed rename, and explicit diagnostics for rejected boundaries.
- Added deterministic `zelyra edit --format=json` requests for validated symbol
  renames of functions, types, records, tables, tableviews, forms, CRUDs, views,
  and components. Proposed changes are reparsed atomically; writes require the
  explicit `--apply` flag, use an atomic replacement, and require a matching
  source fingerprint to prevent stale overwrites.
- Added deterministic, source-only `zelyra impact --format=json` output for
  tables, SQL, forms, CRUD resources, views, APIs, permissions, and contracts.
  Email, job, test, and live schema impact are explicit empty or unavailable
  categories until their source and runtime integrations exist.
- Added contextual typed-hole diagnostics for `_`. Buildable commands report
  the expected type, visible values and functions, active capabilities,
  contract obligations, and source location; incomplete code is rejected
  before lowering or execution.
- Added deterministic `zelyra fmt` source formatting with a non-writing
  `--check` mode for CI. Formatting preserves comments and opaque SQL/HTML
  bodies and is covered by idempotence and example-source checks.
- Added the AI-native development foundation: stable versioned JSON
  diagnostics, deterministic source spans, and a read-only structured project
  context interface for vendor-neutral tooling.
- Added bilingual AI-native architecture, specification, roadmap, handbook,
  and reproducible benchmark documentation. No benchmark results are claimed
  until they are produced by actual repeatable experiments.
- Added named web views with page-level `view: Name` composition and a
  compiler-validated `<slot />` content boundary.
- Added typed self-closing view components with declared properties and safe
  literal-property escaping.
- Added declarative MariaDB-backed `tableview` routes with checked SQL sources,
  allowlisted columns, search, sorting, pagination, URL state, and escaping.
- Added struct-backed tableview results for typed join and aggregate views.
- Added projection alias, field-presence, primitive-type, and NULL-safety
  checks for struct-backed tableview results.
- Added schema-aware CRUD filter operators for text matching, numeric
  comparisons, and null checks, with operator controls and URL state.
- Added safe CRUD list view overrides with table/cards presentation modes
  and configurable empty-state messages. Generated query, authorization, and
  escaping safeguards remain active in both modes.
- Added safe CRUD detail view overrides with standard/cards presentation modes
  and configurable headings while retaining generated action, CSRF,
  authorization, and escaping safeguards.
- Added safe CRUD form view overrides with standard/cards layouts, configurable
  headings, and submit labels while retaining validation, CSRF, parameter, and
  permission safeguards.
- Added safe CRUD delete confirmation overrides with configurable headings,
  warning messages, and submit labels while retaining POST-only, CSRF, and
  delete-authorization safeguards.
- Added reversible CRUD soft deletion with schema-validated timestamp columns,
  archived list/detail views, and CSRF-protected restore actions. Soft-delete
  markers are excluded from generated forms and default list/filter columns.
- Added audit-aware CRUD mutation events for generated forms, delete/archive,
  restore, and custom actions. Events include actor and record context plus
  field-level changes with sensitive-value redaction and share the mutation
  transaction.
- Added CRUD loading metadata and safe configurable error views. Error messages
  remain escaped and generic database details are not exposed; loading metadata
  is suitable for progressive enhancement.
- Added declarative custom CRUD actions with parameterized POST-only SQL,
  detail-view buttons, route-ID binding, CSRF, database-capability,
  authentication, and permission checks. Actions may now define escaped
  labels, browser confirmation messages, typed validated input fields, and
  MariaDB-backed relationship select fields. Action icons and escaped success
  notices are now available as well. Riskier actions can use server-rendered
  `confirm_page` views with CSRF-protected POST confirmation. Structured
  `success_page` notices and safe `error_page` responses are also available.
- Added database-backed role permission groups through `roles` and
  `role_permissions` authentication options. Direct user permissions and
  inherited role permissions are combined for protected requests.
- Added MariaDB CLI administration for idempotent role assignments and role
  permissions with `zelyra auth role` and `zelyra auth role-permission`.
- Added opt-in browser role administration with CSRF-protected grant/revoke
  forms and protection for the last assignment of the configured admin role.
- Added opt-in browser user administration for creating users, resetting
  passwords, and activating/deactivating accounts with persistent-session
  revocation and last-active-administrator protection.
- Administrative password resets now revoke all persistent sessions for the
  affected user atomically with the password change.
- Added optional append-only MariaDB audit logging for browser user and role
  administration, including the latest 100 events in the admin screen.
- Login, failed-login, and logout events are recorded when audit logging is
  enabled; logout now requires a valid CSRF token.
- CLI role and role-permission mutations now append transactional audit events
  when audit logging is enabled. Added `zelyra audit inspect` plus JSON/CSV
  `zelyra audit export` with bounded limits.
- Added structural audit verification and explicitly confirmed, transactional
  audit pruning through `zelyra audit verify` and `zelyra audit prune`.
- Added opt-in cryptographic audit chaining with canonical SHA-256 entry hashes,
  transaction-locked appends, chained verification, schema diagnostics, and a
  safe refusal to prune chained logs. Added the `examples/audit_chain.zyl`
  MariaDB example and end-to-end test.
- Added default and named slots for reusable components, including nested
  component composition and compile-time rejection of unconsumed child content.
- Added typed `tableview` filters with safe equality, text, numeric, boolean,
  and NULL operators, generated controls, URL state, and validation.
- Added a MariaDB tableview end-to-end test covering struct-backed joins,
  aggregates, escaping, search, sorting, pagination, and rejected sort fields.

## 0.1.38-alpha.1 — 2026-09-18

This release establishes the AI-native compiler foundation. It adds stable,
versioned JSON diagnostics, deterministic source spans, a read-only structured
project context command, bilingual architecture and handbook documentation,
and a benchmark specification without invented results.

Diese Version schafft die KI-native Compilergrundlage. Sie ergänzt stabile,
versionierte JSON-Diagnosen, deterministische Source-Spans, einen
schreibgeschützten strukturierten Projektkontext, zweisprachige Architektur-
und Handbuchdokumentation sowie eine Benchmark-Spezifikation ohne erfundene
Ergebnisse.

## 0.1.37-alpha.8 — 2026-09-17

This release contains the changes listed under `Unreleased` above.

Diese Version enthält die oben unter `Unreleased` aufgeführten Änderungen.

## 0.1.37 — 2026-09-17

### Added

- Added tag-based GitHub release automation for Linux and Windows CLI
  archives.
- Added SHA-256 checksums to every generated release archive.
- Alpha tags now create GitHub pre-releases automatically with bilingual
  release notes.
- Extended the MariaDB CRUD end-to-end test to cover search, relationship and
  boolean filters, allowlisted sorting, pagination, invalid query fields, and
  complete cleanup.
- Added a MariaDB authentication end-to-end test covering login, persistent
  sessions, permission denial, and logout.
- Added a protected CRUD/API example and MariaDB integration test covering
  session-based authorization at both HTML and JSON endpoints.
- Added scoped CRUD permissions for `create`, `edit`, and
  `delete`; generated forms and delete routes now enforce their
  action-specific permission.
- Generated CRUD lists and detail views now hide action links and buttons that
  the current session is not permitted to use.
- Form actions can now declare their own authentication and permission
  requirements; checks run on both render and submit.

### Fixed

- Configured relationship filters now preserve their logical query names,
  such as `filter_department`, while still using the stored foreign-key column
  safely.
- Generated CRUD Create/Edit forms no longer bypass authentication and
  authorization checks.

### Compatibility

- Source installation remains available and unchanged.
- Release archives contain the CLI binary, both README files, and license
  notices; MariaDB remains the recommended application backend.

## 0.1.36 — 2026-09-17

### Added

- Added a reproducible MariaDB CRUD end-to-end test at
  `tests/mariadb-e2e.sh`.
- Added GitHub Actions CI with a MariaDB 11 service, workspace tests,
  formatting, Clippy, and the real CRUD integration test.
- Extended `examples/machine_form.zyl` with a Department CRUD resource and
  linked machine IDs for complete browser-level CRUD coverage.

### Compatibility

- The integration test requires only `DATABASE_URL`, `curl`, and a built
  `zelyra` binary; it contains no credentials.
- The GitHub workflow uses an isolated CI database and does not access
  production systems.

## 0.1.35 — 2026-09-17

### Added

- CRUD lists and detail views now render foreign-key relationships with a
  readable display value instead of the raw numeric ID.
- Relationship-aware sort and filter labels no longer expose the storage
  suffix `_id` in the generated interface.
- MariaDB CRUD regression coverage now verifies relationship rendering.

### Compatibility

- Foreign-key filtering continues to use the stored ID and remains safely
  parameterized.
- Existing CRUD configurations and form bindings remain compatible.

## 0.1.34 — 2026-09-17

### Added

- The HTTP server now reads complete requests across multiple network reads.
- Oversized bodies are rejected from their declared `Content-Length` before
  the body is read.
- Added a 64 KiB HTTP-header limit and regression coverage for incomplete and
  fragmented requests.

### Compatibility

- The public `parse_request` API remains unchanged.
- Requests without a body continue to work as before.

## 0.1.33 — 2026-09-17

### Added

- Added API media-type validation for JSON and URL-encoded request bodies.
- Added `Content-Length` validation and a one-mebibyte request-body limit.
- Added secure default response headers for content sniffing, framing, and
  referrer disclosure.
- Added Phase 12 documentation and regression tests.

### Compatibility

- Existing JSON and URL-encoded API clients remain supported.
- Unsupported body media types now receive a structured 415 response.

## 0.1.32 — 2026-09-17

### Added

- Added explicit, exact-origin CORS configuration through the `[web]` section
  in `zelyra.toml`.
- Added automatic API preflight handling with `OPTIONS`, allowed methods and
  headers, credential support, and bounded preflight caching.
- Added CORS integration tests and synchronized German and English guides.

### Security

- CORS remains disabled unless `allowed_origins` is configured; wildcard
  origins are rejected and CORS does not bypass API authorization.

## 0.1.31 — 2026-09-17

### Added

- API error declarations can now attach a checked payload type, for example
  `errors { 422 Validation: ValidationProblem }`.
- Typed API error payloads are returned under `error.details`, described in
  OpenAPI, and exposed through the generated TypeScript client.
- Added `examples/api_errors.zyl` and synchronized German and English API
  documentation.

### Compatibility

- Existing untyped declarations such as `errors { 404 NotFound }` keep their
  previous response shape.

## 0.1.30 — 2026-09-17

### Added

- Added `HttpResult<Response>` and `http_result<Request, Response>(...)` with
  status, headers, raw body, optional typed data, and structured `HttpError`.
- Non-2xx HTTP responses are now available as data through `error` instead of
  being forced into an unstructured runtime failure by this helper.
- Added typed `HttpResult` field checking, local integration coverage, and
  synchronized German and English documentation.

### Known limitations

- Transport failures and invalid JSON in a successful response remain runtime
  errors; response status and headers are available through `HttpResult`.

## 0.1.29 — 2026-09-17

### Added

- Added `http_json<Request, Response>(method, url, headers, body)` for
  automatic JSON encoding of request records and typed response decoding.
- Added automatic `Content-Type: application/json` when no content type is
  supplied, while preserving the existing Network policy and response limits.
- Added end-to-end local HTTP coverage and synchronized examples and docs.

### Known limitations

- `http_json` treats non-2xx responses as runtime errors and currently returns
  the decoded response value without exposing response headers to the caller.

## 0.1.28 — 2026-09-17

### Added

- Added `json_encode(value)` and `json_decode<Type>(text)` for checked JSON
  conversion of scalar values, arrays, options, and declared records.
- Added nested record decoding with required/optional field validation and
  rejection of unknown fields.
- Added typed-call syntax for the JSON decoder, synchronized examples, and
  German and English documentation.

### Known limitations

- JSON decoding currently supports records, arrays, options, and scalar values;
  `Result`, `Bytes`, date/time domain values, streaming JSON, and custom codec
  hooks remain future work.

## 0.1.27 — 2026-09-17

### Added

- Added `http_request(method, url, headers, body)` with typed
  `HttpResponse` values containing status, headers, and body.
- Added support for POST, PUT, PATCH, DELETE, and HEAD alongside GET, with
  safe separate header and body values.
- Added local request/response integration coverage and synchronized German
  and English documentation.

### Known limitations

- Request headers are represented as `Name: value` strings, and response
  bodies are UTF-8 strings. Multipart, streaming, and typed JSON decoding
  remain future work.

## 0.1.26 — 2026-09-17

### Added

- Added `run_process(command, args)` through the Process capability.
- Added exact command allowlists, shell-free argument passing, cleared child
  environments, closed stdin, timeouts, and bounded stdout/stderr capture.
- Added explicit runtime errors for denied commands, failed processes,
  timeouts, invalid UTF-8, and output-limit violations.
- Synchronized German and English documentation.

### Known limitations

- Working-directory selection, environment forwarding, shell pipelines, and
  process streaming remain future work.

## 0.1.25 — 2026-09-17

### Added

- Added HTTPS/TLS support to `http_get(url)` through the Rustls-backed `ureq`
  transport.
- Kept certificate verification enabled, disabled redirects, and preserved
  project host allowlists, timeouts, and response-size limits.
- Synchronized German and English documentation.

### Known limitations

- The Network API remains GET-only and returns successful UTF-8 response bodies;
  request headers, request bodies, and richer HTTP features remain future work.

## 0.1.24 — 2026-09-17

### Added

- Added the `http_get(url)` Network host API with static and runtime capability
  enforcement.
- Added project network policies with exact host allowlists, connection
  timeouts, and maximum response sizes.
- Added local HTTP integration tests and synchronized German and English
  documentation.

### Known limitations

- The first HTTP slice supports only `http://`, successful UTF-8 response
  bodies, no redirects, and no transfer-encoded responses. HTTPS/TLS and a
  richer HTTP client remain future work.

## 0.1.23 — 2026-09-17

### Added

- Added write_text(path, content), delete_file(path), and list_dir(path).
- Added project FileSystem policies with read_roots and write_roots.
- Relative paths are resolved from the project directory, and symlink targets
  are canonicalized before access.
- Extended CLI execution and API handlers with the project FileSystem policy.
- Added positive and negative tests and synchronized German and English
  documentation.

### Known limitations

- File-system roots must already exist; broader sandboxing and special file
  handling remain future work.

## 0.1.22 — 2026-09-17

### Added

- Added write_text(path, content), delete_file(path), and list_dir(path).
- Added project FileSystem policies with read_roots and write_roots.
- Relative paths are resolved from the project directory, and symlink targets
  are canonicalized before access.
- Extended CLI execution and API handlers with the project FileSystem policy.
- Added positive and negative tests and synchronized German and English
  documentation.

### Known limitations

- File-system roots must already exist; broader sandboxing and special file
  handling remain future work.

## 0.1.21 — 2026-09-17

### Added

- Added write_text(path, content), delete_file(path), and list_dir(path).
- Added project FileSystem policies with read_roots and write_roots.
- Relative paths are resolved from the project directory, and symlink targets
  are canonicalized before access.
- Extended CLI execution and API handlers with the project FileSystem policy.
- Added positive and negative tests and synchronized German and English
  documentation.

### Known limitations

- File-system roots must already exist; broader sandboxing and special file
  handling remain future work.

## 0.1.20 — 2026-09-17

### Added

- Added read_text(path) for explicit UTF-8 file reads through the FileSystem
  capability.
- Added static and runtime enforcement of the FileSystem capability.
- Added the runnable examples/filesystem_api.zyl example and synchronized the
  German and English documentation.

### Known limitations

- Writing, deleting, directory listing, and path allowlisting are not exposed
  yet.

## 0.1.19 — 2026-09-17

### Added

- Added read_text(path) for explicit UTF-8 file reads through the FileSystem
  capability.
- Added static and runtime enforcement of the FileSystem capability.
- Added the runnable examples/filesystem_api.zyl example and synchronized the
  German and English documentation.

### Known limitations

- Writing, deleting, directory listing, and path allowlisting are not exposed
  yet.

## 0.1.18 — 2026-09-17

### Added

- Added secure random_int(min, max) with inclusive bounds.
- Added static and runtime enforcement of the Random capability.
- Added the runnable examples/random_apis.zyl example and synchronized the
  German and English documentation.

### Known limitations

- Network, file-system, and process host APIs still require separate resource
  and error contracts.

## 0.1.17 — 2026-09-17

### Added

- Safe Clock and Environment host APIs: now() returns a Unix-epoch timestamp
  in milliseconds and env(name) returns an optional String.
- Static and runtime capability checks for Clock and Environment.
- Added secure random_int(min, max) with inclusive bounds and Random
  capability enforcement.
- Added the runnable examples/host_apis.zyl example and synchronized the
  German and English documentation.

### Known limitations

- Network, file-system, process, and random host APIs are still planned.
- Host APIs do not provide a complete operating-system sandbox.

## 0.1.16 — 2026-09-17

### Added

- WebApp database capability configuration through
  `with_database_capability`.
- Runtime denial of generated form, CRUD, and persistent-authentication
  database operations when the project does not grant `Database`.
- CLI `serve` now propagates the project database grant to the WebApp.
- Added WebApp capability tests and synchronized German and English
  documentation.

### Known limitations

- The OS-level capability sandbox and non-database host APIs remain future
  work.

## 0.1.15 — 2026-09-17

### Added

- Runtime capability enforcement for function calls and native SQL access.
- New runtime APIs for passing project capability grants to program and API
  execution.
- CLI `run` and `serve` now pass the grants from `zelyra.toml` into runtime
  execution, including executable API handlers.
- Added positive and negative runtime enforcement tests and synchronized the
  German and English documentation.

### Known limitations

- Network, file-system, process, clock, and random host APIs are not exposed
  yet; operating-system integration remains future work.

## 0.1.14 — 2026-09-17

### Added

- Runtime capability enforcement for function calls when project grants are
  supplied.
- Native SQL now checks the current function's `Database` capability directly
  before attempting a database connection.
- CLI `run`, `serve`, and executable API handlers now pass the grants from
  `zelyra.toml` into the runtime.
- Added positive and negative runtime capability tests and synchronized German
  and English documentation.

### Known limitations

- Network, file-system, process, clock, and random host APIs are not exposed by
  the runtime yet; operating-system integration remains future work.

## 0.1.13 — 2026-09-17

### Added

- Initial structured concurrency with `parallel` blocks and `await`
  expressions.
- Parallel branches run from immutable environment snapshots, are joined
  before execution continues, and merge their results in source order.
- Static diagnostics reject `await` outside `parallel` and invalid or duplicate
  parallel bindings.
- Added the runnable `examples/parallel.zyl` example and synchronized German
  and English documentation.

### Known limitations

- Cancellation and database connection-pool integration for parallel work are
  planned for a later release.

## 0.1.12 — 2026-09-17

### Added

- Failed logins are throttled per normalized e-mail address after five failed
  attempts within 15 minutes.
- A blocked login responds with HTTP 429 and `Retry-After: 60`.
- Successful logins rotate the session token and invalidate the previous
  token.

### Documentation

- Authentication throttling and session rotation documented in English and
  German.

## 0.1.11 — 2026-09-17

### Added

- `zelyra auth hash-password` creates Argon2 password hashes for authenticated
  user tables.
- Interactive password entry is hidden and requires confirmation; `--stdin`
  supports explicit automation without putting a password in command-line
  arguments.

### Documentation

- Authentication setup now documents the password-hash command in English and
  German.

## 0.1.10 — 2026-09-17

### Added

- `zelyra new <directory> --mariadb` generates a local MariaDB project template
  with `.env.example`, Dockerfile, and Docker Compose services for MariaDB and
  the Zelyra web server.
- `ZELYRA_WEB_PORT` selects the internal web-server port and the local port
  published by Docker Compose.

### Documentation

- Local Docker/MariaDB setup documented in English and German.

## 0.1.9 — 2026-09-17

### Added

- `zelyra db setup <file.zyl>` provides a clear beginner-friendly alias for
  creating a MariaDB database and applying its initial schema.
- Missing `DATABASE_URL` now includes safe Bash and PowerShell setup hints with
  password placeholders only; credentials are never stored or printed.

### Documentation

- MariaDB setup instructions updated in English and German.

## 0.1.8 — 2026-09-17

### Added

- `zelyra doctor --json` emits machine-readable readiness checks for CI and IDE
  integrations.
- JSON output includes project, compiler version, status, warnings, and named
  checks without exposing `DATABASE_URL` credentials.

### Documentation

- JSON diagnostics documented in English and German.

## 0.1.7 — 2026-09-17

### Added

- `zelyra doctor [file.zyl] [--port <port>]` checks project validation, schema,
  database connectivity, the Rust toolchain, and web-port availability without
  changing database state.
- Missing `DATABASE_URL` is presented as a warning; invalid projects,
  unreachable configured databases, and occupied ports produce a failing exit
  status.

### Documentation

- Beginner installation and readiness checks documented in English and German.

## 0.1.6 — 2026-09-17

### Added

- Windows PowerShell and `cmd.exe` installers with user-local installation,
  automatic Rust bootstrap, and user PATH setup.
- Repeated Linux/macOS installations now replace the existing CLI binary with
  `cargo install --force`.

### Documentation

- Cross-platform installation instructions added in English and German.

## 0.1.5 — 2026-09-17

### Added

- Generated TypeScript clients now expose declared API error names through
  `ZelyraApiErrorCode`.
- `ZelyraApiError.fromResponse` extracts HTTP status, error code, and server
  message from Zelyra's structured JSON error responses and keeps the raw body.

### Documentation

- TypeScript client error handling documented in English and German.

## 0.1.3 — 2026-09-17

### Added

- `zelyra doc <file.zyl> --typescript` generates a dependency-free TypeScript
  client from API, record, type, and table declarations.
- Generated clients support standard `fetch`, path/query parameters, JSON
  request bodies, bearer tokens, typed responses, and HTTP errors.
- OpenAPI output now reports the compiler package version automatically.

### Dokumentation

- README, Getting Started, Handbook, and Phase 10 documentation updated in
  English and German.

## 0.1.2 — 2026-09-17

### Added

- Structured `for ... in` iteration over arrays.
- Immutable loop variables scoped to the loop body.
- `break` and `continue` support in array loops.
- Parser, HIR, type-checking, runtime, and integration tests for array loops.

### Dokumentation

- README, Getting Started, Handbook, and Phase 10 documentation updated in
  English and German.
- The runnable `examples/array_for.zyl` example demonstrates iteration and
  loop control.

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
