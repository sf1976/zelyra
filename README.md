# Zelyra

![Zelyra logo](assets/zelyra-logo.png)

**From database to application.** · **AI writes. Zelyra verifies.**

[Deutsch](README.de.md) · English

Zelyra is an independent, statically typed language and application platform
for database-backed business and web applications. It brings schema-aware SQL,
forms, CRUD, views, and application logic together while keeping ordinary code
and native SQL available.

> **Current compiler release: 0.2.0 · language compatibility line: 0.1 · experimental**
>
> Zelyra 0.2.0 is not approved for production use. “Prove correctness” is a
> design goal: the current verifier covers a bounded subset and does not prove
> arbitrary applications correct.

## What changed in 0.2.0

This release adds reusable view layouts and named component slots,
project-local German and English UI catalogs, stricter schema-drift checks with
review gates and safe preflights, and additional MariaDB business-application
coverage. Browser-origin checks and a configurable Host allowlist strengthen
the web runtime. Linux and Windows release archives include SHA-256 checksums;
repeated builds are byte-identical in the pinned CI toolchains.

See the [0.2.0 changelog](CHANGELOG.md) for release notes and the
[roadmap](docs/ROADMAP.md) for the implementation status and limitations. These
links are the source of detail; this README is intended as a project overview,
not a second handbook or changelog.

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
and deployment-specific credentials; 0.2.0 is not production-approved.

The generated MariaDB project defaults to German and learn mode. Set
`ZELYRA_LANGUAGE=de|en` and `ZELYRA_LEVEL=learn|work` in its `.env` to change
the generated interface. See the environment reference for precedence and
other settings.

## Documentation

| If you want to… | Read |
| --- | --- |
| Learn Zelyra step by step | [Getting started](docs/getting-started.md) · [English handbook](docs/handbook/en/handbook.md) · [Deutsches Handbuch](docs/handbook/de/handbuch.md) |
| See what works in compiler 0.2.0 | [Implemented capabilities](docs/implemented.en.md) · [Deutsch](docs/implemented.de.md) |
| Check what the language specifies | [Language specification](docs/specification.md) · [Source authority and validation guide](docs/source-authority.md) |
| Configure a project or its environment | [Environment and configuration reference](docs/env.en.md) · [Deutsche Referenz](docs/env.md) |
| Understand database support | [Database compatibility matrix](docs/database-compatibility.en.md) · [German](docs/database-compatibility.de.md) |
| Review current and planned work | [Roadmap](docs/ROADMAP.md) · [Release plan 0.3.0](docs/release-plans/0.3.0.en.md) |
| Read the design direction | [Manifesto](docs/MANIFESTO.md) · [AI-native architecture](docs/architecture/ai-native-development.md) |
| See exact release changes | [Changelog](CHANGELOG.md) · [GitHub releases](https://github.com/sf1976/zelyra/releases) |

The handbook contains tutorials and detailed language/runtime examples. The
formal specification and compiler tests define what is accepted today; the
roadmap distinguishes implemented, partial, planned, optional, and deferred
work. Zelyra syntax must not be inferred from Rust or another language.

## Product principles

- **Database-first, MariaDB-first:** schema, checked native SQL, forms, and
  generated business interfaces share typed information. SQLite is supported
  for tested workflows; PostgreSQL schema support does not imply full runtime
  parity or universal database portability.
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
