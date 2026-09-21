#!/usr/bin/env bash
set -Eeuo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-installer-e2e.XXXXXX")"
install_root="${temp_dir}/prefix"

cleanup() {
    rm -rf -- "${temp_dir}"
}
trap cleanup EXIT

cd -- "${repo_dir}"

echo "[1/4] installing the source checkout into an isolated prefix"
./install.sh --no-rustup --no-path --root "${install_root}"

installed_binary="${install_root}/bin/zelyra"
[[ -x "${installed_binary}" ]]

echo "[2/4] checking the installed executable"
./install.sh --check --no-path --root "${install_root}"
version_output="$(${installed_binary} --version)"
[[ -n "${version_output}" ]]

echo "[3/4] checking repeatable installation"
./install.sh --no-rustup --no-path --root "${install_root}"
./install.sh --check --no-path --root "${install_root}"

echo "[4/4] uninstalling only the installed executable"
./install.sh --uninstall --no-path --root "${install_root}"
if ./install.sh --check --no-path --root "${install_root}"; then
    echo "error: installer check unexpectedly succeeded after uninstall" >&2
    exit 1
fi
[[ ! -e "${installed_binary}" ]]
echo "Installer E2E passed"
