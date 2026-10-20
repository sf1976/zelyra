//! Private, tenant-scoped storage primitives for validated uploads.
//!
//! This module intentionally does not decide whether a caller is authorized
//! to store or retrieve a file, nor does it coordinate a database commit.
//! Callers must validate the upload policy and commit metadata before exposing
//! the returned opaque reference.

use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    time::SystemTime,
};

const MAX_UPLOAD_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredFile {
    /// A random UUID-shaped identifier, not a path or client filename.
    pub file_ref: String,
    pub tenant_id: i64,
    pub media_type: &'static str,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OldUnreferencedUpload {
    pub tenant_id: i64,
    pub file_ref: String,
    pub size_bytes: u64,
    pub modified_at: SystemTime,
    pub staging_artifact: bool,
}

#[derive(Debug)]
pub struct StagedUpload {
    metadata: StoredFile,
    staging_path: Option<PathBuf>,
    metadata_staging_path: Option<PathBuf>,
    final_path: PathBuf,
    metadata_final_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileStorageError(pub String);

impl std::fmt::Display for FileStorageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for FileStorageError {}

/// Storage root for application-private upload blobs.
#[derive(Clone, Debug)]
pub struct FileStorage {
    root: PathBuf,
}

/// Advisory process-lifetime lock for managed upload storage. Web servers hold
/// a shared lock; destructive maintenance requires a non-blocking exclusive
/// lock. This coordinates Zelyra processes on one host, not arbitrary external
/// programs or network filesystems.
#[derive(Debug)]
pub struct StorageActivityLock {
    _file: File,
}

impl FileStorage {
    /// Creates or opens a private storage root. The configured root itself may
    /// resolve through a symlink, but generated descendants may not.
    pub fn new(root: impl AsRef<Path>) -> Result<Self, FileStorageError> {
        fs::create_dir_all(root.as_ref()).map_err(storage_error)?;
        let root = fs::canonicalize(root.as_ref()).map_err(storage_error)?;
        let metadata = fs::symlink_metadata(&root).map_err(storage_error)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(FileStorageError(
                "private upload storage root is not a real directory".into(),
            ));
        }
        set_private_directory_permissions(&root).map_err(storage_error)?;
        Ok(Self { root })
    }

    /// Acquires a non-blocking shared lock held by a running application
    /// server. A cleanup operation in progress prevents the server from
    /// starting against the same managed storage root.
    pub fn acquire_server_lock(&self) -> Result<StorageActivityLock, FileStorageError> {
        acquire_activity_lock(&self.root, false)
    }

    /// Acquires a non-blocking exclusive lock before destructive maintenance.
    /// It fails while any Zelyra server holds a shared lock or another
    /// maintenance operation holds the exclusive lock.
    pub fn acquire_maintenance_lock(&self) -> Result<StorageActivityLock, FileStorageError> {
        acquire_activity_lock(&self.root, true)
    }

    /// Writes validated bytes to a private staging file and returns an opaque
    /// reference. The client filename and declared MIME header are not used to
    /// derive a path or decide the detected type.
    pub fn stage(
        &self,
        tenant_id: i64,
        bytes: &[u8],
        max_size_bytes: u64,
        allowed_types: &[String],
    ) -> Result<StagedUpload, FileStorageError> {
        if tenant_id < 0 {
            return Err(FileStorageError(
                "tenant id must be a non-negative integer".into(),
            ));
        }
        if max_size_bytes == 0 || max_size_bytes > MAX_UPLOAD_BYTES {
            return Err(FileStorageError(
                "upload size limit must be between 1 byte and 100 MiB".into(),
            ));
        }
        let size_bytes = u64::try_from(bytes.len())
            .map_err(|_| FileStorageError("upload size is out of range".into()))?;
        if size_bytes == 0 || size_bytes > max_size_bytes {
            return Err(FileStorageError(
                "upload is empty or exceeds its declared size limit".into(),
            ));
        }
        let media_type = detect_media_type(bytes).ok_or_else(|| {
            FileStorageError("uploaded content is not a supported PDF, PNG, or JPEG file".into())
        })?;
        if !allowed_types
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(media_type))
        {
            return Err(FileStorageError(format!(
                "detected media type `{media_type}` is not allowed for this field"
            )));
        }

        let tenant_directory = self.tenant_directory(tenant_id)?;
        let file_ref = random_uuid()?;
        let staging_path = tenant_directory.join(format!(".{file_ref}.stage"));
        let final_path = tenant_directory.join(&file_ref);
        let metadata_staging_path = tenant_directory.join(format!(".{file_ref}.meta.stage"));
        let metadata_final_path = tenant_directory.join(format!("{file_ref}.meta"));
        let mut file = create_private_file(&staging_path).map_err(storage_error)?;
        if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
            drop(file);
            let _ = fs::remove_file(&staging_path);
            return Err(storage_error(error));
        }
        let sha256 = sha256_hex(bytes);
        let metadata = StoredFile {
            file_ref,
            tenant_id,
            media_type,
            size_bytes,
            sha256,
        };
        let metadata_bytes = format!(
            "{}\n{}\n{}\n",
            metadata.media_type, metadata.size_bytes, metadata.sha256
        );
        let mut metadata_file = match create_private_file(&metadata_staging_path) {
            Ok(file) => file,
            Err(error) => {
                let _ = fs::remove_file(&staging_path);
                return Err(storage_error(error));
            }
        };
        if let Err(error) = metadata_file
            .write_all(metadata_bytes.as_bytes())
            .and_then(|()| metadata_file.sync_all())
        {
            drop(metadata_file);
            let _ = fs::remove_file(&staging_path);
            let _ = fs::remove_file(&metadata_staging_path);
            return Err(storage_error(error));
        }

        Ok(StagedUpload {
            metadata,
            staging_path: Some(staging_path),
            metadata_staging_path: Some(metadata_staging_path),
            final_path,
            metadata_final_path,
        })
    }

    /// Reads a tenant-owned upload by its opaque id and verifies bytes against
    /// the private manifest that was committed with the blob.
    pub fn read_verified(
        &self,
        tenant_id: i64,
        file_ref: &str,
        max_size_bytes: u64,
    ) -> Result<(StoredFile, Vec<u8>), FileStorageError> {
        self.file_path(tenant_id, file_ref)?;
        let metadata_path = self.metadata_path(tenant_id, file_ref)?;
        let metadata = fs::symlink_metadata(&metadata_path).map_err(storage_error)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 512 {
            return Err(FileStorageError(
                "stored file manifest is not a bounded regular file".into(),
            ));
        }
        let mut encoded = Vec::with_capacity(metadata.len() as usize);
        open_private_file(&metadata_path)
            .map_err(storage_error)?
            .take(513)
            .read_to_end(&mut encoded)
            .map_err(storage_error)?;
        if encoded.len() as u64 != metadata.len() || encoded.len() > 512 {
            return Err(FileStorageError("stored file manifest is invalid".into()));
        }
        let encoded = std::str::from_utf8(&encoded)
            .map_err(|_| FileStorageError("stored file manifest is invalid".into()))?;
        let mut fields = encoded.lines();
        let media_type = match fields.next().unwrap_or_default() {
            "application/pdf" => "application/pdf",
            "image/png" => "image/png",
            "image/jpeg" => "image/jpeg",
            _ => return Err(FileStorageError("stored file manifest is invalid".into())),
        };
        let size_bytes = fields
            .next()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|size| *size > 0 && *size <= max_size_bytes)
            .ok_or_else(|| FileStorageError("stored file manifest is invalid".into()))?;
        let sha256 = fields.next().unwrap_or_default();
        if fields.next().is_some() || !is_sha256_hex(sha256) {
            return Err(FileStorageError("stored file manifest is invalid".into()));
        }
        let bytes = self.read(tenant_id, file_ref, sha256, max_size_bytes)?;
        if bytes.len() as u64 != size_bytes || detect_media_type(&bytes) != Some(media_type) {
            return Err(FileStorageError(
                "stored file content does not match its manifest".into(),
            ));
        }
        Ok((
            StoredFile {
                file_ref: file_ref.to_owned(),
                tenant_id,
                media_type,
                size_bytes,
                sha256: sha256.to_owned(),
            },
            bytes,
        ))
    }

    /// Reads a blob only when its reference is a canonical generated UUID and
    /// the tenant directory and file are not symlinks.
    pub fn read(
        &self,
        tenant_id: i64,
        file_ref: &str,
        expected_sha256: &str,
        max_size_bytes: u64,
    ) -> Result<Vec<u8>, FileStorageError> {
        let path = self.file_path(tenant_id, file_ref)?;
        let metadata = fs::symlink_metadata(&path).map_err(storage_error)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(FileStorageError("stored file is not a regular file".into()));
        }
        if max_size_bytes == 0
            || metadata.len() == 0
            || metadata.len() > max_size_bytes
            || metadata.len() > MAX_UPLOAD_BYTES
        {
            return Err(FileStorageError(
                "stored file exceeds the allowed read limit".into(),
            ));
        }
        let file = open_private_file(&path).map_err(storage_error)?;
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        file.take(max_size_bytes.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(storage_error)?;
        if bytes.len() as u64 != metadata.len()
            || !constant_time_hex_eq(&sha256_hex(&bytes), expected_sha256)
        {
            return Err(FileStorageError(
                "stored file size or SHA-256 does not match its metadata".into(),
            ));
        }
        Ok(bytes)
    }

    /// Removes one generated tenant-owned reference. It never accepts a path,
    /// filename, or directory supplied by a request.
    pub fn remove(&self, tenant_id: i64, file_ref: &str) -> Result<(), FileStorageError> {
        let path = self.file_path(tenant_id, file_ref)?;
        let metadata_path = self.metadata_path(tenant_id, file_ref)?;
        let blob_exists = match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(FileStorageError(
                    "stored file reference does not identify a regular file".into(),
                ));
            }
            Ok(_) => true,
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(error) => return Err(storage_error(error)),
        };
        let manifest_exists = match fs::symlink_metadata(&metadata_path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(FileStorageError(
                    "stored file manifest is not a regular file".into(),
                ));
            }
            Ok(_) => true,
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(error) => return Err(storage_error(error)),
        };

        // Remove metadata first. If unlinking the blob then fails, the
        // remaining bytes have no valid manifest and cannot be served by the
        // verified read path. Most importantly, an invalid manifest is found
        // before either entry is changed.
        if manifest_exists {
            fs::remove_file(metadata_path).map_err(storage_error)?;
        }
        if blob_exists {
            fs::remove_file(path).map_err(storage_error)?;
        }
        Ok(())
    }

    /// Finds old, well-formed managed upload pairs not referenced by the
    /// caller's current database snapshot. The caller must bound the scan and
    /// must recheck references before applying removals.
    pub fn old_unreferenced_uploads(
        &self,
        referenced: &HashSet<(i64, String)>,
        older_than: SystemTime,
        max_entries: usize,
    ) -> Result<Vec<OldUnreferencedUpload>, FileStorageError> {
        if max_entries == 0 {
            return Err(FileStorageError(
                "upload cleanup scan limit must be positive".into(),
            ));
        }
        let tenants_path = self.root.join("tenants");
        let tenants_metadata = match fs::symlink_metadata(&tenants_path) {
            Ok(metadata)
                if metadata.file_type().is_symlink()
                    || !metadata.is_dir()
                    || !tenants_path_is_contained(&self.root, &tenants_path)? =>
            {
                return Err(FileStorageError(
                    "private upload tenant root is not a contained directory".into(),
                ));
            }
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(storage_error(error)),
        };
        let _ = tenants_metadata;

        let mut scanned = 0usize;
        let mut candidates = Vec::new();
        for tenant_entry in fs::read_dir(&tenants_path).map_err(storage_error)? {
            scanned = scanned.saturating_add(1);
            if scanned > max_entries {
                return Err(FileStorageError(
                    "upload cleanup scan exceeded its entry limit".into(),
                ));
            }
            let tenant_entry = tenant_entry.map_err(storage_error)?;
            let tenant_name = tenant_entry.file_name();
            let Some(tenant_name) = tenant_name.to_str() else {
                continue;
            };
            let Ok(tenant_id) = tenant_name.parse::<i64>() else {
                continue;
            };
            if tenant_id < 0 || tenant_id.to_string() != tenant_name {
                continue;
            }
            let tenant_path = tenant_entry.path();
            let tenant_metadata = fs::symlink_metadata(&tenant_path).map_err(storage_error)?;
            if tenant_metadata.file_type().is_symlink() || !tenant_metadata.is_dir() {
                return Err(FileStorageError(
                    "private upload tenant entry is not a real directory".into(),
                ));
            }
            if !tenants_path_is_contained(&self.root, &tenant_path)? {
                return Err(FileStorageError(
                    "private upload tenant directory escapes the storage root".into(),
                ));
            }
            let mut entries = HashMap::new();
            for entry in fs::read_dir(&tenant_path).map_err(storage_error)? {
                scanned = scanned.saturating_add(1);
                if scanned > max_entries {
                    return Err(FileStorageError(
                        "upload cleanup scan exceeded its entry limit".into(),
                    ));
                }
                let entry = entry.map_err(storage_error)?;
                let name = entry.file_name();
                let Some(name) = name.to_str() else {
                    continue;
                };
                let metadata = fs::symlink_metadata(entry.path()).map_err(storage_error)?;
                if metadata.file_type().is_symlink() {
                    continue;
                }
                entries.insert(name.to_owned(), metadata);
            }

            for (name, blob_metadata) in &entries {
                let (file_ref, staging_artifact, manifest_name) =
                    if let Some(file_ref) = name.strip_suffix(".stage") {
                        let Some(file_ref) = file_ref.strip_prefix('.') else {
                            continue;
                        };
                        (file_ref, true, format!(".{file_ref}.meta.stage"))
                    } else if is_canonical_uuid(name) {
                        (name.as_str(), false, format!("{name}.meta"))
                    } else {
                        continue;
                    };
                if !is_canonical_uuid(file_ref) || !blob_metadata.is_file() {
                    continue;
                }
                if !staging_artifact && referenced.contains(&(tenant_id, file_ref.to_owned())) {
                    continue;
                }
                let Some(manifest_metadata) = entries.get(&manifest_name) else {
                    continue;
                };
                if !manifest_metadata.is_file() {
                    continue;
                }
                let blob_modified = blob_metadata.modified().map_err(storage_error)?;
                let manifest_modified = manifest_metadata.modified().map_err(storage_error)?;
                let modified_at = blob_modified.max(manifest_modified);
                if modified_at >= older_than {
                    continue;
                }
                candidates.push(OldUnreferencedUpload {
                    tenant_id,
                    file_ref: file_ref.to_owned(),
                    size_bytes: blob_metadata.len(),
                    modified_at,
                    staging_artifact,
                });
            }
        }
        candidates.sort_by(|left, right| {
            (
                left.tenant_id,
                left.file_ref.as_str(),
                left.staging_artifact,
            )
                .cmp(&(
                    right.tenant_id,
                    right.file_ref.as_str(),
                    right.staging_artifact,
                ))
        });
        Ok(candidates)
    }

    /// Removes one old staging pair previously returned by
    /// `old_unreferenced_uploads`. It never accepts a path from the caller.
    pub fn remove_staging(&self, tenant_id: i64, file_ref: &str) -> Result<(), FileStorageError> {
        if tenant_id < 0 || !is_canonical_uuid(file_ref) {
            return Err(FileStorageError("invalid tenant or file reference".into()));
        }
        let tenant = self.tenant_directory(tenant_id)?;
        let blob = tenant.join(format!(".{file_ref}.stage"));
        let manifest = tenant.join(format!(".{file_ref}.meta.stage"));
        let blob_exists = regular_file_exists(&blob)?;
        let manifest_exists = regular_file_exists(&manifest)?;
        if manifest_exists {
            fs::remove_file(manifest).map_err(storage_error)?;
        }
        if blob_exists {
            fs::remove_file(blob).map_err(storage_error)?;
        }
        Ok(())
    }

    fn tenant_directory(&self, tenant_id: i64) -> Result<PathBuf, FileStorageError> {
        let tenants = self.root.join("tenants");
        ensure_private_directory(&self.root, &tenants)?;
        let tenant = tenants.join(tenant_id.to_string());
        ensure_private_directory(&self.root, &tenant)?;
        Ok(tenant)
    }

    fn file_path(&self, tenant_id: i64, file_ref: &str) -> Result<PathBuf, FileStorageError> {
        if tenant_id < 0 || !is_canonical_uuid(file_ref) {
            return Err(FileStorageError("invalid tenant or file reference".into()));
        }
        let tenant = self.tenant_directory(tenant_id)?;
        let path = tenant.join(file_ref);
        ensure_contained(&self.root, &path)?;
        Ok(path)
    }

    fn metadata_path(&self, tenant_id: i64, file_ref: &str) -> Result<PathBuf, FileStorageError> {
        let path = self.file_path(tenant_id, file_ref)?;
        Ok(path.with_file_name(format!("{file_ref}.meta")))
    }
}

impl StagedUpload {
    pub fn metadata(&self) -> &StoredFile {
        &self.metadata
    }

    /// Atomically publishes the staged blob without replacing an existing
    /// file. Database callers must remove this blob if their metadata
    /// transaction fails after publication.
    pub fn publish(mut self) -> Result<StoredFile, FileStorageError> {
        let staging_path = self
            .staging_path
            .as_ref()
            .ok_or_else(|| FileStorageError("upload staging file is no longer available".into()))?;
        let metadata_staging_path = self.metadata_staging_path.as_ref().ok_or_else(|| {
            FileStorageError("upload metadata staging file is no longer available".into())
        })?;
        fs::hard_link(staging_path, &self.final_path).map_err(storage_error)?;
        if let Err(error) = fs::hard_link(metadata_staging_path, &self.metadata_final_path) {
            let _ = fs::remove_file(&self.final_path);
            return Err(storage_error(error));
        }
        if let Err(error) = fs::remove_file(staging_path) {
            let _ = fs::remove_file(&self.final_path);
            let _ = fs::remove_file(&self.metadata_final_path);
            return Err(storage_error(error));
        }
        self.staging_path = None;
        if let Err(error) = fs::remove_file(metadata_staging_path) {
            let _ = fs::remove_file(&self.final_path);
            let _ = fs::remove_file(&self.metadata_final_path);
            return Err(storage_error(error));
        }
        self.metadata_staging_path = None;
        if let Err(error) = sync_directory(
            self.final_path
                .parent()
                .expect("generated upload path has a parent"),
        ) {
            let _ = fs::remove_file(&self.final_path);
            let _ = fs::remove_file(&self.metadata_final_path);
            return Err(storage_error(error));
        }
        Ok(self.metadata.clone())
    }
}

impl Drop for StagedUpload {
    fn drop(&mut self) {
        if let Some(path) = self.staging_path.take() {
            let _ = fs::remove_file(path);
        }
        if let Some(path) = self.metadata_staging_path.take() {
            let _ = fs::remove_file(path);
        }
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn detect_media_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"%PDF-") {
        Some("application/pdf")
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else {
        None
    }
}

fn random_uuid() -> Result<String, FileStorageError> {
    let mut bytes = [0_u8; 16];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|_| FileStorageError("operating-system random source failed".into()))?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    ))
}

fn is_canonical_uuid(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 36
        && [8, 13, 18, 23]
            .into_iter()
            .all(|index| bytes[index] == b'-')
        && bytes.iter().enumerate().all(|(index, byte)| {
            [8, 13, 18, 23].contains(&index)
                || byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()
        })
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
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

fn ensure_private_directory(root: &Path, path: &Path) -> Result<(), FileStorageError> {
    if path.exists() {
        let metadata = fs::symlink_metadata(path).map_err(storage_error)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(FileStorageError(
                "private upload path contains a symlink or non-directory".into(),
            ));
        }
    } else {
        fs::create_dir(path).map_err(storage_error)?;
    }
    ensure_contained(root, path)?;
    set_private_directory_permissions(path).map_err(storage_error)
}

fn ensure_contained(root: &Path, path: &Path) -> Result<(), FileStorageError> {
    let parent = path
        .parent()
        .ok_or_else(|| FileStorageError("private upload path has no parent".into()))?;
    let canonical_parent = fs::canonicalize(parent).map_err(storage_error)?;
    if !canonical_parent.starts_with(root) {
        return Err(FileStorageError(
            "private upload path escapes the configured storage root".into(),
        ));
    }
    Ok(())
}

fn tenants_path_is_contained(root: &Path, path: &Path) -> Result<bool, FileStorageError> {
    let canonical = fs::canonicalize(path).map_err(storage_error)?;
    Ok(canonical.starts_with(root))
}

fn regular_file_exists(path: &Path) -> Result<bool, FileStorageError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => Err(
            FileStorageError("private upload artifact is not a regular file".into()),
        ),
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(storage_error(error)),
    }
}

#[cfg(unix)]
fn set_private_directory_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn set_private_directory_permissions(_path: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "private upload storage permissions are not implemented on this platform",
    ))
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "durable private upload publication is not implemented on this platform",
    ))
}

#[cfg(unix)]
fn create_private_file(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
}

#[cfg(not(unix))]
fn create_private_file(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

#[cfg(unix)]
fn open_private_file(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
}

#[cfg(not(unix))]
fn open_private_file(path: &Path) -> io::Result<File> {
    OpenOptions::new().read(true).open(path)
}

fn storage_error(error: io::Error) -> FileStorageError {
    FileStorageError(format!("private upload storage operation failed: {error}"))
}

#[cfg(unix)]
fn acquire_activity_lock(
    root: &Path,
    exclusive: bool,
) -> Result<StorageActivityLock, FileStorageError> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};

    let path = root.join(".zelyra-upload-activity.lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&path)
        .map_err(storage_error)?;
    let metadata = file.metadata().map_err(storage_error)?;
    if !metadata.is_file() || metadata.nlink() != 1 {
        return Err(FileStorageError(
            "upload activity lock is not a private regular file".into(),
        ));
    }
    file.set_permissions(fs::Permissions::from_mode(0o600))
        .map_err(storage_error)?;
    let operation = if exclusive {
        libc::LOCK_EX | libc::LOCK_NB
    } else {
        libc::LOCK_SH | libc::LOCK_NB
    };
    // SAFETY: `file` owns a valid descriptor and remains alive for the lock's
    // lifetime. flock does not retain pointers to the caller's memory.
    let result = unsafe { libc::flock(std::os::fd::AsRawFd::as_raw_fd(&file), operation) };
    if result != 0 {
        let error = io::Error::last_os_error();
        let message = if error.kind() == io::ErrorKind::WouldBlock {
            if exclusive {
                "managed uploads are in use by a running server or another cleanup; stop Zelyra servers and retry"
            } else {
                "managed uploads are in a maintenance window; retry starting the server"
            }
        } else {
            return Err(storage_error(error));
        };
        return Err(FileStorageError(message.into()));
    }
    Ok(StorageActivityLock { _file: file })
}

#[cfg(not(unix))]
fn acquire_activity_lock(
    _root: &Path,
    _exclusive: bool,
) -> Result<StorageActivityLock, FileStorageError> {
    Err(FileStorageError(
        "upload activity coordination is not implemented on this platform".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_root() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "zelyra-private-upload-{}-{nonce}",
            std::process::id()
        ))
    }

    fn pdf_allowed() -> Vec<String> {
        vec!["application/pdf".into()]
    }

    #[cfg(unix)]
    #[test]
    fn server_and_maintenance_locks_are_mutually_exclusive() {
        let root = temporary_root();
        let storage = FileStorage::new(&root).expect("create private store");
        let server = storage.acquire_server_lock().expect("shared server lock");
        assert!(storage.acquire_maintenance_lock().is_err());
        drop(server);

        let maintenance = storage
            .acquire_maintenance_lock()
            .expect("exclusive maintenance lock");
        assert!(storage.acquire_server_lock().is_err());
        assert!(storage.acquire_maintenance_lock().is_err());
        drop(maintenance);
        storage.acquire_server_lock().expect("server can restart");
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn staged_upload_is_private_and_round_trips_with_hash_validation() {
        let root = temporary_root();
        let store = FileStorage::new(&root).expect("create private store");
        let bytes = b"%PDF-1.7\nprivate bytes";
        let staged = store
            .stage(42, bytes, 1024, &pdf_allowed())
            .expect("stage valid PDF");
        let metadata = staged.publish().expect("publish staged file");
        assert_eq!(metadata.media_type, "application/pdf");
        assert_eq!(metadata.size_bytes, bytes.len() as u64);
        let reopened = FileStorage::new(&root).expect("reopen private store");
        let (verified, verified_bytes) = reopened
            .read_verified(42, &metadata.file_ref, 1024)
            .expect("read persisted manifest and verify the blob");
        assert_eq!(verified, metadata);
        assert_eq!(verified_bytes, bytes);
        assert_eq!(
            store
                .read(42, &metadata.file_ref, &metadata.sha256, 1024)
                .unwrap(),
            bytes
        );
        assert!(store
            .read(42, &metadata.file_ref, &"0".repeat(64), 1024)
            .is_err());
        assert!(store
            .read(43, &metadata.file_ref, &metadata.sha256, 1024)
            .is_err());
        let manifest = root
            .join("tenants/42")
            .join(format!("{}.meta", metadata.file_ref));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&manifest).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        fs::write(&manifest, "application/pdf\n999\n0000000000000000000000000000000000000000000000000000000000000000\n")
            .unwrap();
        assert!(reopened
            .read_verified(42, &metadata.file_ref, 1024)
            .is_err());
        store.remove(42, &metadata.file_ref).unwrap();
        assert!(!manifest.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_spoofed_content_disallowed_types_and_size_overruns() {
        let root = temporary_root();
        let store = FileStorage::new(&root).unwrap();
        assert!(store.stage(1, b"not a PDF", 100, &pdf_allowed()).is_err());
        assert!(store
            .stage(1, b"%PDF-1.7", 100, &["image/png".into()])
            .is_err());
        assert!(store.stage(1, b"%PDF-1.7", 3, &pdf_allowed()).is_err());
        assert!(store.stage(-1, b"%PDF-1.7", 100, &pdf_allowed()).is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn dropping_unpublished_upload_removes_staged_bytes() {
        let root = temporary_root();
        let store = FileStorage::new(&root).unwrap();
        let staged = store.stage(1, b"%PDF-1.7", 100, &pdf_allowed()).unwrap();
        let directory = root.join("tenants/1");
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 2);
        drop(staged);
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 0);
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn failed_manifest_publication_does_not_leave_a_visible_blob() {
        use std::os::unix::fs::symlink;
        let root = temporary_root();
        let outside = temporary_root().with_extension("manifest-collision");
        fs::create_dir_all(&outside).unwrap();
        let store = FileStorage::new(&root).unwrap();
        let staged = store.stage(5, b"%PDF-1.7", 100, &pdf_allowed()).unwrap();
        let file_ref = staged.metadata().file_ref.clone();
        let directory = root.join("tenants/5");
        let manifest = directory.join(format!("{file_ref}.meta"));
        symlink(outside.join("target"), &manifest).unwrap();
        assert!(staged.publish().is_err());
        assert!(!directory.join(&file_ref).exists());
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
        fs::remove_file(manifest).unwrap();
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }

    #[cfg(unix)]
    #[test]
    fn refuses_to_partially_remove_blob_when_manifest_is_a_symlink() {
        use std::os::unix::fs::symlink;

        let root = temporary_root();
        let outside = temporary_root().with_extension("outside-manifest");
        fs::create_dir_all(&outside).unwrap();
        let store = FileStorage::new(&root).unwrap();
        let metadata = store
            .stage(8, b"%PDF-1.7\nprotected bytes", 1024, &pdf_allowed())
            .unwrap()
            .publish()
            .unwrap();
        let directory = root.join("tenants/8");
        let blob = directory.join(&metadata.file_ref);
        let manifest = directory.join(format!("{}.meta", metadata.file_ref));
        fs::remove_file(&manifest).unwrap();
        symlink(outside.join("target"), &manifest).unwrap();

        assert!(store.remove(8, &metadata.file_ref).is_err());
        assert!(blob.is_file(), "invalid manifest must not remove the blob");
        assert!(fs::symlink_metadata(&manifest)
            .unwrap()
            .file_type()
            .is_symlink());

        fs::remove_file(manifest).unwrap();
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }

    #[cfg(unix)]
    #[test]
    fn old_upload_scan_is_bounded_and_excludes_database_references() {
        let root = temporary_root();
        let store = FileStorage::new(&root).unwrap();
        let metadata = store
            .stage(9, b"%PDF-1.7\nold upload", 1024, &pdf_allowed())
            .unwrap()
            .publish()
            .unwrap();
        let referenced = HashSet::from([(9, metadata.file_ref.clone())]);
        let future_cutoff = SystemTime::now() + std::time::Duration::from_secs(60);

        assert!(store
            .old_unreferenced_uploads(&referenced, future_cutoff, 100)
            .unwrap()
            .is_empty());
        let candidates = store
            .old_unreferenced_uploads(&HashSet::new(), future_cutoff, 100)
            .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].tenant_id, 9);
        assert_eq!(candidates[0].file_ref, metadata.file_ref);
        assert!(!candidates[0].staging_artifact);
        assert!(store
            .old_unreferenced_uploads(&HashSet::new(), future_cutoff, 1)
            .is_err());

        store.remove(9, &metadata.file_ref).unwrap();
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn old_upload_scan_finds_abandoned_staging_pairs_and_removes_them_safely() {
        let root = temporary_root();
        let store = FileStorage::new(&root).unwrap();
        let mut staged = store
            .stage(12, b"%PDF-1.7\nabandoned", 1024, &pdf_allowed())
            .unwrap();
        let file_ref = staged.metadata.file_ref.clone();
        // Simulate a process crash: prevent the Rust Drop cleanup and leave
        // the generated stage pair behind for a later maintenance scan.
        staged.staging_path.take();
        staged.metadata_staging_path.take();
        let future_cutoff = SystemTime::now() + std::time::Duration::from_secs(60);
        let candidates = store
            .old_unreferenced_uploads(&HashSet::new(), future_cutoff, 100)
            .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].file_ref, file_ref);
        assert!(candidates[0].staging_artifact);

        store.remove_staging(12, &file_ref).unwrap();
        assert!(store
            .old_unreferenced_uploads(&HashSet::new(), future_cutoff, 100)
            .unwrap()
            .is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_tenant_directory() {
        use std::os::unix::fs::symlink;
        let root = temporary_root();
        let outside = temporary_root().with_extension("outside");
        fs::create_dir_all(&outside).unwrap();
        fs::create_dir_all(&root).unwrap();
        symlink(&outside, root.join("tenants")).unwrap();
        let store = FileStorage::new(&root).unwrap();
        assert!(store.stage(1, b"%PDF-1.7", 100, &pdf_allowed()).is_err());
        let _ = fs::remove_file(root.join("tenants"));
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }

    #[cfg(not(unix))]
    #[test]
    fn refuses_storage_when_private_platform_permissions_are_unavailable() {
        let root = temporary_root();
        assert!(FileStorage::new(&root).is_err());
        let _ = fs::remove_dir_all(root);
    }
}
