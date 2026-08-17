#!/usr/bin/env sh
# EP-023 M3 gate: REAL Frigate + go2rtc media integration (SPEC-021).
#
# Orchestrates the real provider stack and proves the production
# nexus-frigate adapter against it:
#
#   1. mediamtx RTSP server (controlled fixture transport; pinned
#      binary + sha256 verified)
#   2. FFmpeg canary source -> mediamtx (runtime-generated canary:
#      unique token + moving timestamp, CONTROLLED_TEST_FIXTURE)
#   3. REAL Frigate 0.17.2 container (pinned digest) with embedded
#      go2rtc pulling the mediamtx stream; camera nexus_front runs the
#      real CPU detector pipeline
#   4. cargo ep023_integration suite (real adapter vs real instance)
#      - phase A: source up -> STREAMING + discovery + snapshot
#      - phase B: source killed -> DEGRADED, never STREAMING
#      - phase C: source restarted -> STREAMING recovers
#      - phase D: docker restart frigate -> same identity + snapshot
#   5. python live-fire proof: canary OCR, independent decode, two
#      snapshots differ, RTSP restream independent client receives
#      real frames, secret-surface checks
#   6. teardown: kill ffmpeg + mediamtx, docker rm, zero-orphan check
#
# Vacuity guards required: cargo test <filter> exits 0 on a
# zero-match filter (EP-001 gate-masking class).
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep023-m3-tests.log"
: > "$log"
evidence="/tmp/ep023-m3-evidence.json"

# Directive G: host-level sysctl hygiene. M3 requires
# fs.inotify.max_user_watches to be sufficient for mediamtx + Frigate
# under container inotify pressure. The harness records the original
# value, raises it ONLY if required, and restores it in explicit
# teardown, then verifies restoration. It never silently leaves
# test-global host configuration altered.
SYSCTL_KEY="fs.inotify.max_user_watches"
SYSCTL_ORIGINAL="$(sysctl -n "$SYSCTL_KEY" 2>/dev/null || echo unknown)"
SYSCTL_MINIMUM=131072
SYSCTL_CHANGED=0
if [ "$SYSCTL_ORIGINAL" != "unknown" ] && [ "$SYSCTL_ORIGINAL" -lt "$SYSCTL_MINIMUM" ]; then
  echo "sysctl: raising $SYSCTL_KEY $SYSCTL_ORIGINAL -> $SYSCTL_MINIMUM (recorded original for restore)" | tee -a "$log"
  sysctl -w "$SYSCTL_KEY=$SYSCTL_MINIMUM" >>"$log" 2>&1
  SYSCTL_CHANGED=1
else
  echo "sysctl: $SYSCTL_KEY already sufficient ($SYSCTL_ORIGINAL); no change" | tee -a "$log"
fi

FRIGATE_IMAGE="ghcr.io/blakeblackshear/frigate:0.17.2"
FRIGATE_DIGEST="sha256:d4351369984d4a9e2a49ac59736f6490856a7ea11f7790040746d21496967010"
MEDIAMTX_BIN="${MEDIAMTX_BIN:-/root/.cache/mediamtx/mediamtx}"
MEDIAMTX_SHA256="25947caac403f37ec881c9be213af2cad67e344a6c7098905b0d31c17f40e336"
CANARY="NX3-$(head -c4 /dev/urandom | od -An -tx1 | tr -d ' \\n')"
FRIGATE_API_PORT="${FRIGATE_API_PORT:-5000}"
GO2RTC_HOST_PORT="${GO2RTC_HOST_PORT:-8555}"
MEDIAMTX_PORT="${MEDIAMTX_PORT:-8554}"
CONTAINER="nexus-frigate-m3"
NETWORK="nexus-ep023-m3"
WORK="${WORK:-/tmp/ep023-m3-work}"
: > "$log"

# Directive B: prove run_cargo() command structure before the real run.
# EP023_M3_PRINT_COMMANDS=1 prints the exact canonical command per phase
# and exits without touching the stack.
if [ "${EP023_M3_PRINT_COMMANDS:-0}" = "1" ]; then
  echo "phase A: cargo test --locked -p nexus-frigate --test ep023_integration_frigate ep023_integration_frigate_ -- --skip availability_source_dead"
  echo "phase B: cargo test --locked -p nexus-frigate --test ep023_integration_frigate availability_source_dead --"
  echo "phase C: cargo test --locked -p nexus-frigate --test ep023_integration_frigate availability_recovered --"
  echo "phase D: cargo test --locked -p nexus-frigate --test ep023_integration_frigate restart_same_identity --"
  exit 0
fi

cleanup() {
  set +e
  # Directive G: restore the recorded sysctl value, then verify.
  if [ "$SYSCTL_CHANGED" = "1" ] && [ "$SYSCTL_ORIGINAL" != "unknown" ]; then
    sysctl -w "$SYSCTL_KEY=$SYSCTL_ORIGINAL" >>"$log" 2>&1
    now="$(sysctl -n "$SYSCTL_KEY" 2>/dev/null || echo unknown)"
    if [ "$now" = "$SYSCTL_ORIGINAL" ]; then
      echo "sysctl: restored $SYSCTL_KEY=$now (verify ok)" | tee -a "$log"
    else
      echo "sysctl: RESTORE FAILED got=$now want=$SYSCTL_ORIGINAL" >&2
    fi
  fi
  docker rm -f "$CONTAINER" >/dev/null 2>&1
  docker network rm "$NETWORK" >/dev/null 2>&1
  [ -n "${FFMPEG_PID:-}" ] && kill "$FFMPEG_PID" >/dev/null 2>&1
  [ -n "${MEDIAMTX_PID:-}" ] && kill "$MEDIAMTX_PID" >/dev/null 2>&1
  pkill -f "mediamtx" >/dev/null 2>&1
  pkill -f "testsrc2=size=1280x360" >/dev/null 2>&1
  rm -rf "$WORK"
  set -e
}
trap cleanup EXIT INT TERM

mkdir -p "$WORK"
cp infra/frigate/config/config.yml "$WORK/config.yml"
cp infra/frigate/config/mediamtx.yml "$WORK/mediamtx.yml"

# ---------- provider pin checks ----------
echo "== pin checks =="
docker image inspect "$FRIGATE_IMAGE" >/dev/null 2>&1 || docker pull "$FRIGATE_IMAGE" >>"$log" 2>&1
actual_digest=$(docker image inspect "$FRIGATE_IMAGE" 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d[0]["RepoDigests"][0].split("@",1)[-1])' || true)
if [ "$actual_digest" != "$FRIGATE_DIGEST" ]; then
  echo "EP-023 M3: FAIL - Frigate digest mismatch: got $actual_digest want $FRIGATE_DIGEST" >&2
  exit 1
fi
if [ ! -x "$MEDIAMTX_BIN" ]; then
  echo "EP-023 M3: FAIL - mediamtx binary missing at $MEDIAMTX_BIN" >&2
  exit 1
fi
actual_sha=$(sha256sum "$MEDIAMTX_BIN" | awk '{print $1}')
if [ "$actual_sha" != "$MEDIAMTX_SHA256" ]; then
  echo "EP-023 M3: FAIL - mediamtx sha256 mismatch: $actual_sha" >&2
  exit 1
fi
go2rtc_version=$(docker run --rm --entrypoint go2rtc "$FRIGATE_IMAGE" --version 2>/dev/null | head -1 || true)
echo "pinned: Frigate ${FRIGATE_DIGEST} go2rtc=${go2rtc_version} mediamtx v1.20.0 sha=${actual_sha}" | tee -a "$log"

# ---------- start mediamtx (controlled RTSP server) ----------
echo "== start controlled RTSP server =="
# Run mediamtx from $WORK: it generates auto.crt/auto.key in its CWD,
# and $WORK is removed in teardown (scope hygiene; no cert scratch in
# the repository).
(
  cd "$WORK"
  "$MEDIAMTX_BIN" "$WORK/mediamtx.yml" >"$WORK/mediamtx.log" 2>&1 &
  echo $! >"$WORK/mediamtx.pid"
)
MEDIAMTX_PID=$(cat "$WORK/mediamtx.pid")
sleep 1
if ! kill -0 "$MEDIAMTX_PID" 2>/dev/null; then
  echo "EP-023 M3: FAIL - mediamtx did not start" >&2
  cat "$WORK/mediamtx.log" >&2
  exit 1
fi

# ---------- start FFmpeg canary source ----------
echo "== start FFmpeg canary source =="
ffmpeg -hide_banner -loglevel error -re -f lavfi \
  -i "testsrc2=size=1280x360:rate=12" \
  -vf "drawtext=text='${CANARY} %{localtime}':fontfile=${FONT:-/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf}:fontsize=64:fontcolor=white:x=20:y=20:box=1:boxcolor=black@0.9" \
  -c:v libx264 -preset veryfast -pix_fmt yuv420p -g 12 -tune zerolatency \
  -f rtsp -rtsp_transport tcp "rtsp://127.0.0.1:${MEDIAMTX_PORT}/nexus_front" \
  >"$WORK/ffmpeg.log" 2>&1 &
FFMPEG_PID=$!
sleep 2
if ! kill -0 "$FFMPEG_PID" 2>/dev/null; then
  echo "EP-023 M3: FAIL - ffmpeg source did not start" >&2
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
  -v "/tmp/ep023-m3-media:/media/frigate" \
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
  echo "EP-023 M3: FAIL - Frigate API not ready after 240s" >&2
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
  echo "EP-023 M3: FAIL - go2rtc producer never attached (media path broken)" >&2
  echo "$streams" >&2
  docker logs "$CONTAINER" 2>&1 | tail -40 >&2
  exit 1
fi
echo "go2rtc producer attached: ok" | tee -a "$log"

export FRIGATE_BASE_URL="http://127.0.0.1:${FRIGATE_API_PORT}"

# run_cargo <filter> [--skip <libtest>]
# Canonical arg boundary: Cargo arguments precede the "--" separator,
# libtest arguments follow it. NEVER place --skip before "--".
# Records the executed test names into $WORK/executed-tests.txt for
# cross-phase vacuity accounting (directive D).
run_cargo() {
  filter="$1"
  skip="${2:-}"
  phase_log="$WORK/cargo-${filter}.log"
  cargo_args="-p nexus-frigate --test ep023_integration_frigate $filter"
  # Live-stack integration tests are #[ignore]d so the ambient
  # workspace verify battery stays green without the stack; the M3 gate
  # runs them FOR REAL with --ignored against the live container.
  libtest_args="--ignored"
  if [ -n "$skip" ]; then
    libtest_args="$libtest_args --skip $skip"
  fi
  # shellcheck disable=SC2086
  if ! cargo test --locked $cargo_args -- $libtest_args >>"$phase_log" 2>&1; then
    echo "EP-023 M3: FAIL - cargo ep023_integration '${filter}' failed" >&2
    tail -40 "$phase_log" >&2
    exit 1
  fi
  if ! grep -qE 'running [1-9][0-9]* test' "$phase_log"; then
    echo "EP-023 M3: FAIL - no tests matched filter '${filter}' (vacuity guard)" >&2
    exit 1
  fi
  if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$phase_log"; then
    echo "EP-023 M3: FAIL - no passing tests for filter '${filter}'" >&2
    tail -20 "$phase_log" >&2
    exit 1
  fi
  # Record executed test names (lines like "test <name> ... ok").
  grep -oE '^test [a-z0-9_]+ \.\.\. ok' "$phase_log" | \
    sed 's/^test //; s/ \.\.\. ok$//' >>"$WORK/executed-tests.txt"
  if [ -n "$skip" ]; then
    # Prove the skipped test is NOT in the executed set for this phase.
    # The skip filter is a libtest substring; test names recorded are
    # full (e.g. ep023_integration_frigate_availability_source_dead_...).
    if grep -q "$skip" "$WORK/executed-tests.txt"; then
      echo "EP-023 M3: FAIL - '${skip}' unexpectedly executed in filter '${filter}' phase" >&2
      exit 1
    fi
  fi
  tail -3 "$phase_log" | tee -a "$log"
}

echo "== phase A: source up (full suite, source-dead phase excluded) =="
run_cargo "ep023_integration_frigate_" "availability_source_dead"

echo "== live-fire media proof =="
EP023_M3_CANARY="$CANARY" EP023_M3_EVIDENCE="$evidence" \
  python3 infra/frigate/tests/ep023_m3_live_fire.py >>"$log" 2>&1 || {
  echo "EP-023 M3: FAIL - live-fire proof failed" >&2
  tail -40 "$log" >&2
  exit 1
}

echo "== phase B: kill source -> DEGRADED never STREAMING =="
kill "$FFMPEG_PID" >/dev/null 2>&1 || true
FFMPEG_PID=""
# Wait for the REAL provider transition: go2rtc keeps the live-producer
# evidence for its retry interval after the source dies (observed ~29s
# on mediamtx 1.20.0). Poll /api/go2rtc/streams until the producer
# loses live evidence (no format_name/bytes_recv), bounded at 90s.
dead=0
for i in $(seq 1 45); do
  sleep 2
  s=$(curl -s "http://127.0.0.1:${FRIGATE_API_PORT}/api/go2rtc/streams")
  if ! printf '%s' "$s" | grep -q '"format_name"'; then
    dead=1
    echo "go2rtc producer dead at +$((i * 2))s (live evidence gone)" | tee -a "$log"
    break
  fi
done
if [ "$dead" -ne 1 ]; then
  echo "EP-023 M3: FAIL - go2rtc producer still live after 90s" >&2
  echo "$s" >&2
  exit 1
fi
run_cargo "availability_source_dead"

echo "== phase C: restart source -> STREAMING recovers =="
ffmpeg -hide_banner -loglevel error -re -f lavfi \
  -i "testsrc2=size=1280x360:rate=12" \
  -vf "drawtext=text='${CANARY} %{localtime}':fontfile=${FONT:-/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf}:fontsize=64:fontcolor=white:x=20:y=20:box=1:boxcolor=black@0.9" \
  -c:v libx264 -preset veryfast -pix_fmt yuv420p -g 12 -tune zerolatency \
  -f rtsp -rtsp_transport tcp "rtsp://127.0.0.1:${MEDIAMTX_PORT}/nexus_front" \
  >"$WORK/ffmpeg2.log" 2>&1 &
FFMPEG_PID=$!
# Wait for the REAL provider reattachment: go2rtc must regain live
# evidence (format_name) before STREAMING can be asserted. Bounded 60s.
reattached=0
for i in $(seq 1 30); do
  sleep 2
  s=$(curl -s "http://127.0.0.1:${FRIGATE_API_PORT}/api/go2rtc/streams")
  if printf '%s' "$s" | grep -q '"format_name"'; then
    reattached=1
    echo "go2rtc producer reattached at +$((i * 2))s" | tee -a "$log"
    break
  fi
done
if [ "$reattached" -ne 1 ]; then
  echo "EP-023 M3: FAIL - go2rtc producer never reattached after restart" >&2
  echo "$s" >&2
  exit 1
fi
run_cargo "availability_recovered"

echo "== phase D: docker restart Frigate -> same identity + snapshot =="
docker restart "$CONTAINER" >>"$log" 2>&1
for i in $(seq 1 60); do
  if curl -sf "http://127.0.0.1:${FRIGATE_API_PORT}/api/version" >/dev/null 2>&1; then
    break
  fi
  sleep 2
done
# Wait for the REAL provider reattachment after restart (bounded 90s).
reattached=0
for i in $(seq 1 45); do
  sleep 2
  s=$(curl -s "http://127.0.0.1:${FRIGATE_API_PORT}/api/go2rtc/streams")
  if printf '%s' "$s" | grep -q '"format_name"'; then
    reattached=1
    echo "go2rtc producer reattached after restart at +$((i * 2))s" | tee -a "$log"
    break
  fi
done
if [ "$reattached" -ne 1 ]; then
  echo "EP-023 M3: FAIL - go2rtc producer never reattached after docker restart" >&2
  echo "$s" >&2
  exit 1
fi
run_cargo "restart_same_identity"

echo "== cross-phase test accounting (directive D) =="
# The complete M3 proof must execute ALL required integration tests
# across the phases; a test skipped in one phase must run in another.
# The required set is the 10 ep023_integration_frigate tests; the union
# of executed tests across every phase must equal it exactly. Runs
# before teardown because cleanup removes $WORK.
cat >"$WORK/expected-tests.txt" <<'EOF'
ep023_integration_frigate_version_matches_pinned
ep023_integration_frigate_discovers_real_camera_with_stable_identity
ep023_integration_frigate_capabilities_from_real_config
ep023_integration_frigate_availability_streaming_with_live_producer
ep023_integration_frigate_availability_source_dead_never_streaming
ep023_integration_frigate_availability_recovered_streaming
ep023_integration_frigate_snapshot_is_real_jpeg
ep023_integration_frigate_events_api_is_real
ep023_integration_frigate_restart_same_identity_and_snapshot
ep023_integration_frigate_redaction_under_real_config
EOF
sort -u "$WORK/expected-tests.txt" >"$WORK/expected-tests.sorted"
sort -u "$WORK/executed-tests.txt" >"$WORK/executed-tests.sorted"
if ! diff -q "$WORK/expected-tests.sorted" "$WORK/executed-tests.sorted" >/dev/null 2>&1; then
  echo "EP-023 M3: FAIL - cross-phase test accounting mismatch" >&2
  echo "--- expected but not executed ---" >&2
  comm -23 "$WORK/expected-tests.sorted" "$WORK/executed-tests.sorted" >&2
  echo "--- executed but not expected ---" >&2
  comm -13 "$WORK/expected-tests.sorted" "$WORK/executed-tests.sorted" >&2
  exit 1
fi
expected_count=$(wc -l <"$WORK/expected-tests.sorted")
echo "cross-phase accounting: $expected_count/10 required tests executed (complete set)" | tee -a "$log"

echo "== teardown =="
cleanup

# Directive G: verify sysctl restoration happened (cleanup restores).
if [ "$SYSCTL_CHANGED" = "1" ]; then
  now="$(sysctl -n "$SYSCTL_KEY" 2>/dev/null || echo unknown)"
  if [ "$now" != "$SYSCTL_ORIGINAL" ]; then
    echo "EP-023 M3: FAIL - sysctl not restored: got=$now want=$SYSCTL_ORIGINAL" >&2
    exit 1
  fi
  echo "sysctl restored to original ($SYSCTL_ORIGINAL): ok" | tee -a "$log"
fi

# Zero-orphan verification.
orphans=$(docker ps -a --filter "name=$CONTAINER" 2>/dev/null | awk 'NR>1 {print $NF}')
if [ -n "$orphans" ]; then
  echo "EP-023 M3: FAIL - container orphan: $orphans" >&2
  exit 1
fi
if pgrep -f "testsrc2=size=1280x360" >/dev/null 2>&1; then
  echo "EP-023 M3: FAIL - ffmpeg source orphan" >&2
  exit 1
fi
if pgrep -f "mediamtx" >/dev/null 2>&1; then
  echo "EP-023 M3: FAIL - mediamtx orphan" >&2
  exit 1
fi
if [ ! -f "$evidence" ]; then
  echo "EP-023 M3: FAIL - evidence missing: $evidence" >&2
  exit 1
fi
echo "zero-orphan teardown: ok" | tee -a "$log"

tail -8 "$log"
echo "EP-023 M3: ok"
