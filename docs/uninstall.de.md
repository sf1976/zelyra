# Zelyra deinstallieren

`zelyra uninstall` zeigt die aktive Programmdatei und weitere ausführbare
Zelyra-Kopien im `PATH`. Es entfernt nach interaktiver Bestätigung nur die
aktuell ausgeführte Binärdatei. Projekte, Einstellungen, Logs, Datenbanken und
weitere Zelyra-Kopien bleiben erhalten.

~~~sh
zelyra uninstall --dry-run
zelyra uninstall
~~~

`--dry-run` zeigt das ausgewählte Ziel, ohne etwas zu ändern. Ohne
`--dry-run` verlangt der Befehl eine Bestätigung mit `yes` oder `ja`.
`--yes` beziehungsweise `-y` bestätigt die Entfernung ausdrücklich und ist
für nicht-interaktive Shells vorgesehen. Ohne TTY und ohne diese Option wird
nichts entfernt.

Installationen aus `apt`/`dpkg`, RPM, pacman, Homebrew, Microsoft Store,
Chocolatey oder Scoop müssen über den jeweiligen Paketmanager entfernt werden.
Zelyra verweigert in diesen Fällen die direkte Löschung, damit dessen
Paketdatenbank nicht veraltet. Der genaue Befehl hängt vom installierten Paket
und der Distribution ab.
