//! Offline scaffolding for reusable local Zelyra packages.

use semver::Version;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_STAGING: AtomicU64 = AtomicU64::new(0);

pub(crate) fn command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let Some(directory) = arguments.next() else {
        usage();
        return ExitCode::from(2);
    };
    let mut name = None;
    let mut package_version = "0.1.0".to_owned();
    let mut has_version = false;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--name" => {
                if name.is_some() {
                    report_error("`--name` must be supplied once with a package name");
                    return ExitCode::from(2);
                }
                match arguments.next() {
                    Some(value) => name = Some(value),
                    None => {
                        report_error("`--name` must be supplied once with a package name");
                        return ExitCode::from(2);
                    }
                }
            }
            "--version" => {
                if has_version {
                    report_error("`--version` must be supplied at most once");
                    return ExitCode::from(2);
                }
                match arguments.next() {
                    Some(value) => {
                        package_version = value;
                        has_version = true;
                    }
                    None => {
                        report_error("`--version` requires a SemVer value");
                        return ExitCode::from(2);
                    }
                }
            }
            _ => {
                report_error(&format!("unknown module init option `{argument}`"));
                usage();
                return ExitCode::from(2);
            }
        }
    }
    let Some(name) = name else {
        report_error("`--name` is required");
        usage();
        return ExitCode::from(2);
    };
    if let Err(error) = crate::packages::validate_package_name(&name) {
        report_error(&error.message);
        return ExitCode::from(2);
    }
    let package_version = match Version::parse(&package_version) {
        Ok(version) => version,
        Err(problem) => {
            report_error(&format!("invalid package SemVer version: {problem}"));
            return ExitCode::from(2);
        }
    };
    let compiler =
        Version::parse(env!("CARGO_PKG_VERSION")).expect("CLI package version is valid SemVer");
    let Some(next_minor) = compiler.minor.checked_add(1) else {
        report_error("cannot construct the compiler compatibility range");
        return ExitCode::FAILURE;
    };
    let zelyra_requirement = format!(
        ">={}.{}.{}, <{}.{}.0",
        compiler.major, compiler.minor, 0, compiler.major, next_minor
    );

    let target = PathBuf::from(directory);
    if fs::symlink_metadata(&target).is_ok() {
        report_error(&format!(
            "refusing to initialize `{}` because that path already exists",
            target.display()
        ));
        return ExitCode::FAILURE;
    }
    if let Some(parent) = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        if let Err(problem) = fs::create_dir_all(parent) {
            report_error(&format!("cannot create module parent directory: {problem}"));
            return ExitCode::FAILURE;
        }
    }
    let parent = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let staging = loop {
        let sequence = NEXT_STAGING.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(
            ".zelyra-module-{}-{sequence}.tmp",
            std::process::id()
        ));
        match fs::create_dir(&candidate) {
            Ok(()) => break candidate,
            Err(problem) if problem.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(problem) => {
                report_error(&format!(
                    "cannot create module staging directory: {problem}"
                ));
                return ExitCode::FAILURE;
            }
        }
    };
    let result = write_scaffold(&staging, &name, &package_version, &zelyra_requirement)
        .and_then(|()| crate::module_cli::publish_directory_no_replace(&staging, &target));
    if let Err(problem) = result {
        let _ = fs::remove_dir_all(&staging);
        report_error(&format!("cannot publish module scaffold: {problem}"));
        return ExitCode::FAILURE;
    }
    println!(
        "created local Zelyra package `{name}` at {}",
        target.display()
    );
    println!("next: declare its local path under [dependencies] and import it with `pkg:<dependency-key>`");
    ExitCode::SUCCESS
}

fn write_scaffold(
    staging: &Path,
    name: &str,
    version: &Version,
    zelyra_requirement: &str,
) -> std::io::Result<()> {
    fs::create_dir(staging.join("src"))?;
    let version_text = version.to_string();
    let manifest = format!(
        "[package]\nname = {name:?}\nversion = {version_text:?}\nentry = \"src/lib.zyl\"\nzelyra = {zelyra_requirement:?}\neffects = []\n"
    );
    fs::write(staging.join("zelyra.toml"), manifest)?;
    fs::write(
        staging.join("src/lib.zyl"),
        format!(
            "/// Returns the identity of this starter package.\npub fn module_name() -> String {{\n    return {name:?}\n}}\n"
        ),
    )?;
    fs::write(
        staging.join("README.md"),
        format!(
            "# {name}\n\nLocal Zelyra package `{name}` at version `{version}`.\n\nThe initial API is `module_name() -> String`. Add public, typed declarations to `src/lib.zyl`; keep required effects explicit and configure them in the consuming application.\n\nExample dependency:\n\n```toml\n[dependencies.{name}]\npath = \"packages/{name}\"\nversion = \"^{version}\"\n```\n\nThe package is offline and has no build scripts or external dependencies.\n"
        ),
    )?;
    Ok(())
}

fn report_error(message: &str) {
    eprintln!("error[E-PKG-001]: {message}");
}

fn usage() {
    eprintln!("usage: zelyra module init <directory> --name <package-name> [--version <semver>]");
}
