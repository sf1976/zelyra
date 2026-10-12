# Anwendungs-E-Mail (0.9.5)

Zelyras eigene E-Mail-API reiht Nachrichten über die transaktionale
MariaDB-Outbox ein. Das Einreihen geschieht in derselben Transaktion wie die
Geschäftsänderung. Ein separater, ausdrücklich gestarteter Outbox-Worker
übernimmt den SMTP-Versand; eine Webanfrage wartet daher nicht auf das Relay.

## Projekt konfigurieren

Erlaube E-Mail und Datenbankschreibzugriffe in `zelyra.toml` und trage den
genauen SMTP-Host in die Netzwerkfreigabe ein:

```toml
[capabilities]
email = true
database = true

[network]
allowed_hosts = ["smtp.example.test"]
```

Setze diese Werte in der lokalen `.env` oder Prozessumgebung:

```dotenv
ZELYRA_SMTP_HOST=smtp.example.test
ZELYRA_SMTP_SECURITY=starttls
ZELYRA_SMTP_PORT=587
ZELYRA_SMTP_FROM="Zelyra <noreply@example.test>"
ZELYRA_SMTP_USERNAME=app-user
ZELYRA_SMTP_PASSWORD=durch-ein-secret-ersetzen
# Optional bei einem Relay mit privater Zertifizierungsstelle:
ZELYRA_SMTP_CA_CERT=certs/smtp-ca.pem
```

Standardmäßig gilt `implicit_tls` auf Port 465. `starttls` verwendet Port
587. TLS-Zertifikate werden geprüft; ein fehlgeschlagener TLS-Handshake führt
niemals zu einer unverschlüsselten Rückstufung. `local_plaintext` ist nur für
`localhost`, `127.0.0.1` oder `::1` und ohne Authentifizierung erlaubt. Der
SMTP-Host muss zusätzlich in `[network].allowed_hosts` stehen. `.env` darf
nicht eingecheckt werden.
`ZELYRA_SMTP_CA_CERT` darf auf ein PEM-Wurzelzertifikat zeigen, relativ zum
Projektverzeichnis oder mit absolutem Pfad. Erlaubt ist eine reguläre Datei
ohne Symlink mit höchstens 128 KiB. Das Zertifikat ergänzt den normalen
Vertrauensspeicher; die Hostnamenprüfung bleibt aktiv.

## Einreihen und zustellen

`mail_queue` ist eine Anweisung, die nur direkt in einer Datenbank-
`transaction` vorkommen darf. Sie erwartet Empfänger, Betreff, Text und
optional einen HTML-Text. Header-Injection und zu große Nachrichten werden
vor dem Einfügen abgewiesen. Argumente müssen Werte oder nebenwirkungsfreie
String-Ausdrücke sein.

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
    mail_queue("kunde@example.test", "Willkommen", "Willkommen bei Zelyra", "<p>Willkommen bei Zelyra</p>")
  }
}
```

Richte die Outbox-Tabelle einmalig ein und stelle dann ausdrücklich eine
fällige Nachricht zu oder starte den Worker, bis kein zugeordnetes Ereignis
mehr fällig ist:

```sh
zelyra outbox setup main.zyl
zelyra outbox run main.zyl --handler zelyra.mail.send=deliver_mail --once
```

Das Einreihen und Zustellen von Anwendungs-E-Mails verwendet derzeit die
MariaDB-Outbox. Fehlgeschlagene Zustellungen werden wiederholt; die Zustellung
erfolgt mindestens einmal. Ein Worker kann ausfallen, nachdem das Relay die
Nachricht angenommen hat, aber bevor die Outbox die Quittierung speichert.
Dadurch sind Duplikate möglich. Zelyra verwendet die stabile Outbox-Ereignis-ID
als Nachrichten-ID. Das erleichtert die Diagnose, garantiert aber keine
genau-einmalige Zustellung. Die Outbox ist keine allgemeine Queue-Plattform.

Der lokale End-to-End-Test sendet nur an eine SMTP-Senke auf dem Loopback:

```sh
DATABASE_URL=mariadb://user:password@127.0.0.1:3306/app \
  bash tests/mail-outbox-e2e.sh
```

Die 0.9.5-API unterstützt Text- und HTML-Alternativen. Benannte Templates und
Dateianhänge gehören noch nicht zu dieser ersten API. Die Darstellung bleibt
im Anwendungscode; nicht vertrauenswürdiges HTML muss vor dem Versand
escaped werden.
