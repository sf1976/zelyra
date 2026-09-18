#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
root_password="${ZELYRA_GENERATED_BUSINESS_ROOT_PASSWORD:-}"
database_host="${ZELYRA_GENERATED_BUSINESS_DB_HOST:-127.0.0.1}"
database_port="${ZELYRA_GENERATED_BUSINESS_DB_PORT:-3308}"
generated_database_host_port="${ZELYRA_GENERATED_BUSINESS_GENERATED_DB_HOST_PORT:-3309}"
address="${ZELYRA_GENERATED_BUSINESS_ADDRESS:-127.0.0.1:38530}"

if [[ -z "${root_password}" ]]; then
    echo "error: ZELYRA_GENERATED_BUSINESS_ROOT_PASSWORD must be set" >&2
    exit 1
fi
if [[ ! -x "${zelyra_bin}" ]]; then
    echo "error: Zelyra binary not found at ${zelyra_bin}; run cargo build -p zelyra-cli first" >&2
    exit 1
fi
for command in docker mariadb curl; do
    if ! command -v "${command}" >/dev/null 2>&1; then
        echo "error: ${command} is required for the generated business E2E test" >&2
        exit 1
    fi
done

project_root="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-generated-business-e2e.XXXXXX")"
project_dir="${project_root}/app"
database_name="zelyra_generated_business_e2e_${$}"
database_url="mariadb://root:${root_password}@${database_host}:${database_port}/${database_name}"

cleanup() {
    MYSQL_PWD="${root_password}" mariadb \
        --protocol=tcp --host="${database_host}" --port="${database_port}" \
        --user=root --batch --skip-column-names \
        -e "DROP DATABASE IF EXISTS \`${database_name}\`;" >/dev/null 2>&1 || true
    rm -r -- "${project_root}"
}
trap cleanup EXIT

MYSQL_PWD="${root_password}" mariadb \
    --protocol=tcp --host="${database_host}" --port="${database_port}" \
    --user=root --batch --skip-column-names \
    -e "CREATE DATABASE \`${database_name}\`;"

echo "[1/3] generating the MariaDB business project"
"${zelyra_bin}" new "${project_dir}" --template mariadb-business \
    --web-port 8080 --host-port 18090 \
    --db-host-port "${generated_database_host_port}"
"${zelyra_bin}" setup "${project_dir}"

echo "[2/3] checking the generated business project"
"${zelyra_bin}" check "${project_dir}/main.zyl"
docker compose --env-file "${project_dir}/.env" \
    -f "${project_dir}/docker-compose.mariadb.yml" config >/dev/null

echo "[3/3] running authentication, CRUD, API, and permission integration"
DATABASE_URL="${database_url}" \
    ZELYRA_PROTECTED_E2E_DB_PASSWORD="${root_password}" \
    ZELYRA_BIN="${zelyra_bin}" \
    ZELYRA_PROTECTED_E2E_PROJECT="${project_dir}/main.zyl" \
    ZELYRA_PROTECTED_E2E_ADDRESS="${address}" \
    "${script_dir}/mariadb-protected-e2e.sh"
echo "generated business project E2E passed"
