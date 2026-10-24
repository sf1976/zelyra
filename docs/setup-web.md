# Zelyra Setup Assistant

`zelyra setup` and `zelyra setup --web` use the same setup actions.

## Console

From a MariaDB project directory:

```bash
zelyra setup
zelyra setup --database
zelyra setup --schema
zelyra setup --all
zelyra setup --host-port 18080 --db-host-port 3308
```

`setup` creates a protected `.env` when it is missing. `--database` starts the
generated MariaDB Compose project, `--schema` starts it and applies the
`main.zyl` schema, and `--all` performs both operations. Existing `.env` files
are never overwritten.
A plain `zelyra init <directory>` creates a minimal project that runs without
a database or setup. For MariaDB, use `zelyra init <directory> --mariadb` or
`zelyra new <directory> --mariadb`. Absolute paths stay absolute:
`zelyra setup /test_a` looks for `/test_a`; use `zelyra setup ./test_a` for a
directory under the current directory. If `/test_a` is missing while
`./test_a` exists, the error suggests the relative path.
After starting the application, console setup also prints its local URL. It
uses the effective `ZELYRA_HOST_PORT`, including a port selected during
scaffolding, and does not include database credentials in the message.

When setup creates a missing `.env`, it automatically chooses free published
web and MariaDB ports if the configured defaults are occupied. The optional
`--host-port` and `--db-host-port` flags request exact ports and reject a
conflict. They deliberately refuse to alter an existing `.env`.

## Browser

```bash
zelyra setup --web
```

The server binds to `127.0.0.1:3030` by default and prints a one-time URL with
a random setup token, for example:

```text
Zelyra setup web is running on http://127.0.0.1:3030/
open: http://127.0.0.1:3030/?token=<local-token>
```

The browser offers the same operations as the console: prepare configuration,
start MariaDB and the application, apply the schema, or run everything.
These actions are covered by an end-to-end HTTP test with the one-time token;
the generated MariaDB schema and CRUD page are verified as well.

Use another local port with `zelyra setup --web --port 3031`. The setup server
is intentionally local-only. Do not expose it through a reverse proxy or bind
it to a public interface. Stop it with `Ctrl+C` after setup.
If the default port `3030` is occupied, the assistant selects the next free
local port; an explicitly requested occupied `--port` is rejected.

## Docker boundary

If Docker Compose is available, Zelyra starts the generated MariaDB container
and application with `docker compose` (v2 or newer), or a modern standalone
`docker-compose` binary (v2 or newer). Python Compose v1 is unsupported because
current Docker engines no longer provide its required `ContainerConfig` field.
MariaDB is therefore installed as a container when it is not already running.
If Compose v2 is missing, the assistant shows installation guidance. An
interactive `zelyra setup --database`, `--schema`, or `--all` asks explicitly
whether to start installation through a supported package manager. Linux
supports APT, DNF, YUM, Pacman, and Zypper; macOS can use Homebrew; on Windows,
`winget` offers Docker Desktop with Compose. Linux package installation may
request elevated privileges. No system packages are installed without
consent, in non-interactive calls, or by the browser setup assistant. Docker
Engine itself is not automatically installed on Linux. The console,
`zelyra doctor`, and the browser status page show the platform-specific
official Docker installation page and the verification command
`docker compose version`.

On Linux, install Docker Engine and the Compose plugin from the [official
Linux instructions](https://docs.docker.com/engine/install/). On Windows use
[Docker Desktop for Windows](https://docs.docker.com/desktop/setup/install/windows-install/);
on macOS use [Docker Desktop for Mac](https://docs.docker.com/desktop/setup/install/mac-install/).
After installation, run `docker compose version` and repeat `zelyra setup
--all` or the browser action.

If Docker is installed but access to its socket is denied, setup reports a
safe Linux group-membership remedy instead of showing raw Docker output. After
adding the current user to the `docker` group, either fully sign out of the
Linux session and sign in again, or run these commands in the current terminal:

```bash
newgrp docker
id -nG
docker ps
```

Retry setup when `docker` appears in the group list and `docker ps` succeeds;
opening another terminal window alone may not refresh the session's group list.
Membership in the Docker group grants highly privileged, effectively
root-equivalent access to the machine. Port collisions are also reported
without exposing credentials.

Credentials are generated locally, never printed, and never returned in setup
status messages. Destructive database changes remain outside this first-run
installer and require the existing explicit database commands.
