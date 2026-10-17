# Run Zelyra as a Linux service

`zelyra service install --port <port>` installs and starts the current project
as a systemd user service. It requires Linux and an active systemd user
manager.

```sh
cd /path/to/project
zelyra service install --port 3000
systemctl --user status zelyra-project-<id>.service
```

Zelyra prints the exact unit name after a successful installation. The unit
is stored in `$XDG_CONFIG_HOME/systemd/user/`, or in
`~/.config/systemd/user/` when that variable is unset. It runs the Zelyra
executable that performed installation with `main.zyl` from the project
directory and `serve --fixed-port`. The flag disables the normal local port
fallback if the port becomes occupied between preflight and process startup.

## Port and network

The requested port must be from 1 through 65535 and free on `127.0.0.1`. If
it is occupied, Zelyra exits with an error and does not choose another port.
The service listens on loopback only. Configure Apache or Nginx as a reverse
proxy to the Zelyra process for externally reachable websites. Do not expose
the Zelyra port directly to the internet.

Zelyra checks the port before writing the unit. If another process takes it
before the service starts, systemd startup fails and Zelyra removes the newly
created unit. A regular `zelyra serve` invocation without `--fixed-port`
keeps the existing local fallback behavior.

## Start at boot

A systemd user service starts when its user logs in. To start the user manager
after a reboot without an interactive login, a system administrator must
enable systemd lingering for that account:

```sh
sudo loginctl enable-linger <username>
```

Zelyra does not enable lingering itself because that changes system-wide
account configuration and may require administrator privileges.

## Manage and remove

Zelyra prints the unit name and status command after installation. CLI
subcommands always target the Zelyra project in the current working directory:

```sh
zelyra service status
zelyra service restart
zelyra service stop
zelyra service change --port 3001
zelyra service remove
```

`change` checks the new port, stops the service, atomically rewrites its
managed unit, and starts it again. If the change fails, Zelyra attempts to
restore the previous unit and restart the old service. The error reports if
that rollback also fails. `remove` stops and disables only the Zelyra-marked
unit belonging to this project. `status`, `restart`, `stop`, and `remove` take
no extra arguments. For diagnostics and manual administration,
`systemctl --user status <unit>` remains available.

## Limits and security

- Installation never overwrites an existing unit.
- A busy port is an error; Zelyra never silently selects another port.
- Secrets from `.env` are not copied into the unit file. The service inherits
  the systemd user manager environment; provide required values securely via
  the existing project configuration.
- Database-backed applications require their database service and connection
  settings to be available before the web process starts.
- Install Zelyra at a stable path. The unit references the executable used
  during installation.
- Apache and Nginx do not execute `.zyl` files. They proxy HTTP to the running
  Zelyra web process.

Linux systemd is the only service manager supported so far. Other operating
systems and automatic port changes are not implemented yet.
