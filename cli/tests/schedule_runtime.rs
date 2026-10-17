use std::fs;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
fn run_help_describes_the_resident_scheduler_gate() {
    for (language, expected) in [
        (
            "en",
            "Starting `serve` manually does not start the scheduler",
        ),
        (
            "de",
            "Ein manuell gestartetes `serve` startet den Scheduler nicht",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_zelyra"))
            .args(["run", "--help"])
            .env("ZELYRA_LANGUAGE", language)
            .output()
            .expect("run CLI help");
        assert!(output.status.success());
        let help = String::from_utf8_lossy(&output.stdout);
        assert!(help.contains(expected), "{language} help: {help}");
        assert!(!help.contains("does not run a resident scheduler"));
        assert!(!help.contains("startet keinen residenten Scheduler"));
    }
}

#[test]
#[cfg(target_os = "linux")]
fn active_project_service_starts_resident_scheduler_without_cron_export() {
    let nonce = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos();
    let project = std::env::temp_dir().join(format!(
        "zelyra-schedule-serve-{}-{timestamp}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&project).expect("create temporary project");
    fs::write(
        project.join("main.zyl"),
        r#"page "/" {
    html { <html><body><main>Schedule test</main></body></html> }
}
command daily { handler: run_daily }
schedule daily { command: daily cron: "* * * * *" }
fn run_daily() { print("daily task executed") }
fn main() { }
"#,
    )
    .expect("write scheduled web project");

    let service_id = {
        use sha2::{Digest, Sha256};
        let mut digest = Sha256::new();
        digest.update(
            project
                .canonicalize()
                .expect("canonical project")
                .as_os_str()
                .as_encoded_bytes(),
        );
        format!("{:x}", digest.finalize())[..16].to_owned()
    };
    let config_home = project.join("test-config");
    let unit_dir = config_home.join("systemd/user");
    let fake_bin = project.join("fake-bin");
    let active_marker = project.join("fake-systemd-active");
    fs::create_dir_all(&unit_dir).expect("create fake systemd unit directory");
    fs::create_dir_all(&fake_bin).expect("create fake command directory");
    fs::write(
        unit_dir.join(format!("zelyra-{service_id}.service")),
        "test unit\n",
    )
    .expect("write fake installed unit");
    fs::write(&active_marker, "active\n").expect("mark fake service active");
    let systemctl = fake_bin.join("systemctl");
    fs::write(
        &systemctl,
        "#!/bin/sh\n[ -f \"$ZELYRA_FAKE_SYSTEMD_ACTIVE\" ]\n",
    )
    .expect("write fake systemctl");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&systemctl, fs::Permissions::from_mode(0o755))
            .expect("make fake systemctl executable");
    }
    let mut test_path = fake_bin.as_os_str().to_owned();
    test_path.push(":");
    test_path.push(std::env::var_os("PATH").unwrap_or_default());

    let mut server = Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .args(["serve", "main.zyl", "127.0.0.1:0"])
        .current_dir(&project)
        .env("XDG_CONFIG_HOME", &config_home)
        .env("ZELYRA_SYSTEMD_SERVICE_ID", &service_id)
        .env("ZELYRA_FAKE_SYSTEMD_ACTIVE", &active_marker)
        .env("PATH", &test_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start Zelyra server");
    let stdout = server.stdout.take().expect("capture service stdout");
    let (line_sender, line_receiver) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if line_sender.send(line).is_err() {
                break;
            }
        }
    });

    let state = project.join(".zelyra/serve.json");
    for _ in 0..150 {
        if state.is_file() {
            break;
        }
        if let Some(status) = server.try_wait().expect("poll server process") {
            let output = server.wait_with_output().expect("collect server output");
            let _ = fs::remove_dir_all(&project);
            panic!(
                "zelyra serve exited before starting schedules: {status}\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(!project.join(".zelyra/schedules.cron").exists());

    let deadline = Instant::now() + Duration::from_secs(70);
    let mut scheduler_started = false;
    let mut job_executed = false;
    while Instant::now() < deadline && !job_executed {
        if let Some(status) = server.try_wait().expect("poll scheduler service") {
            panic!("Zelyra service exited before scheduled command ran: {status}");
        }
        match line_receiver.recv_timeout(Duration::from_millis(250)) {
            Ok(line) => {
                scheduler_started |=
                    line.contains("Zelyra scheduler is active in the project service");
                job_executed |= line.contains("daily task executed");
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    assert!(
        scheduler_started,
        "service should start its resident scheduler"
    );
    assert!(
        job_executed,
        "resident scheduler should run the declared command at a minute boundary"
    );

    let legacy_trigger = Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .args(["schedule", "trigger", "main.zyl", "--name", "daily"])
        .current_dir(&project)
        .env("XDG_CONFIG_HOME", &config_home)
        .env("ZELYRA_FAKE_SYSTEMD_ACTIVE", &active_marker)
        .env("PATH", &test_path)
        .output()
        .expect("attempt a legacy cron trigger while resident service runs");
    assert!(legacy_trigger.status.success());
    assert!(String::from_utf8_lossy(&legacy_trigger.stderr)
        .contains("runs declared schedules in-process"));
    assert!(!String::from_utf8_lossy(&legacy_trigger.stdout).contains("daily task executed"));

    fs::remove_file(&active_marker).expect("mark fake service stopped");
    let inactive_trigger = Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .args(["schedule", "trigger", "main.zyl", "--name", "daily"])
        .current_dir(&project)
        .env("XDG_CONFIG_HOME", &config_home)
        .env("ZELYRA_FAKE_SYSTEMD_ACTIVE", &active_marker)
        .env("PATH", &test_path)
        .output()
        .expect("attempt schedule trigger after service stop");
    assert!(inactive_trigger.status.success());
    let inactive_message = String::from_utf8_lossy(&inactive_trigger.stderr);
    assert!(inactive_message.contains("Zelyra service is not installed and active"));
    assert!(!String::from_utf8_lossy(&inactive_trigger.stdout).contains("daily task executed"));

    fs::write(&active_marker, "active\n")
        .expect("restore fake service state for server stop check");

    let _ = server.kill();
    let server_output = server.wait_with_output().expect("collect server output");
    assert!(
        !server_output.status.success(),
        "test intentionally terminates the server"
    );
    let stopped_trigger = Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .args(["schedule", "trigger", "main.zyl", "--name", "daily"])
        .current_dir(&project)
        .env("XDG_CONFIG_HOME", &config_home)
        .env("ZELYRA_FAKE_SYSTEMD_ACTIVE", &active_marker)
        .env("PATH", &test_path)
        .output()
        .expect("attempt schedule trigger after server exits");
    assert!(stopped_trigger.status.success());
    assert!(String::from_utf8_lossy(&stopped_trigger.stderr)
        .contains("this project's `serve` is not running"));
    fs::remove_dir_all(&project).expect("remove temporary project");
}
