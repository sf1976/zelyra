# Zelyra 0.1 — Phase 7: CRUD

Deutsch · [English](phase-7.md)

Phase 7 beginnt CRUD als Sprachabstraktion. Der aktuelle vertikale Schnitt
unterstützt:

~~~zelyra
crud Machine -> machines
~~~

CRUD-Ressourcen können jetzt Titel, sichtbare Listenspalten,
durchsuchbare Spalten und Filterspalten konfigurieren:

~~~zelyra
crud Machine -> machines {
    title: "Machines"
    list { number name department active }
    search { number name }
    filter { department active }
}
~~~

Die Blöcke sind optional. Ohne Konfiguration bleiben die sicheren Defaults
erhalten: alle Schema-Spalten in der Liste, Textspalten für die Suche und alle
Spalten außer der ID für Filter. Konfigurierte Namen werden vor dem
Serverstart gegen das Schema geprüft; Beziehungsfelder wie department
werden automatisch auf ihre gespeicherte Foreign-Key-Spalte abgebildet.

Mit einer auf MariaDB zeigenden `DATABASE_URL` wird `GET /machines`
bereitgestellt. Die generierte Liste umfasst aktuell alle Schema-Spalten in
einer escaped HTML-Tabelle, Suche über Textspalten mit gebundenen Parametern
und begrenzte Pagination über die Query-Parameter `page` und `per_page`.
Zusätzlich gibt es exakte Filter über `filter_<spalte>` sowie eine Allowlist-
Sortierung über `sort` und `order`. Unbekannte Sortier- oder Filterspalten
werden mit HTTP 400 abgelehnt.

Foreign-Key-Spalten werden für die Darstellung automatisch verknüpft. Die
gespeicherte `department_id` wird beispielsweise in Listen und Details als
`name` der Abteilung (`Production`) angezeigt; das generierte Formular sendet
weiterhin die validierte Foreign-Key-ID. Bezeichnungen für Sortierung und
Filter verwenden den logischen Feldnamen, während der Filter eine exakte,
parametrisierte ID-Abfrage bleibt.

Jede Zeile verlinkt auf eine generierte Detailroute, beispielsweise
`GET /machines/1`. Die Detailansicht bietet automatisch erzeugte Create- und
Edit-Formulare unter `/machines/new` und `GET/POST /machines/1/edit`. Diese
Formulare übernehmen Schema-Validierung, Beziehungs-Selects, CSRF-Schutz,
gebundene Parameter und MariaDB-Transaktionen. Edit-Formulare werden mit dem
ausgewählten Datensatz vorausgefüllt.

Die Detailansicht enthält außerdem eine durch CSRF geschützte
Löschbestätigung. Ein gültiges `POST /machines/1/delete` führt ein
parametrisiertes MariaDB-DELETE in einer Transaktion aus und leitet zu
`/machines` weiter. Ein ungültiges Token wird mit HTTP 403 abgelehnt; der
Datensatz bleibt unverändert.

Bei fehlender Datenbankkonfiguration wird HTTP 503 geliefert. Abfragefehler
werden als allgemeiner HTTP-500-Fehler ausgegeben. CRUD-Ressourcen und
Tabellennamen werden vor dem Serverstart geprüft.

Die erste Autorisierungsgrenze ist jetzt in
[Phase 8](phase-8.de.md) dokumentiert. Als Nächstes folgen dort
datenbankgestützter Login und sichere Sessions. Das vollständige Beispiel mit Beziehung ist
`examples/machine_form.zyl`.

~~~bash
export DATABASE_URL='mariadb://root:<passwort>@127.0.0.1:3306/zelyra_crud'
zelyra db bootstrap examples/machine_form.zyl
zelyra serve examples/machine_form.zyl
~~~

Öffne anschließend http://127.0.0.1:3000/machines.
