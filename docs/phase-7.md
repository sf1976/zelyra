# Zelyra 0.1 — Phase 7: CRUD

[Deutsch](phase-7.de.md) · English

Phase 7 starts CRUD as a language-level abstraction. The first vertical slice
supports:

~~~zelyra
crud Machine -> machines
~~~

With a MariaDB `DATABASE_URL`, this exposes `GET /machines`. The generated
list currently includes all schema columns in an escaped HTML table, search
across text columns with bound parameters, exact filters through
`filter_<column>`, allowlisted sorting through `sort` and `order`, and bounded
pagination through the `page` and `per_page` query parameters. Unknown sort or
filter columns are rejected with HTTP 400.

Each row links to a generated detail route, for example `GET /machines/1`.
The detail view offers generated Create and Edit forms at `/machines/new` and
`GET/POST /machines/1/edit`. These forms inherit schema validation,
relationship selects, CSRF protection, prepared parameters, and MariaDB
transactions. Edit forms are prefilled from the selected row.

Missing database configuration returns HTTP 503. Query failures return a
generic HTTP 500. CRUD resource and table names are checked before the server
starts.

The next steps are configured columns and delete actions.
The complete relationship-aware example is `examples/machine_form.zyl`.

~~~bash
export DATABASE_URL='mariadb://root:<password>@127.0.0.1:3306/zelyra_crud'
zelyra db bootstrap examples/machine_form.zyl
zelyra serve examples/machine_form.zyl
~~~

Open http://127.0.0.1:3000/machines.
