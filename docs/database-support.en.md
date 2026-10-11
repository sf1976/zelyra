# Database support matrix

This matrix names the concrete servers and workflows that Zelyra CI runs
against them. An entry does not imply complete SQL or runtime parity. The CI
matrix is in [`ci.yml`](../.github/workflows/ci.yml); every check for the exact
0.9.0 candidate must pass before publication.

| Server | Verified workflows | Limits of the evidence |
| --- | --- | --- |
| MariaDB 10.11.19, 11.4.13, 11.8.9, 12.3.3 | Studio database map and CRUD builder; local TOTP MFA; parameters, pool limits, and statement timeouts; TLS; database/CRUD HTTP E2E; schema safety; backup and restore; handbook examples | MFA remains an explicitly configured preview. CI is neither an independent security review nor product acceptance. Not every optional database driver path is run on every version. |
| MySQL Server 8.4.11 | Parameterized Zelyra SQL queries against the real MySQL server | Limited compatibility evidence; no proof for Studio, full CRUD, authentication, MFA, schema setup, TLS, or backup/restore. Do not infer full MariaDB parity. |
| PostgreSQL 16 | Studio introspection, parameterized runtime queries, and schema safety | No proof for full CRUD, authentication, MFA, TLS, backup/restore, or general SQL portability. |
| SQLite (CI image system version) | Schema integration, parameterization, and basic CRUD workflows | Local development option; not a substitute for the tested server databases and not a 1.0 production promise. |

## Consequence for Zelyra 1.0.0

MariaDB, MySQL, and PostgreSQL remain targets for the first productive release.
The current 0.9 evidence differs by backend. The 0.95 plan must add the
required product workflows and document their limits; 0.99 can freeze the
1.0.0 support scope only after that evidence exists. Accepting a URL such as
`mysql://` is not evidence for MySQL Server support.
