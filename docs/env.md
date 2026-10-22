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
aktiv, damit `ZELYRA_DATABASE_MAIN_URL` und Compose denselben Port verwenden.
Optionale Konfigurationen für Web-Ports, Features, Authentifizierung, MFA,
E-Mail und Datenbank-TLS stehen als auskommentierte Beispiele in `.env` und
`.env.example`. Erst nach bewusstem Entfernen von `#` werden sie wirksam.
Der generierte Compose-Dienst reicht die dafür unterstützten Einstellungen an
die Anwendung weiter. Testvariablen und anwendungseigene, beliebige Werte aus
`env(...)` werden nicht als Zelyra-Standardoptionen vorgetäuscht.
`.env.example` enthält dieselbe Struktur, aber die Platzhalter
`change-me` und die aktivierten Secrets sind nur als Vorlage gedacht.
Am Ende der generierten Datei stehen außerdem auskommentierte Befehle, um
deklarierte Commands und Zeitpläne zu prüfen, eine Cron-Vorschau zu erzeugen
und sie nach Prüfung manuell mit `crontab -e` einzutragen. Die Vorschau
installiert keinen Job; eine eingerichtete Systemplanung bleibt eine bewusste
Betreiberaktion. Details stehen in [Commands und Zeitplänen](tasks.de.md).

| Variable | Standard im generierten Projekt | Verwendung |
|---|---:|---|
| `ZELYRA_WEB_PORT` | `3000` | Port des internen Webservers im Container |
| `ZELYRA_HOST_PORT` | `3000` | lokal veröffentlichter Webport |
| `ZELYRA_DB_HOST_PORT` | `3306` | lokal veröffentlichter MariaDB-Port |
| `ZELYRA_DATABASE_MAIN_URL` | projektabhängig | bevorzugte Datenbankverbindung der Deklaration `database main`; Secret enthalten möglich |
| `DATABASE_URL` | projektabhängig | rückwärtskompatibler Fallback für die Datenbankverbindung; Secret enthalten möglich |
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

## Optionale KI-Vorschläge im Zelyra Studio

Die IDE kann einen OpenAI-kompatiblen Chat-Completions-Endpunkt für manuell
angeforderte Zelyra-Codevorschläge verwenden. Ohne Endpunkt und Modell bleibt
die Funktion ausgeschaltet. Es gibt keine automatische Übertragung, keinen
Hintergrundaufruf und keine automatische Übernahme vorgeschlagener Änderungen.
Studio zeigt Zielhost, Modell und den exakten ausgewählten Quelltext vor dem
Senden an. Nur diese Auswahl und die eingegebene Anweisung werden übertragen;
der vollständige Projektinhalt wird nicht mitgesendet. Die Auswahl ist auf
16 KiB, die Anweisung auf 2 KiB begrenzt. Der Anbieter erhält den markierten
Quelltext nach dem ausdrücklichen Klick auf **Auswahl an Anbieter senden**.

| Variable | Standard | Klassifizierung und Verwendung |
|---|---|---|
| `ZELYRA_STUDIO_AI_ENDPOINT` | nicht gesetzt | Kein Secret. OpenAI-kompatible URL, die auf `/chat/completions` endet; HTTPS erforderlich, HTTP nur für Loopback. |
| `ZELYRA_STUDIO_AI_MODEL` | nicht gesetzt | Kein Secret. Modellbezeichner des konfigurierten Endpunkts. |
| `ZELYRA_STUDIO_AI_API_KEY` | nicht gesetzt | Secret. Optionaler Bearer-Schlüssel; in `.env` oder Prozessumgebung ablegen und niemals einchecken. |

Die Reihenfolge ist Prozessumgebung → `.env` im Projektverzeichnis. Es gibt
keine `zelyra.toml`-Einstellung und keinen Anbieterstandard. Konfiguriere
Endpunkt und Modell gemeinsam, um die Funktion freizuschalten; den Schlüssel
nur, wenn der gewählte Anbieter ihn verlangt. Betroffener Befehl:
`zelyra editor`. Die Studio-API spricht den konfigurierten Endpunkt nur nach
dem Klick auf die Sende-Schaltfläche an, verwendet keine Weiterleitungen und
zeigt keine Zugangsdaten in der Oberfläche an. Fehlerantworten des Anbieters
werden nicht an den Browser durchgereicht.

Die Optionen sind für lokale Anbieter wie Ollama oder einen selbst betriebenen
kompatiblen Dienst gedacht; dabei kann Loopback-HTTP verwendet werden. Ein
öffentlicher Dienst erfordert HTTPS. Tests prüfen URL-Regeln, Begrenzungen,
Payload-Inhalt und dass nur der explizit ausgewählte Quelltext im Vorschlag
landet. Compiler und Tests bleiben maßgeblich; KI-Ausgaben sind nicht
vertrauenswürdig und werden nie automatisch in den Editor eingesetzt.

## Anwendungs-E-Mail ab 0.9.5

Die Zelyra-Mail-API verwendet dieselben SMTP-Variablen wie die
Passwortwiederherstellung. Zusätzlich muss `[capabilities].email = true`
gesetzt sein. Der SMTP-Host muss exakt in `[network].allowed_hosts` stehen.
Die Mail-Outbox unterstützt in 0.9.5 MariaDB und wird separat mit
`zelyra outbox setup main.zyl` eingerichtet.

| Variable | Standard | Verwendung und Sicherheit |
|---|---:|---|
| `ZELYRA_SMTP_HOST` | keiner | Erforderlicher SMTP-Host für die Zustellung. |
| `ZELYRA_SMTP_PORT` | `465` / `587` / `25` | Port für `implicit_tls` / `starttls` / `local_plaintext`; Klartext ist nur am Loopback ohne Authentifizierung erlaubt. |
| `ZELYRA_SMTP_SECURITY` | `implicit_tls` | `implicit_tls`, `starttls` oder `local_plaintext`; TLS-Fehler führen nie zu Klartext. |
| `ZELYRA_SMTP_FROM` | keiner | Erforderliche Absenderadresse. |
| `ZELYRA_SMTP_USERNAME` | keiner | Optionaler SMTP-Benutzer; muss mit dem Passwort zusammen gesetzt werden. Secret. |
| `ZELYRA_SMTP_PASSWORD` | keiner | SMTP-Passwort. Secret; niemals ausgeben oder einchecken. |
| `ZELYRA_SMTP_CA_CERT` | keiner | Optionales PEM-Wurzelzertifikat, relativ zum Projekt oder absolut; reguläre Datei ohne Symlink bis 128 KiB. Kein Secret; Hostnamen werden weiter geprüft. |

Einrichten und ausdrückliches Zustellen:

```sh
zelyra outbox setup main.zyl
zelyra outbox run main.zyl --handler zelyra.mail.send=deliver_mail --once
```

`mail_queue` schreibt die Nachricht innerhalb einer Datenbanktransaktion in
die Outbox. Der Worker stellt sie später über SMTP zu. Die Zustellung erfolgt
mindestens einmal und kann Duplikate erzeugen. Konfiguration und vollständiges
Beispiel stehen in [`mail.de.md`](mail.de.md).

## Passwortwiederherstellung

Eine Auth-Deklaration mit `reset_tokens: password_resets` muss auch
`reset_rate_limits: password_reset_limits` angeben. Die Reset-Tabelle braucht
`user_id`, einen eindeutigen `token_hash`, `expires_at`, `consumed_at`,
`delivery_payload: String(2048)`, die nullable Spalte
`delivery_retry_at: Timestamp` und einen Fremdschlüssel auf `users.id`;
außerdem muss eine Auth-Audit-Tabelle konfiguriert sein. Die separate Tabelle
für Reset-Limits braucht `bucket_key: String(64) primary`,
`window_started: Timestamp required`, `request_count: Int required`, `blocked_until: Timestamp?`
und einen Index auf `window_started`. Ein vollständiges Beispiel steht in
[`examples/password_reset.zyl`](../examples/password_reset.zyl).

Die Limits gelten pro TCP-Peer-IP und gemeinsam für alle Instanzen, die dieselbe
MariaDB, identische Limits und denselben Zustellschlüssel verwenden. Bei
Datenbankfehlern wird der Reset-Versuch abgelehnt. Das Betreiberprotokoll meldet
die bereinigte Diagnose `E-AUTH-043`; öffentliche Antworten bleiben generisch.
Die Tabelle speichert keine IP-Adresse: `bucket_key`
ist ein HMAC-Fingerabdruck mit dem Zustellschlüssel. Die Tabelle nimmt höchstens
4.096 Einträge an; bei voller Tabelle werden neue Clients abgewiesen. Ein
MariaDB-Advisory-Lock koordiniert Kapazität und Zähler aller Instanzen mit
höchstens einer Sekunde Wartezeit. Bei weiteren Anfragen werden jeweils
höchstens 100 abgelaufene Einträge entfernt, deren Fensterbeginn mehr als
48 Stunden zurückliegt. Ohne weitere Anfragen gibt es keinen automatischen
Bereinigungslauf. Forwarded-IP-Header werden ignoriert; hinter einem Proxy
teilen sich dessen Clients daher ein Limit. Das ist ein
Anwendungsschutz für Reset-Endpunkte und kein Netzwerkschutz gegen verteilte
Überlastungsangriffe. Passwortwiederherstellung läuft derzeit nur mit MariaDB.

`reset_rate_limits` hat keinen Standardwert und keinen Umgebungs-Override;
die Auth-Deklaration ist maßgeblich. Die Tabelle enthält sicherheitsrelevanten
Zustand. `check` und `serve` prüfen das Schema, `db setup` legt deklarierte
Tabellen an. Bestehende Reset-Projekte müssen die Auth-Option und Tabelle
ergänzen und einen geprüften additiven Schema-Plan anwenden. Die Diagnosen
`E-AUTH-039` bis `E-AUTH-042` benennen fehlende, inkompatible oder verwaiste
Konfiguration. Parser-, Schema-, Modulrechte- und Neustarttests sowie
`tests/password-reset-rate-limit-e2e.sh` prüfen diesen Vertrag. Die normale
Passwortanmeldung ohne Reset-Funktion benötigt diese Tabelle nicht.

| Variable | Standard | Verwendung und Sicherheitsklasse |
|---|---:|---|
| `ZELYRA_PUBLIC_BASE_URL` | keiner | Öffentliche HTTPS-Origin für E-Mail-Links; HTTP ist nur für Loopback erlaubt. Kein Secret. |
| `ZELYRA_SMTP_HOST` | keiner | SMTP-Relay-Hostname; erforderlich, wenn `reset_tokens` aktiv ist. |
| `ZELYRA_SMTP_PORT` | `465` / `587` / `25` | Port für `implicit_tls` / `starttls` / `local_plaintext`; der letzte Modus ist nur ungeschützt auf Loopback ohne Anmeldung erlaubt. |
| `ZELYRA_SMTP_SECURITY` | `implicit_tls` | `implicit_tls`, `starttls` oder `local_plaintext`; TLS-Fehler führen nicht zu einem unverschlüsselten Fallback. |
| `ZELYRA_SMTP_FROM` | keiner | Absenderadresse für Reset-E-Mails. |
| `ZELYRA_SMTP_USERNAME` | keiner | Optionaler SMTP-Benutzer; muss zusammen mit dem Passwort gesetzt werden. Secret. |
| `ZELYRA_SMTP_PASSWORD` | keiner | SMTP-Passwort. Secret; nie ausgeben oder committen. |
| `ZELYRA_RESET_DELIVERY_KEY` | keiner | Stabiler 32-Byte-Schlüssel als 64 hexadezimale Zeichen für Outbox-Verschlüsselung und HMAC-Fingerabdrücke. Muss auf allen Instanzen identisch sein. Secret. |

Für `zelyra serve` gilt Prozessumgebung vor Projekt-`.env`. Wenn die
Auth-Deklaration `reset_tokens` enthält, startet `serve` ohne vollständige
SMTP-Konfiguration und Zustellschlüssel nicht. Einen Schlüssel erzeugt
`python3 -c 'import secrets; print(secrets.token_hex(32))'`; er muss im
Secret-Speicher aller App-Instanzen stabil hinterlegt werden.
`ZELYRA_PUBLIC_BASE_URL` muss eine HTTPS-Origin oder
eine Loopback-HTTP-Origin ohne Pfad, Query oder Fragment sein. Reset-Links
enthalten 256 Bit Zufall, werden nach 15 Minuten ungültig und nach Austausch
aus der URL in ein HttpOnly-Cookie übernommen. In der Datenbank liegen der
Token-Hash und die E-Mail/der Reset-Link ausschließlich verschlüsselt in der
Outbox. Eine erfolgreiche Änderung verbraucht den Token und
widerruft persistente und prozesslokale Sitzungen des Kontos.

Die Zustellung nutzt eine mit `ZELYRA_RESET_DELIVERY_KEY` verschlüsselte
Datenbank-Outbox; gespeichert werden nur Token-Hash und verschlüsselte
Maildaten. Ein Hintergrund-Worker versucht ausstehende Nachrichten nach
vorübergehenden SMTP-Ausfällen und Prozessneustarts erneut; nach einem
fehlgeschlagenen Versuch wartet er 30 Sekunden. Die Zustellung ist
mindestens einmalig: Ein Absturz nach SMTP-Annahme, aber vor Datenbankbestätigung
kann eine doppelte E-Mail verursachen. Schlüsselverlust macht ausstehende
Nachrichten unlesbar; Schlüssel sicher und stabil aufbewahren. Der HTTP-Request
wartet nicht auf SMTP-Antworten. Das E2E verzögert die SMTP-Antwort absichtlich
und prüft, dass die generische Antwort trotzdem vorher zurückkommt. Betroffene
Befehle sind `zelyra serve` und der generierte MariaDB-Compose-Webdienst; Tests
prüfen Schlüsselparsing/Verschlüsselung sowie Wiederaufnahme nach Neustart in
`tests/password-reset-e2e.sh`.

Diese Funktion ist experimentell. SMTP-Verfügbarkeit, öffentliche TLS-
Terminierung, dauerhafte Ratenbegrenzung über Neustarts und unabhängige
Sicherheitsprüfung bleiben Betriebs- beziehungsweise Release-Gates. Der
Ende-zu-Ende-Test `tests/password-reset-e2e.sh` verwendet eine lokale
Loopback-SMTP-Senke und MariaDB; er sendet keine externe E-Mail.

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

Jede angenommene HTTP-Verbindung hat ein festes
30-Sekunden-Frist für das vollständige Lesen der Anfrage und das Schreiben der
Antwort. Ein Timeout schließt die Verbindung; es gibt keinen konfigurierbaren
Retry. Während der Anfrageverarbeitung verwenden MariaDB und die begrenzte
PostgreSQL-SQL-Runtime den kleineren Wert aus konfigurierter Pool-/Statement-
Frist und verbleibender Request-Frist. Die Datenbank bricht ein Statement bei
Fristablauf ab; Transaktionen werden bei Fehlern zurückgerollt. Ein
Clientabbruch unterbricht den Handler nicht. Danach läuft der synchrone Handler
regulär aus. CPU-lastiger
oder anderer blockierender Anwendungscode wird nicht unterbrochen; eine
verspätete Antwort wird verworfen und der Handler kann bis zu seiner Rückkehr
einen Worker belegen. Externe TLS-Proxys sollten zusätzlich eigene Fristen und
Verbindungsgrenzen setzen.

## Nur für den Integrationstest

| Variable | Standard | Vorrang / Herkunft | Sicherheitsklasse | Betroffene Befehle und Tests |
|---|---|---|---|---|
| `ZELYRA_SCHEMA_SAFETY_MARIADB_URL` | nicht gesetzt; nur SQLite-Test | Nur Prozessumgebung; wird nicht aus Projekt-`.env` geladen. Ein expliziter Wert schaltet den zusätzlichen MariaDB-Testpfad ein. | Kann Benutzername und Passwort enthalten; nur lokale Testdatenbank verwenden, niemals ausgeben oder committen. | `bash tests/schema-safety-e2e.sh`; GitHub Actions setzt eine lokale `zelyra_ci`-Test-URL. Der Test akzeptiert ausschließlich `localhost`/Loopback und eine Basisdatenbank `zelyra_ci` oder `zelyra_test`; er erstellt und entfernt eine isolierte Datenbank. |

## Integriertes UI-Stylesheet

`ZELYRA_UI_STYLESHEET` wählt das lokal ausgelieferte Stylesheet für Zelyra-
Oberflächen. Standard ist `zelyra`; `tabler` wählt das lokal gebündelte
Tabler-Core-CSS; alternativ kann ein projektrelativer Pfad wie
`assets/tabler.css` angegeben werden. Pfade müssen auf `.css` enden, UTF-8
enthalten und dürfen keine Symlinks, `.`-/`..`-Segmente oder Dateien über
512 KiB verwenden.

| Variable | Standard | Vorrang | Sicherheitsklasse und Wirkung | Betroffene Befehle und Prüfungen |
|---|---|---|---|---|
| `ZELYRA_UI_STYLESHEET` | `zelyra` | Prozessumgebung → Projekt-`.env` → `zelyra` | Kein Secret; wählt die öffentlich ausgelieferte CSS-Datei (`zelyra`, `tabler` oder sicherer relativer `.css`-Pfad). | `zelyra serve`, erzeugte MariaDB-Compose-Projekte; CLI- und Webprüfungen zu Auswahl, Pfadsicherheit, MIME-Typ und Cache. |

Die Auswahl **ersetzt** das integrierte Zelyra-UI-Stylesheet; beide CSS-
Frameworks werden nicht parallel geladen. `tabler` verwendet die lokal
gebündelte MIT-lizenzierte Tabler-Core-CSS-Datei. Tabler-JavaScript und CDN-
Anfragen werden dabei nicht geladen. Bei einem eigenen Stylesheet bleibt
`zelyra.theme.css` separat und wird danach als Projekt-Override eingebunden.
Eigenes CSS kann durch `@import` oder `url(...)` externe Anfragen auslösen;
prüfe solche Referenzen selbst.

Die versionierten Asset-Routen lauten `/__zelyra/ui/<zelyra-version>/zelyra-ui.css`
und `/__zelyra/ui/<zelyra-version>/zelyra-ui.js`; das Projekt-Theme liegt unter
`/__zelyra/ui/project.css`. Das CSS-Bundle behält die stabile
`zelyra-ui.v1.css`-Schnittstelle. Die eingebauten Assets kommen aus dem
Zelyra-Binary, funktionieren offline und werden langfristig zwischengespeichert.
Ausgewählte Projekt-CSS-Dateien werden mit `no-cache` ausgeliefert. Zelyra
bindet die UI-Assets nur in Seiten mit Zelyra-UI-Markup ein.

## Projektlokales Theme

`zelyra new` und `zelyra init` erzeugen eine optionale `zelyra.theme.css`.
`zelyra serve` lädt die reguläre UTF-8-Datei aus dem Projektverzeichnis und
bindet sie nach dem ausgewählten Stylesheet ein. Symbolische Links und Dateien
über 128 KiB werden abgewiesen. Das Theme verändert die Darstellung, keine
Datenzugriffe, CSRF-Prüfungen, Authentifizierung oder Berechtigungen.

Die folgenden zusammengehörigen Tokens sollten bei einem eigenen Farbschema
als Paare angepasst werden:

```css
:where(html[data-z-ui]) {
    --z-color-primary: #3157a4;
    --z-color-on-primary: #ffffff;
    --z-color-primary-hover: #274889;
    --z-color-primary-soft: #eaf0fb;
    --z-color-success: #16734c;
    --z-color-on-success: #ffffff;
    --z-color-success-soft: #e9f6ef;
    --z-color-warning: #795000;
    --z-color-on-warning: #ffffff;
    --z-color-warning-soft: #fff4d8;
    --z-color-danger: #a12e39;
    --z-color-on-danger: #ffffff;
    --z-color-danger-soft: #fff0f0;
    --z-color-canvas: #f4f6fa;
    --z-color-surface: #ffffff;
    --z-color-surface-raised: #ffffff;
    --z-color-text: #182230;
    --z-color-text-muted: #526174;
    --z-color-border: #d8e0e9;
    --z-color-focus: #275fc5;
    --z-color-link: #244f9a;
    --z-content-width: 76rem;
    --z-radius-md: .65rem;
    --z-radius-lg: 1rem;
}
```

Passe Primärfarbe und `--z-color-on-primary` gemeinsam an und prüfe die
Textkontraste. Für dunkle Themes gelten eigene `--z-color-on-*`-Paare. Weitere
Anleitungen für Dichte, Inhaltsbreite und einzelne Komponenten enthält das
[Zelyra-UI-Handbuch](ui.de.md). Die Datei ist ein öffentlicher Browser-Asset,
kein Secret; hinterlege darin keine Zugangsdaten oder Tokens.

Die frühere Markierung `/* zelyra:tabler */` aktiviert kein Stylesheet mehr.
Setze `ZELYRA_UI_STYLESHEET=tabler`, um die mitgelieferte Tabler-CSS-Auswahl zu
verwenden. Das `.env`-Setting wird an den erzeugten Compose-Webdienst
weitergereicht; eine laufende Anwendung muss für Änderungen neu gestartet
werden.

`ZELYRA_REF` im generierten Dockerfile ist ein Docker-`ARG` mit einem
veröffentlichten Tag, Branch oder Commit, keine von Zelyra geladene
`.env`-Variable. Der Build lädt die Referenz mit `git fetch` und checkt sie
detached aus. Er kann ausdrücklich über `--build-arg ZELYRA_REF=...` gesetzt
werden.

Ports können unabhängig geändert werden:

```dotenv
ZELYRA_WEB_PORT=3000
ZELYRA_HOST_PORT=18080
ZELYRA_DB_HOST_PORT=3308
```

`zelyra new`, `zelyra init` und `zelyra setup` wählen freie veröffentlichte
Host-Ports, wenn sie eine neue `.env` erzeugen und ein Standardport belegt ist.
`setup` akzeptiert `--host-port` und `--db-host-port` für verbindliche Werte.
Meldet Docker Compose beim Start trotz Vorprüfung einen belegten Datenbank-
oder Webport, wählt `zelyra setup` innerhalb der bestehenden lokalen `.env`
begrenzt einen freien Folgeport, aktualisiert bei MariaDB auch passende lokale
Datenbank-URLs und wiederholt den Start. Andere `.env`-Werte und Datenbank-
Volumes bleiben erhalten. Ein prozessweit gesetztes `ZELYRA_HOST_PORT` oder
`ZELYRA_DB_HOST_PORT` hat Vorrang; Zelyra ändert diese explizite Vorgabe nicht
und meldet, dass sie angepasst oder entfernt werden muss.

Danach:

```bash
docker compose --env-file .env -f docker-compose.mariadb.yml up -d --build
```

Im aktuellen Release 1.0.4 verwendet eine Deklaration
`database <name> { ... }` bevorzugt `ZELYRA_DATABASE_<NAME>_URL`; der Name wird
für die Variable in Großbuchstaben umgewandelt und Nicht-Buchstaben/Ziffern
werden zu `_`. Für `database main` lautet der Schlüssel also
`ZELYRA_DATABASE_MAIN_URL`. Ist dieser Schlüssel nicht gesetzt, bleibt
`DATABASE_URL` der rückwärtskompatible Fallback. Wenn beide gesetzt sind, hat
der namensgebundene Schlüssel Vorrang. Eine gesetzte, leere Variable zählt als
gesetzt und löst keinen Fallback aus.

Normale `run`-, `serve`-, `db`-, `auth`- und `audit`-Befehle lesen diese Werte
aus der Prozessumgebung. Die CLI lädt `.env` dafür nicht allgemein
automatisch; Compose injiziert die Werte oder sie werden vor dem Aufruf
exportiert. `zelyra doctor --env-file <datei>` sucht in dieser Reihenfolge in
der angegebenen Datei: namensgebundener Schlüssel, dann `DATABASE_URL`.
Ohne `--env-file` prüft `doctor` automatisch die `.env` neben der angegebenen
Quelldatei, falls diese Datei existiert. Ein Wert aus der Prozessumgebung hat
Vorrang vor dem Wert aus der Datei. Der Doctor zeigt nur den verwendeten
Variablennamen und Dateipfad an, niemals den Verbindungswert. Diese automatische
Datei-Auswahl gilt nur für `doctor`; `run` und `serve` laden Datenbank-URLs
weiterhin nicht allgemein aus `.env`.
Zelyra-Server müssen mit den aktualisierten Prozesswerten neu gestartet
werden. Die namensgebundenen Variablen sind ab Zelyra 1.0.4 verfügbar.

## MariaDB-Zeitlimits, Connection-Pool und TLS

Diese Einstellungen sind im aktuellen Release 1.0.4 verfügbar. Sie werden ausschließlich aus
der Prozessumgebung gelesen. Zelyra lädt dafür keine `.env`-Datei und kennt
keinen entsprechenden `zelyra.toml`-Schalter. Ein Docker-Compose-Dienst kann
die Werte aus der Compose-`.env` übernehmen; bei einem laufenden Server ist ein
Neustart nötig, damit er neue Prozesswerte erhält.

| Variable | Standard | Zulässig | Vorrang / Quelle | Secret | Betroffene Pfade und Tests |
|---|---:|---:|---|---|---|
| `ZELYRA_DB_CONNECT_TIMEOUT_SECS` | `10` Sekunden | Ganzzahl `1`–`300` | Prozessumgebung; sonst Standardwert | nein | MariaDB- und PostgreSQL-Poolverbindungen; PostgreSQL-Einstellung gilt für die begrenzte Runtime-API in 0.6.0 |
| `ZELYRA_DB_QUERY_TIMEOUT_SECS` | `30` Sekunden | Ganzzahl `1`–`3600` | Prozessumgebung; sonst Standardwert | nein | MariaDB- und PostgreSQL-Runtime-Statements; nicht `db apply`/DDL; PostgreSQL gilt für die begrenzte Runtime-API in 0.6.0 |
| `ZELYRA_DB_POOL_MAX_SIZE` | `8` Verbindungen | Ganzzahl `1`–`64` | Prozessumgebung; sonst Standardwert | nein | Harte maximale MariaDB- und PostgreSQL-Poolgröße pro Zelyra-Prozess |
| `ZELYRA_DB_POOL_WAIT_TIMEOUT_SECS` | `10` Sekunden | Ganzzahl `1`–`300` | Prozessumgebung; sonst Standardwert | nein | Maximale Wartezeit auf eine freie MariaDB- oder PostgreSQL-Poolverbindung |
| `ZELYRA_DB_TLS_MODE` | `auto` | `auto`, `disabled` oder `required` | Prozessumgebung; sonst `auto` | nein, aber sicherheitskritisch | TLS-Richtlinie für Runtime-Pools und MariaDB-CLI; Neustart zum Wechseln |
| `ZELYRA_DB_TLS_CA_CERT_FILE` | nicht gesetzt | absoluter Pfad zu lesbarer PEM-/DER-CA-Datei; PostgreSQL Runtime-API erwartet PEM | Prozessumgebung; optional; mit `disabled` unzulässig | nein; Zertifikat ist öffentlich, Vertrauensanker aber sicherheitskritisch | Zusätzliche vertrauenswürdige CA für Runtime-Pools und MariaDB-CLI |

Ungültige Werte führen zu einer geheimnisfreien Konfigurationsdiagnose; der
übergebene Wert wird nicht ausgegeben. Im Modus `auto` verlangt Zelyra für
nicht lokale Hosts TLS und prüft Zertifikatskette sowie Hostnamen; `localhost`,
Namen unter `.localhost` und Loopback-IP-Adressen bleiben für lokale Entwicklung
ohne TLS. Für den PostgreSQL-Runtime-Pool prüft rustls bei TLS die Zertifikatskette
und den Hostnamen; Mozilla-Web-PKI-Wurzeln werden durch die optionale PEM-CA ergänzt.
`required` erzwingt geprüfte TLS-Verbindungen auch lokal. `disabled`
schaltet TLS ausdrücklich ab und ist nur für isolierte lokale Netze gedacht;
bei TLS-Fehlern gibt es keinen unsicheren Rückfall. Ohne eigene CA verwendet der
Rustls-Treiber die mitgelieferten öffentlichen Stammzertifikate. Eine
PostgreSQL-CA-Datei muss absolut, lesbar und PEM-kodiert sein; der
MariaDB-Client unterstützt PEM/DER.
Für Schema-Inspektion und DDL erhält der MariaDB-Client dieselben TLS-Vorgaben
mit `--ssl` und `--ssl-verify-server-cert` sowie optional `--ssl-ca`.
In Docker muss eine private CA in den Container eingebunden und dort unter dem
konfigurierten Pfad lesbar sein.
Die vollständige MariaDB-Projektvorlage setzt `disabled` für ihr eigens
erzeugtes, internes Compose-Netz explizit. Ein exportiertes Docker-Modul
verbindet sich dagegen mit einer externen Datenbank und startet deshalb mit
`auto`; nur ein bewusst isolierter lokaler Datenbankverbund sollte in dessen
`.env` auf `disabled` umgestellt werden.

Der MariaDB-Client erhält außerdem `--skip-reconnect`, damit ein
Verbindungsverlust nicht unbemerkt zu einem Wiederverbinden oder automatischen
Wiederholen führt. MariaDB-Runtime-SQL und die begrenzte PostgreSQL-API verwenden
prozessweite, begrenzte Pools. Beim Checkout wird die Verbindung geprüft; nach
Statementfehlern wird sie verworfen, und innerhalb einer Transaktion wird ein
Rollback versucht. Es gibt keine automatischen Retries. Ein Prozess kann nur
eine Datenbank-URL und eine Poolkonfiguration verwenden; Änderungen erfordern
einen Neustart. Der Pool gilt nicht für Schema-Inspektion oder DDL, die weiterhin
den MariaDB-Clientprozess verwenden. Das Statement-Limit wird serverseitig
durchgesetzt und begrenzt weder die Übertragung großer Ergebnismengen noch den
gesamten Antwort-/Prozesslebenszyklus. DDL bleibt vom Statement-Limit
ausgenommen, da ein Abbruch einen Teilzustand hinterlassen kann. Der positive
TLS-Handshake und die Ablehnung einer nicht vertrauenswürdigen CA werden in der
MariaDB-Kompatibilitätsmatrix automatisiert geprüft; Windows-TLS ist noch nicht
separat geprüft.

## Laufzeit und Authentifizierung

| Variable | Verwendung | Sicherheit |
|---|---|---|
| `ZELYRA_AUTH_TOKEN` | optionaler lokaler Bearer-Token für geschützte Requests | Secret, niemals committen |
| `ZELYRA_AUTH_PERMISSIONS` | durch Kommas getrennte lokale Permission-Allowlist | keine Passwörter, trotzdem nicht unnötig loggen |
| `ZELYRA_MODE` | Beispiel für `env("ZELYRA_MODE")` bei aktivierter `Environment`-Capability | Inhalt ist anwendungsspezifisch |

Die Anwendung darf mit `env(...)` nur auf Umgebungswerte zugreifen, wenn die
entsprechende Capability gewährt wurde. `DATABASE_URL` und Auth-Secrets
gehören nicht in Zelyra-Quellcode, JSON-Diagnosen, Kontextausgaben oder Logs.

Die lokale, optionale MFA-Verschlüsselung verwendet zusätzlich
`ZELYRA_MFA_ENCRYPTION_KEY`. Der Wert ist ein vom Betreiber verwaltetes
Geheimnis aus 64 hexadezimalen Zeichen (32 zufällige Bytes), wird nie
standardmäßig gesetzt und muss über Neustarts und alle App-Instanzen stabil
bleiben. Er kann in generierten MariaDB-Projekten in `.env` eingetragen
werden; Compose reicht ihn an den Webdienst weiter. Ohne MFA bleibt die
Variable leer. Details zu Schlüsselverlust und Sicherung stehen im
[technischen Referenzhandbuch](handbook/de/technisches-referenzhandbuch.md).

## CLI- und Testvariablen

| Variable | Status | Verwendung |
|---|---|---|
| `ZELYRA_INSTALL_ROOT` | implementiert | benutzerbezogenes Ziel der Installationsskripte |
| `ZELYRA_COLOR` | implementiert | `auto` (Standard), `always` oder `never`; steuert ANSI-Farben in CLI-Hilfe und Erfolgsmeldungen |
| `NO_COLOR` | implementiert | Wenn gesetzt, schaltet es automatische Farben aus; `ZELYRA_COLOR=always` hat Vorrang |
| `CLICOLOR_FORCE` | implementiert | Ein nicht leerer Wert außer `0` erzwingt Farben, sofern `ZELYRA_COLOR` nicht gesetzt ist und `NO_COLOR` fehlt |
| `ZELYRA_DB_TIMEOUT_TEST_URL` | CI-/Testvariable | MariaDB-URL für Timeout- und Pool-Integrationstests; ausschließlich lokale/isolierte Testdatenbank |
| `ZELYRA_MODULE_IMPORTS_DATABASE_URL` | CI-/Testvariable | Optionale MariaDB-Basis-URL für Paket-/Modul-Laufzeitintegrationstests; ohne Wert wird der Datenbankfall übersprungen. Nur mit isoliertem Testserver setzen, da der Test eine eindeutig benannte Datenbank anlegt und löscht |
| `ZELYRA_UPLOAD_TEST_URL` | CI-/Testvariable | Nur für `tests/mariadb-file-upload-e2e.sh`; Verbindung zu einer eigens erstellten, wegwerfbaren MariaDB-Testdatenbank. Der Test legt darin `zelyra_upload_e2e` an und löscht es. Nicht für produktive Datenbanken verwenden; kein Projekt-`.env`-Eintrag |
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

`ZELYRA_UPLOAD_TEST_URL` ist standardmäßig nicht gesetzt und wird nur vom
isolierten `tests/mariadb-file-upload-e2e.sh` aus der Prozessumgebung gelesen;
die Projekt-`.env` wird dafür nicht geladen. Der Wert kann Zugangsdaten
enthalten und ist als Geheimnis zu behandeln. Das Skript startet eine eigene
wegwerfbare MariaDB-Instanz, legt darin `zelyra_upload_e2e` an und entfernt
danach die Instanz. Produktivbefehle sind nicht betroffen.

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
console = false

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

[runtime]
max_steps = 100000
max_stack_depth = 100
scheduler_job_timeout_seconds = 300

[web]
allowed_origins = ["http://localhost:5173"]
allow_credentials = false
```

Diese Bereiche sind keine bequeme Umgehung von Sicherheit: Capabilities,
Allowlisten, SQL-Prüfungen, CSRF und destruktive Datenbankfreigaben bleiben
explizit und werden nicht durch `.env` abgeschaltet.

`[runtime].max_steps` ist optional und muss eine positive, für die Plattform
darstellbare Ganzzahl sein. Ist der Wert nicht gesetzt, begrenzt AST-Ausführung
über `zelyra run` und normale Webhandler die Auswertungsschritte nicht; MIR und
Bytecode verwenden bei CLI-Läufen ihr dokumentiertes Budget von 100.000
Schritten. Ein explizites `zelyra run --max-steps <n>` hat Vorrang vor dem
Projektwert. Das Projektlimit gilt für `zelyra run`, ausgeführte
Webhandler sowie Scheduler-Commands, die der integrierte Dienst per `zelyra
run` startet. Es begrenzt nicht die Laufzeit externer Programme und ersetzt
keine Netzwerk-, Datenbank- oder Prozess-Timeouts. Es ist keine geheime
Einstellung; `.env` und Prozessvariablen überschreiben es nicht.

Der bestehende `[runtime].max_stack_depth`-Wert bleibt davon getrennt und
begrenzt Funktionsaufruftiefe auf Werte von 1 bis 500 (Standard 20).

`[runtime].scheduler_job_timeout_seconds` begrenzt die Wandzeit eines einzelnen
Handlers im residenten Scheduler auf 1 bis 86.400 Sekunden; Standard sind
300 Sekunden. Nach Ablauf beendet der Dienst den Handler-Prozess und protokolliert
den Timeout. Das gilt nur für Zeitpläne im installierten, aktiven Linux-
systemd-Projektdienst, nicht für manuelle `zelyra task run`-Aufrufe oder normale
Webhandler. Eine Beendigung kann externe Effekte, die der Handler bereits
ausgelöst hat, nicht rückgängig machen; der Scheduler wiederholt fehlgeschlagene
Aufträge derzeit nicht automatisch. Der Wert ist nicht geheim und wird weder
durch `.env` noch Prozessvariablen überschrieben.

Funktions-SQL kann die engeren Deklarationen `uses Database(read)` für
`SELECT` und `uses Database(write)` für `INSERT`, `UPDATE` oder `DELETE`
verwenden. Unter `[capabilities]` werden sie mit `database_read = true` und
`database_write = true` freigegeben. `uses Database` und `database = true`
bleiben als rückwärtskompatible breite Freigaben verfügbar. SQL-Anweisungen,
die der Prüfer nicht einordnen kann, benötigen die breite Capability; diese
Quellfreigaben erstellen oder beschränken keine MariaDB-Benutzer.
Framework-CRUD, Formulare, Authentifizierung und Page-Daten benötigen weiterhin
die breite Datenbankfreigabe.

`console = false` ist die standardmäßige Projektfreigabe für interaktive
Terminaleingabe. Programme, die `read_console(prompt)` verwenden, müssen
außerdem in der aufrufenden Funktion `uses Console` deklarieren. Setze die
Freigabe nur für CLI-Programme auf `true`; sie ändert weder Webformulare noch
Browser-Routen. Terminaleingaben werden nicht verborgen und eignen sich nicht
für Passwörter oder andere Geheimnisse. Ohne `[capabilities]`-Abschnitt gibt es
keine zusätzliche Projekt-Allowlist, die Funktionsdeklaration `uses Console`
bleibt trotzdem Pflicht. Prozessvariablen und `.env` können diese Capability
nicht erteilen.
Betroffene Prüfungen und Befehle: `zelyra check`, `zelyra build` und
`zelyra run`; Compiler- und CLI-Integrationstests decken Typ, Capability,
Prompt, Zeilenende und EOF ab. Die Einstellung ist kein Secret.

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

## DATABASE_URL: feste Dezimalpräzision

Mit einer konfigurierten Datenbank und freigegebener Database-Fähigkeit
prüfen `serve` sowie die erste SQL-Ausführung eines Interpreters deklarierte
Decimal-/Money-/UInt-Präzision anhand schreibgeschützter Metadaten.
Metadatenzugriff ist erforderlich; ein altes oder inkompatibles Schema wird
nicht still verwendet oder migriert. Es gibt keinen Abschaltschalter.
Die bestehenden URL-Prioritäts- und Geheimnisregeln bleiben wirksam.
Siehe [Regeln und Tests](decimals.de.md).

## DATABASE_URL: Datum und Uhrzeit

Native Runtime-Verbindungen verwenden UTC; PostgreSQL-URL-Startoptionen bleiben
erhalten und werden um UTC ergänzt. MariaDB/MySQL DATETIME enthält keine
Zeitzone; externe Schreiber müssen dieselbe UTC-Konvention verwenden. Date-,
Time- und Timestamp-Speicher werden vor `serve` und erster Interpreter-SQL-
Ausführung schreibfrei gegen die deklarierte Struktur geprüft. Fehlende Spalten,
falsche Typen und unpassende Zeitpräzision werden mit Fundstelle abgewiesen;
Metadatenzugriff ist erforderlich. Migrationen bleiben ausdrücklich zu prüfen.
Es gibt keinen neuen Schalter. Siehe [Datumsregeln](dates.de.md).

## Interne Kennung des Linux-Systemdienstes

`ZELYRA_SYSTEMD_SERVICE_ID` wird ausschließlich in einer von
`zelyra service install` oder `zelyra service change` erzeugten
systemd-User-Unit gesetzt. Der Wert ist eine aus dem kanonischen Projektpfad
abgeleitete Kennung, kein Geheimnis und kein Berechtigungsnachweis. Die Unit
setzt ihn nach der optionalen Projekt-`.env`, sodass ein dort eingetragener
Wert die interne Kennung nicht überschreibt. Manuelles Setzen aktiviert den
Scheduler nicht: Zelyra prüft zusätzlich, dass die zu genau diesem Projekt
gehörende systemd-Unit tatsächlich aktiv ist. Ohne Kennung oder außerhalb
dieses Dienstes führt `serve` keine Zeitpläne aus. Betroffen sind
`zelyra service install|change` und `zelyra serve`; der Linux-Scheduler-
Integrationstest prüft die Dienstbindung.
## Lokale Paket-Einstellungen (experimentell)

`zelyra.settings.toml` speichert nicht geheime Paketwerte unter
`[package_settings."<paketpfad>"]`; Standardwerte aus dem Paketmanifest gelten
bei ausgelassenen optionalen Feldern. Geheimnisfelder dürfen nur auf
Prozessumgebungsnamen im Format `UPPERCASE_NAME` verweisen. Die CLI liest deren
Werte ausschließlich für `zelyra module settings check`; es gibt keine
Fallback-Datei und keine automatische `.env`-Auflösung für diesen Mechanismus.
Ein vorhandener Prozesswert ist die einzige Quelle; Standardwerte für
Geheimnisse sind verboten. Die Ausgabe enthält nur Verfügbarkeitsstatus und
nie Variablennamen oder Werte. Die Datei wird auf Symlinks, Typen und eine
Größe von höchstens 1 MiB geprüft und ist weder Bestandteil des Paket-Lockfiles
noch derzeit eine Runtime-Einstellung. Geprüft durch Paket- und CLI-
Integrationstests.
