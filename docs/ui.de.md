# Zelyra UI (1.0.6)

Zelyra UI ist das lokale Standard-Designsystem für Geschäftsoberflächen. Die
Komponenten werden vom Zelyra-Compiler geladen und geprüft. CSS und das kleine
Enhancement-Script sind im Zelyra-Binary enthalten; ein Projekt braucht weder
npm noch einen Frontend-Build oder CDN-Zugriff. Dieses Dokument beschreibt die
veröffentlichte Schnittstelle von 1.0.6 und ihre geprüften Grenzen.

## Einstieg

In der Programmeinstiegsdatei die Standardbibliothek ausdrücklich importieren.
Komponentennamen in HTML werden anschließend ohne `ui.` geschrieben:

```zelyra
import "zelyra/ui" as ui

view BusinessShell {
    html {
        <!doctype html>
        <html data-z-ui>
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <title>Kundenverwaltung</title>
            </head>
            <body>
                <AppShell id="business" navigation_label="Hauptnavigation">
                    <slot name="header"><strong>Acme Verwaltung</strong></slot>
                    <slot name="navigation"><a href="/customers">Kunden</a></slot>
                    <slot name="footer">Lokale Geschäftsanwendung</slot>
                </AppShell>
            </body>
        </html>
    }
}

page "/customers" {
    view: BusinessShell
    html {
        <Container>
            <PageHeader title="Kunden" description="Kontakte und Konten verwalten">
                <slot name="actions"><Button label="Kunden anlegen" href="/customers/new" appearance="success" /></slot>
            </PageHeader>
            <Grid columns="two">
                <Card title="Aktive Kunden"><p>125 Kunden erfasst</p></Card>
                <Card title="Offene Aufgaben"><Badge text="8 offen" appearance="warning" /></Card>
            </Grid>
        </Container>
    }
}
```

Das Beispiel entspricht der vorhandenen Grammatik und wird in der
Rechnungstutorial-Komponentenübersicht mit derselben Standardbibliothek
verwendet. Für eigene Komponenten bleibt Zelyras vorhandenes `component`-,
`view`- und Slot-System maßgeblich. Im Gültigkeitsbereich von Zelyra UI muss
`<html data-z-ui>` stehen; dadurch wirken Framework-Regeln nicht auf beliebiges
fremdes HTML.

## Typisierte Varianten und Formzustand

`appearance` akzeptiert compilergeprüft `default`, `success`, `warning` oder
`danger`. Bei Komponenten wird der Wert als HTML-Attribut geschrieben; in
deklarativen Formularen als unquotierter Enumwert:

```zelyra
import "zelyra/ui" as ui

page "/status" {
    html {
        <Badge text="Bezahlt" appearance="success" />
        <Alert title="Zahlungsziel beachten" appearance="warning">Die Frist endet morgen.</Alert>
        <Button label="Löschen" appearance="danger" />
    }
}

form CustomerEdit {
    field email: Email { appearance: warning }
    field active: Bool { appearance: success }
}
```

Ungültige Varianten werden mit einem stabilen Compilerdiagnosecode gemeldet.
Die Darstellung und ein echter Validierungsfehler sind getrennt: `appearance`
ändert die Darstellung; die Formularlaufzeit setzt `aria-invalid="true"` nur
bei einem tatsächlich erkannten Feldfehler. Die Erscheinungsvariante erklärt
keine Eingabe für gültig und ersetzt weder Validierung noch Berechtigungs- oder
CSRF-Prüfungen.

Automatisch erzeugte Formulare übernehmen die Zelyra-UI-Feld-, Fehler-,
Aktion-, Tabellen- und Seitenklassen. Typen wie `Date`, `Time`, `Email`,
`Bool` und numerische Werte bestimmen weiterhin die nativen HTML-Eingaben.
Werte aus zulässigen fehlgeschlagenen Eingaben bleiben erhalten; Passwörter
werden nicht als Wert zurückgerendert. `required`, `disabled`, `readonly`,
CSRF, Escaping, SQL-Bindung und serverseitige Autorisierung stammen aus den
vorhandenen Laufzeitregeln und werden von der Gestaltung nicht ersetzt.

## Komponentenreferenz

`?` bedeutet optional. Nicht aufgeführte Slots gibt es nicht. Ein
„Standardinhalt“-Slot ist der normale Inhalt zwischen öffnendem und
schließendem Komponenten-Tag.

### Layout

| Komponente | Eigenschaften | Slots / Verhalten |
|---|---|---|
| `AppShell` | `id: String`, `navigation_label: String` | `header`, `navigation`, Standardinhalt, `footer`; responsives Kopf-/Navigations-/Inhalts-/Fußlayout |
| `Container` | — | Standardinhalt; zentrierte `--z-content-width` |
| `PageHeader` | `title: String`, `description: String?` | `actions`; Seitentitel, Beschreibung und Aktionen |
| `Grid` | `columns: UiGridColumns?` (`one`, `two`, `three`, `four`; Standard `one`) | Standardinhalt; 3/4 Spalten werden unter 900 px zweispaltig, unter 600 px einspaltig |
| `Stack`, `Inline` | — | Standardinhalt; vertikale bzw. umbrechende horizontale Anordnung |
| `Toolbar` | — | Standardinhalt; Toolbar-Rolle und umbrechende Aktionen |
| `Divider` | — | —; horizontale Trennlinie |

### Inhalt, Status und Navigation

| Komponente | Eigenschaften | Slots / Verhalten |
|---|---|---|
| `Card` | `title: String?` | `header`, Standardinhalt, `footer` |
| `Badge` | `text: String`, `appearance: UiAppearance?` | — |
| `Alert` | `title: String`, `appearance: UiAppearance?` | Standardinhalt; Statusmeldung |
| `EmptyState` | `id: String`, `title: String`, `description: String?` | `actions` |
| `LoadingState` | `label: String` | —; höfliche Statusmeldung |
| `ErrorState` | `title: String`, `message: String` | `actions`; Fehlermeldung |
| `Breadcrumbs` | `label: String?` | Standardinhalt, gewöhnlich `<li>`-Elemente |
| `DescriptionList` | — | Standardinhalt mit `<dt>`/`<dd>` |
| `Progress` | `label: String`, `value: UInt`, `max: UInt` | —; natives `<progress>` |
| `Pagination` | `label: String?` | Standardinhalt mit echten Links |
| `Dropdown` | `label: String` | Standardinhalt; natives `<details>` |
| `Disclosure` | `summary: String` | Standardinhalt; natives `<details>` |
| `MobileNavigation` | `id: String`, `label: String` | Standardinhalt; Navigation bleibt ohne JavaScript sichtbar |
| `Tabs` | `id: String`, `first_label: String`, `second_label: String` | `first`, `second`; genau zwei Bereiche, ohne JavaScript Sprunglinks mit beiden sichtbaren Inhalten |

### Aktionen und Formulare

| Komponente | Eigenschaften | Slots / Verhalten |
|---|---|---|
| `Button` | `label: String`, `href: String?`, `appearance: UiAppearance?`, `translation_key: String?` | `href` erzeugt einen Link, sonst `type="button"`; sendet ein Formular nicht ab |
| `SubmitButton` | `label: String`, `appearance: UiAppearance?`, `translation_key: String?` | expliziter `type="submit"` |
| `ButtonGroup` | — | Standardinhalt; gruppiert Aktionen |
| `Link` | `href: String`, `label: String` | echter Link |
| `TextField` | `id`, `name`, `label: String`; `value`, `placeholder: String?`; `required`, `disabled`, `readonly`, `invalid: Bool?`; `appearance: UiAppearance?` | `hint`, `error`; natives Textfeld |
| `EmailField` | `id`, `name`, `label: String`; `value`, `placeholder: String?`; `required`, `disabled`, `invalid: Bool?`; `appearance: UiAppearance?` | `hint`, `error`; `type="email"` |
| `PasswordField` | `id`, `name`, `label: String`; `autocomplete: String?`; `required`, `disabled`, `invalid: Bool?`; `appearance: UiAppearance?` | `hint`, `error`; hat absichtlich kein `value`-Property |
| `NumberField` | `id`, `name`, `label: String`; `value`, `min`, `max`, `step: String?`; `required`, `disabled`, `invalid: Bool?`; `appearance: UiAppearance?` | `hint`, `error`; `type="number"` |
| `DateField`, `TimeField` | `id`, `name`, `label: String`; `value: Date?` bzw. `Time?`; `required`, `disabled`, `invalid: Bool?`; `appearance: UiAppearance?` | `hint`, `error`; native Datums-/Zeitfelder |
| `Textarea` | `id`, `name`, `label: String`; `value: String?`; `rows: UInt?`; `required`, `disabled`, `readonly`, `invalid: Bool?`; `appearance: UiAppearance?` | `hint`, `error` |
| `SelectField` | `id`, `name`, `label: String`; `required`, `disabled`, `invalid: Bool?`; `appearance: UiAppearance?` | native `<option>` im Standardinhalt, `hint`, `error` |
| `CheckboxField` | `id`, `name`, `label: String`; `value: String?`; `checked`, `required`, `disabled`, `invalid: Bool?`; `appearance: UiAppearance?` | `hint`, `error` |
| `RadioField` | `id`, `name`, `label`, `value: String`; `checked`, `disabled`, `invalid: Bool?`; `appearance: UiAppearance?` | —; mit `RadioGroup` gruppieren |
| `SwitchField` | `id`, `name`, `label: String`; `checked`, `disabled`, `invalid: Bool?`; `appearance: UiAppearance?` | —; native Checkbox mit `role="switch"` |
| `RadioGroup`, `Fieldset` | `legend: String` bzw. `String?` | Standardinhalt; `RadioGroup` ist ein Fieldset mit Legende |
| `FieldGroup` | `label: String?` | Standardinhalt |
| `FormActions` | — | Standardinhalt; umbrechender Aktionsbereich |
| `ErrorSummary` | `title: String` | Standardinhalt mit Fehler-Listeneinträgen; `role="alert"` |
| `ConfirmationForm` | `action`, `message`, `confirm_label`, `cancel_href: String` | `security`; echtes POST-Formular. Route muss CSRF, Authentifizierung und Rechte prüfen |
| `Dialog` | `id`, `title`, `open_label`, `close_label: String` | Standardinhalt; nativ, mit JavaScript modal, Escape und Fokusrückgabe; ohne JavaScript offen verfügbar |
| `ToastRegion`, `Toast` | `Toast`: `message: String`, `appearance: UiAppearance?` | Region hat Standardinhalt; Toast ist eine ergänzende Live-Rückmeldung |

### Daten und Geschäftsdokumente

| Komponente | Eigenschaften | Slots / Verhalten |
|---|---|---|
| `DataTable` | `caption: String` | `head`, `body`, `foot`; echtes Tabellen-Caption und begrenzt horizontal scrollbarer Tabellenbereich |
| `DataTableTools` | — | Standardinhalt in einem GET-Formular; Suche und Filter bleiben serverseitig |
| `BarChart` | `title`, `description: String` | `ChartBar`-Elemente als Standardinhalt; jeder Wert hat Fortschrittsbalken und Zahl |
| `ChartBar` | `label: String`, `value`, `max: UInt` | — |
| `LineChart` | `id`, `title`, `description`, `points: String` | `data`; Inline-SVG plus Text-/Tabellenalternative |
| `DonutChart` | `id`, `title`, `description: String`, `percentage: UInt` | `data`; Inline-SVG plus Textalternative. Prozentwerte müssen fachlich zwischen 0 und 100 liegen; der Compiler prüft hier nur den Typ `UInt` |
| `PrintableDocument` | `title: String`, `document_number: String?` | Standardinhalt, `footer`; druckoptimiertes Dokument |
| `PrintAction` | `label: String`, `translation_key: String?` | —; öffnet `window.print()`; Browser kann in PDF drucken/speichern |
| `EmailLink` | `recipient`, `subject`, `body`, `label: String`; `translation_key: String?` | —; öffnet einen E-Mail-Client per `mailto:`; Empfänger, Betreff und Nachrichtentext werden als URL-Komponenten codiert. Eine zuvor gespeicherte PDF-Datei muss manuell angehängt werden. Für serverseitige Zustellung die explizite `mail_queue`-/Outbox-Pipeline verwenden |

Für weitere View-Links kann `{url_encode:wert}` eine `String`-, `Email`- oder
`Url`-Eigenschaft compilergeprüft als einzelne URL-Komponente ausgeben. Das
verhindert, dass Zeichen wie `&` oder `?` zusätzliche `mailto:`-Parameter
erzeugen. Der Empfänger wird nicht automatisch auf Existenz oder Richtlinien
geprüft; `EmailLink` öffnet nur das lokale Mailprogramm. `body` ist ein
vorausgefüllter Entwurfstext und löst keinen Versand aus.

Die Diagramm-Bibliothek enthält derzeit Balken-, Linien- und Ringdiagramm.
Sie rendert serverseitig und benötigt keine externe Grafikbibliothek. Die
Linienpunkte sind ein `String` und nicht als Zahlenfolge compilergeprüft;
Bereichsgrenzen für `DonutChart.percentage` sind ebenfalls eine fachliche
Eingabeanforderung. Diagramme, die nicht durch eine Textbeschreibung oder
Tabelle verständlich sind, sollten nicht allein grafisch kommuniziert werden.

## Design, Themes und Auslieferung

Das CSS ist in Cascade Layers `reset`, `base`, `layout`, `components`,
`utilities`, `theme` und `overrides` aufgebaut. Öffentliche Klassen beginnen
mit `z-`, Designvariablen mit `--z-`; die Regeln gelten nur unter
`html[data-z-ui]`. Schriftarten stammen aus der Systemschriftliste. Das
Framework benötigt keine externe Schrift oder Laufzeitbibliothek.

Helles und dunkles Theme, Systempräferenz, gespeicherte manuelle Auswahl,
komfortable/kompakte Dichte, `prefers-reduced-motion` und Druckregeln sind
enthalten. JavaScript verbessert Dialoge, Tabs, Mobile Navigation, Theme-
und Dichtewechsel sowie Toasts. Navigation, Links, GET-/POST-Formulare,
Disclosure, Auswahlfelder und Druckinhalte haben native HTML-Grundbedienung.

`ZELYRA_UI_STYLESHEET` wählt `zelyra` (Standard), `tabler` (lokal gebündelte
Tabler-Core-CSS) oder einen sicheren projektrelativen `.css`-Pfad. Die Auswahl
ersetzt das Zelyra-UI-Stylesheet; `zelyra.theme.css` wird danach geladen.
Path Traversal, Symlinks, Nicht-UTF-8 und Dateien über 512 KiB beim Ersatz-CSS
bzw. 128 KiB beim Theme werden abgelehnt. Details zu Vorrang, Sicherheit,
Compose und reservierten URLs stehen in der [Umgebungsreferenz](env.md).

Ein Projekt-Theme verändert verwandte Farben gemeinsam, etwa
`--z-color-primary`, `--z-color-on-primary`, `--z-color-primary-hover` und
`--z-color-primary-soft`. Entsprechend gehören bei Statusfarben jeweils
`--z-color-success`/`--z-color-on-success`/`--z-color-success-soft`,
`--z-color-warning`/`--z-color-on-warning`/`--z-color-warning-soft` und
`--z-color-danger`/`--z-color-on-danger`/`--z-color-danger-soft` zusammen.
Weitere Variablen steuern canvas/surface/text/border/focus/link, Systemschrift,
Schriftgröße, Abstände, Rundungen, Schatten, Kontrollhöhe, Inhaltsbreite,
Überlagerungsebenen und Bewegung. Siehe die [Theme-Vorlage](../examples/invoice_tutorial.theme.css).

## Studio, CRUD und Sicherheit

Steht der Cursor im öffnenden Tag einer UI-Komponente, zeigt Studio im
Eigenschaftenbereich die unterstützten Eigenschaften, Slots sowie automatische
Vorgaben oder explizite Werte. Für `appearance` kann Studio die vier gültigen
Varianten per Auswahl einsetzen. Im deklarativen Formfeld zeigt es zusätzlich,
dass `invalid` aus der Servervalidierung kommt. Diese Anzeige ist Metadatenhilfe
und kein Ersatz für die Compilerprüfung.

Generierte CRUD-Seiten und handgeschriebene Seiten teilen CSS, Sprachrahmen und
zentrale Formular-/Tabellenklassen. Serverseitige Suche, Filter, Sortierung,
Pagination, Berechtigungen, CSRF, Validierung und gebundene SQL-Parameter
bleiben maßgeblich. Ein verborgenes oder deaktiviertes Steuerelement ist keine
Zugriffskontrolle.

Eine automatisch erzeugte `tableview` kann dieselbe Layout-Komponente mit
`layout: BusinessShell` auswählen:

```zelyra
tableview CustomerOverview {
    layout: BusinessShell
    source sql<CustomerRow[]> { SELECT id, name FROM customers }
    columns { id name }
    searchable
}
```

Zelyra setzt den erzeugten Seiteninhalt in den normalen Inhalts-Slot des Views
ein. Das Layout stellt das einzige `<main>`-Element bereit. Ein nicht
vorhandener Layout-View wird mit `E-VIEW-030` gemeldet.

Die Referenzanwendung `/ui/components` zeigt Zustände, Varianten,
Formularfelder, responsive Tabelle, Diagramme, Druck und E-Mail-Aktionen.
`examples/invoice_tutorial/TUTORIAL.de.md` erklärt den Quelltext; der
[Rechnungstutorial-Leitfaden](tutorials/invoice.de.md) beschreibt Start und
Bearbeitung. Eine vollständige Tabler-Kompatibilität wird nicht zugesichert.
Die interne Standard-CSS-Auswahl kann Tabler ersetzen, doch alte Tabler-Klassen
werden nicht automatisch in Zelyra-Komponenten umgewandelt.

## Grenzen

Unterstützt sind die in den Tabellen genannten Komponenten; beliebige
HTML-Attribute, CSS-Klassen oder beliebiges Projekt-CSS werden nicht allgemein
statisch geprüft. PDF bedeutet aktuell Drucken bzw. „Als PDF speichern“ im
Browser; es gibt keinen eigenen PDF-Renderer oder automatischen Versand von
Anhängen. Speichere die Rechnung zuerst im Druckdialog als PDF und hänge sie im
E-Mail-Entwurf manuell an. Serverseitige Mail benötigt eine bewusst
autorisierte Route, `mail_queue` und den dokumentierten Outbox-Worker.
