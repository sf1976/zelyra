#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
database_url="${DATABASE_URL:-}"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-outbox-rollback.XXXXXX")"
suffix="$(python3 -c 'import secrets; print(secrets.token_hex(6))')"
table="outbox_rollback_${suffix}"
marker="rollback-${suffix}"
subject_prefix="ZELYRA-ROLLBACK-${suffix}"

cleanup() {
    local status=$?
    if declare -F client >/dev/null; then
        client --batch --skip-column-names -e "DELETE FROM _zelyra_outbox WHERE JSON_UNQUOTE(JSON_EXTRACT(payload, '$.subject')) LIKE '${subject_prefix}-%'" >/dev/null 2>&1 || true
        client --batch --skip-column-names -e "DROP TABLE IF EXISTS \`${table}\`" >/dev/null 2>&1 || true
    fi
    if [[ "${status}" -ne 0 && -d "${temp_dir}" ]]; then
        for log in "${temp_dir}"/*.log; do [[ -f "${log}" ]] && { echo "--- ${log} ---" >&2; cat "${log}" >&2; }; done
    fi
    python3 -c 'from pathlib import Path; import shutil,sys; shutil.rmtree(sys.argv[1], ignore_errors=True)' "${temp_dir}"
    return "${status}"
}
trap cleanup EXIT

if [[ "${database_url}" != mariadb://* && "${database_url}" != mysql://* ]]; then
    echo "error: DATABASE_URL must identify an isolated MariaDB/MySQL test database" >&2
    exit 1
fi
[[ -x "${zelyra_bin}" ]] || { echo "error: build CLI first (${zelyra_bin})" >&2; exit 1; }
for command in mariadb python3; do
    command -v "${command}" >/dev/null || { echo "error: ${command} is required" >&2; exit 1; }
done

mapfile -t parts < <(python3 - "${database_url}" <<'PY'
import sys
from urllib.parse import unquote, urlsplit

url = urlsplit(sys.argv[1])
if url.scheme not in ("mariadb", "mysql") or url.hostname not in ("127.0.0.1", "localhost"):
    raise SystemExit("error: the rollback E2E only accepts a local MariaDB/MySQL server")
database = url.path.lstrip("/")
if "test" not in database.lower() or not database.replace("_", "").isalnum():
    raise SystemExit("error: refusing a database not explicitly named as a test database")
print(url.hostname, url.port or 3306, unquote(url.username or ""), unquote(url.password or ""), database, sep="\n")
PY
)
[[ "${#parts[@]}" == 5 ]] || exit 1
db_host="${parts[0]}"
db_port="${parts[1]}"
db_user="${parts[2]}"
db_password="${parts[3]}"
db_name="${parts[4]}"
client() {
    MYSQL_PWD="${db_password}" mariadb --protocol=tcp --host="${db_host}" \
        --port="${db_port}" --user="${db_user}" --database="${db_name}" "$@"
}

project_dir="${temp_dir}/project"
mkdir -p "${project_dir}"
cat >"${project_dir}/zelyra.toml" <<'TOML'
[project]
name = "outbox-rollback-e2e"
version = "0.1.0"
zelyra = "0.1"

[capabilities]
database = true
email = true
TOML
cat >"${project_dir}/main.zyl" <<ZELYRA
database main { engine: mariadb database: "${db_name}" }

table ${table} {
    id: Id primary auto
    marker: String(64) required unique
}

struct Receipt {
    number: String
}

view ReceiptPdf {
    html { <html><body><p>{number}</p><slot /></body></html> }
}

fn fail_after_multiple_mail_writes() uses Email, Database(write) {
    marker = "${marker}"
    receipt = Receipt { number: marker }
    transaction {
        sql { INSERT INTO ${table} (marker) VALUES (:marker) }
        mail_queue("first@example.test", "${subject_prefix}-first", "first", "first.pdf", render_pdf("ReceiptPdf", receipt))
        mail_queue("second@example.test", "${subject_prefix}-second", "second", "second.pdf", render_pdf("ReceiptPdf", receipt))
        sql { INSERT INTO ${table} (marker) VALUES (:marker) }
    }
}

fn main() {}
ZELYRA

echo "[1/4] check and prepare an isolated probe table plus the outbox"
"${zelyra_bin}" check "${project_dir}/main.zyl"
DATABASE_URL="${database_url}" "${zelyra_bin}" db setup "${project_dir}/main.zyl"
DATABASE_URL="${database_url}" "${zelyra_bin}" outbox setup "${project_dir}/main.zyl"

for backend in ast mir bytecode; do
    echo "[2/4] provoke a late SQL failure after two outbox writes with ${backend}"
    if DATABASE_URL="${database_url}" "${zelyra_bin}" run \
        "${project_dir}/main.zyl" --entry fail_after_multiple_mail_writes \
        --backend "${backend}" >"${temp_dir}/${backend}.log" 2>&1; then
        echo "error: ${backend} unexpectedly committed a duplicate-key transaction" >&2
        exit 1
    fi
    grep -Eiq 'duplicate entry|duplicate key' "${temp_dir}/${backend}.log" || {
        echo "error: ${backend} failed for a reason other than the deliberate duplicate key" >&2
        exit 1
    }
done

echo "[3/4] verify all transactional writes were rolled back"
row_count="$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM \`${table}\` WHERE marker='${marker}'")"
outbox_count="$(client --batch --skip-column-names -e "SELECT COUNT(*) FROM _zelyra_outbox WHERE JSON_UNQUOTE(JSON_EXTRACT(payload, '$.subject')) LIKE '${subject_prefix}-%'")"
[[ "${row_count}" == "0" ]]
[[ "${outbox_count}" == "0" ]]

echo "[4/4] all three backends rolled back both outbox messages and the preceding row"
