//! Crash-recoverable, hash-checked transactions for Studio multi-file saves.
use super::*;
use serde_json::Value;

const JOURNAL_DIRECTORY: &str = "studio-transactions";
const MAX_TRANSACTION_FILES: usize = 64;
const MAX_TRANSACTION_BYTES: usize = 8 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 131_072;

fn random_hex(bytes: usize) -> Result<String, String> {
    let mut random = vec![0; bytes];
    OsRng
        .try_fill_bytes(&mut random)
        .map_err(|_| "could not create a transaction identifier".to_owned())?;
    Ok(random.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub(super) struct FileChange {
    pub path: String,
    pub before: Option<Vec<u8>>,
    pub after: Option<Vec<u8>>,
    pub create_mode: u32,
}

fn journal_root(root: &Path, create: bool) -> Result<PathBuf, String> {
    let metadata_dir = root.join(".zelyra");
    if create && !metadata_dir.exists() {
        fs::create_dir(&metadata_dir)
            .map_err(|error| format!("cannot create Studio state directory: {error}"))?;
    }
    if metadata_dir.exists() {
        let metadata = fs::symlink_metadata(&metadata_dir)
            .map_err(|error| format!("cannot inspect Studio state directory: {error}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err("Studio state path must be a real directory".into());
        }
    } else {
        return Ok(metadata_dir.join(JOURNAL_DIRECTORY));
    }
    let journal = metadata_dir.join(JOURNAL_DIRECTORY);
    if create && !journal.exists() {
        fs::create_dir(&journal)
            .map_err(|error| format!("cannot create Studio transaction directory: {error}"))?;
    }
    if journal.exists() {
        let metadata = fs::symlink_metadata(&journal)
            .map_err(|error| format!("cannot inspect Studio transaction directory: {error}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err("Studio transaction path must be a real directory".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&journal, fs::Permissions::from_mode(0o700))
                .map_err(|error| format!("cannot protect Studio transaction directory: {error}"))?;
        }
    }
    Ok(journal)
}

fn write_sync(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("cannot create transaction file: {error}"))?;
    file.write_all(bytes)
        .map_err(|error| format!("cannot write transaction file: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("cannot sync transaction file: {error}"))
}

fn sync_directory(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("cannot sync transaction directory: {error}"))?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn atomic_write(target: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = target.parent().ok_or("file has no parent directory")?;
    let name = target
        .file_name()
        .ok_or("file has no name")?
        .to_string_lossy();
    for _ in 0..8 {
        let suffix = random_hex(8)?;
        let temporary = parent.join(format!(".{name}.studio-{suffix}.tmp"));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&temporary) {
            Ok(mut file) => {
                let result = (|| {
                    file.write_all(bytes).map_err(|error| error.to_string())?;
                    file.sync_all().map_err(|error| error.to_string())?;
                    let metadata = fs::metadata(target).map_err(|error| error.to_string())?;
                    fs::set_permissions(&temporary, metadata.permissions())
                        .map_err(|error| error.to_string())?;
                    fs::rename(&temporary, target).map_err(|error| error.to_string())?;
                    sync_directory(parent)
                })();
                if result.is_err() {
                    let _ = fs::remove_file(temporary);
                }
                return result.map_err(|error| format!("atomic file replacement failed: {error}"));
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("cannot create atomic replacement: {error}")),
        }
    }
    Err("cannot allocate a unique atomic replacement file".into())
}

fn atomic_create(target: &Path, bytes: &[u8], mode: u32) -> Result<(), String> {
    let parent = target.parent().ok_or("file has no parent directory")?;
    let name = target
        .file_name()
        .ok_or("file has no name")?
        .to_string_lossy();
    for _ in 0..8 {
        let temporary = parent.join(format!(".{name}.studio-{}.tmp", random_hex(8)?));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(mode);
        }
        match options.open(&temporary) {
            Ok(mut file) => {
                let result = (|| {
                    file.write_all(bytes).map_err(|error| error.to_string())?;
                    file.sync_all().map_err(|error| error.to_string())?;
                    fs::hard_link(&temporary, target).map_err(|error| error.to_string())?;
                    fs::remove_file(&temporary).map_err(|error| error.to_string())?;
                    sync_directory(parent)
                })();
                if result.is_err() {
                    let _ = fs::remove_file(&temporary);
                }
                return result.map_err(|error| format!("atomic file creation failed: {error}"));
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("cannot create atomic file: {error}")),
        }
    }
    Err("cannot allocate a unique atomic creation file".into())
}

fn optional_hash(bytes: Option<&[u8]>) -> Option<String> {
    bytes.map(content_hash)
}

pub(super) fn file_mode(path: &Path) -> Result<u32, String> {
    let metadata =
        fs::metadata(path).map_err(|error| format!("cannot inspect file permissions: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        Ok(metadata.permissions().mode() & 0o7777)
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        Ok(0o666)
    }
}

fn rollback(root: &Path, directory: &Path, manifest: &Value) -> Result<(), String> {
    if manifest.get("schema").and_then(Value::as_u64) != Some(1) {
        return Err("unsupported transaction manifest schema".into());
    }
    let entries = manifest
        .get("files")
        .and_then(Value::as_array)
        .ok_or("transaction manifest has no file list")?;
    if entries.is_empty() || entries.len() > MAX_TRANSACTION_FILES {
        return Err("transaction manifest has an invalid file count".into());
    }
    let mut seen = HashSet::new();
    for entry in entries.iter().rev() {
        let relative = entry
            .get("path")
            .and_then(Value::as_str)
            .ok_or("transaction entry has no path")?;
        if !seen.insert(relative) {
            return Err("transaction manifest contains a duplicate path".into());
        }
        let path = safe_editor_path(root, relative, true).map_err(|(_, error)| {
            format!("cannot recover transaction path `{relative}`: {error}")
        })?;
        let before_hash = entry
            .get("beforeHash")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let after_hash = entry
            .get("afterHash")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let current = match fs::read(&path) {
            Ok(content) => Some(content),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(format!(
                    "cannot read transaction target `{relative}`: {error}"
                ))
            }
        };
        let current_hash = optional_hash(current.as_deref());
        if current_hash == before_hash {
            continue;
        }
        if current_hash != after_hash {
            return Err(format!(
                "transaction recovery found a conflicting edit in `{relative}`"
            ));
        }
        let backup = entry
            .get("backup")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty());
        let before = if let Some(backup) = backup {
            if Path::new(backup).components().count() != 1 || !backup.starts_with("backup-") {
                return Err("transaction backup path is invalid".into());
            }
            let backup_path = directory.join(backup);
            let backup_metadata = fs::symlink_metadata(&backup_path)
                .map_err(|error| format!("cannot inspect transaction backup: {error}"))?;
            if backup_metadata.file_type().is_symlink()
                || !backup_metadata.is_file()
                || backup_metadata.len() > EDITOR_MAX_FILE_BYTES as u64
            {
                return Err("transaction backup is not a safe regular file".into());
            }
            let before = fs::read(backup_path)
                .map_err(|error| format!("cannot read transaction backup: {error}"))?;
            if optional_hash(Some(&before)) != before_hash {
                return Err(format!(
                    "transaction backup for `{relative}` failed its hash check"
                ));
            }
            Some(before)
        } else {
            if before_hash.is_some() {
                return Err("transaction backup is missing".into());
            }
            None
        };
        match before {
            Some(bytes) if path.exists() => atomic_write(&path, &bytes)?,
            Some(bytes) => {
                let mode = entry
                    .get("createMode")
                    .and_then(Value::as_u64)
                    .unwrap_or(0o644) as u32;
                atomic_create(&path, &bytes, mode)?;
            }
            None if path.exists() => {
                fs::remove_file(&path)
                    .map_err(|error| format!("cannot remove created file `{relative}`: {error}"))?;
                if let Some(parent) = path.parent() {
                    sync_directory(parent)?;
                }
            }
            None => {}
        }
    }
    let parent = directory
        .parent()
        .ok_or("transaction has no parent")?
        .to_path_buf();
    fs::remove_dir_all(directory)
        .map_err(|error| format!("cannot remove recovered transaction: {error}"))?;
    sync_directory(&parent)
}

pub(super) fn recover(root: &Path) -> Result<(), String> {
    let journal = journal_root(root, false)?;
    if !journal.exists() {
        return Ok(());
    }
    for item in fs::read_dir(&journal)
        .map_err(|error| format!("cannot read transaction directory: {error}"))?
    {
        let item = item.map_err(|error| format!("cannot inspect transaction entry: {error}"))?;
        let metadata = fs::symlink_metadata(item.path()).map_err(|error| error.to_string())?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err("unexpected entry in Studio transaction directory".into());
        }
        let name = item.file_name();
        if !name.to_string_lossy().starts_with("txn-") {
            return Err("unexpected transaction directory name".into());
        }
        let directory = item.path();
        let committed_path = directory.join("committed");
        if fs::symlink_metadata(&committed_path)
            .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
        {
            fs::remove_dir_all(&directory)
                .map_err(|error| format!("cannot clean committed transaction: {error}"))?;
            sync_directory(&journal)?;
            continue;
        }
        if fs::symlink_metadata(directory.join("prepared")).is_err() {
            // Changes are never applied before a synced prepared marker exists.
            fs::remove_dir_all(&directory)
                .map_err(|error| format!("cannot clean unprepared transaction: {error}"))?;
            sync_directory(&journal)?;
            continue;
        }
        let manifest_path = directory.join("manifest.json");
        let manifest_metadata = fs::symlink_metadata(&manifest_path)
            .map_err(|error| format!("cannot inspect pending transaction manifest: {error}"))?;
        if manifest_metadata.file_type().is_symlink()
            || !manifest_metadata.is_file()
            || manifest_metadata.len() > MAX_MANIFEST_BYTES
        {
            return Err("pending transaction manifest is not a safe regular file".into());
        }
        let manifest_bytes = fs::read(&manifest_path)
            .map_err(|error| format!("cannot read pending transaction manifest: {error}"))?;
        let manifest: Value = serde_json::from_slice(&manifest_bytes)
            .map_err(|error| format!("invalid pending transaction manifest: {error}"))?;
        rollback(root, &directory, &manifest)?;
    }
    Ok(())
}

pub(super) fn apply(root: &Path, changes: &[FileChange]) -> Result<(), String> {
    if changes.is_empty() {
        return Ok(());
    }
    if changes.len() > MAX_TRANSACTION_FILES {
        return Err(format!(
            "transaction exceeds the {MAX_TRANSACTION_FILES}-file safety limit"
        ));
    }
    let total_bytes = changes.iter().fold(0_usize, |total, change| {
        total
            .saturating_add(change.before.as_ref().map_or(0, Vec::len))
            .saturating_add(change.after.as_ref().map_or(0, Vec::len))
    });
    if total_bytes > MAX_TRANSACTION_BYTES {
        return Err("transaction exceeds the 8 MiB safety limit".into());
    }
    let journal_root = journal_root(root, true)?;
    let directory = journal_root.join(format!("txn-{}", random_hex(16)?));
    fs::create_dir(&directory)
        .map_err(|error| format!("cannot start transaction journal: {error}"))?;
    sync_directory(&journal_root)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("cannot protect transaction journal: {error}"))?;
    }
    let mut files = Vec::with_capacity(changes.len());
    let mut seen = HashSet::new();
    let preparation = (|| {
        for (index, change) in changes.iter().enumerate() {
            if !seen.insert(&change.path) {
                return Err(format!(
                    "transaction contains duplicate path `{}`",
                    change.path
                ));
            }
            let path = safe_editor_path(root, &change.path, change.before.is_none())
                .map_err(|(_, error)| format!("invalid transaction path: {error}"))?;
            let current = match fs::read(&path) {
                Ok(content) => Some(content),
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => return Err(format!("cannot read `{}`: {error}", change.path)),
            };
            if current.as_deref() != change.before.as_deref() {
                return Err(format!("`{}` changed after it was opened", change.path));
            }
            let backup = format!("backup-{index:04}");
            if let Some(before) = &change.before {
                write_sync(&directory.join(&backup), before)?;
            }
            files.push(json!({
                "path":change.path,"beforeHash":optional_hash(change.before.as_deref()),
                "afterHash":optional_hash(change.after.as_deref()),
                "backup":if change.before.is_some() { backup } else { String::new() },
                "createMode":change.create_mode
            }));
        }
        let manifest = json!({"schema":1,"files":files});
        let manifest_bytes = manifest.to_string();
        if manifest_bytes.len() > MAX_MANIFEST_BYTES as usize {
            return Err("transaction manifest exceeds its size limit".into());
        }
        write_sync(&directory.join("manifest.json"), manifest_bytes.as_bytes())?;
        sync_directory(&directory)?;
        write_sync(&directory.join("prepared"), b"prepared")?;
        sync_directory(&directory)?;
        Ok::<_, String>(manifest)
    })();
    let manifest = match preparation {
        Ok(manifest) => manifest,
        Err(error) => {
            let _ = fs::remove_dir_all(&directory);
            return Err(error);
        }
    };
    let result = (|| {
        for change in changes {
            let path = safe_editor_path(root, &change.path, change.before.is_none())
                .map_err(|(_, error)| format!("invalid transaction path: {error}"))?;
            let current = match fs::read(&path) {
                Ok(content) => Some(content),
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => return Err(format!("cannot reread `{}`: {error}", change.path)),
            };
            if current.as_deref() != change.before.as_deref() {
                return Err(format!("`{}` changed during the save", change.path));
            }
            match &change.after {
                Some(after) if current.is_some() => atomic_write(&path, after)?,
                Some(after) => atomic_create(&path, after, change.create_mode)?,
                None if current.is_some() => {
                    fs::remove_file(&path)
                        .map_err(|error| format!("cannot remove `{}`: {error}", change.path))?;
                    if let Some(parent) = path.parent() {
                        sync_directory(parent)?;
                    }
                }
                None => {}
            }
        }
        write_sync(&directory.join("committed"), b"committed")?;
        sync_directory(&directory)?;
        Ok(())
    })();
    if let Err(error) = result {
        match rollback(root, &directory, &manifest) {
            Ok(()) => return Err(error),
            Err(recovery_error) => {
                return Err(format!(
                    "{error}; rollback is journaled for next startup: {recovery_error}"
                ));
            }
        }
    }
    let _ = fs::remove_dir_all(&directory);
    let _ = sync_directory(&journal_root);
    Ok(())
}
