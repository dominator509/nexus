#!/usr/bin/env sh
# EP-030 M4 gate: run the nexus-adguard-connector forced-failure suite
# through the REAL cargo test machinery with vacuity guards, and assert
# the ops diagnostic fails closed.
#
# The M4 changed-file fence is connectors/adguard-home/ (forced
# failures, abuse cases, and observability), so the authoritative gate
# is the nexus-adguard-connector cargo suite (unit +
# ep030_failure_* real std::net socket tests) plus the M1/M2/M3
# regressions plus the fail-closed diagnostic. Vacuity guards are
# required: `cargo test <filter>` exits 0 on a zero-match filter
# (EP-001 gate-masking class), so a green M4 must observe a real
# non-zero passing count, an EP-030-owned failure sentinel, and zero
# ignored/filtered tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep030-m4-tests.log"
: > "$log"

fail() {
  echo "EP-030 M4 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-030 M4 gate: $1"; }

# Vacuity guard 0: the adapter crate must exist.
if [ ! -f connectors/adguard-home/Cargo.toml ]; then
  fail "connectors/adguard-home/Cargo.toml missing"
fi

# Vacuity guard 0b: the owned production sources must exist.
for f in src/lib.rs src/adapter.rs src/transport.rs src/observability.rs \
         tests/ep030_m4_adguard.rs adguard-diag.sh; do
  if [ ! -f "connectors/adguard-home/$f" ]; then
    fail "connectors/adguard-home/$f missing"
  fi
done
ok "adapter crate, failure suite, and diagnostic present"

# Real build + full adapter suite (all targets: unit + failure).
# Use the real cargo binary directly (the shell alias wraps
# rust-rtk-tee which collapses output).
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-adguard-connector --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-adguard-connector --all-targets failed" "$log"
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

# Vacuity guard 3 (anti-masking): an EP-030-owned failure sentinel must
# be observed. This fails if the gate accidentally executes only a
# prior node's tests.
if ! grep -q 'ep030_failure_refused_port_is_unavailable .* ok' "$log"; then
  fail "EP-030-owned failure sentinel did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 4: no required test was ignored or filtered out.
if grep -qE 'test result: ok\. [0-9]+ passed; [0-9]+ ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi
if grep -q 'filtered out' "$log" && ! grep -q '0 filtered out' "$log"; then
  fail "required tests were filtered (vacuity guard)" "$log"
fi
ok "real suite passed ($(grep -oE 'test result: ok\. [0-9]+ passed' "$log" | awk '{s+=$4} END {print s}') tests total)"

# Ops diagnostic fails closed: probing an unreachable endpoint exits
# non-zero and reports reachable=no; it never reports healthy from
# config existence.
diag_rc=0
sh connectors/adguard-home/adguard-diag.sh "http://127.0.0.1:1" > /tmp/ep030-diag.out 2>&1 || diag_rc=$?
if [ "$diag_rc" -eq 0 ]; then
  fail "adguard-diag.sh reported healthy for an unreachable endpoint (fail-closed violation)"
fi
if ! grep -q "reachable=no" /tmp/ep030-diag.out; then
  fail "adguard-diag.sh did not report reachable=no for an unreachable endpoint"
fi
ok "ops diagnostic fails closed (rc=$diag_rc, reachable=no)"

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

# M3 adapter regression: the nexus-openwrt-connector suite still green.
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-openwrt-connector --all-targets >>"$log" 2>&1; then
  fail "M3 adapter regression failed" "$log"
fi
if ! grep -q 'ep030_integration_containment_lifecycle_over_real_sockets .* ok' "$log"; then
  fail "M3 integration sentinel regression did not pass" "$log"
fi
ok "M3 adapter regression green"

# Milestone artifact/fence checks: M4 fence paths exist.
for f in .agent/milestone-files/EP-030-M4.txt .agent/node-contracts/EP-030.md \
         .agent/execplans/EP-030-sentinel-core-network-and-dns.md; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-030 M4: ok"
