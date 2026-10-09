#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
project_file="${ZELYRA_PASSWORD_RESET_PROJECT:-${repo_dir}/examples/password_reset.zyl}"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
address="${ZELYRA_PASSWORD_RESET_ADDRESS:-127.0.0.1:38540}"
base_url="http://${address}"
database_url="${DATABASE_URL:-}"
suffix="$(date +%s)"
email="zelyra-reset-${suffix}@example.test"
unknown_email="absent-reset-${suffix}@example.test"
old_password="ZelyraReset-${suffix}-Old"
new_password="ZelyraReset-${suffix}-New"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-password-reset-e2e.XXXXXX")"
server_pid=""
smtp_pid=""

if [[ "${database_url}" == mariadb://* ]]; then
    database_parts="${database_url#mariadb://}"
elif [[ "${database_url}" == mysql://* ]]; then
    database_parts="${database_url#mysql://}"
else
    echo "error: DATABASE_URL must point to a MariaDB test database" >&2
    exit 1
fi

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
    if [[ -n "${smtp_pid}" ]]; then
        kill "${smtp_pid}" 2>/dev/null || true
        wait "${smtp_pid}" 2>/dev/null || true
    fi
    client --batch --skip-column-names <<SQL >/dev/null 2>&1 || true
DELETE FROM password_resets WHERE user_id IN (SELECT id FROM users WHERE email = '${email}');
DELETE FROM auth_sessions WHERE user_id IN (SELECT id FROM users WHERE email = '${email}');
DELETE FROM auth_audit_log WHERE target_user_id IN (SELECT id FROM users WHERE email = '${email}');
DELETE FROM users WHERE email = '${email}';
SQL
    if [[ "${ZELYRA_KEEP_E2E_TEMP:-}" == "1" ]]; then
        echo "password-reset E2E diagnostics retained at ${temp_dir}"
    else
        python3 -c 'from pathlib import Path; import shutil,sys; shutil.rmtree(sys.argv[1], ignore_errors=True)' "${temp_dir}"
    fi
}
trap cleanup EXIT

for command in mariadb curl python3; do
    command -v "${command}" >/dev/null || { echo "error: ${command} is required" >&2; exit 1; }
done
[[ -x "${zelyra_bin}" ]] || { echo "error: Zelyra binary not found at ${zelyra_bin}" >&2; exit 1; }

echo "[1/8] create password-reset schema"
DATABASE_URL="${database_url}" "${zelyra_bin}" db setup "${project_file}"

echo "[2/8] start loopback SMTP capture and protected application"
python3 "${script_dir}/smtp_capture.py" "${temp_dir}/message.eml" "${temp_dir}/smtp.port" &
smtp_pid=$!
for _ in $(seq 1 50); do [[ -s "${temp_dir}/smtp.port" ]] && break; sleep 0.1; done
[[ -s "${temp_dir}/smtp.port" ]]
export DATABASE_URL="${database_url}"
export ZELYRA_PUBLIC_BASE_URL="${base_url}"
export ZELYRA_SMTP_HOST=127.0.0.1
export ZELYRA_SMTP_PORT="$(cat "${temp_dir}/smtp.port")"
export ZELYRA_SMTP_SECURITY=local_plaintext
export ZELYRA_SMTP_FROM=no-reply@example.test
export ZELYRA_LANGUAGE=de
"${zelyra_bin}" serve "${project_file}" "${address}" >"${temp_dir}/server.log" 2>&1 &
server_pid=$!
for _ in $(seq 1 40); do
    if curl --silent --show-error "${base_url}/forgot-password" -o "${temp_dir}/forgot.html"; then break; fi
    sleep 0.25
done
curl --silent --show-error --fail "${base_url}/forgot-password" -o "${temp_dir}/forgot.html"

echo "[3/8] create account and establish a session"
old_hash="$(printf '%s\n' "${old_password}" | "${zelyra_bin}" auth hash-password --stdin)"
client -e "INSERT INTO users (email, password_hash, active) VALUES ('${email}', '${old_hash}', true)"
user_id="$(client --batch --skip-column-names -e "SELECT id FROM users WHERE email = '${email}'")"
csrf="$(sed -n 's/.*name="_zelyra_csrf" value="\([^"]*\)".*/\1/p' "${temp_dir}/forgot.html")"
[[ -n "${csrf}" ]]
cookie="${temp_dir}/session.cookies"
curl --silent --show-error --output /dev/null --cookie-jar "${cookie}" \
    --header "Origin: ${base_url}" --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "email=${email}" --data-urlencode "password=${old_password}" \
    "${base_url}/login"
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM auth_sessions WHERE user_id = ${user_id}")" == "1" ]]

echo "[4/8] compare known and unknown account responses"
known_status="$(curl --silent --show-error --output "${temp_dir}/known.html" --write-out '%{http_code}' \
    --cookie-jar "${temp_dir}/forgot.cookies" --header "Origin: ${base_url}" \
    --data-urlencode "_zelyra_csrf=${csrf}" --data-urlencode "email=${email}" "${base_url}/forgot-password")"
unknown_status="$(curl --silent --show-error --output "${temp_dir}/unknown.html" --write-out '%{http_code}' \
    --cookie-jar "${temp_dir}/unknown.cookies" --header "Origin: ${base_url}" \
    --data-urlencode "_zelyra_csrf=${csrf}" --data-urlencode "email=${unknown_email}" "${base_url}/forgot-password")"
[[ "${known_status}" == 202 && "${unknown_status}" == 202 ]]
cmp -s "${temp_dir}/known.html" "${temp_dir}/unknown.html"
for _ in $(seq 1 40); do [[ -s "${temp_dir}/message.eml" ]] && break; sleep 0.25; done
grep -Fq "${email}" "${temp_dir}/message.eml"
! grep -Fq "${unknown_email}" "${temp_dir}/message.eml"

echo "[5/8] exchange email token for a clean URL and reset cookie"
if [[ ! -s "${temp_dir}/message.eml" ]]; then
    echo "error: local SMTP sink did not capture a reset message" >&2
    exit 1
fi
token="$(python3 - "${temp_dir}/message.eml" <<'PY'
import email, pathlib, re, sys
message = email.message_from_bytes(pathlib.Path(sys.argv[1]).read_bytes())
body = message.get_payload(decode=True).decode("utf-8", errors="replace")
if "Verwende diesen Link innerhalb von 15 Minuten" not in body:
    raise SystemExit("reset email did not use the configured German locale")
match = re.search(r"/reset-password\?token=([0-9a-f]{64})", body)
if not match:
    print(body, file=sys.stderr)
    raise SystemExit("reset token missing from captured email")
print(match.group(1))
PY
)"
stored_token_hash="$(client --batch --skip-column-names -e "SELECT token_hash FROM password_resets WHERE user_id = ${user_id}")"
expected_token_hash="$(python3 - "${token}" <<'PY'
import hashlib, sys
print(hashlib.blake2s(sys.argv[1].encode()).hexdigest())
PY
)"
[[ "${stored_token_hash}" == "${expected_token_hash}" ]]
[[ "${stored_token_hash}" != "${token}" ]]
! grep -Fq "${token}" "${temp_dir}/server.log"
token_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --cookie-jar "${temp_dir}/reset.cookies" --dump-header "${temp_dir}/reset.headers" \
    "${base_url}/reset-password?token=${token}")"
[[ "${token_status}" == 303 ]]
grep -qi '^Location: /reset-password' "${temp_dir}/reset.headers"
grep -qi 'Cache-Control: no-store' "${temp_dir}/reset.headers"
grep -qi 'Referrer-Policy: no-referrer' "${temp_dir}/reset.headers"
reset_csrf="$(curl --silent --show-error --cookie "${temp_dir}/reset.cookies" \
    "${base_url}/reset-password" -o "${temp_dir}/reset.html"; sed -n 's/.*name="_zelyra_csrf" value="\([^"]*\)".*/\1/p' "${temp_dir}/reset.html")"
[[ -n "${reset_csrf}" ]]

echo "[6/8] reject cross-origin reset and accept same-origin reset"
cross_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --cookie "${temp_dir}/reset.cookies" --header 'Origin: https://attacker.example' \
    --data-urlencode "_zelyra_csrf=${reset_csrf}" --data-urlencode "password=${new_password}" \
    "${base_url}/reset-password")"
[[ "${cross_status}" == 400 ]]
reset_status="$(curl --silent --show-error --output "${temp_dir}/reset-result.html" --write-out '%{http_code}' \
    --cookie "${temp_dir}/reset.cookies" --cookie-jar "${temp_dir}/reset.cookies" \
    --header "Origin: ${base_url}" --data-urlencode "_zelyra_csrf=${reset_csrf}" \
    --data-urlencode "password=${new_password}" "${base_url}/reset-password")"
[[ "${reset_status}" == 303 ]]
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM auth_sessions WHERE user_id = ${user_id}")" == "0" ]]
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM password_resets WHERE user_id = ${user_id} AND consumed_at IS NOT NULL")" == "1" ]]

old_session_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' --cookie "${cookie}" "${base_url}/account")"
[[ "${old_session_status}" == "401" ]]

echo "[7/8] reject replay and verify new password"
replay_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    "${base_url}/reset-password?token=${token}")"
[[ "${replay_status}" == 400 ]]
curl --silent --show-error --output /dev/null --header "Origin: ${base_url}" \
    --data-urlencode "_zelyra_csrf=${csrf}" --data-urlencode "email=${email}" \
    "${base_url}/forgot-password"
expired_token="$(python3 - "${temp_dir}/message.eml" <<'PY'
import email, pathlib, re, sys
message = email.message_from_bytes(pathlib.Path(sys.argv[1]).read_bytes())
body = message.get_payload(decode=True).decode("utf-8", errors="replace")
if "Verwende diesen Link innerhalb von 15 Minuten" not in body:
    raise SystemExit("second reset email did not use the configured German locale")
match = re.search(r"/reset-password\?token=([0-9a-f]{64})", body)
if not match:
    raise SystemExit("second reset token missing from captured email")
print(match.group(1))
PY
)"
expired_hash="$(python3 - "${expired_token}" <<'PY'
import hashlib, sys
print(hashlib.blake2s(sys.argv[1].encode()).hexdigest())
PY
)"
client -e "UPDATE password_resets SET expires_at = DATE_SUB(NOW(), INTERVAL 1 SECOND) WHERE user_id = ${user_id} AND token_hash = '${expired_hash}'"
expired_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    "${base_url}/reset-password?token=${expired_token}")"
[[ "${expired_status}" == "400" ]]
new_login="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --header "Origin: ${base_url}" --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "email=${email}" --data-urlencode "password=${new_password}" \
    "${base_url}/login")"
[[ "${new_login}" == 303 ]]
! grep -Fq "${token}" "${temp_dir}/server.log"

echo "[8/8] password-reset E2E passed"
