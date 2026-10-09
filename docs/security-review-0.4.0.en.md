# Zelyra 0.4.0 security review — working status

**Status:** technical inventory, not an external audit or release approval.
This review covers the 0.4 development branch at this commit. It records
implemented boundaries and known gaps; it does not replace penetration testing
or human acceptance.

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
- HTTP connections have a fixed 30-second deadline. It covers reads and writes
  MariaDB pool waits and statements use the remaining request deadline, and
  timed-out statements are aborted server-side. CPU-bound and other blocking
  handler work remains synchronous and can still occupy the single server
  process; client disconnect is only detected when writing the response.

## Accounts and database operations

- Login failure limits are configurable per auth definition and process-local.
  Password recovery is partially implemented in the 0.4 branch: MariaDB stores
  Blake2s hashes of random single-use tokens with a 15-minute expiry; generic
  responses, CSRF/origin checks, audit, and persistent/in-memory session
  revocation are present. A database-backed outbox stores the email/link payload
  authenticated and encrypted with AES-256-GCM under the required
  `ZELYRA_RESET_DELIVERY_KEY`. The worker retries after SMTP failure and process
  restart; the MariaDB E2E confirms encrypted-at-rest storage and recovery after
  restarting the sole active instance. Delivery is at least once, so a crash
  after SMTP acceptance but before database acknowledgement can duplicate an
  email. Losing the key prevents pending messages from being decrypted. Limits
  are process-local and not shared between instances; database access timing
  can still vary. A MariaDB/SMTP-sink E2E checks enumeration, delayed delivery,
  token hashing, replay, expiry, CSRF/origin, and session revocation. The
  E2E now also submits the same reset token concurrently and verifies that one
  request succeeds, the other is rejected, and only the winning password can
  authenticate. Token replacement and outbox insertion are serialized by a
  MariaDB advisory lock; the worker reads pending rows ordered by ID, and the
  E2E verifies the last delivered message matches the
  active token. An E2E sets `expires_at = NOW()` and verifies rejection. A
  frozen-clock test exercises the production lookup predicate before, exactly at,
  and after expiry; equality is rejected. SMTP outage and restart recovery now
  have E2E coverage; the mail/token lifecycle, key-rotation procedure, and
  broader operational/threat review still need review.
- Session administration is partially implemented. Session lists can show a
  bounded, HTML-escaped `User-Agent` label when the persistent session schema
  declares `device_label`; in-memory sessions capture the same label. The value
  is client-controlled and is not a verified device identity. IP addresses
  are not captured. Account-lifecycle acceptance has not passed.
- MariaDB TLS verifies the certificate chain and host names according to the
  documented mode. TLS still terminates at the database server; app HTTP is a
  separate connection. Windows TLS still needs its own matrix evidence.
- PostgreSQL and SQLite schema apply now use transactions; tests show a failed
  DDL step rolls back earlier steps. MariaDB DDL can still leave partial state.
  Read-only preflights block a plan before SQL when new unique indexes have
  duplicate values or new foreign keys have orphan values. Concurrent writes
  can race these checks, so they are not an isolation guarantee.
  Persistent migration history, safe inverse SQL, actual process-interruption
  recovery, and a full restore rehearsal remain open. Existing backups remain
  an operator responsibility.

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
- The SBOM and automated checks are not a human license audit. A review of
  dependency purpose, license obligations, the remaining warnings, signed
  provenance, and every native artifact remains open.
- Granular effects, complete HTTP response deadlines, independent security
  review, and human acceptance remain open.

## Release decision

This review closes no P0 gate. A final `v0.4.0` remains blocked until the
release plan and [risk register](release-readiness/0.4.0-risk-register.en.md)
have evidence for the same candidate.
