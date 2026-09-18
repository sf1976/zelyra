# Zelyra Setup Assistant

`zelyra setup` and `zelyra setup --web` use the same setup actions.

## Console

From a MariaDB project directory:

```bash
zelyra setup
zelyra setup --database
zelyra setup --schema
zelyra setup --all
```

`setup` creates a protected `.env` when it is missing. `--database` starts the
generated MariaDB Compose project, `--schema` starts it and applies the
`main.zyl` schema, and `--all` performs both operations. Existing `.env` files
are never overwritten.

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

Use another local port with `zelyra setup --web --port 3031`. The setup server
is intentionally local-only. Do not expose it through a reverse proxy or bind
it to a public interface. Stop it with `Ctrl+C` after setup.

## Docker boundary

If Docker Compose is available, Zelyra starts the generated MariaDB container
and application with `docker compose` or the legacy `docker-compose` command.
MariaDB is therefore installed as a container when it is not already running.
Zelyra does not install Docker itself, modify operating-system packages, or
request root privileges. If Docker is missing, the assistant reports that
fact and the user must install Docker Desktop or Docker Engine with Compose
before retrying.

Credentials are generated locally, never printed, and never returned in setup
status messages. Destructive database changes remain outside this first-run
installer and require the existing explicit database commands.
