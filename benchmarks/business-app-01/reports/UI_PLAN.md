# Mini-ERP Benchmark 01 — unified UI and design plan

## One design system

The application has one visual design system and one authenticated AppShell.
The existing password-generator overview is the visual reference: its Zelyra
AppShell, light work surface, card layout, typography, and blue/green palette
form the baseline for the ERP.
Every module—master data, inventory, purchasing, sales, invoices, dashboard,
and account management—uses the same Zelyra UI components, theme tokens,
typography, spacing scale, status colors, and interaction states.

There will be no second theme, alternative per-module shell, or duplicate
component library. A screen may arrange shared components differently to fit
its task, but it must not introduce a separate visual language. Project CSS
overrides are allowed only as part of this single theme and must continue to
use the documented Zelyra tokens.

## Application shell

- One shell with Zelyra brand, global navigation, current location/breadcrumb,
  account/profile access, and a responsive main region.
- The overview route is the only dashboard. It contains the application/module
  entry points and useful operational KPIs; there is no separate duplicate
  dashboard page for each module.
- Module entry points may use the same card/list patterns on the overview.
  Navigation does not repeat the same app links in a second menu unless the
  shared navigation contract requires it for orientation.
- No login or registration app tile. Authentication is account infrastructure;
  public registration stays disabled for the benchmark.

## Shared screen patterns

1. **Resource list:** page heading, short purpose text, search, typed filters,
   sort, pagination, empty/loading/error states, and permission-aware actions.
2. **Create/edit form:** consistent field labels, validation summary, inline
   errors, required/optional indicators, save/cancel actions, and confirmation
   for destructive or irreversible operations.
3. **Detail page:** identity/status header, summary cards, tabs or sections,
   related child tables, history, and permission-aware actions.
4. **Workflow screen:** explicit status, allowed next actions, a review of
   calculated totals, confirmation, and a clear immutable/posted state.
5. **Data table:** accessible headings, numeric alignment, stable row actions,
   responsive overflow strategy, and keyboard-accessible selection.
6. **Dialog/notification:** consistent accessible modal, focus handling,
   success/error messages, and no reliance on color alone.

## Modules and route groups

- `/` — the single overview/dashboard and module launch surface.
- `/customers`, `/suppliers`, `/articles`, `/article-groups`, `/units` — master
  data lists, forms, and detail pages.
- `/warehouses`, `/locations`, `/stock` — inventory overview, movement history,
  and adjustment/transfer workflows.
- `/purchases` and `/sales` — document lists and explicit workflow pages.
- `/invoices`, `/credit-notes`, `/payments` — accounting views and posted
  document history.
- Account/profile and session pages remain common identity infrastructure.

Exact paths are reserved only after route collisions are checked against the
actual Zelyra routing model.

## Responsive and accessibility requirements

- At desktop widths, use a persistent navigation rail and dense tables where
  useful. At narrow widths, collapse navigation and present table data as
  scrollable or stacked records without hiding key actions.
- Keep interactive controls usable by keyboard and touch. Preserve visible
  focus, labels, heading order, landmark semantics, and status text.
- Support reduced-motion preferences. Do not encode stock, warning, or invoice
  status using color alone.
- The same tokens control focus, hover, disabled, selected, warning, success,
  and error states. A foreground/background pair must meet contrast checks.

## Implementation rule

Create the shell and component treatment once, then use them across module
routes. If a module seems to need a second design, first extend the common
component/token system and document the exception; do not fork the theme.
