#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
web_port="${ZELYRA_DOCKER_E2E_WEB_PORT:-8081}"
host_port="${ZELYRA_DOCKER_E2E_HOST_PORT:-18082}"
database_host_port="${ZELYRA_DOCKER_E2E_DB_HOST_PORT:-3309}"
bundle_host_port="${ZELYRA_DOCKER_E2E_BUNDLE_HOST_PORT:-18083}"
second_bundle_host_port="${ZELYRA_DOCKER_E2E_SECOND_BUNDLE_HOST_PORT:-18084}"
address="${ZELYRA_DOCKER_E2E_ADDRESS:-127.0.0.1:${host_port}}"
zelyra_ref="${ZELYRA_DOCKER_E2E_REF:-$(git -C "${repo_dir}" branch --show-current 2>/dev/null || true)}"
bundle_compiler_commit="${ZELYRA_DOCKER_E2E_MODULE_COMMIT:-$(git -C "${repo_dir}" rev-parse HEAD)}"

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
bundle_dir="${project_root}/module-bundle"
second_bundle_dir="${project_root}/inventory-bundle"
compose_project="zelyra-generated-docker-e2e-$$"
bundle_compose_project="zelyra-module-docker-e2e-$$"
second_bundle_compose_project="zelyra-inventory-module-docker-e2e-$$"
database_container_id=""

cleanup() {
    if [[ -n "${database_container_id}" ]]; then
        docker network disconnect "${second_bundle_compose_project}_default" \
            "${database_container_id}" >/dev/null 2>&1 || true
        docker network disconnect "${bundle_compose_project}_default" \
            "${database_container_id}" >/dev/null 2>&1 || true
    fi
    if [[ -f "${second_bundle_dir}/.env" ]]; then
        docker compose --project-name "${second_bundle_compose_project}" \
            -f "${second_bundle_dir}/docker-compose.yml" \
            down --remove-orphans >/dev/null 2>&1 || true
    fi
    if [[ -f "${bundle_dir}/.env" ]]; then
        docker compose --project-name "${bundle_compose_project}" \
            -f "${bundle_dir}/docker-compose.yml" \
            down --remove-orphans >/dev/null 2>&1 || true
    fi
    if [[ -f "${project_dir}/.env" ]]; then
        docker compose --project-name "${compose_project}" \
            --env-file "${project_dir}/.env" \
            -f "${project_dir}/docker-compose.mariadb.yml" \
            down --volumes --remove-orphans >/dev/null 2>&1 || true
    fi
    rm -r -- "${project_root}"
}
trap cleanup EXIT

echo "[1/5] generating a fresh MariaDB CRUD project"
"${zelyra_bin}" new "${project_dir}" --template mariadb-crud \
    --web-port "${web_port}" \
    --host-port "${host_port}" \
    --db-host-port "${database_host_port}"
sed -i '/^database main {/,/^}/d' "${project_dir}/main.zyl"
sed -i '1i import "src/invoices.zyl" as invoices' "${project_dir}/main.zyl"
sed -i '1i import "src/inventory.zyl" as inventory' "${project_dir}/main.zyl"
sed -i '1i import "src/docker-smoke.zyl" as docker_smoke' "${project_dir}/main.zyl"
sed -i '1i import "src/inventory-smoke.zyl" as inventory_smoke' "${project_dir}/main.zyl"
printf 'database main { engine: mariadb database: "zelyra_app" }\n' \
    > "${project_dir}/src/database.zyl"
printf 'table invoices { id: Id primary auto number: String(30) required unique }\ncrud Invoice -> invoices\n' \
    > "${project_dir}/src/invoices.zyl"
printf 'table inventory { id: Id primary auto sku: String(30) required unique }\ncrud Inventory -> inventory\n' \
    > "${project_dir}/src/inventory.zyl"
printf 'page "/docker-module" { html { <h1>Imported Docker module</h1> } }\n' \
    > "${project_dir}/src/docker-smoke.zyl"
printf 'page "/inventory-module" { html { <h1>Imported inventory module</h1> } }\n' \
    > "${project_dir}/src/inventory-smoke.zyl"
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

echo "[2/5] validating the generated Compose configuration"
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

echo "[3/5] running the complete first-run setup"
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

echo "[4/5] checking recovery, CRUD pages, and published ports"
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
curl --silent --show-error --fail "http://${address}/docker-module" \
    -o "${project_root}/docker-module.html"
assert_file_contains "${project_root}/docker-module.html" '<h1>Imported Docker module</h1>' \
    "page from imported Docker module"

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
echo "[5/5] exporting two database-backed CRUD modules as independent Docker apps"
database_test_sql="CREATE DATABASE zelyra_invoice CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE TABLE zelyra_invoice.invoices LIKE zelyra_app.invoices;
INSERT INTO zelyra_invoice.invoices (number) VALUES ('INV-MODULE-ONLY');
CREATE USER 'invoice_module'@'%' IDENTIFIED BY 'invoice-module-test-only';
GRANT SELECT ON zelyra_invoice.invoices TO 'invoice_module'@'%';
CREATE DATABASE zelyra_inventory CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE TABLE zelyra_inventory.inventory LIKE zelyra_app.inventory;
INSERT INTO zelyra_inventory.inventory (sku) VALUES ('SKU-MODULE-ONLY');
CREATE USER 'inventory_module'@'%' IDENTIFIED BY 'inventory-module-test-only';
GRANT SELECT ON zelyra_inventory.inventory TO 'inventory_module'@'%';"
docker compose --project-name "${compose_project}" \
    --env-file "${project_dir}/.env" \
    -f "${project_dir}/docker-compose.mariadb.yml" \
    exec -T mariadb sh -c \
    'MYSQL_PWD="$MARIADB_ROOT_PASSWORD" mariadb --user=root --database=zelyra_app --execute="$1"' \
    sh "${database_test_sql}" >/dev/null

write_bundle_environment() {
    local directory="$1"
    local port="$2"
    local username="$3"
    local password="$4"
    local database="$5"
    (
        umask 077
        printf 'ZELYRA_HOST_PORT=%s\nDATABASE_URL=mariadb://%s:%s@mariadb:3306/%s\nZELYRA_DB_TLS_MODE=disabled\n' \
            "${port}" "${username}" "${password}" "${database}" \
            > "${directory}/.env"
    )
}

if ! "${zelyra_bin}" module bundle "${project_dir}/main.zyl" crud:Invoice \
    --output "${bundle_dir}" --docker --compiler-ref "${bundle_compiler_commit}"; then
    echo "error: selected-module Docker package generation failed" >&2
    exit 1
fi
if ! grep -Fq 'src/invoices.zyl' "${bundle_dir}/zelyra.bundle.json"; then
    echo "error: invoice Docker bundle omitted its selected CRUD module" >&2
    exit 1
fi
if ! grep -Fq 'src/database.zyl' "${bundle_dir}/zelyra.bundle.json"; then
    echo "error: invoice Docker bundle omitted the shared database module" >&2
    exit 1
fi
if grep -Fq 'src/inventory.zyl' "${bundle_dir}/zelyra.bundle.json"; then
    echo "error: invoice Docker bundle included the unrelated inventory module" >&2
    exit 1
fi
if ! grep -Fq '"database_connection_scope": "per_exported_compose_project"' \
    "${bundle_dir}/zelyra.bundle.json"; then
    echo "error: invoice Docker bundle omitted its database configuration scope" >&2
    exit 1
fi
if ! grep -Fq 'ZELYRA_DB_TLS_MODE=auto' "${bundle_dir}/.env.example"; then
    echo "error: Docker bundle does not default remote MariaDB connections to verified TLS" >&2
    exit 1
fi
write_bundle_environment "${bundle_dir}" "${bundle_host_port}" \
    invoice_module invoice-module-test-only zelyra_invoice
if ! grep -Fq 'DATABASE_URL=mariadb://invoice_module:invoice-module-test-only@mariadb:3306/zelyra_invoice' \
    "${bundle_dir}/.env"; then
    echo "error: invoice app was not configured for its independently provisioned database" >&2
    exit 1
fi
docker compose --project-name "${bundle_compose_project}" \
    -f "${bundle_dir}/docker-compose.yml" config >/dev/null
docker compose --project-name "${bundle_compose_project}" \
    -f "${bundle_dir}/docker-compose.yml" up --build --detach
database_container_id="$(docker compose --project-name "${compose_project}" \
    --env-file "${project_dir}/.env" \
    -f "${project_dir}/docker-compose.mariadb.yml" ps -q mariadb)"
docker network connect --alias mariadb \
    "${bundle_compose_project}_default" "${database_container_id}"
bundle_address="127.0.0.1:${bundle_host_port}"
for _ in $(seq 1 60); do
    if curl --silent --show-error --fail "http://${bundle_address}/invoices" \
        -o "${project_root}/bundled-module.html"; then
        break
    fi
    sleep 2
done
if ! curl --silent --show-error --fail "http://${bundle_address}/invoices" \
    -o "${project_root}/bundled-module.html"; then
    echo "error: invoice Docker app could not read its MariaDB table" >&2
    docker compose --project-name "${bundle_compose_project}" \
        -f "${bundle_dir}/docker-compose.yml" logs >&2 || true
    exit 1
fi
assert_file_contains "${project_root}/bundled-module.html" 'INV-MODULE-ONLY' \
    "invoice CRUD data read with the invoice-only database user"
if ! grep -Fq '"complete_deployment": false' "${bundle_dir}/zelyra.bundle.json"; then
    echo "error: experimental selected-module package overstated deployment completeness" >&2
    exit 1
fi
docker network disconnect "${bundle_compose_project}_default" \
    "${database_container_id}"
docker compose --project-name "${bundle_compose_project}" \
    -f "${bundle_dir}/docker-compose.yml" down --remove-orphans

if ! "${zelyra_bin}" module bundle "${project_dir}/main.zyl" crud:Inventory \
    --output "${second_bundle_dir}" --docker --compiler-ref "${bundle_compiler_commit}"; then
    echo "error: inventory Docker package generation failed" >&2
    exit 1
fi
if ! grep -Fq 'src/inventory.zyl' "${second_bundle_dir}/zelyra.bundle.json"; then
    echo "error: inventory Docker bundle omitted its selected CRUD module" >&2
    exit 1
fi
if ! grep -Fq 'src/database.zyl' "${second_bundle_dir}/zelyra.bundle.json"; then
    echo "error: inventory Docker bundle omitted the shared database module" >&2
    exit 1
fi
if grep -Fq 'src/invoices.zyl' "${second_bundle_dir}/zelyra.bundle.json"; then
    echo "error: inventory Docker bundle included the unrelated invoice module" >&2
    exit 1
fi
if ! grep -Fq '"database_connection_scope": "per_exported_compose_project"' \
    "${second_bundle_dir}/zelyra.bundle.json"; then
    echo "error: inventory Docker bundle omitted its database configuration scope" >&2
    exit 1
fi
write_bundle_environment "${second_bundle_dir}" "${second_bundle_host_port}" \
    inventory_module inventory-module-test-only zelyra_inventory
if ! grep -Fq 'DATABASE_URL=mariadb://inventory_module:inventory-module-test-only@mariadb:3306/zelyra_inventory' \
    "${second_bundle_dir}/.env"; then
    echo "error: inventory app was not configured for its independently provisioned database" >&2
    exit 1
fi
docker compose --project-name "${second_bundle_compose_project}" \
    -f "${second_bundle_dir}/docker-compose.yml" config >/dev/null
docker compose --project-name "${second_bundle_compose_project}" \
    -f "${second_bundle_dir}/docker-compose.yml" up --build --detach
docker network connect --alias mariadb \
    "${second_bundle_compose_project}_default" "${database_container_id}"
second_bundle_address="127.0.0.1:${second_bundle_host_port}"
for _ in $(seq 1 60); do
    if curl --silent --show-error --fail "http://${second_bundle_address}/inventory" \
        -o "${project_root}/bundled-inventory.html"; then
        break
    fi
    sleep 2
done
if ! curl --silent --show-error --fail "http://${second_bundle_address}/inventory" \
    -o "${project_root}/bundled-inventory.html"; then
    echo "error: inventory Docker app could not read its MariaDB table" >&2
    docker compose --project-name "${second_bundle_compose_project}" \
        -f "${second_bundle_dir}/docker-compose.yml" logs >&2 || true
    exit 1
fi
assert_file_contains "${project_root}/bundled-inventory.html" \
    'SKU-MODULE-ONLY' \
    "inventory CRUD data read with the inventory-only database user"
if ! grep -Fq '"complete_deployment": false' "${second_bundle_dir}/zelyra.bundle.json"; then
    echo "error: second experimental module package overstated deployment completeness" >&2
    exit 1
fi
docker network disconnect "${second_bundle_compose_project}_default" \
    "${database_container_id}"
docker compose --project-name "${second_bundle_compose_project}" \
    -f "${second_bundle_dir}/docker-compose.yml" down --remove-orphans
echo "generated Docker project E2E passed"
