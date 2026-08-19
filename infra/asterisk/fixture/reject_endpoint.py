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
import os
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
            nc = self._next_nc()
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
                f"cnonce=\"{cnonce}\", qop={qop}, nc={nc}\r\n"
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

    # ---- outbound caller (LF-012 governed call) ---------------------
    def build_invite(self, extension: str, with_auth: bool = False) -> bytes:
        # The retry mirrors the proven REGISTER-retry pattern: a NEW
        # Via branch (new transaction), stable Call-ID + From tag, and
        # a bumped CSeq (REGISTER's build_register uses CSeq 2 on the
        # authenticated retry and PJSIP accepts it).
        call_id = getattr(self, "_invite_call_id", None) or f"{secrets.token_hex(12)}@{self.local_ip}"
        from_tag = getattr(self, "_invite_from_tag", None) or secrets.token_hex(8)
        branch = f"z9hG4bK-{secrets.token_hex(8)}"
        cseq = "2" if with_auth else "1"
        self._invite_call_id = call_id
        self._invite_from_tag = from_tag
        sdp = (
            "v=0\r\n"
            f"o=- {secrets.randbits(31)} {secrets.randbits(31)} IN IP4 {self.local_ip}\r\n"
            "s=-\r\n"
            f"c=IN IP4 {self.local_ip}\r\n"
            "t=0 0\r\n"
            f"m=audio {self.rtp_port} RTP/AVP 0 8\r\n"
            "a=rtpmap:0 PCMU/8000\r\n"
            "a=rtpmap:8 PCMA/8000\r\n"
            "a=sendrecv\r\n"
        )
        # CRITICAL: the Authorization header must be emitted BEFORE the
        # empty line that separates headers from the SDP body. PJSIP
        # only parses headers up to that separator; an Authorization
        # appended after the body is invisible to the authenticator
        # ("No Authorization header found" -> endless 401).
        auth_hdr = ""
        if with_auth and self.nonce:
            nc = self._next_nc()
            cnonce = secrets.token_hex(8)
            qop = "auth"
            uri = f"sip:{extension}@{ASTERISK_HOST}"
            resp = digest_response(
                self.name, self.realm, self.password, self.nonce,
                "INVITE", uri, qop, nc, cnonce,
            )
            # PJSIP challenges carry an opaque token; the reference
            # client (baresip) echoes it in the Authorization header.
            # Include it when the challenge provided one. Parameter
            # order mirrors baresip's proven header exactly.
            opaque = getattr(self, "_challenge_opaque", "")
            opaque_part = f", opaque=\"{opaque}\"" if opaque else ""
            auth_hdr = (
                f"Authorization: Digest username=\"{self.name}\", "
                f"realm=\"{self.realm}\", nonce=\"{self.nonce}\", "
                f"uri=\"{uri}\", response=\"{resp}\", "
                f"cnonce=\"{cnonce}\"{opaque_part}, qop={qop}, nc={nc}\r\n"
            )
        msg = (
            f"INVITE sip:{extension}@{ASTERISK_HOST} SIP/2.0\r\n"
            f"Via: SIP/2.0/UDP {self.local_ip}:{self.sip_port};branch={branch}\r\n"
            "Max-Forwards: 70\r\n"
            f"From: <sip:{self.name}@{ASTERISK_HOST}>;tag={from_tag}\r\n"
            f"To: <sip:{extension}@{ASTERISK_HOST}>\r\n"
            f"Call-ID: {call_id}\r\n"
            f"CSeq: {cseq} INVITE\r\n"
            f"Contact: {self.contact}\r\n"
            "User-Agent: nexus-ep025-lf012-caller/1.0\r\n"
            f"{auth_hdr}"
            "Content-Type: application/sdp\r\n"
            f"Content-Length: {len(sdp)}\r\n"
            "\r\n"
            f"{sdp}"
        )
        return msg.encode()

    def _next_nc(self) -> str:
        # PJSIP with qop=auth tracks the nonce count: every authorized
        # request with the SAME nonce must use a strictly increasing
        # nc, or the request is treated as a replay (401 loop). The
        # REGISTER retry consumes nc=1; the INVITE retry on the same
        # nonce must use nc=2+. Reset when Asterisk issues a new nonce.
        if getattr(self, "_auth_nonce_seen", None) != self.nonce:
            self._auth_nc = 0
            self._auth_nonce_seen = self.nonce
        self._auth_nc = getattr(self, "_auth_nc", 0) + 1
        return f"{self._auth_nc:08x}"

    def sdp_sendrecv(self) -> str:
        return (
            "v=0\r\n"
            f"o=- {secrets.randbits(31)} {secrets.randbits(31)} IN IP4 {self.local_ip}\r\n"
            "s=-\r\n"
            f"c=IN IP4 {self.local_ip}\r\n"
            "t=0 0\r\n"
            f"m=audio {self.rtp_port} RTP/AVP 0 8\r\n"
            "a=rtpmap:0 PCMU/8000\r\n"
            "a=rtpmap:8 PCMA/8000\r\n"
            "a=sendrecv\r\n"
        )

    def run_caller(self, extension: str, phrase_raw: str,
                   recv_wav: str, go_file: str) -> None:
        """LF-012 inbound governed-call caller.

        Registers with real digest auth, dials a 1XX extension (the
        dialplan moves the call into the canonical Stasis app), answers
        media, waits for the orchestrator's GO flag, streams a REAL
        speech phrase (8k PCMU raw) over RTP, then records the RTP it
        receives back (the real TTS response) into a WAV.
        """
        import re
        import wave

        # 1. Register with the real digest exchange.
        self.register_once(False)
        deadline = time.monotonic() + 12
        while time.monotonic() < deadline and not self.registered:
            try:
                data, source = self.sock.recvfrom(65535)
            except socket.timeout:
                continue
            line, headers, _ = parse_message(data)
            if line.startswith("SIP/2.0 401"):
                auth = headers.get("www-authenticate", "")
                params = parse_auth_params(auth)
                self.nonce = params.get("nonce")
                self.realm = params.get("realm", "asterisk")
                self.register_once(True)
            elif line.startswith("SIP/2.0 200") and "REGISTER" in headers.get("cseq", ""):
                self.registered = True
                print("REGISTERED", flush=True)
        if not self.registered:
            print("CALLER FAIL: registration timeout", flush=True)
            return

        # 2. Send the INVITE (digest-authenticated: PJSIP challenges
        #    outbound INVITEs from an auth-protected endpoint) and
        #    handle the 200 (media target learned).
        # Brief pause: Asterisk's digest nonce embeds a timestamp, so an
        # INVITE sent in the SAME second as the REGISTER gets the SAME
        # nonce; PJSIP's nonce-count check then requires a strictly
        # increasing nc across request types and rejects a mismatch.
        # Waiting a second lets the challenge rotate to a fresh nonce
        # (baresip's INVITE also uses a fresh nonce).
        time.sleep(1.1)
        invite = self.build_invite(extension)
        self.sock.sendto(invite, (ASTERISK_HOST, ASTERISK_PORT))
        print(f"CALLER INVITE sent to {extension}", flush=True)
        answered = False
        call_id = None
        to_hdr = None
        from_tag = getattr(self, "_invite_from_tag", None) or secrets.token_hex(8)
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline and not answered:
            try:
                data, source = self.sock.recvfrom(65535)
            except socket.timeout:
                continue
            line, headers, body = parse_message(data)
            if not line:
                continue
            call_id = headers.get("call-id", call_id)
            if line.startswith("SIP/2.0 401") and "INVITE" in headers.get("cseq", ""):
                auth = headers.get("www-authenticate", "")
                params = parse_auth_params(auth)
                self.nonce = params.get("nonce")
                self.realm = params.get("realm", "asterisk")
                self._challenge_opaque = params.get("opaque", "")
                print("CALLER INVITE challenged (401)", flush=True)
                self.sock.sendto(self.build_invite(extension, with_auth=True),
                                 (ASTERISK_HOST, ASTERISK_PORT))
            elif line.startswith("SIP/2.0 200") and "INVITE" in headers.get("cseq", ""):
                to_hdr = headers.get("to", "")
                # Learn the RTP target from Asterisk's SDP offer.
                c_match = re.search(r"c=IN IP4 ([0-9.]+)", body)
                m_match = re.search(r"m=audio (\d+)", body)
                if c_match and m_match:
                    self.rtp_target = (c_match.group(1), int(m_match.group(1)))
                    print(f"CALLER RTP_TARGET {self.rtp_target[0]}:{self.rtp_target[1]}", flush=True)
                ack = (
                    f"ACK sip:{extension}@{ASTERISK_HOST} SIP/2.0\r\n"
                    f"Via: SIP/2.0/UDP {self.local_ip}:{self.sip_port};branch=z9hG4bK-{secrets.token_hex(8)}\r\n"
                    "Max-Forwards: 70\r\n"
                    f"From: <sip:{self.name}@{ASTERISK_HOST}>;tag={from_tag}\r\n"
                    f"To: {to_hdr}\r\n"
                    f"Call-ID: {call_id}\r\n"
                    "CSeq: 2 ACK\r\n"
                    "Content-Length: 0\r\n\r\n"
                )
                self.sock.sendto(ack.encode(), (ASTERISK_HOST, ASTERISK_PORT))
                answered = True
                print("CALLER ANSWERED (200 OK, ACK sent)", flush=True)
            elif line.startswith("SIP/2.0"):
                code = line.split(" ")[1]
                print(f"CALLER PROVISIONAL/RESPONSE {code}", flush=True)

        if not answered:
            print("CALLER FAIL: no 200 OK for INVITE", flush=True)
            return

        # 3. Wait for the orchestrator's GO flag (bridge + ARI record
        #    are live before the caller speaks).
        if go_file:
            waited = 0
            while waited < 15 and not os.path.exists(go_file):
                time.sleep(0.2)
                waited += 0.2
            if not os.path.exists(go_file):
                print("CALLER FAIL: GO flag timeout", flush=True)
                return
            print("CALLER GO received", flush=True)

        # 4. Stream the real phrase (8k PCMU raw) over RTP.
        phrase = b""
        if phrase_raw and os.path.exists(phrase_raw):
            with open(phrase_raw, "rb") as f:
                phrase = f.read()
        if not phrase:
            print("CALLER FAIL: empty phrase raw", flush=True)
            return
        print(f"CALLER SPEAK phrase_bytes={len(phrase)}", flush=True)
        ssrc = secrets.randbits(32)
        seq = secrets.randbits(16)
        ts = secrets.randbits(32)
        i = 0
        next_tick = time.monotonic()
        while i + 160 <= len(phrase):
            now = time.monotonic()
            if now < next_tick:
                time.sleep(min(0.005, next_tick - now))
                continue
            payload = phrase[i:i + 160]
            header = bytes([0x80, 0x00]) + seq.to_bytes(2, "big") + ts.to_bytes(4, "big") + ssrc.to_bytes(4, "big")
            self.rtp_sock.sendto(header + payload, self.rtp_target)
            seq = (seq + 1) & 0xFFFF
            ts = (ts + 160) & 0xFFFFFFFF
            i += 160
            next_tick += 0.020
        print(f"CALLER SPOKE frames={i // 160}", flush=True)

        # 5. Record received RTP (the real TTS response) until BYE.
        # The orchestrator transcribes + synthesizes the response before
        # playing it (whisper + Kokoro take tens of seconds), so the
        # receive window must outlive the whole sequence; the call ends
        # with a real BYE from Asterisk.
        #
        # The receive loop must DRAIN the RTP socket in a tight inner
        # loop: the TTS playback arrives as a ~2s burst at 20ms cadence,
        # and an alternating read (one RTP datagram then one SIP read)
        # would only capture a few packets per burst. SIP is polled
        # non-blockingly between RTP drains.
        received = b""
        start = time.monotonic()
        bye_deadline = start + 150
        sip_timeout = 0.2
        self.sock.settimeout(sip_timeout)
        while time.monotonic() < bye_deadline:
            # Drain everything currently buffered on the RTP socket.
            while True:
                try:
                    data, _ = self.rtp_sock.recvfrom(2048)
                    if len(data) >= 12:
                        received += data[12:]
                except socket.timeout:
                    break
            try:
                sdata, bye_source = self.sock.recvfrom(65535)
            except socket.timeout:
                continue
            line, headers, _ = parse_message(sdata)
            if line.startswith("BYE"):
                print("CALLER BYE received", flush=True)
                resp = self.build_response(line, headers, "200", "OK")
                self.sock.sendto(resp, bye_source)
                break
        self.sock.settimeout(0.5)

        # 6. Write the far-end WAV (PCMU -> 16-bit PCM).
        if recv_wav and received:
            # G.711 mu-law decode to signed 16-bit PCM (standard table).
            ulaw = [0] * 256
            for j in range(256):
                u = ~j & 0xFF
                t = ((u & 0x0F) << 3) + 0x84
                t = t << ((u & 0x70) >> 4)
                val = t - 0x84 if (u & 0x80) == 0 else 0x84 - t
                ulaw[j] = val
            samples = bytearray()
            for b in received:
                samples += int(ulaw[b]).to_bytes(2, "little", signed=True)
            with wave.open(recv_wav, "wb") as w:
                w.setnchannels(1)
                w.setsampwidth(2)
                w.setframerate(8000)
                w.writeframes(bytes(samples))
            print(f"CALLER RECEIVED_WAV {recv_wav} bytes={len(received)}", flush=True)
        else:
            print(f"CALLER RECEIVED_WAV EMPTY {recv_wav} received={len(received)}", flush=True)

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


def self_test_dialog() -> int:
    """Structural regression test for the LF-012 caller dialog model.

    Proves, without any network:
      - REGISTER Call-ID != INVITE Call-ID (separate SIP usages);
      - REGISTER From tag  != INVITE From tag;
      - INVITE #1 Call-ID  == authenticated INVITE Call-ID (same dialog);
      - INVITE #1 From tag == authenticated INVITE From tag;
      - INVITE #1 Via branch != authenticated retry Via branch
        (new transaction, same dialog);
      - INVITE CSeq 1 -> 2 on the authenticated retry;
      - Authorization header appears BEFORE the header/body separator
        (the PJSIP 'No Authorization header found' regression).
    """
    import re

    # Pure message-shape test: construct without binding sockets
    # (the caller fixture's wire sockets are not needed to prove the
    # dialog identity contract).
    r = Responder.__new__(Responder)
    r.name = "endpoint-v"
    r.password = "selftest-password"
    r.sip_port = 12130
    r.rtp_port = 12140
    r.mode = "caller"
    r.send_seconds = 8
    r.local_ip = "172.17.0.1"
    r.contact = f"<sip:endpoint-v@172.17.0.1:12130>"
    r.auth_uri = f"sip:{ASTERISK_HOST}"
    r.nonce = "1787000000/selftestnonce"
    r.realm = "asterisk"
    r._auth_nc = 0
    r._auth_nonce_seen = None
    r._invite_call_id = None
    r._invite_from_tag = None
    r._challenge_opaque = ""

    reg = r.build_register(False).decode()
    reg_auth = r.build_register(True).decode()
    inv1 = r.build_invite("110", with_auth=False).decode()
    inv2 = r.build_invite("110", with_auth=True).decode()

    def grab(msg, key):
        m = re.search(rf"^{key}: (.*)$", msg, re.M)
        return m.group(1).strip() if m else None

    reg_cid = grab(reg, "Call-ID")
    reg_tag = grab(reg, "From")
    inv_cid = grab(inv1, "Call-ID")
    inv_tag = grab(inv1, "From")
    inv2_cid = grab(inv2, "Call-ID")
    inv2_tag = grab(inv2, "From")
    br1 = grab(inv1, "Via")
    br2 = grab(inv2, "Via")
    cseq1 = grab(inv1, "CSeq")
    cseq2 = grab(inv2, "CSeq")
    auth_hdr = grab(inv2, "Authorization")

    checks = [
        ("REGISTER Call-ID != INVITE Call-ID", reg_cid != inv_cid),
        ("REGISTER From tag != INVITE From tag", reg_tag != inv_tag),
        ("INVITE #1 Call-ID == retry Call-ID", inv_cid == inv2_cid),
        ("INVITE #1 From tag == retry From tag", inv_tag == inv2_tag),
        ("INVITE #1 Via branch != retry Via branch", br1 != br2),
        ("INVITE CSeq 1 -> 2", cseq1 == "1 INVITE" and cseq2 == "2 INVITE"),
        ("Authorization present on retry", bool(auth_hdr)),
        ("Authorization BEFORE body separator", inv2.find(auth_hdr) < inv2.find("\r\n\r\n") if auth_hdr else False),
    ]
    ok = True
    for name, passed in checks:
        print(f"SELFTEST {'ok' if passed else 'FAIL'} - {name}", flush=True)
        ok = ok and passed
    print(f"SELFTEST {'PASS' if ok else 'FAIL'}", flush=True)
    return 0 if ok else 1


def main() -> int:
    ap = argparse.ArgumentParser(description="EP-025 controlled SIP responder")
    ap.add_argument("--name", required=True)
    ap.add_argument("--password", required=True)
    ap.add_argument("--sip-port", type=int, required=True)
    ap.add_argument("--rtp-port", type=int, required=True)
    ap.add_argument("--mode", required=True,
                    choices=["603", "486", "hybrid", "ring", "silent", "sender", "probe", "caller", "selftest"])
    ap.add_argument("--send-seconds", type=int, default=8)
    ap.add_argument("--dial", default="110", help="extension to dial (caller mode)")
    ap.add_argument("--phrase-raw", default="", help="8k PCMU raw file to stream as caller speech (caller mode)")
    ap.add_argument("--recv-wav", default="", help="WAV path for far-end received audio (caller mode)")
    ap.add_argument("--go-file", default="", help="orchestrator GO flag; caller waits before speaking (caller mode)")
    args = ap.parse_args()

    responder = Responder(args.name, args.password, args.sip_port,
                          args.rtp_port, args.mode, args.send_seconds)
    try:
        if args.mode == "selftest":
            return self_test_dialog()
        if args.mode == "caller":
            responder.run_caller(args.dial, args.phrase_raw, args.recv_wav, args.go_file)
        else:
            responder.run()
    except KeyboardInterrupt:
        pass
    return 0


if __name__ == "__main__":
    sys.exit(main())
