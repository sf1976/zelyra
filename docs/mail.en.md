# Application email (0.9.5)

Zelyra's first-party email API queues messages through the transactional
MariaDB outbox. Enqueueing happens in the same transaction as the business
write. A separate, explicit outbox worker performs SMTP delivery, so a web
request never waits for the relay.

## Configure a project

Grant email and database writes in `zelyra.toml`, and allow the exact SMTP
hostname in the network policy:

```toml
[capabilities]
email = true
database = true

[network]
allowed_hosts = ["smtp.example.test"]
```

Set these values in the local `.env` or the process environment:

```dotenv
ZELYRA_SMTP_HOST=smtp.example.test
ZELYRA_SMTP_SECURITY=starttls
ZELYRA_SMTP_PORT=587
ZELYRA_SMTP_FROM="Zelyra <noreply@example.test>"
ZELYRA_SMTP_USERNAME=app-user
ZELYRA_SMTP_PASSWORD=replace-with-a-secret
# Optional when the relay uses a private certificate authority:
ZELYRA_SMTP_CA_CERT=certs/smtp-ca.pem
```

The default security mode is `implicit_tls` on port 465. `starttls` uses port
587. TLS certificate checks are enabled and a failed TLS handshake never falls
back to plaintext. `local_plaintext` is accepted only for `localhost`,
`127.0.0.1`, or `::1` and cannot use authentication. The SMTP host must also
appear in `[network].allowed_hosts`. Keep `.env` out of version control.
`ZELYRA_SMTP_CA_CERT` may point to a PEM root certificate, relative to the
project directory or at an absolute path. It must be a regular non-symlink file
of at most 128 KiB. The certificate is added to the normal trust store; hostname
verification remains enabled.

## Queue and deliver

`mail_queue` is a statement that may appear only directly inside a database
`transaction`. It accepts recipient, subject, text body, and optional HTML
body. Header injection and oversized messages are rejected before insertion.
Its arguments must be values or side-effect-free string expressions.

```zelyra
pub fn deliver_mail(event_id: String, event_type: String, payload_json: String) -> Bool uses Email {
  if event_type == "zelyra.mail.send" {
    match mail_deliver(event_id, payload_json) {
      Ok(_) => { return true }
      Err(_) => { return false }
    }
  }
  return false
}

fn main() uses Email, Database(write) {
  transaction {
    mail_queue("customer@example.test", "Welcome", "Welcome to Zelyra", "<p>Welcome to Zelyra</p>")
  }
}
```

Prepare the outbox table once, then explicitly dispatch one due message or run
the worker until no mapped event is due:

```sh
zelyra outbox setup main.zyl
zelyra outbox run main.zyl --handler zelyra.mail.send=deliver_mail --once
```

Mail queuing and dispatch currently use the MariaDB outbox. The worker retries
failed deliveries and delivery is at least once. A worker can fail after the
relay accepted a message but before the outbox acknowledgement was stored, so
duplicates are possible. Zelyra reuses the stable outbox event ID as the
message ID; this helps diagnose duplicates but does not provide exactly-once
delivery. The application must not treat the outbox as a general purpose queue.

The local end-to-end test sends only to a loopback SMTP sink:

```sh
DATABASE_URL=mariadb://user:password@127.0.0.1:3306/app \
  bash tests/mail-outbox-e2e.sh
```

This 0.9.5 API supports text and HTML alternatives. Named templates and file
attachments are not part of this initial API; keep presentation in application
code and do not place untrusted HTML into a message body without escaping it.
