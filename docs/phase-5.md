# Zelyra 0.1 — Phase 5: Web Core

[Deutsch](phase-5.de.md) · English

Phase 5 begins the Web Core with a small usable HTTP server. A page can be
declared directly in a Zelyra source file:

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

Start it from a source checkout:

~~~bash
./install.sh
zelyra serve examples/hello_web.zyl
~~~

The default address is 127.0.0.1:3000. A different bind address can be passed
as the second argument:

~~~bash
zelyra serve examples/hello_web.zyl 127.0.0.1:8080
~~~

Routes may contain path parameters in the form {name}. They are available in
the initial HTML template using the same name. Values are HTML-escaped by
default, so a route value cannot directly inject markup.

This first Web Core step supports GET routes, path parameters, query-string
stripping, basic HTTP request parsing, and HTML responses. Sessions, cookies,
CSRF protection, forms, static files, API declarations, and live
database-backed page data are subsequent phases.

Apache is not required for local development or this standalone server.
