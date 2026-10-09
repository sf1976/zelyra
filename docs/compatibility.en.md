# Compatibility policy and format versions

**Scope:** proposal for development from 0.4.0 onward; not a compatibility
promise for unpublished development builds.

## Published baseline

The authoritative published compiler baseline is [`v0.3.0`](https://github.com/sf1976/zelyra/releases/tag/v0.3.0),
source commit [`cd0600a73c22ed84f40aef870c8128ce2cb45be0`](https://github.com/sf1976/zelyra/commit/cd0600a73c22ed84f40aef870c8128ce2cb45be0).
This is an experimental release, not a production approval. The open 0.4
development branch contains changes not included in that release. A branch
build is not 0.4.0 merely because it has newer features.

Until a format is explicitly declared stable and covered by a published
release, treat it as experimental. A Git tag version, a local binary version,
and a format version are distinct pieces of information.

## Actual format state in the repository

| Interface | Verified current state | Meaning |
|---|---|---|
| CLI machine output | Shared envelope `schema_version: "1"` with `command`, `success`, and `diagnostics` | JSON commands such as `check`, `context`, and `impact` use this envelope. Human terminal output is not a machine API. |
| Cross-module table references | Experimental `E-MOD-019`/`E-MOD-021` validation in unreleased 0.4 development | Recognized references require the imported table owner and a matching `access` grant (`read`, `write`, `read_write`) for the exact project-relative module path. CRUD, forms, and authentication require `read_write`, as do unknown SQL accesses. This is a compiler check over recognized edges, not a MariaDB permission; unknown SQL forms and schema-change rights remain unchecked. |
| Database provider in the module graph | Experimental `E-MOD-022` validation in unreleased 0.4 development | SQL functions and database-backed resources need a direct or transitive import path to the database module. An import only in `main.zyl` is not inherited by child modules. This does not enforce runtime connection selection. |
| Database bindings in `context` | Experimental additive field in unreleased 0.4 development; still `schema_version: "1"` | Reports the provider module, database consumers, import resolution, and connection variable including the actual `DATABASE_URL` fallback. Without a declaration, `DATABASE_URL` is marked as legacy configuration. Credentials stay hidden; one connection per process. |
| Cross-module views and components | Experimental public UI visibility in unreleased 0.4 development | `pub view` / `pub component` and a direct or transitive import path are required for page/CRUD layouts and recognized tags in page, view, component, and CRUD-slot HTML. `E-MOD-007` reports private UI declarations; `E-MOD-020` reports a missing dependency edge. HTML component discovery is a known-tag scan, not a complete HTML or namespace analysis. |
| `module bundle --dry-run` | Experimental `schema_version: "1"` envelope with a sorted file list and `writes_performed: false` | Lists planned destination paths after validating a temporary bundle; this is not a stable published interface and never marks deployment completeness true. |
| Edit request | Exact `schema_version: "1"` | Missing or other versions are rejected. |
| `zelyra.toml` | No generally interpreted format-version field is established | The existing feature reader recognizes `[features]`; this does not version the entire project manifest. |
| Generated project files | No generator/template version field is established | `zelyra new` creates project files; that does not imply an automatic upgrade or migration mechanism. |
| `db plan` | Text output or experimental `--format=json` plan `zelyra.schema-plan/v1` in unreleased 0.4 development | JSON contains a stable plan ID, schema SHA-256 fingerprints, drift, ordered SQL steps, and preflights for NULLs, non-empty tables, duplicate values, and foreign-key orphans. No persistent migration history or inverse SQL exists; `rollback.generated` is always `false`. Read-only preflights do not prevent concurrent writes. |
| Module bundle manifest | `format_version: 1` on the unpublished module development branch | Experimental manifest; not part of 0.3.0 and not a compatibility promise across development builds. |
| API route metadata | Experimental source fields `version`, `deprecated`, and `rate_limit` in the unreleased 0.4 branch | Responses expose API metadata; OpenAPI includes the same fields. Quotas are process-local, keyed by route and TCP peer IP, bounded to 4096 client buckets, and reset on restart. Reverse-proxy forwarding headers are not trusted. |
| Auth throttle settings | Experimental `login_rate_limit` and `login_block_seconds` source fields in the unreleased 0.4 branch | Defaults remain 5 failures per 900 seconds and a 60-second block. The process-local limiter is capped at 4,096 keys, rejects new keys when full, and resets on restart; reset flows are not implemented by these settings. |

These version fields were verified in the implementation in
`cli/src/main.rs`, `cli/src/edit.rs`, and the CLI tests. The module bundle
version must not be confused with the general CLI `schema_version`.

## Proposed rules for published formats

1. **Release boundary:** compatibility is checked against an exact published
   tag. Prerelease and branch builds may change project or manifest formats
   and must label them experimental.
2. **Machine-readable JSON:** an existing `schema_version` keeps its meaning
   and types. Optional additional fields are allowed within the same major
   schema version; consumers should ignore unknown fields. An incompatible
   change to required fields, types, or semantics requires a new major schema
   version and an actionable error for unsupported versions.
3. **Inputs:** versioned requests are validated fail-closed. An unknown version
   must not be silently interpreted as the current version. Tests must fix the
   version field, accepted values, and failure behavior.
4. **Project files and source language:** before an incompatible change to
   published stable syntax or configuration, document affected files,
   diagnostics, upgrade instructions, and the supported transition.
   Experimental constructs may change before a stable release but must be
   labeled accordingly in the handbook and release notes.
5. **Generated output:** a new generator version must not silently overwrite
   an existing application. Changes to templates and generated metadata need
   a reviewable diff and tests against previously published example projects.
6. **Database schemas:** `db plan` is currently a readable preview, not a
   version-stable migration artifact. This policy alone does not make schema
   changes reversible. Versioned plans, preflights, approval, drift handling,
   and recovery remain implementation and release gates in the 0.4/0.5 plans.
7. **Docs and tests:** every interface called stable gets positive and
   negative version/compatibility tests and is documented in German and
   English. A passing test for one version does not prove compatibility with
   an earlier release.

These rules do not claim that 0.3.0 stabilizes every format or that the current
0.4 development branch is backward compatible. They define the evidence
needed before making such a future promise.
