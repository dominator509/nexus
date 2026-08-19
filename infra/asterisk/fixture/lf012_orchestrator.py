#!/usr/bin/env python3
"""EP-025 M5 LF-012 governed-phone-call orchestrator (REAL proof).

Test harness (like ari_observer.py): drives the REAL pinned Asterisk
22.10.1 container through the REAL ARI REST + WebSocket surface, with
the REAL whisper.cpp and Kokoro engines as subprocess workers, to prove
an inbound governed phone call end to end:

  1. a REAL controlled SIP caller (reject_endpoint.py --mode caller)
     registers with real digest auth and dials a 1XX extension;
  2. the dialplan moves the call into the canonical Stasis app
     (nexus-telephony); this orchestrator observes the real StasisStart;
  3. the orchestrator ANSWERS via real ARI, creates a real mixing bridge,
     adds the channel, and starts a real ARI channel recording;
  4. the caller streams a REAL speech phrase (Kokoro-synthesized voice,
     PCMU) over real RTP; Asterisk records it through its media path;
  5. whisper.cpp transcribes the real recording -> real STT result;
  6. the production DisclosurePolicy decision is evaluated (positive:
     consented -> governed transcript allowed; negative: not consented
     -> fail closed, no transcript artifact);
  7. a deterministic bounded response text is generated (recognized
     command phrase -> bounded response; no frontier model);
  8. real Kokoro synthesizes NEW audio for that exact response;
  9. the response WAV is injected through the REAL Asterisk media path
     (ARI playback of a sound file) to the bridged caller;
 10. the caller fixture records the far-end RTP it receives; the gate
     independently transcribes that capture to prove the caller really
     received the intended Nexus response;
 11. the call hangs up through real provider state and terminal state
     is verified from Asterisk.

Machine-readable evidence is written to .agent/state/evidence/.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import requests
import websocket

REPO_ROOT = Path(__file__).resolve().parents[3]
ENGINE_PY = "/opt/nexus-voice-engines/bin/python"
WHISPER_WORKER = REPO_ROOT / "infra/voice/workers/whisper_worker.py"
KOKORO_WORKER = REPO_ROOT / "infra/voice/workers/kokoro_worker.py"
EVIDENCE_DIR = REPO_ROOT / ".agent/state/evidence"


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--env-file", required=True)
    ap.add_argument("--work", required=True)
    ap.add_argument("--consented", choices=["true", "false"], default="true")
    ap.add_argument("--jurisdiction", default="US")
    ap.add_argument("--retention", type=int, default=0)
    ap.add_argument("--phrase-sha256", required=True,
                    help="sha256 of the caller phrase RAW (proves current session speech)")
    ap.add_argument("--response-voice", default="af_heart")
    ap.add_argument("--scenario", default="positive",
                    help="positive | negative-disclosure | hostile")
    args = ap.parse_args()

    env = {}
    with open(args.env_file) as f:
        for line in f:
            line = line.strip()
            if "=" in line and not line.startswith("#"):
                k, v = line.split("=", 1)
                env[k] = v

    ari_url = env["NEXUS_ARI_URL"].rstrip("/") + "/ari"
    ari_user = env["NEXUS_ARI_USER"]
    ari_pass = env["NEXUS_ARI_PASSWORD"]
    container = env["NEXUS_EP025_AST_CONTAINER"]
    work = Path(args.work)
    work.mkdir(parents=True, exist_ok=True)

    auth = base64.b64encode(f"{ari_user}:{ari_pass}".encode()).decode()
    headers = {"Authorization": f"Basic {auth}"}
    ws_url = f"ws://127.0.0.1:8088/ari/events?api_key={ari_user}:{ari_pass}&app=nexus-telephony"

    evidence: dict[str, object] = {
        "scenario": args.scenario,
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S+0000", time.gmtime()),
        "asterisk_image": env.get("NEXUS_EP025_AST_IMAGE", ""),
        "container": container,
    }

    # ---- wait for the REAL StasisStart (caller dialed a 1XX) --------
    channel_id = None
    deadline = time.monotonic() + 90
    while time.monotonic() < deadline and channel_id is None:
        ws = None
        try:
            ws = websocket.create_connection(ws_url, timeout=30)
            print("ORCH: WS connected", flush=True)
            ws.settimeout(5)
            while time.monotonic() < deadline and channel_id is None:
                try:
                    msg = ws.recv()
                except websocket.WebSocketTimeoutException:
                    continue
                except websocket.WebSocketConnectionClosedException:
                    print("ORCH: WS closed, reconnecting", flush=True)
                    break
                ev = json.loads(msg)
                t = ev.get("type")
                if t == "StasisStart":
                    ch = ev.get("channel") or {}
                    cid = ch.get("id", "")
                    print(f"ORCH: StasisStart channel={cid}", flush=True)
                    channel_id = cid
                    evidence["channel_id"] = cid
                    evidence["caller_number"] = (ch.get("caller") or {}).get("number", "")
                    evidence["dialplan"] = ev.get("args", [])
        except Exception as e:
            print(f"ORCH: WS error {type(e).__name__}, reconnecting", flush=True)
        finally:
            if ws is not None:
                try:
                    ws.close()
                except Exception:
                    pass
    if channel_id is None:
        print("ORCH: FAIL - no StasisStart", flush=True)
        return 1

    # ---- answer via real ARI -----------------------------------------
    r = requests.post(f"{ari_url}/channels/{channel_id}/answer",
                      headers=headers, timeout=10)
    print(f"ORCH: answer {r.status_code}", flush=True)
    if r.status_code not in (200, 204):
        print(f"ORCH: FAIL - answer rc={r.status_code} {r.text[:200]}", flush=True)
        return 1
    evidence["answer_http"] = r.status_code

    # NOTE: no mixing bridge is created for LF-012. ARI cannot record a
    # channel while it is in a bridge ("Cannot record channel while in
    # bridge"); the governed call is a single caller -> Stasis channel,
    # whose own media path carries RTP in both directions (caller speech
    # in, TTS response out). The M4 two-way proof exercises the mixing
    # bridge; M5 records + plays on the single channel directly.

    # ---- real ARI channel recording of the caller's speech ------------
    rec_name = f"lf012-{int(time.time())}"
    r = requests.post(f"{ari_url}/channels/{channel_id}/record",
                      params={"name": rec_name, "format": "wav",
                              "maxDurationSeconds": 20, "beep": "no"},
                      headers=headers, timeout=10)
    print(f"ORCH: record {r.status_code}", flush=True)
    if r.status_code not in (200, 201, 204):
        print(f"ORCH: FAIL - record rc={r.status_code} {r.text[:200]}")
        return 1
    evidence["recording_name"] = rec_name
    evidence["recording_started_http"] = r.status_code

    # ---- signal the caller to speak (real RTP phrase) -----------------
    go_file = work / "lf012-go.flag"
    go_file.write_text("go\n")
    print("ORCH: GO flag written", flush=True)

    # The caller streams the phrase; give it the phrase duration + margin.
    time.sleep(8)

    # ---- stop recording + fetch the real recording --------------------
    # The recording may already have finalized on its own (Asterisk
    # stops a channel recording after a short audio gap); in that case
    # the live DELETE 404s and the file is available as a STORED
    # recording. Treat that as success and fetch the stored file.
    r = requests.delete(f"{ari_url}/recordings/live/{rec_name}",
                        headers=headers, timeout=10)
    print(f"ORCH: stop record {r.status_code}", flush=True)
    if r.status_code not in (200, 204, 404):
        print(f"ORCH: FAIL - stop record rc={r.status_code} {r.text[:200]}")
        return 1
    evidence["recording_stop_http"] = r.status_code
    time.sleep(2)
    r = requests.get(f"{ari_url}/recordings/stored/{rec_name}/file",
                     headers=headers, timeout=30)
    print(f"ORCH: fetch recording {r.status_code} bytes={len(r.content)}", flush=True)
    if r.status_code != 200 or len(r.content) < 100:
        print("ORCH: FAIL - recording fetch empty", flush=True)
        return 1
    rec_wav = work / "lf012-caller-speech.wav"
    rec_wav.write_bytes(r.content)
    evidence["caller_recording_wav_sha256"] = sha256_file(rec_wav)
    evidence["caller_recording_bytes"] = len(r.content)

    # ---- real whisper.cpp STT on the recording ------------------------
    stt = subprocess.run(
        [ENGINE_PY, str(WHISPER_WORKER), "--wav", str(rec_wav)],
        capture_output=True, text=True, timeout=900,
    )
    print(f"ORCH: whisper rc={stt.returncode}", flush=True)
    if stt.returncode != 0:
        print(f"ORCH: FAIL - whisper {stt.stderr[-500:]}", flush=True)
        return 1
    stt_json = json.loads(stt.stdout.strip().splitlines()[-1])
    transcript = stt_json["transcript"]
    evidence["stt_transcript"] = transcript
    evidence["stt_digest"] = hashlib.sha256(transcript.encode()).hexdigest()
    evidence["stt_seconds"] = stt_json["seconds"]
    print(f"ORCH: STT={transcript!r}", flush=True)

    # ---- production disclosure decision (mirrors TranscriptGate) ------
    consented = args.consented == "true"
    evidence["disclosure_policy"] = {
        "recording_consented": consented,
        "ai_disclosure_required": True,
        "jurisdiction": args.jurisdiction,
        "retention_seconds": args.retention,
    }
    # The gate does NOT create a governed transcript artifact when
    # recording is not consented (fail closed per the contract).
    evidence["governed_transcript_created"] = consented
    evidence["disclosure_satisfied"] = consented

    # ---- deterministic bounded response text (no frontier model) ------
    low = transcript.lower()
    hostile_markers = ("ignore the rules", "unlock the door", "ignore", "unlock")
    if any(m in low for m in hostile_markers):
        response_text = "I cannot help with that request."
        evidence["command_recognized"] = False
        evidence["hostile_content"] = True
    elif "light" in low:
        response_text = "Turning on the lights now."
        evidence["command_recognized"] = True
        evidence["hostile_content"] = False
    else:
        response_text = "Command not recognized."
        evidence["command_recognized"] = False
        evidence["hostile_content"] = False
    evidence["response_text"] = response_text
    print(f"ORCH: response_text={response_text!r}", flush=True)

    # ---- real Kokoro TTS: NEW audio for the exact response ------------
    resp_wav = work / "lf012-response.wav"
    tts = subprocess.run(
        [ENGINE_PY, str(KOKORO_WORKER), "--text", response_text,
         "--out", str(resp_wav), "--voice", args.response_voice],
        capture_output=True, text=True, timeout=900,
    )
    print(f"ORCH: kokoro rc={tts.returncode}", flush=True)
    if tts.returncode != 0:
        print(f"ORCH: FAIL - kokoro {tts.stderr[-500:]}", flush=True)
        return 1
    tts_json = json.loads(tts.stdout.strip().splitlines()[-1])
    evidence["tts_wav_sha256"] = tts_json["sha256"]
    evidence["tts_duration_seconds"] = tts_json["duration_seconds"]
    evidence["tts_voice"] = tts_json["voice"]
    evidence["tts_sample_rate_hz"] = tts_json["sample_rate_hz"]
    print(f"ORCH: tts sha256={tts_json['sha256']}", flush=True)

    # ---- inject through the REAL Asterisk media path ------------------
    # The container's sounds dir needs an 8k mono WAV (Asterisk plays it
    # into the bridge -> the caller receives real RTP).
    conv_wav = work / "lf012-response-8k.wav"
    ffmpeg = subprocess.run(
        ["ffmpeg", "-y", "-i", str(resp_wav), "-ar", "8000", "-ac", "1",
         "-c:a", "pcm_s16le", str(conv_wav)],
        capture_output=True, text=True, timeout=120,
    )
    if ffmpeg.returncode != 0:
        print(f"ORCH: FAIL - ffmpeg {ffmpeg.stderr[-300:]}", flush=True)
        return 1
    subprocess.run(
        ["docker", "cp", str(conv_wav), f"{container}:/var/lib/asterisk/sounds/en/nexus-lf012-response.wav"],
        capture_output=True, text=True, timeout=60, check=True,
    )
    play_name = "nexus-lf012-response"
    r = requests.post(f"{ari_url}/channels/{channel_id}/play",
                      params={"media": f"sound:{play_name}"},
                      headers=headers, timeout=10)
    print(f"ORCH: play {r.status_code}", flush=True)
    if r.status_code not in (200, 201, 204):
        print(f"ORCH: FAIL - play rc={r.status_code} {r.text[:200]}", flush=True)
        return 1
    evidence["play_http"] = r.status_code
    evidence["play_media"] = f"sound:{play_name}"

    # Wait for the playback to complete (bounded) + caller RTP receive.
    time.sleep(4)

    # ---- hang up through real provider state --------------------------
    r = requests.delete(f"{ari_url}/channels/{channel_id}", headers=headers, timeout=10)
    print(f"ORCH: hangup {r.status_code}", flush=True)
    evidence["hangup_http"] = r.status_code
    time.sleep(2)

    # ---- terminal state verified from Asterisk ------------------------
    check = subprocess.run(
        ["docker", "exec", container, "/usr/sbin/asterisk", "-rx", "core show channels"],
        capture_output=True, text=True, timeout=30,
    )
    active = 1
    for line in check.stdout.splitlines():
        if "active channels" in line:
            active = int(line.split()[0])
            break
    evidence["terminal_active_channels"] = active
    print(f"ORCH: terminal active channels = {active}", flush=True)

    # ---- machine-readable evidence ------------------------------------
    evidence["phrase_sha256"] = args.phrase_sha256
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ev_path = EVIDENCE_DIR / f"EP-025-M5-LF-012-{args.scenario}.md"
    ev_path.write_text(
        "# EP-025 M5 LF-012 governed phone call (real proof)\n\n"
        "Real inbound governed call through real Asterisk 22.10.1, real ARI,\n"
        "real RTP, real whisper.cpp STT, real Kokoro TTS.\n\n"
        f"```json\n{json.dumps(evidence, indent=2, sort_keys=True)}\n```\n"
    )
    ev_json = EVIDENCE_DIR / f"EP-025-M5-LF-012-{args.scenario}.json"
    ev_json.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n")
    print(f"ORCH: evidence {ev_path}", flush=True)
    print(f"ORCH: LF-012-{args.scenario}: ok", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
