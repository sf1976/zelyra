#!/usr/bin/env python3
"""Comprehensive E2E test suite for Mail, Notifications, MFA, and Security (Section 4.C).

Verifies Section 4.C of technical release gates:
1. SMTP Contract & Resilient Delivery:
   - Loopback SMTP sink (tests/smtp_capture.py) with STARTTLS, TLS certificate validation.
   - Rejection of untrusted CA without plaintext downgrade, scheduling retry.
   - Successful delivery with trusted CA, MIME multipart/alternative verification.
   - Unique stable Message-ID header (event_id@zelyra.local).
   - Greeting timeout resiliency (retry scheduled, zero crashes).
   - Secret redaction: secrets and credentials never logged or leaked.
   - Persistent password reset limits across restarts, rate limit quotas, batch cleanup.
2. Tenant & Recipient Notifications:
   - examples/notification_center.zyl schema, multi-user, multi-tenant setup.
   - In-app notification feed, unread count calculation.
   - Filtering of expired and archived notices.
   - Strict recipient isolation within the same tenant (User 1 vs User 2: 404 on cross-access).
   - Strict tenant isolation (Tenant 101 vs Tenant 202: 404 on cross-access).
   - CSRF-protected mark-as-read action: unread decrement, audit log record.
   - Tamper protection: 404 on foreign mark-as-read, 403 on bad CSRF, 404 on expired/archived.
   - Compiler safety: recipient column never exposed as field/filter, unscoped SQL blocked.
   - Honest scope: no untried file attachments or named templates claimed.
3. Auth, Sessions, CSRF, MFA & Security Boundaries:
   - Local TOTP MFA (RFC 6238), AES-256-GCM secret encryption, Argon2id recovery codes.
   - Session gating: unverified MFA session blocked with 401 on guarded pages/APIs.
   - Replay protection: used TOTP steps and recovery codes rejected (401).
   - Concurrent race protection: exactly one concurrent request consumes a TOTP/recovery code.
   - Lockout threshold: 5 failed attempts trigger 15-minute lock (429).
   - Operator recovery rehearsal: deleting factor and sessions preserves audit log.
   - Honest backend boundaries: MariaDB (full auth/MFA/notifications), MySQL & PostgreSQL
     boundaries clearly documented.
   - Security review findings documented with responsible owners and open blockers.

Zero disruption to production databases; uses disposable containers on ephemeral ports.
"""
import argparse
import base64
from concurrent.futures import ThreadPoolExecutor
import hashlib
import hmac
import http.client
import json
import os
from pathlib import Path
import re
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time
from urllib.parse import parse_qs, unquote, urlencode, urlsplit

ROOT = Path(__file__).resolve().parents[1]
BIN = Path(os.environ.get("ZELYRA_BIN", ROOT / "target/debug/zelyra")).resolve()


class DisposableMariaDB:
    def __init__(self, tag="11.4.13"):
        self.tag = tag
        self.container = None
        self.port = None

    def __enter__(self):
        pid = os.getpid()
        ts = int(time.time() * 1000) % 1000000
        name = f"zelyra-test-sec-mariadb-{pid}-{ts}"
        cmd = [
            "docker", "run", "-d", "--name", name, "-p", "127.0.0.1::3306",
            "-e", "MARIADB_ROOT_PASSWORD=test_pass",
            "-e", "MARIADB_DATABASE=zelyra_test",
            "--health-cmd=mariadb-admin ping -h 127.0.0.1 -u root -ptest_pass",
            "--health-interval=1s", "--health-retries=30",
            f"mariadb:{self.tag}",
        ]
        subprocess.run(cmd, check=True, capture_output=True)
        self.container = name

        deadline = time.monotonic() + 45
        while True:
            info = json.loads(subprocess.check_output(["docker", "inspect", name]))[0]
            status = info.get("State", {}).get("Health", {}).get("Status")
            if status == "healthy":
                break
            if time.monotonic() > deadline:
                raise RuntimeError(f"MariaDB test container failed to become healthy")
            time.sleep(0.5)

        self.port = int(info["NetworkSettings"]["Ports"]["3306/tcp"][0]["HostPort"])
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        if self.container:
            subprocess.run(["docker", "rm", "-f", self.container], capture_output=True)
            self.container = None

    @property
    def url(self):
        return f"mariadb://root:test_pass@127.0.0.1:{self.port}/zelyra_test"


def pick_ephemeral_port():
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
    s.close()
    return port


def log_step(name):
    print(f"\n{'='*70}\n[STEP] {name}\n{'='*70}")


def run_part1_smtp_outbox_and_limits(bin_path, db_url):
    log_step("Part 1: SMTP Contract, Outbox Resiliency, TLS, and Password Reset Rate Limits")
    env = {**dict(os.environ), "DATABASE_URL": db_url, "ZELYRA_BIN": str(bin_path)}

    # 1.1 Mail outbox E2E
    print("  -> Running mail-outbox-e2e.sh...")
    res = subprocess.run(
        ["bash", str(ROOT / "tests/mail-outbox-e2e.sh")],
        env=env,
        capture_output=True,
        text=True
    )
    if res.returncode != 0:
        print("STDERR:\n", res.stderr)
        raise RuntimeError(f"mail-outbox-e2e.sh failed with return code {res.returncode}")
    print("     [OK] Mail outbox E2E passed (TLS, STARTTLS, untrusted CA rejection, greeting timeout, MIME multipart)")

    # 1.2 Password reset rate limits E2E
    print("  -> Running password-reset-rate-limit-e2e.sh...")
    res = subprocess.run(
        ["bash", str(ROOT / "tests/password-reset-rate-limit-e2e.sh")],
        env=env,
        capture_output=True,
        text=True
    )
    if res.returncode != 0:
        print("STDERR:\n", res.stderr)
        raise RuntimeError(f"password-reset-rate-limit-e2e.sh failed with return code {res.returncode}")
    print("     [OK] Password reset persistent rate limits E2E passed (restart survival, shared capacity, batch cleanup)")


def run_part2_notifications(bin_path, db_url):
    log_step("Part 2: Tenant & Recipient Notifications (Feed, Counter, CSRF Mark-Read, Isolation)")
    app_port = pick_ephemeral_port()
    env = {
        **dict(os.environ),
        "DATABASE_URL": db_url,
        "ZELYRA_BIN": str(bin_path),
        "ZELYRA_NOTIFICATION_E2E_ADDRESS": f"127.0.0.1:{app_port}",
    }

    print("  -> Running mariadb-notification-center-e2e.sh...")
    res = subprocess.run(
        ["bash", str(ROOT / "tests/mariadb-notification-center-e2e.sh")],
        env=env,
        capture_output=True,
        text=True
    )
    if res.returncode != 0:
        print("STDERR:\n", res.stderr)
        raise RuntimeError(f"mariadb-notification-center-e2e.sh failed with return code {res.returncode}")
    print("     [OK] Notification center E2E passed (unread count, tenant isolation, recipient isolation, CSRF mark-read, audit log)")

    # 2.2 Compiler checks for recipient-scoped CRUD
    print("  -> Verifying recipient safety compiler rules...")
    invalid_crud_source = """
        auth users { table: users sessions: auth_sessions permissions: user_permissions memberships: tenant_memberships audit: auth_audit_log }
        table users { id: Id primary auto email: Email required unique password_hash: String(255) required }
        table auth_sessions { id: Id primary auto user: User required token_hash: String(64) required unique expires_at: Timestamp required }
        table user_permissions { id: Id primary auto user_id: Id required permission: String(100) required }
        table tenant_memberships { id: Id primary auto user_id: Id required tenant_id: Id required active: Bool required }
        table auth_audit_log { id: Id primary auto actor_user_id: Id? event: String(100) required target_user_id: Id? details: String(1000) required created_at: Timestamp default now }

        table notifications {
            id: Id primary auto
            tenant_id: Id required
            user_id: Id required
            title: String(160) required
        }

        crud Notification -> notifications {
            tenant: tenant_id
            recipient: user_id
            requires auth
            list {
                title
                user_id
            }
        }
        fn main() {}
    """
    with tempfile.TemporaryDirectory() as td:
        bad_file = Path(td) / "bad.zyl"
        bad_file.write_text(invalid_crud_source, encoding="utf-8")
        chk = subprocess.run([str(bin_path), "check", str(bad_file)], capture_output=True, text=True)
        assert chk.returncode != 0, "Compiler must reject exposing recipient column in CRUD list"
        assert "recipient user ID must not be exposed" in (chk.stdout + chk.stderr), "Missing expected error diagnostic"
    print("     [OK] Compiler safety: exposing recipient column in CRUD is rejected")


def run_part3_auth_mfa_and_security(bin_path, db_url):
    log_step("Part 3: Auth, Sessions, CSRF, MFA, and Security Boundaries")
    env = {
        **dict(os.environ),
        "DATABASE_URL": db_url,
        "ZELYRA_BIN": str(bin_path),
        "ZELYRA_MFA_E2E_DB_PASSWORD": "test_pass",
    }

    print("  -> Running mariadb-mfa-e2e.sh...")
    res = subprocess.run(
        ["bash", str(ROOT / "tests/mariadb-mfa-e2e.sh")],
        env=env,
        capture_output=True,
        text=True
    )
    if res.returncode != 0:
        print("STDERR:\n", res.stderr)
        raise RuntimeError(f"mariadb-mfa-e2e.sh failed with return code {res.returncode}")
    print("     [OK] Local TOTP MFA E2E passed (AES-256-GCM, Argon2id recovery codes, replay prevention, 5-failure lock, operator rehearsal)")

    # 3.2 Basic Auth E2E
    auth_port = pick_ephemeral_port()
    env_auth = {
        **env,
        "ZELYRA_AUTH_DB_PASSWORD": "test_pass",
        "ZELYRA_AUTH_E2E_ADDRESS": f"127.0.0.1:{auth_port}",
    }
    print("  -> Running mariadb-auth-e2e.sh...")
    res = subprocess.run(
        ["bash", str(ROOT / "tests/mariadb-auth-e2e.sh")],
        env=env_auth,
        capture_output=True,
        text=True
    )
    if res.returncode != 0:
        print("STDERR:\n", res.stderr)
        raise RuntimeError(f"mariadb-auth-e2e.sh failed with return code {res.returncode}")
    print("     [OK] MariaDB authentication E2E passed (sessions, Argon2id, permissions, logout)")


def main():
    parser = argparse.ArgumentParser(description="Mail, Notifications, MFA, and Security E2E Verification")
    parser.add_argument("--binary", default=str(BIN), help="Path to zelyra binary")
    args = parser.parse_args()

    bin_path = Path(args.binary).resolve()
    if not bin_path.exists():
        sys.exit(f"Binary not found: {bin_path}")

    print(f"Starting Section 4.C Verification with binary: {bin_path}")
    start_time = time.time()

    with DisposableMariaDB(tag="11.4.13") as db:
        print(f"Disposable MariaDB 11.4.13 running at {db.url}")
        run_part1_smtp_outbox_and_limits(bin_path, db.url)
        run_part2_notifications(bin_path, db.url)
        run_part3_auth_mfa_and_security(bin_path, db.url)

    elapsed = time.time() - start_time
    print(f"\n{'='*70}\n[SUCCESS] Section 4.C Verification fully passed in {elapsed:.2f}s\n{'='*70}")


if __name__ == "__main__":
    main()
