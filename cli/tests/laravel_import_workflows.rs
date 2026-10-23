use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

struct TestTree(PathBuf);

impl TestTree {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "zelyra-laravel-import-workflow-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create test directory");
        Self(path)
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().expect("fixture file has a parent"))
            .expect("create fixture directory");
        fs::write(path, content).expect("write fixture source");
    }
}

impl Drop for TestTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn assert_success(output: std::process::Output, operation: &str) {
    assert!(
        output.status.success(),
        "{operation} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn laravel_import_generates_and_compiler_checks_verified_unsubscribe_workflow() {
    let tree = TestTree::new();
    let source = tree.0.join("laravel");
    let output = tree.0.join("zelyra-import");
    fs::create_dir(&source).expect("create Laravel source root");

    let routes = "<?php\nuse App\\Http\\Controllers\\UnsubscribeController;\nRoute::get('/unsubscribe/{token}', [UnsubscribeController::class, 'show']);\nRoute::post('/unsubscribe/{token}', [UnsubscribeController::class, 'unsubscribe']);\n";
    let controller = "<?php\nnamespace App\\Http\\Controllers;\nuse App\\Models\\Suppression;\nclass UnsubscribeController {\n public function show(string $token): View {\n  $record = Suppression::where('token_hash', hash('sha256', $token))->first();\n  abort_unless($record, 404);\n  return view('public.confirm', compact('token'));\n }\n public function unsubscribe(string $token): View {\n  $record = Suppression::where('token_hash', hash('sha256', $token))->first();\n  abort_unless($record, 404);\n  $record->forceFill(['unsubscribed_at' => $record->unsubscribed_at ?: now()])->save();\n  return view('public.done');\n }\n}\n";
    tree.write("laravel/routes/web.php", routes);
    tree.write(
        "laravel/app/Http/Controllers/UnsubscribeController.php",
        controller,
    );
    tree.write(
        "laravel/app/Models/Suppression.php",
        "<?php\nnamespace App\\Models;\nclass Suppression extends Model { protected $table = 'suppressions'; }\n",
    );
    tree.write(
        "laravel/app/Migrations/create_suppressions.php",
        "<?php\nSchema::create('tenants', function (Blueprint $table) { $table->id(); $table->string('name'); });\nSchema::create('suppressions', function (Blueprint $table) {\n $table->id();\n $table->foreignId('tenant_id')->constrained('tenants');\n $table->string('token_hash', 64)->unique();\n $table->timestamp('unsubscribed_at')->nullable();\n});\n",
    );
    tree.write(
        "laravel/resources/views/public/confirm.blade.php",
        "<h1>Abmeldung</h1><p>Keine E-Mails mehr erhalten?</p><button>Abmelden</button>",
    );
    tree.write(
        "laravel/resources/views/public/done.blade.php",
        "<h1>Abgemeldet</h1><p>Die Abmeldung wurde gespeichert.</p>",
    );

    let generated = Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .args([
            "import",
            "laravel",
            "generate",
            source.to_str().expect("UTF-8 test path"),
            "--output",
            output.to_str().expect("UTF-8 test path"),
            "--all-supported",
        ])
        .output()
        .expect("run Laravel importer");
    assert_success(generated, "Laravel scaffold generation");

    let selection_path = output.join("GENERATION_SELECTION.json");
    let selection = fs::read_to_string(selection_path).expect("read generation selection");
    assert!(selection.contains("generated_scoped_public_token_unsubscribe_module"));
    assert!(selection.contains("src/imported_public_unsubscribe.zyl"));
    let workflow_path = output.join("src/imported_public_unsubscribe.zyl");
    let workflow = fs::read_to_string(workflow_path).expect("read generated workflow");
    assert!(workflow.contains("public_token {"));
    assert!(workflow.contains("hash_field: token_hash"));
    assert!(workflow.contains("timestamp_field: unsubscribed_at"));
    assert!(workflow.contains("require_match"));
    assert!(!workflow.contains("UPDATE suppressions"));
    let schema = fs::read_to_string(output.join("src/imported_schema.zyl"))
        .expect("read generated scoped schema");
    assert!(schema.contains("table suppressions scoped by tenant_id: TenantId"));

    let checked = Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .args([
            "check",
            output.join("main.zyl").to_str().expect("UTF-8 test path"),
        ])
        .output()
        .expect("run compiler on generated scaffold");
    assert_success(checked, "generated scaffold compiler check");

    assert_eq!(
        fs::read_to_string(source.join("routes/web.php")).unwrap(),
        routes
    );
    assert_eq!(
        fs::read_to_string(source.join("app/Http/Controllers/UnsubscribeController.php")).unwrap(),
        controller
    );
    assert!(!contains_php_file(&output));
}

#[test]
fn selected_laravel_tables_include_scalar_foreign_key_dependencies_and_compile() {
    let tree = TestTree::new();
    let source = tree.0.join("laravel");
    let output = tree.0.join("zelyra-import");
    fs::create_dir(&source).expect("create Laravel source root");
    tree.write(
        "laravel/database/migrations/create_business_tables.php",
        "<?php\nSchema::create('users', function (Blueprint $table) { $table->id(); $table->string('name'); });\nSchema::create('tenants', function (Blueprint $table) { $table->id(); $table->foreignId('user_id')->nullable(); $table->foreign('user_id')->references('id')->on('users')->nullOnDelete(); $table->string('name'); });\nSchema::create('customers', function (Blueprint $table) { $table->id(); $table->foreignId('tenant_id'); $table->foreign('tenant_id')->references('id')->on('tenants')->cascadeOnDelete(); $table->string('name'); });\n",
    );

    let generated = Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .args([
            "import",
            "laravel",
            "generate",
            source.to_str().expect("UTF-8 test path"),
            "--output",
            output.to_str().expect("UTF-8 test path"),
            "--table",
            "customers",
        ])
        .output()
        .expect("run targeted Laravel importer");
    assert_success(generated, "targeted Laravel scaffold generation");

    let selection: serde_json::Value = serde_json::from_slice(
        &fs::read(output.join("GENERATION_SELECTION.json")).expect("read selection report"),
    )
    .expect("decode selection report");
    assert_eq!(selection["schema_version"], "2");
    assert_eq!(
        selection["mode"],
        "requested_tables_with_dependency_closure"
    );
    assert_eq!(selection["requested_tables"][0], "customers");
    assert_eq!(
        selection["included_tables"],
        serde_json::json!(["customers", "tenants", "users"])
    );
    assert!(
        selection["relation_dependencies"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("tenants"))
            || selection["foreign_key_dependencies"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("tenants"))
    );

    let checked = Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .args([
            "check",
            output.join("main.zyl").to_str().expect("UTF-8 test path"),
        ])
        .output()
        .expect("compile generated dependency-complete scaffold");
    assert_success(checked, "selected table scaffold compiler check");
}

fn contains_php_file(directory: &Path) -> bool {
    let Ok(entries) = fs::read_dir(directory) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let path = entry.path();
        if path.is_dir() {
            contains_php_file(&path)
        } else {
            path.extension().is_some_and(|extension| extension == "php")
        }
    })
}
