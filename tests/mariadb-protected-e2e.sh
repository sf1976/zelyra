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
managed_email="zelyra-protected-managed-${suffix}@example.test"
primary_role="zelyra-protected-manager-${suffix}"
viewer_role="zelyra-protected-viewer-${suffix}"
temporary_role="zelyra-protected-temporary-${suffix}"
admin_role="admin"
test_password="ZelyraProtected-${suffix}-Password"
managed_password="ZelyraManaged-${suffix}-Password"
managed_new_password="ZelyraManagedNew-${suffix}-Password"
customer_name="Zelyra Protected Customer-${suffix}"
created_customer_name="Zelyra Created Customer-${suffix}"
edited_customer_name="Zelyra Edited Customer-${suffix}"
custom_action_customer_name="Zelyra Custom Action Customer-${suffix}"
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
WHERE user_id IN (SELECT id FROM users WHERE email IN ('${primary_email}', '${secondary_email}', '${viewer_email}', '${managed_email}'));
DELETE FROM user_roles
WHERE user_id IN (SELECT id FROM users WHERE email IN ('${primary_email}', '${secondary_email}', '${viewer_email}', '${managed_email}'));
DELETE FROM role_permissions
WHERE role IN ('${primary_role}', '${viewer_role}', '${temporary_role}', '${admin_role}');
DELETE FROM auth_sessions
WHERE user_id IN (SELECT id FROM users WHERE email IN ('${primary_email}', '${secondary_email}', '${viewer_email}', '${managed_email}'));
DELETE FROM users WHERE email IN ('${primary_email}', '${secondary_email}', '${viewer_email}', '${managed_email}');
DELETE FROM customers
WHERE name IN ('${customer_name}', '${created_customer_name}', '${edited_customer_name}');
DELETE FROM customers WHERE name = '${custom_action_customer_name}';
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

echo "[2/10] creating users, role data, and a customer"
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
VALUES (@secondary_user_id, 'other.permission');
INSERT INTO customers (name) VALUES ('${customer_name}');
SQL
primary_user_id="$(client --batch --skip-column-names -e "SELECT id FROM users WHERE email = '${primary_email}'")"
viewer_user_id="$(client --batch --skip-column-names -e "SELECT id FROM users WHERE email = '${viewer_email}'")"
[[ -n "${primary_user_id}" ]]
[[ -n "${viewer_user_id}" ]]
DATABASE_URL="${database_url}" "${zelyra_bin}" auth role grant "${project_file}" "${primary_user_id}" "${primary_role}"
DATABASE_URL="${database_url}" "${zelyra_bin}" auth role grant "${project_file}" "${primary_user_id}" "${admin_role}"
DATABASE_URL="${database_url}" "${zelyra_bin}" auth role grant "${project_file}" "${viewer_user_id}" "${viewer_role}"
for permission in customers.view customers.create customers.edit customers.delete auth.manage; do
    DATABASE_URL="${database_url}" "${zelyra_bin}" auth role-permission grant \
        "${project_file}" "${primary_role}" "${permission}"
done
DATABASE_URL="${database_url}" "${zelyra_bin}" auth role-permission grant \
    "${project_file}" "${viewer_role}" customers.view
DATABASE_URL="${database_url}" "${zelyra_bin}" auth role grant \
    "${project_file}" "${primary_user_id}" "${primary_role}"
DATABASE_URL="${database_url}" "${zelyra_bin}" auth role-permission grant \
    "${project_file}" "${primary_role}" customers.view
DATABASE_URL="${database_url}" "${zelyra_bin}" auth role-permission grant \
    "${project_file}" "${admin_role}" auth.manage
primary_role_count="$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM user_roles WHERE user_id = '${primary_user_id}' AND role = '${primary_role}'")"
primary_permission_count="$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM role_permissions WHERE role = '${primary_role}' AND permission = 'customers.view'")"
[[ "${primary_role_count}" == "1" ]]
[[ "${primary_permission_count}" == "1" ]]
DATABASE_URL="${database_url}" "${zelyra_bin}" auth role grant \
    "${project_file}" "${primary_user_id}" "${temporary_role}"
DATABASE_URL="${database_url}" "${zelyra_bin}" auth role revoke \
    "${project_file}" "${primary_user_id}" "${temporary_role}"
temporary_role_count="$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM user_roles WHERE user_id = '${primary_user_id}' AND role = '${temporary_role}'")"
[[ "${temporary_role_count}" == "0" ]]
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
primary_login_audit_count="$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM auth_audit_log WHERE actor_user_id = '${primary_user_id}' AND event = 'auth.login' AND target_user_id = '${primary_user_id}'")"
[[ "${primary_login_audit_count}" == "1" ]]
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

primary_admin_status="$(request_status "${temp_dir}/primary-admin.html" \
    --cookie "${primary_cookie}" \
    "${base_url}/admin/access")"
[[ "${primary_admin_status}" == "200" ]]
grep -Fq "Role administration" "${temp_dir}/primary-admin.html"
admin_csrf="$(extract_csrf "${temp_dir}/primary-admin.html")"
[[ -n "${admin_csrf}" ]]
admin_grant_status="$(request_status "${temp_dir}/primary-admin-grant.html" \
    --cookie "${primary_cookie}" \
    --data-urlencode "_zelyra_csrf=${admin_csrf}" \
    --data-urlencode "operation=grant_role" \
    --data-urlencode "user_id=${viewer_user_id}" \
    --data-urlencode "role=${temporary_role}" \
    "${base_url}/admin/access")"
[[ "${admin_grant_status}" == "303" ]]
admin_role_count="$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM user_roles WHERE user_id = '${viewer_user_id}' AND role = '${temporary_role}'")"
[[ "${admin_role_count}" == "1" ]]
admin_create_user_status="$(request_status "${temp_dir}/primary-admin-create-user.html" \
    --cookie "${primary_cookie}" \
    --data-urlencode "_zelyra_csrf=${admin_csrf}" \
    --data-urlencode "operation=create_user" \
    --data-urlencode "email=${managed_email}" \
    --data-urlencode "password=${managed_password}" \
    "${base_url}/admin/access")"
[[ "${admin_create_user_status}" == "303" ]]
managed_user_id="$(client --batch --skip-column-names -e "SELECT id FROM users WHERE email = '${managed_email}'")"
[[ -n "${managed_user_id}" ]]
admin_deactivate_status="$(request_status "${temp_dir}/primary-admin-deactivate.html" \
    --cookie "${primary_cookie}" \
    --data-urlencode "_zelyra_csrf=${admin_csrf}" \
    --data-urlencode "operation=deactivate_user" \
    --data-urlencode "user_id=${managed_user_id}" \
    "${base_url}/admin/access")"
[[ "${admin_deactivate_status}" == "303" ]]
managed_login_status="$(request_status "${temp_dir}/managed-login-disabled.html" \
    --cookie-jar "${temp_dir}/managed.cookies" \
    --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "email=${managed_email}" \
    --data-urlencode "password=${managed_password}" \
    "${base_url}/login")"
[[ "${managed_login_status}" == "401" ]]
managed_failed_login_audit_count="$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM auth_audit_log WHERE event = 'auth.login_failed' AND target_user_id IS NULL")"
[[ "${managed_failed_login_audit_count}" == "1" ]]
admin_activate_status="$(request_status "${temp_dir}/primary-admin-activate.html" \
    --cookie "${primary_cookie}" \
    --data-urlencode "_zelyra_csrf=${admin_csrf}" \
    --data-urlencode "operation=activate_user" \
    --data-urlencode "user_id=${managed_user_id}" \
    "${base_url}/admin/access")"
[[ "${admin_activate_status}" == "303" ]]
managed_login_before_reset_status="$(request_status "${temp_dir}/managed-login-before-reset.html" \
    --cookie-jar "${temp_dir}/managed-before-reset.cookies" \
    --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "email=${managed_email}" \
    --data-urlencode "password=${managed_password}" \
    "${base_url}/login")"
[[ "${managed_login_before_reset_status}" == "303" ]]
admin_reset_password_status="$(request_status "${temp_dir}/primary-admin-reset-password.html" \
    --cookie "${primary_cookie}" \
    --data-urlencode "_zelyra_csrf=${admin_csrf}" \
    --data-urlencode "operation=reset_password" \
    --data-urlencode "user_id=${managed_user_id}" \
    --data-urlencode "password=${managed_new_password}" \
    "${base_url}/admin/access")"
[[ "${admin_reset_password_status}" == "303" ]]
managed_old_session_status="$(request_status "${temp_dir}/managed-old-session.html" \
    --cookie "${temp_dir}/managed-before-reset.cookies" \
    "${base_url}/admin/access")"
[[ "${managed_old_session_status}" == "401" ]]
audit_create_count="$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM auth_audit_log WHERE actor_user_id = '${primary_user_id}' AND event = 'user.create' AND target_user_id IS NULL AND details = 'email=${managed_email}'")"
audit_reset_count="$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM auth_audit_log WHERE actor_user_id = '${primary_user_id}' AND event = 'user.password_reset' AND target_user_id = '${managed_user_id}'")"
[[ "${audit_create_count}" == "1" ]]
[[ "${audit_reset_count}" == "1" ]]
curl --silent --show-error --fail --cookie "${primary_cookie}" \
    "${base_url}/admin/access" -o "${temp_dir}/primary-admin-audit.html"
grep -Fq "Audit log" "${temp_dir}/primary-admin-audit.html"
grep -Fq "user.create" "${temp_dir}/primary-admin-audit.html"
grep -Fq "user.password_reset" "${temp_dir}/primary-admin-audit.html"
managed_login_enabled_status="$(request_status "${temp_dir}/managed-login-enabled.html" \
    --cookie-jar "${temp_dir}/managed-enabled.cookies" \
    --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "email=${managed_email}" \
    --data-urlencode "password=${managed_new_password}" \
    "${base_url}/login")"
[[ "${managed_login_enabled_status}" == "303" ]]
managed_logout_status="$(request_status "${temp_dir}/managed-logout.html" \
    --cookie "${temp_dir}/managed-enabled.cookies" \
    --request POST \
    --data-urlencode "_zelyra_csrf=${csrf}" \
    "${base_url}/logout")"
[[ "${managed_logout_status}" == "303" ]]
managed_logout_count="$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM auth_audit_log WHERE actor_user_id = '${managed_user_id}' AND event = 'auth.logout' AND target_user_id = '${managed_user_id}'")"
[[ "${managed_logout_count}" == "1" ]]
admin_last_role_status="$(request_status "${temp_dir}/primary-admin-last-role.html" \
    --cookie "${primary_cookie}" \
    --data-urlencode "_zelyra_csrf=${admin_csrf}" \
    --data-urlencode "operation=revoke_role" \
    --data-urlencode "user_id=${primary_user_id}" \
    --data-urlencode "role=${admin_role}" \
    "${base_url}/admin/access")"
[[ "${admin_last_role_status}" == "409" ]]
grep -Fq "last administrator role assignment" "${temp_dir}/primary-admin-last-role.html"

custom_form_status="$(request_status "${temp_dir}/primary-custom-form.html" \
    --cookie "${primary_cookie}" \
    "${base_url}/forms/CustomerQuickCreate")"
[[ "${custom_form_status}" == "200" ]]
custom_form_csrf="$(extract_csrf "${temp_dir}/primary-custom-form.html")"
[[ -n "${custom_form_csrf}" ]]
custom_form_submit_status="$(request_status "${temp_dir}/primary-custom-form-submit.html" \
    --cookie "${primary_cookie}" \
    --data-urlencode "_zelyra_csrf=${custom_form_csrf}" \
    --data-urlencode "name=${custom_action_customer_name}" \
    "${base_url}/forms/CustomerQuickCreate")"
[[ "${custom_form_submit_status}" == "303" ]]

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
viewer_admin_status="$(request_status "${temp_dir}/viewer-admin.html" \
    --cookie "${viewer_cookie}" \
    "${base_url}/admin/access")"
[[ "${viewer_admin_status}" == "403" ]]
grep -Fq "Missing permission: auth.manage" "${temp_dir}/viewer-admin.html"
viewer_crud_status="$(request_status "${temp_dir}/viewer-crud.html" \
    --cookie "${viewer_cookie}" \
    "${base_url}/customers")"
[[ "${viewer_crud_status}" == "200" ]]
! grep -Fq 'href="/customers/new"' "${temp_dir}/viewer-crud.html"
viewer_custom_form_status="$(request_status "${temp_dir}/viewer-custom-form.html" \
    --cookie "${viewer_cookie}" \
    "${base_url}/forms/CustomerQuickCreate")"
[[ "${viewer_custom_form_status}" == "403" ]]
grep -Fq "Missing permission: customers.create" "${temp_dir}/viewer-custom-form.html"
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
