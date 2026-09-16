# Zelyra Phase 3: Database Core

Deutsch · [English](phase-3.md)

Phase 3 macht das Datenbankschema zu einem Bestandteil des Zelyra-Programms
und erzeugt daraus PostgreSQL-DDL.

## Schemadefinition

```zelyra
database main {
    engine: postgres
    database: "machine_management"
}

table departments {
    id: Id primary auto
    name: String(100) required unique
}

table machines {
    id: Id primary auto
    number: String(30) required unique
    department: Department required
    active: Bool default true

    index {
        department
    }
}
```

`Department` wird auf die Tabelle `departments` aufgelöst. Die erzeugte
Speicherspalte heißt `department_id` und erhält einen Foreign Key auf
`departments.id`.

## PostgreSQL-DDL

Die initiale DDL kann ohne Datenbankverbindung erzeugt werden:

```bash
zelyra db create examples/machine_management.zyl
```

Die Ausgabe enthält `CREATE TABLE`, PostgreSQL-Datentypen, `NOT NULL`,
Defaultwerte, Foreign Keys, Unique Constraints und Indizes.

## Inspect, Plan und Apply

Für Befehle, die auf eine laufende Datenbank zugreifen, muss eine explizite
PostgreSQL-Verbindungszeichenkette gesetzt werden:

```bash
export DATABASE_URL='postgres://user:password@localhost/machine_management'
zelyra db inspect examples/machine_management.zyl
zelyra db plan examples/machine_management.zyl
zelyra db apply examples/machine_management.zyl
```

`db plan` funktioniert auch ohne `DATABASE_URL`; dann wird gegen eine leere
Datenbank geplant und die initiale DDL kann offline geprüft werden. `db apply`
verweigert die Ausführung ohne echte Verbindung.

Destruktive Änderungen werden im Plan sichtbar gemacht und standardmäßig
abgelehnt:

```bash
zelyra db apply examples/machine_management.zyl --allow-destructive
```

Das Flag ist erst nach Prüfung des erzeugten Plans erforderlich. Zugangsdaten
kommen aus der Umgebung und werden niemals im Schemaquelltext gespeichert.

## Aktuelle Grenzen

PostgreSQL ist das Referenz-Backend der Phase 3. Adapter für MariaDB, MySQL,
SQLite und SQL Server sind für spätere Phasen vorgesehen. Die Live-Inspektion
liest derzeit Tabellen, Spalten und Indizes; weitere Constraint-Metadaten
werden später ergänzt.
