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
guards remain active. Detail, form, loading, and error overrides are available
as well.

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

The delete confirmation can also be configured:

~~~zelyra
crud Customer -> customers {
    view {
        delete {
            title: "Delete customer"
            message: "This cannot be undone."
            submit: "Delete now"
        }
    }
}
~~~

The generated endpoint remains POST-only and continues to enforce CSRF and
delete authorization checks.

CRUD loading and error states can also be configured:

~~~zelyra
crud Customer -> customers {
    view {
        loading { message: "Loading customers..." }
        error {
            title: "Customer unavailable"
            message: "Please try again later."
        }
    }
}
~~~

The loading message is emitted as escaped metadata for progressive enhancement;
the server-rendered response does not claim that loading is active. Configured
error messages replace generic CRUD database-error pages without exposing
internal database details.

Custom CRUD actions add focused business operations to a resource:

~~~zelyra
crud Customer -> customers {
    action deactivate {
        label: "Deactivate customer"
        confirm: "Deactivate this customer?"
        permits "customers.edit"
        sql {
            UPDATE customers
            SET active = false
            WHERE id = :id
        }
        success "Customer deactivated."
        redirect "/customers"
    }
}
~~~

The compiler generates a POST-only route at `/customers/{id}/deactivate` and
adds a button to the detail view. The route enforces the database capability,
CSRF, authentication, and declared permissions. The route `id` is bound to
`:id`; SQL remains parameterized. Action names are currently used as labels.
`label` overrides the escaped button text, while `confirm` adds an escaped
browser confirmation before submission. Without `label`, the action name is
used as the label.

Actions may declare typed fields as well. These fields use the normal form
validation and are bound as SQL parameters:

~~~zelyra
action set_active {
    icon: "check"
    field active: Bool { required }
    sql {
        UPDATE customers SET active = :active WHERE id = :id
    }
}
~~~

An action `icon` is exposed as an escaped `data-icon` hook on its generated
button. Its `success` message is carried to the redirect as a status query
value and rendered escaped with `role="status"` on CRUD lists.

Relationship fields such as `department: Department` are rendered as checked
select fields on the detail view. Options come from the referenced MariaDB
table, and submitted IDs are validated against the current option set before
the action SQL executes.

Riskier actions can request a server-rendered confirmation page:

~~~zelyra
action move_department {
    confirm_page {
        title: "Confirm department change"
        message: "Move this machine?"
        submit: "Move machine"
    }
    field department: Department { required }
    sql { UPDATE machines SET department_id = :department WHERE id = :id }
}
~~~

The detail button becomes a GET link. The confirmation page renders the typed
fields and a fresh CSRF-protected POST form; authorization is checked on both
requests.

Use `success_page` for a structured escaped success title and message, and
`error_page` for a safe action-specific failure page. Database error details
remain server-side and are never rendered into the response.

CRUD resources may opt into reversible soft deletion with a nullable timestamp
column:

~~~zelyra
table customers {
    id: Id primary auto
    name: String(100) required
    deleted_at: Timestamp?
}

crud Customer -> customers {
    soft_delete { column: deleted_at }
}
~~~

Generated delete requests set the configured column to `CURRENT_TIMESTAMP`.
Normal lists and details include only rows where it is `NULL`; the generated
archive link uses `?archived=true`. Archived details expose a CSRF-protected
POST restore route. The marker is excluded from generated Create/Edit forms and
default list/filter columns. Permanent purge and retention policies remain
planned.

The blocks are optional. Without them, Zelyra keeps the safe defaults:
all applicable schema columns in the list, text columns for search, and all
non-ID columns for filters. A configured soft-delete marker is excluded from
the generated defaults. Configured names are checked against the schema before
the server starts; relationship fields such as department resolve to their
stored foreign-key column automatically.

With a MariaDB `DATABASE_URL`, this exposes `GET /machines`. The generated
list currently includes all applicable schema columns in an escaped HTML table, search
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

The same test starts the real application in all four combinations of
`ZELYRA_LANGUAGE=en|de` and `ZELYRA_LEVEL=learn|work`. It checks localized
machine and department views and verifies that the contextual learning guide
appears in `learn` and is absent in `work`. When the fictional demo fixture is
enabled, the German views are also checked against its example records.
