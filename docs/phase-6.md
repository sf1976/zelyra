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

Form actions are parsed together with native SQL, success messages, and
redirect targets. Rendering HTML forms, CSRF tokens, relationship select
controls, and executing a validated action are the next integration steps.
