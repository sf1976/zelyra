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

if [[ ! -x "${zelyra_bin}" ]]; then
    echo "error: Zelyra binary not found at ${zelyra_bin}; build zelyra-cli first" >&2
    exit 1
fi
if ! command -v docker >/dev/null 2>&1; then
    echo "error: docker is required for the upload cleanup E2E test" >&2
    exit 1
fi
cleanup() {
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
table upload_record {
    id: Id primary auto
    file: FileRef?
}
fn main() {}
ZELYRA
export DATABASE_URL="mariadb://root:${database_password}@127.0.0.1:${database_port}/${database_name}"
"${zelyra_bin}" db setup "${project_dir}/main.zyl"

referenced_ref="123e4567-e89b-42d3-a456-426614174000"
orphan_ref="123e4567-e89b-42d3-a456-426614174001"
docker exec --env MYSQL_PWD "${container_name}" mariadb --user=root "${database_name}" \
    --execute="INSERT INTO upload_record (file) VALUES ('${referenced_ref}')"
tenant_dir="${project_dir}/storage/zelyra/tenants/0"
mkdir -p "${tenant_dir}"
chmod 700 "${project_dir}/storage" "${project_dir}/storage/zelyra" \
    "${project_dir}/storage/zelyra/tenants" "${tenant_dir}"
for ref in "${referenced_ref}" "${orphan_ref}"; do
    printf 'test bytes\n' >"${tenant_dir}/${ref}"
    printf 'test manifest\n' >"${tenant_dir}/${ref}.meta"
    chmod 600 "${tenant_dir}/${ref}" "${tenant_dir}/${ref}.meta"
    touch -d '2 days ago' "${tenant_dir}/${ref}" "${tenant_dir}/${ref}.meta"
done

preview="$("${zelyra_bin}" files cleanup "${project_dir}/main.zyl" --min-age-days 1)"
grep -F "1 old unreferenced upload artifact(s) found" <<<"${preview}" >/dev/null
grep -F "file_ref=${orphan_ref}" <<<"${preview}" >/dev/null
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
echo "upload cleanup MariaDB E2E passed (preview, reference preservation, explicit apply)"
