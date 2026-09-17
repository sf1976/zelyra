#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
project_file="${ZELYRA_AUTH_E2E_PROJECT:-${repo_dir}/examples/auth.zyl}"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
address="${ZELYRA_AUTH_E2E_ADDRESS:-127.0.0.1:38505}"
base_url="http://${address}"
database_url="${DATABASE_URL:-}"
suffix="$(date +%s)"
primary_email="zelyra-auth-primary-${suffix}@example.test"
secondary_email="zelyra-auth-secondary-${suffix}@example.test"
test_password="ZelyraAuth-${suffix}-Password"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-mariadb-auth-e2e.XXXXXX")"
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
db_password="${ZELYRA_AUTH_DB_PASSWORD:-${db_password_from_url}}"

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
SQL
    rm -rf "${temp_dir}"
}
trap cleanup EXIT

if ! command -v mariadb >/dev/null 2>&1; then
    echo "error: mariadb client is required for the MariaDB auth integration test" >&2
    exit 1
fi
if ! command -v curl >/dev/null 2>&1; then
    echo "error: curl is required for the MariaDB auth integration test" >&2
    exit 1
fi
if [[ ! -x "${zelyra_bin}" ]]; then
    echo "error: Zelyra binary not found at ${zelyra_bin}; run cargo build -p zelyra-cli first" >&2
    exit 1
fi

echo "[1/8] setting up MariaDB authentication schema"
"${zelyra_bin}" db setup "${project_file}"

echo "[2/8] creating Argon2 users and database permission data"
primary_hash="$(printf '%s\n' "${test_password}" | "${zelyra_bin}" auth hash-password --stdin)"
secondary_hash="$(printf '%s\n' "${test_password}" | "${zelyra_bin}" auth hash-password --stdin)"
client <<SQL
INSERT INTO users (email, password_hash)
VALUES ('${primary_email}', '${primary_hash}'),
       ('${secondary_email}', '${secondary_hash}');
SET @primary_user_id = (SELECT id FROM users WHERE email = '${primary_email}');
SET @secondary_user_id = (SELECT id FROM users WHERE email = '${secondary_email}');
INSERT INTO user_permissions (user_id, permission)
VALUES (@primary_user_id, 'admin.view'),
       (@secondary_user_id, 'other.permission');
SQL
primary_user_id="$(client --batch --skip-column-names -e "SELECT id FROM users WHERE email = '${primary_email}'")"
[[ -n "${primary_user_id}" ]]

echo "[3/8] starting the protected Zelyra server"
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

echo "[4/8] rejecting anonymous and invalid login requests"
anonymous_status="$(request_status "${temp_dir}/anonymous.html" "${base_url}/admin")"
[[ "${anonymous_status}" == "401" ]]
csrf="$(extract_csrf "${temp_dir}/login.html")"
[[ -n "${csrf}" ]]
invalid_status="$(request_status "${temp_dir}/invalid-login.html" \
    --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "email=${primary_email}" \
    --data-urlencode "password=wrong-${test_password}" \
    "${base_url}/login")"
[[ "${invalid_status}" == "401" ]]

echo "[5/8] accepting the permitted user and creating a persistent session"
primary_cookie="${temp_dir}/primary.cookies"
primary_login_status="$(request_status "${temp_dir}/primary-login.html" \
    --cookie-jar "${primary_cookie}" \
    --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "email=${primary_email}" \
    --data-urlencode "password=${test_password}" \
    "${base_url}/login")"
[[ "${primary_login_status}" == "303" ]]
grep -Fq "zelyra_session" "${primary_cookie}"
primary_admin_status="$(request_status "${temp_dir}/primary-admin.html" \
    --cookie "${primary_cookie}" \
    "${base_url}/admin")"
[[ "${primary_admin_status}" == "200" ]]
grep -Fq "Admin" "${temp_dir}/primary-admin.html"
session_count="$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM auth_sessions WHERE user_id = ${primary_user_id}")"
[[ "${session_count}" == "1" ]]

echo "[6/8] denying an authenticated user without the required permission"
secondary_cookie="${temp_dir}/secondary.cookies"
secondary_login_status="$(request_status "${temp_dir}/secondary-login.html" \
    --cookie-jar "${secondary_cookie}" \
    --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "email=${secondary_email}" \
    --data-urlencode "password=${test_password}" \
    "${base_url}/login")"
[[ "${secondary_login_status}" == "303" ]]
secondary_admin_status="$(request_status "${temp_dir}/secondary-admin.html" \
    --cookie "${secondary_cookie}" \
    "${base_url}/admin")"
[[ "${secondary_admin_status}" == "403" ]]
grep -Fq "Missing permission: admin.view" "${temp_dir}/secondary-admin.html"

echo "[7/8] logging out and invalidating the persistent session"
logout_status="$(request_status "${temp_dir}/logout.html" \
    --cookie "${primary_cookie}" \
    --cookie-jar "${primary_cookie}" \
    --request POST \
    --data-urlencode "_zelyra_csrf=${csrf}" \
    "${base_url}/logout")"
[[ "${logout_status}" == "303" ]]
logged_out_admin_status="$(request_status "${temp_dir}/logged-out-admin.html" \
    --cookie "${primary_cookie}" \
    "${base_url}/admin")"
[[ "${logged_out_admin_status}" == "401" ]]
session_count_after_logout="$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM auth_sessions WHERE user_id = ${primary_user_id}")"
[[ "${session_count_after_logout}" == "0" ]]

echo "[8/8] authentication E2E cleanup completed"
echo "MariaDB authentication E2E passed"
