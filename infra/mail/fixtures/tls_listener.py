#!/usr/bin/env python3
"""EP-026 M4 TLS truthfulness listener (CONTROLLED_TEST_FIXTURE).

A REAL TLS peer (Python ssl) with a self-signed certificate. It
completes the TLS handshake, then speaks a minimal protocol greeting:

  IMAP mode: "* OK nexus-mail-fixture ready" then replies NO to LOGIN.
  SMTP mode: "220 nexus-mail-fixture ESMTP" then replies 250 to EHLO
             and 535 to AUTH.

The production clients are exercised for REAL TLS trust behavior:
default trust store must FAIL CLOSED on the self-signed cert; a
trust store containing the fixture cert must succeed through the
handshake (and the subsequent protocol failure, if any, is an
authentication outcome, never a TLS outcome).

Usage:
  tls_listener.py <listen_port> <cert_pem> <key_pem> imap|smtp
"""
import socket
import ssl
import sys


def main():
    listen_port = int(sys.argv[1])
    cert = sys.argv[2]
    key = sys.argv[3]
    mode = sys.argv[4]

    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(cert, key)

    listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    listener.bind(("127.0.0.1", listen_port))
    listener.listen(1)
    print(f"listening {listen_port} {mode}", flush=True)

    raw, _ = listener.accept()
    try:
        conn = context.wrap_socket(raw, server_side=True)
    except ssl.SSLError:
        print("tls handshake rejected", flush=True)
        listener.close()
        return
    print("tls handshake completed", flush=True)
    conn.settimeout(5)

    if mode == "imap":
        conn.sendall(b"* OK nexus-mail-fixture ready\r\n")
        while True:
            try:
                data = conn.recv(65536)
            except OSError:
                break
            if not data:
                break
            line = data.decode("utf-8", "replace").strip().upper()
            if line.startswith("A") and "LOGIN" in line:
                conn.sendall(b"a1 NO [AUTHENTICATIONFAILED] credentials invalid\r\n")
    else:
        conn.sendall(b"220 nexus-mail-fixture ESMTP\r\n")
        while True:
            try:
                data = conn.recv(65536)
            except OSError:
                break
            if not data:
                break
            line = data.decode("utf-8", "replace").strip().upper()
            if line.startswith("EHLO"):
                conn.sendall(b"250-nexus-mail-fixture\r\n250 AUTH PLAIN LOGIN\r\n")
            elif line.startswith("AUTH"):
                conn.sendall(b"535 5.7.8 Authentication credentials invalid\r\n")

    conn.close()
    listener.close()
    print("closed", flush=True)


if __name__ == "__main__":
    main()
