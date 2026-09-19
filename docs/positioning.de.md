# Zelyra-Positionierung und Alleinstellungsmerkmale

Deutsch · [English](positioning.md)

Zelyra ist für den Teil der Softwareentwicklung gedacht, der häufig
wiederholt wird und trotzdem starke Korrektheitsgarantien benötigt:
datenbankbasierte Businessanwendungen.

Digitale Souveränität ist ein durchgängiges Entwicklungskriterium: Zelyra darf
weder einen Cloud- oder KI-Anbieter voraussetzen noch Produkt-Telemetrie
erheben oder externe Effekte verbergen. Das sind verbindliche Prinzipien,
keine Behauptung, dass Portabilität oder Verifikation bereits vollständig
umgesetzt sind. Siehe [Manifest](MANIFESTO.de.md) und [Roadmap](ROADMAP.de.md).

## Was Zelyra unterscheidet

### 1. Eine Quelle der Wahrheit vom Schema bis zur Anwendung

Eine Tabellendefinition kann Datenbank-DDL, Fachtypen, SQL-Prüfungen,
Formularvalidierung, API-Strukturen und CRUD-Ansichten speisen. Ein fachlicher
Fakt soll einmal beschrieben und überall wiederverwendet werden.

### 2. SQL bleibt First-Class und wird prüfbar

Zelyra versteckt komplexes SQL nicht hinter einem verpflichtenden ORM. Native
SQL-Blöcke bleiben als SQL erkennbar; der Compiler prüft Tabellen, Spalten,
Parameter, Nullfähigkeit und Ergebniszuordnung gegen das deklarierte Schema.

### 3. Webfunktionen für Businessanwendungen sind Sprachbausteine

Formulare, CRUD, Suche, Filter, Pagination, Authentifizierung,
Berechtigungen und typisierte APIs gehören zu einem zusammenhängenden Modell.
Sie sind keine Sammlung unabhängiger Framework-Adapter mit jeweils eigenen
Typen und Konventionen.

### 4. Sichere Defaults sind sichtbar und durchsetzbar

HTML-Escaping, parametrisierte SQL-Abfragen, CSRF-Schutz, Null Safety,
Autorisierung und Capability-Prüfungen sind standardmäßig vorgesehen.
Sensible Ausnahmen bleiben ausdrücklich, damit Komfort nicht unbemerkt zu
weitreichenden Rechten wird.

### 5. Beweise werden ehrlich ausgewiesen

`PROVEN`, `RUNTIME_CHECK`, `UNPROVEN` und `FAILED` haben unterschiedliche
Bedeutungen. Zelyra darf eine Runtime-Prüfung niemals als mathematischen
Beweis ausgeben. Dadurch bleiben Verifikationsergebnisse für Entwicklung und
CI belastbar.

### 6. Einfacher Einstieg und vollständige Sprache zugleich

`crud Customer -> customers` ist bewusst kurz. Wenn generiertes Verhalten
nicht genügt, bleiben normale Funktionen, natives SQL, eigene Formulare,
Ansichten, Aktionen, APIs und Geschäftslogik möglich. Zelyra ist keine
abgeschlossene Low-Code-Plattform und verlangt kein ORM.

### 7. Deployment ohne Framework-Stack

Der eingebaute Server, der MariaDB-zentrierte Workflow, die Docker-Compose-
Vorlage, die explizite Portwahl und einfache Quellcode-/Release-Installation
sollen den ersten lauffähigen Einstieg erleichtern. Produktionsdeployment
wird weiterhin ausgebaut.

### 8. Sichere anpassbare Views statt einer generierten Sackgasse

Zelyra-Views sollen mit sinnvollen Defaults beginnen und lokal anpassbar
bleiben. Benannte Layouts und Komponenten können Fallback-Inhalte anbieten,
während der Compiler die Komposition explizit hält und generierte Datenpfade,
Escaping, Authentifizierung und Autorisierung unverändert absichert. Das ist
eine Produkt-Richtung und keine Behauptung, dass ein einzelnes View-Feature
allein einzigartig ist; der Wert entsteht aus der Kombination von Einfachheit
mit demselben Datenbank- und Sicherheitsmodell.

Das gemeinsame CRUD-View-Profil ist ein konkretes Beispiel: Eine Liste in
`view.fields` kann erzeugte Liste, Detailansicht und Formulare prägen, während
explizite lokale Überschreibungen möglich bleiben.

## Was heute implementiert ist

Das Repository demonstriert bereits schemaabhängiges SQL, MariaDB-CRUD,
Formulare, typisierte APIs, Authentifizierung, direkte und rollenbasierte
Berechtigungen, CSRF-Schutz, Capabilities, Contracts und einen ersten
Verifikationsbefehl. Das sind implementierte Funktionen und keine Versprechen
über die vollständige langfristige Sprachspezifikation.

## Was weiterhin Ziel ist

Zelyra wird derzeit mit Rust gebootstrapped. Rust ist ein internes
Implementierungsdetail des Compilers und der Runtime, keine Voraussetzung für
die Nutzung veröffentlichter Zelyra-Anwendungen. Langfristig soll ein Weg zum
Self-Hosting entstehen: Zuerst bleibt der Bootstrap-Compiler stabil, danach
werden geeignete Compilerschichten schrittweise in Zelyra umgesetzt und mit
einem bestehenden vertrauenswürdigen Bootstrap-Compiler gebaut.

Benutzerverwaltung, umfangreichere Verifikation, Produktionspaketierung und
der selbsthostende Compiler sind noch nicht vollständig. Diese Ziele müssen
sichtbar von der implementierten Basis getrennt bleiben.

## Praxistest des Produkts

Der wichtigste Praxistest bleibt eine echte Maschinenverwaltung: Abteilungen
und Maschinen einmal definieren, das MariaDB-Schema anwenden und eine sichere
CRUD-Anwendung mit Beziehungen, Filtern, Berechtigungen und eigener
Geschäftslogik erhalten. Funktionen, die diesen Weg nicht verbessern und
dabei Kontrolle, Sicherheit oder Erweiterbarkeit opfern, gehören nicht in
den Sprachkern.
