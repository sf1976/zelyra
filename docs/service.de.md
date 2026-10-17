# Zelyra als Linux-Dienst

`zelyra service install --port <port>` richtet das aktuelle Projekt als
systemd-Benutzerdienst ein und startet es. Der Befehl ist für Linux mit einem
aktiven systemd-Benutzermanager vorgesehen.

```sh
cd /pfad/zum/projekt
zelyra service install --port 3000
systemctl --user status zelyra-projekt-<kennung>.service
```

Die konkrete Unit-Bezeichnung gibt Zelyra bei erfolgreicher Installation aus.
Die Unit-Datei liegt in `$XDG_CONFIG_HOME/systemd/user/` oder, falls diese
Variable nicht gesetzt ist, in `~/.config/systemd/user/`. Sie startet die
Zelyra-Binärdatei, die den Installationsbefehl ausgeführt hat, mit
`main.zyl` aus dem Projektverzeichnis und `serve --fixed-port`. Der Schalter
unterbindet den normalen lokalen Port-Fallback auch dann, wenn der Port erst
zwischen Vorprüfung und Prozessstart belegt wird.

## Port und Netzwerk

Der gewünschte Port muss zwischen 1 und 65535 liegen und auf `127.0.0.1` frei
sein. Bei einem belegten Port bricht Zelyra ab und wählt keinen Ersatzport.
Der Dienst lauscht ausschließlich lokal. Für extern erreichbare Webseiten
konfiguriere Apache oder Nginx als Reverse Proxy zum Zelyra-Prozess. Öffne
den Zelyra-Port nicht direkt im Internet.

Der Port wird vor der Unit-Erstellung geprüft. Wird er in der kurzen Zeit bis
zum Start von einem anderen Prozess belegt, schlägt der systemd-Start fehl;
Zelyra entfernt dann die neu angelegte Unit wieder.
Bei einem normalen `zelyra serve`-Aufruf ohne `--fixed-port` bleibt der
bestehende lokale Port-Fallback erhalten.

## Start beim Systemstart

Ein systemd-Benutzerdienst startet standardmäßig, wenn sich der Benutzer
anmeldet. Für einen Serverneustart ohne interaktive Anmeldung muss der
Systemadministrator systemd-Lingering für das Konto aktivieren:

```sh
sudo loginctl enable-linger <benutzername>
```

Zelyra aktiviert Lingering nicht selbst, weil dies die systemweite
Kontokonfiguration ändert und Administratorrechte erfordern kann.

## Verwaltung und Entfernen

Zelyra zeigt nach der Installation den Namen der Unit und den Statusbefehl an.
Die CLI-Unterbefehle beziehen sich immer auf das Zelyra-Projekt im aktuellen
Arbeitsverzeichnis:

```sh
zelyra service status
zelyra service restart
zelyra service stop
zelyra service change --port 3001
zelyra service remove
```

`change` prüft den neuen Port, stoppt den Dienst, schreibt die verwaltete Unit
atomar neu und startet ihn wieder. Scheitert die Änderung, versucht Zelyra,
die vorherige Unit wiederherzustellen und den bisherigen Dienst neu zu starten.
Der Fehler nennt ausdrücklich, wenn auch dieser Rollback fehlschlägt.
`remove` stoppt und deaktiviert nur die zu diesem Projekt gehörende,
Zelyra-markierte Unit. `status`, `restart`, `stop` und `remove` benötigen keine
zusätzlichen Argumente. Für Diagnose und manuelle Verwaltung bleibt
`systemctl --user status <unit>` verfügbar.

## Grenzen und Sicherheit

- Die Installation überschreibt keine bestehende Unit.
- Bei belegtem Port wird niemals automatisch ein anderer Port gewählt.
- Zugangsdaten aus `.env` werden nicht in die Unit-Datei kopiert. Der Dienst
  erbt die Umgebung des systemd-Benutzermanagers; benötigte Werte müssen
  sicher über die vorhandene Projektkonfiguration bereitgestellt werden.
- Für Datenbankanwendungen müssen Datenbankdienst und erreichbare
  Verbindungsdaten vor dem Anwendungsstart vorhanden sein.
- Installiere Zelyra an einem stabilen Pfad. Die Unit verweist auf die beim
  Installieren ausgeführte Binärdatei.
- Apache und Nginx führen keine `.zyl`-Dateien aus. Sie leiten HTTP an den
  laufenden Zelyra-Webprozess weiter.

Der Dienstmanager unterstützt derzeit Linux systemd. Andere Betriebssysteme
und automatische Portwechsel sind nicht implementiert.
