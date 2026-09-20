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

    sqlite3 "${database_path}" "INSERT INTO safety_records(label) VALUES ('duplicate'), ('duplicate');"
    local unique_plan unique_after
    unique_after="${fixture_dir}/schema_safety_unique_after_sqlite.zyl"
    unique_plan="$(DATABASE_URL="${sqlite_url}" "${zelyra_bin}" db plan "${unique_after}")"
    grep -Fq "[REVIEW] add unique index" <<<"${unique_plan}"
    if output="$(DATABASE_URL="${sqlite_url}" "${zelyra_bin}" db apply "${unique_after}" 2>&1)"; then
        echo "error: SQLite db apply unexpectedly accepted a unique constraint without approval" >&2
        exit 1
    fi
    grep -Fq "error[E-DB-004]" <<<"${output}"
    [[ "$(sqlite3 "${database_path}" "SELECT COUNT(*) FROM safety_records WHERE label = 'duplicate';")" == "2" ]]
    if output="$(DATABASE_URL="${sqlite_url}" "${zelyra_bin}" db apply "${unique_after}" --allow-destructive 2>&1)"; then
        echo "error: legacy destructive approval also approved a REVIEW change" >&2
        exit 1
    fi
    grep -Fq "error[E-DB-004]" <<<"${output}"
    if output="$(DATABASE_URL="${sqlite_url}" "${zelyra_bin}" db apply "${unique_after}" --allow-risky 2>&1)"; then
        echo "error: SQLite accepted a unique index over duplicate values" >&2
        exit 1
    fi
    grep -Fq "error[E-DB-005]" <<<"${output}"
    [[ "$(sqlite3 "${database_path}" "SELECT COUNT(*) FROM safety_records WHERE label = 'duplicate';")" == "2" ]]
    echo "[SQLite] unique constraint requires review and preserves duplicate rows on failure"

    local required_after required_plan
    required_after="${fixture_dir}/schema_safety_required_after_sqlite.zyl"
    required_plan="$(DATABASE_URL="${sqlite_url}" "${zelyra_bin}" db plan "${required_after}")"
    grep -Fq "[REVIEW] add required column safety_records.review_value without a default" <<<"${required_plan}"
    if output="$(DATABASE_URL="${sqlite_url}" "${zelyra_bin}" db apply "${required_after}" 2>&1)"; then
        echo "error: SQLite unexpectedly accepted a required column without approval" >&2
        exit 1
    fi
    grep -Fq "error[E-DB-004]" <<<"${output}"
    [[ "$(sqlite3 "${database_path}" "PRAGMA table_info(safety_records);" | cut -d'|' -f2 | grep -cx review_value || true)" == "0" ]]
    if output="$(DATABASE_URL="${sqlite_url}" "${zelyra_bin}" db apply "${required_after}" --allow-risky 2>&1)"; then
        echo "error: SQLite unexpectedly added a required column without a default to populated rows" >&2
        exit 1
    fi
    grep -Fq "error[E-DB-005]" <<<"${output}"
    [[ "$(sqlite3 "${database_path}" "SELECT COUNT(*) FROM safety_records;")" == "3" ]]
    echo "[SQLite] required column requires review; failed backfill attempt preserves rows"

    local nullable_after nullable_plan
    nullable_after="${fixture_dir}/schema_safety_nullable_after_sqlite.zyl"
    nullable_plan="$(DATABASE_URL="${sqlite_url}" "${zelyra_bin}" db plan "${nullable_after}")"
    grep -Fq "[UNSUPPORTED] change nullability" <<<"${nullable_plan}"
    if output="$(DATABASE_URL="${sqlite_url}" "${zelyra_bin}" db apply "${nullable_after}" --allow-risky 2>&1)"; then
        echo "error: SQLite applied a schema change the planner marks unsupported" >&2
        exit 1
    fi
    grep -Fq "error[E-DB-006]" <<<"${output}"
    [[ "$(sqlite3 "${database_path}" "SELECT COUNT(*) FROM safety_records;")" == "3" ]]
    echo "[SQLite] unsupported changes are refused even with explicit approval"

    local fk_before fk_after fk_plan
    fk_before="${fixture_dir}/schema_safety_fk_before_sqlite.zyl"
    fk_after="${fixture_dir}/schema_safety_fk_after_sqlite.zyl"
    DATABASE_URL="${sqlite_url}" "${zelyra_bin}" db bootstrap "${fk_before}" >/dev/null
    sqlite3 "${database_path}" "INSERT INTO machines(department_id) VALUES (999);"
    fk_plan="$(DATABASE_URL="${sqlite_url}" "${zelyra_bin}" db plan "${fk_after}")"
    grep -Fq "[UNSUPPORTED] add foreign key machines.department_id" <<<"${fk_plan}"
    if output="$(DATABASE_URL="${sqlite_url}" "${zelyra_bin}" db apply "${fk_after}" --allow-risky 2>&1)"; then
        echo "error: SQLite applied an unsupported foreign-key addition" >&2
        exit 1
    fi
    grep -Fq "error[E-DB-006]" <<<"${output}"
    [[ "$(sqlite3 "${database_path}" "SELECT COUNT(*) FROM machines;")" == "1" ]]
    [[ "$(sqlite3 "${database_path}" "PRAGMA foreign_key_list(machines);" | wc -l)" == "0" ]]
    echo "[SQLite] foreign-key additions are refused without a supported table rebuild"
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

    MYSQL_PWD="${mariadb_password}" mariadb \
        --protocol=tcp --host="${mariadb_host}" --port="${mariadb_port}" \
        --user="${mariadb_user}" "${mariadb_database}" --batch --skip-column-names \
        -e "INSERT INTO safety_records(label) VALUES ('duplicate'), ('duplicate');"
    local unique_plan unique_after
    unique_after="${fixture_dir}/schema_safety_unique_after_mariadb.zyl"
    unique_plan="$(DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db plan "${unique_after}")"
    grep -Fq "[REVIEW] add unique index" <<<"${unique_plan}"
    if output="$(DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db apply "${unique_after}" 2>&1)"; then
        echo "error: MariaDB db apply unexpectedly accepted a unique constraint without approval" >&2
        exit 1
    fi
    grep -Fq "error[E-DB-004]" <<<"${output}"
    if output="$(DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db apply "${unique_after}" --allow-destructive 2>&1)"; then
        echo "error: legacy destructive approval also approved a REVIEW change" >&2
        exit 1
    fi
    grep -Fq "error[E-DB-004]" <<<"${output}"
    local duplicate_count
    duplicate_count="$(MYSQL_PWD="${mariadb_password}" mariadb \
        --protocol=tcp --host="${mariadb_host}" --port="${mariadb_port}" \
        --user="${mariadb_user}" "${mariadb_database}" --batch --skip-column-names \
        -e "SELECT COUNT(*) FROM safety_records WHERE label='duplicate';")"
    [[ "${duplicate_count}" == "2" ]]
    if output="$(DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db apply "${unique_after}" --allow-risky 2>&1)"; then
        echo "error: MariaDB accepted a unique index over duplicate values" >&2
        exit 1
    fi
    grep -Fq "error[E-DB-005]" <<<"${output}"
    duplicate_count="$(MYSQL_PWD="${mariadb_password}" mariadb \
        --protocol=tcp --host="${mariadb_host}" --port="${mariadb_port}" \
        --user="${mariadb_user}" "${mariadb_database}" --batch --skip-column-names \
        -e "SELECT COUNT(*) FROM safety_records WHERE label='duplicate';")"
    [[ "${duplicate_count}" == "2" ]]
    echo "[MariaDB] unique constraint requires review and preserves duplicate rows on failure"

    local required_after required_plan
    required_after="${fixture_dir}/schema_safety_required_after_mariadb.zyl"
    required_plan="$(DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db plan "${required_after}")"
    grep -Fq "[REVIEW] add required column safety_records.review_value without a default" <<<"${required_plan}"
    if output="$(DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db apply "${required_after}" 2>&1)"; then
        echo "error: MariaDB unexpectedly accepted a required column without approval" >&2
        exit 1
    fi
    grep -Fq "error[E-DB-004]" <<<"${output}"
    [[ "$(MYSQL_PWD="${mariadb_password}" mariadb \
        --protocol=tcp --host="${mariadb_host}" --port="${mariadb_port}" \
        --user="${mariadb_user}" "${mariadb_database}" --batch --skip-column-names \
        -e "SELECT COUNT(*) FROM information_schema.columns WHERE table_schema='${mariadb_database}' AND table_name='safety_records' AND column_name='review_value';")" == "0" ]]
    if output="$(DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db apply "${required_after}" --allow-risky 2>&1)"; then
        echo "[MariaDB] explicitly approved required column was applied; verify values before relying on the backfill"
    else
        grep -Fq "error[E-DB-005]" <<<"${output}"
    fi
    [[ "$(MYSQL_PWD="${mariadb_password}" mariadb \
        --protocol=tcp --host="${mariadb_host}" --port="${mariadb_port}" \
        --user="${mariadb_user}" "${mariadb_database}" --batch --skip-column-names \
        -e "SELECT COUNT(*) FROM safety_records;")" == "3" ]]
    echo "[MariaDB] required column requires review; default refusal preserves rows"

    local nullable_after nullable_plan
    nullable_after="${fixture_dir}/schema_safety_nullable_after_mariadb.zyl"
    nullable_plan="$(DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db plan "${nullable_after}")"
    grep -Fq "[UNSUPPORTED] change nullability" <<<"${nullable_plan}"
    if output="$(DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db apply "${nullable_after}" --allow-risky 2>&1)"; then
        echo "error: MariaDB applied a schema change the planner marks unsupported" >&2
        exit 1
    fi
    grep -Fq "error[E-DB-006]" <<<"${output}"
    [[ "$(MYSQL_PWD="${mariadb_password}" mariadb \
        --protocol=tcp --host="${mariadb_host}" --port="${mariadb_port}" \
        --user="${mariadb_user}" "${mariadb_database}" --batch --skip-column-names \
        -e "SELECT COUNT(*) FROM safety_records;")" == "3" ]]
    echo "[MariaDB] unsupported changes are refused even with explicit approval"

    local fk_before fk_after fk_removed fk_plan orphan_rows fk_count
    fk_before="${fixture_dir}/schema_safety_fk_before_mariadb.zyl"
    fk_after="${fixture_dir}/schema_safety_fk_after_mariadb.zyl"
    fk_removed="${fixture_dir}/schema_safety_fk_removed_mariadb.zyl"
    DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db bootstrap "${fk_before}" >/dev/null
    MYSQL_PWD="${mariadb_password}" mariadb \
        --protocol=tcp --host="${mariadb_host}" --port="${mariadb_port}" \
        --user="${mariadb_user}" "${mariadb_database}" --batch --skip-column-names \
        -e "INSERT INTO machines(department_id) VALUES (500);"
    fk_plan="$(DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db plan "${fk_after}")"
    grep -Fq "[REVIEW] add foreign key machines.department_id" <<<"${fk_plan}"
    if output="$(DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db apply "${fk_after}" 2>&1)"; then
        echo "error: MariaDB accepted a foreign-key addition without approval" >&2
        exit 1
    fi
    grep -Fq "error[E-DB-004]" <<<"${output}"
    if output="$(DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db apply "${fk_after}" --allow-risky 2>&1)"; then
        echo "error: MariaDB accepted a foreign key despite an orphan row" >&2
        exit 1
    fi
    grep -Fq "error[E-DB-005]" <<<"${output}"
    orphan_rows="$(MYSQL_PWD="${mariadb_password}" mariadb \
        --protocol=tcp --host="${mariadb_host}" --port="${mariadb_port}" \
        --user="${mariadb_user}" "${mariadb_database}" --batch --skip-column-names \
        -e "SELECT COUNT(*) FROM machines WHERE department_id=500;")"
    [[ "${orphan_rows}" == "1" ]]
    MYSQL_PWD="${mariadb_password}" mariadb \
        --protocol=tcp --host="${mariadb_host}" --port="${mariadb_port}" \
        --user="${mariadb_user}" "${mariadb_database}" --batch --skip-column-names \
        -e "INSERT INTO departments(id, name) VALUES (500, 'reviewed');"
    DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db apply "${fk_after}" --allow-risky >/dev/null
    fk_count="$(MYSQL_PWD="${mariadb_password}" mariadb \
        --protocol=tcp --host="${mariadb_host}" --port="${mariadb_port}" \
        --user="${mariadb_user}" "${mariadb_database}" --batch --skip-column-names \
        -e "SELECT COUNT(*) FROM information_schema.KEY_COLUMN_USAGE WHERE TABLE_SCHEMA='${mariadb_database}' AND TABLE_NAME='machines' AND CONSTRAINT_NAME='fk_machines_department_id';")"
    [[ "${fk_count}" == "1" ]]

    fk_plan="$(DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db plan "${fk_removed}")"
    grep -Fq "[REVIEW] drop foreign key machines.department_id" <<<"${fk_plan}"
    if output="$(DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db apply "${fk_removed}" 2>&1)"; then
        echo "error: MariaDB accepted a foreign-key removal without approval" >&2
        exit 1
    fi
    grep -Fq "error[E-DB-004]" <<<"${output}"
    DATABASE_URL="${mariadb_url}" "${zelyra_bin}" db apply "${fk_removed}" --allow-risky >/dev/null
    fk_count="$(MYSQL_PWD="${mariadb_password}" mariadb \
        --protocol=tcp --host="${mariadb_host}" --port="${mariadb_port}" \
        --user="${mariadb_user}" "${mariadb_database}" --batch --skip-column-names \
        -e "SELECT COUNT(*) FROM information_schema.KEY_COLUMN_USAGE WHERE TABLE_SCHEMA='${mariadb_database}' AND TABLE_NAME='machines' AND CONSTRAINT_NAME='fk_machines_department_id';")"
    [[ "${fk_count}" == "0" ]]
    echo "[MariaDB] foreign-key changes require review and retain data"
}

assert_sqlite_safety
assert_mariadb_safety
echo "Database schema safety E2E passed"
