# Zelyra: `.env` und Konfiguration

Diese Datei ist die zentrale Referenz für umgebungsabhängige Einstellungen
und optionale Projekt-Schalter. Sie muss aktualisiert werden, wenn eine neue
Zelyra-Einstellung hinzukommt oder sich eine bestehende Einstellung ändert.

## Grundregel

Der einfache Einstieg benötigt keine zusätzliche Konfiguration.

- `zelyra.toml` enthält dauerhafte Projektentscheidungen.
- `.env` enthält lokale, umgebungsabhängige Werte und Secrets.
- Die Prozessumgebung kann Werte für einen einzelnen Aufruf überschreiben.
- `.env` darf niemals committed werden.

Für die bekannten optionalen Feature-Schalter gilt diese Reihenfolge:

```text
Prozessumgebung → .env → zelyra.toml → sichere Standardwerte
```

Secrets werden von `zelyra config` niemals ausgegeben.

## Feature-Schalter

Feature-Schalter sind optional. Ohne `[features]` bleiben die aktuell
unterstützten Oberflächen aktiviert, damit bestehende Projekte unverändert
funktionieren.

```toml
[features]
web = true
api = true
crud = true
auth = true
audit = true
```

| Schalter | `.env` / Prozessvariable | Standard | Bedeutung |
|---|---|---:|---|
| `web` | `ZELYRA_FEATURE_WEB` | `true` | Seiten, Formulare und Web-Ressourcen |
| `api` | `ZELYRA_FEATURE_API` | `true` | `api`-Deklarationen und API-Oberfläche |
| `crud` | `ZELYRA_FEATURE_CRUD` | `true` | `crud`-Deklarationen und generierte CRUD-Oberfläche |
| `auth` | `ZELYRA_FEATURE_AUTH` | `true` | `auth`-Deklarationen und Authentifizierungsoberfläche |
| `audit` | `ZELYRA_FEATURE_AUDIT` | `true` | Audit-Konfiguration innerhalb der Authentifizierung |

Beispiel für eine einfache lokale Variante:

```dotenv
ZELYRA_FEATURE_API=false
ZELYRA_FEATURE_CRUD=false
```

Wenn der Quellcode einen deaktivierten Bereich verwendet, meldet der Compiler
`E-FEATURE-001`. Feature-Schalter können niemals Typprüfung, SQL-Prüfung,
Capabilities, Contracts, CSRF-Schutz oder andere Sicherheitsregeln abschalten.

Die wirksame Konfiguration ist lesbar, aber nicht schreibend prüfbar:

```bash
zelyra config main.zyl
zelyra config main.zyl --format=json
```

Die JSON-Ausgabe enthält nur Schalter, Herkunft und die Information, dass
Secrets nicht angezeigt werden.

## Generierte MariaDB-/Docker-Projekte

`zelyra new --mariadb`, `zelyra init --mariadb` und die MariaDB-Templates
erzeugen `.env.example` sowie direkt eine lokale geschützte `.env` mit
zufällig erzeugten Passwörtern. `zelyra setup` bleibt als idempotenter
Nachholbefehl für bestehende MariaDB-Projekte verfügbar.

Die erzeugte `.env` aktiviert nur die für Compose und lokale
Datenbankbefehle notwendigen Werte. Der gewählte MariaDB-Host-Port bleibt
aktiv, damit `DATABASE_URL` und Compose denselben Port verwenden.
Web-Portüberschreibungen, Feature-Schalter,
Auth- und sonstige Optionen stehen ausführlich auskommentiert in der Datei.
`.env.example` enthält dieselbe Struktur, aber die Platzhalter
`change-me` und die aktivierten Secrets sind nur als Vorlage gedacht.

| Variable | Standard im generierten Projekt | Verwendung |
|---|---:|---|
| `ZELYRA_WEB_PORT` | `3000` | Port des internen Webservers im Container |
| `ZELYRA_HOST_PORT` | `3000` | lokal veröffentlichter Webport |
| `ZELYRA_DB_HOST_PORT` | `3306` | lokal veröffentlichter MariaDB-Port |
| `DATABASE_URL` | projektabhängig | Datenbankverbindung für CLI/RUNTIME; Secret enthalten möglich |
| `MARIADB_DATABASE` | `zelyra_app` | Compose: Datenbankname |
| `MARIADB_USER` | `zelyra` | Compose: Anwendungsbenutzer |
| `MARIADB_PASSWORD` | zufällig durch `zelyra new`/`init` oder `setup` | Compose: Passwort des Anwendungsbenutzers |
| `MARIADB_ROOT_PASSWORD` | zufällig durch `zelyra new`/`init` oder `setup` | Compose: MariaDB-Root-Passwort |

## Oberflächensprache und Lernmodus

| Variable | Werte | Standard in neuer MariaDB-`.env` | `serve`-Fallback | Sicherheit / Wirkung |
|---|---|---:|---:|---|
| `ZELYRA_LANGUAGE` | `de`, `en` | `de` | `en` | Kein Secret; wählt den UI-Sprachkatalog |
| `ZELYRA_LEVEL` | `learn`, `work` | `learn` | `work` | Kein Secret und keine Berechtigung; `learn` zeigt die Lernhilfe, `work` blendet sie aus |

Für `zelyra serve` gilt die Reihenfolge Prozessumgebung, dann die `.env` des
Projektverzeichnisses und anschließend der Serve-Fallback. Der Server liest aus `.env`
ausschließlich diese beiden UI-Einstellungen automatisch; andere Werte wie
`DATABASE_URL` müssen weiterhin explizit exportiert oder durch Compose injiziert
werden. Werte müssen exakt `de`/`en` beziehungsweise `learn`/`work` lauten.
Ungültige Werte führen zu einem Konfigurationsfehler, statt stillschweigend
ignoriert zu werden.

Betroffene Befehle und Laufzeitbereiche: `zelyra serve`, die MariaDB-Projekt-
Scaffolds `new`/`init` sowie der erzeugte Compose-Webdienst. Regressionstests
prüfen die Priorität Prozessumgebung → `.env` → Fallback im CLI, die gültigen
Werte und Katalogschlüssel in `web/src/i18n.rs` sowie die erzeugten Defaults
und Compose-Weitergabe in `cli/tests/machine_interfaces.rs`. Projektkataloge
werden zusätzlich auf UTF-8, JSON-Form, Größenlimit und Symlinks geprüft; ein
CLI-Integrationstest kontrolliert die Verwendung der Marker und die sichere
Fehlerausgabe.

Neue MariaDB-Projekte aktivieren `ZELYRA_LANGUAGE=de` und
`ZELYRA_LEVEL=learn`; beide Werte können in `.env` geändert werden. Die
Compose-Vorlage reicht die Variablen an den Webdienst weiter. Die mit Zelyra
gelieferten Übersetzungskataloge liegen in `web/locales/de.json` und
`web/locales/en.json`. Darin befinden sich die von Zelyra bereitgestellten
Oberflächentexte, unter anderem CRUD-, Formular-, Authentifizierungs-, Fehler-
und Lernhilfetexte. Beispiel-Views referenzieren Einträge mit
`data-zelyra-i18n="app.home_title"`; konfigurierbare Zelyra-Texte können
`@i18n:app.home_title` verwenden. Unbekannte deutsche Einträge fallen auf den
englischen Katalog zurück. Ein auch dort unbekannter Schlüssel erscheint als
`[missing translation]` und weist auf einen fehlenden Katalogeintrag hin.

`zelyra new` und `zelyra init` erzeugen zusätzlich optionale Projektkataloge
`locales/de.json` und `locales/en.json`; das erzeugte Dockerfile übernimmt den
Ordner ebenfalls in das Laufzeitimage. Darin können eigene UI-Schlüssel
hinzugefügt und Texte überschrieben werden, auf die eine View mit
`data-zelyra-i18n="eigener.schluessel"` oder eine Zelyra-Texteinstellung mit
`@i18n:eigener.schluessel` verweist. Dieselben Kataloge ergänzen oder
überschreiben alle kataloggebundenen generierten Beschriftungen in
Anwendungsrahmen, CRUD, Formularen, Tableviews, Login,
Authentifizierungsverwaltung, Validierung und Lernhilfe. Generierte
Feldnamen verwenden Schlüssel nach dem Muster `identifier.<feld>`;
parametrisierte Texte können `{field}` und `{max}` enthalten. Für Deutsch gilt
die Auflösung: Projektkatalog Deutsch → Projektkatalog Englisch → eingebauter
deutscher Katalog → dessen englischer Fallback. Die Dateien sind optionale
UTF-8-JSON-Objekte mit nichtleeren Zeichenketten; pro Datei gelten maximal
256 KiB. Ungültige Dateien, Symlinks und andere Dateitypen werden von
`zelyra serve` mit `E-I18N-001` abgewiesen. Katalogtexte werden beim Einfügen in
HTML escaped. Sie sind Anzeigeinhalt, keine Konfiguration für Berechtigungen
oder Geschäftsregeln; Zugangsdaten und andere Secrets gehören nicht hinein.

Diese Kataloge übersetzen keine fachlichen Datensätze oder beliebige HTML-Texte
aus einem Projekt. Maschinelle API-/JSON-Verträge und Compilerdiagnosen bleiben
sprachneutral beziehungsweise in ihrer festgelegten technischen Sprache und
werden nicht anhand der UI-Einstellung verändert.

## Host-Allowlist des Webservers

| Variable | Werte | Standard in neuer MariaDB-`.env` / Fallback | Vorrang | Sicherheitsklasse und Wirkung | Betroffene Befehle und Tests |
|---|---|---|---|---|---|
| `ZELYRA_ALLOWED_HOSTS` | Kommagetrennte ASCII-Hostnamen oder IP-Adressen (IDNs als Punycode); ohne Schema, Port, Wildcard oder leere Listeneinträge | `localhost,127.0.0.1,[::1]` | Prozessumgebung → Projekt-`.env` → Loopback-Fallback | Kein Secret, aber sicherheitsrelevante Allowlist gegen manipulierte `Host`-Header und DNS-Rebinding. Nur tatsächlich verwendete Hosts ergänzen. | `zelyra serve`, MariaDB-Scaffolds `new`/`init`, erzeugtes Compose; CLI-, Web- und Scaffold-Regressionstests |

Jede HTTP-Anfrage mit `Host` muss zu einem Eintrag passen. Der Vergleich ist
ohne Beachtung der Groß-/Kleinschreibung; ein angehängter Port wird separat
geprüft und nicht mit der Allowlist abgeglichen. Wildcards, URLs, Ports und
ungültige Hostnamen sind als Konfigurationswerte unzulässig. Eine leere oder
ungültige Liste verhindert den Serverstart mit `E-ENV-001`. Internationalisierte
Domainnamen müssen als ASCII-Punycode eingetragen werden. Wenn eine
Anwendung über einen Reverse-Proxy oder im LAN mit einem eigenen Hostnamen
erreichbar sein soll, diesen Host ausdrücklich konfigurieren. Der Proxy muss
den öffentlichen `Host` erhalten, `X-Forwarded-Proto` überschreiben und den
direkten Zugriff auf den App-Port verhindern. Die Einstellung erweitert nur
die Host-Allowlist; sie deaktiviert weder CSRF- noch Origin-Prüfungen.

## Nur für den Integrationstest

| Variable | Standard | Vorrang / Herkunft | Sicherheitsklasse | Betroffene Befehle und Tests |
|---|---|---|---|---|
| `ZELYRA_SCHEMA_SAFETY_MARIADB_URL` | nicht gesetzt; nur SQLite-Test | Nur Prozessumgebung; wird nicht aus Projekt-`.env` geladen. Ein expliziter Wert schaltet den zusätzlichen MariaDB-Testpfad ein. | Kann Benutzername und Passwort enthalten; nur lokale Testdatenbank verwenden, niemals ausgeben oder committen. | `bash tests/schema-safety-e2e.sh`; GitHub Actions setzt eine lokale `zelyra_ci`-Test-URL. Der Test akzeptiert ausschließlich `localhost`/Loopback und eine Basisdatenbank `zelyra_ci` oder `zelyra_test`; er erstellt und entfernt eine isolierte Datenbank. |

## Projektlokales Theme (keine `.env`-Variable)

`zelyra new` und `zelyra init` erzeugen eine optionale
`zelyra.theme.css`. `zelyra serve` lädt diese UTF-8-CSS-Datei aus demselben
Verzeichnis wie die angegebene `.zyl`-Datei und bindet sie nach dem eingebauten
Design ein. Fehlt die Datei, bleibt das Standarddesign unverändert. Es gibt
keine Theme-Auswahlvariable und keine zusätzliche Konfigurationspriorität.

Die Datei kann die folgenden öffentlichen CSS-Variablen überschreiben:

```css
:root {
    --zelyra-color-accent: #7557f6;
    --zelyra-color-accent-strong: #665ce9;
    --zelyra-color-accent-text: #634ce0;
    --zelyra-color-accent-soft: #f8f6ff;
    --zelyra-color-ink: #172033;
    --zelyra-color-muted: #738097;
    --zelyra-color-border: #e8edf4;
    --zelyra-color-canvas: #f5f7fb;
    --zelyra-color-surface: #ffffff;
    --zelyra-color-surface-subtle: #f9faff;
    --zelyra-color-sidebar-start: #171c32;
    --zelyra-color-sidebar-middle: #202743;
    --zelyra-color-sidebar-end: #263958;
    --zelyra-color-sidebar-foreground: #f6f7ff;
    --zelyra-color-sidebar-muted: #bac4d8;
    --zelyra-color-hero-start: #262f52;
    --zelyra-color-hero-middle: #3e4381;
    --zelyra-color-hero-end: #6258bb;
    --zelyra-color-success-background: #effbf7;
    --zelyra-color-success-border: #bcebdd;
    --zelyra-color-success-ink: #17654f;
    --zelyra-color-danger-background: #fff5f5;
    --zelyra-color-danger-border: #f2c8cc;
    --zelyra-color-danger-ink: #8b303c;
    --zelyra-color-focus: #8f7aff;
    --zelyra-font-body: Inter, system-ui, sans-serif;
    --zelyra-radius-card: 16px;
    --zelyra-radius-control: 10px;
    --zelyra-content-max-width: 1180px;
}
```

Alle Tokens sind optional; nicht gesetzte Werte behalten ihren eingebauten
Standard. Die CSS-Datei ist ein öffentlicher Browser-Asset, kein Secret. Lege
dort niemals Passwörter, Tokens oder vertrauliche Kommentare ab. CSS kann
Browseranfragen auslösen, etwa über `@import` oder `url(...)`; verwende solche
externen Referenzen nur bewusst. Zelyra akzeptiert nur reguläre, nicht
symbolische Dateien bis 128 KiB mit UTF-8-Inhalt. Die Datei wird unverändert
unter `/__zelyra/theme.css` ausgeliefert; diese Route ist bei vorhandener
Theme-Datei reserviert. Der Zugriff ist auf GET beschränkt und erhält
`X-Content-Type-Options: nosniff` sowie `Cache-Control: no-cache`.

Betroffen sind `zelyra serve`, `new`/`init` und der generierte Docker-Build.
Die Dockerfile-Vorlage kopiert die Theme-Datei explizit ins Image. CLI-Tests
prüfen fehlende und gültige Dateien, Größenlimit, UTF-8, Symlink-Ablehnung,
generierte Scaffold-Datei und reservierte Route; Webtests prüfen Einbindung,
MIME-Typ, Cache-Regel und Zugriffsmethode. Es wurde keine neue
Umgebungsvariable hinzugefügt.

`ZELYRA_REF` im generierten Dockerfile ist ein Docker-`ARG` mit einem
veröffentlichten Tag, keine von Zelyra geladene `.env`-Variable. Es kann beim
Docker-Build ausdrücklich über `--build-arg ZELYRA_REF=...` gesetzt werden.

Ports können unabhängig geändert werden:

```dotenv
ZELYRA_WEB_PORT=3000
ZELYRA_HOST_PORT=18080
ZELYRA_DB_HOST_PORT=3308
```

`zelyra new`, `zelyra init` und `zelyra setup` wählen freie veröffentlichte
Host-Ports, wenn sie eine neue `.env` erzeugen und ein Standardport belegt ist.
`setup` akzeptiert `--host-port` und `--db-host-port` für verbindliche Werte.
Eine vorhandene `.env` wird nie verändert; bei einem laufenden Projekt muss
sie ausdrücklich von Hand angepasst werden.

Danach:

```bash
docker compose --env-file .env -f docker-compose.mariadb.yml up -d --build
```

`DATABASE_URL` wird vom normalen `run`, `serve` und den `db`-/`auth`-/`audit`-
Befehlen aus der Prozessumgebung gelesen. Die CLI lädt `.env` dafür nicht
allgemein automatisch; entweder Compose injiziert die Variable oder sie wird
vor dem Aufruf exportiert. `zelyra doctor --env-file <datei>` liest für seine
read-only Prüfung gezielt `DATABASE_URL` aus der angegebenen Datei.

## Laufzeit und Authentifizierung

| Variable | Verwendung | Sicherheit |
|---|---|---|
| `ZELYRA_AUTH_TOKEN` | optionaler lokaler Bearer-Token für geschützte Requests | Secret, niemals committen |
| `ZELYRA_AUTH_PERMISSIONS` | durch Kommas getrennte lokale Permission-Allowlist | keine Passwörter, trotzdem nicht unnötig loggen |
| `ZELYRA_MODE` | Beispiel für `env("ZELYRA_MODE")` bei aktivierter `Environment`-Capability | Inhalt ist anwendungsspezifisch |

Die Anwendung darf mit `env(...)` nur auf Umgebungswerte zugreifen, wenn die
entsprechende Capability gewährt wurde. `DATABASE_URL` und Auth-Secrets
gehören nicht in Zelyra-Quellcode, JSON-Diagnosen, Kontextausgaben oder Logs.

## CLI- und Testvariablen

| Variable | Status | Verwendung |
|---|---|---|
| `ZELYRA_INSTALL_ROOT` | implementiert | benutzerbezogenes Ziel der Installationsskripte |
| `ZELYRA_MARIADB_ROOT_PASSWORD` | Test-/Entwicklungswerkzeug | Passwort für lokale MariaDB-Testläufe |
| `ZELYRA_MARIADB_PASSWORD` | Test-/Entwicklungswerkzeug | Benutzerpasswort für lokale MariaDB-Testläufe |
| `ZELYRA_GENERATED_E2E_ROOT_PASSWORD` | Test-/Entwicklungswerkzeug | Root-Passwort des generierten Docker-E2E-Tests |
| `ZELYRA_BIN` | Test-/Entwicklungswerkzeug | alternatives zu testendes CLI-Binary |
| `ZELYRA_E2E_PROJECT`, `ZELYRA_E2E_ADDRESS`, `ZELYRA_E2E_DB_PASSWORD` | Test-/Entwicklungswerkzeug | Projekt, HTTP-Adresse und separates Testpasswort für MariaDB-E2E |
| `ZELYRA_GENERATED_E2E_TEMPLATE`, `ZELYRA_GENERATED_E2E_HOST_PORT`, `ZELYRA_GENERATED_E2E_GENERATED_DB_HOST_PORT`, `ZELYRA_GENERATED_E2E_ADDRESS`, `ZELYRA_GENERATED_E2E_DB_HOST`, `ZELYRA_GENERATED_E2E_DB_PORT` | Test-/Entwicklungswerkzeug | erzeugtes CRUD-Projekt und seine Testports/-adresse |
| `ZELYRA_GENERATED_AUTH_ROOT_PASSWORD`, `ZELYRA_GENERATED_AUTH_DB_HOST`, `ZELYRA_GENERATED_AUTH_DB_PORT`, `ZELYRA_GENERATED_AUTH_GENERATED_DB_HOST_PORT`, `ZELYRA_GENERATED_AUTH_ADDRESS` | Test-/Entwicklungswerkzeug | erzeugtes Auth-Projekt und seine Testports/-adresse |
| `ZELYRA_AUTH_E2E_PROJECT`, `ZELYRA_AUTH_E2E_ADDRESS`, `ZELYRA_AUTH_DB_PASSWORD` | Test-/Entwicklungswerkzeug | Auth-E2E-Projekt, Adresse und separates Testpasswort |
| `ZELYRA_GENERATED_BUSINESS_ROOT_PASSWORD`, `ZELYRA_GENERATED_BUSINESS_DB_HOST`, `ZELYRA_GENERATED_BUSINESS_DB_PORT`, `ZELYRA_GENERATED_BUSINESS_GENERATED_DB_HOST_PORT`, `ZELYRA_GENERATED_BUSINESS_ADDRESS` | Test-/Entwicklungswerkzeug | erzeugtes Business-Projekt und seine Testports/-adresse |
| `ZELYRA_PROTECTED_E2E_PROJECT`, `ZELYRA_PROTECTED_E2E_ADDRESS`, `ZELYRA_PROTECTED_E2E_DB_PASSWORD` | Test-/Entwicklungswerkzeug | geschütztes CRUD-/Auth-E2E-Projekt |
| `ZELYRA_TABLEVIEW_E2E_PROJECT`, `ZELYRA_TABLEVIEW_E2E_ADDRESS`, `ZELYRA_TABLEVIEW_E2E_DB_PASSWORD`, `ZELYRA_TABLEVIEW_E2E_KEEP_TEMP` | Test-/Entwicklungswerkzeug | Tableview-E2E-Projekt, Adresse, Testpasswort und temporäre Daten |
| `ZELYRA_AUDIT_CHAIN_E2E_PROJECT`, `ZELYRA_AUDIT_CHAIN_E2E_DB_PASSWORD` | Test-/Entwicklungswerkzeug | Audit-Chain-E2E-Projekt und Testpasswort |
| `ZELYRA_DOCKER_E2E_WEB_PORT`, `ZELYRA_DOCKER_E2E_HOST_PORT`, `ZELYRA_DOCKER_E2E_DB_HOST_PORT`, `ZELYRA_DOCKER_E2E_ADDRESS` | Test-/Entwicklungswerkzeug | erzeugtes Docker-E2E-Projekt und Portwahl |
| `ZELYRA_DOCKER_E2E_REF` | Test-/Entwicklungswerkzeug; Standard ist der lokale Checkout-Branch (bei detached HEAD der veröffentlichte Zelyra-Tag) | Git-Branch oder Tag für das isolierte Docker-E2E-Anwendungsimage |

`ZELYRA_DOCKER_E2E_REF` betrifft ausschließlich
`tests/generated-project-docker-e2e.sh`. Ist die Variable nicht gesetzt,
verwendet der Test den Branch des lokalen Checkouts; bei detached HEAD gilt der
im erzeugten Dockerfile festgelegte veröffentlichte Tag. Ein gesetzter
Prozess-Umgebungswert hat Vorrang; `.env` wird dafür nicht ausgewertet. Der
Wert ist kein Secret, wird auf einen einfachen Git-Ref beschränkt und ohne
wiederverwendete Compiler-Build-Schichten gebaut. Der Branch oder Tag muss im
GitHub-Repository verfügbar sein. CI setzt den Branch des geprüften Commits.
Der Test deckt Ref-Auswahl, Build und Laufzeit ab.

Die Testvariablen mit `GENERATED_*`, `*_E2E_*` und `ZELYRA_BIN` sind keine
öffentliche Anwendungsschnittstelle. Sie dienen reproduzierbaren CI- und
lokalen Integrationstests und dürfen keine Produktionszugänge enthalten.

Testvariablen sind keine Anwendungskonfiguration. Sie dürfen nur in lokalen
Testumgebungen verwendet und nie in Beispieldateien mit echten Werten
gespeichert werden.

## `zelyra.toml`: verwandte nicht geheime Einstellungen

Nicht jede Projektoption gehört in `.env`. Die derzeit unterstützten
Konfigurationsbereiche sind:

```toml
[capabilities]
database = true
network = false
clock = true
environment = true

[filesystem]
read_roots = ["."]
write_roots = ["data"]

[network]
allowed_hosts = ["api.example.test"]
timeout_ms = 5000
max_response_bytes = 1048576

[process]
allowed_commands = ["git"]
timeout_ms = 5000
max_output_bytes = 1048576

[web]
allowed_origins = ["http://localhost:5173"]
allow_credentials = false
```

Diese Bereiche sind keine bequeme Umgehung von Sicherheit: Capabilities,
Allowlisten, SQL-Prüfungen, CSRF und destruktive Datenbankfreigaben bleiben
explizit und werden nicht durch `.env` abgeschaltet.

## Pflegepflicht für neue Einstellungen

Jede neue Umgebungsvariable oder jeder neue Projekt-Schalter muss vor dem
Commit in dieser Datei ergänzt werden. Der Eintrag muss mindestens enthalten:

1. Name und Schreibweise
2. Ablageort (`.env`, Prozessumgebung oder `zelyra.toml`)
3. Standardwert
4. Priorität gegenüber anderen Quellen
5. Zweck und betroffene CLI-/Runtime-Befehle
6. Secret-Einstufung
7. Sicherheitswirkung und Tests

Eine Einstellung gilt nicht als vollständig dokumentiert, wenn sie nur im
Compiler oder in einem Template auftaucht.
