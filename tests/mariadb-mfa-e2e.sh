#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
database_url="${DATABASE_URL:-}"
[[ "${database_url}" == mariadb://* || "${database_url}" == mysql://* ]] || { echo "DATABASE_URL must point to a disposable MariaDB server" >&2; exit 2; }
url="${database_url#*://}"
credentials="${url%@*}"
location="${url#*@}"
user="${credentials%%:*}"
url_password="${credentials#*:}"
host_port="${location%%/*}"
db_host="${host_port%%:*}"
db_port="${host_port##*:}"
base_db="${location#*/}"
password="${ZELYRA_MFA_E2E_DB_PASSWORD:-${url_password}}"
test_db="zelyra_mfa_e2e_$(date +%s)_$$"
email="mfa-${test_db}@example.test"
test_password="Zelyra-MFA-${test_db}-Pass"
key="0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
work="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-mfa-e2e.XXXXXX")"
server_pid=""

client() {
  MYSQL_PWD="${password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${user}" "$@"
}

cleanup() {
  if [[ -n "${server_pid}" ]]; then kill "${server_pid}" 2>/dev/null || true; wait "${server_pid}" 2>/dev/null || true; fi
  client --batch --skip-column-names -e "DROP DATABASE IF EXISTS \`${test_db}\`" >/dev/null 2>&1 || true
  rm -rf "${work}"
}
trap cleanup EXIT

command -v mariadb >/dev/null
command -v curl >/dev/null
[[ -x "${zelyra_bin}" ]] || { echo "Zelyra CLI binary not found: ${zelyra_bin}" >&2; exit 2; }

ready=0
for _ in $(seq 1 60); do
  if client --batch --skip-column-names --execute='SELECT 1' >/dev/null 2>&1; then ready=1; break; fi
  sleep 1
done
[[ "${ready}" -eq 1 ]] || { echo "MariaDB did not become ready" >&2; exit 1; }
client --batch --skip-column-names -e "CREATE DATABASE \`${test_db}\`" >/dev/null
url_prefix="${database_url%/*}"
test_url="${url_prefix}/${test_db}"
DATABASE_URL="${test_url}" "${zelyra_bin}" db setup "${repo_dir}/examples/local_mfa.zyl" >/dev/null
password_hash="$(printf '%s\n' "${test_password}" | "${zelyra_bin}" auth hash-password --stdin)"
MYSQL_PWD="${password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${user}" "${test_db}" \
  --execute="INSERT INTO users (email, password_hash) VALUES ('${email}', '${password_hash}')"

port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()')"
DATABASE_URL="${test_url}" ZELYRA_LANGUAGE=de ZELYRA_MFA_ENCRYPTION_KEY="${key}" "${zelyra_bin}" serve "${repo_dir}/examples/local_mfa.zyl" "127.0.0.1:${port}" >"${work}/server.log" 2>&1 &
server_pid=$!
base="http://127.0.0.1:${port}"
for _ in $(seq 1 100); do
  if curl --silent --output /dev/null "${base}/login"; then break; fi
  if ! kill -0 "${server_pid}" 2>/dev/null; then cat "${work}/server.log" >&2; exit 1; fi
  sleep 0.1
done

MFA_E2E_BASE="${base}" MFA_E2E_EMAIL="${email}" MFA_E2E_PASSWORD="${test_password}" MFA_E2E_DATABASE_URL="${test_url}" MFA_E2E_DB_PASSWORD="${password}" python3 <<'PY'
import base64, hashlib, hmac, http.cookiejar, os, re, time, urllib.error, urllib.parse, urllib.request
from concurrent.futures import ThreadPoolExecutor
import subprocess

base = os.environ["MFA_E2E_BASE"]
email = os.environ["MFA_E2E_EMAIL"]
password = os.environ["MFA_E2E_PASSWORD"]
origin = base
cookies = http.cookiejar.CookieJar()
client = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(cookies))

def get(path):
    req = urllib.request.Request(base + path, headers={"Origin": origin, "Referer": base + "/"})
    try:
        with client.open(req, timeout=10) as response:
            return response.status, response.geturl(), response.read().decode()
    except urllib.error.HTTPError as error:
        return error.code, error.geturl(), error.read().decode()

def post(path, values):
    req = urllib.request.Request(base + path, data=urllib.parse.urlencode(values).encode(), headers={
        "Origin": origin, "Referer": base + path, "Content-Type": "application/x-www-form-urlencoded"
    }, method="POST")
    try:
        with client.open(req, timeout=10) as response:
            return response.status, response.geturl(), response.read().decode()
    except urllib.error.HTTPError as error:
        return error.code, error.geturl(), error.read().decode()

def csrf(html):
    match = re.search(r'name="_zelyra_csrf" value="([^"]+)"', html)
    assert match, "CSRF token missing"
    return match.group(1)

def login():
    _, _, page = get("/login")
    assert "Anmelden" in page, "login language setting was not applied"
    status, url, body = post("/login", {"_zelyra_csrf": csrf(page), "email": email, "password": password})
    assert status == 200 and url.endswith("/login/mfa"), (status, url, body[:200])
    return csrf(body)

def code(secret, step=None):
    step = int(time.time() // 30) + 1 if step is None else step
    key = base64.b32decode(secret + "=" * ((8 - len(secret) % 8) % 8))
    digest = hmac.new(key, step.to_bytes(8, "big"), hashlib.sha1).digest()
    offset = digest[-1] & 0x0f
    binary = int.from_bytes(digest[offset:offset + 4], "big") & 0x7fffffff
    return f"{binary % 1_000_000:06d}"

_, _, page = get("/login")
assert "Anmelden" in page, "login language setting was not applied"
status, url, body = post("/login", {"_zelyra_csrf": csrf(page), "email": email, "password": password})
assert status == 200 and url.endswith("/"), (status, url, body[:200])
_, _, security = get("/account/security")
status, _, enrollment = post("/account/security", {"_zelyra_csrf": csrf(security), "action": "begin", "password": password})
assert status == 200 and "Authenticator einrichten" in enrollment
secret_match = re.search(r"<code>([A-Z2-7]+)</code>", enrollment)
assert secret_match, "enrollment secret missing"
secret = secret_match.group(1)
enrollment_step = int(time.time() // 30)
status, _, codes_page = post("/account/security", {
    "_zelyra_csrf": csrf(enrollment), "action": "confirm", "password": password, "code": code(secret, enrollment_step)
})
assert status == 200 and "MFA aktiviert" in codes_page
recovery = re.findall(r"<li><code>([^<]+)</code></li>", codes_page)
assert len(recovery) == 10

_, _, security = get("/account/security")
assert "Kontosicherheit" in security
post("/logout", {"_zelyra_csrf": csrf(security)})

challenge_csrf = login()
assert get("/")[0] == 401, "password-only session must not access guarded pages"
assert get("/api/private")[0] == 401, "password-only session must not access guarded APIs"
status, url, body = post("/login/mfa", {"_zelyra_csrf": challenge_csrf, "code": code(secret, enrollment_step + 1)})
assert status == 200 and url.endswith("/"), (status, url, body[:200])
assert get("/")[0] == 200, "verified session should access guarded pages"
api_status, api_url, api_body = get("/api/private")
assert api_status == 200 and '"verified"' in api_body, (api_status, api_url, api_body[:200])
_, _, security = get("/account/security")
post("/logout", {"_zelyra_csrf": csrf(security)})

challenge_csrf = login()
status, _, _ = post("/login/mfa", {"_zelyra_csrf": challenge_csrf, "code": code(secret, enrollment_step + 1)})
assert status == 401, "a used TOTP time step must be rejected"
status, url, _ = post("/login/mfa", {"_zelyra_csrf": challenge_csrf, "code": recovery[0]})
assert status == 200 and url.endswith("/"), (status, url)
_, _, security = get("/account/security")
post("/logout", {"_zelyra_csrf": csrf(security)})
challenge_csrf = login()
status, _, _ = post("/login/mfa", {"_zelyra_csrf": challenge_csrf, "code": recovery[0]})
assert status == 401, "a used recovery code must be rejected"

def make_pending_challenge():
    isolated = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
    def send(path, values=None):
        data = None if values is None else urllib.parse.urlencode(values).encode()
        req = urllib.request.Request(base + path, data=data, headers={
            "Origin": origin, "Referer": base + path,
            "Content-Type": "application/x-www-form-urlencoded",
        }, method="GET" if values is None else "POST")
        try:
            with isolated.open(req, timeout=10) as response:
                return response.status, response.geturl(), response.read().decode()
        except urllib.error.HTTPError as error:
            return error.code, error.geturl(), error.read().decode()
    _, _, login_page = send("/login")
    _, _, challenge = send("/login", {
        "_zelyra_csrf": csrf(login_page), "email": email, "password": password,
    })
    assert "Zusätzliche Bestätigung" in challenge, "German MFA challenge copy missing"
    return isolated, csrf(challenge)

challenges = [make_pending_challenge(), make_pending_challenge()]
while int(time.time() // 30) <= enrollment_step:
    time.sleep(0.2)
fresh_step = int(time.time() // 30) + 1
fresh_code = code(secret, fresh_step)

def submit_concurrent_challenge(pair, factor_code):
    isolated, token = pair
    req = urllib.request.Request(base + "/login/mfa", data=urllib.parse.urlencode({
        "_zelyra_csrf": token, "code": factor_code,
    }).encode(), headers={
        "Origin": origin, "Referer": base + "/login/mfa",
        "Content-Type": "application/x-www-form-urlencoded",
    }, method="POST")
    try:
        with isolated.open(req, timeout=10) as response:
            response.read()
            return response.status
    except urllib.error.HTTPError as error:
        error.read()
        return error.code

with ThreadPoolExecutor(max_workers=2) as executor:
    concurrent_statuses = sorted(executor.map(
        lambda pair: submit_concurrent_challenge(pair, fresh_code), challenges
    ))
assert concurrent_statuses == [200, 401], (
    "exactly one concurrent session may consume a TOTP step", concurrent_statuses
)

recovery_challenges = [make_pending_challenge(), make_pending_challenge()]
with ThreadPoolExecutor(max_workers=2) as executor:
    concurrent_recovery_statuses = sorted(executor.map(
        lambda pair: submit_concurrent_challenge(pair, recovery[1]), recovery_challenges
    ))
assert concurrent_recovery_statuses == [200, 401], (
    "exactly one concurrent session may consume a recovery code", concurrent_recovery_statuses
)

lock_client, lock_csrf = make_pending_challenge()
database = urllib.parse.urlsplit(os.environ["MFA_E2E_DATABASE_URL"])
database_name = database.path.lstrip("/")
database_user = urllib.parse.unquote(database.username or "")
database_host = database.hostname or "127.0.0.1"
database_port = str(database.port or 3306)
def db_command(sql):
    result = subprocess.run([
        "mariadb", "--protocol=tcp", "--host", database_host, "--port", database_port,
        "--user", database_user, database_name, "--batch", "--skip-column-names", "--execute", sql,
    ], env={**os.environ, "MYSQL_PWD": os.environ["MFA_E2E_DB_PASSWORD"]},
       stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=False)
    assert result.returncode == 0, "test database operation failed"
    return result.stdout.strip()
db_command(f"UPDATE user_mfa SET failed_attempts = 0, locked_until = NULL WHERE user_id = (SELECT id FROM users WHERE email = '{email}')")
valid_window_codes = {code(secret, step) for step in range(int(time.time() // 30) - 1, int(time.time() // 30) + 2)}
wrong_code = next(f"{candidate:06d}" for candidate in range(1_000_000) if f"{candidate:06d}" not in valid_window_codes)
for _ in range(5):
    status = submit_concurrent_challenge((lock_client, lock_csrf), wrong_code)
    assert status == 401, ("the fifth failed factor attempt should record the lock without leaking lock state", status)
locked_status = submit_concurrent_challenge((lock_client, lock_csrf), recovery[2])
assert locked_status == 429, ("a locked factor must reject further attempts", locked_status)
lockout_state = db_command(
    f"SELECT CONCAT(failed_attempts, ' ', locked_until > CURRENT_TIMESTAMP, ' ', locked_until <= DATE_ADD(CURRENT_TIMESTAMP, INTERVAL 15 MINUTE)) FROM user_mfa WHERE user_id = (SELECT id FROM users WHERE email = '{email}')"
)
assert lockout_state == "5 1 1", ("five failures must store an active 15-minute lock", lockout_state)
db_command(
    f"UPDATE user_mfa SET locked_until = DATE_SUB(CURRENT_TIMESTAMP, INTERVAL 1 SECOND) WHERE user_id = (SELECT id FROM users WHERE email = '{email}')"
)
expired_lock_status = submit_concurrent_challenge((lock_client, lock_csrf), recovery[2])
assert expired_lock_status == 200, ("an expired lock must allow a valid recovery code", expired_lock_status)
print("MariaDB local TOTP enrollment, page/API session gate, sequential and concurrent TOTP/recovery replay rejection, lockout threshold, and German screen copy passed")
PY
echo "MFA fifth-failure lockout and elapsed-expiry recovery passed"
echo "[recovery rehearsal] removing the test account's factor and sessions while preserving its audit trail"
MYSQL_PWD="${password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${user}" "${test_db}" --execute="START TRANSACTION; DELETE FROM user_mfa_recovery WHERE user_id = (SELECT id FROM users WHERE email = '${email}'); DELETE FROM user_mfa WHERE user_id = (SELECT id FROM users WHERE email = '${email}'); DELETE FROM auth_sessions WHERE user_id = (SELECT id FROM users WHERE email = '${email}'); COMMIT;" >/dev/null
recovery_state="$(MYSQL_PWD="${password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${user}" "${test_db}" --batch --skip-column-names --execute="SELECT CONCAT((SELECT COUNT(*) FROM user_mfa WHERE user_id = users.id), ' ', (SELECT COUNT(*) FROM user_mfa_recovery WHERE user_id = users.id), ' ', (SELECT COUNT(*) FROM auth_sessions WHERE user_id = users.id), ' ', (SELECT COUNT(*) FROM auth_audit_log WHERE actor_user_id = users.id)) FROM users WHERE email = '${email}'")"
read -r factor_count recovery_count session_count audit_count <<<"${recovery_state}"
[[ "${factor_count}" == "0" && "${recovery_count}" == "0" && "${session_count}" == "0" && "${audit_count}" -ge 1 ]]
echo "operator recovery rehearsal passed; factor and sessions removed, audit entry retained"
