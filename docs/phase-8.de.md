# Zelyra 0.1 — Phase 8: Authentifizierung und Autorisierung

Deutsch · [English](phase-8.md)

Phase 8 besitzt jetzt die erste Sicherheitsgrenze. Eine Authentifizierung kann
gegen eine Benutzertabelle deklariert werden:

~~~zelyra
auth users {
    table: users
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
Eine optionale active-Spalte deaktiviert inaktive Benutzer.

Ein erfolgreicher Login erzeugt ein HttpOnly-SameSite-Session-Cookie. Logout
ist als POST /logout verfügbar. Sessions werden in dieser ersten
Implementierung im Prozessspeicher gehalten und beim Serverstopp ungültig.

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

Dies ist die erste funktionierende Authentifizierungsscheibe.
Datenbankgestützte Rollen- und Berechtigungsabfragen, persistente Sessions,
Login-Drosselung und ein eigenes Passwortverwaltungs-Kommando folgen als
nächste Authentifizierungsschritte.
