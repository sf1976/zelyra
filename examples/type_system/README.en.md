# Zelyra type coverage

This example tests the types that the Zelyra compiler and runtime in this
repository actually provide. It does not imitate features from other languages;
missing concepts are reported as unsupported.

## Run

```sh
zelyra check main.zyl
zelyra run main.zyl
```

Positive checks are split across modules. Compiler errors and expected runtime
errors live separately under `expected/`; the CLI integration test checks them
with:

```sh
cargo test -p zelyra-cli --test type_system
```

A fixture under `expected/` passes when Zelyra fails with its expected
diagnostic. These cases do not belong in the successful program run.

## Type inventory and result

| Type or concept | Status | Observed behavior |
|---|---|---|
| `Int` | ✅ supported | Signed 64-bit range, including directly written `-9223372036854775808`; arithmetic, remainder, and comparisons. Overflow, negating the minimum, and division by zero return controlled runtime errors. |
| `UInt` | ✅ supported | Unsigned 64-bit range; arithmetic and comparisons; overflow, underflow, and division by zero return runtime errors. |
| `Float` | ✅ supported | IEEE-754 binary64; rounding differences, infinity, and NaN are possible. JSON rejects non-finite values. |
| `Decimal` | ✅ supported | `decimal("0.10")` creates a fixed-point value with nine fractional digits. Multiplication/division can truncate beyond that scale; `decimal_quantize_result(value, scale, mode)` applies explicit rounding without a Float intermediary (`toward_zero`, `floor`, `ceiling`, `half_up`, `half_even`). JSON stores Decimal as an exact decimal string. |
| `Bool` | ✅ supported | `true`/`false`, logical operations, and equality. |
| `Char` | ✅ supported | One Unicode scalar value; JSON uses a one-scalar JSON string. Multi-codepoint grapheme clusters are not one `Char`. |
| `String` | ✅ supported | UTF-8, Unicode, escapes, equality, lexicographic comparisons, and concatenation. `text_chars` counts Unicode scalar values; no byte-length function is available. |
| `format_text` | ✅ supported | Explicit display formatting for scalar values (`Int`, `UInt`, `Float`, `Decimal`, `Bool`, `Char`, `String`, `Date`, `Time`, `Timestamp`, `Duration`). It adds no JSON quotes. Collections, records, classes, enums, `Result`, and function values are rejected; use `format_scaled_result` for fixed-scale money. |
| Null and optional values | ✅ supported | No standalone null type. `Option<T>`, `Some`, and `None` distinguish absence from `0`, `false`, and `""`. JSON maps `None` to `null`. |
| Arrays | ✅ supported | Typed, nested, and empty arrays; `append`/concatenation and `array_replace_result` return values. Duplicates are allowed. Invalid element types fail at compile time; invalid replacement indices return `Err`. |
| Lists | ✅ alias | `List<T>` is a type synonym for `T[]`; literals remain `[a, b]`, and array operations apply unchanged. Zelyra intentionally has one ordered sequence model. |
| Tuples | ✅ supported | `Tuple<A, B, ...>` and `tuple(a, b, ...)` create ordered heterogeneous values. Constant indices are checked statically; dynamic tuple indices are rejected. JSON uses a positional array with an exact length check. |
| Sets | ✅ supported | `Set<T>`, `set([a, b])`, `contains`, `set_add`, and `set_remove`. Values have deterministic ordering and duplicates are removed. Supported elements are scalar comparable key types; JSON uses a sorted array. |
| `Map<K,V>` | ✅ supported | Typed maps; supported key types are `Int`, `UInt`, `String`, `Bool`, `Char`, and `Timestamp`. Duplicate literal keys keep one entry and the last value wins. A missing `get` key returns `None`. JSON supports only string keys. |
| Records (`struct`) | ✅ supported | User-defined nested and generic records, fields, value equality, and typed JSON round trips. `record_with(record, "field", value)` returns a statically checked copy with one field changed; the source record is preserved. Generic records are not SQL column types. |
| Enums | ✅ supported | User-defined unit, payload, and generic variants with checked constructors, pattern bindings, exhaustive matching, value equality, and JSON round trips. Type arguments are inferred or explicit. Public enums also work across module imports. |
| Classes/object identity/references | ✅ supported | `class` creates instances with shared reference identity. Copies, arguments, returns, and collections preserve the instance; `==`/`!=` compare identity. `object.field = value` is type-checked. Generic classes and optional fields are available. There is no method or inheritance syntax; functions receive their object explicitly as a parameter. Direct JSON/HTTP/SQL serialization is rejected; use record DTOs. Reference cycles and mutation inside `parallel` are controlled and rejected. |
| `Timestamp` | ✅ limited | `now()`, equality/order comparisons, UTC formatting, JSON as Unix milliseconds, and checked UTC-duration helpers `timestamp_add_result` / `timestamp_difference_result`. Years are bounded to 0001–9999; zones and local DST are out of scope. |
| `Date`, `Time`, `Duration`, `Bytes` | ✅ supported | Validated ISO dates/times, checked duration operations, and bytes 0–255. `date_add_days_result` and `date_days_between_result` implement calendar-day math, including leap years and year boundaries. |
| Integer range loop | ✅ supported | `for i in start..end step amount` uses Int bounds with an exclusive end; the optional step defaults to `1`, may be positive or negative, and cannot be `0`. Negative bounds, empty ranges, `break`, and `continue` work. A loop does not materialize the range as an array. |
| `Unit` | ✅ supported | Empty return value, explicit function type, and JSON round trip with `null`. |
| `Result<T,E>` | ✅ supported | `Ok`/`Err`, matching, and function returns. Existing `json_encode` output is unchanged; `json_encode_tagged_result` and `json_decode_tagged_result<T,E>` exchange Results losslessly as `{"$result":"ok","value":...}` or `{"$result":"err","error":...}`. `json_decode_result<T>` remains the Result-based decoder for ordinary JSON targets. |
| `HttpResult<T>`, `HttpResponse`, `HttpError` | 🧪 specialized | HTTP subsystem types for network responses and errors; no general literals or constructors. |
| Generics | 🧪 partial | User-defined generic functions, records, and enums support inference or explicit type arguments. General constraints/traits, generic aliases, and generic function values are out of scope. Generic parameters support operations only when the concrete type is known. |
| Type aliases and custom types | 🧪 partial | `type Name = ...`, records, and enums exist. Generic records and enums retain nominal type arguments at runtime; aliases have no separate runtime identity. |
| `Id`, `Email`, `Url`, `Uuid`, `Money` | 🧪 specialized | API/schema scalar domains. Explicit SQL decoding uses checked Int for Id, DECIMAL(19,4) for Money, and text for Email/Url/Uuid. No new identity or domain-content rules; explicit aliases retain precedence. |
| Functions as values | 🧪 partial | Named, non-generic functions can be stored with `&name`, passed, and called indirectly through a typed `Function<(...) -> ...>` value. Capabilities remain checked statically and at runtime. Closures, captured local functions, and generic function values are not supported. |

## Conversions, defaults, and copies

Implicit numeric conversions such as `Int` to `Float` are not allowed.
`json_decode<T>` explicitly parses JSON; it is not a general cast. Normal
bindings need an initial value; `Option` is the available way to express
absence. Missing optional record fields become `None` during JSON decoding.

Scalars, arrays, and records use value behavior for assignment and parameter
passing; arrays and records are cloned internally. Classes are the explicit
exception: copied bindings and passed parameters share instance identity and
observe field updates through every reference. `record_with` and
`array_replace_result` return changed values and preserve the input. Classes
have no method or inheritance syntax and cannot be serialized directly;
reference cycles and parallel class-field mutation are rejected.

## Automated acceptance

The successful run checks numeric ranges, arithmetic, rounding behavior,
Unicode, optional values, arrays, List aliases, tuples, sets, records, unit and
payload enums, named function values and indirect calls, calendar and timestamp
operations, Decimal quantization, explicit scalar text formatting, ranges,
JSON, and user-defined generic functions, records, enums, and classes. The CLI integration test also expects **27 compiler errors** and **21 runtime errors** (type mismatch,
mixed arrays, non-exhaustive matching, overflow/underflow, division by zero,
invalid index, invalid Date/Time/Bytes/Decimal values, wrong tuple length,
non-finite JSON number, non-string-keyed Map serialization, and unsupported general
`json_decode<Result<...>>` decoding; the separate tagged codec is available). An expected error is a passing
test result. When the integration test passes, all 13 positive groups pass and
there are 0 unexpected failures. The current matrix has 19 supported, 6 partial,
and no unavailable entries; the program prints these counts. User-defined
generic functions and data types, payload enums, named function values, and
class references are covered by the positive type-system example and explicit
compiler/runtime negative cases. General constraints, generic aliases,
closures, generic function values, method syntax, and inheritance remain
unavailable. Class reference identity is distinct from business/database IDs;
use explicit records with domain IDs for APIs and persistence.

This is coverage of the existing language surface, not a claim that every type
or conversion is implemented.

Additional exact Decimal/Money boundaries, SQL scales, and alias rules:
[Decimal and monetary amounts](../../docs/decimals.en.md). Boundary regressions
cover scaled minimum/maximum values, valid wide intermediates, and controlled errors.

Current Date/Time/Timestamp/Duration rules and independent evidence:
[Dates and UTC durations](../../docs/dates.en.md). Timestamp input outside
ISO years 0001–9999 fails in a controlled way.

Native Date/Time/UTC Timestamp SQL parameters/storage are verified against
MariaDB/MySQL/PostgreSQL: [SQL evidence](../../docs/release-readiness/0.9.9-calendar-sql.en.md).
MariaDB/MySQL begin at year 1000, PostgreSQL at year 0001.
[`sql_decode<T>`](../../docs/sql-results.en.md) materializes typed scalar/record/
array results with cardinality, native type and NULL checks. Raw rows remain
available with their existing text/HTTP representation.
