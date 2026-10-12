#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${DATABASE_URL:-}" || "${DATABASE_URL}" != mysql://* ]]; then
    echo "DATABASE_URL must point to a disposable MySQL test database using mysql://" >&2
    exit 2
fi
zelyra_bin="${ZELYRA_BIN:-target/debug/zelyra}"
[[ -x "${zelyra_bin}" ]] || { echo "Zelyra binary not found at ${zelyra_bin}" >&2; exit 2; }
for command in mysql python3; do
    command -v "${command}" >/dev/null 2>&1 || { echo "${command} is required" >&2; exit 2; }
done

read -r mysql_host mysql_port mysql_user database_prefix < <(python3 - "${DATABASE_URL}" <<'PY'
import sys
from urllib.parse import urlsplit
url = urlsplit(sys.argv[1])
if url.scheme != "mysql" or not url.hostname or not url.username:
    raise SystemExit("invalid MySQL test URL")
print(url.hostname, url.port or 3306, url.username, sys.argv[1].rsplit("/", 1)[0])
PY
)
export MYSQL_PWD="$(python3 - "${DATABASE_URL}" <<'PY'
import sys
from urllib.parse import unquote, urlsplit
url = urlsplit(sys.argv[1])
print(unquote(url.password or ""))
PY
)"
database_name="zelyra_mysql_schema_$$"
database_url="${database_prefix}/${database_name}"
work_root="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-mysql-schema-e2e.XXXXXX")"
project="${work_root}/project"
mkdir -p "${project}"

mysql_admin() {
    mysql --protocol=tcp --host="${mysql_host}" --port="${mysql_port}" --user="${mysql_user}" --database=mysql --batch --skip-column-names "$@"
}
cleanup() {
    local status=$?
    mysql_admin --execute="DROP DATABASE IF EXISTS \`${database_name}\`" >/dev/null 2>&1 || true
    if [[ "${status}" -ne 0 ]]; then
        echo "MySQL schema E2E failed; project files retained at ${work_root}" >&2
    else
        rm -rf "${work_root}"
    fi
    return "${status}"
}
trap cleanup EXIT

export DATABASE_URL="${database_url}"
cat >"${project}/main.zyl" <<'ZEL'
database main {
    engine: mysql
}

table customers {
    id: Id primary auto
    email: String(100) required unique
}

fn main() {}
ZEL

"${zelyra_bin}" db setup "${project}/main.zyl"
"${zelyra_bin}" db inspect "${project}/main.zyl" | grep -Fq '1 tables'
"${zelyra_bin}" db map "${project}/main.zyl" --format=json >"${work_root}/map.json"
python3 - "${work_root}/map.json" <<'PY'
import json, sys
report = json.load(open(sys.argv[1], encoding="utf-8"))
assert report["backend"] == "mysql"
assert report["read_only"] is True
assert report["modules"][0]["tables"][0]["status"] == "matched"
PY

cat >"${project}/main.zyl" <<'ZEL'
database main {
    engine: mysql
}

table customers {
    id: Id primary auto
    email: String(100) required unique
    display_name: String?
}

fn main() {}
ZEL
"${zelyra_bin}" db plan "${project}/main.zyl" --format=json >"${work_root}/plan.json"
python3 - "${work_root}/plan.json" >"${work_root}/plan-id" <<'PY'
import json, sys
plan = json.load(open(sys.argv[1], encoding="utf-8"))
assert plan["backend"] == "mysql"
assert plan["drift"] == "present"
assert plan["changes"][0]["risk"] == "safe"
print(plan["plan_id"])
PY
"${zelyra_bin}" db apply "${project}/main.zyl" --plan-id "$(cat "${work_root}/plan-id")" >/dev/null
"${zelyra_bin}" db history "${project}/main.zyl" --format=json >"${work_root}/history.json"
python3 - "${work_root}/history.json" <<'PY'
import json, sys
history = json.load(open(sys.argv[1], encoding="utf-8"))
assert len(history) == 1
assert history[0]["status"] == "applied"
assert history[0]["change_count"] == 1
PY
column_count="$(mysql_admin --database="${database_name}" --execute="SELECT COUNT(*) FROM information_schema.COLUMNS WHERE TABLE_SCHEMA=DATABASE() AND TABLE_NAME='customers' AND COLUMN_NAME='display_name'")"
[[ "${column_count}" == "1" ]]
echo "MySQL schema setup, inspection, read-only map, reviewed migration, and migration history passed"
