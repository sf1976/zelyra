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
    field password: String { widget: password }
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
`widget` ist compilergeprüft und akzeptiert `text`, `email`, `number`, `url`,
`checkbox`, `date`, `time` oder `password`. Werte aus zulässigen
fehlgeschlagenen Eingaben bleiben erhalten; Passwortfelder und Felder mit
Namen, die `password`, `token`, `secret` oder `hash` enthalten, werden nie mit
ihrem Eingabewert zurückgerendert. `required`, `disabled`, `readonly`,
CSRF, Escaping, SQL-Bindung und serverseitige Autorisierung stammen aus den
vorhandenen Laufzeitregeln und werden von der Gestaltung nicht ersetzt.

## Komponentenreferenz

`?` bedeutet optional. Nicht aufgeführte Slots gibt es nicht. Ein
„Standardinhalt“-Slot ist der normale Inhalt zwischen öffnendem und
schließendem Komponenten-Tag.

### Layout

| Komponente | Eigenschaften | Slots / Verhalten |
|---|---|---|
| `AppShell` | `id: String`, `navigation_label: String` | `header`, `navigation`, Standardinhalt, `footer`; responsives Layout und integrierter Hell-/Dunkel-Umschalter |
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
| `Dialog` | `title`, `open_label: String`; `id`, `close_label: String?` | Standardinhalt; automatische deterministische IDs, native modale Bedienung mit Escape und Fokusrückgabe; ohne JavaScript offen verfügbar; Standard für `close_label` ist lokalisiertes „Schließen“ |
| `ToastRegion`, `Toast` | `Toast`: `message: String`, `appearance: UiAppearance?` | Region hat Standardinhalt; Toast ist eine ergänzende Live-Rückmeldung |

### Dialoge mit sicheren Standardwerten

Für den üblichen Dialog reichen Titel, öffnende Beschriftung und Inhalt:

```zelyra
<Dialog title="DVDs mit Anfangsbuchstabe 'A'"
        open_label="Nur DVDs mit 'A' anzeigen">
    <ul>
        for dvd in a_dvds {
            <li><strong>{dvd.title}</strong> ({dvd.release_year}) – {dvd.genre}</li>
        }
    </ul>
</Dialog>
```

Ein einfacher Server-View unterstützt `for item in collection { ... }` und
Feldinterpolation. Ein `if len(...)`-Block innerhalb von HTML ist derzeit keine
View-Syntax; das Beispiel lässt die Liste deshalb bei null Treffern leer.
Die ausführbare Datenbankbeispielseite steht unter
[`examples/dvd_dialog/main.zyl`](../examples/dvd_dialog/main.zyl). Ihre `load`
Abfrage läuft wie andere Seitenabfragen beim Seitenaufruf; das Öffnen des
Dialogs lädt keine Daten nach.

Ohne `id` vergibt Zelyra anhand der Komponentenreihenfolge reproduzierbare
IDs. Bei Dialogen innerhalb von `for`-Schleifen kommt der Zeilenindex als
deterministischer Suffix hinzu. Auslöser, Dialog und Überschrift werden
automatisch verknüpft. Eine angegebene `id` bleibt unverändert. Ohne
`close_label` kommt die Beschriftung aus dem bestehenden UI-Lokalisierungskatalog
(Deutsch: „Schließen“). `id` und `close_label` lassen sich bei Bedarf explizit
setzen. Die native Schließen-Schaltfläche verwendet `type="button"`, damit sie
auch innerhalb eines Formulars keine Übermittlung auslöst.

Mit JavaScript wird das native `<dialog>` durch `showModal()` geöffnet. Der
Browser verwaltet modalen Hintergrund, Escape, Fokusfalle, `autofocus` und die
Fokusrückgabe. Ohne JavaScript bleibt der Inhalt als offenes Dialogelement
sichtbar und nutzbar.

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
Das ausgelieferte Bundle behält die stabile Schnittstelle
`zelyra-ui.v1.css`; generierte Asset-URLs enthalten die Zelyra-Releaseversion,
damit Browser- und lokale Caches Releases unterscheiden können. Die Theme-
Tokens sind die unterstützte Anpassungsschnittstelle. Die modularen Quelldateien
werden gemeinsam mit dem Compiler gepflegt und nicht in neue Projekte kopiert.
Der generierte Bundle-Header führt zusätzlich eine unabhängige
Designsystemversion. Patch-Releases verbessern das Aussehen oder korrigieren
Stile ohne Änderung am Token-/Klassenvertrag; Minor-Releases dürfen kompatible
Tokens oder Komponenten ergänzen. Eine inkompatible Änderung an öffentlichen
Tokens oder Klassen erfordert eine neue Major-Version. Das aktuelle CSS-Bundle
hat Version 1.2.2. Die Navigation verwendet in allen AppShell-Ansichten dieselben
blauen Hover- und Aktivzustände. Sie lassen sich wie andere Standardfarben über
die Theme-Tokens `--z-color-primary-soft` und `--z-color-primary` oder reguläre
CSS-Overrides anpassen; Eingabefelder verwenden den anpassbaren Token
`--z-color-control-border`. Der Standardshell zeigt das ausgelieferte Zelyra-Logo
statt eines Textslogans oder eines Platzhalterzeichens. Verwaltungsbereiche
werden im Standard in responsiven Karten mit klaren Gruppen dargestellt.

Das Zelyra-Standardtheme verwendet die klare blaue Dashboard-Palette und startet
hell. `AppShell` enthält den barrierefrei beschrifteten Hell-/Dunkel-Umschalter;
die Auswahl bleibt im Browser gespeichert und gilt für alle Anwendungsseiten.
Ein dunkles Theme, optionale Systempräferenz, komfortable/kompakte Dichte,
`prefers-reduced-motion` und Druckregeln sind enthalten. JavaScript verbessert
Dialoge, Tabs, Mobile Navigation, Theme- und Dichtewechsel sowie Toasts.
Navigation, Links, GET-/POST-Formulare,
Disclosure, Auswahlfelder und Druckinhalte haben native HTML-Grundbedienung.

`ZELYRA_UI_STYLESHEET` wählt `zelyra` (Standard), `tabler` (lokal gebündelte
Tabler-Core-CSS) oder einen sicheren projektrelativen `.css`-Pfad. Die Auswahl
ersetzt das Zelyra-UI-Stylesheet; `zelyra.theme.css` wird danach geladen.
Path Traversal, Symlinks, Nicht-UTF-8 und Dateien über 512 KiB beim Ersatz-CSS
bzw. 128 KiB beim Theme werden abgelehnt. Details zu Vorrang, Sicherheit,
Compose und reservierten URLs stehen in der [Umgebungsreferenz](env.md).

Studio bietet im Projekt-Explorer den festen Zugang **Projekt-CSS öffnen**.
Er öffnet `zelyra.theme.css` direkt; fehlt die Datei in einem älteren Projekt,
legt die ausdrückliche Aktion eine kommentierte Vorlage an. Neu erzeugte
Projekte enthalten die Datei bereits. `import "zelyra/ui" as ui` importiert
die in Zelyra eingebauten, typisierten UI-Komponenten. Es ist kein CSS-Import
und deshalb keine Quelldatei im Projektordner.

Ein Projekt-Theme verändert verwandte Farben gemeinsam, etwa
`--z-color-primary`, `--z-color-on-primary`, `--z-color-primary-hover` und
`--z-color-primary-soft`. Entsprechend gehören bei Statusfarben jeweils
`--z-color-success`/`--z-color-on-success`/`--z-color-success-soft`,
`--z-color-warning`/`--z-color-on-warning`/`--z-color-warning-soft` und
`--z-color-danger`/`--z-color-on-danger`/`--z-color-danger-soft` zusammen.
Weitere Variablen steuern canvas/surface/text/border/focus/link,
Navigationsfarben, Systemschrift, Schriftgröße, Abstände, Rundungen, Schatten,
Kontrollhöhe, Inhaltsbreite, Überlagerungsebenen und Bewegung. Einen
vollständigen Einstieg für Palette, Navigation, Seitenbreite, Rundungen,
Schatten und CRUD-Karten bietet das
[Anpassungsbeispiel](../examples/theme_customization/zelyra.theme.css).
Das [Rechnungstheme](../examples/invoice_tutorial.theme.css) zeigt eine kleinere
projektspezifische Anpassung.
Vorhandene Komponenten lassen sich gezielt unter `:where(html[data-z-ui])`
anpassen, zum Beispiel mit `.z-card` oder `.z-button`. Eigene Klassen können
weiterhin im Seiten-HTML verwendet werden. Ändere möglichst nur benötigte
Tokens und halte Vorder- und Hintergrundfarben gemeinsam passend; Framework-
Dateien müssen dafür nicht kopiert werden.

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
table customers {
    id: Id primary auto
    name: String(100) required
    email: Email required
}
struct CustomerRow { id: Id name: String(100) email: Email }

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

### Generierte CRUD- und Tableview-Seiten gezielt erweitern

Gemeinsame Formularvorgaben bleiben in `form`; `create` und `edit` können
Titel, Absende-Schaltfläche und eigene Inhalte für genau ihre jeweilige Route
überschreiben. Nicht gesetzte Texte übernehmen die gemeinsame Vorgabe und
danach den CRUD-Standard. `before_form` steht nach Überschrift und Fehlerliste,
`after_form` direkt nach dem generierten Formular. `field_content` erscheint
innerhalb des passenden Feldblocks unter dem Steuerelement:

```zelyra
crud Customer -> customers {
    view {
        fields { customer_number name email }
        form { title: "Kundenkonto" submit: "Speichern" }
        create {
            title: "Neues Kundenkonto"
            before_form {
                html { <p>Bitte prüfe zuerst, ob der Kunde bereits existiert.</p> }
            }
            field_content {
                customer_number {
                    html {
                        <small>Aktuelle Eingabe: {values.customer_number}</small>
                        <a href="/customers">Kundennummern in der Übersicht prüfen</a>
                    }
                }
            }
        }
        edit {
            submit: "Änderungen speichern"
            after_form { html { <p>Bearbeitet: {record.name}</p> } }
        }
    }
}
```

`values.feld` bezeichnet einen typisierten, aktuell im Formular vorhandenen
Wert. `record.feld` ist nur im Edit-Block verfügbar und ist auf sichere
Formularfelder begrenzt; IDs, Mandanten-/Besitzerschlüssel und sensible
Passwort-, Token-, Secret- oder Hashfelder sind nicht verfügbar. Beim ersten
Aufruf der Edit-Seite enthält `record` die geladenen Formularwerte. Nach einem
Validierungsfehler enthält es dieselben erhaltenen Eingaben wie `values`,
damit die Zusatzinhalte den sichtbaren Formularstand erklären. Auf Create gibt
es kein gespeichertes `record`. Alle dynamischen Werte werden beim HTML-
Rendering escaped. Eigene Inhaltsbereiche liegen außerhalb des generierten
`<form>`-Elements; verschachtelte Formulare werden mit
`E-CRUD-016` abgewiesen. Suche, Speichern, Berechtigungen, CSRF und
serverseitige Validierung bleiben beim CRUD-System.

Auch eine generierte `tableview` kann Seiteninhalte, zusätzliche Inhalte in
einer Datenzelle und eine typisierte Aktionszelle je Datenzeile erhalten.
`row_fields` benennt ausdrücklich Ergebnisfelder, die eine Vorlage verwenden,
die aber keine eigene Tabellenspalte erhalten sollen. Suche, Filter und
Sortierung verwenden weiterhin nur die in `columns`, `filter` und den
Sortiereinstellungen freigegebenen Felder. Vorhandene UI-Komponenten,
darunter Dialoge, funktionieren auch in einer Datenzelle; automatisch
vergebene Dialog-IDs werden pro Zeile und Zelle eindeutig gemacht:

```zelyra
tableview CustomerOverview {
    source sql<CustomerRow[]> { SELECT id, name, email FROM customers }
    columns { name }
    view {
        before_content { html { <p>Kundenübersicht mit Kontaktoptionen</p> } }
        row_fields { id email }
        column_content {
            item: customer
            name {
                html { <small>{customer.email}</small> }
            }
        }
        row_actions {
            item: customer
            html {
                <Dialog title="Kundenkontakt" open_label="{customer.name} anzeigen">
                    <p>{customer.email}</p>
                </Dialog>
            }
        }
        after_content { html { <a href="/customers/new">Kundenkonto anlegen</a> } }
    }
}
```

Die SQL-Projektion muss alle angezeigten und ausdrücklich in `row_fields`
benannten Eigenschaften liefern. Im Bindungsobjekt sind nur `columns` und
`row_fields` verfügbar. `column_content` muss auf eine angezeigte Spalte
verweisen; ein fehlendes oder unpassendes Feld wird vom Compiler abgewiesen.
Der Renderer wählt die benannten Zusatzfelder zwar für die jeweilige Zeile
aus, zeigt sie aber nicht als separate Spalten an. Zusätzliche Inhalte dürfen
keine `<form>`-Elemente enthalten. Links navigieren;
lokale Buttons starten nur unterstützte UI-Interaktionen. Innerhalb von
`field_content` muss ein eigener Button ausdrücklich `type="button"` haben;
sonst würde ein Browser ihn als weiteren Submit-Button interpretieren.
Änderungen an
Datenbankdaten müssen über vorhandene CSRF- und berechtigungsgeprüfte CRUD-
Aktionen laufen. Beliebiges CSS/HTML oder freie Klassen werden dadurch nicht
allgemein statisch geprüft.

Datensatzabhängige CRUD-Rechte können ausdrücklich gekennzeichnete
TableView-Aktionen steuern. `crud: Modell` ordnet die Aktionen zu; Inhalte für
Bearbeiten und Löschen stehen in getrennten Slots. Das Ergebnis muss
`Modell[]` sein, und alle Modellfelder müssen in `columns` oder `row_fields`
ausgewählt werden:

```zelyra
crud Invoice -> invoices {
    permits edit "invoices.manage" when status == "draft"
    guard delete when locked == true {
        error: "Festgeschriebene Rechnungen dürfen nicht gelöscht werden."
    }
}

tableview InvoiceRows {
    source sql<Invoice[]> { SELECT id, status, locked FROM invoices }
    columns { status }
    view {
        row_fields { id locked }
        row_actions {
            item: invoice
            crud: Invoice
            edit { html { <a href="/invoices/{invoice.id}/edit">Bearbeiten</a> } }
            delete { html { <button type="button">Löschen</button> } }
        }
    }
}
```

Nicht klassifizierte `html`-Aktionen bleiben aus Kompatibilitätsgründen
unterstützt. Zelyra kann bei beliebigem Markup nicht erkennen, ob es eine
Bearbeiten- oder Löschen-Aktion darstellt; solche Inhalte werden daher nicht
automatisch anhand von Datensatzrechten ausgeblendet. Direkte Requests werden
weiterhin serverseitig geprüft.

Das ausführbare, compilergeprüfte Beispiel liegt unter
[`examples/crud_lifecycle_policies/main.zyl`](../examples/crud_lifecycle_policies/main.zyl).
MariaDB-Prüfungen für Requests, Zeilensperre und gerenderte Aktionen enthält
[`tests/mariadb-crud-lifecycle-policies-e2e.sh`](../tests/mariadb-crud-lifecycle-policies-e2e.sh).

Ein vollständiges, compilergeprüftes Beispiel mit Formularhilfen und einem
Dialog je Tabellenzeile liegt unter
[`examples/generated_page_customization/main.zyl`](../examples/generated_page_customization/main.zyl).

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
