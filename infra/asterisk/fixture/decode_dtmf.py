#!/usr/bin/env python3
"""Decode RFC4733 telephone-event (DTMF) digits from a tcpdump pcap.

EP-025 M3/M4/M5 DTMF wire proof: Asterisk sends DTMF to the far
endpoint as RTP telephone-event packets (payload type 101 per
RFC4733). ARI-injected DTMF does NOT emit a ChannelDtmfReceived WS
event (Asterisk treats it as locally generated), so the authoritative
evidence is the RFC4733 capture on the receiving endpoint's RTP
socket.

This decoder walks a pcap captured on docker0 (linktype 276 = SLL2 on
this host; also handles SLL 113 and Ethernet 1), finds UDP/RTP packets
with PT=101, and extracts the event byte (digit).

Usage:
  decode_dtmf.py <pcap>

Exit 0 with "ordered_digits: <seq>" on the first-occurrence digit
sequence (e.g. 539) when at least one telephone-event packet exists;
exit 1 when no telephone-event packets are found.
"""

import struct
import sys


def decode_pcap(path):
    """Return [(src_port, dst_port, event_byte), ...] for PT=101 packets."""
    digits = []
    with open(path, "rb") as f:
        gh = f.read(24)
        if len(gh) < 24 or gh[:4] not in (b"\xd4\xc3\xb2\xa1", b"\xa1\xb2\xc3\xd4"):
            print("decode_dtmf: not a classic pcap", file=sys.stderr)
            return digits
        little = gh[:4] == b"\xd4\xc3\xb2\xa1"
        linktype = struct.unpack("<I" if little else ">I", gh[20:24])[0]
        while True:
            rec = f.read(16)
            if len(rec) < 16:
                break
            if little:
                incl_len = struct.unpack("<I", rec[8:12])[0]
            else:
                incl_len = struct.unpack(">I", rec[8:12])[0]
            pkt = f.read(incl_len)
            if len(pkt) < incl_len:
                break
            if linktype == 276:  # SLL2 (Linux cooked v2): proto at 0..2
                if len(pkt) < 20:
                    continue
                proto = struct.unpack(">H", pkt[0:2])[0]
                off = 20
            elif linktype == 113:  # SLL v1: proto at 14..16
                if len(pkt) < 16:
                    continue
                proto = struct.unpack(">H", pkt[14:16])[0]
                off = 16
            else:  # Ethernet (1): proto at 12..14
                if len(pkt) < 14:
                    continue
                proto = struct.unpack(">H", pkt[12:14])[0]
                off = 14
            if proto != 0x0800:  # IPv4 only
                continue
            if len(pkt) < off + 20:
                continue
            ihl = (pkt[off] & 0x0F) * 4
            if len(pkt) < off + ihl + 8:
                continue
            if pkt[off + 9] != 17:  # UDP
                continue
            sport = struct.unpack(">H", pkt[off + ihl:off + ihl + 2])[0]
            dport = struct.unpack(">H", pkt[off + ihl + 2:off + ihl + 4])[0]
            udp_len = struct.unpack(">H", pkt[off + ihl + 4:off + ihl + 6])[0]
            payload = pkt[off + ihl + 8: off + ihl + udp_len]
            if len(payload) < 13:
                continue
            # RTP header: v=2, PT in byte 1
            if payload[0] >> 6 != 2:
                continue
            pt = payload[1] & 0x7F
            if pt != 101:  # telephone-event (RFC4733)
                continue
            event = payload[12]
            digits.append((sport, dport, event))
    return digits


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else "/tmp/ep025-rtp.pcap"
    digs = decode_pcap(path)
    if not digs:
        print("ordered_digits: (none)", file=sys.stderr)
        print("decode_dtmf: no telephone-event packets found", file=sys.stderr)
        return 1
    names = "0123456789#*ABCD"
    ordered = []
    for _, _, event in digs:
        name = names[event] if event < len(names) else "?"
        if name not in ordered:
            ordered.append(name)
    print(f"telephone-event packets: {len(digs)}")
    for sport, dport, event in digs[:40]:
        name = names[event] if event < len(names) else "?"
        print(f"  {sport} -> {dport}  event={event}  digit={name}")
    print(f"ordered_digits: {''.join(ordered)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
