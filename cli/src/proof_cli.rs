use super::*;
use std::io::Write;
use zelyra_proof::{
    bound_path, deployment, json_bytes, read_bounded, sha256, FileHash, Observation, Outcome,
    Proof, Rule, MAX_INPUT_BYTES, PROOF_NAME,
};
use zelyra_runtime::ContractKind;

struct Snapshot {
    root: PathBuf,
    proof: Proof,
    contents: BTreeMap<String, Vec<u8>>,
}

pub(super) fn certify(arguments: &[String]) -> ExitCode {
    let (entry, output) = match arguments {
        [entry] => (entry, Path::new(PROOF_NAME)),
        [entry, flag, output] if flag == "--output" => (entry, Path::new(output)),
        _ => {
            usage();
            return ExitCode::from(2);
        }
    };
    report((|| {
        let snapshot = snapshot(entry)?;
        let bytes = json_bytes(&snapshot.proof)?;
        check_document_size(&bytes)?;
        write_new(output, &bytes)?;
        println!(
            "{} ({})",
            output.display(),
            snapshot.proof.snapshot_sha256()
        );
        Ok(())
    })())
}

pub(super) fn build_bundle(arguments: &[String]) -> ExitCode {
    let (entry, output, key) = match arguments {
        [entry, bundle, output_flag, output, signing_flag, key]
        | [bundle, entry, output_flag, output, signing_flag, key]
            if bundle == "--bundle"
                && output_flag == "--output"
                && signing_flag == "--signing-key" =>
        {
            (entry, output, key)
        }
        _ => {
            usage();
            return ExitCode::from(2);
        }
    };
    report(bundle(entry, Path::new(output), Path::new(key)))
}

fn report(result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error[E-PROOF-001]: {error}");
            ExitCode::FAILURE
        }
    }
}

fn snapshot(entry: &str) -> Result<Snapshot, String> {
    let source = fs::canonicalize(entry).map_err(|error| error.to_string())?;
    let source_root = parent(&source);
    let config_path = project_config_path(entry)?;
    let root = config_path.as_deref().map_or(source_root, parent);
    let prefix = source_root
        .strip_prefix(root)
        .map_err(|error| error.to_string())?;
    let config_prefix = portable_path(prefix)?;
    let mut contents = BTreeMap::new();
    if let Some(config) = &config_path {
        let config_name = portable_path(
            config
                .strip_prefix(root)
                .map_err(|error| error.to_string())?,
        )?;
        contents.insert(
            config_name.clone(),
            read_bounded(&bound_path(root, &config_name)?)?,
        );
    }
    let lock_name = format!(
        "{config_prefix}{}zelyra.lock",
        if prefix.as_os_str().is_empty() {
            ""
        } else {
            "/"
        }
    );
    if source_root.join("zelyra.lock").exists() {
        contents.insert(
            lock_name.clone(),
            read_bounded(&bound_path(root, &lock_name)?)?,
        );
    }
    let packages = packages::PackageGraph::load(source_root).map_err(|error| error.message)?;
    if let Some(graph) = packages
        .as_ref()
        .filter(|graph| graph.dependencies_present())
    {
        graph.check_lockfile().map_err(|error| error.message)?;
        for relative_path in graph.locked_source_paths().map_err(|error| error.message)? {
            let name = portable_path(&prefix.join(relative_path))?;
            if let std::collections::btree_map::Entry::Vacant(entry) = contents.entry(name) {
                let bytes = read_bounded(&bound_path(root, entry.key())?)?;
                entry.insert(bytes);
            }
        }
    }

    let loaded = load_project(entry).map_err(|()| "project source could not be loaded")?;
    let mut embedded_ui_sha256 = None;
    let mut entries = loaded.input_paths.iter().zip(&loaded.sources);
    let entry_name = entries
        .clone()
        .next()
        .and_then(|(path, _)| path.as_ref())
        .ok_or("missing entry source")?;
    let entry_name = portable_path(&prefix.join(entry_name))?;
    for (relative, source) in &mut entries {
        if let Some(relative) = relative {
            let path = portable_path(&prefix.join(relative))?;
            if let Some(previous) = contents.insert(path, source.text.as_bytes().to_vec()) {
                if previous != source.text.as_bytes() {
                    return Err("project source changed during certification".into());
                }
            }
        } else {
            embedded_ui_sha256 = Some(sha256(source.text.as_bytes()));
        }
    }
    let program =
        validate_loaded_project(entry, loaded).map_err(|()| "project validation failed")?;
    let mut groups = BTreeMap::<(String, Rule), Vec<Outcome>>::new();
    for result in verify_program(&program) {
        let rule = proof_rule(result.kind);
        let outcome = match result.status {
            VerificationStatus::Proven => Outcome::CompilerProven,
            VerificationStatus::RuntimeCheck => Outcome::RuntimeCheck,
            VerificationStatus::Unproven => Outcome::Unproven,
            VerificationStatus::Failed => Outcome::Failed,
        };
        if outcome == Outcome::Failed {
            return Err(format!(
                "failed verifier rule: {} ({})",
                result.function, result.kind
            ));
        }
        groups
            .entry((result.function, rule))
            .or_default()
            .push(outcome);
    }
    let observations = groups
        .into_iter()
        .flat_map(|((subject, rule), outcomes)| {
            outcomes
                .into_iter()
                .enumerate()
                .map(move |(index, outcome)| Observation {
                    subject: subject.clone(),
                    rule,
                    index,
                    outcome,
                    evidence: outcome.evidence(rule),
                })
        })
        .collect();
    if let Some(graph) = packages.filter(|graph| graph.dependencies_present()) {
        let current = packages::PackageGraph::load(source_root)
            .map_err(|error| error.message)?
            .ok_or("package manifest changed during certification")?;
        if current.lock_contents() != graph.lock_contents() {
            return Err("package graph changed during certification".into());
        }
    }
    let files = contents
        .iter()
        .map(|(name, bytes)| FileHash::new(name.clone(), bytes))
        .collect();
    let proof = Proof::new(
        env!("CARGO_PKG_VERSION"),
        entry_name,
        files,
        embedded_ui_sha256,
        observations,
    )?;
    proof.check(root)?;
    if project_config_path(entry)? != config_path
        || contents.contains_key(&lock_name) != source_root.join("zelyra.lock").exists()
    {
        return Err("project configuration changed during certification".into());
    }
    Ok(Snapshot {
        root: root.to_owned(),
        proof,
        contents,
    })
}

pub(super) fn proof_rule(kind: ContractKind) -> Rule {
    match kind {
        ContractKind::Requires => Rule::Requires,
        ContractKind::Ensures => Rule::Ensures,
        ContractKind::LoopInvariant => Rule::Invariant,
        ContractKind::StateMachine => Rule::StateMachine,
        ContractKind::Workflow => Rule::Workflow,
        ContractKind::Authorization => Rule::Authorization,
        ContractKind::Privacy => Rule::Privacy,
    }
}

fn portable_path(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(|path| path.replace(std::path::MAIN_SEPARATOR, "/"))
        .ok_or_else(|| "project input paths must be UTF-8".into())
}

fn parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn check_document_size(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err("document exceeds the v1 size limit".into());
    }
    Ok(())
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(format!("{}: {error}", path.display()));
    }
    Ok(())
}

fn bundle(entry: &str, output: &Path, key_path: &Path) -> Result<(), String> {
    let snapshot = snapshot(entry)?;
    let key_path = fs::canonicalize(key_path).map_err(|error| format!("signing key: {error}"))?;
    if snapshot
        .contents
        .keys()
        .any(|name| snapshot.root.join(name) == key_path)
    {
        return Err("signing key must be separate from all deployment inputs".into());
    }
    let key = zeroize::Zeroizing::new(read_bounded(&key_path)?);
    let output_name = output.file_name().ok_or("missing output directory name")?;
    let output_parent =
        fs::canonicalize(parent(output)).map_err(|error| format!("output parent: {error}"))?;
    let output = output_parent.join(output_name);
    let sbom = deployment::spdx(&snapshot.proof, |name| Ok(snapshot.contents[name].clone()))?;
    let mut files = snapshot
        .contents
        .into_iter()
        .map(|(name, bytes)| (format!("{}/{name}", deployment::PROJECT), bytes))
        .collect::<BTreeMap<_, _>>();
    files.insert(PROOF_NAME.into(), json_bytes(&snapshot.proof)?);
    files.insert(deployment::SBOM.into(), json_bytes(&sbom)?);
    let manifest_files = files
        .iter()
        .map(|(path, bytes)| FileHash::new(path.clone(), bytes))
        .collect();
    let entry_path = format!("{}/{}", deployment::PROJECT, snapshot.proof.entry);
    let manifest = json_bytes(&deployment::Manifest::new(
        entry_path.clone(),
        manifest_files,
    ))?;
    let attestation = deployment::Attestation::sign(&manifest, &key)?;
    files.insert(
        deployment::CHECKSUM.into(),
        deployment::checksum_file(&manifest).into_bytes(),
    );
    files.insert(deployment::MANIFEST.into(), manifest);
    files.insert(deployment::ATTESTATION.into(), json_bytes(&attestation)?);

    let mut random = [0; 16];
    rand_core::OsRng
        .try_fill_bytes(&mut random)
        .map_err(|error| error.to_string())?;
    let staging = output_parent.join(format!(".zelyra-deployment-{}", zelyra_proof::hex(&random)));
    fs::create_dir(&staging).map_err(|error| error.to_string())?;
    let result = (|| {
        for (name, bytes) in files {
            check_document_size(&bytes)?;
            let path = staging.join(name);
            fs::create_dir_all(parent(&path)).map_err(|error| error.to_string())?;
            write_new(&path, &bytes)?;
        }
        deployment::check(&staging, &attestation.public_key)?;
        module_cli::publish_directory_no_replace(&staging, &output)
            .map_err(|error| format!("{}: {error}", output.display()))
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result?;
    println!(
        "{}",
        json!({"entry": entry_path, "public_key": attestation.public_key, "snapshot_sha256": snapshot.proof.snapshot_sha256()})
    );
    Ok(())
}
