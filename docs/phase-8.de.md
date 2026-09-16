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
geschützte Routen standardmäßig. Für den aktuellen Bootstrap-Adapter akzeptiert
der Server ein Bearer-Token nur, wenn ZELYRA_AUTH_TOKEN ausdrücklich gesetzt
ist. Die serverseitige Berechtigungsliste wird über
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

Dies ist bewusst nur ein erster Authentifizierungsadapter und noch nicht das
fertige Users-/Sessions-System. Login, Logout, Passwort-Hashing, sichere
Session-Cookies, Rollen und datenbankgestützte Berechtigungsabfragen folgen als
nächste Authentifizierungsschritte.
