#!/usr/bin/env sh
# EP-043 M2 gate: production readiness engine + release manifest proofs
# through the REAL vitest machinery with vacuity guards (EP-001
# gate-masking class).
#
# M2 owns PRODUCTION_READINESS.md (the real report) and dist/release/
# (the real release manifest). The authoritative gate is the vitest
# suite (M1 contract + M2 readiness/manifest/repo-state proofs), the
# real CLI executions (readiness + manifest), typecheck, dependency-
# direction proof, no-placeholder scan, workspace registration, and the
# M1 regression.
#
# Vacuous green is impossible: a green M2 must observe real non-zero
# passing counts, EP-043-owned test names, and zero failed tests.
set -eu
export CI=true
export NO_COLOR=1

log="/tmp/ep043-m2-tests.log"
: > "$log"

fail() {
  echo "EP-043 M2 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-043 M2 gate: $1"; }

# --- M1 regression first ---------------------------------------------------
if ! sh scripts/ep043-m1-tests.sh >>"$log" 2>&1; then
  fail "M1 regression gate failed" "$log"
fi
ok "M1 regression green"

# --- material presence -----------------------------------------------------
for path in \
  release-evidence/package.json \
  release-evidence/tsconfig.json \
  release-evidence/tsconfig.build.json \
  release-evidence/src/index.ts \
  release-evidence/src/errors.ts \
  release-evidence/src/model.ts \
  release-evidence/src/readiness.ts \
  release-evidence/src/manifest.ts \
  release-evidence/src/report.ts \
  release-evidence/src/repo-state.ts \
  release-evidence/src/cli.ts \
  release-evidence/scripts/ts-resolve-loader.mjs \
  release-evidence/src/__tests__/ep043_unit_contract.test.ts \
  release-evidence/src/__tests__/ep043_unit_readiness.test.ts \
  release-evidence/src/__tests__/ep043_unit_dependency_direction.test.ts; do
  [ -f "$path" ] || fail "missing owned path: $path"
done
ok "M2-owned paths present"

# --- workspace registration ------------------------------------------------
grep -q '"release-evidence"' pnpm-workspace.yaml || fail "release-evidence not registered in pnpm-workspace.yaml"
ok "workspace registration verified"

# --- anti-masking sentinels (node M2 wired to gate) ------------------------
grep -q 'ep043-m2-tests.sh' scripts/nodes/EP-043.sh || fail "node M2 branch not wired to gate"
if grep -q 'node-artifact-check.py EP-043 M2' scripts/nodes/EP-043.sh; then
  fail "node M2 still uses artifact-check masking"
fi
ok "node M2 wired to real gate"

# --- real vitest with vacuity guard ----------------------------------------
# verbose reporter so individual EP-043-owned test names appear in the log
# for anti-masking sentinel observation.
if ! (cd release-evidence && node_modules/.bin/vitest run src/__tests__ --reporter=verbose >>"$log" 2>&1); then
  fail "vitest failed" "$log"
fi
if ! grep -Eq 'Tests[[:space:]]+[1-9][0-9]* passed' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi
if ! grep -Eq 'ep043_unit_[a-z_]+' "$log"; then
  fail "no EP-043-owned test names observed" "$log"
fi
if grep -Eq '[1-9][0-9]* failed' "$log"; then
  fail "failed tests observed" "$log"
fi
passed=$(grep -Eo 'Tests[[:space:]]+[1-9][0-9]* passed' "$log" | grep -Eo '[0-9]+' | tail -1)
if [ "${passed:-0}" -lt 60 ]; then
  fail "vacuity: only $passed tests passed (need >= 60)"
fi
ok "vitest ${passed} passed, 0 failed"

# --- M2-owned test names observed ------------------------------------------
for sentinel in \
  ep043_unit_readiness_ready_when_all_obligations_met \
  ep043_unit_readiness_blocks_on_pending_certification \
  ep043_unit_readiness_blocks_without_fresh_clone_rerun \
  ep043_unit_manifest_builds_with_real_digests \
  ep043_unit_manifest_digest_strip_then_digest \
  ep043_unit_manifest_parse_rejects_digest_mismatch \
  ep043_unit_repo_graph_nodes_real \
  ep043_unit_repo_certifications_pending_honest; do
  if ! grep -q "$sentinel" "$log"; then
    fail "M2-owned test $sentinel did not run (anti-masking)"
  fi
done
ok "M2-owned tests observed (readiness + manifest + repo-state)"

# --- typecheck --------------------------------------------------------------
if ! (cd release-evidence && node_modules/.bin/tsc --noEmit >>"$log" 2>&1); then
  fail "typecheck failed" "$log"
fi
ok "typecheck clean"

# --- real CLI: readiness report --------------------------------------------
rm -f PRODUCTION_READINESS.md
if ! node --experimental-transform-types \
  --import "file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs" \
  release-evidence/src/cli.ts readiness --output PRODUCTION_READINESS.md >>"$log" 2>&1; then
  fail "readiness CLI failed" "$log"
fi
[ -f PRODUCTION_READINESS.md ] || fail "PRODUCTION_READINESS.md not written"
grep -q "Production readiness is NOT declared" PRODUCTION_READINESS.md \
  || fail "readiness report does not honestly declare NOT declared state"
grep -q "certification row" PRODUCTION_READINESS.md \
  || fail "readiness report missing certification blocking reason"
ok "readiness CLI wrote honest PRODUCTION_READINESS.md"

# --- real CLI: release manifest --------------------------------------------
rm -rf dist/release
if ! node --experimental-transform-types \
  --import "file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs" \
  release-evidence/src/cli.ts manifest --output-dir dist/release >>"$log" 2>&1; then
  fail "manifest CLI failed" "$log"
fi
[ -f dist/release/RELEASE_MANIFEST.json ] || fail "RELEASE_MANIFEST.json not written"
python3 - "$log" <<'PYEOF'
import json, sys
with open("dist/release/RELEASE_MANIFEST.json") as f:
    manifest = json.load(f)
assert manifest["schema_version"] == 1, "schema_version"
assert len(manifest["components"]) >= 2, "components"
assert manifest["manifest_digest"].startswith("sha256:"), "digest"
for comp in manifest["components"]:
    assert comp["digest"].startswith("sha256:"), "component digest"
    assert comp["signature"]["key_id"] == "SIGNATURE_PRESENT_NOT_VERIFIED", "honest signature"
print("manifest json valid: ok")
PYEOF
ok "manifest CLI wrote valid RELEASE_MANIFEST.json"

# --- no-placeholder scan (production sources) -------------------------------
if grep -rnE "TODO|FIXME|XXX placeholder|not implemented|demo mode|sample success" release-evidence/src --include="*.ts" | grep -v "__tests__" >/dev/null 2>&1; then
  fail "placeholder scan found production-source placeholder"
fi
ok "no-placeholder scan clean"

echo "EP-043 M2 gate: ok (GATE_EXIT=0)"
