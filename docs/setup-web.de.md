# Zelyra-Setup-Assistent

`zelyra setup` und `zelyra setup --web` verwenden dieselben Setup-Aktionen.

## Konsole

Aus einem MariaDB-Projektverzeichnis:

```bash
zelyra setup
zelyra setup --database
zelyra setup --schema
zelyra setup --all
zelyra setup --host-port 18080 --db-host-port 3308
```

`setup` legt eine geschützte `.env` an, wenn sie fehlt. `--database` startet
das erzeugte MariaDB-Compose-Projekt, `--schema` startet es und wendet das
Schema aus `main.zyl` an, und `--all` führt beide Aktionen aus. Vorhandene
`.env`-Dateien werden niemals überschrieben.
Nach dem Start der Anwendung gibt das Konsolen-Setup auch ihre lokale Adresse
aus. Verwendet wird der wirksame `ZELYRA_HOST_PORT`, einschließlich eines beim
Scaffolding ausgewählten Ports; Zugangsdaten stehen nicht in der Meldung.

Legt Setup eine fehlende `.env` an, wählt es bei belegten Vorlagen-
Standardports automatisch freie veröffentlichte Web- und MariaDB-Ports. Die
optionalen Flags `--host-port` und `--db-host-port` verlangen genaue Ports und
lehnen einen Konflikt ab. Eine vorhandene `.env` wird durch diese Flags bewusst
nicht verändert.

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
Ist der Standardport `3030` belegt, wählt der Assistent den nächsten freien
lokalen Port; ein ausdrücklich angegebener belegter `--port` wird abgelehnt.

## Docker-Grenze

Wenn Docker Compose verfügbar ist, startet Zelyra den erzeugten MariaDB-
Container und die Anwendung mit `docker compose` oder dem Legacy-Befehl
`docker-compose`. MariaDB wird damit als Container eingerichtet, wenn sie
noch nicht läuft. Zelyra installiert Docker selbst nicht, verändert keine
Betriebssystempakete und fordert keine Root-Rechte an. Fehlt Docker, meldet
der Assistent dies; anschließend muss Docker Desktop oder Docker Engine mit
Compose installiert und der Vorgang wiederholt werden. Die Konsole,
`zelyra doctor` und die Statusseite im Browser zeigen jetzt die passende
offizielle Docker-Installationsseite für das erkannte Betriebssystem sowie den
Prüfbefehl `docker compose version`.

Unter Linux verwendet man die [offizielle Linux-Anleitung](https://docs.docker.com/engine/install/),
unter Windows [Docker Desktop für Windows](https://docs.docker.com/desktop/setup/install/windows-install/)
und unter macOS [Docker Desktop für Mac](https://docs.docker.com/desktop/setup/install/mac-install/).
Nach der Installation `docker compose version` prüfen und anschließend
`zelyra setup --all` oder die Browser-Aktion erneut ausführen.

Ist Docker installiert, aber der Zugriff auf seinen Socket verweigert, meldet
Setup einen sicheren Hinweis zur Linux-Gruppenmitgliedschaft statt der rohen
Docker-Ausgabe. Nach dem Hinzufügen des aktuellen Benutzers zur Gruppe `docker`
entweder vollständig von der Linux-Sitzung abmelden und wieder anmelden oder
folgende Befehle im aktuellen Terminal ausführen:

```bash
newgrp docker
id -nG
docker ps
```

Setup erst erneut starten, wenn `docker` in der Gruppenliste erscheint und
`docker ps` funktioniert; ein zusätzlich geöffnetes Terminalfenster
aktualisiert die Gruppenliste möglicherweise nicht. Die Mitgliedschaft in der
Docker-Gruppe gewährt weitreichende, praktisch root-äquivalente Rechnerrechte.
Auch Portkonflikte werden ohne Preisgabe von Zugangsdaten gemeldet.

Zugangsdaten werden lokal erzeugt, niemals ausgegeben und nicht in
Statusmeldungen zurückgegeben. Destruktive Datenbankänderungen gehören nicht
zu diesem Erstinstallationsassistenten und benötigen weiterhin die
ausdrücklichen Datenbankbefehle.
