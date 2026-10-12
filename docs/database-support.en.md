# Database support matrix

This matrix names the concrete servers and workflows that Zelyra CI runs
against them. An entry does not imply complete SQL or runtime parity. The CI
matrix is in [`ci.yml`](../.github/workflows/ci.yml). This table describes the
currently checked scope and its limits for each backend.

| Server | Verified workflows | Limits of the evidence |
| --- | --- | --- |
| MariaDB 10.11.19, 11.4.13, 11.8.9, 12.3.3 | Studio database map and CRUD builder; local TOTP MFA; parameters, pool limits, and statement timeouts; TLS; database/CRUD HTTP E2E; schema safety; backup and restore; handbook examples | MFA remains an explicitly configured preview. CI is neither an independent security review nor product acceptance. Not every optional database driver path is run on every version. |
| MySQL Server 8.4.11 | Parameterized Zelyra SQL queries; schema setup, inspection, read-only project map, and a reviewed safe column migration with history | Limited compatibility evidence. No proof for full CRUD, authentication, MFA, TLS, or backup/restore. The schema E2E covers one additive safe column change; do not infer full MariaDB parity. |
| PostgreSQL 16.15 | Studio introspection, parameterized runtime queries, and schema safety | No proof for full CRUD, authentication, MFA, TLS, backup/restore, or general SQL portability. CI pins the minor server version and checks it before the integration steps. |
| SQLite (CI image system version) | Schema integration, parameterization, and basic CRUD workflows | Local development option; not a substitute for the tested server databases and not a 1.0 production promise. |

## Consequence for Zelyra 1.0.0

MariaDB, MySQL, and PostgreSQL remain targets for the first productive release.
The verified scope differs by backend. 0.9.9 can freeze the 1.0.0 support
scope only after each promised workflow has evidence. Accepting a URL such as
`mysql://` is not evidence for MySQL Server support.
