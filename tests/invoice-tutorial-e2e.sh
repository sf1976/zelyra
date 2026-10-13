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

for route in / /customers /items /invoices /invoice_lines /invoices/new /views/invoicedashboard; do
  response="$(curl --silent --show-error --retry 40 --retry-connrefused \
    --retry-delay 0 --write-out '\n%{http_code}' "http://${address}${route}")"
  status="${response##*$'\n'}"
  body="${response%$'\n'*}"
  if [[ "${status}" != 200 ]]; then
    echo "invoice tutorial route ${route} returned HTTP ${status}" >&2
    cat "${temporary_root}/server.log" >&2
    exit 1
  fi
  if [[ "${route}" == "/" && ("${body}" != *"Guten Tag im Rechnungsbüro"* || "${body}" != *"Erfasster Nettowert"*) ]]; then
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
if [[ "${theme}" != *"Tabler v1.6.1"* || "${theme}" != *".tutorial-metric"* || "${theme}" != *"max-width: 991px"* ]]; then
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
if [[ "${ZELYRA_INVOICE_BROWSER_E2E:-0}" == "1" ]]; then
  ZELYRA_INVOICE_E2E_URL="http://${address}" node editor/tests/invoice-tutorial-browser-e2e.mjs

  # Exercise the generated .env placeholder exactly as Studio receives it. The
  # integration database is already running outside this temporary project.
  DATABASE_URL="${database_url}" python3 - "${project}/.env" <<'PY'
import os
import pathlib
import urllib.parse
import sys

path = pathlib.Path(sys.argv[1])
source = path.read_text(encoding="utf-8").splitlines()
parsed = urllib.parse.urlsplit(os.environ["DATABASE_URL"])
host = parsed.hostname or "127.0.0.1"
if ":" in host and not host.startswith("["):
    host = f"[{host}]"
credentials = urllib.parse.quote(parsed.username or "", safe="")
if parsed.password is not None:
    credentials += ":" + urllib.parse.quote(parsed.password, safe="")
credentials += "@"
port = parsed.port or 3306
compose_port = port + 1 if port < 65535 else port - 1
env_url = urllib.parse.urlunsplit((
    parsed.scheme,
    f"{credentials}{host}:{port}",
    parsed.path,
    parsed.query,
    parsed.fragment,
))
updates = {
    "ZELYRA_DB_HOST_PORT": str(compose_port),
    "ZELYRA_DATABASE_MAIN_URL": env_url,
    "DATABASE_URL": env_url,
}
output = [line for line in source if line.split("=", 1)[0].strip() not in updates]
output.extend(f"{key}={value}" for key, value in updates.items())
path.write_text("\n".join(output) + "\n", encoding="utf-8")
PY
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
  env -u DATABASE_URL -u ZELYRA_DATABASE_MAIN_URL -u ZELYRA_DB_HOST_PORT \
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
