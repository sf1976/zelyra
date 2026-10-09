# HTTP-Healthchecks, Fristen und Wiederholungen

Diese Anleitung beschreibt die Überwachung des eingebauten HTTP-Servers von
Zelyra und den Umgang von Clients mit vorübergehenden Fehlern. Sie beschreibt
das aktuelle Verhalten und verspricht keinen Abbruch eines Request-Handlers.

## Kategorien für Healthchecks

- **Listener-Erreichbarkeit:** Prüfe, ob der konfigurierte TCP-Endpunkt
  erreichbar ist. Ein erfolgreicher TCP-Handshake beweist nicht, dass Zelyra
  die Verbindung verarbeitet hat.
- **Prozess-Liveness:** Rufe `GET /__zelyra/health/live` auf (oder `HEAD` für
  eine Antwort ohne Inhalt). `200` bestätigt, dass Zelyra die Anfrage gelesen
  und an den Router übergeben hat. Die Antwort wird nicht zwischengespeichert
  und prüft weder Datenbanken noch andere Anwendungsabhängigkeiten. Der Pfad
  ist reserviert; kollidierende Projektrouten scheitern bei der Prüfung mit
  `E-WEB-005`. Die normale Host-Prüfung gilt weiterhin; bei aktivierter
  Host-Begrenzung muss die Anfrage einen erlaubten Host verwenden.
- **Anwendungs-Readiness:** Stelle in der Anwendung eine eigene lesende
  `GET`-API-Route für die zum normalen Betrieb erforderlichen Abhängigkeiten
  bereit. Zelyra reserviert oder erzeugt keine Readiness-Route. Die Antwort
  bleibt allgemein und enthält weder Datenbankdiagnosen noch Zugangsdaten oder
  interne Hostnamen.
- **Abhängigkeits-Readiness:** Benötigt die Anwendung MariaDB, kann ihr eigener
  Readiness-Handler eine begrenzte, ausschließlich lesende Abfrage ausführen.
  Schema-Inspektion, Migrationen und Schreibzugriffe gehören nicht in einen
  Probeaufruf. `zelyra doctor --json` ist eine punktuelle CLI-Diagnose, kein
  HTTP-Probeaufruf und kein Liveness-Signal.

Liveness und Readiness werden getrennt ausgewertet. Ein Datenbankausfall
sollte eine davon abhängige Anwendung als nicht bereit markieren; er sollte
einen Orchestrator nicht dazu bringen, einen gesunden Zelyra-Prozess wiederholt
neu zu starten. Anwendungen ohne Datenbank definieren Readiness anhand ihrer
eigenen erforderlichen Abhängigkeiten.

## Verbindungs- und Request-Fristen

Der Listener verarbeitet höchstens 64 Verbindungen parallel und verwirft
weitere. Für jeden Verbindungsdialog gilt eine absolute Frist von 30 Sekunden,
einschließlich Request-Lesen und Response-Schreiben. Bei einem Timeout während
des Lesens kann die Verbindung ohne HTTP-Antwort geschlossen werden; eine erst
nach Ablauf fertige Antwort wird verworfen.

Request-Handler sind synchron und können derzeit nicht abgebrochen werden.
Ein Handler, der die Socket-Frist überschreitet, kann weiterhin einen der 64
Worker belegen, bis seine Operation zurückkehrt. Datenbank-Fristen für
Verbindungsaufbau, Pool-Wartezeit und Statements begrenzen zusätzliche
MariaDB-Operationen, brechen aber keinen beliebigen Anwendungscode ab. Ein
Client-Timeout beweist daher nicht, dass ein Schreibzugriff zurückgerollt
wurde.

## Wiederholungsregeln

Zelyra wiederholt HTTP-Anfragen, SQL-Statements oder Transaktionen nicht
automatisch. Nach einem unsicheren Transaktionsfehler wird keine Verbindung
wiederverwendet. Anwendung und Client müssen jede Wiederholungsstrategie
ausdrücklich festlegen.

- `GET` und `HEAD` lassen sich nach einem Verbindungsfehler oder vorübergehendem
  `502`/`503`/`504` normalerweise sicher wiederholen. Begrenze die Anzahl und
  nutze exponentielle Wartezeiten mit Jitter; beachte ein vorhandenes
  `Retry-After`.
- Schreibzugriffe nur dann wiederholen, wenn sie idempotent oder durch einen
  Idempotency-Key beziehungsweise eine bedingte Aktualisierung geschützt sind.
  Ein Schreibzugriff kann trotz Client-Timeout bereits gespeichert sein.
- Eine Transaktion nach Timeout oder Verbindungsverlust nicht blind erneut
  senden. Ermittle zuerst, ob ihre Änderungen übernommen wurden. Ist das nicht
  feststellbar, gleiche den Anwendungszustand ab, bevor du erneut schreibst.
- `429`-Antworten konfigurierter API-Quoten enthalten `Retry-After`. Beachte
  die Wartezeit und eine begrenzte Anzahl von Wiederholungen.

API- und Login-Quoten liegen im Prozessspeicher. Instanzen teilen keinen
Quotenzustand; ein Neustart leert ihn. Diese Limits schützen nicht verteilt
gegen Missbrauch.

## Aktuelle Grenze

Die eingebaute Liveness-Route stellt keine Anwendungs- oder Abhängigkeits-
Readiness bereit. Readiness-Routen bleiben Aufgabe der Anwendung und
benötigen Abnahmetests in der erzeugten Businessanwendung. Handler-Abbruch und
verteilte Ratenlimits werden ebenfalls nicht bereitgestellt.
