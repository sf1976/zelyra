# Language line 0.1 conformance corpus

This directory is the first checked corpus for the declared language line
`0.1`. It is intentionally small; releases expand it by compiler phase and
semantic area. Rust integration tests copy each source into a temporary project
and invoke the built CLI so the corpus exercises the same command users run.

- `accepted/*.zyl` must pass `zelyra check` and remain stable under repeated
  `zelyra fmt` runs. The audited cases include nested records, arrays, typed
  maps, and pattern matching.
- `rejected/*.zyl` must fail with the stable diagnostic code in its sibling
  `.code` file. Capability denial is part of the checked contract.
- `runtime/*.zyl` must pass checking and produce the exact bytes recorded in
  its sibling `.out` file when run with the fixture project's declared
  capabilities.

Changing a fixture or its expected result is a language-contract change. The
review must explain why the expected result changed and identify its affected
compatibility rule. The test harness pins required baseline fixture names so
an empty or accidentally reduced directory cannot pass as a compatibility
check. New fixtures are additive unless an explicitly reviewed language
contract change removes one. This corpus is not a complete 1.0 conformance
claim.
