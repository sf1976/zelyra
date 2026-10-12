# In Zelyra 0.9.9 umgesetzt

**Stand:** Entwicklungszweig 0.9.9 · Sprachkompatibilitätslinie `0.1` · 2026-10-12
**Reifegrad:** Vorabentwicklung; nicht für den Produktiveinsatz freigegeben

Dies ist eine Entwicklungsübersicht, kein veröffentlichtes 0.9.9-Release. Die
offenen technischen Gates stehen im [Releaseplan 0.9.9](release-plans/0.9.9.de.md).
Die menschliche Abnahme bleibt dem konkreten 1.0.0-Kandidaten vorbehalten.

Diese Übersicht beschreibt, was die aktuelle Zelyra-Version kann. Sie führt die
Funktionen nach Anwendungsbereichen statt nach einzelnen Versionsnummern auf.
Ältere und neue Releases stehen im [Changelog](../CHANGELOG.md); offene Arbeit
steht in der [Roadmap](ROADMAP.de.md). Eine unabhängige menschliche Abnahme ist
für 1.0.0 vorgesehen.

## Was der Entwicklungszweig 0.9.9 bietet

- **Sprache und Compiler:** eine statisch geprüfte Sprache mit Funktionen,
  generischen Records, Enums und Klassen, benannten Funktionswerten, typisierten
  Sammlungen, `Option` und `Result`, Pattern Matching, Modulen, Contracts,
  Effekten und compilergeprüften SQL-/Webdeklarationen. Explizite skalare
  Textformatierung, Kalender-Tagesrechnung, Decimal-Rundung und kontrollierte
  Rekursion sind enthalten.
  Die genaue Typabdeckung und ihre Grenzen stehen im Abschnitt
  [Sprache und Compiler](#sprache-und-compiler).
- **Datenbanken:** schemaabhängige Abfragen, relationale Fremdschlüssel,
  Datenbankinspektion und geprüfte Setup-/Migrationspläne. MariaDB ist die
  primäre Laufzeitreferenz. MySQL 8.4.11 hat neben parametrisierten Abfragen
  nun einen isolierten Schemaablauf; PostgreSQL wird für die in der
  [Supportmatrix](database-support.de.md) ausgewiesenen Abläufe geprüft.
- **Geschäftsanwendungen:** Seiten, wiederverwendbare Views, Formulare,
  Tabellenansichten, Suche, Filter, Sortierung, Pagination, CRUD und typisierte
  APIs mit OpenAPI-Ausgabe.
- **Zelyra Studio:** lokaler Webeditor mit gespeicherter heller und dunkler
  Darstellung, Compilerdiagnosen, Formatierung, Navigation, Vervollständigung,
  Aufgabenhilfe, Datenbankwerkbank und Projektkarte. Ein kurzer Markenstart
  überbrückt das Laden; die CLI verwendet auf Wunsch Terminalfarben. Die Karte
  zeigt bekannte Beziehungen und begründet offene oder
  begrenzte Zuordnungen, statt sie zu erraten.
- **Rechnungstutorial:** eine mehrteilige Anwendung für Kunden, Artikel und
  Rechnungen mit Kennzahlen aus gespeicherten Daten. Tabler Core 1.6.1 wird
  lokal als CSS-Grundlage geladen. Dashboard, CRUD-Wege und druckbare Netto-,
  19%-Steuer- und Bruttoansicht sind für Desktop, schmale Bildschirme und Druck
  ausgelegt. Summen enthalten Entwürfe und sind keine gebuchte Umsatzstatistik.
- **Betrieb und Sicherheit:** CLI für Projektstart und Setup, Routenübersicht,
  Formatierung, Diagnose, Datenbank- und Schemawerkzeuge, Audit und
  Releaseprüfung. Authentifizierung, Rollen, tenant-gefiltertes CRUD, optionale
  lokale MFA und SMTP-Mail über eine transaktionale MariaDB-Outbox sind im
  jeweiligen dokumentierten Umfang verfügbar.
- **Migration:** ein statischer Laravel-Bericht und ein ausdrücklich
  angefordertes separates Zelyra-Grundgerüst mit Fundstellen und offenen
  Zuordnungen. Es wird keine Geschäfts- oder Sicherheitsparität behauptet.

Die Oberfläche und Tools sind lokal nutzbar. Verbindliche Cloud-Dienste,
Telemetrie und externe KI-Anbieter sind nicht erforderlich. Zelyra bleibt
experimentell: CI und automatisierte Prüfungen ersetzen keine menschliche
Produkt- oder Sicherheitsabnahme.

## Statuszeichen

- ✅ **Umgesetzt und geprüft:** im angegebenen Umfang verfügbar und durch Tests
  oder dokumentierte Release-/CI-Prüfungen belegt.
- 🧪 **Teilweise oder experimentell umgesetzt:** der funktionierende Teilumfang
  wird ausdrücklich beschrieben; daraus folgt keine breitere Unterstützung.

Kein Punkt bedeutet allgemeine Produktionsreife, universelle Portabilität oder
den Beweis, dass beliebige Anwendungen korrekt sind.

## Sprache und Compiler

- ✅ Lexer, Parser, Syntaxbaum, Namens- und Typprüfung, Funktionen, Ausdrücke,
  Bedingungen, Schleifen und unveränderliche Bindings als Standard;
  Veränderlichkeit muss ausdrücklich markiert werden.
- ✅ Nominale Fachdaten-Typen, Records mit geprüftem Feldzugriff, Arrays mit
  gebräuchlichen Operationen, deterministische typisierte Maps, `Option`,
  `Result` und exhaustives Pattern Matching als Anweisung oder Ausdruck; ein
  `match`-Ausdruck liefert einen einheitlich typgeprüften Wert. `class`-Instanzen
  teilen überprüfte Referenzidentität bei Zuweisung, Parameterübergabe, Rückgabe
  und in Sammlungen; typisierte Feldupdates ändern die geteilte Instanz. Records
  bleiben wertbasiert. Klassen haben ausdrücklich keine Methoden- oder
  Vererbungssyntax, keine zyklischen Referenzgraphen und keine direkte JSON-,
  HTTP- oder SQL-Serialisierung; API-Daten werden in Records übertragen.
- ✅ `Int` und `UInt` rechnen innerhalb ihrer 64-Bit-Bereiche; das direkt
  geschriebene `-9223372036854775808` wird unterstützt. Überlauf,
  Unterlauf und Division durch null werden als Runtimefehler gemeldet. Das
  mehrteilige `examples/type_system` prüft unterstützte Typen, erwartete
  Compiler-/Runtimefehler und dokumentiert partielle sowie fehlende Typen.
  `Unit` ist in Typsignaturen und JSON-`null`-Roundtrips nutzbar. `Decimal`
  verwendet Festkomma mit neun Nachkommastellen und exakter Basis-10-Arithmetik.
  `decimal_quantize_result` rundet mit explizitem Modus ohne Float-Zwischenwert;
  Multiplikation und Division können über neun Nachkommastellen abschneiden.
  Für verlustfreie JSON-Roundtrips wird Decimal als Dezimalstring serialisiert.
  Kalender-Tagesaddition/-differenz, geordnete Timestampvergleiche und geprüfte
  UTC-Dauerarithmetik sind verfügbar; Zeitzonen und Sommerzeit nicht. `Date`,
  `Time`, `Duration` und `Bytes` haben echte
  Runtimewerte, geprüfte Konstruktoren und JSON-Roundtrips. `List<T>` ist ein
  Alias für `T[]`; heterogene `Tuple<A, B, ...>`-Werte und deterministische
  `Set<T>`-Werte lassen sich erstellen, prüfen, iterieren und als JSON
  austauschen. Benutzerdefinierte Enums unterstützen Nutzdaten, Generics,
  Gleichheit, JSON und exhaustives Matching. Eigene generische Funktionen,
  Records und Enums erlauben explizite oder abgeleitete Typargumente; allgemeine
  Constraints, generische Aliase und generische Funktionsreferenzen fehlen.
  Benannte typisierte Funktionswerte sind implementiert; Closures bleiben
  außerhalb des Umfangs. `record_with` und
  `array_replace_result` liefern typgeprüfte neue Werte mit Wertsemantik.
  `for i in start..end` iteriert mit exklusivem Ende.
- ✅ Nicht-Unit-Funktionen müssen auf jedem erreichbaren Pfad einen passenden
  Rückgabewert liefern (`E-TYPE-RETURN-001`); nicht exhaustive Matches melden
  `E-TYPE-MATCH-001`. Die Laufzeit begrenzt verschachtelte
  Funktionsaufrufe kontrolliert; `zelyra run --max-steps <n>` aktiviert ein
  optionales Schrittbudget, standardmäßig bleibt es unbegrenzt.
- ✅ Stringwerte lassen sich lexikografisch vergleichen. `format_text(wert)`
  formatiert skalare Werte ausdrücklich und ohne implizite Konvertierung;
  Geldbeträge mit fester Skala verwenden `format_scaled_result`. Sammlungen,
  Records, Klassen, Enums, `Result` und Funktionen besitzen keine
  Anzeigekonvertierung.
  `json_decode_result<T>`,
  `read_text_result` und `write_text_result` geben erwartbare Parse- und
  Dateifehler als `Result` zurück. Der explizite getaggte Result-Codec
  `json_encode_tagged_result`/`json_decode_tagged_result<T,E>` erhält Ok- und
  Err-Variante sowie ihren jeweiligen Payload, ohne das bisherige
  `json_encode`-Wireformat zu ändern. `format_timestamp` zeigt UTC-Zeitstempel als
  ISO-8601; Capability- und Projektgrenzen bleiben aktiv. Exklusive
  Dateierstellung, temporäres Schreiben mit atomarem Austausch, Anhängen,
  Kopieren, Umbenennen und Result-basiertes Löschen sind verfügbar. Wörtliche
  Textsuche, Ersetzen und Teilen unterstützen mehrteilige Textdateiabläufe.
  `examples/notes_manager` demonstriert UTF-8-Notizen, Sicherungen vor
  Änderungen, Wiederherstellung, Protokollierung und automatisierte Tests.
- ✅ `zelyra check`, `build`, `run` und `serve`; menschenlesbare Diagnosen,
  stabile Diagnosecodes in unterstützten Maschinenformaten und Quellpositionen.
- ✅ Kanonische Formatierung mit `zelyra fmt` und nicht veränderndem
  `--check`-Modus.
- 🧪 Funktions-Capabilities und zur Laufzeit geprüfte Contracts sind für
  unterstützte Operationen implementiert. `zelyra verify` unterscheidet
  `PROVEN`, `RUNTIME_CHECK`, `UNPROVEN` und `FAILED` für eine begrenzte Menge
  von Ausdrücken und Pfaden; es ist kein allgemeiner Theorembeweiser.

Siehe die [formale Sprachspezifikation](specification.de.md), die
[Quellenlandkarte](source-authority.de.md) und die
[Sprachkapitel im Handbuch](handbook/de/handbuch.md).

## Datenbanken und SQL

- ✅ MariaDB ist das primäre getestete Runtime-Backend. Die CLI implementiert
  `zelyra db create|setup|bootstrap|inspect|map|plan|apply|history` für
  unterstützte Operationen. `zelyra db map <file.zyl>` vergleicht deklarierte
  Tabellen nach Quellmodul mit Live-Tabellen, Spalten und Fremdschlüsseln. Die
  Karte ist schreibgeschützt; Modulzuordnung ist unverbindlich und keine
  Autorisierungsregel.
- ✅ Tabellenschemata, Spalten, Schlüssel, Beziehungen, Indizes,
  Schema-Inspektion, Abgleich zwischen Soll- und Ist-Schema sowie SQL-DDL-
  Planung und -Anwendung sind im jeweils dokumentierten Backend-Umfang
  implementiert.
- ✅ Natives SQL wird anhand bekannter Schemata auf Tabellen, Spalten,
  Parameter, Nullability und Ergebnis-Mapping geprüft. Parameter werden sicher
  gebunden; Transaktionen stehen zur Verfügung.
- 🧪 Typisierte Text-/HTML-Anwendungsnachrichten können transaktional in eine
  MariaDB-Outbox eingereiht und über SMTP mit Authentifizierung, STARTTLS,
  implizitem TLS oder ausdrücklich konfigurierten privaten CA-Wurzeln zugestellt
  werden. Ein fünfsekündiges Transport-Timeout, Nachrichtengrenzen und eine
  Loopback-Testsenke sind geprüft. Die Zustellung erfolgt mindestens einmal;
  genau-einmalige Zustellung, PostgreSQL-Outbox, benannte Vorlagen und
  Dateianhänge sind in 0.9.9 nicht zugesagt.
- ✅ SQLite bietet geprüfte lokale Schemaabläufe. PostgreSQL unterstützt
  ausgewählte Laufzeit-, Schema- und Sicherheitsprüfungen. MySQL Server 8.4.11
  unterstützt typisierte parametrisierte Abfragen sowie einen isolierten
  Schemaablauf mit Setup, Inspektion, schreibgeschützter Projektkarte,
  geprüftem Spaltenplan, ausdrücklicher Anwendung und Historie. Keines dieser Ergebnisse
  belegt vollständige Backend-Parität; siehe die genaue
  [Kompatibilitätsmatrix](database-support.de.md).
- 🧪 Der Schema-Planer stuft ausgewählte Änderungen als sicher,
  prüfungsbedürftig, destruktiv oder nicht unterstützt ein. Destruktive und
  nicht unterstützte Änderungen werden fail-closed behandelt; ausgewählte
  Änderungen an befüllten Tabellen und Nullability erhalten schreibgeschützte
  Vorprüfungen. Prüfungsbedürftige Änderungen benötigen eine ausdrückliche
  Freigabe.

Die genauen Backend-Grenzen und getesteten MariaDB-Versionen stehen in der
[Datenbank-Kompatibilitätsmatrix](database-compatibility.de.md).

## Webanwendungen, Views, Formulare und CRUD

- ✅ Integrierter HTTP-Server, Seiten, Routenparameter, typisierte
  View-Interpolationen, sicheres HTML-Escaping und projektlokale
  Design-Token-Overrides. `zelyra routes <entry.zyl>` erstellt eine sortierte
  Übersicht erkannter Seiten-, API-, Formular-, Tableview-, CRUD- und
  Authentifizierungsrouten mit HTTP-Methode und Quellfundstelle. Importierte
  und generierte Ressourcenrouten sind eingeschlossen; überlappende Pfade
  derselben HTTP-Methode führen zu `E-ROUTE-001`. JSON-Ausgabe ist verfügbar.
- ✅ Schemaabhängige Formulare mit Validierung, parametrierten Datenbankaktionen,
  Transaktionen, Weiterleitungen und geprüften Beziehungs-Auswahlelementen für
  unterstützte Fälle.
- ✅ Erzeugte CRUD-Abläufe für Liste, Detail, Anlegen, Bearbeiten und Löschen
  mit Suche, typisierten Filtern, Sortierung, Pagination, Berechtigungs- und
  CSRF-Schutz.
- 🧪 Wiederverwendbare benannte Views/Layouts, typisierte Component-Properties,
  Default- und benannte Slots mit Fallback sowie CRUD-Darstellungsoptionen
  funktionieren in den dokumentierten Kompositionspfaden. Generierte
  CRUD-Inhalte werden in den Default-Slot eingesetzt; unterstützte eigene
  Inhalte benannter Slots sind compile-geprüftes statisches Markup bzw.
  Components. Projektlokale CSS-Token-Overrides funktionieren; ein vollständiger
  Theme-Editor ist dies nicht.
- ✅ Eingebaute deutsche und englische Kataloge decken die erzeugte Oberfläche
  ab. Projektlokale Sprachdateien können unterstützte Katalogtexte ergänzen
  oder überschreiben. `ZELYRA_LANGUAGE=de|en` wählt die Sprache;
  `ZELYRA_LEVEL=learn|work` schaltet die kontextbezogene Lernhilfe.
- ✅ Erzeugte MariaDB-Starter umfassen Minimal-, CRUD-, Authentifizierungs- und
  Businessvarianten; die Maschinenverwaltung enthält optionale fiktionale
  Beispieldaten.
- 🧪 Mandantengefiltertes CRUD prüft Anmeldung, Mitgliedschaft, Berechtigung
  und generierte Datenbankzugriffe im dokumentierten MariaDB-Umfang. Das ist
  keine allgemeine Multi-Tenancy für beliebige SQL-, Export-, Migrations- oder
  Hintergrundjob-Abläufe.

Details stehen im [Schnelleinstieg für Web und Datenbank](getting-started.de.md),
der [Setup-Anleitung](setup-web.de.md) und den
[Webkapiteln des Handbuchs](handbook/de/handbuch.md).

## Zelyra Studio und Rechnungstutorial

- 🧪 `zelyra editor [directory] [--port <port>] [--no-open]` startet ein lokales Studio
  auf `127.0.0.1` mit Dateibaum, Tabs, Suche/Ersetzen, Vervollständigung,
  Compilerdiagnosen, Formatierung und gespeicherter heller/dunkler Darstellung.
  Tastaturbedienung, Kontrast und Layout bei 360 CSS-Pixeln werden im Browser
  geprüft. Standardmäßig öffnet die CLI den
  Systembrowser; `--no-open` überspringt dies. Schreibzugriffe benötigen
  Sitzungstoken und denselben Browser-Origin; Datenbank/Webanwendung starten nicht mit.
  Der Editor bietet Dateisuche, lokale Vervollständigung, Formatierung,
  Compilerdiagnosen, Kontexthilfe, Aktionssuche, Navigation zu Definitionen,
  lexikalische Fundstellen und einen ausdrücklichen, begrenzten Run-Befehl.
  Eine deterministische Projektkarte zeigt auflösbare Beziehungen und benennt
  Analysegrenzen. Die Datenbankwerkbank vergleicht Schema-Metadaten, zeigt
  Fremdschlüssel, lädt Datensätze nur auf ausdrückliche Anfrage (höchstens 25
  pro Seite) und kann einfache Spaltenänderungen als geprüfte Quelltextvorschau
  übernehmen. Sie wendet keine Migration an. MySQL-Live-Introspektion,
  vollständige semantische Referenzen und ein allgemeiner visueller
  Datenbankdesigner sind nicht enthalten.
  Einzelheiten: [Studio-Anleitung](studio.de.md).
- ✅ `zelyra text edit <plan.zedit>` bietet vorschau-basierte Textänderungen aus
  TOML-Plänen mit linearen regulären Ausdrücken, erwarteter Trefferzahl,
  quellgebundener Plan-ID und ausdrücklichem Anwenden. Projektgrenzen und
  unveränderte Eingaben werden vor der Änderung geprüft; Details und Grenzen
  stehen in [Text edit](text-edit.de.md).
- 🧪 Optionales lokales TOTP für MariaDB unterstützt verschlüsselte Faktoren,
  Recovery-Codes, Replay-Schutz, Sperren und Audit. Es ist ein experimenteller
  Vorschaupfad ohne unabhängige Produktionsfreigabe.
- Das Sprachkompatibilitätskorpus 0.1 umfasst in 0.9 zusätzliche typisierte
  API-Fehler sowie Laufzeitfälle der Rechnungsanwendung und von `Result`.
- 🧪 `zelyra --tutorial invoice` erzeugt jetzt die Anwendung und startet ihre
  lokale MariaDB-/Web-Compose-Umgebung einschließlich Schema. `--no-start`
  erstellt nur das Projektgerüst. Das Tutorial führt durch Kunden, Artikel,
  Dashboard und Rechnungserstellung. Die Rechnungsansicht addiert
  Positionspreise netto, berechnet im Lernbeispiel 19% Umsatzsteuer und zeigt
  den Bruttobetrag. Unter `/invoices/RECHNUNGSNUMMER/print` kann der Browser
  die druckoptimierte Rechnung als PDF speichern. Es ist ein Lernpfad, keine
  Produktions- oder Deploymentfreigabe; Details und Grenzen stehen im
  [Rechnungstutorial](tutorials/invoice.de.md).
- 🧪 `zelyra --help` schließt mit einem deutschen und englischen Schnelleinstieg:
  minimales Projekt oder Rechnungstutorial, gefolgt von Befehlen mit kurzer
  Erklärung ihres Zwecks. `zelyra editor` ist als direkter Start im aktuellen
  Projektordner ausgewiesen.
- Studio ist noch keine vollständige IDE oder beliebiger Datenbankdesigner.
- ✅ `examples/inventory_orders/` enthält ein mehrteiliges Konsolenprojekt für
  Lager, Kunden und Bestellungen mit JSON-Speicherung, Cent-genauer
  Rabatt-/Umsatzsteuerberechnung, Stornierung, Aktionsprotokoll und ausführbaren
  Geschäftsprüfungen.

## Laravel-Migrator

- 🧪 `zelyra migrate laravel <projekt> --report <pfad>` inventarisiert
  erkannte Laravel-Muster ausschließlich statisch. PHP, Artisan,
  Datenbanken und `.env`-Dateien werden nicht ausgeführt, verbunden oder
  gelesen. Der neue Bericht liegt außerhalb des Quellprojekts und enthält
  Fundstellen, Status und nächste Prüfschritte als Markdown oder JSON. Der
  Scanner deckt Composer-Vorgaben, einfache Routen, Models, Controller,
  `Schema::create`, ausgewählte Beziehungen, Validierungseinstiege, Jobs,
  Blade-Views und Integrationsaufrufe ab. Analyzer-Version 0.9.8 und Regelwerk
  0.9.6 sind getrennt; Fixture-Prüfungen benennen erkannte Syntax, behaupten
  jedoch weder einen Routen-Daten-Graphen noch Funktionsübertragung. Der
  experimentelle Muster-Scanner ist kein PHP-Parser und kein Nachweis für
  Geschäftslogik- oder Sicherheitsparität. Das getrennte, compilergeprüfte
  Gerüst enthält keine kopierte Geschäftslogik. [Details zum Migrator](laravel-migrator.de.md).

## APIs, Authentifizierung, Berechtigungen und Audit

- ✅ Typisierte API-Deklarationen prüfen unterstützte Eingaben, Ausgaben und
  Fehler. JSON-Serialisierung, OpenAPI-3.0.3-Erzeugung und TypeScript-
  Clientgenerierung stehen über `zelyra doc` zur Verfügung.
- ✅ MariaDB-basierte Passwortauthentifizierung verwendet Argon2 und persistente
  HttpOnly-Sessions. Direkte und rollenbasierte Berechtigungen, geschützte
  Aktionen, CSRF-geschützte Browserverwaltung und Schutz der letzten
  Administratorzuordnung sind implementiert.
- 🧪 Passwortwiederherstellung verwendet in der 0.9.9-Entwicklung ein
  transaktional gespeichertes, instanzübergreifendes Reset-Limit. Die Tabelle
  speichert einen HMAC-Fingerabdruck statt einer IP-Adresse; Datenbankfehler
  sperren den Vorgang. Unabhängige Sicherheitsprüfung und vollständige
  Release-E2E bleiben offen.
- ✅ Optionales MariaDB-Audit-Logging unterstützt Inspektion, begrenzten
  JSON-/CSV-Export, strukturelle Prüfung und ausdrücklich bestätigtes Bereinigen.
- 🧪 Manipulationsnachweis durch kryptografische Audit-Verkettung ist optional
  verfügbar und durch Hash-Integritätstests abgedeckt; dadurch wird die
  Datenbank weder unveränderlich noch ersetzt dies unabhängige Audit-Kontrollen.

## Setup, Konfiguration und Distribution

- ✅ `zelyra new` und `zelyra init` erzeugen Projekte und Templates. Erzeugte
  MariaDB-Projekte enthalten Compose-Konfiguration und eine lokale `.env`;
  Zugangsdaten werden nicht ausgegeben, neu erzeugte Unix-`.env`-Dateien
  erhalten Modus `0600`, und `.env` wird aus erzeugten Git-/Docker-Kontexten
  ausgeschlossen.
- ✅ `zelyra setup` bietet Konsolenaktionen und einen nur an Loopback gebundenen
  Browser-Assistenten, startet erzeugte lokale MariaDB-/Web-Stacks, wendet das
  Anfangsschema an, unterstützt Portwahl und gibt die App-Adresse aus.
- ✅ `zelyra doctor --json` liefert schreibgeschützte Bereitschaftsprüfungen
  mit stabilen Kategorien für Konfiguration, Projekt, Verbindung, Authentifizierung,
  Timeout, Schema und Werkzeuge; Datenbankdiagnosen werden ohne Zugangsdaten
  normalisiert. `zelyra update` prüft
  Release-Prüfsummen vor dem Austausch unterstützter Linux-/Windows-x86_64-
  Binärdateien.
- ✅ Benutzerlokale Installation aus dem Quellcode ist verfügbar.
  Veröffentlichte Linux- und Windows-x86_64-Release-Archive enthalten
  SHA-256-Prüfsummen; wiederholte Builds waren in den festgelegten
  Release-CI-Toolchains byte-identisch.

Konfigurationsvorrang, Umgang mit Secrets und Plattformgrenzen erläutern die
[Umgebungsreferenz](env.md) und die [Setup-Anleitung](setup-web.de.md).

## KI-native Compiler-Schnittstellen

- ✅ `zelyra check --format=json` und `zelyra context --format=json` liefern
  versionierte maschinenlesbare Ausgaben für ihren dokumentierten Umfang.
- 🧪 `zelyra impact --format=json` liefert deterministische,
  quelltextbasierte Wirkungsinformationen zu unterstützten Deklarationen und
  Referenzen.
- ✅ `zelyra edit --format=json` zeigt unterstützte semantische
  Umbenennungen als Vorschau; die Anwendung ist ausdrücklich und das geänderte
  Projekt wird erneut vom Compiler geprüft.
- ✅ Typisierte Ausdruckslücken (`_`) liefern kontextbezogene Diagnosen und
  verhindern, dass unvollständige Programme gebaut oder ausgeführt werden.

Diese Werkzeuge rufen keinen KI-Anbieter auf. Siehe die
[KI-native Architektur](architecture/ai-native-development.de.md) und deren
[Benchmark-Spezifikation](benchmarks/ai-authoring.de.md); hier werden keine
vergleichenden Benchmark-Ergebnisse behauptet.

## Test- und Release-Nachweise

Das stabile [Zelyra-0.9.7-Release](https://github.com/sf1976/zelyra/releases/tag/v0.9.7)
enthält Linux- und Windows-x86_64-Artefakte, Prüfsummen und SPDX-SBOMs. Die
laufenden Prüfungen und ihre Grenzen stehen in der
[Datenbankmatrix](database-support.de.md). Die menschliche Produktabnahme ist
für 1.0.0 reserviert und wird durch automatisierte Nachweise nicht ersetzt.

Diese Übersicht wird angepasst, wenn sich ausgelieferte Funktionen oder ihre
Nachweise ändern. Ein Spezifikations- oder Roadmap-Eintrag allein wird hier
nicht als Implementierung ausgegeben.

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
