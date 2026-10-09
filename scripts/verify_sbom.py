#!/usr/bin/env python3
"""Check that an SPDX SBOM identifies the release binary it accompanies."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Any


def verify_sbom(*, sbom_path: Path, binary_path: Path, tag: str, target: str) -> dict[str, Any]:
    if not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?", tag):
        raise ValueError("invalid release tag")
    document = json.loads(sbom_path.read_text(encoding="utf-8"))
    if document.get("spdxVersion") != "SPDX-2.3":
        raise ValueError("SBOM must use SPDX-2.3")
    if document.get("name") != f"Zelyra {tag} ({target})":
        raise ValueError("SBOM release identity does not match tag and target")
    release_version = tag[1:].split("-", 1)[0]
    if not re.fullmatch(r"https://zelyra\.dev/spdx/[0-9a-f]{64}", document.get("documentNamespace", "")):
        raise ValueError("SBOM document namespace is invalid")
    packages = document.get("packages")
    if not isinstance(packages, list):
        raise ValueError("SBOM packages must be an array")
    binaries = [package for package in packages if package.get("SPDXID") == "SPDXRef-Zelyra-Binary"]
    if len(binaries) != 1:
        raise ValueError("SBOM must describe exactly one Zelyra binary")
    if binaries[0].get("versionInfo") != release_version:
        raise ValueError("SBOM binary version does not match release tag")
    checksums = binaries[0].get("checksums", [])
    digest = hashlib.sha256(binary_path.read_bytes()).hexdigest()
    if {item.get("checksumValue") for item in checksums if item.get("algorithm") == "SHA256"} != {digest}:
        raise ValueError("SBOM binary SHA-256 does not match release binary")
    known_ids = {"SPDXRef-DOCUMENT", *(p.get("SPDXID") for p in packages)}
    for relationship in document.get("relationships", []):
        if relationship.get("spdxElementId") not in known_ids or relationship.get("relatedSpdxElement") not in known_ids:
            raise ValueError("SBOM relationship references an unknown SPDX identifier")
    return document


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sbom", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--target", required=True)
    args = parser.parse_args()
    try:
        document = verify_sbom(sbom_path=args.sbom, binary_path=args.binary, tag=args.tag, target=args.target)
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        parser.error(str(error))
    print(f"Verified SPDX SBOM: {len(document['packages'])} packages")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
