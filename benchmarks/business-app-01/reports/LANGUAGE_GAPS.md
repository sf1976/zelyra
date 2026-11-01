# Mini-ERP language gap log

This log records language gaps found while applying Zelyra to practical
business applications. `RESOLVED` means the root cause was fixed and its
behavior is covered by regression tests; it does not imply that Mini-ERP is
complete.

Current regression baseline: the full `cargo test --workspace --locked` suite
passes after the fixes below. The Mini-ERP master-data HTTP regression also
passes against a disposable MariaDB instance. On 2026-11-01 the password-
generator HTTP E2E was rerun with a fresh temporary MariaDB using the feature-
branch compiler; all five profiles, sign-in, invalid-input rejection, and
response cache headers passed. The master-data HTTP E2E also passed against a
separate fresh MariaDB, covering defaults, readable relation choices, article
creation, and archived-group exclusion.

## BA01-001 — Standalone forms could not call typed Zelyra logic

```text
ID: BA01-001
Date: 2026-10-24
Affected ERP module: Password-generator prerequisite; reusable for business forms

Business requirement:
A validated standalone form must invoke native Zelyra logic and display a
computed result without introducing an application-specific foreign-language
handler or persisting a value that should remain transient.

Expected Zelyra implementation:
A form action references a typed Zelyra function, declares its effects, receives
validated field values, and renders its result safely.

Actual problem:
Standalone forms supported validation and SQL actions, but could not call a
typed Zelyra function. The password generator therefore had no idiomatic native
path for secure random generation from validated form fields.

Cause:
The form AST and web bridge had no function-handler reference or typed
field-to-parameter invocation path.

Affected language component:
AST, parser, project linker, static action checks, CLI runtime bridge, web form
runtime, localization, architecture/impact/reflection metadata.

General solution:
Add a handler reference for standalone form actions. Require exactly one
handler action; require a non-generic function returning `String`; require
field and parameter names, order, and types to match; keep capability checks;
reject SQL/action-body/redirect mixing and file inputs; escape output and send
it with no-store headers.

Architecture decision:
Use ordinary checked Zelyra functions and the existing effect/capability
model. Keep the feature limited to standalone forms; database-backed CRUD
actions continue to use their existing typed execution mechanism.

Implemented change:
Implemented on the `feature/standalone-native-form-handlers` worktree. The
password-generator project and HTTP E2E test demonstrate the path. The handler
also composes with the native authorization metadata. Publication and merge
into `main` are still pending.

New tests:
Parser syntax test; CLI positive/negative field, return type, and capability
checks; web CSRF, escaped result, no-store, and no-referrer test; live app E2E
for all five character profiles and invalid fields.

Regression result:
Parser, CLI, and web tests pass. The full `cargo test --workspace --locked`
suite passes (including 674 CLI tests and 225 web tests; 4 web tests and 2 CLI
tests are ignored, and MariaDB-only integrations require a disposable DB).
`cargo clippy --workspace --all-targets --locked -- -D warnings`, handbook
snippet validation, formatting, and diff checks pass. The app compiler check,
native Zelyra tests, and live HTTP E2E pass both in this benchmark directory
and at `/opt/zelyra-password-generator`.

Status: RESOLVED
```

## BA01-004 — CRUD create forms did not display declared defaults

```text
ID: BA01-004
Date: 2026-10-24
Affected ERP module: Master data; all generated table-backed create forms

Business requirement:
New forms must present declared table defaults so a normal submission keeps
the same value that Zelyra's table definition declares.

Expected implementation in Zelyra:
Initialize visible create-form fields from safe, literal table defaults.
Boolean defaults must be reflected in the checkbox state. Database expressions
must remain server-side and must not appear as editable literal input.

Actual problem:
`active: Bool default true` created an unchecked checkbox. Submitting the
ordinary create form omitted the field, so the record was stored as inactive.
This also made it unavailable to active-only relation lookups such as article
group selection.

Cause:
Generated CRUD GET routes initialized create forms with an empty value map and
did not copy declared table defaults into the view model.

Affected language component:
Generated web CRUD form initialization and its mapping from typed table
metadata to native form controls.

General solution:
Initialize each visible, non-computed create field from its declared literal
default. Keep temporal SQL identifier expressions server-side. Preserve edit
values and never overwrite an existing form value.

Architecture decision:
Use the existing `TableDef` and `FormDef` as the sole metadata source and the
existing native form renderer. Do not add ERP-specific defaults or browser-side
script.

Implemented change:
Create-route initialization now seeds displayable defaults from table
metadata. This includes Boolean, numeric, string, and enum-like identifier
defaults on both the parent form and the first generated child row;
timestamp/date/time identifier expressions are not rendered as literal values.

New tests:
Unit coverage for boolean and enum-like literal defaults and for withholding
temporal database expressions. Mini-ERP live HTTP verification creates a new
active article group and unit through the generated forms.

Regression result:
Two focused web unit tests pass. Live HTTP confirms both new records are saved
with `active=1`; relation fields then resolve to server-generated option lists.

Status: RESOLVED
```

## BA01-005 — PascalCase relation fields rendered as text inputs

```text
ID: BA01-005
Date: 2026-10-24
Affected ERP module: Articles and all CRUD fields referencing record types

Business requirement:
CRUD fields typed as module-qualified records must render selectable related
records with useful labels and must validate their submitted identities.

Expected implementation in Zelyra:
Resolve the final record type identifier to the declared table using the same
PascalCase-to-snake_case convention as database schema planning. Display the
most useful available human-readable column.

Actual problem:
The `group: groups::ArticleGroup` field rendered as free text because the web
resolver lowercased `ArticleGroup` to `articlegroup`. The unit field was a
dropdown, but displayed numeric IDs because its display column was named
`label`, which the selector did not recognize.

Cause:
The web relation resolver had a separate lowercase-only implementation and
the display-column preference omitted common columns `label` and `code`.

Affected language component:
Generated CRUD relation resolution, option loading, and form rendering.

General solution:
Reuse the database crate's shared type-name normalization for web relation
lookups. Prefer `name`, `label`, `title`, `code`, `number`, or `email` as
available relation display text.

Architecture decision:
Share the existing record-name normalization instead of introducing aliases
or ERP-specific relation metadata. Keep relation options server-loaded and
validate submitted values against those options.

Implemented change:
The shared snake-case normalizer is now public for internal crate reuse, and
web form relation lookup uses it. Display-column selection now supports
`label` and `code`.

New tests:
Database test covers `ArticleGroup`, `InvoiceLine`, `Unit`, and acronyms.
Live Mini-ERP HTTP test confirms `group` and `unit` render selects populated
with the active group and human-readable unit label.

Regression result:
Database naming test passes. Live HTTP confirms `group` is now a select with
`Aktive Gruppe E2E` and the unit select contains the human-readable
`Stück E2E`. A complete article form submission persists both relation IDs,
joins back to the intended group and unit, and stores `active=1`.

Status: RESOLVED
```

## BA01-006 — Archived records remained selectable in relation fields

```text
ID: BA01-006
Date: 2026-10-24
Affected ERP module: Article groups and all soft-deleted relation targets

Business requirement:
Archived master data must remain available in historical records but must not
be selectable for new records.

Expected implementation in Zelyra:
Generated CRUD relation lookups should automatically exclude rows whose
declared soft-delete timestamp is set, while retaining normal tenant scoping.

Actual problem:
Relation option queries selected every row from the target table. Once a
master-data record was soft-deleted, it still appeared in create-form selects.

Cause:
The generic relation option loader applied tenant filtering when configured
but did not consult the target table's soft-delete metadata.

Affected language component:
Generated web CRUD relation option loading.

General solution:
When the resolved schema table contains `archived_at`, add a parameter-free
`archived_at IS NULL` predicate to the option query. Keep the filter generic
and combine it with existing tenant access predicates.

Architecture decision:
Infer the archive predicate from the existing declared schema rather than
adding per-relation application code or a second registry.

Implemented change:
The relation loader now filters archived target rows whenever the table
declares the conventional soft-delete timestamp.

New tests:
The Mini-ERP HTTP regression creates a group, links it to an article, archives
the group through its generated CRUD route, and verifies it no longer appears
in a new article form.

Regression result:
Three focused web unit tests pass. The repeatable live MariaDB HTTP test
archives a group, confirms it disappears from new article options, and keeps
the previously linked article visible in the article list.

Status: RESOLVED
```

## BA01-003 — PascalCase record names did not resolve to snake_case tables

```text
ID: BA01-003
Date: 2026-10-24
Affected ERP module: Article groups, units, articles, warehouse locations

Business requirement:
CRUD fields typed as module-qualified Zelyra records must resolve their
database relation and generate the correct foreign-key column and lookup.
For example, `groups::ArticleGroup` must resolve to `article_groups`.

Expected implementation in Zelyra:
Resolve a record type name to the conventional snake_case table name, then
apply the existing singular/plural table matching rules. Preserve module
qualification handling and acronym boundaries.

Actual problem:
`table_for_type` lowercased the final PascalCase identifier without word
boundaries. `ArticleGroup` became `articlegroup`, so the compiler reported an
unknown relation target even though `article_groups` was declared. The same
problem affected `InvoiceLine` and other multiword record types.

Cause:
The table resolver assumed record identifiers and table names shared a
lowercase spelling convention and did not normalize PascalCase.

Affected language component:
Database schema relation resolution and generated foreign-key metadata.

General solution:
Normalize PascalCase and acronym boundaries to snake_case before matching
declared table names. Keep module-qualified names normalized to their final
record identifier and retain existing pluralization behavior.

Architecture decision:
Use deterministic naming normalization in the shared database resolver; do
not add ERP-specific relation aliases or special cases to Zelyra source.

Implemented change:
`table_for_type` now uses `type_name_to_snake_case`. The Mini-ERP compiles, and
`zelyra db create` emits `group_id` and `unit_id` foreign keys that reference
`article_groups(id)` and `units(id)`.

New tests:
Database tests cover module-qualified `ArticleGroup` and `InvoiceLine`,
pluralized `Unit`, and acronym conversion for `XMLInvoiceLine`.

Regression result:
Both focused database tests pass. `zelyra check` passes for the Mini-ERP and
the generated MariaDB schema contains the expected relation columns and
constraints. Full workspace regression remains to be run before release.

Status: RESOLVED
```

## BA01-002 — Standalone typed form handlers could not declare authorization

```text
ID: BA01-002
Date: 2026-10-24
Affected ERP module: Password-generator access control; reusable business forms

Business requirement:
Require an authenticated user and a named permission before a standalone form
handler can execute.

Expected implementation in Zelyra:
Declare `requires auth` and `permits "permission.name"` on a form handler action.
The web authorization path must reject missing sessions and permissions before
the handler executes.

Actual problem:
The parser and web route carried action authorization metadata, but the CLI's
standalone-handler validator rejected it. Protected forms could not use the
native typed handler.

Cause:
The standalone-handler compatibility check allowed only labels and effects,
even though authorization is a separate policy layer supported by the form
runtime.

Affected language component:
CLI action validation, generated standalone form routes, web authorization.

Proposed general solution:
Keep SQL, redirects, confirmations, guards, notices, and mixed action bodies
out of standalone handler forms, while allowing `requires auth` and `permits`
to reach the existing server-side authorization path.

Architecture decision:
Do not add a second authentication mechanism to forms. Reuse the existing
auth/session/permission evaluator and reject access before invoking the typed
handler.

Implemented change:
Allowed standalone handler actions to declare authorization; route dispatch
uses the existing `form_authorization` path. The password generator uses this
with the reusable local `zelyra_identity` package. Its normal login form,
persistent sessions, role permissions, manual account administration, and
disabled public registration are running at `https://zelyra.eu:8176`.

New tests:
CLI positive test for protected handler actions; web tests for unauthenticated
401, missing-permission 403, and authorized handler invocation; HTTP E2E for
login, session cookie, admin page, all five password profiles, invalid input,
and no-store response headers.

Regression result:
Focused CLI and web tests pass. The protected-app HTTP E2E passes on a separate
MariaDB and through the public HTTPS endpoint. PR CI is still running.

Status: RESOLVED
```
