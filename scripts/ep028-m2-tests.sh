#!/usr/bin/env sh
# EP-028 M2 gate: run the nexus-hydra-connector adapter core suite
# through the REAL cargo test machinery with vacuity guards.
#
# The M2 changed-file fence is connectors/hydra/ (adapter core), so the
# authoritative gate is the nexus-hydra-connector cargo suite plus the
# M1 regression, fmt/clippy on the connector. Vacuity guards are
# required: `cargo test <filter>` exits 0 on a zero-match filter
# (EP-001 gate-masking class), so a green M2 must observe a real
# non-zero passing count, the M1 regression, an EP-028-owned sentinel
# test name, and zero ignored/filtered tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep028-m2-tests.log"
: > "$log"

fail() {
  echo "EP-028 M2 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-028 M2 gate: $1"; }

# Vacuity guard 0: the adapter core crate must exist.
if [ ! -f connectors/hydra/Cargo.toml ]; then
  fail "connectors/hydra/Cargo.toml missing"
fi

# Vacuity guard 0b: the owned production sources must exist.
for f in src/lib.rs src/transport.rs src/adapter.rs src/observability.rs; do
  if [ ! -f "connectors/hydra/$f" ]; then
    fail "connectors/hydra/$f missing"
  fi
done
ok "adapter core crate and sources present"

# Real build + full connector suite (all targets).
if ! cargo test --locked -p nexus-hydra-connector --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-hydra-connector --all-targets failed" "$log"
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

# Vacuity guard 3 (anti-masking): an EP-028-owned sentinel test must be
# observed. This fails if the gate accidentally executes only a prior
# node's tests or a zero-match filter.
if ! grep -q 'ep028_unit_governed_action_denied_makes_zero_transport_calls .* ok' "$log"; then
  fail "EP-028-owned sentinel test did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 4: no required test was ignored or filtered out.
if grep -qE 'test result: ok\. [0-9]+ passed; [0-9]+ ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi
ok "real suite passed ($(grep -oE 'test result: ok\. [0-9]+ passed' "$log" | awk '{s+=$4} END {print s}') tests total)"

# M1 regression: the contract crate must still be green.
if ! cargo test --locked -p nexus-hydra --all-targets >>"$log" 2>&1; then
  fail "M1 contract regression failed" "$log"
fi
if ! grep -q 'ep028_unit_dependency_direction .* ok' "$log"; then
  fail "M1 dependency-direction regression did not pass" "$log"
fi
ok "M1 contract regression green"

# Milestone artifact/fence checks: M2 fence path exists.
if [ ! -f .agent/milestone-files/EP-028-M2.txt ]; then
  fail ".agent/milestone-files/EP-028-M2.txt missing"
fi
ok "milestone fence present"

echo "EP-028 M2: ok"
