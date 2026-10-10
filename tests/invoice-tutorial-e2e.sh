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
"${binary}" --tutorial invoice "${project}"
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
echo "invoice tutorial schema and all customer, item, invoice, and dashboard routes passed"
