#!/usr/bin/env bash
set -Eeuo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
release_tag="${1:-}"

if [[ -z "${release_tag}" ]]; then
    echo "usage: $0 vMAJOR.MINOR.PATCH" >&2
    exit 2
fi
[[ "${release_tag}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?$ ]] || {
    echo "error: invalid release tag: ${release_tag}" >&2
    exit 2
}
command -v curl >/dev/null 2>&1 || { echo "error: curl is required" >&2; exit 1; }
command -v tar >/dev/null 2>&1 || { echo "error: tar is required" >&2; exit 1; }

temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-published-smoke.XXXXXX")"
trap 'rm -rf -- "${temp_dir:-}"' EXIT
install_root="${temp_dir}/install"
binary="${install_root}/bin/zelyra"
expected_version="zelyra ${release_tag#v}"

echo "[1/4] installing published ${release_tag}"
"${repo_dir}/install.sh" \
    --release "${release_tag}" \
    --root "${install_root}" \
    --no-path

echo "[2/4] checking installed release"
[[ -x "${binary}" ]] || { echo "error: installed binary is missing" >&2; exit 1; }
[[ "$("${binary}" --version)" == "${expected_version}" ]] || {
    echo "error: installed binary reports an unexpected version" >&2
    exit 1
}
"${repo_dir}/install.sh" --check --root "${install_root}" --no-path

echo "[3/4] repeating published installation"
"${repo_dir}/install.sh" \
    --release "${release_tag}" \
    --root "${install_root}" \
    --no-path >/dev/null
[[ "$("${binary}" --version)" == "${expected_version}" ]]

echo "[4/4] checking published update path"
update_output="$("${binary}" update --check)"
grep -Eq 'already up to date|Update available:|newer than the latest stable release' <<<"${update_output}" || {
    echo "error: update check returned an unexpected result:" >&2
    printf '%s\n' "${update_output}" >&2
    exit 1
}
printf '%s\n' "${update_output}"
echo "published release smoke test passed for ${release_tag}"
