#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
fixture_dir="${script_dir}/fixtures"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"

if [[ ! -x "${zelyra_bin}" ]]; then
    echo "error: Zelyra binary not found at ${zelyra_bin}; run cargo build -p zelyra-cli first" >&2
    exit 1
fi
if ! command -v sqlite3 >/dev/null 2>&1; then
    echo "error: sqlite3 is required for schema safety integration tests" >&2
    exit 1
fi

temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-schema-safety.XXXXXX")"
database_path="${temp_dir}/schema-safety.sqlite3"
sqlite_url="sqlite://${database_path}"
mariadb_database=""
mariadb_url=""
mariadb_user=""
mariadb_password=""
mariadb_host=""
mariadb_port=""

cleanup() {
    if [[ -n "${mariadb_database}" ]]; then
        MYSQL_PWD="${mariadb_password}" mariadb \
            --protocol=tcp --host="${mariadb_host}" --port="${mariadb_port}" \
            --user="${mariadb_user}" --batch --skip-column-names \
            -e "DROP DATABASE IF EXISTS \`${mariadb_database}\`;" >/dev/null 2>&1 || true
    fi
    rm -rf -- "${temp_dir}"
}
trap cleanup EXIT

assert_sqlite_safety() {
    local before="${fixture_dir}/schema_safety_before_sqlite.zyl"
    local after="${fixture_dir}/schema_safety_after_sqlite.zyl"

    echo "[SQLite] create isolated schema and retained test row"
    DATABASE_URL="${sqlite_url}" "${zelyra_bin}" db bootstrap "${before}"
    sqlite3 "${database_path}" "INSERT INTO safety_records(label, archive_note) VALUES ('keep-row', 'keep-note');"

    local plan
    plan="$(DATABASE_URL="${sqlite_url}" "${zelyra_bin}" db plan "${after}")"
    grep -Fq "[DESTRUCTIVE] drop column safety_records.archive_note" <<<"${plan}"

    local output
    if output="$(DATABASE_URL="${sqlite_url}" "${zelyra_bin}" db apply "${after}" 2>&1)"; then
        echo "error: SQLite db apply unexpectedly accepted a destructive change" >&2
        exit 1
    fi
    grep -Fq "error[E-DB-004]" <<<"${output}"
    [[ "$(sqlite3 "${database_path}" "SELECT label || ':' || archive_note FROM safety_records;")" == "keep-row:keep-note" ]]

    DATABASE_URL="${sqlite_url}" "${zelyra_bin}" db apply "${after}" --allow-destructive >/dev/null
    [[ "$(sqlite3 "${database_path}" "SELECT label FROM safety_records;")" == "keep-row" ]]
    [[ "$(sqlite3 "${database_path}" "PRAGMA table_info(safety_records);" | cut -d'|' -f2 | grep -cx archive_note || true)" == "0" ]]
    echo "[SQLite] default refusal preserves data; explicit approval applies the change"
}

assert_mariadb_safety() {
    local configured_url="${ZELYRA_SCHEMA_SAFETY_MARIADB_URL:-}"
    if [[ -z "${configured_url}" ]]; then
        echo "[MariaDB] skipped (set ZELYRA_SCHEMA_SAFETY_MARIADB_URL to a local zelyra_ci or zelyra_test URL)"
        return
    fi
    if ! command -v mariadb >/dev/null 2>&1; then
        echo "error: mariadb client is required when MariaDB schema safety testing is enabled" >&2
        exit 1
    fi

    local parts authority credentials location host_port base_database
    parts="${configured_url#mariadb://}"
    if [[ "${parts}" == "${configured_url}" ]]; then
        parts="${configured_url#mysql://}"
        [[ "${parts}" != "${configured_url}" ]] || {
            echo "error: MariaDB safety URL must use mariadb:// or mysql://" >&2
            exit 1
        }
    fi
    authority="${parts%%/*}"
    base_database="${parts#*/}"
    credentials="${authority%@*}"
    host_port="${authority#*@}"
    mariadb_user="${credentials%%:*}"
    mariadb_password="${credentials#*:}"
    mariadb_host="${host_port%%:*}"
    mariadb_port="${host_port##*:}"
    [[ "${mariadb_host}" != "${host_port}" ]] || mariadb_port="3306"

    if [[ "${base_database}" != "zelyra_ci" && "${base_database}" != "zelyra_test" ]] || \
       [[ "${mariadb_host}" != "127.0.0.1" && "${mariadb_host}" != "localhost" && "${mariadb_host}" != "::1" ]]; then
        echo "error: MariaDB safety test is restricted to a local server and a zelyra_ci or zelyra_test base database" >&2
        exit 1
    fi

    mariadb_database="zelyra_schema_safety_${$}"
    mariadb_url="mariadb://${mariadb_user}:${mariadb_password}@${mariadb_host}:${mariadb_port}/${mariadb_database}"
    MYSQL_PWD="${mariadb_password}" mariadb \
        --protocol=tcp --host="${mariadb_host}" --port="${mariadb_port}" \
        --user="${mariadb_user}" --batch --skip-column-names \
        -e "CREATE DATABASE \`${mariadb_database}\`;"

    local before="${fixture_dir}/schema_safety_before_mariadb.zyl"
    local after="${fixture_dir}/schema_safety_after_mariadb.zyl"
    echo "[MariaDB] create isolated schema and retained test row"
    DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db bootstrap "${before}"
    MYSQL_PWD="${mariadb_password}" mariadb \
        --protocol=tcp --host="${mariadb_host}" --port="${mariadb_port}" \
        --user="${mariadb_user}" "${mariadb_database}" --batch --skip-column-names \
        -e "INSERT INTO safety_records(label, archive_note) VALUES ('keep-row', 'keep-note');"

    local plan
    plan="$(DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db plan "${after}")"
    grep -Fq "[DESTRUCTIVE] drop column safety_records.archive_note" <<<"${plan}"

    local output
    if output="$(DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db apply "${after}" 2>&1)"; then
        echo "error: MariaDB db apply unexpectedly accepted a destructive change" >&2
        exit 1
    fi
    grep -Fq "error[E-DB-004]" <<<"${output}"
    local retained
    retained="$(MYSQL_PWD="${mariadb_password}" mariadb \
        --protocol=tcp --host="${mariadb_host}" --port="${mariadb_port}" \
        --user="${mariadb_user}" "${mariadb_database}" --batch --skip-column-names \
        -e "SELECT CONCAT(label, ':', archive_note) FROM safety_records;")"
    [[ "${retained}" == "keep-row:keep-note" ]]

    DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db apply "${after}" --allow-destructive >/dev/null
    local remaining_columns remaining_rows
    remaining_columns="$(MYSQL_PWD="${mariadb_password}" mariadb \
        --protocol=tcp --host="${mariadb_host}" --port="${mariadb_port}" \
        --user="${mariadb_user}" "${mariadb_database}" --batch --skip-column-names \
        -e "SELECT COUNT(*) FROM information_schema.columns WHERE table_schema='${mariadb_database}' AND table_name='safety_records' AND column_name='archive_note';")"
    remaining_rows="$(MYSQL_PWD="${mariadb_password}" mariadb \
        --protocol=tcp --host="${mariadb_host}" --port="${mariadb_port}" \
        --user="${mariadb_user}" "${mariadb_database}" --batch --skip-column-names \
        -e "SELECT CONCAT(COUNT(*), ':', MIN(label)) FROM safety_records;")"
    [[ "${remaining_columns}" == "0" && "${remaining_rows}" == "1:keep-row" ]]
    echo "[MariaDB] default refusal preserves data; explicit approval applies the change"
}

assert_sqlite_safety
assert_mariadb_safety
echo "Database schema safety E2E passed"
