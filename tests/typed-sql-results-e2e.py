#!/usr/bin/env python3
"""Opt-in SQL results and legacy HTTP/JSON compatibility on a disposable DB.

Uses the existing DATABASE_URL/ZELYRA_BIN test contract. Owns one random test
schema, creates no production tables, and removes only that schema in finally.
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
from urllib.parse import unquote, urlsplit
import uuid

ROOT = Path(__file__).resolve().parents[1]
BIN = Path(os.environ.get("ZELYRA_BIN", ROOT / "target/debug/zelyra")).resolve()
ORIGINAL = urlsplit(os.environ.get("DATABASE_URL", ""))
if ORIGINAL.scheme not in ("mariadb", "mysql", "postgres", "postgresql"):
    raise SystemExit("DATABASE_URL must identify a disposable MariaDB/MySQL/PostgreSQL database")
ENGINE = "postgres" if ORIGINAL.scheme in ("postgres", "postgresql") else ORIGINAL.scheme
PARTS = ORIGINAL
DATABASE = "zelyra_typed_sql_" + uuid.uuid4().hex[:16]


def sql(statement):
    env = os.environ.copy()
    user = unquote(PARTS.username or "")
    password = unquote(PARTS.password or "")
    if ENGINE == "postgres":
        env["PGPASSWORD"] = password
        command = ["psql", "-X", "-At", "-h", PARTS.hostname, "-p", str(PARTS.port or 5432), "-U", user, "-d", PARTS.path.lstrip("/"), "-c", statement]
    else:
        env["MYSQL_PWD"] = password
        client = shutil.which("mysql") or shutil.which("mariadb")
        if not client:
            raise AssertionError("MySQL/MariaDB test client is missing")
        command = [client, "--protocol=tcp", "--batch", "--skip-column-names", "--host=" + PARTS.hostname, "--port=" + str(PARTS.port or 3306), "--user=" + user, PARTS.path.lstrip("/"), "-e", statement]
    result = subprocess.run(command, env=env, capture_output=True, text=True, timeout=30)
    if result.returncode:
        raise AssertionError("SQL fixture failed: " + result.stderr.replace(password, "[redacted]")[:500])
    return result.stdout.strip()


def compiler(*arguments, ok=True):
    result = subprocess.run([str(BIN), *map(str, arguments)], capture_output=True, text=True, timeout=30)
    if ok and result.returncode:
        raise AssertionError(result.stderr.replace(PARTS.geturl(), "[test database URL]"))
    if not ok:
        assert result.returncode == 1, "negative result must be controlled, not an abort"
        assert "main.zyl:" in result.stderr, result.stderr
    return result


def get(port, path):
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
    connection.request("GET", path)
    response = connection.getresponse()
    status, body = response.status, response.read().decode()
    connection.close()
    assert status == 200, (status, body)
    return json.loads(body)


sql(f"CREATE DATABASE {DATABASE}")
if ENGINE == "postgres":
    sql(f"ALTER DATABASE {DATABASE} SET TimeZone TO 'Asia/Kathmandu'")
PARTS = ORIGINAL._replace(path="/" + DATABASE)
os.environ["DATABASE_URL"] = PARTS.geturl()
try:
    with tempfile.TemporaryDirectory(prefix="zelyra-typed-sql-") as directory:
        directory = Path(directory)
        project = directory / "main.zyl"
        definitions = f'''database Main {{ engine: {ENGINE} }}
        type Amount = Decimal
        struct Snapshot {{ day: Date clock: Time stamp: Timestamp amount: Amount quantity: UInt price: Money note: Option<String> }}
        table events {{ id: Id primary auto day: Date required clock: Time required stamp: Timestamp required amount: Amount required quantity: UInt required price: Money required note: String(100) approximate: Float required }}
        '''
        projection = "day, clock, stamp, amount, quantity, price, note"
        body = f'''
        fn typed() -> Snapshot[] uses Database(read) {{ return sql_decode<Snapshot[]>(sql<events[]> {{ SELECT {projection} FROM events ORDER BY id }}) }}
        fn legacy() -> events[] uses Database(read) {{ return sql<events[]> {{ SELECT {projection} FROM events ORDER BY id }} }}
        fn identifier() -> Id uses Database(read) {{ return sql_decode<Id>(sql<Int> {{ SELECT id FROM events WHERE id = 1 }}) }}
        fn money() -> Money uses Database(read) {{ return sql_decode<Money>(sql<Decimal> {{ SELECT price FROM events WHERE id = 1 }}) }}
        api GET "/typed" {{ handler typed output Snapshot[] }}
        api GET "/legacy" {{ handler legacy output events[] }}
        fn main() uses Database(read) {{
            rows = typed()
            assert(len(rows) == 2, "array cardinality")
            assert(json_decode<Snapshot>(json_encode(rows[0])) == rows[0], "typed DTO JSON roundtrip")
            assert(rows[0].day == date("2000-02-29"), "typed Date")
            assert(rows[0].clock == time("00:00:00"), "typed Time")
            assert(rows[0].stamp == json_decode<Timestamp>("-1"), "pre-1970 millisecond")
            assert(rows[1].stamp == json_decode<Timestamp>("2182550400001"), "post-2038 millisecond")
            assert(rows[0].quantity == json_decode<UInt>("18446744073709551615"), "exact UInt maximum")
            assert(rows[0].amount + decimal("0.000000001") == decimal("119.000000001"), "exact Decimal arithmetic")
            match rows[0].note {{ None => {{ print("native NULL") }} Some(_) => {{ assert(false, "NULL confused with text") }} }}
            match rows[1].note {{ Some(text) => {{ assert(len(text_chars(text)) == 2, "literal sentinel") }} None => {{ assert(false, "literal confused with NULL") }} }}
            match date_add_days_result(rows[0].day, 1) {{ Ok(day) => {{ assert(day == date("2000-03-01"), "downstream Date arithmetic") }} Err(_) => {{ assert(false, "Date calculation") }} }}
            match timestamp_difference_result(rows[1].stamp, rows[0].stamp) {{ Ok(delta) => {{ assert(delta == duration_ms(2182550400002), "exact UTC difference") }} Err(_) => {{ assert(false, "duration calculation") }} }}
            print(identifier())
            print(money())
            absent = sql_decode<Option<Int>>(sql<Int> {{ SELECT id FROM events WHERE id = 999 }})
            match absent {{ None => {{ print("no row") }} Some(_) => {{ assert(false, "optional cardinality") }} }}
            empty = sql_decode<Snapshot[]>(sql<events[]> {{ SELECT {projection} FROM events WHERE id = 999 }})
            assert(len(empty) == 0, "empty typed array")
            table_rows = sql_decode<events[]>(sql<events[]> {{ SELECT id, {projection}, approximate FROM events ORDER BY id }})
            assert(table_rows[0].day == date("2000-02-29"), "synthesized table field")
            print(json_encode(rows))
            print(json_encode(legacy()))
            print("typed SQL results passed")
        }}'''
        project.write_text(definitions + body)
        compiler("check", project)
        compiler("db", "apply", project)
        zone = "+00" if ENGINE == "postgres" else ""
        # No string escaping ambiguity: literal backslash-N is built from codepoints.
        literal = "chr(92) || 'N'" if ENGINE == "postgres" else "CONCAT(CHAR(92), 'N')"
        sql(f"INSERT INTO events (day,clock,stamp,amount,quantity,price,note,approximate) VALUES ('2000-02-29','00:00:00','1969-12-31 23:59:59.999{zone}',119.000000000,18446744073709551615,19.9900,NULL,0.1),('2039-03-01','12:34:56','2039-03-01 00:00:00.001{zone}',-0.000000001,0,-0.0001,{literal},0.2)")
        output = compiler("run", project).stdout
        assert "typed SQL results passed" in output
        encoded = [json.loads(line) for line in output.splitlines() if line.startswith("[")]
        typed, legacy = encoded
        assert typed[0]["note"] is None and typed[1]["note"] == "\\N"
        assert typed[0]["stamp"] == -1 and typed[1]["stamp"] == 2182550400001
        assert typed[0]["quantity"] == 18446744073709551615
        assert typed[0]["amount"] == "119" and typed[0]["price"] == "19.99"
        assert legacy[0]["note"] == legacy[1]["note"] == "\\N"
        assert all(isinstance(value, str) for row in legacy for value in row.values())
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            port = listener.getsockname()[1]
        with (directory / "server.log").open("w") as log:
            server = subprocess.Popen([str(BIN), "serve", str(project), f"127.0.0.1:{port}"], stdout=log, stderr=log)
            try:
                deadline = time.monotonic() + 15
                while True:
                    try:
                        assert get(port, "/typed") == typed
                        break
                    except (OSError, http.client.HTTPException):
                        if server.poll() is not None or time.monotonic() > deadline:
                            raise AssertionError((directory / "server.log").read_text())
                        time.sleep(0.1)
                assert get(port, "/legacy") == legacy
            finally:
                server.terminate()
                try:
                    server.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    server.kill()
                    server.wait(timeout=5)
                    raise AssertionError("server failed to stop within bound")
        negatives = [
            ("Int", "SELECT id FROM events WHERE id = 999", "exactly one row"),
            ("Int", "SELECT id FROM events", "exactly one row"),
            ("Option<Int>", "SELECT id FROM events", "at most one row"),
            ("Int", "SELECT id, quantity FROM events WHERE id = 1", "exactly one column"),
            ("Decimal", "SELECT approximate FROM events WHERE id = 1", "native type Float"),
            ("UInt", "SELECT amount FROM events WHERE id = 2", "row 1, column"),
            ("Date", "SELECT note FROM events WHERE id = 2", "native type Text"),
            ("String", "SELECT note FROM events WHERE id = 1", "row 1, column"),
            ("Snapshot[]", "SELECT day FROM events WHERE id = 999", "missing column"),
            ("Int[]", "SELECT id AS value, id AS value FROM events", "duplicate column"),
            ("Money", "SELECT amount FROM events WHERE id = 2", "row 1, column"),
        ]
        for target, query, reason in negatives:
            project.write_text(definitions + f"fn main() uses Database(read) {{ value = sql_decode<{target}>(sql<events[]> {{ {query} }}) print(value) }}")
            compiler("check", project)
            failure = compiler("run", project, ok=False)
            assert reason in failure.stderr, (target, failure.stderr)
        print(json.dumps({"engine": ENGINE, "server_version": sql("SELECT VERSION()"), "compiler_sha256": hashlib.sha256(BIN.read_bytes()).hexdigest(), "typed_sql_results": "passed", "legacy_http_json": "unchanged", "controlled_negative_cases": len(negatives)}))
finally:
    PARTS = ORIGINAL
    sql(f"DROP DATABASE IF EXISTS {DATABASE}")
