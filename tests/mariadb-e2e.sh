#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
project_file="${ZELYRA_E2E_PROJECT:-${repo_dir}/examples/machine_form.zyl}"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
address="${ZELYRA_E2E_ADDRESS:-127.0.0.1:38500}"
base_url="http://${address}"
database_url="${DATABASE_URL:-}"
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
if [[ "${db_host_port}" == *:* ]]; then
    db_port="${db_host_port##*:}"
else
    db_port="3306"
fi
db_password="${ZELYRA_E2E_DB_PASSWORD:-${db_password_from_url}}"
suffix="$(date +%s)"
department_name="Zelyra E2E Department-${suffix}"
secondary_department_name="Zelyra E2E Department Secondary-${suffix}"
machine_number="ZELYRA-E2E-${suffix}-1"
machine_two_number="ZELYRA-E2E-${suffix}-2"
machine_three_number="ZELYRA-E2E-${suffix}-3"
machine_name="Zelyra E2E Machine A-${suffix}"
machine_two_name="Zelyra E2E Search Target-${suffix}"
machine_three_name="Zelyra E2E Machine C-${suffix}"
updated_machine_name="Zelyra E2E Machine Updated-${suffix}"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-mariadb-e2e.XXXXXX")"
server_pid=""

cleanup() {
    if [[ -n "${server_pid}" ]]; then
        kill "${server_pid}" 2>/dev/null || true
        wait "${server_pid}" 2>/dev/null || true
    fi
    if command -v mariadb >/dev/null 2>&1; then
        MYSQL_PWD="${db_password}" mariadb \
            --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" \
            "${db_name}" --batch --skip-column-names >/dev/null 2>&1 <<SQL || true
DELETE FROM machines WHERE number IN ('${machine_number}', '${machine_two_number}', '${machine_three_number}');
DELETE FROM departments WHERE name IN ('${department_name}', '${secondary_department_name}');
SQL
    fi
    rm -rf "${temp_dir}"
}
trap cleanup EXIT

if [[ -z "${DATABASE_URL:-}" ]]; then
    echo "error: DATABASE_URL must point to a MariaDB test database" >&2
    exit 1
fi
if ! command -v curl >/dev/null 2>&1; then
    echo "error: curl is required for the MariaDB web integration test" >&2
    exit 1
fi
if ! command -v mariadb >/dev/null 2>&1; then
    echo "error: mariadb client is required for the MariaDB CRUD integration test" >&2
    exit 1
fi
if [[ ! -x "${zelyra_bin}" ]]; then
    echo "error: Zelyra binary not found at ${zelyra_bin}; run cargo build -p zelyra-cli first" >&2
    exit 1
fi

echo "[1/11] setting up MariaDB schema"
"${zelyra_bin}" db setup "${project_file}"

echo "[2/11] inspecting MariaDB schema"
inspect_output="$("${zelyra_bin}" db inspect "${project_file}")"
grep -Fq "2 tables" <<<"${inspect_output}"
grep -Fq "1 foreign keys" <<<"${inspect_output}"

echo "[3/11] checking schema plan"
plan_output="$("${zelyra_bin}" db plan "${project_file}")"
grep -Fq "No schema changes." <<<"${plan_output}"

echo "[4/11] starting Zelyra web server"
"${zelyra_bin}" serve "${project_file}" "${address}" >"${temp_dir}/server.log" 2>&1 &
server_pid=$!
for _ in $(seq 1 30); do
    if curl --silent --show-error --fail "${base_url}/machines" -o "${temp_dir}/health.html"; then
        break
    fi
    sleep 1
done
if ! curl --silent --show-error --fail "${base_url}/machines" -o "${temp_dir}/health.html"; then
    echo "error: Zelyra web server did not become ready" >&2
    cat "${temp_dir}/server.log" >&2
    exit 1
fi

extract_csrf() {
    sed -n 's/.*name="_zelyra_csrf" value="\([^"]*\)".*/\1/p' "$1"
}

post_form() {
    local response_file="$1"
    shift
    curl --silent --show-error --fail --output "${response_file}" --write-out '%{http_code}' "$@"
}

extract_option_id() {
    local html_file="$1"
    local label="$2"
    sed -n "s/.*<option value=\"\([^\"]*\)\">${label}<\\/option>.*/\1/p" "${html_file}"
}

create_machine() {
    local number="$1"
    local name="$2"
    local related_department_id="$3"
    local active="$4"
    local file_prefix="$5"

    curl --silent --show-error --fail "${base_url}/machines/new" -o "${temp_dir}/${file_prefix}-form.html"
    local csrf
    csrf="$(extract_csrf "${temp_dir}/${file_prefix}-form.html")"
    [[ -n "${csrf}" ]]
    local status
    status="$(post_form "${temp_dir}/${file_prefix}-response.html" \
        --data-urlencode "_zelyra_csrf=${csrf}" \
        --data-urlencode "number=${number}" \
        --data-urlencode "name=${name}" \
        --data-urlencode "department=${related_department_id}" \
        --data-urlencode "active=${active}" \
        "${base_url}/machines/new")"
    [[ "${status}" == "303" ]]

    curl --silent --show-error --fail --get \
        --data-urlencode "search=${number}" \
        "${base_url}/machines" -o "${temp_dir}/${file_prefix}-list.html"
    grep -Fq "${name}" "${temp_dir}/${file_prefix}-list.html"
    created_machine_id="$(sed -n 's#.*href="/machines/\([0-9][0-9]*\)">.*#\1#p' "${temp_dir}/${file_prefix}-list.html" | head -1)"
    [[ -n "${created_machine_id}" ]]
}

delete_machine() {
    local machine_id_to_delete="$1"
    local file_prefix="$2"
    curl --silent --show-error --fail "${base_url}/machines/${machine_id_to_delete}" -o "${temp_dir}/${file_prefix}-detail.html"
    local csrf
    csrf="$(extract_csrf "${temp_dir}/${file_prefix}-detail.html")"
    local status
    status="$(post_form "${temp_dir}/${file_prefix}-delete-response.html" \
        --data-urlencode "_zelyra_csrf=${csrf}" \
        "${base_url}/machines/${machine_id_to_delete}/delete")"
    [[ "${status}" == "303" ]]
}

echo "[5/11] creating related departments through CRUD"
curl --silent --show-error --fail "${base_url}/departments/new" -o "${temp_dir}/department-form.html"
department_csrf="$(extract_csrf "${temp_dir}/department-form.html")"
[[ -n "${department_csrf}" ]]
department_status="$(post_form "${temp_dir}/department-response.html" \
    --data-urlencode "_zelyra_csrf=${department_csrf}" \
    --data-urlencode "name=${department_name}" \
    "${base_url}/departments/new")"
[[ "${department_status}" == "303" ]]

curl --silent --show-error --fail "${base_url}/departments/new" -o "${temp_dir}/secondary-department-form.html"
secondary_department_csrf="$(extract_csrf "${temp_dir}/secondary-department-form.html")"
[[ -n "${secondary_department_csrf}" ]]
secondary_department_status="$(post_form "${temp_dir}/secondary-department-response.html" \
    --data-urlencode "_zelyra_csrf=${secondary_department_csrf}" \
    --data-urlencode "name=${secondary_department_name}" \
    "${base_url}/departments/new")"
[[ "${secondary_department_status}" == "303" ]]

echo "[6/11] creating and reading related machines"
curl --silent --show-error --fail "${base_url}/machines/new" -o "${temp_dir}/machine-form.html"
department_id="$(extract_option_id "${temp_dir}/machine-form.html" "${department_name}")"
secondary_department_id="$(extract_option_id "${temp_dir}/machine-form.html" "${secondary_department_name}")"
[[ -n "${department_id}" && -n "${secondary_department_id}" ]]
create_machine "${machine_number}" "${machine_name}" "${department_id}" true machine-one
machine_id="${created_machine_id}"
create_machine "${machine_two_number}" "${machine_two_name}" "${department_id}" false machine-two
machine_two_id="${created_machine_id}"
create_machine "${machine_three_number}" "${machine_three_name}" "${secondary_department_id}" true machine-three
machine_three_id="${created_machine_id}"
curl --silent --show-error --fail "${base_url}/machines" -o "${temp_dir}/machine-list.html"
grep -Fq "<th>Department</th>" "${temp_dir}/machine-list.html"
grep -Fq "<td>${department_name}</td>" "${temp_dir}/machine-list.html"
grep -Fq "<td>${machine_number}</td>" "${temp_dir}/machine-list.html"
curl --silent --show-error --fail "${base_url}/machines/${machine_id}" -o "${temp_dir}/machine-detail.html"
grep -Fq "<dt>Department</dt><dd>${department_name}</dd>" "${temp_dir}/machine-detail.html"

echo "[7/11] searching, filtering, sorting, and paginating"
curl --silent --show-error --fail --get \
    --data-urlencode "search=${suffix}" \
    "${base_url}/machines" -o "${temp_dir}/search.html"
grep -Fq "${machine_two_name}" "${temp_dir}/search.html"
grep -Fq "${machine_three_name}" "${temp_dir}/search.html"

curl --silent --show-error --fail --get \
    --data-urlencode "search=${suffix}" \
    --data-urlencode "filter_active=false" \
    "${base_url}/machines" -o "${temp_dir}/active-filter.html"
grep -Fq "${machine_two_name}" "${temp_dir}/active-filter.html"
! grep -Fq "${machine_name}" "${temp_dir}/active-filter.html"
! grep -Fq "${machine_three_name}" "${temp_dir}/active-filter.html"

curl --silent --show-error --fail --get \
    --data-urlencode "search=${suffix}" \
    --data-urlencode "filter_department=${secondary_department_id}" \
    "${base_url}/machines" -o "${temp_dir}/department-filter.html"
grep -Fq "${machine_three_name}" "${temp_dir}/department-filter.html"
! grep -Fq "${machine_name}" "${temp_dir}/department-filter.html"
! grep -Fq "${machine_two_name}" "${temp_dir}/department-filter.html"

curl --silent --show-error --fail --get \
    --data-urlencode "search=${suffix}" \
    --data-urlencode "sort=name" \
    --data-urlencode "order=desc" \
    --data-urlencode "per_page=3" \
    "${base_url}/machines" -o "${temp_dir}/sorted.html"
two_offset="$(grep -b -o -m1 "${machine_two_name}" "${temp_dir}/sorted.html" | cut -d: -f1)"
three_offset="$(grep -b -o -m1 "${machine_three_name}" "${temp_dir}/sorted.html" | cut -d: -f1)"
one_offset="$(grep -b -o -m1 "${machine_name}" "${temp_dir}/sorted.html" | cut -d: -f1)"
[[ "${two_offset}" -lt "${three_offset}" && "${three_offset}" -lt "${one_offset}" ]]

for page in 1 2 3; do
    curl --silent --show-error --fail --get \
        --data-urlencode "search=${suffix}" \
        --data-urlencode "sort=number" \
        --data-urlencode "order=asc" \
        --data-urlencode "per_page=1" \
        --data-urlencode "page=${page}" \
        "${base_url}/machines" -o "${temp_dir}/page-${page}.html"
done
grep -Fq "${machine_name}" "${temp_dir}/page-1.html"
! grep -Fq "${machine_two_name}" "${temp_dir}/page-1.html"
grep -Fq "${machine_two_name}" "${temp_dir}/page-2.html"
! grep -Fq "${machine_three_name}" "${temp_dir}/page-2.html"
grep -Fq "${machine_three_name}" "${temp_dir}/page-3.html"
grep -Fq "page=2" "${temp_dir}/page-1.html"

echo "[8/11] rejecting unknown CRUD query fields"
sort_status="$(curl --silent --show-error --output "${temp_dir}/invalid-sort.html" --write-out '%{http_code}' "${base_url}/machines?sort=not_allowed")"
[[ "${sort_status}" == "400" ]]
filter_status="$(curl --silent --show-error --output "${temp_dir}/invalid-filter.html" --write-out '%{http_code}' "${base_url}/machines?filter_not_allowed=value")"
[[ "${filter_status}" == "400" ]]

echo "[9/11] executing a typed custom CRUD action"
curl --silent --show-error --fail "${base_url}/machines/${machine_id}" -o "${temp_dir}/machine-action-detail.html"
grep -Fq "Set active status" "${temp_dir}/machine-action-detail.html"
grep -Fq 'data-icon="check"' "${temp_dir}/machine-action-detail.html"
grep -Fq "name=\"active\"" "${temp_dir}/machine-action-detail.html"
grep -Fq "Move department" "${temp_dir}/machine-action-detail.html"
grep -Fq 'data-icon="swap"' "${temp_dir}/machine-action-detail.html"
grep -Fq "href=\"/machines/${machine_id}/move_department\"" "${temp_dir}/machine-action-detail.html"
action_csrf="$(extract_csrf "${temp_dir}/machine-action-detail.html")"
invalid_action_status="$(curl --silent --show-error --output "${temp_dir}/invalid-action.html" --write-out '%{http_code}' \
    --data-urlencode "_zelyra_csrf=${action_csrf}" \
    --data-urlencode "active=not-a-boolean" \
    "${base_url}/machines/${machine_id}/set_active")"
[[ "${invalid_action_status}" == "422" ]]
action_status="$(curl --silent --show-error --fail --output "${temp_dir}/machine-action-response.html" --dump-header "${temp_dir}/machine-action-headers.html" --write-out '%{http_code}' \
    --data-urlencode "_zelyra_csrf=${action_csrf}" \
    --data-urlencode "active=false" \
    "${base_url}/machines/${machine_id}/set_active")"
[[ "${action_status}" == "303" ]]
grep -Fq "Location: /machines?zelyra_success=Machine%20status%20updated.&zelyra_success_title=Machine%20updated" "${temp_dir}/machine-action-headers.html"
curl --silent --show-error --fail "${base_url}/machines?zelyra_success=Machine%20status%20updated.&zelyra_success_title=Machine%20updated" -o "${temp_dir}/success-notice.html"
grep -Fq '<section class="zelyra-success" role="status"><h2>Machine updated</h2><p>Machine status updated.</p></section>' "${temp_dir}/success-notice.html"
curl --silent --show-error --fail --get \
    --data-urlencode "search=${suffix}" \
    --data-urlencode "filter_active=false" \
    "${base_url}/machines" -o "${temp_dir}/custom-action-result.html"
grep -Fq "${machine_name}" "${temp_dir}/custom-action-result.html"
grep -Fq "${machine_two_name}" "${temp_dir}/custom-action-result.html"
curl --silent --show-error --fail "${base_url}/machines/${machine_id}/move_department" -o "${temp_dir}/department-confirmation.html"
grep -Fq "Confirm department change" "${temp_dir}/department-confirmation.html"
grep -Fq "Please confirm that this machine should move" "${temp_dir}/department-confirmation.html"
grep -Fq "name=\"department\"" "${temp_dir}/department-confirmation.html"
grep -Fq "value=\"${secondary_department_id}\">${secondary_department_name}" "${temp_dir}/department-confirmation.html"
department_csrf="$(extract_csrf "${temp_dir}/department-confirmation.html")"
invalid_department_status="$(curl --silent --show-error --output "${temp_dir}/invalid-department-action.html" --write-out '%{http_code}' \
    --data-urlencode "_zelyra_csrf=${department_csrf}" \
    --data-urlencode "department=999999" \
    "${base_url}/machines/${machine_id}/move_department")"
[[ "${invalid_department_status}" == "422" ]]
department_action_status="$(post_form "${temp_dir}/department-action-response.html" \
    --data-urlencode "_zelyra_csrf=${department_csrf}" \
    --data-urlencode "department=${secondary_department_id}" \
    "${base_url}/machines/${machine_id}/move_department")"
[[ "${department_action_status}" == "303" ]]
curl --silent --show-error --fail --get \
    --data-urlencode "search=${suffix}" \
    --data-urlencode "filter_department=${secondary_department_id}" \
    "${base_url}/machines" -o "${temp_dir}/relationship-action-result.html"
grep -Fq "${machine_name}" "${temp_dir}/relationship-action-result.html"
! grep -Fq "${machine_two_name}" "${temp_dir}/relationship-action-result.html"

echo "[10/11] editing and deleting through CSRF-protected CRUD"
curl --silent --show-error --fail "${base_url}/machines/${machine_id}/edit" -o "${temp_dir}/machine-edit.html"
edit_csrf="$(extract_csrf "${temp_dir}/machine-edit.html")"
edit_status="$(post_form "${temp_dir}/machine-edit-response.html" \
    --data-urlencode "_zelyra_csrf=${edit_csrf}" \
    --data-urlencode "number=${machine_number}" \
    --data-urlencode "name=${updated_machine_name}" \
    --data-urlencode "department=${department_id}" \
    --data-urlencode "active=true" \
    "${base_url}/machines/${machine_id}/edit")"
[[ "${edit_status}" == "303" ]]
curl --silent --show-error --fail "${base_url}/machines/${machine_id}" -o "${temp_dir}/machine-updated.html"
grep -Fq "<dd>${updated_machine_name}</dd>" "${temp_dir}/machine-updated.html"
delete_csrf="$(extract_csrf "${temp_dir}/machine-updated.html")"
delete_status="$(post_form "${temp_dir}/machine-delete-response.html" \
    --data-urlencode "_zelyra_csrf=${delete_csrf}" \
    "${base_url}/machines/${machine_id}/delete")"
[[ "${delete_status}" == "303" ]]
curl --silent --show-error --fail "${base_url}/machines" -o "${temp_dir}/machine-list-after-delete.html"
! grep -Fq "${machine_number}" "${temp_dir}/machine-list-after-delete.html"
curl --silent --show-error --fail "${base_url}/machines?archived=true" -o "${temp_dir}/machine-list-archived.html"
grep -Fq "${machine_number}" "${temp_dir}/machine-list-archived.html"
grep -Fq "Show active records" "${temp_dir}/machine-list-archived.html"
curl --silent --show-error --fail "${base_url}/machines/${machine_id}?archived=true" -o "${temp_dir}/machine-archived-detail.html"
grep -Fq "Restore" "${temp_dir}/machine-archived-detail.html"
archived_restore_csrf="$(extract_csrf "${temp_dir}/machine-archived-detail.html")"
restore_status="$(post_form "${temp_dir}/machine-restore-response.html" \
    --data-urlencode "_zelyra_csrf=${archived_restore_csrf}" \
    "${base_url}/machines/${machine_id}/restore")"
[[ "${restore_status}" == "303" ]]
curl --silent --show-error --fail "${base_url}/machines" -o "${temp_dir}/machine-list-after-restore.html"
grep -Fq "${machine_number}" "${temp_dir}/machine-list-after-restore.html"
delete_machine "${machine_id}" machine-one-restored
curl --silent --show-error --fail "${base_url}/machines" -o "${temp_dir}/machine-list-after-final-delete.html"
! grep -Fq "${machine_number}" "${temp_dir}/machine-list-after-final-delete.html"

delete_machine "${machine_two_id}" machine-two
delete_machine "${machine_three_id}" machine-three

curl --silent --show-error --fail "${base_url}/machines" -o "${temp_dir}/machine-list-after-cleanup.html"
! grep -Fq "${machine_number}" "${temp_dir}/machine-list-after-cleanup.html"
! grep -Fq "${machine_two_number}" "${temp_dir}/machine-list-after-cleanup.html"
! grep -Fq "${machine_three_number}" "${temp_dir}/machine-list-after-cleanup.html"

echo "[11/11] cleaning the related departments"
MYSQL_PWD="${db_password}" mariadb \
    --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" \
    "${db_name}" --batch --skip-column-names <<SQL >/dev/null
DELETE FROM machines WHERE number IN ('${machine_number}', '${machine_two_number}', '${machine_three_number}');
SQL
for department_pair in "${department_id}:department" "${secondary_department_id}:secondary-department"; do
    department_to_delete="${department_pair%%:*}"
    department_file_prefix="${department_pair##*:}"
    curl --silent --show-error --fail "${base_url}/departments/${department_to_delete}" -o "${temp_dir}/${department_file_prefix}-detail.html"
    department_delete_csrf="$(extract_csrf "${temp_dir}/${department_file_prefix}-detail.html")"
    department_delete_status="$(post_form "${temp_dir}/${department_file_prefix}-delete-response.html" \
        --data-urlencode "_zelyra_csrf=${department_delete_csrf}" \
        "${base_url}/departments/${department_to_delete}/delete")"
    [[ "${department_delete_status}" == "303" ]]
done

echo "MariaDB E2E passed"
