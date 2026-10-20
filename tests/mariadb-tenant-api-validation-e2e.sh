#!/usr/bin/env bash
set -euo pipefail
repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
project_file="${repo_dir}/examples/tenant_api_validation.zyl"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
database_url="${DATABASE_URL:-}"
suffix="$(date +%s%N)"
email="zelyra-tenant-api-${suffix}@example.test"
password="ZelyraTenantApi-${suffix}-Password"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-tenant-api.XXXXXX")"
server_pid=""
base_url=""

if [[ "${database_url}" == mariadb://* ]]; then
    parts="${database_url#mariadb://}"
elif [[ "${database_url}" == mysql://* ]]; then
    parts="${database_url#mysql://}"
else
    echo "error: DATABASE_URL must point to a MariaDB test database" >&2
    exit 1
fi
credentials="${parts%@*}"
location="${parts#*@}"
db_user="${credentials%%:*}"
db_password="${ZELYRA_E2E_DB_PASSWORD:-${credentials#*:}}"
host_port="${location%%/*}"
base_db="${location#*/}"
db_host="${host_port%%:*}"
db_port="${host_port##*:}"
[[ "${db_host}" == 127.0.0.1 || "${db_host}" == localhost ]] || { echo "error: only local MariaDB is allowed" >&2; exit 1; }
[[ "${base_db}" =~ ^zelyra_[A-Za-z0-9_]+$ ]] || { echo "error: test database must use zelyra_* prefix" >&2; exit 1; }
test_db="${base_db}_tenant_api_$$"
test_url="${database_url%/*}/${test_db}"
client() { MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" "$@"; }
cleanup() {
    local status=$?
    if [[ -n "${server_pid}" ]]; then kill "${server_pid}" 2>/dev/null || true; wait "${server_pid}" 2>/dev/null || true; fi
    if [[ "${status}" -ne 0 && -f "${temp_dir}/server.log" ]]; then cat "${temp_dir}/server.log" >&2; fi
    client "${base_db}" --batch --skip-column-names -e "DROP DATABASE IF EXISTS \`${test_db}\`;" >/dev/null 2>&1 || true
    rm -rf -- "${temp_dir}"
    return "${status}"
}
trap cleanup EXIT
for command in mariadb curl; do command -v "${command}" >/dev/null || { echo "error: ${command} is required" >&2; exit 1; }; done
[[ -x "${zelyra_bin}" ]] || { echo "error: Zelyra binary not found at ${zelyra_bin}" >&2; exit 1; }
client "${base_db}" --batch --skip-column-names -e "CREATE DATABASE \`${test_db}\` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;"
DATABASE_URL="${test_url}" "${zelyra_bin}" db setup "${project_file}"
password_hash="$(printf '%s\n' "${password}" | "${zelyra_bin}" auth hash-password --stdin)"
client "${test_db}" --batch --skip-column-names <<SQL
INSERT INTO users (email, password_hash) VALUES ('${email}', '${password_hash}');
SET @user_id = (SELECT id FROM users WHERE email = '${email}');
INSERT INTO tenant_memberships (user_id, tenant_id, active) VALUES (@user_id, 10, TRUE), (@user_id, 20, TRUE);
SQL
# Let the OS assign an unused loopback port. A PID-derived port can collide
# with another parallel CI integration test, and `serve` may then select a
# fallback port that the test would not know about.
DATABASE_URL="${test_url}" "${zelyra_bin}" serve "${project_file}" "127.0.0.1:0" >"${temp_dir}/server.log" 2>&1 &
server_pid=$!
for _ in $(seq 1 120); do
    announced_url="$(sed -n 's/^Zelyra server listening on \(http:\/\/[^[:space:]]*\).*/\1/p' "${temp_dir}/server.log" | tail -n 1)"
    if [[ -n "${announced_url}" ]]; then
        base_url="${announced_url}"
        if curl -sSf "${base_url}/login" -o "${temp_dir}/login.html"; then break; fi
    fi
    if ! kill -0 "${server_pid}" >/dev/null 2>&1; then
        cat "${temp_dir}/server.log" >&2
        echo "error: Zelyra server exited before becoming ready" >&2
        exit 1
    fi
    sleep 0.25
done
[[ -s "${temp_dir}/login.html" ]] || { cat "${temp_dir}/server.log" >&2; echo "error: login page did not become ready" >&2; exit 1; }
csrf="$(sed -n 's/.*name="_zelyra_csrf" value="\([^"]*\)".*/\1/p' "${temp_dir}/login.html")"
cookie="${temp_dir}/cookies"
login_status="$(curl -sS -o /dev/null -w '%{http_code}' --header "Origin: ${base_url}" --cookie-jar "${cookie}" --data-urlencode "_zelyra_csrf=${csrf}" --data-urlencode "email=${email}" --data-urlencode "password=${password}" "${base_url}/login")"
[[ "${login_status}" == 303 ]]
echo "authenticated tenant test user"

missing_selector="$(curl -sS -o "${temp_dir}/ambiguous.json" -w '%{http_code}' --cookie "${cookie}" --header "Origin: ${base_url}" --header 'Content-Type: application/json' --data '{"amount":100,"invoice_date":"2026-10-20","due_date":"2026-10-21"}' "${base_url}/api/invoice-check")"
[[ "${missing_selector}" == 403 ]]
echo "ambiguous tenant selector returned ${missing_selector}"
valid_status="$(curl -sS -o "${temp_dir}/valid.json" -w '%{http_code}' --cookie "${cookie}" --header "Origin: ${base_url}" --header 'Content-Type: application/json' --data '{"amount":100,"invoice_date":"2026-10-20","due_date":"2026-10-21"}' "${base_url}/api/invoice-check?tenant_id=10")"
[[ "${valid_status}" == 200 ]]
echo "verified tenant request returned ${valid_status}"
python3 - "${temp_dir}/valid.json" <<'PY'
import json
import sys
value = json.load(open(sys.argv[1], encoding="utf-8"))
if value != 10:
    raise SystemExit(f"verified tenant API returned unexpected JSON: {value!r}")
PY
forged_status="$(curl -sS -o "${temp_dir}/forged.json" -w '%{http_code}' --cookie "${cookie}" --header "Origin: ${base_url}" --header 'Content-Type: application/json' --data '{"amount":100,"invoice_date":"2026-10-20","due_date":"2026-10-21","tenant_id":20}' "${base_url}/api/invoice-check?tenant_id=10")"
[[ "${forged_status}" == 400 ]]
echo "forged tenant input returned ${forged_status}"
invalid_status="$(curl -sS -o "${temp_dir}/invalid.json" -w '%{http_code}' --cookie "${cookie}" --header "Origin: ${base_url}" --header 'Content-Type: application/json' --data '{"amount":100,"invoice_date":"2026-10-20","due_date":"2026-10-19"}' "${base_url}/api/invoice-check?tenant_id=10")"
[[ "${invalid_status}" == 422 ]]
echo "invalid tenant-bound payload returned ${invalid_status}"
grep -Fq 'Das Fälligkeitsdatum darf nicht vor dem Rechnungsdatum liegen.' "${temp_dir}/invalid.json"
unauthorized_status="$(curl -sS -o "${temp_dir}/unauthorized.json" -w '%{http_code}' --cookie "${cookie}" --header "Origin: ${base_url}" --header 'Content-Type: application/json' --data '{"amount":100,"invoice_date":"2026-10-20","due_date":"2026-10-21"}' "${base_url}/api/invoice-check?tenant_id=30")"
[[ "${unauthorized_status}" == 403 ]]
echo "unauthorized tenant selector returned ${unauthorized_status}"
echo "scoped API validation used verified tenant membership and rejected forged tenant input"
