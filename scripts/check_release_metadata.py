#!/usr/bin/env python3
"""Validate the version and release metadata used by Zelyra artifacts."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tomllib
from pathlib import Path

WORKSPACE_PACKAGES = (
    "zelyra-ast",
    "zelyra-cli",
    "zelyra-database",
    "zelyra-forms",
    "zelyra-hir",
    "zelyra-lexer",
    "zelyra-parser",
    "zelyra-runtime",
    "zelyra-web",
)
SEMVER = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+$")
TAG = re.compile(r"^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$")


def workspace_version(root: Path) -> str:
    manifest = (root / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(
        r"(?ms)^\[workspace\.package\]\s*(.*?)(?:^\[|\Z)", manifest
    )
    if not match:
        raise ValueError("Cargo.toml has no [workspace.package] section")
    version_match = re.search(r'^version\s*=\s*"([^"]+)"\s*$', match.group(1), re.M)
    if not version_match:
        raise ValueError("[workspace.package] has no version")
    version = version_match.group(1)
    if not SEMVER.fullmatch(version):
        raise ValueError(f"workspace version is not stable semantic versioning: {version}")
    return version


def validate_lock(root: Path, version: str) -> None:
    lock = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))
    packages = {
        package["name"]: package["version"]
        for package in lock.get("package", [])
        if package.get("name") in WORKSPACE_PACKAGES
    }
    missing = sorted(set(WORKSPACE_PACKAGES) - packages.keys())
    if missing:
        raise ValueError("Cargo.lock is missing workspace packages: " + ", ".join(missing))
    mismatched = sorted(name for name, package_version in packages.items() if package_version != version)
    if mismatched:
        details = ", ".join(f"{name}={packages[name]}" for name in mismatched)
        raise ValueError(f"Cargo.lock workspace versions do not match {version}: {details}")


def validate_changelog(root: Path) -> None:
    changelog = (root / "CHANGELOG.md").read_text(encoding="utf-8")
    if not re.search(r"^## Unreleased\s*$", changelog, re.M):
        raise ValueError("CHANGELOG.md has no Unreleased section")


def validate_binary(binary: Path, version: str) -> None:
    if not binary.is_file():
        raise ValueError(f"release binary does not exist: {binary}")
    result = subprocess.run(
        [str(binary), "--version"],
        check=False,
        capture_output=True,
        text=True,
        timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"{binary} --version failed with exit code {result.returncode}")
    if result.stdout.strip() != f"zelyra {version}":
        raise ValueError(
            f"{binary} reports {result.stdout.strip()!r}, expected {f'zelyra {version}'!r}"
        )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--tag", help="require this exact release tag")
    parser.add_argument("--binary", type=Path, help="also verify this CLI's --version output")
    parser.add_argument("--print-tag", action="store_true", help="print the matching tag only")
    args = parser.parse_args()

    try:
        version = workspace_version(args.root)
        validate_lock(args.root, version)
        validate_changelog(args.root)
        expected_tag = f"v{version}"
        if args.tag is not None:
            if not TAG.fullmatch(args.tag):
                raise ValueError(f"invalid release tag: {args.tag}")
            if args.tag != expected_tag and not args.tag.startswith(expected_tag + "-"):
                raise ValueError(
                    f"release tag {args.tag} does not match workspace version {version}"
                )
        if args.binary is not None:
            validate_binary(args.binary, version)
    except (OSError, ValueError, tomllib.TOMLDecodeError, subprocess.SubprocessError) as error:
        print(f"release metadata check failed: {error}", file=sys.stderr)
        return 1

    if args.print_tag:
        print(expected_tag)
    else:
        print(f"release metadata is consistent for {expected_tag}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
