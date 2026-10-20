# Zelyra Language Console

`zelyra console [entry.zyl]` starts an interactive REPL. It defaults to `main.zyl`. The entry file is loaded and checked as with `zelyra check`; its declarations and functions are available in the session. AST is the default; `--backend bytecode` selects the experimental bytecode VM for its supported subset. Unsupported programs are rejected with a backend diagnostic. Both backends use the same parser, typechecker, persistent scope, and runtime policies.

An executable example is available at [examples/language_console.zyl](../examples/language_console.zyl).

```sh
zelyra console main.zyl                 # sandbox is the default
zelyra console main.zyl --persist       # keep database changes
zelyra console main.zyl --tenant 42
zelyra console main.zyl --backend bytecode
```

Each input is parsed, typechecked, and executed independently. Successful bindings persist:

```text
>> x = 10
>> y = 20
>> x + y
=> 30
```

In a terminal, Reedline provides ANSI colors for keywords, strings, and numbers; history at `~/.zelyra_history`; tab completion for project functions, tables, records, fields, and common standard functions; and `>>`/`...` prompts. `NO_COLOR` and `ZELYRA_COLOR=never` disable color. Pipe input omits prompts. Inputs containing quotes, SQL, or recognizable secret keywords are excluded from saved history. Syntax, type, and runtime failures report a two-line diagnostic with a code, source location, and message, and leave the next input available.

## Commands

- `:help` lists commands.
- `:tables` lists declared tables. `:schema <table>` shows columns, types, key/required markers, and declared invariants with their source location and failure message.
- `:tenant <id>` sets both `tenant_id` and `session.tenant_id`. The typed `session.tenant_id` field is `Option<Int>` (`None` until a tenant id is set); this does not grant authorization or add automatic filters.
- `:commit` commits the current sandbox transaction and starts another.
- `:rollback` rolls back the current sandbox transaction and starts another.
- `:clear`, `:quit`, and `:exit` clear or exit the session.

## Database safety and limitations

By default, MariaDB/MySQL sessions hold one real transaction on a single connection. Each input also gets a savepoint: if runtime execution fails, only that input's database changes are rolled back. The sandbox accepts only `SELECT`, `INSERT`, `UPDATE`, and `DELETE`. DDL such as `CREATE`, `ALTER`, and `DROP`, plus transaction and session-control statements, are rejected because MariaDB/MySQL may implicitly commit them. On exit, the open transaction is rolled back. `:commit` is an explicit exception: already committed changes remain; later changes are still rolled back on exit. `--persist` uses the regular database path without a sandbox transaction.

If Zelyra cannot safely create, release, or roll back an input savepoint, it disables further expressions in that session. Exit the console so the outer sandbox transaction can be rolled back and the connection discarded if necessary. Zelyra will not execute more SQL after an uncertain rollback state.

The sandbox transaction starts only when a project URL is resolved from the process environment or `.env`. Without a connection, database-free expressions work; database access fails as in normal runtime execution. Console SQL currently requires the aggregate `Database` capability grant (`database = true`, or both `database_read = true` and `database_write = true` in `zelyra.toml`). A read-only grant alone does not yet allow database queries through the REPL; the console does not broaden project grants. AST and bytecode sandbox SQL share the same MariaDB/MySQL connection and per-input savepoints. PostgreSQL sandbox transactions are not supported. `--tenant` exposes a `tenant_id` value in scope but does not provide database row-level security. Capabilities, project runtime policy, compiler checks, and SQL parameter binding remain active.

The MariaDB integration test `tests/language-console-mariadb-e2e.sh` creates an isolated disposable database and verifies implicit rollback, `:rollback`, `:commit`, and `--persist`. It requires Docker and the `mariadb:11.8` image (or a compatible image selected through `ZELYRA_REPL_MARIADB_IMAGE`).

## Multiline input

Continuation is detected from unclosed strings and curly, square, or round delimiters. Delimiters inside strings and single-line comments are ignored. An incomplete pipe at EOF reports a parse error. `Ctrl+C` discards only the current input buffer; `Ctrl+D` or `:quit` exits the session.
