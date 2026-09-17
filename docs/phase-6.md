# Zelyra 0.1 — Phase 6: Forms

[Deutsch](phase-6.de.md) · English

Phase 6 introduces forms as language-level definitions. A form can reuse the
fields and constraints of a database table:

~~~zelyra
form CustomerCreate -> customers {
    fields {
        name
        email
    }
}
~~~

Fields can also be declared explicitly and customized:

~~~zelyra
form CustomerForm {
    field email: Email {
        label: "E-mail"
        required
        max: 255
        widget: email
    }
}
~~~

The form checker verifies that schema-mapped fields exist and that standalone
fields have explicit types. The validation library checks required values,
maximum lengths, integers, booleans, Email, Url, identifiers, and unknown
input fields.

Validation can be exercised from the CLI:

~~~bash
zelyra form validate examples/customer_form.zyl CustomerCreate \
  name=Anna email=anna@example.test
~~~

Successful validation prints valid and invalid input returns field-specific
diagnostics. This command performs local validation and does not need a live
database connection.

Forms are exposed automatically by the built-in server at
/forms/FormName. GET renders a schema-aware HTML form with a per-server CSRF
token. POST parses URL-encoded input, verifies the token, validates the fields,
and renders field-specific errors with HTTP 422 when needed. A form without an
action returns HTTP 202 after validation.

Form actions can execute native SQL after successful validation:

~~~zelyra
form CustomerCreate -> customers {
    fields { name email }

    action save {
        sql {
            INSERT INTO customers (name, email)
            VALUES (:name, :email)
        }
        redirect "/customers"
    }
}
~~~

Form actions may declare their own authorization. The action permission is
checked when the form is rendered and again before submission. It is combined
with any permissions configured on the form route, so custom database actions
cannot bypass the application's authentication boundary:

~~~zelyra
form CustomerCreate -> customers {
    fields { name email }

    action save {
        requires auth
        permits "customers.save"
        redirect "/customers"
    }
}
~~~

The built-in server reads `DATABASE_URL`, binds declared fields as prepared
parameters, and executes all action SQL in one MariaDB transaction. Success
returns HTTP 303. Missing configuration returns HTTP 503 and an execution
failure returns a generic HTTP 500. Action SQL is checked against the source
schema before the server starts.

Sessions and persistent CSRF secret management remain future integration steps.

Relationship fields are now rendered as database-backed select controls. For
example, `department: Department required` maps to the `departments` table,
uses `name` as the display column by convention, and validates the submitted
ID against the current MariaDB rows before an action is executed. The complete
example is `examples/machine_form.zyl`.
