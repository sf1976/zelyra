use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::{Command, Output};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture() -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("zelyra-package-signatures-{stamp}"));
    fs::create_dir_all(root.join("package/src")).unwrap();
    fs::create_dir_all(root.join("package/vendor/text/src")).unwrap();
    fs::write(
        root.join("package/zelyra.toml"),
        "[package]\nname = \"sample\"\nversion = \"1.2.3\"\nentry = \"src/lib.zyl\"\nzelyra = \">=1.1, <2.0\"\n\n[dependencies.text]\npath = \"vendor/text\"\nversion = \"=1.0.0\"\n",
    )
    .unwrap();
    fs::write(
        root.join("package/src/lib.zyl"),
        "import \"pkg:text\" as text\npub fn greeting() -> String { return text::greeting() }\n",
    )
    .unwrap();
    fs::write(
        root.join("package/vendor/text/zelyra.toml"),
        "[package]\nname = \"text\"\nversion = \"1.0.0\"\nentry = \"src/lib.zyl\"\n",
    )
    .unwrap();
    fs::write(
        root.join("package/vendor/text/src/lib.zyl"),
        "pub fn greeting() -> String { return \"hello\" }\n",
    )
    .unwrap();
    root
}

fn zelyra(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn serve_registry(root: std::path::PathBuf) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut request = [0_u8; 4096];
            let count = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..count]);
            let path = request
                .lines()
                .next()
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap();
            let file = if path == "/index.json" {
                root.join("index.json")
            } else {
                let name = path.strip_prefix('/').unwrap();
                assert!(
                    name.ends_with(".zpkg"),
                    "unexpected registry request path: {path}"
                );
                root.join(name)
            };
            let body = fs::read(file).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
        }
    });
    (format!("http://{address}/index.json"), server)
}

fn serve_index(root: std::path::PathBuf) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut request = [0_u8; 4096];
        let count = stream.read(&mut request).unwrap();
        let request = String::from_utf8_lossy(&request[..count]);
        assert!(request
            .lines()
            .next()
            .unwrap()
            .starts_with("GET /index.json "));
        let body = fs::read(root.join("index.json")).unwrap();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .unwrap();
        stream.write_all(&body).unwrap();
    });
    (format!("http://{address}/index.json"), server)
}

#[test]
fn package_signature_requires_explicit_trust_and_detects_tampering() {
    let root = fixture();
    let private_key = root.join("publisher.key");
    let artifact = root.join("sample-1.2.3.zpkg");
    let generated = zelyra(
        &root,
        &[
            "package",
            "keygen",
            "--publisher",
            "example-publisher",
            "--output",
            private_key.to_str().unwrap(),
        ],
    );
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&private_key).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let output = stdout(&generated);
    let public_key = output
        .split("public key: ")
        .nth(1)
        .unwrap()
        .split_ascii_whitespace()
        .next()
        .unwrap();

    let signed = zelyra(
        &root,
        &[
            "package",
            "sign",
            "package",
            "--publisher",
            "example-publisher",
            "--key",
            private_key.to_str().unwrap(),
            "--output",
            artifact.to_str().unwrap(),
        ],
    );
    assert!(
        signed.status.success(),
        "{}",
        String::from_utf8_lossy(&signed.stderr)
    );
    let untrusted = zelyra(&root, &["package", "verify", artifact.to_str().unwrap()]);
    assert!(!untrusted.status.success());
    assert!(String::from_utf8_lossy(&untrusted.stderr).contains("not trusted"));

    let trusted = zelyra(
        &root,
        &["package", "trust", "add", "example-publisher", public_key],
    );
    assert!(
        trusted.status.success(),
        "{}",
        String::from_utf8_lossy(&trusted.stderr)
    );
    let verified = zelyra(&root, &["package", "verify", artifact.to_str().unwrap()]);
    assert!(
        verified.status.success(),
        "{}",
        String::from_utf8_lossy(&verified.stderr)
    );
    assert!(stdout(&verified).contains("verified trusted package sample 1.2.3"));

    let index_path = root.join("registry-index.json");
    let built_index = zelyra(
        &root,
        &[
            "package",
            "registry",
            "build",
            ".",
            "--output",
            index_path.to_str().unwrap(),
        ],
    );
    assert!(
        built_index.status.success(),
        "{}",
        String::from_utf8_lossy(&built_index.stderr)
    );
    let first_index = fs::read(&index_path).unwrap();
    let index: serde_json::Value = serde_json::from_slice(&first_index).unwrap();
    assert_eq!(index["format_version"], 1);
    assert_eq!(index["packages"][0]["name"], "sample");
    assert_eq!(index["packages"][0]["version"], "1.2.3");
    assert_eq!(index["packages"][0]["artifact"], "sample-1.2.3.zpkg");
    let rebuilt_index = zelyra(
        &root,
        &[
            "package",
            "registry",
            "build",
            ".",
            "--output",
            index_path.to_str().unwrap(),
        ],
    );
    assert!(rebuilt_index.status.success());
    assert_eq!(fs::read(&index_path).unwrap(), first_index);

    let mut archive: serde_json::Value =
        serde_json::from_slice(&fs::read(&artifact).unwrap()).unwrap();
    archive["files"][1]["source"] = serde_json::Value::String("pub fn changed() {}".into());
    fs::write(&artifact, serde_json::to_vec(&archive).unwrap()).unwrap();
    let tampered = zelyra(&root, &["package", "verify", artifact.to_str().unwrap()]);
    assert!(!tampered.status.success());
    assert!(String::from_utf8_lossy(&tampered.stderr).contains("checksum"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn keygen_refuses_to_overwrite_a_private_key() {
    let root = fixture();
    let private_key = root.join("publisher.key");
    fs::write(&private_key, b"preserve me").unwrap();
    let output = zelyra(
        &root,
        &[
            "package",
            "keygen",
            "--publisher",
            "example-publisher",
            "--output",
            private_key.to_str().unwrap(),
        ],
    );
    assert!(!output.status.success());
    assert_eq!(fs::read(&private_key).unwrap(), b"preserve me");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn package_add_downloads_exact_trusted_release_and_updates_offline_lock() {
    let root = fixture();
    let registry = root.join("registry");
    let app = root.join("app");
    fs::create_dir_all(&registry).unwrap();
    fs::create_dir_all(&app).unwrap();
    let key = root.join("publisher.key");
    let generated = zelyra(
        &root,
        &[
            "package",
            "keygen",
            "--publisher",
            "example-publisher",
            "--output",
            key.to_str().unwrap(),
        ],
    );
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let generated_output = stdout(&generated);
    let public_key = generated_output
        .split("public key: ")
        .nth(1)
        .unwrap()
        .split_ascii_whitespace()
        .next()
        .unwrap();
    let artifact = registry.join("sample-1.2.3.zpkg");
    let signed = zelyra(
        &root,
        &[
            "package",
            "sign",
            "package",
            "--publisher",
            "example-publisher",
            "--key",
            key.to_str().unwrap(),
            "--output",
            artifact.to_str().unwrap(),
        ],
    );
    assert!(
        signed.status.success(),
        "{}",
        String::from_utf8_lossy(&signed.stderr)
    );
    let package_manifest = root.join("package/zelyra.toml");
    let original_package_manifest = fs::read_to_string(&package_manifest).unwrap();
    fs::write(
        &package_manifest,
        original_package_manifest.replace("version = \"1.2.3\"", "version = \"1.2.4\""),
    )
    .unwrap();
    let artifact_124 = registry.join("sample-1.2.4.zpkg");
    let signed_124 = zelyra(
        &root,
        &[
            "package",
            "sign",
            "package",
            "--publisher",
            "example-publisher",
            "--key",
            key.to_str().unwrap(),
            "--output",
            artifact_124.to_str().unwrap(),
        ],
    );
    assert!(
        signed_124.status.success(),
        "{}",
        String::from_utf8_lossy(&signed_124.stderr)
    );
    fs::write(&package_manifest, original_package_manifest).unwrap();
    let indexed = zelyra(&root, &["package", "registry", "build", "registry"]);
    assert!(
        indexed.status.success(),
        "{}",
        String::from_utf8_lossy(&indexed.stderr)
    );
    let (search_url, search_server) = serve_index(registry.clone());
    let search = zelyra(&root, &["package", "search", &search_url, "sam"]);
    search_server.join().unwrap();
    assert!(search.status.success());
    let search_output = stdout(&search);
    assert!(search_output.contains("metadata is untrusted"));
    assert!(search_output.contains("sample 1.2.4"));
    assert!(!search_output.contains("another-package"));

    fs::write(
        app.join("zelyra.toml"),
        "[package]\nname = \"sample_app\"\nversion = \"0.1.0\"\nentry = \"main.zyl\"\n",
    )
    .unwrap();
    fs::write(
        app.join("main.zyl"),
        "import \"pkg:billing\" as billing\nfn main() {}\n",
    )
    .unwrap();
    let trusted = zelyra(
        &app,
        &["package", "trust", "add", "example-publisher", public_key],
    );
    assert!(
        trusted.status.success(),
        "{}",
        String::from_utf8_lossy(&trusted.stderr)
    );

    let (registry_url, server) = serve_registry(registry.clone());
    let added = zelyra(
        &app,
        &[
            "package",
            "add",
            &registry_url,
            "sample",
            "1.2.3",
            "--as",
            "billing",
        ],
    );
    server.join().unwrap();
    assert!(
        added.status.success(),
        "{}",
        String::from_utf8_lossy(&added.stderr)
    );
    assert!(app.join("zelyra.lock").is_file());
    let lock_text = fs::read_to_string(app.join("zelyra.lock")).unwrap();
    let lock: toml::Value = toml::from_str(&lock_text).expect("package lock must be valid TOML");
    let source = lock["package"][0]["sources"]["billing"].as_table().unwrap();
    assert_eq!(source["publisher"].as_str(), Some("example-publisher"));
    assert_eq!(source["registry"].as_str(), Some(registry_url.as_str()));
    assert_eq!(source["artifact_sha256"].as_str().unwrap().len(), 71);
    assert_eq!(lock["package"].as_array().unwrap().len(), 3);
    assert!(app
        .join(".zelyra/packages/sample/1.2.3/src/lib.zyl")
        .is_file());
    assert!(app
        .join(".zelyra/packages/sample/1.2.3/vendor/text/src/lib.zyl")
        .is_file());
    let checked = zelyra(&app, &["package", "lock", "--check"]);
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    let compiled = zelyra(&app, &["check", "main.zyl"]);
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let bundle = root.join("bundle-output");
    let bundle_text = bundle.to_string_lossy().into_owned();
    let bundled = zelyra(
        &app,
        &[
            "module",
            "bundle",
            "main.zyl",
            "pkg:billing",
            "--output",
            &bundle_text,
        ],
    );
    assert!(
        bundled.status.success(),
        "{}",
        String::from_utf8_lossy(&bundled.stderr)
    );
    let bundled_manifest: toml::Value =
        toml::from_str(&fs::read_to_string(bundle.join("zelyra.toml")).unwrap()).unwrap();
    assert!(bundled_manifest["package_sources"]["billing"].is_table());
    let bundled_check = zelyra(&bundle, &["check", "main.zyl"]);
    assert!(
        bundled_check.status.success(),
        "{}",
        String::from_utf8_lossy(&bundled_check.stderr)
    );
    let (upgrade_registry_url, upgrade_server) = serve_registry(registry.clone());
    let updated = zelyra(
        &app,
        &[
            "package",
            "update",
            &upgrade_registry_url,
            "sample",
            "1.2.4",
            "--as",
            "billing",
        ],
    );
    upgrade_server.join().unwrap();
    assert!(
        updated.status.success(),
        "{}",
        String::from_utf8_lossy(&updated.stderr)
    );
    assert!(app
        .join(".zelyra/packages/sample/1.2.4/src/lib.zyl")
        .is_file());
    assert!(app
        .join(".zelyra/packages/sample/1.2.3/src/lib.zyl")
        .is_file());
    let lock_after_update = fs::read_to_string(app.join("zelyra.lock")).unwrap();
    assert!(lock_after_update.contains("version = \"1.2.4\""));
    let checked_after_update = zelyra(&app, &["package", "lock", "--check"]);
    assert!(checked_after_update.status.success());
    let cache_preview = zelyra(&app, &["package", "cache", "clean"]);
    assert!(cache_preview.status.success());
    assert!(String::from_utf8_lossy(&cache_preview.stdout).contains("1.2.3"));
    assert!(app
        .join(".zelyra/packages/sample/1.2.3/src/lib.zyl")
        .is_file());
    let manifest_before_stale_check = fs::read(app.join("zelyra.toml")).unwrap();
    let mut stale_manifest = manifest_before_stale_check.clone();
    stale_manifest.extend_from_slice(b"\n# force a stale lock\n");
    fs::write(app.join("zelyra.toml"), stale_manifest).unwrap();
    let stale_cache_check = zelyra(&app, &["package", "cache", "clean"]);
    assert!(!stale_cache_check.status.success());
    assert!(String::from_utf8_lossy(&stale_cache_check.stderr).contains("lock"));
    fs::write(app.join("zelyra.toml"), manifest_before_stale_check).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let symlink_path = app.join(".zelyra/packages/sample/1.2.3/unexpected-link");
        symlink(app.join("main.zyl"), &symlink_path).unwrap();
        let symlink_cache_clean = zelyra(&app, &["package", "cache", "clean", "--apply"]);
        assert!(!symlink_cache_clean.status.success());
        assert!(String::from_utf8_lossy(&symlink_cache_clean.stderr).contains("symlink"));
        fs::remove_file(symlink_path).unwrap();
    }
    let cache_cleaned = zelyra(&app, &["package", "cache", "clean", "--apply"]);
    assert!(
        cache_cleaned.status.success(),
        "{}",
        String::from_utf8_lossy(&cache_cleaned.stderr)
    );
    assert!(!app.join(".zelyra/packages/sample/1.2.3").exists());
    assert!(app
        .join(".zelyra/packages/sample/1.2.4/src/lib.zyl")
        .is_file());
    fs::write(
        app.join("main.zyl"),
        "import \"pkg:billing\" as billing\nfn main() {}\n",
    )
    .unwrap();
    let manifest_before_failed_remove = fs::read(app.join("zelyra.toml")).unwrap();
    let lock_before_failed_remove = fs::read(app.join("zelyra.lock")).unwrap();
    let rejected_remove = zelyra(&app, &["package", "remove", "billing"]);
    assert!(!rejected_remove.status.success());
    assert_eq!(
        fs::read(app.join("zelyra.toml")).unwrap(),
        manifest_before_failed_remove
    );
    assert_eq!(
        fs::read(app.join("zelyra.lock")).unwrap(),
        lock_before_failed_remove
    );
    fs::write(app.join("main.zyl"), "fn main() {}\n").unwrap();
    let removed = zelyra(&app, &["package", "remove", "billing"]);
    assert!(
        removed.status.success(),
        "{}",
        String::from_utf8_lossy(&removed.stderr)
    );
    let manifest: toml::Value =
        toml::from_str(&fs::read_to_string(app.join("zelyra.toml")).unwrap()).unwrap();
    assert!(manifest.get("dependencies").is_none());
    assert!(manifest.get("package_sources").is_none());
    assert!(app
        .join(".zelyra/packages/sample/1.2.4/vendor/text/src/lib.zyl")
        .is_file());
    let checked_after_remove = zelyra(&app, &["package", "lock", "--check"]);
    assert!(checked_after_remove.status.success());
    let compiled_after_remove = zelyra(&app, &["check", "main.zyl"]);
    assert!(
        compiled_after_remove.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled_after_remove.stderr)
    );
    let cache_after_remove = zelyra(&app, &["package", "cache", "clean", "--apply"]);
    assert!(cache_after_remove.status.success());
    assert!(!app.join(".zelyra/packages/sample/1.2.4").exists());

    fs::remove_dir_all(root).unwrap();
}
