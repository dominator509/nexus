#!/usr/bin/env sh
# EP-031 M2 gate: run the nexus-zeek-connector adapter suite and the
# nexus-sentinel-advanced-e2e contract-composition suite through the
# REAL cargo test machinery with vacuity guards.
#
# The M2 changed-file fence is connectors/zeek/ (adapter crate) plus
# tests/sentinel/advanced/ (contract-composition e2e crate), so the
# authoritative gate is the Zeek connector cargo suite plus the
# advanced e2e crate plus the M1 contract regression. Vacuity guards
# are required: `cargo test <filter>` exits 0 on a zero-match filter
# (EP-001 gate-masking class), so a green M2 must observe a real
# non-zero passing count, an EP-031-owned sentinel test name, and
# zero ignored/filtered tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep031-m2-tests.log"
: > "$log"

fail() {
  echo "EP-031 M2 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-031 M2 gate: $1"; }

# Vacuity guard 0: the adapter crate and e2e crate must exist.
if [ ! -f connectors/zeek/Cargo.toml ]; then
  fail "connectors/zeek/Cargo.toml missing"
fi
if [ ! -f tests/sentinel/advanced/Cargo.toml ]; then
  fail "tests/sentinel/advanced/Cargo.toml missing"
fi

# Vacuity guard 0b: the owned production sources must exist.
for f in src/lib.rs src/transport.rs src/adapter.rs; do
  if [ ! -f "connectors/zeek/$f" ]; then
    fail "connectors/zeek/$f missing"
  fi
done
for f in tests/contract.rs; do
  if [ ! -f "tests/sentinel/advanced/$f" ]; then
    fail "tests/sentinel/advanced/$f missing"
  fi
done
ok "adapter and e2e crate sources present"

# Real build + full Zeek adapter suite (all targets).
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-zeek-connector --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-zeek-connector --all-targets failed" "$log"
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

# Vacuity guard 3 (anti-masking): an EP-031-owned Zeek test must be
# observed. This fails if the gate accidentally executes only a prior
# node's tests.
if ! grep -q 'ep031_unit_zeek_normalizes_notices_to_events .* ok' "$log"; then
  fail "EP-031-owned Zeek test did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 4: no required test was ignored or filtered out.
if grep -qE 'test result: ok\. [0-9]+ passed; [0-9]+ ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi
if grep -q 'filtered out' "$log" && ! grep -q '0 filtered out' "$log"; then
  fail "required tests were filtered (vacuity guard)" "$log"
fi
ok "Zeek adapter suite passed ($(grep -oE 'test result: ok\. [0-9]+ passed' "$log" | awk '{s+=$4} END {print s}') tests total)"

# The advanced contract-composition e2e crate must pass its
# ep031_unit_* contract proofs.
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-sentinel-advanced-e2e --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-sentinel-advanced-e2e --all-targets failed" "$log"
fi
if ! grep -q 'ep031_unit_zeek_live_detection_over_real_socket .* ok' "$log"; then
  fail "advanced e2e Zeek proof did not run (anti-masking guard)" "$log"
fi
if ! grep -q 'ep031_unit_destructive_response_remains_human_controlled .* ok' "$log"; then
  fail "advanced e2e destructive-response proof did not run (anti-masking guard)" "$log"
fi
ok "advanced contract-composition suite passed"

# M1 regression: the advanced contract crate still passes.
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-sentinel-advanced --all-targets >>"$log" 2>&1; then
  fail "M1 regression (nexus-sentinel-advanced) failed" "$log"
fi
if ! grep -q 'ep031_unit_advanced_dependency_direction .* ok' "$log"; then
  fail "M1 dependency-direction regression did not pass" "$log"
fi
ok "M1 contract regression green"

# Milestone artifact/fence checks: M2 fence paths exist.
for f in .agent/milestone-files/EP-031-M2.txt; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence artifacts present"

echo "EP-031 M2: ok"
