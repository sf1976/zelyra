#!/usr/bin/env python3
"""Exercise an imported Laravel unsubscribe route on disposable MariaDB."""

from __future__ import annotations

import html
import os
import re
import secrets
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path


def run(command: list[str], *, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, env=env, text=True, check=False)


def reserve_port() -> int:
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        return listener.getsockname()[1]


def request(url: str, *, method: str = "GET", body: bytes | None = None, origin: str | None = None):
    headers = {}
    if origin:
        headers["Origin"] = origin
    if body is not None:
        headers["Content-Type"] = "application/x-www-form-urlencoded"
    req = urllib.request.Request(url, data=body, headers=headers, method=method)
    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, request, fp, code, message, headers, new_url):
            return None

    try:
        with urllib.request.build_opener(NoRedirect).open(req, timeout=5) as response:
            return response.status, response.headers, response.read()
    except urllib.error.HTTPError as error:
        return error.code, error.headers, error.read()
    except urllib.error.URLError as error:
        return 0, {}, str(error).encode("utf-8")


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: python3 tests/laravel_import_unsubscribe_e2e.py <laravel-app-directory>", file=sys.stderr)
        return 2
    source = Path(sys.argv[1]).resolve(strict=True)
    if not source.is_dir():
        print("error: Laravel source path must be a directory", file=sys.stderr)
        return 2

    root = Path(__file__).resolve().parents[1]
    binary = Path(os.environ.get("ZELYRA_BIN", root / "target/debug/zelyra")).resolve()
    if not binary.is_file():
        print(f"error: Zelyra binary not found: {binary}", file=sys.stderr)
        return 2
    environment = {
        key: value
        for key, value in os.environ.items()
        if key != "DATABASE_URL" and not key.startswith("ZELYRA_DATABASE_")
    }
    image = environment.get("ZELYRA_REPL_MARIADB_IMAGE", "mariadb:11.8")
    password = f"zelyra_{secrets.token_hex(18)}"
    token = secrets.token_hex(32)
    database = "zelyra_import_e2e"
    container = f"zelyra-import-e2e-{os.getpid()}-{secrets.token_hex(4)}"

    with tempfile.TemporaryDirectory(prefix="zelyra-laravel-import-e2e-") as temporary:
        work = Path(temporary)
        scaffold = work / "generated"
        client_config = work / "mariadb-client.cnf"
        client_config.write_text(
            f"[client]\nuser=root\npassword={password}\ndatabase={database}\n",
            encoding="utf-8",
        )
        client_config.chmod(0o600)
        http_port = reserve_port()
        server: subprocess.Popen[str] | None = None
        container_started = False

        def mysql(*arguments: str) -> subprocess.CompletedProcess[str]:
            return subprocess.run(
                [
                    "docker",
                    "exec",
                    container,
                    "mariadb",
                    "--defaults-extra-file=/tmp/zelyra-client.cnf",
                    "--batch",
                    "--skip-column-names",
                    *arguments,
                ],
                capture_output=True,
                text=True,
                check=False,
            )

        try:
            generated = run(
                [
                    str(binary),
                    "import",
                    "laravel",
                    "generate",
                    str(source),
                    "--output",
                    str(scaffold),
                    "--all-supported",
                ],
                env=environment,
            )
            if generated.returncode:
                return generated.returncode
            workflow_path = scaffold / "src/imported_public_unsubscribe.zyl"
            if not workflow_path.is_file():
                raise RuntimeError(
                    "the globally unique tenant-scoped public unsubscribe workflow was not generated"
                )
            if shutil.which("docker") is None:
                print("error: Docker is required for this disposable MariaDB E2E", file=sys.stderr)
                return 2
            workflow = workflow_path.read_text(encoding="utf-8")
            if "public_token {" not in workflow or "hash_field: token_hash" not in workflow:
                raise RuntimeError("generated tenant-scoped workflow omitted its checked public token scope")
            if "UPDATE csv_mailer_suppressions" in workflow:
                raise RuntimeError("scoped token workflow emitted application-authored SQL")
            if 'import "src/imported_layouts.zyl" as imported_layouts' not in workflow:
                raise RuntimeError("generated unsubscribe module omitted its detected shared layout dependency")
            if "view: ImportedAppShell" not in workflow:
                raise RuntimeError("generated unsubscribe success page omitted the shared application shell")

            docker_run = subprocess.run(
                [
                    "docker",
                    "run",
                    "--detach",
                    "--name",
                    container,
                    "--env",
                    f"MARIADB_ROOT_PASSWORD={password}",
                    "--env",
                    f"MARIADB_DATABASE={database}",
                    "--publish",
                    "127.0.0.1::3306",
                    image,
                ],
                capture_output=True,
                text=True,
                check=False,
            )
            if docker_run.returncode:
                print(docker_run.stderr, file=sys.stderr, end="")
                return docker_run.returncode
            container_started = True
            port_result = subprocess.run(
                ["docker", "port", container, "3306/tcp"],
                capture_output=True,
                text=True,
                check=True,
            )
            host_port = int(port_result.stdout.strip().rsplit(":", 1)[1])
            subprocess.run(
                ["docker", "cp", str(client_config), f"{container}:/tmp/zelyra-client.cnf"],
                capture_output=True,
                text=True,
                check=True,
            )
            for _ in range(90):
                ready = mysql("--execute=SELECT 1")
                if ready.returncode == 0:
                    break
                time.sleep(1)
            else:
                print("error: disposable MariaDB did not become ready", file=sys.stderr)
                return 1

            database_url = f"mariadb://root:{password}@127.0.0.1:{host_port}/{database}"
            database_environment = {**environment, "DATABASE_URL": database_url}
            setup = run([str(binary), "db", "setup", str(scaffold / "main.zyl")], env=database_environment)
            if setup.returncode:
                return setup.returncode

            tenant = mysql(
                "--execute=INSERT INTO tenants (id, name) VALUES "
                "(7, 'Importer E2E'), (8, 'Other Tenant E2E')"
            )
            if tenant.returncode:
                print(tenant.stderr, file=sys.stderr, end="")
                return tenant.returncode
            seed = mysql(
                "--execute="
                "INSERT INTO csv_mailer_suppressions (tenant_id, email, token_hash, token) "
                f"VALUES (7, 'import-e2e@example.test', SHA2('{token}', 256), '{token}')"
            )
            if seed.returncode:
                print(seed.stderr, file=sys.stderr, end="")
                return seed.returncode
            other_token = secrets.token_hex(32)
            other_seed = mysql(
                "--execute="
                "INSERT INTO csv_mailer_suppressions (tenant_id, email, token_hash, token) "
                f"VALUES (8, 'other-tenant@example.test', SHA2('{other_token}', 256), '{other_token}')"
            )
            if other_seed.returncode:
                print(other_seed.stderr, file=sys.stderr, end="")
                return other_seed.returncode

            server = subprocess.Popen(
                [str(binary), "serve", str(scaffold / "main.zyl"), f"127.0.0.1:{http_port}"],
                cwd=scaffold,
                env=database_environment,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE,
                text=True,
            )
            base = f"http://127.0.0.1:{http_port}"
            route = f"{base}/csv-mailer/abmelden/{token}"
            for _ in range(100):
                status, headers, page = request(route)
                if status == 200:
                    break
                if server.poll() is not None:
                    details = server.stderr.read() if server.stderr else ""
                    print(details, file=sys.stderr, end="")
                    print("error: imported Zelyra server exited before serving the route", file=sys.stderr)
                    return 1
                time.sleep(0.1)
            else:
                print("error: imported Zelyra route did not become ready", file=sys.stderr)
                return 1

            if headers.get("Cache-Control", "").lower() != "no-store":
                raise RuntimeError("verified token page did not send Cache-Control: no-store")
            if "E-Mail-Empfang abbestellen" not in page.decode("utf-8"):
                raise RuntimeError("generated confirmation page did not contain the imported heading")
            invalid = f"{base}/csv-mailer/abmelden/{'0' * 64}"
            if request(invalid)[0] != 404:
                raise RuntimeError("invalid GET token was not rejected with HTTP 404")
            if request(invalid)[1].get("Cache-Control", "").lower() != "no-store":
                raise RuntimeError("invalid token response did not send Cache-Control: no-store")

            csrf_match = re.search(rb'name="_zelyra_csrf" value="([^"]+)"', page)
            if not csrf_match:
                raise RuntimeError("generated confirmation page omitted the CSRF token")
            csrf = html.unescape(csrf_match.group(1).decode("ascii"))
            origin = base
            fields = urllib.parse.urlencode({"_zelyra_csrf": csrf}).encode("ascii")
            invalid_status = request(invalid, method="POST", body=fields, origin=origin)[0]
            if invalid_status != 404:
                raise RuntimeError(f"invalid POST token returned HTTP {invalid_status}, expected 404")
            bad_csrf_status, bad_csrf_headers, _ = request(
                route,
                method="POST",
                body=urllib.parse.urlencode({"_zelyra_csrf": "invalid"}).encode("ascii"),
                origin=origin,
            )
            if bad_csrf_status != 403:
                raise RuntimeError(f"invalid CSRF token returned HTTP {bad_csrf_status}, expected 403")
            if bad_csrf_headers.get("Cache-Control", "").lower() != "no-store":
                raise RuntimeError("CSRF failure did not send Cache-Control: no-store")
            before = mysql(
                "--execute=SELECT unsubscribed_at IS NULL FROM csv_mailer_suppressions "
                f"WHERE token_hash = SHA2('{token}', 256)"
            ).stdout.strip()
            if before != "1":
                raise RuntimeError("invalid POST changed the suppression row")
            other_before = mysql(
                "--execute=SELECT unsubscribed_at IS NULL FROM csv_mailer_suppressions "
                f"WHERE token_hash = SHA2('{other_token}', 256)"
            ).stdout.strip()
            if other_before != "1":
                raise RuntimeError("precondition failed: other-tenant row was already unsubscribed")

            status = request(route, method="POST", body=fields, origin=origin)[0]
            if status not in (302, 303):
                raise RuntimeError(f"valid POST returned HTTP {status}, expected redirect")
            first_time = mysql(
                "--execute=SELECT unsubscribed_at FROM csv_mailer_suppressions "
                f"WHERE token_hash = SHA2('{token}', 256)"
            ).stdout.strip()
            if not first_time:
                raise RuntimeError("valid POST did not persist the unsubscribe timestamp")
            other_after = mysql(
                "--execute=SELECT unsubscribed_at IS NULL FROM csv_mailer_suppressions "
                f"WHERE token_hash = SHA2('{other_token}', 256)"
            ).stdout.strip()
            if other_after != "1":
                raise RuntimeError("public token POST changed a row belonging to another tenant")

            second_page = request(route)
            second_csrf_match = re.search(
                rb'name="_zelyra_csrf" value="([^"]+)"', second_page[2]
            )
            if second_page[0] != 200 or not second_csrf_match:
                raise RuntimeError("repeat GET did not render the confirmation form")
            second_csrf = html.unescape(second_csrf_match.group(1).decode("ascii"))
            second_fields = urllib.parse.urlencode({"_zelyra_csrf": second_csrf}).encode("ascii")
            second_status = request(route, method="POST", body=second_fields, origin=origin)[0]
            second_time = mysql(
                "--execute=SELECT unsubscribed_at FROM csv_mailer_suppressions "
                f"WHERE token_hash = SHA2('{token}', 256)"
            ).stdout.strip()
            if second_status not in (302, 303) or second_time != first_time:
                raise RuntimeError("repeat POST changed the first unsubscribe timestamp")

            success = request(f"{base}/csv-mailer/abmelden")
            if success[0] != 200 or "Abmeldung gespeichert" not in success[2].decode("utf-8"):
                raise RuntimeError("generated success route did not render")
            if "zelyra-generated-main" not in success[2].decode("utf-8"):
                raise RuntimeError("generated success page did not render inside the imported application shell")
            print("Laravel-imported unsubscribe MariaDB/HTTP E2E passed.")
            print("The source project was not modified; no mail was sent.")
            return 0
        finally:
            if server is not None:
                server.terminate()
                try:
                    server.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    server.kill()
                    server.wait(timeout=5)
            if container_started:
                subprocess.run(
                    ["docker", "rm", "--force", container],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    check=False,
                )


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.CalledProcessError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
