# Zelyra Phase 3: Database Core

Deutsch · [English](phase-3.md)

Phase 3 macht das Datenbankschema zu einem Bestandteil des Zelyra-Programms
und erzeugt daraus DDL für PostgreSQL, MariaDB und SQLite.

## Schemadefinition

```zelyra
database main {
    engine: mariadb
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

## Datenbank-Backends

MariaDB ist ab jetzt das Standard-Backend für neue Zelyra-Projekte. Das Backend
kann mit `engine` ausdrücklich ausgewählt werden:

```zelyra
database main { engine: postgres database: "machine_management" }
database main { engine: mariadb database: "machine_management" }
database main { engine: sqlite database: "machine_management.sqlite3" }
```

`mysql` wird als kompatibler Name für das MariaDB-Backend akzeptiert. Die
Verbindungszeichenkette wird nur über `DATABASE_URL` übergeben und gehört nicht
in den Quelltext oder ins Repository.

## DDL erzeugen

Die initiale DDL kann ohne Datenbankverbindung erzeugt werden:

```bash
zelyra db create examples/machine_management.zyl
```

Die Ausgabe enthält `CREATE TABLE`, backendgerechte Datentypen, `NOT NULL`,
Defaultwerte, Foreign Keys, Unique Constraints und Indizes.

Für den schnellen lokalen Einstieg können MariaDB und SQLite direkt
initialisiert werden:

```bash
export DATABASE_URL='mariadb://user:password@127.0.0.1:3306/machine_management'
zelyra db bootstrap examples/machine_management_mariadb.zyl

export DATABASE_URL='sqlite:///tmp/machine_management.sqlite3'
zelyra db bootstrap examples/machine_management_sqlite.zyl
```

`db bootstrap` legt bei MariaDB die angegebene Datenbank an und wendet das
Schema an. Bei SQLite wird die Datenbankdatei erstellt. Zugangsdaten sollten
interaktiv oder über einen Secret-Manager gesetzt werden.

## Inspect, Plan und Apply

Für Befehle, die auf eine laufende Datenbank zugreifen, muss eine explizite
Für `inspect`, `plan` und `apply` muss eine passende Verbindungszeichenkette
gesetzt werden:

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

MariaDB verwendet `mariadb://` oder `mysql://`; SQLite verwendet `sqlite://`
mit einem Dateipfad. `db inspect` liest Tabellen, Spalten, Foreign Keys und
Indizes aus allen drei Backends.

## Aktuelle Grenzen

PostgreSQL bleibt die primäre Referenzimplementierung. SQL Server ist noch
nicht integriert. SQLite benötigt für manche destruktiven Änderungen einen
Tabellenumbau; solche Änderungen werden im Plan als destruktiv markiert und
benötigen eine spätere spezialisierte Migration.
