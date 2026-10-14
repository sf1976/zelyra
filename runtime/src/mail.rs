use crate::{MailPolicy, NetworkPolicy};
use zelyra_mail::{MailAttachment, MailMessage, SmtpClient, SmtpConfig, SmtpSecurity};

type DecodedAttachment = (String, String, Vec<u8>);

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

    let mail = mail.ok_or("SMTP configuration is incomplete")?;
    let host = mail
        .host
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or("SMTP configuration is incomplete")?;
    if !network.is_some_and(|policy| policy.allowed_hosts.iter().any(|allowed| allowed == host)) {
        return Err("SMTP host is not allowed by the project network policy");
    }
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
