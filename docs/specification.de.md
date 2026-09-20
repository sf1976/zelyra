# Zelyra-0.1-Spezifikationsindex

Die Implementierungsspezifikation des Repositorys verteilt sich auf
Phasendokumente, das zweisprachige Handbuch und die Roadmap. Diese Datei legt
den querschnittlichen KI-nativen Vertrag fest.

## KI-native, KI-unabhängige Entwicklung

> Die KI schreibt. Zelyra prüft.

Menschen und KI-Systeme dürfen Code schreiben, aber keiner von beiden wird
automatisch als vertrauenswürdig behandelt. Compiler und Tests entscheiden,
ob Quellcode gültig ist. Die Sprache bleibt ohne Modell, Anbieter, Cloud oder
Netzwerk vollständig nutzbar.

KI-Schnittstellen sind offen, deterministisch, maschinenlesbar, versioniert,
lokal nutzbar und herstellerneutral. Die aktuell verfügbaren
maschinenorientierten Schnittstellen sind:

```text
zelyra check <file.zyl> --format=json
zelyra context <file.zyl> --format=json
zelyra impact <file.zyl> --format=json
zelyra impact <file.zyl> --symbol <kind:name> --format=json
zelyra edit --format=json change.json
zelyra fmt <file.zyl> --check
```

Die JSON-Schnittstellen verwenden Maschinen-Schema-Version `1`. JSON geht ausschließlich auf
stdout, Logs auf stderr. Diagnosen besitzen stabile Codes und halb-offene
Source-Spans mit nullbasierten UTF-8-Byte-Offsets sowie einsbasierten
Zeilen-/Byte-Spalten. Ausgaben sind deterministisch und enthalten keine
Secrets, Zeitstempel, Zufalls-IDs, absoluten Pfade oder Live-Datenbankinhalte.

Der Compiler muss weiterhin Namen, Typen, Nullability, SQL, Schemata,
Formulare, Views, APIs, Berechtigungen, Contracts, Capabilities, Tests und
Freigaben destruktiver Änderungen prüfen. KI-Code darf Capabilities nicht
unbemerkt ergänzen, Berechtigungen nicht erweitern, kein destruktives SQL
ausführen, Prüfungen nicht abschwächen und keine Secrets ausgeben. Expression-
Typed-Holes, quelltextbasierte Wirkungsanalyse mit optionaler Fokussierung und
versionierte Vorschauen für semantische Umbenennungen sind als erste Stufen
implementiert. Typed Holes in
Deklarationskontexten, vollständige Laufzeit-/Schema-Wirkungsanalyse,
umfangreichere Edit-Operationen, feinere Effekte und Benchmarks bleiben geplant.

## Interaktive Konsoleneingabe

CLI-Programme lesen eine Zeile mit `read_console(prompt: String) -> String?`.
Die Funktion schreibt den Prompt nach `stdout` und leert den Ausgabepuffer,
bevor sie auf `stdin` wartet. Ein Zeilenende (`LF` oder `CRLF`) wird entfernt;
sonstige Leerzeichen bleiben erhalten. Eine leere Zeile ist `Some("")`, das
Ende der Eingabe ist `None`. Ein I/O-Fehler wird als Laufzeitfehler gemeldet.
Die Eingabe wird nicht verborgen; `read_console` ist daher nicht für
Passwörter oder andere Geheimnisse geeignet.

Der Zugriff ist ausdrücklich: Die aufrufende Funktion benötigt `uses
Console`, und ein Projekt mit `[capabilities]` muss `console = true` setzen.
Neue Projektvorlagen lassen diese Projektfreigabe standardmäßig aus. `.env`
erteilt keine Capability. Interaktive Eingabe ist für `zelyra run` gedacht;
Webanwendungen empfangen Eingaben über ihre typisierten Requests und Formulare.

~~~zelyra
fn main() uses Console {
    eingabe = read_console("Datum: ")
    match eingabe {
        Some(datum) => {
            print("Eingegeben: " + datum)
        }
        None => {
            print("Keine Eingabe.")
        }
    }
}
~~~

## Lokalisierte Weboberfläche und Lernmodus

Die eingebaute Weboberfläche bezieht ihre Texte aus Sprachkatalogen. Deutsche
und englische UI-Texte werden in `web/locales/de.json` und
`web/locales/en.json` gepflegt; beide Dateien müssen dieselben Schlüssel
enthalten. CRUD-, Formular-, Authentifizierungs-, Validierungs-, HTTP-Fehler-
und Lernhilfetexte stammen aus diesen Katalogen. Die
Maschinenverwaltungs-View markiert kataloggebundene HTML-Texte mit
`data-zelyra-i18n="app.home_title"`; konfigurierbare Zelyra-Texte können
`@i18n:app.home_title` verwenden.

`ZELYRA_LANGUAGE=de|en` wählt den UI-Katalog. `ZELYRA_LEVEL=learn|work` zeigt
die kontextbezogene Lernhilfe oder blendet sie aus; dadurch werden keine
Capabilities oder Berechtigungen erteilt. Bei `zelyra serve` überschreiben
Prozessvariablen die `.env` des Projekts. Neue MariaDB-Projekte verwenden
standardmäßig `de`/`learn`; direktes Starten ohne diese Werte verwendet
`en`/`work`. Diese Einstellungen beeinflussen ausschließlich die Darstellung.
Erzeugte CRUD-, eigenständige Formular-, Tableview-, Login- und
Authentifizierungsverwaltungsseiten erhalten standardmäßig den responsiven
Zelyra-Anwendungsrahmen, sofern kein eigenes Layout gewählt wurde. Der Rahmen
enthält beschriftete Navigation, sichtbare Tastaturfokusse und einen
lokalisierten Sprunglink zum Inhalt. Selbst verfasste Seiten bleiben vollständig
unter Kontrolle ihrer Autoren.

Fehlende deutsche Einträge fallen auf Englisch zurück. Fehlt ein Schlüssel in
beiden eingebauten Katalogen, wird `[missing translation]` ausgegeben; ein
Konsistenztest prüft literale Referenzen gegen beide Dateien. `zelyra new` und
`zelyra init` erzeugen optionale Projektkataloge `locales/de.json` und
`locales/en.json`. Sie können Schlüssel ergänzen oder eingebaute Einträge
überschreiben, wenn diese mit `data-zelyra-i18n="schluessel"`, einer
`@i18n:schluessel`-Texteinstellung oder einem kataloggebundenen Standard-HTTP-
Fehler referenziert werden. Dieselben Projektkataloge können alle
kataloggebundenen generierten Texte für Anwendungsrahmen, CRUD, Formulare,
Tableviews, Login, Authentifizierungsverwaltung, Validierung und Lernhilfe
überschreiben. Auch generierte Feldnamen und parametrisierte Beschriftungen
(zum Beispiel ein Filtertext mit Feldnamen) werden über Kataloge aufgelöst.
Für Deutsch gilt die Reihenfolge: Projekt Deutsch, Projekt Englisch,
eingebautes Deutsch und dessen englischer Fallback. Dateien müssen
UTF-8-kodierte JSON-Objekte mit nichtleeren Stringwerten sein und dürfen
jeweils höchstens 256 KiB groß sein; ungültige Dateien und Symlinks weist
`serve` zurück. Eingefügte Texte werden für HTML escaped. Fachliche Datensätze
und unmarkierte eigene Inhalte werden nicht automatisch übersetzt. API- und
Compiler-Maschinenschnittstellen werden von diesen Einstellungen nicht
lokalisiert.

### Projektlokale Theme-Anpassungen

Projekte können das eingebaute Design mit einer Datei `zelyra.theme.css` im
Projektstamm anpassen. `zelyra serve` lädt sie aus demselben Verzeichnis wie
die angegebene `.zyl`-Datei und bindet sie nach dem Zelyra-Standardstylesheet
auf gebrandeten Seiten ein. Fehlt die Datei, bleibt das Standarddesign
unverändert. `zelyra new` und `zelyra init` erzeugen eine kommentierte
Startdatei; erzeugte Dockerfiles kopieren sie in das Laufzeit-Image.

Das Stylesheet kann dokumentierte `--zelyra-*`-Design-Tokens überschreiben
oder eigenes CSS ergänzen. Zelyra parst oder typprüft CSS nicht. Die Datei ist
öffentlicher Browserinhalt, darf höchstens 128 KiB UTF-8 groß sein und muss
eine reguläre Datei ohne symbolischen Link sein. Sie wird als `text/css` über
die reservierte GET-only-Route `/__zelyra/theme.css` ausgeliefert; bei
vorhandener Theme-Datei dürfen Seiten und APIs diese Route nicht belegen.
Lege keine Secrets in das Stylesheet. Browseranfragen durch CSS, etwa über
externe `url(...)`- oder `@import`-Angaben, kontrolliert der Projektautor und
nicht die Zelyra-`Network`-Capability des Servers.

## Typisiertes Laden von Seitendaten

Seiten können einen oder mehrere explizite Datensätze über die native
SQL-Grenze laden:

~~~zelyra
page "/customers/{name}" {
    load customer = sql<Customer> {
        SELECT id, name FROM customers WHERE name = :name
    }
    html { <h1>{customer.name}</h1> }
}
~~~

Collection-Ergebnisse verwenden einen Array-Ergebnistyp und eine typisierte
serverseitige Schleife:

~~~zelyra
page "/customers" {
    load customers = sql<Customer[]> { SELECT id, name FROM customers }
    html { <ul>for customer in customers { <li>{customer.name}</li> }</ul> }
}
~~~

Der Compiler prüft die Abfrage gegen das deklarierte Schema, stellt
Routenparameter als typisierte SQL-Parameter bereit und validiert
Record-Feldinterpolationen. Zur Laufzeit werden Autorisierung und die
`Database`-Capability vor der Abfrage geprüft. Ergebnisse werden mit
HTML-Escaping gerendert; fehlende Pflichtdatensätze und Datenbankfehler
erzeugen generische HTTP-Grenzen, ohne Datenbankdetails preiszugeben.
Collection-Schleifen für Arrays von Records sind verfügbar. Option-aware
Feld-Ausdrücke und reichere lokale View-Daten bleiben geplant.

### Wiederverwendbare View-Layouts

Benannte Views sind deterministische Seitenlayouts. Jeder View muss genau einen
Default-`<slot />` deklarieren; zusätzlich darf jeder benannte Slot höchstens
einmal vorkommen. Benannte Slots können sicheren Fallback-HTML enthalten:

~~~zelyra
view AppShell {
    html {
        <header><slot name="header"><h1>Zelyra</h1></slot></header>
        <main><slot /></main>
    }
}

page "/dashboard" {
    view: AppShell
    html {
        <slot name="header"><h1>Dashboard</h1></slot>
        <p>Seiteninhalt</p>
    }
}
~~~

Die benannten Slot-Blöcke einer Seite müssen zu Slots des ausgewählten Views
gehören; doppelte oder unbekannte Slots sind Compilerfehler. Nicht gelieferte
benannte Slots verwenden ihren deklarierten Fallback. Die Komposition erfolgt
vor Component-Erweiterung und Routing ohne globalen View-Zustand;
Authentifizierung, Capability-Prüfungen, SQL-Prüfung und HTML-Escaping bleiben
aktiv.

Erzeugte CRUD-, Formular- und Tableview-Seiten verwenden standardmäßig den
Zelyra-Anwendungsrahmen. Ein ausdrücklich gesetztes `layout: ViewName` eines
CRUDs ersetzt diesen durch den geprüften Projekt-View. Selbst verfasste
Seiten werden nicht stillschweigend umgebaut und können ihr Seiten-View selbst
wählen.

CRUD-Ressourcen können denselben projektspezifischen Anwendungsrahmen mit
`layout: ViewName` wiederverwenden:

~~~zelyra
crud Customer -> customers {
    layout: AppShell
}
~~~

Der referenzierte View muss existieren und wird wie ein Seiten-View geprüft.
Sein Default-Slot erhält generierte Listen, Details sowie erzeugte Create-,
Edit- oder Aktionsformulare. SQL, Validierung, CSRF, Authentifizierung,
Autorisierung und HTML-Escaping bleiben aktiv. Redirects werden nicht als
HTML umschlossen.

Seiten können typisierte Query-Eingaben deklarieren:

~~~zelyra
page "/customers" {
    input { search: String? }
    load customers = sql<Customer[]> {
        SELECT id, name FROM customers
        WHERE (:search IS NULL OR name LIKE CONCAT('%', :search, '%'))
    }
    html { <p>{search}</p> }
}
~~~

Query-Eingaben stehen dem SQL der Seite und HTML-Interpolationen zur
Verfügung. Der Compiler prüft ihren deklarierten skalaren Typ; die Laufzeit
bindet dekodierte URL-Werte als Datenbankparameter. Fehlende optionale Werte
werden als SQL `NULL` gebunden; fehlende Pflichtwerte und ungültige skalare
Werte erzeugen HTTP 400. Automatische Steuerungen für deklarierte
Page-Collections werden unten beschrieben; reine input-Seiten bleiben manuell.

Collection-Seiten können serverseitige Pagination aktivieren:

~~~zelyra
page "/customers" {
    paginated 25
    load customers = sql<Customer[]> { SELECT id, name FROM customers }
    html { <p>{page}</p> }
}
~~~

Die Größe nach `paginated` muss zwischen 1 und 100 liegen. Der optionale
URL-Wert `page` ist eine positive Ganzzahl mit dem Standardwert `1` und steht
im Seiten-HTML als `UInt` zur Verfügung. Die Laufzeit wendet einen
parametrisierten `LIMIT`-/`OFFSET`-Wrapper auf Collection-Abfragen an.
Pagination für beliebige Einzelabfragen weist der Compiler zurück.

Collection-Seiten können außerdem eine Sortier-Whitelist deklarieren:

~~~zelyra
page "/customers" {
    sort { name }
    load customers = sql<Customer[]> { SELECT id, name FROM customers }
    html { <p>{sort} {order}</p> }
}
~~~

Die Laufzeit akzeptiert für `sort` nur deklarierte Ergebnisfelder und für
`order` nur `asc` oder `desc`. Identifier werden nach der Compilerprüfung
quotiert; URL-Werte werden niemals als ungeprüfte SQL-Identifier in SQL
konkateniert. Sortierung und Pagination können kombiniert werden.

Collection-Seiten können außerdem durchsuchbare Ergebnisfelder deklarieren:

~~~zelyra
page "/customers" {
    search { name email }
    load customers = sql<Customer[]> { SELECT id, name, email FROM customers }
    html { <p>{search}</p> }
}
~~~

Der URL-Wert `search` wird als Parameter gebunden und mit serverseitigen
`LIKE`-Bedingungen auf die deklarierten Felder angewendet. Ein leerer
Suchwert fügt keine Bedingung hinzu; unbekannte Suchfelder sind Compilerfehler.

Collection-Seiten können außerdem typisierte Filter deklarieren:

~~~zelyra
page "/customers" {
    filter { name quantity }
    load customers = sql<Customer[]> { SELECT id, name, quantity FROM customers }
    html { <p>{filter_name}</p> }
}
~~~

Der Compiler prüft jedes Filterfeld gegen den Ergebnistyp der Collection. Für
Textfelder gelten `eq`, `contains`, `starts_with`, `ends_with` und
Nullprüfungen; numerische Felder unterstützen zusätzlich `gt`, `gte`, `lt`
und `lte`; Boolean- und andere Werte unterstützen Gleichheit und
Nullprüfungen. Werte werden als Parameter gebunden, Feldnamen und Operatoren
gegen die Deklaration geprüft. Beispiele sind
`/customers?filter_name__contains=Acme` und
`/customers?filter_quantity__gte=10`. Nicht unterstützte Operatoren und nicht
deklarierte Felder erzeugen eine kontrollierte HTTP-400-Antwort.

Wenn eine Page-Collection Suche, Filter, Sortierung oder Pagination
deklariert, erzeugt Zelyra automatisch vor dem Seiteninhalt ein semantisches
Query-Steuerungsformular und bewahrt den aktuellen URL-Zustand. Pagination
führt zusätzlich eine sichere Zählabfrage aus und stellt `total` und `pages`
als `UInt`-Seitenbindungen bereit. Seiten, die ausschließlich explizite
`input`-Deklarationen verwenden, bleiben bewusst manuell.
