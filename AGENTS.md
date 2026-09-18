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
