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
parallel_password="ZelyraReset-${suffix}-Parallel"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-password-reset-e2e.XXXXXX")"
server_pid=""
smtp_pid=""
server_b_pid=""
smtp_b_pid=""

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

captured_message_count() {
    python3 - "${temp_dir}/message.eml" <<'PY'
import pathlib, sys
base = pathlib.Path(sys.argv[1])
paths = ([base] if base.is_file() else []) + list(base.parent.glob(base.name + ".*"))
print(len(paths))
PY
}

latest_captured_message() {
    python3 - "${temp_dir}/message.eml" <<'PY'
import pathlib, sys
base = pathlib.Path(sys.argv[1])
paths = ([base] if base.is_file() else []) + list(base.parent.glob(base.name + ".*"))
paths.sort(key=lambda path: 0 if path == base else int(path.name.rsplit(".", 1)[1]))
print(paths[-1] if paths else "")
PY
}

captured_reset_tokens() {
    python3 - "${temp_dir}/message.eml" <<'PY'
import email, pathlib, re, sys
base = pathlib.Path(sys.argv[1])
paths = ([base] if base.is_file() else []) + list(base.parent.glob(base.name + ".*"))
paths.sort(key=lambda path: 0 if path == base else int(path.name.rsplit(".", 1)[1]))
for path in paths:
    message = email.message_from_bytes(path.read_bytes())
    body = message.get_payload(decode=True).decode("utf-8", errors="replace")
    for token in re.findall(r"/reset-password\?token=([0-9a-f]{64})", body):
        print(token)
PY
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
    if [[ -n "${server_b_pid}" ]]; then
        kill "${server_b_pid}" 2>/dev/null || true
        wait "${server_b_pid}" 2>/dev/null || true
    fi
    if [[ -n "${smtp_b_pid}" ]]; then
        kill "${smtp_b_pid}" 2>/dev/null || true
        wait "${smtp_b_pid}" 2>/dev/null || true
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

echo "[1/9] create password-reset schema"
DATABASE_URL="${database_url}" "${zelyra_bin}" db setup "${project_file}"

echo "[2/9] start loopback SMTP capture and protected application"
python3 "${script_dir}/smtp_capture.py" "${temp_dir}/message.eml" "${temp_dir}/smtp.port" 1.5 "${temp_dir}/smtp.started" &
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

echo "[3/9] create account and establish a session"
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

echo "[4/9] compare known and unknown account responses"
known_result="$(curl --silent --show-error --output "${temp_dir}/known.html" --write-out '%{http_code} %{time_total}' \
    --cookie-jar "${temp_dir}/forgot.cookies" --header "Origin: ${base_url}" \
    --data-urlencode "_zelyra_csrf=${csrf}" --data-urlencode "email=${email}" "${base_url}/forgot-password")"
known_status="${known_result%% *}"
known_elapsed="${known_result#* }"
python3 - "${known_elapsed}" <<'PY'
import sys
if float(sys.argv[1]) >= 1.0:
    raise SystemExit("password recovery waited for the SMTP server response")
PY
unknown_status="$(curl --silent --show-error --output "${temp_dir}/unknown.html" --write-out '%{http_code}' \
    --cookie-jar "${temp_dir}/unknown.cookies" --header "Origin: ${base_url}" \
    --data-urlencode "_zelyra_csrf=${csrf}" --data-urlencode "email=${unknown_email}" "${base_url}/forgot-password")"
[[ "${known_status}" == 202 && "${unknown_status}" == 202 ]]
cmp -s "${temp_dir}/known.html" "${temp_dir}/unknown.html"
curl --silent --show-error --output "${temp_dir}/issue-race-a.html" --write-out '%{http_code}' \
    --header "Origin: ${base_url}" --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "email=${email}" "${base_url}/forgot-password" >"${temp_dir}/issue-race-a.status" &
issue_a_pid=$!
curl --silent --show-error --output "${temp_dir}/issue-race-b.html" --write-out '%{http_code}' \
    --header "Origin: ${base_url}" --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "email=${email}" "${base_url}/forgot-password" >"${temp_dir}/issue-race-b.status" &
issue_b_pid=$!
wait "${issue_a_pid}"
wait "${issue_b_pid}"
[[ "$(cat "${temp_dir}/issue-race-a.status")" == 202 ]]
[[ "$(cat "${temp_dir}/issue-race-b.status")" == 202 ]]
cmp -s "${temp_dir}/known.html" "${temp_dir}/issue-race-a.html"
cmp -s "${temp_dir}/known.html" "${temp_dir}/issue-race-b.html"
for _ in $(seq 1 40); do [[ "$(captured_message_count)" -ge 3 ]] && break; sleep 0.25; done
[[ "$(captured_message_count)" -ge 3 ]]
grep -Fq "${email}" "${temp_dir}/message.eml"
! grep -Fq "${unknown_email}" "${temp_dir}/message.eml"
! grep -Fq "${unknown_email}" "${temp_dir}"/message.eml.*

echo "[5/9] verify reset ordering across two application instances"
python3 "${script_dir}/smtp_capture.py" "${temp_dir}/message-b.eml" "${temp_dir}/smtp-b.port" 0 "${temp_dir}/smtp-b.started" &
smtp_b_pid=$!
for _ in $(seq 1 50); do [[ -s "${temp_dir}/smtp-b.port" ]] && break; sleep 0.1; done
[[ -s "${temp_dir}/smtp-b.port" ]]
address_b="127.0.0.1:$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()')"
base_url_b="http://${address_b}"
env ZELYRA_PUBLIC_BASE_URL="${base_url_b}" ZELYRA_SMTP_HOST=127.0.0.1 \
    ZELYRA_SMTP_PORT="$(cat "${temp_dir}/smtp-b.port")" ZELYRA_SMTP_SECURITY=local_plaintext \
    ZELYRA_SMTP_FROM=no-reply@example.test ZELYRA_LANGUAGE=de \
    "${zelyra_bin}" serve "${project_file}" "${address_b}" >"${temp_dir}/server-b.log" 2>&1 &
server_b_pid=$!
for _ in $(seq 1 40); do
    if curl --silent --show-error "${base_url_b}/forgot-password" -o /dev/null; then break; fi
    sleep 0.25
done
curl --silent --show-error --fail "${base_url_b}/forgot-password" -o "${temp_dir}/forgot-b.html"
csrf_b="$(sed -n 's/.*name="_zelyra_csrf" value="\([^"]*\)".*/\1/p' "${temp_dir}/forgot-b.html")"
[[ -n "${csrf_b}" ]]
curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --header "Origin: ${base_url}" --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "email=${email}" "${base_url}/forgot-password" >"${temp_dir}/cross-instance-a.status"
[[ "$(cat "${temp_dir}/cross-instance-a.status")" == 202 ]]
for _ in $(seq 1 40); do
    if [[ -s "${temp_dir}/smtp.started" ]] && [[ "$(cat "${temp_dir}/smtp.started")" == 3 ]]; then break; fi
    sleep 0.05
done
[[ -s "${temp_dir}/smtp.started" && "$(cat "${temp_dir}/smtp.started")" == 3 ]]
curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --header "Origin: ${base_url_b}" --data-urlencode "_zelyra_csrf=${csrf_b}" \
    --data-urlencode "email=${email}" "${base_url_b}/forgot-password" >"${temp_dir}/cross-instance-b.status" &
cross_b_pid=$!
wait "${cross_b_pid}"
[[ "$(cat "${temp_dir}/cross-instance-b.status")" == 202 ]]
for _ in $(seq 1 40); do [[ -s "${temp_dir}/message-b.eml" ]] && break; sleep 0.1; done
[[ -s "${temp_dir}/message-b.eml" ]]
cross_token_a="$(python3 - "${temp_dir}/message.eml.3" <<'PY'
from pathlib import Path
import email, re, sys
message = email.message_from_bytes(Path(sys.argv[1]).read_bytes())
body = message.get_payload(decode=True).decode("utf-8", errors="replace")
match = re.search(r"/reset-password\?token=([0-9a-f]{64})", body)
if not match: raise SystemExit("first cross-instance reset token missing")
print(match.group(1))
PY
)"
cross_token_b="$(python3 - "${temp_dir}/message-b.eml" <<'PY'
from pathlib import Path
import email, re, sys
message = email.message_from_bytes(Path(sys.argv[1]).read_bytes())
body = message.get_payload(decode=True).decode("utf-8", errors="replace")
match = re.search(r"/reset-password\?token=([0-9a-f]{64})", body)
if not match: raise SystemExit("second cross-instance reset token missing")
print(match.group(1))
PY
)"
[[ "${cross_token_a}" != "${cross_token_b}" ]]
cross_hash_b="$(python3 - "${cross_token_b}" <<'PY'
import hashlib, sys
print(hashlib.blake2s(sys.argv[1].encode()).hexdigest())
PY
)"
[[ "$(client --batch --skip-column-names -e "SELECT token_hash FROM password_resets WHERE user_id = ${user_id}")" == "${cross_hash_b}" ]]
[[ "${temp_dir}/message.eml.3" -ot "${temp_dir}/message-b.eml" ]]

echo "[6/9] exchange email token for a clean URL and reset cookie"
if [[ "$(captured_message_count)" -lt 3 ]]; then
    echo "error: local SMTP sink did not capture all reset messages" >&2
    exit 1
fi
mapfile -t issued_tokens < <(captured_reset_tokens)
if [[ "${#issued_tokens[@]}" -ne 4 ]]; then
    echo "error: expected four captured reset tokens, got ${#issued_tokens[@]}" >&2
    exit 1
fi
token="${cross_token_b}"
latest_message="$(latest_captured_message)"
python3 - "${latest_message}" <<'PY'
import email, pathlib, re, sys
message = email.message_from_bytes(pathlib.Path(sys.argv[1]).read_bytes())
body = message.get_payload(decode=True).decode("utf-8", errors="replace")
if "Verwende diesen Link innerhalb von 15 Minuten" not in body:
    raise SystemExit("latest reset email did not use the configured German locale")
PY
stored_token_hash="$(client --batch --skip-column-names -e "SELECT token_hash FROM password_resets WHERE user_id = ${user_id}")"
expected_token_hash="$(python3 - "${token}" <<'PY'
import hashlib, sys
print(hashlib.blake2s(sys.argv[1].encode()).hexdigest())
PY
)"
[[ "${stored_token_hash}" == "${expected_token_hash}" ]]
[[ "${stored_token_hash}" != "${token}" ]]
for stale_token in "${issued_tokens[0]}" "${issued_tokens[1]}"; do
    stale_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
        "${base_url}/reset-password?token=${stale_token}")"
    [[ "${stale_status}" == 400 ]]
done
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

echo "[7/9] reject cross-origin reset and race concurrent same-origin resets"
cross_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --cookie "${temp_dir}/reset.cookies" --header 'Origin: https://attacker.example' \
    --data-urlencode "_zelyra_csrf=${reset_csrf}" --data-urlencode "password=${new_password}" \
    "${base_url}/reset-password")"
[[ "${cross_status}" == 400 ]]
curl --silent --show-error --output "${temp_dir}/reset-race-a.html" --write-out '%{http_code}' \
    --cookie "${temp_dir}/reset.cookies" --header "Origin: ${base_url}" \
    --data-urlencode "_zelyra_csrf=${reset_csrf}" --data-urlencode "password=${new_password}" \
    "${base_url}/reset-password" >"${temp_dir}/reset-race-a.status" &
race_a_pid=$!
curl --silent --show-error --output "${temp_dir}/reset-race-b.html" --write-out '%{http_code}' \
    --cookie "${temp_dir}/reset.cookies" --header "Origin: ${base_url}" \
    --data-urlencode "_zelyra_csrf=${reset_csrf}" --data-urlencode "password=${parallel_password}" \
    "${base_url}/reset-password" >"${temp_dir}/reset-race-b.status" &
race_b_pid=$!
wait "${race_a_pid}"
wait "${race_b_pid}"
race_a_status="$(cat "${temp_dir}/reset-race-a.status")"
race_b_status="$(cat "${temp_dir}/reset-race-b.status")"
if [[ "${race_a_status}" == 303 && "${race_b_status}" == 400 ]]; then
    winning_password="${new_password}"
    losing_password="${parallel_password}"
elif [[ "${race_a_status}" == 400 && "${race_b_status}" == 303 ]]; then
    winning_password="${parallel_password}"
    losing_password="${new_password}"
else
    echo "error: concurrent reset results were ${race_a_status} and ${race_b_status}; expected one success and one rejection" >&2
    exit 1
fi
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM auth_sessions WHERE user_id = ${user_id}")" == "0" ]]
[[ "$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM password_resets WHERE user_id = ${user_id} AND consumed_at IS NOT NULL")" == "1" ]]

old_session_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' --cookie "${cookie}" "${base_url}/account")"
[[ "${old_session_status}" == "401" ]]

echo "[8/9] reject replay, exact expiry boundary, and verify new password"
replay_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    "${base_url}/reset-password?token=${token}")"
[[ "${replay_status}" == 400 ]]
winning_login_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --header "Origin: ${base_url}" --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "email=${email}" --data-urlencode "password=${winning_password}" \
    "${base_url}/login")"
losing_login_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --header "Origin: ${base_url}" --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "email=${email}" --data-urlencode "password=${losing_password}" \
    "${base_url}/login")"
[[ "${winning_login_status}" == 303 && "${losing_login_status}" == 401 ]]
rm -f "${temp_dir}"/message.eml*
curl --silent --show-error --output /dev/null --header "Origin: ${base_url}" \
    --data-urlencode "_zelyra_csrf=${csrf}" --data-urlencode "email=${email}" \
    "${base_url}/forgot-password"
for _ in $(seq 1 40); do [[ "$(captured_message_count)" -ge 1 ]] && break; sleep 0.25; done
[[ "$(captured_message_count)" -ge 1 ]]
expired_message="$(latest_captured_message)"
expired_token="$(python3 - "${expired_message}" <<'PY'
from pathlib import Path
import email, pathlib, re, sys
message = email.message_from_bytes(Path(sys.argv[1]).read_bytes())
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
client -e "UPDATE password_resets SET expires_at = NOW() WHERE user_id = ${user_id} AND token_hash = '${expired_hash}'"
expired_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    "${base_url}/reset-password?token=${expired_token}")"
[[ "${expired_status}" == "400" ]]
new_login="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --header "Origin: ${base_url}" --data-urlencode "_zelyra_csrf=${csrf}" \
    --data-urlencode "email=${email}" --data-urlencode "password=${winning_password}" \
    "${base_url}/login")"
[[ "${new_login}" == 303 ]]
! grep -Fq "${token}" "${temp_dir}/server.log"

echo "[9/9] password-reset E2E passed"
