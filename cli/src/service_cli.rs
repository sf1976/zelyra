use super::*;

const SERVICE_HELP: &str = "Usage: zelyra service install --port <1..65535>\n\nInstalls and starts this project as a per-user Linux systemd service. It binds Zelyra to 127.0.0.1 and never chooses a different port. The command requires main.zyl, a working systemd user manager, and an unused port. See docs/service.en.md for reboot persistence and reverse-proxy setup.";

pub(super) fn service_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    match args.next().as_deref() {
        Some("install") => install_command(args),
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
            println!("Manage it with: systemctl --user status {unit_name}");
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
        "[Unit]\nDescription=Zelyra application on port {port}\nAfter=network.target\n\n[Service]\nType=simple\nWorkingDirectory={working_directory}\nExecStart={executable} serve main.zyl 127.0.0.1:{port} --fixed-port\nRestart=on-failure\nRestartSec=3\n\n[Install]\nWantedBy=default.target\n"
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
        assert!(unit.contains("WorkingDirectory=/srv/invoices\\x20app\n"));
        assert!(unit.contains(
            "ExecStart=\"/home/stefan/bin/zelyra\" serve main.zyl 127.0.0.1:4317 --fixed-port\n"
        ));
        assert!(unit.contains("Restart=on-failure\n"));
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
