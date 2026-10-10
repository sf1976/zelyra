# Zelyra language compatibility

Zelyra's compiler package version and language compatibility line describe two
different things. A compiler release such as `0.7.0` may add features while a
project continues to target language line `0.1`.

## Project declaration

Generated projects declare the line in `zelyra.toml`:

```toml
[project]
name = "invoice-app"
version = "0.1.0"
zelyra = "0.1"
```

In compiler release 0.7, `zelyra check`, `build`, `run`, and `serve` accept
`0.1`. An explicitly requested line that the compiler does not support fails
with `E-LANG-001`. Projects without a `zelyra` declaration continue to use
`0.1` as their legacy default, so adding this check does not break older
projects. The field selects a language contract; it does not select a database
backend or compiler binary.

## What line 0.1 covers

The line covers source grammar, type and name rules, and the behavior of
language constructs and built-in functions. The current grammar and supported
constructs are described in the [specification index](specification.md) and
the phase references. CLI machine-output schemas, project settings, database
backends, and generated web behavior have separate documented contracts.

While Zelyra is before 1.0, line 0.1 is an actively expanding, experimental
compatibility target. Additions may extend the language while keeping existing
valid 0.1 programs valid. A change that intentionally breaks valid source or
changes the meaning of an existing construct requires a separately named
language line and migration notes. It must not be introduced silently under
`0.1`.

## Evolution through 1.0.0

Language line `0.1` remains the same line throughout compiler releases through
`0.99.0`. Release numbers such as `0.8.0` and `0.99.0` identify Zelyra
releases, not new language lines. A compiler upgrade never changes a project's
language line automatically. Introduce a new line only when a necessary change
breaks valid programs or changes their established meaning; it requires its
own specification, migration notes, and an explicit project change.

The work proceeds through verifiable stages:

| Release | Language contract and evidence |
| --- | --- |
| `0.7.0` | Enforce `zelyra = "0.1"`, report `E-LANG-001` for unsupported explicit lines, retain projects that omit the field, and add a first runnable corpus for accepted, rejected, and runtime-checked examples. |
| `0.8.0` | Systematically expand the corpus for lexing, parsing, type checking, and formatting. Each grammar family gets positive and negative fixtures; formatter output is checked for idempotence. |
| `0.9.0` | Add runtime and module fixtures for operator and built-in semantics, error paths, effects, contracts, imports, and SQL typing. Check each supported database separately for the subset it claims. |
| `0.95.0` | Audit the language reference and corpus against the planned 1.0 scope. Expand baseline fixtures for nested records, arrays, typed maps, deterministic output, and denied capabilities. Document unresolved semantics, known gaps, and migration cases; do not label unchecked areas stable. |
| `0.99.0` | Freeze language features. Freeze the grammar, type rules, and semantics of `0.1` as a release candidate; allow only fixes, security corrections, and required compatibility fixes. Document and regression-test every exception. |
| `1.0.0` | After independent human acceptance, publish the checked contract, corpus, and support matrix together as the first production language release. |

These stages are acceptance criteria, not claims that planned work is already
implemented. Version 0.7 introduced the first intentionally small corpus. The
0.95 audit adds focused executable cases for nested records, arrays, typed
maps, deterministic map output, and capability denial. It remains an incomplete
cross-release conformance set and does not freeze the language contract for
production. The 1.0 release must publish the supported grammar, compatibility
guarantees, and tested conformance set together.

## What compatibility means

For a declared `0.1` line, the rules are:

- A valid program remains syntactically valid and well typed. New keywords,
  operators, or built-in names must not make it invalid or silently change its
  meaning.
- Defined runtime semantics do not change silently. A bug fix that changes
  observable behavior for valid programs needs a documented assessment, a
  regression fixture, and, where appropriate, a new language line.
- New features must fit together across syntax, types, effects, diagnostics,
  formatting, and runtime. Incomplete or backend-specific features must be
  labeled and must not be presented as a general language guarantee.
- New compiler versions retain support for declared lines while those lines
  remain in their support matrix. The compiler package can independently gain
  features and fixes.
- Diagnostic codes are the stable tool interface. Diagnostic wording and
  source spans may become more precise; tests preserve codes, relevant spans,
  and clear examples.
- Formatter output must be idempotent and must not change program meaning. A
  style change is documented but does not create a new language line.

Compiler binaries, CLI machine schemas, project configuration, database
drivers, and generated web behavior have their own support and version rules.
The language line does not promise automatic portability between backends.

## Compatibility checks

Every release that supports line 0.1 must run the versioned language corpus.
Each fixture records its input, expected phase, success or stable diagnostic
code, and expected runtime result where applicable. Golden machine-output tests
remain tied to their separately versioned schemas. The corpus covers accepted
and rejected syntax, diagnostics, Unicode source positions, arithmetic and
decimal rules, option/result handling, effects, contracts, SQL typing, module
boundaries, and formatter idempotence. New language features bring positive and
negative fixtures. Changes to expected results need an explanation in the
same review record.

See the [release roadmap](ROADMAP.md) for the planned compatibility corpus and
the [implemented inventory](implemented.en.md) for the exact released status.
