#!/usr/bin/env sh
# EP-024 M3 gate: run the REAL appliance integration suite against the
# REAL pinned Home Assistant container through the EP-020-certified
# provider boundary.
#
# The M3 changed-files fence is connectors/appliances/ (Rust crate
# nexus-appliances + CONTROLLED_TEST_FIXTURE config + bootstrap). The
# authoritative gate is the ep024 appliance suite of that crate with
# the REAL pinned HA container (ghcr.io/home-assistant/home-assistant:
# stable@sha256:56690a89...cb42a5, same immutable digest certified by
# EP-020). The vacuity guard is required: `cargo test <filter>` exits 0
# on a zero-match filter (EP-001 gate-masking class).
set -eu
export CI=true
export CARGO_TERM_COLOR=never
export GIT_TERMINAL_PROMPT=0

log="/tmp/ep024-m3-tests.log"
: > "$log"

bootstrap="python3 connectors/appliances/fixture/ha_bootstrap.py"

cleanup() {
  # Real provider teardown + orphan check.
  $bootstrap teardown >>"$log" 2>&1 || true
  if docker ps -aq --filter name=nexus-ep024-ha | grep -q .; then
    echo "EP-024 M3: FAIL - HA container leaked after teardown" >&2
  fi
}
trap cleanup EXIT

# 1. Real provider setup: pinned digest, config mount proof,
#    EP-020-certified auth flow, fixture entities REQUIRED. The
#    bootstrap fails hard if any expected fixture entity is absent
#    (no default-config false green).
$bootstrap start >>"$log" 2>&1 || {
  echo "EP-024 M3: FAIL - HA fixture setup failed (pinned image/config/fixtures)" >&2
  tail -30 "$log" >&2
  exit 1
}

# Load the freshly minted token (never persisted in the tree).
# shellcheck disable=SC1091
. /tmp/ep024-ha.env
export NEXUS_HA_BASE NEXUS_HA_TOKEN NEXUS_HA_CONTAINER

# 2. Unit suite: deterministic adapter rules (test-double zone).
if ! cargo test --locked -p nexus-appliances ep024_unit >>"$log" 2>&1; then
  echo "EP-024 M3: FAIL - appliance unit suite failed" >&2
  tail -30 "$log" >&2
  exit 1
fi

# 3. Real integration suite: read-only probes first, then the single
#    stateful live journey (container restart / offline phases inside).
if ! cargo test --locked -p nexus-appliances --test ep024_integration_appliances \
    ep024_integration_appliances_probe_ -- --ignored >>"$log" 2>&1; then
  echo "EP-024 M3: FAIL - appliance probe suite failed" >&2
  tail -30 "$log" >&2
  exit 1
fi

if ! cargo test --locked -p nexus-appliances --test ep024_integration_appliances \
    ep024_integration_appliances_journey_live -- --ignored --exact >>"$log" 2>&1; then
  echo "EP-024 M3: FAIL - appliance live journey failed" >&2
  tail -30 "$log" >&2
  exit 1
fi

# Vacuity guards: non-zero tests ran, non-zero passed, and the real
# integration binary actually executed (never a zero-match run).
if ! grep -qE 'running [1-9][0-9]* tests' "$log"; then
  echo "EP-024 M3: FAIL - no tests matched (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  echo "EP-024 M3: FAIL - no passing tests (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi
if ! grep -q 'ep024_integration_appliances_journey_live' "$log"; then
  echo "EP-024 M3: FAIL - live journey did not execute (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

# 4. Explicit real-provider teardown, then the orphan check. The live
#    journey deliberately restarts/starts the container for its
#    restart + offline phases, so teardown must happen BEFORE the
#    check; the EXIT trap makes it idempotent.
$bootstrap teardown >>"$log" 2>&1 || true
if docker ps -aq --filter name=nexus-ep024-ha | grep -q .; then
  echo "EP-024 M3: FAIL - HA container still present" >&2
  exit 1
fi

tail -8 "$log"
echo "EP-024 M3: ok"
