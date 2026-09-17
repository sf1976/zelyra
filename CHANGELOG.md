# Changelog

All notable changes to the Zelyra compiler and runtime are documented here.
The language line remains `0.1`; the patch version identifies the compatible
compiler and repository release.

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
