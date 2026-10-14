//! Bounded, authenticated Studio event delivery. Authentication and local
//! authority checks happen in `editor.rs` before a stream is registered.

use serde_json::Value;
use std::io::{self, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const QUEUE_CAPACITY: usize = 32;
const STREAM_LIFETIME: Duration = Duration::from_secs(300);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
const MAX_MESSAGE_BYTES: usize = 256 * 1024;

#[derive(Default)]
pub(super) struct EventHub {
    subscribers: Mutex<Vec<(u64, SyncSender<String>)>>,
    next_subscriber_id: AtomicU64,
    revision: AtomicU64,
}

impl EventHub {
    pub(super) fn subscribe(&self) -> (u64, Receiver<String>) {
        let (sender, receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
        let id = self.next_subscriber_id.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.push((id, sender));
        }
        (id, receiver)
    }

    pub(super) fn unsubscribe(&self, id: u64) {
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.retain(|(subscriber_id, _)| *subscriber_id != id);
        }
    }

    pub(super) fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }

    pub(super) fn changed(&self, path: &str, operation: &str) {
        let revision = self.revision.fetch_add(1, Ordering::AcqRel) + 1;
        self.publish(serde_json::json!({
            "schema":1,"kind":"workspace.changed","document":{"path":path,"version":null},
            "workspaceRevision":revision,"payload":{"operation":operation}
        }));
        self.publish(serde_json::json!({
            "schema":1,"kind":"analysis.invalidated","document":{"path":path,"version":null},
            "workspaceRevision":revision,"payload":{"reason":"workspace.changed"}
        }));
    }

    pub(super) fn publish(&self, event: Value) {
        let encoded = event.to_string();
        if encoded.len() > MAX_MESSAGE_BYTES {
            return;
        }
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.retain(|(_, sender)| match sender.try_send(encoded.clone()) {
                Ok(()) => true,
                Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => false,
            });
        }
    }
}

pub(super) fn stream(mut connection: TcpStream, receiver: Receiver<String>) -> io::Result<()> {
    connection.write_all(
        concat!(
            "HTTP/1.1 200 OK\r\n",
            "Content-Type: text/event-stream; charset=utf-8\r\n",
            "Cache-Control: no-store\r\n",
            "Connection: close\r\n",
            "X-Content-Type-Options: nosniff\r\n",
            "Referrer-Policy: no-referrer\r\n",
            "Cross-Origin-Resource-Policy: same-origin\r\n",
            "X-Frame-Options: DENY\r\n",
            "Content-Security-Policy: default-src 'none'; frame-ancestors 'none'\r\n\r\n",
            "retry: 1500\n\n",
        )
        .as_bytes(),
    )?;
    let started = Instant::now();
    while started.elapsed() < STREAM_LIFETIME {
        match receiver.recv_timeout(HEARTBEAT_INTERVAL) {
            Ok(event) => {
                connection.write_all(b"data: ")?;
                connection.write_all(event.as_bytes())?;
                connection.write_all(b"\n\n")?;
            }
            Err(RecvTimeoutError::Timeout) => connection.write_all(b": heartbeat\n\n")?,
            Err(RecvTimeoutError::Disconnected) => break,
        }
        connection.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slow_subscriber_is_dropped_at_bounded_queue_limit() {
        let hub = EventHub::default();
        let (_, slow) = hub.subscribe();
        for index in 0..QUEUE_CAPACITY {
            hub.publish(serde_json::json!({"index":index}));
        }
        hub.publish(serde_json::json!({"index":"overflow"}));
        for index in 0..QUEUE_CAPACITY {
            let event: Value = serde_json::from_str(&slow.try_recv().unwrap()).unwrap();
            assert_eq!(event["index"], index);
        }
        assert!(matches!(
            slow.try_recv(),
            Err(mpsc::TryRecvError::Disconnected)
        ));
    }

    #[test]
    fn workspace_revision_is_monotone_and_event_has_document_identity() {
        let hub = EventHub::default();
        let (_, receiver) = hub.subscribe();
        hub.changed("src/item.zyl", "save");
        let event: Value = serde_json::from_str(&receiver.try_recv().unwrap()).unwrap();
        assert_eq!(event["schema"], 1);
        assert_eq!(event["kind"], "workspace.changed");
        assert_eq!(event["document"]["path"], "src/item.zyl");
        assert_eq!(event["workspaceRevision"], 1);
        hub.changed("src/item.zyl", "rename");
        assert_eq!(hub.revision(), 2);
    }

    #[test]
    fn disconnected_stream_is_unsubscribed_without_waiting_for_a_publish() {
        let hub = EventHub::default();
        let (id, receiver) = hub.subscribe();
        assert_eq!(hub.subscribers.lock().unwrap().len(), 1);
        hub.unsubscribe(id);
        assert_eq!(hub.subscribers.lock().unwrap().len(), 0);
        assert!(matches!(
            receiver.try_recv(),
            Err(mpsc::TryRecvError::Disconnected)
        ));
    }
}
