#!/usr/bin/env sh
# EP-025 M4 gate: forced failures, abuse cases, and observability.
#
# Replaces the pre-created EP-001-masking gate (a plain cargo unit
# filter). Orchestrates the REAL pinned Asterisk 22.10.1 container +
# REAL controlled SIP fixtures (baresip a/b/c/d + reject_endpoint.py
# responders r/s/t) and proves, with real wire/event evidence:
#
#   - typed BUSY    (real SIP 486 -> ChannelDestroyed cause 17)
#   - typed REJECTED (real SIP 603 -> cause 21)
#   - typed NO_ANSWER (bounded provider originate timeout -> cause 18/19/102)
#   - wrong PJSIP credential denial (real 401, zero contacts)
#   - wrong ARI credential (truthful auth failure)
#   - Asterisk unavailable (honest failure, no fake session)
#   - one-way media (silent peer: bytes one way only)
#   - mid-call media loss (sender window ends, call stays bridged)
#   - restart during active call (call loss observed, reconnect,
#     re-registration, new real call)
#   - ambiguous originate (no blind retry, exactly one real call)
#   - non-Stasis DTMF (real HTTP 409 -> Conflict)
#   - event-stream disconnect (no fabricated terminal state, reconnect)
#   - contract + transport failure suites (delegated M4 workstreams)
#   - redaction canaries (zero secret leakage)
#   - zero-orphan teardown
#
# Vacuity guards: every phase must produce real observed output; the
# gate never emits "EP-025 M4: ok" on an empty or masked run.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

WORK=/tmp/ep025-ast
ENV_FILE=/tmp/ep025-ast.env
CONTAINER=nexus-ep025-ast
LOG=/tmp/ep025-m4-tests.log
EVENTS=$WORK/ari-events.jsonl
PCAP=$WORK/ep025-m4-media.pcap
DIAG=target/debug/asterisk-diag
: > "$LOG"

echo "EP-025 M4: fixture bootstrap (real pinned Asterisk)" | tee -a "$LOG"

# Vacuity guard 0: prerequisite tools exist.
for tool in docker ffmpeg tcpdump python3 cargo; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "EP-025 M4: FAIL - missing tool $tool" >&2
    exit 1
  }
done

# Vacuity guard 0b: whisper-cli + model exist (media proofs depend on
# the real EP-021 whisper artifact).
[ -x /opt/nexus-whisper/build/bin/whisper-cli ] || {
  echo "EP-025 M4: FAIL - whisper-cli missing" >&2
  exit 1
}
[ -f /opt/nexus-voice-models/ggml-tiny.en.bin ] || {
  echo "EP-025 M4: FAIL - whisper model missing" >&2
  exit 1
}

# Vacuity guard 0c: fixture + responder + suites exist.
[ -f infra/asterisk/fixture/asterisk_bootstrap.py ] || { echo "EP-025 M4: FAIL - bootstrap missing" >&2; exit 1; }
[ -f infra/asterisk/fixture/reject_endpoint.py ] || { echo "EP-025 M4: FAIL - reject responder missing" >&2; exit 1; }
[ -f infra/asterisk/fixture/ari_observer.py ] || { echo "EP-025 M4: FAIL - observer missing" >&2; exit 1; }
[ -f infra/asterisk/fixture/decode_dtmf.py ] || { echo "EP-025 M4: FAIL - DTMF decoder missing" >&2; exit 1; }
[ -f connectors/asterisk/tests/ep025_failure_live.rs ] || { echo "EP-025 M4: FAIL - live failure suite missing" >&2; exit 1; }
[ -f connectors/asterisk/tests/ep025_failure_transport.rs ] || { echo "EP-025 M4: FAIL - transport failure suite missing" >&2; exit 1; }
[ -f crates/nexus-telephony/tests/ep025_failure_contract.rs ] || { echo "EP-025 M4: FAIL - contract failure suite missing" >&2; exit 1; }
[ -f connectors/asterisk/tests/ep025_integration_asterisk.rs ] || { echo "EP-025 M4: FAIL - M3 integration suite missing" >&2; exit 1; }

cleanup() {
  [ -n "${OBSERVER_PID:-}" ] && kill "$OBSERVER_PID" 2>/dev/null || true
  [ -n "${BARESIP_A_PID:-}" ] && kill "$BARESIP_A_PID" 2>/dev/null || true
  [ -n "${BARESIP_B_PID:-}" ] && kill "$BARESIP_B_PID" 2>/dev/null || true
  [ -n "${BARESIP_C_PID:-}" ] && kill "$BARESIP_C_PID" 2>/dev/null || true
  [ -n "${BARESIP_D_PID:-}" ] && kill "$BARESIP_D_PID" 2>/dev/null || true
  [ -n "${RESP_R_PID:-}" ] && kill "$RESP_R_PID" 2>/dev/null || true
  [ -n "${RESP_S_PID:-}" ] && kill "$RESP_S_PID" 2>/dev/null || true
  [ -n "${RESP_T_PID:-}" ] && kill "$RESP_T_PID" 2>/dev/null || true
  [ -n "${RESP_U_PID:-}" ] && kill "$RESP_U_PID" 2>/dev/null || true
  [ -n "${TCPDUMP_PID:-}" ] && kill "$TCPDUMP_PID" 2>/dev/null || true
  sleep 1
  python3 infra/asterisk/fixture/asterisk_bootstrap.py teardown >/dev/null 2>&1 || true
}
trap cleanup EXIT

# Kill stale baresip/observer/responder processes (precise pid
# targeting; avoid pkill -f self-match). SIGKILL so a lingering
# baresip cannot hold the SIP/RTP fixture ports (observed: a stale
# baresip-d kept 5100 bound and the fresh endpoint-d never
# registered).
pkill -9 -x baresip 2>/dev/null || true
pkill -9 -f "reject_endpoint.py" 2>/dev/null || true
pkill -9 -f "ari_observer.py" 2>/dev/null || true
sleep 2

# Vacuity guard 1: fresh fixture start must succeed.
if ! python3 infra/asterisk/fixture/asterisk_bootstrap.py start >>"$LOG" 2>&1; then
  echo "EP-025 M4: FAIL - fixture bootstrap start" >&2
  tail -30 "$LOG" >&2
  exit 1
fi
grep -q "bootstrap: ok" "$LOG" || { echo "EP-025 M4: FAIL - bootstrap sentinel" >&2; exit 1; }

# Export every fixture variable to child processes (avoids repeating
# secret-valued assignments in the test invocation).
set -a
. "$ENV_FILE"
set +a

# Build the diagnostic binary once (used for AUTH/UNAVAILABLE proofs).
cargo build --locked -q -p nexus-asterisk --bin asterisk-diag >>"$LOG" 2>&1 || {
  echo "EP-025 M4: FAIL - asterisk-diag build" >&2
  exit 1
}

# Start the real baresip endpoints (controlled SIP fixtures).
(
  cd /usr/lib/baresip/modules && exec baresip -f "$NEXUS_BARESIP_A_DIR" -s -v -p "$NEXUS_EP025_AUDIO_A_DIR"
) >"$WORK/baresip-a-live.log" 2>&1 &
BARESIP_A_PID=$!
(
  cd /usr/lib/baresip/modules && exec baresip -f "$NEXUS_BARESIP_B_DIR" -s -v -p "$NEXUS_EP025_AUDIO_B_DIR"
) >"$WORK/baresip-b-live.log" 2>&1 &
BARESIP_B_PID=$!
(
  cd /usr/lib/baresip/modules && exec baresip -f "$NEXUS_BARESIP_C_DIR" -s -v -p "$NEXUS_EP025_AUDIO_C_DIR"
) >"$WORK/baresip-c-live.log" 2>&1 &
BARESIP_C_PID=$!
(
  cd /usr/lib/baresip/modules && exec baresip -f "$NEXUS_BARESIP_D_DIR" -s -v -p "$NEXUS_EP025_AUDIO_D_DIR"
) >"$WORK/baresip-d-live.log" 2>&1 &
BARESIP_D_PID=$!

# Start the controlled SIP responders (reject_endpoint.py fixture).
#   r: 603 Decline (typed REJECTED)      s: silent 200 OK (one-way)
#   t: sender 200 OK, 8s RTP, then stop (mid-call media loss)
python3 infra/asterisk/fixture/reject_endpoint.py \
  --name endpoint-r --password "$NEXUS_SIP_R_PASSWORD" \
  --sip-port 12030 --rtp-port 12040 --mode hybrid >"$WORK/responder-r.log" 2>&1 &
RESP_R_PID=$!
python3 infra/asterisk/fixture/reject_endpoint.py \
  --name endpoint-s --password "$NEXUS_SIP_S_PASSWORD" \
  --sip-port 12050 --rtp-port 12060 --mode silent >"$WORK/responder-s.log" 2>&1 &
RESP_S_PID=$!
python3 infra/asterisk/fixture/reject_endpoint.py \
  --name endpoint-t --password "$NEXUS_SIP_T_PASSWORD" \
  --sip-port 12080 --rtp-port 12070 --mode sender --send-seconds 8 >"$WORK/responder-t.log" 2>&1 &
RESP_T_PID=$!
# endpoint-u: SECOND deterministic sender (dedicated to the one-way
# media proof). Its RTP source port 12120 is the stream identity for
# guard 10; endpoint-t's 12070 is reserved for the mid-call-loss
# proof (guard 11). No cross-test RTP can satisfy the wrong guard.
python3 infra/asterisk/fixture/reject_endpoint.py \
  --name endpoint-u --password "$NEXUS_SIP_U_PASSWORD" \
  --sip-port 12090 --rtp-port 12120 --mode sender --send-seconds 8 >"$WORK/responder-u.log" 2>&1 &
RESP_U_PID=$!

# Vacuity guard 2: ALL endpoints must register (per-AOR, exactly one
# usable contact each) within the bound window.
aor_ok() {
  aor=$1
  /usr/bin/docker exec "$CONTAINER" /usr/sbin/asterisk -rx "pjsip show aor $aor" 2>/dev/null \
    | grep -c "^    Contact:  $aor/" || true
}
reg_ok=0
i=0
while [ $i -lt 60 ]; do
  ok=1
  for ep in endpoint-a endpoint-b endpoint-c endpoint-d endpoint-r endpoint-s endpoint-t endpoint-u; do
    n=$(aor_ok "$ep")
    [ "$n" -eq 1 ] || ok=0
  done
  if [ "$ok" = 1 ]; then
    reg_ok=1
    break
  fi
  sleep 1
  i=$((i + 1))
done
[ "$reg_ok" = 1 ] || {
  echo "EP-025 M4: FAIL - endpoints did not register" >&2
  for ep in endpoint-a endpoint-b endpoint-c endpoint-d endpoint-r endpoint-s endpoint-t endpoint-u; do
    echo "  $ep: $(aor_ok "$ep") contacts" >&2
  done
  tail -10 "$WORK/responder-r.log" >&2
  exit 1
}
echo "EP-025 M4: registration ok (a/b/c/d/r/s/t/u = 1 contact each, Asterisk state)" | tee -a "$LOG"

# Vacuity guard 3: wrong PJSIP credential -> real 401 + zero contacts.
python3 infra/asterisk/fixture/reject_endpoint.py \
  --name endpoint-x --password "definitely-wrong-password" \
  --sip-port 12100 --rtp-port 12110 --mode probe >"$WORK/probe-wrong.log" 2>&1 || true
if ! grep -q "PROBE_RESULT 401" "$WORK/probe-wrong.log"; then
  echo "EP-025 M4: FAIL - wrong PJSIP password did not produce real 401" >&2
  cat "$WORK/probe-wrong.log" >&2
  exit 1
fi
[ "$(aor_ok endpoint-x)" -eq 0 ] || {
  echo "EP-025 M4: FAIL - wrong-credential AOR has contacts" >&2
  exit 1
}
echo "EP-025 M4: wrong PJSIP credential denied (real 401, 0 contacts)" | tee -a "$LOG"

# Correct credential registers (sanity: the probe itself is real).
python3 infra/asterisk/fixture/reject_endpoint.py \
  --name endpoint-x --password "$NEXUS_SIP_X_PASSWORD" \
  --sip-port 12100 --rtp-port 12110 --mode probe >"$WORK/probe-right.log" 2>&1 || true
if ! grep -q "PROBE_RESULT 200" "$WORK/probe-right.log"; then
  echo "EP-025 M4: FAIL - correct PJSIP password did not produce real 200" >&2
  cat "$WORK/probe-right.log" >&2
  exit 1
fi
echo "EP-025 M4: correct PJSIP credential accepted (real 200)" | tee -a "$LOG"

# Vacuity guard 4: wrong ARI credential -> truthful auth failure
# (asterisk-diag must NOT report AVAILABLE). The credential is
# REMOVED entirely: the diag defaults to an empty password, which the
# real Asterisk digest challenge rejects (401 -> AUTHORIZATION). No
# literal secret appears in this script.
if env -u NEXUS_ARI_PASSWORD "$DIAG" status >"$WORK/diag-badari.log" 2>&1; then
  echo "EP-025 M4: FAIL - asterisk-diag succeeded with wrong ARI credential" >&2
  cat "$WORK/diag-badari.log" >&2
  exit 1
fi
if grep -q "provider: AVAILABLE" "$WORK/diag-badari.log"; then
  echo "EP-025 M4: FAIL - wrong ARI credential reported AVAILABLE" >&2
  exit 1
fi
echo "EP-025 M4: wrong ARI credential truthful failure (diag not AVAILABLE)" | tee -a "$LOG"

# Vacuity guard 5: Asterisk unavailable -> truthful UNAVAILABLE.
/usr/bin/docker stop "$CONTAINER" >/dev/null 2>&1
if "$DIAG" status >"$WORK/diag-down.log" 2>&1; then
  echo "EP-025 M4: FAIL - asterisk-diag succeeded while provider stopped" >&2
  cat "$WORK/diag-down.log" >&2
  /usr/bin/docker start "$CONTAINER" >/dev/null 2>&1 || true
  exit 1
fi
if grep -q "provider: AVAILABLE" "$WORK/diag-down.log"; then
  echo "EP-025 M4: FAIL - stopped provider reported AVAILABLE" >&2
  /usr/bin/docker start "$CONTAINER" >/dev/null 2>&1 || true
  exit 1
fi
echo "EP-025 M4: stopped provider truthful UNAVAILABLE" | tee -a "$LOG"
/usr/bin/docker start "$CONTAINER" >/dev/null 2>&1

# Wait for real health + full re-registration after the stop/start.
python3 - "$ENV_FILE" <<'PY'
import os, sys, time, urllib.request, base64
env = {}
with open(sys.argv[1]) as f:
    for line in f:
        k, _, v = line.strip().partition("=")
        env[k] = v
token = base64.b64encode(f"{env['NEXUS_ARI_USER']}:{env['NEXUS_ARI_PASSWORD']}".encode()).decode()
deadline = time.time() + 120
while time.time() < deadline:
    try:
        req = urllib.request.Request(env["NEXUS_ARI_URL"] + "/ari/asterisk/info",
                                     headers={"Authorization": f"Basic {token}"})
        with urllib.request.urlopen(req, timeout=3) as resp:
            if resp.status == 200:
                print("ari-healthy: ok")
                sys.exit(0)
    except Exception:
        time.sleep(2)
print("ari-healthy: FAIL", file=sys.stderr)
sys.exit(1)
PY
reg_ok=0
i=0
while [ $i -lt 90 ]; do
  ok=1
  for ep in endpoint-a endpoint-b endpoint-c endpoint-d endpoint-r endpoint-s endpoint-t endpoint-u; do
    n=$(aor_ok "$ep")
    [ "$n" -eq 1 ] || ok=0
  done
  if [ "$ok" = 1 ]; then
    reg_ok=1
    break
  fi
  sleep 1
  i=$((i + 1))
done
[ "$reg_ok" = 1 ] || { echo "EP-025 M4: FAIL - re-registration after stop/start" >&2; exit 1; }
echo "EP-025 M4: fixture re-registered after stop/start" | tee -a "$LOG"

# Start the real ARI events observer.
: > "$EVENTS"
python3 infra/asterisk/fixture/ari_observer.py "$ENV_FILE" "$EVENTS" >"$WORK/ari-observer.log" 2>&1 &
OBSERVER_PID=$!
observer_ready=0
i=0
while [ $i -lt 20 ]; do
  if grep -q "OBSERVER: READY" "$WORK/ari-observer.log" 2>/dev/null; then
    observer_ready=1
    break
  fi
  sleep 1
  i=$((i + 1))
done
[ "$observer_ready" = 1 ] || {
  echo "EP-025 M4: FAIL - ARI observer did not connect" >&2
  tail -20 "$WORK/ari-observer.log" >&2
  exit 1
}
echo "EP-025 M4: ARI observer READY" | tee -a "$LOG"

# Real RTP wire capture: docker0 for the bridge RTP range PLUS the
# controlled responder RTP ports (12060 = silent peer s, 12070 =
# sender peer t for mid-call loss, 12120 = sender peer u for the
# one-way proof).
tcpdump -i docker0 -U -w "$PCAP" \
  'udp and ((portrange 10000-10099) or (port 12060) or (port 12070) or (port 12120))' \
  >"$WORK/tcpdump.log" 2>&1 &
TCPDUMP_PID=$!
sleep 1

# Snapshot state BEFORE the media tests: clear all decoded-audio
# captures from prior runs so the post-restart media proof is scoped
# to THIS run (the restart test's new canary call is the only large
# two-way capture).
rm -f "$NEXUS_EP025_AUDIO_A_DIR"/dump-*.wav "$NEXUS_EP025_AUDIO_B_DIR"/dump-*.wav 2>/dev/null || true

# Vacuity guard 6: the live suite has real tests (non-zero).
cargo test --locked -p nexus-asterisk --test ep025_failure_live -- --list 2>/dev/null | grep -c "ep025_live_" >"$WORK/live-count.txt" || true
count=$(cat "$WORK/live-count.txt")
[ "$count" -ge 10 ] || {
  echo "EP-025 M4: FAIL - live failure suite empty (count=$count)" >&2
  exit 1
}

# Vacuity guard 7: contract + transport failure suites (delegated M4
# workstreams) must pass: real assertions, exact counts.
if ! cargo test --locked -p nexus-telephony --test ep025_failure_contract >>"$LOG" 2>&1; then
  echo "EP-025 M4: FAIL - contract failure suite" >&2
  tail -40 "$LOG" >&2
  exit 1
fi
grep -qE 'test result: ok\. [1-9][0-9]* passed' "$LOG" || { echo "EP-025 M4: FAIL - contract suite empty" >&2; exit 1; }
if ! cargo test --locked -p nexus-asterisk --test ep025_failure_transport >>"$LOG" 2>&1; then
  echo "EP-025 M4: FAIL - transport failure suite" >&2
  tail -40 "$LOG" >&2
  exit 1
fi
grep -qE 'test result: ok\. [1-9][0-9]* passed' "$LOG" || { echo "EP-025 M4: FAIL - transport suite empty" >&2; exit 1; }

# Vacuity guard 8: nexus-asterisk + nexus-telephony lib batteries.
if ! cargo test --locked -p nexus-asterisk --lib >>"$LOG" 2>&1; then
  echo "EP-025 M4: FAIL - nexus-asterisk lib tests" >&2
  tail -40 "$LOG" >&2
  exit 1
fi
if ! cargo test --locked -p nexus-telephony --lib >>"$LOG" 2>&1; then
  echo "EP-025 M4: FAIL - nexus-telephony lib tests" >&2
  tail -40 "$LOG" >&2
  exit 1
fi

# Run the REAL live-stack failure suite (--ignored), SERIALLY: the
# tests share ONE real Asterisk container (restart tests tear down
# and rebuild fixture state). All fixture vars are already exported.
if ! cargo test --locked -p nexus-asterisk --test ep025_failure_live -- --ignored --test-threads=1 >>"$LOG" 2>&1; then
  echo "EP-025 M4: FAIL - live failure suite" >&2
  tail -80 "$LOG" >&2
  exit 1
fi

# Stop the wire capture.
kill "$TCPDUMP_PID" 2>/dev/null || true
wait "$TCPDUMP_PID" 2>/dev/null || true

# Vacuity guard 9: the live suite actually ran (non-zero passing).
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$LOG"; then
  echo "EP-025 M4: FAIL - live suite reported no passing tests" >&2
  tail -40 "$LOG" >&2
  exit 1
fi
grep -qE 'ep025_live_rejected_603_typed_rejected.*ok' "$LOG" || { echo "EP-025 M4: FAIL - REJECTED proof did not run" >&2; exit 1; }
grep -qE 'ep025_live_busy_486_typed_busy.*ok' "$LOG" || { echo "EP-025 M4: FAIL - BUSY proof did not run" >&2; exit 1; }
grep -qE 'ep025_live_no_answer_bounded_provider_timeout.*ok' "$LOG" || { echo "EP-025 M4: FAIL - NO_ANSWER proof did not run" >&2; exit 1; }

# Vacuity guard 10: one-way media wire proof. The DEDICATED sender u
# (RTP source port 12120) transmits real PCMU toward the silent peer
# s; the silent peer (port 12060) receives it but sends NOTHING back.
# Stream identity: the sender's OWN source port (12120) must appear on
# the wire (the intended sender really transmitted), media must reach
# the silent peer (dst 12060 >= locked minimum), and the silent peer
# must never transmit (src 12060 == 0). endpoint-t's 12070 stream
# (mid-call-loss proof) CANNOT satisfy this guard.
from_u=$(tcpdump -r "$PCAP" -c 50 'src port 12120' 2>/dev/null | wc -l)
[ "$from_u" -ge 50 ] || {
  echo "EP-025 M4: FAIL - one-way sender u did not transmit (n=$from_u)" >&2
  exit 1
}
to_s=$(tcpdump -r "$PCAP" -c 50 'dst port 12060' 2>/dev/null | wc -l)
[ "$to_s" -ge 50 ] || {
  echo "EP-025 M4: FAIL - no RTP captured toward silent peer (n=$to_s)" >&2
  exit 1
}
from_s=$(tcpdump -r "$PCAP" 'src port 12060' 2>/dev/null | wc -l)
[ "$from_s" -eq 0 ] || {
  echo "EP-025 M4: FAIL - silent peer transmitted RTP back (one-way violated, n=$from_s)" >&2
  exit 1
}
echo "EP-025 M4: one-way media proof ok (sender-u RTP -> silent peer; none back)" | tee -a "$LOG"

# Vacuity guard 11: mid-call media loss wire proof. The sender peer t
# (DEDICATED to this proof; its RTP source port 12070 is its stream
# identity) transmitted real RTP then went silent; the call stayed
# bridged (asserted in Rust). Require sender packets AND that the
# sender's LAST packet precedes the suite end (window ended). The
# one-way proof's sender-u uses port 12120, so endpoint-u RTP CANNOT
# satisfy this guard.
from_t=$(tcpdump -r "$PCAP" -c 100 'src port 12070' 2>/dev/null | wc -l)
[ "$from_t" -ge 100 ] || {
  echo "EP-025 M4: FAIL - sender peer sent no RTP (n=$from_t)" >&2
  exit 1
}
last_t=$(tcpdump -r "$PCAP" -tt 'src port 12070' 2>/dev/null | tail -1 | awk '{print $1}' || true)
now_epoch=$(date +%s)
if [ -n "$last_t" ]; then
  last_int=$(printf '%.0f' "$last_t" 2>/dev/null || echo 0)
  gap=$((now_epoch - last_int))
  [ "$gap" -ge 3 ] || {
    echo "EP-025 M4: FAIL - sender window did not end before suite end (last=$last_int now=$now_epoch)" >&2
    exit 1
  }
fi
echo "EP-025 M4: mid-call media loss proof ok (sender window bounded)" | tee -a "$LOG"

# Vacuity guard 12: restart media proof - the post-restart new call
# produced real two-way audio again (largest dec captures on both
# sides, as in M3).
a_dec=$(ls -S "$NEXUS_EP025_AUDIO_A_DIR"/dump-*-dec.wav 2>/dev/null | head -1 || true)
b_dec=$(ls -S "$NEXUS_EP025_AUDIO_B_DIR"/dump-*-dec.wav 2>/dev/null | head -1 || true)
[ -n "$a_dec" ] && [ -n "$b_dec" ] || {
  echo "EP-025 M4: FAIL - post-restart media captures missing (a=$a_dec b=$b_dec)" >&2
  exit 1
}
sz_a=$(stat -c %s "$a_dec" 2>/dev/null || echo 0)
sz_b=$(stat -c %s "$b_dec" 2>/dev/null || echo 0)
[ "$sz_a" -gt 10000 ] && [ "$sz_b" -gt 10000 ] || {
  echo "EP-025 M4: FAIL - post-restart media captures trivial (a=$sz_a b=$sz_b)" >&2
  exit 1
}
echo "EP-025 M4: post-restart two-way media proof ok (A=$sz_a B=$sz_b bytes)" | tee -a "$LOG"

# Vacuity guard 13: zero-orphan teardown (channels + bridges gone).
left=$(/usr/bin/docker exec "$CONTAINER" /usr/sbin/asterisk -rx "core show channels" 2>/dev/null | awk '/active channels/ {print $1}' | head -1)
left=${left:-1}
bridges_left=$(/usr/bin/docker exec "$CONTAINER" /usr/sbin/asterisk -rx "bridge show all" 2>/dev/null | grep -c "Mixing bridge" || true)
[ "$left" = "0" ] && [ "$bridges_left" = "0" ] || {
  echo "EP-025 M4: FAIL - orphans remain (channels=$left bridges=$bridges_left)" >&2
  exit 1
}

# Vacuity guard 14: no credentials/secrets in ANY log artifact.
# Scan the gate's log plus every produced log/event/capture artifact.
# The fixture CONFIG state under $WORK (etc-asterisk/pjsip.conf,
# ari.conf, baresip-*/accounts) legitimately contains the credentials
# the fixtures must use to authenticate (they are inputs, not logs);
# scanning them would always trip. M3's redaction guard has the same
# scope ($LOG + observer log). The artifacts that could leak are the
# logs, the ARI event stream, and the pcap.
leak=0
for secret in "$NEXUS_ARI_PASSWORD" "$NEXUS_SIP_A_PASSWORD" "$NEXUS_SIP_B_PASSWORD" \
              "$NEXUS_SIP_C_PASSWORD" "$NEXUS_SIP_D_PASSWORD" "$NEXUS_SIP_X_PASSWORD" \
              "$NEXUS_SIP_R_PASSWORD" "$NEXUS_SIP_S_PASSWORD" "$NEXUS_SIP_T_PASSWORD" \
              "$NEXUS_SIP_U_PASSWORD"; do
  if grep -rq "$secret" "$LOG" "$WORK"/*.log "$WORK"/ari-events.jsonl "$PCAP" 2>/dev/null; then
    echo "EP-025 M4: FAIL - credential leaked into artifacts" >&2
    leak=1
  fi
done
[ "$leak" = 0 ] || exit 1
echo "EP-025 M4: redaction ok (zero credential canaries in logs/artifacts)" | tee -a "$LOG"

tail -8 "$LOG"
echo "EP-025 M4: ok"
