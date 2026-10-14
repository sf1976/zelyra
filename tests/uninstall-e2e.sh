#!/usr/bin/env bash
set -euo pipefail

binary="${ZELYRA_BIN:-target/debug/zelyra}"
if [[ ! -x "${binary}" ]]; then
  echo "ZELYRA_BIN must name the built Zelyra CLI" >&2
  exit 2
fi

temporary_root="$(mktemp -d)"
trap 'rm -rf "${temporary_root}"' EXIT
mkdir -p "${temporary_root}/active" "${temporary_root}/other" "${temporary_root}/project"
cp "${binary}" "${temporary_root}/active/zelyra"
cp "${binary}" "${temporary_root}/other/zelyra"
printf 'keep\n' >"${temporary_root}/project/data.txt"

preview="$(PATH="${temporary_root}/other:${PATH}" "${temporary_root}/active/zelyra" uninstall --dry-run)"
[[ "${preview}" == *"${temporary_root}/active/zelyra"* ]]
[[ "${preview}" == *"${temporary_root}/other/zelyra"* ]]
[[ -x "${temporary_root}/active/zelyra" ]]

set +e
PATH="${temporary_root}/other:${PATH}" "${temporary_root}/active/zelyra" uninstall </dev/null >"${temporary_root}/no-confirm.log" 2>&1
status=$?
set -e
[[ "${status}" -eq 2 ]]
[[ -x "${temporary_root}/active/zelyra" ]]

PATH="${temporary_root}/other:${PATH}" "${temporary_root}/active/zelyra" uninstall --yes >"${temporary_root}/uninstall.log"
[[ ! -e "${temporary_root}/active/zelyra" ]]
[[ -x "${temporary_root}/other/zelyra" ]]
[[ "$(cat "${temporary_root}/project/data.txt")" == "keep" ]]
echo "uninstall preview, explicit confirmation, path detection, and data preservation passed"
