# Typed SQL result values

Scope: current stable Zelyra 1.0.4 release. [Deutsch](sql-results.de.md)

`sql<T> { … }` retains its raw-row behavior. Use `sql_decode<T>` explicitly
when the result must support arithmetic, field access, indexing, or iteration:

```zelyra
fn count_customers() -> Int uses Database(read) {
    return sql_decode<Int>(sql<Int> { SELECT COUNT(*) FROM customers })
}
```

The argument must be a **direct SELECT SQL expression**. Its annotation still
participates in existing SQL/schema/parameter checks. The decoder's type is the
materialized result type; it can be a separate flat record projection. Runtime
checks use native column families, names, NULL flags, and exact values. Database
capabilities and project grants apply to the inner SQL expression as usual.
`sql_decode` does not take SQL strings, raw variables, or write statements.

| Target | Required rows | Required columns |
| --- | --- | --- |
| Scalar `T` | Exactly one | Exactly one |
| Record `T` | Exactly one | Exactly its declared fields, by name |
| `T[]` | Zero or more | Scalar or record projection for each row |
| `Option<T>` | Zero or one | Scalar or record projection |

No row becomes `None` for `Option<T>`; one row becomes `Some(value)`. A NULL
cell requires an optional scalar/field. For a nullable scalar in a collection,
use `Option<Int>[]`. For both an absent row and a nullable scalar, use
`Option<Option<Int>>`: no row is `None`, a NULL cell is `Some(None)`, and a value
is `Some(Some(value))`. A missing record column is an error even for optional
fields and empty results. Unexpected and duplicate names are errors; alias
columns explicitly. Projection names are case-sensitive and independent of
column order. Schema/type checks run before row cardinality is accepted.

```zelyra
struct DueDate { issued: Date occurred: Timestamp amount: Decimal }
fn dates() -> DueDate[] uses Database(read) {
    return sql_decode<DueDate[]>(sql<calendar_events[]> {
        SELECT issued, occurred, amount FROM calendar_events ORDER BY id
    })
}
```

Aliases and records with concrete generic arguments are supported. Table row
types use their table columns, with optional fields for nullable storage and
stored `<relation>_id` fields for relations. For a subset, define an explicit
record DTO. Classes, nested records/collections, enums, Bytes, function values,
and unresolved generic decoding targets are outside this flat result API and
are rejected. This does not add generic record SQL column storage.

| Zelyra type | Accepted native SQL family and value rule |
| --- | --- |
| `Int`, `Id` | Integer/NUMERIC/DECIMAL; integral value within signed 64-bit range |
| `UInt` | Integer/NUMERIC/DECIMAL; integral value within unsigned 64-bit range |
| `Decimal` | Integer/NUMERIC/DECIMAL; exact scale of at most 9 digits and existing Decimal bounds |
| `Money` | Integer/NUMERIC/DECIMAL; exact DECIMAL(19,4) range, no rounding |
| `Float` | Integer/NUMERIC/DECIMAL/Float; finite 64-bit Float conversion |
| `Bool` | Boolean or integer; only true/false or 1/0 |
| `String`, `Char` | Text; Char requires exactly one Unicode scalar |
| `Email`, `Url`, `Uuid` | Text representation; this decoder adds no domain-content validation |
| `Date`, `Time`, `Timestamp` | Corresponding native temporal family; validated calendar, whole seconds, UTC milliseconds |
| `Duration` | Integer/NUMERIC/DECIMAL; integral signed millisecond duration |

Float input is rejected for exact numeric targets, even if its displayed text
looks integral. MySQL/MariaDB binary strings are not decoded as text; use an
explicit SQL text conversion if appropriate. Values are never silently rounded,
truncated, clamped, or converted through Float for exact targets. Native NULL
is tracked separately from the literal string `\N`. PostgreSQL binary NUMERIC
and calendar/time values retain their existing exact codecs.

Builtin Money JSON input is an exact decimal string checked against the same
DECIMAL(19,4) contract. Id JSON decoding additionally accepts signed integer
values; its legacy string input remains available. Flat DTOs containing these
types round-trip through `json_encode`/`json_decode`.

Builtin `Id`/`Money` values use Int/Decimal at runtime; builtin text domains use
String. Function boundaries recognize those representations. Existing explicit
aliases/record definitions retain precedence. This does not introduce new Id
identity semantics or domain validation. Date/Time/UTC restrictions are detailed
in [calendar rules](dates.en.md); see [numeric rules](decimals.en.md) for Decimal.

The legacy SQL output, `json_encode(sql<T> { … })`, and existing raw-row HTTP
responses remain arrays of objects with **string** values, including the old
`\N` text for NULL. After explicit decoding, normal typed JSON/HTTP rules apply:
numbers and booleans are native JSON values, Date/Time are strings, Timestamp/
Duration are integer milliseconds, Decimal/Money are exact decimal strings,
and `None` is JSON null. Switching an existing endpoint to decoded values is an
explicit application API change. There is no global switch or implicit upgrade.

Invalid calls/shapes report `E-TYPE-SQL-DECODE` at their source span. Conversion
failures use the existing `E-RUNTIME-001` diagnostic at the decoder call, with
column names and, for invalid cell values, one-based row numbers. Cell contents
are omitted from conversion errors. Query/driver failures retain their existing
diagnostics. This API does not change migration approval or backend SQL support.

Run [the executable example](../examples/typed_sql_results.zyl) against an owned
test database after `zelyra db apply`. Real three-backend HTTP/SQL evidence and
The 0.9.9 verification evidence is in [the package report](release-readiness/0.9.9-typed-sql-results.en.md).
