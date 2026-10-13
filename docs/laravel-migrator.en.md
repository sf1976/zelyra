# Laravel Migrator

The Laravel Migrator starts with a static analysis report. It does not change
the source project, its database, or its environment.

```sh
zelyra migrate laravel ./legacy-project --report ./reports/migration.md
zelyra migrate laravel ./legacy-project --report ./reports/migration.json --format json
```

The report path must be outside the source project, its parent directory must
already exist, and the target file must not exist. Zelyra does not run PHP or
Artisan, open a database connection, or read `.env` files. Symlinks, `vendor`,
`node_modules`, `storage`, and `bootstrap/cache` are skipped. Analysis is
bounded to 10,000 PHP files and 2 MiB per file.

## Recognition Scope and Ruleset

The analysis recognizes Laravel version constraints in `composer.json` (Laravel 10.x and 11.x),
relevant package dependencies (`spatie/*`, `stripe/stripe-php`, `inertiajs/*`, `livewire/*`),
simple `Route` declarations, Eloquent models, controller classes, `Schema::create`,
relationships (`belongsTo`, `hasMany`, `hasOne`, etc.), validation entry points,
`ShouldQueue` jobs, authorization checks (`authorize`, `Gate`, policies),
tenant boundaries (`tenant_id`, `account_id`, etc.), money and aggregate calculations,
file operations (`store`, `download`), payment and billing calls (`charge`, `checkout`, `subscription`),
syntactic and dynamic ambiguity (`DB::raw`, `whereRaw`, `call_user_func`),
direct environment variable access (`env(...)`), integration calls (Mail, Notification, HTTP, Storage),
database transactions (`DB::transaction`), and Blade views.
Each finding includes file, line, status, description, and next review step. JSON is intended
for downstream tools; Markdown is directly readable.

Analyzer and mapping ruleset versions are aligned at `0.9.9` in Zelyra 0.9.9.
The 18 finding categories document review clues; they do not determine whether a control
is complete or correct. No finding is claimed as fully supported (`supported`); all results
are honestly reported as `manual` or `partial`.

## Security and Output Boundaries

1. **Read-Only Operation (Immutability):** The source project, its files, and its environment
   remain strictly unmodified.
2. **Path Isolation:** Report files and scaffold target directories must be outside the
   source project; directory traversals (`..`) and pre-existing target directories or files
   are safely rejected to prevent accidental overwrites.
3. **Privacy & Secrets Redaction:** `.env` files, `APP_KEY`, passwords, and tokens are never
   read or copied into reports or scaffolds. Direct `env(...)` calls capture only the variable
   name without reading the environment value.
4. **Separate Scaffold:** An explicitly requested scaffold generated via `--output` produces
   an independent, `zelyra check`-validated project (`zelyra.toml`, `main.zyl`, `README.md`,
   `MIGRATION_REPORT.md`, `MIGRATION_TODO.md`), but contains no transferred business logic.

## Limits and Accepted Mappings

The scanner is pattern- and line-based, not a PHP parser. It does not link a complete
graph of route, middleware, controller, model, migration, and view. Recognized syntax does
not prove functional or security parity.

Accepted mappings (models/migrations with foreign keys and `Decimal`, relational CRUD
routes with tenant scoping, CSRF, validation, relational joins, and restrictive deletion)
are proven as runnable projects on MariaDB 11.4.13, MySQL 8.4.11, and PostgreSQL 16.15
(`tests/laravel-migrator-business-e2e.py`).
Furthermore, encapsulation of modular business applications with dedicated database
configuration and Docker export via `zelyra module bundle` is proven end-to-end.
