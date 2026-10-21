use super::PhpSourceMap;
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path},
};

/// Evidence that a Laravel application declares and connects a conventional
/// login page, POST handler, and controller action. This is deliberately a
/// narrow structural detector; it does not inspect credentials or execute PHP.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct LoginEvidence {
    pub(super) controller_file: String,
    pub(super) login_action_method: String,
    pub(super) view_file: String,
    pub(super) route_file: String,
}

/// Recognize only a narrow local approval guard in the statically linked
/// login action. This does not claim equivalent messages, admin exceptions,
/// registration, or MFA behavior.
pub(super) fn detect_login_enabled_column(root: &Path, evidence: &LoginEvidence) -> Option<String> {
    let relative = Path::new(&evidence.controller_file);
    if relative
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return None;
    }
    let root = root.canonicalize().ok()?;
    let source_path = root.join(relative).canonicalize().ok()?;
    if !source_path.starts_with(&root) || !source_path.is_file() {
        return None;
    }
    let source = fs::read_to_string(source_path).ok()?;
    let source_map = PhpSourceMap::new(&source);
    let code = String::from_utf8(
        source
            .bytes()
            .enumerate()
            .map(|(offset, byte)| {
                if source_map.is_code(offset) || byte == b'\n' {
                    byte
                } else {
                    b' '
                }
            })
            .collect(),
    )
    .ok()?;
    let method_pattern = regex::Regex::new(&format!(
        r"\bfunction\s+{}\s*\([^)]*\)[^{{}}]*\{{",
        regex::escape(&evidence.login_action_method)
    ))
    .ok()?;
    let method_start = method_pattern.find(&code)?;
    let open_brace = method_start.end().checked_sub(1)?;
    let mut depth = 0usize;
    let mut method_end = None;
    for (offset, byte) in code.bytes().enumerate().skip(open_brace) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    method_end = Some(offset + 1);
                    break;
                }
            }
            _ => {}
        }
    }
    let method = &code[method_start.start()..method_end?];
    let guard = regex::Regex::new(
        r"(?s)if\s*\(\s*!\s*\$user\s*->\s*([A-Za-z_][A-Za-z0-9_]*)\s*&&.{0,500}?ValidationException::withMessages",
    )
    .ok()?;
    let captures = guard.captures(method)?;
    Some(captures.get(1)?.as_str().to_owned())
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct TenantAuthMapping {
    pub(super) membership_table: Option<String>,
    pub(super) owner_table: Option<String>,
    pub(super) owner_column: Option<String>,
}

/// Map tenant ownership only from resolved foreign-key metadata in the
/// generated schema candidates. A conventional pivot is accepted only when
/// both its user and tenant relations are explicit and unambiguous.
pub(super) fn detect_tenant_auth_mapping(candidates: &[Value]) -> TenantAuthMapping {
    let owner_candidates = candidates
        .iter()
        .filter(|candidate| candidate["table"] == "tenants")
        .filter_map(|candidate| {
            let columns = candidate["columns"].as_array()?;
            let owner = columns.iter().find(|column| {
                column["relation_status"] == "resolved"
                    && column["relation_table"] == "users"
                    && column["source_column_name"]
                        .as_str()
                        .is_some_and(|name| name == "user_id" || name == "user")
            })?;
            Some((
                candidate["table"].as_str()?.to_owned(),
                owner["source_column_name"].as_str()?.to_owned(),
            ))
        })
        .collect::<Vec<_>>();
    let owner = match owner_candidates.as_slice() {
        [owner] => Some(owner.clone()),
        _ => None,
    };

    let membership_candidates = candidates
        .iter()
        .filter_map(|candidate| {
            let table = candidate["table"].as_str()?;
            // Laravel's conventional pivot is explicit evidence. A business
            // table can also point at both users and tenants; that alone is
            // not enough to treat it as an authentication membership table.
            if !matches!(
                table,
                "tenant_user" | "tenant_users" | "tenant_membership" | "tenant_memberships"
            ) {
                return None;
            }
            let columns = candidate["columns"].as_array()?;
            let has_user = columns.iter().any(|column| {
                column["relation_status"] == "resolved"
                    && column["relation_table"] == "users"
                    && column["source_column_name"] == "user_id"
            });
            let has_tenant = columns.iter().any(|column| {
                column["relation_status"] == "resolved"
                    && column["relation_table"] == "tenants"
                    && column["source_column_name"] == "tenant_id"
            });
            (has_user && has_tenant).then(|| table.to_owned())
        })
        .collect::<Vec<_>>();
    let membership_table = match membership_candidates.as_slice() {
        [table] => Some(table.clone()),
        _ => None,
    };

    TenantAuthMapping {
        membership_table,
        owner_table: owner.as_ref().map(|(table, _)| table.clone()),
        owner_column: owner.map(|(_, column)| column),
    }
}

pub(super) fn detect_login_module(report: &Value) -> Option<LoginEvidence> {
    let edges = report["graph"]["edges"].as_array()?;
    let route_action = |route: &str, action: &str| {
        edges.iter().find(|edge| {
            edge["kind"] == "route_controller_action"
                && edge["status"] == "resolved"
                && edge["from"]["name"] == route
                && edge["to"]["kind"] == "controller_action"
                && edge["to"]["name"]
                    .as_str()
                    .is_some_and(|name| name.ends_with(action))
        })
    };

    let get_route = route_action("get /login", "@showLogin")?;
    let post_route = route_action("post /login", "@login")?;
    let controller_file = get_route["to"]["file"].as_str()?;
    if controller_file != post_route["to"]["file"].as_str()? {
        return None;
    }
    let show_login_action = get_route["to"]["name"].as_str()?;
    let login_action_method = post_route["to"]["name"].as_str()?.split_once('@')?.1;
    if !login_action_method
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return None;
    }
    let view_edge = edges.iter().find(|edge| {
        edge["kind"] == "controller_view"
            && edge["status"] == "resolved"
            && edge["from"]["name"] == show_login_action
            && (edge["to"]["name"] == "auth.login"
                || edge["to"]["file"] == "resources/views/auth/login.blade.php")
    })?;

    Some(LoginEvidence {
        controller_file: controller_file.to_owned(),
        login_action_method: login_action_method.to_owned(),
        view_file: view_edge["to"]["file"].as_str()?.to_owned(),
        route_file: get_route["from"]["file"].as_str()?.to_owned(),
    })
}

/// Project a Laravel users table into the native Zelyra authentication schema.
/// The source project contains schema only: credentials and password hashes are
/// never copied. Laravel's remember token is replaced by Zelyra's hashed,
/// persistent session table.
pub(super) fn render_native_auth_module(
    users: &Value,
    session_table: &str,
    tenant_mapping: &TenantAuthMapping,
    tenant_tables: &[Value],
    login_enabled_column: Option<&str>,
) -> Result<String, String> {
    if !is_identifier(session_table) {
        return Err("generated authentication session table name is invalid".into());
    }
    let source = users["candidate_source"]
        .as_str()
        .ok_or("users candidate has no generated source")?;
    if !source.starts_with("table users {") {
        return Err("native authentication requires the selected `users` table".into());
    }

    let mut saw_id = false;
    let mut saw_email = false;
    let mut saw_password = false;
    let mut saw_password_hash = false;
    let mut user_id_storage = None;
    let mut projected = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("remember_token:") {
            continue;
        }
        if let Some((field, declaration)) = trimmed.split_once(':') {
            let field = field.trim();
            let declaration = declaration.trim();
            let id_attributes = declaration.split_whitespace().collect::<Vec<_>>();
            saw_id |= field == "id"
                && id_attributes.first() == Some(&"Id")
                && id_attributes.contains(&"primary")
                && id_attributes.contains(&"auto");
            if field == "id" {
                user_id_storage = id_attributes
                    .windows(2)
                    .find(|pair| pair[0] == "unsigned_storage")
                    .map(|pair| pair[1]);
            }
            saw_email |= field == "email"
                && (declaration.starts_with("String") || declaration.starts_with("Email"))
                && declaration.contains("required")
                && declaration.contains("unique");
            if field == "password" {
                if !declaration.starts_with("String") || !declaration.contains("required") {
                    return Err("Laravel `users.password` is not a required string field".into());
                }
                saw_password = true;
                projected.push(line.replacen("password:", "password_hash:", 1));
                continue;
            }
            if field == "password_hash" {
                saw_password_hash =
                    declaration.starts_with("String") && declaration.contains("required");
            }
        }
        projected.push(line.to_owned());
    }
    if !saw_id || !saw_email {
        return Err("native authentication requires an auto-incrementing `id` and a required unique `email`".into());
    }
    if saw_password == saw_password_hash {
        return Err("native authentication requires exactly one required string field named `password` or `password_hash`".into());
    }
    if saw_password_hash {
        // A Laravel password column is framework-owned even when a second
        // password_hash field exists. Never silently keep both identities.
        if source
            .lines()
            .any(|line| line.trim().starts_with("password:"))
        {
            return Err("users table contains both `password` and `password_hash`; review the identity mapping".into());
        }
    }
    if let Some(column) = login_enabled_column {
        let required_boolean = source.lines().any(|line| {
            line.trim()
                .strip_prefix(&format!("{column}:"))
                .is_some_and(|declaration| declaration.trim_start().starts_with("Bool required"))
        });
        if !required_boolean {
            return Err(format!(
                "login approval column `{column}` is not a required boolean in the generated users table"
            ));
        }
    }

    let mut output = String::from(
        "# Native Zelyra authentication model projected from a statically linked Laravel login module.\n# User records and credentials are not copied. Imported password hashes must be reset or deliberately converted before accounts can authenticate.\n\n",
    );
    output.push_str(&projected.join("\n"));
    let mut appended_tables = BTreeSet::new();
    for candidate in tenant_tables {
        let table = candidate["table"]
            .as_str()
            .ok_or("tenant schema candidate has no table name")?;
        if !tenant_mapping.references_table(table) {
            return Err(format!(
                "tenant schema candidate `{table}` is not referenced by the detected auth mapping"
            ));
        }
        if !appended_tables.insert(table) {
            return Err(format!("tenant schema candidate `{table}` is duplicated"));
        }
        let source = candidate["candidate_source"]
            .as_str()
            .ok_or_else(|| format!("tenant schema candidate `{table}` has no generated source"))?;
        output.push_str("\n\n");
        output.push_str(source);
    }
    let user_storage = user_id_storage
        .map(|storage| format!(" unsigned_storage {storage}"))
        .unwrap_or_default();
    output.push_str(&format!(
        "\n\ntable {session_table} {{\n    id: Id primary auto\n    user: User required{user_storage}\n    token_hash: String(64) required unique\n    expires_at: Timestamp required\n}}\n\nauth users {{\n    table: users\n    sessions: {session_table}\n"
    ));
    if let Some(membership_table) = tenant_mapping.membership_table.as_deref() {
        output.push_str(&format!("    memberships: {membership_table}\n"));
    }
    if let (Some(owner_table), Some(owner_column)) = (
        tenant_mapping.owner_table.as_deref(),
        tenant_mapping.owner_column.as_deref(),
    ) {
        output.push_str(&format!("    tenant_owner: {owner_table}.{owner_column}\n"));
    }
    if let Some(column) = login_enabled_column {
        output.push_str(&format!("    login_enabled: {column}\n"));
    }
    output.push_str("}\n");
    Ok(output)
}

impl TenantAuthMapping {
    pub(super) fn references_table(&self, table: &str) -> bool {
        self.membership_table.as_deref() == Some(table)
            || self.owner_table.as_deref() == Some(table)
            || (self.membership_table.is_some() && table == "tenants")
    }
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn users_candidate(source: &str) -> Value {
        json!({ "candidate_source": source })
    }

    #[test]
    fn detects_login_only_when_routes_share_a_controller_and_resolve_to_the_login_view() {
        let report = json!({
            "graph": { "edges": [
                {"kind":"route_controller_action", "status":"resolved", "from":{"name":"get /login", "file":"routes/web.php"}, "to":{"kind":"controller_action", "name":"AuthController@showLogin", "file":"app/AuthController.php"}},
                {"kind":"route_controller_action", "status":"resolved", "from":{"name":"post /login", "file":"routes/web.php"}, "to":{"kind":"controller_action", "name":"AuthController@login", "file":"app/AuthController.php"}},
                {"kind":"controller_view", "status":"resolved", "from":{"name":"AuthController@showLogin"}, "to":{"name":"auth.login", "file":"resources/views/auth/login.blade.php"}}
            ]}
        });
        let detected = detect_login_module(&report).unwrap();
        assert_eq!(detected.controller_file, "app/AuthController.php");
        assert_eq!(detected.route_file, "routes/web.php");
        assert_eq!(detected.view_file, "resources/views/auth/login.blade.php");
    }

    #[test]
    fn does_not_infer_native_login_from_a_view_without_resolved_routes() {
        let report = json!({
            "graph": { "edges": [
                {"kind":"controller_view", "status":"resolved", "from":{"name":"AuthController@showLogin"}, "to":{"name":"auth.login", "file":"resources/views/auth/login.blade.php"}}
            ]}
        });
        assert!(detect_login_module(&report).is_none());
    }

    #[test]
    fn detects_only_a_source_backed_boolean_login_approval_guard() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-login-approval-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let controller = root.join("app/Http/Controllers/Auth/AuthController.php");
        std::fs::create_dir_all(controller.parent().unwrap()).unwrap();
        std::fs::write(
            &controller,
            "public function login(Request $request) { if (!$user->is_approved && strcasecmp($user->email, ADMIN_EMAIL) !== 0) { throw ValidationException::withMessages([]); } }",
        )
        .unwrap();
        let evidence = LoginEvidence {
            controller_file: "app/Http/Controllers/Auth/AuthController.php".into(),
            login_action_method: "login".into(),
            view_file: "resources/views/auth/login.blade.php".into(),
            route_file: "routes/web.php".into(),
        };
        assert_eq!(
            detect_login_enabled_column(&root, &evidence).as_deref(),
            Some("is_approved")
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn does_not_infer_approval_from_comments_or_string_literals() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-login-approval-decoy-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let controller = root.join("app/AuthController.php");
        std::fs::create_dir_all(controller.parent().unwrap()).unwrap();
        std::fs::write(
            &controller,
            "public function login() { } public function register() { if (!$user->is_approved && true) { ValidationException::withMessages([]); } }",
        )
        .unwrap();
        let evidence = LoginEvidence {
            controller_file: "app/AuthController.php".into(),
            login_action_method: "login".into(),
            view_file: "resources/views/auth/login.blade.php".into(),
            route_file: "routes/web.php".into(),
        };
        assert_eq!(detect_login_enabled_column(&root, &evidence), None);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn projects_password_and_discards_laravel_remember_token() {
        let source = "table users {\n    id: Id primary auto\n    email: String(255) required unique\n    password: String(255) required\n    remember_token: String(100)?\n}";
        let generated = render_native_auth_module(
            &users_candidate(source),
            "zelyra_auth_sessions",
            &TenantAuthMapping::default(),
            &[],
            None,
        )
        .unwrap();
        assert!(generated.contains("password_hash: String(255) required"));
        assert!(!generated.contains("password:"));
        assert!(!generated.contains("remember_token"));
        assert!(generated
            .contains("auth users {\n    table: users\n    sessions: zelyra_auth_sessions"));
    }

    #[test]
    fn imports_approval_guard_only_when_the_boolean_user_field_is_present() {
        let source = "table users {\n    id: Id primary auto\n    email: String(255) required unique\n    password: String(255) required\n    is_approved: Bool required default true\n}";
        let generated = render_native_auth_module(
            &users_candidate(source),
            "zelyra_auth_sessions",
            &TenantAuthMapping::default(),
            &[],
            Some("is_approved"),
        )
        .unwrap();
        assert!(generated.contains("login_enabled: is_approved"));
        assert!(render_native_auth_module(
            &users_candidate(&source.replace("is_approved: Bool", "is_approved: String")),
            "zelyra_auth_sessions",
            &TenantAuthMapping::default(),
            &[],
            Some("is_approved"),
        )
        .is_err());
    }

    #[test]
    fn refuses_ambiguous_or_incompatible_user_identity_fields() {
        for source in [
            "table users {\n    id: Id primary auto\n    email: String(255) required unique\n}",
            "table users {\n    id: Id primary auto\n    email: String(255) required unique\n    password: String(255) required\n    password_hash: String(255) required\n}",
            "table users {\n    id: Id primary auto\n    email: String(255) required\n    password: String(255) required\n}",
        ] {
            assert!(render_native_auth_module(
                &users_candidate(source),
                "auth_sessions",
                &TenantAuthMapping::default(),
                &[],
                None,
            )
            .is_err());
        }
    }

    #[test]
    fn detects_tenant_ownership_and_pivot_only_from_resolved_schema_relations() {
        let candidates = vec![
            json!({
                "table":"tenants",
                "candidate_source":"table tenants { id: Id primary auto user: User? }",
                "columns":[
                    {"name":"user", "source_column_name":"user_id", "relation_status":"resolved", "relation_table":"users"}
                ]
            }),
            json!({
                "table":"tenant_user",
                "candidate_source":"table tenant_user { id: Id primary auto tenant: Tenant required user: User required }",
                "columns":[
                    {"name":"tenant", "source_column_name":"tenant_id", "relation_status":"resolved", "relation_table":"tenants"},
                    {"name":"user", "source_column_name":"user_id", "relation_status":"resolved", "relation_table":"users"}
                ]
            }),
        ];
        assert_eq!(
            detect_tenant_auth_mapping(&candidates),
            TenantAuthMapping {
                membership_table: Some("tenant_user".into()),
                owner_table: Some("tenants".into()),
                owner_column: Some("user_id".into()),
            }
        );
        let generated = render_native_auth_module(
            &users_candidate(
                "table users {\n id: Id primary auto\n email: String(255) required unique\n password: String(255) required\n}",
            ),
            "zelyra_auth_sessions",
            &detect_tenant_auth_mapping(&candidates),
            &candidates[0..2],
            None,
        )
        .unwrap();
        assert!(generated.contains("memberships: tenant_user"));
        assert!(generated.contains("tenant_owner: tenants.user_id"));
        assert!(generated.contains("table tenants {"));
        assert!(generated.contains("table tenant_user {"));
    }

    #[test]
    fn does_not_guess_ambiguous_tenant_membership_or_owner_relations() {
        let candidates = vec![
            json!({"table":"tenants", "columns":[
                {"source_column_name":"user_id", "relation_status":"resolved", "relation_table":"users"}
            ]}),
            json!({"table":"tenant_user", "columns":[
                {"source_column_name":"user_id", "relation_status":"resolved", "relation_table":"users"},
                {"source_column_name":"tenant_id", "relation_status":"resolved", "relation_table":"tenants"}
            ]}),
            json!({"table":"tenant_users", "columns":[
                {"source_column_name":"user_id", "relation_status":"resolved", "relation_table":"users"},
                {"source_column_name":"tenant_id", "relation_status":"resolved", "relation_table":"tenants"}
            ]}),
            json!({"table":"cash_book_entries", "columns":[
                {"source_column_name":"user_id", "relation_status":"resolved", "relation_table":"users"},
                {"source_column_name":"tenant_id", "relation_status":"resolved", "relation_table":"tenants"}
            ]}),
        ];
        assert_eq!(
            detect_tenant_auth_mapping(&candidates),
            TenantAuthMapping {
                membership_table: None,
                owner_table: Some("tenants".into()),
                owner_column: Some("user_id".into()),
            }
        );
    }
}
