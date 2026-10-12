#!/usr/bin/env python3
"""Tiny loopback SMTP sink for the password-reset integration test."""

import pathlib
import base64
import os
import ssl
import socketserver
import sys
import threading
import time


message_path = pathlib.Path(sys.argv[1])
port_path = pathlib.Path(sys.argv[2])
reply_delay = float(sys.argv[3]) if len(sys.argv) > 3 else 0.0
started_path = pathlib.Path(sys.argv[4]) if len(sys.argv) > 4 else None
tls_certificate = os.environ.get("ZELYRA_SMTP_TEST_TLS_CERT")
tls_key = os.environ.get("ZELYRA_SMTP_TEST_TLS_KEY")
auth_username = os.environ.get("ZELYRA_SMTP_TEST_USERNAME")
auth_password = os.environ.get("ZELYRA_SMTP_TEST_PASSWORD")
greeting_delay = float(os.environ.get("ZELYRA_SMTP_TEST_GREETING_DELAY", "0"))
capture_lock = threading.Lock()
captured_messages = 0


def capture_message(message):
    global captured_messages
    with capture_lock:
        path = (
            message_path
            if captured_messages == 0
            else message_path.with_name(f"{message_path.name}.{captured_messages}")
        )
        path.write_bytes(b"\n".join(message))
        captured_messages += 1


class Handler(socketserver.StreamRequestHandler):
    def reply(self, message):
        self.wfile.write(message)
        self.wfile.flush()

    def handle(self):
        if greeting_delay:
            time.sleep(greeting_delay)
        self.reply(b"220 localhost test SMTP\r\n")
        data_mode = False
        tls_active = False
        auth_step = 0
        message = []
        while line := self.rfile.readline():
            command = line.rstrip(b"\r\n")
            if data_mode:
                if command == b".":
                    if started_path is not None:
                        with capture_lock:
                            started_path.write_text(str(captured_messages), encoding="ascii")
                    if reply_delay:
                        time.sleep(reply_delay)
                    capture_message(message)
                    self.reply(b"250 queued\r\n")
                    data_mode = False
                    message = []
                else:
                    message.append(command[1:] if command.startswith(b"..") else command)
            elif command.upper().startswith((b"EHLO", b"HELO")):
                extensions = [b"250-localhost"]
                if tls_certificate and not tls_active:
                    extensions.append(b"250-STARTTLS")
                if auth_username and tls_active:
                    extensions.append(b"250-AUTH PLAIN LOGIN")
                extensions.append(b"250 8BITMIME")
                self.reply(b"\r\n".join(extensions) + b"\r\n")
            elif command.upper() == b"STARTTLS" and tls_certificate and not tls_active:
                self.reply(b"220 ready to start TLS\r\n")
                self.wfile.flush()
                self.wfile.close()
                self.rfile.close()
                context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
                context.load_cert_chain(tls_certificate, tls_key)
                try:
                    self.connection = context.wrap_socket(self.connection, server_side=True)
                except ssl.SSLError:
                    return
                self.rfile = self.connection.makefile("rb")
                self.wfile = self.connection.makefile("wb")
                tls_active = True
            elif command.upper().startswith(b"AUTH PLAIN"):
                if not tls_active or not auth_username:
                    self.reply(b"538 encryption required\r\n")
                else:
                    encoded = command.partition(b" ")[2].partition(b" ")[2]
                    if not encoded:
                        self.reply(b"334 \r\n")
                        encoded = self.rfile.readline().strip()
                    try:
                        _, username, password = base64.b64decode(encoded).split(b"\0", 2)
                    except (ValueError, base64.binascii.Error):
                        username, password = b"", b""
                    if username.decode(errors="replace") == auth_username and password.decode(errors="replace") == auth_password:
                        self.reply(b"235 authenticated\r\n")
                    else:
                        self.reply(b"535 authentication failed\r\n")
            elif command.upper() == b"AUTH LOGIN":
                if not tls_active or not auth_username:
                    self.reply(b"538 encryption required\r\n")
                else:
                    auth_step = 1
                    self.reply(b"334 VXNlcm5hbWU6\r\n")
            elif auth_step == 1:
                try:
                    username = base64.b64decode(command).decode()
                except (ValueError, base64.binascii.Error):
                    username = ""
                auth_step = 2
                self.reply(b"334 UGFzc3dvcmQ6\r\n")
            elif auth_step == 2:
                try:
                    password = base64.b64decode(command).decode()
                except (ValueError, base64.binascii.Error):
                    password = ""
                auth_step = 0
                if username == auth_username and password == auth_password:
                    self.reply(b"235 authenticated\r\n")
                else:
                    self.reply(b"535 authentication failed\r\n")
            elif command.upper().startswith((b"MAIL FROM:", b"RCPT TO:")):
                self.reply(b"250 ok\r\n")
            elif command.upper() == b"DATA":
                data_mode = True
                self.reply(b"354 end with dot\r\n")
            elif command.upper() == b"QUIT":
                self.reply(b"221 bye\r\n")
                return
            else:
                self.reply(b"250 ok\r\n")


class Server(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True


with Server(("127.0.0.1", 0), Handler) as server:
    port_path.write_text(str(server.server_address[1]), encoding="ascii")
    server.serve_forever()
