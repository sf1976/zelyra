use super::*;
use std::time::{Duration, SystemTime};

pub(super) fn files_command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    match arguments.next().as_deref() {
        Some("cleanup") => files_cleanup_command(arguments),
        Some("backup") => files_backup_command(arguments),
        Some("restore") => files_restore_command(arguments),
        _ => {
            usage();
            ExitCode::from(2)
        }
    }
}

fn files_cleanup_command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let mut entry = "main.zyl".to_owned();
    let mut entry_set = false;
    let mut min_age_days = 30_u64;
    let mut apply = false;
    let mut maintenance_window_acknowledged = false;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--min-age-days" => {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --min-age-days requires an integer");
                    return ExitCode::from(2);
                };
                min_age_days = match value.parse::<u64>() {
                    Ok(days) if (1..=3650).contains(&days) => days,
                    _ => {
                        eprintln!("error[E-CLI-001]: --min-age-days must be between 1 and 3650");
                        return ExitCode::from(2);
                    }
                };
            }
            "--apply" => apply = true,
            "--maintenance-window" => maintenance_window_acknowledged = true,
            value if value.starts_with('-') => {
                eprintln!("error[E-CLI-001]: unknown files cleanup option `{value}`");
                return ExitCode::from(2);
            }
            value if !entry_set => {
                entry = value.to_owned();
                entry_set = true;
            }
            value => {
                eprintln!("error[E-CLI-001]: unexpected files cleanup argument `{value}`");
                return ExitCode::from(2);
            }
        }
    }
    if apply && !maintenance_window_acknowledged {
        eprintln!("error[E-CLI-001]: --apply requires --maintenance-window; Zelyra checks that no upload-enabled server holds the storage lock");
        return ExitCode::from(2);
    }
    if !apply && maintenance_window_acknowledged {
        eprintln!("error[E-CLI-001]: --maintenance-window is only valid together with --apply");
        return ExitCode::from(2);
    }

    let program = match validate(&entry) {
        Ok(program) => program,
        Err(()) => return ExitCode::from(1),
    };
    let project_directory = Path::new(&entry)
        .parent()
        .filter(|directory| !directory.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let database_name = program
        .databases
        .first()
        .map(|database| database.name.as_str());
    let database_url = match database_url_for_project(project_directory, database_name) {
        Ok(Some(url)) => url,
        Ok(None) => {
            eprintln!("error[E-ENV-001]: DATABASE_URL is required to verify upload references");
            return ExitCode::from(1);
        }
        Err(error) => {
            eprintln!("error[E-ENV-001]: {error}");
            return ExitCode::from(1);
        }
    };
    let storage_root = project_directory.join("storage/zelyra");
    if !storage_root.exists() {
        println!("No managed upload storage exists for this project.");
        return ExitCode::SUCCESS;
    }
    let storage = match zelyra_web::file_storage::FileStorage::new(&storage_root) {
        Ok(storage) => storage,
        Err(error) => {
            eprintln!("error[E-FILE-001]: private upload storage is unavailable: {error}");
            return ExitCode::from(1);
        }
    };
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(
            min_age_days.saturating_mul(24 * 60 * 60),
        ))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let references = match load_file_references(&program, &database_url) {
        Ok(references) => references,
        Err(error) => {
            eprintln!("error[E-FILE-002]: could not verify managed file references: {error}");
            return ExitCode::from(1);
        }
    };
    let candidates = match storage.old_unreferenced_uploads(&references, cutoff, 100_000) {
        Ok(candidates) => candidates,
        Err(error) => {
            eprintln!("error[E-FILE-003]: upload cleanup preview failed: {error}");
            return ExitCode::from(1);
        }
    };
    println!(
        "{} old unreferenced upload artifact(s) found (minimum age: {min_age_days} days).",
        candidates.len()
    );
    for candidate in &candidates {
        println!(
            "tenant={} file_ref={} bytes={} kind={}",
            candidate.tenant_id,
            candidate.file_ref,
            candidate.size_bytes,
            if candidate.staging_artifact {
                "abandoned-staging"
            } else {
                "unreferenced"
            }
        );
    }
    if !apply || candidates.is_empty() {
        if !apply {
            println!("Preview only. Add --apply --maintenance-window after stopping all project servers to remove these files.");
        }
        return ExitCode::SUCCESS;
    }

    let _maintenance_lock = match storage.acquire_maintenance_lock() {
        Ok(lock) => lock,
        Err(error) => {
            eprintln!("error[E-FILE-005]: cannot begin upload maintenance: {error}");
            return ExitCode::from(1);
        }
    };

    // Refresh the database snapshot while the exclusive activity lock prevents
    // Zelyra servers from accepting uploads or starting against this storage.
    let references = match load_file_references(&program, &database_url) {
        Ok(references) => references,
        Err(error) => {
            eprintln!("error[E-FILE-002]: could not recheck managed file references: {error}");
            return ExitCode::from(1);
        }
    };
    let mut removed = 0usize;
    let mut failed = 0usize;
    for candidate in candidates {
        if !candidate.staging_artifact
            && references.contains(&(candidate.tenant_id, candidate.file_ref.clone()))
        {
            println!(
                "skipped tenant={} file_ref={}: a database reference appeared during preview",
                candidate.tenant_id, candidate.file_ref
            );
            continue;
        }
        let result = if candidate.staging_artifact {
            storage.remove_staging(candidate.tenant_id, &candidate.file_ref)
        } else {
            storage.remove(candidate.tenant_id, &candidate.file_ref)
        };
        match result {
            Ok(()) => removed += 1,
            Err(error) => {
                failed += 1;
                eprintln!(
                    "error[E-FILE-004]: could not remove tenant={} file_ref={}: {error}",
                    candidate.tenant_id, candidate.file_ref
                );
            }
        }
    }
    println!("Removed {removed} artifact(s); {failed} failed.");
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn files_backup_command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let mut entry = "main.zyl".to_owned();
    let mut entry_set = false;
    let mut output = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--output" => {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --output requires a new backup directory path");
                    return ExitCode::from(2);
                };
                output = Some(PathBuf::from(value));
            }
            value if value.starts_with('-') => {
                eprintln!("error[E-CLI-001]: unknown files backup option `{value}`");
                return ExitCode::from(2);
            }
            value if !entry_set => {
                entry = value.to_owned();
                entry_set = true;
            }
            value => {
                eprintln!("error[E-CLI-001]: unexpected files backup argument `{value}`");
                return ExitCode::from(2);
            }
        }
    }
    let Some(output) = output else {
        eprintln!("error[E-CLI-001]: files backup requires --output <new-directory>");
        return ExitCode::from(2);
    };
    let (program, database_url, project_directory) = match file_storage_context(&entry) {
        Ok(context) => context,
        Err(code) => return code,
    };
    let references = match load_file_references(&program, &database_url) {
        Ok(references) => references,
        Err(error) => {
            eprintln!("error[E-FILE-002]: could not verify upload references: {error}");
            return ExitCode::from(1);
        }
    };
    let storage = match zelyra_web::file_storage::FileStorage::new(
        project_directory.join("storage/zelyra"),
    ) {
        Ok(storage) => storage,
        Err(error) => {
            eprintln!("error[E-FILE-001]: private upload storage is unavailable: {error}");
            return ExitCode::from(1);
        }
    };
    match storage.backup_to_directory(&output, &references) {
        Ok(summary) => {
            println!(
                "Backed up {} managed upload(s) ({} bytes) to {}.",
                summary.files,
                summary.total_bytes,
                output.display()
            );
            println!("Database contents are not included; back up the database separately while the application is quiescent.");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error[E-FILE-006]: upload backup failed: {error}");
            ExitCode::from(1)
        }
    }
}

fn files_restore_command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let mut entry = "main.zyl".to_owned();
    let mut entry_set = false;
    let mut input = None;
    let mut apply = false;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--from" => {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --from requires a backup directory path");
                    return ExitCode::from(2);
                };
                input = Some(PathBuf::from(value));
            }
            "--apply" => apply = true,
            value if value.starts_with('-') => {
                eprintln!("error[E-CLI-001]: unknown files restore option `{value}`");
                return ExitCode::from(2);
            }
            value if !entry_set => {
                entry = value.to_owned();
                entry_set = true;
            }
            value => {
                eprintln!("error[E-CLI-001]: unexpected files restore argument `{value}`");
                return ExitCode::from(2);
            }
        }
    }
    if !apply {
        eprintln!("error[E-CLI-001]: files restore changes private storage and requires --apply");
        return ExitCode::from(2);
    }
    let Some(input) = input else {
        eprintln!("error[E-CLI-001]: files restore requires --from <backup-directory>");
        return ExitCode::from(2);
    };
    let (program, database_url, project_directory) = match file_storage_context(&entry) {
        Ok(context) => context,
        Err(code) => return code,
    };
    let references = match load_file_references(&program, &database_url) {
        Ok(references) => references,
        Err(error) => {
            eprintln!("error[E-FILE-002]: could not verify upload references: {error}");
            return ExitCode::from(1);
        }
    };
    let storage = match zelyra_web::file_storage::FileStorage::new(
        project_directory.join("storage/zelyra"),
    ) {
        Ok(storage) => storage,
        Err(error) => {
            eprintln!("error[E-FILE-001]: private upload storage is unavailable: {error}");
            return ExitCode::from(1);
        }
    };
    match storage.restore_from_directory(&input, &references) {
        Ok(summary) => {
            println!(
                "Restored {} managed upload(s) ({} bytes) from {}.",
                summary.files,
                summary.total_bytes,
                input.display()
            );
            println!("The database is not part of this file-storage restore and must already be restored.");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error[E-FILE-006]: upload restore failed: {error}");
            ExitCode::from(1)
        }
    }
}

fn file_storage_context(entry: &str) -> Result<(Program, String, PathBuf), ExitCode> {
    let program = validate(entry).map_err(|()| ExitCode::from(1))?;
    let project_directory = Path::new(entry)
        .parent()
        .filter(|directory| !directory.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let database_name = program
        .databases
        .first()
        .map(|database| database.name.as_str());
    let database_url = match database_url_for_project(&project_directory, database_name) {
        Ok(Some(url)) => url,
        Ok(None) => {
            eprintln!("error[E-ENV-001]: DATABASE_URL is required to verify upload references");
            return Err(ExitCode::from(1));
        }
        Err(error) => {
            eprintln!("error[E-ENV-001]: {error}");
            return Err(ExitCode::from(1));
        }
    };
    Ok((program, database_url, project_directory))
}

fn load_file_references(
    program: &Program,
    database_url: &str,
) -> Result<HashSet<(i64, String)>, String> {
    const MAX_REFERENCE_ROWS: usize = 100_000;
    let aliases = program
        .types
        .iter()
        .map(|alias| (alias.name.as_str(), &alias.target))
        .collect::<HashMap<_, _>>();
    let is_postgres = zelyra_database::is_postgres_url(database_url);
    let mut references = HashSet::new();
    let mut scanned_rows = 0usize;
    for table in &program.tables {
        for column in &table.columns {
            if !is_file_ref_type(&column.ty, &aliases, &mut HashSet::new()) {
                continue;
            }
            let file_column = column
                .storage_name
                .as_deref()
                .unwrap_or(column.name.as_str());
            let tenant_column = table.tenant_scope.as_ref().map(|scope| {
                table
                    .columns
                    .iter()
                    .find(|candidate| candidate.name == scope.column)
                    .and_then(|candidate| candidate.storage_name.as_deref())
                    .unwrap_or(scope.column.as_str())
            });
            let quote = |identifier: &str| {
                if is_postgres {
                    format!("\"{}\"", identifier.replace('"', "\"\""))
                } else {
                    format!("`{}`", identifier.replace('`', "``"))
                }
            };
            let tenant_expression = tenant_column.map(quote).unwrap_or_else(|| "0".to_owned());
            let sql = format!(
                "SELECT {tenant_expression}, {} FROM {} WHERE {} IS NOT NULL LIMIT {}",
                quote(file_column),
                quote(&table.name),
                quote(file_column),
                MAX_REFERENCE_ROWS
                    .saturating_sub(scanned_rows)
                    .saturating_add(1)
            );
            let result = zelyra_database::execute_read_only_query(database_url, &sql, Vec::new())
                .map_err(|error| format!("{}.{}: {error}", table.name, column.name))?;
            if result.rows.len() > MAX_REFERENCE_ROWS.saturating_sub(scanned_rows) {
                return Err(format!(
                    "database FileRef scan exceeded {MAX_REFERENCE_ROWS} rows; cleanup aborted"
                ));
            }
            scanned_rows += result.rows.len();
            for row in result.rows {
                let tenant_id = row
                    .first()
                    .and_then(|value| value.parse::<i64>().ok())
                    .ok_or_else(|| {
                        format!(
                            "{}.{} returned an invalid tenant id",
                            table.name, column.name
                        )
                    })?;
                let file_ref = row
                    .get(1)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        format!(
                            "{}.{} returned an invalid file reference",
                            table.name, column.name
                        )
                    })?;
                references.insert((tenant_id, file_ref.clone()));
            }
        }
    }
    Ok(references)
}

fn is_file_ref_type(
    ty: &Type,
    aliases: &HashMap<&str, &Type>,
    visited: &mut HashSet<String>,
) -> bool {
    match ty {
        Type::Option(inner) => is_file_ref_type(inner, aliases, visited),
        Type::Named(name) if name == "FileRef" => true,
        Type::Named(name) if visited.insert(name.clone()) => aliases
            .get(name.as_str())
            .is_some_and(|target| is_file_ref_type(target, aliases, visited)),
        _ => false,
    }
}

fn usage() {
    eprintln!(
        "Usage:\n  zelyra files cleanup [entry.zyl] [--min-age-days <1..3650>] [--apply --maintenance-window]\n  zelyra files backup [entry.zyl] --output <new-directory>\n  zelyra files restore [entry.zyl] --from <backup-directory> --apply"
    );
}
