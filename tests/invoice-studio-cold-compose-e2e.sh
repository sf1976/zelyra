#!/usr/bin/env bash
set -euo pipefail

binary="${ZELYRA_BIN:-target/debug/zelyra}"
if [[ ! -x "${binary}" ]]; then
  echo "ZELYRA_BIN must name the built Zelyra CLI" >&2
  exit 2
fi
if ! docker compose version >/dev/null 2>&1; then
  echo "Docker Compose v2 is required for the cold-start Studio regression test" >&2
  exit 2
fi

temporary_root="$(mktemp -d)"
project="${temporary_root}/invoice-cold-$$"
compose_project="invoice-cold-$$"
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
  if [[ -f "${project}/docker-compose.mariadb.yml" ]]; then
    docker compose -p "${compose_project}" -f "${project}/docker-compose.mariadb.yml" \
      down --volumes --remove-orphans >/dev/null 2>&1 || true
  fi
  rm -rf "${temporary_root}"
}
trap cleanup EXIT

"${binary}" --tutorial invoice "${project}" --no-start >/dev/null
if ! grep -Fq '${ZELYRA_DB_HOST_PORT:-3306}' "${project}/.env"; then
  echo "generated invoice project must retain its configurable database-port placeholder" >&2
  exit 1
fi

studio_port="$(python3 - <<'PY'
import socket
with socket.socket() as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
)"
env -u DATABASE_URL -u ZELYRA_DATABASE_MAIN_URL -u ZELYRA_DB_HOST_PORT \
  "${binary}" editor "${project}" --port "${studio_port}" --no-open \
  >"${temporary_root}/studio.log" 2>&1 &
editor_pid=$!
for _ in $(seq 1 100); do
  if grep -q '/__zelyra/editor/#' "${temporary_root}/studio.log"; then break; fi
  if ! kill -0 "${editor_pid}" 2>/dev/null; then
    sed -E 's/(#__zelyra\/editor\/#)[0-9a-f]+/\1[redacted]/g' "${temporary_root}/studio.log" >&2
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
ZELYRA_INVOICE_STUDIO_E2E_TIMEOUT_MS=180000 \
  node editor/tests/invoice-studio-browser-e2e.mjs
echo "fresh Docker Compose invoice startup, schema editor, and Studio Run passed"
