# Zelyra Phase 2: Type system and matching

[Deutsch](phase-2.de.md) · English

Phase 2 adds semantic analysis on top of the Phase 1 parser and interpreter.

## Nominal types

Type definitions create distinct names. The target type is not an implicit
conversion:

```zelyra
type UserId = Id
type OrderId = Id
```

`UserId` and `OrderId` are therefore different types even though both are
based on `Id`.

## Option

`T?` is shorthand for `Option<T>`. Values are constructed with `Some(value)`
and `None`:

```zelyra
fn find_number(found: Bool) -> Int? {
    if found {
        return Some(42)
    }
    return None
}
```

## Result

`Result<Success, Error>` carries either `Ok(value)` or `Err(error)`:

```zelyra
fn load(ok: Bool) -> Result<Int, String> {
    if ok {
        return Ok(7)
    }
    return Err("failed")
}
```

## Typed maps

Zelyra 0.1 provides deterministic typed maps for scalar keys. The key and
value types are explicit in `Map<Key, Value>`; map literals use `Map { ... }`.
The supported key types are `Int`, `UInt`, `String`, `Bool`, `Char`, and
`Timestamp`.

```zelyra
fn main() {
    prices: Map<String, Int> = Map {
        "standard": 10
        "premium": 20
    }

    mutable current = put(prices, "standard", 12)

    match get(current, "standard") {
        Some(price) => {
            print(price)
        }
        None => {
            print(0)
        }
    }

    print(keys(current))
    print(values(current))
}
```

`get` returns an `Option<Value>`, so a missing key is handled explicitly.
`put` returns a new map and replaces an existing key without changing its
position. Duplicate keys in a literal use the last value while retaining the
first insertion position. `keys`, `values`, `contains`, and `len` are
deterministic. JSON
conversion supports `Map<String, Value>` as a JSON object; maps with other key
types are language values but are rejected by JSON conversion.

## Pattern matching

Match arms bind inner values and must be exhaustive for `Option`, `Result`, and
`Bool`:

```zelyra
match find_number(true) {
    Some(value) => {
        print(value)
    }
    None => {
        print(0)
    }
}
```

The compiler reports a type error when an arm is missing. `_` or a variable
pattern can explicitly cover all remaining cases.

## HIR and name resolution

After parsing, Zelyra lowers the AST into a High-level Intermediate
Representation. Local variables receive `LocalId`s and function calls receive
`FunctionId`s. Unresolved variables and functions are reported before static
type checking.

The public lowering API is provided by the `zelyra-hir` crate:

```rust
let hir_program = zelyra_hir::lower(&program)?;
```

Phase 2 does not yet include database, SQL, web, or generated code.
