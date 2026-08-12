#!/usr/bin/env python3
from urllib.parse import urlparse
import socket, ssl, sys
kind, raw = sys.argv[1], sys.argv[2]
url = urlparse(raw)
port = url.port or (993 if kind == "imap" else 465)
with socket.create_connection((url.hostname, port), timeout=10) as sock:
    with ssl.create_default_context().wrap_socket(sock, server_hostname=url.hostname) as tls:
        banner = tls.recv(256)
        if not banner:
            raise SystemExit(1)
