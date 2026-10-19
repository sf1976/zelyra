#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
test_url="${ZELYRA_DB_CLI_POSTGRES_TEST_URL:-}"
if [[ -z "${test_url}" ]]; then
  echo "error: ZELYRA_DB_CLI_POSTGRES_TEST_URL is required" >&2
  exit 2
fi
project="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-db-postgres.XXXXXX")"
cleanup() {
  python3 -c 'import shutil,sys; shutil.rmtree(sys.argv[1], ignore_errors=True)' "${project}"
}
trap cleanup EXIT

cat >"${project}/main.zyl" <<'EOF'
database main { engine: postgres }
EOF
cat >"${project}/.env" <<EOF
DATABASE_URL=${test_url}
EOF

json_output="$("${zelyra_bin}" db query "${project}/main.zyl" "SELECT count(*) AS total FROM (VALUES (1), (2)) AS values_table(value)" --format json)"
[[ "${json_output}" == '[{"total":2}]' ]]
tenant_output="$("${zelyra_bin}" db query "${project}/main.zyl" "SELECT :tenant_id::bigint AS tenant_id" --tenant 42 --format json)"
[[ "${tenant_output}" == '[{"tenant_id":42}]' ]]
console_output="$(printf 'SELECT 1;\n' | "${zelyra_bin}" db console "${project}/main.zyl")"
[[ "${console_output}" == *"1"* ]]

fake_bin="${project}/fake-bin"
mkdir -p "${fake_bin}"
cat >"${fake_bin}/psql" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$@" >"${ZELYRA_TEST_ARGS}"
printf '%s' "${PGPASSWORD-}" >"${ZELYRA_TEST_PASSWORD}"
cat >/dev/null
EOF
chmod +x "${fake_bin}/psql"
ZELYRA_TEST_ARGS="${project}/args" ZELYRA_TEST_PASSWORD="${project}/password" PATH="${fake_bin}:${PATH}" \
  "${zelyra_bin}" db console "${project}/main.zyl" </dev/null >/dev/null
password="$(python3 - "${test_url}" <<'PY'
import sys
from urllib.parse import urlsplit, unquote
print(unquote(urlsplit(sys.argv[1]).password or ""))
PY
)"
! grep -Fq -- "${password}" "${project}/args"
[[ "$(cat "${project}/password")" == "${password}" ]]

echo "PostgreSQL database CLI integration checks passed"
