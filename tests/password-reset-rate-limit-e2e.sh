#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
project_file="${repo_dir}/examples/password_reset.zyl"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
database_url="${DATABASE_URL:-}"
suffix="$(date +%s)_$$"
rate_table="password_reset_limits_e2e_${suffix}"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-password-reset-limit-e2e.XXXXXX")"
server_pid=""
server_b_pid=""
smtp_pid=""
test_email="reset-rate-e2e-${suffix}@example.test"
test_user_id=""

if [[ "${database_url}" != mariadb://* && "${database_url}" != mysql://* ]]; then
    echo "error: DATABASE_URL must point to a MariaDB test database" >&2
    exit 1
fi
database_parts="${database_url#*://}"
database_credentials="${database_parts%@*}"
database_location="${database_parts#*@}"
db_user="${database_credentials%%:*}"
db_password_from_url="${database_credentials#*:}"
db_host_port="${database_location%%/*}"
db_name="${database_location#*/}"
db_host="${db_host_port%%:*}"
db_port="${db_host_port##*:}"
db_password="${ZELYRA_PASSWORD_RESET_DB_PASSWORD:-${db_password_from_url}}"

client() {
    MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" \
        --port="${db_port}" --user="${db_user}" "${db_name}" "$@"
}

cleanup() {
    if [[ -n "${server_pid}" ]]; then
        kill "${server_pid}" 2>/dev/null || true
        wait "${server_pid}" 2>/dev/null || true
    fi
    if [[ -n "${server_b_pid}" ]]; then
        kill "${server_b_pid}" 2>/dev/null || true
        wait "${server_b_pid}" 2>/dev/null || true
    fi
    if [[ -n "${smtp_pid}" ]]; then
        kill "${smtp_pid}" 2>/dev/null || true
        wait "${smtp_pid}" 2>/dev/null || true
    fi
    if [[ -n "${test_user_id}" ]]; then
        client -e "DELETE FROM password_resets WHERE user_id = ${test_user_id}" >/dev/null 2>&1 || true
        client -e "DELETE FROM auth_sessions WHERE user_id = ${test_user_id}" >/dev/null 2>&1 || true
        client -e "DELETE FROM auth_audit_log WHERE target_user_id = ${test_user_id}" >/dev/null 2>&1 || true
        client -e "DELETE FROM users WHERE id = ${test_user_id}" >/dev/null 2>&1 || true
    fi
    client -e "DROP TABLE IF EXISTS \`${rate_table}\`" >/dev/null 2>&1 || true
    client -e "DROP TABLE IF EXISTS \`${rate_table}_unavailable\`" >/dev/null 2>&1 || true
    if [[ "${ZELYRA_KEEP_E2E_TEMP:-}" == "1" ]]; then
        echo "password-reset rate-limit E2E diagnostics retained at ${temp_dir}"
    else
        python3 -c 'from pathlib import Path; import shutil,sys; shutil.rmtree(sys.argv[1], ignore_errors=True)' "${temp_dir}"
    fi
}
trap cleanup EXIT

for command in mariadb curl python3; do
    command -v "${command}" >/dev/null || { echo "error: ${command} is required" >&2; exit 1; }
done
[[ -x "${zelyra_bin}" ]] || { echo "error: Zelyra binary not found at ${zelyra_bin}" >&2; exit 1; }

python3 - "${project_file}" "${temp_dir}/rate-limit.zyl" "${rate_table}" <<'PY'
from pathlib import Path
import sys

source = Path(sys.argv[1]).read_text(encoding="utf-8")
target = Path(sys.argv[2])
table = sys.argv[3]
source = source.replace(
    "reset_rate_limits: password_reset_limits",
    f"reset_rate_limits: {table}",
    1,
).replace("reset_rate_limit: 10 per 900", "reset_rate_limit: 1 per 900", 1)
source = source.replace(
    "table password_reset_limits {",
    f"table {table} {{",
    1,
)
target.write_text(source, encoding="utf-8")
PY

address="127.0.0.1:$(python3 - <<'PY'
import socket
with socket.socket() as listener:
    listener.bind(("127.0.0.1", 0))
    print(listener.getsockname()[1])
PY
)"
address_b="127.0.0.1:$(python3 - <<'PY'
import socket
with socket.socket() as listener:
    listener.bind(("127.0.0.1", 0))
    print(listener.getsockname()[1])
PY
)"
base_url="http://${address}"
base_url_b="http://${address_b}"
export DATABASE_URL="${database_url}"
export ZELYRA_SMTP_HOST=127.0.0.1
export ZELYRA_SMTP_SECURITY=local_plaintext
export ZELYRA_SMTP_FROM=no-reply@example.test
export ZELYRA_RESET_DELIVERY_KEY=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef

echo "[1/8] validate and create the explicitly declared persistent limiter table"
"${zelyra_bin}" check "${temp_dir}/rate-limit.zyl"
"${zelyra_bin}" db setup "${temp_dir}/rate-limit.zyl"

python3 "${script_dir}/smtp_capture.py" "${temp_dir}/message.eml" "${temp_dir}/smtp.port" 0.5 "${temp_dir}/smtp.started" &
smtp_pid=$!
for _ in $(seq 1 50); do [[ -s "${temp_dir}/smtp.port" ]] && break; sleep 0.1; done
[[ -s "${temp_dir}/smtp.port" ]]
export ZELYRA_SMTP_PORT="$(cat "${temp_dir}/smtp.port")"

start_server() {
    local server_address="$1"
    local public_url="$2"
    local server_name="$3"
    env ZELYRA_PUBLIC_BASE_URL="${public_url}" "${zelyra_bin}" serve \
        "${temp_dir}/rate-limit.zyl" "${server_address}" >"${temp_dir}/${server_name}.log" 2>&1 &
    local started_pid=$!
    if [[ "${server_name}" == "server-b" ]]; then
        server_b_pid="${started_pid}"
    else
        server_pid="${started_pid}"
    fi
    for _ in $(seq 1 40); do
        if curl --silent --show-error "${public_url}/forgot-password" -o "${temp_dir}/${server_name}-forgot.html"; then return; fi
        sleep 0.25
    done
    cat "${temp_dir}/${server_name}.log" >&2
    echo "error: application did not start at ${server_address}" >&2
    exit 1
}

post_reset_request() {
    local public_url="$1"
    local page="$2"
    local origin="$3"
    local requested_email="${4:-missing-${suffix}@example.test}"
    local csrf
    csrf="$(sed -n 's/.*name="_zelyra_csrf" value="\([^"]*\)".*/\1/p' "${page}")"
    [[ -n "${csrf}" ]]
    local status
    status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
        --header "Origin: ${origin}" --data-urlencode "_zelyra_csrf=${csrf}" \
        --data-urlencode "email=${requested_email}" "${public_url}/forgot-password")"
    [[ "${status}" == 202 ]]
}

echo "[2/8] allow the first request and share a block across concurrent instances"
start_server "${address}" "${base_url}" "server"
post_reset_request "${base_url}" "${temp_dir}/server-forgot.html" "${base_url}"
first_state="$(client --batch --skip-column-names -e "SELECT request_count, blocked_until IS NULL FROM \`${rate_table}\`")"
[[ "${first_state}" == $'1\t1' ]]
start_server "${address_b}" "${base_url_b}" "server-b"
post_reset_request "${base_url}" "${temp_dir}/server-forgot.html" "${base_url}" &
post_a_pid=$!
post_reset_request "${base_url_b}" "${temp_dir}/server-b-forgot.html" "${base_url_b}" &
post_b_pid=$!
wait "${post_a_pid}"
wait "${post_b_pid}"
blocked_key="$(client --batch --skip-column-names -e "SELECT bucket_key FROM \`${rate_table}\` WHERE blocked_until > NOW()")"
[[ "${blocked_key}" =~ ^[0-9a-f]{64}$ ]]
[[ "${blocked_key}" != "127.0.0.1" ]]

echo "[3/8] restart the application and verify the block survives"
kill "${server_pid}"
wait "${server_pid}" 2>/dev/null || true
server_pid=""
kill "${server_b_pid}"
wait "${server_b_pid}" 2>/dev/null || true
server_b_pid=""
start_server "${address}" "${base_url}" "server-restarted"
post_reset_request "${base_url}" "${temp_dir}/server-restarted-forgot.html" "${base_url}"
persisted="$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM \`${rate_table}\` WHERE bucket_key = '${blocked_key}' AND blocked_until > NOW()")"
[[ "${persisted}" == "1" ]]

echo "[4/8] ensure the reset-password endpoint observes the same persistent block"
old_hash="$(printf '%s\n' "ResetRateLimit-${suffix}-Old" | "${zelyra_bin}" auth hash-password --stdin)"
client -e "INSERT INTO users (email, password_hash, active) VALUES ('${test_email}', '${old_hash}', true)"
test_user_id="$(client --batch --skip-column-names -e "SELECT id FROM users WHERE email = '${test_email}'")"
reset_token="$(python3 - <<'PY'
import secrets
print(secrets.token_hex(32))
PY
)"
reset_token_hash="$(python3 - "${reset_token}" <<'PY'
import hashlib, sys
print(hashlib.blake2s(sys.argv[1].encode()).hexdigest())
PY
)"
client -e "INSERT INTO password_resets (user_id, token_hash, expires_at, consumed_at) VALUES (${test_user_id}, '${reset_token_hash}', DATE_ADD(NOW(), INTERVAL 15 MINUTE), NULL)"
token_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --cookie-jar "${temp_dir}/reset.cookies" "${base_url}/reset-password?token=${reset_token}")"
[[ "${token_status}" == "303" ]]
curl --silent --show-error --cookie "${temp_dir}/reset.cookies" "${base_url}/reset-password" -o "${temp_dir}/reset.html"
reset_csrf="$(sed -n 's/.*name="_zelyra_csrf" value="\([^"]*\)".*/\1/p' "${temp_dir}/reset.html")"
[[ -n "${reset_csrf}" ]]
reset_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --cookie "${temp_dir}/reset.cookies" --header "Origin: ${base_url}" \
    --data-urlencode "_zelyra_csrf=${reset_csrf}" \
    --data-urlencode "password=ResetRateLimit-${suffix}-New" "${base_url}/reset-password")"
[[ "${reset_status}" == "400" ]]
[[ "$(client --batch --skip-column-names -e "SELECT password_hash FROM users WHERE id = ${test_user_id}")" == "${old_hash}" ]]
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM password_resets WHERE user_id = ${test_user_id} AND consumed_at IS NULL")" == "1" ]]

echo "[5/8] ensure only a keyed fingerprint is persisted"
columns="$(client --batch --skip-column-names -e "SHOW COLUMNS FROM \`${rate_table}\`")"
[[ "${columns}" == *"bucket_key"* ]]
[[ "${columns}" != *"ip_address"* ]]
[[ "${columns}" != *"client_ip"* ]]

echo "[6/8] verify window expiry and a block that outlives its request window"
client -e "UPDATE \`${rate_table}\` SET window_started = DATE_SUB(NOW(), INTERVAL 900 SECOND), blocked_until = NOW() WHERE bucket_key = '${blocked_key}'"
post_reset_request "${base_url}" "${temp_dir}/server-restarted-forgot.html" "${base_url}"
[[ "$(client --batch --skip-column-names -e "SELECT request_count, blocked_until IS NULL FROM \`${rate_table}\` WHERE bucket_key = '${blocked_key}'")" == $'1\t1' ]]
client -e "UPDATE \`${rate_table}\` SET window_started = DATE_SUB(NOW(), INTERVAL 1000 SECOND), blocked_until = DATE_ADD(NOW(), INTERVAL 60 SECOND) WHERE bucket_key = '${blocked_key}'"
post_reset_request "${base_url}" "${temp_dir}/server-restarted-forgot.html" "${base_url}"
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM \`${rate_table}\` WHERE bucket_key = '${blocked_key}' AND blocked_until > NOW()")" == "1" ]]

echo "[7/8] enforce the shared capacity and limit expired-row cleanup to 100 per request"
python3 - "${temp_dir}/capacity.sql" "${rate_table}" <<'PY'
from pathlib import Path
import sys
table = sys.argv[2]
rows = ",\n".join(f"('{index:064x}', NOW(), 0, NULL)" for index in range(1, 4097))
Path(sys.argv[1]).write_text(
    f"DELETE FROM `{table}`;\n"
    f"INSERT INTO `{table}` (bucket_key, window_started, request_count, blocked_until) VALUES\n{rows};\n",
    encoding="utf-8",
)
PY
client <"${temp_dir}/capacity.sql"
post_reset_request "${base_url}" "${temp_dir}/server-restarted-forgot.html" "${base_url}"
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM \`${rate_table}\`")" == "4096" ]]
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM \`${rate_table}\` WHERE bucket_key = '${blocked_key}'")" == "0" ]]
old_upper_bound="$(python3 -c 'print(f"{150:064x}")')"
client -e "UPDATE \`${rate_table}\` SET window_started = DATE_SUB(NOW(), INTERVAL 3 DAY) WHERE bucket_key <= '${old_upper_bound}'"
post_reset_request "${base_url}" "${temp_dir}/server-restarted-forgot.html" "${base_url}"
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM \`${rate_table}\` WHERE window_started < DATE_SUB(NOW(), INTERVAL 2 DAY)")" == "50" ]]
[[ "$(client --batch --skip-column-names -e "SELECT request_count FROM \`${rate_table}\` WHERE bucket_key = '${blocked_key}'")" == "1" ]]
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM \`${rate_table}\`")" == "3997" ]]

echo "[8/8] fail closed on unavailable storage and recover after its restoration"
client -e "UPDATE \`${rate_table}\` SET request_count = 0, blocked_until = NULL, window_started = NOW() WHERE bucket_key = '${blocked_key}'"
client -e "RENAME TABLE \`${rate_table}\` TO \`${rate_table}_unavailable\`"
post_reset_request "${base_url}" "${temp_dir}/server-restarted-forgot.html" "${base_url}" "${test_email}"
grep -Fq 'E-AUTH-043' "${temp_dir}/server-restarted.log"
! grep -Fq "${test_email}" "${temp_dir}/server-restarted.log"
! grep -Fq "${reset_token}" "${temp_dir}/server-restarted.log"
[[ "$(client --batch --skip-column-names -e "SELECT token_hash FROM password_resets WHERE user_id = ${test_user_id}")" == "${reset_token_hash}" ]]
[[ ! -f "${temp_dir}/message.eml" ]]
client -e "RENAME TABLE \`${rate_table}_unavailable\` TO \`${rate_table}\`"
post_reset_request "${base_url}" "${temp_dir}/server-restarted-forgot.html" "${base_url}" "${test_email}"
for _ in $(seq 1 50); do [[ -s "${temp_dir}/message.eml" ]] && break; sleep 0.1; done
[[ -s "${temp_dir}/message.eml" ]]
[[ "$(client --batch --skip-column-names -e "SELECT token_hash FROM password_resets WHERE user_id = ${test_user_id}")" != "${reset_token_hash}" ]]

echo "persistent password-reset rate-limit E2E passed"
