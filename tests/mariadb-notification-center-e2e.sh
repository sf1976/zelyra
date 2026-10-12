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
other_email="zelyra-notifications-other-${suffix}@example.test"
other_password="ZelyraOtherNotification-${suffix}-Password"
tenant_a_title="Tenant A notice ${suffix}"
tenant_a_other_title="Other user's tenant A notice ${suffix}"
tenant_a_expired="Expired notice ${suffix}"
tenant_a_read="Read notice ${suffix}"
tenant_a_archived="Archived notice ${suffix}"
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
DELETE FROM notifications WHERE title IN ('${tenant_a_title}', '${tenant_a_other_title}', '${tenant_a_expired}', '${tenant_a_read}', '${tenant_a_archived}', '${tenant_b_title}');
DELETE FROM tenant_memberships WHERE user_id IN (SELECT id FROM users WHERE email IN ('${email}', '${other_email}'));
DELETE FROM user_permissions WHERE user_id IN (SELECT id FROM users WHERE email IN ('${email}', '${other_email}'));
DELETE FROM auth_sessions WHERE user_id IN (SELECT id FROM users WHERE email IN ('${email}', '${other_email}'));
DELETE FROM users WHERE email IN ('${email}', '${other_email}');
SQL
    rm -rf "${temp_dir}"
    return "${status}"
}
trap cleanup EXIT

for command in mariadb curl; do
    command -v "${command}" >/dev/null 2>&1 || { echo "error: ${command} is required" >&2; exit 1; }
done
[[ -x "${zelyra_bin}" ]] || { echo "error: Zelyra binary not found at ${zelyra_bin}" >&2; exit 1; }

echo "[1/7] applying tenant and recipient notification schema"
DATABASE_URL="${database_url}" "${zelyra_bin}" db setup "${project_file}"
password_hash="$(printf '%s\n' "${password}" | "${zelyra_bin}" auth hash-password --stdin)"
other_password_hash="$(printf '%s\n' "${other_password}" | "${zelyra_bin}" auth hash-password --stdin)"
client <<SQL
INSERT INTO users (email, password_hash) VALUES ('${email}', '${password_hash}'), ('${other_email}', '${other_password_hash}');
SET @user_id = (SELECT id FROM users WHERE email = '${email}');
SET @other_user_id = (SELECT id FROM users WHERE email = '${other_email}');
INSERT INTO tenant_memberships (user_id, tenant_id, active) VALUES (@user_id, 101, TRUE), (@user_id, 202, TRUE), (@other_user_id, 101, TRUE), (@other_user_id, 202, TRUE);
INSERT INTO user_permissions (user_id, permission)
VALUES (@user_id, 'notifications.view'), (@user_id, 'notifications.manage'),
       (@other_user_id, 'notifications.view'), (@other_user_id, 'notifications.manage');
INSERT INTO notifications (tenant_id, user_id, title, message, category, read_at, archived_at, expires_at)
VALUES (101, @user_id, '${tenant_a_title}', 'Visible only to its recipient in tenant A', 'orders', NULL, NULL, NULL),
       (101, @other_user_id, '${tenant_a_other_title}', 'Visible only to the other recipient', 'orders', NULL, NULL, NULL),
       (101, @user_id, '${tenant_a_expired}', 'Expired item', 'orders', NULL, NULL, DATE_SUB(CURRENT_TIMESTAMP, INTERVAL 1 DAY)),
       (101, @user_id, '${tenant_a_read}', 'Already read', 'orders', CURRENT_TIMESTAMP, NULL, NULL),
       (101, @user_id, '${tenant_a_archived}', 'Archived item', 'orders', NULL, CURRENT_TIMESTAMP, NULL),
       (202, @user_id, '${tenant_b_title}', 'Visible only in tenant B', 'orders', NULL, NULL, NULL);
SQL
notification_a_id="$(client --batch --skip-column-names -e "SELECT id FROM notifications WHERE title = '${tenant_a_title}'")"
notification_other_id="$(client --batch --skip-column-names -e "SELECT id FROM notifications WHERE title = '${tenant_a_other_title}'")"
notification_b_id="$(client --batch --skip-column-names -e "SELECT id FROM notifications WHERE title = '${tenant_b_title}'")"
[[ -n "${notification_a_id}" && -n "${notification_other_id}" && -n "${notification_b_id}" ]]

echo "[2/7] starting the app and authenticating"
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

echo "[3/7] requiring an explicit tenant selection"
ambiguous="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' --cookie "${cookie}" "${base_url}/notifications")"
[[ "${ambiguous}" == "403" ]]

echo "[4/7] showing read history while counting only current unread notifications in tenant A"
status_a="$(curl --silent --show-error --output "${temp_dir}/tenant-a.html" --write-out '%{http_code}' \
    --cookie "${cookie}" "${base_url}/notifications?tenant_id=101")"
[[ "${status_a}" == "200" ]]
grep -Fq "${tenant_a_title}" "${temp_dir}/tenant-a.html"
! grep -Fq "${tenant_a_expired}" "${temp_dir}/tenant-a.html"
! grep -Fq "${tenant_a_archived}" "${temp_dir}/tenant-a.html"
! grep -Fq "${tenant_a_other_title}" "${temp_dir}/tenant-a.html"
grep -Fq "${tenant_a_read}" "${temp_dir}/tenant-a.html"
! grep -Fq "${tenant_b_title}" "${temp_dir}/tenant-a.html"
! grep -Fq '\\N' "${temp_dir}/tenant-a.html"
grep -Eq 'Unread notifications: 1|Ungelesene Benachrichtigungen: 1' "${temp_dir}/tenant-a.html"

echo "[5/7] separating tenants and hiding guessed cross-tenant IDs"
status_b="$(curl --silent --show-error --output "${temp_dir}/tenant-b.html" --write-out '%{http_code}' \
    --cookie "${cookie}" --header 'X-Zelyra-Tenant: 202' "${base_url}/notifications")"
[[ "${status_b}" == "200" ]]
grep -Fq "${tenant_b_title}" "${temp_dir}/tenant-b.html"
! grep -Fq "${tenant_a_title}" "${temp_dir}/tenant-b.html"
! grep -Fq "${tenant_a_other_title}" "${temp_dir}/tenant-b.html"
cross_tenant="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --cookie "${cookie}" "${base_url}/notifications/${notification_b_id}?tenant_id=101")"
[[ "${cross_tenant}" == "404" ]]
cross_recipient="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --cookie "${cookie}" "${base_url}/notifications/${notification_other_id}?tenant_id=101")"
[[ "${cross_recipient}" == "404" ]]

echo "[6/6] authenticating as a second user in the same tenant"
other_login_html="${temp_dir}/other-login.html"
other_cookie="${temp_dir}/other-session.cookies"
curl --silent --show-error --fail --cookie-jar "${other_cookie}" "${base_url}/login" -o "${other_login_html}"
other_csrf="$(sed -n 's/.*name="_zelyra_csrf" value="\([^"]*\)".*/\1/p' "${other_login_html}")"
other_login_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --header "Origin: ${base_url}" --cookie "${other_cookie}" --cookie-jar "${other_cookie}" \
    --data-urlencode "_zelyra_csrf=${other_csrf}" --data-urlencode "email=${other_email}" \
    --data-urlencode "password=${other_password}" "${base_url}/login")"
[[ "${other_login_status}" == "303" ]]
other_status="$(curl --silent --show-error --output "${temp_dir}/other-user.html" --write-out '%{http_code}' \
    --cookie "${other_cookie}" "${base_url}/notifications?tenant_id=101")"
[[ "${other_status}" == "200" ]]
grep -Fq "${tenant_a_other_title}" "${temp_dir}/other-user.html"
! grep -Fq "${tenant_a_title}" "${temp_dir}/other-user.html"
grep -Eq 'Unread notifications: 1|Ungelesene Benachrichtigungen: 1' "${temp_dir}/other-user.html"

echo "[7/7] marking an owned notification as read with CSRF protection"
detail_status="$(curl --silent --show-error --output "${temp_dir}/notification-detail.html" --write-out '%{http_code}' \
    --cookie "${cookie}" "${base_url}/notifications/${notification_a_id}?tenant_id=101")"
[[ "${detail_status}" == "200" ]]
grep -Eq 'Mark as read|Als gelesen markieren' "${temp_dir}/notification-detail.html"
read_csrf="$(sed -n 's/.*name="_zelyra_csrf" value="\([^"]*\)".*/\1/p' "${temp_dir}/notification-detail.html")"
read_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --header "Origin: ${base_url}" --cookie "${cookie}" \
    --data-urlencode "_zelyra_csrf=${read_csrf}" \
    "${base_url}/notifications/${notification_a_id}/read?tenant_id=101")"
[[ "${read_status}" == "303" ]]
foreign_read_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --header "Origin: ${base_url}" --cookie "${cookie}" \
    --data-urlencode "_zelyra_csrf=${read_csrf}" \
    "${base_url}/notifications/${notification_other_id}/read?tenant_id=101")"
[[ "${foreign_read_status}" == "404" ]]
post_read_status="$(curl --silent --show-error --output "${temp_dir}/post-read.html" --write-out '%{http_code}' \
    --cookie "${cookie}" "${base_url}/notifications?tenant_id=101")"
[[ "${post_read_status}" == "200" ]]
grep -Eq 'Unread notifications: 0|Ungelesene Benachrichtigungen: 0' "${temp_dir}/post-read.html"
grep -Eq 'Notification marked as read\.|Benachrichtigung als gelesen markiert\.' "${temp_dir}/post-read.html"
read_detail_status="$(curl --silent --show-error --output "${temp_dir}/read-detail.html" --write-out '%{http_code}' \
    --cookie "${cookie}" "${base_url}/notifications/${notification_a_id}?tenant_id=101")"
[[ "${read_detail_status}" == "200" ]]
! grep -Eq 'Mark as read|Als gelesen markieren' "${temp_dir}/read-detail.html"

echo "MariaDB notification-center recipient isolation, CSRF-protected read receipt, unread count, expiry, and tenant-boundary checks passed."
