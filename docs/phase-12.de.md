# Zelyra 0.1 — Phase 12: Request-Validierung und sichere Antwort-Defaults

[English](phase-12.md) · Deutsch

Die API-Verarbeitung besitzt jetzt eine kleine, vorhersehbare
Validierungsgrenze. Bodies bei API-Anfragen außer `GET`/`DELETE` dürfen
`application/json` oder `application/x-www-form-urlencoded` verwenden.
Nicht unterstützte Medientypen liefern `415 Unsupported Media Type` als
strukturierten JSON-Fehler. JSON-Bodies müssen weiterhin ein Objekt sein;
danach wird jedes deklarierte Feld in den Zelyra-Typ umgewandelt und geprüft.

Der HTTP-Parser prüft `Content-Length`, lehnt ungültige Werte und unvollständige
Bodies ab und begrenzt Request-Bodies auf ein Mebibyte. Ein zu großer Body
führt zu `413 Payload Too Large`. Die Prüfung erfolgt vor der Ausführung des
API-Handlers.

Antworten enthalten diese sicheren Standard-Header:

~~~http
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Referrer-Policy: no-referrer
~~~

Die Defaults gelten für HTML, JSON, Weiterleitungen, Fehler und leere
Preflight-Antworten. TLS, Authentifizierung, Autorisierung, CSRF-Schutz oder
eine für den Einsatz geeignete Content-Security-Policy werden dadurch nicht
ersetzt.

Das CORS-Verhalten aus [Phase 11](phase-11.de.md) bleibt unverändert.
Request-Validierung ist unabhängig von CORS; auch direkte Nicht-Browser-
Clients erhalten dasselbe Validierungsverhalten.
