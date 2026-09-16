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
