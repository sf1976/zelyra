#!/usr/bin/env bash
set -euo pipefail

# Install Zelyra for the current user. This script deliberately does not use
# sudo and never asks for an account password.
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
install_root="${ZELYRA_INSTALL_ROOT:-${HOME}/.local}"

if ! command -v cargo >/dev/null 2>&1; then
    if ! command -v curl >/dev/null 2>&1; then
        echo "error: curl is required to install the Rust toolchain" >&2
        exit 1
    fi
    echo "Rust is missing; installing the user-local stable toolchain..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    if [[ -f "${HOME}/.cargo/env" ]]; then
        # shellcheck disable=SC1091
        . "${HOME}/.cargo/env"
    fi
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo is still unavailable after Rust installation" >&2
    echo "start a new shell and run ./install.sh again" >&2
    exit 1
fi

"${CARGO_HOME:-${HOME}/.cargo}/bin/cargo" install --path "${script_dir}/cli" --root "${install_root}" --force

echo
echo "Zelyra installed to ${install_root}/bin/zelyra"
if [[ ":${PATH}:" != *":${install_root}/bin:"* ]]; then
    echo "Add ${install_root}/bin to PATH, for example:"
    echo "  export PATH=\"${install_root}/bin:\$PATH\""
fi
echo "Try: zelyra run ${script_dir}/examples/fibonacci.zyl"
