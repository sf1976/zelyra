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

        workflow = output / "src/imported_public_unsubscribe.zyl"
        layout = output / "src/imported_layouts.zyl"
        if workflow.is_file():
            workflow_source = workflow.read_text(encoding="utf-8")
            if not layout.is_file() or 'import "src/imported_layouts.zyl" as imported_layouts' not in workflow_source:
                print("error: generated public workflow does not depend on the detected shared layout", file=sys.stderr)
                return 1
            if "view: ImportedAppShell" not in workflow_source:
                print("error: generated public success page does not use ImportedAppShell", file=sys.stderr)
                return 1

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

        package_lock = subprocess.run(
            [str(binary), "package", "lock", str(output / "main.zyl"), "--check"],
            env=environment,
            check=False,
            capture_output=True,
            text=True,
        )
        if package_lock.returncode:
            print(package_lock.stdout, file=sys.stderr, end="")
            print(package_lock.stderr, file=sys.stderr, end="")
            return package_lock.returncode

        module_plan = subprocess.run(
            [
                str(binary),
                "module",
                "plan",
                str(output / "main.zyl"),
                "pkg:imported_identity",
            ],
            env=environment,
            check=False,
            capture_output=True,
            text=True,
        )
        if module_plan.returncode:
            print(module_plan.stdout, file=sys.stderr, end="")
            print(module_plan.stderr, file=sys.stderr, end="")
            return module_plan.returncode
        try:
            plan_document = json.loads(module_plan.stdout)
            identity_plan = plan_document["plan"]
            planned_declarations = identity_plan["declaration_closure"]["declarations"]
        except (json.JSONDecodeError, KeyError, TypeError) as error:
            print(f"error: generated identity package plan is invalid JSON: {error}", file=sys.stderr)
            return 1
        if (
            plan_document.get("success") is not True
            or identity_plan.get("selection_kind") != "package"
            or identity_plan.get("selected_package") != "packages/imported_identity/src/lib.zyl"
            or "packages/imported_identity/src/lib.zyl" not in identity_plan.get("source_files", [])
            or "packages/imported_identity/src/lib.zyl::auth:users" not in planned_declarations
        ):
            print(
                "error: generated identity package is not represented as a planned reusable module",
                file=sys.stderr,
            )
            return 1

        print("Laravel importer scaffold, compiler, route graph, locked identity package, and module-plan checks passed.")
        print("Generated files were temporary and have been removed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
