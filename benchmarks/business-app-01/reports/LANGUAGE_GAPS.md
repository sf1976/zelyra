# Mini-ERP language gap log

This log records language gaps found while applying Zelyra to practical
business applications. `RESOLVED` means the root cause was fixed and its
behavior is covered by regression tests; it does not imply that Mini-ERP is
complete.

## BA01-001 — Standalone forms could not call typed Zelyra logic

```text
ID: BA01-001
Date: 2026-10-24
Affected ERP module: Password-generator prerequisite; reusable for business forms

Business requirement:
A validated standalone form must invoke native Zelyra logic and display a
computed result without introducing an application-specific foreign-language
handler or persisting a value that should remain transient.

Expected Zelyra implementation:
A form action references a typed Zelyra function, declares its effects, receives
validated field values, and renders its result safely.

Actual problem:
Standalone forms supported validation and SQL actions, but could not call a
typed Zelyra function. The password generator therefore had no idiomatic native
path for secure random generation from validated form fields.

Cause:
The form AST and web bridge had no function-handler reference or typed
field-to-parameter invocation path.

Affected language component:
AST, parser, project linker, static action checks, CLI runtime bridge, web form
runtime, localization, architecture/impact/reflection metadata.

General solution:
Add a handler reference for standalone form actions. Require exactly one
handler action; require a non-generic function returning `String`; require
field and parameter names, order, and types to match; keep capability checks;
reject SQL/action-body/redirect mixing and file inputs; escape output and send
it with no-store headers.

Architecture decision:
Use ordinary checked Zelyra functions and the existing effect/capability
model. Keep the feature limited to standalone forms; database-backed CRUD
actions continue to use their existing typed execution mechanism.

Implemented change:
Implemented on the `feature/standalone-native-form-handlers` worktree. The
password-generator project and HTTP E2E test demonstrate the path. Publication
and merge into `main` are still pending.

New tests:
Parser syntax test; CLI positive/negative field, return type, and capability
checks; web CSRF, escaped result, no-store, and no-referrer test; live app E2E
for all five character profiles and invalid fields.

Regression result:
Parser, CLI, and web tests pass. The full `cargo test --workspace --locked`
suite passes (including 674 CLI tests and 225 web tests; 4 web tests and 2 CLI
tests are ignored, and MariaDB-only integrations require a disposable DB).
`cargo clippy --workspace --all-targets --locked -- -D warnings`, handbook
snippet validation, formatting, and diff checks pass. The app compiler check,
native Zelyra tests, and live HTTP E2E pass both in this benchmark directory
and at `/opt/zelyra-password-generator`.

Status: TESTING
```
