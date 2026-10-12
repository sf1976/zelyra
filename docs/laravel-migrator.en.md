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

## Initial recognition scope

The early analyzer recognizes Laravel version constraints in `composer.json`,
simple `Route` declarations, Eloquent models, controller classes,
`Schema::create`, selected relationships, validation entry points,
`ShouldQueue` jobs, Blade views, and calls to selected mail, HTTP, file, and
transaction APIs. Each finding includes its file, line, status, description,
and next review step. JSON is intended for tools; Markdown is directly
readable.

Analyzer and mapping ruleset versions are separate. Zelyra 0.9.8 reports
analyzer version `0.9.8` and mapping ruleset `0.9.6`. The ruleset adds explicit
review findings for common authorization calls and policy classes, tenant-like
identifiers, money and aggregate calculations, and file upload/download
operations. The 0.9.8 fixtures cover routes, models, controllers, migrations,
selected relationships, validation, jobs, authorization clues, tenant-like
fields, money calculations, files, integrations, transactions, and Blade
views. These findings describe evidence for review; they do not determine
whether a control is complete or correct.

## Limits

The scanner is pattern- and line-based, not a PHP parser. It does not yet link a
complete graph of route, middleware, controller, model, migration, and view.
Dynamic routes, conditional validation, policy semantics, nested business logic,
and implicit package behavior can be missed. Recognized syntax does not prove
functional or security parity. A pattern that was not recognized does not mean
the feature is absent. Manually review tenant boundaries, permissions,
transactions, money, file storage, and external side effects.

Analysis leaves the Laravel directory unchanged. An explicitly requested
scaffold is separate and compiler-checkable, but contains no copied business
logic, credentials, or application data. The scanner reports individual
findings, not a complete route-to-data graph, and does not claim any Laravel
behavior is directly supported. Source-graph linking and runnable behavior
mappings remain future work.
