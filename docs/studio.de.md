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

Compilerdiagnosen werden während der Bearbeitung gespeicherter Zelyra-
Projektdateien aktualisiert. Die Prüfung arbeitet auf einer privaten
temporären Projektkopie und schreibt nicht in den ungespeicherten Editorpuffer.
**Starten** führt `main.zyl` ausdrücklich aus, höchstens 15 Sekunden und mit
64 KiB je Ausgabekanal. Erkannte geheime Umgebungs- und lokale `.env`-Werte
werden redigiert. Studio startet Anwendung und Datenbank nicht automatisch und
bietet dem Programm keine interaktive Eingabe.

Die **Projektkarte** zeigt nur eindeutige Beziehungen aus dem Compiler-
Projektmodell und verknüpft Knoten mit gespeicherten Quellpositionen. Dynamische
und nicht aufgelöste Beziehungen bleiben sichtbar unvollständig, statt
hergeleitet zu werden.

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
Das Studio liest nur Tabellen- und Spaltenmetadaten. Es liest keine
Datensätze und führt keine Schemaänderungen aus.

Die Übersicht enthält eine anklickbare Beziehungskarte mit Tabellen und
Fremdschlüsselpfeilen. Sie gruppiert deklarierte Tabellen nach bekanntem
Quellmodul und vergleicht Spalten, Typen, NULL-Erlaubnis, Primärschlüssel,
automatische IDs, Eindeutigkeit und Fremdschlüssel mit der Live-Datenbank. Fehlende oder nur in
der Datenbank vorhandene Tabellen und Spalten werden markiert. Die Zuordnung
zum Quellmodul ist eine Orientierung und setzt keine Zugriffsrechte durch.

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
Die Datenbankansicht ist ein lesender Schemaüberblick, kein beliebiger
Datenbankdesigner. Die CRUD-Erzeugung arbeitet mit bereits deklarierten
Tabellen und prüft den erzeugten Code vor dem Schreiben.
