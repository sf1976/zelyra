# Zelyra 0.1

Zelyra is a statically typed language for business, database, and web applications.
This repository contains the Phase 1 language core: lexer, parser, AST, static
type checker, interpreter, and the `zelyra` CLI.

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

See [docs/getting-started.md](docs/getting-started.md) for the complete
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

## Current limitations

The Phase 1 implementation is an interpreter-backed compiler prototype. It
does not yet include database, SQL, web, forms, CRUD, capabilities, contracts,
or code generation. Those belong to later phases of the specification.

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
