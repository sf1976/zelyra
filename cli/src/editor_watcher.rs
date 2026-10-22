//! Bounded metadata watcher for changes made outside Studio.
//! The scan runs off the HTTP startup path and never opens project content.
use super::*;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

const INTERVAL: Duration = Duration::from_millis(1500);
const MAX_CHANGED_EVENTS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileStamp {
    len: u64,
    modified: Option<SystemTime>,
    #[cfg(unix)]
    changed_seconds: i64,
    #[cfg(unix)]
    changed_nanos: i64,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

fn stamp(metadata: &fs::Metadata) -> FileStamp {
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;
    FileStamp {
        len: metadata.len(),
        modified: metadata.modified().ok().or(Some(UNIX_EPOCH)),
        #[cfg(unix)]
        changed_seconds: metadata.ctime(),
        #[cfg(unix)]
        changed_nanos: metadata.ctime_nsec(),
        #[cfg(unix)]
        device: metadata.dev(),
        #[cfg(unix)]
        inode: metadata.ino(),
    }
}

fn watched(path: &str) -> bool {
    path.ends_with(".zyl") || path.ends_with(".css") || path == "zelyra.toml"
}

fn snapshot(state: &EditorState) -> Result<BTreeMap<String, FileStamp>, (u16, String)> {
    let mut files = Vec::new();
    collect_editor_files(&state.root, &state.root, 0, &mut files)?;
    let mut result = BTreeMap::new();
    for relative in files.into_iter().filter(|path| watched(path)) {
        let path = match safe_editor_path(&state.root, &relative, false) {
            Ok(path) => path,
            Err(_) => continue,
        };
        let Ok(metadata) = fs::symlink_metadata(path) else {
            continue;
        };
        if metadata.is_file() {
            result.insert(relative, stamp(&metadata));
        }
    }
    Ok(result)
}

fn notify_changes(
    events: &EventHub,
    before: &BTreeMap<String, FileStamp>,
    after: &BTreeMap<String, FileStamp>,
) -> usize {
    let changed = before
        .keys()
        .chain(after.keys())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .filter(|path| before.get(*path) != after.get(*path))
        .collect::<Vec<_>>();
    if changed.len() > MAX_CHANGED_EVENTS {
        events.changed("", "external-bulk");
    } else {
        for path in &changed {
            let operation = if !before.contains_key(*path) {
                "external-create"
            } else if !after.contains_key(*path) {
                "external-remove"
            } else {
                "external-update"
            };
            events.changed(path, operation);
        }
    }
    changed.len()
}

pub(super) fn start(state: Arc<EditorState>) {
    let state = Arc::downgrade(&state);
    if let Err(error) = thread::Builder::new()
        .name("zelyra-editor-watch".into())
        .spawn(move || {
            let mut previous = state.upgrade().and_then(|state| snapshot(&state).ok());
            loop {
                thread::sleep(INTERVAL);
                let Some(state) = state.upgrade() else {
                    break;
                };
                if let Ok(current) = snapshot(&state) {
                    if let Some(before) = &previous {
                        notify_changes(&state.events, before, &current);
                    }
                    previous = Some(current);
                }
            }
        })
    {
        eprintln!("zelyra editor: could not start workspace watcher: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> EditorState {
        let directory = std::env::temp_dir().join(format!(
            "zelyra-editor-watch-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("main.zyl"), "fn main() {}\n").unwrap();
        EditorState {
            root: directory.canonicalize().unwrap(),
            token: "test".into(),
            authority: "127.0.0.1:4177".into(),
            web_process: Arc::new(std::sync::Mutex::new(None)),
            events: Arc::new(EventHub::default()),
            active_event_streams: Arc::new(AtomicUsize::new(0)),
            active_runs: Arc::new(AtomicUsize::new(0)),
            run_cancel: Arc::new(AtomicBool::new(false)),
            run_stdin: Arc::new(std::sync::Mutex::new(None)),
            run_output: Arc::new(std::sync::Mutex::new(EditorRunOutput::default())),
            terminal: Arc::new(std::sync::Mutex::new(
                super::editor_terminal::Session::default(),
            )),
            module_settings_write_lock: Arc::new(std::sync::Mutex::new(())),
        }
    }

    #[test]
    fn external_changes_publish_paths_and_do_not_read_hidden_or_symlink_files() {
        let state = project();
        let before = snapshot(&state).unwrap();
        fs::create_dir(state.root.join("src")).unwrap();
        fs::write(state.root.join("src/new.zyl"), "fn helper() {}\n").unwrap();
        fs::write(state.root.join(".secret.zyl"), "hidden").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            state.root.join("src/new.zyl"),
            state.root.join("src/link.zyl"),
        )
        .unwrap();
        let after = snapshot(&state).unwrap();
        assert!(after.contains_key("src/new.zyl"));
        assert!(!after.contains_key(".secret.zyl"));
        assert!(!after.contains_key("src/link.zyl"));
        let (_, receiver) = state.events.subscribe();
        assert_eq!(notify_changes(&state.events, &before, &after), 1);
        let event: Value = serde_json::from_str(&receiver.try_recv().unwrap()).unwrap();
        assert_eq!(event["document"]["path"], "src/new.zyl");
        assert_eq!(event["payload"]["operation"], "external-create");
        assert_eq!(notify_changes(&state.events, &after, &after), 0);
        fs::remove_dir_all(state.root.clone()).unwrap();
    }

    #[test]
    fn bulk_external_change_has_one_invalidation_event() {
        let state = project();
        let before = BTreeMap::new();
        let mut after = BTreeMap::new();
        for index in 0..MAX_CHANGED_EVENTS + 1 {
            after.insert(
                format!("{index}.zyl"),
                FileStamp {
                    len: 1,
                    modified: None,
                    #[cfg(unix)]
                    changed_seconds: 0,
                    #[cfg(unix)]
                    changed_nanos: 0,
                    #[cfg(unix)]
                    device: 0,
                    #[cfg(unix)]
                    inode: 0,
                },
            );
        }
        let (_, receiver) = state.events.subscribe();
        assert_eq!(
            notify_changes(&state.events, &before, &after),
            MAX_CHANGED_EVENTS + 1
        );
        let event: Value = serde_json::from_str(&receiver.try_recv().unwrap()).unwrap();
        assert_eq!(event["document"]["path"], "");
        assert_eq!(event["payload"]["operation"], "external-bulk");
        fs::remove_dir_all(state.root.clone()).unwrap();
    }
}
