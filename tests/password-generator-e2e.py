#!/usr/bin/env python3
"""Exercise the Zelyra-only password generator through its HTTP interface."""

import html
import http.cookiejar
import os
import re
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path


project = Path(sys.argv[1] if len(sys.argv) > 1 else "benchmarks/business-app-01/password-generator").resolve()
zelyra = os.environ.get("ZELYRA_BIN", "zelyra")

with socket.socket() as listener:
    listener.bind(("127.0.0.1", 0))
    port = listener.getsockname()[1]
base = f"http://127.0.0.1:{port}"
email = os.environ.get("ZELYRA_TEST_EMAIL")
password = os.environ.get("ZELYRA_TEST_PASSWORD")
if not email or not password:
    raise SystemExit("set ZELYRA_TEST_EMAIL and ZELYRA_TEST_PASSWORD for the protected app E2E")
environment = os.environ.copy()
environment["ZELYRA_LEVEL"] = "work"
environment["ZELYRA_LANGUAGE"] = "de"
environment["ZELYRA_ALLOWED_HOSTS"] = "127.0.0.1,localhost"
server = subprocess.Popen(
    [zelyra, "serve", "main.zyl", f"127.0.0.1:{port}"],
    cwd=project,
    env=environment,
    stdout=subprocess.DEVNULL,
    stderr=subprocess.DEVNULL,
)


jar = http.cookiejar.CookieJar()
opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))


def request_form_token(path):
    request = urllib.request.Request(base + path, headers={"Origin": base})
    with opener.open(request, timeout=2) as response:
        body = response.read().decode()
        assert response.status == 200
        token = re.search(r'name="_zelyra_csrf" value="([^"]+)"', body).group(1)
        return token


def sign_in():
    token = request_form_token("/login")
    data = urllib.parse.urlencode(
        {"_zelyra_csrf": token, "email": email, "password": password}
    ).encode()
    request = urllib.request.Request(
        base + "/login",
        data=data,
        headers={
            "Origin": base,
            "Content-Type": "application/x-www-form-urlencoded",
        },
    )
    with opener.open(request, timeout=5) as response:
        assert response.status == 200
    assert any(cookie.name == "zelyra_session" for cookie in jar)


def post(length, profile):
    token = request_form_token("/generator")
    data = urllib.parse.urlencode(
        {"_zelyra_csrf": token, "length": str(length), "character_set": profile}
    ).encode()
    request = urllib.request.Request(
        base + "/generator",
        data=data,
        headers={
            "Origin": base,
            "Content-Type": "application/x-www-form-urlencoded",
        },
    )
    try:
        response = opener.open(request, timeout=5)
    except urllib.error.HTTPError as error:
        return error.code, error.read().decode(), error.headers
    with response:
        return response.status, response.read().decode(), response.headers


profiles = {
    "Nur Kleinbuchstaben": lambda value: value.islower() and value.isalpha(),
    "Nur Ziffern": lambda value: value.isdigit(),
    "Buchstaben": lambda value: value.isalpha()
    and any(char.islower() for char in value)
    and any(char.isupper() for char in value),
    "Buchstaben und Ziffern": lambda value: value.isalnum()
    and any(char.islower() for char in value)
    and any(char.isupper() for char in value)
    and any(char.isdigit() for char in value),
    "Alle Zeichen": lambda value: all(
        char.isalnum() or char in "!@#$%^&*-_+=?" for char in value
    )
    and any(char.islower() for char in value)
    and any(char.isupper() for char in value)
    and any(char.isdigit() for char in value)
    and any(not char.isalnum() for char in value),
}

try:
    deadline = time.monotonic() + 20
    while True:
        if server.poll() is not None:
            raise RuntimeError(f"zelyra serve exited with status {server.returncode}")
        try:
            with urllib.request.urlopen(base + "/login", timeout=2) as response:
                assert response.status == 200
            break
        except (OSError, urllib.error.URLError):
            if time.monotonic() >= deadline:
                raise
            time.sleep(0.1)

    anonymous = urllib.request.build_opener()
    with anonymous.open(base + "/") as response:
        landing = response.read().decode()
        assert 'href="/login"' in landing
    try:
        anonymous.open(base + "/generator", timeout=2)
        raise AssertionError("anonymous generator request unexpectedly succeeded")
    except urllib.error.HTTPError as error:
        assert error.code == 401
    sign_in()

    with opener.open(base + "/admin/users", timeout=5) as response:
        admin_page = response.read().decode()
        assert response.status == 200 and "create_user" in admin_page
    try:
        anonymous.open(base + "/register", timeout=2)
        raise AssertionError("public registration route unexpectedly exists")
    except urllib.error.HTTPError as error:
        assert error.code == 404

    for profile, check in profiles.items():
        status, body, headers = post(24, profile)
        assert status == 200, (profile, status, body[:500])
        assert "Ergebnis" in body
        match = re.search(r"<output><code>(.*?)</code></output>", body)
        assert match, body[-1000:]
        password = html.unescape(match.group(1))
        assert len(password) == 24 and check(password), (profile, password)
        assert headers.get("Cache-Control") == "no-store, private"
        assert headers.get("Referrer-Policy") == "no-referrer"
        print(f"PASS profile: {profile}")

    for length, profile in ((7, "Alle Zeichen"), (65, "Alle Zeichen"), (24, "invalid")):
        status, body, _ = post(length, profile)
        assert status == 422, (length, profile, status, body[:500])
        assert "<output><code>" not in body
        print(f"PASS rejected invalid input: length={length}, profile={profile}")
finally:
    server.terminate()
    try:
        server.wait(timeout=5)
    except subprocess.TimeoutExpired:
        server.kill()
        server.wait()
