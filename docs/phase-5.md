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

Named views provide the first composition layer for custom page design:

~~~zelyra
view SiteShell {
    html {
        <html><body><main><slot /></main></body></html>
    }
}

page "/customers" {
    view: SiteShell
    html { <h1>Customers</h1> }
}
~~~

Each named view must contain exactly one `<slot />`. The page's HTML is inserted
into that slot before the route is created. This keeps layout customization
separate from authentication, routing, and output escaping. Typed self-closing
components with declared properties are also available:

~~~zelyra
component Badge {
    props { text: String }
    html { <span class="badge">{text}</span> }
}

page "/status" {
    html { <Badge text="Ready" /> }
}
~~~

Multiple slots, nested component composition, styling, and CRUD view overrides
are planned next.

Apache is not required for local development or this standalone server.
