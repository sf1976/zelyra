#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
binary="${ZELYRA_RELEASE_BINARY:-${repo_dir}/target/release/zelyra}"
target="${ZELYRA_RELEASE_TARGET:-x86_64-unknown-linux-gnu}"
temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/zelyra-release-smoke.XXXXXX")"
server_pid=""
cleanup() {
    if [[ -n "${server_pid}" ]]; then
        kill "${server_pid}" 2>/dev/null || true
        wait "${server_pid}" 2>/dev/null || true
    fi
    rm -rf -- "${temp_dir}"
}
trap cleanup EXIT

command -v python3 >/dev/null 2>&1 || { echo "error: python3 is required" >&2; exit 1; }
command -v sha256sum >/dev/null 2>&1 || { echo "error: sha256sum is required" >&2; exit 1; }
command -v tar >/dev/null 2>&1 || { echo "error: tar is required" >&2; exit 1; }

release_tag="$(python3 "${repo_dir}/scripts/check_release_metadata.py" --print-tag)"
version="${release_tag#v}"
if [[ ! -x "${binary}" ]]; then
    echo "[1/6] building release CLI"
    cargo build --locked --release --package zelyra-cli --target "${target}"
    binary="${repo_dir}/target/${target}/release/zelyra"
else
    echo "[1/6] using existing release CLI ${binary}"
fi

python3 "${repo_dir}/scripts/check_release_metadata.py" --binary "${binary}"
echo "[2/6] packaging ${release_tag}"
python3 "${repo_dir}/scripts/package_release.py" \
    --project-root "${repo_dir}" \
    --binary "${binary}" \
    --output-dir "${temp_dir}/release" \
    --platform linux \
    --tag "${release_tag}" \
    --target "${target}" \
    --source-date-epoch "$(git -C "${repo_dir}" log -1 --format=%ct)" >/dev/null

echo "[3/6] verifying SHA-256 sidecars"
for checksum in "${temp_dir}/release"/*.sha256; do
    (cd "$(dirname -- "${checksum}")" && sha256sum --check --strict "$(basename -- "${checksum}")")
done

archive="${temp_dir}/release/zelyra-${release_tag}-${target}.tar.gz"
extract_dir="${temp_dir}/extract"
mkdir -- "${extract_dir}"
echo "[4/6] checking archive paths and executable"
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

echo "[5/6] running packaged CLI"
[[ "$("${binaries[0]}" --version)" == "zelyra ${version}" ]]
"${repo_dir}/install.sh" --release "${release_tag}" --dry-run --no-path --root "${temp_dir}/install" >/dev/null

echo "[6/6] requesting Zelyra UI assets from the packaged CLI"
cat >"${temp_dir}/ui-smoke.zyl" <<'ZELYRA'
import "zelyra/ui" as ui

page "/" {
    html {
        <!doctype html>
        <html data-z-ui>
            <head><meta charset="utf-8" /><title>Release asset check</title></head>
            <body><Button label="Release asset check" /></body>
        </html>
    }
}
ZELYRA
port="$(python3 - <<'PY'
import socket
with socket.socket() as listener:
    listener.bind(("127.0.0.1", 0))
    print(listener.getsockname()[1])
PY
)"
address="127.0.0.1:${port}"
"${binaries[0]}" serve "${temp_dir}/ui-smoke.zyl" "${address}" >"${temp_dir}/ui-smoke.log" 2>&1 &
server_pid=$!
ZELYRA_RELEASE_SMOKE_URL="http://${address}" ZELYRA_RELEASE_VERSION="${version}" python3 - <<'PY'
import os
import time
from urllib.error import URLError
from urllib.request import urlopen

base = os.environ["ZELYRA_RELEASE_SMOKE_URL"]
page = None
for _ in range(100):
    try:
        with urlopen(base, timeout=1) as response:
            page = response.read().decode("utf-8")
            break
    except (OSError, URLError):
        time.sleep(0.1)
if page is None:
    raise SystemExit("packaged Zelyra server did not become ready")

version = os.environ["ZELYRA_RELEASE_VERSION"]
css_path = f"/__zelyra/ui/{version}/zelyra-ui.css"
js_path = f"/__zelyra/ui/{version}/zelyra-ui.js"
if css_path not in page or js_path not in page:
    raise SystemExit("packaged HTML omitted its versioned Zelyra UI assets")

for path, content_type, required in (
    (css_path, "text/css", "@layer reset, base, layout, components, utilities, theme, overrides;"),
    (js_path, "text/javascript", "window.ZelyraUI"),
):
    with urlopen(base + path, timeout=3) as response:
        body = response.read().decode("utf-8")
        if response.status != 200 or content_type not in response.headers.get("Content-Type", ""):
            raise SystemExit(f"packaged asset has an invalid HTTP response: {path}")
        if required not in body:
            raise SystemExit(f"packaged asset is missing expected framework content: {path}")
        if "max-age=31536000" not in response.headers.get("Cache-Control", ""):
            raise SystemExit(f"packaged asset is missing immutable cache headers: {path}")

if "data-z-ui-stylesheet" not in page or 'class="z-button' not in page:
    raise SystemExit("packaged CLI did not render the imported UI component")
PY
echo "release artifact smoke test passed for ${release_tag}"
