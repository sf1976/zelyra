# Zelyra algorithm workbench

This small, offline runnable project demonstrates algorithms using Zelyra's
typed arrays, `Option`, `Result`, loops, recursion, and mutable-array operations.
Each implementation has a deliberate input bound, and `main.zyl` runs checks
for ordinary inputs and edge cases.

From the repository root:

```sh
zelyra check examples/algorithms/main.zyl
zelyra run examples/algorithms/main.zyl
```

The project sets `[runtime].max_steps = 20000` and
`[runtime].max_stack_depth = 100`. The examples also enforce their own limits:
sorting and search accept at most 100 values, recursive factorial accepts 0
through 12, and bottom-up Fibonacci accepts indices 0 through 45. The bounds
keep these teaching examples predictable; they are not performance claims for
large data sets.

Expected output:

```text
PASS insertion sort: empty, single item, duplicates and negative values
PASS linear search: empty, first, duplicate and missing values
PASS array-backed stack: LIFO order and empty pop
PASS bounded recursion: factorial base cases, valid input and rejected bounds
PASS bottom-up Fibonacci: base cases, bounded work and invalid inputs
PASS all algorithm examples
```

## What each example shows

- `src/sorting.zyl`: insertion sort, shifting array elements in place.
- `src/search.zyl`: linear search returning `Option<Int>` and the first match.
- `src/stack.zyl`: LIFO behavior using `push_mut` and `pop_mut`.
- `src/recursion.zyl`: checked recursive factorial with typed errors.
- `src/dynamic_programming.zyl`: iterative Fibonacci that reuses prior results.

Every example is a normal Zelyra project. Run the same `check` and `run`
commands used for an application; no npm, database, or network service is
needed.
