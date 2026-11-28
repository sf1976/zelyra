//! Verification of the v1 signed, closed deployment directory.
use super::{
    bound_path, check_files, decode_hex, hex, read_bounded, sha256, Evidence, FileHash, Proof,
    PROOF_NAME, SCHEMA_VERSION,
};
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path};

pub const MANIFEST: &str = "deployment-manifest.json";
pub const ATTESTATION: &str = "deployment-attestation.json";
pub const CHECKSUM: &str = "SHA256SUMS";
pub const SBOM: &str = "zelyra-deployment.spdx.json";
pub const PROJECT: &str = "project";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    schema_version: u32,
    pub entry: String,
    pub files: Vec<FileHash>,
}

impl Manifest {
    pub fn new(entry: String, files: Vec<FileHash>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            entry,
            files,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Attestation {
    schema_version: u32,
    algorithm: String,
    evidence: Evidence,
    pub public_key: String,
    signature: String,
    manifest_sha256: String,
}

fn signature_payload(manifest: &[u8]) -> Vec<u8> {
    let mut payload = b"Zelyra deployment manifest v1\0".to_vec();
    payload.extend_from_slice(manifest);
    payload
}

impl Attestation {
    pub fn sign(manifest: &[u8], pkcs8: &[u8]) -> Result<Self, String> {
        let key = Ed25519KeyPair::from_pkcs8_maybe_unchecked(pkcs8)
            .map_err(|_| "expected an Ed25519 PKCS#8 DER private key")?;
        Ok(Self {
            schema_version: SCHEMA_VERSION,
            algorithm: "Ed25519".into(),
            evidence: Evidence::Attested,
            public_key: hex(key.public_key().as_ref()),
            signature: hex(key.sign(&signature_payload(manifest)).as_ref()),
            manifest_sha256: sha256(manifest),
        })
    }

    fn check(&self, manifest: &[u8], trusted_key: &str) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION
            || self.algorithm != "Ed25519"
            || self.evidence != Evidence::Attested
            || self.public_key != trusted_key
            || self.manifest_sha256 != sha256(manifest)
        {
            return Err("deployment attestation is inconsistent or untrusted".into());
        }
        UnparsedPublicKey::new(&ED25519, decode_hex::<32>(trusted_key)?)
            .verify(
                &signature_payload(manifest),
                &decode_hex::<64>(&self.signature)?,
            )
            .map_err(|_| "invalid deployment signature".into())
    }
}

pub fn checksum_file(manifest: &[u8]) -> String {
    format!("{}  {MANIFEST}\n", sha256(manifest))
}

pub fn check(root: &Path, trusted_key: &str) -> Result<(), String> {
    let manifest_bytes = read_bounded(&bound_path(root, MANIFEST)?)?;
    let attestation: Attestation = read_json(root, ATTESTATION)?;
    attestation.check(&manifest_bytes, trusted_key)?;
    if read_bounded(&bound_path(root, CHECKSUM)?)? != checksum_file(&manifest_bytes).as_bytes() {
        return Err("deployment checksum mismatch".into());
    }
    let manifest: Manifest =
        serde_json::from_slice(&manifest_bytes).map_err(|error| error.to_string())?;
    if manifest.schema_version != SCHEMA_VERSION {
        return Err("unsupported deployment manifest".into());
    }
    check_files(root, &manifest.files, false)?;
    let proof: Proof = read_json(root, PROOF_NAME)?;
    proof.check(&root.join(PROJECT))?;
    if manifest.entry != format!("{PROJECT}/{}", proof.entry)
        || proof
            .observations
            .iter()
            .any(|item| item.outcome == super::Outcome::Failed)
    {
        return Err("bundle has a different entry or contains a failed verification check".into());
    }
    let project_files = proof.files.iter().map(|file| FileHash {
        path: format!("{PROJECT}/{}", file.path),
        sha256: file.sha256.clone(),
    });
    let mut expected = project_files.collect::<Vec<_>>();
    for path in [PROOF_NAME, SBOM] {
        expected.push(FileHash::new(
            path.into(),
            &read_bounded(&bound_path(root, path)?)?,
        ));
    }
    expected.sort_unstable();
    if expected != manifest.files {
        return Err("deployment inventory does not match the proof snapshot".into());
    }
    let project_root = root.join(PROJECT);
    let expected_sbom = spdx(&proof, |name| {
        read_bounded(&bound_path(&project_root, name)?)
    })?;
    if read_bounded(&bound_path(root, SBOM)?)? != super::json_bytes(&expected_sbom)? {
        return Err("deployment SBOM does not match the checked sources".into());
    }
    let mut expected_paths = manifest
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect::<BTreeSet<_>>();
    expected_paths.extend([MANIFEST, ATTESTATION, CHECKSUM].map(str::to_owned));
    check_tree(root, "", &mut expected_paths)?;
    if !expected_paths.is_empty() {
        return Err("deployment inventory has missing files".into());
    }
    Ok(())
}

fn read_json<T: serde::de::DeserializeOwned>(root: &Path, name: &str) -> Result<T, String> {
    serde_json::from_slice(&read_bounded(&bound_path(root, name)?)?)
        .map_err(|error| format!("{name}: {error}"))
}

fn check_tree(root: &Path, prefix: &str, expected: &mut BTreeSet<String>) -> Result<(), String> {
    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "non-UTF-8 bundle path")?;
        let path = format!("{prefix}{name}");
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        if kind.is_dir() {
            let prefix = format!("{path}/");
            if !expected
                .range(prefix.clone()..)
                .next()
                .is_some_and(|item| item.starts_with(&prefix))
            {
                return Err(format!("unbound deployment directory: {path}"));
            }
            check_tree(&entry.path(), &prefix, expected)?;
        } else if !kind.is_file() || !expected.remove(&path) {
            return Err(format!("unbound deployment file: {path}"));
        }
    }
    Ok(())
}

/// SPDX inventory for the checked application and externally provisioned
/// runtime. Legacy SHA-1 is mandated by SPDX for file/verification-code
/// fields. The deployment integrity check uses signed SHA-256 exclusively.
pub fn spdx(
    proof: &Proof,
    mut read_source: impl FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<serde_json::Value, String> {
    use ring::digest::{digest, SHA1_FOR_LEGACY_USE_ONLY};
    use serde_json::json;

    let sha1 = |bytes: &[u8]| hex(digest(&SHA1_FOR_LEGACY_USE_ONLY, bytes).as_ref());
    let mut file_hashes = Vec::with_capacity(proof.files.len());
    let mut files = Vec::with_capacity(proof.files.len());
    for (index, source) in proof.files.iter().enumerate() {
        let content = read_source(&source.path)?;
        let file_sha1 = sha1(&content);
        files.push(json!({
            "SPDXID": format!("SPDXRef-File-{index}"),
            "fileName": format!("./{PROJECT}/{}", source.path),
            "checksums": [
                {"algorithm": "SHA1", "checksumValue": file_sha1},
                {"algorithm": "SHA256", "checksumValue": source.sha256},
            ],
            "licenseConcluded": "NOASSERTION",
            "licenseInfoInFiles": ["NOASSERTION"],
            "copyrightText": "NOASSERTION",
        }));
        file_hashes.push(file_sha1);
    }
    file_hashes.sort_unstable();
    let verification_code = sha1(file_hashes.concat().as_bytes());
    let relationships = files
        .iter()
        .map(|file| {
            json!({
                "spdxElementId": "SPDXRef-Application",
                "relationshipType": "CONTAINS",
                "relatedSpdxElement": file["SPDXID"],
            })
        })
        .chain([json!({
            "spdxElementId": "SPDXRef-Application",
            "relationshipType": "DEPENDS_ON",
            "relatedSpdxElement": "SPDXRef-ZelyraRuntime",
        })])
        .collect::<Vec<_>>();
    Ok(json!({
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": "Zelyra application deployment",
        "documentNamespace": format!("https://zelyra.dev/spdx/deployment/{}/{}", proof.compiler_version, proof.snapshot_sha256()),
        "creationInfo": {"created": "1970-01-01T00:00:00Z", "creators": ["Tool: zelyra"]},
        "documentDescribes": ["SPDXRef-Application"],
        "packages": [
            {"SPDXID": "SPDXRef-Application", "name": "Zelyra application", "downloadLocation": "NOASSERTION",
             "filesAnalyzed": true, "packageVerificationCode": {"packageVerificationCodeValue": verification_code},
             "licenseConcluded": "NOASSERTION", "licenseDeclared": "NOASSERTION",
             "licenseInfoFromFiles": ["NOASSERTION"], "copyrightText": "NOASSERTION"},
            {"SPDXID": "SPDXRef-ZelyraRuntime", "name": "Zelyra runtime (provisioned separately)", "versionInfo": proof.compiler_version,
             "downloadLocation": "NOASSERTION", "filesAnalyzed": false, "licenseConcluded": "NOASSERTION",
             "licenseDeclared": "MIT OR Apache-2.0", "copyrightText": "NOASSERTION"}
        ],
        "files": files,
        "relationships": relationships,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deployment_signature_uses_external_trust_and_covers_manifest_bytes() {
        let key = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
        let attestation = Attestation::sign(b"manifest", key.as_ref()).unwrap();
        attestation
            .check(b"manifest", &attestation.public_key)
            .unwrap();
        assert!(attestation
            .check(b"changed", &attestation.public_key)
            .is_err());
        assert!(attestation.check(b"manifest", &"00".repeat(32)).is_err());

        let mut changed = attestation;
        changed.signature = "00".repeat(64);
        assert!(changed.check(b"manifest", &changed.public_key).is_err());
        assert!(Attestation::sign(b"manifest", b"not a key").is_err());
    }
}
