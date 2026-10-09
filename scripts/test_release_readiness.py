#!/usr/bin/env python3
"""Tests for the final-release readiness guard."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from check_release_readiness import REQUIRED_GATES_040, unfinished_gates


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

    def test_current_040_plan_blocks_release_on_open_gates(self) -> None:
        root = Path(__file__).resolve().parents[1]
        failures = unfinished_gates(root, "0.4.0")
        self.assertTrue(any("independent human acceptance" in item for item in failures))
        self.assertTrue(any("P0 implementation and tests" in item for item in failures))

    def test_all_040_gates_require_bilingual_human_acceptance_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for relative_path, gates in REQUIRED_GATES_040.items():
                path = root / relative_path
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(
                    "\n".join(f"- [x] {marker}" for _, marker in gates),
                    encoding="utf-8",
                )

            records = {
                "docs/release-readiness/0.4.0-human-acceptance.en.md": (
                    "Candidate commit: abc123\nCompleted on: 2026-10-09\n"
                    "Decision: accepted\nDecision authority: project owner\n"
                ),
                "docs/release-readiness/0.4.0-human-acceptance.de.md": (
                    "Kandidaten-Commit: abc123\nAbgeschlossen am: 2026-10-09\n"
                    "Entscheidung: akzeptiert\nEntscheidungsträger: Projektverantwortlicher\n"
                ),
            }
            for relative_path, content in records.items():
                path = root / relative_path
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content, encoding="utf-8")

            self.assertEqual(unfinished_gates(root, "0.4.0"), [])

            english = root / "docs/release-readiness/0.4.0-human-acceptance.en.md"
            english.write_text(
                "Candidate commit: PENDING\nCompleted on: 2026-10-09\n"
                "Decision: accepted\nDecision authority: project owner\n",
                encoding="utf-8",
            )
            failures = unfinished_gates(root, "0.4.0")
            self.assertTrue(any("contains placeholders" in item for item in failures))

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
                    "not conducted for 0.3.0; mandatory release gate for 0.4.0; "
                    "not a test result; Decision authority: project owner"
                ),
                "docs/release-readiness/0.3.0-human-gate-decision.de.md": (
                    "nicht durchgeführt für 0.3.0; verpflichtendes Gate 0.4.0; "
                    "kein Testergebnis; Entscheidungsträger: Projektverantwortlicher"
                ),
            }
            for relative_path, content in records.items():
                path = root / relative_path
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content, encoding="utf-8")

            self.assertEqual(unfinished_gates(root, "0.3.0"), [])

            for relative_path, without_owner_approval in (
                (
                    "docs/release-readiness/0.3.0-human-gate-decision.en.md",
                    "not conducted for 0.3.0; mandatory release gate for 0.4.0; not a test result",
                ),
                (
                    "docs/release-readiness/0.3.0-human-gate-decision.de.md",
                    "nicht durchgeführt für 0.3.0; verpflichtendes Gate 0.4.0; kein Testergebnis",
                ),
            ):
                (root / relative_path).write_text(
                    without_owner_approval, encoding="utf-8"
                )
                failures = unfinished_gates(root, "0.3.0")
                self.assertTrue(
                    any("decision record incomplete" in failure for failure in failures),
                    f"missing owner approval was accepted in {relative_path}",
                )
                (root / relative_path).write_text(
                    without_owner_approval
                    + (
                        "; Decision authority: project owner"
                        if relative_path.endswith(".en.md")
                        else "; Entscheidungsträger: Projektverantwortlicher"
                    ),
                    encoding="utf-8",
                )

    def test_release_notice_links_to_tag_pinned_decision_records(self) -> None:
        root = Path(__file__).resolve().parents[1]
        workflow = (root / ".github/workflows/release.yml").read_text(
            encoding="utf-8"
        )
        self.assertIn(
            "blob/v0.3.0/docs/release-readiness/0.3.0-human-gate-decision.en.md",
            workflow,
        )
        self.assertIn(
            "blob/v0.3.0/docs/release-readiness/0.3.0-human-gate-decision.de.md",
            workflow,
        )
        self.assertNotIn(
            "blob/main/docs/release-readiness/0.3.0-human-gate-decision", workflow
        )
        self.assertIn(
            "python3 scripts/check_release_readiness.py --version",
            workflow,
        )
        self.assertIn(
            'if [[ "${release_tag}" == "v${package_version}" ]]; then',
            workflow,
        )
        self.assertIn(
            "blob/v0.4.0/docs/release-readiness/0.4.0-human-acceptance.en.md",
            workflow,
        )
        self.assertIn(
            "blob/v0.4.0/docs/release-readiness/0.4.0-human-acceptance.de.md",
            workflow,
        )

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
