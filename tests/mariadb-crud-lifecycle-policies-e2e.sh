#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
project_file="${repo_dir}/examples/crud_lifecycle_policies/main.zyl"
database_url="${DATABASE_URL:-}"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-crud-policy-e2e.XXXXXX")"
suffix="$(date +%s)"
invoice_status="p${suffix}"
server_pid=""
lock_pid=""

cleanup() {
    local status=$?
    if [[ -n "${server_pid}" ]]; then
        kill "${server_pid}" 2>/dev/null || true
        wait "${server_pid}" 2>/dev/null || true
    fi
    if [[ -n "${lock_pid}" ]]; then
        kill "${lock_pid}" 2>/dev/null || true
        wait "${lock_pid}" 2>/dev/null || true
    fi
    if [[ -n "${db_host:-}" && -n "${db_name:-}" ]]; then
        MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" \
            --user="${db_user}" "${db_name}" --batch --skip-column-names \
            -e "DELETE FROM invoices WHERE status='${invoice_status}'" >/dev/null 2>&1 || true
    fi
    if [[ "${status}" -ne 0 && -f "${temp_dir}/server.log" ]]; then
        cat "${temp_dir}/server.log" >&2
    fi
    for file in "${temp_dir}"/*; do
        [[ -e "${file}" ]] && unlink "${file}" 2>/dev/null || true
    done
    rmdir "${temp_dir}" 2>/dev/null || true
    return "${status}"
}
trap cleanup EXIT

if [[ "${database_url}" != mariadb://* && "${database_url}" != mysql://* ]]; then
    echo "error: set DATABASE_URL to a disposable MariaDB/MySQL test database" >&2
    exit 1
fi
if [[ ! -x "${zelyra_bin}" ]]; then
    echo "error: build the CLI first; expected ${zelyra_bin}" >&2
    exit 1
fi
for command in mariadb curl python3; do
    command -v "${command}" >/dev/null || { echo "error: ${command} is required" >&2; exit 1; }
done

url_parts="${database_url#*://}"
credentials="${url_parts%@*}"
location="${url_parts#*@}"
host_port="${location%%/*}"
db_name="${location#*/}"
db_name="${db_name%%\?*}"
db_user="${credentials%%:*}"
db_password="${credentials#*:}"
db_host="${host_port%%:*}"
db_port="${host_port##*:}"
if [[ "${db_name,,}" != *test* ]]; then
    echo "error: refusing lifecycle E2E against a database whose name does not contain 'test'" >&2
    exit 1
fi

client() {
    MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" \
        --user="${db_user}" "${db_name}" "$@"
}

echo "[1/7] checking the lifecycle fixture"
"${zelyra_bin}" check "${project_file}"
echo "[2/7] setting up the test schema"
DATABASE_URL="${database_url}" "${zelyra_bin}" db setup "${project_file}"
client --batch --skip-column-names -e \
    "INSERT INTO invoices (status, locked) VALUES ('${invoice_status}', FALSE)"
record_id="$(client --batch --skip-column-names -e \
    "SELECT id FROM invoices WHERE status='${invoice_status}' LIMIT 1")"
[[ -n "${record_id}" ]]

port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()')"
base_url="http://127.0.0.1:${port}"
"${zelyra_bin}" serve "${project_file}" "127.0.0.1:${port}" >"${temp_dir}/server.log" 2>&1 &
server_pid=$!
for _ in $(seq 1 30); do
    if curl --silent --fail "${base_url}/invoices/${record_id}" -o "${temp_dir}/detail-before.html"; then
        break
    fi
    sleep 1
done
curl --silent --show-error --fail "${base_url}/invoices/${record_id}" -o "${temp_dir}/detail-before.html"
curl --silent --show-error --fail "${base_url}/views/invoicerows" -o "${temp_dir}/tableview-before.html"
grep -Fq "/invoices/${record_id}/edit" "${temp_dir}/detail-before.html"
grep -Fq "Bearbeiten</a>" "${temp_dir}/tableview-before.html"
grep -Fq "Löschen</button>" "${temp_dir}/tableview-before.html"

echo "[3/7] loading an edit snapshot and delete CSRF token while unlocked"
curl --silent --show-error --fail "${base_url}/invoices/${record_id}/edit" -o "${temp_dir}/edit.html"
edit_csrf="$(sed -n 's/.*name="_zelyra_csrf" value="\([^"]*\)".*/\1/p' "${temp_dir}/edit.html")"
snapshot="$(sed -n 's/.*name="_zelyra_snapshot" value="\([^"]*\)".*/\1/p' "${temp_dir}/edit.html")"
delete_csrf="$(sed -n 's/.*action="[^"]*/&/p' "${temp_dir}/detail-before.html" | sed -n 's/.*name="_zelyra_csrf" value="\([^"]*\)".*/\1/p' | head -n 1)"
[[ -n "${edit_csrf}" && -n "${snapshot}" && -n "${delete_csrf}" ]]

echo "[4/7] locking the row after form load, then rejecting stale edit POST"
(
    client --batch <<SQL
START TRANSACTION;
SELECT id FROM invoices WHERE id=${record_id} FOR UPDATE;
UPDATE invoices SET locked=TRUE WHERE id=${record_id};
DO SLEEP(2);
COMMIT;
SQL
) >"${temp_dir}/row-lock.log" 2>&1 &
lock_pid=$!
sleep 0.2
edit_status="$(curl --silent --show-error --output "${temp_dir}/edit-denied.html" \
    --write-out '%{http_code}' --header "Origin: ${base_url}" \
    --data-urlencode "_zelyra_csrf=${edit_csrf}" --data-urlencode "_zelyra_snapshot=${snapshot}" \
    --data-urlencode 'status=sent' --data-urlencode 'locked=false' \
    "${base_url}/invoices/${record_id}/edit")"
wait "${lock_pid}"
lock_pid=""
[[ "${edit_status}" == "403" ]]
actual_status="$(client --batch --skip-column-names -e "SELECT status FROM invoices WHERE id=${record_id}")"
actual_locked="$(client --batch --skip-column-names -e "SELECT locked FROM invoices WHERE id=${record_id}")"
[[ "${actual_status}" == "${invoice_status}" && "${actual_locked}" == "1" ]]

echo "[5/7] rejecting direct guarded delete POST"
delete_status="$(curl --silent --show-error --output "${temp_dir}/delete-denied.html" \
    --write-out '%{http_code}' --header "Origin: ${base_url}" \
    --data-urlencode "_zelyra_csrf=${delete_csrf}" \
    "${base_url}/invoices/${record_id}/delete")"
[[ "${delete_status}" == "403" ]]
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM invoices WHERE id=${record_id}")" == "1" ]]

echo "[6/7] hiding edit/delete actions on details and policy-scoped TableView rows"
locked_edit_status="$(curl --silent --show-error --output "${temp_dir}/edit-get-denied.html" \
    --write-out '%{http_code}' "${base_url}/invoices/${record_id}/edit")"
[[ "${locked_edit_status}" == "403" ]]
curl --silent --show-error --fail "${base_url}/invoices/${record_id}" -o "${temp_dir}/detail-locked.html"
curl --silent --show-error --fail "${base_url}/views/invoicerows" -o "${temp_dir}/tableview-locked.html"
! grep -Fq "/invoices/${record_id}/edit" "${temp_dir}/detail-locked.html"
! grep -Fq "action=\"/invoices/${record_id}/delete\"" "${temp_dir}/detail-locked.html"
! grep -Fq "/invoices/${record_id}/edit" "${temp_dir}/tableview-locked.html"
! grep -Fq "Löschen</button>" "${temp_dir}/tableview-locked.html"

echo "[7/7] all lifecycle policy checks passed"
