#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
project_file="${ZELYRA_PROTECTED_E2E_PROJECT:-${repo_dir}/examples/auth_crud_api.zyl}"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
address="${ZELYRA_PROTECTED_E2E_ADDRESS:-127.0.0.1:38510}"
base_url="http://${address}"
database_url="${DATABASE_URL:-}"
suffix="$(date +%s)"
primary_email="zelyra-protected-primary-${suffix}@example.test"
secondary_email="zelyra-protected-secondary-${suffix}@example.test"
test_password="ZelyraProtected-${suffix}-Password"
customer_name="Zelyra Protected Customer-${suffix}"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-mariadb-protected-e2e.XXXXXX")"
server_pid=""

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
db_password_from_url="${database_credentials#*:}"
db_host_port="${database_location%%/*}"
db_name="${database_location#*/}"
db_host="${db_host_port%%:*}"
db_port="${db_host_port##*:}"
db_password="${ZELYRA_PROTECTED_E2E_DB_PASSWORD:-${db_password_from_url}}"

client() {
    MYSQL_PWD="${db_password}" mariadb \
        --protocol=tcp \
        --host="${db_host}" \
        --port="${db_port}" \
        --user="${db_user}" \
        "${db_name}" "$@"
}

cleanup() {
    if [[ -n "${server_pid}" ]]; then
        kill "${server_pid}" 2>/dev/null || true
        wait "${server_pid}" 2>/dev/null || true
    fi
    client --batch --skip-column-names <<SQL >/dev/null 2>&1 || true
DELETE FROM user_permissions
WHERE user_id IN (SELECT id FROM users WHERE email IN ('${primary_email}', '${secondary_email}'));
DELETE FROM auth_sessions
WHERE user_id IN (SELECT id FROM users WHERE email IN ('${primary_email}', '${secondary_email}'));
DELETE FROM users WHERE email IN ('${primary_email}', '${secondary_email}');
DELETE FROM customers WHERE name = '${customer_name}';
SQL
    rm -rf "${temp_dir}"
}
trap cleanup EXIT

if ! command -v mariadb >/dev/null 2>&1; then
    echo "error: mariadb client is required for the protected-route integration test" >&2
    exit 1
fi
if ! command -v curl >/dev/null 2>&1; then
    echo "error: curl is required for the protected-route integration test" >&2
    exit 1
fi
if [[ ! -x "${zelyra_bin}" ]]; then
    echo "error: Zelyra binary not found at ${zelyra_bin}; run cargo build -p zelyra-cli first" >&2
    exit 1
fi

echo "[1/7] setting up MariaDB protected-resource schema"
"${zelyra_bin}" db setup "${project_file}"

echo "[2/7] creating users, permission data, and a customer"
primary_hash="$(printf '%s\n' "${test_password}" | "${zelyra_bin}" auth hash-password --stdin)"
secondary_hash="$(printf '%s\n' "${test_password}" | "${zelyra_bin}" auth hash-password --stdin)"
client <<SQL
INSERT INTO users (email, password_hash)
VALUES ('${primary_email}', '${primary_hash}'),
       ('${secondary_email}', '${secondary_hash}');
SET @primary_user_id = (SELECT id FROM users WHERE email = '${primary_email}');
SET @secondary_user_id = (SELECT id FROM users WHERE email = '${secondary_email}');
INSERT INTO user_permissions (user_id, permission)
VALUES (@primary_user_id, 'customers.view'),
       (@secondary_user_id, 'other.permission');
INSERT INTO customers (name) VALUES ('${customer_name}');
SQL
customer_id="$(client --batch --skip-column-names -e "SELECT id FROM customers WHERE name = '${customer_name}'")"
[[ -n "${customer_id}" ]]

echo "[3/7] starting the protected CRUD and API server"
"${zelyra_bin}" serve "${project_file}" "${address}" >"${temp_dir}/server.log" 2>&1 &
server_pid=$!
for _ in $(seq 1 30); do
    if curl --silent --show-error --fail "${base_url}/login" -o "${temp_dir}/login.html"; then
        break
    fi
    sleep 1
done
curl --silent --show-error --fail "${base_url}/login" -o "${temp_dir}/login.html"

extract_csrf() {
    sed -n 's/.*name="_zelyra_csrf" value="\([^"]*\)".*/\1/p' "$1"
}

request_status() {
    local output_file="$1"
    shift
    curl --silent --show-error --output "${output_file}" --write-out '%{http_code}' "$@"
}

echo "[4/7] rejecting anonymous CRUD and API requests"
anonymous_crud_status="$(request_status "${temp_dir}/anonymous-crud.html" "${base_url}/customers")"
[[ "${anonymous_crud_status}" == "401" ]]
anonymous_api_status="$(request_status "${temp_dir}/anonymous-api.json" "${base_url}/api/customers/${customer_id}")"
[[ "${anonymous_api_status}" == "401" ]]
grep -Fq '"code":"Unauthorized"' "${temp_dir}/anonymous-api.json"
csrf="$(extract_csrf "${temp_dir}/login.html")"
[[ -n "${csrf}" ]]

echo "[5/7] allowing the declared permission on CRUD and API"
primary_cookie="${temp_dir}/primary.cookies"
primary_login_status="$(request_status "${temp_dir}/primary-login.html" \
    --cookie-jar "${primary_cookie}" \
    --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "email=${primary_email}" \
    --data-urlencode "password=${test_password}" \
    "${base_url}/login")"
[[ "${primary_login_status}" == "303" ]]
primary_crud_status="$(request_status "${temp_dir}/primary-crud.html" \
    --cookie "${primary_cookie}" \
    "${base_url}/customers")"
[[ "${primary_crud_status}" == "200" ]]
grep -Fq "${customer_name}" "${temp_dir}/primary-crud.html"
primary_api_status="$(request_status "${temp_dir}/primary-api.json" \
    --cookie "${primary_cookie}" \
    "${base_url}/api/customers/${customer_id}")"
[[ "${primary_api_status}" == "200" ]]
grep -Fq "\"name\":\"${customer_name}\"" "${temp_dir}/primary-api.json"

echo "[6/7] denying a logged-in user without the CRUD/API permission"
secondary_cookie="${temp_dir}/secondary.cookies"
secondary_login_status="$(request_status "${temp_dir}/secondary-login.html" \
    --cookie-jar "${secondary_cookie}" \
    --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "email=${secondary_email}" \
    --data-urlencode "password=${test_password}" \
    "${base_url}/login")"
[[ "${secondary_login_status}" == "303" ]]
secondary_crud_status="$(request_status "${temp_dir}/secondary-crud.html" \
    --cookie "${secondary_cookie}" \
    "${base_url}/customers")"
[[ "${secondary_crud_status}" == "403" ]]
grep -Fq "Missing permission: customers.view" "${temp_dir}/secondary-crud.html"
secondary_api_status="$(request_status "${temp_dir}/secondary-api.json" \
    --cookie "${secondary_cookie}" \
    "${base_url}/api/customers/${customer_id}")"
[[ "${secondary_api_status}" == "403" ]]
grep -Fq '"code":"Forbidden"' "${temp_dir}/secondary-api.json"

echo "[7/7] protected-resource E2E cleanup completed"
echo "MariaDB protected CRUD/API E2E passed"
