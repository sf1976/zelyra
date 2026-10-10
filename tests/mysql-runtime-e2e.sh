#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${DATABASE_URL:-}" || "${DATABASE_URL}" != mysql://* ]]; then
  echo "DATABASE_URL must point to a disposable MySQL test database using mysql://" >&2
  exit 2
fi

output="$(cargo run --locked --quiet --package zelyra-cli -- run examples/mysql_runtime.zyl)"
if [[ "${output}" != *"MySQL parameterized query passed"* ]]; then
  echo "MySQL runtime fixture did not complete" >&2
  exit 1
fi
echo "MySQL 8.4 parameterized runtime query passed"
