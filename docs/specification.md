# Zelyra 0.1 specification index

The repository's implementation specification is maintained across the phase
documents, the bilingual handbook, and the roadmap. This file defines the
cross-cutting AI-native contract that applies to all of them.

## AI-native, AI-independent development

> AI writes. Zelyra verifies.

Zelyra accepts people and AI systems as code authors without trusting either
author automatically. The compiler and tests decide whether source is valid.
The language remains fully usable without a model, provider, cloud service, or
network connection.

AI-facing interfaces are open, deterministic, machine-readable, versioned,
local-capable, and vendor-neutral. The current machine-oriented interfaces are:

```text
zelyra check <file.zyl> --format=json
zelyra context <file.zyl> --format=json
zelyra impact <file.zyl> --format=json
zelyra edit --format=json change.json
zelyra fmt <file.zyl> --check
```

The JSON interfaces use machine schema version `1`. JSON is stdout-only; logs are stderr-only.
Diagnostics have stable codes and half-open source spans using zero-based UTF-8
byte offsets and one-based line/byte columns. Outputs are deterministic and do
not contain secrets, timestamps, random IDs, absolute paths, or live database
contents.

The compiler must continue to enforce names, types, nullability, SQL, schemas,
forms, views, APIs, permissions, contracts, capabilities, tests, and
destructive-change approvals. AI output may not silently add capabilities,
expand permissions, execute destructive SQL, weaken checks, or expose secrets.
Expression typed holes, source-only impact analysis, and versioned semantic
rename previews are implemented slices; declaration-context holes, complete
runtime/schema impact, richer edit operations, granular effects, and
benchmarks remain roadmap work.
