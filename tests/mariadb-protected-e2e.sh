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
viewer_email="zelyra-protected-viewer-${suffix}@example.test"
test_password="ZelyraProtected-${suffix}-Password"
customer_name="Zelyra Protected Customer-${suffix}"
created_customer_name="Zelyra Created Customer-${suffix}"
edited_customer_name="Zelyra Edited Customer-${suffix}"
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
WHERE user_id IN (SELECT id FROM users WHERE email IN ('${primary_email}', '${secondary_email}', '${viewer_email}'));
DELETE FROM auth_sessions
WHERE user_id IN (SELECT id FROM users WHERE email IN ('${primary_email}', '${secondary_email}', '${viewer_email}'));
DELETE FROM users WHERE email IN ('${primary_email}', '${secondary_email}', '${viewer_email}');
DELETE FROM customers
WHERE name IN ('${customer_name}', '${created_customer_name}', '${edited_customer_name}');
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

echo "[1/10] setting up MariaDB protected-resource schema"
"${zelyra_bin}" db setup "${project_file}"

echo "[2/10] creating users, permission data, and a customer"
primary_hash="$(printf '%s\n' "${test_password}" | "${zelyra_bin}" auth hash-password --stdin)"
secondary_hash="$(printf '%s\n' "${test_password}" | "${zelyra_bin}" auth hash-password --stdin)"
viewer_hash="$(printf '%s\n' "${test_password}" | "${zelyra_bin}" auth hash-password --stdin)"
client <<SQL
INSERT INTO users (email, password_hash)
VALUES ('${primary_email}', '${primary_hash}'),
       ('${secondary_email}', '${secondary_hash}'),
       ('${viewer_email}', '${viewer_hash}');
SET @primary_user_id = (SELECT id FROM users WHERE email = '${primary_email}');
SET @secondary_user_id = (SELECT id FROM users WHERE email = '${secondary_email}');
SET @viewer_user_id = (SELECT id FROM users WHERE email = '${viewer_email}');
INSERT INTO user_permissions (user_id, permission)
VALUES (@primary_user_id, 'customers.view'),
       (@primary_user_id, 'customers.create'),
       (@primary_user_id, 'customers.edit'),
       (@primary_user_id, 'customers.delete'),
       (@secondary_user_id, 'other.permission'),
       (@viewer_user_id, 'customers.view');
INSERT INTO customers (name) VALUES ('${customer_name}');
SQL
customer_id="$(client --batch --skip-column-names -e "SELECT id FROM customers WHERE name = '${customer_name}'")"
[[ -n "${customer_id}" ]]

echo "[3/10] starting the protected CRUD and API server"
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

echo "[4/10] rejecting anonymous CRUD, form, and API requests"
anonymous_crud_status="$(request_status "${temp_dir}/anonymous-crud.html" "${base_url}/customers")"
[[ "${anonymous_crud_status}" == "401" ]]
anonymous_create_status="$(request_status "${temp_dir}/anonymous-create.html" "${base_url}/customers/new")"
[[ "${anonymous_create_status}" == "401" ]]
anonymous_api_status="$(request_status "${temp_dir}/anonymous-api.json" "${base_url}/api/customers/${customer_id}")"
[[ "${anonymous_api_status}" == "401" ]]
grep -Fq '"code":"Unauthorized"' "${temp_dir}/anonymous-api.json"
csrf="$(extract_csrf "${temp_dir}/login.html")"
[[ -n "${csrf}" ]]

echo "[5/10] allowing view and action permissions for the primary user"
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

primary_create_status="$(request_status "${temp_dir}/primary-create.html" \
    --cookie "${primary_cookie}" \
    "${base_url}/customers/new")"
[[ "${primary_create_status}" == "200" ]]
create_csrf="$(extract_csrf "${temp_dir}/primary-create.html")"
[[ -n "${create_csrf}" ]]
primary_create_submit_status="$(request_status "${temp_dir}/primary-create-submit.html" \
    --cookie "${primary_cookie}" \
    --data-urlencode "_zelyra_csrf=${create_csrf}" \
    --data-urlencode "name=${created_customer_name}" \
    "${base_url}/customers/new")"
[[ "${primary_create_submit_status}" == "303" ]]
created_customer_id="$(client --batch --skip-column-names -e "SELECT id FROM customers WHERE name = '${created_customer_name}'")"
[[ -n "${created_customer_id}" ]]

primary_edit_status="$(request_status "${temp_dir}/primary-edit.html" \
    --cookie "${primary_cookie}" \
    "${base_url}/customers/${created_customer_id}/edit")"
[[ "${primary_edit_status}" == "200" ]]
edit_csrf="$(extract_csrf "${temp_dir}/primary-edit.html")"
[[ -n "${edit_csrf}" ]]
primary_edit_submit_status="$(request_status "${temp_dir}/primary-edit-submit.html" \
    --cookie "${primary_cookie}" \
    --data-urlencode "_zelyra_csrf=${edit_csrf}" \
    --data-urlencode "name=${edited_customer_name}" \
    "${base_url}/customers/${created_customer_id}/edit")"
[[ "${primary_edit_submit_status}" == "303" ]]
edited_customer_id="$(client --batch --skip-column-names -e "SELECT id FROM customers WHERE name = '${edited_customer_name}'")"
[[ "${edited_customer_id}" == "${created_customer_id}" ]]

echo "[6/10] denying a logged-in user without the CRUD/API permission"
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

secondary_create_status="$(request_status "${temp_dir}/secondary-create.html" \
    --cookie "${secondary_cookie}" \
    "${base_url}/customers/new")"
[[ "${secondary_create_status}" == "403" ]]
grep -Fq "Missing permission: customers.create" "${temp_dir}/secondary-create.html"
secondary_edit_status="$(request_status "${temp_dir}/secondary-edit.html" \
    --cookie "${secondary_cookie}" \
    "${base_url}/customers/${created_customer_id}/edit")"
[[ "${secondary_edit_status}" == "403" ]]
grep -Fq "Missing permission: customers.edit" "${temp_dir}/secondary-edit.html"

echo "[7/10] showing only permitted CRUD actions to a view-only user"
viewer_cookie="${temp_dir}/viewer.cookies"
viewer_login_status="$(request_status "${temp_dir}/viewer-login.html" \
    --cookie-jar "${viewer_cookie}" \
    --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "email=${viewer_email}" \
    --data-urlencode "password=${test_password}" \
    "${base_url}/login")"
[[ "${viewer_login_status}" == "303" ]]
viewer_crud_status="$(request_status "${temp_dir}/viewer-crud.html" \
    --cookie "${viewer_cookie}" \
    "${base_url}/customers")"
[[ "${viewer_crud_status}" == "200" ]]
! grep -Fq 'href="/customers/new"' "${temp_dir}/viewer-crud.html"
viewer_detail_status="$(request_status "${temp_dir}/viewer-detail.html" \
    --cookie "${viewer_cookie}" \
    "${base_url}/customers/${customer_id}")"
[[ "${viewer_detail_status}" == "200" ]]
! grep -Fq "href=\"/customers/${customer_id}/edit\"" "${temp_dir}/viewer-detail.html"
! grep -Fq 'href="/customers/new"' "${temp_dir}/viewer-detail.html"
! grep -Fq "action=\"/customers/${customer_id}/delete\"" "${temp_dir}/viewer-detail.html"
viewer_create_status="$(request_status "${temp_dir}/viewer-create.html" \
    --cookie "${viewer_cookie}" \
    "${base_url}/customers/new")"
[[ "${viewer_create_status}" == "403" ]]
viewer_edit_status="$(request_status "${temp_dir}/viewer-edit.html" \
    --cookie "${viewer_cookie}" \
    "${base_url}/customers/${customer_id}/edit")"
[[ "${viewer_edit_status}" == "403" ]]
viewer_delete_status="$(request_status "${temp_dir}/viewer-delete.html" \
    --cookie "${viewer_cookie}" \
    --request POST \
    --data-urlencode "_zelyra_csrf=not-authorized" \
    "${base_url}/customers/${customer_id}/delete")"
[[ "${viewer_delete_status}" == "403" ]]

echo "[8/10] denying a delete without the delete permission"
secondary_delete_status="$(request_status "${temp_dir}/secondary-delete.html" \
    --cookie "${secondary_cookie}" \
    --request POST \
    --data-urlencode "_zelyra_csrf=not-authorized" \
    "${base_url}/customers/${created_customer_id}/delete")"
[[ "${secondary_delete_status}" == "403" ]]
grep -Fq "Missing permission: customers.delete" "${temp_dir}/secondary-delete.html"

echo "[9/10] allowing delete for the primary user"
primary_detail_status="$(request_status "${temp_dir}/primary-detail.html" \
    --cookie "${primary_cookie}" \
    "${base_url}/customers/${created_customer_id}")"
[[ "${primary_detail_status}" == "200" ]]
delete_csrf="$(extract_csrf "${temp_dir}/primary-detail.html")"
[[ -n "${delete_csrf}" ]]
primary_delete_status="$(request_status "${temp_dir}/primary-delete.html" \
    --cookie "${primary_cookie}" \
    --request POST \
    --data-urlencode "_zelyra_csrf=${delete_csrf}" \
    "${base_url}/customers/${created_customer_id}/delete")"
[[ "${primary_delete_status}" == "303" ]]
remaining_created="$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM customers WHERE id = '${created_customer_id}'")"
[[ "${remaining_created}" == "0" ]]

echo "[10/10] protected-resource E2E cleanup completed"
echo "MariaDB protected CRUD/API E2E passed"
