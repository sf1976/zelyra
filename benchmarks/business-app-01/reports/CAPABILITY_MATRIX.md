# Mini-ERP Benchmark 01 — Zelyra capability matrix

Evidence date: 2026-10-24.  
This matrix distinguishes tested Zelyra support from an untested ERP workflow.
“Available” does not mean the complete business operation has been proven.

| Capability | Status | Repository evidence | ERP assessment / next proof |
| --- | --- | --- | --- |
| Typed tables, scalar types, nullable/required fields, defaults and keys | Available | `examples/invoice_tutorial/src/invoices.zyl`; schema planning and database matrix tests | Use for all declared master and transaction tables; test live MariaDB migration and constraints. |
| Foreign-key relations between tables | Available | Invoice/customer/item relation declarations; `tests/mariadb-tenant-relations-e2e.sh` | Use declared relations; validate active status and warehouse/location consistency in business actions. |
| Generated CRUD create/read/update/delete, search and filter | Available | `examples/customer_orders_modules/src/customers.zyl`; CRUD integration tests | Suitable for basic master data. ERP archive semantics and protected business states need custom policies/actions. |
| Soft-delete/archive lifecycle | Available, policy-driven | `examples/crud_lifecycle_policies/main.zyl`; MariaDB lifecycle E2E | Prefer archive for customers, suppliers, and articles referenced by history. |
| Master-detail child collections in CRUD forms | Available for generic CRUD | `tests/mariadb-master-detail-crud-e2e.sh`; `tests/mariadb-multiple-crud-children-e2e.sh` | Proves child rows can be edited; does not prove calculated order totals or stock reservation. |
| Typed SQL, joins, aggregates, grouping, sorting and pagination | Available | `examples/customer_orders_modules/src/reporting.zyl`; typed page/tableview tests | Use SQL for reports and read models. Tenant-scoped custom multi-table SQL is a known compiler restriction; the initial ERP is single-company. |
| MariaDB schema setup, inspection, planning and reviewed apply | Available | CLI database plan/apply tests and the live nullable profile-column migration | All ERP schema changes use an inspected plan; production application is outside disposable test DBs. |
| Exact `Decimal` and `Money` representation | Available primitives; domain rules partial | `examples/decimal_money.zyl`; invoice tutorial; exact-decimal runtime/database tests | Use exact values only. Define scale, tax calculation, line/document rounding and posted-value immutability before invoicing. |
| Database transaction blocks | Available on documented MariaDB/MySQL paths; backend parity is bounded | Handbook transaction section; transaction and PDF-outbox examples | Verify rollback and concurrent writes in each stock and order operation. Do not infer concurrency safety from syntax alone. |
| Atomic number-series allocation | Available | Number-series allocator tests and tenant-aware sequence examples in `docs/ROADMAP.md` | Use for human-readable order and invoice numbers; include concurrent allocation test in ERP E2E. |
| Server-side auth, sessions, roles and named permissions | Available | `examples/customer_orders_modules/src/auth.zyl`; MariaDB auth E2E; native self-service account routes | Reuse shared auth. ERP role and object-level policies must be declared and checked on direct requests. |
| Row-level / tenant authorization | Available for defined tenant patterns; custom SQL limitations remain | `examples/tenant_crud.zyl`, tenant CRUD/API E2Es; known `E-TENANT-003` boundary in roadmap | Benchmark itself starts single-company; still test ownership/record access by roles. Revisit tenant SQL before multi-company scope. |
| Audit history and hash-chain options | Available | Auth/CRUD audit examples and audit CLI tests | Record stock, order, receipt, delivery, invoice, credit-note, and permission changes transactionally. |
| File references and controlled PDF/image uploads | Available with declared storage policy | `examples/file_ref_crud.zyl`; upload integration tests | Use for article images and documents only after MIME/size/ownership rules are declared. Factur-X XML is a separate open capability. |
| PDF rendering | Partial for professional ERP documents | `render_pdf` and `examples/pdf_mail_outbox/main.zyl`; invoice tutorial HTML print route | Built-in renderer is a bounded text PDF. Professional page layouts, repeated headers, and page-break correctness are not yet demonstrated. |
| Native UI shell, forms, tables, dialogs and responsive CSS | Available | `zelyra/ui`, invoice/customer examples, UI regression tests | Build an integrated shell and customize generated CRUD using Zelyra views/CSS. Accessibility and large-table behavior still need benchmark tests. |
| Dashboard/report aggregate views | Available | `examples/customer_orders_modules/src/reporting.zyl`; invoice dashboard example | Implement KPIs with typed SQL and bound date/filter inputs; verify aggregates against fixture totals. |
| Background jobs and mail outbox | Available with explicit capabilities | Scheduler and transactional outbox examples/tests | Use only for reorder alerts or document delivery if required; retries/idempotency need ERP tests. |
| Reusable local Zelyra packages | Experimental, usable in-repository | `examples/customer_orders_modules`; password-generator package graph | Use project-local modules where helpful; do not promise a stable external ABI or published package. |
| AST / MIR / Bytecode behavior for ERP operations | Partial; per-feature proof required | Existing backend coverage and interpreter/VM tests; workspace is AST-default | Keep AST as the baseline. Add parity tests for each new operation supported by multiple backends. |
| ERP-scale performance (10k–1m records, 10–250 users) | Not assessed | No Mini-ERP benchmark dataset or load report exists | Seed representative data, measure median/P95/P99, error rate, and throughput before completion. |
| Complete purchase/sales/inventory workflow | Not assessed | Only independent customer/order and in-memory inventory examples exist | Build connected vertical workflows and keep the benchmark open until all required scenarios pass. |

## Known limitations that are not silently counted as ERP failures

- Tenant-scoped custom SQL across multiple tables is deliberately rejected by
  `E-TENANT-003`; the documented fix requires a compiler-enforced safe query
  path. The requested first Mini-ERP is single-company, so this is tracked as a
  separate 1.7.0 business-workflow item unless scope changes.
- The mail outbox supports PDF attachments, but typed Factur-X XML attachment
  support is a separate 1.7.0 item and is not required for the initial Mini-ERP
  acceptance list.
- “Not assessed” means no complete integrated Zelyra attempt has been made. It
  must not be rewritten as either support or a language defect without a
  reproducible test.
