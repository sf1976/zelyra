# Zelyra 0.4.0 security and threat review

**Status:** internal source and threat review for the experimental 0.4.0
release. It is not an external audit, penetration test, legal license opinion,
production approval, or human onboarding acceptance. Human onboarding is
explicitly deferred to 0.5.0 in the owner decision records.

## HTTP and API boundaries

- Health-check categories, the 30-second connection deadline, and safe retry
  boundaries are documented in the [HTTP operations guide](http-operations.en.md).
  Listener reachability is a TCP check; readiness is application-owned. A
  reserved, database-free liveness route confirms request dispatch. Zelyra does
  not retry requests or transactions.
- Zelyra's built-in HTTP server does not terminate TLS. Public or LAN-facing
  deployments need a trusted TLS proxy, and the application port must be
  protected from direct access.
- API quotas use the TCP peer IP. `X-Forwarded-For` and similar proxy headers
  are not accepted as client identity. Behind a proxy, requests are therefore
  initially counted under the proxy's peer IP.
- API and login limiters live in process memory. A restart clears their state;
  separate instances do not coordinate. API buckets are capped at 4096 and
  fail closed for new keys when full.
- Login buckets are also capped at 4096 keys, clean expired windows, and fail
  closed for new keys when full. An attacker can therefore temporarily block
  legitimate new login keys until cleanup; this does not provide distributed
  protection. The normalized email is stored as a Blake2s key rather than in
  clear text in limiter state.
- Request IDs are limited to 128 ASCII letters, digits, periods, underscores,
  and hyphens. Invalid values are replaced. An ID is correlation metadata, not
  authentication and not a secret.
- Exhausted rate limits return `429` and `Retry-After`. They are not DDoS
  protection and do not replace edge limits or persistent controls.
- HTTP connections have a fixed 30-second read/write deadline. MariaDB pool
  waits and statements use the remaining request deadline, and timed-out
  statements are aborted server-side. CPU-bound and other blocking handler
  work remains synchronous and can occupy a worker; the server bounds this to
  64 concurrent connections and drops excess connections. Client disconnect
  is only detected when writing the response.

## Accounts and database operations

- Login failure limits are configurable per auth definition and process-local.
  Password recovery is implemented for the documented experimental MariaDB
  profile: MariaDB stores Blake2s hashes of random single-use tokens with a
  15-minute expiry; generic responses, CSRF/origin checks, audit, and
  persistent/in-memory session revocation are present. A database-backed outbox
  stores the email/link payload authenticated and encrypted with AES-256-GCM
  under `ZELYRA_RESET_DELIVERY_KEY`. The worker retries after SMTP failure and
  process restart. Under the per-user advisory lock it rechecks that a payload
  remains pending and due before sending; malformed or undecryptable rows are
  deferred for 30 seconds. A focused unit regression and two-instance
  MariaDB/SMTP E2E cover these guards. The E2E delays SMTP beyond its timeout to
  exercise a competing worker's stale selection.
- Reset delivery is at least once: a crash after SMTP accepts a message but
  before the database acknowledgement can duplicate email. Losing the key
  strands pending messages. Key rotation is safe only after the pending outbox
  is empty; keep the old key until then. Database timing can vary. The MariaDB/
  SMTP E2E covers enumeration, delayed delivery, token hashing, replay, expiry,
  CSRF/origin, session revocation, concurrent redemption, SMTP outage, and
  restart recovery. A frozen-clock test exercises the production token predicate
  before, at, and after expiry; equality is rejected. Token replacement and
  outbox insertion are serialized by MariaDB advisory locks, and the E2E checks
  that the last delivered message matches the active token.
- Login, reset, and API quotas are intentionally process-local in this release,
  reset on restart, and do not provide distributed abuse protection. The
  documented limits and trusted-proxy requirements remain part of the release
  boundary.
- Session administration and self-service revocation are implemented for
  persistent and in-memory sessions. Session lists can show a
  bounded, HTML-escaped `User-Agent` label when the persistent session schema
  declares `device_label`; in-memory sessions capture the same label. The value
  is client-controlled and is not a verified device identity. IP addresses
  are not captured. This metadata does not establish device identity.
- MariaDB TLS verifies the certificate chain and host names according to the
  documented mode. The Linux CI matrix covers MariaDB 10.11.19, 11.4.13,
  11.8.9, and 12.3.3. Windows artifacts compile, but a Windows-to-MariaDB TLS
  handshake is not separately claimed. TLS terminates at the database server;
  app HTTP is a separate connection.
- PostgreSQL and SQLite schema apply now use transactions; tests show a failed
  DDL step rolls back earlier steps. MariaDB DDL can still leave partial state.
  Read-only preflights block a plan before SQL when new unique indexes have
  duplicate values or new foreign keys have orphan values. Concurrent writes
  can race these checks, so they are not an isolation guarantee.
  All schema backends record migration plans and outcomes; MariaDB additionally
  records step checkpoints. CI exercises interruption between DDL statements,
  interruption while DDL waits on a metadata lock, recovery with a fresh plan,
  and backup/restore. A crash during active MariaDB DDL can leave ambiguous
  partial state; no automatic or data-safe rollback is claimed. Reverse DDL
  remains explicit and requires a fresh reviewed plan and verified backup.

## Supply chain and remaining gates

- Linux and Windows release builds generate SPDX-2.3 SBOMs from the resolved
  Cargo graph and verify the binary SHA-256. Cargo license fields that cannot
  be represented as SPDX are reported as `NOASSERTION` with the original value.
- CI runs pinned `cargo-deny` against Linux and Windows dependency graphs for
  RustSec advisories, SPDX licenses, duplicate versions, wildcard requirements,
  and registry/git sources. The policy clarifies `webpki` as ISC based on a
  hash-verified license file, and records a reasoned exception for the
  currently resolved, unmaintained `rustls-pemfile` compatibility wrapper
  required by `mysql` 28.0.3. Duplicate `base64` and `getrandom` versions
  still warn.
- The automated dependency and license policy passes in CI. `NOASSERTION`
  licenses and duplicate-version warnings remain visible; this is not a legal
  opinion. Release-tag builds are configured to attest archives and SBOMs, but
  0.4.0 candidate provenance and published-artifact verification are not
  complete until the candidate release is built and checked.
- Granular effects and complete module impact/edit coverage remain partial P1
  work. The 30-second network/database deadline does not interrupt arbitrary
  CPU-bound handler code. Neither limitation is advertised as complete.

## Release decision

This review records the tested boundaries and remaining risks for the
experimental release. It makes no external-audit or production-readiness claim.
The release plan and [risk register](release-readiness/0.4.0-risk-register.en.md)
remain authoritative for candidate-specific CI, artifact, checksum, SBOM,
attestation, and installation/upgrade verification.
