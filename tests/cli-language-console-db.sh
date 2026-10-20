#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
test_url="${ZELYRA_REPL_TEST_URL:-${ZELYRA_DB_CLI_TEST_URL:-}}"
if [[ -z "${test_url}" ]]; then
  echo "error: ZELYRA_REPL_TEST_URL must be a local disposable MariaDB URL" >&2
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
    raise SystemExit("error: test database name must start with zelyra_ and use identifier characters")
print("\n".join((u.hostname, str(u.port or 3306), unquote(u.username or ""), unquote(u.password or ""), name)))
PY
)"
mapfile -t parsed <<<"${parts}"
db_host="${parsed[0]}"
db_port="${parsed[1]}"
db_user="${parsed[2]}"
db_password="${parsed[3]}"
db_name="${parsed[4]}"
table="zelyra_repl_probe_$$"
project="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-repl-db.XXXXXX")"
cleanup() {
  MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" --database="${db_name}" -e "DROP TABLE IF EXISTS \`${table}\`" >/dev/null 2>&1 || true
  python3 -c 'import shutil,sys; shutil.rmtree(sys.argv[1])' "${project}"
}
trap cleanup EXIT
MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" --database="${db_name}" -e "CREATE TABLE \`${table}\` (id BIGINT PRIMARY KEY AUTO_INCREMENT, tenant_id BIGINT NOT NULL, name VARCHAR(80) NOT NULL) ENGINE=InnoDB"
cat >"${project}/main.zyl" <<EOF
database main { engine: mariadb database: "${db_name}" }
table ${table} { id: Id primary auto tenant_id: Int required name: String(80) required }
fn main() uses Database {}
EOF
cat >"${project}/zelyra.toml" <<'EOF'
[capabilities]
database_read = true
database_write = true
EOF
printf 'DATABASE_URL=%s\n' "${test_url}" >"${project}/.env"

ddl_attempt="$(printf 'sql { DROP TABLE %s }\n' "${table}" | "${zelyra_bin}" console "${project}/main.zyl" --sandbox 2>&1)"
[[ "${ddl_attempt}" == *'DDL and transaction control are rejected'* ]]
table_exists="$(MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" --database="${db_name}" --batch --skip-column-names -e "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = '${table}'")"
[[ "${table_exists}" == "1" ]]

printf "sql { INSERT INTO ${table} (tenant_id, name) VALUES (:tenant_id, 'rolled-back') }\n" \
  | "${zelyra_bin}" console "${project}/main.zyl" --sandbox --tenant 7
count="$(MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" --database="${db_name}" --batch --skip-column-names -e "SELECT COUNT(*) FROM \`${table}\`")"
[[ "${count}" == "0" ]]

printf "sql { INSERT INTO ${table} (tenant_id, name) VALUES (:tenant_id, 'committed') }\n:commit\nsql { INSERT INTO ${table} (tenant_id, name) VALUES (:tenant_id, 'rolled-back-after-commit') }\n" \
  | "${zelyra_bin}" console "${project}/main.zyl" --sandbox --tenant 7
rows="$(MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" --database="${db_name}" --batch --skip-column-names -e "SELECT name FROM \`${table}\` ORDER BY id")"
[[ "${rows}" == "committed" ]]

echo "language console MariaDB sandbox checks passed"
