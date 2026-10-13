#!/usr/bin/env python3
"""Validate real HTTP date/time values and errors without a database."""
import hashlib
import http.client
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
BIN = Path(os.environ.get("ZELYRA_BIN", ROOT / "target/debug/zelyra")).resolve()


def request(port, payload):
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
    connection.request("POST", "/clock", json.dumps({"packet": payload}), {"Content-Type": "application/json"})
    response = connection.getresponse()
    body = response.read().decode("utf-8")
    status = response.status
    connection.close()
    return status, json.loads(body)


with tempfile.TemporaryDirectory(prefix="zelyra-temporal-api-") as directory:
    directory = Path(directory)
    entry = directory / "main.zyl"
    entry.write_text('''type CalendarDay = Date
        struct Packet {
            day: CalendarDay clock: Time stamp: Timestamp elapsed: Duration optional_day: Date?
        }
        fn echo_packet(packet: Packet) -> Packet {
            assert(packet.day >= date("0001-01-01"))
            assert(packet.clock <= time("23:59:59"))
            assert(packet.stamp >= json_decode<Timestamp>("-62135596800000"))
            return packet
        }
        api POST "/clock" { handler echo_packet input { packet: Packet } output Packet }
        fn main() {}
        ''', encoding="utf-8")
    check = subprocess.run([str(BIN), "check", str(entry)], capture_output=True, text=True, timeout=15)
    assert check.returncode == 0, check.stderr
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    with (directory / "server.log").open("w", encoding="utf-8") as log:
        server = subprocess.Popen([str(BIN), "serve", str(entry), f"127.0.0.1:{port}"], stdout=log, stderr=log)
        try:
            valid = {"day": "2000-02-29", "clock": "23:59:59", "stamp": -1, "elapsed": -123, "optional_day": None}
            deadline = time.monotonic() + 10
            while True:
                assert server.poll() is None, "HTTP server exited"
                try:
                    status, body = request(port, valid)
                    break
                except OSError:
                    assert time.monotonic() < deadline, "HTTP server readiness timeout"
                    time.sleep(.05)
            assert status == 200 and body == valid, (status, body)
            for stamp, expected in [("1969-12-31T23:59:59.999Z", -1), ("1970-01-01T00:00:00Z", 0),
                                    ("0001-01-01T00:00:00.000Z", -62135596800000),
                                    ("9999-12-31T23:59:59.999Z", 253402300799999)]:
                payload = dict(valid, stamp=stamp)
                status, body = request(port, payload)
                assert status == 200 and body["stamp"] == expected, (status, body)
            invalid = [("day", "1900-02-29"), ("day", "0000-01-01"), ("day", "2026-02-30"), ("day", True),
                       ("clock", "24:00:00"), ("clock", "23:59:60"), ("clock", "12:00"),
                       ("stamp", 253402300800000), ("stamp", "2026-01-01T00:00:00"),
                       ("stamp", "2026-01-01T00:00:00+02:00"), ("stamp", True),
                       ("elapsed", "1.5"), ("optional_day", "2026-13-01")]
            for field, value in invalid:
                status, body = request(port, dict(valid, **{field: value}))
                assert status == 400, (field, value, status, body)
            print(json.dumps({"compiler_sha256": hashlib.sha256(BIN.read_bytes()).hexdigest(),
                              "typed_temporal_http_round_trips": "passed", "invalid_inputs": len(invalid)}))
        finally:
            server.terminate()
            try:
                server.wait(timeout=10)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait(timeout=5)
                raise AssertionError("server failed to terminate")
