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

pub(super) fn spawn_connection(
    stream: TcpStream,
    app: Arc<WebApp>,
    active_connections: Arc<AtomicUsize>,
) -> io::Result<Option<JoinHandle<()>>> {
    let reserved = active_connections.fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
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
