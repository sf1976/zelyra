//! Compares the security surface of two independently checked project entries.
use super::*;
use zelyra_ast::{Expr, FormAction, PrivacyClass, Program, Span};
use zelyra_proof::Rule;
use zelyra_runtime::{capability_set_satisfies, parse_capability_spec, ParsedCapability};

#[derive(Default)]
struct Surface {
    capabilities: BTreeMap<String, Vec<String>>,
    privacy: BTreeMap<String, (PrivacyClass, bool)>,
    requirements: BTreeMap<String, BTreeSet<String>>,
    alternatives: BTreeMap<String, BTreeSet<String>>,
    guarantees: BTreeMap<(String, Rule, usize), u8>,
    project_grants: Vec<String>,
    execution_policy: Option<String>,
}

#[derive(Default)]
struct Changes(BTreeSet<(String, String)>);

impl Changes {
    fn add(&mut self, kind: &str, subject: &str) {
        self.0.insert((kind.into(), subject.into()));
    }
}

pub(super) fn command(arguments: &[String]) -> ExitCode {
    let (before, after, strict) = match arguments {
        [before, after] => (before, after, false),
        [before, after, flag] if flag == "--fail-on-privilege-escalation" => (before, after, true),
        _ => {
            usage();
            return ExitCode::from(2);
        }
    };
    let result =
        surface(before).and_then(|before| surface(after).map(|after| compare(&before, &after)));
    match result {
        Ok(changes) => {
            let findings = changes
                .0
                .iter()
                .map(|(kind, subject)| json!({"kind": kind, "subject": subject}))
                .collect::<Vec<_>>();
            let escalated = !findings.is_empty();
            print_machine_document(&json!({
                "schema_version": 1,
                "status": if escalated { "review_required" } else { "pass" },
                "privilege_escalation": escalated,
                "findings": findings,
            }));
            if strict && escalated {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            print_machine_document(
                &json!({"schema_version": 1, "status": "unverified", "privilege_escalation": null, "findings": []}),
            );
            eprintln!("error[E-DIFF-001]: {error}");
            ExitCode::FAILURE
        }
    }
}

fn surface(entry: &str) -> Result<Surface, String> {
    let loaded = load_project(entry).map_err(|()| "cannot load comparison input")?;
    let sources = loaded.sources.clone();
    let program = validate_loaded_project(entry, loaded)
        .map_err(|()| "comparison input did not pass validation")?;
    let mut surface = Surface {
        project_grants: project_capability_grants(entry)?.map_or_else(
            || {
                KNOWN_CAPABILITIES
                    .iter()
                    .map(|name| (*name).into())
                    .collect()
            },
            |grants| grants.into_iter().collect(),
        ),
        execution_policy: execution_policy(entry)?,
        ..Surface::default()
    };
    collect_declarations(&mut surface, &program, &sources)?;
    for result in verify_program(&program) {
        let rule = proof_cli::proof_rule(result.kind);
        let level = match result.status {
            VerificationStatus::Proven => 2,
            VerificationStatus::RuntimeCheck => 1,
            VerificationStatus::Unproven => 0,
            VerificationStatus::Failed => {
                return Err(format!(
                    "comparison input has a failed verifier rule: {} ({rule:?})",
                    result.function
                ))
            }
        };
        // Several loop contracts may use the same local index. Compare the
        // weakest observation for that key and retain every explicit clause.
        surface
            .guarantees
            .entry((result.function.clone(), rule, result.index))
            .and_modify(|previous| *previous = (*previous).min(level))
            .or_insert(level);
        if rule == Rule::Invariant {
            let key = format!("function:{}", result.function);
            add_span(
                &mut surface.requirements,
                &key,
                "invariant",
                result.span,
                &sources,
            )?;
        }
    }
    Ok(surface)
}

fn execution_policy(entry: &str) -> Result<Option<String>, String> {
    let Some(path) = project_config_path(entry)? else {
        return Ok(None);
    };
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let config: toml::Value = toml::from_str(&text).map_err(|_| "invalid project TOML")?;
    let settings = [
        "filesystem",
        "network",
        "process",
        "mail",
        "runtime",
        "features",
        "module",
        "modules",
        "dependencies",
        "package_sources",
        "package_provenance",
    ]
    .into_iter()
    .filter_map(|key| config.get(key).map(|value| (key, value)))
    .collect::<BTreeMap<_, _>>();
    if settings.is_empty() {
        return Ok(None);
    }
    serde_json::to_vec(&settings)
        .map(|bytes| Some(zelyra_proof::sha256(&bytes)))
        .map_err(|_| "invalid project policy".into())
}

fn collect_declarations(
    surface: &mut Surface,
    program: &Program,
    sources: &[project::ProjectSource],
) -> Result<(), String> {
    for function in &program.functions {
        let key = format!("function:{}", function.name);
        surface
            .capabilities
            .insert(key.clone(), function.capabilities.clone());
        if function.is_public && !function.capabilities.is_empty() {
            surface
                .capabilities
                .insert(format!("public-{key}"), function.capabilities.clone());
        }
        for parameter in &function.params {
            surface.privacy.insert(
                format!("{key}:{}", parameter.name),
                (parameter.privacy, false),
            );
        }
        for (kind, rules) in [
            ("requires", &function.requires),
            ("ensures", &function.ensures),
        ] {
            for rule in rules {
                add_expression(&mut surface.requirements, &key, kind, rule, sources)?;
            }
        }
    }
    for record in &program.records {
        for field in &record.fields {
            surface.privacy.insert(
                format!("record:{}.{}", record.name, field.name),
                (field.privacy, field.redact),
            );
        }
    }
    for table in &program.tables {
        let key = format!("table:{}", table.name);
        let rules = surface.requirements.entry(key.clone()).or_default();
        if let Some(scope) = &table.tenant_scope {
            rules.insert(format!("tenant:{}:{}", scope.column, scope.tenant_type));
        }
        if table.is_ledger {
            rules.insert("append-only".into());
        }
        if table.is_auditable {
            rules.insert("auditable".into());
        }
        for column in &table.columns {
            surface.privacy.insert(
                format!("{key}.{}", column.name),
                (column.privacy, column.redact),
            );
        }
        for validation in &table.validations {
            add_expression(
                &mut surface.requirements,
                &key,
                "validation",
                &validation.condition,
                sources,
            )?;
        }
        for machine in &table.state_machines {
            for transition in &machine.transitions {
                let machine_key = format!("{key}.{}:{}", machine.column, transition.from);
                surface
                    .alternatives
                    .entry(machine_key.clone())
                    .or_default()
                    .extend(
                        transition
                            .to
                            .iter()
                            .map(|target| format!("target:{target}")),
                    );
                for to in &transition.to {
                    let edge = format!("{machine_key}->{to}");
                    let rules = surface.requirements.entry(edge.clone()).or_default();
                    if let Some(column) = &transition.four_eyes_column {
                        rules.insert(format!("four_eyes:{column}"));
                    }
                    if !transition.required_roles.is_empty() {
                        rules.insert("requires_role".into());
                        surface.alternatives.insert(
                            edge.clone(),
                            transition.required_roles.iter().cloned().collect(),
                        );
                    }
                    if let Some(guard) = &transition.guard {
                        add_expression(
                            &mut surface.requirements,
                            &edge,
                            "guard",
                            &guard.condition,
                            sources,
                        )?;
                    }
                }
            }
        }
    }
    for api in &program.apis {
        let key = format!("api:{} {}", api.method, api.path);
        let rules = auth_requirements(surface, &key, api.requires_auth, &api.permissions);
        if let Some(table) = &api.validates_table {
            rules.insert(format!("validates:{table}"));
        }
        if let Some(limit) = &api.rate_limit {
            rules.insert(format!(
                "rate_limit:{}/{}",
                limit.requests, limit.window_seconds
            ));
        }
        if let Some(handler) = &api.handler {
            if let Some(capabilities) = surface.capabilities.get(&format!("function:{handler}")) {
                surface.capabilities.insert(key, capabilities.clone());
            }
        }
    }
    for page in &program.pages {
        auth_requirements(
            surface,
            &format!("page:{}", page.path),
            page.requires_auth,
            &page.permissions,
        );
    }
    for view in &program.tableviews {
        auth_requirements(
            surface,
            &format!("tableview:{}", view.name),
            view.requires_auth,
            &view.permissions,
        );
    }
    for form in &program.forms {
        let key = format!("form:{}", form.path.as_deref().unwrap_or(&form.name));
        if form.public_token.is_some() {
            surface
                .requirements
                .entry(key.clone())
                .or_default()
                .insert("public_token".into());
        }
        if let Some(expr) = &form.verify {
            add_expression(&mut surface.requirements, &key, "verify", expr, sources)?;
        }
        collect_actions(surface, &key, &form.actions, sources)?;
    }
    for crud in &program.cruds {
        let key = format!("crud:{}", crud.name);
        let rules = auth_requirements(surface, &key, crud.requires_auth, &crud.permissions);
        if let Some(column) = &crud.tenant_column {
            rules.insert(format!("tenant:{column}"));
        }
        if let Some(column) = &crud.recipient_column {
            rules.insert(format!("recipient:{column}"));
        }
        for (operation, permissions) in [
            ("create", &crud.create_permissions),
            ("edit", &crud.edit_permissions),
            ("delete", &crud.delete_permissions),
        ] {
            let permissions = if permissions.is_empty() {
                &crud.permissions
            } else {
                permissions
            };
            auth_requirements(
                surface,
                &format!("{key}:{operation}"),
                crud.requires_auth,
                permissions,
            );
        }
        for condition in &crud.conditional_permissions {
            add_expression(
                &mut surface.requirements,
                &key,
                &format!("condition:{:?}:{}", condition.action, condition.permission),
                &condition.condition,
                sources,
            )?;
        }
        for guard in &crud.guards {
            add_expression(
                &mut surface.requirements,
                &key,
                &format!("guard:{:?}", guard.action),
                &guard.condition,
                sources,
            )?;
        }
        collect_actions(surface, &key, &crud.actions, sources)?;
    }
    for definition in &program.auth {
        let mut definition = definition.clone();
        definition.span = Span::default();
        surface
            .requirements
            .entry(format!("auth:{}", definition.name))
            .or_default()
            .insert(zelyra_proof::sha256(format!("{definition:?}").as_bytes()));
    }
    for workflow in &program.workflows {
        let key = format!("workflow:{}", workflow.name);
        surface
            .requirements
            .entry(key.clone())
            .or_default()
            .insert(format!("{}.{}", workflow.table, workflow.column));
        surface
            .alternatives
            .entry(key)
            .or_default()
            .extend(workflow.timers.iter().map(|timer| {
                format!(
                    "{}->{}@{}",
                    timer.state, timer.target, timer.duration_seconds
                )
            }));
        for hook in &workflow.hooks {
            surface.capabilities.insert(
                format!(
                    "workflow:{}:{:?}:{:?}",
                    workflow.name, hook.kind, hook.state
                ),
                hook.capabilities.clone(),
            );
        }
    }
    Ok(())
}

fn collect_actions(
    surface: &mut Surface,
    parent: &str,
    actions: &[FormAction],
    sources: &[project::ProjectSource],
) -> Result<(), String> {
    for action in actions {
        let key = format!("{parent}:action:{}", action.name);
        let rules = auth_requirements(surface, &key, action.requires_auth, &action.permissions);
        if action.require_match {
            rules.insert("require_match".into());
        }
        if let Some(guard) = &action.guard {
            add_expression(
                &mut surface.requirements,
                &key,
                "guard",
                &guard.condition,
                sources,
            )?;
        }
        surface
            .capabilities
            .insert(key, action.capabilities.clone());
    }
    Ok(())
}

fn auth_requirements<'a>(
    surface: &'a mut Surface,
    key: &str,
    auth: bool,
    permissions: &[String],
) -> &'a mut BTreeSet<String> {
    let rules = surface.requirements.entry(key.into()).or_default();
    if auth {
        rules.insert("requires_auth".into());
    }
    rules.extend(
        permissions
            .iter()
            .map(|permission| format!("permission:{permission}")),
    );
    rules
}

fn add_expression(
    requirements: &mut BTreeMap<String, BTreeSet<String>>,
    key: &str,
    rule: &str,
    expr: &Expr,
    sources: &[project::ProjectSource],
) -> Result<(), String> {
    add_span(requirements, key, rule, expr.span, sources)
}

fn add_span(
    requirements: &mut BTreeMap<String, BTreeSet<String>>,
    key: &str,
    rule: &str,
    span: Span,
    sources: &[project::ProjectSource],
) -> Result<(), String> {
    let text = sources
        .get(span.source_id as usize)
        .and_then(|source| source.text.get(span.start..span.end))
        .ok_or("expression span is outside its source file")?;
    let tokens = lex(text).map_err(|_| "cannot normalize a rule expression")?;
    let kinds = tokens
        .into_iter()
        .map(|token| token.kind)
        .collect::<Vec<_>>();
    let digest = zelyra_proof::sha256(format!("{kinds:?}").as_bytes());
    requirements
        .entry(key.into())
        .or_default()
        .insert(format!("{rule}:{digest}"));
    Ok(())
}

fn privacy_strength(class: PrivacyClass) -> u8 {
    match class {
        PrivacyClass::Public => 0,
        PrivacyClass::Pii => 1,
        PrivacyClass::Secret => 2,
    }
}

fn capability_expanded(before: &[String], after: &[String]) -> bool {
    after.iter().any(|capability| {
        // Compare each resource independently so merging two equally scoped
        // declarations into one declaration preserves the same privilege.
        let entries = match parse_capability_spec(capability) {
            Some(ParsedCapability::DatabaseReadTables(values)) => {
                split_scoped("Database(read: [", values)
            }
            Some(ParsedCapability::DatabaseWriteTables(values)) => {
                split_scoped("Database(write: [", values)
            }
            Some(ParsedCapability::NetworkAllow(values)) => {
                split_scoped("Network(allow: [", values)
            }
            Some(ParsedCapability::EmailTemplates(values)) => {
                split_scoped("Email(templates: [", values)
            }
            _ => vec![capability.clone()],
        };
        entries
            .iter()
            .any(|entry| !capability_set_satisfies(before.iter().map(String::as_str), entry))
    })
}

fn split_scoped(prefix: &str, values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| format!("{prefix}{value}])"))
        .collect()
}

fn compare(before: &Surface, after: &Surface) -> Changes {
    let mut changes = Changes::default();
    for (subject, scope) in &after.capabilities {
        let previous = before
            .capabilities
            .get(subject)
            .map_or(&[][..], Vec::as_slice);
        if capability_expanded(previous, scope) {
            changes.add("capability_expanded", subject);
        }
    }
    if capability_expanded(&before.project_grants, &after.project_grants) {
        changes.add("capability_expanded", "project_grants");
    }
    if before.execution_policy != after.execution_policy {
        changes.add("execution_policy_changed", "project_configuration");
    }
    for (subject, &(old_class, old_redact)) in &before.privacy {
        let (class, redact) = after.privacy.get(subject).copied().unwrap_or_default();
        if privacy_strength(class) < privacy_strength(old_class) || (old_redact && !redact) {
            changes.add("classification_weakened", subject);
        }
    }
    for (subject, rules) in &before.requirements {
        if !rules.is_empty()
            && after
                .requirements
                .get(subject)
                .is_none_or(|current| !rules.is_subset(current))
        {
            changes.add("boundary_weakened", subject);
        }
    }
    for (subject, alternatives) in &after.alternatives {
        if before
            .alternatives
            .get(subject)
            .is_some_and(|previous| !alternatives.is_subset(previous))
        {
            changes.add("boundary_expanded", subject);
        }
    }
    for (key, level) in &before.guarantees {
        if after.guarantees.get(key).copied().unwrap_or_default() < *level {
            changes.add("verification_weakened", &key.0);
        }
    }
    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scopes(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).into()).collect()
    }

    #[test]
    fn scoped_capabilities_compare_members_and_path_components() {
        assert!(!capability_expanded(
            &scopes(&["Network(allow: [a.example])", "Network(allow: [b.example])"]),
            &scopes(&["Network(allow: [b.example, a.example])"]),
        ));
        assert!(capability_expanded(
            &scopes(&["Network(allow: [a.example])"]),
            &scopes(&["Network(allow: [a.example, b.example])"]),
        ));
        assert!(!capability_expanded(
            &scopes(&["FileSystem(scope: exports/)"]),
            &scopes(&["FileSystem(scope: exports/reports/)"]),
        ));
        assert!(capability_expanded(
            &scopes(&["FileSystem(scope: exports/)"]),
            &scopes(&["FileSystem(scope: exportstore/)"]),
        ));
    }
}
