#!/usr/bin/env python3
"""Create deterministic Zelyra release archives and SHA-256 sidecars."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import re
import shutil
import tarfile
import zipfile
from datetime import datetime, timezone
from pathlib import Path

RELEASE_FILES = ("LICENSE", "LICENSE-MIT", "README.de.md", "README.md")
TARGETS = {
    "linux": "x86_64-unknown-linux-gnu",
    "windows": "x86_64-pc-windows-msvc",
}


def _sha256_sidecar(path: Path) -> None:
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    path.with_name(path.name + ".sha256").write_text(
        f"{digest}  {path.name}\n", encoding="ascii", newline="\n"
    )


def _archive_entries(
    project_root: Path, binary: Path, platform: str, package_name: str
) -> list[tuple[str, bytes, int]]:
    executable_name = "zelyra.exe" if platform == "windows" else "zelyra"
    entries = [
        (f"{package_name}/{executable_name}", binary.read_bytes(), 0o755),
    ]
    entries.extend(
        (f"{package_name}/{name}", (project_root / name).read_bytes(), 0o644)
        for name in RELEASE_FILES
    )
    return sorted(entries, key=lambda entry: entry[0])


def _tar_gz(entries: list[tuple[str, bytes, int]], epoch: int) -> bytes:
    output = io.BytesIO()
    with gzip.GzipFile(
        filename="", mode="wb", fileobj=output, compresslevel=9, mtime=epoch
    ) as compressed:
        with tarfile.open(
            fileobj=compressed, mode="w", format=tarfile.USTAR_FORMAT
        ) as archive:
            for name, content, mode in entries:
                info = tarfile.TarInfo(name)
                info.size = len(content)
                info.mode = mode
                info.uid = 0
                info.gid = 0
                info.uname = ""
                info.gname = ""
                info.mtime = epoch
                info.pax_headers = {}
                archive.addfile(info, io.BytesIO(content))
    return output.getvalue()


def _zip(entries: list[tuple[str, bytes, int]], epoch: int) -> bytes:
    timestamp = datetime.fromtimestamp(epoch, tz=timezone.utc)
    if timestamp.year < 1980:
        timestamp = datetime(1980, 1, 1, tzinfo=timezone.utc)
    elif timestamp.year > 2107:
        timestamp = datetime(2107, 12, 31, 23, 59, 58, tzinfo=timezone.utc)
    else:
        timestamp = timestamp.replace(second=timestamp.second - timestamp.second % 2)
    zip_datetime = timestamp.replace(tzinfo=None)

    output = io.BytesIO()
    with zipfile.ZipFile(
        output, mode="w", compression=zipfile.ZIP_DEFLATED, compresslevel=9
    ) as archive:
        for name, content, mode in entries:
            info = zipfile.ZipInfo(name, date_time=zip_datetime.timetuple()[:6])
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = (0o100000 | mode) << 16
            info.flag_bits |= 0x800
            archive.writestr(info, content, compress_type=zipfile.ZIP_DEFLATED, compresslevel=9)
    return output.getvalue()


def package_release(
    *,
    project_root: Path,
    binary: Path,
    output_dir: Path,
    platform: str,
    tag: str,
    target: str,
    source_date_epoch: int,
) -> list[Path]:
    if platform not in TARGETS:
        raise ValueError(f"unsupported platform: {platform}")
    if target != TARGETS[platform]:
        raise ValueError(f"target {target} does not match platform {platform}")
    if not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?", tag):
        raise ValueError(f"invalid release tag: {tag}")
    if source_date_epoch < 0:
        raise ValueError("SOURCE_DATE_EPOCH must not be negative")
    if not binary.is_file():
        raise ValueError(f"release binary does not exist: {binary}")
    missing = [name for name in RELEASE_FILES if not (project_root / name).is_file()]
    if missing:
        raise ValueError("required release files are missing: " + ", ".join(missing))

    output_dir.mkdir(parents=True, exist_ok=True)
    package_name = f"zelyra-{tag}-{target}"
    extension = ".exe" if platform == "windows" else ".bin"
    executable_name = "zelyra.exe" if platform == "windows" else "zelyra"
    standalone = output_dir / f"{package_name}{extension}"
    shutil.copyfile(binary, standalone)
    if platform == "linux":
        standalone.chmod(0o755)
    _sha256_sidecar(standalone)

    entries = _archive_entries(project_root, binary, platform, package_name)
    archive_extension = ".zip" if platform == "windows" else ".tar.gz"
    archive_path = output_dir / f"{package_name}{archive_extension}"
    archive_bytes = (
        _zip(entries, source_date_epoch)
        if platform == "windows"
        else _tar_gz(entries, source_date_epoch)
    )
    archive_path.write_bytes(archive_bytes)
    _sha256_sidecar(archive_path)
    return [standalone, standalone.with_name(standalone.name + ".sha256"), archive_path, archive_path.with_name(archive_path.name + ".sha256")]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project-root", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--platform", choices=sorted(TARGETS), required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--source-date-epoch", type=int, required=True)
    args = parser.parse_args()
    try:
        outputs = package_release(
            project_root=args.project_root,
            binary=args.binary,
            output_dir=args.output_dir,
            platform=args.platform,
            tag=args.tag,
            target=args.target,
            source_date_epoch=args.source_date_epoch,
        )
    except (OSError, ValueError, tarfile.TarError, zipfile.BadZipFile) as error:
        parser.error(str(error))
    for output in outputs:
        print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
