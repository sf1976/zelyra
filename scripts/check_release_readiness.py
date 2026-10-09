#!/usr/bin/env python3
"""Guard final release tags against unfinished gates and undocumented waivers."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REQUIRED_GATES = {
    "docs/release-plans/0.3.0.en.md": (
        ("human onboarding deferral", "Defer the independent human onboarding test from 0.3.0 to 0.4.0 by"),
        ("candidate artifacts", "Build release binaries and archives for the targets actually supported"),
        ("published install/update smoke", "Smoke-test installation and update paths for the published artifacts"),
        ("P0 decisions", "All included P0 issues are fixed or explicitly deferred with a visible"),
        ("artifact/update verification", "Installation artifacts, checksums, and update path are verified"),
    ),
    "docs/release-plans/0.3.0.de.md": (
        ("Vertagung des menschlichen Einsteigertests", "Den unabhängigen menschlichen Einsteigertest für 0.3.0 auf ausdrückliche"),
        ("Kandidatenartefakte", "Release-Binärdateien und Archive für die tatsächlich unterstützten"),
        ("veröffentlichter Installations-/Update-Smoke", "Installations- und Updatepfad der veröffentlichten Artefakte smoke-testen"),
        ("P0-Entscheidungen", "Alle aufgenommenen P0-Fehler behoben oder mit sichtbarer Begründung"),
        ("Artefakt-/Updateprüfung", "Installationsartefakte, Prüfsummen und Updatepfad geprüft"),
    ),
}

REQUIRED_DECISION_RECORDS = {
    "docs/release-readiness/0.3.0-human-gate-decision.en.md": (
        "not conducted for 0.3.0",
        "mandatory release gate for 0.4.0",
        "not a test result",
        "decision authority: project owner",
    ),
    "docs/release-readiness/0.3.0-human-gate-decision.de.md": (
        "nicht durchgeführt",
        "0.4.0",
        "kein testergebnis",
        "entscheidungsträger: projektverantwortlicher",
    ),
}

REQUIRED_GATES_040 = {
    "docs/release-plans/0.4.0.en.md": (
        ("P0 implementation and tests", "All P0 items are implemented, reviewed, and backed by positive and"),
        ("module and migration recovery acceptance", "The multi-file example, generated business application, and a migration"),
        ("database compatibility claims", "MariaDB compatibility claims match the tested matrix; unsupported"),
        ("security review and residual risks", "Security review, threat model, dependency audit, and residual risks"),
        ("machine interface compatibility", "CLI JSON, diagnostics, project files, migration plans, and release"),
        ("independent human acceptance", "Run a real human acceptance test with a person who does not develop"),
        ("published candidate verification", "Release artifacts, checksums, SBOM/provenance, upgrade/rollback smoke,"),
        ("all mandatory release gates", "Publish `v0.4.0` only after every mandatory gate passes."),
    ),
    "docs/release-plans/0.4.0.de.md": (
        ("P0-Implementierung und Tests", "Alle P0-Punkte sind implementiert, geprüft und durch positive sowie"),
        ("Modul- und Migrationsabnahme", "Mehrdateien-Beispiel, erzeugte Businessanwendung und ein"),
        ("Datenbank-Kompatibilitätsaussagen", "MariaDB-Aussagen entsprechen der getesteten Matrix; nicht unterstütztes"),
        ("Sicherheitsreview und Restgefahren", "Sicherheitsreview, Threat Model, Dependency Audit und Restgefahren sind"),
        ("Kompatibilität der Maschinenschnittstellen", "CLI-JSON, Diagnosen, Projektdateien, Migrationspläne und Releaseartefakte"),
        ("Unabhängige menschliche Abnahme", "Einen echten menschlichen Abnahmetest mit einer Person durchführen, die"),
        ("Prüfung des veröffentlichten Kandidaten", "Releaseartefakte, Prüfsummen, SBOM/Provenance sowie Upgrade-/Rollback-"),
        ("Alle verpflichtenden Release-Gates", "`v0.4.0` erst veröffentlichen, wenn jedes verpflichtende Gate erfüllt"),
    ),
}

REQUIRED_HUMAN_ACCEPTANCE_RECORDS_040 = {
    "docs/release-readiness/0.4.0-human-acceptance.en.md": (
        "candidate commit:",
        "completed on:",
        "decision: accepted",
        "decision authority: project owner",
    ),
    "docs/release-readiness/0.4.0-human-acceptance.de.md": (
        "kandidaten-commit:",
        "abgeschlossen am:",
        "entscheidung: akzeptiert",
        "entscheidungsträger: projektverantwortlicher",
    ),
}


def _checkbox_is_checked(text: str, marker: str) -> bool:
    for line in text.splitlines():
        if marker in line:
            return bool(re.match(r"^\s*- \[x\]", line))
    return False


def unfinished_gates(
    root: Path, version: str, candidate_commit: str | None = None
) -> list[str]:
    if version not in {"0.3.0", "0.4.0"}:
        return []

    failures: list[str] = []
    required_gates = REQUIRED_GATES if version == "0.3.0" else REQUIRED_GATES_040
    for relative_path, gates in required_gates.items():
        path = root / relative_path
        text = path.read_text(encoding="utf-8")
        for name, marker in gates:
            if not _checkbox_is_checked(text, marker):
                failures.append(f"{name} ({relative_path})")

    if version == "0.3.0":
        required_records = REQUIRED_DECISION_RECORDS
    else:
        required_records = REQUIRED_HUMAN_ACCEPTANCE_RECORDS_040
        english_plan = root / "docs/release-plans/0.4.0.en.md"
        try:
            english_text = english_plan.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as error:
            return failures + [f"could not read 0.4.0 human acceptance gate: {error}"]
        if not _checkbox_is_checked(english_text, "Run a real human acceptance test"):
            required_records = {}

    for relative_path, markers in required_records.items():
        path = root / relative_path
        try:
            raw_text = path.read_text(encoding="utf-8")
            text = " ".join(raw_text.casefold().split())
            text = re.sub(r"[*_`]", "", text)
        except FileNotFoundError:
            failures.append(f"human decision record missing ({relative_path})")
            continue
        for marker in markers:
            normalized_marker = re.sub(r"[*_`]", "", " ".join(marker.casefold().split()))
            if normalized_marker not in text:
                failures.append(f"human onboarding decision record incomplete ({relative_path})")
                break
        else:
            if version == "0.4.0" and any(
                placeholder in text
                for placeholder in ("pending", "ausstehend", "[todo]", "tbd")
            ):
                failures.append(f"human acceptance record contains placeholders ({relative_path})")
            if version == "0.4.0" and candidate_commit is not None:
                commit_label = (
                    "kandidaten-commit:" if relative_path.endswith(".de.md") else "candidate commit:"
                )
                recorded_commit = next(
                    (
                        line.strip().split(":", 1)[1].strip().strip("`")
                        for line in raw_text.splitlines()
                        if line.strip().casefold().lstrip("-* ").startswith(commit_label)
                    ),
                    "",
                )
                if recorded_commit.casefold() != candidate_commit.casefold():
                    failures.append(
                        f"human acceptance record is for a different candidate commit ({relative_path})"
                    )
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--version", required=True, help="workspace version being tagged")
    parser.add_argument(
        "--candidate-commit",
        help="exact commit the 0.4.0 human acceptance must cover",
    )
    args = parser.parse_args()

    try:
        failures = unfinished_gates(args.root, args.version, args.candidate_commit)
    except (OSError, UnicodeError) as error:
        print(f"release readiness check failed: {error}", file=sys.stderr)
        return 1

    if failures:
        print(f"final {args.version} release is blocked by unfinished gates:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1

    print(f"release readiness gates passed for {args.version}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
