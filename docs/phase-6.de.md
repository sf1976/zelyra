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

Formulare werden vom eingebauten Server automatisch unter
/forms/FormName bereitgestellt. GET rendert ein schemaabhängiges HTML-Formular
mit einem CSRF-Token pro Serverstart. POST parst URL-encoded Eingaben, prüft
das Token, validiert die Felder und rendert bei Fehlern feldbezogene Meldungen
mit HTTP 422. Ein Formular ohne Aktion liefert nach erfolgreicher Validierung
HTTP 202.

Formularaktionen können nach erfolgreicher Validierung natives SQL ausführen:

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

Der eingebaute Server liest `DATABASE_URL`, bindet deklarierte Felder als
Prepared-Statement-Parameter und führt alle SQL-Anweisungen der Aktion in
einer MariaDB-Transaktion aus. Bei Erfolg wird HTTP 303 geliefert. Eine
fehlende Konfiguration liefert HTTP 503, ein Ausführungsfehler einen
allgemeinen HTTP-500-Fehler. Das Aktions-SQL wird vor dem Serverstart gegen das
Quellschema geprüft.

Sessions und die dauerhafte Verwaltung von CSRF-Geheimnissen bleiben weitere
Integrationsschritte.

Beziehungsfelder werden jetzt als datenbankgestützte Select-Felder gerendert.
Beispielsweise wird `department: Department required` automatisch auf die
Tabelle `departments` abgebildet, verwendet standardmäßig `name` als
Anzeigespalte und prüft die übermittelte ID vor der Aktionsausführung gegen die
aktuellen MariaDB-Datensätze. Das vollständige Beispiel ist
`examples/machine_form.zyl`.
