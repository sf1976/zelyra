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
sed -i '1i import "src/auth.zyl" as identity' "${project_dir}/main.zyl"
sed -i '1i import "src/invoices.zyl" as invoices' "${project_dir}/main.zyl"
sed -i '1i import "src/inventory.zyl" as inventory' "${project_dir}/main.zyl"
sed -i '1i import "src/customers.zyl" as customers' "${project_dir}/main.zyl"
sed -i '1i import "src/orders.zyl" as orders' "${project_dir}/main.zyl"
sed -i '1i import "src/reporting.zyl" as reporting' "${project_dir}/main.zyl"
sed -i '1i import "src/shell.zyl" as customer_order_ui' "${project_dir}/main.zyl"
sed -i '1i import "src/docker-smoke.zyl" as docker_smoke' "${project_dir}/main.zyl"
sed -i '1i import "src/inventory-smoke.zyl" as inventory_smoke' "${project_dir}/main.zyl"
printf 'database main { engine: mariadb database: "zelyra_app" }\n' \
    > "${project_dir}/src/database.zyl"
printf 'import "src/database.zyl" as storage\ntable invoices { id: Id primary auto number: String(30) required unique }\ncrud Invoice -> invoices\n' \
    > "${project_dir}/src/invoices.zyl"
printf 'import "src/database.zyl" as storage\ntable inventory { id: Id primary auto sku: String(30) required unique }\ncrud Inventory -> inventory\n' \
    > "${project_dir}/src/inventory.zyl"
printf 'page "/docker-module" { html { <h1>Imported Docker module</h1> } }\n' \
    > "${project_dir}/src/docker-smoke.zyl"
printf 'page "/inventory-module" { html { <h1>Imported inventory module</h1> } }\n' \
    > "${project_dir}/src/inventory-smoke.zyl"
cp "${repo_dir}/examples/customer_orders_modules/src/customers.zyl" "${project_dir}/src/customers.zyl"
cp "${repo_dir}/examples/customer_orders_modules/src/auth.zyl" "${project_dir}/src/auth.zyl"
cp "${repo_dir}/examples/customer_orders_modules/src/orders.zyl" "${project_dir}/src/orders.zyl"
cp "${repo_dir}/examples/customer_orders_modules/src/reporting.zyl" "${project_dir}/src/reporting.zyl"
cp "${repo_dir}/examples/customer_orders_modules/src/shell.zyl" "${project_dir}/src/shell.zyl"
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
            DATABASE_URL|ZELYRA_DATABASE_MAIN_URL|MARIADB_PASSWORD|MARIADB_ROOT_PASSWORD)
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
auth_password="E2E-Module-Auth-$$-Password"
auth_password_hash="$(printf '%s\n' "${auth_password}" | "${zelyra_bin}" auth hash-password --stdin)"
auth_admin_email="e2e-module-admin-$$@example.test"
auth_viewer_email="e2e-module-viewer-$$@example.test"
database_test_sql="INSERT INTO zelyra_app.invoices (number) VALUES ('INV-COMBINED');
INSERT INTO zelyra_app.inventory (sku) VALUES ('SKU-COMBINED');
INSERT INTO zelyra_app.users (email, password_hash, active)
VALUES ('${auth_admin_email}', '${auth_password_hash}', TRUE),
       ('${auth_viewer_email}', '${auth_password_hash}', TRUE);
SET @e2e_admin_user = (SELECT id FROM zelyra_app.users WHERE email='${auth_admin_email}');
SET @e2e_viewer_user = (SELECT id FROM zelyra_app.users WHERE email='${auth_viewer_email}');
INSERT INTO zelyra_app.role_permissions (role, permission) VALUES
    ('e2e_admin', 'customers.view'), ('e2e_admin', 'customers.create'),
    ('e2e_admin', 'customers.edit'), ('e2e_admin', 'customers.delete'),
    ('e2e_admin', 'orders.view'), ('e2e_admin', 'orders.create'),
    ('e2e_admin', 'orders.edit'), ('e2e_admin', 'orders.delete'),
    ('e2e_admin', 'reporting.view'), ('e2e_viewer', 'customers.view');
INSERT INTO zelyra_app.user_roles (user_id, role)
VALUES (@e2e_admin_user, 'e2e_admin'), (@e2e_viewer_user, 'e2e_viewer');
CREATE DATABASE zelyra_invoice CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE TABLE zelyra_invoice.invoices LIKE zelyra_app.invoices;
INSERT INTO zelyra_invoice.invoices (number) VALUES ('INV-MODULE-ONLY');
CREATE USER 'invoice_module'@'%' IDENTIFIED BY 'invoice-module-test-only';
GRANT SELECT, INSERT, UPDATE, DELETE ON zelyra_invoice.invoices TO 'invoice_module'@'%';
CREATE DATABASE zelyra_inventory CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE TABLE zelyra_inventory.inventory LIKE zelyra_app.inventory;
INSERT INTO zelyra_inventory.inventory (sku) VALUES ('SKU-MODULE-ONLY');
CREATE USER 'inventory_module'@'%' IDENTIFIED BY 'inventory-module-test-only';
GRANT SELECT, INSERT, UPDATE, DELETE ON zelyra_inventory.inventory TO 'inventory_module'@'%';"
docker compose --project-name "${compose_project}" \
    --env-file "${project_dir}/.env" \
    -f "${project_dir}/docker-compose.mariadb.yml" \
    exec -T mariadb sh -c \
    'MYSQL_PWD="$MARIADB_ROOT_PASSWORD" mariadb --user=root --database=zelyra_app --execute="$1"' \
    sh "${database_test_sql}" >/dev/null

for resource in invoices inventory; do
    curl --silent --show-error --fail "http://${address}/${resource}" \
        -o "${project_root}/combined-${resource}.html"
done
assert_file_contains "${project_root}/combined-invoices.html" 'INV-COMBINED' \
    "invoice data in the combined application"
assert_file_contains "${project_root}/combined-inventory.html" 'SKU-COMBINED' \
    "inventory data in the combined application"
# The exported apps must work without the original web process.
docker compose --project-name "${compose_project}" \
    --env-file "${project_dir}/.env" \
    -f "${project_dir}/docker-compose.mariadb.yml" stop web

assert_bundle_isolation() {
    local directory="$1" project="$2" port="$3" other_route="$4"
    local username="$5" password="$6" own_table="$7" other_table="$8"
    local status
    status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
        "http://127.0.0.1:${port}/${other_route}")"
    if [[ "${status}" != 404 ]]; then
        echo "error: unrelated module route returned ${status}, expected 404" >&2
        return 1
    fi
    docker compose --project-name "${project}" -f "${directory}/docker-compose.yml" \
        exec -T app test ! -e /app/.env
    # These are disposable fixture credentials, never production credentials.
    # Check actual database denial, not just the presence of GRANT statements.
    for query in "SELECT * FROM ${other_table}" "DELETE FROM ${other_table} WHERE 1=0"; do
        if docker compose --project-name "${project}" -f "${directory}/docker-compose.yml" \
            exec -T -e MYSQL_PWD="${password}" app mariadb --protocol=tcp \
            --host=mariadb --user="${username}" --execute="${query}" \
            >"${project_root}/permission-check.log" 2>&1; then
            echo "error: module account exceeded its own-table-only database permissions" >&2
            return 1
        fi
        if ! grep -Eq 'ERROR (1044|1142)' "${project_root}/permission-check.log"; then
            echo "error: cross-module database denial was not a permission error" >&2
            return 1
        fi
    done
}

extract_csrf_token() {
    python3 - "$1" <<'PY'
from html.parser import HTMLParser
import sys

class CsrfParser(HTMLParser):
    token = None
    def handle_starttag(self, tag, attrs):
        attributes = dict(attrs)
        if tag == "input" and attributes.get("name") == "_zelyra_csrf":
            self.token = attributes.get("value")

parser = CsrfParser()
parser.feed(open(sys.argv[1], encoding="utf-8").read())
if not parser.token:
    raise SystemExit("CSRF token missing from generated form")
print(parser.token)
PY
}

login_fixture_user() {
    local email="$1" cookie_file="$2" csrf status
    curl --silent --show-error --fail "http://${address}/login" \
        -o "${project_root}/login-form.html"
    csrf="$(extract_csrf_token "${project_root}/login-form.html")"
    status="$(curl --silent --show-error --output "${project_root}/login-response.html" \
        --write-out '%{http_code}' --header "Origin: http://${address}" \
        --cookie-jar "${cookie_file}" --data-urlencode "_zelyra_csrf=${csrf}" \
        --data-urlencode "email=${email}" --data-urlencode "password=${auth_password}" \
        "http://${address}/login")"
    if [[ "${status}" != 303 ]]; then
        echo "error: fixture login returned ${status}, expected 303" >&2
        return 1
    fi
}

extract_form_snapshot() {
    python3 - "$1" <<'PY'
from html.parser import HTMLParser
import sys

class SnapshotParser(HTMLParser):
    snapshot = None
    def handle_starttag(self, tag, attrs):
        attributes = dict(attrs)
        if tag == "input" and attributes.get("name") == "_zelyra_snapshot":
            self.snapshot = attributes.get("value")

parser = SnapshotParser()
parser.feed(open(sys.argv[1], encoding="utf-8").read())
if not parser.snapshot:
    raise SystemExit("edit snapshot missing from generated form")
print(parser.snapshot)
PY
}

assert_bundle_crud() {
    local port="$1" resource="$2" field="$3" created="$4" updated="$5"
    local origin="http://127.0.0.1:${port}" form_file="${project_root}/write-form.html"
    local token snapshot stale_snapshot status list_file="${project_root}/write-list.html" record_id
    local extra_data="${7:-}"
    local cookie_file="${8:-}"
    local extra_args=()
    local cookie_args=()
    if [[ -n "${cookie_file}" ]]; then
        cookie_args+=(--cookie "${cookie_file}")
    fi
    if [[ -n "${extra_data}" ]]; then
        extra_args+=(--data-urlencode "${extra_data}")
    fi

    curl --silent --show-error --fail "${cookie_args[@]}" "${origin}/${resource}/new" -o "${form_file}"
    token="$(extract_csrf_token "${form_file}")"
    status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
        "${cookie_args[@]}" --header "Origin: ${origin}" --data-urlencode "_zelyra_csrf=${token}" \
        --data-urlencode "${field}=${created}" "${extra_args[@]}" "${origin}/${resource}/new")"
    if [[ "${status}" != 303 ]]; then
        echo "error: generated CRUD create returned ${status}, expected 303" >&2
        return 1
    fi
    curl --silent --show-error --fail "${cookie_args[@]}" "${origin}/${resource}" -o "${list_file}"
    assert_file_contains "${list_file}" "${created}" "new row from generated CRUD create"
    record_id="$(python3 - "${list_file}" "${resource}" "${created}" <<'PY'
from html.parser import HTMLParser
import sys

class RowParser(HTMLParser):
    def __init__(self):
        super().__init__()
        self.in_row = False
        self.row_text = []
        self.row_hrefs = []
        self.record_id = None
    def handle_starttag(self, tag, attrs):
        if tag == "tr":
            self.in_row, self.row_text, self.row_hrefs = True, [], []
        if self.in_row and tag == "a":
            href = dict(attrs).get("href", "")
            if href:
                self.row_hrefs.append(href)
    def handle_data(self, data):
        if self.in_row:
            self.row_text.append(data)
    def handle_endtag(self, tag):
        if tag == "tr" and self.in_row:
            if value in " ".join(self.row_text):
                for href in self.row_hrefs:
                    prefix = f"/{resource}/"
                    suffix = href[len(prefix):] if href.startswith(prefix) else ""
                    record_id = suffix.split("/", 1)[0]
                    if record_id.isdigit():
                        self.record_id = record_id
                        return
            self.in_row = False

resource, value = sys.argv[2:]
parser = RowParser()
parser.feed(open(sys.argv[1], encoding="utf-8").read())
if parser.record_id is None:
    raise SystemExit("created CRUD row has no detail link")
print(parser.record_id)
PY
)"

    curl --silent --show-error --fail "${cookie_args[@]}" "${origin}/${resource}/${record_id}/edit" -o "${form_file}"
    token="$(extract_csrf_token "${form_file}")"
    snapshot="$(extract_form_snapshot "${form_file}")"
    stale_snapshot="${snapshot}"
    status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
        "${cookie_args[@]}" --header "Origin: ${origin}" --data-urlencode "_zelyra_csrf=${token}" \
        --data-urlencode "_zelyra_snapshot=${snapshot}" \
        --data-urlencode "${field}=${updated}" "${extra_args[@]}" \
        "${origin}/${resource}/${record_id}/edit")"
    if [[ "${status}" != 303 ]]; then
        echo "error: generated CRUD update returned ${status}, expected 303" >&2
        return 1
    fi
    curl --silent --show-error --fail "${cookie_args[@]}" "${origin}/${resource}" -o "${list_file}"
    assert_file_contains "${list_file}" "${updated}" "updated row from generated CRUD update"

    status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
        "${cookie_args[@]}" --header "Origin: ${origin}" --data-urlencode "_zelyra_csrf=${token}" \
        --data-urlencode "_zelyra_snapshot=${stale_snapshot}" \
        --data-urlencode "${field}=${created}-stale" "${extra_args[@]}" \
        "${origin}/${resource}/${record_id}/edit")"
    if [[ "${status}" != 409 ]]; then
        echo "error: stale generated CRUD update returned ${status}, expected 409" >&2
        return 1
    fi
    curl --silent --show-error --fail "${cookie_args[@]}" "${origin}/${resource}" -o "${list_file}"
    assert_file_contains "${list_file}" "${updated}" "current row after stale update rejection"
    if grep -Fq -- "${created}-stale" "${list_file}"; then
        echo "error: stale generated CRUD update overwrote the current row" >&2
        return 1
    fi

    last_crud_record_id="${record_id}"
    if [[ "${6:-false}" == "true" ]]; then
        return 0
    fi

    curl --silent --show-error --fail "${cookie_args[@]}" "${origin}/${resource}/${record_id}" -o "${form_file}"
    token="$(extract_csrf_token "${form_file}")"
    status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
        "${cookie_args[@]}" --header "Origin: ${origin}" --data-urlencode "_zelyra_csrf=${token}" \
        "${origin}/${resource}/${record_id}/delete")"
    if [[ "${status}" != 303 ]]; then
        echo "error: generated CRUD delete returned ${status}, expected 303" >&2
        return 1
    fi
    curl --silent --show-error --fail "${cookie_args[@]}" "${origin}/${resource}" -o "${list_file}"
    if grep -Fq -- "${updated}" "${list_file}"; then
        echo "error: generated CRUD delete left the removed row visible" >&2
        return 1
    fi
}

write_bundle_environment() {
    local directory="$1"
    local port="$2"
    local username="$3"
    local password="$4"
    local database="$5"
    (
        umask 077
        printf 'ZELYRA_HOST_PORT=%s\nZELYRA_DATABASE_MAIN_URL=mariadb://%s:%s@mariadb:3306/%s\nZELYRA_DB_TLS_MODE=disabled\n' \
            "${port}" "${username}" "${password}" "${database}" \
            > "${directory}/.env"
    )
}

echo "[4/5] testing the modular customer-order workflow"
docker compose --project-name "${compose_project}" \
    --env-file "${project_dir}/.env" \
    -f "${project_dir}/docker-compose.mariadb.yml" up --no-build --detach web >/dev/null
for _ in $(seq 1 60); do
    if curl --silent --show-error --fail "http://${address}/login" \
        -o "${project_root}/login.html"; then
        break
    fi
    sleep 1
done
if ! curl --silent --show-error --fail "http://${address}/login" \
    -o "${project_root}/login.html"; then
    docker compose --project-name "${compose_project}" \
        --env-file "${project_dir}/.env" \
        -f "${project_dir}/docker-compose.mariadb.yml" logs web >&2 || true
    echo "error: modular customer route did not start" >&2
    exit 1
fi
anonymous_customer_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    "http://${address}/customers")"
if [[ "${anonymous_customer_status}" != 401 ]]; then
    echo "error: anonymous customer access returned ${anonymous_customer_status}, expected 401" >&2
    exit 1
fi
admin_cookie="${project_root}/module-admin.cookies"
viewer_cookie="${project_root}/module-viewer.cookies"
login_fixture_user "${auth_admin_email}" "${admin_cookie}"
login_fixture_user "${auth_viewer_email}" "${viewer_cookie}"
viewer_create_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --cookie "${viewer_cookie}" "http://${address}/customers/new")"
if [[ "${viewer_create_status}" != 403 ]]; then
    echo "error: viewer customer create returned ${viewer_create_status}, expected 403" >&2
    exit 1
fi
viewer_report_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --cookie "${viewer_cookie}" "http://${address}/views/customerorderoverview")"
if [[ "${viewer_report_status}" != 403 ]]; then
    echo "error: viewer report access returned ${viewer_report_status}, expected 403" >&2
    exit 1
fi
customer_name="E2E-Module-Customer-$$"
customer_updated="${customer_name}-updated"
assert_bundle_crud "${host_port}" customers name "${customer_name}" "${customer_updated}" true \
    "email=${customer_name}@example.test" "${admin_cookie}"
customer_id="${last_crud_record_id}"
curl --silent --show-error --fail --cookie "${admin_cookie}" "http://${address}/orders/new" \
    -o "${project_root}/order-form.html"
assert_file_contains "${project_root}/order-form.html" \
    '<select id="customer" name="customer" required>' \
    "customer relationship selector on the order form"
assert_file_contains "${project_root}/order-form.html" "value=\"${customer_id}\"" \
    "newly created customer option on the order form"
order_csrf="$(extract_csrf_token "${project_root}/order-form.html")"
order_number="E2E-MODULE-ORDER-$$"
status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --cookie "${admin_cookie}" --header "Origin: http://${address}" \
    --data-urlencode "_zelyra_csrf=${order_csrf}" \
    --data-urlencode "customer=${customer_id}" --data-urlencode "order_number=${order_number}" \
    --data-urlencode 'status=open' --data-urlencode 'total=123.45' \
    "http://${address}/orders/new")"
if [[ "${status}" != 303 ]]; then
    echo "error: valid customer order returned ${status}, expected 303" >&2
    exit 1
fi
status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
    --cookie "${admin_cookie}" --header "Origin: http://${address}" \
    --data-urlencode "_zelyra_csrf=${order_csrf}" \
    --data-urlencode 'customer=999999999' --data-urlencode "order_number=${order_number}-invalid" \
    --data-urlencode 'status=open' --data-urlencode 'total=1.00' \
    "http://${address}/orders/new")"
if [[ "${status}" != 422 ]]; then
    echo "error: order with an unknown customer returned ${status}, expected 422" >&2
    exit 1
fi
curl --silent --show-error --fail --cookie "${admin_cookie}" "http://${address}/orders" \
    -o "${project_root}/orders.html"
assert_file_contains "${project_root}/orders.html" "${order_number}" "new modular order"
curl --silent --show-error --fail --cookie "${admin_cookie}" \
    "http://${address}/views/customerorderoverview" \
    -o "${project_root}/customer-order-report.html"
assert_file_contains "${project_root}/customer-order-report.html" "${customer_updated}" \
    "modular customer-order report row"

docker compose --project-name "${compose_project}" \
    --env-file "${project_dir}/.env" \
    -f "${project_dir}/docker-compose.mariadb.yml" stop web >/dev/null

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
if docker compose --project-name "${bundle_compose_project}" \
    -f "${bundle_dir}/docker-compose.yml" config \
    >"${project_root}/missing-env.log" 2>&1; then
    echo "error: bundle Compose accepted its missing required .env" >&2
    exit 1
fi
assert_file_contains "${project_root}/missing-env.log" '.env' "missing configuration diagnostic"
write_bundle_environment "${bundle_dir}" "${bundle_host_port}" \
    invoice_module invoice-module-test-only zelyra_invoice
if ! grep -Fq 'ZELYRA_DATABASE_MAIN_URL=mariadb://invoice_module:invoice-module-test-only@mariadb:3306/zelyra_invoice' \
    "${bundle_dir}/.env"; then
    echo "error: invoice app was not configured for its independently provisioned database" >&2
    exit 1
fi
docker compose --project-name "${bundle_compose_project}" \
    -f "${bundle_dir}/docker-compose.yml" config >/dev/null
docker compose --project-name "${bundle_compose_project}" \
    -f "${bundle_dir}/docker-compose.yml" build --no-cache
docker compose --project-name "${bundle_compose_project}" \
    -f "${bundle_dir}/docker-compose.yml" up --no-build --detach
if ! docker compose --project-name "${bundle_compose_project}" \
    -f "${bundle_dir}/docker-compose.yml" exec -T app sh -c 'zelyra db --help 2>&1 || :' \
    | grep -Fq 'ZELYRA_DATABASE_<NAME>_URL'; then
    echo "error: invoice Docker image does not contain the pinned named-database CLI" >&2
    exit 1
fi
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
assert_bundle_isolation "${bundle_dir}" "${bundle_compose_project}" "${bundle_host_port}" \
    inventory invoice_module invoice-module-test-only zelyra_invoice.invoices zelyra_inventory.inventory
assert_bundle_crud "${bundle_host_port}" invoices number \
    INV-MODULE-WRITE-001 INV-MODULE-UPDATED-001
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
if ! grep -Fq 'ZELYRA_DATABASE_MAIN_URL=mariadb://inventory_module:inventory-module-test-only@mariadb:3306/zelyra_inventory' \
    "${second_bundle_dir}/.env"; then
    echo "error: inventory app was not configured for its independently provisioned database" >&2
    exit 1
fi
docker compose --project-name "${second_bundle_compose_project}" \
    -f "${second_bundle_dir}/docker-compose.yml" config >/dev/null
docker compose --project-name "${second_bundle_compose_project}" \
    -f "${second_bundle_dir}/docker-compose.yml" build --no-cache
docker compose --project-name "${second_bundle_compose_project}" \
    -f "${second_bundle_dir}/docker-compose.yml" up --no-build --detach
if ! docker compose --project-name "${second_bundle_compose_project}" \
    -f "${second_bundle_dir}/docker-compose.yml" exec -T app sh -c 'zelyra db --help 2>&1 || :' \
    | grep -Fq 'ZELYRA_DATABASE_<NAME>_URL'; then
    echo "error: inventory Docker image does not contain the pinned named-database CLI" >&2
    exit 1
fi
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
assert_bundle_isolation "${second_bundle_dir}" "${second_bundle_compose_project}" "${second_bundle_host_port}" \
    invoices inventory_module inventory-module-test-only zelyra_inventory.inventory zelyra_invoice.invoices
assert_bundle_crud "${second_bundle_host_port}" inventory sku \
    SKU-MODULE-WRITE-001 SKU-MODULE-UPDATED-001
if ! grep -Fq '"complete_deployment": false' "${second_bundle_dir}/zelyra.bundle.json"; then
    echo "error: second experimental module package overstated deployment completeness" >&2
    exit 1
fi
docker network disconnect "${second_bundle_compose_project}_default" \
    "${database_container_id}"
docker compose --project-name "${second_bundle_compose_project}" \
    -f "${second_bundle_dir}/docker-compose.yml" down --remove-orphans
echo "generated Docker project E2E passed"
