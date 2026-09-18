#!/usr/bin/env bash
set -Eeuo pipefail

umask 022

readonly INSTALLER_VERSION="1"
readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly DEFAULT_INSTALL_ROOT="${HOME}/.local"

install_root="${ZELYRA_INSTALL_ROOT:-${DEFAULT_INSTALL_ROOT}}"
auto_rustup=1
update_path=1
dry_run=0
offline=0
uninstall=0
check_only=0

usable_cargo() {
    local candidate
    candidate="$(command -v cargo 2>/dev/null || true)"
    [[ -n "$candidate" && -x "$candidate" ]] || return 1
    printf '%s\n' "$candidate"
}

usage() {
    cat <<'EOF'
Zelyra source installer

Usage:
  ./install.sh [options]

Builds the CLI from this checkout and installs it for the current user.
No sudo or administrator privileges are used.

Options:
  --root PATH       Install below PATH (default: ~/.local)
  --no-rustup       Fail instead of installing Rust when cargo is missing
  --no-path         Do not print PATH guidance
  --offline         Do not access the network during cargo install
  --dry-run         Show actions without changing the system
  --check           Verify an existing installation only
  --uninstall       Remove only the installed Zelyra executable
  -h, --help        Show this help
      --version     Show installer version

Environment:
  ZELYRA_INSTALL_ROOT  Same as --root PATH
  CARGO_HOME           Existing Cargo home used by cargo/rustup
EOF
}

die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

on_error() {
    local status=$?
    printf 'error: installation failed at line %s (exit %s)\n' "${BASH_LINENO[0]}" "${status}" >&2
    exit "${status}"
}
trap on_error ERR

run() {
    if ((dry_run)); then
        printf '+ '
        printf '%q ' "$@"
        printf '\n'
    else
        "$@"
    fi
}

is_absolute_path() {
    [[ "$1" == /* ]]
}

validate_root() {
    is_absolute_path "$install_root" || die "install root must be an absolute path: ${install_root}"
    [[ "$install_root" != "/" ]] || die "refusing to use / as install root"
}

installed_binary() {
    printf '%s/bin/zelyra' "$install_root"
}

parse_args() {
    while (($#)); do
        case "$1" in
            --root)
                (($# >= 2)) || die "--root requires a path"
                install_root="$2"
                shift 2
                ;;
            --root=*)
                install_root="${1#*=}"
                shift
                ;;
            --no-rustup)
                auto_rustup=0
                shift
                ;;
            --no-path)
                update_path=0
                shift
                ;;
            --offline)
                offline=1
                shift
                ;;
            --dry-run)
                dry_run=1
                shift
                ;;
            --check)
                check_only=1
                shift
                ;;
            --uninstall)
                uninstall=1
                shift
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            --version)
                printf '%s\n' "$INSTALLER_VERSION"
                exit 0
                ;;
            *)
                die "unknown option: $1 (use --help)"
                ;;
        esac
    done
}

validate_checkout() {
    [[ -f "${SCRIPT_DIR}/Cargo.toml" ]] || die "Cargo.toml not found in ${SCRIPT_DIR}"
    [[ -f "${SCRIPT_DIR}/cli/Cargo.toml" ]] || die "CLI package not found in ${SCRIPT_DIR}/cli"
}

check_installation() {
    local binary
    binary="$(installed_binary)"
    [[ -x "$binary" ]] || die "Zelyra is not installed at ${binary}"
    "$binary" --help >/dev/null || die "installed executable did not pass --help"
    printf 'ok: %s\n' "$binary"
}

remove_installation() {
    local binary
    binary="$(installed_binary)"
    if [[ ! -e "$binary" ]]; then
        printf 'already absent: %s\n' "$binary"
        return 0
    fi
    [[ -f "$binary" ]] || die "refusing to remove non-file target: ${binary}"
    if ((dry_run)); then
        printf '+ rm -- %q\n' "$binary"
    else
        rm -- "$binary"
        printf 'removed: %s\n' "$binary"
    fi
}

ensure_cargo() {
    if usable_cargo >/dev/null; then
        return 0
    fi
    ((auto_rustup)) || die "cargo is missing; install Rust or remove --no-rustup"
    ((dry_run)) && {
        printf '+ install user-local Rust with rustup\n'
        return 0
    }
    command -v curl >/dev/null 2>&1 || die "curl is required to install Rust automatically"
    local rustup_script
    rustup_script="$(mktemp)"
    trap 'rm -f -- "${rustup_script:-}"' EXIT
    curl --fail --silent --show-error --location \
        --proto '=https' --tlsv1.2 \
        https://sh.rustup.rs --output "$rustup_script"
    sh "$rustup_script" -y --profile minimal
    rm -f -- "$rustup_script"
    trap - EXIT
    if [[ -f "${CARGO_HOME:-${HOME}/.cargo}/env" ]]; then
        # shellcheck disable=SC1091
        . "${CARGO_HOME:-${HOME}/.cargo}/env"
    fi
    usable_cargo >/dev/null || die "cargo is still unavailable after Rust installation; check PATH and ~/.cargo/bin"
}

install_binary() {
    local cargo_command
    local -a cargo_arguments=(install --locked --path "${SCRIPT_DIR}/cli" --root "$install_root" --force)
    ((offline)) && cargo_arguments+=(--offline)
    if cargo_command="$(usable_cargo)"; then
        run "$cargo_command" "${cargo_arguments[@]}"
    else
        ((dry_run)) || die "cargo is unavailable"
        printf '+ cargo'
        printf ' %q' "${cargo_arguments[@]}"
        printf '\n'
    fi
}

print_path_guidance() {
    ((update_path)) || return 0
    local bin_dir="${install_root}/bin"
    case ":${PATH}:" in
        *":${bin_dir}:"*) ;;
        *)
            printf 'PATH note: %s is not currently on PATH.\n' "$bin_dir"
            printf 'For this shell: export PATH="%s:\$PATH"\n' "$bin_dir"
            printf 'For future shells, add that export to ~/.profile, ~/.bashrc, or ~/.zshrc.\n'
            ;;
    esac
}

main() {
    parse_args "$@"
    validate_root
    validate_checkout

    ((check_only && uninstall)) && die "--check and --uninstall cannot be combined"

    if ((uninstall)); then
        remove_installation
        exit 0
    fi
    if ((check_only)); then
        check_installation
        exit 0
    fi

    printf 'Zelyra installer %s\n' "$INSTALLER_VERSION"
    printf 'Source:  %s\n' "$SCRIPT_DIR"
    printf 'Target:  %s\n' "$(installed_binary)"
    if ((dry_run)); then
        printf 'Mode:    dry-run\n'
    fi
    ensure_cargo
    install_binary
    if ((dry_run)); then
        printf 'dry-run complete: no files or toolchains were changed\n'
        exit 0
    fi
    check_installation
    print_path_guidance
    printf 'Try: zelyra run %q\n' "${SCRIPT_DIR}/examples/fibonacci.zyl"
}

main "$@"
