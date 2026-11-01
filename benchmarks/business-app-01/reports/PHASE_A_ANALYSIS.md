# Mini-ERP Benchmark 01 — Phase A analysis

Date: 2026-10-24  
Repository baseline: Zelyra `1.3.0` line, feature branch `feature/standalone-native-form-handlers`  
Status: **Phase A analysis documented; master-data slice implementation is underway**

## Executive status

The password-generator prerequisite is running as a native Zelyra application.
An initial native Mini-ERP slice now declares authentication, customer,
supplier, article, article-group, unit, warehouse, and warehouse-location
models with generated CRUD routes. This is a master-data foundation, not yet
an integrated stock, purchasing, sales, or invoicing system. The repository
also contains smaller, independent examples for customer/order CRUD, relational
master-detail forms, exact decimal calculations, inventory/order workflows,
authorization, dashboards, and PDF output. Those examples prove individual
capabilities; they do not prove the complete connected workflow requested by
Benchmark 01.

Six concrete Zelyra gaps have been found and fixed on the current feature
branch:

1. Standalone validated forms could not invoke a typed Zelyra function. A native
   typed handler path was added and tested.
2. Standalone form actions could not use the existing authentication and
   permission declarations. The CLI rejected authorization metadata that the
   runtime already knew how to enforce. The validation path now accepts and
   checks it before executing the handler.
3. Module-qualified PascalCase record names did not resolve to conventional
   snake_case table names. Shared database name normalization now maps types
   such as `ArticleGroup` to `article_groups` and produces the expected
   relation columns and foreign keys.
4. Generated CRUD create forms did not show declared table defaults. This made
   `Bool default true` appear unchecked and could create inactive records.
   Literal defaults now initialize the form while temporal database
   expressions remain server-side.
5. Web relation pickers had a separate lowercase-only type resolver and did
   not recognize common display columns `label` and `code`. The web path now
   shares record-name normalization and displays human-readable options.
6. Relation pickers included soft-deleted records. Generated lookups now omit
   archived rows while historical records remain accessible.

These were real language/toolchain integration gaps, not ERP business-domain
gaps. They are tracked in [LANGUAGE_GAPS.md](LANGUAGE_GAPS.md). The workspace
test suite and a repeatable HTTP regression for defaults, master-data search
and filters, creation, article relations, archive exclusion, and historical
visibility pass against disposable MariaDB. Publication of the standalone
form changes is still subject to PR #257 and its CI.

## Ist-Zustand and evidence

The [capability matrix](CAPABILITY_MATRIX.md) records implemented, partial,
known-limited, and unassessed areas with repository evidence. The main reusable
starting point is `examples/customer_orders_modules`; it has MariaDB-backed
customer/order CRUD, authentication, permissions, and a typed aggregate view.
`examples/invoice_tutorial` proves related invoice lines, decimal calculations,
and a print view. `examples/inventory_orders` is a console/in-memory teaching
example: it is not an ERP inventory implementation and does not prove
database transactions or concurrent stock booking.

Phase A artefacts are being recorded in this report set:

- [Capability matrix](CAPABILITY_MATRIX.md)
- [Domain model](DOMAIN_MODEL.md)
- [Process and role model](PROCESS_AND_ROLES.md)
- [Unified UI plan](UI_PLAN.md), using the current password-generator overview
  as the visual reference
- [Test strategy](TEST_STRATEGY.md)

## Architecture decisions for the benchmark

- Keep the application as ordinary Zelyra source in a standalone benchmark
  project. Do not make an ERP-only compiler feature or move business logic to
  PHP, JavaScript, Python, or Rust.
- Use MariaDB as the reference database, with declared Zelyra schema and
  reviewed migrations. Retain direct typed SQL where relational queries are
  clearer than generated CRUD.
- Use exact `Decimal`/`Money` values for prices and totals. Define rounding at
  the business-operation boundary; do not use `Float` for money.
- Record every stock change as an immutable movement. Derive or transactionally
  maintain the on-hand and reserved balances. Never make a stock adjustment by
  silently overwriting a quantity.
- Keep the first benchmark single-company. Model warehouses and locations, but
  do not add multi-tenancy unless an ERP requirement demonstrates it. Tenant
  restrictions on custom SQL remain a separate known language boundary.
- Use generated CRUD for conventional master-data screens, and extend them with
  native Zelyra views/actions where needed. Do not equate generated CRUD with
  process completeness.
- Use existing authentication, session, permission, audit, transaction,
  number-series, file-reference, and UI facilities where their documented
  contracts meet the requirement. Verify each in the integrated app.
- Treat native text-PDF output and browser-print HTML as distinct options. The
  PDF requirements are not accepted until page breaks, repeated headings,
  totals, and downloaded output are proven on real data.

## Risks and known constraints

1. **Atomic inventory:** no integrated database-backed stock reservation and
   two-user concurrency proof exists yet. This is the first high-risk feature
   gate after master data.
2. **Dynamic order lines:** generic master-detail CRUD exists, but order entry
   with client-side totals, server-side recalculation, stock checks, and
   transactional writes has not been proven as one workflow.
3. **PDF quality:** Zelyra has a bounded text-PDF renderer and browser-print
   views. Whether these satisfy a professional multi-page invoice is unproven.
4. **Financial rules:** exact decimals are available, but tax-rate/version
   handling, per-line rounding, credit notes, and posted-document immutability
   need explicit domain rules and tests.
5. **Scope:** the requested system is much larger than a CRUD demo. Build it in
   vertical modules and report every deferred requirement; do not mark the
   benchmark complete from an early usable slice.

## Prioritized implementation order

1. Complete Phase A artefacts: capability matrix, domain model, process/role
   model, UI plan, and acceptance test strategy.
2. Build and test customers, suppliers, article groups, units, articles,
   warehouses, and locations using MariaDB and native Zelyra CRUD.
3. Add immutable stock movements and atomic opening balance/receipt/issue
   operations. Stop feature work if concurrency testing exposes a language or
   database bridge gap.
4. Add purchase orders and partial receipts.
5. Add offers/orders, reservations, deliveries, invoices, credit notes, and
   exact totals in vertical slices.
6. Add authorization, audit, dashboard/reporting, PDF, realistic-scale tests,
   and final benchmark measurements.

## Phase 1 acceptance tests

Phase 1 means the first reusable core after analysis: typed item/customer data
and safe inventory booking primitives. It is accepted only when all of these
pass against a disposable MariaDB instance:

- Create/read/update/archive customer, supplier, article group, unit, article,
  warehouse, and location records through the Zelyra application.
- Enforce required, unique, bounded, and typed values in both the UI and server
  routes; direct requests cannot bypass them.
- Search, filter, sort, and paginate seeded records without exposing raw
  database errors.
- Verify every declared foreign key and reject references to nonexistent or
  archived master data where the operation requires an active record.
- Create a stock opening movement and check the computed on-hand balance.
- Reject zero/negative or excessive quantities, invalid warehouse/location
  combinations, and any issue greater than available stock.
- Run two simultaneous issues against one balance; exactly the permitted
  quantity may commit, with no lost update or negative availability.
- Inject a failure after a movement insert but before balance completion; the
  entire operation must roll back.
- Verify AST and Bytecode observable results are identical where both backends
  support the exercised feature; otherwise document the supported backend and
  fail closed.
- Run the complete existing workspace suite and MariaDB compatibility jobs.

No claim of ERP completeness follows from passing this first phase.
