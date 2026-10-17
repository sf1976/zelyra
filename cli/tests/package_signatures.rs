use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture() -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("zelyra-package-signatures-{stamp}"));
    fs::create_dir_all(root.join("package/src")).unwrap();
    fs::write(
        root.join("package/zelyra.toml"),
        "[package]\nname = \"sample\"\nversion = \"1.2.3\"\nentry = \"src/lib.zyl\"\nzelyra = \">=1.1, <2.0\"\n",
    )
    .unwrap();
    fs::write(
        root.join("package/src/lib.zyl"),
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
