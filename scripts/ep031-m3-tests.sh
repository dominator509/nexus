#!/usr/bin/env sh
# EP-031 M3 gate: run the nexus-crowdsec-connector adapter suite (unit
# + REAL std::net socket integration) through the REAL cargo test
# machinery with vacuity guards.
#
# The M3 changed-file fence is connectors/crowdsec/, so the
# authoritative gate is the CrowdSec connector cargo suite plus the M1
# contract regression plus the M2 Zeek regression. Vacuity guards are
# required: `cargo test <filter>` exits 0 on a zero-match filter
# (EP-001 gate-masking class), so a green M3 must observe a real
# non-zero passing count, an EP-031-owned CrowdSec test name, the
# real-socket integration test, and zero ignored/filtered tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep031-m3-tests.log"
: > "$log"

fail() {
  echo "EP-031 M3 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-031 M3 gate: $1"; }

# Vacuity guard 0: the adapter crate must exist.
if [ ! -f connectors/crowdsec/Cargo.toml ]; then
  fail "connectors/crowdsec/Cargo.toml missing"
fi

# Vacuity guard 0b: the owned production sources must exist.
for f in src/lib.rs src/transport.rs src/adapter.rs tests/lapi.rs; do
  if [ ! -f "connectors/crowdsec/$f" ]; then
    fail "connectors/crowdsec/$f missing"
  fi
done
ok "CrowdSec adapter crate and sources present"

# Real build + full CrowdSec suite (all targets: unit + real-socket
# integration). Use the real cargo binary directly (the shell alias
# wraps rust-rtk-tee which collapses output).
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-crowdsec-connector --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-crowdsec-connector --all-targets failed" "$log"
fi

# Vacuity guard 1: a non-zero number of tests actually ran.
if ! grep -qE 'running [1-9][0-9]* tests' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi

# Vacuity guard 2: a passing result with a non-zero count and zero
# failures is observed in the run output.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed; 0 failed' "$log"; then
  fail "no passing non-vacuous result (vacuity guard)" "$log"
fi

# Vacuity guard 3 (anti-masking): an EP-031-owned CrowdSec test must
# be observed. This fails if the gate accidentally executes only a
# prior node's tests.
if ! grep -q 'ep031_unit_crowdsec_ban_decision_normalized_to_event .* ok' "$log"; then
  fail "EP-031-owned CrowdSec unit test did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 3b: the REAL std::net socket integration test must
# have run and passed (production transport over real sockets, mocks
# control the peer only).
if ! grep -q 'ep031_integration_crowdsec_lapi_full_login_and_ban_decision_over_real_socket .* ok' "$log"; then
  fail "CrowdSec real-socket integration test did not run (anti-masking guard)" "$log"
fi
if ! grep -q 'ep031_integration_crowdsec_lapi_unreachable_fails_closed .* ok' "$log"; then
  fail "CrowdSec unreachable fail-closed integration test did not run (anti-masking guard)" "$log"
fi
ok "CrowdSec suite passed ($(grep -oE 'test result: ok\. [0-9]+ passed' "$log" | awk '{s+=$4} END {print s}') tests total)"

# M1 regression: the advanced contract crate still passes.
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-sentinel-advanced --all-targets >>"$log" 2>&1; then
  fail "M1 regression (nexus-sentinel-advanced) failed" "$log"
fi
if ! grep -q 'ep031_unit_advanced_dependency_direction .* ok' "$log"; then
  fail "M1 dependency-direction regression did not pass" "$log"
fi
ok "M1 contract regression green"

# M2 regression: the Zeek connector still passes.
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-zeek-connector --all-targets >>"$log" 2>&1; then
  fail "M2 regression (nexus-zeek-connector) failed" "$log"
fi
if ! grep -q 'ep031_unit_zeek_normalizes_notices_to_events .* ok' "$log"; then
  fail "M2 Zeek regression did not pass" "$log"
fi
ok "M2 Zeek regression green"

# Milestone artifact/fence checks: M3 fence paths exist.
for f in .agent/milestone-files/EP-031-M3.txt; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence artifacts present"

echo "EP-031 M3: ok"
