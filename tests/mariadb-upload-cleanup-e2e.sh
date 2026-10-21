#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
mariadb_image="${ZELYRA_UPLOAD_MARIADB_IMAGE:-mariadb:11.8}"
container_name="zelyra-upload-cleanup-e2e-$$-${RANDOM}"
database_name="zelyra_upload_cleanup_e2e"
database_password="zelyra_upload_cleanup_${$}_${RANDOM}"
project_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-upload-cleanup.XXXXXX")"
export MYSQL_PWD="${database_password}"
server_pid=""

if [[ ! -x "${zelyra_bin}" ]]; then
    echo "error: Zelyra binary not found at ${zelyra_bin}; build zelyra-cli first" >&2
    exit 1
fi
if ! command -v docker >/dev/null 2>&1; then
    echo "error: docker is required for the upload cleanup E2E test" >&2
    exit 1
fi
cleanup() {
    if [[ -n "${server_pid}" ]]; then
        kill -TERM "${server_pid}" >/dev/null 2>&1 || true
        wait "${server_pid}" >/dev/null 2>&1 || true
    fi
    docker stop "${container_name}" >/dev/null 2>&1 || true
    rm -rf -- "${project_dir}"
}
trap cleanup EXIT

docker run --rm --detach --name "${container_name}" \
    --env "MARIADB_ROOT_PASSWORD=${database_password}" \
    --env "MARIADB_DATABASE=${database_name}" \
    --publish 127.0.0.1::3306 "${mariadb_image}" >/dev/null
database_port="$(docker port "${container_name}" 3306/tcp | sed 's/.*://')"
for _ in $(seq 1 60); do
    if docker exec --env MYSQL_PWD "${container_name}" mariadb \
        --user=root "${database_name}" --batch --skip-column-names \
        --execute='SELECT 1' >/dev/null 2>&1; then
        break
    fi
    sleep 1
done
if ! docker exec --env MYSQL_PWD "${container_name}" mariadb \
    --user=root "${database_name}" --batch --skip-column-names \
    --execute='SELECT 1' >/dev/null 2>&1; then
    echo "error: MariaDB test container did not become ready" >&2
    exit 1
fi

cat >"${project_dir}/main.zyl" <<'ZELYRA'
database main { engine: mariadb }
table upload_record {
    id: Id primary auto
    file: FileRef?
}
form Upload -> upload_record {
    field file: File { max_size: "1KB" allowed_types: ["application/pdf"] }
    action save {
        sql { INSERT INTO upload_record (file) VALUES (:file) }
    }
}
fn main() {}
ZELYRA
export DATABASE_URL="mariadb://root:${database_password}@127.0.0.1:${database_port}/${database_name}"
"${zelyra_bin}" db setup "${project_dir}/main.zyl"

referenced_ref="123e4567-e89b-42d3-a456-426614174000"
orphan_ref="123e4567-e89b-42d3-a456-426614174001"
crash_stage_ref="123e4567-e89b-42d3-a456-426614174003"
docker exec --env MYSQL_PWD "${container_name}" mariadb --user=root "${database_name}" \
    --execute="INSERT INTO upload_record (file) VALUES ('${referenced_ref}')"
tenant_dir="${project_dir}/storage/zelyra/tenants/0"
mkdir -p "${tenant_dir}"
chmod 700 "${project_dir}/storage" "${project_dir}/storage/zelyra" \
    "${project_dir}/storage/zelyra/tenants" "${tenant_dir}"
for ref in "${referenced_ref}" "${orphan_ref}"; do
    printf '%%PDF-1.7\ncleanup test document\n' >"${tenant_dir}/${ref}"
    digest="$(sha256sum "${tenant_dir}/${ref}" | cut -d ' ' -f 1)"
    size="$(stat -c '%s' "${tenant_dir}/${ref}")"
    printf 'application/pdf\n%s\n%s\n' "${size}" "${digest}" >"${tenant_dir}/${ref}.meta"
    chmod 600 "${tenant_dir}/${ref}" "${tenant_dir}/${ref}.meta"
    touch -d '2 days ago' "${tenant_dir}/${ref}" "${tenant_dir}/${ref}.meta"
done
printf '%%PDF-1.7\nleft behind by a crashed upload request\n' >"${tenant_dir}/.${crash_stage_ref}.stage"
stage_digest="$(sha256sum "${tenant_dir}/.${crash_stage_ref}.stage" | cut -d ' ' -f 1)"
stage_size="$(stat -c '%s' "${tenant_dir}/.${crash_stage_ref}.stage")"
printf 'application/pdf\n%s\n%s\n' "${stage_size}" "${stage_digest}" >"${tenant_dir}/.${crash_stage_ref}.meta.stage"
chmod 600 "${tenant_dir}/.${crash_stage_ref}.stage" "${tenant_dir}/.${crash_stage_ref}.meta.stage"
touch -d '2 days ago' "${tenant_dir}/.${crash_stage_ref}.stage" "${tenant_dir}/.${crash_stage_ref}.meta.stage"

preview="$("${zelyra_bin}" files cleanup "${project_dir}/main.zyl" --min-age-days 1)"
grep -F "2 old unreferenced upload artifact(s) found" <<<"${preview}" >/dev/null
grep -F "file_ref=${orphan_ref}" <<<"${preview}" >/dev/null
grep -F "file_ref=${crash_stage_ref}" <<<"${preview}" >/dev/null
grep -F "kind=abandoned-staging" <<<"${preview}" >/dev/null
if grep -F "file_ref=${referenced_ref}" <<<"${preview}" >/dev/null; then
    echo "error: cleanup preview included a database-referenced upload" >&2
    exit 1
fi

"${zelyra_bin}" files cleanup "${project_dir}/main.zyl" \
    --min-age-days 1 --apply --maintenance-window >/dev/null
[[ -f "${tenant_dir}/${referenced_ref}" ]]
[[ -f "${tenant_dir}/${referenced_ref}.meta" ]]
[[ ! -e "${tenant_dir}/${orphan_ref}" ]]
[[ ! -e "${tenant_dir}/${orphan_ref}.meta" ]]
[[ ! -e "${tenant_dir}/.${crash_stage_ref}.stage" ]]
[[ ! -e "${tenant_dir}/.${crash_stage_ref}.meta.stage" ]]

# Exercise the actual process-lifetime lock acquired by an upload-enabled
# `zelyra serve`, not only the in-process lock primitive. Keep one old orphan
# around so the cleanup command must reach the destructive maintenance gate.
locked_ref="123e4567-e89b-42d3-a456-426614174002"
printf 'test bytes\n' >"${tenant_dir}/${locked_ref}"
printf 'test manifest\n' >"${tenant_dir}/${locked_ref}.meta"
chmod 600 "${tenant_dir}/${locked_ref}" "${tenant_dir}/${locked_ref}.meta"
touch -d '2 days ago' "${tenant_dir}/${locked_ref}" "${tenant_dir}/${locked_ref}.meta"
"${zelyra_bin}" serve "${project_dir}/main.zyl" 127.0.0.1:0 >"${project_dir}/server.log" 2>&1 &
server_pid=$!
server_port=""
for _ in $(seq 1 60); do
    server_port="$(sed -n 's#^Zelyra server listening on http://127\.0\.0\.1:\([0-9][0-9]*\)$#\1#p' "${project_dir}/server.log" | head -n 1)"
    if [[ -n "${server_port}" ]] && kill -0 "${server_pid}" >/dev/null 2>&1; then
        if (echo >"/dev/tcp/127.0.0.1/${server_port}") >/dev/null 2>&1; then
            break
        fi
    fi
    if ! kill -0 "${server_pid}" >/dev/null 2>&1; then
        cat "${project_dir}/server.log" >&2
        echo "error: upload-enabled Zelyra server exited before becoming ready" >&2
        exit 1
    fi
    sleep 0.25
done
if [[ -z "${server_port}" ]] || ! kill -0 "${server_pid}" >/dev/null 2>&1; then
    cat "${project_dir}/server.log" >&2
    echo "error: upload-enabled Zelyra server did not become ready" >&2
    exit 1
fi
if locked_cleanup="$("${zelyra_bin}" files cleanup "${project_dir}/main.zyl" \
    --min-age-days 1 --apply --maintenance-window 2>&1)"; then
    echo "error: cleanup unexpectedly succeeded while the upload server was running" >&2
    exit 1
fi
grep -F 'error[E-FILE-005]: cannot begin upload maintenance' <<<"${locked_cleanup}" >/dev/null
[[ -f "${tenant_dir}/${locked_ref}" ]]
[[ -f "${tenant_dir}/${locked_ref}.meta" ]]

kill -TERM "${server_pid}"
wait "${server_pid}" >/dev/null 2>&1 || true
server_pid=""
"${zelyra_bin}" files cleanup "${project_dir}/main.zyl" \
    --min-age-days 1 --apply --maintenance-window >/dev/null
[[ ! -e "${tenant_dir}/${locked_ref}" ]]
[[ ! -e "${tenant_dir}/${locked_ref}.meta" ]]

# Exercise the documented quiesced application recovery sequence: take the
# database dump and verified file snapshot while the Zelyra server is stopped,
# restore the database first, and then restore its referenced private files.
# This does not make the two independent snapshots crash-atomic.
backup_directory="${project_dir}/upload-backup"
database_backup="${project_dir}/application-database.sql"
database_backup_partial="${database_backup}.partial"
docker exec --env MYSQL_PWD "${container_name}" mariadb-dump \
    --user=root --single-transaction --skip-lock-tables --no-tablespaces \
    "${database_name}" >"${database_backup_partial}"
chmod 600 "${database_backup_partial}"
mv -- "${database_backup_partial}" "${database_backup}"
[[ -s "${database_backup}" && "$(stat -c '%a' "${database_backup}")" == "600" ]]
"${zelyra_bin}" files backup "${project_dir}/main.zyl" --output "${backup_directory}" >"${project_dir}/backup.log"
grep -F 'Backed up 1 managed upload' "${project_dir}/backup.log" >/dev/null
grep -F 'Database contents are not included' "${project_dir}/backup.log" >/dev/null
cp "${tenant_dir}/${referenced_ref}" "${project_dir}/expected-upload.pdf"
rm -rf -- "${project_dir}/storage/zelyra/tenants"
docker exec --env MYSQL_PWD "${container_name}" mariadb --user=root \
    --execute="DROP DATABASE \`${database_name}\`; CREATE DATABASE \`${database_name}\` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;"
docker exec --interactive --env MYSQL_PWD "${container_name}" mariadb \
    --user=root "${database_name}" <"${database_backup}"
restored_file_ref="$(docker exec --env MYSQL_PWD "${container_name}" mariadb \
    --user=root "${database_name}" --batch --skip-column-names \
    --execute='SELECT file FROM upload_record WHERE id=1')"
[[ "${restored_file_ref}" == "${referenced_ref}" ]]
"${zelyra_bin}" files restore "${project_dir}/main.zyl" --from "${backup_directory}" --apply >/dev/null
cmp "${project_dir}/expected-upload.pdf" "${tenant_dir}/${referenced_ref}"

corrupt_backup="${project_dir}/corrupt-upload-backup"
cp -a "${backup_directory}" "${corrupt_backup}"
printf '%%PDF-1.7\ntampered\n' >"${corrupt_backup}/tenants/0/${referenced_ref}"
rm -rf -- "${project_dir}/storage/zelyra/tenants"
if "${zelyra_bin}" files restore "${project_dir}/main.zyl" --from "${corrupt_backup}" --apply >"${project_dir}/corrupt-restore.log" 2>&1; then
    echo "error: upload restore unexpectedly accepted a checksum-corrupted backup" >&2
    exit 1
fi
grep -F 'checksum or metadata verification failed' "${project_dir}/corrupt-restore.log" >/dev/null
[[ ! -e "${project_dir}/storage/zelyra/tenants" ]]
"${zelyra_bin}" files restore "${project_dir}/main.zyl" --from "${backup_directory}" --apply >/dev/null
cmp "${project_dir}/expected-upload.pdf" "${tenant_dir}/${referenced_ref}"
echo "upload cleanup and backup MariaDB E2E passed (orphan and crash-stage cleanup, live-server lock, checksum-verified backup and restore)"
