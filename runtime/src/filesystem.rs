use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

pub fn atomic_write_text(target: &Path, content: &str) -> io::Result<()> {
    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
    if target.is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "atomic text write refuses to replace a symbolic link",
        ));
    }
    let parent = target.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "atomic text write requires a parent directory",
        )
    })?;
    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "atomic text write requires a UTF-8 file name",
            )
        })?;
    for _ in 0..32 {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(
            ".{file_name}.zelyra-{}-{sequence}.tmp",
            std::process::id()
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
        {
            Ok(mut file) => {
                if let Err(error) = file
                    .write_all(content.as_bytes())
                    .and_then(|()| file.sync_all())
                {
                    let _ = std::fs::remove_file(&temp);
                    return Err(error);
                }
                drop(file);
                if let Err(error) = replace_file_atomically(&temp, target) {
                    let _ = std::fs::remove_file(&temp);
                    return Err(error);
                }
                sync_parent(parent);
                return Ok(());
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not reserve a temporary file",
    ))
}

pub fn write_text_new(target: &Path, content: &str) -> io::Result<()> {
    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let parent = target.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "exclusive text write requires a parent directory",
        )
    })?;
    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "exclusive text write requires a UTF-8 file name",
            )
        })?;
    for _ in 0..32 {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(
            ".{file_name}.zelyra-new-{}-{sequence}.tmp",
            std::process::id()
        ));
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        if let Err(error) = file
            .write_all(content.as_bytes())
            .and_then(|()| file.sync_all())
        {
            let _ = std::fs::remove_file(&temp);
            return Err(error);
        }
        drop(file);
        if let Err(error) = std::fs::hard_link(&temp, target) {
            let _ = std::fs::remove_file(&temp);
            return Err(error);
        }
        std::fs::remove_file(&temp)?;
        sync_parent(parent);
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "exclusive text write could not reserve a temporary file",
    ))
}

pub fn append_text(target: &Path, content: &str) -> io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(target)?;
    file.write_all(content.as_bytes())?;
    file.sync_data()
}

pub fn copy_file_new(source: &Path, destination: &Path) -> io::Result<()> {
    static COPY_SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let mut input = std::fs::File::open(source)?;
    let parent = destination.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "file copy requires a destination directory",
        )
    })?;
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "file copy requires a UTF-8 destination name",
            )
        })?;
    for _ in 0..32 {
        let sequence = COPY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(
            ".{file_name}.zelyra-copy-{}-{sequence}.tmp",
            std::process::id()
        ));
        let mut output = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        if let Err(error) = io::copy(&mut input, &mut output).and_then(|_| output.sync_all()) {
            let _ = std::fs::remove_file(&temp);
            return Err(error);
        }
        drop(output);
        if let Err(error) = std::fs::hard_link(&temp, destination) {
            let _ = std::fs::remove_file(&temp);
            return Err(error);
        }
        std::fs::remove_file(&temp)?;
        sync_parent(parent);
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "file copy could not reserve a temporary file",
    ))
}

pub fn rename_file_new(source: &Path, destination: &Path) -> io::Result<()> {
    std::fs::hard_link(source, destination)?;
    std::fs::remove_file(source)
}

fn sync_parent(parent: &Path) {
    if let Ok(dir) = std::fs::File::open(parent) {
        let _ = dir.sync_all();
    }
}

#[cfg(not(windows))]
fn replace_file_atomically(source: &Path, destination: &Path) -> io::Result<()> {
    std::fs::rename(source, destination)
}

#[cfg(windows)]
fn replace_file_atomically(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn ReplaceFileW(
            replaced_file_name: *const u16,
            replacement_file_name: *const u16,
            backup_file_name: *const u16,
            replace_flags: u32,
            exclude: *mut std::ffi::c_void,
            reserved: *mut std::ffi::c_void,
        ) -> i32;
        fn MoveFileExW(
            existing_file_name: *const u16,
            new_file_name: *const u16,
            flags: u32,
        ) -> i32;
    }

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    let source_wide = wide(source);
    let destination_wide = wide(destination);
    let result = if destination.exists() {
        // ReplaceFileW replaces an existing destination in one filesystem operation.
        unsafe {
            ReplaceFileW(
                destination_wide.as_ptr(),
                source_wide.as_ptr(),
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        }
    } else {
        // MOVEFILE_WRITE_THROUGH asks Windows to complete the move before returning.
        unsafe { MoveFileExW(source_wide.as_ptr(), destination_wide.as_ptr(), 0x0000_0008) }
    };
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
