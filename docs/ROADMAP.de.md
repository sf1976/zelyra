Warning: truncated output (original token count: 15710)
Total output lines: 958

# Zelyra Roadmap

Dies ist die dauerhaft gepflegte Roadmap für Zelyra. Sie enthält notwendige
Arbeiten ebenso wie optionale Ideen. Ein Punkt gilt erst dann als umgesetzt,
wenn — soweit anwendbar — Syntax oder API-Dokumentation, Implementierung,
positive und negative Tests, Diagnostik und ein funktionierendes Beispiel
vorhanden sind.

Statuslegende:

- ✅ umgesetzt (`[x]`)
- 🧪 teilweise umgesetzt oder experimentell (`[~]`)
- 🗺️ geplant (`[ ]`)
- ◻️ optional oder noch in Prüfung (`[?]`)
- ⛔ blockiert oder bewusst zurückgestellt (Grund direkt am Punkt nennen)

Die sichtbaren Emojis machen den Status auf GitHub eindeutig; die
Klammermarker bleiben vorerst maschinell durchsuchbar.

Digitale Souveränität ist eine querschnittliche Produktanforderung, kein
bereits vollständig umgesetztes Feature: lokale Kontrolle, keine verpflichtende
Cloud oder KI, keine ungefragte Telemetrie, explizite Effekte, portable Daten,
Ressourcendisziplin und ehrliche Beweisaussagen leiten jede Phase. Das
[Manifest](MANIFESTO.de.md) beschreibt die Prinzipien; die Roadmap kennzeichnet
den tatsächlich implementierten Stand.

Das aktuelle stabile Release ist 1.1.2. **Aktuelle Meilensteine** ist die
verbindliche Übersicht der nächsten Vorhaben. Ältere, nummerierte Release-
Abschnitte bleiben als historische Planung und Nachweise erhalten; sie legen
für sich keine aktuellen Releaseziele fest. Langfristige Ideen stehen im
Abschnitt zur Weiterentwicklung nach 1.0.0.

## Releasefolge bis 1.3.0

Diese Folge ist der verbindliche Plan für aktive Arbeiten. Jedes aktive offene
Ziel ist einem Release zugeordnet; historische Backlog-Einträge bleiben als
Kontext erhalten und sind keine zusätzlichen, unversionierten Zusagen. Ein
Release gilt erst nach Implementierung, Dokumentation, positiven und negativen
Tests sowie den jeweiligen Sicherheits- und Kompatibilitätsprüfungen als
abgeschlossen.

- [ ] **1.1.5 — wiederverwendbare Zelyra-Module und Pakete:** Offline-
  Paketabläufe, typisierte öffentliche APIs und Effekte, deterministische
  Lockfiles, wiederverwendbare Geschäftsmodule, sichere Einstellungen und
  dokumentierte Deployment-Nachweise abschließen. Offene Paketvertrauens-,
  Erweiterungs-, Modulkonfigurations- und Integrationslücken schließen. CSV
  ist das erste Austauschmodul; ODBC, Excel-kompatibler Export, PDF-Abläufe und
  weitere Backends werden nur aufgenommen, wenn ihre begrenzten Integrationen
  jeweils getestet sind. Keine Laravel-spezifische Umsetzung.
- [ ] **1.2.0 — Betriebs- und Deployment-Grundlage:** Deklarative Commands,
  Scheduler und Jobabläufe samt Verwaltungsansichten schrittweise ergänzen;
  typisierte Request-/Form- und begrenzte Query-Grundlagen sowie den statischen
  Ordner `public/` bereitstellen. Produktionskonfiguration für `serve` und
  geprüfte Apache-/Nginx-Reverse-Proxy-Beispiele dokumentieren. Apache/Nginx
  leiten HTTP an den Zelyra-Prozess weiter und führen keine `.zyl`-Dateien aus.
  `main.zyl` bleibt Programmeinstieg; dessen Routen bedienen `/`. Mit
  `zelyra service install --port <port>` einen festen Port und einen
  Betriebssystemdienst installieren (zuerst Linux systemd). Ein belegter Port
  ist ein Fehler; der Befehl darf nicht still einen anderen Port wählen.
  Projekt, Programmdatei, Arbeitsverzeichnis und sichere Env-Referenzen
  speichern, ohne Geheimniswerte zu kopieren.
- [ ] **1.2.5 — Integration der Geschäftsabläufe:** Mandantenbezogene
  Abfragen, Forms und Berechtigungen, begrenzte dauerhafte Jobs und koordinierte
  Zeitpläne, Moduleinstellungen auf normalen Anwendungsseiten sowie erste
  wiederverwendbare Datenaustausch-Integrationen fertigstellen und prüfen.
  Signierte Quellpakete, Herausgebervertrauen, exakten Versionsabruf aus einer
  statisch hostbaren Registry und Offline-Locks abschließen; siehe den
  [Paketökosystem-Plan](release-plans/1.2.5-module-ecosystem.de.md).
  `zelyra service change --port <port>` prüft den Zielport, stoppt den alten
  Dienst, aktualisiert seine Konfiguration und startet ihn neu. Bei Fehlern
  stellt es Konfiguration und Dienst wieder her. `status`, `restart` und
  `remove` ergänzen, sofern der Plattformdienstmanager dies unterstützt.
  Weitere Betriebssysteme erst nach Lifecycle- und Upgrade-Tests unterstützen.
- [ ] **1.2.8 — Integrität und Distribution:** Geprüfte vertikale Schnitte für
  Zustandsübergänge, Audit/Historie, ausgeglichene Ledger-Buchungen,
  Metadatenexport, Healthchecks und modulare Bundles gemäß Geschäftsplattform-
  Detailplan liefern. `public/`-Pfadbegrenzung und sicheres Verhalten für
  statische Dateien ergänzen sowie Apache-/Nginx-Weiterleitungsheader und
  erlaubte Hosts prüfen. Dies ist ein Integrationsrelease, keine pauschale
  Compliance- oder Performancezusage.
- [ ] **1.3.0 — kanonische Struktur und Bytecode-Standard:** Das optionale,
  versionierte Projektlayout (`routes/`, `domain/`, `schema/`, `public/`,
  `assets/`, `plugins/`), Architekturprüfungen und eine sichere
  Migrationsvorschau abschließen. Geprüfter Bytecode wird erst dann Standard
  für `run`, `serve` und deklarierte Tasks, wenn Sprach-, SQL-, Effekt-,
  Capability-, Contract-, Ressourcenlimit-, Diagnose- und Hostoperations-
  Parität zwischen AST/MIR/Bytecode nachgewiesen ist. AST bleibt Referenz und
  Differentialtest-Backend. Bis zum bestandenen Nachweis müssen nicht
  unterstützte Bytecode-Operationen fail-closed scheitern und der bisherige
  Standard unverändert bleiben. Ziel ist portabler geprüfter Bytecode, keine
  native Maschinencodegenerierung.

**Nach 1.3.0 oder bei ausdrücklicher Neubewertung:** Ein natives AOT-Backend
bleibt optional und hängt von Benchmarks ab. Die Laravel-Website-Migration ist
zurückgestellt. Kein offener Punkt entfällt stillschweigend: Arbeit, deren
Freigabekriterien nicht erfüllt sind, bleibt offen und blockiert das betreffende
Release, statt als fertig dargestellt zu werden.

## Zurückgestellte Idee — Laravel-Website-Migration

Auf Wunsch des Projekteigentümers vom 17.10.2026 wurde das Laravel-
Migrationstool aus dem aktiven Zelyra-Projekt entfernt. Quellcode, Anleitungen,
Tests und bisherige Analyseberichte sind außerhalb des Repositorys archiviert:
a private archive outside the repository. Derzeit wird
kein Laravel-Migrationsbefehl ausgeliefert. Eine Neubewertung erfolgt erst nach
den aktiven Architektur-Meilensteinen oder bei ausdrücklicher Priorisierung.

## Aktive Architektur-Meilensteine (1.1.5 bis 1.3.0)

Diese Meilensteine sind in Arbeit und noch nicht veröffentlicht. Bereits teilweise umgesetzte Punkte sind mit `[~]` markiert; offene Teilziele bleiben ausdrücklich stehen. Einträge veröffentlichter Versionen bleiben historische Nachweise.

### 🗺️ [ ] Zelyra 1.1.5 — backendunabhängiges Modul- und Paketsystem

- [~] **Lokaler Paketimport:** Bestehendes `import "pfad.zyl" as alias` bleibt kompatibel; der experimentelle Import `import "pkg:name" as alias` löst den lokalen Paketeinstieg über den Abhängigkeitsschlüssel auf, der von der Paketidentität abweichen darf. Vorhandene `pub`-, Typ-, Zyklus- und Capability-Prüfungen bleiben aktiv. Paketeinstiege und interne Quellimporte werden innerhalb des Paketverzeichnisses aufgelöst und dürfen `.git/` sowie `target/` nicht betreten, da diese Verzeichnisse bei Prüfsummen bewusst ausgelassen werden. Ein generiertes Bundle meldet den Quellabschluss erst bei leerer Liste ungelöster Verweise und erfolgreicher Compilerprüfung als vollständig; der Deploymentstatus bleibt davon unabhängig.
- [~] **Lokales Manifest und Lockfile:** `[package]`, pfadbasierte `[dependencies.<name>]`, SemVer-Vorgaben, deterministische Graphauflösung und `zelyra.lock` mit sortierten Paketdaten sowie SHA-256 über Manifeste und `.zyl`-Quellen sind implementiert. Die optionale Paketvorgabe `zelyra = "<SemVer-Vorgabe>"` wird vor der Paketverwendung gegen die laufende CLI-Version geprüft und in `zelyra module list` angezeigt; Änderungen machen Manifestprüfsumme und Lockfile ungültig. Fehlerhafte und inkompatible Vorgaben haben Negativtests. Geschwisterpakete dürfen `..` verwenden, wenn der normalisierte Pfad im Workspace bleibt; Symlink-Traversierung wird abgelehnt. `zelyra package lock [entry] [--check|--update]` arbeitet offline; Compilerbefehle lehnen fehlende/veraltete Lockfiles bei Abhängigkeiten ab. Ein Integrationstest ändert eine transitive Paketquelle und bestätigt, dass sowohl `check` als auch `run` bis zur ausdrücklichen Lock-Aktualisierung mit `E-PKG-002` abbrechen. Ein Negativtest schützt außerdem davor, Prüfsummen durch Importe aus `.git/` oder `target/` zu umgehen. `zelyra module plan` akzeptiert `pkg:<Abhängigkeitsschlüssel>`; Bundles übernehmen nur die verwendeten Paketabhängigkeiten samt transitiver Pakete, erhalten deren Prüfsummenquellen und schreiben ein passendes gefiltertes Manifest/Lockfile. Mehrere lokal aufgelöste Versionen desselben Paketnamens funktionieren über unterschiedliche Abhängigkeitsschlüssel und Pfade; Lock-Einträge enthalten Pfad, Name, Version und Prüfsumme, während doppelte Name-Version-Identitäten an verschiedenen Pfaden abgelehnt werden. Die Vertrauensgrenze lokaler Pakete ist dokumentiert: Lock-Prüfsummen erkennen Abweichungen von einem bereits vertrauten Lockfile, authentifizieren aber keine Herausgeber. Registry, Signaturen und Herausgebervertrauen bleiben offen.
- [~] **Grundlage signierter Quellartefakte und statischer Registry — unveröffentlicht:** `zelyra package keygen`, `sign`, `verify` und `trust add|list|remove` verwenden Ed25519 und eine ausdrückliche Projekt-Vertrauensdatei. Versionierte Quellartefakte binden Herausgeber, exakte Identität/Version, Kompatibilitätsmanifest, normalisierten Hash und compilergeprüfte Quellen; begrenzt auf 32 MiB und 4.096 Dateien. Paketlokale Abhängigkeiten sind unterstützt, wenn ihr gesamter Quellabschluss unter dem Paketverzeichnis enthalten ist; entfernte transitive Auflösung bleibt offen. `zelyra package registry build <ordner>` erzeugt einen deterministischen statischen Index. `zelyra package add <index-url> <name> <exact-version> --as <alias>` lädt eine exakte vertrauenswürdige Version über HTTPS (Loopback-HTTP für Tests), prüft Index und Signatur, installiert ohne Überschreiben, speichert Registry/Herausgeber/Schlüssel/Hash im `zelyra.lock`, prüft das vorläufige Projekt mit dem Compiler und rollt fehlgeschlagene Manifest-/Lock-/Projektänderungen zurück. Bundles verschieben Pakete aus dem Cache in portable Pfade `packages/<name>/<version>` und behalten die Herkunft. Loopback-Integrationstests prüfen exakte Installation/Updates, Rollback beim Entfernen und Offline-Kompilierung. Entfernte transitive Auflösung und sichere Cache-Bereinigung bleiben offen; exakte Updates eines Alias und das Entfernen aktualisieren Manifest/Lock mit Rollback und erhalten Cachequellen; das Paketökosystem ist nicht abgeschlossen.
- [x] **Offline-Paketgerüst:** `zelyra module init <verzeichnis> --name <paketname> [--version <semver>]` erstellt ein kleines, compilerprüfbares lokales Paket, bindet seine CLI-Kompatibilität an die aktuelle Minor-Linie, startet ohne deklarierte Effekte und überschreibt keine vorhandenen Ziele. Der Integrationstest sperrt, prüft und startet das erzeugte Paket; ungültige Namen, doppelte Versionsoptionen und Überschreibungsversuche werden abgewiesen.
- [~] Getypte öffentliche Funktionen/Records/Enums/Generics, Exporte, Abhängigkeitsgraphen, Zyklus-/Sichtbarkeitsdiagnosen und Capability-Weitergabe sind implementiert und getestet. Optionale `[package].effects`-Obergrenzen weisen Funktionseffekte außerhalb der Paketfreigabe zurück; sie erteilen keine Laufzeit-Capabilities der Anwendung.
- [~] **Wiederverwendbare Anwendungsmodule und Webkonfiguration:** `zelyra module list` zeigt jetzt den geladenen Modulgraphen, öffentliche Exporte, lokale Paketidentität/-version/Prüfsumme, Abhängigkeitsaliasnamen und deklarierte Effektobergrenzen deterministisch in menschen- oder maschinenlesbarer Ausgabe. Der Befehl erteilt keine Laufzeit-Capabilities und gibt keine Geheimnisse aus. Login/Authentifizierung, Import, Export, Berichte und ähnliche Geschäftsabläufe sollen versionierte Zelyra-Module sein, die projektübergreifend wiederverwendbar sind – keine Laravel-spezifischen Kopien. Pakete stellen typisierte Schema-Metadaten nun über `zelyra module list` bereit; Pflicht-, Standardwert- und Typregeln werden geprüft, Standardwerte für Geheimnisse abgelehnt und Einstellungswerte niemals ausgegeben. Nicht geheime Werte und Env-Referenzen für Geheimnisse lassen sich in `zelyra.settings.toml` persistieren; Env-Geheimnisse werden gemäß dem deklarierten Skalartyp konvertiert und validiert, ohne Variablennamen oder Werte auszugeben. Ungültige, leere oder übergroße Werte werden vor Ausführung abgewiesen; Zwischenwerte werden genullt. `zelyra module settings check` prüft Typen, Pflichtfelder, Defaults und Secret-Verfügbarkeit ohne Werte auszugeben. JSON-Ausgaben weisen die Pflichtfeld-Eigenschaft explizit aus. `zelyra run` und `zelyra serve` blockieren Starts mit fehlenden oder ungültigen Pflichtwerten, ohne Geheimnisnamen auszugeben. Paketcode kann eigene deklarierte Werte jetzt über `module_setting<T>("name")` lesen: der Literalname und exakte Schematyp werden statisch geprüft, und die Laufzeit trennt Werte nach Quelldatei/Paket und redigiert sie in Debug-Ausgaben. Der Zugriff ist für AST, MIR und Bytecode integriert; das ist kein Nachweis vollständiger Parität der experimentellen Backends. Zelyra Studio bietet jetzt eine geschützte lokale Oberfläche zum Bearbeiten typisierter Paketwerte mit Revisionsabgleich, atomarem Schreiben und Rollback bei ungültiger Konfiguration; bestehende Geheimniswerte und Env-Namen werden nicht ausgelesen oder angezeigt. Offen bleiben sichere Secret-Verwaltung außerhalb bestehender Prozessvariablen sowie capabilitybewusste Einstellungsformulare auf normalen Zelyra-Anwendungsseiten. Künftige Oberflächen müssen Werte vor Anwendung prüfen und dürfen Capabilities niemals implizit freigeben. Lokale Paketwiederverwendung und CLI-Inventar allein erfüllen dieses Ziel nicht.
- [~] **Module für Geschäftsdaten-Austausch:** Wiederverwendbare lokale Pakete `csv_export` und das reine `csv_import` liegen in `examples/business_modules`. Der Export kodiert deterministisch UTF-8 und schreibt Dateien mit ausdrücklich deklariertem `FileSystem`-Effekt und Root-Prüfung; der Import dekodiert begrenzte Eingaben, verlangt nicht leere Kopfspalten und weist Zeilen mit abweichender Spaltenzahl zurück. Ein CLI-Integrationstest prüft gültige und fehlerhafte Daten mit AST, MIR und Bytecode. CSV-Dateien sind auf 16 MiB begrenzt und werden im Speicher verarbeitet. ODBC-Import mit Vorschau/Feldzuordnung/transaktionalem Datenbank-Import, PDF-Druck, Excel-kompatibler Export und breitere Backendkompatibilität bleiben offen; das Beispiel schließt das Geschäftsdaten-Austauschziel nicht ab.
- [~] **Nachweis für modularen Docker-Betrieb:** `examples/customer_orders_modules` wurde geprüft und als eigenständige Docker-Quellanwendung gebündelt. Der generierte Docker-E2E-Test führte außerdem den mehrmoduligen Kunden-/Auftragsablauf aus und exportierte und startete Kunden- und Inventarmodul getrennt gegen einen isolierten MariaDB-Testdienst. Das belegt nur die ausgeführten Pfade; das Bundle hält `complete_deployment: false`, und andere Backends sowie Anwendungskonfigurationen sind nicht geprüft. Ein paketbasiertes Beispiel verbindet Root-Anwendung, Billing-Paket und separates Storage-/Datenbankpaket. Die Laufzeit verwendet jetzt `sql_decode`, bevor SQL-Zeilen als Array behandelt werden. Ein optionaler Integrationstest mit isoliertem MariaDB legt eine eindeutig benannte Datenbank an, führt den Aufruf Root → Billing → Storage aus und räumt sie wieder auf. Der Test besteht. Auch ein Docker-Image, das aus dem aktuellen lokalen Quellstand gebaut wurde, führt diesen Aufruf gegen die entbehrliche MariaDB aus und gibt `0` aus. Der Dockerfile führt jetzt während des Image-Builds `zelyra check main.zyl` mit dem gepinnten Compiler aus. Ein negativer Docker-Build mit dem älteren Pin `e33e101` bricht dort mit `E-MOD-001` für `pkg:storage` ab; ein inkompatibler Pin kann somit kein Image erzeugen. Ein veröffentlichter paketfähiger Compiler-Pin muss noch validiert werden. Das Bundle meldet weiter `complete_deployment: false`; HTTP-Verhalten einzelner Ressourcen und Kompatibilität eines veröffentlichten Images bleiben offen. Am 2026-10-17 mit `tests/generated-project-docker-e2e.sh` erneut geprüft: Die lokale CLI wurde aus diesem Arbeitsstand gebaut; für den Compose-Test wurden die Ports 28081–28084 und 23309 verwendet und der Modulcompiler auf den vollständigen Commit `84c1d4e2a0f9f7b968d91dfd51a121a2cf2ad1c1` gepinnt. Der Lauf bestand mit Setup, Wiederherstellung, CRUD und mehrmoduligem Geschäftsablauf der generierten Anwendung sowie separatem Build/Start, HTTP- und Berechtigungstests für Kunden- und Inventar-Export. Entfernt wurden nur testeigene Container, Netzwerke, Volumes und temporäre Dateien; andere laufende Container blieben unangetastet. Das belegt diese Quellmodul- und MariaDB-Pfade, aber keine allgemeine Paket-/Backend- oder Deployment-Parität.
- [~] Die versionierte Rust-Hostservice-API v7 ist für Uhr, Umgebung, Zufall, Konsole, HTTP, Mail, Prozesse, Dateisystem und Datenbank an den AST-Interpreter angebunden ([aktueller Vertrag](runtime-host-services.de.md)). MIR und Bytecode nutzen denselben Provider in den jeweils implementierten Teilmengen. Bytecode v22 unterstützt Inline-Lambdas, gespeicherte Capturing-Lambdas mit typisiertem Capture-Layout sowie statische und dynamische benannte Funktionswerte für `map`/`filter`/`find`/`reduce` und direkte Aufrufe typisierter benannter Funktionswerte in lokalen Variablen. Direkte Aufrufe typisierter benannter Funktionswerte aus lokalen Variablen prüfen Signaturen und Capabilities. Typgeprüfte Record-Copy-updates und Enum-Payloads sowie MIR-abgesenkte Match-Anweisungen/-Ausdrücke sowie `now()`/Clock, `env(name)`/Environment und `read_console(prompt)`/Console sowie `random_int`/`uuid_v4`/Random sowie `read_text`/`read_text_result`, `file_exists`/`dir_exists`, `file_size`, `list_dir` und Root-geprüftes `write_text`/`write_text_result`/`write_text_new_result`/`atomic_write_text_result`/`append_text_result`/`delete_file`/`delete_file_result`/`copy_file_new_result`/`rename_file_result`/FileSystem; Artefakt-Capabilities werden geprüft und die Ausführung benötigt explizite Projektfreigaben. `execute` erteilt standardmäßig keine Freigaben. Bytecode unterstützt `run_process`, `http_get`, `http_request`, `http_json` und `http_result` zusätzlich zu den aufgeführten Clock-, Environment-, Console-, Random- und FileSystem-Aufrufen; `mail_deliver` verwendet dort ebenfalls den injizierten Host nach ausdrücklicher `Email`-Freigabe und gemeinsamer SMTP-/Netzwerk-Policyprüfung; Tests nutzen einen Fake-Transport. Bytecode v21 führt geprüfte SQL- und `mail_queue`-Operationen mit optionalen PDF-Anhängen in einem injizierten Transaktions-Batch aus; `Email` und `Database(write)` werden vor der Auswertung der Nachrichtenfelder geprüft. Andere Transaktionseffekte bleiben nicht unterstützt. MIR deckt mehr davon, aber ebenfalls nur teilweise ab. SQL-, Netzwerk- und Mailprüfungen erfolgen vor dem Provideraufruf. Diese API ist keine stabile native Plugin-ABI.
- [ ] Zuerst reine Zelyra-Pakete unterstützen. Rust-basierte Host-Integrationen kontrolliert ermöglichen; die instabile Rust-ABI nicht als öffentliche Pluginschnittstelle verwenden. Dynamisches Laden gegebenenfalls später über versionierte Host-API, stabile C-ABI oder Komponentenmodell prüfen.
- [ ] Statische SQL-Prüfung, Verträge, explizite Effekte, Sandboxing und menschliche Freigabe destruktiver Aktionen bewahren. Native Einstiegspunkte dürfen Sicherheitsregeln nicht umgehen.
- [ ] Erweiterbarkeit mit einem Referenz-Datenbankadapter und dem Entwurf einer ausschließlich lesenden Banking-Anbindung (FinTS/HBCI und EBICS) demonstrieren: explizite Autorisierung, Geheimnisschutz, Audit und Mock-Tests. Keine pauschale Bankkompatibilität ohne institutsspezifische Tests behaupten.
- [ ] Paketvertrauen, Veröffentlichung, Supply-Chain-Sicherheit, Kompatibilität, CLI-/Studio-Anbindung, Diagnostik, positive/negative Tests und ausführbare Beispiele dokumentieren.
- [ ] Freigabekriterium: 1.1.5 erst nach bestandenen Kompatibilitäts-, Berechtigungsverweigerungs-, Abhängigkeits-, Offline-/Reproduzierbarkeits- und Integrationstests als fertig kennzeichnen.

### 🗺️ [ ] Zelyra 1.3.0 — typisierte MIR und Rust-Bytecode-VM

- [~] **Typisierte MIR und experimentelle Ausführung:** `zelyra-runtime` gibt quellpositionsgebundene inferierte Typen nur nach erfolgreicher Prüfung des gesamten Programms zurück. `zelyra-mir` erstellt und prüft explizite Basisblöcke für `if`, `while`, `loop`, `for`, `match`, `break`, `continue` und `return` und senkt Ausdrücke strukturell in typisierte MIR-Knoten mit Operanden-IDs und Quellpositionen ab. Der Verifier prüft CFG-Dominatoren, Bedingungs-/Invarianten-Typen, Signaturen, typisierte Zuweisungen und Arrayindex-/Tupelupdates (`E-MIR-VERIFY-002`). Vor der MIR-Erzeugung nutzt der Lowerer dieselbe Capabilityanalyse wie die AST-Referenz und weist ungültige Deklarationen/Aufrufe mit `E-MIR-EFFECT-001` ab. Der MIR-Verifier prüft außerdem lokale Mutabilität für Zuweisungen, Arrayindex-Updates sowie bestehende Tupel-/Parallelziele und dazu bekannte/eindeutige Capability-Deklarationen und statische Funktionsaufrufkanten (`E-MIR-EFFECT-001`); direkte Built-in-Effekte werden beim Source-Lowering geprüft. Direkte Built-in- und SQL-Effekte prüft der Verifier nun ebenfalls über die gemeinsame Capabilityzuordnung der AST-Analyse. Der MIR-Verifier prüft zusätzlich Aufrufe über typisierte lokale Funktionswerte: lokalen Funktionstyp, Arity, Argument- und Ergebnistypen sowie benötigte Capabilities; der Interpreter prüft beim Aufruf zusätzlich die Capability-Grenze des Callers. MIR führt jetzt lexikalisch erfassende Lambdas aus, die lokal gespeichert oder aus einer Funktion zurückgegeben und innerhalb desselben Programmlaufs aufgerufen werden. Bytecode v24 senkt gespeicherte Capturing-Closures mit Snapshot-Captures ab. Lambdas dürfen äußere mutable Bindungen lesen, aber nicht verändern; der Typechecker weist solche Schreibzugriffe zurück, bis eine gemeinsame Zellsemantik für veränderliche Captures implementiert ist. Vollständige Closure-/Backend-Parität und eine breitere MIR-Abdeckung bleiben offen. Der experimentelle MIR-Interpreter führt verifizierten Kontrollfluss, Zelyra-Funktionsaufrufe, Records, Felder und typgeprüfte Copy-updates, Arrays und Indizes, ausdrücklich veränderliche Arrayupdates, Tupelkonstruktion, -indizierung und -destrukturierung, Schleifen über Collections/Bereiche, Enum-Payloads, Lambda-Callbacks mit lexikalischen Captures in `map`/`filter`/`find`/`reduce` sowie gespeicherte Capturing-Lambdas, die innerhalb eines MIR-Laufs weitergereicht, zurückgegeben und aufgerufen werden, sowie Match-Ausdrücke und -Anweisungen mit Guards für einen differential geprüften Teil der skalaren Semantik aus. Er enthält reine Implementierungen für `Some`, `None`, `Ok`, `Err`, `len`, `append`, `contains`, `range`, `to_string`, Unicode-Textoperationen (`text_contains`, `text_trim`, `text_chars`, Groß-/Kleinschreibung, Präfix-/Suffixprüfung, Ersetzung und Split) sowie `parse_int`/`parse_float` mit Result-Fehlern. Zusätzlich implementiert sind `abs`, `min`, `max`, `clamp`, `sqrt`, `pow`, `floor`, `ceil`, `round` und den `decimal`-Konstruktor, die Array-Built-ins `sort`, `sort_desc`, `reverse`, `slice`, `concat` und `join` sowie die Set-Built-ins `set`, `set_add` und `set_remove`. Schritt- und Aufruftiefenlimits gelten auf einem begrenzten Runtime-Threadstack; Tests vergleichen unterstützte Ausführungen mit dem AST-Interpreter. Uhr, Umgebung, Zufall, Konsole, Prozesse, Dateisystem, HTTP, einfache SQL-Abfragen und `mail_deliver` verwenden injizierte HostServices. Capability-Deklarationen, optionale Projektfreigaben sowie Prozess-Allowlist, Dateisystemroots, Netzwerk- und Datenbankpolicy werden vor Providerzugriff geprüft. HTTP-Policy und Antwortvalidierung, SQL-Parameterkonvertierung sowie Datenbankschema-/Speicherprüfung verwenden gemeinsame AST-Runtime-Helfer. Die SQL-Teilmenge erfasst Parameter-LocalIds bei der HIR-Auflösung, bindet Werte über den gemeinsamen Placeholder-Scanner, prüft Schema/Speicher einmal je Ausführung und leitet Abfragen an den Host weiter. SQL-Transaktionsblöcke werden in einen verifizierten MIR-Transaktionsknoten gesenkt und über genau einen injizierten Aufruf von `execute_database_batch(..., true)` ausgeführt. Der unterstützte Block enthält direkte SQL-Ausdrücke, SQL-Let-Anweisungen und eigenständige `mail_queue`-Aufrufe mit optionalen PDF-Anhängen; PDF-Bytes verwenden denselben Runtime-View-Renderer wie AST und den gemeinsamen Outbox-Validator. SQL behält HIR-aufgelöste Parameter-LocalIds, Laufzeit-Capabilityprüfungen und die gemeinsame Datenbankspeicherprüfung. Differentialtests mit Fake-Hosts vergleichen SQL-Batches, Outbox-Einträge mit und ohne PDF-Anhang, typisierte SQL-Lets, Providerfehler, fehlenden Datenbankkontext, verweigerte Freigaben und ungültige MIR-Referenzen. Parallele `await`-Bindungen laufen jetzt in gescopten MIR-Worker-Threads mit Snapshot-Eingaben, Bindung erst nach Abschluss aller Zweige, gemeinsamem Schrittbudget und denselben Capability-/Policy-Prüfungen am Host. Andere Transaktionsinhalte bleiben fail-closed. Ein mehrstufiges lokales `pkg:`-Modul wird über den Projektlinker geladen, nach MIR/Bytecode gesenkt/verifiziert und in Backend- sowie CLI-Integrationstests mit `--backend bytecode` wertgleich zum AST ausgeführt; dies belegt nur den geprüften reinen Paketpfad. Der strukturelle MIR-Interpreter ist vom Bytecode getrennt. Der eigenständige versionierte Binär-Bytecode-v23-Schritt ist experimentell und ausschließlich über `zelyra run --backend bytecode` ausdrücklich auswählbar; AST bleibt Standard, nicht unterstützte Programme werden fail-closed abgewiesen; keines der beiden Backends ist das Standardruntime. MIR und Bytecode führen gespeicherte Capturing-Lambdas innerhalb desselben Programmlaufs aus. Bytecode v24 serialisiert typisierte Capture-Layouts, prüft Capture-Typen und Capability-Grenzen und weist Schreiboperationen auf Snapshot-Captures zurück; das Format v23 serialisiert außerdem lokale Mutabilität und exakte Initialisierungsstellen und weist spätere Writes auf immutable Locals zurück; vollständige Closure-/Backend-Parität bleibt offen. Vollständige Built-in-Semantik (CSV-Rundlauf sowie Festkomma-Konvertierung, -Formatierung und Half-even-Rundung sind differentiell abgedeckt), Effektinferenz, Optimierungen, breite Differentialabdeckung und die Auswahl als Standardbackend bleiben offen; Bytecode v20 unterstützt verifizierte parallele `await`-Zweige mit Snapshot-Captures und gemeinsamem Schrittbudget; eine gemeinsame Obergrenze von vier Zweigen wird im Type Checker, MIR-/Bytecode-Verifier und vor dem Workerstart in jeder Runtime geprüft; der experimentelle CLI-Schalter belegt keine Featureparität. Bytecode v19 senkt Inline-Lambdas sowie statische und dynamische benannte Funktionswerte für `map`/`filter`/`find`/`reduce` ab und unterstützt direkte Aufrufe typisierter benannter Funktionswerte aus lokalen Variablen mit Capability-Prüfung sowie typgeprüfte Record-Copy-updates ab und unterstützt Enum-Payload-Konstruktion sowie MIR-abgesenkte Match-Anweisungen/-Ausdrücke sowie injizierte HostServices für `now()`, `env(name)`, `read_console(prompt)`, `random_int`, `uuid_v4`, `read_text(path)`, `read_text_result(path)`, `file_exists(path)`, `dir_exists(path)`, `file_size(path)`, `list_dir(path)`, `write_text(path, content)` und `write_text_result(path, content)`, `delete_file(path)`, `delete_file_result(path)`, `copy_file_new_result(source, destination)` und `rename_file_result(source, destination)` mit serialisierten Deklarationen, ausdrücklichen Freigaben und konfigurierter Dateisystem-Root-Prüfung. `run_process(command, arguments)` nutzt die gemeinsame Prozess-Allowlist, das Timeout-/Ausgabelimit, den injizierten Host und die `Process`-Freigabe. Ein begrenzter deterministischer Encoder/Decoder für diese unterstützte Teilmenge ist implementiert; das Lowering liegt nun in `mir/src/bytecode/compiler.rs`, die Differential- und Verifier-Tests in `mir/src/bytecode/tests.rs`; `mir/src/bytecode.rs` enthält das gemeinsame Instruktionsmodell, den Verifier und die VM. Diese interne Aufteilung ändert weder beabsichtigt API noch Sprachsemantik; eine verifizierte Vorprüfung autorisiert Hostaufrufe vor der Argumentauswertung. Persistenter Cache bleibt offen. MIR und Bytecode prüfen Funktionsvor- und -nachbedingungen zur Laufzeit, beweisen sie aber nicht. Runtime-HostServices API v7 abstrahiert Uhr, Umgebung, Zufall, Prozess, Dateisystem, Datenbank, Konsole, HTTP-Transport und Mailzustellung. Siehe das [Arbeitsprotokoll 1.1.5–1.3.0](release-plans/1.1.5-to-1.3.0-progress.de.md).
- [~] **Audit des Ausführungsmodells:** Das AST-Referenzbackend in `runtime/src/ast_interpreter.rs` und seinen Auswertungsmodulen wertet typisierte AST-Ausdrücke und Anweisungen rekursiv aus; seine öffentlichen Ausführungs-APIs und die Interpreterinitialisierung liegen in `runtime/src/execution_api.rs`, begrenzt Schritte/Aufruftiefe, prüft Laufzeitverträge und verwendet injizierte HostServices mit Capability- und Ressourcenrichtlinien. MIR senkt geprüftes HIR in verifizierte, quellgebundene CFGs ab und führt eine getestete Teilmenge aus; Bytecode v24 verwendet iterative Aufrufrahmen und ein verifiziertes, begrenztes Artefaktformat. Gemeinsame Operator-, Pure-Builtin- und atomare Schrittbudget-Implementierungen verringern Semantikduplikate. Runtime-Werttypen (`Value`, `FunctionValue`, `SqlRowsMetadata`, `ClassObjectRef`) und Map-Normalisierung liegen nun in `runtime/src/value.rs`; Dateisystem-, Netzwerk-, Prozess-, Mail- und Gesamtrichtlinien in `runtime/src/policies.rs`. Bestehende öffentliche Re-Exports bleiben unverändert. Die AST-Auswertungsbereiche liegen nun in `runtime/src/builtin_evaluation.rs`, `runtime/src/call_evaluation.rs`, `runtime/src/expression_evaluation.rs`, `runtime/src/statement_execution.rs`, `runtime/src/json_codec.rs`, `runtime/src/host_effects.rs`, `runtime/src/filesystem_metadata_builtins.rs`, `runtime/src/function_execution.rs`, `runtime/src/environment.rs`, `runtime/src/host_builtins.rs` und `runtime/src/path_builtins.rs` unter dem internen Namespace `ast_interpreter`. `Environment::with_scope` räumt Block-, Loop-, Match- und Lambda-Scopes auch bei zurückgegebenen Runtimefehlern zentral auf; reine Pfad-Hilfsfunktionen, Dateisystem-Metadatenoperationen sowie Uhr-, Umgebungs-, Zufalls- und Console-Built-ins mit Hostzugriff sind aus dem großen Built-in-Dispatcher in eigene Interpreterbausteine verschoben. Gemeinsame Capability-Analyse und statische Typprüfung liegen separat in `runtime/src/capabilities.rs` und `runtime/src/type_checker.rs`; beide behalten gemeinsame Sprachregeln für AST, MIR und CLI. Der formale Contract-Verifier liegt separat in `runtime/src/verifier.rs`; dies ist eine Strukturänderung ohne beabsichtigte Semantik- oder API-Änderung. Weitere Teilungen und vollständige Backend-Parität bleiben offen. AST, MIR und Bytecode teilen nun Überlauf- und Grenzüberschreitungszählung einschließlich paralleler Zweige; der erste überzählige Schritt wird gezählt, spätere parallele Fehler erhöhen den Zähler nicht weiter. Welche Operationen gezählt werden, bleibt backendabhängig. Ein coverage-guided Fuzzziel prüft nun den begrenzten Bytecode-Decoder und Encode/Decode-Rundläufe akzeptierter Artefakte. Ein echter 15-Sekunden-Lauf auf diesem Host verarbeitete 783.917 Eingaben ohne Absturz. Ein weiterer 10-Sekunden-Lauf startete mit einem aus `fn main() -> Int { return 7 }` erzeugten gültigen Artefaktseed und prüfte damit auch den akzeptierten Encode/Decode-Pfad. Dies sind begrenzte Läufe und kein umfassender Sicherheitsnachweis. AST bleibt vollständige Referenz und Standard. Vollständige AST/HIR/SQL-/Effekt-/Closure-/Nebenläufigkeitsparität, Richtlinienoberflächen für derzeit nicht unterstützte Bytecode-Effekte, breite Property-/Fuzz-Abdeckung und reproduzierbare Performance-Messungen bleiben offen; siehe Audit im Arbeitsprotokoll 1.1.5–1.3.0.
- [~] **Bytecode-Mailzustellung:** Bytecode v19 unterstützt `mail_deliver(event_id, payload_json)` über den gemeinsamen HostServices-Mailpfad. Der Verifier prüft Argumentzahl, `Result<Unit, String>`, `Email`-Deklaration und Vorprüfung vor der Argumentauswertung; zur Laufzeit sind außerdem eine ausdrückliche Freigabe und die gemeinsame SMTP-/Netzwerk-Policy erforderlich. AST-/Bytecode-Differentialtests verwenden Fake-Transporte. Bytecode v21 unterstützt SQL- und `mail_queue`-Operationen mit optionalen PDF-Anhängen in geprüften Transaktions-Batches; andere Transaktionseffekte bleiben nicht unterstützt. Statische SQL-Ausdrücke benötigen einen expliziten Datenbankkontext.
- [ ] Typisierte, validierte MIR mit Quellpositionsbezug, explizitem Kontrollfluss und Effekten als gemeinsame Zwischenrepräsentation implementieren.
- [~] **Skalare Konstantenfaltung in MIR:** Die explizite API `optimize_scalar_constants` faltet geprüfte Unary-, Binary- und BinaryChain-Ausdrücke der Typen `Int`, `UInt`, `Float`, `Bool` und `Char` über gemeinsame Operator-Helfer von AST/VM. Funktionsaufrufe und andere nichtliterale beziehungsweise nebenwirkungsbehaftete Operanden bleiben unangetastet; Laufzeitfehler und ihre Quellspans bleiben erhalten. `zelyra run --backend mir --optimize-mir` aktiviert den Pass ausdrücklich; normales `run` und MIR ohne diese Option bleiben unverändert. Das Schrittbudget zählt die ausgeführten MIR-Schritte nach der Optimierung; ein Differenzialtest bestätigt für das Beispiel weniger Schritte. Tests vergleichen außerdem AST und optimiertes MIR hinsichtlich Ergebnis, Division-durch-null-Diagnose und sichtbarer Funktionsausgabe und prüfen die CLI-Ablehnung unpassender Backends. Weitere Optimierungspässe bleiben offen.
- [~] **Bytecode-v24-Ausführungsschritt:** `mir/src/bytecode.rs` übersetzt die erreichbare Aufrufmenge mit unterstützten Skalar- und Aggregatwerten aus verifizierter MIR in ein deterministisches versioniertes Binärformat. Die VM nutzt iterative Aufrufrahmen, Quellspans, Schritt-/Tiefenlimits und Prüfungen für Stack, Kontrollfluss und Aufrufanzahl. Begrenzte Encode-/Decode-APIs prüfen Artefakte strukturell und lehnen fehlerhafte Eingaben ab. AST, MIR und Bytecode delegieren unäre und binäre Wertoperationen sowie gemeinsame reine Built-ins für Konvertierungen und Unicode-Textoperationen einschließlich `text_contains`, `text_trim`, `text_chars`, `text_to_lower`, `text_to_upper`, `text_ucfirst`, `text_starts_with`, `text_ends_with`, `text_replace`, `text_split`, `text_index_of`, `text_repeat`, `text_is_empty`, `text_capitalize`, `text_trim_start`, `text_trim_end`, `text_is_blank`, `text_substring`, `parse_bool`, `text_replace_result`, `text_split_result`, `text_pad_left` und `text_pad_right` – an gemeinsame Runtime-Helfer; der Bytecode-Compiler behält seine dokumentierte Typ-Teilmenge. Enum-Payloads und MIR-abgesenkte Match-Anweisungen/-Ausdrücke mit Guards sind über Encode/Decode differentiell geprüft. Differentialtests decken Arithmetik, Verzweigungen, Kurzschlusslogik, skalare `while`-/`loop`-Kontrollflüsse mit `break`/`continue`, einfach gebundene Integer-`range`-Schleifen (auch absteigend und leer), Array-/Set-Iteration mit Tupeldestrukturierung, Array-/Tupelkonstruktion und Indexierung, Arrayverkettung, Decimal-Arithmetik, Duration-Addition/Subtraktion, Zeitvergleiche, veränderliche Arrayupdates, neue/bestehende Tupelzuweisungen, typisierte generische Record-Konstruktion, Record-Spread-/Copy-with-Update einschließlich generischer Typargumente, Feldzugriffe und Array<Record>-Iteration, typisierte Map-Literale und Lookups mit Duplikatschlüsselregeln, Inline-Lambda-, statische und dynamische benannte Funktionsreferenz-`map`/`filter`/`find`/`reduce`-Aufrufe mit AST/MIR-/Encode-Decode-Differenzialtests, Laufzeitprüfung von Funktionsvor- und -nachbedingungen sowie Schleifeninvarianten (einschließlich break, continue, Return und verschachtelter Returns), gemeinsam genutzte reine Built-ins, Limits, fehlerhafte Tupelwerte, die fail-closed-Ablehnung nicht unterstützter Effekte und ausdrücklich freigegebene Clock-/Environment-/Console-/Random-Aufrufe mit verifizierter Capability-Vorprüfung vor Argumentauswertung ab. `zelyra run --backend bytecode` kann diesen Teilumfang jetzt ausdrücklich und ausschließlich auf Opt-in ausführen; einfache statische SQL-Ausdrücke nutzen die explizite Datenbankverbindung, Capability-/Schema-/Parametertypprüfung und read-only Speicherprüfung; SQL-Transaktionen und `mail_queue` mit optionalen PDF-Anhängen laufen ab Bytecode v21 als geprüfter Batch; andere Transaktionseffekte bleiben nicht unterstützt. AST bleibt Standard. CLI-Tests gleichen Exitcodes mit AST ab, prüfen das Schrittbudget und belegen die fail-closed-Ablehnung nicht unterstützter Konsolenausgabe. Siehe den [Bytecode-Leitfaden](bytecode.de.md). vollständige Closure-Parität über den unterstützten Sprachumfang, persistenter Cache, Signaturen, Effektinferenz, Hostservice-Abdeckung über die unterstützten Aufrufe `now()`/`env(name)`/`read_console(prompt)`, `random_int`, `uuid_v4`, `http_get`, `http_request`, `http_json`, `http_result`, `read_text`, `read_text_result`, `file_exists`, `dir_exists`, `file_size`, `write_text` und `write_text_result` hinaus, vollständige Sprachunterstützung, breite Differentialabdeckung und die Auswahl als Standardbackend bleiben offen; der experimentelle CLI-Schalter ist nur ein Opt-in; Runtime-Berechtigungen müssen aus vertrauenswürdiger Autorisierung stammen.
- [~] Interpreter und VM teilen Runtime-Semantik und Host-Services teilweise: Reine Standardbibliotheksoperationen sowie Operatoren auf Runtime-Werten verwenden für AST, MIR und Bytecode dieselbe Implementierung. Der Bytecode-Compiler begrenzt Operatoren weiterhin auf seine dokumentierte Teilmenge; die gemeinsamen Helfer erweitern das akzeptierte Bytecode-Format nicht stillschweigend. Host-Services und ausgewählte SQL-/Policy-Helfer sind ebenfalls gemeinsam. SQL-Transaktionssemantik, vollständige Capability-/Effektbehandlung, Verträge, Fehlerverhalten, Ressourcenabrechnung und Modul-APIs benötigen weiterhin einen systematischen Paritätsabgleich.
- [~] **AST-Typprüfung strukturiert:** Kontrollfluss-/Match-Exhaustiveness-Analyse liegt in `runtime/src/type_checker/flow.rs`; Typbeziehungen, Generic-Inferenz, Aliasauflösung und Typannotationsvalidierung liegen in `runtime/src/type_checker/type_relations.rs`. Rückgabe-/Fallthrough-, Loop- und Guard-Regeln sowie die Typregeln blieben unverändert. Gezielte Match-, Generic-Record-/Funktions- und Aliasfehler-Tests sowie Workspace-Tests bestehen. Dies ist interne Strukturierung; weitere Teilung, Flow-Diagnostik und vollständige MIR-/Bytecode-Parität bleiben offen.
- [ ] AST-Interpreter als auswählbare Referenz behalten, bis die VM den gesamten unterstützten Sprachumfang korrekt abdeckt. Differential-, Property-, Fuzz-, Sicherheits- und Regressionstests ergänzen.
- [~] Opt-in-CLI-Ausführung ist für AST, MIR und die unterstützte Bytecode-Teilmenge vorhanden. MIR-/Bytecode-Inspektion, sicheres versioniertes Caching, stabile maschinenlesbare Diagnosen und Studio-Integration bleiben offen, ohne bestehendes CLI-Verhalten zu brechen.
- [ ] CPU- und I/O-lastige Benchmarks getrennt durchführen; echte Messwerte für Startzeit, Speicher, Durchsatz und Kompilieraufwand dokumentieren. Keine unbelegten Performanceversprechen.
- [ ] Freigabekriterium: VM erst nach vollständiger Feature-Parität, Sicherheitsnachweisen, Differentialtests, dokumentierten Grenzen und reproduzierbaren Builds zum Standard machen.

### Querschnittliche Geschäftsplattform-Erweiterungen im Ziel 1.2.0–1.3.0

Diese Anforderungen stammen aus `/root/5erweiterungen.md` und gehören jetzt zum aktiven Arbeitsziel. Die vorhandenen Grundlagen sind teilweise vorhanden; keine Zeile behauptet die vollständige Funktion. Siehe den [ausführlichen zweisprachigen Plan](release-plans/1.3.0-business-platform-extensions.de.md).

Die AST-Implementierung bleibt Referenz, ist aber kein alleiniger Erfüllungsnachweis: Jede dafür ergänzte Sprach- oder AST-Semantik muss auch in MIR und Bytecode sicher abgebildet und differentiell geprüft werden oder im jeweiligen Backend mit einer klaren Unsupported-Diagnose fail-closed scheitern.

- [~] **1.2.0–1.3.0 compilererzwungene Mandantengrenzen und typisierte Abfragen:** Die erste Scope-Stufe parst und prüft `table ... scoped by tenant_id: TenantId` (`TenantId` muss nominal zu `Id` auflösen; die Spalte muss erforderlich sein). `zelyra check`, AST, MIR- und Bytecode-Verifier/Runtime weisen rohes SQL für solche Tabellen fail-closed ab; Bytecode v24 führt die Metadaten und gleicht sie bei DB-Ausführung mit dem geprüften Projektschema ab. Authentifiziertes Tenant-CRUD ist nur mit derselben Scope-Spalte zulässig. Autorisierter Tenant-Kontext, Query-HIR und sichere scoped Reads/Writes gehören zu 1.2.5; Backendparität ist Freigabekriterium von 1.3.0. Keine automatische N+1-Vermeidung ohne unabhängigen Nachweis behaupten.
- [~] **1.2.0–1.2.5 Request/Response und Forms:** Vorhandene `Request`-, Page/Form/CRUD- und CSRF-Pfade bilden die Grundlage. Typisierter Handler-/Middleware-/Response-Lebenszyklus, Feldfehler, sichere Übernahme zulässiger Eingaben und Redirects/Meldungen gehören zu 1.2.5. Zugangsdaten und sensible Eingaben dürfen niemals wieder ausgegeben werden.
- [~] **1.2.0–1.2.5 dauerhafte Jobs und Scheduler-Runtime:** Deklarative `command`-/`schedule`-Syntax und CLI-Ausführung/Liste/Export sind vorhanden; die Ausführung nutzt derzeit explizite Host-Scheduler. Begrenzter Jobvertrag und Scheduler-Lifecycle gehören zu 1.2.0; transaktionaler Jobvertrag, Lease-Worker und koordinierter Scheduler in `serve` gehören zu 1.2.5. Die Mail-Outbox allein ist kein Jobsystem.
- [~] **1.2.8–1.3.0 Wasm- und Sidecar-Brücke:** Versionierte HostServices und lokale gesperrte Quellpakete bestehen. Begrenzte versionierte Wasm-Ausführung und ausdrücklich gestartetes, authentifiziertes lokales Sidecar-RPC gehören zu 1.2.8; Cross-Backend- und Releaseparität gehört zu 1.3.0. Keine stabile Rust-ABI, kein automatischer Download und keine implizite Capability-Freigabe. Einer echten Providerintegration geht ein lokaler Mock voraus.
- [ ] **Backendübergreifendes Freigabekriterium:** Query-, Tenant-, Request- und Jobsemantik muss in AST, MIR und Bytecode übereinstimmen oder mit expliziter Unsupported-Diagnose abgewiesen werden. Sicherheits-, MariaDB-Integrations-, Kompatibilitäts-, Offline- und adversariale Tests sind erforderlich. 1.3.0 darf erst nach Erfüllung dieser Gates und des Detailplans als abgeschlossen oder produktionsreif gelten.

### Zusätzliche Enterprise-Erweiterungen bis 1.3.0

Die sechs Arbeitspakete aus `/root/weitereerweiterungen.md` sind als gestufte Roadmap-Ziele aufgenommen; sie sind keine bereits implementierten Funktionen. Abhängigkeiten, Sicherheitsgrenzen und konkrete Abnahmeschritte stehen im [Detailplan](release-plans/1.3.0-business-platform-extensions.de.md).

- [~] **1.1.5 – Architekturverträge:** Syntax und Semantik für Zustandsautomaten, prüfbare/bitemporale Tabellen, Ledger, LiveUI und Bundle-Deployment festlegen. Der deterministische, geprüfte JSON-Schnitt von `zelyra reflect` ist bereits implementiert (`docs/reflect.de.md`); Regeln für Schemaentwicklung und weitere Exportformate fehlen noch. Ein Entwurf allein gilt nicht als Implementierung.
- [ ] **1.2.0 – sichere Grundlagen:** Typisierte Metadaten und Compilerdiagnostik für verwaltete Übergänge, Audit-Tabellen und Ledger vorbereiten. LiveUI-Protokoll/Sitzungsgrenzen, Bundle-Format, eingebettete Assets, Health-Endpunkte und ausdrückliches Migrationsverhalten spezifizieren. Live-Datenbankschema, Markdown/OpenAPI und weitergehende semantische Beziehungen bleiben offen.
- [ ] **1.2.8 und 1.3.0 – geprüfte Vertikalschnitte:** Begrenzte, getestete Zustandsübergänge mit Contracts/Outbox, transaktionalen Ledger-Ausgleich, Audit-/Historienabfragen, versionierte `reflect`-Exporte, LiveUI, Wasm-/Sidecar-Integration und Deployment-Bundles liefern. Jeder ausführbare Pfad muss durch AST/MIR/Bytecode geprüft oder in nicht unterstützten Backends fail-closed abgewiesen werden; Backendparität und Distributionsfreigabe gehören zu 1.3.0. Bitemporale `as_of`-Abfragen hängen vom Query-HIR ab. Universelle Compliance-, Hochlast-, Diff-Latenz- oder Zero-Ops-Zusagen sind ohne Messungen und Backendnachweise keine Abnahmekriterien.
- [ ] **Nach 1.3.0 weiterführen, falls die Gates offen sind:** Vollständige bitemporale Historisierung, produktionsweiter LiveUI-Betrieb, portable statische Bundles und weitere `reflect`-Formate dürfen nicht durch einen Versionsstichtag als fertig erklärt werden.

**Gemeinsame Schutzregeln:** Zustandsfeldänderungen und Ledger-Buchungen müssen vor Commit geprüft und atomar sein. Auditdaten dürfen nicht durch direkte UPDATE-/DELETE-Pfade umgangen werden; Historien-DDL und `as_of` müssen Schema- und Datenbankgrenzen beachten. LiveUI-Sitzungen, Queues und Patches sind größen-/zeitbegrenzt und sitzungsisoliert. `--auto-migrate` ist ausschließlich explizit aktivierbar und darf vor erfolgreichem Plan-/Policycheck keinen Port öffnen. `reflect` exportiert nur geprüfte Projektmetadaten und behauptet keine Unveränderbarkeit oder Vollständigkeit, die nicht belegt ist.

### Kanonisches Projektlayout und Architekturregeln bis 1.3.0

Die Vorgaben aus `/root/struktur.md` sind als gestufte Roadmap-Anforderung
aufgenommen. Das dort gezeigte Layout und die Beispielsyntax sind Zielbild,
keine Behauptung, dass Manifestfelder, Autoloading, Scheduler-Syntax oder
`zelyra migrate-layout` bereits implementiert wären. Vor Syntaxänderungen
müssen Parser, Modulauflösung, Projektkonfiguration und Kompatibilität geprüft
werden. Details stehen im [Projektlayout-Arbeitsplan](release-plans/1.3.0-project-layout.de.md).

- [ ] **1.1.5 — Ist-Aufnahme und Architekturvertrag:** Projektdateien,
  `zelyra.toml`, `use`-/Paketauflösung, `zelyra new`, Build-/Bundlepfade sowie
  Route-, SQL- und Domain-Grenzen inventarisieren. Zuständigkeiten für
  `routes/`, `domain/`, `schema/`, `public/`, `assets/` und `plugins/` festlegen, ohne
  bestehende Projekte ungefragt umzustellen.
- [ ] **1.2.0 — Opt-in-Profil und modulare Grundlage:** Ein versionsmarkiertes,
  abwärtskompatibles Profil samt Validierung und expliziter Namespace-Auflösung
  implementieren, soweit es zur vorhandenen Modularchitektur passt.
  `zelyra new` kann es als ausdrückliche Vorlage erzeugen. Kein stilles
  Autoloading, bevor Importgraph, Mehrdeutigkeiten und Diagnostik feststehen.
- [ ] **1.3.0 — Prüfer und sichere Migration:** Quellgebundene Regeln für
  Route-Schreibzugriffe, HTTP-unabhängige Domänen und deklarierte
  Zustandsautomaten umsetzen. `migrate-layout --check` liefert zunächst nur
  deterministische Funde und Vorschläge; Schreibmodus braucht Vorschau,
  Konfliktprüfung und Wiederherstellung. Bundles und Starterprojekte nur für
  tatsächlich unterstützte Assets/Plugins abnehmen. Ältere Projekte bleiben
  ohne aktiviertes Profil im Legacy-Modus.
- [ ] **Nach 1.3.0, falls nötig:** Automatische Mehrdatei-Verschiebungen
  bleiben Folgearbeit, bis Recovery- und Plattformtests bestehen. Der
  begrenzte Wasm-/Sidecar-Umfang ist oben zugeordnet; eine breitere Runtime
  ist damit nicht versprochen.

### ◻️ [?] Nach 1.3.0 — optionales natives AOT-Backend

- [ ] Cranelift erst bei nachgewiesenem Bedarf prüfen. Typisierte MIR und gemeinsame Runtime wiederverwenden, Capability- und Vertragsgarantien erhalten und keine abweichende zweite Sprachimplementierung aufbauen.
- [ ] AOT-Entscheidung anhand von Benchmarks, Deploymentanforderungen und Wartungsaufwand treffen.

### 🗺️ [ ] Sprachkern vervollständigen (nach tatsächlichem Implementierungsstand)

- [ ] Benutzerdefinierte Enums mit Payload und vollständigem Matching, generische Structs/Records, typgeprüfte String-Interpolation mit Escaping geschweifter Klammern sowie optionale mutable Arrays mit geprüften effizienten Indexupdates im Repository prüfen.
- [ ] Nur tatsächlich offene Punkte planen; Abschluss erst bei Syntax-/API-Dokumentation, Implementierung, positiven/negativen Tests, Diagnostik und ausführbarem Beispiel.

## Aktuelle Meilensteine

- [x] **1.0.5 Studio-Logs:** Studio zeigt ein Logs-Fenster und speichert
  Studio-Start-/Setup-Ereignisse sowie Programm- und Vorschauausgaben als
  lokale JSONL-Dateien. Es gibt Stufen- und Textfilter, Geheimnisbereinigung,
  begrenzte Rotation und nur für den Benutzer lesbare Unix-Dateien. Siehe
  [Logs-Anleitung](studio-logs.de.md).
- [x] **1.0.5 installationsbewusste Deinstallation:** `zelyra uninstall`
  entfernt die aktive benutzereigene Programmdatei auch außerhalb des
  Standardordners. Der Befehl meldet PATH-Kopien, erkennt paketverwaltete
  Installationen und verlangt Bestätigung; Projektdateien und Daten bleiben
  erhalten. Siehe [Deinstallationsleitfaden](uninstall.de.md).
- [x] **1.0.5 Rechnungsdaten:** `issued_at` ist ein lokalisiertes reines
  Datumsfeld mit passender deutscher oder englischer Formatangabe. Der Server
  vergibt eine eindeutige Rechnungsnummer aus der gespeicherten Datensatz-ID
  und verändert sie bei späteren Änderungen nicht.
- [x] **1.0.5 Rechnungsstart und Übersicht:** `/` führt nun durch Kunden,
  Artikel und Rechnungen; `/views/invoicedashboard` ist als
  Rechnungsübersicht gestaltet. Deutsche und englische Beschriftungen passen
  zusammen.
- [x] **1.0.5 Tutorial komplett erläutert:** Die erzeugten deutschen und
  englischen Anleitungen erklären Setup-Befehle, Optionen, Quelldateien und
  wichtige Zeilen. Sie verfolgen die generierte Route `/invoices/new` bis zu
  `crud Invoice -> invoices` und erläutern `zelyra routes main.zyl`.

- [x] **1.0.6 Zelyra UI:** Ein
  lokal gebündeltes, versioniertes UI-Framework mit compilergeprüften
  Komponenten, semantischen Darstellungsvarianten, Themes und progressiver
  Interaktion ist integriert. Standard-CRUD, Studio-Layouts, das Rechnungstutorial
  und die MariaDB-Starter verwenden Zelyra UI; Tabler wird nur bei ausdrücklicher
  CSS-Auswahl geladen. Balken-, Linien- und Ringdiagramme sowie eine
  druckoptimierte Rechnung mit Browser-PDF-Ausgabe sind vorhanden. Die
  E-Mail-Komponente öffnet einen lokalen Entwurf; eine gespeicherte PDF wird
  dort manuell angehängt. Automatischer PDF-Anhangversand und ein serverseitiger
  PDF-Renderer sind nicht enthalten. Keine vollständige Tabler-
  Klassenkompatibilität. Siehe
  [Zelyra-UI-Handbuch](ui.de.md) und das
  [Aufgabenprotokoll](release-plans/1.0.6-zelyra-ui-progress.de.md).

- [x] **1.0.7 Sprache, Laufzeit und PDF-Mail — veröffentlicht:** Ergänzt
  wertliefernde If-Ausdrücke, Record-Updates, typisierte Lambda-Pipelines,
  Match-Guards und verschachtelte Record-Muster, Tupelzuweisung sowie eine
  begrenzte konfigurierbare Rekursionstiefe. `--max-steps` zählt
  Anweisungen und Auswertungen über ein gemeinsames Budget, auch in
  `parallel`-Zweigen. Die bestehende Semantik nominaler IDs,
  `Result`-gebundener API-Fehler, Offline-Prüfung von Transaktionen,
  View-Slot-Fallbacks, Auth-Handler und `parallel`-Join wird abgesichert.
  `render_pdf("ViewName", record)` erzeugt lokal ein durchsuchbares Text-PDF;
  die Mail-Outbox kann ein geprüftes PDF anhängen. CSS, Bilder und Diagramme
  werden vom Server-PDF nicht gerendert. Veröffentlicht als
  [`v1.0.7`](https://github.com/sf1976/zelyra/releases/tag/v1.0.7); siehe das
  [Arbeitsprotokoll 1.0.7](release-plans/1.0.7-language-runtime-progress.de.md).

- [x] **1.1.0 Studio/LSP — veröffentlicht:** Offlinefähiges Einzeldatei-Studio
  mit eingebettetem CodeMirror 6, compilerbasierter Analyse, gemeinsamen
  Overlays, Verifier- und Typed-Hole-Ansicht, Daten-/Nullability-Karte,
  isolierter statischer Vorschau, Theme- und Capability-Werkzeugen,
  Budgetausführung, PDF-/Outbox-Ansicht, semantischem Rename und
  wiederherstellbarem Datei-Rename. `zelyra lsp` nutzt denselben Sprachdienst.
  Dazu kommen geprüfte Stringinterpolation und ausdrücklich veränderliche
  lokale Arrays. Grenzen: Analyseaufträge sind begrenzt, aber nicht kooperativ
  abbrechbar; Rename unterstützt statisch aufgelöste Symbolarten; serverseitige
  PDFs sind textorientiert. Siehe die [Release-Notizen 1.1.0](release-notes/1.1.0.de.md)
  und das [Arbeitsprotokoll](release-plans/1.1.0-studio-deep-native-progress.de.md).

- [x] **1.1.1 Studio und Portkonflikte — veröffentlicht:** Erwartete
  Browserabbrüche füllen stderr nicht mehr, der erfolgreiche Webserver-Start
  wird als Information protokolliert und Canvas erklärt die getrennte,
  ausdrückliche Aktion zum Öffnen der laufenden Anwendung. Bei belegtem
  angefordertem Port bindet `zelyra serve` einen freien lokalen Port;
  `zelyra doctor` meldet den Konflikt als Warnung, wenn ein Ausweichport frei
  ist, und liest standardmäßig die Projekt-`.env`. Outbox-Hinweise zu
  Verbindung und Tabelle nennen die optionale
  Einrichtung und lassen sich bei Nichtnutzung schließen. Mit Chromium,
  echter lokaler Webanwendung, unerreichbarer MariaDB, Port-Fallback-Tests und
  dem kalten Compose-Rechnungstutorial geprüft.
  Siehe [Release-Notizen 1.1.1](release-notes/1.1.1.de.md).

- [x] **1.1.2 Entfernung des Laravel-Migrators — veröffentlicht:** Das
  experimentelle Laravel-Migrationstool wurde aus dem Produkt entfernt; Quellcode
  und frühere Analysen liegen in einem Archiv außerhalb des Repositorys. Als
  [`v1.1.2`](https://github.com/sf1976/zelyra/releases/tag/v1.1.2)
  veröffentlicht; der Release-Workflow für Linux- und Windows-Artefakte war
  erfolgreich.

- [~] **1.1.5 wiederverwendbare Module und Pakete — aktiv:** Offline-
  Paketsystem, typisierte Modul-APIs, Compilerprüfung, backendunabhängige
  Hostservices, sichere Einstellungen und ausführbare Deployment-Nachweise
  fertigstellen. Paketvertrauen, Erweiterungsgrenzen, Einstellungen auf
  Anwendungsseiten, Austauschmodule und Release-Gates bleiben offen; siehe den
  ausführlichen Abschnitt zu 1.1.5. Ein Laravel-Import gehört nicht dazu.
- [ ] **1.2.0 Betriebs- und Deployment-Grundlage:** Command-, Scheduler- und
  Jobabläufe schrittweise hinzufügen, typisierte Request-/Form- und Query-
  Grundlagen schaffen, `public/` bereitstellen, Produktionsbetrieb sowie
  Apache-/Nginx-Reverse-Proxy dokumentieren und Linux-systemd-Dienste mit
  ausdrücklich festgelegtem Port installieren.
- [ ] **1.2.5 Integration der Geschäftsabläufe:** Begrenzte Mandanten-, Form-
  und Jobpfade sowie `zelyra service change --port <port>` mit Stopp, Neustart
  und Rollback fertigstellen. Weitere Serviceplattformen nur mit Lifecycle-
  Tests ergänzen.
- [ ] **1.2.8 Integrität und Distribution:** Geprüfte Schnitte für
  Zustandsübergänge, Audit/Historie, ausgeglichenes Ledger, Reflection,
  Healthchecks, Bundles, statisches `public/` und Reverse-Proxy-Sicherheit
  integrieren; keine unbelegten Compliance-Zusagen.
- [~] **1.3.0 Interpreterarchitektur — nach den 1.2.0-Grundlagen aktiv:**
  Typisierte MIR und die Opt-in-Bytecode-VM weiterentwickeln; der AST bleibt
  Referenz. Geprüfter Bytecode soll erst nach vollständigen Paritäts- und
  Sicherheitsnachweisen Standard für `run`, `serve` und Tasks werden. Bis dahin
  scheitern nicht unterstützte Operationen geschlossen und der aktuelle
  Standard bleibt bestehen. Ziel ist portabler Bytecode, keine native
  Maschinencodegenerierung. Die AST-Interpretermodule für Scopes, Pfade, Host-Built-ins sowie
  Dateisystem-Metadaten und Datei-I/O werden schrittweise abgegrenzt. Die
  native SQL-Zeilendecodierung wird jetzt backendübergreifend von AST, MIR und
  Bytecode genutzt; tabellenbasierte `Invoice[]`-Projektionen werden als
  separate SQL-Metadaten durch MIR und den Bytecode-Roundtrip getragen. Das
  `examples/business_modules` läuft damit auf MariaDB in allen drei Backends.
  Dies ist keine vollständige Backend-Paritätsabnahme; die implementierte
  Teilmenge bleibt experimentell.
  Semantikparität, breitere Differential-/Sicherheitstests und reproduzierbare
  Performancemessungen sind noch offen; siehe den ausführlichen Abschnitt zu
  1.3.0.
- [⛔] **Laravel-Website-Migration — entfernt/zurückgestellt:** Das Migrationstool gehört nicht mehr zum Projekt. Quellcode und frühere Befunde liegen außerhalb des Repositorys im Archiv. Eine Neubewertung erfolgt erst nach den aktiven Architektur-Meilensteinen oder bei ausdrücklicher Priorisierung.

- [ ] **1.2.0 — Compose-Prüfung im Installer:** Die Installer
  erkennen Docker Compose v2 oder neuer und unterscheiden es von der veralteten
  Python-Version v1. Docker bleibt für die Kerninstallation optional. Fehlt
  Compose v2, bieten interaktive Linux-Installer auf unterstützten
  Paketquellen nach ausdrücklicher Zustimmung die Installation des Compose-v2-
  Plugins an; andernfalls zeigen sie passende Installationshinweise. Ohne
  Zustimmung oder in nicht interaktiven Läufen wird nichts mit erhöhten
  Rechten geändert. Die Hinweise machen klar, dass Docker-Compose-Tutorials
  bis zur Installation nicht startbereit sind.
- [ ] **1.2.0 — interaktive Konsolenmenüs:** Eine leicht
  deklarierbare Menü-API und `zelyra new --template console-menu` sollen aus
  Titel, Beschreibung und Auswahlpunkten in wenigen Zeilen ein vollständiges
  Terminalmenü erzeugen. Das Menü erhält eine hochwertige Darstellung mit
  klarer Auswahlhervorhebung, Tastaturbedienung, einem Layout für schmale
  Terminals und einem nummerierten Fallback. Farb- und Unicode-Ausgabe
  berücksichtigen Terminalfähigkeiten, `ZELYRA_COLOR` und `NO_COLOR`; ohne
  ANSI-Unterstützung erscheint eine einfache Textauswahl. EOF beendet die
  Auswahl ohne Hängen. Der Starter, ein lauffähiges Beispiel und Tests decken
  Auswahl, EOF, schmale Breiten sowie Farb- und Fallback-Modus ab.
- [x] **1.0.2 Zuverlässigkeit von Studio und Rechnungstutorial:** Generierte
  MariaDB-Port-Platzhalter in Studio und gestarteter Anwendung einheitlich
  auflösen. Beim ersten Start Compose starten und das Schema nur einrichten,
  wenn die aktive Adresse der erzeugten lokalen Datenbank-URL in `.env`
  entspricht; vorübergehende Verbindungsabbrüche wiederholen und dauerhafte
  Setupfehler sichtbar melden. Externe oder überschriebene Adressen lösen keine
  automatischen Schemaänderungen aus. Datenbankkarte und Datensatzvorschau des
  Rechnungstutorials funktionieren danach. Die Programmausgabe ist mit 13 px
  besser lesbar, ohne die Code-Editor-Schrift zu verändern. Browserprüfungen
  decken eine frische Compose-Datenbank und ein externes MariaDB-Projekt ab;
  MariaDB- und PostgreSQL-Studioabläufe bestehen.
- [x] **1.0.2 Schrittweiten für Integer-Bereiche:** `for i in 1..21 step 2`
  mit exklusivem Ende, Standardschrittweite 1, positiven und negativen Werten,
  leeren Bereichen bei falscher Richtung und geprüfter Schrittweite ungleich 0
  unterstützen. `range(start, end, step)` liefert dieselbe Folge als begrenztes
  Array. Parser-, Laufzeit-, Typ- und Grenztests bestehen.
- [~] **Folgearbeit an der Sprachergonomie:** Typisierte If-Ausdrücke,
  Record-Updates, lexikalische Lambdas, geschützte Record-Muster,
  Tupelrückgaben/-destrukturierung und kontextuelle Feldnamen sind in 1.0.7
  implementiert und getestet. Benutzerdefinierte Operatoren und markierte
  Schleifensteuerung gehören weiterhin nicht zur unterstützten Grammatik;
  eigene Operatorrangfolgen sind nicht konfigurierbar.
- [x] **String- und Math-Bibliothek:** Unicode-Texthelfer, geprüfte Mathematik,
  Rundungsregeln, Fehlerresultate, typgeprüfte Interpolation mit escaped
  Literal-Klammern und Pipeline-Komposition sind durch Compiler-/Runtime-Tests
  abgedeckt. Interpolierte Strings sind Klartext; HTML-Escaping erfolgt weiter
  an der View-Grenze.
- [~] **Sammlungen und Algorithmen:** Typisierte Pipelines mit `map`, `filter`,
  `find`, `reduce`, gefangene lexikalische Lambdas, Tuple-`entries`, geprüftes
  `map_increment` und opt-in veränderliche Arrays mit Indexupdates sowie
  `push_mut`/`pop_mut` sind implementiert. Spezielle Stacktypen und eine
  breitere Übungssammlung bleiben Folgearbeit; funktionale Arrays und Maps
  behalten ansonsten ihre Wertsemantik.
- [ ] **1.2.5 — interaktive Studio-Konsole:** Studio Run soll
  ein interaktives Eingabefeld für `read_console` mit sichtbarer ausstehender
  Eingabe, Zeilenübermittlung, Programmfortsetzung sowie explizitem EOF/Abbruch
  und begrenzter Laufzeit erhalten. CLI-Eingabe bleibt verfügbar; dasselbe
  Zahlenratespiel wird in CLI und Studio geprüft.
- [ ] **1.2.5 — idiomatische Algorithmusübungen:** Ausführbare,
  erläuterte Konsolenbeispiele für verbreitete Algorithmen, einschließlich
  Sortierung, Suche, Stacks, Rekursion und dynamischer Programmierung,
  mit geprüften Eingaben, begrenztem Aufwand, Sollausgaben und Grenztests.
- [~] **Compilerdiagnosen:** Gezielte Fehler und Quellbereiche gibt es für
  Syntax, Typen, Callbacks, Pipelines und Laufzeitgrenzen. Breitere
  Korrekturvorschläge für verschachtelte Fehler bleiben Verbesserungsbedarf.
- [x] **1.0.4 Neustartzuverlässigkeit des Rechnungstutorials:** Jedes neu
  erzeugte Tutorial erhält einen eigenen stabilen Compose-Projektnamensraum.
  Ältere markierte Tutorials beheben veraltete Datenbankzugangsdaten durch
  Start einer isolierten Datenbank; das bisherige Volume bleibt erhalten.
  Compose-v2-Versionsangaben ohne Patchnummer werden aus beiden Ausgabekanälen
  erkannt. Compose 5.6.0 / Engine 29.8.2 bestehen sowohl die Migration eines
  alten Volumes als auch einen frischen Start. Alle sechs Tutorialrouten liefern
  HTTP 200; Beispieldaten: 2 Kunden, 3 Artikel, 1 Rechnung und 2 Positionen. Bei
  belegtem Port 3306 wurde für MariaDB auf 3307 ausgewichen.
- [x] **1.0.3 Rechnungstutorialstart über CLI und Studio:** Bei Compose-
  Portkonflikten automatisch freie Folgeports für MariaDB oder die
  Webanwendung wählen, die lokale `.env` und passende MariaDB-URLs aktualisieren
  und den Start begrenzt wiederholen. Das Rechnungstutorial erhält
  wiederholbare lokale Beispieldaten. Studio Run startet den passenden lokalen
  Datenbankdienst, richtet das Schema ein, lädt den Seed nur im markierten
  lokalen Tutorialprojekt und startet danach die Vorschau. CLI und Studio
  liefen gegen Docker Compose 5.6.0 / Engine 29.8.2; HTTP-Dashboard und
  gespeicherte Beispieldatensätze wurden geprüft.
- [x] **1.0.1 Sprachprüfung des veröffentlichten Artefakts:** 80 Fälle,
  100 Aufgaben-Wiederholungen, 600 Zahlenvergleiche und 1.408 Kalender-/UTC-
  Referenzfälle ohne unerwartete Befunde geprüft; Installation/Update/Rückkehr
  und MariaDB-/PostgreSQL-HTTP-/SQL-Prüfungen ebenfalls bestanden. Siehe den
  [Prüfbericht](release-readiness/1.0.1-language-review.de.md).
- [x] **1.0.1 Tutorialstart korrigiert:** Compose v2 oder neuer voraussetzen
  und Python Compose v1 vor dem Start abweisen. `ContainerConfig`-Fehler und
  Ubuntu-Crashreport-Rechte von Docker-Socket-Zugriff unterscheiden. Erzeugte
  Projekte und Datenbank-Volumes für Wiederholung erhalten. Den Studio-
  Projekteinstieg für Konsole und Webvorschau korrigieren; siehe die
  [Tutorialanleitung](tutorials/invoice.de.md).
- [x] **0.8.0 stabil veröffentlicht:** Zelyra Studio enthält eine lokale,
  schreibgeschützte Datenbankkarte für MariaDB und PostgreSQL sowie eine
  geführte CRUD-Quelltextvorschau mit Compilerprüfung und ausdrücklicher
  Übernahme. Dazu kommen eine begrenzte MariaDB-Outbox, Kompatibilitäts-Fixtures
  für Sprachlinie 0.1 und veröffentlichte Linux-/Windows-Artefakte samt SBOM.
  Der genaue Nachweis steht in
  [0.8.0-Kandidatenprüfung](release-readiness/0.8.0-candidate-verification.md).
- [x] **Stabiles Release 0.9.0 veröffentlicht:** Geführte Studio-Geschäftsabläufe,
  Rechnungstutorial, vorschau-gebundene `zelyra text edit`-CLI und erweitertes
  Sprachkorpus sind integriert. Optionales lokales TOTP bleibt auf MariaDB mit
  persistenten Sessions und Audit begrenzt. Exakte CI, Linux-/Windows-Artefakte,
  Prüfsummen, SBOMs, Attestierungen und das zweisprachige Website-Handbuch sind
  im [0.9.0-Releaseprüfnachweis](release-readiness/0.9.0-candidate-verification.md)
  bestätigt. Die menschliche Abnahme bleibt für 1.0.0 reserviert.
- [x] **Stabiles Release 0.9.1 veröffentlicht:** Studio-Sitzung, Speichern,
  Problems-Bereich und Rechnerabläufe des Rechnungstutorials sind erweitert;
  Sprachlinie 0.1 verwendet `#`- und `##`-Kommentare. Vollständige CI,
  Linux-/Windows-Artefakte, Prüfsummen, SBOMs, Attestierungen und Website-Update
  sind im [0.9.1-Releaseprüfnachweis](release-readiness/0.9.1-candidate-verification.md)
  belegt. Die unabhängige menschliche Abnahme bleibt für 1.0.0 reserviert.

- [x] **Stabiles Release 0.9.3 veröffentlicht:** Studio-Datenbrowser mit
  begrenzten schreibgeschützten Seiten, geprüfte Quellspaltenänderungen,
  bestätigte MariaDB-Anlage, schreibgeschütztes Laravel-Inventar, native
  Typwerte sowie CSV- und Labyrinth-Beispiele. Main-CI und Releasebuild auf
  dem exakten Tag-Commit sind erfolgreich; Artefakte, Prüfsummen, SBOMs und
  Attestierungen sind veröffentlicht. Die Typmatrix bleibt bei 17 unterstützt,
  5 teilweise und 3 nicht verfügbar. Nachweis: [0.9.3-Kandidatenprüfung](release-readiness/0.9.3-candidate-verification.md). Die menschliche
  Produktabnahme bleibt für 1.0.0 reserviert.
- [x] **Stabiles Release 0.9.5 veröffentlicht:** Typisierte Anwendungs-Mails
  werden transaktional in die MariaDB-Outbox eingereiht und über einen
  ausdrücklich gestarteten SMTP-Worker mit verpflichtendem TLS,
  Authentifizierung, privaten CA-Wurzeln, begrenzten Nachrichten und redigierten
  Fehlern zugestellt. Versionierte Laravel-Prüffunde decken Autorisierung,
  mandantenähnliche Felder, Geldberechnungen und Dateioperationen ab. Das
  Release bleibt experimentell; Zustellung erfolgt mindestens einmal und
  PostgreSQL-Outbox, Templates und Anhänge bleiben offen. Exakte Release-CI,
  MariaDB 10.11/11.4/11.8/12.3, MySQL 8.4, Linux-/Windows-Builds, SBOMs und
  Attestierungen sind am [0.9.5-Release](https://github.com/sf1976/zelyra/releases/tag/v0.9.5)
  angehängt. Die menschliche Abnahme bleibt für 1.0.0 reserviert.

- [~] Der 0.4-Entwicklungszweig ergänzt Anzeige und Sperrung dauerhafter
  Sitzungen mit Verwaltungsrecht, CSRF-Prüfung und Audit-Anfrageereignis.
  Unter `/account/sessions` können angemeldete Nutzer außerdem eigene
  dauerhafte und flüchtige Sitzungen mit CSRF- und Same-Origin-Prüfung anzeigen
  und beenden. Speichersitzungen laufen nach 24 Stunden ab. Eine begrenzte,
  escaped `User-Agent`-Angabe wird optional gespeichert, wenn die Sitzungstabelle
  `device_label` deklariert; sie bestätigt keine Geräteidentität.
  Passwortwiederherstellung ist im 0.4-Entwicklungszweig teilweise umgesetzt:
  MariaDB speichert Einweg-Token-Hashes und eine AES-256-GCM-verschlüsselte
  Outbox, die nach SMTP-Ausfällen (mit 30 Sekunden Wartezeit) und
  Prozessneustarts erneut zustellt. Das E2E
  prüft die Wiederaufnahme nach Neustart, wenn kein älterer Worker aktiv ist.
  Zustellung ist mindestens einmalig; das Zeitfenster zwischen SMTP-Annahme und
  Datenbankbestätigung kann Duplikate erzeugen, Schlüsselverlust lässt wartende
  Nachrichten unlesbar zurück. Generische Antworten, Audit, CSRF, SMTP und
  Sitzungswiderruf werden weiter im MariaDB-/SMTP-Senken-E2E geprüft. Der E2E-Test sendet außerdem zwei
  gleichzeitige Einlösungen desselben Tokens und prüft genau einen Erfolg,
  eine Ablehnung und die Authentifizierung mit nur dem siegreichen Passwort.
  MariaDB-Advisory-Locks serialisieren Token-Ersetzung und Outbox-Einfügung;
  der Worker liest wartende Zeilen in ID-Reihenfolge. Advisory-Locks koordinieren
  Ausstellung und Zustellung auch über mehrere Instanzen; ein Zwei-Instanzen-E2E verzögert das erste
  SMTP-Relay, während die zweite Instanz das Token ersetzt, und prüft, dass
  die spätere E-Mail zum gespeicherten Token gehört. Veraltete Tokens in der
  Mailqueue werden übersprungen. Das MariaDB-E2E lehnt ein Token mit
  `expires_at = NOW()` ab und friert die Datenbankzeit ein, um das produktive
  Token-Prädikat vor, exakt bei und nach Ablauf zu prüfen. Gleichheit wird
  abgelehnt. Reset-Limits verwenden jetzt eine deklarierte MariaDB-Tabelle und
  einen instanzübergreifenden Schlüssel-Fingerabdruck. Unabhängige
  Sicherheitsprüfung und vollständige Kontolebenszyklus-Nachweise bleiben offen.
  Bestehende Sitzungstabellen ohne `id`-Spalte behalten ihr bisheriges Verhalten;
  Anzeige und Sperrung im Verwaltungsbereich setzen diese Spalte weiterhin voraus.

- [~] Die [Docker-Modulabnahme](module-docker-acceptance.de.md) prüft die
  Gesamtanwendung, getrennte Rechnungs-/Inventarexporte, fehlende `.env`,
  ausgeschlossene Routen und verweigerte MariaDB-Zugriffe. Die Testkonten
  erlauben nur Lesen; vollständige schreibende CRUD-Abnahme bleibt offen.

- [~] Der 0.4-Release-Workflow erstellt deterministische SPDX-2.3-SBOMs für
  Linux und Windows und prüft den Binary-Hash. CI führt `cargo-deny` mit fester
  Version für Linux- und Windows-Abhängigkeitsgraphen aus. Die Policy prüft
  Advisories, Lizenzen, Quellen, Wildcard-Anforderungen und doppelte Versionen;
  doppelte Versionen von `base64` und `getrandom` warnen, und für
  `rustls-pemfile` besteht eine dokumentierte Ausnahme zum Unmaintained-Advisory.
  Menschlicher Dependency-/Lizenzaudit, eine Attestierungsprobe mit einem
  veröffentlichten Kandidaten und die Prüfung der nativen Artefakte bleiben
  offen. Getaggte Builds attestieren beide Plattformarchive samt SBOM; siehe
  die [Anleitung zur Artefaktprüfung](release-readiness/artefaktverifikation.de.md).

- [~] Der HTTP-Server verarbeitet bis zu 64 Verbindungen parallel und verwirft
  weitere Verbindungen, statt eine unbegrenzte Worker-Warteschlange anzulegen.
  Lesen und Schreiben je Verbindung sind auf 30 Sekunden begrenzt; verspätete
  Antworten entfallen. MariaDB-Poolwartezeiten und Abfragen sind jetzt durch
  die verbleibende Dialogfrist begrenzt; abgelaufene Abfragen werden serverseitig
  abgebrochen. CPU-intensive oder andere blockierende Handler-Arbeit bleibt
  synchron und kann weiterhin einen Worker belegen. Die
  [HTTP-Betriebsanleitung](http-operations.de.md) definiert TCP-Listener-
  Erreichbarkeit, die eingebaute Prozess-Liveness-Route, anwendungseigene
  Readiness und begrenzte Retries für sichere oder idempotente Anfragen.
  Abhängigkeits-Readiness und ihre Abnahme in der erzeugten Anwendung bleiben
  offen; der Server wiederholt nicht automatisch.

- [x] Sprachkern mit Lexer, Parser, AST, Funktionen, Ausdrücken, Kontrollfluss,
  unveränderlichen Bindings als Standard, Arrays, deterministischen typisierten
  Maps, Records, Option, Result und exhaustivem Pattern Matching als Anweisung
  und Ausdruck. `Int`/`UInt`-Überläufe, Division durch null und mehr als 20
  verschachtelte Funktionsaufrufe liefern kontrollierte Runtimefehler; `Unit`
  ist in Typsignaturen nutzbar.
  `examples/type_system` hält die geprüfte Typabdeckung und nicht unterstützte
  Bereiche fest.
- [x] Statische Prüfung, nominale Fachdaten-Typen, Capabilities, Contracts,
  Runtime-Prüfungen und ein erster Verifier.
- [x] MariaDB als primäres Schema-Backend, Inspektion, DDL-Planung,
  Schema-Anwendung, SQLite-Unterstützung und PostgreSQL-Schema-Unterstützung.
- [x] Typisiertes natives SQL mit Schema-, Spalten-, Parameter-, Nullability-,
  Ergebnis-, Transaktions- und sicherer Parameterprüfung.
- [x] HTTP-Server, Seiten, Formulare, CRUD, Suche, Filter, Sortierung,
  Pagination, APIs, OpenAPI und TypeScript-Client-Erzeugung.
- [~] Wiederverwendbare Web-Views: benannte Layouts, Seitenkomposition, ein
  validierter Content-Slot und typisierte selbstschließende Komponenten mit
  Properties sind verfügbar. Benannte Slots mit Fallback-Inhalt, CRUD-View-
  Überschreibungen und ressourcenspezifische Inhalte für benannte CRUD-Layout-
  Slots sind verfügbar. Statische Slot-Markups dürfen geprüfte Komponenten
  verwenden; datensatzgebundene Slot-Inhalte bleiben offen. Projektlokale
  CSS-Token-Overrides sind implementiert; vollständige Theme-Erstellung und
  -Auswahl bleiben offen.
- [~] Eingebaute deutsche/englische UI-Kataloge werden über
  `ZELYRA_LANGUAGE` gewählt; `ZELYRA_LEVEL=learn|work` steuert die
  kontextbezogene Lernhilfe. Die minimalen und
  Maschinenverwaltungs-MariaDB-Starter sowie erzeugte CRUD-, Formular-,
  Tableview-, Login- und Authentifizierungsverwaltungsseiten haben
  standardmäßig einen responsiven Zelyra-Anwendungsrahmen. Explizite CRUD-
  Layouts haben Vorrang; selbst verfasste Seiten bleiben unverändert.
  Projektlokale CSS-Overrides für dokumentierte Design-Tokens sind jetzt
  verfügbar. Projektlokale `locales/de.json`- und `locales/en.json`-Overlays
  können alle kataloggebundenen generierten Texte für Rahmen, CRUD, Formulare,
  Tableviews, Login, Auth-Verwaltung, Validierung und Lernhilfe ergänzen oder
  überschreiben, zusätzlich zu markierten View-/Texteinträgen. Parametrisierte
  Beschriftungen und generierte Feldnamen werden unterstützt; HTML-Escaping
  und englischer Fallback bleiben aktiv. Fachdatensätze und unmarkierte eigene
  Texte werden nicht übersetzt. Umfassendere Vorlagenabdeckung und ein
  vollständiger Theme-Editor bleiben offen.
- [x] Authentifizierung, persistente Sessions, Argon2-Passwörter, direkte und
  rollenbasierte Berechtigungen, Browserverwaltung und MariaDB-Audit.
  Schreibende Browseranfragen benötigen zusätzlich gleichursprüngliche
  Nachweise; das 0.2.0-Release ergänzt eine Loopback-Host-Allowlist gegen
  manipulierte Hosts und DNS-Rebinding.
- [~] Audit-Befehle: Anzeige, JSON/CSV-Export, strukturelle Prüfung und
  bestätigtes Bereinigen sind verfügbar. Manipulationssichere Verkettung ist
  optional verfügbar; Archivierung und Aufbewahrung bleiben offen.

## Querschnitt: KI-native Entwicklung

Das strategische Produktziel lautet: **Die KI schreibt. Zelyra prüft.**
Menschen und KI-Systeme sind gleichwertige Codeautoren; Compiler, Tests,
Capabilities und Sicherheitsregeln bleiben jedoch maßgeblich. Dies ist eine
KI-native und KI-unabhängige Architekturanforderung für jede Phase, kein
Feature eines bestimmten Anbieters.

- [~] **Stufe A — Maschinenbasis:** Versionierte JSON-Diagnosen für
  `check --format=json`, stabile Codes, Source-Spans, deterministische
  Ausgabe, eine schreibgeschützte Projektübersicht mit
  `context --format=json` und Format-Tests sind implementiert. Projektdateien
  verwenden projektrelative portable Pfade; weitere Befehle und eine
  vollständige Secret-Redaction bleiben offen. Im unveröffentlichten
  0.4-Entwicklungszweig enthält der deterministische Modulkontext außerdem
  Importkanten und die derzeit unterstützten öffentlichen Funktionen, Typen
  und Records je Quelldatei (`modules[].exports`). `zelyra module plan` zeigt
  zusätzlich schreibgeschützt den Abschluss expliziter Importe und statisch
  erkannter Verweise aus dem Wirkungsgraphen – wahlweise ab einer Quelldatei
  oder einer unterstützten Seiten-/API-/CRUD-/Formular-/Tableview-Ressource.
  `modules[].declarations` macht alle erkannten Deklarationen der einbezogenen
  Quelldateien sichtbar. `declaration_closure` trennt davon den ausgewählten
  Startpunkt und statisch erreichbare Deklarationen von zusätzlichen
  Deklarationen in den einbezogenen Dateien; `configuration_edges` weist die
  Datenbankkonfiguration separat aus. `database.configurations` nennt Backend,
  logischen Datenbanknamen, Quelldatei und die bevorzugte Laufzeitvariable
  `ZELYRA_DATABASE_<NAME>_URL` (`DATABASE_URL` bleibt Fallback). Die Laufzeit
  hat weiterhin genau eine projektweite Verbindung; die Namenszuordnung ist
  noch keine benannte, wiederverwendbare
  Datenbankschnittstelle. Die Analyse bleibt unvollständig.
  Enthalten sind bekannte Kanten
  von Seiten zu Views/Komponenten, Page-SQL und Formular-/CRUD-Aktions-SQL zu
  Tabellen, API-Handlern zu Funktionen, geschützten Ressourcen zur
  Authentifizierung, Authentifizierung zu Tabellen sowie Typreferenzen aus
  APIs, Funktionssignaturen und Funktionskörpern (einschließlich expliziter
  lokaler Typen, Record-Literalen und SQL-Ergebnistypen), Records und Type
  Aliases (einschließlich Verweisen aus Tabellenspalten und typisierten
  Ressourcenfeldern), Tabellenrelationen und
  Datenbankkonfiguration; nicht auflösbare Verweise werden ausgegeben.
  SQL-Tabellenkanten zeigen außerdem den konservativ erkannten Modus
  `read`, `write`, `read_write` oder `unknown`; komplexe verbundene
  `UPDATE`-/`DELETE`-Formen bleiben `unknown`. `schema_ownership` meldet den
  Tabellenbesitzer nur als aus der Quelldatei abgeleitete Vermutung und setzt
  `enforced: false`. Diese Analyse ist Beobachtung, keine Berechtigungs- oder
  Eigentumsdurchsetzung; nicht erkannte SQL-Formen können fehlen.
  Dynamische oder nicht modellierte Abhängigkeiten, Assets,
  Laufzeitkonfiguration, externe Dienstverträge und Docker-Paketierung bleiben
  außerhalb dieser Vorschau. `complete_deployment` bleibt `false`; der Plan ist
  weder ein vollständiges Exportmanifest noch Teil des veröffentlichten 0.3.0.
- [x] **Stufe B — kanonischer Quellcode:** Das deterministische `zelyra fmt`
  formatiert parsebaren Quellcode, unterstützt `--check` für CI, bewahrt
  Kommentare sowie rohe SQL-/HTML-Blöcke und besitzt Idempotenz- und
  Semantiktests.
- [~] **Stufe C — typisierte Lücken:** Ausdrucks-Lücken mit `_` melden jetzt
  erwartete Kontexttypen, sichtbare Werte/Funktionen, Capabilities,
  Contract-Pflichten und Source-Spans; unvollständiger Code darf nicht gebaut
  oder ausgeführt werden. Lücken in weiteren Deklarationskontexten und eine
  umfassendere Edit-Integration bleiben offen.
- [~] **Stufe D — Wirkungsanalyse:** Eine deterministische, quelltextbasierte
  erste Stufe von `zelyra impact --format=json` meldet Tabellen, SQL,
  Formulare, CRUD, Views, APIs, Berechtigungen, Contracts und eine
  strukturierte, deterministische `references`-Kantenliste. Eine fokussierte
  `--symbol <kind:name>`-Abfrage für direkt verbundene Referenzen ist verfügbar;
  E-Mails, Jobs, Tests und Live-Schemaänderungen bleiben anzubinden.
- [~] **Stufe E — semantische Änderungen:** Eine validierte, atomare
  `zelyra edit --format=json`-Umbenennung für deklarierte Funktionen, Typen,
  Records, Tabellen, Tableviews, Formulare, CRUDs, Views und Komponenten ist
  als versionierte Vorschau und mit ausdrücklichem `--apply` verfügbar.
  Projektlokale `.zyl`-Grenzen, Stale-Source-Fingerprints sowie vollständige
  Compilerprüfungen vor und nach der Änderung sind aktiv; umfangreichere,
  namensauflösungsbasierte Operationen bleiben offen. Umbenennungen von
  Funktionen, Typen und Records lösen Deklarationen und bekannte Referenzen
  jetzt über den AST auf, ohne überschattete lokale Bindungen zu verändern;
  Tabellenumbenennungen aktualisieren außerdem geprüfte SQL-Tabellenpositionen
  und bewahren Literale, Kommentare, Parameter und HTML; eine umfassendere
  ressourcenübergreifende Referenzauflösung bleibt offen.
- [~] **Stufe F — Contracts und Effekte:** Contracts und Capability-Prüfung
  existieren; `Database(read)`, `Database(write)` und `Email` sind mit
  Laufzeitfreigaben und Policy-Prüfungen implementiert. Ein eigener `Jobs`-
  Effekt bleibt geplant. KI darf Effekte niemals unbemerkt ergänzen.
- [ ] **Stufe G — Benchmark:** Reproduzierbaren KI-Autorenschaftsbenchmark
  etablieren, bevor vergleichende Eignungsbehauptungen veröffentlicht werden.
  Ergebnisse bleiben bis zu echten kontrollierten Versuchen leer.

## 1. Einstieg und Distribution

- [~] Einfache Standards mit optionalen, dokumentierten Projekt-Feature-
  Schaltern über `zelyra.toml`, `.env` und Prozessüberschreibungen sind
  verfügbar; umfassendere Profile und interaktive Konfiguration bleiben offen.
  Die vollständige Referenz wird in `docs/env.md` und `docs/env.en.md`
  gepflegt.

- [~] Eine Ein-Befehl-Quellcodeinstallation für Linux, Windows und macOS ist
  mit benutzerlokalen, wiederholbaren Bash-/PowerShell-Installern verfügbar;
  eine Rust-freie Release-Installation ist für veröffentlichte Linux-/Windows-
  x86_64-Assets verfügbar.
- [~] Release-Archive mit SHA-256-Prüfsummen sind für Linux und Windows x86_64
  verfügbar. Das öffentliche [0.2.0-Release][release-020] enthält festgelegte
  Linux-/Windows-Builds, byte-identische Wiederholungs-Builds, normalisierte
  Archive und SHA-256-Prüfsummen. Signierte Binärdateien und Prüfsummen für
  alle unterstützten Plattformen fehlen noch.

[release-020]: https://github.com/sf1976/zelyra/releases/tag/v0.2.0
- [~] Benutzerlokale Installations-, Prüf-, Update- und Deinstallationswege mit
  Dry-Run und sicheren Diagnosen sind verfügbar. Zusätzlich prüft
  `zelyra update [--check]` stabile GitHub-Releases und verifiziert eine
  SHA-256-Checksumme vor dem Austausch der eigenen Linux-/Windows-x86_64-
  Binärdatei; unter Windows wird der Austausch bis zum Ende des laufenden
  Prozesses vorgemerkt. Der Release-Workflow wird solche eigenständigen
  Update-Dateien mit künftigen Releases veröffentlichen; automatische Updates
  für andere Ziele bleiben nicht verfügbar.
- [~] Ein deterministischer Projektstart unterstützt MariaDB-Scaffolding,
  sichere lokale `.env`-Erzeugung direkt bei `zelyra new` und `zelyra init`,
  ausführlich kommentierte optionale Einstellungen und validierte wählbare
  Ports; bei neuen Projekten und beim Anlegen einer fehlenden `.env` durch
  Setup werden belegte Standardports automatisch durch freie Host-Ports
  ersetzt. Explizite Setup-Portwerte bleiben verbindlich und vorhandene
  `.env`-Dateien geschützt. Eine interaktive Verbindungs-Konfiguration bleibt
  offen.
- [~] Ein gemeinsamer Setup-Assistent für Konsole und Browser ist über
  `zelyra setup --database|--schema|--all` und `zelyra setup --web` verfügbar;
  erzeugte MariaDB-Compose-Projekte werden gestartet und das Anfangsschema
  angewendet; Konsolen- und Browser-Aktionen sind jetzt durch HTTP- und
  Integrationstests auf isolierten Ports abgedeckt und zeigen bei fehlendem
  Compose plattformspezifische Docker-Installationshinweise sowie erkannte
  Compose-Befehle und sichere Hinweise zu Docker-Berechtigungen,
  Gruppenaktualisierung und Zugriffsprüfung sowie Portkonflikten. Docker-
  Installation, Fernadministration und produktives Deployment bleiben bewusst
  außerhalb des Assistenten.
- [~] Die erzeugte Docker-Compose-Vorlage startet MariaDB und den internen
  Webserver mit unabhängig konfigurierbaren Web-, MariaDB- und Container-Ports.
  `tests/generated-project-docker-e2e.sh` erzeugt jetzt ein frisches CRUD-Projekt
  und führt `zelyra setup --all` zweimal aus. Geprüft werden
  schemabasierte Maschinen-/Abteilungsseiten, die ausgegebene lokale Adresse,
  geschützte `.env`-Rechte, geheimnisfreie Ausgabe, unveränderte Zugangsdaten
  beim erneuten Setup, Portzuordnungen und die Bereinigung ausschließlich des
  eindeutig benannten Compose-Projekts samt Volume. Abdeckung über diesen
  isolierten Docker-Ablauf hinaus und Produktionshärtung bleiben offen.
- [ ] Optionale automatische Reverse-Proxy-Einrichtung für Apache und Nginx
  mit sicheren Defaults und Vorschau der Konfiguration.
- [~] `zelyra doctor` prüft Projektgültigkeit, Datenbankverbindung, Docker
  Compose, eine optionale `.env` ohne Zugangsdaten auszugeben, und die
  Host-Port-Be…6710 tokens truncated…Update- oder Installationsbefehle müssen klar getrennt bleiben.
- [ ] Reproduzierbare Releases, SBOMs, Provenance-Nachweise und signierte Artefakte.
- [x] Release-Artefakte für Linux und Windows können bei relevanten Pull
  Requests und manuell ohne Veröffentlichung gebaut werden; veröffentlicht
  wird ausschließlich bei geprüften Versions-Tag-Pushes. Siehe die
  zweisprachige [Release-Anleitung](releasing.de.md).
- [ ] Synchron gehaltene deutsche und englische Dokumentation einschließlich
  Migrations- und Upgrade-Anleitungen.
- [ ] Contributing Guide, Architecture Decision Records, Code of Conduct und
  transparente Issue-Labels.
- [ ] SemVer-Regeln, Deprecation-Fenster und Kompatibilitätstests.
- [ ] Kriterien für öffentliche Alpha, Beta und Stable Releases auf Grundlage
  echter Businessanwendungen statt nur Sprachbeispielen.

## Release-Meilenstein 0.2.0

Zelyra 0.2.0 wurde am 2026-09-20 veröffentlicht. Dieser Abschnitt hält den
gelieferten Umfang und die Release-Nachweise fest; das Release bleibt
experimentell und ist nicht für den Produktiveinsatz freigegeben. Die
bestandenen Release-Kriterien bedeuten nicht, dass alle folgenden Roadmap-
Arbeiten abgeschlossen sind.

- [~] **Eigenständiges Views-System:** Wiederverwendbare Layouts/Komponenten,
  typisierte CRUD-Darstellungsoptionen und ressourcenspezifische benannte
  Layout-Slots sind verfügbar. Projektlokale deutsche/englische Kataloge
  können alle kataloggebundenen generierten Standardtexte sowie ausdrücklich
  markierte View-/Texteinträge ergänzen oder überschreiben. Parametrisierte
  Feld- und generierte Identifier-Beschriftungen werden unterstützt;
  Fachdatensätze und unmarkierte eigene Texte bleiben unverändert.
  Weitergehende Theme-Erstellung und datensatzgebundene benutzerdefinierte
  Slots bleiben offen.
- [~] **Vorzeige-Businessanwendung:** Das MariaDB-Business-Template und der
  Integrationstestpfad für erzeugte Projekte existieren. Ein datenbankfreier
  CLI-/HTTP-Test deckt jetzt das generierte CRUD-Formular, Ablehnungen ohne
  Token und ohne ausreichende API-Berechtigung, autorisierten Zugriff sowie
  projektlokale deutsche/englische Texte ab. Zusätzlich wurde der
  MariaDB-gestützte Pfad aus `tests/generated-project-business-e2e.sh` mit einer
  isolierten Testdatenbank erfolgreich ausgeführt und die Bereinigung geprüft.
  Der MariaDB-Akzeptanztest für die Maschinenverwaltung prüft jetzt die
  lokalisierten Maschinen- und Abteilungsansichten in allen vier Kombinationen
  aus Deutsch/Englisch und Learn/Work – einschließlich Anzeige, Ausblendung und
  Übersetzung der Lernhilfe. Einsteigergeprüfter Einstieg und eine umfassendere
  dokumentierte Nutzerabnahme bleiben Release-Arbeiten. Eine geschützte
  Maschinenverwaltungs-Fixture und ein Generated-Project-E2E prüfen jetzt
  zusätzlich anonyme Ablehnung, Viewer-Zugriff, Ablehnung von
  Erstellen/Bearbeiten, autorisierte CRUD-Formulare, Suche und Löschen.
- [~] **Einfacher Erststart:** zelyra new und zelyra setup, geschützte
  `.env`, Docker Compose, freie Portwahl, konkrete Docker-Berechtigungshinweise
  und die Ausgabe der App-Adresse sind vorhanden. Der erzeugte CRUD-Stack hat
  einen isolierten Erststart- und Wiederholungstest bestanden, einschließlich
  schemaabhängiger HTTP-Seiten, unveränderter Zugangsdaten, geheimnisfreier
  Setup-Ausgabe, Portzuordnungen und Bereinigung. Setup erklärt außerdem, dass
  minimale `init`-Projekte keine Datenbank benötigen, und schlägt bei einem
  fehlenden absoluten Pfad einen wahrscheinlichen relativen Pfad vor. Generierte
  Docker-Builds können einen veröffentlichten Tag, Branch oder Commit festlegen;
  die Referenz wird geholt und detached ausgecheckt. Prüfungen auf sauberen
  Hosts und Wiederherstellungsfälle über diesen Docker-Ablauf hinaus bleiben
  offen.
- [~] **Datenbanksicherheit:** MariaDB ist die Runtime-Referenz; für SQLite
  gibt es End-to-End-Pfade. Der Schema-Sicherheitstest prüft auf beiden
  Backends: destruktive Löschungen benötigen Freigabe; Pflichtspalten ohne
  Standardwert, neue Unique-Constraints sowie MariaDB-Foreign-Key-Ergänzungen
  und -Entfernungen erscheinen als `REVIEW`. Eine Pflichtspalte ohne
  Standardwert kann einer befüllten Tabelle erst nach einem lesenden Preflight
  hinzugefügt werden; bei vorhandenen Zeilen wird der gesamte Plan vor
  jeglichem SQL abgelehnt. Doppelte beziehungsweise
  verwaiste Zeilen bleiben erhalten, wenn Index- oder Foreign-Key-Änderungen
  scheitern. Unbekannte externe Indizes bleiben bestehen, nicht verfolgte
  Foreign-Key-Entfernungen werden fail-closed blockiert. Default-Drift bei
  MariaDB und PostgreSQL wird als `REVIEW` geführt; SQLite-Defaultänderungen
  sowie Primärschlüssel-/Auto-Increment-Änderungen ohne unterstützte Migration
  werden mit `E-DB-006` abgelehnt. `--allow-destructive` genehmigt
  keine `REVIEW`-Änderungen; diese benötigen nach Prüfung `--allow-risky`.
  Nullbarkeitsänderungen bei MariaDB und PostgreSQL benötigen Freigabe; vor
  einer Verschärfung prüft Zelyra vorhandene NULL-Werte, bevor irgendein SQL
  des Plans läuft. SQLite-Nullbarkeit sowie SQLite-Typ-, Foreign-Key- und
  Unique-Constraint-Änderungen werden noch nicht unterstützt. Typ- und
  Nullbarkeitsänderungen werden unabhängig bewertet. Die
  [Versionsmatrix](database-compatibility.de.md) führt vier MariaDB-
  Community-LTS-Patch-Images und die getesteten Datenbank-/CRUD-Pfade auf;
  sie ist keine MySQL-Kompatibilitäts- oder vollständige Funktionsgarantie.
  PostgreSQL-Runtime-Parität, allgemeine Daten-Backfills, Zeilen-/Lock-
  Risikoeinschätzungen und betriebliche Risikoanalyse bleiben außerhalb der
  0.2.0-Aussage. PostgreSQL-Drift bei
  Primärschlüsseln und Serial-/Identity-Eigenschaften wird erkannt und
  abgelehnt; Migrationen dieser Metadatenänderungen werden noch nicht
  unterstützt.
- [x] **Release-Nachweise:** Zweisprachige Quickstarts, internes
  Sicherheitsreview, vollständige Branch-CI, Workspace-Prüfungen, die
  MariaDB-Kompatibilitätsmatrix mit vier Versionen und datenbankgestützte
  Integrationstests sind erfolgreich. Eine frisch erzeugte MariaDB-CRUD-
  Anwendung bestand Erststart und wiederholtes Setup im isolierten Docker-E2E-
  Test. Linux- und Windows-Release-Binärdateien wurden in den festgelegten
  CI-Toolchains byte-identisch erneut gebaut; beide Release-Pakete bestanden
  die Prüfung. Das ist technische Abnahme für ein experimentelles Release,
  keine Einsteigerstudie, Windows-Ersteinstiegsstudie auf einem sauberen Host,
  Produktionsfreigabe oder externes Sicherheitsaudit; solche Aussagen werden
  nicht gemacht.

Für 0.2.0 sind weder ein visueller Drag-and-drop-Editor noch eine Anbindung an
einen KI-Anbieter, Compiler-Self-Hosting, vollständige formale Verifikation
oder PostgreSQL-Runtime-Parität erforderlich. Diese Ziele bleiben separat in
der Roadmap; geplante Fähigkeiten dürfen nicht Teil des Releaseversprechens
werden.

## Release-Meilenstein 0.3.0 (veröffentlicht)

Zelyra [`v0.3.0`](https://github.com/sf1976/zelyra/releases/tag/v0.3.0) wurde
am 03.10.2026 als experimentelles Release veröffentlicht. Der abgeschlossene
[Releaseplan](release-plans/0.3.0.de.md) dokumentiert Umfang, Prüfnachweise und
bekannte Grenzen. Der Release-Workflow baute und prüfte Linux- und
Windows-x86_64-Artefakte; der veröffentlichte Linux-Installer einschließlich
Prüfsumme, Wiederholungsinstallation und Update-Check wurde anschließend
erfolgreich smoke-getestet.

Der unabhängige menschliche Einsteigertest fand für 0.3.0 nicht statt. Der
Projektverantwortliche hat ihn ausdrücklich auf 0.4.0 verschoben. Das ist eine
Risikoentscheidung, kein Nutzertestnachweis; die menschliche Abnahme ist vor
dem finalen 0.4.0-Release verpflichtend. Siehe das
[Entscheidungsprotokoll](release-readiness/0.3.0-human-gate-decision.de.md).

Der aktuelle Datenbank-Abnahmenachweis ist im [CI-Lauf
35571692858](https://github.com/sf1976/zelyra/actions/runs/35571692858) grün:
Die MariaDB-Matrix mit vier Versionen, Schema-Sicherheitsprüfungen, der
SQLite-Regressionstest, erzeugte Anwendungen und der isolierte Kunden-/Auftrags-
Workflow waren erfolgreich.
Der Clean-Onboardingpfad mit installierter CLI ist außerdem im [CI-Lauf
35573616419](https://github.com/sf1976/zelyra/actions/runs/35573616419) grün.

## Release-Meilenstein 0.4.0 (veröffentlicht, nicht vollständig abgeschlossen)

Das stabile `v0.4.0` ist veröffentlicht und die Artefakte wurden technisch
geprüft. Der [Releaseplan 0.4.0](release-plans/0.4.0.de.md) bleibt dennoch
offen: Die unabhängige menschliche Einsteigerabnahme wurde zuerst auf 0.5.0
verschoben und ist durch die neuere Entscheidung des Projektverantwortlichen
auf 1.0.0 verschoben. Sie wurde nicht durchgeführt. Eine manuelle Rückmeldung
mit dem veröffentlichten
0.4.0-CLI deckte außerdem eine falsche absolute Pfadangabe bei `setup` sowie
unklare Erwartungen an `setup` für minimale `init`-Projekte auf. Die Diagnose
und Dokumentation wurden für die 0.5.0-Vorbereitung korrigiert und Regressionen
geprüft; die veröffentlichten 0.4.0-Artefakte enthalten diese Korrekturen
nicht. Details und Grenzen stehen im zweisprachigen 0.5.0-
[Studienprotokoll](release-readiness/0.5.0-onboarding-study.de.md). Es ist kein
Nachweis menschlicher Abnahme.

## Release-Meilenstein 0.5.0 (veröffentlicht)

Das stabile `v0.5.0` ist auf Commit `6c0c0e6` veröffentlicht. Es liefert
experimentelle Docker-Bundles für eine statisch erkannte Teilmenge,
getrennte Schemas und Datenbankkonten mit minimalen Rechten sowie einen
modularen Kunden-/Auftragsablauf. Vollständige Abhängigkeits-/Effektauflösung,
gemeinsames Schemaeigentum und ein wiederverwendbares Mehrverbindungs-
Datenbankmodul sind nicht enthalten. Die technischen Release-Gates,
Artefaktprüfungen und der Installations-/Update-/Rollback-Smoke bestanden.
Menschliche Abnahme fand nicht statt und ist auf 1.0.0 verschoben. Siehe
[Release-Notizen](release-notes/0.5.0.de.md) und
[Verifikationsnachweis](release-readiness/0.5.0-candidate-verification.md).

- [🧪] Im unveröffentlichten 0.4-Zweig erzeugt `zelyra module bundle` nun auf
  Wunsch ein commit-gepinntes Dockerfile, eine Compose-App und ein
  geheimnisfreies `.env.example`; `--dry-run` liefert außerdem eine
  deterministische JSON-Dateiliste der geplanten Ausgaben, ohne das angeforderte
  Ziel anzulegen; das temporäre Testpaket wird geprüft und entfernt. Locale-
  JSON-Dateien werden im Manifest sortiert; ein Wiederholungstest bestätigt
  byte-identische Dateien zweier Exporte desselben Bundles. Ein eigenständiger
  Container-Smoke-Test für eine importierte Route ist grün. Die
  exportierte Anwendung erhält ihre
  `DATABASE_URL` separat, MariaDB bleibt ein externer Dienst. Der Compiler
  lehnt erkannte modulübergreifende Tabellenverweise ohne erreichbaren
  Importabhängigkeitspfad mit `E-MOD-019` ab, auch für Tabellen im Einstieg;
  gemeinsam genutzte Tabellen müssen in einem importierbaren Schema-Modul
  liegen. Tabellen unterstützen außerdem explizite `access`-Listen für
  `read`, `write` und `read_write`; erkannte modulübergreifende Zugriffe ohne
  passenden Grant scheitern mit `E-MOD-021`. CRUD/Formular/Auth sowie unbekannte
  SQL-Zugriffe verlangen `read_write`. Das sind Compiler-Verträge, keine
  MariaDB-Grants; Schemaänderungsrechte und unbekannte SQL-Formen bleiben
  ungeprüft. Datenbanknutzende Module müssen den Provider außerdem direkt oder
  transitiv importieren (`E-MOD-022`); ein Import nur im Einstieg wird nicht
  vererbt. Eine Einstiegskonfiguration muss dafür in ein importierbares Modul
  verschoben werden. `context --format=json` weist Provider, DB-Verbraucher,
  direkte/transitive Bindung und Verbindungsvariable samt tatsächlichem
  `DATABASE_URL`-Fallback ohne Zugangsdaten aus. Die Laufzeit nutzt weiterhin
  eine Verbindung pro Prozess; das ist Auskunft, kein Routing. Der
  MariaDB-Projektgenerator legt die Datenbankdeklaration jetzt
  in `src/database.zyl` ab und importiert sie aus `main.zyl`; das trennt nur
  die Quelle, nicht mehrere unabhängig konfigurierbare Verbindungen. Mehrere
  Datenbankdefinitionen im Graphen scheitern mit `E-DB-001`, bevor ein Bundle
  Dateien schreibt. Das Manifest
  weist weiterhin `source_closure_complete: false` und
  `complete_deployment: false` aus. Vollständige Abhängigkeitsanalyse,
  modulare Datenbankschnittstelle und technische Kandidatenprüfung bleiben
  offen. Die menschliche Abnahme ist auf 1.0.0 verschoben.
  `module plan` verfolgt jetzt auch Funktionsaufrufe in Formular- und
  CRUD-Aktionen bis zu den aufgerufenen Deklarationen und deren Quelldateien;
  ein Mehrmodul-Integrationstest prüft beide Fälle. Das erweitert die bekannte
  Kantenmenge, beweist aber keine vollständige Abhängigkeitsanalyse.
  Tabellenspalten verweisen jetzt außerdem auf verwendete benannte Type Aliases;
  dadurch nimmt die bekannte Deklarationsschließung beim Schema-Modul auch die
  Aliasdeklaration auf. Ein Mehrmodul-Regressionstest prüft Ressourcen-Kante
  und Schließung. Typisierte Felder von Formularen, Formularaktionen und CRUD-
  Aktionen verweisen jetzt ebenfalls auf ihre benannten Type Aliases; ein
  Integrationstest prüft Formular- und CRUD-Ressourcenpläne. Auch dies bleibt
  eine statisch erkannte Teilmenge. Typannotationen in Funktionskörpern,
  Record-Konstruktoren und SQL-Ergebnistypen liefern jetzt ebenfalls Typkanten.
  Die MariaDB-Ergebnistypprüfung akzeptiert qualifizierte Record-Namen, indem
  sie vor der Zuordnung zur Tabelle den Modulpräfix entfernt; gezielte
  Tests prüfen den Resolver und den zusammengesetzten Projektpfad.
  Die Veröffentlichung des fertigen Staging-Verzeichnisses erfolgt auf Linux,
  macOS und Windows atomar ohne Ersetzen eines inzwischen angelegten
  Zielverzeichnisses; ein Regressionstest prüft sowohl den Konfliktfall als auch
  die Veröffentlichung bei freiem Ziel.
  Unabhängig davon unterstützt derselbe unveröffentlichte Zweig nun `pub view` und
  `pub component`; importierte Page-/CRUD-Layout-Verweise und erkannte
  Komponenten-Tags in Page-, View-, Komponenten- und CRUD-Slot-HTML verlangen
  eine öffentliche Deklaration und einen ausdrücklichen Importpfad
  (`E-MOD-007` / `E-MOD-020`). Dies ist ein getesteter Sichtbarkeitsschritt,
  kein vollständiges HTML-Namespace- oder Modulvertragssystem.
- [🧪] Das generierte Kunden-/Auftrags-Docker-Rehearsal sichert und restauriert
  Geschäftszeilen nun mit begrenzt berechtigten MariaDB-Konten, wendet eine
  additive Schema-Migration an und beendet danach MariaDB während einer nicht
  bestätigten Kundenänderung hart. Nach dem Neustart prüft es, dass bestätigte
  Kunden-/Auftragszeilen und die migrierte Tabelle erhalten bleiben, während
  die abgebrochene Änderung zurückgerollt wurde. Der Nachweis ist auf eine
  wegwerfbare MariaDB-11-Fixture begrenzt; vollständige Modulschließung und
  Schemaeigentumsdurchsetzung bleiben offen.

## Release 0.6.0

Das stabile [0.6.0-Release](release-plans/0.6.0.de.md) ergänzt einen
nativen, gepoolten PostgreSQL-16-Pfad für direkte SQL-Abfragen mit begrenzten
Parametertypen und Transaktionen. Vollständige PostgreSQL-Parität und
generierte Web-/CRUD-Unterstützung werden nicht versprochen. Das Release
ergänzt eine deterministische Routenübersicht samt Kollisionsfehlern sowie
`zelyra db map`: eine schreibgeschützte Zuordnung von Tabellen und
Beziehungen zum Quellmodul, die Abweichungen zur Live-Datenbank zeigt.
Modulzuordnung bleibt ein Hinweis und ändert weder Schema noch Berechtigungen.
Die menschliche Abnahme bleibt auf 1.0.0 verschoben.

## Release 0.7.0

- [🧪] **Experimentelles Release:** lokales Studio, begrenztes typisiertes
  MySQL-8.4.11-SQL, Rechnungstutorial, erstes Sprachkorpus für 0.1 und ein
  MariaDB-Mandanten-CRUD-Pfad für eine deklarierte Tabelle. Maintainer-
  Sicherheitsprüfung und Mutation-Rollback-E2E sind im 0.7-Plan dokumentiert.
  Dieser Pfad ist auf MariaDB, eine mandanteneigene Tabelle und CRUD ohne
  Beziehungen begrenzt; er ist noch nicht die für 1.0.0 zugesagte vollständige
  mandantenfähige CRUD-Unterstützung für alle Datenbanken. Es gibt keine
  Mandantenmigration oder Backfill. Unabhängiges Sicherheitsreview
  und menschliche Abnahme bleiben Gates vor Produktiveinsatz in 0.9.9/1.0.0.
  Details im [0.7.0-Plan](release-plans/0.7.0.de.md).

## Release 0.8.0

- [🗺️] Eine schreibgeschützte visuelle Datenbankkarte und einen geführten
  CRUD-Assistenten ergänzen. Entwickler wählen Verbindung und Tabelle, prüfen
  Felder und Beziehungen, sehen compilergeprüften Quelltext und übernehmen die
  Änderung ausdrücklich. Sprach-Fixtures für Grammatik, Typprüfung und Formatter
  ausbauen. Dazu kommt die bereits in Prüfung befindliche begrenzte lokale
  MariaDB-Outbox mit ausdrücklich gestartetem Worker und At-least-once-Zustellung.
  Siehe [0.8.0-Plan](release-plans/0.8.0.de.md).

## Release 0.9.0

- [x] **Stabiles Release veröffentlicht:**
  geführte Studio-Geschäftsabläufe, Rechnungspfad, vorschau-basierte lesbare
  Regex-Textbearbeitung und erweitertes Sprachkorpus sind implementiert. Die
  Datenbankaussagen bleiben auf die genaue Servermatrix und belegten Abläufe
  beschränkt. Lokales MariaDB-TOTP bleibt Vorschau bis zum unabhängigen
  Sicherheitsreview vor Produktionsfreigabe. Siehe
  [0.9.0-Plan](release-plans/0.9.0.de.md), Release-Notizen und
  [Releaseprüfnachweis](release-readiness/0.9.0-candidate-verification.md).

## Produktlinie: Sprache, Compiler, Datenbank und Studio

Zelyras zentrale Produktidee ist ein zusammenhängender Weg von verständlichem
Quelltext über Compiler und Datenbank bis zur fertigen Geschäftsanwendung.
Studio soll möglichst alle häufigen, sicher modellierbaren Aufgaben der Sprache
und des Frameworks übernehmen: Eigenschaften auswählen, Abläufe führen, gültige
Optionen erklären, Fehler zeigen und geprüften Quelltext erzeugen. Diagramme und
Präsentationen sind nur Beispiele dafür. Jede generierte Änderung bleibt als
lesbarer Zelyra-Quelltext überprüfbar; folgenreicher Datenzugriff zeigt Plan,
Auswirkungen und eine ausdrückliche Bestätigung. Die Roadmap staffelt diese Ziele über
0.9.2–0.9.8; sie sind noch keine Fähigkeiten des veröffentlichten 0.9.0.

Der Laravel-Migrator ist zugleich ein Umsetzungstreiber für Zelyra selbst.
Blockiert eine angenommene Laravel-Zuordnung durch eine fehlende Zelyra-
Sprach-, Compiler-, Datenbank-, Laufzeit- oder Studio-Funktion, wird diese
Lücke dem dafür nötigen Release zugeordnet und innerhalb von 0.9.3–0.9.8
geschlossen. Eine Zuordnung gilt erst als „direkt abbildbar“, wenn das erzeugte
Projekt den zugesagten Ablauf tatsächlich ausführt. Echte Mehrdeutigkeiten und
unsichere Geschäftslogik bleiben mit Quellbeleg und Begründung ausdrücklich
manuell umzusetzen.

## Release 0.9.1 — IDE-Grundlage

- [🧪] Studio enthält bereits Projektbaum, Tabs, sichtbaren Speicherstatus,
  Rückgängig/Wiederholen, Projektpfadbegrenzung und datei-/positionsbezogene
  Diagnosen. Umbenennen und Löschen von Quelldateien benötigen eine sichtbare
  Bestätigung und einen aktuellen Datei-Hash; vorhandene Ziele werden nie
  überschrieben, `main.zyl` und `zelyra.toml` sind geschützt. Importverweise
  werden beim Umbenennen nicht angepasst. Browser-E2E prüft jetzt Sitzungsstart
  und Neuladen, ungespeicherte Änderungen, Tastaturspeichern, Compilerdiagnosen
  und Neustart. Das Sitzungstoken wird aus der Adresszeile entfernt und für
  diese Tab-Sitzung gehalten. API- und Dateisystemprüfungen decken veraltete
  Änderungen, Pfad-Traversal, Geheimdateien, geschützte Dateien und
  ausdrückliches Anwenden ab. Zugriff bleibt auf Loopback begrenzt; der
  Compiler bleibt maßgeblich. Der getaggte Stand `v0.9.1` bestand die vollständige
  CI; Linux-/Windows-Artefakte, Prüfsummen, SPDX-SBOMs und Attestierungen wurden
  veröffentlicht. Das zweisprachige Website-Handbuch ist live; siehe den
  [0.9.1-Plan](release-plans/0.9.1.de.md).
- Nachweis: Chromium-Browserabdeckung, Editor-API-/Sicherheitsprüfungen und
  MariaDB-/PostgreSQL-Studioabläufe bestehen lokal. CI muss diese für den
  exakten Releasekandidaten wiederholen. Das Rechnungsbeispiel lässt sich öffnen
  und ausführen.
- [🧪] Das Rechnungstutorial zeigt eine Netto-/19%-Umsatzsteuer-/Brutto-
  Rechnung aus den gespeicherten Positionen und eine druckoptimierte Ansicht,
  die der Browser als PDF speichern kann. Der angenommene Standardsatz gilt
  nur für das deutsche Lernbeispiel; dies ist keine native PDF-Engine und keine
  vollständige gesetzeskonforme Rechnungsvorlage.
- [🧪] Die Sprachzeile 0.1 verwendet `#` für einzeilige Kommentare und
  alleinstehende `##`-Zeilen als nicht verschachtelte Blockkommentar-Grenzen.
  `//` bleibt für vorhandene Programme kompatibel.

## Release 0.9.2 — Sprachdienste der IDE

- [✅] Sichere UTF-8-Textdateioperationen für exklusive Erstellung, atomaren
  Dateiaustausch, Anhängen, Kopieren, Umbenennen und Result-basiertes Löschen
  ergänzen. Wörtliche Textsuche, Ersetzen und Teilen unterstützen die
  mehrteilige Konsolen-Notizverwaltung unter `examples/notes_manager` mit
  Sicherungen vor Änderungen, Wiederherstellung, Dateigrenzen, Aktionsprotokoll
  und automatisierten Regressionstests.
- [✅] `assert(condition[, message])` typgeprüft ausführen und falsche
  Bedingungen als Laufzeitfehler melden. Die Tests für Lager/Bestellungen,
  CSV-Import und Notizverwaltung verwenden Assertions; `zelyra run tests.zyl`
  liefert bei fehlgeschlagener Prüfung einen Fehlerstatus. Der Lager-Spielstand
  wird über atomaren Dateiaustausch gespeichert und im Beispiel wieder geladen.
  `text_trim` entfernt Unicode-Leerraum am Rand; der CSV-Import weist damit
  Pflichtfelder zurück, die nur aus Leerraum bestehen.

- [🧪] Der geführte Tutorialbefehl startet MariaDB und Webanwendung und wendet
  das Schema an. Die Compose-Webanwendung bezieht die Datenbank-URL aus der
  Projekteinstellung; Einsteiger müssen weder `DATABASE_URL` setzen noch
  separat `db setup` und `serve` ausführen. Ein responsives Theme gestaltet
  Dashboard, CRUD-Formulare, Listen und Druckansicht.

- [🧪] Studio zur IDE mit einer kontextabhängigen Eigenschaftsseitenleiste und
  Aufgaben-/Befehlsfinder ausbauen. Häufige Abläufe zu Dateien, Symbolen, Routen,
  Views, Formularen, Ressourcen und Diagrammen lassen sich durch Auswahl und
  gültige Eigenschaften einstellen. Beim Schreiben können Beschreibung,
  Optionen und Beispiele zum Befehl am Cursor gezielt aufgerufen werden; Nutzer
  müssen die API nicht auswendig lernen. Die aktuelle Arbeitsfassung ergänzt
  bereits Dateieigenschaften, Hilfe zu zentralen Sprachbegriffen per F1, einen
  filterbaren Aktionsfinder per Strg+Umschalt+P und compilergeprüfte Diagnosen
  während der Eingabe sowie Vorschläge für Deklarationen aus dem Compiler-
  Projektmodell und F12-Sprünge zu Definitionen aus gespeicherten Projektdateien.
  Zusätzlich verfügbar sind eine Literalsuche und lexikalische Bezeichnerfundstellen
  per Umschalt+F12 in gespeicherten `.zyl`-Dateien. Diese Fundstellen lösen keine
  Gültigkeitsbereiche auf und sind keine semantischen Referenzen. Typisierte
  Vervollständigung und symbolbezogene Eigenschaften bleiben offen. Strg+Leertaste
  ergänzt zusätzlich Parameter sowie gültige, im aktuellen Block sichtbare
  `let`-, Schleifen- und Match-Bindungen aus ungespeichertem Quelltext. Typinferenz
  bleibt offen. Der Aktionsfinder startet `main.zyl` nach ausdrücklicher Auswahl
  mit 15-Sekunden-Limit, begrenzter Ausgabe und Ersetzung erkannter Geheimnisse
  aus Prozessumgebung und `.env`. Die Ausführung bleibt ein experimenteller
  lokaler IDE-Ablauf.
- [🧪] Die Projektkarte zeigt Compilerbeziehungen für Modulimporte,
  Deklarationen, CRUD/Tabellen, Seiten/Views und eindeutig aufgelöste
  API-Handler. Quellnavigation ist enthalten. Dynamische und
  Datenbankabfrage-Beziehungen bleiben ausgelassen; mehrdeutige oder begrenzte
  Karten sind als unvollständig markiert.
- [🗺️] IDE-Sprachdienste vervollständigen: scopebewusste semantische Referenzen,
  typisierte Vorschläge, Symbol-/Routen-Eigenschaften und integrierte Verify-
  und Test-Aufgaben ergänzen. Projekt-, Modul-, Routen- und Ressourcendiagramme
  deterministisch vervollständigen. Unbekannte Beziehungen bleiben sichtbar
  unvollständig, statt erraten zu werden.
- [🗺️] Compiler-gestützte Vervollständigung, Symbolnavigation, Suche/Referenzen,
  Formatierung, handlungsorientierte und während der Eingabe aktualisierte
  Diagnosen sowie integrierte Check-, Run- und Test-Aufgaben ergänzen.
  Fortschritt und Ausgabe bleiben sichtbar und geheimnisfrei. Die Funktionen
  decken dokumentierte Sprach-0.1-Konstrukte ab. Zusätzlich entstehen aus dem
  Compiler-Projektmodell verlässliche Architekturdiagramme für Module/Importe,
  Routen und Ressourcenbeziehungen. Knoten und Kanten führen zur passenden
  Quelldatei und Diagnose zurück; unbekannte oder dynamische Beziehungen werden
  sichtbar als unvollständig markiert.
- Nachweis: Positive und negative Mehrdateiprojekte prüfen kontextuelle
  Eigenschaften, Hilfetexte, Vorschläge, Live-Fehlerpositionen, Navigation,
  idempotente Formatierung, Aufgabenausgabe und deterministische Diagrammdaten;
  unbekannte Syntax wird kenntlich gemacht statt erraten.

## Release 0.9.3 — Datenbankwerkbank

Arbeitsumfang und technische Gates stehen im [Releaseplan 0.9.3](release-plans/0.9.3.de.md).

- [✅] CSV-Grundlagen für Import und Export ergänzen: `String[][]` als
  verschachtelten Typ parsen, zitierte Trennzeichen/Quotes/Zeilenumbrüche
  korrekt decodieren und encodieren sowie Dezimaltexte ohne Float exakt in
  skalierte Integer umwandeln. Das Beispiel `examples/csv_catalog` erstellt
  Quelldaten, validiert Felder, protokolliert Fehler, sortiert/exportiert
  gültige Datensätze und prüft den Roundtrip automatisiert. CSV wird als
  vollständiger UTF-8-Text im Speicher verarbeitet, nicht gestreamt. Die
  Zelyra-Tests verwenden `assert`, damit Fehlschläge im Prozessstatus sichtbar
  werden.
- [✅] `text_chars(text)` liefert Unicode-Codepoints als `String[]`. Das
  mehrteilige Beispiel `examples/labyrinth_solver` nutzt es, um unterschiedlich
  große Raster einzulesen, zu validieren und per Breitensuche einen kürzesten
  Weg mit Anzeige, Zählern und Datei-Roundtrip zu berechnen.

- [🧪] Den im 0.9.2-Rechnungstutorial gefundenen Startfehler beheben: Das
  deklarative Weblayout muss das Projekt-Theme laden; die CLI zeigt die lokale
  Browseradresse schon beim Anlegen und behält sie auch bei fehlgeschlagenem
  Schema-Setup bei. Studios **Starten** führt Konsolenprogramme aus und startet
  bei Webprojekten eine lokale Vorschau mit anklickbarer Adresse. Der Start
  beendet eine vorherige Vorschau und Studio beendet die aktive Vorschau beim
  Schließen; **Vorschau beenden** stoppt sie sofort. Datenbanken werden weiterhin
  ausdrücklich separat gestartet. Das Tutorial wird als echtes importiertes
  Mehrdateienprojekt mit eigenem Datenbank-, Kunden-, Artikel-, Rechnungs-,
  Layout- und Dashboard-Modul erzeugt. Der CLI- und Browser-E2E-Nachweis für
  den Studio-Webablauf ist ergänzt; menschliche Abnahme bleibt für 1.0.0.
- [🧪] Die CLI-Hilfe ausbauen: `zelyra <befehl> --help` und verschachtelte
  Aufrufe wie `zelyra db plan --help` erklären Zweck, Optionen, Beispiele und
  Voraussetzungen in Deutsch oder Englisch, ohne den Befehl auszuführen.
- [🧪] Einen schreibgeschützten Laravel-Projektanalysator in der Zelyra-CLI
  beginnen: `zelyra migrate laravel <projekt> --report <pfad>`. Der erste
  statische Inventarlauf erfasst
  Laravel-/Paketversionen, Routen und Middleware, Controller, Models,
  Migrationen, Validierungen, Views, Jobs und Integrationen statisch und
  verknüpft die Funde mit Quellpositionen. Er darf die Anwendung nicht starten,
  Artisan nicht aufrufen, keine Datenbankverbindung öffnen und keine `.env`-
  Dateien lesen. Berichte unterscheiden direkt abbildbare, teilweise
  abbildbare, manuell umzusetzen und nicht sicher zuordenbare Elemente und
  nennen den nächsten sicheren Schritt. Der unterstützte Musterumfang wird
  dokumentiert; mehrdeutige Funde bleiben offen statt erraten zu werden. Die
  erste Fassung erfasst bereits Composer-Versionen, Routen, Models,
  Controller, Schema-Erstellungen, Beziehungen, Validierungsaufrufe, Queued
  Jobs, Views und ausgewählte Integrationen mit Fundstellen in Markdown oder
  JSON. Sie bearbeitet ausschließlich einen neuen Bericht außerhalb des
  Quellprojekts. Versionsmatrix, verknüpfter Projektgraph, Paket-/Regelabgleich
  und Gerüstgenerierung folgen in 0.9.4–0.9.8.
- [🧪] Die Datenbankansicht der IDE zur sicheren Werkbank ausbauen: Tabellen,
  Spalten, Indizes und deklarierte Beziehungen ansehen, Datensätze standardmäßig
  schreibgeschützt durchsuchen und den zuständigen Quellmodulen zuordnen. Das
  ER-Diagramm wird filterbar und navigierbar; Auswahl eines Feldes oder einer
  Beziehung zeigt Definition, Quellmodul und verwendende Ressourcen. Studio
  bietet nun einen sitzungsgeschützten, schreibgeschützten Datenbrowser mit
  höchstens 25 Zeilen je Abruf sowie einen compilergeprüften Assistenten zum
  Ergänzen und Bearbeiten einfacher Quellspalten. Eine deklarierte MariaDB
  lässt sich mit vorhandener Verbindung nach Bestätigung anlegen. Die Auswahl
  der Verbindung, PostgreSQL-/MySQL-Anlage und die vollständige Plan-/Risiko-
  Ansicht bleiben offen.
- [🗺️] Fremdschlüssel bis 1.0.0 als durchgängige Datenbankfunktion liefern:
  Referenzen in Zelyra-Tabellenfeldern deklarieren, Zieltabellen und Feldtypen
  im Compiler prüfen, echte Constraints anlegen und auslesen sowie Beziehungen
  in der Studio-Karte anzeigen. Die unterstützten Abläufe werden getrennt für
  MariaDB, MySQL und PostgreSQL nachgewiesen. Vor dem Hinzufügen zu bestehenden
  Daten prüft der Schema-Plan auf verwaiste Zeilen; bei Fehlern bleiben Daten
  erhalten, riskante Änderungen brauchen eine Prüfung und nicht unterstützte
  Aktionen werden fail-closed abgelehnt. Löschungen sind standardmäßig
  eingeschränkt. Kaskaden oder `SET NULL` müssen ausdrücklich deklariert,
  typgeprüft und je Backend integriert getestet werden, bevor sie als
  unterstützt gelten.
- [🗺️] Das mandanten-gefilterte generierte CRUD über den experimentellen
  MariaDB-Pfad hinaus erweitern: Die Mandantenauswahl an die aktive Mitgliedschaft
  des angemeldeten Benutzers binden, die Ressourcenberechtigung verlangen und
  Mandantenfilter in Listen, Detailansichten, Erstellen, Bearbeiten, Aktualisieren,
  Löschen und Wiederherstellen serverseitig erzwingen. Mandantenschlüssel werden
  serverseitig gebunden und sind keine normalen CRUD-Felder oder Filter. Mit
  Fremdschlüsseln müssen Auswahl und Speicherung verknüpfter Datensätze außerdem
  nachweisen, dass diese zum aktiven Mandanten gehören. Eigene SQL-Aktionen auf
  Mandantendaten müssen den Mandanten sicher berücksichtigen oder vom Compiler
  abgelehnt werden. Der Nachweis umfasst positive und negative Integrationstests
  für zwei Mandanten, alle Lese- und Änderungswege, entzogene Mitgliedschaften
  sowie manipulierte oder widersprüchliche Mandantenauswahl.
- [🧪] Tabellenspalten über die Eigenschaftsseitenleiste grafisch ergänzen und
  bearbeiten. Studio zeigt den vollständigen Schema-Plan mit Auswirkungen und
  Risiken, lässt den Compiler prüfen und verlangt vor jeder Anwendung eine
  ausdrückliche Bestätigung. Außerdem kann der Nutzer den gewählten Datenbanktyp
  und die Verbindung einstellen und — soweit Backend und Rechte es unterstützen
  — die Projektdatenbank aus Studio anlegen. Nicht unterstützte Kombinationen
  werden klar benannt; Zugangsdaten werden nie im Browser gespeichert oder
  ausgegeben. Add/Edit für einfache Quellspalten ist compilergeprüft,
  hashgebunden und ausdrücklich bestätigt. Schlüssel-, eindeutige und
  Default-Felder sind vom visuellen Edit ausgeschlossen. Datenbanktyp,
  Verbindungsdialog sowie PostgreSQL-/MySQL-Datenbankanlage sind noch nicht
  umgesetzt; MariaDB-Anlage nutzt eine bereits konfigurierte Verbindung.
- [🗺️] Lesende, typisierte Datenquellen für Kennzahlen vorbereiten, damit sie
  in 0.9.4 ohne frei formulierte SQL-Ausdrücke gewählt werden können.
- [🗺️] Wiederverwendbare JavaScript-ES-Module als Projektdateien planen:
  gemeinsame Funktionen liegen einmalig in einem klaren Asset-/Client-Bereich
  und können von mehreren Seiten importiert werden. Studio soll Import-/Export-
  Verwendungen auffindbar machen, Referenzen über Dateien hinweg zeigen und
  passende Hilfe und Fehlerstellen anbieten. Häufige Browseraufgaben können
  zusätzlich als einfache Zelyra-Bausteine bereitstehen; sie ersetzen weder
  normales JavaScript noch bestehende JS-Bibliotheken. Die genaue Einbindung,
  Asset-Ausgabe, Sicherheitsgrenzen und Interoperabilität werden vor der
  Implementierung spezifiziert.
- Nachweis: Separate MariaDB-, MySQL- und PostgreSQL-Tests entsprechen der
  veröffentlichten Datenbankmatrix. Nicht verfügbare Schemaoperationen werden
  abgewiesen; Änderungen sind gegen Vorschau-Hash und aktuellen Datenbankstand
  geschützt. Schreibgeschütztes Browsen ist der Standard.

## Release 0.9.4 — CLI-, Setup- und Tutorial-Zuverlässigkeit (abgeschlossen)

- [✅] Jeden dokumentierten obersten und verschachtelten CLI-Befehl mit
  Optionen, Hilfe, Erfolgsweg und erwarteten Fehlern prüfen. Die gemeldeten
  Setup- und Datenbankprobleme nachvollziehen, Verhalten und Erklärungen
  korrigieren und eine zweisprachige Befehlsmatrix mit Regressionstests führen.
  `db create` muss eindeutig nur SQL ausgeben; `db setup` und
  `setup <verzeichnis> --all` müssen ihre tatsächlichen Datenbank- und
  Schemaaktionen benennen.
- [✅] Fortschritt sichtbar ausgeben, während Docker Compose baut und startet.
  Den letzten Fehler beim Schema-Setup ohne Zugangsdaten offenlegen und nach
  dem Start die tatsächliche Anwendungsadresse ausgeben.
- [✅] Die geschützte `.env` des Rechnungstutorials sowohl mit als auch ohne
  `--no-start` bereitstellen. Aus Studio prüfen, dass die Vorschau die
  Projektdatenbank verwendet, und Laden, Anlegen, Weiterleitung und Liste beim
  Kundenformular durchspielen. Bei fehlgeschlagener Abfrage einen sicheren
  nächsten Schritt für Datenbank und angewendetes Schema anzeigen. Nach
  erfolgreichem Tutorialstart den Standardbrowser öffnen und die Adresse in
  Headless-Umgebungen ausgeben.
- [✅] Aus Studio eine noch fehlende MariaDB-Datenbank anlegen: nach sichtbarer
  Bestätigung die geschützten lokalen Compose-Adminzugangsdaten verwenden,
  sonst das konfigurierte Konto versuchen. Kein Host-Programm `mariadb`
  voraussetzen und keine Zugangsdaten an den Browser senden.
- Abschlussnachweis: Zweisprachige Befehls- und Hilfeabdeckung ohne fehlenden
  dokumentierten Befehl; `setup --all` gegen ein isoliertes MariaDB-Compose-
  Projekt; Studio-kompatibler Kundenablauf gegen MariaDB; vollständige
  Workspace-Tests und konsistente Release-Metadaten.

## Gestaffelte Arbeit für 0.9.5–0.9.8

Die folgenden Produktarbeiten bleiben Teil des 1.0.0-Ziels. Sie werden
aufgeteilt, damit jede Version prüfbare Kriterien erhält und Sprach- und
IDE-Rückstand nicht als eine einzige Änderung in 0.9.4 behandelt wird.

- [🧪] Typabdeckung: exklusive Bereichsschleifen, payloadfreie nominale Enums,
  Kalender-/UTC-Zeitstempeloperationen, Decimal-Quantisierung und sichere
  wertsemantische Record-/Arrayänderungen sind in 0.9.8 ergänzt. Der 0.9.9-
  Branch ergänzt benannte typisierte Funktionswerte, begrenzte generische
  Funktionen und Payload-Enums. Generische nutzerdefinierte Records/Enums
  bleiben offen; eine Geschäftsentität mit eigener Identität wird nur bei einem
  belegten Anwendungsfall benötigt. Records bleiben Werttypen; `Int`/`UInt` und `Float`/`Decimal`
  bleiben ausdrücklich getrennt. Compiler, Runtime, Diagnosen,
  Kompatibilitäts-Fixtures, Studio-Hilfe und `examples/type_system` gemeinsam
  aktualisieren. Vor dem 0.9.9-Freeze müssen positive, erwartete
  Compiler-/Runtimefehler- und Serialisierungstests ohne unerwartete Fehler
  bestehen.
- [🗺️] Den Laravel-Analyzer von Inventaren zu belegten Graphen aus Route,
  Middleware, Controller, Model/Beziehung, Migration/Tabelle und View ausbauen.
  Markdown und JSON mit Fundstellen, Paket- und Validierungsfunden, offenen
  Kanten, versionierten Zuordnungen sowie manuellen und sicherheitsrelevanten
  Prüfhinweisen ausgeben. Später eine ausdrücklich angeforderte
  Gerüstgenerierung in ein separates Projekt mit Herkunftsangaben ergänzen;
  nie Geheimnisse oder Produktionsdaten kopieren und keine automatische
  Geschäftsparität behaupten.
- [🗺️] Studio von sicheren Schema- und CRUD-Werkzeugen zu geprüfter relationaler
  CRUD-Gestaltung, Fremdschlüsselauswahl und typisierten Diagrammoptionen in der
  Eigenschaftsseitenleiste ausbauen. Danach compilergeprüfte Diagramme und
  Präsentationsexporte ergänzen. Generierte Abfragen bleiben typisiert und
  lesend; Berechtigungen und Mandantengrenzen gelten. Vor dem ausdrücklich
  bestätigten Übernehmen den gesamten Diff zeigen. Datenbankoperationen nur je
  Backend-Zusage testen; beliebige Client-Hydration bleibt ausgeschlossen.

## Release 0.9.5 — Eigene Mail- und SMTP-Unterstützung

- [x] Die bestehende Laravel-Analyse wurde zu einem versionierten Regelwerk für
  Authentifizierung, Policies/Berechtigungen, Geschäftslogik, Transaktionen,
  Geldfelder, Jobs, Integrationen, Dateiablage und Views erweitern.
  Sicherheitskritische oder verhaltensabhängige Funde müssen zur menschlichen
  Prüfung markiert werden, darunter Statuswechsel, Summen, Nebenwirkungen und
  Fehlerbehandlung. Die statische Analyse bleibt ohne PHP-Pakete und laufende
  Laravel-Umgebung nutzbar.
- [x] Eine erste explizite Mail-API ohne verpflichtenden Anbieter wurde ergänzt:
  MIME-Text/HTML, SMTP-Authentifizierung, implizites TLS, STARTTLS, private
  Zertifizierungsstellen, begrenzte Nachrichten und Anbindung an die bestehende
  MariaDB-Outbox. Der Worker stellt nur ausdrücklich zu; TLS wird nie
  stillschweigend abgestuft. Mail-Capabilities und lokale Testsenke dokumentieren.
- [ ] Sichere benannte Templates und begrenzte Datei-Anhänge in 0.9.7 zusammen
  mit empfängergebundener Benachrichtigungszustellung ergänzen; gemeinsame
  Größen-, Escape- und Herkunftsprüfungen verwenden.
- Abschlusskriterien bestanden am exakten Kandidaten: lokaler SMTP-End-to-End-Test mit vertrauenswürdiger
  lokaler CA, STARTTLS und Authentifizierung; außerdem Tests für
  Konfigurationsablehnung, Größenlimit, Header-Injection, Geheimnisredaktion,
  Capability-Prüfung und transaktionales Einreihen. Zustellung und Duplikatfenster
  zweisprachig dokumentieren. Siehe den [0.9.5-Plan](release-plans/0.9.5.de.md).

## Release 0.9.6 — Benachrichtigungszentrum

- [🗺️] Eine ausdrücklich angeforderte Gerüsterzeugung mit
  `zelyra migrate laravel <projekt> --output <ziel>` ergänzen; `--report` und
  `--output` dürfen gemeinsam verwendet werden. Nur auf Wunsch ein separates
  Zelyra-Projekt erzeugen, Quellfundstellen und IDs der Zuordnungsregeln in
  generierten Dateien erhalten und niemals Geheimnisse, `.env`-Werte,
  personenbezogene Einträge oder Produktionsdaten kopieren. Überlappende
  Quell-/Berichts-/Zielpfade und unsichere bestehende Ziele ablehnen. Der Bericht
  muss Gerüst und tatsächlich übertragene Geschäftslogik unterscheiden; jedes
  erzeugte Projekt muss mit dem passenden Zelyra-Compiler geprüft werden.
  Blockaden angenommener Zuordnungen werden als Zelyra-Umsetzungsaufgabe in
  derselben Release-Reihe geführt und behoben, bevor die Zuordnung als
  unterstützt gilt.
- [🗺️] Einen mandantengebundenen gemeinsamen In-App-Benachrichtigungsfeed mit
  ungelesener Anzahl, Seitennavigation, Ablauf-Filter, Soft-Delete-Archiv und
  bearbeitbaren Einstellungsdatensätzen ergänzen. Dieser erste Schritt behauptet
  noch keine persönlichen Lesebestätigungen, geplante Löschung nach Ablauf oder
  ereignisgesteuerte E-Mail; dafür sind empfängergebundene Zustellung und 0.9.8
  vorgesehen. Der Feed bleibt ohne SMTP nutzbar.
- Nachweis: MariaDB-Integration belegt, dass Mitglieder Nachrichten anderer
  Mandanten weder auflisten noch zählen können, archivierte und abgelaufene
  Einträge wie konfiguriert ausgeblendet werden und erratene mandantenfremde IDs
  verborgen bleiben. Lokalisierung, erzeugte Formulare und Fehlerzustände sind
  abgedeckt. Ein externer Pushdienst ist nicht erforderlich.

## Release 0.9.7 — Studio, Rechnungstutorial und Härtung

- [✅] Das Rechnungstutorial wird in `main.zyl` und Fachmodule gegliedert und
  erhält ein Desktop-Dashboard mit echten Kennzahlen sowie einen geführten
  Kunden-/Artikel-/Rechnungsablauf. Der Tabler-CSS-Bestandteil ist lokal,
  optional und MIT-lizenziert; keine CDN-Abhängigkeit. Druckausgabe mit Netto,
  19% Umsatzsteuer und Brutto bleibt Teil des Lernpfads.
- [✅] Projektkarten-Fehler beheben: Views über Modulgrenzen nur innerhalb des
  eigenen Moduls oder über importierte öffentliche Views auflösen, CRUD-Layouts
  berücksichtigen und TableViews anzeigen. Begrenzungen und offene Beziehungen
  über lokalisierte, stabile Gründe erklären; keine SQL-Beziehungen erraten.
- [✅] Responsives Browser-E2E für Desktop und schmale Ansichten, reale
  Kennzahlen, generierte CRUD-Formulare, lokale Tabler-Auslieferung ohne
  Netzwerk und druckbare Rechnung ergänzen.
- [🗺️] Persönliche Lesebestätigungen und Empfängerautorisierung,
  Aufbewahrung, sichere Templates/Anhänge und optionale SMTP-Zustellung in
  begrenztem Umfang in 0.9.8 liefern. Atomare Einreihung, idempotente
  Wiederholungen, fremde Mandanten-/Empfänger-IDs und Zustellungsduplikate
  prüfen.
- [🗺️] Die Laravel-Analyse für Mandantengrenzen, Autorisierung,
  Datei-Downloads/-Uploads, Zahlungs-/Buchhaltungsabläufe und öffentliche
  Endpunkte bis 0.9.8 härten. Fehlende und mehrdeutige Belege erfordern
  manuelle Prüfung.
- [🗺️] Die für 1.0 zugesagte MariaDB-, MySQL- und PostgreSQL-Ablaufmatrix
  schließen; genaue Serverversionen, unterstützte Abläufe und Grenzen
  dokumentieren. Beziehungen, TLS, Backups, Wiederherstellung und
  mandanten-gefiltertes CRUD mit negativen Autorisierungstests prüfen.
- Abschlusskriterien: Der Projektkarten-Regressionstest löst das importierte
  Tutorial-Layout auf, die Browserprüfung zeigt echte Kennzahlen und alle
  Befunde/Backendgrenzen sind dokumentiert. Der vollständige Umfang steht im
  [0.9.7-Plan](release-plans/0.9.7.de.md).

- [🗺️] Das Benachrichtigungszentrum um persönliche Lesebestätigungen,
  Empfängerautorisierung, begrenzte Aufbewahrung, sichere benannte Templates und
  optionale SMTP-Zustellung über die transaktionale Outbox erweitern. Atomare
  Einreihung, idempotente Wiederholungen, mandanten- und empfängerfremde
  ID-Angriffe sowie SMTP-Duplikatverhalten prüfen.
- [🗺️] Die Laravel-Analyse für Mandantengrenzen, Autorisierung,
  Datei-Downloads/-Uploads, Zahlungs-/Buchhaltungsabläufe und öffentliche
  Endpunkte härten. Soweit die unterstützte Syntax reicht, Belege über Routen,
  Middleware, Controller, Models und Beziehungen verfolgen und manipulierte
  IDs sowie Mandantenüberschreitungen markieren. Bei fehlenden oder mehrdeutigen
  statischen Belegen ist eine manuelle Prüfung erforderlich; Verhalten darf
  nicht stillschweigend ausgelassen werden. Test-Fixtures für Mandantentrennung,
  Fremdschlüssel, Transaktionen, Dezimal-Geldbeträge, Berechtigungen und
  Dateiablage ergänzen.
- [🗺️] Die für 1.0 zugesagte MariaDB-, MySQL- und PostgreSQL-Ablaufmatrix
  schließen: Schemaansicht, Migrationen, CRUD, Transaktionen, TLS, Backup und
  Restore, soweit ein Backend diese Funktion beansprucht. Unabhängige Reviews
  von Authentifizierung, MFA, Mandantentrennung, Autorisierung, CSRF,
  SMTP-Geheimnissen und Studio-Änderungsgrenzen abschließen und alle
  releaseblockierenden Befunde beheben.
- [🗺️] Die vollständige Autorisierungsmatrix für mandanten-gefiltertes CRUD
  mit den festgelegten MariaDB-, MySQL- und PostgreSQL-Versionen durchlaufen.
  Mandantenauswahl, Mitgliedschaft und Berechtigung, generierte Listen und
  Detailansichten, Erstellen/Aktualisieren/Löschen/Wiederherstellen,
  Mandantenzugehörigkeit von Beziehungen und fail-closed-Verhalten bei fehlendem
  oder entzogenem Kontext abdecken. Mandantenmodell und Migrationserwartungen
  dokumentieren; keine automatische Migration bestehender Daten versprechen.
- Nachweis: Exakte Serverversionen und Grenzen sind veröffentlicht; jeder
  unterstützte Ablauf hat CI-Nachweise, negative Autorisierungstests und eine
  maintainergeprüfte Entscheidung zu Sicherheitsbefunden. Keine universelle
  SQL-Parität behaupten.

## Release 0.9.8 — Probe der Produktionsreife

- [✅] Sprachstabilisierungsumfang umgesetzt: kontrollierte Grenzen für tiefe
  Funktionsaufrufe, vollständige Rückgabepfade, direktes `Int::MIN`,
  Kalender-/UTC-Arithmetik, Decimal-Quantisierung, payloadfreie Enums,
  exklusive Integer-Bereiche, wertsemantische Record-/Arrayänderungen,
  getaggter Result-JSON-Codec und iterative Verarbeitung langer Ausdrücke.
  Workspace, Clippy, Formatierung, 324 Handbuchbeispiele, alle 80 Korpusfälle,
  100 Wiederholungsläufe sowie 300 Integer-Division/Rest- und 100 Decimal-
  Referenztests bestanden. Die 0.9.9-Wiederholung schließt die zehn früheren
  Sprachlückenproben; vier begrenzte Beobachtungen sind in Spezifikation und
  Typmatrix ausgewiesen.
- [✅] Die helle Studio-Darstellung deckt Editor, Projektnavigation,
  Eigenschaften, Hilfe, Datenbankwerkbank, Aktionssuche und Dialoge ab.
  Browser-E2E prüft Speicherung, Tastaturbedienung, einheitliche Flächen und
  mindestens 4,5:1 Kontrast für Text und Syntaxfarben. Bei 360 CSS-Pixeln
  bleiben die Arbeitsfläche und der Theme-Schalter ohne horizontalen Überlauf
  nutzbar. Die IDE-Arbeit geht mit den grafischen relationalen und
  Diagrammabläufen für 0.9.9 weiter.
- [✅] MySQL Server 8.4.11 besteht nun den isolierten Schemaablauf über
  `db setup`, `inspect`, schreibgeschütztes `map`, geprüften `plan`, bestätigtes
  `apply` und `history`. Der CI-Job prüft außerdem parametrisierte SQL-Abfragen.
  Der Nachweis deckt kein vollständiges CRUD/Auth, MFA, TLS, Backup/Restore oder
  allgemeine MariaDB-Parität ab; die Backendmatrix nennt die Grenzen.
- [🗺️] Nach 0.9.9 übertragen: Den unterstützten Umfang des Laravel-Migrators und seine versionierte
  Kompatibilitätsmatrix abschließen. Schreibgeschützte Analyse,
  Geheimnis-/PII-Redaktion, Quellpositionsgenauigkeit, Routen-Daten-Beziehungen,
  Behandlung von Mehrdeutigkeiten, Pfadtrennung, Herkunftsverweise in
  generierten Dateien und compilergeprüfte Ausgabe an repräsentativen Laravel-
  Fixtures prüfen. Erzeugte Anwendungen mit echten CRUD-Abläufen auf den
  unterstützten Datenbanken ausführen, einschließlich unterstützter Beziehungen,
  Mandanten, Berechtigungen, Transaktionen und Geldbeträge. Jede Zelyra-Funktion,
  die eine angenommene Zuordnung blockiert, vor dem Abschluss von 0.9.9
  implementieren. Unterstützte Laravel-/Paketmuster und Grenzen dokumentieren.
  Keine automatische Geschäftslogik- oder Sicherheitsparität behaupten; das
  Werkzeug liefert einen überprüfbaren Migrationsstartpunkt.
- [🗺️] Nach 0.9.9 übertragen: Saubere Installation, Upgrade, Rollback, Migration, Backup und Restore
  auf unterstützten Betriebssystemen und Datenbanken proben. Betreiber- und
  Nutzerdokumentation, Lizenz-/Abhängigkeitsreview, Ressourcenlimits,
  Geheimnisredaktion, reproduzierbare Artefakte, SBOM/Provenance und Offline-
  Abläufe abschließen. Präsentationen und Diagramme auf sauberen Umgebungen,
  barrierearme Lesbarkeit, stabile Exporte und Geheimnisfreiheit prüfen. Start,
  übliche CRUD-Abläufe und begrenzten Ressourcenbedarf messen und Ergebnisse
  ohne unbelegte Leistungsversprechen veröffentlichen.
- [🗺️] Verbindliche 0.9.8-Übernahmen in 0.9.9 schließen: saubere Umgebungs-
  und Datenbankproben, Sicherheits-/Abhängigkeitsentscheidungen, integrierter
  Studio-/Tutorial-/Mail-/Benachrichtigungsablauf, Laravel-Migrator-E2E sowie
  Diagramm- und Präsentationsnachweise. Siehe [0.9.8-Plan](release-plans/0.9.8.de.md)
  und [0.9.9-Plan](release-plans/0.9.9.de.md).

## Release 0.9.9 — Feature-Freeze und letzter technischer Kandidat

- [✅] Typisierte Decimal-Tabellenansicht zeigt exakte Werte ohne zusätzliche
  Nullauffüllung; normale Texte und Rohdaten behalten ihre Darstellung.

- [✅] Setup-Abhängigkeit nach Präzisionshärtung korrigieren: MariaDB zuerst,
  Schema im separaten Containerlauf, Webdienst erst nach Erfolg starten;
  Migrationsfreigaben bleiben wirksam.

- [✅] Dezimalpaket: feste SQL-Skala, verlustfreier JSON-Transport, exakte
  Formulare/Filter und PostgreSQL-NUMERIC-Dekodierung; Alias- und Grenzwerte,
  Zwischenüberlauf und kontrollierte Ressourcenfehler werden geprüft.
  Siehe [Regeln](decimals.de.md) und [Paketprotokoll](release-readiness/0.9.9-exact-decimals.de.md).
- [✅] Kalender-/UTC-Rechnung, typisierte Formular-/HTTP-Eingaben und native
  SQL-Date-/Time-/Timestamp-Parameter/Speicherung bestehen auf MariaDB, MySQL
  und PostgreSQL; UTC-Sitzungen und exakte Zeitpräzision werden geprüft.
  Siehe [Regeln](dates.de.md) und [SQL-Nachweise](release-readiness/0.9.9-calendar-sql.de.md).
  Die SMTP-/Reset-Fixture berücksichtigt unterdrückte ersetzte Zustelljobs
  und prüft vor dem Fortsetzen das tatsächlich gespeicherte Ersatztoken.
  Business-Readiness-Prüfungen beachten den nötigen Metadatenzugriff vor dem
  Start samt kontrolliertem, redigiertem Fehler bei unerreichbarer Datenbank.
- [✅] Explizites `sql_decode<T>(sql<…> { … })` erzeugt Skalar-/Record-/Array-
  Werte mit geprüfter Kardinalität, Projektion, nativen Typfamilien und NULL-
  Masken. Exakte Zahlen und Kalenderfolgeoperationen bestehen auf drei realen
  Backends; bestehendes Rohzeilen-HTTP-/JSON bleibt erhalten. Siehe
  [Regeln](sql-results.de.md) und [Nachweise](release-readiness/0.9.9-typed-sql-results.de.md).

- [✅] Edit-Snapshots erhalten native NULL-Masken, erkennen veraltete NULL-/
  Literaltext-Wechsel vor dem Schreiben und zeigen NULL getrennt von `\N`.
  Fehlende implizit nullable Felder binden SQL-NULL; Pflicht-/Readonly-Regeln bleiben.
  MariaDB-/MySQL-HTTP- und fehlerhafte Metadatenregressionen bestehen. Siehe
  [Regeln](edit-snapshots.de.md) und [Nachweise](release-readiness/0.9.9-null-edit-snapshots.de.md).

- [✅] Native Entscheidungstransaktions-Grundlage deckt PostgreSQL/MariaDB/MySQL
  auf einer Verbindung, Rollback/Unwind, ignorierte SQL-Fehler und Request-/Commit-
  Deadlines ab. [Regeln](decision-transactions.de.md), [Nachweise](release-readiness/0.9.9-decision-transactions.de.md).

- [✅] PostgreSQL-Formularentscheidungen führen Mandantenmitgliedschaft,
  Ressourcenberechtigung, gesperrte Zeilenschnappschüsse (NULL vs. `\N`),
  Geschäftsänderung und atomare unverkettete Audits innerhalb derselben nativen
  Transaktion und Verbindung aus. Manipulierte Mandanten-/Fremdschlüssel,
  Mitgliedschaftsentzug während Lock-Wait (Zwei-Verbindungs-Rennen) und Audit-
  Fehler rollen atomar zurück. Verkettetes PostgreSQL-Audit bleibt diagnostiziert
  verweigert. [Nachweise](release-readiness/0.9.9-postgres-form-decisions.de.md).

- [✅] Datenbankmatrix, Transaktionen und relationales CRUD auf MariaDB 11.4.13,
  MySQL 8.4.11 und PostgreSQL 16.15: Setup/Inspektion echter Fremdschlüssel,
  Preflight-Waisenabweisung (`error[E-DB-005]`) mit Datenerhalt, gehärtete
  MySQL-Schreibsperren (`innodb_lock_wait_timeout`), mandantenisolierte
  Auswahllisten, Manipulationsschutz (422), relationale Join-Darstellung
  und restriktiver Löschschutz mit geordnetem Löschzyklus.
  Siehe [Nachweise](release-readiness/0.9.9-relational-crud-matrix.de.md).

- [🗺️] Sprach-, CLI-, Runtime-, Datenbank-, IDE-, Mail-, Benachrichtigungs- und
  Tutorialumfang einfrieren. Alle angenommenen Anforderungen aus 0.9.1–0.9.8
  umsetzen und die vollständigen technischen Gates an einem unveränderlichen
  Kandidaten ausführen. Materialien für die exakte 1.0.0-Menschenstudie
  vorbereiten. Danach sollte nur noch die unabhängige menschliche Abnahme offen
  sein; für 0.9.9 keine Abnahme und keine Produktivfreigabe behaupten.
- [✅] Sprachprüfung gegen den 0.9.9-Quellbuild: 80 Fälle ohne unerwartete
  Ergebnisse, 20 Aufgaben, 35 kontrollierte Negativfälle, 100 Wiederholungen
  und 400 unabhängige Zahlenvergleiche bestanden. Die vier Beobachtungen und
  der Linux-only-Umfang sind im [Prüfbericht](release-readiness/0.9.9-language-review.de.md)
  festgehalten. Nach dem Clippy-Kompatibilitätsfix in `1bf09ea` wurde die
  Prüfung mit Compiler-SHA `89b880d753e12ac09b2fbb4dc754586d18c7a808845b991b331cb1ae00153c0d`
  wiederholt.
- [🧪] Rust-Pakettests, Formatierung und Clippy sowie Installer-, sauberes
  MariaDB-Onboarding und generierte Docker-/Modul-Bundles bestehen auf `1bf09ea`
  lokal. CI und Paketierung bestehen auf dem Dokumentationsnachfolger `8db093c`;
  folgende Quellcodepakete und der endgültige Kandidat brauchen ihre eigene CI.
  Siehe das
  [Prüfprotokoll](release-readiness/0.9.9-technical-rehearsal-2026-10-12.de.md).
- [🗺️] Verbleibende CLI-, Setup-, Datenbank-, Migrator-, Sicherheits-,
  Dokumentations- und Paketierungsgates schließen. Allgemeine Constraints/
  Traits, Closures und generische Funktionsreferenzen bleiben ausdrücklich
  dokumentierte Grenzen der Sprachlinie. Der
  [0.9.9-Plan](release-plans/0.9.9.de.md) enthält die prüfbaren Kriterien.
- [✅] Eine optionale farbige CLI-Hilfe mit `ZELYRA_COLOR`, `NO_COLOR` und
  `CLICOLOR_FORCE` sowie einen kurzen Studio-Startbildschirm mit gespeicherter
  Darstellung und Reduced-Motion-Beachtung ergänzen und prüfen.
- [✅] Generische Funktionen, Records und Enums sind im 0.9.9-Branch umgesetzt:
  Typargumente werden abgeleitet oder ausdrücklich angegeben, auch in
  verschachtelten Records, Arrays, `Option` und `Result`. Konfliktierende,
  fehlende und falsch gezählte Typen haben Regressionstests. Das
  Type-System-Beispiel deckt generische Modulimporte sowie positive und
  negative Fälle ab. SQL-Spalten lehnen generische Records ausdrücklich ab.
  Payload-Enums, benannte typisierte Funktionswerte und indirekte Aufrufe
  bestehen ebenfalls gezielte Tests einschließlich Capability-Weitergabe.
  Allgemeine Constraints/Traits, generische Aliase, Closures und generische
  Funktionsreferenzen bleiben außerhalb des unterstützten Umfangs.
- [🧪] Studio-Diagramme unterstützen nun feste Aggregationen, exakte
  Textfilter, inklusive `DATE`-Zeiträume, sichere Parameterbindung, Speichern
  und erneutes Laden der Konfiguration sowie lokale Präsentations-/PDF-
  Vorschauen. MariaDB- und PostgreSQL-Browser-/Datenbank-E2E bestehen.
  Gespeicherte SQL-Ansichten, Einbindung in Anwendungsseiten, Compilerprüfung
  der Berichtsdateien sowie vollständige Barrierefreiheits- und Exportnachweise
  bleiben Releasekriterien.
- [✅] Persistente MariaDB-Reset-Limits, begrenzte Bereinigung/Kapazität,
  bereinigte Betreiberfehler und Reset-Speicherabhängigkeiten bestehen positive
  und negative Prüfungen. Erzeugte SQL-Namen halten Backend-Grenzen bei
  Erstellung und Planung ein; zu lange Deklarationen scheitern vor SQL.
  Rust-, MariaDB-SMTP-/Limit- und PostgreSQL-Schemaprüfungen stehen im
  [Paketprotokoll](release-readiness/0.9.9-reset-and-schema.de.md).
  Unabhängige Sicherheitsprüfung bleibt offen.
- [🗺️] Visuelle Formular- und Ausgabeeditoren mit Eigenschaftsleiste für
  Felder, Prüfung, Datenbank-/Tabellenbindung, Spaltenauswahl, Filter,
  Sortierung, Suche, Pagination und Diagramme ergänzen. Editierbaren
  Zelyra-Code und lokale Styles mit Compiler-Vorschau und Schutz manueller
  Änderungen erzeugen.
- [✅] Ein begrenztes, Zelyra-gerechtes Klassenmodell bietet nun typisierte
  Konstruktoren, geteilte Referenzidentität, Identitätsgleichheit, Feldzugriffe
  und -updates, generische Klassen sowie optionale Felder. Runtimeprüfungen
  verhindern Referenzzyklen und parallele Mutation; JSON-/API-/SQL-Grenzen
  lehnen Klassenreferenzen ab. Verhalten wird mit gewöhnlichen Funktionen und
  ausdrücklich übergebenem Empfänger geschrieben. Methoden-, Closure- und
  Vererbungssyntax bleiben außerhalb des 0.9.9-Umfangs.
- Abschlusskriterien: Keine offenen Implementierungs-, Sicherheits-, Datenbank-,
  Migrations-, Paketierungs- oder Dokumentationsblocker. Bleibt einer offen, ist
  0.9.9 nicht fertig. Siehe den [0.9.9-Plan](release-plans/0.9.9.de.md).

Eine allgemeine Queue-Plattform bleibt hinter 1.0.0. Die Outbox in 0.8 ist
lokal und auf MariaDB begrenzt; sie bietet weder Scheduler noch verteilte
Worker oder Exactly-once-Zustellung. Vollständige SQL-Parität, beliebige
Client-Hydration, Compiler-Self-Hosting, natives Backend, verpflichtende
Cloud-/KI-Dienste, Telemetrie und unbelegte Leistungsversprechen bleiben
ausgeschlossen.

Die detaillierten Abnahmekriterien stehen in den Plänen für [0.8.0](release-plans/0.8.0.de.md),
[0.9.0](release-plans/0.9.0.de.md), [0.9.5 Mail](release-plans/0.9.5.de.md),
[0.9.8 Stabilisierung](release-plans/0.9.8.de.md),
[0.9.9 Freeze](release-plans/0.9.9.de.md) und [1.0.0](release-plans/1.0.0.de.md).
## Erste produktive Veröffentlichung 1.0.0

`v1.0.0` ist die erste produktive Veröffentlichung und das verpflichtende Ziel für
alle unabhängigen menschlichen Einsteiger- und Abnahmestudien, die aus 0.3.0,
0.4.0 und 0.5.0 verschoben wurden. Kein früheres Release darf behaupten, diese
Studien seien abgeschlossen. Vor dem stabilen `v1.0.0` muss nach bestandenen
technischen Gates ein unveränderlicher Release Candidate veröffentlicht werden.
Unabhängige Personen ohne Entwicklerrolle prüfen den Kandidaten und die
mitgelieferte Dokumentation. Blocker werden behoben und erneut geprüft;
redigierte Beobachtungen und die Entscheidung des Projektverantwortlichen
werden dem exakt getesteten Commit zugeordnet. Der Studienumfang muss zum
endgültigen 1.0.0-Produkt passen. Automatisierte CI, interne Rehearsals und
Rückmeldungen des Projektverantwortlichen ersetzen keinen Teilnehmertest.

## Zukünftige Sprach-, Datenbank- und Ergonomie-Evolution (nach 1.0.0)

- [x] **1.1.0 Studio und eigenständiger LSP (veröffentlicht):** Die geprüfte
  Zusammenfassung steht unter **Aktuelle Meilensteine**, in den zweisprachigen
  [Release-Notizen](release-notes/1.1.0.de.md) und im ausführlichen
  [Arbeitsprotokoll](release-plans/1.1.0-studio-deep-native-progress.de.md).
  Das Protokoll hält getestete Grenzen von semantischem Rename,
  Vorschau-Isolation, Analyseabbruch, Bundlegröße, Browserverhalten und
  Release-Artefakten fest.

Um die Produktivität, Flexibilität und Entwicklerergonomie zu steigern, ohne die Grundsäulen von Zelyra (statische
Sicherheitsverifikation von Mandanten-/Empfängergrenzen, deterministische Geschäftsarithmetik,
digitale Souveränität) aufzuweichen, sind folgende Erweiterungen für die post-1.0-Entwicklung fest eingeplant:

- [ ] **Modulare Datenbank-Adapter-Architektur (`DatabaseAdapter` SPI / Traits):**
  Ablösung der fest verdrahteten backend-spezifischen Verzweigungen im Core durch eine erweiterbare
  Adapter-Schnittstelle. Jeder Treiber-Adapter kapselt Introspektion, Schemamigration (`db apply`),
  Datentyp-Mapping, Transaktionslebenszyklus, Sperrmechanismen (`FOR UPDATE`, Timeout- und Deadline-Steuerung)
  sowie dialektspezifische SQL-Codegenerierung. Dies ermöglicht das saubere Hinzufügen weiterer relationaler
  und spezialisierter Datenbanksysteme (z. B. Microsoft SQL Server, Oracle, CockroachDB) über modulare
  Adapter, ohne den Compilerkern oder das Web-Framework umschreiben zu müssen.
- [ ] **Multi-Datenbank-Integration (Mehrere Datenquellen / Polyglot Persistence):**
  Unterstützung für die Deklaration und parallele Verwaltung mehrerer unabhängiger Datenquellen
  innerhalb eines Zelyra-Projekts (z. B. primäre relationale OLTP-Datenbank für transaktionale Mandanten-
  und CRUD-Daten, getrennte Read-Replicas oder separate Datenbanken für Analytics, Berichte, Archivierung
  oder Drittsystem-Synchronisation). Deklarative Bindung von Schemas, Modulen und Queries an benannte
  Datenbankadapter (`database "secondary" { ... }`) unter voller Beibehaltung der statischen
  Mandanten-Isolation und Capability-Prüfung.
- [ ] **Einheitlicher nativer Query-AST:** Ablösung verbliebener String-Quoting-Pfade in
  der Codeerzeugung durch einen internen Abfrage-Syntaxbaum, der dialektneutral und typgeprüft
  auf MariaDB, MySQL, PostgreSQL und künftige Adapter zielt.
- [ ] **Capture-lose Lambdas (`|x| expr`):** Syntaktische Unterstützung für kompakte,
  anonyme Transformationsfunktionen ohne äußere Umgebungserfassung. Erfordert weder Heap-Allokation
  noch Garbage Collection und bewahrt das deterministische Speichermodell.
- [ ] **Headless Hintergrund-Tasks (`task Name { ... }`):** Dedizierte Syntax und Laufzeit-
  Unterstützung für periodische Aufgaben, Datenpipelines und Hintergrundjobs unter Beibehaltung
  derselben statischen Tenant- und Datenbank-Capability-Grenzen, jedoch ohne HTTP-Web-Zwang.
- [ ] **Aktionsfähige Compiler-Diagnosen („Did you mean?“):** Erweiterung von Typ- und
  Schemamismatch-Diagnosen (`E-SQL`, `E-CRUD`) um präzise, quelltextbezogene Korrekturvorschläge
  zur Steigerung der Entwicklerergonomie bei unveränderter Typprüfungsstrenge.

## Akzeptanzanwendungen aus der Praxis

- [~] Die Maschinenverwaltung ist ein MariaDB-Template mit lokalisierten
  Maschinen-/Bereichs-Views, Suche, Kategorie-/Status-/Bereichsfiltern und
  einer optionalen SQL-Fixture mit 30 fiktionalen Datensätzen. Der
  MariaDB-Integrationstest importiert die Fixture zweimal und prüft die Daten
  über HTTP; die echte Anwendung wird außerdem in allen Deutsch/Englisch- und
  Learn/Work-Kombinationen geprüft. Einsteigergeprüfter Erststart auf sauberen
  Systemen und Produktionshärtung bleiben offen. Native Seed-/Fixture-Befehle
  bleiben optionale Roadmap-Arbeit.
- [~] Kunden-/Auftrags-Akzeptanzanwendung mit Joins, Aggregaten, CRUD-Formularen
  und eigener Projekt-Shell ist enthalten und besteht den wiederverwendbaren
  MariaDB-Tableview-E2E-Pfad; Berechtigungen und der saubere CI-Nachweis für
  die primäre Anwendung bleiben offen.
- [ ] Mehrbenutzer-Inventar mit Transaktionen und parallelen Änderungen.
- [~] Ein erzeugtes Docker-Compose-Deployment mit unabhängig konfigurierbarem
  Web-Port ist getestet; Produktionshärtung bleibt offen.
- [~] Rust-freie Self-Hosted-Installation ist für veröffentlichte Linux- und
  Windows-x86_64-Assets verfügbar; weitere Plattformen bleiben offen.

Diese Datei wird bei jeder Statusänderung eines Meilensteins und bei jeder
neuen verpflichtenden oder optionalen Architekturentscheidung aktualisiert.
