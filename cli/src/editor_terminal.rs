use super::{editor_secret_values, redact_editor_secrets, zelyra_executable, EditorState};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const MAX_TERMINAL_OUTPUT_BYTES: usize = 64 * 1024;

#[derive(Default)]
pub(super) struct Session {
    active: bool,
    output: String,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
}

pub(super) fn start(state: &EditorState) -> Result<(u16, &'static str, String), (u16, String)> {
    let executable = zelyra_executable()?;
    let mut session = state
        .terminal
        .lock()
        .map_err(|_| (500, "could not access the Studio terminal".into()))?;
    if session.active || session.child.is_some() {
        return Err((409, "the Zelyra terminal is already active".into()));
    }
    let mut child = std::process::Command::new(executable)
        .arg("console")
        .current_dir(&state.root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| (500, "could not start the Zelyra console".into()))?;
    let stdin = child
        .stdin
        .take()
        .ok_or((500, "could not open terminal input".into()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or((500, "could not capture terminal output".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or((500, "could not capture terminal errors".into()))?;
    session.active = true;
    session.output.clear();
    session.child = Some(child);
    session.stdin = Some(stdin);
    drop(session);

    let secrets = editor_secret_values(state);
    read_stream(stdout, Arc::clone(&state.terminal), secrets.clone());
    read_stream(stderr, Arc::clone(&state.terminal), secrets);
    watch_process(Arc::clone(&state.terminal));
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({"started":true,"mode":"zelyra-console"}).to_string(),
    ))
}

pub(super) fn state(state: &EditorState) -> Result<(u16, &'static str, String), (u16, String)> {
    let session = state
        .terminal
        .lock()
        .map_err(|_| (500, "could not read the Studio terminal".into()))?;
    let output = redact_editor_secrets(session.output.clone(), &editor_secret_values(state));
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({"active":session.active,"output":output,"maxBytes":MAX_TERMINAL_OUTPUT_BYTES})
            .to_string(),
    ))
}

pub(super) fn input(
    state: &EditorState,
    value: &Value,
) -> Result<(u16, &'static str, String), (u16, String)> {
    let eof = value.get("eof").and_then(Value::as_bool).unwrap_or(false);
    let line = value.get("line").and_then(Value::as_str);
    if eof == line.is_some() {
        return Err((400, "provide exactly one terminal line or eof=true".into()));
    }
    if let Some(line) = line {
        if line.len() > 4096 || line.contains(['\r', '\n']) {
            return Err((
                400,
                "terminal input must be one line of at most 4096 bytes".into(),
            ));
        }
    }
    let mut session = state
        .terminal
        .lock()
        .map_err(|_| (500, "could not access terminal input".into()))?;
    if !session.active {
        return Err((409, "the Zelyra terminal is not active".into()));
    }
    if eof {
        session.stdin.take();
    } else if let Some(line) = line {
        let Some(stdin) = session.stdin.as_mut() else {
            return Err((409, "terminal input has already reached EOF".into()));
        };
        stdin
            .write_all(line.as_bytes())
            .and_then(|()| stdin.write_all(b"\n"))
            .and_then(|()| stdin.flush())
            .map_err(|_| {
                (
                    409,
                    "the Zelyra console is no longer accepting input".into(),
                )
            })?;
        session.output.push_str("> ");
        session.output.push_str(line);
        session.output.push('\n');
    } else {
        return Err((409, "terminal input has already reached EOF".into()));
    }
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({"accepted":true,"eof":eof}).to_string(),
    ))
}

pub(super) fn stop(state: &EditorState) -> Result<(u16, &'static str, String), (u16, String)> {
    let mut session = state
        .terminal
        .lock()
        .map_err(|_| (500, "could not stop the Studio terminal".into()))?;
    if let Some(mut child) = session.child.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    session.stdin.take();
    session.active = false;
    session.output.push_str("\n[Zelyra terminal stopped]\n");
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({"stopped":true}).to_string(),
    ))
}

pub(super) fn shutdown(terminal: &Arc<Mutex<Session>>) {
    if let Ok(mut session) = terminal.lock() {
        if let Some(mut child) = session.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        session.stdin.take();
        session.active = false;
    }
}

fn read_stream<R: Read + Send + 'static>(
    mut reader: R,
    terminal: Arc<Mutex<Session>>,
    secrets: Vec<String>,
) {
    thread::spawn(move || {
        let mut bytes = [0_u8; 2048];
        loop {
            match reader.read(&mut bytes) {
                Ok(0) | Err(_) => break,
                Ok(count) => {
                    let text = String::from_utf8_lossy(&bytes[..count]).into_owned();
                    let text = redact_editor_secrets(text, &secrets);
                    if let Ok(mut session) = terminal.lock() {
                        session.output.push_str(&text);
                        if session.output.len() > MAX_TERMINAL_OUTPUT_BYTES {
                            let mut keep_from = session.output.len() - MAX_TERMINAL_OUTPUT_BYTES;
                            while !session.output.is_char_boundary(keep_from) {
                                keep_from += 1;
                            }
                            session.output.drain(..keep_from);
                            if let Some(first_line) = session.output.find('\n') {
                                session.output.drain(..=first_line);
                            }
                        }
                    }
                }
            }
        }
    });
}

fn watch_process(terminal: Arc<Mutex<Session>>) {
    thread::spawn(move || loop {
        let finished = match terminal.lock() {
            Ok(mut session) => match session
                .child
                .as_mut()
                .and_then(|child| child.try_wait().ok())
                .flatten()
            {
                Some(_) => {
                    session.child.take();
                    session.stdin.take();
                    session.active = false;
                    session.output.push_str("\n[Zelyra console exited]\n");
                    true
                }
                None => !session.active,
            },
            Err(_) => true,
        };
        if finished {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    });
}
