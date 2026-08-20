#!/usr/bin/env python3
"""Controlled AT modem peer for the EP-032 M3 Gammu SMSD fixture.

This is a TEST FIXTURE (TESTING.md test zone): it emulates the GSM
modem at the AT serial boundary ONLY. The component under test is the
real gammu-smsd daemon + real SQL backend + the Rust connector. The
modem/carrier boundary is honestly classified NOT ASSERTED.

Implements the documented AT command surface that gammu-smsd's AT
driver exercises, including SMS-SUBMIT (AT+CMGS in PDU mode) and the
SMS-STATUS-REPORT unsolicited +CDS delivery indication (GSM 03.40
section 9.2.3.15 TP-Status, 0x00 = delivered to handset).

Usage: at_modem.py <device>
"""
import os
import re
import sys
import time
import threading

DEVICE = sys.argv[1] if len(sys.argv) > 1 else "/tmp/smsd-probe/modem-ctrl"

IMEI = "352099001761497"
IMSI = "310150123456789"

_log_lock = threading.Lock()


def log(msg: str) -> None:
    with _log_lock:
        print(f"[at_modem] {msg}", flush=True)


def pdu_hex(b: bytes) -> str:
    return b.hex().upper()


def parse_submit(pdu_hex_str: str):
    """Parse an SMS-SUBMIT PDU; return (mr, destination_digits)."""
    raw = pdu_hex_str.strip().replace(" ", "").replace("\r", "").replace("\n", "")
    data = bytes.fromhex(raw)
    pos = 0
    smsc_len = data[0]
    pos = 1
    if smsc_len > 0:
        pos += smsc_len  # SMSC address field is smsc_len bytes
    first_octet = data[pos]
    pos += 1
    mr = data[pos]
    pos += 1
    addr_len = data[pos]
    pos += 1
    ton_npi = data[pos]
    pos += 1
    digits = ""
    n = (addr_len + 1) // 2
    addr_bytes = data[pos:pos + n]
    for i in range(len(addr_bytes)):
        b = addr_bytes[i]
        digits += chr((b & 0x0F) + 0x30)
        hi = (b >> 4) & 0x0F
        if hi < 0x0F:
            digits += chr(hi + 0x30)
    if len(digits) > addr_len:
        digits = digits[:addr_len]
    return mr, digits, first_octet, ton_npi


def _swap2(s: str) -> str:
    """Swap two decimal digits into GSM semi-octet order."""
    return s[1] + s[0]


def _gsm_timestamp_now() -> str:
    """Current UTC time as a GSM 03.40 SCTS/DT field (7 bytes + TZ)."""
    s = time.strftime("%y%m%d%H%M%S", time.gmtime())
    return "".join(_swap2(s[i:i + 2]) for i in range(0, 12, 2)) + "00"


def build_status_report(mr: int, dest_digits: str, ton_npi: int = 0x91, status: int = 0x00, smsc: str = "07915155214365F7") -> str:
    """Build a +CDS SMS-STATUS-REPORT PDU (GSM 03.40) with TP-Status.

    Includes the same SMSC address the submit PDU used, as a real
    modem's status report does, so SMSD can match the report to the
    sent message. Timestamps are the current time so the daemon's
    delivery-report delay check passes.
    """
    # Address field: odd-length digits get 0xF filler in the low nibble.
    digits = dest_digits
    if len(digits) % 2 == 1:
        digits += "F"
    addr_field = ""
    for i in range(0, len(digits), 2):
        lo = int(digits[i], 16)
        hi = int(digits[i + 1], 16)
        addr_field += f"{hi:01X}{lo:01X}"
    addr_len = len(dest_digits)
    ra = f"{addr_len:02X}{ton_npi:02X}{addr_field}"
    scts = _gsm_timestamp_now()
    dt = _gsm_timestamp_now()
    pdu = f"{smsc}06{mr:02X}{ra}{scts}{dt}{status:02X}"
    return pdu


class AtModem:
    def __init__(self, device: str):
        self.device = device
        self.sent_mrs = []
        self.delivery_reports = []
        self._stop = threading.Event()
        # M4 failure modes (controlled fixture; SIMULATION):
        #   SMSD_NO_REPORT=1        - CMGS accepted, NO +CDS ever sent
        #   SMSD_FAILURE_REPORT=1   - +CDS with TP-Status != 0x00 (failure)
        #   SMSD_UNMATCHED_REPORT=1 - +CDS bound to a DIFFERENT message
        #                             (wrong TPMR/destination) so it can
        #                             never satisfy the current message
        self.no_report = os.environ.get("SMSD_NO_REPORT") == "1"
        self.failure_report = os.environ.get("SMSD_FAILURE_REPORT") == "1"
        self.unmatched_report = os.environ.get("SMSD_UNMATCHED_REPORT") == "1"

    def read_line(self, f) -> str:
        line = f.readline()
        if not line:
            return ""
        return line.strip()

    def send(self, f, s: str) -> None:
        f.write(s)
        f.flush()

    # PTY-safe raw I/O: os.read/os.write on the device fd (no seek).
    _buf = b""

    def raw_read_line(self, fd: int) -> str:
        # AT protocol terminates lines with \r (no \n); an SMS PDU is
        # terminated with Ctrl-Z (0x1A). Split on the first of those
        # so a bare "AT\r" or "PDU\x1a" is a complete line.
        while b"\r" not in self._buf and b"\n" not in self._buf and b"\x1a" not in self._buf:
            chunk = os.read(fd, 256)
            if not chunk:
                return ""
            self._buf += chunk
        for sep in (b"\r", b"\n", b"\x1a"):
            if sep in self._buf:
                line, self._buf = self._buf.split(sep, 1)
                return line.decode("utf-8", "replace").strip()
        return ""

    def raw_send(self, fd: int, s: str) -> None:
        os.write(fd, s.encode("utf-8"))

    def handle_cmgs(self, fd: int, m: re.Match, echo_on: bool = False) -> None:
        length = int(m.group(1))
        # Modem prompts with \r\n> then waits for the PDU hex line.
        # With echo enabled the command line is reflected first; both
        # arrive contiguously so gammu parses one frame.
        prompt = ("\r\n" + "AT+CMGS=" + str(length) + "\r\n" if echo_on else "") + "\r\n> "
        self.raw_send(fd, prompt)
        time.sleep(0.05)
        pdu_line = self.raw_read_line(fd)
        # The PDU is terminated by Ctrl-Z (0x1A); the next command can
        # arrive in the same read. Split at 0x1A and push back the rest.
        if "\x1a" in pdu_line:
            pdu_part, rest = pdu_line.split("\x1a", 1)
            pdu_line = pdu_part
            if rest:
                self._buf = rest.encode("utf-8") + self._buf
        pdu_line = pdu_line.replace("\r", "")
        log(f"CMGS PDU({length}): {pdu_line}")
        try:
            mr, dest, _fo, ton = parse_submit(pdu_line)
        except Exception as e:  # pragma: no cover - defensive
            log(f"CMGS parse failed: {e}")
            self.raw_send(fd, "\r\n+CMGS: 0\r\nOK\r\n")
            return
        self.sent_mrs.append(mr)
        self.raw_send(fd, f"\r\n+CMGS: {mr}\r\nOK\r\n")
        log(f"CMGS accepted: mr={mr} dest={dest}")

        def _deliver_report():
            time.sleep(1.0)
            if self._stop.is_set():
                return
            if self.no_report:
                # M4: no delivery report arrives; the message stays
                # in a non-delivered provider state (SendingOK).
                log("+CDS suppressed (SMSD_NO_REPORT=1)")
                return
            status = 0x00
            if self.failure_report:
                # TP-Status 0x41 = permanent error (GSM 03.40
                # section 9.2.3.15; gammu 1.42 classifies TP-Status
                # with bit 0x40 as "Failed", verified in
                # libgammu/service/sms/gsmsms.c
                # GSM_DecodeSMSStatusReportData); the daemon records
                # the documented DeliveryFailed state.
                status = 0x41
            if self.unmatched_report:
                # Report bound to a DIFFERENT message: bump the TPMR
                # and use a different destination so the daemon can
                # never correlate it to the current message.
                wrong_mr = (mr + 1) % 256
                wrong_dest = "15550000000"
                pdu = build_status_report(wrong_mr, wrong_dest, ton, status)
                log(f"+CDS (unmatched) -> {pdu}")
                with _log_lock:
                    try:
                        d = os.open(self.device, os.O_RDWR | os.O_NOCTTY)
                        os.write(
                            d,
                            f"\r\n+CDS: {len(bytes.fromhex(pdu.replace(' ', '')))}\r\n".encode(),
                        )
                        time.sleep(0.05)
                        os.write(d, (pdu + "\r\n").encode())
                        os.close(d)
                    except Exception as e:  # pragma: no cover
                        log(f"+CDS write failed: {e}")
                self.delivery_reports.append((wrong_mr, wrong_dest))
                return
            pdu = build_status_report(mr, dest, ton, status)
            log(f"+CDS -> {pdu}")
            with _log_lock:
                try:
                    d = os.open(self.device, os.O_RDWR | os.O_NOCTTY)
                    os.write(
                        d,
                        f"\r\n+CDS: {len(bytes.fromhex(pdu.replace(' ', '')))}\r\n".encode(),
                    )
                    time.sleep(0.05)
                    os.write(d, (pdu + "\r\n").encode())
                    os.close(d)
                except Exception as e:  # pragma: no cover
                    log(f"+CDS write failed: {e}")
            self.delivery_reports.append((mr, dest))

        t = threading.Thread(target=_deliver_report, daemon=True)
        t.start()

    def run(self) -> None:
        import fcntl
        import termios

        fd = os.open(self.device, os.O_RDWR | os.O_NOCTTY)
        f = os.fdopen(fd, "rb", buffering=0)
        # Raw-ish mode: no echo, no CR/LF translation.
        try:
            attrs = termios.tcgetattr(fd)
            attrs[3] &= ~(termios.ECHO | termios.ICANON | termios.ICRNL)
            attrs[1] &= ~(termios.OPOST)
            termios.tcsetattr(fd, termios.TCSANOW, attrs)
        except Exception as e:  # pragma: no cover
            log(f"termios not applied: {e}")
        log(f"ready on {self.device}")
        echo_on = False
        while not self._stop.is_set():
            line = self.raw_read_line(fd)
            if not line:
                time.sleep(0.05)
                continue
            line = line.replace("\r", "").replace("\n", "")
            log(f"RX: {line!r}")
            if line == "":
                continue
            if line in ("\x1b", "AT\x1b"):
                # Escape: a real modem silently exits data mode; no reply.
                continue
            if line == "ATE1" or line == "ATE0":
                echo_on = line == "ATE1"
                self.raw_send(fd, "OK\r\n")
                continue
            # Build the full modem reply. With echo enabled, a real
            # modem reflects the command line before its response and
            # both arrive contiguously; gammu parses them as one
            # frame (echoed command + OK), so we emit a single write.
            reply = ""
            if line == "AT":
                reply = "OK"
            elif line.startswith("AT+MODE"):
                # Motorola probe: not a Motorola phone, answer ERROR.
                reply = "ERROR"
            elif line.startswith("AT+CGMI"):
                reply = "NEXUS-FAKE-MODEM\r\nOK"
            elif line.startswith("AT+CGMM"):
                reply = "NEXUS-FAKE-MODEM\r\nOK"
            elif line.startswith("AT+CGMR"):
                reply = "1.0\r\nOK"
            elif line.startswith("AT+CGSN"):
                reply = IMEI + "\r\nOK"
            elif line.startswith("AT+CIMI"):
                reply = IMSI + "\r\nOK"
            elif line.startswith("AT+CSQ"):
                reply = "+CSQ: 15,0\r\nOK"
            elif line.startswith("AT+CBC"):
                reply = "+CBC: 0,80\r\nOK"
            elif line.startswith("AT+CMGS"):
                self.handle_cmgs(fd, re.match(r"AT\+CMGS=(\d+)", line), echo_on)
                continue
            elif line.startswith("AT+CMGF"):
                reply = "OK"
            elif line.startswith("AT+CNMI"):
                reply = "OK"
            elif line.startswith("AT+CPMS"):
                reply = "+CPMS: 1,10,1,10,1,10\r\nOK"
            elif line.startswith("AT+CMGD"):
                reply = "OK"
            elif line.startswith("AT+CMGL"):
                reply = "OK"
            elif line.startswith("AT+CMGR"):
                reply = "OK"
            elif line.startswith("AT+CSMS"):
                reply = "+CSMS: 1,1,1,1\r\nOK"
            elif line.startswith("AT+COPS"):
                reply = "+COPS: 0,0,\"NEXUS-TEST-NET\"\r\nOK"
            elif line.startswith("AT+CREG"):
                reply = "+CREG: 0,1\r\nOK"
            elif line.startswith("AT+CPIN"):
                reply = "+CPIN: READY\r\nOK"
            elif line.startswith("AT+CMEE"):
                reply = "OK"
            elif line.startswith("AT+CSCA?"):
                # SMSC query: real modems return the service centre
                # number; gammu needs it to build the SMS-SUBMIT PDU.
                reply = "+CSCA: \"+15551234567\",145\r\nOK"
            elif line.startswith("AT+CSCA="):
                reply = "OK"
            elif line.startswith("AT+CSCS"):
                reply = "+CSCS: \"GSM\"\r\nOK"
            elif line.startswith("ATE"):
                reply = "OK"
            elif line.startswith("AT+"):
                # Unknown AT command: answer OK (generic phone behavior).
                reply = "OK"
            elif line.startswith("AT"):
                reply = "OK"
            else:
                reply = "OK"
            if echo_on:
                self.raw_send(fd, "\r\n" + line + "\r\n" + reply + "\r\n")
            else:
                self.raw_send(fd, reply + "\r\n")


if __name__ == "__main__":
    AtModem(DEVICE).run()
