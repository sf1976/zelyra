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
    if subcommand == "map" {
        print_privacy_map(&entry_path, &inspect_privacy_inventory(&program), &format);
        return ExitCode::SUCCESS;
    }

    let Ok(schema) = build_schema(&program) else {
        eprintln!("error[E-PRIVACY-002]: cannot derive a privacy plan from an invalid schema");
        return ExitCode::from(1);
    };
    let order = match table_dependency_order(&schema) {
        Ok(order) => order,
        Err(blocked) => {
            let plan = if subcommand == "erase-plan" {
                "gdpr_art_17_erasure"
            } else {
                "gdpr_art_15_20_export"
            };
            if format == "json" {
                println!(
                    "{}",
                    json!({
                        "schema_version": 1,
                        "plan": plan,
                        "entry": entry_path,
                        "evidence_class": "UNVERIFIED",
                        "legal_compliance_evidence": "ASSUMED",
                        "blocking_tables": blocked,
                        "steps": [],
                    })
                );
            }
            eprintln!(
                "error[E-PRIVACY-002]: cyclic foreign-key dependencies require review: {}",
                blocked.join(", ")
            );
            return ExitCode::from(1);
        }
    };
    if subcommand == "export-plan" {
        print_export_plan(&entry_path, &program, &order, &format);
    } else {
        print_erase_plan(&entry_path, &program, &schema, &order, &format);
    }
    ExitCode::SUCCESS
}

fn table_dependency_order(schema: &Schema) -> Result<Vec<&str>, Vec<&str>> {
    let mut pending = BTreeMap::new();
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    for table in &schema.tables {
        let parents = table
            .foreign_keys
            .iter()
            .map(|key| key.referenced_table.as_str())
            .collect::<HashSet<_>>();
        pending.insert(table.name.as_str(), parents.len());
        for parent in parents {
            children.entry(parent).or_default().push(&table.name);
        }
    }

    // Only tables whose parents have been ordered can enter the queue.
    let mut ready = pending
        .iter()
        .filter_map(|(&table, &parents)| (parents == 0).then_some(table))
        .collect::<BTreeSet<_>>();
    let mut ordered = Vec::new();
    while let Some(table) = ready.pop_first() {
        ordered.push(table);
        pending.remove(table);
        for child in children.remove(table).unwrap_or_default() {
            let parents = pending.get_mut(child).expect("child awaits its parent");
            *parents -= 1;
            if *parents == 0 {
                ready.insert(child);
            }
        }
    }
    if pending.is_empty() {
        Ok(ordered)
    } else {
        Err(pending.into_keys().collect())
    }
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

fn print_export_plan(entry_path: &str, program: &Program, order: &[&str], format: &str) {
    let mut steps = Vec::new();
    for table_name in order {
        let Some(table) = program.tables.iter().find(|t| t.name == *table_name) else {
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
            "step": steps.len() + 1,
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
            "execution_evidence": "UNVERIFIED",
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
    schema: &Schema,
    order: &[&str],
    format: &str,
) {
    let mut steps = Vec::new();
    for table_name in order.iter().rev() {
        let Some(table) = program.tables.iter().find(|t| t.name == *table_name) else {
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
            .any(|c| {
                c.guards
                    .iter()
                    .any(|guard| guard.action == zelyra_ast::CrudRecordAction::Delete)
            });
        let dependent_tables = schema
            .tables
            .iter()
            .filter(|child| {
                child
                    .foreign_keys
                    .iter()
                    .any(|key| key.referenced_table == table.name)
            })
            .map(|child| child.name.as_str())
            .collect::<BTreeSet<_>>();
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
        } else if !dependent_tables.is_empty() {
            (
                "review_references_before_erasure",
                "check dependent rows, cascade actions, and retention before erasing the subject's data",
            )
        } else {
            (
                "anonymize_or_delete",
                "review subject identity, retention, and live data before erasure",
            )
        };
        steps.push(json!({
            "step": steps.len() + 1,
            "table": table.name,
            "action": action,
            "reason": note,
            "dependent_tables": dependent_tables,
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
            "execution_evidence": "UNVERIFIED",
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
