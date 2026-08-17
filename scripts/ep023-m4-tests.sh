#!/usr/bin/env sh
# EP-023 M4 gate: REAL forced failures, abuse cases, and observability
# (SPEC-021; M4 directive P/Q).
#
# Orchestrates the real provider stack and proves the production
# nexus-frigate adapter fails safely under dependency, policy, security,
# and resource faults:
#
#   1. mediamtx RTSP server (controlled fixture transport; pinned
#      binary + sha256 verified)
#   2. FFmpeg canary source -> mediamtx (runtime-generated canary,
#      CONTROLLED_TEST_FIXTURE)
#   3. REAL Frigate 0.17.2 container (pinned digest) with embedded
#      go2rtc pulling the mediamtx stream
#   4. cargo ep023_failure_frigate suite
#      - phase A: transport classifiers (silent peer -> Timeout,
#        closed port -> Unavailable, real HTTP 401/403 -> Authorization,
#        404 -> NotFound, 500 -> Unavailable, malformed JSON -> External
#        + counter, schema-invalid -> fail closed), redaction canaries,
#        correlation stability, bounded audit ring, counters, diag
#        status against the healthy provider
#      - phase B: REAL Frigate container STOPPED -> same production
#        operation returns Unavailable; stream truth (never STREAMING
#        without fresh evidence); diag against unavailable provider
#      - phase C: Frigate restarted -> recovery observed
#   5. cross-phase vacuity accounting (directive D): the union of
#      executed tests across all phases must equal the full
#      ep023_failure_frigate suite exactly
#   6. teardown: kill ffmpeg + mediamtx, docker rm, zero-orphan check
#
# Vacuity guards required: cargo test <filter> exits 0 on a
# zero-match filter (EP-001 gate-masking class).
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep023-m4-tests.log"
: > "$log"
evidence="/tmp/ep023-m4-evidence.json"

# Directive G: host-level sysctl hygiene (same contract as M3).
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
CANARY="NX4-$(head -c4 /dev/urandom | od -An -tx1 | tr -d ' \\n')"
FRIGATE_API_PORT="${FRIGATE_API_PORT:-5000}"
GO2RTC_HOST_PORT="${GO2RTC_HOST_PORT:-8555}"
# mediamtx MUST stay on 8554: infra/frigate/config/{mediamtx.yml,config.yml}
# hardcode rtsp://...:8554 for the real media chain (host FFmpeg ->
# mediamtx -> go2rtc -> Frigate detect).
MEDIAMTX_PORT="${MEDIAMTX_PORT:-8554}"
CONTAINER="nexus-frigate-m4"
NETWORK="nexus-ep023-m4"
WORK="${WORK:-/tmp/ep023-m4-work}"
: > "$log"

# Directive B: prove run_cargo() command structure before the real run.
if [ "${EP023_M4_PRINT_COMMANDS:-0}" = "1" ]; then
  echo "phase A: cargo test --locked -p nexus-frigate --test ep023_failure_frigate ep023_failure_frigate_ -- --skip ep023_failure_frigate_provider_stopped_unavailable --skip ep023_failure_frigate_never_streaming_without_fresh_evidence --skip ep023_failure_frigate_recovery_after_provider_restart"
  echo "phase B: cargo test --locked -p nexus-frigate --test ep023_failure_frigate ep023_failure_frigate_provider_stopped_unavailable"
  echo "phase B: cargo test --locked -p nexus-frigate --test ep023_failure_frigate ep023_failure_frigate_never_streaming_without_fresh_evidence"
  echo "phase C: cargo test --locked -p nexus-frigate --test ep023_failure_frigate ep023_failure_frigate_recovery_after_provider_restart"
  exit 0
fi

cleanup() {
  set +e
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
  echo "EP-023 M4: FAIL - Frigate digest mismatch: got $actual_digest want $FRIGATE_DIGEST" >&2
  exit 1
fi
if [ ! -x "$MEDIAMTX_BIN" ]; then
  echo "EP-023 M4: FAIL - mediamtx binary missing at $MEDIAMTX_BIN" >&2
  exit 1
fi
actual_sha=$(sha256sum "$MEDIAMTX_BIN" | awk '{print $1}')
if [ "$actual_sha" != "$MEDIAMTX_SHA256" ]; then
  echo "EP-023 M4: FAIL - mediamtx sha256 mismatch: $actual_sha" >&2
  exit 1
fi
go2rtc_version=$(docker run --rm --entrypoint go2rtc "$FRIGATE_IMAGE" --version 2>/dev/null | head -1 || true)
echo "pinned: Frigate ${FRIGATE_DIGEST} go2rtc=${go2rtc_version} mediamtx v1.20.0 sha=${actual_sha}" | tee -a "$log"

# ---------- start mediamtx (controlled RTSP server) ----------
echo "== start controlled RTSP server =="
(
  cd "$WORK"
  "$MEDIAMTX_BIN" "$WORK/mediamtx.yml" >"$WORK/mediamtx.log" 2>&1 &
  echo $! >"$WORK/mediamtx.pid"
)
MEDIAMTX_PID=$(cat "$WORK/mediamtx.pid")
sleep 1
if ! kill -0 "$MEDIAMTX_PID" 2>/dev/null; then
  echo "EP-023 M4: FAIL - mediamtx did not start" >&2
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
  echo "EP-023 M4: FAIL - ffmpeg source did not start" >&2
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
  -v "/tmp/ep023-m4-media:/media/frigate" \
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
  echo "EP-023 M4: FAIL - Frigate API not ready after 240s" >&2
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
  echo "EP-023 M4: FAIL - go2rtc producer never attached (media path broken)" >&2
  echo "$streams" >&2
  docker logs "$CONTAINER" 2>&1 | tail -40 >&2
  exit 1
fi
echo "go2rtc producer attached: ok" | tee -a "$log"

export FRIGATE_BASE_URL="http://127.0.0.1:${FRIGATE_API_PORT}"

# run_cargo <test-binary> <filter> [--skip <libtest>]...
# Canonical arg boundary: Cargo arguments precede the "--" separator,
# libtest arguments follow it. Records executed test names into
# $WORK/executed-tests.txt for cross-phase vacuity accounting.
run_cargo() {
  test_bin="$1"
  filter="$2"
  shift 2
  phase_log="$WORK/cargo-${filter}.log"
  cargo_args="-p nexus-frigate --test $test_bin $filter"
  libtest_args=""
  skips=""
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --skip)
        libtest_args="$libtest_args --skip $2"
        skips="$skips $2"
        shift 2
        ;;
      *) echo "EP-023 M4: FAIL - unexpected run_cargo arg: $1" >&2; exit 1;;
    esac
  done
  # shellcheck disable=SC2086
  if ! cargo test --locked $cargo_args -- $libtest_args >>"$phase_log" 2>&1; then
    echo "EP-023 M4: FAIL - cargo $test_bin '${filter}' failed" >&2
    tail -60 "$phase_log" >&2
    exit 1
  fi
  if ! grep -qE 'running [1-9][0-9]* test' "$phase_log"; then
    echo "EP-023 M4: FAIL - no tests matched filter '${filter}' (vacuity guard)" >&2
    exit 1
  fi
  if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$phase_log"; then
    echo "EP-023 M4: FAIL - no passing tests for filter '${filter}'" >&2
    tail -30 "$phase_log" >&2
    exit 1
  fi
  grep -oE '^test [a-z0-9_]+ \.\.\. ok' "$phase_log" | \
    sed 's/^test //; s/ \.\.\. ok$//' >>"$WORK/executed-tests.txt"
  # Prove the skipped tests are NOT in the executed set for this phase
  # (a skip that silently runs in the wrong phase must fail the gate).
  for skip in $skips; do
    if grep -qx "$skip" "$WORK/executed-tests.txt"; then
      echo "EP-023 M4: FAIL - '${skip}' unexpectedly executed in filter '${filter}' phase" >&2
      exit 1
    fi
  done
  tail -3 "$phase_log" | tee -a "$log"
}

# Wait for the container to actually stop accepting connections
# (real provider death before phase B assertions).
wait_provider_down() {
  down=0
  for i in $(seq 1 30); do
    sleep 1
    if ! curl -sf --max-time 1 "http://127.0.0.1:${FRIGATE_API_PORT}/api/version" >/dev/null 2>&1; then
      down=1
      break
    fi
  done
  if [ "$down" -ne 1 ]; then
    echo "EP-023 M4: FAIL - provider still answering after stop" >&2
    exit 1
  fi
  echo "provider down confirmed: ok" | tee -a "$log"
}

# Wait for the container to accept connections and go2rtc to reattach.
# Frigate restart can be slower than first boot (pipeline re-init);
# bound generously and dump container logs on failure.
wait_provider_up() {
  up=0
  for i in $(seq 1 150); do
    if curl -sf --max-time 2 "http://127.0.0.1:${FRIGATE_API_PORT}/api/version" >/dev/null 2>&1; then
      up=1
      break
    fi
    sleep 2
  done
  if [ "$up" -ne 1 ]; then
    echo "EP-023 M4: FAIL - provider not up after restart (300s)" >&2
    docker logs "$CONTAINER" 2>&1 | tail -40 >&2
    exit 1
  fi
  attached=0
  for i in $(seq 1 60); do
    sleep 2
    s=$(curl -s "http://127.0.0.1:${FRIGATE_API_PORT}/api/go2rtc/streams")
    if printf '%s' "$s" | grep -q '"format_name"'; then
      attached=1
      break
    fi
  done
  if [ "$attached" -ne 1 ]; then
    echo "EP-023 M4: FAIL - go2rtc producer never reattached after restart (120s)" >&2
    docker logs "$CONTAINER" 2>&1 | tail -40 >&2
    exit 1
  fi
  echo "provider up + go2rtc reattached: ok" | tee -a "$log"
}

echo "== phase A: healthy provider + transport classifiers =="
run_cargo "ep023_failure_frigate" "ep023_failure_frigate_" \
  --skip ep023_failure_frigate_provider_stopped_unavailable \
  --skip ep023_failure_frigate_never_streaming_without_fresh_evidence \
  --skip ep023_failure_frigate_recovery_after_provider_restart

echo "== phase B: REAL provider stopped =="
docker stop "$CONTAINER" >>"$log" 2>&1
wait_provider_down
run_cargo "ep023_failure_frigate" "ep023_failure_frigate_provider_stopped_unavailable"
run_cargo "ep023_failure_frigate" "ep023_failure_frigate_never_streaming_without_fresh_evidence"

echo "== phase C: provider restarted -> recovery =="
docker start "$CONTAINER" >>"$log" 2>&1
wait_provider_up
run_cargo "ep023_failure_frigate" "ep023_failure_frigate_recovery_after_provider_restart"

echo "== cross-phase test accounting (directive D) =="
# The complete M4 proof must execute ALL ep023_failure_frigate tests
# across the phases; the union of executed tests must equal the suite
# exactly. Runs before teardown because cleanup removes $WORK.
cat >"$WORK/expected-tests.txt" <<'EOF'
ep023_failure_frigate_closed_port_connection_failure_unavailable
ep023_failure_frigate_silent_peer_times_out
ep023_failure_frigate_http_401_authorization
ep023_failure_frigate_http_403_authorization
ep023_failure_frigate_http_404_not_found
ep023_failure_frigate_http_500_unavailable
ep023_failure_frigate_malformed_json_external_and_counter
ep023_failure_frigate_schema_invalid_fails_closed
ep023_failure_frigate_redaction_canaries_absent
ep023_failure_frigate_correlation_present_and_stable
ep023_failure_frigate_audit_ring_bounded
ep023_failure_frigate_counters_increment
ep023_failure_frigate_provider_stopped_unavailable
ep023_failure_frigate_never_streaming_without_fresh_evidence
ep023_failure_frigate_diag_status_unavailable
ep023_failure_frigate_diag_redaction
ep023_failure_frigate_diag_status_healthy
ep023_failure_frigate_recovery_after_provider_restart
EOF
sort -u "$WORK/expected-tests.txt" >"$WORK/expected-tests.sorted"
sort -u "$WORK/executed-tests.txt" >"$WORK/executed-tests.sorted"
if ! diff -q "$WORK/expected-tests.sorted" "$WORK/executed-tests.sorted" >/dev/null 2>&1; then
  echo "EP-023 M4: FAIL - cross-phase test accounting mismatch" >&2
  echo "--- expected but not executed ---" >&2
  comm -23 "$WORK/expected-tests.sorted" "$WORK/executed-tests.sorted" >&2
  echo "--- executed but not expected ---" >&2
  comm -13 "$WORK/expected-tests.sorted" "$WORK/executed-tests.sorted" >&2
  exit 1
fi
expected_count=$(wc -l <"$WORK/expected-tests.sorted")
echo "cross-phase accounting: $expected_count/18 required tests executed (complete set)" | tee -a "$log"

echo "== teardown =="
cleanup

# Directive G: verify sysctl restoration happened (cleanup restores).
if [ "$SYSCTL_CHANGED" = "1" ]; then
  now="$(sysctl -n "$SYSCTL_KEY" 2>/dev/null || echo unknown)"
  if [ "$now" != "$SYSCTL_ORIGINAL" ]; then
    echo "EP-023 M4: FAIL - sysctl not restored: got=$now want=$SYSCTL_ORIGINAL" >&2
    exit 1
  fi
  echo "sysctl restored to original ($SYSCTL_ORIGINAL): ok" | tee -a "$log"
fi

# Zero-orphan verification.
orphans=$(docker ps -a --filter "name=$CONTAINER" 2>/dev/null | awk 'NR>1 {print $NF}')
if [ -n "$orphans" ]; then
  echo "EP-023 M4: FAIL - container orphan: $orphans" >&2
  exit 1
fi
if pgrep -f "testsrc2=size=1280x360" >/dev/null 2>&1; then
  echo "EP-023 M4: FAIL - ffmpeg source orphan" >&2
  exit 1
fi
if pgrep -f "mediamtx" >/dev/null 2>&1; then
  echo "EP-023 M4: FAIL - mediamtx orphan" >&2
  exit 1
fi
echo "zero-orphan teardown: ok" | tee -a "$log"

# Evidence summary (real observed sentinels only).
cat >"$evidence" <<EOF
{
  "node": "EP-023",
  "milestone": "M4",
  "phaseA_transport_classifiers": "ok",
  "phaseB_provider_stopped_unavailable": "ok",
  "phaseB_never_streaming_without_fresh_evidence": "ok",
  "phaseC_recovery_after_restart": "ok",
  "cross_phase_accounting": "$expected_count/18",
  "zero_orphan": "ok"
}
EOF
echo "evidence: $evidence" | tee -a "$log"

tail -8 "$log"
echo "EP-023 M4: ok"
