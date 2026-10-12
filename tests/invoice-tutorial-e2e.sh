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
cleanup() {
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

for route in / /customers /items /invoices /invoice_lines /views/invoicedashboard; do
  response="$(curl --silent --show-error --retry 40 --retry-connrefused \
    --retry-delay 0 --write-out '\n%{http_code}' "http://${address}${route}")"
  status="${response##*$'\n'}"
  body="${response%$'\n'*}"
  if [[ "${status}" != 200 ]]; then
    echo "invoice tutorial route ${route} returned HTTP ${status}" >&2
    cat "${temporary_root}/server.log" >&2
    exit 1
  fi
  if [[ "${route}" == "/" && "${body}" != *"Rechnungsübersicht"* ]]; then
    echo "invoice tutorial dashboard did not render" >&2
    exit 1
  fi
done
home="$(curl --silent --show-error --fail "http://${address}/")"
if [[ "${home}" != *'href="/__zelyra/theme.css"'* ]]; then
  echo "invoice tutorial did not link its project theme stylesheet" >&2
  exit 1
fi
theme="$(curl --silent --show-error --fail "http://${address}/__zelyra/theme.css")"
if [[ "${theme}" != *"--tutorial-shadow"* ]]; then
  echo "invoice tutorial theme stylesheet did not contain its project CSS" >&2
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
echo "invoice tutorial schema, routes, theme, and Studio-compatible customer create/list flow passed"
