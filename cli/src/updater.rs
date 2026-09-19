use semver::Version;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{env, fs, io::Write, path::Path, process::ExitCode, time::Duration};

const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/sf1976/zelyra/releases/latest";
const DOWNLOAD_PREFIX: &str = "https://github.com/sf1976/zelyra/releases/download/";
const MAX_RELEASE_JSON_BYTES: u64 = 1024 * 1024;
const MAX_CHECKSUM_BYTES: u64 = 4096;
const MAX_BINARY_BYTES: u64 = 100 * 1024 * 1024;

struct LatestRelease {
    tag: String,
    version: Version,
    assets: Vec<Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InstallResult {
    Installed,
    #[cfg(windows)]
    StagedForWindows,
}

pub fn command(check_only: bool) -> ExitCode {
    match update(check_only) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error[E-UPDATE-001]: {error}");
            ExitCode::from(1)
        }
    }
}

fn update(check_only: bool) -> Result<(), String> {
    let current = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("the installed version is invalid: {error}"))?;
    let release = fetch_latest_release()?;

    if release.version <= current {
        if release.version == current {
            println!("Zelyra {current} is already up to date.");
        } else {
            println!(
                "Zelyra {current} is newer than the latest stable release ({}); no changes made.",
                release.version
            );
        }
        return Ok(());
    }

    if check_only {
        if let Some(target) = release_target() {
            release_asset_urls(&release, target)?;
            println!(
                "Update available: Zelyra {current} → {}. Run `zelyra update` to install it.",
                release.version
            );
        } else {
            println!(
                "Zelyra {} is available, but automatic updates are not published for {}-{}.",
                release.version,
                env::consts::OS,
                env::consts::ARCH
            );
        }
        return Ok(());
    }

    let executable = env::current_exe()
        .and_then(fs::canonicalize)
        .map_err(|error| format!("cannot locate the running Zelyra executable: {error}"))?;
    if is_workspace_binary(&executable) {
        return Err(format!(
            "this is a development build at {}; install Zelyra first, then run `zelyra update`",
            executable.display()
        ));
    }

    let target = release_target().ok_or_else(|| {
        format!(
            "automatic updates are currently available for Linux x86_64 and Windows x86_64; install the latest version for {}-{} using the instructions at https://github.com/sf1976/zelyra/releases",
            env::consts::OS,
            env::consts::ARCH
        )
    })?;
    let (binary_url, checksum_url) = release_asset_urls(&release, target)?;

    println!("Updating Zelyra {current} to {}…", release.version);
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .max_redirects(5)
        .build()
        .into();
    let binary = fetch_bytes(&agent, &binary_url, MAX_BINARY_BYTES)?;
    let checksum = fetch_bytes(&agent, &checksum_url, MAX_CHECKSUM_BYTES)?;
    verify_sha256(&binary, &checksum)?;
    match install_verified_binary(&executable, &binary)? {
        InstallResult::Installed => {
            println!("Updated successfully to Zelyra {}.", release.version);
            println!("Run `zelyra --version` to verify the installed version.");
        }
        #[cfg(windows)]
        InstallResult::StagedForWindows => {
            println!(
                "The verified update to Zelyra {} is staged and will replace Zelyra as this command exits.",
                release.version
            );
            println!("Wait briefly, then run `zelyra --version` to verify the installed version.");
        }
    }
    Ok(())
}

fn fetch_latest_release() -> Result<LatestRelease, String> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .max_redirects(5)
        .build()
        .into();
    let body = fetch_bytes(&agent, LATEST_RELEASE_URL, MAX_RELEASE_JSON_BYTES)?;
    parse_latest_release(&body)
}

fn fetch_bytes(agent: &ureq::Agent, url: &str, maximum: u64) -> Result<Vec<u8>, String> {
    let mut response = agent
        .get(url)
        .header("User-Agent", concat!("zelyra/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|error| format!("could not download the official update data: {error}"))?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(maximum.saturating_add(1))
        .read_to_vec()
        .map_err(|error| format!("could not read update data: {error}"))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > maximum {
        return Err(format!(
            "update response exceeded the {maximum}-byte safety limit"
        ));
    }
    Ok(bytes)
}

fn parse_latest_release(body: &[u8]) -> Result<LatestRelease, String> {
    let document: Value = serde_json::from_slice(body)
        .map_err(|error| format!("GitHub returned invalid release metadata: {error}"))?;
    if document.get("draft").and_then(Value::as_bool) != Some(false)
        || document.get("prerelease").and_then(Value::as_bool) != Some(false)
    {
        return Err("GitHub did not return a published stable release".to_owned());
    }
    let tag = document
        .get("tag_name")
        .and_then(Value::as_str)
        .ok_or_else(|| "GitHub release metadata has no tag name".to_owned())?
        .to_owned();
    validate_tag(&tag)?;
    let version_text = tag.strip_prefix('v').unwrap_or(&tag);
    let version = Version::parse(version_text)
        .map_err(|error| format!("latest release tag `{tag}` is not a valid version: {error}"))?;
    let assets = document
        .get("assets")
        .and_then(Value::as_array)
        .ok_or_else(|| "GitHub release metadata has no asset list".to_owned())?;

    Ok(LatestRelease {
        tag,
        version,
        assets: assets.to_vec(),
    })
}

fn release_asset_urls(release: &LatestRelease, target: &str) -> Result<(String, String), String> {
    let (binary_name, checksum_name) = binary_asset_names(&release.tag, target);
    let binary_url = find_asset_url(&release.assets, &binary_name)?;
    let checksum_url = find_asset_url(&release.assets, &checksum_name)?;
    ensure_release_asset(&release.tag, &binary_url, &binary_name)?;
    ensure_release_asset(&release.tag, &checksum_url, &checksum_name)?;
    Ok((binary_url, checksum_url))
}

fn find_asset_url(assets: &[Value], name: &str) -> Result<String, String> {
    let url = assets
        .iter()
        .find(|asset| asset.get("name").and_then(Value::as_str) == Some(name))
        .and_then(|asset| asset.get("browser_download_url"))
        .and_then(Value::as_str)
        .ok_or_else(|| format!("latest release does not contain the `{name}` update asset"))?;
    if !url.starts_with(DOWNLOAD_PREFIX) {
        return Err(format!(
            "release asset `{name}` has an unexpected download URL"
        ));
    }
    Ok(url.to_owned())
}

fn validate_tag(tag: &str) -> Result<(), String> {
    if tag.is_empty()
        || !tag
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err("latest release tag contains unsupported characters".to_owned());
    }
    Ok(())
}

fn release_target() -> Option<&'static str> {
    match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => Some("x86_64-unknown-linux-gnu"),
        ("windows", "x86_64") => Some("x86_64-pc-windows-msvc"),
        _ => None,
    }
}

fn binary_asset_names(tag: &str, target: &str) -> (String, String) {
    let extension = match target {
        "x86_64-pc-windows-msvc" => ".exe",
        _ => ".bin",
    };
    let binary = format!("zelyra-{tag}-{target}{extension}");
    let checksum = format!("{binary}.sha256");
    (binary, checksum)
}

fn ensure_release_asset(tag: &str, url: &str, name: &str) -> Result<(), String> {
    let expected_url = format!("{DOWNLOAD_PREFIX}{tag}/{name}");
    if url == expected_url {
        Ok(())
    } else {
        Err(format!("latest release has an invalid `{name}` asset URL"))
    }
}

fn verify_sha256(binary: &[u8], checksum_file: &[u8]) -> Result<(), String> {
    let checksum_text = std::str::from_utf8(checksum_file)
        .map_err(|_| "the update checksum is not valid UTF-8".to_owned())?;
    let expected = checksum_text
        .split_whitespace()
        .next()
        .ok_or_else(|| "the update checksum file is empty".to_owned())?;
    if expected.len() != 64 || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("the update checksum is not a SHA-256 digest".to_owned());
    }
    let actual = format!("{:x}", Sha256::digest(binary));
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(
            "SHA-256 verification failed; the installed executable was not changed".to_owned(),
        );
    }
    Ok(())
}

fn is_workspace_binary(executable: &Path) -> bool {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|workspace| executable.starts_with(workspace.join("target")))
        .unwrap_or(false)
}

#[cfg(unix)]
fn install_verified_binary(executable: &Path, bytes: &[u8]) -> Result<InstallResult, String> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let directory = executable
        .parent()
        .ok_or_else(|| "the executable has no parent directory".to_owned())?;
    let mut temporary_path = None;
    for attempt in 0..100_u32 {
        let candidate = directory.join(format!(
            ".zelyra-update-{}-{attempt}.tmp",
            std::process::id()
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o700)
            .open(&candidate)
        {
            Ok(mut file) => {
                if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
                    let _ = fs::remove_file(&candidate);
                    return Err(format!("could not stage the verified update: {error}"));
                }
                temporary_path = Some(candidate);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "cannot write beside {}; use a user-owned installation or your package manager ({error})",
                    executable.display()
                ));
            }
        }
    }
    let temporary_path =
        temporary_path.ok_or_else(|| "could not allocate a temporary update file".to_owned())?;
    let existing_mode = fs::metadata(executable)
        .map_err(|error| {
            let _ = fs::remove_file(&temporary_path);
            format!("cannot inspect the installed executable: {error}")
        })?
        .permissions()
        .mode()
        & 0o777;
    let executable_mode = existing_mode | 0o111;
    fs::set_permissions(&temporary_path, fs::Permissions::from_mode(executable_mode)).map_err(
        |error| {
            let _ = fs::remove_file(&temporary_path);
            format!("could not set executable permissions: {error}")
        },
    )?;
    if let Err(error) = fs::rename(&temporary_path, executable) {
        let _ = fs::remove_file(&temporary_path);
        return Err(format!(
            "could not replace the installed executable: {error}"
        ));
    }
    Ok(InstallResult::Installed)
}

#[cfg(windows)]
fn install_verified_binary(executable: &Path, bytes: &[u8]) -> Result<InstallResult, String> {
    use std::{
        os::windows::process::CommandExt,
        process::{Command, Stdio},
    };

    const REPLACEMENT_SCRIPT: &str = r#"
param([int]$ParentPid, [string]$Source, [string]$Destination, [string]$ScriptPath)
$deadline = [DateTime]::UtcNow.AddSeconds(30)
while ((Get-Process -Id $ParentPid -ErrorAction SilentlyContinue) -and [DateTime]::UtcNow -lt $deadline) {
  Start-Sleep -Milliseconds 100
}
if (Get-Process -Id $ParentPid -ErrorAction SilentlyContinue) { exit 2 }
for ($attempt = 0; $attempt -lt 40; $attempt++) {
  try {
    [System.IO.File]::Replace($Source, $Destination, $null)
    Remove-Item -LiteralPath $ScriptPath -Force -ErrorAction SilentlyContinue
    exit 0
  } catch {
    Start-Sleep -Milliseconds 250
  }
}
exit 1
"#;

    let directory = executable
        .parent()
        .ok_or_else(|| "the executable has no parent directory".to_owned())?;
    let nonce = std::process::id();
    let staged = directory.join(format!(".zelyra-update-{nonce}.exe"));
    let script = directory.join(format!(".zelyra-update-{nonce}.ps1"));
    write_new_file(&staged, bytes)?;
    if let Err(error) = write_new_file(&script, REPLACEMENT_SCRIPT.as_bytes()) {
        let _ = fs::remove_file(&staged);
        return Err(error);
    }
    let spawn_result = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script)
        .arg(std::process::id().to_string())
        .arg(&staged)
        .arg(executable)
        .arg(&script)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x08000000)
        .spawn();
    if let Err(error) = spawn_result {
        let _ = fs::remove_file(&staged);
        let _ = fs::remove_file(&script);
        return Err(format!(
            "could not start the Windows update helper: {error}"
        ));
    }
    Ok(InstallResult::StagedForWindows)
}

#[cfg(windows)]
fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            format!(
                "cannot stage the verified update beside {}; use a user-owned installation or your package manager ({error})",
                path.display()
            )
        })?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(path);
        return Err(format!("could not stage the verified update: {error}"));
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn install_verified_binary(_executable: &Path, _bytes: &[u8]) -> Result<InstallResult, String> {
    Err("automatic executable replacement is not supported on this platform".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::path::PathBuf;

    #[test]
    fn compares_release_versions_using_semver_precedence() {
        assert!(Version::parse("0.1.46").unwrap() > Version::parse("0.1.45").unwrap());
        assert!(Version::parse("0.2.0").unwrap() > Version::parse("0.1.99").unwrap());
        assert!(Version::parse("0.1.46").unwrap() > Version::parse("0.1.46-alpha.1").unwrap());
        assert!(Version::parse("0.1.45").unwrap() < Version::parse("0.1.46-alpha.2").unwrap());
    }

    #[test]
    fn creates_platform_asset_names_from_release_tag() {
        assert_eq!(
            binary_asset_names("v0.1.46", "x86_64-unknown-linux-gnu"),
            (
                "zelyra-v0.1.46-x86_64-unknown-linux-gnu.bin".to_owned(),
                "zelyra-v0.1.46-x86_64-unknown-linux-gnu.bin.sha256".to_owned()
            )
        );
        assert_eq!(
            binary_asset_names("v0.1.46", "x86_64-pc-windows-msvc"),
            (
                "zelyra-v0.1.46-x86_64-pc-windows-msvc.exe".to_owned(),
                "zelyra-v0.1.46-x86_64-pc-windows-msvc.exe.sha256".to_owned()
            )
        );
    }

    #[test]
    fn parses_only_published_stable_github_releases_and_expected_assets() {
        let target = release_target().unwrap_or("x86_64-unknown-linux-gnu");
        let (binary, checksum) = binary_asset_names("v0.1.46", target);
        let document = serde_json::json!({
            "tag_name": "v0.1.46",
            "draft": false,
            "prerelease": false,
            "assets": [
                {"name": binary, "browser_download_url": format!("{DOWNLOAD_PREFIX}v0.1.46/{binary}")},
                {"name": checksum, "browser_download_url": format!("{DOWNLOAD_PREFIX}v0.1.46/{checksum}")}
            ]
        });
        let release = parse_latest_release(document.to_string().as_bytes()).unwrap();
        assert_eq!(release.version, Version::parse("0.1.46").unwrap());
        assert_eq!(release.tag, "v0.1.46");
        release_asset_urls(&release, target).unwrap();

        let prerelease = serde_json::json!({"tag_name":"v0.1.47-alpha.1","draft":false,"prerelease":true,"assets":[]});
        assert!(parse_latest_release(prerelease.to_string().as_bytes()).is_err());
    }

    #[test]
    fn recognizes_older_releases_even_when_they_have_no_standalone_update_assets() {
        let document = serde_json::json!({
            "tag_name": "v0.1.40",
            "draft": false,
            "prerelease": false,
            "assets": []
        });
        let release = parse_latest_release(document.to_string().as_bytes()).unwrap();
        assert_eq!(release.version, Version::parse("0.1.40").unwrap());
        assert!(release_asset_urls(&release, "x86_64-unknown-linux-gnu").is_err());
    }

    #[test]
    fn rejects_untrusted_tags_and_non_github_asset_urls() {
        assert!(validate_tag("v0.1.46/../../other").is_err());
        assert!(validate_tag("v0.1.46-alpha.1").is_ok());
        assert!(ensure_release_asset("v0.1.46", "https://example.com/zelyra", "zelyra").is_err());
        assert!(ensure_release_asset(
            "v0.1.46",
            "https://github.com/sf1976/zelyra/releases/download/v0.1.45/zelyra",
            "zelyra"
        )
        .is_err());
    }

    #[test]
    fn validates_sha256_before_any_installation() {
        verify_sha256(
            b"abc",
            b"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  zelyra\n",
        )
        .unwrap();
        assert!(verify_sha256(
            b"tampered",
            b"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  zelyra\n"
        )
        .is_err());
        assert!(verify_sha256(b"abc", b"not-a-checksum\n").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn atomically_replaces_only_the_requested_binary() {
        use std::os::unix::fs::PermissionsExt;

        let directory = test_directory("atomic-replace");
        let executable = directory.join("zelyra");
        fs::write(&executable, b"old executable").unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(
            install_verified_binary(&executable, b"new executable").unwrap(),
            InstallResult::Installed
        );

        assert_eq!(fs::read(&executable).unwrap(), b"new executable");
        assert_eq!(
            fs::metadata(&executable).unwrap().permissions().mode() & 0o777,
            0o755
        );
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    fn test_directory(name: &str) -> PathBuf {
        let directory =
            env::temp_dir().join(format!("zelyra-update-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        directory
    }
}
