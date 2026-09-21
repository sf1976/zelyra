#!/usr/bin/env python3
"""Regression tests for repeatable release package generation."""

from __future__ import annotations

import hashlib
import os
import re
import tarfile
import tempfile
import unittest
import zipfile
from datetime import datetime, timezone
from pathlib import Path

from package_release import RELEASE_FILES, package_release
from verify_release_artifacts import verify_release_artifacts


class ReleasePackageTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.project = self.root / "project"
        self.project.mkdir()
        self.binary = self.root / "zelyra-binary"
        self.binary.write_bytes(b"reproducible test executable\x00\x01")
        manifest = Path(__file__).resolve().parents[1] / "Cargo.toml"
        match = re.search(
            r'(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"\s*$',
            manifest.read_text(encoding="utf-8"),
        )
        assert match is not None
        self.version = match.group(1)
        self.tag = f"v{self.version}"
        for name in RELEASE_FILES:
            (self.project / name).write_text(f"fixture: {name}\n", encoding="utf-8")

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def package_twice(self, platform: str) -> tuple[list[Path], list[Path]]:
        target = {
            "linux": "x86_64-unknown-linux-gnu",
            "windows": "x86_64-pc-windows-msvc",
        }[platform]
        first = package_release(
            project_root=self.project,
            binary=self.binary,
            output_dir=self.root / f"{platform}-first",
            platform=platform,
            tag=self.tag,
            target=target,
            source_date_epoch=1_800_000_000,
        )
        for source in [self.binary, *self.project.iterdir()]:
            os.utime(source, (1_900_000_000, 1_900_000_000))
        second = package_release(
            project_root=self.project,
            binary=self.binary,
            output_dir=self.root / f"{platform}-second",
            platform=platform,
            tag=self.tag,
            target=target,
            source_date_epoch=1_800_000_000,
        )
        return first, second

    def assert_outputs_identical(self, first: list[Path], second: list[Path]) -> None:
        self.assertEqual([path.name for path in first], [path.name for path in second])
        for left, right in zip(first, second, strict=True):
            self.assertEqual(left.read_bytes(), right.read_bytes(), left.name)

    def test_linux_tar_and_checksums_are_byte_identical(self) -> None:
        first, second = self.package_twice("linux")
        self.assert_outputs_identical(first, second)
        archive = next(path for path in first if path.name.endswith(".tar.gz"))
        package_name = f"zelyra-{self.tag}-x86_64-unknown-linux-gnu"
        with tarfile.open(archive, "r:gz") as tar:
            self.assertEqual(
                tar.getnames(),
                [
                    f"{package_name}/LICENSE",
                    f"{package_name}/LICENSE-MIT",
                    f"{package_name}/README.de.md",
                    f"{package_name}/README.md",
                    f"{package_name}/zelyra",
                ],
            )
            for member in tar.getmembers():
                self.assertEqual(member.mtime, 1_800_000_000)
                self.assertEqual((member.uid, member.gid), (0, 0))
                self.assertEqual(member.uname, "")
                self.assertEqual(member.gname, "")

    def test_windows_zip_and_checksums_are_byte_identical(self) -> None:
        first, second = self.package_twice("windows")
        self.assert_outputs_identical(first, second)
        archive = next(path for path in first if path.name.endswith(".zip"))
        with zipfile.ZipFile(archive) as zipped:
            self.assertEqual(len(zipped.namelist()), 5)
            self.assertTrue(zipped.testzip() is None)
            expected_time = datetime.fromtimestamp(
                1_800_000_000, tz=timezone.utc
            )
            expected_time = expected_time.replace(
                second=expected_time.second - expected_time.second % 2
            )
            for member in zipped.infolist():
                self.assertEqual(member.date_time, expected_time.timetuple()[:6])

    def test_sidecar_checksum_matches_standalone_binary(self) -> None:
        outputs, _ = self.package_twice("linux")
        binary = next(path for path in outputs if path.suffix == ".bin")
        sidecar = binary.with_name(binary.name + ".sha256").read_text(encoding="ascii")
        self.assertEqual(sidecar.split()[0], hashlib.sha256(binary.read_bytes()).hexdigest())

    def test_linux_artifacts_pass_release_verifier(self) -> None:
        outputs, _ = self.package_twice("linux")
        verify_release_artifacts(
            directory=outputs[0].parent,
            platform="linux",
            tag=self.tag,
            target="x86_64-unknown-linux-gnu",
        )

    def test_windows_artifacts_pass_release_verifier(self) -> None:
        outputs, _ = self.package_twice("windows")
        verify_release_artifacts(
            directory=outputs[0].parent,
            platform="windows",
            tag=self.tag,
            target="x86_64-pc-windows-msvc",
        )

    def test_invalid_target_is_refused(self) -> None:
        with self.assertRaisesRegex(ValueError, "does not match platform"):
            package_release(
                project_root=self.project,
                binary=self.binary,
                output_dir=self.root / "invalid",
                platform="linux",
                tag=self.tag,
                target="x86_64-pc-windows-msvc",
                source_date_epoch=1_800_000_000,
            )


if __name__ == "__main__":
    unittest.main()
