#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
mariadb_image="${ZELYRA_UPLOAD_MARIADB_IMAGE:-mariadb:11.8}"
container_name="zelyra-upload-e2e-$$-${RANDOM}"
database_name="zelyra_upload_e2e"
database_password="zelyra_upload_${$}_${RANDOM}"
export MARIADB_ROOT_PASSWORD="${database_password}"
export MYSQL_PWD="${database_password}"

if ! command -v docker >/dev/null 2>&1; then
    echo "error: docker is required for the isolated upload E2E test" >&2
    exit 1
fi

cleanup() {
    docker stop "${container_name}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run --rm --detach --name "${container_name}" \
    --env MARIADB_ROOT_PASSWORD \
    --env "MARIADB_DATABASE=${database_name}" \
    --publish 127.0.0.1::3306 "${mariadb_image}" >/dev/null
database_port="$(docker port "${container_name}" 3306/tcp | sed 's/.*://')"
for _ in $(seq 1 60); do
    if docker exec --env MYSQL_PWD "${container_name}" mariadb \
        --user=root "${database_name}" \
        --batch --skip-column-names --execute='SELECT 1' >/dev/null 2>&1; then
        break
    fi
    sleep 1
done
if ! docker exec --env MYSQL_PWD "${container_name}" mariadb \
    --user=root "${database_name}" \
    --batch --skip-column-names --execute='SELECT 1' >/dev/null 2>&1; then
    echo "error: MariaDB upload test container did not become ready" >&2
    exit 1
fi

ZELYRA_UPLOAD_TEST_URL="mariadb://root:${database_password}@127.0.0.1:${database_port}/${database_name}" \
    cargo test --manifest-path "${repo_dir}/Cargo.toml" -p zelyra-web --lib --locked \
    multipart_upload_commits_reference_and_removes_file_after_sql_rollback \
    -- --ignored --nocapture
