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
clock = true
environment = true
~~~

When a project file is present, every declared capability must be enabled
there. A missing entry or `false` value is denied by `zelyra check`, `build`,
`run`, and `serve`. Standalone source files without a project file retain the
single-file development behavior and only receive declaration checking.

This phase does not grant operating-system privileges. Runtime capability boundaries are nevertheless
enforced: a function call with a project grant set is denied unless the
function's declared capabilities are granted, and native SQL is denied unless
the current function declares `Database`. The CLI passes the grants from
`zelyra.toml` to `run`, `serve`, executable API handlers, forms, CRUD, and
database-backed authentication.

Two safe host APIs are now available:

~~~zelyra
fn runtime_timestamp() -> Timestamp uses Clock {
    return now()
}

fn configured_mode() -> String? uses Environment {
    return env("ZELYRA_MODE")
}
~~~

now() requires Clock and returns the current Unix-epoch timestamp in
milliseconds. env(name) requires Environment and returns None when the
variable is missing. Neither API logs or exposes values automatically; output
only occurs when the program explicitly uses print or another application
operation. File-system and network host APIs require separate resource
policies.

The Random capability exposes secure integer generation:

~~~zelyra
fn dice_roll() -> Int uses Random {
    return random_int(1, 6)
}
~~~

Both bounds are inclusive. The runtime uses the operating system's secure
random source and rejects minimum values greater than maximum values. Random
values are never logged or printed implicitly. Process APIs still require
separate resource and error contracts.

The first network host API is `http_get`:

~~~zelyra
fn load_status(url: String) -> String uses Network {
    return http_get(url)
}
~~~

`http_get` requires the `Network` capability and returns the UTF-8 response
body for a successful HTTP request. Project execution requires an explicit
allowlist:

~~~toml
[network]
allowed_hosts = ["127.0.0.1:8080", "api.example.com"]
timeout_ms = 5000
max_response_bytes = 1048576
~~~

The allowlist matches a host or host-and-port exactly. A project with no
`[network]` section has no allowed hosts. The transport supports `http://` and
`https://` with Rustls certificate verification enabled by default. It does
not follow redirects and limits response size and total request time. The API
remains intentionally limited to GET and UTF-8 response bodies; request
headers, request bodies, and richer HTTP client features remain future work.

The Process capability exposes a deliberately narrow command API:

~~~zelyra
fn render_report(input: String) -> String uses Process {
    return run_process("/usr/bin/printf", ["%s", input])
}
~~~

`run_process(command, args)` never invokes a shell. Each argument is passed as
one separate operating-system argument. Project execution requires an exact
command allowlist and bounded resources:

~~~toml
[process]
allowed_commands = ["/usr/bin/printf"]
timeout_ms = 5000
max_output_bytes = 1048576
~~~

Without `[process]`, no command may run. The runtime clears the child process
environment, provides no stdin, kills processes that exceed the timeout, and
limits captured stdout and stderr. Non-zero exit status, invalid UTF-8, and
resource violations become explicit runtime errors. Shell execution,
arbitrary environment forwarding, working-directory selection, and process
pipelines remain future work.

The FileSystem host APIs are:

~~~zelyra
fn read_source(path: String) -> String uses FileSystem {
    return read_text(path)
}

fn write_note(path: String, content: String) uses FileSystem {
    write_text(path, content)
}

fn entries(path: String) -> String[] uses FileSystem {
    return list_dir(path)
}

fn remove_note(path: String) uses FileSystem {
    delete_file(path)
}
~~~

All four APIs require FileSystem. read_text(path) reads UTF-8 text,
write_text(path, content) creates or replaces one file, delete_file(path)
removes one file, and list_dir(path) returns sorted entry names. Empty paths,
missing files, permission failures, directories used as files, and invalid
UTF-8 become explicit runtime errors.

Projects can restrict access with existing directory roots:

~~~toml
[filesystem]
read_roots = ["."]
write_roots = ["data"]
~~~

Relative paths are resolved from the project directory. Reads and directory
listing use read_roots; writes and deletes use write_roots. Symlink targets are
canonicalized before access, and a new write target must have an existing
parent directory. If no filesystem section exists, project reads are limited
to the project directory and writes/deletes are denied.

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
