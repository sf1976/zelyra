#!/usr/bin/env python3
"""Independent Gregorian date and UTC-millisecond checks using Python datetime."""
import argparse
from datetime import date, datetime, timedelta
from pathlib import Path
import random
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", type=Path, required=True)
    args = parser.parse_args()
    compiler = args.compiler.resolve()
    rng = random.Random(991012)
    source = ['fn date_case(start: Date, days: Int, expected: Date) {',
              '    match date_add_days_result(start, days) {',
              '        Ok(actual) => { assert(actual == expected, "calendar addition reference") }',
              '        Err(reason) => { print(reason) assert(false) }', '    }',
              '    match date_days_between_result(start, expected) {',
              '        Ok(actual) => { assert(actual == days, "calendar difference reference") }',
              '        Err(reason) => { print(reason) assert(false) }', '    }', '}',
              'fn utc_case(start: Timestamp, milliseconds: Int, expected: Timestamp) {',
              '    match timestamp_add_result(start, duration_ms(milliseconds)) {',
              '        Ok(actual) => { assert(actual == expected, "UTC addition reference") }',
              '        Err(reason) => { print(reason) assert(false) }', '    }',
              '    match timestamp_difference_result(expected, start) {',
              '        Ok(actual) => { assert(actual == duration_ms(milliseconds), "UTC difference reference") }',
              '        Err(reason) => { print(reason) assert(false) }', '    }', '}', 'fn main() {']
    fixed = [("0001-01-01", "9999-12-31"), ("9999-12-31", "0001-01-01"),
             ("1900-02-28", "1900-03-01"), ("2000-02-28", "2000-02-29"),
             ("2100-02-28", "2100-03-01"), ("2400-02-28", "2400-02-29"),
             ("2026-12-31", "2027-01-01"), ("0001-01-01", "0001-01-01")]
    pairs = [(date.fromisoformat(first), date.fromisoformat(second)) for first, second in fixed]
    for _ in range(1000):
        pairs.append((date.fromordinal(rng.randint(1, date.max.toordinal())),
                      date.fromordinal(rng.randint(1, date.max.toordinal()))))
    for first, second in pairs:
        source.append(f'    date_case(date("{first.isoformat()}"), {(second-first).days}, date("{second.isoformat()}"))')
    epoch = datetime(1970, 1, 1)
    for _ in range(200):
        first_date = date.fromordinal(rng.randint(1, date.max.toordinal()))
        second_date = date.fromordinal(rng.randint(1, date.max.toordinal()))
        first = datetime.combine(first_date, datetime.min.time()) + timedelta(milliseconds=rng.randint(0, 86_399_999))
        second = datetime.combine(second_date, datetime.min.time()) + timedelta(milliseconds=rng.randint(0, 86_399_999))
        def millis(value):
            delta = value - epoch
            return delta.days * 86_400_000 + delta.seconds * 1000 + delta.microseconds // 1000
        start, end = millis(first), millis(second)
        source.append(f'    utc_case(json_decode<Timestamp>("{start}"), {end-start}, json_decode<Timestamp>("{end}"))')
        # UTC formatting is checked independently, including pre-epoch values.
        expected = f"{first.year:04}-{first.month:02}-{first.day:02}T{first.hour:02}:{first.minute:02}:{first.second:02}.{first.microsecond//1000:03}Z"
        source.append(f'    assert(format_timestamp(json_decode<Timestamp>("{start}")) == "{expected}", "UTC formatting reference")')
    source += ['    print("PASS")', '}']
    with tempfile.TemporaryDirectory(prefix="zelyra-calendar-reference-") as directory:
        entry = Path(directory)/"main.zyl"
        entry.write_text("\n".join(source)+"\n", encoding="utf-8")
        for command in ["check", "run"]:
            result = subprocess.run([str(compiler), command, str(entry)], capture_output=True, text=True, timeout=30)
            if result.returncode or command == "run" and result.stdout.strip() != "PASS":
                print(result.stdout + result.stderr)
                return result.returncode or 1
    print("PASS: 1008 Gregorian date pairs, 200 UTC-duration pairs, 200 UTC format checks")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
