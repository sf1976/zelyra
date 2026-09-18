# Zelyra positioning and unique strengths

[Deutsch](positioning.de.md) · English

Zelyra is built for the part of software work that is often repetitive but
still deserves strong correctness guarantees: database-backed business
applications.

## What makes Zelyra distinct

### 1. One source of truth from schema to application

A table declaration can drive database DDL, domain types, SQL checks, form
validation, API shapes, and CRUD views. The goal is to describe a business
fact once and reuse it everywhere.

### 2. SQL stays first-class and becomes checkable

Zelyra does not hide complex SQL behind a mandatory ORM. Native SQL remains
recognizable SQL, while the compiler checks tables, columns, parameters,
nullability, and result mapping against the declared schema.

### 3. Business web features are language primitives

Forms, CRUD, search, filters, pagination, authentication, permissions, and
typed APIs are part of one coherent model. They are not a collection of
unrelated framework adapters that each need their own types and conventions.

### 4. Secure defaults are visible and enforceable

HTML escaping, parameterized SQL, CSRF protection, null safety, authorization,
and capability checks are enabled by design. Sensitive escape hatches remain
explicit, so convenience does not silently become authority.

### 5. Proof claims are honest

`PROVEN`, `RUNTIME_CHECK`, `UNPROVEN`, and `FAILED` have different meanings.
Zelyra must not label a runtime assertion as a mathematical proof. This makes
verification results useful in engineering decisions and CI.

### 6. Short path for the simple case, full language for the hard case

`crud Customer -> customers` is intentionally compact. Developers can still
write ordinary functions, native SQL, custom forms, views, actions, APIs, and
business logic when generated behavior is not enough. Zelyra is not a sealed
low-code platform and does not require an ORM.

### 7. Deployment should not require a framework stack

The built-in server, MariaDB-first workflow, Docker Compose template, explicit
port selection, and source/release installation are designed to make the first
working application approachable. Production deployment remains an evolving
part of the project.

### 8. Safe adaptive views instead of a generated dead end

Zelyra views are intended to start with useful defaults and remain locally
customizable. Named layouts and components can provide fallback content, while
the compiler keeps composition explicit and the generated data, escaping,
authentication, and authorization paths intact. This is a product direction,
not a claim that any single view feature is unique by itself; its value comes
from combining simplicity with the same database and safety model.

## What is implemented today

The current repository already demonstrates schema-aware SQL, MariaDB CRUD,
forms, typed APIs, authentication, direct and role-based permissions, CSRF
protection, capabilities, contracts, and an initial verification command.
These are implemented features, not promises about the complete language.

## What remains a goal

Zelyra is currently bootstrapped in Rust. Rust is an implementation detail for
the compiler and runtime, not a prerequisite for using released Zelyra
applications. The long-term goal is a self-hosting path: first keep the
bootstrap compiler stable, then move suitable compiler layers into Zelyra and
compile them with an existing trusted bootstrap.

User administration, broader verification, production packaging, and the
self-hosting compiler are not yet complete. The project must keep these items
visibly separate from the implemented core.

## Product test

The strongest practical test is still a real machine-management application:
define departments and machines once, apply the MariaDB schema, and obtain a
secure CRUD application with relationships, filters, permissions, and custom
business logic. If a feature does not improve that path without sacrificing
control, safety, or extensibility, it should not enter the language core.
