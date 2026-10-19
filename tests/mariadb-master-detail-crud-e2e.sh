#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
project_file="${repo_dir}/examples/master_detail/main.zyl"
database_url="${DATABASE_URL:-}"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-master-detail-e2e.XXXXXX")"
token="$(date +%s)-$$"
server_pid=""
invoice_id=""

cleanup() {
    local status=$?
    if [[ -n "${server_pid}" ]]; then
        kill "${server_pid}" 2>/dev/null || true
        wait "${server_pid}" 2>/dev/null || true
    fi
    if [[ -n "${invoice_id}" && -n "${db_host:-}" && -n "${db_name:-}" ]]; then
        client -e "DELETE FROM invoices WHERE id=${invoice_id}" >/dev/null 2>&1 || true
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
    echo "error: set DATABASE_URL to a disposable MariaDB test database" >&2
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
    echo "error: refusing master-detail E2E against a database whose name does not contain 'test'" >&2
    exit 1
fi

client() {
    MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" \
        --user="${db_user}" "${db_name}" "$@"
}

input_value() {
    python3 -c 'import sys
from html.parser import HTMLParser
class Inputs(HTMLParser):
    def __init__(self): super().__init__(); self.values = {}
    def handle_starttag(self, tag, attrs):
        if tag == "input":
            item = dict(attrs)
            if item.get("name"): self.values[item["name"]] = item.get("value", "")
p = Inputs(); p.feed(open(sys.argv[1], encoding="utf-8").read()); print(p.values.get(sys.argv[2], ""))' "$1" "$2"
}

echo "[1/7] checking and applying the master-detail fixture schema"
"${zelyra_bin}" check "${project_file}"
DATABASE_URL="${database_url}" "${zelyra_bin}" db setup "${project_file}"
port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()')"
base_url="http://127.0.0.1:${port}"
"${zelyra_bin}" serve "${project_file}" "127.0.0.1:${port}" >"${temp_dir}/server.log" 2>&1 &
server_pid=$!
for _ in $(seq 1 30); do
    if curl --silent --fail "${base_url}/invoices/new" -o "${temp_dir}/create.html"; then break; fi
    sleep 1
done
curl --silent --show-error --fail "${base_url}/invoices/new" -o "${temp_dir}/create.html"
grep -Fq 'name="items.0.description"' "${temp_dir}/create.html"

csrf="$(input_value "${temp_dir}/create.html" _zelyra_csrf)"
[[ -n "${csrf}" ]]
echo "[2/7] verifying server-rendered add/remove row actions preserve input"
curl --silent --show-error --fail --header "Origin: ${base_url}" \
    --data-urlencode "_zelyra_csrf=${csrf}" --data-urlencode '_zelyra_child_count=1' \
    --data-urlencode 'items.0.description=preserved' --data-urlencode 'items.0.position=1' \
    --data-urlencode 'items.0.total_net=1.00' --data-urlencode 'items.0.total_gross=1.10' \
    --data-urlencode '_zelyra_child_action=add' "${base_url}/invoices/new" -o "${temp_dir}/added-row.html"
grep -Fq 'value="preserved"' "${temp_dir}/added-row.html"
grep -Fq 'name="items.1.description"' "${temp_dir}/added-row.html"

echo "[3/7] checking child invariants and rolling back a failed parent/child save"
before_count="$(client --batch --skip-column-names -e 'SELECT COUNT(*) FROM invoices')"
invalid_status="$(curl --silent --show-error --output "${temp_dir}/invalid.html" --write-out '%{http_code}' \
    --header "Origin: ${base_url}" --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode '_zelyra_child_count=1' --data-urlencode 'invoice_date=2026-10-01' \
    --data-urlencode 'items.0.position=1' --data-urlencode "items.0.description=${token}-invalid" \
    --data-urlencode 'items.0.total_net=10.00' --data-urlencode 'items.0.total_gross=8.00' \
    "${base_url}/invoices/new")"
[[ "${invalid_status}" == "422" ]]
[[ "$(client --batch --skip-column-names -e 'SELECT COUNT(*) FROM invoices')" == "${before_count}" ]]

echo "[4/7] saving an invoice and two lines atomically with computed totals"
save_status="$(curl --silent --show-error --output "${temp_dir}/saved.html" --write-out '%{http_code}' \
    --header "Origin: ${base_url}" --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode '_zelyra_child_count=2' --data-urlencode 'invoice_date=2026-10-02' \
    --data-urlencode 'items.0.position=1' --data-urlencode "items.0.description=${token}-A" \
    --data-urlencode 'items.0.total_net=100.00' --data-urlencode 'items.0.total_gross=110.00' \
    --data-urlencode 'items.1.position=2' --data-urlencode "items.1.description=${token}-B" \
    --data-urlencode 'items.1.total_net=50.00' --data-urlencode 'items.1.total_gross=55.00' \
    "${base_url}/invoices/new")"
if [[ "${save_status}" != "303" ]]; then
    echo "expected create POST to redirect (303), received ${save_status}:" >&2
    cat "${temp_dir}/saved.html" >&2
    exit 1
fi
invoice_id="$(client --batch --skip-column-names -e "SELECT invoice_id FROM invoice_items WHERE description='${token}-A' LIMIT 1")"
[[ -n "${invoice_id}" ]]
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM invoice_items WHERE invoice_id=${invoice_id}")" == "2" ]]
[[ "$(client --batch --skip-column-names -e "SELECT CONCAT(net_amount, '|', tax_amount, '|', gross_amount) FROM invoices WHERE id=${invoice_id}")" == "150.00|15.00|165.00" ]]

echo "[5/7] loading existing lines, editing them, and recalculating totals in one transaction"
curl --silent --show-error --fail "${base_url}/invoices/${invoice_id}/edit" -o "${temp_dir}/edit.html"
edit_csrf="$(input_value "${temp_dir}/edit.html" _zelyra_csrf)"
snapshot="$(input_value "${temp_dir}/edit.html" _zelyra_snapshot)"
[[ -n "${edit_csrf}" && -n "${snapshot}" ]]
grep -Fq "value=\"${token}-A\"" "${temp_dir}/edit.html"
edit_status="$(curl --silent --show-error --output "${temp_dir}/edited.html" --write-out '%{http_code}' \
    --header "Origin: ${base_url}" --data-urlencode "_zelyra_csrf=${edit_csrf}" \
    --data-urlencode "_zelyra_snapshot=${snapshot}" --data-urlencode '_zelyra_child_count=2' \
    --data-urlencode 'invoice_date=2026-10-02' --data-urlencode 'items.0.position=1' \
    --data-urlencode "items.0.description=${token}-A" --data-urlencode 'items.0.total_net=120.00' \
    --data-urlencode 'items.0.total_gross=132.00' --data-urlencode 'items.1.position=2' \
    --data-urlencode "items.1.description=${token}-B" --data-urlencode 'items.1.total_net=60.00' \
    --data-urlencode 'items.1.total_gross=66.00' "${base_url}/invoices/${invoice_id}/edit")"
[[ "${edit_status}" == "303" ]]
[[ "$(client --batch --skip-column-names -e "SELECT CONCAT(net_amount, '|', tax_amount, '|', gross_amount) FROM invoices WHERE id=${invoice_id}")" == "180.00|18.00|198.00" ]]
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM invoice_items WHERE invoice_id=${invoice_id}")" == "2" ]]

echo "[6/7] verifying delete cascade is defined by the schema"
client -e "DELETE FROM invoices WHERE id=${invoice_id}"
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM invoice_items WHERE invoice_id=${invoice_id}")" == "0" ]]
invoice_id=""

echo "[7/7] all master-detail CRUD checks passed"
