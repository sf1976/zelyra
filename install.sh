#!/usr/bin/env bash
set -Eeuo pipefail

umask 022

readonly INSTALLER_VERSION="1"
readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly DEFAULT_INSTALL_ROOT="${HOME}/.local"

install_root="${ZELYRA_INSTALL_ROOT:-${DEFAULT_INSTALL_ROOT}}"
release_tag=""
auto_rustup=1
update_path=1
dry_run=0
offline=0
uninstall=0
check_only=0

usable_cargo() {
    local candidate
    hash -r 2>/dev/null || true
    candidate="$(command -v cargo 2>/dev/null || true)"
    [[ -n "$candidate" && -x "$candidate" ]] || return 1
    printf '%s\n' "$candidate"
}

usage() {
    cat <<'EOF'
Zelyra source installer

Usage:
  ./install.sh [options]

Builds the CLI from this checkout, or installs a published release, for the
current user. No sudo or administrator privileges are used.

Options:
  --root PATH       Install below PATH (default: ~/.local)
  --release TAG     Install a published Linux x86_64 release without Rust
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
            --release)
                (($# >= 2)) || die "--release requires a tag"
                release_tag="$2"
                shift 2
                ;;
            --release=*)
                release_tag="${1#*=}"
                [[ -n "$release_tag" ]] || die "--release requires a tag"
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

validate_release_tag() {
    [[ -n "$release_tag" ]] || return 0
    [[ "$release_tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] ||
        die "release tag must use vMAJOR.MINOR.PATCH with an optional prerelease suffix: ${release_tag}"
}

release_version() {
    local version="${release_tag#v}"
    printf '%s\n' "${version%%-*}"
}

release_target() {
    local system machine
    system="$(uname -s)"
    machine="$(uname -m)"
    if [[ "$system" == "Linux" && ( "$machine" == "x86_64" || "$machine" == "amd64" ) ]]; then
        printf '%s\n' 'x86_64-unknown-linux-gnu'
        return 0
    fi
    die "prebuilt releases currently support Linux x86_64 only; use the source installer on ${system}/${machine}"
}

verify_checksum() {
    local checksum_file="$1"
    if command -v sha256sum >/dev/null 2>&1; then
        (cd -- "$(dirname -- "$checksum_file")" && sha256sum --check --strict "$(basename -- "$checksum_file")")
    elif command -v shasum >/dev/null 2>&1; then
        (cd -- "$(dirname -- "$checksum_file")" && shasum -a 256 --check "$(basename -- "$checksum_file")")
    else
        die "sha256sum or shasum is required to verify release downloads"
    fi
}

install_release() {
    local target archive_name checksum_name base_url temp_dir archive checksum extract_dir
    local binary_count binary temp_binary reported_version expected_version
    local -a binaries=()
    target="$(release_target)"
    archive_name="zelyra-${release_tag}-${target}.tar.gz"
    checksum_name="${archive_name}.sha256"
    base_url="https://github.com/sf1976/zelyra/releases/download/${release_tag}"

    if ((dry_run)); then
        printf '+ download %s/%s\n' "$base_url" "$archive_name"
        printf '+ verify SHA-256 with %s\n' "${checksum_name}"
        printf '+ install verified %s to %s\n' "$target" "$(installed_binary)"
        return 0
    fi

    command -v curl >/dev/null 2>&1 || die "curl is required for release installation"
    command -v tar >/dev/null 2>&1 || die "tar is required for release installation"

    temp_dir="$(mktemp -d)"
    trap 'rm -rf -- "${temp_dir:-}"' EXIT
    archive="${temp_dir}/${archive_name}"
    checksum="${temp_dir}/${checksum_name}"
    extract_dir="${temp_dir}/extract"
    mkdir -- "$extract_dir"

    curl --fail --silent --show-error --location \
        --proto '=https' --tlsv1.2 \
        "${base_url}/${archive_name}" --output "$archive"
    curl --fail --silent --show-error --location \
        --proto '=https' --tlsv1.2 \
        "${base_url}/${checksum_name}" --output "$checksum"
    verify_checksum "$checksum"

    while IFS= read -r entry; do
        [[ "$entry" != /* && ! "$entry" =~ (^|/)\.\.(\/|$) ]] ||
            die "release archive contains an unsafe path: ${entry}"
    done < <(tar -tzf "$archive")
    tar -xzf "$archive" -C "$extract_dir"

    mapfile -t binaries < <(find "$extract_dir" -type f -name zelyra -perm -u+x -print)
    binary_count="${#binaries[@]}"
    [[ "$binary_count" -eq 1 ]] || die "release archive must contain exactly one executable named zelyra (found ${binary_count})"
    binary="${binaries[0]}"

    expected_version="zelyra $(release_version)"
    reported_version="$("$binary" --version)" ||
        die "release binary failed its --version check"
    [[ "$reported_version" == "$expected_version" ]] ||
        die "release binary reports ${reported_version@Q}; expected ${expected_version@Q}"

    mkdir -p "${install_root}/bin"
    temp_binary="${install_root}/bin/.zelyra.tmp.$$"
    cp -- "$binary" "$temp_binary"
    chmod 0755 "$temp_binary"
    mv -f -- "$temp_binary" "$(installed_binary)"
    trap - EXIT
    rm -rf -- "$temp_dir"
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
    validate_release_tag

    ((check_only && uninstall)) && die "--check and --uninstall cannot be combined"
    [[ -z "$release_tag" || "$offline" -eq 0 ]] || die "--offline cannot be combined with --release"
    [[ -z "$release_tag" || ( "$check_only" -eq 0 && "$uninstall" -eq 0 ) ]] ||
        die "--release is only valid for an installation"

    if ((uninstall)); then
        remove_installation
        exit 0
    fi
    if ((check_only)); then
        check_installation
        exit 0
    fi

    if [[ -z "$release_tag" ]]; then
        validate_checkout
    fi

    printf 'Zelyra installer %s\n' "$INSTALLER_VERSION"
    if [[ -n "$release_tag" ]]; then
        printf 'Release: %s\n' "$release_tag"
    else
        printf 'Source:  %s\n' "$SCRIPT_DIR"
    fi
    printf 'Target:  %s\n' "$(installed_binary)"
    if ((dry_run)); then
        printf 'Mode:    dry-run\n'
    fi
    if [[ -n "$release_tag" ]]; then
        install_release
    else
        ensure_cargo
        install_binary
    fi
    if ((dry_run)); then
        printf 'dry-run complete: no files or toolchains were changed\n'
        exit 0
    fi
    check_installation
    print_path_guidance
    if [[ -f "${SCRIPT_DIR}/examples/fibonacci.zyl" ]]; then
        printf 'Try: zelyra run %q\n' "${SCRIPT_DIR}/examples/fibonacci.zyl"
    else
        printf 'Try: zelyra --help\n'
    fi
}

main "$@"
