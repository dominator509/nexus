#!/usr/bin/env sh
# EP-031 M5 gate: live-fire, operations, and node closure.
#
# The M5 changed-file fence is connectors/osquery/ plus the LF-009
# sentinel-quarantine live-fire evidence crate, scripts/live-fire/
# LF-009.sh, this gate, the node script, and the ops runbook. The
# authoritative gate is:
#   - the nexus-osquery-connector suite (unit + REAL std::net socket
#     forced-failure tests against the production collector server);
#   - the nexus-sentinel-advanced-live-fire LF-009 journey (REAL
#     std::net sockets against controlled fixtures emitting REAL
#     Zeek/CrowdSec/Wazuh/osquery/OPNsense-shaped responses) with the
#     full RAW -> EVENT -> INCIDENT -> TRIAGE -> AUTHORIZED ->
#     EXECUTED -> VERIFIED -> REVOKED state separation and the
#     destructive-never-preauthorized proof;
#   - current-run machine-readable evidence embedding EP031_M5_RUN_ID
#     (stale evidence never satisfies);
#   - evidence current-run + redaction + JSON guards;
#   - M1/M2/M3/M4 regressions.
#
# Vacuity guards are required (EP-001 gate-masking class): a green M5
# must observe the LF-009 test name, the osquery connector tests, the
# destructive-denial proof, the current-run evidence file, a matching
# run id, zero credential leakage, and zero ignored tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

EVIDENCE=".agent/state/evidence/LF-009-ep031-m5.json"

log="/tmp/ep031-m5-tests.log"
: > "$log"

fail() {
  echo "EP-031 M5 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-031 M5 gate: $1"; }

# Guard 0: owned production sources exist.
for f in connectors/osquery/Cargo.toml \
         connectors/osquery/src/lib.rs \
         connectors/osquery/src/transport.rs \
         connectors/osquery/src/adapter.rs \
         connectors/osquery/src/observability.rs \
         connectors/osquery/tests/failure.rs \
         infra/sentinel/advanced-live-fire/Cargo.toml \
         infra/sentinel/advanced-live-fire/tests/lf009_sentinel_quarantine.rs \
         scripts/live-fire/LF-009.sh \
         docs/operations/EP-031-sentinel.md; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "osquery + live-fire sources and ops runbook present"

# Real build + full osquery connector suite (all targets: unit +
# real-socket forced-failure).
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-osquery-connector --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-osquery-connector --all-targets failed" "$log"
fi
if ! grep -q 'ep031_failure_osquery_full_enroll_read_write_lifecycle_over_real_socket .* ok' "$log"; then
  fail "osquery real-socket lifecycle proof did not run (anti-masking guard)" "$log"
fi
if ! grep -q 'ep031_failure_osquery_audit_never_leaks_secret .* ok' "$log"; then
  fail "osquery redaction canary proof did not run (anti-masking guard)" "$log"
fi
ok "osquery connector suite green"

# Real build + full LF-009 live-fire suite. A fresh run id forces
# current-run evidence (stale evidence never satisfies).
run_id="run-$(date +%Y%m%d%H%M%S)-ep031m5"
if ! EP031_M5_RUN_ID="$run_id" "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-sentinel-advanced-live-fire --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-sentinel-advanced-live-fire failed" "$log"
fi

# Vacuity guard 1: a non-zero number of tests actually ran.
if ! grep -qE 'running [1-9][0-9]* test' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi

# Vacuity guard 2: a passing result with a non-zero count and zero
# failures is observed in the run output.
if ! grep -qE 'test result: ok[.] [1-9][0-9]* passed; 0 failed' "$log"; then
  fail "no passing non-vacuous result (vacuity guard)" "$log"
fi

# Vacuity guard 3 (anti-masking): the EP-031-owned LF-009 sentinel ran.
if ! grep -q 'ep031_m5_lf009_sentinel_quarantine .* ok' "$log"; then
  fail "LF-009 sentinel did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 3b (anti-masking): osquery tests ran (not only prior
# nodes' suites).
if ! grep -q 'ep031_unit_osquery_wildcard_listener_normalized_with_audit .* ok' "$log"; then
  fail "osquery unit sentinel did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 4: no required test was ignored or filtered out.
if grep -qE 'test result: ok[.] [0-9]+ passed; [0-9]+ ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi
ok "real live-fire suite passed ($(grep -oE 'test result: ok[.] [0-9]+ passed' "$log" | awk '{s+=$4} END {print s}') tests total)"

# Guard 5: current-run evidence exists.
if [ ! -f "$EVIDENCE" ]; then
  fail "$EVIDENCE missing (current-run evidence required)"
fi
ok "evidence file present"

# Guard 6: evidence run_id must match the current run (stale evidence
# never satisfies).
if ! grep -q "\"run_id\": \"$run_id\"" "$EVIDENCE"; then
  fail "$EVIDENCE run_id does not match current run ($run_id)"
fi
ok "evidence run_id matches current run ($run_id)"

# Guard 7: redaction - no credential canary in evidence or output.
if grep -qE 'EP031_M5_CANARY' "$EVIDENCE"; then
  fail "credential canary leaked into evidence"
fi
if grep -q 'EP031_M5_CANARY' "$log"; then
  fail "credential canary leaked into test output"
fi
ok "redaction scan clean (zero credential leakage)"

# Guard 8: evidence is valid JSON with the expected proof fields and
# the state separation preserved (RAW -> ... -> VERIFIED -> REVOKED;
# destructive never preauthorized; certification boundary honest).
python3 - "$EVIDENCE" <<'PY'
import json, sys
path = sys.argv[1]
with open(path, encoding="utf-8") as fh:
    doc = json.load(fh)
assert doc["node"] == "EP-031", path
assert doc["milestone"] == "M5", path
assert doc["proof"] == "LF-009", path
assert doc["incident"]["confidence"] == "HIGH", path
assert doc["execution"]["state"] == "APPLIED", path
assert doc["verification"]["state"] == "VERIFIED", path
assert doc["rollback"]["state"] == "REVOKED", path
assert doc["response"]["destructive_denied"] is True, path
assert doc["response"]["destructive_never_preauthorized"] is True, path
assert doc["redaction"] == "ZERO_LEAKAGE", path
assert doc["certification"]["real_sensors"] == "NOT_ASSERTED", path
assert doc["certification"]["real_firewall_appliance"] == "NOT_ASSERTED", path
assert len(doc["normalized_facts"]) >= 4, path
print("evidence json valid; state separation + destructive invariant + certification boundary preserved")
PY
ok "evidence JSON valid; journey + certification boundary preserved"

# Regressions: M1 contract + M2/M3/M4 connectors.
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-sentinel-advanced --all-targets >>"$log" 2>&1; then
  fail "M1 contract regression failed" "$log"
fi
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-zeek-connector --all-targets >>"$log" 2>&1; then
  fail "M2 Zeek connector regression failed" "$log"
fi
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-crowdsec-connector --all-targets >>"$log" 2>&1; then
  fail "M3 CrowdSec connector regression failed" "$log"
fi
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-wazuh-connector --all-targets >>"$log" 2>&1; then
  fail "M4 Wazuh failure-suite regression failed" "$log"
fi
ok "M1 + M2 + M3 + M4 regressions green"

# Milestone artifact/fence checks.
for f in .agent/milestone-files/EP-031-M5.txt; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence present"

echo "EP-031 M5: ok"
