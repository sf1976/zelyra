use super::*;

pub(super) fn task_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(action) = args.next() else {
        task_usage();
        return ExitCode::from(2);
    };
    match action.as_str() {
        "list" => {
            let Some(path) = args.next() else {
                task_usage();
                return ExitCode::from(2);
            };
            let json = match args.next().as_deref() {
                None => false,
                Some("--json") => true,
                _ => {
                    task_usage();
                    return ExitCode::from(2);
                }
            };
            if args.next().is_some() {
                task_usage();
                return ExitCode::from(2);
            }
            let Ok(program) = validate(&path) else {
                return ExitCode::from(1);
            };
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "schema_version": 1,
                        "commands": program.commands.iter().map(|command| {
                            let function = program.functions.iter().find(|function| function.name == command.handler);
                            json!({
                                "name": command.name,
                                "handler": command.handler,
                                "description": command.description,
                                "source_line": command.span.line,
                                "parameters": function.map(|function| function.params.iter().map(|parameter| json!({"name": parameter.name, "type": parameter.ty.to_string()})).collect::<Vec<_>>()).unwrap_or_default(),
                            })
                        }).collect::<Vec<_>>(),
                    }))
                    .expect("task inventory JSON is serializable")
                );
            } else if program.commands.is_empty() {
                println!("No commands declared in {path}");
            } else {
                for command in &program.commands {
                    let function = program
                        .functions
                        .iter()
                        .find(|function| function.name == command.handler);
                    let parameters = function
                        .map(|function| {
                            function
                                .params
                                .iter()
                                .map(|parameter| format!("{}: {}", parameter.name, parameter.ty))
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    println!("{}({}) -> {}", command.name, parameters, command.handler);
                    if let Some(description) = &command.description {
                        println!("  {description}");
                    }
                }
            }
            ExitCode::SUCCESS
        }
        "run" => {
            let Some(path) = args.next() else {
                task_usage();
                return ExitCode::from(2);
            };
            let Some(name) = args.next() else {
                task_usage();
                return ExitCode::from(2);
            };
            let mut entry_args = None;
            let mut max_steps = None;
            while let Some(argument) = args.next() {
                match argument.as_str() {
                    "--args" if entry_args.is_none() => match args.next() {
                        Some(value) => entry_args = Some(value),
                        None => {
                            task_usage();
                            return ExitCode::from(2);
                        }
                    },
                    "--max-steps" if max_steps.is_none() => match args.next() {
                        Some(value) => max_steps = Some(value),
                        None => {
                            task_usage();
                            return ExitCode::from(2);
                        }
                    },
                    _ => {
                        task_usage();
                        return ExitCode::from(2);
                    }
                }
            }
            let Ok(program) = validate(&path) else {
                return ExitCode::from(1);
            };
            let Some(command) = program.commands.iter().find(|command| command.name == name) else {
                eprintln!("error[E-TASK-002]: command `{name}` was not declared in `{path}`");
                return ExitCode::from(2);
            };
            let mut process = match env::current_exe() {
                Ok(executable) => Command::new(executable),
                Err(error) => {
                    eprintln!("error[E-TASK-003]: cannot locate Zelyra executable: {error}");
                    return ExitCode::from(1);
                }
            };
            process.args(["run", &path, "--entry", &command.handler]);
            if let Some(entry_args) = entry_args {
                process.args(["--entry-args", &entry_args]);
            }
            if let Some(max_steps) = max_steps {
                process.args(["--max-steps", &max_steps]);
            }
            match process.status() {
                Ok(status) if status.success() => ExitCode::SUCCESS,
                Ok(status) => ExitCode::from(
                    status
                        .code()
                        .and_then(|code| u8::try_from(code).ok())
                        .unwrap_or(1),
                ),
                Err(error) => {
                    eprintln!("error[E-TASK-003]: could not start command `{name}`: {error}");
                    ExitCode::from(1)
                }
            }
        }
        _ => {
            task_usage();
            ExitCode::from(2)
        }
    }
}

pub(super) fn schedule_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(action) = args.next() else {
        schedule_usage();
        return ExitCode::from(2);
    };
    let Some(path) = args.next() else {
        schedule_usage();
        return ExitCode::from(2);
    };
    match action.as_str() {
        "list" => {
            let json = match args.next().as_deref() {
                None => false,
                Some("--json") => true,
                _ => {
                    schedule_usage();
                    return ExitCode::from(2);
                }
            };
            if args.next().is_some() {
                schedule_usage();
                return ExitCode::from(2);
            }
            let Ok(program) = validate(&path) else {
                return ExitCode::from(1);
            };
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "schema_version": 1,
                        "schedules": program.schedules.iter().map(|schedule| json!({
                            "name": schedule.name,
                            "command": schedule.command,
                            "cron": schedule.cron,
                            "timezone": schedule.timezone,
                            "source_line": schedule.span.line,
                        })).collect::<Vec<_>>(),
                    }))
                    .expect("schedule inventory JSON is serializable")
                );
            } else if program.schedules.is_empty() {
                println!("No schedules declared in {path}");
            } else {
                for schedule in &program.schedules {
                    println!(
                        "{}: {}  {}",
                        schedule.name,
                        schedule.cron,
                        schedule.timezone.as_deref().unwrap_or("UTC (default)")
                    );
                }
            }
            ExitCode::SUCCESS
        }
        "export" => {
            let mut format_seen = false;
            let mut output = None;
            while let Some(argument) = args.next() {
                match argument.as_str() {
                    "--format" if !format_seen => match args.next() {
                        Some(value) if value == "cron" => format_seen = true,
                        _ => {
                            schedule_usage();
                            return ExitCode::from(2);
                        }
                    },
                    "--output" if output.is_none() => match args.next() {
                        Some(value) if !value.is_empty() => output = Some(value),
                        _ => {
                            schedule_usage();
                            return ExitCode::from(2);
                        }
                    },
                    _ => {
                        schedule_usage();
                        return ExitCode::from(2);
                    }
                }
            }
            let Ok(program) = validate(&path) else {
                return ExitCode::from(1);
            };
            let absolute_entry = match std::path::Path::new(&path).canonicalize() {
                Ok(entry) => entry,
                Err(error) => {
                    eprintln!("error[E-TASK-003]: cannot resolve `{path}`: {error}");
                    return ExitCode::from(1);
                }
            };
            let project_root = absolute_entry.parent().unwrap_or(std::path::Path::new("."));
            let executable = match env::current_exe() {
                Ok(executable) => executable,
                Err(error) => {
                    eprintln!("error[E-TASK-003]: cannot locate Zelyra executable: {error}");
                    return ExitCode::from(1);
                }
            };
            if absolute_entry
                .as_os_str()
                .to_string_lossy()
                .chars()
                .any(char::is_control)
                || executable
                    .as_os_str()
                    .to_string_lossy()
                    .chars()
                    .any(char::is_control)
            {
                eprintln!("error[E-TASK-003]: cron export cannot safely represent control characters in the project or executable path");
                return ExitCode::from(1);
            }
            let mut cron = String::new();
            for schedule in &program.schedules {
                let command = program
                    .commands
                    .iter()
                    .find(|command| command.name == schedule.command)
                    .expect("compiler validates schedule references");
                let timezone = schedule.timezone.as_deref().unwrap_or("UTC");
                writeln!(cron, "# Zelyra schedule: {}", schedule.name)
                    .expect("writing to a String cannot fail");
                writeln!(cron, "CRON_TZ={timezone}").expect("writing to a String cannot fail");
                writeln!(
                    cron,
                    "{} cd {} && {} run {} --entry {}",
                    schedule.cron,
                    shell_quote(&project_root.to_string_lossy()),
                    shell_quote(&executable.to_string_lossy()),
                    shell_quote(&absolute_entry.to_string_lossy()),
                    shell_quote(&command.handler)
                )
                .expect("writing to a String cannot fail");
                cron.push('\n');
            }
            if let Some(output) = output {
                match write_schedule_export(project_root, &output, cron.as_bytes()) {
                    Ok(path) => eprintln!(
                        "Cron preview written to {}. Review it before installing; Zelyra did not install or enable a scheduler.",
                        path.display()
                    ),
                    Err(message) => {
                        eprintln!("error[E-TASK-004]: {message}");
                        return ExitCode::from(1);
                    }
                }
            } else {
                print!("{cron}");
                eprintln!("Review this cron preview before installing it. Zelyra did not install or enable a scheduler.");
            }
            ExitCode::SUCCESS
        }
        _ => {
            schedule_usage();
            ExitCode::from(2)
        }
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn write_schedule_export(
    project_root: &std::path::Path,
    relative_path: &str,
    contents: &[u8],
) -> Result<PathBuf, String> {
    let relative = std::path::Path::new(relative_path);
    if relative_path.chars().any(char::is_control)
        || relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(
            "--output must be a project-relative path without `.` or `..` components".into(),
        );
    }
    let file_name = relative
        .file_name()
        .ok_or_else(|| "--output must name a file".to_owned())?;
    let parent = project_root.join(
        relative
            .parent()
            .unwrap_or_else(|| std::path::Path::new("")),
    );
    let canonical_parent = parent.canonicalize().map_err(|error| {
        format!(
            "cannot resolve output directory `{}`: {error}",
            parent.display()
        )
    })?;
    if !canonical_parent.starts_with(project_root) {
        return Err("--output directory resolves outside the project".into());
    }
    let destination = canonical_parent.join(file_name);
    match fs::symlink_metadata(&destination) {
        Ok(_) => {
            return Err(format!(
                "refusing to replace existing output `{}`",
                destination.display()
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "cannot inspect output `{}`: {error}",
                destination.display()
            ))
        }
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .map_err(|error| format!("cannot create output `{}`: {error}", destination.display()))?;
    if let Err(error) = file.write_all(contents).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(&destination);
        return Err(format!(
            "cannot write output `{}`: {error}",
            destination.display()
        ));
    }
    Ok(destination)
}

fn task_usage() {
    eprintln!("Usage: zelyra task list <main.zyl> [--json] | task run <main.zyl> <command> [--args '<JSON-array>'] [--max-steps <n>]");
}

fn schedule_usage() {
    eprintln!("Usage: zelyra schedule list <main.zyl> [--json] | schedule export <main.zyl> [--format cron] [--output <project-relative-file>]");
}
