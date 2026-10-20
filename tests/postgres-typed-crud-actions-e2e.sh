#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
base_url="${DATABASE_URL:-}"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-pg-actions.XXXXXX")"
server_pid=""
db_created=false
suffix="$$"
database="zelyra_typed_actions_${suffix}"
database_url=""

cleanup() {
    local status=$?
    if [[ -n "${server_pid}" ]]; then kill "${server_pid}" 2>/dev/null || true; wait "${server_pid}" 2>/dev/null || true; fi
    if [[ "${db_created}" == true ]]; then
        psql -X -v ON_ERROR_STOP=1 --dbname="${base_url}" --command="DROP DATABASE IF EXISTS \"${database}\";" >/dev/null 2>&1 || true
    fi
    if [[ "${status}" -ne 0 && -f "${temp_dir}/server.log" ]]; then cat "${temp_dir}/server.log" >&2; fi
    rm -rf -- "${temp_dir}"
    return "${status}"
}
trap cleanup EXIT

if [[ ! -x "${zelyra_bin}" ]]; then echo "error: build CLI first (${zelyra_bin})" >&2; exit 1; fi
command -v psql >/dev/null
command -v curl >/dev/null
command -v python3 >/dev/null
mapfile -t connection < <(python3 - "${base_url}" <<'PY'
import sys
from urllib.parse import unquote, urlsplit
u = urlsplit(sys.argv[1])
if u.scheme not in ("postgres", "postgresql") or u.hostname not in ("127.0.0.1", "localhost", "::1"):
    raise SystemExit("error: DATABASE_URL must point to a local PostgreSQL test service")
if u.path.lstrip("/") not in ("zelyra_ci", "zelyra_test") or u.query or u.fragment:
    raise SystemExit("error: base database must be zelyra_ci or zelyra_test without URL parameters")
print(u.scheme)
print(u.hostname)
print(u.port or 5432)
print(unquote(u.username or ""))
print(unquote(u.password or ""))
print(u.path.lstrip("/"))
PY
)
[[ "${#connection[@]}" == 6 ]]
scheme="${connection[0]}"; host="${connection[1]}"; port="${connection[2]}"; user="${connection[3]}"; password="${connection[4]}"; base_database="${connection[5]}"
base_url="${scheme}://${user}:$(python3 -c 'import sys; from urllib.parse import quote; print(quote(sys.argv[1], safe=""))' "${password}")@${host}:${port}/${base_database}"
database_url="${scheme}://${user}:$(python3 -c 'import sys; from urllib.parse import quote; print(quote(sys.argv[1], safe=""))' "${password}")@${host}:${port}/${database}"
export PGPASSWORD="${password}"
psql -X -v ON_ERROR_STOP=1 --dbname="${base_url}" --command="CREATE DATABASE \"${database}\";" >/dev/null
db_created=true

sed 's/engine: mariadb/engine: postgres/' "${repo_dir}/examples/typed_crud_actions/main.zyl" >"${temp_dir}/main.zyl"
DATABASE_URL="${database_url}" "${zelyra_bin}" check "${temp_dir}/main.zyl"
DATABASE_URL="${database_url}" "${zelyra_bin}" db apply "${temp_dir}/main.zyl" >/dev/null
psql -X -v ON_ERROR_STOP=1 --dbname="${database_url}" --command="INSERT INTO invoices (invoice_number, is_locked) VALUES ('pg-${suffix}', false), ('pg-${suffix}-rollback', false);" >/dev/null
id="$(psql -X -At --dbname="${database_url}" --command="SELECT id FROM invoices WHERE invoice_number='pg-${suffix}';")"
rollback_id="$(psql -X -At --dbname="${database_url}" --command="SELECT id FROM invoices WHERE invoice_number='pg-${suffix}-rollback';")"
app_port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()')"
base="http://127.0.0.1:${app_port}"
DATABASE_URL="${database_url}" "${zelyra_bin}" serve "${temp_dir}/main.zyl" "127.0.0.1:${app_port}" >"${temp_dir}/server.log" 2>&1 & server_pid=$!

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
load_detail() {
    for _ in $(seq 1 30); do if curl --silent --fail "${base}/invoices/$1" -o "$2"; then return; fi; sleep 1; done
    curl --silent --show-error --fail "${base}/invoices/$1" -o "$2"
}
post_action() { curl --silent --show-error --output "$4" --write-out '%{http_code}' --header "Origin: ${base}" --data-urlencode "_zelyra_csrf=$3" "${base}/invoices/$1/$2"; }

load_detail "${id}" "${temp_dir}/lock.html"
token="$(csrf_from "${temp_dir}/lock.html")"; [[ -n "${token}" ]]
status="$(post_action "${id}" lock "${token}" "${temp_dir}/lock-result.html")"; [[ "${status}" == 303 ]]
hash="$(printf '%s' "pg-${suffix}" | sha256sum | cut -d' ' -f1)"
[[ "$(psql -X -At --dbname="${database_url}" --command="SELECT is_locked::int || '|' || (integrity_hash='${hash}')::int || '|' || (locked_at IS NOT NULL)::int FROM invoices WHERE id=${id};")" == "1|1|1" ]]
[[ "$(psql -X -At --dbname="${database_url}" --command="SELECT COUNT(*) FROM invoice_audit_logs WHERE invoice_id=${id} AND action='locked';")" == 1 ]]

load_detail "${rollback_id}" "${temp_dir}/fail.html"
token="$(csrf_from "${temp_dir}/fail.html")"; [[ -n "${token}" ]]
status="$(post_action "${rollback_id}" fail_after_writes "${token}" "${temp_dir}/fail-result.html")"; [[ "${status}" == 500 ]]
[[ "$(psql -X -At --dbname="${database_url}" --command="SELECT is_locked::int || '|' || (SELECT COUNT(*) FROM invoice_audit_logs WHERE invoice_id=${rollback_id}) FROM invoices WHERE id=${rollback_id};")" == "0|0" ]]

echo "Typed CRUD action PostgreSQL E2E passed."
