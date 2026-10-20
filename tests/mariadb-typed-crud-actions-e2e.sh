#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
project_file="${repo_dir}/examples/typed_crud_actions/main.zyl"
database_url="${DATABASE_URL:-}"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-typed-actions-e2e.XXXXXX")"
server_pid=""
invoice="ta-$(date +%s)-$$"

cleanup() {
    local status=$?
    if [[ -n "${server_pid}" ]]; then kill "${server_pid}" 2>/dev/null || true; wait "${server_pid}" 2>/dev/null || true; fi
    if declare -F client >/dev/null && [[ -n "${db_name:-}" ]]; then
        client -e "DELETE FROM _zelyra_outbox WHERE JSON_UNQUOTE(JSON_EXTRACT(payload, '$.attachments[0].filename')) = CONCAT('invoice-', '${invoice}-rollback', '.pdf');" >/dev/null 2>&1 || true
        client -e "DELETE FROM invoice_audit_logs WHERE invoice_id IN (SELECT id FROM invoices WHERE invoice_number LIKE '${invoice}%'); DELETE FROM invoices WHERE invoice_number LIKE '${invoice}%';" >/dev/null 2>&1 || true
    fi
    if [[ "${status}" -ne 0 && -f "${temp_dir}/server.log" ]]; then cat "${temp_dir}/server.log" >&2; fi
    rm -rf -- "${temp_dir}"
    return "${status}"
}
trap cleanup EXIT

if [[ "${database_url}" != mariadb://* && "${database_url}" != mysql://* ]]; then
    echo "error: DATABASE_URL must point to an isolated MariaDB test database" >&2; exit 1
fi
if [[ ! -x "${zelyra_bin}" ]]; then echo "error: build CLI first (${zelyra_bin})" >&2; exit 1; fi
for command in mariadb curl python3 sha256sum; do command -v "${command}" >/dev/null || { echo "error: ${command} required" >&2; exit 1; }; done

mapfile -t parts < <(python3 - "${database_url}" <<'PY'
import sys
from urllib.parse import unquote, urlsplit
u = urlsplit(sys.argv[1])
if u.scheme not in ("mariadb", "mysql") or u.hostname not in ("127.0.0.1", "localhost"):
    raise SystemExit("error: E2E database must be local MariaDB/MySQL")
name = u.path.lstrip("/")
if "test" not in name.lower() or not name.replace("_", "").isalnum():
    raise SystemExit("error: refusing database not explicitly named as a test database")
print(u.hostname, u.port or 3306, unquote(u.username or ""), unquote(u.password or ""), name, sep="\n")
PY
)
[[ "${#parts[@]}" == 5 ]] || exit 1
db_host="${parts[0]}"; db_port="${parts[1]}"; db_user="${parts[2]}"; db_password="${parts[3]}"; db_name="${parts[4]}"
client() { MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" --database="${db_name}" "$@"; }
csrf_from() {
    python3 - "$1" <<'PY'
import sys
from html.parser import HTMLParser
class Inputs(HTMLParser):
    token = ""
    def handle_starttag(self, tag, attrs):
        fields = dict(attrs)
        if tag == "input" and fields.get("name") == "_zelyra_csrf": self.token = fields.get("value", "")
p = Inputs(); p.feed(open(sys.argv[1], encoding="utf-8").read()); print(p.token)
PY
}

"${zelyra_bin}" check "${project_file}"
DATABASE_URL="${database_url}" "${zelyra_bin}" db setup "${project_file}"
DATABASE_URL="${database_url}" "${zelyra_bin}" outbox setup "${project_file}"
client --batch -e "INSERT INTO invoices (invoice_number, is_locked) VALUES ('${invoice}', false), ('${invoice}-rollback', false)"
id="$(client --batch --skip-column-names -e "SELECT id FROM invoices WHERE invoice_number='${invoice}'")"
rollback_id="$(client --batch --skip-column-names -e "SELECT id FROM invoices WHERE invoice_number='${invoice}-rollback'")"
port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()')"
base="http://127.0.0.1:${port}"
DATABASE_URL="${database_url}" "${zelyra_bin}" serve "${project_file}" "127.0.0.1:${port}" >"${temp_dir}/server.log" 2>&1 & server_pid=$!
load_form() { for _ in $(seq 1 30); do if curl --silent --fail "${base}/invoices/$1" -o "$3"; then return; fi; sleep 1; done; curl --silent --show-error --fail "${base}/invoices/$1" -o "$3"; }
post_action() { curl --silent --show-error --output "$4" --write-out '%{http_code}' --header "Origin: ${base}" --data-urlencode "_zelyra_csrf=$3" "${base}/invoices/$1/$2"; }

load_form "${id}" lock "${temp_dir}/lock.html"
token="$(csrf_from "${temp_dir}/lock.html")"; [[ -n "${token}" ]]
status="$(post_action "${id}" lock "${token}" "${temp_dir}/lock-result.html")"; [[ "${status}" == 303 ]]
hash="$(printf '%s' "${invoice}" | sha256sum | cut -d' ' -f1)"
[[ "$(client --batch --skip-column-names -e "SELECT CONCAT(is_locked, '|', integrity_hash='${hash}', '|', locked_at IS NOT NULL) FROM invoices WHERE id=${id}")" == "1|1|1" ]]
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM invoice_audit_logs WHERE invoice_id=${id} AND action='locked'")" == 1 ]]

load_form "${id}" lock "${temp_dir}/guard.html"; token="$(csrf_from "${temp_dir}/guard.html")"
status="$(post_action "${id}" lock "${token}" "${temp_dir}/guard-result.html")"; [[ "${status}" == 409 ]]
grep -Fq "Invoice is already locked." "${temp_dir}/guard-result.html"

load_form "${rollback_id}" fail_after_writes "${temp_dir}/fail.html"; token="$(csrf_from "${temp_dir}/fail.html")"
status="$(post_action "${rollback_id}" fail_after_writes "${token}" "${temp_dir}/fail-result.html")"; [[ "${status}" == 500 ]]
[[ "$(client --batch --skip-column-names -e "SELECT CONCAT(is_locked, '|', (SELECT COUNT(*) FROM invoice_audit_logs WHERE invoice_id=${rollback_id})) FROM invoices WHERE id=${rollback_id}")" == "0|0" ]]

load_form "${rollback_id}" email_pdf "${temp_dir}/email.html"; token="$(csrf_from "${temp_dir}/email.html")"
status="$(post_action "${rollback_id}" email_pdf "${token}" "${temp_dir}/email-result.html")"; [[ "${status}" == 303 ]]
[[ "$(client --batch --skip-column-names -e "SELECT CONCAT(COUNT(*), '|', MIN(delivered_at IS NULL), '|', MIN(JSON_UNQUOTE(JSON_EXTRACT(payload, '$.attachments[0].filename')) = CONCAT('invoice-', '${invoice}-rollback', '.pdf')), '|', MIN(JSON_UNQUOTE(JSON_EXTRACT(payload, '$.attachments[0].content')) LIKE 'JVBER%')) FROM _zelyra_outbox WHERE event_type='zelyra.mail.send' AND JSON_UNQUOTE(JSON_EXTRACT(payload, '$.to'))='customer@example.test'")" == "1|1|1|1" ]]
echo "Typed CRUD action MariaDB E2E passed."
