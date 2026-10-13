#!/usr/bin/env python3
"""Native calendar/time/UTC values across forms and real SQL storage.

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
TABLE = "calendar_values_" + uuid.uuid4().hex[:16]


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
        command = ["mysql" if ENGINE == "mysql" else "mariadb", "--protocol=tcp", "--batch", "--skip-column-names",
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


def stop(server):
    server.terminate()
    try:
        server.wait(timeout=10)
    except subprocess.TimeoutExpired:
        server.kill()
        server.wait(timeout=5)
        raise AssertionError("server failed to stop within the bound")


# Work only in a uniquely owned database.
ORIGINAL_PARTS = PARTS
ORIGINAL_URL = URL
TEST_DATABASE = "zelyra_temporal_" + uuid.uuid4().hex[:16]
sql(f"CREATE DATABASE {TEST_DATABASE}")
if ENGINE == "postgres":
    sql(f"ALTER DATABASE {TEST_DATABASE} SET TimeZone TO 'Asia/Kathmandu'")
PARTS = PARTS._replace(path="/" + TEST_DATABASE)
URL = PARTS.geturl()
os.environ["DATABASE_URL"] = URL
with tempfile.TemporaryDirectory(prefix="zelyra-temporal-sql-") as directory:
    directory = Path(directory)
    project = directory / "main.zyl"
    project.write_text(f'''database Main {{ engine: {ENGINE} }}
        table {TABLE} {{ id: Id primary auto day: Date required clock: Time required stamp: Timestamp required optional_day: Date? created: Timestamp default now }}
        form Calendar -> {TABLE} {{
            fields {{ day clock stamp optional_day }}
            action save {{ sql {{ INSERT INTO {TABLE} (day, clock, stamp, optional_day)
                VALUES (:day, :clock, :stamp, :optional_day) }} success "Saved." }}
        }}
        crud CalendarList -> {TABLE} {{ list {{ id day clock stamp optional_day }} filter {{ day clock stamp }} }}
        fn verify(day: Date, clock: Time, stamp: Timestamp) uses Database {{
            rows = sql<{TABLE}[]> {{ SELECT id, day, clock, stamp, optional_day FROM {TABLE}
                WHERE day = :day AND clock = :clock AND stamp = :stamp }}
            print(rows)
        }}
        fn main() uses Database {{ verify(date("2000-02-29"), time("12:34:56"), json_decode<Timestamp>("2182550400001")) }}
        ''', encoding="utf-8")
    try:
        compiler("check", project)
        compiler("db", "apply", project)
        assert not json.loads(compiler("db", "plan", project, "--format=json").stdout)["changes"]
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0)); port = listener.getsockname()[1]
        with (directory / "server.log").open("w", encoding="utf-8") as log:
            server = subprocess.Popen([str(BIN), "serve", str(project), f"127.0.0.1:{port}"], stdout=log, stderr=log)
            try:
                deadline = time.monotonic() + 15
                while True:
                    assert server.poll() is None, "server exited before readiness: " + (directory/"server.log").read_text().replace(URL, "[test database URL]")
                    try:
                        status, body = request(port, "GET", "/forms/Calendar")
                        break
                    except OSError:
                        assert time.monotonic() < deadline
                        time.sleep(.05)
                assert status == 200
                token = re.search(r'name="_zelyra_csrf" value="([^"]+)"', body).group(1)
                values = dict(_zelyra_csrf=token, day="2000-02-29", clock="12:34:56", stamp="2039-03-01T00:00:00.001Z", optional_day="")
                for stamp in [values["stamp"], "1969-12-31T23:59:59.999Z", "2039-03-01T00:00:00.000Z"]:
                    status, body = request(port, "POST", "/forms/Calendar", urlencode(dict(values, stamp=stamp)), "application/x-www-form-urlencoded")
                    assert status == 303, (status, (directory/"server.log").read_text()[-1000:])
                stored = sql(f"SELECT day, clock, stamp FROM {TABLE} ORDER BY id")
                assert "2000-02-29" in stored and "12:34:56" in stored and ".001" in stored and ".999" in stored, stored
                assert sql(f"SELECT count(*) FROM {TABLE} WHERE optional_day IS NULL") == "3"
                assert sql(f"SELECT count(*) FROM {TABLE} WHERE created IS NOT NULL") == "3"
                for query in [dict(filter_day="2000-02-29"), dict(filter_clock="12:34:56"), dict(filter_stamp=values["stamp"])]:
                    status, body = request(port, "GET", "/" + TABLE + "?" + urlencode(query))
                    assert status == 200, (status, (directory/"server.log").read_text()[-1000:])
                    assert "2000-02-29" in body
                for query in [dict(filter_day="2001-03-01"), dict(filter_clock="00:00:01"), dict(filter_stamp="2040-01-01T00:00:00.001Z")]:
                    status, body = request(port, "GET", "/" + TABLE + "?" + urlencode(query))
                    assert status == 200, (status, body[:500])
                    assert "2000-02-29" not in body, "nonmatching filter returned rows"
                for field, bad in [("day", "1900-02-29"), ("day", "2026-02-30"), ("clock", "24:00:00"), ("stamp", "99999-01-01T00:00:00Z")]:
                    status, body = request(port, "POST", "/forms/Calendar", urlencode(dict(values, **{field:bad})), "application/x-www-form-urlencoded")
                    assert status == 422, (field, bad, status)
                assert sql(f"SELECT count(*) FROM {TABLE}") == "3"
            finally:
                stop(server)
        output = compiler("run", project).stdout
        assert "2000-02-29" in output and "12:34:56" in output and "2039-03-01" in output and ".001" in output, output
        if ENGINE == "postgres": sql(f"ALTER TABLE {TABLE} ALTER COLUMN stamp TYPE TIMESTAMPTZ(0)")
        else: sql(f"ALTER TABLE {TABLE} MODIFY stamp DATETIME NOT NULL")
        failure = compiler("serve", project, f"127.0.0.1:{port}", ok=False)
        assert "E-DB-TEMPORAL" in failure.stderr and "db plan" in failure.stderr, failure.stderr
        assert f"{project.name}:2:" in failure.stderr and f"{TABLE}.stamp" in failure.stderr, failure.stderr
        print(json.dumps({"engine": ENGINE, "server_version": sql("SELECT VERSION()"),
                          "compiler_sha256":hashlib.sha256(BIN.read_bytes()).hexdigest(),
                          "native_temporal_forms_filters_parameters_nulls_precision": "passed"}))
    finally:
        PARTS = ORIGINAL_PARTS; URL = ORIGINAL_URL
        sql(f"DROP DATABASE IF EXISTS {TEST_DATABASE}")
