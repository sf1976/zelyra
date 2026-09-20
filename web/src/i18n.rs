use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiLanguage {
    #[default]
    English,
    German,
}

impl UiLanguage {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "en" => Some(Self::English),
            "de" => Some(Self::German),
            _ => None,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::German => "de",
        }
    }
}

/// Project-provided translations layered over Zelyra's built-in UI catalogs.
///
/// Project entries are presentation text only. They cannot change compiler,
/// authorization, or API behavior.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectUiCatalogs {
    english: BTreeMap<String, String>,
    german: BTreeMap<String, String>,
}

impl ProjectUiCatalogs {
    /// Replaces one language catalog after validating its JSON object and keys.
    pub fn set_json(&mut self, language: UiLanguage, source: &str) -> Result<(), String> {
        let catalog = serde_json::from_str::<BTreeMap<String, String>>(source).map_err(|_| {
            "catalog must be a JSON object containing only string values".to_owned()
        })?;
        for (key, value) in &catalog {
            if !valid_catalog_key(key) {
                return Err("catalog contains an invalid translation key".to_owned());
            }
            if value.trim().is_empty() {
                return Err("catalog contains an empty translation".to_owned());
            }
            if value
                .chars()
                .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
            {
                return Err("catalog contains a control character".to_owned());
            }
        }
        match language {
            UiLanguage::English => self.english = catalog,
            UiLanguage::German => self.german = catalog,
        }
        Ok(())
    }

    pub(crate) fn text<'a>(&'a self, language: UiLanguage, key: &str) -> Cow<'a, str> {
        let selected = match language {
            UiLanguage::English => &self.english,
            UiLanguage::German => &self.german,
        };
        if let Some(value) = selected.get(key) {
            return Cow::Borrowed(value);
        }
        if language == UiLanguage::German {
            if let Some(value) = self.english.get(key) {
                return Cow::Borrowed(value);
            }
        }
        let bundled = text(language, key);
        if bundled != "[missing translation]" {
            return Cow::Borrowed(bundled);
        }
        if let Some(identifier) = key.strip_prefix("identifier.") {
            return Cow::Owned(humanize_identifier(language, identifier));
        }
        Cow::Borrowed(bundled)
    }
}

fn humanize_identifier(language: UiLanguage, identifier: &str) -> String {
    let normalized = if language == UiLanguage::German {
        identifier.to_ascii_lowercase()
    } else {
        identifier.to_owned()
    };
    let words = normalized
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let key = format!("identifier.{part}");
            catalog(language)
                .get(&key)
                .map(String::as_str)
                .unwrap_or(part)
        })
        .collect::<Vec<_>>()
        .join(" ");
    let mut characters = words.chars();
    characters.next().map_or(words.clone(), |first| {
        first.to_uppercase().collect::<String>() + characters.as_str()
    })
}

pub(crate) fn valid_catalog_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 128
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiLevel {
    Learn,
    #[default]
    Work,
}

impl UiLevel {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "learn" => Some(Self::Learn),
            "work" => Some(Self::Work),
            _ => None,
        }
    }
}

type Catalog = HashMap<String, String>;

static ENGLISH: OnceLock<Catalog> = OnceLock::new();
static GERMAN: OnceLock<Catalog> = OnceLock::new();

fn catalog(language: UiLanguage) -> &'static Catalog {
    let (slot, source) = match language {
        UiLanguage::English => (&ENGLISH, include_str!("../locales/en.json")),
        UiLanguage::German => (&GERMAN, include_str!("../locales/de.json")),
    };
    slot.get_or_init(|| {
        serde_json::from_str(source).expect("bundled Zelyra locale file must be valid JSON")
    })
}

pub(crate) fn text(language: UiLanguage, key: &str) -> &'static str {
    catalog(language)
        .get(key)
        .or_else(|| catalog(UiLanguage::English).get(key))
        .map(String::as_str)
        .unwrap_or("[missing translation]")
}

pub(crate) const LOCALE_REFERENCE_START: &str = "\u{e000}zelyra-locale:";
pub(crate) const LOCALE_REFERENCE_PARAMETER: char = '\u{e002}';
pub(crate) const LOCALE_REFERENCE_END: char = '\u{e001}';

pub(crate) fn reference(key: &str) -> String {
    if !valid_catalog_key(key) {
        return "[missing translation]".to_owned();
    }
    format!("{LOCALE_REFERENCE_START}{key}{LOCALE_REFERENCE_END}")
}

pub(crate) fn field_text(_language: UiLanguage, key: &str, field: &str) -> String {
    parameterized_reference(key, "field", field)
}

pub(crate) fn parameterized_reference(key: &str, parameter: &str, value: &str) -> String {
    if !valid_catalog_key(key) || !valid_catalog_key(parameter) {
        return "[missing translation]".to_owned();
    }
    let mut encoded_value = String::with_capacity(value.len() * 2);
    for byte in value.as_bytes() {
        let _ = write!(encoded_value, "{byte:02x}");
    }
    format!(
        "{LOCALE_REFERENCE_START}{key}{LOCALE_REFERENCE_PARAMETER}{parameter}={encoded_value}{LOCALE_REFERENCE_END}"
    )
}

pub(crate) fn framework_text_key(value: &str) -> Option<&'static str> {
    Some(match value {
        "Service Unavailable" => "http.503_short",
        "Internal Server Error" => "http.500_short",
        "400 Bad Request" => "http.400",
        "This request host is not allowed." => "error.host_not_allowed",
        "401 Unauthorized" => "http.401",
        "403 Forbidden" => "http.403",
        "404 Not Found" => "http.404",
        "405 Method Not Allowed" => "http.405",
        "422 Unprocessable Entity" => "http.422",
        "429 Too Many Requests" => "http.429",
        "409 Conflict" => "http.409",
        "500 Internal Server Error" => "http.500",
        "503 Service Unavailable" => "http.503",
        "Authentication is required." => "error.authentication_required",
        "Invalid CSRF token." => "error.invalid_csrf",
        "Email and password are required." => "error.email_password_required",
        "Too many failed login attempts. Try again later." => "error.login_throttled",
        "Invalid credentials." => "error.invalid_credentials",
        "A valid email address is required." => "error.valid_email_required",
        "A valid user ID is required." => "error.valid_user_id_required",
        "A role is required." => "error.role_required",
        "Password must contain at least 8 characters." => "error.password_too_short",
        "Role and permission are required." => "error.role_permission_required",
        "Unknown administration operation." => "error.unknown_admin_operation",
        "The last active administrator cannot be deactivated." => "error.last_active_admin",
        "The last administrator role assignment cannot be removed." => "error.last_admin_role",
        "User activation requires an active column." => "error.active_column_required",
        "The Database capability is not granted." => "error.database_capability_denied",
        "Database is unavailable." => "error.database_unavailable",
        "Page data could not be loaded." => "error.page_data_unavailable",
        "Unknown sort column." => "error.unknown_sort_column",
        "This table view is not sortable." => "error.view_not_sortable",
        "order must be asc or desc." => "error.invalid_sort_order",
        "Unknown filter column." => "error.unknown_filter_column",
        "This table view is not searchable." => "error.view_not_searchable",
        "id must be an integer." => "error.id_must_be_integer",
        "DATABASE_URL is required for login." => "error.database_url_login",
        "DATABASE_URL is required for role administration." => "error.database_url_role_admin",
        "DATABASE_URL is required for CRUD lists." => "error.database_url_crud",
        "DATABASE_URL is required for table views." => "error.database_url_tableview",
        "DATABASE_URL is required for relationship fields." => "error.database_url_relationship",
        "CRUD table is unavailable." => "error.crud_table_unavailable",
        "CRUD table has no columns." => "error.crud_no_columns",
        "DATABASE_URL is required for CRUD details." => "error.database_url_crud_details",
        "DATABASE_URL is required for CRUD actions." => "error.database_url_crud_actions",
        "The action could not be completed." => "error.action_failed",
        _ => return None,
    })
}

#[cfg(test)]
pub(crate) fn framework_text(language: UiLanguage, value: &str) -> Option<String> {
    framework_text_with_catalog(language, value, &ProjectUiCatalogs::default())
}

pub(crate) fn framework_text_with_catalog(
    language: UiLanguage,
    value: &str,
    project: &ProjectUiCatalogs,
) -> Option<String> {
    if let Some(key) = framework_text_key(value) {
        return Some(project.text(language, key).into_owned());
    }
    if let Some(permission) = value.strip_prefix("Missing permission: ") {
        return Some(
            project
                .text(language, "error.missing_permission")
                .replace("{permission}", permission),
        );
    }
    if let Some(column) = value
        .strip_prefix("Unknown filter operator for ")
        .and_then(|value| value.strip_suffix('.'))
    {
        return Some(
            project
                .text(language, "error.unknown_filter_operator")
                .replace("{field}", column),
        );
    }
    if let Some((operator, column)) = value
        .strip_prefix("Operator `")
        .and_then(|value| value.split_once("` is not supported for filter `"))
        .and_then(|(operator, column)| column.strip_suffix("`.").map(|column| (operator, column)))
    {
        return Some(
            project
                .text(language, "error.unsupported_filter_operator")
                .replace("{operator}", operator)
                .replace("{field}", column),
        );
    }
    if let Some(column) = value
        .strip_prefix("Filter `")
        .and_then(|value| value.strip_suffix("` was specified more than once."))
    {
        return Some(
            project
                .text(language, "error.duplicate_filter")
                .replace("{field}", column),
        );
    }
    None
}

pub(crate) fn identifier(language: UiLanguage, name: &str) -> Option<&'static str> {
    let key = format!("identifier.{}", name.to_ascii_lowercase());
    catalog(language).get(&key).map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_files_have_matching_keys_and_nonempty_translations() {
        let english = catalog(UiLanguage::English);
        let german = catalog(UiLanguage::German);
        let mut english_keys = english.keys().collect::<Vec<_>>();
        let mut german_keys = german.keys().collect::<Vec<_>>();
        english_keys.sort();
        german_keys.sort();
        assert_eq!(english_keys, german_keys);
        assert!(english.values().all(|value| !value.trim().is_empty()));
        assert!(german.values().all(|value| !value.trim().is_empty()));
    }

    #[test]
    fn literal_catalog_references_in_web_and_machine_example_exist_in_both_languages() {
        let catalogs = [catalog(UiLanguage::English), catalog(UiLanguage::German)];
        let rust_source = include_str!("lib.rs");
        for marker in ["tr(language, \"", "field_text(language, \""] {
            let mut remaining = rust_source;
            while let Some(index) = remaining.find(marker) {
                remaining = &remaining[index + marker.len()..];
                let key_end = remaining.find('"').expect("catalog key must be closed");
                let key = &remaining[..key_end];
                for translations in catalogs {
                    assert!(translations.contains_key(key), "missing locale key `{key}`");
                }
                remaining = &remaining[key_end + 1..];
            }
        }

        for example in [
            include_str!("../../examples/machine_form.zyl"),
            include_str!("../../examples/mariadb_starter.zyl"),
            include_str!("../../examples/auth.zyl"),
        ] {
            for marker in ["data-zelyra-i18n=\"", "@i18n:"] {
                let mut remaining = example;
                while let Some(index) = remaining.find(marker) {
                    remaining = &remaining[index + marker.len()..];
                    let key_end = remaining.find('"').expect("catalog key must be closed");
                    let key = &remaining[..key_end];
                    for translations in catalogs {
                        assert!(translations.contains_key(key), "missing locale key `{key}`");
                    }
                    remaining = &remaining[key_end + 1..];
                }
            }
        }
    }

    #[test]
    fn language_and_learning_level_accept_only_documented_values() {
        assert_eq!(UiLanguage::parse("de"), Some(UiLanguage::German));
        assert_eq!(UiLanguage::parse("en"), Some(UiLanguage::English));
        assert_eq!(UiLanguage::parse("fr"), None);
        assert_eq!(UiLevel::parse("learn"), Some(UiLevel::Learn));
        assert_eq!(UiLevel::parse("work"), Some(UiLevel::Work));
        assert_eq!(UiLevel::parse("expert"), None);
    }

    #[test]
    fn project_catalogs_validate_values_and_fall_back_to_english() {
        let mut catalogs = ProjectUiCatalogs::default();
        catalogs
            .set_json(
                UiLanguage::English,
                r#"{"custom.title":"Project title","app.home_title":"Home override"}"#,
            )
            .unwrap();
        catalogs
            .set_json(UiLanguage::German, r#"{"custom.title":"Projekttitel"}"#)
            .unwrap();

        assert_eq!(
            catalogs.text(UiLanguage::German, "custom.title"),
            "Projekttitel"
        );
        assert_eq!(
            catalogs.text(UiLanguage::German, "app.home_title"),
            "Home override"
        );
        assert_eq!(
            catalogs.text(UiLanguage::German, "shell.skip_to_content"),
            "Zum Inhalt springen"
        );
        assert!(catalogs
            .set_json(UiLanguage::English, r#"{"label":false}"#)
            .unwrap_err()
            .contains("string values"));
        assert!(catalogs
            .set_json(UiLanguage::English, r#"{"bad key":"value"}"#)
            .unwrap_err()
            .contains("invalid translation key"));
        assert!(catalogs
            .set_json(UiLanguage::English, r#"{"label":"  "}"#)
            .unwrap_err()
            .contains("empty translation"));
    }
}
