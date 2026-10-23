# The Zelyra Handbook

**From foundations to database-backed web applications. State intent. Prove correctness.**

English · [Deutsche Ausgabe](/handbuch)

Welcome to the complete Zelyra Handbook. It includes both the introductory textbook **"Learning Zelyra – Understandable Programming from the Foundations to Your Own Application"** (Parts I to X, Chapters 1 to 43), the **Technical Reference Manual** (Sections 1 to 36), and comprehensive **Appendices** (A to K).

## Zelyra 1.2.24: current scope

This handbook describes **Zelyra 1.2.24** (as of October 19, 2026) with an outlook toward the target milestone 1.3.0. Zelyra 1.0.0 was its first stable major release; the language compatibility line remains `0.1`. Individual compiler, database, operational, and IDE capabilities have defined limits described in their relevant sections. The experimental Laravel migrator (`zelyra migrate laravel`) was removed entirely in version 1.1.2; Zelyra is strictly focused on its native, contract-driven architecture.

- **Language and type system:** typed functions, immutable bindings, user-defined generic records and enums, payload-carrying enums, named function values (`&name`), exhaustive `match` checking, exact 9-scale decimal arithmetic (`Decimal`), typed calendar/temporal values (`Date`, `Time`, `DateTime`, `Instant`), contracts (`requires`, `ensures`), and declared capabilities.
- **Language extensions and runtime:** typed value-producing `if` expressions, functional record updates (`Customer { ..old, active: true }`), typed lexical lambdas in pipelines (`map`, `filter`, `find`, `reduce`), boolean match guards and nested record patterns, first-class tuples and destructuring bindings, contextual field names (`version`, `type`), nominal Id aliases (`type OrderId = Id`), configurable execution step budgets (`--max-steps N`), server-side local text-PDF rendering (`render_pdf`), and transactional PDF mail attachments via MariaDB outbox.
- **Databases and relational CRUD:** multi-backend support for tested workflows across MariaDB 11.4.13, MySQL 8.4.11, and PostgreSQL 16.15. Typed parameterized SQL, transactions with dynamic deadline-bound locks (`innodb_lock_wait_timeout`), real foreign key constraints with orphan row rejection (`error[E-DB-005]`), relational joins, tenant-scoped CRUD, ordered restrictive deletion (RESTRICT), and schema previews with controlled apply boundaries (`zelyra db plan/apply/history`).
- **Tenant isolation and composite foreign keys (v1.2.17 – v1.2.19):** `table ... scoped by tenant_id: TenantId` enforces multi-tenancy at the database level. Zelyra generates `uq_{table}_tenant_id UNIQUE (tenant_id, id)` and composite foreign keys `FOREIGN KEY (tenant_id, fk_id) REFERENCES target (tenant_id, id) ON DELETE RESTRICT`. `unique within tenant` declares tenant-scoped uniqueness (`customer_number: String(50) required unique within tenant`), and optional relations (`Customer?`) generate typed `BIGINT NULL`.
- **Table invariants and typed validation (v1.2.20):** `table ... { validate { due_date >= invoice_date: "..." } }` anchors typed, cross-field invariants directly to the data schema. Declarative forms, CRUD, and mutating APIs (`validates`) enforce these rules before committing writes, returning standardized HTTP 422 contracts.
- **Lifecycle guards and row policies (v1.2.21):** Declarative locks for `edit` and `delete` (`guard edit when is_locked == true { error: "..." }`) block illegal mutations directly inside the mutation transaction. `tableview` action slots and detail pages dynamically disable or hide edit and delete actions based on row state.
- **Transactional master-detail CRUD forms and multiple child collections (v1.2.22 – v1.2.24):** `has_many items: InvoiceItem { ... }` and additional collections (`has_many attachments: InvoiceAttachment { ... }`) render server-side 1:N line-item grids and attachments with atomic parent-child transactions and automated server-side summary aggregates (`aggregates`).
- **Validated tenant APIs (v1.2.23):** Tenant-scoped APIs (`scoped by tenant_id`) strictly validate tenant context from authenticated user sessions.
- **Canonical project layout (v1.2.13 – v1.2.14):** `zelyra new --layout standard` scaffolds a verified directory layout (`routes/`, `domain/`, `schema/`, `public/`, `assets/`). Architecture linting (`strict_architecture = true`) checks declaration placement (`E-ARCH-001`), writes or unclassified SQL in statically reachable API handlers (`E-ARCH-002`/`003`), and rejects route imports in domain modules (`E-ARCH-004`); `zelyra migrate-layout --check` provides a read-only preview.
- **Extensible CRUD and TableViews (v1.2.15 – v1.2.16):** compiler-checked extension regions, field help, and row action slots on generated `crud.view.create`, `crud.view.edit`, and `tableview` pages while preserving generated routing, validation, CSRF, and tenant scoping.
- **Systemd services, scheduler, and jobs (v1.2.5 – v1.2.9):** `zelyra service install --port <port>` installs a managed Linux systemd user service with fixed-port verification. The resident scheduler processes declared crons, dispatching persistent background jobs with renewable leases to the MariaDB outbox (`zelyra schedule jobs`).
- **Package ecosystem (v1.1.5 – v1.2.12):** reusable local and registry packages (`pkg:`), offline lockfile (`zelyra.lock`), Ed25519-signed packages, recoverable mutations with transactional journal rollback (`zelyra package recover`), and authenticated app-level module settings (`/__zelyra/module-settings`).
- **Zelyra UI & Design System:** locally bundled, compiler-checked design system without npm or Webpack (`zelyra-ui.v1.css`). Typed UI components, themes (`light`, `dark`, `system`), harmonized app shell, modal dialogs, and responsive print layouts.
- **Zelyra Studio (IDE):** local editor with session protection, saving, live diagnostics, contextual help, action finder, code completion, navigation, bounded execution, structured logs panel, theme editor, and CRUD preview.
- **Invoice tutorial:** customers, items, invoices, and line items in a unified Tabler/Zelyra UI theme with an interactive "Code Explained" dialog, aligned cards, and browser printing.
- **Email and security:** transactional MariaDB outbox (`mail_queue`), SMTP worker with TLS and CA validation, header-injection protection, restart-resilient persistent password reset limits (`password_reset_limits`), and integrated TOTP two-factor authentication (RFC 6238, AES-256-GCM, Argon2id recovery codes).
- **Notifications:** tenant- and recipient-scoped in-app center (`recipient: user_id`), dynamic unread counter (`unread: read_at`), personal CSRF-protected read receipts with audit logging, expiry filtering, and archiving.
- **Dual-tier execution pipeline (Roadmap 1.3.0):** AST interpreter serves as a reference oracle for development and differential testing; high-speed pipeline lowers typed MIR into versioned bytecode (v24) executed by an iterative Rust VM.

### Changes since version 1.0.0

- **1.0.1:** Compose v2 is supported; Python Compose v1 is rejected with an update hint. Studio's Run button starts the project entry point.
- **1.0.2:** Studio can start MariaDB for the local invoice tutorial, apply its schema, and load sample data. `for` ranges accept step sizes; `range(start, end, step)` returns the corresponding sequence.
- **1.0.3:** The standard library expanded across math, text, arrays, sets, maps, dates, parsing, encodings, and paths. The release also adds type-checked `|>` pipelines, flat `else if` chains, schema display while the database is unavailable, automatic tutorial setup, free ports, repeatable sample data, and bounded recursion. Full signatures are in [Appendix D](technical-reference.md#appendix-d-standard-library-overview-zelyra-103).
- **1.0.4:** Invoice tutorials receive a dedicated Compose project name. Older volumes can fall back to an isolated database without deleting or changing the previous volume. Compose detection accepts short versions such as `2.40`.
- **1.0.5:** Studio adds local structured Logs; `zelyra uninstall` safely removes the active user-owned binary after confirmation. The invoice tutorial adds date-only input, server-assigned invoice numbers, a workflow home, a named invoice overview, and detailed English and German source guides.
- **1.0.6:** Zelyra UI supplies compiler-checked components and locally bundled styles for generated CRUD, starter projects, Studio properties, and the invoice tutorial. It adds themes, responsive layouts, chart components, browser print/PDF, and prefilled email drafts.
- **1.0.7:** Value-producing `if` expressions, record updates (`Customer { ..old, active: true }`), typed lexical lambdas for pipelines (`map`, `filter`, `find`, `reduce`), boolean match guards, nested record patterns, tuple binding, step budgets (`--max-steps`), nominal Id aliases, server-side text PDF rendering (`render_pdf`), and transactional PDF mail attachments via the outbox.
- **1.1.0:** Studio additions with typed-hole suggestions, data schema map, static page preview without code execution, and theme styling editor.
- **1.1.1:** CLI and editor stability improvements for JSON-RPC diagnostics.
- **1.1.2:** **Removal of the experimental Laravel analyzer and scaffold command (`zelyra migrate laravel`).** The migrator and previous reports were completely removed from Zelyra in favor of native contract-driven architecture.
- **1.1.5:** Reusable local packages with typed public APIs, offline lockfiles, module inspection and bundling, local package settings, and CSV examples.
- **1.2.0:** Operational and deployment foundation, declarative commands and schedules, static `public/` directory with MIME delivery, Apache/Nginx reverse proxy guides, and fixed-port service installation.
- **1.2.5:** Fixed-port Linux systemd service management (`zelyra service`), resident scheduler, Ed25519-signed source packages, and idempotent outbox initialization.
- **1.2.6:** Recoverable package mutations: serialized transactions, versioned journals, and quarantine recovery via `zelyra package recover`.
- **1.2.7:** Cryptographically signed transitive registry packages with whole-closure trust checking and cycle detection.
- **1.2.8:** Typed SQL metadata lowering from HIR to MIR with plan re-verification (`E-MIR-VERIFY-004`) and strict form redirect validation to local absolute paths (`E-FORM-007`).
- **1.2.9:** Durable scheduler jobs in the MariaDB outbox; serial lease worker; job inspection with `zelyra schedule jobs`.
- **1.2.10:** Serialized module settings persistence in Studio preventing secret leakage; versioned HostServices embedding contract.
- **1.2.11:** Unified Zelyra UI styling for invoice tutorial, CLI SIGPIPE compliance, and improved Compose v2 packaging support.
- **1.2.12:** Authenticated application-level module settings page (`/__zelyra/module-settings`), automatic modal dialog defaults, and direct theme file access in Studio.
- **1.2.13:** Canonical project layout (`zelyra new --layout standard`) with `routes/`, `domain/`, `schema/`, `public/`, `assets/`, and namespace imports (`import "domain::sales::actions"`).
- **1.2.14:** Deterministic `zelyra migrate-layout --check` preview, `strict_architecture = true` linter (`E-ARCH-001`), styled invoice document, and "Code Explained" dialog.
- **1.2.15:** Extensible generated CRUD create/edit pages (`crud.view.create`, `crud.view.edit`), field help regions, and row actions in `tableview`.
- **1.2.16:** Typed TableView cell content and formatted row action slots with HTML templates and safe escaping.
- **1.2.17:** Enforced tenant isolation (`scoped by tenant_id: TenantId`), composite foreign keys `FOREIGN KEY (tenant_id, fk_id) REFERENCES target (tenant_id, id) ON DELETE RESTRICT`, typed optional relations (`Customer?`), and exact commercial decimal scaling (`Decimal(p, s)`).
- **1.2.18 / 1.2.19:** Tenant-scoped uniqueness `unique within tenant` with automatic composite MariaDB unique index `UNIQUE (tenant_id, column)`.
- **1.2.20:** Typed cross-field table invariants (`validate { condition: "error" }`) in forms and CRUD, plus typed API input validation with `validates`.
- **1.2.21:** Record-dependent lifecycle guards (`guard edit when ...`, `guard delete when ...`) and policy evaluation in detail views and TableView slots.
- **1.2.22:** Transactional master-detail CRUD forms (`has_many`) with atomic saving of parent and child records, cascade deletion, and server-side aggregates (`aggregates`).
- **1.2.23:** Validation of tenant-scoped APIs (`scoped by tenant_id`) from authenticated user sessions; robust tenant isolation at API boundaries.
- **1.2.24:** Support for multiple direct `has_many` child collections per CRUD resource (e.g. items and attachments) with atomic saving, independent row limits, and server-side aggregates.
- **Milestone 1.3.0 outlook:** Canonical vertical slice standard, dual-tier pipeline with Bytecode VM v24, single-binary zero-ops distribution (`zelyra build --bundle`), audit trails (`auditable` with `as_of` time travel), and zero-sum ledgers (`ledger`).

Release details: [1.0.1](https://github.com/sf1976/zelyra/blob/main/docs/release-notes/1.0.1.en.md), [1.0.2](https://github.com/sf1976/zelyra/blob/main/docs/release-notes/1.0.2.en.md), [1.0.3](https://github.com/sf1976/zelyra/blob/main/docs/release-notes/1.0.3.en.md), [1.0.4](https://github.com/sf1976/zelyra/blob/main/docs/release-notes/1.0.4.en.md), [1.0.5](https://github.com/sf1976/zelyra/blob/main/docs/release-notes/1.0.5.en.md), [1.0.6](https://github.com/sf1976/zelyra/blob/main/docs/release-notes/1.0.6.en.md), and [1.0.7](https://github.com/sf1976/zelyra/blob/main/docs/release-notes/1.0.7.en.md). The [Studio guide](https://github.com/sf1976/zelyra/blob/main/docs/studio.en.md), [local logs guide](https://github.com/sf1976/zelyra/blob/main/docs/studio-logs.en.md), [uninstall guide](https://github.com/sf1976/zelyra/blob/main/docs/uninstall.en.md), [mail guide](https://github.com/sf1976/zelyra/blob/main/docs/mail.en.md), and [database matrix](https://github.com/sf1976/zelyra/blob/main/docs/database-support.en.md) describe additional capabilities and limits.

### Development status and agreed architecture

Zelyra is at stable release **1.2.24** with active development towards milestone **1.3.0**. The [Zelyra roadmap](https://github.com/sf1976/zelyra/blob/main/docs/ROADMAP.md) is the source of truth for ongoing progress.

- **Resident schedules and integrated scheduler (✅ Implemented since 1.2.5/1.2.9):** Commands and five-field numeric schedules (`schedule <name> { cron: "..."; timezone: "..."; command: ... }`) are declared directly in Zelyra source code. For instance, a weekly run at noon on Saturday uses `cron: "0 12 * * 6"` and `timezone: "Europe/Berlin"`. The built-in resident scheduler runs automatically within the active Zelyra service (`zelyra service` or `zelyra serve`). At each minute boundary, it evaluates pending schedules and enqueues jobs transactionally into the MariaDB outbox. A serial lease worker (`zelyra schedule jobs`) processes pending tasks with lease renewals and bounded retries. Manual host `crontab -e` configuration is **not required** for regular operation. The `zelyra schedule export` command serves only as an optional preview or export for external third-party systems; Zelyra never silently touches external crontabs.
- **Operating system services (✅ Implemented):** `zelyra service install --port <port>` configures a systemd user service on an explicitly selected fixed port. `zelyra service status`, `restart`, `change --port <port>`, and `remove` manage its lifecycle with atomic rollback on failure.
- **Canonical project structure and vertical slices (✅ Implemented since 1.2.13/1.2.14):** `--layout standard` enforces a structured layout (`schema/`, `domain/`, `routes/`, static `public/`). `strict_architecture = true` validates architectural boundaries (`E-ARCH-001`), and `zelyra migrate-layout --check` provides deterministic layout migration previews.
- **Extensible CRUD and TableView pages (✅ Implemented since 1.2.15/1.2.16):** Generated create and edit pages provide compiler-checked extension points (`crud.view.create`, `crud.view.edit`), while table views support field help and row actions.
- **Tenant isolation and composite foreign keys (✅ Implemented since 1.2.17/1.2.19):** `table ... scoped by tenant_id: TenantId` enforces multi-tenant boundary on database level using composite foreign keys. `unique within tenant` prevents numbering sequence clashes per tenant.
- **Table invariants and validation rules (✅ Implemented since 1.2.20):** Typed table invariants (`validate`) are evaluated prior to database writes in forms and CRUD.
- **Lifecycle guards and master-detail CRUD with multiple child collections (✅ Implemented since 1.2.21 – 1.2.24):** GoBD-compliant mutation guards for `edit` and `delete` (`guard`), transactional collection forms for multiple 1:N child collections (`has_many`) with server-side aggregates, and API tenant validation from verified sessions.
- **Generated `.env` files:** Supported project templates include commented optional configuration examples alongside active values. Comment lines are documentation only and are never executed as shell scripts or Docker commands. Keep credentials in the protected local `.env`, never in Git or source code.
- **Database development:** Zelyra projects describe schema declaratively with Zelyra tables. `zelyra db plan`, `apply`, and `history` are the primary tools; Laravel migration files are not a runtime requirement. The former experimental Laravel migrator was removed from the Zelyra product.
- **Milestone 1.3.0 outlook:** Dual-tier execution model with Bytecode VM v24, compile-time multi-tenancy (`table ... scoped by TenantId`), GoBD-compliant audit trails (`auditable` with `as_of` time travel), zero-sum ledgers (`ledger`), and single-binary deployment (`zelyra build --bundle`).

### Schedules and outbox in operations

In Zelyra projects, the system idempotently ensures all required outbox and scheduler tables during setup. The resident scheduler starts automatically whenever the project runs under its service (`zelyra service`) or `zelyra serve`.

The following commands are available for inspection and management:

~~~bash
zelyra check main.zyl
zelyra task list main.zyl
zelyra schedule list main.zyl
zelyra schedule jobs main.zyl
zelyra schedule status main.zyl
~~~

A manual host crontab entry (`crontab -e`) is not required for Zelyra operation. If an external monitoring system or third-party server scheduler needs to trigger declared commands, you can generate an exportable preview with:

~~~bash
zelyra schedule export main.zyl --format cron --output deploy/zelyra.cron
~~~

`zelyra schedule export` does not overwrite existing files or modify the host crontab. See the [environment reference](https://github.com/sf1976/zelyra/blob/main/docs/env.en.md) for supported variables, and refer to [Chapter 28](technical-reference.md#28-systemd-services-scheduler-and-jobs) for the complete scheduler and task architecture.

## Status marks

- ✅ **Implemented and verified:** present in the documented scope and supported by the cited sources.
- 🧪 **Experimental:** present, but early or constrained.
- 🗺️ **Planned:** part of the language vision, not yet reliably available.
- ❌ **Currently unavailable:** not present in the current CLI.

## Table of Contents

### Learning Zelyra – The Textbook

- **[PART I – UNDERSTANDING ZELYRA AND PROGRAMMING](textbook-foundations.md#part-i-understanding-zelyra-and-programming)**
  - [Chapter 1: Welcome to Zelyra](textbook-foundations.md#chapter-1-welcome-to-zelyra)
  - [Chapter 2: How a Program Works](textbook-foundations.md#chapter-2-how-a-program-works)
  - [Chapter 3: Installing and Setting Up Zelyra](textbook-foundations.md#chapter-3-installing-and-setting-up-zelyra)
  - [Chapter 4: The First Zelyra Project](textbook-foundations.md#chapter-4-the-first-zelyra-project)
- **[PART II – LANGUAGE FUNDAMENTALS](textbook-foundations.md#part-ii-language-fundamentals)**
  - [Chapter 5: Values and Data Types](textbook-foundations.md#chapter-5-values-and-data-types)
  - [Chapter 6: Variables and Immutability](textbook-foundations.md#chapter-6-variables-and-immutability)
  - [Chapter 7: Operators and Expressions](textbook-foundations.md#chapter-7-operators-and-expressions)
  - [Chapter 8: Input and Output](textbook-foundations.md#chapter-8-input-and-output)
  - [Chapter 9: Decisions with Conditions](textbook-foundations.md#chapter-9-decisions-with-conditions)
  - [Chapter 10: Repetition and Loops](textbook-foundations.md#chapter-10-repetition-and-loops)
- **[PART III – STRUCTURING PROGRAMS](textbook-foundations.md#part-iii-structuring-programs)**
  - [Chapter 11: Functions and Procedures](textbook-foundations.md#chapter-11-functions-and-procedures)
  - [Chapter 12: Contracts and Preconditions (Design by Contract)](textbook-foundations.md#chapter-12-contracts-and-preconditions-design-by-contract)
  - [Chapter 13: Collections, Lists, and Dictionaries (Arrays & Maps)](textbook-foundations.md#chapter-13-collections-lists-and-dictionaries-arrays-maps)
  - [Chapter 14: Creating Custom Data Types (Records & Tables)](textbook-foundations.md#chapter-14-creating-custom-data-types-records-tables)
  - [Chapter 15: Modules and Code Organization](textbook-foundations.md#chapter-15-modules-and-code-organization)
- **[PART IV – SAFETY AND ERROR HANDLING](textbook-foundations.md#part-iv-safety-and-error-handling)**
  - [Chapter 16: Error Types and Their Causes](textbook-foundations.md#chapter-16-error-types-and-their-causes)
  - [Chapter 17: Errors as Values – The Result Pattern](textbook-foundations.md#chapter-17-errors-as-values-the-result-pattern)
  - [Chapter 18: Nothingness Does Not Exist – Working Safely with Option](textbook-foundations.md#chapter-18-nothingness-does-not-exist-working-safely-with-option)
  - [Chapter 19: Testing and Quality Assurance](textbook-foundations.md#chapter-19-testing-and-quality-assurance)
- **[PART V – PRACTICAL DATA PROCESSING](textbook-foundations.md#part-v-practical-data-processing)**
  - [Chapter 20: Working with Files](textbook-foundations.md#chapter-20-working-with-files)
  - [Chapter 21: Date, Time, Randomness, and Structured Data](textbook-foundations.md#chapter-21-date-time-randomness-and-structured-data)
  - [Chapter 22: Concurrency and Background Tasks](textbook-foundations.md#chapter-22-concurrency-and-background-tasks)
- **[PART VI – DATABASES WITH ZELYRA](textbook-applications.md#part-vi-databases-with-zelyra)**
  - [Chapter 23: Why Zelyra Understands Databases Directly](textbook-applications.md#chapter-23-why-zelyra-understands-databases-directly)
  - [Chapter 24: Defining Tables and Data Modeling](textbook-applications.md#chapter-24-defining-tables-and-data-modeling)
  - [Chapter 25: Querying and Modifying Data](textbook-applications.md#chapter-25-querying-and-modifying-data)
- **[PART VII – WEB APPLICATIONS AND FORMS](textbook-applications.md#part-vii-web-applications-and-forms)**
  - [Chapter 26: Rendering Web Pages](textbook-applications.md#chapter-26-rendering-web-pages)
  - [Chapter 27: Forms and User Inputs](textbook-applications.md#chapter-27-forms-and-user-inputs)
  - [Chapter 28: The Complete CRUD Pattern](textbook-applications.md#chapter-28-the-complete-crud-pattern)
  - [Chapter 29: Users, Passwords, and Sessions](textbook-applications.md#chapter-29-users-passwords-and-sessions)
  - [Chapter 30: APIs and Data Exchange](textbook-applications.md#chapter-30-apis-and-data-exchange)
- **[PART VIII – THE DISTINCTIVE FEATURES OF ZELYRA](textbook-applications.md#part-viii-the-distinctive-features-of-zelyra)**
  - [Chapter 31: Readability as the Highest Priority](textbook-applications.md#chapter-31-readability-as-the-highest-priority)
  - [Chapter 32: AI-Nativity – Why Zelyra Is Built for AI Assistants](textbook-applications.md#chapter-32-ai-nativity-why-zelyra-is-built-for-ai-assistants)
  - [Chapter 33: Safety through Capabilities](textbook-applications.md#chapter-33-safety-through-capabilities)
  - [Chapter 34: Zelyra in Comparison](textbook-applications.md#chapter-34-zelyra-in-comparison)
- **[PART IX – FROM DESIGN TO FINISHED APPLICATION](textbook-applications.md#part-ix-from-design-to-finished-application)**
  - [Chapter 35: Planning Software – From Idea to Design](textbook-applications.md#chapter-35-planning-software-from-idea-to-design)
  - [Chapter 36: Architecture and Clean Code Structure](textbook-applications.md#chapter-36-architecture-and-clean-code-structure)
  - [Chapter 37: Configuration and Environment Variables](textbook-applications.md#chapter-37-configuration-and-environment-variables)
  - [Chapter 38: Debugging and Optimization](textbook-applications.md#chapter-38-debugging-and-optimization)
  - [Chapter 39: Deployment and Operations](textbook-applications.md#chapter-39-deployment-and-operations)
- **[PART X – CAPSTONE PROJECT AND LOOKING AHEAD](textbook-applications.md#part-x-capstone-project-and-looking-ahead)**
  - [Chapter 40: The Grand Capstone Project: Complete Task Management](textbook-applications.md#chapter-40-the-grand-capstone-project-complete-task-management)
  - [Chapter 41: Functionality and safe operation](textbook-applications.md#chapter-41-functionality-and-safe-operation)
  - [Chapter 42: Your Journey as a Zelyra Developer](textbook-applications.md#chapter-42-your-journey-as-a-zelyra-developer)
  - [Chapter 43: Working with Zelyra Studio](textbook-applications.md#chapter-43-working-with-zelyra-studio)

### Technical Reference Manual

- [1. What makes Zelyra different](technical-reference.md#1-what-makes-zelyra-different)
- [2. Installation](technical-reference.md#2-installation)
- [3. Your first program](technical-reference.md#3-your-first-program)
- [4. Create a project and use the CLI](technical-reference.md#4-create-a-project-and-use-the-cli)
- [5. Variables, types, and functions](technical-reference.md#5-variables-types-and-functions)
- [6. Option, Result, and pattern matching](technical-reference.md#6-option-result-and-pattern-matching)
- [7. MariaDB and tables](technical-reference.md#7-mariadb-and-tables)
- [8. Inspecting and applying schemas](technical-reference.md#8-inspecting-and-applying-schemas)
- [9. Native SQL](technical-reference.md#9-native-sql)
- [10. Web pages](technical-reference.md#10-web-pages)
- [11. Forms](technical-reference.md#11-forms)
- [12. CRUD](technical-reference.md#12-crud)
- [13. Authentication and permissions](technical-reference.md#13-authentication-and-permissions)
- [14. Capabilities](technical-reference.md#14-capabilities)
- [15. Contracts and verification](technical-reference.md#15-contracts-and-verification)
- [16. Configuration and secrets](technical-reference.md#16-configuration-and-secrets)
- [17. Diagnostics and troubleshooting](technical-reference.md#17-diagnostics-and-troubleshooting)
- [18. Testing and contributing](technical-reference.md#18-testing-and-contributing)
- [19. What comes next](technical-reference.md#19-what-comes-next)
- [20. Zelyra compared with Rust](technical-reference.md#20-zelyra-compared-with-rust)
- [21. Positioning and current development status](technical-reference.md#21-positioning-and-current-development-status)
- [22. Support and technical evidence](technical-reference.md#22-support-and-technical-evidence)
- [23. AI-native development](technical-reference.md#23-ai-native-development)
- [24. Authoritative Sources and Compiler Verification (Source Authority)](technical-reference.md#24-authoritative-sources-and-compiler-verification-source-authority)

- [25. Application mail and SMTP](technical-reference.md#25-application-mail-and-smtp)
- [26. Tenant-scoped notifications](technical-reference.md#26-tenant-scoped-notifications)
- [27. Language, Runtime, and PDF Mail](technical-reference.md#27-language-runtime-and-pdf-mail)
- [28. Systemd Services, Scheduler, and Jobs](technical-reference.md#28-systemd-services-scheduler-and-jobs)
- [29. Canonical Project Layout and Vertical Slices](technical-reference.md#29-canonical-project-layout-and-vertical-slices)
- [30. Extensible CRUD and TableViews](technical-reference.md#30-extensible-crud-and-tableviews)
- [31. Dual-Tier Execution Pipeline: AST, MIR, and Bytecode VM](technical-reference.md#31-dual-tier-execution-pipeline-ast-mir-and-bytecode-vm)
- [32. Tenant Isolation and Composite Foreign Keys](technical-reference.md#32-tenant-isolation-and-composite-foreign-keys)
- [33. Table Invariants and Validation Rules](technical-reference.md#33-table-invariants-and-validation-rules)
- [34. Lifecycle Guards, Policies, and Master-Detail CRUD](technical-reference.md#34-lifecycle-guards-policies-and-master-detail-crud)
- [35. The Zelyra Ecosystem: Packages, Modules, and Settings](technical-reference.md#35-the-zelyra-ecosystem-packages-modules-and-settings)
- [36. CSS Design System and UI Styling for CRUD and TableViews](technical-reference.md#36-css-design-system-and-ui-styling-for-crud-and-tableviews)

### Appendices

- [Appendix A: Quickstart / Cheat Sheet (Syntax Cheat Sheet)](appendices.md#appendix-a-quickstart-cheat-sheet-syntax-cheat-sheet)
- [Appendix B: Complete Zelyra Diagnostic & Error Code Reference](appendices.md#appendix-b-complete-zelyra-diagnostic-error-code-reference)
- [Appendix C: Zelyra CLI Command Reference](appendices.md#appendix-c-zelyra-cli-command-reference)
- [Appendix D: Standard Library Overview](appendices.md#appendix-d-standard-library-overview-zelyra-103)
- [Appendix E: SQL Cheat Sheet for Zelyra Developers](appendices.md#appendix-e-sql-cheat-sheet-for-zelyra-developers)
- [Appendix F: HTML and Web Reference in Zelyra](appendices.md#appendix-f-html-and-web-reference-in-zelyra)
- [Appendix G: Glossary of Technical Terms](appendices.md#appendix-g-glossary-of-technical-terms)
- [Appendix H: Solutions to Chapter Exercises](appendices.md#appendix-h-solutions-to-chapter-exercises)
- [Appendix I: Frequently Asked Questions (FAQ)](appendices.md#appendix-i-frequently-asked-questions-faq)
- [Appendix J: Next Resources and Community](appendices.md#appendix-j-next-resources-and-community)
- [Appendix K: The Zelyra Cookbook (Practical Recipes)](appendices.md#appendix-k-the-zelyra-cookbook-practical-recipes)
