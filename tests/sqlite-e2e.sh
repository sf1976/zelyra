#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
project_file="${ZELYRA_SQLITE_E2E_PROJECT:-${repo_dir}/examples/machine_management_sqlite.zyl}"

if ! command -v sqlite3 >/dev/null 2>&1; then
    echo "error: sqlite3 is required for the SQLite integration test" >&2
    exit 1
fi
if [[ ! -x "${zelyra_bin}" ]]; then
    echo "error: Zelyra binary not found at ${zelyra_bin}; run cargo build -p zelyra-cli first" >&2
    exit 1
fi

temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-sqlite-e2e.XXXXXX")"
trap 'rm -rf -- "${temp_dir}"' EXIT
database_path="${temp_dir}/machine-management.sqlite3"
database_url="sqlite://${database_path}"

echo "[1/4] bootstrapping SQLite schema"
DATABASE_URL="${database_url}" "${zelyra_bin}" db bootstrap "${project_file}"

echo "[2/4] inspecting SQLite schema"
inspect_output="$(DATABASE_URL="${database_url}" "${zelyra_bin}" db inspect "${project_file}")"
grep -Fq "2 tables" <<<"${inspect_output}"
grep -Fq "1 foreign keys" <<<"${inspect_output}"

echo "[3/4] checking an idempotent schema plan"
plan_output="$(DATABASE_URL="${database_url}" "${zelyra_bin}" db plan "${project_file}")"
grep -Fq "No schema changes." <<<"${plan_output}"

echo "[4/4] verifying SQLite metadata"
sqlite3 "${database_path}" ".tables" | grep -Fq "departments"
sqlite3 "${database_path}" "PRAGMA foreign_key_list(machines);" | grep -Fq "departments"
echo "SQLite schema E2E passed"
