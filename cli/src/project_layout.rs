use std::{fs, path::PathBuf};

use toml_edit::DocumentMut;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LayoutError {
    pub(crate) path: String,
    pub(crate) code: &'static str,
    pub(crate) message: String,
    pub(crate) line: usize,
    pub(crate) column: usize,
}

const STANDARD_DIRECTORIES: [&str; 5] = ["schema", "domain", "routes", "public", "assets"];

pub(crate) fn validate(entry_path: &str) -> Result<(), LayoutError> {
    let entry = fs::canonicalize(entry_path).map_err(|_| LayoutError {
        path: entry_path.to_owned(),
        code: "E-LAYOUT-001",
        message: "cannot resolve the project entry file".into(),
        line: 1,
        column: 1,
    })?;
    let Some(config_path) = find_config(entry.parent().unwrap_or(&entry)) else {
        return Ok(());
    };
    if fs::symlink_metadata(&config_path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(LayoutError {
            path: config_path.display().to_string(),
            code: "E-LAYOUT-001",
            message: "project configuration must not be a symbolic link".into(),
            line: 1,
            column: 1,
        });
    }
    let config_source = fs::read_to_string(&config_path).map_err(|_| LayoutError {
        path: config_path.display().to_string(),
        code: "E-LAYOUT-001",
        message: "cannot read the project configuration".into(),
        line: 1,
        column: 1,
    })?;
    let document = config_source.parse::<DocumentMut>().map_err(|error| {
        span_error(
            &config_path,
            &config_source,
            error.to_string(),
            error.span(),
        )
    })?;
    let project = document.get("project").and_then(toml_edit::Item::as_table);
    let Some(project) = project else {
        return Ok(());
    };
    let layout = project.get("layout");
    let Some(layout) = layout else {
        return Ok(());
    };
    let strict_item = project.get("strict_architecture");
    if let Some(item) = strict_item {
        if item.as_bool().is_none() {
            return Err(span_error(
                &config_path,
                &config_source,
                "`project.strict_architecture` must be true or false".into(),
                item.span(),
            ));
        }
    }
    let Some(profile) = layout.as_str() else {
        return Err(span_error(
            &config_path,
            &config_source,
            "`project.layout` must be the string `standard`".into(),
            layout.span(),
        ));
    };
    if profile != "standard" {
        return Err(span_error(
            &config_path,
            &config_source,
            format!("unknown project layout `{profile}`; expected `standard`"),
            layout.span(),
        ));
    }
    let root = config_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let canonical_root = fs::canonicalize(root).map_err(|_| LayoutError {
        path: config_path.display().to_string(),
        code: "E-LAYOUT-001",
        message: "cannot resolve the project root".into(),
        line: 1,
        column: 1,
    })?;
    for directory in STANDARD_DIRECTORIES {
        let path = root.join(directory);
        let metadata = fs::symlink_metadata(&path).map_err(|error| LayoutError {
            path: config_path.display().to_string(),
            code: "E-LAYOUT-002",
            message: if error.kind() == std::io::ErrorKind::NotFound {
                format!("standard layout requires the `{directory}/` directory")
            } else {
                format!("cannot inspect standard layout directory `{directory}/`")
            },
            line: layout
                .span()
                .map_or(1, |span| line_column(&config_source, span.start).0),
            column: layout
                .span()
                .map_or(1, |span| line_column(&config_source, span.start).1),
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(LayoutError {
                path: config_path.display().to_string(),
                code: "E-LAYOUT-002",
                message: format!("standard layout path `{directory}/` must be a real directory"),
                line: layout
                    .span()
                    .map_or(1, |span| line_column(&config_source, span.start).0),
                column: layout
                    .span()
                    .map_or(1, |span| line_column(&config_source, span.start).1),
            });
        }
        let canonical_directory = fs::canonicalize(&path).map_err(|_| LayoutError {
            path: config_path.display().to_string(),
            code: "E-LAYOUT-002",
            message: format!("cannot resolve standard layout directory `{directory}/`"),
            line: layout
                .span()
                .map_or(1, |span| line_column(&config_source, span.start).0),
            column: layout
                .span()
                .map_or(1, |span| line_column(&config_source, span.start).1),
        })?;
        if !canonical_directory.starts_with(&canonical_root) {
            return Err(LayoutError {
                path: config_path.display().to_string(),
                code: "E-LAYOUT-002",
                message: format!(
                    "standard layout directory `{directory}/` resolves outside the project root"
                ),
                line: layout
                    .span()
                    .map_or(1, |span| line_column(&config_source, span.start).0),
                column: layout
                    .span()
                    .map_or(1, |span| line_column(&config_source, span.start).1),
            });
        }
    }
    Ok(())
}

/// Whether the project opts into the standard layout. This is also used by the
/// module loader so namespace imports are never enabled for legacy projects.
pub(crate) fn standard_layout_enabled(root: &std::path::Path) -> Result<bool, LayoutError> {
    let Some(config_path) = find_config(root) else {
        return Ok(false);
    };
    if fs::symlink_metadata(&config_path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(LayoutError {
            path: config_path.display().to_string(),
            code: "E-LAYOUT-001",
            message: "project configuration must not be a symbolic link".into(),
            line: 1,
            column: 1,
        });
    }
    let config_source = fs::read_to_string(&config_path).map_err(|_| LayoutError {
        path: config_path.display().to_string(),
        code: "E-LAYOUT-001",
        message: "cannot read the project configuration".into(),
        line: 1,
        column: 1,
    })?;
    let document = config_source.parse::<DocumentMut>().map_err(|error| {
        span_error(
            &config_path,
            &config_source,
            error.to_string(),
            error.span(),
        )
    })?;
    let Some(project) = document.get("project").and_then(toml_edit::Item::as_table) else {
        return Ok(false);
    };
    let Some(layout) = project.get("layout") else {
        return Ok(false);
    };
    match layout.as_str() {
        Some("standard") => Ok(true),
        Some(profile) => Err(span_error(
            &config_path,
            &config_source,
            format!("unknown project layout `{profile}`; expected `standard`"),
            layout.span(),
        )),
        None => Err(span_error(
            &config_path,
            &config_source,
            "`project.layout` must be the string `standard`".into(),
            layout.span(),
        )),
    }
}

pub(crate) fn strict_architecture_enabled(root: &std::path::Path) -> Result<bool, LayoutError> {
    let Some(config_path) = find_config(root) else {
        return Ok(false);
    };
    let source = fs::read_to_string(&config_path).map_err(|_| LayoutError {
        path: config_path.display().to_string(),
        code: "E-LAYOUT-001",
        message: "cannot read the project configuration".into(),
        line: 1,
        column: 1,
    })?;
    let document = source
        .parse::<DocumentMut>()
        .map_err(|error| span_error(&config_path, &source, error.to_string(), error.span()))?;
    let project = document.get("project").and_then(toml_edit::Item::as_table);
    let Some(project) = project else {
        return Ok(false);
    };
    match project.get("strict_architecture") {
        Some(item)
            if project.get("layout").and_then(toml_edit::Item::as_str) == Some("standard") =>
        {
            item.as_bool().ok_or_else(|| {
                span_error(
                    &config_path,
                    &source,
                    "`project.strict_architecture` must be true or false".into(),
                    item.span(),
                )
            })
        }
        Some(item) if item.as_bool().is_none() => Err(span_error(
            &config_path,
            &source,
            "`project.strict_architecture` must be true or false".into(),
            item.span(),
        )),
        Some(_) => Ok(false),
        None => Ok(false),
    }
}

fn find_config(start: &std::path::Path) -> Option<PathBuf> {
    let mut directory = start;
    loop {
        let candidate = directory.join("zelyra.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        directory = directory.parent()?;
    }
}

fn span_error(
    path: &std::path::Path,
    source: &str,
    message: String,
    span: Option<std::ops::Range<usize>>,
) -> LayoutError {
    let (line, column) = span.map_or((1, 1), |span| line_column(source, span.start));
    LayoutError {
        path: path.display().to_string(),
        code: "E-LAYOUT-001",
        message,
        line,
        column,
    }
}

fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(source.len());
    let before = source.get(..offset).unwrap_or_default();
    let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .unwrap_or_default()
        .chars()
        .count()
        + 1;
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn project(config: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("zelyra-layout-{nonce}"));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("main.zyl"), "fn main() {}\n").unwrap();
        fs::write(root.join("zelyra.toml"), config).unwrap();
        root
    }

    #[test]
    fn legacy_projects_do_not_require_layout_directories() {
        let root = project("[project]\nname = \"legacy\"\n");
        assert!(validate(root.join("main.zyl").to_str().unwrap()).is_ok());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn strict_architecture_does_not_change_projects_without_an_opt_in_layout() {
        let root = project("[project]\nname = \"legacy\"\nstrict_architecture = true\n");
        assert!(validate(root.join("main.zyl").to_str().unwrap()).is_ok());
        assert!(!standard_layout_enabled(&root).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn standard_layout_requires_real_project_directories() {
        let root = project("[project]\nname = \"standard\"\nlayout = \"standard\"\n");
        let error = validate(root.join("main.zyl").to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E-LAYOUT-002");
        assert!(error.message.contains("schema/"));
        for directory in STANDARD_DIRECTORIES {
            fs::create_dir_all(root.join(directory)).unwrap();
        }
        assert!(validate(root.join("main.zyl").to_str().unwrap()).is_ok());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unknown_layout_is_rejected_and_standard_strict_setting_is_valid() {
        let root = project("[project]\nname = \"bad\"\nlayout = \"custom\"\n");
        let error = validate(root.join("main.zyl").to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E-LAYOUT-001");
        assert!(error.message.contains("expected `standard`"));
        fs::write(
            root.join("zelyra.toml"),
            "[project]\nname = \"strict\"\nlayout = \"standard\"\nstrict_architecture = true\n",
        )
        .unwrap();
        for directory in STANDARD_DIRECTORIES {
            fs::create_dir_all(root.join(directory)).unwrap();
        }
        assert!(validate(root.join("main.zyl").to_str().unwrap()).is_ok());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn standard_layout_rejects_symlinked_directory_roots() {
        use std::os::unix::fs::symlink;
        let root = project("[project]\nlayout = \"standard\"\n");
        for directory in STANDARD_DIRECTORIES.into_iter().skip(1) {
            fs::create_dir_all(root.join(directory)).unwrap();
        }
        let outside = root
            .parent()
            .unwrap()
            .join(format!("zelyra-layout-outside-{}", std::process::id()));
        fs::create_dir_all(&outside).unwrap();
        symlink(&outside, root.join("schema")).unwrap();
        let error = validate(root.join("main.zyl").to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E-LAYOUT-002");
        assert!(error.message.contains("real directory"));
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }
}
