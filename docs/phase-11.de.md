# Zelyra 0.1 — Phase 11: Browser-API-Integration und CORS

[English](phase-11.md) · Deutsch

Zelyra-APIs können direkt von Browseranwendungen aufgerufen werden. Cross-
Origin-Zugriff ist standardmäßig deaktiviert. Ein Projekt muss erlaubte,
exakte Origins ausdrücklich in `zelyra.toml` angeben:

~~~toml
[web]
allowed_origins = ["http://localhost:5173", "https://app.example"]
allow_credentials = false
~~~

Origins müssen mit `http://` oder `https://` beginnen und dürfen keinen Pfad,
keine Query, kein Fragment, keinen Wildcard-Eintrag und keinen abschließenden
Slash enthalten. Wildcard-Origins werden bewusst nicht unterstützt. Dadurch
bleibt die Entwicklung einfach, ohne die API versehentlich für jede Website
zu öffnen.

Für eine erlaubte Origin erhalten API-Antworten `Access-Control-Allow-Origin`
und `Vary: Origin`. Wenn Zugangsdaten ausdrücklich aktiviert sind, kommt
`Access-Control-Allow-Credentials: true` hinzu. Das wird für Browser-
Session-Cookies benötigt; der Client muss Credentials trotzdem selbst
aktivieren.

Browser-Preflight-Anfragen werden automatisch verarbeitet:

~~~http
OPTIONS /customers HTTP/1.1
Origin: http://localhost:5173
Access-Control-Request-Method: POST
Access-Control-Request-Headers: content-type
~~~

Der Server antwortet mit `204 No Content`, den erlaubten Methoden, den
angeforderten Headern und einer begrenzten `Access-Control-Max-Age`. Eine
verbotene Origin oder Methode erhält einen strukturierten JSON-Fehler.
Methodenfehler bei API-Routen enthalten den `Allow`-Header.

Diese Phase fügt CORS-Header nur bei deklarierten API-Routen hinzu, nicht bei
Seiten, Formularen oder CRUD-Antworten. CORS umgeht weder Authentifizierung
noch Berechtigungsprüfungen.

Der erzeugte TypeScript-Client bleibt ohne zusätzliche Abhängigkeiten. Für
Session-Cookies darf sein `fetch`-Aufruf `credentials: "include"` nur dann
verwenden, wenn das Projekt Credential-CORS ausdrücklich aktiviert.
