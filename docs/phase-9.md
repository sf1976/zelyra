# Zelyra 0.1 — Phase 9: Capabilities

[Deutsch](phase-9.de.md) · English

Zelyra functions can explicitly declare the external capabilities they need:

~~~zelyra
fn send_invoice(invoice: String)
    uses Network
{
    print(invoice)
}
~~~

The initial capability set is:

~~~text
Database
Network
FileSystem
Environment
Process
Clock
Random
~~~

The compiler rejects unknown or duplicate capability names. Capability
requirements are also propagated through function calls: a function that calls
`send_invoice` must itself declare `uses Network`.

Native SQL requires `Database`:

~~~zelyra
fn load_customer() -> Customer[]
    uses Database
{
    return sql<Customer[]> {
        SELECT id, name FROM customers
    }
}
~~~

The declaration is checked statically and is visible in the AST and HIR. A
project can grant capabilities explicitly in `zelyra.toml`:

~~~toml
[capabilities]
database = true
network = false
~~~

When a project file is present, every declared capability must be enabled
there. A missing entry or `false` value is denied by `zelyra check`, `build`,
`run`, and `serve`. Standalone source files without a project file retain the
single-file development behavior and only receive declaration checking.

This phase does not yet grant operating-system privileges or implement network
and file APIs. Runtime capability enforcement for those APIs remains future
work.

The first structured-concurrency slice is available through `parallel` and
`await`:

~~~zelyra
parallel {
    customer = await load_customer()
    orders = await load_orders()
}
~~~

Each branch must bind a distinct result with `await`. Branches receive an
immutable snapshot of the surrounding state and are joined before execution
continues. Results are merged in source order, and a failed branch causes the
whole block to fail after all branches have been joined. `await` outside a
`parallel` block is rejected by the type checker. This initial runtime uses
one worker thread per branch; cancellation and database connection-pool
integration remain future work.

## Diagnostics

Missing `Database` for SQL produces an error such as:

~~~text
error[E-CAP-001]: function `load_customer` uses SQL but does not declare capability `Database`; add `uses Database`
~~~

The CLI checks capabilities as part of `zelyra check`, `zelyra build`, `zelyra
run`, and `zelyra serve`.

## Contracts

Functions can also declare runtime-checked preconditions and postconditions:

~~~zelyra
fn increment(value: Int) -> Int
    requires { value >= 0 }
    ensures { result > value }
{
    return value + 1
}
~~~

`requires` expressions run before the function body. `ensures` expressions run
after it, with the returned value available as `result`. Both expressions must
have type `Bool`. A false condition stops execution with a diagnostic runtime
error. These checks are runtime checks, not mathematical proofs. Formal
verification and proof output remain future work.

## Verification command

The first verifier is available through:

~~~bash
zelyra verify examples/contracts.zyl
~~~

It reports one status for every contract. Constant boolean expressions can be
classified as `PROVEN` or `FAILED`. The initial symbolic verifier substitutes
direct integer return expressions into `ensures` and proves simple affine
relationships. It also analyzes `if`/`else` return paths and uses their simple
integer comparisons as path assumptions.

For example, it can prove `result >= 0` for an absolute-value function with
the two branches `return value` and `return -value`. Exhaustive `match`
expressions with integer or boolean literal patterns and a wildcard arm are
also collected as separate return paths. `Option` and `Result` constructor
patterns such as `Some`, `None`, `Ok`, and `Err` contribute constructor facts
to path feasibility. A payload binding is substituted when the matched value is
a known constructor, so `Some(4)` followed by `return number` can be checked as
`return 4`. Unknown payloads and unsupported binding relationships remain
`RUNTIME_CHECK`; functions without contracts are reported as `UNPROVEN`.

Simple function calls are summarized and inlined into the integer model.
Direct-return calls and path-sensitive calls such as `return absolute(value)`
are handled with bounded depth; callee return paths contribute their own path
constraints. Before a callee summary is used, the verifier checks the callee's
`requires` after substituting the call arguments. A caller's `requires` clauses
are available as assumptions while proving its `ensures`. Calls to complex,
recursive, or otherwise unresolved functions remain `RUNTIME_CHECK`, as do
call preconditions that cannot be proven.

Local state is tracked through a function body as well. Both
`next: Int = value + 1` and the concise `next = value + 1`, followed by `return next`, can be
summarized like the equivalent direct return. Simple linear mutable
initialization and assignment are also tracked, for example `next = next + 1`.
Statically bounded loops with a linearly changing counter are also unfolded
path by path. `break` exits the current loop and `continue` starts its next
iteration; both are represented as separate symbolic control-flow paths.
Explicit loop invariants can be declared directly on a `while` or `loop`:

~~~zelyra
while current > 0
    invariant { current >= 0 }
{
    current = current - 1
}
~~~

The verifier checks the invariant at loop entry and after supported body paths.
When the invariant is proven, it may summarize an otherwise unbounded linear
`while` loop. An unconditional `loop` can use the invariant together with a
symbolically modeled `break` to prove its exit paths. Nonlinear assignments,
invalid or unsupported invariants, and other unsupported state flow remain
`RUNTIME_CHECK`. Runtime execution checks the invariant before and after every
iteration as well.

`zelyra verify` reports every declared invariant separately, after the
function's `ensures` results. The result name uses the zero-based invariant
index, for example:

~~~text
PROVEN [V-001]: reduce.ensures[0] (src/reduce.zyl:3:5-3:21)
PROVEN [V-001]: reduce.invariant[0] (src/reduce.zyl:7:21-7:33)
FAILED [V-004]: reduce.invariant[1] (src/reduce.zyl:8:21-8:34)
~~~

Each line includes a stable verification code and a source range in the form
`(file.zyl:start-line:start-column-end-line:end-column)`, so a result can be
opened directly in an editor. The codes are `V-001` for `PROVEN`, `V-002` for
`RUNTIME_CHECK`, `V-003` for `UNPROVEN`, and `V-004` for `FAILED`.

The text formatter then prints a short explanation and the affected source
line with a caret marker:

~~~text
  = The loop invariant is false on a feasible path or is not preserved by the loop body.
    |
  8 |     invariant { current == value }
    |                ^^^^^^^^^^^^^^^^^^^
~~~

For IDEs and CI, request machine-readable output:

~~~bash
zelyra verify examples/contracts.zyl --json
~~~

The JSON result contains `status`, `code`, `message`, `function`, `kind`,
`index`, `counterexample`, and a `location` object with `file`, `start`, and
`end` line/column positions. `counterexample` is an object when a small
linear integer witness was found, otherwise `null`. The current bounded
search handles at most three linear variables in the range `-32..=32`, for
failed preconditions, postconditions, and loop invariants; no counterexample
is not evidence that none exists.

The included negative example demonstrates a concrete witness:

~~~bash
zelyra verify examples/counterexample.zyl
~~~

Its failed postcondition reports `value = 0`. The command exits with a failure
status because a contract was disproved.

`FAILED` means that the invariant is false on a feasible analyzed path or is
not preserved by the loop body. `RUNTIME_CHECK` means that runtime checking
is required because the current symbolic verifier cannot complete the proof.
Neither status is a mathematical proof.

The command exits unsuccessfully for a failed constant contract or a compiler
diagnostic. No status other than `PROVEN` is a mathematical proof.
