# Zelyra UI (1.0.6)

Zelyra UI is the locally delivered default design system for business
interfaces. Zelyra's compiler loads and checks its components. CSS and a small
enhancement script are embedded in the Zelyra binary; an application needs no
npm, frontend build, or CDN request. This document describes the released 1.0.6
interface and its tested limits.

## Getting started

Explicitly import the standard library in the project entry file. Component
tags in HTML are then written without the `ui.` prefix:

```zelyra
import "zelyra/ui" as ui

view BusinessShell {
    html {
        <!doctype html>
        <html data-z-ui>
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <title>Customer management</title>
            </head>
            <body>
                <AppShell id="business" navigation_label="Main navigation">
                    <slot name="header"><strong>Acme administration</strong></slot>
                    <slot name="navigation"><a href="/customers">Customers</a></slot>
                    <slot name="footer">Local business application</slot>
                </AppShell>
            </body>
        </html>
    }
}

page "/customers" {
    view: BusinessShell
    html {
        <Container>
            <PageHeader title="Customers" description="Manage contacts and accounts">
                <slot name="actions"><Button label="Add customer" href="/customers/new" appearance="success" /></slot>
            </PageHeader>
            <Grid columns="two">
                <Card title="Active customers"><p>125 customers recorded</p></Card>
                <Card title="Open tasks"><Badge text="8 open" appearance="warning" /></Card>
            </Grid>
        </Container>
    }
}
```

The example uses the existing grammar and the same standard library shown by
the invoice tutorial's component gallery. Existing Zelyra `component`, `view`,
and slot syntax remains the extension mechanism. Place `<html data-z-ui>` in
the Zelyra UI scope; this keeps framework rules from restyling unrelated HTML.

## Typed variants and form state

The compiler checks `appearance` against `default`, `success`, `warning`, and
`danger`. Component markup uses an HTML attribute; declarative forms use an
unquoted enum value:

```zelyra
import "zelyra/ui" as ui

page "/status" {
    html {
        <Badge text="Paid" appearance="success" />
        <Alert title="Payment due" appearance="warning">The due date is tomorrow.</Alert>
        <Button label="Delete" appearance="danger" />
    }
}

form CustomerEdit {
    field email: Email { appearance: warning }
    field active: Bool { appearance: success }
    field password: String { widget: password }
}
```

Invalid variants produce a stable compiler diagnostic. Presentation and a real
validation failure are separate: `appearance` changes presentation; the form
runtime sets `aria-invalid="true"` only when a field error actually exists. A
variant does not make input valid and does not replace validation,
authorization, or CSRF checks.

Generated forms use Zelyra UI field, error, action, table, and page classes.
Types such as `Date`, `Time`, `Email`, `Bool`, and numeric values continue to
select native HTML inputs. The compiler checks `widget` against `text`, `email`,
`number`, `url`, `checkbox`, `date`, `time`, and `password`. Permitted submitted
values survive validation errors; password widgets and fields whose names
contain `password`, `token`, `secret`, or `hash` are never rendered back with
their submitted value. `required`, `disabled`, `readonly`, CSRF, escaping, SQL
parameter binding, and server-side authorization continue to come from the
existing runtime rules; styling does not replace them.

## Component reference

`?` marks an optional property. Unlisted slots are not provided. “Default
content” means the regular content between the component's opening and closing
tags.

### Layout

| Component | Properties | Slots / behavior |
|---|---|---|
| `AppShell` | `id: String`, `navigation_label: String` | `header`, `navigation`, default content, `footer`; responsive header/navigation/content/footer layout |
| `Container` | — | Default content; centered `--z-content-width` |
| `PageHeader` | `title: String`, `description: String?` | `actions`; title, description, and actions |
| `Grid` | `columns: UiGridColumns?` (`one`, `two`, `three`, `four`; defaults to `one`) | Default content; 3/4 columns become two below 900 px and one below 600 px |
| `Stack`, `Inline` | — | Default content; vertical or wrapping horizontal arrangement |
| `Toolbar` | — | Default content; toolbar role and wrapping actions |
| `Divider` | — | —; horizontal divider |

### Content, status, and navigation

| Component | Properties | Slots / behavior |
|---|---|---|
| `Card` | `title: String?` | `header`, default content, `footer` |
| `Badge` | `text: String`, `appearance: UiAppearance?` | — |
| `Alert` | `title: String`, `appearance: UiAppearance?` | Default content; status message |
| `EmptyState` | `id: String`, `title: String`, `description: String?` | `actions` |
| `LoadingState` | `label: String` | —; polite status message |
| `ErrorState` | `title: String`, `message: String` | `actions`; error message |
| `Breadcrumbs` | `label: String?` | Default content, usually `<li>` elements |
| `DescriptionList` | — | Default content with `<dt>`/`<dd>` |
| `Progress` | `label: String`, `value: UInt`, `max: UInt` | —; native `<progress>` |
| `Pagination` | `label: String?` | Default content with real links |
| `Dropdown` | `label: String` | Default content; native `<details>` |
| `Disclosure` | `summary: String` | Default content; native `<details>` |
| `MobileNavigation` | `id: String`, `label: String` | Default content; navigation stays visible without JavaScript |
| `Tabs` | `id: String`, `first_label: String`, `second_label: String` | `first`, `second`; exactly two panels; without JavaScript these are fragment links and both panels stay visible |

### Actions and forms

| Component | Properties | Slots / behavior |
|---|---|---|
| `Button` | `label: String`, `href: String?`, `appearance: UiAppearance?`, `translation_key: String?` | `href` creates a link; otherwise a `type="button"`; never submits a form |
| `SubmitButton` | `label: String`, `appearance: UiAppearance?`, `translation_key: String?` | Explicit `type="submit"` |
| `ButtonGroup` | — | Default content; groups actions |
| `Link` | `href: String`, `label: String` | Real link |
| `TextField` | `id`, `name`, `label: String`; `value`, `placeholder: String?`; `required`, `disabled`, `readonly`, `invalid: Bool?`; `appearance: UiAppearance?` | `hint`, `error`; native text input |
| `EmailField` | `id`, `name`, `label: String`; `value`, `placeholder: String?`; `required`, `disabled`, `invalid: Bool?`; `appearance: UiAppearance?` | `hint`, `error`; `type="email"` |
| `PasswordField` | `id`, `name`, `label: String`; `autocomplete: String?`; `required`, `disabled`, `invalid: Bool?`; `appearance: UiAppearance?` | `hint`, `error`; deliberately has no `value` property |
| `NumberField` | `id`, `name`, `label: String`; `value`, `min`, `max`, `step: String?`; `required`, `disabled`, `invalid: Bool?`; `appearance: UiAppearance?` | `hint`, `error`; `type="number"` |
| `DateField`, `TimeField` | `id`, `name`, `label: String`; `value: Date?` or `Time?`; `required`, `disabled`, `invalid: Bool?`; `appearance: UiAppearance?` | `hint`, `error`; native date/time input |
| `Textarea` | `id`, `name`, `label: String`; `value: String?`; `rows: UInt?`; `required`, `disabled`, `readonly`, `invalid: Bool?`; `appearance: UiAppearance?` | `hint`, `error` |
| `SelectField` | `id`, `name`, `label: String`; `required`, `disabled`, `invalid: Bool?`; `appearance: UiAppearance?` | Native `<option>` elements in default content; `hint`, `error` |
| `CheckboxField` | `id`, `name`, `label: String`; `value: String?`; `checked`, `required`, `disabled`, `invalid: Bool?`; `appearance: UiAppearance?` | `hint`, `error` |
| `RadioField` | `id`, `name`, `label`, `value: String`; `checked`, `disabled`, `invalid: Bool?`; `appearance: UiAppearance?` | —; group with `RadioGroup` |
| `SwitchField` | `id`, `name`, `label: String`; `checked`, `disabled`, `invalid: Bool?`; `appearance: UiAppearance?` | —; native checkbox with `role="switch"` |
| `RadioGroup`, `Fieldset` | `legend: String` or `String?` | Default content; `RadioGroup` is a fieldset with a legend |
| `FieldGroup` | `label: String?` | Default content |
| `FormActions` | — | Default content; wrapping action area |
| `ErrorSummary` | `title: String` | Default content with error list items; `role="alert"` |
| `ConfirmationForm` | `action`, `message`, `confirm_label`, `cancel_href: String` | `security`; real POST form. The route must enforce CSRF, authentication, and permissions |
| `Dialog` | `title`, `open_label: String`; `id`, `close_label: String?` | Default content; deterministic automatic IDs, native modal interaction with Escape and focus return; available without JavaScript; `close_label` defaults to localized “Close” |
| `ToastRegion`, `Toast` | `Toast`: `message: String`, `appearance: UiAppearance?` | Region has default content; toast is an additional live message |

### Dialogs with safe defaults

For a typical dialog, provide only its title, opener label, and content:

```zelyra
<Dialog title="DVDs starting with 'A'"
        open_label="Show only DVDs starting with 'A'">
    <ul>
        for dvd in a_dvds {
            <li><strong>{dvd.title}</strong> ({dvd.release_year}) – {dvd.genre}</li>
        }
    </ul>
</Dialog>
```

A server view currently supports `for item in collection { ... }` and field
interpolation. An `if len(...)` block inside HTML is not view syntax yet, so the
list is empty when there are no matches. The runnable database example is
[`examples/dvd_dialog/main.zyl`](../examples/dvd_dialog/main.zyl). Its `load`
query runs with the page request; opening the dialog does not fetch data.

When `id` is omitted, Zelyra derives reproducible IDs from component order.
Dialogs inside `for` loops also receive a deterministic row-index suffix. The
opener, dialog, and heading are linked automatically. An explicit `id` is
preserved. If `close_label` is omitted, the existing UI localization catalog
supplies it (“Close” in English, “Schließen” in German). Both properties can
still be set explicitly. The native close button has `type="button"`, so it
does not submit a containing form.

With JavaScript, the native `<dialog>` opens with `showModal()`. The browser
manages the modal background, Escape, focus containment, `autofocus`, and focus
return. Without JavaScript, the dialog remains visible and usable as an open
dialog element.

### Data and business documents

| Component | Properties | Slots / behavior |
|---|---|---|
| `DataTable` | `caption: String` | `head`, `body`, `foot`; real table caption and a bounded horizontally scrollable table region |
| `DataTableTools` | — | Default content in a GET form; search and filters remain server-side |
| `BarChart` | `title`, `description: String` | `ChartBar` items as default content; each item has a native progress bar and number |
| `ChartBar` | `label: String`, `value`, `max: UInt` | — |
| `LineChart` | `id`, `title`, `description`, `points: String` | `data`; inline SVG plus text/table alternative |
| `DonutChart` | `id`, `title`, `description: String`, `percentage: UInt` | `data`; inline SVG plus text alternative. Values should be between 0 and 100; the compiler currently checks only the `UInt` type |
| `PrintableDocument` | `title: String`, `document_number: String?` | Default content, `footer`; print-ready document |
| `PrintAction` | `label: String`, `translation_key: String?` | —; calls `window.print()`; the browser can print or save as PDF |
| `EmailLink` | `recipient`, `subject`, `body`, `label: String`; `translation_key: String?` | —; opens a mail client with `mailto:`; recipient, subject, and draft body are URL encoded. Attach a previously saved PDF manually. Use the explicit `mail_queue`/outbox pipeline for server-side delivery |

For other view links, `{url_encode:value}` can emit a `String`, `Email`, or
`Url` property as one compiler-checked URL component. This prevents characters
such as `&` or `?` from adding extra `mailto:` parameters. The recipient is
not checked for existence or policy; `EmailLink` only opens the local mail
client. `body` prefills a draft and never sends the message.

The current chart set is bar, line, and donut. It is rendered server-side and
has no external chart library. Line-chart points are a `String`, not a
compiler-checked numeric sequence; the 0–100 range for `DonutChart.percentage`
is also a domain-level input requirement. Data that is not understandable from
a text description or table should not be communicated by graphics alone.

## Design, themes, and delivery

CSS uses Cascade Layers `reset`, `base`, `layout`, `components`, `utilities`,
`theme`, and `overrides`. Public classes start with `z-`, design variables with
`--z-`, and the rules are scoped under `html[data-z-ui]`. System font stacks
are used. The framework makes no external font or runtime-library request.

Light and dark themes, system preference, a persisted user choice,
comfortable/compact density, `prefers-reduced-motion`, and print styles are
included. JavaScript enhances dialogs, tabs, mobile navigation, theme and
density controls, and toasts. Navigation, links, GET/POST forms, disclosure,
selection controls, and printable content use native HTML behavior.

`ZELYRA_UI_STYLESHEET` selects `zelyra` (default), `tabler` (locally bundled
Tabler Core CSS), or a safe project-relative `.css` file. The selection
replaces the Zelyra UI stylesheet; `zelyra.theme.css` loads after it. Path
traversal, symlinks, non-UTF-8 files, files larger than 512 KiB for replacement
CSS or 128 KiB for the project theme are rejected. See the
[environment reference](env.en.md) for precedence, security, Compose, and
reserved URLs.

Project themes should change related color tokens together, for example
`--z-color-primary`, `--z-color-on-primary`, `--z-color-primary-hover`, and
`--z-color-primary-soft`. Likewise keep
`--z-color-success`/`--z-color-on-success`/`--z-color-success-soft`,
`--z-color-warning`/`--z-color-on-warning`/`--z-color-warning-soft`, and
`--z-color-danger`/`--z-color-on-danger`/`--z-color-danger-soft` together.
Other variables control canvas/surface/text/border/focus/link, system fonts,
font size, spacing, radii, shadows, control height, content width, overlay
layers, and motion. See the [theme template](../examples/invoice_tutorial.theme.css).

## Studio, CRUD, and security

When the cursor is in a component's opening tag, Studio's Properties panel
shows supported properties, slots, and automatic defaults or explicit values.
For `appearance`, Studio can insert one of the four valid variants. For a
declarative form field, it also explains that `invalid` comes from server
validation. This metadata view is assistance; the compiler remains the
authority.

Generated CRUD pages and authored pages share CSS, language shell, and central
form/table classes. Server-side search, filters, sorting, pagination,
permissions, CSRF, validation, and bound SQL parameters remain authoritative.
A hidden or disabled control is not access control.

An automatically generated `tableview` can choose the same layout component
with `layout: BusinessShell`:

```zelyra
tableview CustomerOverview {
    layout: BusinessShell
    source sql<CustomerRow[]> { SELECT id, name FROM customers }
    columns { id name }
    searchable
}
```

Zelyra inserts the generated page content into the view's default content
slot. The layout provides the only `<main>` element. A missing layout view
produces `E-VIEW-030`.

The reference application route `/ui/components` demonstrates states,
variants, form fields, a responsive table, charts, print, and email actions.
`examples/invoice_tutorial/TUTORIAL.en.md` explains the source; the
[invoice tutorial guide](tutorials/invoice.en.md) covers startup and editing.
Full Tabler compatibility is not claimed. The built-in stylesheet can be
replaced by Tabler, but old Tabler classes are not automatically converted to
Zelyra components.

## Limits

Only the components listed above are provided; arbitrary HTML attributes, CSS
classes, or project CSS are not generally statically checked. PDF currently
means printing or saving as PDF in the browser; there is no custom PDF renderer
or automatic attachment delivery. Save the invoice as a PDF in the print
dialog, then attach it manually to the email draft. Server-side email requires
an explicitly authorized route, `mail_queue`, and the documented outbox worker.
