//! Version 1's checker depends on the wire format, file I/O and cryptography.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

pub mod deployment;

pub const PROOF_NAME: &str = "zelyra-proof.json";
pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_FILES: usize = 4096;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Evidence {
    Proven,
    Enforced,
    Tested,
    Attested,
    Assumed,
    Unverified,
}

pub const EVIDENCE_CLASSES: [Evidence; 6] = [
    Evidence::Proven,
    Evidence::Enforced,
    Evidence::Tested,
    Evidence::Attested,
    Evidence::Assumed,
    Evidence::Unverified,
];

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    Requires,
    Ensures,
    Invariant,
    StateMachine,
    Workflow,
    Authorization,
    Privacy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    CompilerProven,
    RuntimeCheck,
    Unproven,
    Failed,
    Unknown,
    Timeout,
    Undecidable,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub subject: String,
    pub rule: Rule,
    pub index: usize,
    pub outcome: Outcome,
    pub evidence: Evidence,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FileHash {
    pub path: String,
    pub sha256: String,
}

impl FileHash {
    pub fn new(path: String, bytes: &[u8]) -> Self {
        Self {
            path,
            sha256: sha256(bytes),
        }
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Boundaries {
    source_snapshot: Evidence,
    compiler_checks: Evidence,
    completeness_and_runtime: Evidence,
    external_systems: Evidence,
    legal_compliance: Evidence,
    general_semantic_proof: Evidence,
}

const BOUNDARIES: Boundaries = Boundaries {
    source_snapshot: Evidence::Proven,
    compiler_checks: Evidence::Enforced,
    completeness_and_runtime: Evidence::Assumed,
    external_systems: Evidence::Assumed,
    legal_compliance: Evidence::Assumed,
    general_semantic_proof: Evidence::Unverified,
};

const ASSUMPTIONS: &[&str] = &[
    "Source snapshot: SHA-256 collision resistance, trusted checker and a stable filesystem during collection and verification.",
    "The identified Zelyra compiler performed the reported checks on this complete input snapshot and used its matching embedded standard library.",
    "Live database schemas, identity and permission assignments, host configuration, deployment keys and legal compliance require independent operational review.",
];

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Proof {
    schema_version: u32,
    language_version: String,
    evidence_classes: [Evidence; 6],
    pub compiler_version: String,
    pub entry: String,
    pub files: Vec<FileHash>,
    pub embedded_ui_sha256: Option<String>,
    snapshot_sha256: String,
    assumptions: Vec<String>,
    boundaries: Boundaries,
    pub observations: Vec<Observation>,
}

impl Proof {
    pub fn new(
        compiler_version: &str,
        entry: String,
        files: Vec<FileHash>,
        embedded_ui_sha256: Option<String>,
        observations: Vec<Observation>,
    ) -> Result<Self, String> {
        let snapshot_sha256 = snapshot_hash(&entry, &files, embedded_ui_sha256.as_deref())?;
        Ok(Self {
            schema_version: SCHEMA_VERSION,
            language_version: "0.1".into(),
            evidence_classes: EVIDENCE_CLASSES,
            compiler_version: compiler_version.into(),
            entry,
            files,
            embedded_ui_sha256,
            snapshot_sha256,
            assumptions: ASSUMPTIONS.iter().map(|text| (*text).into()).collect(),
            boundaries: BOUNDARIES,
            observations,
        })
    }

    pub fn snapshot_sha256(&self) -> &str {
        &self.snapshot_sha256
    }

    pub fn check(&self, root: &Path) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION
            || self.language_version != "0.1"
            || self.evidence_classes != EVIDENCE_CLASSES
            || !valid_version(&self.compiler_version)
            || self.assumptions != ASSUMPTIONS
            || self.boundaries != BOUNDARIES
            || self
                .embedded_ui_sha256
                .as_ref()
                .is_some_and(|hash| !valid_hash(hash))
        {
            return Err("unsupported proof schema or inconsistent evidence boundaries".into());
        }
        let mut previous = None;
        for observation in &self.observations {
            let key = (&observation.subject, observation.rule, observation.index);
            if observation.subject.trim().is_empty()
                || previous.is_some_and(|last| last >= key)
                || observation.evidence != observation.outcome.evidence(observation.rule)
            {
                return Err("invalid, duplicate or inconsistently classified observation".into());
            }
            previous = Some(key);
        }
        if !self.entry.ends_with(".zyl") || !self.files.iter().any(|file| file.path == self.entry) {
            return Err("entry must name a bound .zyl file".into());
        }
        if self.snapshot_sha256
            != snapshot_hash(&self.entry, &self.files, self.embedded_ui_sha256.as_deref())?
        {
            return Err("snapshot checksum does not match the proof inventory".into());
        }
        check_files(root, &self.files, true)
    }
}

impl Outcome {
    pub fn evidence(self, rule: Rule) -> Evidence {
        match (self, rule) {
            (Self::RuntimeCheck, Rule::Requires | Rule::Ensures | Rule::Invariant) => {
                Evidence::Enforced
            }
            (Self::CompilerProven | Self::RuntimeCheck, _) => Evidence::Assumed,
            (
                Self::Unproven | Self::Failed | Self::Unknown | Self::Timeout | Self::Undecidable,
                _,
            ) => Evidence::Unverified,
        }
    }
}

fn valid_version(version: &str) -> bool {
    version.split('.').count() == 3 && version.split('.').all(|part| part.parse::<u32>().is_ok())
}

fn snapshot_hash(
    entry: &str,
    files: &[FileHash],
    embedded: Option<&str>,
) -> Result<String, String> {
    let bytes = serde_json::to_vec(&(entry, files, embedded)).map_err(|error| error.to_string())?;
    Ok(sha256(&bytes))
}

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn valid_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(result, "{byte:02x}").expect("write to String");
    }
    result
}

pub fn decode_hex<const N: usize>(value: &str) -> Result<[u8; N], String> {
    if value.len() != N * 2
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("invalid lowercase hexadecimal value".into());
    }
    let mut bytes = [0; N];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[2 * index..2 * index + 2], 16)
            .map_err(|_| "invalid hexadecimal value")?;
    }
    Ok(bytes)
}

pub fn json_bytes(value: &impl Serialize) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    if !fs::metadata(path)
        .map_err(|error| format!("{}: {error}", path.display()))?
        .is_file()
    {
        return Err(format!("expected regular file: {}", path.display()));
    }
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| file.take(MAX_INPUT_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|error| format!("{}: {error}", path.display()))?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err(format!("file exceeds v1 size limit: {}", path.display()));
    }
    Ok(bytes)
}

/// Resolve only project-relative portable paths. Reject symlinks at each
/// component; callers verify against a stable file tree.
pub fn bound_path(root: &Path, name: &str) -> Result<PathBuf, String> {
    let mut path = root.to_owned();
    let mut components = name.split('/').peekable();
    while let Some(part) = components.next() {
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with(['.', ' '])
            || part.contains(['\\', ':', '<', '>', '"', '|', '?', '*'])
            || part.chars().any(char::is_control)
        {
            return Err(format!("invalid bound path: {name}"));
        }
        path.push(part);
        let metadata = fs::symlink_metadata(&path).map_err(|error| format!("{name}: {error}"))?;
        let expected_kind = if components.peek().is_some() {
            metadata.is_dir()
        } else {
            metadata.is_file()
        };
        if metadata.file_type().is_symlink() || !expected_kind {
            return Err(format!("unexpected file type for bound path: {name}"));
        }
    }
    Ok(path)
}

pub fn check_files(root: &Path, files: &[FileHash], require_utf8: bool) -> Result<(), String> {
    if files.is_empty() || files.len() > MAX_FILES {
        return Err("invalid file inventory length".into());
    }
    let mut previous: Option<&str> = None;
    for file in files {
        if previous.is_some_and(|last| last >= file.path.as_str()) || !valid_hash(&file.sha256) {
            return Err("file inventory must have unique sorted paths and SHA-256 hashes".into());
        }
        let bytes = read_bounded(&bound_path(root, &file.path)?)?;
        if sha256(&bytes) != file.sha256 || (require_utf8 && std::str::from_utf8(&bytes).is_err()) {
            return Err(format!("file integrity check failed: {}", file.path));
        }
        previous = Some(&file.path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uncertain_and_failed_outcomes_always_remain_unverified() {
        for outcome in [
            Outcome::Unproven,
            Outcome::Failed,
            Outcome::Unknown,
            Outcome::Timeout,
            Outcome::Undecidable,
        ] {
            for rule in [
                Rule::Requires,
                Rule::Ensures,
                Rule::Invariant,
                Rule::StateMachine,
                Rule::Workflow,
                Rule::Authorization,
                Rule::Privacy,
            ] {
                assert_eq!(outcome.evidence(rule), Evidence::Unverified);
            }
        }
        assert_eq!(
            Outcome::CompilerProven.evidence(Rule::Requires),
            Evidence::Assumed
        );
        assert_eq!(
            Outcome::RuntimeCheck.evidence(Rule::Requires),
            Evidence::Enforced
        );
    }

    #[test]
    fn schema_rejects_unrecognized_outcomes_and_duplicate_fields() {
        let observation = r#"{"subject":"price","rule":"requires","index":0,"outcome":"timeout","evidence":"UNVERIFIED"}"#;
        assert!(serde_json::from_str::<Observation>(observation).is_ok());
        for altered in [
            observation.replace("timeout", "maybe"),
            observation.replace(r#""index":0"#, r#""index":0,"index":1"#),
            observation.replace(r#""index":0"#, r#""index":0,"extra":1"#),
        ] {
            assert!(serde_json::from_str::<Observation>(&altered).is_err());
        }
    }
}
