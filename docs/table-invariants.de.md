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

Für eine Tabelle mit Mandantenscope muss die API `requires auth` angeben und
ein `auth`-Block eine aktive Mitgliedschaftstabelle konfigurieren. Der
Mandantenschlüssel gehört nicht in den API-`input`-Block. Zelyra prüft den
optionalen `tenant_id`-Auswahlschlüssel gegen die authentifizierte Sitzung und
aktive Mitgliedschaften, übergibt den geprüften Mandanten an die Regelprüfung
und als letzten Handlerparameter mit dem Namen und Typ der deklarierten
Mandantenspalte:

```zelyra
type TenantId = Id

api POST "/api/invoice-check" {
    requires auth
    handler check_invoice
    validates invoices
    input { amount: Int }
    output Result<TenantId, String>
    errors { 422 ValidationError: String }
}

fn check_invoice(amount: Int, tenant_id: TenantId) -> Result<TenantId, String> {
    return Ok(tenant_id)
}
```

Ein vom Client übergebener Mandantenschlüssel im Request-Body wird abgewiesen.
Fehlende, widersprüchliche und nicht autorisierte Mandantenauswahl scheitert
vor dem Handler. Die Prüfung umfasst nur deklarierte Eingaben und den
verifizierten Mandantenkontext. Sie ist keine Datenbank-Constraint: Rohe SQL-
Schreibzugriffe und nicht gebundene Handler sind nicht erfasst. Invarianten,
die für jeden Schreiber gelten müssen, benötigen weiterhin passende
Datenbank-Constraints. Ein vollständiges compilergeprüftes Beispiel steht in
[`examples/tenant_api_validation.zyl`](../examples/tenant_api_validation.zyl).
