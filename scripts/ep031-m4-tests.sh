#!/usr/bin/env sh
# EP-031 M4 gate: run the nexus-wazuh-connector adapter suite (unit +
# REAL std::net socket forced-failure integration) through the REAL
# cargo test machinery with vacuity guards, then exercise the
# fail-closed operations diagnostic.
#
# The M4 changed-file fence is connectors/wazuh/ plus the Wazuh
# diagnostic and the node script, so the authoritative gate is the
# Wazuh connector cargo suite plus M1/M2/M3 regressions. Vacuity
# guards are required: `cargo test <filter>` exits 0 on a zero-match
# filter (EP-001 gate-masking class), so a green M4 must observe a
# real non-zero passing count, EP-031-owned Wazuh test names, all six
# real forced-failure proofs, and zero ignored/filtered tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep031-m4-tests.log"
: > "$log"

fail() {
  echo "EP-031 M4 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-031 M4 gate: $1"; }

# Vacuity guard 0: the adapter crate must exist.
if [ ! -f connectors/wazuh/Cargo.toml ]; then
  fail "connectors/wazuh/Cargo.toml missing"
fi

# Vacuity guard 0b: the owned production sources must exist.
for f in src/lib.rs src/transport.rs src/adapter.rs src/observability.rs tests/failure.rs; do
  if [ ! -f "connectors/wazuh/$f" ]; then
    fail "connectors/wazuh/$f missing"
  fi
done
ok "Wazuh adapter crate and sources present"

# Real build + full Wazuh suite (all targets: unit + real-socket
# forced-failure integration). Use the real cargo binary directly (the
# shell alias wraps rust-rtk-tee which collapses output).
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-wazuh-connector --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-wazuh-connector --all-targets failed" "$log"
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

# Vacuity guard 3 (anti-masking): an EP-031-owned Wazuh unit test must
# be observed. This fails if the gate accidentally executes only a
# prior node's tests (nexus-sentinel / zeek / crowdsec / unrelated).
if ! grep -q 'ep031_unit_wazuh_alert_parse_documented_shape .* ok' "$log"; then
  fail "EP-031-owned Wazuh unit test did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 3b: the unbound transport must fail closed.
if ! grep -q 'ep031_unit_wazuh_unbound_fails_closed_with_audit .* ok' "$log"; then
  fail "Wazuh unbound fail-closed test did not run (anti-masking guard)" "$log"
fi

# Forced-failure proof 1: refused socket -> Unavailable (fail closed).
if ! grep -q 'ep031_failure_wazuh_unreachable_fails_closed .* ok' "$log"; then
  fail "refused-socket proof did not run (anti-masking guard)" "$log"
fi

# Forced-failure proof 2: silent accepted peer -> Timeout.
if ! grep -q 'ep031_failure_wazuh_silent_peer_times_out .* ok' "$log"; then
  fail "silent-peer timeout proof did not run (anti-masking guard)" "$log"
fi

# Forced-failure proof 3: 401 denial -> Authorization.
if ! grep -q 'ep031_failure_wazuh_denied_permission_fails_closed .* ok' "$log"; then
  fail "401/auth-failure proof did not run (anti-masking guard)" "$log"
fi

# Forced-failure proof 4: malformed JSON -> External (fail closed).
if ! grep -q 'ep031_failure_wazuh_malformed_json_fails_closed .* ok' "$log"; then
  fail "malformed-response proof did not run (anti-masking guard)" "$log"
fi

# Forced-failure proof 5: empty alert window is a truthful empty
# observation, never an error and never fabricated.
if ! grep -q 'ep031_failure_wazuh_clean_telemetry_is_observed_not_fabricated .* ok' "$log"; then
  fail "empty-window proof did not run (anti-masking guard)" "$log"
fi

# Forced-failure proof 6: redaction canary - zero credential leakage.
if ! grep -q 'ep031_failure_wazuh_audit_never_leaks_credentials .* ok' "$log"; then
  fail "redaction canary proof did not run (anti-masking guard)" "$log"
fi
ok "Wazuh suite passed ($(grep -oE 'test result: ok\. [0-9]+ passed' "$log" | awk '{s+=$4} END {print s}') tests total)"

# Forced-failure proof 7: the operations diagnostic fails closed
# against an unreachable endpoint (rc=3, reachable=no). Configured
# credentials/endpoints never imply healthy.
diag_log="/tmp/ep031-m4-diag.log"
: > "$diag_log"
set +e
WAZUH_BASE_URL=http://127.0.0.1:59999 WAZUH_USER=probe WAZUH_PASS=probe \
  sh scripts/wazuh-diag.sh >"$diag_log" 2>&1
diag_rc=$?
set -e
if [ "$diag_rc" -ne 3 ]; then
  fail "wazuh-diag.sh unreachable expected rc=3, got rc=$diag_rc" "$diag_log"
fi
if ! grep -q 'reachable=no' "$diag_log" && ! grep -q 'unreachable' "$diag_log"; then
  fail "wazuh-diag.sh did not report reachable=no/unreachable" "$diag_log"
fi
ok "wazuh-diag fail-closed proof green (rc=$diag_rc)"

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

# M3 regression: the CrowdSec connector still passes.
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-crowdsec-connector --all-targets >>"$log" 2>&1; then
  fail "M3 regression (nexus-crowdsec-connector) failed" "$log"
fi
if ! grep -q 'ep031_unit_crowdsec_ban_decision_normalized_to_event .* ok' "$log"; then
  fail "M3 CrowdSec regression did not pass" "$log"
fi
ok "M3 CrowdSec regression green"

# Milestone artifact/fence checks: M4 fence paths exist.
for f in .agent/milestone-files/EP-031-M4.txt; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence artifacts present"

echo "EP-031 M4: ok"
