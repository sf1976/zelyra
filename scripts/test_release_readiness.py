#!/usr/bin/env python3
"""Tests for the final-release readiness guard."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from check_release_readiness import unfinished_gates


class ReleaseReadinessTests(unittest.TestCase):
    def test_non_030_version_has_no_final_gate(self) -> None:
        self.assertEqual(unfinished_gates(Path("."), "0.2.0"), [])

    def test_current_030_plan_is_not_ready(self) -> None:
        root = Path(__file__).resolve().parents[1]
        failures = unfinished_gates(root, "0.3.0")
        self.assertGreaterEqual(len(failures), 1)
        self.assertTrue(any("human onboarding" in failure for failure in failures))

    def test_all_required_gates_can_be_checked(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for relative_path, gates in (
                ("docs/release-plans/0.3.0.en.md", (
                    "Run at least one documented onboarding test with a person",
                    "Build release binaries and archives for the targets actually supported",
                    "Smoke-test installation and update paths for the published artifacts",
                    "All included P0 issues are fixed or explicitly deferred with a visible",
                    "Installation artifacts, checksums, and update path are verified",
                )),
                ("docs/release-plans/0.3.0.de.md", (
                    "Mindestens einen dokumentierten Einsteigertest mit einer Person",
                    "Release-Binärdateien und Archive für die tatsächlich unterstützten",
                    "Installations- und Updatepfad der veröffentlichten Artefakte smoke-testen",
                    "Alle aufgenommenen P0-Fehler behoben oder mit sichtbarer Begründung",
                    "Installationsartefakte, Prüfsummen und Updatepfad geprüft",
                )),
            ):
                path = root / relative_path
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("\n".join(f"- [x] {marker}" for marker in gates), encoding="utf-8")
            self.assertEqual(unfinished_gates(root, "0.3.0"), [])


if __name__ == "__main__":
    unittest.main()
