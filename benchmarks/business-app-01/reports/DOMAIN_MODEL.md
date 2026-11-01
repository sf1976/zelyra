# Mini-ERP Benchmark 01 — domain model

The model describes the initial single-company benchmark. It is a domain
specification, not yet a committed Zelyra schema. The application will use
declared Zelyra tables and relations; these concepts do not imply an ORM.

## Master data

| Entity | Key and important fields | Relations / rules |
| --- | --- | --- |
| Customer | Internal ID, unique customer number, company/person name, contacts, billing and shipping addresses, payment terms, credit limit, active/archive state, notes | May have multiple contacts and addresses; historical orders and invoices remain readable after archive. |
| Supplier | Internal ID, unique supplier number, legal name, contacts, address, payment/delivery terms, lead time, active/archive state | Supplier articles and purchase orders retain supplier snapshots for audit. |
| Article group | Internal ID, unique name/code, description, active state | Groups articles; archived groups cannot be selected for new articles. |
| Unit | Internal ID, unique code and label, quantity precision | Precision must be explicit before fractional stock is accepted. |
| Article | Internal ID, unique article number, name/description, group, unit, purchase/sales price, tax category, reorder threshold, barcode, optional image, active/archive state | Referenced by supplier offers, order lines, stock movements and invoice lines. Prices in documents are snapshots, not live joins. |
| Supplier article | Supplier + article, supplier article number, purchase price, currency, minimum order, lead time, validity dates | Unique by supplier/article/valid-from policy; preserve historical prices. |
| Warehouse | Internal ID, unique code, name, address, active state | Owns locations and stock balances. |
| Warehouse location | Internal ID, warehouse, unique location code, description, active state | An operation must use a location belonging to the selected warehouse. |

## Inventory and purchasing

| Entity | Important fields | Invariants |
| --- | --- | --- |
| Stock balance | Article, warehouse/location, on-hand quantity, reserved quantity, version or locking identity | One balance per article/location. `available = on_hand - reserved`; never below zero unless an explicitly approved adjustment policy allows it. Treat a cached balance as transactionally maintained and reconcile it against movements. |
| Stock movement | Article, source/destination location as applicable, signed quantity, kind, source document/line, actor, timestamp, reversal reference, note | Append-only. Corrections and cancellations create compensating movements; they never erase posted movements. Every balance change has a matching movement in the same transaction. |
| Purchase order | Supplier, unique number, status, dates, terms and immutable totals/snapshots when submitted | Draft → submitted/confirmed → partially received → received/closed, or cancelled. Invalid transitions fail server-side. |
| Purchase order line | Order, article, ordered/received quantities, unit price/currency, discount, tax snapshot | Receipt quantity cannot exceed remaining quantity unless an explicit over-delivery policy is configured. |
| Goods receipt | Purchase order, received date, actor and status | Each posted line creates exactly one linked inbound movement. Reversal is an explicit compensating receipt/correction. |

## Sales and accounting documents

| Entity | Important fields | Invariants |
| --- | --- | --- |
| Offer | Customer, unique number, validity, status, totals and address/term snapshots | Draft offers can change; accepted or expired status follows explicit transition rules. |
| Sales order | Customer, unique number, status, dates, payment/shipping snapshots, totals | A submitted order preserves the prices, descriptions, addresses and tax category used at submission. |
| Sales order line | Order, article, ordered/reserved/delivered quantity, unit price, discount, tax snapshot | Server recalculates totals; submitted client totals are never trusted. |
| Reservation | Order line, stock balance/location, quantity, status, timestamp | Created and released atomically with the order transition; cannot exceed available stock. |
| Delivery | Order, status, date, carrier/reference | Posted delivery creates linked outbound movement(s); partial shipments leave remaining quantities explicit. |
| Invoice | Customer/order reference, unique number, issue/due dates, status, currency, totals, immutable billing snapshots | Draft → posted → paid/part-paid/overdue; posted invoices cannot be edited in place. Corrections use credit/reversal documents. |
| Invoice line | Invoice, article snapshot, quantity, unit price, discounts, tax category/rate snapshot, exact calculated amounts | Tax and rounding rules are defined at the operation boundary and repeatable from stored inputs. |
| Credit note | Invoice reference, unique number, reason, status, lines and totals | Does not mutate the original posted invoice; reverses the appropriate receivable and, if policy requires, stock movement. |
| Payment | Invoice, amount, date, payment method/reference, actor | Amount is exact and cannot exceed the open balance without an explicit overpayment workflow. |
| Audit entry | Actor, entity/action, record identity, timestamp, safe before/after summary | Insert in the same transaction as the business mutation where supported; never include credentials or secrets. |

## Value and lifecycle rules

- Use exact base-10 monetary values. Define currency (initially EUR), scale,
  rounding mode, tax rate precision, and whether rounding occurs per line or at
  document total before implementing invoices.
- Quantities are positive exact decimals with unit-specific scale; do not use
  binary floating point for quantities that affect financial or stock totals.
- Human-readable numbers use the existing atomic number-series facility and
  remain unique under concurrency.
- Draft documents may be edited. Posted stock movements and invoices are
  immutable; later errors are repaired with explicit reversal/credit workflows.
- Historical document lines snapshot names, numbers, addresses, unit prices,
  discount, and tax metadata. Updating master data cannot rewrite history.
- IDs and foreign keys are database-enforced. Business status transitions and
  stock invariants are also checked by server-side Zelyra actions.

## Minimum initial schema sequence

1. Users/roles/permissions/audit, customers, suppliers, article groups, units,
   articles, warehouses, and locations.
2. Stock balance and append-only stock movement.
3. Purchase orders, lines, and receipts.
4. Offers, sales orders, lines, reservations, and deliveries.
5. Invoices, credit notes, payments, and document delivery records.

Do not generate a schema for all later phases before the earlier constraints
and migration behavior have been exercised against MariaDB.
