#!/usr/bin/env bash
set -euo pipefail

zelyra_bin="${ZELYRA_BIN:-target/debug/zelyra}"
database_url="${ZELYRA_STUDIO_E2E_DATABASE_URL:-}"
backend="${ZELYRA_STUDIO_E2E_BACKEND:-mariadb}"
database_port="${ZELYRA_STUDIO_E2E_DATABASE_PORT:-$([[ "$backend" == "mariadb" ]] && echo 3306 || echo 5432)}"
if [[ "$backend" != "mariadb" && "$backend" != "postgres" ]]; then
  echo "ZELYRA_STUDIO_E2E_BACKEND must be mariadb or postgres" >&2
  exit 2
fi
if [[ -z "$database_url" || ( "$backend" == "mariadb" && "$database_url" != mariadb://* ) || ( "$backend" == "postgres" && "$database_url" != postgres://* && "$database_url" != postgresql://* ) ]]; then
  echo "ZELYRA_STUDIO_E2E_DATABASE_URL must point to a disposable $backend test database" >&2
  exit 2
fi
if [[ ! -x "$zelyra_bin" ]]; then
  echo "Zelyra CLI binary is not executable: $zelyra_bin" >&2
  exit 2
fi

work_root="$(mktemp -d)"
database_name="zelyra_studio_e2e_$$"
database_created=0
server_pid=""
export MYSQL_PWD="${ZELYRA_STUDIO_E2E_ROOT_PASSWORD:-root}"
export PGPASSWORD="${ZELYRA_STUDIO_E2E_POSTGRES_PASSWORD:-zelyra_ci}"

cleanup() {
  if [[ -n "$server_pid" ]]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  if [[ "$database_created" -eq 1 ]]; then
    if [[ "$backend" == "mariadb" ]]; then
      mariadb --protocol=tcp --host=127.0.0.1 --port="$database_port" --user=root \
        --batch --skip-column-names -e "DROP DATABASE IF EXISTS \`$database_name\`" >/dev/null
    else
      psql --host=127.0.0.1 --port="$database_port" --username=zelyra_ci --dbname=zelyra_ci \
        --command="DROP DATABASE IF EXISTS \"$database_name\"" >/dev/null
    fi
  fi
  rm -rf "$work_root"
}
trap cleanup EXIT

database_url="${database_url%/*}/$database_name"
if [[ "$backend" == "mariadb" ]]; then
  mariadb --protocol=tcp --host=127.0.0.1 --port="$database_port" --user=root \
    --batch --skip-column-names \
    -e "CREATE DATABASE \`$database_name\`; CREATE TABLE \`$database_name\`.departments (id BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY, name VARCHAR(80) NOT NULL) ENGINE=InnoDB; CREATE TABLE \`$database_name\`.customers (id BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY, department_id BIGINT NOT NULL, name VARCHAR(120) NOT NULL, legacy_code VARCHAR(20) NULL, CONSTRAINT fk_studio_customer_department FOREIGN KEY (department_id) REFERENCES \`$database_name\`.departments(id)) ENGINE=InnoDB"
  engine="mariadb"
else
  psql --host=127.0.0.1 --port="$database_port" --username=zelyra_ci --dbname=zelyra_ci \
    --command="CREATE DATABASE \"$database_name\"" >/dev/null
  psql --host=127.0.0.1 --port="$database_port" --username=zelyra_ci --dbname="$database_name" \
    --command='CREATE TABLE departments (id BIGSERIAL PRIMARY KEY, name VARCHAR(80) NOT NULL); CREATE TABLE customers (id BIGSERIAL PRIMARY KEY, department_id BIGINT NOT NULL REFERENCES departments(id), name VARCHAR(120) NOT NULL, legacy_code VARCHAR(20))' >/dev/null
  engine="postgres"
fi
database_created=1

project="$work_root/project"
mkdir -p "$project"
mkdir -p "$project/src"
cat >"$project/src/math.zyl" <<'ZELYRA'
pub fn sum(left: Int, right: Int) -> Int {
    return left + right
}
ZELYRA
cat >"$project/main.zyl" <<ZELYRA
import "src/math.zyl" as math

database main {
    engine: $engine
}

table departments {
    id: Id primary auto
    name: String(80) required
}

table customers {
    id: Id primary auto
    department: Department required
    name: String(120) required
    description: String?
}

fn main() {
    print(math::sum(1, 2))
}
ZELYRA

port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()')"
DATABASE_URL="$database_url" "$zelyra_bin" editor "$project" --port "$port" \
  >"$work_root/editor.log" 2>&1 &
server_pid=$!

for _ in $(seq 1 100); do
  if grep -q '/__zelyra/editor/#' "$work_root/editor.log"; then break; fi
  if ! kill -0 "$server_pid" 2>/dev/null; then
    sed -E 's/(#__zelyra\/editor\/#)[0-9a-f]+/\1[redacted]/g' "$work_root/editor.log" >&2
    exit 1
  fi
  sleep 0.1
done

token="$(python3 - "$work_root/editor.log" <<'PY'
import re
import sys
text = open(sys.argv[1], encoding="utf-8").read()
match = re.search(r"/__zelyra/editor/#([0-9a-f]{64})", text)
if not match:
    raise SystemExit("editor did not print a session URL")
print(match.group(1))
PY
)"

ZELYRA_STUDIO_E2E_PORT="$port" \
ZELYRA_STUDIO_E2E_TOKEN="$token" \
ZELYRA_STUDIO_E2E_PROJECT="$project" \
ZELYRA_STUDIO_E2E_DATABASE="$database_name" \
ZELYRA_STUDIO_E2E_BACKEND="$backend" \
python3 <<'PY'
import json
import os
import urllib.error
import urllib.parse
import urllib.request

port = os.environ["ZELYRA_STUDIO_E2E_PORT"]
token = os.environ["ZELYRA_STUDIO_E2E_TOKEN"]
project = os.environ["ZELYRA_STUDIO_E2E_PROJECT"]
database = os.environ["ZELYRA_STUDIO_E2E_DATABASE"]
base = f"http://127.0.0.1:{port}/__zelyra/editor"

def request(path, method="GET", body=None, expect=200):
    headers = {"X-Zelyra-Editor-Token": token}
    data = None
    if body is not None:
        data = json.dumps(body).encode()
        headers["Content-Type"] = "application/json"
        headers["Origin"] = f"http://127.0.0.1:{port}"
    req = urllib.request.Request(base + "/api/" + path, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=10) as response:
            status, content = response.status, response.read()
    except urllib.error.HTTPError as error:
        status, content = error.code, error.read()
    if status != expect:
        raise SystemExit(f"{method} {path} expected HTTP {expect}, got {status}: {content.decode(errors='replace')}")
    return json.loads(content)

schema = request("database-schema")
tables = [table for module in schema["modules"] for table in module["tables"]]
customers = next((table for table in tables if table["name"] == "customers"), None)
assert schema["read_only"] is True
assert customers is not None and customers["status"] == "matched"
declared = {column["name"] for column in customers["declared_columns"]}
live = {column["name"] for column in customers["live_columns"]}
assert "description" in declared and "description" not in live
assert "legacy_code" in live and "legacy_code" not in declared
assert any(key["references_table"] == "departments" for key in customers["foreign_keys"])
assert any(key["references_table"] == "departments" for key in customers["live_foreign_keys"])

project_map = request("project-map")
assert project_map == request("project-map"), "project map output must be deterministic"
math_function = next(
    node for node in project_map["nodes"]
    if node["kind"] == "function" and node["label"] == "sum"
)
assert math_function["path"] == "src/math.zyl" and math_function["line"] == 1
assert isinstance(math_function["position"], int)
assert any(
    edge["from"] == "module:main.zyl"
    and edge["to"] == "module:src/math.zyl"
    and edge["label"] == "imports"
    for edge in project_map["edges"]
), "project map must include the imported module edge"

path = "file?path=" + urllib.parse.quote("main.zyl")
original = request(path)
payload = {"path": "main.zyl", "baseHash": original["hash"], "table": "customers", "fields": ["name"], "resourceName": "Customer", "title": "Customer"}
preview = request("crud-preview", "POST", payload)
assert preview["success"] is True, preview["diagnostics"]
assert "crud Customer -> customers" in preview["snippet"]
impact = preview["impact"]
assert impact["files"] == ["main.zyl"]
assert impact["tables"] == ["customers"]
assert any(route["path"] == "/customers/{id}/edit" for route in impact["routes"])
assert any(relation["toTable"] == "departments" for relation in impact["relationships"])
assert open(os.path.join(project, "main.zyl"), encoding="utf-8").read() == original["content"]

with open(os.path.join(project, "main.zyl"), "a", encoding="utf-8") as source:
    source.write("\n// external edit after preview\n")
request("crud-apply", "POST", payload, expect=409)
after_conflict = request(path)
assert "external edit after preview" in after_conflict["content"]

fresh = {**payload, "baseHash": after_conflict["hash"]}
preview = request("crud-preview", "POST", fresh)
assert preview["success"] is True
applied = request("crud-apply", "POST", fresh)
assert applied["applied"] is True
assert "crud Customer -> customers" in applied["content"]

with open(os.path.join(project, "main.zyl"), encoding="utf-8") as source:
    assert "crud Customer -> customers" in source.read()
PY

if [[ "${ZELYRA_STUDIO_BROWSER_E2E:-0}" == "1" ]]; then
  ZELYRA_STUDIO_E2E_PORT="$port" ZELYRA_STUDIO_E2E_TOKEN="$token" \
    node editor/tests/editor-browser-e2e.mjs
fi

if [[ "$backend" == "mariadb" ]]; then
  table_count="$(mariadb --protocol=tcp --host=127.0.0.1 --port="$database_port" --user=root \
    --batch --skip-column-names "$database_name" -e 'SELECT COUNT(*) FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE()')"
  row_count="$(mariadb --protocol=tcp --host=127.0.0.1 --port="$database_port" --user=root \
    --batch --skip-column-names "$database_name" -e 'SELECT COUNT(*) FROM customers')"
else
  table_count="$(psql --host=127.0.0.1 --port="$database_port" --username=zelyra_ci --dbname="$database_name" --tuples-only --no-align --command="SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public'")"
  row_count="$(psql --host=127.0.0.1 --port="$database_port" --username=zelyra_ci --dbname="$database_name" --tuples-only --no-align --command='SELECT COUNT(*) FROM customers')"
  table_count="$(xargs <<<"$table_count")"
  row_count="$(xargs <<<"$row_count")"
fi
[[ "$table_count" == 2 && "$row_count" == 0 ]]
echo "Zelyra Studio $backend schema, compiler preview, stale-file conflict, and explicit apply passed"
