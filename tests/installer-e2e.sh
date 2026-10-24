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

echo "[1/6] rejecting malformed release tags before network access"
if ./install.sh --release "release/0.3.0" --dry-run --no-path --root "${install_root}"; then
    echo "error: installer accepted a malformed release tag" >&2
    exit 1
fi
./install.sh --release "v0.3.0-rc.1" --dry-run --no-path --root "${install_root}" >/dev/null

echo "[2/6] installing the source checkout into an isolated prefix"
./install.sh --no-rustup --no-path --root "${install_root}"

installed_binary="${install_root}/bin/zelyra"
[[ -x "${installed_binary}" ]]

echo "[3/6] checking the installed executable"
./install.sh --check --no-path --root "${install_root}"
version_output="$(${installed_binary} --version)"
[[ -n "${version_output}" ]]

echo "[4/6] checking repeatable installation"
./install.sh --no-rustup --no-path --root "${install_root}"
./install.sh --check --no-path --root "${install_root}"

echo "[5/6] uninstalling only the installed executable"
./install.sh --uninstall --no-path --root "${install_root}"
if ./install.sh --check --no-path --root "${install_root}"; then
    echo "error: installer check unexpectedly succeeded after uninstall" >&2
    exit 1
fi
[[ ! -e "${installed_binary}" ]]
echo "[6/6] persisting the Cargo PATH in the user's login profile"
path_home="${temp_dir}/home with spaces"
cargo_bin="${path_home}/.cargo/bin"
mkdir -p "${cargo_bin}"
cat >"${cargo_bin}/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "${1:-}" == "install" ]] || exit 2
shift
root=""
while (($#)); do
    if [[ "$1" == "--root" ]]; then
        root="$2"
        shift 2
    else
        shift
    fi
done
[[ -n "$root" ]]
mkdir -p "${root}/bin"
cat >"${root}/bin/zelyra" <<'BINARY'
#!/bin/sh
exit 0
BINARY
chmod +x "${root}/bin/zelyra"
EOF
chmod +x "${cargo_bin}/cargo"
HOME="${path_home}" SHELL="/bin/bash" CARGO_HOME="${path_home}/.cargo" PATH="${cargo_bin}:/usr/bin:/bin" \
    ./install.sh --root "${temp_dir}/path-prefix" >/dev/null
grep -Fq "${cargo_bin}" "${path_home}/.profile"
resolved_cargo="$(HOME="${path_home}" PATH="/usr/bin:/bin" bash --noprofile --norc -c '. "$HOME/.profile"; command -v cargo')"
[[ "${resolved_cargo}" == "${cargo_bin}/cargo" ]]
printf '# Existing Bash login profile\n' >"${path_home}/.bash_profile"
HOME="${path_home}" SHELL="/bin/bash" CARGO_HOME="${path_home}/.cargo" PATH="${cargo_bin}:/usr/bin:/bin" \
    ./install.sh --root "${temp_dir}/bash-profile-prefix" >/dev/null
grep -Fq "${cargo_bin}" "${path_home}/.bash_profile"
resolved_cargo="$(HOME="${path_home}" PATH="/usr/bin:/bin" bash --noprofile --norc -c '. "$HOME/.bash_profile"; command -v cargo')"
[[ "${resolved_cargo}" == "${cargo_bin}/cargo" ]]
echo "Installer E2E passed"
