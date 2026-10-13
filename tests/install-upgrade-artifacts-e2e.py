#!/usr/bin/env python3
"""Comprehensive E2E test suite for Installation, Upgrade, Rollback, and Release Artifacts (Section 4.D).

Verifies Section 4.D of technical release gates:
1. Clean End-User Installation & Onboarding:
   - Prefix installation without Rustup/Rust toolchain dependencies (install.sh --no-rustup).
   - Verification of installed executable (install.sh --check), version output.
   - Idempotent reinstallation and clean uninstallation (install.sh --uninstall).
   - Generated MariaDB project onboarding lifecycle (clean-onboarding-e2e.sh).
2. Guided Invoice Tutorial & Business Calculations:
   - Project creation via CLI (zelyra --tutorial invoice --no-start).
   - Multi-file modular structure (database, shell, customers, items, invoices, dashboard).
   - Idempotent schema migrations on MariaDB.
   - Net, 19% VAT, and Gross calculation integrity; PDF-ready print layout without CDN dependencies.
3. Backup, Restore, and Crash Recovery:
   - Scoped database dump with read-only permissions (mariadb-dump).
   - Partial failure protection: .partial temporary dump files never masquerade as complete backups.
   - Scoped database restore with target-isolated user.
   - Server-crash recovery: interrupted migrations at the DDL/journal boundary recover cleanly from fresh plans.
4. Reproducible Release Artifacts, Checksums, and Provenance:
   - Metadata consistency check across Cargo.toml, scripts, and binary (v0.9.9).
   - Packaging with deterministic SOURCE_DATE_EPOCH.
   - Strict SHA-256 sidecar validation.
   - Archive path security: no absolute paths, no directory traversal ('..').
   - Packaged CLI execution and release installation check.

Zero disruption to production databases; uses disposable containers on ephemeral ports.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]
BIN = Path(os.environ.get("ZELYRA_BIN", ROOT / "target/release/zelyra")).resolve()


def log_step(name):
    print(f"\n{'='*70}\n[STEP] {name}\n{'='*70}")


def run_part1_installer_and_clean_onboarding(bin_path):
    log_step("Part 1: Installer and Clean End-User Onboarding Rehearsal")
    print("  -> Running tests/installer-e2e.sh...")
    res = subprocess.run(["bash", str(ROOT / "tests/installer-e2e.sh")], capture_output=True, text=True)
    if res.returncode != 0:
        print("STDERR:\n", res.stderr)
        raise RuntimeError(f"installer-e2e.sh failed with return code {res.returncode}")
    print("     [OK] Installer E2E passed (malformed tag rejection, prefix install, check, repeatable install, clean uninstall)")

    print("  -> Running clean onboarding E2E with disposable MariaDB...")
    container_name = f"zelyra-test-onboard-{os.getpid()}-{int(time.time())}"
    cmd = [
        "docker", "run", "-d", "--name", container_name, "-p", "127.0.0.1:3308:3306",
        "-e", "MARIADB_ROOT_PASSWORD=onboarding_pass",
        "--health-cmd=mariadb-admin ping -h 127.0.0.1 -u root -ponboarding_pass",
        "--health-interval=1s", "--health-retries=30",
        "mariadb:11.4.13"
    ]
    subprocess.run(cmd, check=True, capture_output=True)
    try:
        deadline = time.monotonic() + 45
        while True:
            info = json.loads(subprocess.check_output(["docker", "inspect", container_name]))[0]
            if info.get("State", {}).get("Health", {}).get("Status") == "healthy":
                break
            if time.monotonic() > deadline:
                raise RuntimeError("MariaDB onboarding container timed out")
            time.sleep(0.5)

        env = {
            **dict(os.environ),
            "ZELYRA_BIN": str(bin_path),
            "ZELYRA_CLEAN_ONBOARDING_DB_PORT": "3308",
            "ZELYRA_CLEAN_ONBOARDING_ROOT_PASSWORD": "onboarding_pass",
        }
        res = subprocess.run(["bash", str(ROOT / "tests/clean-onboarding-e2e.sh")], env=env, capture_output=True, text=True)
        if res.returncode != 0:
            print("STDERR:\n", res.stderr)
            raise RuntimeError(f"clean-onboarding-e2e.sh failed with return code {res.returncode}")
        print("     [OK] Clean onboarding E2E passed (fresh MariaDB project, Compose, doctor, schema bootstrap, CRUD operations)")
    finally:
        subprocess.run(["docker", "rm", "-f", container_name], capture_output=True)


def run_part2_invoice_tutorial(bin_path):
    log_step("Part 2: Guided Invoice Tutorial & Business Calculations")
    container_name = f"zelyra-test-inv-{os.getpid()}-{int(time.time())}"
    cmd = [
        "docker", "run", "-d", "--name", container_name, "-p", "127.0.0.1::3306",
        "-e", "MARIADB_ROOT_PASSWORD=test_pass",
        "-e", "MARIADB_DATABASE=zelyra_test",
        "--health-cmd=mariadb-admin ping -h 127.0.0.1 -u root -ptest_pass",
        "--health-interval=1s", "--health-retries=30",
        "mariadb:11.4.13"
    ]
    subprocess.run(cmd, check=True, capture_output=True)
    try:
        deadline = time.monotonic() + 45
        while True:
            info = json.loads(subprocess.check_output(["docker", "inspect", container_name]))[0]
            if info.get("State", {}).get("Health", {}).get("Status") == "healthy":
                break
            if time.monotonic() > deadline:
                raise RuntimeError("MariaDB invoice container timed out")
            time.sleep(0.5)

        port = info["NetworkSettings"]["Ports"]["3306/tcp"][0]["HostPort"]
        db_url = f"mariadb://root:test_pass@127.0.0.1:{port}/zelyra_test"
        env = {
            **dict(os.environ),
            "ZELYRA_BIN": str(bin_path),
            "DATABASE_URL": db_url,
        }
        print(f"  -> Running tests/invoice-tutorial-e2e.sh on port {port}...")
        res = subprocess.run(["bash", str(ROOT / "tests/invoice-tutorial-e2e.sh")], env=env, capture_output=True, text=True)
        if res.returncode != 0:
            print("STDERR:\n", res.stderr)
            raise RuntimeError(f"invoice-tutorial-e2e.sh failed with return code {res.returncode}")
        print("     [OK] Invoice tutorial E2E passed (modular layout, schema setup, customer creation, 19% VAT calculation, print layout)")
    finally:
        subprocess.run(["docker", "rm", "-f", container_name], capture_output=True)


def run_part3_backup_restore_and_crash_recovery(bin_path):
    log_step("Part 3: Backup/Restore and Crash Recovery Rehearsal")
    container_name = f"zelyra-test-bak-{os.getpid()}-{int(time.time())}"
    cmd = [
        "docker", "run", "-d", "--name", container_name, "-p", "127.0.0.1::3306",
        "-e", "MARIADB_ROOT_PASSWORD=test_pass",
        "-e", "MARIADB_DATABASE=zelyra_test",
        "--health-cmd=mariadb-admin ping -h 127.0.0.1 -u root -ptest_pass",
        "--health-interval=1s", "--health-retries=30",
        "mariadb:11.4.13"
    ]
    subprocess.run(cmd, check=True, capture_output=True)
    try:
        deadline = time.monotonic() + 45
        while True:
            info = json.loads(subprocess.check_output(["docker", "inspect", container_name]))[0]
            if info.get("State", {}).get("Health", {}).get("Status") == "healthy":
                break
            if time.monotonic() > deadline:
                raise RuntimeError("MariaDB backup container timed out")
            time.sleep(0.5)

        port = info["NetworkSettings"]["Ports"]["3306/tcp"][0]["HostPort"]
        db_url = f"mariadb://root:test_pass@127.0.0.1:{port}/zelyra_test"
        env = {
            **dict(os.environ),
            "ZELYRA_BACKUP_RESTORE_MARIADB_URL": db_url,
            "ZELYRA_BIN": str(bin_path),
        }
        print(f"  -> Running tests/mariadb-backup-restore-e2e.sh on port {port}...")
        res = subprocess.run(["bash", str(ROOT / "tests/mariadb-backup-restore-e2e.sh")], env=env, capture_output=True, text=True)
        if res.returncode != 0:
            print("STDERR:\n", res.stderr)
            raise RuntimeError(f"mariadb-backup-restore-e2e.sh failed with return code {res.returncode}")
        print("     [OK] MariaDB backup/restore E2E passed (scoped dump, atomic .partial handling, target-isolated restore)")

        print("  -> Running tests/mariadb-crash-recovery-e2e.sh...")
        res = subprocess.run(["bash", str(ROOT / "tests/mariadb-crash-recovery-e2e.sh")], env={"ZELYRA_BIN": str(bin_path), **dict(os.environ)}, capture_output=True, text=True)
        if res.returncode != 0:
            print("STDERR:\n", res.stderr)
            raise RuntimeError(f"mariadb-crash-recovery-e2e.sh failed with return code {res.returncode}")
        print("     [OK] Server-crash recovery E2E passed (DDL/journal boundary recovery, clean replay)")
    finally:
        subprocess.run(["docker", "rm", "-f", container_name], capture_output=True)


def run_part4_release_artifacts_and_provenance(bin_path):
    log_step("Part 4: Release Artifacts, Metadata, SHA-256 Sidecars, and Archive Safety")
    env = {
        **dict(os.environ),
        "ZELYRA_RELEASE_BINARY": str(bin_path),
    }
    print("  -> Running tests/release-artifact-smoke.sh...")
    res = subprocess.run(["bash", str(ROOT / "tests/release-artifact-smoke.sh")], env=env, capture_output=True, text=True)
    if res.returncode != 0:
        print("STDERR:\n", res.stderr)
        raise RuntimeError(f"release-artifact-smoke.sh failed with return code {res.returncode}")
    print("     [OK] Release artifact smoke test passed (metadata check, deterministic packaging, SHA-256 sidecars, archive path safety, --version validation)")


def main():
    parser = argparse.ArgumentParser(description="Section 4.D Installation, Upgrade, Rollback & Artifacts Verification")
    parser.add_argument("--binary", default=str(BIN), help="Path to zelyra binary")
    args = parser.parse_args()

    bin_path = Path(args.binary).resolve()
    if not bin_path.exists():
        sys.exit(f"Binary not found: {bin_path}")

    print(f"Starting Section 4.D Verification with binary: {bin_path}")
    start_time = time.time()

    run_part1_installer_and_clean_onboarding(bin_path)
    run_part2_invoice_tutorial(bin_path)
    run_part3_backup_restore_and_crash_recovery(bin_path)
    run_part4_release_artifacts_and_provenance(bin_path)

    elapsed = time.time() - start_time
    print(f"\n{'='*70}\n[SUCCESS] Section 4.D Verification fully passed in {elapsed:.2f}s\n{'='*70}")


if __name__ == "__main__":
    main()
