# Zelyra 0.1 — Phase 8: Authentifizierung und Autorisierung

Deutsch · [English](phase-8.md)

Phase 8 besitzt jetzt die erste Sicherheitsgrenze. Eine Authentifizierung kann
gegen eine Benutzertabelle deklariert werden:

~~~zelyra
auth users {
    table: users
    sessions: auth_sessions
    permissions: user_permissions
}
~~~

Seiten und CRUD-Ressourcen können Authentifizierung und Berechtigungen
verlangen:

~~~zelyra
page "/admin" {
    requires auth
    permits "admin.view"

    html {
        <h1>Admin</h1>
    }
}
~~~

Der Compiler prüft, ob die konfigurierte Benutzertabelle existiert und ob
geschützte Routen eine Auth-Definition besitzen. Die Weblaufzeit verweigert
geschützte Routen standardmäßig. Er stellt jetzt einen datenbankgestützten
Login unter /login bereit. Die konfigurierte Benutzertabelle muss die Spalten
id, email und password_hash besitzen; password_hash-Werte verwenden Argon2.
Eine optionale active-Spalte deaktiviert inaktive Benutzer. Fehlgeschlagene
Loginversuche werden pro normalisierter E-Mail-Adresse gezählt. Nach fünf
Fehlern innerhalb von 15 Minuten werden weitere Versuche 60 Sekunden lang mit
HTTP 429 und dem Header `Retry-After: 60` abgelehnt.

Ein erfolgreicher Login erzeugt ein HttpOnly-SameSite-Session-Cookie und rotiert
ein vorhandenes Session-Token dieses Browsers. Wenn die
optionale Session-Tabelle konfiguriert ist, wird nur ein Blake2s-256-Hash des
Session-Tokens in MariaDB gespeichert; das Cookie selbst wird nie in der
Datenbank gespeichert. Sessions laufen nach 24 Stunden ab und Logout entfernt
den Datenbankeintrag. Ohne sessions-Option verwendet der ausdrückliche
Entwicklungs-Fallback den Prozessspeicher.

Wenn die optionale Berechtigungstabelle konfiguriert ist, muss sie die Spalten
user_id und permission enthalten. Für jede geschützte Anfrage werden die
Berechtigungen aus dieser Tabelle geladen. Ein authentifizierter Benutzer ohne
passende Berechtigung erhält HTTP 403. Ohne permissions-Option bleibt die
ausdrückliche ZELYRA_AUTH_PERMISSIONS-Allowlist für lokale Entwicklung und
Reverse-Proxy-Deployments verfügbar.

Für Deployments, die Benutzer in einem Reverse Proxy authentifizieren,
akzeptiert der Server zusätzlich ein Bearer-Token, wenn ZELYRA_AUTH_TOKEN
ausdrücklich gesetzt ist. Die serverseitige Berechtigungsliste wird über
ZELYRA_AUTH_PERMISSIONS gesetzt, zum Beispiel:

~~~bash
export ZELYRA_AUTH_TOKEN='local-development-secret'
export ZELYRA_AUTH_PERMISSIONS='admin.view,customers.view'
zelyra serve examples/auth.zyl
~~~

Das Token wird mit einer Anfrage gesendet:

~~~bash
curl -H 'Authorization: Bearer local-development-secret' \
  http://127.0.0.1:3000/admin
~~~

Ohne Token liefert die Route HTTP 401. Mit gültigem Token, aber ohne deklarierte
Berechtigung, liefert sie HTTP 403. Tokens und Berechtigungen werden vom
Compiler niemals im Quelltext gespeichert.

Den für die Spalte `password_hash` erforderlichen Argon2-Wert erzeugt man mit:

~~~bash
zelyra auth hash-password
~~~

Der interaktive Befehl zeigt das Passwort nicht an und verlangt eine
Bestätigung. Für bewusste Automatisierung liest
`zelyra auth hash-password --stdin` eine Passwortzeile von der Standardeingabe.
Echte Passwörter nicht als Kommandoargument oder in der Versionsverwaltung
ablegen.

Dies ist die erste funktionierende Authentifizierungsscheibe mit persistenten
Sessions und datenbankgestützter Berechtigungsabfrage. Login-Drosselung und
Session-Rotation sind implementiert; Datenbankrollen bleiben eine zukünftige
Authentifizierungsaufgabe.

Der Repository-Test `tests/mariadb-auth-e2e.sh` prüft diesen Ablauf gegen
MariaDB mit zwei temporären Benutzern: anonymer Zugriff wird abgelehnt, falsche
Zugangsdaten schlagen fehl, der berechtigte Benutzer erhält eine persistente
Session und erreicht `/admin`, ein eingeloggter Benutzer ohne `admin.view`
erhält HTTP 403, und Logout entfernt die Session aus MariaDB.

Das kombinierte Beispiel `examples/auth_crud_api.zyl` verwendet dieselbe
Sicherheitsgrenze sowohl für eine CRUD-Ressource als auch für eine typisierte
API:

~~~zelyra
crud Customer -> customers {
    requires auth
    permits "customers.view"
}

api GET "/api/customers/{id}" {
    handler get_customer
    requires auth
    permits "customers.view"
    input { id: CustomerId }
    output Customer
}
~~~

Der Integrationstest `tests/mariadb-protected-e2e.sh` prüft, dass anonyme
Anfragen HTTP 401 erhalten, eine berechtigte Session beide Endpunkte mit HTTP
200 erreicht und ein eingeloggter Benutzer ohne `customers.view` sowohl für
die HTML-CRUD-Route als auch für die JSON-API HTTP 403 erhält. Zusätzlich
werden die getrennten Berechtigungen `customers.create`,
`customers.edit` und `customers.delete` an den erzeugten
CRUD-Aktionen geprüft.
