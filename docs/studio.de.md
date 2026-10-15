# Zelyra Studio

Zelyra Studio ist der mitgelieferte lokale Webeditor. Starte ihn im
Projektverzeichnis mit:

```sh
zelyra editor
```

Die CLI startet den lokalen Editor und öffnet den Standardbrowser automatisch.
Falls keine grafische Browsersitzung verfügbar ist, zeigt sie die URL zum
manuellen Öffnen an. Auf SSH- oder headless-Systemen lässt sich der Browserstart
mit `zelyra editor --no-open` überspringen. Studio braucht zum Bearbeiten weder
eine laufende Datenbank noch die gestartete Webanwendung.

Der Editor ist nur unter `127.0.0.1` erreichbar. Die Startadresse enthält ein
zufälliges Sitzungstoken. Schreibende Anfragen benötigen dieses Token und
müssen vom selben Browser-Origin kommen.

## Darstellung wechseln

Über das Farbsymbol oben rechts kannst du zwischen dunkler und heller
Darstellung wechseln. Die Auswahl wird in diesem Browser gespeichert und beim
nächsten Öffnen vor dem Laden des Studio-Stylesheets angewendet. So vermeidet
Studio einen sichtbaren Wechsel aus dem dunklen Standard. Die Umschaltung ist
eine normale Schaltfläche und lässt sich mit Tab sowie Eingabe- oder Leertaste
bedienen. Editor, Syntaxfarben, Dateibaum, Hilfebereich, Datenbankwerkbank und
Dialoge verwenden dieselben Farbwerte. Für die hellen Text- und Syntaxfarben
prüft der Browser-E2E-Test mindestens 4,5:1 Kontrast auf den Arbeitsflächen.
Auch bei der unterstützten Mindestbreite von 360 CSS-Pixeln bleibt der
Darstellungsschalter erreichbar, ohne dass Studio horizontal überläuft.

## IDE-Werkzeuge verwenden

Mit **F1** oder über die Hilfeaktion öffnest du Beschreibung, Optionen und ein
Beispiel zum unterstützten Sprachbegriff am Cursor. **Strg+Umschalt+P** öffnet
den durchsuchbaren Aktionsfinder. **Strg+Leertaste** fordert
Vervollständigungsvorschläge an, darunter Projekterklärungen sowie unterstützte
lokale Parameter und Bindungen. F12 öffnet die aufgelöste Funktionsdefinition
in derselben oder einer importierten Datei. Umschalt+F12 zeigt Referenzen auf
diese Funktion samt Deklaration. Beide Aktionen verwenden den verlinkten
Compiler-AST und berücksichtigen alle geöffneten, ungespeicherten `.zyl`-Dateien
als gemeinsame Projektanalyse. Weitere Symbolarten unterstützt dieser
semantische Index noch nicht. Die freie projektweite Textsuche bleibt ein
getrenntes Werkzeug.

Mit **F2** lässt sich eine aufgelöste Funktion umbenennen. Studio zeigt vorab
alle betroffenen Dateien und prüft die Änderung mit dem Compiler. Die Übernahme
ändert die geöffneten Puffer; jede Datei kann separat rückgängig gemacht und
gespeichert werden. **Alle speichern** schreibt bis zu 16 Dateien gemeinsam
und legt vorher ein privates Wiederherstellungsjournal unter `.zelyra/` an.
Nach einer Unterbrechung stellt Studio nicht abgeschlossene Schreibvorgänge
beim nächsten Start wieder her. Beim Umbenennen einer `.zyl`-Datei zeigt Studio
zuerst alle erkannten Importdateien an, prüft das verschobene Projekt mit dem
Compiler und übernimmt die Datei- und Importänderungen anschließend gemeinsam
über das Journal. Vorher werden offene Änderungen gespeichert; veraltete
Vorschauen werden abgewiesen. Der Vorgang ist auf 64 Dateien und insgesamt
8 MiB begrenzt. Andere Dateitypen werden ohne Importänderung umbenannt.

Compilerdiagnosen werden während der Bearbeitung von Zelyra-Projektdateien
aktualisiert. Bei vorhandenen `.zyl`-Dateien prüft der Compiler den
ungespeicherten Puffer und weitere geänderte `.zyl`-Tabs als In-Memory-Overlays
(höchstens 16 Dateien und 1 MiB Quelltext pro Analyse). Vorschauen neu erzeugter
Dateien verwenden weiterhin eine private temporäre Projektkopie. Beide Wege schreiben
den Puffer nicht in das Projekt. **Starten** führt `main.zyl` ausdrücklich aus,
höchstens 15 Sekunden und mit
64 KiB je Ausgabekanal. Beim markierten Rechnungstutorial startet die Aktion
die isolierte Compose-Datenbank, richtet Schema und Beispieldaten ein und
öffnet danach die Webvorschau. Andere Webprojekte benötigen eine erreichbare
Datenbank. Unter **Logs** kannst du strukturierte Start-/Setup-Ereignisse sowie
Programm- und Vorschauausgaben filtern. Erkannte geheime Umgebungs- und lokale
`.env`-Werte werden redigiert. Das Programm erhält keine interaktive Eingabe.

Die **Projektkarte** zeigt eindeutige Beziehungen aus dem Compiler-
Projektmodell: Module, Tabellen, SQL-Abfragen, Records, Formulare, Tabellen-
ansichten, CRUD, Seiten, Views, APIs und Funktionen. Sie unterstützt mehrere
Verbraucher derselben Tabelle und alle geöffneten ungespeicherten `.zyl`-Dateien
als Overlays. Knoten führen zur tatsächlichen Deklaration beziehungsweise zum
SQL-Token. In der Leiste am Editorkopf erscheinen Beziehungen zum aktuell
gewählten Knoten und führen ebenfalls zur Quelle. Der NULL-Radar zeigt nur
statische Nullability-Diagnosen des Compilers mit `E-SQL-004`; er prüft keine
laufende Datenbank. Dynamische oder vom konservativen SQL-Analysator nicht
aufgelöste Beziehungen bleiben sichtbar unvollständig.

Bei einem parsebaren Projekt zeigt die Contract-Leiste Ergebnisse des echten
Zelyra-Verifiers für den aktuellen ungespeicherten Puffer. `PROVEN` (grün,
`V-001`) bedeutet bewiesen; `RUNTIME_CHECK` (gelb, `V-002`) erfordert eine
Laufzeitprüfung; `FAILED` (rot, `V-004`) ist widerlegt; `UNPROVEN` (neutral,
`V-003`) liefert keinen Beweis. Mit Klick, Eingabe oder Leertaste liest du die
Begründung eines Markers. Bei Syntax- oder Typfehlern verschwinden ältere
Marker, statt einen veralteten Beweis anzuzeigen. Die Leiste folgt dem aktuellen
Dokument; Dokumentversionen verhindern verspätete Ergebnisse.

Bei einer Ausdruckslücke `_` öffnet **Strg+Leertaste** die CodeMirror-
Vorschläge aus dem Compilerkontext. Sie enthalten sichtbare Bindungen mit
bekanntem passenden Typ sowie parameterlose Funktionen, deren deklarierte
Capabilities zum aktuellen Funktionskontext passen. Vorschläge mit Contracts
sind zur Prüfung gekennzeichnet. Wähle per Tastatur oder Klick; nur die Stelle
von `_` wird ersetzt und Undo stellt sie wieder her. Ein `_`-Wildcard in einem
Match-Muster ist keine Ausdruckslücke. Typinferenz für unannotierte lokale
Werte und allgemeine Subtypisierung werden für diese Vorschläge noch nicht
genutzt; deshalb können gültige Ausdrücke fehlen.

### Lokaler Ereigniskanal

Studio öffnet `GET /__zelyra/editor/api/events` als SSE-Stream mit `fetch` und
dem Sitzungstoken im `X-Zelyra-Editor-Token`-Header. Der Token steht nicht in
der Stream-URL. Der Server prüft lokalen Host, Token und vorhandene Browser-
Origin-/Fetch-Metadaten. Schreibende APIs bleiben an den Token-Header und den
exakt passenden Origin gebunden. Bei Verbindungsabbruch verbindet sich Studio
mit begrenzter Wartezeit erneut; einzelne Analyseanfragen funktionieren auch
ohne Stream.

Jedes Datenereignis ist ein JSON-Objekt mit `schema: 1`, `kind`, `document`
(`path`, `version`, bei Browseranalysen `clientId`), `workspaceRevision` und
`payload`. Der Kanal sendet `analysis.progress`, `diagnostics`,
`verification`, `workspace.changed`, `analysis.invalidated` und
`preview.changed`. Studio übernimmt Analyseergebnisse nur für die aktuelle
Client- und Dokumentversion. Der Server begrenzt auf vier Streams mit je 32
ausstehenden Nachrichten und trennt langsame Empfänger. Ein begrenzter
Metadatenwächter meldet außerdem externe Änderungen an `.zyl`-, `.css`- und
`zelyra.toml`-Dateien im Projektbaum. Saubere geöffnete Puffer werden neu
geladen; ungespeicherte Änderungen bleiben erhalten und Studio zeigt einen
Hinweis. Der Wächter folgt keinen Symlinks. Bei grober Zeitauflösung des
Dateisystems kann er Änderungen gleicher Dateigröße mit unverändertem
Zeitstempel übersehen.

### Laufzeitbudget und Rekursion

Über **Budget** öffnest du die Einstellungen für einen Konsolenlauf. Ein
Schalter aktiviert `--max-steps`; der Regler deckt 1.000 bis 1.000.000 Schritte
ab, das Zahlenfeld erlaubt ganze Werte von 1 bis 100.000.000. Ohne Schalter
bleibt das bestehende Verhalten ohne Schrittbudget erhalten. **Starten** gibt
den gewählten Wert als getrennte CLI-Argumente an `zelyra run` weiter. Der
Server begrenzt parallele Studio-Läufe auf einen; ein Konsolenlauf kann im
Ausgabefenster abgebrochen werden. Die bisherige Zeitgrenze von 15 Sekunden
und die Ausgabebegrenzung bleiben zusätzlich wirksam. Eine Budgetüberschreitung
zeigt die echte `E-RUNTIME-001`-Ausgabe an. Webvorschauen verwenden nicht
`--max-steps`. Bereits ausgegebene Programmzeilen bleiben auch dann sichtbar,
wenn der Lauf später mit einem Laufzeitfehler endet.

Die Anzeige liest `[runtime] max_stack_depth` aus `zelyra.toml` und zeigt bei
fehlendem Eintrag den Runtime-Standard 20. Der erlaubte Konfigurationsbereich
ist 1 bis 500; fehlerhafte Konfiguration wird als Fehler angezeigt. Der
compilerbasierte Aufrufgraph markiert mögliche direkte und gegenseitige
Rekursion auch im ungespeicherten aktuellen Puffer. Bei mehr als 500
Funktionen meldet Studio die Analyse als unvollständig. Aufrufgraphen können
keine exakte Rekursionstiefe belegen; Studio zeigt keine erfundene Tiefe oder
Schrittzahl an. Die angezeigte Laufzeit in Millisekunden stammt aus dem
tatsächlichen Programmlauf.

### Serverseitige PDF-Vorschau

Über **PDF-Vorschau** wählst du eine deklarierte `view` und gibst Beispieldaten
als JSON-Objekt ein, etwa `{"number":"R-104"}` für eine View mit
`{number}`. Erst **PDF erzeugen** startet die Vorschau. Studio prüft das
Projekt einschließlich des aktuellen ungespeicherten `.zyl`-Puffers und nutzt
denselben lokalen Renderer wie `render_pdf("ViewName", record)`. Das Ergebnis
wird als echte PDF-Datei im isolierten Browserrahmen gezeigt und kann
heruntergeladen werden. Studio führt dafür keine Projektfunktion aus und
fragt keine Datenbank ab. Die eingegebenen Beispieldaten verlassen den lokalen
Studio-Server nicht und werden nicht in die Projektdatei geschrieben.

Der Renderer erzeugt derzeit ein durchsuchbares Text-PDF aus sichtbarem
View-Inhalt; CSS, Bilder und Diagramme werden darin nicht als grafisches Layout
wiedergegeben. Der Browser kann den eingebetteten PDF-Rahmen je nach
PDF-Unterstützung anders darstellen; der Download enthält dieselben
generierten PDF-Bytes. Ungültige Daten, unbekannte Views und Compilerfehler
werden als Fehler angezeigt. Diese Funktion ist getrennt von der vorhandenen
Präsentations-Druckansicht, die weiterhin den Browser-Druckdialog verwendet.

### MariaDB-Outbox prüfen

**Outbox** liest die vorhandene transaktionale MariaDB-Outbox erst beim Öffnen
und auf **Aktualisieren**. Die Ansicht zeigt 25 Ereignisse je Seite mit Status,
Versuchen, Empfänger, Zeitpunkten, redigiertem Fehlercode und dem Namen des
einen derzeit unterstützten PDF-Anhangs. Nachrichtentext und Anhangsbytes
werden nicht an den Browser übertragen. Der lokale Studio-Token schützt auch
die lesende API. Ohne konfigurierte MariaDB-Verbindung zeigt die Ansicht einen
Konfigurationshinweis; `zelyra outbox setup main.zyl` legt die Tabelle
ausdrücklich an.

Nur bei `exhausted` erscheint **Erneut einreihen**. Nach deiner Bestätigung
ruft Studio die bestehende fachliche Requeue-Operation auf. Sie setzt das
Ereignis für einen späteren Worker-Lauf zurück und versendet keine Nachricht.
Ein Worker wird in Studio weder gestartet noch automatisch konfiguriert.
Zähler und Zeilen werden in zwei Datenbankabfragen gelesen; bei gleichzeitigen
Worker-Aktionen kann sich die Trefferzahl zwischen den Abfragen ändern.

## Rechnungstutorial in Studio starten

Bei einem erzeugten Rechnungstutorial startet Studio beim Klick auf
**Starten** den isolierten MariaDB-Dienst und richtet das Schema ein. Nach dem
Anlegen mit `--no-start` öffnest du das Projekt so:

```sh
zelyra editor .
```

Wähle `main.zyl` und klicke auf **Starten**. Studio startet die Datenbank,
richtet Schema und Beispieldaten ein und öffnet danach die Ablauf-Startseite.
**Logs** zeigt strukturierte Start- und Vorschauausgaben. Rechnungsdaten
werden als `JJJJ-MM-TT` oder `YYYY-MM-DD` eingegeben; die Rechnungsnummer
vergibt der Server beim Speichern. Die lokale Vorschau liest die
Projektverbindung aus der geschützten `.env`. Wenn eine CRUD-Seite keine Daten
lädt, prüfe **Logs** auf Setupfehler; Zugangsdaten werden nicht auf der Seite
ausgegeben.

## Quellcode bearbeiten

Die Dateiliste zeigt Zelyra-Quelldateien des Projekts. Tabs halten mehrere
Dateien geöffnet. Suche, Vervollständigung, Formatierung, Compilerdiagnosen,
Rückgängig/Wiederholen und das Speichern mit Versionsprüfung unterstützen den
Arbeitsablauf. Über **Umbenennen** und **Löschen** kannst du Quelldateien
bearbeiten. Studio verlangt vorher eine Bestätigung; Importverweise werden beim
Beim Umbenennen einer `.zyl`-Datei werden erkannte `import`-Pfade nach einer
Compilerprüfung gemeinsam mit der Dateiänderung angepasst. `main.zyl` und
`zelyra.toml` sind geschützt.
Projektpfade bleiben begrenzt, und Dateiaktionen prüfen den gespeicherten Hash.
Änderungen, die eine andere Anwendung seit dem Laden vorgenommen hat, werden
abgelehnt; lade die Datei neu und wiederhole die Aktion.

## Datenbankschema ansehen

Wähle **Datenbank** und lade das Schema. Studio benötigt eine
`DATABASE_URL` aus der Prozessumgebung oder der lokalen Projektdatei `.env`.
Das Studio liest Tabellen- und Spaltenmetadaten. Datenbankänderungen führt es
nicht ohne eigene, bestätigte Aktion aus.

Die Übersicht enthält eine anklickbare Beziehungskarte mit Tabellen und
Fremdschlüsselpfeilen. Sie gruppiert deklarierte Tabellen nach bekanntem
Quellmodul und vergleicht Spalten, Typen, NULL-Erlaubnis, Primärschlüssel,
automatische IDs, Eindeutigkeit und Fremdschlüssel mit der Live-Datenbank. Fehlende oder nur in
der Datenbank vorhandene Tabellen und Spalten werden markiert. Die Zuordnung
zum Quellmodul ist eine Orientierung und setzt keine Zugriffsrechte durch.

Der **Datenbrowser** zeigt Datensätze nur auf ausdrücklichen Klick und in
Seiten mit höchstens 25 Zeilen. Er ist schreibgeschützt; Tabellen- und
Spaltennamen stammen aus der geladenen Schemaansicht, die SQL-Bezeichner werden
backendgerecht maskiert, und Werte werden als Text dargestellt. Studio lädt
keine Datensätze automatisch beim Öffnen der Werkbank.

Der **Diagramm-Assistent** erstellt aus einer deklarierten Live-Tabelle ein
gruppiertes Balken- oder Liniendiagramm. Gruppierung und Kennzahl wählst du
über Felder aus; verfügbar sind Anzahl sowie Summe, Durchschnitt, Minimum und
Maximum. Summe und Durchschnitt lassen nur numerische Datenbankspalten zu.
Studio erzeugt die Abfrage aus Schema-Bezeichnern, führt sie lesend aus und
zeigt höchstens 20 Gruppen. Es gibt kein frei eingebbares SQL. Du kannst einen
exakten Textfilter und einen inklusiven Zeitraum auf einer `DATE`-Spalte
setzen. Werte werden als Parameter gebunden; Tabellen- und Spaltenauswahl
bleiben an die deklarierten und live vorhandenen Projektfelder gebunden. Die
Konfiguration wird als `chart-<name>.zchart.json` gespeichert und enthält keine
abgefragten Datensätze. Beim Laden wird sie geprüft und die Vorschau erneut aus
der aktuellen Datenbank berechnet. Ein vorhandener Zelyra-Bericht desselben
Typs lässt sich aktualisieren; eine gleichnamige fremde Datei wird nicht
überschrieben.

Mehrere Diagramme können als Folien zu einer lokalen Präsentation mit Titel
und Einleitung zusammengestellt werden. Die Konfiguration wird als
`presentation-<name>.zelyra-presentation.json` gespeichert. **Druckansicht /
PDF** erstellt eine Folienvorschau und öffnet den Druckdialog des Browsers; dort
kann die PDF-Datei lokal gespeichert werden. Die Folien speichern geprüfte
Diagrammkonfigurationen statt eines Datenbank-Schnappschusses. Beim Laden einer
Präsentation bleiben Titel, Einleitung und Folien erhalten; beim Erstellen der
Druckansicht werden die Diagrammabfragen erneut ausgeführt.

Dieser Studio-Ausschnitt bietet keine frei formulierbaren Filter,
Zeitstempel- oder Zeitzonenbereiche, Einbindung gespeicherter Diagramme in
laufende Anwendungsseiten oder Compiler-Typprüfung für die JSON-Dateien. Die
Präsentation ist ein einfacher Folien- und Druckablauf, kein allgemeiner
Diagramm- oder Präsentationseditor.

Im geführten **Spaltenassistenten** lassen sich neue Tabellenfelder ergänzen
und einfache bestehende Felder ändern. Schlüssel-, eindeutige und Felder mit
Defaultwert sind von der visuellen Bearbeitung ausgeschlossen. Vor der
Übernahme prüft Zelyra den Quelltext mit dem Projektcompiler und zeigt Diff,
Diagnosen und den Hinweis auf den erforderlichen Schema-Plan. Die Übernahme
ändert ausschließlich die Zelyra-Datei. Vor einer Datenbankmigration muss der
Nutzer `zelyra db plan main.zyl` prüfen und die Datenbankaktion separat
ausführen.

Für ein MariaDB-Projekt mit bereits konfigurierter Verbindung kann **MariaDB-
Datenbank anlegen** die deklarierte Datenbank ausdrücklich anlegen oder ihre
Existenz sicherstellen. Studio fragt vorher nach Bestätigung. Bei einem lokal
generierten Compose-Projekt verwendet es dafür das geschützte
`MARIADB_ROOT_PASSWORD` aus `.env`; andernfalls versucht es die konfigurierte
Datenbankverbindung. Zugangsdaten bleiben serverseitig. Die Aktion wendet
keine Tabellenmigration an. Backend- und Zugangsdaten lassen sich in dieser
Version noch nicht in Studio konfigurieren; PostgreSQL- und MySQL-
Datenbankanlage sind nicht enthalten.

In dieser Version unterstützt die visuelle Live-Introspektion MariaDB und
PostgreSQL entsprechend der geprüften `db map`-Unterstützung. Für MySQL zeigt
Studio ausdrücklich an, dass die Schema-Introspektion noch nicht unterstützt
wird. Die MySQL-Laufzeitunterstützung für typisierte, parametrisierte Abfragen
bleibt davon getrennt.

## CRUD-Quelltext erzeugen

Bei einer deklarierten Tabelle, die auch live existiert und noch keine CRUD-
Ressource besitzt, kannst du eine importierte Quelldatei, einen Ressourcennamen,
einen Titel und sichtbare Felder auswählen. Spalten, die nicht live vorhanden
sind, können nicht ausgewählt werden.

Speichere Änderungen an der ausgewählten Datei zunächst im Editor. **Code
prüfen und Vorschau zeigen** baut den Kandidaten in einem privaten temporären
Projekt auf demselben Rechner und ruft den normalen Zelyra-Projektcompiler
auf. Die Vorschau zeigt den generierten Code, die Änderung und Compilerfehler.
Sie ändert die Projektdatei nicht.

Erst **Änderung übernehmen** schreibt die geprüfte Fassung in die Datei.
Unmittelbar davor prüft Studio den Datei-Hash erneut. Hat sich die Datei seit
der Vorschau geändert, wird der Schreibvorgang abgelehnt und die Vorschau muss
neu erstellt werden. Das erzeugte CRUD ist Quelltext; es migriert keine
Datenbank und verändert keine Datensätze.

## Statische Canvas-Vorschau (Arbeitsstand 1.1.0)

**Canvas** zeigt eine deklarierte Seite oder View neben dem Quelltext. Die
Seitenkomposition und die Ersetzung von Datenwerten verwenden dieselben
Funktionen wie die Webauslieferung. Werte in **Beispieldaten (JSON)** werden
HTML-escaped; Listen aus JSON-Objekten können bestehende `for`-Blöcke füllen.
Alle geöffneten ungespeicherten `.zyl`-Puffer werden vom Compiler als Overlays
analysiert. Projektabfragen, Handler, `main()` und JavaScript werden in der
Vorschau nicht ausgeführt. Der Browserrahmen hat ein leeres `sandbox`-Attribut
und eine einschränkende CSP. Die Vorschau zeigt nur ausdrücklich eingegebene
Beispieldaten; fehlende Laufzeitdaten werden nicht erfunden.

Die Canvas verwendet die konfigurierte Stylesheet-Auswahl und
`zelyra.theme.css`. Ungespeicherte Änderungen dieser Theme-Datei aktualisieren
den Canvas ohne Reload von Studio. Der Farbwähler erkennt Hex-Werte von
`--z-color-*`-Deklarationen außerhalb von CSS-Kommentaren. **Im Quelltext
ändern** öffnet die Theme-Datei und erzeugt einen rückgängig machbaren
Editor-Edit. Für Primär- und Statushintergründe setzt er die jeweilige
`--z-color-on-*`-Textfarbe auf Schwarz oder Weiß mit dem höheren Kontrast.
Hover- und Soft-Farben sowie andere thematisch gekoppelte Variablen bleiben
manuell zu gestalten. Der Quellknopf springt zur Seiten- oder View-Deklaration;
einzelne verschachtelte Komponenten haben noch keine Source-Span-Navigation.

## Capability- und Pfadmatrix (Arbeitsstand 1.1.0)

Die Schaltfläche **Rechte** analysiert die aktuelle `.zyl`-Datei einschließlich
ungespeicherter Änderungen. Die Matrix zeigt `uses`-Deklarationen, tatsächlich
erkannte Capability-Operationen und die Freigaben aus `zelyra.toml`.
Freigaben ohne erkannten Funktionsbedarf sind nur Hinweise; generierte Routen
können sie dennoch benötigen. Bei
wörtlichen Dateipfaden verwendet sie dieselbe kanonisierte Root-Prüfung wie
die Runtime. Ein dynamischer Pfad bleibt als unbekannt markiert; ein zur
Analysezeit erlaubter Pfad wird zur Laufzeit erneut geprüft. Bei Compilerfehlern
ist der Befund als unvollständig gekennzeichnet.

Eine fehlende Freigabe lässt sich über **Freigabe prüfen** vorbereiten. Studio
zeigt die vollständige neue `zelyra.toml` vor dem Speichern. Erst
**Freigabe übernehmen** schreibt die Änderung. Dabei prüft Studio den
Datei-Hash erneut, bewahrt vorhandene TOML-Kommentare und lehnt geänderte
Konfigurationen ab. Ohne `zelyra.toml` hat die Runtime keine projektweite
Capability-Schranke; Studio erzeugt deshalb keine vorgetäuschte Freigabe.
Der Bericht umfasst analysierte Funktionsaufrufe; dynamische Aufrufe und
automatisch generierte Webrouten sind keine vollständig statisch bewerteten
Rechteketten.

Studio prüft `.zyl`-, `.css`- und `zelyra.toml`-Dateien im freigegebenen
Projektbaum alle 1,5 Sekunden auf äußere Änderungen. Der Wächter liest dafür
Dateimetadaten und folgt keinen Symlinks. Saubere geöffnete Puffer werden neu
geladen; ungespeicherte Änderungen bleiben erhalten und Studio zeigt einen
Hinweis. Der Wächter ist auf die bestehende Baumtiefe und Dateizahl des
Editors begrenzt. Auf Dateisystemen mit grober Zeitauflösung kann eine
Änderung mit gleicher Länge und gleichem Zeitstempel unerkannt bleiben.

## Umfang

Studio ist eine lokale IDE-Grundlage und noch keine vollständige IDE.
Die Datenbankwerkbank unterstützt ausgewählte Schema-/Datenbankaktionen, aber
keinen beliebigen Datenbankdesigner. Die CRUD-Erzeugung arbeitet mit bereits
deklarierten Tabellen und prüft den erzeugten Code vor dem Schreiben.
