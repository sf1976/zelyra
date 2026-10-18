# Zelyra-Projekte als Linux-Dienst

`zelyra service` verwaltet das Projekt im aktuellen Verzeichnis als
systemd-Benutzerdienst. Die Implementierung ist für Linux mit systemd gedacht;
Windows, macOS und systemweite Root-Dienste werden nicht verwaltet.

```sh
cd /pfad/zum/projekt
zelyra check main.zyl
zelyra service install --port 3000
zelyra service status
```

Der Port ist verbindlich. Ist `127.0.0.1:<port>` belegt, wird die Installation
abgewiesen; Zelyra wählt keinen Ersatzport. Der Dienst startet
`zelyra serve <absoluter-pfad>/main.zyl 127.0.0.1:<port>` mit dem Projektordner
als Arbeitsverzeichnis. Falls im Projektstamm eine `.env` liegt, verweist die
systemd-Unit auf diese Datei als `EnvironmentFile`; Zugangsdaten werden nicht
in die Unit kopiert. Die lokale `.env` muss passende Leserechte haben und darf
nicht in die Versionsverwaltung gelangen.

## Portwechsel und Verwaltung

```sh
zelyra service change --port 3001
zelyra service restart
zelyra service status
zelyra service remove
```

Beim Portwechsel stoppt Zelyra zuerst den alten Dienst, prüft den Zielport,
schreibt die neue Unit atomar und startet sie neu. Wird der neue Dienst nicht
innerhalb von fünf Sekunden erreichbar, stellt Zelyra die vorherige Unit wieder
her und versucht, den vorherigen Dienst erneut zu starten. Der Fehler meldet
auch, wenn diese Wiederherstellung selbst nicht vollständig gelingt. Ein
belegter Zielport lässt die alte Unit unverändert und startet den alten Dienst
wieder. Der Wechsel verweigert außerdem eine Unit, die seit der Installation
manuell verändert wurde, damit eigene Anpassungen nicht überschrieben werden.
Die Portprüfung reduziert Konflikte, kann aber ein Rennen mit einem
anderen Prozess zwischen Prüfung und Bindung nicht ausschließen; ein solcher
Startfehler löst den Rollback aus.

`remove` stoppt und deaktiviert nur diese Projekt-Unit und entfernt ihre
Datei. Projektdateien, `.env`, Datenbanken und Docker-Volumes bleiben
unverändert. Alle Befehle beziehen sich auf den Projektordner, aus dem sie
aufgerufen werden. Die Unit liegt im systemd-Benutzerverzeichnis und erhält
einen stabilen Namen aus dem aufgelösten Projektpfad.

## Start nach einem Rechnerneustart

Ein systemd-Benutzerdienst wird gestartet, wenn der Benutzer-Systemmanager
läuft. Damit er auch vor der nächsten interaktiven Anmeldung startet, muss
Lingering ausdrücklich für das Benutzerkonto aktiviert sein:

```sh
loginctl enable-linger "$USER"
```

Zelyra aktiviert Lingering nicht selbst und benötigt dafür keine versteckte
Privilegienerhöhung. Apache oder Nginx können auf `127.0.0.1:<port>` proxien;
sie führen keine Zelyra-Quelldateien aus.

Neue Projekte enthalten `public/` als Dokumentenverzeichnis für den
vorgeschalteten Webserver. Beispiele und Grenzen stehen in der
[Deployment-Anleitung](deployment.de.md).
