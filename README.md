# Zelyra

![Zelyra logo](assets/zelyra-logo.png)

**From database to application.** · **AI writes. Zelyra verifies.**

[Deutsch](README.de.md) · English

Zelyra is an independent, statically typed language and application platform
for database-backed business and web applications. It brings schema-aware SQL,
forms, CRUD, views, and application logic together while keeping ordinary code
and native SQL available.

> **Release 1.2.5 · language compatibility line: 0.1** · [GitHub release](https://github.com/sf1976/zelyra/releases/tag/v1.2.5)
>
> Compiler checks and the verifier cover defined scopes. They do not prove arbitrary applications correct. Review the documented support and limits for each workflow before deployment.

Zelyra 1.2.5 adds signed source-package registries with publisher trust, a
fixed-port Linux systemd service with its resident scheduler, and confined
static files from `public/`. See the [1.2.5 release notes](docs/release-notes/1.2.5.en.md),
[package guide](docs/packages.en.md), and [deployment guide](docs/deployment.en.md).
Zelyra 1.1.5 added the local offline package workflow with deterministic locks
and typed settings. Zelyra 1.1.2 removed the experimental
Laravel analyzer and scaffold command; see the [1.1.2 release notes](docs/release-notes/1.1.2.en.md). Zelyra 1.1.1
includes the Studio fixes described in the
[1.1.1 release notes](docs/release-notes/1.1.1.en.md). Zelyra 1.1.0 added
offline, embedded Studio language services and a standalone
LSP, plus typed string interpolation and opt-in in-place mutation for local
arrays. The 1.0.7 language features remain available. See the
[1.1.0 release notes](docs/release-notes/1.1.0.en.md),
[UI guide](docs/ui.en.md), [database support matrix](docs/database-support.en.md),
and [implementation inventory](docs/implemented.en.md).

The language compatibility line remains `0.1`; compiler releases do not change it automatically. “Prove correctness” is a design goal: the verifier distinguishes proven, runtime-checked, and unproven scopes, but is not a general-purpose theorem prover.

## What Zelyra 1.2.5 provides

Zelyra connects its language, compiler, database, web application, and Studio in
one workflow. Release 1.2.5 retains the 1.1.5, 1.1.2, 1.1.1, and 1.1.0
capabilities and adds operations and package features:

- statically checked business logic and schema-aware SQL;
- MariaDB as the primary runtime, plus verified MySQL and PostgreSQL workflows;
- generated forms, table views, CRUD, relationships, and APIs;
- the expanded 1.0.3 standard library for math, text, arrays, sets, maps, date
  arithmetic, parsing, encodings, and paths, plus `|>` pipelines and flat
  `else if` chains;
- offline Studio with CodeMirror 6, compiler diagnostics and language services,
  semantic navigation/refactoring, verifier status, a project data map,
  typed-hole assistance, isolated UI/theme preview, capability review,
  runtime budgets, PDF preview, and MariaDB outbox inspection/requeue;
- standalone `zelyra lsp` using the same project overlays and semantic service;
- Studio-accessible, redacted local JSONL run and preview logs, plus
  `zelyra uninstall` for the active user-owned installation;
- Zelyra UI with compiler-checked components, generated CRUD styling,
  light/dark themes, compact density, local asset delivery, print styles, and
  text-equivalent bar, line, and donut charts;
- an invoice tutorial with repeatable sample data, available-port selection,
  isolated Compose volumes, a task-focused home page, date-only invoice input,
  server-assigned invoice numbers, and complete English and German guides;
- CLI tools for routes, setup, diagnostics, databases, audit, and SMTP;
- a Linux systemd user service with fixed ports, status/restart/remove, and
  checked port changes with rollback; ports are never changed automatically;
- a resident scheduler that runs only in the active installed service,
  idempotent outbox setup, and `zelyra schedule status` without host cron;
- signed source packages with explicit publisher trust, registry search, exact
  updates, lockfile provenance, and portable Docker bundles;
- static `public/` files with MIME types, GET/HEAD, route precedence, and
  path/symlink confinement, plus documented Apache/Nginx proxy deployment;
- offline local packages with SemVer compatibility, deterministic source locks,
  explicit imports, checked package plans/bundles, typed settings, and
  `zelyra module init`; reusable bounded CSV import/export examples;
- if expressions, record updates, typed lambdas, guarded nested patterns,
  tuples, string interpolation, and opt-in mutable local arrays;
  `--max-steps` stops unbounded evaluation with a source diagnostic;
- release limits are listed in the [1.2.5 release notes](docs/release-notes/1.2.5.en.md);
  a general transactional job and query system is not included yet;
- local text-PDF rendering and checked PDF outbox attachments with byte and
  row limits. The renderer does not preserve CSS layout, images, or charts;
  use browser print for graphical documents.

Docker Engine and Docker Compose plugin v2 or newer must be installed; Python
Compose v1 is unsupported. Zelyra does not install these packages. Check with
`docker compose version`. The [invoice tutorial guide](docs/tutorials/invoice.en.md)
covers startup, editing in Studio, sample data, and troubleshooting.

The [implementation inventory](docs/implemented.en.md) describes supported
scope and its limits. The [roadmap](docs/ROADMAP.md) tracks planned work;
[release notes](docs/release-notes/) and the [changelog](CHANGELOG.md) record
published changes.

Use `zelyra uninstall --dry-run` to inspect which active user-owned executable
would be removed. Read the [uninstall guide](docs/uninstall.en.md) first for
package-manager and duplicate-PATH behavior.

## Start here

### Try the language

From a source checkout:

~~~bash
git clone https://github.com/sf1976/zelyra.git
cd zelyra
./install.sh
zelyra run examples/fibonacci.zyl
~~~

Expected output: `55`. Published Linux and Windows x86_64 binaries can also be
installed without a Rust toolchain; see the [release downloads](https://github.com/sf1976/zelyra/releases)
and the installation guide.

For an interactive CLI example, run `zelyra run examples/console_input.zyl`.
It uses the explicit `Console` capability; the example's project grant is
enabled in the repository's `zelyra.toml`. See the [I/O chapter in the
handbook](docs/handbook/en/textbook-foundations.md#chapter-8-input-and-output) for new
projects.

### Start a MariaDB CRUD application

~~~bash
zelyra new machine-management --template mariadb-crud
cd machine-management
zelyra setup --all
~~~

The setup command reports the local application address. This path requires
Docker Engine and Compose to be installed and accessible; Zelyra does not
install or modify Docker. The [setup guide](docs/setup-web.md) explains the
console and browser setup flows, permissions, ports, and troubleshooting.

Generated `.env` files contain local credentials in plaintext. They are
excluded from Git and Docker build contexts; Zelyra sets newly created files to
owner-only permissions (`0600`) on Unix. On Windows, the file inherits the
directory's ACL, and existing `.env` files are not re-permissioned. Do not
commit or share the file. For production, use an appropriate secrets manager
and deployment-specific credentials. Review the documented workflow,
security requirements, and limitations before production use.

The generated MariaDB project defaults to German and learn mode. Set
`ZELYRA_LANGUAGE=de|en` and `ZELYRA_LEVEL=learn|work` in its `.env` to change
the generated interface. See the environment reference for precedence and
other settings.

## Documentation

| If you want to… | Read |
| --- | --- |
| Learn Zelyra step by step | [Getting started](docs/getting-started.md) · [English handbook](docs/handbook/en/handbook.md) · [Deutsches Handbuch](docs/handbook/de/handbuch.md) |
| See current implementation and its limits | [Implemented capabilities](docs/implemented.en.md) · [Deutsch](docs/implemented.de.md) |
| Check what the language specifies | [Language specification](docs/specification.md) · [Source authority and validation guide](docs/source-authority.md) |
| Configure a project or its environment | [Environment and configuration reference](docs/env.en.md) · [Deutsche Referenz](docs/env.md) |
| Understand database support | [Database support matrix](docs/database-support.en.md) · [German](docs/database-support.de.md) |
| Use the local editor or preview a safe regex edit | [Zelyra Studio](docs/web-editor.en.md) · [Text edit command](docs/text-edit.en.md) |
| Use Zelyra UI | [Components, forms, themes, print, charts, and email reference](docs/ui.en.md) · [Deutsch](docs/ui.de.md) |
| Review current and planned work | [Roadmap](docs/ROADMAP.md) · [1.0.7 release notes](docs/release-notes/1.0.7.en.md) |
| Read the design direction | [Manifesto](docs/MANIFESTO.md) · [AI-native architecture](docs/architecture/ai-native-development.md) · [Readable regex text-edit design](docs/design/readable-regex-text-editing.en.md) |
| See exact release changes | [Changelog](CHANGELOG.md) · [GitHub releases](https://github.com/sf1976/zelyra/releases) |

The handbook contains tutorials and detailed language/runtime examples. The
formal specification and compiler tests define what is accepted today; the
roadmap distinguishes implemented, partial, planned, optional, and deferred
work. Zelyra syntax must not be inferred from Rust or another language.

## Product principles

- **Database-first, MariaDB-first:** schema, checked native SQL, forms, and
  generated business interfaces share typed information. SQLite is supported
  for tested workflows; PostgreSQL 16 has a bounded experimental direct-SQL
  runtime, not full parity or universal database portability.
- **Views are part of the language platform:** reusable layouts, components,
  slots, generated CRUD views, localization, and project-local style
  customization are evolving together. The roadmap documents remaining gaps.
- **AI-native, not AI-dependent:** compiler diagnostics, project context,
  impact analysis, and formatting provide machine-usable tools. No AI provider
  or cloud service is required; compiler checks and tests remain authoritative.
- **Digital sovereignty:** local execution and data control, no unsolicited
  telemetry, explicit security boundaries, and evidence-based claims guide the
  project. These are requirements and goals, not claims that every objective is
  already complete.

## License and implementation

The compiler is currently implemented in Rust; Rust is not Zelyra's source
language or product identity. Zelyra source is licensed under either the MIT
License or Apache License 2.0, at the licensee's option. See [LICENSE](LICENSE),
[LICENSE-MIT](LICENSE-MIT), and [third-party notices](THIRD_PARTY_NOTICES.md).
