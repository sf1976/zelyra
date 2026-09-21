#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
project_file="${ZELYRA_MACHINE_AUTH_PROJECT:-${repo_dir}/examples/machine_management_auth.zyl}"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
address="${ZELYRA_MACHINE_AUTH_ADDRESS:-127.0.0.1:38531}"
base_url="http://${address}"
database_url="${DATABASE_URL:-}"
suffix="$(date +%s)"
primary_email="zelyra-machine-primary-${suffix}@example.test"
secondary_email="zelyra-machine-viewer-${suffix}@example.test"
test_password="ZelyraMachine-${suffix}-Password"
department_name="Zelyra Machine Department-${suffix}"
machine_number="ZELYRA-AUTH-${suffix}"
machine_name="Zelyra Protected Machine-${suffix}"
updated_machine_name="Zelyra Updated Machine-${suffix}"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-machine-auth-e2e.XXXXXX")"
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
db_password="${ZELYRA_MACHINE_AUTH_DB_PASSWORD:-${db_password_from_url}}"

client() {
    MYSQL_PWD="${db_password}" mariadb \
        --protocol=tcp --host="${db_host}" --port="${db_port}" \
        --user="${db_user}" "${db_name}" "$@"
}

cleanup() {
    if [[ -n "${server_pid}" ]]; then
        kill "${server_pid}" 2>/dev/null || true
        wait "${server_pid}" 2>/dev/null || true
    fi
    client --batch --skip-column-names <<SQL >/dev/null 2>&1 || true
DELETE FROM machines WHERE number = '${machine_number}';
DELETE FROM departments WHERE name = '${department_name}';
DELETE FROM user_permissions
WHERE user_id IN (SELECT id FROM users WHERE email IN ('${primary_email}', '${secondary_email}'));
DELETE FROM auth_sessions
WHERE user_id IN (SELECT id FROM users WHERE email IN ('${primary_email}', '${secondary_email}'));
DELETE FROM users WHERE email IN ('${primary_email}', '${secondary_email}');
SQL
    rm -rf "${temp_dir}"
}
trap cleanup EXIT

for command in mariadb curl; do
    command -v "${command}" >/dev/null 2>&1 || {
        echo "error: ${command} is required for the machine auth E2E test" >&2
        exit 1
    }
done
[[ -x "${zelyra_bin}" ]] || {
    echo "error: Zelyra binary not found at ${zelyra_bin}" >&2
    exit 1
}
[[ -f "${project_file}" ]] || {
    echo "error: machine auth project not found at ${project_file}" >&2
    exit 1
}

extract_csrf() {
    sed -n 's/.*name="_zelyra_csrf" value="\([^"]*\)".*/\1/p' "$1"
}

request_status() {
    local output_file="$1"
    shift
    curl --silent --show-error --output "${output_file}" --write-out '%{http_code}' \
        --header "Origin: ${base_url}" "$@"
}

echo "[1/8] checking and setting up protected machine application"
"${zelyra_bin}" check "${project_file}" >/dev/null
"${zelyra_bin}" db setup "${project_file}"

echo "[2/8] creating users, permissions, and related department"
primary_hash="$(printf '%s\n' "${test_password}" | "${zelyra_bin}" auth hash-password --stdin)"
secondary_hash="$(printf '%s\n' "${test_password}" | "${zelyra_bin}" auth hash-password --stdin)"
client <<SQL
INSERT INTO users (email, password_hash)
VALUES ('${primary_email}', '${primary_hash}'),
       ('${secondary_email}', '${secondary_hash}');
SET @primary_user_id = (SELECT id FROM users WHERE email = '${primary_email}');
SET @secondary_user_id = (SELECT id FROM users WHERE email = '${secondary_email}');
INSERT INTO user_permissions (user_id, permission)
VALUES (@primary_user_id, 'machines.view'),
       (@primary_user_id, 'machines.create'),
       (@primary_user_id, 'machines.edit'),
       (@primary_user_id, 'machines.delete'),
       (@secondary_user_id, 'machines.view');
INSERT INTO departments (code, name) VALUES ('E2E-${suffix}', '${department_name}');
SQL
department_id="$(client --batch --skip-column-names -e "SELECT id FROM departments WHERE name = '${department_name}'")"
[[ -n "${department_id}" ]]

echo "[3/8] starting protected server"
ZELYRA_LANGUAGE="${ZELYRA_MACHINE_AUTH_LANGUAGE:-en}" \
    "${zelyra_bin}" serve "${project_file}" "${address}" >"${temp_dir}/server.log" 2>&1 &
server_pid=$!
for _ in $(seq 1 30); do
    if curl --silent --show-error --fail "${base_url}/login" -o "${temp_dir}/login.html"; then
        break
    fi
    sleep 1
done
curl --silent --show-error --fail "${base_url}/login" -o "${temp_dir}/login.html"
login_csrf="$(extract_csrf "${temp_dir}/login.html")"
[[ -n "${login_csrf}" ]]

echo "[4/8] rejecting anonymous access and accepting the permitted user"
anonymous_status="$(request_status "${temp_dir}/anonymous.html" "${base_url}/machines")"
[[ "${anonymous_status}" == "401" ]]
primary_cookie="${temp_dir}/primary.cookies"
primary_login_status="$(request_status "${temp_dir}/primary-login.html" \
    --cookie-jar "${primary_cookie}" \
    --data-urlencode "_zelyra_csrf=${login_csrf}" \
    --data-urlencode "email=${primary_email}" \
    --data-urlencode "password=${test_password}" \
    "${base_url}/login")"
[[ "${primary_login_status}" == "303" ]]

echo "[5/8] testing authorized search, filter, and create form"
primary_list_status="$(request_status "${temp_dir}/primary-list.html" \
    --cookie "${primary_cookie}" "${base_url}/machines")"
[[ "${primary_list_status}" == "200" ]]
create_form_status="$(request_status "${temp_dir}/create-form.html" \
    --cookie "${primary_cookie}" "${base_url}/machines/new")"
[[ "${create_form_status}" == "200" ]]
create_csrf="$(extract_csrf "${temp_dir}/create-form.html")"
[[ -n "${create_csrf}" ]]
create_status="$(request_status "${temp_dir}/create-response.html" \
    --cookie "${primary_cookie}" \
    --data-urlencode "_zelyra_csrf=${create_csrf}" \
    --data-urlencode "number=${machine_number}" \
    --data-urlencode "name=${machine_name}" \
    --data-urlencode "category=Production" \
    --data-urlencode "department=${department_id}" \
    --data-urlencode "status=operational" \
    --data-urlencode "active=true" \
    "${base_url}/machines/new")"
[[ "${create_status}" == "303" ]]
request_status "${temp_dir}/search.html" --cookie "${primary_cookie}" \
    --get --data-urlencode "search=${machine_number}" "${base_url}/machines" >/dev/null
grep -Fq "${machine_name}" "${temp_dir}/search.html"
machine_id="$(sed -n 's#.*href="/machines/\([0-9][0-9]*\)">.*#\1#p' "${temp_dir}/search.html" | head -1)"
[[ -n "${machine_id}" ]]

echo "[6/8] denying the viewer create and edit permissions"
secondary_cookie="${temp_dir}/secondary.cookies"
secondary_login_status="$(request_status "${temp_dir}/secondary-login.html" \
    --cookie-jar "${secondary_cookie}" \
    --data-urlencode "_zelyra_csrf=${login_csrf}" \
    --data-urlencode "email=${secondary_email}" \
    --data-urlencode "password=${test_password}" \
    "${base_url}/login")"
[[ "${secondary_login_status}" == "303" ]]
viewer_list_status="$(request_status "${temp_dir}/viewer-list.html" \
    --cookie "${secondary_cookie}" "${base_url}/machines")"
[[ "${viewer_list_status}" == "200" ]]
viewer_create_status="$(request_status "${temp_dir}/viewer-create.html" \
    --cookie "${secondary_cookie}" "${base_url}/machines/new")"
[[ "${viewer_create_status}" == "403" ]]
grep -Fq "Missing permission: machines.create" "${temp_dir}/viewer-create.html"
viewer_edit_status="$(request_status "${temp_dir}/viewer-edit.html" \
    --cookie "${secondary_cookie}" "${base_url}/machines/${machine_id}/edit")"
[[ "${viewer_edit_status}" == "403" ]]
grep -Fq "Missing permission: machines.edit" "${temp_dir}/viewer-edit.html"

echo "[7/8] testing authorized edit and delete permissions"
edit_form_status="$(request_status "${temp_dir}/edit-form.html" \
    --cookie "${primary_cookie}" "${base_url}/machines/${machine_id}/edit")"
[[ "${edit_form_status}" == "200" ]]
edit_csrf="$(extract_csrf "${temp_dir}/edit-form.html")"
[[ -n "${edit_csrf}" ]]
edit_status="$(request_status "${temp_dir}/edit-response.html" \
    --cookie "${primary_cookie}" \
    --data-urlencode "_zelyra_csrf=${edit_csrf}" \
    --data-urlencode "number=${machine_number}" \
    --data-urlencode "name=${updated_machine_name}" \
    --data-urlencode "category=Production" \
    --data-urlencode "department=${department_id}" \
    --data-urlencode "status=maintenance" \
    --data-urlencode "active=true" \
    "${base_url}/machines/${machine_id}/edit")"
[[ "${edit_status}" == "303" ]]
request_status "${temp_dir}/updated-detail.html" --cookie "${primary_cookie}" \
    "${base_url}/machines/${machine_id}" >/dev/null
delete_csrf="$(extract_csrf "${temp_dir}/updated-detail.html")"
[[ -n "${delete_csrf}" ]]
delete_status="$(request_status "${temp_dir}/delete-response.html" \
    --cookie "${primary_cookie}" \
    --data-urlencode "_zelyra_csrf=${delete_csrf}" \
    "${base_url}/machines/${machine_id}/delete")"
[[ "${delete_status}" == "303" ]]

echo "[8/8] verifying cleanup and protected machine acceptance"
remaining="$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM machines WHERE number = '${machine_number}'")"
[[ "${remaining}" == "0" ]]
echo "machine management authorization E2E passed"
