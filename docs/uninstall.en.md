# Uninstall Zelyra

`zelyra uninstall` shows the active executable and other runnable Zelyra
copies on `PATH`. After interactive confirmation, it removes only the currently
running binary. Projects, settings, logs, databases, and other Zelyra copies
remain in place.

~~~sh
zelyra uninstall --dry-run
zelyra uninstall
~~~

`--dry-run` displays the selected target without changing files. Without
`--dry-run`, the command asks you to type `yes` or `ja`.
`--yes` or `-y` explicitly confirms removal and is intended for
non-interactive shells. Without a TTY and without this option, nothing is
removed.

Installations from `apt`/`dpkg`, RPM, pacman, Homebrew, Microsoft Store,
Chocolatey, or Scoop must be removed with that package manager. Zelyra refuses
to delete those files directly so the package database does not become stale.
The exact command depends on the package name and distribution.
