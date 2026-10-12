#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
project_file="${repo_dir}/examples/notification_center.zyl"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
address="${ZELYRA_NOTIFICATION_E2E_ADDRESS:-127.0.0.1:38519}"
base_url="http://${address}"
database_url="${DATABASE_URL:-}"
suffix="$(date +%s%N)"
email="zelyra-notifications-${suffix}@example.test"
password="ZelyraNotification-${suffix}-Password"
tenant_a_title="Tenant A notice ${suffix}"
tenant_a_expired="Expired notice ${suffix}"
tenant_a_read="Read notice ${suffix}"
tenant_b_title="Tenant B notice ${suffix}"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-notifications-e2e.XXXXXX")"
server_pid=""

database_parts="${database_url#mariadb://}"
if [[ "${database_url}" != mariadb://* && "${database_url}" != mysql://* ]]; then
    echo "error: DATABASE_URL must point to an isolated MariaDB test database" >&2
    exit 1
fi
database_parts="${database_url#*://}"
database_credentials="${database_parts%@*}"
database_location="${database_parts#*@}"
db_user="${database_credentials%%:*}"
db_password="${database_credentials#*:}"
db_host_port="${database_location%%/*}"
db_name="${database_location#*/}"
db_host="${db_host_port%%:*}"
db_port="${db_host_port##*:}"

client() {
    MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" "${db_name}" "$@"
}

cleanup() {
    local status=$?
    if [[ -n "${server_pid}" ]]; then
        kill "${server_pid}" 2>/dev/null || true
        wait "${server_pid}" 2>/dev/null || true
    fi
    if [[ "${status}" -ne 0 && -f "${temp_dir}/server.log" ]]; then
        cat "${temp_dir}/server.log" >&2
    fi
    if [[ "${status}" -ne 0 && -f "${temp_dir}/tenant-a.html" ]]; then
        cat "${temp_dir}/tenant-a.html" >&2
    fi
    client --batch --skip-column-names >/dev/null 2>&1 <<SQL || true
DELETE FROM notifications WHERE title IN ('${tenant_a_title}', '${tenant_a_expired}', '${tenant_a_read}', '${tenant_b_title}');
DELETE FROM tenant_memberships WHERE user_id IN (SELECT id FROM users WHERE email = '${email}');
DELETE FROM user_permissions WHERE user_id IN (SELECT id FROM users WHERE email = '${email}');
DELETE FROM auth_sessions WHERE user_id IN (SELECT id FROM users WHERE email = '${email}');
DELETE FROM users WHERE email = '${email}';
SQL
    rm -rf "${temp_dir}"
    return "${status}"
}
trap cleanup EXIT

for command in mariadb curl; do
    command -v "${command}" >/dev/null 2>&1 || { echo "error: ${command} is required" >&2; exit 1; }
done
[[ -x "${zelyra_bin}" ]] || { echo "error: Zelyra binary not found at ${zelyra_bin}" >&2; exit 1; }

echo "[1/5] applying tenant notification schema"
DATABASE_URL="${database_url}" "${zelyra_bin}" db setup "${project_file}"
password_hash="$(printf '%s\n' "${password}" | "${zelyra_bin}" auth hash-password --stdin)"
client <<SQL
INSERT INTO users (email, password_hash) VALUES ('${email}', '${password_hash}');
SET @user_id = (SELECT id FROM users WHERE email = '${email}');
INSERT INTO tenant_memberships (user_id, tenant_id, active) VALUES (@user_id, 101, TRUE), (@user_id, 202, TRUE);
INSERT INTO user_permissions (user_id, permission)
VALUES (@user_id, 'notifications.view'), (@user_id, 'notifications.manage');
INSERT INTO notifications (tenant_id, title, message, category, read_at, archived_at, expires_at)
VALUES (101, '${tenant_a_title}', 'Visible only in tenant A', 'orders', NULL, NULL, NULL),
       (101, '${tenant_a_expired}', 'Expired item', 'orders', NULL, NULL, DATE_SUB(CURRENT_TIMESTAMP, INTERVAL 1 DAY)),
       (101, '${tenant_a_read}', 'Already read', 'orders', CURRENT_TIMESTAMP, NULL, NULL),
       (202, '${tenant_b_title}', 'Visible only in tenant B', 'orders', NULL, NULL, NULL);
SQL
notification_a_id="$(client --batch --skip-column-names -e "SELECT id FROM notifications WHERE title = '${tenant_a_title}'")"
notification_b_id="$(client --batch --skip-column-names -e "SELECT id FROM notifications WHERE title = '${tenant_b_title}'")"
[[ -n "${notification_a_id}" && -n "${notification_b_id}" ]]

echo "[2/5] starting the app and authenticating"
"${zelyra_bin}" serve "${project_file}" "${address}" >"${temp_dir}/server.log" 2>&1 &
server_pid=$!
for _ in $(seq 1 30); do
    if curl --silent --show-error --fail "${base_url}/login" -o "${temp_dir}/login.html"; then break; fi
    sleep 1
done
csrf="$(sed -n 's/.*name="_zelyra_csrf" value="\([^"]*\)".*/\1/p' "${temp_dir}/login.html")"
cookie="${temp_dir}/session.cookies"
login_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --header "Origin: ${base_url}" --cookie-jar "${cookie}" \
    --data-urlencode "_zelyra_csrf=${csrf}" --data-urlencode "email=${email}" \
    --data-urlencode "password=${password}" "${base_url}/login")"
[[ "${login_status}" == "303" ]]

echo "[3/5] requiring an explicit tenant selection"
ambiguous="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' --cookie "${cookie}" "${base_url}/notifications")"
[[ "${ambiguous}" == "403" ]]

echo "[4/5] counting only current unread notifications in tenant A"
status_a="$(curl --silent --show-error --output "${temp_dir}/tenant-a.html" --write-out '%{http_code}' \
    --cookie "${cookie}" "${base_url}/notifications?tenant_id=101")"
[[ "${status_a}" == "200" ]]
grep -Fq "${tenant_a_title}" "${temp_dir}/tenant-a.html"
! grep -Fq "${tenant_a_expired}" "${temp_dir}/tenant-a.html"
! grep -Fq "${tenant_a_read}" "${temp_dir}/tenant-a.html"
! grep -Fq "${tenant_b_title}" "${temp_dir}/tenant-a.html"
! grep -Fq '\\N' "${temp_dir}/tenant-a.html"
grep -Eq 'Unread notifications: 1|Ungelesene Benachrichtigungen: 1' "${temp_dir}/tenant-a.html"

echo "[5/5] separating tenants and hiding guessed cross-tenant IDs"
status_b="$(curl --silent --show-error --output "${temp_dir}/tenant-b.html" --write-out '%{http_code}' \
    --cookie "${cookie}" --header 'X-Zelyra-Tenant: 202' "${base_url}/notifications")"
[[ "${status_b}" == "200" ]]
grep -Fq "${tenant_b_title}" "${temp_dir}/tenant-b.html"
! grep -Fq "${tenant_a_title}" "${temp_dir}/tenant-b.html"
cross_tenant="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --cookie "${cookie}" "${base_url}/notifications/${notification_b_id}?tenant_id=101")"
[[ "${cross_tenant}" == "404" ]]

echo "MariaDB notification-center tenant isolation, unread count, expiry, and direct-ID checks passed."
