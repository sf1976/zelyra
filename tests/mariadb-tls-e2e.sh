#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
tls_image="${MARIADB_TLS_TEST_IMAGE:-mariadb:11.4}"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-mariadb-tls.XXXXXX")"
container_name="zelyra-mariadb-tls-${$}"
root_password="zelyra-tls-test-only"
container_started=0

cleanup() {
    if [[ "${container_started}" == "1" ]]; then
        docker rm --force "${container_name}" >/dev/null 2>&1 || true
    fi
    case "${temp_dir}" in
        "${TMPDIR:-/tmp}"/zelyra-mariadb-tls.*) rm -rf -- "${temp_dir}" ;;
        *) echo "refusing to remove unexpected TLS test directory: ${temp_dir}" >&2 ;;
    esac
}
trap cleanup EXIT

for tool in docker openssl mariadb cargo; do
    if ! command -v "${tool}" >/dev/null 2>&1; then
        echo "error: ${tool} is required for the MariaDB TLS integration test" >&2
        exit 1
    fi
done

chmod 755 "${temp_dir}"
openssl req -x509 -newkey rsa:2048 -nodes -sha256 -days 2 \
    -subj "/CN=Zelyra temporary TLS test CA" \
    -addext "basicConstraints=critical,CA:TRUE" \
    -addext "keyUsage=critical,keyCertSign,cRLSign" \
    -keyout "${temp_dir}/ca-key.pem" \
    -out "${temp_dir}/ca.pem" >/dev/null 2>&1
openssl req -newkey rsa:2048 -nodes -sha256 \
    -subj "/CN=localhost" \
    -keyout "${temp_dir}/server-key.pem" \
    -out "${temp_dir}/server.csr" >/dev/null 2>&1
cat >"${temp_dir}/server.ext" <<'EOF'
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
subjectAltName=DNS:localhost,IP:127.0.0.1
EOF
openssl x509 -req -sha256 -days 2 \
    -in "${temp_dir}/server.csr" \
    -CA "${temp_dir}/ca.pem" \
    -CAkey "${temp_dir}/ca-key.pem" \
    -CAcreateserial \
    -extfile "${temp_dir}/server.ext" \
    -out "${temp_dir}/server-cert.pem" >/dev/null 2>&1
chmod 644 "${temp_dir}"/*.pem "${temp_dir}"/*.srl

docker run --detach --rm \
    --name "${container_name}" \
    --env "MARIADB_ROOT_PASSWORD=${root_password}" \
    --env MARIADB_DATABASE=zelyra_ci \
    --publish 127.0.0.1::3306 \
    --volume "${temp_dir}:/tls:ro" \
    "${tls_image}" \
    --ssl-ca=/tls/ca.pem \
    --ssl-cert=/tls/server-cert.pem \
    --ssl-key=/tls/server-key.pem \
    --require-secure-transport=ON \
    >/dev/null
container_started=1
port="$(docker port "${container_name}" 3306/tcp | awk -F: 'END {print $NF}')"
if [[ ! "${port}" =~ ^[0-9]+$ ]]; then
    echo "error: could not determine the published MariaDB TLS port" >&2
    exit 1
fi

export MYSQL_PWD="${root_password}"
ready=0
for _ in $(seq 1 60); do
    if mariadb --protocol=tcp --host=127.0.0.1 --port="${port}" \
        --user=root --ssl-ca="${temp_dir}/ca.pem" \
        --ssl-verify-server-cert --batch --skip-column-names \
        -e "SHOW SESSION STATUS LIKE 'Ssl_version'" \
        >"${temp_dir}/tls-status.txt" 2>"${temp_dir}/mariadb-client.log"; then
        if grep -q '^Ssl_version[[:space:]]\+TLSv' "${temp_dir}/tls-status.txt"; then
            ready=1
            break
        fi
    fi
    sleep 2
done
unset MYSQL_PWD
if [[ "${ready}" != "1" ]]; then
    echo "error: temporary MariaDB did not accept a verified TLS connection" >&2
    cat "${temp_dir}/mariadb-client.log" >&2
    docker logs "${container_name}" >&2
    exit 1
fi

database_url="mariadb://root:${root_password}@127.0.0.1:${port}/zelyra_ci"
export ZELYRA_DB_TIMEOUT_TEST_URL="${database_url}"
export ZELYRA_DB_TLS_MODE=required
export ZELYRA_DB_TLS_CA_CERT_FILE="${temp_dir}/ca.pem"

echo "Testing verified TLS against ${tls_image}"
cargo test -p zelyra-database \
    mariadb_pool_uses_verified_tls_when_enabled_for_the_test_database

zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
cargo build --locked -p zelyra-cli >/dev/null
DATABASE_URL="${database_url}" "${zelyra_bin}" \
    db inspect "${repo_dir}/examples/machine_management_mariadb.zyl" >/dev/null
DATABASE_URL="${database_url}" "${zelyra_bin}" \
    db apply "${repo_dir}/tests/fixtures/mariadb_tls_migration.zyl" >/dev/null
tls_history="$(DATABASE_URL="${database_url}" "${zelyra_bin}" \
    db history "${repo_dir}/tests/fixtures/mariadb_tls_migration.zyl" --format=json)"
grep -Fq '"status": "applied"' <<<"${tls_history}"

export ZELYRA_DB_TLS_CA_CERT_FILE="${temp_dir}/server-cert.pem"
if cargo test -p zelyra-database \
    mariadb_pool_uses_verified_tls_when_enabled_for_the_test_database \
    >"${temp_dir}/untrusted-ca.log" 2>&1; then
    echo "error: MariaDB TLS unexpectedly accepted an untrusted CA certificate" >&2
    exit 1
fi
if ! grep -Fq 'could not establish a verified MariaDB/MySQL TLS connection' "${temp_dir}/untrusted-ca.log"; then
    echo "error: untrusted CA failed without the expected safe TLS diagnostic" >&2
    cat "${temp_dir}/untrusted-ca.log" >&2
    exit 1
fi
if grep -Fq "${root_password}" "${temp_dir}/untrusted-ca.log"; then
    echo "error: TLS failure output exposed the test database password" >&2
    exit 1
fi

echo "Verified TLS handshake, schema inspect, migration apply/history, untrusted-CA rejection, and secret-safe diagnostics."
