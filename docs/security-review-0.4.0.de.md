# Sicherheitsreview 0.4.0 — Arbeitsstand

**Status:** technische Bestandsaufnahme, kein externes Audit und keine
Releasefreigabe. Geprüft wurde der 0.4-Entwicklungszweig zum Zeitpunkt dieses
Commits. Dieses Dokument beschreibt implementierte Grenzen und bekannte Lücken;
es ersetzt weder Penetrationstest noch menschliche Abnahme.

## HTTP- und API-Grenzen

- Zelyras eingebauter HTTP-Server terminiert kein TLS. Für öffentliche oder
  LAN-erreichbare Dienste ist ein vertrauenswürdiger TLS-Proxy erforderlich;
  der Anwendungsport muss vor direktem Zugriff geschützt sein.
- API-Kontingente verwenden die TCP-Peer-IP. `X-Forwarded-For` und ähnliche
  Proxy-Header werden nicht als Clientidentität akzeptiert. Hinter einem Proxy
  werden daher zunächst alle Requests unter dessen Peer-IP gezählt.
- API- und Login-Limiter liegen im Prozessspeicher. Ein Neustart setzt den
  Zustand zurück; getrennte Instanzen koordinieren sich nicht. API-Buckets sind
  auf 4096 begrenzt und bei vollem Speicher für neue Schlüssel fail-closed.
- Login-Buckets sind ebenfalls auf 4096 Schlüssel begrenzt, bereinigen
  abgelaufene Zeitfenster und lehnen bei voller Tabelle neue Schlüssel
  fail-closed ab. Ein Angreifer kann so bis zur Bereinigung legitime neue
  Anmeldeschlüssel aussperren; ein verteilter Schutz entsteht dadurch nicht.
  Die normalisierte E-Mail-Adresse wird als Blake2s-Schlüssel gespeichert,
  nicht im Limiterzustand im Klartext gehalten.
- Request-IDs werden auf höchstens 128 ASCII-Zeichen aus Buchstaben, Ziffern,
  Punkt, Unterstrich und Bindestrich beschränkt. Ungültige Werte werden ersetzt.
  Die ID ist Korrelationsmetadatum, keine Authentifizierung und kein Geheimnis.
- Rate Limits erschöpfen mit `429` und `Retry-After`. Sie sind keine
  DDoS-Abwehr und ersetzen weder Edge-Limits noch persistente Kontrollen.
- HTTP-Verbindungen haben eine feste 30-Sekunden-Frist. Sie schließt Lese- und
  Schreibvorgänge ein, bricht synchrone Handler aber nicht ab; ein blockierter
  Handler kann weiter den einzelnen Serverprozess belegen.

## Konten und Datenbankbetrieb

- Login-Fehlversuche sind pro Auth-Definition konfigurierbar und prozesslokal.
  Passwortzurücksetzung ist im 0.4-Zweig teilweise implementiert: MariaDB
  speichert Blake2s-Hashes zufälliger Einweg-Tokens mit 15 Minuten Ablaufzeit;
  generische Antworten, CSRF/Origin-Prüfung, Audit und Widerruf persistenter
  sowie flüchtiger Sitzungen sind vorhanden. Eine begrenzte Ein-Worker-Queue
  mit bis zu 64 E-Mails hält den SMTP-Versand aus dem HTTP-Request heraus; bei
  vollem Queue wird der nicht zugestellte Token entfernt. SMTP-Ausfälle nach
  Einreihung werden protokolliert, aber nicht erneut zugestellt. Die Limits
  sind prozesslokal und nicht zwischen Instanzen geteilt; DB-Zugriffszeiten
  können weiterhin variieren. Ein MariaDB-/SMTP-Senken-E2E prüft Enumeration,
  verzögerte Zustellung, Token-Hash, Replay, Ablauf, CSRF/Origin und
  Sitzungswiderruf. Der E2E-Test löst dasselbe Reset-Token nun außerdem
  gleichzeitig zweimal ein und prüft genau einen Erfolg, eine Ablehnung und
  die Anmeldung nur mit dem siegreichen Passwort. Parallele Token-Ausstellung
  und FIFO-Maileinreihung sind innerhalb eines Prozesses serialisiert; der
  E2E-Test bestätigt, dass die letzte E-Mail zum gültigen Token gehört.
  Mehrprozess-Reihenfolge, genaues Ablaufgrenzverhalten, SMTP-Ausfall und der
  Mail-/Token-Lebenszyklus bleiben unabhängig zu prüfen.
- Sitzungsadministration ist teilweise umgesetzt. Gerätemetadaten und die
  vollständige Selbstverwaltung bleiben offen; die Kontolebenszyklus-Abnahme
  ist nicht bestanden.
- MariaDB-TLS prüft Zertifikatskette und Hostnamen gemäß dokumentiertem Modus.
  TLS endet weiterhin am jeweiligen Datenbankserver; App-HTTP bleibt separat.
  Windows-TLS braucht noch einen eigenen Matrixnachweis.
- PostgreSQL- und SQLite-Schemaanwendungen laufen jetzt transaktional; Tests
  zeigen, dass ein fehlgeschlagener DDL-Schritt vorherige Änderungen
  zurücksetzt. MariaDB-DDL kann weiterhin Teilzustände hinterlassen. Lesende
  Vorprüfungen blockieren einen Plan vor SQL, wenn neue Unique-Indizes
  Dubletten oder neue Foreign Keys verwaiste Werte finden. Parallele
  Schreibzugriffe können diese Prüfungen überholen; sie bieten keine Isolation.
  Persistenter Migrationsverlauf, sicheres inverses SQL, Wiederherstellung nach
  tatsächlichem Prozessabbruch und eine vollständige Restore-Probe bleiben
  offen. Vorhandene Backups bleiben Betreiberverantwortung.

## Lieferkette und verbleibende Gates

- Linux- und Windows-Release-Builds erzeugen SPDX-2.3-SBOMs aus dem
  aufgelösten Cargo-Graph und prüfen den Binary-SHA-256. Nicht als SPDX
  ausdrückbare Cargo-Lizenzfelder stehen als `NOASSERTION` samt Originalwert.
- CI führt `cargo-deny` mit fester Version für Linux- und Windows-
  Abhängigkeitsgraphen aus. Geprüft werden RustSec-Advisories, SPDX-Lizenzen,
  doppelte Versionen, Wildcard-Anforderungen und Registry-/Git-Quellen. Die
  Policy klärt `webpki` anhand einer gehashten Lizenzdatei als ISC und enthält
  eine begründete Ausnahme für die aktuell aufgelöste, nicht mehr gepflegte
  Kompatibilitätsbibliothek `rustls-pemfile`, die `mysql` 28.0.3 benötigt.
  Doppelte Versionen von `base64` und `getrandom` warnen weiterhin.
- SBOM und automatische Prüfungen sind kein menschliches Lizenzgutachten.
  Prüfung von Abhängigkeitszweck, Lizenzpflichten, übrigen Warnungen, signierter
  Provenance und jedem nativen Artefakt bleiben offen.
- Granulare Effekte, vollständige HTTP-Response-Deadlines, unabhängiger
  Sicherheitsreview und menschliche Abnahme sind offen.

## Releaseentscheidung

Dieses Review schließt kein P0-Gate. Ein finales `v0.4.0` bleibt gesperrt, bis
der Releaseplan und das [Risikoregister](release-readiness/0.4.0-risikoregister.de.md)
für denselben Kandidaten vollständig belegt sind.
