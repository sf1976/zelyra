# Rechnungstutorial

Das mehrteilige Beispiel enthält Kunden, Artikel, Rechnungen, Rechnungspositionen,
eine filterbare Rechnungsübersicht und eine druckbare Rechnung. Das vollständige
zweisprachige Befehls- und Quelltextkapitel steht im
[generierten Projektleitfaden](../../examples/invoice_tutorial/TUTORIAL.de.md);
`zelyra --tutorial invoice` legt beide Sprachfassungen als `TUTORIAL.de.md` und
`TUTORIAL.en.md` im Projektordner ab.

## Voraussetzungen und Start

Erforderlich sind Docker Engine und das Compose-Plugin v2 oder neuer. Prüfe
beides mit `docker compose version`. Python Compose v1 wird nicht unterstützt;
Zelyra installiert Docker nicht.

~~~sh
zelyra --tutorial invoice
~~~

Der Befehl erzeugt `invoice-tutorial`, schreibt lokale Zugangsdaten in die
ignorierte `.env`, richtet MariaDB und Schema ein, fügt Beispieldaten hinzu und
startet die Webanwendung. Öffne die ausgegebene URL, falls kein Browser startet.
Die Daten umfassen zwei Kunden, drei Artikel, einen Rechnungsentwurf und zwei
Positionen. Wiederholte Starts fügen sie nicht doppelt hinzu.

Nur die Dateien erzeugen:

~~~sh
zelyra --tutorial invoice ./meine-rechnungen --no-start
cd ./meine-rechnungen
zelyra setup . --all
~~~

`--no-start` überspringt Docker und Webstart, schreibt aber `.env`, Compose-,
Schema-, Seed- und Anwendungsdateien. Später führt `zelyra setup . --all` den
kompletten lokalen Start aus. Das Tutorial wählt freie Folgeports, wenn die
verwalteten Anwendungs- oder MariaDB-Ports belegt sind; die neue URL wird
ausgegeben.

## Start aus Studio und Bearbeitung

~~~sh
zelyra editor ./invoice-tutorial
~~~

Öffne `main.zyl` im Dateibaum und klicke **Starten**. Studio startet die zum
Tutorial gehörende lokale Datenbank, wartet auf MariaDB, richtet das Schema
ein, lädt den wiederholbaren Seed und öffnet dann die Webvorschau. Mit **Stop**
beendest du die Vorschau. **Logs** in der Studio-Kopfzeile zeigt Programm- und
Vorschauausgaben als lokale, rotierende JSONL-Protokolle; sie sind durch
Projektfilter und Textsuche abrufbar.

Die Startseite `/` führt zu den Schritten Kunde, Artikel und Rechnung. Die
filterbare Tabelle heißt **Rechnungsübersicht** und bleibt unter
`/views/invoicedashboard` erreichbar. `layout: InvoiceTutorialShell` bindet
auch diese automatisch gerenderte Ansicht in denselben Kopf-, Navigations-,
Inhalts- und Fußbereich ein wie die übrigen Tutorial-Seiten. `GET /invoices/new` wird aus
`crud Invoice -> invoices` in `src/invoices.zyl` generiert; es gibt dafür keine
separate `page`-Zeile. `zelyra routes main.zyl` zeigt deklarierte und erzeugte
Routen samt Quelldatei.

Auch die Darstellung bleibt durchgehend gleich: Zelyra UI stellt den gemeinsamen
Rahmen, CRUD-Karten, Filter, Formulare und Tabellen zentral bereit. Das
Projekt-Theme setzt die Farben und ergänzt nur tutorialeigene Marke,
Startseite und Druckansicht. `examples/invoice_tutorial.theme.css` und die
Projektdatei `zelyra.theme.css` bleiben als Projekt-Themevorlage synchron.

Das Feld **Ausgestellt am** verwendet `Date` und nimmt nur ein Datum an. Die
lokalisierte Formatangabe lautet `JJJJ-MM-TT` beziehungsweise `YYYY-MM-DD`.
Die Rechnungsnummer wird serverseitig aus der neu erzeugten ID gebildet und
bleibt bei späteren Änderungen unverändert. `TUTORIAL.de.md` und
`TUTORIAL.en.md` erklären jeden Quellabschnitt und die Befehlsoptionen.

## Zelyra UI, Druck, Diagramme und E-Mail

Das Projekt importiert `zelyra/ui` und verwendet dieselbe `AppShell`-View für
Tutorialseiten und erzeugte CRUD-Ansichten. Öffne `/ui/components`, um die
compilergeprüften Varianten `default`, `success`, `warning` und `danger`, native
Formularfelder, die responsive Datentabelle, Balken-/Linien-/Ringdiagramme,
Tabs, Dialog, Druck und E-Mail-Entwurf zu sehen. `appearance` ist nur die
Darstellung; tatsächliche Feldfehler setzt die Servervalidierung.

Auf `/invoices/{invoice_number}/print` öffnet **Drucken** den Browserdialog.
Dort kannst du **Als PDF speichern** auswählen. Zelyra erzeugt derzeit keine
eigene PDF-Datei; Kopfzeile, Navigation und Druckaktionen werden im Druckmodus
ausgeblendet. Die `EmailLink`-Aktion öffnet einen vorausgefüllten Entwurf im
lokalen Mailprogramm. Hänge die gespeicherte PDF-Datei dort manuell an. Der
Link versendet die Nachricht nicht selbst. Serverseitiger Versand benötigt
eine ausdrücklich autorisierte Route mit `mail_queue` und Outbox-Worker; die
Outbox unterstützt derzeit keine Dateianhänge.

Die eingebauten CSS-/JavaScript-Dateien werden lokal und versioniert aus dem
Zelyra-Binary ausgeliefert. Die optionale Projektdatei `zelyra.theme.css`
überschreibt zuletzt Variablen und Tutorialdetails. Mit
`ZELYRA_UI_STYLESHEET=tabler` oder einem sicheren relativen `.css`-Pfad kann das
integrierte Stylesheet ersetzt werden; `ZELYRA_UI_STYLESHEET=zelyra` stellt es
wieder her. Einzelheiten und Theme-Tokens stehen in der
[Zelyra-UI-Referenz](../ui.de.md) und [Umgebungsreferenz](../env.md).

## Fehlerbehebung

- **Compose fehlt:** Installiere das Docker-Compose-Plugin v2 nach der
  [Docker-Anleitung](https://docs.docker.com/engine/install/) und prüfe erneut
  `docker compose version`.
- **Port ist belegt:** Für verwaltete `.env`-Ports versucht Zelyra einen freien
  Folgeport und aktualisiert die dazugehörigen Datenbankadressen. Eine explizite
  Prozessvariable wie `ZELYRA_DB_HOST_PORT` hat Vorrang; entferne oder ändere
  sie, falls sie einen belegten Port fest vorgibt.
- **`ERROR 1045 (28000): Access denied`:** Die Zugangsdaten passen nicht zum
  bereits initialisierten MariaDB-Volume. Zugangsdaten werden beim ersten
  Volume-Start angewendet. Neue Tutorialprojekte erhalten einen isolierten
  Compose-Namensraum. Bei älteren markierten Projekten startet der
  Wiederherstellungspfad eine getrennte Datenbank und lässt das alte Volume
  unangetastet.
- **Studio zeigt eine leere Ausgabe:** Prüfe mit **Logs**, ob Prozessausgaben
  vorhanden sind. Für eine Web-App zeigt **Starten** die lokale Vorschau-URL;
  öffne sie oder klicke auf **Anwendung im Browser öffnen**.

`.env` enthält lokale Geheimnisse und darf nicht committet oder geteilt werden.
Das Beispiel verwendet für alle Rechnungspositionen pauschal 19 % Umsatzsteuer
und ist keine vollständige Buchhaltungs- oder Rechnungssoftware.
