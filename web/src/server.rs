use super::{handle_connection, WebApp};
use std::io;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

pub(super) const MAX_CONCURRENT_CONNECTIONS: usize = 64;

struct ConnectionSlot(Arc<AtomicUsize>);

impl Drop for ConnectionSlot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Release);
    }
}

pub fn serve_app(app: WebApp, address: &str) -> io::Result<()> {
    let listener = TcpListener::bind(address)?;
    serve_app_listener(app, listener)
}

/// Serve an application on a listener that the caller has already bound.
/// This lets commands select and report a fallback port without a bind race.
pub fn serve_app_listener(app: WebApp, listener: TcpListener) -> io::Result<()> {
    let uploads_enabled = app.forms.iter().any(|form| {
        form.form
            .fields
            .iter()
            .any(|field| field.file_upload.is_some())
    });
    let _upload_activity_lock =
        acquire_upload_activity_lock(app.upload_storage_root.as_deref(), uploads_enabled)?;
    let app = Arc::new(app);
    let active_connections = Arc::new(AtomicUsize::new(0));
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                match spawn_connection(stream, Arc::clone(&app), Arc::clone(&active_connections)) {
                    Ok(Some(_worker)) => {}
                    Ok(None) => {
                        // Drop excess connections immediately instead of growing an
                        // unbounded worker queue while all handler slots are busy.
                    }
                    Err(error) => eprintln!("zelyra web: request worker failed: {error}"),
                }
            }
            Err(error) => eprintln!("zelyra web: connection failed: {error}"),
        }
    }
    Ok(())
}

fn acquire_upload_activity_lock(
    storage_root: Option<&std::path::Path>,
    uploads_enabled: bool,
) -> io::Result<Option<super::file_storage::StorageActivityLock>> {
    if !uploads_enabled {
        return Ok(None);
    }
    let root = storage_root.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "upload-enabled server has no private storage root",
        )
    })?;
    let storage = super::file_storage::FileStorage::new(root)
        .map_err(|error| io::Error::other(error.to_string()))?;
    storage
        .acquire_server_lock()
        .map(Some)
        .map_err(|error| io::Error::other(error.to_string()))
}

pub(super) fn spawn_connection(
    stream: TcpStream,
    app: Arc<WebApp>,
    active_connections: Arc<AtomicUsize>,
) -> io::Result<Option<JoinHandle<()>>> {
    let reserved = active_connections.try_update(Ordering::AcqRel, Ordering::Acquire, |active| {
        (active < MAX_CONCURRENT_CONNECTIONS).then_some(active + 1)
    });
    if reserved.is_err() {
        return Ok(None);
    }

    let slot = ConnectionSlot(active_connections);
    let worker = thread::Builder::new()
        .name("zelyra-http-connection".into())
        .spawn(move || {
            let _slot = slot;
            let mut stream = stream;
            if let Err(error) = handle_connection(&mut stream, &app) {
                eprintln!("zelyra web: request failed: {error}");
            }
        })?;
    Ok(Some(worker))
}

#[cfg(test)]
mod tests {
    use super::acquire_upload_activity_lock;
    use crate::file_storage::FileStorage;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_root() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "zelyra-server-upload-lock-{}-{nonce}",
            std::process::id()
        ))
    }

    #[cfg(unix)]
    #[test]
    fn server_lifetime_lock_blocks_maintenance_only_for_upload_apps() {
        let root = temporary_root();
        let storage = FileStorage::new(&root).expect("create private store");
        let maintenance = storage
            .acquire_maintenance_lock()
            .expect("exclusive maintenance lock");
        assert!(acquire_upload_activity_lock(Some(&root), true).is_err());
        assert!(acquire_upload_activity_lock(Some(&root), false)
            .unwrap()
            .is_none());
        drop(maintenance);

        let server = acquire_upload_activity_lock(Some(&root), true)
            .unwrap()
            .expect("upload app holds shared lock");
        assert!(storage.acquire_maintenance_lock().is_err());
        drop(server);
        storage
            .acquire_maintenance_lock()
            .expect("maintenance proceeds after server exits");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn upload_app_requires_a_storage_root_before_server_start() {
        let error = acquire_upload_activity_lock(None, true).unwrap_err();
        assert!(error.to_string().contains("no private storage root"));
    }
}
