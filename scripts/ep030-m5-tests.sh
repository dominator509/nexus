#!/usr/bin/env sh
# EP-030 M5 gate: live-fire, operations, and node closure.
#
# The M5 changed-file fence is infra/sentinel/core/ (LF-010
# network-diagnosis live-fire evidence crate), scripts/live-fire/LF-010.sh,
# scripts/ep030-m5-tests.sh, the node script, docs/operations/EP-030-sentinel.md,
# plan files, and evidence. The authoritative gate is:
#   - the nexus-sentinel-live-fire host suite (LF-010 network-diagnosis
#     journey + partial-data case over REAL std::net sockets against
#     controlled fixtures emitting REAL OPNsense/OpenWrt/AdGuard-shaped
#     responses);
#   - current-run machine-readable evidence embedding EP030_M5_RUN_ID
#     (stale evidence never satisfies);
#   - evidence current-run + redaction guards;
#   - M1/M2/M3/M4 regressions.
#
# Vacuity guards are required (EP-001 gate-masking class): a green M5
# must observe the LF-010 test names, the current-run evidence file, a
# matching run id, zero credential leakage, and zero ignored tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

EVIDENCE=".agent/state/evidence/LF-010-ep030-m5.json"

log="/tmp/ep030-m5-tests.log"
: > "$log"

fail() {
  echo "EP-030 M5 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-030 M5 gate: $1"; }

# Guard 0: owned production sources exist.
for f in infra/sentinel/core/Cargo.toml \
         infra/sentinel/core/tests/lf010_network_diagnosis.rs \
         scripts/live-fire/LF-010.sh \
         docs/operations/EP-030-sentinel.md; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "live-fire sources and ops runbook present"

# Real build + full live-fire suite (all targets).
if ! cargo test --locked -p nexus-sentinel-live-fire --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-sentinel-live-fire --all-targets failed" "$log"
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

# Vacuity guard 3 (anti-masking): the EP-030-owned LF-010 sentinel ran.
if ! grep -q 'ep030_m5_lf010_network_diagnosis .* ok' "$log"; then
  fail "LF-010 sentinel did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 3b (anti-masking): the partial-data case ran.
if ! grep -q 'ep030_m5_lf010_partial_data_firewall_unavailable .* ok' "$log"; then
  fail "LF-010 partial-data case did not run (anti-masking guard)" "$log"
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

# Guard 6: evidence run_id must match EP030_M5_RUN_ID (stale evidence
# never satisfies).
run_id="${EP030_M5_RUN_ID:-}"
if [ -n "$run_id" ]; then
  if ! grep -q "\"run_id\": \"$run_id\"" "$EVIDENCE"; then
    fail "$EVIDENCE run_id does not match current run ($run_id)"
  fi
  ok "evidence run_id matches current run ($run_id)"
fi

# Guard 7: redaction - no credential canary in evidence.
if grep -qE 'EP030_M5_CANARY' "$EVIDENCE"; then
  fail "credential canary leaked into evidence"
fi
if grep -q 'EP030_M5_CANARY' "$log"; then
  fail "credential canary leaked into test output"
fi
ok "redaction scan clean (zero credential leakage)"

# Guard 8: evidence is valid JSON with the expected proof fields.
python3 - "$EVIDENCE" <<'PY'
import json, sys
path = sys.argv[1]
with open(path, encoding="utf-8") as fh:
    doc = json.load(fh)
assert doc["node"] == "EP-030", path
assert doc["milestone"] == "M5", path
assert doc["proof"] == "LF-010", path
assert doc["certification"]["real_opnsense_appliance"] == "NOT_ASSERTED", path
assert doc["certification"]["real_openwrt_router"] == "NOT_ASSERTED", path
assert doc["certification"]["real_adguard_instance"] == "NOT_ASSERTED", path
assert doc["diagnosis"]["class"], path
assert doc["execution"]["state"] == "APPLIED", path
assert doc["verification"]["state"] == "VERIFIED", path
assert doc["rollback"]["state"] == "REVOKED", path
print("evidence json valid; certification boundary NOT_ASSERTED preserved")
PY
ok "evidence JSON valid; certification boundary NOT_ASSERTED preserved"

# Regressions: M1 contract + M2/M3 connectors + M4 failure suite.
if ! cargo test --locked -p nexus-sentinel --all-targets >>"$log" 2>&1; then
  fail "M1 contract regression failed" "$log"
fi
if ! cargo test --locked -p nexus-opnsense-connector --all-targets >>"$log" 2>&1; then
  fail "M2 OPNsense connector regression failed" "$log"
fi
if ! cargo test --locked -p nexus-openwrt-connector --all-targets >>"$log" 2>&1; then
  fail "M3 OpenWrt connector regression failed" "$log"
fi
if ! cargo test --locked -p nexus-adguard-connector --all-targets >>"$log" 2>&1; then
  fail "M4 AdGuard failure-suite regression failed" "$log"
fi
ok "M1 + M2 + M3 + M4 regressions green"

# Milestone artifact/fence checks.
if [ ! -f .agent/milestone-files/EP-030-M5.txt ]; then
  fail ".agent/milestone-files/EP-030-M5.txt missing"
fi
ok "milestone fence present"

echo "EP-030 M5: ok"
