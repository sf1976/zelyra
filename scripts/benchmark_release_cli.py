#!/usr/bin/env python3
"""Compare release CLI timings for the invoice tutorial on Linux."""

import argparse
import os
from pathlib import Path
from statistics import median
import subprocess
from time import perf_counter_ns


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("candidate", type=Path)
    args = parser.parse_args()

    binaries = [args.baseline.resolve(), args.candidate.resolve()]
    os.sched_setaffinity(0, {min(os.sched_getaffinity(0))})
    root = Path(__file__).resolve().parents[1]
    entry = "examples/invoice_tutorial/main.zyl"
    warmups = 3
    rounds = 21
    tolerance = 20
    failed = False

    for command, options in [("check", []), ("reflect", ["--format=json"])]:
        samples = [[], []]
        for trial in range(warmups + rounds):
            order = (0, 1) if trial % 2 == 0 else (1, 0)
            for index in order:
                start = perf_counter_ns()
                subprocess.run(
                    [str(binaries[index]), command, entry, *options],
                    cwd=root,
                    check=True,
                    stdout=subprocess.DEVNULL,
                )
                elapsed_ms = (perf_counter_ns() - start) / 1_000_000
                if trial >= warmups:
                    samples[index].append(elapsed_ms)
        baseline, candidate = (median(values) for values in samples)
        change = 100 * (candidate / baseline - 1)
        failed |= change > tolerance
        print(
            f"{command}: baseline {baseline:.2f} ms, candidate {candidate:.2f} ms, "
            f"delta {change:+.1f}%, tolerance +{tolerance}%"
        )
        print("sample ranges:", ", ".join(f"{min(s):.2f}–{max(s):.2f} ms" for s in samples))
    return int(failed)


if __name__ == "__main__":
    raise SystemExit(main())
