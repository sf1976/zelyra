# Zelyra 0.1

[Deutsch](README.de.md) · English

Zelyra is a statically typed language for business, database, and web applications.
This repository contains the Phase 1 and Phase 2 language core plus the Phase 3
Database Core: lexer, parser, AST, HIR, static type checker, interpreter, schema
compiler, and the `zelyra` CLI.

## Quick start

For a source checkout, the easiest installation is:

```bash
./install.sh
zelyra run examples/fibonacci.zyl
```

The installer uses the current user account, installs Rust locally only when
needed, and places the binary in `~/.local/bin` by default. No root access,
Apache installation, database server, or global system configuration is
required for the Phase 1 language core.

See the [English getting-started guide](docs/getting-started.md) or the
[German getting-started guide](docs/getting-started.de.md) for the complete
beginner path and the planned web deployment modes.

```bash
cargo run --bin zelyra -- run examples/fibonacci.zyl
```

Expected output:

```text
55
```

Check a program without running it:

```bash
cargo run --bin zelyra -- check examples/fibonacci.zyl
```

## Phase 1 syntax

Bindings are immutable by default. A binding is introduced with `name = value`
or an explicit type annotation. Mutable bindings use `mutable`:

```zelyra
fn main() {
    name: String = "Zelyra"
    mutable counter = 0

    while counter < 3 {
        print(name)
        counter = counter + 1
    }
}
```

Supported Phase 1 constructs include primitive values, functions, calls,
arithmetic and boolean expressions, `if`/`else`, `while`, `loop`, `break`,
`return`, and `print`.

Phase 2 adds nominal types, `Option`/`Result`, and exhaustive pattern matching.
See [docs/phase-2.md](docs/phase-2.md) or the
[German Phase 2 guide](docs/phase-2.de.md).

Phase 3 adds the Database Core for MariaDB, PostgreSQL, and SQLite. MariaDB is
the default backend for new projects. See
[docs/phase-3.md](docs/phase-3.md) or the [German Phase 3 guide](docs/phase-3.de.md).

Phase 4 adds native SQL blocks with schema, column, and parameter checking plus
MariaDB runtime execution. See [docs/phase-4.md](docs/phase-4.md) or the
[German Phase 4 guide](docs/phase-4.de.md).

## Current limitations

SQL, web, forms, CRUD, capabilities, contracts, and code generation belong to
the following phases. The Database Core already supports schema DDL, inspection,
diff, planning, and applying for PostgreSQL, MariaDB, and SQLite.

## Easy deployment principle

Web applications should run with one Zelyra command during development. A
Zelyra project must not require Apache merely to get started. The planned
deployment modes are:

```text
zelyra dev                 Built-in development server
zelyra serve               Standalone production server
zelyra web apache          Generate an Apache reverse-proxy configuration
```

Apache will remain an optional integration for existing infrastructure. The
same application should also be deployable behind nginx, Caddy, a cloud load
balancer, or directly on the Zelyra server. These commands are intentionally
documented as roadmap items until the Web Core phase implements them.
