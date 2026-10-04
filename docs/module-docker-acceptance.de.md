# Rechnungen und Inventar getrennt per Docker prüfen

🧪 Entwicklungsstand für 0.4.0, keine allgemeine Exportgarantie.
[English](module-docker-acceptance.en.md)

✅ Diese begrenzte Probe bestand lokal am 04.10.2026 einschließlich aller
unten genannten Negativtests. Beide Exportimages nutzten Compiler-Commit
`999eb978b7e8dcd23303437f80e210cd3caf5ce1`.

Die Referenzprüfung erzeugt eine Gesamtanwendung mit zwei Fachmodulen und
einem gemeinsamen Datenbankmodul. Anschließend exportiert sie beide Fachmodule
und startet sie nacheinander als eigene Docker-Anwendungen. Der ursprüngliche
Webserver wird vorher gestoppt. MariaDB bleibt ein externer Dienst.

## Was wird getrennt?

| Datei | Aufgabe | Rechnungsexport | Inventarexport |
| --- | --- | --- | --- |
| `main.zyl` | Einstieg der Gesamtanwendung | neu erzeugter Einstieg | neu erzeugter Einstieg |
| `src/database.zyl` | gemeinsame MariaDB-Deklaration | enthalten | enthalten |
| `src/invoices.zyl` | Rechnungstabelle und CRUD | enthalten | ausgeschlossen |
| `src/inventory.zyl` | Inventartabelle und CRUD | ausgeschlossen | enthalten |

Beide Fachmodule importieren `src/database.zyl`. Die Deklaration `database main`
legt den Namen der Laufzeitvariable `ZELYRA_DATABASE_MAIN_URL` fest. Jeder
exportierte Prozess erhält seinen eigenen Wert. Das ermöglicht unterschiedliche
Datenbanken je Anwendung; mehrere Verbindungen innerhalb desselben Prozesses
sind damit nicht implementiert.

## Den vollständigen Test ausführen

Voraussetzungen sind ein Checkout dieses Entwicklungsstands, Rust/Cargo,
Docker mit Compose, Bash und curl. Docker muss erreichbar sein. Die lokalen
Ports 18082, 18083, 18084 und 3309 müssen frei sein. Der Test baut Compilerimages
aus GitHub und benötigt dafür Netzwerkzugriff sowie mehrere Minuten Buildzeit.
Er greift nicht auf eine bestehende MariaDB-Installation zu.

Im Repository-Stamm:

```bash
cargo build --locked -p zelyra-cli
ZELYRA_DOCKER_E2E_REF=feature/0.4-project-modules \
ZELYRA_DOCKER_E2E_MODULE_COMMIT=999eb978b7e8dcd23303437f80e210cd3caf5ce1 \
bash tests/generated-project-docker-e2e.sh
```

Der Commit pinnt den Compiler für beide Exporte. Die Gesamtanwendung verwendet
in dieser Entwicklungsprobe den angegebenen Branch. Für eine Releaseabnahme
müssen beide auf den veröffentlichten Kandidaten festgelegt werden.

Das Skript erstellt ein temporäres Projekt, eigene Compose-Projektnamen und
ein entbehrliches MariaDB-Volume. Es erzeugt Testdaten und ausschließlich für
diese Fixture bestimmte Zugangsdaten. Beim Beenden entfernt es seine Container,
sein Volume und das temporäre Projekt; erzeugte Dockerimages bleiben im Cache.
Ein abgebrochener Test ist kein erfolgreiches Ergebnis.

## Was der Ablauf überprüft

1. Die Gesamtanwendung liefert Rechnungs- und Inventardaten über HTTP.
2. `module bundle ... crud:Invoice --docker --compiler-ref ...` erzeugt das
   Rechnungspaket; analog entsteht `crud:Inventory`. Der Datenbankquelltext
   ist jeweils enthalten, das andere Fachmodul nicht.
3. Ohne die erforderliche `.env` verweigert Compose die Konfiguration mit
   einem Hinweis auf die fehlende Datei. Dies prüft noch nicht jeden möglichen
   ungültigen oder leeren Verbindungswert.
4. Beide Pakete bauen und starten mit getrennten Zugangsdaten und getrennten
   Schemas auf demselben Test-MariaDB-Server. Ihre Listen zeigen den jeweils
   erwarteten Datensatz, während der ursprüngliche Webserver gestoppt ist.
5. Die Route des anderen Fachmoduls liefert HTTP 404. Im Image liegt keine
   `/app/.env`; Compose injiziert die Konfiguration erst zur Laufzeit.
6. MariaDB verweigert dem jeweiligen Konto sowohl das Lesen der fremden Tabelle
   als auch ein `DELETE ... WHERE 1=0` auf der eigenen Tabelle. Der zweite
   Versuch ändert auch bei einem fehlerhaften Grant keine Daten, prüft aber
   die Schreibberechtigung. Nur eine echte Berechtigungsdiagnose zählt als
   erfolgreicher Negativtest, kein beliebiger Verbindungsfehler.

## Grenzen und Fehlerbehebung

Die Konten dieser Probe besitzen absichtlich nur `SELECT`. Sie belegen die
Leseisolation, keine vollständige schreibende CRUD-Abnahme. Compilerverträge
und MariaDB-Grants sind zwei getrennte Sicherheitsgrenzen. Die Probe prüft
keine Rollenoberfläche, Schemaupdates zwischen Versionen oder vollständige
Extraktion beliebiger Projekte. `complete_deployment` bleibt `false`.

Bei einem belegten Port zuerst den eigenen Testprozess prüfen; keine fremden
Dienste stoppen. `docker compose ps` im jeweiligen Projekt zeigt dessen
Container. Fehlende Dockerrechte, Netzwerkfehler beim Compilerbuild und ein
falscher Compiler-Commit müssen behoben werden, bevor das Ergebnis bewertet
wird. Keine `.env` oder vollständigen Verbindungsstrings in Fehlerberichte
kopieren. TLS ist nur im isolierten Testnetz ausdrücklich deaktiviert; ein
normaler Export verwendet `ZELYRA_DB_TLS_MODE=auto`.

Weiter: [0.4.0-Abnahme](release-plans/0.4.0.de.md),
[0.5.0-Modulziel](release-plans/0.5.0.de.md),
[Handbuch](handbook/de/handbuch.md).
