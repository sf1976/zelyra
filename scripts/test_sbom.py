#!/usr/bin/env python3
"""Regression tests for deterministic SPDX release inventories."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from generate_sbom import generate_sbom
from verify_sbom import verify_sbom


class SbomTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.binary = self.root / "zelyra"
        self.binary.write_bytes(b"fixture binary")
        self.metadata = self.root / "metadata.json"
        self.metadata.write_text(json.dumps({
            "packages": [
                {"id": "cli 0.4.0 (path+file:///workspace)", "name": "zelyra-cli", "version": "0.4.0", "source": None, "license": "MIT"},
                {"id": "dep 1.2.3 (registry+https://github.com/rust-lang/crates.io-index)", "name": "dep", "version": "1.2.3", "source": "registry+https://github.com/rust-lang/crates.io-index", "license": "MIT/Apache-2.0"},
            ],
            "resolve": {"nodes": [
                {"id": "cli 0.4.0 (path+file:///workspace)", "deps": [{"pkg": "dep 1.2.3 (registry+https://github.com/rust-lang/crates.io-index)"}]},
                {"id": "dep 1.2.3 (registry+https://github.com/rust-lang/crates.io-index)", "deps": []},
            ]},
        }), encoding="utf-8")
        self.lock = self.root / "Cargo.lock"
        self.lock.write_text('''version = 4

[[package]]
name = "dep"
version = "1.2.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
''', encoding="utf-8")

    def tearDown(self) -> None:
        self.temp.cleanup()

    def build(self, output: Path) -> None:
        generate_sbom(metadata_path=self.metadata, lock_path=self.lock,
            binary_path=self.binary, output_path=output, tag="v0.4.0",
            target="x86_64-unknown-linux-gnu", source_date_epoch=1_800_000_000)

    def test_output_is_deterministic_and_describes_dependency_graph(self) -> None:
        first, second = self.root / "first.json", self.root / "second.json"
        self.build(first)
        self.build(second)
        self.assertEqual(first.read_bytes(), second.read_bytes())
        doc = json.loads(first.read_text(encoding="utf-8"))
        self.assertEqual(len(doc["packages"]), 3)
        dependency = next(p for p in doc["packages"] if p["name"] == "dep")
        self.assertEqual(dependency["checksums"][0]["checksumValue"], "a" * 64)
        self.assertEqual(dependency["licenseDeclared"], "NOASSERTION")
        self.assertIn("MIT/Apache-2.0", dependency["licenseComments"])
        verify_sbom(sbom_path=first, binary_path=self.binary, tag="v0.4.0", target="x86_64-unknown-linux-gnu")

    def test_verifier_rejects_binary_mismatch(self) -> None:
        path = self.root / "sbom.json"
        self.build(path)
        self.binary.write_bytes(b"changed binary")
        with self.assertRaisesRegex(ValueError, "SHA-256"):
            verify_sbom(sbom_path=path, binary_path=self.binary, tag="v0.4.0", target="x86_64-unknown-linux-gnu")


if __name__ == "__main__":
    unittest.main()
