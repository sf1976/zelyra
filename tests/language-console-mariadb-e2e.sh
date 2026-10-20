#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
mariadb_image="${ZELYRA_REPL_MARIADB_IMAGE:-mariadb:11.8}"
container_name="zelyra-repl-e2e-$$-${RANDOM}"
database_name="zelyra_repl_e2e"
database_password="zelyra_repl_${$}_${RANDOM}"
project_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-repl-e2e.XXXXXX")"

if [[ ! -x "${zelyra_bin}" ]]; then
    echo "error: Zelyra binary not found at ${zelyra_bin}; run cargo build -p zelyra-cli first" >&2
    exit 1
fi
for command in docker; do
    if ! command -v "${command}" >/dev/null 2>&1; then
        echo "error: ${command} is required for the REPL MariaDB E2E test" >&2
        exit 1
    fi
done

cleanup() {
    docker stop "${container_name}" >/dev/null 2>&1 || true
    rm -r -- "${project_dir}"
}
trap cleanup EXIT

docker run --rm --detach --name "${container_name}" \
    --env "MARIADB_ROOT_PASSWORD=${database_password}" \
    --env "MARIADB_DATABASE=${database_name}" \
    --publish 127.0.0.1::3306 "${mariadb_image}" >/dev/null
database_port="$(docker port "${container_name}" 3306/tcp | sed 's/.*://')"
cat >"${project_dir}/mariadb-client.cnf" <<EOF
[client]
user=root
password=${database_password}
database=${database_name}
EOF
chmod 600 "${project_dir}/mariadb-client.cnf"
docker cp "${project_dir}/mariadb-client.cnf" \
    "${container_name}:/tmp/zelyra-repl-client.cnf" >/dev/null
for _ in $(seq 1 60); do
    if docker exec "${container_name}" mariadb \
        --defaults-extra-file=/tmp/zelyra-repl-client.cnf \
        --batch --skip-column-names \
        --execute='SELECT 1' >/dev/null 2>&1; then
        break
    fi
    sleep 1
done
if ! docker exec "${container_name}" mariadb \
    --defaults-extra-file=/tmp/zelyra-repl-client.cnf \
    --batch --skip-column-names \
    --execute='SELECT 1' >/dev/null 2>&1; then
    echo "error: MariaDB test container did not become ready" >&2
    exit 1
fi

cat >"${project_dir}/main.zyl" <<'EOF'
table console_probe {
    id: Id primary auto
    name: String(100) required
}
fn main() {}
EOF
cat >"${project_dir}/zelyra.toml" <<'EOF'
[capabilities]
database = true
EOF
database_url="mariadb://root:${database_password}@127.0.0.1:${database_port}/${database_name}"
export DATABASE_URL="${database_url}"
"${zelyra_bin}" db setup "${project_dir}/main.zyl"

count_rows() {
    docker exec "${container_name}" mariadb \
        --defaults-extra-file=/tmp/zelyra-repl-client.cnf \
        --batch --skip-column-names \
        --execute='SELECT COUNT(*) FROM console_probe'
}
run_console() {
    local mode="$1"
    local input="$2"
    printf '%b' "${input}" | "${zelyra_bin}" console "${project_dir}/main.zyl" "${mode}"
}

run_console --sandbox "sql { INSERT INTO console_probe (name) VALUES ('implicit rollback') }\n"
[[ "$(count_rows)" == 0 ]]

run_console --sandbox "sql { INSERT INTO console_probe (name) VALUES ('explicit rollback') }\n:rollback\n"
[[ "$(count_rows)" == 0 ]]

run_console --sandbox \
    "sql { INSERT INTO console_probe (name) VALUES ('committed batch') }\n:commit\nsql { INSERT INTO console_probe (name) VALUES ('rolled back batch') }\n"
[[ "$(count_rows)" == 1 ]]

run_console --persist "sql { INSERT INTO console_probe (name) VALUES ('persistent') }\n"
[[ "$(count_rows)" == 2 ]]

echo "language console MariaDB sandbox E2E passed (rollback, commit, persist)"
