#!/usr/bin/env python3
"""Exercise the Zelyra-only password generator through its HTTP interface."""

import html
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
environment = os.environ.copy()
environment["ZELYRA_LEVEL"] = "work"
environment["ZELYRA_LANGUAGE"] = "de"
server = subprocess.Popen(
    [zelyra, "serve", "main.zyl", f"127.0.0.1:{port}"],
    cwd=project,
    env=environment,
    stdout=subprocess.DEVNULL,
    stderr=subprocess.DEVNULL,
)


def get_form():
    request = urllib.request.Request(base + "/generator", headers={"Origin": base})
    with urllib.request.urlopen(request, timeout=2) as response:
        body = response.read().decode()
        assert response.status == 200
        assert "Passwortgenerator" in body
        assert "Passwort erzeugen" in body
        token = re.search(r'name="_zelyra_csrf" value="([^"]+)"', body).group(1)
        return token


def post(length, profile):
    token = get_form()
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
        response = urllib.request.urlopen(request, timeout=5)
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
            get_form()
            break
        except (OSError, urllib.error.URLError):
            if time.monotonic() >= deadline:
                raise
            time.sleep(0.1)

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
