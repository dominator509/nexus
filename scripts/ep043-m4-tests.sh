#!/usr/bin/env sh
# EP-043 M4 gate: forced-failure, abuse-case, and observability proofs
# through the REAL vitest machinery with vacuity guards (EP-001
# gate-masking class).
#
# M4 owns RELEASE.md (the release procedure, release-blocking
# conditions, ship-gate semantics, signing boundary, emergency
# abort/rollback triggers, fresh-clone prerequisite, artifact checklist,
# observability fields, diagnostic/recovery mapping) and the
# ep043_failure_* proof suite. The authoritative gate is the vitest
# suite (M1 contract + M2 readiness + M3 integration + M4 forced-
# failure proofs), real CLI forced-failure executions (tampered
# manifest fails closed, forged READY report is not trusted, unknown
# flag rejected, pending certification stays blocking), RELEASE.md
# command resolution, the security and license gates, typecheck, the
# dependency-direction proof, no-placeholder scan, and the M1/M2/M3
# regressions.
#
# Vacuous green is impossible: a green M4 must observe real non-zero
# passing counts, EP-043-owned unit/integration/failure test names, real
# fail-closed CLI behavior on corrupted inputs, and zero failed tests.
set -eu
export CI=true
export NO_COLOR=1

log="/tmp/ep043-m4-tests.log"
: > "$log"

fail() {
  echo "EP-043 M4 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-043 M4 gate: $1"; }

CLI_INVOKE="node --experimental-transform-types --import file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs $(pwd)/release-evidence/src/cli.ts"

# --- resource preflight (M3 lesson: classify, do not mask) ----------------
disk_free=$(df -P / | awk 'NR==2 {print $4}')
if [ "${disk_free:-0}" -lt 1048576 ]; then
  echo "EP-043 M4 gate: RESOURCE_EXHAUSTION - disk free ${disk_free} KB below 1 GB threshold" >&2
  exit 1
fi
ok "resource preflight ok (disk free ${disk_free} KB)"

# --- M1, M2 and M3 regressions first --------------------------------------
if ! sh scripts/ep043-m1-tests.sh >>"$log" 2>&1; then
  fail "M1 regression gate failed" "$log"
fi
ok "M1 regression green"

if ! sh scripts/ep043-m2-tests.sh >>"$log" 2>&1; then
  fail "M2 regression gate failed" "$log"
fi
ok "M2 regression green"

if ! sh scripts/ep043-m3-tests.sh >>"$log" 2>&1; then
  fail "M3 regression gate failed" "$log"
fi
ok "M3 regression green"

# --- material presence ------------------------------------------------------
for path in \
  RELEASE.md \
  release-evidence/src/cli.ts \
  release-evidence/src/repo-state.ts \
  release-evidence/src/errors.ts \
  release-evidence/src/__tests__/ep043_failure.test.ts; do
  [ -f "$path" ] || fail "missing owned path: $path"
done
ok "M4-owned paths present"

# --- RELEASE.md owns real commands -----------------------------------------
grep -q "release-evidence/src/cli.ts readiness" RELEASE.md \
  || fail "RELEASE.md missing readiness command"
grep -q "release-evidence/src/cli.ts manifest" RELEASE.md \
  || fail "RELEASE.md missing manifest command"
grep -q "release-evidence/src/cli.ts verify-manifest" RELEASE.md \
  || fail "RELEASE.md missing verify-manifest command"
grep -q "release-evidence/src/cli.ts ship-gate-status" RELEASE.md \
  || fail "RELEASE.md missing ship-gate-status command"
grep -q "release-evidence/src/cli.ts certification-rows" RELEASE.md \
  || fail "RELEASE.md missing certification-rows command"
grep -q "Release-blocking conditions" RELEASE.md \
  || fail "RELEASE.md missing release-blocking conditions"
grep -q "Ship-gate semantics" RELEASE.md \
  || fail "RELEASE.md missing ship-gate semantics"
grep -q "Signing boundary" RELEASE.md \
  || fail "RELEASE.md missing signing boundary"
grep -q "PRESENT_NOT_VERIFIED" RELEASE.md \
  || fail "RELEASE.md missing signing boundary state"
grep -q "Fresh-clone prerequisite" RELEASE.md \
  || fail "RELEASE.md missing fresh-clone prerequisite"
grep -q "Emergency abort and rollback" RELEASE.md \
  || fail "RELEASE.md missing emergency abort/rollback"
grep -q "Readiness observability" RELEASE.md \
  || fail "RELEASE.md missing observability section"
ok "RELEASE.md real command surface and procedure present"

# --- anti-masking sentinels (node M4 wired to gate) -------------------------
grep -q 'ep043-m4-tests.sh' scripts/nodes/EP-043.sh || fail "node M4 branch not wired to gate"
if grep -q 'node-artifact-check.py EP-043 M4' scripts/nodes/EP-043.sh; then
  fail "node M4 still uses artifact-check masking"
fi
ok "node M4 wired to real gate"

# --- real vitest with vacuity guard ----------------------------------------
if ! (cd release-evidence && node_modules/.bin/vitest run src/__tests__ --reporter=verbose >>"$log" 2>&1); then
  fail "vitest failed" "$log"
fi
if ! grep -Eq 'Tests[[:space:]]+[1-9][0-9]* passed' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi
if ! grep -Eq 'ep043_unit_[a-z_]+' "$log"; then
  fail "no EP-043-owned unit test names observed" "$log"
fi
if ! grep -Eq 'ep043_integration_[a-z_]+' "$log"; then
  fail "no EP-043-owned integration test names observed" "$log"
fi
if ! grep -Eq 'ep043_failure_[a-z_]+' "$log"; then
  fail "no EP-043-owned failure test names observed" "$log"
fi
if grep -Eq '[1-9][0-9]* failed' "$log"; then
  fail "failed tests observed" "$log"
fi
passed=$(grep -Eo 'Tests[[:space:]]+[1-9][0-9]* passed' "$log" | grep -Eo '[0-9]+' | tail -1)
if [ "${passed:-0}" -lt 120 ]; then
  fail "vacuity: only $passed tests passed (need >= 120)"
fi
ok "vitest ${passed} passed, 0 failed"

# --- M4-owned failure test names observed -----------------------------------
for sentinel in \
  ep043_failure_unavailable_dependency_missing_graph \
  ep043_failure_malformed_manifest_json \
  ep043_failure_manifest_tamper_digest_mismatch \
  ep043_failure_missing_artifact_bytes \
  ep043_failure_denied_read_is_fail_closed \
  ep043_failure_duplicate_conflicting_certification_rows \
  ep043_failure_malformed_certification_results_pending_blocking \
  ep043_failure_pending_certification_remains_blocking \
  ep043_failure_forged_ready_report_not_trusted \
  ep043_failure_stale_evidence_not_trusted \
  ep043_failure_ship_gate_blocked_not_inferred \
  ep043_failure_signature_present_not_verified_honest \
  ep043_failure_timeout_blocked_dependency \
  ep043_failure_cancelled_work_no_partial_output \
  ep043_failure_partial_side_effect_no_partial_file \
  ep043_failure_operator_bypass_unknown_flag_rejected \
  ep043_failure_unknown_release_state_fails_closed \
  ep043_failure_redaction_structured_errors \
  ep043_failure_incident_correlation_run_id_recorded; do
  if ! grep -q "$sentinel" "$log"; then
    fail "M4-owned failure test $sentinel did not run (anti-masking)"
  fi
done
ok "M4 failure tests observed (real failure mechanisms)"

# --- typecheck --------------------------------------------------------------
if ! (cd release-evidence && node_modules/.bin/tsc --noEmit >>"$log" 2>&1); then
  fail "typecheck failed" "$log"
fi
ok "typecheck clean"

# --- real CLI: pending certification remains blocking ----------------------
ship_out=$(mktemp /tmp/ep043-m4-ship.XXXXXX)
if ! $CLI_INVOKE ship-gate-status >"$ship_out" 2>>"$log"; then
  fail "ship-gate-status CLI failed" "$log"
fi
grep -q "ship-gate verdict: BLOCKED" "$ship_out" \
  || fail "ship-gate-status did not report BLOCKED honestly"
grep -q "readiness decision: NOT_READY" "$ship_out" \
  || fail "ship-gate-status did not preserve NOT_READY"
rm -f "$ship_out"
ok "ship-gate-status honest BLOCKED/NOT_READY observed"

# --- real CLI: unknown flag rejected (operator bypass) ---------------------
bypass_out=$(mktemp /tmp/ep043-m4-bypass.XXXXXX)
if $CLI_INVOKE readiness --output "$bypass_out" --force >/dev/null 2>>"$log"; then
  fail "unknown flag --force was accepted (operator bypass)"
fi
rm -f "$bypass_out"
ok "operator bypass flag rejected"

# --- real CLI: tampered manifest fails closed -------------------------------
manifest_dir=$(mktemp -d /tmp/ep043-m4-tamper.XXXXXX)
$CLI_INVOKE manifest --output-dir "$manifest_dir" >>"$log" 2>&1 || fail "manifest (tamper prep) failed"
python3 - "$manifest_dir/RELEASE_MANIFEST.json" <<'PYEOF'
import json, sys
path = sys.argv[1]
with open(path) as f:
    manifest = json.load(f)
manifest["components"][0]["digest"] = "sha256:" + "0" * 64
with open(path, "w") as f:
    json.dump(manifest, f)
PYEOF
if $CLI_INVOKE verify-manifest --manifest "$manifest_dir/RELEASE_MANIFEST.json" >/dev/null 2>>"$log"; then
  fail "verify-manifest accepted a tampered manifest (fail-closed violated)"
fi
rm -rf "$manifest_dir"
ok "tampered manifest fails closed"

# --- real CLI: forged READY report is not trusted ---------------------------
forge_dir=$(mktemp -d /tmp/ep043-m4-forge.XXXXXX)
mkdir -p "$forge_dir/.agent/state" "$forge_dir/live-fire" "$forge_dir/provider-certification" "$forge_dir/hardware" "$forge_dir/.git/refs/heads"
printf '# GRAPH\n\n| EP-001 | DEP | DONE |\n| EP-043 | DEP | IN_PROGRESS |\n' > "$forge_dir/.agent/GRAPH.md"
printf '# LEDGER\n| 2026-08-25 | agent | EP-001 | NODE_DONE | ok |\n' > "$forge_dir/.agent/state/LEDGER.md"
printf 'LF-001|EP-001|scripts/live-fire/001.sh|lf-001|proof\n' > "$forge_dir/live-fire/REGISTRY.tsv"
printf '# PROVIDER\n\nRELEASE-BLOCKING-PENDING: DeepSeek required.\n' > "$forge_dir/provider-certification/RESULTS.md"
printf '# HARDWARE\n\nRELEASE-BLOCKING-PENDING: Lab evidence pending.\n' > "$forge_dir/hardware/CERTIFICATION_RESULTS.md"
printf 'ref: refs/heads/main\n' > "$forge_dir/.git/HEAD"
printf '%040d\n' 0 > "$forge_dir/.git/refs/heads/main"
printf '# PRODUCTION READINESS\n\nDecision: READY\nRun: ep043-readiness-forged\nGit commit: %040d\nGenerated: 2026-08-25T00:00:00.000Z\n\nAll obligations met.\n' 0 > "$forge_dir/PRODUCTION_READINESS.md"
if ! (cd "$forge_dir" && $CLI_INVOKE ship-gate-status >"$ship_out" 2>>"$log"); then
  fail "ship-gate-status failed on forged-report tree"
fi
grep -q "readiness decision: NOT_READY" "$ship_out" \
  || fail "forged READY report changed the canonical verdict"
rm -rf "$forge_dir" "$ship_out"
ok "forged READY report does not change canonical truth"

# --- redaction scan on M4-owned text (no tracked secret literals) -----------
for path in RELEASE.md release-evidence/src/__tests__/ep043_failure.test.ts; do
  if grep -nE "sk-[A-Za-z0-9]{8,}|ghp_[A-Za-z0-9]{8,}|AKIA[0-9A-Z]{8,}|-----BEGIN [A-Z ]*PRIVATE KEY-----" "$path" >/dev/null 2>&1; then
    fail "$path contains a tracked secret literal"
  fi
done
ok "M4 redaction clean"

# --- no-placeholder scan (production sources) -------------------------------
if grep -rnE "TODO|FIXME|XXX placeholder|not implemented|demo mode|sample success" release-evidence/src --include="*.ts" | grep -v "__tests__" >/dev/null 2>&1; then
  fail "placeholder scan found production-source placeholder"
fi
ok "no-placeholder scan clean"

# --- security and license gates (fence RUN requirement) ---------------------
if ! sh scripts/security-check.sh >>"$log" 2>&1; then
  fail "security-check failed" "$log"
fi
ok "security check ok"
if ! sh scripts/license-gate.sh >>"$log" 2>&1; then
  fail "license-gate failed" "$log"
fi
ok "license gate ok"

echo "EP-043 M4 gate: ok (GATE_EXIT=0)"
