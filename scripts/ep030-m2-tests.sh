#!/usr/bin/env sh
# EP-030 M2 gate: run the nexus-opnsense-connector adapter suite through
# the REAL cargo test machinery with vacuity guards.
#
# The M2 changed-file fence is connectors/opnsense/ (adapter crate) plus
# the node script and plan files, so the authoritative gate is the
# nexus-opnsense-connector cargo suite plus the M1 contract regression.
# Vacuity guards are required: `cargo test <filter>` exits 0 on a
# zero-match filter (EP-001 gate-masking class), so a green M2 must
# observe a real non-zero passing count, an EP-030-owned sentinel test
# name, and zero ignored/filtered tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep030-m2-tests.log"
: > "$log"

fail() {
  echo "EP-030 M2 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-030 M2 gate: $1"; }

# Vacuity guard 0: the adapter crate must exist.
if [ ! -f connectors/opnsense/Cargo.toml ]; then
  fail "connectors/opnsense/Cargo.toml missing"
fi

# Vacuity guard 0b: the owned production sources must exist.
for f in src/lib.rs src/adapter.rs src/observability.rs src/transport.rs; do
  if [ ! -f "connectors/opnsense/$f" ]; then
    fail "connectors/opnsense/$f missing"
  fi
done
ok "adapter crate and sources present"

# Real build + full adapter suite (all targets). Use the real cargo
# binary directly (the shell alias wraps rust-rtk-tee which collapses
# output).
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-opnsense-connector --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-opnsense-connector --all-targets failed" "$log"
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

# Vacuity guard 3 (anti-masking): an EP-030-owned sentinel test must be
# observed. This fails if the gate accidentally executes only a prior
# node's tests.
if ! grep -q 'ep030_unit_apply_requires_approved_state_zero_calls_on_denial .* ok' "$log"; then
  fail "EP-030-owned sentinel test did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 4: no required test was ignored or filtered out.
if grep -qE 'test result: ok\. [0-9]+ passed; [0-9]+ ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi
if grep -q 'filtered out' "$log" && ! grep -q '0 filtered out' "$log"; then
  fail "required tests were filtered (vacuity guard)" "$log"
fi
ok "real suite passed ($(grep -oE 'test result: ok\. [0-9]+ passed' "$log" | awk '{s+=$4} END {print s}') tests total)"

# M1 contract regression: the nexus-sentinel contract crate still green.
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-sentinel --all-targets >>"$log" 2>&1; then
  fail "M1 contract regression failed" "$log"
fi
if ! grep -q 'ep030_unit_dependency_direction .* ok' "$log"; then
  fail "M1 dependency-direction regression did not pass" "$log"
fi
ok "M1 contract regression green"

# Milestone artifact/fence checks: M2 fence paths exist.
for f in .agent/milestone-files/EP-030-M2.txt .agent/node-contracts/EP-030.md \
         .agent/execplans/EP-030-sentinel-core-network-and-dns.md; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-030 M2: ok"
