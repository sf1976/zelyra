# Zelyra 0.4.0 security review — working status

**Status:** technical inventory, not an external audit or release approval.
This review covers the 0.4 development branch at this commit. It records
implemented boundaries and known gaps; it does not replace penetration testing
or human acceptance.

## HTTP and API boundaries

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
  but does not cancel synchronous handlers; a blocked handler can still occupy
  the single server process.

## Accounts and database operations

- Login failure limits are configurable per auth definition and process-local.
  Password recovery, reset tokens, and reset limits are not implemented; there
  is no secure email or token delivery path for them yet.
- Session administration is partially implemented. Device metadata and full
  self-service remain open; account-lifecycle acceptance has not passed.
- MariaDB TLS verifies the certificate chain and host names according to the
  documented mode. TLS still terminates at the database server; app HTTP is a
  separate connection. Windows TLS still needs its own matrix evidence.
- Versioned reversible schema migrations, drift/rollback evidence, and recovery
  after interruption are incomplete. Existing backups remain an operator
  responsibility.

## Supply chain and remaining gates

- Linux and Windows release builds generate SPDX-2.3 SBOMs from the resolved
  Cargo graph and verify the binary SHA-256. Cargo license fields that cannot
  be represented as SPDX are reported as `NOASSERTION` with the original value.
- The SBOM is not a license audit. A complete dependency/license audit, signed
  provenance, and review of every native artifact remain open.
- Granular effects, complete HTTP response deadlines, independent security
  review, and human acceptance remain open.

## Release decision

This review closes no P0 gate. A final `v0.4.0` remains blocked until the
release plan and [risk register](release-readiness/0.4.0-risk-register.en.md)
have evidence for the same candidate.
