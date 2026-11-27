use super::*;
use zelyra_ast::PrivacyClass;
use zelyra_runtime::inspect_privacy_inventory;

pub(super) fn privacy_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(subcommand) = args.next() else {
        usage();
        return ExitCode::from(2);
    };
    if !matches!(subcommand.as_str(), "map" | "export-plan" | "erase-plan") {
        usage();
        return ExitCode::from(2);
    }

    let mut path: Option<String> = None;
    let mut format = "human".to_owned();
    while let Some(arg) = args.next() {
        if let Some(value) = arg.strip_prefix("--format=") {
            format = value.to_owned();
        } else if arg == "--format" {
            let Some(value) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            format = value;
        } else if arg.starts_with('-') || path.is_some() {
            usage();
            return ExitCode::from(2);
        } else {
            path = Some(arg);
        }
    }
    let Some(entry_path) = path else {
        usage();
        return ExitCode::from(2);
    };
    let valid_format = match subcommand.as_str() {
        "map" => matches!(format.as_str(), "human" | "json" | "md"),
        _ => matches!(format.as_str(), "human" | "json"),
    };
    if !valid_format {
        usage();
        return ExitCode::from(2);
    }

    let program = match validate(&entry_path) {
        Ok(program) => program,
        Err(()) => return ExitCode::from(1),
    };
    let inventory = inspect_privacy_inventory(&program);

    match subcommand.as_str() {
        "map" => print_privacy_map(&entry_path, &inventory, &format),
        "export-plan" => print_export_plan(&entry_path, &program, &inventory, &format),
        "erase-plan" => print_erase_plan(&entry_path, &program, &inventory, &format),
        _ => unreachable!(),
    }
    ExitCode::SUCCESS
}

fn table_dependency_order(program: &Program) -> Vec<String> {
    let mut names = program
        .tables
        .iter()
        .map(|t| t.name.clone())
        .collect::<Vec<_>>();
    names.sort();
    let table_set = names
        .iter()
        .map(|n| n.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let mut deps: HashMap<String, Vec<String>> = HashMap::new();
    for t in &program.tables {
        let mut refs = Vec::new();
        for c in &t.columns {
            if let Some(target) = &c.references {
                let low = target.to_ascii_lowercase();
                if table_set.contains(&low) && !low.eq_ignore_ascii_case(&t.name) {
                    if let Some(canonical) = program
                        .tables
                        .iter()
                        .find(|cand| cand.name.eq_ignore_ascii_case(target))
                        .map(|cand| cand.name.clone())
                    {
                        refs.push(canonical);
                    }
                }
            }
        }
        refs.sort();
        refs.dedup();
        deps.insert(t.name.clone(), refs);
    }
    // Topological sort: parent (referenced) tables first, dependent child tables after.
    let mut visited = HashSet::new();
    let mut visiting = HashSet::new();
    let mut ordered = Vec::new();
    fn visit(
        name: &str,
        deps: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
        ordered: &mut Vec<String>,
    ) {
        if visited.contains(name) || visiting.contains(name) {
            return;
        }
        visiting.insert(name.to_owned());
        if let Some(parents) = deps.get(name) {
            for p in parents {
                visit(p, deps, visited, visiting, ordered);
            }
        }
        visiting.remove(name);
        visited.insert(name.to_owned());
        ordered.push(name.to_owned());
    }
    for name in &names {
        visit(name, &deps, &mut visited, &mut visiting, &mut ordered);
    }
    ordered
}

fn print_privacy_map(entry_path: &str, inventory: &zelyra_runtime::PrivacyInventory, format: &str) {
    if format == "json" {
        let payload = json!({
            "schema_version": 1,
            "entry": entry_path,
            "evidence_class": "PROVEN",
            "enforcement_class": "ENFORCED",
            "legal_compliance_evidence": "ASSUMED",
            "scope_boundaries": [
                "Covers statically declared @pii/@secret/@redact columns, record fields, routes, and Network/Email/FileSystem capabilities visible in Zelyra source.",
                "External databases, third-party SaaS processors, backups, and organizational GDPR processes are outside static compiler verification (ASSUMED)."
            ],
            "columns": inventory.columns.iter().map(|c| json!({
                "table": c.table,
                "column": c.column,
                "type": c.ty,
                "privacy": c.privacy.as_str(),
                "redact": c.redact,
                "tenant_scoped": c.tenant_scoped,
                "is_ledger": c.is_ledger,
                "is_auditable": c.is_auditable,
                "has_delete_guard": c.has_delete_guard,
                "foreign_keys": c.foreign_keys,
            })).collect::<Vec<_>>(),
            "record_fields": inventory.record_fields.iter().map(|f| json!({
                "record": f.record,
                "field": f.field,
                "type": f.ty,
                "privacy": f.privacy.as_str(),
                "redact": f.redact,
            })).collect::<Vec<_>>(),
            "routes": inventory.routes.iter().map(|r| json!({
                "kind": r.kind,
                "target": r.target,
                "requires_auth": r.requires_auth,
                "permissions": r.permissions,
                "privacy_level": r.privacy_level.as_str(),
                "tables": r.tables,
            })).collect::<Vec<_>>(),
            "external_sinks": inventory.external_sinks.iter().map(|s| json!({
                "function": s.function,
                "capability": s.capability,
            })).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&payload).unwrap());
        return;
    }

    if format == "md" {
        println!("# Zelyra Privacy & Data-Flow Map\n");
        println!("- Entry: `{entry_path}`");
        println!("- Static Catalog Evidence: `PROVEN` | Leak Prevention: `ENFORCED` | Legal Compliance: `ASSUMED`\n");
        println!("## Sensitive Table Columns\n");
        println!("| Table | Column | Type | Privacy | Redact | Scoped | Ledger/Auditable |");
        println!("| --- | --- | --- | --- | --- | --- | --- |");
        for c in &inventory.columns {
            println!(
                "| {} | {} | {} | @{} | {} | {} | {}/{} |",
                c.table,
                c.column,
                c.ty,
                c.privacy.as_str(),
                c.redact,
                c.tenant_scoped,
                c.is_ledger,
                c.is_auditable
            );
        }
        println!("\n## Routes Touching Sensitive Data\n");
        for r in &inventory.routes {
            println!(
                "- [{}] `{}` (auth={}, max_privacy={})",
                r.kind,
                r.target,
                r.requires_auth,
                r.privacy_level.as_str()
            );
        }
        println!("\n## External Sinks\n");
        for s in &inventory.external_sinks {
            println!("- `{}` uses `{}`", s.function, s.capability);
        }
        return;
    }

    println!("Zelyra Privacy Map ({entry_path}) [PROVEN catalog / ENFORCED E-PRIVACY-001 / ASSUMED legal]");
    println!("Columns ({}):", inventory.columns.len());
    for c in &inventory.columns {
        println!(
            "  - {}.{} : {} [@{}{}]",
            c.table,
            c.column,
            c.ty,
            c.privacy.as_str(),
            if c.redact { ", @redact" } else { "" }
        );
    }
    println!("Record fields ({}):", inventory.record_fields.len());
    for f in &inventory.record_fields {
        println!(
            "  - {}.{} : {} [@{}{}]",
            f.record,
            f.field,
            f.ty,
            f.privacy.as_str(),
            if f.redact { ", @redact" } else { "" }
        );
    }
    println!("Routes ({}):", inventory.routes.len());
    for r in &inventory.routes {
        println!(
            "  - {} {} (auth={}, privacy={})",
            r.kind,
            r.target,
            r.requires_auth,
            r.privacy_level.as_str()
        );
    }
    println!("External sinks ({}):", inventory.external_sinks.len());
    for s in &inventory.external_sinks {
        println!("  - {} -> {}", s.function, s.capability);
    }
}

fn print_export_plan(
    entry_path: &str,
    program: &Program,
    _inventory: &zelyra_runtime::PrivacyInventory,
    format: &str,
) {
    let order = table_dependency_order(program);
    let mut steps = Vec::new();
    for (idx, table_name) in order.iter().enumerate() {
        let Some(table) = program.tables.iter().find(|t| &t.name == table_name) else {
            continue;
        };
        let mut pii_cols = table
            .columns
            .iter()
            .filter(|c| c.privacy == PrivacyClass::Pii)
            .map(|c| c.name.clone())
            .collect::<Vec<_>>();
        pii_cols.sort();
        if pii_cols.is_empty() {
            continue;
        }
        let mut secret_excluded = table
            .columns
            .iter()
            .filter(|c| c.privacy == PrivacyClass::Secret)
            .map(|c| c.name.clone())
            .collect::<Vec<_>>();
        secret_excluded.sort();
        steps.push(json!({
            "step": idx + 1,
            "table": table.name,
            "tenant_column": table.tenant_scope.as_ref().map(|s| s.column.clone()),
            "exported_pii_columns": pii_cols,
            "excluded_secret_columns": secret_excluded,
            "is_auditable": table.is_auditable,
        }));
    }

    if format == "json" {
        let payload = json!({
            "schema_version": 1,
            "plan": "gdpr_art_15_20_export",
            "entry": entry_path,
            "evidence_class": "PROVEN",
            "legal_compliance_evidence": "ASSUMED",
            "assumptions": [
                "Secret columns (@secret) are excluded from subject exports by policy.",
                "Subject identity resolution across tenants and external archives is operator-verified (ASSUMED)."
            ],
            "steps": steps,
        });
        println!("{}", serde_json::to_string_pretty(&payload).unwrap());
        return;
    }

    println!("Zelyra GDPR Export Plan ({entry_path}) [PROVEN schema plan / ASSUMED legal]");
    for step in &steps {
        println!(
            "  {}. {} -> export PII {:?} (exclude secrets {:?})",
            step["step"],
            step["table"].as_str().unwrap_or(""),
            step["exported_pii_columns"],
            step["excluded_secret_columns"]
        );
    }
}

fn print_erase_plan(
    entry_path: &str,
    program: &Program,
    _inventory: &zelyra_runtime::PrivacyInventory,
    format: &str,
) {
    // Reverse FK order: children before referenced parents.
    let mut order = table_dependency_order(program);
    order.reverse();
    let mut steps = Vec::new();
    for table_name in &order {
        let Some(table) = program.tables.iter().find(|t| &t.name == table_name) else {
            continue;
        };
        let mut sensitive_cols = table
            .columns
            .iter()
            .filter(|c| c.privacy != PrivacyClass::Public || c.redact)
            .map(|c| format!("{}:@{}", c.name, c.privacy.as_str()))
            .collect::<Vec<_>>();
        sensitive_cols.sort();
        if sensitive_cols.is_empty() {
            continue;
        }
        let has_delete_guard = program
            .cruds
            .iter()
            .filter(|c| c.table == table.name)
            .any(|c| !c.guards.is_empty());
        let (action, note) = if table.is_ledger {
            (
                "retain_immutable_ledger",
                "ledger table is append-only (E-LEDGER-001 forbids UPDATE/DELETE); retain ledger rows for statutory retention and pseudonymize external references",
            )
        } else if table.is_auditable {
            (
                "review_legal_retention_and_anonymize",
                "auditable bitemporal table tracks historical validity; verify statutory retention before anonymizing subject fields",
            )
        } else if has_delete_guard {
            (
                "respect_delete_guard_or_anonymize",
                "CRUD delete guard constrains deletion; check active guard conditions or anonymize columns",
            )
        } else {
            (
                "anonymize_or_delete",
                "no ledger lock or delete guard detected; safe to anonymize or delete in child-first foreign-key order",
            )
        };
        steps.push(json!({
            "step": steps.len() + 1,
            "table": table.name,
            "action": action,
            "reason": note,
            "columns": sensitive_cols,
            "tenant_column": table.tenant_scope.as_ref().map(|s| s.column.clone()),
            "is_ledger": table.is_ledger,
            "is_auditable": table.is_auditable,
            "has_delete_guard": has_delete_guard,
        }));
    }

    if format == "json" {
        let payload = json!({
            "schema_version": 1,
            "plan": "gdpr_art_17_erasure",
            "entry": entry_path,
            "evidence_class": "PROVEN",
            "legal_compliance_evidence": "ASSUMED",
            "assumptions": [
                "Statutory commercial/tax retention periods (e.g. GoBD/HGB for ledgers and invoices) take legal precedence and remain ASSUMED.",
                "Offline backups and external replicated targets must follow operator retention policies (ASSUMED)."
            ],
            "steps": steps,
        });
        println!("{}", serde_json::to_string_pretty(&payload).unwrap());
        return;
    }

    println!("Zelyra GDPR Erase Plan ({entry_path}) [PROVEN dependency order / ASSUMED retention]");
    for step in &steps {
        println!(
            "  {}. {} -> {} ({})",
            step["step"],
            step["table"].as_str().unwrap_or(""),
            step["action"].as_str().unwrap_or(""),
            step["reason"].as_str().unwrap_or("")
        );
    }
}
