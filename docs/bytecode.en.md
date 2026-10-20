# Experimental Zelyra Bytecode v26

`zelyra-mir::bytecode` provides the first executable bytecode backend for
already type-checked and verified MIR. The AST interpreter remains the reference implementation and the normal
runtime backend. The bytecode VM can be selected explicitly for its documented
subset with `zelyra run <file.zyl> --backend bytecode`; unsupported programs fail
closed with a source-positioned diagnostic. AST remains the default.

## Current scope

The compiler selects one named entry function and its direct-call closure,
including functions called by preconditions or postconditions.
`print(value)` is supported as captured output: the CLI emits each line, and
the captured execution API preserves output produced before a later runtime
error. It does not require the `Console` capability, matching the AST runtime.
It lowers scalar literals (`Int`, `UInt`, `Float`, `Bool`, `String`, `Char`),
locals, unary and binary operations, short-circuit boolean operators, direct
pure function calls, local assignment, branches, and returns. Selected pure standard-library calls use the same dispatcher as the MIR interpreter, including `len`, `append`, `get`, `contains`, `set`, `set_add`, `set_remove`, `range`, `sort`, `sort_desc`, `reverse`, `to_float`, and supported text/math/date helpers. `to_float` accepts finite numeric values and parseable finite numeric strings; unsupported values, invalid strings, and non-finite results produce the same runtime errors as the AST interpreter. Arithmetic and
ordering are supported for `Int`, `UInt`, and `Float`; strings support
concatenation and ordering, booleans support logic/equality, and `Char` supports
equality. Array literals, tuple literals, generic record literals (with explicit
or inferred type arguments), typed record-field reads, and array/tuple index
reads are also supported. Arrays with matching element types can be concatenated
with `+`. Typed map literals, empty maps with an explicit
map type, and lookup through the shared `get(map, key) -> Option<V>` builtin
are supported. Duplicate literal keys keep their original position and the last
value wins. Direct bracket indexing on maps is not accepted by Zelyra syntax.
Record construction validates every field against
the checked record declaration; field access validates the receiver and result
types at runtime. `..base` record updates copy a checked value and replace
explicitly named fields; generic record types are checked after substituting
their concrete type arguments. JSON field reads remain outside this slice.
Arrays of records can be iterated and their typed fields read in the loop body;
Set/Map-of-record behavior has not been tested as a supported path.
Opt-in mutable arrays support indexed replacement, `+=` updates, and
`push_mut`/`pop_mut` on explicitly mutable bindings. Bytecode v25 encodes
`pop_mut` as a dedicated verified instruction; the verifier checks target
mutability and payload type. Indexed writes update the existing local vector
in place in AST, MIR, and Bytecode; they do not clone the entire array. Arrays,
tuples, Options, and Results of matching
types can use `==`/`!=`. Indexing uses the shared MIR runtime semantics, including
source-mapped negative-index and bounds errors. Tuple indices must be
non-negative integer literals, as required by the language type checker.
Decimal arithmetic, `Duration` addition/subtraction, and ordering comparisons
for `Decimal`, `Duration`, `Date`, `Time`, and `Timestamp` are supported. Date,
time, and timestamp arithmetic remains rejected during compilation.
AST, MIR, and bytecode execution delegate value-level unary and binary
operators to shared runtime helpers. The bytecode compiler still permits only
the type/operator combinations listed here; helper support for Decimal,
Duration, arrays, or temporal comparisons does not imply bytecode support.
Scalar `while` and `loop` control flow, including `break` and `continue`, are
supported through verified MIR branches and jumps; their instructions consume
the same deterministic step budget. Single-binding `for value in range(start,
end[, step])` loops are supported for `Int` ranges. Range bounds and step are
evaluated once at initialization, the end is exclusive, descending ranges use
a negative step, and a zero step fails at runtime. Collection iterators and
single-binding `for value in values` loops over `Array<T>` and `Set<T>` are also
supported; the collection is evaluated once and preserves the runtime's
iteration order. Collection items can be destructured into tuple bindings; normal tuple assignments
may bind new or existing locals. The VM checks tuple type and arity at runtime. The VM has an
iterative Rust-side call-frame stack, source spans on instructions, an operand
stack verifier, verified jump targets and call arities, and execution step and
call-depth limits.

MIR and bytecode evaluate loop invariants before each condition/body iteration
and after the body, including `break`, `continue`, and `return` paths. A return
expression is evaluated before its active loop invariants, matching AST
interpreter behavior. Nested returns check invariants from the innermost loop
outward. Failed checks report the source span of the relevant loop statement.

The supported HostServices effects are `Clock` (`now()`), `Environment`
(`env(name)`), `Console` (`read_console(prompt)`), and `Random`
(`random_int(min, max)` and `uuid_v4()`), plus `Process` through
`run_process(command, arguments)`. Process execution requires a declared and
explicitly granted `Process` capability and a runtime policy with an exact
command allowlist entry, positive timeout, and output limit. The host receives
separate arguments and does not invoke a shell. FileSystem calls include reads via
`read_text(path)` and `read_text_result(path)`, metadata via
`file_exists(path)`, `dir_exists(path)`, and `file_size(path)`, plus
directory listing through `list_dir(path)` and write-root-restricted text
writes through `write_text(path, content)`, `write_text_result(path, content)`,
`write_text_new_result(path, content)`, `atomic_write_text_result(path, content)`,
and `append_text_result(path, content)`, plus write-root-restricted deletion through `delete_file(path)` and
`delete_file_result(path)`, `copy_file_new_result(source, destination)`, and
`rename_file_result(source, destination)`. They use the injected host and the same
canonicalization, symlink, and configured-root checks as the AST runtime.
`read_text_result` returns path and I/O failures as
`Result` errors; `read_text` reports them as runtime errors. Read-only metadata
calls include `file_exists(path)`, `dir_exists(path)`, and `file_size(path)`;
the latter returns `Result<Int, String>`. Existence checks return `false` when
a path is outside configured roots or cannot be inspected. Write failures from
`write_text_result` stay in `Result`; `write_text` reports them as runtime
errors. `delete_file_result` returns path and I/O failures as `Result` errors;
`delete_file` reports them as runtime errors. Both require the `FileSystem`
capability and configured write-root containment. Exclusive creation, atomic replacement, appending, copying, and renaming are
available through their dedicated builtins. All listed filesystem calls use
the project runtime policy. `list_dir(path)` returns host-provided entry names as `String[]` (the system host sorts them) and reports
path, permission, or I/O failures as runtime errors.  A verified `HostCallPreflight`
instruction checks the declaration and explicit project grant before call
arguments are evaluated. The verifier requires each host builtin to have a
matching preflight and rejects inconsistent pending calls at control-flow
joins. This preserves AST/MIR authorization ordering and prevents argument
side effects before denial. Function declarations are serialized and verified; execution requires an explicit
matching project grant and an injected host provider. The convenience `execute`
API grants no host capabilities. `http_get(url)` and
`http_request(method, url, headers, body)` are supported along with
`http_json<Request, Response>(method, url, headers, body)` and
`http_result<Request, Response>(...)`. The typed JSON operations serialize and
decode records, aliases, and enums using checked type definitions carried by
the bytecode artifact. Like the AST interpreter, they add
`Content-Type: application/json` when none is supplied. `http_json` accepts
only 2xx responses; `http_result` maps status, headers, body, and optional
data or `HttpError`. All use the shared URL, host allowlist, timeout, header,
response-size, and UTF-8 checks. `mail_deliver(event_id, payload_json)` is also
supported and returns `Result<Unit, String>`. It requires the declared and
explicitly granted `Email` capability plus configured SMTP and network policies;
the shared runtime validates those policies before the injected host is called.
Tests use a fake host and do not send email. Static SQL expressions are
supported in bytecode v20 when the embedding supplies a
`BytecodeDatabaseExecution` context with `DATABASE_URL` and the checked project
schema. The VM rechecks SQL shape, schema, parameter types, capability, and any
project grant before storage validation or calling the database host.
`zelyra run --backend bytecode` supplies this context when a database connection
is configured. Bytecode v21 supports transactions containing checked SQL
statements and `mail_queue` calls with optional PDF attachments. For an
outbox operation, the VM checks both `Email` and
`Database(write)` grants before evaluating message fields. It validates the
message and submits the outbox insert together with SQL in one batch; it never
sends the email itself. SQL schema and storage compatibility are checked before
the host batch. PDF attachments are supported; other transaction effects
remain unsupported and fail closed.
Enum payload constructors and match statements/expressions with
enum/Option/Result/record/literal/wildcard patterns, including guards, are supported.
Inline lambda callbacks, named function references such as `&double`, and local callback variables work with `map`, `filter`, `find`, and `reduce`. Named function values can be forwarded as typed parameters and invoked at runtime; the VM rechecks the target signature and capabilities. Bytecode v22 also materializes stored lambdas as function values: capture LocalIds and types are encoded in the artifact, the verifier checks capture layout and capability ceilings and rejects writes to captured LocalIds; the VM snapshots captured values when creating a closure. Since v23, each function also carries local mutability metadata and the exact instruction sites that initialize new bindings. The verifier rejects tampered writes to immutable locals, including indexed/appended array updates, tuple and `parallel` bindings, loop bindings, and match bindings. The compiler checks those sites against their declarations. Within the supported bytecode subset, closures can be passed through typed function parameters, returned, and invoked. The existing inline-lambda path is unchanged. Unsupported operations in lambda bodies, dynamic JSON field access, and unsupported transaction effects still fail closed.
Parallel `await` blocks run verified branches on scoped worker threads. A block
may contain at most four branches; the type checker, MIR and bytecode verifiers,
and runtimes enforce the same cap before starting workers. This bounds the
aggregate worker-stack reservation. Branches use snapshots, share the step
budget and host policies, and publish results in source order.
`E-MIR-BYTECODE-001` marks an unsupported operation. The compiler only emits a
reachable call closure, so unsupported code in unrelated functions does not
block a selected entry.

## SQL expressions with explicit database context

The public bytecode entry point `execute_with_database_and_host_services_and_policy_captured`
accepts `BytecodeDatabaseExecution { url, schema }`. `schema` is the checked
project schema from `MirProgram::database_schema`; a SQL expression fails with
a runtime diagnostic when that context is missing. Database access requires
both the capability declared by the function (for example `Database(read)`)
and the project grant. Before calling the provider, the VM rechecks the SQL
expression against the schema and actual parameter types and runs read-only
storage validation. The bytecode CLI path supplies this context when
`DATABASE_URL` is set.

`sql_decode<T>(sql<T> { SELECT ... })` uses the same native-row decoder as the
AST and MIR backends. It checks column names, native column families, nulls,
scalar/record shape, and result cardinality before returning typed Zelyra
values. Table-derived row records are carried as SQL projection metadata in
MIR and serialized bytecode; they do not become constructible as ordinary
source records. The source compiler currently requires a direct `SELECT` expression;
the bytecode backend remains an experimental subset.

```rust
let database = BytecodeDatabaseExecution {
    url: &database_url,
    schema: &mir.database_schema,
};
let outcome = execute_with_database_and_host_services_and_policy_captured(
    &bytecode, arguments, limits, Some(&grants), policy.as_ref(), database, host,
);
```

Bytecode v21 supports transactions containing checked SQL statements and
`mail_queue` operations with optional HTML and PDF attachments. The verified
artifact carries the referenced view definitions; `render_pdf` uses the same
bounded, local text-oriented renderer as the AST and MIR runtimes. The verifier
checks SQL capabilities, result types, parameter names, and view-name
uniqueness. For outbox operations the VM preflights `Email` and
`Database(write)` before evaluating message fields, validates the message,
then submits the outbox insert with SQL in one transactional batch. It does
not send email. Other transaction effects remain unsupported.

## Rust API sketch

```rust,ignore
let mir = zelyra_mir::lower(&ast)?;
let bytecode = zelyra_mir::bytecode::compile(&mir, "calculate")?;
let result = zelyra_mir::bytecode::execute(
    &bytecode,
    vec![zelyra_runtime::Value::Int(40)],
    zelyra_mir::execution::ExecutionLimits::default(),
)?;
assert_eq!(result.value, zelyra_runtime::Value::Int(42));
```

The concrete API lives in `mir/src/bytecode.rs`; tests are the executable
examples for current syntax. For example, a checked expression such as `values |> filter(|n| n > minimum) |> map(|n| n * factor)` is lowered into looping bytecode that reads captured locals; this inline callback does not become a function value. A stored or returned lambda is materialized with its typed capture layout and can be invoked through a function value. AST/MIR/bytecode differential tests cover multiple independent closures after encode/decode. Named function references such as `&double`, mutable or immutable callback variables, and parameters of type `Function<(Int) -> Int>` work as `map`/`filter`/`find`/`reduce` callbacks. Dynamic calls are limited to functions included in the verified bytecode program. Compilation starts from checked MIR and calls the
existing MIR verifier. The bytecode verifier checks its in-memory control-flow
and stack structure before execution, while runtime checks guard value kinds,
function arguments, return values, capability declarations, and host grants.

## Format and safety limits

Bytecode v26 uses explicit source-step markers for the user-visible
`--max-steps` budget, so VM opcode count no longer defines the language-level
fuel unit. The tested statement, expression, while-loop, collection-loop, match, and if-expression boundaries match AST and MIR, including failure spans. Calls and function pre/postconditions are also covered by a separate budget sweep. Transactions, parallel work, and
other source constructs still need broader boundary coverage. The VM also imposes a separate 10,000,000-op execution ceiling for
malformed or hand-built artifacts.

`BYTECODE_VERSION` identifies version `26` of a deterministic binary format.
Version 26 adds source-step markers. Version 25 added the verified mutable-array
pop instruction and typed structural array/tuple/Option/Result equality
operations.
Version 24 serializes tenant-scoped table names. The verifier and VM reject raw
SQL against those tables; database execution also requires this list to match
the schema derived from the checked project. Typed tenant queries are not yet
supported.
Named function references and supported capturing lambdas can be passed as
typed runtime values and invoked dynamically by `map`, `filter`, `find`, and
`reduce`. The VM rechecks the target signature, capture layout, and capabilities
at invocation. Captured values are snapshots; arbitrary AST operations remain
outside this bytecode subset.
`encode` and `decode` are available as Rust APIs; `decode_with_limits` accepts
explicit limits. Defaults are 16 MiB per artifact, 10,000 functions, 1,000,000
instructions, 1 MiB per string, 64 nested type levels, and 100,000 type nodes.
Encoding and loading run the structural bytecode verifier. The decoder rejects
unknown tags, invalid UTF-8, truncated or trailing bytes, and invalid control
flow. Version 17 adds verified parallel branch descriptors and serializes
checked record, alias, and enum definitions plus typed `http_json`/`http_result`
calls. The format contains the supported bytecode subset and explicitly
capability-checked clock, environment, console, and random host calls. Version 13 added enum payload construction and serialized match-pattern instructions. Version 14 adds inline lambda lowering for the four array transformation operations, plus checked internal array append and Option construction instructions. Version 15 adds type-checked record copy updates. Earlier versions added verified host-call preflight so authorization occurs before argument evaluation.
There is still no persistent cache, stored-artifact workflow, or automatic
artifact distribution. The CLI switch compiles the checked project on each run;
it does not load externally supplied bytecode artifacts. Artifacts are not signed; structural verification does not prove
origin or complete static type, effect, or contract checking. Runtime resource
limits and dynamic checks remain required.

The backend is experimental and does not establish full AST/MIR parity or a
performance gain. The listed Clock, Environment, Console, Random, Process,
FileSystem, Network, and Email operations have varying fake-host or fixture
coverage; this does not establish complete effect parity. SQL and outbox
transactions with optional PDF attachments are supported from bytecode v21;
other transaction effects remain unsupported. Differential tests currently
cover selected scalar calls,
branches, boolean short-circuiting, scalar `while` and `loop` control flow with
`break`/`continue`, ascending/descending/empty integer-range `for` loops,
array and set iteration (including empty collections), tuple-bound collection
loops, inline-lambda `map`/`filter`/`find`/`reduce` with lexical capture and
empty-input behavior, statically named function callbacks, array/tuple construction and indexing, tuple assignment, mutable array replacement and `+=`, shared pure built-ins (including Option/Result constructors, collections, range, `to_float`, text, CSV, fixed-point money helpers, math, and dates),
serialized execution,
source-mapped out-of-bounds errors, zero-step errors, step exhaustion,
call-depth exhaustion, function preconditions and postconditions (including
failures after encode/decode), fail-closed handling of effects and malformed
tuple values, malformed bytecode structure, and parallel `await` branches with
snapshot captures, ordered output, capability denial, shared step exhaustion,
and encode/decode verification. `requires` runs after argument
binding; `ensures` runs at every return with the returned value available as
`result`. Contract checks consume the normal step budget. They enforce runtime
conditions but do not constitute formal proofs. The AST interpreter remains
available as the semantic reference.
