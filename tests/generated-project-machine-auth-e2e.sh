#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
root_password="${ZELYRA_MACHINE_AUTH_GENERATED_ROOT_PASSWORD:-}"
database_host="${ZELYRA_MACHINE_AUTH_GENERATED_DB_HOST:-127.0.0.1}"
database_port="${ZELYRA_MACHINE_AUTH_GENERATED_DB_PORT:-3306}"
generated_database_host_port="${ZELYRA_MACHINE_AUTH_GENERATED_DB_HOST_PORT:-3312}"
host_port="${ZELYRA_MACHINE_AUTH_GENERATED_HOST_PORT:-18087}"
address="${ZELYRA_MACHINE_AUTH_GENERATED_ADDRESS:-127.0.0.1:38531}"

[[ -n "${root_password}" ]] || {
    echo "error: ZELYRA_MACHINE_AUTH_GENERATED_ROOT_PASSWORD must be set" >&2
    exit 1
}
[[ -x "${zelyra_bin}" ]] || {
    echo "error: Zelyra binary not found at ${zelyra_bin}" >&2
    exit 1
}
for command in docker mariadb; do
    command -v "${command}" >/dev/null 2>&1 || {
        echo "error: ${command} is required for the generated machine auth E2E test" >&2
        exit 1
    }
done

project_root="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-generated-machine-auth.XXXXXX")"
project_dir="${project_root}/app"
database_name="zelyra_generated_machine_auth_${$}"
database_url="mariadb://root:${root_password}@${database_host}:${database_port}/${database_name}"

cleanup() {
    MYSQL_PWD="${root_password}" mariadb \
        --protocol=tcp --host="${database_host}" --port="${database_port}" \
        --user=root --batch --skip-column-names \
        -e "DROP DATABASE IF EXISTS \`${database_name}\`;" >/dev/null 2>&1 || true
    rm -rf "${project_root}"
}
trap cleanup EXIT

echo "[1/5] generating a fresh MariaDB project shell"
"${zelyra_bin}" new "${project_dir}" --template mariadb-auth \
    --web-port 8080 --host-port "${host_port}" \
    --db-host-port "${generated_database_host_port}"
cp "${repo_dir}/examples/machine_management_auth.zyl" "${project_dir}/main.zyl"
docker compose --env-file "${project_dir}/.env" \
    -f "${project_dir}/docker-compose.mariadb.yml" config >/dev/null

echo "[2/5] creating an isolated acceptance database"
MYSQL_PWD="${root_password}" mariadb \
    --protocol=tcp --host="${database_host}" --port="${database_port}" \
    --user=root --batch --skip-column-names \
    -e "CREATE DATABASE \`${database_name}\`;"

echo "[3/5] checking the protected machine source"
"${zelyra_bin}" check "${project_dir}/main.zyl" >/dev/null

echo "[4/5] running search, forms, and permission acceptance"
DATABASE_URL="${database_url}" \
    ZELYRA_MACHINE_AUTH_DB_PASSWORD="${root_password}" \
    ZELYRA_MACHINE_AUTH_PROJECT="${project_dir}/main.zyl" \
    ZELYRA_MACHINE_AUTH_ADDRESS="${address}" \
    ZELYRA_BIN="${zelyra_bin}" \
    "${script_dir}/machine-management-auth-e2e.sh"

echo "[5/5] generated machine authorization E2E passed"
