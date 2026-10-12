//! Shared, bounded SMTP delivery used by Zelyra's web and language runtimes.
//!
//! Errors intentionally omit transport diagnostics and credentials. Applications
//! can report failure without leaking SMTP server responses into user output.

use lettre::{
    message::{header::ContentType, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    transport::smtp::client::{Certificate, Tls, TlsParameters},
    Message, SmtpTransport, Transport,
};
use std::time::Duration;

pub const MAX_MAIL_BODY_BYTES: usize = 64 * 1024;
pub const SMTP_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmtpSecurity {
    ImplicitTls,
    StartTls,
    LocalPlaintext,
}

impl SmtpSecurity {
    pub fn parse(value: &str) -> Result<Self, MailError> {
        match value {
            "implicit_tls" => Ok(Self::ImplicitTls),
            "starttls" => Ok(Self::StartTls),
            "local_plaintext" => Ok(Self::LocalPlaintext),
            _ => Err(MailError::Configuration),
        }
    }
}

pub struct SmtpConfig<'a> {
    pub host: &'a str,
    pub port: u16,
    pub security: SmtpSecurity,
    pub username: Option<&'a str>,
    pub password: Option<&'a str>,
    pub ca_certificate_pem: Option<&'a [u8]>,
    pub from: &'a str,
}

pub struct MailMessage<'a> {
    pub to: &'a str,
    pub subject: &'a str,
    pub text_body: &'a str,
    pub html_body: Option<&'a str>,
    pub message_id: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailError {
    Configuration,
    Message,
    Delivery,
}

impl std::fmt::Display for MailError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Configuration => "SMTP configuration is invalid or incomplete",
            Self::Message => "email message is invalid or exceeds its size limit",
            Self::Delivery => "SMTP delivery failed",
        })
    }
}

impl std::error::Error for MailError {}

#[derive(Clone)]
pub struct SmtpClient {
    transport: SmtpTransport,
    from: Mailbox,
}

impl std::fmt::Debug for SmtpClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SmtpClient")
            .field("from", &self.from)
            .field("credentials", &"<redacted>")
            .finish()
    }
}

impl SmtpClient {
    pub fn new(config: SmtpConfig<'_>) -> Result<Self, MailError> {
        let SmtpConfig {
            host,
            port,
            security,
            username,
            password,
            ca_certificate_pem,
            from,
        } = config;
        if host.is_empty() || host.contains(['/', '\\', '@', '?', '#', '\r', '\n']) || port == 0 {
            return Err(MailError::Configuration);
        }
        if username.is_some() != password.is_some() {
            return Err(MailError::Configuration);
        }
        if security == SmtpSecurity::LocalPlaintext
            && (!matches!(host, "localhost" | "127.0.0.1" | "::1") || username.is_some())
        {
            return Err(MailError::Configuration);
        }
        if ca_certificate_pem.is_some() && security == SmtpSecurity::LocalPlaintext {
            return Err(MailError::Configuration);
        }
        let mut builder = if let Some(ca_certificate_pem) = ca_certificate_pem {
            let certificate = parse_ca_certificate(ca_certificate_pem)?;
            let tls = TlsParameters::builder(host.to_owned())
                .add_root_certificate(certificate)
                .build()
                .map_err(|_| MailError::Configuration)?;
            let mode = match security {
                SmtpSecurity::ImplicitTls => Tls::Wrapper(tls),
                SmtpSecurity::StartTls => Tls::Required(tls),
                SmtpSecurity::LocalPlaintext => unreachable!("rejected above"),
            };
            SmtpTransport::builder_dangerous(host).tls(mode)
        } else {
            match security {
                SmtpSecurity::ImplicitTls => SmtpTransport::relay(host),
                SmtpSecurity::StartTls => SmtpTransport::starttls_relay(host),
                SmtpSecurity::LocalPlaintext => Ok(SmtpTransport::builder_dangerous(host)),
            }
            .map_err(|_| MailError::Configuration)?
        }
        .port(port)
        .timeout(Some(SMTP_TIMEOUT));
        if let (Some(username), Some(password)) = (username, password) {
            builder =
                builder.credentials(Credentials::new(username.to_owned(), password.to_owned()));
        }
        let from = parse_mailbox(from)?;
        Ok(Self {
            transport: builder.build(),
            from,
        })
    }

    pub fn send(&self, message: MailMessage<'_>) -> Result<(), MailError> {
        validate_message(&message)?;
        let to = parse_mailbox(message.to)?;
        let mut builder = Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject(message.subject);
        if let Some(id) = message.message_id {
            builder = builder.message_id(Some(format!("<{id}>")));
        }
        let email = if let Some(html) = message.html_body {
            builder
                .multipart(
                    MultiPart::alternative()
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_PLAIN)
                                .body(message.text_body.to_owned()),
                        )
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_HTML)
                                .body(html.to_owned()),
                        ),
                )
                .map_err(|_| MailError::Message)?
        } else {
            builder
                .header(ContentType::TEXT_PLAIN)
                .body(message.text_body.to_owned())
                .map_err(|_| MailError::Message)?
        };
        self.transport
            .send(&email)
            .map(|_| ())
            .map_err(|_| MailError::Delivery)
    }
}

fn parse_mailbox(address: &str) -> Result<Mailbox, MailError> {
    if address.is_empty() || address.len() > 320 || address.contains(['\r', '\n', '\0']) {
        return Err(MailError::Message);
    }
    address.parse::<Mailbox>().map_err(|_| MailError::Message)
}

pub fn validate_message(message: &MailMessage<'_>) -> Result<(), MailError> {
    parse_mailbox(message.to)?;
    if message.subject.is_empty()
        || message.subject.len() > 998
        || message.subject.contains(['\r', '\n', '\0'])
        || message.text_body.len() + message.html_body.unwrap_or_default().len()
            > MAX_MAIL_BODY_BYTES
        || message.message_id.is_some_and(|id| {
            id.is_empty()
                || id.len() > 128
                || !id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-._@".contains(&byte))
        })
    {
        return Err(MailError::Message);
    }
    Ok(())
}

pub fn validate_ca_certificate(pem: &[u8]) -> Result<(), MailError> {
    parse_ca_certificate(pem).map(|_| ())
}

fn parse_ca_certificate(pem: &[u8]) -> Result<Certificate, MailError> {
    const BEGIN: &[u8] = b"-----BEGIN CERTIFICATE-----";
    const END: &[u8] = b"-----END CERTIFICATE-----";
    if !pem.windows(BEGIN.len()).any(|window| window == BEGIN)
        || !pem.windows(END.len()).any(|window| window == END)
    {
        return Err(MailError::Configuration);
    }
    Certificate::from_pem(pem).map_err(|_| MailError::Configuration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_or_incomplete_smtp_configuration() {
        for config in [
            SmtpConfig {
                host: "smtp.example.test/path",
                port: 465,
                security: SmtpSecurity::ImplicitTls,
                username: None,
                password: None,
                ca_certificate_pem: None,
                from: "sender@example.test",
            },
            SmtpConfig {
                host: "smtp.example.test",
                port: 0,
                security: SmtpSecurity::ImplicitTls,
                username: None,
                password: None,
                ca_certificate_pem: None,
                from: "sender@example.test",
            },
            SmtpConfig {
                host: "smtp.example.test",
                port: 465,
                security: SmtpSecurity::ImplicitTls,
                username: Some("user"),
                password: None,
                ca_certificate_pem: None,
                from: "sender@example.test",
            },
            SmtpConfig {
                host: "mail.example.test",
                port: 25,
                security: SmtpSecurity::LocalPlaintext,
                username: None,
                password: None,
                ca_certificate_pem: None,
                from: "sender@example.test",
            },
            SmtpConfig {
                host: "localhost",
                port: 25,
                security: SmtpSecurity::LocalPlaintext,
                username: Some("user"),
                password: Some("secret"),
                ca_certificate_pem: None,
                from: "sender@example.test",
            },
            SmtpConfig {
                host: "localhost",
                port: 25,
                security: SmtpSecurity::ImplicitTls,
                username: None,
                password: None,
                ca_certificate_pem: None,
                from: "not-an-address",
            },
        ] {
            assert!(SmtpClient::new(config).is_err());
        }
    }

    #[test]
    fn rejects_invalid_custom_ca_and_plaintext_ca_configuration() {
        let invalid_pem = b"-----BEGIN CERTIFICATE-----\nnot-base64\n-----END CERTIFICATE-----";
        assert!(SmtpClient::new(SmtpConfig {
            host: "smtp.example.test",
            port: 587,
            security: SmtpSecurity::StartTls,
            username: None,
            password: None,
            ca_certificate_pem: Some(invalid_pem),
            from: "sender@example.test",
        })
        .is_err());
        assert!(SmtpClient::new(SmtpConfig {
            host: "localhost",
            port: 25,
            security: SmtpSecurity::LocalPlaintext,
            username: None,
            password: None,
            ca_certificate_pem: Some(invalid_pem),
            from: "sender@example.test",
        })
        .is_err());
    }

    #[test]
    fn rejects_header_injection_and_oversized_body() {
        let bad_header = MailMessage {
            to: "user@example.test",
            subject: "hello\r\nBcc: victim@example.test",
            text_body: "body",
            html_body: None,
            message_id: None,
        };
        assert_eq!(validate_message(&bad_header), Err(MailError::Message));
        let too_large = "x".repeat(MAX_MAIL_BODY_BYTES + 1);
        let bad_body = MailMessage {
            to: "user@example.test",
            subject: "hello",
            text_body: &too_large,
            html_body: None,
            message_id: None,
        };
        assert_eq!(validate_message(&bad_body), Err(MailError::Message));
        let bad_id = MailMessage {
            to: "user@example.test",
            subject: "hello",
            text_body: "body",
            html_body: None,
            message_id: Some("bad\r\nid"),
        };
        assert_eq!(validate_message(&bad_id), Err(MailError::Message));
    }

    #[test]
    fn accepts_local_sink_and_unicode_content() {
        let message = MailMessage {
            to: "Änne <aenne@example.test>",
            subject: "Grüße aus Zelyra",
            text_body: "Hallo, Grüße!",
            html_body: Some("<p>Hallo, Grüße!</p>"),
            message_id: Some("outbox-01@example.test"),
        };
        assert_eq!(validate_message(&message), Ok(()));
        assert_eq!(
            SmtpSecurity::parse("local_plaintext"),
            Ok(SmtpSecurity::LocalPlaintext)
        );
        assert_eq!(
            SmtpSecurity::parse("tls_fallback"),
            Err(MailError::Configuration)
        );
    }
}
