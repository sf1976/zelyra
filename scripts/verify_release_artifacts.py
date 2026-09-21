#!/usr/bin/env python3
"""Verify the files produced by ``scripts/package_release.py``."""

from __future__ import annotations

import argparse
import hashlib
import re
import subprocess
import tarfile
import zipfile
from pathlib import Path

from package_release import RELEASE_FILES, TARGETS


def _expected_names(platform: str, tag: str, target: str) -> tuple[str, str, str, str]:
    if platform not in TARGETS:
        raise ValueError(f"unsupported platform: {platform}")
    if target != TARGETS[platform]:
        raise ValueError(f"target {target} does not match platform {platform}")
    if not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?", tag):
        raise ValueError(f"invalid release tag: {tag}")
    package = f"zelyra-{tag}-{target}"
    standalone = f"{package}.{'exe' if platform == 'windows' else 'bin'}"
    archive = f"{package}.{'zip' if platform == 'windows' else 'tar.gz'}"
    return standalone, f"{standalone}.sha256", archive, f"{archive}.sha256"


def _stable_version_from_tag(tag: str) -> str:
    return tag[1:].split("-", 1)[0]


def _verify_checksum(path: Path) -> None:
    sidecar = path.with_name(path.name + ".sha256")
    if not sidecar.is_file():
        raise ValueError(f"missing checksum sidecar: {sidecar.name}")
    fields = sidecar.read_text(encoding="ascii").strip().split()
    if len(fields) != 2 or fields[1] != path.name or not re.fullmatch(r"[0-9a-f]{64}", fields[0]):
        raise ValueError(f"invalid checksum sidecar: {sidecar.name}")
    if fields[0] != hashlib.sha256(path.read_bytes()).hexdigest():
        raise ValueError(f"checksum mismatch: {path.name}")


def _expected_archive_entries(package: str, executable: str) -> set[str]:
    return {f"{package}/{executable}", *(f"{package}/{name}" for name in RELEASE_FILES)}


def _verify_archive(path: Path, platform: str, package: str) -> None:
    executable = "zelyra.exe" if platform == "windows" else "zelyra"
    expected = _expected_archive_entries(package, executable)
    if platform == "windows":
        with zipfile.ZipFile(path) as archive:
            entries = archive.infolist()
            names = {entry.filename for entry in entries}
            if names != expected:
                raise ValueError(f"unexpected ZIP entries: {sorted(names)}")
            for entry in entries:
                if entry.filename.startswith("/") or ".." in Path(entry.filename).parts:
                    raise ValueError(f"unsafe ZIP path: {entry.filename}")
                if entry.is_dir() or (entry.external_attr >> 16) & 0o170000 != 0o100000:
                    raise ValueError(f"non-regular ZIP entry: {entry.filename}")
    else:
        with tarfile.open(path, "r:gz") as archive:
            entries = archive.getmembers()
            names = {entry.name for entry in entries}
            if names != expected:
                raise ValueError(f"unexpected TAR entries: {sorted(names)}")
            for entry in entries:
                if entry.name.startswith("/") or ".." in Path(entry.name).parts:
                    raise ValueError(f"unsafe TAR path: {entry.name}")
                if not entry.isfile():
                    raise ValueError(f"non-regular TAR entry: {entry.name}")


def verify_release_artifacts(
    *, directory: Path, platform: str, tag: str, target: str, run_binary: bool = False
) -> None:
    standalone_name, standalone_checksum, archive_name, archive_checksum = _expected_names(
        platform, tag, target
    )
    expected_files = {standalone_name, standalone_checksum, archive_name, archive_checksum}
    actual_files = {path.name for path in directory.iterdir() if path.is_file()}
    if actual_files != expected_files:
        raise ValueError(f"unexpected release files: {sorted(actual_files)}")

    standalone = directory / standalone_name
    archive = directory / archive_name
    _verify_checksum(standalone)
    _verify_checksum(archive)
    _verify_archive(archive, platform, f"zelyra-{tag}-{target}")

    if run_binary:
        result = subprocess.run(
            [str(standalone), "--version"], capture_output=True, text=True, check=False
        )
        expected_version = f"zelyra {_stable_version_from_tag(tag)}"
        if result.returncode != 0 or result.stdout.strip() != expected_version:
            raise ValueError(
                f"{standalone.name} --version did not return {expected_version!r}"
            )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--platform", choices=sorted(TARGETS), required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--run-binary", action="store_true")
    args = parser.parse_args()
    try:
        verify_release_artifacts(
            directory=args.directory,
            platform=args.platform,
            tag=args.tag,
            target=args.target,
            run_binary=args.run_binary,
        )
    except (OSError, ValueError, tarfile.TarError, zipfile.BadZipFile) as error:
        parser.error(str(error))
    print(f"release artifacts verified: {args.directory}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
