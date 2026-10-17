use semver::{Version, VersionReq};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

mod registry;

const MANIFEST: &str = "zelyra.toml";
const LOCKFILE: &str = "zelyra.lock";
static NEXT_LOCK_TEMP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[derive(Clone, Debug)]
pub(crate) struct Package {
    pub(crate) name: String,
    pub(crate) version: Version,
    pub(crate) root: PathBuf,
    pub(crate) relative_root: String,
    pub(crate) entry: PathBuf,
    pub(crate) dependencies: BTreeMap<String, String>,
    zelyra_requirement: Option<String>,
    declared_effects: Option<BTreeSet<String>>,
    settings: Vec<PackageSettingSchema>,
    checksum: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PackageSettingSchema {
    pub(crate) name: String,
    pub(crate) type_name: String,
    pub(crate) required: bool,
    pub(crate) secret: bool,
    pub(crate) default: Option<toml::Value>,
    pub(crate) description: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct PackageGraph {
    workspace_root: PathBuf,
    packages: BTreeMap<String, Package>,
    paths: BTreeMap<PathBuf, String>,
    lock_contents: String,
}

pub(crate) struct PackageBundleSupport {
    pub(crate) root_manifest: String,
    pub(crate) package_files: Vec<String>,
    pub(crate) package_names: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct PackageInventory {
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) version: Version,
    pub(crate) entry: String,
    pub(crate) zelyra_requirement: Option<String>,
    pub(crate) checksum: String,
    pub(crate) effects: Option<Vec<String>>,
    pub(crate) settings: Vec<PackageSettingSchema>,
    pub(crate) dependencies: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PackageSettingValueStatus {
    pub(crate) name: String,
    pub(crate) status: &'static str,
    pub(crate) secret: bool,
    pub(crate) required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PackageSettingsStatus {
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) values: Vec<PackageSettingValueStatus>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PackageError {
    pub(crate) message: String,
}

pub(crate) fn is_unhashed_package_path(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(component, Component::Normal(name) if name == ".git" || name == "target")
    })
}

impl PackageGraph {
    /// Load a local, offline-only package graph rooted at the project directory.
    /// Projects without `[package]` metadata keep the legacy file-import model.
    pub(crate) fn load(root: &Path) -> Result<Option<Self>, PackageError> {
        let workspace_root = fs::canonicalize(root).map_err(|error| PackageError {
            message: format!("cannot resolve package workspace: {error}"),
        })?;
        let manifest_path = workspace_root.join(MANIFEST);
        reject_symlink(&manifest_path, "package manifest")?;
        if !manifest_path.is_file() {
            return Ok(None);
        }
        let root_manifest = read_manifest(&manifest_path)?;
        if !root_manifest.is_table() || root_manifest.get("package").is_none() {
            return Ok(None);
        }

        // Paths are package-instance identities. Package names alone are not
        // unique when the same library is used at multiple SemVer versions.
        let mut packages = BTreeMap::<String, Package>::new();
        let mut paths = BTreeMap::<PathBuf, String>::new();
        let mut pending = vec![(workspace_root.clone(), None::<VersionReq>)];
        let mut seen_roots = HashSet::new();
        while let Some((package_root, expected)) = pending.pop() {
            reject_path_symlinks(&workspace_root, &package_root)?;
            let canonical = fs::canonicalize(&package_root).map_err(|error| PackageError {
                message: format!(
                    "cannot resolve package path `{}`: {error}",
                    package_root.display()
                ),
            })?;
            if !canonical.starts_with(&workspace_root) {
                return Err(PackageError {
                    message: format!(
                        "package path `{}` resolves outside the project workspace",
                        package_root.display()
                    ),
                });
            }
            if seen_roots.contains(&canonical) {
                if let Some(requirement) = expected {
                    let package_key = paths.get(&canonical).ok_or_else(|| PackageError {
                        message: "package dependency cycle was encountered before resolution"
                            .into(),
                    })?;
                    let actual = packages
                        .get(package_key)
                        .expect("package path index is valid");
                    if !requirement.matches(&actual.version) {
                        return Err(PackageError {
                            message: format!(
                                "dependency at `{}` resolves to package `{}` version {}, which does not satisfy `{requirement}`",
                                canonical.display(),
                                actual.name,
                                actual.version
                            ),
                        });
                    }
                }
                continue;
            }
            seen_roots.insert(canonical.clone());
            let manifest_path = canonical.join(MANIFEST);
            let manifest = read_manifest(&manifest_path)?;
            let package_table = manifest
                .get("package")
                .and_then(toml::Value::as_table)
                .ok_or_else(|| PackageError {
                    message: format!(
                        "`{}` must declare a [package] table",
                        manifest_path.display()
                    ),
                })?;
            if let Some(field) = package_table.keys().find(|field| {
                !matches!(
                    field.as_str(),
                    "name" | "version" | "entry" | "effects" | "zelyra"
                )
            }) {
                return Err(PackageError {
                    message: format!(
                        "unknown package field `{field}` in `{}`",
                        manifest_path.display()
                    ),
                });
            }
            let name = string_field(package_table, "name", &manifest_path)?;
            validate_package_name(&name)?;
            let version_text = string_field(package_table, "version", &manifest_path)?;
            let version = Version::parse(&version_text).map_err(|error| PackageError {
                message: format!("invalid package version `{version_text}`: {error}"),
            })?;
            let zelyra_requirement = match package_table.get("zelyra") {
                None => None,
                Some(value) => {
                    let requirement = value.as_str().ok_or_else(|| PackageError {
                        message: format!(
                            "package `{name}` field `zelyra` must be a SemVer requirement string"
                        ),
                    })?;
                    let parsed = VersionReq::parse(requirement).map_err(|error| PackageError {
                        message: format!("invalid Zelyra compatibility requirement `{requirement}` for package `{name}`: {error}"),
                    })?;
                    let current = Version::parse(env!("CARGO_PKG_VERSION"))
                        .expect("CLI package version is valid SemVer");
                    if !parsed.matches(&current) {
                        return Err(PackageError {
                            message: format!("package `{name}` requires Zelyra `{requirement}`, but this compiler is `{current}`"),
                        });
                    }
                    Some(requirement.to_owned())
                }
            };
            let declared_effects = parse_declared_effects(package_table, &manifest_path, &name)?;
            let settings = parse_settings(&manifest, &manifest_path, &name)?;
            if let Some(requirement) = &expected {
                if !requirement.matches(&version) {
                    return Err(PackageError {
                        message: format!(
                            "package `{name}` version {version} does not satisfy `{requirement}`"
                        ),
                    });
                }
            }
            let default_entry =
                if canonical == workspace_root && canonical.join("main.zyl").is_file() {
                    "main.zyl"
                } else {
                    "src/lib.zyl"
                };
            let entry_text = package_table
                .get("entry")
                .and_then(toml::Value::as_str)
                .unwrap_or(default_entry);
            let entry_relative = safe_relative_path(entry_text).ok_or_else(|| PackageError {
                message: format!("package `{name}` entry must be a relative `.zyl` path"),
            })?;
            if is_unhashed_package_path(&entry_relative) {
                return Err(PackageError {
                    message: format!(
                        "package `{name}` entry may not be inside `.git` or `target`; those directories are excluded from package checksums"
                    ),
                });
            }
            if entry_relative
                .extension()
                .is_none_or(|extension| extension != "zyl")
            {
                return Err(PackageError {
                    message: format!("package `{name}` entry must end in `.zyl`"),
                });
            }
            let entry = canonical.join(entry_relative);
            let canonical_entry = fs::canonicalize(&entry).map_err(|error| PackageError {
                message: format!("cannot read package `{name}` entry `{entry_text}`: {error}"),
            })?;
            if !canonical_entry.starts_with(&canonical) || !canonical_entry.is_file() {
                return Err(PackageError {
                    message: format!("package `{name}` entry resolves outside its package"),
                });
            }
            let dependencies = parse_dependencies(&manifest, &manifest_path)?;
            let mut resolved_dependencies = BTreeMap::new();
            for (alias, dependency) in &dependencies {
                let dependency_root =
                    resolve_dependency_root(&workspace_root, &canonical, &dependency.0)?;
                resolved_dependencies.insert(alias.clone(), dependency_root.clone());
                pending.push((dependency_root, Some(dependency.1.clone())));
            }
            let relative_root = canonical
                .strip_prefix(&workspace_root)
                .expect("package root was checked beneath workspace")
                .to_string_lossy()
                .replace('\\', "/");
            if packages.values().any(|existing| {
                existing.name == name && existing.version == version && existing.root != canonical
            }) {
                return Err(PackageError {
                    message: format!(
                        "package `{name}` version {version} resolves to multiple local paths"
                    ),
                });
            }
            let checksum = package_checksum(&canonical)?;
            let package = Package {
                name: name.clone(),
                version,
                root: canonical.clone(),
                relative_root: relative_root.clone(),
                entry: canonical_entry,
                zelyra_requirement,
                dependencies: resolved_dependencies
                    .iter()
                    .map(|(alias, dependency_root)| {
                        let relative = dependency_root
                            .strip_prefix(&workspace_root)
                            .expect("dependency root is constrained to the workspace")
                            .to_string_lossy()
                            .replace('\\', "/");
                        (alias.clone(), relative)
                    })
                    .collect(),
                declared_effects,
                settings,
                checksum,
            };
            paths.insert(canonical, relative_root.clone());
            packages.insert(relative_root, package);
        }
        detect_dependency_cycles(&packages)?;
        let lock_contents = render_lock(&packages);
        Ok(Some(Self {
            workspace_root,
            packages,
            paths,
            lock_contents,
        }))
    }

    pub(crate) fn dependencies_present(&self) -> bool {
        self.packages
            .values()
            .any(|package| !package.dependencies.is_empty())
    }

    /// Return a deterministic, credential-free package catalogue for project
    /// inspection. Declared effects are informational ceilings, never grants.
    pub(crate) fn inventory(&self) -> Vec<PackageInventory> {
        self.packages
            .values()
            .map(|package| {
                let entry = package
                    .entry
                    .strip_prefix(&package.root)
                    .expect("package entry is contained by its package root")
                    .to_string_lossy()
                    .replace('\\', "/");
                let mut dependencies = package
                    .dependencies
                    .iter()
                    .filter_map(|(alias, relative)| {
                        self.packages
                            .get(relative)
                            .map(|dependency| (alias.clone(), dependency.relative_root.clone()))
                    })
                    .collect::<Vec<_>>();
                dependencies.sort();
                PackageInventory {
                    path: package.relative_root.clone(),
                    name: package.name.clone(),
                    version: package.version.clone(),
                    entry,
                    zelyra_requirement: package.zelyra_requirement.clone(),
                    checksum: package.checksum.clone(),
                    effects: package
                        .declared_effects
                        .as_ref()
                        .map(|effects| effects.iter().cloned().collect()),
                    settings: package.settings.clone(),
                    dependencies,
                }
            })
            .collect()
    }

    /// Validate application-owned package settings without returning their values.
    /// Secret settings are references to environment variables, never plaintext.
    pub(crate) fn settings_status(
        &self,
        environment_value: impl FnMut(&str) -> Option<String>,
    ) -> Result<Vec<PackageSettingsStatus>, PackageError> {
        const SETTINGS_FILE: &str = "zelyra.settings.toml";
        const MAX_SETTINGS_BYTES: u64 = 1024 * 1024;

        let settings_path = self.workspace_root.join(SETTINGS_FILE);
        reject_symlink(&settings_path, "module settings file")?;
        let contents = if settings_path.exists() {
            let metadata = fs::metadata(&settings_path).map_err(|error| PackageError {
                message: format!("cannot inspect `{SETTINGS_FILE}`: {error}"),
            })?;
            if metadata.len() > MAX_SETTINGS_BYTES {
                return Err(PackageError {
                    message: format!("`{SETTINGS_FILE}` exceeds the 1 MiB limit"),
                });
            }
            fs::read_to_string(&settings_path).map_err(|error| PackageError {
                message: format!("cannot read `{SETTINGS_FILE}`: {error}"),
            })?
        } else {
            String::new()
        };
        self.validate_settings_contents(&contents, environment_value)
    }

    /// Validate candidate settings before an editor or tool publishes them.
    /// The source is parsed and checked without touching the on-disk file.
    pub(crate) fn validate_settings_contents(
        &self,
        contents: &str,
        mut environment_value: impl FnMut(&str) -> Option<String>,
    ) -> Result<Vec<PackageSettingsStatus>, PackageError> {
        const SETTINGS_FILE: &str = "zelyra.settings.toml";
        const MAX_SETTINGS_BYTES: usize = 1024 * 1024;
        if contents.len() > MAX_SETTINGS_BYTES {
            return Err(PackageError {
                message: format!("`{SETTINGS_FILE}` exceeds the 1 MiB limit"),
            });
        }
        let config = if contents.trim().is_empty() {
            toml::Value::Table(toml::map::Map::new())
        } else {
            toml::from_str::<toml::Value>(contents).map_err(|error| PackageError {
                message: format!("invalid `{SETTINGS_FILE}`: {error}"),
            })?
        };
        let root = config.as_table().ok_or_else(|| PackageError {
            message: format!("`{SETTINGS_FILE}` must contain a TOML table"),
        })?;
        if let Some(field) = root
            .keys()
            .find(|field| field.as_str() != "package_settings")
        {
            return Err(PackageError {
                message: format!("unknown top-level field `{field}` in `{SETTINGS_FILE}`"),
            });
        }
        let configured_packages = root
            .get("package_settings")
            .map(|value| {
                value.as_table().ok_or_else(|| PackageError {
                    message: format!("`{SETTINGS_FILE}` [package_settings] must be a table"),
                })
            })
            .transpose()?
            .cloned()
            .unwrap_or_default();

        for (configured_path, values) in &configured_packages {
            let package_path = normalize_package_settings_path(configured_path);
            let package = self
                .packages
                .get(&package_path)
                .ok_or_else(|| PackageError {
                    message: format!(
                        "`{SETTINGS_FILE}` configures unknown package path `{configured_path}`"
                    ),
                })?;
            let values = values.as_table().ok_or_else(|| PackageError {
                message: format!(
                    "`{SETTINGS_FILE}` package settings for `{configured_path}` must be a table"
                ),
            })?;
            for key in values.keys() {
                if !package.settings.iter().any(|setting| setting.name == *key) {
                    return Err(PackageError {
                        message: format!(
                            "`{SETTINGS_FILE}` contains undeclared setting `{key}` for `{configured_path}`"
                        ),
                    });
                }
            }
        }

        let mut statuses = Vec::new();
        for package in self.packages.values() {
            if package.settings.is_empty() {
                continue;
            }
            let config_path = normalize_package_settings_path(&package.relative_root);
            let values = configured_packages
                .get(&config_path)
                .and_then(toml::Value::as_table);
            let mut setting_statuses = Vec::with_capacity(package.settings.len());
            for setting in &package.settings {
                let configured = values.and_then(|values| values.get(&setting.name));
                let status = if setting.secret {
                    match configured {
                        None => "missing",
                        Some(value) => {
                            let env_name = value
                                .as_table()
                                .filter(|table| table.len() == 1)
                                .and_then(|table| table.get("env"))
                                .and_then(toml::Value::as_str)
                                .filter(|name| valid_environment_name(name))
                                .ok_or_else(|| PackageError {
                                    message: format!(
                                        "secret setting `{}` for package `{}` must be an {{ env = \"UPPERCASE_NAME\" }} reference",
                                        setting.name, package.relative_root
                                    ),
                                })?;
                            match environment_value(env_name) {
                                Some(value) => {
                                    let value = zeroize::Zeroizing::new(value);
                                    if encode_secret_setting(&setting.type_name, &value).is_some() {
                                        "secret_available"
                                    } else {
                                        return Err(PackageError {
                                            message: format!(
                                                "secret setting `{}` for package `{}` does not match type `{}`",
                                                setting.name,
                                                package.relative_root,
                                                setting.type_name
                                            ),
                                        });
                                    }
                                }
                                _ => "missing",
                            }
                        }
                    }
                } else {
                    match configured {
                        Some(value) => {
                            if !setting_value_matches(&setting.type_name, value) {
                                return Err(PackageError {
                                    message: format!(
                                        "setting `{}` for package `{}` does not match type `{}`",
                                        setting.name, package.relative_root, setting.type_name
                                    ),
                                });
                            }
                            "configured"
                        }
                        None if setting.default.is_some() => "defaulted",
                        None => "missing",
                    }
                };
                setting_statuses.push(PackageSettingValueStatus {
                    name: setting.name.clone(),
                    status,
                    secret: setting.secret,
                    required: setting.required,
                });
            }
            statuses.push(PackageSettingsStatus {
                path: config_path,
                name: package.name.clone(),
                values: setting_statuses,
            });
        }
        Ok(statuses)
    }

    pub(crate) fn setting_type_for_source(
        &self,
        source_path: &str,
        setting_name: &str,
    ) -> Option<(&str, &str)> {
        let package = self
            .packages
            .values()
            .filter(|package| {
                package_owns_source(&package.relative_root, source_path)
                    && package
                        .settings
                        .iter()
                        .any(|setting| setting.name == setting_name)
            })
            .max_by_key(|package| package.relative_root.len())?;
        let setting = package
            .settings
            .iter()
            .find(|setting| setting.name == setting_name)?;
        Some((&package.relative_root, &setting.type_name))
    }

    /// Resolve validated setting values for the package owning each source.
    /// The resulting map is source-scoped so one module cannot read another
    /// package's configuration through the runtime builtin.
    pub(crate) fn runtime_settings(
        &self,
        sources: &[(u32, String)],
        mut environment_value: impl FnMut(&str) -> Option<String>,
    ) -> Result<zelyra_runtime::PackageSettings, PackageError> {
        self.settings_status(|name| environment_value(name))?;

        const SETTINGS_FILE: &str = "zelyra.settings.toml";
        let path = self.workspace_root.join(SETTINGS_FILE);
        reject_symlink(&path, "module settings file")?;
        let configured = if path.exists() {
            let metadata = fs::metadata(&path).map_err(|error| PackageError {
                message: format!("cannot inspect `{SETTINGS_FILE}`: {error}"),
            })?;
            if metadata.len() > 1024 * 1024 {
                return Err(PackageError {
                    message: format!("`{SETTINGS_FILE}` exceeds the 1 MiB limit"),
                });
            }
            let contents = fs::read_to_string(&path).map_err(|error| PackageError {
                message: format!("cannot read `{SETTINGS_FILE}`: {error}"),
            })?;
            toml::from_str::<toml::Value>(&contents).map_err(|error| PackageError {
                message: format!("invalid `{SETTINGS_FILE}`: {error}"),
            })?
        } else {
            toml::Value::Table(toml::map::Map::new())
        };
        let root = configured.as_table().ok_or_else(|| PackageError {
            message: format!("`{SETTINGS_FILE}` must contain a TOML table"),
        })?;
        if let Some(field) = root
            .keys()
            .find(|field| field.as_str() != "package_settings")
        {
            return Err(PackageError {
                message: format!("unknown top-level field `{field}` in `{SETTINGS_FILE}`"),
            });
        }
        let package_values = configured
            .get("package_settings")
            .and_then(toml::Value::as_table);
        if let Some(package_values) = package_values {
            for (configured_path, values) in package_values {
                let key = normalize_package_settings_path(configured_path);
                let package = self.packages.get(&key).ok_or_else(|| PackageError {
                    message: format!(
                        "`{SETTINGS_FILE}` configures unknown package path `{configured_path}`"
                    ),
                })?;
                let values = values.as_table().ok_or_else(|| PackageError {
                    message: format!(
                        "`{SETTINGS_FILE}` package settings for `{configured_path}` must be a table"
                    ),
                })?;
                if let Some(name) = values.keys().find(|name| {
                    !package
                        .settings
                        .iter()
                        .any(|setting| setting.name == **name)
                }) {
                    return Err(PackageError {
                        message: format!(
                            "`{SETTINGS_FILE}` contains undeclared setting `{name}` for `{configured_path}`"
                        ),
                    });
                }
            }
        }
        let mut resolved = zelyra_runtime::PackageSettings::default();
        for (source_id, source_path) in sources {
            let Some(package) = self
                .packages
                .values()
                .filter(|package| package_owns_source(&package.relative_root, source_path))
                .max_by_key(|package| package.relative_root.len())
            else {
                continue;
            };
            let key = normalize_package_settings_path(&package.relative_root);
            let configured_values = package_values
                .and_then(|values| values.get(&key))
                .and_then(toml::Value::as_table);
            for setting in &package.settings {
                let value = if setting.secret {
                    let env_name = configured_values
                        .and_then(|values| values.get(&setting.name))
                        .and_then(toml::Value::as_table)
                        .filter(|table| table.len() == 1)
                        .and_then(|table| table.get("env"))
                        .and_then(toml::Value::as_str)
                        .filter(|name| valid_environment_name(name))
                        .ok_or_else(|| PackageError {
                            message: format!(
                                "secret setting `{}` for package `{}` must be an {{ env = \"UPPERCASE_NAME\" }} reference",
                                setting.name, package.relative_root
                            ),
                        })?;
                    let secret = environment_value(env_name)
                        .filter(|value| !value.is_empty())
                        .ok_or_else(|| PackageError {
                            message: format!(
                                "required module setting `{}` for package `{}` is missing",
                                setting.name, package.relative_root
                            ),
                        })?;
                    let secret = zeroize::Zeroizing::new(secret);
                    encode_secret_setting(&setting.type_name, &secret)
                        .map(|serialized| serialized.as_str().to_owned())
                        .ok_or_else(|| PackageError {
                            message: format!(
                                "secret setting `{}` for package `{}` does not match type `{}`",
                                setting.name, package.relative_root, setting.type_name
                            ),
                        })?
                } else {
                    let value = configured_values
                        .and_then(|values| values.get(&setting.name))
                        .or(setting.default.as_ref())
                        .ok_or_else(|| PackageError {
                            message: format!(
                                "required module setting `{}` for package `{}` is missing",
                                setting.name, package.relative_root
                            ),
                        })?;
                    if !setting_value_matches(&setting.type_name, value) {
                        return Err(PackageError {
                            message: format!(
                                "setting `{}` for package `{}` does not match type `{}`",
                                setting.name, package.relative_root, setting.type_name
                            ),
                        });
                    }
                    serde_json::to_string(value).map_err(|error| PackageError {
                        message: format!("cannot encode module setting: {error}"),
                    })?
                };
                resolved
                    .insert_json(*source_id, setting.name.clone(), value)
                    .map_err(|error| PackageError {
                        message: format!("invalid resolved setting: {error}"),
                    })?;
            }
        }
        Ok(resolved)
    }

    fn package_count(&self) -> usize {
        self.packages.len()
    }

    pub(crate) fn check_lockfile(&self) -> Result<(), PackageError> {
        let path = self.workspace_root.join(LOCKFILE);
        reject_symlink(&path, "package lockfile")?;
        let actual = fs::read_to_string(&path).map_err(|_| PackageError {
            message: format!(
                "package lockfile `{LOCKFILE}` is missing; run `zelyra package lock <entry.zyl>`"
            ),
        })?;
        if actual != self.lock_contents {
            return Err(PackageError {
                message: format!(
                    "package lockfile `{LOCKFILE}` is stale; review dependencies and run `zelyra package lock --update`"
                ),
            });
        }
        Ok(())
    }

    pub(crate) fn lock_contents(&self) -> &str {
        &self.lock_contents
    }

    pub(crate) fn lock_path(&self) -> PathBuf {
        self.workspace_root.join(LOCKFILE)
    }

    pub(crate) fn root_dependency_entry_relative(&self, alias: &str) -> Option<String> {
        let root_key = self.paths.get(&self.workspace_root)?;
        let root = self.packages.get(root_key)?;
        let dependency_root = self.workspace_root.join(root.dependencies.get(alias)?);
        let package_key = self.paths.get(&dependency_root)?;
        let package = self.packages.get(package_key)?;
        Some(
            package
                .entry
                .strip_prefix(&self.workspace_root)
                .ok()?
                .to_string_lossy()
                .replace('\\', "/"),
        )
    }

    /// Return a minimal package graph needed by a module source closure.
    /// Package source files are copied as package content so the generated
    /// lockfile continues to cover each included package deterministically.
    pub(crate) fn bundle_support(
        &self,
        source_paths: &BTreeSet<String>,
    ) -> Result<PackageBundleSupport, PackageError> {
        let root_key = self
            .paths
            .get(&self.workspace_root)
            .ok_or_else(|| PackageError {
                message: "package graph has no workspace package".into(),
            })?;
        let root = self.packages.get(root_key).ok_or_else(|| PackageError {
            message: "package graph workspace package is missing".into(),
        })?;

        let mut included = BTreeSet::<String>::new();
        for source_path in source_paths {
            let source = self.workspace_root.join(source_path);
            if let Some((package_key, _)) = self
                .packages
                .iter()
                .filter(|(_, package)| package.root != self.workspace_root)
                .filter(|(_, package)| source.starts_with(&package.root))
                .max_by_key(|(_, package)| package.root.components().count())
            {
                included.insert(package_key.clone());
            }
        }

        let mut pending = included.iter().cloned().collect::<Vec<_>>();
        while let Some(package_key) = pending.pop() {
            let package = self
                .packages
                .get(&package_key)
                .ok_or_else(|| PackageError {
                    message: format!(
                        "package instance `{package_key}` disappeared from the resolved graph"
                    ),
                })?;
            for relative_root in package.dependencies.values() {
                let dependency_root = self.workspace_root.join(relative_root);
                let dependency_key =
                    self.paths
                        .get(&dependency_root)
                        .ok_or_else(|| PackageError {
                            message: format!("dependency at `{relative_root}` is not resolved"),
                        })?;
                if included.insert(dependency_key.clone()) {
                    pending.push(dependency_key.clone());
                }
            }
        }

        let mut manifest = read_manifest(&self.workspace_root.join(MANIFEST))?;
        let manifest_table = manifest.as_table_mut().ok_or_else(|| PackageError {
            message: "workspace package manifest must be a TOML table".into(),
        })?;
        let mut dependencies = toml::map::Map::new();
        for (alias, relative_root) in &root.dependencies {
            let dependency_root = self.workspace_root.join(relative_root);
            let Some(package_key) = self.paths.get(&dependency_root) else {
                return Err(PackageError {
                    message: format!("root dependency `{alias}` is not resolved"),
                });
            };
            if !included.contains(package_key) {
                continue;
            }
            let package = self.packages.get(package_key).ok_or_else(|| PackageError {
                message: format!(
                    "package instance `{package_key}` disappeared from the resolved graph"
                ),
            })?;
            let mut dependency = toml::map::Map::new();
            dependency.insert(
                "path".into(),
                toml::Value::String(package.relative_root.clone()),
            );
            dependency.insert(
                "version".into(),
                toml::Value::String(format!("={}", package.version)),
            );
            dependencies.insert(alias.clone(), toml::Value::Table(dependency));
        }
        if dependencies.is_empty() {
            manifest_table.remove("dependencies");
        } else {
            manifest_table.insert("dependencies".into(), toml::Value::Table(dependencies));
        }
        let root_manifest = toml::to_string_pretty(&manifest).map_err(|error| PackageError {
            message: format!("cannot render filtered bundle package manifest: {error}"),
        })?;

        let mut package_files = BTreeSet::<String>::new();
        for package_key in &included {
            let package = self.packages.get(package_key).ok_or_else(|| PackageError {
                message: format!(
                    "package instance `{package_key}` disappeared from the resolved graph"
                ),
            })?;
            for relative in package_source_files(&package.root)? {
                let source = package.root.join(&relative);
                let relative_to_workspace = source
                    .strip_prefix(&self.workspace_root)
                    .map_err(|_| PackageError {
                        message: format!(
                            "package source `{}` is outside the workspace",
                            source.display()
                        ),
                    })?
                    .to_string_lossy()
                    .replace('\\', "/");
                package_files.insert(relative_to_workspace);
            }
        }
        let package_names = included
            .iter()
            .filter_map(|package_key| self.packages.get(package_key))
            .map(|package| format!("{}@{}", package.name, package.version))
            .collect();
        Ok(PackageBundleSupport {
            root_manifest,
            package_files: package_files.into_iter().collect(),
            package_names,
        })
    }

    pub(crate) fn source_base(&self, importer: &Path) -> Option<PathBuf> {
        self.paths
            .iter()
            .filter(|(root, _)| importer.starts_with(root.as_path()))
            .max_by_key(|(root, _)| root.components().count())
            .and_then(|(_, package_key)| self.packages.get(package_key))
            .map(|package| package.root.clone())
    }

    pub(crate) fn declared_effects_for(&self, source: &Path) -> Option<(&str, &BTreeSet<String>)> {
        let package = self
            .paths
            .iter()
            .filter(|(root, _)| source.starts_with(root.as_path()))
            .max_by_key(|(root, _)| root.components().count())
            .and_then(|(_, package_key)| self.packages.get(package_key))?;
        Some((package.name.as_str(), package.declared_effects.as_ref()?))
    }

    pub(crate) fn import_target(
        &self,
        importer: &Path,
        spec: &str,
    ) -> Result<Option<PathBuf>, PackageError> {
        let Some(alias) = spec.strip_prefix("pkg:") else {
            return Ok(None);
        };
        let owner = self
            .paths
            .iter()
            .filter(|(root, _)| importer.starts_with(root.as_path()))
            .max_by_key(|(root, _)| root.components().count())
            .and_then(|(_, package_key)| self.packages.get(package_key))
            .ok_or_else(|| PackageError {
                message: format!("import `{spec}` is outside a declared package"),
            })?;
        let relative = owner.dependencies.get(alias).ok_or_else(|| PackageError {
            message: format!(
                "package `{}` does not declare dependency `{alias}`",
                owner.name
            ),
        })?;
        let target_root = self.workspace_root.join(relative);
        let target_key = self.paths.get(&target_root).ok_or_else(|| PackageError {
            message: format!("dependency package `{alias}` was not resolved"),
        })?;
        let target = self.packages.get(target_key).ok_or_else(|| PackageError {
            message: format!("dependency package `{alias}` was not resolved"),
        })?;
        if target.relative_root != *relative {
            return Err(PackageError {
                message: format!("dependency alias `{alias}` resolves inconsistently"),
            });
        }
        Ok(Some(target.entry.clone()))
    }
}

fn package_owns_source(package_root: &str, source_path: &str) -> bool {
    let package_root = package_root.trim_matches('/');
    package_root.is_empty()
        || package_root == "."
        || source_path == package_root
        || source_path.starts_with(&format!("{package_root}/"))
}

fn parse_declared_effects(
    package_table: &toml::map::Map<String, toml::Value>,
    manifest: &Path,
    package_name: &str,
) -> Result<Option<BTreeSet<String>>, PackageError> {
    let Some(value) = package_table.get("effects") else {
        return Ok(None);
    };
    let effects = value.as_array().ok_or_else(|| PackageError {
        message: format!(
            "`{}` package.effects must be an array of capability names",
            manifest.display()
        ),
    })?;
    let mut declared = BTreeSet::new();
    for effect in effects {
        let Some(effect) = effect.as_str() else {
            return Err(PackageError {
                message: format!(
                    "`{}` package.effects entries must be strings",
                    manifest.display()
                ),
            });
        };
        if !zelyra_runtime::KNOWN_CAPABILITIES.contains(&effect) {
            return Err(PackageError {
                message: format!(
                    "package `{package_name}` declares unknown effect `{effect}`; expected one of {}",
                    zelyra_runtime::KNOWN_CAPABILITIES.join(", ")
                ),
            });
        }
        if !declared.insert(effect.to_owned()) {
            return Err(PackageError {
                message: format!(
                    "package `{package_name}` declares effect `{effect}` more than once"
                ),
            });
        }
    }
    Ok(Some(declared))
}

fn normalize_package_settings_path(path: &str) -> String {
    if path.is_empty() {
        ".".into()
    } else {
        path.to_owned()
    }
}

fn valid_environment_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name.as_bytes()[0].is_ascii_uppercase()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

fn setting_value_matches(type_name: &str, value: &toml::Value) -> bool {
    match type_name {
        "String" => value.as_str().is_some_and(|value| value.len() <= 4096),
        "Bool" => value.is_bool(),
        "Int" => value.as_integer().is_some(),
        "Float" => value.as_float().is_some_and(f64::is_finite),
        _ => false,
    }
}

/// Convert an environment-backed setting to the JSON scalar representation
/// expected by `PackageSettings`. Errors intentionally never include the
/// environment value because it may be secret.
fn encode_secret_setting(type_name: &str, value: &str) -> Option<zeroize::Zeroizing<String>> {
    if value.is_empty() {
        return None;
    }
    let serialized = match type_name {
        "String" => serde_json::to_string(value).ok()?,
        "Bool" => match value {
            "true" | "false" => value.to_owned(),
            _ => return None,
        },
        "Int" => value.parse::<i64>().ok()?.to_string(),
        "Float" => {
            let value = value.parse::<f64>().ok()?;
            serde_json::Number::from_f64(value)?.to_string()
        }
        _ => return None,
    };
    (serialized.len() <= 8192).then(|| zeroize::Zeroizing::new(serialized))
}

fn parse_settings(
    manifest_value: &toml::Value,
    manifest: &Path,
    package_name: &str,
) -> Result<Vec<PackageSettingSchema>, PackageError> {
    let Some(settings) = manifest_value.get("settings") else {
        return Ok(Vec::new());
    };
    let settings = settings.as_table().ok_or_else(|| PackageError {
        message: format!("`{}` [settings] must be a table", manifest.display()),
    })?;
    if settings.len() > 128 {
        return Err(PackageError {
            message: format!("package `{package_name}` may declare at most 128 settings"),
        });
    }
    let mut result = Vec::with_capacity(settings.len());
    for (name, value) in settings {
        if name.is_empty()
            || name.len() > 64
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
            || !name.as_bytes()[0].is_ascii_lowercase()
        {
            return Err(PackageError {
                message: format!("package setting `{name}` must use lowercase snake_case"),
            });
        }
        let fields = value.as_table().ok_or_else(|| PackageError {
            message: format!("package setting `{name}` must be a table"),
        })?;
        if let Some(field) = fields.keys().find(|field| {
            !matches!(
                field.as_str(),
                "type" | "required" | "secret" | "default" | "description"
            )
        }) {
            return Err(PackageError {
                message: format!("unknown field `{field}` in package setting `{name}`"),
            });
        }
        let type_name = fields
            .get("type")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| PackageError {
                message: format!("package setting `{name}` requires a string `type`"),
            })?;
        if !matches!(type_name, "String" | "Bool" | "Int" | "Float") {
            return Err(PackageError {
                message: format!("package setting `{name}` has unsupported type `{type_name}`; expected String, Bool, Int or Float"),
            });
        }
        let required = fields
            .get("required")
            .and_then(toml::Value::as_bool)
            .ok_or_else(|| PackageError {
                message: format!("package setting `{name}` requires a boolean `required`"),
            })?;
        let secret = match fields.get("secret") {
            None => false,
            Some(value) => value.as_bool().ok_or_else(|| PackageError {
                message: format!("package setting `{name}` field `secret` must be a boolean"),
            })?,
        };
        let default = fields.get("default").cloned();
        if let Some(default) = &default {
            let matches_type = match type_name {
                "String" => default.is_str(),
                "Bool" => default.is_bool(),
                "Int" => default.is_integer(),
                "Float" => default.as_float().is_some_and(f64::is_finite),
                _ => false,
            };
            if !matches_type {
                return Err(PackageError {
                    message: format!(
                        "package setting `{name}` default does not match type `{type_name}`"
                    ),
                });
            }
            if default.as_str().is_some_and(|value| value.len() > 4096) {
                return Err(PackageError {
                    message: format!("package setting `{name}` default exceeds 4096 bytes"),
                });
            }
            if secret {
                return Err(PackageError {
                    message: format!("secret package setting `{name}` may not declare a default"),
                });
            }
            if required {
                return Err(PackageError {
                    message: format!(
                        "package setting `{name}` cannot be both required and have a default"
                    ),
                });
            }
        } else if !required {
            return Err(PackageError {
                message: format!("optional package setting `{name}` requires a default"),
            });
        }
        let description = match fields.get("description") {
            None => None,
            Some(value) => {
                let description = value
                    .as_str()
                    .filter(|text| !text.trim().is_empty())
                    .ok_or_else(|| PackageError {
                        message: format!(
                            "package setting `{name}` description must be a nonempty string"
                        ),
                    })?;
                if description.len() > 512 {
                    return Err(PackageError {
                        message: format!("package setting `{name}` description exceeds 512 bytes"),
                    });
                }
                Some(description.to_owned())
            }
        };
        result.push(PackageSettingSchema {
            name: name.clone(),
            type_name: type_name.to_owned(),
            required,
            secret,
            default,
            description,
        });
    }
    Ok(result)
}

pub(crate) fn write_lockfile(graph: &PackageGraph, update: bool) -> Result<bool, PackageError> {
    let path = graph.lock_path();
    reject_symlink(&path, "package lockfile")?;
    match fs::read_to_string(&path) {
        Ok(existing) if existing == graph.lock_contents() => return Ok(false),
        Ok(_) if !update => {
            return Err(PackageError {
                message: "lockfile exists and differs; pass `--update` after reviewing the dependency changes".into(),
            })
        }
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            return Err(PackageError {
                message: format!("cannot read package lockfile: {error}"),
            })
        }
        _ => {}
    }
    let temporary = loop {
        let sequence = NEXT_LOCK_TEMP.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let candidate = path.with_file_name(format!(
            "{LOCKFILE}.{}.{}.tmp",
            std::process::id(),
            sequence
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => break (candidate, file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(PackageError {
                    message: format!("cannot create temporary package lockfile: {error}"),
                })
            }
        }
    };
    let (temporary, mut file) = temporary;
    use std::io::Write;
    if let Err(error) = file
        .write_all(graph.lock_contents().as_bytes())
        .and_then(|()| file.sync_all())
    {
        let _ = fs::remove_file(&temporary);
        return Err(PackageError {
            message: format!("cannot write package lockfile: {error}"),
        });
    }
    #[cfg(unix)]
    let publish = fs::rename(&temporary, &path);
    #[cfg(windows)]
    let publish = replace_lockfile_windows(&temporary, &path);
    #[cfg(not(any(unix, windows)))]
    let publish = if path.exists() {
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "atomic replacement is unsupported on this platform",
        ))
    } else {
        fs::rename(&temporary, &path)
    };
    if let Err(error) = publish {
        let _ = fs::remove_file(&temporary);
        return Err(PackageError {
            message: format!("cannot atomically publish package lockfile: {error}"),
        });
    }
    Ok(true)
}

#[cfg(windows)]
fn replace_lockfile_windows(temporary: &Path, target: &Path) -> std::io::Result<()> {
    if !target.exists() {
        return fs::rename(temporary, target);
    }
    let sequence = NEXT_LOCK_TEMP.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let backup = target.with_file_name(format!(
        "{LOCKFILE}.{}.{}.bak",
        std::process::id(),
        sequence
    ));
    fs::rename(target, &backup)?;
    match fs::rename(temporary, target) {
        Ok(()) => {
            let _ = fs::remove_file(backup);
            Ok(())
        }
        Err(error) => {
            let _ = fs::rename(backup, target);
            Err(error)
        }
    }
}

pub(super) fn command(mut arguments: impl Iterator<Item = String>) -> std::process::ExitCode {
    use std::process::ExitCode;
    let Some(action) = arguments.next() else {
        eprintln!("Usage: zelyra package <lock|keygen|sign|verify|trust|registry> ...");
        return ExitCode::from(2);
    };
    if action != "lock" {
        return registry::command(&action, arguments);
    }
    let mut entry = None;
    let mut mode = None;
    for argument in arguments {
        match argument.as_str() {
            "--check" | "--update" => {
                if mode.replace(argument).is_some() {
                    eprintln!("error[E-PKG-001]: choose only one package lock mode");
                    return ExitCode::from(2);
                }
            }
            _ if argument.starts_with('-') => {
                eprintln!("Usage: zelyra package lock [entry.zyl] [--check|--update]");
                return ExitCode::from(2);
            }
            _ if entry.is_none() => entry = Some(argument),
            _ => {
                eprintln!("Usage: zelyra package lock [entry.zyl] [--check|--update]");
                return ExitCode::from(2);
            }
        }
    }
    let entry = entry.unwrap_or_else(|| ".".into());
    let path = Path::new(&entry);
    let root = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    };
    let graph = match PackageGraph::load(&root) {
        Ok(Some(graph)) => graph,
        Ok(None) => {
            eprintln!("error[E-PKG-001]: package lock requires a [package] table in `zelyra.toml`");
            return ExitCode::FAILURE;
        }
        Err(error) => {
            eprintln!("error[E-PKG-001]: {}", error.message);
            return ExitCode::FAILURE;
        }
    };
    if mode.as_deref() == Some("--check") {
        return match graph.check_lockfile() {
            Ok(()) => {
                println!(
                    "package lock is current ({} packages)",
                    graph.package_count()
                );
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error[E-PKG-002]: {}", error.message);
                ExitCode::FAILURE
            }
        };
    }
    match write_lockfile(&graph, mode.as_deref() == Some("--update")) {
        Ok(true) => {
            println!("wrote {}", graph.lock_path().display());
            ExitCode::SUCCESS
        }
        Ok(false) => {
            println!("package lock is already current");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error[E-PKG-002]: {}", error.message);
            ExitCode::FAILURE
        }
    }
}

fn read_manifest(path: &Path) -> Result<toml::Value, PackageError> {
    reject_symlink(path, "package manifest")?;
    let source = fs::read_to_string(path).map_err(|error| PackageError {
        message: format!("cannot read package manifest `{}`: {error}", path.display()),
    })?;
    toml::from_str(&source).map_err(|error| PackageError {
        message: format!("invalid package manifest `{}`: {error}", path.display()),
    })
}

fn reject_symlink(path: &Path, description: &str) -> Result<(), PackageError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(PackageError {
                message: format!(
                    "{description} `{}` must be a regular non-symlink file",
                    path.display()
                ),
            })
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(PackageError {
            message: format!("cannot inspect {description} `{}`: {error}", path.display()),
        }),
    }
}

fn reject_path_symlinks(root: &Path, path: &Path) -> Result<(), PackageError> {
    let relative = path.strip_prefix(root).map_err(|_| PackageError {
        message: format!(
            "package path `{}` is outside the project workspace",
            path.display()
        ),
    })?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(PackageError {
                message: format!("package path `{}` has an invalid component", path.display()),
            });
        }
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(PackageError {
                    message: format!(
                        "package paths may not traverse symlinks: `{}`",
                        current.display()
                    ),
                });
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(PackageError {
                    message: format!(
                        "cannot inspect package path `{}`: {error}",
                        current.display()
                    ),
                });
            }
        }
    }
    Ok(())
}

fn string_field(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
    manifest: &Path,
) -> Result<String, PackageError> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| PackageError {
            message: format!("`{}` requires a nonempty package.{key}", manifest.display()),
        })
}

pub(crate) fn validate_package_name(name: &str) -> Result<(), PackageError> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_-".contains(&byte))
        || !name.as_bytes()[0].is_ascii_lowercase()
    {
        return Err(PackageError {
            message: format!("package name `{name}` must start with a lowercase letter and use lowercase ASCII, digits, `_` or `-`"),
        });
    }
    Ok(())
}

fn safe_relative_path(value: &str) -> Option<PathBuf> {
    let path = Path::new(value);
    if value.is_empty()
        || value.contains('\\')
        || path.is_absolute()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return None;
    }
    Some(path.to_path_buf())
}

fn safe_dependency_path(value: &str) -> Option<PathBuf> {
    let path = Path::new(value);
    if value.is_empty()
        || value.contains('\\')
        || path.is_absolute()
        || !path.components().all(|component| {
            matches!(
                component,
                Component::Normal(_) | Component::CurDir | Component::ParentDir
            )
        })
    {
        return None;
    }
    Some(path.to_path_buf())
}

fn resolve_dependency_root(
    workspace_root: &Path,
    package_root: &Path,
    relative: &Path,
) -> Result<PathBuf, PackageError> {
    let mut candidate = package_root.to_path_buf();
    for component in relative.components() {
        match component {
            Component::Normal(name) => {
                candidate.push(name);
                reject_path_symlinks(workspace_root, &candidate)?;
            }
            Component::CurDir => {}
            Component::ParentDir if candidate != workspace_root => {
                candidate.pop();
            }
            Component::ParentDir => {
                return Err(PackageError {
                    message: "package dependency path resolves outside the project workspace"
                        .into(),
                });
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(PackageError {
                    message: "package dependency path must remain inside the project workspace"
                        .into(),
                });
            }
        }
        if !candidate.starts_with(workspace_root) {
            return Err(PackageError {
                message: "package dependency path resolves outside the project workspace".into(),
            });
        }
    }
    reject_path_symlinks(workspace_root, &candidate)?;
    let canonical = fs::canonicalize(&candidate).map_err(|error| PackageError {
        message: format!(
            "cannot resolve package dependency path `{}`: {error}",
            candidate.display()
        ),
    })?;
    if !canonical.starts_with(workspace_root) {
        return Err(PackageError {
            message: format!(
                "package dependency path `{}` resolves outside the project workspace",
                candidate.display()
            ),
        });
    }
    Ok(canonical)
}

fn parse_dependencies(
    manifest: &toml::Value,
    path: &Path,
) -> Result<BTreeMap<String, (PathBuf, VersionReq)>, PackageError> {
    let Some(dependencies) = manifest.get("dependencies") else {
        return Ok(BTreeMap::new());
    };
    let table = dependencies.as_table().ok_or_else(|| PackageError {
        message: format!("`{}` [dependencies] must be a table", path.display()),
    })?;
    let mut result = BTreeMap::new();
    for (alias, value) in table {
        validate_package_name(alias)?;
        let fields = value.as_table().ok_or_else(|| PackageError {
            message: format!("dependency `{alias}` must use a path and version table"),
        })?;
        if fields.len() != 2 || !fields.contains_key("path") || !fields.contains_key("version") {
            return Err(PackageError {
                message: format!(
                    "dependency `{alias}` currently supports only explicit `path` and `version` fields"
                ),
            });
        }
        let dependency_path = fields
            .get("path")
            .and_then(toml::Value::as_str)
            .and_then(safe_dependency_path)
            .ok_or_else(|| PackageError {
                message: format!("dependency `{alias}` requires a safe relative path"),
            })?;
        let version_text = fields
            .get("version")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| PackageError {
                message: format!("dependency `{alias}` requires a semver version constraint"),
            })?;
        let version = VersionReq::parse(version_text).map_err(|error| PackageError {
            message: format!("invalid version constraint `{version_text}` for `{alias}`: {error}"),
        })?;
        if result
            .insert(alias.clone(), (dependency_path, version))
            .is_some()
        {
            return Err(PackageError {
                message: format!("duplicate dependency alias `{alias}`"),
            });
        }
    }
    Ok(result)
}

fn detect_dependency_cycles(packages: &BTreeMap<String, Package>) -> Result<(), PackageError> {
    fn visit<'a>(
        package_key: &'a str,
        packages: &'a BTreeMap<String, Package>,
        active: &mut Vec<&'a str>,
        done: &mut HashSet<&'a str>,
    ) -> Result<(), PackageError> {
        if done.contains(package_key) {
            return Ok(());
        }
        if let Some(index) = active.iter().position(|current| *current == package_key) {
            let mut cycle = active[index..].to_vec();
            cycle.push(package_key);
            return Err(PackageError {
                message: format!("package dependency cycle: {}", cycle.join(" -> ")),
            });
        }
        active.push(package_key);
        let package = packages
            .get(package_key)
            .expect("dependency package instance exists");
        for target_path in package.dependencies.values() {
            if !packages.contains_key(target_path) {
                return Err(PackageError {
                    message: format!(
                        "dependency path `{target_path}` is absent from the resolved package graph"
                    ),
                });
            }
            visit(target_path, packages, active, done)?;
        }
        active.pop();
        done.insert(package_key);
        Ok(())
    }
    let mut done = HashSet::new();
    for package_key in packages.keys() {
        visit(package_key, packages, &mut Vec::new(), &mut done)?;
    }
    Ok(())
}

fn package_checksum(root: &Path) -> Result<String, PackageError> {
    let mut files = package_source_files(root)?
        .into_iter()
        .map(|relative| {
            let raw = relative.to_str().ok_or_else(|| PackageError {
                message: "package source paths must be valid UTF-8".into(),
            })?;
            if !cfg!(windows) && raw.contains('\\') {
                return Err(PackageError {
                    message: format!(
                        "package source path `{raw}` contains an unsupported backslash"
                    ),
                });
            }
            let name = raw.replace('\\', "/");
            Ok((name, relative))
        })
        .collect::<Result<Vec<_>, PackageError>>()?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hash = Sha256::new();
    for (name, relative) in files {
        let source = fs::read_to_string(root.join(&relative)).map_err(|error| PackageError {
            message: format!("cannot read package source `{name}`: {error}"),
        })?;
        let bytes = source.replace("\r\n", "\n").into_bytes();
        hash.update((name.len() as u64).to_be_bytes());
        hash.update(name.as_bytes());
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    }
    Ok(format!("sha256:{:x}", hash.finalize()))
}

fn package_source_files(root: &Path) -> Result<Vec<PathBuf>, PackageError> {
    fn visit(root: &Path, directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), PackageError> {
        let mut entries = fs::read_dir(directory)
            .map_err(|error| PackageError {
                message: format!(
                    "cannot read package directory `{}`: {error}",
                    directory.display()
                ),
            })?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| PackageError {
                message: format!("cannot inspect package directory: {error}"),
            })?;
        entries.sort();
        for path in entries {
            let metadata = fs::symlink_metadata(&path).map_err(|error| PackageError {
                message: format!("cannot inspect package path `{}`: {error}", path.display()),
            })?;
            if metadata.file_type().is_symlink() {
                return Err(PackageError {
                    message: format!(
                        "package source may not contain symlinks: `{}`",
                        path.display()
                    ),
                });
            }
            if metadata.is_dir() {
                if path
                    .file_name()
                    .is_some_and(|name| name == ".git" || name == "target")
                {
                    continue;
                }
                visit(root, &path, files)?;
            } else if metadata.is_file()
                && (path.file_name().is_some_and(|name| name == MANIFEST)
                    || path.extension().is_some_and(|extension| extension == "zyl"))
            {
                files.push(
                    path.strip_prefix(root)
                        .expect("walk remains in package")
                        .to_path_buf(),
                );
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    visit(root, root, &mut files)?;
    files.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
    Ok(files)
}

fn render_lock(packages: &BTreeMap<String, Package>) -> String {
    let mut output = String::from("lock_version = 1\n\n");
    for (index, package) in packages.values().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        output.push_str("[[package]]\n");
        output.push_str(&format!("name = {:?}\n", package.name));
        output.push_str(&format!("version = {:?}\n", package.version.to_string()));
        output.push_str(&format!("path = {:?}\n", package.relative_root));
        output.push_str(&format!("checksum = {:?}\n", package.checksum));
        let dependencies = package
            .dependencies
            .iter()
            .map(|(alias, path)| format!("{alias}={path}"))
            .collect::<Vec<_>>();
        output.push_str(&format!("dependencies = {:?}\n", dependencies));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture() -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("zelyra-packages-{id}"));
        fs::create_dir_all(root.join("packages/math/src")).unwrap();
        fs::write(
            root.join("zelyra.toml"),
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nentry = \"main.zyl\"\n\n[dependencies.math]\npath = \"packages/math\"\nversion = \"^1.2\"\n",
        )
        .unwrap();
        fs::write(
            root.join("main.zyl"),
            "import \"pkg:math\" as math\nfn main() {}\n",
        )
        .unwrap();
        fs::write(
            root.join("packages/math/zelyra.toml"),
            "[package]\nname = \"math\"\nversion = \"1.2.3\"\nentry = \"src/lib.zyl\"\n",
        )
        .unwrap();
        fs::write(
            root.join("packages/math/src/lib.zyl"),
            "pub fn add(left: Int, right: Int) -> Int { return left + right }\n",
        )
        .unwrap();
        root
    }

    #[test]
    fn environment_secrets_are_converted_to_the_declared_scalar_type() {
        assert_eq!(
            encode_secret_setting("String", "api-token").map(|value| value.as_str().to_owned()),
            Some("\"api-token\"".into())
        );
        assert_eq!(
            encode_secret_setting("Bool", "true").map(|value| value.as_str().to_owned()),
            Some("true".into())
        );
        assert_eq!(encode_secret_setting("Bool", "1"), None);
        assert_eq!(
            encode_secret_setting("Int", "-42").map(|value| value.as_str().to_owned()),
            Some("-42".into())
        );
        assert_eq!(encode_secret_setting("Int", "9223372036854775808"), None);
        assert_eq!(
            encode_secret_setting("Float", "1.25").map(|value| value.as_str().to_owned()),
            Some("1.25".into())
        );
        assert_eq!(encode_secret_setting("Float", "NaN"), None);
        assert_eq!(encode_secret_setting("String", ""), None);

        let too_large = "x".repeat(8192);
        assert_eq!(encode_secret_setting("String", &too_large), None);
    }

    #[test]
    fn resolves_local_package_imports_and_writes_a_deterministic_content_lock() {
        let root = fixture();
        let graph = PackageGraph::load(&root).unwrap().unwrap();
        let entry = fs::canonicalize(root.join("main.zyl")).unwrap();
        let resolved = graph.import_target(&entry, "pkg:math").unwrap().unwrap();
        assert_eq!(
            resolved,
            fs::canonicalize(root.join("packages/math/src/lib.zyl")).unwrap()
        );
        let original_source = fs::read_to_string(root.join("packages/math/src/lib.zyl")).unwrap();
        let crlf_source = original_source.replace('\n', "\r\n");
        fs::write(root.join("packages/math/src/lib.zyl"), crlf_source).unwrap();
        assert_eq!(
            graph.lock_contents(),
            PackageGraph::load(&root).unwrap().unwrap().lock_contents()
        );
        assert!(graph.lock_contents().ends_with('\n'));
        assert!(!graph.lock_contents().ends_with("\n\n"));
        assert!(write_lockfile(&graph, false).unwrap());
        graph.check_lockfile().unwrap();
        assert!(!write_lockfile(&graph, false).unwrap());

        fs::write(
            root.join("packages/math/src/lib.zyl"),
            "pub fn add(left: Int, right: Int) -> Int { return left + right + 1 }\n",
        )
        .unwrap();
        let changed = PackageGraph::load(&root).unwrap().unwrap();
        assert!(changed.check_lockfile().is_err());
        assert!(write_lockfile(&changed, false).is_err());
        assert!(write_lockfile(&changed, true).unwrap());
        changed.check_lockfile().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_package_path_escape_and_unsatisfied_version_constraints() {
        let root = fixture();
        let manifest = root.join("zelyra.toml");
        let original = fs::read_to_string(&manifest).unwrap();
        fs::write(&manifest, original.replace("packages/math", "../math")).unwrap();
        assert!(PackageGraph::load(&root)
            .unwrap_err()
            .message
            .contains("outside the project workspace"));
        fs::write(&manifest, original.replace("^1.2", "^2.0")).unwrap();
        assert!(PackageGraph::load(&root)
            .unwrap_err()
            .message
            .contains("does not satisfy"));
        fs::write(
            &manifest,
            original.replace("packages/math", "packages\\\\math"),
        )
        .unwrap();
        assert!(PackageGraph::load(&root)
            .unwrap_err()
            .message
            .contains("safe relative path"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn package_zelyra_requirement_is_validated_reported_and_lock_covered() {
        let root = fixture();
        let manifest = root.join("packages/math/zelyra.toml");
        let original = fs::read_to_string(&manifest).unwrap();
        let current = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();

        write_lockfile(&PackageGraph::load(&root).unwrap().unwrap(), true).unwrap();
        fs::write(
            &manifest,
            format!(
                "{original}zelyra = \">={}.{}\"\n",
                current.major, current.minor
            ),
        )
        .unwrap();
        let compatible = PackageGraph::load(&root).unwrap().unwrap();
        let expected_requirement = format!(">={}.{}", current.major, current.minor);
        let package = compatible
            .inventory()
            .into_iter()
            .find(|package| package.name == "math")
            .unwrap();
        assert_eq!(
            package.zelyra_requirement.as_deref(),
            Some(expected_requirement.as_str())
        );
        assert!(compatible.check_lockfile().is_err());
        write_lockfile(&compatible, true).unwrap();
        compatible.check_lockfile().unwrap();

        fs::write(&manifest, format!("{original}zelyra = \">=99.0.0\"\n")).unwrap();
        let incompatible = PackageGraph::load(&root).unwrap_err();
        assert!(incompatible.message.contains("requires Zelyra `>=99.0.0`"));
        assert!(incompatible.message.contains(&current.to_string()));

        fs::write(&manifest, format!("{original}zelyra = \"not-semver\"\n")).unwrap();
        assert!(PackageGraph::load(&root)
            .unwrap_err()
            .message
            .contains("invalid Zelyra compatibility requirement"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_the_same_package_version_at_two_local_paths() {
        let root = fixture();
        let duplicate = root.join("packages/math-copy");
        fs::create_dir_all(duplicate.join("src")).unwrap();
        fs::write(
            duplicate.join("zelyra.toml"),
            "[package]\nname = \"math\"\nversion = \"1.2.3\"\nentry = \"src/lib.zyl\"\n",
        )
        .unwrap();
        fs::write(
            duplicate.join("src/lib.zyl"),
            "pub fn add(left: Int, right: Int) -> Int { return left + right }\n",
        )
        .unwrap();
        let root_manifest = root.join("zelyra.toml");
        let original = fs::read_to_string(&root_manifest).unwrap();
        fs::write(
            &root_manifest,
            format!(
                "{original}\n[dependencies.math_copy]\npath = \"packages/math-copy\"\nversion = \"^1.2\"\n"
            ),
        )
        .unwrap();

        let error = PackageGraph::load(&root).unwrap_err();
        assert!(error.message.contains("math` version 1.2.3"));
        assert!(error.message.contains("multiple local paths"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn detects_cycles_by_package_path_even_when_names_can_repeat() {
        let root = fixture();
        let manifest = root.join("zelyra.toml");
        fs::write(
            &manifest,
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nentry = \"main.zyl\"\n\n[dependencies.alpha]\npath = \"packages/alpha\"\nversion = \"^1.0\"\n",
        )
        .unwrap();
        for (directory, name, dependency, target) in [
            ("alpha", "alpha", "beta", "../beta"),
            ("beta", "beta", "alpha", "../alpha"),
        ] {
            let package = root.join("packages").join(directory);
            fs::create_dir_all(package.join("src")).unwrap();
            fs::write(
                package.join("zelyra.toml"),
                format!(
                    "[package]\nname = {name:?}\nversion = \"1.0.0\"\nentry = \"src/lib.zyl\"\n\n[dependencies.{dependency}]\npath = {target:?}\nversion = \"^1.0\"\n"
                ),
            )
            .unwrap();
            fs::write(
                package.join("src/lib.zyl"),
                "pub fn value() -> Int { return 1 }\n",
            )
            .unwrap();
        }

        let error = PackageGraph::load(&root).unwrap_err();
        assert!(error.message.contains("package dependency cycle"));
        assert!(error.message.contains("packages/alpha"));
        assert!(error.message.contains("packages/beta"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_unknown_package_manifest_fields_instead_of_silently_ignoring_typos() {
        let root = fixture();
        let manifest = root.join("packages/math/zelyra.toml");
        let original = fs::read_to_string(&manifest).unwrap();
        fs::write(&manifest, original.replace("version =", "versoin =")).unwrap();
        assert!(PackageGraph::load(&root)
            .unwrap_err()
            .message
            .contains("unknown package field `versoin`"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_symlinked_package_source_files() {
        let root = fixture();
        #[cfg(unix)]
        {
            let target = std::env::temp_dir()
                .join(format!("zelyra-package-outside-{}.zyl", std::process::id()));
            fs::write(&target, "pub fn escaped() {}\n").unwrap();
            let package_source = root.join("packages/math/src/escape.zyl");
            std::os::unix::fs::symlink(&target, &package_source).unwrap();
            assert!(PackageGraph::load(&root)
                .unwrap_err()
                .message
                .contains("may not contain symlinks"));
            fs::remove_file(target).unwrap();
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn package_effects_must_be_known_and_unique() {
        let root = fixture();
        let manifest = root.join("packages/math/zelyra.toml");
        let original = fs::read_to_string(&manifest).unwrap();

        fs::write(&manifest, format!("{original}effects = [\"Telepathy\"]\n")).unwrap();
        let unknown = PackageGraph::load(&root).unwrap_err();
        assert!(unknown.message.contains("unknown effect `Telepathy`"));

        fs::write(
            &manifest,
            format!("{original}effects = [\"Network\", \"Network\"]\n"),
        )
        .unwrap();
        let duplicate = PackageGraph::load(&root).unwrap_err();
        assert!(duplicate
            .message
            .contains("declares effect `Network` more than once"));

        fs::write(&manifest, original).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn typed_setting_schemas_are_sorted_redacted_and_lock_covered() {
        let root = fixture();
        let manifest = root.join("packages/math/zelyra.toml");
        let original = fs::read_to_string(&manifest).unwrap();
        fs::write(
            &manifest,
            format!(
                "{original}\n[settings.api_timeout]\ntype = \"Int\"\nrequired = false\ndefault = 15\ndescription = \"Request timeout\"\n\n[settings.api_token]\ntype = \"String\"\nrequired = true\nsecret = true\ndescription = \"Credential\"\n"
            ),
        )
        .unwrap();
        let graph = PackageGraph::load(&root).unwrap().unwrap();
        let package = graph
            .inventory()
            .into_iter()
            .find(|package| package.name == "math")
            .unwrap();
        assert_eq!(
            package.settings,
            vec![
                PackageSettingSchema {
                    name: "api_timeout".into(),
                    type_name: "Int".into(),
                    required: false,
                    secret: false,
                    default: Some(toml::Value::Integer(15)),
                    description: Some("Request timeout".into()),
                },
                PackageSettingSchema {
                    name: "api_token".into(),
                    type_name: "String".into(),
                    required: true,
                    secret: true,
                    default: None,
                    description: Some("Credential".into()),
                },
            ]
        );
        assert!(write_lockfile(&graph, true).unwrap());
        graph.check_lockfile().unwrap();
        let changed = fs::read_to_string(&manifest)
            .unwrap()
            .replace("default = 15", "default = 20");
        fs::write(&manifest, changed).unwrap();
        assert!(PackageGraph::load(&root)
            .unwrap()
            .unwrap()
            .check_lockfile()
            .is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn application_settings_validate_types_and_report_only_secret_availability() {
        let root = fixture();
        let package_manifest = root.join("packages/math/zelyra.toml");
        let original = fs::read_to_string(&package_manifest).unwrap();
        fs::write(
            &package_manifest,
            format!(
                "{original}\n[settings.timeout]\ntype = \"Int\"\nrequired = false\ndefault = 15\n\n[settings.api_token]\ntype = \"String\"\nrequired = true\nsecret = true\n"
            ),
        )
        .unwrap();
        fs::write(
            root.join("zelyra.settings.toml"),
            "[package_settings.\"packages/math\"]\ntimeout = 25\napi_token = { env = \"MATH_API_TOKEN\" }\n",
        )
        .unwrap();

        let graph = PackageGraph::load(&root).unwrap().unwrap();
        let statuses = graph
            .settings_status(|name| {
                (name == "MATH_API_TOKEN").then(|| "secret-value-must-not-escape".into())
            })
            .unwrap();
        let package = statuses
            .iter()
            .find(|package| package.path == "packages/math")
            .unwrap();
        assert_eq!(
            package.values,
            vec![
                PackageSettingValueStatus {
                    name: "api_token".into(),
                    status: "secret_available",
                    secret: true,
                    required: true,
                },
                PackageSettingValueStatus {
                    name: "timeout".into(),
                    status: "configured",
                    secret: false,
                    required: false,
                },
            ]
        );
        assert!(!format!("{statuses:?}").contains("secret-value-must-not-escape"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn application_settings_reject_type_errors_plaintext_secrets_and_unknown_packages() {
        let root = fixture();
        let package_manifest = root.join("packages/math/zelyra.toml");
        let original = fs::read_to_string(&package_manifest).unwrap();
        fs::write(
            &package_manifest,
            format!(
                "{original}\n[settings.timeout]\ntype = \"Int\"\nrequired = false\ndefault = 15\n\n[settings.api_token]\ntype = \"String\"\nrequired = true\nsecret = true\n"
            ),
        )
        .unwrap();
        let graph = PackageGraph::load(&root).unwrap().unwrap();
        let settings_file = root.join("zelyra.settings.toml");

        fs::write(
            &settings_file,
            "[package_settings.\"packages/math\"]\ntimeout = \"25\"\n",
        )
        .unwrap();
        assert!(graph
            .settings_status(|_| None)
            .unwrap_err()
            .message
            .contains("does not match type `Int`"));

        fs::write(
            &settings_file,
            "[package_settings.\"packages/math\"]\ntimeout = 25\napi_token = \"must-not-be-reported\"\n",
        )
        .unwrap();
        let secret_error = graph.settings_status(|_| None).unwrap_err();
        assert!(secret_error.message.contains("must be an"));
        assert!(!secret_error.message.contains("must-not-be-reported"));

        fs::write(
            &settings_file,
            "[package_settings.\"packages/unknown\"]\ntimeout = 25\n",
        )
        .unwrap();
        assert!(graph
            .settings_status(|_| None)
            .unwrap_err()
            .message
            .contains("unknown package path"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_typed_package_setting_schemas_fail_closed() {
        let root = fixture();
        let manifest = root.join("packages/math/zelyra.toml");
        let original = fs::read_to_string(&manifest).unwrap();
        for (schema, expected) in [
            (
                "[settings.timeout]\ntype = \"Duration\"\nrequired = true\n",
                "unsupported type",
            ),
            (
                "[settings.timeout]\ntype = \"Int\"\nrequired = false\ndefault = \"15\"\n",
                "default does not match",
            ),
            (
                "[settings.secret_value]\ntype = \"String\"\nrequired = false\nsecret = true\ndefault = \"hidden\"\n",
                "may not declare a default",
            ),
            (
                "[settings.timeout]\ntype = \"Int\"\nrequired = false\n",
                "requires a default",
            ),
            (
                "[settings.timeout]\ntype = \"Int\"\nrequired = true\ndefault = 15\n",
                "both required and have a default",
            ),
            (
                "[settings.api-token]\ntype = \"String\"\nrequired = true\n",
                "lowercase snake_case",
            ),
        ] {
            fs::write(&manifest, format!("{original}\n{schema}")).unwrap();
            let error = PackageGraph::load(&root).unwrap_err();
            assert!(error.message.contains(expected), "{}", error.message);
        }
        fs::write(
            &manifest,
            format!(
                "{original}\n[settings.large_value]\ntype = \"String\"\nrequired = false\ndefault = {:?}\n",
                "x".repeat(4097)
            ),
        )
        .unwrap();
        assert!(PackageGraph::load(&root)
            .unwrap_err()
            .message
            .contains("default exceeds 4096 bytes"));

        let long_name = "a".repeat(65);
        fs::write(
            &manifest,
            format!("{original}\n[settings.{long_name}]\ntype = \"String\"\nrequired = true\n"),
        )
        .unwrap();
        assert!(PackageGraph::load(&root)
            .unwrap_err()
            .message
            .contains("lowercase snake_case"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refuses_to_read_or_replace_a_symlinked_lockfile() {
        let root = fixture();
        let graph = PackageGraph::load(&root).unwrap().unwrap();
        #[cfg(unix)]
        {
            let target = std::env::temp_dir().join(format!(
                "zelyra-package-lock-target-{}.lock",
                std::process::id()
            ));
            fs::write(&target, "user data\n").unwrap();
            std::os::unix::fs::symlink(&target, root.join(LOCKFILE)).unwrap();
            assert!(graph
                .check_lockfile()
                .unwrap_err()
                .message
                .contains("package lockfile"));
            assert!(write_lockfile(&graph, true)
                .unwrap_err()
                .message
                .contains("package lockfile"));
            assert_eq!(fs::read_to_string(&target).unwrap(), "user data\n");
            fs::remove_file(target).unwrap();
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_symlinks_even_when_parent_segments_would_cancel_them() {
        let root = fixture();
        #[cfg(unix)]
        {
            let outside = std::env::temp_dir().join(format!(
                "zelyra-package-path-outside-{}",
                std::process::id()
            ));
            fs::create_dir_all(&outside).unwrap();
            let link = root.join("packages/link");
            std::os::unix::fs::symlink(&outside, &link).unwrap();
            let manifest = root.join("zelyra.toml");
            let source = fs::read_to_string(&manifest).unwrap();
            fs::write(
                &manifest,
                source.replace("packages/math", "packages/link/../math"),
            )
            .unwrap();
            assert!(PackageGraph::load(&root)
                .unwrap_err()
                .message
                .contains("may not traverse symlinks"));
            fs::remove_file(link).unwrap();
            fs::remove_dir_all(outside).unwrap();
        }
        fs::remove_dir_all(root).unwrap();
    }
}
