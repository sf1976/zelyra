#!/usr/bin/env python3
"""Exact fixed-point values across compiler, HTTP, forms and real SQL storage.

DATABASE_URL must identify a disposable test database. Every created table has
an independent random name and is removed in finally; no production data is used.
"""
import hashlib
import http.client
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import tempfile
import time
from urllib.parse import urlencode, urlsplit, unquote
import uuid


ROOT = Path(__file__).resolve().parents[1]
BIN = Path(os.environ.get("ZELYRA_BIN", ROOT / "target/debug/zelyra")).resolve()
URL = os.environ.get("DATABASE_URL", "")
PARTS = urlsplit(URL)
if PARTS.scheme not in ("postgres", "postgresql", "mariadb", "mysql"):
    raise SystemExit("DATABASE_URL must identify a disposable PostgreSQL/MariaDB/MySQL database")
ENGINE = "postgres" if PARTS.scheme in ("postgres", "postgresql") else PARTS.scheme
TABLE = "exact_amounts_" + uuid.uuid4().hex[:16]


def sql(statement):
    env = os.environ.copy()
    host = PARTS.hostname or "127.0.0.1"
    user = unquote(PARTS.username or "")
    password = unquote(PARTS.password or "")
    database = unquote(PARTS.path.lstrip("/"))
    if ENGINE == "postgres":
        env["PGPASSWORD"] = password
        command = ["psql", "-X", "-At", "-h", host, "-p", str(PARTS.port or 5432),
                   "-U", user, "-d", database, "-c", statement]
    else:
        env["MYSQL_PWD"] = password
        command = ["mariadb", "--protocol=tcp", "--batch", "--skip-column-names",
                   "--host=" + host, "--port=" + str(PARTS.port or 3306),
                   "--user=" + user, database, "-e", statement]
    result = subprocess.run(command, env=env, text=True, capture_output=True, timeout=30)
    if result.returncode:
        raise AssertionError("test SQL command failed: " + result.stderr.replace(password, "[redacted]")[:500])
    return result.stdout.strip()


def compiler(*arguments, ok=True, timeout=30):
    result = subprocess.run([str(BIN), *map(str, arguments)], text=True,
                            capture_output=True, timeout=timeout)
    if ok and result.returncode:
        raise AssertionError(result.stderr.replace(URL, "[test database URL]"))
    if not ok:
        assert result.returncode == 1, "failure must be controlled, not a signal/abort"
    return result


def request(port, method, path, payload=None, content_type=None):
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
    headers = {"Origin": f"http://127.0.0.1:{port}"}
    if content_type:
        headers["Content-Type"] = content_type
    connection.request(method, path, payload, headers)
    response = connection.getresponse()
    body = response.read().decode("utf-8")
    status = response.status
    connection.close()
    return status, body


def wait_server(server, port):
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        if server.poll() is not None:
            raise AssertionError("test web server exited before readiness")
        try:
            return request(port, "GET", "/forms/ExactAmount")
        except (OSError, http.client.HTTPException):
            time.sleep(0.05)
    raise AssertionError("test web server did not become ready")


def stop(server):
    server.terminate()
    try:
        server.wait(timeout=10)
    except subprocess.TimeoutExpired:
        server.kill()
        server.wait(timeout=5)
        raise AssertionError("server failed to stop within the bound")


# Migration planning sees only this test's own database, including in shared CI.
ORIGINAL_PARTS = PARTS
ORIGINAL_URL = URL
TEST_DATABASE = "zelyra_exact_" + uuid.uuid4().hex[:16]
sql(f"CREATE DATABASE {TEST_DATABASE}")
PARTS = PARTS._replace(path="/" + TEST_DATABASE)
URL = PARTS.geturl()
os.environ["DATABASE_URL"] = URL

with tempfile.TemporaryDirectory(prefix="zelyra-exact-numbers-") as directory:
    directory = Path(directory)
    project = directory / "main.zyl"
    project.write_text(f'''database Main {{ engine: {ENGINE} }}
        type Amount = Decimal
        table {TABLE} {{
            id: Id primary auto
            amount: Amount required
            price: Money required
            quantity: UInt required
            optional_amount: Decimal?
        }}
        form ExactAmount -> {TABLE} {{
            fields {{ amount price quantity optional_amount }}
            action save {{
                sql {{ INSERT INTO {TABLE} (amount, price, quantity, optional_amount)
                    VALUES (:amount, :price, :quantity, :optional_amount) }}
                success "Amount saved."
            }}
        }}
        crud ExactList -> {TABLE} {{
            list {{ id amount price quantity optional_amount }}
            filter {{ amount price quantity }}
        }}
        fn echo_amount(amount: Amount) -> Amount {{ return amount }}
        api POST "/exact" {{ handler echo_amount input {{ amount: Amount }} output Amount }}
        struct Packet<T> {{ item: T other: T[] }}
        fn echo_packet(packet: Packet<Decimal>) -> Packet<Decimal> {{ return packet }}
        api POST "/packet" {{ handler echo_packet input {{ packet: Packet<Decimal> }} output Packet<Decimal> }}
        fn main() uses Database {{
            amount = decimal("9007199254740993.123456789")
            rows = sql<Decimal[]> {{ SELECT amount FROM {TABLE} WHERE amount = :amount }}
            print(rows)
        }}
        ''', encoding="utf-8")
    try:
        compiler("check", project)
        compiler("db", "apply", project)
        plan = compiler("db", "plan", project, "--format=json")
        assert not json.loads(plan.stdout)["changes"], "new exact schema must have no drift"
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            port = listener.getsockname()[1]
        with (directory / "server.log").open("w+", encoding="utf-8") as log:
            server = subprocess.Popen([str(BIN), "serve", str(project), f"127.0.0.1:{port}"],
                                      stdout=log, stderr=log)
            try:
                status, body = wait_server(server, port)
                assert status == 200
                assert 'inputmode="decimal"' in body
                token = re.search(r'name="_zelyra_csrf" value="([^"]+)"', body).group(1)
                amount = "9007199254740993.123456789"
                money = "999999999999999.9999"
                for price in (money, "999999999999999.9998"):
                    payload = urlencode(dict(_zelyra_csrf=token, amount=amount, price=price,
                                             quantity="18446744073709551615", optional_amount=""))
                    status, body = request(port, "POST", "/forms/ExactAmount", payload,
                                           "application/x-www-form-urlencoded")
                    assert status == 303, (status, (directory / "server.log").read_text()[-1200:].replace(URL, "[redacted]"))
                stored = sql(f"SELECT amount, price, quantity, optional_amount IS NULL FROM {TABLE} ORDER BY id")
                assert amount in stored and money in stored and "18446744073709551615" in stored, stored
                assert len(stored.splitlines()) == 2
                assert sql(f"SELECT count(*) FROM {TABLE} WHERE optional_amount IS NULL") == "2"
                # These large values differ below Float precision. Exactly one row must match.
                status, body = request(port, "GET", "/" + TABLE + "?" + urlencode(dict(filter_price=money)))
                assert status == 200, (status, (directory / "server.log").read_text()[-1200:].replace(URL, "[redacted]"))
                assert money in body and "999999999999999.9998" not in body, body
                # Exact HTTP numeric lexemes, exponent notation and nested generic records.
                for raw, expected in [(amount, amount), ('"' + amount + '"', amount), ("1e-9", "0.000000001")]:
                    status, body = request(port, "POST", "/exact", '{"amount":' + raw + '}', "application/json")
                    assert status == 200 and json.loads(body) == expected, (status, body)
                payload = '{"packet":{"item":' + amount + ',"other":[1e-9,"0.10"]}}'
                status, body = request(port, "POST", "/packet", payload, "application/json")
                assert status == 200 and json.loads(body) == {"item": amount, "other": ["0.000000001", "0.1"]}, body
                for raw in ["1e-10", "1e999", '"NaN"', "true", "null", '"1.0000000001"']:
                    status, _ = request(port, "POST", "/exact", '{"amount":' + raw + '}', "application/json")
                    assert status == 400, raw
                for invalid, field in [("NaN", "amount"), ("inf", "price"), ("1.00001", "price"),
                                       ("0.0000000001", "amount"), ("1000000000000000", "price"), ("-1", "quantity")]:
                    fields = dict(_zelyra_csrf=token, amount=amount, price=money, quantity="1")
                    fields[field] = invalid
                    status, body = request(port, "POST", "/forms/ExactAmount", urlencode(fields),
                                           "application/x-www-form-urlencoded")
                    assert status == 422, (field, invalid, status, body)
                assert sql(f"SELECT count(*) FROM {TABLE}") == "2"
            finally:
                stop(server)
        run = compiler("run", project)
        assert amount in run.stdout, run.stdout
        # A legacy zero-scale schema must be rejected before a runtime query or HTTP startup.
        if ENGINE == "postgres":
            sql(f"ALTER TABLE {TABLE} ALTER COLUMN amount TYPE NUMERIC(39,0)")
        else:
            sql(f"ALTER TABLE {TABLE} MODIFY amount DECIMAL(39,0) NOT NULL")
        for command in [("run", project), ("serve", project, f"127.0.0.1:{port}")]:
            failure = compiler(*command, ok=False, timeout=15)
            assert "db plan" in failure.stderr and "main.zyl:" in failure.stderr, failure.stderr
        assert "E-DB-PRECISION" in failure.stderr
        print(json.dumps({"engine": ENGINE, "server_version": sql("SELECT VERSION()"), "compiler_sha256": hashlib.sha256(BIN.read_bytes()).hexdigest(),
                          "exact_forms_sql_filters_api_generics_null_and_legacy_schema": "passed"}))
    finally:
        PARTS = ORIGINAL_PARTS
        URL = ORIGINAL_URL
        sql(f"DROP DATABASE IF EXISTS {TEST_DATABASE}")
