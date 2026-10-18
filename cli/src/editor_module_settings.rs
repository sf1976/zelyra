//! Authenticated Studio access to typed, application-owned package settings.
//! Secret values are never read or returned; only environment references may
//! be written for secret schema fields.
use super::*;
use fs2::FileExt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use toml_edit::{DocumentMut, Item, Table};

type EditorReply = Result<(u16, &'static str, String), (u16, String)>;
const SETTINGS_FILE: &str = "zelyra.settings.toml";
const SETTINGS_LOCK_FILE: &str = ".zelyra.settings.lock";
const MAX_SETTINGS_BYTES: usize = 1024 * 1024;
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

fn settings_path(state: &EditorState) -> Result<PathBuf, (u16, String)> {
    let root = fs::canonicalize(&state.root)
        .map_err(|_| (422, "project directory is unavailable".into()))?;
    let path = root.join(SETTINGS_FILE);
    if fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err((
            422,
            "module settings file must not be a symbolic link".into(),
        ));
    }
    Ok(path)
}

fn contents(path: &Path) -> Result<Vec<u8>, (u16, String)> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err((422, "module settings file could not be inspected".into())),
    };
    if metadata.file_type().is_symlink() {
        return Err((
            422,
            "module settings file must not be a symbolic link".into(),
        ));
    }
    if metadata.len() > MAX_SETTINGS_BYTES as u64 {
        return Err((413, "module settings file exceeds 1 MiB".into()));
    }
    let file =
        fs::File::open(path).map_err(|_| (422, "module settings file could not be read".into()))?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take((MAX_SETTINGS_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| (422, "module settings file could not be read".into()))?;
    if bytes.len() > MAX_SETTINGS_BYTES {
        return Err((413, "module settings file exceeds 1 MiB".into()));
    }
    Ok(bytes)
}

fn acquire_settings_file_lock(root: &Path) -> Result<fs::File, (u16, String)> {
    let path = root.join(SETTINGS_LOCK_FILE);
    if fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err((
            422,
            "module settings lock must not be a symbolic link".into(),
        ));
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // Open the reparse point itself so the metadata check below can reject
        // a link without following it.
        options.custom_flags(0x0020_0000);
    }
    let file = options
        .open(path)
        .map_err(|_| (500, "module settings lock could not be opened".into()))?;
    let metadata = file
        .metadata()
        .map_err(|_| (500, "module settings lock could not be inspected".into()))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err((422, "module settings lock must be a regular file".into()));
    }
    file.lock_exclusive()
        .map_err(|_| (500, "module settings lock could not be acquired".into()))?;
    Ok(file)
}

fn revision(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn graph(state: &EditorState) -> Result<Option<crate::packages::PackageGraph>, (u16, String)> {
    crate::packages::PackageGraph::load(&state.root).map_err(|error| (422, error.message))
}

pub(super) fn read(state: &EditorState) -> EditorReply {
    let path = settings_path(state)?;
    let bytes = contents(&path)?;
    let Some(graph) = graph(state)? else {
        return Ok((200, "application/json; charset=utf-8", json!({"settingsFile": SETTINGS_FILE, "revision": revision(&bytes), "packages": [], "secretValuesIncluded": false}).to_string()));
    };
    let configured: toml::Value = if bytes.is_empty() {
        toml::Value::Table(toml::map::Map::new())
    } else {
        toml::from_str(
            std::str::from_utf8(&bytes)
                .map_err(|_| (422, "module settings file must be UTF-8".into()))?,
        )
        .map_err(|_| (422, "module settings file contains invalid TOML".into()))?
    };
    let values = configured
        .get("package_settings")
        .and_then(toml::Value::as_table);
    let statuses = graph
        .settings_status(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
        .map_err(|error| (422, error.message))?;
    let status_by_path = statuses
        .into_iter()
        .map(|package| (package.path, package.values))
        .collect::<BTreeMap<_, _>>();
    let packages = graph.inventory().into_iter().filter(|package| !package.settings.is_empty()).map(|package| {
        let package_values = values
            .and_then(|packages| packages.get(&package.path))
            .and_then(toml::Value::as_table);
        let statuses = status_by_path.get(&package.path);
        let settings = package.settings.into_iter().map(|setting| {
            let current = package_values.and_then(|map| map.get(&setting.name));
            let status = statuses.and_then(|items| items.iter().find(|item| item.name == setting.name)).map(|item| item.status).unwrap_or("missing");
            json!({
                "name": setting.name,
                "type": setting.type_name,
                "required": setting.required,
                "secret": setting.secret,
                "description": setting.description,
                "default": setting.default,
                "value": if setting.secret { Value::Null } else { current.and_then(toml_to_json).or_else(|| setting.default.as_ref().and_then(toml_to_json)).unwrap_or(Value::Null) },
                "configured": current.is_some(),
                "status": status,
            })
        }).collect::<Vec<_>>();
        json!({"path": package.path, "name": package.name, "version": package.version.to_string(), "settings": settings})
    }).collect::<Vec<_>>();
    Ok((200, "application/json; charset=utf-8", json!({"settingsFile": SETTINGS_FILE, "revision": revision(&bytes), "packages": packages, "secretValuesIncluded": false}).to_string()))
}

fn toml_to_json(value: &toml::Value) -> Option<Value> {
    match value {
        toml::Value::String(value) => Some(json!(value)),
        toml::Value::Integer(value) => Some(json!(value)),
        toml::Value::Float(value) if value.is_finite() => Some(json!(value)),
        toml::Value::Boolean(value) => Some(json!(value)),
        _ => None,
    }
}

fn json_to_toml(value: &Value, type_name: &str) -> Result<toml::Value, (u16, String)> {
    match (type_name, value) {
        ("String", Value::String(value)) => Ok(toml::Value::String(value.clone())),
        ("Bool", Value::Bool(value)) => Ok(toml::Value::Boolean(*value)),
        ("Int", Value::Number(value)) if value.as_i64().is_some() => {
            Ok(toml::Value::Integer(value.as_i64().unwrap_or_default()))
        }
        ("Float", Value::Number(value)) if value.as_f64().is_some_and(f64::is_finite) => {
            Ok(toml::Value::Float(value.as_f64().unwrap_or_default()))
        }
        _ => Err((
            422,
            format!("value does not match declared setting type `{type_name}`"),
        )),
    }
}

pub(super) fn write(state: &EditorState, request: &Value) -> EditorReply {
    let _write_guard = state
        .module_settings_write_lock
        .lock()
        .map_err(|_| (500, "module settings write lock is unavailable".into()))?;
    if request.to_string().len() > MAX_SETTINGS_BYTES {
        return Err((413, "module settings update exceeds 1 MiB".into()));
    }
    let root = fs::canonicalize(&state.root)
        .map_err(|_| (422, "project directory is unavailable".into()))?;
    let _process_lock = acquire_settings_file_lock(&root)?;
    let path = settings_path(state)?;
    let before = contents(&path)?;
    if request.get("revision").and_then(Value::as_str) != Some(revision(&before).as_str()) {
        return Err((
            409,
            "module settings changed in another editor; reload before saving".into(),
        ));
    }
    let graph = graph(state)?.ok_or((422, "project does not have a local package graph".into()))?;
    let packages = graph
        .inventory()
        .into_iter()
        .map(|package| (package.path.clone(), package))
        .collect::<BTreeMap<_, _>>();
    let updates = request
        .get("updates")
        .and_then(Value::as_array)
        .ok_or((400, "updates must be an array".into()))?;
    if updates.len() > 4096 {
        return Err((413, "too many module setting updates".into()));
    }
    let mut seen = HashSet::new();
    let original_text = std::str::from_utf8(&before)
        .map_err(|_| (422, "module settings file must be UTF-8".into()))?;
    let mut document = if original_text.trim().is_empty() {
        DocumentMut::new()
    } else {
        original_text
            .parse::<DocumentMut>()
            .map_err(|_| (422, "module settings file contains invalid TOML".into()))?
    };
    if !document.as_table().is_empty()
        && document
            .as_table()
            .iter()
            .any(|(key, _)| key != "package_settings")
    {
        return Err((
            422,
            "module settings file contains an unsupported top-level key".into(),
        ));
    }
    if document
        .get("package_settings")
        .is_none_or(|item| !item.is_table())
    {
        document["package_settings"] = Item::Table(Table::new());
    }
    for update in updates {
        let package_path = update
            .get("packagePath")
            .and_then(Value::as_str)
            .ok_or((400, "packagePath is required".into()))?;
        let name = update
            .get("name")
            .and_then(Value::as_str)
            .ok_or((400, "setting name is required".into()))?;
        if !seen.insert((package_path.to_owned(), name.to_owned())) {
            return Err((400, "duplicate setting update".into()));
        }
        let package = packages
            .get(package_path)
            .ok_or((400, "unknown package path".into()))?;
        let schema = package
            .settings
            .iter()
            .find(|setting| setting.name == name)
            .ok_or((400, "unknown package setting".into()))?;
        let package_table = document["package_settings"]
            .as_table_mut()
            .ok_or((422, "invalid package_settings table".into()))?;
        if !package_table.contains_key(package_path) {
            package_table[package_path] = Item::Table(Table::new());
        }
        let settings = package_table
            .get_mut(package_path)
            .and_then(Item::as_table_mut)
            .ok_or((422, "package settings must be a table".into()))?;
        if update.get("clear").and_then(Value::as_bool) == Some(true) {
            settings.remove(name);
            continue;
        }
        if schema.secret {
            let Some(env_name) = update
                .get("env")
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
            else {
                continue;
            };
            if !valid_env_name(env_name) {
                return Err((
                    400,
                    "secret settings require a valid environment variable name".into(),
                ));
            }
            let mut reference = toml_edit::InlineTable::new();
            reference.insert("env", toml_edit::Value::from(env_name));
            settings[name] = Item::Value(toml_edit::Value::InlineTable(reference));
        } else {
            let json_value = update
                .get("value")
                .ok_or((400, "value is required".into()))?;
            let value = json_to_toml(json_value, &schema.type_name)?;
            settings[name] = Item::Value(match value {
                toml::Value::String(value) => toml_edit::Value::from(value),
                toml::Value::Boolean(value) => toml_edit::Value::from(value),
                toml::Value::Integer(value) => toml_edit::Value::from(value),
                toml::Value::Float(value) => toml_edit::Value::from(value),
                _ => return Err((400, "unsupported module setting value".into())),
            });
        }
    }
    let updated = document.to_string();
    if updated.len() > MAX_SETTINGS_BYTES {
        return Err((413, "module settings file exceeds 1 MiB".into()));
    }
    graph
        .validate_settings_contents(&updated, |name| {
            std::env::var(name).ok().filter(|value| !value.is_empty())
        })
        .map_err(|error| (422, error.message))?;
    let written = updated.as_bytes();
    atomic_replace(&path, written)
        .map_err(|_| (500, "module settings could not be saved".into()))?;
    read(state)
}

fn valid_env_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        && name.as_bytes()[0].is_ascii_uppercase()
}

fn atomic_replace(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("settings path has no parent"))?;
    let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    let temp = parent.join(format!(
        ".zelyra-settings-{}-{sequence}.tmp",
        std::process::id()
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temp)?;
    if let Err(error) = file.write_all(contents).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    drop(file);
    if let Err(error) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> EditorState {
        let root = std::env::temp_dir().join(format!(
            "zelyra-module-settings-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("packages/ledger/src")).unwrap();
        fs::write(root.join("main.zyl"), "fn main() {}\n").unwrap();
        fs::write(root.join("zelyra.toml"), "[package]\nname = \"settings_app\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[dependencies.ledger]\npath = \"packages/ledger\"\nversion = \"^1.0\"\n").unwrap();
        fs::write(root.join("packages/ledger/zelyra.toml"), "[package]\nname = \"ledger\"\nversion = \"1.0.0\"\nentry = \"src/lib.zyl\"\n\n[settings.timeout]\ntype = \"Int\"\nrequired = true\n\n[settings.rate]\ntype = \"Float\"\nrequired = true\n\n[settings.api_token]\ntype = \"String\"\nrequired = true\nsecret = true\n").unwrap();
        fs::write(
            root.join("packages/ledger/src/lib.zyl"),
            "pub fn timeout() -> Int { module_setting<Int>(\"timeout\") }\n",
        )
        .unwrap();
        EditorState {
            root: root.canonicalize().unwrap(),
            token: "session".into(),
            authority: "127.0.0.1:4177".into(),
            web_process: Arc::new(std::sync::Mutex::new(None)),
            events: Arc::new(EventHub::default()),
            active_event_streams: Arc::new(AtomicUsize::new(0)),
            active_runs: Arc::new(AtomicUsize::new(0)),
            run_cancel: Arc::new(AtomicBool::new(false)),
            run_stdin: Arc::new(std::sync::Mutex::new(None)),
            run_output: Arc::new(std::sync::Mutex::new(EditorRunOutput::default())),
            module_settings_write_lock: Arc::new(std::sync::Mutex::new(())),
        }
    }

    #[test]
    fn studio_settings_api_preserves_types_redacts_secret_references_and_checks_revisions() {
        let state = state();
        let (_, _, body) = read(&state).unwrap();
        assert!(!body.contains("LEDGER_TOKEN"));
        assert!(!body.contains("secret-value"));
        let initial: Value = serde_json::from_str(&body).unwrap();
        let revision = initial["revision"].as_str().unwrap();
        let (_, _, saved) = write(
            &state,
            &json!({"revision":revision,"updates":[
                {"packagePath":"packages/ledger","name":"timeout","value":30},
                {"packagePath":"packages/ledger","name":"rate","value":2},
                {"packagePath":"packages/ledger","name":"api_token","env":"LEDGER_TOKEN"}
            ]}),
        )
        .unwrap();
        let saved_json: Value = serde_json::from_str(&saved).unwrap();
        assert_eq!(saved_json["secretValuesIncluded"], false);
        assert!(!saved.contains("LEDGER_TOKEN"));
        assert!(!saved.contains("secret-value"));
        let source = fs::read_to_string(state.root.join(SETTINGS_FILE)).unwrap();
        assert!(source.contains("timeout = 30"));
        assert!(source.contains("rate = 2.0"));
        assert!(source.contains("api_token = { env = \"LEDGER_TOKEN\" }"));
        assert!(!source.contains("secret-value"));
        assert_eq!(
            write(&state, &json!({"revision":revision,"updates":[]}))
                .unwrap_err()
                .0,
            409
        );
        assert_eq!(
            fs::read_to_string(state.root.join(SETTINGS_FILE)).unwrap(),
            source
        );
        let _ = fs::remove_dir_all(state.root.clone());
    }

    #[test]
    fn settings_file_lock_serializes_independent_writers() {
        let state = state();
        let root = state.root.clone();
        let first = acquire_settings_file_lock(&root).unwrap();
        let second = OpenOptions::new()
            .read(true)
            .write(true)
            .open(root.join(SETTINGS_LOCK_FILE))
            .unwrap();
        assert!(second.try_lock_exclusive().is_err());
        drop(first);
        second.try_lock_exclusive().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn settings_file_lock_rejects_symlink() {
        let state = state();
        let root = state.root.clone();
        let target = root.join("lock-target");
        fs::write(&target, "").unwrap();
        std::os::unix::fs::symlink(&target, root.join(SETTINGS_LOCK_FILE)).unwrap();
        assert_eq!(acquire_settings_file_lock(&root).unwrap_err().0, 422);
    }

    #[test]
    fn studio_settings_api_rejects_wrong_types_and_symlinks_without_partial_writes() {
        let state = state();
        let (_, _, body) = read(&state).unwrap();
        let revision = serde_json::from_str::<Value>(&body).unwrap()["revision"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(write(&state, &json!({"revision":revision,"updates":[{"packagePath":"packages/ledger","name":"timeout","value":"30"}]})).unwrap_err().0, 422);
        assert!(!state.root.join(SETTINGS_FILE).exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let target = state.root.join("outside-settings.toml");
            fs::write(&target, "safe = true\n").unwrap();
            symlink(&target, state.root.join(SETTINGS_FILE)).unwrap();
            assert_eq!(read(&state).unwrap_err().0, 422);
            assert_eq!(fs::read_to_string(target).unwrap(), "safe = true\n");
        }
        let _ = fs::remove_dir_all(state.root.clone());
    }

    #[test]
    fn concurrent_settings_saves_with_the_same_revision_cannot_overwrite_each_other() {
        let state = state();
        let (_, _, body) = read(&state).unwrap();
        let revision = serde_json::from_str::<Value>(&body).unwrap()["revision"]
            .as_str()
            .unwrap()
            .to_owned();
        let barrier = Arc::new(std::sync::Barrier::new(3));
        let updates = [
            json!({"revision":revision,"updates":[
                {"packagePath":"packages/ledger","name":"timeout","value":30},
                {"packagePath":"packages/ledger","name":"rate","value":2}
            ]}),
            json!({"revision":revision,"updates":[
                {"packagePath":"packages/ledger","name":"timeout","value":45},
                {"packagePath":"packages/ledger","name":"rate","value":3}
            ]}),
        ];
        let handles = updates.map(|request| {
            let state = state.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                write(&state, &request)
                    .map(|_| ())
                    .map_err(|(status, _)| status)
            })
        });
        barrier.wait();
        let results = handles.map(|handle| handle.join().unwrap());
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results.iter().filter(|result| **result == Err(409)).count(),
            1
        );
        let saved: toml::Value =
            toml::from_str(&fs::read_to_string(state.root.join(SETTINGS_FILE)).unwrap()).unwrap();
        let settings = saved["package_settings"]["packages/ledger"]
            .as_table()
            .unwrap();
        let pair = (
            settings["timeout"].as_integer().unwrap(),
            settings["rate"].as_float().unwrap(),
        );
        assert!(pair == (30, 2.0) || pair == (45, 3.0));
        let _ = fs::remove_dir_all(state.root.clone());
    }
}
