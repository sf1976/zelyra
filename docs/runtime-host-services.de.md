# Runtime-Hostservices (experimentell)

Zelyras Runtime kann ausgewählte Betriebssystemdienste über eine injizierte
Schnittstelle beziehen. Der Vertrag ist als versionierte Rust-API für
Embedding-Code gedacht, nicht als stabiler nativer ABI- oder Pluginvertrag.
`HOST_SERVICES_API_VERSION` steht auf `7`. Version 2 ergänzte injizierbare
Prozessausführung, Version 3 Dateisystemdienste, Version 4 Datenbankdienste und
Version 5 Konsoleneingabe, Version 6 HTTP-Transport und Version 7 Mailzustellung.

## Unterstützte Dienste

- Der MIR-Interpreter unterstützt `http_get`, `http_request`, `http_json` und
  `http_result` über denselben Runtime-Transport und dieselben URL-, Allowlist-,
  Header-, Timeout-, Antwortgrößen- und UTF-8-Prüfungen wie der AST-Interpreter.
  Er verwendet außerdem die gemeinsamen JSON-Codecs `encode_json_value`,
  `decode_json_value`, `encode_tagged_result_json` und
  `decode_tagged_result_json`. `MirProgram` führt dafür die typgeprüften
  Record-, Alias- und Enumdefinitionen mit. Fake-Transporttests vergleichen AST
  und MIR ohne externe Netzwerkanfragen.
- MIR bindet `mail_deliver` über dieselbe redigierte SMTP-Policy-Grenze an.
  `Email`, SMTP-Konfiguration und Netzwerk-Allowlist werden vor dem Hostaufruf
  geprüft. Fake-Hosttests vergleichen AST und MIR ohne Mailversand. MIR unterstützt
  `mail_queue` mit optionalen PDF-Anhängen in einer Datenbanktransaktion. AST-
  Renderer (`render_pdf_view`), Payloadvalidator und Outbox-Encoder werden geteilt.
  Ein Differentialtest mit Fake-Host vergleicht den Outbox-Eintrag, ohne Mail zu
  versenden.
- MIR führt die unterstützte Teilmenge eigenständiger `sql<T>`-Abfragen und
  SQL-/Outbox-`transaction { ... }`-Blöcke über den injizierten Datenbankhost aus.
  HIR erfasst SQL-Operation, deterministisch geordnete Tabellenzugriffe und
  benannte Platzhalter mit lexikalischen LocalIds und inferierten Typen. MIR
  prüft diesen Plan erneut gegen die SQL-Quelle (`E-MIR-VERIFY-004`); der
  gemeinsame Runtime-Scanner bindet ausschließlich erfasste Werte. Diese
  Analysemetadaten vergeben keine Tabellenberechtigungen. Transaktionen
  übergeben alle geprüften SQL-Anweisungen nach gemeinsamer Schema- und
  Speicherprüfung in genau einem `execute_database_batch(..., true)`-Aufruf.
  Datenbank-Capabilities und Projektfreigaben werden vor dem Provideraufruf
  geprüft. Differentiale Fake-Hosttests prüfen Bindung, atomare Batchauswahl,
  Providerfehler, Speicherprüfung, fehlenden Datenbankkontext und verweigerte
  Zugriffe ohne echte Datenbank. Ein Outbox-Aufruf mit gerendertem PDF-Anhang wird gemeinsam mit SQL in einem
  Transaktionsbatch getestet; der MIR-Interpreter versendet keine E-Mail.


`HostServices` stellt aktuell neun Dienstgruppen bereit:

- `unix_time_millis()` für `now()`; Rückgabewerte außerhalb des Zelyra-
  Timestampbereichs werden abgewiesen.
- `environment_variable(name)` für `env(name)`; Werte müssen UTF-8 sein.
- `fill_random(buffer)` für `random_int(...)` und `uuid_v4()`; die Runtime
  übernimmt Bereichsprüfung und UUID-Formatierung.
- `read_console_line(prompt)` für `read_console(...)`; der Runtime-Capability-
  Check erfolgt vor dem Aufruf. Die Systemimplementierung serialisiert den
  Zugriff auf stdin und schreibt den Prompt nach stdout. Eingebettete Hosts
  ohne Konsoleneingabe lehnen den Aufruf ausdrücklich ab.
- `http_request(method, url, headers, body, timeout_ms, max_response_bytes)`
  für `http_get`, `http_request`, `http_json` und `http_result`. Die Runtime
  prüft Capability, URL-Schema, Host-Allowlist und Header vor dem Aufruf und
  prüft Antwortgröße sowie UTF-8 danach erneut. Request- und Response-Header
  sind jeweils auf 65.536 Bytes begrenzt. Timeout und Antwortbudget müssen
  positiv sein. Der Systemhost deaktiviert Weiterleitungen und begrenzt die
  gelesene Antwort auf die konfigurierte Bytezahl.
- `run_process(command, arguments, timeout_ms, max_output_bytes)` für
  `run_process(...)`; die Runtime prüft Capability, Allowlist und positive
  Grenzen, bevor sie den Host aufruft. Die Systemimplementierung verwendet
  keine Shell, leert die Prozessumgebung, erfasst stdout/stderr begrenzt und
  beendet Zeitüberschreitungen.
- Dateisystemdienste für Pfadkanonisierung, Metadaten, Text lesen/schreiben,
  exklusives und atomares Schreiben, Anhängen, Kopieren, Umbenennen, Löschen
  und Verzeichnislisten. Die Runtime prüft nach der hostseitigen
  Pfadkanonisierung weiterhin, dass der Pfad innerhalb der konfigurierten
  Roots liegt. Capability und Projektfreigaben werden vor Providerzugriffen
  geprüft. Die Standardkanonisierung nutzt das Systemdateisystem; virtuelle
  Hosts müssen die Methode selbst überschreiben und Symlink-/Traversal-Schutz
  erhalten. Nicht implementierte optionale I/O-Dienste schlagen ausdrücklich
  fehl.
- `deliver_outbox_message(...)` für den SMTP-Transport von Outbox-Nachrichten.
  Die Runtime prüft Email-Capability und SMTP-Host-Allowlist vor dem Aufruf;
  der Systemhost validiert Payload und Mailkonfiguration erneut und delegiert
  den Versand an `zelyra-mail`.
- Datenbankabfragen, Transaktionsbatches und die erforderliche Prüfung von
  Decimal-/Temporal-Speicherregeln. Die Runtime prüft SQL-Capabilities, bevor
  der Host die Datenbank erhält. SQL bleibt bereits geparst und gebunden; der
  Host erhält Query-Objekte mit getrennten Parametern. Storage-Prüfungen lesen
  Metadaten und führen keine Schemaänderungen aus. Der Systemhost delegiert an
  `zelyra-database`; ein Host ohne Datenbankdienst schlägt bei solchen Abfragen
  ausdrücklich fehl.

Bestehende `execute`-APIs verwenden `SystemHostServices` und behalten damit
ihre bisherige Systemuhr, Umgebung, kryptografische Zufallsquelle und
Konsoleneingabe. Hosts können für Einbettung und Tests injiziert werden:

```rust,ignore
let host: Arc<dyn HostServices> = Arc::new(MyHost);
let output = execute_with_host_services(
    &program,
    host,
    Some(&grants),
    Some(&policy),
)?;
```

Die echten Funktionssignaturen stehen in `runtime/src/lib.rs`. Der Dienst muss
`Send + Sync` erfüllen, weil parallele Zelyra-Ausdrücke dieselbe Hostinstanz
verwenden.

Für eine Einbettung, die bereits eine Request-Transaktion besitzt, bietet die
AST-Runtime außerdem `execute_function_captured_with_transactional_query`.
Jeder `sql<T>`-Ausdruck wird dabei über den geliehenen Query-Callback
ausgeführt; Capability- und Storage-Prüfungen bleiben aktiv. Der Callback ist
auf eine begrenzte Runtime-Ausführung beschränkt. Verschachtelte
`transaction { ... }`- und `parallel { ... }`-Blöcke werden in diesem Modus
abgewiesen, damit sie nicht auf eine andere Verbindung ausweichen. Dies ist
eine niedrigstufige Einbettungs-API; CRUD-Web-Actions verwenden sie noch nicht.

## Sicherheitsgrenze und Status

Zelyras Funktions-Capabilities und Projektfreigaben werden vor jedem dieser
Provideraufrufe geprüft. Ein Host kann diese Prüfungen nicht durch eine
Capability-Deklaration umgehen. Providerfehler werden als Laufzeitdiagnostik
mit dem ursprünglichen Quellspan zurückgegeben. Tests verwenden einen
deterministischen Provider und prüfen auch, dass bei verweigerter Capability
kein Hostaufruf erfolgt.

AST, MIR und die versionierte Bytecode-VM leiten ihre jeweils unterstützten
externen Effekte über `HostServices`. Die derzeit getestete Bytecode-Teilmenge
umfasst Uhr, Umgebung, Konsole, Zufall, allowlist-geprüfte Prozesse,
Dateisystemoperationen mit Root-Prüfung, geprüfte HTTP-Aufrufe, Mailzustellung
sowie ausgewählte SQL- und transaktionale Outbox-Abläufe. Der Bytecode-Verifier
serialisiert deklarierte Capabilities und prüft Projektfreigaben vor
Hostaufrufen; `execute` erteilt standardmäßig keine Freigaben. MIR und Bytecode
unterstützen weiterhin nur dokumentierte Teilmengen; manche Aufrufe benötigen
eine ausdrückliche `RuntimePolicy`. Nicht unterstützte Operationen scheitern
geschlossen. Diese API ist experimentell: eine versionierte Rust-
Embedding-Schnittstelle, keine stabile native ABI und keine Paket-Plugin-ABI.
Den konkreten Operationsumfang und die verbleibenden Paritätsgrenzen nennt der
Bytecode-Leitfaden.
