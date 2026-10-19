#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
project_file="${repo_dir}/examples/multiple_children/main.zyl"
database_url="${DATABASE_URL:-}"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-multiple-children.XXXXXX")"
token="$(date +%s)-$$"
server_pid=""

cleanup() {
    local status=$?
    if [[ -n "${server_pid}" ]]; then
        kill "${server_pid}" 2>/dev/null || true
        wait "${server_pid}" 2>/dev/null || true
    fi
    if [[ "${status}" -ne 0 && -f "${temp_dir}/server.log" ]]; then cat "${temp_dir}/server.log" >&2; fi
    if [[ -n "${db_name:-}" ]]; then
        client -e "DELETE FROM invoices WHERE reference LIKE '${token}-%'" >/dev/null 2>&1 || true
    fi
    for file in "${temp_dir}"/*; do [[ -e "${file}" ]] && unlink "${file}" 2>/dev/null || true; done
    rmdir "${temp_dir}" 2>/dev/null || true
    return "${status}"
}
trap cleanup EXIT

if [[ "${database_url}" != mariadb://* && "${database_url}" != mysql://* ]]; then
    echo "error: set DATABASE_URL to a disposable MariaDB test database" >&2
    exit 1
fi
[[ -x "${zelyra_bin}" ]] || { echo "error: build the CLI first; expected ${zelyra_bin}" >&2; exit 1; }
for command in mariadb curl python3; do command -v "${command}" >/dev/null || { echo "error: ${command} is required" >&2; exit 1; }; done

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
[[ "${db_name,,}" == *test* ]] || { echo "error: refusing to run against a database whose name does not contain 'test'" >&2; exit 1; }
client() {
    MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" "${db_name}" "$@"
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

echo "[1/6] checking the two-collection project and preparing its schema"
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
grep -Fq 'name="attachments.0.file_name"' "${temp_dir}/create.html"
grep -Fq 'name="_items_zelyra_child_count"' "${temp_dir}/create.html"
grep -Fq 'name="_attachments_zelyra_child_count"' "${temp_dir}/create.html"
[[ "$(grep -o '<form ' "${temp_dir}/create.html" | wc -l)" == 1 ]]
csrf="$(input_value "${temp_dir}/create.html" _zelyra_csrf)"
[[ -n "${csrf}" ]]

echo "[2/6] rejecting invalid attachment data before any parent or child insert"
before="$(client --batch --skip-column-names -e "SELECT CONCAT((SELECT COUNT(*) FROM invoices WHERE reference LIKE '${token}-%'),'|',(SELECT COUNT(*) FROM invoice_items),'|',(SELECT COUNT(*) FROM invoice_attachments))")"
invalid_status="$(curl --silent --show-error --output "${temp_dir}/invalid.html" --write-out '%{http_code}' \
    --header "Origin: ${base_url}" --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "reference=${token}-invalid" --data-urlencode 'invoice_date=2026-10-02' \
    --data-urlencode '_items_zelyra_child_count=1' --data-urlencode 'items.0.position=1' \
    --data-urlencode "items.0.description=${token}-bad" --data-urlencode 'items.0.total_net=10.00' \
    --data-urlencode 'items.0.total_gross=11.00' --data-urlencode '_attachments_zelyra_child_count=1' \
    --data-urlencode 'attachments.0.file_name=bad.pdf' --data-urlencode 'attachments.0.size_bytes=-1' \
    "${base_url}/invoices/new")"
[[ "${invalid_status}" == 422 ]]
[[ "$(client --batch --skip-column-names -e "SELECT CONCAT((SELECT COUNT(*) FROM invoices WHERE reference LIKE '${token}-%'),'|',(SELECT COUNT(*) FROM invoice_items),'|',(SELECT COUNT(*) FROM invoice_attachments))")" == "${before}" ]]

echo "[3/6] creating an invoice with both collections in one transaction"
save_status="$(curl --silent --show-error --output "${temp_dir}/saved.html" --write-out '%{http_code}' \
    --header "Origin: ${base_url}" --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "reference=${token}-main" --data-urlencode 'invoice_date=2026-10-03' \
    --data-urlencode '_items_zelyra_child_count=2' --data-urlencode 'items.0.position=1' \
    --data-urlencode "items.0.description=${token}-line-a" --data-urlencode 'items.0.total_net=100.00' \
    --data-urlencode 'items.0.total_gross=110.00' --data-urlencode 'items.1.position=2' \
    --data-urlencode "items.1.description=${token}-line-b" --data-urlencode 'items.1.total_net=50.00' \
    --data-urlencode 'items.1.total_gross=55.00' --data-urlencode '_attachments_zelyra_child_count=1' \
    --data-urlencode "attachments.0.file_name=${token}-main.pdf" --data-urlencode 'attachments.0.size_bytes=2048' \
    "${base_url}/invoices/new")"
[[ "${save_status}" == 303 ]]
invoice_id="$(client --batch --skip-column-names -e "SELECT id FROM invoices WHERE reference='${token}-main'")"
[[ -n "${invoice_id}" ]]
[[ "$(client --batch --skip-column-names -e "SELECT CONCAT((SELECT COUNT(*) FROM invoice_items WHERE invoice_id=${invoice_id}),'|',(SELECT COUNT(*) FROM invoice_attachments WHERE invoice_id=${invoice_id}),'|',(SELECT net_amount FROM invoices WHERE id=${invoice_id}),'|',(SELECT gross_amount FROM invoices WHERE id=${invoice_id}))")" == "2|1|150.00|165.00" ]]

echo "[4/6] creating a second parent's rows to prove edit replacement is parent-scoped"
client --batch --skip-column-names -e "INSERT INTO invoices (reference, invoice_date) VALUES ('${token}-other', '2026-10-04'); SET @other_id=LAST_INSERT_ID(); INSERT INTO invoice_items (invoice_id, position, description, total_net, total_gross) VALUES (@other_id, 1, '${token}-other-line', 7.00, 8.00); INSERT INTO invoice_attachments (invoice_id, file_name, size_bytes) VALUES (@other_id, '${token}-other.pdf', 512);" >/dev/null

echo "[5/6] replacing both collections on edit while preserving the other parent's children"
curl --silent --show-error --fail "${base_url}/invoices/${invoice_id}/edit" -o "${temp_dir}/edit.html"
edit_csrf="$(input_value "${temp_dir}/edit.html" _zelyra_csrf)"
snapshot="$(input_value "${temp_dir}/edit.html" _zelyra_snapshot)"
grep -Fq "value=\"${token}-main.pdf\"" "${temp_dir}/edit.html"
edit_status="$(curl --silent --show-error --output "${temp_dir}/edited.html" --write-out '%{http_code}' \
    --header "Origin: ${base_url}" --data-urlencode "_zelyra_csrf=${edit_csrf}" \
    --data-urlencode "_zelyra_snapshot=${snapshot}" --data-urlencode "reference=${token}-main" \
    --data-urlencode 'invoice_date=2026-10-03' --data-urlencode '_items_zelyra_child_count=1' \
    --data-urlencode 'items.0.position=1' --data-urlencode "items.0.description=${token}-updated" \
    --data-urlencode 'items.0.total_net=120.00' --data-urlencode 'items.0.total_gross=132.00' \
    --data-urlencode '_attachments_zelyra_child_count=1' --data-urlencode "attachments.0.file_name=${token}-updated.pdf" \
    --data-urlencode 'attachments.0.size_bytes=4096' "${base_url}/invoices/${invoice_id}/edit")"
[[ "${edit_status}" == 303 ]]
[[ "$(client --batch --skip-column-names -e "SELECT CONCAT((SELECT COUNT(*) FROM invoice_items WHERE invoice_id=${invoice_id}),'|',(SELECT COUNT(*) FROM invoice_attachments WHERE invoice_id=${invoice_id}),'|',(SELECT net_amount FROM invoices WHERE id=${invoice_id}),'|',(SELECT gross_amount FROM invoices WHERE id=${invoice_id}))")" == "1|1|120.00|132.00" ]]
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM invoice_items WHERE description='${token}-other-line'")" == 1 ]]
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM invoice_attachments WHERE file_name='${token}-other.pdf'")" == 1 ]]

echo "[6/6] multiple child collections are validated, saved, and replaced atomically"
