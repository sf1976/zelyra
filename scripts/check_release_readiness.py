#!/usr/bin/env python3
"""Guard the final 0.3.0 tag against unfinished mandatory release gates."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REQUIRED_GATES = {
    "docs/release-plans/0.3.0.en.md": (
        ("human onboarding", "Run at least one documented onboarding test with a person"),
        ("candidate artifacts", "Build release binaries and archives for the targets actually supported"),
        ("published install/update smoke", "Smoke-test installation and update paths for the published artifacts"),
        ("P0 decisions", "All included P0 issues are fixed or explicitly deferred with a visible"),
        ("artifact/update verification", "Installation artifacts, checksums, and update path are verified"),
    ),
    "docs/release-plans/0.3.0.de.md": (
        ("menschlicher Einsteigertest", "Mindestens einen dokumentierten Einsteigertest mit einer Person"),
        ("Kandidatenartefakte", "Release-Binärdateien und Archive für die tatsächlich unterstützten"),
        ("veröffentlichter Installations-/Update-Smoke", "Installations- und Updatepfad der veröffentlichten Artefakte smoke-testen"),
        ("P0-Entscheidungen", "Alle aufgenommenen P0-Fehler behoben oder mit sichtbarer Begründung"),
        ("Artefakt-/Updateprüfung", "Installationsartefakte, Prüfsummen und Updatepfad geprüft"),
    ),
}


def _checkbox_is_checked(text: str, marker: str) -> bool:
    for line in text.splitlines():
        if marker in line:
            return bool(re.match(r"^\s*- \[x\]", line))
    return False


def unfinished_gates(root: Path, version: str) -> list[str]:
    if version != "0.3.0":
        return []

    failures: list[str] = []
    for relative_path, gates in REQUIRED_GATES.items():
        path = root / relative_path
        text = path.read_text(encoding="utf-8")
        for name, marker in gates:
            if not _checkbox_is_checked(text, marker):
                failures.append(f"{name} ({relative_path})")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--version", required=True, help="workspace version being tagged")
    args = parser.parse_args()

    try:
        failures = unfinished_gates(args.root, args.version)
    except (OSError, UnicodeError) as error:
        print(f"release readiness check failed: {error}", file=sys.stderr)
        return 1

    if failures:
        print("final 0.3.0 release is blocked by unfinished gates:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1

    print(f"release readiness gates passed for {args.version}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
