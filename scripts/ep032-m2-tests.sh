#!/usr/bin/env sh
# EP-032 M2 gate: run the push connector real suite through the REAL
# cargo test machinery with vacuity guards, plus the M1 regression.
#
# The M2 changed-file fence is connectors/push/ plus the node script
# and plan files, so the authoritative gate is the push connector
# cargo suite (unit + real-socket transport) plus the M1 contract
# regression. Vacuity guards are required: `cargo test <filter>` exits
# 0 on a zero-match filter (EP-001 gate-masking class), so a green M2
# must observe a real non-zero passing count, an EP-032-owned test
# name, and zero ignored/filtered tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep032-m2-tests.log"
: > "$log"

fail() {
  echo "EP-032 M2 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-032 M2 gate: $1"; }

# Vacuity guard 0: the push connector must exist.
if [ ! -f connectors/push/Cargo.toml ]; then
  fail "connectors/push/Cargo.toml missing"
fi

# Vacuity guard 0b: the owned production sources must exist.
for f in src/lib.rs src/transport.rs src/adapter.rs; do
  if [ ! -f "connectors/push/$f" ]; then
    fail "connectors/push/$f missing"
  fi
done
ok "push connector crate and sources present"

# Real build + full push connector suite (all targets).
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-push-connector --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-push-connector --all-targets failed" "$log"
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

# Vacuity guard 3 (anti-masking): an EP-032-owned push test must be
# observed. This fails if the gate accidentally executes only a prior
# node's tests.
if ! grep -q 'ep032_unit_push_provider_delivered_receipt_with_correlation .* ok' "$log"; then
  fail "EP-032-owned push test did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 4: the real-socket transport roundtrip must have run
# (real std::net duplex, not a mocked transport).
if ! grep -q 'ep032_unit_push_transport_roundtrip_over_real_duplex .* ok' "$log"; then
  fail "real-socket push transport roundtrip did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 5: no required test was ignored or filtered out.
if grep -qE 'test result: ok\. [0-9]+ passed; [0-9]+ ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi
if grep -q 'filtered out' "$log" && ! grep -q '0 filtered out' "$log"; then
  fail "required tests were filtered (vacuity guard)" "$log"
fi
ok "real suite passed ($(grep -oE 'test result: ok\. [0-9]+ passed' "$log" | awk '{s+=$4} END {print s}') tests total)"

# M1 regression: the contract crate must still be green.
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-notifications --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-notifications --all-targets failed (M1 regression)" "$log"
fi
if ! grep -q 'ep032_unit_envelope_constructs_valid .* ok' "$log"; then
  fail "M1 contract test did not run (regression guard)" "$log"
fi
ok "M1 contract regression green"

# Milestone artifact/fence checks: M2 fence paths exist.
for f in .agent/milestone-files/EP-032-M2.txt connectors/push/Cargo.toml; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-032 M2: ok"
