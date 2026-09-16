# Zelyra 0.1 — Phase 7: CRUD

Deutsch · [English](phase-7.md)

Phase 7 beginnt CRUD als Sprachabstraktion. Der erste vertikale Schnitt
unterstützt:

~~~zelyra
crud Machine -> machines
~~~

Mit einer auf MariaDB zeigenden `DATABASE_URL` wird `GET /machines`
bereitgestellt. Die generierte Liste umfasst aktuell alle Schema-Spalten in
einer escaped HTML-Tabelle, Suche über Textspalten mit gebundenen Parametern
und begrenzte Pagination über die Query-Parameter `page` und `per_page`.

Jede Zeile verlinkt auf eine generierte Detailroute, beispielsweise
`GET /machines/1`. Die Detailansicht bietet automatisch erzeugte Create- und
Edit-Formulare unter `/machines/new` und `GET/POST /machines/1/edit`. Diese
Formulare übernehmen Schema-Validierung, Beziehungs-Selects, CSRF-Schutz,
gebundene Parameter und MariaDB-Transaktionen. Edit-Formulare werden mit dem
ausgewählten Datensatz vorausgefüllt.

Bei fehlender Datenbankkonfiguration wird HTTP 503 geliefert. Abfragefehler
werden als allgemeiner HTTP-500-Fehler ausgegeben. CRUD-Ressourcen und
Tabellennamen werden vor dem Serverstart geprüft.

Als Nächstes folgen konfigurierbare Spalten, Sortierung, Filter und die
Löschaktion. Das vollständige Beispiel mit Beziehung ist
`examples/machine_form.zyl`.

~~~bash
export DATABASE_URL='mariadb://root:<passwort>@127.0.0.1:3306/zelyra_crud'
zelyra db bootstrap examples/machine_form.zyl
zelyra serve examples/machine_form.zyl
~~~

Öffne anschließend http://127.0.0.1:3000/machines.
