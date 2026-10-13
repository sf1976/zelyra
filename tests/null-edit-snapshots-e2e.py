#!/usr/bin/env python3
"""Native NULL/literal-text optimistic edits and rendering on an owned DB.

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
if ORIGINAL.scheme not in ("mariadb", "mysql"):
    raise SystemExit("DATABASE_URL must identify a disposable MariaDB/MySQL database")
ENGINE = "postgres" if ORIGINAL.scheme in ("postgres", "postgresql") else ORIGINAL.scheme
PARTS = ORIGINAL
DATABASE = "zelyra_edit_nulls_" + uuid.uuid4().hex[:16]


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


class Inputs(__import__('html.parser', fromlist=['HTMLParser']).HTMLParser):
    def __init__(self, text):
        super().__init__(); self.fields = {}; self.feed(text)
    def handle_starttag(self, tag, attributes):
        attrs = dict(attributes)
        if tag == 'input' and 'name' in attrs:
            self.fields[attrs['name']] = attrs.get('value', '')


def request(port, method, path, fields=None):
    from urllib.parse import urlencode
    connection = http.client.HTTPConnection('127.0.0.1', port, timeout=5)
    headers = {'Origin': f'http://127.0.0.1:{port}'}
    body = None
    if fields is not None:
        headers['Content-Type'] = 'application/x-www-form-urlencoded'
        body = urlencode(fields)
    connection.request(method, path, body, headers)
    response = connection.getresponse()
    status, body = response.status, response.read().decode()
    connection.close()
    return status, body


sql(f"CREATE DATABASE {DATABASE}")
PARTS = ORIGINAL._replace(path='/' + DATABASE)
os.environ['DATABASE_URL'] = PARTS.geturl()
try:
    with tempfile.TemporaryDirectory(prefix='zelyra-edit-nulls-') as directory:
        project = Path(directory) / 'main.zyl'
        project.write_text(f"""database Main {{ engine: {ENGINE} }}
            table notes {{ id: Id primary auto title: String(100) required note: String(100) }}
            crud Note -> notes {{ list {{ id title note }} }}
            fn main() {{ }}
        """)
        compiler('check', project)
        compiler('db', 'apply', project)
        sql("INSERT INTO notes (title,note) VALUES ('original',NULL)")
        with socket.socket() as listener:
            listener.bind(('127.0.0.1',0)); port = listener.getsockname()[1]
        log_path = Path(directory) / 'server.log'
        with log_path.open('w') as log:
            server = subprocess.Popen([str(BIN),'serve',str(project),f'127.0.0.1:{port}'],stdout=log,stderr=log)
            try:
                deadline = time.monotonic() + 15
                while True:
                    try:
                        status, page = request(port,'GET','/notes/1/edit')
                        assert status == 200, (status,page)
                        break
                    except (OSError, http.client.HTTPException):
                        if server.poll() is not None or time.monotonic() > deadline: raise AssertionError(log_path.read_text())
                        time.sleep(.1)
                original_inputs = Inputs(page).fields
                assert '_zelyra_snapshot' in original_inputs and '_zelyra_csrf' in original_inputs
                # First demonstrate the stale-edit vulnerability independently of UI.
                sql("UPDATE notes SET note=CONCAT(CHAR(92),'N') WHERE id=1")
                status, body = request(port,'POST','/notes/1/edit',dict(original_inputs,title='overwritten',note='posted'))
                assert status == 409, f'NULL-to-literal stale edit must conflict, got {status}; {log_path.read_text()}'
                assert sql("SELECT title FROM notes WHERE id=1") == 'original'
                assert original_inputs['note'] == '', 'real NULL must render as an empty input'
                status, page = request(port,'GET','/notes/1/edit'); assert status==200
                literal_inputs = Inputs(page).fields
                assert literal_inputs['note'] == chr(92)+'N', 'literal sentinel must remain visible'
                assert literal_inputs['_zelyra_snapshot'] != original_inputs['_zelyra_snapshot']
                # Unchanged literal values can be saved without becoming SQL NULL.
                status, _ = request(port,'POST','/notes/1/edit',dict(literal_inputs,title='literal saved'))
                assert status==303, status
                assert sql("SELECT note IS NULL FROM notes WHERE id=1")=='0'
                # Reverse transition also conflicts and preserves the untouched title.
                status, page = request(port,'GET','/notes/1/edit'); assert status==200
                literal_inputs = Inputs(page).fields
                sql('UPDATE notes SET note=NULL WHERE id=1')
                status, _ = request(port,'POST','/notes/1/edit',dict(literal_inputs,title='reverse overwritten'))
                assert status==409, status
                assert sql("SELECT title FROM notes WHERE id=1")=='literal saved'
                # An unchanged native NULL can still be edited successfully.
                status, page = request(port,'GET','/notes/1/edit'); assert status==200
                null_inputs = Inputs(page).fields
                status, _ = request(port,'POST','/notes/1/edit',dict(null_inputs,title='NULL saved'))
                assert status==303, status
                assert sql('SELECT note IS NULL FROM notes WHERE id=1')=='1'
                # Reusing a snapshot after an ordinary content change is denied.
                status, _ = request(port,'POST','/notes/1/edit',dict(null_inputs,title='stale'))
                assert status==409, status
            finally:
                server.terminate()
                try: server.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    server.kill(); server.wait(timeout=5); raise AssertionError('server shutdown timeout')
        print(json.dumps({'engine':ENGINE,'server_version':sql('SELECT VERSION()'),'compiler_sha256':hashlib.sha256(BIN.read_bytes()).hexdigest(),'nullable_edit_snapshots_and_rendering':'passed','controlled_stale_edits':3}))
finally:
    PARTS = ORIGINAL
    sql(f'DROP DATABASE IF EXISTS {DATABASE}')
