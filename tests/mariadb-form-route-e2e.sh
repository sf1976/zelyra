#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
database_url="${ZELYRA_FORM_ROUTE_TEST_URL:-}"
if [[ -z "${database_url}" ]]; then
    echo "error: ZELYRA_FORM_ROUTE_TEST_URL must point to a disposable MariaDB test database" >&2
    exit 2
fi
if [[ ! -x "${zelyra_bin}" ]]; then
    echo "error: Zelyra binary not found at ${zelyra_bin}; run cargo build -p zelyra-cli first" >&2
    exit 1
fi

mapfile -t connection < <(python3 - "${database_url}" <<'PY'
import sys
from urllib.parse import unquote, urlsplit
url = urlsplit(sys.argv[1])
if url.scheme not in ("mariadb", "mysql") or url.hostname not in ("127.0.0.1", "localhost"):
    raise SystemExit("error: the test URL must use local MariaDB/MySQL")
database = url.path.lstrip("/")
if not database.startswith("zelyra_") or not database.replace("_", "").isalnum():
    raise SystemExit("error: test database name must be an isolated zelyra_* database")
print(url.hostname)
print(url.port or 3306)
print(unquote(url.username or ""))
print(unquote(url.password or ""))
print(database)
PY
)
db_host="${connection[0]}"
db_port="${connection[1]}"
db_user="${connection[2]}"
db_password="${connection[3]}"
project="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-form-route.XXXXXX")"
server_pid=""
http_port="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
token="$(python3 - <<'PY'
import secrets
print(secrets.token_hex(32))
PY
)"
cleanup() {
    local status=$?
    if [[ -n "${server_pid}" ]]; then
        kill "${server_pid}" 2>/dev/null || true
        wait "${server_pid}" 2>/dev/null || true
    fi
    MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" \
        "${connection[4]}" --batch --skip-column-names \
        -e "DELETE FROM csv_mailer_suppressions WHERE token_hash = SHA2('${token}', 256)" >/dev/null 2>&1 || true
    rm -rf "${project}"
    return "${status}"
}
trap cleanup EXIT

"${zelyra_bin}" check "${repo_dir}/examples/csv_mailer_unsubscribe.zyl"
DATABASE_URL="${database_url}" "${zelyra_bin}" db setup "${repo_dir}/examples/csv_mailer_unsubscribe.zyl"
MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" \
    "${connection[4]}" --batch --skip-column-names \
    -e "INSERT INTO csv_mailer_suppressions (email, token_hash) VALUES ('route-e2e@example.test', SHA2('${token}', 256))"

cp "${repo_dir}/examples/csv_mailer_unsubscribe.zyl" "${project}/main.zyl"
DATABASE_URL="${database_url}" "${zelyra_bin}" serve "${project}/main.zyl" "127.0.0.1:${http_port}" \
    >"${project}/server.log" 2>&1 &
server_pid=$!
base_url="http://127.0.0.1:${http_port}"
for _ in $(seq 1 100); do
    if curl --silent --fail "${base_url}/csv-mailer/abmelden/${token}" -o "${project}/confirmation.html"; then
        break
    fi
    sleep 0.1
done
if [[ ! -s "${project}/confirmation.html" ]]; then
    cat "${project}/server.log" >&2
    echo "error: Zelyra server did not render the valid-token confirmation" >&2
    exit 1
fi
grep -q 'E-Mail-Abmeldung bestätigen' "${project}/confirmation.html"
grep -qi '^Cache-Control: no-store' <(curl --silent --dump-header - --output /dev/null "${base_url}/csv-mailer/abmelden/${token}")

unknown_status="$(curl --silent --output /dev/null --write-out '%{http_code}' "${base_url}/csv-mailer/abmelden/$(printf '0%.0s' {1..64})")"
[[ "${unknown_status}" == "404" ]]
csrf="$(python3 - "${project}/confirmation.html" <<'PY'
import html, re, sys
source = open(sys.argv[1], encoding="utf-8").read()
match = re.search(r'name="_zelyra_csrf" value="([^"]+)"', source)
if not match:
    raise SystemExit("error: confirmation page did not contain its CSRF field")
print(html.unescape(match.group(1)))
PY
)"
unknown_token="$(printf '0%.0s' {1..64})"
unknown_post_status="$(curl --silent --output /dev/null --write-out '%{http_code}' --request POST \
    --header "Origin: ${base_url}" --header 'Content-Type: application/x-www-form-urlencoded' \
    --data-urlencode "_zelyra_csrf=${csrf}" "${base_url}/csv-mailer/abmelden/${unknown_token}")"
[[ "${unknown_post_status}" == "404" ]]
unchanged="$(MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" \
    "${connection[4]}" --batch --skip-column-names \
    -e "SELECT unsubscribed_at IS NULL FROM csv_mailer_suppressions WHERE token_hash = SHA2('${token}', 256)")"
[[ "${unchanged}" == "1" ]]
post_url="${base_url}/csv-mailer/abmelden/${token}"
origin="${base_url}"
status="$(curl --silent --output /dev/null --write-out '%{http_code}' --request POST \
    --header "Origin: ${origin}" --header 'Content-Type: application/x-www-form-urlencoded' \
    --data-urlencode "_zelyra_csrf=${csrf}" "${post_url}")"
[[ "${status}" == "303" || "${status}" == "302" ]]
first_timestamp="$(MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" \
    "${connection[4]}" --batch --skip-column-names \
    -e "SELECT unsubscribed_at FROM csv_mailer_suppressions WHERE token_hash = SHA2('${token}', 256)")"
[[ -n "${first_timestamp}" ]]

curl --silent --fail "${post_url}" -o "${project}/confirmation.html"
csrf="$(python3 - "${project}/confirmation.html" <<'PY'
import html, re, sys
source = open(sys.argv[1], encoding="utf-8").read()
match = re.search(r'name="_zelyra_csrf" value="([^"]+)"', source)
if not match:
    raise SystemExit("error: repeated confirmation did not contain a CSRF field")
print(html.unescape(match.group(1)))
PY
)"
status="$(curl --silent --output /dev/null --write-out '%{http_code}' --request POST \
    --header "Origin: ${origin}" --header 'Content-Type: application/x-www-form-urlencoded' \
    --data-urlencode "_zelyra_csrf=${csrf}" "${post_url}")"
[[ "${status}" == "303" || "${status}" == "302" ]]
second_timestamp="$(MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" \
    "${connection[4]}" --batch --skip-column-names \
    -e "SELECT unsubscribed_at FROM csv_mailer_suppressions WHERE token_hash = SHA2('${token}', 256)")"
[[ "${first_timestamp}" == "${second_timestamp}" ]]

echo "MariaDB verified unsubscribe route E2E passed"
