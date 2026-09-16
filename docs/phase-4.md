# Zelyra Phase 4: Native SQL

[Deutsch](phase-4.de.md) · English

Phase 4 integrates SQL into the language. An SQL block is not an ordinary
string: Zelyra recognizes its structure, checks tables and columns against the
schema, validates named parameters, and checks the declared result type.

## Typed SQL block

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

`Customer[]` maps to the `customers` table. The `:id` syntax is a named
parameter and must exist in the current Zelyra scope. Parameters are not
inserted into SQL through string concatenation.

For statements without a result, omit the result type:

```zelyra
sql {
    UPDATE customers
    SET name = :name
    WHERE id = :id
}
```

## Checking

```bash
zelyra check examples/native_sql.zyl
```

The current checker validates:

* `SELECT`, `INSERT`, `UPDATE`, and `DELETE`
* referenced tables
* qualified and unqualified columns
* table aliases
* named parameters in the current scope
* result mapping to known table types
* basic parameter/column types

Example diagnostic:

```text
error[E-SQL-004]: column `username` does not exist in the referenced tables
```

## Transactions

Transactions have dedicated syntax:

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

The block is parsed, statically checked, and executed atomically through
MariaDB:

```bash
export DATABASE_URL='mariadb://user:password@127.0.0.1:3306/my_app'
zelyra run examples/native_sql_runtime.zyl
```

All statements in the block run on the same MariaDB connection inside one
transaction. An error prevents the commit.

## Current limitations

SQL query checking and MariaDB interpreter execution are integrated. SELECT
results are currently returned as a typed row collection whose values are
strings; full deserialization into domain-specific Zelyra records follows.
SQLite and PostgreSQL runtime adapters are not wired yet. Complex SQL
expressions will be expanded incrementally; the original SQL remains
available and is not rewritten into an ORM chain.
