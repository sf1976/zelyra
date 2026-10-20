#!/usr/bin/env python3
"""Generate and compiler-check an isolated scaffold from a Laravel app tree."""

from __future__ import annotations

import json
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

        routes = subprocess.run(
            [str(binary), "routes", str(output / "main.zyl"), "--format", "json"],
            env=environment,
            check=False,
            capture_output=True,
            text=True,
        )
        if routes.returncode:
            print(routes.stderr, file=sys.stderr, end="")
            return routes.returncode
        try:
            route_entries = json.loads(routes.stdout)["routes"]
        except (json.JSONDecodeError, KeyError, TypeError) as error:
            print(f"error: generated route inventory is invalid JSON: {error}", file=sys.stderr)
            return 1

        route_keys = {
            (entry.get("method"), entry.get("path"), entry.get("kind"))
            for entry in route_entries
            if isinstance(entry, dict)
        }
        expected_routes = {
            ("GET", "/login", "authentication"),
            ("POST", "/login", "authentication"),
            ("POST", "/logout", "authentication"),
            ("GET", "/csv-mailer/abmelden/{token}", "form"),
            ("POST", "/csv-mailer/abmelden/{token}", "form"),
        }
        missing_routes = sorted(expected_routes - route_keys)
        if missing_routes:
            print(
                "error: generated project is missing imported routes: "
                + ", ".join(f"{method} {path} ({kind})" for method, path, kind in missing_routes),
                file=sys.stderr,
            )
            return 1

        identity_entry = output / "packages/imported_identity/src/lib.zyl"
        identity_lock = output / "zelyra.lock"
        if not identity_entry.is_file() or not identity_lock.is_file():
            print(
                "error: generated project is missing its locked reusable native identity package",
                file=sys.stderr,
            )
            return 1

        print("Laravel importer scaffold, compiler, route graph, and identity package checks passed.")
        print("Generated files were temporary and have been removed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
