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
for command in docker mariadb curl python3; do
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

echo "[1/4] generating the MariaDB business project"
"${zelyra_bin}" new "${project_dir}" --template mariadb-business \
    --web-port 8080 --host-port 18090 \
    --db-host-port "${generated_database_host_port}"
"${zelyra_bin}" setup "${project_dir}"

echo "[2/4] checking the generated business project"
"${zelyra_bin}" check "${project_dir}/main.zyl"
docker compose --env-file "${project_dir}/.env" \
    -f "${project_dir}/docker-compose.mariadb.yml" config >/dev/null

TEST_DATABASE_URL="${database_url}" awk '
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
    --env-file "${project_dir}/.env.e2e" --port 18090 --json)"
printf '%s' "${doctor_json}" | python3 -c '
import json
import sys

document = json.load(sys.stdin)
if document.get("status") != "ready":
    raise SystemExit("generated business project doctor did not report ready")
if any(check.get("status") != "pass" for check in document.get("checks", [])):
    raise SystemExit("generated business project doctor reported a non-passing check")
print("generated business project doctor: ready")
'
"${zelyra_bin}" doc "${project_dir}/main.zyl" --openapi > "${project_root}/openapi.json"
"${zelyra_bin}" doc "${project_dir}/main.zyl" --typescript > "${project_root}/client.ts"
python3 - "${project_root}/openapi.json" "${project_root}/client.ts" <<'PY'
import json
import pathlib
import sys

openapi = json.loads(pathlib.Path(sys.argv[1]).read_text())
if openapi.get("openapi") != "3.0.3":
    raise SystemExit("generated business project did not produce OpenAPI 3.0.3")
if "/api/customers/{id}" not in openapi.get("paths", {}):
    raise SystemExit("generated business project OpenAPI is missing the customer API")
client = pathlib.Path(sys.argv[2]).read_text()
if "class ZelyraClient" not in client or "get_api_customers_id" not in client:
    raise SystemExit("generated business project TypeScript client is incomplete")
print("generated business project API documentation: ready")
PY

echo "[3/4] running authentication, CRUD, API, and permission integration"
DATABASE_URL="${database_url}" \
    ZELYRA_LANGUAGE=en \
    ZELYRA_PROTECTED_E2E_DB_PASSWORD="${root_password}" \
    ZELYRA_BIN="${zelyra_bin}" \
    ZELYRA_PROTECTED_E2E_PROJECT="${project_dir}/main.zyl" \
    ZELYRA_PROTECTED_E2E_ADDRESS="${address}" \
    "${script_dir}/mariadb-protected-e2e.sh"
echo "[4/4] generated business project E2E passed"
