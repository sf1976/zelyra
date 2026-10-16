# Rechnungstutorial: Befehle und Quelltext erklärt

Dieses Dokument wird mit dem Projekt erzeugt. Es erklärt den Start, die
Studio-Bedienung und die Quellmodule der Vorlage. Zeilennummern beziehen sich
auf die mitgelieferten Dateien. Leere Zeilen trennen Abschnitte; geschweifte
Klammern, HTML-Tags und SQL-Klammern begrenzen jeweils den Block, den sie
umschließen.

## Starten und bearbeiten

Voraussetzung ist Docker Engine mit dem Compose-Plugin v2 oder neuer. Prüfe es
mit `docker compose version`. Python Compose v1 wird nicht unterstützt.

~~~sh
zelyra --tutorial invoice
~~~

`zelyra` startet die CLI. `--tutorial` wählt den Vorlagenmodus und `invoice`
die Rechnungsvorlage. Ohne Zielordner heißt das neue Verzeichnis
`invoice-tutorial`. Der Befehl erzeugt Projektdateien, lokale Zugangsdaten,
Schema und wiederholbare Beispieldaten, startet MariaDB und die Anwendung und
zeigt die lokale URL an.

~~~sh
zelyra --tutorial invoice ./meine-rechnungen --no-start
cd ./meine-rechnungen
zelyra setup . --all
~~~

Beim ersten Befehl legt `./meine-rechnungen` das Ziel fest. `--no-start`
überspringt Docker und den Webstart, erzeugt aber alle Projektdateien und die
geschützte `.env`. `cd` wechselt in das Projektverzeichnis. `setup` liest die
lokale Projektkonfiguration; `.` bedeutet aktuelles Verzeichnis und `--all`
startet Datenbank, Schema, Beispieldaten und Anwendung vollständig.

`zelyra setup [ordner]` akzeptiert außerdem genau einen der Schritte
`--database` (Datenbankdienst), `--schema` (Schema einrichten) oder `--all`
(alles ausführen). `--host-port <port>` legt den Host-Port der Anwendung fest,
`--db-host-port <port>` den MariaDB-Port. Ist ein verwalteter Port belegt,
versucht Zelyra einen freien Folgeport. `--web [--port <port>]` startet eine
Browseranwendung ohne Compose-Datenbankstart. Für diese Option muss die
Datenbank bereits erreichbar sein.

~~~sh
zelyra editor ./meine-rechnungen
~~~

`editor` öffnet Zelyra Studio für den Zielordner. Ohne Ordner gilt `.`.
`--port <port>` setzt den Studio-Port (Standard `4177`); `--no-open` startet
Studio, ohne automatisch einen Browser zu öffnen. Wähle im Dateibaum
`main.zyl` und klicke **Starten**. Studio startet die passende lokale
Tutorialdatenbank, richtet das Schema ein, lädt den Seed und öffnet die
Webvorschau. **Logs** in der oberen Leiste zeigt die lokalen JSONL-Protokolle
mit Stufenfilter und Textsuche.

Diese Prüf- und Erkundungsbefehle laufen im Projektverzeichnis:

~~~sh
zelyra check main.zyl
zelyra fmt main.zyl --check
zelyra routes main.zyl
zelyra db map main.zyl
zelyra db plan main.zyl --format json
~~~

`check` kompiliert das importierte Projekt. `fmt` formatiert den Quelltext;
`--check` meldet nur Abweichungen und schreibt nichts. `routes` listet
deklarierte und generierte URLs samt Quelldatei. `db map` zeigt Tabellen und
Modulzuordnung. `db plan` vergleicht Quellmodell und Live-Schema, ohne es zu
ändern; `--format json` liefert maschinenlesbare Ausgabe.

Eine geprüfte Schemaänderung wird erst nach Sichtprüfung angewendet:

~~~sh
zelyra db apply main.zyl --plan-id sha256:<plan-id>
~~~

`--plan-id` bindet die Anwendung an die zuvor angezeigte Plan-ID. Bei riskanten
Änderungen ist zusätzlich `--allow-risky` nötig; der Befehl zeigt dann eine
Bestätigung. Führe die Anwendung nicht aus, ohne den Plan zu prüfen.

## Wo `/invoices/new` entsteht

Die Route steht nicht als `page "/invoices/new"` im Quelltext. Sie entsteht
aus `crud Invoice -> invoices` in `src/invoices.zyl`. `crud` fordert Zelyra
auf, die üblichen Listen-, Formular-, Detail- und Änderungswege für die
Tabelle `invoices` zu erzeugen. Der Ressourcenname rechts vom Pfeil wird zum
URL-Basispfad:

- `GET /invoices` zeigt die Liste.
- `GET /invoices/new` zeigt das Formular.
- `POST /invoices/new` speichert die neue Rechnung.
- `GET /invoices/{id}/edit` und `POST /invoices/{id}/edit` bearbeiten einen
  vorhandenen Datensatz.

Die Zeile `page "/invoices/{invoice_number}/print"` in derselben Quelldatei
ist dagegen eine ausdrücklich deklarierte Route. `zelyra routes main.zyl`
zeigt beide Arten samt Quelle. `/views/invoicedashboard` stammt aus
`tableview InvoiceDashboard` in `src/dashboard.zyl`; die URL bleibt aus
Kompatibilitätsgründen erhalten, während der Seitentitel „Rechnungsübersicht“
lautet.

## Rechnungsdatum und Rechnungsnummer

`issued_at` ist in Struktur und Datenbanktabelle als `Date` deklariert. Der
Browser zeigt damit ein Datumseingabefeld, keine Uhrzeit. Die Hilfe nennt
`JJJJ-MM-TT` auf Deutsch und `YYYY-MM-DD` auf Englisch; genau dieses Format
akzeptiert die Validierung. Das Datenbankschema speichert nur den Kalendertag.

Beim Erstellen ist `invoice_number` schreibgeschützt. Der Server nimmt einen
zufälligen temporären Wert entgegen, fügt die Rechnung in derselben
Datenbanktransaktion ein, liest ihre automatisch erzeugte ID aus und setzt
`RE-<ID>` als endgültige Nummer. Bestehende Nummern werden geprüft und bei
Bedarf durch ein Suffix eindeutig gemacht. Bei späteren Änderungen wird die
Nummer nicht in das Bearbeitungsformular aufgenommen und kann dort nicht
geändert werden. Die Beispieldaten behalten `RE-2026-001`.

## Quelltext nach Dateien und Zeilen

### `main.zyl` — Programmeinstieg und Startseite

- Zeile 1 erklärt, dass diese Datei die Fachmodule verbindet und die Startseite enthält.
- Zeile 2 importiert die compilergeprüfte Standardbibliothek `zelyra/ui`; die Komponenten werden in HTML über ihren Namen verwendet. Zeilen 3–8 importieren Datenbank, Layout, Kunden, Artikel, Rechnungen und Übersicht. Die Namen nach `as` sind lokale Modul-Aliase.
- Zeilen 9–11 deklarieren `GET /`, wählen `InvoiceTutorialShell` und beginnen die HTML-Ausgabe. Die Route `/` ist hier ausdrücklich festgelegt.
- Zeile 12 öffnet den Hauptbereich. Zeilen 13–14 beginnen das zweispaltige Hero- und Beispielrechnungs-Layout.
- Zeilen 15–19 bilden Kennzeichnung, Z-Marke und lokale Statusplakette. Die Beschriftungen mit `data-zelyra-i18n` werden aus den Sprachdateien ersetzt.
- Zeile 20 setzt die übersetzte Hauptüberschrift; Zeile 21 den Einführungstext.
- Zeilen 22–25 bilden Aktionsbereich und Links: `/invoices/new` startet die Rechnungserstellung, `/views/invoicedashboard` öffnet die Übersicht.
- Zeilen 26–30 zeigen den lokalen Datenhinweis und schließen den Hero-Bereich.
- Zeilen 32–39 beginnen die Beispielrechnung: Ihre sichtbare Kennung `RE-2026-001` entspricht dem Seed; die Beschriftungen und der Entwurfsstatus sind übersetzt.
- Zeilen 40–45 trennen den Kopf von den fest eingetragenen Empfängerdaten. Die `.test`-Adresse ist ein Beispieldatensatz.
- Zeilen 46–55 zeigen Ausstellungsdatum und Umsatzsteuersatz in zwei Spalten. Datum und Beschriftungen sind für Deutsch und Englisch lokalisiert.
- Zeilen 56–71 stellen die zwei Beispielpositionen mit Menge, Einzelpreis und Zeilensumme dar.
- Zeilen 72–76 zeigen Nettosumme, 19-prozentige Umsatzsteuer und Bruttobetrag. Die Werte entsprechen den Seed-Positionen (298,00 € netto, 56,62 € Steuer, 354,62 € brutto).
- Zeilen 77–79 verlinken zur tatsächlichen Druckansicht der Beispielrechnung und schließen den oberen Rasterbereich.
- Zeilen 81–103 zeigen drei Funktionshinweise zu Datumseingabe, automatischer Nummer und Druckansicht; `<article>` gruppiert jeden Hinweis, `<strong>` setzt den Titel und `<p>` den erklärenden Satz.
- Zeilen 105–112 setzen Überschrift, Untertitel und Link der nächsten Sektion.
- Zeilen 114–142 bilden mit `Grid columns="three"` die drei Schritte Kunde, Artikel und Rechnung. Die Komponente verwaltet die Spalten responsiv; die Links führen zu `/customers/new`, `/items/new` und `/invoices/new`.
- Zeilen 143–146 schließen Ablaufsektion, Startseite, HTML-Ausgabe und Route.
- Zeilen 148–155 deklarieren `/ui/components` als Komponentenübersicht und zeigen eine Warnung, dass Darstellung keine Servervalidierung ersetzt.
- Zeilen 156–180 demonstrieren die vier compilergeprüften Erscheinungsvarianten, normale und Formularbuttons, Hinweisslot, tatsächlichen Fehlerzustand und ein natives GET-Formular.
- Zeilen 182–209 zeigen Datums-, Zahlen-, Auswahl-, Text-, Radio-, Checkbox- und Switch-Felder. Erscheinungsvarianten verändern nur die Darstellung; `invalid` bleibt eine eigene Angabe.
- Zeilen 210–215 zeigen eine beschriftete responsive Tabelle mit Spalten- und Zeilenüberschriften sowie einer echten Detailnavigation.
- Zeilen 216–229 zeigen serverseitig gerenderte Balken-, Ring- und Liniendiagramme. Die Linie erhält eine lesbare Tabellenalternative.
- Zeilen 230–259 demonstrieren Fortschritt, Breadcrumbs, Pagination, Leer-/Lade-/Fehlerzustände, Toast, Disclosure, Dropdown, Tastaturtabs und Dialog.
- Zeilen 260–269 erklären Browserdruck/PDF, den lokalen E-Mail-Entwurf mit vorausgefüllter Rechnungsnummer und die Druckaktion für die Übersicht. Speichere die Rechnung zuerst als PDF und hänge sie im Entwurf manuell an. Die E-Mail-Schaltfläche versendet nicht selbst; Servermail verwendet eine autorisierte Route, `mail_queue` und den Outbox-Worker. Die Outbox kann noch keine Anhänge versenden.
- Zeilen 270–272 schließen Komponentenübersicht und Route.
- Zeile 274 deklariert `page "/tour"` als interaktive Architektur- und Code-Tour mit einfahrbaren Quelltext-Blöcken (`Disclosure`), Code-Beispielen für alle Kernmodule und einer Gegenüberstellung zu klassischen Framework-Stacks.

### `src/database.zyl` — gemeinsame Verbindung

- Zeile 1 erklärt, warum alle Module dieselbe Deklaration importieren.
- Zeile 2 beginnt die Datenbankdeklaration `main`.
- Zeile 3 wählt MariaDB als Backend.
- Zeile 4 beendet die Deklaration. Zugangsdaten kommen aus der lokalen `.env`,
  nicht aus dieser Datei.

### `src/customers.zyl` — Kunden

- Zeilen 1–2 importieren die Datenbank und das gemeinsame HTML-Layout.
- Zeilen 4–8 definieren den öffentlichen Datensatz `Customer`: `id` ist die
  Datenbankkennung, `name` Text und `email: Email?` eine optionale E-Mail.
- Zeilen 10–17 deklarieren `customers`. `id` wird automatisch erzeugt; `name`
  ist höchstens 120 Zeichen lang und erforderlich; E-Mail darf fehlen.
  `access.read` gibt den Modulen an, die Kundendaten für ihre Abfragen lesen
  dürfen.
- Zeilen 19–30 erzeugen mit `crud Customer -> customers` die Kundenverwaltung.
  Das Layout und der Titel bestimmen die Darstellung. `view.fields` begrenzt
  sichtbare Felder; `mode: cards` verwendet Karten für Liste, Detail und
  Formulare. `list` legt Tabellenfelder fest, `search` die durchsuchbaren
  Felder.

### `src/items.zyl` — Artikel

- Zeilen 1–2 importieren Datenbank und Layout.
- Zeilen 4–10 definieren `Item` mit ID, Artikelnummer `sku`, Name, Nettopreis
  als exaktem `Decimal` und Aktiv-Status als `Bool`.
- Zeilen 12–21 deklarieren die Tabelle. `sku` ist erforderlich, maximal 40
  Zeichen lang und eindeutig; `name` ist erforderlich und maximal 120 Zeichen.
  Der Preis ist ein Pflichtfeld, `active` beginnt standardmäßig mit `true`.
  Die Lesezugriffe erlauben Rechnungsformular und Übersicht den Artikelbezug.
- Zeilen 23–35 erzeugen Artikel-CRUD. Die Feldlisten steuern Formular, Liste,
  Suche und Filter; `active` dient als boolescher Filter.

### `src/shell.zyl` — gemeinsames Layout

- Zeile 1 importiert Zelyra UI für dieses Modul; Zeile 3 erläutert die gemeinsame Verwendung.
- Zeile 4 deklariert `InvoiceTutorialShell` als öffentliche View.
- Zeilen 6–11 öffnen ein HTML-Dokument mit `data-z-ui`, UTF-8, Mobilviewport und lokalisiertem Titel.
- Zeilen 13–14 öffnen den Dokumentkörper und `AppShell`. `id` verbindet den Navigationsknopf mit der Navigation; die Beschriftung ist zugleich zugänglich.
- Zeilen 15–21 füllen den benannten Header-Slot mit Marke und Link zum Rechnungsformular.
- Zeilen 22–29 füllen den Navigationsslot mit echten Links zu Startseite, Kunden, Artikeln, Rechnungen, Positionen und Komponentenübersicht. `data-zelyra-i18n` übersetzt Beschriftungen; die Routen bleiben gleich.
- Zeile 30 setzt den Standardinhaltsslot in den Projektbereich; dadurch teilen Handbuchseiten und generierte CRUD-Seiten dasselbe Layout.
- Zeilen 31–33 setzen den Fußbereich. Zeilen 34–38 schließen AppShell, Body, HTML, View-Block und View.

### `src/invoices.zyl` — Rechnungen und Druckansicht

- Zeilen 1–4 importieren Datenbank, Kunden, Artikel und Layout.
- Zeilen 6–12 definieren `Invoice`: Nummer, Kunde, reines Datum und Status.
- Zeilen 14–20 definieren `InvoiceLine`: Rechnung, Artikel, Stückzahl und
  Einzelpreis bilden eine Rechnungsposition.
- Zeilen 22–31 deklarieren `invoices`. Die ID wird automatisch erzeugt;
  `invoice_number` ist erforderlich und eindeutig; `customer` ist erforderlich;
  `issued_at` ist ein erforderliches Datum mit aktuellem Tagesdatum als
  Vorgabewert; neue Rechnungen beginnen mit Status `draft`. Der Zugriff erlaubt
  die Druckansicht, Übersicht und Startseite.
- Zeilen 33–42 deklarieren `invoice_lines` und die erforderlichen Bezüge zu
  Rechnung und Artikel. Menge und Preis werden je Position gespeichert.
- Zeilen 44–56 erzeugen CRUD-Routen für Rechnungen. Zeile 46 setzt den
  lokalisierten Titel. Die Feldlisten steuern Formular, Detail und Suche.
  Zelyra markiert die Nummer im Formular als automatisch vergeben. Das
  Bearbeitungsformular lässt sie weg; bestehende Nummern bleiben damit erhalten.
- Zeilen 58–68 erzeugen CRUD-Routen für Positionen.
- Zeilen 70–79 beschreiben die Ergebnisspalten der Druckabfrage, einschließlich
  Netto, 19-%-Steuer und Bruttosumme.
- Zeilen 81–87 beschreiben die Artikeldaten jeder Druckzeile und ihren
  berechneten Nettobetrag.
- Zeilen 89–106 deklarieren ausdrücklich die Druckroute. Der Pfadparameter
  `invoice_number` bindet die Abfrage an genau eine Rechnung. `JOIN` liest den
  Kundennamen; `LEFT JOIN` lässt auch Rechnungen ohne Positionen erscheinen.
  `COALESCE` macht fehlende Summen zu null, `SUM` addiert Positionen und
  `ROUND(..., 2)` rundet die sichtbaren Beträge auf Cent. `GROUP BY` fasst die
  Positionen je Rechnung zusammen.
- Zeilen 107–119 laden die zugehörigen Positionen. Die Joins liefern Artikel
  und Rechnung zusammen; `ORDER BY` hält die Ausgabe stabil.
- Zeilen 120–141 bilden den Druckinhalt. Die Schaltfläche ruft den Browserdruck
  auf; `no-print` blendet sie beim Drucken aus. Die Tabellenzeilen laufen über
  `lines`. Die Summenkarte weist Netto, Steuer und Brutto getrennt aus.
- Zeile 142 schließt die Druckseite.

### `src/dashboard.zyl` — filterbare Rechnungsliste

- Zeilen 1–5 importieren Datenbank, Kunden, Artikel, Rechnungen und Layout.
- Zeilen 7–15 typisieren die gelesenen Spalten einschließlich berechneter
  Dezimalbeträge.
- Zeilen 17–18 deklarieren die Tabelle `InvoiceDashboard` und den typisierten
  SQL-Datenstrom. Zeile 18 bindet die Übersicht an `InvoiceTutorialShell`, damit
  Kopf, Navigation, Inhalt und Fußzeile dem restlichen Tutorial entsprechen.
  Aus dem Namen entsteht `/views/invoicedashboard`.
- Zeilen 19–32 wählen Rechnungsnummer, Kunde und Status. Die Aggregatfunktionen
  berechnen Netto, Steuer und Brutto. Rechnungen ohne Positionen bleiben durch
  `LEFT JOIN` sichtbar.
- Zeile 33 schließt den SQL-Abfrageblock.
- Zeilen 34–36 bestimmen die angezeigten Spalten.
- Zeile 37 bietet den Statusfilter, Zeile 38 aktiviert Suche, Zeile 39 die
  Sortierung und Zeile 40 begrenzt die Ergebnisse auf 20 je Seite.
- Zeile 41 schließt die Tabellenansicht.
- Zeile 39 schließt die TableView.

### `invoice-tutorial.seed.sql` — Beispieldaten

- Zeilen 1–2 dokumentieren, dass stabile E-Mail-Adressen, Artikelnummern und
  Rechnungsnummer den Seed wiederholbar machen.
- Zeilen 3–7 fügen `Beispiel GmbH` nur ein, falls ihre E-Mail noch fehlt.
- Zeilen 9–13 fügen auf dieselbe Weise `Nordlicht Büro` ein.
- Zeilen 15–19 fügen den Beratungsartikel hinzu, wenn seine SKU noch nicht
  existiert.
- Zeilen 21–25 ergänzen den Wartungsartikel.
- Zeilen 27–31 ergänzen das Arbeitsplatzpaket.
- Zeilen 33–39 fügen einen Rechnungsentwurf für den ersten Kunden ein. Das
  Datum enthält nur Jahr, Monat und Tag; `NOT EXISTS` verhindert ein Duplikat.
- Zeilen 41–49 verknüpfen zwei Beratungsartikel mit dem Entwurf, falls diese
  Position noch fehlt.
- Zeilen 51–59 fügen eine Wartungsposition hinzu. Jede `INSERT ... SELECT`
  verwendet gespeicherte Schlüssel und die `NOT EXISTS`-Bedingung, daher
  erzeugt ein weiterer Start keine Kopien.

### `zelyra.theme.css` — lokale Gestaltung

- Zeile 1 erklärt: Dies ist ein Theme-Override von Zelyra UI und kein zweites CSS-Framework.
- Zeilen 2–27 passen im begrenzten `html[data-z-ui]`-Bereich die Geschäftsfarben, Kontrastfarben, Flächen, Fokus, Inhaltsbreite und Rundungen an.
- Zeilen 28–42 gestalten Marke und Abstände für Tutorialkarten, Komponentenübersicht und Druckaktionen.
- Zeilen 43–60 bauen Startseitenrahmen und Hero mit Geschäftsmarke, Handlungslinks und lokalem Beispiel auf.
- Zeilen 61–83 gestalten die Beispielrechnung als Papierkarte mit Empfänger, Datum, Positionen, Summen und Link.
- Zeilen 84–105 gestalten Funktionshinweise und die drei Workflowkarten.
- Zeilen 106–114 passen Abschnittsabstände, Formular-/Tabellenbereiche und Rechnungssummen an.
- Zeilen 116–135 ordnen Hero und Karten für Tablet und schmale Mobilansichten um; bei 600 px wird der Ablauf einspaltig.
- Zeilen 136–191 passen Designvariablen und Beispielkarten an ein explizites dunkles Theme sowie an die dunkle Systempräferenz an.
- Zeilen 192–209 sind Druckregeln: Kopf/Fuß und `z-no-print`-Aktionen verschwinden; Rechnungstabellen bleiben druckbar und Karten verlieren Schatten.

### `locales/de.json` und `locales/en.json` — lokalisierte Beschriftungen

Beide Dateien enthalten dieselben Schlüssel in derselben Reihenfolge. Die Zeilenerklärung gilt für beide: `de.json` ordnet jeden stabilen Schlüssel dem deutschen und `en.json` dem englischen Anzeigetext zu. `main.zyl`, das Layout und generierte Formulare verweisen auf diese Schlüssel; die Werte verändern weder Routen noch Datenbankzugriff.

- Zeile 1 öffnet das JSON-Objekt; Zeile 61 schließt es.
- Zeile 2 (`invoice.number.label`) beschriftet das Eingabefeld der Rechnungsnummer.
- Zeile 3 (`invoice.number.auto_hint`) erklärt die automatische Vergabe beim Speichern.
- Zeile 4 (`invoice.issued_at.label`) beschriftet das Ausstellungsdatum.
- Zeile 5 (`invoice.overview.title`) setzt den Titel der Rechnungsübersicht.
- Zeile 6 (`invoice.nav.document_title`) setzt den Titel im Browser-Tab.
- Zeile 7 (`invoice.nav.subtitle`) setzt den Untertitel neben der Marke.
- Zeile 8 (`invoice.nav.home`) beschriftet den Startseitenlink.
- Zeile 9 (`invoice.nav.customers`) beschriftet den Kundenlink.
- Zeile 10 (`invoice.nav.items`) beschriftet den Artikellink.
- Zeile 11 (`invoice.nav.invoices`) beschriftet den Übersichtslink.
- Zeile 12 (`invoice.nav.lines`) beschriftet den Rechnungspositionslink.
- Zeile 13 (`invoice.nav.new_invoice`) beschriftet die Schaltfläche für neue Rechnungen.
- Zeile 14 (`invoice.nav.footer`) liefert den Hinweis im Fußbereich.
- Zeile 15 (`invoice.home.eyebrow`) liefert die kleine Kennzeichnung über der Überschrift.
- Zeile 16 (`invoice.home.title`) liefert die Hauptüberschrift der Startseite.
- Zeile 17 (`invoice.home.intro`) erklärt den Beispielablauf.
- Zeile 18 (`invoice.home.new_invoice`) beschriftet die primäre Aktion.
- Zeile 19 (`invoice.home.overview`) beschriftet den Link zur Rechnungsübersicht.
- Zeile 20 (`invoice.home.workflow`) liefert den Namen des Rechnungsablaufs.
- Zeile 21 (`invoice.home.customer_title`) benennt Schritt und Karte für Kunden.
- Zeile 22 (`invoice.home.customer_text`) erklärt den Kundenschritt.
- Zeile 23 (`invoice.home.customer_link`) beschriftet den Link zum Kundenformular.
- Zeile 24 (`invoice.home.item_title`) benennt Schritt und Karte für Artikel.
- Zeile 25 (`invoice.home.item_text`) erklärt den Artikelschritt.
- Zeile 26 (`invoice.home.item_link`) beschriftet den Link zum Artikelformular.
- Zeile 27 (`invoice.home.invoice_title`) benennt Schritt und Karte für Rechnungen.
- Zeile 28 (`invoice.home.invoice_text`) erklärt Auswahl des Kunden und Nummernvergabe.
- Zeile 29 (`invoice.home.invoice_link`) beschriftet den Link zum Rechnungsformular.
- Zeile 30 (`invoice.home.local_badge`) kennzeichnet die lokale, startbereite Beispielanwendung.
- Zeile 31 (`invoice.home.proof`) erklärt, dass die Daten lokal im Projekt bleiben.
- Zeile 32 (`invoice.home.sample_label`) kennzeichnet die Rechnung als Beispiel.
- Zeile 33 (`invoice.home.sample_status`) zeigt den Entwurfsstatus der Beispielrechnung.
- Zeile 34 (`invoice.home.billed_to`) kennzeichnet den Rechnungsempfänger.
- Zeile 35 (`invoice.home.issue_date`) kennzeichnet das Ausstellungsdatum.
- Zeile 36 (`invoice.home.tax_rate`) kennzeichnet den Umsatzsteuersatz.
- Zeile 37 (`invoice.home.tax_rate_value`) zeigt den in der Druckansicht verwendeten Steuersatz.
- Zeile 38 (`invoice.home.sample_date`) zeigt das lokalisierte Beispieldatum.
- Zeile 39 (`invoice.home.sample_item_1`) benennt den ersten Beispielartikel.
- Zeile 40 (`invoice.home.sample_item_1_detail`) zeigt Menge und Nettopreis je Einheit des ersten Artikels.
- Zeile 41 (`invoice.home.sample_amount_1`) zeigt die Nettosumme des ersten Artikels.
- Zeile 42 (`invoice.home.sample_item_2`) benennt den zweiten Beispielartikel.
- Zeile 43 (`invoice.home.sample_item_2_detail`) zeigt Menge und Nettopreis je Einheit des zweiten Artikels.
- Zeile 44 (`invoice.home.sample_amount_2`) zeigt die Nettosumme des zweiten Artikels.
- Zeile 45 (`invoice.home.net_total`) beschriftet die Nettosumme.
- Zeile 46 (`invoice.home.sample_net`) zeigt den Nettobetrag der Beispielrechnung.
- Zeile 47 (`invoice.home.tax_total`) beschriftet Umsatzsteuer und Steuersatz.
- Zeile 48 (`invoice.home.sample_tax`) zeigt den Steuerbetrag der Beispielrechnung.
- Zeile 49 (`invoice.home.gross_total`) beschriftet den fälligen Bruttobetrag.
- Zeile 50 (`invoice.home.sample_gross`) zeigt den Bruttobetrag der Beispielrechnung.
- Zeile 51 (`invoice.home.open_sample`) beschriftet den Link zur Druckansicht.
- Zeile 52 (`invoice.home.feature_date_title`) benennt den Datums-Hinweis.
- Zeile 53 (`invoice.home.feature_date_text`) erklärt die reine Datumseingabe ohne Uhrzeit.
- Zeile 54 (`invoice.home.feature_number_title`) benennt die automatische Nummernvergabe.
- Zeile 55 (`invoice.home.feature_number_text`) erklärt, wann die Nummer entsteht.
- Zeile 56 (`invoice.home.feature_print_title`) benennt die Druckfunktion.
- Zeile 57 (`invoice.home.feature_print_text`) erklärt die berechneten Summen.
- Zeile 58 (`invoice.home.workflow_kicker`) kennzeichnet die Ablaufsektion als drei Schritte.
- Zeile 59 (`invoice.home.workflow_title`) setzt deren Hauptüberschrift.
- Zeile 60 (`invoice.home.all_invoices`) beschriftet den Link zu allen Rechnungen.
