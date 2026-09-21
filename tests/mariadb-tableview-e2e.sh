#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
project_file="${ZELYRA_TABLEVIEW_E2E_PROJECT:-${repo_dir}/examples/tableview.zyl}"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
address="${ZELYRA_TABLEVIEW_E2E_ADDRESS:-127.0.0.1:38515}"
tableview_path="${ZELYRA_TABLEVIEW_E2E_PATH:-/views/customers}"
base_url="http://${address}"
database_url="${DATABASE_URL:-}"
suffix="$(date +%s)"
customer_prefix="Zelyra E2E Customer ${suffix}"
pagination_prefix="Zelyra E2E Pagination ${suffix}"
alpha_name="${customer_prefix} Alpha"
beta_name="${customer_prefix} Beta <Customer>"
gamma_name="${customer_prefix} Gamma"
escaped_beta_name="$(printf '%s' "${beta_name}" | sed 's/&/\&amp;/g; s/</\&lt;/g; s/>/\&gt;/g')"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-mariadb-tableview-e2e.XXXXXX")"
server_pid=""

if [[ "${database_url}" == mariadb://* ]]; then
    database_parts="${database_url#mariadb://}"
elif [[ "${database_url}" == mysql://* ]]; then
    database_parts="${database_url#mysql://}"
else
    echo "error: DATABASE_URL must point to a MariaDB test database" >&2
    exit 1
fi

database_credentials="${database_parts%@*}"
database_location="${database_parts#*@}"
db_user="${database_credentials%%:*}"
db_password_from_url="${database_credentials#*:}"
db_host_port="${database_location%%/*}"
db_name="${database_location#*/}"
db_host="${db_host_port%%:*}"
if [[ "${db_host_port}" == *:* ]]; then
    db_port="${db_host_port##*:}"
else
    db_port="3306"
fi
db_password="${ZELYRA_TABLEVIEW_E2E_DB_PASSWORD:-${db_password_from_url}}"

client() {
    MYSQL_PWD="${db_password}" mariadb \
        --protocol=tcp \
        --host="${db_host}" \
        --port="${db_port}" \
        --user="${db_user}" \
        "${db_name}" "$@"
}

cleanup() {
    if [[ -n "${server_pid}" ]]; then
        kill "${server_pid}" 2>/dev/null || true
        wait "${server_pid}" 2>/dev/null || true
    fi
    client --batch --skip-column-names <<SQL >/dev/null 2>&1 || true
DELETE o
FROM orders o
JOIN customers c ON c.id = o.customer_id
WHERE c.name LIKE '${customer_prefix}%'
   OR c.name LIKE '${pagination_prefix}%';
DELETE FROM customers
WHERE name LIKE '${customer_prefix}%'
   OR name LIKE '${pagination_prefix}%';
SQL
    if [[ "${ZELYRA_TABLEVIEW_E2E_KEEP_TEMP:-}" == "1" ]]; then
        echo "kept tableview E2E files in ${temp_dir}" >&2
    else
        rm -rf "${temp_dir}"
    fi
}
trap cleanup EXIT

if ! command -v mariadb >/dev/null 2>&1; then
    echo "error: mariadb client is required for the MariaDB tableview integration test" >&2
    exit 1
fi
if ! command -v curl >/dev/null 2>&1; then
    echo "error: curl is required for the MariaDB tableview integration test" >&2
    exit 1
fi
if [[ ! -x "${zelyra_bin}" ]]; then
    echo "error: Zelyra binary not found at ${zelyra_bin}; run cargo build -p zelyra-cli first" >&2
    exit 1
fi

echo "[1/7] setting up MariaDB schema"
"${zelyra_bin}" db setup "${project_file}"

echo "[2/7] checking the struct-backed tableview"
"${zelyra_bin}" check "${project_file}"

echo "[3/7] inserting aggregate and pagination fixtures"
client --batch --skip-column-names <<SQL
INSERT INTO customers (name, email) VALUES
    ('${alpha_name}', 'alpha-${suffix}@example.test'),
    ('${beta_name}', 'beta-${suffix}@example.test'),
    ('${gamma_name}', NULL);
SQL

alpha_id="$(client --batch --skip-column-names <<SQL
SELECT id FROM customers WHERE name = '${alpha_name}';
SQL
)"
beta_id="$(client --batch --skip-column-names <<SQL
SELECT id FROM customers WHERE name = '${beta_name}';
SQL
)"
gamma_id="$(client --batch --skip-column-names <<SQL
SELECT id FROM customers WHERE name = '${gamma_name}';
SQL
)"
[[ -n "${alpha_id}" && -n "${beta_id}" && -n "${gamma_id}" ]]

client --batch --skip-column-names <<SQL
INSERT INTO orders (customer_id, total) VALUES
    (${alpha_id}, 100.00),
    (${alpha_id}, 50.00),
    (${beta_id}, 75.00);
SQL

for index in $(seq -w 1 26); do
    client --batch --skip-column-names <<SQL
INSERT INTO customers (name, email)
VALUES ('${pagination_prefix}-${index}', NULL);
SQL
done

echo "[4/7] starting Zelyra web server"
"${zelyra_bin}" serve "${project_file}" "${address}" >"${temp_dir}/server.log" 2>&1 &
server_pid=$!
for _ in $(seq 1 30); do
    if curl --silent --show-error --fail "${base_url}${tableview_path}" -o "${temp_dir}/health.html"; then
        break
    fi
    sleep 1
done
if ! curl --silent --show-error --fail "${base_url}${tableview_path}" -o "${temp_dir}/health.html"; then
    echo "error: Zelyra web server did not become ready" >&2
    cat "${temp_dir}/server.log" >&2
    exit 1
fi
if [[ "${ZELYRA_TABLEVIEW_E2E_EXPECT_CUSTOM_APP:-0}" == "1" ]]; then
    curl --silent --show-error --fail "${base_url}/" -o "${temp_dir}/custom-home.html"
    grep -Fq "Customer order workspace" "${temp_dir}/custom-home.html"
    grep -Fq 'href="/customers"' "${temp_dir}/custom-home.html"
    grep -Fq 'href="/orders"' "${temp_dir}/custom-home.html"
fi

echo "[5/7] checking struct projection, aggregates, and escaping"
grep -Fq "<th>Orders</th>" "${temp_dir}/health.html"
grep -Fq "<th>Turnover</th>" "${temp_dir}/health.html"
grep -Fq "${alpha_name}" "${temp_dir}/health.html"
grep -Fq "${escaped_beta_name}" "${temp_dir}/health.html"
! grep -Fq "${beta_name}" "${temp_dir}/health.html"
grep -Fq "<td>2</td>" "${temp_dir}/health.html"
grep -Fq "<td>150</td>" "${temp_dir}/health.html"

curl --silent --show-error --fail --get \
    --data-urlencode "search=${beta_name}" \
    "${base_url}${tableview_path}" -o "${temp_dir}/search.html"
grep -Fq "${escaped_beta_name}" "${temp_dir}/search.html"
! grep -Fq "${alpha_name}" "${temp_dir}/search.html"
! grep -Fq "${gamma_name}" "${temp_dir}/search.html"

curl --silent --show-error --fail --get \
    --data-urlencode "filter_orders=2" \
    "${base_url}${tableview_path}" -o "${temp_dir}/filter-equal.html"
grep -Fq "${alpha_name}" "${temp_dir}/filter-equal.html"
! grep -Fq "${beta_name}" "${temp_dir}/filter-equal.html"
! grep -Fq "${gamma_name}" "${temp_dir}/filter-equal.html"

curl --silent --show-error --fail --get \
    --data-urlencode "filter_orders__operator=gte" \
    --data-urlencode "filter_orders=1" \
    "${base_url}${tableview_path}" -o "${temp_dir}/filter-range.html"
grep -Fq "${alpha_name}" "${temp_dir}/filter-range.html"
grep -Fq "${escaped_beta_name}" "${temp_dir}/filter-range.html"
! grep -Fq "${gamma_name}" "${temp_dir}/filter-range.html"

curl --silent --show-error --fail --get \
    --data-urlencode "filter_name__operator=contains" \
    --data-urlencode "filter_name=Beta" \
    "${base_url}${tableview_path}" -o "${temp_dir}/filter-text.html"
grep -Fq "${escaped_beta_name}" "${temp_dir}/filter-text.html"
! grep -Fq "${alpha_name}" "${temp_dir}/filter-text.html"

filter_status="$(curl --silent --show-error --output "${temp_dir}/invalid-filter.html" --write-out '%{http_code}' "${base_url}${tableview_path}?filter_not_allowed=value")"
[[ "${filter_status}" == "400" ]]
unsupported_filter_status="$(curl --silent --show-error --output "${temp_dir}/unsupported-filter.html" --write-out '%{http_code}' "${base_url}${tableview_path}?filter_orders__operator=contains&filter_orders=2")"
[[ "${unsupported_filter_status}" == "400" ]]

echo "[6/7] checking allowlisted sorting and pagination"
curl --silent --show-error --fail --get \
    --data-urlencode "search=${customer_prefix}" \
    --data-urlencode "sort=name" \
    --data-urlencode "order=desc" \
    "${base_url}${tableview_path}" -o "${temp_dir}/sorted.html"
gamma_offset="$(grep -b -o -m1 "${gamma_name}" "${temp_dir}/sorted.html" | cut -d: -f1)"
beta_offset="$(grep -b -o -m1 "${escaped_beta_name}" "${temp_dir}/sorted.html" | cut -d: -f1)"
alpha_offset="$(grep -b -o -m1 "${alpha_name}" "${temp_dir}/sorted.html" | cut -d: -f1)"
[[ "${gamma_offset}" -lt "${beta_offset}" && "${beta_offset}" -lt "${alpha_offset}" ]]

curl --silent --show-error --fail --get \
    --data-urlencode "search=${pagination_prefix}" \
    --data-urlencode "sort=name" \
    --data-urlencode "order=asc" \
    "${base_url}${tableview_path}?page=1" -o "${temp_dir}/page-1.html"
curl --silent --show-error --fail --get \
    --data-urlencode "search=${pagination_prefix}" \
    --data-urlencode "sort=name" \
    --data-urlencode "order=asc" \
    "${base_url}${tableview_path}?page=2" -o "${temp_dir}/page-2.html"
grep -Fq "${pagination_prefix}-01" "${temp_dir}/page-1.html"
! grep -Fq "${pagination_prefix}-26" "${temp_dir}/page-1.html"
grep -Fq "${pagination_prefix}-26" "${temp_dir}/page-2.html"
! grep -Fq "${pagination_prefix}-01" "${temp_dir}/page-2.html"
grep -Fq "page=2" "${temp_dir}/page-1.html"
grep -Fq "page=1" "${temp_dir}/page-2.html"

echo "[7/7] rejecting unknown sort columns"
sort_status="$(curl --silent --show-error --output "${temp_dir}/invalid-sort.html" --write-out '%{http_code}' "${base_url}${tableview_path}?sort=not_allowed")"
[[ "${sort_status}" == "400" ]]

echo "MariaDB tableview E2E passed"
