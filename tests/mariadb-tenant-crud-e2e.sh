#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
project_file="${repo_dir}/examples/tenant_crud.zyl"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
address="127.0.0.1:38517"
base_url="http://${address}"
database_url="${DATABASE_URL:-}"
suffix="$(date +%s%N)"
email="zelyra-tenant-${suffix}@example.test"
permissionless_email="zelyra-tenant-no-permission-${suffix}@example.test"
membershipless_email="zelyra-tenant-no-membership-${suffix}@example.test"
password="ZelyraTenant-${suffix}-Password"
tenant_a_title="Tenant A invoice ${suffix}"
tenant_b_title="Tenant B invoice ${suffix}"
created_title="Created invoice ${suffix}"
edited_title="Edited invoice ${suffix}"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-mariadb-tenant-e2e.XXXXXX")"
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
db_password="${db_password_from_url}"

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
DELETE FROM invoices WHERE title IN ('${tenant_a_title}', '${tenant_b_title}', '${created_title}', '${edited_title}');
DELETE FROM user_permissions WHERE user_id IN (SELECT id FROM users WHERE email IN ('${email}', '${permissionless_email}', '${membershipless_email}'));
DELETE FROM auth_sessions WHERE user_id IN (SELECT id FROM users WHERE email IN ('${email}', '${permissionless_email}', '${membershipless_email}'));
DELETE FROM tenant_memberships WHERE user_id IN (SELECT id FROM users WHERE email IN ('${email}', '${permissionless_email}', '${membershipless_email}'));
DELETE FROM users WHERE email IN ('${email}', '${permissionless_email}', '${membershipless_email}');
SQL
    rm -rf "${temp_dir}"
}
trap cleanup EXIT

for command in mariadb curl; do
    if ! command -v "${command}" >/dev/null 2>&1; then
        echo "error: ${command} is required for the tenant integration test" >&2
        exit 1
    fi
done
if [[ ! -x "${zelyra_bin}" ]]; then
    echo "error: Zelyra binary not found at ${zelyra_bin}; run cargo build -p zelyra-cli first" >&2
    exit 1
fi

echo "[1/12] setting up tenant CRUD schema"
DATABASE_URL="${database_url}" "${zelyra_bin}" db setup "${project_file}"
password_hash="$(printf '%s\n' "${password}" | "${zelyra_bin}" auth hash-password --stdin)"
permissionless_hash="$(printf '%s\n' "${password}" | "${zelyra_bin}" auth hash-password --stdin)"
membershipless_hash="$(printf '%s\n' "${password}" | "${zelyra_bin}" auth hash-password --stdin)"
client <<SQL
INSERT INTO users (email, password_hash)
VALUES ('${email}', '${password_hash}'),
       ('${permissionless_email}', '${permissionless_hash}'),
       ('${membershipless_email}', '${membershipless_hash}');
SET @user_id = (SELECT id FROM users WHERE email = '${email}');
SET @permissionless_user_id = (SELECT id FROM users WHERE email = '${permissionless_email}');
SET @membershipless_user_id = (SELECT id FROM users WHERE email = '${membershipless_email}');
INSERT INTO tenant_memberships (user_id, tenant_id, active)
VALUES (@user_id, 10, TRUE), (@user_id, 20, TRUE), (@permissionless_user_id, 10, TRUE);
INSERT INTO user_permissions (user_id, permission)
VALUES (@user_id, 'invoices.view'), (@membershipless_user_id, 'invoices.view');
INSERT INTO invoices (tenant_id, title)
VALUES (10, '${tenant_a_title}'), (20, '${tenant_b_title}');
SQL
tenant_b_id="$(client --batch --skip-column-names -e "SELECT id FROM invoices WHERE title = '${tenant_b_title}'")"
tenant_a_id="$(client --batch --skip-column-names -e "SELECT id FROM invoices WHERE title = '${tenant_a_title}'")"
[[ -n "${tenant_b_id}" ]]
[[ -n "${tenant_a_id}" ]]

echo "[2/12] starting tenant application and authenticating"
"${zelyra_bin}" serve "${project_file}" "${address}" >"${temp_dir}/server.log" 2>&1 &
server_pid=$!
for _ in $(seq 1 30); do
    if curl --silent --show-error --fail "${base_url}/login" -o "${temp_dir}/login.html"; then
        break
    fi
    sleep 1
done
curl --silent --show-error --fail "${base_url}/login" -o "${temp_dir}/login.html"
csrf="$(sed -n 's/.*name="_zelyra_csrf" value="\([^"]*\)".*/\1/p' "${temp_dir}/login.html")"
[[ -n "${csrf}" ]]
cookie="${temp_dir}/session.cookies"
login_status="$(curl --silent --show-error --output "${temp_dir}/login-response.html" \
    --write-out '%{http_code}' --header "Origin: ${base_url}" --cookie-jar "${cookie}" \
    --data-urlencode "_zelyra_csrf=${csrf}" --data-urlencode "email=${email}" \
    --data-urlencode "password=${password}" "${base_url}/login")"
[[ "${login_status}" == "303" ]]

echo "[3/12] requiring explicit selection for a multi-tenant user"
ambiguous_status="$(curl --silent --show-error --output "${temp_dir}/ambiguous.html" \
    --write-out '%{http_code}' --cookie "${cookie}" "${base_url}/invoices")"
[[ "${ambiguous_status}" == "403" ]]

echo "[4/12] enforcing permission and membership independently"
permissionless_cookie="${temp_dir}/permissionless.cookies"
permissionless_login_status="$(curl --silent --show-error --output "${temp_dir}/permissionless-login.html" \
    --write-out '%{http_code}' --header "Origin: ${base_url}" --cookie-jar "${permissionless_cookie}" \
    --data-urlencode "_zelyra_csrf=${csrf}" --data-urlencode "email=${permissionless_email}" \
    --data-urlencode "password=${password}" "${base_url}/login")"
[[ "${permissionless_login_status}" == "303" ]]
permissionless_status="$(curl --silent --show-error --output "${temp_dir}/permissionless.html" \
    --write-out '%{http_code}' --cookie "${permissionless_cookie}" \
    "${base_url}/invoices?tenant_id=10")"
[[ "${permissionless_status}" == "403" ]]

membershipless_cookie="${temp_dir}/membershipless.cookies"
membershipless_login_status="$(curl --silent --show-error --output "${temp_dir}/membershipless-login.html" \
    --write-out '%{http_code}' --header "Origin: ${base_url}" --cookie-jar "${membershipless_cookie}" \
    --data-urlencode "_zelyra_csrf=${csrf}" --data-urlencode "email=${membershipless_email}" \
    --data-urlencode "password=${password}" "${base_url}/login")"
[[ "${membershipless_login_status}" == "303" ]]
membershipless_status="$(curl --silent --show-error --output "${temp_dir}/membershipless.html" \
    --write-out '%{http_code}' --cookie "${membershipless_cookie}" \
    "${base_url}/invoices?tenant_id=10")"
[[ "${membershipless_status}" == "403" ]]

echo "[5/12] isolating tenant list and detail reads"
list_status="$(curl --silent --show-error --output "${temp_dir}/tenant-a.html" \
    --write-out '%{http_code}' --cookie "${cookie}" "${base_url}/invoices?tenant_id=10")"
[[ "${list_status}" == "200" ]]
grep -Fq "${tenant_a_title}" "${temp_dir}/tenant-a.html"
! grep -Fq "${tenant_b_title}" "${temp_dir}/tenant-a.html"
! grep -Fq 'tenant_id' "${temp_dir}/tenant-a.html"
grep -Fq 'href="/invoices/new?tenant_id=10"' "${temp_dir}/tenant-a.html"
cross_tenant_detail_status="$(curl --silent --show-error --output "${temp_dir}/cross-tenant.html" \
    --write-out '%{http_code}' --cookie "${cookie}" \
    "${base_url}/invoices/${tenant_b_id}?tenant_id=10")"
[[ "${cross_tenant_detail_status}" == "404" ]]
tenant_b_status="$(curl --silent --show-error --output "${temp_dir}/tenant-b.html" \
    --write-out '%{http_code}' --cookie "${cookie}" \
    --header 'X-Zelyra-Tenant: 20' "${base_url}/invoices")"
[[ "${tenant_b_status}" == "200" ]]
grep -Fq "${tenant_b_title}" "${temp_dir}/tenant-b.html"
! grep -Fq "${tenant_a_title}" "${temp_dir}/tenant-b.html"

echo "[6/12] rejecting conflicting and forged selectors"
conflict_status="$(curl --silent --show-error --output "${temp_dir}/conflict.html" \
    --write-out '%{http_code}' --cookie "${cookie}" \
    --header 'X-Zelyra-Tenant: 20' "${base_url}/invoices?tenant_id=10")"
[[ "${conflict_status}" == "400" ]]
forged_status="$(curl --silent --show-error --output "${temp_dir}/forged.html" \
    --write-out '%{http_code}' --cookie "${cookie}" \
    "${base_url}/invoices?tenant_id=30")"
[[ "${forged_status}" == "403" ]]

echo "[7/12] creating with a client-supplied tenant key"
form_status="$(curl --silent --show-error --output "${temp_dir}/create-form.html" \
    --write-out '%{http_code}' --cookie "${cookie}" \
    "${base_url}/invoices/new?tenant_id=10")"
[[ "${form_status}" == "200" ]]
grep -Fq 'action="/invoices/new?tenant_id=10"' "${temp_dir}/create-form.html"
form_csrf="$(sed -n 's/.*name="_zelyra_csrf" value="\([^"]*\)".*/\1/p' "${temp_dir}/create-form.html")"
[[ -n "${form_csrf}" ]]
forged_create_status="$(curl --silent --show-error --output "${temp_dir}/forged-create-response.html" \
    --write-out '%{http_code}' --header "Origin: ${base_url}" --cookie "${cookie}" \
    --data-urlencode "_zelyra_csrf=${form_csrf}" --data-urlencode "title=${created_title}" \
    --data-urlencode 'tenant_id=20' "${base_url}/invoices/new?tenant_id=10")"
[[ "${forged_create_status}" == "422" ]]
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM invoices WHERE title = '${created_title}'")" == "0" ]]
create_status="$(curl --silent --show-error --output "${temp_dir}/create-response.html" \
    --write-out '%{http_code}' --dump-header "${temp_dir}/create-headers.txt" \
    --header "Origin: ${base_url}" --cookie "${cookie}" \
    --data-urlencode "_zelyra_csrf=${form_csrf}" --data-urlencode "title=${created_title}" \
    "${base_url}/invoices/new?tenant_id=10")"
[[ "${create_status}" == "303" ]]
grep -Fq 'Location: /invoices?' "${temp_dir}/create-headers.txt"
grep -Fq 'tenant_id=10' "${temp_dir}/create-headers.txt"
created_tenant="$(client --batch --skip-column-names -e "SELECT tenant_id FROM invoices WHERE title = '${created_title}'")"
[[ "${created_tenant}" == "10" ]]

echo "[8/12] editing an owned record and rejecting a cross-tenant edit"
edit_status="$(curl --silent --show-error --output "${temp_dir}/edit-a.html" \
    --write-out '%{http_code}' --cookie "${cookie}" \
    "${base_url}/invoices/${tenant_a_id}/edit?tenant_id=10")"
[[ "${edit_status}" == "200" ]]
edit_csrf="$(sed -n 's/.*name="_zelyra_csrf" value="\([^"]*\)".*/\1/p' "${temp_dir}/edit-a.html")"
edit_snapshot="$(sed -n 's/.*name="_zelyra_snapshot" value="\([^"]*\)".*/\1/p' "${temp_dir}/edit-a.html")"
[[ -n "${edit_csrf}" && -n "${edit_snapshot}" ]]
edit_success="$(curl --silent --show-error --output "${temp_dir}/edit-success.html" \
    --write-out '%{http_code}' --header "Origin: ${base_url}" --cookie "${cookie}" \
    --data-urlencode "_zelyra_csrf=${edit_csrf}" \
    --data-urlencode "_zelyra_snapshot=${edit_snapshot}" \
    --data-urlencode "title=${edited_title}" \
    "${base_url}/invoices/${tenant_a_id}/edit?tenant_id=10")"
[[ "${edit_success}" == "303" ]]
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM invoices WHERE id = ${tenant_a_id} AND title = '${edited_title}' AND tenant_id = 10")" == "1" ]]

tenant_b_edit_status="$(curl --silent --show-error --output "${temp_dir}/edit-b.html" \
    --write-out '%{http_code}' --cookie "${cookie}" \
    "${base_url}/invoices/${tenant_b_id}/edit?tenant_id=20")"
[[ "${tenant_b_edit_status}" == "200" ]]
tenant_b_snapshot="$(sed -n 's/.*name="_zelyra_snapshot" value="\([^"]*\)".*/\1/p' "${temp_dir}/edit-b.html")"
tenant_b_edit_csrf="$(sed -n 's/.*name="_zelyra_csrf" value="\([^"]*\)".*/\1/p' "${temp_dir}/edit-b.html")"
cross_edit_status="$(curl --silent --show-error --output "${temp_dir}/cross-edit.html" \
    --write-out '%{http_code}' --header "Origin: ${base_url}" --cookie "${cookie}" \
    --data-urlencode "_zelyra_csrf=${tenant_b_edit_csrf}" \
    --data-urlencode "_zelyra_snapshot=${tenant_b_snapshot}" \
    --data-urlencode "title=${edited_title}" \
    "${base_url}/invoices/${tenant_b_id}/edit?tenant_id=10")"
[[ "${cross_edit_status}" == "404" ]]
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM invoices WHERE id = ${tenant_b_id} AND title = '${tenant_b_title}' AND tenant_id = 20")" == "1" ]]

echo "[9/12] rejecting cross-tenant delete and allowing scoped delete/restore"
tenant_b_detail_status="$(curl --silent --show-error --output "${temp_dir}/tenant-b-detail.html" \
    --write-out '%{http_code}' --cookie "${cookie}" \
    "${base_url}/invoices/${tenant_b_id}?tenant_id=20")"
[[ "${tenant_b_detail_status}" == "200" ]]
tenant_b_csrf="$(sed -n 's/.*name="_zelyra_csrf" value="\([^"]*\)".*/\1/p' "${temp_dir}/tenant-b-detail.html")"
cross_delete_status="$(curl --silent --show-error --output "${temp_dir}/cross-delete.html" \
    --write-out '%{http_code}' --header "Origin: ${base_url}" --cookie "${cookie}" \
    --data-urlencode "_zelyra_csrf=${tenant_b_csrf}" \
    "${base_url}/invoices/${tenant_b_id}/delete?tenant_id=10")"
[[ "${cross_delete_status}" == "404" ]]
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM invoices WHERE id = ${tenant_b_id} AND tenant_id = 20")" == "1" ]]

tenant_a_detail_status="$(curl --silent --show-error --output "${temp_dir}/tenant-a-detail.html" \
    --write-out '%{http_code}' --cookie "${cookie}" \
    "${base_url}/invoices/${tenant_a_id}?tenant_id=10")"
[[ "${tenant_a_detail_status}" == "200" ]]
tenant_a_csrf="$(sed -n 's/.*name="_zelyra_csrf" value="\([^"]*\)".*/\1/p' "${temp_dir}/tenant-a-detail.html")"
delete_status="$(curl --silent --show-error --output "${temp_dir}/delete-a.html" \
    --write-out '%{http_code}' --header "Origin: ${base_url}" --cookie "${cookie}" \
    --data-urlencode "_zelyra_csrf=${tenant_a_csrf}" \
    "${base_url}/invoices/${tenant_a_id}/delete?tenant_id=10")"
[[ "${delete_status}" == "303" ]]
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM invoices WHERE id = ${tenant_a_id} AND deleted_at IS NOT NULL")" == "1" ]]
restore_status="$(curl --silent --show-error --output "${temp_dir}/restore-a.html" \
    --write-out '%{http_code}' --header "Origin: ${base_url}" --cookie "${cookie}" \
    --data-urlencode "_zelyra_csrf=${tenant_a_csrf}" \
    "${base_url}/invoices/${tenant_a_id}/restore?tenant_id=10")"
[[ "${restore_status}" == "303" ]]
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM invoices WHERE id = ${tenant_a_id} AND deleted_at IS NULL")" == "1" ]]

echo "[10/12] failing closed when membership lookup fails"
client -e "RENAME TABLE tenant_memberships TO tenant_memberships_unavailable"
membership_failure_status="$(curl --silent --show-error --output "${temp_dir}/membership-failure.html" \
    --write-out '%{http_code}' --cookie "${cookie}" \
    "${base_url}/invoices?tenant_id=10")"
[[ "${membership_failure_status}" == "503" ]]
! grep -Eiq 'tenant_memberships|unknown table|database error' "${temp_dir}/membership-failure.html"
client -e "RENAME TABLE tenant_memberships_unavailable TO tenant_memberships"

echo "[11/12] serializing concurrent membership revocation and delete"
membership_lock_name="zelyra-tenant-${suffix}"
client --batch --skip-column-names >"${temp_dir}/membership-lock.txt" <<SQL &
START TRANSACTION;
SELECT id FROM tenant_memberships
WHERE user_id = (SELECT id FROM users WHERE email = '${email}')
  AND tenant_id = 10 AND active = TRUE
FOR UPDATE;
SELECT GET_LOCK('${membership_lock_name}', 0);
DO SLEEP(3);
UPDATE tenant_memberships SET active = FALSE
WHERE user_id = (SELECT id FROM users WHERE email = '${email}') AND tenant_id = 10;
COMMIT;
SELECT RELEASE_LOCK('${membership_lock_name}');
SQL
membership_lock_pid=$!
for _ in $(seq 1 60); do
    lock_ready="$(client --batch --skip-column-names -e "SELECT IS_USED_LOCK('${membership_lock_name}') IS NOT NULL")"
    if [[ "${lock_ready}" == "1" ]]; then
        break
    fi
    sleep 0.05
done
[[ "${lock_ready}" == "1" ]]
revoked_delete_status="$(curl --silent --show-error --output "${temp_dir}/revoked-delete.html" \
    --write-out '%{http_code}' --header "Origin: ${base_url}" --cookie "${cookie}" \
    --data-urlencode "_zelyra_csrf=${tenant_a_csrf}" \
    "${base_url}/invoices/${tenant_a_id}/delete?tenant_id=10")"
wait "${membership_lock_pid}"
[[ "${revoked_delete_status}" == "404" ]]
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM invoices WHERE id = ${tenant_a_id} AND deleted_at IS NULL")" == "1" ]]

echo "[12/12] checking revoked membership takes effect immediately"
revoked_status="$(curl --silent --show-error --output "${temp_dir}/revoked.html" \
    --write-out '%{http_code}' --cookie "${cookie}" \
    "${base_url}/invoices?tenant_id=10")"
[[ "${revoked_status}" == "403" ]]
echo "MariaDB tenant CRUD isolation checks passed"
