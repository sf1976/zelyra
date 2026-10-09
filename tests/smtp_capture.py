#!/usr/bin/env python3
"""Tiny loopback SMTP sink for the password-reset integration test."""

import pathlib
import socketserver
import sys
import threading
import time


message_path = pathlib.Path(sys.argv[1])
port_path = pathlib.Path(sys.argv[2])
reply_delay = float(sys.argv[3]) if len(sys.argv) > 3 else 0.0
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
        self.reply(b"220 localhost test SMTP\r\n")
        data_mode = False
        message = []
        while line := self.rfile.readline():
            command = line.rstrip(b"\r\n")
            if data_mode:
                if command == b".":
                    capture_message(message)
                    if reply_delay:
                        time.sleep(reply_delay)
                    self.reply(b"250 queued\r\n")
                    data_mode = False
                    message = []
                else:
                    message.append(command[1:] if command.startswith(b"..") else command)
            elif command.upper().startswith((b"EHLO", b"HELO")):
                self.reply(b"250-localhost\r\n250 8BITMIME\r\n")
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
