#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
project_file="${ZELYRA_AUDIT_CHAIN_E2E_PROJECT:-${repo_dir}/examples/audit_chain.zyl}"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
database_url="${DATABASE_URL:-}"
suffix="$(date +%s)"
email="zelyra-audit-chain-${suffix}@example.test"
role="zelyra-audit-chain-${suffix}"

if [[ "${database_url}" == mariadb://* ]]; then
    database_parts="${database_url#mariadb://}"
elif [[ "${database_url}" == mysql://* ]]; then
    database_parts="${database_url#mysql://}"
else
    echo "error: DATABASE_URL must point to a MariaDB test database" >&2
    exit 1
fi
database_credentials="${database_parts%@*}"
database_location="${database_parts#*@}"
db_user="${database_credentials%%:*}"
db_password="${ZELYRA_AUDIT_CHAIN_E2E_DB_PASSWORD:-${database_credentials#*:}}"
db_host_port="${database_location%%/*}"
db_name="${database_location#*/}"
db_host="${db_host_port%%:*}"
db_port="${db_host_port##*:}"

client() {
    MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" "${db_name}" "$@"
}

cleanup() {
    client --batch --skip-column-names <<SQL >/dev/null 2>&1 || true
DELETE FROM audit_chain_log WHERE target_user_id IN (SELECT id FROM users WHERE email = '${email}');
DELETE FROM user_roles WHERE user_id IN (SELECT id FROM users WHERE email = '${email}');
DELETE FROM users WHERE email = '${email}';
SQL
}
trap cleanup EXIT

if ! command -v mariadb >/dev/null 2>&1; then
    echo "error: mariadb client is required for the audit-chain integration test" >&2
    exit 1
fi
if [[ ! -x "${zelyra_bin}" ]]; then
    echo "error: Zelyra binary not found at ${zelyra_bin}; run cargo build -p zelyra-cli first" >&2
    exit 1
fi

echo "[1/6] setting up the chained-audit schema"
DATABASE_URL="${database_url}" "${zelyra_bin}" db setup "${project_file}"

echo "[2/6] writing chained role audit entries"
client --batch --skip-column-names -e "INSERT INTO users (email, password_hash) VALUES ('${email}', 'test-hash')"
user_id="$(client --batch --skip-column-names -e "SELECT id FROM users WHERE email = '${email}'")"
DATABASE_URL="${database_url}" "${zelyra_bin}" auth role grant "${project_file}" "${user_id}" "${role}"
DATABASE_URL="${database_url}" "${zelyra_bin}" auth role grant "${project_file}" "${user_id}" "${role}"
chain_count="$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM audit_chain_log WHERE target_user_id = '${user_id}' AND event = 'role.grant' AND previous_hash IS NOT NULL AND entry_hash IS NOT NULL")"
[[ "${chain_count}" == "2" ]]

echo "[3/6] verifying the complete hash chain"
DATABASE_URL="${database_url}" "${zelyra_bin}" audit verify "${project_file}" | grep -Fq "no invalid rows"

echo "[4/6] refusing destructive pruning of a chain"
if DATABASE_URL="${database_url}" "${zelyra_bin}" audit prune "${project_file}" --before 2100-01-01T00:00:00; then
    echo "error: chained audit prune unexpectedly succeeded" >&2
    exit 1
else
    prune_status=$?
fi
[[ "${prune_status}" == "1" ]]

echo "[5/6] detecting tampering"
client --batch --skip-column-names -e "UPDATE audit_chain_log SET details = CONCAT(details, ';tampered') WHERE target_user_id = '${user_id}'"
if DATABASE_URL="${database_url}" "${zelyra_bin}" audit verify "${project_file}"; then
    echo "error: tampered audit chain unexpectedly verified" >&2
    exit 1
else
    verify_status=$?
fi
[[ "${verify_status}" == "1" ]]

echo "[6/6] restoring data and verifying again"
client --batch --skip-column-names -e "UPDATE audit_chain_log SET details = 'source=cli;role=${role}' WHERE target_user_id = '${user_id}'"
DATABASE_URL="${database_url}" "${zelyra_bin}" audit verify "${project_file}" | grep -Fq "no invalid rows"
echo "MariaDB chained-audit E2E passed"
