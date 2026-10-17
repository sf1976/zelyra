use super::*;

const SERVICE_HELP: &str = "Usage: zelyra service install --port <1..65535>\n       zelyra service change --port <1..65535>\n       zelyra service status\n       zelyra service restart\n       zelyra service stop\n       zelyra service remove\n\nManages this project as a per-user Linux systemd service bound to 127.0.0.1. Ports are always explicit; Zelyra never chooses another port. See docs/service.en.md for reboot persistence and reverse-proxy setup.";

pub(super) fn service_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    match args.next().as_deref() {
        Some("install") => install_command(args),
        Some("change") => change_command(args),
        Some("status") => lifecycle_command("status", args),
        Some("restart") => lifecycle_command("restart", args),
        Some("stop") => lifecycle_command("stop", args),
        Some("remove") => lifecycle_command("remove", args),
        Some("--help" | "-h") | None => {
            println!("{SERVICE_HELP}");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{SERVICE_HELP}");
            ExitCode::from(2)
        }
    }
}

fn install_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let mut port = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--port" if port.is_none() => {
                let Some(value) = args.next() else {
                    eprintln!("service install: --port requires a port number");
                    return ExitCode::from(2);
                };
                match value.parse::<u16>() {
                    Ok(value) if value != 0 => port = Some(value),
                    _ => {
                        eprintln!("service install: port must be an integer from 1 through 65535");
                        return ExitCode::from(2);
                    }
                }
            }
            _ => {
                eprintln!("{SERVICE_HELP}");
                return ExitCode::from(2);
            }
        }
    }
    let Some(port) = port else {
        eprintln!("service install: --port is required\n\n{SERVICE_HELP}");
        return ExitCode::from(2);
    };

    match install_user_service(port) {
        Ok(unit_name) => {
            println!("Installed and started {unit_name} on http://127.0.0.1:{port}");
            println!("Check it with: zelyra service status");
            println!("A user service starts after login. See docs/service.en.md to enable start-at-boot with systemd lingering.");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("service install: {error}");
            ExitCode::FAILURE
        }
    }
}

fn install_user_service(port: u16) -> Result<String, String> {
    if !cfg!(target_os = "linux") {
        return Err("this command currently supports Linux with systemd only".into());
    }

    let project_root =
        env::current_dir().map_err(|error| format!("cannot read current directory: {error}"))?;
    let project_root = fs::canonicalize(&project_root)
        .map_err(|error| format!("cannot resolve project directory: {error}"))?;
    if !project_root.join("main.zyl").is_file() {
        return Err(format!(
            "{} does not contain main.zyl; run this command from a Zelyra project directory",
            project_root.display()
        ));
    }
    let project_root_text = project_root.to_str().ok_or_else(|| {
        "project path is not valid UTF-8 and cannot be written to a systemd unit".to_owned()
    })?;
    let executable = env::current_exe()
        .map_err(|error| format!("cannot locate the running Zelyra executable: {error}"))?;
    let executable = fs::canonicalize(executable)
        .map_err(|error| format!("cannot resolve the Zelyra executable path: {error}"))?;
    let executable_text = executable
        .to_str()
        .ok_or_else(|| "Zelyra executable path is not valid UTF-8".to_owned())?;
    if executable_text.chars().any(char::is_control) || executable_text.contains('$') {
        return Err(
            "Zelyra executable path contains characters systemd cannot safely represent".into(),
        );
    }

    ensure_port_is_free(port)?;
    ensure_systemd_user_manager()?;

    let unit_name = unit_name_for_project(&project_root)?;
    let config_directory = user_unit_directory()?;
    fs::create_dir_all(&config_directory)
        .map_err(|error| format!("cannot create {}: {error}", config_directory.display()))?;
    let unit_path = config_directory.join(&unit_name);
    if unit_path.exists() {
        return Err(format!("{unit_name} is already installed for this project; inspect it with `systemctl --user status {unit_name}`"));
    }

    let unit_contents = render_systemd_user_unit(executable_text, project_root_text, port)?;
    write_unit_file(&unit_path, unit_contents.as_bytes())?;
    let reload = systemctl_user(&["daemon-reload"]);
    if let Err(error) = reload {
        let _ = fs::remove_file(&unit_path);
        return Err(format!("systemd could not reload user units: {error}"));
    }
    if let Err(error) = systemctl_user(&["enable", "--now", &unit_name]) {
        let _ = systemctl_user(&["disable", "--now", &unit_name]);
        let _ = fs::remove_file(&unit_path);
        let _ = systemctl_user(&["daemon-reload"]);
        return Err(format!("systemd could not enable and start the service; the unit file was rolled back: {error}"));
    }
    Ok(unit_name)
}

fn change_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let port = match parse_service_port(&mut args, "change") {
        Ok(port) => port,
        Err(error) => {
            eprintln!("{error}\n\n{SERVICE_HELP}");
            return ExitCode::from(2);
        }
    };
    match change_user_service(port) {
        Ok(unit_name) => {
            println!("Changed and restarted {unit_name} on http://127.0.0.1:{port}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("service change: {error}");
            ExitCode::FAILURE
        }
    }
}

fn parse_service_port(
    args: &mut impl Iterator<Item = String>,
    command: &str,
) -> Result<u16, String> {
    let mut port = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--port" if port.is_none() => {
                let value = args
                    .next()
                    .ok_or_else(|| format!("service {command}: --port requires a port number"))?;
                port = Some(
                    value
                        .parse::<u16>()
                        .ok()
                        .filter(|value| *value != 0)
                        .ok_or_else(|| {
                            format!(
                                "service {command}: port must be an integer from 1 through 65535"
                            )
                        })?,
                );
            }
            _ => {
                return Err(format!(
                    "service {command}: unexpected argument {argument:?}"
                ))
            }
        }
    }
    port.ok_or_else(|| format!("service {command}: --port is required"))
}

fn current_project_service() -> Result<(PathBuf, String, PathBuf), String> {
    if !cfg!(target_os = "linux") {
        return Err("this command currently supports Linux with systemd only".into());
    }
    let project_root = fs::canonicalize(
        env::current_dir().map_err(|error| format!("cannot read current directory: {error}"))?,
    )
    .map_err(|error| format!("cannot resolve project directory: {error}"))?;
    if !project_root.join("main.zyl").is_file() {
        return Err("run this command from a Zelyra project directory containing main.zyl".into());
    }
    let unit_name = unit_name_for_project(&project_root)?;
    let unit_path = user_unit_directory()?.join(&unit_name);
    Ok((project_root, unit_name, unit_path))
}

fn load_managed_unit(path: &Path, project_root: &Path) -> Result<Vec<u8>, String> {
    let contents = fs::read(path)
        .map_err(|error| format!("cannot read service unit {}: {error}", path.display()))?;
    let text =
        std::str::from_utf8(&contents).map_err(|_| "service unit is not valid UTF-8".to_owned())?;
    let expected = format!(
        "WorkingDirectory={}",
        escape_systemd_value(
            project_root
                .to_str()
                .ok_or_else(|| "project path is not valid UTF-8".to_owned())?
        )
    );
    if !text.lines().any(|line| line == "# Managed by Zelyra")
        || !text.lines().any(|line| line == expected)
    {
        return Err(
            "the unit is not a Zelyra-managed service for this project; refusing to modify it"
                .into(),
        );
    }
    Ok(contents)
}

fn change_user_service(port: u16) -> Result<String, String> {
    let (project_root, unit_name, unit_path) = current_project_service()?;
    let previous = load_managed_unit(&unit_path, &project_root)?;
    let old_text =
        std::str::from_utf8(&previous).map_err(|_| "service unit is not valid UTF-8".to_owned())?;
    let old_port = old_text
        .lines()
        .find_map(|line| line.strip_prefix("# Port: ")?.parse::<u16>().ok())
        .ok_or_else(|| {
            "managed service unit has no valid port marker; refusing to change it".to_owned()
        })?;
    if old_port == port {
        return Err(format!("service already uses port {port}"));
    }
    ensure_port_is_free(port)?;
    ensure_systemd_user_manager()?;
    let executable = fs::canonicalize(
        env::current_exe().map_err(|error| format!("cannot locate Zelyra executable: {error}"))?,
    )
    .map_err(|error| format!("cannot resolve Zelyra executable: {error}"))?;
    let executable = executable
        .to_str()
        .ok_or_else(|| "Zelyra executable path is not valid UTF-8".to_owned())?;
    let project = project_root
        .to_str()
        .ok_or_else(|| "project path is not valid UTF-8".to_owned())?;
    let replacement = render_systemd_user_unit(executable, project, port)?;
    systemctl_user(&["stop", &unit_name])
        .map_err(|error| format!("could not stop service on port {old_port}: {error}"))?;
    let update_result = write_unit_file(&unit_path, replacement.as_bytes())
        .and_then(|()| systemctl_user(&["daemon-reload"]))
        .and_then(|()| systemctl_user(&["start", &unit_name]));
    if let Err(error) = update_result {
        let restored = write_unit_file(&unit_path, &previous)
            .and_then(|()| systemctl_user(&["daemon-reload"]))
            .and_then(|()| systemctl_user(&["start", &unit_name]));
        return match restored {
            Ok(()) => Err(format!("new port {port} could not be activated ({error}); restored and restarted the previous service on port {old_port}")),
            Err(rollback_error) => Err(format!("new port {port} failed ({error}); rollback also failed ({rollback_error}). The saved unit is at {}", unit_path.display())),
        };
    }
    Ok(unit_name)
}

fn lifecycle_command(action: &str, mut args: impl Iterator<Item = String>) -> ExitCode {
    if args.next().is_some() {
        eprintln!("service {action}: this command takes no arguments\n\n{SERVICE_HELP}");
        return ExitCode::from(2);
    }
    match lifecycle_user_service(action) {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("service {action}: {error}");
            ExitCode::FAILURE
        }
    }
}

fn lifecycle_user_service(action: &str) -> Result<String, String> {
    let (project_root, unit_name, unit_path) = current_project_service()?;
    let contents = load_managed_unit(&unit_path, &project_root)?;
    ensure_systemd_user_manager()?;
    if action == "remove" {
        if let Err(error) = systemctl_user(&["disable", "--now", &unit_name]) {
            let rollback = systemctl_user(&["enable", "--now", &unit_name]);
            return Err(match rollback {
                Ok(()) => format!("could not stop and disable {unit_name}; it was re-enabled: {error}"),
                Err(rollback_error) => format!("could not stop and disable {unit_name} ({error}); restoring its enabled state also failed ({rollback_error})"),
            });
        }
        if let Err(error) = fs::remove_file(&unit_path) {
            let rollback = systemctl_user(&["enable", "--now", &unit_name]);
            return Err(match rollback {
                Ok(()) => format!("cannot remove unit file; service was re-enabled: {error}"),
                Err(rollback_error) => format!(
                    "cannot remove unit file ({error}); re-enabling also failed ({rollback_error})"
                ),
            });
        }
        if let Err(error) = systemctl_user(&["daemon-reload"]) {
            let restored = write_unit_file(&unit_path, &contents)
                .and_then(|()| systemctl_user(&["daemon-reload"]))
                .and_then(|()| systemctl_user(&["enable", "--now", &unit_name]));
            return Err(match restored {
                Ok(()) => format!("systemd could not reload after removal; restored and restarted {unit_name}: {error}"),
                Err(rollback_error) => format!("systemd could not reload after removal ({error}); restoring the service also failed ({rollback_error})"),
            });
        }
        return Ok(format!("Removed {unit_name}"));
    }
    if action == "status" {
        let output = Command::new("systemctl")
            .args(["--user", "status", &unit_name])
            .output()
            .map_err(|error| format!("cannot run systemctl: {error}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        if !stdout.is_empty() {
            return Ok(stdout);
        }
        return if output.status.success() {
            Ok(format!("{unit_name} is active"))
        } else {
            Err(if stderr.is_empty() {
                output.status.to_string()
            } else {
                stderr
            })
        };
    }
    systemctl_user(&[action, &unit_name])?;
    let port = std::str::from_utf8(&contents).ok().and_then(|text| {
        text.lines()
            .find_map(|line| line.strip_prefix("# Port: ")?.parse::<u16>().ok())
    });
    Ok(match (action, port) {
        (_, Some(port)) => format!("{action} requested for {unit_name} (configured port {port})"),
        _ => format!("{action} requested for {unit_name}"),
    })
}

fn ensure_port_is_free(port: u16) -> Result<(), String> {
    TcpListener::bind(("127.0.0.1", port)).map(drop).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AddrInUse {
            format!("127.0.0.1:{port} is already in use; choose a free port explicitly with `zelyra service install --port <free-port>`")
        } else {
            format!("cannot reserve 127.0.0.1:{port}: {error}")
        }
    })
}

fn ensure_systemd_user_manager() -> Result<(), String> {
    let output = Command::new("systemctl")
        .args(["--user", "show-environment"])
        .output()
        .map_err(|error| {
            format!("cannot run systemctl: {error}; install systemd or manage the service manually")
        })?;
    if output.status.success() {
        Ok(())
    } else {
        Err("the systemd user manager is unavailable; log into a systemd session and retry (no service file was written)".into())
    }
}

fn user_unit_directory() -> Result<PathBuf, String> {
    let config_home = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .ok_or_else(|| "neither XDG_CONFIG_HOME nor HOME is set".to_owned())?;
    if !config_home.is_absolute() {
        return Err("XDG_CONFIG_HOME must be an absolute path".into());
    }
    Ok(config_home.join("systemd").join("user"))
}

fn unit_name_for_project(project_root: &Path) -> Result<String, String> {
    let project_name = project_root
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "cannot derive a service name from the project directory".to_owned())?;
    let slug = project_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let mut digest = Sha256::new();
    digest.update(project_root.as_os_str().as_encoded_bytes());
    let digest = digest.finalize();
    let suffix = digest[..5]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(format!(
        "zelyra-{}-{suffix}.service",
        if slug.is_empty() { "project" } else { &slug }
    ))
}

fn render_systemd_user_unit(
    executable: &str,
    project_root: &str,
    port: u16,
) -> Result<String, String> {
    if project_root.chars().any(char::is_control) {
        return Err(
            "project path contains control characters that systemd cannot safely represent".into(),
        );
    }
    let executable = quote_systemd_exec_arg(executable)?;
    let working_directory = escape_systemd_value(project_root);
    Ok(format!(
        "# Managed by Zelyra\n# Port: {port}\n[Unit]\nDescription=Zelyra application on port {port}\nAfter=network.target\n\n[Service]\nType=simple\nWorkingDirectory={working_directory}\nExecStart={executable} serve main.zyl 127.0.0.1:{port} --fixed-port\nRestart=on-failure\nRestartSec=3\n\n[Install]\nWantedBy=default.target\n"
    ))
}

fn quote_systemd_exec_arg(value: &str) -> Result<String, String> {
    if value.chars().any(char::is_control) || value.contains('$') {
        return Err("executable path contains characters systemd cannot safely represent".into());
    }
    Ok(format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('%', "%%")
    ))
}

fn escape_systemd_value(value: &str) -> String {
    value
        .replace('%', "%%")
        .replace('\\', "\\\\")
        .replace(' ', "\\x20")
        .replace('\t', "\\t")
}

fn write_unit_file(path: &Path, contents: &[u8]) -> Result<(), String> {
    let temporary_path = path.with_extension("service.tmp");
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary_path).map_err(|error| {
        format!(
            "cannot create temporary unit file {}: {error}",
            temporary_path.display()
        )
    })?;
    file.write_all(contents)
        .and_then(|()| file.sync_all())
        .map_err(|error| {
            let _ = fs::remove_file(&temporary_path);
            format!("cannot write temporary unit file: {error}")
        })?;
    fs::rename(&temporary_path, path).map_err(|error| {
        let _ = fs::remove_file(&temporary_path);
        format!(
            "cannot atomically install unit file {}: {error}",
            path.display()
        )
    })
}

fn systemctl_user(arguments: &[&str]) -> Result<(), String> {
    let output = Command::new("systemctl")
        .arg("--user")
        .args(arguments)
        .output()
        .map_err(|error| format!("cannot run systemctl: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(if stderr.is_empty() {
            format!("systemctl exited with {}", output.status)
        } else {
            stderr
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_uses_the_requested_fixed_loopback_port_and_project_working_directory() {
        let unit = render_systemd_user_unit("/home/stefan/bin/zelyra", "/srv/invoices app", 4317)
            .expect("valid unit");
        assert!(unit.starts_with("# Managed by Zelyra\n# Port: 4317\n"));
        assert!(unit.contains("WorkingDirectory=/srv/invoices\\x20app\n"));
        assert!(unit.contains(
            "ExecStart=\"/home/stefan/bin/zelyra\" serve main.zyl 127.0.0.1:4317 --fixed-port\n"
        ));
        assert!(unit.contains("Restart=on-failure\n"));
    }

    #[test]
    fn service_port_parser_requires_one_explicit_valid_port() {
        assert_eq!(
            parse_service_port(&mut ["--port".into(), "3001".into()].into_iter(), "change"),
            Ok(3001)
        );
        assert!(
            parse_service_port(&mut ["--port".into(), "0".into()].into_iter(), "change").is_err()
        );
        assert!(
            parse_service_port(&mut ["--port".into(), "70000".into()].into_iter(), "change")
                .is_err()
        );
        assert!(parse_service_port(
            &mut [
                "--port".into(),
                "3001".into(),
                "--port".into(),
                "3002".into()
            ]
            .into_iter(),
            "change"
        )
        .is_err());
        assert!(parse_service_port(&mut std::iter::empty(), "change").is_err());
    }

    #[test]
    fn unit_escapes_systemd_specifiers_and_exec_quoting() {
        let unit = render_systemd_user_unit("/opt/100%/zelyra", "/srv/100%/app", 3000)
            .expect("valid unit");
        assert!(unit.contains("WorkingDirectory=/srv/100%%/app\n"));
        assert!(unit.contains("ExecStart=\"/opt/100%%/zelyra\" serve"));
        assert!(quote_systemd_exec_arg("/tmp/a\"b")
            .unwrap()
            .contains("a\\\"b"));
        assert!(quote_systemd_exec_arg("/tmp/$HOME/zelyra").is_err());
    }

    #[test]
    fn service_names_are_stable_and_distinguish_same_named_project_directories() {
        let first = unit_name_for_project(Path::new("/srv/client-a/invoices")).unwrap();
        let second = unit_name_for_project(Path::new("/srv/client-b/invoices")).unwrap();
        assert!(first.starts_with("zelyra-invoices-"));
        assert!(first.ends_with(".service"));
        assert_ne!(first, second);
        assert_eq!(
            first,
            unit_name_for_project(Path::new("/srv/client-a/invoices")).unwrap()
        );
    }

    #[test]
    fn port_check_rejects_an_already_bound_loopback_port() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let error = ensure_port_is_free(port).unwrap_err();
        assert!(error.contains("already in use"));
        assert!(error.contains("service install --port <free-port>"));
    }

    #[test]
    fn fixed_port_mode_fails_instead_of_using_the_regular_serve_fallback() {
        let occupied = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = occupied.local_addr().unwrap().to_string();
        assert!(bind_web_application(&address, false).is_err());
        let (listener, actual, used_fallback) = bind_web_application(&address, true).unwrap();
        assert!(used_fallback);
        assert_ne!(actual.port(), occupied.local_addr().unwrap().port());
        drop(listener);
    }
}
