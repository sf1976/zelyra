use super::*;
use reedline::{
    Completer, CompletionResult, DefaultCompleter, FileBackedHistory, Highlighter, History,
    HistoryItem, HistoryItemId, Prompt, PromptEditMode, PromptHistorySearch, Reedline, SearchQuery,
    Signal, StyledText, ValidationResult, Validator,
};
use std::{
    borrow::Cow,
    io::{self, BufRead, IsTerminal},
    sync::{Arc, Mutex},
};

pub(super) fn command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let mut entry = "main.zyl".to_owned();
    let mut entry_set = false;
    let mut sandbox = true;
    let mut tenant: Option<i64> = None;
    let mut backend = "ast";
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--sandbox" => sandbox = true,
            "--persist" => sandbox = false,
            "--tenant" => {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --tenant requires an integer id");
                    return ExitCode::from(2);
                };
                tenant = match value.parse::<i64>() {
                    Ok(value) if value >= 0 => Some(value),
                    _ => {
                        eprintln!("error[E-CLI-001]: --tenant requires a non-negative integer id");
                        return ExitCode::from(2);
                    }
                };
            }
            "--backend" => {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --backend requires ast or bytecode");
                    return ExitCode::from(2);
                };
                backend = if value == "ast" {
                    "ast"
                } else if value == "bytecode" {
                    "bytecode"
                } else {
                    eprintln!("error[E-CLI-001]: unknown console backend `{value}`; expected ast or bytecode");
                    return ExitCode::from(2);
                };
            }
            value if value.starts_with('-') => {
                eprintln!("error[E-CLI-001]: unknown console option `{value}`");
                return ExitCode::from(2);
            }
            value if !entry_set => {
                entry = value.to_owned();
                entry_set = true;
            }
            value => {
                eprintln!("error[E-CLI-001]: unexpected console argument `{value}`");
                return ExitCode::from(2);
            }
        }
    }
    let program = match validate(&entry) {
        Ok(program) => program,
        Err(()) => return ExitCode::from(1),
    };
    let project_directory = Path::new(&entry)
        .parent()
        .filter(|directory| !directory.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let database_name = program
        .databases
        .first()
        .map(|database| database.name.as_str());
    let database_url = match database_url_for_project(project_directory, database_name) {
        Ok(url) => url,
        Err(error) => {
            eprintln!("error[E-ENV-001]: {error}");
            return ExitCode::from(1);
        }
    };
    let grants = match project_capability_grants(&entry) {
        Ok(grants) => grants,
        Err(error) => {
            eprintln!("error[E-CAP-002]: {error}");
            return ExitCode::from(1);
        }
    };
    let policy = match project_runtime_policy(&entry) {
        Ok(policy) => policy,
        Err(error) => {
            eprintln!("error[E-POLICY-002]: {error}");
            return ExitCode::from(1);
        }
    };
    let mut session =
        match zelyra_runtime::ReplSession::new(program, database_url, grants, policy, sandbox) {
            Ok(session) => session,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
        };
    if let Some(tenant_id) = tenant {
        session.set_tenant_id(tenant_id);
    }
    let interactive = io::stdin().is_terminal();
    if interactive {
        println!(
            "Zelyra language console ({}, {} mode). Type :help for commands.",
            backend.to_ascii_uppercase(),
            if sandbox { "sandbox" } else { "persistent" }
        );
    }
    let mut continuation = String::new();
    let mut exit_code = ExitCode::SUCCESS;
    if interactive {
        let words = Arc::new(Mutex::new(session.completion_candidates()));
        let history_path = env::var_os("HOME")
            .or_else(|| env::var_os("USERPROFILE"))
            .map(|home| PathBuf::from(home).join(".zelyra_history"));
        let history = history_path.as_ref().and_then(|path| {
            FileBackedHistory::with_file(10_000, path.clone())
                .ok()
                .map(|history| Box::new(FilteredHistory(history)) as Box<dyn History>)
        });
        let mut editor = Reedline::create()
            .with_completer(Box::new(ConsoleCompleter(words.clone())))
            .with_highlighter(Box::new(ConsoleHighlighter))
            .with_validator(Box::new(ConsoleValidator));
        if let Some(history) = history {
            editor = editor.with_history(history);
        }
        if let Some(path) = &history_path {
            #[cfg(unix)]
            if path.exists() {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
            }
        }
        let prompt = ConsolePrompt;
        loop {
            if let Ok(mut candidates) = words.lock() {
                *candidates = session.completion_candidates();
            }
            match editor.read_line(&prompt) {
                Ok(Signal::Success(line)) => {
                    // Reedline reports deliberately filtered history writes via this slot.
                    // Consume it so a skipped secret never leaves stale editor state.
                    let _ = editor.take_history_save_error();
                    match handle_meta(&line, &mut session, &entry, &mut exit_code) {
                        Some(true) => break,
                        Some(false) => continue,
                        None => {}
                    }
                    evaluate(&line, &mut session, backend);
                }
                Ok(Signal::CtrlC) => println!("^C"),
                Ok(Signal::CtrlD) => break,
                Ok(Signal::ExternalBreak(_)) | Ok(Signal::HostCommand(_)) => {}
                Ok(_) => {}
                Err(error) => {
                    eprintln!("error[E-REPL-002]: {error}");
                    exit_code = ExitCode::from(1);
                    break;
                }
            }
        }
        let _ = editor.sync_history();
        if let Some(path) = history_path {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
            }
        }
    } else {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(line) => line,
                Err(error) => {
                    eprintln!("error[E-REPL-002]: {error}");
                    exit_code = ExitCode::from(1);
                    break;
                }
            };
            if continuation.is_empty() {
                match handle_meta(&line, &mut session, &entry, &mut exit_code) {
                    Some(true) => break,
                    Some(false) => continue,
                    None => {}
                }
            }
            continuation.push_str(&line);
            continuation.push('\n');
            if input_complete(&continuation) {
                evaluate(&continuation, &mut session, backend);
                continuation.clear();
            }
        }
        if !continuation.trim().is_empty() {
            eprintln!("error[E-PARSE-001]: incomplete input at end of file");
            exit_code = ExitCode::from(1);
        }
    }
    drop(session); // sandbox transaction rolls back here unless manually committed
    exit_code
}

/// Bytecode execution normally routes queries through `SystemHostServices`.
/// The REPL sandbox decorates that provider so query and batch calls share the
/// session's existing MariaDB connection and cannot escape its rollback.
struct ReplHostServices {
    inner: Arc<dyn HostServices>,
    transaction: Arc<Mutex<zelyra_database::MariaDbInteractiveTransaction>>,
}

impl HostServices for ReplHostServices {
    fn unix_time_millis(&self) -> Result<i64, String> {
        self.inner.unix_time_millis()
    }

    fn environment_variable(&self, name: &str) -> Result<Option<String>, String> {
        self.inner.environment_variable(name)
    }

    fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
        self.inner.fill_random(output)
    }

    fn read_console_line(&self, prompt: &str) -> Result<Option<String>, String> {
        self.inner.read_console_line(prompt)
    }

    fn http_request(
        &self,
        method: &str,
        url: &str,
        headers: &[(String, String)],
        body: Option<&[u8]>,
        timeout_ms: u64,
        max_response_bytes: usize,
    ) -> Result<HostHttpResponse, String> {
        self.inner
            .http_request(method, url, headers, body, timeout_ms, max_response_bytes)
    }

    fn deliver_outbox_message(
        &self,
        event_id: &str,
        payload_json: &str,
        mail: &MailPolicy,
        network: &NetworkPolicy,
    ) -> Result<(), String> {
        self.inner
            .deliver_outbox_message(event_id, payload_json, mail, network)
    }

    fn run_process(
        &self,
        command: &str,
        arguments: &[String],
        timeout_ms: u64,
        max_output_bytes: usize,
    ) -> Result<String, String> {
        self.inner
            .run_process(command, arguments, timeout_ms, max_output_bytes)
    }

    fn canonicalize_path(&self, path: &Path, allow_missing: bool) -> Result<PathBuf, String> {
        self.inner.canonicalize_path(path, allow_missing)
    }

    fn file_metadata(&self, path: &Path) -> Result<HostFileMetadata, String> {
        self.inner.file_metadata(path)
    }

    fn read_text(&self, path: &Path) -> Result<String, String> {
        self.inner.read_text(path)
    }

    fn write_text(&self, path: &Path, content: &str) -> Result<(), String> {
        self.inner.write_text(path, content)
    }

    fn write_text_new(&self, path: &Path, content: &str) -> Result<(), String> {
        self.inner.write_text_new(path, content)
    }

    fn atomic_write_text(&self, path: &Path, content: &str) -> Result<(), String> {
        self.inner.atomic_write_text(path, content)
    }

    fn append_text(&self, path: &Path, content: &str) -> Result<(), String> {
        self.inner.append_text(path, content)
    }

    fn copy_file_new(&self, source: &Path, destination: &Path) -> Result<(), String> {
        self.inner.copy_file_new(source, destination)
    }

    fn rename_file_new(&self, source: &Path, destination: &Path) -> Result<(), String> {
        self.inner.rename_file_new(source, destination)
    }

    fn remove_file(&self, path: &Path) -> Result<(), String> {
        self.inner.remove_file(path)
    }

    fn list_directory(&self, path: &Path) -> Result<Vec<String>, String> {
        self.inner.list_directory(path)
    }

    fn execute_database_query(
        &self,
        _database_url: &str,
        query: &Query,
    ) -> Result<QueryResult, String> {
        self.transaction
            .lock()
            .map_err(|_| "sandbox database transaction is unavailable".to_owned())?
            .execute(query)
            .map_err(|error| error.to_string())
    }

    fn execute_database_batch(
        &self,
        _database_url: &str,
        queries: &[Query],
        _transaction: bool,
    ) -> Result<Vec<QueryResult>, String> {
        self.transaction
            .lock()
            .map_err(|_| "sandbox database transaction is unavailable".to_owned())?
            .execute_batch(queries)
            .map_err(|error| error.to_string())
    }

    fn validate_database_storage(&self, database_url: &str, schema: &Schema) -> Result<(), String> {
        self.inner.validate_database_storage(database_url, schema)
    }
}

struct ConsoleCompleter(Arc<Mutex<Vec<String>>>);
impl Completer for ConsoleCompleter {
    fn complete(&mut self, line: &str, position: usize) -> CompletionResult {
        let mut completer = DefaultCompleter::default();
        if let Ok(words) = self.0.lock() {
            completer.insert(words.clone());
        }
        completer.complete(line, position)
    }
}

struct ConsoleHighlighter;
impl Highlighter for ConsoleHighlighter {
    fn highlight(&self, line: &str, _: usize) -> StyledText {
        let mut styled = StyledText::new();
        let rendered = if env::var_os("NO_COLOR").is_some()
            || env::var("ZELYRA_COLOR").is_ok_and(|mode| mode.eq_ignore_ascii_case("never"))
        {
            line.to_owned()
        } else {
            colorize_input(line)
        };
        styled.push((nu_ansi_term::Style::new(), rendered));
        styled
    }
}

struct ConsoleValidator;
impl Validator for ConsoleValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        if input_complete(line) {
            ValidationResult::Complete
        } else {
            ValidationResult::Incomplete
        }
    }
}

struct ConsolePrompt;
impl Prompt for ConsolePrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
    fn render_prompt_indicator(&self, _: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed(">> ")
    }
    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("... ")
    }
    fn render_prompt_history_search_indicator(&self, _: PromptHistorySearch) -> Cow<'_, str> {
        Cow::Borrowed("history search: ")
    }
}

struct FilteredHistory(FileBackedHistory);
impl History for FilteredHistory {
    fn save(&mut self, item: HistoryItem) -> reedline::Result<HistoryItem> {
        if history_safe(&item.command_line) {
            self.0.save(item)
        } else {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "sensitive input omitted from history",
            )
            .into())
        }
    }
    fn load(&self, id: HistoryItemId) -> reedline::Result<HistoryItem> {
        self.0.load(id)
    }
    fn count(&self, query: SearchQuery) -> reedline::Result<i64> {
        self.0.count(query)
    }
    fn search(&self, query: SearchQuery) -> reedline::Result<Vec<HistoryItem>> {
        self.0.search(query)
    }
    fn update(
        &mut self,
        id: HistoryItemId,
        updater: &dyn Fn(HistoryItem) -> HistoryItem,
    ) -> reedline::Result<()> {
        self.0.update(id, updater)
    }
    fn clear(&mut self) -> reedline::Result<()> {
        self.0.clear()
    }
    fn delete(&mut self, id: HistoryItemId) -> reedline::Result<()> {
        self.0.delete(id)
    }
    fn sync(&mut self) -> io::Result<()> {
        self.0.sync()
    }
    fn session(&self) -> Option<reedline::HistorySessionId> {
        self.0.session()
    }
}

fn colorize_input(line: &str) -> String {
    let mut output = String::with_capacity(line.len() + 32);
    let mut chars = line.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '"' {
            output.push_str("\x1b[32m\"");
            let mut escaped = false;
            for next in chars.by_ref() {
                output.push(next);
                if escaped {
                    escaped = false;
                } else if next == '\\' {
                    escaped = true;
                } else if next == '"' {
                    break;
                }
            }
            output.push_str("\x1b[0m");
        } else if character.is_ascii_digit() {
            output.push_str("\x1b[33m");
            output.push(character);
            while chars.peek().is_some_and(char::is_ascii_digit) {
                output.push(chars.next().expect("peeked digit"));
            }
            output.push_str("\x1b[0m");
        } else if character.is_ascii_alphabetic() || character == '_' {
            let mut word = String::from(character);
            while chars
                .peek()
                .is_some_and(|next| next.is_ascii_alphanumeric() || *next == '_')
            {
                word.push(chars.next().expect("peeked identifier character"));
            }
            if matches!(
                word.as_str(),
                "fn" | "let"
                    | "mutable"
                    | "if"
                    | "else"
                    | "match"
                    | "for"
                    | "while"
                    | "return"
                    | "true"
                    | "false"
                    | "sql"
                    | "transaction"
                    | "uses"
            ) {
                output.push_str("\x1b[36m");
                output.push_str(&word);
                output.push_str("\x1b[0m");
            } else {
                output.push_str(&word);
            }
        } else {
            output.push(character);
        }
    }
    output
}
fn handle_meta(
    line: &str,
    session: &mut zelyra_runtime::ReplSession,
    entry: &str,
    exit_code: &mut ExitCode,
) -> Option<bool> {
    let command = line.trim();
    if !command.starts_with(':') {
        return None;
    }
    let mut parts = command.split_whitespace();
    match parts.next().unwrap_or("") {
        ":help" => println!(
            ":help :tables :schema <table> :tenant <id> :commit :rollback :clear :quit/:exit"
        ),
        ":tables" => {
            for table in session.table_names() {
                println!("{table}");
            }
        }
        ":schema" => match parts
            .next()
            .and_then(|name| session.schema_description(name))
        {
            Some(schema) => println!("{schema}"),
            None => println!("unknown table; inspect {} for project schemas", entry),
        },
        ":tenant" => match parts
            .next()
            .and_then(|id| id.parse::<i64>().ok())
            .filter(|id| *id >= 0)
        {
            Some(id) => {
                session.set_tenant_id(id);
                println!("tenant_id = {id}");
            }
            None => println!("error[E-REPL-001]: usage: :tenant <non-negative integer id>"),
        },
        ":commit" => report_meta(session.commit()),
        ":rollback" => report_meta(session.rollback()),
        ":clear" => print!("\x1b[2J\x1b[H"),
        ":quit" | ":exit" => return Some(true),
        _ => println!("error[E-REPL-001]: unknown console command; use :help"),
    }
    let _ = exit_code;
    Some(false)
}

fn report_meta(result: Result<(), zelyra_runtime::ReplError>) {
    match result {
        Ok(()) => println!("ok"),
        Err(error) => println!("{error}"),
    }
}

fn evaluate(input: &str, session: &mut zelyra_runtime::ReplSession, backend: &str) {
    match session.eval_line_with_executor(input, |context| execute_repl_backend(context, backend)) {
        Ok(result) => {
            for line in result.output {
                println!("{line}");
            }
            if let Some(value) = result.value {
                println!("=> {}", pretty_value(&value, 0));
            }
        }
        Err(error) => eprintln!("{error}"),
    }
}

fn execute_repl_backend(
    context: zelyra_runtime::ReplExecutionContext<'_>,
    backend: &str,
) -> Result<zelyra_runtime::FunctionExecutionOutcome, zelyra_runtime::ReplError> {
    if backend == "ast" {
        let outcome =
            if let (Some(url), Some(transaction)) = (context.database_url, context.transaction) {
                zelyra_runtime::execute_function_captured_with_mariadb_transaction(
                    context.program,
                    context.function_name,
                    context.arguments,
                    url,
                    context.grants,
                    context.policy,
                    transaction,
                )
            } else {
                zelyra_runtime::execute_function_captured_with_capabilities_and_policies(
                    context.program,
                    context.function_name,
                    context.arguments,
                    context.database_url,
                    context.grants,
                    context.policy,
                )
            };
        return Ok(outcome);
    }

    let mir = zelyra_mir::lower(context.program).map_err(|errors| {
        let error = errors
            .into_iter()
            .next()
            .expect("lowering returned an error");
        zelyra_runtime::ReplError::Backend {
            code: error.code.to_owned(),
            message: error.message,
            span: error.span,
        }
    })?;
    let bytecode =
        zelyra_mir::bytecode::compile(&mir, context.function_name).map_err(|errors| {
            let error = errors
                .into_iter()
                .next()
                .expect("bytecode compilation returned an error");
            zelyra_runtime::ReplError::Backend {
                code: error.code.to_owned(),
                message: error.message,
                span: error.span,
            }
        })?;
    let limits = zelyra_mir::execution::ExecutionLimits {
        max_steps: context
            .policy
            .and_then(|policy| policy.step_budget)
            .unwrap_or(100_000),
        max_stack_depth: context
            .policy
            .and_then(|policy| policy.max_stack_depth)
            .unwrap_or(20),
    };
    let host_services: Arc<dyn HostServices> = match context.transaction.clone() {
        Some(transaction) => Arc::new(ReplHostServices {
            inner: Arc::new(SystemHostServices),
            transaction,
        }),
        None => Arc::new(SystemHostServices),
    };
    let outcome = if let Some(url) = context.database_url {
        zelyra_mir::bytecode::execute_with_database_and_host_services_and_policy_captured(
            &bytecode,
            context.arguments,
            limits,
            context.grants,
            context.policy,
            zelyra_mir::bytecode::BytecodeDatabaseExecution {
                url,
                schema: &mir.database_schema,
            },
            host_services,
        )
    } else {
        zelyra_mir::bytecode::execute_with_host_services_and_policy_captured(
            &bytecode,
            context.arguments,
            limits,
            context.grants,
            context.policy,
            host_services,
        )
    };
    Ok(zelyra_runtime::FunctionExecutionOutcome {
        output: outcome.output,
        value: outcome.value,
        error: outcome.error,
    })
}

fn pretty_value(value: &RuntimeValue, depth: usize) -> String {
    if depth >= 32 {
        return "<maximum display depth>".into();
    }
    match value {
        RuntimeValue::String(value) => format!("{:?}", value),
        RuntimeValue::Decimal(value) => {
            format!("{} [Decimal]", RuntimeValue::Decimal(*value).output())
        }
        RuntimeValue::Object {
            type_name, fields, ..
        } => {
            let mut fields = fields.iter().collect::<Vec<_>>();
            fields.sort_by(|left, right| left.0.cmp(right.0));
            let indent = " ".repeat((depth + 1) * 2);
            let close_indent = " ".repeat(depth * 2);
            let body = fields
                .into_iter()
                .map(|(name, value)| format!("{indent}{name}: {}", pretty_value(value, depth + 1)))
                .collect::<Vec<_>>()
                .join(",\n");
            format!("{type_name} {{\n{body}\n{close_indent}}}")
        }
        RuntimeValue::Array(values) | RuntimeValue::Tuple(values) | RuntimeValue::Set(values) => {
            let open = if matches!(value, RuntimeValue::Tuple(_)) {
                "("
            } else {
                "["
            };
            let close = if matches!(value, RuntimeValue::Tuple(_)) {
                ")"
            } else {
                "]"
            };
            format!(
                "{open}{}{close}",
                values
                    .iter()
                    .map(|value| pretty_value(value, depth + 1))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        RuntimeValue::Option(Some(value)) => format!("Some({})", pretty_value(value, depth + 1)),
        RuntimeValue::Option(None) => "None".into(),
        RuntimeValue::Result(Ok(value)) => format!("Ok({})", pretty_value(value, depth + 1)),
        RuntimeValue::Result(Err(value)) => format!("Err({})", pretty_value(value, depth + 1)),
        _ => value.output(),
    }
}

fn input_complete(input: &str) -> bool {
    let mut delimiters = Vec::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut line_comment = false;
    let mut characters = input.chars().peekable();
    while let Some(character) = characters.next() {
        if line_comment {
            if character == '\n' {
                line_comment = false;
            }
            continue;
        }
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
            continue;
        }
        if character == '/' && characters.peek() == Some(&'/') {
            characters.next();
            line_comment = true;
            continue;
        }
        match character {
            '"' => quoted = true,
            '{' | '[' | '(' => delimiters.push(character),
            '}' | ']' | ')' => {
                let expected_open = match character {
                    '}' => '{',
                    ']' => '[',
                    ')' => '(',
                    _ => unreachable!(),
                };
                if delimiters.last() == Some(&expected_open) {
                    delimiters.pop();
                } else {
                    // Let the parser report mismatched or unexpected closers
                    // immediately instead of waiting for another input line.
                    return true;
                }
            }
            _ => {}
        }
    }
    !quoted && delimiters.is_empty()
}

fn history_safe(input: &str) -> bool {
    let lower = input.to_ascii_lowercase();
    !input.contains(['\'', '"'])
        && !lower.contains("sql {")
        && ![
            "password",
            "passwd",
            "secret",
            "token",
            "credential",
            "api_key",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::{colorize_input, history_safe, input_complete, pretty_value, FilteredHistory};
    use reedline::{FileBackedHistory, History, HistoryItem};
    use std::fs;
    use zelyra_runtime::Value;

    #[test]
    fn multiline_detection_waits_for_open_blocks_and_strings() {
        assert!(!input_complete("if true {\n"));
        assert!(!input_complete("value = \"open\n"));
        assert!(input_complete("if true {\n value = 1\n}\n"));
    }

    #[test]
    fn multiline_detection_waits_for_calls_arrays_and_tuples_but_ignores_literals() {
        assert!(!input_complete("value = calculate(\n"));
        assert!(!input_complete("value = [1,\n"));
        assert!(!input_complete("value = (1,\n"));
        assert!(input_complete("value = calculate(\n  1,\n  2\n)\n"));
        assert!(input_complete("value = [\"(\", 2]\n"));
        assert!(input_complete(
            "// an opening brace { in a comment\nvalue = 1\n"
        ));
    }

    #[test]
    fn history_skips_values_and_database_commands_that_may_contain_secrets() {
        assert!(history_safe("x + 2"));
        assert!(!history_safe("secret = \"value\""));
        assert!(!history_safe("sql { SELECT 'credential' }"));
    }

    #[test]
    fn filtered_history_never_persists_sensitive_input() {
        let path = std::env::temp_dir().join(format!(
            "zelyra-repl-history-{}-{}.txt",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = fs::remove_file(&path);
        let mut history = FilteredHistory(FileBackedHistory::with_file(10, path.clone()).unwrap());
        assert!(history
            .save(HistoryItem::from_command_line(
                "password = \"secret-value\""
            ))
            .is_err());
        history
            .save(HistoryItem::from_command_line("1 + 2"))
            .unwrap();
        history.sync().unwrap();
        let contents = fs::read_to_string(&path).unwrap();
        assert!(!contents.contains("secret-value"));
        assert!(contents.contains("1 + 2"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn strings_are_quoted_and_records_are_multiline() {
        assert_eq!(
            pretty_value(&Value::String("weiter".into()), 0),
            "\"weiter\""
        );
        let value = Value::Object {
            type_name: "Customer".into(),
            type_args: Vec::new(),
            fields: [
                ("id".into(), Value::Int(7)),
                ("name".into(), Value::String("A".into())),
            ]
            .into_iter()
            .collect(),
        };
        assert_eq!(
            pretty_value(&value, 0),
            "Customer {\n  id: 7,\n  name: \"A\"\n}"
        );
    }

    #[test]
    fn decimal_values_keep_their_type_visible_in_repl_output() {
        assert_eq!(
            pretty_value(&Value::Decimal(1_190_000_000), 0),
            "1.19 [Decimal]"
        );
        let value = Value::Object {
            type_name: "Invoice".into(),
            type_args: Vec::new(),
            fields: [("gross_amount".into(), Value::Decimal(119_000_000_000))]
                .into_iter()
                .collect(),
        };
        assert_eq!(
            pretty_value(&value, 0),
            "Invoice {\n  gross_amount: 119 [Decimal]\n}"
        );
    }

    #[test]
    fn console_highlighter_colors_keywords_and_literals_without_changing_text() {
        let highlighted = colorize_input("return \"A\" + 42");
        assert!(highlighted.contains("\x1b[36mreturn\x1b[0m"));
        assert!(highlighted.contains("\x1b[32m\"A\"\x1b[0m"));
        assert!(highlighted.contains("\x1b[33m42\x1b[0m"));
    }
}
