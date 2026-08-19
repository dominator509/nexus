#!/usr/bin/env python3
"""EP-026 M4 controlled TCP break proxy (CONTROLLED_TEST_FIXTURE).

Relays bytes between a client and a REAL backend server. When a
trigger byte-sequence is observed in the CLIENT->SERVER stream, the
proxy stops relaying after a short hold and closes both sides. This
injects a real network failure at a deterministic protocol phase:

  - SMTP ambiguous: trigger after the DATA terminator CRLF.CRLF, so
    the backend's final 250 never reaches the client.
  - SMTP mid-session: trigger after RCPT TO, before DATA.
  - IMAP mid-session: trigger after SELECT, during the fetch.

Usage:
  tcp_break_proxy.py <listen_port> <backend_host> <backend_port> <trigger_hex> [hold_secs]

Prints JSON evidence lines to stdout when triggered.
"""
import json
import socket
import sys
import threading
import time


def log(msg):
    """Emit evidence without dying on a closed stdout (EPIPE)."""
    try:
        print(msg, flush=True)
    except BrokenPipeError:
        pass


def main():
    listen_port = int(sys.argv[1])
    backend_host = sys.argv[2]
    backend_port = int(sys.argv[3])
    trigger = bytes.fromhex(sys.argv[4])
    hold = float(sys.argv[5]) if len(sys.argv) > 5 else 0.3

    listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    listener.bind(("127.0.0.1", listen_port))
    listener.listen(1)
    log(json.dumps({"event": "listening", "port": listen_port}))

    client, _ = listener.accept()
    backend = socket.create_connection((backend_host, backend_port), timeout=5)
    log(json.dumps({"event": "connected", "backend": f"{backend_host}:{backend_port}"}))

    triggered = threading.Event()
    stop = threading.Event()
    # Serializes relay with trigger publication: once the trigger chunk
    # is forwarded, the flag is set while holding the lock, so no
    # server response (e.g. SMTP's final 250) can race past it to the
    # client. The authoritative final response is withheld
    # deterministically.
    relay_lock = threading.Lock()

    def pump(src, dst, direction):
        buf = b""
        try:
            while not stop.is_set():
                data = src.recv(65536)
                if not data:
                    break
                with relay_lock:
                    if triggered.is_set():
                        # The failure has been injected: server bytes
                        # are withheld from the client from here on.
                        continue
                    if direction == "c2s":
                        buf += data
                        if trigger in buf:
                            log(json.dumps({"event": "trigger", "phase": trigger.hex()}))
                            # Forward the ENTIRE chunk containing the
                            # trigger FIRST: the provider must receive
                            # the protocol phase (e.g. the SMTP DATA
                            # terminator CRLF.CRLF) so the message is
                            # really accepted. The flag is set while
                            # still holding the lock, so the
                            # provider's authoritative final response
                            # can never reach the client afterwards.
                            dst.sendall(data)
                            triggered.set()
                            time.sleep(hold)
                            continue
                        if len(buf) > 1 << 20:
                            buf = buf[-len(trigger):]
                    dst.sendall(data)
        except OSError:
            pass
        finally:
            stop.set()

    t1 = threading.Thread(target=pump, args=(client, backend, "c2s"), daemon=True)
    t2 = threading.Thread(target=pump, args=(backend, client, "s2c"), daemon=True)
    t1.start()
    t2.start()
    t1.join()
    t2.join()
    try:
        client.shutdown(socket.SHUT_RDWR)
    except OSError:
        pass
    try:
        backend.shutdown(socket.SHUT_RDWR)
    except OSError:
        pass
    client.close()
    backend.close()
    listener.close()
    log(json.dumps({"event": "closed", "triggered": triggered.is_set()}))


if __name__ == "__main__":
    main()
