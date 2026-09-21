#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-clean-onboarding.XXXXXX")"
install_root="${temp_dir}/prefix"

cleanup() {
    rm -rf -- "${temp_dir}"
}
trap cleanup EXIT

cd -- "${repo_dir}"

echo "[1/2] installing Zelyra into an isolated user prefix"
./install.sh --no-rustup --no-path --root "${install_root}"
./install.sh --check --no-path --root "${install_root}"
installed_binary="${install_root}/bin/zelyra"
[[ -x "${installed_binary}" ]]

echo "[2/2] running the complete generated-project onboarding path"
ZELYRA_BIN="${installed_binary}" \
ZELYRA_GENERATED_E2E_DB_PORT="${ZELYRA_CLEAN_ONBOARDING_DB_PORT:-3308}" \
ZELYRA_GENERATED_E2E_HOST_PORT="${ZELYRA_CLEAN_ONBOARDING_HOST_PORT:-18083}" \
ZELYRA_GENERATED_E2E_GENERATED_DB_HOST_PORT="${ZELYRA_CLEAN_ONBOARDING_GENERATED_DB_HOST_PORT:-3311}" \
ZELYRA_GENERATED_E2E_ADDRESS="${ZELYRA_CLEAN_ONBOARDING_ADDRESS:-127.0.0.1:38521}" \
ZELYRA_GENERATED_E2E_ROOT_PASSWORD="${ZELYRA_CLEAN_ONBOARDING_ROOT_PASSWORD:?ZELYRA_CLEAN_ONBOARDING_ROOT_PASSWORD must be set}" \
    "${script_dir}/generated-project-mariadb-e2e.sh"

echo "Clean onboarding E2E passed"
