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

The 0.7 compiler enforces the declared line, but it does not yet ship a complete
cross-release conformance corpus or freeze the 0.1 language contract for
production. Those remain release work on the path to 1.0.0. The 1.0 release
must publish the supported grammar, the line's compatibility guarantees, and
the tested conformance set together.

## Compatibility checks

Every release that supports line 0.1 must run the language fixtures through
lexing, parsing, type checking, formatting, and execution where applicable.
Fixtures should cover accepted and rejected syntax, diagnostics, Unicode source
positions, arithmetic and decimal rules, option/result handling, effects,
contracts, SQL typing, and module boundaries. Golden machine-output tests
remain tied to their separately versioned schemas.

See the [release roadmap](ROADMAP.md) for the planned compatibility corpus and
the [implemented inventory](implemented.en.md) for the exact released status.
