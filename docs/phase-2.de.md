# Zelyra Phase 2: Typsystem und Pattern Matching

Deutsch · [English](phase-2.md)

Phase 2 ergänzt die semantische Analyse auf dem Parser und Interpreter der
Phase 1.

## Nominale Typen

Typdefinitionen erzeugen eigenständige Namen. Der Zieltyp wird nicht implizit
konvertiert:

```zelyra
type UserId = Id
type OrderId = Id
```

`UserId` und `OrderId` sind deshalb unterschiedliche Typen, obwohl beide auf
`Id` basieren.

## Option

`T?` ist eine Kurzform für `Option<T>`. Werte werden mit `Some(value)` und
`None` erzeugt:

```zelyra
fn find_number(found: Bool) -> Int? {
    if found {
        return Some(42)
    }
    return None
}
```

## Result

`Result<Erfolg, Fehler>` enthält entweder `Ok(value)` oder `Err(error)`:

```zelyra
fn load(ok: Bool) -> Result<Int, String> {
    if ok {
        return Ok(7)
    }
    return Err("failed")
}
```

## Pattern Matching

Match-Arme binden innere Werte und müssen für `Option`, `Result` und `Bool`
vollständig sein:

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

Bei einer fehlenden Variante meldet der Compiler einen Typfehler. `_` oder ein
Variablen-Pattern kann ausdrücklich alle übrigen Fälle abdecken.

## HIR und Name Resolution

Nach dem Parsen senkt Zelyra den AST in eine High-level Intermediate
Representation ab. Lokale Variablen erhalten `LocalId`s und Funktionsaufrufe
`FunctionId`s. Nicht aufgelöste Variablen und Funktionen werden vor der
statischen Typprüfung gemeldet.

Die öffentliche Lowering-API wird durch die `zelyra-hir`-Crate bereitgestellt:

```rust
let hir_program = zelyra_hir::lower(&program)?;
```

Datenbank, SQL, Web und generierter Code gehören noch nicht zu Phase 2.
