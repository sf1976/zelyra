# Local Zelyra packages (experimental)

This unreleased 1.1.5 work adds offline local packages on top of the existing
explicit module imports. It does not download packages or run build scripts.
Projects without a `[package]` table keep the existing file-import behavior.

Local packages are currently source dependencies, not installable modules
managed through ordinary application pages. `zelyra module list` provides a
CLI catalogue of source modules, exports, package versions, checksums, effect
ceilings, and typed setting schemas from package manifests. Values can be
stored in `zelyra.settings.toml` and read by package code through
`module_setting<T>`.
Secrets are supplied only through process-environment references. Studio has a
protected local settings screen; application-served settings pages and secret
management beyond that process environment remain open. Reusable
business and integration modules (including auth, ODBC, CSV, PDF, and Excel)
are roadmap goals; the package format alone does not provide those features.

## Module inventory

```sh
zelyra module list main.zyl
zelyra module list main.zyl --format json
```

The JSON output is deterministic and includes project-relative source paths,
import aliases, public exports, local package identities and checksums, and
declared effect ceilings. These ceilings do not grant runtime capabilities.
The inventory does not include `.env` values, credentials, or other secrets.
For package dependencies, the lock is reported as current only after the
normal compiler validation has accepted it.

## Manifest

Create a local package without an installer, registry, build script, or
network access:

```sh
zelyra module init packages/customer_tools --name customer_tools --version 0.1.0
```

The command creates a typed public example function, a package manifest with
the current CLI compatibility range and an empty effect ceiling, and a short
README. It refuses to replace an existing path. Add the path and a SemVer
version to the application's `[dependencies.<alias>]`, import it with
`import "pkg:<alias>" as tools`, then run `zelyra package lock .` and
`zelyra check main.zyl`.

An application declares its identity and local dependencies in `zelyra.toml`:

```toml
[package]
name = "invoice_app"
version = "1.0.0"
entry = "main.zyl"
zelyra = ">=1.1.0, <1.2.0"

[dependencies.money]
path = "packages/money"
version = "^1.2"
```

The package at `packages/money/zelyra.toml` declares its own identity and
entry. `[package]` accepts `name`, `version`, optional `entry`, `zelyra`, and
`effects`, so misspelled settings are reported instead of silently ignored.
The optional `zelyra` field is a SemVer requirement checked against the running
Zelyra CLI version whenever the package graph is loaded. For example,
`">=1.1.0, <1.2.0"` accepts compatible 1.1 releases and rejects other CLI
versions before the package is checked or run. It is part of the package
manifest checksum, so changing it makes the lock stale. `zelyra module list`
includes this requirement in its human and JSON inventories:

```toml
[package]
name = "money"
version = "1.2.3"
entry = "src/lib.zyl"
zelyra = ">=1.1.0, <1.2.0"
effects = ["Database(read)"]
```

`effects` is optional. When present, it limits every function's declared
`uses` effects in the package to the listed known capability names. The ceiling
must match the capabilities the functions declare; for SQL reads, use the
granular `Database(read)` capability rather than the broader `Database` when
that is what the function declares. `[]` permits pure functions with no
declared effects. When omitted, there is no
additional package ceiling. This field does not grant runtime rights: the
application configuration must still authorize each effect. Unknown and
duplicate names are rejected. Because the manifest is hashed, changing this
field makes the lockfile stale.

## Trust and integrity

Package sources are executable application code. A lockfile SHA-256 detects
changes relative to the lockfile that the project already trusts; it does not
authenticate a publisher, prove where files came from, or make a malicious
package safe. `zelyra package lock --update` deliberately accepts the current
local files as the new locked contents, so review the source and diff before
using that option. The package system currently has no registry, signatures,
publisher identities, or remote downloads. Package `effects` ceilings are
additional compile-time limits; they do not grant or bypass application
capabilities, SQL checks, contracts, or runtime policy.

Packages may describe configuration fields under `[settings.<name>]`:

```toml
[settings.api_timeout]
type = "Int"
required = false
default = 15
description = "Request timeout in seconds"

[settings.api_token]
type = "String"
required = true
secret = true
description = "Private API credential"
```

The schema currently accepts `String`, `Bool`, `Int`, and `Float`. `required`
must be explicit. Optional settings need a type-matching default. Secrets may
use any supported scalar type but may not define defaults. Environment values
are parsed strictly as `Bool`, `Int`, or finite `Float`; secret strings are
limited to 8 KiB including JSON encoding. Names use lowercase snake_case and
are limited to 64 bytes; string defaults are limited to 4 KiB.
Descriptions are optional and limited to 512 bytes. `module list` returns this schema in deterministic JSON and human output, but
not configured values. Schema changes are covered by the package manifest
checksum and make the lock stale.

## Validating application settings (experimental)

Project values are stored separately from `zelyra.toml` in the optional
`zelyra.settings.toml`. This keeps them out of package checksums and the lockfile.
Packages are keyed by their normalized workspace path; use `.` for the project
root:

```toml
[package_settings."packages/ledger"]
api_timeout = 15
api_token = { env = "LEDGER_API_TOKEN" }
```

`zelyra module settings check main.zyl [--format=json]` validates values against
the schemas of loaded packages. Supported types are `String` (up to 4 KiB),
`Bool`, `Int`, and finite `Float`. An omitted optional setting uses its
declared default; a missing required setting reports `E-PKG-SETTINGS-002`.
Unknown packages/fields, wrong types, symlinks, and files larger than 1 MiB are
rejected. This settings document is not part of package checksums.
The JSON output from `module settings check` also includes `required`, allowing
tools to distinguish required values from optional defaults.

Secrets must use `{ env = "UPPERCASE_NAME" }` references. The check reads,
type-checks, and converts the process value to the declared scalar type
(`String`, `Bool`, `Int`, or finite `Float`); it never emits the variable name
or secret value, and reports `secret_available` or `missing`. Empty or invalid
values are rejected with a value-free diagnostic. Variables must
be present in the Zelyra process environment before the command starts; the
CLI does not load `.env` for this setting source.
Before execution, `zelyra run` and `zelyra serve` also require all mandatory
package values to be present and type-valid. A missing required value reports
`E-PKG-SETTINGS-002`; secret environment-variable names are not emitted.
Package code reads values explicitly and with a type, for example
`module_setting<Int>("api_timeout")`. The name must be a string literal, the
setting must be declared by the owning package manifest, and the requested
type must exactly match its schema. Other packages' settings are inaccessible.
Optional fields use their manifest default; secrets come from the referenced
process environment variable. Runtime debug output redacts values. This access
works with AST, MIR, and bytecode. MIR and bytecode remain experimental overall;
this setting alone does not establish complete backend parity. Zelyra Studio provides a protected local **Module settings** screen to view and edit these values. Non-secret values are type-checked and saved atomically with revision checks; unknown fields, invalid types, and symlinks are rejected. The screen never displays existing secret values or stored environment-variable names. A new environment reference can be entered for a secret; leaving that field blank preserves its existing reference. Management through ordinary application-served Zelyra pages and secret management beyond process environment variables remain open.

If omitted, the entry defaults to `main.zyl` for an application with that file,
or `src/lib.zyl` for a library. A package name uses lowercase ASCII letters,
digits, `_`, and `-`; names begin with a lowercase letter. The dependency key
is a local alias and may differ from the package name; the package version
must satisfy the declared SemVer requirement. Only project-relative `path` dependencies
with an explicit `version` are supported. Paths use `/` separators; `..` may
refer to a sibling directory only while the resolved package remains inside
the project workspace. Symlink traversal, workspace escapes, backslashes,
registries, Git sources, and native plugins are not supported.

## Import and lock

Import a dependency by its manifest key using the existing namespace syntax.
The dependency key can differ from the package's own identity:

```zelyra
import "pkg:money_core" as money

fn main() {
    print(money::add_cents(1250, 50))
}
```

For this import, `[dependencies.money_core]` may point to a package whose
`[package].name` is `money`. The import name remains explicit and package
identity and version compatibility are checked independently.

```toml
[dependencies.money_core]
path = "packages/money"
version = "^2.1"
```

The package entry must export imported declarations with `pub`. Files imported
inside a package resolve relative to that package; application file imports
remain project-root-relative. Existing cycle, visibility, type, and capability
checks still apply.

Two dependency aliases may point to different local versions that share the
same `[package].name`, for example `math@1.x` at `packages/math-v1` and
`math@2.x` at `packages/math-v2`. A transitive importer uses its own manifest
alias for each package edge. The lock records both package paths and versions;
no version is selected implicitly.

`zelyra module bundle` claims a complete source closure only when the static
plan has no unresolved references and the generated entry passes the compiler
check. Its manifest records that evidence. `complete_deployment` remains
`false`; runtime effects and operational requirements still need review.

Module planning accepts `pkg:<dependency-key>` and resolves it to the package
entry. A bundle includes packages in the selected module closure and their
transitive dependencies. It preserves package manifests and every Zelyra source
file covered by their checksums, filters unused project dependencies from the
generated application manifest, and writes a matching lockfile. Unused project
packages are omitted. The closure is compiler-checked but is not a guarantee of
a complete or operational deployment.

For `zelyra module bundle ... --docker --compiler-ref <40-hex-commit>`, the
commit must contain the language and package features used by the selected
closure. The generated Docker build compiles that exact compiler, copies the
bundle into its build stage, and runs `zelyra check main.zyl` before producing
the runtime image. A stale or incompatible compiler pin therefore fails the
image build. `complete_deployment` remains `false` until external effects and
operational requirements are reviewed.

Create and validate a reproducible local lock:

```sh
zelyra package lock main.zyl
zelyra package lock main.zyl --check
zelyra check main.zyl
zelyra run main.zyl
```

The generated `zelyra.lock` lists package instances in normalized workspace
path order, with each name, version, path, dependency edge, and SHA-256
checksum. Different versions may have the same package name. A checksum covers each package's
`zelyra.toml` and `.zyl` files in sorted UTF-8 relative-path order after CRLF
line endings are normalized to LF. Package symlinks are rejected. Editing a manifest or source makes the lock stale; review the
change, then run `zelyra package lock main.zyl --update`. `zelyra check`,
`build`, `run`, and `serve` reject a stale or missing lock when dependencies
are declared. Lock validation is local and offline.

The checksum deliberately omits generated `target/` output and Git metadata in
`.git/`. A package entry and package source imports may not enter either
directory, so executable Zelyra sources cannot bypass the lock checksum.

## Scope and security limits

This is the first package-resolution slice, not the complete 1.1.5 module and
package milestone. It supports multiple locally resolved versions of the same
package name when they are reached through distinct dependency aliases and
local paths. Package
instances are locked by path, name, version, and content hash; two different
paths claiming the same name and version are rejected as ambiguous. The
resolver supports workspace-local paths, exact local content hashes, and
existing `pub` visibility. It does not provide a registry, signed provenance,
or package trust prompts. HostServices is a versioned Rust embedding API
(currently v7), not a stable native ABI or package plugin contract. Package
checksums detect content changes relative to a reviewed lockfile; they do not
prove who published a package or whether its behavior is safe. Review package
source, effects, and lockfile changes before running an application that uses
the package. Package sources are local workspace paths; Zelyra does not fetch
them from an untrusted remote registry.

The `examples/business_modules` project also contains a pure `csv_import`
package. Its `decode_table(source, delimiter)` uses the bounded CSV decoder,
requires a header row with non-empty column names, and rejects records whose
field count differs from the header. It returns parsed rows and performs no database writes. The example
runs this package with AST, MIR, and Bytecode; this does not provide column
mapping, preview UI, or transactional import into an application database.
Package-level `effects` ceilings constrain declared function effects but do
not replace application capability grants. Package source is loaded for
analysis and compiled normally; `check` does not execute it. `run` executes the
application just as it does for ordinary imported modules.
