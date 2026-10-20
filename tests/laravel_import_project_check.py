#!/usr/bin/env python3
"""Generate and compiler-check an isolated scaffold from a Laravel app tree."""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
from pathlib import Path


def main() -> int:
    if len(sys.argv) != 2:
        print(
            "usage: python3 tests/laravel_import_project_check.py <laravel-app-directory>",
            file=sys.stderr,
        )
        return 2

    source = Path(sys.argv[1]).resolve(strict=True)
    if not source.is_dir():
        print("error: Laravel source path must be a directory", file=sys.stderr)
        return 2

    repository = Path(__file__).resolve().parents[1]
    binary = Path(os.environ.get("ZELYRA_BIN", repository / "target/debug/zelyra"))
    if not binary.is_file():
        print(f"error: Zelyra binary not found: {binary}", file=sys.stderr)
        return 2

    environment = {
        key: value
        for key, value in os.environ.items()
        if key != "DATABASE_URL" and not key.startswith("ZELYRA_DATABASE_")
    }
    with tempfile.TemporaryDirectory(prefix="zelyra-laravel-project-check-") as temporary:
        output = Path(temporary) / "generated-zelyra-project"
        generate = subprocess.run(
            [
                str(binary),
                "import",
                "laravel",
                "generate",
                str(source),
                "--output",
                str(output),
                "--all-supported",
            ],
            env=environment,
            check=False,
        )
        if generate.returncode:
            return generate.returncode

        check = subprocess.run(
            [str(binary), "check", str(output / "main.zyl")],
            env=environment,
            check=False,
        )
        if check.returncode:
            return check.returncode

        print("Laravel importer scaffold generation and compiler check passed.")
        print("Generated files were temporary and have been removed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
