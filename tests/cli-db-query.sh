#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
test_url="${ZELYRA_DB_CLI_TEST_URL:-}"
if [[ -z "${test_url}" ]]; then
  echo "error: ZELYRA_DB_CLI_TEST_URL must point to a local disposable zelyra_* database server" >&2
  exit 2
fi
parts="$(python3 - "${test_url}" <<'PY'
import sys
from urllib.parse import urlsplit, unquote
u = urlsplit(sys.argv[1])
if u.scheme not in ("mysql", "mariadb") or u.hostname not in ("127.0.0.1", "localhost"):
    raise SystemExit("error: test URL must use local MariaDB/MySQL")
name = u.path.lstrip("/")
if not name.startswith("zelyra_") or not name.replace("_", "").isalnum():
    raise SystemExit("error: test database name must be a zelyra_* identifier")
print("\n".join((u.hostname, str(u.port or 3306), unquote(u.username or ""), unquote(u.password or ""), name)))
PY
)"
mapfile -t parsed <<<"${parts}"
db_host="${parsed[0]}"
db_port="${parsed[1]}"
db_user="${parsed[2]}"
db_password="${parsed[3]}"
base_db="${parsed[4]}"
db_name="${base_db}_cli_e2e_$$"
project="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-db-cli.XXXXXX")"
cleanup() {
  MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" --batch --skip-column-names -e "DROP DATABASE IF EXISTS \`${db_name}\`" >/dev/null 2>&1 || true
  rm -rf "${project}"
}
trap cleanup EXIT
MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" --batch --skip-column-names -e "CREATE DATABASE \`${db_name}\`; CREATE TABLE \`${db_name}\`.items (id INT PRIMARY KEY, name VARCHAR(100)); INSERT INTO \`${db_name}\`.items VALUES (1, 'a,b');"
cat >"${project}/main.zyl" <<EOF
database main { engine: mariadb database: "${db_name}" }
EOF
cat >"${project}/.env" <<EOF
DATABASE_URL=mariadb://${db_user}:${db_password}@${db_host}:${db_port}/${db_name}
EOF

json_output="$("${zelyra_bin}" db query "${project}/main.zyl" "SELECT count(*) AS total FROM items" --format json)"
[[ "${json_output}" == '[{"total":1}]' ]]
table_output="$("${zelyra_bin}" db query "${project}/main.zyl" "SELECT id, name FROM items" --format table)"
[[ "${table_output}" == *"| 1  | a,b  |"* ]]
limited_output="$("${zelyra_bin}" db query "${project}/main.zyl" "SELECT id, name FROM items ORDER BY id" --format json --limit 1)"
[[ "${limited_output}" == '[{"id":1,"name":"a,b"}]' ]]
tenant_output="$("${zelyra_bin}" db query "${project}/main.zyl" "SELECT :tenant_id AS tenant_id" --tenant 42 --format json)"
[[ "${tenant_output}" == '[{"tenant_id":42}]' ]]
csv_output="$("${zelyra_bin}" db query "${project}/main.zyl" "SELECT name FROM items" --format csv)"
[[ "${csv_output}" == $'name\r\n"a,b"\r' ]]
if "${zelyra_bin}" db query "${project}/main.zyl" "INSERT INTO items VALUES (2, 'blocked')" >/dev/null 2>&1; then
  echo "error: query accepted a mutation without --allow-mutation" >&2
  exit 1
fi
"${zelyra_bin}" db query "${project}/main.zyl" "INSERT INTO items VALUES (2, 'committed')" --allow-mutation >/dev/null
if "${zelyra_bin}" db query "${project}/main.zyl" "INSERT INTO items VALUES (1, 'duplicate')" --allow-mutation >/dev/null 2>&1; then
  echo "error: failed DML unexpectedly committed" >&2
  exit 1
fi
after_rollback="$("${zelyra_bin}" db query "${project}/main.zyl" "SELECT count(*) AS total FROM items" --format json)"
[[ "${after_rollback}" == '[{"total":2}]' ]]

echo "db query integration checks passed"
