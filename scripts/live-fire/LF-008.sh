#!/usr/bin/env sh
# LF-008 visitor-response (EP-023 M5 live-fire).
#
# Real cross-node composition of the production components:
# nexus-vision (CameraEvent -> VisitorEvent, identity, two-way audio,
# notification decision), nexus-frigate (REAL Frigate 0.17.2 adapter +
# transport), nexus-roku-home (real fail-closed Roku ladder).
#
# Proves the node contract acceptance obligations with a REAL person
# event (never a canned fixture):
# - a REAL person photograph is streamed through mediamtx -> go2rtc ->
#   Frigate cpu detector; the gate polls /api/events until a genuine
#   person detection appears (real evidence, bounded, honest failure);
# - the E2E journey maps that real CameraEvent through the production
#   adapter into VisitorEvent, computes the deterministic
#   notification-target decision, and proves two-way audio stays
#   NOT certified (fails closed, never fabricated);
# - stream refs stay Unverified; Roku reports UNAVAILABLE truthfully;
# - machine-readable evidence is written to
#   .agent/state/evidence/EP-023-M5-LF-008-visitor-response.json.
#
# Vacuity guards required: cargo test <filter> exits 0 on a zero-match
# filter (EP-001 gate-masking class); the real person-event poll must
# observe an actual detection, not assume one.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/lf008-e2e.log"
: > "$log"
export EVIDENCE_DIR="$(pwd)/.agent/state/evidence"

FRIGATE_IMAGE="ghcr.io/blakeblackshear/frigate:0.17.2"
FRIGATE_DIGEST="sha256:d4351369984d4a9e2a49ac59736f6490856a7ea11f7790040746d21496967010"
MEDIAMTX_BIN="${MEDIAMTX_BIN:-/root/.cache/mediamtx/mediamtx}"
MEDIAMTX_SHA256="25947caac403f37ec881c9be213af2cad67e344a6c7098905b0d31c17f40e336"
PERSON_IMAGE="infra/frigate/fixtures/person-einstein.jpg"
FRIGATE_API_PORT="${FRIGATE_API_PORT:-5000}"
GO2RTC_HOST_PORT="${GO2RTC_HOST_PORT:-8555}"
# mediamtx MUST stay on 8554: infra/frigate/config/{mediamtx.yml,config.yml}
# hardcode rtsp://...:8554 for the real media chain.
MEDIAMTX_PORT="${MEDIAMTX_PORT:-8554}"
CONTAINER="nexus-frigate-lf008"
NETWORK="nexus-ep023-lf008"
WORK="${WORK:-/tmp/ep023-lf008-work}"
FONT="${FONT:-/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf}"

cleanup() {
  set +e
  docker rm -f "$CONTAINER" >/dev/null 2>&1
  docker network rm "$NETWORK" >/dev/null 2>&1
  [ -n "${FFMPEG_PID:-}" ] && kill "$FFMPEG_PID" >/dev/null 2>&1
  [ -n "${MEDIAMTX_PID:-}" ] && kill "$MEDIAMTX_PID" >/dev/null 2>&1
  pkill -f "mediamtx" >/dev/null 2>&1
  pkill -f "person-einstein" >/dev/null 2>&1
  # Bounded wait for the tracked processes to actually exit so the
  # zero-orphan check below is deterministic (SIGTERM is async; under
  # load ffmpeg can linger a moment).
  if [ -n "${FFMPEG_PID:-}" ]; then
    for _i in $(seq 1 25); do
      kill -0 "$FFMPEG_PID" 2>/dev/null || break
      sleep 0.2
    done
  fi
  if [ -n "${MEDIAMTX_PID:-}" ]; then
    for _i in $(seq 1 25); do
      kill -0 "$MEDIAMTX_PID" 2>/dev/null || break
      sleep 0.2
    done
  fi
  rm -rf "$WORK"
  set -e
}
trap cleanup EXIT INT TERM

mkdir -p "$WORK"
cp infra/frigate/config/config.yml "$WORK/config.yml"
cp infra/frigate/config/mediamtx.yml "$WORK/mediamtx.yml"

# ---------- pin checks ----------
echo "== pin checks =="
docker image inspect "$FRIGATE_IMAGE" >/dev/null 2>&1 || docker pull "$FRIGATE_IMAGE" >>"$log" 2>&1
actual_digest=$(docker image inspect "$FRIGATE_IMAGE" 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d[0]["RepoDigests"][0].split("@",1)[-1])' || true)
if [ "$actual_digest" != "$FRIGATE_DIGEST" ]; then
  echo "LF-008: FAIL - Frigate digest mismatch: got $actual_digest want $FRIGATE_DIGEST" >&2
  exit 1
fi
if [ ! -x "$MEDIAMTX_BIN" ]; then
  echo "LF-008: FAIL - mediamtx binary missing at $MEDIAMTX_BIN" >&2
  exit 1
fi
actual_sha=$(sha256sum "$MEDIAMTX_BIN" | awk '{print $1}')
if [ "$actual_sha" != "$MEDIAMTX_SHA256" ]; then
  echo "LF-008: FAIL - mediamtx sha256 mismatch: $actual_sha" >&2
  exit 1
fi
if [ ! -f "$PERSON_IMAGE" ]; then
  echo "LF-008: FAIL - real person fixture missing at $PERSON_IMAGE" >&2
  exit 1
fi
echo "pinned: Frigate ${FRIGATE_DIGEST} mediamtx sha=${actual_sha} person=${PERSON_IMAGE}" | tee -a "$log"

# ---------- start mediamtx (controlled RTSP transport) ----------
echo "== start controlled RTSP server =="
(
  cd "$WORK"
  "$MEDIAMTX_BIN" "$WORK/mediamtx.yml" >"$WORK/mediamtx.log" 2>&1 &
  echo $! >"$WORK/mediamtx.pid"
)
MEDIAMTX_PID=$(cat "$WORK/mediamtx.pid")
sleep 1
if ! kill -0 "$MEDIAMTX_PID" 2>/dev/null; then
  echo "LF-008: FAIL - mediamtx did not start" >&2
  cat "$WORK/mediamtx.log" >&2
  exit 1
fi

# ---------- start REAL person stream (photo -> RTSP -> detect) ----------
# Loop the REAL person photograph with a slow horizontal pan (walking
# pace) so Frigate's motion gate opens and the cpu detector classifies
# a REAL person (probe-verified: detection_fps=13.5, label 'person').
# The gate later polls for an actual person detection event - no canned
# fixture, honest failure if none appears.
echo "== start REAL person stream =="
ffmpeg -hide_banner -loglevel error -re -loop 1 \
  -i "$PERSON_IMAGE" \
  -vf "scale=1920:1080:force_original_aspect_ratio=increase,crop=1920:1080:x='min(3*t*30,1920-1280)':y=0,scale=1280:720,fps=8" \
  -c:v libx264 -preset veryfast -pix_fmt yuv420p -g 8 -tune zerolatency \
  -f rtsp -rtsp_transport tcp "rtsp://127.0.0.1:${MEDIAMTX_PORT}/nexus_front" \
  >"$WORK/ffmpeg.log" 2>&1 &
FFMPEG_PID=$!
sleep 2
if ! kill -0 "$FFMPEG_PID" 2>/dev/null; then
  echo "LF-008: FAIL - ffmpeg person source did not start" >&2
  cat "$WORK/ffmpeg.log" >&2
  exit 1
fi

# ---------- start REAL Frigate container ----------
echo "== start Frigate container =="
docker network create "$NETWORK" >/dev/null 2>&1 || true
docker run -d --name "$CONTAINER" \
  --network "$NETWORK" \
  --add-host host.docker.internal:host-gateway \
  -p "127.0.0.1:${FRIGATE_API_PORT}:5000" \
  -p "127.0.0.1:${GO2RTC_HOST_PORT}:8554" \
  -v "$WORK:/config" \
  -v "/tmp/ep023-lf008-media:/media/frigate" \
  --shm-size=128m \
  --tmpfs /tmp/cache \
  "$FRIGATE_IMAGE@$FRIGATE_DIGEST" >>"$log" 2>&1

echo "== wait for Frigate readiness =="
ready=0
for i in $(seq 1 120); do
  if curl -sf "http://127.0.0.1:${FRIGATE_API_PORT}/api/version" >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 2
done
if [ "$ready" -ne 1 ]; then
  echo "LF-008: FAIL - Frigate API not ready after 240s" >&2
  docker logs "$CONTAINER" 2>&1 | tail -40 >&2
  exit 1
fi
ver=$(curl -s "http://127.0.0.1:${FRIGATE_API_PORT}/api/version")
echo "Frigate API ready: version=$ver" | tee -a "$log"

# Wait for go2rtc producer to attach to nexus_front (real media path).
attached=0
for i in $(seq 1 60); do
  streams=$(curl -s "http://127.0.0.1:${FRIGATE_API_PORT}/api/go2rtc/streams")
  if printf '%s' "$streams" | grep -q '"producers"' && \
     printf '%s' "$streams" | grep -q 'format_name'; then
    attached=1
    break
  fi
  sleep 2
done
if [ "$attached" -ne 1 ]; then
  echo "LF-008: FAIL - go2rtc producer never attached (media path broken)" >&2
  echo "$streams" >&2
  docker logs "$CONTAINER" 2>&1 | tail -40 >&2
  exit 1
fi
echo "go2rtc producer attached: ok" | tee -a "$log"

# Wait for a REAL person detection event. Frigate's cpu detector must
# actually detect the person in the photo; bounded 240s, honest failure.
echo "== wait for REAL person detection event =="
person_seen=0
for i in $(seq 1 120); do
  events=$(curl -s "http://127.0.0.1:${FRIGATE_API_PORT}/api/events?camera=nexus_front&limit=20")
  if printf '%s' "$events" | grep -qi '"label"[[:space:]]*:[[:space:]]*"person"'; then
    person_seen=1
    break
  fi
  sleep 2
done
if [ "$person_seen" -ne 1 ]; then
  echo "LF-008: FAIL - no real person detection event after 240s" >&2
  echo "last events payload:" >&2
  echo "$events" | head -c 2000 >&2
  echo >&2
  docker logs "$CONTAINER" 2>&1 | tail -40 >&2
  exit 1
fi
echo "real person detection observed: ok" | tee -a "$log"

export FRIGATE_BASE_URL="http://127.0.0.1:${FRIGATE_API_PORT}"

# ---------- run the E2E journey FOR REAL (--ignored) ----------
echo "== run E2E visitor-response journey =="
if ! cargo test --locked -p nexus-vision-e2e --test ep023_e2e_visitor_response \
  ep023_e2e_visitor_response_lf008 -- --ignored >>"$log" 2>&1; then
  echo "LF-008: FAIL - nexus-vision-e2e lf008 journey failed" >&2
  tail -60 "$log" >&2
  exit 1
fi

# Vacuity guards (EP-001 gate-masking class): real tests ran and
# passed, and the full journey test is present and green.
if ! grep -qE 'running [1-9][0-9]* test' "$log"; then
  echo "LF-008: FAIL - no E2E tests ran (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  echo "LF-008: FAIL - no passing E2E tests (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi
if ! grep -qE '^test ep023_e2e_visitor_response_lf008 \.\.\. ok$' "$log"; then
  echo "LF-008: FAIL - journey test missing or not ok (vacuity guard)" >&2
  tail -20 "$log" >&2
  exit 1
fi
if [ ! -f .agent/state/evidence/EP-023-M5-LF-008-visitor-response.json ]; then
  echo "LF-008: FAIL - machine-readable evidence file missing" >&2
  exit 1
fi

echo "== teardown =="
cleanup

# Zero-orphan verification.
orphans=$(docker ps -a --filter "name=$CONTAINER" 2>/dev/null | awk 'NR>1 {print $NF}')
if [ -n "$orphans" ]; then
  echo "LF-008: FAIL - container orphan: $orphans" >&2
  exit 1
fi
if pgrep -f "person-einstein" >/dev/null 2>&1; then
  echo "LF-008: FAIL - ffmpeg person source orphan" >&2
  exit 1
fi
if pgrep -f "mediamtx" >/dev/null 2>&1; then
  echo "LF-008: FAIL - mediamtx orphan" >&2
  exit 1
fi
echo "zero-orphan teardown: ok" | tee -a "$log"

tail -6 "$log"
echo "LF-008: ok"
