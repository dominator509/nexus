#!/usr/bin/env sh
# EP-030 M3 gate: run the nexus-openwrt-connector real-socket
# integration suite through the REAL cargo test machinery with vacuity
# guards.
#
# The M3 changed-file fence is connectors/openwrt/ (real dependency and
# transport integration), so the authoritative gate is the
# nexus-openwrt-connector cargo suite (unit + ep030_integration_* real
# std::net socket tests) plus the M1+M2 regressions. Vacuity guards are
# required: `cargo test <filter>` exits 0 on a zero-match filter (EP-001
# gate-masking class), so a green M3 must observe a real non-zero
# passing count, an EP-030-owned integration sentinel, and zero
# ignored/filtered tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep030-m3-tests.log"
: > "$log"

fail() {
  echo "EP-030 M3 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-030 M3 gate: $1"; }

# Vacuity guard 0: the connector crate must exist.
if [ ! -f connectors/openwrt/Cargo.toml ]; then
  fail "connectors/openwrt/Cargo.toml missing"
fi

# Vacuity guard 0b: the owned production sources must exist.
for f in src/lib.rs src/adapter.rs src/transport.rs src/observability.rs \
         tests/ep030_m3_openwrt.rs; do
  if [ ! -f "connectors/openwrt/$f" ]; then
    fail "connectors/openwrt/$f missing"
  fi
done
ok "connector crate and sources present"

# Real build + full connector suite (all targets: unit + integration).
# Use the real cargo binary directly (the shell alias wraps
# rust-rtk-tee which collapses output).
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-openwrt-connector --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-openwrt-connector --all-targets failed" "$log"
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

# Vacuity guard 3 (anti-masking): an EP-030-owned integration sentinel
# must be observed. This fails if the gate accidentally executes only a
# prior node's tests.
if ! grep -q 'ep030_integration_containment_lifecycle_over_real_sockets .* ok' "$log"; then
  fail "EP-030-owned integration sentinel did not run (anti-masking guard)" "$log"
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

# M2 adapter regression: the nexus-opnsense-connector suite still green.
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-opnsense-connector --all-targets >>"$log" 2>&1; then
  fail "M2 adapter regression failed" "$log"
fi
if ! grep -q 'ep030_unit_apply_requires_approved_state_zero_calls_on_denial .* ok' "$log"; then
  fail "M2 adapter sentinel regression did not pass" "$log"
fi
ok "M2 adapter regression green"

# Milestone artifact/fence checks: M3 fence paths exist.
for f in .agent/milestone-files/EP-030-M3.txt .agent/node-contracts/EP-030.md \
         .agent/execplans/EP-030-sentinel-core-network-and-dns.md; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-030 M3: ok"
