# Zelyra 0.1 — Phase 6: Forms

Deutsch · [English](phase-6.md)

Phase 6 führt Formulare als Sprachelemente ein. Ein Formular kann die Felder
und Einschränkungen einer Datenbanktabelle wiederverwenden:

~~~zelyra
form CustomerCreate -> customers {
    fields {
        name
        email
    }
}
~~~

Felder können auch ausdrücklich definiert und angepasst werden:

~~~zelyra
form CustomerForm {
    field email: Email {
        label: "E-Mail"
        required
        max: 255
        widget: email
    }
}
~~~

Der Formular-Checker prüft, ob schemaabhängige Felder existieren und ob
eigenständige Felder einen expliziten Typ besitzen. Die Validierungsbibliothek
prüft Pflichtwerte, maximale Längen, Ganzzahlen, boolesche Werte, Email, Url,
IDs und unbekannte Eingabefelder.

Die Validierung kann über die CLI getestet werden:

~~~bash
zelyra form validate examples/customer_form.zyl CustomerCreate \
  name=Anna email=anna@example.test
~~~

Bei erfolgreicher Prüfung wird valid ausgegeben. Ungültige Eingaben liefern
feldbezogene Diagnosen. Dieser Befehl arbeitet lokal und benötigt keine aktive
Datenbankverbindung.

Form-Aktionen werden zusammen mit nativem SQL, Erfolgsmeldungen und
Weiterleitungszielen geparst. Das Rendern von HTML-Formularen, CSRF-Tokens,
Select-Feldern für Beziehungen und das Ausführen validierter Aktionen sind die
nächsten Integrationsschritte.
