use std::{
    env, fs,
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
};

pub(super) fn command(arguments: impl Iterator<Item = String>) -> ExitCode {
    let mut confirmed = false;
    let mut dry_run = false;
    for argument in arguments {
        match argument.as_str() {
            "--yes" | "-y" if !confirmed => confirmed = true,
            "--dry-run" if !dry_run => dry_run = true,
            _ => {
                eprintln!("Usage: zelyra uninstall [--yes] [--dry-run]");
                return ExitCode::from(2);
            }
        }
    }

    let executable = match env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            eprintln!(
                "error[E-UNINSTALL-001]: cannot locate the active Zelyra executable: {error}"
            );
            return ExitCode::FAILURE;
        }
    };
    let executable = match fs::canonicalize(&executable) {
        Ok(path) => path,
        Err(error) => {
            eprintln!(
                "error[E-UNINSTALL-001]: cannot resolve the active Zelyra executable: {error}"
            );
            return ExitCode::FAILURE;
        }
    };
    if !executable.is_file() {
        eprintln!(
            "error[E-UNINSTALL-001]: active executable is not a regular file: {}",
            executable.display()
        );
        return ExitCode::FAILURE;
    }

    let duplicates = path_copies(&executable);
    if let Some(manager) = package_manager_owner(&executable) {
        eprintln!("Zelyra is managed by {manager}: {}", executable.display());
        eprintln!("Remove it with that package manager so its files and package database stay consistent.");
        print_duplicate_copies(&duplicates);
        return ExitCode::from(1);
    }

    println!("Selected Zelyra executable: {}", executable.display());
    println!(
        "Only this executable will be removed. Projects, settings, logs, and databases are kept."
    );
    print_duplicate_copies(&duplicates);
    if dry_run {
        println!("Dry run: no files were changed.");
        return ExitCode::SUCCESS;
    }

    if !confirmed {
        if !io::stdin().is_terminal() {
            eprintln!("error[E-UNINSTALL-002]: confirmation is required in a terminal; rerun with `zelyra uninstall --yes` to confirm explicitly");
            return ExitCode::from(2);
        }
        print!("Remove this Zelyra executable? Type `yes` to continue: ");
        if io::stdout().flush().is_err() {
            eprintln!("error[E-UNINSTALL-003]: could not display the confirmation prompt");
            return ExitCode::FAILURE;
        }
        let mut answer = String::new();
        if io::stdin().read_line(&mut answer).is_err()
            || !matches!(answer.trim().to_ascii_lowercase().as_str(), "yes" | "ja")
        {
            println!("Uninstall cancelled.");
            return ExitCode::SUCCESS;
        }
    }

    #[cfg(windows)]
    let result = schedule_windows_removal(&executable);
    #[cfg(not(windows))]
    let result = fs::remove_file(&executable);

    match result {
        Ok(()) => {
            println!("Zelyra executable removed: {}", executable.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!(
                "error[E-UNINSTALL-004]: could not remove `{}`: {error}",
                executable.display()
            );
            eprintln!("Check file permissions or remove this executable manually.");
            ExitCode::FAILURE
        }
    }
}

fn print_duplicate_copies(duplicates: &[PathBuf]) {
    if duplicates.is_empty() {
        println!("No other Zelyra executable was found on PATH.");
        return;
    }
    println!("Other Zelyra executables on PATH (left untouched):");
    for path in duplicates {
        println!("  {}", path.display());
    }
}

fn path_copies(active: &Path) -> Vec<PathBuf> {
    let binary_name = if cfg!(windows) {
        "zelyra.exe"
    } else {
        "zelyra"
    };
    let active = fs::canonicalize(active).unwrap_or_else(|_| active.to_path_buf());
    let mut found = env::split_paths(&env::var_os("PATH").unwrap_or_default())
        .filter_map(|directory| {
            let candidate = directory.join(binary_name);
            if !candidate.is_file() {
                return None;
            }
            let candidate = fs::canonicalize(&candidate).unwrap_or(candidate);
            (candidate != active).then_some(candidate)
        })
        .collect::<Vec<_>>();
    found.sort();
    found.dedup();
    found
}

fn package_manager_owner(executable: &Path) -> Option<&'static str> {
    #[cfg(unix)]
    {
        let path = executable.as_os_str();
        for (program, args, label) in [
            ("dpkg-query", vec!["-S", path.to_str()?], "dpkg/apt"),
            ("rpm", vec!["-qf", path.to_str()?], "RPM"),
            ("pacman", vec!["-Qo", path.to_str()?], "pacman"),
        ] {
            if Command::new(program)
                .args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success())
            {
                return Some(label);
            }
        }
        if Command::new("brew")
            .args(["list", "--formula", "zelyra"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
        {
            return Some("Homebrew");
        }
    }
    #[cfg(windows)]
    {
        let lower = executable.to_string_lossy().to_ascii_lowercase();
        if lower.contains("\\windowsapps\\") {
            return Some("Microsoft Store");
        }
        if lower.contains("\\chocolatey\\") {
            return Some("Chocolatey");
        }
        if lower.contains("\\scoop\\apps\\") {
            return Some("Scoop");
        }
    }
    None
}

#[cfg(windows)]
fn schedule_windows_removal(executable: &Path) -> io::Result<()> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let encoded_path = base64_utf16(executable.as_os_str().to_string_lossy().as_ref());
    let script = format!(
        "$p=[Text.Encoding]::Unicode.GetString([Convert]::FromBase64String('{}')); Start-Sleep -Milliseconds 900; Remove-Item -LiteralPath $p -Force",
        encoded_path
    );
    Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
        ])
        .arg(&script)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
}

#[cfg(windows)]
fn base64_utf16(value: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = value
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0] as u32;
        let b = chunk.get(1).copied().unwrap_or(0) as u32;
        let c = chunk.get(2).copied().unwrap_or(0) as u32;
        let value = (a << 16) | (b << 8) | c;
        for index in 0..4 {
            if index > chunk.len() {
                encoded.push('=');
            } else {
                encoded.push(ALPHABET[((value >> (18 - index * 6)) & 0x3f) as usize] as char);
            }
        }
    }
    encoded
}
