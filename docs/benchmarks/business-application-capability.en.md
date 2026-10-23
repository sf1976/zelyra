# Functional comparison with a real business application

**As of:** 23 October 2026

**Reference:** `/opt/siedelmann-laravel/app`

This comparison tests Zelyra's business capabilities. It is not a Laravel
port: Laravel code, architecture, and views are not implementation
requirements. The comparison focuses on workflows, data relationships,
permissions, validation, side effects, and persisted results implemented with
Zelyra's language, runtime, and database concepts.

## Initial comparison

| Business workflow | Zelyra evidence | Result |
| --- | --- | --- |
| Customers, invoices, line items, related data, and computed totals | `examples/invoice_tutorial/`; MariaDB web check in `tests/invoice-tutorial-e2e.sh` | Demonstrated by the example. This does not cover every accounting rule in the reference application. |
| Tenant-scoped CRUD and permissions | `examples/tenant_crud.zyl`; `tests/mariadb-tenant-crud-e2e.sh` | The declared CRUD and isolation scenario is part of CI. |
| Protected actions, audit entries, atomic transactions, and PDF email | `examples/typed_crud_actions/main.zyl`; `tests/mariadb-typed-crud-actions-e2e.sh` | The test checks authorization, duplicate-action protection, rollback, and the PDF attachment in the outbox. |
| Unsubscribe and select eligible campaign recipients | Reference: `/opt/siedelmann-laravel/app/app/Http/Controllers/App/CsvMailerController.php` and `tests/Feature/CsvMailerImmediateSendingTest.php`. Zelyra: `examples/csv_mailer_unsubscribe.zyl`; `tests/mariadb-form-route-e2e.sh`. | Unsubscribe exists; this change wires the E2E script into MariaDB CI. A separate attempt to select recipients across tenant-scoped tables is rejected with `E-TENANT-003`. Customers without email, unsubscribes, campaign exclusions, and tenant isolation are not yet demonstrated as one complete workflow. |
| Send Factur-X XML alongside the invoice PDF | Outbox contract in `docs/mail.en.md`; reference at `/opt/siedelmann-laravel/app/app/Services/DocumentMailService.php` | **Unsupported:** Zelyra mail attachments are currently restricted to validated PDF files. Added as a 1.7.0 acceptance item. |
| Appointment scheduling, availability rules, follow-up cases, banking, DATEV/ELSTER, and GoBD cancellation workflows | Reference controllers, services, and existing feature/smoke tests; no equivalent end-to-end Zelyra workflow yet | **Not assessed.** Each workflow must be reproduced and tested as a Zelyra application before claiming support. |

The existing CI already exercises important parts such as invoices,
tenant-scoped CRUD, and transactional PDF email. Passing type checking alone is
not a functional result. Each additional workflow needs a runnable Zelyra
example and suitable positive and negative integration tests.

### Checks performed

On source revision `5ebe066a` (codebase `1.2.55`),
`examples/invoice_tutorial/main.zyl`, `examples/tenant_crud.zyl`,
`examples/typed_crud_actions/main.zyl`,
`examples/csv_mailer_unsubscribe.zyl`,
`examples/pdf_mail_outbox/main.zyl`, and
`examples/customer_orders_modules/main.zyl` passed `zelyra check` using a CLI
binary built from that source. The two
targeted `zelyra-runtime` tests
`pdf_bytes_are_accepted_as_pure_transactional_outbox_attachments` and
`mail_queue_creates_a_validated_outbox_row_inside_one_database_batch` also
passed. The first test additionally verifies that arbitrary bytes are rejected
as a PDF attachment. MariaDB web tests run in CI; the local Laravel production
database was not accessed.

A separate compilation attempt for campaign recipient selection failed with
`E-TENANT-003` because the query reads related tenant-scoped tables through
custom SQL. The compiler rejects this form instead of trusting a tenant value
supplied by the caller. This makes the gap reproducible.

As a reference-behavior check, all three tests in
`CsvMailerImmediateSendingTest` passed (13 assertions) in the Laravel working
copy. They assert SQLite `:memory:` and use `Queue::fake()` and
`Process::fake()`; no production data, SMTP server, or external process was
accessed.

## Repeatable review process

1. Select a concrete business workflow from the reference and record its
   expected behavior, data flow, permissions, and failure cases.
2. Model the workflow with Zelyra concepts; do not reproduce Laravel layers or
   views.
3. Compile the complete Zelyra project and run it against an isolated,
   disposable database.
4. Check successful and invalid inputs, tenant boundaries, retries, rollback,
   and persisted data where relevant.
5. Update this evidence and add reproducible regressions to CI. Fix small
   capability gaps directly; record larger language, runtime, or architecture
   gaps in the roadmap with evidence and a proposed release.

The Laravel working copy may contain uncommitted changes. It is used only as a
read-only source of business requirements here. Tests against its production
database and edits to that application are outside this review process.
