use std::process::ExitCode;

#[cfg(target_os = "linux")]
use sha2::{Digest, Sha256};
#[cfg(target_os = "linux")]
use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Output},
    thread,
    time::{Duration, Instant},
};

#[cfg(target_os = "linux")]
const SERVICE_PREFIX: &str = "zelyra-";

pub(super) fn service_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(action) = args.next() else {
        usage();
        return ExitCode::from(2);
    };
    let port = match action.as_str() {
        "install" | "change" => match parse_port_arg(&mut args) {
            Ok(port) => Some(port),
            Err(message) => {
                eprintln!("error[E-SERVICE-001]: {message}");
                return ExitCode::from(2);
            }
        },
        "status" | "restart" | "remove" => {
            if args.next().is_some() {
                usage();
                return ExitCode::from(2);
            }
            None
        }
        _ => {
            usage();
            return ExitCode::from(2);
        }
    };

    #[cfg(not(target_os = "linux"))]
    {
        let _ = port;
        eprintln!("error[E-SERVICE-002]: OS services are currently supported on Linux with systemd --user");
        return ExitCode::from(1);
    }

    #[cfg(target_os = "linux")]
    {
        match run_linux_service(&action, port) {
            Ok(message) => {
                println!("{message}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error[E-SERVICE-003]: {error}");
                ExitCode::from(1)
            }
        }
    }
}

pub(super) fn current_project_service_id(workspace: &std::path::Path) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let workspace = workspace.canonicalize().ok()?;
        Some(project_service_id(&workspace))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = workspace;
        None
    }
}

pub(super) fn current_project_service_is_active(workspace: &std::path::Path) -> bool {
    current_project_service_state(workspace).1
}

pub(super) fn current_project_service_state(workspace: &std::path::Path) -> (bool, bool) {
    #[cfg(target_os = "linux")]
    {
        let Ok(workspace) = workspace.canonicalize() else {
            return (false, false);
        };
        let Ok(unit_path) = service_file_path(&workspace) else {
            return (false, false);
        };
        let Ok(metadata) = std::fs::symlink_metadata(&unit_path) else {
            return (false, false);
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return (false, false);
        }
        let Some(name) = unit_path.file_name().and_then(|name| name.to_str()) else {
            return (false, false);
        };
        let active = Command::new("systemctl")
            .args(["--user", "is-active", "--quiet", name])
            .status()
            .is_ok_and(|status| status.success());
        (true, active)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = workspace;
        (false, false)
    }
}

fn parse_port_arg(args: &mut impl Iterator<Item = String>) -> Result<u16, String> {
    if args.next().as_deref() != Some("--port") {
        return Err("expected --port <port>".into());
    }
    let value = args
        .next()
        .ok_or_else(|| "missing port after --port".to_owned())?;
    if args.next().is_some() {
        return Err("unexpected argument after port".into());
    }
    let port = value
        .parse::<u16>()
        .map_err(|_| "port must be an integer from 1 to 65535".to_owned())?;
    if port == 0 {
        return Err("port must be an integer from 1 to 65535".into());
    }
    Ok(port)
}

#[cfg(target_os = "linux")]
fn run_linux_service(action: &str, port: Option<u16>) -> Result<String, String> {
    let workspace =
        env::current_dir().map_err(|error| format!("cannot read working directory: {error}"))?;
    let workspace = workspace
        .canonicalize()
        .map_err(|error| format!("cannot resolve project directory: {error}"))?;
    let entry = workspace.join("main.zyl");
    if !entry.is_file() {
        return Err(format!(
            "`{}` does not contain main.zyl",
            workspace.display()
        ));
    }
    let service_path = service_file_path(&workspace)?;
    let service_name = service_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "invalid generated service name".to_owned())?;

    match action {
        "install" => {
            let port = port.expect("install requires a parsed port");
            check_project(&entry)?;
            if service_path.exists() {
                return Err(format!("service {service_name} is already installed; use `zelyra service change --port <port>` to change its port"));
            }
            ensure_port_available(port)?;
            let unit = render_unit(&workspace, port)?;
            write_new_unit(&service_path, unit.as_bytes())?;
            if let Err(error) = systemctl(&["daemon-reload"]) {
                let _ = fs::remove_file(&service_path);
                let _ = systemctl(&["daemon-reload"]);
                return Err(format!(
                    "could not reload the new service unit; its file was rolled back: {error}"
                ));
            }
            let enable = systemctl(&["enable", "--now", service_name]);
            if let Err(error) = enable {
                let _ = systemctl(&["disable", "--now", service_name]);
                let _ = fs::remove_file(&service_path);
                let _ = systemctl(&["daemon-reload"]);
                return Err(format!(
                    "could not enable the service; its unit file was rolled back: {error}"
                ));
            }
            if let Err(error) = wait_until_ready(service_name, port) {
                let _ = systemctl(&["disable", "--now", service_name]);
                let _ = fs::remove_file(&service_path);
                let _ = systemctl(&["daemon-reload"]);
                return Err(format!(
                    "service did not become ready; its unit file was rolled back: {error}"
                ));
            }
            Ok(format!("installed and started {service_name} on 127.0.0.1:{port}; check status with `zelyra service status`. To start user services before login after reboot, enable systemd lingering for this account with `loginctl enable-linger <user>`."))
        }
        "change" => {
            let port = port.expect("change requires a parsed port");
            check_project(&entry)?;
            let original = fs::read(&service_path).map_err(|error| {
                format!("service is not installed or its unit cannot be read: {error}")
            })?;
            let old_port = current_port(&original)?;
            if old_port == port {
                return Ok(format!("{service_name} already uses port {port}"));
            }
            let expected = render_unit(&workspace, old_port)?;
            if original != expected.as_bytes() {
                return Err(format!("{service_name} differs from the generated unit; refusing to overwrite manual changes. Remove and reinstall the service to replace it."));
            }
            let replacement = render_unit(&workspace, port)?;
            change_service_unit(
                ServicePortChange {
                    service_path: &service_path,
                    service_name,
                    old_port,
                    new_port: port,
                    original: &original,
                    replacement: replacement.as_bytes(),
                },
                systemctl,
                ensure_port_available,
                wait_until_ready,
            )
        }
        "status" => {
            systemctl(&["status", "--no-pager", service_name])?;
            Ok(String::new())
        }
        "restart" => {
            systemctl(&["restart", service_name])?;
            Ok(format!("restarted {service_name}"))
        }
        "remove" => {
            systemctl(&["disable", "--now", service_name])?;
            fs::remove_file(&service_path).map_err(|error| {
                format!("stopped the service but could not remove its unit file: {error}")
            })?;
            systemctl(&["daemon-reload"])?;
            Ok(format!(
                "stopped and removed {service_name}; project files and data were left unchanged"
            ))
        }
        _ => unreachable!("action is validated before platform dispatch"),
    }
}

#[cfg(target_os = "linux")]
fn service_file_path(workspace: &Path) -> Result<PathBuf, String> {
    let config_home = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .ok_or_else(|| "neither XDG_CONFIG_HOME nor HOME is set".to_owned())?;
    if !config_home.is_absolute() {
        return Err("XDG_CONFIG_HOME must be an absolute path".into());
    }
    let id = project_service_id(workspace);
    Ok(config_home
        .join("systemd/user")
        .join(format!("{SERVICE_PREFIX}{}.service", &id[..16])))
}

#[cfg(target_os = "linux")]
fn project_service_id(workspace: &Path) -> String {
    let mut digest = Sha256::new();
    digest.update(workspace.as_os_str().as_encoded_bytes());
    format!("{:x}", digest.finalize())[..16].to_owned()
}

#[cfg(target_os = "linux")]
fn render_unit(workspace: &Path, port: u16) -> Result<String, String> {
    let executable = env::current_exe()
        .map_err(|error| format!("cannot determine the Zelyra executable: {error}"))?;
    let entry = workspace.join("main.zyl");
    let env_file = workspace.join(".env");
    render_unit_for(
        workspace,
        &entry,
        &executable,
        &env_file,
        env_file.is_file(),
        port,
    )
}

#[cfg(target_os = "linux")]
fn render_unit_for(
    workspace: &Path,
    entry: &Path,
    executable: &Path,
    env_file: &Path,
    include_env_file: bool,
    port: u16,
) -> Result<String, String> {
    for path in [workspace, entry, executable, env_file] {
        if path.to_string_lossy().chars().any(char::is_control) {
            return Err("project and executable paths must not contain control characters".into());
        }
    }
    let mut unit = format!(
        "[Unit]\nDescription=Zelyra application service\nAfter=network.target\n\n[Service]\nType=simple\nWorkingDirectory={}\nExecStart={} serve {} 127.0.0.1:{}\nRestart=on-failure\nRestartSec=2\n# Zelyra-Port: {}\n",
        systemd_escape_path(&workspace.to_string_lossy()),
        systemd_quote(&executable.to_string_lossy()),
        systemd_quote(&entry.to_string_lossy()),
        port,
        port,
    );
    if include_env_file {
        unit.push_str(&format!(
            "EnvironmentFile=-{}\n",
            systemd_escape_path(&env_file.to_string_lossy())
        ));
    }
    unit.push_str(&format!(
        "Environment=ZELYRA_SYSTEMD_SERVICE_ID={}\n",
        project_service_id(workspace)
    ));
    unit.push_str("\n[Install]\nWantedBy=default.target\n");
    Ok(unit)
}

#[cfg(target_os = "linux")]
fn systemd_quote(value: &str) -> String {
    let escaped = value
        .replace('%', "%%")
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    format!("\"{escaped}\"")
}

#[cfg(target_os = "linux")]
fn systemd_escape_path(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            ' ' => escaped.push_str("\\x20"),
            '\\' => escaped.push_str("\\x5c"),
            '"' => escaped.push_str("\\x22"),
            '%' => escaped.push_str("%%"),
            _ => escaped.push(character),
        }
    }
    escaped
}

#[cfg(target_os = "linux")]
fn current_port(unit: &[u8]) -> Result<u16, String> {
    let unit =
        std::str::from_utf8(unit).map_err(|_| "service unit is not valid UTF-8".to_owned())?;
    let value = unit
        .lines()
        .find_map(|line| line.strip_prefix("# Zelyra-Port: "))
        .ok_or_else(|| {
            "service unit has no Zelyra port metadata; refusing to modify it".to_owned()
        })?;
    value
        .parse::<u16>()
        .ok()
        .filter(|port| *port > 0)
        .ok_or_else(|| "service unit has invalid Zelyra port metadata".to_owned())
}

#[cfg(target_os = "linux")]
fn ensure_port_available(port: u16) -> Result<(), String> {
    TcpListener::bind(("127.0.0.1", port))
        .map(drop)
        .map_err(|_| format!("127.0.0.1:{port} is already in use; choose another port explicitly"))
}

#[cfg(target_os = "linux")]
struct ServicePortChange<'a> {
    service_path: &'a Path,
    service_name: &'a str,
    old_port: u16,
    new_port: u16,
    original: &'a [u8],
    replacement: &'a [u8],
}

#[cfg(target_os = "linux")]
fn change_service_unit<C, P, W>(
    change: ServicePortChange<'_>,
    mut call: C,
    mut ensure_port: P,
    mut wait_ready: W,
) -> Result<String, String>
where
    C: FnMut(&[&str]) -> Result<(), String>,
    P: FnMut(u16) -> Result<(), String>,
    W: FnMut(&str, u16) -> Result<(), String>,
{
    let ServicePortChange {
        service_path,
        service_name,
        old_port,
        new_port,
        original,
        replacement,
    } = change;
    call(&["stop", service_name])?;
    if let Err(error) = ensure_port(new_port) {
        let restart =
            call(&["start", service_name]).and_then(|()| wait_ready(service_name, old_port));
        return match restart {
            Ok(()) => Err(format!("{error}; the existing service was started again")),
            Err(restart_error) => Err(format!(
                "{error}; additionally, restarting the previous service failed: {restart_error}"
            )),
        };
    }
    if let Err(error) = replace_unit(service_path, replacement) {
        let restart =
            call(&["start", service_name]).and_then(|()| wait_ready(service_name, old_port));
        return match restart {
            Ok(()) => Err(format!("could not write the new unit; the old service was started again: {error}")),
            Err(restart_error) => Err(format!("could not write the new unit: {error}; restarting the old service also failed: {restart_error}")),
        };
    }
    if let Err(reload_error) = call(&["daemon-reload"]) {
        let restore = replace_unit(service_path, original)
            .and_then(|()| call(&["daemon-reload"]))
            .and_then(|()| call(&["start", service_name]))
            .and_then(|()| wait_ready(service_name, old_port));
        return match restore {
            Ok(()) => Err(format!("systemd could not reload the new unit; the previous unit and service were restored: {reload_error}")),
            Err(restore_error) => Err(format!("systemd could not reload the new unit: {reload_error}; rollback could not fully restore the previous service: {restore_error}")),
        };
    }
    let start_result =
        call(&["restart", service_name]).and_then(|()| wait_ready(service_name, new_port));
    if let Err(start_error) = start_result {
        let restore = replace_unit(service_path, original)
            .and_then(|()| call(&["daemon-reload"]))
            .and_then(|()| call(&["start", service_name]))
            .and_then(|()| wait_ready(service_name, old_port));
        return match restore {
            Ok(()) => Err(format!("new service failed to start; previous unit and service were restored: {start_error}")),
            Err(restore_error) => Err(format!("new service failed to start: {start_error}; rollback could not fully restore the previous service: {restore_error}")),
        };
    }
    Ok(format!(
        "changed {service_name} to port {new_port}; the service was restarted"
    ))
}

#[cfg(target_os = "linux")]
fn systemctl(arguments: &[&str]) -> Result<(), String> {
    let output = Command::new("systemctl")
        .arg("--user")
        .args(arguments)
        .output()
        .map_err(|error| format!("could not execute systemctl --user: {error}"))?;
    check_systemctl_output(output, arguments)
}

#[cfg(target_os = "linux")]
fn check_project(entry: &Path) -> Result<(), String> {
    let executable = env::current_exe()
        .map_err(|error| format!("cannot determine the Zelyra executable: {error}"))?;
    let output = Command::new(executable)
        .arg("check")
        .arg(entry)
        .output()
        .map_err(|error| format!("could not check the project before service changes: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    Err("project compiler checks failed; the service was not changed".into())
}

#[cfg(target_os = "linux")]
fn wait_until_ready(service_name: &str, port: u16) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let active = systemctl(&["is-active", "--quiet", service_name]).is_ok();
        let accepting_connections = TcpStream::connect(("127.0.0.1", port)).is_ok();
        if active && accepting_connections {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "127.0.0.1:{port} did not become ready within five seconds"
            ));
        }
        thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(target_os = "linux")]
fn check_systemctl_output(output: Output, arguments: &[&str]) -> Result<(), String> {
    if output.status.success() {
        if arguments.first() == Some(&"status") && !output.stdout.is_empty() {
            print!("{}", String::from_utf8_lossy(&output.stdout));
        }
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Err(if stderr.is_empty() {
        format!(
            "systemctl --user {} failed with {}",
            arguments.join(" "),
            output.status
        )
    } else {
        format!("systemctl --user {} failed: {stderr}", arguments.join(" "))
    })
}

#[cfg(target_os = "linux")]
fn write_new_unit(path: &Path, contents: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "unit path has no parent".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            format!(
                "cannot create {} without overwriting: {error}",
                path.display()
            )
        })?;
    if let Err(error) = file.write_all(contents).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(path);
        return Err(format!("cannot write service unit: {error}"));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn replace_unit(path: &Path, contents: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "unit path has no parent".to_owned())?;
    let temp = parent.join(format!(".zelyra-service-{}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|error| format!("cannot create temporary service unit: {error}"))?;
    if let Err(error) = file.write_all(contents).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temp);
        return Err(format!("cannot write temporary service unit: {error}"));
    }
    if let Err(error) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(format!("cannot atomically replace service unit: {error}"));
    }
    Ok(())
}

fn usage() {
    eprintln!(
        "Usage: zelyra service <install --port <port>|change --port <port>|status|restart|remove>"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_port_is_required_and_bounded() {
        for args in [
            vec![],
            vec!["--port".into()],
            vec!["--port".into(), "0".into()],
            vec!["--port".into(), "65536".into()],
            vec!["3000".into()],
        ] {
            assert!(parse_port_arg(&mut args.into_iter()).is_err());
        }
        assert_eq!(
            parse_port_arg(&mut vec!["--port".into(), "3001".into()].into_iter()),
            Ok(3001)
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn systemd_argument_quoting_preserves_spaces_and_specifiers() {
        assert_eq!(
            systemd_quote("/opt/My app/100%/zelyra"),
            "\"/opt/My app/100%%/zelyra\""
        );
        assert_eq!(systemd_quote("a\\b\"c"), "\"a\\\\b\\\"c\"");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn service_port_metadata_is_required_and_validated() {
        assert_eq!(current_port(b"# Zelyra-Port: 3001\n"), Ok(3001));
        assert!(current_port(b"[Service]\n").is_err());
        assert!(current_port(b"# Zelyra-Port: 0\n").is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn generated_systemd_unit_has_fixed_port_and_optional_project_environment() {
        let unit = render_unit_for(
            Path::new("/srv/invoice tutorial"),
            Path::new("/srv/invoice tutorial/main.zyl"),
            Path::new("/usr/local/bin/zelyra"),
            Path::new("/srv/invoice tutorial/.env"),
            true,
            3001,
        )
        .unwrap();
        assert!(unit.contains("WorkingDirectory=/srv/invoice\\x20tutorial"));
        assert!(unit.contains("serve \"/srv/invoice tutorial/main.zyl\" 127.0.0.1:3001"));
        assert!(unit.contains("EnvironmentFile=-/srv/invoice\\x20tutorial/.env"));
        assert!(unit.contains("Environment=ZELYRA_SYSTEMD_SERVICE_ID="));
        assert!(
            unit.find("Environment=ZELYRA_SYSTEMD_SERVICE_ID=").unwrap()
                > unit.find("EnvironmentFile=-").unwrap()
        );
        assert!(unit.contains("# Zelyra-Port: 3001"));
        assert!(!unit.contains("Description=Zelyra application (/srv"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn service_refuses_a_port_that_is_already_bound() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(ensure_port_available(port)
            .unwrap_err()
            .contains("already in use"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn unit_generation_rejects_newline_path_injection() {
        let result = render_unit_for(
            Path::new("/srv/project\nExecStart=/bin/sh"),
            Path::new("/srv/project/main.zyl"),
            Path::new("/usr/bin/zelyra"),
            Path::new("/srv/project/.env"),
            false,
            3000,
        );
        assert!(result.unwrap_err().contains("control characters"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn systemd_analyze_accepts_generated_unit_when_available() {
        if Command::new("systemd-analyze")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let directory =
            env::temp_dir().join(format!("zelyra-systemd-unit-test-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let workspace = directory.join("invoice tutorial");
        fs::create_dir_all(&workspace).unwrap();
        let unit = render_unit_for(
            &workspace,
            &workspace.join("main.zyl"),
            Path::new("/usr/bin/true"),
            &workspace.join(".env"),
            true,
            3000,
        )
        .unwrap();
        let unit_path = directory.join("zelyra-test.service");
        fs::write(&unit_path, unit).unwrap();
        let output = Command::new("systemd-analyze")
            .arg("verify")
            .arg(&unit_path)
            .output()
            .unwrap();
        let _ = fs::remove_dir_all(&directory);
        assert!(
            output.status.success(),
            "systemd-analyze verify rejected generated unit: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn failed_port_restart_restores_previous_unit_and_service() {
        let directory = env::temp_dir().join(format!(
            "zelyra-service-rollback-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let unit_path = directory.join("zelyra-test.service");
        let original = b"# Zelyra-Port: 3000\nold unit\n";
        let replacement = b"# Zelyra-Port: 3001\nnew unit\n";
        fs::write(&unit_path, original).unwrap();
        let mut calls = Vec::new();
        let result = change_service_unit(
            ServicePortChange {
                service_path: &unit_path,
                service_name: "zelyra-test.service",
                old_port: 3000,
                new_port: 3001,
                original,
                replacement,
            },
            |arguments| {
                calls.push(
                    arguments
                        .iter()
                        .map(|argument| (*argument).to_owned())
                        .collect::<Vec<_>>(),
                );
                if arguments.first() == Some(&"restart") {
                    Err("simulated start failure".into())
                } else {
                    Ok(())
                }
            },
            |_| Ok(()),
            |_, _| Ok(()),
        );
        assert!(result
            .unwrap_err()
            .contains("previous unit and service were restored"));
        assert_eq!(fs::read(&unit_path).unwrap(), original);
        assert_eq!(
            calls,
            vec![
                vec!["stop".to_owned(), "zelyra-test.service".to_owned()],
                vec!["daemon-reload".to_owned()],
                vec!["restart".to_owned(), "zelyra-test.service".to_owned()],
                vec!["daemon-reload".to_owned()],
                vec!["start".to_owned(), "zelyra-test.service".to_owned()],
            ]
        );
        let _ = fs::remove_dir_all(&directory);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn occupied_new_port_keeps_old_unit_and_restarts_old_service() {
        let directory = env::temp_dir().join(format!(
            "zelyra-service-busy-port-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let unit_path = directory.join("zelyra-test.service");
        let original = b"# Zelyra-Port: 3000\nold unit\n";
        fs::write(&unit_path, original).unwrap();
        let mut calls = Vec::new();
        let result = change_service_unit(
            ServicePortChange {
                service_path: &unit_path,
                service_name: "zelyra-test.service",
                old_port: 3000,
                new_port: 3001,
                original,
                replacement: b"new unit",
            },
            |arguments| {
                calls.push(
                    arguments
                        .iter()
                        .map(|argument| (*argument).to_owned())
                        .collect::<Vec<_>>(),
                );
                Ok(())
            },
            |_| Err("target is busy".into()),
            |_, _| Ok(()),
        );
        assert!(result.unwrap_err().contains("target is busy"));
        assert_eq!(fs::read(&unit_path).unwrap(), original);
        assert_eq!(
            calls,
            vec![
                vec!["stop".to_owned(), "zelyra-test.service".to_owned()],
                vec!["start".to_owned(), "zelyra-test.service".to_owned()],
            ]
        );
        let _ = fs::remove_dir_all(&directory);
    }
}
