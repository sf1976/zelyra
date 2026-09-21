#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
binary="${ZELYRA_RELEASE_BINARY:-${repo_dir}/target/release/zelyra}"
target="${ZELYRA_RELEASE_TARGET:-x86_64-unknown-linux-gnu}"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-release-smoke.XXXXXX")"
trap 'rm -rf -- "${temp_dir}"' EXIT

command -v python3 >/dev/null 2>&1 || { echo "error: python3 is required" >&2; exit 1; }
command -v sha256sum >/dev/null 2>&1 || { echo "error: sha256sum is required" >&2; exit 1; }
command -v tar >/dev/null 2>&1 || { echo "error: tar is required" >&2; exit 1; }

release_tag="$(python3 "${repo_dir}/scripts/check_release_metadata.py" --print-tag)"
version="${release_tag#v}"
if [[ ! -x "${binary}" ]]; then
    echo "[1/5] building release CLI"
    cargo build --locked --release --package zelyra-cli --target "${target}"
    binary="${repo_dir}/target/${target}/release/zelyra"
else
    echo "[1/5] using existing release CLI ${binary}"
fi

python3 "${repo_dir}/scripts/check_release_metadata.py" --binary "${binary}"
echo "[2/5] packaging ${release_tag}"
python3 "${repo_dir}/scripts/package_release.py" \
    --project-root "${repo_dir}" \
    --binary "${binary}" \
    --output-dir "${temp_dir}/release" \
    --platform linux \
    --tag "${release_tag}" \
    --target "${target}" \
    --source-date-epoch "$(git -C "${repo_dir}" log -1 --format=%ct)" >/dev/null

echo "[3/5] verifying SHA-256 sidecars"
for checksum in "${temp_dir}/release"/*.sha256; do
    (cd "$(dirname -- "${checksum}")" && sha256sum --check --strict "$(basename -- "${checksum}")")
done

archive="${temp_dir}/release/zelyra-${release_tag}-${target}.tar.gz"
extract_dir="${temp_dir}/extract"
mkdir -- "${extract_dir}"
echo "[4/5] checking archive paths and executable"
while IFS= read -r entry; do
    [[ "${entry}" != /* && ! "${entry}" =~ (^|/)\.\.(\/|$) ]] || {
        echo "error: unsafe archive path: ${entry}" >&2
        exit 1
    }
done < <(tar -tzf "${archive}")
tar -xzf "${archive}" -C "${extract_dir}"
mapfile -t binaries < <(find "${extract_dir}" -type f -name zelyra -perm -u+x -print)
[[ "${#binaries[@]}" -eq 1 ]] || {
    echo "error: expected one executable in release archive" >&2
    exit 1
}

echo "[5/5] running packaged CLI"
[[ "$("${binaries[0]}" --version)" == "zelyra ${version}" ]]
"${repo_dir}/install.sh" --release "${release_tag}" --dry-run --no-path --root "${temp_dir}/install" >/dev/null
echo "release artifact smoke test passed for ${release_tag}"
