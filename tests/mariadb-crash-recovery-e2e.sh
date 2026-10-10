#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
container_name="zelyra-mariadb-crash-e2e-$$"
db_name="zelyra_crash_e2e"
db_password="CrashRecoveryE2E$RANDOM$RANDOM"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-mariadb-crash-e2e.XXXXXX")"
apply_pid=""

cleanup() {
    if [[ -n "${apply_pid}" ]] && kill -0 "-${apply_pid}" 2>/dev/null; then
        kill -KILL -- "-${apply_pid}" 2>/dev/null || true
        wait "${apply_pid}" 2>/dev/null || true
    fi
    docker rm --force "${container_name}" >/dev/null 2>&1 || true
    rm -rf -- "${temp_dir}"
}
trap cleanup EXIT

for command in docker mariadb mariadb-admin; do
    command -v "${command}" >/dev/null 2>&1 || {
        echo "error: ${command} is required for MariaDB crash recovery E2E" >&2
        exit 1
    }
done
[[ -x "${zelyra_bin}" ]] || {
    echo "error: Zelyra binary not found at ${zelyra_bin}; run cargo build -p zelyra-cli first" >&2
    exit 1
}

project_before="${temp_dir}/before.zyl"
project_after="${temp_dir}/after.zyl"
cat >"${project_before}" <<'ZYL'
database main { engine: mariadb database: "zelyra_crash_e2e" }
table crash_first { id: Id primary auto }
table crash_second { id: Id primary auto }
fn main() {}
ZYL
cat >"${project_after}" <<'ZYL'
database main { engine: mariadb database: "zelyra_crash_e2e" }
table crash_first { id: Id primary auto applied_marker: String(80) }
table crash_second { id: Id primary auto operator_review: String(80) }
fn main() {}
ZYL

docker run --detach --name "${container_name}" \
    --env "MARIADB_ROOT_PASSWORD=${db_password}" \
    --publish 127.0.0.1::3306 mariadb:11 >/dev/null
db_port="$(docker port "${container_name}" 3306/tcp | sed 's/.*://')"
for _ in $(seq 1 90); do
    if MYSQL_PWD="${db_password}" mariadb-admin --protocol=tcp --host=127.0.0.1 \
        --port="${db_port}" --user=root ping >/dev/null 2>&1; then
        break
    fi
    sleep 1
done
MYSQL_PWD="${db_password}" mariadb-admin --protocol=tcp --host=127.0.0.1 \
    --port="${db_port}" --user=root ping >/dev/null
MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host=127.0.0.1 \
    --port="${db_port}" --user=root --execute="CREATE DATABASE ${db_name}"
database_url="mariadb://root:${db_password}@127.0.0.1:${db_port}/${db_name}"

echo "[MariaDB] create the baseline schema and prepare a two-step migration"
DATABASE_URL="${database_url}" "${zelyra_bin}" db bootstrap "${project_before}" >/dev/null
plan="$(DATABASE_URL="${database_url}" "${zelyra_bin}" db plan "${project_after}" --format=json)"
plan_id="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["plan_id"])' <<<"${plan}")"
[[ "$(python3 -c 'import json,sys; print(len(json.load(sys.stdin)["changes"]))' <<<"${plan}")" == "2" ]]

wrapper_dir="${temp_dir}/wrapper"
mkdir -p "${wrapper_dir}"
cat >"${wrapper_dir}/mariadb" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
for argument in "$@"; do
    if [[ "${argument}" == "--execute" || "${argument}" == "-e" ]]; then
        exec "${ZELYRA_TEST_REAL_MARIADB}" "$@"
    fi
done
sql_file="$(mktemp "${ZELYRA_TEST_TEMP_DIR}/mariadb-sql.XXXXXX")"
trap 'rm -f -- "${sql_file}"' EXIT
cat >"${sql_file}"
set +e
"${ZELYRA_TEST_REAL_MARIADB}" "$@" <"${sql_file}"
status=$?
set -e
if [[ "${ZELYRA_TEST_PAUSE_AFTER_DDL:-}" == "1" ]] \
    && grep -Fq 'ALTER TABLE `crash_first` ADD COLUMN `applied_marker`' "${sql_file}"; then
    : >"${ZELYRA_TEST_DDL_COMMITTED}"
    while [[ ! -e "${ZELYRA_TEST_RELEASE_WRAPPER}" ]]; do sleep 0.05; done
fi
exit "${status}"
SH
chmod +x "${wrapper_dir}/mariadb"
export ZELYRA_TEST_REAL_MARIADB="$(command -v mariadb)"
export ZELYRA_TEST_TEMP_DIR="${temp_dir}"
export ZELYRA_TEST_DDL_COMMITTED="${temp_dir}/ddl-committed"
export ZELYRA_TEST_RELEASE_WRAPPER="${temp_dir}/release-wrapper"

echo "[MariaDB] stop the server after DDL commits but before Zelyra journals completion"
setsid env PATH="${wrapper_dir}:${PATH}" \
    ZELYRA_TEST_PAUSE_AFTER_DDL=1 DATABASE_URL="${database_url}" \
    "${zelyra_bin}" db apply "${project_after}" --allow-risky --plan-id "${plan_id}" \
    >"${temp_dir}/apply.log" 2>&1 &
apply_pid=$!
for _ in $(seq 1 120); do
    [[ -e "${ZELYRA_TEST_DDL_COMMITTED}" ]] && break
    if ! kill -0 "${apply_pid}" 2>/dev/null; then
        cat "${temp_dir}/apply.log" >&2
        echo "error: migration exited before the committed-DDL pause" >&2
        exit 1
    fi
    sleep 0.1
done
if [[ ! -e "${ZELYRA_TEST_DDL_COMMITTED}" ]]; then
    cat "${temp_dir}/apply.log" >&2
    echo "error: did not observe the first committed DDL step" >&2
    exit 1
fi
docker kill "${container_name}" >/dev/null
: >"${ZELYRA_TEST_RELEASE_WRAPPER}"
set +e
wait "${apply_pid}"
apply_status=$?
set -e
apply_pid=""
if [[ "${apply_status}" == "0" ]]; then
    echo "error: migration unexpectedly completed its journal update with MariaDB stopped" >&2
    exit 1
fi

docker start "${container_name}" >/dev/null
db_port="$(docker port "${container_name}" 3306/tcp | sed 's/.*://')"
database_url="mariadb://root:${db_password}@127.0.0.1:${db_port}/${db_name}"
for _ in $(seq 1 90); do
    if MYSQL_PWD="${db_password}" mariadb-admin --protocol=tcp --host=127.0.0.1 \
        --port="${db_port}" --user=root ping >/dev/null 2>&1; then
        break
    fi
    sleep 1
done
MYSQL_PWD="${db_password}" mariadb-admin --protocol=tcp --host=127.0.0.1 \
    --port="${db_port}" --user=root ping >/dev/null

history="$(DATABASE_URL="${database_url}" "${zelyra_bin}" db history "${project_after}" --format=json)"
grep -Fq '"status": "interrupted"' <<<"${history}"
schema_state="$(MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host=127.0.0.1 \
    --port="${db_port}" --user=root "${db_name}" --batch --skip-column-names \
    --execute="SELECT CONCAT((SELECT COUNT(*) FROM information_schema.COLUMNS WHERE TABLE_SCHEMA='${db_name}' AND TABLE_NAME='crash_first' AND COLUMN_NAME='applied_marker'), ':', (SELECT COUNT(*) FROM information_schema.COLUMNS WHERE TABLE_SCHEMA='${db_name}' AND TABLE_NAME='crash_second' AND COLUMN_NAME='operator_review'))")"
[[ "${schema_state}" == "1:0" ]]

echo "[MariaDB] recover from a fresh plan after server loss at the DDL/journal boundary"
DATABASE_URL="${database_url}" "${zelyra_bin}" db apply "${project_after}" --allow-risky >/dev/null
DATABASE_URL="${database_url}" "${zelyra_bin}" db plan "${project_after}" | grep -Fq "No schema changes."
history="$(DATABASE_URL="${database_url}" "${zelyra_bin}" db history "${project_after}" --format=json)"
grep -Fq '"status": "applied"' <<<"${history}"
schema_state="$(MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host=127.0.0.1 \
    --port="${db_port}" --user=root "${db_name}" --batch --skip-column-names \
    --execute="SELECT CONCAT((SELECT COUNT(*) FROM information_schema.COLUMNS WHERE TABLE_SCHEMA='${db_name}' AND TABLE_NAME='crash_first' AND COLUMN_NAME='applied_marker'), ':', (SELECT COUNT(*) FROM information_schema.COLUMNS WHERE TABLE_SCHEMA='${db_name}' AND TABLE_NAME='crash_second' AND COLUMN_NAME='operator_review'))")"
[[ "${schema_state}" == "1:1" ]]
echo "MariaDB server-crash recovery E2E passed"
