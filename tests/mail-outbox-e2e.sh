#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
database_url="${DATABASE_URL:-}"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-mail-e2e.XXXXXX")"
smtp_pid=""

cleanup() {
  if [[ -n "${smtp_pid}" ]]; then
    kill "${smtp_pid}" 2>/dev/null || true
    wait "${smtp_pid}" 2>/dev/null || true
  fi
  if [[ "${ZELYRA_KEEP_E2E_TEMP:-}" == "1" ]]; then
    echo "mail E2E diagnostics retained at ${temp_dir}"
  else
    python3 -c 'from pathlib import Path; import shutil,sys; shutil.rmtree(sys.argv[1], ignore_errors=True)' "${temp_dir}"
  fi
}
trap cleanup EXIT

[[ -n "${database_url}" ]] || { echo "error: DATABASE_URL is required" >&2; exit 1; }
[[ -x "${zelyra_bin}" ]] || { echo "error: Zelyra binary not found at ${zelyra_bin}" >&2; exit 1; }
command -v mariadb >/dev/null
command -v python3 >/dev/null
command -v openssl >/dev/null

mkdir "${temp_dir}/project"
cp "${script_dir}/../examples/mail_outbox.zyl" "${temp_dir}/project/main.zyl"
cat >"${temp_dir}/project/zelyra.toml" <<'TOML'
[project]
name = "mail-outbox-e2e"
version = "0.1.0"
zelyra = "0.1"

[capabilities]
database = true
email = true
TOML
cat >>"${temp_dir}/project/zelyra.toml" <<'TOML'

[network]
allowed_hosts = ["localhost"]
TOML

openssl req -x509 -newkey rsa:2048 -nodes -days 1 \
  -keyout "${temp_dir}/ca.key" -out "${temp_dir}/ca.crt" \
  -subj "/CN=Zelyra E2E SMTP Root" \
  -addext "basicConstraints=critical,CA:TRUE" \
  -addext "keyUsage=critical,keyCertSign,cRLSign" >/dev/null 2>&1
openssl req -x509 -newkey rsa:2048 -nodes -days 1 \
  -keyout "${temp_dir}/untrusted-ca.key" -out "${temp_dir}/untrusted-ca.crt" \
  -subj "/CN=Untrusted Zelyra E2E SMTP Root" \
  -addext "basicConstraints=critical,CA:TRUE" \
  -addext "keyUsage=critical,keyCertSign,cRLSign" >/dev/null 2>&1
openssl req -newkey rsa:2048 -nodes -keyout "${temp_dir}/smtp.key" \
  -out "${temp_dir}/smtp.csr" -subj "/CN=localhost" >/dev/null 2>&1
cat >"${temp_dir}/smtp.ext" <<'EXT'
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
subjectAltName=DNS:localhost
EXT
openssl x509 -req -in "${temp_dir}/smtp.csr" -CA "${temp_dir}/ca.crt" \
  -CAkey "${temp_dir}/ca.key" -CAcreateserial -days 1 \
  -extfile "${temp_dir}/smtp.ext" -out "${temp_dir}/smtp.crt" >/dev/null 2>&1
export ZELYRA_SMTP_TEST_TLS_CERT="${temp_dir}/smtp.crt"
export ZELYRA_SMTP_TEST_TLS_KEY="${temp_dir}/smtp.key"
export ZELYRA_SMTP_TEST_USERNAME=zelyra-test-user
export ZELYRA_SMTP_TEST_PASSWORD=zelyra-test-password
python3 "${script_dir}/smtp_capture.py" "${temp_dir}/message.eml" "${temp_dir}/smtp.port" 0 &
smtp_pid=$!
for _ in $(seq 1 50); do [[ -s "${temp_dir}/smtp.port" ]] && break; sleep 0.1; done
[[ -s "${temp_dir}/smtp.port" ]]

export DATABASE_URL="${database_url}"
export ZELYRA_SMTP_HOST=localhost
export ZELYRA_SMTP_PORT="$(cat "${temp_dir}/smtp.port")"
export ZELYRA_SMTP_SECURITY=starttls
export ZELYRA_SMTP_CA_CERT="${temp_dir}/ca.crt"
export ZELYRA_SMTP_FROM=no-reply@example.test
export ZELYRA_SMTP_USERNAME="${ZELYRA_SMTP_TEST_USERNAME}"
export ZELYRA_SMTP_PASSWORD="${ZELYRA_SMTP_TEST_PASSWORD}"

echo "[1/4] create the internal outbox table"
"${zelyra_bin}" outbox setup "${temp_dir}/project/main.zyl"

echo "[2/4] queue a message inside the business transaction"
"${zelyra_bin}" run "${temp_dir}/project/main.zyl"

echo "[3/4] dispatch one message through the local SMTP sink"
export ZELYRA_SMTP_CA_CERT="${temp_dir}/untrusted-ca.crt"
"${zelyra_bin}" outbox run "${temp_dir}/project/main.zyl" \
  --handler zelyra.mail.send=deliver_mail --once >"${temp_dir}/untrusted.log" 2>&1
grep -Fq "0 delivered, 1 retry scheduled" "${temp_dir}/untrusted.log"
[[ ! -e "${temp_dir}/message.eml" ]]

export ZELYRA_SMTP_CA_CERT="${temp_dir}/ca.crt"
delivered=0
for _ in $(seq 1 30); do
  "${zelyra_bin}" outbox run "${temp_dir}/project/main.zyl" \
    --handler zelyra.mail.send=deliver_mail --once >"${temp_dir}/trusted.log" 2>&1
  if grep -Fq "1 delivered" "${temp_dir}/trusted.log"; then delivered=1; break; fi
  sleep 0.5
done
[[ "${delivered}" == "1" ]]

echo "[4/4] verify MIME content and acknowledgement"
for _ in $(seq 1 50); do [[ -s "${temp_dir}/message.eml" ]] && break; sleep 0.1; done
[[ -s "${temp_dir}/message.eml" ]]
grep -Fq "Your Zelyra order" "${temp_dir}/message.eml"
grep -Fq "Your order has been recorded." "${temp_dir}/message.eml"
grep -Fq "<p>Your order has been recorded.</p>" "${temp_dir}/message.eml"
grep -Fq "Message-ID:" "${temp_dir}/message.eml"

python3 - "${temp_dir}/message.eml" <<'PY'
import email, pathlib, sys
message = email.message_from_bytes(pathlib.Path(sys.argv[1]).read_bytes())
assert message.get_content_type() == "multipart/alternative"
assert message.get("To") == "customer@example.test"
assert message.get("From") == "no-reply@example.test"
assert message.get("Message-ID", "").startswith("<")
parts = {part.get_content_type(): part.get_payload(decode=True).decode("utf-8")
         for part in message.walk() if part.get_content_maintype() != "multipart"}
assert parts["text/plain"] == "Your order has been recorded."
assert parts["text/html"] == "<p>Your order has been recorded.</p>"
PY

echo "[5/5] verify the SMTP greeting timeout"
kill "${smtp_pid}" 2>/dev/null || true
wait "${smtp_pid}" 2>/dev/null || true
smtp_pid=""
export ZELYRA_SMTP_TEST_GREETING_DELAY=6
python3 "${script_dir}/smtp_capture.py" "${temp_dir}/timeout.eml" "${temp_dir}/timeout.port" 0 &
smtp_pid=$!
for _ in $(seq 1 50); do [[ -s "${temp_dir}/timeout.port" ]] && break; sleep 0.1; done
[[ -s "${temp_dir}/timeout.port" ]]
export ZELYRA_SMTP_PORT="$(cat "${temp_dir}/timeout.port")"
"${zelyra_bin}" run "${temp_dir}/project/main.zyl"
"${zelyra_bin}" outbox run "${temp_dir}/project/main.zyl" \
  --handler zelyra.mail.send=deliver_mail --once >"${temp_dir}/timeout.log" 2>&1
grep -Fq "0 delivered, 1 retry scheduled" "${temp_dir}/timeout.log"
[[ ! -e "${temp_dir}/timeout.eml" ]]
