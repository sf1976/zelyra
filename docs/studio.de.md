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
lokale Parameter und Bindungen. F12 öffnet die gespeicherte Funktions- oder
Typdefinition in derselben oder einer direkt importierten Datei. Umschalt+F12
findet exakt gleiche lexikalische Bezeichner in gespeicherten `.zyl`-Dateien.
Diese Treffer sind keine scopebewussten semantischen Referenzen. Speichere
geänderte Dateien vor der Navigation, damit die Fundstellen stimmen.

Compilerdiagnosen werden während der Bearbeitung von Zelyra-Projektdateien
aktualisiert. Bei vorhandenen `.zyl`-Dateien prüft der Compiler den
ungespeicherten Puffer als In-Memory-Overlay. Vorschauen neu erzeugter Dateien
verwenden weiterhin eine private temporäre Projektkopie. Beide Wege schreiben
den Puffer nicht in das Projekt. **Starten** führt `main.zyl` ausdrücklich aus,
höchstens 15 Sekunden und mit
64 KiB je Ausgabekanal. Beim markierten Rechnungstutorial startet die Aktion
die isolierte Compose-Datenbank, richtet Schema und Beispieldaten ein und
öffnet danach die Webvorschau. Andere Webprojekte benötigen eine erreichbare
Datenbank. Unter **Logs** kannst du strukturierte Start-/Setup-Ereignisse sowie
Programm- und Vorschauausgaben filtern. Erkannte geheime Umgebungs- und lokale
`.env`-Werte werden redigiert. Das Programm erhält keine interaktive Eingabe.

Die **Projektkarte** zeigt nur eindeutige Beziehungen aus dem Compiler-
Projektmodell und verknüpft Knoten mit gespeicherten Quellpositionen. Dynamische
und nicht aufgelöste Beziehungen bleiben sichtbar unvollständig, statt
hergeleitet zu werden.

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
Umbenennen nicht angepasst. `main.zyl` und `zelyra.toml` sind geschützt.
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

## Umfang

Studio ist eine lokale IDE-Grundlage und noch keine vollständige IDE.
Die Datenbankwerkbank unterstützt ausgewählte Schema-/Datenbankaktionen, aber
keinen beliebigen Datenbankdesigner. Die CRUD-Erzeugung arbeitet mit bereits
deklarierten Tabellen und prüft den erzeugten Code vor dem Schreiben.
