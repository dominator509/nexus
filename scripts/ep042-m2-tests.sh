#!/usr/bin/env sh
# EP-042 M2 gate: deterministic update planning behavior proofs through
# the REAL vitest machinery with vacuity guards (EP-001 gate-masking
# class).
#
# M2 owns apps/setup/src/update/ (pure TypeScript update core adapting
# the canonical M1 release contracts at the boundary) and tests/release/
# (the deterministic proof suite). The authoritative gate is the vitest
# suite plus typecheck, dependency-direction proof, no-placeholder scan,
# workspace registration, and the M1 regression.
#
# Vacuous green is impossible: a green M2 must observe real non-zero
# passing counts, EP-042-owned test names, and zero failed tests.
set -eu
export CI=true
export NO_COLOR=1

log="/tmp/ep042-m2-tests.log"
: > "$log"

fail() {
  echo "EP-042 M2 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-042 M2 gate: $1"; }

# --- M1 regression first ---------------------------------------------------
if ! sh scripts/ep042-m1-tests.sh >>"$log" 2>&1; then
  fail "M1 regression gate failed" "$log"
fi
ok "M1 regression green"

# --- material presence -----------------------------------------------------
for path in \
  apps/setup/src/update/errors.ts \
  apps/setup/src/update/types.ts \
  apps/setup/src/update/validate.ts \
  apps/setup/src/update/digest.ts \
  apps/setup/src/update/manifest.ts \
  apps/setup/src/update/compatibility.ts \
  apps/setup/src/update/planner.ts \
  apps/setup/src/update/backup.ts \
  apps/setup/src/update/rollback.ts \
  apps/setup/src/update/canary.ts \
  apps/setup/src/update/evidence.ts \
  apps/setup/src/update/index.ts \
  tests/release/package.json \
  tests/release/vitest.config.ts \
  tests/release/src/__tests__/ep042_unit_manifest_validation.test.ts \
  tests/release/src/__tests__/ep042_unit_compatibility.test.ts \
  tests/release/src/__tests__/ep042_unit_planner.test.ts \
  tests/release/src/__tests__/ep042_unit_backup_policy.test.ts \
  tests/release/src/__tests__/ep042_unit_rollback_preconditions.test.ts \
  tests/release/src/__tests__/ep042_unit_canary_promotion.test.ts \
  tests/release/src/__tests__/ep042_unit_evidence_redaction.test.ts \
  tests/release/src/__tests__/ep042_unit_dependency_direction.test.ts; do
  [ -f "$path" ] || fail "missing owned path: $path"
done
ok "M2-owned paths present"

# --- workspace registration ------------------------------------------------
grep -q '"tests/release"' pnpm-workspace.yaml || fail "tests/release not registered in pnpm-workspace.yaml"
grep -q 'export \* from "./update"' apps/setup/src/index.ts || fail "update barrel not exported from @nexus/setup"
ok "workspace registration verified"

# --- anti-masking sentinels (node M2 wired to gate) ------------------------
grep -q 'ep042-m2-tests.sh' scripts/nodes/EP-042.sh || fail "node M2 branch not wired to gate"
if grep -q 'node-artifact-check.py EP-042 M2' scripts/nodes/EP-042.sh; then
  fail "node M2 still uses artifact-check masking"
fi
ok "node M2 wired to real gate"

# --- real vitest with vacuity guard ----------------------------------------
if ! (cd tests/release && node_modules/.bin/vitest run src/__tests__ >>"$log" 2>&1); then
  fail "vitest failed" "$log"
fi
if ! grep -Eq 'Tests[[:space:]]+[1-9][0-9]* passed' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi
count=$(grep -Eo 'Tests[[:space:]]+[0-9]+ passed' "$log" | grep -Eo '[0-9]+' | head -1)
if [ "${count:-0}" -lt 60 ]; then
  fail "too few proofs passed: ${count:-0} (need >= 60)"
fi
if grep -Eq '[1-9][0-9]* failed' "$log"; then
  fail "failures present in vitest output" "$log"
fi
ok "vitest ${count:-0} proofs passed, zero failed"

# --- anti-masking sentinels: owned proofs must exist in source -------------
for sentinel in \
  ep042_unit_manifest_rejects_missing_manifest \
  ep042_unit_manifest_rejects_unsupported_schema_version \
  ep042_unit_manifest_rejects_duplicate_component_identity \
  ep042_unit_manifest_exists_not_verified_without_binding \
  ep042_unit_manifest_digest_binding_mismatch_denied \
  ep042_unit_signature_present_not_valid \
  ep042_unit_compatibility_rejects_unknown_component \
  ep042_unit_compatibility_is_deterministic \
  ep042_unit_compatibility_supports_all_profiles \
  ep042_unit_planner_returns_planned_only \
  ep042_unit_planner_never_executes_installation \
  ep042_unit_planner_rejects_downgrade \
  ep042_unit_planner_rejects_incompatible_component_set \
  ep042_unit_planner_plan_contains_no_promote_step \
  ep042_unit_backup_requested_not_completed_denied \
  ep042_unit_backup_proof_wrong_install_id_denied \
  ep042_unit_backup_completed_and_verified_approved \
  ep042_unit_rollback_requires_plan_rollback_path \
  ep042_unit_rollback_receipt_exists_not_proven_without_drill \
  ep042_unit_rollback_all_preconditions_met_proven \
  ep042_unit_canary_ring_defined_not_rollout_approved \
  ep042_unit_promotion_never_automatic \
  ep042_unit_promotion_backup_precondition_not_bypassed \
  ep042_unit_manual_promotion_never_deploys \
  ep042_unit_evidence_binds_current_run_fields \
  ep042_unit_evidence_redacts_runtime_secret_canary \
  ep042_unit_update_core_has_no_node_builtin_imports; do
  if ! grep -rq "$sentinel" tests/release/src/__tests__/; then
    fail "EP-042-owned proof $sentinel missing from test sources"
  fi
done
ok "anti-masking sentinels present (manifest/digest/signature/compatibility/plan/backup/rollback/canary/redaction/dependency)"

# --- dependency direction: update core has no node builtins ----------------
if grep -rqE "from ['\"](node:fs|node:child_process|node:net|node:http|node:https|node:process)['\"]" apps/setup/src/update/; then
  fail "update core imports a node builtin (dependency direction)"
fi
ok "dependency-direction clean (update core is pure)"

# --- no-placeholder scan (production path only; test sources legitimately
# reference the scan patterns in their own assertions) -----------------------
if grep -rniE 'placeholder|TODO|FIXME|not implemented|unimplemented!' apps/setup/src/update 2>/dev/null; then
  fail "placeholder content in apps/setup/src/update"
fi
ok "no-placeholder scan clean (update core)"

# --- typecheck both packages ------------------------------------------------
if ! (cd apps/setup && node_modules/.bin/tsc --noEmit -p tsconfig.json >>"$log" 2>&1); then
  fail "apps/setup typecheck failed" "$log"
fi
if ! (cd tests/release && node_modules/.bin/tsc --noEmit -p tsconfig.json >>"$log" 2>&1); then
  fail "tests/release typecheck failed" "$log"
fi
ok "typecheck clean (apps/setup + tests/release)"

echo "EP-042 M2 gate: ok"
