# Local MariaDB outbox (experimental)

The 0.8 outbox stores application events in the same MariaDB transaction as
business data. It is an experimental database feature; it is not a complete
queue service.

## Set up and inspect

Use a MariaDB project with its database URL configured, then explicitly create
the internal table:

```sh
zelyra outbox setup main.zyl
```

List up to 200 rows of operator metadata:

```sh
zelyra outbox list main.zyl --limit 50 --format text
zelyra outbox list main.zyl --limit 50 --format json
```

The listing omits event payloads. It reports event ID and type, status, attempt
count, timestamps, and fixed redacted error codes. The limit defaults to 50 and
must be between 1 and 200.

An exhausted event can be explicitly released after operator review:

```sh
zelyra outbox requeue <event-id> main.zyl
```

Requeueing resets the attempt count and makes the event available; it does not
deliver the event. Keep payload access inside the application that owns the
data.

## Handler API boundary

The Rust database API can register handlers by event type and process at most
one event per explicit call. The CLI can map public Zelyra functions to event
types explicitly:

```sh
zelyra outbox run main.zyl \
  --handler orders.created=handle_order_created \
  --handler invoices.issued=handle_invoice_issued \
  --lease-seconds 300
```

Each mapped function must have the signature
`pub fn name(event_id: String, event_type: String, payload_json: String) -> Bool`.
Returning `true` acknowledges the event; returning `false` or raising a
runtime error schedules a retry. The CLI checks project capability grants and
runtime policies. By default it drains currently due events and exits when no
mapped event is due. Add `--once` to process at most one event. The default
lease lasts 300 seconds; `--lease-seconds` accepts 5 through 3600. CLI handlers
cannot renew leases themselves, so long-running handlers can be delivered
again after lease expiry. Zelyra source has no handler declaration syntax and
the web application runtime does not start the dispatcher. No worker starts
automatically with the web server.

Delivery is at least once. A process can fail after a handler performs its
side effect but before the acknowledgement is stored. Handlers must therefore
use the stable event ID for idempotency. A handler can renew its own live
lease; an expired or stale owner cannot renew or acknowledge it.
