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
[[ "${table_output}" == *"| 1  | a,b |"* ]]
csv_output="$("${zelyra_bin}" db query "${project}/main.zyl" "SELECT name FROM items" --format csv)"
[[ "${csv_output}" == $'name\n"a,b"' ]]
if "${zelyra_bin}" db query "${project}/main.zyl" "INSERT INTO items VALUES (2, 'blocked')" >/dev/null 2>&1; then
  echo "error: query accepted a mutation without --allow-mutation" >&2
  exit 1
fi
"${zelyra_bin}" db query "${project}/main.zyl" "INSERT INTO items VALUES (2, 'committed')" --allow-mutation >/dev/null

console_output="$(printf 'SELECT 1;\n' | "${zelyra_bin}" db console "${project}/main.zyl")"
[[ "${console_output}" == *"1"* ]]

# A fake client checks the credential transport contract without logging the
# secret in process arguments. The real client pipe above verifies passthrough.
fake_bin="${project}/fake-bin"
mkdir -p "${fake_bin}"
cat >"${fake_bin}/mariadb" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$@" >"${ZELYRA_TEST_ARGS}"
printf '%s' "${MYSQL_PWD-}" >"${ZELYRA_TEST_PASSWORD}"
cat >/dev/null
EOF
chmod +x "${fake_bin}/mariadb"
ZELYRA_TEST_ARGS="${project}/args" ZELYRA_TEST_PASSWORD="${project}/password" PATH="${fake_bin}:${PATH}" \
  "${zelyra_bin}" db console "${project}/main.zyl" </dev/null >/dev/null
! grep -Fq -- "${db_password}" "${project}/args"
[[ "$(cat "${project}/password")" == "${db_password}" ]]

echo "db console and query integration checks passed"
