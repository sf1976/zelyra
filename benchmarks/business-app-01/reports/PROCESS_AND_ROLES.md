# Mini-ERP Benchmark 01 — processes and roles

This document specifies business transitions and the initial least-privilege
role model. The route and permission declarations are implemented later in
Zelyra; a hidden button is never an authorization check.

## Purchase process

```text
Request → Draft order → Submitted → Confirmed
                                  ↓
                   Partial receipt → Complete receipt → Closed
                                  ↘ Cancelled (only if remaining quantity permits)
```

- Purchase lines snapshot supplier, article, agreed price, currency and tax.
- Receiving a partial quantity creates one receipt and matching inbound stock
  movements in a single transaction; the remaining quantity stays open.
- Duplicate submission/replay must not create duplicate receipts or movements.
- Invoice entry and purchase-order matching are a later vertical step; no
  supplier invoice is inferred from a goods receipt.

## Sales process

```text
Draft offer → Sent → Accepted → Sales order → Reserve stock
                                           → Pick/partial delivery → Invoice
Offer/order may expire or cancel only through allowed status transitions.
```

- Price and tax are resolved on the server from the effective customer/article
  rules; submitted totals are ignored and recomputed.
- Reservation and order status are changed atomically. A race for the last
  available units must have one clear winner and no negative availability.
- Partial delivery releases exactly the delivered reservation and creates
  matching outbound movements. The rest remains reserved or is explicitly
  released according to the selected workflow.
- Invoice creation is idempotent per order/delivery policy. Posted documents
  cannot be overwritten.

## Stock process

```text
Opening balance / receipt / return / transfer / adjustment
                   ↓
              Stock movement
                   ↓ same transaction
              Stock balance
```

- Transfers create a source issue and destination receipt as one transaction.
- Adjustment requires a reason and permission; the old/new quantity and actor
  are auditable.
- A cancelled posted event receives a compensating movement. A second cancel
  cannot repeat the effect.
- Reservations change availability but do not change on-hand quantity.

## Roles and permissions

| Role | Read scope | Mutations |
| --- | --- | --- |
| Administrator | All ERP records and audit/configuration | User/role provisioning, all business actions; cannot bypass posted-document reversal rules. |
| Management | All customers, suppliers, articles, orders, stock and reports | Approve exceptional discounts, adjustments, and permitted cancellations; no user security administration by default. |
| Purchasing | Suppliers, articles, purchase orders, receipts and relevant stock | Create/edit/submit purchase orders, receive against orders, request corrections; no sales invoice posting. |
| Sales | Customers, articles, offers, sales orders and delivery status | Create offers/orders, reserve and dispatch according to policy; no inventory adjustment or supplier changes. |
| Warehouse | Articles, locations, available/reserved stock, receipts and deliveries | Receive, pick, transfer, count, and submit adjustments; cannot change invoice prices or customer credit limits. |
| Accounting | Customer/vendor balances, invoices, credit notes, payments, tax and reports | Post invoices/credit notes/payments and export accounting data; no warehouse adjustments. |
| Read-only | Explicitly granted lists/details/reports | No mutation permissions. |

All roles authenticate through the shared identity module. Initial account
creation is administrator-only. Every write route rechecks its permission and
record scope. The role matrix is refined against the actual test project before
it is considered accepted.

## Required negative authorization cases

- Anonymous request to every protected page, API, form handler and CRUD action.
- Authenticated user with no permission submits a direct POST to each mutation.
- A role with access to one function attempts a different module's action.
- Read-only user attempts create, edit, delete, archive, movement, posting,
  payment and export operations.
- User changes record IDs, related customer IDs, warehouse IDs or document
  state in the request body.
- Deactivated account/session attempts to reuse an existing cookie.
- Cross-record access attempts to fetch another customer's or supplier's
  private document.

Record every denial as a safe audit/security event where the existing auth
model supports it; never log passwords, session tokens, or complete secrets.
