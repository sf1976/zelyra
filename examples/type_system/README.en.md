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
| Null and optional values | ✅ supported | No standalone null type. `Option<T>`, `Some`, and `None` distinguish absence from `0`, `false`, and `""`. JSON maps `None` to `null`. |
| Arrays | ✅ supported | Typed, nested, and empty arrays; `append`/concatenation and `array_replace_result` return values. Duplicates are allowed. Invalid element types fail at compile time; invalid replacement indices return `Err`. |
| Lists | ✅ alias | `List<T>` is a type synonym for `T[]`; literals remain `[a, b]`, and array operations apply unchanged. Zelyra intentionally has one ordered sequence model. |
| Tuples | ✅ supported | `Tuple<A, B, ...>` and `tuple(a, b, ...)` create ordered heterogeneous values. Constant indices are checked statically; dynamic tuple indices are rejected. JSON uses a positional array with an exact length check. |
| Sets | ✅ supported | `Set<T>`, `set([a, b])`, `contains`, `set_add`, and `set_remove`. Values have deterministic ordering and duplicates are removed. Supported elements are scalar comparable key types; JSON uses a sorted array. |
| `Map<K,V>` | ✅ supported | Typed maps; supported key types are `Int`, `UInt`, `String`, `Bool`, `Char`, and `Timestamp`. Duplicate literal keys keep one entry and the last value wins. A missing `get` key returns `None`. JSON supports only string keys. |
| Records (`struct`) | ✅ supported | User-defined nested records, fields, value equality, and JSON round trips. `record_with(record, "field", value)` returns a statically checked copy with one field changed; the source record is preserved. |
| Enums | ✅ supported | User-defined unit and payload variants (`enum Event { Created(Int) Idle }`), checked constructors, pattern bindings, exhaustive matching, value equality, and JSON round trips. Public enums also work across module imports. |
| Classes/object identity/references | — unavailable | Zelyra uses value-semantics records and immutable function flows. Classes, observable object identity, and mutable references are not currently part of the language line. |
| `Timestamp` | ✅ limited | `now()`, equality/order comparisons, UTC formatting, JSON as Unix milliseconds, and checked UTC-duration helpers `timestamp_add_result` / `timestamp_difference_result`. Years are bounded to 0001–9999; zones and local DST are out of scope. |
| `Date`, `Time`, `Duration`, `Bytes` | ✅ supported | Validated ISO dates/times, checked duration operations, and bytes 0–255. `date_add_days_result` and `date_days_between_result` implement calendar-day math, including leap years and year boundaries. |
| `Unit` | ✅ supported | Empty return value, explicit function type, and JSON round trip with `null`. |
| `Result<T,E>` | ✅ supported | `Ok`/`Err`, matching, and function returns. Existing `json_encode` output is unchanged; `json_encode_tagged_result` and `json_decode_tagged_result<T,E>` exchange Results losslessly as `{"$result":"ok","value":...}` or `{"$result":"err","error":...}`. `json_decode_result<T>` remains the Result-based decoder for ordinary JSON targets. |
| `HttpResult<T>`, `HttpResponse`, `HttpError` | 🧪 specialized | HTTP subsystem types for network responses and errors; no general literals or constructors. |
| Generics | 🧪 partial | Generic standard types and user-defined generic functions (`fn identity<T>`) with inference or explicit type arguments are available. User-defined generic records/enums and general constraints are not available yet. |
| Type aliases and custom types | 🧪 partial | `type Name = ...` and records exist. Records execute and serialize; aliases have no separate runtime identity. |
| `Id`, `Email`, `Url`, `Uuid`, `Money` | 🧪 specialized | Named string domain values for API/schema use; no separate numeric or identity rules in the runtime. |
| Functions as values | — unavailable | Named functions can be called but cannot yet be stored or passed as values. This remains an open language gap for 0.9.9. |

## Conversions, defaults, and copies

Implicit numeric conversions such as `Int` to `Float` are not allowed.
`json_decode<T>` explicitly parses JSON; it is not a general cast. Normal
bindings need an initial value; `Option` is the available way to express
absence. Missing optional record fields become `None` during JSON decoding.

Assignments and parameter passing use value behavior. Arrays and records are
cloned internally; the language exposes neither reference identity nor
separate shallow/deep copy operations. The value-update helpers return a new record or array; they do not mutate the input value or introduce reference identity.

## Automated acceptance

The successful run checks numeric ranges, arithmetic, rounding behavior,
Unicode, optional values, arrays, List aliases, tuples, sets, records, unit and
payload enums, calendar and timestamp operations, Decimal quantization, ranges,
JSON, and user-defined generic functions. The CLI integration test also expects **15 compiler errors** and **18 runtime errors** (type mismatch,
mixed arrays, non-exhaustive matching, overflow/underflow, division by zero,
invalid index, invalid Date/Time/Bytes/Decimal values, wrong tuple length,
non-finite JSON number, non-string-keyed Map serialization, and unsupported general
`json_decode<Result<...>>` decoding; the separate tagged codec is available). An expected error is a passing
test result. When the integration test passes, all 11 positive groups pass and
there are 0 unexpected failures. The current matrix has 18 supported, 5 partial,
and 2 unavailable entries; the program prints these counts. User-defined
generic functions and payload enums are covered by the positive type-system
example and explicit compiler-negative cases. Generic data types and function
values remain unavailable. Classes and reference identity are a deliberate departure
from the value-oriented record model; they are not required for records, CRUD,
or database relationships.

This is coverage of the existing language surface, not a claim that every type
or conversion is implemented.
