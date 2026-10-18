#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
database_url="${DATABASE_URL:-}"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
suffix="$(date +%s%N)"
work="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-tenant-relations.XXXXXX")"

if [[ "${database_url}" == mariadb://* ]]; then
    parts="${database_url#mariadb://}"
elif [[ "${database_url}" == mysql://* ]]; then
    parts="${database_url#mysql://}"
else
    echo "error: DATABASE_URL must point to a local MariaDB test database" >&2
    exit 1
fi
credentials="${parts%@*}"
location="${parts#*@}"
user="${credentials%%:*}"
password="${ZELYRA_E2E_DB_PASSWORD:-${credentials#*:}}"
host_port="${location%%/*}"
base_db="${location#*/}"
host="${host_port%%:*}"
port="${host_port##*:}"
[[ "${host}" == 127.0.0.1 || "${host}" == localhost ]] || {
    echo "error: this integration test only accepts a local MariaDB" >&2
    exit 1
}
[[ "${base_db}" =~ ^zelyra_[A-Za-z0-9_]+$ ]] || {
    echo "error: base database must use the zelyra_* test prefix" >&2
    exit 1
}
database="${base_db}_relations_$$"
test_url="${database_url%/*}/${database}"

client() {
    MYSQL_PWD="${password}" mariadb --protocol=tcp --host="${host}" \
        --port="${port}" --user="${user}" "$@"
}

cleanup() {
    local status=$?
    client "${base_db}" --batch --skip-column-names \
        -e "DROP DATABASE IF EXISTS \`${database}\`;" >/dev/null 2>&1 || true
    rm -rf -- "${work}"
    return "${status}"
}
trap cleanup EXIT

for command in mariadb; do
    command -v "${command}" >/dev/null || {
        echo "error: ${command} is required" >&2
        exit 1
    }
done
[[ -x "${zelyra_bin}" ]] || {
    echo "error: Zelyra binary not found at ${zelyra_bin}" >&2
    exit 1
}

client "${base_db}" --batch --skip-column-names \
    -e "CREATE DATABASE \`${database}\` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;"
cat >"${work}/main.zyl" <<'ZELYRA'
type TenantId = Id

table customers scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    name: String required
}

table invoices scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    customer: Customer?
}

fn main() {}
ZELYRA

echo "[1/3] applying optional tenant relation schema"
DATABASE_URL="${test_url}" "${zelyra_bin}" db setup "${work}/main.zyl"
plan="$(DATABASE_URL="${test_url}" "${zelyra_bin}" db plan "${work}/main.zyl")"
grep -Fq "No schema changes" <<<"${plan}"

echo "[2/3] proving same-tenant and optional relations work"
client "${database}" --batch --skip-column-names <<SQL
INSERT INTO customers (tenant_id, name) VALUES (101, 'tenant A'), (202, 'tenant B');
INSERT INTO invoices (tenant_id, customer_id) VALUES
    (101, NULL),
    (101, (SELECT id FROM customers WHERE tenant_id = 101)),
    (202, (SELECT id FROM customers WHERE tenant_id = 202));
SQL

echo "[3/3] proving MariaDB rejects a cross-tenant relation"
if output="$(client "${database}" --batch --skip-column-names \
    -e "INSERT INTO invoices (tenant_id, customer_id) VALUES (101, (SELECT id FROM customers WHERE tenant_id = 202));" 2>&1)"; then
    echo "error: MariaDB accepted a relation to a different tenant" >&2
    exit 1
fi
grep -Eiq 'foreign key|constraint|1452' <<<"${output}" || {
    echo "error: failed insert did not report a foreign-key violation" >&2
    exit 1
}
echo "tenant relation constraint rejected cross-tenant insert"
