#!/usr/bin/env python3
"""Generate a deterministic SPDX 2.3 SBOM for a Zelyra CLI binary."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import tomllib
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _spdx_id(name: str, version: str, source: str) -> str:
    identity = f"{name}\0{version}\0{source}".encode()
    return "SPDXRef-Package-" + hashlib.sha256(identity).hexdigest()[:20]


def _license_info(package: dict[str, Any]) -> tuple[str, str | None]:
    license_value = package.get("license")
    if not license_value:
        return "NOASSERTION", "Cargo.toml does not declare a license."
    # SPDX expressions use AND/OR/WITH; Cargo's legacy `MIT/Apache-2.0`
    # spelling is not an SPDX expression and must not be emitted as one.
    if re.fullmatch(r"[A-Za-z0-9.+-]+(?:\s+(?:AND|OR|WITH)\s+[A-Za-z0-9.+-]+)*", license_value):
        return license_value, None
    return "NOASSERTION", f"Cargo license field: {license_value}"


def generate_sbom(
    *, metadata_path: Path, lock_path: Path, binary_path: Path,
    output_path: Path, tag: str, target: str, source_date_epoch: int,
) -> dict[str, Any]:
    if not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?", tag):
        raise ValueError(f"invalid release tag: {tag}")
    if source_date_epoch < 0:
        raise ValueError("SOURCE_DATE_EPOCH must not be negative")
    metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
    lock = tomllib.loads(lock_path.read_text(encoding="utf-8"))
    package_by_id = {package["id"]: package for package in metadata["packages"]}
    cli = [p for p in metadata["packages"] if p["name"] == "zelyra-cli"]
    if len(cli) != 1:
        raise ValueError("Cargo metadata must contain exactly one zelyra-cli package")
    root = cli[0]
    release_version = tag[1:].split("-", 1)[0]
    if root["version"] != release_version:
        raise ValueError(
            f"release tag version {release_version} does not match zelyra-cli {root['version']}"
        )
    nodes = {node["id"]: node for node in (metadata.get("resolve") or {}).get("nodes", [])}
    if root["id"] not in nodes:
        raise ValueError("Cargo metadata resolve graph is missing zelyra-cli")

    reachable: set[str] = set()
    pending = [root["id"]]
    while pending:
        package_id = pending.pop()
        if package_id in reachable:
            continue
        reachable.add(package_id)
        node = nodes.get(package_id, {})
        pending.extend(dep["pkg"] for dep in node.get("deps", []) if dep.get("pkg") in nodes)

    lock_entries = {
        (entry["name"], entry["version"], entry.get("source")): entry
        for entry in lock.get("package", [])
    }
    spdx_for = {
        package_id: _spdx_id(
            package_by_id[package_id]["name"], package_by_id[package_id]["version"],
            package_by_id[package_id].get("source") or "workspace",
        )
        for package_id in reachable
    }
    packages: list[dict[str, Any]] = []
    for package_id in sorted(reachable, key=lambda item: (package_by_id[item]["name"], package_by_id[item]["version"])):
        package = package_by_id[package_id]
        entry = lock_entries.get((package["name"], package["version"], package.get("source")))
        declared, comments = _license_info(package)
        item: dict[str, Any] = {
            "name": package["name"],
            "SPDXID": spdx_for[package_id],
            "versionInfo": package["version"],
            "downloadLocation": package.get("source") or "NOASSERTION",
            "filesAnalyzed": False,
            "licenseConcluded": "NOASSERTION",
            "licenseDeclared": declared,
            "copyrightText": "NOASSERTION",
            "checksums": ([{"algorithm": "SHA256", "checksumValue": entry["checksum"]}] if entry and entry.get("checksum") else []),
            "externalRefs": [{
                "referenceCategory": "PACKAGE-MANAGER",
                "referenceType": "purl",
                "referenceLocator": f"pkg:cargo/{package['name']}@{package['version']}",
            }],
        }
        if comments:
            item["licenseComments"] = comments
        packages.append(item)

    relationships = [{
        "spdxElementId": "SPDXRef-DOCUMENT",
        "relationshipType": "DESCRIBES",
        "relatedSpdxElement": spdx_for[root["id"]],
    }]
    for package_id in sorted(reachable):
        for dependency in nodes.get(package_id, {}).get("deps", []):
            target_id = dependency.get("pkg")
            if target_id in reachable:
                relationships.append({
                    "spdxElementId": spdx_for[package_id],
                    "relationshipType": "DEPENDS_ON",
                    "relatedSpdxElement": spdx_for[target_id],
                })
    relationships.sort(key=lambda rel: (rel["spdxElementId"], rel["relatedSpdxElement"]))
    created = datetime.fromtimestamp(source_date_epoch, tz=timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")
    doc: dict[str, Any] = {
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": f"Zelyra {tag} ({target})",
        "documentNamespace": "https://zelyra.dev/spdx/" + hashlib.sha256(
            f"{tag}\0{target}\0{_sha256(binary_path)}".encode()
        ).hexdigest(),
        "creationInfo": {
            "created": created,
            "creators": ["Tool: zelyra-sbom-generator"],
        },
        "packages": packages + [{
            "name": "Zelyra CLI binary",
            "SPDXID": "SPDXRef-Zelyra-Binary",
            "versionInfo": release_version,
            "downloadLocation": "NOASSERTION",
            "filesAnalyzed": False,
            "licenseConcluded": "NOASSERTION",
            "licenseDeclared": "NOASSERTION",
            "copyrightText": "NOASSERTION",
            "checksums": [{"algorithm": "SHA256", "checksumValue": _sha256(binary_path)}],
        }],
        "relationships": relationships + [{
            "spdxElementId": "SPDXRef-DOCUMENT",
            "relationshipType": "DESCRIBES",
            "relatedSpdxElement": "SPDXRef-Zelyra-Binary",
        }],
    }
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(doc, indent=2, sort_keys=True) + "\n", encoding="utf-8", newline="\n")
    return doc


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--metadata", type=Path, required=True, help="cargo metadata JSON")
    parser.add_argument("--lock", type=Path, default=Path("Cargo.lock"))
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--source-date-epoch", type=int, required=True)
    args = parser.parse_args()
    try:
        doc = generate_sbom(metadata_path=args.metadata, lock_path=args.lock,
            binary_path=args.binary, output_path=args.output, tag=args.tag,
            target=args.target, source_date_epoch=args.source_date_epoch)
    except (OSError, ValueError, KeyError, json.JSONDecodeError, tomllib.TOMLDecodeError) as error:
        parser.error(str(error))
    print(f"Generated {args.output} ({len(doc['packages'])} packages)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
