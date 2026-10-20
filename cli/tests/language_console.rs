use std::io::Write;
use std::process::{Command, Stdio};

fn run_console(input: &str) -> std::process::Output {
    let directory = std::env::temp_dir().join(format!(
        "zelyra-language-console-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock is after epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).expect("create temporary project");
    std::fs::write(directory.join("main.zyl"), "fn main() {}\n")
        .expect("write valid project entry");

    let mut child = Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .arg("console")
        .arg("main.zyl")
        .arg("--sandbox")
        .current_dir(&directory)
        .env_remove("DATABASE_URL")
        .env_remove("ZELYRA_DATABASE_MAIN_URL")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start zelyra console");
    child
        .stdin
        .take()
        .expect("console stdin")
        .write_all(input.as_bytes())
        .expect("write console input");
    let output = child.wait_with_output().expect("wait for console");
    std::fs::remove_dir_all(directory).expect("remove temporary project");
    output
}

#[test]
fn piped_console_keeps_bindings_between_inputs() {
    let output = run_console("x = 10\ny = 20\nx + y\n");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("=> 30"));
}

#[test]
fn piped_console_recovers_after_a_runtime_error() {
    let output = run_console("1 / 0\n\"weiter\"\n");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error[E-RUNTIME-001]"), "{stderr}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("=> \"weiter\""));
}
