#!/usr/bin/env python3
"""Compare Zelyra's integer and Decimal arithmetic with Python references."""

from __future__ import annotations

import argparse
import random
import subprocess
import tempfile
from pathlib import Path


def decimal_text(micros: int) -> str:
    sign = "-" if micros < 0 else ""
    whole, fractional = divmod(abs(micros), 1_000_000)
    return f"{sign}{whole}.{fractional:06d}"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", type=Path, required=True)
    args = parser.parse_args()
    compiler = args.compiler.resolve()
    if not compiler.is_file():
        parser.error(f"compiler does not exist: {compiler}")

    randomizer = random.Random(990912)
    source = ["fn main() {"]
    for _ in range(300):
        left = randomizer.randint(-(2**50), 2**50)
        right = randomizer.randint(-(2**31), 2**31) or 1
        quotient = abs(left) // abs(right)
        if (left < 0) != (right < 0):
            quotient = -quotient
        remainder = left - quotient * right
        source.append(
            f'    assert(({left}) / ({right}) == {quotient}, "integer division reference")'
        )
        source.append(
            f'    assert(({left}) % ({right}) == {remainder}, "integer remainder reference")'
        )

    for _ in range(100):
        left = randomizer.randint(-10**12, 10**12)
        right = randomizer.randint(-10**12, 10**12)
        total = left + right
        source.append(
            "    assert("
            f'decimal("{decimal_text(left)}") + '
            f'decimal("{decimal_text(right)}") == '
            f'decimal("{decimal_text(total)}"), "decimal addition reference")'
        )

    source.extend(['    print("PASS")', "}"])
    with tempfile.TemporaryDirectory(prefix="zelyra-numeric-reference-") as directory:
        program = Path(directory) / "main.zyl"
        program.write_text("\n".join(source) + "\n", encoding="utf-8")
        checked = subprocess.run(
            [str(compiler), "check", str(program)],
            capture_output=True,
            text=True,
            timeout=20,
            check=False,
        )
        if checked.returncode != 0:
            print(checked.stdout, end="")
            print(checked.stderr, end="")
            return checked.returncode or 1
        executed = subprocess.run(
            [str(compiler), "run", str(program)],
            capture_output=True,
            text=True,
            timeout=20,
            check=False,
        )
        if executed.returncode != 0 or executed.stdout.strip() != "PASS":
            print(executed.stdout, end="")
            print(executed.stderr, end="")
            return executed.returncode or 1

    print("PASS: 300 integer division/remainder pairs and 100 Decimal additions")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
