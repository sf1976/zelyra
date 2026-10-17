# Run Zelyra projects as a Linux service

`zelyra service` manages the project in the current directory as a systemd
user service. The implementation targets Linux with systemd; it does not
manage Windows, macOS, or system-wide root services.

```sh
cd /path/to/project
zelyra check main.zyl
zelyra service install --port 3000
zelyra service status
```

The port is fixed. If `127.0.0.1:<port>` is occupied, installation fails;
Zelyra does not select a fallback port. The service starts
`zelyra serve <absolute-path>/main.zyl 127.0.0.1:<port>` with the project
directory as its working directory. If `.env` exists in the project root, the
systemd unit references it with `EnvironmentFile`; credentials are not copied
into the unit. The local `.env` needs appropriate read permissions and must
not be committed to version control.

## Change the port and manage the service

```sh
zelyra service change --port 3001
zelyra service restart
zelyra service status
zelyra service remove
```

Changing the port stops the old service first, checks the target port, writes
the new unit atomically, and restarts it. If the new service does not become
reachable within five seconds, Zelyra restores the previous unit and attempts
to start the previous service again. The error also reports if restoration
itself is incomplete. If the target port is occupied, the old unit is left
unchanged and the old service is started again. The preflight port check
reduces conflicts but cannot eliminate a race with another process between
the check and bind; a failed start triggers rollback. Port changes also refuse
to overwrite a unit that has been manually edited since installation.

`remove` stops and disables only this project unit and removes its file. Project
files, `.env`, databases, and Docker volumes are left untouched. All commands
refer to the project directory from which they are run. The unit is stored in
the systemd user directory and receives a stable name derived from the
canonical project path.

## Start after a machine reboot

A systemd user service starts when the user's service manager is running. To
start it before the next interactive login, explicitly enable lingering for
the account:

```sh
loginctl enable-linger "$USER"
```

Zelyra does not enable lingering itself or perform a hidden privilege
escalation. Apache or Nginx can proxy to `127.0.0.1:<port>`; they do not
execute Zelyra source files.

New projects include `public/` as the fronting web server's document root. See
the [deployment guide](deployment.en.md) for examples and limits.
