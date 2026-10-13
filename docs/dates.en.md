# Dates, clock times, and UTC durations

Snapshot: unreleased 0.9.9 development branch.
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

This verifies calendar/UTC arithmetic and the listed input/HTTP paths.
Database storage, PostgreSQL temporal codecs, Timestamp scale, and typed SQL
result consumption are separate remaining technical 0.9.9 gates. The entire
date path is considered verified only after these pass. A SQL type annotation
alone currently does not demonstrate an automatically decoded date variable.
