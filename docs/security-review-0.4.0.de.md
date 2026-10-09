# Sicherheits- und Threat-Review 0.4.0

**Status:** internes Source- und Threat-Review für das experimentelle Release
0.4.0. Es ist kein externes Audit, Penetrationstest, Rechtsgutachten zu
Lizenzen, Produktionsfreigabe oder menschlicher Einsteigertest. Die menschliche
Einsteigerabnahme wurde in den Entscheidungsprotokollen ausdrücklich auf 0.5.0
verschoben.

## HTTP- und API-Grenzen

- Healthcheck-Kategorien, die 30-Sekunden-Verbindungsfrist und sichere
  Wiederholungsgrenzen stehen in der
  [HTTP-Betriebsanleitung](http-operations.de.md). Die TCP-Probe prüft die
  Listener-Erreichbarkeit; Readiness gehört zur Anwendung. Eine reservierte,
  datenbankfreie Liveness-Route bestätigt die Anfrageverarbeitung. Zelyra
  wiederholt keine Requests oder Transaktionen.
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
- HTTP-Verbindungen haben eine feste Lese-/Schreibfrist von 30 Sekunden.
  MariaDB-Poolwartezeiten und Abfragen nutzen die verbleibende Request-Frist;
  abgelaufene Abfragen werden serverseitig abgebrochen. CPU-intensive und
  andere blockierende Handler-Arbeit bleibt synchron und kann einen Worker
  belegen; der Server begrenzt dies auf 64 parallele Verbindungen und verwirft
  weitere. Eine Client-Trennung wird erst beim Schreiben der Antwort erkannt.

## Konten und Datenbankbetrieb

- Login-Fehlversuche sind pro Auth-Definition konfigurierbar und prozesslokal.
  Passwortzurücksetzung ist für das dokumentierte experimentelle MariaDB-Profil
  implementiert: MariaDB speichert Blake2s-Hashes zufälliger Einweg-Tokens mit
  15 Minuten Ablaufzeit;
  generische Antworten, CSRF/Origin-Prüfung, Audit und Widerruf persistenter
  sowie flüchtiger Sitzungen sind vorhanden. Eine datenbankgestützte Outbox
  speichert E-Mail und Link authentifiziert und AES-256-GCM-verschlüsselt unter
  `ZELYRA_RESET_DELIVERY_KEY`. Der Worker versucht nach SMTP-Ausfall und
  Prozessneustart erneut. Das MariaDB-/SMTP-E2E prüft Verschlüsselung im
  Ruhezustand sowie Wiederaufnahme nach Neustart und mit zwei konkurrierenden
  Instanzen.
  Unter dem Advisory-Lock je Nutzer prüft der Worker vor dem Versand erneut, ob
  die Outbox-Nachricht offen und fällig ist. Fehlerhafte oder nicht entschlüsselbare
  Zeilen werden um 30 Sekunden verschoben, damit sie nicht wiederholt den ersten
  begrenzten Poll-Batch belegen. Ein gezielter Unit-Test und ein MariaDB-/SMTP-
  E2E mit zwei Instanzen decken offene und fällige Einträge ab. Das E2E verzögert
  SMTP über dessen Timeout hinaus und prüft die veraltete Auswahl des zweiten
  Workers.
  Zustellung erfolgt mindestens einmalig: Ein Absturz nach SMTP-Annahme, aber
  vor Datenbankbestätigung kann E-Mail-Duplikate erzeugen. Schlüsselverlust lässt
  wartende Nachrichten unlesbar zurück. Schlüsselrotation ist nur sicher, wenn
  die Outbox leer ist; bis dahin muss der alte Schlüssel erhalten bleiben.
  Datenbankzeiten können variieren. Das MariaDB-/SMTP-E2E prüft Enumeration,
  verzögerte Zustellung, Token-Hash, Replay, Ablauf, CSRF/Origin,
  Sitzungswiderruf, parallele Einlösung, SMTP-Ausfall und Wiederaufnahme nach
  Neustart. Ein Test mit eingefrorener Uhr prüft das produktive Token-Prädikat
  vor, exakt bei und nach Ablauf; Gleichheit wird abgelehnt. Token-Ersetzung und
  Outbox-Einfügung werden durch MariaDB-Advisory-Locks serialisiert. Der E2E-
  Test prüft, dass die letzte E-Mail zum gespeicherten aktiven Token gehört.
- Login-, Reset- und API-Kontingente sind in diesem Release bewusst
  prozesslokal, werden beim Neustart zurückgesetzt und bieten keinen verteilten
  Missbrauchsschutz. Die dokumentierten Grenzen und Anforderungen an vertrauens-
  würdige Proxys gehören zum Releaseumfang.
- Sitzungsadministration und Selbstverwaltung des Widerrufs sind für dauerhafte
  und flüchtige Sitzungen umgesetzt. Sitzungslisten zeigen eine
  begrenzte, HTML-escaped `User-Agent`-Angabe, wenn das Schema dauerhafter
  Sitzungen `device_label` deklariert; flüchtige Sitzungen erfassen dieselbe
  Angabe. Der Wert ist clientkontrolliert und bestätigt keine Geräteidentität.
  IP-Adressen werden nicht erfasst.
- MariaDB-TLS prüft Zertifikatskette und Hostnamen gemäß dokumentiertem Modus.
  Die Linux-CI-Matrix deckt MariaDB 10.11.19, 11.4.13, 11.8.9 und 12.3.3 ab.
  Windows-Artefakte werden gebaut, aber ein Windows-zu-MariaDB-TLS-Handshake
  wird nicht separat behauptet. TLS endet weiterhin am Datenbankserver;
  App-HTTP bleibt separat.
- PostgreSQL- und SQLite-Schemaanwendungen laufen jetzt transaktional; Tests
  zeigen, dass ein fehlgeschlagener DDL-Schritt vorherige Änderungen
  zurücksetzt. MariaDB-DDL kann weiterhin Teilzustände hinterlassen. Lesende
  Vorprüfungen blockieren einen Plan vor SQL, wenn neue Unique-Indizes
  Dubletten oder neue Foreign Keys verwaiste Werte finden. Parallele
  Schreibzugriffe können diese Prüfungen überholen; sie bieten keine Isolation.
  Alle Schema-Backends protokollieren Migrationspläne und Ergebnisse; MariaDB
  protokolliert zusätzlich Prüfpunkte je DDL-Schritt. CI prüft Unterbrechungen
  zwischen DDL-Befehlen und während DDL auf eine Metadatensperre wartet, die
  Wiederherstellung mit neuem Plan sowie Backup/Restore. Ein Absturz während
  aktiver MariaDB-DDL kann einen mehrdeutigen Teilzustand hinterlassen; ein
  automatisches oder datensicheres Rollback wird nicht behauptet. Rückwärts-DDL
  erfordert weiterhin einen neuen geprüften Plan und ein geprüftes Backup.

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
- Dependency- und Lizenzrichtlinie bestehen automatisiert in CI. `NOASSERTION`-
  Lizenzen und Warnungen zu doppelten Versionen bleiben sichtbar; dies ist kein
  Rechtsgutachten. Getaggte Builds sollen Archive und SBOMs attestieren; die
  Provenance und Prüfung veröffentlichter 0.4.0-Kandidatenartefakte sind erst
  nach Kandidatenbau abgeschlossen.
- Granulare Effekte sowie vollständige Modul-`impact`-/`edit`-Abdeckung bleiben
  teilweise offene P1-Arbeit. Die 30-Sekunden-Frist für Netzwerk und Datenbank
  unterbricht beliebigen CPU-intensiven Handlercode nicht. Keine der Grenzen
  wird als vollständig umgesetzt beworben.

## Releaseentscheidung

Dieses Review hält die getesteten Grenzen und verbleibenden Risiken des
experimentellen Releases fest. Es behauptet weder ein externes Audit noch
Produktionsreife. Releaseplan und
[Risikoregister](release-readiness/0.4.0-risikoregister.de.md) bleiben für
kandidatenbezogene CI-, Artefakt-, Prüfsummen-, SBOM-, Attestierungs- und
Installations-/Upgradeprüfungen maßgeblich.
