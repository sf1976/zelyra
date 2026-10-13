# Zelyra-0.1-Spezifikationsindex

Die Implementierungsspezifikation des Repositorys verteilt sich auf
Phasendokumente, das zweisprachige Handbuch und die Roadmap. Diese Datei legt
den querschnittlichen KI-nativen Vertrag fest.

Die Sprachkompatibilitätserklärung und ihre Regeln stehen im
[Leitfaden zur Sprachkompatibilität](language-compatibility.de.md).

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

String- und Zeichenliterale bewahren gültige Unicode-Zeichen unverändert.
Source-Spans verwenden weiterhin UTF-8-Byte-Offsets.

## Kommentare in Sprachlinie 0.1

`#` beginnt einen Zeilenkommentar bis zum Zeilenende. Eine alleinstehende
`##`-Zeile öffnet einen Blockkommentar; die nächste alleinstehende `##`-Zeile
schließt ihn. Blockkommentare werden nicht verschachtelt. Fehlt das Ende,
meldet der Lexer einen Fehler am Öffner. Die ältere `//`-Zeilenkommentarsyntax
bleibt für gültige Bestandsprogramme erhalten; neuer Zelyra-Quelltext verwendet
`#` und `##`.

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

Funktions-SQL kann für klassifizierte `SELECT`-Anweisungen `uses
Database(read)` oder für klassifizierte `INSERT`-, `UPDATE`- und `DELETE`-
Anweisungen `uses Database(write)` deklarieren. Nicht klassifizierbare
Anweisungen benötigen `uses Database`. Die breite Form bleibt mit bestehenden
Programmen kompatibel. Projekte können die engeren Effekte über
`database_read` und `database_write` in `zelyra.toml` einzeln freigeben. Diese
Freigaben ändern keine MariaDB-Kontorechte.

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

## Experimentelle Modul-Imports in Projekten

Der aktuelle Entwicklungszweig enthält einen experimentellen ersten Schritt
für Datei-Imports. Er ist nicht Teil des veröffentlichten 0.3.0-Releases und
noch keine stabile Zusage zur Sprachkompatibilität.

Importpfade sind relativ zum Verzeichnis der ausdrücklich gewählten
Einstiegsquelldatei. Ein Alias ist Pflicht; Imports müssen vor allen
Deklarationen stehen:

~~~zelyra
import "src/math.zyl" as math

pub fn add(left: Int, right: Int) -> Int {
    return left + right
}

fn main() {
    print(math::add(2, 3))
}
~~~

Eine importierte Quelldatei darf derzeit Funktionen, Typ-Aliase, Records,
Tabellen, benannte Views, typisierte Komponenten und eine projektweite
Datenbankverbindungsdefinition enthalten. Die
Datenbankkonfiguration wird in die Anwendung übernommen und nicht über den
Importalias angesprochen; im gesamten Projektgraphen ist höchstens eine
Datenbank zulässig. Tabellen aus importierten Dateien werden in das gemeinsame
physische Schema der Anwendung aufgenommen. Tabellennamen sind globale
SQL-Bezeichner statt modulqualifizierter Namen; doppelte Tabellennamen werden
abgelehnt. Importierte Views und Komponenten werden unter ihren deklarierten,
nicht qualifizierten Namen in die Anwendung aufgenommen. Sie sind wie andere
Modulressourcen standardmäßig privat und werden mit `pub view` beziehungsweise
`pub component` exportiert. Modulübergreifende Page- und CRUD-Layout-Verweise
sowie erkannte Komponenten-Tags in Page-, View-, Komponenten- und CRUD-Slot-HTML
benötigen außerdem einen direkten oder transitiven Importpfad zum Besitzer.
Private UI-Verweise melden `E-MOD-007`, fehlende Importkanten `E-MOD-020`.
Komponenten-Tags werden aktuell durch Abgleich bekannter Namen im HTML-Text
erkannt; das ist keine vollständige HTML- oder Namespace-Analyse. Doppelte
View- und Komponentennamen werden abgelehnt.
Importierte Pages, Formulare, CRUD-, API- und Authentifizierungsdefinitionen
werden im aktuellen Entwicklungszweig unterstützt; die veröffentlichte
Version 0.3.0 enthält das Modulmodell nicht.
Funktions-, Typ- und Record-Deklarationen sind standardmäßig privat;
Deklarationen überschreiten eine Modulgrenze nur mit `pub` und einem
ausdrücklichen Importalias, zum Beispiel als `pub fn`, `pub type`,
`pub struct`, `pub view` oder `pub component`. Ein qualifizierter Typ wie
`money::Amount` wird über den Import `money` der aktuellen Datei aufgelöst.
Private Hilfsfunktionen und Typen bleiben innerhalb ihrer Datei verfügbar.
Öffentliche Signaturen dürfen keine privaten Typen offenlegen. Der Compiler
lehnt fehlende Dateien, doppelte Aliasse, Importzyklen, Pfad-Traversal,
Symlinks außerhalb des Projektstamms sowie Aufrufe und Typreferenzen auf
private oder unbekannte Deklarationen ab. Beim Laden der Module findet kein
Netzwerkzugriff statt.

Zum Beispiel kann `src/money.zyl` einen Record exportieren, den
`src/invoice.zyl` in einer Funktionssignatur verwendet:

~~~zelyra
pub struct Money {
    cents: Int
}
~~~

~~~zelyra
import "src/money.zyl" as money

pub fn total() -> money::Money {
    return money::Money { cents: 2500 }
}
~~~

`check`, `build`, `run`, `serve`, `context` und `verify` prüfen derzeit diesen
Projektgraphen. Das maschinenlesbare Kontextdokument enthält jedes erreichbare
Modul und seine sortierten Importkanten. Importierte Datenbankdefinitionen und
Tabellen werden in das gemeinsame Projektschema übernommen; ihre
Kontextspannen nennen unter `span.file` den projektrelativen Quelldateipfad.
Das Kontextinventar enthält außerdem importierte Views und Komponenten mit
ihren Quellspannen. `serve` verknüpft importierte Views/Komponenten mit den
Seiten und startet die zusammengesetzte Anwendung. Bei einigen
Template-Diagnosen fehlt noch eine vollständige Zuordnung zum jeweiligen
Modul. `verify` prüft verknüpfte Funktionen, bewahrt in
Verifikationsergebnissen aber noch keine Quellzuordnung pro Modul. Seiten,
APIs und andere routengebundene Anwendungsressourcen bleiben in importierten
Dateien unzulässig. `fmt`,
`edit` und Datenbankbefehle arbeiten weiterhin nur mit der angegebenen
Quelldatei. `impact` wertet den verknüpften Modulgraphen aus und weist seine
Spannen der jeweiligen Quelldatei zu. Diese Grenzen machen die
Implementierung experimentell und noch nicht zu einem vollständigen
Mehrdatei-Projektmodell. Die vollständigen Anforderungen stehen im
[Releaseplan 0.4.0](release-plans/0.4.0.de.md).

Wenn eine Page-Collection Suche, Filter, Sortierung oder Pagination
deklariert, erzeugt Zelyra automatisch vor dem Seiteninhalt ein semantisches
Query-Steuerungsformular und bewahrt den aktuellen URL-Zustand. Pagination
führt zusätzlich eine sichere Zählabfrage aus und stellt `total` und `pages`
als `UInt`-Seitenbindungen bereit. Seiten, die ausschließlich explizite
`input`-Deklarationen verwenden, bleiben bewusst manuell.

## Sprach- und Laufzeitergänzungen für 0.9.8

Diese Ergänzungen erweitern die Sprachlinie `0.1` und sind durch positive und
negative Regressionstests abgedeckt. Sie ändern keine bestehenden gültigen
Konstrukte absichtlich.

- Ein ganzzahliges Minus direkt vor der Magnitude `9223372036854775808` bildet
  `Int::MIN` (`-9223372036854775808`). Die positive Magnitude allein und Werte
  unterhalb des Minimums bleiben ungültig; eine erneute Negation des Minimums
  ergibt einen kontrollierten Laufzeitfehler.
- Funktionen mit deklariertem Nicht-`Unit`-Rückgabetyp müssen jeden erreichbaren
  Pfad mit einem Wert dieses Typs beenden. Andernfalls meldet der Compiler
  `E-TYPE-RETURN-001`. Unvollständige Varianten-Matches verwenden den stabilen
  Code `E-TYPE-MATCH-001`. Die Analyse behandelt Sequenzen, Bedingungen, `match`,
  Schleifen und Kontrollflussabbrüche konservativ; eine möglicherweise
  terminierende Schleife gilt nicht als garantierte Rückgabe.
- Verschachtelte Funktionsaufrufe werden zur Laufzeit begrenzt und bei
  Überschreitung mit einer quellgebundenen Runtime-Diagnose abgebrochen. Das
  schützt vor Stackabstürzen, ist aber kein konfigurierbares Rekursionslimit.
  Zusätzliche Tests in 0.9.9 zeigten, dass die zuvor gewählte Grenze auf
  kleinen Runtime-Stacks noch abstürzen konnte; sie ist deshalb auf 24 aktive
  Aufruf-Frames abgesenkt.
- Eigene Enums unterstützen payloadfreie und payloadtragende Varianten.
  `enum Status { Draft Paid }` sowie `enum Event { Created(Int) Paid(Date) }`
  erzeugen typsichere Werte. Muster binden Nutzdaten, und `match` muss jede
  Variante abdecken. Öffentliche Enums samt Konstruktoren und Mustern dürfen
  über explizite Modulimporte verwendet werden. Unit-Varianten behalten ihr
  bisheriges JSON-Format `{"$enum":"Status","variant":"Draft"}`;
  Varianten mit Nutzdaten verwenden zusätzlich ein `payload`-Array, zum
  Beispiel `{"$enum":"Event","payload":[7],"variant":"Created"}`.
- `for item in start..end` durchläuft Integerwerte mit exklusivem Ende. Leere
  Bereiche sind erlaubt; negative Werte, `break` und `continue` funktionieren.
  Ein direkt verwendeter `range(start, end)`-Arraywert ist auf eine Million
  Elemente begrenzt.
- `record_with(record, "field", value)` erstellt einen statisch geprüften
  Record mit einem ersetzten Feld. `array_replace_result(array, index, value)`
  gibt einen neuen Arraywert als `Result` zurück und meldet ungültige Indizes
  als `Err`; beide Operationen bewahren den Eingabewert.
- `date_add_days_result` und `date_days_between_result` rechnen in
  gregorianischen Kalendertagen. `timestamp_add_result` und
  `timestamp_difference_result` rechnen mit UTC-Dauern; Vergleiche ordnen
  Zeitstempel chronologisch. Unterstützt sind ISO-Jahre 0001–9999. Zeitzonen
  und lokale Sommerzeit sind nicht enthalten.
- `decimal_quantize_result(value, scale, mode)` begrenzt auf 0 bis 9
  Nachkommastellen und unterstützt `toward_zero`, `floor`, `ceiling`, `half_up`
  und `half_even`. Die Berechnung verwendet Ganzzahlen. Die vorhandene
  Festkommadivision kann weiterhin jenseits der neunten Stelle abschneiden.
- `zelyra run file.zyl --max-steps N` aktiviert eine optionale Obergrenze für
  ausgeführte Anweisungen und Schleifeniterationen. Ohne Option bleibt das
  Budget unbegrenzt. Das Budget ist keine Prozessisolation. Benannte
  Funktionswerte, benutzerdefinierte generische Datentypen und allgemeine
  generische Constraints sind noch offen.

### Generische Funktionen in 0.9.9

Der 0.9.9-Entwicklungszweig unterstützt benutzerdefinierte generische
Funktionen. Typparameter stehen direkt nach dem Funktionsnamen; Typen können
beim Aufruf aus Argumenten abgeleitet oder ausdrücklich angegeben werden:

~~~zelyra
fn identity<T>(value: T) -> T {
    return value
}

fn first_or<T>(values: T[], fallback: T) -> T {
    if len(values) > 0 { return values[0] }
    return fallback
}

fn main() {
    print(identity(42))
    print(identity<String>("Zelyra"))
    print(first_or(["Kunde", "Artikel"], "unbekannt"))
}
~~~

Ein Typparameter muss aus Argumenten ableitbar sein oder explizit angegeben
werden. Wiederholte Verwendungen desselben Parameters müssen denselben Typ
haben; andernfalls meldet der Compiler `E-TYPE-001`. Nicht ableitbare Typen,
eine falsche Anzahl expliziter Typargumente und Typparameter, die deklarierte
Typen verdecken, verwenden `E-TYPE-GENERIC-001`. Generische Funktionen dürfen
andere generische Funktionen aufrufen. Der Type-System-Beispiellauf deckt
Inference, explizite Typen, Arrays, `Option`, Modulimporte und erwartete
Compilerfehler ab. Generische Funktionen, Records und Enums sind verfügbar;
allgemeine Constraints bleiben außerhalb des aktuellen Umfangs.

### Generische nutzerdefinierte Typen in 0.9.9

Records und Enums dürfen Typparameter deklarieren. Explizite Typargumente
stehen in spitzen Klammern. Bei Record-Literalen und Enum-Payloads leitet der
Compiler Typargumente ab, wenn alle Parameter aus den Feldern beziehungsweise
Payloads hervorgehen:

~~~zelyra
struct Box<T> { value: T }
enum Maybe<T> { Some(T) None }

fn unwrap_or<T>(value: Maybe<T>, fallback: T) -> T {
    match value {
        Maybe::Some(found) => { return found }
        Maybe::None => { return fallback }
    }
}

fn main() {
    box: Box<Int> = Box<Int> { value: 7 }
    inferred = Box { value: "Artikel" }
    some = Maybe::Some(42)
    none: Maybe<String> = Maybe::None<String>()
    print(unwrap_or(some, 0))
    print(unwrap_or(none, "leer"))
}
~~~

Ein generischer Enum-Unit-Konstruktor ohne Payload kann seinen Typ nicht
ableiten und braucht deshalb die Form `Maybe::None<String>()`. In `match`
werden Payload-Muster anhand der konkreten Enum-Typargumente geprüft; alle
Varianten bleiben exhaustive abzudecken. `json_decode<Box<Int>>` und
`json_decode<Maybe<Int>>` prüfen Payloads gegen den expliziten Zieltyp. Das
bestehende JSON-Format fügt keine Typargumente hinzu. Records mit generischen
Parametern sind keine SQL-Spaltentypen und werden dort ausdrücklich abgelehnt.

Allgemeine Constraints/Traits (`T: Ord`), generische Typ-Aliase und generische
Funktionsreferenzen werden nicht unterstützt. Generische Parameter dürfen nur
Operationen verwenden, deren Typ für den konkreten Wert feststeht. Der
Compiler meldet fehlende oder widersprüchliche Ableitungen mit
`E-TYPE-GENERIC-001` beziehungsweise einem Typfehler.

### Records und Klassen

`struct`-Records sind Werte: Zuweisung und Parameterübergabe kopieren den
Recordwert, Gleichheit vergleicht seine Felder, und `record_with` erzeugt einen
geänderten neuen Wert. `class`-Instanzen besitzen dagegen Referenzidentität.
`class` ist nur vor einer Klassendeklaration ein Kontextwort. An allen anderen
Stellen bleibt es ein normaler Bezeichner, damit gültiger Quelltext der
Sprachlinie `0.1` mit einer Variable oder Funktion namens `class` gültig bleibt.
Konstruktoren verwenden dieselbe lesbare Feldschreibweise:

~~~zelyra
class Counter {
    value: Int
}

fn increment(counter: Counter) {
    counter.value = counter.value + 1
}

fn main() {
    first = Counter { value: 0 }
    alias = first
    increment(first)
    print(alias.value) # 1
    print(first == alias) # true
}
~~~

Alle nicht-optionalen Felder müssen beim Erstellen angegeben werden. Eine
Zuweisung an ein Feld wird statisch auf Empfänger, Feldname und Werttyp geprüft
und aktualisiert dieselbe Instanz; Kopieren einer Bindung, Rückgabe aus einer
Funktion und Übergabe als Parameter erhalten dieselbe Identität. `==` und `!=`
vergleichen Klassenreferenzen anhand der Instanzidentität, auch wenn zwei
Instanzen gleiche Feldwerte haben. Generische Klassen sind möglich, sofern ihre
Typargumente aus dem Konstruktor ableitbar sind oder ausdrücklich angegeben
werden.

Klassen besitzen keine Methoden- oder Vererbungssyntax. Verhalten wird als
gewöhnliche Funktion mit einem ausdrücklich typisierten Empfängerparameter
geschrieben; dadurch bleibt der Datenfluss sichtbar und es gibt kein implizites
`this`. `record_with` ist für Klassen unzulässig. Ein Feldupdate, das einen
Referenzzyklus erzeugen würde, wird mit `E-RUNTIME-001` abgewiesen;
Feldänderungen innerhalb eines `parallel`-Blocks liefern ebenfalls eine
kontrollierte Runtime-Diagnose. Damit verhindert die Runtime unbeschränkte
Referenzzyklen und nichtdeterministische gleichzeitige Mutationen.

Klassenreferenzen lassen sich nicht direkt mit JSON, HTTP-APIs oder SQL
serialisieren, weil diese Formate keine Runtime-Objektidentität bewahren.
Konvertiere die für einen Austausch benötigten Felder ausdrücklich in einen
`struct`-DTO. Auch Records und Varianten, die Klassenreferenzen enthalten,
werden an JSON-Grenzen abgewiesen. Diese Klassenregeln gelten für den
0.9.9-Entwicklungszweig; Vererbung, Methoden, Closures und ein allgemeiner
Objekt-/ORM-Mapper bleiben außerhalb dieses Umfangs.

### Benannte Funktionswerte in 0.9.9

Der 0.9.9-Entwicklungszweig kann Referenzen auf benannte, nicht generische
Funktionen speichern, übergeben und indirekt aufrufen. Ein vorangestelltes `&`
kennzeichnet die Referenz; es ruft die Funktion nicht auf:

~~~zelyra
fn increment(value: Int) -> Int { return value + 1 }

fn apply(callback: Function<(Int) -> Int>, value: Int) -> Int {
    return callback(value)
}

fn main() {
    transform = &increment
    print(apply(transform, 8))
}
~~~

Funktionswerte besitzen ausdrückliche Parameter- und Rückgabetypen. Ein
effektbehafteter Funktionstyp nennt seine benötigten Capabilities mit
`uses {Capability}`. Die Anforderungen der referenzierten Funktion müssen in
den deklarierten Funktionstyp passen; auch die aufrufende Funktion eines
indirekten Aufrufs muss diese Anforderungen deklarieren. Die Typprüfung folgt
Funktionswerten durch lokale Bindungen und Zuweisungen; die Laufzeit prüft die
Capability zusätzlich vor dem Aufruf. Öffentliche Funktionen dürfen über
explizite Modulaliase referenziert werden, etwa `&math::increment`; private
Funktionen sind nicht modulübergreifend verfügbar. `function` bleibt ein
gewöhnlicher Bezeichner.

Closures, eingefangene lokale Funktionen und Referenzen auf generische
Funktionen sind in diesem begrenzten Schritt nicht enthalten. Funktionswerte
haben keine Gleichheits- oder Identitätssemantik; sie sind keine JSON- oder
SQL-Daten und werden an diesen Grenzen abgewiesen. Ihre Signaturen stehen
Compilerwerkzeugen zur Verfügung, aber nicht als HTTP-Schemadaten.

### Enum-Varianten mit Nutzdaten in 0.9.9

Ein Enum kann bei jeder Variante die Typen ihrer gespeicherten Werte angeben:

~~~zelyra
enum OrderEvent { Created(Int) Paid(Date, Decimal) Rejected(String) }

fn describe(event: OrderEvent) -> String {
    match event {
        OrderEvent::Created(id) => { return "created" }
        OrderEvent::Paid(day, total) => { return "paid" }
        OrderEvent::Rejected(reason) => { return "rejected" }
    }
}
~~~

Konstruktorargumente und Muster werden gegen die deklarierte Anzahl und die
jeweiligen Typen geprüft. Ein unvollständiger Match liefert
`E-TYPE-MATCH-001`; ein falscher Variant- oder Payload-Gebrauch liefert
`E-TYPE-ENUM-001`. JSON für Payload-Varianten hat ein explizites
`{"$enum": ..., "variant": ..., "payload": [...]}`-Format und wird beim
Decodieren anhand der Enumdeklaration validiert. Das Format der bisherigen
Payload-freien Varianten bleibt gleich.

### Getaggter JSON-Codec für `Result`

`json_encode_tagged_result(value)` serialisiert ausschließlich einen
`Result<T,E>`-Wert in ein explizites Objekt mit Discriminator. `Ok(value)` wird
als `{"$result":"ok","value":...}` kodiert, `Err(error)` als
`{"$result":"err","error":...}`. `json_decode_tagged_result<T,E>(text)`
liest dieses Format als `Result<Result<T,E>, String>`: Der äußere Fehler enthält
JSON-, Form- oder Payloadfehler, der innere Wert ist die dekodierte fachliche
`Ok`-/`Err`-Variante. Unbekannte Discriminators, fehlende oder zusätzliche
Felder und ungültige Payloads liefern einen äußeren `Err`. Das bestehende
`json_encode`-Format wird nicht verändert; HTTP- und API-Wireformate bleiben
damit kompatibel.

### Explizite skalare Textformatierung in 0.9.9

`format_text(wert)` wandelt ausdrücklich einen einzelnen skalaren Wert in
Anzeigetext um. Zulässig sind `Int`, `UInt`, `Float`, `Decimal`, `Bool`, `Char`,
`String`, `Date`, `Time`, `Timestamp` und `Duration`. Die Funktion fügt keine
JSON-Anführungszeichen hinzu und erlaubt keine implizite Konvertierung in
Zuweisungen oder Textverkettungen. Arrays, Records, Enums, Option-/Result-Werte,
Unit und Funktionswerte besitzen keinen impliziten Anzeigestandard und werden
abgewiesen. Bei Decimal werden bedeutungslose Nullen am Ende weggelassen; für
Geldbeträge mit fester Nachkommastellenzahl ist `format_scaled_result` gedacht.

## Persistente Passwort-Reset-Limits in 0.9.9

Eine Auth-Deklaration mit `reset_tokens` benötigt auch
`reset_rate_limits: <tabelle>`. Das ist eine ausdrückliche Schemaabhängigkeit;
die Reset-Runtime legt keine Tabellen an. Die Limit-Tabelle hat ein eindeutiges,
nicht-nullbares `bucket_key: String(64)`, erforderliche Felder
`window_started: Timestamp` und `request_count: Int`, ein nullable
`blocked_until: Timestamp?` sowie einen Index beginnend mit `window_started`.
Die Kernspalten dürfen kein `auto` verwenden. Das vollständige Beispiel steht
in [`examples/password_reset.zyl`](../examples/password_reset.zyl).

Der Standard bleibt drei Anfragen je 900 Sekunden mit anschließender Sperre
für 900 Sekunden. Forgot-Password-Anfragen mit gültigem CSRF verbrauchen das
Limit vor der Kontosuche; Reset-Password-Formulare mit gültigem CSRF und
syntaktisch gültigem Token verwenden denselben Eintrag. Maßgeblich ist die
TCP-Peer-IP unabhängig vom Quellport; Forwarded-IP-Header werden ignoriert.
Einträge bleiben nach Neustart bestehen und gelten gemeinsam für Instanzen mit
derselben schreibbaren MariaDB, denselben Limits und demselben `ZELYRA_RESET_DELIVERY_KEY`.
HMAC-SHA256 mit einer eigenen Zweckkennung leitet den gespeicherten Schlüssel
ab; die Datenbank erhält keine Peer-IP. Es handelt sich um pseudonymisierten
Sicherheitszustand, nicht um eine Anonymitätsgarantie.

Advisory-Lock und Transaktion serialisieren Limitänderungen und begrenzen die
Tabelle auf 4.096 Einträge. Das Lock wartet höchstens eine Sekunde; Datenbank-,
Lock- und Schlüsselfehler verhindern den Reset-Versuch und melden die bereinigte
Betreiberdiagnose `E-AUTH-043` ohne Konto-, Token-, IP- oder Verbindungsdetails. Die generische Antwort
auf Forgot-Password bleibt HTTP `202`; abgewiesene Reset-Password-Formulare
liefern weiterhin `400`. Eine aktive Sperre gilt auch nach Ablauf des
Anfragefensters. Ein abgelaufenes, ungesperrtes Fenster beginnt mit der aktuellen
Anfrage als erster Anfrage neu. Maßgeblich ist die MariaDB-Uhr. Pro Anfrage
werden höchstens 100 inaktive Einträge mit einem mehr als 48 Stunden alten
Fenster entfernt; ohne Anfragen läuft keine zeitgesteuerte Bereinigung. Eine
volle Tabelle weist neue Einträge ab und behält bestehende Limits bei.

Bestehende experimentelle Reset-Projekte müssen Auth-Option und Tabelle
ergänzen und ihre additive Migration vor dem Compilerwechsel prüfen. Fehlende
oder inkompatible Speicherung liefert `E-AUTH-039`/`E-AUTH-040`, ein fehlender
Bereinigungsindex `E-AUTH-041` und die Option ohne `reset_tokens` `E-AUTH-042`.
Die Diagnosen behalten Quelldatei und Span der Auth-Deklaration. Normale
Login-Deklarationen sind unverändert. `context` ergänzt optionale Reset-Felder
in Schemaversion `1`; `impact`, Modulpläne, Tabellenrechte und Tabellenumbenennung
berücksichtigen jetzt Reset-, Tenant-Mitgliedschafts- und MFA-Speicherung.
Für zuvor ausgelassene Auth-Tabellenreferenzen können ein ausdrücklicher
Importpfad und eine `read_write`-Freigabe erforderlich sein. Siehe
[Konfiguration](env.md).

### Grenzen für SQL-Bezeichner

Deklarierte Tabellen- und Speicherspaltennamen werden vor der SQL-Ausführung
geprüft: MariaDB/MySQL erlauben höchstens 64 Bytes, PostgreSQL 63 Bytes für
Zelyras ASCII-Bezeichner. Bei Beziehungen zählt die erzeugte Endung `_id`
zum Speicherspaltennamen. Zu lange Namen liefern `E-DB-001` an der Deklaration.
Für SQLite wird diese Grenze nicht gesetzt.

Erzeugte Namen für Indizes, eindeutige Indizes und Fremdschlüssel behalten ihre
bisherige kurze Schreibweise. Längere Namen erhalten einen begrenzten Präfix
und die ersten 16 hexadezimalen Zeichen des SHA-256-Hashes ihres vollständigen
Namens. Schemaerstellung, Migrationspläne und die Erkennung verwalteter Objekte
verwenden dieselbe deterministische Regel. Bestehende PostgreSQL-Objekte, deren
alte Namen der Server still gekürzt hat, können eine geprüfte Übernahme
erfordern; sie werden nicht still umbenannt oder gelöscht.

## Exakte Dezimalgrenzen in 0.9.9

`betrag: Decimal = decimal("119.00")` deklariert einen exakten Festkommawert.
Formulare, HTTP und SQL verwenden geprüfte Dezimaltexte. MariaDB/MySQL speichern
`Decimal` ausdrücklich als `DECIMAL(39,9)`, PostgreSQL als `NUMERIC(39,9)`.
Alte oder abweichende feste Skalen verhindern SQL-Ausführung beziehungsweise
Serverstart mit einer quellbezogenen Diagnose. `Money` behält `(19,4)`.
Multiplikation/Division prüfen das Endergebnis mit einem begrenzten exakten
Zwischenwert; Restbildung am negativen Minimum verursacht keinen Panic.
JSON-Ausgaben bleiben Dezimalstrings. Siehe [vollständige Regeln](decimals.de.md)
und [Beispiel](../examples/decimal_money.zyl). Das belegt keine volle
PostgreSQL-CRUD-/Auth-Parität oder Produktivfreigabe.

## Datums-/Zeitgrenzen im Entwicklungszweig

Kalender-Tagesrechnung und UTC-Millisekunden sind mit unabhängigen Python-
Referenzen geprüft. Formulare und APIs weisen ungültige Kalender-/Uhrzeitwerte
ab; Timestamp ist auf ISO-Jahre 0001–9999 begrenzt. API-Handler erhalten echte
Date-/Time-/Timestamp-/Duration-Werte. JSON-Ausgaben bleiben unverändert;
generierte Clients werden daran angepasst. Native Date-/Time-/UTC-Timestamp-
Parameter und Millisekundenspeicherung sind auf MariaDB, MySQL und PostgreSQL
geprüft. Falsche Speichertypen/Präzision scheitern vor Anwendung mit Fundstelle.
Explizites [`sql_decode<T>`](sql-results.de.md) liefert typisierte Skalar-/Record-/
Array-Ergebnisse mit Kardinalitäts- und NULL-Prüfung; das Rohzeilen-JSON-Format
bleibt unverändert. Siehe [Regeln](dates.de.md),
[HTTP-Nachweise](release-readiness/0.9.9-calendar-http.de.md) und
[SQL-Nachweise](release-readiness/0.9.9-calendar-sql.de.md).
