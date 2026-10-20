use super::{compiled_regex, PhpSourceMap, SourceSpan};
use regex::Regex;
use serde_json::{json, Value};

#[derive(Clone, Debug)]
pub(super) struct SqlScriptEvidence {
    pub(super) path: String,
    pub(super) sha256: String,
}

#[derive(Default)]
pub(super) struct ConsoleAnalyzer {
    commands: Vec<Value>,
    schedules: Vec<Value>,
}

impl ConsoleAnalyzer {
    pub(super) fn scan(&mut self, file: &str, source: &str) {
        let source_map = PhpSourceMap::new(source);
        self.scan_class_signatures(file, source, &source_map);
        self.scan_attribute_signatures(file, source, &source_map);
        self.scan_inline_commands(file, source, &source_map);
        self.scan_schedules(file, source, &source_map);
    }

    fn scan_class_signatures(&mut self, file: &str, source: &str, source_map: &PhpSourceMap) {
        let signature = compiled_regex(
            r"\b(?:protected|public|private)\s+(?:static\s+)?\$signature\s*=",
            "Laravel command signature regex",
        );
        for matched in signature.find_iter(source) {
            if !source_map.is_code(matched.start()) {
                continue;
            }
            let semicolon = find_code_byte(source, source_map, matched.end(), b';');
            let statement_end = semicolon.map_or(matched.end(), |end| end + 1);
            let mut value_start = matched.end();
            while source
                .as_bytes()
                .get(value_start)
                .is_some_and(u8::is_ascii_whitespace)
            {
                value_start += 1;
            }
            let literal = parse_quoted_literal(source, value_start).filter(|(_, literal_end)| {
                semicolon.is_some_and(|semicolon| {
                    *literal_end <= semicolon && source[*literal_end..semicolon].trim().is_empty()
                })
            });
            if let Some((literal, _)) = literal {
                let command_name = literal.split_whitespace().next().unwrap_or_default();
                if !command_name.is_empty()
                    && !command_name.contains('$')
                    && !command_name.contains('{')
                {
                    let mut command = command_json(
                        file,
                        source_map.span(source, matched.start()..statement_end),
                        "class_signature",
                        Some(command_name),
                        Some(parse_signature(&literal)),
                        "static_signature",
                    );
                    command["implementation"] =
                        command_implementation(source, source_map, matched.start());
                    self.commands.push(command);
                    continue;
                }
            }
            let mut command = command_json(
                file,
                source_map.span(source, matched.start()..statement_end),
                "class_signature",
                None,
                None,
                "dynamic_or_unsupported_signature",
            );
            command["implementation"] = command_implementation(source, source_map, matched.start());
            self.commands.push(command);
        }
    }

    fn scan_inline_commands(&mut self, file: &str, source: &str, source_map: &PhpSourceMap) {
        self.scan_command_calls(file, source, source_map, "Artisan", "inline_artisan");
    }

    fn scan_attribute_signatures(&mut self, file: &str, source: &str, source_map: &PhpSourceMap) {
        let attribute = compiled_regex(
            r#"#\[\s*Signature\s*\(\s*(['"])"#,
            "Laravel Signature attribute regex",
        );
        for matched in attribute.find_iter(source) {
            if !source_map.is_code(matched.start()) {
                continue;
            }
            let quote_start = matched.end() - 1;
            let literal = parse_quoted_literal(source, quote_start);
            let scan_from = literal
                .as_ref()
                .map_or(quote_start + 1, |(_, literal_end)| *literal_end);
            let Some(close_paren) = find_code_byte(source, source_map, scan_from, b')') else {
                continue;
            };
            let Some(close_bracket) = find_code_byte(source, source_map, close_paren + 1, b']')
            else {
                continue;
            };
            let Some((literal, _)) = literal else {
                self.commands.push(command_json(
                    file,
                    source_map.span(source, matched.start()..close_bracket + 1),
                    "signature_attribute",
                    None,
                    None,
                    "dynamic_or_unsupported_signature",
                ));
                continue;
            };
            let name = literal.split_whitespace().next().unwrap_or_default();
            let mut command = command_json(
                file,
                source_map.span(source, matched.start()..close_bracket + 1),
                "signature_attribute",
                (!name.is_empty()).then_some(name),
                Some(parse_signature(&literal)),
                "static_signature",
            );
            command["implementation"] = command_implementation(source, source_map, matched.start());
            self.commands.push(command);
        }
    }

    fn scan_schedules(&mut self, file: &str, source: &str, source_map: &PhpSourceMap) {
        let call = compiled_regex(
            r"(?:\bSchedule\s*::\s*|\$schedule\s*->\s*)(command|job|call|exec)\s*\(",
            "Laravel scheduled task regex",
        );
        for capture in call.captures_iter(source) {
            let matched = capture.get(0).expect("scheduled task match");
            if !source_map.is_code(matched.start()) {
                continue;
            }
            let method = capture.get(1).expect("scheduled task method").as_str();
            let open = matched.end() - 1;
            let Some(close) = matching_delimiter(source, source_map, open, b'(', b')') else {
                continue;
            };
            let statement_end = source[close + 1..]
                .char_indices()
                .find_map(|(offset, ch)| {
                    let index = close + 1 + offset;
                    (ch == ';' && source_map.is_code(index)).then_some(index + 1)
                })
                .unwrap_or(close + 1);
            let first_arg = first_argument_range(source, source_map, open + 1, close);
            let target = first_arg
                .as_ref()
                .map(|range| schedule_target(source, source_map, range.clone(), method))
                .unwrap_or_else(|| json!({ "kind": "missing_or_unsupported_target" }));
            let raw_command = (method == "command" || method == "exec")
                .then(|| target["command_literal"].as_str().map(str::to_owned))
                .flatten();
            let (command_name, arguments) = raw_command
                .as_deref()
                .map(parse_invocation)
                .unwrap_or((None, Vec::new()));
            let schedule_span = source_map.span(source, matched.start()..statement_end);
            let kind = match method {
                "command" => "scheduled_command",
                "job" => "scheduled_job",
                "call" => "scheduled_callback",
                "exec" => "scheduled_shell_command",
                _ => unreachable!("regex limits scheduled task methods"),
            };
            let resolution = match method {
                "command" => "pending",
                "job" if target["resolution"] == "static_class_reference" => {
                    "static_job_class_unresolved"
                }
                "call" if target["resolution"] == "static_callback_reference" => {
                    "static_callback_unresolved"
                }
                "exec" if raw_command.is_some() => "static_shell_command_manual_review",
                _ => "dynamic_or_unsupported_target",
            };
            self.schedules.push(json!({
                "id": format!("laravel:schedule:{file}:{}", schedule_span.start_byte),
                "file": file,
                "source_span": span_json(schedule_span),
                "kind": kind,
                "target": target,
                "command_literal": raw_command.as_deref().map(redact_command),
                "command_name": command_name,
                "arguments": arguments,
                "literal_status": if method == "command" {
                    if target["resolution"] == "static_literal" {
                        "static_literal"
                    } else {
                        "dynamic_or_unsupported_command_expression"
                    }
                } else {
                    target["resolution"].as_str().unwrap_or("unknown")
                },
                "modifiers": schedule_modifiers(
                    source,
                    source_map,
                    close + 1,
                    statement_end,
                ),
                "resolution": resolution
            }));
        }
    }

    fn scan_command_calls(
        &mut self,
        file: &str,
        source: &str,
        source_map: &PhpSourceMap,
        facade: &str,
        origin: &str,
    ) {
        let pattern = format!(r"\b{facade}\s*::\s*command\s*\(");
        let call = Regex::new(&pattern).expect("static facade command regex");
        for matched in call.find_iter(source) {
            if !source_map.is_code(matched.start()) {
                continue;
            }
            let open = matched.end() - 1;
            let Some(close) = matching_delimiter(source, source_map, open, b'(', b')') else {
                continue;
            };
            let arguments = argument_ranges(source, source_map, open + 1, close);
            let definition = arguments
                .first()
                .and_then(|range| parse_quoted_literal(source, range.start))
                .filter(|(_, literal_end)| {
                    arguments.first().is_some_and(|range| {
                        *literal_end <= range.end
                            && source.as_bytes()[*literal_end..range.end]
                                .iter()
                                .enumerate()
                                .all(|(offset, byte)| {
                                    !source_map.is_code(*literal_end + offset)
                                        || byte.is_ascii_whitespace()
                                })
                    })
                })
                .map(|(value, _)| StringArgument { value });
            let span = source_map.span(source, matched.start()..close + 1);
            let implementation = arguments
                .get(1)
                .map(|range| inline_command_implementation(source, source_map, range.clone()))
                .unwrap_or_else(|| {
                    json!({
                        "kind": "callback",
                        "resolution": "callback_argument_missing"
                    })
                });
            match definition {
                Some(value) => {
                    let name = value.value.split_whitespace().next().unwrap_or_default();
                    let mut command = command_json(
                        file,
                        span,
                        origin,
                        (!name.is_empty()).then_some(name),
                        Some(parse_signature(&value.value)),
                        "static_signature",
                    );
                    command["implementation"] = implementation;
                    self.commands.push(command);
                }
                None => {
                    let mut command = command_json(
                        file,
                        span,
                        origin,
                        None,
                        None,
                        "dynamic_or_unsupported_command_expression",
                    );
                    command["implementation"] = implementation;
                    self.commands.push(command);
                }
            }
        }
    }

    #[cfg(test)]
    pub(super) fn finish(self) -> Value {
        self.finish_with_deployment_sql_files(&[])
    }

    pub(super) fn finish_with_deployment_sql_files(
        mut self,
        deployment_sql_files: &[SqlScriptEvidence],
    ) -> Value {
        for schedule in &mut self.schedules {
            if schedule["kind"] != "scheduled_command" {
                continue;
            }
            let name = schedule["command_name"].as_str().map(str::to_owned);
            let candidates = name.as_deref().map_or_else(Vec::new, |name| {
                self.commands
                    .iter()
                    .filter(|command| command["command_name"] == name)
                    .collect()
            });
            let framework_builtin = name.as_deref().is_some_and(is_known_framework_command);
            schedule["resolution"] = json!(if framework_builtin {
                "framework_builtin"
            } else {
                match candidates.len() {
                    1 => "resolved_unique_definition",
                    0 => "unresolved_definition_not_scanned",
                    _ => "unresolved_multiple_definitions",
                }
            });
            schedule["execution_role"] = json!(if name.as_deref() == Some("queue:work") {
                "long_running_queue_worker"
            } else {
                "scheduled_command"
            });
            schedule["definition_candidate_count"] = json!(candidates.len());
            schedule["definition_ids"] = json!(candidates
                .iter()
                .filter_map(|command| command["id"].as_str())
                .collect::<Vec<_>>());
        }
        for command in &mut self.commands {
            let patterns = command["implementation"]["sql_file_globs"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            if patterns.is_empty() {
                continue;
            }
            let discoveries = patterns
                .iter()
                .filter_map(|item| item["pattern"].as_str())
                .map(|pattern| {
                    let file_evidence = deployment_sql_files
                        .iter()
                        .filter(|file| static_sql_glob_matches(pattern, &file.path))
                        .map(|file| json!({"path": file.path, "sha256": file.sha256}))
                        .collect::<Vec<_>>();
                    let files = file_evidence
                        .iter()
                        .filter_map(|file| file["path"].as_str())
                        .collect::<Vec<_>>();
                    json!({
                        "pattern": pattern,
                        "files": files,
                        "file_evidence": file_evidence,
                        "ordering": "lexicographic_project_path_only_not_proven_runtime_execution_order",
                        "resolution": if files.is_empty() { "no_scanned_match" } else { "matched_scanned_files" },
                        "semantics": "static_path_pattern_only_not_runtime_execution_or_glob_result"
                    })
                })
                .collect::<Vec<_>>();
            command["implementation"]["sql_script_discovery"] = json!(discoveries);
        }
        self.commands.sort_by(|left, right| {
            left["file"]
                .as_str()
                .cmp(&right["file"].as_str())
                .then_with(|| {
                    left["source_span"]["start"]["byte_offset"]
                        .as_u64()
                        .cmp(&right["source_span"]["start"]["byte_offset"].as_u64())
                })
        });
        self.schedules.sort_by(|left, right| {
            left["file"]
                .as_str()
                .cmp(&right["file"].as_str())
                .then_with(|| {
                    left["source_span"]["start"]["byte_offset"]
                        .as_u64()
                        .cmp(&right["source_span"]["start"]["byte_offset"].as_u64())
                })
        });
        let command_count = self.commands.len();
        let scheduled_tasks = self.schedules;
        let scheduled_commands = scheduled_tasks
            .iter()
            .filter(|schedule| schedule["kind"] == "scheduled_command")
            .cloned()
            .collect::<Vec<_>>();
        let schedule_count = scheduled_commands.len();
        let resolved_count = scheduled_commands
            .iter()
            .filter(|schedule| schedule["resolution"] == "resolved_unique_definition")
            .count();
        let builtin_count = scheduled_commands
            .iter()
            .filter(|schedule| schedule["resolution"] == "framework_builtin")
            .count();
        let task_count = scheduled_tasks.len();
        json!({
            "schema_version": "4",
            "scope": ["static_class_signature", "static_inline_artisan_command", "static_schedule_command", "static_schedule_job", "static_schedule_call", "static_schedule_exec", "literal_schedule_modifiers", "static_base_path_deployment_sql_glob"],
            "resolution_semantics": "static_source_reference_only_not_runtime_registration_or_behavioral_equivalence",
            "commands": self.commands,
            "scheduled_commands": scheduled_commands,
            "scheduled_tasks": scheduled_tasks,
            "summary": {
                "commands": command_count,
                "scheduled_commands": schedule_count,
                "scheduled_tasks": task_count,
                "resolved_schedule_definitions": resolved_count,
                "framework_builtin_schedule_references": builtin_count,
                "unresolved_schedule_definitions": schedule_count - resolved_count - builtin_count
            }
        })
    }
}

struct StringArgument {
    value: String,
}

fn first_argument_range(
    source: &str,
    source_map: &PhpSourceMap,
    start: usize,
    end: usize,
) -> Option<std::ops::Range<usize>> {
    let mut nesting = 0_usize;
    let mut argument_end = end;
    for index in start..end {
        if !source_map.is_code(index) {
            continue;
        }
        match source.as_bytes()[index] {
            b'(' | b'[' | b'{' => nesting += 1,
            b')' | b']' | b'}' => nesting = nesting.saturating_sub(1),
            b',' if nesting == 0 => {
                argument_end = index;
                break;
            }
            _ => {}
        }
    }
    let argument = start..argument_end;
    let trimmed_start = argument.start + source[argument.clone()].len()
        - source[argument.clone()].trim_start().len();
    let trimmed_end =
        argument.end - source[argument.clone()].len() + source[argument.clone()].trim_end().len();
    (trimmed_start < trimmed_end).then_some(trimmed_start..trimmed_end)
}

fn argument_ranges(
    source: &str,
    source_map: &PhpSourceMap,
    start: usize,
    end: usize,
) -> Vec<std::ops::Range<usize>> {
    let mut ranges = Vec::new();
    let mut nesting = 0_usize;
    let mut range_start = start;
    for index in start..end {
        if !source_map.is_code(index) {
            continue;
        }
        match source.as_bytes()[index] {
            b'(' | b'[' | b'{' => nesting += 1,
            b')' | b']' | b'}' => nesting = nesting.saturating_sub(1),
            b',' if nesting == 0 => {
                push_trimmed_range(source, range_start..index, &mut ranges);
                range_start = index + 1;
            }
            _ => {}
        }
    }
    push_trimmed_range(source, range_start..end, &mut ranges);
    ranges
}

fn push_trimmed_range(
    source: &str,
    range: std::ops::Range<usize>,
    ranges: &mut Vec<std::ops::Range<usize>>,
) {
    let trimmed_start =
        range.start + source[range.clone()].len() - source[range.clone()].trim_start().len();
    let trimmed_end =
        range.end - source[range.clone()].len() + source[range.clone()].trim_end().len();
    if trimmed_start < trimmed_end {
        ranges.push(trimmed_start..trimmed_end);
    }
}

fn inline_command_implementation(
    source: &str,
    source_map: &PhpSourceMap,
    range: std::ops::Range<usize>,
) -> Value {
    let source_text = source[range.clone()].trim_start();
    let kind = if source_text.starts_with("function")
        && source_text["function".len()..]
            .chars()
            .next()
            .is_none_or(char::is_whitespace)
    {
        "closure"
    } else if source_text.starts_with("fn")
        && source_text["fn".len()..]
            .chars()
            .next()
            .is_some_and(|ch| ch.is_whitespace() || ch == '(')
    {
        "arrow_closure"
    } else {
        return json!({
            "kind": "callback",
            "resolution": "dynamic_or_unsupported_callback",
            "source_span": span_json(source_map.span(source, range))
        });
    };
    json!({
        "kind": kind,
        "resolution": "source_span_only_not_translated",
        "source_span": span_json(source_map.span(source, range))
    })
}

fn static_sql_glob_matches(pattern: &str, path: &str) -> bool {
    let Some((prefix, suffix)) = pattern.split_once('*') else {
        return pattern == path;
    };
    if suffix.contains('*')
        || !pattern.starts_with("deploy/sql/")
        || !pattern.ends_with(".sql")
        || pattern.contains(['?', '[', ']'])
        || pattern
            .split('/')
            .any(|component| component == "." || component == "..")
    {
        return false;
    }
    if !path.starts_with(prefix) || !path.ends_with(suffix) {
        return false;
    }
    let Some(middle_end) = path.len().checked_sub(suffix.len()) else {
        return false;
    };
    let Some(middle) = path.get(prefix.len()..middle_end) else {
        return false;
    };
    !middle.contains('/')
}

fn schedule_target(
    source: &str,
    source_map: &PhpSourceMap,
    range: std::ops::Range<usize>,
    method: &str,
) -> Value {
    if matches!(method, "command" | "exec") {
        let literal = parse_quoted_literal(source, range.start).filter(|(_, literal_end)| {
            *literal_end <= range.end
                && source.as_bytes()[*literal_end..range.end]
                    .iter()
                    .enumerate()
                    .all(|(offset, byte)| {
                        !source_map.is_code(*literal_end + offset) || byte.is_ascii_whitespace()
                    })
        });
        return match literal {
            Some((value, _)) => json!({
                "kind": if method == "command" { "artisan_command" } else { "shell_command" },
                "resolution": "static_literal",
                "command_literal": redact_command(&value)
            }),
            None => json!({
                "kind": if method == "command" { "artisan_command" } else { "shell_command" },
                "resolution": "dynamic_or_unsupported_expression"
            }),
        };
    }

    let expression = source[range.clone()].trim();
    let class = compiled_regex(
        r"^new\s+([\\A-Za-z_][\\A-Za-z0-9_]*)",
        "scheduled class construction regex",
    );
    if let Some(captures) = class.captures(expression) {
        return json!({
            "kind": if method == "job" { "job_class" } else { "invokable_class" },
            "class_name": captures.get(1).expect("class capture").as_str(),
            "resolution": "static_class_reference",
            "constructor_arguments": "not_analyzed"
        });
    }

    let callback = compiled_regex(
        r"^(?:static\s+)?(?:function\s*\(|fn\s*\()",
        "schedule callback regex",
    );
    if callback.is_match(expression) {
        return json!({
            "kind": "inline_callback",
            "resolution": "static_callback_reference",
            "source_span": span_json(source_map.span(source, range))
        });
    }

    let literal = parse_quoted_literal(source, range.start).filter(|(_, literal_end)| {
        *literal_end <= range.end
            && source.as_bytes()[*literal_end..range.end]
                .iter()
                .enumerate()
                .all(|(offset, byte)| {
                    !source_map.is_code(*literal_end + offset) || byte.is_ascii_whitespace()
                })
    });
    if let Some((value, _)) = literal {
        let callable = compiled_regex(
            r"^[\\A-Za-z_][\\A-Za-z0-9_]*(?:::[A-Za-z_]\w*)?$",
            "static callback name regex",
        );
        if callable.is_match(&value) {
            return json!({
                "kind": "named_callback",
                "target_name": value,
                "resolution": "static_callback_reference"
            });
        }
    }
    json!({
        "kind": if method == "job" { "job" } else { "callback" },
        "resolution": "dynamic_or_unsupported_expression"
    })
}

fn first_string_argument(source: &str, mut start: usize, end: usize) -> Option<StringArgument> {
    let bytes = source.as_bytes();
    while start < end && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    let (value, literal_end) = parse_quoted_literal(source, start)?;
    if literal_end > end {
        return None;
    }
    Some(StringArgument { value })
}

fn parse_quoted_literal(source: &str, start: usize) -> Option<(String, usize)> {
    let bytes = source.as_bytes();
    let quote = *bytes.get(start)?;
    if !matches!(quote, b'\'' | b'"') {
        return None;
    }
    let mut cursor = start + 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\\' {
            cursor = cursor.checked_add(2)?;
            continue;
        }
        if bytes[cursor] == quote {
            let value = unescape_literal(&source[start + 1..cursor], quote);
            if quote == b'"' && (value.contains('$') || value.contains("{")) {
                return None;
            }
            return Some((value, cursor + 1));
        }
        cursor += 1;
    }
    None
}

fn parse_signature(signature: &str) -> Value {
    let mut parameters = Vec::new();
    let mut remaining = signature;
    while let Some(open) = remaining.find('{') {
        let tail = &remaining[open + 1..];
        let Some(close) = tail.find('}') else { break };
        let declaration = tail[..close].split(':').next().unwrap_or_default().trim();
        if declaration.is_empty() {
            remaining = &tail[close + 1..];
            continue;
        }
        let option = declaration.starts_with('-');
        let token = declaration.trim_start_matches('-');
        let long_name = token.rsplit('|').next().unwrap_or(token);
        let (name, default) = long_name
            .split_once('=')
            .map_or((long_name, None), |(name, default)| (name, Some(default)));
        let takes_value = default.is_some();
        let required = !option && !name.ends_with('?');
        let variadic = name.ends_with('*');
        parameters.push(json!({
            "name": name.trim_end_matches(['?', '*']),
            "kind": if option { "option" } else { "argument" },
            "required": required,
            "takes_value": takes_value || !option,
            "variadic": variadic,
            "default": default.map(|value| redact_value(name, value)),
            "typing": "not_inferred_from_artisan_signature"
        }));
        remaining = &tail[close + 1..];
    }
    json!(parameters)
}

fn parse_invocation(command: &str) -> (Option<String>, Vec<Value>) {
    let tokens = split_command_words(command);
    let Some(name) = tokens.first() else {
        return (None, Vec::new());
    };
    let mut redact_next_value = false;
    let mut arguments = Vec::new();
    for argument in tokens.iter().skip(1) {
        if redact_next_value {
            arguments.push(json!({ "kind": "argument", "value": "<redacted>" }));
            redact_next_value = false;
            continue;
        }
        if argument.starts_with('-') {
            let name = argument.split('=').next().unwrap_or(argument);
            let value = argument
                .split_once('=')
                .map(|(_, value)| redact_value(name, value));
            redact_next_value = value.is_none() && looks_sensitive(name);
            arguments.push(json!({ "kind": "option", "name": name, "value": value }));
        } else {
            arguments.push(json!({ "kind": "argument", "value": redact_value("", argument) }));
        }
    }
    (Some(name.clone()), arguments)
}

fn is_known_framework_command(name: &str) -> bool {
    matches!(
        name,
        "queue:work" | "queue:listen" | "schedule:run" | "schedule:work"
    )
}

fn split_command_words(command: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    for ch in command.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if let Some(active) = quote {
            if ch == active {
                quote = None
            } else {
                current.push(ch)
            }
        } else if ch == '\'' || ch == '"' {
            quote = Some(ch);
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }
    if escaped {
        current.push('\\');
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn redact_command(command: &str) -> String {
    let words = split_command_words(command);
    let mut result = Vec::new();
    let mut redact_next_value = false;
    for word in words {
        if redact_next_value {
            result.push("<redacted>".into());
            redact_next_value = false;
        } else if let Some((name, value)) = word.split_once('=') {
            result.push(format!("{name}={}", redact_value(name, value)));
        } else if word.starts_with('-') && looks_sensitive(&word) {
            result.push(word);
            redact_next_value = true;
        } else {
            result.push(word);
        }
    }
    result.join(" ")
}

fn redact_value(name: &str, value: &str) -> String {
    if looks_sensitive(name) {
        "<redacted>".into()
    } else {
        value.into()
    }
}

fn looks_sensitive(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase().replace(['-', '_'], "");
    [
        "password",
        "passwd",
        "secret",
        "token",
        "credential",
        "apikey",
        "privatekey",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn schedule_modifiers(source: &str, source_map: &PhpSourceMap, start: usize, end: usize) -> Value {
    let statement = &source[start..end];
    let call = compiled_regex(r"->\s*([A-Za-z_]\w*)\s*\(", "schedule modifier regex");
    let mut cadence = Vec::new();
    let mut timezone = None;
    let mut without_overlapping = false;
    let mut overlap_expiry_minutes = None;
    let mut on_one_server = false;
    let mut run_in_background = false;
    let mut calls = Vec::new();
    let mut unmodeled_calls = Vec::new();
    let mut cursor = 0;
    while let Some(capture) = call.captures_at(statement, cursor) {
        let matched = capture.get(0).expect("modifier match");
        let absolute = start + matched.start();
        if !source_map.is_code(absolute) {
            cursor = matched.end();
            continue;
        }
        let method = capture.get(1).expect("modifier method").as_str();
        let absolute_open = start + matched.end() - 1;
        let Some(absolute_close) =
            matching_delimiter(source, source_map, absolute_open, b'(', b')')
        else {
            cursor = matched.end();
            continue;
        };
        let args = parse_literal_arguments(source, source_map, absolute_open + 1, absolute_close);
        let recognized = is_cadence(method)
            || matches!(
                method,
                "timezone" | "withoutOverlapping" | "runInBackground" | "onOneServer"
            );
        if is_cadence(method) {
            calls.push(json!({ "name": method, "literal_arguments": args }));
            cadence.push(json!({ "name": method, "literal_arguments": args }));
        }
        if method == "timezone" {
            timezone = args
                .as_array()
                .and_then(|args| args.first())
                .and_then(Value::as_str)
                .map(str::to_owned);
            calls.push(json!({ "name": method, "literal_arguments": args }));
        }
        if method == "withoutOverlapping" {
            without_overlapping = true;
            overlap_expiry_minutes = args
                .as_array()
                .and_then(|args| args.first())
                .and_then(Value::as_u64);
            calls.push(json!({ "name": method, "literal_arguments": args }));
        }
        if method == "runInBackground" {
            run_in_background = true;
            calls.push(json!({ "name": method }));
        }
        if method == "onOneServer" {
            on_one_server = true;
            calls.push(json!({ "name": method }));
        }
        if !recognized {
            unmodeled_calls.push(method.to_owned());
        }
        cursor = absolute_close + 1 - start;
    }
    json!({
        "cadence": cadence,
        "timezone": timezone,
        "without_overlapping": without_overlapping,
        "overlap_expiry_minutes": overlap_expiry_minutes,
        "on_one_server": on_one_server,
        "run_in_background": run_in_background,
        "calls": calls,
        "unmodeled_calls": unmodeled_calls
    })
}

fn parse_literal_arguments(
    source: &str,
    source_map: &PhpSourceMap,
    start: usize,
    end: usize,
) -> Value {
    let bytes = source.as_bytes();
    let mut parts = Vec::new();
    let mut part_start = start;
    let mut nesting = 0_usize;
    for (index, byte) in bytes.iter().enumerate().take(end).skip(start) {
        if !source_map.is_code(index) {
            continue;
        }
        match *byte {
            b'(' | b'[' | b'{' => nesting += 1,
            b')' | b']' | b'}' => nesting = nesting.saturating_sub(1),
            b',' if nesting == 0 => {
                parts.push(part_start..index);
                part_start = index + 1;
            }
            _ => {}
        }
    }
    if part_start < end || !parts.is_empty() {
        parts.push(part_start..end);
    }
    if parts.is_empty() {
        return json!([]);
    }
    let mut values = Vec::new();
    for part in parts {
        let text = source[part.clone()].trim();
        if text.is_empty() {
            return Value::Null;
        }
        if let Some(value) = first_string_argument(source, part.start, part.end) {
            values.push(json!(value.value));
            continue;
        }
        if text == "true" || text == "false" || text == "null" {
            values.push(match text {
                "true" => json!(true),
                "false" => json!(false),
                _ => Value::Null,
            });
            continue;
        }
        if let Ok(value) = text.parse::<i64>() {
            values.push(json!(value));
            continue;
        }
        if let Ok(value) = text.parse::<f64>() {
            values.push(json!(value));
            continue;
        }
        return Value::Null;
    }
    json!(values)
}

fn unescape_literal(raw: &str, quote: u8) -> String {
    let mut result = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek().copied() {
                Some(next) if next == quote as char || next == '\\' => {
                    result.push(chars.next().expect("peeked escape"));
                }
                _ => result.push(ch),
            }
        } else {
            result.push(ch);
        }
    }
    result
}

fn is_cadence(method: &str) -> bool {
    matches!(
        method,
        "everySecond"
            | "everyTwoSeconds"
            | "everyFiveSeconds"
            | "everyTenSeconds"
            | "everyFifteenSeconds"
            | "everyThirtySeconds"
            | "everyMinute"
            | "everyTwoMinutes"
            | "everyFiveMinutes"
            | "everyTenMinutes"
            | "everyFifteenMinutes"
            | "everyThirtyMinutes"
            | "hourly"
            | "hourlyAt"
            | "everyTwoHours"
            | "everyThreeHours"
            | "everyFourHours"
            | "everySixHours"
            | "daily"
            | "dailyAt"
            | "twiceDaily"
            | "weekly"
            | "weeklyOn"
            | "monthly"
            | "monthlyOn"
            | "twiceMonthly"
            | "quarterly"
            | "yearly"
            | "yearlyOn"
            | "cron"
    )
}

fn matching_delimiter(
    source: &str,
    source_map: &PhpSourceMap,
    open: usize,
    left: u8,
    right: u8,
) -> Option<usize> {
    let bytes = source.as_bytes();
    if bytes.get(open) != Some(&left) {
        return None;
    }
    let mut depth = 0_usize;
    for (index, byte) in bytes.iter().copied().enumerate().skip(open) {
        if !source_map.is_code(index) {
            continue;
        }
        if byte == left {
            depth += 1;
        }
        if byte == right {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn find_code_byte(
    source: &str,
    source_map: &PhpSourceMap,
    start: usize,
    sought: u8,
) -> Option<usize> {
    source
        .as_bytes()
        .iter()
        .enumerate()
        .skip(start)
        .find_map(|(index, byte)| (*byte == sought && source_map.is_code(index)).then_some(index))
}

fn command_implementation(
    source: &str,
    source_map: &PhpSourceMap,
    signature_offset: usize,
) -> Value {
    let class = compiled_regex(
        r"\bclass\s+[A-Za-z_]\w*[^{}]*\{",
        "Laravel command class regex",
    );
    let classes = class
        .captures_iter(source)
        .filter_map(|capture| {
            let matched = capture.get(0)?;
            if !source_map.is_code(matched.start()) {
                return None;
            }
            let open = matched.end().checked_sub(1)?;
            let close = matching_delimiter(source, source_map, open, b'{', b'}')?;
            Some((matched.start(), open, close))
        })
        .collect::<Vec<_>>();
    let class_range = classes
        .iter()
        .find(|(_, open, close)| *open < signature_offset && signature_offset < *close)
        .or_else(|| {
            classes
                .iter()
                .find(|(start, _, _)| *start >= signature_offset)
        });
    let Some((_, class_open, class_close)) = class_range else {
        return json!({
            "kind": "class_method",
            "resolution": "declaring_class_not_found"
        });
    };
    let (class_open, class_close) = (*class_open, *class_close);

    let method = compiled_regex(
        r"\bfunction\s+(handle|__invoke)\s*\(",
        "Laravel command handler method regex",
    );
    let mut handlers = Vec::new();
    for capture in method.captures_iter(&source[class_open + 1..class_close]) {
        let relative = capture.get(0).expect("command handler match");
        let start = class_open + 1 + relative.start();
        if !source_map.is_code(start) {
            continue;
        }
        let method_name = capture.get(1).expect("command handler name").as_str();
        let open_paren = class_open + 1 + relative.end() - 1;
        let Some(close_paren) = matching_delimiter(source, source_map, open_paren, b'(', b')')
        else {
            continue;
        };
        let Some(open_brace) = find_code_byte(source, source_map, close_paren + 1, b'{') else {
            continue;
        };
        let Some(close_brace) = matching_delimiter(source, source_map, open_brace, b'{', b'}')
        else {
            continue;
        };
        handlers.push((method_name, source_map.span(source, start..close_brace + 1)));
    }

    let selected = handlers
        .iter()
        .find(|(name, _)| *name == "handle")
        .or_else(|| handlers.iter().find(|(name, _)| *name == "__invoke"));
    match selected {
        Some((method, span)) => json!({
            "kind": "class_method",
            "method": method,
            "resolution": "resolved_unique",
            "source_span": span_json(*span),
            "sql_file_globs": static_sql_globs(
                source,
                source_map,
                span.start_byte..span.end_byte
            )
        }),
        None => json!({
            "kind": "class_method",
            "resolution": "handler_not_found_in_declaring_class"
        }),
    }
}

fn static_sql_globs(
    source: &str,
    source_map: &PhpSourceMap,
    method_range: std::ops::Range<usize>,
) -> Vec<Value> {
    let single_quoted = compiled_regex(
        r"\bglob\s*\(\s*base_path\s*\(\s*'([^']+)'\s*\)\s*\)",
        "static Laravel base_path SQL glob regex",
    );
    let double_quoted = compiled_regex(
        r#"\bglob\s*\(\s*base_path\s*\(\s*"([^"]+)"\s*\)\s*\)"#,
        "static Laravel base_path SQL glob regex",
    );
    let mut patterns = single_quoted
        .captures_iter(source)
        .chain(double_quoted.captures_iter(source))
        .filter_map(|capture| {
            let matched = capture.get(0)?;
            let literal = capture.get(1)?;
            if matched.start() < method_range.start
                || matched.end() > method_range.end
                || !source_map.is_code(matched.start())
            {
                return None;
            }
            let value = literal.as_str();
            if !value.starts_with("deploy/sql/")
                || !value.ends_with(".sql")
                || value.matches('*').count() != 1
                || value.contains(['?', '[', ']'])
                || value.split('/').any(|part| part == "." || part == "..")
                || value.chars().any(|ch| {
                    !(ch.is_ascii_alphanumeric() || matches!(ch, '/' | '_' | '-' | '.' | '*'))
                })
            {
                return None;
            }
            Some((
                matched.start(),
                json!({
                    "pattern": value,
                    "source_span": span_json(source_map.span(source, matched.range()))
                }),
            ))
        })
        .collect::<Vec<_>>();
    patterns.sort_by_key(|(offset, _)| *offset);
    patterns.into_iter().map(|(_, pattern)| pattern).collect()
}

fn command_json(
    file: &str,
    span: SourceSpan,
    origin: &str,
    name: Option<&str>,
    parameters: Option<Value>,
    status: &str,
) -> Value {
    let name = name.filter(|name| !name.is_empty());
    json!({
        "id": format!("laravel:command:{file}:{}", span.start_byte),
        "file": file,
        "source_span": span_json(span),
        "origin": origin,
        "command_name": name,
        "parameters": parameters.unwrap_or_else(|| json!([])),
        "literal_status": status,
        "resolution": "definition_only_php_body_not_translated"
    })
}

fn span_json(span: SourceSpan) -> Value {
    json!({
        "start": { "byte_offset": span.start_byte, "line": span.start_line, "column": span.start_column },
        "end": { "byte_offset": span.end_byte, "line": span.end_line, "column": span.end_column },
        "end_exclusive": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_links_static_commands_and_preserves_scheduler_modifiers() {
        let mut analyzer = ConsoleAnalyzer::default();
        analyzer.scan(
            "app/Console/Commands/HelloCommand.php",
            "<?php\nclass HelloCommand extends Command {\n protected $signature = 'hello {name} {--force : Force it}';\n}",
        );
        analyzer.scan(
            "routes/console.php",
            "<?php\nArtisan::command('inline:ping {--loud}', fn () => null);\nSchedule::command('hello Ada --force')->dailyAt('04:00')->timezone('Europe/Berlin')->runInBackground()->withoutOverlapping(7)->onOneServer();\nSchedule::command('missing')->everyFifteenMinutes();\n",
        );
        let report = analyzer.finish();

        assert_eq!(report["summary"]["commands"], 2);
        assert_eq!(report["summary"]["scheduled_commands"], 2);
        assert_eq!(report["summary"]["resolved_schedule_definitions"], 1);
        assert!(report["scheduled_commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|schedule| schedule["command_name"] == "hello"
                && schedule["resolution"] == "resolved_unique_definition"));
        let hello = report["scheduled_commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|schedule| schedule["command_name"] == "hello")
            .unwrap();
        assert_eq!(hello["arguments"][0]["value"], "Ada");
        assert_eq!(hello["modifiers"]["cadence"][0]["name"], "dailyAt");
        assert_eq!(hello["modifiers"]["timezone"], "Europe/Berlin");
        assert_eq!(hello["modifiers"]["run_in_background"], true);
        assert_eq!(hello["modifiers"]["without_overlapping"], true);
        assert_eq!(hello["modifiers"]["overlap_expiry_minutes"], 7);
        assert_eq!(hello["modifiers"]["on_one_server"], true);
        assert_eq!(hello["source_span"]["start"]["line"], 3);
        assert_eq!(report["commands"][0]["parameters"][0]["name"], "name");
        let inline = report["commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|command| command["command_name"] == "inline:ping")
            .unwrap();
        assert_eq!(inline["implementation"]["kind"], "arrow_closure");
        assert_eq!(
            inline["implementation"]["resolution"],
            "source_span_only_not_translated"
        );
        assert!(
            inline["implementation"]["source_span"]["start"]["byte_offset"]
                .as_u64()
                .unwrap()
                > inline["source_span"]["start"]["byte_offset"]
                    .as_u64()
                    .unwrap()
        );
    }

    #[test]
    fn class_commands_link_literal_deployment_sql_globs_to_scanned_files() {
        let mut analyzer = ConsoleAnalyzer::default();
        analyzer.scan(
            "app/Console/Commands/DispatchInstall.php",
            r#"<?php
class DispatchInstall extends Command {
    protected $signature = 'dispatch:install';
    public function handle(): int {
        $files = glob(base_path('deploy/sql/dispatch/*.sql'));
        $example = "glob(base_path('deploy/sql/ignored/*.sql'))";
        // glob(base_path('deploy/sql/commented/*.sql'))
        return 0;
    }
}"#,
        );
        let report = analyzer.finish_with_deployment_sql_files(&[
            SqlScriptEvidence {
                path: "deploy/sql/dispatch/001_dispatch.sql".into(),
                sha256: "a".repeat(64),
            },
            SqlScriptEvidence {
                path: "deploy/sql/dispatch/002_relations.sql".into(),
                sha256: "b".repeat(64),
            },
            SqlScriptEvidence {
                path: "deploy/sql/other/001_other.sql".into(),
                sha256: "c".repeat(64),
            },
        ]);
        let command = report["commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|command| command["command_name"] == "dispatch:install")
            .unwrap();
        assert_eq!(
            command["implementation"]["sql_file_globs"][0]["pattern"], "deploy/sql/dispatch/*.sql",
            "{command:#}"
        );
        let discovery = &command["implementation"]["sql_script_discovery"][0];
        assert_eq!(
            discovery["resolution"], "matched_scanned_files",
            "{discovery:#}"
        );
        assert_eq!(
            discovery["files"],
            json!([
                "deploy/sql/dispatch/001_dispatch.sql",
                "deploy/sql/dispatch/002_relations.sql"
            ])
        );
        assert_eq!(
            discovery["file_evidence"],
            json!([
                {
                    "path": "deploy/sql/dispatch/001_dispatch.sql",
                    "sha256": "a".repeat(64)
                },
                {
                    "path": "deploy/sql/dispatch/002_relations.sql",
                    "sha256": "b".repeat(64)
                }
            ])
        );
        assert_eq!(
            discovery["semantics"],
            "static_path_pattern_only_not_runtime_execution_or_glob_result"
        );
    }

    #[test]
    fn deployment_sql_globs_reject_unsupported_and_cross_directory_matches() {
        assert!(static_sql_glob_matches(
            "deploy/sql/dispatch/*.sql",
            "deploy/sql/dispatch/001.sql"
        ));
        assert!(!static_sql_glob_matches(
            "deploy/sql/dispatch/*.sql",
            "deploy/sql/other/001.sql"
        ));
        assert!(!static_sql_glob_matches(
            "deploy/sql/../*.sql",
            "deploy/sql/001.sql"
        ));
        assert!(!static_sql_glob_matches(
            "deploy/sql/**/*.sql",
            "deploy/sql/dispatch/001.sql"
        ));
    }

    #[test]
    fn inline_command_callback_spans_exclude_signature_and_nested_commas_are_safe() {
        let mut analyzer = ConsoleAnalyzer::default();
        let source = "<?php\nArtisan::command('report {--kind=}', function () {\n    $rows = collect([['a', 'b'], ['c', 'd']]);\n    $this->info('ready');\n});\nArtisan::command('dynamic', $callback);\n";
        analyzer.scan("routes/console.php", source);
        let report = analyzer.finish();
        assert_eq!(report["schema_version"], "4");
        let commands = report["commands"].as_array().unwrap();
        assert_eq!(commands.len(), 2);

        let closure = &commands[0];
        assert_eq!(closure["command_name"], "report");
        assert_eq!(closure["implementation"]["kind"], "closure");
        assert_eq!(
            closure["implementation"]["resolution"],
            "source_span_only_not_translated"
        );
        let span = &closure["implementation"]["source_span"];
        let start = span["start"]["byte_offset"].as_u64().unwrap() as usize;
        let end = span["end"]["byte_offset"].as_u64().unwrap() as usize;
        assert_eq!(&source[start..end], "function () {\n    $rows = collect([['a', 'b'], ['c', 'd']]);\n    $this->info('ready');\n}");

        let dynamic = &commands[1];
        assert_eq!(dynamic["command_name"], "dynamic");
        assert_eq!(
            dynamic["implementation"]["resolution"],
            "dynamic_or_unsupported_callback"
        );
        let span = &dynamic["implementation"]["source_span"];
        let start = span["start"]["byte_offset"].as_u64().unwrap() as usize;
        let end = span["end"]["byte_offset"].as_u64().unwrap() as usize;
        assert_eq!(&source[start..end], "$callback");
    }

    #[test]
    fn inventory_ignores_comments_and_strings_and_redacts_sensitive_options() {
        let mut analyzer = ConsoleAnalyzer::default();
        analyzer.scan(
            "routes/console.php",
            "<?php\n// Schedule::command('fake')->daily();\n$example = \"Artisan::command('not:real')\";\nSchedule::command('mail:send --api-token supersecret')->daily();\nSchedule::command($dynamic)->cron($expression);\n",
        );
        let report = analyzer.finish();

        assert_eq!(report["summary"]["commands"], 0);
        assert_eq!(report["summary"]["scheduled_commands"], 2);
        let mail = report["scheduled_commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|schedule| schedule["command_name"] == "mail:send")
            .unwrap();
        assert_eq!(mail["arguments"][1]["value"], "<redacted>");
        let serialized = serde_json::to_string(&report).unwrap();
        assert!(!serialized.contains("supersecret"));
        assert!(report["scheduled_commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|schedule| schedule["literal_status"]
                == "dynamic_or_unsupported_command_expression"));
    }

    #[test]
    fn inventory_reads_signature_attributes_and_schedule_callback_variables() {
        let mut analyzer = ConsoleAnalyzer::default();
        analyzer.scan(
            "app/Console/Commands/BankSync.php",
            "<?php\n#[Signature('banking:sync {--tenant= : Tenant ID}')]\nclass BankSync extends Command {}",
        );
        analyzer.scan(
            "bootstrap/app.php",
            "<?php\n->withSchedule(function (Schedule $schedule): void {\n $schedule->command('banking:sync --tenant=3')->weeklyOn(1, '04:00');\n})",
        );
        let report = analyzer.finish();
        assert_eq!(report["summary"]["commands"], 1);
        assert_eq!(report["summary"]["scheduled_commands"], 1);
        assert_eq!(report["summary"]["resolved_schedule_definitions"], 1);
        assert_eq!(report["commands"][0]["origin"], "signature_attribute");
        assert_eq!(report["commands"][0]["command_name"], "banking:sync");
        assert_eq!(
            report["scheduled_commands"][0]["modifiers"]["cadence"][0]["name"],
            "weeklyOn"
        );
        assert_eq!(
            report["scheduled_commands"][0]["modifiers"]["cadence"][0]["literal_arguments"][0],
            1
        );
        assert_eq!(
            report["scheduled_commands"][0]["modifiers"]["cadence"][0]["literal_arguments"][1],
            "04:00"
        );
    }

    #[test]
    fn class_command_signatures_link_only_to_the_declaring_handlers() {
        let mut analyzer = ConsoleAnalyzer::default();
        analyzer.scan(
            "app/Console/Commands/Refresh.php",
            "<?php\nclass Refresh extends Command {\n protected $signature = 'app:refresh';\n // function handle() { return false; }\n public function handle(): int { $text = \"function handle() {}\"; return 0; }\n}\n#[Signature('app:other')] class Other extends Command { public function __invoke(): int { return 1; } }",
        );
        let commands = analyzer.finish()["commands"].as_array().unwrap().clone();
        let refresh = commands
            .iter()
            .find(|command| command["command_name"] == "app:refresh")
            .unwrap();
        assert_eq!(refresh["implementation"]["resolution"], "resolved_unique");
        assert_eq!(refresh["implementation"]["method"], "handle");
        assert_eq!(refresh["implementation"]["source_span"]["start"]["line"], 5);

        let other = commands
            .iter()
            .find(|command| command["command_name"] == "app:other")
            .unwrap();
        assert_eq!(other["implementation"]["resolution"], "resolved_unique");
        assert_eq!(other["implementation"]["method"], "__invoke");
        assert_eq!(other["implementation"]["source_span"]["start"]["line"], 7);
    }

    #[test]
    fn framework_worker_commands_are_identified_without_project_definitions() {
        let mut analyzer = ConsoleAnalyzer::default();
        analyzer.scan(
            "bootstrap/app.php",
            "<?php\n$schedule->command('queue:work --queue=mail')->everyMinute()->withoutOverlapping();",
        );
        let report = analyzer.finish();
        let worker = &report["scheduled_commands"][0];
        assert_eq!(worker["resolution"], "framework_builtin");
        assert_eq!(worker["execution_role"], "long_running_queue_worker");
        assert_eq!(report["summary"]["unresolved_schedule_definitions"], 0);
        assert_eq!(
            report["summary"]["framework_builtin_schedule_references"],
            1
        );
    }

    #[test]
    fn inventories_job_callback_and_shell_schedule_targets_without_executing_them() {
        let mut analyzer = ConsoleAnalyzer::default();
        analyzer.scan(
            "routes/console.php",
            "<?php\nSchedule::job(new App\\Jobs\\Heartbeat(['id' => 4]))->everyFiveMinutes();\nSchedule::call(function (): void { echo 'callback'; })->hourly();\nSchedule::exec('php artisan reports:run --api-token topsecret')->daily();\nSchedule::job($dynamic)->weekly();\n",
        );
        let report = analyzer.finish();
        let tasks = report["scheduled_tasks"].as_array().unwrap();

        assert_eq!(report["summary"]["scheduled_tasks"], 4);
        assert_eq!(report["summary"]["scheduled_commands"], 0);
        assert_eq!(tasks[0]["kind"], "scheduled_job");
        assert_eq!(tasks[0]["target"]["class_name"], "App\\Jobs\\Heartbeat");
        assert_eq!(tasks[0]["resolution"], "static_job_class_unresolved");
        assert_eq!(tasks[1]["kind"], "scheduled_callback");
        assert_eq!(tasks[1]["target"]["kind"], "inline_callback");
        assert_eq!(tasks[1]["target"]["source_span"]["end_exclusive"], true);
        assert_eq!(tasks[2]["kind"], "scheduled_shell_command");
        assert_eq!(tasks[2]["target"]["resolution"], "static_literal");
        assert_eq!(
            tasks[2]["command_literal"],
            "php artisan reports:run --api-token <redacted>"
        );
        assert_eq!(tasks[3]["resolution"], "dynamic_or_unsupported_target");
        assert!(!serde_json::to_string(&report)
            .unwrap()
            .contains("topsecret"));
    }

    #[test]
    fn schedule_job_and_callback_scanning_ignores_comments_and_strings() {
        let mut analyzer = ConsoleAnalyzer::default();
        analyzer.scan(
            "routes/console.php",
            "<?php\n// Schedule::job(new Fake)->daily();\n$text = \"Schedule::call(new Fake)->hourly()\";\nSchedule::job($real)->daily();\n",
        );
        let report = analyzer.finish();
        assert_eq!(report["summary"]["scheduled_tasks"], 1);
        assert_eq!(
            report["scheduled_tasks"][0]["resolution"],
            "dynamic_or_unsupported_target"
        );
    }

    #[test]
    fn schedule_modifiers_report_conditions_that_require_manual_translation() {
        let mut analyzer = ConsoleAnalyzer::default();
        analyzer.scan(
            "routes/console.php",
            "<?php\nSchedule::command('billing:run')->weekdays()->between('08:00', '17:00')->when(fn () => config('billing.enabled'))->onOneServer();\n",
        );
        let report = analyzer.finish();
        let schedule = &report["scheduled_commands"][0];
        assert_eq!(schedule["modifiers"]["on_one_server"], true);
        assert_eq!(
            schedule["modifiers"]["unmodeled_calls"],
            json!(["weekdays", "between", "when"])
        );
        assert_eq!(schedule["modifiers"]["calls"][0]["name"], "onOneServer");
    }

    #[test]
    fn dynamic_class_signatures_are_reported_and_option_defaults_are_typed_as_unknown() {
        let mut analyzer = ConsoleAnalyzer::default();
        analyzer.scan(
            "app/Console/Commands/DynamicCommand.php",
            "<?php\nprotected $signature = config('commands.dynamic');\n",
        );
        analyzer.scan(
            "app/Console/Commands/StaticCommand.php",
            "<?php\nprotected $signature = 'mail:work {--queue=mail : Queue name}';\n",
        );
        let report = analyzer.finish();
        assert_eq!(report["summary"]["commands"], 2);
        let dynamic = report["commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|command| {
                command["file"].as_str() == Some("app/Console/Commands/DynamicCommand.php")
            })
            .unwrap();
        assert_eq!(
            dynamic["literal_status"],
            "dynamic_or_unsupported_signature"
        );
        let static_command = report["commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|command| command["command_name"] == "mail:work")
            .unwrap();
        assert_eq!(static_command["parameters"][0]["name"], "queue");
        assert_eq!(static_command["parameters"][0]["default"], "mail");
        assert_eq!(
            static_command["parameters"][0]["typing"],
            "not_inferred_from_artisan_signature"
        );
    }
}
