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

## Working agreement

Before changing a phase, inspect the repository and the relevant bilingual
documentation. Implement syntax, semantic checks, diagnostics, positive and
negative tests, examples, and documentation together. Keep German and English
documentation synchronized. Do not claim a feature is implemented when it is
only a roadmap design.

Risky database operations, capability changes, secret handling, and external
side effects require explicit human-visible boundaries and must never be
silently approved by tooling.

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
