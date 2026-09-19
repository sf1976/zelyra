#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
root_password="${ZELYRA_GENERATED_E2E_ROOT_PASSWORD:-}"
template="${ZELYRA_GENERATED_E2E_TEMPLATE:-mariadb-crud}"
host_port="${ZELYRA_GENERATED_E2E_HOST_PORT:-18081}"
generated_database_host_port="${ZELYRA_GENERATED_E2E_GENERATED_DB_HOST_PORT:-3309}"
address="${ZELYRA_GENERATED_E2E_ADDRESS:-127.0.0.1:38520}"
database_host="${ZELYRA_GENERATED_E2E_DB_HOST:-127.0.0.1}"
database_port="${ZELYRA_GENERATED_E2E_DB_PORT:-3308}"

if [[ -z "${root_password}" ]]; then
    echo "error: ZELYRA_GENERATED_E2E_ROOT_PASSWORD must be set" >&2
    exit 1
fi
if [[ ! -x "${zelyra_bin}" ]]; then
    echo "error: Zelyra binary not found at ${zelyra_bin}; run cargo build -p zelyra-cli first" >&2
    exit 1
fi
for command in docker mariadb curl python3; do
    if ! command -v "${command}" >/dev/null 2>&1; then
        echo "error: ${command} is required for the generated-project E2E test" >&2
        exit 1
    fi
done

project_root="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-generated-project-e2e.XXXXXX")"
project_dir="${project_root}/app"
database_name="zelyra_generated_e2e_${$}"
test_database_url="mariadb://root:${root_password}@${database_host}:${database_port}/${database_name}"

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

echo "[1/4] generating a fresh MariaDB project"
"${zelyra_bin}" new "${project_dir}" --template "${template}" \
    --web-port 8080 --host-port "${host_port}" \
    --db-host-port "${generated_database_host_port}"
"${zelyra_bin}" setup "${project_dir}"

echo "[2/4] validating generated Compose and doctor configuration"
docker compose --env-file "${project_dir}/.env" \
    -f "${project_dir}/docker-compose.mariadb.yml" config >/dev/null
TEST_DATABASE_URL="${test_database_url}" awk '
    BEGIN { updated = 0 }
    /^DATABASE_URL=/ {
        print "DATABASE_URL=" ENVIRON["TEST_DATABASE_URL"]
        updated = 1
        next
    }
    { print }
    END {
        if (!updated) print "DATABASE_URL=" ENVIRON["TEST_DATABASE_URL"]
    }
' "${project_dir}/.env" > "${project_dir}/.env.e2e"
doctor_json="$("${zelyra_bin}" doctor "${project_dir}/main.zyl" \
    --env-file "${project_dir}/.env.e2e" --port "${host_port}" --json)"
printf '%s' "${doctor_json}" | python3 -c '
import json
import sys

document = json.load(sys.stdin)
if document.get("status") != "ready":
    raise SystemExit("generated project doctor did not report ready")
if any(check.get("status") != "pass" for check in document.get("checks", [])):
    raise SystemExit("generated project doctor reported a non-passing check")
print("generated project doctor: ready")
'

echo "[3/4] running schema and CRUD HTTP integration"
DATABASE_URL="${test_database_url}" \
    ZELYRA_LANGUAGE=en \
    ZELYRA_BIN="${zelyra_bin}" \
    ZELYRA_E2E_PROJECT="${project_dir}/main.zyl" \
    ZELYRA_E2E_ADDRESS="${address}" \
    "${script_dir}/mariadb-e2e.sh"

echo "[4/4] generated-project MariaDB E2E passed"
