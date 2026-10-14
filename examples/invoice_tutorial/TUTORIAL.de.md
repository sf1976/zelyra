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

### `main.zyl` — Projektkarte und Startseite

- Zeile 1 benennt die Aufgabe dieser Einstiegsdatei.
- Zeilen 2–7 importieren Datenbank, Layout, Kunden, Artikel, Rechnungen und
  Rechnungsübersicht. Die Aliasnamen machen die Module im Projektgraphen
  erreichbar; die fachlichen Deklarationen bleiben in `src/`.
- Zeilen 9–11 deklarieren `GET /` und wählen `InvoiceTutorialShell` als
  gemeinsamen Rahmen.
- Zeile 12 beginnt den Seiteninhalt. Zeilen 13–21 bilden den hervorgehobenen
  Startbereich. `data-zelyra-i18n` bindet Überschrift, Beschreibung und Links
  an die zweisprachigen Einträge in `locales/de.json` und `locales/en.json`.
- Zeilen 23–33 bilden die Kundenkarte. Die Klasse `col-md-4` sorgt für drei gleich breite Karten
  auf mittleren und großen Bildschirmen. Zeilen 27–30 zeigen Kennzeichnung,
  Überschrift, Erklärung und Link zu `/customers/new`.
- Zeilen 34–43 bilden dieselbe Struktur für Artikel und führen zu
  `/items/new`.
- Zeilen 44–53 bilden die Rechnungskarte und führen zu `/invoices/new`.
- Zeilen 54–57 schließen Kartenabschnitt, Hauptinhalt, HTML-Ausgabe und
  Seitendeklaration. Die öffnenden und schließenden Tags gehören paarweise
  zusammen.

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

- Zeile 1 beschreibt das Layout.
- Zeile 2 deklariert eine öffentliche View namens `InvoiceTutorialShell`.
- Zeilen 3–4 beginnen HTML-Ausgabe und HTML-Dokument.
- Zeilen 5–10 setzen Sprach-/Dokumentmetadaten, Mobilansicht und Seitentitel.
- Zeilen 11–18 beginnen den Dokumentkörper und Markenbereich. Das Logo ist
  reiner Text; der Untertitel wird aus `locales/de.json` oder `locales/en.json`
  gelesen.
- Zeilen 19–25 bilden die Navigation zu Startseite, Kunden, Artikeln,
  Rechnungsübersicht und Positionen. `data-zelyra-i18n` übersetzt die Labels;
  `href` bleibt jeweils der konkrete URL-Pfad.
- Zeile 26 fügt den auffälligen Link für eine neue Rechnung hinzu.
- Zeile 27 schließt den inneren Navigationscontainer; Zeile 28 schließt den
  Kopfbereich.
- Zeilen 29–31 bilden den Hauptbereich. `<slot />` ist der Platzhalter, in den
  Zelyra den Inhalt der Seite oder der generierten CRUD-Ansicht einsetzt.
- Zeilen 32–34 bilden den lokalisierten Fußbereich.
- Zeilen 35–39 schließen HTML-Körper, Dokument, HTML-Block und View.

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
  SQL-Datenstrom. Aus diesem Namen entsteht `/views/invoicedashboard`.
- Zeilen 19–30 wählen Rechnungsnummer, Kunde und Status. Die Aggregatfunktionen
  berechnen Netto, Steuer und Brutto. Rechnungen ohne Positionen bleiben durch
  `LEFT JOIN` sichtbar.
- Zeile 31 schließt den SQL-Abfrageblock.
- Zeilen 32–34 bestimmen die angezeigten Spalten.
- Zeile 35 bietet den Statusfilter, Zeile 36 aktiviert Suche, Zeile 37 die
  Sortierung und Zeile 38 begrenzt die Ergebnisse auf 20 je Seite.
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

### `zelyra.theme.css` — lokales Layout

- Zeile 1 aktiviert die lokal mitgelieferte Tabler-Grundgestaltung.
- Zeile 2 hält fest, dass keine CDN-Anfrage nötig ist.
- Zeilen 3–12 setzen helle Darstellung, Primärfarbe, Hintergrund, Text,
  Kartenradius und Schatten.
- Zeilen 14–21 formatieren Seitenhintergrund, Standardabstände und gut lesbare
  Systemschrift.
- Zeilen 23–28 entfernen das IDE-Raster aus der eigenständigen Web-App.
- Zeilen 30–38 setzen den äußeren Seitenbereich zurück, damit Tabler das Layout
  steuern kann.
- Zeilen 40–50 gestalten Seitenrahmen, Kopfzeile, Marke und Navigation.
- Zeile 51 setzt den Innenabstand des Inhaltsbereichs.
- Zeilen 52–55 gestalten den Startseitenkopf samt Überschrift, Beschreibung und
  Aktionslinks.
- Zeilen 56–59 gestalten Ablaufkarten, Überschriften, Beschreibungen und Links.
- Zeile 60 formatiert den Fußbereich.
- Zeilen 61–65 vereinheitlichen Abstände, Formulare, Tabellen und Fokus.
- Zeilen 66–68 gestalten die Netto-/Steuersummen auf der Druckansicht.
- Zeilen 70–83 passen Navigation und Inhalt für schmalere Bildschirme an.
- Zeilen 84–88 blenden beim Drucken Navigation und Fußbereich aus und nehmen
  unnötige Innenabstände sowie Schatten weg.

### `locales/de.json` und `locales/en.json` — übersetzte Beschriftungen

Beide JSON-Dateien haben denselben Aufbau mit 30 Zeilen. Jeder Schlüssel ist
in beiden Dateien gleich und steht in derselben Zeile; nur der angezeigte
deutsche oder englische Wert unterscheidet sich. Die Schlüssel verbinden die
Übersetzungen mit `data-zelyra-i18n` und den lokalisierten CRUD-Feldern. Die
Werte ändern weder Routen noch Datenbankverhalten.

- Zeile 1 öffnet das JSON-Objekt. Zeile 30 schließt es.
- Zeile 2 (`invoice.number.label`) beschriftet die Rechnungsnummer.
- Zeile 3 (`invoice.number.auto_hint`) erklärt, dass der Server sie beim
  Speichern vergibt.
- Zeile 4 (`invoice.issued_at.label`) beschriftet das Ausstellungsdatum.
- Zeile 5 (`invoice.overview.title`) benennt die Rechnungsübersicht.
- Zeile 6 (`invoice.nav.document_title`) setzt den Titel im Browserfenster.
- Zeile 7 (`invoice.nav.subtitle`) setzt den Untertitel der Navigation.
- Zeile 8 (`invoice.nav.home`) beschriftet den Startseitenlink.
- Zeile 9 (`invoice.nav.customers`) beschriftet den Kundenlink.
- Zeile 10 (`invoice.nav.items`) beschriftet den Artikellink.
- Zeile 11 (`invoice.nav.invoices`) beschriftet den Link zur Rechnungsübersicht.
- Zeile 12 (`invoice.nav.lines`) beschriftet den Link zu den Positionen.
- Zeile 13 (`invoice.nav.new_invoice`) beschriftet die Schaltfläche für eine
  neue Rechnung.
- Zeile 14 (`invoice.nav.footer`) liefert den Fußbereichstext.
- Zeile 15 (`invoice.home.eyebrow`) liefert die kleine Einleitung über der
  Startseitenüberschrift.
- Zeile 16 (`invoice.home.title`) liefert die Hauptüberschrift der Startseite.
- Zeile 17 (`invoice.home.intro`) erklärt den Ablauf in einem Satz.
- Zeile 18 (`invoice.home.new_invoice`) beschriftet die Aktion zum Erstellen.
- Zeile 19 (`invoice.home.overview`) beschriftet die Aktion zur Übersicht.
- Zeile 20 (`invoice.home.workflow`) benennt den Ablaufbereich.
- Zeile 21 (`invoice.home.customer_title`) benennt die Kundenkarte.
- Zeile 22 (`invoice.home.customer_text`) erklärt den Kundenschritt.
- Zeile 23 (`invoice.home.customer_link`) beschriftet den Link zum Formular.
- Zeile 24 (`invoice.home.item_title`) benennt die Artikelkarte.
- Zeile 25 (`invoice.home.item_text`) erklärt den Artikelschritt.
- Zeile 26 (`invoice.home.item_link`) beschriftet den Link zum Formular.
- Zeile 27 (`invoice.home.invoice_title`) benennt die Rechnungskarte.
- Zeile 28 (`invoice.home.invoice_text`) erklärt Rechnungserstellung und
  automatische Nummernvergabe.
- Zeile 29 (`invoice.home.invoice_link`) beschriftet den Link zum Formular.
