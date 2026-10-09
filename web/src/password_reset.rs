use super::*;
use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    Message, SmtpTransport, Transport,
};
use rand_core::{OsRng, RngCore};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use std::sync::mpsc::{SyncSender, TrySendError};
#[cfg(not(test))]
use std::{sync::mpsc::sync_channel, thread, time::Duration};
use zeroize::Zeroize;

const RESET_TOKEN_TTL_MINUTES: u32 = 15;
const RESET_COOKIE: &str = "zelyra_password_reset";
const RESET_COOKIE_PATH: &str = "/reset-password";
const RESET_ADVISORY_LOCK_WAIT_SECONDS: u32 = 10;
#[cfg(not(test))]
const RESET_OUTBOX_RETRY_DELAY_SECONDS: u32 = 30;
#[cfg(not(test))]
const RESET_OUTBOX_POLL_INTERVAL: Duration = Duration::from_secs(5);
type ResetMailSender = dyn Fn(&str, &str, UiLanguage) -> Result<(), String> + Send + Sync;

struct ResetDeliveryKey([u8; 32]);

impl Drop for ResetDeliveryKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

pub struct PasswordResetSmtpConfig<'a> {
    pub host: &'a str,
    pub port: u16,
    pub security: &'a str,
    pub username: Option<&'a str>,
    pub password: Option<&'a str>,
    pub from: &'a str,
    pub base_url: &'a str,
    pub database_url: &'a str,
    pub reset_table: &'a str,
    pub delivery_key: &'a str,
}

#[cfg(not(test))]
struct ResetMailJob {
    email: String,
    link: String,
    language: UiLanguage,
    delivery_guard: Option<ResetMailDeliveryGuard>,
}

struct ResetMailDeliveryGuard {
    #[cfg_attr(test, allow(dead_code))]
    database_url: String,
    #[cfg_attr(test, allow(dead_code))]
    lock_name: String,
    reset_table: String,
    token_hash: String,
}

#[derive(Clone)]
pub struct PasswordResetMailer {
    base_url: String,
    delivery_key: Option<Arc<ResetDeliveryKey>>,
    #[cfg(test)]
    sender: Arc<ResetMailSender>,
    queue: Option<SyncSender<()>>,
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
    pub fn smtp_with_outbox(config: PasswordResetSmtpConfig<'_>) -> Result<Self, String> {
        let PasswordResetSmtpConfig {
            host,
            port,
            security,
            username,
            password,
            from,
            base_url,
            database_url,
            reset_table,
            delivery_key,
        } = config;
        #[cfg(test)]
        let _ = (database_url, reset_table);
        validate_public_base_url(base_url)?;
        if host.is_empty() || host.contains(['/', '\\', '@', '?', '#', '\r', '\n']) {
            return Err("ZELYRA_SMTP_HOST must be a hostname without a URL or credentials".into());
        }
        if port == 0 {
            return Err("ZELYRA_SMTP_PORT must be between 1 and 65535".into());
        }
        let delivery_key = Arc::new(ResetDeliveryKey(parse_delivery_key(delivery_key)?));
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
        #[cfg(not(test))]
        let queue = {
            let (queue, receiver) = sync_channel::<()>(1);
            let worker_sender = sender.clone();
            let worker_database_url = database_url.to_owned();
            let worker_reset_table = reset_table.to_owned();
            let worker_delivery_key = delivery_key.clone();
            thread::Builder::new()
                .name("zelyra-password-reset-mail".into())
                .spawn(move || loop {
                    let _ = receiver.recv_timeout(RESET_OUTBOX_POLL_INTERVAL);
                    if let Err(error) = process_reset_outbox(
                        &worker_database_url,
                        &worker_reset_table,
                        &worker_delivery_key,
                        &worker_sender,
                    ) {
                        eprintln!("zelyra web: password reset outbox recovery failed: {error}");
                    }
                })
                .map_err(|_| "password reset mail worker could not be started".to_owned())?;
            Some(queue)
        };
        #[cfg(test)]
        let queue = None;
        Ok(Self {
            base_url,
            delivery_key: Some(delivery_key),
            #[cfg(test)]
            sender,
            queue,
        })
    }

    #[cfg(test)]
    fn smtp(
        host: &str,
        port: u16,
        security: &str,
        username: Option<&str>,
        password: Option<&str>,
        from: &str,
        base_url: &str,
    ) -> Result<Self, String> {
        Self::smtp_with_outbox(PasswordResetSmtpConfig {
            host,
            port,
            security,
            username,
            password,
            from,
            base_url,
            database_url: "mariadb://test:test@127.0.0.1/test",
            reset_table: "password_resets",
            delivery_key: &"00".repeat(32),
        })
    }

    #[cfg(test)]
    fn test_sender(
        base_url: &str,
        sender: impl Fn(&str, &str) -> Result<(), String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            delivery_key: None,
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
        queue.try_send(()).map_err(|error| match error {
            TrySendError::Full(_) => "password reset mail queue is full".to_owned(),
            TrySendError::Disconnected(_) => "password reset mail worker is unavailable".to_owned(),
        })
    }

    fn enqueue_reset(
        &self,
        transaction: &mut zelyra_database::MariaDbTransaction<'_>,
        delivery_guard: ResetMailDeliveryGuard,
        email: &str,
        token: &str,
        language: UiLanguage,
    ) -> Result<(), String> {
        let Some(queue) = self.queue.as_ref() else {
            return Err("password reset mail queue is unavailable".into());
        };
        let link = format!("{}/reset-password?token={token}", self.base_url);
        let payload = serde_json::to_vec(&serde_json::json!({
            "email": email,
            "link": link,
            "language": match language {
                UiLanguage::English => "en",
                UiLanguage::German => "de",
            },
        }))
        .map_err(|_| "password reset delivery payload could not be encoded".to_owned())?;
        let key = self
            .delivery_key
            .as_ref()
            .ok_or_else(|| "password reset delivery encryption is unavailable".to_owned())?;
        let encrypted_payload = encrypt_delivery_payload(&payload, &key.0)?;
        transaction
            .execute(&zelyra_database::Query {
                sql: format!(
                    "UPDATE {} SET delivery_payload = :payload, delivery_retry_at = NULL WHERE token_hash = :token_hash",
                    quote_identifier(&delivery_guard.reset_table)
                ),
                params: vec![
                    ("payload".into(), QueryValue::String(encrypted_payload)),
                    (
                        "token_hash".into(),
                        QueryValue::String(delivery_guard.token_hash),
                    ),
                ],
            })
            .map_err(|_| "password reset delivery could not be persisted".to_owned())?;
        match queue.try_send(()) {
            Ok(()) | Err(TrySendError::Full(())) => Ok(()),
            Err(TrySendError::Disconnected(())) => {
                Err("password reset mail worker is unavailable".to_owned())
            }
        }
    }
}

#[cfg(not(test))]
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
            let query = active_pending_reset_query(&guard.reset_table, &guard.token_hash);
            let current = !transaction.execute(&query)?.rows.is_empty();
            if !current {
                transaction.execute(&zelyra_database::Query {
                    sql: format!(
                        "UPDATE {} SET delivery_payload = NULL, delivery_retry_at = NULL WHERE token_hash = :token_hash",
                        quote_identifier(&guard.reset_table)
                    ),
                    params: vec![(
                        "token_hash".into(),
                        QueryValue::String(guard.token_hash.clone()),
                    )],
                })?;
                return Ok(None);
            }
            let delivery = sender(&job.email, &job.link, job.language);
            if delivery.is_ok() {
                transaction.execute(&zelyra_database::Query {
                    sql: format!(
                        "UPDATE {} SET delivery_payload = NULL, delivery_retry_at = NULL WHERE token_hash = :token_hash",
                        quote_identifier(&guard.reset_table)
                    ),
                    params: vec![(
                        "token_hash".into(),
                        QueryValue::String(guard.token_hash.clone()),
                    )],
                })?;
            } else {
                transaction.execute(&zelyra_database::Query {
                    sql: format!(
                        "UPDATE {} SET delivery_retry_at = DATE_ADD(NOW(), INTERVAL {RESET_OUTBOX_RETRY_DELAY_SECONDS} SECOND) WHERE token_hash = :token_hash",
                        quote_identifier(&guard.reset_table)
                    ),
                    params: vec![(
                        "token_hash".into(),
                        QueryValue::String(guard.token_hash.clone()),
                    )],
                })?;
            }
            Ok(Some(delivery))
        },
    );
    match result {
        Ok(None) => Ok(()),
        Ok(Some(delivery)) => delivery,
        Err(_) => Err("could not verify the current password reset token".into()),
    }
}

fn active_pending_reset_query(reset_table: &str, token_hash: &str) -> zelyra_database::Query {
    zelyra_database::Query {
        sql: format!(
            "SELECT user_id FROM {} WHERE token_hash = :token_hash AND delivery_payload IS NOT NULL AND (delivery_retry_at IS NULL OR delivery_retry_at <= NOW()) AND {} LIMIT 1 FOR UPDATE",
            quote_identifier(reset_table),
            active_reset_token_predicate(None)
        ),
        params: vec![(
            "token_hash".into(),
            QueryValue::String(token_hash.to_owned()),
        )],
    }
}

#[cfg(not(test))]
fn defer_invalid_reset_delivery(
    database_url: &str,
    reset_table: &str,
    token_hash: &str,
) -> Result<(), String> {
    let query = zelyra_database::Query {
        sql: format!(
            "UPDATE {} SET delivery_retry_at = DATE_ADD(NOW(), INTERVAL {RESET_OUTBOX_RETRY_DELAY_SECONDS} SECOND) WHERE token_hash = :token_hash AND delivery_payload IS NOT NULL",
            quote_identifier(reset_table)
        ),
        params: vec![(
            "token_hash".into(),
            QueryValue::String(token_hash.to_owned()),
        )],
    };
    zelyra_database::execute_mariadb_queries(database_url, &[query], false)
        .map(|_| ())
        .map_err(|_| "could not defer invalid password reset delivery".to_owned())
}

#[cfg(not(test))]
fn process_reset_outbox(
    database_url: &str,
    reset_table: &str,
    delivery_key: &ResetDeliveryKey,
    sender: &Arc<ResetMailSender>,
) -> Result<(), String> {
    let query = zelyra_database::Query {
        sql: format!(
            "SELECT user_id, token_hash, delivery_payload FROM {} WHERE delivery_payload IS NOT NULL AND (delivery_retry_at IS NULL OR delivery_retry_at <= NOW()) ORDER BY id LIMIT 16",
            quote_identifier(reset_table)
        ),
        params: Vec::new(),
    };
    let rows = zelyra_database::execute_mariadb_queries(database_url, &[query], false)
        .map_err(|_| "could not read pending password reset deliveries".to_owned())?;
    let Some(result) = rows.first() else {
        return Ok(());
    };
    for row in &result.rows {
        let (Some(user_id), Some(token_hash), Some(encrypted_payload)) = (
            row.first().and_then(|value| value.parse::<i64>().ok()),
            row.get(1),
            row.get(2),
        ) else {
            continue;
        };
        let payload = match decrypt_delivery_payload(encrypted_payload, &delivery_key.0) {
            Ok(payload) => payload,
            Err(error) => {
                eprintln!(
                    "zelyra web: pending password reset delivery could not be decrypted: {error}"
                );
                defer_invalid_reset_delivery(database_url, reset_table, token_hash)?;
                continue;
            }
        };
        let (Some(email), Some(link), Some(language)) = (
            payload.get("email").and_then(serde_json::Value::as_str),
            payload.get("link").and_then(serde_json::Value::as_str),
            payload.get("language").and_then(serde_json::Value::as_str),
        ) else {
            eprintln!("zelyra web: pending password reset delivery payload is invalid");
            defer_invalid_reset_delivery(database_url, reset_table, token_hash)?;
            continue;
        };
        let Some(language) = UiLanguage::parse(language) else {
            eprintln!("zelyra web: pending password reset delivery language is invalid");
            defer_invalid_reset_delivery(database_url, reset_table, token_hash)?;
            continue;
        };
        let lock_name = zelyra_database::mariadb_advisory_lock_name(
            database_url,
            &format!("password-reset:{reset_table}:{user_id}"),
        )
        .map_err(|_| "could not prepare password reset delivery lock".to_owned())?;
        let job = ResetMailJob {
            email: email.to_owned(),
            link: link.to_owned(),
            language,
            delivery_guard: Some(ResetMailDeliveryGuard {
                database_url: database_url.to_owned(),
                lock_name,
                reset_table: reset_table.to_owned(),
                token_hash: token_hash.to_owned(),
            }),
        };
        let guard = job
            .delivery_guard
            .as_ref()
            .expect("outbox guard is present");
        if let Err(error) = deliver_if_current_reset(&job, guard, sender) {
            eprintln!("zelyra web: password reset message delivery failed: {error}");
        }
    }
    Ok(())
}

fn parse_delivery_key(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(
            "ZELYRA_RESET_DELIVERY_KEY must contain exactly 64 hexadecimal characters".into(),
        );
    }
    let mut key = [0u8; 32];
    for (index, byte) in key.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| "ZELYRA_RESET_DELIVERY_KEY is invalid".to_owned())?;
    }
    Ok(key)
}

fn encrypt_delivery_payload(payload: &[u8], key: &[u8; 32]) -> Result<String, String> {
    let unbound = UnboundKey::new(&AES_256_GCM, key)
        .map_err(|_| "password reset delivery encryption is unavailable".to_owned())?;
    let key = LessSafeKey::new(unbound);
    let mut nonce_bytes = [0u8; 12];
    OsRng
        .try_fill_bytes(&mut nonce_bytes)
        .map_err(|_| "secure password reset delivery nonce generation failed".to_owned())?;
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut ciphertext = payload.to_vec();
    key.seal_in_place_append_tag(nonce, Aad::from(b"zelyra-reset-mail-v1"), &mut ciphertext)
        .map_err(|_| "password reset delivery encryption failed".to_owned())?;
    let mut packed = nonce_bytes.to_vec();
    packed.extend(ciphertext);
    Ok(packed.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn decrypt_delivery_payload(encrypted: &str, key: &[u8; 32]) -> Result<serde_json::Value, String> {
    if encrypted.len() < 56 || !encrypted.len().is_multiple_of(2) {
        return Err("encrypted payload is malformed".into());
    }
    let packed = (0..encrypted.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&encrypted[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "encrypted payload is malformed".to_owned())?;
    let nonce_bytes: [u8; 12] = packed[..12]
        .try_into()
        .map_err(|_| "encrypted payload is malformed".to_owned())?;
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let unbound = UnboundKey::new(&AES_256_GCM, key)
        .map_err(|_| "password reset delivery decryption is unavailable".to_owned())?;
    let key = LessSafeKey::new(unbound);
    let mut ciphertext = packed[12..].to_vec();
    let plaintext = key
        .open_in_place(nonce, Aad::from(b"zelyra-reset-mail-v1"), &mut ciphertext)
        .map_err(|_| "encrypted payload authentication failed".to_owned())?;
    serde_json::from_slice(plaintext).map_err(|_| "decrypted payload is malformed".to_owned())
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
                                    transaction,
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
                        "SELECT r.user_id FROM {} AS r JOIN {} AS u ON u.id = r.user_id WHERE r.token_hash = :token_hash AND {}{} LIMIT 1 FOR UPDATE",
                        quote_identifier(reset_table),
                        quote_identifier(&auth.table),
                        active_reset_token_predicate(Some("r")),
                        if active_filter { " AND u.active = true" } else { "" }
                    ),
                    params: vec![("token_hash".into(), QueryValue::String(token_hash.clone()))],
                };
                let row = transaction.execute(&token_query)?.rows.into_iter().next();
                let Some(user_id) = row.as_ref().and_then(|row| row.first()).and_then(|value| value.parse::<i64>().ok()) else {
                    return Ok(None);
                };
                transaction.execute(&zelyra_database::Query {
                    sql: format!("UPDATE {} SET consumed_at = NOW() WHERE token_hash = :token_hash AND {}", quote_identifier(reset_table), active_reset_token_predicate(None)),
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
    let query = active_reset_token_query(reset_table, &session_token_hash(token));
    zelyra_database::execute_mariadb_queries(database_url, &[query], false)
        .ok()
        .is_some_and(|results| {
            results
                .first()
                .is_some_and(|result| !result.rows.is_empty())
        })
}

fn active_reset_token_query(reset_table: &str, token_hash: &str) -> zelyra_database::Query {
    zelyra_database::Query {
        sql: format!(
            "SELECT user_id FROM {} WHERE token_hash = :token_hash AND {} LIMIT 1",
            quote_identifier(reset_table),
            active_reset_token_predicate(None)
        ),
        params: vec![(
            "token_hash".into(),
            QueryValue::String(token_hash.to_owned()),
        )],
    }
}

fn active_reset_token_predicate(alias: Option<&str>) -> String {
    let prefix = alias.map_or_else(String::new, |alias| format!("{alias}."));
    format!("{prefix}consumed_at IS NULL AND {prefix}expires_at > NOW()")
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
    use std::time::{SystemTime, UNIX_EPOCH};

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
    fn reset_delivery_payload_is_authenticated_and_encrypted() {
        let key = [42u8; 32];
        let message = br#"{"email":"person@example.test","link":"https://example.test/reset-password?token=secret","language":"en"}"#;
        let encrypted = encrypt_delivery_payload(message, &key).unwrap();
        assert!(!encrypted.contains("person@example.test"));
        assert!(!encrypted.contains("secret"));
        let decrypted = decrypt_delivery_payload(&encrypted, &key).unwrap();
        assert_eq!(decrypted["email"], "person@example.test");
        assert_eq!(decrypted["language"], "en");

        let mut tampered = encrypted.into_bytes();
        tampered[30] = if tampered[30] == b'0' { b'1' } else { b'0' };
        let tampered = String::from_utf8(tampered).unwrap();
        assert!(decrypt_delivery_payload(&tampered, &key).is_err());
    }

    #[test]
    fn reset_delivery_key_requires_32_bytes_of_hex() {
        assert_eq!(parse_delivery_key(&"ab".repeat(32)).unwrap(), [0xabu8; 32]);
        assert!(parse_delivery_key("too-short").is_err());
        assert!(parse_delivery_key(&"gg".repeat(32)).is_err());
    }

    #[test]
    fn reset_delivery_guard_requires_an_outbox_item_still_pending() {
        let query = active_pending_reset_query("password_resets", &"ab".repeat(32));
        assert!(query.sql.contains("delivery_payload IS NOT NULL"));
        assert!(query
            .sql
            .contains("delivery_retry_at IS NULL OR delivery_retry_at <= NOW()"));
        assert!(query.sql.contains("consumed_at IS NULL"));
        assert!(query.sql.contains("expires_at > NOW()"));
        assert_eq!(query.params.len(), 1);
    }

    #[test]
    #[ignore = "requires the MariaDB password-reset E2E fixture"]
    fn reset_token_expiry_is_exclusive_at_the_frozen_database_clock() {
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL is required for the password-reset clock test");
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be after Unix epoch")
            .as_nanos();
        let token_hash = session_token_hash(&format!("clock-boundary-{unique}"));

        let (
            is_exact_expiry,
            equality_rejected,
            valid_before_expiry,
            invalid_after_expiry,
        ) = zelyra_database::with_mariadb_transaction(&database_url, |transaction| {
            let fixture_user = transaction.execute(&zelyra_database::Query {
                sql: "SELECT id FROM users WHERE email LIKE 'zelyra-reset-%@example.test' ORDER BY id DESC LIMIT 1".into(),
                params: Vec::new(),
            })?;
            let user_id = fixture_user
                .rows
                .first()
                .and_then(|row| row.first())
                .and_then(|value| value.parse::<i64>().ok())
                .ok_or_else(|| zelyra_database::DatabaseError {
                    message: "password-reset E2E fixture user is missing".into(),
                })?;
            transaction.execute(&zelyra_database::Query {
                sql: "SET timestamp = 1700000000".into(),
                params: Vec::new(),
            })?;
            transaction.execute(&zelyra_database::Query {
                sql: "INSERT INTO password_resets (user_id, token_hash, expires_at, consumed_at) VALUES (:user_id, :token_hash, NOW(), NULL)".into(),
                params: vec![
                    ("user_id".into(), QueryValue::Int(user_id)),
                    ("token_hash".into(), QueryValue::String(token_hash.clone())),
                ],
            })?;

            let exact_expiry = transaction.execute(&zelyra_database::Query {
                sql: "SELECT expires_at = NOW() FROM password_resets WHERE token_hash = :token_hash".into(),
                params: vec![("token_hash".into(), QueryValue::String(token_hash.clone()))],
            })?;
            let is_exact_expiry = exact_expiry
                .rows
                .first()
                .and_then(|row| row.first())
                .is_some_and(|value| value == "1");

            let active_at_equality = transaction.execute(&active_reset_token_query("password_resets", &token_hash))?;
            let equality_rejected = active_at_equality.rows.is_empty();

            transaction.execute(&zelyra_database::Query {
                sql: "SET timestamp = 1699999999".into(),
                params: Vec::new(),
            })?;
            let active_before_expiry = transaction.execute(&active_reset_token_query("password_resets", &token_hash))?;
            let valid_before_expiry = active_before_expiry.rows.len() == 1;

            transaction.execute(&zelyra_database::Query {
                sql: "SET timestamp = 1700000001".into(),
                params: Vec::new(),
            })?;
            let active_after_expiry = transaction.execute(&active_reset_token_query("password_resets", &token_hash))?;
            let invalid_after_expiry = active_after_expiry.rows.is_empty();

            transaction.execute(&zelyra_database::Query {
                sql: "DELETE FROM password_resets WHERE token_hash = :token_hash".into(),
                params: vec![("token_hash".into(), QueryValue::String(token_hash))],
            })?;
            transaction.execute(&zelyra_database::Query {
                sql: "SET timestamp = 0".into(),
                params: Vec::new(),
            })?;
            Ok((is_exact_expiry, equality_rejected, valid_before_expiry, invalid_after_expiry))
        })
        .expect("the exact token-expiry boundary must be checked on one MariaDB connection");
        assert!(
            is_exact_expiry,
            "the fixture expiry must equal the frozen database time"
        );
        assert!(
            equality_rejected,
            "a token expiring exactly now must be rejected"
        );
        assert!(
            valid_before_expiry,
            "a token must be valid immediately before expiry"
        );
        assert!(
            invalid_after_expiry,
            "a token must be invalid immediately after expiry"
        );
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
}
