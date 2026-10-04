#!/usr/bin/env bash
set -euo pipefail

database_url="${ZELYRA_BACKUP_RESTORE_MARIADB_URL:-${ZELYRA_SCHEMA_SAFETY_MARIADB_URL:-}}"
if [[ -z "${database_url}" ]]; then
    echo "error: set ZELYRA_BACKUP_RESTORE_MARIADB_URL to a local zelyra_ci or zelyra_test database" >&2
    exit 1
fi
if [[ "${database_url}" == mariadb://* ]]; then
    url_parts="${database_url#mariadb://}"
elif [[ "${database_url}" == mysql://* ]]; then
    url_parts="${database_url#mysql://}"
else
    echo "error: backup/restore test URL must use mariadb:// or mysql://" >&2
    exit 1
fi

authority="${url_parts%%/*}"
base_database="${url_parts#*/}"
credentials="${authority%@*}"
host_port="${authority#*@}"
if [[ "${credentials}" == "${authority}" || "${base_database}" == "${url_parts}" ]]; then
    echo "error: backup/restore test URL must include credentials and a database" >&2
    exit 1
fi
db_user="${credentials%%:*}"
db_password="${credentials#*:}"
db_host="${host_port%%:*}"
if [[ "${host_port}" == *:* ]]; then
    db_port="${host_port##*:}"
else
    db_port="3306"
fi

if [[ "${db_host}" != "127.0.0.1" && "${db_host}" != "localhost" ]] || \
    [[ "${base_database}" != "zelyra_ci" && "${base_database}" != "zelyra_test" ]] || \
    [[ ! "${db_port}" =~ ^[0-9]{1,5}$ ]] || ((db_port < 1 || db_port > 65535)); then
    echo "error: test is restricted to 127.0.0.1/localhost and a zelyra_ci or zelyra_test base database" >&2
    exit 1
fi
if ! command -v mariadb >/dev/null 2>&1 || ! command -v mariadb-dump >/dev/null 2>&1; then
    echo "error: mariadb and mariadb-dump clients are required" >&2
    exit 1
fi

suffix="$(date -u +%Y%m%d%H%M%S)_$$"
source_database="zelyra_backup_source_${suffix}"
restore_database="zelyra_backup_restore_${suffix}"
backup_user="zelyra_backup_${suffix}"
restore_user="zelyra_restore_${suffix}"
backup_password="dump-${suffix}-test"
restore_password="restore-${suffix}-test"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-backup-restore.XXXXXX")"
backup_file="${temp_dir}/${source_database}.sql"
temporary_file="${backup_file}.partial"

cleanup() {
    MYSQL_PWD="${db_password}" mariadb \
        --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" \
        --batch --skip-column-names >/dev/null 2>&1 <<SQL || true
DROP DATABASE IF EXISTS \`${restore_database}\`;
DROP DATABASE IF EXISTS \`${source_database}\`;
DROP USER IF EXISTS '${restore_user}'@'%';
DROP USER IF EXISTS '${backup_user}'@'%';
SQL
    rm -rf -- "${temp_dir}"
}
trap cleanup EXIT

MYSQL_PWD="${db_password}" mariadb \
    --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${db_user}" \
    --batch --skip-column-names >/dev/null <<SQL
CREATE DATABASE \`${source_database}\` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE DATABASE \`${restore_database}\` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE USER '${backup_user}'@'%' IDENTIFIED BY '${backup_password}';
GRANT SELECT, SHOW VIEW, TRIGGER ON \`${source_database}\`.* TO '${backup_user}'@'%';
CREATE USER '${restore_user}'@'%' IDENTIFIED BY '${restore_password}';
GRANT ALL PRIVILEGES ON \`${restore_database}\`.* TO '${restore_user}'@'%';
CREATE TABLE \`${source_database}\`.restore_probe (
    id INT PRIMARY KEY,
    label VARCHAR(100) NOT NULL,
    payload MEDIUMTEXT NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
INSERT INTO \`${source_database}\`.restore_probe VALUES (1, 'backup restore test', REPEAT('x', 100000));
SQL

echo "[MariaDB backup/restore] dump using a source-scoped read-only account"
(
    set -eu
    trap 'rm -f -- "${temporary_file}"' EXIT
    MYSQL_PWD="${backup_password}" mariadb-dump \
        --protocol=tcp \
        --host="${db_host}" \
        --port="${db_port}" \
        --user="${backup_user}" \
        --single-transaction \
        --skip-lock-tables \
        --no-tablespaces \
        "${source_database}" > "${temporary_file}"
    chmod 600 "${temporary_file}"
    mv -- "${temporary_file}" "${backup_file}"
)
[[ -s "${backup_file}" && ! -e "${temporary_file}" ]]
[[ "$(stat -c '%a' "${backup_file}")" == "600" ]]

echo "[MariaDB backup/restore] failed publication cannot leave a final-looking backup"
failed_directory="${temp_dir}/missing-output-directory"
failed_backup="${failed_directory}/failed-publication.sql"
failed_partial="${temp_dir}/failed-publication.sql.partial"
if (
    set -eu
    trap 'rm -f -- "${failed_partial}"' EXIT
    MYSQL_PWD="${backup_password}" mariadb-dump \
        --protocol=tcp \
        --host="${db_host}" \
        --port="${db_port}" \
        --user="${backup_user}" \
        --single-transaction \
        --skip-lock-tables \
        --no-tablespaces \
        "${source_database}" > "${failed_partial}"
    chmod 600 "${failed_partial}"
    mv -- "${failed_partial}" "${failed_backup}"
) >/dev/null 2>&1; then
    echo "error: publication into a missing directory unexpectedly succeeded" >&2
    exit 1
fi
[[ ! -e "${failed_backup}" && ! -e "${failed_partial}" ]]

echo "[MariaDB backup/restore] restore using an account scoped to the target database"
MYSQL_PWD="${restore_password}" mariadb \
    --protocol=tcp \
    --host="${db_host}" \
    --port="${db_port}" \
    --user="${restore_user}" \
    --database="${restore_database}" < "${backup_file}"

restored_row="$(MYSQL_PWD="${restore_password}" mariadb \
    --protocol=tcp \
    --host="${db_host}" \
    --port="${db_port}" \
    --user="${restore_user}" \
    --database="${restore_database}" \
    --batch --skip-column-names \
    --execute='SELECT CONCAT(id, ":", label, ":", CHAR_LENGTH(payload)) FROM restore_probe')"
[[ "${restored_row}" == "1:backup restore test:100000" ]]
if MYSQL_PWD="${restore_password}" mariadb \
    --protocol=tcp --host="${db_host}" --port="${db_port}" --user="${restore_user}" \
    --database="${source_database}" --execute='SELECT 1' >/dev/null 2>&1; then
    echo "error: restore account unexpectedly accessed the source database" >&2
    exit 1
fi
echo "[MariaDB backup/restore] disposable restore and least-scope checks passed"
