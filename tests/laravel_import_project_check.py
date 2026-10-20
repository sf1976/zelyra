#!/usr/bin/env python3
"""Generate and compiler-check an isolated scaffold from a Laravel app tree."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


EXCLUDED_DIRECTORIES = {".git", "vendor", "node_modules", "storage"}


def php_source_files(root: Path) -> set[str]:
    """Mirror the importer's documented PHP scan boundary without following symlinks."""
    files: set[str] = set()
    pending = [root]
    while pending:
        directory = pending.pop()
        for entry in directory.iterdir():
            relative = entry.relative_to(root)
            parts = relative.parts
            if entry.is_symlink():
                continue
            if any(part == ".env" or part.startswith(".env.") for part in parts):
                continue
            if any(part in EXCLUDED_DIRECTORIES for part in parts):
                continue
            if parts[0] == "bootstrap" and parts[1:2] == ("cache",):
                continue
            if entry.is_dir():
                pending.append(entry)
            elif entry.is_file() and entry.suffix.lower() == ".php":
                files.add(relative.as_posix())
    return files


def check_source_file_inventory(source: Path, output: Path) -> bool:
    plan_path = output / "SOURCE_FILE_PLAN.json"
    markdown_path = output / "MIGRATION_FILES.md"
    try:
        plan = json.loads(plan_path.read_text(encoding="utf-8"))
        markdown = markdown_path.read_text(encoding="utf-8")
    except (OSError, json.JSONDecodeError) as error:
        print(f"error: generated source-file inventory cannot be read: {error}", file=sys.stderr)
        return False

    if not isinstance(plan, list):
        print("error: SOURCE_FILE_PLAN.json must contain an array", file=sys.stderr)
        return False
    entries: dict[str, dict[str, object]] = {}
    for entry in plan:
        if not isinstance(entry, dict) or not isinstance(entry.get("file"), str):
            print(
                "error: source-file plan contains an entry without a file path",
                file=sys.stderr,
            )
            return False
        file = entry["file"]
        if file in entries:
            print(f"error: source-file plan lists {file} more than once", file=sys.stderr)
            return False
        if (
            not entry.get("category")
            or not entry.get("translation_status")
            or not entry.get("proposed_action")
        ):
            print(
                f"error: source-file plan has no disposition metadata for {file}",
                file=sys.stderr,
            )
            return False
        entries[file] = entry

    expected = php_source_files(source)
    actual = set(entries)
    if missing := sorted(expected - actual):
        print(
            "error: PHP source files are missing from SOURCE_FILE_PLAN.json: "
            + ", ".join(missing),
            file=sys.stderr,
        )
        return False
    if extra := sorted(actual - expected):
        print(
            "error: source-file plan contains paths outside the scanned PHP source set: "
            + ", ".join(extra),
            file=sys.stderr,
        )
        return False

    missing_markdown = sorted(file for file in expected if f"`{file}`" not in markdown)
    if missing_markdown:
        print(
            "error: PHP source files are missing from MIGRATION_FILES.md: "
            + ", ".join(missing_markdown),
            file=sys.stderr,
        )
        return False
    rows = [line for line in markdown.splitlines() if line.startswith("| `")]
    if len(rows) != len(expected):
        print(
            f"error: MIGRATION_FILES.md has {len(rows)} file rows for {len(expected)} scanned PHP files",
            file=sys.stderr,
        )
        return False
    print(f"Source-file inventory accounts for all {len(expected)} scanned PHP files.")
    return True


def check_import_coverage(output: Path) -> bool:
    try:
        coverage = json.loads((output / "IMPORT_COVERAGE.json").read_text(encoding="utf-8"))
        file_plan = json.loads((output / "SOURCE_FILE_PLAN.json").read_text(encoding="utf-8"))
        generation = json.loads((output / "GENERATION_SELECTION.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"error: generated coverage report cannot be read: {error}", file=sys.stderr)
        return False

    scope = coverage.get("scope", {})
    findings = coverage.get("findings", {})
    generated = coverage.get("generated_artifacts", {})
    behavior = coverage.get("behavior_verification", {})
    safety = coverage.get("safety", {})
    expected = {
        "schema_version": "1",
        "source_files_scanned": len(file_plan),
        "full_behavior_coverage_claimed": False,
        "schema_candidates": len(generation.get("included_tables", [])),
        "compiler_check": "passed",
        "executed_by_scaffold_generation": False,
        "generated_workflows_are_behavior_tested": False,
        "php_executed": False,
        "database_changes_applied": False,
    }
    actual = {
        "schema_version": coverage.get("schema_version"),
        "source_files_scanned": scope.get("source_files_scanned"),
        "full_behavior_coverage_claimed": scope.get("full_behavior_coverage_claimed"),
        "schema_candidates": generated.get("schema_candidates"),
        "compiler_check": generated.get("compiler_check"),
        "executed_by_scaffold_generation": behavior.get("executed_by_scaffold_generation"),
        "generated_workflows_are_behavior_tested": behavior.get("generated_workflows_are_behavior_tested"),
        "php_executed": safety.get("php_executed"),
        "database_changes_applied": safety.get("database_changes_applied"),
    }
    if actual != expected:
        print(f"error: importer coverage report contradicts generated artifacts: {actual!r}", file=sys.stderr)
        return False
    if findings.get("total") != findings.get("unresolved_or_manual", 0) + findings.get("partial", 0) + findings.get("by_status", {}).get("direct", 0):
        print("error: importer coverage finding totals are inconsistent", file=sys.stderr)
        return False
    print("Importer coverage report matches the source inventory and generated selection.")
    return True


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

        if not check_source_file_inventory(source, output):
            return 1
        if not check_import_coverage(output):
            return 1

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
        }
        missing_routes = sorted(expected_routes - route_keys)
        if missing_routes:
            print(
                "error: generated project is missing imported routes: "
                + ", ".join(f"{method} {path} ({kind})" for method, path, kind in missing_routes),
                file=sys.stderr,
            )
            return 1

        schema_source = (output / "src/imported_schema.zyl").read_text(encoding="utf-8")
        if "table csv_mailer_suppressions scoped by tenant_id: TenantId" not in schema_source:
            print("error: imported tenant suppression data is missing its declared tenant scope", file=sys.stderr)
            return 1
        if (output / "src/imported_public_unsubscribe.zyl").exists():
            print("error: public unsubscribe SQL must not bypass tenant isolation", file=sys.stderr)
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

        bundle_output = Path(temporary) / "imported-identity-bundle"
        bundle = subprocess.run(
            [
                str(binary),
                "module",
                "bundle",
                str(output / "main.zyl"),
                "pkg:imported_identity",
                "--output",
                str(bundle_output),
            ],
            env=environment,
            check=False,
            capture_output=True,
            text=True,
        )
        if bundle.returncode:
            print(bundle.stdout, file=sys.stderr, end="")
            print(bundle.stderr, file=sys.stderr, end="")
            return bundle.returncode
        bundled_entry = bundle_output / "main.zyl"
        bundled_identity = bundle_output / "packages/imported_identity/src/lib.zyl"
        if not bundled_entry.is_file() or not bundled_identity.is_file():
            print(
                "error: module bundle is missing the generated entry or reusable identity package",
                file=sys.stderr,
            )
            return 1

        bundle_check = subprocess.run(
            [str(binary), "check", str(bundled_entry)],
            env=environment,
            check=False,
            capture_output=True,
            text=True,
        )
        if bundle_check.returncode:
            print(bundle_check.stdout, file=sys.stderr, end="")
            print(bundle_check.stderr, file=sys.stderr, end="")
            return bundle_check.returncode

        bundle_lock = subprocess.run(
            [str(binary), "package", "lock", str(bundled_entry), "--check"],
            env=environment,
            check=False,
            capture_output=True,
            text=True,
        )
        if bundle_lock.returncode:
            print(bundle_lock.stdout, file=sys.stderr, end="")
            print(bundle_lock.stderr, file=sys.stderr, end="")
            return bundle_lock.returncode

        print("Laravel importer source-file coverage, scaffold, compiler, route graph, locked identity package, module plan, and offline bundle checks passed.")
        print("Generated files were temporary and have been removed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
