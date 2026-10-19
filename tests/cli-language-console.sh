#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
project="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-console.XXXXXX")"
trap 'rm -rf "${project}"' EXIT
cat >"${project}/main.zyl" <<'EOF'
fn main() {}
EOF

bindings="$(printf 'x = 10\ny = 20\nx + y\n' | "${zelyra_bin}" console "${project}/main.zyl" --sandbox)"
[[ "${bindings}" == *'=> 30'* ]]

recovery="$(printf '1 / 0\n"weiter"\n' | "${zelyra_bin}" console "${project}/main.zyl" --sandbox 2>&1)"
[[ "${recovery}" == *'division by zero'* ]]
[[ "${recovery}" == *'=> "weiter"'* ]]

meta="$(printf ':tenant 42\ntenant_id\n:help\n' | "${zelyra_bin}" console "${project}/main.zyl" --sandbox 2>&1)"
[[ "${meta}" == *'=> 42'* ]]
[[ "${meta}" == *':commit'* ]]
[[ "${meta}" != *'E-PARSE-001'* ]]

echo "language console pipe checks passed"
