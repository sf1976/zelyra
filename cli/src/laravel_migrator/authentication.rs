use serde_json::Value;

/// Evidence that a Laravel application declares and connects a conventional
/// login page, POST handler, and controller action. This is deliberately a
/// narrow structural detector; it does not inspect credentials or execute PHP.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct LoginEvidence {
    pub(super) controller_file: String,
    pub(super) view_file: String,
    pub(super) route_file: String,
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
    let view_edge = edges.iter().find(|edge| {
        edge["kind"] == "controller_view"
            && edge["status"] == "resolved"
            && edge["from"]["name"] == show_login_action
            && (edge["to"]["name"] == "auth.login"
                || edge["to"]["file"] == "resources/views/auth/login.blade.php")
    })?;

    Some(LoginEvidence {
        controller_file: controller_file.to_owned(),
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
    let mut projected = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("remember_token:") {
            continue;
        }
        if let Some((field, declaration)) = trimmed.split_once(':') {
            let field = field.trim();
            let declaration = declaration.trim();
            saw_id |= field == "id" && declaration == "Id primary auto";
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

    let mut output = String::from(
        "# Native Zelyra authentication model projected from a statically linked Laravel login module.\n# User records and credentials are not copied. Imported password hashes must be reset or deliberately converted before accounts can authenticate.\n\n",
    );
    output.push_str(&projected.join("\n"));
    output.push_str(&format!(
        "\n\ntable {session_table} {{\n    id: Id primary auto\n    user: User required\n    token_hash: String(64) required unique\n    expires_at: Timestamp required\n}}\n\nauth users {{\n    table: users\n    sessions: {session_table}\n}}\n"
    ));
    Ok(output)
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
    fn projects_password_and_discards_laravel_remember_token() {
        let source = "table users {\n    id: Id primary auto\n    email: String(255) required unique\n    password: String(255) required\n    remember_token: String(100)?\n}";
        let generated =
            render_native_auth_module(&users_candidate(source), "zelyra_auth_sessions").unwrap();
        assert!(generated.contains("password_hash: String(255) required"));
        assert!(!generated.contains("password:"));
        assert!(!generated.contains("remember_token"));
        assert!(generated
            .contains("auth users {\n    table: users\n    sessions: zelyra_auth_sessions"));
    }

    #[test]
    fn refuses_ambiguous_or_incompatible_user_identity_fields() {
        for source in [
            "table users {\n    id: Id primary auto\n    email: String(255) required unique\n}",
            "table users {\n    id: Id primary auto\n    email: String(255) required unique\n    password: String(255) required\n    password_hash: String(255) required\n}",
            "table users {\n    id: Id primary auto\n    email: String(255) required\n    password: String(255) required\n}",
        ] {
            assert!(render_native_auth_module(&users_candidate(source), "auth_sessions").is_err());
        }
    }
}
