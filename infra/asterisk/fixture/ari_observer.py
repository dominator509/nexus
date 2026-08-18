#!/usr/bin/env python3
"""EP-025 M3 ARI WebSocket events observer (real consumer).

Connects to the REAL Asterisk ARI events WebSocket
(ws://127.0.0.1:8088/ari/events?api_key=...&app=nexus-telephony),
subscribes to the nexus-telephony Stasis application, and appends
every event (JSON line) to the events file. Prints READY once the
WebSocket is connected so the gate can sequence the integration suite
after subscription.

This is a TEST HARNESS (real WS client, real events from the real
Asterisk provider). The production control surface is the Rust
adapter; this observer only records Asterisk's own events for the
suite's assertions (StasisStart / ChannelDtmfReceived / StasisEnd).

Usage:
  ari_observer.py <env_file> <events_file>
"""

import json
import os
import sys
import time

import websocket

env_file, events_file = sys.argv[1], sys.argv[2]

env = {}
with open(env_file) as f:
    for line in f:
        line = line.strip()
        if "=" in line and not line.startswith("#"):
            k, v = line.split("=", 1)
            env[k] = v

user = env["NEXUS_ARI_USER"]
pwd = env["NEXUS_ARI_PASSWORD"]
url = f"ws://127.0.0.1:8088/ari/events?api_key={user}:{pwd}&app=nexus-telephony"

# Reconnect loop: the gate may restart the container (M3 restart test).
# On every successful connect we append an ObserverReady marker to the
# events file so tests can wait for the subscription to be re-established
# after a container restart (StasisStart events before READY are lost).
while True:
    try:
        ws = websocket.create_connection(url, timeout=15)
        with open(events_file, "a") as f:
            f.write(json.dumps({"type": "ObserverReady", "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S+0000", time.gmtime())}) + "\n")
        print("OBSERVER: READY", flush=True)
        ws.settimeout(1.0)
        while True:
            try:
                msg = ws.recv()
                if not msg:
                    continue
                ev = json.loads(msg)
                with open(events_file, "a") as f:
                    f.write(json.dumps(ev) + "\n")
                t = ev.get("type")
                if t in ("StasisStart", "ChannelDtmfReceived", "StasisEnd", "ChannelDestroyed"):
                    cid = (ev.get("channel") or {}).get("id", "?")
                    extra = f" digit={ev.get('digit')}" if t == "ChannelDtmfReceived" else ""
                    print(f"OBSERVER: {t} channel={cid}{extra}", flush=True)
            except websocket.WebSocketTimeoutException:
                continue
            except websocket.WebSocketConnectionClosedException:
                break
            except Exception:
                continue
    except Exception as e:
        print(f"OBSERVER: reconnect in 2s ({type(e).__name__})", flush=True)
        time.sleep(2)
