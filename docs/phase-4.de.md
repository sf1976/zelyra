# Zelyra Phase 4: Native SQL

Deutsch · [English](phase-4.md)

Phase 4 integriert SQL in die Sprache. Ein SQL-Block ist kein gewöhnlicher
String: Zelyra erkennt seine Struktur, prüft Tabellen und Spalten gegen das
Schema, validiert benannte Parameter und überprüft den angegebenen Ergebnis-
Typ.

## Typisierter SQL-Block

```zelyra
fn load_customer(id: Int) {
    customers = sql<Customer[]> {
        SELECT
            id,
            name,
            email
        FROM customers
        WHERE id = :id
    }
}
```

`Customer[]` wird auf die Tabelle `customers` abgebildet. Die Schreibweise
`:id` ist ein benannter Parameter und muss im aktuellen Zelyra-Scope vorhanden
sein. Parameter werden nicht per String-Konkatenation in SQL eingesetzt.

Für Befehle ohne Ergebnis wird der Result-Typ weggelassen:

```zelyra
sql {
    UPDATE customers
    SET name = :name
    WHERE id = :id
}
```

## Prüfung

```bash
zelyra check examples/native_sql.zyl
```

Geprüft werden aktuell:

* `SELECT`, `INSERT`, `UPDATE` und `DELETE`
* referenzierte Tabellen
* qualifizierte und unqualifizierte Spalten
* Tabellen-Aliase
* benannte Parameter im aktuellen Scope
* Ergebnis-Mapping auf bekannte Tabellen-Typen
* grundlegende Parameter-/Spaltentypen

Beispiel für einen Fehler:

```text
error[E-SQL-004]: column `username` does not exist in the referenced tables
```

## Transaktionen

Transaktionen besitzen eine eigene Syntax:

```zelyra
transaction {
    sql {
        UPDATE inventory
        SET quantity = quantity - :amount
        WHERE product_id = :product
    }

    sql {
        INSERT INTO movements (...) VALUES (...)
    }
}
```

Der Block wird bereits geparst und statisch geprüft. Die Ausführung über den
MariaDB-Runtime-Adapter folgt im nächsten Implementierungsschritt.

## Aktuelle Grenzen

Die SQL-Abfrageprüfung ist in Phase 4 integriert. Die Interpreter-Ausführung
von SQL und die vollständige Ergebnis-Deserialisierung in Zelyra-Werte sind
noch nicht abgeschlossen. Komplexe SQL-Ausdrücke werden schrittweise erweitert;
das originale SQL bleibt dabei erhalten und wird nicht in eine ORM-Kette
umgeschrieben.
