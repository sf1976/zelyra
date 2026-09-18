# Zelyra-Setup-Assistent

`zelyra setup` und `zelyra setup --web` verwenden dieselben Setup-Aktionen.

## Konsole

Aus einem MariaDB-Projektverzeichnis:

```bash
zelyra setup
zelyra setup --database
zelyra setup --schema
zelyra setup --all
```

`setup` legt eine geschützte `.env` an, wenn sie fehlt. `--database` startet
das erzeugte MariaDB-Compose-Projekt, `--schema` startet es und wendet das
Schema aus `main.zyl` an, und `--all` führt beide Aktionen aus. Vorhandene
`.env`-Dateien werden niemals überschrieben.

## Browser

```bash
zelyra setup --web
```

Der Server bindet standardmäßig nur an `127.0.0.1:3030` und gibt eine einmalige
URL mit zufälligem Setup-Token aus, zum Beispiel:

```text
Zelyra setup web is running on http://127.0.0.1:3030/
open: http://127.0.0.1:3030/?token=<local-token>
```

Der Browser bietet dieselben Aktionen wie die Konsole: Konfiguration
vorbereiten, MariaDB und Anwendung starten, Schema anwenden oder alles
ausführen.
Diese Aktionen sind durch einen HTTP-End-to-End-Test mit dem einmaligen Token
abgedeckt; das erzeugte MariaDB-Schema und eine CRUD-Seite werden ebenfalls
geprüft.

Einen anderen lokalen Port setzt man mit `zelyra setup --web --port 3031`.
Der Setup-Server ist absichtlich nur lokal erreichbar. Er darf nicht über
einen Reverse Proxy veröffentlicht oder an eine öffentliche Schnittstelle
gebunden werden. Nach dem Setup mit `Ctrl+C` beenden.

## Docker-Grenze

Wenn Docker Compose verfügbar ist, startet Zelyra den erzeugten MariaDB-
Container und die Anwendung mit `docker compose` oder dem Legacy-Befehl
`docker-compose`. MariaDB wird damit als Container eingerichtet, wenn sie
noch nicht läuft. Zelyra installiert Docker selbst nicht, verändert keine
Betriebssystempakete und fordert keine Root-Rechte an. Fehlt Docker, meldet
der Assistent dies; anschließend muss Docker Desktop oder Docker Engine mit
Compose installiert und der Vorgang wiederholt werden.

Zugangsdaten werden lokal erzeugt, niemals ausgegeben und nicht in
Statusmeldungen zurückgegeben. Destruktive Datenbankänderungen gehören nicht
zu diesem Erstinstallationsassistenten und benötigen weiterhin die
ausdrücklichen Datenbankbefehle.
