# Language line 0.1 conformance corpus

This directory is the first checked corpus for the declared language line
`0.1`. It is intentionally small; releases expand it by compiler phase and
semantic area. Rust integration tests copy each source into a temporary project
and invoke the built CLI so the corpus exercises the same command users run.

- `accepted/*.zyl` must pass `zelyra check` and remain stable under repeated
  `zelyra fmt` runs.
- `rejected/*.zyl` must fail with the stable diagnostic code in its sibling
  `.code` file.
- `runtime/*.zyl` must pass checking and produce the exact bytes recorded in
  its sibling `.out` file when run with the fixture project's declared
  capabilities.

Changing a fixture or its expected result is a language-contract change. The
review must explain why the expected result changed and identify its affected
compatibility rule. This corpus is an initial baseline, not a complete 1.0
conformance claim.
