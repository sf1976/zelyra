//! Crash-recoverable, hash-checked transactions for Studio multi-file saves.
use super::*;
use serde_json::Value;

const JOURNAL_DIRECTORY: &str = "studio-transactions";

fn random_hex(bytes: usize) -> Result<String, String> {
    let mut random = vec![0; bytes];
    OsRng
        .try_fill_bytes(&mut random)
        .map_err(|_| "could not create a transaction identifier".to_owned())?;
    Ok(random.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub(super) struct FileChange {
    pub path: String,
    pub before: Vec<u8>,
    pub after: Vec<u8>,
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

fn rollback(root: &Path, directory: &Path, manifest: &Value) -> Result<(), String> {
    let entries = manifest
        .get("files")
        .and_then(Value::as_array)
        .ok_or("transaction manifest has no file list")?;
    for entry in entries.iter().rev() {
        let relative = entry
            .get("path")
            .and_then(Value::as_str)
            .ok_or("transaction entry has no path")?;
        let path = safe_editor_path(root, relative, false).map_err(|(_, error)| {
            format!("cannot recover transaction path `{relative}`: {error}")
        })?;
        let before_hash = entry
            .get("beforeHash")
            .and_then(Value::as_str)
            .ok_or("missing before hash")?;
        let after_hash = entry
            .get("afterHash")
            .and_then(Value::as_str)
            .ok_or("missing after hash")?;
        let current = fs::read(&path)
            .map_err(|error| format!("cannot read transaction target `{relative}`: {error}"))?;
        let current_hash = content_hash(&current);
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
            .ok_or("missing backup name")?;
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
        if content_hash(&before) != before_hash {
            return Err(format!(
                "transaction backup for `{relative}` failed its hash check"
            ));
        }
        atomic_write(&path, &before)?;
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
            || manifest_metadata.len() > 65_536
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
    let preparation = (|| {
        for (index, change) in changes.iter().enumerate() {
            let path = safe_editor_path(root, &change.path, false)
                .map_err(|(_, error)| format!("invalid transaction path: {error}"))?;
            let current = fs::read(&path)
                .map_err(|error| format!("cannot read `{}`: {error}", change.path))?;
            if current != change.before {
                return Err(format!("`{}` changed after it was opened", change.path));
            }
            let backup = format!("backup-{index:04}");
            write_sync(&directory.join(&backup), &change.before)?;
            files.push(json!({
                "path":change.path,"beforeHash":content_hash(&change.before),
                "afterHash":content_hash(&change.after),"backup":backup
            }));
        }
        let manifest = json!({"schema":1,"files":files});
        write_sync(
            &directory.join("manifest.json"),
            manifest.to_string().as_bytes(),
        )?;
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
            let path = safe_editor_path(root, &change.path, false)
                .map_err(|(_, error)| format!("invalid transaction path: {error}"))?;
            let current = fs::read(&path)
                .map_err(|error| format!("cannot reread `{}`: {error}", change.path))?;
            if current != change.before {
                return Err(format!("`{}` changed during the save", change.path));
            }
            atomic_write(&path, &change.after)?;
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
    fs::remove_dir_all(&directory)
        .map_err(|error| format!("save committed but journal cleanup failed: {error}"))?;
    sync_directory(&journal_root)
}
