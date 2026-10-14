# Dates, clock times, and UTC durations

Scope: current stable Zelyra 1.0.4 release.
[Deutsch](dates.de.md)

```zelyra
issued: Date = date("2026-10-12")
match date_add_days_result(issued, 14) {
    Ok(due) => { print(due) }
    Err(reason) => { print(reason) }
}
```

The result is `2026-10-26`. `date_days_between_result(start, end)` returns
`end - start` in calendar days. Negative values and backward arithmetic are
allowed. The proleptic Gregorian calendar supports years 0001 through 9999;
1900/2100 are not leap years, while 2000/2400 are. Month, day, and leap-year
rules are validated. Out-of-range arithmetic returns `Err`; invalid dates
are not normalized to another month.

`Date` uses only `YYYY-MM-DD`; `Time` uses only `HH:MM:SS` between `00:00:00`
and `23:59:59`. The `date`/`time` constructors and JSON decoding validate them.
Leap seconds, time zones, DST, and local wall-clock conversions are not
implemented. Calendar-day arithmetic is separate from UTC duration arithmetic.

## Timestamp and Duration

`Timestamp` is a UTC instant expressed in integer Unix milliseconds.
`Duration` is a signed integer millisecond duration (the `Int` range).
Timestamp ranges from `-62135596800000` (`0001-01-01T00:00:00.000Z`) through
`253402300799999` (`9999-12-31T23:59:59.999Z`). `json_decode<Timestamp>` requires
a JSON integer in that range; out-of-range values now fail in a controlled
way with a source position. The previously accepted entire i64 range is not
retained as a valid ISO calendar range.

`timestamp_add_result(stamp, duration)` and
`timestamp_difference_result(end, start)` return checked `Result` values.
Chronological comparisons use millisecond values. `format_timestamp` renders
UTC with three fractional digits and `Z`, including pre-1970 values.
No computation uses Float.

## Forms, HTTP, and generated clients

Date/Time form inputs are validated against calendar/clock rules independently
of the browser. Table-bound aliases retain validation. Timestamp forms accept
Unix milliseconds or explicit UTC ISO text with `Z`; local times and offset
zones are not inferred. Whole seconds or up to three fractional digits are
accepted. Duration forms require integer milliseconds.

HTTP APIs construct real Date, Time, Timestamp, and Duration runtime values
instead of passing generic strings to handlers. Invalid inputs produce HTTP
400. Date/Time remain JSON strings; Timestamp/Duration remain JSON integers
in milliseconds. Timestamp API inputs may additionally use explicit UTC ISO
text or integer text. OpenAPI/TypeScript now correctly describe existing
output wire formats. The language JSON codec is not silently changed.

## Verification and remaining data paths

`scripts/check_calendar_references.py` independently uses Python `datetime`
to test 1,008 date pairs, 200 UTC-duration pairs, and 200 UTC formatting values
across the full calendar range. See runnable
[calendar_due_date.zyl](../examples/calendar_due_date.zyl).
`tests/temporal-api-e2e.py` tests real HTTP round trips, Date aliases, optional
values, year boundaries, and 13 invalid inputs.

## Native SQL storage

| Zelyra | MariaDB/MySQL | PostgreSQL |
| --- | --- | --- |
| Date | DATE, years 1000–9999 | DATE, years 0001–9999 |
| Time | TIME, whole seconds only | TIME(0), whole seconds only |
| Timestamp | DATETIME(3), UTC convention, years 1000–9999 | TIMESTAMPTZ(3), years 0001–9999 |
| Duration | signed BIGINT milliseconds | signed BIGINT milliseconds |

Date/Time/Timestamp parameters use native driver values. PostgreSQL receives
binary days, microseconds of clock time, or UTC epoch values, with explicit
validation instead of generic strings or Float. PostgreSQL readers reject
infinity, sub-millisecond timestamps, fractional clock seconds, and timestamp
without time zone. MariaDB/MySQL date and timestamp parameters below year 1000
fail before statement execution; this storage limit does not narrow the
language calendar.

Native runtime connections start in UTC, including a PostgreSQL database whose
default timezone differs. PostgreSQL URL startup options are preserved, with
UTC appended. This defines the application connection convention; it does not
provide local-zone/DST conversion. DATETIME itself has no timezone marker, so
external writers must also use UTC.

Before `serve` or the first interpreter SQL operation, a read-only metadata
check verifies declared DATE, whole-second TIME, and millisecond Timestamp
storage. A missing column, wrong SQL kind, or incompatible time precision is
rejected. `serve` reports `E-DB-TEMPORAL` at the declared source column; runtime
execution reports a source-located runtime error. Metadata access is required.
No database is silently created or migrated by this check.

Older TIMESTAMP/TIMESTAMPTZ columns and scales need a reviewed migration. Use
`zelyra db plan`, back up the database, and inspect the generated SQL before
applying it. Converting an old MariaDB TIMESTAMP to DATETIME requires an
explicit decision about the old connection timezone and stored instants.
`Timestamp default now` emits CURRENT_TIMESTAMP(3), matching the column and
retaining milliseconds. Schema inspection retains time precision so a current schema has no false
migration drift. The storage change also affects auth/reset Timestamp tables.

See [calendar_storage.zyl](../examples/calendar_storage.zyl) and the
[SQL package evidence](release-readiness/0.9.9-calendar-sql.en.md).
`tests/temporal-sql-e2e.py` exercises real storage, forms, filters, native function
parameters, NULLs, pre-1970/post-2038 values, milliseconds, and old-schema errors.
It owns a separate temporary database and requires a test account able to
create/drop that database.

## Typed SQL results and compatibility

Raw `sql<T>` output keeps the existing driver text, including MySQL/MariaDB
midnight date-only text and PostgreSQL UTC timestamps with Z. Its JSON/HTTP
representation remains string-valued, including the legacy NULL marker `\N`.
Native NULL is now preserved independently from literal text in result metadata.
Use [`sql_decode<T>`](sql-results.en.md) to materialize checked scalar/record/
array/optional results. The decoded Date/Time/Timestamp values support ordinary
calendar and UTC-duration operations; exact numeric conversion does not use Float.
See [the executable example](../examples/typed_sql_results.zyl) and
[three-backend evidence](release-readiness/0.9.9-typed-sql-results.en.md).
