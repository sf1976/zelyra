# NULL-aware edit snapshots

Snapshot: unreleased 0.9.9 development branch. [Deutsch](edit-snapshots.de.md)

Generated MariaDB/MySQL edit forms sign the field values loaded from the
database. Before an update, the same transaction locks and reads the current
record, verifies the snapshot and only then writes. A changed record returns
HTTP 409; the pending update is not applied.

Native SQL NULL flags are part of the signed data. NULL, the literal string
`\N`, and an empty string are different states. The driver text alone is not
used to infer absence. Missing/inconsistent NULL metadata, unexpected row widths
and multiple rows fail closed. The signature stays bound to the CSRF key and
field set and uses the existing HMAC-SHA256/64-hex-character transport.
Non-NULL payloads retain their previous signatures. Forms opened before this
change with NULL fields may require a reload because their old signatures did
not encode nullness.

For display, a real NULL becomes an empty input; literal `\N` remains visible.
An omitted or empty optional field binds native SQL NULL, including nullable
columns declared without an explicit `Option<T>`. Required/read-only field
checks remain. Empty optional input follows the existing NULL convention; use
SQL explicitly if an application needs to store an empty string separately.

This fixes stale edits in both directions between NULL and literal text. It
also fixes missing SQL parameters when optional, implicitly nullable fields
are omitted. Raw SQL/HTTP/JSON row formats remain unchanged. Audit detail text
keeps its existing representation; this is not a general audit-format upgrade.

PostgreSQL guarded edit workflows still require the separate decision-making
transaction/runtime package. No guard is bypassed and no schema migration,
new environment setting, dependency or external service is added.
See [the package evidence](release-readiness/0.9.9-null-edit-snapshots.en.md)
and [the release plan](release-plans/0.9.9.en.md).
