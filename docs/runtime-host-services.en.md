# Runtime host services (experimental)

The Zelyra runtime can obtain selected operating-system services through an
injected interface. This is a versioned Rust API for embedding code, not a
stable native ABI or plugin contract. `HOST_SERVICES_API_VERSION` is currently
`7`; version 2 added injectable process execution, version 3 filesystem
services, version 4 database services, and version 5 console input, version 6 HTTP transport, and version 7 mail delivery.

## Supported services

- The MIR interpreter supports `http_get`, `http_request`, `http_json`, and
  `http_result` through the same runtime transport and URL, allowlist, header,
  timeout, response-size, and UTF-8 checks as the AST interpreter. It also uses
  the shared `encode_json_value`, `decode_json_value`, `encode_tagged_result_json`,
  and `decode_tagged_result_json` codecs. `MirProgram` retains checked record,
  alias, and enum declarations for typed decoding. Fake transport tests compare
  AST and MIR; they make no external requests.
- MIR wires `mail_deliver` through the shared redacted SMTP policy boundary. It
  checks `Email`, SMTP configuration, and network allowlist before the host call.
  Fake-host tests compare AST and MIR without sending mail. MIR supports
  standalone `mail_queue` calls, including PDF attachments, inside a database
  transaction. It shares the AST renderer (`render_pdf_view`), payload validator,
  and outbox encoder. A fake-host differential test compares the resulting
  outbox entry without sending mail.
- MIR executes the supported standalone `sql<T>` query subset and SQL/outbox
  `transaction { ... }` blocks through the injected database host. HIR records
  the SQL operation, deterministically ordered table accesses, and named
  placeholders with lexical local IDs and inferred types. MIR rechecks this
  plan against the SQL source (`E-MIR-VERIFY-004`); the shared runtime
  placeholder scanner binds only the captured values. This analysis metadata
  does not grant table access. Transactions send all checked SQL
  statements through one `execute_database_batch(..., true)` call after shared
  schema/storage validation; capabilities and project grants are checked before
  provider access. Differential fake-host tests cover query and transaction
  binding, atomic batch selection, provider errors, storage validation, missing
  database context, and denied access without a real database. An outbox call with a rendered PDF attachment is tested in a transaction
  batch with SQL; MIR itself does not send mail.


`HostServices` currently exposes nine service groups:

- `unix_time_millis()` for `now()`; values outside Zelyra's timestamp range
  are rejected.
- `environment_variable(name)` for `env(name)`; returned values must be UTF-8.
- `fill_random(buffer)` for `random_int(...)` and `uuid_v4()`; the runtime
  retains range validation and UUID formatting.
- `read_console_line(prompt)` for `read_console(...)`; the runtime capability
  check runs before the host call. The system implementation serializes access
  to stdin and writes the prompt to stdout. Hosts without console input reject
  the call explicitly.
- `http_request(method, url, headers, body, timeout_ms, max_response_bytes)`
  for `http_get`, `http_request`, `http_json`, and `http_result`. The runtime
  checks the capability, URL scheme, host allowlist, and headers before the
  call, then rechecks response size and UTF-8 afterward. Request and response
  headers are each limited to 65,536 bytes, and timeout/response limits must
  be positive. The system host disables redirects and bounds response reads.
- `run_process(command, arguments, timeout_ms, max_output_bytes)` for
  `run_process(...)`; the runtime checks capability, allowlist, and positive
  limits before calling the host. The system implementation avoids a shell,
  clears the process environment, bounds stdout/stderr capture, and terminates
  timed-out processes.
- Filesystem services for path canonicalization, metadata, text reads/writes,
  exclusive and atomic writes, append, copy, rename, delete, and directory
  listing. After host canonicalization, the runtime still checks that the path
  is inside configured roots. Capabilities and project grants are checked
  before provider access. Default canonicalization uses the system filesystem;
  virtual hosts must override it while preserving symlink/traversal safety.
  Optional I/O services that are not implemented fail explicitly.
- `deliver_outbox_message(...)` for SMTP transport of outbox messages. The
  runtime checks the Email capability and SMTP host allowlist before the call;
  the system host revalidates the payload and mail configuration and delegates
  delivery to `zelyra-mail`.
- Database queries, transaction batches, and required Decimal/temporal storage
  validation. The runtime checks SQL capabilities before passing the database
  to the host. SQL is already parsed and bound; the host receives Query objects
  with separate parameters. Storage validation reads metadata and does not
  change schemas. The system host delegates to `zelyra-database`; a host without
  database support fails explicitly when such operations are requested.

Existing `execute` APIs use `SystemHostServices`, preserving the system clock,
environment, cryptographic random source, and console input. Embedders and
tests can inject a host:

```rust,ignore
let host: Arc<dyn HostServices> = Arc::new(MyHost);
let output = execute_with_host_services(
    &program,
    host,
    Some(&grants),
    Some(&policy),
)?;
```

The exact public signatures are in `runtime/src/lib.rs`. A host must implement
`Send + Sync`, because parallel Zelyra expressions share the same host object.

For an embedding that already owns a request transaction, the AST runtime also
provides `execute_function_captured_with_transactional_query`. It routes every
`sql<T>` expression through the supplied borrowed query callback while keeping
ordinary capability and storage checks. The callback is scoped to one bounded
runtime execution; nested `transaction { ... }` and `parallel { ... }` blocks
are rejected in this mode so they cannot escape to another connection. This
is a low-level embedding API; web CRUD actions do not use it yet.

## Security boundary and status

Zelyra function capabilities and project grants are checked before each of
these provider calls. A host cannot bypass those checks through a capability
declaration. Provider errors return as runtime diagnostics at the original
source span. Tests use a deterministic provider and also verify that denied
capabilities do not invoke it.

AST, MIR, and the versioned Bytecode VM route their supported external effects
through `HostServices`. The current tested Bytecode subset includes clock,
environment, console, random, allowlisted process execution, root-checked
filesystem operations, checked HTTP requests, mail delivery, and selected SQL
and transactional outbox operations. The Bytecode verifier records declared
capabilities and checks grants before host calls; `execute` grants none by
default. MIR and Bytecode still support only documented subsets, and some
operations require an explicit `RuntimePolicy`. Unsupported operations fail
closed. This API is experimental: it is a versioned Rust embedding contract,
not a stable native ABI or a package plugin ABI. See the Bytecode guide for
the exact operation subset and remaining parity limits.
