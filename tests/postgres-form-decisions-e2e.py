#!/usr/bin/env python3
"""Native PostgreSQL Form Decisions E2E test suite.

Verifies transactional atomicity across tenant checks, post-only recipient checks,
optimistic edit snapshots (NULL vs. literal \\N), mutation statements, foreign-key
isolation, and audit writes on a single connection.

Tests against a disposable postgres:16.15 container (or DATABASE_URL if provided).
"""
import hashlib
import http.client
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import tempfile
import time
from urllib.parse import parse_qs, unquote, urlencode, urlsplit
import uuid

ROOT = Path(__file__).resolve().parents[1]
BIN = Path(os.environ.get("ZELYRA_BIN", ROOT / "target/debug/zelyra")).resolve()


class DisposablePostgres:
    def __init__(self):
        self.container = None
        self.port = None
        self.url = None

    def start(self):
        raw_url = os.environ.get("DATABASE_URL")
        if raw_url and urlsplit(raw_url).scheme in ("postgres", "postgresql"):
            self.url = raw_url
            return self

        name = f"zelyra-pg-form-decisions-{os.getpid()}"
        cmd = [
            "docker", "run", "-d", "--name", name, "-p", "127.0.0.1::5432",
            "--health-cmd=pg_isready -h 127.0.0.1 -U exact_test -d zelyra_test",
            "--health-interval=1s", "--health-retries=30",
            "-e", "POSTGRES_USER=exact_test",
            "-e", "POSTGRES_PASSWORD=exact_test",
            "-e", "POSTGRES_DB=zelyra_test",
            "postgres:16.15",
        ]
        subprocess.run(cmd, check=True, capture_output=True)
        self.container = name

        deadline = time.monotonic() + 30
        while True:
            info = json.loads(subprocess.check_output(["docker", "inspect", name]))[0]
            if info["State"]["Health"]["Status"] == "healthy":
                break
            if time.monotonic() > deadline:
                raise RuntimeError("PostgreSQL test container failed to become healthy")
            time.sleep(0.5)

        self.port = info["NetworkSettings"]["Ports"]["5432/tcp"][0]["HostPort"]
        self.url = f"postgres://exact_test:exact_test@127.0.0.1:{self.port}/zelyra_test"
        return self

    def stop(self):
        if self.container:
            subprocess.run(["docker", "rm", "-f", self.container], capture_output=True)
            self.container = None


class Inputs(__import__('html.parser', fromlist=['HTMLParser']).HTMLParser):
    def __init__(self, text):
        super().__init__()
        self.fields = {}
        self.options = {}
        self.current_select = None
        self.feed(text)

    def handle_starttag(self, tag, attributes):
        attrs = dict(attributes)
        if tag == "input" and "name" in attrs:
            self.fields[attrs["name"]] = attrs.get("value", "")
        elif tag == "select" and "name" in attrs:
            self.current_select = attrs["name"]
            self.options[self.current_select] = []
        elif tag == "option" and self.current_select:
            val = attrs.get("value", "")
            self.options[self.current_select].append(val)
            if "selected" in attrs or self.current_select not in self.fields:
                self.fields[self.current_select] = val

    def handle_endtag(self, tag):
        if tag == "select":
            self.current_select = None


def run_sql(db_url, statement):
    parts = urlsplit(db_url)
    user = unquote(parts.username or "exact_test")
    password = unquote(parts.password or "exact_test")
    env = os.environ.copy()
    env["PGPASSWORD"] = password
    command = [
        "psql", "-X", "-At",
        "-h", parts.hostname or "127.0.0.1",
        "-p", str(parts.port or 5432),
        "-U", user,
        "-d", parts.path.lstrip("/"),
        "-c", statement,
    ]
    res = subprocess.run(command, env=env, capture_output=True, text=True, timeout=30)
    if res.returncode != 0:
        raise AssertionError("psql failed: " + res.stderr[:500])
    return res.stdout.strip()


def http_request(port, method, path, headers=None, body_fields=None):
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=10)
    h = {"Origin": f"http://127.0.0.1:{port}"}
    if headers:
        h.update(headers)
    body = None
    if body_fields is not None:
        h["Content-Type"] = "application/x-www-form-urlencoded"
        body = urlencode(body_fields)
    conn.request(method, path, body, h)
    res = conn.getresponse()
    status = res.status
    resp_headers = dict(res.getheaders())
    resp_body = res.read().decode("utf-8", errors="replace")
    conn.close()
    return status, resp_headers, resp_body


def token_hash(token):
    return hashlib.blake2s(token.encode("utf-8")).hexdigest()


def test_postgres_form_decisions():
    pg = DisposablePostgres().start()
    base_parts = urlsplit(pg.url)
    db_name = f"zelyra_form_decisions_{uuid.uuid4().hex[:12]}"
    run_sql(pg.url, f"CREATE DATABASE {db_name}")
    db_url = base_parts._replace(path="/" + db_name).geturl()

    try:
        with tempfile.TemporaryDirectory(prefix="zelyra-pg-decisions-") as temp_dir:
            project_dir = Path(temp_dir)
            project_file = project_dir / "main.zyl"
            project_file.write_text(f"""
database Main {{
    engine: postgres
}}

auth users {{
    table: users
    sessions: auth_sessions
    memberships: tenant_memberships
    audit: audit_logs
}}

table users {{
    id: Id primary auto
    email: Email required unique
    password_hash: String(255) required
    active: Bool default true
}}

table auth_sessions {{
    id: Id primary auto
    user: User required
    token_hash: String(64) required unique
    expires_at: Timestamp required
}}

table tenant_memberships {{
    id: Id primary auto
    user_id: Int required
    tenant_id: Int required
    active: Bool required
}}

table audit_logs {{
    id: Id primary auto
    actor_user_id: Int?
    event: String(100) required
    target_user_id: Int?
    details: String(1000) required
    created_at: Timestamp default now
}}

table categories {{
    id: Id primary auto
    tenant_id: Int required
    name: String(100) required
}}

table orders {{
    id: Id primary auto
    tenant_id: Int required
    recipient_id: Int required
    category: Category required
    title: String(100) required
    amount: Int required
    status: String(50) required
    note: String(100)
}}

crud Category -> categories {{
    requires auth
    tenant: tenant_id
    list {{ id name }}
}}

crud Order -> orders {{
    requires auth
    tenant: tenant_id
    recipient: recipient_id
    list {{ id title amount status note }}

    action approve {{
        label: "Approve"
        confirm: "Approve order?"
        sql {{
            UPDATE orders SET status = 'approved' WHERE id = :id AND recipient_id = :zelyra_user_id
        }}
    }}
}}

fn main() {{ }}
""")
            # Apply schema
            subprocess.run([str(BIN), "check", str(project_file)], check=True, capture_output=True, env=dict(os.environ, DATABASE_URL=db_url))
            subprocess.run([str(BIN), "db", "apply", str(project_file)], check=True, capture_output=True, env=dict(os.environ, DATABASE_URL=db_url))

            # Seed data
            # User 10 (Tenant 1 member), User 20 (Tenant 2 member)
            run_sql(db_url, "INSERT INTO users (id, email, password_hash, active) VALUES (10, 'u10@example.com', 'hash', true), (20, 'u20@example.com', 'hash', true)")
            # Active memberships
            run_sql(db_url, "INSERT INTO tenant_memberships (user_id, tenant_id, active) VALUES (10, 1, true), (20, 2, true)")
            # Valid sessions
            t10_hash = token_hash("session-token-10")
            t20_hash = token_hash("session-token-20")
            run_sql(db_url, f"INSERT INTO auth_sessions (user_id, token_hash, expires_at) VALUES (10, '{t10_hash}', CURRENT_TIMESTAMP + INTERVAL '1 day'), (20, '{t20_hash}', CURRENT_TIMESTAMP + INTERVAL '1 day')")
            # Categories (Tenant 1: 1="T1 Hardware", Tenant 2: 2="T2 Cloud")
            run_sql(db_url, "INSERT INTO categories (id, tenant_id, name) VALUES (1, 1, 'T1 Hardware'), (2, 2, 'T2 Cloud')")
            # Order 1 in Tenant 1, assigned to recipient 10
            run_sql(db_url, "INSERT INTO orders (id, tenant_id, recipient_id, category_id, title, amount, status, note) VALUES (1, 1, 10, 1, 'Original Order', 100, 'pending', NULL)")

            # Start web server
            with socket.socket() as s:
                s.bind(("127.0.0.1", 0))
                port = s.getsockname()[1]

            log_path = project_dir / "server.log"
            server_env = dict(os.environ, DATABASE_URL=db_url)
            server = subprocess.Popen(
                [str(BIN), "serve", str(project_file), f"127.0.0.1:{port}"],
                stdout=log_path.open("w"),
                stderr=subprocess.STDOUT,
                env=server_env,
            )

            try:
                deadline = time.monotonic() + 15
                cookie_t1 = "zelyra_session=session-token-10"
                last_status = None
                while True:
                    try:
                        status, headers, body = http_request(port, "GET", "/orders/1/edit", {"Cookie": cookie_t1, "X-Tenant-Id": "1"})
                        last_status = status
                        if status == 200:
                            break
                    except (OSError, http.client.HTTPException):
                        pass
                    if server.poll() is not None or time.monotonic() > deadline:
                        raise AssertionError(f"Server failed to start or serve 200 (last status: {last_status}):\n" + log_path.read_text())
                    time.sleep(0.1)

                print("[1] Verifying Form Presentation and Cross-Tenant FK Option Isolation...")
                inputs = Inputs(body)
                assert "_zelyra_snapshot" in inputs.fields, "Snapshot must be generated"
                assert "_zelyra_csrf" in inputs.fields, "CSRF must be generated"
                assert inputs.fields.get("title") == "Original Order"
                # Check category select options: ONLY category 1 ('1') should be visible to Tenant 1
                cat_options = inputs.options.get("category", [])
                assert "1" in cat_options, f"Category 1 must be present in select options: {cat_options}"
                assert "2" not in cat_options, f"Category 2 belongs to Tenant 2 and must NOT appear in Tenant 1 form options: {cat_options}"

                print("[2] Verifying Successful Edit Mutation & Atomic Audit on PostgreSQL...")
                edit_fields = dict(inputs.fields, title="Updated Order 1", amount="120", category="1")
                status, headers, body = http_request(port, "POST", "/orders/1/edit", {"Cookie": cookie_t1, "X-Tenant-Id": "1"}, edit_fields)
                assert status == 303, f"Expected 303 redirect on successful edit, got {status}: {body}"
                # Verify DB update
                assert run_sql(db_url, "SELECT title FROM orders WHERE id=1") == "Updated Order 1"
                assert run_sql(db_url, "SELECT amount FROM orders WHERE id=1") == "120"
                # Verify atomic audit write
                audit_rows = run_sql(db_url, "SELECT event, target_user_id, details FROM audit_logs WHERE target_user_id=1")
                assert "crud.update" in audit_rows, f"Audit row must be recorded: {audit_rows}"
                assert "tenant_id=1" in audit_rows, f"Audit details must contain tenant_id=1: {audit_rows}"

                print("[3] Verifying Cross-Tenant FK Tampering Rejection (422 Unprocessable Entity)...")
                # Reload form to get fresh snapshot
                status, headers, body = http_request(port, "GET", "/orders/1/edit", {"Cookie": cookie_t1, "X-Tenant-Id": "1"})
                assert status == 200
                fresh_inputs = Inputs(body).fields
                # Try to tamper category to 2 (belongs to Tenant 2)
                tampered = dict(fresh_inputs, category="2", title="Tampered FK")
                status, headers, body = http_request(port, "POST", "/orders/1/edit", {"Cookie": cookie_t1, "X-Tenant-Id": "1"}, tampered)
                assert status == 422, f"Tampered foreign key from other tenant must return 422, got {status}: {body}"
                assert "selected value does not exist" in body
                assert run_sql(db_url, "SELECT title FROM orders WHERE id=1") == "Updated Order 1", "Title must not change on 422"

                print("[4] Verifying Optimistic Edit Snapshots (NULL <-> literal \\N) on PostgreSQL...")
                # Note is currently NULL
                assert run_sql(db_url, "SELECT note IS NULL FROM orders WHERE id=1") == "t"
                status, headers, body = http_request(port, "GET", "/orders/1/edit", {"Cookie": cookie_t1, "X-Tenant-Id": "1"})
                snapshot_null = Inputs(body).fields
                # Concurrently update note in DB to literal '\N'
                run_sql(db_url, r"UPDATE orders SET note=E'\\N' WHERE id=1")
                # Submitting with snapshot_null must conflict with 409
                status, headers, body = http_request(port, "POST", "/orders/1/edit", {"Cookie": cookie_t1, "X-Tenant-Id": "1"}, dict(snapshot_null, title="Overwritten Stale"))
                assert status == 409, f"NULL->literal change must conflict with 409, got {status}"
                assert run_sql(db_url, "SELECT title FROM orders WHERE id=1") == "Updated Order 1", "Data must remain untouched on 409"

                # Reverse direction: form loaded with '\N'
                status, headers, body = http_request(port, "GET", "/orders/1/edit", {"Cookie": cookie_t1, "X-Tenant-Id": "1"})
                snapshot_literal = Inputs(body).fields
                assert snapshot_literal.get("note") == "\\N", "Literal sentinel must be visible in inputs"
                # Concurrently set note back to NULL in DB
                run_sql(db_url, "UPDATE orders SET note=NULL WHERE id=1")
                # Submitting with snapshot_literal must conflict with 409
                status, headers, body = http_request(port, "POST", "/orders/1/edit", {"Cookie": cookie_t1, "X-Tenant-Id": "1"}, dict(snapshot_literal, title="Overwritten Reverse"))
                assert status == 409, f"Literal->NULL change must conflict with 409, got {status}"
                assert run_sql(db_url, "SELECT title FROM orders WHERE id=1") == "Updated Order 1"

                print("[5] Verifying Two-Connection Race on Membership Revocation...")
                # We test that if membership is revoked while the request is blocked waiting on lock,
                # the decision transaction aborts atomically.
                status, headers, body = http_request(port, "GET", "/orders/1/edit", {"Cookie": cookie_t1, "X-Tenant-Id": "1"})
                valid_edit_inputs = Inputs(body).fields

                # Connection 1 acquires FOR UPDATE lock on tenant_memberships for user 10 and sleeps for 2 seconds
                parts = urlsplit(db_url)
                p1_cmd = [
                    "psql", "-X", "-h", parts.hostname or "127.0.0.1", "-p", str(parts.port or 5432),
                    "-U", unquote(parts.username or "exact_test"), "-d", parts.path.lstrip("/"),
                    "-c", "BEGIN; SELECT * FROM tenant_memberships WHERE user_id=10 FOR UPDATE; SELECT pg_sleep(2); UPDATE tenant_memberships SET active=false WHERE user_id=10; COMMIT;",
                ]
                env = os.environ.copy()
                env["PGPASSWORD"] = unquote(parts.password or "exact_test")
                p1 = subprocess.Popen(p1_cmd, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                time.sleep(0.3)  # Ensure p1 holds the lock

                # Send POST while p1 holds membership lock
                race_fields = dict(valid_edit_inputs, title="Raced Title While Revoked")
                status, headers, body = http_request(port, "POST", "/orders/1/edit", {"Cookie": cookie_t1, "X-Tenant-Id": "1"}, race_fields)
                p1.wait(timeout=5)

                # After p1 committed active=false, the server query for active membership returned 0 rows -> rejected
                assert status in (403, 404, 500), f"Expected rejection when membership revoked during lock wait, got {status}: {body}"
                assert run_sql(db_url, "SELECT title FROM orders WHERE id=1") == "Updated Order 1", "Title must NOT have been updated when membership was revoked"
                # Restore membership for subsequent tests
                run_sql(db_url, "UPDATE tenant_memberships SET active=true WHERE user_id=10")

                print("[6] Verifying Atomic Rollback on Audit Failure...")
                # Install a trigger on audit_logs that rejects insert if event == 'crud.action.approve'
                run_sql(db_url, """
CREATE OR REPLACE FUNCTION fail_audit_insert() RETURNS trigger AS $$
BEGIN
    IF NEW.event LIKE 'crud.action.approve%' THEN
        RAISE EXCEPTION 'Simulated Audit Storage Failure';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER trg_audit_fail BEFORE INSERT ON audit_logs FOR EACH ROW EXECUTE FUNCTION fail_audit_insert();
""")
                # Order 1 status is currently 'pending'
                assert run_sql(db_url, "SELECT status FROM orders WHERE id=1") == "pending"

                # Send approve action request (which updates orders SET status='approved' then inserts audit)
                approve_form_status, _, approve_page = http_request(port, "GET", "/orders/1", {"Cookie": cookie_t1, "X-Tenant-Id": "1"})
                approve_inputs = Inputs(approve_page).fields
                approve_csrf = approve_inputs.get("_zelyra_csrf", "")

                status, headers, body = http_request(
                    port, "POST", "/orders/1/approve",
                    {"Cookie": cookie_t1, "X-Tenant-Id": "1"},
                    {"_zelyra_csrf": approve_csrf}
                )
                assert status == 500, f"Expected 500 on audit trigger failure, got {status}: {body}"
                # CRUCIAL: The status of orders MUST STILL BE 'pending', rolled back atomically!
                current_status = run_sql(db_url, "SELECT status FROM orders WHERE id=1")
                assert current_status == "pending", f"Business mutation MUST be rolled back atomically when audit fails! Current: {current_status}"

                # Remove the failing trigger
                run_sql(db_url, "DROP TRIGGER trg_audit_fail ON audit_logs; DROP FUNCTION fail_audit_insert();")

                print("[7] Verifying Successful Action & Single-Connection Reuse After Rollback...")
                # Now that trigger is removed, connection in pool must be clean and operational
                status, headers, body = http_request(
                    port, "POST", "/orders/1/approve",
                    {"Cookie": cookie_t1, "X-Tenant-Id": "1"},
                    {"_zelyra_csrf": approve_csrf}
                )
                assert status == 303, f"Expected 303 after trigger removal on action retry, got {status}: {body}"
                assert run_sql(db_url, "SELECT status FROM orders WHERE id=1") == "approved", "Status must now be approved"
                approve_audit = run_sql(db_url, "SELECT event FROM audit_logs WHERE target_user_id=1 AND event LIKE 'crud.action.approve%'")
                assert "crud.action.approve" in approve_audit, f"Action audit entry must be recorded: {approve_audit}"

                print("[8] Verifying Post-Only Recipient Guard Reassignment...")
                # Order 1 is approved. Now reassign recipient to user 20
                run_sql(db_url, "UPDATE orders SET recipient_id=20 WHERE id=1")
                # User 10 tries to approve order 1 again
                status, headers, body = http_request(
                    port, "POST", "/orders/1/approve",
                    {"Cookie": cookie_t1, "X-Tenant-Id": "1"},
                    {"_zelyra_csrf": approve_csrf}
                )
                # Must be rejected because recipient does not match user 10
                assert status in (403, 404), f"Reassigned recipient must be denied, got {status}"

                print("[9] Verifying Unauthorized Tenant Selection and Missing Membership...")
                # User 10 specifies X-Tenant-Id: 2 (where they have no membership)
                status, headers, body = http_request(port, "GET", "/orders", {"Cookie": cookie_t1, "X-Tenant-Id": "2"})
                assert status == 403, f"Cross-tenant request without membership must return 403, got {status}"

                # Request without any session cookie
                status, headers, body = http_request(port, "GET", "/orders")
                assert status in (401, 403), f"Unauthenticated request must be denied, got {status}"

                print("\nALL 9 POSTGRESQL FORM DECISION E2E TESTS PASSED!")
                return True
            finally:
                server.terminate()
                try:
                    server.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    server.kill()
                    server.wait(timeout=5)
    finally:
        # Drop test database
        try:
            run_sql(pg.url, f"SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{db_name}' AND pid <> pg_backend_pid()")
            run_sql(pg.url, f"DROP DATABASE IF EXISTS {db_name}")
        except Exception as e:
            print("Cleanup warning:", e)
        pg.stop()


if __name__ == "__main__":
    test_postgres_form_decisions()
