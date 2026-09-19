# Zelyra contribution rules

Zelyra is developed as a statically typed, MariaDB-first language and
platform for database-backed business and web applications. Safety, explicit
effects, SQL validation, readable source code, and reproducible tests are
architectural requirements.

## AI-native development

Zelyra is designed as an AI-native but AI-independent language for
database-backed business applications.

Every architectural and language change must consider:

- deterministic machine-readable compiler output
- stable diagnostic codes
- precise source spans
- explicit types and effects
- minimal hidden context
- safe defaults
- structured project introspection
- semantic change impact
- human-readable source code
- human approval for destructive operations
- vendor-neutral AI tooling
- backwards-compatible versioned machine interfaces

AI-generated code is never trusted merely because it looks plausible. The
compiler, tests and security rules remain authoritative.

Do not weaken diagnostics, type safety, SQL validation, contracts,
capabilities or tests to make generated code pass.

## Digital sovereignty and engineering craft

Digital sovereignty is a core decision criterion for every Zelyra feature,
dependency, default, integration, and public claim. The manifesto in
`docs/MANIFESTO.de.md` and `docs/MANIFESTO.md` describes the product direction;
it must not be presented as proof that every aspiration is already delivered.

Before accepting a design, assess whether it:

- keeps application code, data, and infrastructure under the user's control;
- avoids mandatory cloud accounts and closed providers;
- prohibits telemetry, analytics, crash reporting, and background phone-home
  behavior;
- makes network access and other external effects explicit and reviewable;
- favors local execution, open interfaces, portable data, and replaceable
  dependencies without claiming that Zelyra has zero dependencies;
- preserves a simple default path while making advanced controls optional;
- measures memory, performance, and reliability rather than relying on
  superlatives;
- states exactly what is proven, checked at runtime, unproven, or unsupported.

Do not claim universal mathematical error-freedom, absolute independence,
unrestricted database portability, or peak performance unless the specific
scope is demonstrated and tested. Design-by-contract and the verifier provide
scoped guarantees, not a proof that every application is defect-free.
Telemetry, analytics, crash reporting, and background phone-home behavior are
prohibited; reconsidering that policy requires an explicit change to this core
rule by the project owner. Explicit application network operations are not
telemetry and must remain capability-checked, visible, and reviewable.

Rust is the current bootstrap implementation, not Zelyra's language identity
or a required end-user runtime dependency. Keep the source language and its
semantics independent of Rust, preserve Rust-free installation where
available, and track self-hosting as a long-term direction rather than a
completed capability.

## Working agreement

Before changing a phase, inspect the repository and the relevant bilingual
documentation. Implement syntax, semantic checks, diagnostics, positive and
negative tests, examples, and documentation together. Keep German and English
documentation synchronized. Do not claim a feature is implemented when it is
only a roadmap design.

Risky database operations, capability changes, secret handling, and external
side effects require explicit human-visible boundaries and must never be
silently approved by tooling.

For each milestone, check its effect on data ownership, offline/local use,
external network calls, telemetry, dependency surface, database portability,
resource use, and the accuracy of public guarantees. Record any unmet
sovereignty goal as a limitation or roadmap item.

Every new environment variable or project configuration switch must be added
to `docs/env.md` and `docs/env.en.md` before commit, including its default,
precedence, security classification, affected commands, and tests.

After every completed and tested development step, review both
`docs/ROADMAP.md` and `docs/ROADMAP.de.md`. Update affected statuses and text
in the same change. The roadmap legend maps the existing machine-searchable
markers to `✅` (complete and tested), `🧪` (partial or experimental), `🗺️`
(planned), `◻️` (optional), and `⛔` (documented blocker or deliberate
deferral). Keep the English and German roadmaps semantically synchronized.

## Git workflow

- Create a commit after each completed and tested subfeature.
- Push after each verified milestone or at the latest before a longer pause.
- Create releases less frequently, only for stable public versions.
- Avoid both oversized aggregate commits and commits for every trivial change.
- Keep commits small, focused, and understandable; do not push unverified
  changes or publish a release merely because a commit was created.

### Branch lifecycle

- Keep `main` releasable and use a focused feature, fix, documentation, or
  chore branch for each independent task.
- Before merging, verify that the branch is clean, tested, and based on the
  current `main` (or has been rebased or merged from it as appropriate).
- Merge only completed work into `main`; do not leave known failing tests or
  unfinished experiments in the branch being merged.
- After a successful merge, push `main` and verify that the remote branch
  points to the merge result.
- Delete the merged local and remote topic branch after the merge. Retain a
  branch only when it contains intentionally unmerged work, is needed for an
  active review, or is explicitly preserved for a release or maintenance
  reason.
- Do not delete `main`, release tags, or a branch that is not demonstrably
  merged. Check `git branch --merged main` and
  `git branch --no-merged main` before cleanup.
- If a branch was merged with squash or rebase, compare its changes with
  `main` before deletion; ancestry alone may not prove equivalence in that
  case.
- Branch deletion is cleanup, not a release. Create or move tags only at the
  separately approved release milestone.
