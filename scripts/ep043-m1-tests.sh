#!/usr/bin/env sh
# EP-043 M1 gate: production readiness and ship contract proofs through
# the REAL vitest machinery with vacuity guards (EP-001 gate-masking
# class).
#
# M1 owns release-evidence/ (@nexus/release-evidence), the provider-
# neutral ship contract package encoding ShipGate, ReleaseEvidence,
# ManualDeployHandoff, and ProductionReadinessDecision. The authoritative
# gate is the vitest suite plus typecheck, dependency-direction proof,
# no-placeholder scan, workspace registration, and the EP-042 regression.
#
# Vacuous green is impossible: a green M1 must observe real non-zero
# passing counts, EP-043-owned test names, and zero failed tests.
set -eu
export CI=true
export NO_COLOR=1

log="/tmp/ep043-m1-tests.log"
: > "$log"

fail() {
  echo "EP-043 M1 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-043 M1 gate: $1"; }

# --- EP-042 regression (owned test surfaces, not the node-fenced gate) ---
# The EP-042 M5 gate carries a scope audit fenced to EP-042's own
# expected-files; once EP-043 files exist that audit cannot pass, which is
# correct fence behavior. The regression requirement is the predecessor's
# owned test surfaces: the nexus-release contract crate and the
# tests/release vitest suites (unit + integration + failure + bundle).
if ! sh -c 'cargo test -p nexus-release --locked >> "$1" 2>&1' _ "$log"; then
  fail "EP-042 regression: nexus-release cargo test failed" "$log"
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "EP-042 regression: no nexus-release tests ran (vacuity guard)" "$log"
fi
if ! (cd tests/release && node_modules/.bin/vitest run >>"$log" 2>&1); then
  fail "EP-042 regression: tests/release vitest failed" "$log"
fi
if ! grep -Eq 'Tests[[:space:]]+[1-9][0-9]* passed' "$log"; then
  fail "EP-042 regression: no tests/release tests ran (vacuity guard)" "$log"
fi
ok "EP-042 regression green (nexus-release + tests/release surfaces)"

# --- material presence -----------------------------------------------------
for path in \
  release-evidence/package.json \
  release-evidence/tsconfig.json \
  release-evidence/tsconfig.build.json \
  release-evidence/src/index.ts \
  release-evidence/src/errors.ts \
  release-evidence/src/model.ts \
  release-evidence/src/__tests__/ep043_unit_contract.test.ts \
  release-evidence/src/__tests__/ep043_unit_dependency_direction.test.ts; do
  [ -f "$path" ] || fail "missing owned path: $path"
done
ok "M1-owned paths present"

# --- workspace registration ------------------------------------------------
grep -q '"release-evidence"' pnpm-workspace.yaml || fail "release-evidence not registered in pnpm-workspace.yaml"
grep -q '"name": "@nexus/release-evidence"' release-evidence/package.json || fail "@nexus/release-evidence name missing"
ok "workspace registration verified"

# --- anti-masking sentinels (node M1 wired to gate) ------------------------
grep -q 'ep043-m1-tests.sh' scripts/nodes/EP-043.sh || fail "node M1 branch not wired to gate"
if grep -q 'node-artifact-check.py EP-043 M1' scripts/nodes/EP-043.sh; then
  fail "node M1 still uses artifact-check masking"
fi
ok "node M1 wired to real gate"

# --- real vitest with vacuity guard ----------------------------------------
if ! (cd release-evidence && node_modules/.bin/vitest run src/__tests__ >>"$log" 2>&1); then
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
if [ "${passed:-0}" -lt 30 ]; then
  fail "vacuity: only $passed tests passed (need >= 30)" "$log"
fi
ok "vitest ${passed} passed, 0 failed"

# --- typecheck --------------------------------------------------------------
if ! (cd release-evidence && node_modules/.bin/tsc --noEmit >>"$log" 2>&1); then
  fail "typecheck failed" "$log"
fi
ok "typecheck clean"

# --- no-placeholder scan (production sources only) --------------------------
if grep -rnE "TODO|FIXME|XXX placeholder|not implemented|demo mode|sample success" release-evidence/src --include="*.ts" | grep -v "__tests__" >/dev/null 2>&1; then
  fail "placeholder scan found production-source placeholder"
fi
ok "no-placeholder scan clean"

# --- dependency-direction: pure domain has no node/provider imports ---------
# Pure modules (errors/model/readiness/manifest/report) must stay node-free;
# I/O adapters (repo-state/cli) may use node builtins but never provider SDKs.
if grep -rnE 'from "(node:|@nexus/|aws-|@aws-|minio|seaweedfs|openai|anthropic|temporal|keycloak|pg|redis)' release-evidence/src/errors.ts release-evidence/src/model.ts release-evidence/src/readiness.ts release-evidence/src/manifest.ts release-evidence/src/report.ts >/dev/null 2>&1; then
  fail "dependency-direction scan found foreign import in pure domain"
fi
if grep -rnE 'from "(@nexus/|aws-|@aws-|minio|seaweedfs|openai|anthropic|temporal|keycloak|pg|redis)' release-evidence/src/repo-state.ts release-evidence/src/cli.ts >/dev/null 2>&1; then
  fail "dependency-direction scan found provider import in adapter"
fi
ok "dependency-direction clean (pure domain node-free; adapters provider-free)"

echo "EP-043 M1 gate: ok (GATE_EXIT=0)"
