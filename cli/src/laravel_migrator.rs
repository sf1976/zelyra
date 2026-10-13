use super::*;
use regex::Regex;
use std::{
    ops::Range,
    path::{Component, Path},
};

const MAX_SOURCE_FILES: usize = 10_000;
const MAX_SOURCE_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_TOTAL_SOURCE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_DIRECTORY_DEPTH: usize = 64;
const ANALYZER_VERSION: &str = "1.0.0";
const MAPPING_RULESET_VERSION: &str = "1.0.0";

#[derive(Clone)]
struct Finding {
    kind: &'static str,
    status: &'static str,
    file: String,
    line: usize,
    name: String,
    summary: String,
    next_step: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct FindingCounts {
    total: usize,
    unresolved: usize,
    partial: usize,
}

#[derive(Debug)]
struct AnalysisOutput {
    report_path: PathBuf,
    counts: FindingCounts,
}

struct CommandOutput {
    report_path: Option<PathBuf>,
    scaffold_path: Option<PathBuf>,
    counts: FindingCounts,
}

pub(super) fn command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    if arguments.next().as_deref() != Some("laravel") {
        eprintln!("Usage: zelyra migrate laravel <project> [--report <path>] [--output <target>] [--format markdown|json]");
        return ExitCode::from(2);
    }
    let Some(project) = arguments.next() else {
        eprintln!("error[E-MIGRATE-001]: Laravel project directory is required");
        return ExitCode::from(2);
    };
    let mut report = None;
    let mut output = None;
    let mut format = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--report" => report = arguments.next(),
            "--output" => output = arguments.next(),
            "--format" => format = arguments.next(),
            _ => {
                eprintln!("error[E-MIGRATE-001]: unknown migrate option `{argument}`");
                return ExitCode::from(2);
            }
        }
    }
    if report.is_none() && output.is_none() {
        eprintln!("error[E-MIGRATE-001]: provide `--report <path>`, `--output <target>`, or both");
        return ExitCode::from(2);
    }
    let project = PathBuf::from(project);
    let report = report.map(PathBuf::from);
    let output = output.map(PathBuf::from);
    let format = match format.as_deref() {
        None => match report
            .as_ref()
            .and_then(|path| path.extension())
            .and_then(|extension| extension.to_str())
        {
            Some("json") => "json",
            _ => "markdown",
        },
        Some("json") => "json",
        Some("markdown") | Some("md") => "markdown",
        Some(other) => {
            eprintln!("error[E-MIGRATE-001]: unsupported report format `{other}`; choose `markdown` or `json`");
            return ExitCode::from(2);
        }
    };
    let result = if let Some(output) = output {
        generate_scaffold(&project, report.as_deref(), &output, format)
    } else {
        analyze(
            &project,
            report.as_deref().expect("report required without output"),
            format,
        )
        .map(|analysis| CommandOutput {
            report_path: Some(analysis.report_path),
            scaffold_path: None,
            counts: analysis.counts,
        })
    };
    match result {
        Ok(output) => {
            println!("Laravel source analyzed without executing PHP or reading environment files.");
            if let Some(path) = output.report_path {
                println!("Report: {}", path.display());
            }
            if let Some(path) = output.scaffold_path {
                println!("Separate Zelyra scaffold: {}", path.display());
                println!("Transferred business logic: none; review MIGRATION_TODO.md and MIGRATION_REPORT.md.");
            }
            println!(
                "Findings: {} · unresolved/manual: {} · partial: {}",
                output.counts.total, output.counts.unresolved, output.counts.partial
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error[E-MIGRATE-002]: {error}");
            ExitCode::from(1)
        }
    }
}

fn generate_scaffold(
    project: &Path,
    external_report: Option<&Path>,
    output: &Path,
    format: &str,
) -> Result<CommandOutput, String> {
    let root = fs::canonicalize(project)
        .map_err(|_| "Laravel project directory does not exist or is not readable".to_owned())?;
    if !root.is_dir() {
        return Err("Laravel project path must be a directory".into());
    }
    let output = resolve_new_report_path(output)?;
    let output_parent = output.parent().ok_or("target needs a parent directory")?;
    let canonical_parent = fs::canonicalize(output_parent)
        .map_err(|_| "target parent directory does not exist".to_owned())?;
    let output_name = output
        .file_name()
        .ok_or("target directory name is required")?;
    let target = canonical_parent.join(output_name);
    if target.starts_with(&root) || root.starts_with(&target) {
        return Err("scaffold target must be separate from the Laravel source project".into());
    }
    if fs::symlink_metadata(&target).is_ok() {
        return Err(
            "scaffold target already exists; choose a new directory so files are never overwritten"
                .into(),
        );
    }
    let report_target = if let Some(report) = external_report {
        let report = resolve_new_report_path(report)?;
        let parent = report.parent().ok_or("report needs a parent directory")?;
        let parent = fs::canonicalize(parent)
            .map_err(|_| "report parent directory does not exist".to_owned())?;
        let name = report.file_name().ok_or("report filename is required")?;
        let report_target = parent.join(name);
        if report_target.starts_with(&root) || root.starts_with(&report_target) {
            return Err("report path must be outside the Laravel source project".into());
        }
        if report_target.starts_with(&target) || target.starts_with(&report_target) {
            return Err("report path and scaffold target must be separate".into());
        }
        if fs::symlink_metadata(&report_target).is_ok() {
            return Err(
                "report already exists; choose a new path so reports are never overwritten".into(),
            );
        }
        Some((report_target, report))
    } else {
        None
    };

    let mut staging = None;
    for attempt in 0..16_u32 {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| "system clock is before the Unix epoch".to_owned())?
            .as_nanos();
        let path = canonical_parent.join(format!(
            ".zelyra-laravel-{}-{nonce}-{attempt}",
            std::process::id()
        ));
        match fs::create_dir(&path) {
            Ok(()) => {
                staging = Some(path);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "could not create private staging directory: {error}"
                ))
            }
        }
    }
    let staging = staging.ok_or("could not allocate a unique staging directory")?;
    let mut external_report_temporary = None;
    let result = (|| {
        let staged_report = staging.join("MIGRATION_REPORT.json");
        let analysis = analyze(&root, &staged_report, "json")?;
        let counts = analysis.counts;
        let report_json: Value = zelyra_ast::json::parse_slice(
            &fs::read(&staged_report)
                .map_err(|error| format!("could not read staged report: {error}"))?,
        )
        .map_err(|_| "generated migration report is invalid JSON".to_owned())?;
        let project_name = root
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| {
                name.chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
            })
            .unwrap_or("laravel-migration");
        write_new_file(
            &staging.join("zelyra.toml"),
            &format!(
                "[project]\nname = \"{}\"\nlanguage = \"0.1\"\n",
                project_name
            ),
        )?;
        write_new_file(
            &staging.join("main.zyl"),
            "# Generated scaffold: no Laravel business behavior was transferred.\nfn main() {\n    print(\"Review MIGRATION_TODO.md before implementing this application.\")\n}\n",
        )?;
        write_new_file(
            &staging.join("README.md"),
            "# Zelyra migration scaffold\n\nThis is a separate, compiler-checkable starting point. It contains no copied Laravel code, configuration, secrets, personal records, or production data. The analysis report is an inventory, not a claim of functional or security parity.\n\nStart with `zelyra check main.zyl`, then review `MIGRATION_TODO.md` and implement each behavior explicitly.\n",
        )?;
        let findings = report_json["findings"]
            .as_array()
            .ok_or("generated migration report has no findings list")?;
        let ruleset = report_json["mapping_ruleset"].as_str().unwrap_or("unknown");
        let mut report_markdown = format!(
            "# Laravel migration inventory\n\n- Mapping ruleset: `{ruleset}`\n- Source: `{}`\n- Analysis: static and read-only; PHP/Artisan and `.env` were not read.\n- Transferred business behavior: none.\n\n",
            report_json["source_project"].as_str().unwrap_or("Laravel project")
        );
        let mut todo = String::from(
            "# Migration work still required\n\nNo Laravel business logic has been transferred. Each mapping ID below identifies an analyzer finding, not a claim that the behavior is supported. Review the original source and reimplement and test authorization, tenant boundaries, validation, transactions, money calculations, file handling, jobs, integrations, and error paths before enabling any workflow.\n\n",
        );
        report_markdown.push_str("| Status | Mapping ID | Source location | Finding | Next step |\n|---|---|---|---|---|\n");
        for finding in findings {
            let kind = finding["kind"].as_str().unwrap_or("unknown");
            let id = format!("{ruleset}/{kind}");
            let file = finding["file"].as_str().unwrap_or("unknown");
            let line = finding["line"].as_u64().unwrap_or(0);
            let name = finding["name"].as_str().unwrap_or("unclassified finding");
            let summary = finding["summary"].as_str().unwrap_or("");
            let next_step = finding["next_step"].as_str().unwrap_or("Review manually.");
            let status = finding["status"].as_str().unwrap_or("unresolved");
            let safe_cell = |value: &str| value.replace('|', "\\|").replace(['\n', '\r'], " ");
            use std::fmt::Write as _;
            let _ = writeln!(
                report_markdown,
                "| {} | `{}` | `{}:{}` | {}: {} | {} |",
                safe_cell(status),
                id,
                safe_cell(file),
                line,
                safe_cell(name),
                safe_cell(summary),
                safe_cell(next_step)
            );
            let _ = writeln!(todo, "- **`{id}`** · `{file}:{line}` · {status}: {name}\n  - {summary}\n  - Next: {next_step}");
        }
        write_new_file(&staging.join("MIGRATION_REPORT.md"), &report_markdown)?;
        write_new_file(&staging.join("MIGRATION_TODO.md"), &todo)?;
        let main = staging.join("main.zyl");
        super::validate(
            main.to_str()
                .ok_or("staged source path is not valid UTF-8")?,
        )
        .map_err(|_| "generated scaffold failed the Zelyra compiler check".to_owned())?;

        let staged_external_report = if let Some((destination, requested_path)) =
            report_target.as_ref()
        {
            let extension = if format == "json" { "json" } else { "md" };
            let report_parent = requested_path
                .parent()
                .ok_or("report needs a parent directory")?;
            let report_parent = fs::canonicalize(report_parent)
                .map_err(|_| "report parent directory does not exist".to_owned())?;
            let mut temporary_report_directory = None;
            for _ in 0..16 {
                let candidate = report_parent.join(format!(
                    ".zelyra-laravel-report-{}-{}",
                    std::process::id(),
                    OsRng.next_u64()
                ));
                match create_private_directory(&candidate) {
                    Ok(()) => {
                        temporary_report_directory = Some(candidate);
                        break;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(error) => {
                        return Err(format!(
                            "could not create a private report staging directory: {error}"
                        ));
                    }
                }
            }
            let temporary_report_directory = temporary_report_directory
                .ok_or("could not allocate a temporary external report directory")?;
            external_report_temporary = Some(temporary_report_directory.clone());
            let temporary_report = temporary_report_directory.join(format!("report.{extension}"));
            let analysis = analyze(&root, &temporary_report, format)?;
            Some((destination.clone(), analysis.report_path))
        } else {
            None
        };
        super::module_cli::publish_directory_no_replace(&staging, &target).map_err(|error| {
            format!("could not publish scaffold without replacing an existing target: {error}")
        })?;
        let external_report_path = if let Some((destination, staged_report)) =
            staged_external_report
        {
            if let Err(error) = fs::hard_link(&staged_report, &destination) {
                let _ = fs::remove_dir_all(&target);
                return Err(format!(
                    "could not publish the external report without replacing an existing file: {error}"
                ));
            }
            if let Err(error) = fs::remove_file(&staged_report) {
                let _ = fs::remove_file(&destination);
                let _ = fs::remove_dir_all(&target);
                return Err(format!(
                    "could not finalize the scaffold report files: {error}"
                ));
            }
            if let Some(temporary_report_directory) = external_report_temporary.take() {
                if let Err(error) = fs::remove_dir_all(&temporary_report_directory) {
                    let _ = fs::remove_file(&destination);
                    let _ = fs::remove_dir_all(&target);
                    return Err(format!(
                        "could not remove the private report staging directory: {error}"
                    ));
                }
            }
            Some(destination)
        } else {
            None
        };
        Ok(CommandOutput {
            report_path: external_report_path,
            scaffold_path: Some(target.clone()),
            counts,
        })
    })();
    if staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }
    if let Some(temporary_report) = external_report_temporary {
        let _ = fs::remove_dir_all(temporary_report);
    }
    result
}

fn write_new_file(path: &Path, contents: &str) -> Result<(), String> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("could not create scaffold file: {error}"))?;
    if let Err(error) = file.write_all(contents.as_bytes()) {
        let _ = fs::remove_file(path);
        return Err(format!("could not write scaffold file: {error}"));
    }
    Ok(())
}

fn create_private_directory(path: &Path) -> std::io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}

fn analyze(project: &Path, report: &Path, format: &str) -> Result<AnalysisOutput, String> {
    let root = fs::canonicalize(project)
        .map_err(|_| "Laravel project directory does not exist or is not readable".to_owned())?;
    if !root.is_dir() {
        return Err("Laravel project path must be a directory".into());
    }
    let output = resolve_new_report_path(report)?;
    let output_parent = output.parent().ok_or("report needs a parent directory")?;
    let canonical_parent = fs::canonicalize(output_parent)
        .map_err(|_| "report parent directory does not exist".to_owned())?;
    let output_name = output.file_name().ok_or("report filename is required")?;
    let canonical_output = canonical_parent.join(output_name);
    if canonical_output.starts_with(&root) || root.starts_with(&canonical_output) {
        return Err("report path must be outside the Laravel source project".into());
    }
    if canonical_output.exists() {
        return Err(
            "report already exists; choose a new path so source reports are never overwritten"
                .into(),
        );
    }
    let mut files = Vec::new();
    collect_php_files(&root, &root, &mut files)?;
    files.sort();
    if files.is_empty() {
        return Err("no readable PHP source files were found outside excluded environment and dependency directories".into());
    }
    let total_source_bytes = files.iter().try_fold(0_u64, |total, (path, _)| {
        fs::metadata(path)
            .map(|metadata| total.saturating_add(metadata.len()))
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))
    })?;
    if total_source_bytes > MAX_TOTAL_SOURCE_BYTES {
        return Err("project exceeds the 256 MiB total PHP source analysis limit".into());
    }
    let mut findings = Vec::new();
    let composer = read_composer_metadata(&root)?;
    if let Some(version) = &composer.0 {
        findings.push(Finding { kind: "framework", status: "partial", file: "composer.json".into(), line: 1, name: version.clone(), summary: "Laravel version constraint found in Composer metadata; installed package resolution was not executed.".into(), next_step: "Confirm the installed framework and package versions in a controlled review." });
    }
    for (path, relative) in &files {
        let source = fs::read_to_string(path)
            .map_err(|_| format!("{} is not valid UTF-8 source", relative))?;
        let finding_count = findings.len();
        scan_source(relative, &source, &mut findings);
        if findings.len() == finding_count {
            findings.push(Finding {
                kind: "source_file",
                status: "unresolved",
                file: relative.clone(),
                line: 1,
                name: "no supported pattern recognized".into(),
                summary: "This PHP file contains syntax outside the analyzer's recognized pattern set; its behavior is not described by this report.".into(),
                next_step: "Review this source file for routes, business rules, authorization, or side effects that need manual mapping.",
            });
        }
    }
    if findings.is_empty() {
        findings.push(Finding {
            kind: "project",
            status: "unresolved",
            file: "—".into(),
            line: 0,
            name: "no recognized Laravel patterns".into(),
            summary: "No supported Laravel patterns were recognized; this does not prove that the project has no routes or business behavior.".into(),
            next_step: "Review project structure and add an analyzer fixture for any important unsupported pattern.",
        });
    }
    findings.sort_by(|left, right| {
        (&left.file, left.line, left.kind, &left.name).cmp(&(
            &right.file,
            right.line,
            right.kind,
            &right.name,
        ))
    });
    let unresolved = findings
        .iter()
        .filter(|finding| finding.status == "manual" || finding.status == "unresolved")
        .count();
    let partial = findings
        .iter()
        .filter(|finding| finding.status == "partial")
        .count();
    let rendered = if format == "json" {
        json_report(&root, &composer.0, &composer.1, &findings)
    } else {
        markdown_report(&root, &composer.0, &composer.1, &findings)
    };
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&canonical_output)
        .map_err(|error| format!("could not create report: {error}"))?;
    use std::io::Write;
    file.write_all(rendered.as_bytes()).map_err(|error| {
        let _ = fs::remove_file(&canonical_output);
        format!("could not write report: {error}")
    })?;
    Ok(AnalysisOutput {
        report_path: canonical_output,
        counts: FindingCounts {
            total: findings.len(),
            unresolved,
            partial,
        },
    })
}

fn resolve_new_report_path(report: &Path) -> Result<PathBuf, String> {
    let path = if report.is_absolute() {
        report.to_owned()
    } else {
        env::current_dir()
            .map_err(|error| error.to_string())?
            .join(report)
    };
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("report paths containing `..` are not accepted".into());
    }
    Ok(path)
}

fn collect_php_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(PathBuf, String)>,
) -> Result<(), String> {
    if files.len() > MAX_SOURCE_FILES {
        return Err("project exceeds the 10,000 PHP source file analysis limit".into());
    }
    let entries = fs::read_dir(directory).map_err(|error| {
        format!(
            "could not read project directory {}: {error}",
            directory.display()
        )
    })?;
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("could not read a project directory entry: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("could not inspect {}: {error}", entry.path().display()))?;
        if file_type.is_symlink() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let relative_path = path
            .strip_prefix(root)
            .map_err(|_| "source path escaped the project root".to_owned())?
            .to_string_lossy()
            .replace('\\', "/");
        if relative_path.split('/').count() > MAX_DIRECTORY_DEPTH {
            return Err("project directory nesting exceeds the 64-level analysis limit".into());
        }
        if name == ".env"
            || name.starts_with(".env.")
            || matches!(
                name.as_str(),
                ".git" | "vendor" | "node_modules" | "storage"
            )
            || relative_path == "bootstrap/cache"
            || relative_path.starts_with("bootstrap/cache/")
        {
            continue;
        }
        if file_type.is_dir() {
            collect_php_files(root, &path, files)?;
        } else if file_type.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("php"))
        {
            let metadata = fs::metadata(&path)
                .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
            if metadata.len() > MAX_SOURCE_FILE_BYTES {
                return Err(format!(
                    "{} exceeds the 2 MiB source file limit",
                    path.display()
                ));
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|_| "source path escaped the project root".to_owned())?
                .to_string_lossy()
                .replace('\\', "/");
            if relative
                .split('/')
                .any(|part| part == ".env" || part.starts_with(".env."))
            {
                continue;
            }
            files.push((path, relative));
        }
        if files.len() > MAX_SOURCE_FILES {
            return Err("project exceeds the 10,000 PHP source file analysis limit".into());
        }
    }
    Ok(())
}

fn read_composer_metadata(root: &Path) -> Result<(Option<String>, Vec<String>), String> {
    let path = root.join("composer.json");
    if !path.is_file() {
        return Ok((None, Vec::new()));
    }
    let source =
        fs::read_to_string(path).map_err(|_| "composer.json is not valid UTF-8".to_owned())?;
    if source.len() as u64 > MAX_SOURCE_FILE_BYTES {
        return Err("composer.json exceeds the 2 MiB file limit".into());
    }
    let value: Value = zelyra_ast::json::parse(&source)
        .map_err(|_| "composer.json is not valid JSON".to_owned())?;
    let dependencies = ["require", "require-dev"]
        .into_iter()
        .filter_map(|key| value.get(key).and_then(Value::as_object))
        .flat_map(|map| map.iter())
        .filter_map(|(name, version)| {
            let version = version.as_str()?;
            Some((name.clone(), version.to_owned()))
        })
        .collect::<Vec<_>>();
    let framework = dependencies
        .iter()
        .find(|(name, _)| name == "laravel/framework")
        .map(|(_, version)| version.clone());
    let relevant = dependencies
        .into_iter()
        .filter(|(name, _)| {
            name.starts_with("laravel/")
                || name.starts_with("illuminate/")
                || name.starts_with("spatie/")
                || name == "stripe/stripe-php"
                || name.starts_with("inertiajs/")
                || name.starts_with("livewire/")
        })
        .map(|(name, version)| format!("{name} {version}"))
        .collect();
    Ok((framework, relevant))
}

fn scan_source(file: &str, source: &str, findings: &mut Vec<Finding>) {
    let route = Regex::new(r#"Route\s*::\s*(get|post|put|patch|delete|options|any|resource|apiResource)\s*\(\s*['\"]([^'\"]+)['\"]"#).expect("static route regex");
    for capture in route.captures_iter(source) {
        add_match(findings, file, source, capture.get(0).unwrap().range(), "route", "partial", capture.get(2).unwrap().as_str(), format!("Laravel {} route; chained middleware, policy checks, and runtime route resolution need review.", capture.get(1).unwrap().as_str()), "Review middleware and authorization for this route, then map it to a Zelyra route.");
    }
    let model = Regex::new(
        r"(?m)\bclass\s+(\w+)\s+extends\s+(?:\\?Illuminate\\Database\\Eloquent\\)?Model\b",
    )
    .expect("static model regex");
    for capture in model.captures_iter(source) {
        add_match(
            findings,
            file,
            source,
            capture.get(0).unwrap().range(),
            "model",
            "partial",
            capture.get(1).unwrap().as_str(),
            "Eloquent model detected; casts, scopes, events, and traits are not inferred.",
            "Review casts, scopes, events, traits, and relationships before mapping the model.",
        );
    }
    let controller =
        Regex::new(r"(?m)\bclass\s+(\w*Controller)\b").expect("static controller regex");
    for capture in controller.captures_iter(source) {
        add_match(findings, file, source, capture.get(0).unwrap().range(), "controller", "manual", capture.get(1).unwrap().as_str(), "Controller behavior can include authorization, side effects, and branching that static analysis does not transfer.", "Review each action's business rules and security checks manually.");
    }
    let migration = Regex::new(r#"Schema\s*::\s*create\s*\(\s*['\"]([^'\"]+)['\"]"#)
        .expect("static migration regex");
    for capture in migration.captures_iter(source) {
        add_match(findings, file, source, capture.get(0).unwrap().range(), "migration", "partial", capture.get(1).unwrap().as_str(), "Schema::create table found; column modifiers, indexes, foreign keys, and rollback semantics require review.", "Compare the complete migration and existing data before generating a Zelyra schema.");
    }
    let relation = Regex::new(r"(?s)function\s+(\w+)\s*\([^)]*\)\s*\{\s*return\s+\$this\s*->\s*(belongsTo|hasOne|hasMany|belongsToMany|hasManyThrough)\s*\(").expect("static relationship regex");
    for capture in relation.captures_iter(source) {
        add_match(
            findings,
            file,
            source,
            capture.get(0).unwrap().range(),
            "relationship",
            "partial",
            capture.get(1).unwrap().as_str(),
            format!(
                "Eloquent {} relation found; keys and tenant constraints need review.",
                capture.get(2).unwrap().as_str()
            ),
            "Verify foreign-key names, delete behavior, and tenant ownership.",
        );
    }
    let validation = Regex::new(r"(?m)->validate\s*\(|Validator\s*::\s*make\s*\(")
        .expect("static validation regex");
    for matched in validation.find_iter(source) {
        add_match(findings, file, source, matched.range(), "validation", "manual", "validation rules", "Validation entry point found; individual rules and conditional behavior are not evaluated.", "Translate each rule into a Zelyra form/API contract and test invalid input.");
    }
    let queued = Regex::new(r"(?m)\bclass\s+(\w+)\s+implements\s+[^\n{]*ShouldQueue")
        .expect("static job regex");
    for capture in queued.captures_iter(source) {
        add_match(findings, file, source, capture.get(0).unwrap().range(), "job", "manual", capture.get(1).unwrap().as_str(), "Queued job found; retries, idempotency, transaction boundaries, and failure handling are not transferred.", "Review delivery guarantees and side effects before mapping the job.");
    }
    let authorization = Regex::new(
        r"(?i)(\$this\s*->\s*authorize\s*\(|\bGate\s*::\s*(?:authorize|allows|denies)\s*\(|\bcan\s*:\s*[A-Za-z0-9_.-]+|\bclass\s+\w+Policy\b)",
    )
    .expect("static authorization regex");
    for matched in authorization.find_iter(source) {
        add_match(
            findings,
            file,
            source,
            matched.range(),
            "authorization",
            "manual",
            "authorization or policy check",
            "Laravel authorization syntax found; this scan does not prove which records or actions the rule protects.",
            "Trace the authenticated principal, resource ownership, tenant boundary, and every create/read/update/delete path.",
        );
    }
    let tenant = Regex::new(r"(?i)\b(?:tenant_id|account_id|organization_id|workspace_id)\b")
        .expect("static tenant identifier regex");
    for matched in tenant.find_iter(source) {
        add_match(
            findings,
            file,
            source,
            matched.range(),
            "tenant_boundary",
            "manual",
            "tenant-scoped field",
            "A tenant-like identifier appears in source; the analyzer cannot establish that every query and relation is scoped to it.",
            "Review list, detail, mutation, relationship, and background-job access for cross-tenant ID manipulation.",
        );
    }
    let financial = Regex::new(
        r"(?i)(\b(?:subtotal|grand_total|total|tax|vat|price|amount|discount)\b\s*(?:=|\+=)|->\s*sum\s*\()",
    )
    .expect("static financial expression regex");
    for matched in financial.find_iter(source) {
        add_match(
            findings,
            file,
            source,
            matched.range(),
            "financial_rule",
            "manual",
            "money or total calculation",
            "A money-like field or aggregate is calculated; decimal precision, rounding, tax rules, and currency are not inferred.",
            "Specify currency, decimal precision, rounding mode, tax basis, and transaction boundaries, then add numeric acceptance tests.",
        );
    }
    let file_operation = Regex::new(
        r"(?i)(UploadedFile|response\s*\(\s*\)\s*->\s*download|->\s*(?:file|store|storeAs|download)\s*\()",
    )
    .expect("static file-operation regex");
    for matched in file_operation.find_iter(source) {
        add_match(
            findings,
            file,
            source,
            matched.range(),
            "file_access",
            "manual",
            "file upload or download",
            "A file upload/download operation appears; path confinement, content validation, authorization, and storage exposure are not analyzed.",
            "Review size and content limits, path traversal, tenant ownership, authorization, and download response headers.",
        );
    }
    let payment = Regex::new(
        r"(?i)(\b(?:charge|paymentMethod|checkout|subscribe|subscription|refund)\s*\(|Cashier\s*::|Stripe\s*::)",
    )
    .expect("static payment regex");
    for matched in payment.find_iter(source) {
        add_match(
            findings,
            file,
            source,
            matched.range(),
            "payment_rule",
            "manual",
            "payment or billing operation",
            "A payment, checkout, or billing operation appears; payment processor contracts, webhooks, idempotency keys, and refunds are not analyzed.",
            "Map payment flows to explicit external webhooks and verified transaction records with numeric precision.",
        );
    }
    let ambiguity = Regex::new(
        r"(?i)(DB\s*::\s*(?:raw|selectRaw|whereRaw|havingRaw|orderByRaw)\s*\(|eval\s*\(|call_user_func(?:_array)?\s*\(|->\s*action\s*\(\s*\$)",
    )
    .expect("static ambiguity regex");
    for matched in ambiguity.find_iter(source) {
        add_match(
            findings,
            file,
            source,
            matched.range(),
            "ambiguity",
            "manual",
            "dynamic or unparsed SQL/execution ambiguity",
            "Dynamic dispatch, raw SQL expression, or runtime execution found; static type safety and dialect compatibility cannot be verified.",
            "Replace raw SQL or dynamic dispatch with structured Zelyra typed queries and explicit routes.",
        );
    }
    let env_call =
        Regex::new(r#"(?i)\benv\s*\(\s*['"]([^'"]+)['"]"#).expect("static env call regex");
    for capture in env_call.captures_iter(source) {
        let var_name = capture.get(1).unwrap().as_str();
        add_match(
            findings,
            file,
            source,
            capture.get(0).unwrap().range(),
            "environment_access",
            "manual",
            format!("env('{var_name}') access"),
            "Direct env() access in source code; environment variables outside config files are fragile and values are never read or copied into reports.",
            "Define explicit Zelyra configuration or module capabilities instead of runtime env lookups.",
        );
    }
    for (needle, kind, label) in [
        ("Mail::", "integration", "mail integration"),
        ("Notification::", "integration", "notification integration"),
        ("Http::", "integration", "HTTP integration"),
        ("Storage::", "integration", "file storage integration"),
        ("DB::transaction", "transaction", "database transaction"),
    ] {
        let mut offset = 0;
        while let Some(index) = source[offset..].find(needle) {
            let start = offset + index;
            let matched = start..start + needle.len();
            add_match(findings, file, source, matched, kind, "manual", label, format!("{label} call found; credentials, retries, transactions, and error handling require review."), "Identify external effects and their failure/rollback behavior.");
            offset = start + needle.len();
        }
    }
    if file.starts_with("resources/views/") && file.ends_with(".blade.php") {
        findings.push(Finding { kind: "view", status: "manual", file: file.into(), line: 1, name: file.rsplit('/').next().unwrap_or(file).into(), summary: "Blade view found; directives, escaping, components, and client behavior are not translated.".into(), next_step: "Review rendering, output escaping, forms, and JavaScript behavior." });
    }
}

#[allow(clippy::too_many_arguments)]
fn add_match(
    findings: &mut Vec<Finding>,
    file: &str,
    source: &str,
    matched: Range<usize>,
    kind: &'static str,
    status: &'static str,
    name: impl Into<String>,
    summary: impl Into<String>,
    next_step: &'static str,
) {
    let line = source[..matched.start.min(source.len())]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1;
    findings.push(Finding {
        kind,
        status,
        file: file.into(),
        line,
        name: name.into(),
        summary: summary.into(),
        next_step,
    });
}

fn json_report(
    root: &Path,
    framework: &Option<String>,
    packages: &[String],
    findings: &[Finding],
) -> String {
    let counts = report_counts(findings);
    let findings = findings.iter().map(|finding| json!({"kind": finding.kind, "status": finding.status, "file": finding.file, "line": finding.line, "name": finding.name, "summary": finding.summary, "next_step": finding.next_step})).collect::<Vec<_>>();
    serde_json::to_string_pretty(&json!({"schema_version": "1", "analyzer": format!("Zelyra Laravel Migrator {ANALYZER_VERSION}"), "mapping_ruleset": MAPPING_RULESET_VERSION, "source_project": root.file_name().and_then(|name| name.to_str()).unwrap_or("Laravel project"), "laravel_constraint": framework, "laravel_packages": packages, "execution": "static_read_only", "env_files_read": false, "summary": counts, "findings": findings})).expect("report JSON is serializable")
}

fn markdown_report(
    root: &Path,
    framework: &Option<String>,
    packages: &[String],
    findings: &[Finding],
) -> String {
    let counts = report_counts(findings);
    let mut report = format!("# Zelyra Laravel migration analysis\n\n- Analyzer: Zelyra Laravel Migrator {ANALYZER_VERSION}\n- Mapping ruleset: {MAPPING_RULESET_VERSION}\n- Project: {}\n- Laravel constraint: {}\n- Laravel packages: {}\n- Method: static, read-only source scan; PHP and Artisan were not run; `.env` files were not read.\n- Findings: {} total · {} partial · {} manual · {} unresolved\n\n", root.file_name().and_then(|name| name.to_str()).unwrap_or("Laravel project"), framework.as_deref().unwrap_or("not detected"), if packages.is_empty() { "none detected".into() } else { packages.join(", ") }, findings.len(), counts["partial"], counts["manual"], counts["unresolved"]);
    report.push_str("| Status | Kind | Source location | Finding |\n|---|---|---|---|\n");
    for finding in findings {
        let summary = finding.summary.replace('|', "\\|").replace('\n', " ");
        let _ = writeln!(
            report,
            "| {} | {} | `{}:{}` | {} |",
            finding.status, finding.kind, finding.file, finding.line, summary
        );
        let _ = writeln!(report, "|  | Next step |  | {} |", finding.next_step);
    }
    if findings.is_empty() {
        report.push_str("| unresolved | project | — | No supported Laravel patterns were recognized. This does not prove that the project has no routes or business behavior. |\n");
    }
    report.push_str("\n## Scope limits\n\nThis report inventories recognized syntax. It does not prove feature or security parity. Unrecognized syntax and dynamic behavior remain outside the report. Review authorization, tenant boundaries, transactions, money, file handling, and integration side effects before implementing the mapped behavior in Zelyra. The source project and its database were not changed.\n");
    report
}

fn report_counts(findings: &[Finding]) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::from([
        ("direct", 0),
        ("partial", 0),
        ("manual", 0),
        ("unresolved", 0),
    ]);
    for finding in findings {
        if let Some(count) = counts.get_mut(finding.status) {
            *count += 1;
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzer_ignores_env_and_writes_report_outside_source() {
        let root = env::temp_dir().join(format!(
            "zelyra-laravel-{}-{}",
            std::process::id(),
            rand_core::OsRng.next_u64()
        ));
        fs::create_dir_all(root.join("routes")).unwrap();
        fs::create_dir_all(root.join("vendor")).unwrap();
        fs::write(root.join(".env"), "APP_KEY=must-not-be-read-or-reported").unwrap();
        fs::write(
            root.join("routes/web.php"),
            "<?php\nRoute::get('/orders', [OrderController::class, 'index']);\n",
        )
        .unwrap();
        fs::write(
            root.join("vendor/ignored.php"),
            "Route::get('/secret', fn () => null);",
        )
        .unwrap();
        let report = env::temp_dir().join(format!(
            "zelyra-laravel-report-{}-{}.json",
            std::process::id(),
            rand_core::OsRng.next_u64()
        ));
        let analysis = analyze(&root, &report, "json").unwrap();
        let body = fs::read_to_string(&analysis.report_path).unwrap();
        assert!(body.contains("/orders"));
        assert!(body.contains("Zelyra Laravel Migrator 1.0.0"));
        assert!(body.contains("\"mapping_ruleset\": \"1.0.0\""));
        assert!(!body.contains("must-not-be-read-or-reported"));
        assert!(!body.contains("/secret"));
        assert!(analysis.counts.total >= 1);
        assert_eq!(
            fs::read_to_string(root.join(".env")).unwrap(),
            "APP_KEY=must-not-be-read-or-reported"
        );
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(analysis.report_path);
    }

    #[test]
    fn analyzer_rejects_report_inside_source_and_existing_report() {
        let root = env::temp_dir().join(format!(
            "zelyra-laravel-{}-{}",
            std::process::id(),
            rand_core::OsRng.next_u64()
        ));
        fs::create_dir_all(&root).unwrap();
        let inside = root.join("report.json");
        assert!(analyze(&root, &inside, "json")
            .unwrap_err()
            .contains("outside"));
        let outside = env::temp_dir().join(format!(
            "zelyra-laravel-report-{}-{}.md",
            std::process::id(),
            rand_core::OsRng.next_u64()
        ));
        fs::write(&outside, "existing").unwrap();
        assert!(analyze(&root, &outside, "markdown")
            .unwrap_err()
            .contains("already exists"));
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(outside);
    }

    #[test]
    fn analyzer_labels_business_and_security_work_for_review() {
        let source = r#"<?php
class Customer extends Model {
    function invoices() { return $this->hasMany(Invoice::class); }
}
class CustomerController {}
Schema::create('customers', function ($table) {});
class SendInvoice implements ShouldQueue {}
class InvoicePolicy {}
Gate::authorize('update', $invoice);
$invoice->tenant_id = $tenantId;
$total = $invoice->price * $invoice->quantity;
$path = $request->file('attachment')->store('invoices');
$request->validate([]);
return $this->hasMany(Invoice::class);
Mail::to($email)->send($message);
$customer->charge(5000);
DB::raw("NOW()");
$key = env('STRIPE_SECRET');
"#;
        let mut findings = Vec::new();
        scan_source("app/Customer.php", source, &mut findings);
        assert!(findings
            .iter()
            .any(|item| item.kind == "model" && item.status == "partial"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "controller" && item.status == "manual"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "migration" && item.name == "customers"));
        assert!(findings.iter().any(|item| {
            item.kind == "relationship"
                && item.name == "invoices"
                && item.summary.contains("hasMany")
        }));
        assert!(findings
            .iter()
            .any(|item| item.kind == "validation" && item.status == "manual"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "job" && item.name == "SendInvoice"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "authorization" && item.status == "manual"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "tenant_boundary" && item.status == "manual"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "financial_rule" && item.status == "manual"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "file_access" && item.status == "manual"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "payment_rule" && item.status == "manual"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "ambiguity" && item.status == "manual"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "environment_access" && item.status == "manual"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "integration" && item.name == "mail integration"));
    }

    #[test]
    fn analyzer_reports_unrecognized_php_as_unresolved() {
        let root = env::temp_dir().join(format!(
            "zelyra-laravel-{}-{}",
            std::process::id(),
            rand_core::OsRng.next_u64()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("custom.php"),
            "<?php function custom_pipeline() {}\n",
        )
        .unwrap();
        let report = env::temp_dir().join(format!(
            "zelyra-laravel-report-{}-{}.json",
            std::process::id(),
            rand_core::OsRng.next_u64()
        ));
        let analysis = analyze(&root, &report, "json").unwrap();
        let value: Value =
            serde_json::from_str(&fs::read_to_string(&analysis.report_path).unwrap()).unwrap();
        assert_eq!(value["summary"]["unresolved"], 1);
        assert_eq!(analysis.counts.total, 1);
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(analysis.report_path);
    }
}
