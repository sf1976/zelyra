use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_zelyra"))
}

fn example() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples/labyrinth_solver")
}

fn temporary_copy() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos();
    let destination = std::env::temp_dir().join(format!(
        "zelyra-labyrinth-solver-{}-{unique}",
        std::process::id()
    ));
    copy_directory(&example(), &destination);
    destination
}

fn copy_directory(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("temporary project directory should be created");
    for entry in fs::read_dir(source).expect("example directory should be readable") {
        let source_path = entry.expect("example entry should be readable").path();
        let destination_path = destination.join(source_path.file_name().unwrap());
        if source_path.is_dir() {
            copy_directory(&source_path, &destination_path);
        } else {
            fs::copy(source_path, destination_path).expect("example file should be copied");
        }
    }
}

fn cli(project: &Path, arguments: &[&str]) -> std::process::Output {
    Command::new(binary())
        .args(arguments)
        .current_dir(project)
        .output()
        .expect("Zelyra CLI should run")
}

#[test]
fn labyrinth_solver_checks_bfs_validation_persistence_and_display() {
    let project = temporary_copy();
    for source in ["main.zyl", "tests.zyl"] {
        let check = cli(&project, &["check", source]);
        assert!(
            check.status.success(),
            "{source} did not compile: {}",
            String::from_utf8_lossy(&check.stderr)
        );
    }

    let tests = cli(&project, &["run", "tests.zyl"]);
    assert!(
        tests.status.success(),
        "labyrinth exercise tests failed: {}",
        String::from_utf8_lossy(&tests.stderr)
    );
    assert!(String::from_utf8_lossy(&tests.stdout).contains("Alle Labyrinthtests bestanden."));

    let run = cli(&project, &["run", "main.zyl"]);
    assert!(run.status.success());
    let output = String::from_utf8_lossy(&run.stdout);
    assert!(output.contains("Kuerzester Weg: 6 Schritte"));
    assert!(output.contains("Untersuchte Felder:"));
    assert!(output.contains("Pfad: (0, 1)"));
    assert!(output.contains('*'));

    fs::write(project.join("data/maze.txt"), "S.\n.G.")
        .expect("invalid maze fixture should be written");
    let invalid = cli(&project, &["run", "main.zyl"]);
    assert!(invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stdout).contains("andere Laenge"));

    fs::remove_dir_all(project).expect("temporary project should be removed");
}
