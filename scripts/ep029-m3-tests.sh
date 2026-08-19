#!/usr/bin/env sh
# EP-029 M3 gate: run the nexus-social-direct-connector real-socket
# integration suite through the REAL cargo test machinery with vacuity
# guards.
#
# The M3 changed-file fence is connectors/social-direct/ (direct
# official API connector), so the authoritative gate is the
# nexus-social-direct-connector cargo suite (unit + ep029_integration_*
# real std::net socket tests) plus the M1+M2 regressions. Vacuity
# guards are required: `cargo test <filter>` exits 0 on a zero-match
# filter (EP-001 gate-masking class), so a green M3 must observe a
# real non-zero passing count, an EP-029-owned integration sentinel,
# and zero ignored/filtered tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep029-m3-tests.log"
: > "$log"

fail() {
  echo "EP-029 M3 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-029 M3 gate: $1"; }

# Vacuity guard 0: the connector crate must exist.
if [ ! -f connectors/social-direct/Cargo.toml ]; then
  fail "connectors/social-direct/Cargo.toml missing"
fi

# Vacuity guard 0b: the owned production sources must exist.
for f in src/lib.rs src/adapter.rs src/transport.rs \
         tests/ep029_m3_direct.rs; do
  if [ ! -f "connectors/social-direct/$f" ]; then
    fail "connectors/social-direct/$f missing"
  fi
done
ok "connector crate and sources present"

# Real build + full connector suite (all targets: unit + integration).
# Use the real cargo binary directly (the shell alias wraps
# rust-rtk-tee which collapses output).
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --locked -p nexus-social-direct-connector --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-social-direct-connector --all-targets failed" "$log"
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

# Vacuity guard 3 (anti-masking): an EP-029-owned integration sentinel
# test must be observed (real std::net socket proof over the
# documented X API v2 surface).
if ! grep -q 'ep029_integration_adapter_capabilities_and_strategic_gaps .* ok' "$log"; then
  fail "EP-029-owned integration sentinel did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 4: no required test was ignored or filtered out.
if grep -qE 'test result: ok\. [0-9]+ passed; [0-9]+ ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi
if grep -q 'filtered out' "$log" && ! grep -q '0 filtered out' "$log"; then
  fail "required tests were filtered (vacuity guard)" "$log"
fi
ok "real suite passed ($(grep -oE 'test result: ok\. [0-9]+ passed' "$log" | awk '{s+=$4} END {print s}') tests total)"

# M1 + M2 regressions: the contract and Postiz adapter crates stay
# green.
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --locked -p nexus-social --all-targets >>"$log" 2>&1; then
  fail "M1 contract regression failed" "$log"
fi
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --locked -p nexus-postiz-connector --all-targets >>"$log" 2>&1; then
  fail "M2 Postiz adapter regression failed" "$log"
fi
ok "M1 + M2 regressions green"

# Milestone artifact/fence checks: M3 fence paths exist.
for f in .agent/milestone-files/EP-029-M3.txt .agent/node-contracts/EP-029.md \
         .agent/execplans/EP-029-social-command-center.md; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-029 M3: ok"
