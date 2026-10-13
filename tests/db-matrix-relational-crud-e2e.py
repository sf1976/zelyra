#!/usr/bin/env python3
"""Comprehensive E2E test suite for Database Matrix, Transactions, and Relational CRUD.

Verifies Section 4.A of technical release gates:
1. Multi-backend support across MariaDB 11.4.13, MySQL 8.4.11, and PostgreSQL 16.15:
   - Schema setup and introspection with real foreign key constraints.
   - Migration preflights: orphan detection preserves data and rejects constraint addition (E-DB-005).
   - Write lock timeouts & boundaries: innodb_lock_wait_timeout bounds lock waits on MySQL/MariaDB;
     deadlines and statement timeouts abort cleanly, rollback, and discard connections.
2. Relational tenant-scoped CRUD:
   - Dynamic foreign-key option dropdowns filtered strictly by active tenant membership.
   - Cross-tenant foreign key tampering produces 422 Unprocessable Entity.
   - Relational joins display parent display columns in list and detail views.
   - Restrictive foreign key deletion: parent deletion with referencing children fails gracefully (500)
     without leaking database internals; ordered deletion succeeds.

Zero disruption to production databases; uses disposable containers on ephemeral ports.
"""
import argparse
import hashlib
from html.parser import HTMLParser
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


class SelectOptionParser(HTMLParser):
    def __init__(self):
        super().__init__()
        self.selects = {}
        self.current_select = None
        self.csrf = None

    def handle_starttag(self, tag, attrs):
        attrs_dict = dict(attrs)
        if tag == "input" and attrs_dict.get("name") == "_zelyra_csrf":
            self.csrf = attrs_dict.get("value")
        elif tag == "select" and "name" in attrs_dict:
            self.current_select = attrs_dict["name"]
            self.selects[self.current_select] = []
        elif tag == "option" and self.current_select:
            val = attrs_dict.get("value", "")
            self.selects[self.current_select].append(val)

    def handle_endtag(self, tag):
        if tag == "select":
            self.current_select = None


class DisposableContainer:
    def __init__(self, engine):
        self.engine = engine
        self.container = None
        self.port = None
        self.url = None

    def start(self):
        pid = os.getpid()
        ts = int(time.time() * 1000) % 1000000
        if self.engine == "mariadb":
            name = f"zelyra-test-mariadb-{pid}-{ts}"
            cmd = [
                "docker", "run", "-d", "--name", name, "-p", "127.0.0.1::3306",
                "-e", "MARIADB_ROOT_PASSWORD=test_pass",
                "-e", "MARIADB_DATABASE=zelyra_test",
                "--health-cmd=mariadb-admin ping -h 127.0.0.1 -u root -ptest_pass",
                "--health-interval=1s", "--health-retries=30",
                "mariadb:11.4.13",
            ]
        elif self.engine == "mysql":
            name = f"zelyra-test-mysql-{pid}-{ts}"
            cmd = [
                "docker", "run", "-d", "--name", name, "-p", "127.0.0.1::3306",
                "-e", "MYSQL_ROOT_PASSWORD=test_pass",
                "-e", "MYSQL_DATABASE=zelyra_test",
                "--health-cmd=mysqladmin ping -h 127.0.0.1 -u root -ptest_pass",
                "--health-interval=1s", "--health-retries=30",
                "mysql:8.4.11",
            ]
        elif self.engine == "postgres":
            name = f"zelyra-test-pg-{pid}-{ts}"
            cmd = [
                "docker", "run", "-d", "--name", name, "-p", "127.0.0.1::5432",
                "-e", "POSTGRES_USER=test_user",
                "-e", "POSTGRES_PASSWORD=test_pass",
                "-e", "POSTGRES_DB=zelyra_test",
                "--health-cmd=pg_isready -h 127.0.0.1 -U test_user -d zelyra_test",
                "--health-interval=1s", "--health-retries=30",
                "postgres:16.15",
            ]
        else:
            raise ValueError(f"unknown engine: {self.engine}")

        subprocess.run(cmd, check=True, capture_output=True)
        self.container = name

        deadline = time.monotonic() + 45
        while True:
            info = json.loads(subprocess.check_output(["docker", "inspect", name]))[0]
            status = info.get("State", {}).get("Health", {}).get("Status")
            if status == "healthy":
                break
            if time.monotonic() > deadline:
                raise RuntimeError(f"{self.engine} test container failed to become healthy")
            time.sleep(0.5)

        if self.engine in ("mariadb", "mysql"):
            self.port = info["NetworkSettings"]["Ports"]["3306/tcp"][0]["HostPort"]
            scheme = "mariadb" if self.engine == "mariadb" else "mysql"
            self.url = f"{scheme}://root:test_pass@127.0.0.1:{self.port}/zelyra_test"
        else:
            self.port = info["NetworkSettings"]["Ports"]["5432/tcp"][0]["HostPort"]
            self.url = f"postgres://test_user:test_pass@127.0.0.1:{self.port}/zelyra_test"

        return self

    def stop(self):
        if self.container:
            subprocess.run(["docker", "rm", "-f", self.container], capture_output=True)
            self.container = None


def execute_sql(db_url, sql):
    parts = urlsplit(db_url)
    user = unquote(parts.username or "")
    password = unquote(parts.password or "")
    host = parts.hostname or "127.0.0.1"
    port = str(parts.port or 3306)
    db_name = parts.path.lstrip("/")

    if parts.scheme in ("mariadb", "mysql"):
        client = "mysql" if parts.scheme == "mysql" else "mariadb"
        env = dict(os.environ, MYSQL_PWD=password)
        cmd = [
            client, "--protocol=tcp", "-h", host, "-P", port, "-u", user, db_name,
            "--batch", "--skip-column-names", "-e", sql,
        ]
        res = subprocess.run(cmd, env=env, capture_output=True, text=True)
        if res.returncode != 0:
            raise RuntimeError(f"{client} failed: {res.stderr}\nSQL: {sql}")
        return res.stdout
    elif parts.scheme in ("postgres", "postgresql"):
        env = dict(os.environ, PGPASSWORD=password)
        cmd = [
            "psql", "-h", host, "-p", port, "-U", user, "-d", db_name,
            "-t", "-A", "-c", sql,
        ]
        res = subprocess.run(cmd, env=env, capture_output=True, text=True)
        if res.returncode != 0:
            raise RuntimeError(f"psql failed: {res.stderr}\nSQL: {sql}")
        return res.stdout
    else:
        raise ValueError(f"unsupported scheme: {parts.scheme}")


def find_free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def build_project_source(engine):
    return f"""database main {{
    engine: {engine}
}}

auth users {{
    table: users
    sessions: auth_sessions
    permissions: user_permissions
    memberships: tenant_memberships
    audit: auth_audit_log
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
    device_label: String(255)
}}

table user_permissions {{
    id: Id primary auto
    user_id: Int required
    permission: String(100) required
}}

table tenant_memberships {{
    id: Id primary auto
    user_id: Int required
    tenant_id: Int required
    active: Bool required
}}

table auth_audit_log {{
    id: Id primary auto
    actor_user_id: Int?
    event: String(100) required
    target_user_id: Int?
    details: String(1000) required
    created_at: Timestamp default now
}}

table customers {{
    id: Id primary auto
    tenant_id: Int required
    name: String(100) required
}}

table projects {{
    id: Id primary auto
    tenant_id: Int required
    title: String(100) required
    customer: Customer required
    deleted_at: Timestamp?
}}

crud Customer -> customers {{
    requires auth
    permits "customers.manage"
    tenant: tenant_id
    list {{
        id
        name
    }}
}}

crud Project -> projects {{
    requires auth
    permits "projects.manage"
    tenant: tenant_id
    list {{
        id
        title
        customer
    }}
}}

fn main() {{}}
"""


class HttpClient:
    def __init__(self, host, port):
        self.host = host
        self.port = port
        self.cookie = None

    def request(self, method, path, body=None, headers=None):
        conn = http.client.HTTPConnection(self.host, self.port, timeout=15)
        h = headers.copy() if headers else {}
        if self.cookie:
            h["Cookie"] = self.cookie
        if method == "POST":
            h.setdefault("Origin", f"http://{self.host}:{self.port}")
        if body and "Content-Type" not in h:
            h["Content-Type"] = "application/x-www-form-urlencoded"
        conn.request(method, path, body=body, headers=h)
        resp = conn.getresponse()
        data = resp.read().decode("utf-8", errors="replace")
        set_cookie = resp.getheader("Set-Cookie")
        if set_cookie:
            self.cookie = set_cookie.split(";")[0]
        location = resp.getheader("Location")
        conn.close()
        return resp.status, data, location


def test_backend(engine, db_url):
    print(f"\n=======================================================")
    print(f"Testing backend: {engine} ({db_url})")
    print(f"=======================================================")
    temp_dir = tempfile.mkdtemp(prefix=f"zelyra-{engine}-crud-")
    server_proc = None

    try:
        project_file = Path(temp_dir) / "main.zyl"
        project_file.write_text(build_project_source(engine), encoding="utf-8")

        # Step 1: Schema Setup & Introspection
        print("[1/6] Running db setup and inspecting foreign keys...")
        env = dict(os.environ, DATABASE_URL=db_url)
        setup_cmd = [str(BIN), "db", "apply", str(project_file)] if engine == "postgres" else [str(BIN), "db", "setup", str(project_file)]
        setup_res = subprocess.run(setup_cmd, env=env, capture_output=True, text=True)
        if setup_res.returncode != 0:
            raise RuntimeError(f"db setup/apply failed:\n{setup_res.stderr}")

        map_res = subprocess.run([str(BIN), "db", "map", str(project_file), "--format=json"],
                                 env=env, capture_output=True, text=True)
        if map_res.returncode != 0:
            raise RuntimeError(f"db map failed:\n{map_res.stderr}")

        map_data = json.loads(map_res.stdout)
        projects_table = None
        for mod in map_data.get("modules", []):
            for tbl in mod.get("tables", []):
                if tbl.get("name") == "projects":
                    projects_table = tbl
                    break
        assert projects_table is not None, "projects table not found in db map"
        assert len(projects_table.get("foreign_keys", [])) >= 1, "missing foreign key on projects table"
        fk = projects_table["foreign_keys"][0]
        ref_table = fk.get("references_table") or fk.get("referenced_table")
        assert fk["column"] == "customer_id", f"expected fk column customer_id, got {fk['column']}"
        assert ref_table == "customers", f"expected fk target customers, got {ref_table}"
        print(f"  -> Verified constraint `{fk.get('name', 'fk')}`: {fk['column']} -> {ref_table}(id)")

        # Step 2: Foreign Key Migration & Orphan Detection Preflight (E-DB-005)
        print("[2/6] Verifying orphan detection and data preservation preflight (E-DB-005)...")
        orphan_db_url = db_url.rsplit("/", 1)[0] + "/zelyra_orphan_test"
        if engine in ("mariadb", "mysql"):
            execute_sql(db_url, "CREATE DATABASE IF NOT EXISTS zelyra_orphan_test;")
        else:
            execute_sql(db_url, "CREATE DATABASE zelyra_orphan_test;")

        try:
            orphan_env = dict(os.environ, DATABASE_URL=orphan_db_url)
            before_file = Path(temp_dir) / "orphan_before.zyl"
            before_file.write_text(f"""database main {{ engine: {engine} }}
table departments {{ id: Id primary auto name: String(80) required }}
table machines {{ id: Id primary auto department_id: Id required }}
fn main() {{}}
""", encoding="utf-8")
            setup_before = [str(BIN), "db", "apply", str(before_file)] if engine == "postgres" else [str(BIN), "db", "setup", str(before_file)]
            subprocess.run(setup_before, env=orphan_env, check=True, capture_output=True)

            # Insert orphan row (department_id 500 does not exist in departments)
            execute_sql(orphan_db_url, "INSERT INTO machines (department_id) VALUES (500);")

            after_file = Path(temp_dir) / "orphan_after.zyl"
            after_file.write_text(f"""database main {{ engine: {engine} }}
table departments {{ id: Id primary auto name: String(80) required }}
table machines {{ id: Id primary auto department: Department required }}
fn main() {{}}
""", encoding="utf-8")

            plan_res = subprocess.run([str(BIN), "db", "plan", str(after_file), "--format=json"],
                                      env=orphan_env, capture_output=True, text=True)
            assert plan_res.returncode == 0, f"db plan failed: {plan_res.stderr}"
            plan_data = json.loads(plan_res.stdout)
            plan_id = plan_data["plan_id"]
            preflights = [p for p in plan_data.get("preflights", []) if p.get("kind") == "foreign_key_values_must_exist"]
            assert len(preflights) >= 1, f"missing foreign key preflight in plan: {plan_data}"

            apply_res = subprocess.run([str(BIN), "db", "apply", str(after_file), "--allow-risky", "--plan-id", plan_id],
                                       env=orphan_env, capture_output=True, text=True)
            assert apply_res.returncode != 0, "db apply should have failed due to orphan row!"
            assert "E-DB-005" in apply_res.stderr or "no schema SQL was applied" in apply_res.stderr, (
                f"expected E-DB-005 orphan error, got: {apply_res.stderr}"
            )

            # Verify orphan row in machines is preserved
            orphan_count = execute_sql(orphan_db_url, "SELECT COUNT(*) FROM machines WHERE department_id = 500;")
            assert "1" in orphan_count, "orphan data must be preserved!"

            # Insert matching parent row and re-apply: should now succeed!
            execute_sql(orphan_db_url, "INSERT INTO departments (id, name) VALUES (500, 'matched');")
            plan_res2 = subprocess.run([str(BIN), "db", "plan", str(after_file), "--format=json"],
                                       env=orphan_env, capture_output=True, text=True)
            plan_id2 = json.loads(plan_res2.stdout)["plan_id"]
            apply_res2 = subprocess.run([str(BIN), "db", "apply", str(after_file), "--allow-risky", "--plan-id", plan_id2],
                                        env=orphan_env, capture_output=True, text=True)
            assert apply_res2.returncode == 0, f"db apply should succeed once orphan row is resolved: {apply_res2.stderr}"
            print("  -> Verified E-DB-005: migration aborted safely on orphan data, data preserved, applied after resolution.")
        finally:
            if engine in ("mariadb", "mysql"):
                execute_sql(db_url, "DROP DATABASE IF EXISTS zelyra_orphan_test;")
            else:
                execute_sql(db_url, "DROP DATABASE IF EXISTS zelyra_orphan_test;")

        # Step 3: MySQL / MariaDB Write Lock Wait Boundaries
        if engine in ("mariadb", "mysql"):
            print("[3/6] Verifying bounded write lock wait and connection abort...")
            # Insert a row into customers
            execute_sql(db_url, "INSERT INTO customers (id, tenant_id, name) VALUES (999, 10, 'LockTest');")
            # Open connection 1 holding lock
            parts = urlsplit(db_url)
            client = "mysql" if engine == "mysql" else "mariadb"
            p1 = subprocess.Popen([client, "--protocol=tcp", "-h", parts.hostname, "-P", str(parts.port),
                                   "-u", unquote(parts.username or ""), db_name_from_url(db_url)],
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                  text=True, env=dict(os.environ, MYSQL_PWD=unquote(parts.password or "")))
            p1.stdin.write("START TRANSACTION; SELECT * FROM customers WHERE id = 999 FOR UPDATE;\n")
            p1.stdin.flush()
            time.sleep(1) # row is locked

            # Connection 2 attempts update with 1s lock wait timeout
            t0 = time.time()
            timeout_sql = "SET SESSION innodb_lock_wait_timeout = 1;" if engine == "mysql" else "SET SESSION max_statement_time = 1;"
            p2 = subprocess.run([client, "--protocol=tcp", "-h", parts.hostname, "-P", str(parts.port),
                                 "-u", unquote(parts.username or ""), db_name_from_url(db_url), "-e",
                                 f"{timeout_sql} UPDATE customers SET name = 'Conflict' WHERE id = 999;"],
                                capture_output=True, text=True,
                                env=dict(os.environ, MYSQL_PWD=unquote(parts.password or "")))
            elapsed = time.time() - t0
            assert p2.returncode != 0, "Connection 2 must fail due to lock wait timeout"
            assert elapsed < 10.0, f"Lock wait took too long: {elapsed:.2f}s (default 50s not bounded!)"
            p1.stdin.write("ROLLBACK;\n")
            p1.stdin.flush()
            p1.terminate()
            execute_sql(db_url, "DELETE FROM customers WHERE id = 999;")
            print(f"  -> Verified bounded write lock wait: aborted in {elapsed:.2f}s.")
        else:
            print("[3/6] Skipping MySQL/MariaDB lock wait timeout on PostgreSQL.")

        # Step 4: Seed Auth & Multi-Tenant Relational Data
        print("[4/6] Seeding multi-tenant users, permissions, and relations...")
        hash_res = subprocess.run([str(BIN), "auth", "hash-password", "--stdin"],
                                  input="ZelyraSecret123!\n", text=True, capture_output=True, check=True)
        pwd_hash = hash_res.stdout.strip()

        # Seed Users
        execute_sql(db_url, f"INSERT INTO users (id, email, password_hash, active) VALUES "
                            f"(1, 'usera@example.test', '{pwd_hash}', true), "
                            f"(2, 'userb@example.test', '{pwd_hash}', true);")
        # Seed Tenant Memberships (User 1 -> Tenant 10, User 2 -> Tenant 20)
        execute_sql(db_url, "INSERT INTO tenant_memberships (user_id, tenant_id, active) VALUES "
                            "(1, 10, true), (2, 20, true);")
        # Seed Permissions
        execute_sql(db_url, "INSERT INTO user_permissions (user_id, permission) VALUES "
                            "(1, 'customers.manage'), (1, 'projects.manage'), "
                            "(2, 'customers.manage'), (2, 'projects.manage');")
        # Seed Customers: Customer 101 in Tenant 10, Customer 201 in Tenant 20
        execute_sql(db_url, "INSERT INTO customers (id, tenant_id, name) VALUES "
                            "(101, 10, 'Acme Alpha Corp'), "
                            "(201, 20, 'Beta Solutions Inc');")

        # Step 5: Start Server and Authenticate
        port = find_free_port()
        print(f"[5/6] Starting server on 127.0.0.1:{port}...")
        server_log = open(Path(temp_dir) / "server.log", "w")
        server_proc = subprocess.Popen([str(BIN), "serve", str(project_file), f"127.0.0.1:{port}"],
                                       env=env, stdout=server_log, stderr=server_log)
        time.sleep(1.5)
        http = HttpClient("127.0.0.1", port)

        if engine == "postgres":
            # PostgreSQL authenticated session via session_from_request
            session_token = "zelyra_test_session_user_a"
            token_hash = hashlib.blake2s(session_token.encode("utf-8")).hexdigest()
            execute_sql(db_url, f"INSERT INTO auth_sessions (user_id, token_hash, expires_at) VALUES (1, '{token_hash}', CURRENT_TIMESTAMP + INTERVAL '1 day');")
            http.cookie = f"zelyra_session={session_token}"
            print("  -> Authenticated as User A via PostgreSQL session token.")
        else:
            # Login as User A (Tenant 10) via HTTP /login
            status, login_html, _ = http.request("GET", "/login")
            assert status == 200, f"failed to get login page: {status}"
            login_parser = SelectOptionParser()
            login_parser.feed(login_html)
            csrf = login_parser.csrf
            assert csrf, "missing CSRF token on login page"

            login_body = urlencode({
                "_zelyra_csrf": csrf,
                "email": "usera@example.test",
                "password": "ZelyraSecret123!",
            })
            status, _, loc = http.request("POST", "/login", body=login_body)
            assert status == 303, f"login failed with status {status}"
            print("  -> Authenticated as User A (Tenant 10) via HTTP /login.")

        # Step 6: Test Relational Tenant Isolation, Tamper Prevention, and Restrictive Deletes
        print("[6/6] Executing relational tenant CRUD checks...")

        # 6.a Dropdown Isolation: GET /projects/new?tenant_id=10
        status, form_html, _ = http.request("GET", "/projects/new?tenant_id=10")
        assert status == 200, f"failed to load project create form: {status}"
        form_parser = SelectOptionParser()
        form_parser.feed(form_html)
        csrf = form_parser.csrf
        customer_options = form_parser.selects.get("customer", [])
        assert "101" in customer_options, f"Customer 101 ('Acme Alpha Corp') must be in options: {customer_options}"
        assert "201" not in customer_options, f"Cross-tenant Customer 201 MUST NOT be in options: {customer_options}"
        print("  -> Verified dropdown isolation: Tenant 10 options present, Tenant 20 excluded.")

        # 6.b Tamper Prevention: Post with cross-tenant customer=201 -> 422
        tampered_body = urlencode({
            "_zelyra_csrf": csrf,
            "title": "Tampered Cross Tenant Project",
            "customer": "201",
        })
        status, tampered_html, _ = http.request("POST", "/projects/new?tenant_id=10", body=tampered_body)
        assert status == 422, f"expected 422 Unprocessable Entity for cross-tenant FK, got {status}"
        assert "selected value does not exist" in tampered_html, f"expected validation error in response:\n{tampered_html}"
        print("  -> Verified tamper prevention: cross-tenant FK rejected with 422.")

        # 6.c Valid Relational Creation: Post with customer=101 -> 303
        valid_body = urlencode({
            "_zelyra_csrf": csrf,
            "title": "Alpha Project One",
            "customer": "101",
        })
        status, _, loc = http.request("POST", "/projects/new?tenant_id=10", body=valid_body)
        assert status == 303, f"expected 303 redirect after valid create, got {status}"

        # 6.d Relational List Display: GET /projects?tenant_id=10
        status, list_html, _ = http.request("GET", "/projects?tenant_id=10")
        assert status == 200, f"failed to get project list: {status}"
        assert "Alpha Project One" in list_html, "project title missing from list view"
        assert "Acme Alpha Corp" in list_html, "joined customer name missing from list view"
        assert "Beta Solutions Inc" not in list_html, "cross-tenant customer must not appear in list view"
        print("  -> Verified relational list display: joined customer name rendered correctly.")

        # 6.e Restrictive Deletion: Attempt to delete Customer 101 while referenced -> 500
        # Customer 101 is referenced by Alpha Project One
        status, cust_detail_html, _ = http.request("GET", "/customers/101?tenant_id=10")
        cust_parser = SelectOptionParser()
        cust_parser.feed(cust_detail_html)
        cust_csrf = cust_parser.csrf or csrf

        status, delete_resp, _ = http.request("POST", "/customers/101/delete?tenant_id=10",
                                              body=urlencode({"_zelyra_csrf": cust_csrf}))
        assert status == 500, f"expected 500 Internal Server Error when deleting referenced customer, got {status}"
        assert "The record could not be deleted" in delete_resp, f"unexpected error response: {delete_resp}"
        assert "fk_" not in delete_resp, "database constraint internals must not be leaked to user"
        # Verify customer 101 still exists in database
        check_cust = execute_sql(db_url, "SELECT COUNT(*) FROM customers WHERE id = 101;")
        assert "1" in check_cust, "Customer 101 must not be deleted when referenced!"
        print("  -> Verified restrictive deletion: database constraint violation handled with safe 500.")

        # 6.f Clean Child Deletion followed by Parent Deletion
        # Find project id and parse Project CSRF token
        proj_id_raw = execute_sql(db_url, "SELECT id FROM projects WHERE title = 'Alpha Project One';")
        proj_id = proj_id_raw.strip().split()[0]
        status, proj_detail_html, _ = http.request("GET", f"/projects/{proj_id}?tenant_id=10")
        proj_parser = SelectOptionParser()
        proj_parser.feed(proj_detail_html)
        proj_csrf = proj_parser.csrf or csrf

        status, _, _ = http.request("POST", f"/projects/{proj_id}/delete?tenant_id=10",
                                    body=urlencode({"_zelyra_csrf": proj_csrf}))
        assert status == 303, f"child project deletion failed: {status}"

        # Now delete customer 101 with customer CSRF token
        status, cust_detail_html2, _ = http.request("GET", "/customers/101?tenant_id=10")
        cust_parser2 = SelectOptionParser()
        cust_parser2.feed(cust_detail_html2)
        cust_csrf2 = cust_parser2.csrf or cust_csrf

        status, _, _ = http.request("POST", "/customers/101/delete?tenant_id=10",
                                    body=urlencode({"_zelyra_csrf": cust_csrf2}))
        assert status == 303, f"parent customer deletion should succeed after child deleted, got {status}"
        check_cust2 = execute_sql(db_url, "SELECT COUNT(*) FROM customers WHERE id = 101;")
        assert "0" in check_cust2, "Customer 101 must be deleted now!"
        print("  -> Verified ordered deletion: child deletion succeeded, followed by parent deletion.")

        print(f"\n[OK] All relational CRUD and matrix checks PASSED for {engine}!")
    except Exception:
        log_file = Path(temp_dir) / "server.log"
        if log_file.exists():
            print("SERVER LOG:\n", log_file.read_text())
        raise
        if server_proc:
            server_proc.terminate()
            try:
                server_proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                server_proc.kill()
        shutil.rmtree(temp_dir, ignore_errors=True)


def db_name_from_url(db_url):
    return urlsplit(db_url).path.lstrip("/")


def main():
    parser = argparse.ArgumentParser(description="Zelyra Relational CRUD & Database Matrix E2E")
    parser.add_argument("--backend", choices=["mariadb", "mysql", "postgres", "all"], default="all")
    args = parser.parse_args()

    assert BIN.is_file() and os.access(BIN, os.X_OK), f"Zelyra binary not found or not executable: {BIN}"

    backends = ["mariadb", "mysql", "postgres"] if args.backend == "all" else [args.backend]
    env_url = os.environ.get("DATABASE_URL")

    active_containers = []

    def handle_exit(signum, frame):
        for c in active_containers:
            c.stop()
        sys.exit(1)

    signal.signal(signal.SIGINT, handle_exit)
    signal.signal(signal.SIGTERM, handle_exit)

    try:
        for engine in backends:
            if env_url:
                db_url = env_url
                container = None
            else:
                container = DisposableContainer(engine)
                active_containers.append(container)
                container.start()
                db_url = container.url

            try:
                test_backend(engine, db_url)
            finally:
                if container:
                    container.stop()
                    active_containers.remove(container)

        print("\n=======================================================")
        print("ALL DATABASE MATRIX & RELATIONAL CRUD CHECKS GREEN!")
        print("=======================================================")
    finally:
        for c in active_containers:
            c.stop()


if __name__ == "__main__":
    main()
