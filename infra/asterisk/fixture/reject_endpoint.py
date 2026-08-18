#!/usr/bin/env python3
"""EP-025 M4 controlled SIP responder fixture (endpoint-r / endpoint-s).

Classification: CONTROLLED_TEST_FIXTURE (never part of the production
call path). Asterisk 22.10.1 and the production Nexus consumer remain
real; only this peer is a fixture.

Implements just enough REAL SIP (RFC 3261) to make Asterisk PJSIP
treat it as a real peer:

  - REGISTER with RFC 2617 digest auth (real 401 challenge from
    Asterisk, real MD5 response). Asterisk records one contact on the
    AOR (max_contacts=1, remove_existing=yes). Re-registers every 12 s
    (Expires: 30).

  - INVITE handling by --mode:
      603    -> SIP/2.0 603 Decline          (real REJECTED proof)
      486    -> SIP/2.0 486 Busy Here        (real BUSY proof)
      ring   -> 180 Ringing, never answers   (real NO_ANSWER ringing)
      silent -> 200 OK with a=recvonly SDP, sends NO RTP
                (one-way media: receives from the bridge, sends back
                nothing)
      sender -> 200 OK, sends PCMU silence RTP for --send-seconds
                then goes silent (mid-call media loss)
      probe  -> perform ONE registration attempt with the GIVEN
                password, print PROBE_RESULT <code>, exit (wrong-cred
                denial proof)

  - OPTIONS -> 200 OK (PJSIP qualify); ACK/BYE/CANCEL tolerated.

Wire sentinels (consumed by scripts/ep025-m4-tests.sh):
  REGISTERED            first successful registration
  REGISTER_REFRESH      refresh registration
  INVITE <method>       each received INVITE
  RESPONSE <code>       each SIP response the responder emits
  MEDIA_START / MEDIA_STOP  sender mode RTP window sentinels

Digest flow (RFC 2617 with qop=auth, as Asterisk issues):
  1. REGISTER (no auth)                 -> 401 WWW-Authenticate: Digest
  2. REGISTER + Authorization           -> 200 OK
"""

import argparse
import hashlib
import secrets
import socket
import sys
import threading
import time

ASTERISK_HOST = "127.0.0.1"
ASTERISK_PORT = 5060
REGISTER_INTERVAL = 12  # Expires: 30, refresh well before expiry


def md5hex(data: bytes) -> str:
    return hashlib.md5(data).hexdigest()


def parse_message(datagram: bytes):
    """Split a SIP message into (start_line, headers dict, body)."""
    text = datagram.decode("utf-8", errors="replace")
    head, sep, body = text.partition("\r\n\r\n")
    if not sep:
        head, _sep2, body = text.partition("\n\n")
    lines = head.split("\r\n") if "\r\n" in head else head.split("\n")
    headers = {}
    for line in lines[1:]:
        if ":" in line:
            key, _, value = line.partition(":")
            headers[key.strip().lower()] = value.strip()
    return lines[0] if lines else "", headers, body


def parse_auth_params(header_value: str) -> dict:
    """Parse WWW-Authenticate: Digest ... into a dict."""
    params = {}
    # header_value looks like: Digest realm="asterisk", nonce="...", ...
    _, _, rest = header_value.partition("Digest")
    parts = rest.split(",")
    for part in parts:
        key, _, value = part.strip().partition("=")
        params[key.strip().lower()] = value.strip().strip('"')
    return params


def digest_response(user: str, realm: str, password: str, nonce: str,
                    method: str, uri: str, qop: str, nc: str, cnonce: str) -> str:
    ha1 = md5hex(f"{user}:{realm}:{password}".encode())
    ha2 = md5hex(f"{method}:{uri}".encode())
    if qop:
        return md5hex(f"{ha1}:{nonce}:{nc}:{cnonce}:{qop}:{ha2}".encode())
    return md5hex(f"{ha1}:{nonce}:{ha2}".encode())


class Responder:
    def __init__(self, name: str, password: str, sip_port: int,
                 rtp_port: int, mode: str, send_seconds: int):
        self.name = name
        self.password = password
        self.sip_port = sip_port
        self.rtp_port = rtp_port
        self.mode = mode
        self.send_seconds = send_seconds
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self.sock.bind(("0.0.0.0", sip_port))
        self.sock.settimeout(0.5)
        self.rtp_sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self.rtp_sock.bind(("0.0.0.0", rtp_port))
        self.rtp_sock.settimeout(0.2)
        # Retransmission cache: (call_id, cseq) -> response bytes.
        self.seen = {}
        # Current auth nonce from the last 401 challenge.
        self.nonce = None
        self.realm = "asterisk"
        self.registered = False
        self.local_ip = self._detect_local_ip()
        self.contact = f"<sip:{self.name}@{self.local_ip}:{self.sip_port}>"
        self.auth_uri = f"sip:{ASTERISK_HOST}"
        # RTP destination learned from Asterisk's SDP offer (the
        # container-side address/port the bridge listens on).
        self.rtp_target = (ASTERISK_HOST, self.rtp_port)

    def _detect_local_ip(self) -> str:
        """Local address Asterisk can reach (docker bridge gateway).

        The responder must advertise an RTP/Contact address that the
        Asterisk CONTAINER can route to. The docker0 bridge address
        (172.17.0.1) is authoritative here; a loopback connect would
        yield 127.0.0.1, which is unreachable from inside the
        container.
        """
        try:
            import fcntl
            import struct

            sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
            packed = fcntl.ioctl(
                sock.fileno(), 0x8915, struct.pack("256s", b"docker0")
            )
            addr = socket.inet_ntoa(packed[20:24])
            if addr != "0.0.0.0":
                return addr
        except OSError:
            pass
        # Fallback: connect to a docker-bridge subnet address; the
        # kernel picks the docker0 source address for that route.
        try:
            probe = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
            probe.connect(("172.17.0.2", 9))
            addr = probe.getsockname()[0]
            probe.close()
            return addr
        except OSError:
            return "127.0.0.1"

    # ---- registration -------------------------------------------------
    def build_register(self, auth: bool, expires: int = 30) -> bytes:
        cseq = "1" if not auth else "2"
        msg = (
            f"REGISTER sip:{ASTERISK_HOST} SIP/2.0\r\n"
            f"Via: SIP/2.0/UDP {self.local_ip}:{self.sip_port};branch=z9hG4bK-{secrets.token_hex(8)}\r\n"
            f"Max-Forwards: 70\r\n"
            f"From: <sip:{self.name}@{ASTERISK_HOST}>;tag={secrets.token_hex(8)}\r\n"
            f"To: <sip:{self.name}@{ASTERISK_HOST}>\r\n"
            f"Call-ID: {secrets.token_hex(12)}@{self.local_ip}\r\n"
            f"CSeq: {cseq} REGISTER\r\n"
            f"Contact: {self.contact};expires={expires}\r\n"
            f"Expires: {expires}\r\n"
            f"User-Agent: nexus-ep025-responder/1.0\r\n"
            f"Content-Length: 0\r\n"
        )
        if auth and self.nonce:
            nc = "00000001"
            cnonce = secrets.token_hex(8)
            qop = "auth"
            resp = digest_response(
                self.name, self.realm, self.password, self.nonce,
                "REGISTER", self.auth_uri, qop, nc, cnonce,
            )
            msg += (
                f"Authorization: Digest username=\"{self.name}\", "
                f"realm=\"{self.realm}\", nonce=\"{self.nonce}\", "
                f"uri=\"{self.auth_uri}\", response=\"{resp}\", "
                f"algorithm=MD5, cnonce=\"{cnonce}\", "
                f"nc={nc}, qop={qop}\r\n"
            )
        msg += "\r\n"
        return msg.encode()

    def register_once(self, with_auth: bool) -> None:
        self.sock.sendto(self.build_register(with_auth), (ASTERISK_HOST, ASTERISK_PORT))

    # ---- INVITE handling ---------------------------------------------
    def build_response(self, request_line: str, headers: dict, code: str,
                       reason: str, body: str = "", to_tag: bool = True) -> bytes:
        from_hdr = headers.get("from", "")
        to_hdr = headers.get("to", "")
        if to_tag and ";tag=" not in to_hdr:
            to_hdr = f"{to_hdr};tag={secrets.token_hex(8)}"
        msg = (
            f"SIP/2.0 {code} {reason}\r\n"
            f"Via: {headers.get('via', '')}\r\n"
            f"From: {from_hdr}\r\n"
            f"To: {to_hdr}\r\n"
            f"Call-ID: {headers.get('call-id', '')}\r\n"
            f"CSeq: {headers.get('cseq', '')}\r\n"
            f"Contact: {self.contact}\r\n"
        )
        if body:
            msg += f"Content-Type: application/sdp\r\n"
            msg += f"Content-Length: {len(body)}\r\n\r\n"
            msg += body
        else:
            msg += "Content-Length: 0\r\n\r\n"
        return msg.encode()

    def sdp_answer(self, recvonly: bool) -> str:
        direction = "recvonly" if recvonly else "sendrecv"
        return (
            "v=0\r\n"
            f"o=- {secrets.randbits(31)} {secrets.randbits(31)} IN IP4 {self.local_ip}\r\n"
            "s=-\r\n"
            f"c=IN IP4 {self.local_ip}\r\n"
            "t=0 0\r\n"
            f"m=audio {self.rtp_port} RTP/AVP 0 8\r\n"
            "a=rtpmap:0 PCMU/8000\r\n"
            "a=rtpmap:8 PCMA/8000\r\n"
            f"a={direction}\r\n"
        )

    def handle_invite(self, request_line: str, headers: dict,
                      source: tuple) -> bytes | None:
        call_id = headers.get("call-id", "")
        cseq = headers.get("cseq", "")
        cache_key = (call_id, cseq)
        if cache_key in self.seen:
            return self.seen[cache_key]

        if self.mode == "hybrid":
            # Caller-driven rejection: the M4 live suite originates
            # with tokens ep025-reject-* (expects 603 -> cause 21) and
            # ep025-busy-* (expects 486 -> cause 17). The From header
            # carries the caller id, so one responder serves both
            # proofs on the same AOR.
            from_hdr = headers.get("from", "").lower()
            if "busy" in from_hdr:
                resp = self.build_response(request_line, headers, "486", "Busy Here")
            else:
                resp = self.build_response(request_line, headers, "603", "Decline")
        elif self.mode == "603":
            resp = self.build_response(request_line, headers, "603", "Decline")
        elif self.mode == "486":
            resp = self.build_response(request_line, headers, "486", "Busy Here")
        elif self.mode == "ring":
            resp = self.build_response(request_line, headers, "180", "Ringing")
        elif self.mode in ("silent", "sender"):
            resp = self.build_response(
                request_line, headers, "200", "OK",
                body=self.sdp_answer(recvonly=(self.mode == "silent")),
            )
            # Learn the real RTP destination from Asterisk's SDP offer
            # (the container-side address/port for this leg). The
            # sender loop transmits real PCMU there.
            self._learn_rtp_target(headers)
        else:
            resp = self.build_response(request_line, headers, "603", "Decline")

        self.seen[cache_key] = resp
        print(f"INVITE {self.mode} from {source[0]}:{source[1]} "
              f"call-id={call_id}", flush=True)
        print(f"RESPONSE {code_of(resp)}", flush=True)
        return resp

    def _learn_rtp_target(self, headers: dict) -> None:
        """Parse Asterisk's SDP offer: c=IN IP4 <ip> and m=audio <port>."""
        # The body is not in the parsed headers dict; re-read it from
        # the request we just processed is not possible here, so the
        # caller passes it via a stash set in handle_invite's caller.
        target = getattr(self, "_pending_body", None)
        if not target:
            return
        import re
        c_match = re.search(r"c=IN IP4 ([0-9.]+)", target)
        m_match = re.search(r"m=audio (\d+)", target)
        if c_match and m_match:
            self.rtp_target = (c_match.group(1), int(m_match.group(1)))
            print(f"RTP_TARGET {self.rtp_target[0]}:{self.rtp_target[1]}", flush=True)

    # ---- RTP sender ---------------------------------------------------
    def rtp_sender_loop(self) -> None:
        """Send PCMU silence (160 samples/20 ms) for send_seconds."""
        ssrc = secrets.randbits(32)
        seq = secrets.randbits(16)
        ts = secrets.randbits(32)
        payload = b"\x00" * 160
        deadline = time.monotonic() + self.send_seconds
        next_tick = time.monotonic()
        print("MEDIA_START", flush=True)
        while time.monotonic() < deadline:
            now = time.monotonic()
            if now < next_tick:
                time.sleep(min(0.005, next_tick - now))
                continue
            header = bytes([0x80, 0x00]) + seq.to_bytes(2, "big") + ts.to_bytes(4, "big") + ssrc.to_bytes(4, "big")
            self.rtp_sock.sendto(header + payload, self.rtp_target)
            seq = (seq + 1) & 0xFFFF
            ts = (ts + 160) & 0xFFFFFFFF
            next_tick += 0.020
        print("MEDIA_STOP", flush=True)

    # ---- main loop ----------------------------------------------------
    def run(self) -> None:
        if self.mode == "probe":
            self.register_once(False)
            sent_auth = False
            # Wait for the 401, then answer with auth exactly ONCE and
            # report the final response code. A second 401 (digest
            # rejected) is itself the answer for the wrong-password
            # proof; never loop retries here.
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline:
                try:
                    data, _ = self.sock.recvfrom(65535)
                except socket.timeout:
                    continue
                line, headers, _ = parse_message(data)
                if line.startswith("SIP/2.0 401"):
                    if not sent_auth:
                        auth = headers.get("www-authenticate", "")
                        params = parse_auth_params(auth)
                        self.nonce = params.get("nonce")
                        self.realm = params.get("realm", "asterisk")
                        sent_auth = True
                        self.register_once(True)
                    else:
                        print("PROBE_RESULT 401", flush=True)
                        return
                elif line.startswith("SIP/2.0"):
                    code = line.split(" ")[1]
                    print(f"PROBE_RESULT {code}", flush=True)
                    return
            print("PROBE_RESULT TIMEOUT", flush=True)
            return

        # Normal modes: register, then serve.
        self.register_once(False)
        next_reg = time.monotonic() + 3
        last_code = None
        while True:
            if time.monotonic() >= next_reg and self.nonce:
                self.register_once(True)
                next_reg = time.monotonic() + REGISTER_INTERVAL
            try:
                data, source = self.sock.recvfrom(65535)
            except socket.timeout:
                continue
            line, headers, body = parse_message(data)
            if not line:
                continue
            method = line.split(" ")[0]

            if method == "REGISTER":
                if line.startswith("SIP/2.0"):
                    continue  # response, not request
                code_line = line
                # A REGISTER request: reply 401 (challenge) or 200.
                if "authorization" not in headers:
                    self.register_once(False)  # handled above
                    continue
                resp = self.build_response(code_line, headers, "200", "OK")
                self.sock.sendto(resp, source)
                if not self.registered:
                    self.registered = True
                    print("REGISTERED", flush=True)
                else:
                    print("REGISTER_REFRESH", flush=True)
            elif method == "INVITE":
                self._pending_body = body
                resp = self.handle_invite(line, headers, source)
                if resp:
                    self.sock.sendto(resp, source)
                if self.mode == "sender":
                    # One bounded RTP window per INVITE (the M4 suite
                    # dials the sender in two separate tests).
                    threading.Thread(target=self.rtp_sender_loop, daemon=True).start()
            elif method == "OPTIONS":
                resp = self.build_response(line, headers, "200", "OK")
                self.sock.sendto(resp, source)
                print("RESPONSE 200", flush=True)
            elif method in ("ACK", "BYE", "CANCEL", "PRACK", "INFO"):
                if method == "BYE" or method == "CANCEL":
                    resp = self.build_response(line, headers, "200", "OK")
                    self.sock.sendto(resp, source)
                    print(f"RESPONSE 200 ({method})", flush=True)
                # ACK/PRACK/INFO: no response required.

            # Handle the 401 challenge for our own REGISTER.
            if line.startswith("SIP/2.0 401"):
                auth = headers.get("www-authenticate", "")
                params = parse_auth_params(auth)
                self.nonce = params.get("nonce")
                self.realm = params.get("realm", "asterisk")
                print(f"CHALLENGE realm={self.realm} nonce={str(self.nonce)[:16]}... "
                      f"raw={auth[:120]!r}", flush=True)
                self.register_once(True)
            elif line.startswith("SIP/2.0 200") and "REGISTER" in headers.get("cseq", ""):
                if not self.registered:
                    self.registered = True
                    print("REGISTERED", flush=True)
                else:
                    print("REGISTER_REFRESH", flush=True)


def code_of(resp: bytes) -> str:
    return resp.split(b" ")[1].decode()


def main() -> int:
    ap = argparse.ArgumentParser(description="EP-025 controlled SIP responder")
    ap.add_argument("--name", required=True)
    ap.add_argument("--password", required=True)
    ap.add_argument("--sip-port", type=int, required=True)
    ap.add_argument("--rtp-port", type=int, required=True)
    ap.add_argument("--mode", required=True,
                    choices=["603", "486", "hybrid", "ring", "silent", "sender", "probe"])
    ap.add_argument("--send-seconds", type=int, default=8)
    args = ap.parse_args()

    responder = Responder(args.name, args.password, args.sip_port,
                          args.rtp_port, args.mode, args.send_seconds)
    try:
        responder.run()
    except KeyboardInterrupt:
        pass
    return 0


if __name__ == "__main__":
    sys.exit(main())
