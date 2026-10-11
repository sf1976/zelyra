use regex::Regex;
use sha2::{Digest, Sha256};
use similar::TextDiff;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
    process::ExitCode,
};

const MAX_PLAN_BYTES: u64 = 256 * 1024;
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_FILES: usize = 500;

struct Change {
    name: String,
    files: Vec<String>,
    find: String,
    replace: String,
    expect: usize,
}

pub(super) fn command(mut args: impl Iterator<Item = String>) -> ExitCode {
    if args.next().as_deref() != Some("edit") {
        eprintln!("error[E-TEXT-001]: expected `zelyra text edit <plan.zedit> [--apply --plan-id sha256:<id>]`");
        return ExitCode::from(2);
    }
    let mut plan_path = None;
    let mut apply = false;
    let mut plan_id = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--apply" if !apply => apply = true,
            "--plan-id" if plan_id.is_none() => {
                let Some(value) = args.next().filter(|value| !value.starts_with('-')) else {
                    eprintln!("error[E-TEXT-001]: `--plan-id` requires the SHA-256 ID printed by the preview");
                    return ExitCode::from(2);
                };
                plan_id = Some(value);
            }
            value if value.starts_with('-') => {
                eprintln!("error[E-TEXT-001]: unknown or repeated option `{value}`");
                return ExitCode::from(2);
            }
            value if plan_path.is_none() => plan_path = Some(value.to_owned()),
            _ => {
                eprintln!("error[E-TEXT-001]: provide exactly one .zedit plan");
                return ExitCode::from(2);
            }
        }
    }
    if apply != plan_id.is_some() {
        eprintln!("error[E-TEXT-001]: preview is the default; applying requires both `--apply` and the previewed `--plan-id`");
        return ExitCode::from(2);
    }
    let Some(plan_path) = plan_path else {
        eprintln!("error[E-TEXT-001]: provide a .zedit plan file");
        return ExitCode::from(2);
    };
    match run(&plan_path, apply, plan_id.as_deref()) {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error[E-TEXT-002]: {error}");
            ExitCode::from(1)
        }
    }
}

fn run(plan_path: &str, apply: bool, supplied_id: Option<&str>) -> Result<String, String> {
    let path = Path::new(plan_path);
    if path.extension().and_then(|extension| extension.to_str()) != Some("zedit") {
        return Err("text edit plans must use the `.zedit` extension".into());
    }
    let absolute_plan = fs::canonicalize(path)
        .map_err(|error| format!("cannot resolve plan `{plan_path}`: {error}"))?;
    let root = find_project_root(
        absolute_plan
            .parent()
            .ok_or("plan has no parent directory")?,
    )?;
    if !absolute_plan.starts_with(&root) {
        return Err("plan file is outside the project root".into());
    }
    let metadata = fs::metadata(&absolute_plan).map_err(|error| error.to_string())?;
    if metadata.len() > MAX_PLAN_BYTES {
        return Err("plan exceeds the 256 KiB size limit".into());
    }
    let plan_source = fs::read_to_string(&absolute_plan)
        .map_err(|error| format!("plan must be readable UTF-8 text: {error}"))?;
    let changes = parse_plan(&plan_source)?;
    let mut files = BTreeMap::<PathBuf, String>::new();
    let mut change_targets = Vec::with_capacity(changes.len());
    let mut claimed_targets = BTreeSet::new();
    for change in &changes {
        let mut selected = BTreeSet::new();
        for pattern in &change.files {
            validate_pattern(pattern)?;
            let full_pattern = root.join(pattern).to_string_lossy().into_owned();
            let entries = glob::glob(&full_pattern)
                .map_err(|error| format!("invalid file pattern `{pattern}`: {error}"))?;
            for entry in entries {
                let candidate =
                    entry.map_err(|error| format!("cannot enumerate `{pattern}`: {error}"))?;
                let metadata = fs::symlink_metadata(&candidate)
                    .map_err(|error| format!("cannot inspect matched path: {error}"))?;
                if metadata.file_type().is_symlink() {
                    return Err(format!("matched path `{pattern}` is a symbolic link; edit plans do not follow symbolic links"));
                }
                if !metadata.is_file() || excluded_path(&candidate, &root) {
                    continue;
                }
                let canonical = fs::canonicalize(&candidate)
                    .map_err(|error| format!("cannot resolve matched file: {error}"))?;
                if !canonical.starts_with(&root) {
                    return Err(format!("matched file `{pattern}` escapes the project root"));
                }
                selected.insert(canonical);
            }
        }
        if selected.is_empty() {
            return Err(format!("change `{}` matched no project files", change.name));
        }
        if selected.iter().any(|path| claimed_targets.contains(path)) {
            return Err(format!("change `{}` overlaps files selected by another change; use one rule per file in a plan", change.name));
        }
        claimed_targets.extend(selected.iter().cloned());
        change_targets.push(selected.clone());
        for file in selected {
            if !files.contains_key(&file) {
                if files.len() >= MAX_FILES {
                    return Err("plan matches more than 500 files".into());
                }
                let metadata = fs::metadata(&file).map_err(|error| error.to_string())?;
                if metadata.len() > MAX_FILE_BYTES {
                    return Err(format!(
                        "matched file `{}` exceeds the 2 MiB size limit",
                        file.display()
                    ));
                }
                let source = fs::read_to_string(&file).map_err(|error| {
                    format!(
                        "matched file `{}` must be UTF-8 text: {error}",
                        file.display()
                    )
                })?;
                files.insert(file, source);
            }
        }
    }

    let original_files = files.clone();
    let mut summary = Vec::new();
    for (change, targets) in changes.iter().zip(&change_targets) {
        let expression = Regex::new(&change.find).map_err(|error| {
            format!(
                "change `{}` has an invalid or unsupported regular expression: {error}",
                change.name
            )
        })?;
        let mut count = 0usize;
        for path in targets {
            let source = files.get_mut(path).expect("selected file was loaded");
            count += expression.captures_iter(source).count();
            *source = expression
                .replace_all(source, change.replace.as_str())
                .into_owned();
        }
        if count != change.expect {
            return Err(format!("change `{}` expected {} matches across selected files but found {count}; no files were changed", change.name, change.expect));
        }
        summary.push(format!("- {}: {count} Treffer", change.name));
    }

    let digest = plan_digest(&plan_source, &original_files);
    let computed_id = format!("sha256:{digest}");
    if apply && supplied_id != Some(computed_id.as_str()) {
        return Err(format!(
            "plan ID does not match current inputs; review the new preview (`{computed_id}`)"
        ));
    }
    let relative_paths = files
        .keys()
        .map(|path| {
            path.strip_prefix(&root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<Vec<_>>();
    let mut output = format!(
        "Zelyra Text Edit\nPlan: {computed_id}\nDateien: {}\n{}\n",
        relative_paths.len(),
        summary.join("\n")
    );
    for (path, source) in &files {
        let old = &original_files[path];
        if old != source {
            let relative = path.strip_prefix(&root).unwrap_or(path).display();
            output.push_str(&format!(
                "\n--- Vorschau: {relative} ({} → {} Bytes)\n",
                old.len(),
                source.len()
            ));
            output.push_str(&simple_diff(old, source));
        }
    }
    if apply {
        replace_files(&original_files, &files)?;
        output.push_str("\nGeprüfter Plan wurde angewendet.\n");
    } else {
        output.push_str("\nVorschau: keine Datei geändert. Zum Anwenden die angezeigte Plan-ID mit --apply --plan-id übergeben.\n");
    }
    Ok(output)
}

fn parse_plan(source: &str) -> Result<Vec<Change>, String> {
    let document: toml::Value =
        toml::from_str(source).map_err(|error| format!("invalid TOML plan: {error}"))?;
    let rows = document
        .get("change")
        .and_then(toml::Value::as_array)
        .ok_or("plan needs one or more [[change]] sections")?;
    if rows.is_empty() || rows.len() > 100 {
        return Err("plan must contain between 1 and 100 changes".into());
    }
    rows.iter()
        .map(|row| {
            let field = |name: &str| {
                row.get(name)
                    .ok_or_else(|| format!("change is missing `{name}`"))
            };
            let name = field("name")?
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .ok_or("change `name` must be non-empty text")?
                .to_owned();
            let files = field("files")?
                .as_array()
                .ok_or("change `files` must be an array of project-relative patterns")?
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .map(str::to_owned)
                        .ok_or("each file pattern must be text".to_owned())
                })
                .collect::<Result<Vec<_>, _>>()?;
            if files.is_empty() || files.len() > 100 {
                return Err(format!(
                    "change `{name}` must have between 1 and 100 file patterns"
                ));
            }
            let find = field("find")?
                .as_str()
                .filter(|value| !value.is_empty())
                .ok_or("change `find` must be a non-empty regex")?
                .to_owned();
            let replace = field("replace")?
                .as_str()
                .ok_or("change `replace` must be text")?
                .to_owned();
            let expect = field("expect")?
                .as_integer()
                .filter(|value| *value > 0 && *value <= 1_000_000)
                .ok_or("change `expect` must be an integer from 1 to 1,000,000")?
                as usize;
            Ok(Change {
                name,
                files,
                find,
                replace,
                expect,
            })
        })
        .collect()
}

fn validate_pattern(pattern: &str) -> Result<(), String> {
    let path = Path::new(pattern);
    if pattern.trim().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "file pattern `{pattern}` must stay within the project root"
        ));
    }
    if pattern.contains('\\')
        || pattern.contains('[')
        || pattern.contains(']')
        || pattern.contains('{')
        || pattern.contains('}')
    {
        return Err(format!(
            "file pattern `{pattern}` uses unsupported syntax; use `*`, `?`, or `**`"
        ));
    }
    Ok(())
}

fn find_project_root(from: &Path) -> Result<PathBuf, String> {
    let mut directory = from;
    loop {
        if directory.join("zelyra.toml").is_file() {
            return fs::canonicalize(directory).map_err(|error| error.to_string());
        }
        directory = directory
            .parent()
            .ok_or(".zedit plan is not inside a Zelyra project")?;
    }
}

fn excluded_path(path: &Path, root: &Path) -> bool {
    path.strip_prefix(root).ok().is_none_or(|relative| relative.components().any(|part| {
        matches!(part, Component::Normal(name) if name == ".git" || name == "target" || name == "node_modules")
    }))
}

fn plan_digest(plan: &str, files: &BTreeMap<PathBuf, String>) -> String {
    let mut hash = Sha256::new();
    hash.update(plan.as_bytes());
    for (path, source) in files {
        hash.update((path.as_os_str().to_string_lossy().len() as u64).to_be_bytes());
        hash.update(path.as_os_str().to_string_lossy().as_bytes());
        hash.update((source.len() as u64).to_be_bytes());
        hash.update(source.as_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn simple_diff(old: &str, new: &str) -> String {
    TextDiff::from_lines(old, new)
        .unified_diff()
        .context_radius(3)
        .to_string()
}

fn replace_files(
    originals: &BTreeMap<PathBuf, String>,
    files: &BTreeMap<PathBuf, String>,
) -> Result<(), String> {
    let pid = std::process::id();
    let mut staged = Vec::new();
    for (index, (target, content)) in files.iter().enumerate() {
        if fs::read_to_string(target).map_err(|error| {
            format!("cannot reread `{}` before apply: {error}", target.display())
        })? != originals[target]
        {
            for (_, pending, _) in &staged {
                let _ = fs::remove_file(pending);
            }
            return Err(format!(
                "`{}` changed after the preview; create a new preview before applying",
                target.display()
            ));
        }
        if originals[target] == *content {
            continue;
        }
        let parent = target.parent().ok_or("target has no parent")?;
        let file_name = target
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("target has no valid file name")?;
        let temporary = parent.join(format!(".{file_name}.zelyra-text-{pid}-{index}.tmp"));
        let backup = parent.join(format!(".{file_name}.zelyra-text-{pid}-{index}.bak"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("cannot stage `{}`: {error}", target.display()))?;
        file.write_all(content.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|error| {
                let _ = fs::remove_file(&temporary);
                format!("cannot stage `{}`: {error}", target.display())
            })?;
        let permissions = fs::metadata(target)
            .map_err(|error| error.to_string())?
            .permissions();
        fs::set_permissions(&temporary, permissions).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            error.to_string()
        })?;
        staged.push((target.clone(), temporary, backup));
    }
    let mut applied = Vec::new();
    for (target, temporary, backup) in &staged {
        let current = match fs::read_to_string(target) {
            Ok(current) => current,
            Err(error) => {
                for (applied_target, _, applied_backup) in applied.iter().rev() {
                    let _ = fs::rename(applied_backup, applied_target);
                }
                for (_, pending, pending_backup) in &staged {
                    let _ = fs::remove_file(pending);
                    let _ = fs::remove_file(pending_backup);
                }
                return Err(format!(
                    "cannot reread `{}` before apply: {error}",
                    target.display()
                ));
            }
        };
        if current != originals[target] {
            for (applied_target, _, applied_backup) in applied.iter().rev() {
                let _ = fs::rename(applied_backup, applied_target);
            }
            for (_, pending, pending_backup) in &staged {
                let _ = fs::remove_file(pending);
                let _ = fs::remove_file(pending_backup);
            }
            return Err(format!(
                "`{}` changed immediately before apply; create a new preview",
                target.display()
            ));
        }
        if let Err(error) = fs::copy(target, backup).and_then(|_| fs::rename(temporary, target)) {
            for (applied_target, _, applied_backup) in applied.iter().rev() {
                let _ = fs::rename(applied_backup, applied_target);
            }
            for (_, pending, pending_backup) in &staged {
                let _ = fs::remove_file(pending);
                let _ = fs::remove_file(pending_backup);
            }
            return Err(format!(
                "could not atomically apply `{}`; completed files were restored: {error}",
                target.display()
            ));
        }
        applied.push((target.clone(), temporary.clone(), backup.clone()));
    }
    for (_, _, backup) in &staged {
        let _ = fs::remove_file(backup);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_named_regex_plan_and_rejects_path_escape() {
        let plan = r#"[[change]]
name = "Reformat names"
files = ["src/**/*.zyl"]
find = '''(?P<first>\w+) (?P<last>\w+)'''
replace = "${last}, ${first}"
expect = 2
"#;
        let changes = parse_plan(plan).expect("valid plan");
        assert_eq!(changes[0].expect, 2);
        assert!(validate_pattern("../outside.zyl").is_err());
    }

    #[test]
    #[allow(clippy::invalid_regex)]
    fn regex_replacement_uses_named_groups_and_linear_engine() {
        let expression = Regex::new(r"(?P<first>\w+) (?P<last>\w+)").unwrap();
        assert_eq!(
            expression.replace_all("Ada Lovelace", "${last}, ${first}"),
            "Lovelace, Ada"
        );
        assert!(Regex::new(r"(a)\1").is_err());
        assert!(Regex::new(r"(?=a)a").is_err());
    }

    #[test]
    fn digest_changes_when_input_changes() {
        let a = BTreeMap::from([(PathBuf::from("a.zyl"), "one".to_owned())]);
        let b = BTreeMap::from([(PathBuf::from("a.zyl"), "two".to_owned())]);
        assert_ne!(plan_digest("plan", &a), plan_digest("plan", &b));
    }

    #[test]
    fn preview_is_read_only_and_apply_requires_the_current_plan_id() {
        let directory = std::env::temp_dir().join(format!(
            "zelyra-text-edit-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(directory.join("src")).unwrap();
        fs::write(directory.join("zelyra.toml"), "name = \"fixture\"\n").unwrap();
        fs::write(directory.join("src/main.zyl"), "name = \"Ada Lovelace\"\n").unwrap();
        fs::write(
            directory.join("rename.zedit"),
            r#"[[change]]
name = "Reverse display name"
files = ["src/**/*.zyl"]
find = '''(?P<first>\w+) (?P<last>\w+)'''
replace = "${last}, ${first}"
expect = 1
"#,
        )
        .unwrap();
        let plan = directory
            .join("rename.zedit")
            .to_string_lossy()
            .into_owned();
        let preview = run(&plan, false, None).unwrap();
        assert!(preview.contains("Vorschau"));
        let source_path = directory.join("src/main.zyl");
        assert_eq!(
            fs::read_to_string(&source_path).unwrap(),
            "name = \"Ada Lovelace\"\n"
        );
        let stale_id = preview
            .lines()
            .find_map(|line| line.strip_prefix("Plan: "))
            .unwrap();
        fs::write(&source_path, "name = \"Ada Lovelace\"\n// changed\n").unwrap();
        let stale_error = run(&plan, true, Some(stale_id)).unwrap_err();
        assert!(
            stale_error.contains("plan ID does not match"),
            "{stale_error}"
        );
        let refreshed = run(&plan, false, None).unwrap();
        let id = refreshed
            .lines()
            .find_map(|line| line.strip_prefix("Plan: "))
            .unwrap();
        run(&plan, true, Some(id)).unwrap();
        assert_eq!(
            fs::read_to_string(&source_path).unwrap(),
            "name = \"Lovelace, Ada\"\n// changed\n"
        );
        let _ = fs::remove_dir_all(directory);
    }
}
