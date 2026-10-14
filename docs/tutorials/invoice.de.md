# Rechnungstutorial

Dieses Tutorial führt durch eine mehrteilige Rechnungsanwendung: Kundinnen und
Kunden, Artikel, Rechnungen, Positionen, ein Live-Dashboard und eine druckbare
Ansicht. Die erzeugten Dateien bleiben lokal und lassen sich sowohl über die
Kommandozeile als auch mit Zelyra Studio starten.

## Voraussetzungen und erster Start

Installiere Docker und Docker Compose v2 oder neuer. Prüfe die Installation mit
`docker compose version`. Zelyra installiert Docker nicht selbst.

Erzeuge, initialisiere und starte das Tutorial mit:

~~~sh
zelyra --tutorial invoice
~~~

Der Befehl erstellt `invoice-tutorial`, legt eine geschützte `.env` an, startet
MariaDB, richtet Datenbankschema und Beispieldaten ein, startet die Anwendung
und gibt ihre Adresse aus. Öffne die ausgegebene Adresse, falls sich kein
Browser öffnet. Die Beispieldaten enthalten zwei Kunden, drei Artikel und den
Rechnungsentwurf `RE-2026-001` mit zwei Positionen. Wiederholte Starts fügen
diese Datensätze nicht erneut hinzu.

Die Startseite `/` ist derzeit eine Tutorialübersicht mit Kennzahlen und
Arbeitsschritten. `/views/invoicedashboard` ist die filter- und durchsuchbare
Rechnungstabelle mit berechneten Netto-, Steuer- und Bruttosummen. Die
Startseite und Rechnungsübersicht werden für Version 1.0.5 stärker auf den
Rechnungsablauf ausgerichtet.

Nur die Dateien erzeugen, ohne Docker zu starten:

~~~sh
zelyra --tutorial invoice ./meine-rechnungen --no-start
cd ./meine-rechnungen
zelyra setup . --all
~~~

`--no-start` erzeugt trotzdem `.env`, Compose-Datei, Schema, Seed-SQL und
Anwendung. `zelyra setup . --all` führt später den vollständigen lokalen Start
aus. Die Zugangsdaten stehen nur in der lokalen, von Git ignorierten `.env`.

## Start aus Zelyra Studio

Öffne das Projekt in Studio:

~~~sh
zelyra editor ./invoice-tutorial
~~~

Wähle im Projektbaum `main.zyl` und klicke auf **Run**. Studio startet die
lokale MariaDB, wartet auf die Verbindung, richtet das deklarierte Schema ein,
lädt die wiederholbaren Beispieldaten und startet anschließend eine lokale
Webvorschau. Der Run-Befehl zeigt die Vorschauadresse an; **Stop preview**
beendet sie. Dieser Weg benötigt Docker Compose sowie die lokale
Tutorialkonfiguration in `.env`.

Wenn Studio während `zelyra update` geöffnet war, beende diese Sitzung mit
`Ctrl+C` und starte Studio danach neu. Ein Update ersetzt die Programmdatei für
neue Prozesse; ein bereits laufendes Studio lädt den neuen IDE-Code nicht
automatisch nach. Prüfe bei Bedarf zuerst `zelyra --version`.

Für eine andere Datenbank, die über Prozessvariablen konfiguriert wurde, startet
Studio nicht eigenständig einen lokalen Datenbankdienst und lädt keine
Tutorialdaten hinein. So bleiben externe Datenbanken unverändert.

## Befehle zum Erkunden

Führe die Befehle im Tutorialverzeichnis aus:

~~~sh
zelyra check main.zyl
zelyra fmt main.zyl --check
zelyra routes main.zyl
zelyra db map main.zyl
zelyra setup . --database
zelyra setup . --schema
zelyra setup . --all
~~~

`check` prüft das importierte Projekt. `fmt --check` meldet Formatabweichungen,
ohne Dateien zu ändern. `routes` zeigt Seiten, APIs und erzeugte CRUD-Routen.
`db map` stellt deklarierte Tabellen und ihren Modulbesitz dar. Die Setup-Modi
starten MariaDB, richten das Schema ein oder erledigen den vollständigen Ablauf.

### Woher kommt `/invoices/new`?

In `main.zyl` und `src/shell.zyl` verweisen Schaltflächen auf `/invoices/new`.
Eine Zeile `page "/invoices/new"` gibt es im Tutorial nicht. Die Route wird
aus der CRUD-Deklaration `crud Invoice -> invoices` in `src/invoices.zyl`
erzeugt. `crud` aktiviert automatisch erzeugte Ansichten und Aktionen für die
Ressource `invoices`. Dazu gehören `GET /invoices` für die Liste,
`GET /invoices/new` für das Formular zum Anlegen und `POST /invoices/new` zum
Speichern. Der Name hinter `->` bestimmt den URL-Basispfad.

Die Druckansicht ist dagegen ausdrücklich in derselben Datei deklariert:
`page "/invoices/{invoice_number}/print"`. Mit `zelyra routes main.zyl`
kannst du beide Arten von Routen und ihre Herkunft in der Befehlsausgabe
nachvollziehen.

Vor einer späteren Schemaänderung kannst du den Plan ansehen und dann den
angezeigten Plan gezielt anwenden:

~~~sh
zelyra db plan main.zyl --format=json
zelyra db apply main.zyl --plan-id sha256:<plan-id>
~~~

Bei riskanten Änderungen verlangt Zelyra eine gesonderte Freigabe mit
`--allow-risky`; prüfe den Plan vor dem Anwenden.

## Projektdateien und Arbeitsablauf

~~~text
invoice-tutorial/
├── main.zyl                      Einstieg und Dashboard
├── zelyra.toml                   Projektkonfiguration
├── zelyra.theme.css              lokale Designanpassungen
├── invoice-tutorial.seed.sql     wiederholbare Beispieldaten
├── zelyra.tutorial               Tutorialkennung für den Seed-Schritt
└── src/
    ├── database.zyl              MariaDB-Deklaration
    ├── shell.zyl                 gemeinsames Layout
    ├── customers.zyl              Kunden und CRUD-Seiten
    ├── items.zyl                  Artikel und CRUD-Seiten
    ├── invoices.zyl               Rechnungen, Positionen, Druckansicht
    └── dashboard.zyl              Rechnungsübersicht
~~~

Im Dashboard siehst du Kunden- und Rechnungszahl sowie Netto-, Steuer- und
Bruttosumme. Beginne unter `/customers` mit einem Kunden und unter `/items` mit
einem Artikel. Lege dann unter `/invoices` einen Entwurf an und ergänze unter
`/invoice_lines` seine Positionen. Die Beispieldatensätze machen Dashboard,
Suche und Druckansicht sofort sichtbar.

`zelyra --help` und `zelyra db --help` zeigen die Befehle und Optionen. Studio
zeigt die importierten Quelldateien im Projektbaum; Compiler und Formatter
prüfen jeweils das gesamte Projekt, wenn du den Einstieg `main.zyl` verwendest.

## Ports und Fehlerbehebung

Beim Erzeugen einer neuen lokalen Konfiguration wählt Zelyra freie Ports für
MariaDB und die Webanwendung. Meldet Docker Compose beim Start trotzdem einen
Portkonflikt, wählt Zelyra bei einer verwalteten `.env` automatisch einen
anderen freien Port, aktualisiert die zugehörige lokale Datenbank-URL und
wiederholt den Start. Die neue Anwendungsadresse wird ausgegeben. Prozessweite
Variablen wie `ZELYRA_DB_HOST_PORT` haben Vorrang; wenn sie den belegten Port
fest vorgeben, ändere oder entferne diese Variable und wiederhole den Befehl.

Compose v1 wird nicht unterstützt. Bei fehlendem Compose-Plugin installiere
Compose v2 nach der [Docker-Anleitung](https://docs.docker.com/engine/install/)
und prüfe erneut `docker compose version`. Bestehende `.env`-Dateien und
Datenbank-Volumes werden beim Wechsel des Ports nicht gelöscht.

`ERROR 1045 (28000): Access denied` weist auf Zugangsdaten hin, die nicht zum
bereits initialisierten MariaDB-Volume passen. Die Zugangsdaten in `.env`
werden nur beim ersten Initialisieren eines Volumes angewendet. Neue
Rechnungstutorials ab Zelyra 1.0.4 erhalten einen eigenen Compose-Namen und
verwenden dadurch keine alten Tutorial-Volumes wieder. Bei älteren
Rechnungstutorials startet `zelyra setup --all` nach diesem Fehler eine
isolierte Datenbank und richtet das Schema dort ein. Das bisherige Volume wird
nicht gelöscht oder verändert; seine Daten bleiben erhalten.

## Grenzen des Beispiels

Die Vorlage berechnet für alle Positionen pauschal 19 % Umsatzsteuer. Sie ist
ein Lernbeispiel, keine vollständige Rechnungs- oder Buchhaltungssoftware.
Steuerfälle, Pflichtangaben, GoBD-Anforderungen, E-Mail-Versand und native
PDF-Erzeugung sind nicht vollständig umgesetzt. Vor einem produktiven Einsatz
müssen diese Anforderungen geprüft und ergänzt werden.
