#!/usr/bin/env sh
# EP-043 M3 gate: real dependency and transport integration proofs.
#
# M3 owns OPERATIONS.md (real operational commands) and the real
# transport boundary: the release-evidence CLI reads real repository
# state, digests real artifact bytes, and the documented commands are
# real and executable. The authoritative gate is the vitest suite
# (M1 contract + M2 readiness/manifest/repo-state proofs + M3
# ep043_integration_* proofs against real CLI executions), the real
# operational CLI runs, OPERATIONS.md command resolution, NOT_READY
# preservation, fail-closed negative proofs, typecheck, dependency-
# direction proof, no-placeholder scan, and the M1/M2 regressions.
#
# Vacuous green is impossible: a green M3 must observe real non-zero
# passing counts, EP-043-owned unit and integration test names, real
# operational command output, and zero failed tests.
set -eu
export CI=true
export NO_COLOR=1

log="/tmp/ep043-m3-tests.log"
: > "$log"

fail() {
  echo "EP-043 M3 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-043 M3 gate: $1"; }

CLI_INVOKE="node --experimental-transform-types --import file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs release-evidence/src/cli.ts"

# --- M1 and M2 regressions first -------------------------------------------
if ! sh scripts/ep043-m1-tests.sh >>"$log" 2>&1; then
  fail "M1 regression gate failed" "$log"
fi
ok "M1 regression green"

if ! sh scripts/ep043-m2-tests.sh >>"$log" 2>&1; then
  fail "M2 regression gate failed" "$log"
fi
ok "M2 regression green"

# --- material presence ------------------------------------------------------
for path in \
  OPERATIONS.md \
  release-evidence/src/cli.ts \
  release-evidence/src/__tests__/ep043_integration.test.ts \
  release-evidence/scripts/ts-resolve-loader.mjs \
  infra/release/fixtures/components/nexus-core \
  infra/release/fixtures/components/nexus-model; do
  [ -f "$path" ] || fail "missing owned path: $path"
done
ok "M3-owned paths present"

# --- OPERATIONS.md owns real commands --------------------------------------
grep -q "release-evidence/src/cli.ts readiness" OPERATIONS.md \
  || fail "OPERATIONS.md missing readiness command"
grep -q "release-evidence/src/cli.ts manifest" OPERATIONS.md \
  || fail "OPERATIONS.md missing manifest command"
grep -q "release-evidence/src/cli.ts verify-manifest" OPERATIONS.md \
  || fail "OPERATIONS.md missing verify-manifest command"
grep -q "release-evidence/src/cli.ts ship-gate-status" OPERATIONS.md \
  || fail "OPERATIONS.md missing ship-gate-status command"
grep -q "release-evidence/src/cli.ts certification-rows" OPERATIONS.md \
  || fail "OPERATIONS.md missing certification-rows command"
grep -q "Fresh-clone procedure" OPERATIONS.md \
  || fail "OPERATIONS.md missing fresh-clone procedure"
grep -q "Rollback" OPERATIONS.md \
  || fail "OPERATIONS.md missing rollback reference"
ok "OPERATIONS.md real command surface present"

# --- anti-masking sentinels (node M3 wired to gate) -------------------------
grep -q 'ep043-m3-tests.sh' scripts/nodes/EP-043.sh || fail "node M3 branch not wired to gate"
if grep -q 'node-artifact-check.py EP-043 M3' scripts/nodes/EP-043.sh; then
  fail "node M3 still uses artifact-check masking"
fi
ok "node M3 wired to real gate"

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
if grep -Eq '[1-9][0-9]* failed' "$log"; then
  fail "failed tests observed" "$log"
fi
passed=$(grep -Eo 'Tests[[:space:]]+[1-9][0-9]* passed' "$log" | grep -Eo '[0-9]+' | tail -1)
if [ "${passed:-0}" -lt 100 ]; then
  fail "vacuity: only $passed tests passed (need >= 100)"
fi
ok "vitest ${passed} passed, 0 failed"

# --- M3-owned integration test names observed -------------------------------
for sentinel in \
  ep043_integration_cli_reads_real_repo_state \
  ep043_integration_manifest_digests_real_artifact_bytes \
  ep043_integration_operations_commands_resolve \
  ep043_integration_not_ready_preserved \
  ep043_integration_fail_closed_missing_dependency \
  ep043_integration_manifest_component_digests_deterministic \
  ep043_integration_verify_manifest_detects_tamper \
  ep043_integration_verify_manifest_fails_closed_missing_artifact \
  ep043_integration_fresh_clone_temp_checkout \
  ep043_integration_cli_fails_closed_on_bad_args \
  ep043_integration_timeout_bounded_completion \
  ep043_integration_cancellation_writes_no_partial \
  ep043_integration_certification_rows_read_real \
  ep043_integration_audit_fields_recorded \
  ep043_integration_event_emission_deterministic; do
  if ! grep -q "$sentinel" "$log"; then
    fail "M3-owned integration test $sentinel did not run (anti-masking)"
  fi
done
ok "M3 integration tests observed (real CLI + repo transport + fresh clone)"

# --- typecheck --------------------------------------------------------------
if ! (cd release-evidence && node_modules/.bin/tsc --noEmit >>"$log" 2>&1); then
  fail "typecheck failed" "$log"
fi
ok "typecheck clean"

# --- real CLI: ship-gate-status (honest NOT_READY preserved) ----------------
ship_out=$(mktemp /tmp/ep043-m3-ship.XXXXXX)
if ! $CLI_INVOKE ship-gate-status >"$ship_out" 2>>"$log"; then
  fail "ship-gate-status CLI failed" "$log"
fi
grep -q "ship-gate verdict: BLOCKED" "$ship_out" \
  || fail "ship-gate-status did not report BLOCKED honestly"
grep -q "readiness decision: NOT_READY" "$ship_out" \
  || fail "ship-gate-status did not preserve NOT_READY"
grep -q "certification row" "$ship_out" \
  || fail "ship-gate-status missing certification blocking reason"
rm -f "$ship_out"
ok "ship-gate-status honest BLOCKED/NOT_READY observed"

# --- real CLI: certification-rows (real RESULTS.md rows) --------------------
rows_out=$(mktemp /tmp/ep043-m3-rows.XXXXXX)
if ! $CLI_INVOKE certification-rows >"$rows_out" 2>>"$log"; then
  fail "certification-rows CLI failed" "$log"
fi
grep -q "PROVIDER" "$rows_out" || fail "certification-rows missing PROVIDER rows"
grep -q "HARDWARE" "$rows_out" || fail "certification-rows missing HARDWARE rows"
grep -q "RELEASE-BLOCKING-PENDING" "$rows_out" \
  || fail "certification-rows missing pending state"
rm -f "$rows_out"
ok "certification-rows read real RESULTS.md rows"

# --- real CLI: manifest + verify-manifest (real artifact digests) -----------
manifest_dir=$(mktemp -d /tmp/ep043-m3-manifest.XXXXXX)
if ! $CLI_INVOKE manifest --output-dir "$manifest_dir" >>"$log" 2>&1; then
  fail "manifest CLI failed" "$log"
fi
[ -f "$manifest_dir/RELEASE_MANIFEST.json" ] || fail "manifest not written"
if ! $CLI_INVOKE verify-manifest --manifest "$manifest_dir/RELEASE_MANIFEST.json" >"$ship_out" 2>>"$log"; then
  fail "verify-manifest CLI failed on real manifest" "$log"
fi
grep -q "verify-manifest: ok" "$ship_out" || fail "verify-manifest did not report ok"
rm -rf "$manifest_dir" "$ship_out"
ok "manifest + verify-manifest verified real artifact digests"

# --- real CLI: verify-manifest fails closed on tamper -----------------------
manifest_dir=$(mktemp -d /tmp/ep043-m3-tamper.XXXXXX)
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
ok "verify-manifest fails closed on tampered manifest"

# --- redaction scan on OPERATIONS.md (no tracked secret literals) -----------
if grep -nE "sk-[A-Za-z0-9]{8,}|ghp_[A-Za-z0-9]{8,}|AKIA[0-9A-Z]{8,}|-----BEGIN [A-Z ]*PRIVATE KEY-----" OPERATIONS.md >/dev/null 2>&1; then
  fail "OPERATIONS.md contains a tracked secret literal"
fi
ok "OPERATIONS.md redaction clean"

# --- no-placeholder scan (production sources) -------------------------------
if grep -rnE "TODO|FIXME|XXX placeholder|not implemented|demo mode|sample success" release-evidence/src --include="*.ts" | grep -v "__tests__" >/dev/null 2>&1; then
  fail "placeholder scan found production-source placeholder"
fi
ok "no-placeholder scan clean"

echo "EP-043 M3 gate: ok (GATE_EXIT=0)"
