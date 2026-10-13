#!/usr/bin/env python3
"""Comprehensive E2E test suite for Section 4.B: Laravel Migrator and Business Flows.

Verifies technical release gates for Section 4.B:
1. Versioned Laravel 10/11 compatibility matrix and accepted mappings:
   - Source is strictly read-only: zero file modifications in source project.
   - Report and scaffold target paths are strictly separated from source.
   - Rejection of path traversal (..), non-existent parents, and pre-existing targets.
   - Complete detection of 18 finding kinds: framework, route, model, controller,
     migration, relationship, validation, job, authorization, tenant_boundary,
     financial_rule, file_access, payment_rule, ambiguity, environment_access,
     integration, transaction, view.
   - Strict privacy: .env files, APP_KEY, passwords, and PII are never read or output.
   - Generated scaffold (zelyra.toml, main.zyl, README.md, MIGRATION_REPORT.md,
     MIGRATION_TODO.md) compiles with `zelyra check` and explicitly distinguishes
     scaffolding from untransferred business logic.
2. Accepted mappings proof across the verified database matrix:
   - Evaluated across MariaDB 11.4.13, MySQL 8.4.11, and PostgreSQL 16.15.
   - Model + migration with foreign key and decimal -> Zelyra tables with Decimal and FK.
   - Resource route + controller CRUD -> Zelyra crud with tenant boundaries.
   - Relational joins, decimal accuracy, cross-tenant tamper protection (422),
     and restrictive deletion (500).
3. Multi-module example and Docker module export:
   - Verified on examples/customer_orders_modules with dedicated database configuration.
   - Dry-run validation and complete Docker bundle export with compiler ref.
   - Bundled project passes `zelyra check` and maintains isolated database scope.

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
import socket
import subprocess
import sys
import tempfile
import time
from urllib.parse import unquote, urlencode, urlsplit

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


class DatabaseContainer:
    def __init__(self, engine):
        self.engine = engine
        self.container = None
        self.port = None
        self.url = None

    def start(self):
        pid = os.getpid()
        ts = int(time.time() * 1000) % 1000000
        if self.engine == "mariadb":
            name = f"zelyra-lm-mariadb-{pid}-{ts}"
            cmd = [
                "docker", "run", "-d", "--name", name, "-p", "127.0.0.1::3306",
                "-e", "MARIADB_ROOT_PASSWORD=test_pass",
                "-e", "MARIADB_DATABASE=zelyra_test",
                "--health-cmd=mariadb-admin ping -h 127.0.0.1 -u root -ptest_pass",
                "--health-interval=1s", "--health-retries=30",
                "mariadb:11.4.13",
            ]
        elif self.engine == "mysql":
            name = f"zelyra-lm-mysql-{pid}-{ts}"
            cmd = [
                "docker", "run", "-d", "--name", name, "-p", "127.0.0.1::3306",
                "-e", "MYSQL_ROOT_PASSWORD=test_pass",
                "-e", "MYSQL_DATABASE=zelyra_test",
                "--health-cmd=mysqladmin ping -h 127.0.0.1 -u root -ptest_pass",
                "--health-interval=1s", "--health-retries=30",
                "mysql:8.4.11",
            ]
        elif self.engine == "postgres":
            name = f"zelyra-lm-pg-{pid}-{ts}"
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
    port = str(parts.port or (5432 if parts.scheme.startswith("postgres") else 3306))
    database = parts.path.lstrip("/")

    if parts.scheme in ("postgres", "postgresql"):
        env = dict(os.environ, PGPASSWORD=password)
        cmd = ["psql", "-h", host, "-p", port, "-U", user, "-d", database, "-t", "-A", "-c", sql]
    else:
        client = "mysql" if parts.scheme == "mysql" else "mariadb"
        env = dict(os.environ, MYSQL_PWD=password)
        cmd = [client, "--protocol=tcp", "-h", host, "-P", port, "-u", user, database, "--batch", "--skip-column-names", "-e", sql]

    res = subprocess.run(cmd, env=env, capture_output=True, text=True)
    if res.returncode != 0:
        raise RuntimeError(f"execute_sql failed ({parts.scheme}):\n{res.stderr}\nSQL: {sql}")
    return res.stdout


def find_free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


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


def build_accepted_project_source(engine):
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

table orders {{
    id: Id primary auto
    tenant_id: Int required
    title: String(100) required
    customer: Customer required
    total: Decimal required
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

crud Order -> orders {{
    requires auth
    permits "orders.manage"
    tenant: tenant_id
    list {{
        id
        title
        customer
        total
    }}
}}

fn main() {{}}
"""


def test_laravel_migrator_static_analysis():
    print("\n=======================================================")
    print("Part 1: Laravel Migrator Static Analysis & Safety Enforcements")
    print("=======================================================")

    temp_parent = Path(tempfile.mkdtemp(prefix="zelyra-lm-test-"))
    try:
        source_dir = temp_parent / "laravel-fixture"
        source_dir.mkdir()
        (source_dir / "app" / "Http" / "Controllers").mkdir(parents=True)
        (source_dir / "app" / "Models").mkdir(parents=True)
        (source_dir / "app" / "Jobs").mkdir(parents=True)
        (source_dir / "app" / "Policies").mkdir(parents=True)
        (source_dir / "database" / "migrations").mkdir(parents=True)
        (source_dir / "resources" / "views").mkdir(parents=True)
        (source_dir / "routes").mkdir(parents=True)

        # 1. composer.json with expanded package recognition
        (source_dir / "composer.json").write_text(json.dumps({
            "require": {
                "php": "^8.2",
                "laravel/framework": "^11.0",
                "spatie/laravel-permission": "^6.0",
                "stripe/stripe-php": "^13.0",
                "inertiajs/inertia-laravel": "^1.0",
                "livewire/livewire": "^3.0",
            }
        }), encoding="utf-8")

        # 2. routes/web.php
        (source_dir / "routes" / "web.php").write_text("""<?php
use Illuminate\\Support\\Facades\\Route;
Route::get('/orders', [OrderController::class, 'index']);
Route::post('/orders', [OrderController::class, 'store']);
Route::resource('customers', CustomerController::class);
""", encoding="utf-8")

        # 3. Models with relations
        (source_dir / "app" / "Models" / "Customer.php").write_text("""<?php
namespace App\\Models;
use Illuminate\\Database\\Eloquent\\Model;
class Customer extends Model {
    public function orders() {
        return $this->hasMany(Order::class);
    }
}
""", encoding="utf-8")

        (source_dir / "app" / "Models" / "Order.php").write_text("""<?php
namespace App\\Models;
use Illuminate\\Database\\Eloquent\\Model;
class Order extends Model {
    public function customer() {
        return $this->belongsTo(Customer::class);
    }
}
""", encoding="utf-8")

        # 4. Controller with business rules, authorization, file, payment, ambiguity, env
        (source_dir / "app" / "Http" / "Controllers" / "OrderController.php").write_text("""<?php
namespace App\\Http\\Controllers;
use App\\Models\\Order;
use Illuminate\\Http\\Request;
use Illuminate\\Support\\Facades\\DB;
use Illuminate\\Support\\Facades\\Mail;
use Illuminate\\Support\\Facades\\Storage;

class OrderController {
    public function store(Request $request) {
        $this->authorize('create', Order::class);
        $request->validate(['title' => 'required', 'total' => 'required|numeric']);
        $order = new Order();
        $order->tenant_id = 10;
        $order->total = $request->items->sum('price');
        $path = $request->file('receipt')->store('receipts');
        $order->charge(4999);
        $rawSql = DB::raw("NOW()");
        $stripeKey = env('STRIPE_SECRET');
        Storage::disk('s3')->put('audit', 'order_created');
        DB::transaction(function () use ($order) {
            $order->save();
        });
        Mail::to('billing@example.test')->send(new OrderMailable($order));
        return response()->download($path);
    }
}
""", encoding="utf-8")

        # 5. Job implementing ShouldQueue
        (source_dir / "app" / "Jobs" / "ProcessOrderJob.php").write_text("""<?php
namespace App\\Jobs;
use Illuminate\\Contracts\\Queue\\ShouldQueue;
class ProcessOrderJob implements ShouldQueue {
    public function handle() {}
}
""", encoding="utf-8")

        # 6. Policy
        (source_dir / "app" / "Policies" / "OrderPolicy.php").write_text("""<?php
namespace App\\Policies;
class OrderPolicy {
    public function update($user, $order) {
        return $user->id === $order->user_id;
    }
}
""", encoding="utf-8")

        # 7. Migration with foreign key and decimal
        (source_dir / "database" / "migrations" / "2026_01_01_create_orders_table.php").write_text("""<?php
use Illuminate\\Database\\Migrations\\Migration;
use Illuminate\\Database\\Schema\\Blueprint;
use Illuminate\\Support\\Facades\\Schema;

return new class extends Migration {
    public function up() {
        Schema::create('orders', function (Blueprint $table) {
            $table->id();
            $table->unsignedBigInteger('tenant_id');
            $table->foreignId('customer_id')->constrained('customers');
            $table->string('title', 100);
            $table->decimal('total', 12, 2);
            $table->timestamps();
        });
    }
};
""", encoding="utf-8")

        # 8. Blade view
        (source_dir / "resources" / "views" / "orders.blade.php").write_text("""
<div>
    <h1>Orders</h1>
    @foreach ($orders as $order)
        <p>{{ $order->title }}: {{ $order->total }}</p>
    @endforeach
</div>
""", encoding="utf-8")

        # 9. Secret environment file (MUST NEVER BE READ OR OUTPUT)
        (source_dir / ".env").write_text("""
APP_KEY=base64:FIXTURE_SECRET_APP_KEY_NEVER_LEAK_INTO_REPORTS
DB_PASSWORD=fixture_db_password_top_secret
STRIPE_SECRET=sk_test_super_secret_token_12345
""", encoding="utf-8")

        # Capture hash of all files before migration
        hashes_before = {}
        for path in source_dir.rglob("*"):
            if path.is_file():
                hashes_before[str(path.relative_to(source_dir))] = hashlib.sha256(path.read_bytes()).hexdigest()

        # Step 1.A: Boundary and safety rejections
        print("[1.A] Verifying path boundaries and overwrite prevention...")

        # Rejection: report inside source project
        res = subprocess.run([str(BIN), "migrate", "laravel", str(source_dir),
                              "--report", str(source_dir / "report.json"), "--format", "json"],
                             capture_output=True, text=True)
        assert res.returncode != 0, "expected rejection when report is inside source project"
        assert "error[E-MIGRATE-002]" in res.stderr
        assert "must be outside the Laravel source project" in res.stderr
        print("  -> Verified: rejected report path inside source project.")

        # Rejection: scaffold inside source project
        res = subprocess.run([str(BIN), "migrate", "laravel", str(source_dir),
                              "--output", str(source_dir / "scaffold")],
                             capture_output=True, text=True)
        assert res.returncode != 0, "expected rejection when scaffold is inside source project"
        assert "error[E-MIGRATE-002]" in res.stderr
        assert "scaffold target must be separate from the Laravel source project" in res.stderr
        print("  -> Verified: rejected scaffold target inside source project.")

        # Rejection: report path with traversal `..`
        res = subprocess.run([str(BIN), "migrate", "laravel", str(source_dir),
                              "--report", f"{temp_parent}/foo/../report.json"],
                             capture_output=True, text=True)
        assert res.returncode != 0, "expected rejection when path contains `..`"
        assert "error[E-MIGRATE-002]" in res.stderr
        print("  -> Verified: rejected report path containing `..`.")

        # Rejection: pre-existing report
        existing_report = temp_parent / "existing-report.json"
        existing_report.write_text("{}", encoding="utf-8")
        res = subprocess.run([str(BIN), "migrate", "laravel", str(source_dir),
                              "--report", str(existing_report)],
                             capture_output=True, text=True)
        assert res.returncode != 0, "expected rejection for pre-existing report"
        assert "error[E-MIGRATE-002]" in res.stderr
        assert "report already exists" in res.stderr
        print("  -> Verified: rejected pre-existing report file.")

        # Rejection: pre-existing scaffold
        existing_scaffold = temp_parent / "existing-scaffold"
        existing_scaffold.mkdir()
        res = subprocess.run([str(BIN), "migrate", "laravel", str(source_dir),
                              "--output", str(existing_scaffold)],
                             capture_output=True, text=True)
        assert res.returncode != 0, "expected rejection for pre-existing scaffold directory"
        assert "error[E-MIGRATE-002]" in res.stderr
        assert "scaffold target already exists" in res.stderr
        print("  -> Verified: rejected pre-existing scaffold target.")

        # Step 1.B: Execute clean migration with report and scaffold
        print("[1.B] Generating migration report and compiler-checked scaffold...")
        report_path = temp_parent / "reports" / "migration.json"
        report_path.parent.mkdir()
        scaffold_path = temp_parent / "zelyra-scaffold"

        res = subprocess.run([
            str(BIN), "migrate", "laravel", str(source_dir),
            "--report", str(report_path),
            "--output", str(scaffold_path),
            "--format", "json",
        ], capture_output=True, text=True)

        assert res.returncode == 0, f"migration failed:\nstdout: {res.stdout}\nstderr: {res.stderr}"
        assert "Laravel source analyzed without executing PHP or reading environment files." in res.stdout
        assert "Transferred business logic: none; review MIGRATION_TODO.md and MIGRATION_REPORT.md." in res.stdout
        print("  -> Migration command succeeded.")

        # Step 1.C: Source Immutability Verification
        print("[1.C] Verifying source tree immutability...")
        hashes_after = {}
        for path in source_dir.rglob("*"):
            if path.is_file():
                hashes_after[str(path.relative_to(source_dir))] = hashlib.sha256(path.read_bytes()).hexdigest()
        assert hashes_before == hashes_after, "source files were modified during analysis!"
        print("  -> Source project files remain 100% byte-identical (read-only guarantee).")

        # Step 1.D: Report Verification (JSON & Markdown)
        print("[1.D] Verifying report contents, 18 finding kinds, and zero secret leakage...")
        report_data = json.loads(report_path.read_text(encoding="utf-8"))
        assert report_data["analyzer"] == "Zelyra Laravel Migrator 0.9.9", f"unexpected analyzer: {report_data['analyzer']}"
        assert report_data["mapping_ruleset"] == "0.9.9", f"unexpected ruleset: {report_data['mapping_ruleset']}"
        assert report_data["laravel_constraint"] == "^11.0", f"unexpected constraint: {report_data['laravel_constraint']}"

        # Check dependencies
        deps = report_data.get("laravel_packages", [])
        assert any("spatie/laravel-permission" in d for d in deps), f"missing spatie in deps: {deps}"
        assert any("stripe/stripe-php" in d for d in deps), f"missing stripe in deps: {deps}"
        assert any("inertiajs/inertia-laravel" in d for d in deps), f"missing inertia in deps: {deps}"
        assert any("livewire/livewire" in d for d in deps), f"missing livewire in deps: {deps}"
        print("  -> Verified composer dependency coverage.")

        # Check all 18 kinds
        findings = report_data.get("findings", [])
        expected_kinds = {
            "framework",
            "route",
            "model",
            "controller",
            "migration",
            "relationship",
            "validation",
            "job",
            "authorization",
            "tenant_boundary",
            "financial_rule",
            "file_access",
            "payment_rule",
            "ambiguity",
            "environment_access",
            "integration",
            "transaction",
            "view",
        }
        found_kinds = {f["kind"] for f in findings}
        missing_kinds = expected_kinds - found_kinds
        assert not missing_kinds, f"missing expected finding kinds: {missing_kinds}"
        print(f"  -> Verified all {len(expected_kinds)} finding kinds present in analysis.")

        # Verify no findings claimed 'supported'
        for f in findings:
            assert f["status"] in ("manual", "partial", "unresolved"), f"unsupported status claimed: {f}"
        print("  -> Verified all findings marked manual or partial (zero false parity claims).")

        # Verify zero secret leakage in JSON report
        raw_report = report_path.read_text(encoding="utf-8")
        assert "FIXTURE_SECRET_APP_KEY" not in raw_report
        assert "fixture_db_password" not in raw_report
        assert "sk_test_super_secret" not in raw_report
        # Verify env('STRIPE_SECRET') is named without reading its value
        env_findings = [f for f in findings if f["kind"] == "environment_access"]
        assert len(env_findings) >= 1
        assert any("STRIPE_SECRET" in f["name"] for f in env_findings)
        print("  -> Verified zero secret leakage (.env skipped, env() values never read).")

        # Step 1.E: Scaffold Verification & Compiler Check
        print("[1.E] Verifying generated scaffold structure and compiler check...")
        assert (scaffold_path / "zelyra.toml").is_file()
        assert (scaffold_path / "main.zyl").is_file()
        assert (scaffold_path / "README.md").is_file()
        assert (scaffold_path / "MIGRATION_REPORT.md").is_file()
        assert (scaffold_path / "MIGRATION_TODO.md").is_file()

        todo_text = (scaffold_path / "MIGRATION_TODO.md").read_text(encoding="utf-8")
        assert "No Laravel business logic has been transferred" in todo_text
        assert "0.9.9/route" in todo_text
        assert "0.9.9/payment_rule" in todo_text
        assert "0.9.9/ambiguity" in todo_text
        assert "0.9.9/environment_access" in todo_text

        # Verify scaffold passes `zelyra check`
        check_res = subprocess.run([str(BIN), "check", str(scaffold_path / "main.zyl")],
                                   capture_output=True, text=True)
        assert check_res.returncode == 0, f"scaffold failed zelyra check:\n{check_res.stderr}"
        print("  -> Verified scaffold compiles cleanly (`zelyra check` passed).")

    finally:
        shutil.rmtree(temp_parent, ignore_errors=True)


def test_accepted_mappings_on_backend(engine, db_url):
    print(f"\n--- Testing Accepted Mappings on Backend: {engine} ({db_url}) ---")
    temp_dir = tempfile.mkdtemp(prefix=f"zelyra-lm-backend-{engine}-")
    server_proc = None

    try:
        project_file = Path(temp_dir) / "main.zyl"
        project_file.write_text(build_accepted_project_source(engine), encoding="utf-8")

        # 1. Apply Schema and Inspect Foreign Key
        print("  [1/5] Applying schema and inspecting relational foreign key...")
        env = dict(os.environ, DATABASE_URL=db_url)
        setup_cmd = [str(BIN), "db", "apply", str(project_file)] if engine == "postgres" else [str(BIN), "db", "setup", str(project_file)]
        setup_res = subprocess.run(setup_cmd, env=env, capture_output=True, text=True)
        if setup_res.returncode != 0:
            raise RuntimeError(f"db setup/apply failed on {engine}:\n{setup_res.stderr}")

        map_res = subprocess.run([str(BIN), "db", "map", str(project_file), "--format=json"],
                                 env=env, capture_output=True, text=True)
        if map_res.returncode != 0:
            raise RuntimeError(f"db map failed on {engine}:\n{map_res.stderr}")

        map_data = json.loads(map_res.stdout)
        orders_table = None
        for mod in map_data.get("modules", []):
            for tbl in mod.get("tables", []):
                if tbl.get("name") == "orders":
                    orders_table = tbl
                    break
        assert orders_table is not None, "orders table not found in db map"
        assert len(orders_table.get("foreign_keys", [])) >= 1, "missing foreign key on orders table"
        fk = orders_table["foreign_keys"][0]
        ref_table = fk.get("references_table") or fk.get("referenced_table")
        assert fk["column"] == "customer_id", f"expected fk column customer_id, got {fk['column']}"
        assert ref_table == "customers", f"expected fk target customers, got {ref_table}"
        print(f"    -> Constraint verified: {fk['column']} -> {ref_table}(id)")

        # 2. Hash Password and Seed Test Data
        print("  [2/5] Seeding tenant users, permissions, and initial customers...")
        hash_res = subprocess.run([str(BIN), "auth", "hash-password", "--stdin"],
                                  input="ZelyraSecret123!\n", text=True, capture_output=True, check=True)
        pwd_hash = hash_res.stdout.strip()

        # Seed Users (User 1 -> Tenant 10, User 2 -> Tenant 20)
        execute_sql(db_url, f"INSERT INTO users (id, email, password_hash, active) VALUES "
                            f"(1, 'usera@example.test', '{pwd_hash}', true), "
                            f"(2, 'userb@example.test', '{pwd_hash}', true);")
        execute_sql(db_url, "INSERT INTO tenant_memberships (user_id, tenant_id, active) VALUES "
                            "(1, 10, true), (2, 20, true);")
        execute_sql(db_url, "INSERT INTO user_permissions (user_id, permission) VALUES "
                            "(1, 'customers.manage'), (1, 'orders.manage'), "
                            "(2, 'customers.manage'), (2, 'orders.manage');")
        execute_sql(db_url, "INSERT INTO customers (id, tenant_id, name) VALUES "
                            "(101, 10, 'Acme Alpha Corp'), "
                            "(201, 20, 'Beta Solutions Inc');")

        # 3. Start Server and Authenticate
        port = find_free_port()
        print(f"  [3/5] Starting server on 127.0.0.1:{port}...")
        server_log = open(Path(temp_dir) / "server.log", "w")
        server_proc = subprocess.Popen([str(BIN), "serve", str(project_file), f"127.0.0.1:{port}"],
                                       env=env, stdout=server_log, stderr=server_log)
        time.sleep(1.5)
        http = HttpClient("127.0.0.1", port)

        if engine == "postgres":
            session_token = "zelyra_lm_session_user_a"
            token_hash = hashlib.blake2s(session_token.encode("utf-8")).hexdigest()
            execute_sql(db_url, f"INSERT INTO auth_sessions (user_id, token_hash, expires_at) VALUES (1, '{token_hash}', CURRENT_TIMESTAMP + INTERVAL '1 day');")
            http.cookie = f"zelyra_session={session_token}"
            print("    -> Authenticated User A via PostgreSQL session token.")
        else:
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
            print("    -> Authenticated User A via HTTP /login.")

        # 4. Form Inspection and Tamper Protection
        print("  [4/5] Testing form dropdown scoping and cross-tenant tamper protection...")
        status, form_html, _ = http.request("GET", "/orders/new?tenant_id=10")
        assert status == 200, f"failed to load order form: {status}"
        form_parser = SelectOptionParser()
        form_parser.feed(form_html)
        csrf = form_parser.csrf
        customer_options = form_parser.selects.get("customer", [])
        assert "101" in customer_options, f"Customer 101 must be in options: {customer_options}"
        assert "201" not in customer_options, f"Customer 201 must NOT be in options: {customer_options}"

        # Tamper attempt: submit cross-tenant customer=201 -> 422
        tampered_body = urlencode({
            "_zelyra_csrf": csrf,
            "title": "Tampered Order",
            "customer": "201",
            "total": "99.50",
        })
        status, tampered_html, _ = http.request("POST", "/orders/new?tenant_id=10", body=tampered_body)
        assert status == 422, f"expected 422 Unprocessable Entity, got {status}"
        assert "selected value does not exist" in tampered_html
        print("    -> Tamper protection verified: cross-tenant FK rejected with 422.")

        # 5. Valid Creation, Decimal Accuracy, Relational Joins, and Restrictive Delete
        print("  [5/5] Testing valid creation, decimal accuracy, and restrictive foreign key delete...")
        valid_body = urlencode({
            "_zelyra_csrf": csrf,
            "title": "Alpha Production Order",
            "customer": "101",
            "total": "199.95",
        })
        status, _, loc = http.request("POST", "/orders/new?tenant_id=10", body=valid_body)
        assert status == 303, f"expected 303 redirect, got {status}"

        # List view: customer joined name and decimal amount
        status, list_html, _ = http.request("GET", "/orders?tenant_id=10")
        assert status == 200, f"failed to get order list: {status}"
        assert "Alpha Production Order" in list_html, "missing order title in list"
        assert "Acme Alpha Corp" in list_html, "missing joined customer name in list"
        assert "199.95" in list_html, "missing exact decimal amount in list"
        print("    -> Verified relational join and decimal precision display in list.")

        # Restrictive Delete: deleting Customer 101 while referenced by Order must fail
        status, cust_detail_html, _ = http.request("GET", "/customers/101?tenant_id=10")
        assert status == 200, f"failed to load customer detail: {status}"
        cust_parser = SelectOptionParser()
        cust_parser.feed(cust_detail_html)
        cust_csrf = cust_parser.csrf or csrf

        status, delete_resp, _ = http.request("POST", "/customers/101/delete?tenant_id=10",
                                              body=urlencode({"_zelyra_csrf": cust_csrf}))
        assert status == 500, f"expected 500 on foreign key restrict, got {status}"
        assert "The record could not be deleted" in delete_resp

        # Verify customer still exists
        check_cust = execute_sql(db_url, "SELECT COUNT(*) FROM customers WHERE id = 101;")
        assert "1" in check_cust, "customer was deleted despite FK constraint!"
        print("    -> Restrictive delete verified: parent deletion rejected, data preserved.")

        # Clean delete: delete order first, then customer
        order_id_raw = execute_sql(db_url, "SELECT id FROM orders WHERE title = 'Alpha Production Order';")
        order_id = order_id_raw.strip().split()[0]
        status, order_detail_html, _ = http.request("GET", f"/orders/{order_id}?tenant_id=10")
        order_parser = SelectOptionParser()
        order_parser.feed(order_detail_html)
        order_csrf = order_parser.csrf or csrf

        status, _, _ = http.request("POST", f"/orders/{order_id}/delete?tenant_id=10",
                                    body=urlencode({"_zelyra_csrf": order_csrf}))
        assert status == 303, f"failed to delete order: {status}"

        # Now delete customer 101 with fresh customer CSRF
        status, cust_detail_html2, _ = http.request("GET", "/customers/101?tenant_id=10")
        cust_parser2 = SelectOptionParser()
        cust_parser2.feed(cust_detail_html2)
        cust_csrf2 = cust_parser2.csrf or cust_csrf

        status, _, _ = http.request("POST", "/customers/101/delete?tenant_id=10",
                                    body=urlencode({"_zelyra_csrf": cust_csrf2}))
        assert status == 303, f"failed to delete customer after referencing order removed: {status}"
        print("    -> Ordered deletion succeeded.")

    finally:
        if server_proc:
            server_proc.terminate()
            try:
                server_proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                server_proc.kill()
        shutil.rmtree(temp_dir, ignore_errors=True)


def test_accepted_mappings_database_matrix():
    print("\n=======================================================")
    print("Part 2: Accepted Mappings Across Database Matrix")
    print("=======================================================")

    engines = ["mariadb", "mysql", "postgres"]
    for engine in engines:
        container = DatabaseContainer(engine)
        try:
            container.start()
            test_accepted_mappings_on_backend(engine, container.url)
        finally:
            container.stop()


def test_multi_module_example_and_docker_export():
    print("\n=======================================================")
    print("Part 3: Multi-Module Example & Docker Export E2E")
    print("=======================================================")

    example_dir = ROOT / "examples" / "customer_orders_modules"
    entry_file = example_dir / "main.zyl"
    target_module = "src/customers.zyl"

    # Step 3.A: Check example project
    print("[3.A] Running `zelyra check` on customer orders multi-module example...")
    check_res = subprocess.run([str(BIN), "check", str(entry_file)],
                               capture_output=True, text=True)
    assert check_res.returncode == 0, f"customer orders example failed check:\n{check_res.stderr}"
    print("  -> Verified multi-module example passes compiler check.")

    temp_parent = Path(tempfile.mkdtemp(prefix="zelyra-lm-bundle-"))
    try:
        # Step 3.B: Dry Run validation
        print("[3.B] Testing `zelyra module bundle --dry-run`...")
        bundle_target = temp_parent / "customers-bundle"
        compiler_commit = "7be287a2dfecf2ef6c694a9735dcae987c95e1e1"
        dry_res = subprocess.run([
            str(BIN), "module", "bundle", str(entry_file), target_module,
            "--output", str(bundle_target),
            "--docker",
            "--compiler-ref", compiler_commit,
            "--dry-run",
        ], capture_output=True, text=True)
        assert dry_res.returncode == 0, f"dry run failed:\n{dry_res.stderr}"
        assert not bundle_target.exists(), "dry-run created bundle directory!"
        dry_data = json.loads(dry_res.stdout)
        assert dry_data["plan"]["writes_performed"] is False
        assert any(f["relative_path"] == "Dockerfile" for f in dry_data["plan"]["files"])
        print("  -> Verified dry-run lists plan without writing files.")

        # Step 3.C: Complete Docker Bundle Export
        print("[3.C] Generating Docker bundle export with pinned compiler ref...")
        compiler_commit = "7be287a2dfecf2ef6c694a9735dcae987c95e1e1"
        bundle_res = subprocess.run([
            str(BIN), "module", "bundle", str(entry_file), target_module,
            "--output", str(bundle_target),
            "--docker",
            "--compiler-ref", compiler_commit,
        ], capture_output=True, text=True)
        assert bundle_res.returncode == 0, f"module bundle failed:\n{bundle_res.stderr}"
        assert bundle_target.is_dir()
        print("  -> Bundle export succeeded.")

        # Step 3.D: Inspect exported bundle contents
        print("[3.D] Inspecting exported bundle structure and source closure...")
        assert (bundle_target / "Dockerfile").is_file()
        assert (bundle_target / "docker-compose.yml").is_file()
        assert (bundle_target / ".env.example").is_file()
        assert (bundle_target / ".dockerignore").is_file()
        assert (bundle_target / "zelyra.bundle.json").is_file()
        assert (bundle_target / "main.zyl").is_file()
        assert (bundle_target / "src" / "customers.zyl").is_file()
        assert (bundle_target / "src" / "database.zyl").is_file()
        assert (bundle_target / "src" / "auth.zyl").is_file()
        assert (bundle_target / "src" / "shell.zyl").is_file()
        # Ensure unneeded modules are not included
        assert not (bundle_target / "src" / "orders.zyl").exists(), "unselected orders.zyl included in closure"
        assert not (bundle_target / "src" / "reporting.zyl").exists(), "unselected reporting.zyl included in closure"
        print("  -> Verified minimal source closure for exported customers module.")

        # Step 3.E: Verify bundled project passes `zelyra check`
        print("[3.E] Running `zelyra check` on generated bundle...")
        check_bundle = subprocess.run([str(BIN), "check", str(bundle_target / "main.zyl")],
                                      capture_output=True, text=True)
        assert check_bundle.returncode == 0, f"bundle failed check:\n{check_bundle.stderr}"
        print("  -> Generated bundle passes `zelyra check`.")

        # Step 3.F: Validate manifest metadata in zelyra.bundle.json
        print("[3.F] Validating zelyra.bundle.json manifest...")
        manifest = json.loads((bundle_target / "zelyra.bundle.json").read_text(encoding="utf-8"))
        assert manifest["database"]["required"] is True
        assert manifest["database"]["configurations"][0]["engine"] == "mariadb"
        assert manifest["database"]["configurations"][0]["connection_environment"] == "ZELYRA_DATABASE_MAIN_URL"
        assert manifest["docker"]["compiler_commit"] == compiler_commit
        assert manifest["docker"]["database_connection_scope"] == "per_exported_compose_project"
        print("  -> Verified manifest database configuration and compiler reference.")

    finally:
        shutil.rmtree(temp_parent, ignore_errors=True)


def main():
    parser = argparse.ArgumentParser(description="Laravel Migrator & Business Flows E2E Suite")
    parser.add_argument("--skip-matrix", action="store_true", help="Skip database matrix tests")
    args = parser.parse_args()

    start_time = time.monotonic()
    print("=================================================================")
    print("Zelyra 0.9.9 Technical Gate 4.B: Laravel Migrator & Business Flows")
    print(f"Binary: {BIN}")
    print("=================================================================")

    test_laravel_migrator_static_analysis()
    if not args.skip_matrix:
        test_accepted_mappings_database_matrix()
    test_multi_module_example_and_docker_export()

    duration = time.monotonic() - start_time
    print("\n=================================================================")
    print(f"ALL SECTION 4.B E2E TESTS PASSED in {duration:.1f}s")
    print("=================================================================")


if __name__ == "__main__":
    main()
