#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
project_file="${ZELYRA_E2E_PROJECT:-${repo_dir}/examples/machine_form.zyl}"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
address="${ZELYRA_E2E_ADDRESS:-127.0.0.1:38500}"
base_url="http://${address}"
suffix="$(date +%s)"
department_name="Zelyra E2E Department-${suffix}"
machine_number="ZELYRA-E2E-${suffix}"
machine_name="Zelyra E2E Machine"
updated_machine_name="Zelyra E2E Machine Updated"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-mariadb-e2e.XXXXXX")"
server_pid=""

cleanup() {
    if [[ -n "${server_pid}" ]]; then
        kill "${server_pid}" 2>/dev/null || true
        wait "${server_pid}" 2>/dev/null || true
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
if [[ ! -x "${zelyra_bin}" ]]; then
    echo "error: Zelyra binary not found at ${zelyra_bin}; run cargo build -p zelyra-cli first" >&2
    exit 1
fi

echo "[1/8] setting up MariaDB schema"
"${zelyra_bin}" db setup "${project_file}"

echo "[2/8] inspecting MariaDB schema"
inspect_output="$("${zelyra_bin}" db inspect "${project_file}")"
grep -Fq "2 tables" <<<"${inspect_output}"
grep -Fq "1 foreign keys" <<<"${inspect_output}"

echo "[3/8] checking schema plan"
plan_output="$("${zelyra_bin}" db plan "${project_file}")"
grep -Fq "No schema changes." <<<"${plan_output}"

echo "[4/8] starting Zelyra web server"
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

echo "[5/8] creating a related department through CRUD"
curl --silent --show-error --fail "${base_url}/departments/new" -o "${temp_dir}/department-form.html"
department_csrf="$(extract_csrf "${temp_dir}/department-form.html")"
[[ -n "${department_csrf}" ]]
department_status="$(post_form "${temp_dir}/department-response.html" \
    --data-urlencode "_zelyra_csrf=${department_csrf}" \
    --data-urlencode "name=${department_name}" \
    "${base_url}/departments/new")"
[[ "${department_status}" == "303" ]]

echo "[6/8] creating and reading a related machine"
curl --silent --show-error --fail "${base_url}/machines/new" -o "${temp_dir}/machine-form.html"
machine_csrf="$(extract_csrf "${temp_dir}/machine-form.html")"
department_id="$(sed -n 's/.*<option value="\([^"]*\)">Zelyra E2E Department-[0-9]*<\/option>.*/\1/p' "${temp_dir}/machine-form.html")"
[[ -n "${machine_csrf}" && -n "${department_id}" ]]
machine_status="$(post_form "${temp_dir}/machine-response.html" \
    --data-urlencode "_zelyra_csrf=${machine_csrf}" \
    --data-urlencode "number=${machine_number}" \
    --data-urlencode "name=${machine_name}" \
    --data-urlencode "department=${department_id}" \
    --data-urlencode "active=true" \
    "${base_url}/machines/new")"
[[ "${machine_status}" == "303" ]]
curl --silent --show-error --fail "${base_url}/machines" -o "${temp_dir}/machine-list.html"
grep -Fq "<th>Department</th>" "${temp_dir}/machine-list.html"
grep -Fq "<td>${department_name}</td>" "${temp_dir}/machine-list.html"
grep -Fq "<td>${machine_number}</td>" "${temp_dir}/machine-list.html"
machine_id="$(sed -n 's#.*href="/machines/\([0-9][0-9]*\)">.*#\1#p' "${temp_dir}/machine-list.html" | head -1)"
[[ -n "${machine_id}" ]]
curl --silent --show-error --fail "${base_url}/machines/${machine_id}" -o "${temp_dir}/machine-detail.html"
grep -Fq "<dt>Department</dt><dd>${department_name}</dd>" "${temp_dir}/machine-detail.html"

echo "[7/8] editing and deleting through CSRF-protected CRUD"
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

echo "[8/8] cleaning the related department"
curl --silent --show-error --fail "${base_url}/departments/${department_id}" -o "${temp_dir}/department-detail.html"
department_delete_csrf="$(extract_csrf "${temp_dir}/department-detail.html")"
department_delete_status="$(post_form "${temp_dir}/department-delete-response.html" \
    --data-urlencode "_zelyra_csrf=${department_delete_csrf}" \
    "${base_url}/departments/${department_id}/delete")"
[[ "${department_delete_status}" == "303" ]]

echo "MariaDB E2E passed"
