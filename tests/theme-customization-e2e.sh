#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
zelyra_bin="${ZELYRA_BIN:-${repo_dir}/target/debug/zelyra}"
address="${ZELYRA_THEME_E2E_ADDRESS:-127.0.0.1:38600}"
source_file="${ZELYRA_THEME_E2E_SOURCE:-${repo_dir}/examples/theme_customization/main.zyl}"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-theme-e2e.XXXXXX")"
server_pid=""

cleanup() {
    if [[ -n "${server_pid}" ]]; then
        kill "${server_pid}" 2>/dev/null || true
        wait "${server_pid}" 2>/dev/null || true
    fi
    rm -rf -- "${temp_dir}"
}
trap cleanup EXIT

if [[ ! -x "${zelyra_bin}" ]]; then
    echo "error: Zelyra binary not found at ${zelyra_bin}; run cargo build -p zelyra-cli first" >&2
    exit 1
fi
if ! command -v curl >/dev/null 2>&1; then
    echo "error: curl is required for the theme customization E2E test" >&2
    exit 1
fi

echo "[1/2] checking the theme customization source"
check_json="$("${zelyra_bin}" check "${source_file}" --format=json)"
python3 -c 'import json, sys; assert json.load(sys.stdin)["success"] is True' <<<"${check_json}"

echo "[2/2] serving the project-local layout and theme"
env -u DATABASE_URL "${zelyra_bin}" serve "${source_file}" "${address}" \
    >"${temp_dir}/server.log" 2>&1 &
server_pid=$!
base_url="http://${address}"
for _ in $(seq 1 30); do
    if curl --silent --show-error --fail "${base_url}/workspace" \
        -o "${temp_dir}/workspace.html"; then
        break
    fi
    sleep 1
done
if ! curl --silent --show-error --fail "${base_url}/workspace" \
    -o "${temp_dir}/workspace.html"; then
    echo "error: theme customization server did not become ready" >&2
    cat "${temp_dir}/server.log" >&2
    exit 1
fi
curl --silent --show-error --fail "${base_url}/__zelyra/theme.css" \
    -o "${temp_dir}/theme.css"
grep -Fq -- "The project theme changes the shared palette and typography." "${temp_dir}/workspace.html"
grep -Fq -- "--zelyra-color-accent: #087f8c" "${temp_dir}/theme.css"
echo "Theme customization E2E passed"
