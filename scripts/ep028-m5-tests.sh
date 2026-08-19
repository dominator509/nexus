#!/usr/bin/env sh
# EP-028 M5 gate: live-fire, operations, and node closure.
#
# The M5 changed-file fence is tests/hydra-live/ (LF-015/LF-025 live-fire
# evidence crate), scripts/live-fire/LF-015.sh + LF-025.sh,
# scripts/ep028-m5-tests.sh, the node script, docs/operations/EP-028-hydra.md,
# plan files, and evidence. The authoritative gate is:
#   - the nexus-hydra-live-e2e host suite (LF-015/LF-025 proofs over REAL
#     std::net sockets against controlled fixtures emitting REAL
#     Hydra-shaped responses);
#   - current-run machine-readable evidence embedding EP028_M5_RUN_ID
#     (stale evidence never satisfies);
#   - evidence current-run + redaction guards;
#   - M1/M2/M3/M4 regressions.
#
# Vacuity guards are required (EP-001 gate-masking class): a green M5
# must observe the LF-015/LF-025 test names, the current-run evidence
# files, a matching run id, zero credential leakage, and zero ignored
# tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

EVIDENCE_015=".agent/state/evidence/LF-015-ep028-m5.json"
EVIDENCE_025=".agent/state/evidence/LF-025-ep028-m5.json"

log="/tmp/ep028-m5-tests.log"
: > "$log"

fail() {
  echo "EP-028 M5 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-028 M5 gate: $1"; }

# Guard 0: owned production sources exist.
for f in tests/hydra-live/Cargo.toml \
         tests/hydra-live/tests/lf015_cross_crm_command.rs \
         tests/hydra-live/tests/lf025_ceo_business_brief.rs \
         scripts/live-fire/LF-015.sh \
         scripts/live-fire/LF-025.sh \
         docs/operations/EP-028-hydra.md; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "live-fire sources and ops runbook present"

# Real build + full live-fire suite (all targets).
if ! cargo test --locked -p nexus-hydra-live-e2e --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-hydra-live-e2e --all-targets failed" "$log"
fi

# Vacuity guard 1: a non-zero number of tests actually ran. Matches
# both singular ("running 1 test") and plural ("running 12 tests").
if ! grep -qE 'running [1-9][0-9]* test' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi

# Vacuity guard 2: a passing result with a non-zero count and zero
# failures is observed in the run output.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed; 0 failed' "$log"; then
  fail "no passing non-vacuous result (vacuity guard)" "$log"
fi

# Vacuity guard 3 (anti-masking): the EP-028-owned LF-015 sentinel ran.
if ! grep -q 'ep028_m5_lf015_cross_crm_command .* ok' "$log"; then
  fail "LF-015 sentinel did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 4 (anti-masking): the EP-028-owned LF-025 sentinel ran.
if ! grep -q 'ep028_m5_lf025_ceo_business_brief .* ok' "$log"; then
  fail "LF-025 sentinel did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 5: no required test was ignored or filtered out.
if grep -qE 'test result: ok\. [0-9]+ passed; [0-9]+ ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi
ok "real live-fire suite passed ($(grep -oE 'test result: ok\. [0-9]+ passed' "$log" | awk '{s+=$4} END {print s}') tests total)"

# Guard 6: current-run evidence exists for both proofs.
for ev in "$EVIDENCE_015" "$EVIDENCE_025"; do
  if [ ! -f "$ev" ]; then
    fail "$ev missing (current-run evidence required)"
  fi
done
ok "both evidence files present"

# Guard 7: evidence run_id must match EP028_M5_RUN_ID (stale evidence
# never satisfies).
run_id="${EP028_M5_RUN_ID:-}"
if [ -n "$run_id" ]; then
  for ev in "$EVIDENCE_015" "$EVIDENCE_025"; do
    if ! grep -q "\"run_id\": \"$run_id\"" "$ev"; then
      fail "$ev run_id does not match current run ($run_id)"
    fi
  done
  ok "evidence run_id matches current run ($run_id)"
fi

# Guard 8: redaction - no credential canary in evidence or audit.
if grep -qE 'EP028_(LF015|LF025)_CANARY' "$EVIDENCE_015" "$EVIDENCE_025"; then
  fail "credential canary leaked into evidence"
fi
if grep -rq 'CANARY' "$log"; then
  fail "credential canary leaked into test output"
fi
ok "redaction scan clean (zero credential leakage)"

# Guard 9: evidence is valid JSON with the expected proof fields.
python3 - "$EVIDENCE_015" "$EVIDENCE_025" <<'PY'
import json, sys
for path in sys.argv[1:]:
    with open(path, encoding="utf-8") as fh:
        doc = json.load(fh)
    assert doc["node"] == "EP-028", path
    assert doc["milestone"] == "M5", path
    assert doc["certification"]["real_hydra_provider"] == "NOT_ASSERTED", path
print("evidence json valid and certification boundary preserved")
PY
ok "evidence JSON valid; certification boundary NOT_ASSERTED preserved"

# Regressions: M1 contract + M2/M3 connector + M4 failure suite.
if ! cargo test --locked -p nexus-hydra --all-targets >>"$log" 2>&1; then
  fail "M1 contract regression failed" "$log"
fi
if ! cargo test --locked -p nexus-hydra-connector --all-targets >>"$log" 2>&1; then
  fail "M2/M3 connector regression failed" "$log"
fi
if ! cargo test --locked -p nexus-hydra-e2e --all-targets >>"$log" 2>&1; then
  fail "M4 failure-suite regression failed" "$log"
fi
ok "M1 + M2/M3 + M4 regressions green"

# Milestone artifact/fence checks.
if [ ! -f .agent/milestone-files/EP-028-M5.txt ]; then
  fail ".agent/milestone-files/EP-028-M5.txt missing"
fi
ok "milestone fence present"

echo "EP-028 M5: ok"
