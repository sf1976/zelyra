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
        .join("examples/csv_catalog")
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

fn temporary_copy() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos();
    let destination = std::env::temp_dir().join(format!(
        "zelyra-csv-catalog-{}-{unique}",
        std::process::id()
    ));
    copy_directory(&example(), &destination);
    destination
}

fn cli(project: &Path, arguments: &[&str]) -> std::process::Output {
    Command::new(binary())
        .args(arguments)
        .current_dir(project)
        .output()
        .expect("Zelyra CLI should run")
}

#[test]
fn csv_catalog_checks_import_validation_error_export_and_round_trip() {
    let project = temporary_copy();
    let check = cli(&project, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "project check failed: {}",
        String::from_utf8_lossy(&check.stderr)
    );

    let tests = cli(&project, &["run", "tests.zyl"]);
    assert!(
        tests.status.success(),
        "application tests failed: {}",
        String::from_utf8_lossy(&tests.stderr)
    );
    let output = String::from_utf8_lossy(&tests.stdout);
    assert!(output.contains("PASS Export und erneuter Import erhalten alle Artikel"));
    assert!(output.contains("PASS Leere, falsche Kopfzeile und beschaedigte CSV"));
    assert!(output.contains("PASS Verstaendlicher Fehler fuer fehlende Datei"));
    assert!(output.contains("Testergebnis bestanden / fehlgeschlagen:\n10\n0\n"));

    let valid_csv = fs::read_to_string(project.join("data/articles_valid.csv"))
        .expect("valid article export should be UTF-8");
    assert!(valid_csv.starts_with("Artikelnummer;Name;Kategorie;Preis;Lagerbestand;Aktiv\n"));
    assert!(valid_csv.contains("101;\"Äpfel; \"\"Bio\"\"\nErnte\";"));
    assert!(valid_csv.find("101;").unwrap() < valid_csv.find("202;").unwrap());
    assert!(valid_csv.find("202;").unwrap() < valid_csv.find("303;").unwrap());

    let errors_csv = fs::read_to_string(project.join("data/import_errors.csv"))
        .expect("error export should be UTF-8");
    assert!(errors_csv.starts_with("Datensatznummer;Feld;Fehlergrund\n"));
    assert!(errors_csv.contains("4;Artikelnummer;ist bereits"));
    assert!(errors_csv.contains("6;Preis;muss eine Zahl mit hoechstens zwei Nachkommastellen sein"));

    fs::remove_dir_all(project).expect("temporary project should be removed");
}
