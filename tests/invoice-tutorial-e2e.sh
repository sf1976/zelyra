#!/usr/bin/env bash
set -euo pipefail

binary="${ZELYRA_BIN:-target/debug/zelyra}"
database_url="${DATABASE_URL:-}"
if [[ ! -x "${binary}" ]]; then
  echo "ZELYRA_BIN must name the built Zelyra CLI" >&2
  exit 2
fi
if [[ -z "${database_url}" || "${database_url}" != mariadb://* ]]; then
  echo "DATABASE_URL must point to a disposable MariaDB database" >&2
  exit 2
fi

temporary_root="$(mktemp -d)"
server_pid=""
editor_pid=""
studio_port=""
studio_token=""
cleanup() {
  if [[ -n "${studio_port}" && -n "${studio_token}" ]]; then
    curl --silent --show-error --max-time 3 -X POST \
      -H "Origin: http://127.0.0.1:${studio_port}" \
      -H "X-Zelyra-Editor-Token: ${studio_token}" \
      -H 'Content-Type: application/json' --data '{}' \
      "http://127.0.0.1:${studio_port}/__zelyra/editor/api/stop-web-preview" >/dev/null 2>&1 || true
  fi
  if [[ -n "${editor_pid}" ]]; then
    kill "${editor_pid}" 2>/dev/null || true
    wait "${editor_pid}" 2>/dev/null || true
  fi
  if [[ -n "${server_pid}" ]]; then
    kill "${server_pid}" 2>/dev/null || true
    wait "${server_pid}" 2>/dev/null || true
  fi
  rm -rf "${temporary_root}"
}
trap cleanup EXIT

project="${temporary_root}/invoice"
"${binary}" --tutorial invoice "${project}" --no-start
DATABASE_URL="${database_url}" ZELYRA_DATABASE_MAIN_URL="${database_url}" \
  "${binary}" db setup "${project}/main.zyl"
# Reapplying the unchanged tutorial schema must not propose destructive
# DECIMAL changes caused by MariaDB's default DECIMAL(10,0) metadata.
DATABASE_URL="${database_url}" ZELYRA_DATABASE_MAIN_URL="${database_url}" \
  "${binary}" db setup "${project}/main.zyl"
# The shared integration database is outside the generated project's Compose
# stack. Load the repeatable sample SQL explicitly so its print route is covered.
python3 - "${database_url}" "${project}/invoice-tutorial.seed.sql" <<'PY'
import os
import shutil
import subprocess
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit

parts = urlsplit(sys.argv[1])
client = shutil.which("mariadb") or shutil.which("mysql")
if client is None:
    raise SystemExit("the invoice E2E requires the MariaDB or MySQL command-line client")
if parts.scheme != "mariadb" or not parts.hostname or not parts.username:
    raise SystemExit("DATABASE_URL must include MariaDB host and user details")
environment = os.environ.copy()
environment["MYSQL_PWD"] = unquote(parts.password or "")
database = unquote(parts.path.lstrip("/"))
subprocess.run(
    [client, "--protocol=tcp", "--host", parts.hostname, "--port", str(parts.port or 3306),
     "--user", unquote(parts.username), "--database", database, "--batch", "--silent"],
    input=Path(sys.argv[2]).read_bytes(),
    env=environment,
    check=True,
)
PY

port="$(python3 - <<'PY'
import socket
with socket.socket() as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
)"
address="127.0.0.1:${port}"
DATABASE_URL="${database_url}" ZELYRA_DATABASE_MAIN_URL="${database_url}" \
  "${binary}" serve "${project}/main.zyl" "${address}" \
  >"${temporary_root}/server.log" 2>&1 &
server_pid=$!

ready=0
for _ in $(seq 1 80); do
  if curl --silent --output /dev/null --fail "http://${address}/"; then
    ready=1
    break
  fi
  if ! kill -0 "${server_pid}" 2>/dev/null; then
    cat "${temporary_root}/server.log" >&2
    exit 1
  fi
  sleep 0.1
done
if [[ "${ready}" != "1" ]]; then
  echo "invoice tutorial web server did not become ready at http://${address}" >&2
  cat "${temporary_root}/server.log" >&2
  exit 1
fi

for route in / /customers /items /invoices /invoice_lines /invoices/new /views/invoicedashboard /invoices/RE-2026-001/print; do
  response="$(curl --silent --show-error --retry 40 --retry-connrefused \
    --retry-delay 0 --write-out '\n%{http_code}' "http://${address}${route}")"
  status="${response##*$'\n'}"
  body="${response%$'\n'*}"
  if [[ "${status}" != 200 ]]; then
    echo "invoice tutorial route ${route} returned HTTP ${status}" >&2
    cat "${temporary_root}/server.log" >&2
    exit 1
  fi
  if [[ "${route}" == "/" && \
    ! ( ("${body}" == *"Vom ersten Kunden bis zur fertigen Rechnung."* && "${body}" == *"Rechnungsübersicht öffnen"* && "${body}" == *"354,62 €"*) || \
       ("${body}" == *"From the first customer to the finished invoice."* && "${body}" == *"Open invoice overview"* && "${body}" == *"€354.62"*) ) ]]; then
    echo "invoice tutorial landing page did not render its workflow" >&2
    python3 -c 'import re,sys; s=sys.stdin.read(); s=s[s.find("invoice-home"):]; print(re.sub(r"<[^>]+>", " ", s)[:1200])' <<<"${body}" >&2
    exit 1
  fi
done
sample_print_response="$(curl --silent --show-error --write-out '\n%{http_code}' "http://${address}/invoices/RE-2026-001/print")"
sample_print_status="${sample_print_response##*$'\n'}"
sample_print="${sample_print_response%$'\n'*}"
if [[ "${sample_print_status}" != "200" ]]; then
  echo "the sample invoice print route returned HTTP ${sample_print_status}" >&2
  cat "${temporary_root}/server.log" >&2
  exit 1
fi
if [[ "${sample_print}" != *"RE-2026-001"* || "${sample_print}" != *"Beispiel GmbH"* || "${sample_print}" != *"354.62"* ]]; then
  echo "the landing-page sample invoice did not open with its seeded details and totals" >&2
  cat "${temporary_root}/server.log" >&2
  exit 1
fi
home="$(curl --silent --show-error --fail "http://${address}/")"
if [[ "${home}" != *'href="/__zelyra/theme.css"'* ]]; then
  echo "invoice tutorial did not link its project theme stylesheet" >&2
  exit 1
fi
theme="$(curl --silent --show-error --fail "http://${address}/__zelyra/theme.css")"
if [[ "${theme}" != *"Tabler v1.6.1"* || "${theme}" != *".invoice-home-hero"* || "${theme}" != *"max-width: 991px"* ]]; then
  echo "invoice tutorial did not serve its local responsive Tabler theme" >&2
  exit 1
fi
if [[ "${theme}" == *"cdn.tabler.io"* || "${theme}" == *"cdn.jsdelivr.net"* ]]; then
  echo "invoice tutorial theme must remain self-hosted" >&2
  exit 1
fi
form_page="${temporary_root}/customer-form.html"
cookie_jar="${temporary_root}/cookies.txt"
curl --silent --show-error --fail --cookie-jar "${cookie_jar}" \
  "http://${address}/customers/new" >"${form_page}"
csrf_token="$(python3 - "${form_page}" <<'PY'
import pathlib, re, sys
page = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
match = re.search(r'name="_zelyra_csrf"\s+value="([^"]+)"', page)
if not match:
    raise SystemExit("customer form did not include a CSRF token")
print(match.group(1))
PY
)"
customer_name="Zelyra invoice E2E ${$}"
curl --silent --show-error --fail --output /dev/null --dump-header "${temporary_root}/customer-post.headers" \
  --cookie "${cookie_jar}" --cookie-jar "${cookie_jar}" \
  --header "Origin: http://${address}" \
  --data-urlencode "_zelyra_csrf=${csrf_token}" \
  --data-urlencode "name=${customer_name}" \
  --data-urlencode "email=zelyra-e2e-${$}@example.invalid" \
  "http://${address}/customers/new"
if ! grep -Eiq '^HTTP/[^ ]+ 303' "${temporary_root}/customer-post.headers"; then
  echo "creating a tutorial customer did not redirect after saving" >&2
  cat "${temporary_root}/server.log" >&2
  exit 1
fi
customer_list="$(curl --silent --show-error --fail "http://${address}/customers")"
if [[ "${customer_list}" != *"${customer_name}"* ]]; then
  echo "the saved tutorial customer was not shown in the customer list" >&2
  cat "${temporary_root}/server.log" >&2
  exit 1
fi
invoice_form="${temporary_root}/invoice-form.html"
invoice_cookie_jar="${temporary_root}/invoice-cookies.txt"
curl --silent --show-error --fail --cookie-jar "${invoice_cookie_jar}" \
  "http://${address}/invoices/new" >"${invoice_form}"
if ! grep -Eq 'name="issued_at"[^>]*type="date"|type="date"[^>]*name="issued_at"' "${invoice_form}"; then
  echo "invoice issued_at field is not a date-only input" >&2
  exit 1
fi
if [[ "$(cat "${invoice_form}")" != *"Format: JJJJ-MM-TT"* && "$(cat "${invoice_form}")" != *"Format: YYYY-MM-DD"* ]]; then
  echo "invoice date field does not explain the accepted format" >&2
  exit 1
fi
invoice_csrf="$(python3 - "${invoice_form}" <<'PY'
import pathlib, re, sys
page = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
match = re.search(r'name="_zelyra_csrf"\s+value="([^"]+)"', page)
if not match:
    raise SystemExit("invoice form did not include a CSRF token")
print(match.group(1))
PY
)"
customer_id="$(python3 - "${invoice_form}" <<'PY'
import pathlib, re, sys
page = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
match = re.search(r'<select[^>]*name="customer"[^>]*>.*?<option value="(\d+)"', page, re.S)
if not match:
    raise SystemExit("invoice form did not include a seeded customer option")
print(match.group(1))
PY
)"
curl --silent --show-error --fail --output /dev/null --dump-header "${temporary_root}/invoice-post.headers" \
  --cookie "${invoice_cookie_jar}" --cookie-jar "${invoice_cookie_jar}" \
  --header "Origin: http://${address}" \
  --data-urlencode "_zelyra_csrf=${invoice_csrf}" \
  --data-urlencode "invoice_number=FORGED-BY-CLIENT" \
  --data-urlencode "customer=${customer_id}" \
  --data-urlencode "issued_at=2026-10-14" \
  --data-urlencode "status=draft" \
  "http://${address}/invoices/new"
if ! grep -Eiq '^HTTP/[^ ]+ 303' "${temporary_root}/invoice-post.headers"; then
  echo "creating an invoice did not redirect after saving" >&2
  cat "${temporary_root}/server.log" >&2
  exit 1
fi
overview="$(curl --silent --show-error --fail "http://${address}/views/invoicedashboard")"
if [[ "${overview}" == *"FORGED-BY-CLIENT"* || "${overview}" != *"RE-00000"* ]]; then
  echo "invoice number was not generated from the database ID" >&2
  cat "${temporary_root}/server.log" >&2
  exit 1
fi
invoice_id="$(python3 -c 'import re,sys; match=re.search(r"RE-(\d{6,})", sys.stdin.read()); print(str(int(match.group(1))) if match else "")' <<<"${overview}")"
if [[ -z "${invoice_id}" ]]; then
  echo "invoice overview did not show the generated invoice number" >&2
  exit 1
fi
invoice_edit="$(curl --silent --show-error --fail "http://${address}/invoices/${invoice_id}/edit")"
if [[ "${invoice_edit}" == *'name="invoice_number"'* ]]; then
  echo "invoice number must be retained and omitted from the edit form" >&2
  exit 1
fi
if [[ "${ZELYRA_INVOICE_BROWSER_E2E:-0}" == "1" ]]; then
  ZELYRA_INVOICE_E2E_URL="http://${address}" node editor/tests/invoice-tutorial-browser-e2e.mjs

  # Keep the generated local settings in .env, then override the active URL
  # with an external database. Run must neither start Compose nor apply the
  # project schema to that ambient connection.
  no_docker_path="${temporary_root}/no-docker-bin"
  mkdir -p "${no_docker_path}"
  for database_client in mariadb mysql; do
    database_client_path="$(command -v "${database_client}" || true)"
    if [[ -n "${database_client_path}" ]]; then
      ln -s "${database_client_path}" "${no_docker_path}/${database_client}"
    fi
  done
  studio_port="$(python3 - <<'PY'
import socket
with socket.socket() as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
)"
  env -u ZELYRA_DB_HOST_PORT \
    DATABASE_URL="${database_url}" \
    ZELYRA_DATABASE_MAIN_URL="${database_url}" \
    PATH="${no_docker_path}" \
    "${binary}" editor "${project}" --port "${studio_port}" --no-open \
    >"${temporary_root}/studio.log" 2>&1 &
  editor_pid=$!
  for _ in $(seq 1 100); do
    if grep -q '/__zelyra/editor/#' "${temporary_root}/studio.log"; then break; fi
    if ! kill -0 "${editor_pid}" 2>/dev/null; then
      cat "${temporary_root}/studio.log" >&2
      exit 1
    fi
    sleep 0.1
  done
  studio_token="$(python3 - "${temporary_root}/studio.log" <<'PY'
import re
import sys
text = open(sys.argv[1], encoding="utf-8").read()
match = re.search(r"/__zelyra/editor/#([0-9a-f]{64})", text)
if not match:
    raise SystemExit("invoice Studio did not print a session URL")
print(match.group(1))
PY
)"
  ZELYRA_INVOICE_STUDIO_E2E_PORT="${studio_port}" \
  ZELYRA_INVOICE_STUDIO_E2E_TOKEN="${studio_token}" \
    node editor/tests/invoice-studio-browser-e2e.mjs
fi
echo "invoice tutorial schema, routes, theme, customer flow, and Studio launch passed"
