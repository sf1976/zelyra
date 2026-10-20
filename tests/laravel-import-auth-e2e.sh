#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
source_project="${1:-/opt/siedelmann-laravel/app}"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
database_host="${ZELYRA_LARAVEL_IMPORT_DB_HOST:-127.0.0.1}"
database_port="${ZELYRA_LARAVEL_IMPORT_DB_PORT:-3306}"
database_user="${ZELYRA_LARAVEL_IMPORT_DB_USER:-root}"
database_password="${ZELYRA_LARAVEL_IMPORT_DB_PASSWORD:-}"
address="${ZELYRA_LARAVEL_IMPORT_AUTH_ADDRESS:-127.0.0.1:38536}"
base_url="http://${address}"
suffix="$(date +%s%N)"
database_name="zelyra_import_auth_${$}"
test_email="import-auth-${suffix}@example.test"
test_password="ZelyraImport-${suffix}-Password"
temporary="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-import-auth-e2e.XXXXXX")"
project="${temporary}/generated"
server_pid=""

if [[ ! -x "${zelyra_bin}" ]]; then
    echo "error: Zelyra binary not found: ${zelyra_bin}; run cargo build -p zelyra-cli first" >&2
    exit 1
fi
for command in mariadb curl python3; do
    if ! command -v "${command}" >/dev/null 2>&1; then
        echo "error: ${command} is required for Laravel importer auth E2E" >&2
        exit 1
    fi
done
if [[ ! -d "${source_project}" ]]; then
    echo "error: Laravel source project does not exist: ${source_project}" >&2
    exit 2
fi

client() {
    MYSQL_PWD="${database_password}" mariadb \
        --protocol=tcp --host="${database_host}" --port="${database_port}" \
        --user="${database_user}" "$@"
}

cleanup() {
    if [[ -n "${server_pid}" ]]; then
        kill "${server_pid}" 2>/dev/null || true
        wait "${server_pid}" 2>/dev/null || true
    fi
    if [[ -n "${database_password}" ]]; then
        client --batch --skip-column-names \
            --execute="DROP DATABASE IF EXISTS \`${database_name}\`;" >/dev/null 2>&1 || true
    fi
    rm -rf -- "${temporary}"
}
trap cleanup EXIT

echo "[1/6] generating compiler-checked native modules from Laravel source"
env -u DATABASE_URL -u ZELYRA_DATABASE_MAIN_URL \
    "${zelyra_bin}" import laravel generate "${source_project}" \
    --output "${project}" --all-supported
cat >>"${project}/main.zyl" <<'ZYL'

page "/" {
    html {
        <main><h1>Imported identity consumer</h1></main>
    }
}
ZYL
"${zelyra_bin}" package lock "${project}/main.zyl" --update
"${zelyra_bin}" check "${project}/main.zyl"

if [[ -e "${project}/.env" || -e "${project}/storage" ]]; then
    echo "error: generated scaffold unexpectedly contains Laravel environment or runtime data" >&2
    exit 1
fi
grep -F 'login_enabled: is_approved' "${project}/packages/imported_identity/src/lib.zyl" >/dev/null
grep -F '"approval_guard_transferred": true' "${project}/GENERATION_SELECTION.json" >/dev/null

echo "[2/6] creating a separate empty destination database"
client --batch --skip-column-names \
    --execute="CREATE DATABASE \`${database_name}\`;"
database_url="mariadb://${database_user}:${database_password}@${database_host}:${database_port}/${database_name}"
export DATABASE_URL="${database_url}"
"${zelyra_bin}" db setup "${project}/main.zyl"

echo "[3/6] creating a new test account in the destination only"
password_hash="$(printf '%s\n' "${test_password}" | "${zelyra_bin}" auth hash-password --stdin)"
MYSQL_PWD="${database_password}" mariadb \
    --protocol=tcp --host="${database_host}" --port="${database_port}" \
    --user="${database_user}" "${database_name}" \
    --execute="INSERT INTO users (name, email, password_hash, is_approved) VALUES ('Importer E2E', '${test_email}', '${password_hash}', false);"

echo "[4/6] starting the generated authentication routes"
"${zelyra_bin}" serve "${project}/main.zyl" "${address}" \
    >"${temporary}/server.log" 2>&1 &
server_pid=$!
for _ in $(seq 1 60); do
    status="$(curl --silent --output "${temporary}/login.html" --write-out '%{http_code}' "${base_url}/login" || true)"
    [[ "${status}" == "200" ]] && break
    if ! kill -0 "${server_pid}" 2>/dev/null; then
        cat "${temporary}/server.log" >&2
        echo "error: generated Zelyra server exited before login was served" >&2
        exit 1
    fi
    sleep 0.1
done
if [[ "${status:-}" != "200" ]]; then
    cat "${temporary}/server.log" >&2
    echo "error: generated login route did not become available" >&2
    exit 1
fi
csrf="$(python3 - "${temporary}/login.html" <<'PY'
import html
import re
import sys
from pathlib import Path

page = Path(sys.argv[1]).read_text(encoding="utf-8")
match = re.search(r'name=["\']_zelyra_csrf["\']\s+value=["\']([^"\']+)', page)
if not match:
    raise SystemExit("generated login form has no CSRF token")
print(html.unescape(match.group(1)))
PY
)"

echo "[5/6] authenticating and verifying a persisted native session"
unapproved_status="$(curl --silent --output "${temporary}/unapproved.html" --write-out '%{http_code}' \
    --header "Origin: ${base_url}" \
    --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "email=${test_email}" \
    --data-urlencode "password=${test_password}" \
    "${base_url}/login")"
[[ "${unapproved_status}" == "401" ]]
MYSQL_PWD="${database_password}" mariadb \
    --protocol=tcp --host="${database_host}" --port="${database_port}" \
    --user="${database_user}" "${database_name}" \
    --execute="UPDATE users SET is_approved = true WHERE email = '${test_email}';"
invalid_status="$(curl --silent --output "${temporary}/invalid.html" --write-out '%{http_code}' \
    --header "Origin: ${base_url}" \
    --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "email=${test_email}" \
    --data-urlencode "password=wrong-${test_password}" \
    "${base_url}/login")"
[[ "${invalid_status}" == "401" ]]

cookie_jar="${temporary}/cookies.txt"
login_status="$(curl --silent --output "${temporary}/login-response.html" --write-out '%{http_code}' \
    --cookie-jar "${cookie_jar}" \
    --header "Origin: ${base_url}" \
    --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "email=${test_email}" \
    --data-urlencode "password=${test_password}" \
    "${base_url}/login")"
[[ "${login_status}" == "303" ]]
grep -q 'zelyra_session' "${cookie_jar}"
user_id="$(MYSQL_PWD="${database_password}" mariadb \
    --protocol=tcp --host="${database_host}" --port="${database_port}" \
    --user="${database_user}" --batch --skip-column-names "${database_name}" \
    --execute="SELECT id FROM users WHERE email = '${test_email}'")"
session_count="$(MYSQL_PWD="${database_password}" mariadb \
    --protocol=tcp --host="${database_host}" --port="${database_port}" \
    --user="${database_user}" --batch --skip-column-names "${database_name}" \
    --execute="SELECT COUNT(*) FROM zelyra_auth_sessions WHERE user_id = ${user_id}")"
[[ "${session_count}" == "1" ]]

echo "[6/6] logging out and verifying session revocation"
logout_status="$(curl --silent --output "${temporary}/logout.html" --write-out '%{http_code}' \
    --cookie "${cookie_jar}" --cookie-jar "${cookie_jar}" \
    --header "Origin: ${base_url}" --request POST \
    --data-urlencode "_zelyra_csrf=${csrf}" "${base_url}/logout")"
[[ "${logout_status}" == "303" ]]
session_count_after_logout="$(MYSQL_PWD="${database_password}" mariadb \
    --protocol=tcp --host="${database_host}" --port="${database_port}" \
    --user="${database_user}" --batch --skip-column-names "${database_name}" \
    --execute="SELECT COUNT(*) FROM zelyra_auth_sessions WHERE user_id = ${user_id}")"
[[ "${session_count_after_logout}" == "0" ]]
echo "Laravel-imported native authentication MariaDB/HTTP E2E passed."
echo "The Laravel source was not modified; no source credentials or user data were imported."
