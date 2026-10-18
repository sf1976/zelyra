use super::*;
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};
use serde_json::Value;
use std::fs::File;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use toml_edit::{value, ArrayOfTables, DocumentMut, Item, Table};
use url::Url;
use zeroize::Zeroize;

const PACKAGE_FORMAT: u64 = 1;
const MAX_PACKAGE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_PACKAGE_FILES: usize = 4096;
const MAX_REGISTRY_ARTIFACTS: usize = 10_000;
const MAX_REGISTRY_BYTES: u64 = 256 * 1024 * 1024;
const MAX_TRUST_BYTES: u64 = 1024 * 1024;
const DEFAULT_TRUST_FILE: &str = "zelyra.trust.toml";
const PACKAGE_TRANSACTION_DIR: &str = ".zelyra/package-transaction";
const PACKAGE_OPERATION_LOCK: &str = ".zelyra/package-operation.lock";
const PACKAGE_JOURNAL_VERSION: u64 = 1;
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

pub(super) fn command(
    action: &str,
    mut arguments: impl Iterator<Item = String>,
) -> std::process::ExitCode {
    use std::process::ExitCode;
    let _package_lock = if matches!(action, "trust" | "add" | "update" | "remove" | "cache") {
        match PackageOperationLock::acquire_current().and_then(|lock| {
            lock.require_no_pending_transaction()?;
            Ok(lock)
        }) {
            Ok(lock) => Some(lock),
            Err(error) => {
                eprintln!("error[E-PKG-003]: {error}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        None
    };
    let result = match action {
        "keygen" => keygen_command(&mut arguments),
        "sign" => sign_command(&mut arguments),
        "verify" => verify_command(&mut arguments),
        "trust" => trust_command(&mut arguments),
        "registry" => registry_command(&mut arguments),
        "search" => search_command(&mut arguments),
        "add" => add_command(&mut arguments, false),
        "update" => {
            let mut arguments = arguments.peekable();
            if arguments
                .peek()
                .is_some_and(|argument| argument == "--plan")
            {
                arguments.next();
                update_plan_command(&mut arguments)
            } else {
                add_command(&mut arguments, true)
            }
        }
        "remove" => remove_command(&mut arguments),
        "cache" => cache_command(&mut arguments),
        "recover" => recover_command(&mut arguments),
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

/// Serializes package mutations across processes. The OS releases this lock if
/// Zelyra exits unexpectedly, while the journal below preserves transaction
/// recovery information.
#[derive(Debug)]
pub(super) struct PackageOperationLock {
    _file: File,
    workspace: PathBuf,
}

impl PackageOperationLock {
    pub(super) fn acquire_current() -> Result<Self, String> {
        let current = std::env::current_dir()
            .map_err(|error| format!("cannot read current directory: {error}"))?;
        let workspace = fs::canonicalize(current)
            .map_err(|error| format!("cannot resolve project directory: {error}"))?;
        Self::acquire(&workspace)
    }

    fn acquire(workspace: &Path) -> Result<Self, String> {
        let lock_path = workspace.join(PACKAGE_OPERATION_LOCK);
        ensure_no_symlink_path(workspace, &lock_path)?;
        let parent = lock_path
            .parent()
            .ok_or_else(|| "package operation lock has no parent directory".to_owned())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create package state directory: {error}"))?;
        ensure_no_symlink_path(workspace, parent)?;
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.custom_flags(0x0020_0000); // FILE_FLAG_OPEN_REPARSE_POINT
        }
        let file = options
            .open(&lock_path)
            .map_err(|error| format!("cannot open package operation lock: {error}"))?;
        let metadata = file
            .metadata()
            .map_err(|error| format!("cannot inspect package operation lock: {error}"))?;
        if !metadata.is_file() {
            return Err("package operation lock must be a regular file".into());
        }
        match file.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => {
                return Err("another Zelyra package operation is active for this project".into())
            }
            Err(std::fs::TryLockError::Error(error)) => {
                return Err(format!("cannot lock package operations: {error}"))
            }
        }
        Ok(Self {
            _file: file,
            workspace: workspace.to_path_buf(),
        })
    }

    pub(super) fn require_no_pending_transaction(&self) -> Result<(), String> {
        let transaction_dir = self.workspace.join(PACKAGE_TRANSACTION_DIR);
        match fs::symlink_metadata(&transaction_dir) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("cannot inspect package recovery journal: {error}")),
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                Err("package recovery path must be a real directory".into())
            }
            Ok(_) => Err("an interrupted package transaction is pending; review `zelyra package recover` before starting another package operation".into()),
        }
    }
}

struct PackageTransaction {
    workspace: PathBuf,
    directory: PathBuf,
    operation: String,
    cleanup_paths: Vec<String>,
    phase: &'static str,
}

impl PackageTransaction {
    fn prepare(
        workspace: &Path,
        operation: &str,
        original_manifest: &[u8],
        original_lock: Option<&[u8]>,
        cleanup_paths: &[PathBuf],
    ) -> Result<Self, String> {
        validate_transaction_operation(operation)?;
        let directory = workspace.join(PACKAGE_TRANSACTION_DIR);
        let state_root = directory
            .parent()
            .ok_or_else(|| "package recovery journal has no state directory".to_owned())?;
        ensure_no_symlink_path(workspace, state_root)?;
        fs::create_dir_all(state_root)
            .map_err(|error| format!("cannot create package state directory: {error}"))?;
        ensure_no_symlink_path(workspace, state_root)?;
        ensure_no_symlink_path(workspace, &directory)?;
        if directory.exists() {
            return Err(
                "an interrupted package transaction is pending; run `zelyra package recover` first"
                    .into(),
            );
        }
        let cleanup_paths = cleanup_paths
            .iter()
            .map(|path| package_cleanup_relative_path(workspace, path))
            .collect::<Result<Vec<_>, _>>()?;
        fs::create_dir(&directory)
            .map_err(|error| format!("cannot create package recovery journal: {error}"))?;
        let prepare_result = (|| {
            write_new_file(
                &directory.join("manifest.backup"),
                original_manifest,
                0o600,
                "package manifest backup",
            )?;
            if let Some(lock) = original_lock {
                write_new_file(
                    &directory.join("lock.backup"),
                    lock,
                    0o600,
                    "package lockfile backup",
                )?;
            }
            let journal = serde_json::json!({
                "journal_version": PACKAGE_JOURNAL_VERSION,
                "operation": operation,
                "phase": "prepared",
                "has_lock_backup": original_lock.is_some(),
                "cleanup_paths": cleanup_paths,
            });
            let bytes = serde_json::to_vec(&journal)
                .map_err(|error| format!("cannot encode package recovery journal: {error}"))?;
            write_new_file(
                &directory.join("journal.json"),
                &bytes,
                0o600,
                "package recovery journal",
            )?;
            sync_directory(&directory)?;
            if let Some(parent) = directory.parent() {
                sync_directory(parent)?;
            }
            Ok::<(), String>(())
        })();
        if let Err(error) = prepare_result {
            // No project files are changed before prepare returns successfully.
            let cleanup = fs::remove_dir_all(&directory);
            return Err(match cleanup {
                Ok(()) => error,
                Err(cleanup_error) => format!(
                    "{error}; incomplete recovery data remains at {} ({cleanup_error})",
                    directory.display()
                ),
            });
        }
        Ok(Self {
            workspace: workspace.to_path_buf(),
            directory,
            operation: operation.to_owned(),
            cleanup_paths,
            phase: "prepared",
        })
    }

    fn commit(&mut self) -> Result<(), String> {
        let journal_path = self.directory.join("journal.json");
        let journal = serde_json::json!({
            "journal_version": PACKAGE_JOURNAL_VERSION,
            "operation": &self.operation,
            "phase": "committed",
            "has_lock_backup": self.directory.join("lock.backup").is_file(),
            "cleanup_paths": &self.cleanup_paths,
        });
        let bytes = serde_json::to_vec(&journal)
            .map_err(|error| format!("cannot encode committed package journal: {error}"))?;
        write_atomic(&journal_path, &bytes, 0o600)?;
        sync_directory(&self.directory)?;
        self.phase = "committed";
        self.finish()
    }

    fn rollback(&mut self) -> Result<(), String> {
        recover_transaction_impl(&self.workspace, true, false)?;
        self.phase = "rolled_back";
        Ok(())
    }

    fn finish(&mut self) -> Result<(), String> {
        fs::remove_dir_all(&self.directory)
            .map_err(|error| format!("cannot remove completed package journal: {error}"))?;
        if let Some(parent) = self.directory.parent() {
            sync_directory(parent)?;
        }
        Ok(())
    }
}

impl Drop for PackageTransaction {
    fn drop(&mut self) {
        // A prepared journal intentionally survives unexpected or unhandled
        // failures. Normal paths call commit or rollback explicitly.
        let _ = self.phase;
    }
}

fn validate_transaction_operation(operation: &str) -> Result<(), String> {
    if matches!(operation, "add" | "update" | "remove") {
        Ok(())
    } else {
        Err("unsupported package transaction operation".into())
    }
}

fn package_cleanup_relative_path(workspace: &Path, path: &Path) -> Result<String, String> {
    let relative = path
        .strip_prefix(workspace)
        .map_err(|_| "package recovery target is outside the project".to_owned())?;
    let mut components = relative.components();
    if components.next().and_then(|part| part.as_os_str().to_str()) != Some(".zelyra")
        || components.next().and_then(|part| part.as_os_str().to_str()) != Some("packages")
    {
        return Err("package recovery targets must be under `.zelyra/packages`".into());
    }
    let text = relative
        .to_str()
        .ok_or_else(|| "package recovery path is not UTF-8".to_owned())?
        .replace('\\', "/");
    if !safe_relative_path(&text).is_some() {
        return Err("package recovery path is not a safe relative path".into());
    }
    Ok(text)
}

fn recover_command(args: &mut impl Iterator<Item = String>) -> Result<String, String> {
    let mut apply = false;
    for argument in args {
        match argument.as_str() {
            "--apply" if !apply => apply = true,
            _ => return Err("usage: zelyra package recover [--apply]".into()),
        }
    }
    let lock = PackageOperationLock::acquire_current()?;
    recover_transaction(&lock.workspace, apply)
}

fn recover_transaction(workspace: &Path, apply: bool) -> Result<String, String> {
    recover_transaction_impl(workspace, apply, true)
}

fn recover_transaction_impl(
    workspace: &Path,
    apply: bool,
    include_cache_quarantines: bool,
) -> Result<String, String> {
    let directory = workspace.join(PACKAGE_TRANSACTION_DIR);
    match fs::symlink_metadata(&directory) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return if include_cache_quarantines {
                recover_cache_quarantines(workspace, apply)
            } else {
                Ok("no interrupted package transaction is pending".into())
            }
        }
        Err(error) => return Err(format!("cannot inspect package recovery journal: {error}")),
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err("package recovery path must be a real directory".into())
        }
        Ok(_) => {}
    }
    ensure_no_symlinks_in_tree(&directory)?;
    let journal_path = directory.join("journal.json");
    let journal = match read_regular_file(&journal_path, 1024 * 1024, "package recovery journal") {
        Ok(bytes) => serde_json::from_slice::<Value>(&bytes)
            .map_err(|error| format!("invalid package recovery journal: {error}"))?,
        Err(error) if !journal_path.exists() => {
            validate_journal_storage(&directory, false, false)?;
            if !apply {
                return Ok(format!("incomplete package journal contains no project mutation; run `zelyra package recover --apply` to remove {}", directory.display()));
            }
            fs::remove_dir_all(&directory).map_err(|cleanup| {
                format!("{error}; cannot remove incomplete journal: {cleanup}")
            })?;
            return Ok("removed an incomplete journal; no project mutation had started".into());
        }
        Err(error) => return Err(error),
    };
    validate_recovery_journal(&journal)?;
    let phase = journal
        .get("phase")
        .and_then(Value::as_str)
        .ok_or_else(|| "package recovery journal has no phase".to_owned())?;
    let has_lock_backup = journal
        .get("has_lock_backup")
        .and_then(Value::as_bool)
        .ok_or_else(|| "package recovery journal has invalid lock backup state".to_owned())?;
    validate_journal_storage(&directory, true, has_lock_backup)?;
    if !apply {
        return Ok(if phase == "committed" {
            "package transaction is committed; run `zelyra package recover --apply` to remove its completed journal".into()
        } else {
            "interrupted package transaction will restore the previous manifest and lockfile and remove its new cache paths; review, then run `zelyra package recover --apply`".into()
        });
    }
    if phase == "prepared" {
        let manifest = read_regular_file(
            &directory.join("manifest.backup"),
            4 * 1024 * 1024,
            "package manifest backup",
        )?;
        let lock_path = workspace.join(LOCKFILE);
        reject_output_target(&lock_path, "package lockfile")?;
        let lock = if has_lock_backup {
            let lock = read_regular_file(
                &directory.join("lock.backup"),
                16 * 1024 * 1024,
                "package lockfile backup",
            )?;
            Some(lock)
        } else {
            None
        };
        let cleanup_targets = journal["cleanup_paths"]
            .as_array()
            .expect("validated recovery paths")
            .iter()
            .map(|relative| workspace.join(relative.as_str().expect("validated string path")))
            .collect::<Vec<_>>();
        for target in &cleanup_targets {
            ensure_no_symlink_path(workspace, target)?;
            match fs::symlink_metadata(target) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(format!(
                        "recovery refuses symlinked package path {}",
                        target.display()
                    ))
                }
                Ok(metadata) if metadata.is_dir() => ensure_no_symlinks_in_tree(target)?,
                Ok(_) => {
                    return Err(format!(
                        "recovery target {} is not a package directory",
                        target.display()
                    ))
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(format!("cannot inspect package recovery target: {error}"))
                }
            }
        }
        write_atomic(&workspace.join(MANIFEST), &manifest, 0o644)?;
        if let Some(lock) = lock {
            write_atomic(&lock_path, &lock, 0o644)?;
        } else if lock_path.exists() {
            fs::remove_file(&lock_path)
                .map_err(|error| format!("cannot remove interrupted package lockfile: {error}"))?;
        }
        for target in cleanup_targets {
            if target.is_dir() {
                fs::remove_dir_all(&target).map_err(|error| {
                    format!(
                        "cannot remove interrupted package path {}: {error}",
                        target.display()
                    )
                })?;
            }
        }
    }
    fs::remove_dir_all(&directory)
        .map_err(|error| format!("cannot remove recovered package journal: {error}"))?;
    let transaction_result = if phase == "committed" {
        "removed the journal for a completed package transaction".into()
    } else {
        "restored the previous package manifest and lockfile and removed newly installed package paths".into()
    };
    if !include_cache_quarantines {
        return Ok(transaction_result);
    }
    let cache_result = recover_cache_quarantines(workspace, apply)?;
    if cache_result == "no interrupted package-cache cleanup is pending" {
        Ok(transaction_result)
    } else {
        Ok(format!("{transaction_result}; {cache_result}"))
    }
}

fn recover_cache_quarantines(workspace: &Path, apply: bool) -> Result<String, String> {
    let state_root = workspace.join(".zelyra");
    match fs::symlink_metadata(&state_root) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok("no interrupted package-cache cleanup is pending".into())
        }
        Err(error) => return Err(format!("cannot inspect package state directory: {error}")),
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err("package state directory must be a real directory".into())
        }
        Ok(_) => {}
    }
    let mut quarantine_paths = fs::read_dir(&state_root)
        .map_err(|error| format!("cannot inspect package state directory: {error}"))?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|error| format!("cannot inspect package state entry: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    quarantine_paths.sort();
    quarantine_paths.retain(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("package-cache-trash-"))
    });
    if quarantine_paths.is_empty() {
        return Ok("no interrupted package-cache cleanup is pending".into());
    }
    let cache_root = state_root.join("packages");
    let mut recoveries = Vec::<(PathBuf, PathBuf)>::new();
    let mut empty_quarantines = Vec::new();
    for quarantine in &quarantine_paths {
        let recoveries_before = recoveries.len();
        let metadata = fs::symlink_metadata(quarantine)
            .map_err(|error| format!("cannot inspect package-cache quarantine: {error}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(format!(
                "package-cache quarantine {} must be a real directory",
                quarantine.display()
            ));
        }
        ensure_no_symlinks_in_tree(quarantine)?;
        let mut package_names = fs::read_dir(quarantine)
            .map_err(|error| format!("cannot inspect package-cache quarantine: {error}"))?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|error| format!("cannot inspect quarantined package: {error}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        package_names.sort();
        for name_dir in package_names {
            let name = name_dir
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| "quarantined package name is not UTF-8".to_owned())?;
            validate_package_name(name).map_err(|error| error.message)?;
            let metadata = fs::symlink_metadata(&name_dir)
                .map_err(|error| format!("cannot inspect quarantined package: {error}"))?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(format!(
                    "quarantined package path {} must be a real directory",
                    name_dir.display()
                ));
            }
            let mut versions = fs::read_dir(&name_dir)
                .map_err(|error| format!("cannot inspect quarantined package versions: {error}"))?
                .map(|entry| {
                    entry.map(|entry| entry.path()).map_err(|error| {
                        format!("cannot inspect quarantined package version: {error}")
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            versions.sort();
            for version_dir in versions {
                let version = version_dir
                    .file_name()
                    .and_then(|version| version.to_str())
                    .ok_or_else(|| "quarantined package version is not UTF-8".to_owned())?;
                let parsed = Version::parse(version).map_err(|error| {
                    format!("invalid quarantined package version `{version}`: {error}")
                })?;
                if parsed.to_string() != version {
                    return Err(format!(
                        "quarantined package version `{version}` is not normalized SemVer"
                    ));
                }
                let metadata = fs::symlink_metadata(&version_dir).map_err(|error| {
                    format!("cannot inspect quarantined package version: {error}")
                })?;
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(format!(
                        "quarantined package version {} must be a real directory",
                        version_dir.display()
                    ));
                }
                ensure_no_symlinks_in_tree(&version_dir)?;
                let target = cache_root.join(name).join(version);
                ensure_no_symlink_path(workspace, &target)?;
                if target.exists() {
                    return Err(format!(
                        "cannot recover quarantined package `{name}` {version}: cache destination already exists"
                    ));
                }
                recoveries.push((version_dir, target));
            }
        }
        if recoveries.len() == recoveries_before {
            empty_quarantines.push(quarantine.clone());
        }
    }
    if !apply {
        return Ok(format!(
            "{} quarantined package version(s) will be restored; run `zelyra package recover --apply`",
            recoveries.len()
        ));
    }
    let mut restored = Vec::<(PathBuf, PathBuf)>::new();
    for (source, target) in &recoveries {
        let parent = target.parent().expect("package cache target has a parent");
        ensure_no_symlink_path(workspace, parent)?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot restore package cache directory: {error}"))?;
        ensure_no_symlink_path(workspace, parent)?;
        if let Err(error) = fs::rename(source, target) {
            let mut rollback_errors = Vec::new();
            for (original, restored_path) in restored.iter().rev() {
                if let Err(rollback) = fs::rename(restored_path, original) {
                    rollback_errors.push(format!("{}: {rollback}", original.display()));
                }
            }
            return Err(if rollback_errors.is_empty() {
                format!(
                    "cannot restore quarantined package {}: {error}",
                    source.display()
                )
            } else {
                format!(
                    "cannot restore quarantined package {}: {error}; rollback incomplete for {}",
                    source.display(),
                    rollback_errors.join(", ")
                )
            });
        }
        restored.push((source.clone(), target.clone()));
    }
    for quarantine in &quarantine_paths {
        fs::remove_dir_all(quarantine).map_err(|error| {
            format!(
                "cannot remove recovered package quarantine {}: {error}",
                quarantine.display()
            )
        })?;
    }
    Ok(if recoveries.is_empty() {
        format!(
            "removed {} empty package-cache cleanup quarantine(s)",
            empty_quarantines.len()
        )
    } else {
        format!(
            "restored {} quarantined package version(s)",
            recoveries.len()
        )
    })
}

fn validate_journal_storage(
    directory: &Path,
    has_journal: bool,
    has_lock_backup: bool,
) -> Result<(), String> {
    let mut expected = BTreeSet::new();
    let mut allowed = BTreeSet::from(["manifest.backup", "lock.backup"]);
    if has_journal {
        expected.insert("manifest.backup");
        allowed.insert("journal.json");
        expected.insert("journal.json");
        if has_lock_backup {
            expected.insert("lock.backup");
        }
    }
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("cannot inspect package recovery data: {error}"))?
    {
        let path = entry
            .map_err(|error| format!("cannot inspect package recovery data: {error}"))?
            .path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "package recovery file name is not UTF-8".to_owned())?;
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect package recovery file: {error}"))?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || !allowed.contains(name)
            || (has_journal && !expected.remove(name))
        {
            return Err(format!(
                "package recovery data contains an unknown or unsafe file: {name}"
            ));
        }
    }
    if has_journal && !expected.is_empty() {
        return Err(format!(
            "package recovery data is incomplete; missing {}",
            expected.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    Ok(())
}

fn validate_recovery_journal(journal: &Value) -> Result<(), String> {
    let object = journal
        .as_object()
        .ok_or_else(|| "package recovery journal must be a JSON object".to_owned())?;
    if object.len() != 5
        || object.get("journal_version").and_then(Value::as_u64) != Some(PACKAGE_JOURNAL_VERSION)
        || !matches!(
            object.get("operation").and_then(Value::as_str),
            Some("add" | "update" | "remove" | "package mutation")
        )
        || !matches!(
            object.get("phase").and_then(Value::as_str),
            Some("prepared" | "committed")
        )
        || object
            .get("has_lock_backup")
            .and_then(Value::as_bool)
            .is_none()
    {
        return Err("unsupported or malformed package recovery journal".into());
    }
    let paths = object
        .get("cleanup_paths")
        .and_then(Value::as_array)
        .ok_or_else(|| "package recovery journal has no cleanup paths".to_owned())?;
    if paths.len() > 256 {
        return Err("package recovery journal exceeds the 256 path limit".into());
    }
    let mut unique = BTreeSet::new();
    for path in paths {
        let path = path
            .as_str()
            .ok_or_else(|| "package recovery path must be a string".to_owned())?;
        let relative =
            safe_relative_path(path).ok_or_else(|| "package recovery path is unsafe".to_owned())?;
        if relative.to_string_lossy().replace('\\', "/") != path
            || !path.starts_with(".zelyra/packages/")
            || !unique.insert(path)
        {
            return Err("package recovery path is invalid or duplicated".into());
        }
    }
    Ok(())
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("cannot sync package transaction directory: {error}"))
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn search_command(args: &mut impl Iterator<Item = String>) -> Result<String, String> {
    let registry_text = args.next().ok_or_else(|| {
        "usage: zelyra package search <registry-index-url> [name-filter]".to_owned()
    })?;
    let filter = args.next().map(|value| value.to_ascii_lowercase());
    if args.next().is_some() {
        return Err("usage: zelyra package search <registry-index-url> [name-filter]".into());
    }
    let registry_url = parse_registry_url(&registry_text)?;
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .max_redirects(0)
        .build()
        .into();
    let index_bytes = fetch_limited(&agent, registry_url.as_str(), 4 * 1024 * 1024)?;
    let index: Value = serde_json::from_slice(&index_bytes)
        .map_err(|error| format!("invalid package registry index: {error}"))?;
    let entries = parse_registry_index(&index)?;
    let matches = entries
        .iter()
        .filter(|entry| {
            filter
                .as_deref()
                .is_none_or(|filter| entry.name.to_ascii_lowercase().contains(filter))
        })
        .collect::<Vec<_>>();
    let mut output = vec![
        "Registry metadata is untrusted; `package add` verifies publisher trust and signatures."
            .to_owned(),
    ];
    if matches.is_empty() {
        output.push("no matching packages".into());
    } else {
        output.extend(matches.into_iter().map(|entry| {
            format!(
                "{} {}  publisher: {}",
                entry.name, entry.version, entry.publisher
            )
        }));
    }
    Ok(output.join("\n"))
}

fn cache_command(args: &mut impl Iterator<Item = String>) -> Result<String, String> {
    if args.next().as_deref() != Some("clean") {
        return Err("usage: zelyra package cache clean [--apply]".into());
    }
    let mut apply = false;
    for argument in args {
        match argument.as_str() {
            "--apply" if !apply => apply = true,
            _ => return Err("usage: zelyra package cache clean [--apply]".into()),
        }
    }
    clean_package_cache(apply)
}

fn clean_package_cache(apply: bool) -> Result<String, String> {
    let workspace = std::env::current_dir()
        .map_err(|error| format!("cannot read current directory: {error}"))?;
    let workspace = fs::canonicalize(workspace)
        .map_err(|error| format!("cannot resolve project directory: {error}"))?;
    let graph = PackageGraph::load(&workspace)
        .map_err(|error| error.message)?
        .ok_or_else(|| "package cache clean requires a project [package] table".to_owned())?;
    graph.check_lockfile().map_err(|error| {
        format!(
            "{}; run `zelyra package lock --update` after reviewing dependency changes",
            error.message
        )
    })?;
    let active = graph
        .inventory()
        .into_iter()
        .filter(|package| package.path.starts_with(".zelyra/packages/"))
        .map(|package| package.path)
        .collect::<BTreeSet<_>>();
    let cache_root = workspace.join(".zelyra/packages");
    ensure_no_symlink_path(&workspace, &cache_root)?;
    match fs::symlink_metadata(&cache_root) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok("package cache is empty".into())
        }
        Err(error) => return Err(format!("cannot inspect package cache: {error}")),
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err("package cache root must be a real directory".into())
        }
        Ok(_) => {}
    }
    let mut candidates = Vec::new();
    let mut names = fs::read_dir(&cache_root)
        .map_err(|error| format!("cannot read package cache: {error}"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot inspect package cache: {error}"))?;
    names.sort();
    for name_dir in names {
        let name_meta = fs::symlink_metadata(&name_dir)
            .map_err(|error| format!("cannot inspect package cache path: {error}"))?;
        if name_meta.file_type().is_symlink() {
            return Err(format!(
                "package cache may not contain symlinks: {}",
                name_dir.display()
            ));
        }
        if !name_meta.is_dir() {
            continue;
        }
        let mut versions = fs::read_dir(&name_dir)
            .map_err(|error| format!("cannot read cached package versions: {error}"))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("cannot inspect cached package versions: {error}"))?;
        versions.sort();
        for version_dir in versions {
            let metadata = fs::symlink_metadata(&version_dir)
                .map_err(|error| format!("cannot inspect cached package: {error}"))?;
            if metadata.file_type().is_symlink() {
                return Err(format!(
                    "package cache may not contain symlinks: {}",
                    version_dir.display()
                ));
            }
            if !metadata.is_dir() {
                continue;
            }
            let manifest = version_dir.join(MANIFEST);
            match fs::symlink_metadata(&manifest) {
                Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => {}
                Ok(_) => continue,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(format!("cannot inspect cached package manifest: {error}"))
                }
            }
            let relative = version_dir
                .strip_prefix(&workspace)
                .map_err(|_| "package cache path escaped the project workspace".to_owned())?
                .to_string_lossy()
                .replace('\\', "/");
            if !active.contains(&relative) {
                ensure_no_symlinks_in_tree(&version_dir)?;
                candidates.push((relative, version_dir));
            }
        }
    }
    if candidates.is_empty() {
        return Ok(
            "package cache is clean; all cached package versions are referenced by zelyra.lock"
                .into(),
        );
    }
    if !apply {
        let mut lines = vec![format!(
            "{} unused package version(s) found (dry run):",
            candidates.len()
        )];
        lines.extend(
            candidates
                .iter()
                .map(|(relative, _)| format!("  {relative}")),
        );
        lines.push("review the list, then run `zelyra package cache clean --apply` to remove these sources".into());
        return Ok(lines.join("\n"));
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock is before Unix epoch: {error}"))?
        .as_nanos();
    let quarantine = workspace.join(".zelyra").join(format!(
        "package-cache-trash-{}-{stamp}",
        std::process::id()
    ));
    ensure_no_symlink_path(&workspace, &quarantine)?;
    fs::create_dir(&quarantine)
        .map_err(|error| format!("cannot create package cache quarantine: {error}"))?;
    for (relative, _) in &candidates {
        let destination = quarantine.join(
            relative
                .strip_prefix(".zelyra/packages/")
                .unwrap_or(relative),
        );
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot prepare package cache quarantine: {error}"))?;
        }
    }
    let mut moved = Vec::<(PathBuf, PathBuf)>::new();
    for (relative, source) in &candidates {
        let destination = quarantine.join(
            relative
                .strip_prefix(".zelyra/packages/")
                .unwrap_or(relative),
        );
        if let Err(error) = fs::rename(source, &destination) {
            let mut recovery_errors = Vec::new();
            for (original, staged) in moved.iter().rev() {
                if let Err(rollback_error) = fs::rename(staged, original) {
                    recovery_errors.push(format!("{}: {rollback_error}", original.display()));
                }
            }
            if recovery_errors.is_empty() {
                let _ = fs::remove_dir_all(&quarantine);
                return Err(format!(
                    "cannot stage unused package `{relative}` for removal: {error}; prior moves were restored"
                ));
            }
            return Err(format!(
                "cannot stage unused package `{relative}` for removal: {error}; rollback was incomplete for {}; recoverable files remain under {}",
                recovery_errors.join(", "),
                quarantine.display()
            ));
        }
        moved.push((source.clone(), destination));
    }
    if let Err(error) = fs::remove_dir_all(&quarantine) {
        return Err(format!("unused packages were moved out of the active cache but quarantine cleanup failed: {error}; recoverable files remain under {}", quarantine.display()));
    }
    Ok(format!(
        "removed {} unused cached package version(s); active package sources were preserved",
        candidates.len()
    ))
}

fn ensure_no_symlinks_in_tree(root: &Path) -> Result<(), String> {
    for entry in
        fs::read_dir(root).map_err(|error| format!("cannot inspect package tree: {error}"))?
    {
        let path = entry
            .map_err(|error| format!("cannot inspect package tree entry: {error}"))?
            .path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect package tree entry: {error}"))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "package cache cleanup refuses symlinked package content: {}",
                path.display()
            ));
        }
        if metadata.is_dir() {
            ensure_no_symlinks_in_tree(&path)?;
        }
    }
    Ok(())
}

fn remove_command(args: &mut impl Iterator<Item = String>) -> Result<String, String> {
    let alias = args
        .next()
        .ok_or_else(|| "usage: zelyra package remove <dependency-alias>".to_owned())?;
    if args.next().is_some() {
        return Err("usage: zelyra package remove <dependency-alias>".into());
    }
    validate_dependency_alias(&alias)?;
    let workspace = std::env::current_dir()
        .map_err(|error| format!("cannot read current directory: {error}"))?;
    let workspace = fs::canonicalize(workspace)
        .map_err(|error| format!("cannot resolve project directory: {error}"))?;
    let manifest_path = workspace.join(MANIFEST);
    let original_manifest = read_regular_file(&manifest_path, 4 * 1024 * 1024, "project manifest")?;
    let mut document = std::str::from_utf8(&original_manifest)
        .map_err(|_| "project manifest must be UTF-8".to_owned())?
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid project manifest: {error}"))?;
    let no_dependencies = {
        let dependencies = document
            .get_mut("dependencies")
            .and_then(Item::as_table_mut)
            .ok_or_else(|| format!("project dependency alias `{alias}` does not exist"))?;
        if dependencies.remove(&alias).is_none() {
            return Err(format!("project dependency alias `{alias}` does not exist"));
        }
        dependencies.is_empty()
    };
    if no_dependencies {
        document.remove("dependencies");
    }
    let no_sources = if let Some(sources) = document
        .get_mut("package_sources")
        .and_then(Item::as_table_mut)
    {
        sources.remove(&alias);
        sources.is_empty()
    } else {
        false
    };
    if no_sources {
        document.remove("package_sources");
    }
    let lock_path = workspace.join(LOCKFILE);
    reject_output_target(&lock_path, "package lockfile")?;
    let previous_lock = match fs::read(&lock_path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(format!("cannot read existing package lockfile: {error}")),
    };
    let entry = document
        .get("package")
        .and_then(Item::as_table)
        .and_then(|package| package.get("entry"))
        .and_then(Item::as_str)
        .unwrap_or("main.zyl");
    let entry_relative = safe_relative_path(entry)
        .ok_or_else(|| "project package entry must be a safe relative path".to_owned())?;
    let entry_path = workspace.join(entry_relative);
    let entry_path = fs::canonicalize(&entry_path)
        .map_err(|error| format!("cannot resolve project entry for validation: {error}"))?;
    if !entry_path.starts_with(&workspace) || !entry_path.is_file() {
        return Err("project entry resolves outside the workspace".into());
    }
    let mut transaction = PackageTransaction::prepare(
        &workspace,
        "remove",
        &original_manifest,
        previous_lock.as_deref(),
        &[],
    )?;
    let result = (|| {
        write_atomic(&manifest_path, document.to_string().as_bytes(), 0o644)?;
        let graph = PackageGraph::load(&workspace)
            .map_err(|error| error.message)?
            .ok_or_else(|| "project package graph disappeared after manifest update".to_owned())?;
        write_lockfile(&graph, true).map_err(|error| error.message)?;
        let entry = entry_path
            .to_str()
            .ok_or_else(|| "project entry path is not UTF-8".to_owned())?;
        super::super::validate(entry).map_err(|_| {
            "project compiler validation failed after dependency removal".to_owned()
        })?;
        Ok::<(), String>(())
    })();
    if let Err(error) = result {
        return Err(match transaction.rollback() {
            Ok(()) => error,
            Err(recovery) => format!("{error}; package recovery is pending: {recovery}"),
        });
    }
    transaction.commit()?;
    Ok(format!("removed package dependency `{alias}`; updated zelyra.lock (cached source files were retained)"))
}

fn reject_output_target(path: &Path, description: &str) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => Err(format!(
            "{description} {} must be a regular non-symlink file",
            path.display()
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "cannot inspect {description} {}: {error}",
            path.display()
        )),
    }
}

fn add_command(args: &mut impl Iterator<Item = String>, updating: bool) -> Result<String, String> {
    let action = if updating { "update" } else { "add" };
    let registry_text = args.next().ok_or_else(|| {
        format!("usage: zelyra package {action} <registry-index-url> <name> <exact-version> --as <alias>")
    })?;
    let name = args
        .next()
        .ok_or_else(|| format!("package {action} requires a package name"))?;
    validate_package_name(&name).map_err(|error| error.message)?;
    let version_text = args
        .next()
        .ok_or_else(|| format!("package {action} requires an exact SemVer version"))?;
    let version = Version::parse(&version_text)
        .map_err(|error| format!("invalid exact package version `{version_text}`: {error}"))?;
    if version.to_string() != version_text {
        return Err(format!(
            "package {action} requires a normalized exact SemVer version"
        ));
    }
    let mut alias = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--as" if alias.is_none() => alias = Some(required_value(args, "--as")?),
            _ => return Err("unexpected or repeated package add option".into()),
        }
    }
    let alias = alias.ok_or_else(|| format!("package {action} requires `--as <alias>`"))?;
    validate_dependency_alias(&alias)?;
    let registry_url = parse_registry_url(&registry_text)?;
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .max_redirects(0)
        .build()
        .into();
    let index_bytes = fetch_limited(&agent, registry_url.as_str(), 4 * 1024 * 1024)?;
    let index: Value = serde_json::from_slice(&index_bytes)
        .map_err(|error| format!("invalid package registry index: {error}"))?;
    let entries = parse_registry_index(&index)?;
    let entry = entries
        .iter()
        .find(|entry| entry.name == name && entry.version == version_text)
        .ok_or_else(|| format!("registry has no exact release for {name} {version_text}"))?;
    let artifact_url = registry_url
        .join(&entry.artifact)
        .map_err(|error| format!("invalid registry artifact URL: {error}"))?;
    ensure_same_registry_origin(&registry_url, &artifact_url)?;
    let artifact_bytes = fetch_limited(&agent, artifact_url.as_str(), MAX_PACKAGE_BYTES)?;
    let artifact_sha256 = format!("sha256:{:x}", Sha256::digest(&artifact_bytes));
    if artifact_sha256 != entry.artifact_sha256 {
        return Err("downloaded artifact digest does not match the registry index".into());
    }
    let artifact = verify_artifact(&artifact_bytes)?;
    if artifact.name != entry.name
        || artifact.version != entry.version
        || artifact.publisher != entry.publisher
        || hex_encode(&artifact.public_key) != entry.public_key
        || artifact.checksum != entry.checksum
    {
        return Err("registry index metadata does not match the signed package artifact".into());
    }

    let workspace = std::env::current_dir()
        .map_err(|error| format!("cannot read current directory: {error}"))?;
    let workspace = fs::canonicalize(&workspace)
        .map_err(|error| format!("cannot resolve project directory: {error}"))?;
    if !trusted_key(
        &workspace.join(DEFAULT_TRUST_FILE),
        &artifact.publisher,
        &artifact.public_key,
    )? {
        return Err(format!("publisher `{}` is not trusted in {}; add the reviewed key with `zelyra package trust add` before installing", artifact.publisher, workspace.join(DEFAULT_TRUST_FILE).display()));
    }
    let manifest_path = workspace.join(MANIFEST);
    let manifest_bytes = read_regular_file(&manifest_path, 4 * 1024 * 1024, "project manifest")?;
    let mut document = std::str::from_utf8(&manifest_bytes)
        .map_err(|_| "project manifest must be UTF-8".to_owned())?
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid project manifest: {error}"))?;
    if document.get("package").and_then(Item::as_table).is_none() {
        return Err(format!(
            "package {action} requires a project [package] table"
        ));
    }
    if let Some(sources) = document.get("package_sources") {
        let sources = sources
            .as_table()
            .ok_or_else(|| "project `package_sources` must be a table".to_owned())?;
        if !updating && sources.contains_key(&alias) {
            return Err(format!(
                "package source metadata for `{alias}` already exists"
            ));
        }
        if updating {
            if let Some(previous) = sources.get(&alias).and_then(Item::as_table) {
                let previous_publisher = previous.get("publisher").and_then(Item::as_str);
                let previous_key = previous.get("public_key").and_then(Item::as_str);
                if previous_publisher != Some(artifact.publisher.as_str())
                    || previous_key != Some(hex_encode(&artifact.public_key).as_str())
                {
                    return Err(format!("package update for `{alias}` changes the trusted publisher or signing key; remove and add the dependency again after reviewing the new publisher"));
                }
            }
        }
    }
    let dependencies = document
        .entry("dependencies")
        .or_insert(Item::Table(Table::new()))
        .as_table_mut()
        .ok_or_else(|| "project `dependencies` must be a table".to_owned())?;
    if updating {
        let current = dependencies
            .get(&alias)
            .and_then(Item::as_table)
            .ok_or_else(|| {
                format!("cannot update `{alias}` because it is not a declared dependency")
            })?;
        let current_path = current
            .get("path")
            .and_then(Item::as_str)
            .and_then(super::safe_dependency_path)
            .ok_or_else(|| {
                format!("dependency `{alias}` does not have a safe local package path")
            })?;
        let current_root = fs::canonicalize(workspace.join(current_path))
            .map_err(|error| format!("cannot resolve current dependency `{alias}`: {error}"))?;
        if !current_root.starts_with(&workspace) {
            return Err(format!(
                "current dependency `{alias}` resolves outside the project workspace"
            ));
        }
        let current_manifest =
            read_manifest(&current_root.join(MANIFEST)).map_err(|error| error.message)?;
        let current_name = current_manifest
            .get("package")
            .and_then(toml::Value::as_table)
            .and_then(|package| package.get("name"))
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("current dependency `{alias}` has no package identity"))?;
        if current_name != name {
            return Err(format!(
                "dependency alias `{alias}` currently names package `{current_name}`, not `{name}`"
            ));
        }
    } else if dependencies.contains_key(&alias) {
        return Err(format!("dependency alias `{alias}` already exists; use `zelyra package update` for an explicit exact-version upgrade"));
    }
    let relative_install = PathBuf::from(".zelyra")
        .join("packages")
        .join(&name)
        .join(&version_text);
    let install_root = workspace.join(&relative_install);
    ensure_no_symlink_path(&workspace, &install_root)?;
    if install_root.exists() {
        return Err(format!(
            "package install destination {} already exists",
            relative_install.display()
        ));
    }
    let parent = install_root.parent().expect("package install has parent");
    ensure_no_symlink_path(&workspace, parent)?;
    let staging = parent.join(format!(".{version_text}.{}.tmp", std::process::id()));
    if staging.exists() {
        return Err("package install staging directory already exists".into());
    }
    let lock_path = workspace.join(LOCKFILE);
    reject_output_target(&lock_path, "package lockfile")?;
    let previous_lock = match fs::read(&lock_path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(format!("cannot read existing package lockfile: {error}")),
    };
    let mut transaction = PackageTransaction::prepare(
        &workspace,
        action,
        &manifest_bytes,
        previous_lock.as_deref(),
        &[staging.clone(), install_root.clone()],
    )?;
    fs::create_dir_all(parent).map_err(|error| {
        let rollback = transaction.rollback();
        format!(
            "cannot create package store: {error}{}",
            rollback
                .err()
                .map(|error| format!("; recovery failed: {error}"))
                .unwrap_or_default()
        )
    })?;
    ensure_no_symlink_path(&workspace, parent)?;
    fs::create_dir(&staging).map_err(|error| {
        let rollback = transaction.rollback();
        format!(
            "cannot create package staging directory: {error}{}",
            rollback
                .err()
                .map(|error| format!("; recovery failed: {error}"))
                .unwrap_or_default()
        )
    })?;
    let materialize_result = (|| {
        for (path, source) in &artifact.files {
            validate_archive_path(path)?;
            let output_path = staging.join(path);
            fs::create_dir_all(output_path.parent().expect("source path has parent"))
                .map_err(|error| format!("cannot prepare package source path: {error}"))?;
            fs::write(output_path, source)
                .map_err(|error| format!("cannot write verified package source: {error}"))?;
        }
        validate_package_sources(&artifact.files, &name, &version_text)?;
        fs::rename(&staging, &install_root).map_err(|error| {
            format!("cannot publish verified package into local store: {error}")
        })?;
        Ok::<(), String>(())
    })();
    if let Err(error) = materialize_result {
        return Err(match transaction.rollback() {
            Ok(()) => error,
            Err(recovery) => format!("{error}; package recovery is pending: {recovery}"),
        });
    }

    let dependency_path = relative_install.to_string_lossy().replace('\\', "/");
    let mut dependency = Table::new();
    dependency["path"] = value(dependency_path);
    dependency["version"] = value(format!("={version_text}"));
    dependencies.insert(&alias, Item::Table(dependency));
    let source_table = document
        .entry("package_sources")
        .or_insert(Item::Table(Table::new()))
        .as_table_mut()
        .expect("package_sources table was validated before installation");
    let mut source = Table::new();
    source["registry"] = value(registry_url.as_str());
    source["publisher"] = value(artifact.publisher.clone());
    source["public_key"] = value(hex_encode(&artifact.public_key));
    source["artifact_sha256"] = value(entry.artifact_sha256.clone());
    source_table.insert(&alias, Item::Table(source));
    let new_manifest = document.to_string();
    let transaction_result = (|| {
        write_atomic(&manifest_path, new_manifest.as_bytes(), 0o644)?;
        let graph = PackageGraph::load(&workspace)
            .map_err(|error| error.message)?
            .ok_or_else(|| "project package graph disappeared after manifest update".to_owned())?;
        write_lockfile(&graph, true).map_err(|error| error.message)?;
        validate_project_manifest(&workspace, &new_manifest)?;
        Ok::<(), String>(())
    })();
    if let Err(error) = transaction_result {
        return Err(match transaction.rollback() {
            Ok(()) => error,
            Err(recovery) => format!("{error}; package recovery is pending: {recovery}"),
        });
    }
    transaction.commit()?;
    if updating {
        Ok(format!("updated `{alias}` to trusted package {name} {version_text} from {} and updated zelyra.lock; previous cached versions were retained", registry_url))
    } else {
        Ok(format!("installed trusted package {name} {version_text} as `{alias}` from {} and updated zelyra.lock", registry_url))
    }
}

#[derive(Debug)]
struct PlannedPackageUpdate {
    alias: String,
    name: String,
    version: String,
    install_root: PathBuf,
    staging: Option<PathBuf>,
}

#[derive(Default)]
struct UpdatePlanStageCleanup(Vec<PathBuf>);

impl Drop for UpdatePlanStageCleanup {
    fn drop(&mut self) {
        for path in &self.0 {
            if path.exists() {
                let _ = fs::remove_dir_all(path);
            }
        }
    }
}

fn update_plan_command(args: &mut impl Iterator<Item = String>) -> Result<String, String> {
    let plan_path = args
        .next()
        .ok_or_else(|| "usage: zelyra package update --plan <updates.toml>".to_owned())?;
    if args.next().is_some() {
        return Err("usage: zelyra package update --plan <updates.toml>".into());
    }
    let plan_bytes = read_regular_file(Path::new(&plan_path), 1024 * 1024, "package update plan")?;
    let plan_text = std::str::from_utf8(&plan_bytes)
        .map_err(|_| "package update plan must be UTF-8".to_owned())?;
    let plan: toml::Value = toml::from_str(plan_text)
        .map_err(|error| format!("invalid package update plan: {error}"))?;
    let plan_table = plan
        .as_table()
        .filter(|table| table.len() == 1 && table.contains_key("update"))
        .ok_or_else(|| "package update plan requires only [[update]] entries".to_owned())?;
    let updates = plan_table["update"]
        .as_array()
        .filter(|updates| !updates.is_empty() && updates.len() <= 128)
        .ok_or_else(|| {
            "package update plan must contain between 1 and 128 [[update]] entries".to_owned()
        })?;
    let mut specs = Vec::with_capacity(updates.len());
    let mut aliases = BTreeSet::new();
    let mut package_versions = BTreeSet::new();
    for entry in updates {
        let fields = entry
            .as_table()
            .filter(|fields| {
                fields.len() == 4
                    && ["alias", "registry", "name", "version"]
                        .iter()
                        .all(|field| fields.contains_key(*field))
            })
            .ok_or_else(|| {
                "each [[update]] requires exactly alias, registry, name, and version".to_owned()
            })?;
        let text = |field: &str| -> Result<String, String> {
            fields[field]
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .map(str::to_owned)
                .ok_or_else(|| {
                    format!("package update plan field `{field}` must be a nonempty string")
                })
        };
        let alias = text("alias")?;
        validate_dependency_alias(&alias)?;
        if !aliases.insert(alias.clone()) {
            return Err(format!(
                "package update plan contains duplicate alias `{alias}`"
            ));
        }
        let name = text("name")?;
        validate_package_name(&name).map_err(|error| error.message)?;
        let version = text("version")?;
        let parsed = Version::parse(&version)
            .map_err(|error| format!("invalid exact package version `{version}`: {error}"))?;
        if parsed.to_string() != version {
            return Err(format!(
                "package update plan requires normalized exact SemVer for `{alias}`"
            ));
        }
        if !package_versions.insert((name.clone(), version.clone())) {
            return Err(format!(
                "package update plan repeats package {name} {version}; use one dependency alias"
            ));
        }
        let registry = parse_registry_url(&text("registry")?)?;
        specs.push((alias, name, version, registry));
    }

    let workspace = std::env::current_dir()
        .map_err(|error| format!("cannot read current directory: {error}"))?;
    let workspace = fs::canonicalize(workspace)
        .map_err(|error| format!("cannot resolve project directory: {error}"))?;
    let manifest_path = workspace.join(MANIFEST);
    let original_manifest = read_regular_file(&manifest_path, 4 * 1024 * 1024, "project manifest")?;
    let mut document = std::str::from_utf8(&original_manifest)
        .map_err(|_| "project manifest must be UTF-8".to_owned())?
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid project manifest: {error}"))?;
    if document.get("package").and_then(Item::as_table).is_none() {
        return Err("package update plan requires a project [package] table".into());
    }
    let current_graph = PackageGraph::load(&workspace)
        .map_err(|error| error.message)?
        .ok_or_else(|| "project package graph is unavailable".to_owned())?;
    current_graph.check_lockfile().map_err(|error| {
        format!(
            "{}; resolve existing changes before applying an update plan",
            error.message
        )
    })?;
    let lock_path = workspace.join(LOCKFILE);
    reject_output_target(&lock_path, "package lockfile")?;
    let original_lock = read_regular_file(&lock_path, 16 * 1024 * 1024, "package lockfile")?;
    let trust_path = workspace.join(DEFAULT_TRUST_FILE);
    let trust_snapshot = match fs::symlink_metadata(&trust_path) {
        Ok(_) => Some(read_regular_file(
            &trust_path,
            MAX_TRUST_BYTES as usize,
            "package trust file",
        )?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(format!("cannot inspect package trust file: {error}")),
    };

    if document
        .get("dependencies")
        .and_then(Item::as_table)
        .is_none()
    {
        return Err("package update plan requires existing package dependencies".into());
    }
    let mut staging_by_alias = BTreeMap::new();
    let mut cleanup_paths = Vec::with_capacity(specs.len() * 2);
    for (alias, name, version, _) in &specs {
        let install_root = workspace
            .join(".zelyra")
            .join("packages")
            .join(name)
            .join(version);
        ensure_no_symlink_path(&workspace, &install_root)?;
        let parent = install_root
            .parent()
            .expect("package installation path has a parent");
        ensure_no_symlink_path(&workspace, parent)?;
        let staging = parent.join(format!(
            ".{version}.update-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        ensure_no_symlink_path(&workspace, &staging)?;
        if staging.exists() {
            return Err(format!(
                "package update staging path already exists: {}",
                staging.display()
            ));
        }
        cleanup_paths.push(staging.clone());
        if !install_root.exists() {
            cleanup_paths.push(install_root);
        }
        staging_by_alias.insert(alias.clone(), staging);
    }
    let mut transaction_journal = PackageTransaction::prepare(
        &workspace,
        "update",
        &original_manifest,
        Some(&original_lock),
        &cleanup_paths,
    )?;
    let mut prepared = Vec::with_capacity(specs.len());
    let mut staged_paths = UpdatePlanStageCleanup::default();
    let mut total_download_bytes = 0_u64;
    let mut total_source_files = 0_usize;
    let deadline = std::time::Instant::now() + Duration::from_secs(300);
    let update_result = (|| {
        for (alias, name, version, registry) in specs {
            let current_path = document
                .get("dependencies")
                .and_then(Item::as_table)
                .and_then(|dependencies| dependencies.get(&alias))
                .and_then(Item::as_table)
                .ok_or_else(|| {
                    format!("cannot update `{alias}` because it is not a declared dependency")
                })?
                .get("path")
                .and_then(Item::as_str)
                .and_then(super::safe_dependency_path)
                .ok_or_else(|| {
                    format!("dependency `{alias}` does not have a safe local package path")
                })?;
            let current_root =
                super::resolve_dependency_root(&workspace, &workspace, &current_path)
                    .map_err(|error| error.message)?;
            let current_manifest =
                read_manifest(&current_root.join(MANIFEST)).map_err(|error| error.message)?;
            let current_package = current_manifest
                .get("package")
                .and_then(toml::Value::as_table)
                .ok_or_else(|| format!("current dependency `{alias}` has no package identity"))?;
            let current_name = current_package.get("name").and_then(toml::Value::as_str);
            if current_name != Some(name.as_str()) {
                return Err(format!(
                    "dependency alias `{alias}` does not name package `{name}`"
                ));
            }
            let sources = document
            .get("package_sources")
            .and_then(Item::as_table)
            .and_then(|sources| sources.get(&alias))
            .and_then(Item::as_table)
            .ok_or_else(|| format!("dependency `{alias}` has no trusted registry provenance; remove and add it after review"))?;
            let old_publisher =
                sources
                    .get("publisher")
                    .and_then(Item::as_str)
                    .ok_or_else(|| {
                        format!("dependency `{alias}` has incomplete registry provenance")
                    })?;
            let old_key = sources
                .get("public_key")
                .and_then(Item::as_str)
                .ok_or_else(|| {
                    format!("dependency `{alias}` has incomplete registry provenance")
                })?;

            let index_bytes = fetch_limited_before(registry.as_str(), 4 * 1024 * 1024, deadline)?;
            total_download_bytes = total_download_bytes.saturating_add(index_bytes.len() as u64);
            if total_download_bytes > MAX_REGISTRY_BYTES {
                return Err("package update plan downloads exceed the 256 MiB total limit".into());
            }
            let index: Value = serde_json::from_slice(&index_bytes)
                .map_err(|error| format!("invalid package registry index: {error}"))?;
            let entries = parse_registry_index(&index)?;
            let entry = entries
                .iter()
                .find(|entry| entry.name == name && entry.version == version)
                .ok_or_else(|| format!("registry has no exact release for {name} {version}"))?;
            let artifact_url = registry
                .join(&entry.artifact)
                .map_err(|error| format!("invalid registry artifact URL: {error}"))?;
            ensure_same_registry_origin(&registry, &artifact_url)?;
            let bytes = fetch_limited_before(artifact_url.as_str(), MAX_PACKAGE_BYTES, deadline)?;
            total_download_bytes = total_download_bytes.saturating_add(bytes.len() as u64);
            if total_download_bytes > MAX_REGISTRY_BYTES {
                return Err("package update plan downloads exceed the 256 MiB total limit".into());
            }
            let artifact_sha256 = format!("sha256:{:x}", Sha256::digest(&bytes));
            if artifact_sha256 != entry.artifact_sha256 {
                return Err(format!(
                    "downloaded artifact digest for `{alias}` does not match the registry index"
                ));
            }
            let artifact = verify_artifact(&bytes)?;
            total_source_files = total_source_files.saturating_add(artifact.files.len());
            if total_source_files > 16_384 {
                return Err(
                    "package update plan exceeds the 16,384 total source-file limit".into(),
                );
            }
            if artifact.name != name
                || artifact.version != version
                || artifact.publisher != entry.publisher
                || hex_encode(&artifact.public_key) != entry.public_key
                || artifact.checksum != entry.checksum
            {
                return Err(format!(
                    "registry metadata does not match signed package `{alias}`"
                ));
            }
            if artifact.publisher != old_publisher || hex_encode(&artifact.public_key) != old_key {
                return Err(format!("package update for `{alias}` changes the trusted publisher or signing key; remove and re-add it after reviewing the new publisher"));
            }
            if !trusted_key(&trust_path, &artifact.publisher, &artifact.public_key)? {
                return Err(format!(
                    "publisher `{}` is no longer trusted for `{alias}`",
                    artifact.publisher
                ));
            }
            validate_package_sources(&artifact.files, &artifact.name, &artifact.version)?;

            let relative_install = PathBuf::from(".zelyra")
                .join("packages")
                .join(&name)
                .join(&version);
            let install_root = workspace.join(&relative_install);
            ensure_no_symlink_path(&workspace, &install_root)?;
            let reused = if install_root.exists() {
                ensure_no_symlinks_in_tree(&install_root)?;
                let cached_manifest =
                    read_manifest(&install_root.join(MANIFEST)).map_err(|_| {
                        format!("cached package destination for `{alias}` is not a valid package")
                    })?;
                let cached_identity = cached_manifest
                    .get("package")
                    .and_then(toml::Value::as_table);
                let cached_checksum =
                    super::package_checksum(&install_root).map_err(|error| error.message)?;
                if cached_identity
                    .and_then(|package| package.get("name"))
                    .and_then(toml::Value::as_str)
                    != Some(name.as_str())
                    || cached_identity
                        .and_then(|package| package.get("version"))
                        .and_then(toml::Value::as_str)
                        != Some(version.as_str())
                    || cached_checksum != artifact.checksum
                {
                    return Err(format!(
                    "cached package destination for `{alias}` does not match the verified artifact"
                ));
                }
                true
            } else {
                false
            };
            let staging = if reused {
                None
            } else {
                let parent = install_root.parent().expect("package cache parent exists");
                ensure_no_symlink_path(&workspace, parent)?;
                fs::create_dir_all(parent)
                    .map_err(|error| format!("cannot create package cache: {error}"))?;
                ensure_no_symlink_path(&workspace, parent)?;
                let staging = staging_by_alias
                    .get(&alias)
                    .expect("every update alias has a precomputed staging path")
                    .clone();
                ensure_no_symlink_path(&workspace, &staging)?;
                if staging.exists() {
                    return Err(format!(
                        "package update staging path already exists: {}",
                        staging.display()
                    ));
                }
                fs::create_dir(&staging)
                    .map_err(|error| format!("cannot stage package `{alias}`: {error}"))?;
                staged_paths.0.push(staging.clone());
                for (path, source) in &artifact.files {
                    if let Err(error) = validate_archive_path(path) {
                        let _ = fs::remove_dir_all(&staging);
                        return Err(error);
                    }
                    let output = staging.join(path);
                    if let Err(error) =
                        fs::create_dir_all(output.parent().expect("archive file has parent"))
                            .and_then(|()| fs::write(output, source))
                    {
                        let _ = fs::remove_dir_all(&staging);
                        return Err(format!("cannot stage verified package `{alias}`: {error}"));
                    }
                }
                Some(staging)
            };
            let dependency = document
                .get_mut("dependencies")
                .and_then(Item::as_table_mut)
                .and_then(|dependencies| dependencies.get_mut(&alias))
                .and_then(Item::as_table_mut)
                .expect("dependency was validated above");
            dependency["path"] = value(relative_install.to_string_lossy().replace('\\', "/"));
            dependency["version"] = value(format!("={version}"));
            let source = document
                .get_mut("package_sources")
                .and_then(Item::as_table_mut)
                .and_then(|sources| sources.get_mut(&alias))
                .and_then(Item::as_table_mut)
                .expect("package source was validated above");
            source["registry"] = value(registry.as_str());
            source["artifact_sha256"] = value(artifact_sha256.clone());
            prepared.push(PlannedPackageUpdate {
                alias,
                name,
                version,
                install_root,
                staging,
            });
        }
        let new_manifest = document.to_string();
        let current_manifest =
            read_regular_file(&manifest_path, 4 * 1024 * 1024, "project manifest")?;
        let current_lock = read_regular_file(&lock_path, 16 * 1024 * 1024, "package lockfile")?;
        let current_trust = match fs::symlink_metadata(&trust_path) {
            Ok(_) => Some(read_regular_file(
                &trust_path,
                MAX_TRUST_BYTES as usize,
                "package trust file",
            )?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(format!("cannot inspect package trust file: {error}")),
        };
        if current_manifest != original_manifest
            || current_lock != original_lock
            || current_trust != trust_snapshot
        {
            return Err("project manifest, lockfile, or package trust changed while the update plan was downloading; review changes and retry".into());
        }
        for update in &prepared {
            if let Some(staging) = &update.staging {
                fs::rename(staging, &update.install_root).map_err(|error| {
                    format!(
                        "cannot install verified package `{}`: {error}",
                        update.alias
                    )
                })?;
            }
        }
        write_atomic(&manifest_path, new_manifest.as_bytes(), 0o644)?;
        let graph = PackageGraph::load(&workspace)
            .map_err(|error| error.message)?
            .ok_or_else(|| "project package graph disappeared after update plan".to_owned())?;
        write_lockfile(&graph, true).map_err(|error| error.message)?;
        validate_project_manifest(&workspace, &new_manifest)?;
        Ok::<(), String>(())
    })();
    if let Err(error) = update_result {
        return Err(match transaction_journal.rollback() {
            Ok(()) => error,
            Err(recovery) => format!("{error}; package recovery is pending: {recovery}"),
        });
    }
    transaction_journal.commit()?;
    let summary = prepared
        .iter()
        .map(|update| format!("{}={}/{}", update.alias, update.name, update.version))
        .collect::<Vec<_>>()
        .join(", ");
    drop(staged_paths);
    Ok(format!(
        "updated {} package alias(es) atomically: {summary}; zelyra.lock is current",
        prepared.len()
    ))
}

#[derive(Debug)]
struct RegistryEntry {
    name: String,
    version: String,
    publisher: String,
    public_key: String,
    checksum: String,
    artifact_sha256: String,
    artifact: String,
}

fn parse_registry_index(index: &Value) -> Result<Vec<RegistryEntry>, String> {
    let object = index
        .as_object()
        .ok_or_else(|| "registry index must be an object".to_owned())?;
    if object.len() != 2 || object.get("format_version").and_then(Value::as_u64) != Some(1) {
        return Err("unsupported or malformed registry index".into());
    }
    let packages = object
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(|| "registry index must contain a packages array".to_owned())?;
    if packages.len() > MAX_REGISTRY_ARTIFACTS {
        return Err("registry index exceeds the 10,000 package limit".into());
    }
    let mut entries = Vec::with_capacity(packages.len());
    let mut identities = BTreeSet::new();
    for package in packages {
        let package = package
            .as_object()
            .ok_or_else(|| "registry package entry must be an object".to_owned())?;
        let fields = [
            "name",
            "version",
            "publisher",
            "public_key",
            "checksum",
            "artifact_sha256",
            "artifact",
        ];
        if package.len() != fields.len()
            || package
                .keys()
                .any(|field| !fields.contains(&field.as_str()))
        {
            return Err("registry package entry has missing or unknown fields".into());
        }
        let get = |key: &str| required_string(package, key).map(str::to_owned);
        let entry = RegistryEntry {
            name: get("name")?,
            version: get("version")?,
            publisher: get("publisher")?,
            public_key: get("public_key")?,
            checksum: get("checksum")?,
            artifact_sha256: get("artifact_sha256")?,
            artifact: get("artifact")?,
        };
        validate_package_name(&entry.name).map_err(|error| error.message)?;
        Version::parse(&entry.version)
            .map_err(|error| format!("invalid package version in registry: {error}"))?;
        validate_publisher_id(&entry.publisher)?;
        validate_registry_artifact_path(&entry.artifact)?;
        if !entry.checksum.starts_with("sha256:")
            || entry.checksum.len() != 71
            || !entry.artifact_sha256.starts_with("sha256:")
            || entry.artifact_sha256.len() != 71
            || !entry.artifact_sha256[7..]
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || !entry.checksum[7..]
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || hex_decode(&entry.public_key)?.len() != 32
        {
            return Err(
                "registry package entry contains malformed digest, key, or artifact path".into(),
            );
        }
        if !identities.insert((entry.name.clone(), entry.version.clone())) {
            return Err("registry index repeats a package name and version".into());
        }
        entries.push(entry);
    }
    Ok(entries)
}

fn parse_registry_url(text: &str) -> Result<Url, String> {
    let url = Url::parse(text).map_err(|error| format!("invalid registry URL: {error}"))?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("registry URL may not contain credentials, a query, or a fragment".into());
    }
    let loopback_http = url.scheme() == "http" && url.host_str().is_some_and(is_loopback_host);
    if url.scheme() != "https" && !loopback_http {
        return Err(
            "package registries require HTTPS (HTTP is allowed only on loopback for local testing)"
                .into(),
        );
    }
    Ok(url)
}

fn validate_registry_artifact_path(path: &str) -> Result<(), String> {
    let candidate = Path::new(path);
    if path.contains('\\')
        || path
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
        || candidate
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || candidate
            .extension()
            .is_none_or(|extension| extension != "zpkg")
        || path.contains(['?', '#', '%'])
    {
        return Err("registry artifact path must be a safe relative `.zpkg` path".into());
    }
    Ok(())
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

fn ensure_same_registry_origin(index: &Url, artifact: &Url) -> Result<(), String> {
    if index.scheme() != artifact.scheme()
        || index.host_str() != artifact.host_str()
        || index.port_or_known_default() != artifact.port_or_known_default()
        || artifact.username() != ""
        || artifact.password().is_some()
    {
        return Err("registry artifact URL must remain on the same origin as the index".into());
    }
    Ok(())
}

fn fetch_limited(agent: &ureq::Agent, url: &str, maximum: u64) -> Result<Vec<u8>, String> {
    let mut response = agent
        .get(url)
        .header("User-Agent", concat!("zelyra/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/json, application/octet-stream")
        .call()
        .map_err(|error| format!("package registry request failed: {error}"))?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(maximum + 1)
        .read_to_vec()
        .map_err(|error| format!("cannot read package registry response: {error}"))?;
    if bytes.len() as u64 > maximum {
        return Err(format!("package registry response exceeds {maximum} bytes"));
    }
    Ok(bytes)
}

fn fetch_limited_before(
    url: &str,
    maximum: u64,
    deadline: std::time::Instant,
) -> Result<Vec<u8>, String> {
    let remaining = deadline.saturating_duration_since(std::time::Instant::now());
    if remaining.is_zero() {
        return Err("package update plan exceeded its five-minute download limit".into());
    }
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(remaining))
        .max_redirects(0)
        .build()
        .into();
    fetch_limited(&agent, url, maximum).map_err(|error| {
        if std::time::Instant::now() >= deadline {
            "package update plan exceeded its five-minute download limit".into()
        } else {
            error
        }
    })
}

fn validate_dependency_alias(alias: &str) -> Result<(), String> {
    if alias.is_empty()
        || alias.len() > 64
        || !alias
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err("dependency alias must contain 1–64 ASCII letters, digits, `_`, or `-`".into());
    }
    Ok(())
}

fn validate_project_manifest(workspace: &Path, manifest_source: &str) -> Result<(), String> {
    let manifest: toml::Value = toml::from_str(manifest_source)
        .map_err(|error| format!("updated project manifest is invalid: {error}"))?;
    let entry = manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .and_then(|package| package.get("entry"))
        .and_then(toml::Value::as_str)
        .unwrap_or("main.zyl");
    let relative = super::safe_relative_path(entry)
        .ok_or_else(|| "project package entry must be a safe relative path".to_owned())?;
    let entry_path = fs::canonicalize(workspace.join(relative)).map_err(|error| {
        format!("cannot resolve project entry for compiler validation: {error}")
    })?;
    if !entry_path.starts_with(workspace) || !entry_path.is_file() {
        return Err("project entry resolves outside the workspace".into());
    }
    let entry = entry_path
        .to_str()
        .ok_or_else(|| "project entry path is not UTF-8".to_owned())?;
    super::super::validate(entry)
        .map(|_| ())
        .map_err(|_| "project compiler validation failed after package manifest change".to_owned())
}

fn ensure_no_symlink_path(root: &Path, path: &Path) -> Result<(), String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| "package store path escaped project root".to_owned())?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!(
                    "package store may not traverse symlink {}",
                    current.display()
                ))
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("cannot inspect package store path: {error}")),
        }
    }
    Ok(())
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
    let mut total_bytes = 0;
    collect_registry_artifacts(&root, &root, &mut artifacts, &mut total_bytes)?;
    if artifacts.len() > MAX_REGISTRY_ARTIFACTS {
        return Err(format!(
            "registry contains more than {MAX_REGISTRY_ARTIFACTS} package artifacts"
        ));
    }
    artifacts.sort_by(|left, right| left.0.cmp(&right.0));
    let mut identities = BTreeSet::new();
    let mut entries = Vec::with_capacity(artifacts.len());
    for (path, bytes) in artifacts {
        let artifact = verify_artifact(&bytes)?;
        if !identities.insert((artifact.name.clone(), artifact.version.clone())) {
            return Err(format!(
                "registry contains more than one artifact for {} {}",
                artifact.name, artifact.version
            ));
        }
        entries.push(serde_json::json!({
            "name": artifact.name,
            "version": artifact.version,
            "publisher": artifact.publisher,
            "public_key": hex_encode(&artifact.public_key),
            "checksum": artifact.checksum,
            "artifact_sha256": format!("sha256:{:x}", Sha256::digest(&bytes)),
            "artifact": path,
        }));
    }
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
    total_bytes: &mut u64,
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
            collect_registry_artifacts(root, &path, artifacts, total_bytes)?;
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
            *total_bytes = total_bytes.saturating_add(metadata.len());
            if *total_bytes > MAX_REGISTRY_BYTES {
                return Err("registry artifact tree exceeds the 256 MiB scan limit".into());
            }
            artifacts.push((
                relative,
                read_regular_file(
                    &path,
                    MAX_PACKAGE_BYTES as usize,
                    "registry package artifact",
                )?,
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
    let mut key_bytes = read_regular_file(&key_path, 4096, "private signing key")?;
    let key_result = Ed25519KeyPair::from_pkcs8(&key_bytes);
    key_bytes.zeroize();
    let key = key_result
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
    files: Vec<(String, String)>,
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
        files,
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
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains(':')
        || path
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(format!("unsafe package source path `{path}`"));
    }
    let candidate = Path::new(path);
    if candidate
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
        || is_unhashed_package_path(candidate)
        || !(path == MANIFEST || path.ends_with("/zelyra.toml") || path.ends_with(".zyl"))
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

    fn temporary_workspace(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("zelyra-package-{label}-{stamp}"));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn package_operation_lock_excludes_concurrent_mutations_and_releases_on_drop() {
        let root = temporary_workspace("operation-lock");
        let first = PackageOperationLock::acquire(&root).unwrap();
        assert!(PackageOperationLock::acquire(&root)
            .unwrap_err()
            .contains("another Zelyra package operation is active"));
        drop(first);
        let second = PackageOperationLock::acquire(&root).unwrap();
        drop(second);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn interrupted_package_transaction_preview_is_read_only_and_apply_restores_all_state() {
        let root = temporary_workspace("recovery");
        let old_manifest = b"[package]\nname = \"app\"\nversion = \"1.0.0\"\n";
        let old_lock = b"lock_version = 1\n\n[[package]]\nname = \"old\"\n";
        fs::write(root.join(MANIFEST), old_manifest).unwrap();
        fs::write(root.join(LOCKFILE), old_lock).unwrap();
        let installed = root.join(".zelyra/packages/widget/2.0.0");
        let staging = root.join(".zelyra/packages/widget/.2.0.0.123.tmp");
        let transaction = PackageTransaction::prepare(
            &root,
            "add",
            old_manifest,
            Some(old_lock),
            &[staging.clone(), installed.clone()],
        )
        .unwrap();
        fs::create_dir_all(&installed).unwrap();
        fs::create_dir_all(&staging).unwrap();
        fs::write(root.join(MANIFEST), b"new manifest").unwrap();
        fs::write(root.join(LOCKFILE), b"new lock").unwrap();
        drop(transaction);

        let preview = recover_transaction(&root, false).unwrap();
        assert!(preview.contains("will restore"));
        assert_eq!(fs::read(root.join(MANIFEST)).unwrap(), b"new manifest");
        assert_eq!(fs::read(root.join(LOCKFILE)).unwrap(), b"new lock");
        assert!(installed.exists());

        recover_transaction(&root, true).unwrap();
        assert_eq!(fs::read(root.join(MANIFEST)).unwrap(), old_manifest);
        assert_eq!(fs::read(root.join(LOCKFILE)).unwrap(), old_lock);
        assert!(!installed.exists());
        assert!(!staging.exists());
        assert!(!root.join(PACKAGE_TRANSACTION_DIR).exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn committed_transaction_recovery_only_removes_the_journal() {
        let root = temporary_workspace("committed-recovery");
        let old_manifest = b"old";
        fs::write(root.join(MANIFEST), old_manifest).unwrap();
        let transaction =
            PackageTransaction::prepare(&root, "remove", old_manifest, None, &[]).unwrap();
        fs::write(root.join(MANIFEST), b"committed manifest").unwrap();
        let journal_path = root.join(PACKAGE_TRANSACTION_DIR).join("journal.json");
        let mut journal: Value = serde_json::from_slice(&fs::read(&journal_path).unwrap()).unwrap();
        journal["phase"] = Value::String("committed".into());
        write_atomic(&journal_path, &serde_json::to_vec(&journal).unwrap(), 0o600).unwrap();
        drop(transaction);

        recover_transaction(&root, true).unwrap();
        assert_eq!(
            fs::read(root.join(MANIFEST)).unwrap(),
            b"committed manifest"
        );
        assert!(!root.join(PACKAGE_TRANSACTION_DIR).exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn transaction_rollback_does_not_recover_unrelated_cache_quarantines() {
        let root = temporary_workspace("rollback-isolation");
        let original = b"original manifest";
        fs::write(root.join(MANIFEST), original).unwrap();
        let mut transaction =
            PackageTransaction::prepare(&root, "add", original, None, &[]).unwrap();
        fs::write(root.join(MANIFEST), b"partial manifest").unwrap();
        let quarantined = root.join(".zelyra/package-cache-trash-7-123/widget/2.0.0");
        fs::create_dir_all(&quarantined).unwrap();
        fs::write(quarantined.join(MANIFEST), "cache package").unwrap();

        transaction.rollback().unwrap();

        assert_eq!(fs::read(root.join(MANIFEST)).unwrap(), original);
        assert!(quarantined.join(MANIFEST).is_file());
        assert!(!root.join(".zelyra/packages/widget/2.0.0").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recovery_journal_rejects_paths_outside_the_package_cache() {
        let journal = serde_json::json!({
            "journal_version": PACKAGE_JOURNAL_VERSION,
            "operation": "add",
            "phase": "prepared",
            "has_lock_backup": false,
            "cleanup_paths": [".zelyra/packages/../../outside"],
        });
        assert!(validate_recovery_journal(&journal)
            .unwrap_err()
            .contains("unsafe"));
        assert!(package_cleanup_relative_path(
            Path::new("/tmp/project"),
            Path::new("/tmp/project/.zelyra/package-operation.lock")
        )
        .is_err());
    }

    #[test]
    fn package_recovery_restores_a_crash_interrupted_cache_cleanup() {
        let root = temporary_workspace("cache-recovery");
        let quarantine = root.join(".zelyra/package-cache-trash-7-123");
        let removed = quarantine.join("widget/2.0.0");
        fs::create_dir_all(&removed).unwrap();
        fs::write(
            removed.join(MANIFEST),
            "[package]\nname = \"widget\"\nversion = \"2.0.0\"\n",
        )
        .unwrap();

        let preview = recover_cache_quarantines(&root, false).unwrap();
        assert!(preview.contains("1 quarantined package version"));
        assert!(removed.exists());
        assert!(!root.join(".zelyra/packages/widget/2.0.0").exists());

        let recovered = recover_cache_quarantines(&root, true).unwrap();
        assert!(recovered.contains("restored 1 quarantined package version"));
        assert!(root
            .join(".zelyra/packages/widget/2.0.0/zelyra.toml")
            .is_file());
        assert!(!quarantine.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn package_recovery_does_not_overwrite_a_cache_version() {
        let root = temporary_workspace("cache-recovery-conflict");
        let quarantine = root.join(".zelyra/package-cache-trash-9-456");
        let removed = quarantine.join("widget/2.0.0");
        fs::create_dir_all(&removed).unwrap();
        fs::write(removed.join(MANIFEST), "quarantined").unwrap();
        let existing = root.join(".zelyra/packages/widget/2.0.0");
        fs::create_dir_all(&existing).unwrap();

        let error = recover_cache_quarantines(&root, true).unwrap_err();
        assert!(error.contains("cache destination already exists"));
        assert!(removed.join(MANIFEST).is_file());
        assert!(existing.is_dir());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn package_cache_recovery_refuses_symlinked_quarantine_content() {
        use std::os::unix::fs::symlink;

        let root = temporary_workspace("cache-recovery-symlink");
        let quarantine = root.join(".zelyra/package-cache-trash-10-789");
        let package_dir = quarantine.join("widget");
        fs::create_dir_all(&package_dir).unwrap();
        let outside = root.join("outside");
        fs::create_dir_all(&outside).unwrap();
        symlink(&outside, package_dir.join("2.0.0")).unwrap();

        let error = recover_cache_quarantines(&root, true).unwrap_err();

        assert!(error.contains("symlink"));
        assert!(outside.is_dir());
        assert!(quarantine.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn package_recovery_preserves_journal_with_unknown_files() {
        let root = temporary_workspace("recovery-unknown-file");
        let original = b"original manifest";
        fs::write(root.join(MANIFEST), original).unwrap();
        let transaction = PackageTransaction::prepare(&root, "add", original, None, &[]).unwrap();
        fs::write(
            root.join(PACKAGE_TRANSACTION_DIR).join("unexpected"),
            b"preserve",
        )
        .unwrap();
        drop(transaction);

        let error = recover_transaction(&root, true).unwrap_err();

        assert!(error.contains("unknown or unsafe file"));
        assert_eq!(fs::read(root.join(MANIFEST)).unwrap(), original);
        assert!(root
            .join(PACKAGE_TRANSACTION_DIR)
            .join("journal.json")
            .is_file());
        assert!(root
            .join(PACKAGE_TRANSACTION_DIR)
            .join("unexpected")
            .is_file());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recovery_removes_journals_interrupted_before_any_project_mutation() {
        for (label, backups) in [
            ("empty", Vec::<&str>::new()),
            ("manifest", vec!["manifest.backup"]),
            ("lock", vec!["lock.backup"]),
        ] {
            let root = temporary_workspace(&format!("incomplete-{label}"));
            let directory = root.join(PACKAGE_TRANSACTION_DIR);
            fs::create_dir_all(&directory).unwrap();
            for backup in backups {
                fs::write(directory.join(backup), b"partial backup").unwrap();
            }

            let preview = recover_transaction(&root, false).unwrap();
            assert!(preview.contains("no project mutation"));
            assert!(directory.exists());

            let applied = recover_transaction(&root, true).unwrap();
            assert!(applied.contains("no project mutation"));
            assert!(!directory.exists());
            fs::remove_dir_all(root).unwrap();
        }
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
    fn registry_index_rejects_path_escape_and_non_loopback_http() {
        let mut index = serde_json::json!({
            "format_version": 1,
            "packages": [{
                "name": "hello", "version": "1.0.0", "publisher": "acme",
                "public_key": hex_encode(&[1; 32]), "checksum": format!("sha256:{}", "a".repeat(64)),
                "artifact_sha256": format!("sha256:{}", "b".repeat(64)), "artifact": "hello-1.0.0.zpkg"
            }]
        });
        assert!(parse_registry_index(&index).is_ok());
        index["packages"][0]["artifact"] = Value::String("../hello.zpkg".into());
        assert!(parse_registry_index(&index)
            .unwrap_err()
            .contains("safe relative"));
        assert!(parse_registry_url("http://packages.example.test/index.json").is_err());
        assert!(parse_registry_url("http://127.0.0.1:8123/index.json").is_ok());
        assert!(validate_archive_path("src/./mod.zyl").is_err());
        assert!(validate_archive_path("src//mod.zyl").is_err());
        assert!(validate_registry_artifact_path("releases//x.zpkg").is_err());
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
