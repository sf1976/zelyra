# Decimal and monetary amounts

These rules describe the unreleased 0.9.9 development branch.
[Deutsch](decimals.de.md)

## Declaration and arithmetic

```zelyra
amount: Decimal = decimal("119.00")
tax_rate: Decimal = decimal("0.19")
tax: Decimal = amount * tax_rate
```

`decimal` takes a decimal string. A literal such as `119.00` is `Float` and
is not implicitly converted. Fixed point has nine fractional digits and a
scaled signed 128-bit range:
`-170141183460469231731687303715.884105728` through
`170141183460469231731687303715.884105727`. Result overflow and division or
remainder by zero produce controlled runtime errors with source positions.
Multiplication/division use an exact intermediate bounded to 256 bits when
needed; valid results no longer fail solely because an intermediate exceeds
128 bits. Excess fractional digits still truncate toward zero. This is not
automatic monetary rounding.

Constructors and forms use a decimal point, reject exponents, leading plus,
whitespace, NaN/infinity, and more than nine fractional digits.
`decimal_scaled_result` retains its separate checked cent-input contract,
including decimal commas.

## Explicit monetary rounding

`decimal_quantize_result(amount, 2, "half_up")` returns
`Result<Decimal, String>`. Modes are `toward_zero`, `floor`, `ceiling`,
`half_up` (ties away from zero), and `half_even` (ties to an even final digit).
They also apply to negative values. Scales 0 through 9 are supported. Rounding
cannot undo precision already lost during arithmetic.

`format_text` removes insignificant fractional zeros. For a fixed display such
as `119.00`, pass the rounded value through `decimal_scaled_result` and
`format_scaled_result`. See the runnable
[decimal_money.zyl](../examples/decimal_money.zyl):

```bash
zelyra check examples/decimal_money.zyl
zelyra run examples/decimal_money.zyl
```

## Databases, forms, and HTTP

| Declaration | MariaDB / MySQL | PostgreSQL | Form contract |
| --- | --- | --- | --- |
| `amount: Decimal required` | `DECIMAL(39,9)` | `NUMERIC(39,9)` | Exact fixed point with at most 9 fractional digits |
| `price: Money required` | `DECIMAL(19,4)` | `NUMERIC(19,4)` | At most 15 integer and 4 fractional digits |
| `quantity: UInt required` | `DECIMAL(20,0)` | `NUMERIC(20,0)` | Integer from 0 through `18446744073709551615` |

`Money` remains a specialized schema/form type. It does not replace the numeric
runtime type `Decimal` or imply automatic rounding to cents. Its existing
four-digit scale remains intact. Empty optional numeric form fields bind as
SQL `NULL`. Table-bound Decimal/Money aliases preserve the storage contract,
including optional columns. Default exact inputs use text with
`inputmode="decimal"`, without conversion to JavaScript numbers. An explicitly
selected widget remains a separate presentation choice.

Decimal, Money, and corresponding filter parameters travel as validated
text and bind explicitly as numeric values without Float. PostgreSQL NUMERIC
results decode exactly; nonfinite numeric results fail in a controlled way.
Plain PostgreSQL form actions are atomic. Tenant/recipient checks, edit snapshots,
and audit decisions retain an explicit PostgreSQL blocker; this package does
not prove full backend parity. SQLite schema tools do not guarantee exact
Decimal runtime storage through NUMERIC affinity.

The language codec `json_encode` still writes Decimal strings;
`json_decode<Decimal>` requires that string. HTTP APIs additionally accept
exactly representable JSON numbers, including exponents, and reject required
rounding. Responses, OpenAPI, and TypeScript use decimal strings. Recommended:

```json
{"amount":"119.00"}
```

A browser may round a JavaScript number before sending it; the server cannot
recover missing digits. JSON parsing preserves numeric lexemes and treats keys
such as `$serde_json::private::Number` as ordinary object keys. The JSON nesting
limit remains 128.

## Updating existing projects

Legacy MariaDB `Decimal` columns may have been stored as `DECIMAL(10,0)`.
`zelyra db plan` exposes the change; it is not automatically approved. Declared
fixed-point precision is checked before SQL execution and before `serve` when
the database capability is granted. A mismatch or unavailable metadata prevents
operation. `E-DB-PRECISION` points to the affected source column for `serve`;
the interpreter reports the SQL call site.

Back up first, review the plan, then apply it deliberately. A schema change
cannot recover fractional digits previously discarded. Checks run at server
startup or before an interpreter's first SQL operation; they do not replace
control of concurrent schema changes. Metadata privileges are required.

Decimal aliases are checked against their underlying type in type checking,
returns, and SQL. Cyclic aliases produce `E-TYPE-ALIAS-001`; expansion is bounded
to 64 levels and 4,096 type nodes. Aliases do not make incompatible String/Float
values valid. Nominal records, classes, and enums retain their identity.

Raw SQL rows retain their existing text representation. A declared SQL result
type alone currently does not guarantee automatic typed scalar/record
consumption; the broader data-path review is a separate 0.9.9 gate.
