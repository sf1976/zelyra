# Anwendungs-E-Mail (seit 0.9.5, PDF-Anhänge in 1.0.7)

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
Ausdrücke sein.

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

## Serverseitige PDF-Dateien (1.0.7)

`render_pdf("ViewName", datensatz)` liefert `Bytes`. Der View-Name muss als
Stringliteral auf einen im selben Programm deklarierten `view` zeigen; die
Daten sind ein Record oder eine `Map<String, T>`. `{feld}`-Platzhalter werden
mit HTML-Escaping eingesetzt. View-Collection-Blöcke wie `for line in lines {
... }` werden für Arrays von Records ausgewertet. Unbekannte Views und falsche
Datentypen scheitern beim Prüfen beziehungsweise bei der Ausführung.

Der lokale Renderer benötigt weder Browser noch CDN oder Systemprogramm. Er
erstellt ein paginiertes, durchsuchbares PDF aus dem sichtbaren Text der View.
HTML-Tags und CSS werden nicht als Layout-Engine interpretiert; Bilder,
Diagramme, Schriften, Farben und präzise Tabellenlayouts werden nicht in die
PDF-Geometrie übernommen. Für eine originalgetreue grafische Druckansicht
bleibt die browserbasierte Druckansicht der Anwendung zuständig. Die
serverseitige Variante ist für schlichte textbasierte Geschäftsdokumente
gedacht. Sie begrenzt HTML-Eingabe und expandierte View auf jeweils 2 MiB,
Collection-Blöcke auf 10.000 Datensatzzeilen und das Dokument auf 200 Seiten.
Zeichen außerhalb der unterstützten WinAnsi-Schrift werden als `?` ausgegeben.

Für einen Versand ohne HTML-Text nimmt `mail_queue` fünf Argumente an:
Empfänger, Betreff, Text, PDF-Dateiname und `Bytes`. Mit HTML-Alternative sind
es sechs Argumente; der HTML-Text steht an vierter Stelle. Je Nachricht ist
höchstens ein PDF mit maximal 2 MiB erlaubt. Dateinamen dürfen keine Pfade
enthalten. Die Outbox-Payload ist auf 3 MiB begrenzt. Der Versand bleibt eine
MariaDB-Transaktion und erfolgt über den vorhandenen, ausdrücklich gestarteten
Worker:

```zelyra
transaction {
    mail_queue(
        "kunde@example.test",
        "Rechnung " + invoice.number,
        "Ihre Rechnung ist beigefügt.",
        "rechnung-" + invoice.number + ".pdf",
        render_pdf("InvoicePrint", invoice)
    )
}
```

Das vollständige prüfbare Beispiel steht in
[`examples/pdf_mail_outbox/main.zyl`](../examples/pdf_mail_outbox/main.zyl). Diese
API fügt keinen untrusted HTML-Mailtext ein; der optionale HTML-Body muss
weiterhin vom Anwendungscode sicher erstellt werden.
