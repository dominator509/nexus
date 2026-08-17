#!/usr/bin/env sh
# EP-023 M5 gate: live-fire, operations, and node closure (SPEC-021;
# M5 directive).
#
# Proves the visitor-response contract end to end:
#
#   1. pure-contract E2E (no stack): stream refs never claim verified
#      without evidence, two-way audio fails closed without
#      certification, identity is advisory-only, Roku ladder fails
#      closed truthfully (nexus-vision-e2e, nexus-roku-home)
#   2. LF-008 live-fire: REAL person photograph -> mediamtx -> go2rtc
#      -> Frigate cpu detector -> real person detection event ->
#      production adapter -> VisitorEvent -> notification decision ->
#      two-way audio stays NOT certified (scripts/live-fire/LF-008.sh
#      starts the pinned stack, polls for a genuine person event, runs
#      the journey test with --ignored, writes machine-readable
#      evidence, tears down with zero orphans)
#   3. vacuity guards: cargo test <filter> exits 0 on a zero-match
#      filter (EP-001 gate-masking class), so each phase proves real
#      tests ran and passed
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep023-m5-tests.log"
: > "$log"

run_cargo() {
  pkg="$1"
  filter="$2"
  shift 2
  libtest=""
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --ignored) libtest="--ignored"; shift;;
      *) echo "EP-023 M5: FAIL - unexpected arg: $1" >&2; exit 1;;
    esac
  done
  phase_log="/tmp/ep023-m5-${pkg}-${filter}.log"
  # shellcheck disable=SC2086
  if ! cargo test --locked -p "$pkg" "$filter" -- $libtest >>"$phase_log" 2>&1; then
    echo "EP-023 M5: FAIL - cargo -p $pkg '$filter' failed" >&2
    tail -60 "$phase_log" >&2
    exit 1
  fi
  if ! grep -qE 'running [1-9][0-9]* test' "$phase_log"; then
    echo "EP-023 M5: FAIL - no tests matched filter '$filter' (vacuity guard)" >&2
    tail -10 "$phase_log" >&2
    exit 1
  fi
  if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$phase_log"; then
    echo "EP-023 M5: FAIL - no passing tests for filter '$filter'" >&2
    tail -30 "$phase_log" >&2
    exit 1
  fi
  tail -3 "$phase_log" | tee -a "$log"
}

echo "== phase 1: pure-contract E2E + Roku ladder (no stack) =="
run_cargo "nexus-vision-e2e" "ep023_e2e_stream_ref_never_claims_verified_without_evidence"
run_cargo "nexus-vision-e2e" "ep023_e2e_two_way_audio_fails_closed_without_certification"
run_cargo "nexus-vision-e2e" "ep023_e2e_visitor_identity_advisory_only"
run_cargo "nexus-vision-e2e" "ep023_e2e_roku_ladder_fails_closed"
run_cargo "nexus-roku-home" "ep023_unit_roku"

echo "== phase 2: LF-008 live-fire (REAL person event) =="
if ! sh scripts/live-fire/LF-008.sh >>"$log" 2>&1; then
  echo "EP-023 M5: FAIL - LF-008 live-fire failed" >&2
  tail -60 "$log" >&2
  exit 1
fi
if ! grep -q "LF-008: ok" "$log"; then
  echo "EP-023 M5: FAIL - LF-008 sentinel missing (vacuity guard)" >&2
  tail -20 "$log" >&2
  exit 1
fi

tail -8 "$log"
echo "EP-023 M5: ok"
