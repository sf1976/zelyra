# Zelyra 0.1 — Phase 7: CRUD

[Deutsch](phase-7.de.md) · English

Phase 7 starts CRUD as a language-level abstraction. The current vertical
slice supports:

~~~zelyra
crud Machine -> machines
~~~

CRUD resources can now configure their title, visible list columns, searchable
columns, and filter columns:

~~~zelyra
crud Machine -> machines {
    title: "Machines"
    list { number name department active }
    search { number name }
    filter { department active }
}
~~~

The blocks are optional. Without them, Zelyra keeps the safe defaults:
all schema columns in the list, text columns for search, and all non-ID
columns for filters. Configured names are checked against the schema before
the server starts; relationship fields such as department resolve to their
stored foreign-key column automatically.

With a MariaDB `DATABASE_URL`, this exposes `GET /machines`. The generated
list currently includes all schema columns in an escaped HTML table, search
across text columns with bound parameters, exact filters through
`filter_<column>`, allowlisted sorting through `sort` and `order`, and bounded
pagination through the `page` and `per_page` query parameters. Unknown sort or
filter columns are rejected with HTTP 400.

Foreign-key columns are joined automatically for presentation. For example,
the stored `department_id` is shown as the department's `name` (`Production`)
in lists and detail views; the generated form continues to submit the
validated foreign-key ID. Relationship sort and filter labels use the logical
field name while filtering remains an exact, parameterized ID comparison.

Each row links to a generated detail route, for example `GET /machines/1`.
The detail view offers generated Create and Edit forms at `/machines/new` and
`GET/POST /machines/1/edit`. These forms inherit schema validation,
relationship selects, CSRF protection, prepared parameters, and MariaDB
transactions. Edit forms are prefilled from the selected row.

The detail view also contains a CSRF-protected delete confirmation. A valid
`POST /machines/1/delete` executes a parameterized MariaDB delete in a
transaction and redirects to `/machines`; an invalid token is rejected with
HTTP 403 and the record is left unchanged.

Missing database configuration returns HTTP 503. Query failures return a
generic HTTP 500. CRUD resource and table names are checked before the server
starts.

The first authorization boundary is now documented in
[Phase 8](phase-8.md). The next step there is database-backed login and
secure sessions.
The complete relationship-aware example is `examples/machine_form.zyl`.

~~~bash
export DATABASE_URL='mariadb://root:<password>@127.0.0.1:3306/zelyra_crud'
zelyra db bootstrap examples/machine_form.zyl
zelyra serve examples/machine_form.zyl
~~~

Open http://127.0.0.1:3000/machines.
