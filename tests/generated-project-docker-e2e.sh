#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
web_port="${ZELYRA_DOCKER_E2E_WEB_PORT:-8081}"
host_port="${ZELYRA_DOCKER_E2E_HOST_PORT:-18082}"
database_host_port="${ZELYRA_DOCKER_E2E_DB_HOST_PORT:-3309}"
address="${ZELYRA_DOCKER_E2E_ADDRESS:-127.0.0.1:${host_port}}"
zelyra_ref="${ZELYRA_DOCKER_E2E_REF:-$(git -C "${repo_dir}" branch --show-current 2>/dev/null || true)}"

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
if [[ -n "${zelyra_ref}" && ( ! "${zelyra_ref}" =~ ^[A-Za-z0-9._/-]+$ || "${zelyra_ref}" == *..* ) ]]; then
    echo "error: ZELYRA_DOCKER_E2E_REF must be a simple Git branch or tag name" >&2
    exit 1
fi

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

echo "[1/4] generating a fresh MariaDB CRUD project"
"${zelyra_bin}" new "${project_dir}" --template mariadb-crud \
    --web-port "${web_port}" \
    --host-port "${host_port}" \
    --db-host-port "${database_host_port}"
if [[ ! -f "${project_dir}/.env" ]]; then
    echo "error: zelyra new did not create the protected local .env file" >&2
    exit 1
fi
env_mode="$(stat -c '%a' "${project_dir}/.env" 2>/dev/null || stat -f '%Lp' "${project_dir}/.env" 2>/dev/null || true)"
if [[ -n "${env_mode}" && "${env_mode}" != "600" ]]; then
    echo "error: generated .env permissions are not owner-only (0600)" >&2
    exit 1
fi
cp -p "${project_dir}/.env" "${project_root}/env.before-setup"

if [[ -n "${zelyra_ref}" ]]; then
    echo "using Zelyra runtime ref: ${zelyra_ref}"
    awk -v ref="${zelyra_ref}" '
        /^ARG ZELYRA_REF=/ { print "ARG ZELYRA_REF=" ref; replaced = 1; next }
        { print }
        END { if (!replaced) exit 1 }
    ' "${project_dir}/Dockerfile" > "${project_dir}/Dockerfile.e2e"
    mv "${project_dir}/Dockerfile.e2e" "${project_dir}/Dockerfile"
fi

echo "[2/4] validating the generated Compose configuration"
docker compose --project-name "${compose_project}" \
    --env-file "${project_dir}/.env" \
    -f "${project_dir}/docker-compose.mariadb.yml" config >/dev/null

assert_secret_free_setup_output() {
    local output="$1"
    while IFS='=' read -r key value; do
        case "${key}" in
            DATABASE_URL|MARIADB_PASSWORD|MARIADB_ROOT_PASSWORD)
                if [[ -n "${value}" && "${output}" == *"${value}"* ]]; then
                    echo "error: setup output exposed a value from .env (${key})" >&2
                    return 1
                fi
                ;;
        esac
    done < "${project_dir}/.env"
}

assert_output_contains() {
    local output="$1"
    local expected="$2"
    local description="$3"
    if [[ "${output}" != *"${expected}"* ]]; then
        echo "error: ${description} was missing from setup output" >&2
        return 1
    fi
}

assert_file_contains() {
    local file="$1"
    local expected="$2"
    local description="$3"
    if ! grep -Fq -- "${expected}" "${file}"; then
        echo "error: ${description} was missing from ${file}" >&2
        echo "rendered headings:" >&2
        grep -o '<h1[^>]*>[^<]*</h1>' "${file}" | head -n 5 >&2 || true
        echo "rendered text near relevant labels:" >&2
        grep -Eio '.{0,70}(maschine|produktion|gefunden|eintr[aä]ge|anlegen).{0,110}' \
            "${file}" | head -n 8 >&2 || true
        return 1
    fi
}

echo "[3/4] running the complete first-run setup"
if [[ -n "${zelyra_ref}" ]]; then
    echo "building the test runtime from the requested Zelyra ref without cached compiler layers"
    docker compose --project-name "${compose_project}" \
        --env-file "${project_dir}/.env" \
        -f "${project_dir}/docker-compose.mariadb.yml" \
        build --no-cache web
fi
if ! setup_output="$(COMPOSE_PROJECT_NAME="${compose_project}" \
    "${zelyra_bin}" setup --all "${project_dir}" 2>&1)"; then
    printf '%s\n' "${setup_output}" >&2
    docker compose --project-name "${compose_project}" \
        --env-file "${project_dir}/.env" \
        -f "${project_dir}/docker-compose.mariadb.yml" logs >&2 || true
    exit 1
fi
assert_output_contains "${setup_output}" 'MariaDB and the application were started' "stack start confirmation"
assert_output_contains "${setup_output}" 'database schema setup completed' "schema setup confirmation"
assert_output_contains "${setup_output}" "open: http://127.0.0.1:${host_port}" "effective web URL"
assert_secret_free_setup_output "${setup_output}"
if ! cmp -s "${project_root}/env.before-setup" "${project_dir}/.env"; then
    echo "error: zelyra setup --all changed the generated .env" >&2
    exit 1
fi

echo "[4/4] checking recovery, CRUD pages, and published ports"
if ! recovery_output="$(COMPOSE_PROJECT_NAME="${compose_project}" \
    "${zelyra_bin}" setup --all "${project_dir}" 2>&1)"; then
    printf '%s\n' "${recovery_output}" >&2
    exit 1
fi
assert_output_contains "${recovery_output}" 'kept existing .env; credentials were not changed' "existing .env preservation message"
assert_output_contains "${recovery_output}" 'database schema setup completed' "repeat schema setup confirmation"
assert_output_contains "${recovery_output}" "open: http://127.0.0.1:${host_port}" "recovery web URL"
assert_secret_free_setup_output "${recovery_output}"
if ! cmp -s "${project_root}/env.before-setup" "${project_dir}/.env"; then
    echo "error: repeated setup changed the generated .env" >&2
    exit 1
fi

for _ in $(seq 1 60); do
    if curl --silent --show-error --fail "http://${address}/machines" \
        -o "${project_root}/machines.html"; then
        break
    fi
    sleep 2
done
if ! curl --silent --show-error --fail "http://${address}/machines" \
    -o "${project_root}/machines.html"; then
    echo "error: generated Docker web container did not become ready" >&2
    docker compose --project-name "${compose_project}" \
        --env-file "${project_dir}/.env" \
        -f "${project_dir}/docker-compose.mariadb.yml" logs >&2 || true
    exit 1
fi
assert_file_contains "${project_root}/machines.html" '<h1>Maschinenpark</h1>' "German machine CRUD title"
assert_file_contains "${project_root}/machines.html" 'href="/machines/new"' "machine create link"
curl --silent --show-error --fail "http://${address}/machines/new" \
    -o "${project_root}/machine-form.html"
assert_file_contains "${project_root}/machine-form.html" 'name="number"' "machine form number field"
assert_file_contains "${project_root}/machine-form.html" 'name="department"' "machine form department field"
curl --silent --show-error --fail "http://${address}/departments" \
    -o "${project_root}/departments.html"
assert_file_contains "${project_root}/departments.html" '<h1>Produktionsbereiche</h1>' "German department CRUD title"

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
