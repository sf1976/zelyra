use crate::{HostServices, MailPolicy, NetworkPolicy, RuntimeError, Value};
use zelyra_ast::Span;
use zelyra_mail::{MailAttachment, MailMessage, SmtpClient, SmtpConfig, SmtpSecurity};

type DecodedAttachment = (String, String, Vec<u8>);

pub fn supports_mail_outbox_database(database_url: &str) -> bool {
    database_url.starts_with("mariadb://") || database_url.starts_with("mysql://")
}

/// Build the transactional outbox INSERT for an already evaluated
/// `mail_queue(...)` call. Capability and database-backend checks belong to the
/// interpreter before this helper is called.
pub fn prepare_mail_queue_query(
    values: &[Value],
    argument_spans: &[Span],
    call_span: Span,
    host_services: &dyn HostServices,
) -> Result<zelyra_database::Query, RuntimeError> {
    if !matches!(values.len(), 3..=6) {
        return Err(RuntimeError {
            message: "mail_queue expects recipient, subject, text body, optional HTML body, and optional PDF filename and Bytes".into(),
            span: call_span,
        });
    }
    let string_argument = |index: usize| -> Result<String, RuntimeError> {
        match values.get(index) {
            Some(Value::String(value)) => Ok(value.clone()),
            Some(value) => Err(RuntimeError {
                message: format!("mail_queue expects String argument, found {}", value.ty()),
                span: argument_spans.get(index).copied().unwrap_or(call_span),
            }),
            None => Err(RuntimeError {
                message: "mail_queue is missing a required argument".into(),
                span: call_span,
            }),
        }
    };
    let to = string_argument(0)?;
    let subject = string_argument(1)?;
    let text_body = string_argument(2)?;
    let html = match values.len() {
        4 | 6 => Some(string_argument(3)?),
        _ => None,
    };
    let attachment_start = match values.len() {
        5 => Some(3),
        6 => Some(4),
        _ => None,
    };
    let attachment_storage = if let Some(start) = attachment_start {
        let filename = string_argument(start)?;
        let bytes = match values.get(start + 1) {
            Some(Value::Bytes(bytes)) => bytes.clone(),
            Some(value) => {
                return Err(RuntimeError {
                    message: format!(
                        "mail_queue PDF attachment expects Bytes, found {}",
                        value.ty()
                    ),
                    span: argument_spans.get(start + 1).copied().unwrap_or(call_span),
                });
            }
            None => {
                return Err(RuntimeError {
                    message: "mail_queue is missing PDF attachment bytes".into(),
                    span: call_span,
                });
            }
        };
        Some((filename, bytes))
    } else {
        None
    };
    let attachment_refs = attachment_storage
        .as_ref()
        .map(|(filename, bytes)| {
            vec![MailAttachment {
                filename,
                content_type: "application/pdf",
                bytes,
            }]
        })
        .unwrap_or_default();
    let message = MailMessage {
        to: &to,
        subject: &subject,
        text_body: &text_body,
        html_body: html.as_deref(),
        message_id: None,
        attachments: &attachment_refs,
    };
    zelyra_mail::validate_message(&message).map_err(|_| RuntimeError {
        message: "mail_queue received an invalid message".into(),
        span: call_span,
    })?;

    let attachments = attachment_storage
        .as_ref()
        .map(|(filename, bytes)| {
            vec![serde_json::json!({
                "filename": filename,
                "content_type": "application/pdf",
                "content": super::base64_encode_bytes(bytes),
            })]
        })
        .unwrap_or_default();
    let payload = serde_json::json!({
        "to": &to,
        "subject": &subject,
        "text": &text_body,
        "html": html,
        "attachments": attachments,
    })
    .to_string();
    if payload.len() > zelyra_database::outbox::MAX_EVENT_PAYLOAD_BYTES {
        return Err(RuntimeError {
            message: "mail_queue payload exceeds the outbox size limit".into(),
            span: call_span,
        });
    }

    let mut random = [0_u8; 16];
    host_services
        .fill_random(&mut random)
        .map_err(|_| RuntimeError {
            message: "mail_queue could not create a unique event ID".into(),
            span: call_span,
        })?;
    let event_id = random
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(zelyra_database::Query {
        sql: "INSERT INTO `_zelyra_outbox` (`event_id`, `event_type`, `payload`) VALUES (:event_id, :event_type, :payload)".into(),
        params: vec![
            ("event_id".into(), zelyra_database::QueryValue::String(event_id)),
            ("event_type".into(), zelyra_database::QueryValue::String("zelyra.mail.send".into())),
            ("payload".into(), zelyra_database::QueryValue::String(payload)),
        ],
    })
}

pub(crate) fn deliver_outbox_message(
    event_id: &str,
    payload_json: &str,
    mail: Option<&MailPolicy>,
    network: Option<&NetworkPolicy>,
) -> Result<(), &'static str> {
    let payload: serde_json::Value =
        zelyra_ast::json::parse(payload_json).map_err(|_| "invalid mail outbox payload")?;
    let field = |name: &str| payload.get(name).and_then(serde_json::Value::as_str);
    let to = field("to").ok_or("invalid mail outbox payload")?;
    let subject = field("subject").ok_or("invalid mail outbox payload")?;
    let text_body = field("text").ok_or("invalid mail outbox payload")?;
    let html_body = field("html");
    let attachment_storage = parse_attachments(&payload)?;
    let attachments = attachment_storage
        .iter()
        .map(|(filename, content_type, bytes)| MailAttachment {
            filename,
            content_type,
            bytes,
        })
        .collect::<Vec<_>>();

    validate_mail_network_policy(mail, network)?;
    let mail = mail.ok_or("SMTP configuration is incomplete")?;
    let host = mail.host.as_deref().unwrap_or_default();
    let security_mode = SmtpSecurity::parse(mail.security.as_deref().unwrap_or("implicit_tls"))
        .map_err(|_| "SMTP configuration is invalid")?;
    let port = match mail.port {
        Some(port) if port > 0 => port,
        Some(_) => return Err("SMTP configuration is invalid"),
        None => match security_mode {
            SmtpSecurity::ImplicitTls => 465,
            SmtpSecurity::StartTls => 587,
            SmtpSecurity::LocalPlaintext => 25,
        },
    };
    let from = mail
        .from
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or("SMTP configuration is incomplete")?;
    let client = SmtpClient::new(SmtpConfig {
        host,
        port,
        security: security_mode,
        username: mail.username.as_deref(),
        password: mail.password.as_deref(),
        ca_certificate_pem: mail.ca_certificate_pem.as_deref(),
        from,
    })
    .map_err(|_| "SMTP configuration is invalid")?;
    let message_id = format!("{event_id}@zelyra.local");
    client
        .send(MailMessage {
            to,
            subject,
            text_body,
            html_body,
            message_id: Some(&message_id),
            attachments: &attachments,
        })
        .map_err(|_| "SMTP delivery failed")
}

pub(crate) fn validate_mail_network_policy(
    mail: Option<&MailPolicy>,
    network: Option<&NetworkPolicy>,
) -> Result<(), &'static str> {
    let mail = mail.ok_or("SMTP configuration is incomplete")?;
    let host = mail
        .host
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or("SMTP configuration is incomplete")?;
    if !network.is_some_and(|policy| policy.allowed_hosts.iter().any(|allowed| allowed == host)) {
        return Err("SMTP host is not allowed by the project network policy");
    }
    Ok(())
}

fn parse_attachments(payload: &serde_json::Value) -> Result<Vec<DecodedAttachment>, &'static str> {
    let mut decoded = Vec::new();
    if let Some(attachments) = payload.get("attachments") {
        let attachments = attachments
            .as_array()
            .ok_or("invalid mail outbox payload")?;
        if attachments.len() > 1 {
            return Err("invalid mail outbox attachment");
        }
        for attachment in attachments {
            let filename = attachment
                .get("filename")
                .and_then(serde_json::Value::as_str)
                .ok_or("invalid mail outbox payload")?;
            let content_type = attachment
                .get("content_type")
                .and_then(serde_json::Value::as_str)
                .ok_or("invalid mail outbox payload")?;
            let content = attachment
                .get("content")
                .and_then(serde_json::Value::as_str)
                .ok_or("invalid mail outbox payload")?;
            let bytes = super::base64_decode_bytes(content)
                .map_err(|_| "invalid mail outbox attachment")?;
            decoded.push((filename.to_owned(), content_type.to_owned(), bytes));
        }
    }
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_bounded_pdf_attachment_payloads() {
        let payload = serde_json::json!({
            "attachments": [{
                "filename": "invoice.pdf",
                "content_type": "application/pdf",
                "content": super::super::base64_encode_bytes(b"%PDF-1.4\n%%EOF")
            }]
        });
        let decoded = parse_attachments(&payload).unwrap();
        assert_eq!(decoded[0].0, "invoice.pdf");
        assert_eq!(decoded[0].1, "application/pdf");
        assert_eq!(decoded[0].2, b"%PDF-1.4\n%%EOF");

        let invalid = serde_json::json!({
            "attachments": [{
                "filename": "invoice.pdf",
                "content_type": "application/pdf",
                "content": "$$$"
            }]
        });
        assert_eq!(
            parse_attachments(&invalid),
            Err("invalid mail outbox attachment")
        );
    }
}
