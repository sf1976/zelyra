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
use std::path::{Path, PathBuf};

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

#[derive(Debug)]
pub struct StagedUpload {
    metadata: StoredFile,
    staging_path: Option<PathBuf>,
    final_path: PathBuf,
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
        let mut file = create_private_file(&staging_path).map_err(storage_error)?;
        if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
            drop(file);
            let _ = fs::remove_file(&staging_path);
            return Err(storage_error(error));
        }
        let sha256 = sha256_hex(bytes);

        Ok(StagedUpload {
            metadata: StoredFile {
                file_ref,
                tenant_id,
                media_type,
                size_bytes,
                sha256,
            },
            staging_path: Some(staging_path),
            final_path,
        })
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
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => Err(
                FileStorageError("stored file reference does not identify a regular file".into()),
            ),
            Ok(_) => fs::remove_file(path).map_err(storage_error),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(storage_error(error)),
        }
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
        fs::hard_link(staging_path, &self.final_path).map_err(storage_error)?;
        if let Err(error) = fs::remove_file(staging_path) {
            let _ = fs::remove_file(&self.final_path);
            return Err(storage_error(error));
        }
        self.staging_path = None;
        if let Err(error) = sync_directory(
            self.final_path
                .parent()
                .expect("generated upload path has a parent"),
        ) {
            let _ = fs::remove_file(&self.final_path);
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
    }
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
        store.remove(42, &metadata.file_ref).unwrap();
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
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
        drop(staged);
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 0);
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
