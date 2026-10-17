use std::fs;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
#[cfg(target_os = "linux")]
fn serve_prepares_declared_schedule_export_before_listening() {
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
schedule daily { command: daily cron: "0 0 * * *" }
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
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start Zelyra server");

    let export = project.join(".zelyra/schedules.cron");
    for _ in 0..150 {
        if export.is_file() {
            break;
        }
        if let Some(status) = server.try_wait().expect("poll server process") {
            let output = server.wait_with_output().expect("collect server output");
            let _ = fs::remove_dir_all(&project);
            panic!(
                "zelyra serve exited before preparing schedules: {status}\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        thread::sleep(Duration::from_millis(20));
    }
    let generated = fs::read_to_string(&export).expect("serve should create schedule export");
    assert!(generated.contains("Zelyra schedule: daily"));
    assert!(generated.contains("schedule trigger"));
    assert!(generated.contains("--name 'daily'"));
    for _ in 0..150 {
        if project.join(".zelyra/serve.json").is_file() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    let trigger = Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .args(["schedule", "trigger", "main.zyl", "--name", "daily"])
        .current_dir(&project)
        .env("XDG_CONFIG_HOME", &config_home)
        .env("ZELYRA_FAKE_SYSTEMD_ACTIVE", &active_marker)
        .env("PATH", &test_path)
        .output()
        .expect("trigger schedule through live server");
    assert!(trigger.status.success());
    assert!(
        String::from_utf8_lossy(&trigger.stdout).contains("daily task executed"),
        "trigger output: {}\n{}",
        String::from_utf8_lossy(&trigger.stdout),
        String::from_utf8_lossy(&trigger.stderr)
    );

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
    let _ = server.wait();
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
