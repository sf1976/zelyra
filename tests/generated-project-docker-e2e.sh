#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
web_port="${ZELYRA_DOCKER_E2E_WEB_PORT:-8081}"
host_port="${ZELYRA_DOCKER_E2E_HOST_PORT:-18082}"
database_host_port="${ZELYRA_DOCKER_E2E_DB_HOST_PORT:-3309}"
address="${ZELYRA_DOCKER_E2E_ADDRESS:-127.0.0.1:${host_port}}"
zelyra_ref="${ZELYRA_DOCKER_E2E_REF:-main}"

if [[ ! -x "${zelyra_bin}" ]]; then
    echo "error: Zelyra binary not found at ${zelyra_bin}; run cargo build -p zelyra-cli first" >&2
    exit 1
fi
for command in docker curl; do
    if ! command -v "${command}" >/dev/null 2>&1; then
        echo "error: ${command} is required for the generated Docker E2E test" >&2
        exit 1
    fi
done

project_root="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-generated-docker-e2e.XXXXXX")"
project_dir="${project_root}/app"
compose_project="zelyra-generated-docker-e2e-$$"

cleanup() {
    if [[ -f "${project_dir}/.env" ]]; then
        docker compose --project-name "${compose_project}" \
            --env-file "${project_dir}/.env" \
            -f "${project_dir}/docker-compose.mariadb.yml" \
            down --volumes --remove-orphans >/dev/null 2>&1 || true
    fi
    rm -r -- "${project_root}"
}
trap cleanup EXIT

echo "[1/3] generating the Docker project"
"${zelyra_bin}" new "${project_dir}" --mariadb \
    --web-port "${web_port}" \
    --host-port "${host_port}" \
    --db-host-port "${database_host_port}"
"${zelyra_bin}" setup "${project_dir}"

echo "[2/3] starting the generated MariaDB and web containers"
docker compose --project-name "${compose_project}" \
    --env-file "${project_dir}/.env" \
    -f "${project_dir}/docker-compose.mariadb.yml" config >/dev/null
docker compose --project-name "${compose_project}" \
    --env-file "${project_dir}/.env" \
    -f "${project_dir}/docker-compose.mariadb.yml" build \
    --build-arg "ZELYRA_REF=${zelyra_ref}"
docker compose --project-name "${compose_project}" \
    --env-file "${project_dir}/.env" \
    -f "${project_dir}/docker-compose.mariadb.yml" up -d

for _ in $(seq 1 60); do
    if curl --silent --show-error --fail "http://${address}/" \
        -o "${project_root}/response.html"; then
        break
    fi
    sleep 2
done
if ! curl --silent --show-error --fail "http://${address}/" \
    -o "${project_root}/response.html"; then
    echo "error: generated Docker web container did not become ready" >&2
    docker compose --project-name "${compose_project}" \
        --env-file "${project_dir}/.env" \
        -f "${project_dir}/docker-compose.mariadb.yml" logs >&2 || true
    exit 1
fi
# Fresh minimal MariaDB projects default to German unless overridden.
grep -Fq 'Dein Arbeitsbereich kann wachsen.' "${project_root}/response.html"

echo "[3/3] checking the published ports"
docker compose --project-name "${compose_project}" \
    --env-file "${project_dir}/.env" \
    -f "${project_dir}/docker-compose.mariadb.yml" port mariadb 3306 \
    | grep -Fq ":${database_host_port}"
docker compose --project-name "${compose_project}" \
    --env-file "${project_dir}/.env" \
    -f "${project_dir}/docker-compose.mariadb.yml" port web "${web_port}" \
    | grep -Fq ":${host_port}"
docker compose --project-name "${compose_project}" \
    --env-file "${project_dir}/.env" \
    -f "${project_dir}/docker-compose.mariadb.yml" ps
echo "generated Docker project E2E passed"
