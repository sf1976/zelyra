# Compatibility policy and format versions

**Scope:** working compatibility policy for development after 0.4.0. This
document records current guarantees and proposals; it adds no guarantee beyond
the exact published release.

## Published baseline

The authoritative published baseline is [`v0.4.0`](https://github.com/sf1976/zelyra/releases/tag/v0.4.0),
source commit [`507c29e95084a59029d6b436bd69d9132c3a8937`](https://github.com/sf1976/zelyra/commit/507c29e95084a59029d6b436bd69d9132c3a8937).
It is the current stable release. A stable release does not make every
experimental command, generated format, or JSON field a compatibility promise;
use the interface table below and the exact tag when checking support. Work
after this baseline belongs to the separate 0.5.0 development branch.

Until a format is explicitly declared stable and covered by a published
release, treat it as experimental. A Git tag version, a local binary version,
and a format version are distinct pieces of information.

## Actual format state in the repository

| Interface | Verified current state | Meaning |
|---|---|---|
| CLI machine output | Shared envelope `schema_version: "1"` with `command`, `success`, and `diagnostics` | JSON commands such as `check`, `context`, and `impact` use this envelope. Human terminal output is not a machine API. |
| Cross-module table references | Published in 0.4.0; `E-MOD-019`/`E-MOD-021` validate recognized accesses | Recognized references require the imported table owner and a matching `access` grant (`read`, `write`, `read_write`) for the exact project-relative module path. CRUD, forms, and authentication require `read_write`, as do unknown SQL accesses. This is a compiler check over recognized edges, not a MariaDB permission; unknown SQL forms and schema-change rights remain unchecked. |
| Database provider in the module graph | Published in 0.4.0; `E-MOD-022` validates imports | SQL functions and database-backed resources need a direct or transitive import path to the database module. An import only in `main.zyl` is not inherited by child modules. This does not enforce runtime connection selection. |
| Database bindings in `context` | Published additive field in 0.4.0; still `schema_version: "1"` | Reports the provider module, database consumers, import resolution, and connection variable including the actual `DATABASE_URL` fallback. Without a declaration, `DATABASE_URL` is marked as legacy configuration. Credentials stay hidden; one connection per process. |
| Cross-module views and components | Published experimental public UI visibility in 0.4.0 | `pub view` / `pub component` and a direct or transitive import path are required for page/CRUD layouts and recognized tags in page, view, component, and CRUD-slot HTML. `E-MOD-007` reports private UI declarations; `E-MOD-020` reports a missing dependency edge. HTML component discovery is a known-tag scan, not a complete HTML or namespace analysis. |
| `module bundle --dry-run` | Experimental `schema_version: "1"` envelope with a sorted file list and `writes_performed: false` | Lists planned destination paths after validating a temporary bundle; this is not a stable published interface and never marks deployment completeness true. |
| Edit request | Exact `schema_version: "1"` | Missing or other versions are rejected. |
| `zelyra.toml` | No generally interpreted format-version field is established | The existing feature reader recognizes `[features]`; this does not version the entire project manifest. |
| Generated project files | No generator/template version field is established | `zelyra new` creates project files; that does not imply an automatic upgrade or migration mechanism. |
| `db plan` | Published experimental `--format=json` plan `zelyra.schema-plan/v1` in 0.4.0 | JSON contains a stable forward plan ID, schema SHA-256 fingerprints, drift, ordered SQL steps, and preflights for NULLs, required columns, duplicate values, and foreign-key orphans. `rollback` includes a fingerprint-bound reverse plan and its own ID when every reverse operation is supported; unsupported reversals fail closed. Any schema change requires an independently verified backup, and reverse DDL can discard later data. All three schema backends record migration plans and outcomes in `_zelyra_schema_history`; MariaDB records per-step checkpoints. Read-only preflights do not prevent concurrent writes. |
| Module bundle manifest | `format_version: 1` on the unpublished 0.5.0 preparation branch | Experimental manifest; not part of 0.4.0 and not a compatibility promise across development builds. |
| API route metadata | Published source fields `version`, `deprecated`, and `rate_limit` in 0.4.0 | Responses expose API metadata; OpenAPI includes the same fields. Quotas are process-local, keyed by route and TCP peer IP, bounded to 4096 client buckets, and reset on restart. Reverse-proxy forwarding headers are not trusted. |
| Auth throttle settings | Published `login_rate_limit` and `login_block_seconds` source fields in 0.4.0 | Defaults remain 5 failures per 900 seconds and a 60-second block. The process-local limiter is capped at 4,096 keys, rejects new keys when full, and resets on restart; reset flows are not implemented by these settings. |
| Session device label | Published optional `device_label: String(255)` column on an authentication session table | New sessions record a bounded, control-character-free client `User-Agent` only when the column exists. Existing tables remain valid without it. The value is untrusted display metadata, not device authentication; IP addresses are not stored. |

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
6. **Database schemas:** 0.4.0 publishes the experimental
   `zelyra.schema-plan/v1` format. Its plan and preflights do not by themselves
   make a schema change safe or reversible; backup, concurrent-write, and
   interrupted-update limits remain explicit. Further recovery evidence is a
   0.5.0 release gate.
7. **Docs and tests:** every interface called stable gets positive and
   negative version/compatibility tests and is documented in German and
   English. A passing test for one version does not prove compatibility with
   an earlier release.

These rules do not claim that 0.4.0 stabilizes every format or that the
unpublished 0.5.0 development branch is backward compatible. They define the
evidence needed before making such a future promise.
