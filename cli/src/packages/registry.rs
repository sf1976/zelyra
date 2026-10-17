use super::*;
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};
use serde_json::Value;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use toml_edit::{value, ArrayOfTables, DocumentMut, Item, Table};

const PACKAGE_FORMAT: u64 = 1;
const MAX_PACKAGE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_PACKAGE_FILES: usize = 4096;
const MAX_REGISTRY_ARTIFACTS: usize = 10_000;
const MAX_TRUST_BYTES: u64 = 1024 * 1024;
const DEFAULT_TRUST_FILE: &str = "zelyra.trust.toml";
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

pub(super) fn command(
    action: &str,
    mut arguments: impl Iterator<Item = String>,
) -> std::process::ExitCode {
    use std::process::ExitCode;
    let result = match action {
        "keygen" => keygen_command(&mut arguments),
        "sign" => sign_command(&mut arguments),
        "verify" => verify_command(&mut arguments),
        "trust" => trust_command(&mut arguments),
        "registry" => registry_command(&mut arguments),
        _ => Err(format!("unknown package action `{action}`")),
    };
    match result {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error[E-PKG-003]: {error}");
            ExitCode::FAILURE
        }
    }
}

fn registry_command(args: &mut impl Iterator<Item = String>) -> Result<String, String> {
    match args.next().as_deref() {
        Some("build") => registry_build(args),
        _ => Err(
            "usage: zelyra package registry build <artifact-directory> [--output <index.json>]"
                .into(),
        ),
    }
}

fn registry_build(args: &mut impl Iterator<Item = String>) -> Result<String, String> {
    let root = args.next().map(PathBuf::from).ok_or_else(|| {
        "usage: zelyra package registry build <artifact-directory> [--output <index.json>]"
            .to_owned()
    })?;
    let mut output = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--output" if output.is_none() => {
                output = Some(PathBuf::from(required_value(args, "--output")?));
            }
            _ => return Err("unexpected or repeated package registry build option".into()),
        }
    }
    let metadata = fs::symlink_metadata(&root)
        .map_err(|error| format!("cannot inspect registry artifact directory: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("registry artifact root must be a real directory".into());
    }
    let root = fs::canonicalize(&root)
        .map_err(|error| format!("cannot resolve registry artifact directory: {error}"))?;
    let mut artifacts = Vec::new();
    collect_registry_artifacts(&root, &root, &mut artifacts)?;
    if artifacts.len() > MAX_REGISTRY_ARTIFACTS {
        return Err(format!(
            "registry contains more than {MAX_REGISTRY_ARTIFACTS} package artifacts"
        ));
    }
    artifacts.sort_by(|left, right| left.0.cmp(&right.0));
    let entries = artifacts
        .into_iter()
        .map(|(path, bytes)| {
            let artifact = verify_artifact(&bytes)?;
            Ok(serde_json::json!({
                "name": artifact.name,
                "version": artifact.version,
                "publisher": artifact.publisher,
                "public_key": hex_encode(&artifact.public_key),
                "checksum": artifact.checksum,
                "artifact_sha256": format!("sha256:{:x}", Sha256::digest(&bytes)),
                "artifact": path,
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let package_count = entries.len();
    let bytes = serde_json::to_vec_pretty(&serde_json::json!({
        "format_version": 1,
        "packages": entries,
    }))
    .map_err(|error| format!("cannot encode registry index: {error}"))?;
    let output = output.unwrap_or_else(|| root.join("index.json"));
    write_atomic(&output, &bytes, 0o644)?;
    Ok(format!(
        "wrote deterministic static registry index {} ({} packages)",
        output.display(),
        package_count
    ))
}

fn collect_registry_artifacts(
    root: &Path,
    directory: &Path,
    artifacts: &mut Vec<(String, Vec<u8>)>,
) -> Result<(), String> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| format!("cannot read registry directory: {error}"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot inspect registry directory: {error}"))?;
    entries.sort();
    for path in entries {
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect registry entry: {error}"))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "registry tree may not contain symlinks: {}",
                path.display()
            ));
        }
        if metadata.is_dir() {
            collect_registry_artifacts(root, &path, artifacts)?;
        } else if metadata.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension == "zpkg")
        {
            if metadata.len() > MAX_PACKAGE_BYTES {
                return Err(format!(
                    "package artifact exceeds the 32 MiB limit: {}",
                    path.display()
                ));
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|_| "registry artifact escaped its root".to_owned())?
                .to_str()
                .ok_or_else(|| "registry paths must be UTF-8".to_owned())?
                .replace('\\', "/");
            artifacts.push((
                relative,
                fs::read(&path)
                    .map_err(|error| format!("cannot read registry artifact: {error}"))?,
            ));
            if artifacts.len() > MAX_REGISTRY_ARTIFACTS {
                return Err(format!(
                    "registry contains more than {MAX_REGISTRY_ARTIFACTS} package artifacts"
                ));
            }
        }
    }
    Ok(())
}

fn keygen_command(args: &mut impl Iterator<Item = String>) -> Result<String, String> {
    let mut output = None;
    let mut publisher = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--output" if output.is_none() => {
                output = Some(PathBuf::from(required_value(args, "--output")?));
            }
            "--publisher" if publisher.is_none() => {
                publisher = Some(required_value(args, "--publisher")?);
            }
            _ => {
                return Err(
                    "usage: zelyra package keygen --publisher <id> --output <private-key-file>"
                        .into(),
                )
            }
        }
    }
    let output = output.ok_or_else(|| "package keygen requires --output".to_owned())?;
    let publisher = publisher.ok_or_else(|| "package keygen requires --publisher".to_owned())?;
    validate_publisher_id(&publisher)?;
    let rng = ring::rand::SystemRandom::new();
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng)
        .map_err(|_| "could not generate an Ed25519 key".to_owned())?;
    create_private_key_file(&output, pkcs8.as_ref())?;
    let key = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref())
        .map_err(|_| "generated Ed25519 key could not be loaded".to_owned())?;
    let public_key = hex_encode(key.public_key().as_ref());
    let fingerprint = key_fingerprint(key.public_key().as_ref());
    Ok(format!(
        "created private signing key {}; publisher `{publisher}` public key: {public_key} (SHA-256 {fingerprint}); keep the private key offline",
        output.display()
    ))
}

fn sign_command(args: &mut impl Iterator<Item = String>) -> Result<String, String> {
    let directory = args.next().map(PathBuf::from).ok_or_else(|| {
        "usage: zelyra package sign <package-directory> --publisher <id> --key <private-key-file> --output <artifact.zpkg>".to_owned()
    })?;
    let mut publisher = None;
    let mut key_path = None;
    let mut output = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--publisher" if publisher.is_none() => {
                publisher = Some(required_value(args, "--publisher")?)
            }
            "--key" if key_path.is_none() => {
                key_path = Some(PathBuf::from(required_value(args, "--key")?))
            }
            "--output" if output.is_none() => {
                output = Some(PathBuf::from(required_value(args, "--output")?))
            }
            _ => return Err("unexpected or repeated package sign option".into()),
        }
    }
    let publisher = publisher.ok_or_else(|| "package sign requires --publisher".to_owned())?;
    validate_publisher_id(&publisher)?;
    let key_path = key_path.ok_or_else(|| "package sign requires --key".to_owned())?;
    let output = output.ok_or_else(|| "package sign requires --output".to_owned())?;
    let manifest_path = directory.join(MANIFEST);
    let manifest = read_manifest(&manifest_path).map_err(|error| error.message)?;
    if manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .is_some_and(|deps| !deps.is_empty())
    {
        return Err("signed source artifacts currently require a dependency-free package; publish dependencies as separate packages first".into());
    }
    let package_table = manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| "package source must declare [package]".to_owned())?;
    let name =
        string_field(package_table, "name", &manifest_path).map_err(|error| error.message)?;
    validate_package_name(&name).map_err(|error| error.message)?;
    let version =
        string_field(package_table, "version", &manifest_path).map_err(|error| error.message)?;
    Version::parse(&version)
        .map_err(|error| format!("invalid package version `{version}`: {error}"))?;
    let package_metadata = fs::symlink_metadata(&directory).map_err(|error| {
        format!(
            "cannot inspect package directory {}: {error}",
            directory.display()
        )
    })?;
    if package_metadata.file_type().is_symlink() || !package_metadata.is_dir() {
        return Err("package signing requires a real directory, not a symlink".into());
    }
    let package_root = fs::canonicalize(&directory)
        .map_err(|error| format!("cannot resolve package directory: {error}"))?;
    let paths = package_source_files(&package_root).map_err(|error| error.message)?;
    if paths.len() > MAX_PACKAGE_FILES {
        return Err(format!(
            "package has more than {MAX_PACKAGE_FILES} source files"
        ));
    }
    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        let relative = path
            .to_str()
            .ok_or_else(|| "package source paths must be UTF-8".to_owned())?
            .replace('\\', "/");
        let source = fs::read_to_string(package_root.join(path))
            .map_err(|error| format!("cannot read package source `{relative}`: {error}"))?;
        files.push((relative, source.replace("\r\n", "\n")));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    validate_package_sources(&files, &name, &version)?;
    let checksum = checksum_source_files(&files);
    let key_bytes = read_regular_file(&key_path, 4096, "private signing key")?;
    let key = Ed25519KeyPair::from_pkcs8(&key_bytes)
        .map_err(|_| "private signing key is not a valid Ed25519 PKCS#8 key".to_owned())?;
    let payload = signature_payload(&publisher, &name, &version, &checksum);
    let signature = key.sign(payload.as_bytes());
    let archive = serde_json::json!({
        "format_version": PACKAGE_FORMAT,
        "publisher": publisher,
        "package": {"name": name, "version": version},
        "checksum": checksum,
        "public_key": hex_encode(key.public_key().as_ref()),
        "signature": hex_encode(signature.as_ref()),
        "files": files.iter().map(|(path, source)| serde_json::json!({"path": path, "source": source})).collect::<Vec<_>>(),
    });
    let bytes = serde_json::to_vec(&archive)
        .map_err(|error| format!("cannot encode package artifact: {error}"))?;
    if bytes.len() as u64 > MAX_PACKAGE_BYTES {
        return Err(format!(
            "signed package artifact exceeds the {} MiB limit",
            MAX_PACKAGE_BYTES / (1024 * 1024)
        ));
    }
    write_new_file(&output, &bytes, 0o644, "package artifact")?;
    Ok(format!(
        "wrote signed package {} {version} to {} ({} bytes, {})",
        name,
        output.display(),
        bytes.len(),
        checksum
    ))
}

fn verify_command(args: &mut impl Iterator<Item = String>) -> Result<String, String> {
    let artifact = args.next().map(PathBuf::from).ok_or_else(|| {
        "usage: zelyra package verify <artifact.zpkg> [--trust <trust-file>]".to_owned()
    })?;
    let mut trust_path = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--trust" if trust_path.is_none() => {
                trust_path = Some(PathBuf::from(required_value(args, "--trust")?))
            }
            _ => return Err("unexpected or repeated package verify option".into()),
        }
    }
    let artifact_bytes = read_regular_file(
        &artifact,
        MAX_PACKAGE_BYTES as usize,
        "signed package artifact",
    )?;
    let parsed = verify_artifact(&artifact_bytes)?;
    let trust_path = trust_path.unwrap_or_else(|| PathBuf::from(DEFAULT_TRUST_FILE));
    let trusted = trusted_key(&trust_path, &parsed.publisher, &parsed.public_key)?;
    if !trusted {
        return Err(format!("package signature is cryptographically valid, but publisher `{}` is not trusted in {}; review the publisher key and add it explicitly with `zelyra package trust add`", parsed.publisher, trust_path.display()));
    }
    Ok(format!(
        "verified trusted package {} {} from publisher `{}` ({})",
        parsed.name, parsed.version, parsed.publisher, parsed.checksum
    ))
}

#[derive(Debug)]
struct VerifiedArtifact {
    publisher: String,
    name: String,
    version: String,
    checksum: String,
    public_key: Vec<u8>,
}

fn verify_artifact(bytes: &[u8]) -> Result<VerifiedArtifact, String> {
    if bytes.len() as u64 > MAX_PACKAGE_BYTES {
        return Err("signed package artifact exceeds the 32 MiB limit".into());
    }
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("invalid package artifact JSON: {error}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "package artifact must be a JSON object".to_owned())?;
    let expected_fields = [
        "format_version",
        "publisher",
        "package",
        "checksum",
        "public_key",
        "signature",
        "files",
    ];
    if object.len() != expected_fields.len()
        || object
            .keys()
            .any(|key| !expected_fields.contains(&key.as_str()))
    {
        return Err("package artifact has missing or unknown fields".into());
    }
    if object.get("format_version").and_then(Value::as_u64) != Some(PACKAGE_FORMAT) {
        return Err("unsupported package artifact format version".into());
    }
    let publisher = required_string(object, "publisher")?.to_owned();
    validate_publisher_id(&publisher)?;
    let package = object
        .get("package")
        .and_then(Value::as_object)
        .ok_or_else(|| "package identity must be an object".to_owned())?;
    if package.len() != 2 || package.keys().any(|key| key != "name" && key != "version") {
        return Err("package identity has missing or unknown fields".into());
    }
    let name = required_string(package, "name")?.to_owned();
    validate_package_name(&name).map_err(|error| error.message)?;
    let version = required_string(package, "version")?.to_owned();
    Version::parse(&version)
        .map_err(|error| format!("invalid package version `{version}`: {error}"))?;
    let checksum = required_string(object, "checksum")?.to_owned();
    if !checksum.starts_with("sha256:") || checksum.len() != 71 {
        return Err("package checksum must be a SHA-256 digest".into());
    }
    let public_key = hex_decode(required_string(object, "public_key")?)?;
    if public_key.len() != 32 {
        return Err("Ed25519 public key must contain 32 bytes".into());
    }
    let signature = hex_decode(required_string(object, "signature")?)?;
    if signature.len() != 64 {
        return Err("Ed25519 signature must contain 64 bytes".into());
    }
    let file_values = object
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| "package files must be an array".to_owned())?;
    if file_values.is_empty() || file_values.len() > MAX_PACKAGE_FILES {
        return Err(format!(
            "package artifact must contain between 1 and {MAX_PACKAGE_FILES} source files"
        ));
    }
    let mut files = Vec::with_capacity(file_values.len());
    let mut seen = BTreeSet::new();
    for file in file_values {
        let file = file
            .as_object()
            .ok_or_else(|| "each package source file must be an object".to_owned())?;
        if file.len() != 2 || file.keys().any(|key| key != "path" && key != "source") {
            return Err("package source entry has missing or unknown fields".into());
        }
        let path = required_string(file, "path")?.to_owned();
        validate_archive_path(&path)?;
        if !seen.insert(path.clone()) {
            return Err(format!("package artifact repeats source path `{path}`"));
        }
        let source = required_string(file, "source")?.replace("\r\n", "\n");
        files.push((path, source));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    if !files.iter().any(|(path, _)| path == MANIFEST) {
        return Err("package artifact does not contain zelyra.toml".into());
    }
    let actual_checksum = checksum_source_files(&files);
    if actual_checksum != checksum {
        return Err("package source checksum does not match the signed metadata".into());
    }
    let manifest = files
        .iter()
        .find(|(path, _)| path == MANIFEST)
        .expect("manifest checked")
        .1
        .parse::<toml::Value>()
        .map_err(|error| format!("package manifest is invalid: {error}"))?;
    let manifest_package = manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| "package manifest must declare [package]".to_owned())?;
    if manifest_package.get("name").and_then(toml::Value::as_str) != Some(name.as_str())
        || manifest_package
            .get("version")
            .and_then(toml::Value::as_str)
            != Some(version.as_str())
    {
        return Err("signed package identity does not match zelyra.toml".into());
    }
    let payload = signature_payload(&publisher, &name, &version, &checksum);
    UnparsedPublicKey::new(&ED25519, &public_key)
        .verify(payload.as_bytes(), &signature)
        .map_err(|_| "package signature is invalid".to_owned())?;
    Ok(VerifiedArtifact {
        publisher,
        name,
        version,
        checksum,
        public_key,
    })
}

fn trust_command(args: &mut impl Iterator<Item = String>) -> Result<String, String> {
    let action = args
        .next()
        .ok_or_else(|| "usage: zelyra package trust <add|list|remove> ...".to_owned())?;
    match action.as_str() {
        "add" => trust_add(args),
        "list" => trust_list(args),
        "remove" => trust_remove(args),
        _ => Err(format!("unknown package trust action `{action}`")),
    }
}

fn trust_add(args: &mut impl Iterator<Item = String>) -> Result<String, String> {
    let publisher = args.next().ok_or_else(|| {
        "usage: zelyra package trust add <publisher-id> <public-key-hex> [--file <trust-file>]"
            .to_owned()
    })?;
    validate_publisher_id(&publisher)?;
    let public_key = hex_decode(
        &args
            .next()
            .ok_or_else(|| "package trust add requires an Ed25519 public key".to_owned())?,
    )?;
    if public_key.len() != 32 {
        return Err("Ed25519 public key must contain 32 bytes".into());
    }
    let trust_path = parse_trust_file(args)?;
    let mut document = read_trust_document(&trust_path)?;
    let entries = trusted_publishers_mut(&mut document)?;
    if entries.iter().any(|item| {
        item.get("publisher").and_then(Item::as_str) == Some(publisher.as_str())
            && item.get("public_key").and_then(Item::as_str)
                == Some(hex_encode(&public_key).as_str())
    }) {
        return Ok(format!(
            "publisher `{publisher}` key is already trusted in {}",
            trust_path.display()
        ));
    }
    let mut entry = Table::new();
    entry["publisher"] = value(publisher.clone());
    entry["public_key"] = value(hex_encode(&public_key));
    entries.push(entry);
    write_atomic(&trust_path, document.to_string().as_bytes(), 0o600)?;
    Ok(format!(
        "trusted Ed25519 key for publisher `{publisher}` in {} (SHA-256 {})",
        trust_path.display(),
        key_fingerprint(&public_key)
    ))
}

fn trust_list(args: &mut impl Iterator<Item = String>) -> Result<String, String> {
    let trust_path = parse_trust_file(args)?;
    let document = read_trust_document(&trust_path)?;
    let entries = document
        .get("trusted_publishers")
        .and_then(Item::as_array_of_tables);
    let Some(entries) = entries else {
        return Ok(format!("no trusted publishers in {}", trust_path.display()));
    };
    if entries.is_empty() {
        return Ok(format!("no trusted publishers in {}", trust_path.display()));
    }
    let mut lines = Vec::new();
    for entry in entries.iter() {
        let publisher = entry
            .get("publisher")
            .and_then(Item::as_str)
            .ok_or_else(|| "trust entry is missing publisher".to_owned())?;
        let key = entry
            .get("public_key")
            .and_then(Item::as_str)
            .ok_or_else(|| "trust entry is missing public_key".to_owned())?;
        let bytes = hex_decode(key)?;
        if bytes.len() != 32 {
            return Err(format!(
                "trust entry for `{publisher}` has an invalid key length"
            ));
        }
        lines.push(format!(
            "{publisher} {} SHA-256 {}",
            key,
            key_fingerprint(&bytes)
        ));
    }
    Ok(lines.join("\n"))
}

fn trust_remove(args: &mut impl Iterator<Item = String>) -> Result<String, String> {
    let publisher = args.next().ok_or_else(|| "usage: zelyra package trust remove <publisher-id> <public-key-hex> [--file <trust-file>]".to_owned())?;
    validate_publisher_id(&publisher)?;
    let public_key = args
        .next()
        .ok_or_else(|| "package trust remove requires the exact public key".to_owned())?;
    let decoded = hex_decode(&public_key)?;
    let trust_path = parse_trust_file(args)?;
    let mut document = read_trust_document(&trust_path)?;
    let entries = trusted_publishers_mut(&mut document)?;
    let old_len = entries.len();
    entries.retain(|entry| {
        !(entry.get("publisher").and_then(Item::as_str) == Some(publisher.as_str())
            && entry.get("public_key").and_then(Item::as_str) == Some(public_key.as_str()))
    });
    if entries.len() == old_len {
        return Err(format!("trusted key for `{publisher}` was not found"));
    }
    write_atomic(&trust_path, document.to_string().as_bytes(), 0o600)?;
    Ok(format!(
        "removed trusted key for publisher `{publisher}` (SHA-256 {})",
        key_fingerprint(&decoded)
    ))
}

fn parse_trust_file(args: &mut impl Iterator<Item = String>) -> Result<PathBuf, String> {
    match (args.next(), args.next()) {
        (None, None) => Ok(PathBuf::from(DEFAULT_TRUST_FILE)),
        (Some(option), Some(path)) if option == "--file" => Ok(PathBuf::from(path)),
        _ => Err("trust command accepts only an optional `--file <trust-file>`".into()),
    }
}

fn read_trust_document(path: &Path) -> Result<DocumentMut, String> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(DocumentMut::new()),
        Err(error) => Err(format!(
            "cannot inspect publisher trust file {}: {error}",
            path.display()
        )),
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => Err(format!(
            "publisher trust file {} must be a regular non-symlink file",
            path.display()
        )),
        Ok(_) => {
            let bytes = read_regular_file(path, MAX_TRUST_BYTES as usize, "publisher trust file")?;
            std::str::from_utf8(&bytes)
                .map_err(|_| "publisher trust file must be UTF-8".to_owned())?
                .parse::<DocumentMut>()
                .map_err(|error| format!("invalid publisher trust TOML: {error}"))
        }
    }
}

fn trusted_publishers_mut(document: &mut DocumentMut) -> Result<&mut ArrayOfTables, String> {
    if !document.contains_key("trusted_publishers") {
        document["trusted_publishers"] = Item::ArrayOfTables(ArrayOfTables::new());
    }
    document["trusted_publishers"]
        .as_array_of_tables_mut()
        .ok_or_else(|| "`trusted_publishers` must be an array of tables".to_owned())
}

fn trusted_key(path: &Path, publisher: &str, key: &[u8]) -> Result<bool, String> {
    let document = read_trust_document(path)?;
    let Some(entries) = document
        .get("trusted_publishers")
        .and_then(Item::as_array_of_tables)
    else {
        return Ok(false);
    };
    for entry in entries.iter() {
        let name = entry
            .get("publisher")
            .and_then(Item::as_str)
            .ok_or_else(|| "trust entry is missing publisher".to_owned())?;
        let public_key = entry
            .get("public_key")
            .and_then(Item::as_str)
            .ok_or_else(|| "trust entry is missing public_key".to_owned())?;
        if name == publisher && hex_decode(public_key)? == key {
            return Ok(true);
        }
    }
    Ok(false)
}

fn validate_archive_path(path: &str) -> Result<(), String> {
    if path.is_empty() || path.starts_with('/') || path.contains('\\') || path.contains(':') {
        return Err(format!("unsafe package source path `{path}`"));
    }
    let candidate = Path::new(path);
    if candidate
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
        || is_unhashed_package_path(candidate)
        || !(path == MANIFEST || path.ends_with(".zyl"))
    {
        return Err(format!(
            "unsafe or unsupported package source path `{path}`"
        ));
    }
    Ok(())
}

fn checksum_source_files(files: &[(String, String)]) -> String {
    let mut files = files.iter().collect::<Vec<_>>();
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hash = Sha256::new();
    for (path, source) in files {
        let bytes = source.replace("\r\n", "\n").into_bytes();
        hash.update((path.len() as u64).to_be_bytes());
        hash.update(path.as_bytes());
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    }
    format!("sha256:{:x}", hash.finalize())
}

fn validate_package_sources(
    files: &[(String, String)],
    package_name: &str,
    package_version: &str,
) -> Result<(), String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock is before Unix epoch: {error}"))?
        .as_nanos();
    let validation_root = std::env::temp_dir().join(format!(
        "zelyra-package-check-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir(&validation_root)
        .map_err(|error| format!("cannot create isolated package validation directory: {error}"))?;
    let package_root = validation_root.join("candidate");
    let result = (|| {
        for (relative, source) in files {
            validate_archive_path(relative)?;
            let destination = package_root.join(relative);
            let parent = destination
                .parent()
                .ok_or_else(|| "invalid package source path".to_owned())?;
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot prepare isolated package source: {error}"))?;
            fs::write(destination, source)
                .map_err(|error| format!("cannot stage isolated package source: {error}"))?;
        }
        let root_manifest = format!(
            "[package]\nname = \"package_validation\"\nversion = \"0.0.0\"\nentry = \"main.zyl\"\n\n[dependencies.candidate]\npath = \"candidate\"\nversion = \"={package_version}\"\n"
        );
        fs::write(validation_root.join(MANIFEST), root_manifest)
            .map_err(|error| format!("cannot write isolated package manifest: {error}"))?;
        fs::write(
            validation_root.join("main.zyl"),
            "import \"pkg:candidate\" as candidate\nfn main() {}\n",
        )
        .map_err(|error| format!("cannot write isolated validation entry: {error}"))?;
        let graph = PackageGraph::load(&validation_root)
            .map_err(|error| error.message)?
            .ok_or_else(|| "isolated package validation graph was not created".to_owned())?;
        write_lockfile(&graph, true).map_err(|error| error.message)?;
        let entry = validation_root.join("main.zyl");
        let entry = entry
            .to_str()
            .ok_or_else(|| "temporary package validation path is not UTF-8".to_owned())?;
        super::super::validate(entry)
            .map_err(|_| format!("package `{package_name}` failed Zelyra compiler validation"))?;
        Ok(())
    })();
    let cleanup = fs::remove_dir_all(&validation_root);
    match (result, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(format!(
            "package validation passed but its temporary sources could not be removed: {error}"
        )),
    }
}

fn signature_payload(publisher: &str, package: &str, version: &str, checksum: &str) -> String {
    format!("zelyra-package-signature-v1\n{publisher}\n{package}\n{version}\n{checksum}")
}

fn validate_publisher_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 64
        || !id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
    {
        return Err("publisher ID must be 1–64 lowercase ASCII letters, digits, dots, underscores, or hyphens".into());
    }
    Ok(())
}

fn required_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn required_string<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("package artifact field `{key}` must be a string"))
}

fn read_regular_file(path: &Path, max_bytes: usize, description: &str) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect {description} {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "{description} {} must be a regular non-symlink file",
            path.display()
        ));
    }
    if metadata.len() > max_bytes as u64 {
        return Err(format!(
            "{description} {} exceeds the {} byte limit",
            path.display(),
            max_bytes
        ));
    }
    fs::read(path).map_err(|error| format!("cannot read {description} {}: {error}", path.display()))
}

fn create_private_key_file(path: &Path, contents: &[u8]) -> Result<(), String> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|error| {
        format!(
            "cannot create private key {} without overwriting: {error}",
            path.display()
        )
    })?;
    file.write_all(contents)
        .and_then(|()| file.sync_all())
        .map_err(|error| {
            let _ = fs::remove_file(path);
            format!(
                "cannot safely write private key {}: {error}",
                path.display()
            )
        })
}

fn write_new_file(
    path: &Path,
    contents: &[u8],
    mode: u32,
    description: &str,
) -> Result<(), String> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(mode);
    }
    let mut file = options.open(path).map_err(|error| {
        format!(
            "cannot create {description} {} without overwriting: {error}",
            path.display()
        )
    })?;
    file.write_all(contents)
        .and_then(|()| file.sync_all())
        .map_err(|error| {
            let _ = fs::remove_file(path);
            format!("cannot write {description} {}: {error}", path.display())
        })
}

fn write_atomic(path: &Path, contents: &[u8], mode: u32) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(format!(
                "output file {} must be a regular non-symlink file",
                path.display()
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "cannot inspect output file {}: {error}",
                path.display()
            ));
        }
    }
    let temporary = loop {
        let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let candidate = path.with_file_name(format!(
            ".zelyra-trust-{}-{sequence}.tmp",
            std::process::id()
        ));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(mode);
        }
        match options.open(&candidate) {
            Ok(mut file) => {
                file.write_all(contents)
                    .and_then(|()| file.sync_all())
                    .map_err(|error| {
                        let _ = fs::remove_file(&candidate);
                        format!("cannot write temporary trust file: {error}")
                    })?;
                break candidate;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("cannot create temporary trust file: {error}")),
        }
    };
    #[cfg(unix)]
    let publish = fs::rename(&temporary, path);
    #[cfg(windows)]
    let publish = replace_lockfile_windows(&temporary, path);
    #[cfg(not(any(unix, windows)))]
    let publish = if path.exists() {
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "atomic replacement is unsupported",
        ))
    } else {
        fs::rename(&temporary, path)
    };
    publish.map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!(
            "cannot atomically publish trust file {}: {error}",
            path.display()
        )
    })
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn hex_decode(value: &str) -> Result<Vec<u8>, String> {
    if value.len() & 1 != 0 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("key or signature must be an even-length hexadecimal string".into());
    }
    value
        .as_bytes()
        .chunks(2)
        .map(|pair| {
            let high = (pair[0] as char)
                .to_digit(16)
                .ok_or_else(|| "invalid hex digit".to_owned())?;
            let low = (pair[1] as char)
                .to_digit(16)
                .ok_or_else(|| "invalid hex digit".to_owned())?;
            Ok(((high << 4) | low) as u8)
        })
        .collect()
}

fn key_fingerprint(public_key: &[u8]) -> String {
    format!("{:x}", Sha256::digest(public_key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("zelyra-signed-package-{stamp}"));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join(MANIFEST), "[package]\nname = \"hello\"\nversion = \"1.2.3\"\nentry = \"src/lib.zyl\"\nzelyra = \">=1.1, <2.0\"\n").unwrap();
        fs::write(
            root.join("src/lib.zyl"),
            "pub fn greeting() -> String { \"hello\" }\n",
        )
        .unwrap();
        root
    }

    #[test]
    fn signed_artifact_binds_source_identity_and_trusted_publisher() {
        let root = fixture();
        let rng = ring::rand::SystemRandom::new();
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let signer = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
        let files = vec![
            (
                MANIFEST.to_owned(),
                fs::read_to_string(root.join(MANIFEST)).unwrap(),
            ),
            (
                "src/lib.zyl".to_owned(),
                fs::read_to_string(root.join("src/lib.zyl")).unwrap(),
            ),
        ];
        let checksum = checksum_source_files(&files);
        let signature =
            signer.sign(signature_payload("acme", "hello", "1.2.3", &checksum).as_bytes());
        let artifact = serde_json::json!({
            "format_version": PACKAGE_FORMAT,
            "publisher": "acme",
            "package": {"name": "hello", "version": "1.2.3"},
            "checksum": checksum,
            "public_key": hex_encode(signer.public_key().as_ref()),
            "signature": hex_encode(signature.as_ref()),
            "files": files.iter().map(|(path, source)| serde_json::json!({"path": path, "source": source})).collect::<Vec<_>>(),
        });
        let bytes = serde_json::to_vec(&artifact).unwrap();
        let verified = verify_artifact(&bytes).unwrap();
        assert_eq!(verified.name, "hello");
        assert_eq!(verified.version, "1.2.3");
        assert_eq!(verified.publisher, "acme");
        assert_eq!(verified.public_key, signer.public_key().as_ref());

        let mut tampered = artifact;
        tampered["files"][1]["source"] = Value::String("pub fn unsafe() {}".into());
        assert!(verify_artifact(&serde_json::to_vec(&tampered).unwrap()).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn artifact_verification_rejects_traversal_and_unsigned_identity_changes() {
        let mut value = serde_json::json!({"format_version": 1, "publisher": "x", "package": {"name": "x", "version": "1.0.0"}, "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000", "public_key": "00", "signature": "00", "files": [{"path": "../escape.zyl", "source": ""}]});
        assert!(verify_artifact(&serde_json::to_vec(&value).unwrap())
            .unwrap_err()
            .contains("public key"));
        value["public_key"] = Value::String(hex_encode(&[0; 32]));
        value["signature"] = Value::String(hex_encode(&[0; 64]));
        assert!(verify_artifact(&serde_json::to_vec(&value).unwrap())
            .unwrap_err()
            .contains("unsafe"));
    }

    #[test]
    fn trust_file_add_list_and_revoke_preserve_key_identity() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("zelyra-trust-{stamp}.toml"));
        let key = [7u8; 32];
        let mut document = read_trust_document(&path).unwrap();
        let entries = trusted_publishers_mut(&mut document).unwrap();
        let mut entry = Table::new();
        entry["publisher"] = value("acme");
        entry["public_key"] = value(hex_encode(&key));
        entries.push(entry);
        write_atomic(&path, document.to_string().as_bytes(), 0o600).unwrap();
        assert!(trusted_key(&path, "acme", &key).unwrap());
        assert!(!trusted_key(&path, "other", &key).unwrap());
        fs::remove_file(path).unwrap();
    }
}
