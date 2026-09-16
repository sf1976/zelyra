# Zelyra 0.1 — Phase 5: Web Core

Deutsch · [English](phase-5.md)

Phase 5 beginnt den Web Core mit einem kleinen nutzbaren HTTP-Server. Eine
Seite kann direkt in einer Zelyra-Quelldatei definiert werden:

~~~zelyra
page "/hello/{name}" {
    html {
        <html>
            <body>
                <h1>Hello, {name}!</h1>
            </body>
        </html>
    }
}
~~~

Start aus einem ausgecheckten Quellcode:

~~~bash
./install.sh
zelyra serve examples/hello_web.zyl
~~~

Die Standardadresse ist 127.0.0.1:3000. Eine andere Bind-Adresse kann als
zweites Argument angegeben werden:

~~~bash
zelyra serve examples/hello_web.zyl 127.0.0.1:8080
~~~

Routen dürfen Pfadparameter in der Form {name} enthalten. Diese stehen im
ersten HTML-Template unter demselben Namen zur Verfügung. Werte werden
standardmäßig HTML-escaped, sodass ein Routenwert nicht direkt Markup
einschleusen kann.

Dieser erste Web-Core-Schritt unterstützt GET-Routen, Pfadparameter, das
Entfernen von Query-Strings für das Routing, grundlegendes HTTP-Request-Parsing
und HTML-Responses. Sessions, Cookies, CSRF-Schutz, Formulare, statische
Dateien, API-Definitionen und datenbankgestützte Seiten folgen in späteren
Phasen.

Für lokale Entwicklung oder diesen eigenständigen Server ist Apache nicht
erforderlich.
