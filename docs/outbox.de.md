# Lokale MariaDB-Outbox (experimentell)

Die Outbox aus 0.8 speichert Anwendungsereignisse in derselben MariaDB-
Transaktion wie die Geschäftsänderung. Sie ist ein experimentelles
Datenbankfeature und kein vollständiger Queue-Dienst.

## Einrichten und inspizieren

Verwende ein MariaDB-Projekt mit konfigurierter Datenbank-URL und erstelle die
interne Tabelle ausdrücklich:

```sh
zelyra outbox setup main.zyl
```

Zeige bis zu 200 Zeilen mit Betreiber-Metadaten an:

```sh
zelyra outbox list main.zyl --limit 50 --format text
zelyra outbox list main.zyl --limit 50 --format json
```

Die Liste lässt Ereignisnutzlasten aus. Sie zeigt Ereignis-ID und -Typ,
Status, Versuchszahl, Zeitstempel und feste redigierte Fehlercodes. Das Limit
beträgt standardmäßig 50 und muss zwischen 1 und 200 liegen.

Ein erschöpftes Ereignis kann nach Prüfung durch den Betreiber ausdrücklich
wieder freigegeben werden:

```sh
zelyra outbox requeue <ereignis-id> main.zyl
```

Die Wiederfreigabe setzt den Versuchszähler zurück und macht das Ereignis
wieder verfügbar; sie stellt es nicht zu. Der Zugriff auf Nutzlasten bleibt in
der Anwendung, der die Daten gehören.

## Grenze der Handler-API

Die Rust-Database-API kann Handler nach Ereignistyp registrieren und pro
ausdrücklichem Aufruf höchstens ein Ereignis verarbeiten. Die CLI ordnet
öffentliche Zelyra-Funktionen ausdrücklich Ereignistypen zu:

```sh
zelyra outbox run main.zyl \
  --handler orders.created=handle_order_created \
  --handler invoices.issued=handle_invoice_issued \
  --lease-seconds 300
```

Jede zugeordnete Funktion muss die Signatur
`pub fn name(event_id: String, event_type: String, payload_json: String) -> Bool`
haben. `true` quittiert das Ereignis; `false` oder ein Runtime-Fehler plant
einen Retry. Die CLI prüft Projekt-Capability-Grants und Runtime-Policies.
Standardmäßig verarbeitet sie aktuell fällige, zugeordnete Ereignisse und
beendet sich ohne fälliges Ereignis. Mit `--once` wird höchstens ein Ereignis
verarbeitet. Die Lease dauert standardmäßig 300 Sekunden;
`--lease-seconds` akzeptiert 5 bis 3600. CLI-Handler können die Lease nicht
selbst verlängern; lange laufende Handler können nach Ablauf erneut zugestellt
werden. Zelyra-Quelltext besitzt noch keine Handler-Deklaration und die
Anwendungs-Runtime startet den Dispatcher nicht. Der Webserver startet keinen
Worker automatisch.

Die Zustellung erfolgt mindestens einmal. Ein Prozess kann nach einer
Nebenwirkung des Handlers abstürzen, bevor die Quittierung gespeichert ist.
Handler müssen deshalb die stabile Ereignis-ID für Idempotenz verwenden. Ein
Handler darf nur seine eigene noch gültige Lease verlängern; ein abgelaufener
oder veralteter Besitzer kann weder verlängern noch quittieren.
