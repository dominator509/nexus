#!/usr/bin/env python3
"""EP-026 M4 silent peer listener (CONTROLLED_TEST_FIXTURE).

Accepts ONE connection, consumes whatever the client sends, then
HOLDS the socket open without replying for `hold_secs` (default 6).
The client's bounded timeout must fire and classify Timeout.

Usage:
  silent_listener.py <listen_port> [hold_secs]
"""
import socket
import sys
import time


def log(msg):
    """Emit evidence without dying on a closed stdout (EPIPE)."""
    try:
        print(msg, flush=True)
    except BrokenPipeError:
        pass


def main():
    listen_port = int(sys.argv[1])
    hold = float(sys.argv[2]) if len(sys.argv) > 2 else 6.0

    listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    listener.bind(("127.0.0.1", listen_port))
    listener.listen(1)
    log(f"listening {listen_port}")

    conn, _ = listener.accept()
    log("accepted")
    conn.settimeout(hold + 2)
    try:
        while True:
            data = conn.recv(65536)
            if not data:
                break
    except OSError:
        pass
    # Hold the socket open well past the client timeout without
    # writing a single byte (silent peer).
    time.sleep(hold)
    conn.close()
    listener.close()
    log("closed")


if __name__ == "__main__":
    main()
