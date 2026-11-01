# Mini-ERP Benchmark 01 — test strategy

The benchmark is accepted by workflow evidence, not by compilation or page
appearance. Tests use disposable MariaDB databases and deterministic fixtures;
the live customer/password-generator database is never a test target.

## Layers

### 1. Zelyra language and compiler

For every new Zelyra capability:

- Positive syntax/type/effect test.
- Negative tests for missing, mismatched, unsafe, and unsupported forms with a
  stable diagnostic and source location.
- Runtime behavior and rollback/error tests.
- AST baseline and MIR/Bytecode parity wherever those backends claim support.
- Regression test demonstrating the feature works through the public Zelyra
  project command, not only a Rust helper.

### 2. Database and domain

- Apply declared schema from an empty database and compare it to a reviewed
  schema plan.
- Run a repeat setup and verify it is idempotent.
- Test all required/unique/foreign-key/check constraints and archive rules.
- Assert document totals using exact values, including half-rounding,
  multi-line tax rates, discount boundaries, zero quantities, and large values.
- Assert every inventory event updates movement and balance together.
- Inject a failure at each write boundary and verify the whole operation rolls
  back.
- Test numbering, idempotency keys, duplicate requests, and allowed/forbidden
  state transitions.

### 3. Web integration

- Exercise list, search, filters, sort, pagination, create, edit, archive,
  detail, child rows, and workflow transitions through HTTP.
- Repeat mutation attempts without CSRF, with a foreign Origin, with invalid
  types, and with stale/concurrent edit state.
- Verify values and errors are escaped, private responses are not cacheable,
  and server errors do not reveal SQL, credentials, or filesystem paths.
- Test the same one-theme shell at desktop, tablet, and phone widths, including
  keyboard focus and no horizontal overflow in high-value screens.

The first repeatable master-data HTTP slice is
`mini-erp/tests/master_data_http.py`. Run it against a disposable database
after `zelyra db setup` and after creating an administrator with article-group,
unit, and article create permissions:

```sh
ZELYRA_BASE_URL=http://127.0.0.1:4315 \
MINI_ERP_ADMIN_EMAIL=admin@example.test \
MINI_ERP_ADMIN_PASSWORD='test-only-password' \
python3 benchmarks/business-app-01/mini-erp/tests/master_data_http.py
```

It logs in through the native Zelyra auth route, creates an active group and
unit through generated forms, checks the default checkbox and readable
relation options, exercises the group search and active filter, creates an
article, archives its linked group, and confirms the archived group is hidden
from new relations while the article remains visible. The script is an HTTP
test driver only; application behavior remains in Zelyra. It does not yet
replace the broader CRUD, authorization, or concurrency matrix above.

### 4. Security and permissions

- Run the negative authorization matrix in
  [PROCESS_AND_ROLES.md](PROCESS_AND_ROLES.md) against direct requests, not
  only visible controls.
- Check SQL parameter binding, CSRF/origin checks, XSS escaping, session
  invalidation, account deactivation, and object ownership.
- Verify all posted stock, order, receipt, invoice, credit-note, and payment
  changes have an attributable audit trail.
- Exercise upload allowlists, size bounds, private download routes, and
  cross-record access denial for images/documents.

### 5. Concurrency and integrity

Use multiple independent MariaDB connections/processes and synchronization
barriers, not sequential simulation:

- Two issues compete for the final available quantity.
- Two orders reserve the same balance while a receipt is committed.
- A transfer and an outbound delivery contend on the same stock location.
- Two workers allocate the next order/invoice number.
- Two users edit the same draft; stale input must not overwrite the winning
  update.
- A request retries after a network timeout and must not duplicate movements,
  receipts, deliveries, or invoices.

For every case assert final rows, movement totals, reserved/on-hand balances,
audit count, and number uniqueness. Require no negative available stock and no
partially posted business document.

### 6. Scale and benchmark

Seed at least 10,000 customers, 50,000 articles, 100,000 orders, 500,000 order
lines, and 1,000,000 stock movements. Test 10, 50, 100, and 250 concurrent
virtual users. Record hardware, MariaDB version/configuration, fixture seed,
median/P95/P99 latency, throughput, error rate, and memory/CPU. Measure list,
search, report, booking, and invoice workflows separately. Use indexes and
query improvements; never remove required behavior to improve a score.

## Release gates

| Gate | Acceptance |
| --- | --- |
| Phase 1 master data | CRUD/security/search/schema and opening-stock invariants pass on MariaDB; core rollback and concurrency tests pass. |
| Purchasing | Draft/submit/partial receipt/cancel path works end to end; inventory movements and audit are transactionally consistent. |
| Sales | Offer/order/reservation/partial delivery path works; contention cannot oversell; document totals match exact expected values. |
| Invoicing | Posting, credit, payment, immutable state, PDF output, numbering and rounding pass with multi-page data. |
| Full benchmark | All mandatory flows and security checks pass; full repository suite and supported-database matrix pass; scale results are published with limitations. |

An open critical security, data-integrity, or concurrency failure blocks its
workflow gate. An unsupported feature is documented in `LANGUAGE_GAPS.md` and
must not be hidden by a benchmark-only workaround.
