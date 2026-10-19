# Tabelleninvarianten

Tabellen können reine, typgeprüfte Regeln über mehrere Felder deklarieren.
Der Compiler senkt sie auf private `Bool`-Funktionen mit den deklarierten
Tabellenfeldern als Parametern ab. Die normale Typprüfung und Runtime werten
sie aus. Fehlermeldungen sind statische String-Literale und werden bei der
Ausgabe HTML-escaped.

```zelyra
table invoices {
    invoice_date: Date required
    due_date: Date?

    validate {
        match due_date {
            Some(due) => due >= invoice_date
            None => true
        }: "Das Fälligkeitsdatum darf nicht vor dem Rechnungsdatum liegen."
    }
}
```

Die Regeln laufen nach der normalen Feld- und Relationsprüfung und vor einer
deklarativen Formularaktion oder einem generierten CRUD-Schreibvorgang. Eine
fehlgeschlagene Regel liefert HTTP 422, erhält sichere Formulareingaben und
erscheint in der Formular-Fehlerübersicht. Regeln müssen `Bool` ergeben;
unbekannte Felder und Operationen mit Effekten weist der Compiler zurück. Jede
Regel läuft mit einem festen Budget von 10.000 Auswertungsschritten. Optionale
Felder sollten mit `match` behandelt werden, damit `Some` und `None` definiert
sind.

Eine API kann ihre geprüften JSON-Eingabefelder ausdrücklich an die Regeln
einer Tabelle binden. Vor dem Handler-Aufruf werden die typisierten Werte
geprüft; bei einem Verstoß antwortet die Route mit dem deklarierten HTTP-422-
Fehlervertrag. Das bindet die eingehenden Werte, nicht beliebige spätere
Transformationen des Handlers oder rohe SQL-Anweisungen.

~~~zelyra
table invoices {
    id: Id primary auto
    amount: Int required
    validate { amount > 0: "Der Betrag muss positiv sein." }
}

api POST "/invoices" {
    handler create_invoice
    input { amount: Int }
    validates invoices
    output Result<Int, String>
    errors { 422 ValidationError: String }
}
~~~

Die Bindung ist derzeit auf POST, PUT und PATCH sowie Tabellen ohne
Mandantenscope beschränkt. Für scoped Tabellen verweigert der Compiler die
Bindung, solange die API-Laufzeit keinen verifizierten Mandantenkontext an die
Prüfung übergeben kann. Die Prüfung ist keine Datenbank-Constraint: Rohe SQL-
Schreibzugriffe und andere nicht gebundene Handler umgehen sie. Invarianten,
die für jeden Schreiber gelten müssen, benötigen weiterhin passende
Datenbank-Constraints.
