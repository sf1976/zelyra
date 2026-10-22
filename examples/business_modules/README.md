# Business modules with a database provider

This offline example composes an application, a billing package, and a
separate storage package. The storage package owns the database declaration
and `invoices` table and exposes one typed query operation. The application
enables the Database capability; package `effects` ceilings only constrain
declared effects and do not grant runtime access.

Each package manifest declares `zelyra = ">=1.1.0, <2.0.0"`. The CLI checks
this compatibility range before using the package graph, and the requirement
is covered by the package manifest checksum and lockfile.

The root entry calls `billing::invoice_count()`. The storage package uses
`sql_decode<Invoice[]>` to materialize table-derived typed records before
calling `len`; plain `sql<T>` keeps its raw-row representation. With an
isolated MariaDB database
configured as `business`, bootstrap the schema and run the example:

```sh
DATABASE_URL=mariadb://USER:PASSWORD@127.0.0.1:3306/business zelyra db bootstrap main.zyl
DATABASE_URL=mariadb://USER:PASSWORD@127.0.0.1:3306/business zelyra run main.zyl
```

The example prints `0` for an empty `invoices` table. Use disposable local
credentials; the bootstrap command creates schema objects.

The database-backed `billing → storage → Invoice[]` module call was run against an
isolated MariaDB 11.4 instance with default AST, `--backend mir`, and
`--backend bytecode`. Each backend printed `0`. These experimental backends
remain partial; this verifies this module path, not complete language or
effect parity.

The billing package also exposes `calculate_line_amounts`, returning a
`LineAmounts` record with typed net, tax, and gross Decimal amounts for one
priced business line. The same API works for quote, order, and invoice lines.
Callers pass a fractional tax
rate, currency scale, and explicit rounding mode; the package does not assume a
country's tax rules, currency, invoice numbering, or posting workflow. It
quantizes net first, then tax on the rounded net, then gross as rounded net
plus rounded tax. Decimal arithmetic currently has at most nine fractional
digits before quantization, so applications that require different precision
or line-versus-document rounding rules must choose and test those rules
explicitly. `billing_calculation_demo` checks a 3 × 0.05 line at 19% using
`half_even`, then checks both an invoice-line and a quote-line call against the
same package API. It also confirms an invalid rounding mode returns an error
on AST, MIR, and Bytecode. This is a reusable calculation primitive, not a
complete invoicing or accounting system.

The `csv_export` package is a reusable, local package in this example. Its
`encode(rows)` function returns UTF-8 CSV text with quoted delimiters and
embedded line breaks. `write_csv(path, rows)` writes through Zelyra's guarded
filesystem API and declares the `FileSystem` effect; the application must
explicitly grant that capability and configure an allowed `write_roots` path.
The encoder rejects output larger than 16 MiB. It builds the bounded document
in memory; this is not streaming, an ODBC importer, or an Excel workbook
exporter.

The separate pure `csv_import` package decodes a bounded delimited document,
requires a header row with non-empty column names, and rejects data rows with a
different number of fields.
`preview_table(rows, max_rows)` returns at most 100 data rows plus the header.
`map_columns(rows, source_columns, target_columns)` reorders and renames
explicitly selected columns, rejecting missing or ambiguous headers and
duplicate target names. `convert_columns(rows, mappings)` also applies
explicit, compiler-visible conversions to text, integer, Boolean, and Decimal
cells and returns a table of tagged `CsvValue` cells. Invalid values identify
their CSV row and target column. Integer parsing uses `parse_int`; Boolean
parsing accepts `true`, `false`, `1`, and `0`; Decimal parsing uses
`parse_decimal` and preserves exact base-10 values with up to nine fractional
digits. Date and time parsing use the canonical, locale-independent forms
`YYYY-MM-DD` and `HH:MM:SS`, validate Gregorian dates and 24-hour clock ranges,
and return tagged `Date` and `Time` values. Conversion is never inferred from
a database schema. Database writes and transactional apply remain open. The
`csv_import_demo` and `csv_convert_demo` entries exercise valid and invalid
inputs on AST, MIR, and Bytecode. They are not a preview UI or a transactional
database importer.

To run the CSV demonstration without connecting to MariaDB:

```sh
cd examples/business_modules
zelyra package lock main.zyl
zelyra check main.zyl
zelyra run main.zyl --entry csv_demo
zelyra run main.zyl --entry csv_demo --backend mir
zelyra run main.zyl --entry csv_demo --backend bytecode
zelyra run main.zyl --entry csv_import_demo --backend ast
zelyra run main.zyl --entry csv_import_demo --backend mir
zelyra run main.zyl --entry csv_import_demo --backend bytecode
zelyra run main.zyl --entry csv_convert_demo
zelyra run main.zyl --entry csv_convert_demo --backend mir
zelyra run main.zyl --entry csv_convert_demo --backend bytecode
zelyra run main.zyl --entry csv_temporal_demo --backend ast
zelyra run main.zyl --entry csv_temporal_demo --backend mir
zelyra run main.zyl --entry csv_temporal_demo --backend bytecode
zelyra run main.zyl --entry billing_calculation_demo --backend ast
zelyra run main.zyl --entry billing_calculation_demo --backend mir
zelyra run main.zyl --entry billing_calculation_demo --backend bytecode
```

The checked-in `zelyra.toml` explicitly enables the filesystem capability for
the example. `csv_encode`, `csv_decode_result`, `parse_int`, and `parse_bool`
use the same pure runtime implementation on AST, MIR, and bytecode. The CLI
integration tests check CSV export, valid and malformed parsing, explicit typed
conversion, and exact output across all three backends. A consuming application
must also configure its allowed roots before calling `write_csv`; the package
effect declaration alone never grants access.

Validate the dependency graph and build a standalone Docker app from the full
web application. The app imports the reusable billing and storage packages;
bundling only `pkg:billing` would produce a library without an HTTP route.

```sh
cd examples/business_modules
zelyra package lock main.zyl
zelyra check main.zyl
zelyra module bundle main.zyl main.zyl \
  --output /tmp/zelyra-billing-bundle \
  --docker --compiler-ref <40-character-commit>
```

Use the full Git commit of a Zelyra revision that already supports `pkg:`
imports and the syntax in this project. The generated Docker build compiles
that pinned compiler and checks `main.zyl` with it before creating the runtime
image; an incompatible compiler commit fails the image build.

The bundle includes the transitive storage provider, a lockfile, Docker files,
and a `.env.example` with `ZELYRA_DATABASE_MAIN_URL`. `zelyra check` and bundle
generation do not connect to MariaDB. Running the generated service requires a
reachable database with the declared schema; the bundle does not create or
seed one. The generated bundle is a checked source package, not proof of a
complete production deployment.
