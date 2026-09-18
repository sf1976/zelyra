# AI-native development architecture

Status: foundational architecture decision, Zelyra 0.1

## Goal and boundary

Zelyra is intended to be the most useful language for AI-assisted development
of database-backed business and web applications. This is a strategic goal,
not a present public benchmark claim. The product contract is:

> AI writes. Zelyra verifies.

People and AI systems may author the same Zelyra source. The compiler remains
the trust boundary. Zelyra must remain fully usable without an AI provider,
cloud account, network connection, or model invocation.

The AI-native layer covers compiler interfaces, diagnostics, project
introspection, semantic change proposals, impact analysis, safety checks, and
reproducible benchmarks. It does not make model output part of language
semantics.

## Authors and trust boundary

Human-written and AI-written code follow the same lexer, parser, name
resolution, type checker, SQL checker, capability checker, contract checker,
tests, and runtime rules. A model's explanation is never evidence of
correctness. Only compiler results, tests, security policy, and explicit human
approval can authorize a change.

AI tools must not silently add capabilities, widen permissions, execute
destructive SQL, approve destructive schema plans, disable diagnostics, weaken
tests, or expose credentials. Risky operations must produce a preview and
require an explicit human decision.

## Deterministic, versioned machine interfaces

Machine-readable CLI output uses a shared envelope:

```json
{
  "schema_version": "1",
  "command": "check",
  "success": false,
  "diagnostics": []
}
```

`schema_version` is mandatory. Existing fields keep their meaning within a
version. New optional fields may be added; incompatible changes require a new
schema version. Consumers must ignore unknown optional fields. JSON is written
only to stdout; operational logs belong on stderr. Human output remains the
default and is not replaced by machine output.

Compiler results are deterministic: diagnostic ordering follows source order
and stable phase order, declarations are emitted in source order, and no
timestamps, random IDs, absolute machine-specific paths, secrets, or database
contents are included unless explicitly requested by a future command.

Offsets are zero-based UTF-8 byte offsets. Lines and columns are one-based;
columns follow the current lexer convention and therefore count source bytes.
Spans are half-open (`start` included, `end` excluded). This convention is
documented now so future compiler crates can share it.

## Project context

`zelyra context --format=json` is a read-only, deterministic summary of the
source declarations the compiler understands. It contains project identity,
tables and fields, CRUD resources, views, APIs, forms, and authentication
metadata that is safe to expose. It never connects to a database, sends
network traffic, executes email/jobs, renders confidential data, or returns
secrets. Unsupported semantic details are omitted or represented as empty
arrays rather than invented.

The context is intentionally a bounded snapshot, not a replacement for type
checking. An invalid source produces diagnostics and a non-zero exit status.

## Canonical source formatting

`zelyra fmt <file.zyl>` produces deterministic source formatting after a
successful lexical and syntactic parse. `zelyra fmt <file.zyl> --check` does
not write files and exits non-zero when formatting would change the source, so
repositories can enforce canonical source in CI. The formatter preserves line
comments and treats SQL and HTML bodies as opaque source content. Formatting is
idempotent: formatting an already formatted file produces the same bytes.

## Typed gaps, semantic edits, and impact

Expression typed holes written as `_` are available as a first safe slice. The
compiler reports their contextual expected type, available values and
functions, active capabilities, contract obligations, and source span.
Buildable commands reject incomplete code before lowering or execution. Typed
holes in declaration contexts remain planned interfaces.

`zelyra impact --format=json` now provides a deterministic, source-only first
slice for affected tables, SQL, forms, CRUD resources, views, APIs,
permissions, and contracts. Emails, jobs, tests, live schema changes, and
deeper runtime dependency analysis remain planned. `zelyra edit --format=json
change.json` provides a validated, atomic preview for symbol renames and
returns a deterministic source fingerprint. The request must echo that
fingerprint when the explicit `--apply` flag atomically writes the validated
result, preventing stale overwrites. Richer operations remain additional
semantic change features; text patches remain supported.

## Security, privacy, and providers

The compiler does not send source code or project context to external AI
services. AI integrations must be open, versioned, local-capable,
vendor-neutral, and deterministic at the interface boundary. Machine output
must redact database URLs with passwords, API tokens, SMTP credentials,
private keys, session secrets, `.env` contents, and unnecessary personal data.

Database writes, network access, file access, process execution, mail, jobs,
and capability changes remain explicit effects. Human approval is required for
destructive schema operations and other irreversible actions.

## Compatibility and benchmarks

Machine interfaces are versioned independently from human prose. The project
must preserve the default human CLI and add machine formats without changing
their meaning unexpectedly.

The future Zelyra AI Benchmark will compare reproducible tasks such as adding
fields, repairing SQL, changing validation, adding permissions, updating
schemas, and fixing null handling. It will measure first-pass compilation,
correction loops, time to passing tests, tokens, code size, security findings,
missed dependencies, faulty database changes, and human review effort.
No superiority claim is made until controlled, reproducible results exist.
