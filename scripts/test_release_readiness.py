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

    def test_current_030_plan_requires_a_documented_onboarding_deferral(self) -> None:
        root = Path(__file__).resolve().parents[1]
        failures = unfinished_gates(root, "0.3.0")
        self.assertEqual(failures, [])
        decision = (root / "docs/release-readiness/0.3.0-human-gate-decision.en.md").read_text(encoding="utf-8")
        self.assertIn("not conducted for 0.3.0", decision)
        self.assertIn("mandatory release gate for 0.4.0", decision)

    def test_040_plan_keeps_human_acceptance_mandatory(self) -> None:
        root = Path(__file__).resolve().parents[1]
        english = (root / "docs/release-plans/0.4.0.en.md").read_text(encoding="utf-8")
        german = (root / "docs/release-plans/0.4.0.de.md").read_text(encoding="utf-8")
        self.assertIn("- [ ] Run a real human acceptance test", english)
        self.assertIn("- [ ] Einen echten menschlichen Abnahmetest", german)

    def test_all_required_gates_can_be_checked(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for relative_path, gates in (
                ("docs/release-plans/0.3.0.en.md", (
                    "Defer the independent human onboarding test from 0.3.0 to 0.4.0 by",
                    "Build release binaries and archives for the targets actually supported",
                    "Smoke-test installation and update paths for the published artifacts",
                    "All included P0 issues are fixed or explicitly deferred with a visible",
                    "Installation artifacts, checksums, and update path are verified",
                )),
                ("docs/release-plans/0.3.0.de.md", (
                    "Den unabhängigen menschlichen Einsteigertest für 0.3.0 auf ausdrückliche",
                    "Release-Binärdateien und Archive für die tatsächlich unterstützten",
                    "Installations- und Updatepfad der veröffentlichten Artefakte smoke-testen",
                    "Alle aufgenommenen P0-Fehler behoben oder mit sichtbarer Begründung",
                    "Installationsartefakte, Prüfsummen und Updatepfad geprüft",
                )),
            ):
                path = root / relative_path
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("\n".join(f"- [x] {marker}" for marker in gates), encoding="utf-8")

            records = {
                "docs/release-readiness/0.3.0-human-gate-decision.en.md": (
                    "not conducted for 0.3.0; mandatory release gate for 0.4.0; not a test result"
                ),
                "docs/release-readiness/0.3.0-human-gate-decision.de.md": (
                    "nicht durchgeführt für 0.3.0; verpflichtendes Gate 0.4.0; kein Testergebnis"
                ),
            }
            for relative_path, content in records.items():
                path = root / relative_path
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content, encoding="utf-8")

            self.assertEqual(unfinished_gates(root, "0.3.0"), [])

    def test_human_gate_checkbox_without_decision_record_is_blocked(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for relative_path, gates in (
                ("docs/release-plans/0.3.0.en.md", (
                    "Defer the independent human onboarding test from 0.3.0 to 0.4.0 by",
                    "Build release binaries and archives for the targets actually supported",
                    "Smoke-test installation and update paths for the published artifacts",
                    "All included P0 issues are fixed or explicitly deferred with a visible",
                    "Installation artifacts, checksums, and update path are verified",
                )),
                ("docs/release-plans/0.3.0.de.md", (
                    "Den unabhängigen menschlichen Einsteigertest für 0.3.0 auf ausdrückliche",
                    "Release-Binärdateien und Archive für die tatsächlich unterstützten",
                    "Installations- und Updatepfad der veröffentlichten Artefakte smoke-testen",
                    "Alle aufgenommenen P0-Fehler behoben oder mit sichtbarer Begründung",
                    "Installationsartefakte, Prüfsummen und Updatepfad geprüft",
                )),
            ):
                path = root / relative_path
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("\n".join(f"- [x] {marker}" for marker in gates), encoding="utf-8")

            failures = unfinished_gates(root, "0.3.0")
            self.assertTrue(any("decision record missing" in failure for failure in failures))


if __name__ == "__main__":
    unittest.main()
