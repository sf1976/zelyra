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
| `Int` | ✅ supported | Signed 64-bit range; addition, subtraction, multiplication, division, remainder, and comparisons; overflow and division by zero return runtime errors. |
| `UInt` | ✅ supported | Unsigned 64-bit range; arithmetic and comparisons; overflow, underflow, and division by zero return runtime errors. |
| `Float` | ✅ supported | IEEE-754 binary64; rounding differences, infinity, and NaN are possible. JSON rejects non-finite values. |
| `Decimal` | ✅ supported | `decimal("0.10")` creates a fixed-point value with nine fractional digits and exact base-10 arithmetic. Addition, subtraction, multiplication, division, remainder, and comparisons are implemented; overflow and division by zero return runtime errors. More than nine fractional digits are rejected. JSON stores Decimal as an exact decimal string. |
| `Bool` | ✅ supported | `true`/`false`, logical operations, and equality. |
| `Char` | ✅ supported | One Unicode scalar value; JSON uses a one-scalar JSON string. Multi-codepoint grapheme clusters are not one `Char`. |
| `String` | ✅ supported | UTF-8, Unicode, escapes, equality, lexicographic comparisons, and concatenation. `text_chars` counts Unicode scalar values; no byte-length function is available. |
| Null and optional values | ✅ supported | No standalone null type. `Option<T>`, `Some`, and `None` distinguish absence from `0`, `false`, and `""`. JSON maps `None` to `null`. |
| Arrays | ✅ supported | Typed, nested, and empty arrays; `append`/concatenation return values. Duplicates are allowed. Invalid element types fail at compile time and invalid indices at runtime. |
| Lists | ✅ alias | `List<T>` is a type synonym for `T[]`; literals remain `[a, b]`, and array operations apply unchanged. Zelyra intentionally has one ordered sequence model. |
| Tuples | ✅ supported | `Tuple<A, B, ...>` and `tuple(a, b, ...)` create ordered heterogeneous values. Constant indices are checked statically; dynamic tuple indices are rejected. JSON uses a positional array with an exact length check. |
| Sets | ✅ supported | `Set<T>`, `set([a, b])`, `contains`, `set_add`, and `set_remove`. Values have deterministic ordering and duplicates are removed. Supported elements are scalar comparable key types; JSON uses a sorted array. |
| `Map<K,V>` | ✅ supported | Typed maps; supported key types are `Int`, `UInt`, `String`, `Bool`, `Char`, and `Timestamp`. Duplicate literal keys keep one entry and the last value wins. A missing `get` key returns `None`. JSON supports only string keys. |
| Records (`struct`) | ✅ supported | User-defined nested records, fields, value equality, and JSON round trips. Bindings are immutable by default; a `mutable` binding can be replaced with a new record value. |
| Enums | — unavailable | No user-defined enum type. `Option` and `Result` are built-in variant types. |
| Classes/object identity/references | — unavailable | Records behave as values. There is no observable object identity or reference semantics, and individual record fields cannot be mutated. |
| `Timestamp` | ✅ limited | `now()`, equality, UTC formatting, and JSON as Unix milliseconds. No general date/time arithmetic or ordered comparisons in the language core. |
| `Date`, `Time`, `Duration`, `Bytes` | ✅ supported | `date("YYYY-MM-DD")` validates Gregorian dates; `time("HH:MM:SS")` validates clock values; both order chronologically and serialize as ISO text. `duration_ms(Int)` creates a millisecond duration with checked addition/subtraction and integer JSON. `bytes([Int, ...])` accepts only values from 0 to 255 and serializes as an integer array. |
| `Unit` | ✅ supported | Empty return value, explicit function type, and JSON round trip with `null`. |
| `Result<T,E>` | ✅ supported | `Ok`/`Err`, pattern matching, and function returns. JSON encodes `Ok` as its contained value and `Err` as an object with `error`; a generic `Result` decoder is missing. |
| `HttpResult<T>`, `HttpResponse`, `HttpError` | 🧪 specialized | HTTP subsystem types for network responses and errors; no general literals or constructors. |
| Generics | 🧪 built-ins | Generic standard types such as `Option`, `Result`, `Map`, arrays, and `json_decode<T>` exist. User-defined generic functions and data types are unavailable. |
| Type aliases and custom types | 🧪 partial | `type Name = ...` and records exist. Records execute and serialize; aliases have no separate runtime identity. |
| `Id`, `Email`, `Url`, `Uuid`, `Money` | 🧪 specialized | Named string domain values for API/schema use; no separate numeric or identity rules in the runtime. |
| Functions as values | — unavailable | Functions can be called but cannot be stored or passed as values. |

## Conversions, defaults, and copies

Implicit numeric conversions such as `Int` to `Float` are not allowed.
`json_decode<T>` explicitly parses JSON; it is not a general cast. Normal
bindings need an initial value; `Option` is the available way to express
absence. Missing optional record fields become `None` during JSON decoding.

Assignments and parameter passing use value behavior. Arrays and records are
cloned internally; the language exposes neither reference identity nor
separate shallow/deep copy operations. The tested mutation path reassigns a
`mutable` binding; it does not mutate a record field or array element by index.

## Automated acceptance

The successful run checks numeric ranges, arithmetic, rounding behavior,
Unicode, optional values, arrays, List aliases, tuples, sets, records, and JSON. The CLI integration
test also expects **7 compiler errors** and **18 runtime errors** (type mismatch,
mixed arrays, non-exhaustive matching, overflow/underflow, division by zero,
invalid index, invalid Date/Time/Bytes/Decimal values, wrong tuple length,
non-finite JSON number, non-string-keyed Map serialization, and unsupported
`Result` decoding). An expected error is a passing
test result. When the integration test passes, all 8 positive groups pass and
there are 0 unexpected failures. The type matrix explicitly records that
user-defined enums, function values, and user-defined generics are not yet
available. Records intentionally use value semantics; custom class/reference
identity is not implemented.

This is coverage of the existing language surface, not a claim that every type
or conversion is implemented.
