use super::{
    create_private_file, detect_media_type, is_canonical_uuid, open_private_file,
    set_private_directory_permissions, sha256_hex, storage_error, FileStorage, FileStorageError,
    StoredFile, MAX_UPLOAD_BYTES,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const BACKUP_FORMAT_VERSION: u32 = 1;
const MAX_BACKUP_FILES: usize = 100_000;
const MAX_BACKUP_BYTES: u64 = 10 * 1024 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackupSummary {
    pub files: usize,
    pub total_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BackupManifest {
    format_version: u32,
    files: Vec<BackupEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BackupEntry {
    tenant_id: i64,
    file_ref: String,
    media_type: String,
    size_bytes: u64,
    sha256: String,
}

impl FileStorage {
    /// Create a complete private-storage snapshot in a new directory. The
    /// database is deliberately not included: callers must back it up
    /// separately while the application is quiescent. The exclusive activity
    /// lock prevents Zelyra upload servers and cleanup from racing this copy.
    pub fn backup_to_directory(
        &self,
        output_directory: impl AsRef<Path>,
        required_references: &HashSet<(i64, String)>,
    ) -> Result<BackupSummary, FileStorageError> {
        let _maintenance = self.acquire_maintenance_lock()?;
        let (parent, output) = new_destination(output_directory.as_ref())?;
        if parent.starts_with(&self.root) {
            return Err(FileStorageError(
                "upload backup destination must be outside the live private storage root".into(),
            ));
        }
        let staging = TemporaryDirectory::create(parent.join(format!(
            ".zelyra-upload-backup-{}.stage",
            super::random_uuid()?
        )))?;

        let result = self.write_backup_snapshot(staging.path(), required_references);
        match result {
            Ok(summary) => {
                sync_directory(staging.path()).map_err(storage_error)?;
                fs::rename(staging.path(), &output).map_err(storage_error)?;
                sync_directory(&parent).map_err(storage_error)?;
                Ok(summary)
            }
            Err(error) => Err(error),
        }
    }

    /// Validate a complete backup in a private staging directory before
    /// atomically making its tenant tree available. Existing managed uploads
    /// are never overwritten. Every database reference supplied by the caller
    /// must be present and match its checksum before publication.
    pub fn restore_from_directory(
        &self,
        backup_directory: impl AsRef<Path>,
        required_references: &HashSet<(i64, String)>,
    ) -> Result<BackupSummary, FileStorageError> {
        let _maintenance = self.acquire_maintenance_lock()?;
        let backup_root = checked_directory(backup_directory.as_ref())?;
        if backup_root.starts_with(&self.root) {
            return Err(FileStorageError(
                "upload backup source must be outside the live private storage root".into(),
            ));
        }
        let backup_tenants = backup_root.join("tenants");
        ensure_backup_directory(&backup_root, &backup_tenants)?;
        let manifest_path = backup_root.join("manifest.json");
        let manifest_metadata = fs::symlink_metadata(&manifest_path).map_err(storage_error)?;
        if manifest_metadata.file_type().is_symlink()
            || !manifest_metadata.is_file()
            || manifest_metadata.len() > MAX_MANIFEST_BYTES
        {
            return Err(FileStorageError(
                "upload backup manifest is not a bounded regular file".into(),
            ));
        }
        let manifest_bytes = read_limited(&manifest_path, MAX_MANIFEST_BYTES)?;
        let manifest: BackupManifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|_| FileStorageError("upload backup manifest is invalid".into()))?;
        if manifest.format_version != BACKUP_FORMAT_VERSION
            || manifest.files.len() > MAX_BACKUP_FILES
        {
            return Err(FileStorageError(
                "upload backup format version or file count is unsupported".into(),
            ));
        }

        let stage = TemporaryDirectory::create(self.root.join(format!(
            ".zelyra-upload-restore-{}.stage",
            super::random_uuid()?
        )))?;
        let staged_tenants = stage.path().join("tenants");
        fs::create_dir(&staged_tenants).map_err(storage_error)?;
        if let Err(error) = set_private_directory_permissions(&staged_tenants) {
            return Err(storage_error(error));
        }

        let result =
            self.stage_restore(&backup_root, &staged_tenants, manifest, required_references);
        let summary = result?;

        let live_tenants = self.root.join("tenants");
        match fs::symlink_metadata(&live_tenants) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(FileStorageError(
                    "live upload tenant root is not a real directory".into(),
                ));
            }
            Ok(_) => {
                let mut entries = fs::read_dir(&live_tenants).map_err(storage_error)?;
                if entries.next().transpose().map_err(storage_error)?.is_some() {
                    return Err(FileStorageError(
                        "restore refuses to overwrite existing managed uploads".into(),
                    ));
                }
                fs::remove_dir(&live_tenants).map_err(storage_error)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(storage_error(error));
            }
        }

        fs::rename(&staged_tenants, &live_tenants).map_err(storage_error)?;
        if let Err(error) = sync_directory(&self.root) {
            let _ = fs::remove_dir_all(&live_tenants);
            return Err(storage_error(error));
        }
        Ok(summary)
    }

    fn write_backup_snapshot(
        &self,
        staging: &Path,
        required_references: &HashSet<(i64, String)>,
    ) -> Result<BackupSummary, FileStorageError> {
        let pairs = self.live_file_pairs()?;
        let backup_tenants = staging.join("tenants");
        fs::create_dir(&backup_tenants).map_err(storage_error)?;
        set_private_directory_permissions(&backup_tenants).map_err(storage_error)?;
        let mut found = HashSet::new();
        let mut files = Vec::with_capacity(pairs.len());
        let mut total_bytes = 0_u64;

        for (tenant_id, file_ref) in pairs {
            let (metadata, bytes) = self.read_verified(tenant_id, &file_ref, MAX_UPLOAD_BYTES)?;
            total_bytes = total_bytes
                .checked_add(metadata.size_bytes)
                .filter(|total| *total <= MAX_BACKUP_BYTES)
                .ok_or_else(|| FileStorageError("upload backup exceeds its 10 GiB limit".into()))?;
            found.insert((tenant_id, file_ref.clone()));
            let tenant_directory = backup_tenants.join(tenant_id.to_string());
            create_private_directory(&tenant_directory)?;
            write_new_private_file(&tenant_directory.join(&file_ref), &bytes)?;
            write_new_private_file(
                &tenant_directory.join(format!("{file_ref}.meta")),
                manifest_bytes(&metadata).as_bytes(),
            )?;
            sync_directory(&tenant_directory).map_err(storage_error)?;
            files.push(BackupEntry {
                tenant_id,
                file_ref,
                media_type: metadata.media_type.to_owned(),
                size_bytes: metadata.size_bytes,
                sha256: metadata.sha256,
            });
        }

        if let Some((tenant_id, file_ref)) = required_references
            .iter()
            .find(|reference| !found.contains(*reference))
        {
            return Err(FileStorageError(format!(
                "upload backup is incomplete: database reference tenant={tenant_id} file_ref={file_ref} has no verified stored file"
            )));
        }
        files.sort_by(|left, right| {
            (left.tenant_id, left.file_ref.as_str())
                .cmp(&(right.tenant_id, right.file_ref.as_str()))
        });
        let manifest = BackupManifest {
            format_version: BACKUP_FORMAT_VERSION,
            files,
        };
        let encoded = serde_json::to_vec_pretty(&manifest)
            .map_err(|_| FileStorageError("could not encode upload backup manifest".into()))?;
        if encoded.len() as u64 > MAX_MANIFEST_BYTES {
            return Err(FileStorageError(
                "upload backup manifest is too large".into(),
            ));
        }
        write_new_private_file(&staging.join("manifest.json"), &encoded)?;
        sync_directory(&backup_tenants).map_err(storage_error)?;
        Ok(BackupSummary {
            files: manifest.files.len(),
            total_bytes,
        })
    }

    fn stage_restore(
        &self,
        backup_root: &Path,
        staged_tenants: &Path,
        manifest: BackupManifest,
        required_references: &HashSet<(i64, String)>,
    ) -> Result<BackupSummary, FileStorageError> {
        let mut seen = HashSet::new();
        let mut total_bytes = 0_u64;
        for entry in &manifest.files {
            if entry.tenant_id < 0
                || !is_canonical_uuid(&entry.file_ref)
                || !is_sha256_hex(&entry.sha256)
                || !matches!(
                    entry.media_type.as_str(),
                    "application/pdf" | "image/png" | "image/jpeg"
                )
                || entry.size_bytes == 0
                || entry.size_bytes > MAX_UPLOAD_BYTES
                || !seen.insert((entry.tenant_id, entry.file_ref.clone()))
            {
                return Err(FileStorageError(
                    "upload backup manifest contains an invalid or duplicate file entry".into(),
                ));
            }
            total_bytes = total_bytes
                .checked_add(entry.size_bytes)
                .filter(|total| *total <= MAX_BACKUP_BYTES)
                .ok_or_else(|| FileStorageError("upload backup exceeds its 10 GiB limit".into()))?;

            let source_tenant = backup_root
                .join("tenants")
                .join(entry.tenant_id.to_string());
            ensure_backup_directory(backup_root, &source_tenant)?;
            let source_blob = source_tenant.join(&entry.file_ref);
            let source_manifest = source_tenant.join(format!("{}.meta", entry.file_ref));
            let bytes = read_limited(&source_blob, MAX_UPLOAD_BYTES)?;
            let stored_manifest = read_limited(&source_manifest, 512)?;
            let expected_manifest = format!(
                "{}\n{}\n{}\n",
                entry.media_type, entry.size_bytes, entry.sha256
            );
            if stored_manifest != expected_manifest.as_bytes()
                || bytes.len() as u64 != entry.size_bytes
                || !constant_time_hex_eq(&sha256_hex(&bytes), &entry.sha256)
                || detect_media_type(&bytes) != Some(entry.media_type.as_str())
            {
                return Err(FileStorageError(format!(
                    "upload backup checksum or metadata verification failed for tenant={} file_ref={}",
                    entry.tenant_id, entry.file_ref
                )));
            }

            let target_tenant = staged_tenants.join(entry.tenant_id.to_string());
            create_private_directory(&target_tenant)?;
            write_new_private_file(&target_tenant.join(&entry.file_ref), &bytes)?;
            write_new_private_file(
                &target_tenant.join(format!("{}.meta", entry.file_ref)),
                &stored_manifest,
            )?;
            sync_directory(&target_tenant).map_err(storage_error)?;
        }

        if let Some((tenant_id, file_ref)) = required_references
            .iter()
            .find(|reference| !seen.contains(*reference))
        {
            return Err(FileStorageError(format!(
                "upload restore is incomplete: database reference tenant={tenant_id} file_ref={file_ref} is missing from the backup"
            )));
        }
        sync_directory(staged_tenants).map_err(storage_error)?;
        Ok(BackupSummary {
            files: manifest.files.len(),
            total_bytes,
        })
    }

    fn live_file_pairs(&self) -> Result<Vec<(i64, String)>, FileStorageError> {
        let tenants = self.root.join("tenants");
        let tenants_metadata = match fs::symlink_metadata(&tenants) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(FileStorageError(
                    "private upload tenant root is not a real directory".into(),
                ));
            }
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(storage_error(error)),
        };
        let _ = tenants_metadata;
        let mut scanned = 0_usize;
        let mut pairs = Vec::new();
        for tenant in fs::read_dir(&tenants).map_err(storage_error)? {
            scanned = scanned.saturating_add(1);
            if scanned > MAX_BACKUP_FILES {
                return Err(FileStorageError(
                    "upload backup exceeded 100,000 entries".into(),
                ));
            }
            let tenant = tenant.map_err(storage_error)?;
            let tenant_name = tenant.file_name();
            let tenant_name = tenant_name.to_str().ok_or_else(|| {
                FileStorageError("upload tenant directory name is invalid".into())
            })?;
            let tenant_id = tenant_name
                .parse::<i64>()
                .ok()
                .filter(|id| *id >= 0 && id.to_string() == tenant_name)
                .ok_or_else(|| {
                    FileStorageError("upload tenant directory name is invalid".into())
                })?;
            let directory = tenant.path();
            let metadata = fs::symlink_metadata(&directory).map_err(storage_error)?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(FileStorageError(
                    "upload tenant entry is not a real directory".into(),
                ));
            }
            let mut blobs = HashSet::new();
            let mut manifests = HashSet::new();
            for item in fs::read_dir(&directory).map_err(storage_error)? {
                scanned = scanned.saturating_add(1);
                if scanned > MAX_BACKUP_FILES {
                    return Err(FileStorageError(
                        "upload backup exceeded 100,000 entries".into(),
                    ));
                }
                let item = item.map_err(storage_error)?;
                let name = item.file_name();
                let name = name
                    .to_str()
                    .ok_or_else(|| FileStorageError("upload artifact name is invalid".into()))?;
                let metadata = fs::symlink_metadata(item.path()).map_err(storage_error)?;
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(FileStorageError(
                        "upload backup found a symlink or non-regular artifact".into(),
                    ));
                }
                if is_canonical_uuid(name) {
                    blobs.insert(name.to_owned());
                } else if let Some(file_ref) = name.strip_suffix(".meta") {
                    if !is_canonical_uuid(file_ref) {
                        return Err(FileStorageError("upload manifest name is invalid".into()));
                    }
                    manifests.insert(file_ref.to_owned());
                } else {
                    return Err(FileStorageError(
                        "upload storage contains staging or unrecognized artifacts; resolve them before backup".into(),
                    ));
                }
            }
            if blobs != manifests {
                return Err(FileStorageError(
                    "upload storage contains an incomplete blob/manifest pair".into(),
                ));
            }
            for file_ref in blobs {
                pairs.push((tenant_id, file_ref));
            }
        }
        pairs.sort();
        if pairs.len() > MAX_BACKUP_FILES {
            return Err(FileStorageError(
                "upload backup exceeded 100,000 files".into(),
            ));
        }
        Ok(pairs)
    }
}

fn new_destination(path: &Path) -> Result<(PathBuf, PathBuf), FileStorageError> {
    let name = path
        .file_name()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| FileStorageError("upload backup output must name a new directory".into()))?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = fs::canonicalize(parent).map_err(storage_error)?;
    let output = parent.join(name);
    match fs::symlink_metadata(&output) {
        Ok(_) => Err(FileStorageError(
            "upload backup output already exists".into(),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok((parent, output)),
        Err(error) => Err(storage_error(error)),
    }
}

fn checked_directory(path: &Path) -> Result<PathBuf, FileStorageError> {
    let metadata = fs::symlink_metadata(path).map_err(storage_error)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(FileStorageError(
            "upload backup source is not a real directory".into(),
        ));
    }
    fs::canonicalize(path).map_err(storage_error)
}

fn ensure_backup_directory(root: &Path, path: &Path) -> Result<(), FileStorageError> {
    let metadata = fs::symlink_metadata(path).map_err(storage_error)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(FileStorageError(
            "upload backup contains an unsafe directory".into(),
        ));
    }
    let canonical = fs::canonicalize(path).map_err(storage_error)?;
    if !canonical.starts_with(root) {
        return Err(FileStorageError(
            "upload backup path escapes its root".into(),
        ));
    }
    Ok(())
}

fn create_private_directory(path: &Path) -> Result<(), FileStorageError> {
    fs::create_dir(path).map_err(storage_error)?;
    set_private_directory_permissions(path).map_err(storage_error)
}

fn write_new_private_file(path: &Path, bytes: &[u8]) -> Result<(), FileStorageError> {
    let mut file = create_private_file(path).map_err(storage_error)?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(storage_error)
}

struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    fn create(path: PathBuf) -> Result<Self, FileStorageError> {
        fs::create_dir(&path).map_err(storage_error)?;
        let guard = Self(path);
        if let Err(error) = set_private_directory_permissions(&guard.0) {
            return Err(storage_error(error));
        }
        Ok(guard)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn read_limited(path: &Path, max_bytes: u64) -> Result<Vec<u8>, FileStorageError> {
    let metadata = fs::symlink_metadata(path).map_err(storage_error)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > max_bytes {
        return Err(FileStorageError(
            "upload backup artifact is not a bounded regular file".into(),
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    open_private_file(path)
        .map_err(storage_error)?
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(storage_error)?;
    if bytes.len() as u64 != metadata.len() || bytes.len() as u64 > max_bytes {
        return Err(FileStorageError(
            "upload backup artifact changed while reading".into(),
        ));
    }
    Ok(bytes)
}

fn manifest_bytes(metadata: &StoredFile) -> String {
    format!(
        "{}\n{}\n{}\n",
        metadata.media_type, metadata.size_bytes, metadata.sha256
    )
}

fn sync_directory(path: &Path) -> std::io::Result<()> {
    File::open(path)?.sync_all()
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn constant_time_hex_eq(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.bytes()
        .zip(right.bytes())
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "zelyra-upload-backup-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    fn create_file(storage: &FileStorage) -> StoredFile {
        storage
            .stage(
                7,
                b"%PDF-1.7\nbackup test document\n",
                1024,
                &["application/pdf".into()],
            )
            .expect("stage PDF")
            .publish()
            .expect("publish PDF")
    }

    #[test]
    fn backup_and_restore_preserve_file_identity_bytes_and_manifest() {
        let root = temporary_root("roundtrip");
        let live = FileStorage::new(root.join("live")).expect("create live store");
        let expected = create_file(&live);
        let references = HashSet::from([(expected.tenant_id, expected.file_ref.clone())]);
        let backup = root.join("backup");

        let written = live
            .backup_to_directory(&backup, &references)
            .expect("write storage backup");
        assert_eq!(written.files, 1);
        assert_eq!(written.total_bytes, expected.size_bytes);
        fs::remove_dir_all(live.root.join("tenants")).expect("clear live files");

        let restored = live
            .restore_from_directory(&backup, &references)
            .expect("restore storage backup");
        assert_eq!(restored, written);
        let (actual, bytes) = live
            .read_verified(expected.tenant_id, &expected.file_ref, MAX_UPLOAD_BYTES)
            .expect("read restored file");
        assert_eq!(actual, expected);
        assert_eq!(bytes, b"%PDF-1.7\nbackup test document\n");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn corrupt_backup_is_rejected_before_any_file_is_published() {
        let root = temporary_root("corrupt");
        let live = FileStorage::new(root.join("live")).expect("create live store");
        let stored = create_file(&live);
        let references = HashSet::from([(stored.tenant_id, stored.file_ref.clone())]);
        let backup = root.join("backup");
        live.backup_to_directory(&backup, &references)
            .expect("write storage backup");
        fs::remove_dir_all(live.root.join("tenants")).expect("clear live files");
        fs::write(
            backup
                .join("tenants")
                .join(stored.tenant_id.to_string())
                .join(&stored.file_ref),
            b"%PDF-1.7\ntampered\n",
        )
        .expect("tamper backup blob");

        let error = live
            .restore_from_directory(&backup, &references)
            .expect_err("reject corrupt backup");
        assert!(error
            .to_string()
            .contains("checksum or metadata verification failed"));
        assert!(!live.root.join("tenants").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn backup_refuses_missing_database_files_without_publishing_partial_output() {
        let root = temporary_root("missing-reference");
        let live = FileStorage::new(root.join("live")).expect("create live store");
        let missing = HashSet::from([(7, "123e4567-e89b-42d3-a456-426614174000".to_owned())]);
        let backup = root.join("backup");

        let error = live
            .backup_to_directory(&backup, &missing)
            .expect_err("missing reference makes the backup incomplete");
        assert!(error.to_string().contains("upload backup is incomplete"));
        assert!(!backup.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restore_refuses_database_references_missing_from_backup() {
        let root = temporary_root("restore-missing-reference");
        let live = FileStorage::new(root.join("live")).expect("create live store");
        let stored = create_file(&live);
        let included = HashSet::from([(stored.tenant_id, stored.file_ref.clone())]);
        let backup = root.join("backup");
        live.backup_to_directory(&backup, &included)
            .expect("write storage backup");
        fs::remove_dir_all(live.root.join("tenants")).expect("clear live files");
        let required = HashSet::from([
            (stored.tenant_id, stored.file_ref.clone()),
            (
                stored.tenant_id,
                "123e4567-e89b-42d3-a456-426614174000".to_owned(),
            ),
        ]);

        let error = live
            .restore_from_directory(&backup, &required)
            .expect_err("missing database reference makes restore incomplete");
        assert!(error.to_string().contains("upload restore is incomplete"));
        assert!(!live.root.join("tenants").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restore_never_overwrites_existing_managed_files() {
        let root = temporary_root("no-overwrite");
        let live = FileStorage::new(root.join("live")).expect("create live store");
        let expected = create_file(&live);
        let references = HashSet::from([(expected.tenant_id, expected.file_ref.clone())]);
        let backup = root.join("backup");
        live.backup_to_directory(&backup, &references)
            .expect("write storage backup");

        let error = live
            .restore_from_directory(&backup, &references)
            .expect_err("refuse restoring over live files");
        assert!(error.to_string().contains("refuses to overwrite"));
        assert_eq!(
            live.read_verified(expected.tenant_id, &expected.file_ref, MAX_UPLOAD_BYTES)
                .expect("original remains readable")
                .0,
            expected
        );
        let _ = fs::remove_dir_all(root);
    }
}
