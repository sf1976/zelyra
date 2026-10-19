#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
test_url="${ZELYRA_DB_CLI_TEST_URL:-}"
if [[ -z "${test_url}" ]]; then
  echo "error: ZELYRA_DB_CLI_TEST_URL must point to a local disposable zelyra_* database" >&2
  exit 2
fi
connection="$(python3 - "${test_url}" <<'PY'
import sys
from urllib.parse import urlsplit
u = urlsplit(sys.argv[1])
if u.scheme not in ("mysql", "mariadb") or u.hostname not in ("127.0.0.1", "localhost"):
    raise SystemExit("error: test URL must use local MariaDB/MySQL")
name = u.path.lstrip("/")
if not name.startswith("zelyra_") or not name.replace("_", "").isalnum():
    raise SystemExit("error: test database name must be a zelyra_* identifier")
print(name)
PY
)"
project="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-db-console.XXXXXX")"
cleanup() {
  python3 -c 'import shutil,sys; shutil.rmtree(sys.argv[1], ignore_errors=True)' "${project}"
}
trap cleanup EXIT

cat >"${project}/main.zyl" <<EOF
database main { engine: mariadb database: "${connection}" }
EOF
cat >"${project}/.env" <<EOF
DATABASE_URL=${test_url}
EOF

console_output="$(printf 'SELECT 1;\n' | "${zelyra_bin}" db console "${project}/main.zyl")"
[[ "${console_output}" == *"1"* ]]

# A fake client checks that passwords do not enter argv. The preceding real
# client invocation verifies interactive stdin/stdout passthrough.
fake_bin="${project}/fake-bin"
mkdir -p "${fake_bin}"
cat >"${fake_bin}/mariadb" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$@" >"${ZELYRA_TEST_ARGS}"
printf '%s' "${MYSQL_PWD-}" >"${ZELYRA_TEST_PASSWORD}"
cat >/dev/null
EOF
chmod +x "${fake_bin}/mariadb"
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

echo "db console integration checks passed"
