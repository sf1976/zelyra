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
        ("human acceptance deferral", "Defer independent human acceptance to 0.5.0 by explicit project-owner"),
        ("published candidate verification", "Release artifacts, checksums, SBOM/provenance, upgrade/rollback smoke,"),
        ("all mandatory release gates", "Publish `v0.4.0` only after every technical, artifact, security, and"),
    ),
    "docs/release-plans/0.4.0.de.md": (
        ("P0-Implementierung und Tests", "Alle P0-Punkte sind implementiert, geprüft und durch positive sowie"),
        ("Modul- und Migrationsabnahme", "Mehrdateien-Beispiel, erzeugte Businessanwendung und ein"),
        ("Datenbank-Kompatibilitätsaussagen", "MariaDB-Aussagen entsprechen der getesteten Matrix; nicht unterstütztes"),
        ("Sicherheitsreview und Restgefahren", "Sicherheitsreview, Threat Model, Dependency Audit und Restgefahren sind"),
        ("Kompatibilität der Maschinenschnittstellen", "CLI-JSON, Diagnosen, Projektdateien, Migrationspläne und Releaseartefakte"),
        ("Vertagung der menschlichen Abnahme", "Die unabhängige menschliche Abnahme auf ausdrückliche Entscheidung des"),
        ("Prüfung des veröffentlichten Kandidaten", "Releaseartefakte, Prüfsummen, SBOM/Provenance sowie Upgrade-/Rollback-"),
        ("Alle verpflichtenden Release-Gates", "`v0.4.0` erst veröffentlichen, wenn alle technischen, Artefakt-"),
    ),
}

REQUIRED_GATES_050 = {
    "docs/release-plans/0.5.0.en.md": (
        ("human acceptance deferral", "Record the explicit owner decision to defer independent human acceptance"),
        ("published candidate verification", "Release artifacts, checksums, installation, update, and rollback verification"),
    ),
    "docs/release-plans/0.5.0.de.md": (
        ("Vertagung der menschlichen Abnahme", "Die ausdrückliche Entscheidung, die unabhängige menschliche Abnahme auf"),
        ("Prüfung des veröffentlichten Kandidaten", "Releaseartefakte, Prüfsummen, Installation, Update und Rollback"),
    ),
}

REQUIRED_GATES_100 = {
    "docs/release-plans/1.0.0.en.md": (
        ("frozen release scope", "Freeze the 1.0.0 product scope, support matrix"),
        ("published candidate human study", "Publish an immutable 1.0.0 release candidate and conduct the independent"),
        ("completed human acceptance", "Complete the bilingual human acceptance record with the exact candidate"),
        ("stable release gate", "Publish stable `v1.0.0` only after every technical gate and the human"),
    ),
    "docs/release-plans/1.0.0.de.md": (
        ("festgelegter Releaseumfang", "Produktumfang, Supportmatrix, Kompatibilitätszusagen und Dokumentation"),
        ("veröffentlichte Kandidatenstudie", "Einen unveränderlichen 1.0.0-Release-Kandidaten veröffentlichen und die"),
        ("abgeschlossene menschliche Abnahme", "Das zweisprachige Abnahmeprotokoll mit exaktem Kandidaten-Tag und Commit"),
        ("stabiles Release-Gate", "`v1.0.0` stabil erst veröffentlichen, wenn alle technischen Gates und die"),
    ),
}

REQUIRED_HUMAN_DECISION_RECORDS_040 = {
    "docs/release-readiness/0.4.0-human-acceptance.en.md": (
        "decision: deferred to 0.5.0",
        "not conducted",
        "not a test result",
        "decision authority: project owner",
    ),
    "docs/release-readiness/0.4.0-human-acceptance.de.md": (
        "entscheidung: auf 0.5.0 verschoben",
        "nicht durchgeführt",
        "kein testergebnis",
        "entscheidungsträger: projektverantwortlicher",
    ),
}

REQUIRED_HUMAN_ACCEPTANCE_RECORDS_050 = {
    "docs/release-readiness/0.5.0-human-acceptance.en.md": (
        "decision: deferred to 1.0.0",
        "not conducted; not a test result",
        "candidate tag:",
        "candidate commit:",
        "decision authority: project owner",
    ),
    "docs/release-readiness/0.5.0-human-acceptance.de.md": (
        "entscheidung: auf 1.0.0 verschoben",
        "nicht durchgeführt; kein testergebnis",
        "kandidaten-tag:",
        "kandidaten-commit:",
        "entscheidungsträger: projektverantwortlicher",
    ),
}

REQUIRED_HUMAN_ACCEPTANCE_RECORDS_100 = {
    "docs/release-readiness/1.0.0-human-acceptance.en.md": (
        "candidate tag:",
        "candidate commit:",
        "study completed on:",
        "participant count",
        "task outcomes:",
        "interventions:",
        "redacted observations:",
        "blockers:",
        "decision: accepted",
        "decision authority: project owner",
    ),
    "docs/release-readiness/1.0.0-human-acceptance.de.md": (
        "kandidaten-tag:",
        "kandidaten-commit:",
        "studie abgeschlossen am:",
        "anzahl teilnehmende",
        "aufgabenergebnisse:",
        "eingriffe:",
        "redigierte beobachtungen:",
        "blockaden:",
        "entscheidung: akzeptiert",
        "entscheidungsträger: projektverantwortlicher",
    ),
}

REQUIRED_CANDIDATE_RECORD_040 = (
    "candidate tag:",
    "candidate source commit:",
    "linux archive sha-256:",
    "linux sbom sha-256:",
    "windows archive sha-256:",
    "windows sbom sha-256:",
    "linux attestation:",
    "windows attestation:",
    "upgrade smoke:",
    "rollback smoke:",
)


def _checkbox_is_checked(text: str, marker: str) -> bool:
    for line in text.splitlines():
        if marker in line:
            return bool(re.match(r"^\s*- \[x\]", line))
    return False


def unfinished_gates(
    root: Path, version: str, candidate_commit: str | None = None
) -> list[str]:
    if version not in {"0.3.0", "0.4.0", "0.5.0", "1.0.0"}:
        return []

    failures: list[str] = []
    required_gates = {
        "0.3.0": REQUIRED_GATES,
        "0.4.0": REQUIRED_GATES_040,
        "0.5.0": REQUIRED_GATES_050,
        "1.0.0": REQUIRED_GATES_100,
    }[version]
    for relative_path, gates in required_gates.items():
        path = root / relative_path
        text = path.read_text(encoding="utf-8")
        for name, marker in gates:
            if not _checkbox_is_checked(text, marker):
                failures.append(f"{name} ({relative_path})")

    if version == "0.4.0":
        candidate_path = root / "docs/release-readiness/0.4.0-candidate-verification.md"
        try:
            candidate_text = candidate_path.read_text(encoding="utf-8")
            flattened = re.sub(r"[*_`]", "", " ".join(candidate_text.casefold().split()))
            missing = [marker for marker in REQUIRED_CANDIDATE_RECORD_040 if marker not in flattened]
            if missing:
                failures.append(
                    "candidate verification record incomplete "
                    "(docs/release-readiness/0.4.0-candidate-verification.md)"
                )
            if any(placeholder in flattened for placeholder in ("pending", "ausstehend", "[todo]", "tbd", "not run")):
                failures.append(
                    "candidate verification record contains placeholders "
                    "(docs/release-readiness/0.4.0-candidate-verification.md)"
                )
        except (OSError, UnicodeError):
            failures.append("candidate verification record missing (docs/release-readiness/0.4.0-candidate-verification.md)")

    if version == "0.3.0":
        required_records = REQUIRED_DECISION_RECORDS
    elif version == "0.4.0":
        required_records = {
            **REQUIRED_HUMAN_DECISION_RECORDS_040,
            "docs/release-readiness/0.4.0-candidate-verification.md": REQUIRED_CANDIDATE_RECORD_040,
        }
    elif version == "1.0.0":
        required_records = REQUIRED_HUMAN_ACCEPTANCE_RECORDS_100
    else:
        required_records = REQUIRED_HUMAN_ACCEPTANCE_RECORDS_050

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
            if version in {"0.4.0", "0.5.0", "1.0.0"} and any(
                placeholder in text
                for placeholder in ("pending", "ausstehend", "[todo]", "tbd")
            ):
                failures.append(f"release readiness record contains placeholders ({relative_path})")
            if version == "0.4.0" and relative_path.endswith("candidate-verification.md"):
                recorded_commit = _record_value(raw_text, "candidate source commit:")
                recorded_tag = _record_value(raw_text, "candidate tag:")
                if not re.fullmatch(r"[0-9a-f]{40}", recorded_commit.casefold()):
                    failures.append(f"candidate source commit is invalid ({relative_path})")
                if not re.fullmatch(r"v0\.4\.0-rc\.[0-9]+", recorded_tag.casefold()):
                    failures.append(f"candidate tag is invalid ({relative_path})")
                for label in (
                    "linux archive sha-256:",
                    "linux sbom sha-256:",
                    "windows archive sha-256:",
                    "windows sbom sha-256:",
                ):
                    if not re.fullmatch(r"[0-9a-f]{64}", _record_value(raw_text, label).casefold()):
                        failures.append(f"candidate artifact digest is invalid ({relative_path})")
                for label in ("linux attestation:", "windows attestation:"):
                    if not _record_value(raw_text, label).casefold().startswith("verified"):
                        failures.append(f"candidate attestation is not verified ({relative_path})")
                for label in ("upgrade smoke:", "rollback smoke:"):
                    if not _record_value(raw_text, label).casefold().startswith("passed"):
                        failures.append(f"candidate {label[:-1]} is not passed ({relative_path})")
                if candidate_commit is not None and recorded_commit.casefold() != candidate_commit.casefold():
                    failures.append(
                        f"candidate record does not match published candidate tag ({relative_path})"
                    )
            if version in {"0.5.0", "1.0.0"} and candidate_commit is not None:
                commit_label = (
                    "kandidaten-commit:" if relative_path.endswith(".de.md") else "candidate commit:"
                )
                tag_label = "kandidaten-tag:" if relative_path.endswith(".de.md") else "candidate tag:"
                recorded_commit = next(
                    (
                        line.strip().split(":", 1)[1].strip().strip("`")
                        for line in raw_text.splitlines()
                        if line.strip().casefold().lstrip("-* ").startswith(commit_label)
                    ),
                    "",
                )
                recorded_tag = _record_value(raw_text, tag_label)
                if not re.fullmatch(r"[0-9a-f]{40}", recorded_commit.casefold()):
                    failures.append(f"human acceptance candidate commit is invalid ({relative_path})")
                if not re.fullmatch(rf"v{re.escape(version)}-rc\.[0-9]+", recorded_tag.casefold()):
                    failures.append(f"human acceptance candidate tag is invalid ({relative_path})")
                if recorded_commit.casefold() != candidate_commit.casefold():
                    failures.append(
                        f"human acceptance record is for a different candidate commit ({relative_path})"
                    )
    return failures


def _record_value(raw_text: str, label: str) -> str:
    normalized_label = label.casefold()
    for line in raw_text.splitlines():
        cleaned = line.strip().lstrip("-* ").strip()
        if cleaned.casefold().startswith(normalized_label):
            return cleaned.split(":", 1)[1].strip().strip("` ")
    return ""


def candidate_tag(root: Path, version: str) -> str:
    if version not in {"0.4.0", "0.5.0", "1.0.0"}:
        raise ValueError(f"candidate tags are not configured for {version}")
    paths = (
        [root / "docs/release-readiness/0.4.0-candidate-verification.md"]
        if version == "0.4.0"
        else (
            [
                root / "docs/release-readiness/0.5.0-human-acceptance.en.md",
                root / "docs/release-readiness/0.5.0-human-acceptance.de.md",
            ]
            if version == "0.5.0"
            else [
                root / "docs/release-readiness/1.0.0-human-acceptance.en.md",
                root / "docs/release-readiness/1.0.0-human-acceptance.de.md",
            ]
        )
    )
    values = [
        _record_value(
            path.read_text(encoding="utf-8"),
            "kandidaten-tag:" if path.name.endswith(".de.md") else "candidate tag:",
        )
        for path in paths
    ]
    if not values or not values[0] or any(value.casefold() != values[0].casefold() for value in values):
        raise ValueError(f"candidate tag missing or inconsistent for {version}")
    if not re.fullmatch(rf"v{re.escape(version)}-rc\.[0-9]+", values[0].casefold()):
        raise ValueError(f"candidate tag is invalid for {version}")
    return values[0]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--version", required=True, help="workspace version being tagged")
    parser.add_argument(
        "--candidate-commit",
        help="exact candidate tag commit the release or human decision record must cover",
    )
    parser.add_argument("--print-candidate-tag", action="store_true")
    args = parser.parse_args()

    if args.print_candidate_tag:
        try:
            print(candidate_tag(args.root, args.version))
        except (OSError, UnicodeError, ValueError) as error:
            print(f"release readiness check failed: {error}", file=sys.stderr)
            return 1
        return 0

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
