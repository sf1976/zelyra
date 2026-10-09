use super::*;
use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    Message, SmtpTransport, Transport,
};
use rand_core::{OsRng, RngCore};
use std::{
    sync::mpsc::{sync_channel, SyncSender, TrySendError},
    thread,
    time::Duration,
};

const RESET_TOKEN_TTL_MINUTES: u32 = 15;
const RESET_COOKIE: &str = "zelyra_password_reset";
const RESET_COOKIE_PATH: &str = "/reset-password";
const RESET_MAIL_QUEUE_CAPACITY: usize = 64;
const RESET_ADVISORY_LOCK_WAIT_SECONDS: u32 = 10;
type ResetMailSender = dyn Fn(&str, &str, UiLanguage) -> Result<(), String> + Send + Sync;

struct ResetMailJob {
    email: String,
    link: String,
    language: UiLanguage,
    delivery_guard: Option<ResetMailDeliveryGuard>,
}

struct ResetMailDeliveryGuard {
    database_url: String,
    lock_name: String,
    reset_table: String,
    token_hash: String,
}

#[derive(Clone)]
pub struct PasswordResetMailer {
    base_url: String,
    #[cfg(test)]
    sender: Arc<ResetMailSender>,
    queue: Option<SyncSender<ResetMailJob>>,
}

impl fmt::Debug for PasswordResetMailer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PasswordResetMailer")
            .field("base_url", &self.base_url)
            .field("sender", &"<redacted>")
            .finish()
    }
}

impl PasswordResetMailer {
    pub fn smtp(
        host: &str,
        port: u16,
        security: &str,
        username: Option<&str>,
        password: Option<&str>,
        from: &str,
        base_url: &str,
    ) -> Result<Self, String> {
        validate_public_base_url(base_url)?;
        if host.is_empty() || host.contains(['/', '\\', '@', '?', '#', '\r', '\n']) {
            return Err("ZELYRA_SMTP_HOST must be a hostname without a URL or credentials".into());
        }
        if port == 0 {
            return Err("ZELYRA_SMTP_PORT must be between 1 and 65535".into());
        }
        if username.is_some() != password.is_some() {
            return Err(
                "ZELYRA_SMTP_USERNAME and ZELYRA_SMTP_PASSWORD must be set together".into(),
            );
        }
        let local_plaintext = security == "local_plaintext";
        if local_plaintext
            && (!matches!(host, "localhost" | "127.0.0.1" | "::1") || username.is_some())
        {
            return Err(
                "local_plaintext SMTP is restricted to loopback without authentication".into(),
            );
        }
        let mut builder = match security {
            "implicit_tls" => SmtpTransport::relay(host),
            "starttls" => SmtpTransport::starttls_relay(host),
            "local_plaintext" => Ok(SmtpTransport::builder_dangerous(host)),
            _ => {
                return Err(
                    "ZELYRA_SMTP_SECURITY must be implicit_tls, starttls, or local_plaintext"
                        .into(),
                )
            }
        }
        .map_err(|_| "SMTP relay configuration is invalid".to_owned())?
        .port(port)
        .timeout(Some(Duration::from_secs(5)));
        if let (Some(username), Some(password)) = (username, password) {
            builder = builder.credentials(Credentials::new(username.into(), password.into()));
        }
        let transport = Arc::new(builder.build());
        let from = from
            .parse::<Mailbox>()
            .map_err(|_| "ZELYRA_SMTP_FROM must be a valid email address".to_owned())?;
        let base_url = base_url.trim_end_matches('/').to_owned();
        let sender: Arc<ResetMailSender> = Arc::new(move |recipient, link, language| {
            let to = recipient
                .parse::<Mailbox>()
                .map_err(|_| "password reset recipient is invalid".to_owned())?;
            let subject = i18n::text(language, "auth.reset_email_subject").to_owned();
            let body = i18n::text(language, "auth.reset_email_body")
                .replace("{minutes}", &RESET_TOKEN_TTL_MINUTES.to_string())
                .replace("{link}", link);
            let message = Message::builder()
                .from(from.clone())
                .to(to)
                .subject(subject)
                .header(ContentType::TEXT_PLAIN)
                .body(body)
                .map_err(|_| "password reset message could not be built".to_owned())?;
            transport
                .send(&message)
                .map(|_| ())
                .map_err(|_| "SMTP transport could not deliver the message".to_owned())
        });
        let (queue, receiver) = sync_channel::<ResetMailJob>(RESET_MAIL_QUEUE_CAPACITY);
        let worker_sender = sender.clone();
        thread::Builder::new()
            .name("zelyra-password-reset-mail".into())
            .spawn(move || {
                while let Ok(job) = receiver.recv() {
                    let delivery = if let Some(guard) = job.delivery_guard.as_ref() {
                        deliver_if_current_reset(&job, guard, &worker_sender)
                    } else {
                        worker_sender(&job.email, &job.link, job.language)
                    };
                    if let Err(error) = delivery {
                        eprintln!("zelyra web: password reset message delivery failed: {error}");
                    }
                }
            })
            .map_err(|_| "password reset mail worker could not be started".to_owned())?;
        Ok(Self {
            base_url,
            #[cfg(test)]
            sender,
            queue: Some(queue),
        })
    }

    #[cfg(test)]
    fn test_sender(
        base_url: &str,
        sender: impl Fn(&str, &str) -> Result<(), String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            sender: Arc::new(move |email, link, _language| sender(email, link)),
            queue: None,
        }
    }

    #[cfg(test)]
    fn send(&self, email: &str, token: &str, language: UiLanguage) -> Result<(), String> {
        let link = format!("{}/reset-password?token={token}", self.base_url);
        let Some(queue) = self.queue.as_ref() else {
            return (self.sender)(email, &link, language);
        };
        queue
            .try_send(ResetMailJob {
                email: email.to_owned(),
                link,
                language,
                delivery_guard: None,
            })
            .map_err(|error| match error {
                TrySendError::Full(_) => "password reset mail queue is full".to_owned(),
                TrySendError::Disconnected(_) => {
                    "password reset mail worker is unavailable".to_owned()
                }
            })
    }

    fn enqueue_reset(
        &self,
        delivery_guard: ResetMailDeliveryGuard,
        email: &str,
        token: &str,
        language: UiLanguage,
    ) -> Result<(), String> {
        let Some(queue) = self.queue.as_ref() else {
            return Err("password reset mail queue is unavailable".into());
        };
        let link = format!("{}/reset-password?token={token}", self.base_url);
        queue
            .try_send(ResetMailJob {
                email: email.to_owned(),
                link,
                language,
                delivery_guard: Some(delivery_guard),
            })
            .map_err(|error| match error {
                TrySendError::Full(_) => "password reset mail queue is full".to_owned(),
                TrySendError::Disconnected(_) => {
                    "password reset mail worker is unavailable".to_owned()
                }
            })
    }
}

fn deliver_if_current_reset(
    job: &ResetMailJob,
    guard: &ResetMailDeliveryGuard,
    sender: &Arc<ResetMailSender>,
) -> Result<(), String> {
    let result = zelyra_database::with_mariadb_advisory_transaction(
        &guard.database_url,
        &guard.lock_name,
        RESET_ADVISORY_LOCK_WAIT_SECONDS,
        |transaction| {
            let query = zelyra_database::Query {
                sql: format!(
                    "SELECT user_id FROM {} WHERE token_hash = :token_hash AND consumed_at IS NULL AND expires_at > NOW() LIMIT 1 FOR UPDATE",
                    quote_identifier(&guard.reset_table)
                ),
                params: vec![(
                    "token_hash".into(),
                    QueryValue::String(guard.token_hash.clone()),
                )],
            };
            let current = !transaction.execute(&query)?.rows.is_empty();
            if !current {
                return Ok(None);
            }
            Ok(Some(sender(&job.email, &job.link, job.language)))
        },
    );
    match result {
        Ok(None) => Ok(()),
        Ok(Some(delivery)) => delivery,
        Err(_) => Err("could not verify the current password reset token".into()),
    }
}

fn validate_public_base_url(value: &str) -> Result<(), String> {
    let valid = url::Url::parse(value).ok().is_some_and(|url| {
        let secure = url.scheme() == "https";
        let local_http = url.scheme() == "http"
            && url.host().is_some_and(|host| match host {
                url::Host::Domain(domain) => domain.eq_ignore_ascii_case("localhost"),
                url::Host::Ipv4(address) => address.is_loopback(),
                url::Host::Ipv6(address) => address.is_loopback(),
            });
        (secure || local_http)
            && url.host_str().is_some_and(|host| !host.is_empty())
            && url.username().is_empty()
            && url.password().is_none()
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none()
    });
    if !valid {
        return Err("ZELYRA_PUBLIC_BASE_URL must be an HTTPS origin or a loopback HTTP origin without a path".into());
    }
    Ok(())
}

pub(super) fn dispatch_password_reset(
    app: &WebApp,
    auth: &AuthRoute,
    request: &Request,
) -> Response {
    let Some(reset_table) = auth.reset_tokens_table.as_deref() else {
        return Response::empty(404);
    };
    if app.database_capability_granted == Some(false) {
        return database_capability_denied();
    }
    let Some(database_url) = app.database_url.as_deref() else {
        return Response::html(503, "<h1>503 Service Unavailable</h1>");
    };
    let Some(mailer) = app.password_reset_mailer.as_ref() else {
        return Response::html(503, "<h1>503 Service Unavailable</h1>");
    };
    match (request.path.as_str(), request.method.as_str()) {
        ("/forgot-password", "GET") => reset_form_response(auth, app.ui_language, false),
        ("/forgot-password", "POST") => {
            let input = match parse_urlencoded(&request.body) {
                Ok(input) => input,
                Err(_) => return Response::html(400, "<h1>400 Bad Request</h1>"),
            };
            if !verify_csrf_request(
                request,
                &auth.csrf,
                input.get("_zelyra_csrf").map(String::as_str),
            ) {
                return reset_generic_response(app.ui_language, 202);
            }
            let key = reset_rate_key(request);
            if reset_rate_limited(app, &key, auth.reset_rate_limit, auth.reset_block_seconds) {
                return reset_generic_response(app.ui_language, 202);
            }
            let email = input
                .get("email")
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_default();
            if email.len() > 320 || email.is_empty() || email.contains(['\r', '\n', '\0']) {
                return reset_generic_response(app.ui_language, 202);
            }
            // Keep local issue order and coordinate issuance with delivery across instances.
            let _issue_guard = match app.password_reset_issue_lock.lock() {
                Ok(guard) => guard,
                Err(_) => return reset_generic_response(app.ui_language, 202),
            };
            let active_filter = auth
                .schema
                .tables
                .iter()
                .find(|table| table.name == auth.table)
                .is_some_and(|table| table.columns.iter().any(|column| column.name == "active"));
            let user_query = zelyra_database::Query {
                sql: format!(
                    "SELECT id, email FROM {} WHERE LOWER(email) = :email{} LIMIT 1",
                    quote_identifier(&auth.table),
                    if active_filter {
                        " AND active = true"
                    } else {
                        ""
                    }
                ),
                params: vec![("email".into(), QueryValue::String(email.clone()))],
            };
            let user = zelyra_database::execute_mariadb_queries(database_url, &[user_query], false)
                .ok()
                .and_then(|rows| rows.first().cloned())
                .and_then(|result| result.rows.first().cloned());
            let mut audit = if let Some(audit_table) = auth.audit_table.as_deref() {
                audit_insert_queries(
                    audit_table,
                    auth.audit_chain,
                    None,
                    "auth.password_reset.request",
                    None,
                    &format!("email_hash={}", session_token_hash(&email)),
                )
            } else {
                Vec::new()
            };
            if let Some(row) = user {
                if let (Some(user_id), Some(recipient), Ok(token)) = (
                    row.first().and_then(|value| value.parse::<i64>().ok()),
                    row.get(1),
                    new_token(),
                ) {
                    let token_hash = session_token_hash(&token);
                    let lock_name = match zelyra_database::mariadb_advisory_lock_name(
                        database_url,
                        &format!("password-reset:{reset_table}:{user_id}"),
                    ) {
                        Ok(name) => name,
                        Err(_) => {
                            eprintln!("zelyra web: password reset lock could not be prepared");
                            return reset_generic_response(app.ui_language, 202);
                        }
                    };
                    let queries = [
                        zelyra_database::Query {
                            sql: format!(
                                "DELETE FROM {} WHERE consumed_at IS NOT NULL OR expires_at <= NOW()",
                                quote_identifier(reset_table)
                            ),
                            params: Vec::new(),
                        },
                        zelyra_database::Query {
                            sql: format!(
                                "SELECT id FROM {} WHERE id = :user_id FOR UPDATE",
                                quote_identifier(&auth.table)
                            ),
                            params: vec![("user_id".into(), QueryValue::Int(user_id))],
                        },
                        zelyra_database::Query {
                            sql: format!("DELETE FROM {} WHERE user_id = :user_id", quote_identifier(reset_table)),
                            params: vec![("user_id".into(), QueryValue::Int(user_id))],
                        },
                        zelyra_database::Query {
                            sql: format!("INSERT INTO {} (user_id, token_hash, expires_at, consumed_at) VALUES (:user_id, :token_hash, DATE_ADD(NOW(), INTERVAL {RESET_TOKEN_TTL_MINUTES} MINUTE), NULL)", quote_identifier(reset_table)),
                            params: vec![("user_id".into(), QueryValue::Int(user_id)), ("token_hash".into(), QueryValue::String(token_hash.clone()))],
                        },
                    ];
                    audit.extend(audit_insert_queries(
                        auth.audit_table.as_deref().unwrap_or(""),
                        auth.audit_chain,
                        None,
                        "auth.password_reset.issued",
                        Some(user_id),
                        "",
                    ));
                    let result = zelyra_database::with_mariadb_advisory_transaction(
                        database_url,
                        &lock_name,
                        RESET_ADVISORY_LOCK_WAIT_SECONDS,
                        |transaction| {
                            for query in audit.iter().chain(queries.iter()) {
                                transaction.execute(query)?;
                            }
                            if mailer
                                .enqueue_reset(
                                    ResetMailDeliveryGuard {
                                        database_url: database_url.to_owned(),
                                        lock_name: lock_name.clone(),
                                        reset_table: reset_table.to_owned(),
                                        token_hash: token_hash.clone(),
                                    },
                                    recipient,
                                    &token,
                                    app.ui_language,
                                )
                                .is_err()
                            {
                                transaction.execute(&zelyra_database::Query {
                                    sql: format!(
                                        "DELETE FROM {} WHERE token_hash = :token_hash",
                                        quote_identifier(reset_table)
                                    ),
                                    params: vec![(
                                        "token_hash".into(),
                                        QueryValue::String(token_hash.clone()),
                                    )],
                                })?;
                                if let Some(audit_table) = auth.audit_table.as_deref() {
                                    for query in audit_insert_queries(
                                        audit_table,
                                        auth.audit_chain,
                                        None,
                                        "auth.password_reset.delivery_unqueued",
                                        Some(user_id),
                                        "",
                                    ) {
                                        transaction.execute(&query)?;
                                    }
                                }
                            }
                            Ok(())
                        },
                    );
                    if result.is_err() {
                        eprintln!("zelyra web: password reset request could not be recorded");
                    }
                }
            } else if !audit.is_empty() {
                let _ = zelyra_database::execute_mariadb_queries(database_url, &audit, true);
            }
            reset_generic_response(app.ui_language, 202)
        }
        ("/reset-password", "GET") if request.target.contains('?') => {
            let query = request
                .target
                .split_once('?')
                .map(|(_, query)| query)
                .unwrap_or_default();
            let token = parse_urlencoded(query)
                .ok()
                .and_then(|values| values.get("token").cloned());
            let Some(token) = token.filter(|token| valid_token(token)) else {
                return reset_invalid_response(app.ui_language);
            };
            if !reset_token_exists(database_url, reset_table, &token) {
                return reset_invalid_response(app.ui_language);
            }
            Response::redirect("/reset-password")
                .with_header("Set-Cookie", format!("{RESET_COOKIE}={token}; Path={RESET_COOKIE_PATH}; HttpOnly; SameSite=Strict; Max-Age={};{}", RESET_TOKEN_TTL_MINUTES * 60, secure_cookie_attribute(request)))
                .with_header("Cache-Control", "no-store")
                .with_header("Referrer-Policy", "no-referrer")
        }
        ("/reset-password", "GET") => {
            let token = cookie_value(request, RESET_COOKIE);
            if !token.as_deref().is_some_and(valid_token)
                || !reset_token_exists(
                    database_url,
                    reset_table,
                    token.as_deref().unwrap_or_default(),
                )
            {
                return reset_invalid_response(app.ui_language);
            }
            reset_form_response(auth, app.ui_language, true)
                .with_header("Cache-Control", "no-store")
                .with_header("Referrer-Policy", "no-referrer")
        }
        ("/reset-password", "POST") => {
            let input = match parse_urlencoded(&request.body) {
                Ok(input) => input,
                Err(_) => return reset_invalid_response(app.ui_language),
            };
            let token = cookie_value(request, RESET_COOKIE);
            if !verify_csrf_request(
                request,
                &auth.csrf,
                input.get("_zelyra_csrf").map(String::as_str),
            ) || !token.as_deref().is_some_and(valid_token)
                || reset_rate_limited(
                    app,
                    &reset_rate_key(request),
                    auth.reset_rate_limit,
                    auth.reset_block_seconds,
                )
            {
                return reset_invalid_response(app.ui_language)
                    .with_header("Set-Cookie", clear_reset_cookie(request));
            }
            let password = input
                .get("password")
                .map(String::as_str)
                .unwrap_or_default();
            if password.chars().count() < 8 || password.chars().count() > 1024 {
                return reset_form_response(auth, app.ui_language, true)
                    .with_header("Cache-Control", "no-store")
                    .with_header("Referrer-Policy", "no-referrer");
            }
            let password_hash = match hash_password(password) {
                Ok(hash) => hash,
                Err(_) => return reset_invalid_response(app.ui_language),
            };
            let token_hash = session_token_hash(token.as_deref().unwrap_or_default());
            let active_filter = auth
                .schema
                .tables
                .iter()
                .find(|table| table.name == auth.table)
                .is_some_and(|table| table.columns.iter().any(|column| column.name == "active"));
            let completed = zelyra_database::with_mariadb_transaction(database_url, |transaction| {
                let token_query = zelyra_database::Query {
                    sql: format!(
                        "SELECT r.user_id FROM {} AS r JOIN {} AS u ON u.id = r.user_id WHERE r.token_hash = :token_hash AND r.consumed_at IS NULL AND r.expires_at > NOW(){} LIMIT 1 FOR UPDATE",
                        quote_identifier(reset_table),
                        quote_identifier(&auth.table),
                        if active_filter { " AND u.active = true" } else { "" }
                    ),
                    params: vec![("token_hash".into(), QueryValue::String(token_hash.clone()))],
                };
                let row = transaction.execute(&token_query)?.rows.into_iter().next();
                let Some(user_id) = row.as_ref().and_then(|row| row.first()).and_then(|value| value.parse::<i64>().ok()) else {
                    return Ok(None);
                };
                transaction.execute(&zelyra_database::Query {
                    sql: format!("UPDATE {} SET consumed_at = NOW() WHERE token_hash = :token_hash AND consumed_at IS NULL AND expires_at > NOW()", quote_identifier(reset_table)),
                    params: vec![("token_hash".into(), QueryValue::String(token_hash))],
                })?;
                transaction.execute(&zelyra_database::Query {
                    sql: format!("UPDATE {} SET password_hash = :password_hash WHERE id = :user_id", quote_identifier(&auth.table)),
                    params: vec![("password_hash".into(), QueryValue::String(password_hash)), ("user_id".into(), QueryValue::Int(user_id))],
                })?;
                if let Some(session_table) = auth.session_table.as_deref() {
                    transaction.execute(&zelyra_database::Query {
                        sql: format!("DELETE FROM {} WHERE user_id = :user_id", quote_identifier(session_table)),
                        params: vec![("user_id".into(), QueryValue::Int(user_id))],
                    })?;
                }
                if let Some(audit_table) = auth.audit_table.as_deref() {
                    for query in audit_insert_queries(audit_table, auth.audit_chain, None, "auth.password_reset.completed", Some(user_id), "") {
                        transaction.execute(&query)?;
                    }
                }
                Ok(Some(user_id))
            }).ok().flatten();
            if let Some(user_id) = completed {
                if let Ok(mut sessions) = app.sessions.lock() {
                    sessions.retain(|_, session| session.user_id != Some(user_id));
                }
                Response::redirect("/login")
                    .with_header("Set-Cookie", clear_reset_cookie(request))
                    .with_header("Cache-Control", "no-store")
                    .with_header("Referrer-Policy", "no-referrer")
            } else {
                reset_invalid_response(app.ui_language)
                    .with_header("Set-Cookie", clear_reset_cookie(request))
            }
        }
        _ => Response::empty(405).with_header("Allow", "GET, POST"),
    }
}

fn reset_form_response(auth: &AuthRoute, language: UiLanguage, with_password: bool) -> Response {
    let (path, title, field, button) = if with_password {
        (
            "/reset-password",
            tr(language, "auth.reset_password_title"),
            format!("<label for=\"password\">{}</label><input id=\"password\" name=\"password\" type=\"password\" minlength=\"8\" maxlength=\"1024\" autocomplete=\"new-password\" required>", tr(language, "auth.password")),
            tr(language, "auth.reset_password_submit"),
        )
    } else {
        (
            "/forgot-password",
            tr(language, "auth.reset_request_title"),
            format!("<label for=\"email\">{}</label><input id=\"email\" name=\"email\" type=\"email\" maxlength=\"320\" autocomplete=\"email\" required>", tr(language, "auth.email")),
            tr(language, "auth.reset_request_submit"),
        )
    };
    Response::html(200, format!("<main><h1>{}</h1><form method=\"post\" action=\"{}\"><input type=\"hidden\" name=\"_zelyra_csrf\" value=\"{}\">{}<button type=\"submit\">{}</button></form></main>", html_escape(&title), path, html_escape(auth.csrf.token()), field, html_escape(&button)))
        .with_header("Cache-Control", "no-store")
        .with_header("Referrer-Policy", "no-referrer")
}

fn reset_generic_response(language: UiLanguage, status: u16) -> Response {
    Response::html(
        status,
        format!(
            "<main><p>{}</p></main>",
            html_escape(&tr(language, "auth.reset_generic_response"))
        ),
    )
    .with_header("Cache-Control", "no-store")
    .with_header("Referrer-Policy", "no-referrer")
}

fn reset_invalid_response(language: UiLanguage) -> Response {
    Response::html(
        400,
        format!(
            "<main><p>{}</p><a href=\"/forgot-password\">{}</a></main>",
            html_escape(&tr(language, "auth.reset_invalid")),
            html_escape(&tr(language, "auth.reset_request_title"))
        ),
    )
    .with_header("Cache-Control", "no-store")
    .with_header("Referrer-Policy", "no-referrer")
}

fn reset_rate_key(request: &Request) -> String {
    let ip = request
        .remote_addr
        .map(|address| address.ip().to_string())
        .unwrap_or_else(|| "unknown".into());
    format!("ip:{ip}")
}

fn reset_rate_limited(
    app: &WebApp,
    key: &str,
    policy: zelyra_ast::ApiRateLimit,
    block_seconds: u32,
) -> bool {
    let Ok(mut throttle) = app.reset_throttle.lock() else {
        return true;
    };
    let now = Instant::now();
    throttle.retain(|_, state| {
        now.duration_since(state.window_started)
            < Duration::from_secs(u64::from(state.window_seconds))
            || state.blocked_until.is_some_and(|until| now < until)
    });
    if !throttle.contains_key(key) && throttle.len() >= LOGIN_THROTTLE_MAX_KEYS {
        return true;
    }
    let state = throttle.entry(key.to_owned()).or_insert(LoginThrottle {
        window_started: now,
        window_seconds: policy.window_seconds,
        failures: 0,
        blocked_until: None,
    });
    if state.blocked_until.is_some_and(|until| now < until) {
        return true;
    }
    if now.duration_since(state.window_started)
        >= Duration::from_secs(u64::from(policy.window_seconds))
    {
        *state = LoginThrottle {
            window_started: now,
            window_seconds: policy.window_seconds,
            failures: 0,
            blocked_until: None,
        };
    }
    if state.failures >= policy.requests {
        state.blocked_until = Some(now + Duration::from_secs(u64::from(block_seconds)));
        return true;
    }
    state.failures += 1;
    false
}

fn reset_token_exists(database_url: &str, reset_table: &str, token: &str) -> bool {
    let query = zelyra_database::Query {
        sql: format!("SELECT user_id FROM {} WHERE token_hash = :token_hash AND consumed_at IS NULL AND expires_at > NOW() LIMIT 1", quote_identifier(reset_table)),
        params: vec![("token_hash".into(), QueryValue::String(session_token_hash(token)))],
    };
    zelyra_database::execute_mariadb_queries(database_url, &[query], false)
        .ok()
        .is_some_and(|results| {
            results
                .first()
                .is_some_and(|result| !result.rows.is_empty())
        })
}

fn valid_token(token: &str) -> bool {
    token.len() == 64 && token.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn new_token() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|_| "secure token generation failed".to_owned())?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn clear_reset_cookie(request: &Request) -> String {
    format!(
        "{RESET_COOKIE}=; Path={RESET_COOKIE_PATH}; HttpOnly; SameSite=Strict; Max-Age=0;{}",
        secure_cookie_attribute(request)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_tokens_are_random_hex_and_only_hashes_are_stored() {
        let first = new_token().unwrap();
        let second = new_token().unwrap();
        assert!(valid_token(&first));
        assert_ne!(first, second);
        assert_ne!(session_token_hash(&first), first);
        assert_eq!(session_token_hash(&first).len(), 64);
    }

    #[test]
    fn reset_mail_configuration_requires_tls_except_explicit_loopback() {
        assert!(PasswordResetMailer::smtp(
            "relay.example.test",
            587,
            "local_plaintext",
            None,
            None,
            "Zelyra <no-reply@example.test>",
            "https://app.example.test"
        )
        .is_err());
        assert!(PasswordResetMailer::smtp(
            "relay.example.test",
            587,
            "starttls",
            Some("u"),
            None,
            "no-reply@example.test",
            "https://app.example.test"
        )
        .is_err());
        assert!(PasswordResetMailer::smtp(
            "127.0.0.1",
            2525,
            "local_plaintext",
            None,
            None,
            "no-reply@example.test",
            "https://app.example.test"
        )
        .is_ok());
        assert!(PasswordResetMailer::smtp(
            "relay.example.test",
            465,
            "implicit_tls",
            None,
            None,
            "no-reply@example.test",
            "http://relay.example.test"
        )
        .is_err());
        for url in [
            "https://",
            "https://user:password@app.example.test",
            "https://app.example.test/path",
            "https://app.example.test?next=/",
            "http://app.example.test",
            "http://192.0.2.1",
        ] {
            assert!(
                PasswordResetMailer::smtp(
                    "127.0.0.1",
                    2525,
                    "local_plaintext",
                    None,
                    None,
                    "no-reply@example.test",
                    url
                )
                .is_err(),
                "accepted unsafe public base URL: {url}"
            );
        }
        for url in [
            "https://app.example.test",
            "https://app.example.test/",
            "http://localhost:8080",
            "http://127.0.0.1:8080",
            "http://[::1]:8080",
        ] {
            assert!(
                PasswordResetMailer::smtp(
                    "127.0.0.1",
                    2525,
                    "local_plaintext",
                    None,
                    None,
                    "no-reply@example.test",
                    url
                )
                .is_ok(),
                "rejected valid public base URL: {url}"
            );
        }
        assert!(PasswordResetMailer::smtp(
            "127.0.0.1",
            0,
            "local_plaintext",
            None,
            None,
            "no-reply@example.test",
            "http://localhost"
        )
        .is_err());
    }

    #[test]
    fn reset_rate_limit_is_bounded_and_expires() {
        let app = WebApp::new(Vec::new(), Vec::new());
        let limit = zelyra_ast::ApiRateLimit {
            requests: 1,
            window_seconds: 30,
        };
        assert!(!reset_rate_limited(&app, "ip:127.0.0.1", limit, 60));
        assert!(reset_rate_limited(&app, "ip:127.0.0.1", limit, 60));
    }

    #[test]
    fn reset_link_sender_receives_only_clean_origin_and_token_path() {
        let sent = Arc::new(Mutex::new(None));
        let captured = sent.clone();
        let mailer =
            PasswordResetMailer::test_sender("https://app.example.test", move |_, link| {
                *captured.lock().unwrap() = Some(link.to_owned());
                Ok(())
            });
        mailer
            .send("user@example.test", &"a".repeat(64), UiLanguage::English)
            .unwrap();
        assert_eq!(
            sent.lock().unwrap().as_deref(),
            Some(
                format!(
                    "https://app.example.test/reset-password?token={}",
                    "a".repeat(64)
                )
                .as_str()
            )
        );
    }

    #[test]
    fn reset_mail_queue_is_bounded_and_fails_closed_when_full() {
        let (queue, _receiver) = sync_channel(RESET_MAIL_QUEUE_CAPACITY);
        let mailer = PasswordResetMailer {
            base_url: "https://app.example.test".into(),
            sender: Arc::new(|_, _, _| Ok(())),
            queue: Some(queue),
        };
        for _ in 0..RESET_MAIL_QUEUE_CAPACITY {
            mailer
                .send("user@example.test", &"a".repeat(64), UiLanguage::English)
                .unwrap();
        }
        assert_eq!(
            mailer
                .send("user@example.test", &"b".repeat(64), UiLanguage::English)
                .unwrap_err(),
            "password reset mail queue is full"
        );
    }
}
