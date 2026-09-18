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

The generated list can also use a safe card layout and a custom empty state:

~~~zelyra
crud Customer -> customers {
    view {
        list {
            mode: cards
            empty: "No customers found."
        }
    }
}
~~~

`mode` accepts `table` (the default) or `cards`. This changes only the
presentation. The same checked MariaDB query, search, typed filters,
allowlisted sorting, pagination, URL state, HTML escaping, and authorization
guards remain active. Detail and form overrides remain future work.

Detail views now support a controlled card layout and an explicit heading:

~~~zelyra
crud Customer -> customers {
    view {
        detail {
            mode: cards
            title: "Customer details"
        }
    }
}
~~~

The default is `standard`. Generated edit, create, delete, CSRF, escaping, and
authorization safeguards remain active.

CRUD forms support the same controlled layout, heading, and submit label:

~~~zelyra
crud Customer -> customers {
    view {
        form {
            mode: cards
            title: "Customer form"
            submit: "Save customer"
        }
    }
}
~~~

The title and submit label are escaped. Schema validation, readonly checks,
CSRF protection, parameter binding, and action permissions remain active.

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

Configured relationship filters keep the logical query name as well. For the
example above, use `filter_department=<department-id>`; Zelyra maps that value
to the stored `department_id` column internally.

Each row links to a generated detail route, for example `GET /machines/1`.
The detail view offers generated Create and Edit forms at `/machines/new` and
`GET/POST /machines/1/edit`. These forms inherit schema validation,
relationship selects, CSRF protection, prepared parameters, and MariaDB
transactions. Edit forms are prefilled from the selected row.

The detail view also contains a CSRF-protected delete confirmation. A valid
`POST /machines/1/delete` executes a parameterized MariaDB delete in a
transaction and redirects to `/machines`; an invalid token is rejected with
HTTP 403 and the record is left unchanged.

CRUD authorization can use separate permissions for each operation:

~~~zelyra
crud Customer -> customers {
    requires auth
    permits "customers.view"
    permits create "customers.create"
    permits edit "customers.edit"
    permits delete "customers.delete"
}
~~~

The unscoped `permits` form remains the backward-compatible view/default
permission. It protects list and detail routes and is used as the fallback
for Create, Edit, and Delete when no scoped permission is declared. Scoped
permissions protect the generated Create and Edit forms as well as the Delete
endpoint independently.

The generated list and detail views also hide action links and buttons when the
current session lacks the corresponding permission. This is a usability
feature, not the security boundary: direct requests are still checked and
receive HTTP 401 or 403 as appropriate.

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

The repository integration test `tests/mariadb-e2e.sh` exercises the complete
vertical slice against MariaDB: schema setup and inspection, related CRUD
creation, search, exact relationship and boolean filters, allowlisted sorting,
pagination, validation of unknown query fields, editing, CSRF-protected
deletion, and cleanup.
