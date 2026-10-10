# Rechnungen und Inventar getrennt per Docker prüfen

🧪 Experimentelle Modulexport-Rehearsal für 0.5.0; keine allgemeine Exportgarantie.
[English](module-docker-acceptance.en.md)

✅ Die letzte begrenzte Probe bestand lokal am 10.10.2026 mit Vorbereitungsstand
`749bde28d8e5f7624efa082734fdde4f504893da`, einschließlich schreibendem CRUD
in beiden Exporten und aller unten genannten Negativtests. Außerdem prüfte sie
Laufzeitblocker im Bundle-Manifest und den generierten Einstieg. Die erzeugte
Gesamtanwendung und die Compiler beider Exportimages nutzten den
veröffentlichten Commit `507c29e95084a59029d6b436bd69d9132c3a8937`
(`v0.4.0`). Eine frühere Probe am selben Tag bestand mit Quellstand
`c7718d5774b3ed10ca0ff1984a545acf35c6341f`; die erste Probe bestand am
09.10.2026 mit Commit `917e8c707e49332b323e51e4dcbd39a4f37ced96`.

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
ZELYRA_DOCKER_E2E_REF=v0.4.0 \
ZELYRA_DOCKER_E2E_MODULE_COMMIT=507c29e95084a59029d6b436bd69d9132c3a8937 \
bash tests/generated-project-docker-e2e.sh
```

Die Befehle pinnen den Runtime-Compiler und den Compiler in beiden Exportimages
auf den veröffentlichten `v0.4.0`-Commit. So lässt sich die Probe unverändert
wiederholen. Der aktuelle 0.5-Branch wird zum Generieren der Anwendung und
Exportpakete verwendet; eine 0.5.0-Releaseabnahme muss beide Compiler-Refs auf
den exakten Kandidaten setzen.

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
   erwarteten Datensatz, während der ursprüngliche Webserver gestoppt ist. Der
   Test legt über jedes generierte HTTP-CRUD-Formular Datensätze an, ändert und
   löscht sie und prüft CSRF- sowie Same-Origin-Schutz.
5. Die Route des anderen Fachmoduls liefert HTTP 404. Im Image liegt keine
   `/app/.env`; Compose injiziert die Konfiguration erst zur Laufzeit.
6. Jedes Testkonto besitzt `SELECT`, `INSERT`, `UPDATE` und `DELETE` nur für
   seine eigene Tabelle. MariaDB verweigert das Lesen und Löschen in der Tabelle
   des anderen Moduls. Nur eine echte Berechtigungsdiagnose zählt als
   erfolgreicher Negativtest, kein beliebiger Verbindungsfehler.

## Grenzen und Fehlerbehebung

Die Probe belegt schreibendes CRUD für diese beiden begrenzten Exporte; sie
belegt keine Rollenoberfläche, Schemaupdates zwischen Versionen oder
vollständige Extraktion beliebiger Projekte. Compilerverträge und
MariaDB-Grants sind zwei getrennte Sicherheitsgrenzen.
`complete_deployment` bleibt `false`.

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
