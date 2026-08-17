#!/usr/bin/env sh
# EP-024 M5 gate: run the REAL vacuum live-fire + forced-failure suite
# against the REAL pinned Home Assistant container through the
# EP-020-certified provider boundary.
#
# The M5 changed-files fence is connectors/vacuum/ (Rust crate
# nexus-vacuum + CONTROLLED_TEST_FIXTURE config + bootstrap). The
# authoritative gate is the nexus-vacuum unit + failure suites with the
# REAL pinned HA container (ghcr.io/home-assistant/home-assistant:
# stable@sha256:56690a89...cb42a5, same immutable digest certified by
# EP-020). The vacuity guard is required: `cargo test <filter>` exits 0
# on a zero-match filter (EP-001 gate-masking class).
set -eu
export CI=true
export CARGO_TERM_COLOR=never
export GIT_TERMINAL_PROMPT=0

log="/tmp/ep024-m5-tests.log"
: > "$log"

bootstrap="python3 connectors/vacuum/fixture/ha_bootstrap.py"
container="nexus-ep024-vac"

cleanup() {
  # Real provider teardown + orphan check.
  $bootstrap teardown >>"$log" 2>&1 || true
  if docker ps -aq --filter name="$container" | grep -q .; then
    echo "EP-024 M5: FAIL - HA container leaked after teardown" >&2
  fi
}
trap cleanup EXIT

# 1. Real provider setup: pinned digest, config mount proof,
#    EP-020-certified auth flow, fixture entities REQUIRED (vacuum A +
#    vacuum B). The bootstrap fails hard if any expected fixture entity
#    is absent (no default-config false green).
$bootstrap start >>"$log" 2>&1 || {
  echo "EP-024 M5: FAIL - HA fixture setup failed (pinned image/config/fixtures)" >&2
  tail -30 "$log" >&2
  exit 1
}

# Load the freshly minted token (never persisted in the tree).
# shellcheck disable=SC1091
. /tmp/ep024-vac.env
export NEXUS_HA_BASE NEXUS_HA_TOKEN NEXUS_HA_CONTAINER

# 2. Unit suite: deterministic adapter rules (test-double zone),
#    including the Condvar-bounded in-flight idempotency concurrency
#    proof (no hang, no race).
if ! cargo test --locked -p nexus-vacuum ep024_unit >>"$log" 2>&1; then
  echo "EP-024 M5: FAIL - vacuum unit suite failed" >&2
  tail -30 "$log" >&2
  exit 1
fi

# 3. Real forced-failure probe suite: read-only failure mechanisms
#    (bad credential, silent peer TIMEOUT, refused endpoint UNAVAILABLE,
#    malformed response, unknown vacuum, capability discovery from real
#    feature bits, MapReadback-without-map denied, vocabulary,
#    redaction). Safe to run concurrently.
if ! cargo test --locked -p nexus-vacuum --test ep024_failure_vacuum \
    ep024_failure_vacuum_probe_ -- --ignored >>"$log" 2>&1; then
  echo "EP-024 M5: FAIL - vacuum failure probe suite failed" >&2
  tail -30 "$log" >&2
  exit 1
fi

# 4. vacuum-diag: healthy status + recover (bounded) + redaction.
if ! cargo run --locked -p nexus-vacuum --bin vacuum-diag -- \
    status >>"$log" 2>&1; then
  echo "EP-024 M5: FAIL - vacuum-diag status must be healthy against the live provider" >&2
  tail -30 "$log" >&2
  exit 1
fi
if ! cargo run --locked -p nexus-vacuum --bin vacuum-diag -- \
    recover >>"$log" 2>&1; then
  echo "EP-024 M5: FAIL - vacuum-diag recover must succeed (bounded)" >&2
  tail -30 "$log" >&2
  exit 1
fi
if grep -q "$NEXUS_HA_TOKEN" "$log"; then
  echo "EP-024 M5: FAIL - provider token leaked into diagnostic output" >&2
  exit 1
fi

# 5. Sequential live journey: StartClean -> CLEANING, Pause -> PAUSED,
#    ReturnHome -> RETURNING -> DOCKED, Dock -> same action, wrong-target
#    never verifies, retry-not-conflict, correlation, restart stable
#    identity, offline UNAVAILABLE, bounded recovery, restored fresh
#    readback only, zero secret leakage. Owns ALL stateful phases.
if ! cargo test --locked -p nexus-vacuum --test ep024_failure_vacuum \
    ep024_failure_vacuum_journey_live -- --ignored --exact >>"$log" 2>&1; then
  echo "EP-024 M5: FAIL - vacuum live journey failed" >&2
  tail -30 "$log" >&2
  exit 1
fi

# 6. Ops recovery proof at the diagnostic level: provider unavailable
#    -> status must FAIL; provider restored -> status healthy ONLY via
#    fresh readback (no stale cache can produce recovery success).
docker stop "$container" >>"$log" 2>&1
sleep 3
if cargo run --locked -p nexus-vacuum --bin vacuum-diag -- \
    status >>"$log" 2>&1; then
  echo "EP-024 M5: FAIL - diag status must report unavailable while provider is down" >&2
  exit 1
fi
docker start "$container" >>"$log" 2>&1
up=0
deadline=$(($(date +%s) + 300))
while [ "$(date +%s)" -lt "$deadline" ]; do
  code=$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:8126/api/ 2>/dev/null || true)
  if [ "$code" = "200" ] || [ "$code" = "401" ]; then
    up=1
    break
  fi
  sleep 2
done
[ "$up" -eq 1 ] || {
  echo "EP-024 M5: FAIL - HA did not become ready after restore" >&2
  exit 1
}
restored=1
deadline=$(($(date +%s) + 240))
while [ "$(date +%s)" -lt "$deadline" ]; do
  if cargo run --locked -p nexus-vacuum --bin vacuum-diag -- \
      status >>"$log" 2>&1; then
    restored=0
    break
  fi
  sleep 5
done
[ "$restored" -eq 0 ] || {
  echo "EP-024 M5: FAIL - provider did not become healthy after restore (fresh readback only)" >&2
  exit 1
}

# 7. Vacuity guards: non-zero tests ran, non-zero passed, and the real
#    failure binary actually executed (never a zero-match run).
if ! grep -qE 'running [1-9][0-9]* tests' "$log"; then
  echo "EP-024 M5: FAIL - no tests matched (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  echo "EP-024 M5: FAIL - no passing tests (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi
if ! grep -q 'ep024_failure_vacuum_journey_live' "$log"; then
  echo "EP-024 M5: FAIL - live journey did not execute (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi
if ! grep -q 'ep024_failure_vacuum_probe_' "$log"; then
  echo "EP-024 M5: FAIL - failure probe suite did not execute (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

# 8. Explicit real-provider teardown, then the orphan check. The live
#    journey deliberately stops/starts the container for its offline +
#    recovery phases, so teardown must happen BEFORE the check; the
#    EXIT trap makes it idempotent.
$bootstrap teardown >>"$log" 2>&1 || true
if docker ps -aq --filter name="$container" | grep -q .; then
  echo "EP-024 M5: FAIL - HA container still present" >&2
  exit 1
fi

tail -8 "$log"
echo "EP-024 M5: ok"
